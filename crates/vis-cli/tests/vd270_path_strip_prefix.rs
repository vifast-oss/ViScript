// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.strip_prefix)** — `std.path.strip_prefix(p: str, base: str) -> str?`, bỏ
//! **tiền tố theo thành phần** đường dẫn, **thuần từ vựng POSIX** (không chạm đĩa/`cwd`/symlink/
//! tồn tại, không chuẩn hoá `.`/`..`).
//!
//! Hợp đồng được khoá ở đây:
//! - `base` rỗng ⇒ `Some(p)`; `p == base` (sau khi bỏ `/` cuối của `base`) ⇒ `Some("")`.
//! - `base` khớp **trọn thành phần** của `p` ⇒ `Some(phần còn lại)`, bỏ **đúng một** `/` phân cách
//!   đầu; phần còn lại giữ **nguyên văn** (kể cả `.`/`..`/`//` bên trong): `a//b` bỏ `a` ⇒
//!   `Some("/b")` (khác Rust `b`), `a/./b` bỏ `a` ⇒ `Some("./b")` (khác Rust `b`).
//! - `base` kết thúc bằng `/`: bỏ **mọi** `/` cuối trước khi so (trừ khi `base` toàn `/`). Vì vậy
//!   `a/b` bỏ `a/`, `a//`, `a///` đều ⇒ `Some("b")`.
//! - `base` toàn `/` ⇒ **gốc**: `p` tuyệt đối ⇒ `Some(p[1..])` (bỏ **đúng một** `/` đầu;
//!   `p="/"` ⇒ `Some("")`, `p="//a"` ⇒ `Some("/a")` — khác Rust `a`); `p` tương đối ⇒ `None`.
//! - Tuyệt đối vs tương đối khác loại ⇒ `None` (trừ `base` rỗng); `p` ngắn hơn/khác thành phần ⇒
//!   `None`. So **BINARY** từng byte: phân biệt NFC/NFD; `\` và NUL là ký tự **thường**.
//! - Không có lỗi dữ liệu/môi trường ⇒ **không trap**.
//!
//! **Không sugar** (module-only): tên `path.strip_prefix` **không** nằm trong bảng `module_sugar`.
//! Lưu ý: `str.strip_prefix` **đã có** từ trước, nên `"a//b".strip_prefix("a")` vẫn là **str**
//! sugar (`Some("//b")`) — test dưới chứng minh nó **không** đi qua `path.strip_prefix`
//! (`Some("/b")`); receiver không phải `str` (vd `int`) ⇒ `VS0403`. Sai kiểu ⇒ `VS0412`, sai arity
//! ⇒ `VS0401`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `f00bc50`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.path
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.path` không có hàm `strip_prefix`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.path` không có hàm `strip_prefix`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng dòng)
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-sp-{tag}-{}-{n}", std::process::id()));
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
    c.arg("run").arg("--no-cache").args(extra).arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM; assert hai engine giống **byte** stdout/stderr/exit.
