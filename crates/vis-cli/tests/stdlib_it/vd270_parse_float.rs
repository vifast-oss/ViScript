// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (parse-float)** — `std.str.parse_float(s) -> float?`.
//!
//! Hợp đồng: parse bằng **cú pháp thập phân của Rust** (`f64::from_str`): dấu `+`/`-` tuỳ chọn,
//! phần nguyên/thập phân có thể khuyết một phía (`.5`, `5.`), mũ `e`/`E` có dấu; **không**
//! khoảng trắng, `_`, `0x`; chuỗi rỗng/sai cú pháp ⇒ `None`. Sau khi parse, **từ chối mọi giá trị
//! không hữu hạn** (`inf`/`-inf`/`NaN`, kể cả tràn `1e400` ⇒ `inf`) ⇒ `None`; **underflow** thành
//! `0.0`/`-0.0` vẫn `Some` (giữ dấu `-0`). Không trap.
//!
//! Sugar R5c `s.parse_float()`. Mọi ca chạy cả hai engine và so byte stdout/stderr/exit.
//!
//! **RED-BEFORE**: trước khi cài, `VS0301 module 'std.str' không có hàm 'parse_float'`
//! (và sugar `"1.5".parse_float()` ⇒ `VS0403`).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-pf-{tag}-{}-{n}", std::process::id()));
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
// 1 — bảng hợp lệ: số nguyên/thập phân/mũ, `.5`, `5.`, dấu, số 0 âm
// ===========================================================================

/// Cú pháp Rust thập phân: `42`, `+1.5`, `-0.0`, `.5`, `5.`, `1e3`, `1E-3`, `+.5`, `-.5`,
/// `5.e2`, `.5e2`, `007`; in `float` theo `Value::show` (`Some(-0)` cho `-0.0`).
#[test]
fn hop_le_bang_cu_phap_rust() {
    dung(
        "valid",
        r#"use std.str

fn main():
    print(str.parse_float("42"))
    print(str.parse_float("+1.5"))
    print(str.parse_float("-0.0"))
    print(str.parse_float(".5"))
    print(str.parse_float("5."))
    print(str.parse_float("1e3"))
    print(str.parse_float("1E-3"))
    print(str.parse_float("+.5"))
    print(str.parse_float("-.5"))
    print(str.parse_float("5.e2"))
    print(str.parse_float(".5e2"))
    print(str.parse_float("007"))
    print(str.parse_float("0.1"))
    print(str.parse_float("-2.5e-2"))
"#,
        "Some(42)\nSome(1.5)\nSome(-0)\nSome(0.5)\nSome(5)\nSome(1000)\nSome(0.001)\n\
Some(0.5)\nSome(-0.5)\nSome(500)\nSome(50)\nSome(7)\nSome(0.1)\nSome(-0.025)\n",
    );
}

// ===========================================================================
// 2 — underflow ⇒ ±0.0 vẫn `Some` (giữ dấu âm); không phải `None`
// ===========================================================================

/// Dưới ngưỡng `f64` ⇒ `0.0`/`-0.0` (hữu hạn) nên **hợp lệ**; dấu `-` được giữ (`Some(-0)`).
#[test]
fn underflow_thanh_zero_van_some() {
    dung(
        "underflow",
        r#"use std.str

fn main():
    print(str.parse_float("1e-400"))
    print(str.parse_float("-1e-400"))
    print(str.parse_float("1e-999"))
    print(str.parse_float("-2.4e-324"))
"#,
        "Some(0)\nSome(-0)\nSome(0)\nSome(-0)\n",
    );
}

// ===========================================================================
// 3 — không hữu hạn: `inf`/`NaN`/tràn ⇒ `None` (API không biểu diễn non-finite)
// ===========================================================================

