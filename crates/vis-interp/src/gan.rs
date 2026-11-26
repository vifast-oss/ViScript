// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-56b (VD-97)** gán trường/chỉ số, **VD-268** gán đường.
//!
//! **ST-316a (VD-292 §4.1)**: toàn bộ ngữ nghĩa đã chuyển về **một nguồn**
//! [`vis_runtime::value_ops`]; tệp này chỉ còn lớp mỏng để `vis-interp` giữ API cũ.
//! Nhờ đó chuỗi `VS0501`/`VS0503` không còn hai bản phải sửa tay.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

pub(crate) use vis_runtime::value_ops::Buoc;

/// `x.f = v` — sửa **tại chỗ** (không chép cả bản ghi); thiếu trường ⇒ `VS0501`.
pub(crate) fn gan_truong(base: &mut Value, name: &str, v: Value, span: Span) -> R<()> {
    vis_runtime::value_ops::dat_truong(base, name, v, span)
}

/// `x[i] = v` — sửa **tại chỗ**; list ngoài phạm vi ⇒ `VS0501`, map last-wins.
pub(crate) fn gan_chi_so(base: &mut Value, idx: Value, v: Value, span: Span) -> R<()> {
    vis_runtime::value_ops::dat(base, &idx, v, span)
}

/// Đọc trường — cùng nguồn với VM.
pub(crate) fn doc_truong(b: Value, name: &str, span: Span) -> R<Value> {
    vis_runtime::value_ops::doc_truong(b, name, span)
}

/// Đọc chỉ số — cùng nguồn với VM.
pub(crate) fn doc_chi_so(b: Value, i: Value, span: Span) -> R<Value> {
    vis_runtime::value_ops::chi_so(&b, &i, span)
}

/// Đọc lá của đường gán.
pub(crate) fn doc_duong(base: &Value, duong: &[Buoc], span: Span) -> R<Value> {
    vis_runtime::value_ops::doc_duong(base, duong, span)
}

/// Ghi lá rồi dựng lại từng tầng (tầng đầu sửa tại chỗ khi có thể).
pub(crate) fn gan_duong(base: &mut Value, duong: &[Buoc], v: Value, span: Span) -> R<()> {
    vis_runtime::value_ops::gan_duong(base, duong, v, span)
}
