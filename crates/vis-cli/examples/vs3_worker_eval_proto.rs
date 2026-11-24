// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (prototype, test/example-only)** — evaluator **độc lập** trên worker thread,
//! env-gated cho 10k task + soak; **không** sửa production/ABI/default.
//!
//! Mục tiêu:
//! 1. `Hir`/`Symbols`/`TypeTable`/`vis_bytecode::Module`/`SendValue` là `Send + Sync` bằng
//!    ràng buộc biên dịch (không `unsafe`, không sửa kiểu).
//! 2. Dựng **một** ảnh chương trình (`Arc<Module>` + HIR) rồi cho mỗi job/thread dựng
//!    evaluator **riêng** và chạy cùng chương trình; trả **checksum** thay vì in N dòng.
//! 3. So W=1 vs W=4 **byte-identical** (checksum + output khi N nhỏ), wall/speedup, RSS,
//!    threads, fd; 3 vòng. Ghép `WorkerPool` cho budget + cross-write.
//! 4. Đo chi phí dựng evaluator/task, so với **tái dùng** `Vm` (chỉ chương trình không
//!    async/chan — xem caveat trong journal).
//! 5. Soak W=4 tới `VS3_EVAL_SOAK_SECS` giây, kiểm RSS plateau/thread/fd, dừng nếu sai.
//!
//! Biến môi trường (đều tuỳ chọn):
//! - `VS3_EVAL_N` — số job mỗi lần chạy, mặc định **64**; đo 10k: `10000`.
//! - `VS3_EVAL_W` — 1 hoặc 4, mặc định **4** (dùng cho soak; phần so sánh luôn chạy cả 1 và 4).
//! - `VS3_EVAL_ITERS` — số vòng `work(n)` trong chương trình, mặc định **2000** (tự **1** khi
//!   `N >= 1000` nếu không đặt) để 10k đo overhead evaluator/task thay vì tính toán.
//! - `VS3_EVAL_SOAK_SECS` — soak W=`VS3_EVAL_W` trong bấy nhiêu giây, mặc định **0** (tắt).
//!
//! Chạy thủ công (example, không nằm trong test suite):
//! ```text
//! cargo run -p vis-cli --example vs3_worker_eval_proto
//! VS3_EVAL_N=10000 VS3_EVAL_ITERS=1 cargo run -p vis-cli --example vs3_worker_eval_proto
//! VS3_EVAL_N=10000 VS3_EVAL_ITERS=1 VS3_EVAL_W=4 VS3_EVAL_SOAK_SECS=600 \
//!   cargo run -p vis-cli --example vs3_worker_eval_proto
//! ```
//!
//! Ghi chú: `vis_interp::interp::Interp::new` là `pub(crate)`, nên bên ngoài mỗi lần gọi
//! [`vis_interp::run`] là **một `Interp` mới**. Đường VM dùng `Vm::new` công khai trên
//! `Arc<Module>`. Không có dòng production nào bị sửa bởi example này.

use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use vis_core::RunOutput;
use vis_interp::rt::fee::{BudgetSnapshot, SharedBudget};
use vis_interp::value::worker::{SendValue, WorkerCtx, WorkerError, WorkerPool};
use vis_syntax::{lex, parse, Mode};
use vis_types::{Hir, Symbols, TypeTable};
use vis_vm::vm::Vm;

const N_MAC_DINH: usize = 64;
const W_MAC_DINH: usize = 4;
const ITERS_MAC_DINH: usize = 2_000;
const VONG_DO: usize = 3;
/// In output của task ra pool chỉ khi N nhỏ; N lớn dùng checksum để tránh 10k dòng.
const ECHO_TOI_DA: usize = 256;
const BUDGET_DU: i64 = 1_000;
const BUDGET_CHUNK: i64 = 256;
const SOAK_MAU_GIAY: u64 = 5;
/// Dao động RSS sau warm-up tối đa cho phép trong soak (kB).
const SOAK_RSS_DAO_DONG_KB: u64 = 65_536;

