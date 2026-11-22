// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.ancestors)** — `std.path.ancestors(p: str) -> list[str]`, chuỗi **tổ tiên
//! từ vựng POSIX** của `p`, **gồm chính `p`** — **thuần chuỗi**: không chạm đĩa, không đọc
//! `cwd`, không phân giải symlink, không kiểm tra tồn tại, **không chuẩn hoá** `.`/`..`.
//!
//! Hợp đồng được khoá ở đây:
//!
//! 1. Chuẩn hoá **chỉ** `/` cuối, trừ gốc: bỏ **mọi** `/` cuối khi phần còn lại **không rỗng**;
//!    `"/"`, `"//"`, `"///"`, `"////"` giữ `"/"`. Phần tử **đầu** = `p` đã bỏ `/` cuối.
//! 2. Lặp: bỏ **thành phần cuối** (đoạn sau `/` cuối) **kể cả** dấu `/` phân cách; phần còn
//!    lại bỏ `/` cuối (trừ gốc); dừng khi `cur == "/"` hoặc `cur` **không còn** `/`; quẫn
//!    cùng **không** có `/` ⇒ thêm `""` (trừ khi chính `p` rỗng — khi đó chỉ `[""]`).
//!
//! | đầu vào | kết quả |
//! |---|---|
//! | `""` | `[""]` (đúng **một** phần tử rỗng) |
//! | `"/"`, `"//"`, `"///"`, `"////"` | `["/"]` |
//! | `"a"`, `"a/"`, `"a//"` | `["a",""]` |
//! | `"a/b/c"`, `"a/b/c//"` | `["a/b/c","a/b","a",""]` |
//! | `"a/b/"` | `["a/b","a",""]` |
//! | `"a//b"` | `["a//b","a",""]` (bỏ `b`, prefix `a/` → bỏ `/` cuối ⇒ `a`) |
//! | `"a///b//"` | `["a///b","a",""]` |
//! | `"a//b//c"` | `["a//b//c","a//b","a",""]` |
//! | `"/a"`, `"/a/"` | `["/a","/"]` (gốc `/` là cuối, **không** thêm `""`) |
//! | `"/a/b"` | `["/a/b","/a","/"]` |
//! | `"/a//b"` | `["/a//b","/a","/"]` |
//! | `"//srv/share"` | `["//srv/share","//srv","/"]` |
//! | `"//srv"` | `["//srv","/"]` |
//! | `"///srv/share"` | `["///srv/share","///srv","/"]` |
//! | `"."`, `".."` | `[".",""]`, `["..",""]` |
//! | `"./a"` | `["./a",".",""]` |
//! | `"a/."`, `"a/.."` | `["a/.","a",""]`, `["a/..","a",""]` |
//! | `"../a/b"` | `["../a/b","../a","..",""]` |
//! | `"thư/mục/tệp.txt"` | `["thư/mục/tệp.txt","thư/mục","thư",""]` |
//!
//! `\` và NUL là ký tự **thường** trên POSIX; Unicode giữ **từng scalar** (không NFC/NFD).
//! Gốc tuyệt đối **không** "ăn" `..`: `"/a/.."` ⇒ `["/a/..","/a","/"]`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `e28eeb7`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_path_ancestors
//! test result: FAILED. 1 passed; 9 failed
//! ```
//!
//! - **9 ca** đỏ `VS0301`: `vis check` báo ``module `std.path` không có hàm `ancestors` ``;
//!   `vis run` và `vis run --vm` đều exit 2 với đúng dòng lỗi cho **từng** lời gọi.
//! - **1 ca no-sugar** đã xanh đúng `VS0403` `` `str` không có trường `ancestors` `` — không
//!   phụ thuộc hàm mới (hành vi khoá).
//!
//! Sau khi thêm: 10/10 xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pa-{tag}-{}-{n}", std::process::id()));
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
// 1 — rỗng ⇒ [""] (đúng một phần tử rỗng) và gốc tuyệt đối ⇒ ["/"] (không thêm "")
// ===========================================================================

/// `""` ⇒ **một** phần tử rỗng (`len` = 1, khác `[]`); `"/"`/`"//"`/`"///"`/`"////"` ⇒ `["/"]`,
/// không có `""` sau gốc.
#[test]
fn rong_va_goc_tuyet_doi() {
    dung(
        "pa-rong-goc",
        r#"use std.path

fn main():
    let z = path.ancestors("")
    print(z.len())
    print(z[0] == "")
    print(z)
    print(path.ancestors("/"))
    print(path.ancestors("//"))
    print(path.ancestors("///"))
    print(path.ancestors("////"))
    print(path.ancestors("/").len())
"#,
        "1\ntrue\n[]\n[/]\n[/]\n[/]\n[/]\n1\n",
    );
}

