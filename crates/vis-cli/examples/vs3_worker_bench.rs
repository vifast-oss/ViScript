// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b gate B7** — benchmark CPU-only + payload matrix cho `WorkerPool` opt-in.
//!
//! Chạy thủ công (example, không nằm trong test suite):
//! ```text
//! cargo run -p vis-cli --example vs3_worker_bench
//! ```
//! Đo W=1/2/4; payload list 1k/10k, map 1k, record 16 trường + list1k, enum(list1k),
//! str 1MiB; mỗi task copy payload **trên producer** (giống spawn snapshot một lần) rồi
//! CPU ~200 µs + charge ngân sách. Median 5 lần; ghi topology/load ở đầu/cuối.
//!
//! Đây là benchmark **cơ chế** WorkerPool + SendValue (chưa nối evaluator; W1 sở hữu).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use vis_interp::rt::fee::SharedBudget;
use vis_interp::value::worker::{SendValue, WorkerError, WorkerPool};

fn tmp_env() -> HashMap<String, SendValue> {
    HashMap::new()
}

fn cpu_model() -> String {
    let s = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("model name") {
            if let Some(v) = rest.split(':').nth(1) {
                return v.trim().to_string();
            }
        }
    }
    "unknown".to_string()
}

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[inline(never)]
fn cpu_work(iters: u64) -> u64 {
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    for i in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i ^ 0x5deece66d);
        x ^= x >> 29;
    }
    std::hint::black_box(x)
}

fn calibrate_cpu(target_ns: f64) -> (u64, f64) {
    let mut iters = 10_000u64;
    for _ in 0..24 {
        let t = Instant::now();
        std::hint::black_box(cpu_work(iters));
        let ns = t.elapsed().as_nanos() as f64;
        if ns >= target_ns {
            return (iters, ns / 1000.0);
        }
        let scale = (target_ns / ns.max(1.0)).clamp(1.2, 4.0);
        iters = ((iters as f64) * scale) as u64 + 1;
    }
    let t = Instant::now();
    std::hint::black_box(cpu_work(iters));
    (iters, t.elapsed().as_nanos() as f64 / 1000.0)
}

