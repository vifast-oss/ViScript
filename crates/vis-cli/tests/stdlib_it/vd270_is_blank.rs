// SPDX-License-Identifier: Apache-2.0
//! **VD-270 (is-blank)** — `std.str.is_blank(s) -> bool`.
//!
//! Hợp đồng được khoá ở đây:
//! - `true` **khi và chỉ khi** `s` rỗng **hoặc mọi scalar Unicode** của `s` đều là khoảng
//!   trắng theo `char::is_whitespace` (Unicode `White_Space`).
//! - Một scalar **không** phải khoảng trắng ở bất kỳ vị trí nào ⇒ `false` — kể cả scalar vô
//!   hình như ZWSP `U+200B`/BOM `U+FEFF`/MVS `U+180E` (không thuộc `White_Space`).
//! - Đơn vị là **scalar**, không byte/grapheme: dấu kết hợp `U+0301` **không** phải khoảng
//!   trắng nên `" \u{301} "` ⇒ `false`.
//! - Không trap, không cấp phát, O(số scalar), dừng ở scalar không phải khoảng trắng đầu tiên.
//! - Chữ ký `(str) -> bool`; sugar R5c `s.is_blank()` chạy **không** cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit;
//! ca âm chạy `vis check` + cả hai engine.
//!
//! **RED-BEFORE** (binary `target/debug/vis` dựng từ cây này @ `330d86c`, **trước** khi thêm
//! hàm):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_is_blank
//! running 9 tests
//! test ket_qua_sai_kieu_bi_chan ... FAILED
//! test sai_kieu_bi_chan ... FAILED
//! test sai_so_doi_bi_chan ... FAILED
//! test rong_va_khoang_trang_ascii ... FAILED
//! test khoang_trang_unicode ... FAILED
//! test hon_hop_khoang_trang ... FAILED
//! test khong_blank ... FAILED
//! test unicode_va_khoang_trang ... FAILED
//! test sugar_phuong_thuc_khong_can_use ... FAILED
//!
//! test result: FAILED. 0 passed; 9 failed
//! # mọi ca dương: VS0301 4:11 module `std.str` không có hàm `is_blank`
//! # sugar: VS0403 `str` không có trường `is_blank`
//! ```
//!
//! Sau khi thêm: 9/9 xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-ib-{tag}-{}-{n}", std::process::id()));
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
// 1 — rỗng + khoảng trắng ASCII (dấu cách, tab, LF, CR, CRLF, VT, FF)
// ===========================================================================

