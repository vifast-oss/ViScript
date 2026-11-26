// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-317b §4.4** — 10 test đặc tả cho orchestration worker **dùng chung**
//! (`vis_runtime::worker::{ap_dung_outcome, flush_proxy, rut_outcome, loi_task, ma_ghi_cheo}`).
//!
//! Hợp đồng bất biến qua refactor:
//! - output merge theo **spawn order** (không theo thời điểm xong);
//! - ghi top-level từ worker W>1 ⇒ `VS0509` ("ghi chéo worker");
//! - task bị huỷ ⇒ `VS0506` ("tác vụ bị huỷ");
//! - giá trị không Sendable ⇒ `VS0508` (guard trước khi tạo job);
//! - await/flush đúng **một lần**; final join surface lỗi task không ai await;
//! - `Off`/`Local` không đổi; interp ↔ VM parity cùng nguồn.

use std::collections::HashMap;
use std::sync::Arc;

use vis_core::value::Value;
use vis_core::RunOutput;
use vis_runtime::worker::{SendValue, ToSend, WorkerPool};
use vis_syntax::{lex, parse, Mode};

type Anh = (
    vis_hir::Hir,
    vis_hir::Symbols,
    vis_hir::TypeTable,
    vis_bytecode::Module,
);

fn anh(src: &str) -> Anh {
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
    let m = vis_bytecode::compile(&h.hir, &h.symbols, src).expect("compile");
    (h.hir, h.symbols, h.types, m)
}

fn interp(src: &str, a: &Anh, w: usize) -> RunOutput {
    if w == 1 {
        vis_interp::run_workers_local(src, &a.0, &a.1, &a.2, &[])
    } else {
        vis_interp::run_workers_parallel(src, &a.0, &a.1, &a.2, &[], w)
    }
}

fn vm(src: &str, a: &Anh, w: usize) -> RunOutput {
    if w == 1 {
        vis_vm::run_workers_local_vm(&a.3, src, &[])
    } else {
        vis_vm::run_workers_parallel_vm(&a.3, src, &[], w)
    }
}

fn frame_1(src: &str, a: &Anh) -> RunOutput {
    vis_interp::run_async_with_hosts_caps(src, &a.0, &a.1, &a.2, &[], None, None, None, None, &[])
}

fn frame_w(src: &str, a: &Anh, w: usize) -> RunOutput {
    vis_interp::run_async_workers_with_hosts_caps(
        src,
        &a.0,
        &a.1,
        &a.2,
        &[],
        None,
        None,
        None,
        None,
        &[],
        w,
        None,
        None,
    )
}

/// Frame W=1 của **VM** (production `vis run --vm` cho chương trình async).
fn frame_vm_1(src: &str, a: &Anh) -> RunOutput {
    vis_vm::run_async_with_full_hosts_caps(&a.3, src, &[], None, None, None, None, &[])
}

/// Frame W=2/4 của **VM** (production `vis run --vm --workers N` cho chương trình async).
fn frame_vm_w(src: &str, a: &Anh, w: usize) -> RunOutput {
    vis_vm::run_async_workers_with_full_hosts_caps(
        &a.3,
        src,
        &[],
        None,
        None,
        None,
        None,
        &[],
        w,
        None,
        None,
    )
}

fn so_khop(a: &RunOutput, b: &RunOutput, nhan: &str) {
    assert_eq!(a.stdout, b.stdout, "{nhan}: stdout");
    assert_eq!(a.exit, b.exit, "{nhan}: exit");
    assert_eq!(a.codes(), b.codes(), "{nhan}: codes");
    assert_eq!(
        a.value.as_ref().map(Value::show),
        b.value.as_ref().map(Value::show),
        "{nhan}: giá trị cuối main"
    );
}

const IN_THEO_THU_TU: &str = r#"async fn t(n: int) -> int:
    print(n)
    n * 10

fn main():
    let a = spawn t(1)
    let b = spawn t(2)
    let c = spawn t(3)
    print((await a) + (await b) + (await c))
