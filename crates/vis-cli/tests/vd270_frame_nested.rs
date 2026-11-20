// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 — gán lồng trên đường frame** (`a.b.f = v`, `xs[0].f = v`, `m["k"].f = v`,
//! `a.bs[0].f = v`, `p.xs[1] = v`, `xs[1][0] = v`, `var self`).
//!
//! VD-268 đã cho **hai runtime chính** (interp + VM) gán **đường** có gốc là biến, nhưng
//! **frame evaluator** (`crates/vis-interp/src/frame/`) vẫn chỉ nhận đích mà **gốc trực tiếp**
//! là `HExpr::Var`; mọi đích sâu hơn trap
//! `"frame test: gán trường/phần tử chỉ hỗ trợ gốc là \`var\`"`.
//!
//! Bộ test này chạy **frame interp ↔ frame VM** trên cùng HIR/bytecode (đòi stdout, exit,
//! chẩn đoán, log, snapshot khớp) và **interp thường ↔ `--vm`** để chứng minh ngữ nghĩa giá
//! trị/evaluation order không đổi. Phần cuối khoá:
//!
//! - gốc `let`/tham số ⇒ trap `VS0412` phòng thủ ở frame interp (tyck đã chặn tĩnh trước);
//! - `+=` lồng vẫn **chưa** hỗ trợ ở frame interp (giữ đúng phạm vi VD-270: chỉ `=`), trong
//!   khi interp/VM thường và frame VM chạy được — **nợ đã ghi rõ**, không giả vờ khớp;
//! - chỉ số ngoài phạm vi giữa đường trap **trước** vế phải (`VS0503`), chỉ số ngoài phạm vi
//!   ở **lá** trap **sau** vế phải (`VS0501`) — y hệt interp/VM.
//!
//! **Đỏ-trước (bằng chứng)**: trước bản sửa, mọi ca `.ok` đều đỏ ở frame interp với
//! `"gán trường/phần tử chỉ hỗ trợ gốc là \`var\`"`.

use std::path::{Path, PathBuf};
use std::process::Command;

use vis_interp::frame::{run_frame_for_test as run_frame_i, FrameConfig, FrameOutput};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm::Vm;
use vis_vm::vm_frame::run_frame_for_test as run_frame_v;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower(&p.cst, &l);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn ma(ds: &[vis_syntax::Diagnostic]) -> Vec<&'static str> {
    ds.iter().map(|d| d.code.short).collect()
}

/// Chạy **frame interp** + **frame VM** trên cùng HIR; đòi khớp từng byte và trả
/// `(interp, vm)`.
fn frame_hai_duong(src: &str) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_frame_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_frame_v(&m, src, &[], FrameConfig::default());
    assert_eq!(
        i.stdout, v.stdout,
        "frame interp ≠ frame VM stdout\n interp: {:?}\n vm:     {:?}\n diag i: {:?}\n diag v: {:?}",
        i.stdout, v.stdout, i.diagnostics, v.diagnostics
    );
    assert_eq!(i.exit, v.exit, "frame exit lệch");
    assert_eq!(i.log, v.log, "frame log lệch");
    assert_eq!(i.snapshot, v.snapshot, "frame snapshot lệch");
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch: i={:?} v={:?}",
        i.diagnostics,
        v.diagnostics
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "thông điệp chẩn đoán lệch");
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí chẩn đoán lệch");
    }
    (i, v)
}

/// Chạy **interp thường** + **VM thường** trên cùng nguồn; đòi stdout/exit khớp.
fn runtime_hai_duong(src: &str) -> (String, i32) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let mut vm = Vm::new(&m, src);
    let v = vm.run(&[]);
    assert_eq!(
        i.stdout, v.stdout,
        "interp ≠ --vm stdout\n interp: {:?}\n vm:     {:?}\n diag i: {:?}\n diag v: {:?}",
        i.stdout, v.stdout, i.diagnostics, v.diagnostics
    );
    assert_eq!(i.exit, v.exit, "interp/VM exit lệch");
    assert_eq!(ma(&i.diagnostics), ma(&v.diagnostics), "mã chẩn đoán lệch");
    (i.stdout, i.exit)
}

/// Ca chạy được: frame hai đường + runtime hai đường + stdout kỳ vọng.
fn ca_ok(src: &str, ky_vong: &str) {
    let (fi, fv) = frame_hai_duong(src);
    assert_eq!(fi.exit, 0, "frame interp: {:?}", fi.diagnostics);
    assert_eq!(fi.stdout, ky_vong, "frame interp stdout");
    assert_eq!(fv.stdout, ky_vong, "frame VM stdout");
    let (ro, re) = runtime_hai_duong(src);
    assert_eq!(ro, ky_vong, "interp/VM stdout");
    assert_eq!(re, 0, "interp/VM exit");
}

