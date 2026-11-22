// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-265 — kiểu trả về của phương thức người dùng đi vào Option lift, `?.`, `with` và gán
//! trường phải giống nhau ở interp và VM**.
//!
//! Bối cảnh: VD-264 đã suy kiểu đệ quy sau pattern và kết quả gọi **hàm**, nhưng lời gọi
//! **phương thức người dùng** còn hai lỗ:
//!
//! 1. **Dispatch tĩnh** (`lower::static_ty`) chỉ biết `type_env` của `let`/`var`/tham số **có
//!    annot**; `self` trong thân `impl` không có annot nên `self.m()` bị `VS0403`
//!    ("không phân giải được phương thức … cho kiểu không tĩnh"). Các ca dưới đây gọi qua
//!    `self` để **đỏ-trước** đúng lỗ đó (trước bản sửa: exit 2, VS0403, không chạy được).
//! 2. Khi HIR vẫn còn callee `HExpr::Field` (`recv.m`), `infer_ty` chỉ biết method dựng sẵn
//!    (`list.get/first/last/find`, `map.get`); phương thức người dùng trả `None`. Bảng
//!    `vis_types::MethodRet` (VD-265) bổ sung nhánh này + thay generic cho instantiation;
//!    chứng minh đỏ-trước ở unit test `goi::tests` (interp) và `than::tests` (bytecode).
//!    Lưu ý phạm vi: chương trình **hợp lệ** hiện luôn dispatch tĩnh (`Call(Var)`), còn dạng
//!    `recv?.m()` do **VD-269** mở (checker + interp + VM tra `MethodRet::sym`); nhánh `Field`
//!    ở đây vẫn là bảo hiểm cho HIR/API.
//!
//! Mỗi ca chạy **cả hai engine** trên cùng tệp — `vis run --no-cache` và
//! `vis run --no-cache --vm` — so **byte** exit/stdout/stderr.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd265-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
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

/// Chạy interp + VM trên cùng tệp, trả `[interp, vm]`.
fn hai_duong(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// Ca thành công: đòi exit + stdout + stderr **byte-khớp**, `exit = 0`, stderr rỗng.
fn thanh_cong(tag: &str, src: &str, stdout: &str) {
    let [a, b] = hai_duong(tag, src);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "{tag}: interp ≠ VM (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(a.0, 0, "{tag}: phải chạy sạch, stderr: {}", a.2);
    assert_eq!(a.1, stdout, "{tag}: stdout");
    assert_eq!(a.2, "", "{tag}: stderr phải rỗng");
}

// ===========================================================================
// 1 — `self.m() -> int?`: nhánh `if` lệch lớp Option phải nâng plain → Option
// ===========================================================================

#[test]
fn self_goi_phuong_thuc_tra_int_option() {
    thanh_cong(
        "self-int-opt",
        r#"type A:
    n: int?

impl A:
    fn lay(self) -> int?:
        self.n

    fn chon(self) -> int:
        let x = if false: self.lay() else: 5
        match x:
            Some(v): v
            None: 0

fn main():
    let a: A = A(n: None)
    print(a.chon())
"#,
        "5\n",
    );
}

// ===========================================================================
// 2 — `self.m() -> list[int?]`: `push` trên kết quả phương thức nâng `T → T?`
// ===========================================================================

#[test]
fn self_goi_phuong_thuc_tra_list_option() {
    thanh_cong(
        "self-list-opt",
        r#"type A:
    n: int

impl A:
    fn xs(self) -> list[int?]:
        [None]

    fn day(self) -> list[int?]:
        self.xs().push(self.n)

fn main():
    let a: A = A(n: 7)
    print(a.day())
"#,
        "[None, Some(7)]\n",
    );
}

// ===========================================================================
// 3 — phương thức trả **bản ghi có trường optional**: `with`, `?.`, `.f`
// ===========================================================================

#[test]
fn self_goi_phuong_thuc_tra_ban_ghi_option() {
    thanh_cong(
        "self-record-opt",
        r#"type B:
    f: int?
    n: int

type A:
    b: B

impl A:
    fn mk(self) -> B:
        B(f: None, n: 1)

    fn omk(self) -> B?:
        Some(B(f: None, n: 2))

    fn khong(self) -> B?:
        None

    fn demo(self) -> B:
        print(self.mk() with f: 9)
        print(self.omk()?.f)
        print(self.khong()?.f)
        print(self.mk().f)
        self.mk()

fn main():
    let a: A = A(b: B(f: None, n: 0))
    print(a.demo())
"#,
        "B(f: Some(9), n: 1)
Some(None)
None
None
B(f: None, n: 1)
",
    );
}

// ===========================================================================
// 4 — gán trường trên kết quả phương thức: dạng gần nhất được hỗ trợ là
//     `var b = recv.m(); b.f = …` (`recv.m().f = …` bị tyck từ chối — bản ghi bất biến)
// ===========================================================================

#[test]
fn gan_truong_tren_ket_qua_phuong_thuc() {
    thanh_cong(
        "assign-method-result",
        r#"type B:
    f: int?
    n: int

type A:
    b: B

impl A:
    fn mk(self) -> B:
        B(f: None, n: 1)

    fn gan(self) -> B:
        var b = self.mk()
        b.f = 7
        b

    fn gan_list(self) -> list[int?]:
        var xs = self.xs()
        xs[0] = 3
        xs

    fn xs(self) -> list[int?]:
        [None]

fn main():
    let a: A = A(b: B(f: None, n: 0))
    print(a.gan())
    print(a.gan_list())
"#,
        "B(f: Some(7), n: 1)
[Some(3)]
",
    );
}

// ===========================================================================
// 5 — `match` trên kết quả phương thức trả `int?` (bind arm có kiểu)
// ===========================================================================

#[test]
fn match_tren_ket_qua_phuong_thuc() {
    thanh_cong(
        "match-method",
        r#"type A:
    n: int?

impl A:
    fn lay(self) -> int?:
        self.n

    fn chon(self) -> int:
        match self.lay():
            Some(v): v
            None: 9

fn main():
    let a: A = A(n: None)
    print(a.chon())
    let b: A = A(n: Some(4))
    print(b.chon())
"#,
        "9\n4\n",
    );
}

// ===========================================================================
// 6 — nhận có annot (đường dispatch tĩnh cũ): giữ nguyên ngữ nghĩa — khoá hồi quy
// ===========================================================================

#[test]
fn nhan_co_annot_giu_nguyen_ngu_nghia() {
    thanh_cong(
        "annot-receiver",
        r#"type B:
    f: int?
    n: int

impl B:
    fn mk(self) -> B:
        B(f: None, n: self.n)

    fn omk(self) -> B?:
        Some(B(f: None, n: self.n))

    fn xs(self) -> list[int?]:
        [None]

fn main():
    let b: B = B(f: Some(1), n: 2)
    print(b.mk() with f: 5)
    print(b.omk()?.f)
    print(b.mk().f)
    print(b.xs().push(3))
    var q = b.mk()
    q.f = 4
    print(q)
"#,
        "B(f: Some(5), n: 2)
Some(None)
None
[None, Some(3)]
B(f: Some(4), n: 2)
",
    );
}
