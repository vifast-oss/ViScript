// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (replace-first)** — `std.str.replace_first(s, from, to) -> str`.
//!
//! Hợp đồng được khoá ở đây:
//! - Thay **đúng lần xuất hiện đầu tiên** của `from` trong `s` bằng `to`, giữ nguyên mọi lần
//!   xuất hiện sau: `replace_first("aaa", "a", "b")` ⇒ `"baa"`,
//!   `replace_first("banana", "na", "X")` ⇒ `"baXna"`.
//! - Không khớp ⇒ `s` **nguyên vẹn** (không cấp phát kết quả khác).
//! - `from` rỗng ⇒ **chèn** `to` một lần ở **đầu** (Rust `replacen(…, 1)`):
//!   `replace_first("abc", "", "-")` ⇒ `"-abc"`, `replace_first("", "", "-")` ⇒ `"-"`.
//! - `s` rỗng hoặc `to` rỗng vẫn theo cùng quy tắc: `replace_first("", "a", "b")` ⇒ `""`,
//!   `replace_first("abc", "b", "")` ⇒ `"ac"`.
//! - Đơn vị khớp là **scalar Unicode** (cùng `str.replace`/`find`, không phải byte):
//!   `replace_first("đ😀x😀", "😀", "Z")` ⇒ `"đZx😀"`; mẫu nhiều scalar
//!   `replace_first("đ😀x", "đ😀", "Q")` ⇒ `"Qx"`; không bao giờ cắt giữa scalar.
//! - Chồng lấn: chỉ thay **một** lần, không lặp: `replace_first("aaa", "aa", "b")` ⇒ `"ba"`,
//!   `replace_first("aaaa", "aa", "b")` ⇒ `"baa"`.
//! - Kết quả là **`str`** (không phải `str?`); sugar R5c `s.replace_first(from, to)` ≡
//!   `str.replace_first(s, from, to)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `e17998f`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.str
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.str` không có hàm `replace_first`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `replace_first`
//! probe.vis:6:11: error VS0301: module `std.str` không có hàm `replace_first`
//! $ vis check sugar.vis                    # "banana".replace_first("na", "X"), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `str` không có trường `replace_first`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-rf-{tag}-{}-{n}", std::process::id()));
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
// 1 — chỉ thay lần đầu, giữ mọi lần sau
// ===========================================================================

/// Nhiều lần xuất hiện ⇒ chỉ lần **đầu** đổi; phần còn lại nguyên văn.
#[test]
fn chi_thay_lan_dau() {
    dung(
        "first-only",
        r#"use std.str

fn main():
    print(str.replace_first("aaa", "a", "b"))
    print(str.replace_first("banana", "na", "X"))
    print(str.replace_first("ababab", "ab", "X"))
    print(str.replace_first("hello world hello", "hello", "hi"))
    print(str.replace_first("x.y.z", ".", "/"))
    print(str.replace_first("aaaa", "a", "bb"))
"#,
        "baa\nbaXna\nXabab\nhi world hello\nx/y.z\nbbaaa\n",
    );
}

// ===========================================================================
// 2 — không khớp ⇒ nguyên vẹn
// ===========================================================================

/// `from` không có trong `s` (kể cả dài hơn `s`) ⇒ trả đúng `s`.
#[test]
fn khong_khop_tra_nguyen_ven() {
    dung(
        "no-match",
        r#"use std.str

fn main():
    print(str.replace_first("abc", "z", "X"))
    print(str.replace_first("abc", "abcd", "X"))
    print(str.replace_first("", "a", "b"))
    print(str.replace_first("đ😀x", "d", "D"))
    print(str.replace_first("abc", "ABC", "X"))
"#,
        "abc\nabc\n\nđ😀x\nabc\n",
    );
}

// ===========================================================================
// 3 — `from` rỗng: chèn `to` một lần ở đầu
// ===========================================================================

/// Nghĩa Rust `replacen(…, 1)`: mẫu rỗng khớp ở vị trí 0 ⇒ `to + s`, kể cả `s` rỗng.
#[test]
fn from_rong_chen_dau_mot_lan() {
    dung(
        "empty-from",
        r#"use std.str

fn main():
    print(str.replace_first("abc", "", "-"))
    print(str.replace_first("", "", "-"))
    print(str.replace_first("abc", "", ""))
    print(str.replace_first("", "", ""))
    print(str.replace_first("đ", "", "😀"))
    print(str.replace_first("😀", "", "đ"))
"#,
        "-abc\n-\nabc\n\n😀đ\nđ😀\n",
    );
}

// ===========================================================================
// 4 — `s` rỗng / `to` rỗng
// ===========================================================================

/// `s` rỗng với `from` khác rỗng ⇒ `""`; `to` rỗng ⇒ **xoá** đúng lần khớp đầu.
#[test]
fn s_rong_va_to_rong() {
    dung(
        "empty-s-to",
        r#"use std.str

fn main():
    print(str.replace_first("", "a", "b"))
    print(str.replace_first("abc", "b", ""))
    print(str.replace_first("abc", "abc", ""))
    print(str.replace_first("banana", "na", ""))
    print(str.replace_first("a", "a", ""))
    print(str.replace_first("đ😀", "😀", ""))
"#,
        "\nac\n\nbana\n\nđ\n",
    );
}

// ===========================================================================
// 5 — Unicode: scalar, không phải byte
// ===========================================================================

/// Tiếng Việt/emoji: khớp theo **scalar**; kết quả không bao giờ là UTF-8 hỏng.
#[test]
fn unicode_theo_scalar_khong_phai_byte() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.replace_first("đ😀x😀", "😀", "Z"))
    print(str.replace_first("đađ", "đ", "Đ"))
    print(str.replace_first("😀😀😀", "😀", "🙂"))
    print(str.replace_first("đ😀x", "đ😀", "Q"))
    print(str.replace_first("đ😀x", "x", "đ"))
    print(str.replace_first("👨‍👩‍👧x", "👩", "M"))
"#,
        "đZx😀\nĐađ\n🙂😀😀\nQx\nđ😀đ\n👨‍M‍👧x\n",
    );
}

// ===========================================================================
// 6 — chồng lấn: chỉ một lần
// ===========================================================================

/// `"aaa"` thay `"aa"` ⇒ `"ba"` (không lặp); `"aaaa"` ⇒ `"baa"`; `to` dài hơn cũng chỉ một lần.
#[test]
fn chong_lan_chi_mot_lan() {
    dung(
        "overlap",
        r#"use std.str

fn main():
    print(str.replace_first("aaa", "aa", "b"))
    print(str.replace_first("aaaa", "aa", "b"))
    print(str.replace_first("aaa", "a", "aa"))
    print(str.replace_first("aaaa", "aaa", "b"))
    print(str.replace_first("abab", "aba", "X"))
"#,
        "ba\nbaa\naaaa\nba\nXb\n",
    );
}

// ===========================================================================
// 7 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.replace_first(…)` ≡ `str.replace_first(s, …)`; kết quả vẫn là `str`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "banana"
    print(s.replace_first("na", "X"))
    print("aaa".replace_first("aa", "b"))
    print("abc".replace_first("", "-"))
    print(s.replace_first("z", "X"))
    let t: str = s.replace_first("ban", "BAN")
    print(t)
    print("đ😀x😀".replace_first("😀", "Z"))
    print("aaaa".replace_first("aa", "b"))
