// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (prototype, test/example-only)** — evaluator **độc lập** trên worker thread.
//!
//! Mục tiêu (time-box 2 h, **không** sửa production/ABI/default):
//! 1. Khẳng định `Hir`, `Symbols`, `TypeTable`, `vis_bytecode::Module`, `SendValue` là
//!    `Send + Sync` bằng ràng buộc biên dịch (không `unsafe`, không sửa kiểu).
//! 2. Dựng một chương trình nhỏ (in + tính toán + biến) thành `Arc<Module>` **một lần**;
//!    mỗi worker/thread dựng evaluator **riêng** từ ảnh bất biến đó và chạy cùng chương trình.
//! 3. So **W=1 vs W=4 byte-identical**, kiểm không rò thread, đo wall/RSS/thread/fd; ghép
//!    `WorkerPool` cho budget + cross-write.
//!
//! Chạy thủ công (example, không nằm trong test suite):
//! ```text
//! cargo run -p vis-cli --example vs3_worker_eval_proto
//! ```
//!
//! Ghi chú phạm vi: `vis_interp::interp::Interp` có `new` là `pub(crate)`, nên bên ngoài crate
//! mỗi lần gọi [`vis_interp::run`] chính là **một `Interp` mới**. Đường VM dùng `Vm::new` công
//! khai trên `Arc<Module>`. Không có dòng production nào bị sửa bởi example này.
//!
//! Kết quả in ra dạng `KHOA<TAB>...` để dán vào JOURNAL; mọi kiểm tra là `assert!` thật, sai
//! thì example panic (exit 101).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_interp::rt::fee::{BudgetSnapshot, SharedBudget};
use vis_interp::value::worker::{SendValue, WorkerCtx, WorkerError, WorkerPool};
use vis_interp::RunOutput;
use vis_syntax::{lex, parse, Mode};
use vis_types::{Hir, Symbols, TypeTable};
use vis_vm::vm::Vm;

/// Chương trình mẫu: hàm + `var`/`let` + vòng `while` + số học + 3 dòng `print`.
/// Kết quả tất định, đủ nặng để đo song song mà vẫn nhanh ở build debug (~0,2 s/lần interp).
const SRC: &str = r#"fn work(n: int) -> int:
    var acc = 0
    var i = 0
    while i < n:
        acc = acc + (i * 3) % 7
        i = i + 1
    acc

fn main():
    var total = 0
    var k = 1
    while k <= 8:
        total = total + work(k * 500)
        k = k + 1
    let nhan = 2
    print("tong")
    print(total)
    print(total * nhan - 7)
"#;

/// Số job mỗi lần chạy pool (bằng W tối đa) — đủ để thấy 4 evaluator chạy cùng lúc.
const JOBS: usize = 4;
/// Số vòng đo wall; lấy **median** để giảm nhiễu do load máy.
const VONG_DO: usize = 3;
/// Ngân sách: mỗi job charge [`BUDGET_CHARGE`]; dư rất nhiều để không cạn.
const BUDGET_LIMIT: i64 = 100_000;
const BUDGET_CHARGE: i64 = 1_000;
const BUDGET_CHUNK: i64 = 256;

/// Ảnh chương trình **bất biến**, chia sẻ `Arc` cho mọi worker/thread.
///
/// `Hir`/`Symbols`/`TypeTable` cho đường interp; `module` cho đường VM. `text` giữ nguyên
/// nguồn để chẩn đoán `line`/`col` và cho `Vm::new`.
struct Anh {
    hir: Hir,
    symbols: Symbols,
    types: TypeTable,
    text: String,
    module: Arc<vis_bytecode::Module>,
}

fn khang_dinh_send_sync<T: Send + Sync>() {}

fn dung_anh(src: &str) -> Arc<Anh> {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile VBC1");
    Arc::new(Anh {
        hir: h.hir,
        symbols: h.symbols,
        types: h.types,
        text: src.to_string(),
        module: Arc::new(m),
    })
}

/// Đường interp: mỗi lần gọi tạo **một `Interp` mới** bên trong crate.
fn chay_hir(a: &Anh) -> RunOutput {
    vis_interp::run(&a.text, &a.hir, &a.symbols, &a.types, &[])
}

/// Đường VM: dựng `Vm` riêng trên cùng `Arc<Module>` bất biến.
fn chay_vm(a: &Anh) -> RunOutput {
    let mut vm = Vm::new(&a.module, &a.text);
    vm.run(&[])
}