const P: &str = "type P:\n    x: int\n\ntype Q:\n    p: P\n\n";

// ===========================================================================
// 1 — Đích đường chạy được: trường / chỉ số / hỗn hợp
// ===========================================================================

#[test]
fn ban_ghi_trong_ban_ghi() {
    let src =
        format!("{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    print(q.p.x)\n");
    ca_ok(&src, "2\n");
}

#[test]
fn phan_tu_list_la_ban_ghi() {
    let src = format!(
        "{P}fn main():\n    var xs: list[P] = [{{x: 1}}, {{x: 2}}]\n    xs[1].x = 9\n    print(xs[0].x)\n    print(xs[1].x)\n"
    );
    ca_ok(&src, "1\n9\n");
}

#[test]
fn gia_tri_map_la_ban_ghi() {
    let src = format!(
        "{P}fn main():\n    var m: map[str, P] = {{\"a\": {{x: 1}}, \"b\": {{x: 2}}}}\n    m[\"b\"].x = 9\n    print(m[\"a\"].x)\n    print(m[\"b\"].x)\n"
    );
    ca_ok(&src, "1\n9\n");
}

#[test]
fn truong_ban_ghi_la_list_va_list_long() {
    let src = "type P:\n    xs: list[int]\n\ntype I:\n    n: int\n\ntype O:\n    bs: list[I]\n\nfn main():\n    var p: P = {xs: [1, 2, 3]}\n    p.xs[1] = 9\n    print(p.xs)\n    var a: O = {bs: [{n: 1}, {n: 2}]}\n    a.bs[0].n = 7\n    print(a.bs[0].n)\n    print(a.bs[1].n)\n";
    ca_ok(src, "[1, 9, 3]\n7\n2\n");
}

#[test]
fn chi_so_long_hai_tang() {
    let src = "fn main():\n    var xs: list[list[int]] = [[1, 2], [3, 4]]\n    xs[1][0] = 9\n    print(xs)\n";
    ca_ok(src, "[[1, 2], [9, 4]]\n");
}

#[test]
fn var_self_gan_long() {
    // `var self` là gốc hợp lệ; người gọi không thấy đổi (giá trị).
    let src = format!(
        "{P}type R:\n    q: Q\n\nimpl R:\n    fn bump(var self) -> int:\n        self.q.p.x = 9\n        return self.q.p.x\n\nfn main():\n    let r: R = {{q: {{p: {{x: 1}}}}}}\n    print(r.bump())\n    print(r.q.p.x)\n"
    );
    ca_ok(&src, "9\n1\n");
}

#[test]
fn mot_tang_optional_cung_di_cung_duong() {
    // Dạng một tầng cũ nay dùng **cùng** đường: nâng `int?`/`list[int?]` như interp/VM.
    let src = "type P:\n    x: int?\n\nfn main():\n    var p: P = {x: Some(1)}\n    p.x = 7\n    print(p.x)\n    var xs: list[int?] = [Some(2)]\n    xs[0] = 8\n    print(xs)\n";
    ca_ok(src, "Some(7)\n[Some(8)]\n");
}

#[test]
fn dich_truong_optional_duoc_nang() {
    // Đích `int?` ở lá: `=` nâng `7` thành `Some(7)` như interp/codegen (`lift_assign_set`).
    let src = "type P:\n    x: int?\n\ntype Q:\n    p: P\n\nfn main():\n    var q: Q = {p: {x: Some(1)}}\n    q.p.x = 7\n    print(q.p.x)\n";
    ca_ok(src, "Some(7)\n");
}

#[test]
fn doc_lap_voi_ban_sao() {
    // Giá trị: `let b = q.p` là bản sao; `q.p.x = 2` không đổi `b.x`.
    let src = format!(
        "{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    let b: P = q.p\n    q.p.x = 2\n    print(b.x)\n    print(q.p.x)\n"
    );
    ca_ok(&src, "1\n2\n");
}

// ===========================================================================
// 2 — Evaluation order: chỉ số theo đường trước, rồi vế phải
// ===========================================================================

#[test]
fn chi_so_duong_truoc_ve_phai() {
    let src = "type P:\n    xs: list[int]\n\nfn idx(tag: str, i: int) -> int:\n    print(tag)\n    i\n\nfn rhs() -> int:\n    print(\"rhs\")\n    9\n\nfn main():\n    var p: P = {xs: [1, 2, 3]}\n    p.xs[idx(\"i\", 1)] = rhs()\n    print(p.xs)\n";
    ca_ok(src, "i\nrhs\n[1, 9, 3]\n");
}

#[test]
fn await_trong_chi_so_giu_duong_qua_suspend() {
    // Chỉ số là `await`: frame phải giữ gốc + các bước đã gom qua chỗ treo rồi resume.
    let src = "async fn ext() -> int:\n    1\n\nfn main():\n    var xs: list[list[int]] = [[1, 2], [3, 4]]\n    xs[await ext()][1] = 9\n    print(xs)\n";
    ca_ok(src, "[[1, 2], [3, 9]]\n");
}

