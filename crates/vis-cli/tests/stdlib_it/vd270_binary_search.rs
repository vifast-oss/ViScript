// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (binary-search)** — `std.list.binary_search(xs, value) -> int?`.
//!
//! Hợp đồng được khoá ở đây:
//! - `xs` phải **đã sắp tăng** theo **cùng** thứ tự `list.sort`/`list.min`/`list.max`
//!   (`Value::cmp_value`); hàm **không** kiểm lại thứ tự.
//! - Trả **`Some(i)`** với `i` là **biên dưới** — vị trí **đầu tiên** của cụm phần tử bằng
//!   `value` (trùng lặp ⇒ vị trí đầu, không phải một vị trí bất kỳ).
//! - Không có ⇒ `None`; `xs` rỗng ⇒ `None`; phần tử không so được tại điểm giữa ⇒ `None`
//!   (không trap).
//! - Chưa sắp ⇒ **hành vi không xác định** nhưng **không trap**: vẫn trả `int?` (có thể `None`
//!   dù phần tử có mặt). Ca dưới chỉ khẳng định "chạy sạch + đúng hình dạng `int?`".
//! - Chữ ký `(list[T], T) -> int?` (`Ty::OptInt` → `Option[int]`); `T` buộc từ `xs` nên
//!   `value` sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.
//! - Sugar R5c `xs.binary_search(v)` ≡ `list.binary_search(xs, v)`, không cần `use std.list`.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so **byte** stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `e17998f`):
//!
//! ```text
//! $ vis check probe.vis                  # có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 6:11 module `std.list` không có hàm `binary_search`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:6:11: error VS0301: module `std.list` không có hàm `binary_search`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng dòng)
//! $ vis check sugar.vis                  # xs.binary_search(20), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `list[int]` không có trường `binary_search`
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-bs-{tag}-{}-{n}", std::process::id()));
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

/// Một dòng stdout phải là `None` hoặc `Some(<int>)` — dùng cho ca "không xác định".
fn la_option_int(dong: &str) -> bool {
    if dong == "None" {
        return true;
    }
    let Some(ruot) = dong.strip_prefix("Some(").and_then(|s| s.strip_suffix(')')) else {
        return false;
    };
    ruot.parse::<i64>().is_ok()
}

// ===========================================================================
// 1 — tìm thấy ở đầu / giữa / cuối
// ===========================================================================

/// Danh sách sắp tăng: phần tử đầu, giữa, cuối đều trả đúng vị trí **0-based**.
#[test]
fn dau_giua_cuoi() {
    dung(
        "dau-giua-cuoi",
        r#"use std.list

fn main():
    let xs = [10, 20, 30, 40, 50]
    print(list.binary_search(xs, 10))
    print(list.binary_search(xs, 30))
    print(list.binary_search(xs, 50))
    let mot = [7]
    print(list.binary_search(mot, 7))
"#,
        "Some(0)\nSome(2)\nSome(4)\nSome(0)\n",
    );
}

// ===========================================================================
// 2 — không có: dưới / trên / kẹp giữa
// ===========================================================================

/// Giá trị nhỏ hơn mọi phần tử, lớn hơn mọi phần tử, hoặc nằm giữa hai phần tử kề ⇒ `None`.
#[test]
fn khong_co_duoi_tren_va_kep_giua() {
    dung(
        "khong-co",
        r#"use std.list

fn main():
    let xs = [10, 20, 30, 40]
    print(list.binary_search(xs, 5))
    print(list.binary_search(xs, 45))
    print(list.binary_search(xs, 25))
    print(list.binary_search(xs, 35))
    let mot = [7]
    print(list.binary_search(mot, 6))
    print(list.binary_search(mot, 8))
"#,
        "None\nNone\nNone\nNone\nNone\nNone\n",
    );
}

// ===========================================================================
// 3 — danh sách rỗng
// ===========================================================================

/// `list[int]`, `list[str]` rỗng ⇒ `None` (không trap).
#[test]
fn danh_sach_rong() {
    dung(
        "rong",
        r#"use std.list

fn main():
    let r: list[int] = []
    let s: list[str] = []
    print(list.binary_search(r, 1))
    print(list.binary_search(s, "x"))
"#,
        "None\nNone\n",
    );
}

// ===========================================================================
// 4 — trùng lặp ⇒ biên dưới
// ===========================================================================

/// Cụm bằng nhau trả vị trí **đầu tiên** (biên dưới), kể cả khi cả danh sách toàn một giá
/// trị; phần tử lớn hơn cụm vẫn tìm đúng.
#[test]
fn trung_lap_tra_bien_duoi() {
    dung(
        "trung-lap",
        r#"use std.list

fn main():
    let xs = [1, 2, 2, 2, 2, 3, 5, 5, 9]
    print(list.binary_search(xs, 2))
    print(list.binary_search(xs, 5))
    print(list.binary_search(xs, 1))
    print(list.binary_search(xs, 9))
    let cung = [4, 4, 4, 4]
    print(list.binary_search(cung, 4))
    print(list.binary_search(cung, 3))
    print(list.binary_search(cung, 5))
"#,
        "Some(1)\nSome(6)\nSome(0)\nSome(8)\nSome(0)\nNone\nNone\n",
    );
}

// ===========================================================================
// 5 — số âm và `str`
// ===========================================================================

