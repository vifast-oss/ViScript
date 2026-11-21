// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.with_stem)** — `std.path.with_stem(p: str, stem: str) -> str`, thay **phần
//! thân** (file stem) của **thành phần cuối** đường dẫn, **giữ nguyên phần mở rộng hiện có**;
//! **thuần từ vựng POSIX**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm
//! tra tồn tại.
//!
//! **Probe Rust (toolchain ghim 1.100.0)** — `Path::with_stem` **không tồn tại** trên toolchain
//! này (`E0599: no method named with_stem`, feature `path_with_stem` cũng không có ⇒ `E0635`),
//! nên bảng dưới lấy từ **đúng** `file_stem()`/`extension()` + `with_file_name()` của Rust và
//! hợp đồng tài liệu (`foo.rs` + `bar` ⇒ `bar.rs`): thay stem, **giữ** extension. Probe cho:
//!
//! ```text
//! "a/b.txt"   + "c"  => "a/c.txt"     (stem "b", ext "txt")
//! "a/b.tar.gz"+ "c"  => "a/c.gz"      (stem "b.tar", ext "gz")
//! "a/b"       + "c"  => "a/c"         (không ext)
//! ".gitignore"+ "c"  => "c"           (stem = cả tên, ext None)
//! "a/b/"      + "c"  => "a/c"         (trailing slash chỉ đánh dấu thư mục)
//! "a/b.txt"   + ""   => "a/.txt"      (stem rỗng, ext giữ)
//! "a/b"       + ""   => "a/"          (stem rỗng, không ext)
//! ```
//!
//! Chỗ **khác Rust** (có chủ ý, vì tái dùng **đúng** `stem_voi`/`ext_voi` của
//! `path.stem`/`path.ext` trong repo — xem `pathwithext.rs`):
//! - **Không có tên tệp** (`""`, `/`, `///`, `.`, `..`, `a/..`, `a/.`) ⇒ trả `p` **nguyên văn**;
//!   Rust `with_file_name` lại sinh `"c"`, `"/c"`, `"./c"`, `"../c"`, `"a/../c"`, `"c"`.
//! - Tên **bắt đầu bằng `.` không có ext** theo `path.ext`: `.a.b` + `c` ⇒ `c` (Rust `c.b`),
//!   `foo.` + `c` ⇒ `c` (Rust `c.`), `foo.` + `""` ⇒ `""` (Rust `"."`).
//! - Tiền tố thư mục giữ **nguyên văn**: `a//b` + `c` ⇒ `a//c` (Rust `a/c`), `a/./b.txt` + `c`
//!   ⇒ `a/./c.txt` (Rust `a/c.txt`).
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `c6e0a2a`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_with_stem
//! test result: FAILED. 1 passed; 11 failed
//! ```
//!
//! - **10 ca dương** (mục 1–9 + ca gộp hai engine) đỏ `VS0301`: `vis check` báo
//!   `module `std.path` không có hàm `with_stem``; `vis run` và `vis run --vm` đều exit 2 với
//!   đúng dòng lỗi cho **từng** lời gọi.
//! - **1 ca sai kiểu/arity** đỏ vì `VS0301` xuất hiện thay cho `VS0412`/`VS0401`.
//! - **1 ca no-sugar** đã xanh đúng `VS0403` `` `str` không có trường `with_stem` `` — không phụ
//!   thuộc hàm mới, nên RED-BEFORE không thể xanh hơn 1/12.
//!
//! Sau khi thêm: 12/12 xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pws-{tag}-{}-{n}", std::process::id()));
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
// 1 — đã có phần mở rộng: thay stem, **giữ** phần mở rộng cuối
// ===========================================================================

/// `a/b.txt` + `c` ⇒ `a/c.txt`; chỉ stem **cuối** đổi, `b.tar.gz` có stem `b.tar` + ext `gz`.
#[test]
fn thay_than_giu_phan_mo_rong() {
    dung(
        "pws-co-ext",
        r#"use std.path

fn main():
    print(path.with_stem("a/b.txt", "c"))
    print(path.with_stem("a/b.tar.gz", "c"))
    print(path.with_stem("a/b.tar.gz", "d.e"))
    print(path.with_stem("a", "c"))
    print(path.with_stem("./a", "c"))
    print(path.with_stem("../a.txt", "c"))
    print(path.with_stem("/a/b.txt", "c"))
    print(path.with_stem("//srv/share/x.txt", "c"))
    print(path.with_stem("thư/mục/tệp.txt", "c"))
    let r: str = path.with_stem("x.y", "z")
    print(r)
"#,
        "a/c.txt\na/c.gz\na/d.e.gz\nc\n./c\n../c.txt\n/a/c.txt\n//srv/share/c.txt\nthư/mục/c.txt\nz.y\n",
    );
}

