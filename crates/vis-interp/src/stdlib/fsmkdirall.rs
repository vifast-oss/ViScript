// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (mkdir-all)** — `std.fs.mkdir` / `std.fs.mkdir_all`.
//!
//! Tách khỏi `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng **cùng** hàm này.
//!
//! - `fs.mkdir`: **một** cấp (giữ nguyên ngữ nghĩa VD-266; đã tồn tại ⇒ `Err`).
//! - `fs.mkdir_all`: tạo **đệ quy** mọi thư mục thiếu; đích đã là thư mục ⇒ `Ok(())`
//!   (**idempotent**); thành phần là **tệp**/`symlink` ⇒ `Err(Error)` **dữ liệu**, không trap.
//!   Việc đi đường và kiểm symlink do **host** quyết định (host thật dùng `symlink_metadata`;
//!   `FsGuard` kiểm `cap[Dir] (write)` trước khi chạm đĩa).

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::R;
use crate::value::Value;

/// Dispatch hai hàm tạo thư mục (interp ↔ VM dùng chung).
pub(super) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi(ten, &args[0], 0, span)?;
    let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
    match ten {
        "fs.mkdir" => super::hop_nhat_fs(host.mkdir(p).map(|()| Value::Unit), span),
        "fs.mkdir_all" => super::hop_nhat_fs(host.mkdir_all(p).map(|()| Value::Unit), span),
        _ => Err(super::loi(
            format!("hàm tạo thư mục không xác định: `{ten}`"),
            span,
        )),
    }
}
