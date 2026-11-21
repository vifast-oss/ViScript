// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-W2 (VD-271)** — lõi HTTP/2 **dùng chung** cho client sync (`net/h2.rs`) và
//! reactor-native (`net/h2_async.rs`, feature `async-h2`, default-off).
//!
//! Phần này chỉ giữ **state machine protocol**: HPACK/framing/flow-control của crate `h2`,
//! một `HashMap` stream đang chờ và cap header/body. Không biết socket, reactor hay `mio`:
//! `H2Core<T>` nhận `T: AsyncRead + AsyncWrite + Unpin` đã handshake xong; wrapper quyết định
//! cách lái (`cho()` + `yield` cho sync, `mio` readiness cho async).
//!
//! Nhờ vậy không có hai client h2 lệch nhau; sync và async dùng đúng một logic cap/lỗi/trailer.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll as TaskPoll, Waker};

use bytes::Bytes;
use http::Request;
use tokio::io::{AsyncRead, AsyncWrite};

use vis_interp::builtin::KetQuaHttp2;
use vis_interp::net::MAX_HTTP;

/// Cap header phản hồi HTTP/2 (cùng ngưỡng HTTP/1.1 64 KiB) để server không làm phình bộ nhớ.
pub(super) const MAX_HEADER: usize = 64 * 1024;

pub(super) fn loi_h2(e: &::h2::Error, viec: &str) -> String {
    let loai = if e.is_go_away() {
        "GOAWAY"
    } else if e.is_reset() {
        "RST_STREAM"
    } else {
        "lỗi"
    };
    format!("{loai} HTTP/2 khi {viec}: {e}")
}

/// Một stream h2 đang chờ; tách khỏi connection để nhiều stream sống song song.
struct Song {
    /// Giữ `SendStream` để còn `RST_STREAM` khi timeout/cap/lỗi.
    gui: Option<::h2::SendStream<Bytes>>,
    /// Chờ header response; `None` sau khi đã nhận header.
    resp: Option<Pin<Box<::h2::client::ResponseFuture>>>,
    recv: Option<::h2::RecvStream>,
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    xong_data: bool,
    ket_qua: Option<Result<KetQuaHttp2, String>>,
}

impl Song {
    fn moi(resp: ::h2::client::ResponseFuture, gui: ::h2::SendStream<Bytes>) -> Self {
        Self {
            gui: Some(gui),
            resp: Some(Box::pin(resp)),
            recv: None,
            status: 0,
            headers: Vec::new(),
            body: Vec::new(),
            xong_data: false,
            ket_qua: None,
        }
    }

    fn dat_loi(&mut self, loi: String) {
        if let Some(s) = self.gui.as_mut() {
            s.send_reset(::h2::Reason::CANCEL);
        }
        self.ket_qua = Some(Err(loi));
        self.gui = None;
        self.resp = None;
        self.recv = None;
    }

    fn hoan_tat(&mut self) {
        self.ket_qua = Some(Ok(KetQuaHttp2 {
            status: self.status,
            headers: std::mem::take(&mut self.headers),
            body: std::mem::take(&mut self.body),
        }));
        self.gui = None;
        self.resp = None;
        self.recv = None;
    }

    /// Một lượt poll stream; lỗi **của stream** được ghi vào `ket_qua`, không phá session.
    fn buoc(&mut self, cx: &mut Context<'_>, tien: &mut bool) {
        if self.ket_qua.is_some() {
            return;
        }
        if self.recv.is_none() {
            let Some(resp) = self.resp.as_mut() else {
                return;
            };
            match resp.as_mut().poll(cx) {
                TaskPoll::Ready(Ok(r)) => {
                    *tien = true;
                    let (parts, b) = r.into_parts();
                    let tong: usize = parts
                        .headers
                        .iter()
                        .map(|(k, v)| k.as_str().len() + v.as_bytes().len())
                        .sum();
                    if tong > MAX_HEADER {
                        self.dat_loi(format!("header HTTP/2 quá lớn (> {MAX_HEADER} byte)"));
                        return;
                    }
                    self.status = parts.status.as_u16();
                    self.headers = parts
                        .headers
                        .iter()
                        .map(|(k, v)| {
                            (
                                k.as_str().to_string(),
                                String::from_utf8_lossy(v.as_bytes()).into_owned(),
                            )
                        })
                        .collect();
                    self.recv = Some(b);
                    self.resp = None;
                }
                TaskPoll::Ready(Err(e)) => {
                    *tien = true;
                    self.dat_loi(loi_h2(&e, "response"));
                    return;
                }
                TaskPoll::Pending => {}
            }
        }
        if self.ket_qua.is_some() {
            return;
        }
        let Some(mut r) = self.recv.take() else {
            return;
        };
        if !self.xong_data {
            match r.poll_data(cx) {
                TaskPoll::Ready(Some(Ok(b))) => {
                    *tien = true;
                    if self.body.len().saturating_add(b.len()) > MAX_HTTP {
                        self.dat_loi(format!(
                            "phản hồi HTTP/2 vượt giới hạn {MAX_HTTP} byte (8 MiB)"
                        ));
                        return;
                    }
                    let n = b.len();
                    self.body.extend_from_slice(&b);
                    drop(b);
                    // h2 không tự trả cửa sổ flow-control; phải release từng chunk.
                    if let Err(e) = r.flow_control().release_capacity(n) {
                        self.dat_loi(format!("lỗi HTTP/2 khi giải phóng flow-control: {e}"));
                        return;
                    }
                    self.recv = Some(r);
                }
                TaskPoll::Ready(Some(Err(e))) => {
                    *tien = true;
                    self.dat_loi(loi_h2(&e, "đọc body"));
                }
                TaskPoll::Ready(None) => {
                    *tien = true;
                    self.xong_data = true;
                    self.recv = Some(r);
                }
                TaskPoll::Pending => {
                    self.recv = Some(r);
                }
            }
        } else {
            match r.poll_trailers(cx) {
                TaskPoll::Ready(Ok(_)) => {
                    *tien = true;
                    self.hoan_tat();
                }
                TaskPoll::Ready(Err(e)) => {
                    *tien = true;
                    self.dat_loi(loi_h2(&e, "trailer"));
                }
                TaskPoll::Pending => {
                    self.recv = Some(r);
                }
            }
        }
    }
}