// ===========================================================================
// 2 — chưa có phần mở rộng: thay **toàn bộ** tên bằng stem mới
// ===========================================================================

/// `a/b` + `c` ⇒ `a/c` (không thêm ext); `b` + `c` ⇒ `c`; tiền tố thư mục giữ nguyên văn.
#[test]
fn khong_co_phan_mo_rong() {
    dung(
        "pws-khong-ext",
        r#"use std.path

fn main():
    print(path.with_stem("a/b", "c"))
    print(path.with_stem("b", "c"))
    print(path.with_stem("a/b/", "c"))
    print(path.with_stem("/a/b/", "c"))
    print(path.with_stem("a//b", "c"))
    print(path.with_stem("a/./b", "c"))
    print(path.with_stem("a\\b", "c"))
"#,
        // Windows coi `\` là phân cách: `a\b` = thư mục `a` + tệp `b` (không ext) ⇒ `a\c`.
        // POSIX coi `a\b` là **một** tên tệp ⇒ thay stem ⇒ `c`.
        if cfg!(windows) {
            "a/c\nc\na/c\n/a/c\na//c\na/./c\na\\c\n"
        } else {
            "a/c\nc\na/c\n/a/c\na//c\na/./c\nc\n"
        },
    );
}

// ===========================================================================
// 3 — tên bắt đầu bằng `.`: **không** có ext theo `path.ext` ⇒ stem là cả tên
// ===========================================================================

/// `.gitignore` + `c` ⇒ `c`; `.a.b` + `c` ⇒ `c` (khác Rust `c.b`, đã ghi trong tài liệu hàm).
#[test]
fn ten_bat_dau_bang_cham() {
    dung(
        "pws-cham-dau",
        r#"use std.path

fn main():
    print(path.with_stem(".gitignore", "c"))
    print(path.with_stem("a/.gitignore", "c"))
    print(path.with_stem(".a.b", "c"))
    print(path.with_stem("a/.a.b", "c"))
    print(path.with_stem(".gitignore", ""))
    print(path.with_stem("a/.a.b", "z"))
"#,
        "c\na/c\nc\na/c\n\na/z\n",
    );
}

// ===========================================================================
// 4 — `/` cuối: chỉ đánh dấu thư mục, bỏ khi ghi tên tệp
// ===========================================================================

/// `a/b.txt/` + `c` ⇒ `a/c.txt`; stem rỗng + không ext ⇒ `a/` (cùng Rust, đã probe).
#[test]
fn slash_cuoi() {
    dung(
        "pws-slash-cuoi",
        r#"use std.path

fn main():
    print(path.with_stem("a/b.txt/", "c"))
    print(path.with_stem("a/b.txt//", "c"))
    print(path.with_stem("a/b/", ""))
    print(path.with_stem("a/b.txt/", ""))
    print(path.with_stem("///", "c"))
"#,
        "a/c.txt\na/c.txt\na/\na/.txt\n///\n",
    );
}

// ===========================================================================
// 5 — không có tên tệp: nguyên văn, bỏ qua stem
// ===========================================================================

/// `""`, chỉ toàn `/`, `.`/`..`/`a/..`/`a/.`: trả `p` **nguyên văn** (khác Rust, đã ghi).
#[test]
fn khong_co_ten_tep() {
    dung(
        "pws-khong-ten",
        r#"use std.path

fn main():
    print(path.with_stem("", "c"))
    print(path.with_stem("/", "c"))
    print(path.with_stem("///", "c"))
    print(path.with_stem(".", "c"))
    print(path.with_stem("..", "c"))
    print(path.with_stem("a/..", "c"))
    print(path.with_stem("a/.", "c"))
"#,
        "\n/\n///\n.\n..\na/..\na/.\n",
    );
}

// ===========================================================================
// 6 — stem rỗng: giữ dấu chấm + ext; `foo.`/`...` không ext theo `path.ext`
// ===========================================================================

/// `a/b.txt` + `""` ⇒ `a/.txt`; `a/b` + `""` ⇒ `a/`; `foo.`/`...` + `c` ⇒ `c` (khác Rust).
#[test]
fn stem_rong_va_dau_cham_cuoi() {
    dung(
        "pws-stem-rong",
        r#"use std.path

fn main():
    print(path.with_stem("a/b.txt", ""))
    print(path.with_stem("a/b", ""))
    print(path.with_stem("/a/b.txt", ""))
    print(path.with_stem(".gitignore", ""))
    print(path.with_stem("foo.", ""))
    print(path.with_stem("foo.", "c"))
    print(path.with_stem("...", "c"))
"#,
        "a/.txt\na/\n/a/.txt\n\n\nc\nc\n",
    );
}

// ===========================================================================
// 7 — stem chứa dấu chấm / `/` / `\` / NUL / dấu cách: dùng **nguyên văn**
// ===========================================================================

