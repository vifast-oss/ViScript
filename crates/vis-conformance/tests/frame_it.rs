// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-313c (VD-289)** — test API thư viện (parity/frame) chuyển từ `vis-cli/tests/lang_it`.
//!
//! Các ca chỉ dùng API `vis_interp`/`vis_vm`/`vis_runtime`; không CLI, không binary.

#[path = "frame_it/vd260_frame3.rs"]
mod vd260_frame3;
#[path = "frame_it/vd265_frame.rs"]
mod vd265_frame;
#[path = "frame_it/vd284_async_impl.rs"]
mod vd284_async_impl;
#[path = "frame_it/vd284_spawn_method.rs"]
mod vd284_spawn_method;