/// Ảnh chương trình **bất biến**, chia sẻ `Arc` cho mọi worker/thread.
struct Anh {
    hir: Hir,
    symbols: Symbols,
    types: TypeTable,
    text: String,
    module: Arc<vis_bytecode::Module>,
}

struct CauHinh {
    n: usize,
    w: usize,
    iters: usize,
    soak_secs: u64,
}

fn khang_dinh_send_sync<T: Send + Sync>() {}

fn bien_usize(ten: &str, mac_dinh: usize) -> usize {
    match std::env::var(ten) {
        Ok(s) => s
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("{ten} không phải số nguyên dương: {s:?}")),
        Err(_) => mac_dinh,
    }
}

fn cau_hinh() -> CauHinh {
    let n = bien_usize("VS3_EVAL_N", N_MAC_DINH);
    let w = bien_usize("VS3_EVAL_W", W_MAC_DINH);
    // N lớn mà người dùng quên `VS3_EVAL_ITERS` ⇒ tự dùng 1, tránh 10k × công việc nặng.
    let iters = match std::env::var("VS3_EVAL_ITERS") {
        Ok(s) => s
            .trim()
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("VS3_EVAL_ITERS không phải số nguyên dương: {s:?}")),
        Err(_) if n >= 1_000 => 1,
        Err(_) => ITERS_MAC_DINH,
    };
    let soak_secs = bien_usize("VS3_EVAL_SOAK_SECS", 0) as u64;
    assert!(n > 0, "VS3_EVAL_N phải > 0");
    assert!(w == 1 || w == 4, "VS3_EVAL_W chỉ nhận 1 hoặc 4, gặp {w}");
    CauHinh {
        n,
        w,
        iters,
        soak_secs,
    }
}

/// Chương trình mẫu: hàm + `var`/`let` + vòng `while` + số học + **một** dòng `print`.
/// `iters` càng nhỏ càng đo overhead evaluator/task thay vì công việc CPU.
fn lam_src(iters: usize) -> String {
    format!(
        r#"fn work(n: int) -> int:
    var acc = 0
    var i = 0
    while i < n:
        acc = acc + (i * 3) % 7
        i = i + 1
    acc

fn main():
    let nhan = 2
    print(work({iters}) * nhan)
"#
    )
}

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

/// Kết quả một job — đủ để checksum, không cần giữ `Value`/`Rc` qua biên worker.
struct KetQua {
    stdout: String,
    exit: i32,
    steps_hir: usize,
    steps_vm: usize,
}

/// Chạy **cả hai engine** trên cùng ảnh; chứng minh HIR + Module dùng được đồng thời.
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

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// Checksum tất định của một job: stdout + exit + steps hai engine + version.
fn checksum(k: &KetQua) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    h = fnv1a(h, b"vs3-w5-eval-v2");
    h = fnv1a(h, k.stdout.as_bytes());
    h = fnv1a(h, &k.exit.to_le_bytes());
    h = fnv1a(h, &(k.steps_hir as u64).to_le_bytes());
    h = fnv1a(h, &(k.steps_vm as u64).to_le_bytes());
    h
}

fn dem_thread() -> Option<usize> {
    std::fs::read_dir("/proc/self/task")
        .ok()
        .map(|it| it.filter_map(Result::ok).count())
}

