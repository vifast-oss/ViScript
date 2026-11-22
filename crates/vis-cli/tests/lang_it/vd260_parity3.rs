// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-260 / parity 3** — kiểu đích **suy ra** và nhánh `if`/`match` lệch lớp Option.
//!
//! Căn cứ: VD-242 Q5 + §3.3 — coercion theo **kiểu đích đã biết**, kể cả biến `var`/`let`
//! suy ra Option, field và index; **không** phân biệt chỉ vì có/không có annotation. Khi đích
//! là `Option[T]` và nguồn khớp `T` thì nâng **đúng một lớp**; không nâng nhiều lớp.
//!
//! Các ca dưới đây từng lệch interp ↔ VM (hoặc cả hai cùng sai so với kiểu đích mà checker
//! đã chấp nhận):
//! 1. `var u = pick(true); u.f = 5` — interp `Some(5)`, VM `5` (VM không suy được `u: U`).
//! 2. `var x = Some(5); x = 7` — interp `7`, VM `Some(7)` (interp bỏ qua kiểu suy ra).
//! 3. `var xs = [Some(1)]; xs[0] = 5` và map tương tự — cả hai lưu `5`/`7` trần.
//! 4. `self.f = 5` trong `var self` — cả hai lưu `5` trần (thiếu kiểu của `self`).
//! 5. `let x = if false: f() else: 2` / `match` — nhánh plain không được nâng, `match` trap.
//!
//! Mọi ca hợp lệ chạy **cả hai** engine, so byte stdout/stderr/exit. Ca sai kiểu vẫn phải bị
//! checker từ chối ở cả hai đường (`VS0412`) — không nới lỏng để xanh.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd260p3-{tag}-{}-{n}", std::process::id()));
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

