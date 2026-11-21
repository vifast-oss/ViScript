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

// ===========================================================================
// **VS3-3b/W5 hardening** — stress `spawn_shared` read-only nhiều vòng.
//
// Mặc định **10 vòng** để L0/gate không chậm; bản hardening chạy **100 vòng**:
//   `VS3_W5_SHARED_ROUNDS=100 cargo test --release -p vis-cli --test vs3_worker \
//      spawn_shared_stress -- --nocapture --test-threads=1`
//
// Mỗi vòng (N = 10.000 task/pha), payload **xen kẽ** map1k/list10k:
//   A. in + budget + Arc: W=1 và W=4, 10k task, print merge byte-identical,
//      `used + remaining == limit`, `Arc::strong_count == 1`, payload không đổi;
//   B. cancel: W=1/W=4, huỷ nửa số handle, join đủ, không deadlock, Arc về 1;
//   C. cross-write W=4: vẫn `Err(CrossWrite { vars: ["counter"], tasks: 10k })`;
//   D. budget cạn W=4: `used == limit/2`, `remaining == 0`, `used+remaining==limit`.
// Cứ 10 vòng đo median shared/worker-copy/producer cho map1k+list10k, W=1/W=4.
// Chỉ test/đo — không đổi production/ABI/default.
// ===========================================================================

fn shared_rounds() -> usize {
    std::env::var("VS3_W5_SHARED_ROUNDS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(10)
}

const SHARED_N: usize = 10_000;

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

#[inline(never)]
fn shared_cpu_work(iters: u64) -> u64 {
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    for i in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i ^ 0x5deece66d);
        x ^= x >> 29;
    }
    std::hint::black_box(x)
}

fn shared_calibrate(target_ns: f64) -> (u64, f64) {
    let mut iters = 10_000u64;
    for _ in 0..24 {
        let t = Instant::now();
        std::hint::black_box(shared_cpu_work(iters));
        let ns = t.elapsed().as_nanos() as f64;
        if ns >= target_ns {
            return (iters, ns / 1000.0);
        }
        let scale = (target_ns / ns.max(1.0)).clamp(1.2, 4.0);
        iters = ((iters as f64) * scale) as u64 + 1;
    }
    let t = Instant::now();
    std::hint::black_box(shared_cpu_work(iters));
    (iters, t.elapsed().as_nanos() as f64 / 1000.0)
}

fn shared_warm(ms: u64) {
    let start = Instant::now();
    let mut acc = 0u64;
    while start.elapsed().as_millis() < ms as u128 {
        acc ^= shared_cpu_work(100_000);
    }
    std::hint::black_box(acc);
}

fn shared_median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.total_cmp(b));
    xs[xs.len() / 2]
}

const SHARED_BENCH_TASKS: usize = 256;
const SHARED_BENCH_CHARGES: usize = 10;

