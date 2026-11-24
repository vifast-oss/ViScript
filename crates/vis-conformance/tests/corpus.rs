// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313c/ST-313d (VD-289)** — chạy toàn bộ corpus `test/conformance/**` trên cả hai engine.

use std::panic::{catch_unwind, AssertUnwindSafe};

use vis_conformance::{goc_repo, khang_dinh, nap_corpus};

#[test]
fn conformance_toan_bo_corpus() {
    let goc = goc_repo().join("test/conformance");
    let cases = nap_corpus(&goc);
    assert!(!cases.is_empty(), "corpus rỗng — thiếu {}", goc.display());
    let mut hong = Vec::new();
    for c in &cases {
        if catch_unwind(AssertUnwindSafe(|| khang_dinh(c))).is_err() {
            hong.push(c.ten.clone());
        }
    }
    eprintln!(
        "conformance: {}/{} ca xanh",
        cases.len() - hong.len(),
        cases.len()
    );
    assert!(
        hong.is_empty(),
        "{} ca đỏ:\n{}",
        hong.len(),
        hong.join("\n")
    );
}
