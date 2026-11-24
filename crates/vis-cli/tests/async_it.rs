// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Async: frame/reactor/select/chan/budget/lat*.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

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
#[path = "async_it/test_timeout.rs"]
mod test_timeout;
#[path = "async_it/vd259_semantics.rs"]
mod vd259_semantics;
#[path = "async_it/vd270_frame_nested.rs"]
mod vd270_frame_nested;
#[path = "async_it/vd270_recv_opt_infer.rs"]
mod vd270_recv_opt_infer;
#[path = "async_it/vs3_budget_default.rs"]
mod vs3_budget_default;
#[path = "async_it/vs3_chan_cap.rs"]
mod vs3_chan_cap;