/// `/proc/self/task` có thể còn entry thoáng qua sau `join` (procfs dọn trễ); đọc vài lần
/// trong ~20 ms, lấy **min** để không báo rò giả. Chỉ dùng ở ranh giới pool/soak.
fn dem_thread_on_dinh() -> Option<usize> {
    let mut min = None;
    for _ in 0..10 {
        if let Some(n) = dem_thread() {
            min = Some(min.map_or(n, |m: usize| m.min(n)));
            if n <= 1 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    min
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

struct KetQuaPool {
    output: String,
    results: Vec<i64>,
    wall: Duration,
    budget: Option<BudgetSnapshot>,
    threads_sau: Option<usize>,
    fd_sau: Option<usize>,
}

/// Chạy `n` job trên pool W=1/W=N. Mỗi job dựng evaluator riêng từ `Arc<Anh>`, trả checksum;
/// `ctx.print` chỉ khi `echo` (N nhỏ) để kiểm merge output theo spawn order.
fn chay_pool(anh: Arc<Anh>, workers: usize, n: usize, echo: bool) -> KetQuaPool {
    let limit = n as i64 + BUDGET_DU;
    let budget = Arc::new(SharedBudget::new(limit as u64, BUDGET_CHUNK as u64));
    let mut pool =
        WorkerPool::with_budget(workers, HashMap::new(), Arc::clone(&budget)).expect("pool");
    let t0 = Instant::now();
    for _ in 0..n {
        let a = Arc::clone(&anh);
        pool.spawn(move |ctx: &mut WorkerCtx| {
            ctx.check_cancel()?;
            let k = chay_hai_engine(&a);
            if echo {
                ctx.print(&k.stdout);
            }
            ctx.charge(1)?;
            Ok(SendValue::Int(checksum(&k) as i64))
        })
        .expect("spawn");
    }
    let r = pool.join().expect("join pool");
    let wall = t0.elapsed();
    let results = r
        .outcomes
        .iter()
        .map(|o| match &o.result {
            Ok(SendValue::Int(x)) => *x,
            other => panic!("kết quả task không phải Int: {other:?}"),
        })
        .collect();
    KetQuaPool {
        output: r.output,
        results,
        wall,
        budget: r.budget,
        threads_sau: dem_thread_on_dinh(),
        fd_sau: dem_fd(),
    }
}

/// W=1 vs W>1 cho **ghi top-level `var`**: W=1 ghi env chung; W>1 trả `CrossWrite`.
fn thu_cross_write(workers: usize) -> String {
    let mut root = HashMap::new();
    root.insert("counter".to_string(), SendValue::Int(0));
    let mut pool = WorkerPool::new(workers, root).expect("pool cross");
    for i in 0..4 {
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
                Some(&SendValue::Int(3)),
                "W=1 phải thấy env chung sau các lần ghi"
            );
            "W=1 shared-env counter=3".to_string()
        }
        Err(WorkerError::CrossWrite { vars, tasks }) => {
            assert!(workers > 1, "W=1 không được báo CrossWrite");
            assert_eq!(vars, vec!["counter".to_string()], "vars cross-write");
            assert_eq!(tasks.len(), 4, "mọi task ghi đều bị đánh dấu");
            format!("W={workers} CrossWrite vars={vars:?} tasks={}", tasks.len())
        }
        Err(e) => panic!("W={workers} lỗi bất ngờ: {e:?}"),
    }
}

/// Chi phí dựng `Vm` (chỉ constructor) cho `n` task.
fn do_dung_vm(anh: &Anh, n: usize) -> Duration {
    let t = Instant::now();
    for _ in 0..n {
        black_box(Vm::new(&anh.module, &anh.text));
    }
    t.elapsed()
}

/// VM mới cho **mỗi** task: `Vm::new` + `run`.
fn do_vm_moi_lan(anh: &Anh, n: usize) -> (Duration, u64) {
    let t = Instant::now();
    let mut acc = 0u64;
    for _ in 0..n {
        let mut vm = Vm::new(&anh.module, &anh.text);
        let o = vm.run(&[]);
        assert!(o.is_clean(), "vm mới: {:?}", o.codes());
        acc ^= checksum(&KetQua {
            stdout: o.stdout,
            exit: o.exit,
            steps_hir: 0,
            steps_vm: o.steps,
        });
    }
    (t.elapsed(), acc)
}

/// **Tái dùng một `Vm`** cho `n` task. Chỉ đúng cho chương trình không `spawn`/`chan`:
/// `Vm::run` reset steps/globals/stdout nhưng `Scheduler` không được reset (xem journal).
fn do_vm_tai_su_dung(anh: &Anh, n: usize) -> (Duration, u64) {
    let t = Instant::now();
    let mut vm = Vm::new(&anh.module, &anh.text);
    let mut acc = 0u64;
    for _ in 0..n {
        let o = vm.run(&[]);
        assert!(o.is_clean(), "vm tái dùng: {:?}", o.codes());
        acc ^= checksum(&KetQua {
            stdout: o.stdout,
            exit: o.exit,
            steps_hir: 0,
            steps_vm: o.steps,
        });
    }
    (t.elapsed(), acc)
}

/// `vis_interp::run` cho mỗi task — **không** tái dùng được vì `Interp::new` là `pub(crate)`.
fn do_interp_moi_lan(anh: &Anh, n: usize) -> Duration {
    let t = Instant::now();
    for _ in 0..n {
        let o = chay_hir(anh);
        assert!(o.is_clean(), "interp mới: {:?}", o.codes());
        black_box(o);
    }
    t.elapsed()
}

struct KetQuaSoak {
    batches: u64,
    jobs: u64,
    rss_min: u64,
    rss_max: u64,
    hwm: u64,
    threads_sau: Option<usize>,
    fd_sau: Option<usize>,
}

/// Soak W=`cfg.w` tới `cfg.soak_secs`: mỗi batch `cfg.n` job, so checksum với oracle, kiểm
/// thread/fd sau mỗi batch và RSS plateau sau warm-up. Sai checksum ⇒ panic ngay.
fn soak(anh: &Arc<Anh>, cfg: &CauHinh, oracle: i64) -> KetQuaSoak {
    let threads_truoc = dem_thread_on_dinh();
    let fd_truoc = dem_fd();
    let ky_vong = vec![oracle; cfg.n];
    let t0 = Instant::now();
    let mut batches = 0u64;
    let mut jobs = 0u64;
    let mut rss_samples: Vec<u64> = Vec::new();
    let mut last_print = Instant::now();
    let mut hwm = 0u64;
    let mut rss_min = u64::MAX;
    let mut rss_max = 0u64;
    while t0.elapsed() < Duration::from_secs(cfg.soak_secs) {
        let r = chay_pool(Arc::clone(anh), cfg.w, cfg.n, false);
        assert_eq!(
            r.results, ky_vong,
            "soak: checksum lệch oracle ở batch {batches}"
        );
        assert!(r.output.is_empty(), "soak N lớn không được echo output");
        batches += 1;
        jobs += cfg.n as u64;
        if let (Some(t0n), Some(t1)) = (threads_truoc, r.threads_sau) {
            assert!(
                t1 <= t0n,
                "soak: rò thread sau batch {batches}: {t0n}->{t1}"
            );
        }
        if let (Some(f0), Some(f1)) = (fd_truoc, r.fd_sau) {
            assert!(f1 <= f0 + 4, "soak: rò fd sau batch {batches}: {f0}->{f1}");
        }
        if let Some((rss, h)) = rss_kb() {
            hwm = hwm.max(h);
            rss_samples.push(rss);
            // Warm-up 30 s (hoặc 3 batch đầu) rồi mới xét plateau.
            if t0.elapsed() >= Duration::from_secs(30) || batches <= 3 {
                rss_min = rss_min.min(rss);
                rss_max = rss_max.max(rss);
            }
        }
        if last_print.elapsed() >= Duration::from_secs(SOAK_MAU_GIAY) {
            println!(
                "SOAK\telapsed_s={:.1}\tbatches={batches}\tjobs={jobs}\trss_kb={:?}\tthreads={:?}\tfd={:?}\tload={}",
                t0.elapsed().as_secs_f64(),
                rss_samples.last(),
                r.threads_sau,
                r.fd_sau,
                load()
            );
            last_print = Instant::now();
        }
    }
    if rss_min == u64::MAX {
        rss_min = 0;
    }
    assert!(
        rss_max.saturating_sub(rss_min) <= SOAK_RSS_DAO_DONG_KB,
        "soak: RSS không plateau: min={rss_min} max={rss_max} kB"
    );
    KetQuaSoak {
        batches,
        jobs,
        rss_min,
        rss_max,
        hwm,
        threads_sau: dem_thread_on_dinh(),
        fd_sau: dem_fd(),
    }
}

fn main() {
    let cfg = cau_hinh();
    let src = lam_src(cfg.iters);
    println!(
        "VS3_W5_PROTO\tbat_dau\tN={}\tW={}\titers={}\tsoak_s={}\tload={}",
        cfg.n,
        cfg.w,
        cfg.iters,
        cfg.soak_secs,
        load()
    );
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
    let anh = dung_anh(&src);
    println!(
        "ANH\thir_items={}\tmodule_funcs={}\tmodule_code={}\tsrc_bytes={}",
        anh.hir.items.len(),
        anh.module.funcs.len(),
        anh.module.total_code(),
        src.len()
    );

    // Oracle một job (interp == VM, steps > 0) — mọi job phải cho cùng checksum.
    let oracle = chay_hai_engine(&anh);
    let oracle_cs = checksum(&oracle) as i64;
    let echo = cfg.n <= ECHO_TOI_DA;
    let ky_vong = vec![oracle_cs; cfg.n];
    let ky_vong_out = if echo {
        oracle.stdout.repeat(cfg.n)
    } else {
        String::new()
    };
    println!(
        "ORACLE\tstdout={:?}\texit={}\tsteps_hir={}\tsteps_vm={}\tchecksum={oracle_cs}\techo={echo}",
        oracle.stdout, oracle.exit, oracle.steps_hir, oracle.steps_vm
    );

    // (3) W=1 vs W=4, 3 vòng; output chỉ echo khi N nhỏ.
    let mut w1_walls = Vec::with_capacity(VONG_DO);
    let mut w4_walls = Vec::with_capacity(VONG_DO);
    let mut w1_out = String::new();
    let mut w4_out = String::new();
    let mut b1: Option<BudgetSnapshot> = None;
    let mut b4: Option<BudgetSnapshot> = None;
    for vong in 0..VONG_DO {
        let r1 = chay_pool(Arc::clone(&anh), 1, cfg.n, echo);
        assert_eq!(r1.results, ky_vong, "vòng {vong} W=1 checksum lệch");
        assert_eq!(r1.output, ky_vong_out, "vòng {vong} W=1 output lệch");
        let b = r1.budget.expect("budget W=1");
        assert_eq!(b.used, cfg.n as i64, "W=1 used");
        assert_eq!(b.remaining, BUDGET_DU, "W=1 remaining");
        assert_eq!(b.used + b.remaining, b.limit, "W=1 used+remaining");
        if vong == 0 {
            w1_out = r1.output.clone();
            b1 = Some(b);
        }
        if let (Some(t0), Some(t1)) = (threads_truoc, r1.threads_sau) {
            assert!(t1 <= t0, "vòng {vong} W=1 rò thread: {t0}->{t1}");
        }

        let r4 = chay_pool(Arc::clone(&anh), 4, cfg.n, echo);
        assert_eq!(r4.results, ky_vong, "vòng {vong} W=4 checksum lệch");
        assert_eq!(r4.output, ky_vong_out, "vòng {vong} W=4 output lệch");
        let b = r4.budget.expect("budget W=4");
        assert_eq!(b.used, cfg.n as i64, "W=4 used");
        assert_eq!(b.remaining, BUDGET_DU, "W=4 remaining");
        assert_eq!(b.used + b.remaining, b.limit, "W=4 used+remaining");
        if vong == 0 {
            w4_out = r4.output.clone();
            b4 = Some(b);
        }
        if let (Some(t0), Some(t1)) = (threads_truoc, r4.threads_sau) {
            assert!(t1 <= t0, "vòng {vong} W=4 rò thread: {t0}->{t1}");
        }

        println!(
            "VONG\t{vong}\tW=1_ms={:.1}\tW=4_ms={:.1}\tW=1_threads={:?}\tW=4_threads={:?}\tW=1_fd={:?}\tW=4_fd={:?}",
            r1.wall.as_secs_f64() * 1000.0,
            r4.wall.as_secs_f64() * 1000.0,
            r1.threads_sau,
            r4.threads_sau,
            r1.fd_sau,
            r4.fd_sau
        );
        w1_walls.push(r1.wall);
        w4_walls.push(r4.wall);
    }
    let w1_med = median(w1_walls);
    let w4_med = median(w4_walls);
    assert_eq!(w1_out, w4_out, "W=1 vs W=4 output không byte-identical");
    assert_eq!(w1_out, ky_vong_out, "W=1 output lệch oracle × N");
    assert_eq!(w4_out, ky_vong_out, "W=4 output lệch oracle × N");
    println!(
        "POOL_EVAL\tW=1\twall_median_ms={:.1}\tN={}\techo={echo}\tchecksum={oracle_cs}",
        w1_med.as_secs_f64() * 1000.0,
        cfg.n
    );
    println!(
        "POOL_EVAL\tW=4\twall_median_ms={:.1}\tN={}\techo={echo}\tchecksum={oracle_cs}\tspeedup={:.2}x",
        w4_med.as_secs_f64() * 1000.0,
        cfg.n,
        w1_med.as_secs_f64() / w4_med.as_secs_f64()
    );
    println!(
        "CHECK\tbyte_identical\tOK\tchecksum={oracle_cs}\tresults={}\toutput_bytes={}",
        cfg.n,
        w1_out.len()
    );
    for (ten, b) in [
        ("W=1", b1.expect("budget W=1")),
        ("W=4", b4.expect("budget W=4")),
    ] {
        println!(
            "BUDGET\t{ten}\tused={}\tremaining={}\tlimit={}\texhausted={}",
            b.used, b.remaining, b.limit, b.exhausted
        );
    }
    let c1 = thu_cross_write(1);
    let c4 = thu_cross_write(4);
    println!("CROSS_WRITE\tW=1\t{c1}");
    println!("CROSS_WRITE\tW=4\t{c4}");
    println!("CHECK\tcross_write\tOK");

    // (4) Chi phí dựng evaluator/task + tái dùng `Vm` (chương trình không async/chan).
    let n_do = cfg.n.min(10_000);
    let d_dung = do_dung_vm(&anh, n_do);
    let (d_moi, cs_moi) = do_vm_moi_lan(&anh, n_do);
    let (d_reuse, cs_reuse) = do_vm_tai_su_dung(&anh, n_do);
    let d_interp = do_interp_moi_lan(&anh, n_do);
    assert_eq!(cs_moi, cs_reuse, "VM mới vs tái dùng checksum lệch");
    println!(
        "EVAL_COST\tN={n_do}\tvm_new_us/task={:.2}\tvm_fresh_us/task={:.2}\tvm_reuse_us/task={:.2}\tvm_reuse_speedup={:.2}x\tinterp_fresh_us/task={:.2}",
        d_dung.as_secs_f64() * 1e6 / n_do as f64,
        d_moi.as_secs_f64() * 1e6 / n_do as f64,
        d_reuse.as_secs_f64() * 1e6 / n_do as f64,
        d_moi.as_secs_f64() / d_reuse.as_secs_f64(),
        d_interp.as_secs_f64() * 1e6 / n_do as f64
    );
    println!(
        "EVAL_COST\tcaveat=vm_reuse_chỉ_đúng_khi_không_spawn/chan;Interp_new_pub(crate)_không_tái_dùng_được"
    );

    // (5) Soak (mặc định tắt) — W=cfg.w, N=cfg.n, dừng nếu checksum sai.
    if cfg.soak_secs > 0 {
        let s = soak(&anh, &cfg, oracle_cs);
        println!(
            "SOAK_TONG\tsecs={}\tW={}\tN={}\tbatches={}\tjobs={}\trss_min_kb={}\trss_max_kb={}\thwm_kb={}\tthreads={:?}\tfd={:?}",
            cfg.soak_secs,
            cfg.w,
            cfg.n,
            s.batches,
            s.jobs,
            s.rss_min,
            s.rss_max,
            s.hwm,
            s.threads_sau,
            s.fd_sau
        );
    } else {
        println!("SOAK\tskip\tVS3_EVAL_SOAK_SECS=0");
    }

    let threads_sau = dem_thread_on_dinh();
    let fd_sau = dem_fd();
    let rss_sau = rss_kb();
    if let (Some(t0), Some(t1)) = (threads_truoc, threads_sau) {
        assert!(t1 <= t0, "rò thread: trước={t0} sau={t1}");
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
    println!(
        "KET_LUAN\tPASS\tN={}\tW=1/W=4 evaluator độc lập, checksum byte-identical, không rò thread",
        cfg.n
    );
}
