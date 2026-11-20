// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-minmax-float)** — `std.list.min_float(xs)` / `std.list.max_float(xs)`
//! `-> float?`.
//!
//! Hợp đồng được khoá ở đây:
//! - `xs` rỗng ⇒ `None` (không có cực trị, không trap).
//! - Ngược lại `Some(giá_trị_phần_tử)` — **đúng** phần tử đã lưu; `-0.0`/`+0.0` giữ nguyên
//!   dấu khi là cực trị **duy nhất**.
//! - `NaN` ở **bất kỳ vị trí** ⇒ `None` (tất định, cùng chính sách `list.median`, không trap).
//! - `±inf` so bình thường: `-inf` là nhỏ nhất, `+inf` là lớn nhất.
//! - Phần tử **bằng nhau** — kể cả `-0.0 == +0.0` — giữ **phần tử đầu tiên** theo thứ tự
//!   danh sách (`min([0.0, -0.0])` ⇒ `Some(+0)`, `min([-0.0, 0.0])` ⇒ `Some(-0)`).
//! - Chữ ký `(list[float]) -> float?` (`Ty::OptFloat` → `Option[float]`); sai kiểu ⇒ `VS0412`,
//!   sai arity ⇒ `VS0401`; kết quả `float?` **không** gán được `int`/`float`.
//! - Sugar R5c `xs.min_float()`/`xs.max_float()` ≡ lời gọi module, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `88da99c`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_minmax_float
//! test result: FAILED. 0 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
//! $ vis check probe.vis                  # probe: print(list.min_float(xs)), có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 6:11 module `std.list` không có hàm `min_float`
//! # VS0301 7:11 module `std.list` không có hàm `max_float`
//! $ vis check sugar.vis                  # print(xs.min_float()), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `list[float]` không có trường `min_float`
//! $ vis check static.vis                 # let x: int = list.min_float([1.0])
//! not ok 1 - static.vis
//! # VS0301 5:18 module `std.list` không có hàm `min_float`
//! ```
//!
//! Sau khi thêm: 9/9 ca xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-mmf-{tag}-{}-{n}", std::process::id()));
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
// 1 — danh sách rỗng ⇒ None (cả hai hàm)
// ===========================================================================

