// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `std.list.windows(xs, n)`: **cửa sổ trượt** độ dài `n`, chồng nhau đúng
//! `n - 1` phần tử (bước 1), theo thứ tự xuất hiện.
//!
//! Hợp đồng được khoá ở đây:
//! - `n == 1` ⇒ mỗi phần tử là một cửa sổ một phần tử; `n == len(xs)` ⇒ **đúng một** cửa sổ;
//! - `n > len(xs)` ⇒ `[]` (không phải lỗi);
//! - `xs == []` ⇒ `[]` với mọi `n > 0`;
//! - `n <= 0` ⇒ **trap `VS0501`** (exit 101) nêu `list.windows`, **không** panic Rust; kiểm
//!   `n` **trước** dữ liệu nên `list.windows([], 0)` cũng trap (không trả `[]`);
//! - kiểu trả **`list[list[T]]`** (`Ty::List(&Ty::List(&Ty::Var(0)))` trong bảng đóng băng);
//!   `T` suy từ `xs` nên `let w: list[list[int]] = list.windows(xs, 2)` hợp lệ, còn gán sang
//!   `list[list[str]]` bị `VS0412`;
//! - sugar R5c `xs.windows(2)` ≡ `list.windows(xs, 2)` (tham số đầu `list[T]`).
//!
//! Mọi ca chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.
//!
//! **RED-BEFORE** (trước khi thêm hàm, chính binary `vis` của cây này; tệp dò
//! `list.windows([1, 2, 3], 2)`):
//!
//! ```text
//! $ vis check win.vis
//! TAP version 13
//! 1..1
//! not ok 1 - win.vis
//! # VS0301 4:11 module `std.list` không có hàm `windows`
//! $ vis run --no-cache win.vis       ⇒ exit 2, VS0301 (interp)
//! $ vis run --no-cache --vm win.vis  ⇒ exit 2, VS0301 (VM)
//! ```
//!
//! Không ca dương nào chạy tới `print`; ca sugar còn đỏ sớm hơn (`VS0403` — `list[int]` không
//! có trường `windows`, vì R5c cũ chỉ khớp tham số đầu **không** generic). Sau khi thêm:
//! 8/8 ca xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-{tag}-{}-{n}", std::process::id()));
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
// 1 — biên độ dài: n = 1, 2, len, len+1, danh sách rỗng
// ===========================================================================

/// Cửa sổ trượt bước 1; mọi biên độ dài; phần tử `int`/`str`.
#[test]
fn windows_hop_le_moi_do_dai() {
    dung(
        "win-bien",
        r#"use std.list

fn main():
    print(list.windows([1, 2, 3, 4], 1))
    print(list.windows([1, 2, 3, 4], 2))
    print(list.windows([1, 2, 3, 4], 3))
    print(list.windows([1, 2, 3], 3))
    print(list.windows([1, 2, 3], 4))
    print(list.windows([1], 1))
    print(list.windows([1], 2))
    print(list.windows([], 1))
    print(list.windows([], 7))
    print(list.windows(["a", "b", "c"], 2))
"#,
        "[[1], [2], [3], [4]]\n[[1, 2], [2, 3], [3, 4]]\n[[1, 2, 3], [2, 3, 4]]\n[[1, 2, 3]]\n[]\n[[1]]\n[]\n[]\n[]\n[[a, b], [b, c]]\n",
    );
}

/// Phần tử là **list lồng nhau**: cửa sổ giữ nguyên giá trị từng phần tử (không làm phẳng).
#[test]
fn windows_danh_sach_long() {
    dung(
        "win-long",
        r#"use std.list

fn main():
    print(list.windows([[1, 2], [3, 4], [5, 6]], 2))
    print(list.windows([[1], [], [2, 3]], 3))
    print(list.windows([[[1]], [[2]]], 2))
"#,
        "[[[1, 2], [3, 4]], [[3, 4], [5, 6]]]\n[[[1], [], [2, 3]]]\n[[[[1]], [[2]]]]\n",
    );
}

