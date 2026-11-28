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
// **ST-319d** — đóng `#208`: codegen thiếu tập **mutable** nên `push` trên `var`
// không ghi lại ô ở vị trí biểu thức, còn `push` trên `let` ở câu lệnh lại ghi bừa.
// Oracle = interp (`VD-51`: `var` ghi ô, `let`/tạm trả list mới).
// ---------------------------------------------------------------------------

/// (a)+(b) `#208`: push là **biểu thức cuối** của `async fn` ⇒ có `Assign` ô global.
/// W=1 cả 2 engine + **frame** (đường production `vis run --workers 1`).
const PUSH_VAR_DUOI: &str = r#"var xs = [1]

async fn them():
    xs.push(2)

fn main():
    let t = spawn them()
    await t
    print(xs)
"#;

#[test]
fn w19_push_var_bieu_thuc_cuoi() {
    let a = anh(PUSH_VAR_DUOI);
    let i = interp(PUSH_VAR_DUOI, &a, 1);
    let v = vm(PUSH_VAR_DUOI, &a, 1);
    assert_eq!(i.stdout, "[1, 2]\n", "interp W=1: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "[1, 2]\n", "VM W=1: {:?}", v.diagnostics);
    so_khop(&i, &v, "push var W=1");
    let fi = frame_1(PUSH_VAR_DUOI, &a);
    let fv = frame_vm_1(PUSH_VAR_DUOI, &a);
    assert_eq!(fi.stdout, "[1, 2]\n", "frame interp: {:?}", fi.diagnostics);
    assert_eq!(fv.stdout, "[1, 2]\n", "frame VM: {:?}", fv.diagnostics);
    so_khop(&fi, &fv, "push var frame W=1");
}

/// (c) hàm **sync** — loại trừ task/frame khỏi phương trình.
const PUSH_VAR_SYNC: &str = r#"var xs = [1]
fn them(): xs.push(2)
fn main():
    them()
    print(xs)
"#;
#[test]
fn w20_push_var_ham_sync() {
    let a = anh(PUSH_VAR_SYNC);
    let i = interp(PUSH_VAR_SYNC, &a, 1);
    let v = vm(PUSH_VAR_SYNC, &a, 1);
    assert_eq!(i.stdout, "[1, 2]\n");
    assert_eq!(v.stdout, "[1, 2]\n", "VM: {:?}", v.diagnostics);
    so_khop(&i, &v, "push var sync");
}

/// (h) `let` + push **câu lệnh** ⇒ **không** ghi lại (trước đây VM ghi ⇒ `[1, 2]`).
const PUSH_LET_CAU_LENH: &str = r#"fn main():
    let xs = [1]
    xs.push(2)
    print(xs)
"#;
#[test]
fn w21_push_let_khong_ghi_lai() {
    let a = anh(PUSH_LET_CAU_LENH);
    let i = interp(PUSH_LET_CAU_LENH, &a, 1);
    let v = vm(PUSH_LET_CAU_LENH, &a, 1);
    assert_eq!(i.stdout, "[1]\n");
    assert_eq!(v.stdout, "[1]\n", "VM: {:?}", v.diagnostics);
    so_khop(&i, &v, "push let");
}

/// (e) `let y = xs.push(2)` trên `var xs` ⇒ **cả hai** `[1, 2]`.
const PUSH_VAR_GAN: &str = r#"fn main():
    var xs = [1]
    let y = xs.push(2)
    print(xs)
    print(y)
"#;
#[test]
fn w22_push_var_trong_let() {
    let a = anh(PUSH_VAR_GAN);
    let i = interp(PUSH_VAR_GAN, &a, 1);
    let v = vm(PUSH_VAR_GAN, &a, 1);
    assert_eq!(i.stdout, "[1, 2]\n[1, 2]\n");
    assert_eq!(v.stdout, "[1, 2]\n[1, 2]\n", "VM: {:?}", v.diagnostics);
    so_khop(&i, &v, "push var trong let");
}

/// (W=2) push trên `var` top-level trong task ⇒ `VS0509` **cả hai** engine (W=1 vẫn `[1, 2]`).
#[test]
fn w23_push_var_w2_ghi_cheo_vs0509() {
    let a = anh(PUSH_VAR_DUOI);
    for out in [interp(PUSH_VAR_DUOI, &a, 1), vm(PUSH_VAR_DUOI, &a, 1)] {
        assert_eq!(out.exit, 0, "W=1: {:?}", out.diagnostics);
        assert_eq!(out.stdout, "[1, 2]\n");
    }
    for w in [2usize, 4] {
        for (nhan, out) in [
            ("interp", interp(PUSH_VAR_DUOI, &a, w)),
            ("vms", vm(PUSH_VAR_DUOI, &a, w)),
        ] {
            assert_eq!(out.exit, 1, "{nhan} W={w} phải lỗi");
            assert!(
                out.diagnostics
                    .iter()
                    .any(|d| d.code.short == "VS0509" && d.message.contains("ghi chéo worker")),
                "{nhan} W={w}: {:?}",
                out.diagnostics
            );
        }
    }
}

/// (j) `map.insert` (chưa hỗ trợ) — parity 2 engine, chống patch ghi bừa cho `insert`.
const PUSH_MAP_INSERT: &str = r#"var m = {"a": 1}
async fn them(): m.insert("b", 2)
fn main():
    let t = spawn them()
    await t
    print(m)
"#;
#[test]
fn w24_map_insert_parity() {
    let a = anh(PUSH_MAP_INSERT);
    let i = interp(PUSH_MAP_INSERT, &a, 1);
    let v = vm(PUSH_MAP_INSERT, &a, 1);
    assert_eq!(i.exit, 1, "interp: {:?}", i.diagnostics);
    assert_eq!(v.exit, 1, "VM: {:?}", v.diagnostics);
    assert!(
        v.diagnostics
            .iter()
            .any(|d| d.code.short == "VS0506" && d.message.contains("`map.insert()` chưa hỗ trợ")),
        "{:?}",
        v.diagnostics
    );
    so_khop(&i, &v, "map.insert parity");
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
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
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
    assert_eq!(v.stdout, "5\n5\n10\n10\n3\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "closure đọc sau ghi + task ghi global");
}

/// **w21** — closure **treo giữa chừng** (`await dat()` trong thân): khi resume, `nap_vao_env`
/// chỉ đồng bộ khung ngoài closure ⇒ snapshot `g=0` không bị ghi `g=3` của task đè lên; closure
/// cộng `5` trên bản của nó, global ngoài giữ `3`.
#[test]
fn w21_frame_w1_closure_await_ghi_global_sau_resume() {
    const SRC: &str = r#"var g = 0

async fn dat() -> int:
    g = 3
    3

fn main():
    let f = fn() -> int:
        await dat()
        g = g + 5
        g
    print(f())
    print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "5\n3\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "5\n3\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "closure await rồi ghi global");
}

/// **w22** — closure **bên trong task**: ghi global chỉ đổi snapshot của closure (in `5`),
/// `g` trong task vẫn `0`, kết quả task `1`; main `await` + in kết quả.
#[test]
fn w22_frame_w1_closure_ghi_global_trong_task() {
    const SRC: &str = r#"var g = 0

async fn chay() -> int:
    let f = fn() -> int:
        g = 5
        g
    print(f())
    print(g)
    1

fn main():
    let t = spawn chay()
    print(await t)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "5\n0\n1\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "5\n0\n1\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "closure ghi global trong task");
}

