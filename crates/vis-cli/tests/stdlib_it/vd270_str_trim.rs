// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (strtrim)** — cắt theo **tập scalar**: `std.str.trim_matches(s, chars)`,
//! `str.trim_start_matches(s, chars)`, `str.trim_end_matches(s, chars)`.
//!
//! Hợp đồng được khoá ở đây:
//! - `chars` là **tập** scalar Unicode: thứ tự và lặp lại **không** quan trọng.
//! - Bỏ **lặp lại** scalar ở đầu/cuối (hoặc cả hai) khi scalar đó thuộc `chars`, tới scalar
//!   đầu tiên (từ phía tương ứng) **không** thuộc tập — như Rust `trim*_matches` + closure.
//! - `chars` rỗng ⇒ `s` **nguyên vẹn**; mọi scalar đều thuộc tập ⇒ `""`.
//! - Đơn vị là **scalar** (`char`), **không** phải byte và **không** phải grapheme: emoji nhiều
//!   scalar (vd. `👨‍👩‍👧` = 5 scalar) chỉ bị cắt từng scalar có mặt trong `chars`.
//! - Kiểu `(str, str) -> str`; sugar R5c `s.trim_matches(...)` ≡ `str.trim_matches(s, ...)`,
//!   không cần `use`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `target/debug/vis` dựng từ cây này @ `4babfb0`):
//!
//! ```text
//! $ vis check probe.vis                    # có use std.str, 3 lời gọi
//! TAP version 13
//! 1..1
//! not ok 1 - /tmp/opencode/vd270-strtrim-red/probe.vis
//! # VS0301 4:11 module `std.str` không có hàm `trim_matches`
//! $ vis run --no-cache probe.vis           ⇒ exit 2
//! probe.vis:4:11: error VS0301: module `std.str` không có hàm `trim_matches`
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `trim_start_matches`
//! probe.vis:6:11: error VS0301: module `std.str` không có hàm `trim_end_matches`
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng ba dòng)
//! $ vis check sugar.vis                    # "--x--".trim_matches("-"), không use
//! not ok 1 - /tmp/opencode/vd270-strtrim-red/sugar.vis
//! # VS0403 2:11 `str` không có trường `trim_matches`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    // **ST-310b**: tiền tố riêng cho module — `vd270_str_title` dùng cùng `vis-vd270-st-…` + tag
    // `sugar-arity-thua` + cùng pid, nên chạy song song có thể xoá nhầm thư mục của nhau.
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-strtrim-{tag}-{}-{n}",
        std::process::id()
    ));
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
// 1 — ASCII: tập ký tự, thứ tự/lặp không quan trọng, dừng ở scalar không thuộc
// ===========================================================================

#[test]
fn ascii_tap_ky_tu() {
    dung(
        "ascii",
        r#"use std.str

fn main():
    print(str.trim_matches("--x--", "-"))
    print(str.trim_start_matches("--x--", "-"))
    print(str.trim_end_matches("--x--", "-"))
    print(str.trim_matches("abbaHELLOabba", "ab"))
    print(str.trim_matches("abbaHELLOabba", "ba"))
    print(str.trim_matches("a-b", "-a"))
    print(str.trim_matches("xyx", "xy"))
"#,
        "x\nx--\n--x\nHELLO\nHELLO\nb\n\n",
    );
}

// ===========================================================================
// 2 — Unicode / emoji: scalar, không phải byte/grapheme
// ===========================================================================

#[test]
fn unicode_va_emoji_theo_scalar() {
    dung(
        "unicode",
        r#"use std.str

fn main():
    print(str.trim_matches("đ😀đxđ😀đ", "đ😀"))
    print(str.trim_start_matches("đ😀đabc", "đ😀"))
    print(str.trim_end_matches("abcđ😀đ", "đ😀"))
    print(str.trim_matches("ếếchàoếế", "ế"))
    print(str.char_count(str.trim_start_matches("👨‍👩‍👧", "👨")))
    print(str.char_count(str.trim_matches("👨‍👩‍👧", "👨👧")))
    print(str.trim_matches("😀a😀", "😀"))
"#,
        "x\nabc\nabc\nchào\n4\n3\na\n",
    );
}

// ===========================================================================
// 3 — `chars` rỗng / toàn bộ scalar thuộc tập / chuỗi rỗng
// ===========================================================================

