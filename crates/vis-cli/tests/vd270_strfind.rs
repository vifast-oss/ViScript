// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strfind)** — `std.str.find(s, needle) -> int?`, `str.rfind(s, needle) -> int?`,
//! `str.count(s, needle) -> int`.
//!
//! Hợp đồng được khoá ở đây:
//! - Chỉ số là **offset scalar Unicode** (cùng đơn vị `s.chars().count()`), **không** phải byte:
//!   `str.find("đ😀x", "x") == Some(2)`.
//! - `find` = lần xuất hiện **đầu tiên**, `rfind` = **cuối cùng**; không khớp ⇒ `None`.
//! - `count` = số lần xuất hiện **không chồng** (như Rust `matches`):
//!   `count("aaa", "aa") == 1`, `count("aaaa", "aa") == 2`.
//! - `needle` rỗng theo nghĩa Rust: `find("") == Some(0)`, `rfind("") == Some(len_scalar)`,
//!   `count("") == len_scalar + 1` (`count("")` trên chuỗi rỗng = `1`).
//! - Kiểu trả: `find`/`rfind` là **`int?`** (`Ty::OptInt` → `Option[int]`), `count` là `int`;
//!   sugar R5c `s.find(...)`/`s.rfind(...)`/`s.count(...)` ≡ `str.*(s, ...)`, không cần import.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `3d30be1`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.str
//! TAP version 13
//! 1..1
//! not ok 1 - /tmp/opencode/vd270-strfind-red/probe.vis
//! # VS0301 5:11 module `std.str` không có hàm `find`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `find`
//! probe.vis:6:11: error VS0301: module `std.str` không có hàm `rfind`
//! probe.vis:7:11: error VS0301: module `std.str` không có hàm `count`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng ba dòng)
//! $ vis check sugar.vis                    # "banana".find("na"), không use
//! not ok 1 - /tmp/opencode/vd270-strfind-red/sugar.vis
//! # VS0403 3:11 `str` không có trường `find`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-sf-{tag}-{}-{n}", std::process::id()));
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
// 1 — ASCII / Unicode / emoji: offset scalar, first / last
// ===========================================================================

/// `find`/`rfind` trên ASCII, tiếng Việt nhiều dấu, emoji (mỗi emoji là **một** scalar).
#[test]
fn find_rfind_ascii_unicode_emoji() {
    dung(
        "first-last",
        r#"use std.str

fn main():
    print(str.find("banana", "na"))
    print(str.find("banana", "x"))
    print(str.find("xin chào thế giới", "chào"))
    print(str.find("😀a😀b", "b"))
    print(str.find("đ😀x", "x"))
    print(str.rfind("banana", "na"))
    print(str.rfind("aXaXa", "a"))
    print(str.rfind("😀a😀", "😀"))
"#,
        "Some(2)\nNone\nSome(4)\nSome(3)\nSome(2)\nSome(4)\nSome(4)\nSome(2)\n",
    );
}

/// `int?` là `Option[int]` **thật**: `match Some/None` chạy được, gán `int?`/`int` đúng kiểu.
#[test]
fn ket_qua_la_option_int_that() {
    dung(
        "option-that",
        r#"use std.str

fn main():
    match str.find("banana", "na"):
        Some(i): print(i)
        None: print("none")
    match str.find("banana", "zz"):
        Some(_): print("some")
        None: print("none")
    match str.rfind("abcabc", "abc"):
        Some(i): print(i)
        None: print("none")
    let a: int? = str.find("abc", "b")
    let b: int = str.count("abc", "b")
    print(a)
    print(b)
"#,
        "2\nnone\n3\nSome(1)\n1\n",
    );
}

// ===========================================================================
// 2 — count: không chồng, có/không khớp, Unicode
// ===========================================================================

