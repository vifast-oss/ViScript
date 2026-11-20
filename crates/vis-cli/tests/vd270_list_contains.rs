// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-contains)** — `std.list.contains(xs, value) -> bool`.
//!
//! Hợp đồng được khoá ở đây:
//! - `true` khi có **ít nhất một** phần tử **bằng** `value` theo **cùng** ngữ nghĩa `==` như
//!   `index_of` (`Value::eq_value`: cấu trúc, list/map/record/enum lồng nhau).
//! - Không có phần tử bằng hoặc `xs` rỗng ⇒ `false` (không trap); khác kiểu phần tử ⇒ `false`
//!   ở tầng chạy, còn checker bắt `VS0412` khi kiểu tĩnh biết trước.
//! - Chữ ký `(list[T], T) -> bool`; `T` buộc từ `xs` nên `value` phải cùng kiểu phần tử.
//! - Sugar R5c `xs.contains(v)` ≡ `list.contains(xs, v)`, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `c75c0da`):
//!
//! ```text
//! $ vis check probe.vis                  # có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.list` không có hàm `contains`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.list` không có hàm `contains`
//! probe.vis:6:11: error VS0301: module `std.list` không có hàm `contains`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng hai dòng)
//! $ vis check sugar.vis                  # xs.contains(2), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `list[int]` không có trường `contains`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-lc-{tag}-{}-{n}", std::process::id()));
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
// 1 — có / không có / rỗng / trùng lặp
// ===========================================================================

/// Phần tử có ⇒ `true`, không có ⇒ `false`; danh sách rỗng ⇒ `false`; trùng lặp không đổi
/// kết quả (chỉ cần **một** phần tử bằng).
#[test]
fn co_khong_co_rong_va_trung_lap() {
    dung(
        "co-khong",
        r#"use std.list

fn main():
    let xs = [10, 20, 30, 20, 10]
    print(list.contains(xs, 10))
    print(list.contains(xs, 20))
    print(list.contains(xs, 30))
    print(list.contains(xs, 40))
    let r: list[int] = []
    print(list.contains(r, 1))
    let moi: list[int] = [7, 7, 7]
    print(list.contains(moi, 7))
    print(list.contains(moi, 8))
    print(list.contains(["a", "b"], "b"))
    print(list.contains(["a", "b"], "z"))
"#,
        "true\ntrue\ntrue\nfalse\nfalse\ntrue\nfalse\ntrue\nfalse\n",
    );
}

/// Kết quả là `bool` thật: gán `bool` và dùng trực tiếp trong `if`.
#[test]
fn ket_qua_la_bool_that() {
    dung(
        "bool-that",
        r#"use std.list

fn main():
    let xs = [5, 6]
    let a: bool = list.contains(xs, 6)
    let b: bool = list.contains(xs, 7)
    print(a)
    print(b)
    if list.contains(xs, 5):
        print("co")
    else:
        print("khong")
"#,
        "true\nfalse\nco\n",
    );
}

// ===========================================================================
// 2 — bằng nhau cấu trúc: list / map / record / enum lồng nhau
// ===========================================================================

/// List lồng nhau: so **cấu trúc** (không cùng tham chiếu); sai thứ tự/độ dài ⇒ `false`.
#[test]
fn list_long_nhau() {
    dung(
        "nested-list",
        r#"use std.list

fn main():
    let xs = [[1, 2], [3, 4], [1, 2]]
    print(list.contains(xs, [1, 2]))
    print(list.contains(xs, [4, 3]))
    print(list.contains(xs, [1, 2, 3]))
    print(list.contains(xs, []))
    print(list.contains([], [1]))
"#,
        "true\nfalse\nfalse\nfalse\nfalse\n",
    );
}

