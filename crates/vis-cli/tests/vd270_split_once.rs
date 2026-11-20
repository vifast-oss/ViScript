// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (split-once)** — `std.str.split_once(s, sep) -> list[str]?`.
//!
//! Hợp đồng được khoá ở đây:
//! - Tách tại **lần xuất hiện đầu tiên** của `sep`, **không** tách các lần sau:
//!   `split_once("a,b,c", ",")` ⇒ `Some(["a", "b,c"])`;
//!   `split_once("aXbXc", "X")` ⇒ `Some(["a", "bXc"])`.
//! - Không có `sep` trong `s` (hoặc `sep` dài hơn `s`) ⇒ `None`.
//! - `sep` rỗng ⇒ `Some(["", s])`; `s` rỗng + `sep` khác rỗng ⇒ `None`; `s` rỗng + `sep` rỗng
//!   ⇒ `Some(["", ""])` (bảng đã kiểm bằng probe Rust, xem dưới). **Không** trap.
//! - Dấu phân cách ở **đầu**/`cuối` sinh phần rỗng tương ứng:
//!   `split_once("abc", "a")` ⇒ `Some(["", "bc"])`; `split_once("abc", "c")` ⇒
//!   `Some(["ab", ""])`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `split_once("đ😀x", "😀")` ⇒ `Some(["đ", "x"])`;
//!   ZWJ trong emoji không phải biên đặc biệt: `split_once("👨‍👩‍👧x", "👩")` ⇒
//!   `Some(["👨‍", "‍👧x"])`, phần còn lại vẫn là UTF-8 hợp lệ.
//! - `Some` luôn có **đúng 2** phần tử; chỉ cấp phát hai chuỗi kết quả.
//! - Chữ ký `(str, str) -> list[str]?` (`Ty::OptListStr` = `Option[list[str]]` thật); sugar
//!   R5c `s.split_once(sep)` ≡ `str.split_once(s, sep)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **Probe Rust** (`rustc 1.100.0`, `str::split_once`) — 15/15 khớp:
//!
//! ```text
//! "a,b,c".split_once(",")      = Some(("a", "b,c"))
//! "a,b,c".split_once("x")      = None
//! "abc".split_once("")         = Some(("", "abc"))
//! "".split_once("a")           = None
//! "".split_once("")            = Some(("", ""))
//! "abc".split_once("abcd")     = None
//! "a::b::c".split_once("::")   = Some(("a", "b::c"))
//! "đ😀x".split_once("😀")       = Some(("đ", "x"))
//! "👨‍👩‍👧x".split_once("👩")      = Some(("👨‍", "‍👧x"))
//! "aXbXc".split_once("X")      = Some(("a", "bXc"))
//! "abc".split_once("c")        = Some(("ab", ""))
//! "abc".split_once("a")        = Some(("", "bc"))
//! "".split_once("ab")          = None
//! "đ".split_once("đ")          = Some(("", ""))
//! "ađb".split_once("đ")        = Some(("a", "b"))
//! bad=0
//! ```
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `c6e0a2a` + commit report):
//!
//! ```text
//! $ vis check probe.vis                  # use std.str; str.split_once(...), "...".split_once(...)
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.str` không có hàm `split_once`
//! exit=1
//! $ vis run --no-cache probe.vis
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `split_once`
//! probe.vis:6:11: error VS0403: `str` không có trường `split_once`
//! exit=2
//! $ vis run --no-cache --vm probe.vis
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `split_once`
//! probe.vis:6:11: error VS0403: `str` không có trường `split_once`
//! exit=2
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-spo-{tag}-{}-{n}", std::process::id()));
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
// 1 — khớp / không khớp: chỉ tách lần đầu
// ===========================================================================

/// `Some` tại lần khớp **đầu**, giữ nguyên mọi dấu phân cách sau; không khớp/`sep` dài hơn ⇒ `None`.
#[test]
fn khop_va_khong_khop_chi_lan_dau() {
    dung(
        "spo-co-ban",
        r#"use std.str

fn main():
    print(str.split_once("a,b,c", ","))
    print(str.split_once("aXbXc", "X"))
    print(str.split_once("a::b::c", "::"))
    print(str.split_once("aaaa", "aa"))
    print(str.split_once("abc", "a"))
    print(str.split_once("abc", "c"))
    print(str.split_once("a,b,c", "x"))
    print(str.split_once("abc", "abcd"))
    print(str.split_once("abc", "ac"))
    match str.split_once("a,b,c", ","):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
"#,
        "Some([a, b,c])\nSome([a, bXc])\nSome([a, b::c])\nSome([, aa])\nSome([, bc])\nSome([ab, ])\nNone\nNone\nNone\na|b,c\n",
    );
}

// ===========================================================================
// 2 — `sep` rỗng và `s` rỗng: bảng Rust `str::split_once("")`
// ===========================================================================

/// `sep` rỗng ⇒ `Some(["", s])`, kể cả `s` rỗng; `s` rỗng + `sep` khác rỗng ⇒ `None`.
#[test]
fn sep_rong_va_s_rong_theo_bang_rust() {
    dung(
        "spo-empty",
        r#"use std.str

fn main():
    print(str.split_once("abc", ""))
    print(str.split_once("", ""))
    print(str.split_once("", "a"))
    print(str.split_once("", "ab"))
    print(str.split_once("đ😀", ""))
"#,
        "Some([, abc])\nSome([, ])\nNone\nNone\nSome([, đ😀])\n",
    );
}

// ===========================================================================
// 3 — Unicode/emoji: khớp chuỗi con trên biên scalar
// ===========================================================================

/// Tiếng Việt + emoji + ZWJ: dấu phân cách nhiều byte vẫn khớp đúng; không sinh UTF-8 hỏng.
#[test]
fn unicode_emoji_khong_cat_scalar() {
    dung(
        "spo-unicode",
        r#"use std.str

fn main():
    print(str.split_once("đ,😀,x", ","))
    match str.split_once("đ,😀,x", ","):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
    print(str.split_once("a😀b😀c", "😀"))
    print(str.split_once("đ😀x", "😀"))
    print(str.split_once("ađbđc", "đ"))
    print(str.split_once("👨‍👩‍👧x", "👩"))
    match str.split_once("👨‍👩‍👧x", "👩"):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
    print(str.split_once("👨‍👩‍👧", "👨‍👩‍👧"))
    print(str.split_once("đ😀x", "😀a"))
"#,
        "Some([đ, 😀,x])\nđ|😀,x\nSome([a, b😀c])\nSome([đ, x])\nSome([a, bđc])\nSome([👨‍, ‍👧x])\n👨‍|‍👧x\nSome([, ])\nNone\n",
    );
}

// ===========================================================================
// 4 — `Some` luôn đúng **2** phần; kiểu `list[str]?` thật
// ===========================================================================

/// `len` payload luôn `2`; `match Some/None` chạy được; gán `list[str]?` hợp lệ.
#[test]
fn some_dung_hai_phan_va_option_that() {
    dung(
        "spo-hai-phan",
        r#"use std.str

fn main():
    match str.split_once("a,b,c", ","):
        Some(xs):
            print(len(xs))
            print(str.join(xs, "|"))
        None:
            print("none")
    match str.split_once("a,b,c", "x"):
        Some(_): print("some")
        None: print("none")
    let a: list[str]? = str.split_once("a,b", ",")
    let b: list[str]? = str.split_once("a,b", "x")
    print(a)
    print(b)
    match str.split_once("abc", ""):
        Some(xs): print(len(xs))
        None: print(-1)
    match str.split_once(",", ","):
        Some(xs): print(len(xs))
        None: print(-1)
"#,
        "2\na|b,c\nnone\nSome([a, b])\nNone\n2\n2\n",
    );
}

// ===========================================================================
// 5 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.split_once(sep)` ≡ `str.split_once(s, sep)`; kiểu trả qua sugar vẫn `list[str]?`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "spo-sugar",
        r#"fn main():
    let s: str = "a,b,c"
    print(s.split_once(","))
    print("a,b,c".split_once("x"))
    print(s.split_once(""))
    let o: list[str]? = s.split_once(",")
    print(o)
    print("abc".split_once("b"))
    print("đ😀x😀".split_once("😀"))
    match "đ😀x😀".split_once("😀"):
        Some(xs): print(xs.join("|"))
        None: print("none")
