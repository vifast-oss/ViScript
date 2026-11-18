// SPDX-License-Identifier: MIT OR Apache-2.0
//! **R2a (VD-178)** — `none` là **từ khoá đã bỏ** ⇒ `VS0203` + gợi ý `None`; `vis fmt` tự sửa
//! `none` → `None`; `None` là **biến thể** của `Option[T]` (dựng được, tyck được, so sánh được);
//! `Some(1)` vẫn chạy, `some(1)` vẫn `VS0501` lúc chạy.
//!
//! **Đỏ-trước**: trước R2a, `none` còn là **từ khoá** nên `let x: int? = none` **xanh** (không có
//! `VS0203`), `vis fmt` **không** sửa `none` → `None`, và pattern `none` cũng **xanh**.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-r2a-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("phải tạo được thư mục tạm");
    d
}

fn write(d: &Path, name: &str, src: &str) -> String {
    let p = d.join(name);
    std::fs::write(&p, src).expect("phải ghi được tệp tạm");
    p.to_string_lossy().into_owned()
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("phải chạy được binary `vis`")
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// `none` dùng như **giá trị** ⇒ `VS0203` ("từ khoá đã bỏ") + gợi ý `None`, báo **đúng chỗ dùng**.
#[test]
fn none_gia_tri_la_vs0203_va_goi_y_none() {
    let d = dir("gia-tri");
    let f = write(&d, "a.vis", "let x: int? = none\nprint(x ?? 0)\n");
    let o = run(&["check", &f]);
    let out = stdout(&o);
    assert_eq!(
        o.status.code(),
        Some(1),
        "`none` đã bỏ ⇒ `vis check` phải đỏ: {out}"
    );
    assert!(out.contains("VS0203"), "phải là VS0203: {out}");
    assert!(out.contains("None"), "gợi ý phải nêu `None`: {out}");
    assert!(
        out.contains("1:15"),
        "phải báo đúng chỗ dùng `none` (1:15): {out}"
    );
}

/// `none` dùng làm **pattern** ⇒ cũng `VS0203` + gợi ý `None`.
#[test]
fn none_pattern_la_vs0203() {
    let d = dir("pattern");
    let f = write(
        &d,
        "a.vis",
        "let x: int? = None\nmatch x:\n    Some(v): print(v)\n    none: print(0)\n",
    );
    let o = run(&["check", &f]);
    let out = stdout(&o);
    assert_eq!(
        o.status.code(),
        Some(1),
        "pattern `none` đã bỏ ⇒ phải đỏ: {out}"
    );
    assert!(out.contains("VS0203"), "phải là VS0203: {out}");
    assert!(out.contains("None"), "gợi ý phải nêu `None`: {out}");
}

/// `vis fmt` **tự sửa** `none` → `None` (cả giá trị lẫn pattern).
#[test]
fn fmt_tu_sua_none_thanh_none() {
    let d = dir("fmt");
    let f = write(
        &d,
        "a.vis",
        "let x: int? = none\nmatch x:\n    Some(v): print(v)\n    none: print(0)\n",
    );
    let o = run(&["fmt", &f]);
    let out = stdout(&o);
    assert_eq!(o.status.code(), Some(0), "fmt phải chạy: {out}");
    assert!(
        out.contains("let x: int? = None"),
        "fmt phải sửa `none` → `None`: {out}"
    );
    assert!(
        out.contains("    None: print(0)"),
        "fmt phải sửa pattern `none` → `None`: {out}"
    );
    assert!(!out.contains("none"), "không còn `none` sau fmt: {out}");
}

/// `None` là **biến thể** của `Option[T]`: dựng được, tyck được, so sánh được, `??` được.
#[test]
fn none_la_bien_the_option() {
    let d = dir("bien-the");
    let f = write(
        &d,
        "a.vis",
        "let a: int? = None\nlet b: int? = None\nprint(a == b)\nprint(a ?? 7)\nfn f() -> int?:\n    None\nprint(f() ?? 9)\n",
    );
    let o = run(&["run", &f]);
    let out = stdout(&o);
    assert_eq!(
        o.status.code(),
        Some(0),
        "`None` phải chạy được: {out} {}",
        stderr(&o)
    );
    assert_eq!(out, "true\n7\n9\n", "kết quả `None` sai: {out}");
}

/// `None` làm **pattern** trong `match` vẫn khớp (biến thể thật, không phải tên lạ).
#[test]
fn none_pattern_hop_le() {
    let d = dir("pattern-xanh");
    let f = write(
        &d,
        "a.vis",
        "let x: int? = None\nmatch x:\n    Some(v): print(v)\n    None: print(0)\n",
    );
    let o = run(&["run", &f]);
    let out = stdout(&o);
    assert_eq!(o.status.code(), Some(0), "phải chạy: {out} {}", stderr(&o));
    assert_eq!(out, "0\n", "pattern `None` phải khớp: {out}");
}

/// `Some(1)` vẫn chạy (không bị R2a làm hỏng).
#[test]
fn some_van_chay() {
    let d = dir("some");
    let f = write(&d, "a.vis", "let y = Some(1)\nprint(y ?? 0)\n");
    let o = run(&["run", &f]);
    let out = stdout(&o);
    assert_eq!(
        o.status.code(),
        Some(0),
        "`Some(1)` phải chạy: {out} {}",
        stderr(&o)
    );
    assert_eq!(out, "1\n", "`Some(1)` sai: {out}");
}

/// `some(1)` viết thường vẫn **`VS0501`** lúc chạy (R1a chỉ cảnh báo `VS0111`, không làm nó chạy).
#[test]
fn some_thuong_van_vs0501() {
    let d = dir("some-thuong");
    let f = write(&d, "a.vis", "let y = some(1)\nprint(y)\n");
    let o = run(&["run", &f]);
    let err = stderr(&o);
    assert_eq!(
        o.status.code(),
        Some(101),
        "`some(1)` ⇒ VS0501 ⇒ exit 101 (C1): {err}"
    );
    assert!(err.contains("VS0501"), "phải là VS0501: {err}");
}
