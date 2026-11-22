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
        let base = Self::h2_base(handle).to_string();
        let mut tien = false;
        let kq = {
            let Some(p) = self.h2.get_mut(&base) else {
                return ReactorH2Outcome::Err(format!("handle HTTP/2 `{handle}` không tồn tại"));
            };
            p.xoa_co();
            loop {
                match p.ket_qua(id) {
                    Some(Ok(kq)) => {
                        break ReactorH2Outcome::Response(ReactorH2Response {
                            status: kq.status,
                            headers: kq.headers,
                            body: kq.body,
                        })
                    }
                    Some(Err(e)) => break ReactorH2Outcome::Err(e),
                    None => {}
                }
                match p.tien() {
                    Ok(true) => tien = true,
                    Ok(false) => {
                        break if p.con_viec() {
                            ReactorH2Outcome::WouldBlock
                        } else {
                            ReactorH2Outcome::Err(
                                "phiên HTTP/2 không còn stream nhưng chưa có kết quả".to_string(),
                            )
                        }
                    }
                    Err(e) => break ReactorH2Outcome::Err(e),
                }
            }
        };
        // Mọi tiến triển đều có thể đã phục vụ stream của frame khác — kể cả khi frame
        // hiện tại đã có response; đánh thức peer trước khi trả.
        if tien {
            self.danh_thuc_nguon(&base);
        }
        kq
    }

    /// Đánh thức các token khác cùng nguồn h2 (frame khác có thể vừa được phục vụ hộ).
    pub(super) fn danh_thuc_nguon(&mut self, key: &str) {
        if self
            .nguon_tokens
            .get(key)
            .map(|d| d.len() > 1)
            .unwrap_or(false)
        {
            self.wake_nguon.insert(key.to_string());
            let _ = self.waker.wake();
        }
    }

    #[cfg(feature = "async-h2")]
    pub(super) fn h2_cancel_impl(&mut self, handle: &str, id: u64) {
        if let Some(p) = self.h2.get_mut(Self::h2_base(handle)) {
            p.huy_stream(id);
            // **ST-287 (VD-273)** — `send_reset` chỉ xếp frame vào `h2`; phải lái session
            // một nhịp để `RST_STREAM` thực sự ra socket ngay tại safe point (không đợi
            // request kế tiếp). Có trần để không kéo I/O không sẵn sàng.
            for _ in 0..8 {
                match p.tien() {
                    Ok(true) => continue,
                    _ => break,
                }
            }
        }
    }
    /// Xử lý close cho khoá TLS/h2. `true` = đã xử lý xong (không cần cleanup chung).
    pub(super) fn dong_h2(&mut self, handle: &str) -> bool {
        if handle.starts_with(FRAME_TLS_CONNECT) {
            if let Some(ck) = self.tls_alias.remove(handle) {
                if !self.tls_alias.values().any(|v| *v == ck) {
                    self.tls_connects.remove(&ck);
                }
            } else {
                self.tls_connects.remove(handle);
            }
            self.streams.remove(handle);
            self.listeners.remove(handle);
            self.connects.remove(handle);
            let keys: Vec<u64> = self
                .tokens
                .iter()
                .filter(|(_, k)| k.as_str() == handle)
                .map(|(t, _)| *t)
                .collect();
            for t in keys {
                let _ = <Self as FrameReactorBackend>::deregister(self, t, handle, ReactorOp::Read);
            }
            self.writes.retain(|_, w| w.tay != handle);
            self.bo_sach_nguon(handle);
            return true;
        }
        if self.h2.contains_key(Self::h2_base(handle)) {
            let base = Self::h2_base(handle).to_string();
            let con_lai = match self.h2_users.get_mut(&base) {
                Some(n) => {
                    *n = n.saturating_sub(1);
                    *n
                }
                None => 0,
            };
            if con_lai == 0 {
                self.h2_idle.insert(base.clone(), Instant::now());
            }
        }
        false
    }
}
