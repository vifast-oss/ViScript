// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 — `recv?.m(args)` dùng kiểu nhận *suy ra*, không chỉ chú thích tĩnh.**
//!
//! VD-269 mở `recv?.m(…)` nhưng các ca chạy được đều có chú thích kiểu (`let a: U? = …`)
//! hoặc kiểu nhận là biểu thức tự mang kiểu. Khi biến nhận chỉ có kiểu **suy ra** từ
//! `let <tên> = <expr>` và kiểu đó không nằm trong `type_env`/annot, lowering (và codegen VM)
//! mất đường phân giải sym phương thức:
//!
//! ```text
//! # trước bản sửa — `fn may[T](x: T) -> T?`, `let r = may(U(n: 3))`, `r?.lay()`
//! $ vis run --no-cache --vm …
//! …: VS0501: `U(n: 3)` không có phương thức `lay`      (interp: `Some(3)`)
//!
//! # trước bản sửa — `impl Hop[int]` + `let h: Hop[int]?` / suy từ `tim()`
//! $ vis check …
//! # VS0405 … `Hop` không có phương thức `lay` (VD-269 — kiểm tra `impl`/tên)
//! ```
//!
//! Bản sửa: (a) [`vis_types::MethodRet`] đăng ký được `impl` trên **instantiation**
//! (`impl Hop[int]`) và checker tra sym **kèm `TypeTable`**; (b) VM có **fallback theo tên
//! phương thức duy nhất** khi kiểu nhận vẫn chưa biết (lambda/generic/kênh), khớp fallback
//! theo tên `Value` của interp. Không đổi `recv.m()` (không optional), không đổi kiểm số đối
//! số R5c.
//!
//! **Ngoài phạm vi (giữ nguyên)**: `let h = H(…); h.get()` — `recv.m()` **không** optional
//! trên biến nhận suy ra vẫn là `VS0403` "không phân giải được phương thức … cho kiểu không
//! tĩnh" như trước; bản sửa chỉ mở đường cho `recv?.m(…)`.
//!
//! Mỗi ca chạy `vis check` + **cả hai engine** (`--no-cache` và `--no-cache --vm`) và đòi
//! byte exit/stdout/stderr khớp nhau.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-oi-{tag}-{}-{n}", std::process::id()));
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
// 0 — kiểu tĩnh (chú thích) vẫn là ưu tiên: hồi quy VD-269
// ---------------------------------------------------------------------------

