// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.join_all)** — `std.path.join_all(parts: list[str]) -> str`, nối các thành
//! phần đường dẫn **trái → phải** bằng **đúng** quy tắc `std.path.join(a, b)`.
//!
//! Hợp đồng được khoá ở đây:
//! - `[]` ⇒ `""` (không có gì để nối).
//! - Một phần tử ⇒ **chính nó** qua quy tắc `path.join` (không chuẩn hoá, giữ `/` cuối).
//! - Nhiều phần tử ⇒ gấp trái: `acc = path.join(acc, phần_tử)`, nên `path.join_all([a, b])`
//!   **byte-identical** với `path.join(a, b)`.
//! - Thành phần **tuyệt đối** (bắt đầu `/`) ở giữa/cuối **thay** phần đã ghép trước đó;
//!   các thành phần sau nối tiếp trên nền tuyệt đối mới.
//! - Thành phần **rỗng** theo `path.join`: `["a", ""]` ⇒ `"a/"`, `["", "a"]` ⇒ `"a"`,
//!   `["a", "", "b"]` ⇒ `"a/b"`; `["a", "b", ""]` ⇒ `"a/b/"`.
//! - `/` cuối của **acc** bị bỏ khi ghép tiếp (`["a/", "b"]` ⇒ `"a/b"`), nhưng `/` cuối của
//!   **phần tử cuối** được giữ nguyên (`["a", "b//"]` ⇒ `"a/b//"`).
//! - Không chuẩn hoá `.`/`..`, không chạm đĩa/`cwd`/symlink/tồn tại; `\` là ký tự **thường**
//!   trên POSIX; thành phần Unicode giữ nguyên từng scalar.
//! - **Không sugar** (module-only): `s.join_all(...)`/`xs.join_all(...)` ⇒ `VS0403`, tránh
//!   nhập nhằng receiver `str`/`list`; sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `146cf32`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.path
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 4:11 module `std.path` không có hàm `join_all`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:4:11: error VS0301: module `std.path` không có hàm `join_all`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng dòng)
//! $ vis check sugar.vis                    # "a".join_all(...), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `join_all`
//! ```
//!
//! Chạy **đúng** bộ test này trước khi thêm hàm (`cargo test -p vis-cli --test
//! vd270_path_join_all`): `FAILED. 1 passed; 10 failed` — ca no-sugar xanh sẵn với `VS0403`
//! (hành vi khoá), 10 ca còn lại đỏ `VS0301 module std.path không có hàm join_all`.
//!
//! Thêm hàm xong: 11/11 test xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pja-{tag}-{}-{n}", std::process::id()));
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

/// Kỳ vọng theo **nền**: `posix` cho Unix, `windows` cho Windows (`path.join` theo `cfg!(windows)`).
fn ky_vong(posix: &'static str, windows: &'static str) -> &'static str {
    if cfg!(windows) {
        windows
    } else {
        posix
    }
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
// 1 — danh sách rỗng ⇒ "" (cả literal lẫn biến `list[str]`)
// ===========================================================================

#[test]
fn danh_sach_rong_tra_chuoi_rong() {
    dung(
        "pja-rong",
        r#"use std.path

fn main():
    print("[" + path.join_all([]) + "]")
    let rong: list[str] = []
    print("[" + path.join_all(rong) + "]")
"#,
        "[]\n[]\n",
    );
}

// ===========================================================================
// 2 — một phần tử ⇒ chính nó qua quy tắc join (không chuẩn hoá)
// ===========================================================================

#[test]
fn mot_phan_tu_giu_nguyen() {
    dung(
        "pja-mot",
        r#"use std.path

fn main():
    print("[" + path.join_all(["a"]) + "]")
    print("[" + path.join_all([""]) + "]")
    print("[" + path.join_all(["/a"]) + "]")
    print("[" + path.join_all(["a/"]) + "]")
    print("[" + path.join_all(["//a/"]) + "]")
"#,
        "[a]\n[]\n[/a]\n[a/]\n[//a/]\n",
    );
}

// ===========================================================================
// 3 — hai/ba/bốn phần tử: gấp trái, giống hệt path.join từng bước
// ===========================================================================

