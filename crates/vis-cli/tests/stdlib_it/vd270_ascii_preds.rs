// SPDX-License-Identifier: Apache-2.0
//! **VD-270 (ascii-preds)** — bốn vị từ ASCII: `std.str.is_ascii_digit`,
//! `is_ascii_alphabetic`, `is_ascii_alphanumeric`, `is_ascii_whitespace` — mỗi hàm
//! `(str) -> bool`.
//!
//! Hợp đồng được khoá ở đây:
//! - Mỗi hàm `true` **khi và chỉ khi** `s` **không rỗng** và **mọi scalar** đều là ASCII
//!   thuộc nhóm tương ứng (`char::is_ascii_digit`, `char::is_ascii_alphabetic`,
//!   `char::is_ascii_alphanumeric`, `char::is_ascii_whitespace`).
//! - Chuỗi **rỗng ⇒ `false`** cho cả bốn — **khác** `str.is_ascii("")` hiện hành ⇒ `true`
//!   (hợp đồng `is_ascii` là "mọi byte ASCII", không đòi khác rỗng).
//! - `digit` chỉ `0-9`; `alphabetic` chỉ `A-Z`/`a-z`; `alphanumeric` là hợp hai tập.
//!   Chữ số Unicode (Arabic-Indic `١`, fullwidth `１`) và chữ Unicode (`đ`, `Σ`, `á`) đều
//!   **không** thuộc nhóm ASCII ⇒ `false`.
//! - `whitespace` đúng **Rust ASCII whitespace** (probe `rustc 1.100.0`): space `0x20`,
//!   `\t` `0x09`, `\n` `0x0A`, `\r` `0x0D`, form feed `\x0C` ⇒ `true`; **VT `0x0B` ⇒
//!   `false`**; NBSP/ideographic space (khoảng trắng Unicode) ⇒ `false`.
//! - NUL `\0` **không** thuộc nhóm nào ⇒ `false` cả bốn; trộn ASCII hợp lệ + một scalar sai
//!   ⇒ `false`; non-ASCII (kể cả emoji/astral) ⇒ `false`.
//! - Không trap, không cấp phát, O(số scalar), dừng ở scalar sai đầu tiên.
//! - Chữ ký `(str) -> bool`; sugar R5c `s.is_ascii_digit()`, `s.is_ascii_alphabetic()`,
//!   `s.is_ascii_alphanumeric()`, `s.is_ascii_whitespace()` chạy **không** cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit;
//! ca âm chạy `vis check` + cả hai engine.
//!
//! **RED-BEFORE** (binary `target/debug/vis` dựng từ cây này @ `1734ab5`, **trước** khi thêm
//! bốn hàm):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_ascii_preds
//! running 11 tests
//! test rong_luon_false ... FAILED
//! test digit_chi_0_den_9 ... FAILED
//! test alphabetic_chi_a_z ... FAILED
//! test alphanumeric_hop_hai_tap ... FAILED
//! test whitespace_ascii_khong_co_vt ... FAILED
//! test tron_va_nhom_cheo ... FAILED
//! test unicode_nbsp_emoji_va_nul ... FAILED
//! test sugar_phuong_thuc_khong_can_use ... FAILED
//! test kieu_tra_bool_qua_bien_gan ... FAILED
//! test sai_kieu_bi_chan ... FAILED
//! test sai_so_doi_bi_chan ... FAILED
//!
//! test result: FAILED. 0 passed; 11 failed; 0 ignored; 0 measured; 0 filtered out
//! # ca dương: VS0301 5:11 module `std.str` không có hàm `is_ascii_digit` /
//! #   `is_ascii_alphabetic` / `is_ascii_alphanumeric` / `is_ascii_whitespace`
//! # sugar: VS0403 `str` không có trường `is_ascii_digit` / … (cả bốn)
//! # ca kiểu/arity cũng đỏ VS0301 thay vì VS0412/VS0401 vì hàm chưa tồn tại
//! ```
//!
//! Sau khi thêm: 11/11 xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-ap-{tag}-{}-{n}", std::process::id()));
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
// 1 — rỗng ⇒ false cả bốn (khác `str.is_ascii("")` ⇒ true)
// ===========================================================================

