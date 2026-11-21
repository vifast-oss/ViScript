// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (matches)** — `std.str.matches(s, pat) -> list[str]`.
//!
//! Hợp đồng khoá ở đây, **đúng** `str::matches` của Rust:
//! - Trả **các chuỗi con** khớp `pat`, theo thứ tự **trái → phải**, khớp **không chồng**:
//!   `matches("banana", "ana")` ⇒ `["ana"]` (không khớp lại phần đã dùng).
//! - `"aaaa"` cắt `"aa"` ⇒ `["aa", "aa"]` (2 lần); `"aaa"` cắt `"aa"` ⇒ `["aa"]` (1 lần —
//!   không chồng); `"abab"` cắt `"aba"` ⇒ `["aba"]`; `"abc"` cắt `"abcd"`/`"x"` ⇒ `[]`.
//! - `pat` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu/cuối ⇒
//!   `"abc"` ⇒ `["", "", "", ""]` (`len` = 4); `""` + `""` ⇒ `[""]` (`len` = 1, **khác** `[]`).
//! - `s` rỗng + `pat` khác rỗng ⇒ `[]`, không trap.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.find`/`str.split`):
//!   `matches("đ😀x😀", "😀")` ⇒ `["😀", "😀"]`; ZWJ không phải biên đặc biệt:
//!   `matches("👨‍👩‍👧x", "👩")` ⇒ `["👩"]`, cả cụm ZWJ vẫn khớp nguyên vẹn.
//! - Chữ ký `(str, str) -> list[str]`; sugar R5c `s.matches(pat)` ≡ `str.matches(s, pat)`,
//!   không cần `use std.str`. Không đụng `find`/`rfind`/`count`/`split`.
//!
//! Bảng probe `rustc 1.100.0` (toolchain ghim):
//!
//! | biểu thức | kết quả | `len` |
//! |---|---|---|
//! | `"banana".matches("ana")` | `["ana"]` | 1 |
//! | `"aaaa".matches("aa")` | `["aa", "aa"]` | 2 |
//! | `"abab".matches("aba")` | `["aba"]` | 1 |
//! | `"abc".matches("abcd")` | `[]` | 0 |
//! | `"abc".matches("x")` | `[]` | 0 |
//! | `"".matches("")` | `[""]` | 1 |
//! | `"abc".matches("")` | `["", "", "", ""]` | 4 |
//! | `"đ😀x😀".matches("😀")` | `["😀", "😀"]` | 2 |
//! | `"👨‍👩‍👧‍👦x👨‍👩‍👧‍👦".matches("👨‍👩‍👧‍👦")` | `["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]` | 2 |
//! | `"👨‍👩‍👧x".matches("👩")` | `["👩"]` | 1 |
//! | `"đ😀x😀".matches("")` | `["", "", "", "", ""]` | 5 |
//! | `"aaa".matches("aa")` | `["aa"]` | 1 |
//! | `"ababa".matches("aba")` | `["aba"]` | 1 |
//! | `"abcabc".matches("abc")` | `["abc", "abc"]` | 2 |
//! | `"😀😀😀".matches("😀😀")` | `["😀😀"]` | 1 |
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (binary `vis` của cây này **trước** khi thêm hàm; `cargo test -p vis-cli
//! --test vd270_matches`): **0 passed / 9 failed** — mọi ca dương đỏ `VS0301` «module
//! `std.str` không có hàm `matches`», sugar đỏ `VS0403` «`str` không có trường `matches`», ca
//! kiểu/arity cũng đỏ `VS0301` thay vì `VS0412`/`VS0401`:
//!
//! ```text
//! $ vis run --no-cache mat-bang.vis
//! mat-bang.vis:5:15: error VS0301: module `std.str` không có hàm `matches`
//! ...
//! $ vis run --no-cache mat-sugar.vis
//! mat-sugar.vis:5:15: error VS0403: `str` không có trường `matches`
//! ```
//!
//! Sau khi thêm: **9/9 xanh**, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-mat-{tag}-{}-{n}", std::process::id()));
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
// 1 — bảng probe Rust 1.100.0: khớp không chồng, trả chuỗi con
// ===========================================================================

