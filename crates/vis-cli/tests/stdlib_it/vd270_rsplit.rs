// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (rsplit)** — `std.str.rsplit(s, sep) -> list[str]`.
//!
//! Hợp đồng khoá ở đây:
//! - Tách **phải → trái**, trả các phần theo **thứ tự ngược** (phần **cuối** trước), **đúng**
//!   nghĩa Rust `str::rsplit`: `rsplit("a,b,c", ",")` ⇒ `["c", "b", "a"]`.
//! - Không khớp/`sep` dài hơn `s` ⇒ `[s]`; `s` rỗng + `sep` khác rỗng ⇒ `[""]` (`len` = 1,
//!   khác `str.split_inclusive` rỗng ⇒ `[]`).
//! - Dấu phân cách ở **cuối** sinh phần tử rỗng **đầu**: `rsplit("a,b,", ",")` ⇒ `["", "b", "a"]`;
//!   ở **đầu** sinh phần tử rỗng **cuối**: `rsplit(",a", ",")` ⇒ `["a", ""]`; dấu **liên tiếp**
//!   là phần tử rỗng riêng: `rsplit("a,,b", ",")` ⇒ `["b", "", "a"]`.
//! - Chồng lấn **không** khớp lại (khớp không chồng, từ phải): `rsplit("aaa", "aa")` ⇒ `["", "a"]`,
//!   `rsplit("aaaa", "aa")` ⇒ `["", "", ""]`, `rsplit("abab", "aba")` ⇒ `["b", ""]`,
//!   `rsplit("banana", "ana")` ⇒ `["", "ban"]`.
//! - `sep` rỗng theo **đúng nghĩa Rust `str::rsplit`** (bảng probe bên dưới):
//!   `"abc"` ⇒ `["", "c", "b", "a", ""]`; `""` ⇒ `["", ""]`; `"a"` ⇒ `["", "a", ""]`;
//!   `"đ😀"` ⇒ `["", "😀", "đ", ""]`. **Không** trap vì `sep` rỗng. Đây là khác biệt **có chủ ý**
//!   với `str.split` của VS (rỗng ⇒ tách theo scalar, **không** sinh phần rỗng đầu/cuối).
//! - Đơn vị khớp là **chuỗi con UTF-8** trên biên scalar (cùng `str.split`/`str.find`), **không
//!   bao giờ** cắt giữa scalar: `rsplit("đ😀x😀", "😀")` ⇒ `["", "x", "đ"]`; ZWJ trong emoji
//!   không phải biên đặc biệt: `rsplit("👨‍👩‍👧x", "👩")` ⇒ `["‍👧x", "👨‍"]`.
//! - Chữ ký `(str, str) -> list[str]`; sugar R5c `s.rsplit(sep)` ≡ `str.rsplit(s, sep)`,
//!   không cần `use std.str`. Không đụng `str.split`/`str.splitn`/`str.rsplit_once`.
//!
//! Bảng probe `rustc 1.100.0` (toolchain ghim, chạy lại trong `strrsplit.rs` unit test):
//!
//! | biểu thức | kết quả |
//! |---|---|
//! | `"a,b,c".rsplit(",")` | `["c", "b", "a"]` |
//! | `"a,b,".rsplit(",")` | `["", "b", "a"]` |
//! | `",a".rsplit(",")` | `["a", ""]` |
//! | `"a,,b".rsplit(",")` | `["b", "", "a"]` |
//! | `"abc".rsplit(",")` | `["abc"]` |
//! | `"".rsplit(",")` | `[""]` |
//! | `"abc".rsplit("")` | `["", "c", "b", "a", ""]` |
//! | `"".rsplit("")` | `["", ""]` |
//! | `"a".rsplit("")` | `["", "a", ""]` |
//! | `"aaa".rsplit("aa")` | `["", "a"]` |
//! | `"aaaa".rsplit("aa")` | `["", ""]` |
//! | `"abab".rsplit("aba")` | `["b", ""]` |
//! | `"banana".rsplit("ana")` | `["", "ban"]` |
//! | `"đ😀x😀".rsplit("😀")` | `["", "x", "đ"]` |
//! | `"👨‍👩‍👧x".rsplit("👩")` | `["‍👧x", "👨‍"]` |
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (binary `vis` của cây này **trước** khi thêm hàm; `cargo test -p vis-cli
//! --test vd270_rsplit`): **0 passed / 9 failed** — mọi ca dương đỏ `VS0301` «module `std.str`
//! không có hàm `rsplit`», sugar đỏ `VS0403` «`str` không có trường `rsplit`», ca kiểu/arity
//! cũng đỏ `VS0301` thay vì `VS0412`/`VS0401`:
//!
//! ```text
//! $ vis run --no-cache rsp-bang.vis
//! rsp-bang.vis:5:11: error VS0301: module `std.str` không có hàm `rsplit`
//! ...
//! $ vis run --no-cache rsp-sugar.vis
//! rsp-sugar.vis:5:11: error VS0403: `str` không có trường `rsplit`
//! ```
//!
//! Sau khi thêm: **9/9 xanh**, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-rsp-{tag}-{}-{n}", std::process::id()));
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
// 1 — bảng probe Rust: thứ tự ngược, không khớp, dấu cuối/đầu/liên tiếp
// ===========================================================================