// ===========================================================================
// 2 — kiểu trả `list[list[T]]`
// ===========================================================================

/// `T` suy từ `xs`; gán cho `list[list[int]]` hợp lệ, chỉ số cho `list[int]` (không phải `int`).
#[test]
fn windows_giu_kieu_long_qua_bien_gan() {
    dung(
        "win-kieu",
        r#"use std.list

fn main():
    let xs: list[int] = [3, 1, 4, 1, 5]
    let w: list[list[int]] = list.windows(xs, 2)
    print(w)
    let dau: list[int] = w[0]
    print(dau)
    print(w.len())
"#,
        "[[3, 1], [1, 4], [4, 1], [1, 5]]\n[3, 1]\n4\n",
    );
}

/// Gán kết quả sang kiểu phần tử khác (`list[list[str]]`) bị checker chặn (`VS0412`).
#[test]
fn windows_sai_kieu_ket_qua_bi_tu_choi() {
    tu_choi(
        "win-kieu-sai",
        "use std.list\n\nfn main():\n    let w: list[list[str]] = list.windows([1, 2, 3], 2)\n    print(w)\n",
        "VS0412",
    );
}

// ===========================================================================
// 3 — sugar R5c
// ===========================================================================

/// `xs.windows(2)` ≡ `list.windows(xs, 2)`; kiểu trả vẫn là `list[list[int]]`.
#[test]
fn sugar_phuong_thuc_list() {
    dung(
        "win-sugar",
        r#"use std.list

fn main():
    let xs: list[int] = [1, 2, 3]
    print(xs.windows(2))
    let w: list[list[int]] = xs.windows(2)
    print(w[1])
    print(["a", "b", "c"].windows(2))
"#,
        "[[1, 2], [2, 3]]\n[2, 3]\n[[a, b], [b, c]]\n",
    );
}

// ===========================================================================
// 4 — ca âm chạy: n <= 0 là trap, không panic
// ===========================================================================

/// `n == 0` và `n < 0` ⇒ trap `VS0501` nêu `list.windows`; áp cả khi `xs` rỗng (kiểm `n` trước).
#[test]
fn windows_n_khong_duong_la_trap() {
    trap_ca_hai(
        "win-0",
        "use std.list\n\nfn main():\n    print(list.windows([1, 2, 3], 0))\n",
        "list.windows",
    );
    trap_ca_hai(
        "win-am",
        "use std.list\n\nfn main():\n    print(list.windows([1, 2, 3], -2))\n",
        "list.windows",
    );
    trap_ca_hai(
        "win-rong-0",
        "use std.list\n\nfn main():\n    let xs: list[int] = []\n    print(list.windows(xs, 0))\n",
        "list.windows",
    );
    trap_ca_hai(
        "win-sugar-0",
        "use std.list\n\nfn main():\n    let xs: list[int] = [1]\n    print(xs.windows(0))\n",
        "list.windows",
    );
}

// ===========================================================================
// 5 — ca âm tĩnh: kiểu đối số, số đối số
// ===========================================================================

/// `n` phải `int`; `xs` phải list — sai ⇒ `VS0412`.
#[test]
fn windows_sai_kieu_doi_so_bi_tu_choi() {
    tu_choi(
        "win-n-str",
        "use std.list\n\nfn main():\n    print(list.windows([1, 2], \"2\"))\n",
        "VS0412",
    );
    tu_choi(
        "win-xs-str",
        "use std.list\n\nfn main():\n    print(list.windows(\"abc\", 2))\n",
        "VS0412",
    );
}

/// Sai số đối số ⇒ `VS0401` (thiếu và thừa).
#[test]
fn windows_sai_so_doi_bi_tu_choi() {
    tu_choi(
        "win-thieu",
        "use std.list\n\nfn main():\n    print(list.windows([1, 2]))\n",
        "VS0401",
    );
    tu_choi(
        "win-thua",
        "use std.list\n\nfn main():\n    print(list.windows([1, 2], 1, 3))\n",
        "VS0401",
    );
}
