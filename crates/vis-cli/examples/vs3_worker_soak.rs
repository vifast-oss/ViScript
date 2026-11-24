// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b (W5, VD-271)** — mixed soak dài hạn cho worker pool opt-in/test-only.
//!
//! Tách khỏi `vs3_worker.rs` để giữ luật R1 (≤1500 dòng/tệp). Chạy đầy đủ 30 phút:
//!
//! ```text
//! VS3_W5_MIXED_SECS=1800 VS3_W5_MIXED_ROUNDS=1000000 \
//!   cargo run --release -p vis-cli --example vs3_worker_soak
//! ```
//!
//! Mỗi vòng 4 pha × 10k task × W=1/W=4 = 80k spawn; xoay `spawn_shared` map1k/list10k,
//! `spawn_worker_copy`, `spawn_with_payload`; kiểm print merge, budget, cancel, cross-write,
//! Arc/payload, RSS/threads. Mặc định 1 vòng (~3 s) cho gate.

use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_runtime::rt::fee::SharedBudget;
use vis_runtime::worker::{EnvMap, SendValue, WorkerError, WorkerPool};

fn env() -> EnvMap {
    let mut m = EnvMap::new();
    m.insert("base".into(), SendValue::Int(10));
    m.insert(
        "data".into(),
        SendValue::List((0..8).map(SendValue::Int).collect()),
    );
    m
}

fn shared_map1k() -> SendValue {
    SendValue::Map(
        (0..1_000)
            .map(|i| (SendValue::Str(format!("k{i}")), SendValue::Int(i as i64)))
            .collect(),
    )
}

fn shared_list10k() -> SendValue {
    SendValue::List((0..10_000).map(|i| SendValue::Int(i as i64)).collect())
}

fn shared_len(v: &SendValue) -> usize {
    match v {
        SendValue::Map(kv) => kv.len(),
        SendValue::List(xs) => xs.len(),
        _ => 0,
    }
}

fn assert_shared_payload(v: &SendValue, round: usize, tag: &str) {
    match v {
        SendValue::Map(kv) => {
            assert_eq!(kv.len(), 1_000, "vòng {round}/{tag}: map1k phải còn 1.000");
            assert_eq!(
                kv[0],
                (SendValue::Str("k0".into()), SendValue::Int(0)),
                "vòng {round}/{tag}: map[0] đổi"
            );
            assert_eq!(
                kv[999],
                (SendValue::Str("k999".into()), SendValue::Int(999)),
                "vòng {round}/{tag}: map[999] đổi"
            );
        }
        SendValue::List(xs) => {
            assert_eq!(
                xs.len(),
                10_000,
                "vòng {round}/{tag}: list10k phải còn 10.000"
            );
            assert_eq!(xs[0], SendValue::Int(0), "vòng {round}/{tag}: list[0] đổi");
            assert_eq!(
                xs[9_999],
                SendValue::Int(9_999),
                "vòng {round}/{tag}: list[9999] đổi"
            );
        }
        other => panic!("vòng {round}/{tag}: payload lạ {other:?}"),
    }
}