/// Bảng **probe `rustc 1.100.0`**: `"a,b,c"` ⇒ `[c, b, a]`; không khớp ⇒ `[abc]`; `sep` dài
/// hơn `s` ⇒ `[abc]`; `"a,b,"` ⇒ `[, b, a]`; `",a"` ⇒ `[a, ]`; `","`/`",,"`; `"a,,b"`.
#[test]
fn bang_probe_rust_thu_tu_nguoc() {
    dung(
        "rsp-bang",
        r#"use std.str

fn main():
    print(str.rsplit("a,b,c", ","))
    print(str.rsplit("abc", ","))
    print(str.rsplit("abc", "abcd"))
    print(str.rsplit("a,b,", ","))
    print(str.rsplit(",a", ","))
    print(str.rsplit(",", ","))
    print(str.rsplit(",,", ","))
    print(str.rsplit("a,,b", ","))
    print(str.join(str.rsplit("a,b,", ","), "|"))
    print(str.join(str.rsplit("a,,b", ","), "|"))
    print(str.join(str.rsplit("a,b,c", ","), "|"))
"#,
        "[c, b, a]\n[abc]\n[abc]\n[, b, a]\n[a, ]\n[, ]\n[, , ]\n[b, , a]\n|b|a\nb||a\nc|b|a\n",
    );
}

// ===========================================================================
// 2 — `s` rỗng + `sep` khác rỗng ⇒ `[""]` (`len` = 1)
// ===========================================================================

/// `rsplit("", ",")` trả **một** phần tử rỗng (`len` = 1), khác `split_inclusive("", ",")` ⇒ `[]`;
/// `""` + `sep` nhiều byte cũng vậy.
#[test]
fn s_rong_sep_khong_rong_tra_mot_phan_rong() {
    dung(
        "rsp-s-rong",
        r#"use std.str

fn main():
    print(str.rsplit("", ","))
    print(len(str.rsplit("", ",")))
    print(len(str.rsplit("", "::")))
    print(str.join(str.rsplit("", ","), "|"))
    print(str.rsplit("a", ","))
"#,
        "[]\n1\n1\n\n[a]\n",
    );
}

// ===========================================================================
// 3 — `sep` rỗng: bảng Rust `str::rsplit("")`
// ===========================================================================

/// Bảng đối chiếu **probe Rust**: `"abc"`, `""`, `"a"`, `"đ😀"` với `sep` rỗng — phần rỗng
/// **đầu và cuối**, scalar theo thứ tự ngược; `len` phân biệt `[""]` với `[]`.
#[test]
fn sep_rong_theo_bang_rust() {
    dung(
        "rsp-empty-sep",
        r#"use std.str

fn main():
    print(str.rsplit("abc", ""))
    print(len(str.rsplit("abc", "")))
    print(str.rsplit("", ""))
    print(len(str.rsplit("", "")))
    print(str.rsplit("a", ""))
    print(len(str.rsplit("a", "")))
    print(str.rsplit("đ😀", ""))
    print(len(str.rsplit("đ😀", "")))
    print(str.join(str.rsplit("abc", ""), "|"))
"#,
        "[, c, b, a, ]\n5\n[, ]\n2\n[, a, ]\n3\n[, 😀, đ, ]\n4\n|c|b|a|\n",
    );
}

// ===========================================================================
// 4 — chồng lấn: khớp **không chồng** từ phải
// ===========================================================================

/// `"aaa"` cắt `"aa"` ⇒ `["", "a"]`; `"aaaa"` ⇒ `["", ""]`; `"abab"` cắt `"aba"` ⇒ `["b", ""]`;
/// `"banana"` cắt `"ana"` ⇒ `["", "ban"]` — không khớp lại phần đã dùng.
#[test]
fn chong_lan_khong_khop_lai() {
    dung(
        "rsp-chong-lan",
        r#"use std.str

fn main():
    print(str.rsplit("aaa", "aa"))
    print(str.rsplit("aaaa", "aa"))
    print(str.rsplit("abab", "aba"))
    print(str.rsplit("banana", "ana"))
    print(str.join(str.rsplit("aaa", "aa"), "|"))
    print(str.join(str.rsplit("banana", "ana"), "|"))
"#,
        "[, a]\n[, , ]\n[b, ]\n[, ban]\n|a\n|ban\n",
    );
}

