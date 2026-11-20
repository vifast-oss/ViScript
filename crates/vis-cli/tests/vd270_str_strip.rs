// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strstrip)** — `std.str.strip_prefix(s, p) -> str?`, `str.strip_suffix(s, p) -> str?`.
//!
//! Hợp đồng được khoá ở đây:
//! - Khớp **đúng nguyên** tiền tố/hậu tố, bỏ **một lần**: `strip_prefix("banana", "ban")`
//!   ⇒ `Some("ana")`, `strip_suffix("banana", "ana")` ⇒ `Some("ban")`; không khớp/khớp giữa
//!   chuỗi ⇒ `None`.
//! - Mốc rỗng ⇒ `Some(s)` **nguyên vẹn** (Rust `str::strip_prefix`/`strip_suffix`), kể cả
//!   `strip_prefix("", "") == Some("")`; mốc khác rỗng trên chuỗi rỗng ⇒ `None`.
//! - Chồng lấn chỉ bỏ **một lần** ở đúng đầu/cuối: `"aaa"` bỏ `"a"` ⇒ `Some("aa")`, bỏ `"aa"`
//!   ⇒ `Some("a")`; `"aaaa"` bỏ `"aa"` ⇒ `Some("aa")` (không lặp tới cạn).
//! - Đơn vị khớp theo **byte** nhưng biên luôn là **ranh giới scalar Unicode** (không cắt giữa
//!   scalar): `"đ😀x"` bỏ `"đ"` ⇒ `Some("😀x")`, bỏ hậu tố `"😀x"` ⇒ `Some("đ")`.
//! - Kiểu trả **`str?`** (`Ty::OptStr` → `Option[str]`) là `Option` **thật**: `match Some/None`,
//!   `let x: str? = …` hợp lệ, gán vào `str` ⇒ `VS0412`.
//! - Sugar R5c `s.strip_prefix(…)`/`s.strip_suffix(…)` ≡ `str.*(s, …)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `78b9626`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.str
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.str` không có hàm `strip_prefix`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `strip_prefix`
//! probe.vis:6:11: error VS0301: module `std.str` không có hàm `strip_prefix`
//! probe.vis:7:11: error VS0301: module `std.str` không có hàm `strip_suffix`
//! probe.vis:8:11: error VS0301: module `std.str` không có hàm `strip_suffix`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng bốn dòng)
//! $ vis check sugar.vis                    # "banana".strip_prefix("ban"), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `str` không có trường `strip_prefix`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-ss-{tag}-{}-{n}", std::process::id()));
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
// 1 — ASCII: khớp đầu/cuối, không khớp, khớp giữa chuỗi
// ===========================================================================

/// Khớp đúng đầu/cuối ⇒ `Some(phần còn lại)`; sai vị trí/không có/khoá dài hơn ⇒ `None`.
#[test]
fn ascii_khop_va_khong_khop() {
    dung(
        "ascii",
        r#"use std.str

fn main():
    print(str.strip_prefix("banana", "ban"))
    print(str.strip_prefix("banana", "x"))
    print(str.strip_prefix("banana", "ana"))
    print(str.strip_prefix("abc", "abcd"))
    print(str.strip_suffix("banana", "ana"))
    print(str.strip_suffix("banana", "x"))
    print(str.strip_suffix("banana", "ban"))
    print(str.strip_suffix("abc", "abcd"))
    print(str.strip_prefix("banana", "banana"))
    print(str.strip_suffix("banana", "banana"))
"#,
        "Some(ana)\nNone\nNone\nNone\nSome(ban)\nNone\nNone\nNone\nSome()\nSome()\n",
    );
}

// ===========================================================================
// 2 — mốc rỗng: trả nguyên chuỗi (nghĩa Rust)
// ===========================================================================

/// Mốc rỗng ⇒ `Some(s)` nguyên vẹn, kể cả chuỗi rỗng; mốc khác rỗng trên chuỗi rỗng ⇒ `None`.
#[test]
fn moc_rong_tra_nguyen_chuoi() {
    dung(
        "empty-needle",
        r#"use std.str

fn main():
    print(str.strip_prefix("abc", ""))
    print(str.strip_suffix("abc", ""))
    print(str.strip_prefix("", ""))
    print(str.strip_suffix("", ""))
    print(str.strip_prefix("", "a"))
    print(str.strip_suffix("", "a"))
    print(str.strip_prefix(" ", ""))
    print(str.strip_suffix("đ😀", ""))
"#,
        "Some(abc)\nSome(abc)\nSome()\nSome()\nNone\nNone\nSome( )\nSome(đ😀)\n",
    );
}

// ===========================================================================
// 3 — chồng lấn: chỉ bỏ một lần, đúng đầu/cuối
// ===========================================================================

/// `"aaa"` bỏ `"a"` ⇒ `Some("aa")` (không lặp); bỏ `"aa"` ⇒ `Some("a")`; `"aaaa"` bỏ `"aa"`
/// ⇒ `Some("aa")`; khoá dài hơn chuỗi ⇒ `None`.
#[test]
fn chong_lan_chi_bo_mot_lan() {
    dung(
        "overlap",
        r#"use std.str

fn main():
    print(str.strip_prefix("aaa", "a"))
    print(str.strip_suffix("aaa", "a"))
    print(str.strip_prefix("aaa", "aa"))
    print(str.strip_suffix("aaa", "aa"))
    print(str.strip_prefix("aaa", "aaa"))
    print(str.strip_suffix("aaa", "aaa"))
    print(str.strip_prefix("aaaa", "aa"))
    print(str.strip_suffix("aaaa", "aa"))
    print(str.strip_prefix("aaa", "aaaa"))
    print(str.strip_suffix("aaa", "aaaa"))
"#,
        "Some(aa)\nSome(aa)\nSome(a)\nSome(a)\nSome()\nSome()\nSome(aa)\nSome(aa)\nNone\nNone\n",
    );
}

// ===========================================================================
// 4 — Unicode: ranh giới scalar, không phải byte
// ===========================================================================

/// Tiếng Việt/emoji: bỏ đúng scalar ở đầu/cuối; kết quả không bao giờ là UTF-8 hỏng.
#[test]
fn unicode_theo_scalar_khong_phai_byte() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.strip_prefix("đ😀x", "đ"))
    print(str.strip_suffix("đ😀x", "😀x"))
    print(str.strip_prefix("đ😀x", "đ😀"))
    print(str.strip_suffix("đ😀x", "x"))
    print(str.strip_prefix("😀a", "😀"))
    print(str.strip_suffix("a😀", "😀"))
    print(str.strip_prefix("😀😀", "😀"))
    print(str.strip_suffix("😀😀", "😀"))
    print(str.strip_prefix("đ", "d"))
    print(str.strip_suffix("đ", "d"))
"#,
        "Some(😀x)\nSome(đ)\nSome(x)\nSome(đ😀)\nSome(a)\nSome(a)\nSome(😀)\nSome(😀)\nNone\nNone\n",
    );
}

// ===========================================================================
// 5 — `str?` là `Option[str]` thật
// ===========================================================================

/// `match Some/None` chạy được; gán `str?`/`str` đúng chiều; in `Some(x)` không kèm nháy.
#[test]
fn ket_qua_la_option_str_that() {
    dung(
        "option-that",
        r#"use std.str

fn main():
    match str.strip_prefix("banana", "ban"):
        Some(r): print(r)
        None: print("none")
    match str.strip_prefix("banana", "zz"):
        Some(_): print("some")
        None: print("none")
    match str.strip_suffix("banana", "ana"):
        Some(r): print(r)
        None: print("none")
    match str.strip_suffix("banana", "zz"):
        Some(_): print("some")
        None: print("none")
    let a: str? = str.strip_prefix("abc", "a")
    let b: str? = str.strip_suffix("abc", "c")
    print(a)
    print(b)
"#,
        "ana\nnone\nban\nnone\nSome(bc)\nSome(ab)\n",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.strip_prefix(…)`/`s.strip_suffix(…)` ≡ `str.*(s, …)`; kiểu trả qua sugar vẫn `str?`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "banana"
    print(s.strip_prefix("ban"))
    print(s.strip_suffix("ana"))
    print(s.strip_prefix("x"))
    let o: str? = s.strip_prefix("ban")
    print(o)
    print("đ😀x".strip_prefix("đ"))
    print("đ😀x".strip_suffix("😀x"))
    print("aaa".strip_prefix("a"))
    print("aaa".strip_suffix("aa"))
    match "banana".strip_suffix("ana"):
        Some(r): print(r)
        None: print("none")
