// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 — điều tra map producer-copy <1×** (example, không production).
//!
//! Tách chi phí **clone/drop** khỏi CPU: map1k khoá `Str` (1000 String alloc) so map1k khoá
//! `Int` (1 Vec alloc) và list10k (1 Vec alloc). Đo:
//! - clone trên main (không drop trong lúc đo),
//! - drop trên main,
//! - pool drop: producer-copy (clone trong lúc đo) / move (prebuild ngoài) / worker-copy,
//!   W=1/2/4, không CPU/charge → chỉ còn allocator + channel.
//!
//! Chạy release:
//! ```text
//! cargo run --release -p vis-cli --example vs3_worker_map_probe
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use vis_interp::value::worker::{SendValue, WorkerPool};

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn list(n: usize) -> SendValue {
    SendValue::List((0..n).map(|i| SendValue::Int(i as i64)).collect())
}

fn map_str(n: usize) -> SendValue {
    SendValue::Map(
        (0..n)
            .map(|i| (SendValue::Str(format!("k{i}")), SendValue::Int(i as i64)))
            .collect(),
    )
}

fn map_int(n: usize) -> SendValue {
    SendValue::Map(
        (0..n)
            .map(|i| (SendValue::Int(i as i64), SendValue::Int(i as i64)))
            .collect(),
    )
}

fn map_bytes(v: &SendValue) -> (usize, usize) {
    match v {
        SendValue::Map(kv) => (
            kv.len(),
            kv.len() * std::mem::size_of::<(SendValue, SendValue)>(),
        ),
        SendValue::List(xs) => (xs.len(), xs.len() * std::mem::size_of::<SendValue>()),
        _ => (0, 0),
    }
}

/// Clone thủ công có `with_capacity` — kiểm xem reserve có giúp gì không.
fn clone_reserve(v: &SendValue) -> SendValue {
    match v {
        SendValue::Map(kv) => {
            let mut out = Vec::with_capacity(kv.len());
            for (k, val) in kv {
                out.push((k.clone(), val.clone()));
            }
            SendValue::Map(out)
        }
        other => other.clone(),
    }
}

fn clone_med(payload: &SendValue, reps: usize) -> (f64, usize) {
    let mut xs = Vec::with_capacity(reps);
    let mut clones = Vec::with_capacity(reps);
    for _ in 0..reps {
        let t = Instant::now();
        let c = payload.clone();
        xs.push(t.elapsed().as_nanos() as f64);
        clones.push(c);
    }
    let bytes = clones.first().map(|c| map_bytes(c).1).unwrap_or(0);
    drop(clones);
    (median(xs), bytes)
}