#[test]
fn kieu_tinh_van_chay() {
    chay_duoc(
        "static",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

    fn cong(self, x: int) -> int:
        self.n + x

    fn tang(self) -> int?:
        Some(self.n + 1)

fn main():
    let a: U? = Some(U(n: 3))
    let b: Option[U] = Some(U(n: 5))
    let c: U? = None
    print(a?.lay())
    print(a?.cong(4))
    print(a?.tang())
    print(b?.lay())
    print(c?.lay())
"#,
        "Some(3)\nSome(7)\nSome(4)\nSome(5)\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 1 — suy từ `let r = tim()` với `fn tim() -> T?` và `Option[T]`
// ---------------------------------------------------------------------------

#[test]
fn suy_tu_let_goi_ham_tra_option() {
    chay_duoc(
        "infer-call",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

    fn cong(self, x: int) -> int:
        self.n + x

    fn tang(self) -> int?:
        Some(self.n + 1)

fn tim() -> U?:
    Some(U(n: 3))

fn tim2() -> Option[U]:
    Some(U(n: 4))

fn khong() -> U?:
    None

fn main():
    let r = tim()
    let s = tim2()
    let z = khong()
    print(r?.lay())
    print(r?.cong(4))
    print(r?.tang())
    print(s?.lay())
    print(z?.lay())
"#,
        "Some(3)\nSome(7)\nSome(4)\nSome(4)\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 2 — suy từ lời gọi **generic** (`T?`) — trước bản sửa VM mất sym
// ---------------------------------------------------------------------------

#[test]
fn suy_tu_let_goi_ham_generic() {
    chay_duoc(
        "infer-generic-call",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

fn may[T](x: T) -> T?:
    Some(x)

fn main():
    let r = may(U(n: 6))
    print(r?.lay())
"#,
        "Some(6)\n",
    );
}

// ---------------------------------------------------------------------------
// 3 — suy qua biến **giá trị hàm** (`let tim = fn() -> T?`)
// ---------------------------------------------------------------------------

#[test]
fn suy_qua_bien_gia_tri_ham() {
    chay_duoc(
        "infer-lambda",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

    fn cong(self, x: int) -> int:
        self.n + x

fn main():
    let tim = fn() -> U?:
        Some(U(n: 7))
    let r = tim()
    print(r?.lay())
    print(r?.cong(x: 4))
"#,
        "Some(7)\nSome(11)\n",
    );
}

// ---------------------------------------------------------------------------
// 4 — suy từ `recv()` của kênh (`T?`): interp đúng, VM từng mất sym
// ---------------------------------------------------------------------------

#[test]
fn suy_tu_kenh_recv() {
    chay_duoc(
        "infer-chan",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

fn main():
    let c = chan[U](1)
    c.send(U(n: 8))
    let r = c.recv()
    print(r?.lay())
"#,
        "Some(8)\n",
    );
}

// ---------------------------------------------------------------------------
// 5 — `impl` trên **instantiation** (`impl Hop[int]`) — trước bản sửa `VS0405`
// ---------------------------------------------------------------------------

#[test]
fn impl_tren_instantiation() {
    chay_duoc(
        "impl-inst",
        r#"type Hop[T]:
    v: T

impl Hop[int]:
    fn lay(self) -> int:
        self.v

    fn tang(self) -> int?:
        Some(self.v + 1)

fn tim() -> Hop[int]?:
    Some(Hop(v: 9))

fn main():
    let tinh: Hop[int]? = Some(Hop(v: 10))
    let r = tim()
    print(tinh?.lay())
    print(tinh?.tang())
    print(r?.lay())
    print(r?.tang())
"#,
        "Some(10)\nSome(11)\nSome(9)\nSome(10)\n",
    );
}

// ---------------------------------------------------------------------------
// 6 — `Result` rồi bóc bằng `?`: biến nhận suy ra `T?`
// ---------------------------------------------------------------------------

#[test]
fn suy_tu_result_boc_bang_hoi() {
    chay_duoc(
        "infer-try",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

fn may() -> Result[U?, Error]:
    Ok(Some(U(n: 11)))

fn chay() -> Result[(), Error]:
    let r = may()?
    print(r?.lay())
    Ok(())

fn main() -> Result[(), Error]:
    chay()
"#,
        "Some(11)\n",
    );
}

// ---------------------------------------------------------------------------
// 7 — đoản mạch: `None` **không** đánh giá đối số (kể cả biến nhận suy ra)
// ---------------------------------------------------------------------------

#[test]
fn doan_mach_khong_goi_doi_so() {
    chay_duoc(
        "infer-short",
        r#"type U:
    n: int

impl U:
    fn cong(self, x: int) -> int:
        self.n + x

fn side() -> int:
    print("side")
    1

fn tim() -> U?:
    Some(U(n: 3))

fn khong() -> U?:
    None

fn main():
    let a = tim()
    let b = khong()
    print(a?.cong(side()))
    print(b?.cong(side()))
    print(b?.cong(4))
"#,
        "side\nSome(4)\nNone\nNone\n",
    );
}

// ---------------------------------------------------------------------------
// 8 — thiếu phương thức vẫn là lỗi tĩnh trên biến nhận **suy ra**
// ---------------------------------------------------------------------------

#[test]
fn thieu_phuong_thuc_van_loi() {
    tu_choi(
        "infer-no-method",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

fn tim() -> U?:
    Some(U(n: 1))

fn main():
    let r = tim()
    print(r?.khong_co())
"#,
        "VS0405",
        &["phương thức", "khong_co"],
    );
}

// ---------------------------------------------------------------------------
// 9 — fallback theo tên **không** cướp phương thức lõi của `list`/`str`/`map`
// ---------------------------------------------------------------------------

#[test]
fn fallback_khong_cuop_phuong_thuc_loi() {
    chay_duoc(
        "fallback-builtin",
        r#"type U:
    n: int

impl U:
    fn get(self) -> int:
        self.n

fn main():
    let xs: list[int]? = Some([5, 6])
    let u = Some(U(n: 9))
    print(xs?.get(0))
    print(xs?.first())
    print(u?.get())
"#,
        "Some(5)\nSome(5)\nSome(9)\n",
    );
}

// ---------------------------------------------------------------------------
// 10 — biến nhận cấp tệp suy ra từ `let` vẫn phân giải
// ---------------------------------------------------------------------------

#[test]
fn bien_nhan_cap_tep_suy_ra() {
    chay_duoc(
        "infer-global",
        r#"type U:
    n: int

impl U:
    fn lay(self) -> int:
        self.n

fn tim() -> U?:
    Some(U(n: 12))

let r = tim()

fn main():
    print(r?.lay())
"#,
        "Some(12)\n",
    );
}