/// `list[float]` rỗng khai báo tường minh / literal rỗng qua sugar ⇒ `None`, không trap.
#[test]
fn rong_tra_none() {
    dung(
        "rong",
        r#"use std.list

fn main():
    let r: list[float] = []
    print(list.min_float(r))
    print(list.max_float(r))
    print([0.0].take(0).min_float())
    print([].max_float())
"#,
        "None\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 2 — một phần tử ⇒ chính nó (cả hai hàm)
// ===========================================================================

/// Danh sách một phần tử: `min_float` = `max_float` = phần tử đó, kể cả số âm.
#[test]
fn mot_phan_tu() {
    dung(
        "mot",
        r#"use std.list

fn main():
    print(list.min_float([1.5]))
    print(list.max_float([1.5]))
    print(list.min_float([-2.5]))
    print(list.max_float([-2.5]))
    print([7.25].min_float())
    print([7.25].max_float())
"#,
        "Some(1.5)\nSome(1.5)\nSome(-2.5)\nSome(-2.5)\nSome(7.25)\nSome(7.25)\n",
    );
}

// ===========================================================================
// 3 — trộn: không sắp, âm, trùng lặp
// ===========================================================================

/// Cực trị tìm trên danh sách **không sắp**; số âm/trộn dấu/trùng lặp đều đúng.
#[test]
fn co_ban_tron() {
    dung(
        "tron",
        r#"use std.list

fn main():
    let xs: list[float] = [3.0, 1.0, 2.0]
    print(list.min_float(xs))
    print(list.max_float(xs))
    print(xs)
    print(list.min_float([-1.0, -5.0, -3.0]))
    print(list.max_float([-1.0, -5.0, -3.0]))
    print(list.min_float([10.0, -2.5, 3.5, 0.0]))
    print(list.max_float([10.0, -2.5, 3.5, 0.0]))
    print(list.min_float([2.0, 2.0, 2.0]))
    print(list.max_float([2.0, 2.0, 2.0]))
"#,
        "Some(1)\nSome(3)\n[3, 1, 2]\nSome(-5)\nSome(-1)\nSome(-2.5)\nSome(10)\nSome(2)\nSome(2)\n",
    );
}

// ===========================================================================
// 4 — ±inf so bình thường
// ===========================================================================

/// `±inf` **không** phải `NaN` nên `-inf` là nhỏ nhất, `+inf` là lớn nhất; không trap.
#[test]
fn inf_so_binh_thuong() {
    dung(
        "inf",
        r#"use std.list

fn main():
    let inf = 1.0 / 0.0
    print(list.min_float([inf, 1.0, 2.0]))
    print(list.max_float([inf, 1.0, 2.0]))
    print(list.min_float([1.0, -inf, 2.0]))
    print(list.max_float([1.0, -inf, 2.0]))
    print(list.min_float([inf, -inf]))
    print(list.max_float([inf, -inf]))
    print(list.min_float([inf, inf]))
    print(list.max_float([-inf, -inf]))
"#,
        "Some(1)\nSome(inf)\nSome(-inf)\nSome(2)\nSome(-inf)\nSome(inf)\nSome(inf)\nSome(-inf)\n",
    );
}

// ===========================================================================
// 5 — NaN ở bất kỳ vị trí ⇒ None (cả hai hàm), không trap
// ===========================================================================

/// `NaN` đầu/giữa/cuối, một mình hoặc xen trong danh sách ⇒ `None`; exit 0, stderr rỗng.
#[test]
fn nan_tra_none() {
    dung(
        "nan",
        r#"use std.list

fn main():
    let nan = 0.0 / 0.0
    print(list.min_float([nan]))
    print(list.max_float([nan]))
    print(list.min_float([nan, 1.0]))
    print(list.max_float([1.0, nan]))
    print(list.min_float([1.0, nan, 2.0]))
    print(list.max_float([1.0, nan, 2.0]))
    print(list.min_float([1.0, 2.0, 3.0, nan]))
    print(list.max_float([1.0, 2.0, 3.0, nan]))
    print([1.0, nan].min_float())
    print([nan, 1.0].max_float())
"#,
        "None\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 6 — dấu `±0.0`: cực trị duy nhất giữ dấu; có cả hai ⇒ phần tử đầu
// ===========================================================================

/// `1.0 / x` cho `inf` với `+0` và `-inf` với `-0`, nên khoá được **dấu** của `Some`.
/// Cực trị duy nhất giữ đúng bit; có cả `-0` lẫn `+0` (bằng nhau theo `==`) ⇒ **đầu tiên**.
#[test]
fn zero_giu_dau_va_vi_tri_dau() {
    dung(
        "zero",
        r#"use std.list

fn dau(v: float?) -> str:
    match v:
        Some(m):
            if 1.0 / m < 0.0:
                return "-0"
            return "+0"
        None:
            return "none"

fn main():
    print(dau(list.min_float([-0.0])))
    print(dau(list.max_float([-0.0])))
    print(dau(list.min_float([0.0])))
    print(dau(list.max_float([0.0])))
    print(dau(list.min_float([1.0, -0.0])))
    print(dau(list.max_float([-1.0, -0.0])))
    print(dau(list.min_float([1.0, 0.0])))
    print(dau(list.max_float([-1.0, 0.0])))
    print(dau(list.min_float([-0.0, 0.0])))
    print(dau(list.max_float([-0.0, 0.0])))
    print(dau(list.min_float([0.0, -0.0])))
    print(dau(list.max_float([0.0, -0.0])))
    print(dau([0.0, -0.0].min_float()))
    print(dau([-0.0, 0.0].max_float()))
"#,
        "-0\n-0\n+0\n+0\n-0\n-0\n+0\n+0\n-0\n-0\n+0\n+0\n+0\n-0\n",
    );
}

// ===========================================================================
// 7 — `float?` là `Option[float]` thật; sugar R5c
// ===========================================================================

/// Kết quả gán được `float?`, `match Some/None` chạy; sugar không cần `use std.list`, kể cả
/// trên biến `list[float]` và literal.
#[test]
fn option_that_va_sugar_phuong_thuc() {
    dung(
        "sugar",
        r#"fn main():
    let xs = [3.0, 1.0, 2.0]
    let a: float? = xs.min_float()
    let b: float? = xs.max_float()
    print(a)
    print(b)
    print([4.0, 1.0].min_float())
    print([4.0, 1.0].max_float())
    print([].min_float())
    print([].max_float())
    match xs.min_float():
        Some(m): print(m)
        None: print("none")
    match [].max_float():
        Some(_): print("some")
        None: print("none")
"#,
        "Some(1)\nSome(3)\nSome(1)\nSome(4)\nNone\nNone\n1\nnone\n",
    );
}

// ===========================================================================
// 8 — ca âm: kiểu / arity (module + sugar), cả hai hàm
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); `min_float`/`max_float` chỉ nhận `list[float]` — `list[int]`/`list[str]` bị
/// từ chối.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for ham in ["min_float", "max_float"] {
        for (tag, call, ma) in [
            ("kieu-int", format!("list.{ham}(1)"), "VS0412"),
            ("kieu-list-int", format!("list.{ham}([1, 2])"), "VS0412"),
            ("kieu-list-str", format!("list.{ham}([\"1.0\"])"), "VS0412"),
            ("arity-0", format!("list.{ham}()"), "VS0401"),
            ("arity-2", format!("list.{ham}([1.0], [2.0])"), "VS0401"),
            ("sugar-kieu", format!("[1, 2].{ham}()"), "VS0403"),
            ("sugar-arity", format!("[1.0, 2.0].{ham}(1)"), "VS0401"),
        ] {
            let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
            tu_choi(&format!("{ham}-{tag}"), &src, ma);
        }
    }
}

/// `float?` không gán được sang `int` (module + sugar) — kiểu trả là `Option[float]` thật.
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    for ham in ["min_float", "max_float"] {
        tu_choi(
            &format!("{ham}-ket-qua-sang-int"),
            &format!(
                "use std.list\n\nfn main():\n    let x: int = list.{ham}([1.0, 2.0])\n    print(x)\n"
            ),
            "VS0412",
        );
        tu_choi(
            &format!("{ham}-sugar-ket-qua-sang-int"),
            &format!("fn main():\n    let x: int = [1.0, 2.0].{ham}()\n    print(x)\n"),
            "VS0412",
        );
    }
}
