// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `fs_net` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod fsbytes {
    //! **VD-270 (fsbytes)** — `std.fs.read_bytes` / `std.fs.write_bytes`.
    //!
    //! Byte thô được biểu diễn bằng `list[int]` (mỗi phần tử `0..=255`), vì VS0 **chưa có** kiểu
    //! `byte`/`list[byte]`. Tách khỏi `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng
    //! **cùng** hàm này (một ngữ nghĩa, byte-identical).
    //!
    //! - `read_bytes`: mọi lỗi host (thiếu tệp, thư mục, lỗi đọc) ⇒ `Err(Error)` dữ liệu, **không**
    //!   trap; dấu [`DAU_QUYEN`](super::super::super::super::super::super::chung::DAU_QUYEN) vẫn được `hop_nhat_fs` nâng thành `VS0604`.
    //! - `write_bytes`: **kiểm toàn bộ** phần tử trước khi chạm đĩa — phần tử không phải `int` hoặc
    //!   ngoài `0..=255` ⇒ `Err(Error)`, không tạo/cắt tệp, không ghi một phần. Sau đó host tạo mới
    //!   hoặc cắt rồi ghi đúng từng byte.

    use vis_syntax::Span;

    use crate::builtin::Host;
    use vis_core::trap::R;
    use vis_core::value::Value;
    use vis_hir::stdlib::StdId;

    /// `list[int]` (mỗi phần tử `0..=255`) → `Vec<u8>`. Kiểm **hết** trước khi trả về; lỗi là **dữ
    /// liệu** (chuỗi thông điệp), caller bọc thành `Err(Error)` — không trap.
    fn thanh_byte(ten: &str, v: &Value) -> Result<Vec<u8>, String> {
        let Value::List(xs) = v else {
            return Err(format!(
                "`{ten}`: đối số 2 cần `list[int]`, được `{}`",
                v.show()
            ));
        };
        let mut ra = Vec::with_capacity(xs.len());
        for (i, x) in xs.iter().enumerate() {
            let Value::Int(n) = x else {
                return Err(format!(
                    "`{ten}`: phần tử thứ {i} cần `int`, được `{}`",
                    x.show()
                ));
            };
            if !(0..=255).contains(n) {
                return Err(format!(
                    "`{ten}`: phần tử thứ {i} = {n} ngoài phạm vi byte 0..=255"
                ));
            }
            ra.push(*n as u8);
        }
        Ok(ra)
    }

    /// `Vec<u8>` → `list[int]` (mỗi byte thành một `int`).
    fn ds_byte(ds: Vec<u8>) -> Value {
        Value::List(ds.into_iter().map(|b| Value::Int(i64::from(b))).collect())
    }

    /// Dispatch hai hàm `std.fs.*` byte thô (interp ↔ VM dùng chung).
    pub(crate) fn goi(host: &mut dyn Host, id: StdId, args: &[Value], span: Span) -> R<Value> {
        let ten = id.as_str();
        match id {
            StdId::FsReadBytes => {
                let p = super::super::super::chuoi(ten, &args[0], 0, span)?;
                let host = host
                    .fs()
                    .ok_or_else(|| super::super::super::khong_host(ten, span))?;
                super::super::super::hop_nhat_fs(host.read_bytes(p).map(ds_byte), span)
            }
            StdId::FsWriteBytes => {
                let p = super::super::super::chuoi(ten, &args[0], 0, span)?;
                // **Kiểm hết trước khi lấy host/ghi** — dữ liệu sai không được tạo hay cắt tệp.
                let ds = match thanh_byte(ten, &args[1]) {
                    Ok(ds) => ds,
                    Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
                };
                let host = host
                    .fs()
                    .ok_or_else(|| super::super::super::khong_host(ten, span))?;
                super::super::super::hop_nhat_fs(
                    host.write_bytes(p, &ds).map(|()| Value::Unit),
                    span,
                )
            }
            _ => Err(super::super::super::loi(
                format!("hàm byte thô không xác định: `{ten}`"),
                span,
            )),
        }
    }
}

pub(crate) mod fscanonicalize {
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
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Dispatch `std.fs.canonicalize`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
    /// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::super::khong_host(ten, span))?;
        super::super::super::hop_nhat_fs(host.canonicalize(p).map(Value::Str), span)
    }
}

pub(crate) mod fscurrentdir {
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
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Dispatch `std.fs.current_dir`: gọi host (không có đối số để kiểm kiểu động), rồi chuyển
    /// `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, _args: &[Value], span: Span) -> R<Value> {
        let host = host
            .fs()
            .ok_or_else(|| super::super::super::khong_host(ten, span))?;
        super::super::super::hop_nhat_fs(host.current_dir().map(Value::Str), span)
    }
}

pub(crate) mod fsissymlink {
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
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Dispatch `std.fs.is_symlink`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
    /// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::super::khong_host(ten, span))?;
        super::super::super::hop_nhat_fs(host.is_symlink(p).map(Value::Bool), span)
    }
}

pub(crate) mod fsmkdirall {
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
    use vis_core::trap::R;
    use vis_core::value::Value;
    use vis_hir::stdlib::StdId;

    /// Dispatch hai hàm tạo thư mục (interp ↔ VM dùng chung).
    pub(crate) fn goi(host: &mut dyn Host, id: StdId, args: &[Value], span: Span) -> R<Value> {
        let ten = id.as_str();
        let p = super::super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::super::khong_host(ten, span))?;
        match id {
            StdId::FsMkdir => {
                super::super::super::hop_nhat_fs(host.mkdir(p).map(|()| Value::Unit), span)
            }
            StdId::FsMkdirAll => {
                super::super::super::hop_nhat_fs(host.mkdir_all(p).map(|()| Value::Unit), span)
            }
            _ => Err(super::super::super::loi(
                format!("hàm tạo thư mục không xác định: `{ten}`"),
                span,
            )),
        }
    }
}

pub(crate) mod fsreadlink {
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
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Dispatch `std.fs.read_link`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
    /// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::super::khong_host(ten, span))?;
        super::super::super::hop_nhat_fs(host.read_link(p).map(Value::Str), span)
    }
}