/// Chạy interp + VM; assert hai engine giống **byte** stdout/stderr/exit, rồi trả kết quả.
fn chay_ca_hai(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
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

/// Chương trình sai kiểu: `vis check` đỏ + **cả hai** engine từ chối tĩnh (exit 2, cùng mã).
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (d, p) = ghi_tep(tag, src);
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let co = String::from_utf8_lossy(&o.stdout).to_string();
    let ce = String::from_utf8_lossy(&o.stderr).to_string();
    assert_ne!(o.status.code().unwrap_or(-1), 0, "{tag}: `check` phải đỏ");
    assert!(
        co.contains(ma) || ce.contains(ma),
        "{tag}: `check` thiếu `{ma}`\nstdout: {co}\nstderr: {ce}"
    );
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "{tag} engine={i}: thiếu `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 1 — đích `field` với biến suy ra từ hàm trả record
// ===========================================================================

#[test]
fn field_tren_bien_suy_tu_ham() {
    dung(
        "field-suy-ham",
        r#"type U:
    f: int?

fn pick(flag: bool) -> U:
    U(f: Some(1))

fn main():
    var u = pick(true)
    u.f = 5
    print(u.f)
"#,
        "Some(5)\n",
    );
}

/// Đích field không Option vẫn **không** nâng (chống bọc oan).
#[test]
fn field_khong_option_khong_nang() {
    dung(
        "field-thuong",
        r#"type U:
    n: int
    f: int?

fn main():
    var u: U = U(n: 0, f: None)
    u.n = 5
    u.f = 7
    print(u.n)
    print(u.f)
"#,
        "5\nSome(7)\n",
    );
}

// ===========================================================================
// 2 — gán lại biến `var` có kiểu suy ra Option
// ===========================================================================

#[test]
fn gan_lai_bien_suy_option() {
    dung(
        "gan-bien-suy",
        r#"fn f() -> int?:
    Some(1)

fn main():
    var x = Some(5)
    x = 7
    print(x)
    x = None
    print(x)
    var y = f()
    y = 3
    print(y)
"#,
        "Some(7)\nNone\nSome(3)\n",
    );
}

// ===========================================================================
// 3 — đích index với list/map suy ra phần tử Option
// ===========================================================================

#[test]
fn index_list_map_suy_option() {
    dung(
        "index-suy",
        r#"fn main():
    var xs = [Some(1)]
    xs[0] = 5
    print(xs[0])
    var m = {"a": Some(1)}
    m["a"] = 7
    m["b"] = 8
    print(m["a"])
    print(m["b"])
"#,
        "Some(5)\nSome(7)\nSome(8)\n",
    );
}

/// `None` suy ra phần tử Option; gán plain phải nâng một lớp, không nhiều lớp.
#[test]
fn index_tu_none_suy_option() {
    dung(
        "index-none",
        r#"fn main():
    var xs = [None]
    xs[0] = 5
    print(xs[0])
"#,
        "Some(5)\n",
    );
}

#[test]
fn index_khong_option_khong_nang() {
    dung(
        "index-thuong",
        r#"fn main():
    var xs = [1, 2]
    xs[0] = 9
    print(xs)
"#,
        "[9, 2]\n",
    );
}

// ===========================================================================
// 4 — `self.f = 5` trong `var self` (thiếu kiểu của `self`)
// ===========================================================================

#[test]
fn self_field_var_self() {
    dung(
        "self-field",
        r#"type U:
    f: int?

impl U:
    fn set(var self) -> ():
        self.f = 5
        print(self.f)
        match self.f:
            Some(v): print(v)
            None: print("none")

fn main():
    let u: U = U(f: Some(1))
    u.set()
    print(u.f)
"#,
        "Some(5)\n5\nSome(1)\n",
    );
}

// ===========================================================================
// 5 — nhánh `if`/`match` plain ↔ Option: nâng nhánh plain theo kiểu đích
// ===========================================================================

#[test]
fn if_nhanh_plain_nang_option() {
    dung(
        "if-nhanh",
        r#"fn f() -> int?:
    Some(1)

fn main():
    let x = if false: f() else: 2
    print(x)
    match x:
        Some(v): print(v)
        None: print("none")
"#,
        "Some(2)\n2\n",
    );
}

/// Nhánh Option được chọn (đã đúng trước đây) không bị nâng chồng.
#[test]
fn if_nhanh_option_khong_nang_chong() {
    dung(
        "if-nhanh-opt",
        r#"fn f() -> int?:
    Some(1)

fn main():
    let x = if true: f() else: 2
    print(x)
    match x:
        Some(v): print(v)
        None: print("none")
"#,
        "Some(1)\n1\n",
    );
}

/// Dạng **câu lệnh** `if`/`match` ở cuối hàm cũng nâng nhánh plain (đường `gia_tri`).
#[test]
fn cau_lenh_if_match_nhanh_plain() {
    dung(
        "cau-lenh-nhanh",
        r#"fn f() -> int?:
    Some(1)

fn g(flag: bool) -> int?:
    if flag:
        f()
    else:
        2

fn h() -> int?:
    match 2:
        1: f()
        _: 3

fn main():
    print(g(false))
    print(g(true))
    print(h())
    match g(false):
        Some(v): print(v)
        None: print("none")
"#,
        "Some(2)\nSome(1)\nSome(3)\n2\n",
    );
}

#[test]
fn match_nhanh_plain_nang_option() {
    dung(
        "match-nhanh",
        r#"fn f() -> int?:
    Some(1)

fn main():
    let x = match 2:
        1: f()
        _: 2
    print(x)
    match x:
        Some(v): print(v)
        None: print("none")
"#,
        "Some(2)\n2\n",
    );
}

/// `var` suy từ nhánh `if`/`match`, sau đó gán lại plain: kiểu đích vẫn là Option.
/// Method dựng sẵn trả Option (`list.get/first`, `map.get`) cũng là kiểu đích của nhánh.
#[test]
fn if_nhanh_goi_method_tra_option() {
    dung(
        "if-method-opt",
        r#"fn main():
    let xs = [1]
    let m = {"a": 1}
    let a = if false: xs.get(0) else: 2
    let b = if false: xs.first() else: 3
    let c = if false: m.get("a") else: 4
    let d = if false: first(xs) else: 5
    print(a)
    print(b)
    print(c)
    print(d)
    match a:
        Some(v): print(v)
        None: print("none")
"#,
        "Some(2)\nSome(3)\nSome(4)\nSome(5)\n2\n",
    );
}

#[test]
fn bien_suy_tu_nhanh_roi_gan_lai() {
    dung(
        "bien-nhanh",
        r#"fn f() -> int?:
    Some(1)

fn main():
    var x = if false: f() else: 2
    x = 7
    print(x)
    var y = match 2:
        1: f()
        _: 2
    y = 8
    print(y)
"#,
        "Some(7)\nSome(8)\n",
    );
}

// ===========================================================================
// 6 — ca âm: literal trộn Option/plain vẫn bị từ chối (không nới lỏng)
// ===========================================================================

#[test]
fn list_tron_option_plain_van_tu_choi() {
    tu_choi(
        "list-tron",
        r#"fn f() -> int?:
    Some(1)

fn main():
    let xs = [f(), 2]
    print(xs)
"#,
        "VS0412",
    );
}

#[test]
fn map_tron_option_plain_van_tu_choi() {
    tu_choi(
        "map-tron",
        r#"fn f() -> int?:
    Some(1)

fn main():
    let m = {"a": f(), "b": 2}
    print(m["b"])
"#,
        "VS0412",
    );
}

#[test]
fn index_sai_kieu_van_tu_choi() {
    tu_choi(
        "index-sai-kieu",
        r#"fn main():
    var xs = [Some(1)]
    xs[0] = "x"
    print(xs[0])
"#,
        "VS0412",
    );
}
