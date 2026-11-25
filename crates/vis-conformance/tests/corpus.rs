// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313c/ST-313d (VD-289)** — chạy toàn bộ corpus `test/conformance/**` trên cả hai engine.
//!
//! **ST-314a hậu-kiểm (low-peak 2026-11-25)**: chạy trong luồng **256 MiB** như `vis run` — ca
//! `.trap` đệ quy sâu (`VS0501` quá 2.000 khung) cần đủ stack để **chạm ngưỡng có kiểm soát**
//! thay vì tràn stack của luồng test mặc định (2 MiB) và giết cả binary.

use std::panic::{catch_unwind, AssertUnwindSafe};

use vis_conformance::{goc_repo, khang_dinh, nap_corpus};

#[test]
fn conformance_toan_bo_corpus() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(chay_toan_bo)
        .expect("tạo luồng")
        .join()
        .expect("luồng corpus lỗi");
}

fn chay_toan_bo() {
    let goc = goc_repo().join("test/conformance");
    let cases = nap_corpus(&goc);
    assert!(!cases.is_empty(), "corpus rỗng — thiếu {}", goc.display());
    let mut hong = Vec::new();
    let mut bo_qua = 0usize;
    for c in &cases {
        if c.bo_qua_windows && cfg!(windows) {
            bo_qua += 1;
            continue;
        }
        if catch_unwind(AssertUnwindSafe(|| khang_dinh(c))).is_err() {
            hong.push(c.ten.clone());
        }
    }
    eprintln!(
        "conformance: {}/{} ca xanh{}",
        cases.len() - hong.len() - bo_qua,
        cases.len() - bo_qua,
        if bo_qua > 0 {
            format!(" (bỏ qua {bo_qua} ca POSIX-only trên Windows)")
        } else {
            String::new()
        }
    );
    assert!(
        hong.is_empty(),
        "{} ca đỏ:\n{}",
        hong.len(),
        hong.join("\n")
    );
}
