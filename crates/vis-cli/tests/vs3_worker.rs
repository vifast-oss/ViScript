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
use std::time::Duration;

use vis_interp::rt::fee::SharedBudget;
use vis_interp::rt::{Task, TrangThai};
use vis_interp::value::worker::{EnvMap, SendValue, WorkerError, WorkerPool};
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
