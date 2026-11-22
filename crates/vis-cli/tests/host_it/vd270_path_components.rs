// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.components)** — `std.path.components(p: str) -> list[str]`, tách đường dẫn
//! **POSIX** thành **thành phần từ vựng** — **thuần chuỗi**: không chạm đĩa, không đọc `cwd`,
//! không phân giải symlink, không kiểm tra tồn tại, **không chuẩn hoá**.
//!
//! Hợp đồng được khoá ở đây:
//!
//! | đầu vào | kết quả | ghi chú |
//! |---|---|---|
//! | `""` | `[]` | không có thành phần nào |
//! | `"/"`, `"//"`, `"///"`, `"////"` | `["/"]` | chỉ toàn `/` tuyệt đối ⇒ gốc |
//! | `"a//b/"`, `"a///b//c///"` | `["a","b"]`, `["a","b","c"]` | bỏ thành phần rỗng giữa/cuối |
//! | `"a/"` | `["a"]` | `/` cuối không sinh thành phần rỗng |
//! | `"/a//b"`, `"///a"` | `["/","a","b"]`, `["/","a"]` | tuyệt đối ⇒ `"/"` đầu, gộp `/` đầu |
//! | `"//srv/share"` | `["/","srv","share"]` | hai `/` đầu gộp thành root `/` |
//! | `"."`, `".."` | `["."]`, `[".."]` | giữ **nguyên văn**, không chuẩn hoá |
//! | `"./a"`, `"a/./b"` | `[".","a"]`, `["a",".","b"]` | `.` là thành phần **thường** |
//! | `"a/../b"`, `"a/.."` | `["a","..","b"]`, `["a",".."]` | `..` **không** phân giải |
//! | `"a/../.."`, `"../a"` | `["a","..",".."]`, `["..","a"]` | giữ đủ số `..` |
//! | `"/./a"`, `"/../a"`, `"//../x"` | `["/",".","a"]`, `["/","..","a"]`, `["/","..","x"]` | gốc **không** "ăn" `..` |
//! | `"thư/mục/tệp.txt"` | `["thư","mục","tệp.txt"]` | Unicode giữ **từng scalar** (không NFC/NFD) |
//! | `"a\\b"`, `"a\0b"` | `["a\\b"]`, `["a\0b"]` | `\` và NUL là ký tự **thường**, không phân cách |
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `f1755e0`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_path_components
//! test result: FAILED. 1 passed; 8 failed
//! ```
//!
//! - **8 ca** đỏ `VS0301`: `vis check` báo `module `std.path` không có hàm `components``;
//!   `vis run` và `vis run --vm` đều exit 2 với đúng dòng lỗi cho **từng** lời gọi.
//! - **1 ca no-sugar** đã xanh đúng `VS0403` `` `str` không có trường `components` `` — không
//!   phụ thuộc hàm mới (hành vi khoá).
//!
//! Sau khi thêm: 9/9 xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pc-{tag}-{}-{n}", std::process::id()));
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
// 1 — rỗng ⇒ [] và gốc tuyệt đối ⇒ ["/"]
// ===========================================================================

/// `""` không có thành phần; `"/"`, `"//"`, `"///"`, `"////"` đều chỉ là gốc `["/"]`.
#[test]
fn rong_va_goc_tuyet_doi() {
    dung(
        "pc-rong-goc",
        r#"use std.path

fn main():
    let rong = path.components("")
    print(rong.len())
    print(rong)
    print(path.components("/"))
    print(path.components("//"))
    print(path.components("///"))
    print(path.components("////"))
"#,
        "0\n[]\n[/]\n[/]\n[/]\n[/]\n",
    );
}

// ===========================================================================
// 2 — `/` lặp, `/` cuối, tuyệt đối: bỏ thành phần rỗng, `"/"` đầu
// ===========================================================================

/// `"a//b/"` ⇒ `["a","b"]`; `"/a//b"` ⇒ `["/","a","b"]`; `"///a"` ⇒ `["/","a"]`;
/// `"//srv/share"` ⇒ `["/","srv","share"]` (hai `/` đầu gộp thành root duy nhất).
#[test]
fn slash_lap_va_tuyet_doi() {
    dung(
        "pc-slash",
        r#"use std.path

fn main():
    print(path.components("a"))
    print(path.components("a/b"))
    print(path.components("a//b/"))
    print(path.components("a/"))
    print(path.components("a///b//c///"))
    print(path.components("/a//b"))
    print(path.components("///a"))
    print(path.components("//srv/share"))
    print(path.components("/a/b/"))
"#,
        "[a]\n[a, b]\n[a, b]\n[a]\n[a, b, c]\n[/, a, b]\n[/, a]\n[/, srv, share]\n[/, a, b]\n",
    );
}

// ===========================================================================
// 3 — `.`/`..` giữ **nguyên văn**, không chuẩn hoá, gốc không "ăn" `..`
// ===========================================================================

