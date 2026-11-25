// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-314a hậu-kiểm (low-peak 2026-11-25)** — meta-test cho định dạng `.trap` của
//! `vis-conformance`: chứng minh harness **thật sự kiểm** mã lỗi lúc chạy, không bỏ qua.
//!
//! Positive control + hai negative control (sai mã, thiếu mã) để `.trap` không thể "xanh giả".

use std::path::PathBuf;

use vis_conformance::{khang_dinh, Case};

fn ca(ma_trap: &[&str], exit: Option<i32>) -> Case {
    Case {
        ten: "meta-trap".to_string(),
        duong_dan: PathBuf::from("meta-trap.vis"),
        src: "fn main():\n    print(1 % 0)\n".to_string(),
        stdout: String::new(),
        ma_loi: Vec::new(),
        ma_trap: ma_trap.iter().map(|s| s.to_string()).collect(),
        exit: exit.unwrap_or(0),
        loi_tinh: false,
        la_trap: true,
        co_exit: exit.is_some(),
        kiem_fmt: false,
        out_prefix: None,
        bo_qua_windows: false,
    }
}

#[test]
fn trap_dung_ma_va_exit_khop() {
    // Mã thật của `1 % 0` là `VS0502`; exit in-process của trap là 1.
    khang_dinh(&ca(&["VS0502"], None));
    khang_dinh(&ca(&["VS0502"], Some(1)));
}

#[test]
#[should_panic(expected = "mã chẩn đoán sai")]
fn trap_sai_ma_bi_bat() {
    khang_dinh(&ca(&["VS0503"], None));
}

#[test]
#[should_panic(expected = "mã chẩn đoán sai")]
fn trap_khong_khai_ma_bi_bat() {
    // `.trap` rỗng mà chương trình trap ⇒ không được coi là "không có chẩn đoán".
    khang_dinh(&ca(&[], None));
}

#[test]
#[should_panic(expected = "exit sai")]
fn trap_exit_0_bi_bat() {
    khang_dinh(&ca(&["VS0502"], Some(0)));
}
