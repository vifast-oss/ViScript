// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (split-inclusive)** — `std.str.split_inclusive(s, sep) -> list[str]`.
//!
//! Hợp đồng được khoá ở đây (đúng nghĩa Rust `str::split_inclusive`):
//! - Tách **trái → phải** tại mỗi lần xuất hiện **không chồng** của `sep`; **mỗi phần tử
//!   (trừ phần cuối nếu `s` không kết thúc bằng `sep`) giữ luôn dấu phân cách**:
//!   `split_inclusive("a,b,c", ",")` ⇒ `["a,", "b,", "c"]`.
//! - Không khớp (hoặc `sep` dài hơn `s`) ⇒ `[s]` — **không** rỗng.
//! - `sep` rỗng theo **đúng nghĩa Rust** (probe `rustc 1.100.0`):
//!   `split_inclusive("abc", "")` ⇒ `["", "a", "b", "c"]` (không có phần rỗng cuối);
//!   `split_inclusive("", "")` ⇒ `[""]`; `split_inclusive("a", "")` ⇒ `["", "a"]`.
//! - `s` rỗng + `sep` **khác** rỗng ⇒ **`[]`** (KHÔNG phải `[""]`): Rust
//!   `"".split_inclusive(",")` trả iterator rỗng. Đây là điểm khác `str.split` của VS
//!   (VS trả `[""]`) và đã được probe xác nhận ở 1.100.0.
//! - Dấu phân cách ở **cuối** ⇒ phần tử cuối **kết thúc bằng** `sep`, **không** sinh phần rỗng
//!   thêm: `split_inclusive("a,b,", ",")` ⇒ `["a,", "b,"]`.
//! - Dấu phân cách liên tiếp ⇒ mỗi dấu là một phần tử riêng:
//!   `split_inclusive("a,b,,c", ",")` ⇒ `["a,", "b,", ",", "c"]`.
//! - Chồng lấn **không** được khớp lại: `split_inclusive("aaa", "aa")` ⇒ `["aa", "a"]`,
//!   `split_inclusive("abab", "aba")` ⇒ `["aba", "b"]`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `split_inclusive("đ😀x😀", "😀")` ⇒
//!   `["đ😀", "x😀"]`; ZWJ trong emoji không phải biên đặc biệt:
//!   `split_inclusive("👨‍👩‍👧x", "👩")` ⇒ `["👨‍👩", "‍👧x"]`, phần còn lại vẫn là UTF-8 hợp lệ.
//! - Chữ ký `(str, str) -> list[str]` (tái dùng `Ty::ListStr`, **không** thêm biến thể `Ty`);
//!   sugar R5c `s.split_inclusive(sep)` ≡ `str.split_inclusive(s, sep)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **Probe Rust** (`rustc 1.100.0`, `str::split_inclusive`) — bảng dưới khớp 26/26:
//!
//! ```text
//! "a,b,c".split_inclusive(",")  = ["a,", "b,", "c"]
//! "abc".split_inclusive(",")    = ["abc"]
//! "a,b,".split_inclusive(",")   = ["a,", "b,"]
//! ",a".split_inclusive(",")     = [",", "a"]
//! ",".split_inclusive(",")      = [","]
//! ",,".split_inclusive(",")     = [",", ","]
//! "".split_inclusive(",")       = []
//! "".split_inclusive("")        = [""]
//! "abc".split_inclusive("")     = ["", "a", "b", "c"]
//! "aa".split_inclusive("aa")    = ["aa"]
//! "aaa".split_inclusive("aa")   = ["aa", "a"]
//! "abab".split_inclusive("aba") = ["aba", "b"]
//! "a--b--".split_inclusive("--")= ["a--", "b--"]
//! "aXbXc".split_inclusive("X")  = ["aX", "bX", "c"]
//! "ab".split_inclusive("abc")   = ["ab"]
//! "abc".split_inclusive("abc")  = ["abc"]
//! "héllo,🌍,👨‍👩‍👧‍👦".split_inclusive(",") = ["héllo,", "🌍,", "👨‍👩‍👧‍👦"]
//! "a🌍b🌍".split_inclusive("🌍") = ["a🌍", "b🌍"]
//! "👨‍👩‍👧‍👦x".split_inclusive("x") = ["👨‍👩‍👧‍👦x"]
//! "👨‍👩‍👧x".split_inclusive("👩") = ["👨‍👩", "‍👧x"]
//! "a\r\nb\r\n".split_inclusive("\r\n") = ["a\r\n", "b\r\n"]
//! "a\0b\0".split_inclusive("\0") = ["a\0", "b\0"]
//! "a,b,,c".split_inclusive(",") = ["a,", "b,", ",", "c"]
//! "a\tb\t".split_inclusive("\t") = ["a\t", "b\t"]
//! "a".split_inclusive(",")      = ["a"]
//! "".split_inclusive("abc")     = []
//! bad=0
//! ```
//!
//! **RED-BEFORE**: xem khối log ở cuối tệp (chạy trước khi thêm hàm; binary `vis` của cây này
//! @ `0161008`).
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-spi-{tag}-{}-{n}", std::process::id()));
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
// 1 — bảng probe: giữ dấu phân cách, không khớp ⇒ `[s]`, cuối ⇒ không phần rỗng
// ===========================================================================

