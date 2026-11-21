// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d/W1f-HTTP** — `http.get/head` **hợp tác** trên frame scheduler.
//!
//! Mỗi lời gọi là một state machine lưu theo `frame.current`; mỗi bước I/O đi qua
//! [`Scheduler::frame_reactor_poll_io`]/[`Scheduler::frame_reactor_poll_bytes`] nên khi kernel
//! chưa sẵn sàng thì frame trả `Abort::Pending` và scheduler chạy tác vụ khác. Không gọi
//! `NetHost::connect/read/write` blocking giữa request. TLS dùng handle `T:<addr>` do
//! `AsyncNet` (CLI) cài; HTTP/1.1 trước, HTTP/2 async còn mở.
//!
//! Phần parser/redirect dùng **chung** helper của [`super`] với đường sync; chỉ khác nguồn byte
//! (reactor thay vì host blocking). Lỗi mạng/HTTP vẫn là `Result.Err(Error)`; `cap[Net]` được
//! kiểm qua `NetHost::check_net` trước mỗi lần connect.

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;

use vis_diag::COMMAND_CODES;
use vis_syntax::Span;

use crate::builtin::Host;
use crate::interp::{Abort, Trap, R};
use crate::stdlib::{DAU_QUYEN, FRAME_NET_CONNECT, FRAME_NET_WRITE, FRAME_TLS_CONNECT};
use crate::value::Value;

use super::{
    con_lai_ms, doc_header_day_du, ghep_chunked, header_h2, noi_location, tach_url, tim_header,
    DauHttp, GiaoThuc, UrlHttp, MAX_HEADER, MAX_HTTP,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PhuongThuc {
    Get,
    Head,
}

impl PhuongThuc {
    fn ten(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
        }
    }
}

struct HttpAsync {
    phuong_thuc: PhuongThuc,
    /// Frame đang sở hữu — hậu tố khoá connect để hai task cùng host không chung state.
    khoa: u32,
    /// URL ban đầu — dùng nhận diện state còn đúng lời gọi khi Redispatch.
    url_goc: String,
    /// URL hiện tại (đổi khi redirect).
    url: String,
    timeout_ms: u64,
    han: Instant,
    so_chuyen: u8,
    /// Handle kết nối đã sẵn sàng (`C<n>`).
    handle: Option<String>,
    /// Khoá connect đang treo (`N:…`/`T:…`) — cần đóng nếu lỗi trước khi có handle.
    dang_noi: Option<String>,
    da_gui: bool,
    du: Vec<u8>,
    dau: Option<DauHttp>,
    than_bat_dau: usize,
    /// **VS3-W2** — id stream h2 khi handle là `H<n>`; `None` ở đường HTTP/1.1.
    h2_id: Option<u64>,
}

thread_local! {
    /// `frame.current` → state HTTP đang treo. Chỉ một lời gọi HTTP mỗi frame tại một thời điểm.
    static DANG_CHAY: RefCell<HashMap<u32, HttpAsync>> = RefCell::new(HashMap::new());
}

enum Buoc {
    Cho,
    Xong(Value),
    ChuyenHuong,
}

pub(crate) fn http_get_async(
    host: &mut dyn Host,
    url: &str,
    timeout_ms: u64,
    span: Span,
) -> R<Value> {
    chay(host, PhuongThuc::Get, url, timeout_ms, span)
}

pub(crate) fn http_head_async(
    host: &mut dyn Host,
    url: &str,
    timeout_ms: u64,
    span: Span,
) -> R<Value> {
    chay(host, PhuongThuc::Head, url, timeout_ms, span)
}