fn status_kb(key: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

// ===========================================================================
// **VS3-3b/W5 mixed soak** — env-gated, mặc định ngắn cho gate.
//
// Chạy đầy đủ **30 phút** (1800 s, dừng ở biên batch):
//   VS3_W5_MIXED_SECS=1800 VS3_W5_MIXED_ROUNDS=1000000 \
//     cargo run --release -p vis-cli --example vs3_worker_soak
//
// Mỗi vòng: **4 pha × 10.000 task**, W=1 và W=4. Task xoay vòng 4 mode:
//   0. spawn_shared(map1k)        1. spawn_shared(list10k)
//   2. spawn_worker_copy(list10k) 3. spawn_with_payload(list256)
// Pha A print merge + budget, B budget cạn, C cancel, D cross-write (W=1 cộng dồn,
// W=4 `Err(CrossWrite)`). Batch = **5 vòng**; mỗi batch kiểm bất biến + RSS/threads,
// log một dòng; lỗi bất biến ⇒ panic dừng ngay. Chỉ test — không đổi
// production/ABI/default. Mặc định `VS3_W5_MIXED_ROUNDS=1` (~3 s release).
// ===========================================================================

const MIXED_N: usize = 10_000;
const MIXED_BATCH: usize = 5;
const MIXED_MOVE_LEN: usize = 256;
const MIXED_CANCEL_SLEEP_US: u64 = 20;

fn mixed_rounds() -> usize {
    std::env::var("VS3_W5_MIXED_ROUNDS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(1)
}

fn mixed_secs() -> f64 {
    std::env::var("VS3_W5_MIXED_SECS")
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|n| *n > 0.0)
        .unwrap_or(120.0)
}

#[derive(Debug, Clone, Copy)]
enum MixedMode {
    SharedMap,
    SharedList,
    WorkerCopy,
    Move,
}

impl MixedMode {
    fn of(i: usize) -> Self {
        match i % 4 {
            0 => Self::SharedMap,
            1 => Self::SharedList,
            2 => Self::WorkerCopy,
            _ => Self::Move,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Self::SharedMap => "sm",
            Self::SharedList => "sl",
            Self::WorkerCopy => "wc",
            Self::Move => "mv",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum MixedAction {
    Print,
    Charge,
    Cancel,
    Cross,
}

enum MixedPayload<'a> {
    Shared(&'a SendValue),
    Owned(SendValue),
}

fn mixed_payload_len(p: &MixedPayload<'_>) -> usize {
    match p {
        MixedPayload::Shared(v) => shared_len(v),
        MixedPayload::Owned(v) => shared_len(v),
    }
}

fn mixed_move_payload() -> SendValue {
    SendValue::List(vec![SendValue::Int(7); MIXED_MOVE_LEN])
}

fn mixed_body(
    ctx: &mut vis_runtime::worker::WorkerCtx,
    i: usize,
    mode: MixedMode,
    payload: MixedPayload<'_>,
    action: MixedAction,
) -> Result<SendValue, String> {
    match action {
        MixedAction::Print => {
            match (mode, payload) {
                (MixedMode::SharedMap | MixedMode::SharedList, MixedPayload::Shared(v)) => {
                    let n = shared_len(v);
                    ctx.print_line(&format!("{}:{i}:{n}", mode.tag()));
                }
                (MixedMode::WorkerCopy, MixedPayload::Owned(SendValue::List(mut xs))) => {
                    let n0 = xs.len();
                    xs.push(SendValue::Int(i as i64));
                    ctx.print_line(&format!("{}:{i}:{n0}:{}", mode.tag(), xs.len()));
                }
                (MixedMode::Move, MixedPayload::Owned(SendValue::List(mut xs))) => {
                    assert_eq!(
                        xs.len(),
                        MIXED_MOVE_LEN,
                        "move payload len: {} != {MIXED_MOVE_LEN}",
                        xs.len()
                    );
                    assert!(
                        xs.iter().all(|v| matches!(v, SendValue::Int(7))),
                        "move payload nội dung đổi"
                    );
                    xs.push(SendValue::Int(i as i64));
                    ctx.print_line(&format!("{}:{i}:{}", mode.tag(), xs.len()));
                }
                _ => panic!("print mixed: mode/payload lệch: {mode:?}"),
            }
            ctx.charge(1)?;
            Ok(SendValue::Int(i as i64))
        }
        MixedAction::Charge => {
            std::hint::black_box(mixed_payload_len(&payload));
            ctx.charge(1)?;
            Ok(SendValue::Unit)
        }
        MixedAction::Cancel => {
            ctx.check_cancel()?;
            std::thread::sleep(Duration::from_micros(MIXED_CANCEL_SLEEP_US));
            std::hint::black_box(mixed_payload_len(&payload));
            ctx.check_cancel()?;
            Ok(SendValue::Unit)
        }
        MixedAction::Cross => {
            std::hint::black_box(mixed_payload_len(&payload));
            let cur = match ctx.get("counter") {
                Some(SendValue::Int(v)) => *v,
                _ => 0,
            };
            ctx.set("counter", SendValue::Int(cur + 1));
            Ok(SendValue::Unit)
        }
    }
}

fn mixed_spawn(
    pool: &mut WorkerPool,
    i: usize,
    payloads: &MixedPayloads,
    action: MixedAction,
) -> Result<vis_runtime::worker::TaskHandle, WorkerError> {
    let mode = MixedMode::of(i);
    match mode {
        MixedMode::SharedMap => {
            let payload = Arc::clone(&payloads.map);
            pool.spawn_shared(payload, move |ctx, v| {
                mixed_body(ctx, i, mode, MixedPayload::Shared(v), action)
            })
        }
        MixedMode::SharedList => {
            let payload = Arc::clone(&payloads.list);
            pool.spawn_shared(payload, move |ctx, v| {
                mixed_body(ctx, i, mode, MixedPayload::Shared(v), action)
            })
        }
        MixedMode::WorkerCopy => {
            let payload = Arc::clone(&payloads.copy);
            pool.spawn_worker_copy(payload, move |ctx, v| {
                mixed_body(ctx, i, mode, MixedPayload::Owned(v), action)
            })
        }
        MixedMode::Move => {
            let payload = mixed_move_payload();
            pool.spawn_with_payload(payload, move |ctx, v| {
                mixed_body(ctx, i, mode, MixedPayload::Owned(v), action)
            })
        }
    }
}

struct MixedPayloads {
    map: Arc<SendValue>,
    list: Arc<SendValue>,
    copy: Arc<SendValue>,
}

fn mixed_expected_print() -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(MIXED_N * 20);
    for i in 0..MIXED_N {
        match MixedMode::of(i) {
            MixedMode::SharedMap => writeln!(s, "sm:{i}:1000").unwrap(),
            MixedMode::SharedList => writeln!(s, "sl:{i}:10000").unwrap(),
            MixedMode::WorkerCopy => writeln!(s, "wc:{i}:10000:10001").unwrap(),
            MixedMode::Move => writeln!(s, "mv:{i}:257").unwrap(),
        }
    }
    s
}

#[derive(Default, Clone, Copy)]
struct MixedChecks {
    print: usize,
    budget: usize,
    cancel: usize,
    cross: usize,
    arc: usize,
    payload: usize,
}

impl MixedChecks {
    fn delta(self, prev: Self) -> Self {
        Self {
            print: self.print - prev.print,
            budget: self.budget - prev.budget,
            cancel: self.cancel - prev.cancel,
            cross: self.cross - prev.cross,
            arc: self.arc - prev.arc,
            payload: self.payload - prev.payload,
        }
    }
}

fn mixed_assert_payloads(p: &MixedPayloads, round: usize, tag: &str) {
    assert_eq!(
        Arc::strong_count(&p.map),
        1,
        "vòng {round}/{tag}: Arc map rò"
    );
    assert_eq!(
        Arc::strong_count(&p.list),
        1,
        "vòng {round}/{tag}: Arc list rò"
    );
    assert_eq!(
        Arc::strong_count(&p.copy),
        1,
        "vòng {round}/{tag}: Arc copy rò"
    );
    assert_shared_payload(&p.map, round, tag);
    assert_shared_payload(&p.list, round, tag);
    assert_shared_payload(&p.copy, round, tag);
}

struct MixedRoundSample {
    secs: f64,
    print_s: f64,
    budget_s: f64,
    cancel_s: f64,
    cross_s: f64,
    rss_kb: u64,
    hwm_kb: u64,
    threads: u64,
    peak_threads: u64,
}

#[allow(clippy::too_many_lines)]
fn mixed_round(
    round: usize,
    payloads: &MixedPayloads,
    expected: &str,
    checks: &mut MixedChecks,
) -> MixedRoundSample {
    let t_round = Instant::now();
    let mut peak_threads = status_kb("Threads:");

    // A. Print merge + budget atomic: W=1 == W=4 và == expected (byte-identical).
    let t_phase = Instant::now();
    let mut outs = Vec::with_capacity(2);
    for w in [1usize, 4] {
        let budget = Arc::new(SharedBudget::new((MIXED_N + 1_000) as u64, 256));
        let mut pool =
            WorkerPool::with_budget(w, env(), Arc::clone(&budget)).expect("pool print mixed");
        for i in 0..MIXED_N {
            mixed_spawn(&mut pool, i, payloads, MixedAction::Print).expect("spawn print mixed");
        }
        peak_threads = peak_threads.max(status_kb("Threads:"));
        let r = pool.join().expect("join print mixed");
        let snap = r.budget.expect("budget print mixed");
        assert_eq!(
            r.outcomes.len(),
            MIXED_N,
            "vòng {round}: outcomes print W={w}"
        );
        assert_eq!(snap.used, MIXED_N as i64, "vòng {round}: used print W={w}");
        assert_eq!(
            snap.used + snap.remaining,
            snap.limit,
            "vòng {round}: used+remaining print W={w}"
        );
        assert_eq!(snap.remaining, 1_000, "vòng {round}: remaining print W={w}");
        assert!(!snap.exhausted, "vòng {round}: print W={w} không được cạn");
        checks.print += 1;
        checks.budget += 1;
        checks.arc += 1;
        checks.payload += 3;
        mixed_assert_payloads(payloads, round, "print");
        outs.push(r.output);
    }
    assert_eq!(
        outs[0], outs[1],
        "vòng {round}: print W=1/W=4 phải byte-identical"
    );
    assert_eq!(
        outs[0], expected,
        "vòng {round}: print không khớp expected (spawn order/mode)"
    );
    assert!(
        outs[0].starts_with("sm:0:1000\n"),
        "vòng {round}: dòng in đầu"
    );
    assert_eq!(outs[0].lines().count(), MIXED_N, "vòng {round}: số dòng in");
    let print_s = t_phase.elapsed().as_secs_f64();

    // B. Budget cạn: 10k task, limit = N/2, mỗi task charge 1.
    let t_phase = Instant::now();
    for w in [1usize, 4] {
        let budget = Arc::new(SharedBudget::new((MIXED_N / 2) as u64, 256));
        let mut pool =
            WorkerPool::with_budget(w, env(), Arc::clone(&budget)).expect("pool budget mixed");
        for i in 0..MIXED_N {
            mixed_spawn(&mut pool, i, payloads, MixedAction::Charge).expect("spawn budget mixed");
        }
        peak_threads = peak_threads.max(status_kb("Threads:"));
        let r = pool.join().expect("join budget mixed");
        let snap = r.budget.expect("budget mixed");
        assert_eq!(
            r.outcomes.len(),
            MIXED_N,
            "vòng {round}: outcomes budget W={w}"
        );
        assert_eq!(
            snap.used,
            (MIXED_N / 2) as i64,
            "vòng {round}: used budget W={w}"
        );
        assert_eq!(snap.remaining, 0, "vòng {round}: remaining budget W={w}");
        assert_eq!(
            snap.used + snap.remaining,
            snap.limit,
            "vòng {round}: used+remaining budget W={w}"
        );
        assert!(snap.exhausted, "vòng {round}: budget W={w} phải cạn");
        let tong: i64 = r.outcomes.iter().map(|o| o.budget_used).sum();
        assert_eq!(tong, snap.used, "vòng {round}: budget used W={w}");
        assert!(
            r.outcomes.iter().any(|o| o.result.is_err()),
            "vòng {round}: budget W={w} phải có task lỗi"
        );
        checks.budget += 1;
        checks.arc += 1;
        checks.payload += 3;
        mixed_assert_payloads(payloads, round, "budget");
    }
    let budget_s = t_phase.elapsed().as_secs_f64();

    // C. Cancel: huỷ nửa handle, join đủ, Arc về 1, task huỷ trả Err.
    let t_phase = Instant::now();
    for w in [1usize, 4] {
        let mut pool = WorkerPool::new(w, env()).expect("pool cancel mixed");
        let mut handles = Vec::with_capacity(MIXED_N);
        for i in 0..MIXED_N {
            handles.push(
                mixed_spawn(&mut pool, i, payloads, MixedAction::Cancel)
                    .expect("spawn cancel mixed"),
            );
        }
        for h in handles.iter().step_by(2) {
            h.cancel();
        }
        peak_threads = peak_threads.max(status_kb("Threads:"));
        let r = pool.join().expect("join cancel mixed");
        assert_eq!(
            r.outcomes.len(),
            MIXED_N,
            "vòng {round}: outcomes cancel W={w}"
        );
        let cancelled = r.outcomes.iter().filter(|o| o.cancelled).count();
        if w == 1 {
            assert_eq!(
                cancelled,
                MIXED_N / 2,
                "vòng {round}: W=1 phải huỷ đúng nửa"
            );
        } else {
            assert!(
                cancelled <= MIXED_N / 2,
                "vòng {round}: W=4 huỷ quá nửa: {cancelled}"
            );
        }
        // `cancelled` là **yêu cầu huỷ tại thời điểm outcome**, không phải bằng chứng
        // task dừng: W>1 có thể đặt cờ sau `check_cancel` cuối và task vẫn trả Ok.
        // Chỉ W=1 mới đảm bảo task bị huỷ trước khi chạy (đếm đúng nửa ở trên).
        checks.cancel += 1;
        checks.arc += 1;
        checks.payload += 3;
        mixed_assert_payloads(payloads, round, "cancel");
    }
    let cancel_s = t_phase.elapsed().as_secs_f64();

    // D. Cross-write: W=1 cộng dồn env chung; W=4 vẫn Err và đủ 10k task id.
    let t_phase = Instant::now();
    let mut pool = WorkerPool::new(1, env()).expect("pool cross mixed w1");
    for i in 0..MIXED_N {
        mixed_spawn(&mut pool, i, payloads, MixedAction::Cross).expect("spawn cross mixed w1");
    }
    peak_threads = peak_threads.max(status_kb("Threads:"));
    let r = pool.join().expect("join cross mixed w1");
    assert_eq!(
        r.outcomes.len(),
        MIXED_N,
        "vòng {round}: outcomes cross W=1"
    );
    assert_eq!(
        r.env.get("counter"),
        Some(&SendValue::Int(MIXED_N as i64)),
        "vòng {round}: W=1 counter phải cộng dồn đủ"
    );
    checks.cross += 1;
    checks.arc += 1;
    checks.payload += 3;
    mixed_assert_payloads(payloads, round, "cross-w1");

    let mut pool = WorkerPool::new(4, env()).expect("pool cross mixed w4");
    for i in 0..MIXED_N {
        mixed_spawn(&mut pool, i, payloads, MixedAction::Cross).expect("spawn cross mixed w4");
    }
    peak_threads = peak_threads.max(status_kb("Threads:"));
    match pool.join() {
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            assert_eq!(vars, vec!["counter".to_string()], "vòng {round}: vars");
            assert_eq!(tasks.len(), MIXED_N, "vòng {round}: tasks cross-write");
        }
        other => panic!("vòng {round}: cross-write W=4 phải Err: {other:?}"),
    }
    checks.cross += 1;
    checks.arc += 1;
    checks.payload += 3;
    mixed_assert_payloads(payloads, round, "cross-w4");
    let cross_s = t_phase.elapsed().as_secs_f64();

    MixedRoundSample {
        secs: t_round.elapsed().as_secs_f64(),
        print_s,
        budget_s,
        cancel_s,
        cross_s,
        rss_kb: status_kb("VmRSS:"),
        hwm_kb: status_kb("VmHWM:"),
        threads: status_kb("Threads:"),
        peak_threads,
    }
}

fn mixed_median_u64(xs: &[u64]) -> u64 {
    let mut v = xs.to_vec();
    v.sort_unstable();
    v[v.len() / 2]
}

#[allow(clippy::too_many_arguments)]
fn mixed_log_batch(
    batch: usize,
    rows: usize,
    total_rounds: usize,
    task_total: usize,
    secs: f64,
    checks: MixedChecks,
    partial: bool,
    rss0: u64,
    threads0: u64,
    rss: u64,
    hwm: u64,
    threads: u64,
    peak_threads: u64,
) {
    assert_eq!(checks.print, rows * 2, "batch {batch}: print checks");
    assert_eq!(checks.budget, rows * 4, "batch {batch}: budget checks");
    assert_eq!(checks.cancel, rows * 2, "batch {batch}: cancel checks");
    assert_eq!(checks.cross, rows * 2, "batch {batch}: cross checks");
    assert_eq!(checks.arc, rows * 8, "batch {batch}: arc checks");
    assert_eq!(checks.payload, rows * 24, "batch {batch}: payload checks");
    assert!(
        threads <= threads0 + 1,
        "batch {batch}: rò worker thread: {threads0} -> {threads}"
    );
    assert!(
        rss.saturating_sub(rss0) < 256 * 1024,
        "batch {batch}: RSS tăng quá 256 MB: {rss0} -> {rss}"
    );
    println!(
        "MIXED_BATCH\tbatch={batch}\trounds={rows}\ttotal_rounds={total_rounds}\ttasks={task_total}\tsecs={secs:.2}\trss0_kb={rss0}\trss_kb={rss}\thwm_kb={hwm}\tthreads0={threads0}\tthreads={threads}\tpeak_threads={peak_threads}\tprint={}\tbudget={}\tcancel={}\tcross={}\tarc={}\tpayload={}\tpartial={partial}",
        checks.print, checks.budget, checks.cancel, checks.cross, checks.arc, checks.payload
    );
}

/// **Mixed soak 30 phút** (mặc định 1 vòng cho gate): xoay `spawn_shared` map1k/list10k,
/// `spawn_worker_copy`, `spawn_with_payload`; W=1/W=4; print/budget/cancel/cross-write;
/// batch 5 vòng ghi RSS/threads, dừng ngay nếu bất biến sai.
#[allow(dead_code)]
fn main() {
    mixed_soak_shared_copy_move_w1_w4();
}

pub fn mixed_soak_shared_copy_move_w1_w4() {
    let target_rounds = mixed_rounds();
    let target_secs = mixed_secs();
    let expected = mixed_expected_print();
    let t_all = Instant::now();
    let rss0 = status_kb("VmRSS:");
    let threads0 = status_kb("Threads:");
    let hwm0 = status_kb("VmHWM:");
    let mut checks = MixedChecks::default();
    let mut batch_checks = MixedChecks::default();
    let mut batch_rss = Vec::new();
    let mut batch_threads = Vec::new();
    let mut batch_secs = Vec::new();
    let mut total_rounds = 0usize;
    let mut batch = 0usize;
    let mut batch_rows = 0usize;
    let mut batch_t0 = Instant::now();
    let mut peak_threads = threads0;
    let mut hwm_peak = hwm0;
    let mut print_s = 0.0;
    let mut budget_s = 0.0;
    let mut cancel_s = 0.0;
    let mut cross_s = 0.0;

    println!(
        "MIXED_SOAK_ENV\ttarget_rounds={target_rounds}\ttarget_secs={target_secs:.0}\tbatch={MIXED_BATCH}\tn={MIXED_N}\tspawns_per_round={}\trss0_kb={rss0}\tthreads0={threads0}",
        MIXED_N * 8
    );

    loop {
        if total_rounds >= target_rounds {
            break;
        }
        // Chỉ dừng theo thời gian ở biên batch (batch luôn đủ 5 vòng khi soak dài).
        if total_rounds > 0
            && total_rounds.is_multiple_of(MIXED_BATCH)
            && t_all.elapsed().as_secs_f64() >= target_secs
        {
            break;
        }

        let payloads = MixedPayloads {
            map: Arc::new(shared_map1k()),
            list: Arc::new(shared_list10k()),
            copy: Arc::new(shared_list10k()),
        };
        let sample = mixed_round(total_rounds, &payloads, &expected, &mut checks);
        drop(payloads);
        total_rounds += 1;
        batch_rows += 1;
        print_s += sample.print_s;
        budget_s += sample.budget_s;
        cancel_s += sample.cancel_s;
        cross_s += sample.cross_s;
        peak_threads = peak_threads.max(sample.peak_threads);
        hwm_peak = hwm_peak.max(sample.hwm_kb);
        println!(
            "MIXED_ROUND\tround={}\tsecs={:.3}\tprint_s={:.3}\tbudget_s={:.3}\tcancel_s={:.3}\tcross_s={:.3}\trss_kb={}\thwm_kb={}\tthreads={}\tpeak_threads={}",
            total_rounds - 1,
            sample.secs,
            sample.print_s,
            sample.budget_s,
            sample.cancel_s,
            sample.cross_s,
            sample.rss_kb,
            sample.hwm_kb,
            sample.threads,
            sample.peak_threads
        );

        if total_rounds.is_multiple_of(MIXED_BATCH) {
            batch += 1;
            let rss = status_kb("VmRSS:");
            let threads = status_kb("Threads:");
            let hwm = status_kb("VmHWM:");
            hwm_peak = hwm_peak.max(hwm);
            let secs = batch_t0.elapsed().as_secs_f64();
            let delta = checks.delta(batch_checks);
            mixed_log_batch(
                batch,
                batch_rows,
                total_rounds,
                total_rounds * MIXED_N * 8,
                secs,
                delta,
                false,
                rss0,
                threads0,
                rss,
                hwm,
                threads,
                peak_threads,
            );
            batch_checks = checks;
            batch_rss.push(rss);
            batch_threads.push(threads);
            batch_secs.push(secs);
            batch_rows = 0;
            batch_t0 = Instant::now();

            // Rò dài hạn: median 10 batch cuối so median batch 5..15 (bỏ warmup).
            if batch_rss.len() >= 20 {
                let ref_med = mixed_median_u64(&batch_rss[5..15]);
                let tail_med = mixed_median_u64(&batch_rss[batch_rss.len() - 10..]);
                assert!(
                    tail_med.saturating_sub(ref_med) < 64 * 1024,
                    "RSS 10 batch cuối tăng quá 64 MB so plateau: {ref_med} -> {tail_med}"
                );
            }
        }
    }

    // Batch lẻ cuối (mặc định ngắn, hoặc dừng giữa batch do hết vòng).
    if batch_rows > 0 {
        batch += 1;
        let rss = status_kb("VmRSS:");
        let threads = status_kb("Threads:");
        let hwm = status_kb("VmHWM:");
        hwm_peak = hwm_peak.max(hwm);
        let secs = batch_t0.elapsed().as_secs_f64();
        let delta = checks.delta(batch_checks);
        mixed_log_batch(
            batch,
            batch_rows,
            total_rounds,
            total_rounds * MIXED_N * 8,
            secs,
            delta,
            true,
            rss0,
            threads0,
            rss,
            hwm,
            threads,
            peak_threads,
        );
        batch_rss.push(rss);
        batch_threads.push(threads);
        batch_secs.push(secs);
    }

    let secs_all = t_all.elapsed().as_secs_f64();
    let rss1 = status_kb("VmRSS:");
    let hwm_final = status_kb("VmHWM:");
    let threads1 = status_kb("Threads:");
    let rss_min = batch_rss.iter().copied().min().unwrap_or(rss1);
    let rss_max = batch_rss.iter().copied().max().unwrap_or(rss1);
    let rss_med = mixed_median_u64(&batch_rss);
    let threads_peak_batch = batch_threads.iter().copied().max().unwrap_or(threads1);
    let secs_batch_med = {
        let mut xs = batch_secs.clone();
        xs.sort_by(|a, b| a.total_cmp(b));
        if xs.is_empty() {
            0.0
        } else {
            xs[xs.len() / 2]
        }
    };
    assert_eq!(checks.print, total_rounds * 2, "tổng print checks");
    assert_eq!(checks.budget, total_rounds * 4, "tổng budget checks");
    assert_eq!(checks.cancel, total_rounds * 2, "tổng cancel checks");
    assert_eq!(checks.cross, total_rounds * 2, "tổng cross checks");
    assert_eq!(checks.arc, total_rounds * 8, "tổng arc checks");
    assert_eq!(checks.payload, total_rounds * 24, "tổng payload checks");
    assert!(
        threads1 <= threads0 + 1,
        "rò worker thread: {threads0} -> {threads1}"
    );
    assert!(
        rss1.saturating_sub(rss0) < 256 * 1024,
        "RSS tăng quá 256 MB (nghi rò): {rss0} -> {rss1}"
    );
    if target_secs >= 1200.0 && target_rounds > MIXED_BATCH {
        assert!(
            secs_all >= target_secs,
            "soak ngắn hơn mục tiêu {target_secs:.0}s: {secs_all:.1}s"
        );
    }

    println!(
        "MIXED_SOAK_SUMMARY\tbatches={batch}\trounds={total_rounds}\ttasks={}\tbatch_size={MIXED_BATCH}\ttarget_secs={target_secs:.0}\tsecs={secs_all:.2}\tsecs_batch_med={secs_batch_med:.2}\tprint_s={print_s:.2}\tbudget_s={budget_s:.2}\tcancel_s={cancel_s:.2}\tcross_s={cross_s:.2}\trss0_kb={rss0}\trss1_kb={rss1}\trss_min_kb={rss_min}\trss_med_kb={rss_med}\trss_max_kb={rss_max}\thwm0_kb={hwm0}\thwm_peak_kb={hwm_peak}\thwm_final_kb={hwm_final}\tthreads0={threads0}\tthreads1={threads1}\tthreads_peak_sample={peak_threads}\tthreads_peak_batch={threads_peak_batch}\tprint_checks={}\tbudget_checks={}\tcancel_checks={}\tcross_checks={}\tarc_checks={}\tpayload_checks={}",
        total_rounds * MIXED_N * 8,
        checks.print,
        checks.budget,
        checks.cancel,
        checks.cross,
        checks.arc,
        checks.payload
    );
}
