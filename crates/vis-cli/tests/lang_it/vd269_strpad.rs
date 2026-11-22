// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — `std.str` thực dụng cho định dạng chữ: `pad_left`, `pad_right`, `center`,
//! `trim_start`, `trim_end`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - `width` = số **ký tự Unicode** (scalar, như `s.chars().count()`), **không** phải byte:
//!   `"đ"` và `"😀"` đều là **một** ký tự; `"e" + U+0301` là **hai**.
//! - `width <= 0` hoặc `width <= len(s)` ⇒ trả `s` **nguyên vẹn**.
//! - `fill` phải **khác rỗng** — kiểm **trước** cả khi không cần đệm; rỗng ⇒ trap `VS0501`
//!   (exit 101), hai engine byte-identical, **không** panic.
//! - phần đệm = `fill` **lặp rồi cắt** đúng số scalar cần; `fill` nhiều ký tự (kể cả emoji)
//!   không bị cắt theo byte.
//! - `center` chia đôi phần đệm; scalar **dư nằm bên phải** (`"ab"` rộng 5 ⇒ `-ab--`); mỗi phía
//!   **lặp `fill` từ đầu** (`center("x", 4, "ab")` ⇒ `axab`).
//! - `trim_start`/`trim_end` = `trim` **một phía** theo khoảng trắng Unicode (`char::is_whitespace`,
//!   gồm NBSP U+00A0 và U+3000).
//! - trần phần đệm 10^7 scalar (cùng mức `str.repeat`): `width` khổng lồ ⇒ trap `VS0501`.
//! - `s.pad_left(...)`/`s.trim_start()` là sugar R5c (`s.f(a) ≡ std.str.f(s, a)`), không cần
//!   `use std.str`.
//!
//! RED-BEFORE (HEAD `55756dd`, trước khi thêm): cả **interp** và `--vm --no-cache` đều báo
//! `VS0301: module std.str không có hàm pad_left`, exit 2 (tương tự các tên còn lại).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd269-{tag}-{}-{n}", std::process::id()));
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

