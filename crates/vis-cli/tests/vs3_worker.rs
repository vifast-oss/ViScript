// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b (W5, VD-271 B1–B7)** — worker pool opt-in/test-only:
//! `TaskId`/proxy completion, snapshot Env, print merge theo spawn order, budget atomic + chunk,
//! cross-write bị từ chối W>1, cancel/join. W=1 giữ shared env; W>1 snapshot.
//!
//! Đây là mô hình worker trong `vis-interp` (chưa nối evaluator; W1 hook). Không đổi ABI
//! `Value::Task(Rc<Task>)` — `TaskId` là handle mới, `Show`/API của `Value` giữ nguyên.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_interp::rt::fee::SharedBudget;
use vis_interp::rt::{Task, TrangThai};
use vis_interp::value::worker::{
    EnvMap, SendValue, TaskWaitBoard, WaitError, WorkerError, WorkerPool,
};
use vis_interp::value::{TaskId, Value};
use vis_syntax::Span;

fn env() -> EnvMap {
    let mut m = EnvMap::new();
    m.insert("base".into(), SendValue::Int(10));
    m.insert(
        "data".into(),
        SendValue::List((0..8).map(SendValue::Int).collect()),
    );
    m
}

fn base_of(ctx: &vis_interp::value::worker::WorkerCtx) -> i64 {
    match ctx.get("base") {
        Some(SendValue::Int(v)) => *v,
        _ => -1,
    }
}

fn data_len(ctx: &vis_interp::value::worker::WorkerCtx) -> usize {
    match ctx.get("data") {
        Some(SendValue::List(xs)) => xs.len(),
        _ => 0,
    }
}

fn run_safe(w: usize) -> (String, Vec<i64>, Vec<TaskId>) {
    let mut pool = WorkerPool::new(w, env()).expect("pool");
    for i in 0..8usize {
        pool.spawn(move |ctx| {
            let base = base_of(ctx);
            ctx.print_line(&format!("t{i}:base={base}"));
            ctx.print_line(&format!("t{i}:data={}", data_len(ctx)));
            Ok(SendValue::Int(base * 100 + i as i64))
        })
        .expect("spawn");
    }
    let r = pool.join().expect("join");
    let results = r
        .outcomes
        .iter()
        .map(|o| match &o.result {
            Ok(SendValue::Int(i)) => *i,
            _ => -1,
        })
        .collect();
    (r.output, results, r.outcomes.iter().map(|o| o.id).collect())
}

/// W=1/W=2/W=4 cùng output byte-identical, cùng result; TaskId theo thứ tự spawn.
#[test]
fn w1_w2_w4_cung_output_byte_identical() {
    let (o1, r1, ids1) = run_safe(1);
    let (o2, r2, ids2) = run_safe(2);
    let (o4, r4, ids4) = run_safe(4);
    assert_eq!(o1, o2, "W=1 vs W=2 output");
    assert_eq!(o1, o4, "W=1 vs W=4 output");
    assert_eq!(r1, r2);
    assert_eq!(r1, r4);
    assert_eq!(ids1, ids2);
    assert_eq!(ids1, ids4);
    assert_eq!(ids4, (0..8).map(TaskId::from_raw).collect::<Vec<_>>());
    assert!(o4.starts_with("t0:base=10\nt0:data=8\n"));
    assert_eq!(o4.lines().count(), 16);
}

/// B2: W=1 shared env (main đổi sau spawn thì task thấy); W>1 snapshot tại spawn.
#[test]
fn w1_shared_env_va_wn_snapshot() {
    let mut p1 = WorkerPool::new(1, env()).expect("pool W1");
    p1.spawn(|ctx| {
        ctx.print_line(&format!("base={}", base_of(ctx)));
        Ok(SendValue::Unit)
    })
    .unwrap();
    p1.set_env("base", SendValue::Int(99));
    let r1 = p1.join().unwrap();
    assert_eq!(r1.output, "base=99\n", "W=1 phải shared env");

    let mut p4 = WorkerPool::new(4, env()).expect("pool W4");
    p4.spawn(|ctx| {
        ctx.print_line(&format!("base={}", base_of(ctx)));
        Ok(SendValue::Unit)
    })
    .unwrap();
    p4.set_env("base", SendValue::Int(99));
    let r4 = p4.join().unwrap();
    assert_eq!(r4.output, "base=10\n", "W>1 phải snapshot tại spawn");
}