fn hai_engine(tag: &str, src: &str) -> (i32, String, String) {
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
    let (code, out, err) = hai_engine(tag, src);
    assert_eq!(
        code, 0,
        "{tag}: phải chạy sạch\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.is_empty(), "{tag}: stderr phải rỗng: {err}");
    assert_eq!(out, stdout, "{tag}: stdout sai");
}

/// Lỗi **tĩnh**: `vis check` đỏ (exit 1, mã trong TAP) và **cả hai** engine từ chối (exit 2).
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (d, p) = ghi_tep(tag, src);
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let co = String::from_utf8_lossy(&o.stdout).to_string();
    let ce = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(
        o.status.code().unwrap_or(-1),
        1,
        "{tag}: `check` phải đỏ (exit 1)\nstdout: {co}\nstderr: {ce}"
    );
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
// 0 — probe Rust `Path::strip_prefix` (toolchain ghim 1.100.0)
// ===========================================================================

/// Bảng tham chiếu `Path::strip_prefix` của Rust. Đây là **probe**, không gọi ViScript.
///
/// Các ca ViScript **cố ý khác** Rust (xem bảng hợp đồng ở đầu tệp):
/// - Rust chuẩn hoá phần còn lại (`a//b` bỏ `a` ⇒ `b`, `a/./b` bỏ `a` ⇒ `b`); ViScript giữ
///   **nguyên văn** và chỉ bỏ **một** `/` đầu (`/b`, `./b`).
/// - `base` rỗng: Rust trả `p` đã đi qua `Components` (`a/` ⇒ `a`); ViScript trả `p` **nguyên văn**.
/// - `/` đầu dư: Rust bỏ **mọi** `/` đầu của phần còn lại (`//a` bỏ `/` ⇒ `a`); ViScript bỏ đúng
///   **một** (`/a`).
fn khop_rust(p: &str, base: &str, mong: Result<&str, ()>) {
    let thuc = Path::new(p)
        .strip_prefix(base)
        .map(|r| r.to_str().expect("probe: đường dẫn UTF-8"))
        .map_err(|_| ());
    assert_eq!(thuc, mong, "Rust Path::strip_prefix({p:?}, {base:?})");
}

#[test]
fn probe_rust_path_strip_prefix() {
    // `p`, `base`, kết quả Rust.
    let bang: &[(&str, &str, Result<&str, ()>)] = &[
        ("", "", Ok("")),
        ("a", "", Ok("a")),
        ("/a", "", Ok("/a")),
        ("a/", "", Ok("a")),
        ("//a", "", Ok("//a")),
        ("/", "", Ok("/")),
        ("a", "a", Ok("")),
        ("a/", "a", Ok("")),
        ("a//", "a", Ok("")),
        ("a/b", "a", Ok("b")),
        ("a/b/", "a", Ok("b")),
        ("a/b/c", "a/b", Ok("c")),
        ("a/b/c/", "a/b", Ok("c")),
        ("a/b", "a/", Ok("b")),
        ("a/b", "a//", Ok("b")),
        ("a/b", "a///", Ok("b")),
        ("a//b", "a", Ok("b")),
        ("a//b/", "a", Ok("b")),
        ("a//b", "a//", Ok("b")),
        ("a//b", "a/", Ok("b")),
        ("a//b", "a//b", Ok("")),
        ("a/b", "a//b", Ok("")),
        ("a/./b", "a", Ok("b")),
        ("a/./b", "a/.", Ok("b")),
        ("a/../b", "a", Ok("../b")),
        ("a/../b", "a/..", Ok("b")),
        ("a/..", "a", Ok("..")),
        ("a/.", "a", Ok("")),
        ("./a", ".", Ok("a")),
        ("./a", "./", Ok("a")),
        ("a/b", ".", Err(())),
        ("", ".", Err(())),
        (".", "", Ok(".")),
        (".", ".", Ok("")),
        ("./", ".", Ok("")),
        ("", "/", Err(())),
        ("/", "/", Ok("")),
        ("/a", "/", Ok("a")),
        ("/a/b", "/", Ok("a/b")),
        ("//a", "/", Ok("a")),
        ("///a", "//", Ok("a")),
        ("/a", "//", Ok("a")),
        ("/a/b", "/a", Ok("b")),
        ("/a//b", "/a", Ok("b")),
        ("/a/./b", "/a/.", Ok("b")),
        ("/a/../b", "/a", Ok("../b")),
        ("/a/b", "/a/b/c", Err(())),
        ("a", "a/b", Err(())),
        ("a", "/a", Err(())),
        ("/a", "a", Err(())),
        ("a/b", "/a/b", Err(())),
        ("đ😀/x", "đ😀", Ok("x")),
        ("đ😀/x", "đ", Err(())),
        ("e\u{0301}/x", "e\u{0300}", Err(())),
        (r"a\b/c", r"a\b", Ok("c")),
        ("a\0b/c", "a\0b", Ok("c")),
        ("a/./b/./c", "a/./b", Ok("c")),
        ("a//./b", "a", Ok("b")),
        ("//srv/share", "//", Ok("srv/share")),
        ("//srv/share", "/", Ok("srv/share")),
        ("///", "//", Ok("")),
    ];
    for &(p, base, mong) in bang {
        khop_rust(p, base, mong);
    }
}

// ===========================================================================
// 1 — base rỗng và p == base
// ===========================================================================

/// `base` rỗng ⇒ `Some(p)` nguyên văn; bằng nhau (kể cả `/` cuối của `base`) ⇒ `Some("")`.
#[test]
fn rong_va_bang_nhau() {
    dung(
        "sp-rong",
        r#"use std.path

fn main():
    print(path.strip_prefix("a/b", ""))
    print(path.strip_prefix("", ""))
    print(path.strip_prefix("/a", ""))
    print(path.strip_prefix("a/b", "a/b"))
    print(path.strip_prefix("a/b/", "a/b/"))
    print(path.strip_prefix("a//", "a//"))
    print(path.strip_prefix("a//b//", "a//b//"))
    print(path.strip_prefix("//", "//"))
    print(path.strip_prefix("/a//", "/a//"))
    print(path.strip_prefix("/", "/"))
    print(path.strip_prefix("a", "a/"))
    print(path.strip_prefix("a/", "a"))
    print(path.strip_prefix("a//", "a"))
"#,
        "Some(a/b)\nSome()\nSome(/a)\nSome()\nSome()\nSome()\nSome()\nSome()\nSome()\nSome()\nSome()\nSome()\nSome(/)\n",
    );
}

// ===========================================================================
// 2 — biên thành phần: khớp trọn thành phần, không khớp tiền tố byte
// ===========================================================================

/// `a` khớp `a/b` nhưng `ab`/`abc` **không** khớp `a`; `p` ngắn hơn/dài hơn sai thành phần ⇒
/// `None`; bỏ `/` cuối của `base` trước khi so.
#[test]
fn bien_thanh_phan() {
    dung(
        "sp-bien",
        r#"use std.path

fn main():
    print(path.strip_prefix("a/b/c", "a"))
    print(path.strip_prefix("a/b/c", "a/b"))
    print(path.strip_prefix("a/b/c", "a/b/c"))
    print(path.strip_prefix("a/b/c", "a/bb"))
    print(path.strip_prefix("a/b", "a/b/c"))
    print(path.strip_prefix("abc", "ab"))
    print(path.strip_prefix("ab/c", "a"))
    print(path.strip_prefix("a/bc", "a/b"))
    print(path.strip_prefix("a/b/c", "a/b/"))
    print(path.strip_prefix("a/b/c", "a/b/c/"))
    print(path.strip_prefix("a/b", "a/b//"))
"#,
        "Some(b/c)\nSome(c)\nSome()\nNone\nNone\nNone\nNone\nNone\nSome(c)\nSome()\nSome()\n",
    );
}

// ===========================================================================
// 3 — `/` cuối của `base` bị bỏ trước khi so
// ===========================================================================

/// Mọi `/` cuối của `base` bị bỏ (trừ `base` toàn `/`); `base` có `.` vẫn so **nguyên văn**.
#[test]
fn slash_cuoi_cua_base() {
    dung(
        "sp-slash-cuoi",
        r#"use std.path

fn main():
    print(path.strip_prefix("a/b", "a/"))
    print(path.strip_prefix("a/b", "a//"))
    print(path.strip_prefix("a/b", "a///"))
    print(path.strip_prefix("a/b", "a/b/"))
    print(path.strip_prefix("a/b", "a/b//"))
    print(path.strip_prefix("a/./b", "a/./"))
    print(path.strip_prefix("a/./b", "a//./"))
    print(path.strip_prefix("/a/b", "/a/"))
    print(path.strip_prefix("/a/b", "/a//"))
    print(path.strip_prefix("/a/b", "///a//"))
    print(path.strip_prefix("a/b", "a/./"))
"#,
        "Some(b)\nSome(b)\nSome(b)\nSome()\nSome()\nSome(b)\nSome(b)\nSome(b)\nSome(b)\nSome(b)\nNone\n",
    );
}

// ===========================================================================
// 4 — `base` toàn `/` = gốc; bỏ đúng **một** `/` đầu của `p`
// ===========================================================================

/// `base="/"` (và mọi `base` toàn `/`) khớp **gốc**: bỏ đúng một `/` đầu của `p` tuyệt đối,
/// giữ nguyên các `/` còn lại; `p` tương đối ⇒ `None`.
#[test]
fn goc_va_nhieu_slash_dau() {
    dung(
        "sp-goc",
        r#"use std.path

fn main():
    print(path.strip_prefix("/", "/"))
    print(path.strip_prefix("/a", "/"))
    print(path.strip_prefix("/a/b", "/"))
    print(path.strip_prefix("//a", "/"))
    print(path.strip_prefix("///a", "//"))
    print(path.strip_prefix("//srv/share", "//"))
    print(path.strip_prefix("/a/b", "///"))
    print(path.strip_prefix("//", "/"))
    print(path.strip_prefix("a", "/"))
    print(path.strip_prefix("", "/"))
    print(path.strip_prefix("a/b", "//"))
"#,
        "Some()\nSome(a)\nSome(a/b)\nSome(/a)\nSome(//a)\nSome(/srv/share)\nSome(a/b)\nSome(/)\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 5 — tuyệt đối vs tương đối khác loại ⇒ None (trừ base rỗng)
// ===========================================================================

/// Khác loại ⇒ `None` **kể cả** khi chuỗi byte trùng; `base` rỗng là ngoại lệ duy nhất.
#[test]
fn tuyet_doi_vs_tuong_doi() {
    dung(
        "sp-tuyet-doi",
        r#"use std.path

fn main():
    print(path.strip_prefix("/a", "a"))
    print(path.strip_prefix("a", "/a"))
    print(path.strip_prefix("/a/b", "a/b"))
    print(path.strip_prefix("a/b", "/a/b"))
    print(path.strip_prefix("", "/"))
    print(path.strip_prefix("", "a"))
    print(path.strip_prefix("//a", "a"))
    print(path.strip_prefix("/", ""))
    print(path.strip_prefix("a", ""))
    print(path.strip_prefix("", ""))
"#,
        "None\nNone\nNone\nNone\nNone\nNone\nNone\nSome(/)\nSome(a)\nSome()\n",
    );
}

// ===========================================================================
// 6 — `.`/`..` là thành phần **thường**, giữ nguyên văn
// ===========================================================================

/// Không chuẩn hoá: khớp nguyên văn từng thành phần `.`/`..`; phần còn lại giữ `.`/`..`.
#[test]
fn cham_cham_va_cham_thuong() {
    dung(
        "sp-cham",
        r#"use std.path

fn main():
    print(path.strip_prefix("a/../b", "a"))
    print(path.strip_prefix("a/../b", "a/.."))
    print(path.strip_prefix("a/..", "a"))
    print(path.strip_prefix("../a", ".."))
    print(path.strip_prefix("../../a/b", "../../a"))
    print(path.strip_prefix("a/./b", "a"))
    print(path.strip_prefix("a/./b", "a/."))
    print(path.strip_prefix("a/./b/./c", "a/./b"))
    print(path.strip_prefix("a/..", "a/../"))
    print(path.strip_prefix("./a", "."))
    print(path.strip_prefix("./a", "./"))
    print(path.strip_prefix(".", "."))
    print(path.strip_prefix("./", "."))
    print(path.strip_prefix("a/.", "a"))
    print(path.strip_prefix("a/b", "."))
    print(path.strip_prefix("", "."))
"#,
        "Some(../b)\nSome(b)\nSome(..)\nSome(a)\nSome(b)\nSome(./b)\nSome(b)\nSome(./c)\nSome()\nSome(a)\nSome(a)\nSome()\nSome()\nSome(.)\nNone\nNone\n",
    );
}

// ===========================================================================
// 7 — `//` trong `p`: chỉ bỏ **một** `/` phân cách đầu
// ===========================================================================

/// Khớp theo **thành phần** (nhiều `/` liền nhau không tạo thành phần rỗng) nhưng phần còn lại
/// **nguyên văn**, chỉ bỏ đúng một `/`; khác Rust (Rust gộp hết `/` và chuẩn hoá `.`).
#[test]
fn hai_slash_trong_p() {
    dung(
        "sp-hai-slash",
        r#"use std.path

fn main():
    print(path.strip_prefix("a//b", "a"))
    print(path.strip_prefix("a//b", "a/"))
    print(path.strip_prefix("a//b", "a//"))
    print(path.strip_prefix("a//b", "a//b"))
    print(path.strip_prefix("a/b", "a//b"))
    print(path.strip_prefix("a//b//c", "a/b"))
    print(path.strip_prefix("a//b", "a//b//"))
    print(path.strip_prefix("/a//b", "/a"))
    print(path.strip_prefix("/a//b", "/a//"))
    print(path.strip_prefix("a//b//", "a//b"))
    print(path.strip_prefix("a//./b", "a"))
"#,
        "Some(/b)\nSome(/b)\nSome(/b)\nSome()\nSome()\nSome(/c)\nSome()\nSome(/b)\nSome(/b)\nSome(/)\nSome(/./b)\n",
    );
}

// ===========================================================================
// 8 — Unicode/NFD, `\` và NUL là ký tự thường; so BINARY
// ===========================================================================

/// Thành phần Unicode giữ nguyên từng scalar; NFC ≠ NFD; `\`/NUL **không** phải phân cách và
/// nằm trong thành phần bình thường.
#[test]
fn unicode_nfd_va_ky_tu_thuong() {
    dung(
        "sp-unicode",
        r#"use std.path

fn main():
    print(path.strip_prefix("thư/mục/tệp.txt", "thư"))
    print(path.strip_prefix("thư/mục/tệp.txt", "thư/mục"))
    print(path.strip_prefix("đ😀/x", "đ😀"))
    print(path.strip_prefix("đ😀/x", "đ"))
    print(path.strip_prefix("😀/x", "😀"))
    print(path.strip_prefix("e\u{0302}\u{0301}/x", "e\u{0302}\u{0301}"))
    print(path.strip_prefix("e\u{0301}/x", "e\u{0300}"))
    print(path.strip_prefix("e\u{0301}/x", "e"))
    print(path.strip_prefix("a\\b/c", "a\\b"))
    print(path.strip_prefix("a\\b/c", "a"))
    print(path.strip_prefix("a\0b/c", "a\0b"))
    print(path.strip_prefix("a\0b/c", "a"))
    print(path.strip_prefix("/a\\b/c", "/a\\b"))
"#,
        "Some(mục/tệp.txt)\nSome(tệp.txt)\nSome(x)\nNone\nSome(x)\nSome(x)\nNone\nNone\nSome(c)\nNone\nSome(c)\nNone\nSome(c)\n",
    );
}

// ===========================================================================
// 9 — `str?` là `Option[str]` thật
// ===========================================================================

/// `match Some/None` chạy được; `let x: str?` hợp lệ; in `Some()` cho phần còn lại rỗng.
#[test]
fn ket_qua_la_option_str_that() {
    dung(
        "sp-option",
        r#"use std.path

fn main():
    match path.strip_prefix("a/b", "a"):
        Some(r): print(r)
        None: print("none")
    match path.strip_prefix("a/b", "x"):
        Some(_): print("some")
        None: print("none")
    let a: str? = path.strip_prefix("a/b", "a/b")
    let b: str? = path.strip_prefix("a/b", "x")
    print(a)
    print(b)
"#,
        "b\nnone\nSome()\nNone\n",
    );
}

// ===========================================================================
// 10 — không sugar **path**: tên trùng `str.strip_prefix`
// ===========================================================================

/// `path.strip_prefix` không nằm trong bảng `module_sugar`; nhưng vì `str.strip_prefix` **đã có**,
/// `"a//b".strip_prefix("a")` là **str** sugar (`Some("//b")`), **không** phải path
/// (`Some("/b")`). Receiver không phải `str` ⇒ `VS0403` (không có trường).
#[test]
fn khong_sugar_path_do_trung_ten_str() {
    dung(
        "sp-sugar-path",
        r#"use std.path

fn main():
    let p: str = "a//b"
    print(p.strip_prefix("a"))
    print(path.strip_prefix(p, "a"))
    print("a/./b".strip_prefix("a"))
    print(path.strip_prefix("a/./b", "a"))
"#,
        "Some(//b)\nSome(/b)\nSome(/./b)\nSome(./b)\n",
    );
    tu_choi(
        "sp-sugar-int",
        "fn main():\n    let x = 1.strip_prefix(\"a\")\n    print(x)\n",
        "VS0403",
    );
}

// ===========================================================================
// 11 — ca âm tĩnh: sai kiểu / sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 0/1/3 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("sp-kieu-p", "path.strip_prefix(1, \"a\")", "VS0412"),
        ("sp-kieu-base", "path.strip_prefix(\"a\", 1)", "VS0412"),
        ("sp-kieu-bool", "path.strip_prefix(true, \"a\")", "VS0412"),
        ("sp-arity-0", "path.strip_prefix()", "VS0401"),
        ("sp-arity-1", "path.strip_prefix(\"a\")", "VS0401"),
        (
            "sp-arity-3",
            "path.strip_prefix(\"a\", \"b\", \"c\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Gán `str?` vào `str` bị chặn; gán vào `str?` hợp lệ (ca dương ở test 9).
#[test]
fn ket_qua_sang_str_bi_chan() {
    tu_choi(
        "sp-ket-qua-sang-str",
        "use std.path\n\nfn main():\n    let x: str = path.strip_prefix(\"a/b\", \"a\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 12 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp rỗng/bằng nhau/biên/trailing slash/gốc/`..`/`//`/Unicode/`\`/NUL vào một chương trình để
/// khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "sp-hai-engine",
        r#"use std.path

fn main():
    print(path.strip_prefix("a/b/c", ""))
    print(path.strip_prefix("a/b/c", "a/b/c"))
    print(path.strip_prefix("a/b/c", "a"))
    print(path.strip_prefix("a/b/c", "a/b"))
    print(path.strip_prefix("a/b", "a/"))
    print(path.strip_prefix("/", "/"))
    print(path.strip_prefix("//a", "/"))
    print(path.strip_prefix("a", "/a"))
    print(path.strip_prefix("a/../b", "a"))
    print(path.strip_prefix("a//b", "a"))
    print(path.strip_prefix("đ😀/x", "đ😀"))
    print(path.strip_prefix("e\u{0301}/x", "e\u{0300}"))
    print(path.strip_prefix("a\0b/c", "a\0b"))
    print(path.strip_prefix("a\\b/c", "a\\b"))
    match path.strip_prefix("a/b", "a"):
        Some(r): print(r)
        None: print("none")
"#,
        "Some(a/b/c)\nSome()\nSome(b/c)\nSome(c)\nSome(b)\nSome()\nSome(/a)\nNone\nSome(../b)\nSome(/b)\nSome(x)\nNone\nSome(c)\nSome(c)\nb\n",
    );
}
