// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-median)** — `std.list.median(xs) -> float?`.
//!
//! Hợp đồng được khoá ở đây:
//! - `xs` rỗng ⇒ `None` (không có phần tử giữa, không trap).
//! - Sao chép **snapshot** rồi sắp **tăng**; danh sách gốc **không** bị đổi.
//! - Số phần tử **lẻ** ⇒ phần tử giữa; **chẵn** ⇒ trung bình cộng `f64` của hai phần tử giữa,
//!   đúng công thức `(a + b) / 2.0`.
//! - `NaN` ở **bất kỳ** vị trí ⇒ `None` (tất định, không trap); `±inf` sắp bình thường
//!   (`-inf` < hữu hạn < `+inf`).
//! - Chữ ký `(list[float]) -> float?` (`Ty::OptFloat` → `Option[float]`); sai kiểu ⇒ `VS0412`,
//!   sai arity ⇒ `VS0401`; kết quả `float?` **không** gán được `float`.
//! - Sugar R5c `xs.median()` ≡ `list.median(xs)`, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `0eb6b32`):
//!
//! ```text
//! $ vis check probe.vis                  # probe: print(list.median(xs)), có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 6:11 module `std.list` không có hàm `median`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:6:11: error VS0301: module `std.list` không có hàm `median`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng dòng)
//! $ vis check sugar.vis                  # print(xs.median()), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `list[float]` không có trường `median`
//! $ vis run --no-cache sugar.vis         ⇒ exit 2
//! sugar.vis:4:11: error VS0403: `list[float]` không có trường `median`
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
    let d = std::env::temp_dir().join(format!("vis-vd270-lmd-{tag}-{}-{n}", std::process::id()));
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

/// `list[float]` rỗng (khai báo tường minh / literal rỗng qua sugar) ⇒ `None`, không trap.
#[test]
fn rong_tra_none() {
    dung(
        "rong",
        r#"use std.list

fn main():
    let r: list[float] = []
    print(list.median(r))
    print([0.0].take(0).median())
    print([].median())
"#,
        "None\nNone\nNone\n",
    );
}

// ===========================================================================
// 2 — số lẻ ⇒ phần tử giữa
// ===========================================================================

/// Một phần tử ⇒ chính nó; ba/năm phần tử ⇒ phần tử giữa sau khi sắp; trùng lặp giữ đúng vị trí.
#[test]
fn le_lay_phan_tu_giua() {
    dung(
        "le",
        r#"use std.list

fn main():
    print(list.median([1.5]))
    print(list.median([3.0, 1.0, 2.0]))
    print(list.median([5.0, 9.0, 1.0, 7.0, 3.0]))
    print(list.median([2.0, 1.0, 1.0]))
    print(list.median([1.0, 2.0, 2.0]))
"#,
        "Some(1.5)\nSome(2)\nSome(5)\nSome(1)\nSome(2)\n",
    );
}

// ===========================================================================
// 3 — số chẵn ⇒ trung bình cộng hai phần tử giữa
// ===========================================================================

/// Hai/bốn/tám phần tử (kể cả không sắp) ⇒ `(a + b) / 2.0` của hai phần tử giữa.
#[test]
fn chan_lay_trung_binh_hai_phan_tu_giua() {
    dung(
        "chan",
        r#"use std.list

fn main():
    print(list.median([1.0, 2.0]))
    print(list.median([1.0, 2.0, 3.0, 4.0]))
    print(list.median([4.0, 1.0, 3.0, 2.0]))
    print(list.median([8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0]))
    print(list.median([2.0, 2.0, 2.0, 2.0]))
"#,
        "Some(1.5)\nSome(2.5)\nSome(2.5)\nSome(4.5)\nSome(2)\n",
    );
}

// ===========================================================================
// 4 — đầu vào không sắp xếp; snapshot không đổi danh sách gốc
// ===========================================================================

/// Sắp trên **bản sao**: kết quả đúng và `xs` in ra vẫn **nguyên thứ tự** ban đầu.
#[test]
fn khong_sap_va_khong_doi_danh_sach_goc() {
    dung(
        "khong-sap",
        r#"use std.list

fn main():
    let xs: list[float] = [10.0, -2.5, 3.5, 0.0]
    print(xs)
    print(list.median(xs))
    print(xs)
    print(xs.median())
    print(xs)
"#,
        "[10, -2.5, 3.5, 0]\nSome(1.75)\n[10, -2.5, 3.5, 0]\nSome(1.75)\n[10, -2.5, 3.5, 0]\n",
    );
}

// ===========================================================================
// 5 — số âm
// ===========================================================================

/// Số âm thuần và trộn dấu; sắp tăng trước khi lấy giữa.
#[test]
fn so_am() {
    dung(
        "am",
        r#"use std.list

fn main():
    print(list.median([-5.0, -1.0, -3.0]))
    print(list.median([-4.0, -3.0, -2.0, -1.0]))
    print(list.median([-10.0, 2.0, -4.0]))
    print(list.median([-1.0, 1.0]))
"#,
        "Some(-3)\nSome(-2.5)\nSome(-4)\nSome(0)\n",
    );
}