fn chay(
    host: &mut dyn Host,
    phuong_thuc: PhuongThuc,
    url: &str,
    timeout_ms: u64,
    span: Span,
) -> R<Value> {
    let id = host.sched().borrow().frame.current;
    let cu = DANG_CHAY.with(|m| m.borrow_mut().remove(&id));
    let mut st = match cu {
        Some(s) if s.phuong_thuc == phuong_thuc && s.url_goc == url => {
            // State đang treo đúng lời gọi này: tiếp tục.
            s
        }
        Some(mut s) => {
            // State cũ bị bỏ (vd task bị huỷ rồi frame id tái dùng): đóng tài nguyên.
            s.dong(host);
            HttpAsync::moi(phuong_thuc, url, timeout_ms, id)
        }
        None => HttpAsync::moi(phuong_thuc, url, timeout_ms, id),
    };
    match drive(host, &mut st, span) {
        Ok(Some(v)) => {
            st.dong(host);
            Ok(v)
        }
        Err(Abort::Pending(p)) => {
            DANG_CHAY.with(|m| m.borrow_mut().insert(id, st));
            Err(Abort::Pending(p))
        }
        Err(e) => {
            st.dong(host);
            Err(e)
        }
        Ok(None) => unreachable!("drive chỉ trả Xong/Pending"),
    }
}

impl HttpAsync {
    fn moi(phuong_thuc: PhuongThuc, url: &str, timeout_ms: u64, khoa: u32) -> Self {
        let timeout_ms = timeout_ms.max(1);
        Self {
            phuong_thuc,
            khoa,
            url_goc: url.to_string(),
            url: url.to_string(),
            timeout_ms,
            han: Instant::now() + std::time::Duration::from_millis(timeout_ms),
            so_chuyen: 0,
            handle: None,
            dang_noi: None,
            da_gui: false,
            du: Vec::new(),
            dau: None,
            than_bat_dau: 0,
            h2_id: None,
        }
    }

    fn dat_lai_sau_chuyen_huong(&mut self) {
        self.da_gui = false;
        self.du.clear();
        self.dau = None;
        self.than_bat_dau = 0;
        self.h2_id = None;
    }

    /// Đóng handle/kết nối đang treo; huỷ mọi đăng ký reactor còn lại (timeout/lỗi).
    fn dong(&mut self, host: &mut dyn Host) {
        let backend = host.sched().borrow().frame.backend.clone();
        let Some(b) = backend else {
            self.handle = None;
            self.dang_noi = None;
            return;
        };
        if let Some(h) = self.handle.take() {
            if h.starts_with('H') {
                if let Some(id) = self.h2_id.take() {
                    host.sched().borrow_mut().frame_reactor_h2_cancel(&h, id);
                }
            }
            let wk = format!("{FRAME_NET_WRITE}{h}");
            {
                let mut sc = host.sched().borrow_mut();
                sc.frame_reactor_cancel_handle(&h);
                sc.frame_reactor_cancel_handle(&wk);
            }
            let _ = b.borrow_mut().close(&h);
        }
        if let Some(k) = self.dang_noi.take() {
            host.sched().borrow_mut().frame_reactor_cancel_handle(&k);
            let _ = b.borrow_mut().close(&k);
        }
    }
}