fn drop_med(payload: &SendValue, reps: usize) -> f64 {
    let clones: Vec<SendValue> = (0..reps).map(|_| payload.clone()).collect();
    let t = Instant::now();
    drop(clones);
    t.elapsed().as_nanos() as f64 / reps as f64
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

fn pool_cpu(workers: usize, payload: &SendValue, mode: &str, tasks: usize, cpu_iters: u64) -> f64 {
    let template = Arc::new(payload.clone());
    let t0 = Instant::now();
    let mut pool = WorkerPool::new(workers, HashMap::new()).expect("pool");
    for _ in 0..tasks {
        let cpu = cpu_iters;
        match mode {
            "producer" => {
                let p = payload.clone();
                pool.spawn(move |_ctx| {
                    std::hint::black_box(&p);
                    std::hint::black_box(cpu_work(cpu));
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            "worker" => {
                let tmpl = Arc::clone(&template);
                pool.spawn_worker_copy(tmpl, move |_ctx, p| {
                    std::hint::black_box(&p);
                    std::hint::black_box(cpu_work(cpu));
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            _ => panic!("mode"),
        }
    }
    pool.join().expect("join");
    t0.elapsed().as_secs_f64() * 1000.0
}

fn pool_drop(workers: usize, payload: &SendValue, mode: &str, tasks: usize) -> f64 {
    let template = Arc::new(payload.clone());
    let mut prebuilt: Vec<SendValue> = if mode == "move" {
        (0..tasks).map(|_| payload.clone()).collect()
    } else {
        Vec::new()
    };
    let t0 = Instant::now();
    let mut pool = WorkerPool::new(workers, HashMap::new()).expect("pool");
    for _ in 0..tasks {
        match mode {
            "producer" => {
                let p = payload.clone();
                pool.spawn(move |_ctx| {
                    std::hint::black_box(&p);
                    drop(p);
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            "move" => {
                let p = prebuilt.pop().expect("prebuilt");
                pool.spawn(move |_ctx| {
                    std::hint::black_box(&p);
                    drop(p);
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            "worker" => {
                let tmpl = Arc::clone(&template);
                pool.spawn_worker_copy(tmpl, |_ctx, p| {
                    std::hint::black_box(&p);
                    drop(p);
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            "shared" => {
                // Read-only: task giữ `Arc`, không deep-clone payload.
                let tmpl = Arc::clone(&template);
                pool.spawn(move |_ctx| {
                    std::hint::black_box(&*tmpl);
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            _ => panic!("mode"),
        }
    }
    pool.join().expect("join");
    t0.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let list10k = list(10_000);
    let map_s = map_str(1_000);
    let map_int = map_int(1_000);
    let reps = 64usize;
    let tasks = 64usize;

    println!(
        "SIZE\tsend={}\tpair={}\tmap1k_vec={}\tlist10k_vec={}\tmap_str_key_allocs=1000",
        std::mem::size_of::<SendValue>(),
        std::mem::size_of::<(SendValue, SendValue)>(),
        map_bytes(&map_s).1,
        map_bytes(&list10k).1
    );
    println!("CLONE\tpayload\tns_med\tvec_bytes");
    for (name, p) in [
        ("list10k", &list10k),
        ("map_int", &map_int),
        ("map_str", &map_s),
    ] {
        let (ns, bytes) = clone_med(p, reps);
        println!("CLONE\t{name}\t{ns:.0}\t{bytes}");
    }
    let (ns_res, bytes_res) = clone_med(&clone_reserve(&map_s), reps);
    println!("CLONE\tmap_str_reserve\t{ns_res:.0}\t{bytes_res}");
    println!("DROP\tpayload\tns_per_drop");
    for (name, p) in [
        ("list10k", &list10k),
        ("map_int", &map_int),
        ("map_str", &map_s),
    ] {
        println!("DROP\t{name}\t{:.0}", drop_med(p, reps));
    }

    println!("POOLDROP\tpayload\tmode\tworkers\tms_med");
    for (name, p) in [
        ("list10k", &list10k),
        ("map_int", &map_int),
        ("map_str", &map_s),
    ] {
        for mode in ["producer", "move", "worker", "shared"] {
            for w in [1usize, 2, 4] {
                let xs: Vec<f64> = (0..3).map(|_| pool_drop(w, p, mode, tasks)).collect();
                println!("POOLDROP\t{name}\t{mode}\t{w}\t{:.3}", median(xs));
            }
        }
    }

    println!("MAPSIZE\tentries\tmode\tworkers\tms_med");
    for n in [64usize, 256, 1_024, 4_096] {
        let p = map_str(n);
        for mode in ["producer", "move", "worker"] {
            for w in [1usize, 4] {
                let xs: Vec<f64> = (0..3).map(|_| pool_drop(w, &p, mode, tasks)).collect();
                println!("MAPSIZE\t{n}\t{mode}\t{w}\t{:.3}", median(xs));
            }
        }
    }

    let (cpu_iters, cpu_us) = calibrate_cpu(200_000.0);
    println!("CALIB\tcpu_iters={cpu_iters}\tper_task_us={cpu_us:.1}");
    println!("MAPCPU\tentries\tmode\tworkers\tms_med");
    for n in [64usize, 256, 1_024] {
        let p = map_str(n);
        for mode in ["producer", "worker"] {
            for w in [1usize, 4] {
                let xs: Vec<f64> = (0..3)
                    .map(|_| pool_cpu(w, &p, mode, tasks, cpu_iters))
                    .collect();
                println!("MAPCPU\t{n}\t{mode}\t{w}\t{:.3}", median(xs));
            }
        }
    }
}
