// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-312d (VD-289)** — adapter HTTP async: cài [`AsyncHttpEnv`] cho `Host` của interp.
//!
//! Protocol/state machine nằm ở `vis-http` (sans-IO); tệp này chỉ nối scheduler frame +
//! `NetHost` của engine vào trait. Không đổi hành vi/không thêm API ngôn ngữ.

use vis_core::trap::R;
use vis_core::value::Value;
use vis_http::{AsyncHttpEnv, FrameReactorBackend, ReactorH2Response, ReactorOp};
use vis_syntax::Span;

use crate::builtin::Host;

/// Bọc `&mut dyn Host` để cài trait của `vis-http` (không impl trực tiếp: orphan rule).
pub(crate) struct Env<'a>(pub(crate) &'a mut dyn Host);

impl AsyncHttpEnv for Env<'_> {
    fn frame_current(&self) -> u32 {
        self.0.sched().borrow().frame.current
    }

    fn frame_backend(&self) -> Option<std::rc::Rc<std::cell::RefCell<dyn FrameReactorBackend>>> {
        self.0.sched().borrow().frame.backend.clone()
    }

    fn note_handle(&mut self, handle: &str) {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_note_handle(handle);
    }

    fn cancel_handle(&mut self, handle: &str) {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_cancel_handle(handle);
    }

    fn poll_io(
        &mut self,
        op: ReactorOp,
        handle: &str,
        max: usize,
        timeout_ms: i64,
        span: Span,
    ) -> R<Value> {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_poll_io(op, handle, max, timeout_ms, span)
    }

    fn poll_bytes(
        &mut self,
        handle: &str,
        max: usize,
        timeout_ms: i64,
        span: Span,
    ) -> R<Result<Vec<u8>, String>> {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_poll_bytes(handle, max, timeout_ms, span)
    }

    fn h2_send(
        &mut self,
        handle: &str,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_h2_send(handle, method, target, headers)
    }

    fn h2_poll(
        &mut self,
        handle: &str,
        id: u64,
        timeout_ms: i64,
        span: Span,
    ) -> R<Result<ReactorH2Response, String>> {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_h2_poll(handle, id, timeout_ms, span)
    }

    fn h2_cancel(&mut self, handle: &str, id: u64) {
        self.0
            .sched()
            .borrow_mut()
            .frame_reactor_h2_cancel(handle, id);
    }

    fn net(&mut self) -> Option<&mut dyn vis_core::host::NetHost> {
        self.0.net()
    }
}

pub(crate) fn http_get_async(
    host: &mut dyn Host,
    url: &str,
    timeout_ms: u64,
    span: Span,
) -> R<Value> {
    vis_http::http_get_async(&mut Env(host), url, timeout_ms, span)
}

pub(crate) fn http_head_async(
    host: &mut dyn Host,
    url: &str,
    timeout_ms: u64,
    span: Span,
) -> R<Value> {
    vis_http::http_head_async(&mut Env(host), url, timeout_ms, span)
}

pub(crate) fn xoa_trang_thai_frame(id: u32) {
    vis_http::xoa_trang_thai_frame(id);
}
