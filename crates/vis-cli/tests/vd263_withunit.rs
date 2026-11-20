// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-263 / with+unit** — hai lỗi parity interp ↔ VM do review độc lập ở `6c2c9c2`:
//!
//! 1. **`with` nâng Option chỉ khi gốc là `Var`/`Field`/`Index`** — codegen lấy `RecId` từ
//!    `base_ty`, nên `U(f: None) with f: 5`, `make() with f: 5`, gọi phương thức, `if`/`match`,
//!    `with` lồng và biến bind bằng pattern đều **không** nâng ⇒ VM lưu `5` trần trong khi interp
//!    (tra tên bản ghi lúc chạy) lưu `Some(5)`. Đích là kiểu tĩnh của gốc, **không** dựa vào
//!    fallback tên lúc chạy của interp.
//! 2. **`[]` khớp `Value::Unit` trên VM** — `TestList(0)` và `TestTuple(0)` dùng chung một nhánh
//!    nên `match ():` rơi vào arm `[]:` trên VM, còn interp chỉ khớp `HPat::List` với
//!    `Value::List`. `TestList` phải chỉ khớp `Value::List`; `TestTuple(0)` vẫn khớp `Unit`.
//!
//! Mọi ca chạy **cả hai** engine (`vis run` và `vis run --vm --no-cache`), so **byte**
//! stdout/stderr/exit, và so stdout với kỳ vọng tường minh (parity một mình không đủ: cả hai
//! engine cùng sai vẫn "khớp").

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd263-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    c.args(extra);
    c.arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM; assert hai engine giống **byte** stdout/stderr/exit, rồi trả kết quả.