fn warm_cpu(ms: u64) {
    let start = Instant::now();
    let mut acc = 0u64;
    while start.elapsed().as_millis() < ms as u128 {
        acc ^= cpu_work(100_000);
    }
    std::hint::black_box(acc);
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn payload_bytes(v: &SendValue) -> usize {
    match v {
        SendValue::Unit => 0,
        SendValue::Int(_)
        | SendValue::Float(_)
        | SendValue::Size(_)
        | SendValue::Duration(_)
        | SendValue::Percent(_)
        | SendValue::Bool(_)
        | SendValue::Char(_) => 8,
        SendValue::Str(s) | SendValue::Path(s) => s.len(),
        SendValue::Bytes(b) => b.len(),
        SendValue::List(xs) => {
            xs.len() * std::mem::size_of::<SendValue>()
                + xs.iter().map(payload_bytes).sum::<usize>()
        }
        SendValue::Map(kv) => {
            kv.len() * std::mem::size_of::<(SendValue, SendValue)>()
                + kv.iter()
                    .map(|(k, v)| payload_bytes(k) + payload_bytes(v))
                    .sum::<usize>()
        }
        SendValue::Record(_, fs) => {
            fs.len() * std::mem::size_of::<(String, SendValue)>()
                + fs.iter()
                    .map(|(k, v)| k.len() + payload_bytes(v))
                    .sum::<usize>()
        }
        SendValue::Enum(_, _, args) => {
            args.len() * std::mem::size_of::<SendValue>()
                + args.iter().map(payload_bytes).sum::<usize>()
        }
    }
}

fn payloads() -> Vec<(&'static str, SendValue)> {
    let list1k = SendValue::List((0..1_000).map(SendValue::Int).collect());
    let list10k = SendValue::List((0..10_000).map(SendValue::Int).collect());
    let map1k = SendValue::Map(
        (0..1_000)
            .map(|i| (SendValue::Str(format!("k{i}")), SendValue::Int(i as i64)))
            .collect(),
    );
    let record = SendValue::Record(
        "R".to_string(),
        (0..16)
            .map(|i| {
                (
                    format!("f{i}"),
                    if i == 0 {
                        list1k.clone()
                    } else {
                        SendValue::Int(i as i64)
                    },
                )
            })
            .collect(),
    );
    let enum_v = SendValue::Enum("Result".into(), "Ok".into(), vec![list1k.clone()]);
    let str1m = SendValue::Str("x".repeat(1024 * 1024));
    vec![
        ("unit", SendValue::Unit),
        ("list1k", list1k),
        ("list10k", list10k),
        ("map1k", map1k),
        ("record", record),
        ("enum", enum_v),
        ("str1MiB", str1m),
    ]
}

struct BenchRun {
    ms: f64,
    used: i64,
    remaining: i64,
    limit: i64,
}

fn run_matrix(
    workers: usize,
    payload: &SendValue,
    cpu_iters: u64,
    tasks: usize,
    charges: usize,
    chunk: u64,
) -> BenchRun {
    let limit = (tasks * charges) as i64 + 10_000;
    let budget = Arc::new(SharedBudget::new(limit as u64, chunk));
    let t0 = Instant::now();
    let mut pool = WorkerPool::with_budget(workers, tmp_env(), Arc::clone(&budget)).expect("pool");
    for _ in 0..tasks {
        // Copy payload trên producer (biên spawn), rồi task dùng bản của nó.
        let p = payload.clone();
        let cpu = cpu_iters;
        let charge = charges;
        pool.spawn(move |ctx| {
            std::hint::black_box(&p);
            std::hint::black_box(cpu_work(cpu));
            for _ in 0..charge {
                ctx.charge(1)?;
            }
            Ok(SendValue::Unit)
        })
        .expect("spawn");
    }
    let r = pool.join().expect("join");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let snap = r.budget.expect("budget snapshot");
    assert_eq!(snap.used, (tasks * charges) as i64, "used");
    assert_eq!(
        snap.used + snap.remaining,
        snap.limit,
        "used + remaining == limit"
    );
    assert!(!snap.exhausted, "budget không được cạn");
    assert_eq!(r.outcomes.len(), tasks);
    BenchRun {
        ms,
        used: snap.used,
        remaining: snap.remaining,
        limit: snap.limit,
    }
}

/// **Worker-copy**: producer chỉ clone `Arc` O(1); worker clone payload vào heap.
fn run_worker_copy(
    workers: usize,
    payload: &SendValue,
    cpu_iters: u64,
    tasks: usize,
    charges: usize,
    chunk: u64,
) -> BenchRun {
    let limit = (tasks * charges) as i64 + 10_000;
    let budget = Arc::new(SharedBudget::new(limit as u64, chunk));
    let template = Arc::new(payload.clone());
    let t0 = Instant::now();
    let mut pool = WorkerPool::with_budget(workers, tmp_env(), Arc::clone(&budget)).expect("pool");
    for _ in 0..tasks {
        let p = Arc::clone(&template);
        let cpu = cpu_iters;
        let charge = charges;
        pool.spawn_worker_copy(p, move |ctx, payload| {
            std::hint::black_box(&payload);
            std::hint::black_box(cpu_work(cpu));
            for _ in 0..charge {
                ctx.charge(1)?;
            }
            Ok(SendValue::Unit)
        })
        .expect("spawn worker-copy");
    }
    let r = pool.join().expect("join");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let snap = r.budget.expect("budget");
    assert_eq!(snap.used, (tasks * charges) as i64);
    assert_eq!(snap.used + snap.remaining, snap.limit);
    assert!(!snap.exhausted);
    assert_eq!(r.outcomes.len(), tasks);
    BenchRun {
        ms,
        used: snap.used,
        remaining: snap.remaining,
        limit: snap.limit,
    }
}

fn run_charge_only(workers: usize, tasks: usize, charges: usize, chunk: u64) -> BenchRun {
    let limit = (tasks * charges) as i64 + 10_000;
    let budget = Arc::new(SharedBudget::new(limit as u64, chunk));
    let t0 = Instant::now();
    let mut pool = WorkerPool::with_budget(workers, tmp_env(), Arc::clone(&budget)).expect("pool");
    for _ in 0..tasks {
        pool.spawn(move |ctx| {
            for _ in 0..charges {
                ctx.charge(1)?;
            }
            Ok(SendValue::Unit)
        })
        .unwrap();
    }
    let r = pool.join().expect("join");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let snap = r.budget.expect("budget");
    assert_eq!(snap.used, (tasks * charges) as i64);
    assert_eq!(snap.used + snap.remaining, snap.limit);
    BenchRun {
        ms,
        used: snap.used,
        remaining: snap.remaining,
        limit: snap.limit,
    }
}

fn run_probe(
    workers: usize,
    payload: &SendValue,
    mode: &str,
    cpu_iters: u64,
    tasks: usize,
    charges: usize,
    chunk: u64,
) -> BenchRun {
    let limit = (tasks * charges) as i64 + 10_000;
    let budget = Arc::new(SharedBudget::new(limit as u64, chunk));
    let shared = Arc::new(payload.clone());
    let mut prebuilt: Vec<SendValue> = if mode == "move" {
        (0..tasks).map(|_| payload.clone()).collect()
    } else {
        Vec::new()
    };
    let t0 = Instant::now();
    let mut pool = WorkerPool::with_budget(workers, tmp_env(), Arc::clone(&budget)).expect("pool");
    for _ in 0..tasks {
        let cpu = cpu_iters;
        let charge = charges;
        let mode = mode.to_string();
        let p = if mode == "producer" {
            Some(payload.clone())
        } else if mode == "move" {
            prebuilt.pop()
        } else {
            None
        };
        let arc = Arc::clone(&shared);
        pool.spawn(move |ctx| {
            if let Some(p) = &p {
                std::hint::black_box(p);
            } else {
                // `worker-copy`: copy payload trong worker (song song).
                let p = (*arc).clone();
                std::hint::black_box(&p);
            }
            std::hint::black_box(cpu_work(cpu));
            for _ in 0..charge {
                ctx.charge(1)?;
            }
            Ok(SendValue::Unit)
        })
        .unwrap();
    }
    let r = pool.join().expect("join");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let snap = r.budget.expect("budget");
    assert_eq!(snap.used, (tasks * charges) as i64);
    assert_eq!(snap.used + snap.remaining, snap.limit);
    BenchRun {
        ms,
        used: snap.used,
        remaining: snap.remaining,
        limit: snap.limit,
    }
}

fn check_print_merge() -> usize {
    let n = 12usize;
    let mut outs = Vec::new();
    for w in [1usize, 4] {
        let mut pool = WorkerPool::new(w, tmp_env()).expect("pool");
        for i in 0..n {
            pool.spawn(move |ctx| {
                ctx.print_line(&format!("t{i}:a"));
                ctx.print_line(&format!("t{i}:b"));
                Ok(SendValue::Int(i as i64))
            })
            .unwrap();
        }
        let r = pool.join().expect("join");
        assert_eq!(r.outcomes.len(), n);
        outs.push(r.output);
    }
    assert_eq!(outs[0], outs[1], "W=1 vs W=4 output phải byte-identical");
    outs[0].len()
}

fn check_cross_write() {
    for w in [1usize, 4] {
        let mut pool = WorkerPool::new(w, tmp_env()).expect("pool");
        for i in 0..8usize {
            pool.spawn(move |ctx| {
                let cur = match ctx.get("counter") {
                    Some(SendValue::Int(v)) => *v,
                    _ => 0,
                };
                ctx.set("counter", SendValue::Int(cur + i as i64));
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        match pool.join() {
            Ok(r) if w == 1 => {
                assert_eq!(r.env.get("counter"), Some(&SendValue::Int(28)));
            }
            Err(WorkerError::CrossWrite { vars, tasks: _ }) if w == 4 => {
                assert_eq!(vars, vec!["counter".to_string()]);
            }
            other => panic!("W={w}: cross-write check nhận {other:?}"),
        }
    }
}

fn main() {
    warm_cpu(400);
    let (cpu_iters, cpu_us) = calibrate_cpu(200_000.0);
    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!(
        "ENV\tnproc={nproc}\tcpu={}\tload_start={}\tos={}",
        cpu_model(),
        load(),
        std::env::consts::OS
    );
    println!("CALIB\tcpu_iters={cpu_iters}\tper_task_us={cpu_us:.1}");

    println!("CLONE\tpayload\tns_med\tbytes_logical");
    for (name, payload) in payloads() {
        let mut samples = Vec::new();
        for _ in 0..20 {
            let t = Instant::now();
            let p = payload.clone();
            std::hint::black_box(&p);
            samples.push(t.elapsed().as_nanos() as f64);
        }
        let med = median(samples);
        println!("CLONE\t{name}\t{med:.0}\t{}", payload_bytes(&payload));
    }

    let tasks = 64usize;
    let charges = 100usize;
    let reps = 5usize;
    let chunk = 256u64;
    println!(
        "MATRIX\tpayload\tworkers\tms_med\tms_min\tns_per_task\tspeedup\tused\tremaining\tlimit"
    );
    for (name, payload) in payloads() {
        let mut base = 0.0;
        let mut rows = Vec::new();
        for w in [1usize, 2, 4] {
            let mut samples = Vec::new();
            for _ in 0..reps {
                let r = run_matrix(w, &payload, cpu_iters, tasks, charges, chunk);
                samples.push(r.ms);
                assert_eq!(r.used + r.remaining, r.limit);
            }
            let med = median(samples.clone());
            if w == 1 {
                base = med;
            }
            rows.push((
                w,
                med,
                samples.iter().copied().fold(f64::INFINITY, f64::min),
                base / med,
            ));
        }
        for (w, med, min, sp) in rows {
            println!(
                "MATRIX\t{name}\t{w}\t{med:.2}\t{min:.2}\t{:.0}\t{sp:.2}\t{}\t{}\t{}",
                med * 1e6 / tasks as f64,
                (tasks * charges) as i64,
                10_000,
                (tasks * charges) as i64 + 10_000
            );
        }
    }

    println!(
        "MATRIX_WC\tpayload\tworkers\tms_med\tms_min\tns_per_task\tspeedup\tused\tremaining\tlimit"
    );
    for (name, payload) in payloads() {
        let mut base = 0.0;
        let mut rows = Vec::new();
        for w in [1usize, 2, 4] {
            let mut samples = Vec::new();
            let mut used = 0;
            let mut remaining = 0;
            let mut limit = 0;
            for _ in 0..reps {
                let r = run_worker_copy(w, &payload, cpu_iters, tasks, charges, chunk);
                assert_eq!(r.used + r.remaining, r.limit);
                samples.push(r.ms);
                used = r.used;
                remaining = r.remaining;
                limit = r.limit;
            }
            let med = median(samples.clone());
            if w == 1 {
                base = med;
            }
            rows.push((
                w,
                med,
                samples.iter().copied().fold(f64::INFINITY, f64::min),
                base / med,
                used,
                remaining,
                limit,
            ));
        }
        for (w, med, min, sp, used, remaining, limit) in rows {
            println!(
                "MATRIX_WC\t{name}\t{w}\t{med:.2}\t{min:.2}\t{:.0}\t{sp:.2}\t{used}\t{remaining}\t{limit}",
                med * 1e6 / tasks as f64
            );
        }
    }

    println!("PROBE\tpayload\tmode\tworkers\tms_med");
    for (name, payload) in payloads() {
        if name != "map1k" && name != "list10k" {
            continue;
        }
        for mode in ["producer", "worker", "move"] {
            for w in [1usize, 4] {
                let mut xs = Vec::new();
                for _ in 0..3 {
                    xs.push(run_probe(w, &payload, mode, cpu_iters, tasks, charges, chunk).ms);
                }
                println!("PROBE\t{name}\t{mode}\t{w}\t{:.2}", median(xs));
            }
        }
    }

    println!(
        "CHUNK\tchunk\tworkers\ttasks\tcharges\tms_med\tns_per_charge\tused\tremaining\tlimit"
    );
    for chunk in [1u64, 16, 256, 4096] {
        let mut samples = Vec::new();
        let mut used = 0;
        let mut remaining = 0;
        let mut limit = 0;
        for _ in 0..reps {
            let r = run_charge_only(4, 64, 2_000, chunk);
            samples.push(r.ms);
            used = r.used;
            remaining = r.remaining;
            limit = r.limit;
        }
        let med = median(samples);
        println!(
            "CHUNK\t{chunk}\t4\t64\t2000\t{med:.2}\t{:.1}\t{used}\t{remaining}\t{limit}",
            med * 1e6 / (64.0 * 2000.0)
        );
    }

    let bytes = check_print_merge();
    println!("CHECK\tprint_merge\tW=1==W=4\tOK\t{bytes} byte");
    check_cross_write();
    println!("CHECK\tcross_write\tW=1 allowed/W=4 refused\tOK");
    println!("CHECK\tbudget\tused+remaining==limit\tOK");
    println!("LOAD_END\t{}", load());
}