/// Bảng probe Rust: mỗi phần tử giữ dấu phân cách, dấu cuối không sinh phần rỗng, dấu liên
/// tiếp là phần tử riêng, chồng lấn không khớp lại, không khớp ⇒ `[s]`.
#[test]
fn bang_probe_rust_giu_dau_phan_cach() {
    dung(
        "spi-probe",
        r#"use std.str

fn main():
    print(str.split_inclusive("a,b,c", ","))
    print(str.split_inclusive("abc", ","))
    print(str.split_inclusive("a,b,", ","))
    print(str.split_inclusive(",a", ","))
    print(str.split_inclusive(",", ","))
    print(str.split_inclusive(",,", ","))
    print(str.split_inclusive("a,b,,c", ","))
    print(str.split_inclusive("aXbXc", "X"))
    print(str.split_inclusive("a--b--", "--"))
    print(str.split_inclusive("aa", "aa"))
    print(str.split_inclusive("aaa", "aa"))
    print(str.split_inclusive("abab", "aba"))
    print(str.split_inclusive("ab", "abc"))
    print(str.split_inclusive("abc", "abc"))
    print(len(str.split_inclusive("a,b,c", ",")))
    print(str.split_inclusive("a,b,c", ",").join("|"))
    print(str.split_inclusive("a,b,", ",").join("|"))
    print(str.split_inclusive("a,b,,c", ",").join("|"))
    print(len(str.split_inclusive("a,b,", ",")))
    let t: list[str] = str.split_inclusive("a,b,c", ",")
    print(t)
"#,
        "[a,, b,, c]\n[abc]\n[a,, b,]\n[,, a]\n[,]\n[,, ,]\n[a,, b,, ,, c]\n[aX, bX, c]\n[a--, b--]\n[aa]\n[aa, a]\n[aba, b]\n[ab]\n[abc]\n3\na,|b,|c\na,|b,\na,|b,|,|c\n2\n[a,, b,, c]\n",
    );
}

// ===========================================================================
// 2 — `sep` rỗng và `s` rỗng: bảng Rust `str::split_inclusive`
// ===========================================================================

/// `sep` rỗng: phần rỗng **đầu** rồi từng scalar, **không** có phần rỗng cuối. `s` rỗng +
/// `sep` khác rỗng ⇒ **`[]`** (không phải `[""]`); `""` + `""` ⇒ `[""]` (phân biệt bằng `len`).
#[test]
fn sep_rong_va_s_rong_theo_bang_rust() {
    dung(
        "spi-empty",
        r#"use std.str

fn main():
    print(str.split_inclusive("abc", ""))
    print(len(str.split_inclusive("abc", "")))
    print(str.split_inclusive("abc", "").join("|"))
    print(str.split_inclusive("a", ""))
    print(str.split_inclusive("đ😀", ""))
    print(len(str.split_inclusive("đ😀", "")))
    print(str.split_inclusive("", ""))
    print(len(str.split_inclusive("", "")))
    print(str.split_inclusive("", ","))
    print(len(str.split_inclusive("", ",")))
    print(str.split_inclusive("", "abc"))
    print(len(str.split_inclusive("", "abc")))
    print(str.split_inclusive("a", ","))
    print(len(str.split_inclusive("a", ",")))
"#,
        "[, a, b, c]\n4\n|a|b|c\n[, a]\n[, đ, 😀]\n3\n[]\n1\n[]\n0\n[]\n0\n[a]\n1\n",
    );
}

