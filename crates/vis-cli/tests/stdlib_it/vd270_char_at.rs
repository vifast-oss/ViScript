// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (char-at)** — scalar Unicode tại chỉ số: `std.str.char_at`.
//!
//! Hợp đồng được khoá ở đây:
//! - `char_at(s, i)` trả scalar Unicode thứ `i` (**zero-based**, cùng đơn vị
//!   `char_count`/`find`) dưới dạng `str?` (`Some(str)`/`None`).
//! - `i < 0`, `i >= char_count(s)`, hoặc `s` rỗng ⇒ `None` — **không trap**, **không** cắt
//!   byte: chỉ số đếm theo **scalar**, không phải byte UTF-8.
//! - Kết quả là **một scalar** (`char`), **không** phải grapheme: `"e\u{301}"` có 2 scalar,
//!   `"👨‍👩‍👧"` tách theo từng scalar (emoji + ZWJ), mỗi kết quả dài đúng số byte của scalar đó.
//! - Sugar R5c `s.char_at(i)` chạy **không** cần `use std.str`; sai kiểu/sai arity bị checker
//!   chặn tĩnh (`VS0412`/`VS0401`), kết quả `str?` gán vào `int` cũng bị chặn.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `target/debug/vis` dựng từ cây này @ `330d86c`):
//!
//! ```text
//! $ vis check probe.vis                    # use std.str, 2 lời gọi
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 4:11 module `std.str` không có hàm `char_at`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:4:11: error VS0301: module `std.str` không có hàm `char_at`
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `char_at`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng hai dòng)
//! $ vis check sugar.vis                    # "abc".char_at(2), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `char_at`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-ca-{tag}-{}-{n}", std::process::id()));
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
// 1 — ASCII: zero-based, đủ biên
// ===========================================================================

/// `"abc"` ⇒ `Some(a)`/`Some(b)`/`Some(c)`; chỉ số 0 là scalar **đầu**, không phải byte.
#[test]
fn ascii_zero_based() {
    dung(
        "ascii",
        r#"use std.str

fn main():
    print(str.char_at("abc", 0))
    print(str.char_at("abc", 1))
    print(str.char_at("abc", 2))
    print(str.char_at("x", 0))
    let x: str? = str.char_at("abc", 1)
    match x:
        Some(c): print(c)
        None: print("none")
"#,
        "Some(a)\nSome(b)\nSome(c)\nSome(x)\nb\n",
    );
}

// ===========================================================================
// 2 — Unicode/emoji: scalar, không phải byte hay grapheme
// ===========================================================================

/// `"đ😀x"` ⇒ 3 scalar; `e` + dấu kết hợp ⇒ 2 scalar; gia đình ZWJ tách theo từng scalar.
/// Mỗi kết quả là **một** scalar: `byte_len` của nó đúng bằng scalar đó (đ/😀 = 2/4 byte,
/// ZWJ = 3 byte), không cắt giữa UTF-8.
#[test]
fn unicode_emoji_theo_scalar() {
    dung(
        "unicode",
        r#"use std.str

fn do_dai(c: str?) -> int:
    match c:
        Some(x): str.byte_len(x)
        None: 0

fn main():
    print(str.char_at("đ😀x", 0))
    print(str.char_at("đ😀x", 1))
    print(str.char_at("đ😀x", 2))
    print(do_dai(str.char_at("đ😀x", 0)))
    print(do_dai(str.char_at("đ😀x", 1)))
    print(do_dai(str.char_at("e\u{301}", 0)))
    print(do_dai(str.char_at("e\u{301}", 1)))
    print(do_dai(str.char_at("👨‍👩‍👧", 0)))
    print(do_dai(str.char_at("👨‍👩‍👧", 1)))
    print(do_dai(str.char_at("👨‍👩‍👧", 2)))
    print(do_dai(str.char_at("👨‍👩‍👧", 3)))
    print(do_dai(str.char_at("👨‍👩‍👧", 4)))
"#,
        "Some(đ)\nSome(😀)\nSome(x)\n2\n4\n1\n2\n4\n3\n4\n3\n4\n",
    );
}

