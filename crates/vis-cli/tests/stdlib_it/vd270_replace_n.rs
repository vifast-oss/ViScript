// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (replace-n)** — `std.str.replace_n(s, from, to, n) -> str`.
//!
//! Hợp đồng được khoá ở đây:
//! - Thay **`n` lần khớp KHÔNG chồng đầu tiên** của `from` trong `s` bằng `to`, quét
//!   **trái → phải** — đúng nghĩa Rust `str::replacen(from, to, n)`; cùng đơn vị khớp
//!   **scalar Unicode** với `str.replace`/`replace_first`/`replace_last`/`find`:
//!   `replace_n("aaaa", "a", "b", 2)` ⇒ `"bbaa"`, `replace_n("banana", "na", "X", 2)`
//!   ⇒ `"baXX"`.
//! - `n == 0` ⇒ `s` **nguyên vẹn**; `n` lớn hơn số lần khớp ⇒ thay **tất cả** (như
//!   `str.replace`), kể cả `n = i64::MAX`; không khớp ⇒ `s` nguyên vẹn.
//! - `from` rỗng theo **đúng nghĩa Rust `replacen`**: khớp rỗng tại **mọi biên scalar**
//!   (kể cả đầu/cuối), chèn `to` ở `n` lần khớp rỗng đầu tiên — bảng kiểm bằng probe Rust:
//!
//!   | biểu thức | kết quả |
//!   |---|---|
//!   | `"abc".replacen("", "-", 0)` | `"abc"` |
//!   | `"abc".replacen("", "-", 1)` | `"-abc"` |
//!   | `"abc".replacen("", "-", 2)` | `"-a-bc"` |
//!   | `"abc".replacen("", "-", 3)` | `"-a-b-c"` |
//!   | `"abc".replacen("", "-", 4)` | `"-a-b-c-"` |
//!   | `"abc".replacen("", "-", 9)` | `"-a-b-c-"` |
//!   | `"".replacen("", "-", 1)` | `"-"` |
//!   | `"đ".replacen("", "😀", 2)` | `"😀đ😀"` |
//!
//! - `s` rỗng hoặc `to` rỗng vẫn theo cùng quy tắc: `replace_n("", "a", "b", 3)` ⇒ `""`,
//!   `replace_n("banana", "na", "", 2)` ⇒ `"ba"`.
//! - Chồng lấn: **không** chồng — sau mỗi lần thay quét tiếp **sau** phần `from` vừa khớp:
//!   `replace_n("aaaa", "aa", "b", 2)` ⇒ `"bb"`, `replace_n("aaa", "aa", "b", 9)` ⇒ `"ba"`.
//! - `n < 0` là trap `VS0501` nêu `str.replace_n`, exit 101 ở **cả hai** engine.
//! - Kết quả là **`str`** (không phải `str?`); sugar R5c `s.replace_n(from, to, n)` ≡
//!   `str.replace_n(s, from, to, n)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `ff780d4`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.str
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.str` không có hàm `replace_n`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `replace_n`
//! probe.vis:6:11: error VS0301: module `std.str` không có hàm `replace_n`
//! $ vis check sugar.vis                    # "banana".replace_n("na", "X", 1), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `str` không có trường `replace_n`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-rn-{tag}-{}-{n}", std::process::id()));
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
        "{tag}: stderr phải nêu `{needle}`: {err}"
    );
}

// ===========================================================================
// 1 — n = 0/1/2/tất cả/quá số lần
// ===========================================================================

/// `n` chặn số lần thay từ trái sang phải; `n == 0` ⇒ nguyên vẹn; `n` quá số lần ⇒ thay tất cả.
#[test]
fn n_0_1_2_tat_ca_va_qua_so_lan() {
    dung(
        "n-bounded",
        r#"use std.str

fn main():
    print(str.replace_n("aaaa", "a", "b", 0))
    print(str.replace_n("aaaa", "a", "b", 1))
    print(str.replace_n("aaaa", "a", "b", 2))
    print(str.replace_n("aaaa", "a", "b", 3))
    print(str.replace_n("aaaa", "a", "b", 4))
    print(str.replace_n("aaaa", "a", "b", 5))
    print(str.replace_n("banana", "na", "X", 1))
    print(str.replace_n("banana", "na", "X", 2))
    print(str.replace_n("banana", "na", "X", 99))
    print(str.replace_n("ababab", "ab", "X", 2))
    print(str.replace_n("x.y.z", ".", "/", 1))
    print(str.replace_n("x.y.z", ".", "/", 2))
    let n: int = 2
    print(str.replace_n("aaaa", "a", "b", n))
    print(str.replace_n("aaaa", "a", "b", 9223372036854775807))
"#,
        "aaaa\nbaaa\nbbaa\nbbba\nbbbb\nbbbb\nbaXna\nbaXX\nbaXX\nXXab\nx/y.z\nx/y/z\nbbaa\nbbbb\n",
    );
}

// ===========================================================================
// 2 — không khớp ⇒ nguyên vẹn
// ===========================================================================

/// `from` không có trong `s` (kể cả dài hơn `s`) ⇒ trả đúng `s`, không phụ thuộc `n`.
#[test]
fn khong_khop_tra_nguyen_ven() {
    dung(
        "no-match",
        r#"use std.str

fn main():
    print(str.replace_n("abc", "z", "X", 0))
    print(str.replace_n("abc", "z", "X", 2))
    print(str.replace_n("abc", "abcd", "X", 5))
    print(str.replace_n("", "a", "b", 3))
    print(str.replace_n("đ😀x", "d", "D", 1))
    print(str.replace_n("abc", "ABC", "X", 9))
"#,
        "abc\nabc\nabc\n\nđ😀x\nabc\n",
    );
}

// ===========================================================================
// 3 — `from` rỗng: bảng đúng nghĩa Rust `replacen`
// ===========================================================================

/// Khớp rỗng tại **mọi biên scalar**; `n` lần đầu được chèn `to` — bảng probe Rust.
#[test]
fn from_rong_bang_rust() {
    dung(
        "empty-from",
        r#"use std.str

fn main():
    print(str.replace_n("abc", "", "-", 0))
    print(str.replace_n("abc", "", "-", 1))
    print(str.replace_n("abc", "", "-", 2))
    print(str.replace_n("abc", "", "-", 3))
    print(str.replace_n("abc", "", "-", 4))
    print(str.replace_n("abc", "", "-", 9))
    print(str.replace_n("", "", "-", 1))
    print(str.replace_n("", "", "-", 9))
    print(str.replace_n("abc", "", "", 2))
    print(str.replace_n("đ", "", "😀", 2))
    print(str.replace_n("😀", "", "đ", 2))
"#,
        "abc\n-abc\n-a-bc\n-a-b-c\n-a-b-c-\n-a-b-c-\n-\n-\nabc\n😀đ😀\nđ😀đ\n",
    );
}

// ===========================================================================
// 4 — `s` rỗng / `to` rỗng
// ===========================================================================

/// `s` rỗng với `from` khác rỗng ⇒ `""`; `to` rỗng ⇒ **xoá** tối đa `n` lần khớp.
#[test]
fn s_rong_va_to_rong() {
    dung(
        "empty-s-to",
        r#"use std.str

fn main():
    print(str.replace_n("", "a", "b", 1))
    print(str.replace_n("abc", "b", "", 1))
    print(str.replace_n("abc", "b", "", 2))
    print(str.replace_n("banana", "na", "", 2))
    print(str.replace_n("banana", "na", "", 1))
    print(str.replace_n("a", "a", "", 1))
    print(str.replace_n("đ😀", "😀", "", 5))
"#,
        "\nac\nac\nba\nbana\n\nđ\n",
    );
}

// ===========================================================================
// 5 — chồng lấn: khớp không chồng, quét tiếp sau phần vừa khớp
// ===========================================================================

/// `"aaaa"` thay `"aa"` hai lần ⇒ `"bb"` (khớp tại 0 và 2); `"aaa"` chỉ có **một** khớp
/// không chồng ⇒ `"ba"` dù `n` lớn; `to` dài hơn không tạo khớp mới trong cùng lượt.
#[test]
fn chong_lan_khong_chong() {
    dung(
        "overlap",
        r#"use std.str

fn main():
    print(str.replace_n("aaaa", "aa", "b", 1))
    print(str.replace_n("aaaa", "aa", "b", 2))
    print(str.replace_n("aaaa", "aa", "b", 3))
    print(str.replace_n("aaa", "aa", "b", 2))
    print(str.replace_n("ababa", "aba", "X", 2))
    print(str.replace_n("aaa", "a", "aa", 1))
    print(str.replace_n("aaa", "a", "aa", 2))
    print(str.replace_n("aaa", "a", "aa", 9))
    print(str.replace_n("aaa", "aa", "", 9))
"#,
        "baa\nbb\nbb\nba\nXba\naaaa\naaaaa\naaaaaa\na\n",
    );
}

// ===========================================================================
// 6 — Unicode/emoji: scalar, không phải byte
// ===========================================================================

/// Tiếng Việt/emoji: khớp theo **scalar**; không bao giờ cắt giữa scalar hay ZWJ.
#[test]
fn unicode_emoji() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.replace_n("đ😀x😀", "😀", "Z", 1))
    print(str.replace_n("đ😀x😀", "😀", "Z", 2))
    print(str.replace_n("đađ", "đ", "Đ", 2))
    print(str.replace_n("😀😀😀", "😀", "🙂", 2))
    print(str.replace_n("đ😀x", "đ😀", "Q", 1))
    print(str.replace_n("đ😀x", "x", "đ", 1))
    print(str.replace_n("👨‍👩‍👧x", "👩", "M", 1))
    print(str.replace_n("đ😀xđ😀y", "đ😀", "Q", 2))
"#,
        "đZx😀\nđZxZ\nĐaĐ\n🙂🙂😀\nQx\nđ😀đ\n👨‍M‍👧x\nQxQy\n",
    );
}

// ===========================================================================
// 7 — `n < 0` ⇒ trap VS0501 nêu `str.replace_n` ở cả hai engine
// ===========================================================================

/// `n` âm là lỗi **chạy** (không phải lỗi kiểu): trap `VS0501`, exit 101, stderr nêu hàm.
#[test]
fn n_am_la_trap_ca_hai_engine() {
    trap_ca_hai(
        "n-am",
        r#"use std.str

fn main():
    print(str.replace_n("abc", "a", "b", -1))
"#,
        "str.replace_n",
    );
    trap_ca_hai(
        "n-am-sugar",
        r#"fn main():
    print("abc".replace_n("a", "b", -2))
"#,
        "str.replace_n",
    );
    trap_ca_hai(
        "n-am-bien",
        r#"use std.str

fn main():
    let n: int = 0 - 7
    print(str.replace_n("aaaa", "a", "b", n))
"#,
        "str.replace_n",
    );
}

// ===========================================================================
// 8 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.replace_n(…)` ≡ `str.replace_n(s, …)`; kết quả vẫn là `str`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "banana"
    print(s.replace_n("na", "X", 1))
    print(s.replace_n("na", "X", 2))
    print("aaaa".replace_n("a", "b", 2))
    print("abc".replace_n("", "-", 2))
    print(s.replace_n("z", "X", 3))
    let t: str = s.replace_n("ban", "BAN", 1)
    print(t)
    print("đ😀x😀".replace_n("😀", "Z", 2))
    print("aaaa".replace_n("aa", "b", 2))
