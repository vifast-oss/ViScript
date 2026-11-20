// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (indexof)** — `std.list.index_of(xs, value) -> int?` và
//! `std.list.last_index_of(xs, value) -> int?`.
//!
//! Hợp đồng được khoá ở đây:
//! - Vị trí **0-based** của phần tử **đầu tiên** / **cuối cùng** bằng `value` theo **cùng**
//!   ngữ nghĩa `==` (`Value::eq_value`: cấu trúc, list/map/record/enum lồng nhau).
//! - Không có phần tử bằng ⇒ `None`; danh sách rỗng ⇒ `None` (không trap).
//! - Kiểu trả **`int?`** (`Ty::OptInt` → `Option[int]`), chữ ký `(list[T], T) -> int?`; `T`
//!   buộc từ `xs` nên `value` phải cùng kiểu phần tử (sai ⇒ `VS0412`).
//! - Sugar R5c `xs.index_of(v)` / `xs.last_index_of(v)` ≡ `list.*(xs, v)`, không cần
//!   `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `c40afb4`):
//!
//! ```text
//! $ vis check probe.vis                  # có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.list` không có hàm `index_of`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.list` không có hàm `index_of`
//! probe.vis:6:11: error VS0301: module `std.list` không có hàm `index_of`
//! probe.vis:7:11: error VS0301: module `std.list` không có hàm `last_index_of`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng ba dòng)
//! $ vis check sugar.vis                  # xs.index_of(20), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `list[int]` không có trường `index_of`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-io-{tag}-{}-{n}", std::process::id()));
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
// 1 — vị trí đầu / cuối, có phần tử trùng
// ===========================================================================

/// `index_of` = lần xuất hiện **đầu**, `last_index_of` = **cuối**; phần tử trùng lặp không bị
/// gộp; vị trí **0-based**.
#[test]
fn dau_cuoi_va_trung_lap() {
    dung(
        "dau-cuoi",
        r#"use std.list

fn main():
    let xs = [10, 20, 30, 20, 10]
    print(list.index_of(xs, 10))
    print(list.index_of(xs, 20))
    print(list.index_of(xs, 30))
    print(list.last_index_of(xs, 10))
    print(list.last_index_of(xs, 20))
    print(list.last_index_of(xs, 30))
"#,
        "Some(0)\nSome(1)\nSome(2)\nSome(4)\nSome(3)\nSome(2)\n",
    );
}

/// Không có phần tử bằng ⇒ `None`; danh sách rỗng ⇒ `None`; kiểu phần tử `str` cũng chạy.
#[test]
fn khong_co_va_danh_sach_rong() {
    dung(
        "not-found",
        r#"use std.list

fn main():
    let xs: list[int] = [1, 2, 3]
    let r: list[int] = []
    print(list.index_of(xs, 9))
    print(list.last_index_of(xs, 9))
    print(list.index_of(r, 1))
    print(list.last_index_of(r, 1))
    print(list.index_of(["a", "b"], "b"))
    print(list.last_index_of(["a", "b"], "z"))
"#,
        "None\nNone\nNone\nNone\nSome(1)\nNone\n",
    );
}

// ===========================================================================
// 2 — bằng nhau cấu trúc: list lồng nhau và bản ghi
// ===========================================================================

/// List lồng nhau: `==` so **cấu trúc** nên `[1, 2]` khớp đúng nội dung, không cùng tham
/// chiếu; list rỗng không khớp phần tử khác rỗng.
#[test]
fn list_long_nhau() {
    dung(
        "nested",
        r#"use std.list

fn main():
    let xs = [[1, 2], [3, 4], [1, 2]]
    print(list.index_of(xs, [1, 2]))
    print(list.last_index_of(xs, [1, 2]))
    print(list.index_of(xs, [4, 3]))
    print(list.last_index_of(xs, []))
"#,
        "Some(0)\nSome(2)\nNone\nNone\n",
    );
}