"#,
        "baXna\nba\n-abc\nbanana\nBANana\nđZx😀\nbaa\n",
    );
}

// ===========================================================================
// 8 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.replace_first(1, \"a\", \"b\")", "VS0412"),
        ("kieu-2", "str.replace_first(\"a\", 1, \"b\")", "VS0412"),
        ("kieu-3", "str.replace_first(\"a\", \"b\", 1)", "VS0412"),
        ("arity-1", "str.replace_first(\"a\", \"b\")", "VS0401"),
        ("arity-2", "str.replace_first(\"a\")", "VS0401"),
        ("arity-3", "str.replace_first()", "VS0401"),
        (
            "arity-4",
            "str.replace_first(\"a\", \"b\", \"c\", \"d\")",
            "VS0401",
        ),
        ("sugar-kieu", "\"a\".replace_first(1, \"b\")", "VS0412"),
        ("sugar-kieu-2", "\"a\".replace_first(\"b\", 1)", "VS0412"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `str` thật: gán vào `str` hợp lệ (ca dương ở test 7); gán vào `int` bị chặn.
#[test]
fn ket_qua_la_str_that() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.replace_first(\"abc\", \"b\", \"X\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"abc\".replace_first(\"b\", \"X\")\n    print(x)\n",
        "VS0412",
    );
}
