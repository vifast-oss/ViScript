// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Bảng chữ ký `std.*`** (ST-26, VD-43) — **một nguồn** từ ST-317c: `registry`.
//!
//! `StdId` + tên + chữ ký + metadata module sống trong [`registry`]; các tệp còn lại chỉ là API
//! tra cứu (`api`), danh sách module hỗ trợ/hoãn (`dinh_nghia`), kiểu ([`ty`]) và builtin trần
//! (`builtin`). Giữ nguyên toàn bộ API công khai trước ST-317c.

mod api;
mod builtin;
mod dinh_nghia;
mod registry;
mod ty;

pub use api::{
    funcs_cua, hoan, la_std, module_ho_tro, module_sugar, sig_builtin, sig_of, HA_CHUA_HO_TRO,
};
pub use builtin::BUILTIN_FUNCS;
pub use dinh_nghia::StdFn;
pub use registry::{StdId, FUNCS, MODULES, MODULES_HOAN};
pub use ty::{ten_bien, Ty};