/// B1/B2: W=1 cho ghi top-level `var` (shared env); W>1 từ chối rõ, không treo.
#[test]
fn cross_write_w1_allowed_wn_refused() {
    let mut p1 = WorkerPool::new(1, env()).expect("pool W1");
    for i in 0..8usize {
        p1.spawn(move |ctx| {
            let cur = match ctx.get("counter") {
                Some(SendValue::Int(v)) => *v,
                _ => 0,
            };
            ctx.set("counter", SendValue::Int(cur + 1));
            ctx.print_line(&format!("cw{i}={}", cur + 1));
            Ok(SendValue::Unit)
        })
        .unwrap();
    }
    let r1 = p1.join().unwrap();
    assert_eq!(
        r1.env.get("counter"),
        Some(&SendValue::Int(8)),
        "W=1 shared env phải cộng dồn"
    );
    assert_eq!(r1.output.lines().count(), 8);

    let mut p4 = WorkerPool::new(4, env()).expect("pool W4");
    for _ in 0..8usize {
        p4.spawn(|ctx| {
            let cur = match ctx.get("counter") {
                Some(SendValue::Int(v)) => *v,
                _ => 0,
            };
            ctx.set("counter", SendValue::Int(cur + 1));
            Ok(SendValue::Unit)
        })
        .unwrap();
    }
    match p4.join() {
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            assert_eq!(vars, vec!["counter".to_string()]);
            assert_eq!(tasks.len(), 8);
        }
        other => panic!("W=4 phải từ chối cross-write, nhận {other:?}"),
    }
}