/// Một lần đo `shared`/`worker`(copy)/`producer`(clone trên producer) cùng 256 task
/// CPU + charge; trả ms. Không lặp ma trận payload khác.
fn shared_bench_run(w: usize, payload: &Arc<SendValue>, mode: &str, cpu: u64) -> f64 {
    let limit = (SHARED_BENCH_TASKS * SHARED_BENCH_CHARGES + 10_000) as u64;
    let budget = Arc::new(SharedBudget::new(limit, 256));
    let t0 = Instant::now();
    let mut pool =
        WorkerPool::with_budget(w, env(), Arc::clone(&budget)).expect("pool bench shared");
    for _ in 0..SHARED_BENCH_TASKS {
        match mode {
            "shared" => {
                let p = Arc::clone(payload);
                pool.spawn_shared(p, move |ctx, p| {
                    std::hint::black_box(shared_len(p));
                    std::hint::black_box(shared_cpu_work(cpu));
                    for _ in 0..SHARED_BENCH_CHARGES {
                        ctx.charge(1)?;
                    }
                    Ok(SendValue::Unit)
                })
                .expect("spawn shared bench");
            }
            "worker" => {
                let p = Arc::clone(payload);
                pool.spawn_worker_copy(p, move |ctx, p| {
                    std::hint::black_box(shared_len(&p));
                    std::hint::black_box(shared_cpu_work(cpu));
                    for _ in 0..SHARED_BENCH_CHARGES {
                        ctx.charge(1)?;
                    }
                    Ok(SendValue::Unit)
                })
                .expect("spawn worker-copy bench");
            }
            "producer" => {
                let p = (**payload).clone();
                pool.spawn(move |ctx| {
                    std::hint::black_box(shared_len(&p));
                    std::hint::black_box(shared_cpu_work(cpu));
                    for _ in 0..SHARED_BENCH_CHARGES {
                        ctx.charge(1)?;
                    }
                    Ok(SendValue::Unit)
                })
                .expect("spawn producer bench");
            }
            _ => unreachable!("mode bench lạ: {mode}"),
        }
    }
    let r = pool.join().expect("join bench shared");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let snap = r.budget.expect("budget bench");
    assert_eq!(
        snap.used,
        (SHARED_BENCH_TASKS * SHARED_BENCH_CHARGES) as i64,
        "bench {mode} W={w}: used"
    );
    assert_eq!(snap.used + snap.remaining, snap.limit, "bench {mode} W={w}");
    assert!(!snap.exhausted, "bench {mode} W={w}: cạn ngân sách");
    assert_eq!(r.outcomes.len(), SHARED_BENCH_TASKS);
    assert_eq!(Arc::strong_count(payload), 1, "bench {mode} W={w}: Arc rò");
    ms
}

