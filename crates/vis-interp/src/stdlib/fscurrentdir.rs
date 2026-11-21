// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-current-dir)** — `std.fs.current_dir() -> Result[str, Error]`.
//!
//! Trả **thư mục làm việc hiện tại** dưới dạng **đường dẫn tuyệt đối** chuỗi UTF-8 — đúng
//! `std::env::current_dir`. Hợp đồng:
//!
//! | tình huống | `current_dir` |
//! |---|---|
//! | cwd đọc được, đường dẫn UTF-8 | `Ok(đường_dẫn_tuyệt_đối)` (không có `/` cuối thừa) |
//! | cwd bị **xoá** | `Err(Error)` **dữ liệu**, **không** trap |
//! | cwd **không truy cập được** (lỗi OS) | `Err(Error)` dữ liệu |
//! | cwd **không phải UTF-8** | `Err(Error)` dữ liệu (không cắt cụt bằng lossy) |
//!
//! Hàm **không tham số** và **không** nhận đường dẫn người dùng ⇒ tầng này **không** đi qua
//! `FsGuard`/`cap[Dir]` và **không** có side effect; host thật chỉ đọc môi trường. Tách khỏi
//! `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng **cùng** hàm này nên
//! byte-identical.

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::R;
use crate::value::Value;

/// Dispatch `std.fs.current_dir`: gọi host (không có đối số để kiểm kiểu động), rồi chuyển
/// `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
pub(super) fn goi(host: &mut dyn Host, ten: &str, _args: &[Value], span: Span) -> R<Value> {
    let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
    super::hop_nhat_fs(host.current_dir().map(Value::Str), span)
}