/// Literal không hữu hạn của Rust (`inf`, `-inf`, `infinity`, `NaN`, `nan`) và **tràn**
/// (`1e400`, `-1e400`) đều parse được thành `inf`/`NaN` nhưng bị **từ chối** ⇒ `None`.
#[test]
fn khong_huu_han_va_tran_tra_none() {
    dung(
        "nonfinite",
        r#"use std.str

fn main():
    print(str.parse_float("inf"))
    print(str.parse_float("-inf"))
    print(str.parse_float("+inf"))
    print(str.parse_float("infinity"))
    print(str.parse_float("-infinity"))
    print(str.parse_float("NaN"))
    print(str.parse_float("nan"))
    print(str.parse_float("-NaN"))
    print(str.parse_float("1e400"))
    print(str.parse_float("-1e400"))
    print(str.parse_float("1.8e308"))
    print(str.parse_float("1e999999"))
"#,
        "None\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 4 — sai cú pháp: rỗng/khoảng trắng/`_`/`0x`/thiếu chữ số ⇒ `None`, không trap
// ===========================================================================

/// Từ chối: rỗng, khoảng trắng hai phía, `_` phân cách, hex, chấm trần, mũ thiếu số, dấu lặp.
#[test]
fn sai_cu_phap_tra_none() {
    dung(
        "invalid",
        r#"use std.str

fn main():
    print(str.parse_float(""))
    print(str.parse_float(" "))
    print(str.parse_float(" 42"))
    print(str.parse_float("42 "))
    print(str.parse_float("1_000"))
    print(str.parse_float("0x10"))
    print(str.parse_float("1,5"))
    print(str.parse_float("4.2.3"))
    print(str.parse_float("."))
    print(str.parse_float("+"))
    print(str.parse_float("-"))
    print(str.parse_float("e3"))
    print(str.parse_float("1e"))
    print(str.parse_float("1e+"))
    print(str.parse_float("++1"))
    print(str.parse_float("--1"))
    print(str.parse_float("abc"))
    print(str.parse_float("1.5f64"))
"#,
        "None\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 5 — `float?` là Option thật: match Some/None, gán `float?`, không gán `float`
// ===========================================================================

/// `Option[float]` thật: `match`, gán `float?`, in payload; gán vào `float` ⇒ `VS0412`.
#[test]
fn ket_qua_la_option_float_that() {
    dung(
        "option",
        r#"use std.str

fn main():
    let a: float? = str.parse_float("2.5")
    print(a)
    let b: float? = str.parse_float("x")
    print(b)
    match a:
        Some(v): print(v)
        None: print("none")
    match b:
        Some(v): print(v)
        None: print("none")
"#,
        "Some(2.5)\nNone\n2.5\nnone\n",
    );
    tu_choi(
        "option-kieu-sai",
        "use std.str\n\nfn main():\n    let x: float = str.parse_float(\"1.5\")\n    print(x)\n",
        "VS0412",
    );
}

// ===========================================================================
// 6 — sugar R5c `s.parse_float()` (không cần `use std.str`)
// ===========================================================================

/// Sugar phương thức ≡ gọi module; kiểu trả qua sugar vẫn `float?`; match được `Some/None`.
#[test]
fn sugar_phuong_thuc_khong_can_use() {
    dung(
        "sugar",
        r#"fn main():
    let s: str = "2.5"
    print(s.parse_float())
    print("1e3".parse_float())
    print("-0.0".parse_float())
    print("x".parse_float())
    let o: float? = s.parse_float()
    print(o)
    match "banana".parse_float():
        Some(v): print(v)
        None: print("none")
    match "0.125".parse_float():
        Some(v): print(v)
        None: print("none")
"#,
        "Some(2.5)\nSome(1000)\nSome(-0)\nNone\nSome(2.5)\nnone\n0.125\n",
    );
}

// ===========================================================================
// 7 — ca âm tĩnh: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); sugar phương thức cũng kiểm kiểu đối số.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "str.parse_float(1)", "VS0412"),
        ("kieu-2", "str.parse_float(\"1\", \"2\")", "VS0401"),
        ("arity-1", "str.parse_float()", "VS0401"),
        ("arity-2", "str.parse_float(\"1\", \"2\", \"3\")", "VS0401"),
        ("sugar-arity", "\"1\".parse_float(1)", "VS0401"),
        ("sugar-doi-tuong", "1.parse_float()", "VS0403"),
    ] {
        let src = format!("use std.str\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