// ===========================================================================
// 2 — tương đối: bỏ `/` cuối, tổ tiên cuối là ""
// ===========================================================================

/// `"a"`/`"a/"`/`"a//"` ⇒ `["a",""]`; `"a/b/c"` ⇒ `["a/b/c","a/b","a",""]`.
#[test]
fn tuong_doi_bo_slash_cuoi() {
    dung(
        "pa-tuong-doi",
        r#"use std.path

fn main():
    print(path.ancestors("a"))
    print(path.ancestors("a/"))
    print(path.ancestors("a//"))
    print(path.ancestors("a/b/c"))
    print(path.ancestors("a/b/c//"))
    print(path.ancestors("a/b/"))
    print(path.ancestors("a//b"))
    print(path.ancestors("a///b//"))
    print(path.ancestors("a//b//c"))
"#,
        "[a, ]\n[a, ]\n[a, ]\n[a/b/c, a/b, a, ]\n[a/b/c, a/b, a, ]\n[a/b, a, ]\n[a//b, a, ]\n[a///b, a, ]\n[a//b//c, a//b, a, ]\n",
    );
}

// ===========================================================================
// 3 — tuyệt đối: gốc `/` là cuối, **không** thêm ""; `//` đầu giữ nguyên
// ===========================================================================

/// `"/a"`/`"/a/"` ⇒ `["/a","/"]`; `"/a/b"` ⇒ `["/a/b","/a","/"]`;
/// `"//srv/share"` ⇒ `["//srv/share","//srv","/"]` (prefix `//` đầu **không** gộp).
#[test]
fn tuyet_doi_goc_la_cuoi() {
    dung(
        "pa-tuyet-doi",
        r#"use std.path

fn main():
    print(path.ancestors("/a"))
    print(path.ancestors("/a/"))
    print(path.ancestors("/a/b"))
    print(path.ancestors("/a/b//"))
    print(path.ancestors("/a//b"))
    print(path.ancestors("//srv"))
    print(path.ancestors("//srv/share"))
    print(path.ancestors("///srv/share"))
"#,
        "[/a, /]\n[/a, /]\n[/a/b, /a, /]\n[/a/b, /a, /]\n[/a//b, /a, /]\n[//srv, /]\n[//srv/share, //srv, /]\n[///srv/share, ///srv, /]\n",
    );
}

// ===========================================================================
// 4 — `.`/`..` **nguyên văn**, không chuẩn hoá, gốc không "ăn" `..`
// ===========================================================================

/// `"."`/`".."` ⇒ `[".",""]`/`["..",""]`; `"./a"` ⇒ `["./a",".",""]`;
/// `"a/."`/`"a/.."` giữ nguyên văn; `"../a/b"` ⇒ `["../a/b","../a","..",""]`.
#[test]
fn cham_va_cham_cham_nguyen_van() {
    dung(
        "pa-cham",
        r#"use std.path

fn main():
    print(path.ancestors("."))
    print(path.ancestors(".."))
    print(path.ancestors("./a"))
    print(path.ancestors("a/."))
    print(path.ancestors("a/.."))
    print(path.ancestors("../a/b"))
    print(path.ancestors("/a/.."))
    print(path.ancestors("/../a"))
"#,
        "[., ]\n[.., ]\n[./a, ., ]\n[a/., a, ]\n[a/.., a, ]\n[../a/b, ../a, .., ]\n[/a/.., /a, /]\n[/../a, /.., /]\n",
    );
}

// ===========================================================================
// 5 — Unicode giữ **từng scalar** (không NFC/NFD, emoji astral)
// ===========================================================================

/// Tên tiếng Việt/emoji/tổ hợp NFD chỉ cắt theo `/`, **không** đổi chuẩn hoá.
#[test]
fn unicode_giu_nguyen_scalar() {
    dung(
        "pa-unicode",
        r#"use std.path

fn main():
    print(path.ancestors("thư/mục/tệp.txt"))
    print(path.ancestors("đ😀/x"))
    print(path.ancestors("/đ😀//x/"))
    print(path.ancestors("e\u{0302}\u{0301}/x"))
"#,
        "[thư/mục/tệp.txt, thư/mục, thư, ]\n[đ😀/x, đ😀, ]\n[/đ😀//x, /đ😀, /]\n[e\u{0302}\u{0301}/x, e\u{0302}\u{0301}, ]\n",
    );
}

