// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (split-terminator)** — `std.str.split_terminator(s, sep) -> list[str]`.
//!
//! Hợp đồng khoá ở đây, **đúng** nghĩa Rust `str::split_terminator` (probe `rustc 1.100.0`):
//! - Như `str.split` nhưng **không** sinh phần rỗng cuối khi `s` kết thúc bằng `sep`:
//!   `split_terminator("a,b,c", ",")` ⇒ `["a", "b", "c"]` (giống `str.split`),
//!   `split_terminator("a,b,", ",")` ⇒ `["a", "b"]` (khác `str.split`: `["a", "b", ""]`).
//! - Chỉ bỏ **một** phần rỗng cuối: `","` ⇒ `[""]` (len 1), `",,"` ⇒ `["", ""]` (len 2),
//!   `"a,b,,"` ⇒ `["a", "b", ""]` (len 3).
//! - Dấu ở **đầu** vẫn sinh phần rỗng: `",a"` ⇒ `["", "a"]`.
//! - Không khớp (hoặc `sep` dài hơn `s`) ⇒ `[s]`, kể cả `"abc"` cắt `"abc"` ⇒ `[""]`
//!   (một phần tử rỗng vì cả chuỗi là dấu phân cách).
//! - **`s` rỗng + `sep` khác rỗng ⇒ `[]`** (probe Rust; khác `str.split` của VS trả `[""]`).
//!   Đây là điểm khác thứ hai ngoài "bỏ phần rỗng cuối"; đã khoá bằng `len` = 0.
//! - `sep` rỗng theo **đúng nghĩa Rust** — khớp rỗng tại mọi biên scalar kể cả đầu, **bỏ**
//!   phần rỗng cuối: `"abc"` ⇒ `["", "a", "b", "c"]`, `"a"` ⇒ `["", "a"]`,
//!   `"đ😀"` ⇒ `["", "đ", "😀"]`, `""` ⇒ `[""]` (len 1). Khác `str.split` của VS (sep rỗng
//!   ⇒ tách scalar, **không** có phần rỗng đầu).
//! - Chồng lấn **không** khớp lại: `"aaa"` cắt `"aa"` ⇒ `["", "a"]`, `"abab"` cắt `"aba"` ⇒
//!   `["", "b"]`, `"banana"` cắt `"ana"` ⇒ `["b", "na"]`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   không bao giờ cắt giữa scalar: `"đ😀x😀"` cắt `"😀"` ⇒ `["đ", "x"]`; ZWJ trong emoji
//!   không phải biên đặc biệt: `"👨‍👩‍👧x"` cắt `"👩"` ⇒ `["👨‍", "‍👧x"]`.
//! - Chữ ký `(str, str) -> list[str]` (tái dùng `Ty::ListStr`); sugar R5c
//!   `s.split_terminator(sep)` ≡ `str.split_terminator(s, sep)`, không cần `use std.str`.
//!   **Không** đụng `str.split`/`splitn`/`split_inclusive`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **Probe Rust** (`rustc 1.100.0`, `str::split_terminator`) — bảng dưới khớp **30/30**:
//!
//! ```text
//! "a,b,c".split_terminator(",")  = ["a", "b", "c"]
//! "a,b,".split_terminator(",")   = ["a", "b"]
//! ",a".split_terminator(",")     = ["", "a"]
//! ",".split_terminator(",")      = [""]
//! ",,".split_terminator(",")     = ["", ""]
//! "".split_terminator(",")       = []
//! "a".split_terminator(",")      = ["a"]
//! "a,,b".split_terminator(",")   = ["a", "", "b"]
//! "a,b,,".split_terminator(",")  = ["a", "b", ""]
//! "a--b--".split_terminator("--")= ["a", "b"]
//! "aaa".split_terminator("aa")   = ["", "a"]
//! "abab".split_terminator("aba") = ["", "b"]
//! "banana".split_terminator("ana") = ["b", "na"]
//! "a,b,c".split_terminator("x")  = ["a,b,c"]
//! "abc".split_terminator("abcd") = ["abc"]
//! "abc".split_terminator("abc")  = [""]
//! "aa".split_terminator("aa")    = [""]
//! "".split_terminator("")        = [""]
//! "a".split_terminator("")       = ["", "a"]
//! "abc".split_terminator("")     = ["", "a", "b", "c"]
//! "đ😀".split_terminator("")     = ["", "đ", "😀"]
//! "đ,😀,x".split_terminator(",") = ["đ", "😀", "x"]
//! "đ😀x😀".split_terminator("😀") = ["đ", "x"]
//! "a😀b😀".split_terminator("😀") = ["a", "b"]
//! "👨‍👩‍👧x".split_terminator("👩") = ["👨‍", "‍👧x"]
//! "héllo,🌍,👨‍👩‍👧‍👦".split_terminator(",") = ["héllo", "🌍", "👨‍👩‍👧‍👦"]
//! "a\r\nb\r\n".split_terminator("\r\n") = ["a", "b"]
//! "a\0b\0".split_terminator("\0") = ["a", "b"]
//! "a\tb\t".split_terminator("\t") = ["a", "b"]
//! "a b ".split_terminator(" ")   = ["a", "b"]
//! bad=0
//! ```
//!
//! **RED-BEFORE**: xem khối log ở cuối tệp (chạy trước khi thêm hàm).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-spt-{tag}-{}-{n}", std::process::id()));
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
// 1 — bảng probe Rust (sep khác rỗng): bỏ đúng một phần rỗng cuối
// ===========================================================================