#[test]
fn hai_chi_so_theo_thu_tu_roi_ve_phai() {
    let src = "fn idx(tag: str, i: int) -> int:\n    print(tag)\n    i\n\nfn rhs() -> int:\n    print(\"rhs\")\n    9\n\nfn main():\n    var xs: list[list[int]] = [[1, 2], [3, 4]]\n    xs[idx(\"row\", 1)][idx(\"col\", 0)] = rhs()\n    print(xs)\n";
    ca_ok(src, "row\ncol\nrhs\n[[1, 2], [9, 4]]\n");
}

// ===========================================================================
// 3 — Chỉ số ngoài phạm vi: giữa đường trước RHS, ở lá sau RHS
// ===========================================================================

#[test]
fn chi_so_ngoai_pham_vi_giua_duong_trap_truoc_ve_phai() {
    // Đọc tầng giữa ngoài phạm vi ⇒ `VS0503` **trước** vế phải / ghi.
    let src = format!(
        "{P}fn hong() -> int:\n    print(\"rhs\")\n    1\n\nfn main():\n    var xs: list[P] = [{{x: 1}}]\n    xs[3].x = hong()\n    print(\"khong\")\n"
    );
    let (i, v) = frame_hai_duong(&src);
    assert_eq!(i.exit, 1, "frame interp phải trap");
    assert_eq!(i.stdout, "", "vế phải không được chạy");
    assert!(!i.stdout.contains("rhs"));
    assert!(!i.stdout.contains("khong"));
    assert_eq!(i.diagnostics[0].code.short, "VS0503");
    assert!(
        i.diagnostics[0].message.contains("ngoài phạm vi"),
        "{:?}",
        i.diagnostics[0]
    );
    // Hai runtime chính cũng trap cùng mã/nội dung (exit của chúng là 101).
    let (hir, syms, tys) = tien_trinh(&src);
    let ri = vis_interp::run(&src, &hir, &syms, &tys, &[]);
    let m = vis_bytecode::compile(&hir, &syms, &src).expect("compile");
    let mut vm = Vm::new(&m, &src);
    let rv = vm.run(&[]);
    assert_eq!(ri.stdout, "", "interp thường cũng không chạy RHS");
    assert_eq!(rv.stdout, "");
    assert_eq!(ri.codes(), vec!["VS0503"]);
    assert_eq!(rv.codes(), vec!["VS0503"]);
    assert_eq!(ma(&i.diagnostics), ma(&ri.diagnostics));
    assert_eq!(ma(&v.diagnostics), ma(&rv.diagnostics));
    assert_eq!(i.diagnostics[0].message, ri.diagnostics[0].message);
    assert_eq!(v.diagnostics[0].message, rv.diagnostics[0].message);
}

#[test]
fn chi_so_ngoai_pham_vi_o_la_trap_sau_ve_phai() {
    // Lá `xs[0][3]`: RHS chạy trước, rồi `Set` mới trap `VS0501`.
    let src = "fn rhs() -> int:\n    print(\"rhs\")\n    9\n\nfn main():\n    var xs: list[list[int]] = [[1]]\n    xs[0][3] = rhs()\n    print(\"khong\")\n";
    let (i, v) = frame_hai_duong(src);
    assert_eq!(i.exit, 1);
    assert_eq!(i.stdout, "rhs\n", "vế phải phải chạy trước khi ghi lá");
    assert_eq!(i.diagnostics[0].code.short, "VS0501");
    assert!(
        i.diagnostics[0].message.contains("chỉ số 3 ngoài phạm vi"),
        "{:?}",
        i.diagnostics[0]
    );
    assert_eq!(v.diagnostics[0].message, i.diagnostics[0].message);
}

// ===========================================================================
// 4 — Gốc bất biến: tyck chặn tĩnh; frame interp còn chốt phòng thủ runtime
// ===========================================================================

