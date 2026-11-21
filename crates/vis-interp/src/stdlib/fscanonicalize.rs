// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-canonicalize)** — `std.fs.canonicalize(p: str) -> Result[str, Error]`.
//!
//! Đường dẫn **tuyệt đối đã phân giải** của `p` bằng `std::fs::canonicalize`:
//!
//! | `p` | `canonicalize` |
//! |---|---|
//! | tệp/thư mục **tồn tại** (tương đối hay tuyệt đối) | `Ok(đường dẫn tuyệt đối đã chuẩn hoá)` |
//! | symlink (kể cả chuỗi link) | `Ok(đích **đã phân giải**)` — link được đi theo |
//! | `.`/`..` trong `p` | được phân giải theo **trạng thái đĩa**, không thuần từ vựng |
//! | `p` **rỗng**/thiếu (`ENOENT`, kể cả **dangling**) | `Err(Error)` dữ liệu, **không** trap |
//! | đích không UTF-8 / lỗi quyền OS / lỗi khác | `Err(Error)` dữ liệu |
//!
//! **Khác `path.normalize`**: hàm này **chạm đĩa** và đọc `cwd` để phân giải đường dẫn tương đối;
//! `path.normalize` chỉ sửa chuỗi (`a/../b` ⇒ `b` kể cả khi `b` không tồn tại, và không biết
//! symlink). Ở đây `p` phải **tồn tại** và kết quả phản ánh đích **thật** tại thời điểm gọi.
//!
//! Tách khỏi `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng **cùng** hàm này.
//! Quyền (`cap[Dir]`/`cap[File]` chỉ **đọc**, ngoài root ⇒ `VS0604`) và bước canonicalize khi
//! kiểm quyền do `FsGuard`/host quyết định — tầng này chỉ chuyển lỗi host thành `Err(Error)`.
//! Log `cwd`/symlink **không** được trả về ngoài đường dẫn kết quả; không giữ handle.

use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::R;
use crate::value::Value;

/// Dispatch `std.fs.canonicalize`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
/// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
pub(super) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi(ten, &args[0], 0, span)?;
    let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
    super::hop_nhat_fs(host.canonicalize(p).map(Value::Str), span)
}