/// Bảng probe chính: `banana`/`ana`, `aaaa`/`aa`, `abab`/`aba`, không khớp, `pat` dài hơn `s`.
#[test]
fn bang_probe_rust_khong_chong() {
    dung(
        "mat-bang",
        r#"use std.str

fn main():
    print(str.matches("banana", "ana"))
    print(str.matches("aaaa", "aa"))
    print(str.matches("abab", "aba"))
    print(str.matches("abc", "abcd"))
    print(str.matches("abc", "x"))
    print(str.matches("aaa", "aa"))
    print(str.matches("ababa", "aba"))
    print(str.matches("abcabc", "abc"))
    print(str.matches("😀😀😀", "😀😀"))
    print(str.join(str.matches("banana", "ana"), "|"))
    print(str.join(str.matches("aaaa", "aa"), "|"))
"#,
        "[ana]\n[aa, aa]\n[aba]\n[]\n[]\n[aa]\n[aba]\n[abc, abc]\n[😀😀]\nana\naa|aa\n",
    );
}

// ===========================================================================
// 2 — `pat` rỗng và `s` rỗng: đúng nghĩa Rust, phân biệt `[""]` với `[]`
// ===========================================================================

/// `pat` rỗng khớp tại **mọi biên scalar** (`len = số scalar + 1`); `"" + ""` ⇒ **một** phần tử
/// rỗng; `s` rỗng + `pat` khác rỗng ⇒ `[]`. Dùng `len` + `join` vì `[]` và `[""]` in giống nhau.
#[test]
fn pat_rong_va_s_rong_theo_nghia_rust() {
    dung(
        "mat-rong",
        r#"use std.str

fn main():
    let a = str.matches("", "")
    print(len(a))
    print(str.join(a, "|"))
    let b = str.matches("abc", "")
    print(len(b))
    print(str.join(b, "|"))
    let c = str.matches("", "a")
    print(len(c))
    print(str.join(c, "|"))
    let d = str.matches("đ😀x😀", "")
    print(len(d))
    print(str.join(d, "|"))
"#,
        "1\n\n4\n|||\n0\n\n5\n||||\n",
    );
}

// ===========================================================================
// 3 — không khớp / `pat` dài hơn `s` / `s` rỗng, không trap
// ===========================================================================

/// Không khớp ở mọi vị trí ⇒ `[]` (không phải `[s]` như `split`); `pat` dài hơn `s` cũng `[]`;
/// chuỗi rỗng với `pat` khác rỗng ⇒ `[]`.
#[test]
fn khong_khop_tra_list_rong() {
    dung(
        "mat-khong",
        r#"use std.str

fn main():
    print(str.matches("abc", "z"))
    print(str.matches("abc", "abcd"))
    print(str.matches("", "a"))
    print(len(str.matches("abc", "z")))
    print(len(str.matches("", "a")))
    print(str.join(str.matches("abc", "z"), "|"))
"#,
        "[]\n[]\n[]\n0\n0\n\n",
    );
}

// ===========================================================================
// 4 — chồng lấn: khớp **không chồng**, không quay lại phần đã dùng
// ===========================================================================

/// Khớp không chồng: `"aaaa"` cắt `"aa"` ⇒ 2, `"aaa"` cắt `"aa"` ⇒ 1, `"ababa"` cắt `"aba"` ⇒ 1,
/// `"abcabc"` cắt `"abc"` ⇒ 2. Khoá cả `count` không hồi quy nếu có (`count` vẫn là `int`).
#[test]
fn chong_lan_khong_khop_lai() {
    dung(
        "mat-chong",
        r#"use std.str

fn main():
    print(len(str.matches("aaaa", "aa")))
    print(len(str.matches("aaa", "aa")))
    print(len(str.matches("ababa", "aba")))
    print(len(str.matches("abcabc", "abc")))
    print(len(str.matches("aaaaa", "aa")))
    print(str.matches("aaaaa", "aa"))
    print(str.count("aaaa", "aa"))
"#,
        "2\n1\n1\n2\n2\n[aa, aa]\n2\n",
    );
}

// ===========================================================================
// 5 — Unicode/emoji/ZWJ: khớp chuỗi con UTF-8 trên biên scalar
// ===========================================================================