/// **w23** — khoá heuristic `c.name.is_empty()`: **hàm tên** truyền qua giá trị (`let h = tang`)
/// vẫn là env chia sẻ ⇒ `h()` ghi global **xuyên** như F2/F3 (`1`), không bị nhầm thành closure.
#[test]
fn w23_frame_w1_ham_ten_qua_gia_tri_van_xuyen_global() {
    const SRC: &str = r#"var g = 0

async fn dummy() -> int:
    1

fn tang():
    g = g + 1

fn main():
    let h = tang
    let t = spawn dummy()
    await t
    h()
    print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "1\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "1\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "hàm tên qua giá trị vẫn xuyên global");
}

/// **w24 (P2-1 audit)** — **khung hàm tên nằm TRÊN closure** trong stack (`doc()` gọi từ closure
/// và tự `await dat()`): `nap_vao_env` chỉ được bỏ khung `by_value`, không được cắt cả khung
/// hàm tên (env dựng từ `globals`). Trước bản vá, hồi quy cho `0\n3` (interp) trong khi VM và
/// non-frame là `3\n3`.
#[test]
fn w24_frame_w1_ham_ten_tren_closure_nap_van_sync() {
    const SRC: &str = r#"var g = 0

async fn dat() -> int:
    g = 3
    3

fn doc() -> int:
    await dat()
    g

fn main():
    let f = fn() -> int:
        doc()
    print(f())
    print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "3\n3\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "3\n3\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "hàm tên trên closure vẫn sync global");
}