// ===========================================================================
// 3 — Unicode/emoji/ZWJ: khớp chuỗi con trên biên scalar
// ===========================================================================

/// Tiếng Việt + emoji + ZWJ: dấu phân cách nhiều byte vẫn khớp đúng và **giữ** dấu; không sinh
/// UTF-8 hỏng; không khớp ⇒ `[s]`.
#[test]
fn unicode_emoji_khong_cat_scalar() {
    dung(
        "spi-unicode",
        r#"use std.str

fn main():
    print(str.split_inclusive("đ,😀,x", ","))
    print(str.split_inclusive("đ,😀,x", ",").join("|"))
    print(str.split_inclusive("a😀b😀c", "😀"))
    print(str.split_inclusive("a🌍b🌍", "🌍"))
    print(str.split_inclusive("héllo,🌍,👨‍👩‍👧‍👦", ","))
    print(str.split_inclusive("👨‍👩‍👧x", "👩"))
    print(str.split_inclusive("👨‍👩‍👧x", "👩").join("|"))
    print(str.split_inclusive("👨‍👩‍👧‍👦x", "x"))
    print(str.split_inclusive("đ😀x", "😀a"))
    print(str.split_inclusive("a\tb\t", "\t"))
    print(str.split_inclusive("a\tb\t", "\t").join("|"))
"#,
        "[đ,, 😀,, x]\nđ,|😀,|x\n[a😀, b😀, c]\n[a🌍, b🌍]\n[héllo,, 🌍,, 👨‍👩‍👧‍👦]\n[👨‍👩, ‍👧x]\n👨‍👩|‍👧x\n[👨‍👩‍👧‍👦x]\n[đ😀x]\n[a\t, b\t]\na\t|b\t\n",
    );
}

// ===========================================================================
// 4 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.split_inclusive(sep)` ≡ `str.split_inclusive(s, sep)`; kết quả `list[str]` thật.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "spi-sugar",
        r#"fn main():
    let s: str = "a,b,c"
    print(s.split_inclusive(","))
    print("a,b,c".split_inclusive(","))
    print(s.split_inclusive(",").join("|"))
    print("a,b,".split_inclusive(",").join("|"))
    print("abc".split_inclusive(""))
    print(len("abc".split_inclusive("")))
    print("đ😀x😀".split_inclusive("😀"))
    let t: list[str] = s.split_inclusive(",")
    print(t)
    print(len("".split_inclusive(",")))
"#,
        "[a,, b,, c]\n[a,, b,, c]\na,|b,|c\na,|b,\n[, a, b, c]\n4\n[đ😀, x😀]\n[a,, b,, c]\n0\n",
    );
}

