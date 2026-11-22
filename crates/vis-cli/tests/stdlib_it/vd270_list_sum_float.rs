// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-sum-float)** — `std.list.sum_float(xs) -> float`.
//!
//! Hợp đồng được khoá ở đây:
//! - Tổng **trái → phải** bằng `f64`, **hạt giống `0.0`**; `xs` rỗng ⇒ `0.0` (không trap).
//! - **IEEE 754**: `NaN` đầu vào ⇒ `NaN`; `+inf`/`-inf` lan truyền; `+inf + -inf` ⇒ `NaN`;
//!   tràn hai số hữu hạn lớn ⇒ `inf`; không trap, không bão hoà.
//! - Dấu `0.0`: hạt giống `+0.0` nên `0.0 + -0.0 = +0.0` (IEEE, làm tròn gần nhất) và
//!   **mọi** tổng bằng 0 — kể cả `[-0.0]`, `[-0.0, -0.0]` — đều là `+0.0`; hàm **không**
//!   trả `-0.0` cho đầu vào hữu hạn (khác `mean`/`median` lấy phần tử đầu làm hạt giống).
//! - Độ chính xác: `[0.1, 0.2]` ⇒ `0.30000000000000004` (không format lại `0.3`); thứ tự
//!   cộng trái → phải được khoá bằng `[1.0, -1.0, 1e-16]` ⇒ `1e-16` còn
//!   `[1e-16, 1.0, -1.0]` ⇒ `0`.
//! - Chữ ký `(list[float]) -> float` (`Ty::Float`, **không** `float?`); sai kiểu ⇒ `VS0412`,
//!   sai arity ⇒ `VS0401`; kết quả gán được `float`, không gán được `int`.
//! - Sugar R5c `xs.sum_float()` ≡ `list.sum_float(xs)`, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `9099201`):
//!
//! ```text
//! $ vis check probe.vis                  # probe: print(list.sum_float([1.0, 2.0])), có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.list` không có hàm `sum_float`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.list` không có hàm `sum_float`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng dòng)
//! $ vis check sugar.vis                  # print(xs.sum_float()), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `list[float]` không có trường `sum_float`
//! $ vis run --no-cache sugar.vis         ⇒ exit 2
//! sugar.vis:4:11: error VS0403: `list[float]` không có trường `sum_float`
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
    let d = std::env::temp_dir().join(format!("vis-vd270-lsf-{tag}-{}-{n}", std::process::id()));
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
// 1 — danh sách rỗng ⇒ 0.0
// ===========================================================================

/// `list[float]` rỗng (khai báo tường minh), literal rỗng, và kết quả `take(0)` ⇒ `0`,
/// không trap, không `None`.
#[test]
fn rong_tra_zero() {
    dung(
        "rong",
        r#"use std.list

fn main():
    let r: list[float] = []
    print(list.sum_float(r))
    print([0.0].take(0).sum_float())
    print([].sum_float())
"#,
        "0\n0\n0\n",
    );
}

// ===========================================================================
// 2 — một phần tử
// ===========================================================================

/// Một phần tử ⇒ chính nó (dương/âm/0); `[-0.0]` ⇒ `+0.0` vì hạt giống `+0.0`.
#[test]
fn mot_phan_tu() {
    dung(
        "mot",
        r#"use std.list

fn main():
    print(list.sum_float([1.5]))
    print(list.sum_float([-2.5]))
    print(list.sum_float([0.0]))
    print(list.sum_float([-0.0]))
"#,
        "1.5\n-2.5\n0\n0\n",
    );
}

// ===========================================================================
// 3 — dương / âm / trộn
// ===========================================================================

/// Tổng dương, tổng âm, trộn dấu (kể cả tổng 0), và tổng không nguyên.
#[test]
fn duong_am_va_tron() {
    dung(
        "dau",
        r#"use std.list

fn main():
    print(list.sum_float([1.0, 2.0, 3.0]))
    print(list.sum_float([-1.0, -2.0, -3.0]))
    print(list.sum_float([-2.5, 2.5]))
    print(list.sum_float([10.0, -3.0, 7.0]))
    print(list.sum_float([1.0, 2.0]))
"#,
        "6\n-6\n0\n14\n3\n",
    );
}

// ===========================================================================
// 4 — độ chính xác f64 và thứ tự cộng trái → phải
// ===========================================================================

