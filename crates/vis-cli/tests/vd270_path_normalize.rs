// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.normalize)** — `std.path.normalize(p: str) -> str`, chuẩn hoá **từ vựng**
//! đường dẫn POSIX, **thuần chuỗi** (không chạm đĩa/cwd/symlink/tồn tại).
//!
//! Hợp đồng được khoá ở đây:
//! - Tách theo `/`; gộp `/` lặp; bỏ `.`; `..` phân giải với thành phần **thường** đứng trước.
//! - `..` đầu chuỗi tương đối **không** có gì để phân giải ⇒ **giữ** (`../a`, `a/../../b` ⇒
//!   `../b`); gốc tuyệt đối **không** vượt được (`/..` ⇒ `/`).
//! - Giữ `/` đầu cho đầu vào tuyệt đối; bỏ `/` cuối trừ gốc `/`.
//! - Đầu vào rỗng/tương đối rỗng ⇒ `"."`; chỉ toàn `/` tuyệt đối ⇒ `"/"`.
//! - `\` và NUL là ký tự **thường** (helper POSIX thuần từ vựng, không có trap); thành phần
//!   Unicode giữ nguyên từng scalar.
//! - Sugar R5c `p.normalize()` ≡ `path.normalize(p)`, không cần `use std.path`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `484dd7c`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.path
//! TAP version 13
//! 1..1
//! not ok 1 - /tmp/opencode/vd270-path-red/probe.vis
//! # VS0301 5:11 module `std.path` không có hàm `normalize`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.path` không có hàm `normalize`
//! probe.vis:6:11: error VS0301: module `std.path` không có hàm `normalize`
//! probe.vis:7:11: error VS0301: module `std.path` không có hàm `normalize`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng ba dòng)
//! $ vis check sugar.vis                    # "a/./b".normalize(), không use
//! not ok 1 - /tmp/opencode/vd270-path-red/sugar.vis
//! # VS0403 3:11 `str` không có trường `normalize`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pn-{tag}-{}-{n}", std::process::id()));
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

