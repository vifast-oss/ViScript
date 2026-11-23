// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311c (VD-286)** — binary gộp cho bộ kiểm thử parity & hồi quy của `vis-vm`.
//!
//! `diff_100k.rs` (`bin/vis-diff`) và `fuzz_mutate.rs` (watchdog 2h) giữ binary riêng.

#[path = "vm_it/st238_parity.rs"]
mod st238_parity;
#[path = "vm_it/st239_parity.rs"]
mod st239_parity;
#[path = "vm_it/st245_async_regression.rs"]
mod st245_async_regression;
#[path = "vm_it/st247_global_domain.rs"]
mod st247_global_domain;
#[path = "vm_it/st248_field_lift.rs"]
mod st248_field_lift;
#[path = "vm_it/st250_global_init.rs"]
mod st250_global_init;
#[path = "vm_it/st252_capture_local.rs"]
mod st252_capture_local;
#[path = "vm_it/vm_neg.rs"]
mod vm_neg;
#[path = "vm_it/vs_interp.rs"]
mod vs_interp;