#[test]
fn goc_let_bi_tu_choi_tinh() {
    let src =
        format!("{P}fn main():\n    let q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    print(q.p.x)\n");
    let p = parse(&src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(&src, Mode::Script);
    let h = vis_types::lower(&p.cst, &l);
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(!t.is_clean(), "tyck phải chặn gốc `let`");
    let d = t
        .diags
        .iter()
        .find(|d| d.code.short == "VS0412")
        .expect("phải có VS0412");
    assert!(
        d.message.contains("`let`/tham số"),
        "thông điệp: {}",
        d.message
    );
}

#[test]
fn goc_let_frame_interp_trap_phong_thu() {
    // Bỏ qua tyck để chạm chốt 2 của frame interp (chương trình thật không bao giờ tới đây).
    // So với đích `Var` trực tiếp: **cùng mã** `MISMATCH` (hiện là `VS0405`, chốt phòng thủ
    // của interp `ghi_o`; tyck mới là nơi phát `VS0412`), cùng tinh thần "`let`/tham số".
    let src =
        format!("{P}fn main():\n    let q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    print(q.p.x)\n");
    let truc_tiep = "fn main():\n    let x = 1\n    x = 2\n    print(x)\n";
    let chay_khong_check = |nguon: &str| {
        let p = parse(nguon, Mode::Script);
        assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
        let l = lex(nguon, Mode::Script);
        let h = vis_types::lower(&p.cst, &l);
        assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
        let o = run_frame_i(
            nguon,
            &h.hir,
            &h.symbols,
            &h.types,
            &[],
            FrameConfig::default(),
        );
        assert_eq!(o.exit, 1, "frame interp phải trap");
        assert_eq!(o.diagnostics.len(), 1, "{:?}", o.diagnostics);
        assert_eq!(o.stdout, "", "không được ghi/ in gì");
        o
    };
    let i = chay_khong_check(&src);
    let j = chay_khong_check(truc_tiep);
    assert_eq!(
        i.diagnostics[0].code.short, j.diagnostics[0].code.short,
        "mã phòng thủ phải khớp gán `var` trực tiếp"
    );
    assert!(
        i.diagnostics[0].message.contains("`let`/tham số"),
        "{:?}",
        i.diagnostics[0]
    );
}

// ===========================================================================
// 5 — `+=` lồng: ngoài phạm vi VD-270, frame interp từ chối **nhất quán**
// ===========================================================================

#[test]
fn ghep_long_van_bi_frame_interp_tu_choi_nhat_quan() {
    // Hai hình dạng lá (trường / phần tử) phải trap **cùng một kiểu**; interp/VM thường và
    // frame VM vẫn chạy được — đây là nợ đã ghi, không phải parity giả.
    let field_src = "type P:\n    n: int\n\ntype Q:\n    p: P\n\nfn main():\n    var q: Q = {p: {n: 40}}\n    q.p.n += 2\n    print(q.p.n)\n";
    let index_src = "type P:\n    xs: list[int]\n\ntype Q:\n    p: P\n\nfn main():\n    var q: Q = {p: {xs: [1, 2]}}\n    q.p.xs[1] += 8\n    print(q.p.xs)\n";

    for (src, manh) in [
        (field_src, "gán ghép trên trường"),
        (index_src, "gán ghép trên phần tử"),
    ] {
        let (hir, syms, tys) = tien_trinh(src);
        let i = run_frame_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
        assert_eq!(i.exit, 1, "frame interp phải từ chối: {:?}", i.stdout);
        assert_eq!(i.diagnostics.len(), 1);
        assert_eq!(i.diagnostics[0].code.short, "VS0501");
        assert!(
            i.diagnostics[0].message.contains(manh),
            "thông điệp phải nêu `{manh}`: {:?}",
            i.diagnostics
        );
        assert!(i.diagnostics[0]
            .message
            .contains("chưa được đường frame test hỗ trợ"));

        // Runtime chính vẫn hỗ trợ (VD-268), và frame VM chạy bytecode nên cũng hỗ trợ.
        let (_o, e) = runtime_hai_duong(src);
        assert_eq!(e, 0);
        let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
        let v = run_frame_v(&m, src, &[], FrameConfig::default());
        assert_eq!(v.exit, 0, "frame VM: {:?}", v.diagnostics);
        assert!(
            !v.stdout.is_empty(),
            "frame VM phải in kết quả, không trap như frame interp"
        );
    }
}

// ===========================================================================
// 6 — `vis run` / `vis run --vm` không hồi quy
// ===========================================================================

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-vd270-frame-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn chay_cli(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run");
    if vm {
        c.arg("--vm").arg("--no-cache");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn vis_run_va_vm_khong_hoi_quy() {
    let d = tam("cli");
    let src = format!(
        "{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    var xs: list[P] = [{{x: 3}}]\n    xs[0].x = 4\n    var m: map[str, P] = {{\"k\": {{x: 5}}}}\n    m[\"k\"].x = 6\n    print(q.p.x)\n    print(xs[0].x)\n    print(m[\"k\"].x)\n"
    );
    let p = d.join("nested.vis");
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    let a = chay_cli(&p, false);
    let b = chay_cli(&p, true);
    assert_eq!(a.0, 0, "interp stderr: {}", a.2);
    assert_eq!(b.0, 0, "vm stderr: {}", b.2);
    assert_eq!(a.1, "2\n4\n6\n");
    assert_eq!((a.0, &a.1), (b.0, &b.1), "interp ≠ --vm");
    let _ = std::fs::remove_dir_all(&d);
}
