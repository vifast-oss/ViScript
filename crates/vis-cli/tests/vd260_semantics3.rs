// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-260** — ba lỗ hổng còn lại của checker (đo trên main `6e26836`):
//!
//! 1. **Pattern bác được trong `let`/`for`**: `let Some(a) = None` từng xanh ở `vis check`,
//!    interp trap còn VM in `()`; `for Some(a) in list[int?]` bỏ qua phần tử không khớp và
//!    (ở interp) dùng lại binding cũ. Nay phải từ chối tĩnh `VS0412` trừ `let … else`,
//!    `if let`/`while let`, hoặc pattern **phủ hết** (`_`, tên trần, tuple/bản ghi toàn `_`).
//! 2. **Rò phạm vi + tên chưa phân giải**: binding `if let` không được sống ngoài nhánh;
//!    tên giá trị chưa khai (`nope`) phải `VS0301` thay vì im lặng `Type::Error`.
//!    Tên dựng sẵn/module/tiền tố kiểu vẫn được miễn.
//! 3. **Luật bản ghi**: ctor trường lạ (`U(a: 1, zzz: 2)`), ctor vị trí/rỗng (`U(1)`, `U()`),
//!    pattern trùng trường (`{a, a, b}`), `with` trùng trường, khai báo trùng trường/biến thể
//!    ⇒ từ chối tĩnh. Trường mặc định vẫn phải chạy.
//!
//! Mọi ca hợp lệ chạy cả `vis run` (interp) và `vis run --vm`, so byte stdout/stderr/exit.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd260-{tag}-{}-{n}", std::process::id()));
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

/// Chạy interp + VM, đã assert hai engine giống **byte** stdout/stderr/exit.
fn chay_ca_hai(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, b.0, "{tag}: exit interp↔VM lệch");
    assert_eq!(a.1, b.1, "{tag}: stdout interp↔VM lệch");
    assert_eq!(a.2, b.2, "{tag}: stderr interp↔VM lệch");
    [a, b]
}

fn check_src(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chương trình **không hợp lệ**: `vis check` đỏ + đúng mã, và **cả hai engine** `vis run`
/// cũng từ chối tĩnh (exit 2) với cùng mã — không engine nào chạy tiếp.
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_ne!(
        c, 0,
        "{tag}: `vis check` phải đỏ\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains(ma) || err.contains(ma),
        "{tag}: thiếu mã `{ma}`\nstdout: {out}\nstderr: {err}"
    );
    let (d, p) = ghi_tep(&format!("{tag}-run"), src);
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "{tag} engine={i}: thiếu mã `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Chạy hợp lệ trên cả hai engine và so stdout.
fn chay_duoc(tag: &str, src: &str, muon: &str) {
    let [a, b] = chay_ca_hai(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, muon, "{name}: stdout sai\nstderr: {}", o.2);
    }
}

// ===========================================================================
// 1 — pattern bác được trong `let`/`for`
// ===========================================================================

/// `let Some(a) = o` (không `else`) là refutable ⇒ `VS0412`; hai engine không được chạy tiếp.
#[test]
fn let_some_refutable_bi_tu_choi() {
    tu_choi(
        "let_some",
        r#"fn main():
    let o: int? = None
    let Some(a) = o
    print(a)
"#,
        "VS0412",
    );
}

/// `for Some(a) in list[int?]` — phần tử `None` không bind được ⇒ phải từ chối tĩnh.
#[test]
fn for_some_refutable_bi_tu_choi() {
    tu_choi(
        "for_some",
        r#"fn main():
    let xs: list[int?] = [Some(1), None]
    for Some(a) in xs:
        print(a)
"#,
        "VS0412",
    );
}

/// `let Ok(v) = r` không `else` là refutable ⇒ `VS0412`.
#[test]
fn let_ok_refutable_bi_tu_choi() {
    tu_choi(
        "let_ok",
        r#"fn main():
    let r: Result[int, str] = Ok(1)
    let Ok(v) = r
    print(v)
"#,
        "VS0412",
    );
}

