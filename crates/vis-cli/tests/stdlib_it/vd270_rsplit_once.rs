// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (rsplit-once)** — `std.str.rsplit_once(s, sep) -> list[str]?`.
//!
//! Hợp đồng được khoá ở đây:
//! - Tách tại **lần xuất hiện cuối cùng** của `sep`, **không** tách các lần trước:
//!   `rsplit_once("a,b,c", ",")` ⇒ `Some(["a,b", "c"])`;
//!   `rsplit_once("aXbXc", "X")` ⇒ `Some(["aXb", "c"])`.
//! - Không có `sep` trong `s` (hoặc `sep` dài hơn `s`) ⇒ `None`.
//! - `sep` rỗng ⇒ `Some([s, ""])` — **biên cuối** (khác `split_once`: `Some(["", s])`);
//!   `s` rỗng + `sep` khác rỗng ⇒ `None`; `s` rỗng + `sep` rỗng ⇒ `Some(["", ""])`
//!   (bảng đã kiểm bằng probe Rust, xem dưới). **Không** trap.
//! - Dấu phân cách ở **đầu**/`cuối` sinh phần rỗng tương ứng:
//!   `rsplit_once("abc", "a")` ⇒ `Some(["", "bc"])`; `rsplit_once("abc", "c")` ⇒
//!   `Some(["ab", ""])`.
//! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
//!   **không bao giờ** cắt giữa scalar: `rsplit_once("đ😀x", "😀")` ⇒ `Some(["đ", "x"])`;
//!   ZWJ trong emoji không phải biên đặc biệt: `rsplit_once("👨‍👩‍👧x", "👩")` ⇒
//!   `Some(["👨‍", "‍👧x"])`, phần còn lại vẫn là UTF-8 hợp lệ.
//! - `Some` luôn có **đúng 2** phần tử; chỉ cấp phát hai chuỗi kết quả.
//! - Chữ ký `(str, str) -> list[str]?` (tái dùng `Ty::OptListStr` = `Option[list[str]]` thật);
//!   sugar R5c `s.rsplit_once(sep)` ≡ `str.rsplit_once(s, sep)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **Probe Rust** (`rustc 1.100.0`, `str::rsplit_once`) — 24/24 khớp:
//!
//! ```text
//! "a,b,c".rsplit_once(",")      = Some(("a,b", "c"))
//! "a,b,c".rsplit_once("x")      = None
//! "abc".rsplit_once("")         = Some(("abc", ""))
//! "".rsplit_once("a")           = None
//! "".rsplit_once("")            = Some(("", ""))
//! "abc".rsplit_once("abcd")     = None
//! "a::b::c".rsplit_once("::")   = Some(("a::b", "c"))
//! "đ😀x".rsplit_once("😀")       = Some(("đ", "x"))
//! "👨‍👩‍👧x".rsplit_once("👩")      = Some(("👨‍", "‍👧x"))
//! "aXbXc".rsplit_once("X")      = Some(("aXb", "c"))
//! "abc".rsplit_once("c")        = Some(("ab", ""))
//! "abc".rsplit_once("a")        = Some(("", "bc"))
//! "".rsplit_once("ab")          = None
//! "đ".rsplit_once("đ")          = Some(("", ""))
//! "ađb".rsplit_once("đ")        = Some(("a", "b"))
//! "aaaa".rsplit_once("aa")      = Some(("aa", ""))
//! "aaa".rsplit_once("aa")       = Some(("a", ""))
//! "đ,😀,x".rsplit_once(",")     = Some(("đ,😀", "x"))
//! "a😀b😀c".rsplit_once("😀")     = Some(("a😀b", "c"))
//! "👨‍👩‍👧".rsplit_once("👨‍👩‍👧")   = Some(("", ""))
//! ",".rsplit_once(",")          = Some(("", ""))
//! ",a,".rsplit_once(",")        = Some((",a", ""))
//! "abc".rsplit_once("b")        = Some(("a", "c"))
//! "banana".rsplit_once("na")    = Some(("bana", ""))
//! bad=0
//! ```
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `88da99c` + commit report):
//!
//! ```text
//! $ vis check probe.vis                  # use std.str; str.rsplit_once(...), "...".rsplit_once(...)
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 4:13 module `std.str` không có hàm `rsplit_once`
//! check_exit=1
//! $ vis run --no-cache probe.vis
//! probe.vis:4:13: error VS0301: module `std.str` không có hàm `rsplit_once`
//! probe.vis:5:13: error VS0403: `str` không có trường `rsplit_once`
//! run_exit=2
//! $ vis run --no-cache --vm probe.vis
//! probe.vis:4:13: error VS0301: module `std.str` không có hàm `rsplit_once`
//! probe.vis:5:13: error VS0403: `str` không có trường `rsplit_once`
//! vm_exit=2
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-rspo-{tag}-{}-{n}", std::process::id()));
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
// 1 — bảng probe: khớp/không khớp, chỉ tách lần cuối
// ===========================================================================