/// Bản ghi: so theo **trường** như `==`, không theo danh tính; literal cùng nội dung khớp.
#[test]
fn ban_ghi() {
    dung(
        "record",
        r#"use std.list

type P:
    x: int
    y: int

fn main():
    let ps: list[P] = [{x: 1, y: 2}, {x: 3, y: 4}, {x: 1, y: 2}]
    print(list.index_of(ps, {x: 3, y: 4}))
    print(list.last_index_of(ps, {x: 1, y: 2}))
    print(list.index_of(ps, {x: 9, y: 9}))
"#,
        "Some(1)\nSome(2)\nNone\n",
    );
}

// ===========================================================================
// 3 — `int?` là `Option[int]` thật; sugar R5c
// ===========================================================================

/// Kết quả gán được `int?`, `match Some/None` chạy được; gán sang `int` bị chặn (ca âm dưới).
#[test]
fn ket_qua_la_option_int_that() {
    dung(
        "option-that",
        r#"use std.list

fn main():
    let xs = [5, 6]
    let a: int? = list.index_of(xs, 6)
    let b: int? = list.last_index_of(xs, 6)
    print(a)
    print(b)
    match list.index_of(xs, 5):
        Some(i): print(i)
        None: print("none")
    match list.index_of(xs, 7):
        Some(_): print("some")
        None: print("none")
"#,
        "Some(1)\nSome(1)\n0\nnone\n",
    );
}

/// `xs.index_of(v)`/`xs.last_index_of(v)` ≡ `list.*(xs, v)`; `T` buộc từ `xs` nên sugar chạy
/// cả trên list lồng nhau và list `str`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let xs: list[int] = [10, 20, 30, 20]
    print(xs.index_of(20))
    print(xs.last_index_of(20))
    print(xs.index_of(99))
    print([1, 2, 1].last_index_of(1))
    print(["a", "b"].index_of("b"))
    let o: int? = xs.index_of(30)
    print(o)
    print([[1], [2], [1]].last_index_of([1]))
"#,
        "Some(1)\nSome(3)\nNone\nSome(2)\nSome(1)\nSome(2)\nSome(2)\n",
    );
}

// ===========================================================================
// 4 — ca âm: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); `value` phải cùng kiểu phần tử (`T` buộc từ `xs`), sugar cũng kiểm kiểu.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-list", "list.index_of(1, 1)", "VS0412"),
        ("kieu-value", "list.index_of([1, 2], \"x\")", "VS0412"),
        (
            "kieu-last-value",
            "list.last_index_of([1, 2], \"x\")",
            "VS0412",
        ),
        ("arity-0", "list.index_of()", "VS0401"),
        ("arity-1", "list.index_of([1, 2])", "VS0401"),
        ("arity-3", "list.index_of([1, 2], 2, 3)", "VS0401"),
        ("arity-last-1", "list.last_index_of([1, 2])", "VS0401"),
        ("sugar-kieu", "[1, 2].index_of(\"x\")", "VS0412"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// `int?` không gán được sang `int` (kể cả qua sugar); `count`-kiểu nhầm cũng bị chặn.
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.list\n\nfn main():\n    let x: int = list.index_of([1, 2], 2)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = [1, 2].index_of(2)\n    print(x)\n",
        "VS0412",
    );
}

/// Sugar R5c chưa kiểm **arity** ở checker (nợ chung của R5c); lời gọi thiếu đối số vẫn bị
/// **tầng chạy** chặn: trap `VS0501` nêu `list.index_of`, exit 101 ở **cả hai** engine.
#[test]
fn sugar_sai_arity_la_trap_ca_hai_engine() {
    let (code, out, err) = hai_engine(
        "sugar-arity",
        "fn main():\n    let xs = [1, 2]\n    print(xs.index_of())\n",
    );
    assert_eq!(
        code, 101,
        "sugar-arity: phải trap (exit 101)\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.contains("VS0501"), "sugar-arity: thiếu VS0501: {err}");
    assert!(
        err.contains("list.index_of"),
        "sugar-arity: stderr thiếu `list.index_of`: {err}"
    );
}
