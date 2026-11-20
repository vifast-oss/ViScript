// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (list-sum-product)** — `std.list.sum(xs)` / `std.list.product(xs)`.
//!
//! Hợp đồng được khoá ở đây:
//! - `sum(xs: list[int]) -> int`: tổng các phần tử, **rỗng ⇒ `0`**; cộng **checked** theo
//!   `i64` (như phép `+`), tràn ở **bất kỳ bước nào** ⇒ trap `VS0507`, **không wrap**.
//! - `product(xs: list[int]) -> int`: tích các phần tử, **rỗng ⇒ `1`**; nhân **checked**,
//!   tràn ở bất kỳ bước nào ⇒ trap `VS0507`.
//! - Cả hai gộp **trái → phải** (fold trái), dừng ngay tại bước tràn: `[MAX, 1, -1]` **trap**
//!   ở `MAX + 1` dù kết quả toán học bằng `MAX`; `[MAX, 2, 0]` **trap** ở `MAX * 2` dù tích
//!   cuối bằng `0`.
//! - Sugar R5c: `xs.product()` ≡ `list.product(xs)` (không cần `use std.list`).
//!   `xs.sum()` **giữ** phương thức lõi `sum` (đứng trước R5c): với `list[int]` nó cộng
//!   checked y hệt `list.sum`, nên **không hồi quy**; nó vẫn nhận `list[float]` (tổng float)
//!   như trước, còn `list.sum` chỉ nhận `list[int]` — khác biệt có chủ đích, ghi trong
//!   `stdlib/listmath.rs`.
//! - Sai kiểu/sai arity bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
//!   hai engine); sugar cũng kiểm arity (`VS0401`).
//! - Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, binary `vis` của cây này @ `a456c5a`):
//!
//! ```text
//! $ vis check probe.vis                  # probe: list.sum + list.product, có use std.list
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.list` không có hàm `sum`
//! $ vis run --no-cache probe.vis         ⇒ exit 2
//! probe.vis:5:11: error VS0301: module `std.list` không có hàm `sum`
//! probe.vis:6:11: error VS0301: module `std.list` không có hàm `product`
//! $ vis run --no-cache --vm probe.vis    ⇒ exit 2 (cùng hai dòng)
//! $ vis check sugar.vis                  # sugar: xs.sum() + xs.product(), không use
//! not ok 1 - sugar.vis
//! # VS0403 4:11 `list[int]` không có trường `product`
//! $ vis run --no-cache sugar.vis         ⇒ exit 2
//! sugar.vis:4:11: error VS0403: `list[int]` không có trường `product`
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
    let d = std::env::temp_dir().join(format!("vis-vd270-lsp-{tag}-{}-{n}", std::process::id()));
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

/// Trap **lúc chạy** trên **cả hai** engine: exit 101, `stderr` có `VS0507`, stdout đúng phần
/// đã in trước đó (không có kết quả nửa vời).
fn trap_hai_engine(tag: &str, src: &str, stdout_truoc: &str) {
    let (code, out, err) = hai_engine(tag, src);
    assert_eq!(
        code, 101,
        "{tag}: phải trap VS0507 (exit 101), được {code}\nstdout: {out}\nstderr: {err}"
    );
    assert_eq!(out, stdout_truoc, "{tag}: stdout trước khi trap sai: {out}");
    assert!(
        err.contains("VS0507"),
        "{tag}: thiếu VS0507 (tràn i64)\nstdout: {out}\nstderr: {err}"
    );
}

// ===========================================================================
// 1 — rỗng, số dương, số âm, trộn, có số 0
// ===========================================================================

/// Danh sách rỗng dùng **phần tử đơn vị**: `sum` ⇒ `0`, `product` ⇒ `1`; các ca còn lại khoá
/// dấu/thứ tự và trường hợp có `0` (tích bằng `0` ngay, không lỗi).
#[test]
fn rong_duong_am_tron_va_so_khong() {
    dung(
        "co-ban",
        r#"use std.list

fn main():
    let r: list[int] = []
    print(list.sum(r))
    print(list.product(r))
    print(list.sum([1, 2, 3]))
    print(list.product([1, 2, 3]))
    print(list.sum([-4, -6]))
    print(list.product([-4, -6]))
    print(list.sum([10, -3, 7]))
    print(list.product([10, -3, 7]))
    print(list.sum([5, 0, -5]))
    print(list.product([5, 0, -5]))
"#,
        "0\n1\n6\n6\n-10\n24\n14\n-210\n0\n0\n",
    );
}

/// Biên `i64` **không tràn** phải cho đúng kết quả: `MAX`, `MIN`, tổng `MIN + MAX = -1`,
/// nhân với `1`/`0` giữ nguyên hoặc về `0`.
#[test]
fn bien_i64_khong_tran() {
    dung(
        "bien-i64",
        r#"use std.list

fn main():
    let max = 9223372036854775807
    let min = -9223372036854775808
    print(list.sum([max]))
    print(list.sum([max, 0]))
    print(list.sum([min, 0]))
    print(list.sum([min, max]))
    print(list.product([max]))
    print(list.product([max, 1]))
    print(list.product([min, 1]))
    print(list.product([max, -1, 0]))
    print(list.product([0, max]))
"#,
        "9223372036854775807\n9223372036854775807\n-9223372036854775808\n-1\n9223372036854775807\n9223372036854775807\n-9223372036854775808\n0\n0\n",
    );
}

// ===========================================================================
// 2 — tràn: cộng/tích checked, trap `VS0507` ở cả hai engine
// ===========================================================================

/// Cộng checked: `MAX + 1`, `MIN + -1`, và tràn **trung gian** `[MAX, 1, -1]` đều trap
/// `VS0507` — không wrap, không bỏ qua bước tràn dù kết quả cuối "đẹp".
#[test]
fn tran_cong_trap_hai_engine() {
    let max = "9223372036854775807";
    let min = "-9223372036854775808";
    for (tag, xs) in [
        ("sum-max-1", format!("[{max}, 1]")),
        ("sum-min--1", format!("[{min}, -1]")),
        ("sum-trung-gian", format!("[{max}, 1, -1]")),
    ] {
        let src = format!(
            "use std.list\n\nfn main():\n    print(\"truoc\")\n    print(list.sum({xs}))\n"
        );
        trap_hai_engine(tag, &src, "truoc\n");
    }
}

/// Nhân checked: `MAX * 2`, `MIN * -1`, tràn trung gian `[MAX, 2, 0]` (dù tích cuối `0`), và
/// cặp `3037000500² > i64::MAX` đều trap `VS0507`.
#[test]
fn tran_nhan_trap_hai_engine() {
    let max = "9223372036854775807";
    let min = "-9223372036854775808";
    for (tag, xs) in [
        ("product-max-2", format!("[{max}, 2]")),
        ("product-min--1", format!("[{min}, -1]")),
        ("product-trung-gian", format!("[{max}, 2, 0]")),
        ("product-3037000500", "[3037000500, 3037000500]".to_string()),
    ] {
        let src = format!(
            "use std.list\n\nfn main():\n    print(\"truoc\")\n    print(list.product({xs}))\n"
        );
        trap_hai_engine(tag, &src, "truoc\n");
    }
    // Phép `*` của ngôn ngữ trên cùng cặp cũng trap — product không được wrap.
    trap_hai_engine(
        "product-so-sanh-phep-nhan",
        "fn main():\n    print(\"truoc\")\n    print(9223372036854775807 * 2)\n",
        "truoc\n",
    );
}

// ===========================================================================
// 3 — sugar R5c và không hồi quy `sum()` lõi
// ===========================================================================

/// `xs.product()` là sugar R5c của `list.product` (không cần `use std.list`); `xs.sum()`
/// vẫn là phương thức lõi nhưng với `list[int]` cho **cùng** kết quả `list.sum`.
#[test]
fn sugar_product_va_sum_loi() {
    dung(
        "sugar",
        r#"fn main():
    let xs: list[int] = [2, 3, 4]
    print(xs.sum())
    print(xs.product())
    print([5, 6].product())
    print([5, 6].sum())
    let a: int = xs.sum()
    let b: int = xs.product()
    print(a + b)
"#,
        "9\n24\n30\n11\n33\n",
    );
}

/// Không hồi quy phương thức lõi `sum`: `list[int]` rỗng ⇒ `0`, cộng checked (tràn ⇒
/// `VS0507`), và **vẫn** nhận `list[float]` (tổng float) — điều `list.sum` (chỉ `list[int]`)
/// không làm. Đây là lý do `xs.sum()` **không** bị R5c đổi sang `list.sum`.
#[test]
fn sum_loi_khong_hoi_quy() {
    dung(
        "sum-loi",
        r#"fn main():
    let r: list[int] = []
    print(r.sum())
    print([1, 2, 3].sum())
    let f: list[float] = [1.5, 2.5]
    print(f.sum())
"#,
        "0\n6\n4\n",
    );
    trap_hai_engine(
        "sum-loi-tran",
        "fn main():\n    print(\"truoc\")\n    print([9223372036854775807, 1].sum())\n",
        "truoc\n",
    );
}

// ===========================================================================
// 4 — kiểu trả `int` và ca âm kiểu/arity
// ===========================================================================

/// Kết quả là `int` **thật**: gán `int`, cộng tiếp, và so sánh `list.sum` ↔ `xs.sum`.
#[test]
fn ket_qua_la_int_that() {
    dung(
        "int-that",
        r#"use std.list

fn main():
    let xs: list[int] = [1, 2, 3]
    let s: int = list.sum(xs)
    let p: int = list.product(xs)
    print(s + 1)
    print(p * 2)
    print(s + p)
    print(xs.sum() == list.sum(xs))
    print(xs.product() == list.product(xs))
"#,
        "7\n12\n12\ntrue\ntrue\n",
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine). `list[float]`/`list[str]`/không-phải-list đều `VS0412`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("sum-khong-list", "list.sum(1)", "VS0412"),
        ("sum-list-float", "list.sum([1.5, 2.5])", "VS0412"),
        ("sum-list-str", "list.sum([\"a\", \"b\"])", "VS0412"),
        ("product-khong-list", "list.product(1)", "VS0412"),
        ("product-list-float", "list.product([1.5, 2.5])", "VS0412"),
        ("product-list-str", "list.product([\"a\", \"b\"])", "VS0412"),
        ("sum-0-arg", "list.sum()", "VS0401"),
        ("sum-2-arg", "list.sum([1, 2], 3)", "VS0401"),
        ("product-0-arg", "list.product()", "VS0401"),
        ("product-2-arg", "list.product([1, 2], 3)", "VS0401"),
        ("sugar-product-arity", "[1, 2].product(3)", "VS0401"),
        ("sugar-sum-arity", "[1, 2].sum(3)", "VS0401"),
    ] {
        let src = format!("use std.list\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}

/// Kiểu trả không gán được sang `list[int]`/`str` (kể cả qua sugar `product`).
#[test]
fn ket_qua_int_khong_gan_sang_kieu_khac() {
    tu_choi(
        "sum-sang-list",
        "use std.list\n\nfn main():\n    let x: list[int] = list.sum([1, 2])\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "product-sang-str",
        "use std.list\n\nfn main():\n    let x: str = list.product([1, 2])\n    print(x)\n",
        "VS0412",
    );
    tu_choi(
        "sugar-product-sang-bool",
        "fn main():\n    let x: bool = [1, 2].product()\n    print(x)\n",
        "VS0412",
    );
}

/// Sugar `xs.product()` trên `list[float]` **không** khớp `list[int]` ⇒ checker báo không có
/// phương thức/trường (`VS0403`); người dùng phải dùng `xs.sum()`/`xs.product()` đúng kiểu
/// hoặc ép danh sách về `list[int]`. Khoá hành vi để sau này đổi mã lỗi phải có VD.
#[test]
fn sugar_product_tren_list_float_bi_tu_choi() {
    tu_choi(
        "sugar-product-float",
        "fn main():\n    print([1.5, 2.5].product())\n",
        "VS0403",
    );
}