/// Lỗi **chạy**: hai engine cùng exit **101** (R9/C1), cùng `VS0501`, stderr nêu hàm.
fn trap_ca_hai(tag: &str, src: &str, needle: &str) {
    let (code, out, err) = hai_engine(tag, src);
    assert_eq!(
        code, 101,
        "{tag}: lỗi chạy phải exit 101\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.contains("VS0501"), "{tag}: thiếu VS0501: {err}");
    assert!(
        err.contains(needle),
        "{tag}: stderr thiếu `{needle}`: {err}"
    );
}

// ===========================================================================
// 1 — tuyệt đối/tương đối, gộp `/`, bỏ `/` cuối
// ===========================================================================

/// Giữ `/` đầu; gộp `/` lặp; bỏ `/` cuối (trừ gốc); đường dẫn tương đối giữ nguyên dạng.
#[test]
fn tuyet_doi_tuong_doi_va_gop_slash() {
    dung(
        "pn-tuyet-doi",
        r#"use std.path

fn main():
    print(path.normalize("/a//b/./c/../d/"))
    print(path.normalize("//server///share/"))
    print(path.normalize("/a/b//"))
    print(path.normalize("a//b/"))
    print(path.normalize("a/b"))
    print(path.normalize("a/./b"))
"#,
        "/a/b/d\n/server/share\n/a/b\na/b\na/b\na/b\n",
    );
}

// ===========================================================================
// 2 — `.`/`..`: phân giải với thành phần thường; gốc tuyệt đối không vượt
// ===========================================================================

/// `..` ăn thành phần thường trước nó; `a/..` ⇒ `.`; `/..` ⇒ `/`; `/a/../../b` ⇒ `/b`.
#[test]
fn cham_va_cham_cham() {
    dung(
        "pn-cham",
        r#"use std.path

fn main():
    print(path.normalize("a/../b"))
    print(path.normalize("a/b/../../c"))
    print(path.normalize("a/.."))
    print(path.normalize("./a"))
    print(path.normalize("/a/../../b"))
    print(path.normalize("/.."))
    print(path.normalize("/../.."))
"#,
        "b\nc\n.\na\n/b\n/\n/\n",
    );
}

// ===========================================================================
// 3 — `..` đầu chuỗi tương đối được giữ; không ăn `..` khác
// ===========================================================================

/// `../a` giữ nguyên; `a/../../b` ⇒ `../b`; `..` không bị `..` sau ăn.
#[test]
fn giu_cham_cham_dau_chuoi() {
    dung(
        "pn-giu-cham-cham",
        r#"use std.path

fn main():
    print(path.normalize("../a"))
    print(path.normalize("../../a/b"))
    print(path.normalize("a/../../b"))
    print(path.normalize(".."))
    print(path.normalize("../a/../b"))
    print(path.normalize("../.."))
"#,
        "../a\n../../a/b\n../b\n..\n../b\n../..\n",
    );
}

// ===========================================================================
// 4 — rỗng, chỉ toàn `/`, gốc và dấu `/` cuối
// ===========================================================================

/// `""`/`.`/`./` ⇒ `.`; `/`, `//`, `///`, `/./` ⇒ `/`; `/a/` ⇒ `/a`.
#[test]
fn rong_goc_va_slash_cuoi() {
    dung(
        "pn-rong",
        r#"use std.path

fn main():
    print(path.normalize(""))
    print(path.normalize("."))
    print(path.normalize("./"))
    print(path.normalize("/"))
    print(path.normalize("//"))
    print(path.normalize("///"))
    print(path.normalize("/./"))
    print(path.normalize("/a/"))
"#,
        ".\n.\n.\n/\n/\n/\n/\n/a\n",
    );
}

// ===========================================================================
// 5 — thành phần Unicode giữ nguyên từng scalar
// ===========================================================================

/// Tên thư mục/tệp tiếng Việt, emoji và chuỗi tổ hợp NFD đều không bị đổi; `..` vẫn phân giải.
#[test]
fn thanh_phan_unicode() {
    dung(
        "pn-unicode",
        r#"use std.path

fn main():
    print(path.normalize("/thư/mục/../tệp.txt"))
    print(path.normalize("đ😀/x/../y"))
    print(path.normalize("e\u{0302}\u{0301}/x"))
    let r: str = path.normalize("/ư/..")
    print(r)
"#,
        "/thư/tệp.txt\nđ😀/y\ne\u{0302}\u{0301}/x\n/\n",
    );
}

// ===========================================================================
// 6 — `\` và NUL là ký tự thường (không trap)
// ===========================================================================

/// `\` **không** phải phân cách POSIX; `a\..\b` là **một** thành phần thường; NUL được giữ.
#[test]
fn backslash_va_nul_thuong() {
    dung(
        "pn-backslash",
        r#"use std.path

fn main():
    print(path.normalize("a\\b\\c"))
    print(path.normalize("a\\..\\b"))
    print(path.normalize("/a\\b/c/../d"))
    print(path.normalize("a\u{0000}b") == "a\u{0000}b")
    print(path.normalize("a\u{0000}/../b") == "b")
"#,
        "a\\b\\c\na\\..\\b\n/a\\b/d\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 7 — sugar R5c, không cần `use std.path`
// ===========================================================================

/// `p.normalize()` ≡ `path.normalize(p)`; biến `str` và literal đều nhận.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "pn-sugar",
        r#"fn main():
    print("a/./b".normalize())
    print("/x//y/../z/".normalize())
    let p = "../../a//b/"
    print(p.normalize())
    let r: str = p.normalize()
    print(r)
"#,
        "a/b\n/x/z\n../../a/b\n../../a/b\n",
    );
}

// ===========================================================================
// 8 — ca âm tĩnh: sai kiểu/arity; sugar thừa đối số ⇒ trap tầng chạy
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 0/2 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "path.normalize(1)", "VS0412"),
        ("kieu-bool", "path.normalize(true)", "VS0412"),
        ("arity-0", "path.normalize()", "VS0401"),
        ("arity-2", "path.normalize(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Sugar R5c chưa kiểm **arity** ở checker (nợ chung của R5c); lời gọi **thừa** đối số vẫn bị
/// **tầng chạy** chặn: trap `VS0501` nêu `path.normalize`, exit 101 ở **cả hai** engine.
#[test]
fn sugar_thua_doi_la_trap_ca_hai_engine() {
    trap_ca_hai(
        "pn-sugar-thua",
        "fn main():\n    print(\"a/./b\".normalize(1))\n",
        "path.normalize",
    );
}

// ===========================================================================
// 9 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp tuyệt đối/tương đối/Unicode/backslash/sugar vào một chương trình để khoá **byte**
/// stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pn-hai-engine",
        r#"use std.path

fn main():
    print(path.normalize("/a//b/./c/../d/"))
    print(path.normalize("a/../b"))
    print(path.normalize("../a"))
    print(path.normalize("a/../../b"))
    print(path.normalize(""))
    print(path.normalize("///"))
    print(path.normalize("/thư/mục/../tệp.txt"))
    print(path.normalize("a\\b\\c"))
    print("x/./y/../z".normalize())
"#,
        "/a/b/d\nb\n../a\n../b\n.\n/\n/thư/tệp.txt\na\\b\\c\nx/z\n",
    );
}
