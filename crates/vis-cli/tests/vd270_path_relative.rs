// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.relative)** — `std.path.relative(from: str, to: str) -> str`, đường dẫn
//! **tương đối** từ `from` tới `to`, **thuần từ vựng POSIX** (không chạm đĩa/`cwd`/symlink/tồn tại).
//!
//! Hợp đồng được khoá ở đây:
//! - Hai đối số được chuẩn hoá bằng **đúng** logic `path.normalize` (cùng helper `pathnorm`).
//! - Cùng loại (cả hai tuyệt đối hoặc cả hai tương đối): bỏ **tiền tố chung** theo thành phần,
//!   mỗi thành phần `from` còn lại phát `..`, rồi nối các thành phần `to` còn lại; kết quả rỗng
//!   ⇒ `"."`. Kết quả **luôn tương đối** khi cùng loại (kể cả cả hai tuyệt đối).
//! - `..` được đối xử như thành phần **thường** khi so tiền tố (không phân giải thêm).
//! - Một tuyệt đối, một tương đối: trả **`to` đã chuẩn hoá** — **không** phân giải `cwd`.
//! - `from`/`to` rỗng theo normalize: `""` ⇒ `.` (rỗng tương đối), chỉ toàn `/` ⇒ `/`.
//! - Không sugar (module-only): `s.relative(...)` bị từ chối `VS0403`, tránh nhập nhằng receiver.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `5dead58`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.path
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.path` không có hàm `relative`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.path` không có hàm `relative`
//! probe.vis:6:11: error VS0301: module `std.path` không có hàm `relative`
//! probe.vis:7:11: error VS0301: module `std.path` không có hàm `relative`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng ba dòng)
//! $ vis check sugar.vis                    # "a/b".relative("a/c"), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `str` không có trường `relative`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pr-{tag}-{}-{n}", std::process::id()));
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
// 1 — cùng đường dẫn ⇒ "." (cả tuyệt đối lẫn tương đối)
// ===========================================================================

/// `from == to` (sau chuẩn hoá) ⇒ `"."`.
#[test]
fn cung_duong_dan_tra_cham() {
    dung(
        "pr-cung",
        r#"use std.path

fn main():
    print(path.relative("a/b", "a/b"))
    print(path.relative("/a/b", "/a/b"))
    print(path.relative("", ""))
    print(path.relative(".", "./"))
    print(path.relative("a/./b/", "a//b"))
    print(path.relative("///", "/./"))
"#,
        ".\n.\n.\n.\n.\n.\n",
    );
}

// ===========================================================================
// 2 — con, cháu: bỏ tiền tố chung rồi nối phần còn lại của `to`
// ===========================================================================

/// `to` nằm dưới `from` ⇒ chỉ còn phần `to` sau tiền tố chung (không có `..`).
#[test]
fn con_va_chau() {
    dung(
        "pr-con",
        r#"use std.path

fn main():
    print(path.relative("a", "a/b"))
    print(path.relative("a/b", "a/b/c/d"))
    print(path.relative("/a", "/a/b/c"))
    print(path.relative("/", "/a"))
    print(path.relative("/a/b", "/a/b/c"))
"#,
        "b\nc/d\nb/c\na\nc\n",
    );
}

// ===========================================================================
// 3 — anh em: mỗi thành phần `from` còn lại phát một `..`
// ===========================================================================

/// Anh em cùng cấp và khác cấp: `..` cho từng thành phần `from` còn lại.
#[test]
fn anh_em_phat_cham_cham() {
    dung(
        "pr-anh-em",
        r#"use std.path

fn main():
    print(path.relative("a/b", "a/c"))
    print(path.relative("a/b/c", "a/d"))
    print(path.relative("/a/b", "/a/c/d"))
    print(path.relative("/a/b/c", "/a/x/y/z"))
"#,
        "../c\n../../d\n../c/d\n../../x/y/z\n",
    );
}

// ===========================================================================
// 4 — cơ sở sâu hơn: nhiều `..` rồi mới tới `to`
// ===========================================================================

/// `from` sâu hơn `to`: `..` đủ cho phần `from` còn lại, sau đó nối `to`.
#[test]
fn co_so_sau_hon() {
    dung(
        "pr-sau",
        r#"use std.path

fn main():
    print(path.relative("a/b/c", "a"))
    print(path.relative("a/b/c/d", "a/b"))
    print(path.relative("/a/b/c/d", "/a"))
    print(path.relative("a/b/c", "x/y"))
"#,
        "../..\n../..\n../../..\n../../../x/y\n",
    );
}

// ===========================================================================
// 5 — tuyệt đối ↔ tuyệt đối và trộn tuyệt đối/tương đối
// ===========================================================================

