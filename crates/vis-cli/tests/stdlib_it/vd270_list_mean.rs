// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-mean)** — `std.list.mean(xs) -> float?`.
//!
//! Hợp đồng được khoá ở đây:
//! - `xs` rỗng ⇒ `None` (không chia 0, không trap).
//! - Ngược lại: cộng **trái → phải** bằng `f64` (phần tử đầu là hạt giống, cùng quy ước
//!   phương thức lõi `xs.sum()`) rồi chia `len as f64` ⇒ `Some(mean)`.
//! - **IEEE 754**: `NaN` đầu vào ⇒ `Some(NaN)`; `+inf`/`-inf` lan truyền; `+inf + -inf` ⇒
//!   `Some(NaN)`; không trap, không bão hoà.
//! - Dấu `0.0`: `[-0.0]` ⇒ `Some(-0)`; `[0.0, -0.0]` ⇒ `Some(0)` (số học `f64` thường).
//! - Độ chính xác: `[0.1, 0.2]` ⇒ `Some(0.15000000000000002)` — đúng `(0.1 + 0.2) / 2` trong
//!   `f64`, không dùng tổng chính xác/format lại thành `0.15`.
//! - Chữ ký `(list[float]) -> float?` (`Ty::OptFloat` → `Option[float]`); sai kiểu ⇒ `VS0412`,
//!   sai arity ⇒ `VS0401`; kết quả `float?` **không** gán được `float`.
//! - Sugar R5c `xs.mean()` ≡ `list.mean(xs)`, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `146cf32`):
//!
//! ```text
//! $ vis check probe.vis                  # probe: print(list.mean(xs)), có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 6:11 module `std.list` không có hàm `mean`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:6:11: error VS0301: module `std.list` không có hàm `mean`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng dòng)
//! $ vis check sugar.vis                  # print(xs.mean()), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `list[float]` không có trường `mean`
//! $ vis run --no-cache sugar.vis         ⇒ exit 2
//! sugar.vis:4:11: error VS0403: `list[float]` không có trường `mean`
//! $ vis run --no-cache --vm sugar.vis    ⇒ exit 2 (cùng dòng)
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-lm-{tag}-{}-{n}", std::process::id()));
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
// 1 — danh sách rỗng ⇒ None
// ===========================================================================

/// `list[float]` rỗng (khai báo tường minh) ⇒ `None`, không chia 0, không trap.
#[test]
fn rong_tra_none() {
    dung(
        "rong",
        r#"use std.list

fn main():
    let r: list[float] = []
    print(list.mean(r))
    print([0.0].take(0).mean())
"#,
        "None\nNone\n",
    );
}

// ===========================================================================
// 2 — một phần tử
// ===========================================================================

/// Một phần tử ⇒ chính nó (dương/âm/0), không làm tròn.
#[test]
fn mot_phan_tu() {
    dung(
        "mot",
        r#"use std.list

fn main():
    print(list.mean([1.5]))
    print(list.mean([-2.5]))
    print(list.mean([0.0]))
"#,
        "Some(1.5)\nSome(-2.5)\nSome(0)\n",
    );
}

// ===========================================================================
// 3 — dương / âm / trộn
// ===========================================================================

/// Tổng dương, tổng âm, trộn dấu (kể cả trung bình 0), và kết quả không nguyên.
#[test]
fn duong_am_va_tron() {
    dung(
        "dau",
        r#"use std.list

fn main():
    print(list.mean([1.0, 2.0, 3.0]))
    print(list.mean([-1.0, -2.0, -3.0]))
    print(list.mean([-2.5, 2.5]))
    print(list.mean([10.0, -3.0, 7.0]))
    print(list.mean([1.0, 2.0]))
"#,
        "Some(2)\nSome(-2)\nSome(0)\nSome(4.666666666666667)\nSome(1.5)\n",
    );
}

// ===========================================================================
// 4 — độ chính xác f64 (cộng trái → phải rồi chia)
// ===========================================================================

