// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Bảng chữ ký `std.*`** (ST-26, VD-43) — đã tách thành các module con (ST-296c) nhưng
//! **giữ nguyên** toàn bộ API công khai và nội dung bảng.

mod api;
mod bang;
mod builtin;
mod dinh_nghia;
mod funcs_fs_path;
mod funcs_list_map;
mod funcs_misc;
mod funcs_net_http;
mod funcs_sqlite;
mod funcs_str;
mod std_id;
mod ty;

pub use api::{
    funcs_cua, hoan, la_std, module_ho_tro, module_sugar, sig_builtin, sig_of, HA_CHUA_HO_TRO,
};
pub use bang::FUNCS;
pub use builtin::BUILTIN_FUNCS;
pub use dinh_nghia::{StdFn, MODULES, MODULES_HOAN};
pub use std_id::StdId;
pub use ty::{ten_bien, Ty};