fn drive(host: &mut dyn Host, st: &mut HttpAsync, span: Span) -> R<Option<Value>> {
    'ngoai: loop {
        if st.handle.is_none() {
            let u = match tach_url(&st.url) {
                Ok(u) => u,
                Err(e) => return Ok(Some(loi_http(e))),
            };
            kiem_quyen(host, &u.dia_chi(), span)?;
            let Some(con) = con_lai_ms(st.han) else {
                return Ok(Some(het_gio(st)));
            };
            let key = if u.giao_thuc == GiaoThuc::Https {
                format!("{FRAME_TLS_CONNECT}{}|{}", u.dia_chi(), st.khoa)
            } else {
                format!("{FRAME_NET_CONNECT}{}|{}", u.dia_chi(), st.khoa)
            };
            st.dang_noi = Some(key.clone());
            let v = host.sched().borrow_mut().frame_reactor_poll_io(
                crate::rt::ReactorOp::Read,
                &key,
                0,
                con as i64,
                span,
            )?;
            let Some(h) = nhan_ok(&v) else {
                if la_het_gio(&v) {
                    return Ok(Some(het_gio(st)));
                }
                return Ok(Some(v));
            };
            st.dang_noi = None;
            st.handle = Some(h.to_string());
            st.dat_lai_sau_chuyen_huong();
        }
        let handle = st.handle.clone().expect("handle vừa có");
        if handle.starts_with('H') {
            match drive_h2(host, st, span)? {
                H2Buoc::Xong(v) => return Ok(Some(v)),
                H2Buoc::ChuyenHuong => continue 'ngoai,
            }
        }
        if !st.da_gui {
            let u = match tach_url(&st.url) {
                Ok(u) => u,
                Err(e) => return Ok(Some(loi_http(e))),
            };
            let (auth, cookie) = thong_tin(host, &u);
            ghi_log(host, st.phuong_thuc, &u);
            let req = yeu_cau(st.phuong_thuc, &u.target, &u.authority(), auth, cookie);
            let wk = format!("{FRAME_NET_WRITE}{handle}");
            crate::stdlib::frame_net_set_write(&wk, req.as_bytes());
            let Some(con) = con_lai_ms(st.han) else {
                crate::stdlib::frame_net_take_write(&wk);
                return Ok(Some(het_gio(st)));
            };
            let r = host.sched().borrow_mut().frame_reactor_poll_io(
                crate::rt::ReactorOp::Read,
                &wk,
                req.len(),
                con as i64,
                span,
            );
            crate::stdlib::frame_net_take_write(&wk);
            let v = r?;
            if nhan_ok(&v).is_none() {
                if la_het_gio(&v) {
                    return Ok(Some(het_gio(st)));
                }
                return Ok(Some(v));
            }
            st.da_gui = true;
        }
        loop {
            match kiem_tra(st, host) {
                Ok(Buoc::Xong(v)) => return Ok(Some(v)),
                Ok(Buoc::ChuyenHuong) => continue 'ngoai,
                Ok(Buoc::Cho) => {}
                Err(e) => return Ok(Some(loi_http(e))),
            }
            let Some(con) = con_lai_ms(st.han) else {
                return Ok(Some(het_gio(st)));
            };
            match host.sched().borrow_mut().frame_reactor_poll_bytes(
                &handle,
                16 * 1024,
                con.clamp(1, 2_000) as i64,
                span,
            ) {
                Ok(Ok(bytes)) => {
                    if bytes.is_empty() {
                        return Ok(Some(ket_thuc_eof(st)));
                    }
                    st.du.extend_from_slice(&bytes);
                }
                Ok(Err(e)) if e.contains("quá thời gian") => return Ok(Some(het_gio(st))),
                Ok(Err(e)) => return Ok(Some(loi_http(e))),
                Err(p) => return Err(p),
            }
        }
    }
}

enum H2Buoc {
    Xong(Value),
    ChuyenHuong,
}