/// `[0.1, 0.2]` ⇒ `Some(0.15000000000000002)` — đúng `(0.1f64 + 0.2f64) / 2`, **không** phải
/// `Some(0.15)`; chuỗi ba phần tử cũng khoá thứ tự cộng trái → phải.
#[test]
fn do_chinh_xac_f64() {
    dung(
        "chinh-xac",
        r#"use std.list

fn main():
    print(list.mean([0.1, 0.2]))
    print(list.mean([0.1, 0.2, 0.3]))
    print(list.mean([1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]))
"#,
        "Some(0.15000000000000002)\nSome(0.20000000000000004)\nSome(0.3333333333333333)\n",
    );
}

// ===========================================================================
// 5 — NaN / ±inf lan truyền, không trap
// ===========================================================================

/// `NaN` ⇒ `NaN`; `±inf` giữ dấu; `+inf + -inf` ⇒ `NaN`; mọi ca exit 0, stderr rỗng.
#[test]
fn nan_va_inf_lan_truyen() {
    dung(
        "nan-inf",
        r#"use std.list

fn main():
    let nan = 0.0 / 0.0
    let inf = 1.0 / 0.0
    print(list.mean([nan, 1.0]))
    print(list.mean([1.0, nan]))
    print(list.mean([inf, 1.0]))
    print(list.mean([-inf, 1.0]))
    print(list.mean([inf, -inf]))
    print(list.mean([nan, inf]))
"#,
        "Some(NaN)\nSome(NaN)\nSome(inf)\nSome(-inf)\nSome(NaN)\nSome(NaN)\n",
    );
}

// ===========================================================================
// 6 — dấu 0.0 theo số học f64
// ===========================================================================

/// `[-0.0]` ⇒ `Some(-0)` (hạt giống là phần tử đầu); `[0.0, -0.0]` ⇒ `Some(0)` vì
/// `+0.0 + -0.0 = +0.0`.
#[test]
fn zero_co_dau() {
    dung(
        "zero-dau",
        r#"use std.list

fn main():
    print(list.mean([-0.0]))
    print(list.mean([0.0, -0.0]))
    print(list.mean([-0.0, -0.0]))
"#,
        "Some(-0)\nSome(0)\nSome(-0)\n",
    );
}

// ===========================================================================
// 7 — `float?` là `Option[float]` thật; sugar R5c
// ===========================================================================

/// Kết quả gán được `float?`, `match Some/None` chạy; sugar `xs.mean()` không cần
/// `use std.list`, kể cả trên biến `list[float]` và literal.
#[test]
fn option_that_va_sugar_phuong_thuc() {
    dung(
        "sugar",
        r#"fn main():
    let xs = [2.0, 4.0]
    let a: float? = xs.mean()
    print(a)
    print(xs.mean())
    print([1.0].mean())
    print([0.1, 0.2].mean())
    print([].mean())
    match xs.mean():
        Some(m): print(m)
        None: print("none")
    let r: list[float] = []
    match r.mean():
        Some(_): print("some")
        None: print("none")
"#,
        "Some(3)\nSome(3)\nSome(1)\nSome(0.15000000000000002)\nNone\n3\nnone\n",
    );
}

// ===========================================================================
// 8 — ca âm: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); `mean` chỉ nhận `list[float]` — `list[int]`/`list[str]` bị từ chối.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "list.mean(1)", "VS0412"),
        ("kieu-list-int", "list.mean([1, 2])", "VS0412"),
        ("kieu-list-str", "list.mean([\"1.0\"])", "VS0412"),
        ("arity-0", "list.mean()", "VS0401"),
        ("arity-2", "list.mean([1.0], [2.0])", "VS0401"),
        ("sugar-kieu", "[1, 2].mean()", "VS0403"),
        ("sugar-arity", "[1.0, 2.0].mean(1)", "VS0401"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// `float?` không gán được sang `float` (module + sugar) — kiểu trả là `Option[float]` thật.
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    tu_choi(
        "ket-qua-sang-float",
        "use std.list\n\nfn main():\n    let x: float = list.mean([1.0, 2.0])\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-float",
        "fn main():\n    let x: float = [1.0, 2.0].mean()\n    print(x)\n",
        "VS0412",
    );
}