/// `[0.1, 0.2]` ⇒ `0.30000000000000004` — đúng `0.1f64 + 0.2`, **không** phải `0.3`;
/// `[1.0, -1.0, 1e-16]` ⇒ `1e-16` chứng minh cộng **trái → phải** (đảo thứ tự ⇒ 0).
#[test]
fn do_chinh_xac_va_thu_tu() {
    dung(
        "chinh-xac",
        r#"use std.list

fn main():
    print(list.sum_float([0.1, 0.2]))
    print(list.sum_float([0.1, 0.2, 0.3]))
    print(list.sum_float([1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]))
    print(list.sum_float([1.0, -1.0, 1e-16]))
    print(list.sum_float([1e-16, 1.0, -1.0]))
"#,
        "0.30000000000000004\n0.6000000000000001\n1\n0.0000000000000001\n0\n",
    );
}

// ===========================================================================
// 5 — NaN / ±inf lan truyền, không trap
// ===========================================================================

/// `NaN` ⇒ `NaN`; `±inf` giữ dấu; `+inf + -inf` ⇒ `NaN`; hai số lớn tràn ⇒ `inf`;
/// mọi ca exit 0, stderr rỗng.
#[test]
fn nan_va_inf_lan_truyen() {
    dung(
        "nan-inf",
        r#"use std.list

fn main():
    let nan = 0.0 / 0.0
    let inf = 1.0 / 0.0
    print(list.sum_float([nan, 1.0]))
    print(list.sum_float([1.0, nan]))
    print(list.sum_float([inf, 1.0]))
    print(list.sum_float([-inf, 1.0]))
    print(list.sum_float([inf, -inf]))
    print(list.sum_float([inf, inf]))
    print(list.sum_float([-inf, -inf]))
    print(list.sum_float([1.0e308, 1.0e308]))
"#,
        "NaN\nNaN\ninf\n-inf\nNaN\ninf\n-inf\ninf\n",
    );
}

// ===========================================================================
// 6 — dấu 0.0: hạt giống +0.0 ⇒ mọi tổng 0 là +0
// ===========================================================================

/// Hạt giống `+0.0` và IEEE `+0.0 + -0.0 = +0.0` ⇒ `[-0.0]`/`[-0.0, -0.0]` đều in `0`;
/// phép chia `1.0 / tổng` cho `inf` (không phải `-inf`) chứng minh dấu **dương**.
#[test]
fn zero_co_dau_theo_hat_giong_duong() {
    dung(
        "zero-dau",
        r#"use std.list

fn main():
    print(list.sum_float([-0.0]))
    print(list.sum_float([-0.0, -0.0]))
    print(list.sum_float([0.0, -0.0]))
    print(list.sum_float([-0.0, 0.0]))
    print(list.sum_float([-0.0, 0.0, -0.0]))
    print(1.0 / list.sum_float([-0.0]))
    print(1.0 / list.sum_float([-0.0, -0.0]))
"#,
        "0\n0\n0\n0\n0\ninf\ninf\n",
    );
}

// ===========================================================================
// 7 — `float` thật (không `Option`) + sugar R5c
// ===========================================================================

/// Kết quả gán được `float`; sugar `xs.sum_float()` không cần `use std.list`, kể cả trên
/// biến `list[float]`, literal và literal rỗng.
#[test]
fn float_that_va_sugar_phuong_thuc() {
    dung(
        "sugar",
        r#"fn main():
    let xs = [2.0, 4.0]
    let a: float = xs.sum_float()
    print(a)
    print(xs.sum_float())
    print([1.0].sum_float())
    print([0.1, 0.2].sum_float())
    let r: list[float] = []
    print(r.sum_float())
    print([].sum_float())
"#,
        "6\n6\n1\n0.30000000000000004\n0\n0\n",
    );
}

// ===========================================================================
// 8 — ca âm: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); `sum_float` chỉ nhận `list[float]` — `list[int]`/`list[str]` bị từ chối.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "list.sum_float(1)", "VS0412"),
        ("kieu-list-int", "list.sum_float([1, 2])", "VS0412"),
        ("kieu-list-str", "list.sum_float([\"1.0\"])", "VS0412"),
        ("arity-0", "list.sum_float()", "VS0401"),
        ("arity-2", "list.sum_float([1.0], [2.0])", "VS0401"),
        ("sugar-kieu", "[1, 2].sum_float()", "VS0403"),
        ("sugar-arity", "[1.0, 2.0].sum_float(1)", "VS0401"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kiểu trả là `float` (không `Option`): gán `float` hợp lệ, gán `int` ⇒ `VS0412`.
#[test]
fn kieu_tra_la_float_khong_option() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.list\n\nfn main():\n    let x: int = list.sum_float([1.0, 2.0])\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = [1.0, 2.0].sum_float()\n    print(x)\n",
        "VS0412",
    );
}