/// `let … else` vẫn hợp lệ (đây là đường thoát cho pattern bác được), cả nhánh khớp và `else`.
#[test]
fn let_else_some_van_chay_hai_engine() {
    chay_duoc(
        "let_else",
        r#"fn lay(o: int?) -> int:
    let Some(a) = o else:
        return 0
    a

fn main():
    print(lay(Some(5)))
    print(lay(None))
"#,
        "5\n0\n",
    );
}

/// `let Ok(v) = r else err:` — nhánh khớp bind `v`, nhánh lỗi bind `err` đúng payload.
#[test]
fn let_else_ok_van_chay_hai_engine() {
    chay_duoc(
        "let_else_ok",
        r#"fn mo(r: Result[int, str]) -> str:
    let Ok(v) = r else err:
        return "loi: {err}"
    "ok: {v}"

fn main():
    print(mo(Ok(7)))
    print(mo(Err("hong")))
"#,
        "ok: 7\nloi: hong\n",
    );
}

/// `if let` vẫn hợp lệ.
#[test]
fn if_let_some_van_chay_hai_engine() {
    chay_duoc(
        "if_let",
        r#"fn main():
    let o: int? = Some(6)
    if let Some(v) = o:
        print(v)
"#,
        "6\n",
    );
}

/// Pattern **phủ hết** trong `let`: `_`, tên trần, tuple và bản ghi toàn pattern phủ hết.
#[test]
fn let_pattern_phu_het_van_chay_hai_engine() {
    chay_duoc(
        "let_total",
        r#"type P:
    a: int
    b: int

fn main():
    let _ = 1
    let pair = (2, 3)
    let (x, y) = pair
    let p = P(a: 4, b: 5)
    let {a, b} = p
    print(x, y)
    print(a, b)
"#,
        "2 3\n4 5\n",
    );
}

/// Pattern tuple phủ hết trong `for` vẫn chạy đúng trên hai engine.
#[test]
fn for_pattern_phu_het_van_chay_hai_engine() {
    chay_duoc(
        "for_total",
        r#"fn main():
    for (i, v) in [10, 20].enumerate():
        print(i, v)
"#,
        "0 10\n1 20\n",
    );
}

// ===========================================================================
// 2 — phạm vi checker + tên chưa phân giải
// ===========================================================================

/// Binding `if let` không được rò ra câu sau ⇒ `VS0301`, không để runtime `VS0501`.
#[test]
fn if_let_binding_khong_ro_ra_ngoai() {
    tu_choi(
        "iflet_leak",
        r#"fn main():
    let o: int? = Some(1)
    if let Some(v) = o:
        print(v)
    print(v)
"#,
        "VS0301",
    );
}

/// Binding `while let` cũng không rò ra ngoài vòng.
#[test]
fn while_let_binding_khong_ro_ra_ngoai() {
    tu_choi(
        "whilelet_leak",
        r#"fn main():
    var o: int? = Some(1)
    while let Some(v) = o:
        print(v)
        o = None
    print(v)
"#,
        "VS0301",
    );
}

/// Tên giá trị chưa khai trong `let` phải `VS0301` tĩnh, không để runtime `VS0501`.
#[test]
fn ten_chua_phan_giai_bi_tu_choi() {
    tu_choi(
        "free_name",
        r#"fn main():
    let x: int = nope
    print(x)
"#,
        "VS0301",
    );
}

/// Tên chưa khai trong lời gọi cũng phải `VS0301`.
#[test]
fn ten_chua_phan_giai_trong_loi_goi_bi_tu_choi() {
    tu_choi(
        "free_call",
        r#"fn main():
    print(nope)
"#,
        "VS0301",
    );
}

/// Tên dựng sẵn và hàm `std.*` vẫn được miễn `VS0301`.
#[test]
fn builtin_va_std_khong_bi_bao_oan() {
    chay_duoc(
        "builtin_ok",
        r#"use std.str

fn main():
    print(len("ab"))
    print(Some(1))
    print(None)
    print(str(3))
    print(str.split("a,b", ","))
"#,
        "2\nSome(1)\nNone\n3\n[a, b]\n",
    );
}