"#,
        "Some([a, b,c])\nNone\nSome([, a,b,c])\nSome([a, b,c])\nSome([a, c])\nSome([đ, x😀])\nđ|x😀\n",
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
        ("kieu-1", "str.split_once(1, \",\")", "VS0412"),
        ("kieu-2", "str.split_once(\"a\", 1)", "VS0412"),
        ("arity-1", "str.split_once(\"a\")", "VS0401"),
        ("arity-2", "str.split_once()", "VS0401"),
        ("arity-3", "str.split_once(\"a\", \",\", 3)", "VS0401"),
        ("sugar-kieu", "\"a\".split_once(1)", "VS0412"),
        ("sugar-arity-1", "\"a\".split_once()", "VS0401"),
        ("sugar-arity-2", "\"a\".split_once(\",\", 3)", "VS0401"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `list[str]?` thật: gán `list[str]?` hợp lệ (ca dương ở trên); gán `list[str]`,
/// `list[int]?`, `str?` đều bị chặn.
#[test]
fn ket_qua_la_list_str_option_that() {
    tu_choi(
        "spo-ket-qua-sang-list-str",
        "use std.str\n\nfn main():\n    let x: list[str] = str.split_once(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spo-ket-qua-sang-list-int-opt",
        "use std.str\n\nfn main():\n    let x: list[int]? = str.split_once(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spo-ket-qua-sang-str-opt",
        "use std.str\n\nfn main():\n    let x: str? = str.split_once(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spo-sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"a,b\".split_once(\",\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 7 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính (khớp/không khớp, `sep` rỗng, `s` rỗng, đầu/cuối, Unicode, emoji,
/// sugar) để khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "spo-hai-engine",
        r#"use std.str

fn main():
    let s: str = "a,b,c"
    print(str.split_once(s, ","))
    print(s.split_once(","))
    print(str.split_once(s, "x"))
    print(str.split_once("abc", ""))
    print(str.split_once("", ""))
    print(str.split_once("", ","))
    print(str.split_once("abc", "a"))
    print(str.split_once("abc", "c"))
    print(str.split_once("aXbXc", "X"))
    print(str.split_once("đ😀x😀", "😀"))
    print(str.split_once("👨‍👩‍👧x", "👩"))
    match str.split_once("a,b,c", ","):
        Some(xs): print(len(xs))
        None: print(-1)
    match str.split_once("a,b,c", ","):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
"#,
        "Some([a, b,c])\nSome([a, b,c])\nNone\nSome([, abc])\nSome([, ])\nNone\nSome([, bc])\nSome([ab, ])\nSome([a, bXc])\nSome([đ, x😀])\nSome([👨‍, ‍👧x])\n2\na|b,c\n",
    );
}