/// Lỗi **chạy**: hai engine cùng exit **101**, cùng `VS0501`, stderr nêu `needle`.
fn trap_ca_hai(tag: &str, src: &str, needle: &str) {
    let (code, out, err) = hai_engine(tag, src);
    assert_eq!(
        code, 101,
        "{tag}: lỗi chạy phải exit 101\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.contains("VS0501"), "{tag}: thiếu VS0501: {err}");
    assert!(
        err.contains(needle),
        "{tag}: stderr thiếu `{needle}`: {err}"
    );
}

// ===========================================================================
// 1 — pad_left / pad_right: width ASCII và biên
// ===========================================================================

/// `width <= 0`, `width < len`, `= len`, `> len`; chuỗi rỗng. Chuỗi không đổi khi đã đủ rộng.
#[test]
fn pad_ascii_va_bien_width() {
    dung(
        "pad-bien",
        r#"use std.str

fn main():
    print("[" + str.pad_left("a", 0, "-") + "]")
    print("[" + str.pad_left("a", -3, "-") + "]")
    print("[" + str.pad_left("abc", 3, "-") + "]")
    print("[" + str.pad_left("abc", 2, "-") + "]")
    print("[" + str.pad_left("abc", 4, "-") + "]")
    print("[" + str.pad_right("a", 0, "-") + "]")
    print("[" + str.pad_right("a", -1, "-") + "]")
    print("[" + str.pad_right("abc", 3, "-") + "]")
    print("[" + str.pad_right("abc", 5, "-") + "]")
    print("[" + str.center("ab", 0, "-") + "]")
    print("[" + str.center("ab", -2, "-") + "]")
    print("[" + str.center("ab", 2, "-") + "]")
    print("[" + str.center("ab", 3, "-") + "]")
    print("[" + str.pad_left("", 2, "-") + "]")
    print("[" + str.pad_right("", 2, "-") + "]")
    print("[" + str.center("", 3, "-") + "]")
"#,
        "[a]\n[a]\n[abc]\n[abc]\n[-abc]\n[a]\n[a]\n[abc]\n[abc--]\n[ab]\n[ab]\n[ab]\n[ab-]\n[--]\n[--]\n[---]\n",
    );
}

// ===========================================================================
// 2 — Unicode: scalar không phải byte; fill lặp rồi cắt
// ===========================================================================

/// `width` đếm **scalar**: `đ` (2 byte), `😀` (4 byte) đều là 1; `e` + U+0301 là 2. Fill nhiều
/// scalar (`xy`, `😀x`) được lặp rồi **cắt đúng scalar**, không cắt byte.
#[test]
fn pad_unicode_emoji_va_fill_nhieu_ky_tu() {
    dung(
        "pad-unicode",
        r#"use std.str

fn main():
    print("[" + str.pad_left("đ", 3, "-") + "]")
    print("[" + str.pad_right("đ", 4, "·") + "]")
    print("[" + str.pad_left("a", 5, "xy") + "]")
    print("[" + str.center("a", 4, "xy") + "]")
    print("[" + str.center("a", 5, "xy") + "]")
    print("[" + str.pad_left("a", 4, "😀x") + "]")
    print("[" + str.pad_right("😀", 3, "😀") + "]")
    print("[" + str.pad_left("e\u{301}", 3, "-") + "]")
    print(str.pad_left("đ", 4, "-").len())
    print(str.pad_right("😀", 3, "-").len())
    print(str.center("đ", 5, "·").len())
"#,
        "[--đ]\n[đ···]\n[xyxya]\n[xaxy]\n[xyaxy]\n[😀x😀a]\n[😀😀😀]\n[-e\u{301}]\n4\n3\n5\n",
    );
}

// ===========================================================================
// 3 — center: chẵn/lẻ, scalar dư bên phải
// ===========================================================================

/// Tổng đệm lẻ ⇒ bên **trái** `floor`, bên **phải** `ceil`: `ab` rộng 5 ⇒ `-ab--`; fill nhiều
/// scalar cũng chia theo **scalar**; chuỗi rỗng vẫn đệm đủ.
#[test]
fn center_chan_le_du_ben_phai() {
    dung(
        "center",
        r#"use std.str

fn main():
    print("[" + str.center("ab", 5, "-") + "]")
    print("[" + str.center("ab", 6, "-") + "]")
    print("[" + str.center("ab", 7, "-") + "]")
    print("[" + str.center("x", 4, "ab") + "]")
    print("[" + str.center("x", 5, "abc") + "]")
    print("[" + str.center("", 2, "xy") + "]")
    print("[" + str.center("abc", 10, "·") + "]")
"#,
        "[-ab--]\n[--ab--]\n[--ab---]\n[axab]\n[abxab]\n[xx]\n[···abc····]\n",
    );
}

// ===========================================================================
// 4 — trim_start / trim_end: khoảng trắng Unicode một phía
// ===========================================================================

/// Chỉ cắt **một** phía; `\t\n\r`, NBSP U+00A0, U+3000; chuỗi rỗng/toàn khoảng trắng; `trim`
/// cũ không đổi.
#[test]
fn trim_start_end_mot_phia() {
    dung(
        "trim-mot-phia",
        r#"use std.str

fn main():
    print("[" + str.trim_start("  x  ") + "]")
    print("[" + str.trim_end("  x  ") + "]")
    print("[" + str.trim_start("\t\n x") + "]")
    print("[" + str.trim_end("x \r\n\t") + "]")
    print("[" + str.trim_start("\u{00a0}\u{3000}x") + "]")
    print("[" + str.trim_end("x\u{00a0}\u{3000}") + "]")
    print("[" + str.trim_start("") + "]")
    print("[" + str.trim_end("   ") + "]")
    print("[" + str.trim_start("x y") + "]")
    print("[" + str.trim_end("x y") + "]")
    print("[" + str.trim("  x  ") + "]")
"#,
        "[x  ]\n[  x]\n[x]\n[x]\n[x]\n[x]\n[]\n[]\n[x y]\n[x y]\n[x]\n",
    );
}

// ===========================================================================
// 5 — fill rỗng / width khổng lồ: trap VS0501, hai engine byte-identical
// ===========================================================================

/// `fill == ""` ⇒ trap `VS0501`, exit 101 cho **cả** ba hàm; kiểm **trước** cả khi width không
/// cần đệm (`"abc"` rộng 1 vẫn trap, không lặng lẽ trả nguyên chuỗi).
#[test]
fn fill_rong_la_trap() {
    trap_ca_hai(
        "fill-rong-trai",
        "use std.str\n\nfn main():\n    print(str.pad_left(\"a\", 5, \"\"))\n",
        "fill",
    );
    trap_ca_hai(
        "fill-rong-phai",
        "use std.str\n\nfn main():\n    print(str.pad_right(\"abc\", 1, \"\"))\n",
        "fill",
    );
    trap_ca_hai(
        "fill-rong-giua",
        "use std.str\n\nfn main():\n    print(str.center(\"a\", 5, \"\"))\n",
        "fill",
    );
}

/// Trần phần đệm 10^7 scalar: `width` khổng lồ ⇒ trap `VS0501` với thông điệp "quá lớn", không
/// OOM/panic; hai engine cùng kết quả.
#[test]
fn width_khong_lo_la_trap() {
    trap_ca_hai(
        "width-khong-lo",
        "use std.str\n\nfn main():\n    print(str.pad_left(\"a\", 100000000, \"-\"))\n",
        "quá lớn",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.pad_left(...)`/`s.trim_start()` ≡ `std.str.*(s, ...)`; dùng được khi **không** import module.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    print("[" + "a".pad_left(3, "-") + "]")
    print("[" + "a".pad_right(3, "-") + "]")
    print("[" + "ab".center(5, "-") + "]")
    print("[" + "  x  ".trim_start() + "]")
    print("[" + "  x  ".trim_end() + "]")
    print("[" + "đ".pad_left(3, "·") + "]")
"#,
        "[--a]\n[a--]\n[-ab--]\n[x  ]\n[  x]\n[··đ]\n",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả hai
/// engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.pad_left(1, 5, \"-\")", "VS0412"),
        ("kieu-2", "str.pad_left(\"a\", \"-\", \"-\")", "VS0412"),
        ("kieu-3", "str.center(\"a\", 5, 1)", "VS0412"),
        ("arity-1", "str.pad_left(\"a\", 5)", "VS0401"),
        ("arity-2", "str.pad_left(\"a\", 5, \"-\", 1)", "VS0401"),
        ("trim-kieu", "str.trim_start(1)", "VS0412"),
        ("trim-arity", "str.trim_end()", "VS0401"),
        ("sugar-kieu", "\"a\".pad_right(5, 1)", "VS0412"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
