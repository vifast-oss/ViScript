// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — đệm chuỗi `std.str.pad_left` / `pad_right` / `center` theo **ký tự Unicode**
//! (scalar), không theo byte.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `width` = số **scalar** (`s.chars().count()`); `"đ"` và `"😀"` đều là **một** ký tự.
//! - `width <= 0` hoặc `width <= len(s)` ⇒ trả `s` **nguyên vẹn**.
//! - `fill` phải **khác rỗng** — kiểm **trước** cả khi không cần đệm; rỗng ⇒ trap `VS0501`.
//! - phần đệm = `fill` **lặp rồi cắt** đúng số scalar cần (fill nhiều ký tự không bị cắt thô).
//! - `center` chia đôi phần đệm; scalar **dư nằm bên phải**; mỗi phía **lặp `fill` từ đầu**.
//! - trần phần đệm 10^7 scalar (cùng mức `str.repeat`) ⇒ `width` khổng lồ không OOM; vượt trần
//!   là trap `VS0501`, **không** panic.

use vis_syntax::Span;

use super::loi;
use crate::interp::Abort;

/// Trần số scalar **thêm vào** (không tính `s`) — như `str.repeat` (10^7).
pub(super) const TRAN_DEM: i64 = 10_000_000;

/// Đệm `s` lên `width` scalar bằng `fill`; `name` chọn phía (`str.pad_left` / `pad_right` /
/// `center`). Trả `s` khi `width <= 0` hoặc `width <= len(s)`.
pub(super) fn dem(
    name: &str,
    s: &str,
    width: i64,
    fill: &str,
    span: Span,
) -> Result<String, Abort> {
    if fill.is_empty() {
        return Err(loi(format!("`{name}`: `fill` phải khác rỗng"), span));
    }
    let dai = s.chars().count() as i64;
    if width <= 0 || width <= dai {
        return Ok(s.to_string());
    }
    let can = width - dai;
    if can > TRAN_DEM {
        return Err(loi(
            format!("`{name}`: phần đệm quá lớn ({can} ký tự > 10^7)"),
            span,
        ));
    }
    let can = can as usize;
    let (trai, phai) = match name {
        "str.pad_left" => (can, 0),
        "str.pad_right" => (0, can),
        _ => (can / 2, can - can / 2),
    };
    let mut ra = String::new();
    ra.extend(fill.chars().cycle().take(trai));
    ra.push_str(s);
    ra.extend(fill.chars().cycle().take(phai));
    Ok(ra)
}
