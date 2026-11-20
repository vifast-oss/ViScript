// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Chỉ test** — trạng thái `net.write` trên đường frame (tách khỏi `stdlib.rs` để giữ R1).

use std::cell::RefCell;
use std::collections::HashMap;

use super::netconst::FRAME_WRITE_TIMEOUT_MS;
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
pub(super) fn frame_unit_result(v: Value) -> Value {
    match &v {
        Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
            Value::ok(Value::Unit)
        }
        _ => v,
    }
}