/// Bảng probe `rustc 1.100.0`: tách trái→phải, **không** sinh phần rỗng cuối; dấu đầu vẫn
/// sinh phần rỗng; chỉ dấu ⇒ `[""]`; `s` rỗng ⇒ `[]`; không khớp ⇒ `[s]`; chồng lấn không khớp.
#[test]
fn bang_probe_rust_bo_phan_rong_cuoi() {
    dung(
        "spt-probe",
        r#"use std.str

fn main():
    print(str.split_terminator("a,b,c", ","))
    print(str.split_terminator("a,b,", ","))
    print(str.split_terminator(",a", ","))
    print(str.split_terminator(",", ","))
    print(len(str.split_terminator(",", ",")))
    print(str.split_terminator(",", ",").join("|"))
    print(str.split_terminator(",,", ","))
    print(len(str.split_terminator(",,", ",")))
    print(str.split_terminator("", ","))
    print(len(str.split_terminator("", ",")))
    print(str.split_terminator("a", ","))
    print(str.split_terminator("a,,b", ","))
    print(str.split_terminator("a,b,,", ","))
    print(str.split_terminator("a,b,,", ",").join("|"))
    print(str.split_terminator("a--b--", "--"))
    print(str.split_terminator("aaa", "aa"))
    print(str.split_terminator("abab", "aba"))
    print(str.split_terminator("banana", "ana"))
    print(str.split_terminator("a,b,c", "x"))
    print(str.split_terminator("abc", "abcd"))
    print(str.split_terminator("abc", "abc"))
    print(len(str.split_terminator("abc", "abc")))
    print(str.split_terminator("aa", "aa"))
    print(len(str.split_terminator("aa", "aa")))
    print(len(str.split_terminator("a,b,c", ",")))
    print(str.split_terminator("a,b,c", ",").join("|"))
    print(str.split_terminator("a,b,", ",").join("|"))
    let t: list[str] = str.split_terminator("a,b,c", ",")
    print(t)
"#,
        "[a, b, c]\n[a, b]\n[, a]\n[]\n1\n\n[, ]\n2\n[]\n0\n[a]\n[a, , b]\n[a, b, ]\na|b|\n[a, b]\n[, a]\n[, b]\n[b, na]\n[a,b,c]\n[abc]\n[]\n1\n[]\n1\n3\na|b|c\na|b\n[a, b, c]\n",
    );
}

// ===========================================================================
// 2 — so sánh trực tiếp với `str.split` trên cùng đầu vào
// ===========================================================================

