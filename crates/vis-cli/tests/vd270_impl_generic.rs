// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 — `impl` có tham số kiểu generic (`impl Hop[T]:`)**
//!
//! Trước bản sửa, khai báo `impl Hop[T]:` bị từ chối và mọi phương thức của nó biến mất:
//!
//! ```text
//! $ vis check hop.vis
//! hop.vis:5:1: error VS0403: impl cho kiểu không phải record/enum khai trong tệp (VS1b)
//! hop.vis:15:11: error VS0403: không có phương thức `get` cho `Hop` (VS0403)
//! hop.vis:6:5: error VS0303: tên không tồn tại: kiểu `T` chưa hỗ trợ (VD-43) …
//! ```
//!
//! Bản sửa: (a) `ten_kieu` quy `TypeRef{"Hop", [T]}` về **tên kiểu gốc** — `impl` được nhận và
//! đăng ký sym đúng; (b) tham số kiểu của `impl` được **hợp nhất** vào `generics` từng phương
//! thức (chữ ký giữ `T`, thân xoá như `fn f[T]`); (c) `hop_nhat` buộc `T` theo **vị trí** khi
//! kiểu nhận là instantiation (`Hop[int]`), nhờ vậy `a.get()` trả `int` chứ không còn `T`.
//!
//! Hợp đồng khoá bởi tệp này:
//! 1. `impl Hop[T]` / `impl Pair[A, B]` / `impl` trên `enum` generic được nhận;
//! 2. gọi trên **thực thể cụ thể** `Hop[int]`/`Hop[str]` phân giải đúng và thay `T`;
//! 3. `self` đúng kiểu nhận (dispatch `self.m()` bên trong phương thức);
//! 4. trả `T`/`Hop[T]`/`Pair[B, A]` được thay và kiểm tĩnh;
//! 5. sai kiểu ⇒ `VS0412`, thiếu phương thức ⇒ `VS0403`;
//! 6. interp và `--vm` **byte-khớp** (exit/stdout/stderr) trên cùng tệp;
//! 7. `impl` không generic và `impl Hop[int]` giữ nguyên hành vi.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-ig-{tag}-{}-{n}", std::process::id()));
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
// 1 — `impl Hop[T]`: `T` trong trả về, tham số, `Hop[T]`, và `self.m()`
// ---------------------------------------------------------------------------

#[test]
fn impl_generic_hop_int_va_str() {
    chay_duoc(
        "hop",
        r#"type Hop[T]:
    v: T

impl Hop[T]:
    fn get(self) -> T:
        self.v

    fn wrap(self, x: T) -> Hop[T]:
        Hop(v: x)

    fn lai(self) -> T:
        self.wrap(self.v).get()

fn main():
    let a: Hop[int] = Hop(v: 1)
    let b: Hop[str] = Hop(v: "x")
    let n: int = a.get()
    let s: str = b.get()
    print(n)
    print(s)
    print(a.lai())
    let c: Hop[str] = b.wrap("y")
    print(c.get())
"#,
        "1\nx\n1\ny\n",
    );
}

// ---------------------------------------------------------------------------
// 2 — hai `impl` generic **khác nhau** (một type, một enum, hai tham số)
// ---------------------------------------------------------------------------

#[test]
fn hai_impl_generic_doc_lap() {
    chay_duoc(
        "hai",
        r#"type Pair[A, B]:
    trai: A
    phai: B

impl Pair[A, B]:
    fn lay_trai(self) -> A:
        self.trai

    fn lay_phai(self) -> B:
        self.phai

    fn doi(self) -> Pair[B, A]:
        Pair(trai: self.phai, phai: self.trai)

enum Hop[T]:
    Co(v: T)
    Khong

impl Hop[T]:
    fn hoac(self, mac_dinh: T) -> T:
        match self:
            Co(v): v
            Khong: mac_dinh

fn main():
    let p: Pair[int, str] = Pair(trai: 1, phai: "x")
    let a: int = p.lay_trai()
    let b: str = p.lay_phai()
    print(a)
    print(b)
    let q: Pair[str, int] = p.doi()
    print(q.lay_trai())
    print(q.lay_phai())
    let h: Hop[int] = Hop.Co(5)
    let k: Hop[str] = Hop.Khong
    print(h.hoac(9))
    print(k.hoac("z"))
"#,
        "1\nx\nx\n1\n5\nz\n",
    );
}

// ---------------------------------------------------------------------------
// 3 — `impl` generic và không generic cùng tồn tại; `impl Hop[int]` vẫn chạy
// ---------------------------------------------------------------------------

#[test]
fn impl_khong_generic_va_instantiation_giu_nguyen() {
    chay_duoc(
        "hon-hop",
        r#"type R:
    n: int

impl R:
    fn lay(self) -> int:
        self.n

type Hop[T]:
    v: T

impl Hop[int]:
    fn rieng(self) -> int:
        self.v + 100

impl Hop[T]:
    fn chung(self) -> T:
        self.v

fn main():
    let r: R = R(n: 3)
    let h: Hop[int] = Hop(v: 4)
    print(r.lay())
    print(h.rieng())
    print(h.chung())
"#,
        "3\n104\n4\n",
    );
}

// ---------------------------------------------------------------------------
// 4 — sai kiểu trả `T` ⇒ VS0412; sai kiểu đối số `T` ⇒ VS0412
// ---------------------------------------------------------------------------

#[test]
fn sai_kieu_tra_ve_t() {
    tu_choi(
        "sai-tra",
        r#"type Hop[T]:
    v: T

impl Hop[T]:
    fn get(self) -> T:
        self.v

fn main():
    let a: Hop[int] = Hop(v: 1)
    let s: str = a.get()
    print(s)
"#,
        "VS0412",
        &["khai báo `let`", "cần `str`", "được `int`"],
    );
}

#[test]
fn sai_kieu_doi_so_t() {
    tu_choi(
        "sai-doi-so",
        r#"type Hop[T]:
    v: T

impl Hop[T]:
    fn dat(self, x: T) -> T:
        x

fn main():
    let a: Hop[int] = Hop(v: 1)
    print(a.dat("sai"))
"#,
        "VS0412",
        &["tham số kiểu `T` đã suy là `int`", "`str`"],
    );
}

// ---------------------------------------------------------------------------
// 5 — thiếu phương thức ⇒ VS0403 (không bị "generic nuốt lỗi")
// ---------------------------------------------------------------------------

#[test]
fn khong_co_phuong_thuc() {
    tu_choi(
        "khong-co",
        r#"type Hop[T]:
    v: T

impl Hop[T]:
    fn get(self) -> T:
        self.v

fn main():
    let a: Hop[int] = Hop(v: 1)
    print(a.khong_co())
"#,
        "VS0403",
        &["không có phương thức `khong_co` cho `Hop`"],
    );
}