#[test]
fn hai_va_ba_phan_tu() {
    dung(
        "pja-hai-ba",
        r#"use std.path

fn main():
    print("[" + path.join_all(["a", "b"]) + "]")
    print("[" + path.join_all(["a", "b", "c"]) + "]")
    print("[" + path.join_all(["a", "b", "c", "d"]) + "]")
    print("[" + path.join_all(["/a", "b", "c"]) + "]")
"#,
        ky_vong(
            "[a/b]\n[a/b/c]\n[a/b/c/d]\n[/a/b/c]\n",
            "[a\\b]\n[a\\b\\c]\n[a\\b\\c\\d]\n[/a/b/c]\n",
        ),
    );
}

// ===========================================================================
// 4 — thành phần rỗng theo đúng path.join (không bỏ qua, không chuẩn hoá)
// ===========================================================================

#[test]
fn thanh_phan_rong() {
    dung(
        "pja-rong-thanh-phan",
        r#"use std.path

fn main():
    print("[" + path.join_all(["", ""]) + "]")
    print("[" + path.join_all(["", "a"]) + "]")
    print("[" + path.join_all(["a", ""]) + "]")
    print("[" + path.join_all(["a", "", "b"]) + "]")
    print("[" + path.join_all(["", "a", ""]) + "]")
    print("[" + path.join_all(["a", "b", ""]) + "]")
"#,
        ky_vong(
            "[]\n[a]\n[a/]\n[a/b]\n[a/]\n[a/b/]\n",
            "[]\n[a]\n[a\\]\n[a\\b]\n[a\\]\n[a\\b\\]\n",
        ),
    );
}

// ===========================================================================
// 5 — `/` đầu/cuối: bỏ `/` cuối của acc khi ghép tiếp, giữ `/` cuối phần tử cuối
// ===========================================================================

#[test]
fn slash_dau_va_cuoi() {
    dung(
        "pja-slash",
        r#"use std.path

fn main():
    print("[" + path.join_all(["a/", "b"]) + "]")
    print("[" + path.join_all(["a//", "b"]) + "]")
    print("[" + path.join_all(["/", "a"]) + "]")
    print("[" + path.join_all(["//", "a"]) + "]")
    print("[" + path.join_all(["a", "b/", "c"]) + "]")
    print("[" + path.join_all(["a", "b//"]) + "]")
    print("[" + path.join_all(["/a/", "b/"]) + "]")
    print("[" + path.join_all(["a", "//b"]) + "]")
    print("[" + path.join_all(["a", "//b", "c"]) + "]")
"#,
        ky_vong(
            "[a/b]\n[a/b]\n[/a]\n[/a]\n[a/b/c]\n[a/b//]\n[/a/b/]\n[//b]\n[//b/c]\n",
            "[a\\b]\n[a\\b]\n[//a]\n[///a]\n[a\\b\\c]\n[a\\b//]\n[/a/b/]\n[//b]\n[//b/c]\n",
        ),
    );
}

// ===========================================================================
// 6 — thành phần tuyệt đối **đầu** tiên: giữ `/` đầu và nối tiếp
// ===========================================================================

#[test]
fn tuyet_doi_dau() {
    dung(
        "pja-tuyet-doi-dau",
        r#"use std.path

fn main():
    print("[" + path.join_all(["/a", "b"]) + "]")
    print("[" + path.join_all(["/", "a", "b"]) + "]")
    print("[" + path.join_all(["//a", "b"]) + "]")
    print("[" + path.join_all(["/a/", "b", "c"]) + "]")
"#,
        ky_vong(
            "[/a/b]\n[/a/b]\n[//a/b]\n[/a/b/c]\n",
            "[/a/b]\n[//a/b]\n[//a/b]\n[/a/b/c]\n",
        ),
    );
}

// ===========================================================================
// 7 — thành phần tuyệt đối ở **giữa/cuối** thay phần đã ghép (đúng path.join)
// ===========================================================================