/// Không kết thúc bằng `sep` ⇒ **giống hệt** `str.split` (kể cả dấu đầu/liên tiếp/chồng lấn).
/// Kết thúc bằng `sep` ⇒ `split_terminator` = `split` **bỏ đúng một** phần tử rỗng cuối.
/// `s` rỗng + `sep` khác rỗng ⇒ `str.split` của VS trả `[""]` còn `split_terminator` trả `[]`
/// (probe Rust — khác biệt **có chủ ý**, đã ghi trong tài liệu hàm).
#[test]
fn so_sanh_truc_tiep_voi_split() {
    dung(
        "spt-so-sanh",
        r#"use std.str

fn main():
    let a = "a,b,c"
    print(str.split(a, ","))
    print(str.split_terminator(a, ","))
    print(str.split(a, ",") == str.split_terminator(a, ","))
    print(str.split(",a", ",") == str.split_terminator(",a", ","))
    print(str.split("a,,b", ",") == str.split_terminator("a,,b", ","))
    print(str.split("aaa", "aa") == str.split_terminator("aaa", "aa"))
    print(str.split("abc", "x") == str.split_terminator("abc", "x"))
    print(str.split("a,b,", ","))
    print(str.split_terminator("a,b,", ","))
    print(len(str.split("a,b,", ",")) - len(str.split_terminator("a,b,", ",")))
    print(str.split(",", ","))
    print(str.split_terminator(",", ","))
    print(len(str.split_terminator(",", ",")))
    print(str.split("a--b--", "--"))
    print(str.split_terminator("a--b--", "--"))
    print(str.split("đ😀x😀", "😀"))
    print(str.split_terminator("đ😀x😀", "😀"))
    print(str.split("", ","))
    print(len(str.split("", ",")))
    print(str.split_terminator("", ","))
    print(len(str.split_terminator("", ",")))
"#,
        "[a, b, c]\n[a, b, c]\ntrue\ntrue\ntrue\ntrue\ntrue\n[a, b, ]\n[a, b]\n1\n[, ]\n[]\n1\n[a, b, ]\n[a, b]\n[đ, x, ]\n[đ, x]\n[]\n1\n[]\n0\n",
    );
}

// ===========================================================================
// 3 — `sep` rỗng và `s` rỗng: bảng Rust `str::split_terminator`
// ===========================================================================

/// `sep` rỗng ⇒ phần rỗng **đầu** rồi từng scalar, **không** có phần rỗng cuối (khác
/// `str.split` của VS: tách scalar, không có phần rỗng đầu). `s` rỗng + `sep` khác rỗng ⇒
/// **`[]`** (không phải `[""]`); `""` + `""` ⇒ `[""]` (phân biệt bằng `len`).
#[test]
fn sep_rong_va_s_rong_theo_bang_rust() {
    dung(
        "spt-empty",
        r#"use std.str

fn main():
    print(str.split_terminator("abc", ""))
    print(len(str.split_terminator("abc", "")))
    print(str.split_terminator("abc", "").join("|"))
    print(str.split_terminator("a", ""))
    print(str.split_terminator("đ😀", ""))
    print(len(str.split_terminator("đ😀", "")))
    print(str.split_terminator("", ""))
    print(len(str.split_terminator("", "")))
    print(str.split_terminator("", ","))
    print(len(str.split_terminator("", ",")))
    print(str.split_terminator("", "abc"))
    print(len(str.split_terminator("", "abc")))
    print(str.split_terminator("a", ","))
    print(len(str.split_terminator("a", ",")))
    print(str.split("abc", ""))
    print(str.split_terminator("abc", ""))
    print(str.split("", ""))
    print(len(str.split("", "")))
    print(str.split_terminator("", ""))
    print(len(str.split_terminator("", "")))
"#,
        "[, a, b, c]\n4\n|a|b|c\n[, a]\n[, đ, 😀]\n3\n[]\n1\n[]\n0\n[]\n0\n[a]\n1\n[a, b, c]\n[, a, b, c]\n[]\n0\n[]\n1\n",
    );
}

