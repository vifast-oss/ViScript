// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Async: frame/reactor/select/chan/budget/lat*.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "async_it/async_10k.rs"]
mod async_10k;
#[path = "async_it/async_vis.rs"]
mod async_vis;
#[path = "async_it/closure_cmd.rs"]
mod closure_cmd;
#[path = "async_it/get_select.rs"]
mod get_select;
#[path = "async_it/kenh.rs"]
mod kenh;
#[path = "async_it/kenh_cap0.rs"]
mod kenh_cap0;
#[path = "async_it/khe_ong.rs"]
mod khe_ong;
#[path = "async_it/lat1_frame.rs"]
mod lat1_frame;
#[path = "async_it/lat255_coverage.rs"]
mod lat255_coverage;
#[path = "async_it/lat256_budget.rs"]
mod lat256_budget;
#[path = "async_it/lat256_net.rs"]
mod lat256_net;
#[path = "async_it/lat257_a_coverage.rs"]
mod lat257_a_coverage;
#[path = "async_it/lat257_b_budget.rs"]
mod lat257_b_budget;
#[path = "async_it/lat257_b_closure_await.rs"]
mod lat257_b_closure_await;
#[path = "async_it/lat257_b_vm_steps.rs"]
mod lat257_b_vm_steps;
#[path = "async_it/lat257_c_lifecycle.rs"]
mod lat257_c_lifecycle;
#[path = "async_it/lat257_coverage_extra.rs"]
mod lat257_coverage_extra;
#[path = "async_it/lat257_fairness.rs"]
mod lat257_fairness;
#[path = "async_it/lat257_trace_audit.rs"]
mod lat257_trace_audit;
#[path = "async_it/lat258_frame_fixes.rs"]
mod lat258_frame_fixes;
#[path = "async_it/lat2_reactor.rs"]
mod lat2_reactor;
#[path = "async_it/lat3_real.rs"]
mod lat3_real;
#[path = "async_it/test_timeout.rs"]
mod test_timeout;
#[path = "async_it/vd259_frame2.rs"]
mod vd259_frame2;
#[path = "async_it/vd259_semantics.rs"]
mod vd259_semantics;
#[path = "async_it/vd270_frame_add_assign.rs"]
mod vd270_frame_add_assign;
#[path = "async_it/vd270_frame_nested.rs"]
mod vd270_frame_nested;
#[path = "async_it/vd270_recv_opt_infer.rs"]
mod vd270_recv_opt_infer;
#[path = "async_it/vs3_budget.rs"]
mod vs3_budget;
#[path = "async_it/vs3_budget_default.rs"]
mod vs3_budget_default;
#[path = "async_it/vs3_cancel.rs"]
mod vs3_cancel;
#[path = "async_it/vs3_chan0.rs"]
mod vs3_chan0;
#[path = "async_it/vs3_chan_cap.rs"]
mod vs3_chan_cap;
#[path = "async_it/vs3_chan_timeout.rs"]
mod vs3_chan_timeout;
#[path = "async_it/vs3_select.rs"]
mod vs3_select;
