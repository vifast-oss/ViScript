// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-265** — bổ sung stdlib thực dụng: `list.zip`, `list.min`, `list.max`, `str.lines`,
//! `str.split_whitespace`.
//!
//! Hợp đồng được khoá ở đây:
//! - `zip` dừng ở danh sách **ngắn hơn**; một bên rỗng ⇒ `[]`; cặp là list 2 phần tử.
//! - `min`/`max` dùng **cùng thứ tự** với `list.sort` (`cmp_value`); danh sách **rỗng** ⇒
//!   trap `VS0501` (exit 101), **không** panic; phần tử không so được (NaN/list) cũng trap.
//! - `lines` theo nghĩa Rust: tách `\n`, bỏ `\r` cuối dòng, dấu xuống dòng cuối **không**
//!   sinh dòng rỗng; `split_whitespace` tách theo khoảng trắng Unicode và bỏ phần rỗng.
//! - `s.lines()` / `s.split_whitespace()` là sugar R5c (`s.f(a) ≡ std.str.f(s, a)`).
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm`), so byte stdout/stderr/exit.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd265-{tag}-{}-{n}", std::process::id()));
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
// 1 — list.zip
// ===========================================================================

/// Cặp theo vị trí, dừng ở danh sách ngắn hơn; mọi biên độ dài.
#[test]
fn zip_hop_le_bien_do_dai() {
    dung(
        "zip-bien",
        r#"use std.list

fn main():
    print(list.zip([1, 2, 3], [10, 20, 30]))
    print(list.zip([1, 2, 3], [10, 20]))
    print(list.zip([1], [10, 20, 30]))
    print(list.zip([], [1, 2]))
    print(list.zip([1, 2], []))
    print(list.zip([], []))
    print(list.zip(["a", "b"], ["x", "y"]))
    print(list.zip([1.5, 2.5], [3.5, 4.5]))
"#,
        "[[1, 10], [2, 20], [3, 30]]\n[[1, 10], [2, 20]]\n[[1, 10]]\n[]\n[]\n[]\n[[a, x], [b, y]]\n[[1.5, 3.5], [2.5, 4.5]]\n",
    );
}

/// Kiểu trả `list[list[T]]` dùng được ở biên gán; `T` suy từ hai vế.
#[test]
fn zip_giu_kieu_qua_bien_gan() {
    dung(
        "zip-kieu",
        r#"use std.list

fn main():
    let z: list[list[int]] = list.zip([1, 2], [3, 4])
    print(z)
"#,
        "[[1, 3], [2, 4]]\n",
    );
}

// ===========================================================================
// 2 — list.min / list.max
// ===========================================================================

/// Gộp int/float/str, phần tử đơn, trùng nhau, số âm.
#[test]
fn min_max_hop_le() {
    dung(
        "minmax",
        r#"use std.list

fn main():
    print(list.min([3, 1, 2]))
    print(list.max([3, 1, 2]))
    print(list.min([-5, -2, -9]))
    print(list.max([-5, -2, -9]))
    print(list.min([7]))
    print(list.max([7]))
    print(list.min([4, 4, 4]))
    print(list.max([4, 4, 4]))
    print(list.min([2.5, 1.25, 9.0]))
    print(list.max([2.5, 1.25, 9.0]))
    print(list.min(["b", "a", "c"]))
    print(list.max(["b", "a", "c"]))
"#,
        "1\n3\n-9\n-2\n7\n7\n4\n4\n1.25\n9\na\nc\n",
    );
}

/// Kiểu trả là **kiểu phần tử** (`T`), không phải list; sai kiểu bị checker chặn.
#[test]
fn min_max_giu_kieu_phan_tu() {
    dung(
        "minmax-kieu",
        r#"use std.list

fn main():
    let n: int = list.min([3, 1, 2])
    let s: str = list.max(["b", "a"])
    let f: float = list.max([1.5, 2.5])
    print(n)
    print(s)
    print(f)
"#,
        "1\nb\n2.5\n",
    );
}

/// Danh sách rỗng: **trap sạch** `VS0501`, nêu tên hàm, không panic; hai engine byte-khớp.
#[test]
fn min_max_danh_sach_rong_la_trap() {
    trap_ca_hai(
        "min-rong",
        "use std.list\n\nfn main():\n    print(list.min([]))\n",
        "list.min",
    );
    trap_ca_hai(
        "max-rong-bien",
        "use std.list\n\nfn main():\n    let xs: list[int] = []\n    print(list.max(xs))\n",
        "list.max",
    );
}

/// Phần tử **không so được** (list lồng nhau) cũng trap `VS0501` như `list.sort`, không trả
/// kết quả nửa vời.
#[test]
fn min_khong_so_duoc_la_trap() {
    trap_ca_hai(
        "min-khong-so",
        "use std.list\n\nfn main():\n    print(list.min([[1], [2]]))\n",
        "list.min",
    );
}

/// `NaN` — kể cả danh sách **một** phần tử — phải trap (không lặng lẽ trả `NaN`), như `sort`.
#[test]
fn min_max_nan_la_trap() {
    trap_ca_hai(
        "min-nan",
        "use std.list\n\nfn main():\n    let xs: list[float] = [1.0, 0.0 / 0.0]\n    print(list.min(xs))\n",
        "list.min",
    );
    trap_ca_hai(
        "max-nan-mot",
        "use std.list\n\nfn main():\n    let xs: list[float] = [0.0 / 0.0]\n    print(list.max(xs))\n",
        "list.max",
    );
}

// ===========================================================================
// 3 — str.lines / str.split_whitespace
// ===========================================================================

/// `lines`: `\r\n`/`\n`, dấu cuối dòng, dòng rỗng giữa, chuỗi rỗng.
#[test]
fn lines_hop_le_bien() {
    dung(
        "lines",
        r#"use std.str

fn main():
    let a = str.lines("")
    let b = str.lines("\n")
    print(a)
    print(a.len())
    print(b)
    print(b.len())
    print(str.lines("a"))
    print(str.lines("a\n"))
    print(str.lines("a\r\nb"))
    print(str.lines("a\r\nb\n"))
    print(str.lines("a\n\n"))
"#,
        "[]\n0\n[]\n1\n[a]\n[a]\n[a, b]\n[a, b]\n[a, ]\n",
    );
}

/// `split_whitespace`: bỏ đầu/cuối, gộp nhiều khoảng trắng (space/tab/CR/LF), chuỗi rỗng.
#[test]
fn split_whitespace_hop_le_bien() {
    dung(
        "ws",
        r#"use std.str

fn main():
    print(str.split_whitespace(""))
    print(str.split_whitespace("   "))
    print(str.split_whitespace("a b"))
    print(str.split_whitespace("  a\tb\n c  "))
    print(str.split_whitespace("a\r\nb"))
"#,
        "[]\n[]\n[a, b]\n[a, b, c]\n[a, b]\n",
    );
}

/// Sugar R5c: `s.lines()` / `s.split_whitespace()` ≡ `str.*(s)` — cả checker lẫn hai engine.
#[test]
fn sugar_phuong_thuc_str() {
    dung(
        "sugar-str",
        r#"use std.str

fn main():
    print("a\r\nb\n".lines())
    print("  x  y ".split_whitespace())
    print("ab".repeat(3))
"#,
        "[a, b]\n[x, y]\nababab\n",
    );
}

// ===========================================================================
// 4 — ca âm tĩnh
// ===========================================================================

/// `zip` hai kiểu phần tử khác nhau ⇒ `VS0412` (một biến kiểu `T` cho cả hai vế).
#[test]
fn zip_lech_kieu_bi_tu_choi() {
    tu_choi(
        "zip-lech",
        "use std.list\n\nfn main():\n    print(list.zip([1, 2], [\"a\"]))\n",
        "VS0412",
    );
}

/// `zip` sai số đối số ⇒ `VS0401`.
#[test]
fn zip_sai_so_doi_bi_tu_choi() {
    tu_choi(
        "zip-arity",
        "use std.list\n\nfn main():\n    print(list.zip([1]))\n",
        "VS0401",
    );
}

/// `lines` nhận `int` ⇒ `VS0412`.
#[test]
fn lines_sai_kieu_bi_tu_choi() {
    tu_choi(
        "lines-kieu",
        "use std.str\n\nfn main():\n    print(str.lines(1))\n",
        "VS0412",
    );
}

/// `min` trả kiểu phần tử: gán cho `str` khi phần tử là `int` ⇒ `VS0412`.
#[test]
fn min_ket_qua_sai_kieu_bi_tu_choi() {
    tu_choi(
        "min-ketqua",
        "use std.list\n\nfn main():\n    let s: str = list.min([3, 1])\n    print(s)\n",
        "VS0412",
    );
}

/// `min` nhận không phải list ⇒ `VS0412`.
#[test]
fn min_sai_kieu_doi_so_bi_tu_choi() {
    tu_choi(
        "min-doi",
        "use std.list\n\nfn main():\n    print(list.min(\"abc\"))\n",
        "VS0412",
    );
}