"#;

/// **W1** — output của nhiều task merge theo **spawn order**, không theo thứ tự hoàn thành; cả
/// hai engine cho cùng stdout.
#[test]
fn w1_output_theo_spawn_order() {
    let a = anh(IN_THEO_THU_TU);
    let i1 = interp(IN_THEO_THU_TU, &a, 1);
    assert_eq!(i1.stdout, "1\n2\n3\n60\n");
    for w in [2usize, 4] {
        let i = interp(IN_THEO_THU_TU, &a, w);
        let v = vm(IN_THEO_THU_TU, &a, w);
        assert_eq!(i.stdout, i1.stdout, "interp W={w}");
        assert_eq!(v.stdout, i1.stdout, "VM W={w}");
        so_khop(&i, &v, &format!("W={w}"));
    }
}

const GHI_CHEO: &str = r#"var counter = 0

async fn tang():
    counter = counter + 1

fn main():
    let t = spawn tang()
    await t
    print(counter)
"#;

/// **W2** — worker W>1 ghi top-level ⇒ `VS0509` ("ghi chéo worker") ở **cả hai** engine; W=1
/// (shared env) vẫn chạy đúng.
#[test]
fn w2_ghi_cheo_vs0509_ca_hai_engine() {
    let a = anh(GHI_CHEO);
    for out in [interp(GHI_CHEO, &a, 1), vm(GHI_CHEO, &a, 1)] {
        assert_eq!(out.exit, 0, "W=1: {:?}", out.diagnostics);
        assert_eq!(out.stdout, "1\n");
    }
    for w in [2usize, 4] {
        for (nhan, out) in [
            ("interp", interp(GHI_CHEO, &a, w)),
            ("vms", vm(GHI_CHEO, &a, w)),
        ] {
            assert_eq!(out.exit, 1, "{nhan} W={w} phải lỗi");
            assert!(
                out.diagnostics
                    .iter()
                    .any(|d| d.message.contains("ghi chéo worker")),
                "{nhan} W={w}: {:?}",
                out.diagnostics
            );
        }
    }
}

const NANG: &str = r#"async fn nang(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    s

fn main():
    let t = spawn nang(2000000)
    print(await t)
"#;

/// **W3** — huỷ hợp tác giữa chừng (`cancel_after`): task bị huỷ ⇒ `VS0506`, exit 1, cả hai
/// engine, không panic.
#[test]
fn w3_huy_giua_chung_vs0506() {
    let a = anh(NANG);
    let i = vis_interp::run_workers_parallel_cancel_after(NANG, &a.0, &a.1, &a.2, &[], 2, 5);
    let v = vis_vm::run_workers_parallel_vm_cancel_after(&a.3, NANG, &[], 2, 5);
    for (nhan, out) in [("interp", i), ("vm", v)] {
        assert_eq!(out.exit, 1, "{nhan}: phải dừng khi huỷ");
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code.short == "VS0506" && d.message.contains("huỷ")),
            "{nhan}: {:?}",
            out.diagnostics
        );
    }
}

/// **W4** — giá trị **không Sendable** (cap/closure/task) bị từ chối **trước** khi tạo job;
/// message mang `VS0508` (guard dùng chung `ToSend`).
#[test]
fn w4_khong_sendable_vs0508() {
    let cap = Value::cap("Dir", "/tmp", false);
    let e = cap.to_send().expect_err("cap không Sendable");
    assert!(e.contains("VS0508"), "message phải nêu VS0508: {e}");
    assert!(e.contains("không Sendable"), "{e}");
    // Range nội bộ cũng bị chặn (không lọt qua snapshot worker).
    let range = Value::Range(0, 3, false);
    assert!(range.to_send().is_err());

    // Ca **program-level**: đối số là giá trị hàm ⇒ `VS0508` ngay tại spawn.
    const ARG: &str = r#"fn tang(n: int) -> int:
    n + 1

async fn dung(f: fn(int) -> int) -> int:
    f(1)

fn main():
    print(await spawn dung(tang))
"#;
    let a = anh(ARG);
    let out = interp(ARG, &a, 2);
    assert_ne!(out.exit, 0, "spawn với giá trị hàm phải lỗi");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("VS0508")),
        "{:?}",
        out.diagnostics
    );

    // Ca **program-level**: kết quả task là closure/hàm ⇒ `VS0508` khi đọc kết quả.
    const KQ: &str = r#"fn tang(n: int) -> int:
    n + 1