// ===========================================================================
// 6 — NaN ⇒ None (tất định), không trap
// ===========================================================================

/// `NaN` ở đầu/giữa/cuối, một mình, hoặc xen trong danh sách chẵn ⇒ `None`; exit 0, stderr rỗng.
#[test]
fn nan_tra_none() {
    dung(
        "nan",
        r#"use std.list

fn main():
    let nan = 0.0 / 0.0
    print(list.median([nan]))
    print(list.median([nan, 1.0]))
    print(list.median([1.0, nan, 2.0]))
    print(list.median([1.0, 2.0, 3.0, nan]))
    print(list.median([1.0, nan, 3.0, 4.0]))
"#,
        "None\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 7 — ±inf sắp bình thường; không trap
// ===========================================================================

/// `±inf` sắp theo `-inf < hữu hạn < +inf`; cặp giữa `[-inf, +inf]` cho `(a+b)/2.0` = `NaN`
/// (số học `f64`, **không** phải NaN đầu vào); `inf + inf` ⇒ `inf`.
#[test]
fn inf_sap_binh_thuong() {
    dung(
        "inf",
        r#"use std.list

fn main():
    let inf = 1.0 / 0.0
    print(list.median([inf, 1.0, 2.0]))
    print(list.median([1.0, inf, -inf]))
    print(list.median([-inf, 1.0, 2.0]))
    print(list.median([inf, inf, 1.0]))
    print(list.median([-inf, -inf, 1.0]))
    print(list.median([1.0, inf]))
    print(list.median([-inf, 1.0]))
    print(list.median([-inf, inf]))
    print(list.median([inf, inf]))
    print(list.median([-inf, -inf]))
"#,
        "Some(2)\nSome(1)\nSome(1)\nSome(inf)\nSome(-inf)\nSome(inf)\nSome(-inf)\nSome(NaN)\nSome(inf)\nSome(-inf)\n",
    );
}

// ===========================================================================
// 8 — độ chính xác f64 của phép lấy giữa
// ===========================================================================

/// Số lẻ trả **đúng** `0.2` đã lưu; số chẵn dùng `(a + b) / 2.0` nên `[0.1, 0.2]` ⇒
/// `Some(0.15000000000000002)` (không format lại `0.15`). Hai số cực lớn cùng dấu tràn thành
/// `inf` theo đúng số học `(a + b) / 2.0` — **không trap**, đã ghi trong hợp đồng.
#[test]
fn do_chinh_xac_f64() {
    dung(
        "chinh-xac",
        r#"use std.list

fn main():
    print(list.median([0.1, 0.2]))
    print(list.median([0.3, 0.1, 0.2]))
    print(list.median([0.4, 0.1, 0.3, 0.2]))
    print(list.median([1.0 / 3.0, 1.0 / 3.0]))
    print(list.median([1.0e308, 1.0e308]))
"#,
        "Some(0.15000000000000002)\nSome(0.2)\nSome(0.25)\nSome(0.3333333333333333)\nSome(inf)\n",
    );
}

// ===========================================================================
// 9 — `float?` là `Option[float]` thật; sugar R5c
// ===========================================================================

/// Kết quả gán được `float?`, `match Some/None` chạy; sugar `xs.median()` không cần
/// `use std.list`, kể cả trên biến `list[float]` và literal.
#[test]
fn option_that_va_sugar_phuong_thuc() {
    dung(
        "sugar",
        r#"fn main():
    let xs = [4.0, 1.0, 3.0, 2.0]
    let a: float? = xs.median()
    print(a)
    print(xs.median())
    print([1.0].median())
    print([0.1, 0.2].median())
    print([].median())
    match xs.median():
        Some(m): print(m)
        None: print("none")
    let r: list[float] = []
    match r.median():
        Some(_): print("some")
        None: print("none")
"#,
        "Some(2.5)\nSome(2.5)\nSome(1)\nSome(0.15000000000000002)\nNone\n2.5\nnone\n",
    );
}

// ===========================================================================
// 10 — ca âm: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); `median` chỉ nhận `list[float]` — `list[int]`/`list[str]` bị từ chối.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-int", "list.median(1)", "VS0412"),
        ("kieu-list-int", "list.median([1, 2])", "VS0412"),
        ("kieu-list-str", "list.median([\"1.0\"])", "VS0412"),
        ("arity-0", "list.median()", "VS0401"),
        ("arity-2", "list.median([1.0], [2.0])", "VS0401"),
        ("sugar-kieu", "[1, 2].median()", "VS0403"),
        ("sugar-arity", "[1.0, 2.0].median(1)", "VS0401"),
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
        "use std.list\n\nfn main():\n    let x: float = list.median([1.0, 2.0])\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-float",
        "fn main():\n    let x: float = [1.0, 2.0].median()\n    print(x)\n",
        "VS0412",
    );
}