fn chay_ca_hai(tag: &str, src: &str) -> (i32, String, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
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

/// Chương trình hợp lệ: hai engine cùng chạy sạch, stdout đúng kỳ vọng.
fn dung(tag: &str, src: &str, stdout: &str) {
    let (code, out, err) = chay_ca_hai(tag, src);
    assert_eq!(
        code, 0,
        "{tag}: phải chạy sạch\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.is_empty(), "{tag}: stderr phải rỗng: {err}");
    assert_eq!(out, stdout, "{tag}: stdout sai");
}

/// Biên dịch + tyck nguồn rồi bọc `HExpr::With` quanh **initializer** của `let` đầu tiên trong
/// `main` (dùng cho gốc `if`/`match` — parser nguồn chưa nhận `(if …)`/`(match …)`).
fn boc_with_trong_main(src: &str, tag: &str) -> vis_types::Hir {
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    assert!(
        p.diagnostics.is_empty(),
        "{tag}: parse: {:?}",
        p.diagnostics
    );
    let l = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    assert!(h.is_clean(), "{tag}: lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "{tag}: tyck: {:?}", t.diags);
    let mut hir = h.hir;
    let mut da_boc = false;
    for it in &mut hir.items {
        let vis_types::HItem::Fn(f) = it else {
            continue;
        };
        if f.name != "main" {
            continue;
        }
        for s in &mut f.body {
            let vis_types::HStmt::Let { init, span, .. } = s else {
                continue;
            };
            let span = *span;
            let base = std::mem::replace(init, vis_types::HExpr::Error { span });
            *init = vis_types::HExpr::With {
                base: Box::new(base),
                fields: vec![(
                    "f".to_string(),
                    vis_types::HExpr::Lit {
                        lit: vis_types::hir::Lit {
                            kind: vis_types::LitKind::Int,
                            text: "5".to_string(),
                        },
                        ty: vis_types::Type::Int,
                        span,
                    },
                )],
                span,
            };
            da_boc = true;
            break;
        }
    }
    assert!(da_boc, "{tag}: không thấy `let` trong `main` để bọc `with`");
    hir
}

/// Chạy **cả hai** engine trên cùng HIR đã sửa (interp + VM) và so byte stdout/kỳ vọng.
fn chay_hir_hai_duong(src: &str, hir: &vis_types::Hir, tag: &str, stdout: &str) {
    let p = vis_syntax::parse(src, vis_syntax::Mode::Script);
    let l = vis_syntax::lex(src, vis_syntax::Mode::Script);
    let h = vis_types::lower_with(&p.cst, &l, vis_types::LowerOptions::default());
    let t = vis_types::check(hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "{tag}: tyck HIR đã sửa: {:?}", t.diags);

    let m = vis_bytecode::compile(hir, &h.symbols, src).expect("compile HIR đã sửa");
    let a = vis_interp::run(src, hir, &h.symbols, &h.types, &[]);
    assert!(a.is_clean(), "{tag}: interp: {:?}", a.diagnostics);
    let b = vis_vm::run(&m, src, &[]);
    assert!(b.is_clean(), "{tag}: VM: {:?}", b.diagnostics);
    assert_eq!(a.stdout, b.stdout, "{tag}: stdout interp↔VM lệch");
    assert_eq!(a.stdout, stdout, "{tag}: stdout sai");
}

// ===========================================================================
// 1 — `with` trên các dạng gốc khác `Var`/`Field`/`Index`
// ===========================================================================

/// Gốc là **literal bản ghi**: `U(f: None) with f: 5` ⇒ `Some(5)` ở cả hai engine.
#[test]
fn with_goc_literal_ban_ghi() {
    dung(
        "with-record-lit",
        r#"type U:
    f: int?

fn main():
    let r = U(f: None) with f: 5
    print(r)
    print(r.f)
    match r.f:
        Some(v): print(v)
        None: print("none")
"#,
        "U(f: Some(5))\nSome(5)\n5\n",
    );
}

/// Gốc là **lời gọi hàm cấp tệp** trả bản ghi: `make() with f: 5`.
#[test]
fn with_goc_loi_goi_ham() {
    dung(
        "with-call",
        r#"type U:
    f: int?

fn make() -> U:
    U(f: None)

fn main():
    let r = make() with f: 5
    print(r)
    print(r.f)
"#,
        "U(f: Some(5))\nSome(5)\n",
    );
}

/// Gốc là **lời gọi phương thức** (dispatch tĩnh `impl`): `u.make() with f: 5`.
#[test]
fn with_goc_loi_goi_phuong_thuc() {
    dung(
        "with-method",
        r#"type U:
    f: int?

impl U:
    fn make(self) -> U:
        U(f: None)

fn main():
    let u: U = U(f: Some(1))
    let r = u.make() with f: 5
    print(r)
    print(r.f)
"#,
        "U(f: Some(5))\nSome(5)\n",
    );
}

/// Gốc là **biến bind bằng pattern** (`Some(u)` và phần tử list) — kiểu đến từ tyck.
#[test]
fn with_goc_bien_bind_pattern() {
    dung(
        "with-pattern",
        r#"type U:
    f: int?

fn main():
    match Some(U(f: None)):
        Some(u): print(u with f: 5)
        None: print("none")
    match [U(f: None)]:
        [u]: print(u with f: 6)
        _: print("khong")
"#,
        "U(f: Some(5))\nU(f: Some(6))\n",
    );
}

/// Gốc là nhánh **`if`** (kiểu join hai nhánh đều là `U`).
///
/// **Hạn chế parser (ngoài phạm vi VD-263)**: `(if … )` / `(match … )` trong biểu thức chưa
/// được chấp nhận (`VS0201` "mong xuống dòng cuối khối"), nên không viết được `with` trực tiếp
/// trên `if`/`match` từ nguồn. Ca này dựng HIR bằng tay: lấy chính `HExpr::If` do lowering sinh
/// cho `let x = if …` rồi bọc `HExpr::With` — kiểm đúng đường `base_ty` mới + chạy **cả hai**
/// engine trên cùng HIR, đối chiếu byte.
#[test]
fn with_goc_if_hir() {
    let src = r#"type U:
    f: int?

fn main():
    let x = if true: U(f: None) else: U(f: None)
    print(x)
"#;
    let hir = boc_with_trong_main(src, "with-if-hir");
    chay_hir_hai_duong(src, &hir, "with-if-hir", "U(f: Some(5))\n");
}

/// Gốc là nhánh **`match`** — cùng cách dựng HIR như ca `if` (xem hạn chế parser ở trên).
#[test]
fn with_goc_match_hir() {
    let src = r#"type U:
    f: int?

fn main():
    let x = match true:
        true: U(f: None)
        false: U(f: Some(2))
    print(x)
"#;
    let hir = boc_with_trong_main(src, "with-match-hir");
    chay_hir_hai_duong(src, &hir, "with-match-hir", "U(f: Some(5))\n");
}

/// Gốc là **`with` lồng**: kết quả `with` vẫn mang kiểu bản ghi của gốc.
#[test]
fn with_goc_with_long() {
    dung(
        "with-nested",
        r#"type U:
    f: int?
    g: int?

fn main():
    let r = (U(f: None, g: None) with f: 1) with g: 2
    print(r)
    print(r.f)
    print(r.g)
"#,
        "U(f: Some(1), g: Some(2))\nSome(1)\nSome(2)\n",
    );
}

/// Gốc là **`?` postfix** trên hàm trả `U?`: `maybe()? with f: 5`.
#[test]
fn with_goc_try() {
    dung(
        "with-try",
        r#"type U:
    f: int?

fn maybe() -> U?:
    Some(U(f: None))

fn lay() -> U?:
    let r = maybe()? with f: 5
    Some(r)

fn main():
    match lay():
        Some(r): print(r)
        None: print("none")
"#,
        "U(f: Some(5))\n",
    );
}

/// Trường đích **không** Option vẫn không bị bọc oan khi gốc là literal/call.
#[test]
fn with_goc_khac_truong_thuong_khong_nang() {
    dung(
        "with-plain",
        r#"type U:
    n: int
    f: int?

fn main():
    let a = U(n: 0, f: None) with n: 5
    print(a)
    let b = U(n: 0, f: None) with f: None
    print(b)
    print(b.f)
"#,
        "U(n: 5, f: None)\nU(n: 0, f: None)\nNone\n",
    );
}

// ===========================================================================
// 2 — `[]` chỉ khớp list; `()` chỉ khớp pattern tuple rỗng
// ===========================================================================

/// `match ():` với arm `[]:` — Unit **không** khớp `[]` (interp `other`).
#[test]
fn match_unit_khong_khop_list_rong() {
    dung(
        "unit-vs-list",
        r#"fn main():
    match ():
        []: print("empty list")
        _: print("other")
    match []:
        []: print("empty list")
        _: print("other")
"#,
        "other\nempty list\n",
    );
}

/// `()` vẫn khớp pattern tuple rỗng `():` ở cả hai engine. Giống interp, pattern tuple rỗng
/// cũng khớp `[]` (list rỗng) — `Value::List([])` được cả hai coi là "tuple rỗng" tại runtime;
/// điểm sửa VD-263 chỉ là `[]` **không** được khớp `Unit`.
#[test]
fn match_unit_khop_tuple_rong() {
    dung(
        "unit-tuple",
        r#"fn main():
    match ():
        (): print("unit")
        _: print("other")
    match []:
        (): print("unit")
        _: print("other")
"#,
        "unit\nunit\n",
    );
}

/// `if let [] = ()` phải false; `if let [] = []` phải true.
#[test]
fn if_let_list_rong_chi_khop_list() {
    dung(
        "if-let-list",
        r#"fn main():
    if let [] = ():
        print("unit la list")
    else:
        print("unit khong phai list")
    if let [] = []:
        print("list rong")
    else:
        print("khong")
"#,
        "unit khong phai list\nlist rong\n",
    );
}

/// `let Some([]) = … else:` — `[]` trong payload **không** khớp `()` (và ngược lại `Some(())`
/// không khớp `[]`); trước VD-263 `TestList(0)` coi `Unit` là list rỗng nên VM rơi sai nhánh.
#[test]
fn let_else_list_rong_chi_khop_list() {
    dung(
        "let-else-list",
        r#"fn f(o: ()?) -> int:
    let Some([]) = o else:
        return 1
    0

fn g(o: ()?) -> int:
    let Some(()) = o else:
        return 1
    2

fn main():
    print(f(Some(())))
    print(g(Some(())))
    print(f(None))
"#,
        "1\n2\n1\n",
    );
}

/// Phần tử lồng: `[[]]` chỉ khớp list lồng, không khớp `[()]`; `[()]` chỉ khớp tuple.
#[test]
fn list_long_khong_khop_unit() {
    dung(
        "list-long",
        r#"fn main():
    match [[]]:
        [[]]: print("list long")
        _: print("khac")
    match [()]:
        [[]]: print("list long")
        _: print("khac")
    match [()]:
        [()]: print("tuple long")
        _: print("khac")
"#,
        "list long\nkhac\ntuple long\n",
    );
}