// ===========================================================================
// 3 — luật bản ghi
// ===========================================================================

/// Ctor trường lạ: `U(a: 1, zzz: 2)` ⇒ `VS0403`, cả hai engine dừng tĩnh.
#[test]
fn record_ctor_truong_la_bi_tu_choi() {
    tu_choi(
        "rec_unknown",
        r#"type U:
    a: int

fn main():
    let u = U(a: 1, zzz: 2)
    print(u)
"#,
        "VS0403",
    );
}

/// Ctor **vị trí** `U(1)` không được lặng lẽ dựng `U()` ⇒ `VS0412`.
#[test]
fn record_ctor_vi_tri_bi_tu_choi() {
    tu_choi(
        "rec_positional",
        r#"type U:
    a: int

fn main():
    let u = U(1)
    print(u)
"#,
        "VS0412",
    );
}

/// Ctor **rỗng** `U()` thiếu trường bắt buộc ⇒ `VS0412`.
#[test]
fn record_ctor_rong_bi_tu_choi() {
    tu_choi(
        "rec_empty",
        r#"type U:
    a: int

fn main():
    let u = U()
    print(u)
"#,
        "VS0412",
    );
}

/// Pattern bản ghi trùng trường `{a, a, b}` ⇒ `VS0412`.
#[test]
fn record_pattern_trung_truong_bi_tu_choi() {
    tu_choi(
        "rec_pat_dup",
        r#"type P:
    a: int
    b: int

fn main():
    let p = P(a: 1, b: 2)
    let {a, a, b} = p
    print(a, b)
"#,
        "VS0412",
    );
}

/// Pattern bản ghi trên **không phải** bản ghi (`let {a} = 5`) ⇒ `VS0412` (trước: runtime lệch).
#[test]
fn record_pattern_tren_khong_phai_ban_ghi_bi_tu_choi() {
    tu_choi(
        "rec_pat_not_rec",
        r#"fn main():
    let {a} = 5
    print(a)
"#,
        "VS0412",
    );
}

/// `with` trùng trường ⇒ `VS0412`, không lặng lẽ last-wins.
#[test]
fn with_trung_truong_bi_tu_choi() {
    tu_choi(
        "with_dup",
        r#"type P:
    a: int
    b: int

fn main():
    let p = P(a: 1, b: 2)
    let q = p with a: 3, a: 4
    print(q)
"#,
        "VS0412",
    );
}

/// Khai báo bản ghi trùng trường ⇒ `VS0412`.
#[test]
fn khai_bao_record_trung_truong_bi_tu_choi() {
    tu_choi(
        "rec_decl_dup",
        r#"type U:
    a: int
    a: str

fn main():
    print("ok")
"#,
        "VS0412",
    );
}

/// Khai báo enum trùng biến thể ⇒ `VS0412`.
#[test]
fn khai_bao_enum_trung_bien_the_bi_tu_choi() {
    tu_choi(
        "enum_decl_dup",
        r#"enum E:
    A(x: int)
    A(y: str)

fn main():
    print("ok")
"#,
        "VS0412",
    );
}

/// Khai báo biến thể trùng tên trường ⇒ `VS0412`.
#[test]
fn khai_bao_bien_the_trung_truong_bi_tu_choi() {
    tu_choi(
        "enum_field_dup",
        r#"enum E:
    A(x: int, x: str)

fn main():
    print("ok")
"#,
        "VS0412",
    );
}

/// Trường có giá trị mặc định vẫn dựng đúng ở cả hai engine (không bị luật mới làm hỏng).
#[test]
fn record_default_van_chay_hai_engine() {
    chay_duoc(
        "rec_default",
        r#"type U:
    a: int
    b: int = 7

fn main():
    let u = U(a: 1)
    print(u)
"#,
        "U(a: 1, b: 7)\n",
    );
}
