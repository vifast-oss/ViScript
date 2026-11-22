// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (str-is-ascii)** — `std.str.is_ascii(s) -> bool`.
//!
//! Hợp đồng được khoá ở đây:
//! - `true` ⇔ **mọi byte** của `s` thuộc `0x00..=0x7F`; rỗng ⇒ `true`.
//! - Một scalar non-ASCII bất kỳ (tiếng Việt, emoji, astral `U+10FFFF`, dấu kết hợp) ⇒
//!   `false`; trộn ASCII + non-ASCII ⇒ `false`.
//! - `\0`, `\n`, `\t`, `\r` và DEL `0x7F` **là** ASCII ⇒ `true`; không trap, không cấp phát.
//! - Chữ ký `(str) -> bool`; sugar R5c `s.is_ascii()` chạy **không** cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (binary `target/debug/vis` dựng từ cây này @ `0eb6b32`, **trước** khi thêm
//! hàm):
//!
//! ```text
//! $ vis check probe.vis                    # use std.str, 5 lời gọi
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 4:11 module `std.str` không có hàm `is_ascii`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:4:11: error VS0301: module `std.str` không có hàm `is_ascii`
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `is_ascii`
//! probe.vis:6:11: error VS0301: module `std.str` không có hàm `is_ascii`
//! probe.vis:7:11: error VS0301: module `std.str` không có hàm `is_ascii`
//! probe.vis:8:11: error VS0301: module `std.str` không có hàm `is_ascii`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng năm dòng)
//! $ vis check sugar.vis                    # "abc".is_ascii(), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `is_ascii`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-ia-{tag}-{}-{n}", std::process::id()));
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
// 1 — rỗng + ASCII đủ loại: chữ, số, dấu câu, khoảng trắng, điều khiển
// ===========================================================================

/// `""`, chữ/số ASCII, dấu câu, khoảng trắng, `\0`, `\n`, `\t`, `\r` và DEL `0x7F` ⇒ `true`.
#[test]
fn rong_va_ascii_moi_loai() {
    dung(
        "ascii",
        r#"use std.str

fn main():
    print(str.is_ascii(""))
    print(str.is_ascii("abc"))
    print(str.is_ascii("ABCxyz"))
    print(str.is_ascii("0123456789"))
    print(str.is_ascii("!@#$%^&*()_+-=[]{{}};':\",./<>?`~|\\"))
    print(str.is_ascii(" \t\r\n"))
    print(str.is_ascii("\0"))
    print(str.is_ascii("\n"))
    print(str.is_ascii("\t"))
    print(str.is_ascii("\u{7f}"))
    print(str.is_ascii("a\0b\nc\u{7f}d"))
    let s = "abc"
    print(str.is_ascii(s))
    let u = "đ"
    print(str.is_ascii(u))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\nfalse\n",
    );
}

// ===========================================================================
// 2 — non-ASCII: tiếng Việt, emoji, astral, dấu kết hợp ⇒ false
// ===========================================================================

/// Mọi scalar > `0x7F` (kể cả astral/dấu kết hợp) ⇒ `false`; rỗng/ASCII vẫn `true` độc lập.
#[test]
fn non_ascii_va_astral_false() {
    dung(
        "non-ascii",
        r#"use std.str

fn main():
    print(str.is_ascii("đ"))
    print(str.is_ascii("xin chào"))
    print(str.is_ascii("😀"))
    print(str.is_ascii("👨‍👩‍👧"))
    print(str.is_ascii("e\u{301}"))
    print(str.is_ascii("\u{80}"))
    print(str.is_ascii("\u{10FFFF}"))
"#,
        "false\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 3 — trộn ASCII + non-ASCII ⇒ false (một byte non-ASCII là đủ)
// ===========================================================================

/// Chỉ **một** scalar non-ASCII ở đầu/giữa/cuối ⇒ `false`; ASCII hai bên không "cứu".
#[test]
fn tron_ascii_non_ascii_false() {
    dung(
        "tron",
        r#"use std.str

fn main():
    print(str.is_ascii("abcđ"))
    print(str.is_ascii("đabc"))
    print(str.is_ascii("a😀b"))
    print(str.is_ascii("abc def\nđ"))
    print(str.is_ascii("\0😀"))
    print(str.is_ascii("0\u{80}1"))
"#,
        "false\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 4 — sugar phương thức R5c (không cần `use`)
// ===========================================================================

/// `s.is_ascii()` ≡ `str.is_ascii(s)` cho literal/biến, cả `true` lẫn `false`; kiểu trả `bool`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    print("abc".is_ascii())
    print("".is_ascii())
    print("đ".is_ascii())
    print("a😀b".is_ascii())
    let s = "abc"
    print(s.is_ascii())
    let t = "đ😀"
    print(t.is_ascii())
    let b: bool = s.is_ascii()
    print(b)
"#,
        "true\ntrue\nfalse\nfalse\ntrue\nfalse\ntrue\n",
    );
}

// ===========================================================================
// 5 — kiểu trả bool qua biên gán (module + sugar)
// ===========================================================================

/// `str.is_ascii` và sugar trả `bool` thật — gán có chú thích `bool` chạy sạch cả hai engine.
#[test]
fn kieu_tra_bool_qua_bien_gan() {
    dung(
        "kieu",
        r#"use std.str

fn main():
    let a: bool = str.is_ascii("")
    let b: bool = str.is_ascii("đ")
    let c: bool = "abc".is_ascii()
    print(a)
    print(b)
    print(c)
"#,
        "true\nfalse\ntrue\n",
    );
}

// ===========================================================================
// 6 — ca âm tĩnh: kiểu đối số / kiểu kết quả
// ===========================================================================

/// Sai kiểu đối số (`int` thay `str`) ⇒ `VS0412`; kết quả `bool` gán `int`/`str` ⇒ `VS0412`.
#[test]
fn sai_kieu_bi_chan() {
    tu_choi(
        "sai-kieu-doi-so",
        "use std.str\n\nfn main():\n    print(str.is_ascii(1))\n",
        "VS0412",
    );
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.is_ascii(\"a\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-str",
        "fn main():\n    let x: str = \"a\".is_ascii()\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: arity (module + sugar)
// ===========================================================================

/// Thiếu/thừa đối số ở lời gọi module và sugar ⇒ `VS0401` (không rơi xuống trap `VS0501`).
#[test]
fn sai_so_doi_bi_chan() {
    for (tag, call) in [
        ("thieu", "str.is_ascii()"),
        ("thua", "str.is_ascii(\"a\", \"b\")"),
        ("sugar-thua", "\"a\".is_ascii(1)"),
        ("sugar-thua-str", "\"a\".is_ascii(\"b\")"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}