/// **VS3-W2** — lái request h2 trên handle `H<n>` bằng meta có cấu trúc.
fn drive_h2(host: &mut dyn Host, st: &mut HttpAsync, span: Span) -> R<H2Buoc> {
    let handle = st.handle.clone().expect("handle h2");
    if st.h2_id.is_none() {
        if con_lai_ms(st.han).is_none() {
            return Ok(H2Buoc::Xong(het_gio(st)));
        }
        let u = match tach_url(&st.url) {
            Ok(u) => u,
            Err(e) => return Ok(H2Buoc::Xong(loi_http(e))),
        };
        let (auth, cookie) = thong_tin(host, &u);
        ghi_log(host, st.phuong_thuc, &u);
        let hs = header_h2(auth.as_deref(), cookie.as_deref());
        let id = match host.sched().borrow_mut().frame_reactor_h2_send(
            &handle,
            st.phuong_thuc.ten(),
            &u.target,
            &hs,
        ) {
            Ok(id) => id,
            Err(e) => return Ok(H2Buoc::Xong(loi_http(e))),
        };
        st.h2_id = Some(id);
        st.da_gui = true;
    }
    let id = st.h2_id.expect("vừa gửi");
    let Some(con) = con_lai_ms(st.han) else {
        host.sched()
            .borrow_mut()
            .frame_reactor_h2_cancel(&handle, id);
        return Ok(H2Buoc::Xong(het_gio(st)));
    };
    // Một lần poll h2 bao cả response (nhiều frame), không phải một chunk như h1;
    // dùng trọn thời gian còn lại để không cắt body lớn giữa chừng.
    let kq =
        host.sched()
            .borrow_mut()
            .frame_reactor_h2_poll(&handle, id, con.max(1) as i64, span)?;
    let resp = match kq {
        Ok(r) => r,
        Err(e) if e.contains("thời gian") => {
            host.sched()
                .borrow_mut()
                .frame_reactor_h2_cancel(&handle, id);
            return Ok(H2Buoc::Xong(het_gio(st)));
        }
        Err(e) => {
            return Ok(H2Buoc::Xong(loi_http(e)));
        }
    };
    if let Ok(u) = tach_url(&st.url) {
        luu_set_cookie(host, &u, &resp.headers);
    }
    if st.phuong_thuc == PhuongThuc::Head {
        return Ok(H2Buoc::Xong(ket_qua_head_meta(resp.status, &resp.headers)));
    }
    if (300..400).contains(&resp.status) {
        let Some(loc) = tim_header_gia_tri(&resp.headers, "location") else {
            return Ok(H2Buoc::Xong(loi_http(format!("HTTP {}", resp.status))));
        };
        let u = match tach_url(&st.url) {
            Ok(u) => u,
            Err(e) => return Ok(H2Buoc::Xong(loi_http(e))),
        };
        let url = match noi_location(&u, &loc) {
            Ok(Some(u)) => u,
            Ok(None) => return Ok(H2Buoc::Xong(loi_http(format!("HTTP {}", resp.status)))),
            Err(e) => return Ok(H2Buoc::Xong(loi_http(e))),
        };
        st.so_chuyen += 1;
        if st.so_chuyen > 5 {
            return Ok(H2Buoc::Xong(loi_http(
                "quá 5 lần chuyển hướng HTTP (tối đa 5)".to_string(),
            )));
        }
        st.dong(host);
        st.url = url;
        st.dat_lai_sau_chuyen_huong();
        return Ok(H2Buoc::ChuyenHuong);
    }
    if !(200..300).contains(&resp.status) {
        return Ok(H2Buoc::Xong(loi_http(format!("HTTP {}", resp.status))));
    }
    Ok(H2Buoc::Xong(ok_body(&resp.body)))
}

fn tim_header_gia_tri(headers: &[(String, String)], ten: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(ten))
        .map(|(_, v)| v.clone())
}

fn ket_qua_head_meta(status: u16, headers: &[(String, String)]) -> Value {
    let headers = Value::Map(
        headers
            .iter()
            .map(|(k, v)| (Value::Str(k.clone()), Value::Str(v.clone())))
            .collect(),
    );
    Value::ok(Value::Map(vec![
        (Value::Str("status".to_string()), Value::Int(status as i64)),
        (Value::Str("headers".to_string()), headers),
    ]))
}

