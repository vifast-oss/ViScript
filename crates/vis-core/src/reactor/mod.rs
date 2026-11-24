// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — trait **reactor** dùng chung + trạng thái `net.write` của đường frame.
//!
//! Nguồn gốc lịch sử: `vis-interp/src/rt.rs` (trait adapter selector) và
//! `stdlib/fs_net.rs` (hằng số + kênh write) — nay `vis-runtime/src/rt/`. Adapter thật (`mio`, TLS, h2) nằm ở host; crate
//! này **zero-dep** và không bao giờ biết fd/socket thật.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

use crate::value::Value;

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
pub fn gioi_han_net_read(toi_da: i64) -> Result<usize, String> {
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

/// **Chỉ test (Lát 2)**: phép toán mạng được reactor giả phủ (mở rộng dần).
/// `connect`/`write` vẫn đồng bộ ở đường test — xem report (có thể chặn scheduler).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactorOp {
    Read,
    Accept,
    /// **VS3-W2** — chờ cả READABLE lẫn WRITABLE cho một bước HTTP/2 (session tự quyết).
    ReadWrite,
}

/// **VS3-W2** — response h2 có cấu trúc cho core (không nhồi lại thành HTTP/1.1).
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ReactorH2Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// **VS3-W2** — kết quả một bước `h2_poll`.
#[doc(hidden)]
#[derive(Debug)]
pub enum ReactorH2Outcome {
    /// Cần readiness READABLE|WRITABLE (scheduler đăng ký rồi trả `Pending`).
    WouldBlock,
    Response(ReactorH2Response),
    Err(String),
}

/// Trait adapter nằm ở crate lõi (zero-dep, cross-platform) để core/wasm32 không
/// bao giờ phụ thuộc crate selector. Crate host (Linux) cài đặt bằng `mio` — mọi
/// API ta gọi đều safe; `unsafe` FFI epoll nằm hoàn toàn trong dependency.
/// Không có implementation mặc định ⇒ không có fallback im lặng sang đồng bộ.
#[doc(hidden)]
pub trait FrameReactorBackend {
    /// Thử `read` không chặn. `max` đã bị chặn bởi `NET_READ_CAP` ở `stdlib`.
    fn try_read(&mut self, handle: &str, max: usize) -> ReactorIoOutcome;
    /// Thử `accept` không chặn — khi có kết nối, adapter cấp handle mới.
    fn try_accept(&mut self, handle: &str) -> ReactorIoOutcome;
    /// Đăng ký/kích hoạt lại interest READABLE cho `handle` với `token` MỚI.
    fn register(&mut self, token: u64, handle: &str, op: ReactorOp) -> Result<(), String>;
    /// Hủy đăng ký `token` (idempotent).
    fn deregister(&mut self, token: u64, handle: &str, op: ReactorOp) -> Result<(), String>;
    /// Chờ readiness tối đa `timeout`; trả sự kiện thật đã dịch sang token/core.
    fn wait(&mut self, timeout: Duration) -> Result<Vec<ReactorRawEvent>, String>;
    /// Đóng tay (`net.close`) — adapter đóng cả socket thật; stale event vô hiệu ở core.
    fn close(&mut self, handle: &str) -> Result<(), String>;
    /// **VS3-W2** — lái handshake h2 một bước; `Ok(true)` = sẵn sàng gửi request.
    /// Backend không hỗ trợ h2 giữ mặc định lỗi rõ (test scripted không phải sửa).
    fn h2_ready(&mut self, _handle: &str) -> Result<bool, String> {
        Err("backend không hỗ trợ HTTP/2 async".to_string())
    }
    /// **VS3-W2** — gửi request h2 trên connection `handle`; trả id stream nội bộ.
    fn h2_send(
        &mut self,
        _handle: &str,
        _method: &str,
        _target: &str,
        _headers: &[(String, String)],
    ) -> Result<u64, String> {
        Err("backend không hỗ trợ HTTP/2 async".to_string())
    }
    /// **VS3-W2** — poll một bước session + kiểm stream `id`.
    fn h2_poll(&mut self, _handle: &str, _id: u64) -> ReactorH2Outcome {
        ReactorH2Outcome::Err("backend không hỗ trợ HTTP/2 async".to_string())
    }
    /// **VS3-W2** — huỷ riêng stream `id` (timeout/cancel) mà không đóng session.
    fn h2_cancel(&mut self, _handle: &str, _id: u64) {}
    /// **ST-287 (VD-273)** — ghi nhận `handle` do **HTTP async** của frame `frame_id` đang giữ.
    /// Chỉ dùng cho tài nguyên nội bộ của `http.get/head` (không phải language handle); backend
    /// mặc định bỏ qua, backend thật ghi sổ để [`Self::cancel_task_handles`] đóng đúng tay.
    fn note_task_handle(&mut self, _frame_id: u32, _handle: &str) {}
    /// **ST-287 (VD-273)** — ghi nhận stream h2 `id` trên `handle` của frame `frame_id` để huỷ
    /// gửi `RST_STREAM` đúng stream (session có thể dùng chung với frame khác).
    fn note_task_h2(&mut self, _frame_id: u32, _handle: &str, _id: u64) {}
    /// **ST-287 (VD-273)** — **safe point** của huỷ task: đóng mọi tay HTTP async còn treo của
    /// frame `frame_id` (`H2Async::dong()` + `RST_STREAM` cho h2, đóng socket cho h1).
    /// Backend mặc định no-op để adapter scripted/test cũ không phải sửa.
    fn cancel_task_handles(&mut self, _frame_id: u32) {}
}

impl std::fmt::Debug for dyn FrameReactorBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FrameReactorBackend")
    }
}

/// Kết quả một lần thử I/O thật. `WouldBlock` ⇒ đăng ký readiness + `Pending`;
/// không phải "không có sự kiện" (select error là `Err`, lan rõ).
#[doc(hidden)]
#[derive(Debug)]
pub enum ReactorIoOutcome {
    /// `read`: byte nhận được (rỗng = EOF). Caller giải mã lossy/đóng gói `Result`.
    Data(Vec<u8>),
    /// `accept`: handle kết nối mới do adapter cấp.
    Accepted(String),
    /// Kernel chưa sẵn sàng — re-arm, không hot-spin, không mất wake.
    WouldBlock,
    /// Lỗi I/O/selector — lan thành `Result.Err` rõ, không nuốt.
    Err(String),
}

/// Một sự kiện readiness thật đã dịch sang token/core (không lộ fd hay `mio`).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactorRawEvent {
    pub token: u64,
    pub readable: bool,
    pub hup: bool,
    pub error: bool,
}

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
pub fn frame_unit_result(v: Value) -> Value {
    match &v {
        Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
            Value::ok(Value::Unit)
        }
        _ => v,
    }
}