// ===========================================================================
// 4 — không khớp, dấu lặp, Unicode/emoji/ZWJ
// ===========================================================================

/// Không khớp/`sep` dài hơn `s` ⇒ `[s]`; dấu liên tiếp là phần tử rỗng giữa; khớp **chuỗi con
/// UTF-8** trên biên scalar, không sinh UTF-8 hỏng; ZWJ không phải biên đặc biệt.
#[test]
fn khong_khop_lap_unicode_emoji_zwj() {
    dung(
        "spt-unicode",
        r#"use std.str

fn main():
    print(str.split_terminator("a,b,c", "x"))
    print(str.split_terminator("abc", "abcd"))
    print(str.split_terminator("ab", "abc"))
    print(str.split_terminator("abc", "ac"))
    print(str.split_terminator("a,,b", ","))
    print(str.split_terminator("a,b,,", ","))
    print(str.split_terminator("a,b,,", ",").join("|"))
    print(str.split_terminator("đ,😀,x", ","))
    print(str.split_terminator("đ😀x😀", "😀"))
    print(str.split_terminator("a😀b😀", "😀").join("|"))
    print(str.split_terminator("héllo,🌍,👨‍👩‍👧‍👦", ","))
    print(str.split_terminator("héllo,🌍,👨‍👩‍👧‍👦", ",").join("|"))
    print(str.split_terminator("👨‍👩‍👧x", "👩"))
    print(str.split_terminator("👨‍👩‍👧x", "👩").join("|"))
    print(str.split_terminator("👨‍👩‍👧‍👦x", "x"))
    print(str.split_terminator("a\r\nb\r\n", "\r\n"))
    print(str.split_terminator("a\0b\0", "\0").join("|"))
    print(str.split_terminator("a\tb\t", "\t").join("|"))
    print(str.split_terminator("a b ", " "))
"#,
        "[a,b,c]\n[abc]\n[ab]\n[abc]\n[a, , b]\n[a, b, ]\na|b|\n[đ, 😀, x]\n[đ, x]\na|b\n[héllo, 🌍, 👨‍👩‍👧‍👦]\nhéllo|🌍|👨‍👩‍👧‍👦\n[👨‍, ‍👧x]\n👨‍|‍👧x\n[👨‍👩‍👧‍👦]\n[a, b]\na|b\na|b\n[a, b]\n",
    );
}

// ===========================================================================
// 5 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.split_terminator(sep)` ≡ `str.split_terminator(s, sep)`; kết quả `list[str]` thật.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "spt-sugar",
        r#"fn main():
    let s: str = "a,b,c"
    print(s.split_terminator(","))
    print("a,b,".split_terminator(","))
    print(s.split_terminator(",").join("|"))
    print("a,b,".split_terminator(",").join("|"))
    print("abc".split_terminator(""))
    print(len("abc".split_terminator("")))
    print("đ😀x😀".split_terminator("😀"))
    let t: list[str] = s.split_terminator(",")
    print(t)
    print(len("".split_terminator(",")))
    print("".split_terminator(","))
"#,
        "[a, b, c]\n[a, b]\na|b|c\na|b\n[, a, b, c]\n4\n[đ, x]\n[a, b, c]\n0\n[]\n",
    );
}

// ===========================================================================
// 6 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.split_terminator(1, \",\")", "VS0412"),
        ("kieu-2", "str.split_terminator(\"a\", 1)", "VS0412"),
        ("arity-1", "str.split_terminator(\"a\")", "VS0401"),
        ("arity-2", "str.split_terminator()", "VS0401"),
        ("arity-3", "str.split_terminator(\"a\", \",\", 3)", "VS0401"),
        ("sugar-kieu", "\"a\".split_terminator(1)", "VS0412"),
        ("sugar-arity-1", "\"a\".split_terminator()", "VS0401"),
        (
            "sugar-arity-2",
            "\"a\".split_terminator(\",\", 3)",
            "VS0401",
        ),
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
        "spt-ket-qua-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.split_terminator(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spt-ket-qua-sang-list-int",
        "use std.str\n\nfn main():\n    let x: list[int] = str.split_terminator(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spt-sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"a,b\".split_terminator(\",\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 7 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính (khớp/không khớp, `sep` rỗng, `s` rỗng, dấu cuối, dấu liên tiếp,
