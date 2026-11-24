// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P2b, test-only)** — điều tra overhead 10k task cực nhỏ.
//!
//! Chạy tay (release):
//! ```text
//! VS3_P2B_N=10000 cargo run --release -p vis-cli --example vs3_worker_eval_p2b_overhead
//! ```
//!
//! Đo trên cùng main hiện hành:
//! - baseline interp P2 / VM P3: 10k `spawn`/`await`, W=1/2/4;
//! - tách chi phí: dựng `Vm`, `vis_interp::run` (dựng Interp + chạy tối thiểu),
//!   pool thuần (không evaluator), pool + evaluator, outcome board, print;
//! - thử tái dùng `Vm` theo thread (thread-local) — **không** áp dụng cho Interp vì
//!   `Interp::new` là `pub(crate)` và chưa có `reset_for_task`.
//!
//! Không sửa production/default/ABI. `Interp` reuse và runner thật vẫn là đề xuất P2b.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use vis_runtime::rt::Scheduler;
use vis_runtime::worker::{OutcomeBoard, SendValue, TaskId, TaskOutcome, WorkerCtx, WorkerPool};
use vis_syntax::{lex, parse, Mode, Span};
use vis_vm::Vm;

fn src_spawn(n: usize) -> String {
    format!(
        r#"async fn t(n: int) -> int:
    n + 1

fn main():
    var s = 0
    var i = 0
    while i < {n}:
        let x = spawn t(i)
        s = s + await x
        i = i + 1
    print(s)
"#
    )
}

fn src_batch(n: usize) -> String {
    format!(
        r#"async fn t(n: int) -> int:
    n + 1

fn main():
    var i = 0
    while i < {n}:
        let _x = spawn t(i)
        i = i + 1
"#
    )
}

const RONG: &str = r#"fn main():
    let x = 1
"#;

struct Image {
    hir: vis_hir::Hir,
    syms: vis_hir::Symbols,
    tys: vis_hir::TypeTable,
    module: vis_bytecode::Module,
    src: String,
}