/// Protocol HTTP/2 trên **một** connection đã handshake; nhiều stream song song.
pub(super) struct H2Core<T> {
    send: ::h2::client::SendRequest<Bytes>,
    conn: Pin<Box<::h2::client::Connection<T, Bytes>>>,
    authority: String,
    songs: HashMap<u64, Song>,
    next_id: u64,
    da_dong: bool,
}

impl<T: AsyncRead + AsyncWrite + Unpin> H2Core<T> {
    pub(super) fn moi(
        send: ::h2::client::SendRequest<Bytes>,
        conn: ::h2::client::Connection<T, Bytes>,
        authority: String,
    ) -> Self {
        Self {
            send,
            conn: Box::pin(conn),
            authority,
            songs: HashMap::new(),
            next_id: 0,
            da_dong: false,
        }
    }

    /// Gửi request (end_stream ngay, body rỗng) và trả id nội bộ để lấy kết quả.
    pub(super) fn gui(
        &mut self,
        method: &str,
        target: &str,
        headers: &[(String, String)],
    ) -> Result<u64, String> {
        if self.da_dong {
            // Giữ tiền tố để `Net` nhận diện connection đã GOAWAY/đóng và thử lại một lần.
            return Err("GOAWAY HTTP/2 khi gửi request: phiên đã đóng".to_string());
        }
        let uri = format!("https://{}{}", self.authority, target);
        let mut builder = Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        let req = builder
            .body(())
            .map_err(|e| format!("request HTTP/2 không hợp lệ: {e}"))?;
        let (resp, gui) = self
            .send
            .send_request(req, true)
            .map_err(|e| loi_h2(&e, "gửi request"))?;
        let id = self.next_id;
        self.next_id += 1;
        self.songs.insert(id, Song::moi(resp, gui));
        Ok(id)
    }

    /// Còn stream chưa có kết quả?
    #[cfg(any(test, feature = "async-h2"))]
    pub(super) fn con_cho(&self) -> bool {
        self.songs.values().any(|s| s.ket_qua.is_none())
    }

    pub(super) fn da_dong(&self) -> bool {
        self.da_dong
    }

    /// Poll connection + mọi stream **một lượt**; `true` = vừa có tiến triển.
    pub(super) fn buoc(&mut self) -> Result<bool, String> {
        if self.da_dong {
            return Ok(false);
        }
        let mut tien = false;
        let w = Waker::noop();
        let mut cx = Context::from_waker(w);
        // Khi connection đóng/lỗi, poll nốt các stream **trước** để h2 kịp trả lỗi GOAWAY/RST
        // theo từng stream; chỉ stream còn treo mới nhận lỗi connection.
        let loi_conn = match self.conn.as_mut().poll(&mut cx) {
            TaskPoll::Ready(Ok(())) => {
                Some("kết nối HTTP/2 đã đóng khi còn stream chờ".to_string())
            }
            TaskPoll::Ready(Err(e)) => Some(loi_h2(&e, "kết nối")),
            TaskPoll::Pending => None,
        };
        if let Some(loi) = loi_conn {
            self.da_dong = true;
            // Connection đã đóng nhưng h2 có thể còn frame response đã đệm: poll cạn từng
            // stream (data/trailer) trước khi biến stream còn treo thành lỗi connection.
            loop {
                let mut buoc_nay = false;
                for song in self.songs.values_mut() {
                    song.buoc(&mut cx, &mut buoc_nay);
                }
                if !buoc_nay {
                    break;
                }
                tien = true;
            }
            // Connection đóng **sau** khi stream đã xong (vd GOAWAY sau response) không phải
            // lỗi của request đó; chỉ trả lỗi khi còn stream treo thật sự.
            let con_treo = self.songs.values().any(|s| s.ket_qua.is_none());
            self.loi_het(loi.clone());
            return if con_treo { Err(loi) } else { Ok(tien) };
        }
        for song in self.songs.values_mut() {
            song.buoc(&mut cx, &mut tien);
        }
        Ok(tien)
    }

    pub(super) fn loi_het(&mut self, loi: String) {
        for s in self.songs.values_mut() {
            if s.ket_qua.is_none() {
                s.dat_loi(loi.clone());
            }
        }
    }

    /// Hết hạn: RST mọi stream còn mở và đánh dấu connection không tái dùng.
    #[cfg(feature = "async-h2")]
    pub(super) fn huy(&mut self, loi: String) {
        self.loi_het(loi);
        self.da_dong = true;
    }

    /// Huỷ riêng một stream (timeout/cancel) nhưng giữ connection cho stream khác.
    #[cfg(feature = "async-h2")]
    pub(super) fn huy_stream(&mut self, id: u64) {
        if let Some(s) = self.songs.get_mut(&id) {
            s.dat_loi("quá thời gian chờ phản hồi HTTP/2".to_string());
        }
    }

    /// Lấy kết quả của một stream (một lần).
    pub(super) fn ket_qua(&mut self, id: u64) -> Option<Result<KetQuaHttp2, String>> {
        self.songs.get_mut(&id).and_then(|s| s.ket_qua.take())
    }
}
