// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269 — gọi phương thức qua `?.`: `recv?.m(args)`.**
//!
//! **Đỏ-trước (bằng chứng, main `b1d962f`)**: `u?.f` chạy được, `u.m()` chạy được, nhưng
//! `u?.m()` bị **checker** từ chối:
//!
//! ```text
//! $ vis check vd269.vis
//! not ok 1 - …
//! # VS0405 11:13 bản ghi không có trường `lay`
//! ```
//!
//! (exit **1**; `vis run` và `vis run --vm` cùng exit **2** với đúng dòng trên). Nguyên nhân:
//! lowering không dispatch callee `HExpr::Field { optional: true }`, checker rơi vào nhánh
//! truy cập **trường** nên báo "không có trường" cho tên **phương thức**, và hai engine không
//! có đường gọi.
//!
//! Sau VD-269: base `Option[T]` được đánh giá **một lần**; `None` ⇒ `None` **trước** khi đánh
//! giá đối số (đoản mạch); `Some(v)` ⇒ gọi `v.m(args)` (phương thức `impl`, phương thức lõi
//! `list`/`str`/`map`/`Chan`, hoặc `std.*`), rồi lift:
//!
//! * `U` ⇒ `U?` (plain → `Some`);
//! * `U?` ⇒ `U?` (không bọc hai lớp);
//! * `()` ⇒ `()` — ngôn ngữ **có** biểu diễn `()?` (`Some(())`), nhưng giữ `()` cho lời gọi
//!   chỉ vì tác dụng phụ (hai engine bỏ lift khi kết quả là `Value::Unit`);
//! * `{error}`/`never` giữ nguyên;
//! * base không phải `Option` (kể cả `Result`) ⇒ `VS0412`; thiếu phương thức trên bản ghi/enum
//!   ⇒ `VS0405`; sai kiểu đối số ⇒ `VS0412`.
//!
//! Mỗi ca chạy `vis check` + **cả hai engine** (`--no-cache` và `--no-cache --vm`) và đòi
//! byte exit/stdout/stderr khớp nhau.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd269-{tag}-{}-{n}", std::process::id()));
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

fn check_tep(p: &Path) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(p)
        .output()
        .expect("chạy `vis check`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM, đòi **byte** exit/stdout/stderr khớp nhau; trả `[interp, vm]`.
fn hai_duong(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "{tag}: interp ≠ VM (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    [a, b]
}

/// Ca hợp lệ: `check` xanh, hai engine exit 0, stdout đúng byte, stderr rỗng.
fn chay_duoc(tag: &str, src: &str, muon: &str) {
    let (d, p) = ghi_tep(tag, src);
    let (c, co, ce) = check_tep(&p);
    assert_eq!(
        c, 0,
        "{tag}: `vis check` phải xanh\nstdout: {co}\nstderr: {ce}"
    );
    let _ = std::fs::remove_dir_all(&d);
    let [a, b] = hai_duong(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{tag} [{name}]: phải chạy sạch\nstderr: {}", o.2);
        assert_eq!(o.1, muon, "{tag} [{name}]: stdout sai\nstderr: {}", o.2);
        assert_eq!(o.2, "", "{tag} [{name}]: stderr phải rỗng");
    }
}

/// Ca bị từ chối tĩnh: `check` exit 1 + chứa mã; hai engine exit 2, chẩn đoán **byte-khớp**
/// nhau và chứa đủ các mảnh (`manh`).
fn tu_choi(tag: &str, src: &str, ma: &str, manh: &[&str]) {
    let (d, p) = ghi_tep(tag, src);
    let (c, co, ce) = check_tep(&p);
    assert_eq!(
        c, 1,
        "{tag}: `vis check` phải đỏ\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        co.contains(ma) || ce.contains(ma),
        "{tag}: `check` thiếu mã {ma}\nstdout: {co}\nstderr: {ce}"
    );
    let _ = std::fs::remove_dir_all(&d);
    let [a, b] = hai_duong(tag, src);
    assert_eq!(
        (a.0, b.0),
        (2, 2),
        "{tag}: hai engine phải exit 2: {a:?} / {b:?}"
    );
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert!(o.2.contains(ma), "{tag} [{name}]: thiếu {ma}: {}", o.2);
        for m in manh {
            assert!(o.2.contains(m), "{tag} [{name}]: thiếu `{m}`: {}", o.2);
        }
    }
}

// ---------------------------------------------------------------------------
// 1 — kiểu trả `int` / `int?`: lift đúng một lớp, `None` giữ `None`
// ---------------------------------------------------------------------------

