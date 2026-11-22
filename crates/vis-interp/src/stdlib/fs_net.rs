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
    //!   trap; dấu [`DAU_QUYEN`](super::super::super::super::chung::DAU_QUYEN) vẫn được `hop_nhat_fs` nâng thành `VS0604`.
    //! - `write_bytes`: **kiểm toàn bộ** phần tử trước khi chạm đĩa — phần tử không phải `int` hoặc
    //!   ngoài `0..=255` ⇒ `Err(Error)`, không tạo/cắt tệp, không ghi một phần. Sau đó host tạo mới
    //!   hoặc cắt rồi ghi đúng từng byte.

    use vis_syntax::Span;

    use crate::builtin::Host;
    use crate::interp::R;
    use crate::value::Value;

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
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        match ten {
            "fs.read_bytes" => {
                let p = super::super::chuoi(ten, &args[0], 0, span)?;
                let host = host
                    .fs()
                    .ok_or_else(|| super::super::khong_host(ten, span))?;
                super::super::hop_nhat_fs(host.read_bytes(p).map(ds_byte), span)
            }
            "fs.write_bytes" => {
                let p = super::super::chuoi(ten, &args[0], 0, span)?;
                // **Kiểm hết trước khi lấy host/ghi** — dữ liệu sai không được tạo hay cắt tệp.
                let ds = match thanh_byte(ten, &args[1]) {
                    Ok(ds) => ds,
                    Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
                };
                let host = host
                    .fs()
                    .ok_or_else(|| super::super::khong_host(ten, span))?;
                super::super::hop_nhat_fs(host.write_bytes(p, &ds).map(|()| Value::Unit), span)
            }
            _ => Err(super::super::loi(
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
    use crate::interp::R;
    use crate::value::Value;

    /// Dispatch `std.fs.canonicalize`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
    /// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::khong_host(ten, span))?;
        super::super::hop_nhat_fs(host.canonicalize(p).map(Value::Str), span)
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
    use crate::interp::R;
    use crate::value::Value;

    /// Dispatch `std.fs.current_dir`: gọi host (không có đối số để kiểm kiểu động), rồi chuyển
    /// `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, _args: &[Value], span: Span) -> R<Value> {
        let host = host
            .fs()
            .ok_or_else(|| super::super::khong_host(ten, span))?;
        super::super::hop_nhat_fs(host.current_dir().map(Value::Str), span)
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
    use crate::interp::R;
    use crate::value::Value;

    /// Dispatch `std.fs.is_symlink`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
    /// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::khong_host(ten, span))?;
        super::super::hop_nhat_fs(host.is_symlink(p).map(Value::Bool), span)
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
    use crate::interp::R;
    use crate::value::Value;

    /// Dispatch hai hàm tạo thư mục (interp ↔ VM dùng chung).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::khong_host(ten, span))?;
        match ten {
            "fs.mkdir" => super::super::hop_nhat_fs(host.mkdir(p).map(|()| Value::Unit), span),
            "fs.mkdir_all" => {
                super::super::hop_nhat_fs(host.mkdir_all(p).map(|()| Value::Unit), span)
            }
            _ => Err(super::super::loi(
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
    use crate::interp::R;
    use crate::value::Value;

    /// Dispatch `std.fs.read_link`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), gọi host, rồi
    /// chuyển `Err` của host thành `Err(Error)` **dữ liệu** (dấu quyền ⇒ `VS0604`, như mọi hàm fs).
    pub(crate) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi(ten, &args[0], 0, span)?;
        let host = host
            .fs()
            .ok_or_else(|| super::super::khong_host(ten, span))?;
        super::super::hop_nhat_fs(host.read_link(p).map(Value::Str), span)
    }
}

pub(crate) mod netconst {
    //! Hằng số `std.*` tách khỏi `stdlib.rs` để giữ R1 (không đổi API công khai).

    /// **ST-238 / VD-242 (nợ #167)**: trần `max_bytes` cho `net.read`. Input người dùng (kể cả
    /// `i64::MAX`, âm, hay giá trị chuyển kiểu overflow) **không** được phép ép host cấp phát bộ nhớ
    /// tùy ý (OOM ⇒ abort). Vượt trần ⇒ `Result.Err` dữ liệu, không trap, không panic.
    pub const NET_READ_CAP: i64 = 8 * 1024 * 1024;

    /// Tiền tố handle cho một lần `net.connect` trên đường frame test (`N:<dia_chi>`).
    #[doc(hidden)]
    pub const FRAME_NET_CONNECT: &str = "N:";
    /// Tiền tố handle cho **TLS connect** trên đường frame (HTTP async): `T:<dia_chi>`.
    /// Chỉ `async_http` dùng; không lộ ra API ngôn ngữ.
    #[doc(hidden)]
    pub const FRAME_TLS_CONNECT: &str = "T:";
    /// Tiền tố handle cho một lần `net.write` trên đường frame test (`W:<tay>`).
    #[doc(hidden)]
    pub const FRAME_NET_WRITE: &str = "W:";
    /// Hạn mặc định của `net.write` trên đường frame test (ngôn ngữ không có đối số timeout).
    /// Chọn rộng cho loopback; vẫn là hạn monotonic DUY NHẤT, không reset theo readiness.
    #[doc(hidden)]
    pub const FRAME_WRITE_TIMEOUT_MS: i64 = 5_000;

    /// **ST-238 / VD-242 (nợ #167)**: validate `max_bytes` từ người dùng. `i64::MAX` (overflow khi
    /// chuyển `usize`), âm, và giá trị vượt trần đều bị từ chối **trước** khi host cấp phát.
    pub(crate) fn gioi_han_net_read(toi_da: i64) -> Result<usize, String> {
        if toi_da < 0 {
            return Err("net.read: `max_bytes` không được âm".to_string());
        }
        if toi_da > NET_READ_CAP {
            return Err(format!(
                "net.read: `max_bytes` {toi_da} vượt giới hạn {NET_READ_CAP} byte"
            ));
        }
        Ok(toi_da as usize)
    }
}

pub(crate) mod frame_net {
    //! **Chỉ test** — trạng thái `net.write` trên đường frame (tách khỏi `stdlib.rs` để giữ R1).

    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::super::netconst::FRAME_WRITE_TIMEOUT_MS;
    use crate::value::Value;

    #[derive(Default)]
    struct FrameNetIo {
        /// `W:<tay>` → bytes cần gửi (adapter `take` sang trạng thái riêng ở lần poll đầu).
        writes: HashMap<String, Vec<u8>>,
        /// Test có thể thu hẹp hạn write để kiểm chứng timeout mà không đổi API ngôn ngữ.
        write_timeout_ms: Option<i64>,
    }
    thread_local! {
        static FRAME_NET_IO: RefCell<FrameNetIo> = RefCell::new(FrameNetIo::default());
    }

    /// **Chỉ test**: ghi bytes cần gửi cho khoá `W:<tay>` trước khi gọi `frame_reactor_poll_io`.
    #[doc(hidden)]
    pub fn frame_net_set_write(key: &str, data: &[u8]) {
        FRAME_NET_IO.with(|x| {
            x.borrow_mut().writes.insert(key.to_string(), data.to_vec());
        });
    }

    /// **Chỉ test**: lấy (và xoá) bytes cần gửi của khoá `W:<tay>`.
    #[doc(hidden)]
    pub fn frame_net_take_write(key: &str) -> Option<Vec<u8>> {
        FRAME_NET_IO.with(|x| x.borrow_mut().writes.remove(key))
    }

    /// **Chỉ test**: đặt lại hạn write (ms); `None` ⇒ dùng [`FRAME_WRITE_TIMEOUT_MS`].
    #[doc(hidden)]
    pub fn frame_net_set_write_timeout(ms: Option<i64>) {
        FRAME_NET_IO.with(|x| x.borrow_mut().write_timeout_ms = ms);
    }

    /// **Chỉ test**: hạn write hiệu lực, luôn dương (tránh trần chờ selector vô hạn).
    #[doc(hidden)]
    pub fn frame_net_write_timeout() -> i64 {
        FRAME_NET_IO.with(|x| {
            x.borrow()
                .write_timeout_ms
                .unwrap_or(FRAME_WRITE_TIMEOUT_MS)
                .max(1)
        })
    }

    /// **Chỉ test**: `net.write` trả `Result<(), str>` — đổi payload `Ok(str)` của reactor
    /// thành `Ok(())`, giữ nguyên `Err` (hợp đồng số byte/lỗi KHÔNG đổi).
    pub(crate) fn frame_unit_result(v: Value) -> Value {
        match &v {
            Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
                Value::ok(Value::Unit)
            }
            _ => v,
        }
    }
}