fn kiem_tra(st: &mut HttpAsync, host: &mut dyn Host) -> Result<Buoc, String> {
    if st.dau.is_none() {
        let Some(i) = tim_header(&st.du) else {
            if st.du.len() > MAX_HEADER {
                return Err(format!(
                    "phản hồi HTTP header quá lớn (> {MAX_HEADER} byte)"
                ));
            }
            return Ok(Buoc::Cho);
        };
        let d = doc_header_day_du(&st.du[..i])?;
        if d.cl.map(|n| n > MAX_HTTP).unwrap_or(false) {
            return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
        }
        if let Ok(u) = tach_url(&st.url) {
            luu_set_cookie(host, &u, &d.headers);
        }
        st.than_bat_dau = i + 4;
        st.dau = Some(d);
    }
    let d = st.dau.as_ref().expect("vừa parse");
    let status = d.status;
    let location = d.location.clone();
    let chunked = d.chunked;
    let cl = d.cl;
    if st.phuong_thuc == PhuongThuc::Head {
        return Ok(Buoc::Xong(ket_qua_head(d)));
    }
    if (300..400).contains(&status) {
        let Some(loc) = location else {
            return Ok(Buoc::Xong(loi_http(format!("HTTP {status}"))));
        };
        let u = tach_url(&st.url)?;
        let Some(url) = noi_location(&u, &loc)? else {
            return Ok(Buoc::Xong(loi_http(format!("HTTP {status}"))));
        };
        st.so_chuyen += 1;
        if st.so_chuyen > 5 {
            return Err("quá 5 lần chuyển hướng HTTP (tối đa 5)".to_string());
        }
        st.dong(host);
        st.url = url;
        st.dat_lai_sau_chuyen_huong();
        return Ok(Buoc::ChuyenHuong);
    }
    if !(200..300).contains(&status) {
        return Ok(Buoc::Xong(loi_http(format!("HTTP {status}"))));
    }
    let h = st.than_bat_dau;
    if chunked {
        if let Some((body, _)) = ghep_chunked(&st.du[h..])? {
            return Ok(Buoc::Xong(ok_body(&body)));
        }
        if st.du.len().saturating_sub(h) > MAX_HTTP {
            return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
        }
    } else if let Some(n) = cl {
        if st.du.len() >= h + n {
            return Ok(Buoc::Xong(ok_body(&st.du[h..h + n])));
        }
        if st.du.len().saturating_sub(h) > MAX_HTTP {
            return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
        }
    } else if st.du.len().saturating_sub(h) > MAX_HTTP {
        return Err(format!("phản hồi vượt giới hạn {MAX_HTTP} byte (8 MiB)"));
    }
    Ok(Buoc::Cho)
}

fn ket_thuc_eof(st: &HttpAsync) -> Value {
    let Some(d) = st.dau.as_ref() else {
        return loi_http("phản hồi HTTP không đủ header (kết nối đóng sớm)");
    };
    if st.phuong_thuc == PhuongThuc::Head {
        return ket_qua_head(d);
    }
    let h = st.than_bat_dau;
    if d.chunked {
        return match ghep_chunked(&st.du[h..]) {
            Ok(Some((body, _))) => ok_body(&body),
            _ => loi_http("chunked: kết nối đóng khi chưa đủ chunk"),
        };
    }
    if let Some(n) = d.cl {
        if st.du.len() < h + n {
            return loi_http(format!(
                "Content-Length {n} nhưng mới đọc {} byte (kết nối đóng sớm)",
                st.du.len().saturating_sub(h)
            ));
        }
        return ok_body(&st.du[h..h + n]);
    }
    ok_body(&st.du[h..])
}

fn ket_qua_head(d: &DauHttp) -> Value {
    let headers = Value::Map(
        d.headers
            .iter()
            .map(|(k, v)| (Value::Str(k.clone()), Value::Str(v.clone())))
            .collect(),
    );
    Value::ok(Value::Map(vec![
        (
            Value::Str("status".to_string()),
            Value::Int(d.status as i64),
        ),
        (Value::Str("headers".to_string()), headers),
    ]))
}

fn ok_body(bytes: &[u8]) -> Value {
    Value::ok(Value::Str(String::from_utf8_lossy(bytes).into_owned()))
}

fn loi_http(msg: impl Into<String>) -> Value {
    Value::err(Value::ban_ghi_error(msg, None))
}

