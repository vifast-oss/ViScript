// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-read-link)** — `std.fs.read_link(p: str) -> Result[str, Error]`.
//!
//! Đọc **đích** của symbolic link tại `p` bằng `std::fs::read_link` — **không** đi theo link.
//! Hợp đồng:
//!
//! | `p` | `read_link` |
//! |---|---|
//! | symlink → tệp/thư mục, đích **tương đối** | `Ok(target)` **nguyên văn** (`"tep.txt"`) |
//! | symlink → đích **tuyệt đối** | `Ok(target)` đúng chuỗi lúc tạo |
//! | symlink → đích **thiếu** (dangling) | `Ok(target)` — khác `exists`/`fs.read` |
//! | tệp thường / thư mục thường / `p` **thiếu** | `Err(Error)` dữ liệu, **không** trap |
//! | lỗi khác (`ENOTDIR`, không đủ quyền OS, đích không phải UTF-8) | `Err(Error)` dữ liệu |
//!
//! Tách khỏi `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng **cùng** hàm này.
//! Quyền (`cap[Dir]`/`cap[File]` chỉ **đọc**, ngoài root ⇒ `VS0604`) và cách canonicalize khi
//! kiểm quyền do `FsGuard`/host quyết định — tầng này chỉ chuyển lỗi host thành `Err(Error)`.

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::R;
use crate::value::Value;

/// Dispatch `std.fs.read_link`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
/// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
pub(super) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi(ten, &args[0], 0, span)?;
    let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
    super::hop_nhat_fs(host.read_link(p).map(Value::Str), span)
}