// ---------------------------------------------------------------------------
// **ST-320a (#211, đề xuất Sóng 9)** — `var self` + `push` trên **trường** (`self.xs.push(2)`).
//
// SPEC §4.7: chỉ `var self` được gán; §4.8: gán lồng chỉ trên gốc `var`. VD-51: `push` trên
// **ô biến** `var` phải ghi lại lvalue (CoW), còn `let`/biểu thức tạm trả list mới (không no-op
// im lặng). Receiver `self.xs` là **Field** (biểu thức tạm), không phải `HExpr::Var` trực tiếp,
// nên **không** ghi lại — interp (oracle) và VM đều vậy từ trước `ST-319d`; `mutables` chỉ chứa
// `sym` của `var`/`var self` khi receiver là `Var` trực tiếp.
//
// Ca này khoá **parity** 4 đường default + W=1 non-frame, cộng frame W=1 (đường `sym_ghi_cua`/
// `mutables` của `var self`), nên **không cần sửa codegen** cho #211. Nếu A0 đổi ngữ nghĩa
// (push trường cũng rebind như ô `var`), đây là test đỏ có chủ đích.
// ---------------------------------------------------------------------------

/// Chạy **6 đường**: interp/VM mặc định + W=1 non-frame + frame W=1; assert stdout/exit/
/// diagnostics rỗng từng đường và parity từng cặp với interp mặc định.
fn sau_duong_st320a(src: &str, ky_vong: &str, nhan: &str) -> RunOutput {
    let a = anh(src);
    let di = vis_interp::run(src, &a.0, &a.1, &a.2, &[]);
    let dv = vis_vm::run(&a.3, src, &[]);
    let wi = interp(src, &a, 1);
    let wv = vm(src, &a, 1);
    let fi = frame_1(src, &a);
    let fv = frame_vm_1(src, &a);
    for (ten, out) in [
        ("interp", &di),
        ("VM", &dv),
        ("interp W1", &wi),
        ("VM W1", &wv),
        ("frame interp W1", &fi),
        ("frame VM W1", &fv),
    ] {
        so_khop(&di, out, &format!("{nhan}/{ten}"));
        assert_eq!(out.stdout, ky_vong, "{nhan}/{ten}: stdout");
        assert_eq!(out.exit, 0, "{nhan}/{ten}: exit {:?}", out.diagnostics);
        assert!(
            out.diagnostics.is_empty(),
            "{nhan}/{ten}: {:?}",
            out.diagnostics
        );
    }
    di
}

