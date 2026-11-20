// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (splitn)** — `std.str.splitn(s, sep, n) -> list[str]`.
//!
//! Hợp đồng được khoá ở đây:
//! - Tách **trái → phải**, tối đa `n` phần; **phần cuối** là **phần còn lại nguyên văn** của
//!   `s`, kể cả các dấu phân cách chưa dùng:
//!   `splitn("a,b,c", ",", 2)` ⇒ `["a", "b,c"]`.
//! - `n == 0` ⇒ `[]`; `n == 1` ⇒ `[s]`; `n > số phần` ⇒ toàn bộ phần (như `str.split`).
//! - Dấu phân cách ở **cuối** sinh phần tử rỗng cuối: `splitn("a,b,", ",", 3)` ⇒ `["a", "b", ""]`.
//! - Đơn vị khớp là **chuỗi con UTF-8** (cùng `str.split`/`str.find`), **không bao giờ** cắt
//!   giữa scalar: `splitn("đ😀x", "😀", 2)` ⇒ `["đ", "x"]`.
//! - `sep` rỗng theo **đúng nghĩa Rust `str::splitn`** (bảng đã kiểm bằng probe Rust):
//!   `"abc".splitn(2,"")` ⇒ `["", "abc"]`; `"abc".splitn(1,"")` ⇒ `["abc"]`;
//!   `"".splitn(2,"")` ⇒ `["", ""]`; `"a".splitn(3,"")` ⇒ `["", "a", ""]`;
//!   `"abc".splitn(3,"")` ⇒ `["", "a", "bc"]`; `"đ😀".splitn(3,"")` ⇒ `["", "đ", "😀"]`.
//!   **Không** trap vì `sep` rỗng. Đây là khác biệt **có chủ ý** với `str.split` của VS (khi
//!   `sep` rỗng, `str.split` tách theo scalar và **không** sinh phần rỗng đầu/cuối:
//!   `str.split("abc", "")` ⇒ `["a", "b", "c"]`).
//! - `n < 0` là trap `VS0501` nêu `str.splitn`, exit 101 ở **cả hai** engine.
//! - Chữ ký `(str, str, int) -> list[str]`; sugar R5c `s.splitn(sep, n)` ≡
//!   `str.splitn(s, sep, n)`, không cần `use std.str`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `254c03f` + commit report):
//!
//! ```text
//! $ vis check probe.vis                  # use std.str; str.splitn(...), "...".splitn(...)
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.str` không có hàm `splitn`
//! exit=1
//! $ vis run --no-cache probe.vis
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `splitn`
//! probe.vis:6:11: error VS0403: `str` không có trường `splitn`
//! exit=2
//! $ vis run --no-cache --vm probe.vis
//! probe.vis:5:11: error VS0301: module `std.str` không có hàm `splitn`
//! probe.vis:6:11: error VS0403: `str` không có trường `splitn`
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
    let d = std::env::temp_dir().join(format!("vis-vd270-spn-{tag}-{}-{n}", std::process::id()));
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

/// Lỗi **chạy**: hai engine cùng exit **101** (R9/C1), cùng `VS0501`, stderr nêu hàm.
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
// 1 — cơ bản: tối đa `n` phần, phần cuối giữ nguyên
// ===========================================================================

/// `n = 2/1/3/4`, không có dấu phân cách, `n` lớn — phần cuối chứa cả dấu phân cách còn lại.
#[test]
fn co_ban_toi_da_n_phan() {
    dung(
        "spn-co-ban",
        r#"use std.str

fn main():
    print(str.splitn("a,b,c", ",", 2))
    print(str.splitn("a,b,c", ",", 1))
    print(str.splitn("a,b,c", ",", 3))
    print(str.splitn("a,b,c", ",", 4))
    print(str.splitn("abc", ",", 2))
    print(str.splitn("a", ",", 99))
    print(str.splitn("a::b::c", "::", 2))
"#,
        "[a, b,c]\n[a,b,c]\n[a, b, c]\n[a, b, c]\n[abc]\n[a]\n[a, b::c]\n",
    );
}

// ===========================================================================
// 2 — `n == 0` ⇒ `[]`; `n == 1` ⇒ `[s]`
// ===========================================================================

/// `n == 0` luôn rỗng (kể cả `s` rỗng); `n == 1` trả đúng một phần là toàn bộ `s`.
#[test]
fn n_bang_khong_va_n_bang_mot() {
    dung(
        "spn-n-01",
        r#"use std.str

fn main():
    print(str.splitn("a,b,c", ",", 0))
    print(len(str.splitn("a,b,c", ",", 0)))
    print(str.splitn("", ",", 0))
    print(str.splitn("abc", "", 0))
    print(str.join(str.splitn("a,b,c", ",", 1), "|"))
    print(str.join(str.splitn("", ",", 1), "--"))
    print(len(str.splitn("", ",", 1)))
    print(str.join(str.splitn(",", ",", 1), "|"))
"#,
        "[]\n0\n[]\n[]\na,b,c\n\n1\n,\n",
    );
}

// ===========================================================================
// 3 — dấu phân cách ở cuối: phần tử rỗng cuối
// ===========================================================================

/// `"a,b,"` chia 3 ⇒ `["a", "b", ""]`; `","` chia 2 ⇒ `["", ""]`; `",,"` chia 3 ⇒ ba rỗng.
#[test]
fn dau_phan_cach_cuoi_sinh_phan_rong() {
    dung(
        "spn-trailing",
        r#"use std.str

fn main():
    print(str.splitn("a,b,", ",", 2))
    print(str.splitn("a,b,", ",", 3))
    print(str.join(str.splitn("a,b,", ",", 3), "|"))
    print(str.splitn(",", ",", 2))
    print(str.join(str.splitn(",", ",", 2), "|"))
    print(str.splitn(",,", ",", 3))
    print(str.join(str.splitn(",,", ",", 3), "|"))
    print(str.join(str.splitn("a,,b", ",", 2), "|"))
    print(str.join(str.splitn("a,,b", ",", 3), "|"))
"#,
        "[a, b,]\n[a, b, ]\na|b|\n[, ]\n|\n[, , ]\n||\na|,b\na||b\n",
    );
}

// ===========================================================================
// 4 — `sep` rỗng: bảng Rust `str::splitn("")`
// ===========================================================================

/// Bảng đối chiếu **probe Rust**:
/// `"abc"` × n=2/1/3/9; `""` × n=1/2/6; `"a"` × n=3; `"đ😀"` × n=3.
#[test]
fn sep_rong_theo_bang_rust() {
    dung(
        "spn-empty-sep",
        r#"use std.str

fn main():
    print(str.splitn("abc", "", 2))
    print(str.splitn("abc", "", 1))
    print(str.splitn("abc", "", 3))
    print(str.splitn("abc", "", 9))
    print(str.splitn("", "", 2))
    print(str.splitn("", "", 1))
    print(len(str.splitn("", "", 1)))
    print(len(str.splitn("", "", 2)))
    print(str.splitn("", "", 6))
    print(str.splitn("a", "", 2))
    print(str.splitn("a", "", 3))
    print(str.splitn("a", "", 4))
    print(str.splitn("đ😀", "", 3))
    print(str.splitn("đ😀", "", 4))
"#,
        "[, abc]\n[abc]\n[, a, bc]\n[, a, b, c, ]\n[, ]\n[]\n1\n2\n[, ]\n[, a]\n[, a, ]\n[, a, ]\n[, đ, 😀]\n[, đ, 😀, ]\n",
    );
}

// ===========================================================================
// 5 — Unicode/emoji: khớp theo chuỗi con, không cắt giữa scalar
// ===========================================================================

/// Tiếng Việt + emoji: dấu phân cách nhiều byte vẫn khớp đúng; không sinh UTF-8 hỏng.
#[test]
fn unicode_emoji_khong_cat_scalar() {
    dung(
        "spn-unicode",
        r#"use std.str

fn main():
    print(str.splitn("đ,😀,x", ",", 2))
    print(str.join(str.splitn("đ,😀,x", ",", 2), "|"))
    print(str.splitn("a😀b😀c", "😀", 2))
    print(str.splitn("đ😀x😀", "😀", 3))
    print(str.join(str.splitn("đ😀x😀", "😀", 3), "|"))
    print(str.splitn("👨‍👩‍👧x", "👩", 2))
    print(str.join(str.splitn("👨‍👩‍👧x", "👩", 2), "|"))
    print(str.splitn("đ😀", "", 4))
"#,
        "[đ, 😀,x]\nđ|😀,x\n[a, b😀c]\n[đ, x, ]\nđ|x|\n[👨‍, ‍👧x]\n👨‍|‍👧x\n[, đ, 😀, ]\n",
    );
}

// ===========================================================================
// 6 — sugar phương thức R5c (không cần `use std.str`)
// ===========================================================================

/// `s.splitn(sep, n)` ≡ `str.splitn(s, sep, n)`; kết quả là `list[str]` thật.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "spn-sugar",
        r#"fn main():
    let s: str = "a,b,c"
    print(s.splitn(",", 2))
    print("a,b,c".splitn(",", 1))
    let t: list[str] = s.splitn(",", 3)
    print(t)
    print("a,b,".splitn(",", 3).join("|"))
    print("abc".splitn("", 2))
    print("đ😀x😀".splitn("😀", 2))
"#,
        "[a, b,c]\n[a,b,c]\n[a, b, c]\na|b|\n[, abc]\n[đ, x😀]\n",
    );
}

// ===========================================================================
// 7 — `n < 0` ⇒ trap VS0501 nêu `str.splitn` ở cả hai engine
// ===========================================================================

/// `n` âm là lỗi **chạy** (không phải lỗi kiểu): trap `VS0501`, exit 101, stderr nêu hàm.
#[test]
fn n_am_la_trap_ca_hai_engine() {
    trap_ca_hai(
        "spn-n-am",
        "use std.str\n\nfn main():\n    print(str.splitn(\"a,b\", \",\", -1))\n",
        "str.splitn",
    );
    trap_ca_hai(
        "spn-sugar-n-am",
        "fn main():\n    print(\"a,b\".splitn(\",\", -2))\n",
        "str.splitn",
    );
}

// ===========================================================================
// 8 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.splitn(1, \",\", 2)", "VS0412"),
        ("kieu-2", "str.splitn(\"a\", 1, 2)", "VS0412"),
        ("kieu-3", "str.splitn(\"a\", \",\", \"2\")", "VS0412"),
        ("arity-1", "str.splitn(\"a\", \",\")", "VS0401"),
        ("arity-2", "str.splitn(\"a\")", "VS0401"),
        ("arity-3", "str.splitn()", "VS0401"),
        ("arity-4", "str.splitn(\"a\", \",\", 2, 3)", "VS0401"),
        ("sugar-kieu", "\"a\".splitn(1, 2)", "VS0412"),
        ("sugar-kieu-2", "\"a\".splitn(\",\", \"2\")", "VS0412"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả là `list[str]` thật: gán `list[str]` hợp lệ; gán `str`/`list[int]` bị chặn.
#[test]
fn ket_qua_la_list_str_that() {
    tu_choi(
        "spn-ket-qua-sang-str",
        "use std.str\n\nfn main():\n    let x: str = str.splitn(\"a,b\", \",\", 2)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spn-ket-qua-sang-list-int",
        "use std.str\n\nfn main():\n    let x: list[int] = str.splitn(\"a,b\", \",\", 2)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "spn-sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = \"a,b\".splitn(\",\", 2)\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 9 — ca âm chạy: sugar sai arity ⇒ trap, không panic
// ===========================================================================

/// Sugar R5c chưa kiểm **arity** ở checker (nợ chung của R5c); lời gọi **thừa** đối số vẫn bị
/// **tầng chạy** chặn: trap `VS0501` nêu `str.splitn`, exit 101 ở **cả hai** engine.
#[test]
fn sugar_thua_doi_la_trap_ca_hai_engine() {
    trap_ca_hai(
        "spn-sugar-thua",
        "fn main():\n    print(\"a,b\".splitn(\",\", 2, 3))\n",
        "str.splitn",
    );
}

// ===========================================================================
// 10 — mọi ca dương trong **một** chương trình: interp ↔ VM byte-identical
// ===========================================================================

/// Gộp các ngữ nghĩa chính (kể cả `n = 0`, dấu cuối, `sep` rỗng, Unicode, sugar) để khoá
/// **byte** stdout/stderr/exit giữa hai engine.
#[test]
fn hai_engine_byte_identical_toan_bo() {
    dung(
        "spn-hai-engine",
        r#"use std.str

fn main():
    let s: str = "a,b,c"
    print(str.splitn(s, ",", 2))
    print(s.splitn(",", 2))
    print(str.splitn(s, ",", 0))
    print(str.splitn(s, ",", 1))
    print(str.splitn(s, ",", 9))
    print(str.join(str.splitn("a,b,", ",", 3), "|"))
    print(str.splitn("abc", "", 2))
    print(str.splitn("", "", 2))
    print(str.splitn("a", "", 3))
    print(str.splitn("đ😀x😀", "😀", 2))
"#,
        "[a, b,c]\n[a, b,c]\n[]\n[a,b,c]\n[a, b, c]\na|b|\n[, abc]\n[, ]\n[, a, ]\n[đ, x😀]\n",
    );
}
