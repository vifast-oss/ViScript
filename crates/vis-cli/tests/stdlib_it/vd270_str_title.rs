// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (str-title)** — `std.str.title(s) -> str`: chữ hoa **scalar đầu** mỗi từ, chữ thường
//! phần còn lại.
//!
//! Hợp đồng được khoá ở đây:
//! - Từ tách theo **khoảng trắng Unicode** (`char::is_whitespace`), **không** theo dấu câu:
//!   `"hello-world"` ⇒ `"Hello-world"`, `"hello, world!"` ⇒ `"Hello, World!"`.
//! - Mỗi từ: **scalar đầu** viết hoa, **mọi scalar sau** viết thường — đơn vị **scalar Unicode**,
//!   không phải byte/grapheme; phép đổi có thể **nở** (`ß` ⇒ `SS`, `"ßig"` ⇒ `"SSig"`).
//! - Khoảng trắng **giữ nguyên từng scalar, đúng thứ tự** (dấu cách, tab, `\n`, `\r\n`, NBSP,
//!   ideographic space); chuỗi rỗng / chỉ khoảng trắng ⇒ **nguyên vẹn**; không trap.
//! - Kiểu `(str) -> str`; sugar R5c `s.title()` ≡ `str.title(s)`, không cần `use`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `target/debug/vis` dựng từ cây này @ `09823e1`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.str, 2 lời gọi
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 4:11 module `std.str` không có hàm `title`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:4:11: error VS0301: module `std.str` không có hàm `title`
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `title`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng hai dòng)
//! $ vis check sugar.vis                    # "hello world".title(), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `title`
//! $ vis run --no-cache sugar.vis           ⇒ exit 2
//! sugar.vis:2:11: error VS0403: `str` không có trường `title`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-st-{tag}-{}-{n}", std::process::id()));
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
// 1 — ASCII: hoa đầu từ, thường phần còn lại; đầu vào trộn hoa/thường
// ===========================================================================

#[test]
fn ascii_hoa_dau_tu_va_tron_hoa_thuong() {
    dung(
        "ascii",
        r#"use std.str

fn main():
    print(str.title("hello world"))
    print(str.title("hello"))
    print(str.title("a b c"))
    print(str.title("hELLo wORLD"))
    print(str.title("mIxEd CaSe TeXt"))
"#,
        "Hello World\nHello\nA B C\nHello World\nMixed Case Text\n",
    );
}

// ===========================================================================
// 2 — khoảng trắng: nhiều dấu cách, tab, xuống dòng, CRLF, Unicode — giữ nguyên
// ===========================================================================

#[test]
fn khoang_trang_giu_nguyen_tung_scalar() {
    dung(
        "khoang-trang",
        r#"use std.str

fn main():
    print(str.title("a  b\tc\nd"))
    print(str.title("  a\r\n b  "))
    print(str.title("a\u{00A0}b\u{3000}c"))
    print(str.byte_len(str.title("\r\n")))
"#,
        "A  B\tC\nD\n  A\r\n B  \nA\u{00a0}B\u{3000}C\n2\n",
    );
}

// ===========================================================================
// 3 — rỗng / chỉ khoảng trắng: nguyên vẹn, không trap
// ===========================================================================

#[test]
fn rong_va_chi_khoang_trang_nguyen_ven() {
    dung(
        "rong",
        r#"use std.str

fn main():
    print(str.title(""))
    print(str.title("   "))
    print(str.title("\t\n\r"))
    print(str.char_count(str.title("   ")))
    print(str.char_count(str.title("\t\n\r")))
"#,
        "\n   \n\t\n\r\n3\n3\n",
    );
}

// ===========================================================================
// 4 — dấu câu / gạch nối: biên từ **chỉ** là khoảng trắng
// ===========================================================================

#[test]
fn dau_cau_va_gach_noi() {
    dung(
        "dau-cau",
        r#"use std.str

fn main():
    print(str.title("hello-world"))
    print(str.title("hello, world!"))
    print(str.title("don't STOP"))
    print(str.title("foo_bar-baz"))
"#,
        "Hello-world\nHello, World!\nDon't Stop\nFoo_bar-baz\n",
    );
}

// ===========================================================================
// 5 — Unicode: tiếng Việt + scalar astral (emoji)
// ===========================================================================

#[test]
fn unicode_viet_va_astral() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.title("đẶNG văn hÙNG"))
    print(str.title("XIN CHÀO THẾ GIỚI"))
    print(str.title("😀abc 😀dEF"))
    print(str.title("👨‍👩‍👧 FAMILY"))
    print(str.char_count(str.title("😀ABC")))
"#,
        "Đặng Văn Hùng\nXin Chào Thế Giới\n😀abc 😀def\n👨‍👩‍👧 Family\n4\n",
    );
}

// ===========================================================================
// 6 — `ß` nở thành hai scalar (`SS`) ở vị trí scalar đầu; giữ nguyên khi ở sau
// ===========================================================================

#[test]
fn ss_no_thanh_hai_scalar() {
    dung(
        "ss",
        r#"use std.str

fn main():
    print(str.title("ßig ß"))
    print(str.title("ẞIG"))
    print(str.title("aß"))
    print(str.char_count(str.title("ß")))
"#,
        "SSig SS\nẞig\nAß\n2\n",
    );
}

// ===========================================================================
// 7 — sugar phương thức R5c (không cần `use`) + kết quả là `str`
// ===========================================================================

#[test]
fn sugar_va_ket_qua_la_str() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "hELLo wORLD"
    print(s.title())
    print("hello-world".title())
    print("".title())
    let t: str = "ok".title()
    print(t)
"#,
        "Hello World\nHello-world\n\nOk\n",
    );
}

// ===========================================================================
// 8 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu", "str.title(1)", "VS0412"),
        ("arity-thieu", "str.title()", "VS0401"),
        ("arity-thua", "str.title(\"a\", \"b\")", "VS0401"),
        ("sugar-arity-thua", "\"a\".title(\"b\")", "VS0401"),
        ("sugar-nhan-sai-kieu", "(1).title()", "VS0403"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.title(\"abc\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"abc\".title()\n    print(x)\n",
        "VS0412",
    );
}
