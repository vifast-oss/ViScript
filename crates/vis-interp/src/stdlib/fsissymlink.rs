// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-is-symlink)** — `std.fs.is_symlink(p: str) -> Result[bool, Error]`.
//!
//! `true` ⇔ **chính** `p` là **symbolic link**, dùng `symlink_metadata` nên **không** đi theo
//! link. Hợp đồng **trùng khít** `fs.is_file`/`fs.is_dir`:
//!
//! | `p` | `is_symlink` |
//! |---|---|
//! | symlink → tệp, symlink → thư mục | `Ok(true)` |
//! | symlink → đích **thiếu** (dangling) | `Ok(true)` — khác `exists` (theo link ⇒ `false`) |
//! | tệp thường / thư mục | `Ok(false)` |
//! | **thiếu** (không có gì ở `p`) | `Ok(false)` |
//! | lỗi metadata khác (`ENOTDIR`, không đủ quyền OS, …) | `Err(Error)` dữ liệu, **không** trap |
//!
//! Tách khỏi `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng **cùng** hàm này.
//! Quyền (`cap[Dir]`/`cap[File]` chỉ **đọc**, ngoài root ⇒ `VS0604`) và cách đi theo symlink khi
//! kiểm quyền do `FsGuard`/host quyết định — tầng này chỉ chuyển lỗi host thành `Err(Error)`.

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::R;
use crate::value::Value;

/// Dispatch `std.fs.is_symlink`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
/// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
pub(super) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi(ten, &args[0], 0, span)?;
    let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
    super::hop_nhat_fs(host.is_symlink(p).map(Value::Bool), span)
}