/// Kết quả `str?` là `Option[str]` **thật**: `match Some/None` chạy được, `Some` mang `str`.
#[test]
fn option_str_that() {
    dung(
        "option",
        r#"use std.str

fn main():
    let a: str? = str.char_at("abc", 1)
    match a:
        Some(c): print(c + "!")
        None: print("none")
    let b: str? = str.char_at("abc", 9)
    match b:
        Some(c): print(c)
        None: print("none")
    let c: str? = str.char_at("", 0)
    match c:
        Some(c): print(c)
        None: print("none")
"#,
        "b!\nnone\nnone\n",
    );
}

// ===========================================================================
// 3 — chỉ số âm ⇒ None (không trap)
// ===========================================================================

/// `i < 0` ⇒ `None`, kể cả `-1` và biến âm; chương trình vẫn exit 0, không trap `VS0501`.
#[test]
fn chi_so_am_tra_none_khong_trap() {
    dung(
        "am",
        r#"use std.str

fn main():
    print(str.char_at("abc", -1))
    print(str.char_at("abc", -5))
    print(str.char_at("đ😀", -2))
    let i: int = 0 - 3
    print(str.char_at("abc", i))
    print(str.char_at("", -1))
"#,
        "None\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 4 — vượt quá char_count ⇒ None (không trap)
// ===========================================================================

/// `i >= char_count` ⇒ `None`, kể cả chỉ số rất lớn; không cấp phát theo `i`.
#[test]
fn qua_dai_tra_none_khong_trap() {
    dung(
        "qua-dai",
        r#"use std.str

fn main():
    print(str.char_at("abc", 3))
    print(str.char_at("abc", 4))
    print(str.char_at("abc", 9223372036854775807))
    print(str.char_at("đ😀x", 3))
    print(str.char_at("x", 1))
    let i: int = 100
    print(str.char_at("abc", i))
"#,
        "None\nNone\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 5 — chuỗi rỗng ⇒ None với mọi chỉ số
// ===========================================================================

/// `""` có 0 scalar ⇒ mọi `i` (âm, 0, dương) đều `None`.
#[test]
fn rong_tra_none() {
    dung(
        "rong",
        r#"use std.str

fn main():
    print(str.char_at("", 0))
    print(str.char_at("", 1))
    print(str.char_at("", 999))
    let s: str = ""
    print(s.char_at(0))
"#,
        "None\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.char_at(i)` ≡ `str.char_at(s, i)`; giữ nguyên `str?` và các biên.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "đ😀x"
    print(s.char_at(0))
    print(s.char_at(1))
    print(s.char_at(2))
    print(s.char_at(3))
    print("abc".char_at(1))
    print("abc".char_at(-1))
    print("".char_at(0))
    let c: str? = s.char_at(1)
    print(c)
"#,
        "Some(đ)\nSome(😀)\nSome(x)\nNone\nSome(b)\nNone\nNone\nSome(😀)\n",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số (`i` phải `int`).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.char_at(1, 0)", "VS0412"),
        ("kieu-2", "str.char_at(\"abc\", \"1\")", "VS0412"),
        ("kieu-3", "str.char_at(\"abc\", 1.5)", "VS0412"),
        ("arity-1", "str.char_at(\"abc\")", "VS0401"),
        ("arity-2", "str.char_at()", "VS0401"),
        ("arity-3", "str.char_at(\"abc\", 0, 1)", "VS0401"),
        ("sugar-kieu", "\"abc\".char_at(\"1\")", "VS0412"),
        ("sugar-kieu-2", "\"abc\".char_at(1.5)", "VS0412"),
        ("sugar-arity", "\"abc\".char_at()", "VS0401"),
        ("sugar-arity-2", "\"abc\".char_at(0, 1)", "VS0401"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `str?` thật: gán vào `int` bị chặn (ca dương gán `str?` ở test 2).
#[test]
fn ket_qua_la_str_option_khong_phai_int() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.char_at(\"abc\", 0)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"abc\".char_at(0)\n    print(x)\n",
        "VS0412",
    );
}
