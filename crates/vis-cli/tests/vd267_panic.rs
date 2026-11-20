// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267 — `panic(msg)`: builtin phân kỳ, kiểu tĩnh `never`.**
//!
//! RED BEFORE (main `b619c08`, trước vòng này): `panic(...)` **không** có trong bảng builtin.
//! `vis check` trên chương trình `let x = if true: 1 else: panic("boom")` in
//! `# VS0301 2:26 tên không tồn tại: `panic` chưa được khai báo (VD-260)` và thoát **1**.
//! Sau VD-267 chương trình đó **xanh**; `panic` chỉ còn đường **trap có chủ đích** `VS0501`
//! (exit **101**), không Rust panic/abort.
//!
//! Mỗi ca hợp lệ chạy `vis check` + **cả hai engine** (`--no-cache`, `--no-cache --vm`) và đòi
//! byte exit/stdout/stderr khớp nhau. Ca âm đòi `VS0401` (số đối số) hoặc `VS0412` (kiểu).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd267-{tag}-{}-{n}", std::process::id()));
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

/// Chạy interp + VM, đòi **byte** exit/stdout/stderr khớp nhau.
fn chay_ca_hai(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, b.0,
        "{tag}: exit interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(
        a.1, b.1,
        "{tag}: stdout interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(
        a.2, b.2,
        "{tag}: stderr interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    [a, b]
}

/// Ca **hợp lệ**: `vis check` sạch + hai engine chạy ra cùng stdout.
fn chay_duoc(tag: &str, src: &str, muon: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_eq!(
        c, 0,
        "{tag}: `vis check` phải xanh\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        !out.contains("VS0301") && !err.contains("VS0301"),
        "{tag}: `panic` không còn là tên chưa khai báo\nstdout: {out}\nstderr: {err}"
    );
    let [a, b] = chay_ca_hai(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, muon, "{name}: stdout sai\nstderr: {}", o.2);
    }
}

/// Ca **trap có chủ đích**: `vis check` xanh; cả hai engine exit **101**, stdout đúng, stderr chứa
/// thông điệp người dùng + `VS0501`, và **không** có dấu Rust panic.
fn chay_trap(tag: &str, src: &str, muon_stdout: &str, chua: &[&str]) {
    let (c, out, err) = check_src(tag, src);
    assert_eq!(
        c, 0,
        "{tag}: `vis check` phải xanh (trap là lỗi lúc chạy)\nstdout: {out}\nstderr: {err}"
    );
    let [a, b] = chay_ca_hai(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 101, "{name}: trap phải exit 101\nstderr: {}", o.2);
        assert_eq!(o.1, muon_stdout, "{name}: stdout sai\nstderr: {}", o.2);
        for s in chua {
            assert!(
                o.2.contains(s),
                "{name}: stderr thiếu `{s}`\nstderr: {}",
                o.2
            );
        }
        assert!(
            !o.2.contains("panicked at"),
            "{name}: không được có Rust panic\nstderr: {}",
            o.2
        );
    }
}

/// Ca **bị từ chối tĩnh**: `vis check` đỏ + đúng mã; cả hai engine cũng từ chối (exit 2).
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

// ===========================================================================
// 1 — vị trí biểu thức, kiểu `never` và `return panic`
// ===========================================================================

/// Nhánh `else` của `if` biểu thức có `panic` ⇒ cả `if` vẫn là `int`; `let y: int` nhận được.
/// Hàm chỉ `return panic(...)` cũng hợp lệ tĩnh (không gọi trong ca này).
#[test]
fn bieu_thuc_else_va_return_panic() {
    chay_duoc(
        "expr-else",
        r#"fn stop():
    panic("không bao giờ chạy")

fn f() -> int:
    return panic("không bao giờ chạy")

fn g(b: bool) -> int:
    if b: return 1
    else: return panic("không bao giờ chạy")

fn main():
    let y: int = if true: 1 else: panic("boom")
    print(y)
    print(g(true))
"#,
        "1\n1\n",
    );
}

/// `panic` là **biểu thức** ở mọi vị trí: đối số gọi hàm, phần tử list, giá trị map.
#[test]
fn panic_trong_doi_so_va_collection() {
    chay_duoc(
        "expr-args",
        r#"fn pick(n: int) -> int:
    n

fn main():
    print(pick(if true: 1 else: panic("boom")))
    let xs = [if true: 2 else: panic("boom")]
    print(xs[0])
    let m = {"k": if true: 3 else: panic("boom")}
    print(m["k"])
"#,
        "1\n2\n3\n",
    );
}

/// `panic` dùng như **giá trị hàm** (`let f = panic; f("…")`) — cả hai engine cùng trap.
#[test]
fn panic_la_gia_tri_goi_duoc() {
    chay_trap(
        "value",
        r#"fn main():
    let f = panic
    f("qua giá trị")
"#,
        "",
        &["VS0501", "qua giá trị"],
    );
}

// ===========================================================================
// 2 — trap lúc chạy: exit 101, thông điệp, không chạy câu sau
// ===========================================================================

/// `panic("hỏng")` ở vị trí câu lệnh: in dòng trước, **không** in dòng sau; exit 101; stderr có
/// thông điệp gốc.
#[test]
fn panic_dung_chuong_trinh_va_khong_chay_sau() {
    chay_trap(
        "stmt",
        r#"fn main():
    print("truoc")
    panic("hỏng")
    print("sau")
"#,
        "truoc\n",
        &["VS0501", "hỏng"],
    );
}

/// Nhánh `else` chạy `panic` ⇒ trap; nhánh `then` không chạm `panic` vẫn chạy bình thường.
#[test]
fn panic_trong_nhanh_else() {
    chay_trap(
        "else-trap",
        r#"fn f(b: bool) -> int:
    if b: return 1
    else: return panic("bad")

fn main():
    print(f(false))
"#,
        "",
        &["VS0501", "bad"],
    );
}

/// Payload `Error` (thiết kế §6.4): thông điệp lấy từ trường `message`, không in cả bản ghi.
#[test]
fn panic_nhan_ban_ghi_error() {
    chay_trap(
        "error",
        r#"fn main():
    let e: Error = {message: "lỗi thật", source: None, span: None}
    panic(e)
"#,
        "",
        &["VS0501", "lỗi thật"],
    );
}

// ===========================================================================
// 3 — ca âm: sai số đối số (VS0401) / sai kiểu (VS0412)
// ===========================================================================

#[test]
fn panic_sai_so_doi_so() {
    tu_choi("arity-0", "panic()\n", "VS0401");
    tu_choi("arity-2", "panic(\"a\", \"b\")\n", "VS0401");
}

#[test]
fn panic_sai_kieu_doi_so() {
    tu_choi("type-int", "panic(1)\n", "VS0412");
    tu_choi("type-bool", "panic(true)\n", "VS0412");
}