/// Chuỗi rỗng **không** thoả "không rỗng + mọi scalar thuộc nhóm" ⇒ `false` cả bốn;
/// `str.is_ascii("")` vẫn `true` (hợp đồng cũ giữ nguyên).
#[test]
fn rong_luon_false() {
    dung(
        "rong",
        r#"use std.str

fn main():
    print(str.is_ascii_digit(""))
    print(str.is_ascii_alphabetic(""))
    print(str.is_ascii_alphanumeric(""))
    print(str.is_ascii_whitespace(""))
    print(str.is_ascii(""))
"#,
        "false\nfalse\nfalse\nfalse\ntrue\n",
    );
}

// ===========================================================================
// 2 — digit: chỉ 0-9
// ===========================================================================

/// Chuỗi chỉ gồm `0-9` (kể cả nhiều chữ số) ⇒ `true`; chữ/dấu/khoảng trắng/chữ số Unicode
/// ⇒ `false`.
#[test]
fn digit_chi_0_den_9() {
    dung(
        "digit",
        r#"use std.str

fn main():
    print(str.is_ascii_digit("0"))
    print(str.is_ascii_digit("9"))
    print(str.is_ascii_digit("0123456789"))
    let s = "42"
    print(str.is_ascii_digit(s))
    print(str.is_ascii_digit("12a"))
    print(str.is_ascii_digit("a12"))
    print(str.is_ascii_digit("1 2"))
    print(str.is_ascii_digit(" 12"))
    print(str.is_ascii_digit("-1"))
    print(str.is_ascii_digit("1.0"))
    print(str.is_ascii_digit("!"))
    print(str.is_ascii_digit("١٢٣"))
    print(str.is_ascii_digit("１２３"))
"#,
        "true\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 3 — alphabetic: chỉ A-Z/a-z
// ===========================================================================

/// Chỉ chữ ASCII (cả hai hoa/thường) ⇒ `true`; có chữ số/dấu cách/dấu câu/chữ Unicode ⇒
/// `false`.
#[test]
fn alphabetic_chi_a_z() {
    dung(
        "alpha",
        r#"use std.str

fn main():
    print(str.is_ascii_alphabetic("a"))
    print(str.is_ascii_alphabetic("Z"))
    print(str.is_ascii_alphabetic("abcXYZ"))
    let s = "Hello"
    print(str.is_ascii_alphabetic(s))
    print(str.is_ascii_alphabetic("aZ0"))
    print(str.is_ascii_alphabetic("0a"))
    print(str.is_ascii_alphabetic("a-b"))
    print(str.is_ascii_alphabetic("a b"))
    print(str.is_ascii_alphabetic("_"))
    print(str.is_ascii_alphabetic("đ"))
    print(str.is_ascii_alphabetic("Σ"))
"#,
        "true\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 4 — alphanumeric: hợp digit + alphabetic
// ===========================================================================

/// Chỉ chữ/số ASCII (trộn tuỳ ý) ⇒ `true`; dấu cách/dấu câu/gạch dưới/chữ Unicode ⇒ `false`.
#[test]
fn alphanumeric_hop_hai_tap() {
    dung(
        "alnum",
        r#"use std.str

fn main():
    print(str.is_ascii_alphanumeric("abc123"))
    print(str.is_ascii_alphanumeric("A1z"))
    print(str.is_ascii_alphanumeric("0"))
    print(str.is_ascii_alphanumeric("z"))
    print(str.is_ascii_alphanumeric("ABCxyz019"))
    print(str.is_ascii_alphanumeric("abc-123"))
    print(str.is_ascii_alphanumeric("a_b"))
    print(str.is_ascii_alphanumeric("a b"))
    print(str.is_ascii_alphanumeric("abc!"))
    print(str.is_ascii_alphanumeric("đ1"))
    print(str.is_ascii_alphanumeric("١٢٣"))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 5 — whitespace ASCII đúng Rust: space, \t, \n, \r, FF; KHÔNG VT
// ===========================================================================

/// Bảng probe `rustc 1.100.0`: `" "`, `"\t"`, `"\n"`, `"\r"`, form feed `U+000C` ⇒ `true`;
/// **VT `U+000B` ⇒ `false`**; NBSP/ideographic space (Unicode) ⇒ `false`.
#[test]
fn whitespace_ascii_khong_co_vt() {
    dung(
        "ws",
        r#"use std.str

fn main():
    print(str.is_ascii_whitespace(" "))
    print(str.is_ascii_whitespace("\t"))
    print(str.is_ascii_whitespace("\n"))
    print(str.is_ascii_whitespace("\r"))
    print(str.is_ascii_whitespace("\u{000C}"))
    print(str.is_ascii_whitespace(" \t\r\n\u{000C}"))
    print(str.is_ascii_whitespace("\u{000B}"))
    print(str.is_ascii_whitespace("\u{000B} "))
    print(str.is_ascii_whitespace("a"))
    print(str.is_ascii_whitespace(" a"))
    print(str.is_ascii_whitespace("a "))
    print(str.is_ascii_whitespace("a b"))
    print(str.is_ascii_whitespace("\u{00A0}"))
    print(str.is_ascii_whitespace("\u{3000}"))
    let s = "\r\n\t"
    print(str.is_ascii_whitespace(s))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\ntrue\n",
    );
}

// ===========================================================================
// 6 — trộn ASCII hợp lệ + một scalar sai ⇒ false; nhóm chéo nhau
// ===========================================================================

/// Trộn đúng + sai ở đầu/giữa/cuối đều `false`; mỗi hàm từ chối nhóm khác (digit ⊄ alpha, …).
#[test]
fn tron_va_nhom_cheo() {
    dung(
        "tron",
        r#"use std.str

fn main():
    print(str.is_ascii_digit("123a"))
    print(str.is_ascii_digit("a123"))
    print(str.is_ascii_digit("12 3"))
    print(str.is_ascii_digit("123"))
    print(str.is_ascii_alphabetic("abC1"))
    print(str.is_ascii_alphabetic("1abC"))
    print(str.is_ascii_alphabetic("abC-"))
    print(str.is_ascii_alphabetic("abC"))
    print(str.is_ascii_alphanumeric("abc123!"))
    print(str.is_ascii_alphanumeric("abc 123"))
    print(str.is_ascii_alphanumeric("abc123"))
    print(str.is_ascii_whitespace(" \t\nx"))
    print(str.is_ascii_whitespace("x \t\n"))
    print(str.is_ascii_whitespace(" \t\n"))
    print(str.is_ascii_digit("abc"))
    print(str.is_ascii_alphabetic("123"))
    print(str.is_ascii_alphanumeric("!@#"))
    print(str.is_ascii_whitespace("\u{000C}\u{000B}"))
"#,
        "false\nfalse\nfalse\ntrue\nfalse\nfalse\nfalse\ntrue\nfalse\nfalse\ntrue\nfalse\nfalse\ntrue\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 7 — Unicode, NBSP, emoji, NUL: false cả bốn
// ===========================================================================

/// Chữ số/chữ Unicode, khoảng trắng Unicode, emoji/astral và NUL đều **không** thuộc nhóm
/// ASCII ⇒ `false`, kể cả khi trộn với ASCII hợp lệ.
#[test]
fn unicode_nbsp_emoji_va_nul() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.is_ascii_digit("đ"))
    print(str.is_ascii_alphabetic("đ"))
    print(str.is_ascii_alphanumeric("đ"))
    print(str.is_ascii_whitespace("đ"))
    print(str.is_ascii_digit("١"))
    print(str.is_ascii_alphabetic("Σ"))
    print(str.is_ascii_alphanumeric("１２３"))
    print(str.is_ascii_whitespace("\u{00A0}"))
    print(str.is_ascii_whitespace("\u{3000}"))
    print(str.is_ascii_digit("😀"))
    print(str.is_ascii_alphabetic("😀"))
    print(str.is_ascii_alphanumeric("😀"))
    print(str.is_ascii_whitespace("😀"))
    print(str.is_ascii_digit("\0"))
    print(str.is_ascii_alphabetic("\0"))
    print(str.is_ascii_alphanumeric("\0"))
    print(str.is_ascii_whitespace("\0"))
    print(str.is_ascii_digit("12\0"))
    print(str.is_ascii_alphabetic("a\0"))
    print(str.is_ascii_alphanumeric("a1\0"))
    print(str.is_ascii_whitespace(" \0"))
    print(str.is_ascii_digit("\0😀"))
"#,
        "false\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 8 — sugar phương thức R5c (không cần `use`)
// ===========================================================================

/// `s.is_ascii_*()` ≡ `str.is_ascii_*(s)` cho literal/biến, cả `true` lẫn `false`; trả `bool`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    print("123".is_ascii_digit())
    print("abc".is_ascii_alphabetic())
    print("abc123".is_ascii_alphanumeric())
    print(" \t\n\r\u{000C}".is_ascii_whitespace())
    print("".is_ascii_digit())
    print("12a".is_ascii_digit())
    print("\u{000B}".is_ascii_whitespace())
    print("\u{00A0}".is_ascii_whitespace())
    let s = "abc123"
    print(s.is_ascii_digit())
    print(s.is_ascii_alphabetic())
    print(s.is_ascii_alphanumeric())
    print(s.is_ascii_whitespace())
"#,
        "true\ntrue\ntrue\ntrue\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\ntrue\nfalse\n",
    );
}

// ===========================================================================
// 9 — kiểu trả bool qua biên gán (module + sugar)
// ===========================================================================

/// Cả bốn trả `bool` thật — gán có chú thích `bool` chạy sạch cả hai engine.
#[test]
fn kieu_tra_bool_qua_bien_gan() {
    dung(
        "kieu",
        r#"use std.str

fn main():
    let a: bool = str.is_ascii_digit("7")
    let b: bool = str.is_ascii_alphabetic("x")
    let c: bool = str.is_ascii_alphanumeric("x7")
    let d: bool = str.is_ascii_whitespace("\n")
    let e: bool = "9".is_ascii_digit()
    print(a)
    print(b)
    print(c)
    print(d)
    print(e)
"#,
        "true\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 10 — ca âm tĩnh: kiểu đối số / kiểu kết quả
// ===========================================================================

/// Sai kiểu đối số (`int` thay `str`) ⇒ `VS0412`; kết quả `bool` gán `int`/`str` ⇒ `VS0412`.
#[test]
fn sai_kieu_bi_chan() {
    for (tag, call) in [
        ("digit", "str.is_ascii_digit(1)"),
        ("alpha", "str.is_ascii_alphabetic(1)"),
        ("alnum", "str.is_ascii_alphanumeric(1)"),
        ("ws", "str.is_ascii_whitespace(1)"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0412");
    }
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.is_ascii_digit(\"1\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-str",
        "fn main():\n    let x: str = \"1\".is_ascii_digit()\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 11 — ca âm tĩnh: arity (module + sugar)
// ===========================================================================

/// Thiếu/thừa đối số ở lời gọi module và sugar ⇒ `VS0401` (không rơi xuống trap `VS0501`).
#[test]
fn sai_so_doi_bi_chan() {
    for (tag, call) in [
        ("digit-thieu", "str.is_ascii_digit()"),
        ("alpha-thua", "str.is_ascii_alphabetic(\"a\", \"b\")"),
        ("alnum-thieu", "str.is_ascii_alphanumeric()"),
        ("ws-thua", "str.is_ascii_whitespace(\" \", 1)"),
        ("sugar-digit-thua", "\"1\".is_ascii_digit(1)"),
        ("sugar-alpha-thua", "\"a\".is_ascii_alphabetic(\"b\")"),
        ("sugar-ws-thua", "\" \".is_ascii_whitespace(1)"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}
