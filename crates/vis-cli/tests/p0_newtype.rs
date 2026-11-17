// SPDX-License-Identifier: MIT OR Apache-2.0
//! **P0 (VD-170)** — `newtype` chưa hạ ⇒ **báo lỗi ngay dòng khai báo**, không im lặng.
//!
//! Đỏ-trước: trước P0, `newtype Met = float` một mình ⇒ `vis check` exit 0 (im lặng);
//! dùng `Met` mới báo `VS0301` ở **dòng dùng**, không phải dòng khai.
//!
//! Cổng doc-examples chạy `vis check --syntax` (chỉ parse/hạ) ⇒ khối tài liệu
//! `newtype Email = str` vẫn xanh; lỗi P0 chỉ ở tầng **kiểm kiểu**.

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
fn newtype_bao_loi_ngay_dong_khai_bao() {
    let d = dir("decl");
    let f = write(&d, "a.vis", "newtype Met = float\n");
    let o = run(&["check", &f]);
    let out = stdout(&o);
    assert_eq!(
        o.status.code(),
        Some(1),
        "phải đỏ vì newtype chưa hỗ trợ: {out}"
    );
    assert!(out.contains("not ok"), "phải có dòng not ok: {out}");
    assert!(
        out.contains("newtype"),
        "thông điệp phải nêu `newtype`: {out}"
    );
    assert!(
        out.contains("1:1"),
        "phải báo ở **dòng khai báo** (1:1), không phải dòng dùng: {out}"
    );
    assert!(
        !out.contains("VS0 "),
        "thông điệp không được chứa nhãn `VS0` (P0 mục 2): {out}"
    );
}

#[test]
fn newtype_khong_lam_hong_cong_doc_syntax() {
    // Cổng doc-examples (QD §7.4) chạy `vis check --syntax` — chỉ parse/hạ, không kiểm kiểu.
    // Lỗi P0 ở tầng tyck nên khối tài liệu `newtype Email = str` vẫn xanh.
    let d = dir("syntax");
    let f = write(&d, "a.vis", "newtype Met = float\n");
    let o = run(&["check", "--syntax", &f]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "`--syntax` phải xanh (cổng tài liệu): {}",
        stdout(&o)
    );
}

#[test]
fn alias_van_xanh() {
    let d = dir("alias");
    let f = write(
        &d,
        "a.vis",
        "alias Met = float\nlet x: Met = 1.5\nprint(x)\n",
    );
    let o = run(&["check", &f]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "`alias` vẫn hợp lệ tới R12: {}",
        stdout(&o)
    );
}