/// B4: budget atomic + chunk reservation; `used + remaining == limit`, không reset.
#[test]
fn budget_atomic_chunk_used_remaining_limit() {
    for w in [1usize, 2, 4] {
        let b = Arc::new(SharedBudget::new(100_000, 128));
        let mut pool = WorkerPool::with_budget(w, env(), Arc::clone(&b)).expect("pool");
        for _ in 0..8 {
            pool.spawn(|ctx| {
                for _ in 0..100 {
                    ctx.charge(7)?;
                }
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        let r = pool.join().unwrap();
        let snap = r.budget.expect("budget snapshot");
        assert_eq!(snap.used, 8 * 100 * 7, "W={w}: used");
        assert_eq!(
            snap.used + snap.remaining,
            snap.limit,
            "W={w}: used + remaining == limit"
        );
        assert!(!snap.exhausted, "W={w}: không được cạn");
        let tong: i64 = r.outcomes.iter().map(|o| o.budget_used).sum();
        assert_eq!(tong, snap.used, "W={w}: outcome used khớp pool");
    }
}

/// B4: cạn pool ⇒ không deadlock; used+remaining vẫn == limit; task lỗi được surface trong outcome.
#[test]
fn budget_exhaustion_khong_deadlock() {
    let b = Arc::new(SharedBudget::new(1_000, 64));
    let mut pool = WorkerPool::with_budget(4, env(), Arc::clone(&b)).expect("pool");
    for _ in 0..4 {
        pool.spawn(|ctx| {
            for _ in 0..500 {
                ctx.charge(1)?;
            }
            Ok(SendValue::Unit)
        })
        .unwrap();
    }
    let r = pool.join().unwrap();
    let snap = r.budget.expect("budget");
    assert!(snap.exhausted, "phải đánh dấu cạn");
    assert_eq!(snap.used + snap.remaining, snap.limit);
    assert!(snap.used <= snap.limit);
    assert!(
        r.outcomes.iter().any(|o| o.result.is_err()),
        "ít nhất một task phải thấy lỗi ngân sách"
    );
}

/// Cancel/join: W=1 huỷ trước join; W=4 task tự kiểm `check_cancel`; không treo.
#[test]
fn cancel_va_join_khong_deadlock() {
    for w in [1usize, 4] {
        let mut pool = WorkerPool::new(w, env()).expect("pool");
        let h = pool
            .spawn(|ctx| {
                for _ in 0..200 {
                    ctx.check_cancel()?;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(SendValue::Unit)
            })
            .unwrap();
        std::thread::sleep(Duration::from_millis(5));
        h.cancel();
        let r = pool.join().unwrap();
        let o = &r.outcomes[0];
        assert!(o.cancelled, "W={w}: task phải bị huỷ");
        assert_eq!(o.id, h.id());
        assert!(o.output.is_empty());
    }
}

/// B5: `Value::Task` ABI/`Show` giữ nguyên; `TaskId` là handle mới, `SendValue` chuyển đổi được.
#[test]
fn value_task_show_va_taskid_khong_doi_abi() {
    let task = Task {
        id: 1,
        than: 0,
        args: vec![],
        ten: "f".into(),
        span: Span::default(),
        trang_thai: Cell::new(TrangThai::Moi),
        kq: RefCell::new(None),
    };
    let v = Value::Task(Rc::new(task));
    assert_eq!(v.show(), "<tác vụ>");
    assert!(v.to_send().is_err(), "Task không Sendable (VS0508)");

    let goc = Value::List(vec![
        Value::Int(1),
        Value::Str("x".into()),
        Value::some(Value::Int(2)),
    ]);
    let send = goc.to_send().expect("Sendable");
    assert_eq!(send.to_value().show(), goc.show());
    assert!(Value::Cap {
        kind: "Dir".into(),
        root: "./x".into(),
        write: false
    }
    .to_send()
    .is_err());
    assert_eq!(format!("{}", TaskId::from_raw(7)), "#7");
}

/// Proxy/merge: task 0 chậm (completion đến sau) nhưng output vẫn theo **spawn order**.
#[test]
fn merge_spawn_order_khi_completion_lech() {
    let mut pool = WorkerPool::new(4, env()).expect("pool");
    for i in 0..4usize {
        pool.spawn(move |ctx| {
            if i == 0 {
                std::thread::sleep(Duration::from_millis(10));
            }
            ctx.print_line(&format!("t{i}"));
            Ok(SendValue::Int(i as i64))
        })
        .unwrap();
    }
    let r = pool.join().unwrap();
    assert_eq!(r.output, "t0\nt1\nt2\nt3\n");
    let ids: Vec<u64> = r.outcomes.iter().map(|o| o.id.as_u64()).collect();
    assert_eq!(ids, vec![0, 1, 2, 3]);
}

/// **VS3-3b payload lớn**: `spawn_worker_copy` (clone trong worker) và
/// `spawn_with_payload` (move) giữ output W=1==W=4 và budget atomic.
#[test]
fn worker_copy_va_move_payload_w1_w4_parity() {
    let mut outs = Vec::new();
    for w in [1usize, 4] {
        let template = Arc::new(SendValue::List((0..1_000).map(SendValue::Int).collect()));
        let b = Arc::new(SharedBudget::new(100_000, 256));
        let mut pool = WorkerPool::with_budget(w, env(), Arc::clone(&b)).expect("pool");
        for i in 0..8usize {
            let payload = Arc::clone(&template);
            pool.spawn_worker_copy(payload, move |ctx, p| {
                let n = match &p {
                    SendValue::List(xs) => xs.len(),
                    _ => 0,
                };
                ctx.print_line(&format!("wc{i}:{n}"));
                for _ in 0..10 {
                    ctx.charge(1)?;
                }
                Ok(SendValue::Int(n as i64))
            })
            .unwrap();
        }
        let r = pool.join().expect("join");
        let snap = r.budget.expect("budget");
        assert_eq!(snap.used, 80);
        assert_eq!(snap.used + snap.remaining, snap.limit);
        assert_eq!(r.outcomes.len(), 8);
        outs.push(r.output);
    }
    assert_eq!(outs[0], outs[1], "worker-copy W=1 vs W=4 output");
    assert!(outs[0].starts_with("wc0:1000\n"));
    assert_eq!(outs[0].lines().count(), 8);

    // Move payload: không copy producer; W=1/W=4 cùng output.
    let mut moved = Vec::new();
    for w in [1usize, 4] {
        let mut pool = WorkerPool::new(w, env()).expect("pool move");
        for i in 0..4usize {
            let p = SendValue::Record(
                "R".to_string(),
                vec![(format!("f{i}"), SendValue::Int(i as i64))],
            );
            pool.spawn_with_payload(p, move |ctx, p| {
                let n = match &p {
                    SendValue::Record(_, fs) => fs.len(),
                    _ => 0,
                };
                ctx.print_line(&format!("mv{i}:{n}"));
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        moved.push(pool.join().expect("join move").output);
    }
    assert_eq!(moved[0], moved[1], "move W=1 vs W=4 output");
    assert!(moved[0].starts_with("mv0:1\n"));
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

/// Stress dài: **5 vòng × 10k task** (50k task/pha), print/cancel/budget/cross-write;
/// đo thời gian/RSS/thread theo vòng; join trả về, không deadlock, không rò worker thread.
#[test]
fn stress_dai_5x10k_cancel_budget_print_cross() {
    const ROUNDS: usize = 5;
    const N: usize = 10_000;
    let t0 = Instant::now();
    let rss0 = status_kb("VmRSS:");
    let threads0 = status_kb("Threads:");
    let mut print_s = 0.0;
    let mut cancel_s = 0.0;
    let mut budget_s = 0.0;
    let mut cross_s = 0.0;
    let mut rss_rounds = Vec::with_capacity(ROUNDS);

    for round in 0..ROUNDS {
        let t_round = Instant::now();

        // Print merge 10k: W=1 == W=4 byte-identical, outcomes đủ.
        let t_phase = Instant::now();
        let mut outs = Vec::new();
        for w in [1usize, 4] {
            let mut pool = WorkerPool::new(w, env()).expect("pool print");
            for i in 0..N {
                pool.spawn(move |ctx| {
                    ctx.print_line(&format!("t{i}"));
                    Ok(SendValue::Unit)
                })
                .unwrap();
            }
            let r = pool.join().expect("join print");
            assert_eq!(r.outcomes.len(), N);
            outs.push(r.output);
        }
        assert_eq!(outs[0], outs[1], "vòng {round}: print W=1/W=4");
        assert_eq!(outs[0].lines().count(), N);
        print_s += t_phase.elapsed().as_secs_f64();

        // Cancel nửa 10k ở W=1/W=4; task tự kiểm cancel trước/sau sleep.
        let t_phase = Instant::now();
        for w in [1usize, 4] {
            let mut pool = WorkerPool::new(w, env()).expect("pool cancel");
            let mut handles = Vec::with_capacity(N);
            for i in 0..N {
                let h = pool
                    .spawn(move |ctx| {
                        ctx.check_cancel()?;
                        std::thread::sleep(Duration::from_micros(200));
                        ctx.check_cancel()?;
                        if i % 2 == 0 {
                            ctx.print_line(&format!("c{i}"));
                        }
                        Ok(SendValue::Unit)
                    })
                    .unwrap();
                handles.push(h);
            }
            for h in handles.iter().step_by(2) {
                h.cancel();
            }
            let r = pool.join().expect("join cancel");
            assert_eq!(r.outcomes.len(), N);
            // Stress chỉ cần join đủ + không deadlock; ca cancel ngữ nghĩa nằm ở `vs3_cancel`.
            // Không bắt buộc có task bị huỷ: dưới tải, task ngắn có thể xong trước khi cancel.
            assert!(
                r.outcomes.iter().filter(|o| o.cancelled).count() <= N / 2,
                "vòng {round}: không thể huỷ quá nửa ở W={w}"
            );
        }
        cancel_s += t_phase.elapsed().as_secs_f64();

        // Budget cạn: 10k task cùng pool 5k đơn vị; không deadlock, accounting đúng.
        let t_phase = Instant::now();
        let b = Arc::new(SharedBudget::new((N / 2) as u64, 256));
        let mut pool = WorkerPool::with_budget(4, env(), Arc::clone(&b)).expect("pool budget");
        for _ in 0..N {
            pool.spawn(|ctx| {
                ctx.charge(1)?;
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        let r = pool.join().expect("join budget");
        let snap = r.budget.expect("budget");
        assert_eq!(r.outcomes.len(), N);
        assert_eq!(snap.used, (N / 2) as i64, "vòng {round}: dùng đúng pool");
        assert_eq!(snap.remaining, 0, "vòng {round}: cạn đúng 0");
        assert_eq!(snap.used + snap.remaining, snap.limit);
        assert!(snap.exhausted);
        budget_s += t_phase.elapsed().as_secs_f64();

        // Cross-write 10k task W=4: refused, không deadlock, đủ danh sách task.
        let t_phase = Instant::now();
        let mut pool = WorkerPool::new(4, env()).expect("pool cross");
        for i in 0..N {
            pool.spawn(move |ctx| {
                ctx.set("counter", SendValue::Int(i as i64));
                Ok(SendValue::Unit)
            })
            .unwrap();
        }
        match pool.join() {
            Err(WorkerError::CrossWrite { vars, tasks }) => {
                assert_eq!(vars, vec!["counter".to_string()]);
                assert_eq!(tasks.len(), N);
            }
            other => panic!("vòng {round}: stress cross-write phải refused: {other:?}"),
        }
        cross_s += t_phase.elapsed().as_secs_f64();

        let round_s = t_round.elapsed().as_secs_f64();
        let rss = status_kb("VmRSS:");
        let threads = status_kb("Threads:");
        rss_rounds.push(rss);
        println!(
            "STRESS_DAI\tround={round}\ttasks={N}\tsecs={round_s:.2}\trss_kb={rss}\tthreads={threads}"
        );
    }

    let rss1 = status_kb("VmRSS:");
    let hwm = status_kb("VmHWM:");
    let threads1 = status_kb("Threads:");
    let secs = t0.elapsed().as_secs_f64();
    println!(
        "STRESS_DAI_SUMMARY\trounds={ROUNDS}\ttasks_per_round={N}\ttotal_tasks={}\tsecs={secs:.2}\tprint_s={print_s:.2}\tcancel_s={cancel_s:.2}\tbudget_s={budget_s:.2}\tcross_s={cross_s:.2}\trss0_kb={rss0}\trss1_kb={rss1}\thwm_kb={hwm}\tthreads0={threads0}\tthreads1={threads1}\trss_rounds={rss_rounds:?}",
        ROUNDS * N
    );
    assert!(
        threads1 <= threads0 + 1,
        "worker thread rò: {threads0} -> {threads1}"
    );
    assert!(
        rss1.saturating_sub(rss0) < 256 * 1024,
        "RSS tăng quá 256 MB (nghi rò): {rss0} -> {rss1}"
    );
    assert!(secs < 120.0, "stress quá chậm: {secs:.2}s");
}

/// **VS3-3b/W5 prototype**: `Value` ↔ `ProtoValue` (Task = `TaskId`), `Show`/eq giữ nguyên.
#[test]
fn taskid_proto_value_show_eq_roundtrip() {
    use vis_interp::value::worker::ProtoValue;
    use vis_interp::value::Name;

    let vals = vec![
        Value::Int(7),
        Value::Str("x".into()),
        Value::List(vec![Value::Int(1), Value::Bool(true)]),
        Value::Map(vec![
            (Value::Str("k".into()), Value::Int(1)),
            (Value::Str("k2".into()), Value::List(vec![])),
        ]),
        Value::Record("R".to_string(), vec![("a".to_string(), Value::Int(1))]),
        Value::Enum(Name::new("E"), Name::new("A"), vec![Value::Int(1)]),
        Value::Range(1, 5, false),
    ];
    for v in &vals {
        let p = ProtoValue::from_value(v).expect("from_value");
        assert_eq!(p.show(), v.show(), "Show phải khớp: {v:?}");
        let back = p.to_value().expect("to_value");
        // `Range` là giá trị nội bộ: `Value::eq_value` luôn false, chỉ cần round-trip.
        if !matches!(v, Value::Range(..)) {
            assert!(back.eq_value(v), "eq_value phải khớp: {v:?}");
        }
    }

    let task = Task {
        id: 1,
        than: 0,
        args: vec![],
        ten: "f".into(),
        span: Span::default(),
        trang_thai: Cell::new(TrangThai::Moi),
        kq: RefCell::new(None),
    };
    let task_v = Value::Task(Rc::new(task));
    assert!(
        ProtoValue::from_value(&task_v).is_err(),
        "Rc<Task> phải qua registry, không tự bịa TaskId"
    );
    let id = ProtoValue::Task(TaskId::from_raw(42));
    assert_eq!(id.show(), "<tác vụ>");
    assert_eq!(id.task_id(), Some(TaskId::from_raw(42)));
    assert!(!id.eq_value(&id), "task không bao giờ bằng nhau");
    assert!(id.to_value().is_err(), "TaskId phải qua proxy/registry");
    assert_eq!(ProtoValue::List(vec![id]).show(), "[<tác vụ>]");
}

/// Proxy `TaskWaitBoard`: await theo `TaskId` ở W=1/W=4, cancel/join, timeout không treo.
#[test]
fn taskid_proxy_await_w1_w4() {
    let mut outs = Vec::new();
    for w in [1usize, 4] {
        let board = Arc::new(TaskWaitBoard::new());
        let mut pool = WorkerPool::new(w, env()).expect("pool");
        pool.attach_wait_board(Arc::clone(&board));
        let mut ids = Vec::new();
        for i in 0..64i64 {
            let h = pool
                .spawn(move |_ctx| {
                    std::thread::sleep(Duration::from_micros(100));
                    Ok(SendValue::Int(i))
                })
                .unwrap();
            ids.push(h.id());
        }
        if w > 1 {
            // W>1: await được khi pool còn chạy (worker publish ngay khi task xong).
            let r = board
                .wait_timeout(ids[0], Duration::from_secs(5))
                .expect("await id 0");
            assert_eq!(r.expect("task 0"), SendValue::Int(0));
        }
        let r = pool.join().expect("join");
        assert_eq!(r.outcomes.len(), 64);
        for (i, id) in ids.iter().enumerate() {
            let v = board.try_take(*id).expect("board có kết quả");
            assert_eq!(v.expect("ok"), SendValue::Int(i as i64));
        }
        assert!(board.try_take(ids[0]).is_none(), "take xong phải bỏ");
        outs.push(r.output);
    }
    assert_eq!(outs[0], outs[1], "W=1/W=4 output");

    // Cancel: W=4 publish từ worker; W=1 publish lúc join.
    for w in [1usize, 4] {
        let board = Arc::new(TaskWaitBoard::new());
        let mut pool = WorkerPool::new(w, env()).expect("pool");
        pool.attach_wait_board(Arc::clone(&board));
        let h = pool
            .spawn(|ctx| {
                std::thread::sleep(Duration::from_millis(2));
                ctx.check_cancel()?;
                Ok(SendValue::Unit)
            })
            .unwrap();
        let id = h.id();
        h.cancel();
        let r = pool.join().expect("join");
        assert!(r.outcomes[0].cancelled, "W={w}: phải cancelled");
        let got = board.try_take(id).expect("board có kết quả huỷ");
        assert!(got.is_err(), "W={w}: await task huỷ phải Err");
    }

    // Timeout: không chờ vô hạn trên id không tồn tại.
    let board = TaskWaitBoard::new();
    assert_eq!(
        board.wait_timeout(TaskId::from_raw(999), Duration::from_millis(10)),
        Err(WaitError::Timeout)
    );
}

/// `spawn_shared`: map1k/list10k read-only, W=1/W=4, shared vs worker-copy/producer;
/// print merge, budget atomic, cross-write vẫn bị từ chối; payload không đổi/không rò Arc.
#[test]
fn spawn_shared_map_list_w1_w4_parity() {
    let map = Arc::new(SendValue::Map(
        (0..1_000)
            .map(|i| (SendValue::Str(format!("k{i}")), SendValue::Int(i as i64)))
            .collect(),
    ));
    let list = Arc::new(SendValue::List(
        (0..10_000).map(|i| SendValue::Int(i as i64)).collect(),
    ));

    for (name, payload) in [("map", &map), ("list", &list)] {
        let mut outs = Vec::new();
        for w in [1usize, 4] {
            for mode in ["shared", "worker", "producer"] {
                let b = Arc::new(SharedBudget::new(10_000, 256));
                let mut pool = WorkerPool::with_budget(w, env(), Arc::clone(&b)).expect("pool");
                for i in 0..16usize {
                    let i = i as i64;
                    match mode {
                        "shared" => {
                            let p = Arc::clone(payload);
                            pool.spawn_shared(p, move |ctx, p| {
                                let len = match p {
                                    SendValue::Map(kv) => kv.len(),
                                    SendValue::List(xs) => xs.len(),
                                    _ => 0,
                                };
                                ctx.print_line(&format!("{name}:{i}:{len}"));
                                ctx.charge(2)?;
                                Ok(SendValue::Int(i))
                            })
                            .unwrap();
                        }
                        "worker" => {
                            let p = Arc::clone(payload);
                            pool.spawn_worker_copy(p, move |ctx, p| {
                                let len = match &p {
                                    SendValue::Map(kv) => kv.len(),
                                    SendValue::List(xs) => xs.len(),
                                    _ => 0,
                                };
                                ctx.print_line(&format!("{name}:{i}:{len}"));
                                ctx.charge(2)?;
                                Ok(SendValue::Int(i))
                            })
                            .unwrap();
                        }
                        "producer" => {
                            let p = (**payload).clone();
                            pool.spawn(move |ctx| {
                                let len = match &p {
                                    SendValue::Map(kv) => kv.len(),
                                    SendValue::List(xs) => xs.len(),
                                    _ => 0,
                                };
                                ctx.print_line(&format!("{name}:{i}:{len}"));
                                ctx.charge(2)?;
                                Ok(SendValue::Int(i))
                            })
                            .unwrap();
                        }
                        _ => unreachable!(),
                    }
                }
                let r = pool.join().expect("join shared parity");
                let snap = r.budget.expect("budget");
                assert_eq!(r.outcomes.len(), 16, "{name} {mode} W={w}");
                assert_eq!(snap.used, 32, "{name} {mode} W={w}: used");
                assert_eq!(snap.used + snap.remaining, snap.limit);
                assert!(!snap.exhausted);
                assert_eq!(Arc::strong_count(payload), 1, "{name} {mode} W={w}: Arc rò");
                outs.push((mode, w, r.output));
            }
        }
        let first = &outs[0].2;
        for (mode, w, out) in &outs {
            assert_eq!(out, first, "{name} {mode} W={w}: print merge");
        }
        assert_eq!(first.lines().count(), 16);
        match &**payload {
            SendValue::Map(kv) => assert_eq!(kv.len(), 1_000),
            SendValue::List(xs) => assert_eq!(xs.len(), 10_000),
            _ => unreachable!(),
        }
    }

    // Cross-write vẫn bị từ chối ở W=4; W=1 ghi env chung như cũ.
    let p = Arc::new(SendValue::List(vec![SendValue::Int(1)]));
    let mut pool = WorkerPool::new(1, env()).expect("pool shared cross w1");
    let p1 = Arc::clone(&p);
    pool.spawn_shared(p1, |ctx, _| {
        ctx.set("counter", SendValue::Int(1));
        Ok(SendValue::Unit)
    })
    .unwrap();
    let r = pool.join().expect("join shared cross w1");
    assert_eq!(r.env.get("counter"), Some(&SendValue::Int(1)));

    let mut pool = WorkerPool::new(4, env()).expect("pool shared cross w4");
    for _ in 0..8 {
        let p = Arc::clone(&p);
        pool.spawn_shared(p, |ctx, _| {
            ctx.set("counter", SendValue::Int(1));
            Ok(SendValue::Unit)
        })
        .unwrap();
    }
    match pool.join() {
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            assert_eq!(vars, vec!["counter".to_string()]);
            assert_eq!(tasks.len(), 8);
        }
        other => panic!("spawn_shared W=4 phải CrossWrite: {other:?}"),
    }
    assert_eq!(Arc::strong_count(&p), 1, "Arc shared rò sau cross-write");
}