/// Bảng probe Rust 24 ca (dạng `print`/`match`) + `Some`/`None`/`len`; chỉ tách lần **cuối**.
#[test]
fn bang_probe_rust_va_chi_tach_lan_cuoi() {
    dung(
        "rspo-probe",
        r#"use std.str

fn main():
    print(str.rsplit_once("a,b,c", ","))
    print(str.rsplit_once("a,b,c", "x"))
    print(str.rsplit_once("abc", ""))
    print(str.rsplit_once("", "a"))
    print(str.rsplit_once("", ""))
    print(str.rsplit_once("abc", "abcd"))
    print(str.rsplit_once("a::b::c", "::"))
    print(str.rsplit_once("đ😀x", "😀"))
    print(str.rsplit_once("👨‍👩‍👧x", "👩"))
    print(str.rsplit_once("aXbXc", "X"))
    print(str.rsplit_once("abc", "c"))
    print(str.rsplit_once("abc", "a"))
    print(str.rsplit_once("", "ab"))
    print(str.rsplit_once("đ", "đ"))
    print(str.rsplit_once("ađb", "đ"))
    print(str.rsplit_once("aaaa", "aa"))
    print(str.rsplit_once("aaa", "aa"))
    print(str.rsplit_once("đ,😀,x", ","))
    print(str.rsplit_once("a😀b😀c", "😀"))
    print(str.rsplit_once("👨‍👩‍👧", "👨‍👩‍👧"))
    print(str.rsplit_once(",", ","))
    print(str.rsplit_once(",a,", ","))
    print(str.rsplit_once("abc", "b"))
    print(str.rsplit_once("banana", "na"))
    match str.rsplit_once("a,b,c", ","):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
    match str.rsplit_once("a,b,c", "x"):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
"#,
        "Some([a,b, c])\nNone\nSome([abc, ])\nNone\nSome([, ])\nNone\nSome([a::b, c])\nSome([đ, x])\nSome([👨‍, ‍👧x])\nSome([aXb, c])\nSome([ab, ])\nSome([, bc])\nNone\nSome([, ])\nSome([a, b])\nSome([aa, ])\nSome([a, ])\nSome([đ,😀, x])\nSome([a😀b, c])\nSome([, ])\nSome([, ])\nSome([,a, ])\nSome([a, c])\nSome([bana, ])\na,b|c\nnone\n",
    );
}

// ===========================================================================
// 2 — `sep` rỗng và `s` rỗng: bảng Rust `str::rsplit_once("")`
// ===========================================================================

/// `sep` rỗng ⇒ `Some([s, ""])` — **biên cuối** (khác `split_once`); `s` rỗng + `sep` khác rỗng
/// ⇒ `None`.
#[test]
fn sep_rong_va_s_rong_theo_bang_rust() {
    dung(
        "rspo-empty",
        r#"use std.str

fn main():
    print(str.rsplit_once("abc", ""))
    print(str.rsplit_once("", ""))
    print(str.rsplit_once("", "a"))
    print(str.rsplit_once("", "ab"))
    print(str.rsplit_once("đ😀", ""))
    print(str.rsplit_once("a,b,c", ""))
    match str.rsplit_once("abc", ""):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
"#,
        "Some([abc, ])\nSome([, ])\nNone\nNone\nSome([đ😀, ])\nSome([a,b,c, ])\nabc|\n",
    );
}

// ===========================================================================
// 3 — Unicode/emoji/ZWJ: khớp chuỗi con trên biên scalar
// ===========================================================================

/// Tiếng Việt + emoji + ZWJ: dấu phân cách nhiều byte vẫn khớp đúng lần **cuối**; không sinh
/// UTF-8 hỏng.
#[test]
fn unicode_emoji_khong_cat_scalar() {
    dung(
        "rspo-unicode",
        r#"use std.str

fn main():
    print(str.rsplit_once("đ,😀,x", ","))
    match str.rsplit_once("đ,😀,x", ","):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
    print(str.rsplit_once("a😀b😀c", "😀"))
    print(str.rsplit_once("đ😀x", "😀"))
    print(str.rsplit_once("ađbđc", "đ"))
    print(str.rsplit_once("👨‍👩‍👧x", "👩"))
    match str.rsplit_once("👨‍👩‍👧x", "👩"):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
    print(str.rsplit_once("👨‍👩‍👧", "👨‍👩‍👧"))
    print(str.rsplit_once("đ😀x", "😀a"))
"#,
        "Some([đ,😀, x])\nđ,😀|x\nSome([a😀b, c])\nSome([đ, x])\nSome([ađb, c])\nSome([👨‍, ‍👧x])\n👨‍|‍👧x\nSome([, ])\nNone\n",
    );
}