/// chồng lấn, Unicode, emoji, ZWJ, sugar) để khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "spt-hai-engine",
        r#"use std.str

fn main():
    let s: str = "a,b,c"
    print(str.split_terminator(s, ","))
    print(s.split_terminator(","))
    print(str.split_terminator("abc", ","))
    print(str.split_terminator("a,b,", ","))
    print(str.split_terminator("", ","))
    print(len(str.split_terminator("", ",")))
    print(str.split_terminator("", ""))
    print(len(str.split_terminator("", "")))
    print(str.split_terminator("abc", ""))
    print(str.split_terminator("aaa", "aa"))
    print(str.split_terminator("a--b--", "--"))
    print(str.split_terminator("a,b,,", ",").join("|"))
    print(str.split_terminator("đ😀x😀", "😀").join("|"))
    print(str.split_terminator("👨‍👩‍👧x", "👩"))
    print(str.split_terminator("héllo,🌍,👨‍👩‍👧‍👦", ","))
    print(str.split_terminator("a,b,c", ",") == str.split("a,b,c", ","))
    print(str.split_terminator("a,b,", ",") == str.split("a,b,", ","))
    let t: list[str] = "a,b,c".split_terminator(",")
    print(t)
    print(str.split_terminator("a\tb\t", "\t").join("|"))
"#,
        "[a, b, c]\n[a, b, c]\n[abc]\n[a, b]\n[]\n0\n[]\n1\n[, a, b, c]\n[, a]\n[a, b]\na|b|\nđ|x\n[👨‍, ‍👧x]\n[héllo, 🌍, 👨‍👩‍👧‍👦]\ntrue\nfalse\n[a, b, c]\na|b\n",
    );
}

// ===========================================================================
// RED-BEFORE — log chạy **trước** khi thêm hàm
// ===========================================================================

// **RED-BEFORE** — `cargo test -p vis-cli --test vd270_split_terminator` trên cây **chưa** có
// `str.split_terminator`:
//
// ```text
// test bang_probe_rust_bo_phan_rong_cuoi ... FAILED
// test hai_engine_byte_identical_toan_bo ... FAILED
// test ket_qua_la_list_str_that ... FAILED
// test khong_khop_lap_unicode_emoji_zwj ... FAILED
// test sai_kieu_sai_arity_bi_chan ... FAILED
// test sep_rong_va_s_rong_theo_bang_rust ... FAILED
// test so_sanh_truc_tiep_voi_split ... FAILED
// test sugar_phuong_thuc_khong_can_use ... FAILED
//
// test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out
//
// ---- bang_probe_rust_bo_phan_rong_cuoi stdout ----
// assertion `left == right` failed: spt-probe: phải chạy sạch
// stderr: spt-probe.vis:5:11: error VS0301: module `std.str` không có hàm `split_terminator`
// ...
//
// ---- sugar_phuong_thuc_khong_can_use stdout ----
// stderr: spt-sugar.vis:4:11: error VS0403: `str` không có trường `split_terminator`
// ...
//
// ---- sai_kieu_sai_arity_bi_chan stdout ----
// kieu-1: `check` thiếu `VS0412`
// stdout: TAP version 13
// 1..1
// not ok 1 - .../kieu-1.vis
// # VS0301 5:13 module `std.str` không có hàm `split_terminator`
// ...
//
// ---- ket_qua_la_list_str_that stdout ----
// spt-ket-qua-sang-str: `check` thiếu `VS0412`
// # VS0301 5:18 module `std.str` không có hàm `split_terminator`
// ```
//
// Sau khi thêm hàm: 8/8 ca xanh, interp ↔ VM byte-identical.