#[test]
fn chars_rong_va_toan_bo_stripped() {
    dung(
        "rong-toan-bo",
        r#"use std.str

fn main():
    print(str.char_count(str.trim_matches("abc", "")))
    print(str.char_count(str.trim_start_matches("abc", "")))
    print(str.char_count(str.trim_end_matches("abc", "")))
    print(str.trim_matches("abc", ""))
    print(str.trim_matches("aaa", "a"))
    print(str.char_count(str.trim_matches("aaa", "a")))
    print(str.trim_start_matches("aaa", "a"))
    print(str.char_count(str.trim_end_matches("😀😀", "😀")))
    print(str.trim_matches("", "abc"))
    print(str.trim_start_matches("", ""))
"#,
        "3\n3\n3\nabc\n\n0\n\n0\n\n\n",
    );
}

// ===========================================================================
// 4 — chỉ trái / chỉ phải: bất đối xứng
// ===========================================================================

#[test]
fn chi_mot_phia() {
    dung(
        "mot-phia",
        r#"use std.str

fn main():
    print(str.trim_start_matches("  x  ", " "))
    print(str.trim_end_matches("  x  ", " "))
    print(str.trim_matches("  x  ", " "))
    print(str.trim_start_matches("xx", "x"))
    print(str.trim_end_matches("xx", "x"))
"#,
        "x  \n  x\nx\n\n\n",
    );
}

// ===========================================================================
// 5 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "--x--"
    print(s.trim_matches("-"))
    print(s.trim_start_matches("-"))
    print(s.trim_end_matches("-"))
    let t: str = "đ😀đx".trim_matches("đ😀")
    print(t)
    print("  x  ".trim_matches(" "))
    print("ab".trim_matches(""))
"#,
        "x\nx--\n--x\nx\nx\nab\n",
    );
}

// ===========================================================================
// 6 — kiểu kết quả là `str` (module + sugar)
// ===========================================================================

#[test]
fn ket_qua_la_str() {
    dung(
        "kieu-str",
        r#"use std.str

fn main():
    let a: str = str.trim_matches("--x--", "-")
    let b: str = str.trim_start_matches("--x--", "-")
    let c: str = str.trim_end_matches("--x--", "-")
    let d: str = "--y--".trim_matches("-")
    print(a)
    print(b)
    print(c)
    print(d)
"#,
        "x\nx--\n--x\ny\n",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số; kết quả `str` gán sang `int` bị chặn.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.trim_matches(1, \"a\")", "VS0412"),
        ("kieu-2", "str.trim_matches(\"a\", 1)", "VS0412"),
        ("kieu-3", "str.trim_start_matches(\"a\", 1)", "VS0412"),
        ("kieu-4", "str.trim_end_matches(\"a\", 1)", "VS0412"),
        ("arity-1", "str.trim_matches(\"a\")", "VS0401"),
        ("arity-2", "str.trim_matches(\"a\", \"b\", \"c\")", "VS0401"),
        ("arity-3", "str.trim_start_matches()", "VS0401"),
        ("arity-4", "str.trim_end_matches(\"a\")", "VS0401"),
        ("sugar-kieu", "\"a\".trim_matches(1)", "VS0412"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
    tu_choi(
        "ket-qua-sang-int",
        "use std.str\n\nfn main():\n    let x: int = str.trim_matches(\"abc\", \"a\")\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"abc\".trim_matches(\"a\")\n    print(x)\n",
        "VS0412",
    );
}

/// **VD-270 (R5c arity)**: sugar `s.trim_matches(…)` thiếu/thừa đối số bị checker chặn **trước
/// khi chạy** — đúng `VS0401` như gọi module `str.trim_matches` (`cần 1 đối số`), tại span lời
/// gọi. Không còn rơi xuống tầng chạy thành trap `VS0501` (exit 101).
#[test]
fn sugar_sai_arity_bi_chan() {
    for (tag, call) in [
        ("sugar-arity-thieu", "\"a\".trim_matches()"),
        ("sugar-arity-thua", "\"a\".trim_matches(\"a\", \"b\")"),
        ("sugar-arity-start-thieu", "\"a\".trim_start_matches()"),
        (
            "sugar-arity-end-thua",
            "\"a\".trim_end_matches(\"a\", \"b\")",
        ),
    ] {
        let src = format!("fn main():\n    print({call})\n");
        tu_choi(tag, &src, "VS0401");
    }
}
