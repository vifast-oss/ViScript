// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-4b (VD-271)** — HTTP/1.1 **keep-alive**: tái dùng kết nối thật trong test, không Internet.
//!
//! Ca phủ: 2 request cùng kết nối; server yêu cầu `close`; server đóng giữa chừng (client thử lại
//! một lần trên kết nối mới); timeout trên kết nối idle; redirect cùng host dùng lại kết nối;
//! redirect khác host không rò/rò kết nối.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use vis_cli::net::Net;
use vis_core::http_client::http_get;

/// Đọc một request tới `\r\n\r\n`; `None` = EOF/lỗi.
fn doc_yeu_cau(s: &mut TcpStream) -> Option<String> {
    let _ = s.set_read_timeout(Some(Duration::from_secs(3)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    while buf.len() < 65_536 && !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        match s.read(&mut tmp) {
            Ok(0) => return None,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => return None,
        }
    }
    String::from_utf8(buf).ok()
}

fn dap_ung(status: &str, body: &str, ket_noi: &str, them: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: {ket_noi}\r\n{them}\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn ghi(s: &mut TcpStream, du: &[u8]) {
    let _ = s.write_all(du);
    let _ = s.flush();
}

struct MayChu {
    dia_chi: String,
    so_ket_noi: Arc<AtomicUsize>,
    log: Arc<Mutex<Vec<String>>>,
    tay: Option<std::thread::JoinHandle<()>>,
}

impl MayChu {
    fn dung(&mut self) {
        if let Some(t) = self.tay.take() {
            t.join().expect("join server");
        }
    }
}

/// Server loopback một luồng; `xu_ly` nhận `(kết nối thứ i, request thứ j, stream, request)`.
/// `so_yeu_cau[i]` = số request server phục vụ trên kết nối thứ i rồi đóng.
fn may_chu(
    so_yeu_cau: Vec<usize>,
    xu_ly: impl Fn(usize, usize, &mut TcpStream, &str) + Send + 'static,
) -> MayChu {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let dia_chi = l.local_addr().expect("local_addr").to_string();
    let dem = Arc::new(AtomicUsize::new(0));
    let log = Arc::new(Mutex::new(Vec::new()));
    let dem2 = Arc::clone(&dem);
    let log2 = Arc::clone(&log);
    let tay = std::thread::spawn(move || {
        for (i, so) in so_yeu_cau.into_iter().enumerate() {
            let Ok((mut s, _)) = l.accept() else { return };
            dem2.fetch_add(1, Ordering::SeqCst);
            for j in 0..so {
                let Some(req) = doc_yeu_cau(&mut s) else {
                    break;
                };
                log2.lock().expect("log").push(req.clone());
                xu_ly(i, j, &mut s, &req);
            }
            let _ = s.shutdown(Shutdown::Both);
        }
    });
    MayChu {
        dia_chi,
        so_ket_noi: dem,
        log,
        tay: Some(tay),
    }
}

#[test]
fn hai_request_cung_mot_ket_noi() {
    let mut s = may_chu(vec![2], |_, j, sock, req| {
        assert!(
            req.contains("Connection: keep-alive\r\n"),
            "request thiếu keep-alive: {req:?}"
        );
        let body = if j == 0 { "mot" } else { "hai" };
        ghi(sock, &dap_ung("200 OK", body, "keep-alive", ""));
    });
    let mut net = Net::moi(false);
    let a = format!("http://{}/mot", s.dia_chi);
    let b = format!("http://{}/hai", s.dia_chi);
    assert_eq!(http_get(&mut net, &a, 3_000).expect("request 1"), "mot");
    assert_eq!(http_get(&mut net, &b, 3_000).expect("request 2"), "hai");
    s.dung();
    assert_eq!(
        s.so_ket_noi.load(Ordering::SeqCst),
        1,
        "hai request phải dùng chung một kết nối"
    );
    assert_eq!(s.log.lock().expect("log").len(), 2);
}

#[test]
fn server_yeu_cau_close_thi_mo_ket_noi_moi() {
    let mut s = may_chu(vec![1, 1], |i, _, sock, _| {
        let body = if i == 0 { "mot" } else { "hai" };
        ghi(sock, &dap_ung("200 OK", body, "close", ""));
        let _ = sock.shutdown(Shutdown::Both);
    });
    let mut net = Net::moi(false);
    let a = format!("http://{}/mot", s.dia_chi);
    let b = format!("http://{}/hai", s.dia_chi);
    assert_eq!(http_get(&mut net, &a, 3_000).expect("request 1"), "mot");
    assert_eq!(http_get(&mut net, &b, 3_000).expect("request 2"), "hai");
    s.dung();
    assert_eq!(
        s.so_ket_noi.load(Ordering::SeqCst),
        2,
        "Connection: close phải buộc kết nối mới"
    );
}

/// Server trả lời xong rồi đóng kết nối **dù** header nói keep-alive: client phải phát hiện kết
/// nối idle đã chết và thử lại một lần trên kết nối mới.
#[test]
fn server_dong_giua_chung_thi_thu_lai() {
    let mut s = may_chu(vec![1, 1], |i, _, sock, _| {
        let body = if i == 0 { "mot" } else { "hai" };
        ghi(sock, &dap_ung("200 OK", body, "keep-alive", ""));
        if i == 0 {
            let _ = sock.shutdown(Shutdown::Both);
        }
    });
    let mut net = Net::moi(false);
    let a = format!("http://{}/mot", s.dia_chi);
    let b = format!("http://{}/hai", s.dia_chi);
    assert_eq!(http_get(&mut net, &a, 3_000).expect("request 1"), "mot");
    assert_eq!(
        http_get(&mut net, &b, 3_000).expect("request 2 phải thử lại"),
        "hai"
    );
    s.dung();
    assert_eq!(s.so_ket_noi.load(Ordering::SeqCst), 2);
}

/// Kết nối idle bị server treo ⇒ timeout; kết nối đó **không** được trả lại pool và request sau
/// phải mở kết nối mới.
#[test]
fn timeout_tren_ket_noi_idle_khong_lam_hong_pool() {
    let mut s = may_chu(vec![2, 1], |i, j, sock, _| {
        if i == 0 {
            if j == 0 {
                ghi(sock, &dap_ung("200 OK", "mot", "keep-alive", ""));
            } else {
                // Request thứ 2 trên kết nối cũ: treo quá timeout của client.
                std::thread::sleep(Duration::from_millis(1_000));
                let _ = sock.shutdown(Shutdown::Both);
            }
        } else {
            ghi(sock, &dap_ung("200 OK", "ba", "close", ""));
        }
    });
    let mut net = Net::moi(false);
    let a = format!("http://{}/mot", s.dia_chi);
    let b = format!("http://{}/treo", s.dia_chi);
    let c = format!("http://{}/ba", s.dia_chi);
    assert_eq!(http_get(&mut net, &a, 3_000).expect("request 1"), "mot");
    let e = http_get(&mut net, &b, 250).expect_err("phải timeout");
    assert!(e.contains("quá thời gian"), "lỗi phải nêu timeout: {e}");
    assert_eq!(
        http_get(&mut net, &c, 3_000).expect("request sau timeout"),
        "ba"
    );
    s.dung();
    assert_eq!(s.so_ket_noi.load(Ordering::SeqCst), 2);
}

/// Chunked + trailer vẫn tái dùng được: trailer phải được đọc hết trước khi trả kết nối về pool.
#[test]
fn chunked_trailer_roi_moi_tai_su_dung() {
    let mut s = may_chu(vec![2], |_, j, sock, _| {
        if j == 0 {
            ghi(
                sock,
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\n\r\n3\r\nabc\r\n0\r\nX-T: 1\r\n\r\n",
            );
        } else {
            ghi(sock, &dap_ung("200 OK", "hai", "keep-alive", ""));
        }
    });
    let mut net = Net::moi(false);
    let a = format!("http://{}/chunk", s.dia_chi);
    let b = format!("http://{}/sau", s.dia_chi);
    assert_eq!(http_get(&mut net, &a, 3_000).expect("chunked"), "abc");
    assert_eq!(http_get(&mut net, &b, 3_000).expect("sau chunked"), "hai");
    s.dung();
    assert_eq!(s.so_ket_noi.load(Ordering::SeqCst), 1);
}

/// HEAD cũng tái dùng kết nối (không đọc thân, giữ đúng hành vi không redirect).
#[test]
fn head_hai_lan_cung_mot_ket_noi() {
    let mut s = may_chu(vec![2], |_, j, sock, req| {
        assert!(req.starts_with("HEAD /x "), "phải gửi HEAD: {req:?}");
        let them = if j == 0 {
            "X-Lan: 1\r\n"
        } else {
            "X-Lan: 2\r\n"
        };
        ghi(
            sock,
            format!("HTTP/1.1 204 No Content\r\nConnection: keep-alive\r\n{them}\r\n").as_bytes(),
        );
    });
    let mut net = Net::moi(false);
    let u = format!("http://{}/x", s.dia_chi);
    let (st1, h1) = vis_core::http_client::http_head(&mut net, &u, 3_000).expect("HEAD 1");
    let (st2, h2) = vis_core::http_client::http_head(&mut net, &u, 3_000).expect("HEAD 2");
    assert_eq!((st1, st2), (204, 204));
    assert!(h1.iter().any(|(k, v)| k == "x-lan" && v == "1"));
    assert!(h2.iter().any(|(k, v)| k == "x-lan" && v == "2"));
    s.dung();
    assert_eq!(s.so_ket_noi.load(Ordering::SeqCst), 1);
}

/// Redirect **cùng host** phải dùng lại đúng kết nối, không mở thêm kết nối cho mỗi hop.
#[test]
fn redirect_cung_host_dung_lai_ket_noi() {
    let mut s = may_chu(vec![3], |_, j, sock, req| match j {
        0 => {
            assert!(req.starts_with("GET /a "), "request 0: {req:?}");
            ghi(
                sock,
                &dap_ung("302 Found", "", "keep-alive", "Location: /b\r\n"),
            );
        }
        1 => {
            assert!(req.starts_with("GET /b "), "request 1: {req:?}");
            ghi(
                sock,
                &dap_ung("302 Found", "", "keep-alive", "Location: /c\r\n"),
            );
        }
        _ => {
            assert!(req.starts_with("GET /c "), "request 2: {req:?}");
            ghi(sock, &dap_ung("200 OK", "dich", "keep-alive", ""));
        }
    });
    let mut net = Net::moi(false);
    let u = format!("http://{}/a", s.dia_chi);
    assert_eq!(http_get(&mut net, &u, 3_000).expect("redirect"), "dich");
    s.dung();
    assert_eq!(
        s.so_ket_noi.load(Ordering::SeqCst),
        1,
        "redirect cùng host không được mở thêm kết nối"
    );
    assert_eq!(s.log.lock().expect("log").len(), 3);
}

/// Redirect **khác host**: mỗi host một kết nối; quay lại host A phải tái dùng kết nối A đã pool,
/// không rò thêm kết nối.
#[test]
fn redirect_khac_host_khong_ro_ket_noi() {
    let mut b = may_chu(vec![1], |_, _, sock, _| {
        ghi(sock, &dap_ung("200 OK", "B", "close", ""));
    });
    let location = format!("http://{}/b", b.dia_chi);
    let mut a = may_chu(vec![2], move |_, j, sock, _| {
        if j == 0 {
            ghi(
                sock,
                &dap_ung(
                    "302 Found",
                    "",
                    "keep-alive",
                    &format!("Location: {location}\r\n"),
                ),
            );
        } else {
            ghi(sock, &dap_ung("200 OK", "A2", "close", ""));
        }
    });
    let mut net = Net::moi(false);
    assert_eq!(
        http_get(&mut net, &format!("http://{}/a", a.dia_chi), 3_000).expect("qua B"),
        "B"
    );
    assert_eq!(
        http_get(&mut net, &format!("http://{}/x", a.dia_chi), 3_000).expect("lại A"),
        "A2"
    );
    a.dung();
    b.dung();
    assert_eq!(a.so_ket_noi.load(Ordering::SeqCst), 1, "A phải tái dùng");
    assert_eq!(b.so_ket_noi.load(Ordering::SeqCst), 1);
}