#[test]
fn tuyet_doi_giua_va_cuoi() {
    dung(
        "pja-tuyet-doi-giua",
        r#"use std.path

fn main():
    print("[" + path.join_all(["a", "/b", "c"]) + "]")
    print("[" + path.join_all(["a/b", "/c/d", "e"]) + "]")
    print("[" + path.join_all(["a", "/b", "/c", "d"]) + "]")
    print("[" + path.join_all(["a", "/", "b"]) + "]")
    print("[" + path.join_all(["a", "b", "/c"]) + "]")
    print("[" + path.join_all(["a", "/b/"]) + "]")
"#,
        ky_vong(
            "[/b/c]\n[/c/d/e]\n[/c/d]\n[/b]\n[/c]\n[/b/]\n",
            "[/b/c]\n[/c/d/e]\n[/c/d]\n[//b]\n[/c]\n[/b/]\n",
        ),
    );
}

// ===========================================================================
// 8 — Unicode: giữ nguyên từng scalar, không chuẩn hoá tổ hợp
// ===========================================================================

#[test]
fn thanh_phan_unicode() {
    dung(
        "pja-unicode",
        r#"use std.path

fn main():
    print("[" + path.join_all(["thư", "mục", "tệp.txt"]) + "]")
    print("[" + path.join_all(["/thư", "mục"]) + "]")
    print("[" + path.join_all(["đ😀", "x"]) + "]")
    print("[" + path.join_all(["thư mục", "x"]) + "]")
    print("[" + path.join_all(["e\u{0302}\u{0301}", "x"]) + "]")
"#,
        ky_vong(
            "[thư/mục/tệp.txt]\n[/thư/mục]\n[đ😀/x]\n[thư mục/x]\n[e\u{0302}\u{0301}/x]\n",
            "[thư\\mục\\tệp.txt]\n[/thư/mục]\n[đ😀\\x]\n[thư mục\\x]\n[e\u{0302}\u{0301}\\x]\n",
        ),
    );
}

// ===========================================================================
// 9 — ca âm tĩnh: sai kiểu (không phải list[str])/sai arity
// ===========================================================================

#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-str", "path.join_all(\"a\")", "VS0412"),
        ("kieu-int", "path.join_all(1)", "VS0412"),
        ("kieu-list-int", "path.join_all([1, 2])", "VS0412"),
        ("kieu-list-list", "path.join_all([[\"a\"]])", "VS0412"),
        ("arity-0", "path.join_all()", "VS0401"),
        ("arity-2", "path.join_all([\"a\"], [\"b\"])", "VS0401"),
    ] {
        let src = format!("use std.path\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 10 — **không** sugar: module-only cho cả receiver `str` lẫn `list`
// ===========================================================================

#[test]
fn khong_sugar_module_only() {
    for (tag, src) in [
        ("pja-sugar-str", "fn main():\n    print(\"a\".join_all())\n"),
        (
            "pja-sugar-str-arg",
            "fn main():\n    print(\"a\".join_all([\"b\"]))\n",
        ),
        (
            "pja-sugar-list",
            "fn main():\n    print([\"a\", \"b\"].join_all())\n",
        ),
        (
            "pja-sugar-bien",
            "fn main():\n    let xs = [\"a\", \"b\"]\n    print(xs.join_all())\n",
        ),
    ] {
        tu_choi(tag, src, "VS0403");
    }
}

// ===========================================================================
// 11 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "pja-hai-engine",
        r#"use std.path

fn main():
    print(path.join_all([]))
    print(path.join_all([""]))
    print(path.join_all(["a"]))
    print(path.join_all(["a", "b"]))
    print(path.join_all(["a", "b", "c"]))
    print(path.join_all(["a", "", "b"]))
    print(path.join_all(["a", ""]))
    print(path.join_all(["a/", "b"]))
    print(path.join_all(["a", "//b"]))
    print(path.join_all(["/a", "b", "c"]))
    print(path.join_all(["a", "/b", "c"]))
    print(path.join_all(["a", "/b", "/c", "d"]))
    print(path.join_all(["thư", "mục", "tệp.txt"]))
    let r: str = path.join_all(["đ😀", "x"])
    print(r)
"#,
        ky_vong(
            "\n\na\na/b\na/b/c\na/b\na/\na/b\n//b\n/a/b/c\n/b/c\n/c/d\nthư/mục/tệp.txt\nđ😀/x\n",
            "\n\na\na\\b\na\\b\\c\na\\b\na\\\na\\b\n//b\n/a/b/c\n/b/c\n/c/d\nthư\\mục\\tệp.txt\nđ😀\\x\n",
        ),
    );
}