/// Tiếng Việt + emoji: `"đ😀x😀"` cắt `"😀"` ⇒ 2; ZWJ **không** phải biên đặc biệt; cụm ZWJ
/// nguyên vẹn vẫn khớp; không sinh UTF-8 hỏng.
#[test]
fn unicode_emoji_zwj() {
    dung(
        "mat-unicode",
        r#"use std.str

fn main():
    print(str.matches("đ😀x😀", "😀"))
    print(str.join(str.matches("đ😀x😀", "😀"), "|"))
    print(str.matches("a😀b😀c", "😀"))
    print(str.matches("👨‍👩‍👧x", "👩"))
    print(str.join(str.matches("👨‍👩‍👧x", "👩"), "|"))
    print(str.matches("👨‍👩‍👧‍👦x👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"))
    print(str.join(str.matches("👨‍👩‍👧‍👦x👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"), "|"))
    print(len(str.matches("đ😀x😀", "")))
"#,
        "[😀, 😀]\n😀|😀\n[😀, 😀]\n[👩]\n👩\n[👨‍👩‍👧‍👦, 👨‍👩‍👧‍👦]\n👨‍👩‍👧‍👦|👨‍👩‍👧‍👦\n5\n",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.matches(pat)` ≡ `str.matches(s, pat)`; kết quả là `list[str]` thật; không cần `use`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "mat-sugar",
        r#"fn main():
    let s: str = "banana"
    print(s.matches("ana"))
    print("aaaa".matches("aa"))
    let t: list[str] = s.matches("ana")
    print(t)
    print(t.len())
    print(t[0])
    for m in "a😀b😀c".matches("😀"):
        print(m)
    print("abc".matches(""))
    print(len("abc".matches("")))
    print("".matches(""))
    print(len("".matches("")))
"#,
        "[ana]\n[aa, aa]\n[ana]\n1\nana\n😀\n😀\n[, , , ]\n4\n[]\n1\n",
    );
}

// ===========================================================================
// 7 — `list[str]` thật qua biên gán/chỉ số/`for`; `count` giữ nguyên
// ===========================================================================

/// Kết quả gán `list[str]` hợp lệ, truy cập chỉ số/`len`/`for` được; gán `str` ⇒ `VS0412`.
#[test]
fn ket_qua_la_list_str_that() {
    dung(
        "mat-kieu",
        r#"use std.str

fn main():
    let xs: list[str] = str.matches("abcabc", "abc")
    print(xs)
    print(len(xs))
    print(xs[0])
    print(xs[1])
    for x in xs:
        print(x)
    print(str.matches("abc", "abc"))
"#,
        "[abc, abc]\n2\nabc\nabc\nabc\nabc\n[abc]\n",
    );
    tu_choi(
        "mat-kieu-sai",
        "use std.str\n\nfn main():\n    let x: str = str.matches(\"abc\", \"a\")\n    print(x)\n",
        "VS0412",
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
        ("kieu-1", "str.matches(1, \"a\")", "VS0412"),
        ("kieu-2", "str.matches(\"a\", 1)", "VS0412"),
        ("arity-1", "str.matches(\"a\")", "VS0401"),
        ("arity-2", "str.matches()", "VS0401"),
        ("arity-3", "str.matches(\"a\", \"b\", \"c\")", "VS0401"),
        ("sugar-kieu", "\"a\".matches(1)", "VS0412"),
        ("sugar-arity-1", "\"a\".matches()", "VS0401"),
        ("sugar-arity-2", "\"a\".matches(\"b\", 1)", "VS0401"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

// ===========================================================================
// 9 — không hồi quy `find`/`rfind`/`count`/`split`
// ===========================================================================

/// Các hàm lân cận giữ nguyên ngữ nghĩa sau khi thêm `matches`; `matches` không đổi `count`.
#[test]
fn khong_hoi_quy_find_count_split() {
    dung(
        "mat-hoi-quy",
        r#"use std.str

fn main():
    print(str.find("banana", "ana"))
    print(str.rfind("banana", "ana"))
    print(str.count("banana", "ana"))
    print(str.split("banana", "ana"))
    print(str.matches("banana", "ana"))
    print(str.count("aaaa", "aa"))
    print(str.matches("aaaa", "aa"))
"#,
        "Some(1)\nSome(3)\n1\n[b, na]\n[ana]\n2\n[aa, aa]\n",
    );
}