/// `self.xs.push(2)` ở **biểu thức cuối** và trong `let` ⇒ trả list mới nhưng **không** ghi lại
/// `self.xs`; người gọi (giá trị) không thấy đổi. Đọc lại `self.xs` ngay trong hàm vẫn `[1]`.
const VAR_SELF_PUSH_FIELD: &str = r#"type C:
    xs: list[int]

impl C:
    fn cuoi(var self) -> list[int]:
        self.xs.push(2)

    fn trong_let(var self) -> list[int]:
        let kq = self.xs.push(2)
        print(self.xs)
        kq

async fn dummy() -> int:
    1

fn main():
    var c: C = {xs: [1]}
    let t = spawn dummy()
    await t
    print(c.cuoi())
    print(c.xs)
    print(c.trong_let())
    print(c.xs)
"#;

#[test]
fn st320a_var_self_push_truong_parity() {
    sau_duong_st320a(
        VAR_SELF_PUSH_FIELD,
        "[1, 2]\n[1]\n[1]\n[1, 2]\n[1]\n",
        "var self push trường",
    );
}

/// Đối chứng: `self.xs = self.xs.push(2)` (gán tường minh trên gốc `var self`, §4.8) **có** đổi
/// `self.xs` **bên trong** hàm (`[1, 2]` cả hai lần đọc), nhưng người gọi vẫn `[1]` (giá trị).
const VAR_SELF_GAN_TRUONG: &str = r#"type C:
    xs: list[int]

impl C:
    fn gan(var self) -> list[int]:
        self.xs = self.xs.push(2)
        print(self.xs)
        self.xs

async fn dummy() -> int:
    1

fn main():
    var c: C = {xs: [1]}
    let t = spawn dummy()
    await t
    print(c.gan())
    print(c.xs)
"#;

#[test]
fn st320a_var_self_gan_truong_cuc_bo() {
    sau_duong_st320a(
        VAR_SELF_GAN_TRUONG,
        "[1, 2]\n[1, 2]\n[1]\n",
        "var self gán trường",
    );
}

/// Đường **`mutables` + `is_var_self` trực tiếp**: `self.push(2)` có base `HExpr::Var` trỏ
/// tham số `var self` (codegen `expr.rs` nhánh `(Var, "push")`). Đây là dạng **duy nhất** chạm
/// nhánh đó: method `impl` có `self` kiểu record nên `self.push` không hợp lệ, còn tham số
/// `var self` tự do **không nhận chú thích kiểu** (VS1b — `{error}`) và được phép gọi `push`.
/// Kỳ vọng: `self` **ghi ô** trong thân (`[1, 2]`, `len = 2`), người gọi vẫn `[1]` (giá trị).
const VAR_SELF_LA_LIST: &str = r#"fn keo(var self) -> int:
    let y = self.push(2)
    print(self)
    print(y)
    self.len()

async fn dummy() -> int:
    1

fn main():
    var xs = [1]
    let t = spawn dummy()
    await t
    print(keo(xs))
    print(xs)
"#;

#[test]
fn st320a_var_self_la_list_push_ghi_o() {
    sau_duong_st320a(
        VAR_SELF_LA_LIST,
        "[1, 2]\n[1, 2]\n2\n[1]\n",
        "var self list push",
    );
}

