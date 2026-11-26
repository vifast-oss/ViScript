// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267** — `panic(msg)`: dừng chương trình bằng trap `VS0501` (exit **101**) ở cả hai engine.
//!
//! Payload thiết kế §6.4 là `AnyError`: `panic("chuỗi")` là đường tắt; `panic(err)` nhận bản ghi
//! dựng sẵn `Error` và dùng `message` của nó. Không bắt được trong code thường (chỉ ranh giới
//! task/shell bắt — chưa có ở VS0/VS1, ghi ở báo cáo).

use crate::builtin::trap;
use crate::codes::RT;
use vis_core::trap::R;
use vis_core::value::Value;
use vis_syntax::Span;

/// `panic(payload)` — 1 đối số; `str` hoặc bản ghi `Error`; luôn trả trap (kiểu tĩnh `never`).
pub fn panic_builtin(args: &[Value], span: Span) -> R<Value> {
    if args.len() != 1 {
        return Err(trap(RT, "`panic` cần 1 đối số", span));
    }
    let msg = match &args[0] {
        Value::Str(s) => s.clone(),
        // **VD-222**: bản ghi `Error` dựng sẵn (hoặc bản ghi có `message` do annot `Error` tạo
        // ở dạng ẩn danh) — dùng `message` làm thông điệp.
        Value::Record(r) => match r.fields.iter().find(|(n, _)| n == "message") {
            Some((_, v)) => v.show(),
            None => {
                return Err(trap(
                    RT,
                    format!("`panic` cần `str` hoặc `Error`, được `{}`", args[0].show()),
                    span,
                ))
            }
        },
        v => {
            return Err(trap(
                RT,
                format!("`panic` cần `str` hoặc `Error`, được `{}`", v.show()),
                span,
            ))
        }
    };
    Err(trap(RT, msg, span))
}