/// **Hardening 100 vòng** (mặc định 10): `spawn_shared` read-only, payload xen kẽ
/// map1k/list10k, W=1/W=4, 10k task/pha; in/budget/cancel/cross-write/Arc/payload.
#[test]
fn spawn_shared_stress_rounds_read_only() {
    let rounds = shared_rounds();
    let t_all = Instant::now();
    let rss0 = status_kb("VmRSS:");
    let threads0 = status_kb("Threads:");
    let mut rss_rounds = Vec::with_capacity(rounds);
    let mut threads_rounds = Vec::with_capacity(rounds);
    let mut secs_rounds = Vec::with_capacity(rounds);
    let mut peak_threads = threads0;
    let mut hwm_kb = status_kb("VmHWM:");
    let mut bench: Vec<(String, usize, String, f64)> = Vec::new();

    let (cpu_iters, cpu_us) = shared_calibrate(100_000.0);
    shared_warm(150);
    println!(
        "SHARED_STRESS_ENV\trounds={rounds}\tn={SHARED_N}\tspawns_per_round={}\tcpu_iters={cpu_iters}\tper_task_us={cpu_us:.1}\trss0_kb={rss0}\tthreads0={threads0}",
        SHARED_N * 6
    );

    for round in 0..rounds {
        let t_round = Instant::now();
        let (ten, payload_v): (&'static str, SendValue) = if round % 2 == 0 {
            ("map1k", shared_map1k())
        } else {
            ("list10k", shared_list10k())
        };
        let payload = Arc::new(payload_v);
        let mong_doi_len = if round % 2 == 0 { 1_000 } else { 10_000 };

        // A. Print merge + budget atomic + Arc về 1, W=1/W=4.
        let mut outs = Vec::with_capacity(2);
        for w in [1usize, 4] {
            let b = Arc::new(SharedBudget::new(SHARED_N as u64 + 1_000, 256));
            let mut pool = WorkerPool::with_budget(w, env(), Arc::clone(&b)).expect("pool in");
            for i in 0..SHARED_N {
                let p = Arc::clone(&payload);
                pool.spawn_shared(p, move |ctx, p| {
                    let n = shared_len(p);
                    ctx.print_line(&format!("{ten}:{i}:{n}"));
                    ctx.charge(1)?;
                    Ok(SendValue::Int(i as i64))
                })
                .expect("spawn in");
            }
            peak_threads = peak_threads.max(status_kb("Threads:"));
            let r = pool.join().expect("join in");
            let snap = r.budget.expect("budget in");
            assert_eq!(
                r.outcomes.len(),
                SHARED_N,
                "vòng {round}: outcomes in W={w}"
            );
            assert_eq!(snap.used, SHARED_N as i64, "vòng {round}: used in W={w}");
            assert_eq!(
                snap.used + snap.remaining,
                snap.limit,
                "vòng {round}: used+remaining==limit in W={w}"
            );
            assert_eq!(snap.remaining, 1_000, "vòng {round}: remaining in W={w}");
            assert!(
                !snap.exhausted,
                "vòng {round}: budget in W={w} không được cạn"
            );
            assert_eq!(
                Arc::strong_count(&payload),
                1,
                "vòng {round}: Arc in W={w} phải về 1"
            );
            assert_shared_payload(&payload, round, "in");
            outs.push(r.output);
        }
        assert_eq!(
            outs[0], outs[1],
            "vòng {round}: print merge W=1/W=4 phải byte-identical"
        );
        assert_eq!(
            outs[0].lines().count(),
            SHARED_N,
            "vòng {round}: số dòng in"
        );
        assert!(
            outs[0].starts_with(&format!("{ten}:0:{mong_doi_len}\n")),
            "vòng {round}: dòng in đầu"
        );
        assert!(
            outs[0].ends_with(&format!("{ten}:{}:{mong_doi_len}\n", SHARED_N - 1)),
            "vòng {round}: dòng in cuối"
        );

        // B. Cancel nửa số handle, W=1/W=4; join đủ, Arc về 1, không deadlock.
        for w in [1usize, 4] {
            let mut pool = WorkerPool::new(w, env()).expect("pool cancel shared");
            let mut handles = Vec::with_capacity(SHARED_N);
            for _ in 0..SHARED_N {
                let p = Arc::clone(&payload);
                let h = pool
                    .spawn_shared(p, |ctx, p| {
                        ctx.check_cancel()?;
                        std::hint::black_box(shared_len(p));
                        Ok(SendValue::Unit)
                    })
                    .expect("spawn cancel shared");
                handles.push(h);
            }
            for h in handles.iter().step_by(2) {
                h.cancel();
            }
            peak_threads = peak_threads.max(status_kb("Threads:"));
            let r = pool.join().expect("join cancel shared");
            assert_eq!(
                r.outcomes.len(),
                SHARED_N,
                "vòng {round}: outcomes cancel W={w}"
            );
            let cancelled = r.outcomes.iter().filter(|o| o.cancelled).count();
            assert!(
                cancelled <= SHARED_N / 2,
                "vòng {round}: W={w} huỷ quá nửa: {cancelled}"
            );
            if w == 1 {
                assert_eq!(
                    cancelled,
                    SHARED_N / 2,
                    "vòng {round}: W=1 phải huỷ đúng nửa (task chẵn)"
                );
            }
            assert_eq!(
                Arc::strong_count(&payload),
                1,
                "vòng {round}: Arc cancel W={w} phải về 1"
            );
            assert_shared_payload(&payload, round, "cancel");
        }

        // C. Cross-write W=4 vẫn bị từ chối, đủ 10k task id; Arc về 1.
        let mut pool = WorkerPool::new(4, env()).expect("pool cross shared");
        for i in 0..SHARED_N {
            let p = Arc::clone(&payload);
            pool.spawn_shared(p, move |ctx, p| {
                std::hint::black_box(shared_len(p));
                ctx.set("counter", SendValue::Int(i as i64));
                Ok(SendValue::Unit)
            })
            .expect("spawn cross shared");
        }
        peak_threads = peak_threads.max(status_kb("Threads:"));
        match pool.join() {
            Err(WorkerError::CrossWrite { vars, tasks }) => {
                assert_eq!(vars, vec!["counter".to_string()], "vòng {round}: vars");
                assert_eq!(tasks.len(), SHARED_N, "vòng {round}: tasks cross-write");
            }
            other => panic!("vòng {round}: spawn_shared cross-write phải Err: {other:?}"),
        }
        assert_eq!(
            Arc::strong_count(&payload),
            1,
            "vòng {round}: Arc cross-write phải về 1"
        );
        assert_shared_payload(&payload, round, "cross-write");

        // D. Budget cạn W=4: used+remaining==limit, used=limit/2, remaining=0.
        let b = Arc::new(SharedBudget::new((SHARED_N / 2) as u64, 256));
        let mut pool = WorkerPool::with_budget(4, env(), Arc::clone(&b)).expect("pool exhausted");
        for _ in 0..SHARED_N {
            let p = Arc::clone(&payload);
            pool.spawn_shared(p, |ctx, p| {
                std::hint::black_box(shared_len(p));
                ctx.charge(1)?;
                Ok(SendValue::Unit)
            })
            .expect("spawn shared exhausted");
        }
        peak_threads = peak_threads.max(status_kb("Threads:"));
        let r = pool.join().expect("join shared exhausted");
        let snap = r.budget.expect("budget exhausted");
        assert_eq!(r.outcomes.len(), SHARED_N, "vòng {round}: outcomes budget");
        assert_eq!(snap.used, (SHARED_N / 2) as i64, "vòng {round}: used cạn");
        assert_eq!(snap.remaining, 0, "vòng {round}: remaining phải 0");
        assert_eq!(
            snap.used + snap.remaining,
            snap.limit,
            "vòng {round}: used+remaining==limit khi cạn"
        );
        assert!(snap.exhausted, "vòng {round}: phải đánh dấu cạn");
        assert!(
            r.outcomes.iter().any(|o| o.result.is_err()),
            "vòng {round}: phải có task lỗi ngân sách"
        );
        assert_eq!(
            Arc::strong_count(&payload),
            1,
            "vòng {round}: Arc budget phải về 1"
        );
        assert_shared_payload(&payload, round, "budget");

        // Đo sau khi vòng xong (trước benchmark) để RSS/threads phản ánh stress.
        let rss = status_kb("VmRSS:");
        let hwm = status_kb("VmHWM:");
        let threads = status_kb("Threads:");
        let secs = t_round.elapsed().as_secs_f64();
        rss_rounds.push(rss);
        threads_rounds.push(threads);
        secs_rounds.push(secs);
        hwm_kb = hwm_kb.max(hwm);
        println!(
            "SHARED_STRESS\tround={round}\tpayload={ten}\tn={SHARED_N}\tsecs={secs:.3}\trss_kb={rss}\thwm_kb={hwm}\tthreads={threads}"
        );

        // Mỗi 10 vòng: median shared/worker-copy/producer cho cả map1k+list10k.
        if (round + 1) % 10 == 0 {
            for (bname, bv) in [("map1k", shared_map1k()), ("list10k", shared_list10k())] {
                let bp = Arc::new(bv);
                for w in [1usize, 4] {
                    for mode in ["shared", "worker", "producer"] {
                        let mut xs = Vec::with_capacity(3);
                        for _ in 0..3 {
                            xs.push(shared_bench_run(w, &bp, mode, cpu_iters));
                        }
                        let med = shared_median(xs);
                        assert_eq!(
                            Arc::strong_count(&bp),
                            1,
                            "vòng {round}: Arc bench {bname}/{mode}/W={w} rò"
                        );
                        bench.push((bname.to_string(), w, mode.to_string(), med));
                        println!(
                            "SHARED_BENCH10\tround={}\tpayload={bname}\tw={w}\tmode={mode}\tms={med:.2}",
                            round + 1
                        );
                    }
                }
            }
            assert_shared_payload(&payload, round, "bench");
        }
    }

    let rss1 = status_kb("VmRSS:");
    let hwm_final = status_kb("VmHWM:");
    let threads1 = status_kb("Threads:");
    let secs_all = t_all.elapsed().as_secs_f64();
    let mut sorted_rss = rss_rounds.clone();
    sorted_rss.sort_unstable();
    let rss_min = sorted_rss.first().copied().unwrap_or(0);
    let rss_med = sorted_rss[sorted_rss.len() / 2];
    let rss_max = sorted_rss.last().copied().unwrap_or(0);
    let n_dau = rounds.min(10);
    let mut dau = rss_rounds[..n_dau].to_vec();
    let mut cuoi = rss_rounds[rounds - n_dau..].to_vec();
    dau.sort_unstable();
    cuoi.sort_unstable();
    let rss_dau = dau[dau.len() / 2];
    let rss_cuoi = cuoi[cuoi.len() / 2];
    // Rò dài hạn: so 10 vòng cuối với **plateau** sau checkpoint benchmark đầu
    // (vòng 10–19). RSS tăng một lần ở vòng 10 là allocator giữ buffer benchmark,
    // không phải rò; nếu vẫn tăng ở các vòng sau mới là rò.
    let (start_plateau, n_plateau) = if rounds > 20 { (10, 10) } else { (0, rounds) };
    let mut plateau = rss_rounds[start_plateau..start_plateau + n_plateau].to_vec();
    plateau.sort_unstable();
    let rss_plateau = plateau[plateau.len() / 2];
    let mut sorted_secs = secs_rounds.clone();
    sorted_secs.sort_by(|a, b| a.total_cmp(b));
    let secs_med = sorted_secs[sorted_secs.len() / 2];
    let secs_min = sorted_secs.first().copied().unwrap_or(0.0);
    let secs_max = sorted_secs.last().copied().unwrap_or(0.0);
    let threads_peak_round = threads_rounds.iter().copied().max().unwrap_or(0);
    println!(
        "SHARED_STRESS_SUMMARY\trounds={rounds}\tn={SHARED_N}\tspawns_total={}\tsecs={secs_all:.2}\tsecs_med={secs_med:.3}\tsecs_min={secs_min:.3}\tsecs_max={secs_max:.3}\trss0_kb={rss0}\trss1_kb={rss1}\trss_min_kb={rss_min}\trss_med_kb={rss_med}\trss_max_kb={rss_max}\trss_dau10_kb={rss_dau}\trss_plateau_kb={rss_plateau}\trss_cuoi10_kb={rss_cuoi}\thwm_peak_kb={}\thwm_final_kb={hwm_final}\tthreads0={threads0}\tthreads1={threads1}\tthreads_peak={peak_threads}\tthreads_round_max={threads_peak_round}",
        rounds * SHARED_N * 6,
        hwm_kb.max(hwm_final)
    );

    for bname in ["map1k", "list10k"] {
        for w in [1usize, 4] {
            for mode in ["shared", "worker", "producer"] {
                let xs: Vec<f64> = bench
                    .iter()
                    .filter(|(p, ww, m, _)| p.as_str() == bname && *ww == w && m == mode)
                    .map(|x| x.3)
                    .collect();
                if xs.is_empty() {
                    continue;
                }
                println!(
                    "SHARED_BENCH_MEDIAN\tpayload={bname}\tw={w}\tmode={mode}\tms={:.2}\tcheckpoints={}",
                    shared_median(xs.clone()),
                    xs.len()
                );
            }
        }
    }

    assert!(
        threads1 <= threads0 + 1,
        "rò worker thread: {threads0} -> {threads1}"
    );
    assert!(
        rss1.saturating_sub(rss0) < 256 * 1024,
        "RSS tăng quá 256 MB (nghi rò): {rss0} -> {rss1}"
    );
    assert!(
        rss_cuoi.saturating_sub(rss_plateau) < 64 * 1024,
        "RSS 10 vòng cuối tăng quá 64 MB so plateau: {rss_plateau} -> {rss_cuoi}"
    );
    assert!(secs_all < 900.0, "stress quá chậm: {secs_all:.1}s");
}

// ===========================================================================
// **VS3-3b/W5 mixed soak** — env-gated, mặc định ngắn cho gate.
//
// Chạy đầy đủ **30 phút** (1800 s, dừng ở biên batch):
//   VS3_W5_MIXED_SECS=1800 VS3_W5_MIXED_ROUNDS=1000000 \
//     cargo test --release -p vis-cli --test vs3_worker mixed_soak -- \
//       --nocapture --test-threads=1
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
    ctx: &mut vis_interp::value::worker::WorkerCtx,
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
) -> Result<vis_interp::value::worker::TaskHandle, WorkerError> {
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
#[test]
fn mixed_soak_shared_copy_move_w1_w4() {
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