// ===========================================================================
// 5 — Unicode/emoji: khớp theo chuỗi con, không cắt giữa scalar
// ===========================================================================

/// Tiếng Việt + emoji: dấu phân cách nhiều byte vẫn khớp đúng; ZWJ không phải biên đặc biệt;
/// không sinh UTF-8 hỏng.
#[test]
fn unicode_emoji_zwj() {
    dung(
        "rsp-unicode",
        r#"use std.str

fn main():
    print(str.rsplit("đ😀x😀", "😀"))
    print(str.join(str.rsplit("đ😀x😀", "😀"), "|"))
    print(str.rsplit("a😀b😀c", "😀"))
    print(str.rsplit("a::b::c", "::"))
    print(str.rsplit("👨‍👩‍👧x", "👩"))
    print(str.join(str.rsplit("👨‍👩‍👧x", "👩"), "|"))
    print(str.rsplit("👨‍👩‍👧", "👨‍👩‍👧"))
    print(str.rsplit("đ😀", ""))
"#,
        "[, x, đ]\n|x|đ\n[c, b, a]\n[c, b, a]\n[‍👧x, 👨‍]\n‍👧x|👨‍\n[, ]\n[, 😀, đ, ]\n",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.rsplit(sep)` ≡ `str.rsplit(s, sep)`; kết quả là `list[str]` thật; `sep` rỗng theo Rust.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "rsp-sugar",
        r#"fn main():
    let s: str = "a,b,c"
    print(s.rsplit(","))
    print("a,b,c".rsplit(","))
    let t: list[str] = s.rsplit(",")
    print(t)
    print("a,b,".rsplit(",").join("|"))
    print("abc".rsplit(""))
    print("đ😀x😀".rsplit("😀"))
"#,
        "[c, b, a]\n[c, b, a]\n[c, b, a]\n|b|a\n[, c, b, a, ]\n[, x, đ]\n",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.rsplit(1, \",\")", "VS0412"),
        ("kieu-2", "str.rsplit(\"a\", 1)", "VS0412"),
        ("arity-1", "str.rsplit(\"a\")", "VS0401"),
        ("arity-2", "str.rsplit()", "VS0401"),
        ("arity-3", "str.rsplit(\"a\", \",\", 1)", "VS0401"),
        ("sugar-kieu", "\"a\".rsplit(1)", "VS0412"),
        ("sugar-arity-1", "\"a\".rsplit()", "VS0401"),
        ("sugar-arity-2", "\"a\".rsplit(\",\", 1)", "VS0401"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `list[str]` thật: gán `list[str]` hợp lệ; gán `str`/`list[int]` bị chặn.
#[test]
fn ket_qua_la_list_str_that() {
    tu_choi(
        "rsp-ket-qua-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.rsplit(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "rsp-ket-qua-sang-list-int",
        "use std.str\n\nfn main():\n    let x: list[int] = str.rsplit(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "rsp-sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"a,b\".rsplit(\",\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 8 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp ngữ nghĩa chính (bảng probe, `sep` rỗng, `s` rỗng, dấu cuối/đầu, chồng lấn, Unicode,
/// sugar) và **hồi quy** `str.split`/`str.splitn`/`str.rsplit_once` không đổi.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "rsp-hai-engine",
        r#"use std.str

fn main():
    let s: str = "a,b,c"
    print(str.rsplit(s, ","))
    print(s.rsplit(","))
    print(str.rsplit("a,b,", ","))
    print(str.rsplit(",a", ","))
    print(str.rsplit("a,,b", ","))
    print(str.rsplit("abc", ","))
    print(len(str.rsplit("", ",")))
    print(str.rsplit("abc", ""))
    print(str.rsplit("", ""))
    print(str.rsplit("aaa", "aa"))
    print(str.rsplit("banana", "ana"))
    print(str.rsplit("đ😀x😀", "😀"))
    print(str.rsplit("👨‍👩‍👧x", "👩"))
    print(str.rsplit("a::b::c", "::"))
    print(str.split("a,b,c", ","))
    print(str.splitn("a,b,c", ",", 2))
    print(str.rsplit_once("a,b,c", ","))
"#,
        "[c, b, a]\n[c, b, a]\n[, b, a]\n[a, ]\n[b, , a]\n[abc]\n1\n[, c, b, a, ]\n[, ]\n[, a]\n[, ban]\n[, x, đ]\n[‍👧x, 👨‍]\n[c, b, a]\n[a, b, c]\n[a, b,c]\nSome([a,b, c])\n",
    );
}