/// Stem **không** bị kiểm: mọi byte được nối nguyên văn; `\`/NUL là ký tự **thường** trên POSIX.
#[test]
fn stem_nguyen_van() {
    dung(
        "pws-stem-nguyen-van",
        r#"use std.path

fn main():
    print(path.with_stem("a/b.txt", "c.d"))
    print(path.with_stem("a/b.txt", "x."))
    print(path.with_stem("a/b.txt", ".."))
    print(path.with_stem("a/b.txt", ".x"))
    print(path.with_stem("a/b", "x/y"))
    print(path.with_stem("a/b", "x\\y"))
    print(path.with_stem("a/b", " "))
    print(path.with_stem("a/b", "\0"))
"#,
        "a/c.d.txt\na/x..txt\na/...txt\na/.x.txt\na/x/y\na/x\\y\na/ \na/\0\n",
    );
}

// ===========================================================================
// 8 — Unicode: giữ nguyên từng scalar
// ===========================================================================

/// Tên tiếng Việt/emoji/tổ hợp NFD trong **cả** đường dẫn lẫn stem: giữ nguyên từng scalar.
#[test]
fn unicode_giu_nguyen_scalar() {
    dung(
        "pws-unicode",
        r#"use std.path

fn main():
    print(path.with_stem("thư/mục/tệp.txt", "mới"))
    print(path.with_stem("đ😀.txt", "x"))
    print(path.with_stem("thư/mục/không", "c"))
    print(path.with_stem("e\u{0302}\u{0301}.txt", "m"))
    print(path.with_stem("a/b.txt", "tệp😀"))
"#,
        "thư/mục/mới.txt\nx.txt\nthư/mục/c\nm.txt\na/tệp😀.txt\n",
    );
}

// ===========================================================================
// 9 — bất biến với `path.stem`/`path.ext`: thay stem nhưng ext **không đổi**
// ===========================================================================

/// Sau `with_stem`, `path.stem` = stem mới và `path.ext` = **đúng** ext cũ — chứng minh ba hàm
/// dùng cùng quy tắc (kể cả tên bắt đầu `.` và `foo.`).
#[test]
fn bat_bien_stem_va_ext() {
    dung(
        "pws-bat-bien",
        r#"use std.path

fn main():
    let ps = ["a/b.txt", "a/b.tar.gz", ".gitignore", "a/.a.b", "foo.", "a/b", "/a/b.txt", "a/b/", "thư/tệp.txt"]
    for p in ps:
        let r = path.with_stem(p, "c")
        print(path.stem(r) == "c")
        print(path.ext(r) == path.ext(p))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 10 — **không** sugar: module-only
// ===========================================================================

/// `p.with_stem(...)` **không** phải sugar: `str` không có trường `with_stem` ⇒ `VS0403`,
/// kể cả khi đã `use std.path`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pws-sugar-literal",
            "fn main():\n    print(\"a.txt\".with_stem(\"c\"))\n",
        ),
        (
            "pws-sugar-bien",
            "fn main():\n    let p = \"a.txt\"\n    print(p.with_stem(\"c\"))\n",
        ),
        (
            "pws-sugar-co-use",
            "use std.path\n\nfn main():\n    print(\"a.txt\".with_stem(\"c\"))\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 11 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 1/3 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("pws-kieu-dau", "path.with_stem(1, \"c\")", "VS0412"),
        ("pws-kieu-sau", "path.with_stem(\"a\", 2)", "VS0412"),
        ("pws-kieu-bool", "path.with_stem(\"a\", true)", "VS0412"),
        ("pws-arity-1", "path.with_stem(\"a\")", "VS0401"),
        (
            "pws-arity-3",
            "path.with_stem(\"a\", \"b\", \"c\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 12 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp thay stem/không ext/chấm đầu/stem rỗng/slash cuối/không tên/Unicode/NUL để khoá **byte**
/// stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pws-hai-engine",
        r#"use std.path

fn main():
    print(path.with_stem("a/b.txt", "c"))
    print(path.with_stem("a/b", "c"))
    print(path.with_stem("a/b.txt", ""))
    print(path.with_stem(".gitignore", "c"))
    print(path.with_stem("a.b/c.tar.gz", "xz"))
    print(path.with_stem("a/b/", "c"))
    print(path.with_stem("/a/b/", ""))
    print(path.with_stem("thư/mục/tệp.txt", "mới"))
    print(path.with_stem("", "c"))
    print(path.with_stem("/", "c"))
    print(path.with_stem("a//b", "c"))
    print(path.with_stem("a/b.txt", "c.d"))
    let r: str = path.with_stem("x.y", "z")
    print(r)
"#,
        "a/c.txt\na/c\na/.txt\nc\na.b/xz.gz\na/c\n/a/\nthư/mục/mới.txt\n\n/\na//c\na/c.d.txt\nz.y\n",
    );
}