/// Map lồng trong list: so **ánh xạ khoá → giá trị**, không phụ thuộc thứ tự chèn.
#[test]
fn map_long_nhau() {
    dung(
        "nested-map",
        r#"use std.list

fn main():
    let xs: list[map[str, int]] = [{"a": 1}, {"b": 2}, {"a": 1, "c": 3}]
    print(list.contains(xs, {"b": 2}))
    print(list.contains(xs, {"c": 3, "a": 1}))
    print(list.contains(xs, {"a": 2}))
    print(list.contains(xs, {"a": 1, "c": 4}))
    print(list.contains(xs, {}))
"#,
        "true\ntrue\nfalse\nfalse\nfalse\n",
    );
}

/// Bản ghi: so theo **trường** như `==`, không theo danh tính/thứ tự khai báo.
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
    print(list.contains(ps, {x: 3, y: 4}))
    print(list.contains(ps, {x: 4, y: 3}))
    print(list.contains(ps, {x: 1, y: 2}))
    print(list.contains(ps, {x: 9, y: 9}))
"#,
        "true\nfalse\ntrue\nfalse\n",
    );
}

/// Enum: so **biến thể + payload** như `==`; biến thể rỗng và khác payload không khớp.
#[test]
fn enum_lit() {
    dung(
        "enum",
        r#"use std.list

enum E:
    A(x: int)
    B

fn main():
    let xs: list[E] = [E.A(1), E.B, E.A(2)]
    print(list.contains(xs, E.A(1)))
    print(list.contains(xs, E.A(2)))
    print(list.contains(xs, E.B))
    print(list.contains(xs, E.A(3)))
"#,
        "true\ntrue\ntrue\nfalse\n",
    );
}

// ===========================================================================
// 3 — sugar R5c
// ===========================================================================

/// `xs.contains(v)` ≡ `list.contains(xs, v)`; `T` buộc từ `xs` nên sugar chạy trên list
/// `int`, `str`, list lồng nhau — không cần `use std.list`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let xs: list[int] = [10, 20, 30, 20]
    print(xs.contains(20))
    print(xs.contains(99))
    print([1, 2, 1].contains(1))
    print(["a", "b"].contains("b"))
    print([[1], [2], [1]].contains([1]))
    print([[1], [2]].contains([3]))
    let b: bool = xs.contains(30)
    print(b)
"#,
        "true\nfalse\ntrue\ntrue\ntrue\nfalse\ntrue\n",
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
        ("kieu-list", "list.contains(1, 1)", "VS0412"),
        (
            "kieu-value-int-str",
            "list.contains([1, 2], \"x\")",
            "VS0412",
        ),
        ("kieu-value-str-int", "list.contains([\"a\"], 1)", "VS0412"),
        ("arity-0", "list.contains()", "VS0401"),
        ("arity-1", "list.contains([1, 2])", "VS0401"),
        ("arity-3", "list.contains([1, 2], 2, 3)", "VS0401"),
        ("sugar-kieu", "[1, 2].contains(\"x\")", "VS0412"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kết quả `bool` không gán được sang `int` (kể cả qua sugar).
#[test]
fn ket_qua_bool_khong_gan_sang_int() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.list\n\nfn main():\n    let x: int = list.contains([1, 2], 2)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = [1, 2].contains(2)\n    print(x)\n",
        "VS0412",
    );
}

/// Sugar R5c chưa kiểm **arity** ở checker (nợ chung của R5c); lời gọi thiếu đối số vẫn bị
/// **tầng chạy** chặn: trap `VS0501` nêu `list.contains`, exit 101 ở **cả hai** engine.
#[test]
fn sugar_sai_arity_la_trap_ca_hai_engine() {
    let (code, out, err) = hai_engine(
        "sugar-arity",
        "fn main():\n    let xs = [1, 2]\n    print(xs.contains())\n",
    );
    assert_eq!(
        code, 101,
        "sugar-arity: phải trap (exit 101)\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.contains("VS0501"), "sugar-arity: thiếu VS0501: {err}");
    assert!(
        err.contains("list.contains"),
        "sugar-arity: stderr thiếu `list.contains`: {err}"
    );
}