/// `""` ⇒ `true`; mọi tổ hợp khoảng trắng ASCII ⇒ `true` (không trap, không cấp phát).
#[test]
fn rong_va_khoang_trang_ascii() {
    dung(
        "ascii",
        r#"use std.str

fn main():
    print(str.is_blank(""))
    print(str.is_blank(" "))
    print(str.is_blank("   "))
    print(str.is_blank("\t"))
    print(str.is_blank("\n"))
    print(str.is_blank("\r"))
    print(str.is_blank("\r\n"))
    print(str.is_blank("\t \r\n"))
    print(str.is_blank("\r\n\t "))
    print(str.is_blank("\u{000B}"))
    print(str.is_blank("\u{000C}"))
    let s = "  \t\n "
    print(str.is_blank(s))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 2 — khoảng trắng Unicode: NBSP, ideographic space, phân cách dòng/đoạn, …
// ===========================================================================

/// `char::is_whitespace` theo **scalar**: NBSP/ideographic space/NEL/LS/PS/… đều ⇒ `true`.
#[test]
fn khoang_trang_unicode() {
    dung(
        "unicode-ws",
        r#"use std.str

fn main():
    print(str.is_blank("\u{00A0}"))
    print(str.is_blank("\u{3000}"))
    print(str.is_blank("\u{0085}"))
    print(str.is_blank("\u{1680}"))
    print(str.is_blank("\u{2000}"))
    print(str.is_blank("\u{2003}"))
    print(str.is_blank("\u{2028}"))
    print(str.is_blank("\u{2029}"))
    print(str.is_blank("\u{202F}"))
    print(str.is_blank("\u{205F}"))
"#,
        "true\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 3 — trộn nhiều loại khoảng trắng ⇒ true
// ===========================================================================

/// Trộn ASCII + Unicode whitespace ở nhiều thứ tự vẫn ⇒ `true` (mọi scalar đều trắng).
#[test]
fn hon_hop_khoang_trang() {
    dung(
        "hon-hop",
        r#"use std.str

fn main():
    print(str.is_blank(" \t\r\n\u{00A0}\u{3000}"))
    print(str.is_blank("\u{3000}\u{00A0}\r\n\t "))
    print(str.is_blank("\u{2028}\u{2029}\u{2003}\u{0085}"))
"#,
        "true\ntrue\ntrue\n",
    );
}

// ===========================================================================
// 4 — không blank: một scalar khác là đủ, kể cả scalar vô hình / dấu kết hợp
// ===========================================================================

/// Một scalar **không** trắng ở đầu/giữa/cuối ⇒ `false`; ZWSP/BOM/MVS/NUL/dấu kết hợp
/// **không** phải khoảng trắng (`char::is_whitespace`), nên cũng ⇒ `false`.
#[test]
fn khong_blank() {
    dung(
        "khong-blank",
        r#"use std.str

fn main():
    print(str.is_blank("a"))
    print(str.is_blank(" a"))
    print(str.is_blank("a "))
    print(str.is_blank(" a "))
    print(str.is_blank("a b"))
    print(str.is_blank(" \t\n0\r\n"))
    print(str.is_blank("  x  "))
    print(str.is_blank(" - "))
    print(str.is_blank("\u{200B}"))
    print(str.is_blank("\u{FEFF}"))
    print(str.is_blank("\u{180E}"))
    print(str.is_blank("\u{0301}"))
    print(str.is_blank(" \u{0301} "))
    print(str.is_blank("\0"))
"#,
        "false\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 5 — Unicode: tiếng Việt, emoji, chữ Hy Lạp; khoảng trắng bao quanh không "cứu"
// ===========================================================================

/// Chữ/emoji (kể cả astral) là scalar không trắng ⇒ `false`, dù có khoảng trắng hai bên.
#[test]
fn unicode_va_khoang_trang() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.is_blank("xin chào"))
    print(str.is_blank("đ"))
    print(str.is_blank("đ "))
    print(str.is_blank(" đ"))
    print(str.is_blank("😀"))
    print(str.is_blank(" \u{1F600} "))
    print(str.is_blank("👨‍👩‍👧"))
    print(str.is_blank("Σ"))
    print(str.is_blank("ß"))
"#,
        "false\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\nfalse\n",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use`)
// ===========================================================================

/// `s.is_blank()` ≡ `str.is_blank(s)` cho literal/biến, cả `true` lẫn `false`; trả `bool`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    print("".is_blank())
    print("   ".is_blank())
    print("\t\r\n".is_blank())
    print("\u{00A0}\u{3000}".is_blank())
    print("a".is_blank())
    print(" a ".is_blank())
    let s = " \n\t "
    print(s.is_blank())
    let t = "đ"
    print(t.is_blank())
    let b: bool = s.is_blank()
    print(b)
"#,
        "true\ntrue\ntrue\ntrue\nfalse\nfalse\ntrue\nfalse\ntrue\n",
    );
}

// ===========================================================================
// 7 — kiểu trả bool qua biên gán (module + sugar)
// ===========================================================================

/// `str.is_blank` và sugar trả `bool` thật — gán có chú thích `bool` chạy sạch cả hai engine.
#[test]
fn kieu_tra_bool_qua_bien_gan() {
    dung(
        "kieu",
        r#"use std.str

fn main():
    let a: bool = str.is_blank("")
    let b: bool = str.is_blank("x")
    let c: bool = " \t ".is_blank()
    print(a)
    print(b)
    print(c)
"#,
        "true\nfalse\ntrue\n",
    );
}

// ===========================================================================
// 8 — ca âm tĩnh: kiểu đối số / kiểu kết quả
// ===========================================================================

/// Sai kiểu đối số (`int` thay `str`) ⇒ `VS0412`; kết quả `bool` gán `int`/`str` ⇒ `VS0412`.
#[test]
fn sai_kieu_bi_chan() {
    tu_choi(
        "sai-kieu-doi-so",
        "use std.str\n\nfn main():\n    print(str.is_blank(1))\n",
        "VS0412",
    );
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.is_blank(\" \")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-str",
        "fn main():\n    let x: str = \" \".is_blank()\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 9 — ca âm tĩnh: arity (module + sugar)
// ===========================================================================

/// Thiếu/thừa đối số ở lời gọi module và sugar ⇒ `VS0401` (không rơi xuống trap `VS0501`).
#[test]
fn sai_so_doi_bi_chan() {
    for (tag, call) in [
        ("thieu", "str.is_blank()"),
        ("thua", "str.is_blank(\"a\", \"b\")"),
        ("sugar-thua", "\"a\".is_blank(1)"),
        ("sugar-thua-str", "\"a\".is_blank(\"b\")"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}