// ===========================================================================
// 6 — `\` và NUL là ký tự **thường**, không phải phân cách
// ===========================================================================

/// `"a\\b"` là **một** thành phần; NUL nằm trong thành phần và không bị cắt.
#[test]
fn backslash_va_nul_la_ky_tu_thuong() {
    dung(
        "pa-thuong",
        r#"use std.path

fn main():
    print(path.ancestors("a\\b"))
    print(path.ancestors("\\a\\b/c"))
    print(path.ancestors("a\0b"))
    print(path.ancestors("/a\0b/c"))
"#,
        "[a\\b, ]\n[\\a\\b/c, \\a\\b, ]\n[a\0b, ]\n[/a\0b/c, /a\0b, /]\n",
    );
}

// ===========================================================================
// 7 — kết quả là `list[str]` thật: gán, len, chỉ số, duyệt
// ===========================================================================

/// Kiểu trả `list[str]` qua biên gán; `len`/chỉ số/`for` đều dùng được.
#[test]
fn ket_qua_la_list_str_that() {
    dung(
        "pa-list-str",
        r#"use std.path

fn main():
    let xs: list[str] = path.ancestors("a/b/c")
    print(xs.len())
    print(xs[0])
    print(xs[2])
    print(xs[3] == "")
    for x in xs:
        print(x)
"#,
        "4\na/b/c\na\ntrue\na/b/c\na/b\na\n\n",
    );
}

// ===========================================================================
// 8 — **không** sugar: module-only
// ===========================================================================

/// `p.ancestors()` **không** phải sugar: `str` không có trường `ancestors` ⇒ `VS0403`,
/// kể cả khi đã `use std.path`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pa-sugar-literal",
            "fn main():\n    print(\"a/b\".ancestors())\n",
        ),
        (
            "pa-sugar-bien",
            "fn main():\n    let p = \"a/b\"\n    print(p.ancestors())\n",
        ),
        (
            "pa-sugar-co-use",
            "use std.path\n\nfn main():\n    print(\"a/b\".ancestors())\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 9 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 0/2 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("pa-kieu-int", "path.ancestors(1)", "VS0412"),
        ("pa-kieu-bool", "path.ancestors(true)", "VS0412"),
        ("pa-arity-0", "path.ancestors()", "VS0401"),
        ("pa-arity-2", "path.ancestors(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 10 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp rỗng/gốc/`/` lặp/`.`/`..`/tuyệt đối/Unicode/NUL để khoá **byte** stdout/stderr/exit
/// giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pa-hai-engine",
        r#"use std.path

fn main():
    print(path.ancestors(""))
    print(path.ancestors("/"))
    print(path.ancestors("//"))
    print(path.ancestors("///"))
    print(path.ancestors("a"))
    print(path.ancestors("a/"))
    print(path.ancestors("a//b"))
    print(path.ancestors("a///b//"))
    print(path.ancestors("a//b//c"))
    print(path.ancestors("/a"))
    print(path.ancestors("/a/b//"))
    print(path.ancestors("/a//b"))
    print(path.ancestors("//srv/share"))
    print(path.ancestors("."))
    print(path.ancestors(".."))
    print(path.ancestors("./a"))
    print(path.ancestors("a/."))
    print(path.ancestors("a/.."))
    print(path.ancestors("../a/b"))
    print(path.ancestors("thư/mục/tệp.txt"))
    print(path.ancestors("đ😀/x"))
    print(path.ancestors("e\u{0302}\u{0301}/x"))
    print(path.ancestors("a\\b"))
    print(path.ancestors("a\0b"))
    let xs: list[str] = path.ancestors("/a/b/")
    print(xs.len())
"#,
        "[]\n[/]\n[/]\n[/]\n[a, ]\n[a, ]\n[a//b, a, ]\n[a///b, a, ]\n[a//b//c, a//b, a, ]\n[/a, /]\n[/a/b, /a, /]\n[/a//b, /a, /]\n[//srv/share, //srv, /]\n[., ]\n[.., ]\n[./a, ., ]\n[a/., a, ]\n[a/.., a, ]\n[../a/b, ../a, .., ]\n[thư/mục/tệp.txt, thư/mục, thư, ]\n[đ😀/x, đ😀, ]\n[e\u{0302}\u{0301}/x, e\u{0302}\u{0301}, ]\n[a\\b, ]\n[a\0b, ]\n3\n",
    );
}
