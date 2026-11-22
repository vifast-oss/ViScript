// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 — `impl Trait for Type` xuyên module**
//!
//! Trước bản sửa, bảng `impl` chỉ có **cục bộ từng tệp** (`Lowerer::impls`). `impl DoLuong for Hinh`
//! hạ ở `geometry.vis`, nhưng dispatch `h.dien_tich()` ở `main.vis` không thấy ⇒:
//!
//! ```text
//! $ vis check main.vis
//! not ok 1 - …/main.vis
//! # VS0403 6:11 trait `DoLuong` có phương thức `dien_tich` nhưng `Hinh` chưa cài
//! ```
//!
//! Bản sửa: `impl` (viết tay **và** sinh từ `derives`) được đăng ký thêm vào bảng **chung** của
//! đơn vị biên dịch (`Symbols::impls`), mỗi bản ghi kèm module khai. Khi hạ một tệp, dispatch tra
//! bảng cục bộ **+** bảng chung của module khác ⇒ gọi được phương thức `impl` đã hạ ở tệp phụ thuộc.
//!
//! Hợp đồng khoá bởi tệp này:
//! 1. `trait` + `type` + `impl` cùng một module, gọi từ tệp chính qua `use` ⇒ chạy đúng;
//! 2. `impl` generic (`impl Hop[T]`) xuyên module giữ đúng thay `T` ở call-site;
//! 3. `impl` đặt ở module **khai trait** (kiểu nhập từ module khác) vẫn hợp lệ (mồ côi đúng luật);
//! 4. `impl` mồ côi **thật** (trait + kiểu đều nhập) vẫn `VS0403` + nêu "mồ côi";
//! 5. `impl` trùng vẫn lỗi (`VS0302`) và dispatch xuyên module báo **mơ hồ** (`VS0403`);
//! 6. inherent `impl Type:` xuyên module cũng dispatch được;
//! 7. `impl` sinh từ `derives` (Show/Eq) cũng vào bảng chung;
//! 8. tên phương thức ở module khác **không** làm hẹp dispatch lõi khi kiểu nhận chưa tĩnh
//!    (`xs.get(1)` trên `list` vẫn chạy);
//! 9. interp và `--vm` **byte-khớp** (exit/stdout/stderr) ở mọi ca, kể cả ca lỗi.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-im-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) {
    let p = d.join(ten);
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
}

/// Ghi một cây nhiều tệp; trả `(thư mục, tệp chính)`.
fn cay(tag: &str, tep: &[(&str, &str)]) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    for (ten, src) in tep {
        ghi(&d, ten, src);
    }
    let main = d.join("main.vis");
    (d, main)
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
fn hai_duong(p: &Path) -> [(i32, String, String); 2] {
    let a = chay_tep(p, false);
    let b = chay_tep(p, true);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "interp ≠ VM (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    [a, b]
}