async fn tao() -> fn(int) -> int:
    tang

fn main():
    let t = spawn tao()
    let f = await t
    print(f(1))
"#;
    let a = anh(KQ);
    let out = interp(KQ, &a, 2);
    assert_ne!(out.exit, 0, "kết quả không Sendable phải lỗi");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("VS0508")),
        "{:?}",
        out.diagnostics
    );
}

const HAI_LAN_AWAIT: &str = r#"async fn tao() -> int:
    7

fn main():
    let t = spawn tao()
    print(await t)
    print(await t)
"#;

/// **W5** — `await` cùng task hai lần: thân chạy **một** lần (output flush một lần), kết quả
/// cache ổn định; cả hai engine cho cùng stdout.
#[test]
fn w5_await_hai_lan_mot_lan_chay() {
    let a = anh(HAI_LAN_AWAIT);
    for w in [1usize, 2, 4] {
        let i = interp(HAI_LAN_AWAIT, &a, w);
        let v = vm(HAI_LAN_AWAIT, &a, w);
        assert_eq!(i.stdout, "7\n7\n", "interp W={w}");
        so_khop(&i, &v, &format!("W={w}"));
    }
}

const LOI_KHONG_AWAIT: &str = r#"async fn hong() -> int:
    let a = 10
    let b = 0
    a % b

fn main():
    let t = spawn hong()
    print("sau")
"#;

/// **W6** — **lỗi thân task không await** phải được `final_join` surface (`VS0506`) trên
/// **frame API** (production `vis run --workers` dùng đúng đường này), cả W=1 và W=2/4.
#[test]
fn w6_loi_task_khong_await_frame_final_join() {
    let a = anh(LOI_KHONG_AWAIT);
    let w1 = frame_1(LOI_KHONG_AWAIT, &a);
    assert_eq!(w1.exit, 1, "W=1: {:?}", w1.diagnostics);
    assert!(
        w1.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0506" && d.message.contains("hong")),
        "W=1: {:?}",
        w1.diagnostics
    );
    for w in [2usize, 4] {
        let out = frame_w(LOI_KHONG_AWAIT, &a, w);
        assert_eq!(out.exit, 1, "frame W={w}: {:?}", out.diagnostics);
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code.short == "VS0506" && d.message.contains("hong")),
            "frame W={w}: {:?}",
            out.diagnostics
        );
    }
}

