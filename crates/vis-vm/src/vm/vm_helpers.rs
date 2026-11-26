// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-257**: helper giá trị của VM.
//!
//! **ST-316a (VD-292 §4.1)**: phần thao tác `Value` đã dời về **một nguồn**
//! [`vis_runtime::value_ops`] dùng chung với `vis-interp`; tệp này chỉ còn `lit_kind` và
//! lớp chuyển tên mỏng để `exec.rs` giữ cách gọi cũ.

use vis_hir::hir::LitKind;

/// `kind` số của const pool → `LitKind`.
pub(super) fn lit_kind(k: u8) -> Option<LitKind> {
    Some(match k {
        0 => LitKind::Int,
        1 => LitKind::Float,
        2 => LitKind::Size,
        3 => LitKind::Duration,
        4 => LitKind::Percent,
        5 => LitKind::Str,
        6 => LitKind::Bytes,
        7 => LitKind::Path,
        8 => LitKind::Char,
        9 => LitKind::Bool,
        _ => return None,
    })
}

pub(super) use vis_runtime::value_ops::{
    chi_so as index, chi_so_opt as index_opt, dat as set, truong as field, voi as with,
};