// ===========================================================================
// 5 — ca âm tĩnh: kiểu / arity (module + sugar) và kiểu trả `list[str]`
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.split_inclusive(1, \",\")", "VS0412"),
        ("kieu-2", "str.split_inclusive(\"a\", 1)", "VS0412"),
        ("arity-1", "str.split_inclusive(\"a\")", "VS0401"),
        ("arity-2", "str.split_inclusive()", "VS0401"),
        ("arity-3", "str.split_inclusive(\"a\", \",\", 3)", "VS0401"),
        ("sugar-kieu", "\"a\".split_inclusive(1)", "VS0412"),
        ("sugar-arity-1", "\"a\".split_inclusive()", "VS0401"),
        ("sugar-arity-2", "\"a\".split_inclusive(\",\", 3)", "VS0401"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `list[str]` thật: gán `list[str]` hợp lệ (ca dương ở trên); gán `str`,
/// `list[int]` hoặc qua sugar sang `int` đều bị chặn.
#[test]
fn ket_qua_la_list_str_that() {
    tu_choi(
        "spi-ket-qua-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.split_inclusive(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spi-ket-qua-sang-list-int",
        "use std.str\n\nfn main():\n    let x: list[int] = str.split_inclusive(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spi-sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"a,b\".split_inclusive(\",\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 6 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính (khớp/không khớp, `sep` rỗng, `s` rỗng, dấu cuối, dấu liên tiếp,
/// chồng lấn, Unicode, emoji, ZWJ, sugar) để khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "spi-hai-engine",
        r#"use std.str

fn main():
    let s: str = "a,b,c"
    print(str.split_inclusive(s, ","))
    print(s.split_inclusive(","))
    print(str.split_inclusive("abc", ","))
    print(str.split_inclusive("a,b,", ","))
    print(str.split_inclusive("", ","))
    print(len(str.split_inclusive("", ",")))
    print(str.split_inclusive("", ""))
    print(len(str.split_inclusive("", "")))
    print(str.split_inclusive("abc", ""))
    print(str.split_inclusive("aaa", "aa"))
    print(str.split_inclusive("a--b--", "--"))
    print(str.split_inclusive("a,b,,c", ",").join("|"))
    print(str.split_inclusive("đ😀x😀", "😀").join("|"))
    print(str.split_inclusive("👨‍👩‍👧x", "👩"))
    print(str.split_inclusive("héllo,🌍,👨‍👩‍👧‍👦", ","))
    let t: list[str] = "a,b,c".split_inclusive(",")
    print(t)
    print(str.split_inclusive("a\tb\t", "\t").join("|"))
"#,
        "[a,, b,, c]\n[a,, b,, c]\n[abc]\n[a,, b,]\n[]\n0\n[]\n1\n[, a, b, c]\n[aa, a]\n[a--, b--]\na,|b,|,|c\nđ😀|x😀\n[👨‍👩, ‍👧x]\n[héllo,, 🌍,, 👨‍👩‍👧‍👦]\n[a,, b,, c]\na\t|b\t\n",
    );
}

// ===========================================================================
// RED-BEFORE — log chạy **trước** khi thêm hàm (binary `vis` @ `0161008`)
// ===========================================================================

// **RED-BEFORE** — `cargo test -p vis-cli --test vd270_split_inclusive` trên cây **chưa** có
// `str.split_inclusive` (binary `vis` dựng @ `0161008`):
//
// ```text
// test bang_probe_rust_giu_dau_phan_cach ... FAILED
// test hai_engine_byte_identical_toan_bo ... FAILED
// test ket_qua_la_list_str_that ... FAILED
// test sai_kieu_sai_arity_bi_chan ... FAILED
// test sep_rong_va_s_rong_theo_bang_rust ... FAILED
// test sugar_phuong_thuc_khong_can_use ... FAILED
// test unicode_emoji_khong_cat_scalar ... FAILED
//
// test result: FAILED. 1 passed; 7 failed; 0 ignored; 0 measured
//
// ---- sai_kieu_sai_arity_bi_chan stdout ----
// kieu-1: `check` thiếu `VS0412`
// stdout: TAP version 13
// 1..1
// not ok 1 - .../kieu-1.vis
// # VS0301 5:13 module `std.str` không có hàm `split_inclusive`
//
// ---- sep_rong_va_s_rong_theo_bang_rust stdout ----
// assertion `left == right` failed: spi-empty: phải chạy sạch
// stdout:
// stderr: spi-empty.vis:5:11: error VS0301: module `std.str` không có hàm `split_inclusive`
// spi-empty.vis:6:15: error VS0301: module `std.str` không có hàm `split_inclusive`
// spi-empty.vis:7:11: error VS0301: module `std.str` không có hàm `split_inclusive`
// ...
//
// ---- sugar_phuong_thuc_khong_can_use stdout ----
// spi-sugar.vis:4:11: error VS0403: `str` không có trường `split_inclusive`
// spi-sugar.vis:5:11: error VS0403: `str` không có trường `split_inclusive`
// ...
// ```
//
// (Ca `red_before_ghi_chu` rỗng thời điểm đó là test duy nhất "xanh"; nó đã được bỏ sau khi
// điền log này — 7/7 ca thực chất đỏ trước, xanh sau khi thêm hàm.)