#[test]
fn tra_int_va_int_option() {
    chay_duoc(
        "int-opt",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

    fn tang(self) -> int?:
        Some(self.n + 1)

    fn khong(self) -> int?:
        None

fn main():
    let a: U? = Some(U(n: 3))
    let b: U? = None
    print(a?.lay())
    print(b?.lay())
    print(a?.tang())
    print(a?.khong())
    print(b?.khong())
"#,
        "Some(3)\nNone\nSome(4)\nNone\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 2 — kiểu trả bản ghi / list: chain `?.` trên kết quả
// ---------------------------------------------------------------------------

#[test]
fn tra_ban_ghi_va_list_roi_chain() {
    chay_duoc(
        "rec-list",
        r#"type U:
    n: int

impl U:
    fn rec(self) -> U:
        U(n: self.n + 1)

    fn xs(self) -> list[int]:
        [self.n, 2]

fn main():
    let a: U? = Some(U(n: 3))
    let b: U? = None
    print(a?.rec())
    print(a?.rec()?.n)
    print(a?.xs())
    print(a?.xs()?.len())
    print(b?.rec()?.n)
"#,
        "Some(U(n: 4))\nSome(4)\nSome([3, 2])\nSome(2)\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 3 — đối số + đoản mạch: `None` **không** đánh giá đối số có tác dụng phụ
// ---------------------------------------------------------------------------

#[test]
fn doan_mach_khong_goi_doi_so() {
    chay_duoc(
        "short-circuit",
        r#"type U:
    n: int

impl U:
    fn cong(self, x: int) -> int:
        self.n + x

    fn hai(self, x: int, y: int) -> int:
        x * 10 + y

fn side() -> int:
    print("side")
    1

fn main():
    let a: U? = Some(U(n: 3))
    let b: U? = None
    print(a?.cong(4))
    print(a?.cong(side()))
    print(b?.cong(side()))
    print(b?.cong(4))
    print(a?.hai(y: 2, x: 1))
"#,
        // `side` chỉ chạy **một** lần (nhánh `Some`); `None` không đánh giá đối số.
        "Some(7)\nside\nSome(4)\nNone\nNone\nSome(12)\n",
    );
}

// ---------------------------------------------------------------------------
// 4 — `()`: giữ `()`, không bọc `Some(())`; `None` không chạy thân
// ---------------------------------------------------------------------------

#[test]
fn tra_unit_giu_unit() {
    chay_duoc(
        "unit",
        r#"type U:
    n: int

impl U:
    fn ping(self) -> ():
        print("ping")

fn main():
    let a: U? = Some(U(n: 1))
    let b: U? = None
    print(a?.ping())
    print(b?.ping())
"#,
        "ping\n()\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 5 — phương thức lõi (`list`/`str`): `get` không bọc hai lớp
// ---------------------------------------------------------------------------

#[test]
fn phuong_thuc_loi_tren_option() {
    chay_duoc(
        "builtin",
        r#"fn main():
    let xs: list[int]? = Some([1, 2, 3])
    let ys: list[int]? = None
    let s: str? = Some("a@b")
    let z: str? = None
    print(xs?.len())
    print(xs?.get(0))
    print(xs?.get(9))
    print(ys?.len())
    print(s?.split("@")?.last())
    print(z?.split("@"))
"#,
        "Some(3)\nSome(1)\nNone\nNone\nSome(b)\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 6 — chain sâu `a?.m()?.m()?.f` (bản ghi optional)
// ---------------------------------------------------------------------------

#[test]
fn chain_nhieu_tang() {
    chay_duoc(
        "chain",
        r#"type U:
    n: int

impl U:
    fn rec(self) -> U:
        U(n: self.n + 1)

    fn ore(self) -> U?:
        Some(U(n: self.n + 10))

    fn lay(self) -> int:
        self.n

fn main():
    let a: U? = Some(U(n: 1))
    let b: U? = None
    print(a?.rec()?.lay())
    print(a?.ore()?.lay())
    print(a?.ore()?.rec()?.lay())
    print(b?.rec()?.lay())
    print(b?.ore()?.lay())
    print((if true: Some(U(n: 8)) else: None)?.lay())
    print((if true: U(n: 10) else: None)?.lay())
    print((match 1:
        1: Some(U(n: 9))
        _: None)?.lay())
"#,
        "Some(2)\nSome(11)\nSome(12)\nNone\nNone\nSome(8)\nSome(10)\nSome(9)\n",
    );
}

// ---------------------------------------------------------------------------
// 7 — enum optional (Option là enum thật): phương thức `impl` trên enum
// ---------------------------------------------------------------------------

#[test]
fn enum_option() {
    chay_duoc(
        "enum",
        r#"enum E:
    A(n: int)
    B

impl E:
    fn val(self) -> int:
        match self:
            A(n): n
            B: 0

fn main():
    let a: E? = Some(E.A(5))
    let b: E? = None
    print(a?.val())
    print(b?.val())
"#,
        "Some(5)\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 8 — `u.m()` và `u?.f` giữ nguyên
// ---------------------------------------------------------------------------

#[test]
fn hanh_vi_cu_khong_doi() {
    chay_duoc(
        "unchanged",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

fn main():
    let a: U = U(n: 7)
    let b: U? = Some(a)
    print(a.lay())
    print(b?.n)
"#,
        "7\nSome(7)\n",
    );
}

// ---------------------------------------------------------------------------
// 9 — lỗi tĩnh: thiếu phương thức / sai đối số / base `Result` / base không Option
// ---------------------------------------------------------------------------

const U_CO: &str = "type U:\n    n: int\n\nimpl U:\n    fn lay(self) -> int:\n        self.n\n\n    fn cong(self, x: int) -> int:\n        self.n + x\n\n";

#[test]
fn loi_thieu_phuong_thuc() {
    tu_choi(
        "no-method",
        &format!("{U_CO}fn main():\n    let a: U? = Some(U(n: 1))\n    print(a?.khong_co())\n"),
        "VS0405",
        &["phương thức", "khong_co"],
    );
}

#[test]
fn loi_sai_kieu_doi_so() {
    tu_choi(
        "arg-type",
        &format!("{U_CO}fn main():\n    let a: U? = Some(U(n: 1))\n    print(a?.cong(\"x\"))\n"),
        "VS0412",
        &["đối số", "int"],
    );
}

#[test]
fn loi_base_result() {
    tu_choi(
        "result-base",
        "fn main():\n    let r: Result[int, str] = Ok(1)\n    print(r?.lay())\n",
        "VS0412",
        &["`?.`", "Option"],
    );
}

#[test]
fn loi_base_khong_option() {
    tu_choi(
        "non-opt-base",
        &format!("{U_CO}fn main():\n    let a: U = U(n: 1)\n    print(a?.lay())\n"),
        "VS0412",
        &["`?.`", "Option"],
    );
}
