// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P1)** — runner W=1 dùng chung cho `await`/`join` trên caller.
//!
//! Thân tác vụ chạy bằng `Host::goi_than` (interp: `Hfn` trong `Interp.env`; VM: hàm bytecode),
//! nên W=1 thấy **shared env** và in trực tiếp như runtime một luồng. Lỗi thân được bọc
//! `VS0506` giữ mã gốc + span lúc tạo, y hệt `await_task` trước P1.
use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::{Abort, Trap, R, RT};
use crate::value::Value;

/// Chạy thân một tác vụ **trên caller**. Không snapshot/buffer/thread; mọi ghi đi vào env chung
/// của host (W=1). `ten` dùng cho thông điệp `VS0506`.
pub(crate) fn chay_than_local(
    host: &mut dyn Host,
    than: u32,
    args: Vec<Value>,
    ten: &str,
    span: Span,
) -> R<Value> {
    match host.goi_than(than, args, span) {
        Ok(v) => Ok(v),
        // `return` trong thân tác vụ: `Abort::Ret` là kết quả bình thường.
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Trap(goc)) => Err(Abort::Trap(crate::rt::wrap_task_error(ten, goc, span))),
        // Đường frame không đi qua Local (P1 non-frame); phòng thủ thay vì panic.
        Err(Abort::Pending(_)) => Err(Abort::Trap(crate::rt::wrap_task_error(
            ten,
            Trap::new(RT, "pending trong thân tác vụ", span),
            span,
        ))),
    }
}