/// Số âm sắp tăng (giá trị đại số) và chuỗi (thứ tự byte/scalar của `str`) đều tìm đúng; biên
/// dưới vẫn áp dụng cho `str` trùng.
#[test]
fn so_am_va_chuoi() {
    dung(
        "am-chuoi",
        r#"use std.list

fn main():
    let am = [-100, -9, -5, -5, -1, 0, 3]
    print(list.binary_search(am, -100))
    print(list.binary_search(am, -5))
    print(list.binary_search(am, 0))
    print(list.binary_search(am, -6))
    print(list.binary_search(am, 2))
    let chu = ["a", "b", "b", "đ", "😀"]
    print(list.binary_search(chu, "a"))
    print(list.binary_search(chu, "b"))
    print(list.binary_search(chu, "đ"))
    print(list.binary_search(chu, "c"))
"#,
        "Some(0)\nSome(2)\nSome(5)\nNone\nNone\nSome(0)\nSome(1)\nSome(3)\nNone\n",
    );
}

// ===========================================================================
// 6 — chưa sắp: chạy được, không trap, vẫn là `int?`
// ===========================================================================

/// **Hành vi không xác định** khi `xs` chưa sắp, nhưng hợp đồng tối thiểu: **không trap**, cả
/// hai engine chạy sạch, mỗi dòng là `None` hoặc `Some(<int>)` (không phải giá trị rác/khác
/// kiểu). Không khoá **vị trí cụ thể** vì đó là hệ quả của thuật toán trên dữ liệu sai điều
/// kiện tiên quyết.
#[test]
fn chua_sap_chay_duoc_khong_trap() {
    let (code, out, err) = hai_engine(
        "chua-sap",
        r#"use std.list

fn main():
    let xs = [3, 1, 2, 5, 4]
    print(list.binary_search(xs, 1))
    print(list.binary_search(xs, 2))
    print(list.binary_search(xs, 3))
    print(list.binary_search(xs, 9))
"#,
    );
    assert_eq!(
        code, 0,
        "chua-sap: không được trap (exit {code})\nstdout: {out}\nstderr: {err}"
    );
    assert!(err.is_empty(), "chua-sap: stderr phải rỗng: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(dong.len(), 4, "chua-sap: phải in 4 dòng, được: {out:?}");
    for d in dong {
        assert!(
            la_option_int(d),
            "chua-sap: dòng `{d}` không phải `int?` (`None`/`Some(int)`)"
        );
    }
}

// ===========================================================================
// 7 — `int?` là `Option[int]` thật; sugar R5c
// ===========================================================================

/// Kết quả gán được `int?`, `match Some/None` chạy; sugar `xs.binary_search(v)` không cần
/// `use std.list` và dùng **đúng** kiểu phần tử (int/str).
#[test]
fn option_that_va_sugar_phuong_thuc() {
    dung(
        "sugar",
        r#"fn main():
    let xs = [10, 20, 20, 30]
    let a: int? = xs.binary_search(20)
    print(a)
    print(xs.binary_search(10))
    print(xs.binary_search(30))
    print(xs.binary_search(99))
    print([1, 2, 2, 3].binary_search(2))
    print(["a", "b"].binary_search("b"))
    match xs.binary_search(20):
        Some(i): print(i)
        None: print("none")
    match xs.binary_search(77):
        Some(_): print("some")
        None: print("none")
"#,
        "Some(1)\nSome(0)\nSome(3)\nNone\nSome(1)\nSome(1)\n1\nnone\n",
    );
}

// ===========================================================================
// 8 — ca âm: kiểu / arity (module + sugar)
// ===========================================================================

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine); `value` phải cùng kiểu phần tử (`T` buộc từ `xs`), sugar cũng kiểm kiểu.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-list", "list.binary_search(1, 1)", "VS0412"),
        ("kieu-value", "list.binary_search([1, 2], \"x\")", "VS0412"),
        ("kieu-float", "list.binary_search([1, 2], 2.0)", "VS0412"),
        ("arity-0", "list.binary_search()", "VS0401"),
        ("arity-1", "list.binary_search([1, 2])", "VS0401"),
        ("arity-3", "list.binary_search([1, 2], 2, 3)", "VS0401"),
        ("sugar-kieu", "[1, 2].binary_search(\"x\")", "VS0412"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// `int?` không gán được sang `int` (module + sugar) — kiểu trả là `Option[int]` thật.
#[test]
fn sai_kieu_ket_qua_bi_chan() {
    tu_choi(
        "ket-qua-sang-int",
        "use std.list\n\nfn main():\n    let x: int = list.binary_search([1, 2], 2)\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-ket-qua-sang-int",
        "fn main():\n    let x: int = [1, 2].binary_search(2)\n    print(x)\n",
        "VS0412",
    );
}

/// **VD-270 (R5c arity)**: sugar `xs.binary_search(…)` thiếu/thừa đối số bị checker chặn trước
/// khi chạy — `VS0401` `list.binary_search cần 1 đối số`, cùng thông điệp như module call; không
/// còn trap `VS0501` (exit 101).
#[test]
fn sugar_sai_arity_bi_chan() {
    for (tag, src) in [
        (
            "sugar-arity-thieu",
            "fn main():\n    let xs = [1, 2]\n    print(xs.binary_search())\n",
        ),
        (
            "sugar-arity-thua",
            "fn main():\n    let xs = [1, 2]\n    print(xs.binary_search(1, 2))\n",
        ),
    ] {
        tu_choi(tag, src, "VS0401");
    }
}