/// **w25 (ST-320b / #212, P2-2 ST-319c2)** — **hàm tên qua giá trị tạo TRONG closure**:
/// `let h = tang`/`let h = doc` phải chạy trên **env module** (như `call_hfn` non-frame
/// ST-319e), không thừa hưởng snapshot closure. Trước bản vá, frame chụp `frame.cur().env` rồi
/// `merge` chép ngược vào snapshot:
/// - write-only: frame `1,1,1` vs VM `0,0,1` — closure thấy global vừa ghi qua `h()`;
/// - read-only + `await`: frame `900,200,300,400` vs VM `903,203,303,403` — `h()` đọc snapshot
///   `g=0` thay vì global `g=3`.
#[test]
fn w25_frame_w1_ham_ten_qua_gia_tri_trong_closure() {
    // Write-only: `tang` ghi `g`, closure trả `g` (snapshot) — global chỉ đổi ngoài closure.
    const GHI: &str = r#"var g = 0

async fn dummy() -> int:
    1

fn tang():
    g = 1

fn main():
    let f = fn() -> int:
        let h = tang
        h()
        g
    let t = spawn dummy()
    await t
    print(f())
    print(f())
    tang()
    print(g)
"#;
    let a = anh(GHI);
    let i = frame_1(GHI, &a);
    let v = frame_vm_1(GHI, &a);
    assert_eq!(i.stdout, "0\n0\n1\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "0\n0\n1\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "write-only: hàm tên qua giá trị trong closure");

    // Read-only + await: closure tạo trước task ghi `g=3`; `h = doc` (chỉ đọc) gọi sau `await`
    // phải thấy global `3` (env module), không phải snapshot `0` của closure.
    const DOC: &str = r#"var g = 0

async fn dat() -> int:
    g = 3
    3

fn doc() -> int:
    g

fn main():
    let f = fn():
        let h = doc
        let t = spawn dat()
        await t
        print(900 + h())
        print(200 + h())
        print(300 + h())
        print(400 + h())
    f()
"#;
    let a = anh(DOC);
    let i = frame_1(DOC, &a);
    let v = frame_vm_1(DOC, &a);
    assert_eq!(
        i.stdout, "903\n203\n303\n403\n",
        "interp: {:?}",
        i.diagnostics
    );
    assert_eq!(v.stdout, "903\n203\n303\n403\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(
        &i,
        &v,
        "read-only + await: hàm tên qua giá trị trong closure",
    );
}

/// **w26 (ST-320b F1 / #212, P2-1 audit)** — **hàm tên trong `const`** + free HOF:
/// vòng `const` chạy trước các điểm sync `env_module` cũ, nên `const H: fn(int) -> bool = lay`
/// từng chụp map rỗng của `Interp::new`; `where([1], H)` gọi callback qua `c.env` ⇒ frame trap
/// `VS0501` trong khi VM in `[1]`. F1 sync `it.env`/`env_module` vào `globals` **trước** const.
#[test]
fn w26_frame_w1_const_ham_ten_qua_hof() {
    const SRC: &str = r#"var g = 1

async fn dummy() -> int:
    1

fn lay(x: int) -> bool:
    g > 0

const H: fn(int) -> bool = lay

fn main():
    let t = spawn dummy()
    await t
    print(where([1], H))
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "[1]\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "[1]\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "const hàm tên qua HOF trong frame");
}

/// **w27 (ST-320b / P1-320b-1 — audit sâu, ĐỎ trước vá)**: HOF `where` gọi callback hàm tên
/// ghi biến cấp tệp. Callback chạy non-frame (`Host::call` → `call_closure`) ghi thẳng globals
/// nhưng **không đi qua `DongBo::ghi`** ⇒ frame peer/cache giữ ô cũ, in `g = 0`; main frame
/// và VM in `g = 1`. Kỳ vọng đúng (SPEC §4.7, hàm tên đọc/ghi global module): `[1]\n1\n`
/// cho **cả hai** engine. Test này đỏ trên tip `8b164d48` — giữ làm bằng chứng HOLD merge.
#[test]
fn w27_frame_w1_hof_callback_ghi_global() {
    const SRC: &str = r#"var g = 0

async fn dummy() -> int:
    1

fn tang(x: int) -> bool:
    g = g + 1
    true

fn main():
    let t = spawn dummy()
    await t
    print(where([1], tang))
    print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    // VM/main frame là oracle: `[1]` + g=1. Frame interp hiện `[1]` + g=0 ⇒ ĐỎ.
    assert_eq!(i.stdout, "[1]\n1\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "[1]\n1\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "HOF callback hàm tên ghi global trong frame");
}