/// `"./a"` ⇒ `[".","a"]`; `"a/../b"` ⇒ `["a","..","b"]`; `"a/../.."` ⇒ `["a","..",".."]`;
/// `"a/.."` ⇒ `["a",".."]` (khác `path.normalize`); `/..` ở gốc vẫn là thành phần `..`.
#[test]
fn cham_va_cham_cham_nguyen_van() {
    dung(
        "pc-cham",
        r#"use std.path

fn main():
    print(path.components("."))
    print(path.components(".."))
    print(path.components("./"))
    print(path.components("../"))
    print(path.components("./a"))
    print(path.components("../a"))
    print(path.components("a/./b"))
    print(path.components("a/../b"))
    print(path.components("a/.."))
    print(path.components("a/../.."))
    print(path.components("/./a"))
    print(path.components("/../a"))
    print(path.components("//../x"))
"#,
        "[.]\n[..]\n[.]\n[..]\n[., a]\n[.., a]\n[a, ., b]\n[a, .., b]\n[a, ..]\n[a, .., ..]\n[/, ., a]\n[/, .., a]\n[/, .., x]\n",
    );
}

// ===========================================================================
// 4 — Unicode giữ **từng scalar** (không NFC/NFD, emoji astral)
// ===========================================================================

/// Tên tiếng Việt/emoji/tổ hợp NFD được tách theo `/` nhưng **không** đổi chuẩn hoá.
#[test]
fn unicode_giu_nguyen_scalar() {
    dung(
        "pc-unicode",
        r#"use std.path

fn main():
    print(path.components("thư/mục/tệp.txt"))
    print(path.components("đ😀/x"))
    print(path.components("/đ😀//x/"))
    print(path.components("e\u{0302}\u{0301}/x"))
"#,
        "[thư, mục, tệp.txt]\n[đ😀, x]\n[/, đ😀, x]\n[e\u{0302}\u{0301}, x]\n",
    );
}

// ===========================================================================
// 5 — `\` và NUL là ký tự **thường**, không phải phân cách
// ===========================================================================

/// `"a\\b"` là **một** thành phần; NUL nằm trong thành phần và không bị cắt.
#[test]
fn backslash_va_nul_la_ky_tu_thuong() {
    dung(
        "pc-thuong",
        r#"use std.path

fn main():
    print(path.components("a\\b"))
    print(path.components("\\a\\b/c"))
    print(path.components("a\0b"))
    print(path.components("/a\0b/c"))
"#,
        "[a\\b]\n[\\a\\b, c]\n[a\0b]\n[/, a\0b, c]\n",
    );
}

// ===========================================================================
// 6 — kết quả là `list[str]` thật: gán, len, chỉ số, duyệt
// ===========================================================================

/// Kiểu trả `list[str]` qua biên gán; `len`/chỉ số/`for` đều dùng được — chứng minh không
/// phải chuỗi hay bản ghi.
#[test]
fn ket_qua_la_list_str_that() {
    dung(
        "pc-list-str",
        r#"use std.path

fn main():
    let xs: list[str] = path.components("a/b/c")
    print(xs.len())
    print(xs[0])
    print(xs[2])
    for x in xs:
        print(x)
    let rong: list[str] = path.components("")
    print(rong.len())
"#,
        "3\na\nc\na\nb\nc\n0\n",
    );
}

// ===========================================================================
// 7 — **không** sugar: module-only
// ===========================================================================

/// `p.components()` **không** phải sugar: `str` không có trường `components` ⇒ `VS0403`,
/// kể cả khi đã `use std.path`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pc-sugar-literal",
            "fn main():\n    print(\"a/b\".components())\n",
        ),
        (
            "pc-sugar-bien",
            "fn main():\n    let p = \"a/b\"\n    print(p.components())\n",
        ),
        (
            "pc-sugar-co-use",
            "use std.path\n\nfn main():\n    print(\"a/b\".components())\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 8 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 0/2 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("pc-kieu-int", "path.components(1)", "VS0412"),
        ("pc-kieu-bool", "path.components(true)", "VS0412"),
        ("pc-arity-0", "path.components()", "VS0401"),
        ("pc-arity-2", "path.components(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 9 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp rỗng/gốc/`/` lặp/`.`/`..`/tuyệt đối/Unicode/NUL để khoá **byte** stdout/stderr/exit
/// giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pc-hai-engine",
        r#"use std.path

fn main():
    print(path.components(""))
    print(path.components("/"))
    print(path.components("//"))
    print(path.components("///"))
    print(path.components("a//b/"))
    print(path.components("/a//b"))
    print(path.components("///a"))
    print(path.components("//srv/share"))
    print(path.components("a/./b"))
    print(path.components("a/../.."))
    print(path.components("a/.."))
    print(path.components("./a"))
    print(path.components("../a"))
    print(path.components("thư/mục/tệp.txt"))
    print(path.components("đ😀/x"))
    print(path.components("e\u{0302}\u{0301}/x"))
    print(path.components("a\\b"))
    print(path.components("a\0b"))
    let xs: list[str] = path.components("/a/b/")
    print(xs.len())
"#,
        "[]\n[/]\n[/]\n[/]\n[a, b]\n[/, a, b]\n[/, a]\n[/, srv, share]\n[a, ., b]\n[a, .., ..]\n[a, ..]\n[., a]\n[.., a]\n[thư, mục, tệp.txt]\n[đ😀, x]\n[e\u{0302}\u{0301}, x]\n[a\\b]\n[a\0b]\n3\n",
    );
}