/// **W7** — **frame mode**: `await` task worker khi chưa xong trả `Pending` (suspend) rồi được
/// driver resume; kết quả byte-identical W=1, task chạy đúng một lần, không block.
#[test]
fn w7_frame_mode_suspend_roi_resume() {
    const SRC: &str = r#"async fn tinh(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    print(s)
    s

fn main():
    let t = spawn tinh(1000)
    print((await t) + 1)
"#;
    let a = anh(SRC);
    let w1 = vis_interp::run_async_with_hosts_caps(
        SRC,
        &a.0,
        &a.1,
        &a.2,
        &[],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(w1.stdout, "499500\n499501\n");
    for w in [2usize, 4] {
        let out = frame_w(SRC, &a, w);
        assert_eq!(out.stdout, w1.stdout, "frame W={w}");
        assert_eq!(out.exit, w1.exit, "frame W={w}");
        assert!(out.steps > 0, "chống xanh-giả frame W={w}");
    }
}

const DON_GIAN: &str = r#"async fn t(n: int) -> int:
    n + 1

fn main():
    print(await spawn t(4))
"#;

/// **W8** — `Local`/`Off` giữ đường cũ: runner Local cho cùng stdout/exit như chạy mặc định.
#[test]
fn w8_local_khong_doi_duong_cu() {
    let a = anh(DON_GIAN);
    let mac_dinh = vis_interp::run(DON_GIAN, &a.0, &a.1, &a.2, &[]);
    let local = vis_interp::run_workers_local(DON_GIAN, &a.0, &a.1, &a.2, &[]);
    assert_eq!(mac_dinh.stdout, "5\n");
    so_khop(&mac_dinh, &local, "Off vs Local");
}

const TRON: &str = r#"async fn tinh(n: int) -> int:
    var s = 0
    var i = 0
    while i < n:
        s = s + i
        i = i + 1
    print(s)
    s

fn main():
    let a = spawn tinh(1000)
    let b = spawn tinh(2000)
    print(await a + await b)
"#;

/// **W9** — parity interp ↔ VM trên cùng workload W=1/2/4 (stdout/exit/codes/giá trị cuối).
#[test]
fn w9_parity_hai_engine() {
    let a = anh(TRON);
    for w in [1usize, 2, 4] {
        let i = interp(TRON, &a, w);
        let v = vm(TRON, &a, w);
        assert_eq!(i.stdout, "499500\n1999000\n2498500\n", "interp W={w}");
        so_khop(&i, &v, &format!("W={w}"));
    }
}

/// **W10** — huỷ **idempotent** trên **cùng handle**: cancel 2 lần trước khi chạy và 2 lần sau
/// khi task đã xong — không panic, cờ huỷ atomic, kết quả đã chốt không đổi.
#[test]
fn w10_cancel_idempotent_tren_cung_handle() {
    // (a) Task đã xong rồi mới cancel 2 lần: kết quả giữ nguyên, không panic.
    let mut pool = WorkerPool::new(1, HashMap::new()).expect("pool");
    let snap = Arc::new(SendValue::Int(0));
    let h = pool
        .spawn_shared(snap, |_ctx, _snap| Ok(SendValue::Int(7)))
        .expect("spawn");
    let kq = pool.join().expect("join");
    assert_eq!(kq.outcomes.len(), 1);
    assert_eq!(
        kq.outcomes[0].result.as_ref().ok(),
        Some(&SendValue::Int(7))
    );
    assert!(!kq.outcomes[0].cancelled, "task đã xong không bị huỷ");
    h.cancel();
    h.cancel();
    assert!(h.is_cancelled());

    // (b) Cancel 2 lần trước khi join: một outcome duy nhất, cờ cancelled, không panic.
    let mut pool = WorkerPool::new(1, HashMap::new()).expect("pool");
    let snap = Arc::new(SendValue::Int(0));
    let h = pool
        .spawn_shared(snap, |_ctx, _snap| Ok(SendValue::Int(9)))
        .expect("spawn");
    h.cancel();
    h.cancel();
    let kq = pool.join().expect("join");
    assert_eq!(kq.outcomes.len(), 1, "cancel 2 lần không nhân đôi outcome");
    assert!(kq.outcomes[0].cancelled, "task bị huỷ trước khi chạy");
}

// ---------------------------------------------------------------------------
// **ST-318a** — đóng nợ `#203` (frame W=1 ghi biến cấp tệp) + `#204` (non-frame W≥2
// nuốt lỗi task không await).
// ---------------------------------------------------------------------------

/// **#203** — cùng `GHI_CHEO` (task ghi `var` top-level rồi main đọc): **frame W=1** của hai
/// engine phải in `1` + exit 0. Trước ST-318a interp giữ `Env` riêng nên in `0`, VM in `1`.
#[test]
fn w11_frame_w1_parity_ghi_bien_cap_tep() {
    let a = anh(GHI_CHEO);
    let i = frame_1(GHI_CHEO, &a);
    let v = frame_vm_1(GHI_CHEO, &a);
    assert_eq!(i.stdout, "1\n", "interp frame W=1: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "1\n", "VM frame W=1: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp frame W=1 exit");
    assert_eq!(v.exit, 0, "VM frame W=1 exit");
    so_khop(&i, &v, "frame W=1");
}

/// **#203** — như trên nhưng lệnh ghi nằm trong **hàm được gọi** (env tạm bị pop): vẫn phải
/// chung một ô top-level, kể cả khi đọc lại ngay trong cùng step của main.
#[test]
fn w11_frame_w1_ghi_qua_ham_duoc_goi() {
    const SRC: &str = r#"var g = 0

async fn dummy() -> int:
    1

fn tang():
    g = g + 1

fn main():
    let t = spawn dummy()
    await t
    tang()
    print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "1\n", "interp frame W=1: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "1\n", "VM frame W=1: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    so_khop(&i, &v, "frame W=1 qua hàm");
}

/// **#203 (P3)** — chương trình **không có `main`**: thân cấp tệp chạy trên cùng `globals` với
/// task frame, `var` top-level task ghi thấy ngay ở lệnh `print` cấp tệp — parity interp ↔ VM.
#[test]
fn w11_frame_w1_no_main_ghi_bien_cap_tep() {
    const SRC: &str = r#"var g = 0

async fn tang():
    g = g + 1

let t = spawn tang()
await t
print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "1\n", "interp frame no-main: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "1\n", "VM frame no-main: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    so_khop(&i, &v, "frame W=1 no-main");
}

/// **#204** — non-frame `run_workers_parallel*`/`run_workers_parallel_vm*` W=2/4: lỗi thân task
/// **không ai await** phải được `final_join` surface `VS0506` (cùng message/span như frame W=1),
/// không còn exit 0 + diagnostics rỗng. Output sạch của main vẫn giữ nguyên thứ tự.
#[test]
fn w12_non_frame_loi_task_khong_await_w2_w4() {
    let a = anh(LOI_KHONG_AWAIT);
    let frame = frame_1(LOI_KHONG_AWAIT, &a);
    let msg_frame = frame
        .diagnostics
        .iter()
        .find(|d| d.code.short == "VS0506")
        .map(|d| d.message.clone())
        .expect("frame W=1 phải có VS0506");
    for w in [2usize, 4] {
        for (nhan, out) in [
            ("interp", interp(LOI_KHONG_AWAIT, &a, w)),
            ("vm", vm(LOI_KHONG_AWAIT, &a, w)),
        ] {
            assert_eq!(out.exit, 1, "{nhan} W={w} phải lỗi");
            assert_eq!(out.stdout, "sau\n", "{nhan} W={w}: output trước trap");
            let d = out
                .diagnostics
                .iter()
                .find(|d| d.code.short == "VS0506" && d.message.contains("hong"))
                .unwrap_or_else(|| panic!("{nhan} W={w}: {:?}", out.diagnostics));
            assert_eq!(d.message, msg_frame, "{nhan} W={w}: message như frame W=1");
        }
    }
}

/// **P2-B (ST-318a F1)** — task lỗi đứng **trước**, task sạch in `b` đứng **sau**: `final_join`
/// phải flush output **mọi** task rồi mới trả trap đầu — không được cắt stdout của task sau.
const LOI_TRUOC_OK_SAU: &str = r#"async fn hong() -> int:
    let a = 10
    let b = 0
    a % b

async fn ok() -> int:
    print("b")
    1

fn main():
    let _t = spawn hong()
    let _u = spawn ok()
    print("sau")
"#;

#[test]
fn w13_final_join_flush_het_output_truoc_trap() {
    let a = anh(LOI_TRUOC_OK_SAU);
    let f = frame_1(LOI_TRUOC_OK_SAU, &a);
    assert_eq!(f.stdout, "sau\nb\n", "frame W=1: {:?}", f.diagnostics);
    assert_eq!(f.exit, 1);
    for w in [2usize, 4] {
        for (nhan, out) in [
            ("interp", interp(LOI_TRUOC_OK_SAU, &a, w)),
            ("vm", vm(LOI_TRUOC_OK_SAU, &a, w)),
        ] {
            assert_eq!(out.stdout, "sau\nb\n", "{nhan} W={w}: stdout");
            assert_eq!(out.exit, 1, "{nhan} W={w} phải lỗi");
            assert!(
                out.diagnostics
                    .iter()
                    .any(|d| d.code.short == "VS0506" && d.message.contains("hong")),
                "{nhan} W={w}: {:?}",
                out.diagnostics
            );
        }
    }
}

/// **F2 (P2-C)** — dirty-set: nhiều `var` top-level (200) nhưng chỉ 3 sym bị ghi trong task;
/// cả 3 phải thấy ở `main` (và 1 sym không ghi giữ nguyên), parity interp ↔ VM.
#[test]
fn w14_frame_w1_dirty_set_nhieu_var() {
    let mut src = String::new();
    for i in 0..200 {
        src.push_str(&format!("var v{i} = {i}\n"));
    }
    src.push_str(
        r#"
async fn tang_a():
    v0 = v0 + 1
    v7 = v7 + 10

async fn tang_b():
    v199 = v199 + 2

fn main():
    let a = spawn tang_a()
    let b = spawn tang_b()
    await a
    await b
    print(v0)
    print(v7)
    print(v199)
    print(v100)
"#,
    );
    let a = anh(&src);
    let i = frame_1(&src, &a);
    let v = frame_vm_1(&src, &a);
    assert_eq!(i.stdout, "1\n17\n201\n100\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, i.stdout, "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    so_khop(&i, &v, "dirty-set nhiều var");
}

/// **P2-A khoá** — hai task cộng dồn **cùng** `var` top-level (task nọ thấy ghi của task kia);
/// frame W=1 interp ↔ VM đều `2`.
const HAI_TASK_TANG: &str = r#"var g = 0

async fn tang():
    g = g + 1

fn main():
    let a = spawn tang()
    let b = spawn tang()
    await a
    await b
    print(g)
"#;

#[test]
fn w15_frame_w1_hai_task_cong_don() {
    let a = anh(HAI_TASK_TANG);
    let i = frame_1(HAI_TASK_TANG, &a);
    let v = frame_vm_1(HAI_TASK_TANG, &a);
    assert_eq!(i.stdout, "2\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "2\n", "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    so_khop(&i, &v, "hai task cộng dồn");
}

/// **P2-A khoá** — main ghi `var` **sau khi spawn** rồi task đọc: task thấy giá trị mới (`5`),
/// frame W=1 interp ↔ VM.
const MAIN_GHI_TRUOC_AWAIT: &str = r#"var g = 0

async fn doc() -> int:
    g

fn main():
    let t = spawn doc()
    g = 5
    print(await t)
"#;

#[test]
fn w16_frame_w1_main_ghi_truoc_await() {
    let a = anh(MAIN_GHI_TRUOC_AWAIT);
    let i = frame_1(MAIN_GHI_TRUOC_AWAIT, &a);
    let v = frame_vm_1(MAIN_GHI_TRUOC_AWAIT, &a);
    assert_eq!(i.stdout, "5\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "5\n", "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    so_khop(&i, &v, "main ghi trước await");
}

/// **P2-C (F2)** — frame **VM W=2/4**: task lỗi không ai `await` phải surface `VS0506`, exit 1,
/// output sạch của main giữ nguyên (`sau\n`).
#[test]
fn w17_frame_vm_w2_w4_loi_task_khong_await() {
    let a = anh(LOI_KHONG_AWAIT);
    for w in [2usize, 4] {
        let out = frame_vm_w(LOI_KHONG_AWAIT, &a, w);
        assert_eq!(out.exit, 1, "VM frame W={w}: {:?}", out.diagnostics);
        assert_eq!(out.stdout, "sau\n", "VM frame W={w}: stdout");
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.code.short == "VS0506" && d.message.contains("hong")),
            "VM frame W={w}: {:?}",
            out.diagnostics
        );
    }
}

/// **F3 (P2-1)** — main ghi **nhiều** `var` top-level rồi `await` nhiều task rỗng: sau turn đầu
/// `dirty` phải co lại (prune) nhưng giá trị vẫn đúng; frame W=1 interp ↔ VM.
#[test]
fn w18_frame_w1_main_ghi_nhieu_var_roi_await() {
    let mut src = String::new();
    for i in 0..100 {
        src.push_str(&format!("var v{i} = {i}\n"));
    }
    src.push_str("\nasync fn rong() -> int:\n    1\n\nfn main():\n");
    for i in 0..100 {
        src.push_str(&format!("    v{i} = {i} + 1\n"));
    }
    for _ in 0..50 {
        src.push_str("    await spawn rong()\n");
    }
    src.push_str("    print(v0)\n    print(v99)\n");
    let a = anh(&src);
    let i = frame_1(&src, &a);
    let v = frame_vm_1(&src, &a);
    assert_eq!(i.stdout, "1\n100\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, i.stdout, "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0);
    assert_eq!(v.exit, 0);
    so_khop(&i, &v, "main ghi nhiều var rồi await");
}

// ---------------------------------------------------------------------------
// **ST-319c2 (#206)** — closure bắt biến cấp tệp **theo giá trị** (SPEC §4.7) trên đường
// **frame/async**: lệnh ghi trong thân closure chỉ đổi ô snapshot của closure, **không**
// xuyên ra `globals`. Trước bản vá, `DongBo::ghi` đồng bộ mọi lệnh ghi global kể cả trong
// closure ⇒ frame interp in `5\n5` còn frame VM in `5\n0`.
// ---------------------------------------------------------------------------

/// Closure tạo **trước** task, ghi global **sau** `await`: frame interp ↔ VM đều `5\n0`
/// (closure đọc bản của nó, global ngoài không đổi).
const CLOSURE_GHI_GLOBAL_FRAME: &str = r#"var g = 0

async fn dummy() -> int:
    1

fn main():
    let f = fn() -> int:
        g = 5
        g
    let t = spawn dummy()
    await t
    print(f())
    print(g)
"#;

#[test]
fn w19_frame_w1_closure_ghi_global_khong_xuyen() {
    let a = anh(CLOSURE_GHI_GLOBAL_FRAME);
    let i = frame_1(CLOSURE_GHI_GLOBAL_FRAME, &a);
    let v = frame_vm_1(CLOSURE_GHI_GLOBAL_FRAME, &a);
    assert_eq!(i.stdout, "5\n0\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "5\n0\n", "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "closure ghi global trong frame W=1");
}

/// Closure **đọc lại sau ghi** qua nhiều lần gọi; task `async` ghi cùng global ở giữa:
/// closure thấy snapshot của nó tăng `5 → 10`, còn global ngoài giữ giá trị task ghi (`3`).
const CLOSURE_DOC_SAU_GHI_FRAME: &str = r#"var g = 0

async fn dat() -> int:
    g = 3
    3

fn main():
    let f = fn() -> int:
        g = g + 5
        print(g)
        g
    let t = spawn dat()
    await t
    print(f())
    print(f())
    print(g)
"#;

#[test]
fn w20_frame_w1_closure_doc_sau_ghi_task_ghi_global() {
    let a = anh(CLOSURE_DOC_SAU_GHI_FRAME);
    let i = frame_1(CLOSURE_DOC_SAU_GHI_FRAME, &a);
    let v = frame_vm_1(CLOSURE_DOC_SAU_GHI_FRAME, &a);
    assert_eq!(i.stdout, "5\n5\n10\n10\n3\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, i.stdout, "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "closure đọc sau ghi + task ghi global");
}