/// **w28 (ST-323c / #213, P2-2)** — **free HOF gọi callback hàm tên trong closure lambda ghi
/// global**: `let h = tang` trong lambda `f()`; `where([1, 3], h)` chạy callback **non-frame**
/// (`Host::call` → `call_closure`, env module) ghi thẳng globals mà **không** đi qua
/// `DongBo::ghi` ⇒ cache của **main frame** (clone của globals) giữ ô cũ. Oracle là VM/main
/// frame: `[3]\n3\n0\n3\n` (`print(g)` trong lambda vẫn `0` theo by-value closure #206/#209).
/// Repro gốc: `/home/ps/vislogs/review-manual-hof-frame2.vis` — interp `[3],3,0,0` vs VM
/// `[3],3,0,3`. Đỏ trước vá ST-323c.
#[test]
fn w28_frame_w1_free_hof_ghi_global_trong_closure() {
    const SRC: &str = r#"var g = 0

async fn dummy() -> int:
    1

fn doc() -> int:
    g

fn tang(x: int) -> bool:
    g = x
    x > 2

fn main():
    let t = spawn dummy()
    await t
    let f = fn():
        let h = tang
        print(where([1, 3], h))
        print(doc())
        print(g)
    f()
    print(g)
"#;
    let a = anh(SRC);
    let i = frame_1(SRC, &a);
    let v = frame_vm_1(SRC, &a);
    assert_eq!(i.stdout, "[3]\n3\n0\n3\n", "interp: {:?}", i.diagnostics);
    assert_eq!(v.stdout, "[3]\n3\n0\n3\n", "VM: {:?}", v.diagnostics);
    assert!(i.diagnostics.is_empty(), "interp: {:?}", i.diagnostics);
    assert!(v.diagnostics.is_empty(), "VM: {:?}", v.diagnostics);
    assert_eq!(i.exit, 0, "interp exit");
    assert_eq!(v.exit, 0, "VM exit");
    so_khop(&i, &v, "free HOF ghi global qua callback trong closure");
}

/// **w29 (ST-323c / #215, test gap W>1)** — task **chỉ đọc** global qua hàm tên lấy từ giá trị
/// (`let h = doc`) và free HOF read-only (`where([1,2], than)`) trong closure; chạy W=2/4 để khoá
/// parity hai engine trên đường W>1 (audit Sóng 9 chỉ kiểm tay: read-only không trap `VS0509`).
/// Kỳ vọng literal `[1, 2]\n107\n7\n` cho **mỗi** engine, mỗi W.
#[test]
fn w29_frame_w2_w4_doc_only_hof_trong_closure() {
    const SRC: &str = r#"var g = 7

async fn doc_task() -> int:
    let h = doc
    print(where([1, 2], than))
    100 + h()

fn doc() -> int:
    g

fn than(x: int) -> bool:
    g > x

fn main():
    let t = spawn doc_task()
    print(await t)
    print(g)
"#;
    let a = anh(SRC);
    for w in [2, 4] {
        let i = frame_w(SRC, &a, w);
        let v = frame_vm_w(SRC, &a, w);
        assert_eq!(
            i.stdout, "[1, 2]\n107\n7\n",
            "interp W={w}: {:?}",
            i.diagnostics
        );
        assert_eq!(
            v.stdout, "[1, 2]\n107\n7\n",
            "VM W={w}: {:?}",
            v.diagnostics
        );
        assert!(
            i.diagnostics.is_empty(),
            "interp W={w}: {:?}",
            i.diagnostics
        );
        assert!(v.diagnostics.is_empty(), "VM W={w}: {:?}", v.diagnostics);
        assert_eq!(i.exit, 0, "interp W={w} exit");
        assert_eq!(v.exit, 0, "VM W={w} exit");
        so_khop(&i, &v, "read-only hàm tên/HOF trong closure W>1");
    }
}