/// Chạy **cả hai engine** trên cùng ảnh; trả kết quả gọn để gửi qua biên worker.
/// Chạy cả hai trong mỗi job để chứng minh cả HIR lẫn `Module` dùng được đồng thời.
struct KetQua {
    stdout: String,
    exit: i32,
    steps_hir: usize,
    steps_vm: usize,
}

fn chay_hai_engine(a: &Anh) -> KetQua {
    let i = chay_hir(a);
    let v = chay_vm(a);
    assert!(i.is_clean(), "interp: {:?}", i.codes());
    assert!(v.is_clean(), "vm: {:?}", v.codes());
    assert_eq!(i.exit, v.exit, "exit lệch interp/VM");
    assert_eq!(i.stdout, v.stdout, "stdout lệch interp/VM");
    assert!(i.steps > 0 && v.steps > 0, "chống xanh-giả: steps = 0");
    KetQua {
        stdout: i.stdout,
        exit: i.exit,
        steps_hir: i.steps,
        steps_vm: v.steps,
    }
}

fn send_ket_qua(k: &KetQua) -> SendValue {
    SendValue::Record(
        "KetQua".to_string(),
        vec![
            ("exit".to_string(), SendValue::Int(k.exit as i64)),
            ("steps_hir".to_string(), SendValue::Int(k.steps_hir as i64)),
            ("steps_vm".to_string(), SendValue::Int(k.steps_vm as i64)),
        ],
    )
}

fn dem_thread() -> Option<usize> {
    std::fs::read_dir("/proc/self/task")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

fn dem_fd() -> Option<usize> {
    std::fs::read_dir("/proc/self/fd")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

/// `(VmRSS kB, VmHWM kB)` từ `/proc/self/status`; nền khác Linux trả `None`.
fn rss_kb() -> Option<(u64, u64)> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    let mut rss = None;
    let mut hwm = None;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            rss = rest.split_whitespace().next()?.parse().ok();
        } else if let Some(rest) = line.strip_prefix("VmHWM:") {
            hwm = rest.split_whitespace().next()?.parse().ok();
        }
    }
    Some((rss?, hwm?))
}

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg")
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn median(mut v: Vec<Duration>) -> Duration {
    v.sort();
    v[v.len() / 2]
}

/// Chạy `JOBS` job trên pool W=1/W=N; mỗi job dựng evaluator riêng từ `Arc<Anh>`, in stdout
/// vào buffer của task (pool merge theo spawn order), charge ngân sách và trả record kết quả.
struct KetQuaPool {
    output: String,
    results: Vec<SendValue>,
    wall: Duration,
    budget: Option<BudgetSnapshot>,
    threads_sau: Option<usize>,
}

fn chay_pool(anh: Arc<Anh>, workers: usize) -> KetQuaPool {
    let budget = Arc::new(SharedBudget::new(BUDGET_LIMIT as u64, BUDGET_CHUNK as u64));
    let mut pool =
        WorkerPool::with_budget(workers, HashMap::new(), Arc::clone(&budget)).expect("pool");
    let t0 = Instant::now();
    for _ in 0..JOBS {
        let a = Arc::clone(&anh);
        pool.spawn(move |ctx: &mut WorkerCtx| {
            ctx.check_cancel()?;
            let k = chay_hai_engine(&a);
            // In của task vào buffer riêng; `join` merge theo spawn order ⇒ W=1/W=4 phải trùng.
            ctx.print(&k.stdout);
            ctx.charge(BUDGET_CHARGE)?;
            Ok(send_ket_qua(&k))
        })
        .expect("spawn");
    }
    let r = pool.join().expect("join pool");
    let wall = t0.elapsed();
    let results = r
        .outcomes
        .iter()
        .map(|o| o.result.clone().expect("kết quả task"))
        .collect();
    KetQuaPool {
        output: r.output,
        results,
        wall,
        budget: r.budget,
        threads_sau: dem_thread(),
    }
}

/// W=1 vs W>1 cho **ghi top-level `var`**: W=1 ghi env chung; W>1 trả `CrossWrite`.
fn thu_cross_write(workers: usize) -> String {
    let mut root = HashMap::new();
    root.insert("counter".to_string(), SendValue::Int(0));
    let mut pool = WorkerPool::new(workers, root).expect("pool cross");
    for i in 0..JOBS {
        pool.spawn(move |ctx: &mut WorkerCtx| {
            ctx.set("counter", SendValue::Int(i as i64));
            Ok(SendValue::Int(i as i64))
        })
        .expect("spawn cross");
    }
    match pool.join() {
        Ok(r) => {
            assert_eq!(workers, 1, "W>1 không được chấp nhận ghi chéo");
            assert_eq!(
                r.env.get("counter"),
                Some(&SendValue::Int((JOBS - 1) as i64)),
                "W=1 phải thấy env chung sau các lần ghi"
            );
            format!("W={workers} shared-env counter={}", JOBS - 1)
        }
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            assert!(workers > 1, "W=1 không được báo CrossWrite");
            assert_eq!(vars, vec!["counter".to_string()], "vars cross-write");
            assert_eq!(tasks.len(), JOBS, "mọi task ghi đều bị đánh dấu");
            format!("W={workers} CrossWrite vars={vars:?} tasks={}", tasks.len())
        }
        Err(e) => panic!("W={workers} lỗi bất ngờ: {e:?}"),
    }
}

