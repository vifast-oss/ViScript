// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fsbytes)** — `std.fs.read_bytes` / `std.fs.write_bytes`.
//!
//! Byte thô được biểu diễn bằng `list[int]` (mỗi phần tử `0..=255`), vì VS0 **chưa có** kiểu
//! `byte`/`list[byte]`. Tách khỏi `stdlib.rs` để giữ **R1** (≤ 1.500 dòng/tệp); interp và VM dùng
//! **cùng** hàm này (một ngữ nghĩa, byte-identical).
//!
//! - `read_bytes`: mọi lỗi host (thiếu tệp, thư mục, lỗi đọc) ⇒ `Err(Error)` dữ liệu, **không**
//!   trap; dấu [`DAU_QUYEN`](super::DAU_QUYEN) vẫn được `hop_nhat_fs` nâng thành `VS0604`.
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
pub(super) fn goi(host: &mut dyn Host, ten: &str, args: &[Value], span: Span) -> R<Value> {
    match ten {
        "fs.read_bytes" => {
            let p = super::chuoi(ten, &args[0], 0, span)?;
            let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
            super::hop_nhat_fs(host.read_bytes(p).map(ds_byte), span)
        }
        "fs.write_bytes" => {
            let p = super::chuoi(ten, &args[0], 0, span)?;
            // **Kiểm hết trước khi lấy host/ghi** — dữ liệu sai không được tạo hay cắt tệp.
            let ds = match thanh_byte(ten, &args[1]) {
                Ok(ds) => ds,
                Err(e) => return Ok(Value::err(Value::ban_ghi_error(e, None))),
            };
            let host = host.fs().ok_or_else(|| super::khong_host(ten, span))?;
            super::hop_nhat_fs(host.write_bytes(p, &ds).map(|()| Value::Unit), span)
        }
        _ => Err(super::loi(
            format!("hàm byte thô không xác định: `{ten}`"),
            span,
        )),
    }
}