"#,
        "baXna\nbaXX\nbbaa\n-a-bc\nbanana\nBANana\nđZxZ\nbb\n",
    );
}

// ===========================================================================
// 9 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số (kể cả `n` phải `int`).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.replace_n(1, \"a\", \"b\", 1)", "VS0412"),
        ("kieu-2", "str.replace_n(\"a\", 1, \"b\", 1)", "VS0412"),
        ("kieu-3", "str.replace_n(\"a\", \"b\", 1, 1)", "VS0412"),
        (
            "kieu-4",
            "str.replace_n(\"a\", \"b\", \"c\", \"1\")",
            "VS0412",
        ),
        ("arity-1", "str.replace_n(\"a\", \"b\", \"c\")", "VS0401"),
        ("arity-2", "str.replace_n(\"a\", \"b\")", "VS0401"),
        ("arity-3", "str.replace_n(\"a\")", "VS0401"),
        ("arity-4", "str.replace_n()", "VS0401"),
        (
            "arity-5",
            "str.replace_n(\"a\", \"b\", \"c\", 1, 2)",
            "VS0401",
        ),
        ("sugar-kieu", "\"a\".replace_n(1, \"b\", 1)", "VS0412"),
        ("sugar-kieu-2", "\"a\".replace_n(\"b\", 1, 1)", "VS0412"),
        (
            "sugar-kieu-3",
            "\"a\".replace_n(\"b\", \"c\", \"1\")",
            "VS0412",
        ),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `str` thật: gán vào `str` hợp lệ (ca dương ở test 8); gán vào `int` bị chặn.
#[test]
fn ket_qua_la_str_that() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.replace_n(\"abc\", \"b\", \"X\", 1)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"abc\".replace_n(\"b\", \"X\", 1)\n    print(x)\n",
        "VS0412",
    );
}