// ===========================================================================
// 4 — `Some` luôn đúng **2** phần; kiểu `list[str]?` thật
// ===========================================================================

/// `len` payload luôn `2`; `match Some/None` chạy được; gán `list[str]?` hợp lệ.
#[test]
fn some_dung_hai_phan_va_option_that() {
    dung(
        "rspo-hai-phan",
        r#"use std.str

fn main():
    match str.rsplit_once("a,b,c", ","):
        Some(xs):
            print(len(xs))
            print(str.join(xs, "|"))
        None:
            print("none")
    match str.rsplit_once("a,b,c", "x"):
        Some(_): print("some")
        None: print("none")
    let a: list[str]? = str.rsplit_once("a,b", ",")
    let b: list[str]? = str.rsplit_once("a,b", "x")
    print(a)
    print(b)
    match str.rsplit_once("abc", ""):
        Some(xs): print(len(xs))
        None: print(-1)
    match str.rsplit_once(",", ","):
        Some(xs): print(len(xs))
        None: print(-1)
"#,
        "2\na,b|c\nnone\nSome([a, b])\nNone\n2\n2\n",
    );
}

// ===========================================================================
// 5 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.rsplit_once(sep)` ≡ `str.rsplit_once(s, sep)`; kiểu trả qua sugar vẫn `list[str]?`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "rspo-sugar",
        r#"fn main():
    let s: str = "a,b,c"
    print(s.rsplit_once(","))
    print("a,b,c".rsplit_once("x"))
    print(s.rsplit_once(""))
    let o: list[str]? = s.rsplit_once(",")
    print(o)
    print("abc".rsplit_once("b"))
    print("đ😀x😀".rsplit_once("😀"))
    match "đ😀x😀".rsplit_once("😀"):
        Some(xs): print(xs.join("|"))
        None: print("none")
"#,
        "Some([a,b, c])\nNone\nSome([a,b,c, ])\nSome([a,b, c])\nSome([a, c])\nSome([đ😀x, ])\nđ😀x|\n",
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
        ("kieu-1", "str.rsplit_once(1, \",\")", "VS0412"),
        ("kieu-2", "str.rsplit_once(\"a\", 1)", "VS0412"),
        ("arity-1", "str.rsplit_once(\"a\")", "VS0401"),
        ("arity-2", "str.rsplit_once()", "VS0401"),
        ("arity-3", "str.rsplit_once(\"a\", \",\", 3)", "VS0401"),
        ("sugar-kieu", "\"a\".rsplit_once(1)", "VS0412"),
        ("sugar-arity-1", "\"a\".rsplit_once()", "VS0401"),
        ("sugar-arity-2", "\"a\".rsplit_once(\",\", 3)", "VS0401"),
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
        "rspo-ket-qua-sang-list-str",
        "use std.str\n\nfn main():\n    let x: list[str] = str.rsplit_once(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "rspo-ket-qua-sang-list-int-opt",
        "use std.str\n\nfn main():\n    let x: list[int]? = str.rsplit_once(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "rspo-ket-qua-sang-str-opt",
        "use std.str\n\nfn main():\n    let x: str? = str.rsplit_once(\"a,b\", \",\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "rspo-sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"a,b\".rsplit_once(\",\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 7 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính (khớp/không khớp, `sep` rỗng, `s` rỗng, đầu/cuối, Unicode, emoji,
/// ZWJ, sugar) để khoá **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "rspo-hai-engine",
        r#"use std.str

fn main():
    let s: str = "a,b,c"
    print(str.rsplit_once(s, ","))
    print(s.rsplit_once(","))
    print(str.rsplit_once(s, "x"))
    print(str.rsplit_once("abc", ""))
    print(str.rsplit_once("", ""))
    print(str.rsplit_once("", ","))
    print(str.rsplit_once("abc", "a"))
    print(str.rsplit_once("abc", "c"))
    print(str.rsplit_once("aXbXc", "X"))
    print(str.rsplit_once("đ😀x😀", "😀"))
    print(str.rsplit_once("👨‍👩‍👧x", "👩"))
    match str.rsplit_once("a,b,c", ","):
        Some(xs): print(len(xs))
        None: print(-1)
    match str.rsplit_once("a,b,c", ","):
        Some(xs): print(str.join(xs, "|"))
        None: print("none")
"#,
        "Some([a,b, c])\nSome([a,b, c])\nNone\nSome([abc, ])\nSome([, ])\nNone\nSome([, bc])\nSome([ab, ])\nSome([aXb, c])\nSome([đ😀x, ])\nSome([👨‍, ‍👧x])\n2\na,b|c\n",
    );
}
