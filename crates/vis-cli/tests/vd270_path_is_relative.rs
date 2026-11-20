// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.is_relative)** — `std.path.is_relative(p: str) -> bool`, **đúng** phần bù từ
//! vựng của `path.is_abs`: **thuần chuỗi**, không chạm đĩa, không đọc `cwd`, không phân giải
//! symlink, không kiểm tra tồn tại.
//!
//! Hợp đồng được khoá ở đây:
//! - `is_relative(p) == !is_abs(p)` — **cùng** helper `la_tuyet_doi` và cùng cờ nền tảng, nên
//!   hai hàm không bao giờ lệch nhau.
//! - `""` ⇒ `true` (đường dẫn rỗng là **tương đối**); `"."`, `".."`, `"a"`, `"a/b"` ⇒ `true`.
//! - `"/"`, `"/a"`, `"//"`, `"///a"` ⇒ `false` — chỉ cần mở đầu bằng `/`; **không** gộp `/`.
//! - `\` và NUL là ký tự **thường** trên POSIX ⇒ `"a\\b"`, `"\a"`, `"a\0b"` đều **tương đối**.
//! - Thành phần Unicode giữ nguyên từng scalar (không kiểm tồn tại/chuẩn hoá Unicode).
//! - **Không sugar** (module-only): `p.is_relative()` bị từ chối `VS0403`; sai kiểu ⇒ `VS0412`;
//!   sai arity ⇒ `VS0401`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `0eb6b32`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.path, 7 lời gọi is_relative
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.path` không có hàm `is_relative`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.path` không có hàm `is_relative`
//! ... (đủ 7 dòng, mỗi lời gọi một lỗi)
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng 7 dòng)
//! $ vis check sugar.vis                    # "a".is_relative(), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `str` không có trường `is_relative`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pir-{tag}-{}-{n}", std::process::id()));
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
// 1 — rỗng và các dạng tương đối thường gặp
// ===========================================================================

/// `""` ⇒ `true`; tương đối không mở đầu bằng `/` ⇒ `true`.
#[test]
fn rong_va_tuong_doi_thuong() {
    dung(
        "pir-rong",
        r#"use std.path

fn main():
    print(path.is_relative(""))
    print(path.is_relative("a"))
    print(path.is_relative("a/b"))
    print(path.is_relative("./a"))
    print(path.is_relative("../a"))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 2 — tuyệt đối POSIX: chỉ cần mở đầu bằng `/`, không gộp `/`
// ===========================================================================

/// `/`, `/a`, `/a/b`, `//`, `///a` đều **không** tương đối.
#[test]
fn tuyet_doi_posix_va_hai_slash() {
    dung(
        "pir-tuyet-doi",
        r#"use std.path

fn main():
    print(path.is_relative("/"))
    print(path.is_relative("/a"))
    print(path.is_relative("/a/b"))
    print(path.is_relative("//"))
    print(path.is_relative("///a"))
    print(path.is_relative("//server/share"))
"#,
        "false\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 3 — `.` / `..`: vẫn là tương đối (không chuẩn hoá, không trap)
// ===========================================================================

/// `.`/`..` là thành phần **thường** ở tầng từ vựng ⇒ tương đối; không kiểm `cwd`.
#[test]
fn cham_va_cham_cham() {
    dung(
        "pir-cham",
        r#"use std.path

fn main():
    print(path.is_relative("."))
    print(path.is_relative(".."))
    print(path.is_relative("./"))
    print(path.is_relative("../.."))
    print(path.is_relative("a/../b"))
    print(path.is_relative("../../a"))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 4 — `is_relative` đúng là phần bù của `is_abs`
// ===========================================================================

/// Cùng danh sách đầu vào: `is_relative(p)` ⇔ `!is_abs(p)` cho mọi dạng (kể cả `//`, rỗng).
#[test]
fn phan_bu_cua_is_abs() {
    dung(
        "pir-phan-bu",
        r#"use std.path

fn main():
    let ps = ["", "/", "a", "a/b", "/a", "/a/b", "//", "//a", ".", "..", "../a", "đ😀"]
    for p in ps:
        print(path.is_relative(p) != path.is_abs(p))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 5 — `\` và NUL là ký tự **thường** trên POSIX
// ===========================================================================

/// `a\b`, `\a` (một `\` đầu) và `a\0b` đều **tương đối** — không phải phân cách, không trap.
#[test]
fn backslash_va_nul_thuong() {
    dung(
        "pir-backslash",
        r#"use std.path

fn main():
    print(path.is_relative("a\\b"))
    print(path.is_relative("\\a"))
    print(path.is_relative("a\0b"))
    print(path.is_relative("\\"))
    print(path.is_relative("/\\a"))
    print(path.is_relative("/a\0b"))
"#,
        "true\ntrue\ntrue\ntrue\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 6 — Unicode: giữ nguyên từng scalar, không kiểm tồn tại
// ===========================================================================

/// Tên tiếng Việt/emoji/tổ hợp NFD: tương đối hay tuyệt đối chỉ do `/` đầu quyết định.
#[test]
fn unicode_giu_nguyen_scalar() {
    dung(
        "pir-unicode",
        r#"use std.path

fn main():
    print(path.is_relative("thư/mục/tệp.txt"))
    print(path.is_relative("đ😀"))
    print(path.is_relative("/thư/mục"))
    print(path.is_relative("e\u{0302}\u{0301}/x"))
    print(path.is_relative("/đ😀"))
"#,
        "true\ntrue\nfalse\ntrue\nfalse\n",
    );
}

// ===========================================================================
// 7 — **không** sugar: module-only
// ===========================================================================

/// `p.is_relative()` **không** phải sugar: `str` không có trường `is_relative` ⇒ `VS0403`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pir-sugar-literal",
            "fn main():\n    print(\"a\".is_relative())\n",
        ),
        (
            "pir-sugar-bien",
            "fn main():\n    let p = \"/a\"\n    print(p.is_relative())\n",
        ),
        (
            "pir-sugar-co-use",
            "use std.path\n\nfn main():\n    print(\"a\".is_relative())\n",
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
        ("pir-kieu-int", "path.is_relative(1)", "VS0412"),
        ("pir-kieu-bool", "path.is_relative(true)", "VS0412"),
        ("pir-arity-0", "path.is_relative()", "VS0401"),
        ("pir-arity-2", "path.is_relative(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 9 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp rỗng/tương đối/tuyệt đối/`//`/`.`/`..`/Unicode/`\`/NUL vào một chương trình để khoá
/// **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pir-hai-engine",
        r#"use std.path

fn main():
    print(path.is_relative(""))
    print(path.is_relative("a/b"))
    print(path.is_relative("/a/b"))
    print(path.is_relative("//"))
    print(path.is_relative("."))
    print(path.is_relative(".."))
    print(path.is_relative("thư/mục"))
    print(path.is_relative("/thư/mục"))
    print(path.is_relative("a\\b"))
    print(path.is_relative("a\0b"))
    let r: bool = path.is_relative("x")
    print(r)
"#,
        "true\ntrue\nfalse\nfalse\ntrue\ntrue\ntrue\nfalse\ntrue\ntrue\ntrue\n",
    );
}