fn anh(src: &str) -> Arc<Image> {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into(), "fs".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let module = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    Arc::new(Image {
        hir: h.hir,
        syms: h.symbols,
        tys: h.types,
        module,
        src: src.to_string(),
    })
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn chay_interp(a: &Image, w: usize) -> vis_core::RunOutput {
    if w == 1 {
        vis_interp::run_workers_local(&a.src, &a.hir, &a.syms, &a.tys, &[])
    } else {
        vis_interp::run_workers_parallel(&a.src, &a.hir, &a.syms, &a.tys, &[], w)
    }
}

fn chay_vm(a: &Image, w: usize) -> vis_core::RunOutput {
    if w == 1 {
        vis_vm::run_workers_local_vm(&a.module, &a.src, &[])
    } else {
        vis_vm::run_workers_parallel_vm(&a.module, &a.src, &[], w)
    }
}

fn run_kind(kind: &Kind, _ctx: &mut WorkerCtx) -> Result<SendValue, String> {
    match kind {
        Kind::None => Ok(SendValue::Int(1)),
        Kind::Interp(img) => {
            let out = vis_interp::run(&img.src, &img.hir, &img.syms, &img.tys, &[]);
            Ok(SendValue::Int(out.exit as i64))
        }
        Kind::Vm(img) => {
            let mut vm = Vm::new(&img.module, &img.src);
            let out = vm.run(&[]);
            Ok(SendValue::Int(out.exit as i64))
        }
        Kind::VmReuse(m, text) => vm_reuse_job(m, text),
    }
}

fn do_pool(workers: usize, n: usize, board: bool, print: bool, kind: Kind) -> f64 {
    let root: HashMap<String, SendValue> = HashMap::new();
    let mut pool = WorkerPool::new(workers, root).expect("pool");
    if board {
        pool.attach_outcome_board(Arc::new(OutcomeBoard::new()));
    }
    let t = Instant::now();
    for _ in 0..n {
        let kind = kind.clone();
        pool.spawn(move |ctx: &mut WorkerCtx| {
            if print {
                ctx.print("x");
            }
            run_kind(&kind, ctx)
        })
        .expect("spawn");
    }
    let r = pool.join().expect("join");
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(r.outcomes.len(), n);
    if print {
        assert_eq!(r.output.len(), n);
    }
    ms
}

#[derive(Clone)]
enum Kind {
    None,
    Interp(Arc<Image>),
    Vm(Arc<Image>),
    VmReuse(&'static vis_bytecode::Module, &'static str),
}

thread_local! {
    static REUSE_VM: RefCell<Option<VisVmStatic>> = const { RefCell::new(None) };
}

type VisVmStatic = Vm<'static>;

fn vm_reuse_job(m: &'static vis_bytecode::Module, text: &'static str) -> Result<SendValue, String> {
    REUSE_VM.with(|vm| {
        let mut vm = vm.borrow_mut();
        if vm.is_none() {
            *vm = Some(Vm::new(m, text));
        }
        let out = vm.as_mut().unwrap().run(&[]);
        Ok(SendValue::Int(out.exit as i64))
    })
}

fn do_eval_cost(img: &Image, n: usize) -> (f64, f64, f64, f64) {
    // Vm::new (chỉ constructor) 10k
    let t = Instant::now();
    for _ in 0..n {
        black_box(Vm::new(&img.module, &img.src));
    }
    let vm_new = t.elapsed().as_secs_f64() * 1000.0;

    // VM mới + run 10k
    let t = Instant::now();
    for _ in 0..n {
        let mut vm = Vm::new(&img.module, &img.src);
        black_box(vm.run(&[]));
    }
    let vm_fresh = t.elapsed().as_secs_f64() * 1000.0;

    // VM tái dùng 10k (cùng thread)
    let mut vm = Vm::new(&img.module, &img.src);
    let t = Instant::now();
    for _ in 0..n {
        black_box(vm.run(&[]));
    }
    let vm_reuse = t.elapsed().as_secs_f64() * 1000.0;

    // vis_interp::run 10k (constructor Interp + chạy tối thiểu)
    let t = Instant::now();
    for _ in 0..n {
        black_box(vis_interp::run(
            &img.src,
            &img.hir,
            &img.syms,
            &img.tys,
            &[],
        ));
    }
    let interp_fresh = t.elapsed().as_secs_f64() * 1000.0;
    (vm_new, vm_fresh, vm_reuse, interp_fresh)
}

fn do_pool_new(workers: usize, reps: usize) -> f64 {
    let t = Instant::now();
    for _ in 0..reps {
        let p = WorkerPool::new(workers, HashMap::new()).expect("pool");
        black_box(&p);
        drop(p);
    }
    t.elapsed().as_secs_f64() * 1000.0 / reps as f64
}

fn do_sched(n: usize) -> (f64, f64) {
    let mut s = Scheduler::new();
    let t = Instant::now();
    for _ in 0..n {
        black_box(s.tao(0, Vec::new(), "t".to_string(), Span::new(0, 0)));
    }
    let tao = t.elapsed().as_secs_f64() * 1000.0;

    let b = OutcomeBoard::new();
    let t = Instant::now();
    for i in 0..n {
        b.publish(TaskOutcome {
            id: TaskId::from_raw(i as u64),
            result: Ok(SendValue::Int(1)),
            output: String::new(),
            cancelled: false,
            dirty: Vec::new(),
            budget_used: 0,
        });
    }
    let mut got = 0usize;
    for i in 0..n {
        got += b.try_take(TaskId::from_raw(i as u64)).is_some() as usize;
    }
    assert_eq!(got, n);
    let board = t.elapsed().as_secs_f64() * 1000.0;
    (tao, board)
}

fn do_pool_shared(n: usize) -> f64 {
    let mut pool = WorkerPool::new(4, HashMap::new()).expect("pool");
    let t = Instant::now();
    for _ in 0..n {
        let payload = Arc::new(SendValue::Int(0));
        pool.spawn_shared(payload, |_ctx, _p| Ok(SendValue::Int(1)))
            .expect("spawn");
    }
    let r = pool.join().expect("join");
    assert_eq!(r.outcomes.len(), n);
    t.elapsed().as_secs_f64() * 1000.0
}

fn do_lazy(workers: usize, n: usize, kind: Kind) -> f64 {
    // Mô phỏng P2b-1: tạo Task (Scheduler::tao) + proxy map nhưng **chưa gửi pool**;
    // flush toàn bộ queue một lần, rồi join. Đây chỉ là mô phỏng bằng API công khai.
    let mut sched = Scheduler::new();
    let mut queue = Vec::with_capacity(n);
    let mut proxy: HashMap<u32, u32> = HashMap::with_capacity(n);
    let t = Instant::now();
    for i in 0..n {
        let task = sched.tao(0, Vec::new(), "t".to_string(), Span::new(0, 0));
        proxy.insert(task.id, i as u32);
        queue.push((task.id, kind.clone()));
    }
    let mut pool = WorkerPool::new(workers, HashMap::new()).expect("pool");
    for (_, job) in queue {
        let payload = Arc::new(SendValue::Int(0));
        pool.spawn_shared(payload, move |ctx, _snap| run_kind(&job, ctx))
            .expect("spawn");
    }
    let r = pool.join().expect("join");
    assert_eq!(r.outcomes.len(), n);
    assert_eq!(proxy.len(), n);
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() {
    let n = std::env::var("VS3_P2B_N")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(10_000);
    let reps = std::env::var("VS3_P2B_REPS")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(3)
        .max(1);
    assert!(n > 0);
    let src = src_spawn(n);
    let img = anh(&src);
    let img_batch = anh(&src_batch(n));
    let ky_vong = n * (n + 1) / 2;
    let rong = anh(RONG);
    let m_leak: &'static vis_bytecode::Module = Box::leak(Box::new(rong.module.clone()));
    let s_leak: &'static str = Box::leak(rong.src.clone().into_boxed_str());

    println!(
        "P2B\tbat_dau\tN={n}\treps={reps}\tload={}",
        std::fs::read_to_string("/proc/loadavg")
            .unwrap_or_default()
            .trim()
    );

    // 1) Baseline P2/P3 10k task, W=1/2/4.
    let mut base: HashMap<(&str, usize), f64> = HashMap::new();
    for w in [1usize, 2, 4] {
        let mut vi = Vec::new();
        let mut vv = Vec::new();
        for _ in 0..reps {
            let t = Instant::now();
            let i = chay_interp(&img, w);
            vi.push(t.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(i.exit, 0, "interp W={w}: {:?}", i.diagnostics);
            assert_eq!(i.stdout, format!("{ky_vong}\n"));
            let t = Instant::now();
            let v = chay_vm(&img, w);
            vv.push(t.elapsed().as_secs_f64() * 1000.0);
            assert_eq!(v.exit, 0, "VM W={w}: {:?}", v.diagnostics);
            assert_eq!(v.stdout, i.stdout);
        }
        let mi = median(vi);
        let mv = median(vv);
        base.insert(("interp", w), mi);
        base.insert(("vm", w), mv);
        println!(
            "P2B_BASE\tinterp\tW={w}\tms={mi:.1}\ttask_s={:.0}",
            n as f64 / (mi / 1000.0)
        );
        println!(
            "P2B_BASE\tvm\tW={w}\tms={mv:.1}\ttask_s={:.0}",
            n as f64 / (mv / 1000.0)
        );
    }

    // 1b) Batch: spawn 10k, không await từng task; final join một lần.
    for w in [1usize, 4] {
        let t = Instant::now();
        let i = chay_interp(&img_batch, w);
        let mi = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(i.exit, 0, "batch interp W={w}: {:?}", i.diagnostics);
        assert!(i.stdout.is_empty());
        let t = Instant::now();
        let v = chay_vm(&img_batch, w);
        let mv = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(v.exit, 0, "batch VM W={w}: {:?}", v.diagnostics);
        assert!(v.stdout.is_empty());
        println!("P2B_BATCH\tW={w}\tinterp_ms={mi:.1}\tvm_ms={mv:.1}");
    }

    // 1c) Lazy/batched queue mô phỏng P2b-1: tạo Task/proxy, chưa gửi pool; flush một lần.
    for w in [1usize, 2, 4] {
        let mi = do_lazy(w, n, Kind::Interp(rong.clone()));
        let mv = do_lazy(w, n, Kind::Vm(rong.clone()));
        println!(
            "P2B_LAZY\tW={w}\tinterp_ms={mi:.1}\tvm_ms={mv:.1}\tinterp_tasks_s={:.0}",
            n as f64 / (mi / 1000.0)
        );
    }

    // 2) Dựng evaluator/task + tái dùng VM.
    let (vm_new, vm_fresh, vm_reuse, interp_fresh) = do_eval_cost(&rong, n);
    println!(
        "P2B_EVAL_COST\tN={n}\tvm_new_us_task={:.2}\tvm_fresh_us_task={:.2}\tvm_reuse_us_task={:.2}\tvm_reuse_speedup={:.2}x\tinterp_run_us_task={:.2}",
        vm_new * 1000.0 / n as f64,
        vm_fresh * 1000.0 / n as f64,
        vm_reuse * 1000.0 / n as f64,
        vm_fresh / vm_reuse,
        interp_fresh * 1000.0 / n as f64
    );

    let (tao, board_take) = do_sched(n);
    println!("P2B_SCHED\tN={n}\ttao_ms={tao:.1}\toutcome_publish_try_ms={board_take:.1}");

    // 3) Pool thuần: W=1/2/4; board/no-board; print.
    for w in [1usize, 2, 4] {
        let ms = do_pool(w, n, false, false, Kind::None);
        println!("P2B_POOL\tno_board\tW={w}\tms={ms:.1}");
    }
    let no_board = do_pool(4, n, false, false, Kind::None);
    let board = do_pool(4, n, true, false, Kind::None);
    let print = do_pool(4, n, false, true, Kind::None);
    println!(
        "P2B_POOL\tW=4\tno_board_ms={no_board:.1}\tboard_ms={board:.1}\tboard_delta_ms={:.1}\tprint_same_board_off_ms={print:.1}",
        board - no_board
    );

    let shared = do_pool_shared(n);
    println!("P2B_POOL\tshared_payload_W=4\tms={shared:.1}");

    // 4) Pool + evaluator mới/task (VM/Interp), và VM reuse theo thread.
    let pool_vm = do_pool(4, n, false, false, Kind::Vm(rong.clone()));
    let pool_interp = do_pool(4, n, false, false, Kind::Interp(rong.clone()));
    let pool_reuse = do_pool(4, n, false, false, Kind::VmReuse(m_leak, s_leak));
    println!(
        "P2B_POOL_EVAL\tW=4\tpool_vm_ms={pool_vm:.1}\tpool_interp_ms={pool_interp:.1}\tpool_vm_reuse_ms={pool_reuse:.1}\tvm_reuse_speedup={:.2}x",
        pool_vm / pool_reuse
    );

    // 5) Pool creation (lazy pool không tạo khi không spawn; đo tạo thật).
    let new4 = do_pool_new(4, 20);
    println!("P2B_POOL_NEW\tW=4\tus_pool={new4:.1}");

    // 6) Tổng hợp tách chi phí so baseline W=4.
    let bi = base[&("interp", 4)];
    let bv = base[&("vm", 4)];
    println!(
        "P2B_SUMMARY\tN={n}\tinterp_W4_ms={bi:.1}\tvm_W4_ms={bv:.1}\tpool4_none_ms={no_board:.1}\tpool4_vm_ms={pool_vm:.1}\tpool4_interp_ms={pool_interp:.1}\tvm_new_us_task={:.2}\tinterp_run_us_task={:.2}",
        vm_new * 1000.0 / n as f64,
        interp_fresh * 1000.0 / n as f64
    );
    println!(
        "P2B_BIEN_THE\tinterp_reuse=KHONG_THA_DUOC_pub_crate_Interp_new;vm_reuse=THU_NGHIEM_thread_local;board=THU_NGHIEM;batch_outcome=THU_NGHIEM;lazy_pool=DA_CO;print=DO_DUOC"
    );
    println!("KET_LUAN\tPASS\tN={n}\tsum={ky_vong}\tbaseline W4 interp={bi:.1}ms vm={bv:.1}ms");
}