fn het_gio(st: &HttpAsync) -> Value {
    if let Some(k) = st.dang_noi.as_deref() {
        if let Some(dia_chi) = k.strip_prefix(FRAME_TLS_CONNECT) {
            let dia_chi = dia_chi.split_once('|').map(|(a, _)| a).unwrap_or(dia_chi);
            return loi_http(format!(
                "quá thời gian TLS handshake tới `{dia_chi}` sau {} ms",
                st.timeout_ms
            ));
        }
    }
    let authority = tach_url(&st.url)
        .map(|u| u.authority())
        .unwrap_or_else(|_| st.url.clone());
    loi_http(format!(
        "quá thời gian chờ HTTP từ `{authority}` sau {} ms",
        st.timeout_ms
    ))
}

fn nhan_ok(v: &Value) -> Option<&str> {
    if let Value::Enum(ea, va, ag) = v {
        if ea == "Result" && va == "Ok" {
            if let Some(Value::Str(s)) = ag.first() {
                return Some(s);
            }
        }
    }
    None
}

/// Bóc `message` của `Err(Error)` để nhận diện timeout do lớp reactor trả.
fn la_het_gio(v: &Value) -> bool {
    if let Value::Enum(ea, va, ag) = v {
        if ea == "Result" && va == "Err" {
            if let Some(Value::Record(ten, fields)) = ag.first() {
                if ten == "Error" {
                    for (k, val) in fields {
                        if k == "message" {
                            if let Value::Str(s) = val {
                                return s.contains("quá thời gian");
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn kiem_quyen(host: &mut dyn Host, dia_chi: &str, span: Span) -> R<()> {
    let r = host.net().map(|n| n.check_net(dia_chi)).unwrap_or(Ok(()));
    match r {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = e.strip_prefix(DAU_QUYEN).unwrap_or(&e).to_string();
            Err(Abort::Trap(Trap::new(&COMMAND_CODES[3], msg, span)))
        }
    }
}

fn thong_tin(host: &mut dyn Host, u: &UrlHttp) -> (Option<String>, Option<String>) {
    let auth = u.authorization();
    let path = u.target.split('?').next().unwrap_or("/");
    let secure = u.giao_thuc == GiaoThuc::Https;
    let cookie = host
        .net()
        .and_then(|n| n.cookie_jar().and_then(|j| j.header(&u.host, path, secure)));
    (auth, cookie)
}

/// **VS3-W2** — lưu `Set-Cookie` của response vào jar async (giống `xu_ly_h2` sync).
fn luu_set_cookie(host: &mut dyn Host, u: &UrlHttp, headers: &[(String, String)]) {
    let set_cookie: Vec<String> = headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.clone())
        .collect();
    if set_cookie.is_empty() {
        return;
    }
    let path = u.target.split('?').next().unwrap_or("/");
    if let Some(n) = host.net() {
        if let Some(j) = n.cookie_jar() {
            j.nhan_set_cookie(&u.host, path, &set_cookie);
        }
    }
}

fn ghi_log(host: &mut dyn Host, phuong_thuc: PhuongThuc, u: &UrlHttp) {
    if let Some(n) = host.net() {
        n.ghi_nhat_ky(&format!(
            "NET {} {} {}{}",
            u.giao_thuc.ten(),
            phuong_thuc.ten(),
            u.authority(),
            u.target
        ));
    }
}

fn yeu_cau(
    phuong_thuc: PhuongThuc,
    target: &str,
    authority: &str,
    auth: Option<String>,
    cookie: Option<String>,
) -> String {
    let mut req = format!(
        "{} {target} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n",
        phuong_thuc.ten()
    );
    if let Some(a) = auth {
        req.push_str("Authorization: ");
        req.push_str(&a);
        req.push_str("\r\n");
    }
    if let Some(c) = cookie {
        req.push_str("Cookie: ");
        req.push_str(&c);
        req.push_str("\r\n");
    }
    req.push_str("\r\n");
    req
}