/// Cả hai tuyệt đối ⇒ kết quả **tương đối** (không giữ `/` đầu). Trộn loại ⇒ trả
/// **`to` đã chuẩn hoá** (không phân giải `cwd`, không ghép).
#[test]
fn tuyet_doi_va_tron_loai() {
    dung(
        "pr-tuyet-doi",
        r#"use std.path

fn main():
    print(path.relative("/a", "/b"))
    print(path.relative("/a/b", "/"))
    print(path.relative("/", "/a/b"))
    print(path.relative("a", "/b"))
    print(path.relative("/a", "b"))
    print(path.relative("a", "/"))
    print(path.relative("/", "a"))
    print(path.relative("", "/a/"))
    print(path.relative("/x/", "a//b/"))
"#,
        "../b\n../..\na/b\n/b\nb\n/\na\n/a\na/b\n",
    );
}

// ===========================================================================
// 6 — `..` là thành phần **thường** khi so tiền tố
// ===========================================================================

/// `..` đầu chuỗi tương đối được so **nguyên văn** như thành phần; không phân giải thêm.
#[test]
fn cham_cham_la_thanh_phan_thuong() {
    dung(
        "pr-cham-cham",
        r#"use std.path

fn main():
    print(path.relative("../a", "../b"))
    print(path.relative("../../a/b", "../../c"))
    print(path.relative("../a", "x"))
    print(path.relative("..", ".."))
    print(path.relative("../a/b", "../a/c"))
"#,
        "../b\n../../c\n../../x\n.\n../c\n",
    );
}

// ===========================================================================
// 7 — gốc `/` và dấu `/` cuối
// ===========================================================================

/// Gốc ↔ gốc ⇒ `.`; `/` cuối và `/` lặp bị bỏ qua bởi chuẩn hoá trước khi so.
#[test]
fn goc_va_slash_cuoi() {
    dung(
        "pr-goc",
        r#"use std.path

fn main():
    print(path.relative("/", "/"))
    print(path.relative("///", "/./"))
    print(path.relative("/a/b/", "/a/b/c/"))
    print(path.relative("a/b/", "a/c/"))
    print(path.relative("a//b//", "./a//c/"))
"#,
        ".\n.\nc\n../c\n../c\n",
    );
}

// ===========================================================================
// 8 — thành phần Unicode giữ nguyên từng scalar
// ===========================================================================

/// Tên thư mục/tệp tiếng Việt, emoji và chuỗi tổ hợp NFD đều không bị đổi.
#[test]
fn thanh_phan_unicode() {
    dung(
        "pr-unicode",
        r#"use std.path

fn main():
    print(path.relative("/thư/mục", "/thư/tệp.txt"))
    print(path.relative("đ😀", "đ😀/x"))
    print(path.relative("/ư", "/ư/../.."))
    print(path.relative("e\u{0302}\u{0301}/x", "e\u{0302}\u{0301}/y"))
"#,
        "../tệp.txt\nx\n..\n../y\n",
    );
}

// ===========================================================================
// 9 — đầu vào rỗng, `.`/`..`, chuẩn hoá dùng chung
// ===========================================================================

/// `""` theo normalize (`.`) trước khi so; `.` và `..` không tạo thành phần giả.
#[test]
fn rong_va_chuan_hoa_dung_chung() {
    dung(
        "pr-rong",
        r#"use std.path

fn main():
    print(path.relative("", "a"))
    print(path.relative("a", ""))
    print(path.relative("", "."))
    print(path.relative(".", "a/b"))
    print(path.relative("a/./b/", "a/b/../c/"))
    print(path.relative("/", ""))
    print(path.relative("a", "b/./c/../d"))
"#,
        "a\n..\n.\na/b\n../c\n.\n../b/d\n",
    );
}

// ===========================================================================
// 10 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 1/3 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "path.relative(1, \"a\")", "VS0412"),
        ("kieu-bool", "path.relative(true, \"a\")", "VS0412"),
        ("kieu-to-int", "path.relative(\"a\", 1)", "VS0412"),
        ("arity-1", "path.relative(\"a\")", "VS0401"),
        ("arity-3", "path.relative(\"a\", \"b\", \"c\")", "VS0401"),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 11 — **không** sugar: module-only, tránh nhập nhằng receiver
// ===========================================================================

/// `s.relative(...)` **không** phải sugar: `str` không có trường `relative` ⇒ `VS0403`.
#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        (
            "pr-sugar-literal",
            "fn main():\n    print(\"a/b\".relative(\"a/c\"))\n",
        ),
        (
            "pr-sugar-bien",
            "fn main():\n    let p = \"a/b\"\n    print(p.relative(\"a/c\"))\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 12 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp cùng loại/khác loại/tuyệt đối/`..`/Unicode/rỗng vào một chương trình để khoá **byte**
/// stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pr-hai-engine",
        r#"use std.path

fn main():
    print(path.relative("a/b", "a/c"))
    print(path.relative("a/b/c", "a"))
    print(path.relative("/a/b", "/a/c/d"))
    print(path.relative("/", "/a"))
    print(path.relative("/a", "b"))
    print(path.relative("a", "/b"))
    print(path.relative("../a", "../b"))
    print(path.relative("", ""))
    print(path.relative("/", ""))
    print(path.relative("/thư/mục", "/thư/tệp.txt"))
    let r: str = path.relative("a//b/", "a/./c/")
    print(r)
"#,
        "../c\n../..\n../c/d\na\nb\n/b\n../b\n.\n.\n../tệp.txt\n../c\n",
    );
}