fn main() {
    println!("VS3_W5_PROTO\tbat_dau\tload={}", load());
    let t_tong = Instant::now();
    let threads_truoc = dem_thread();
    let fd_truoc = dem_fd();
    let rss_truoc = rss_kb();

    // (1) Send + Sync — nếu dòng này không biên dịch được thì đó là blocker, không `unsafe`.
    khang_dinh_send_sync::<Hir>();
    khang_dinh_send_sync::<Symbols>();
    khang_dinh_send_sync::<TypeTable>();
    khang_dinh_send_sync::<vis_bytecode::Module>();
    khang_dinh_send_sync::<SendValue>();
    khang_dinh_send_sync::<Anh>();
    println!("SEND_SYNC\tHir\tOK");
    println!("SEND_SYNC\tSymbols\tOK");
    println!("SEND_SYNC\tTypeTable\tOK");
    println!("SEND_SYNC\tModule\tOK");
    println!("SEND_SYNC\tSendValue\tOK");
    println!("SEND_SYNC\tAnh(Arc<Module>)\tOK");

    // (2) Ảnh chương trình dựng một lần; evaluator dựng lại cho mỗi lần chạy.
    let anh = dung_anh(SRC);
    println!(
        "ANH\thir_items={}\tmodule_funcs={}\tmodule_code={}",
        anh.hir.items.len(),
        anh.module.funcs.len(),
        anh.module.total_code()
    );

    // Oracle một luồng (W=1 ngữ nghĩa): interp == VM, steps > 0.
    let oracle = chay_hai_engine(&anh);
    let oracle_out = oracle.stdout.clone();
    assert_eq!(oracle_out.lines().count(), 3, "chương trình phải in 3 dòng");
    println!(
        "ORACLE\tstdout={:?}\texit={}\tsteps_hir={}\tsteps_vm={}",
        oracle_out, oracle.exit, oracle.steps_hir, oracle.steps_vm
    );

    // (3a) Thread OS độc lập: cùng `&Anh` (Sync), mỗi thread tự dựng `Interp`/`Vm`.
    let doc_lap = std::thread::scope(|s| {
        let hs: Vec<_> = (0..JOBS)
            .map(|_| {
                let a = &anh;
                s.spawn(move || {
                    // `vis_interp::run` tạo Interp mới; `Vm::new` tạo VM mới.
                    let k = chay_hai_engine(a);
                    (k.stdout, k.exit, k.steps_hir, k.steps_vm)
                })
            })
            .collect();
        hs.into_iter()
            .map(|h| h.join().expect("thread độc lập panic"))
            .collect::<Vec<_>>()
    });
    for (i, (out, exit, sh, sv)) in doc_lap.iter().enumerate() {
        assert_eq!(out, &oracle_out, "thread {i} stdout lệch oracle");
        assert_eq!(*exit, oracle.exit, "thread {i} exit lệch oracle");
        assert!(*sh > 0 && *sv > 0, "thread {i} steps = 0");
    }
    println!(
        "THREAD_DOC_LAP\tn={}\tbyte_identical=OK\tthreads_sau={:?}",
        JOBS,
        dem_thread()
    );

    // (3b) WorkerPool W=1 vs W=4, 3 vòng lấy median; output merge theo spawn order.
    let mut w1 = Vec::new();
    let mut w4 = Vec::new();
    for _ in 0..VONG_DO {
        w1.push(chay_pool(Arc::clone(&anh), 1));
        w4.push(chay_pool(Arc::clone(&anh), 4));
    }
    let w1_median = median(w1.iter().map(|r| r.wall).collect());
    let w4_median = median(w4.iter().map(|r| r.wall).collect());

    let w1_out = &w1[0].output;
    let w4_out = &w4[0].output;
    let ky_vong: String = (0..JOBS).map(|_| oracle_out.as_str()).collect();
    assert_eq!(w1_out, &ky_vong, "W=1 output lệch oracle/JOBS lần");
    assert_eq!(w4_out, &ky_vong, "W=4 output lệch oracle/JOBS lần");
    assert_eq!(w1_out, w4_out, "W=1 vs W=4 output không byte-identical");
    for (i, r) in w1.iter().enumerate() {
        assert_eq!(r.output, ky_vong, "vòng W=1 #{i} output lệch");
    }
    for (i, r) in w4.iter().enumerate() {
        assert_eq!(r.output, ky_vong, "vòng W=4 #{i} output lệch");
    }
    assert_eq!(w1[0].results, w4[0].results, "W=1 vs W=4 result lệch");
    assert_eq!(w1[0].results.len(), JOBS);
    println!(
        "POOL_EVAL\tW=1\twall_median_ms={:.1}\toutput_bytes={}\tresults={}",
        w1_median.as_secs_f64() * 1000.0,
        w1_out.len(),
        w1[0].results.len()
    );
    println!(
        "POOL_EVAL\tW=4\twall_median_ms={:.1}\toutput_bytes={}\tresults={}\tspeedup={:.2}x",
        w4_median.as_secs_f64() * 1000.0,
        w4_out.len(),
        w4[0].results.len(),
        w1_median.as_secs_f64() / w4_median.as_secs_f64()
    );
    println!(
        "CHECK\tbyte_identical\tOK\tlen={}\tjobs={}",
        w1_out.len(),
        JOBS
    );
    // Mỗi pool phải join sạch: số thread sau mỗi lần chạy quay về mốc trước pool.
    for (ten, r) in [("W=1", &w1), ("W=4", &w4)] {
        if let (Some(t0), Some(t1)) = (threads_truoc, r[0].threads_sau) {
            assert!(t1 <= t0, "{ten}: rò thread sau pool (trước={t0} sau={t1})");
            println!("THREAD_SAU_POOL\t{ten}\tsau={t1}\ttruoc={t0}");
        }
    }

    // (3c) Budget atomic trên cùng pool eval: mọi job charge như nhau ⇒ dùng đúng, không cạn.
    let b1 = w1[0].budget.expect("budget W=1");
    let b4 = w4[0].budget.expect("budget W=4");
    for (ten, b) in [("W=1", &b1), ("W=4", &b4)] {
        assert_eq!(b.limit, BUDGET_LIMIT, "{ten}: limit");
        assert_eq!(
            b.used,
            BUDGET_CHARGE * JOBS as i64,
            "{ten}: used phải đúng số charge"
        );
        assert_eq!(
            b.used + b.remaining,
            BUDGET_LIMIT,
            "{ten}: used + remaining != limit"
        );
        assert!(!b.exhausted, "{ten}: budget không được cạn");
        println!(
            "BUDGET\t{ten}\tused={}\tremaining={}\tlimit={}\texhausted={}",
            b.used, b.remaining, b.limit, b.exhausted
        );
    }

    // (3d) Cross-write: W=1 shared env, W=4 từ chối và không trả env đã sửa.
    let c1 = thu_cross_write(1);
    let c4 = thu_cross_write(4);
    println!("CROSS_WRITE\tW=1\t{c1}");
    println!("CROSS_WRITE\tW=4\t{c4}");
    println!("CHECK\tcross_write\tOK");

    // (3e) Rò thread: mọi pool/scope đã join; số thread phải quay về mốc đầu.
    let threads_sau = dem_thread();
    let fd_sau = dem_fd();
    let rss_sau = rss_kb();
    if let (Some(t0), Some(t1)) = (threads_truoc, threads_sau) {
        assert!(
            t1 <= t0,
            "rò thread: trước={t0} sau={t1} (W=1/W=4 pool đều phải join)"
        );
        println!("THREADS\ttruoc={t0}\tsau={t1}\tro=0\tOK");
    } else {
        println!("THREADS\tn/a (không phải Linux)");
    }
    if let (Some(f0), Some(f1)) = (fd_truoc, fd_sau) {
        assert!(f1 <= f0, "rò fd: trước={f0} sau={f1}");
        println!("FD\ttruoc={f0}\tsau={f1}\tro=0\tOK");
    }
    if let (Some((r0, _)), Some((r1, hwm))) = (rss_truoc, rss_sau) {
        println!("RSS_KB\ttruoc={r0}\tsau={r1}\thwm={hwm}");
    }
    println!(
        "WALL_TONG\tms={:.1}\tload_cuoi={}",
        t_tong.elapsed().as_secs_f64() * 1000.0,
        load()
    );
    println!("KET_LUAN\tPASS\tW=1/W=4 evaluator độc lập, byte-identical, không rò thread");
}
