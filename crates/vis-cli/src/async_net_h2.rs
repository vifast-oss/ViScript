// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-W2** — phần HTTP/2 của [`super::AsyncNet`]: handle `H<n>#<frame>`, session dùng chung,
//! refcount, và cầu nối trait `FrameReactorBackend` (structured meta).
//!
//! Tách khỏi `async_net.rs` để giữ luật R1 (≤ 1500 dòng/tệp); là module con nên vẫn truy cập
//! được field private của `AsyncNet`.

use super::*;

use vis_interp::rt::{ReactorH2Outcome, ReactorH2Response};

impl AsyncNet {
    /// **VS3-W2** — cấp handle `H<n>` cho một connection h2 dùng chung.
    #[cfg(feature = "async-h2")]
    pub(super) fn dat_h2(&mut self, p: crate::net::h2_async::H2Async) -> String {
        self.next_h2 += 1;
        let h = format!("H{}", self.next_h2);
        self.h2.insert(h.clone(), p);
        h
    }

    /// **VS3-W2** — khoá canonical cho handshake TLS dùng chung; authority đã ra HTTP/1.1
    /// thì giữ khoá riêng từng frame (mỗi task cần socket riêng).
    #[cfg(feature = "async-h2")]
    pub(super) fn khoa_tls_canonical(&self, key: &str) -> String {
        let dia_chi = Self::dia_chi_cua_khoa_noi(key, FRAME_TLS_CONNECT);
        if self.h1_authority.contains(dia_chi) {
            key.to_string()
        } else {
            format!("{FRAME_TLS_CONNECT}{dia_chi}")
        }
    }

    /// **VS3-W2** — handle `H<n>#<frame>` → session `H<n>` (mỗi frame một khoá reactor riêng,
    /// nhưng chung một connection).
    #[cfg(feature = "async-h2")]
    pub(super) fn h2_base(handle: &str) -> &str {
        handle.split_once('#').map(|(b, _)| b).unwrap_or(handle)
    }

    #[cfg(feature = "async-h2")]
    pub(super) fn h2_ready_impl(&mut self, handle: &str) -> Result<bool, String> {
        let base = Self::h2_base(handle);
        let Some(p) = self.h2.get_mut(base) else {
            return Err(format!("handle HTTP/2 `{handle}` không tồn tại"));
        };
        p.xoa_co();
        loop {
            if p.san_sang() {
                return Ok(true);
            }
            if !p.tien()? {
                return Ok(false);
            }
        }
    }

    #[cfg(feature = "async-h2")]
    pub(super) fn h2_send_impl(
        &mut self,
        handle: &str,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        let Some(p) = self.h2.get_mut(Self::h2_base(handle)) else {
            return Err(format!("handle HTTP/2 `{handle}` không tồn tại"));
        };
        p.gui(method, target, headers)
    }

    #[cfg(feature = "async-h2")]
    pub(super) fn h2_poll_impl(&mut self, handle: &str, id: u64) -> ReactorH2Outcome {
        let Some(p) = self.h2.get_mut(Self::h2_base(handle)) else {
            return ReactorH2Outcome::Err(format!("handle HTTP/2 `{handle}` không tồn tại"));
        };
        p.xoa_co();
        loop {
            match p.ket_qua(id) {
                Some(Ok(kq)) => {
                    return ReactorH2Outcome::Response(ReactorH2Response {
                        status: kq.status,
                        headers: kq.headers,
                        body: kq.body,
                    })
                }
                Some(Err(e)) => return ReactorH2Outcome::Err(e),
                None => {}
            }
            match p.tien() {
                Ok(true) => {}
                Ok(false) => {
                    return if p.con_viec() {
                        ReactorH2Outcome::WouldBlock
                    } else {
                        ReactorH2Outcome::Err(
                            "phiên HTTP/2 không còn stream nhưng chưa có kết quả".to_string(),
                        )
                    }
                }
                Err(e) => return ReactorH2Outcome::Err(e),
            }
        }
    }

    #[cfg(feature = "async-h2")]
    pub(super) fn h2_cancel_impl(&mut self, handle: &str, id: u64) {
        if let Some(p) = self.h2.get_mut(Self::h2_base(handle)) {
            p.huy_stream(id);
        }
    }
}
