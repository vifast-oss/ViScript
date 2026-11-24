// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-264 — `for` trên range phải duyệt lười, ngữ nghĩa không đổi ở mọi engine**.
//!
//! Trước bản sửa, `for i in 0..2_000_000` vật chất hoá **toàn bộ** range thành `Vec<Value>`
//! (đo được ~146 MB RSS @2M, ~70 B/phần tử) trước khi chạy thân vòng. Nay interp/VM/frame dùng
//! chung `ForIter` (`crates/vis-interp/src/value.rs`): range giữ con trỏ `i64` + cận, không cấp
//! phát N phần tử; list/str/map vẫn vật chất hoá như cũ.
//!
//! Hợp đồng được khoá ở đây:
//! 1. Cận **đóng/mở**, range âm/rỗng/ngược, thứ tự tăng, `break`/`continue` — hai engine
//!    byte-khớp stdout/stderr/exit.
//! 2. Chạm `i64::MAX` **không tràn** (`a..=i64::MAX` dừng đúng sau `MAX`).
//! 3. Trần `10^7` giữ **nguyên thời điểm**: range >10^7 trap `VS0501` **trước** mọi thân vòng
//!    (stdout rỗng, không có `E_LOOP`), đúng như bản vật chất hoá cũ; đúng 10^7 vẫn chạy.
//! 4. `E_LOOP` vẫn charge **mỗi lần thử tiến vòng, gồm lần cuối sai**, khớp interp ↔ VM.
//! 5. Đường frame test cũng duyệt range đúng như hai engine chính.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use vis_core::frame::FrameConfig;
use vis_interp::frame::run_frame_for_test as run_frame_i;
use vis_runtime::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;
use vis_vm::vm_frame::run_frame_for_test as run_frame_v;

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd264-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi_tep(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

fn chay_tep(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy **cả hai** engine trên cùng tệp; đòi exit/stdout/stderr byte-khớp; trả kết quả interp.
fn chay_ca_hai(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, b.0,
        "{tag}: exit interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.1, b.1,
        "{tag}: stdout interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.2, b.2,
        "{tag}: stderr interp↔VM lệch\ninterp: {a:?}\nvm: {b:?}"
    );
    a
}

/// Chương trình hợp lệ: hai engine chạy sạch, stdout đúng kỳ vọng.
fn dung(tag: &str, src: &str, stdout: &str) {
    let (code, out, err) = chay_ca_hai(tag, src);
    assert_eq!(
        code, 0,
        "{tag}: phải chạy sạch\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.is_empty(), "{tag}: stderr phải rỗng: {err}");
    assert_eq!(out, stdout, "{tag}: stdout sai");
}

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

/// Nhãn `KIND@site` — hai crate có bản `FeeEntry` riêng (cùng nguồn `include!`), nên nhận hàm chiếu.
fn nhan<T>(trace: &[T], nhan_mot: impl Fn(&T) -> String) -> Vec<String> {
    trace.iter().map(nhan_mot).collect()
}

/// Chạy interp + VM có bật ngân sách test-hook; đòi hai dãy sự kiện động **byte-khớp**;
/// trả trace interp (nhãn `KIND@site`).
fn trace_interp_vm(src: &str) -> Vec<String> {
    let (hir, syms, tys) = tien_trinh(src);
    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");

    ifee::enable_for_test(1_000_000);
    let out = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let tr_i = ifee::take_trace_for_test();
    ifee::disable_for_test();
    assert!(out.codes().is_empty(), "interp: {:?}", out.diagnostics);

    let mut vm = Vm::new(&m, src);
    vm.budget_enable_for_test(fee, 1_000_000);
    let vo = vm.run(&[]);
    assert!(vo.codes().is_empty(), "VM: {:?}", vo.diagnostics);
    assert_eq!(
        nhan(vm.budget_trace_for_test(), |e| e.label()),
        nhan(&tr_i, |e| e.label()),
        "VM ↔ interp lệch dãy sự kiện cho:\n{src}"
    );
    nhan(&tr_i, |e| e.label())
}

fn so_e_loop(trace: &[String]) -> usize {
    trace.iter().filter(|l| l.starts_with("E_LOOP@")).count()
}

// ===========================================================================
// 1 — Cận, dấu, range rỗng/ngược, `i64::MAX`, range gán biến
// ===========================================================================

#[test]
fn range_bien_va_thu_tu_khop_hai_engine() {
    dung(
        "bien-thu-tu",
        r#"fn main():
    print("excl")
    for i in 0..5:
        print(i)
    print("incl")
    for i in 0..=5:
        print(i)
    print("neg")
    for i in -3..2:
        print(i)
    print("empty")
    for i in 3..3:
        print(i)
    for i in 3..2:
        print(i)
    for i in 2..=1:
        print(i)
    print("max")
    for i in 9223372036854775805..=9223372036854775807:
        print(i)
    print("var")
    let r = 1..4
    for i in r:
        print(i)
"#,
        "excl\n0\n1\n2\n3\n4\nincl\n0\n1\n2\n3\n4\n5\nneg\n-3\n-2\n-1\n0\n1\n\
         empty\nmax\n9223372036854775805\n9223372036854775806\n\
         9223372036854775807\nvar\n1\n2\n3\n",
    );
}

// ===========================================================================
// 2 — `break`/`continue` trên range giữ nguyên thứ tự + giá trị tích luỹ
// ===========================================================================

#[test]
fn range_break_continue_khop_hai_engine() {
    dung(
        "break-continue",
        r#"fn main():
    var s = 0
    for i in 0..10:
        if i == 2:
            continue
        if i == 5:
            break
        s = s + i
    print(s)
    var dem = 0
    for i in 5..0:
        dem = dem + 1
    print(dem)
"#,
        "8\n0\n",
    );
}

// ===========================================================================
// 3 — Trần `10^7`: đúng 10^7 chạy; >10^7 trap **trước** thân vòng
// ===========================================================================

/// Đúng 10^7 phần tử (biên) vẫn hợp lệ; `break` ngay nên kiểm nhanh, không phụ thuộc tốc độ.
#[test]
fn range_dung_10m_van_chay_va_break_som() {
    dung(
        "dung-10m",
        r#"fn main():
    var n = 0
    for i in 0..10_000_000:
        n = n + 1
        break
    print(n)
"#,
        "1\n",
    );
}

/// >10^7 phần tử: trap `VS0501` **trước** mọi thân vòng (stdout rỗng), giống bản vật chất hoá cũ.
#[test]
fn range_qua_tran_trap_truoc_than_vong() {
    for (tag, cận) in [("excl", "0..10_000_001"), ("incl", "0..=10_000_000")] {
        let src = format!("fn main():\n    for i in {cận}:\n        print(i)\n");
        let (code, out, err) = chay_ca_hai(&format!("qua-tran-{tag}"), &src);
        assert_eq!(code, 101, "{tag}: lỗi chạy có mã phải exit 101: {err}");
        assert!(
            out.is_empty(),
            "{tag}: trap phải xảy ra trước thân vòng, stdout: {out:?}"
        );
        assert!(
            err.contains("VS0501") && err.contains("khoảng quá dài (>10^7 phần tử)"),
            "{tag}: thiếu VS0501/trần range: {err}"
        );
    }
}

// ===========================================================================
// 4 — `E_LOOP` giữ đúng số lần thử (kể cả lần cuối sai) và khớp VM
// ===========================================================================

#[test]
fn range_e_loop_khop_vm_gom_lan_cuoi_sai() {
    // 2 phần tử ⇒ 3 lần thử (2 có giá trị + 1 thất bại).
    let tr = trace_interp_vm("fn main():\n    for x in 0..2:\n        print(x)\n");
    assert_eq!(so_e_loop(&tr), 3, "range 2 phần tử phải 3 E_LOOP: {tr:?}");

    // Range rỗng ⇒ đúng 1 lần thử thất bại.
    let tr = trace_interp_vm("fn main():\n    for x in 0..0:\n        print(x)\n");
    assert_eq!(so_e_loop(&tr), 1, "range rỗng phải 1 E_LOOP: {tr:?}");

    // `break` sau phần tử đầu ⇒ không cộng lượt kế.
    let tr = trace_interp_vm("fn main():\n    for x in 0..5:\n        print(x)\n        break\n");
    assert_eq!(so_e_loop(&tr), 1, "break không được cộng E_LOOP: {tr:?}");

    // `continue` vẫn tới lượt kế; 3 phần tử ⇒ 4 lần thử.
    let tr = trace_interp_vm("fn main():\n    for x in 0..3:\n        continue\n");
    assert_eq!(
        so_e_loop(&tr),
        4,
        "continue phải chạy hết range + lần cuối: {tr:?}"
    );
}

// ===========================================================================
// 5 — Đường frame test duyệt range giống hai engine chính
// ===========================================================================

#[test]
fn range_duong_frame_khop_interp_va_vm() {
    let src = r#"fn main():
    var s = 0
    for i in -2..=2:
        if i == 0:
            continue
        s = s + i
    print(s)
    for i in 4..1:
        print(i)
    var n = 0
    for i in 0..100:
        n = n + 1
        if n == 2:
            break
    print(n)
"#;
    let (hir, syms, tys) = tien_trinh(src);
    let fi = run_frame_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let fv = run_frame_v(&m, src, &[], FrameConfig::default());
    let ky_vong = "0\n2\n";
    assert_eq!(fi.stdout, ky_vong, "frame interp sai: {:?}", fi.diagnostics);
    assert_eq!(fv.stdout, ky_vong, "frame VM sai: {:?}", fv.diagnostics);
    assert_eq!(fi.exit, fv.exit, "frame exit lệch");
}
