// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313c (VD-289)** — test API thư viện chuyển từ `vis-cli/tests` (frame/worker/async).
//!
//! Một binary gộp để giữ trần R8; mỗi tệp ca giữ nguyên nội dung/assertion.

#[path = "async_api_it/lat1_frame.rs"]
mod lat1_frame;
#[path = "async_api_it/lat255_coverage.rs"]
mod lat255_coverage;
#[path = "async_api_it/lat256_budget.rs"]
mod lat256_budget;
#[path = "async_api_it/lat256_net.rs"]
mod lat256_net;
#[path = "async_api_it/lat257_a_coverage.rs"]
mod lat257_a_coverage;
#[path = "async_api_it/lat257_b_budget.rs"]
mod lat257_b_budget;
#[path = "async_api_it/lat257_b_closure_await.rs"]
mod lat257_b_closure_await;
#[path = "async_api_it/lat257_b_vm_steps.rs"]
mod lat257_b_vm_steps;
#[path = "async_api_it/lat257_c_lifecycle.rs"]
mod lat257_c_lifecycle;
#[path = "async_api_it/lat257_coverage_extra.rs"]
mod lat257_coverage_extra;
#[path = "async_api_it/lat257_fairness.rs"]
mod lat257_fairness;
#[path = "async_api_it/lat257_trace_audit.rs"]
mod lat257_trace_audit;
#[path = "async_api_it/lat258_frame_fixes.rs"]
mod lat258_frame_fixes;
#[path = "async_api_it/lat2_reactor.rs"]
mod lat2_reactor;
#[path = "async_api_it/lat3_real.rs"]
mod lat3_real;
#[path = "async_api_it/vd259_frame2.rs"]
mod vd259_frame2;
#[path = "frame_it/vd260_frame3.rs"]
mod vd260_frame3;
#[path = "frame_it/vd265_frame.rs"]
mod vd265_frame;
#[path = "async_api_it/vd270_frame_add_assign.rs"]
mod vd270_frame_add_assign;
#[path = "async_api_it/vd270_frame_nested_lib.rs"]
mod vd270_frame_nested_lib;
#[path = "frame_it/vd284_async_impl.rs"]
mod vd284_async_impl;
#[path = "frame_it/vd284_spawn_method.rs"]
mod vd284_spawn_method;
#[path = "async_api_it/vs3_budget.rs"]
mod vs3_budget;
#[path = "async_api_it/vs3_budget_default_lib.rs"]
mod vs3_budget_default_lib;
#[path = "async_api_it/vs3_cancel.rs"]
mod vs3_cancel;
#[path = "async_api_it/vs3_chan0.rs"]
mod vs3_chan0;
#[path = "async_api_it/vs3_chan_cap_lib.rs"]
mod vs3_chan_cap_lib;
#[path = "async_api_it/vs3_chan_timeout.rs"]
mod vs3_chan_timeout;
#[path = "async_api_it/vs3_select.rs"]
mod vs3_select;
#[path = "worker_api_it/vs3_worker.rs"]
mod vs3_worker;
#[path = "worker_api_it/vs3_worker_eval_local.rs"]
mod vs3_worker_eval_local;
#[path = "worker_api_it/vs3_worker_eval_parallel.rs"]
mod vs3_worker_eval_parallel;
#[path = "worker_api_it/vs3_worker_eval_vm.rs"]
mod vs3_worker_eval_vm;
#[path = "worker_api_it/vs3_worker_frame_lib.rs"]
mod vs3_worker_frame_lib;
#[path = "worker_api_it/vs3_worker_frame_vm.rs"]
mod vs3_worker_frame_vm;

#[path = "misc_api_it/api_dong_bang.rs"]
mod api_dong_bang;
#[path = "misc_api_it/so_ma.rs"]
mod so_ma;
#[path = "misc_api_it/vs3_coverage_matrix.rs"]
mod vs3_coverage_matrix;