"#,
        "Some(ana)\nSome(ban)\nNone\nSome(ana)\nSome(😀x)\nSome(đ)\nSome(aa)\nSome(a)\nban\n",
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
        ("kieu-1", "str.strip_prefix(1, \"a\")", "VS0412"),
        ("kieu-2", "str.strip_prefix(\"a\", 1)", "VS0412"),
        ("kieu-3", "str.strip_suffix(1, \"a\")", "VS0412"),
        ("kieu-4", "str.strip_suffix(\"a\", 1)", "VS0412"),
        ("arity-1", "str.strip_prefix(\"a\")", "VS0401"),
        ("arity-2", "str.strip_prefix(\"a\", \"b\", \"c\")", "VS0401"),
        ("arity-3", "str.strip_suffix()", "VS0401"),
        ("arity-4", "str.strip_suffix(\"a\")", "VS0401"),
        ("sugar-kieu", "\"a\".strip_prefix(1)", "VS0412"),
        ("sugar-kieu-2", "\"a\".strip_suffix(1)", "VS0412"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Gán kết quả (`str?`) vào `str` bị chặn; gán vào `str?` hợp lệ (ca dương ở test 5).
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    tu_choi(
        "prefix-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.strip_prefix(\"abc\", \"a\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "suffix-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.strip_suffix(\"abc\", \"c\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-prefix-sang-str",
        "fn main():\n    let x: str = \"abc\".strip_prefix(\"a\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-suffix-sang-str",
        "fn main():\n    let x: str = \"abc\".strip_suffix(\"c\")\n    print(x)\n",
        "VS0412",
    );
}

/// Sugar R5c chưa kiểm **arity** ở checker (nợ chung của R5c — chỉ kiểm kiểu đối số có mặt);
/// lời gọi thiếu đối số vẫn bị **tầng chạy** chặn: trap `VS0501` nêu `str.strip_prefix`, exit
/// 101 ở **cả hai** engine, không panic. (Module `str.strip_prefix()` thì checker bắt `VS0401`.)
#[test]
fn sugar_sai_arity_la_trap_ca_hai_engine() {
    for (tag, call) in [
        ("sugar-arity-prefix", "\"a\".strip_prefix()"),
        ("sugar-arity-suffix", "\"a\".strip_suffix()"),
    ] {
        let (code, out, err) = hai_engine(tag, &format!("fn main():\n    print({call})\n"));
        assert_eq!(
            code, 101,
            "{tag}: phải trap (exit 101)\nstdout: {out}\nstderr: {err}"
        );
        assert!(err.contains("VS0501"), "{tag}: thiếu VS0501: {err}");
        assert!(
            err.contains("str.strip_"),
            "{tag}: stderr thiếu tên hàm `str.strip_*`: {err}"
        );
    }
}
