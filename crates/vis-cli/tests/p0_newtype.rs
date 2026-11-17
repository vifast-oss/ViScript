// SPDX-License-Identifier: MIT OR Apache-2.0
//! **P0 (VD-170) + R12a (VD-178)** — `newtype`/`alias` là từ khoá **đã bỏ** ⇒ `VS0203` ngay dòng
//! khai báo; cú pháp mới `type X = T` / `type Email(str)` xanh.
//!
//! Đỏ-trước (P0): trước P0, `newtype Met = float` một mình ⇒ `vis check` exit 0 (im lặng).
//! **R12a**: parser chặn `alias`/`newtype` ⇒ `VS0203` + gợi ý; `type Met(float)` thay thế.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-p0-newtype-{tag}-{}", std::process::id()));
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

#[test]
fn newtype_tu_khoa_da_bo() {
    let d = dir("decl");
    let f = write(&d, "a.vis", "newtype Met = float\n");
    let o = run(&["check", &f]);
    let out = stdout(&o);
    assert_eq!(
        o.status.code(),
        Some(1),
        "phải đỏ vì `newtype` đã bỏ: {out}"
    );
    assert!(out.contains("not ok"), "phải có dòng not ok: {out}");
    assert!(
        out.contains("VS0203"),
        "phải là VS0203 (từ khoá đã bỏ): {out}"
    );
    assert!(
        out.contains("type Email(str)"),
        "gợi ý phải nêu cú pháp mới: {out}"
    );
    assert!(
        out.contains("1:1"),
        "phải báo ở **dòng khai báo** (1:1): {out}"
    );
}

#[test]
fn type_newtype_cu_phap_moi_xanh() {
    // Cổng doc-examples (QD §7.4) chạy `vis check --syntax` — cú pháp mới phải xanh.
    let d = dir("syntax");
    let f = write(&d, "a.vis", "type Met(float)\n");
    let o = run(&["check", "--syntax", &f]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "`--syntax` phải xanh (cổng tài liệu): {}",
        stdout(&o)
    );
}

#[test]
fn alias_tu_khoa_da_bo() {
    let d = dir("alias");
    let f = write(
        &d,
        "a.vis",
        "alias Met = float\nlet x: Met = 1.5\nprint(x)\n",
    );
    let o = run(&["check", &f]);
    let out = stdout(&o);
    assert_eq!(o.status.code(), Some(1), "`alias` đã bỏ ⇒ đỏ: {out}");
    assert!(out.contains("VS0203"), "phải là VS0203: {out}");
    assert!(
        out.contains("type X = T"),
        "gợi ý phải nêu `type X = T`: {out}"
    );
}