/// Ca hợp lệ: `check` xanh, hai engine exit 0, stdout đúng byte, stderr rỗng.
fn chay_duoc(tag: &str, tep: &[(&str, &str)], muon: &str) {
    let (d, p) = cay(tag, tep);
    let (c, co, ce) = check_tep(&p);
    assert_eq!(
        c, 0,
        "{tag}: `vis check` phải xanh\nstdout: {co}\nstderr: {ce}"
    );
    let [a, b] = hai_duong(&p);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{tag} [{name}]: phải chạy sạch\nstderr: {}", o.2);
        assert_eq!(o.1, muon, "{tag} [{name}]: stdout sai\nstderr: {}", o.2);
        assert_eq!(o.2, "", "{tag} [{name}]: stderr phải rỗng");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca bị từ chối tĩnh: `check` exit 1 + chứa mã; hai engine exit 2, chẩn đoán **byte-khớp**
/// nhau và chứa đủ các mảnh (`manh`).
fn tu_choi(tag: &str, tep: &[(&str, &str)], ma: &str, manh: &[&str]) {
    let (d, p) = cay(tag, tep);
    let (c, co, ce) = check_tep(&p);
    assert_eq!(
        c, 1,
        "{tag}: `vis check` phải đỏ\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        co.contains(ma) || ce.contains(ma),
        "{tag}: `check` thiếu mã {ma}\nstdout: {co}\nstderr: {ce}"
    );
    let [a, b] = hai_duong(&p);
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
    let _ = std::fs::remove_dir_all(&d);
}

const GEOMETRY: &str = r#"pub trait DoLuong:
    fn dien_tich(self) -> int

pub type Hinh:
    canh: int

impl DoLuong for Hinh:
    fn dien_tich(self) -> int:
        self.canh * self.canh
"#;

const MAIN_GEOMETRY: &str = r#"use ./geometry

fn main():
    let h: Hinh = Hinh(canh: 3)
    print(h.dien_tich())
"#;

// ---------------------------------------------------------------------------
// 1 — trait + type + impl ở module khác, gọi từ tệp chính
// ---------------------------------------------------------------------------

#[test]
fn trait_impl_o_module_khac_dispatch_duoc() {
    chay_duoc(
        "co-ban",
        &[("geometry.vis", GEOMETRY), ("main.vis", MAIN_GEOMETRY)],
        "9\n",
    );
}

// ---------------------------------------------------------------------------
// 2 — impl generic xuyên module: `T` vẫn được thay ở call-site
// ---------------------------------------------------------------------------

#[test]
fn impl_generic_xuyen_module() {
    chay_duoc(
        "generic",
        &[
            (
                "hop.vis",
                r#"pub type Hop[T]:
    v: T

impl Hop[T]:
    fn get(self) -> T:
        self.v

    fn wrap(self, x: T) -> Hop[T]:
        Hop(v: x)
"#,
            ),
            (
                "main.vis",
                r#"use ./hop

fn main():
    let a: Hop[int] = Hop(v: 1)
    print(a.get())
    let b: Hop[str] = Hop(v: "x")
    let c: Hop[str] = b.wrap("y")
    print(c.get())
"#,
            ),
        ],
        "1\ny\n",
    );
}

/// **VD-270** — template kiểu generic dùng chung toàn chương trình ⇒ `T` được thay **thật** ở
/// call-site khác module (không chỉ dispatch chạy).
#[test]
fn impl_generic_xuyen_module_kiem_kieu_that() {
    tu_choi(
        "generic-sai",
        &[
            (
                "hop.vis",
                r#"pub type Hop[T]:
    v: T

impl Hop[T]:
    fn get(self) -> T:
        self.v
"#,
            ),
            (
                "main.vis",
                r#"use ./hop

fn main():
    let a: Hop[int] = Hop(v: 1)
    let s: str = a.get()
    print(s)
"#,
            ),
        ],
        "VS0412",
        &["khai báo `let`", "cần `str`", "được `int`"],
    );
}

// ---------------------------------------------------------------------------
// 3 — impl đặt ở module **khai trait**, kiểu nhập từ module khác (mồ côi hợp lệ)
// ---------------------------------------------------------------------------

#[test]
fn impl_o_module_khai_trait_ba_tep() {
    chay_duoc(
        "ba-tep",
        &[
            (
                "hinh.vis",
                r#"pub type P:
    x: int
"#,
            ),
            (
                "do.vis",
                r#"use ./hinh

pub trait Hien:
    fn hien(self) -> int

impl Hien for P:
    fn hien(self) -> int:
        self.x + 1
"#,
            ),
            (
                "main.vis",
                r#"use ./do
use ./hinh

fn main():
    let p: P = P(x: 2)
    print(p.hien())
"#,
            ),
        ],
        "3\n",
    );
}

// ---------------------------------------------------------------------------
// 4 — mồ côi **thật** vẫn lỗi: trait + kiểu đều nhập
// ---------------------------------------------------------------------------

#[test]
fn mo_coi_that_van_bao_vs0403() {
    tu_choi(
        "mo-coi-that",
        &[
            (
                "a.vis",
                r#"pub trait Hien:
    fn hien(self) -> int
"#,
            ),
            (
                "b.vis",
                r#"pub type P:
    x: int
"#,
            ),
            (
                "main.vis",
                r#"use ./a
use ./b

impl Hien for P:
    fn hien(self) -> int:
        self.x
"#,
            ),
        ],
        "VS0403",
        &["mồ côi"],
    );
}

// ---------------------------------------------------------------------------
// 5 — impl trùng trong module vẫn lỗi; dispatch xuyên module báo **mơ hồ**
// ---------------------------------------------------------------------------

#[test]
fn impl_trung_van_bao_va_dispatch_mo_ho() {
    tu_choi(
        "trung",
        &[
            (
                "geometry.vis",
                r#"pub trait DoLuong:
    fn dien_tich(self) -> int

pub type Hinh:
    canh: int

impl DoLuong for Hinh:
    fn dien_tich(self) -> int:
        self.canh * self.canh

impl DoLuong for Hinh:
    fn dien_tich(self) -> int:
        self.canh + self.canh
"#,
            ),
            ("main.vis", MAIN_GEOMETRY),
        ],
        "VS0302",
        &["dien_tich", "mơ hồ"],
    );
}

// ---------------------------------------------------------------------------
// 6 — inherent `impl Type:` xuyên module (không trait) cũng dispatch được
// ---------------------------------------------------------------------------

#[test]
fn inherent_impl_xuyen_module() {
    chay_duoc(
        "inherent",
        &[
            (
                "hinh.vis",
                r#"pub type Hinh:
    canh: int

impl Hinh:
    fn dien_tich(self) -> int:
        self.canh * self.canh
"#,
            ),
            (
                "main.vis",
                r#"use ./hinh

fn main():
    let h: Hinh = Hinh(canh: 4)
    print(h.dien_tich())
"#,
            ),
        ],
        "16\n",
    );
}

// ---------------------------------------------------------------------------
// 7 — `impl` **sinh từ `derives`** cũng vào bảng chung (Show/Eq gọi từ tệp chính)
// ---------------------------------------------------------------------------

#[test]
fn derives_impl_xuyen_module() {
    chay_duoc(
        "derives",
        &[
            (
                "san_pham.vis",
                r#"pub type SanPham derives Show, Eq:
    ten: str
    gia: int
"#,
            ),
            (
                "main.vis",
                r#"use ./san_pham

fn main():
    let a: SanPham = SanPham(ten: "banh", gia: 3)
    let b: SanPham = SanPham(ten: "banh", gia: 3)
    print(a.show())
    print(a.eq(b))
"#,
            ),
        ],
        "{ten: banh, gia: 3}\ntrue\n",
    );
}

// ---------------------------------------------------------------------------
// 8 — tên phương thức module khác không được làm hẹp dispatch lõi khi kiểu chưa tĩnh
// ---------------------------------------------------------------------------

#[test]
fn ten_phuong_thuc_module_khac_khong_chan_builtin() {
    chay_duoc(
        "builtin",
        &[
            (
                "hop.vis",
                r#"pub type Hop:
    v: int

impl Hop:
    fn get(self) -> int:
        self.v
"#,
            ),
            (
                "main.vis",
                r#"use ./hop

fn main():
    let xs = [10, 20]
    print(xs.get(1))
"#,
            ),
        ],
        "Some(20)\n",
    );
}

// ---------------------------------------------------------------------------
// 2b — enum generic xuyên module: template biến thể + `T` thay ở call-site
// ---------------------------------------------------------------------------

#[test]
fn enum_generic_xuyen_module() {
    chay_duoc(
        "enum-generic",
        &[
            (
                "hop.vis",
                r#"pub enum Hop[T]:
    Co(v: T)
    Khong

impl Hop[T]:
    fn hoac(self, mac_dinh: T) -> T:
        match self:
            Co(v): v
            Khong: mac_dinh
"#,
            ),
            (
                "main.vis",
                r#"use ./hop

fn main():
    let h: Hop[int] = Hop.Co(5)
    let k: Hop[str] = Hop.Khong
    print(h.hoac(9))
    print(k.hoac("z"))
"#,
            ),
        ],
        "5\nz\n",
    );
}