/// `count` đếm **không chồng** (Rust `matches`), không đếm chồng; `0` khi không khớp.
#[test]
fn count_khong_chong() {
    dung(
        "count",
        r#"use std.str

fn main():
    print(str.count("aaaa", "aa"))
    print(str.count("aaa", "aa"))
    print(str.count("ababab", "ab"))
    print(str.count("banana", "na"))
    print(str.count("banana", "x"))
    print(str.count("😀😀😀", "😀😀"))
    print(str.count("a😀a😀", "😀"))
"#,
        "2\n1\n3\n2\n0\n1\n2\n",
    );
}

// ===========================================================================
// 3 — needle rỗng: nghĩa Rust
// ===========================================================================

/// `find("") == Some(0)`, `rfind("") == Some(len_scalar)`, `count("") == len_scalar + 1`;
/// trên chuỗi rỗng: `Some(0)`, `Some(0)`, `1`; needle khác rỗng trên chuỗi rỗng ⇒ `None`/`0`.
#[test]
fn needle_rong_nghia_rust() {
    dung(
        "empty-needle",
        r#"use std.str

fn main():
    print(str.find("abc", ""))
    print(str.rfind("abc", ""))
    print(str.count("abc", ""))
    print(str.find("", ""))
    print(str.rfind("", ""))
    print(str.count("", ""))
    print(str.rfind("đ😀", ""))
    print(str.count("đ😀", ""))
    print(str.find("", "a"))
    print(str.count("", "a"))
"#,
        "Some(0)\nSome(3)\n4\nSome(0)\nSome(0)\n1\nSome(2)\n3\nNone\n0\n",
    );
}

// ===========================================================================
// 4 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.find(...)`/`s.rfind(...)`/`s.count(...)` ≡ `str.*(s, ...)`; kiểu trả qua sugar vẫn
/// `int?`/`int` nên gán có chú thích hợp lệ.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "banana"
    print(s.find("na"))
    print(s.rfind("na"))
    print(s.count("na"))
    let o: int? = s.find("na")
    let n: int = s.count("na")
    print(o)
    print(n)
    print("đ😀x".find("x"))
    print("😀a😀".rfind("😀"))
    print("aaaa".count("aa"))
"#,
        "Some(2)\nSome(4)\n2\nSome(2)\n2\nSome(2)\nSome(2)\n2\n",
    );
}

// ===========================================================================
// 5 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.find(1, \"a\")", "VS0412"),
        ("kieu-2", "str.find(\"a\", 1)", "VS0412"),
        ("kieu-3", "str.rfind(\"a\", 1)", "VS0412"),
        ("kieu-4", "str.count(\"a\", 1)", "VS0412"),
        ("arity-1", "str.find(\"a\")", "VS0401"),
        ("arity-2", "str.find(\"a\", \"b\", \"c\")", "VS0401"),
        ("arity-3", "str.rfind()", "VS0401"),
        ("arity-4", "str.count(\"a\")", "VS0401"),
        ("sugar-kieu", "\"a\".find(1)", "VS0412"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// **VD-270 (R5c arity)**: sugar `s.rfind(…)` thiếu/thừa đối số bị checker chặn trước khi chạy —
/// `VS0401` `str.rfind cần 1 đối số`, đúng như module call; không còn trap `VS0501`.
#[test]
fn sugar_sai_arity_bi_chan() {
    for (tag, call) in [
        ("sugar-arity-thieu", "\"a\".rfind()"),
        ("sugar-arity-thua", "\"a\".rfind(\"a\", \"b\")"),
    ] {
        let src = format!("fn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}

/// Gán sai kiểu kết quả: `find` là `int?` (không phải `int`), `count` là `int`
/// (không phải `int?`); sugar cũng bị chặn.
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    tu_choi(
        "find-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.find(\"abc\", \"b\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "count-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.count(\"abc\", \"b\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-find-sang-int",
        "fn main():\n    let x: int = \"abc\".find(\"b\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-count-sang-str",
        "fn main():\n    let x: str = \"abc\".count(\"b\")\n    print(x)\n",
        "VS0412",
    );
}
