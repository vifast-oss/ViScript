// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — parity `interp ↔ VM` cho **10 hàm `net`** + **4 hàm `http`** trên
//! loopback/server Rust thật (cổng ephemeral, timeout, cổng đóng).
//!
//! **ST-316d (VD-292 §4.1)** — tách từ `misc_api_it/host_parity.rs` (cận trần R1); nội dung và
//! assertion giữ nguyên 100 %.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{chuan_hoa_cong, hai_duong, hai_duong_chuan_hoa, tam};
// ===== net: 10 hàm (VD-291 §3.4) =====

/// Trọn 10 hàm `net` trên loopback thật **trong cùng tiến trình `vis`**: `listen` (cổng `:0`),
/// `local_addr`, `connect`, `accept`, `read`, `write`, `close`, `accept_timeout` 80 ms,
/// `close_listen`, `connect_timeout`; mọi nhánh `Err` in **nguyên văn `e.message`** (F2 review).
/// Kết nối trước `accept` dựa vào backlog TCP của kernel nên **không cần** canh thời gian.
pub(super) const SRC_NET: &str = r#"use std.net

fn main():
    match net.listen("127.0.0.1:0"):
        Err(e): print("listen=" + e.message)
        Ok(l):
            match net.local_addr(l):
                Ok(a):
                    print("addr=" + a)
                    match net.listen(a):
                        Ok(_): print("double=KHONG-DUOC-PHEP")
                        Err(e): print("double=" + e.message)
                    match net.accept_timeout(l, 80):
                        Ok(_): print("accept_timeout=KHONG-DUOC-PHEP")
                        Err(e): print("accept_timeout=" + e.message)
                    match net.connect(a):
                        Ok(c):
                            match net.accept(l):
                                Ok(s):
                                    match net.write(c, "ping"):
                                        Ok(_): print("ghi-c ok")
                                        Err(e): print("ghi-c=" + e.message)
                                    match net.read(s, 64, 500):
                                        Ok(x): print("server doc " + x)
                                        Err(e): print("server doc=" + e.message)
                                    match net.write(s, "pong"):
                                        Ok(_): print("ghi-s ok")
                                        Err(e): print("ghi-s=" + e.message)
                                    match net.read(c, 64, 500):
                                        Ok(x): print("client doc " + x)
                                        Err(e): print("client doc=" + e.message)
                                    match net.close(c):
                                        Ok(_): print("dong-c ok")
                                        Err(e): print("dong-c=" + e.message)
                                    match net.read(c, 64, 100):
                                        Ok(_): print("doc-sau-dong=KHONG-DUOC-PHEP")
                                        Err(e): print("doc-sau-dong=" + e.message)
                                    match net.write(c, "x"):
                                        Ok(_): print("ghi-sau-dong=KHONG-DUOC-PHEP")
                                        Err(e): print("ghi-sau-dong=" + e.message)
                                    match net.close(s):
                                        Ok(_): print("dong-s ok")
                                        Err(e): print("dong-s=" + e.message)
                                Err(e): print("accept=" + e.message)
                        Err(e): print("connect-self=" + e.message)
                    match net.close_listen(l):
                        Ok(_): print("dong-l ok")
                        Err(e): print("dong-l=" + e.message)
                    match net.close_listen(l):
                        Ok(_): print("dong-l2=KHONG-DUOC-PHEP")
                        Err(e): print("dong-l2=" + e.message)
                    match net.accept_timeout(l, 80):
                        Ok(_): print("accept-sau-dong=KHONG-DUOC-PHEP")
                        Err(e): print("accept-sau-dong=" + e.message)
                    match net.local_addr(l):
                        Ok(_): print("local-sau-dong=KHONG-DUOC-PHEP")
                        Err(e): print("local-sau-dong=" + e.message)
                Err(e): print("local_addr=" + e.message)
    match net.read("C99", 8, 50):
        Ok(_): print("read=KHONG-DUOC-PHEP")
        Err(e): print("read=" + e.message)
    match net.write("C99", "x"):
        Ok(_): print("write=KHONG-DUOC-PHEP")
        Err(e): print("write=" + e.message)
    match net.close("C99"):
        Ok(_): print("close=KHONG-DUOC-PHEP")
        Err(e): print("close=" + e.message)
    match net.connect("__CLOSED__"):
        Ok(_): print("connect=KHONG-DUOC-PHEP")
        Err(e): print("connect=" + e.message)
    match net.connect_timeout("__CLOSED__", 120):
        Ok(_): print("connect_timeout=KHONG-DUOC-PHEP")
        Err(e): print("connect_timeout=" + e.message)
"#;

/// **ST-315d** — 10 hàm `net`, in đầy đủ thông điệp lỗi; cổng ephemeral được chuẩn hoá `<P>`
/// nhưng phần còn lại của `e.message` phải khớp **từng byte** giữa hai engine.
#[test]
fn net_loopback_parity() {
    let d = tam("net");
    // Cổng "đóng" thật: bind `:0` rồi thả (không dùng cổng cố định 1 — F8 review).
    let l = TcpListener::bind("127.0.0.1:0").expect("bind cổng đóng");
    let dong_addr = l.local_addr().expect("addr cổng đóng").to_string();
    drop(l);
    let src = SRC_NET.replace("__CLOSED__", &dong_addr);
    let (out, err) = hai_duong_chuan_hoa(&d, "net", &src, || {}, chuan_hoa_cong);
    let mong = "addr=127.0.0.1:<P>\n\
         double=cổng `127.0.0.1:<P>` đang bận (đã có tiến trình listen)\n\
         accept_timeout=quá thời gian chờ kết nối sau 80 ms\n\
         ghi-c ok\n\
         server doc ping\n\
         ghi-s ok\n\
         client doc pong\n\
         dong-c ok\n\
         doc-sau-dong=tay mạng `C1` không tồn tại (chưa nối hoặc đã `close`)\n\
         ghi-sau-dong=tay mạng `C1` không tồn tại (chưa nối hoặc đã `close`)\n\
         dong-s ok\n\
         dong-l ok\n\
         dong-l2=tay mạng `L1` không tồn tại (chưa nối/listen hoặc đã `close`)\n\
         accept-sau-dong=tay listener `L1` không tồn tại (đã close hoặc chưa listen)\n\
         local-sau-dong=tay mạng `L1` không tồn tại (chưa nối hoặc đã `close`)\n\
         read=tay mạng `C99` không tồn tại (chưa nối hoặc đã `close`)\n\
         write=tay mạng `C99` không tồn tại (chưa nối hoặc đã `close`)\n\
         close=tay mạng `C99` không tồn tại (chưa nối/listen hoặc đã `close`)\n";
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(dong.len(), 20, "net: {out}\nstderr: {err}");
    assert_eq!(
        dong[..18].join("\n") + "\n",
        mong,
        "net: {out}\nstderr: {err}"
    );
    // Hai dòng cuối là thông điệp "bị từ chối" — câu chữ đuôi khác nhau theo OS, nhưng **phải
    // nêu đúng cổng đã đóng** và không được là nhánh Ok.
    for (i, tien_to) in [(18, "connect="), (19, "connect_timeout=")] {
        assert!(dong[i].starts_with(tien_to), "net dòng {i}: {}", dong[i]);
        assert!(
            dong[i].contains("127.0.0.1:<P>"),
            "net dòng {i} phải nêu cổng đích: {}",
            dong[i]
        );
        assert!(
            !dong[i].contains("KHONG-DUOC-PHEP"),
            "net dòng {i} phải là Err: {}",
            dong[i]
        );
    }
    // **ST-321c**: `ECONNREFUSED` có mã errno khác nhau giữa các OS — Linux = 111,
    // macOS/BSD = 61. Cả hai engine (interp lẫn VM) đều in **nguyên văn errno thật** nên parity
    // interp↔VM vẫn được so từng byte ở [`hai_duong_chuan_hoa`]; chỉ hằng số kỳ vọng trong test
    // cần chọn theo `target_os`. Không đụng runtime, không đổi 18 assert phía trên.
    #[cfg(unix)]
    {
        #[cfg(target_os = "linux")]
        let errno = 111;
        #[cfg(not(target_os = "linux"))]
        let errno = 61;
        assert_eq!(
            &dong[18..],
            [
                format!("connect=bị từ chối kết nối tới `127.0.0.1:<P>`: Connection refused (os error {errno})"),
                format!("connect_timeout=bị từ chối kết nối tới `127.0.0.1:<P>`: Connection refused (os error {errno})"),
            ],
            "net từ chối (Unix): {out}"
        );
    }
}

/// Server TCP loopback phục vụ `n` kết nối: đọc tới 64 byte, ghi `hi`, đóng chiều ghi.
/// `stop` + hạn 8 s giữ `join` **có hạn** (không treo khi client lỗi vì không bao giờ nối).
fn tcp_server(
    n: usize,
) -> (
    String,
    Arc<AtomicBool>,
    std::thread::JoinHandle<Vec<String>>,
) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = l.local_addr().expect("local_addr").to_string();
    l.set_nonblocking(true).expect("nonblocking loopback");
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    let h = std::thread::spawn(move || {
        let han = Instant::now() + Duration::from_secs(8);
        let mut nhan = Vec::new();
        while nhan.len() < n && !stop2.load(Ordering::Relaxed) && Instant::now() < han {
            match l.accept() {
                Ok((mut s, _)) => {
                    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
                    // Client gửi 3–4 byte một lần ⇒ đọc **một** khối là đủ; đừng chờ thêm
                    // (mỗi vòng chờ thêm 500 ms chỉ làm test chậm vô ích).
                    let mut buf = [0u8; 64];
                    let k = s.read(&mut buf).unwrap_or(0);
                    nhan.push(String::from_utf8_lossy(&buf[..k]).to_string());
                    let _ = s.write_all(b"hi");
                    let _ = s.shutdown(Shutdown::Write);
                    let mut rac = [0u8; 64];
                    while matches!(s.read(&mut rac), Ok(k) if k > 0) {}
                }
                // **R-Win2**: `accept` có thể trả lỗi tạm (`ConnectionAborted`/`Reset`/`Interrupted`)
                // khi client trước vừa đóng trên Windows. Trước đây `break` giết server ⇒ kết nối
                // kế tiếp bị RST (`os error 10053`). Loopback: bỏ qua lỗi tạm, chờ tới hạn 8 s.
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        nhan
    });
    (addr, stop, h)
}

pub(super) const SRC_NET_CLIENT: &str = r#"use std.net

fn main():
    match net.connect("__A__"):
        Err(e): print("connect=" + e.message)
        Ok(t):
            match net.write(t, "ping"):
                Ok(_): print("ghi1 ok")
                Err(e): print("ghi1=" + e.message)
            match net.read(t, 64, 2000):
                Ok(s): print("doc1=" + s)
                Err(e): print("doc1=" + e.message)
            match net.close(t):
                Ok(_): print("dong1 ok")
                Err(e): print("dong1=" + e.message)
    match net.connect_timeout("__A__", 2000):
        Err(e): print("ct=" + e.message)
        Ok(t):
            match net.write(t, "bye"):
                Ok(_): print("ghi2 ok")
                Err(e): print("ghi2=" + e.message)
            match net.read(t, 64, 2000):
                Ok(s): print("doc2=" + s)
                Err(e): print("doc2=" + e.message)
            match net.close(t):
                Ok(_): print("dong2 ok")
                Err(e): print("dong2=" + e.message)
"#;

/// **ST-315d** — `net.connect` + `net.connect_timeout` + `net.read`/`write`/`close` tới
/// **server Rust thật** (thread trong test, `TcpListener` loopback). Server ghi lại đúng byte
/// client gửi ⇒ chứng minh chiều ghi của cả hai engine, không chỉ tự nói với mình.
#[test]
fn net_client_toi_server_rust_parity() {
    let (addr, stop, h) = tcp_server(4);
    let d = tam("net-rust");
    let src = SRC_NET_CLIENT.replace("__A__", &addr);
    let (out, err) = hai_duong(&d, "net-rust", &src);
    assert_eq!(
        out, "ghi1 ok\ndoc1=hi\ndong1 ok\nghi2 ok\ndoc2=hi\ndong2 ok\n",
        "net client: {out}\nstderr: {err}"
    );
    stop.store(true, Ordering::Relaxed);
    let nhan = h.join().expect("join server TCP");
    assert_eq!(
        nhan,
        ["ping", "bye", "ping", "bye"],
        "server Rust phải nhận đúng `ping`/`bye` của interp rồi VM"
    );
}

// ===== http: 4 hàm (VD-291 §3.4) =====

/// Dựng phản hồi HTTP/1.1 với `Content-Length` **tường minh** + một header phụ.
fn dap_ung(status: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut r = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\n{extra}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    r.extend_from_slice(body);
    r
}

/// Server HTTP/1.1 tối giản loopback: trả theo **đường dẫn** request, ghi lại **dòng request**
/// từng kết nối. `/cham*` ngủ 300 ms để client timeout 80 ms. `stop` + hạn 8 s giữ `join` có hạn.
fn http_server(
    n: usize,
) -> (
    String,
    Arc<AtomicBool>,
    std::thread::JoinHandle<Vec<String>>,
) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind http");
    let addr = l.local_addr().expect("local_addr http").to_string();
    l.set_nonblocking(true).expect("nonblocking http");
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = Arc::clone(&stop);
    let h = std::thread::spawn(move || {
        let han = Instant::now() + Duration::from_secs(8);
        let mut thay = Vec::new();
        let mut tra_loi = Vec::new();
        while thay.len() < n && !stop2.load(Ordering::Relaxed) && Instant::now() < han {
            match l.accept() {
                Ok((mut s, _)) => {
                    // **Đọc request ngay trong vòng accept** (nhanh) rồi mới trả lời ở thread
                    // riêng: endpoint `/cham` ngủ 300 ms; nếu ngủ ngay trong vòng accept thì kết
                    // nối kế tiếp còn trong backlog có thể bị kernel huỷ khi client kịp thoát
                    // (ca `head_timeout` 80 ms) ⇒ mất dòng request của engine sau.
                    let _ = s.set_read_timeout(Some(Duration::from_millis(2_000)));
                    let mut req = Vec::new();
                    let mut buf = [0u8; 4096];
                    while req.len() < 65_536 && !req.windows(4).any(|w| w == b"\r\n\r\n") {
                        match s.read(&mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(k) => req.extend_from_slice(&buf[..k]),
                        }
                    }
                    let dong = String::from_utf8_lossy(&req)
                        .lines()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    let duong_dan = dong.split_whitespace().nth(1).unwrap_or("").to_string();
                    thay.push(dong);
                    let cham = matches!(duong_dan.as_str(), "/cham" | "/cham-head");
                    let phan = match duong_dan.as_str() {
                        "/ok" => dap_ung("200 OK", "xin chào".as_bytes(), "X-Test: yes"),
                        "/head" => {
                            b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\nX-Test: yes\r\n\r\n"
                                .to_vec()
                        }
                        "/nhanh" => dap_ung("200 OK", b"nhanh", "X-Test: yes"),
                        "/head-nhanh" => dap_ung("200 OK", b"", "X-Test: fast"),
                        "/cham" | "/cham-head" => dap_ung("200 OK", b"muon", "X-Test: yes"),
                        _ => dap_ung("404 Not Found", b"", "X-Test: no"),
                    };
                    tra_loi.push(std::thread::spawn(move || {
                        if cham {
                            std::thread::sleep(Duration::from_millis(300));
                        }
                        let _ = s.write_all(&phan);
                        let _ = s.shutdown(Shutdown::Both);
                    }));
                }
                // **R-Win2**: `accept` có thể trả lỗi tạm (`ConnectionAborted`/`Reset`/`Interrupted`)
                // khi client trước vừa đóng trên Windows. Trước đây `break` giết server ⇒ kết nối
                // kế tiếp bị RST (`os error 10053`). Loopback: bỏ qua lỗi tạm, chờ tới hạn 8 s.
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        for t in tra_loi {
            let _ = t.join();
        }
        thay
    });
    (addr, stop, h)
}

/// Script 4 hàm `http`; mỗi URL là một token để test gắn cổng loopback thật.
pub(super) const SRC_HTTP: &str = r#"use std.http

fn main():
    match http.get("__OK__"):
        Ok(s): print("get={s}")
        Err(e): print("get loi {e.message}")
    match http.head("__HEAD__"):
        Ok(m): print("head={m["status"]} {m["headers"]["x-test"]}")
        Err(e): print("head loi {e.message}")
    match http.get_timeout("__FAST__", 2000):
        Ok(s): print("get_timeout={s}")
        Err(e): print("get_timeout loi {e.message}")
    match http.head_timeout("__HEADFAST__", 2000):
        Ok(m): print("head_timeout={m["status"]} {m["headers"]["x-test"]}")
        Err(e): print("head_timeout loi {e.message}")
    match http.get_timeout("__SLOW__", 80):
        Ok(s): print("slow: KHONG DUOC PHEP {s}")
        Err(e): print("slow={e.message}")
    match http.head_timeout("__SLOWHEAD__", 80):
        Ok(m): print("slowhead: KHONG DUOC PHEP {m["status"]}")
        Err(e): print("slowhead={e.message}")
    match http.get("__CLOSED__"):
        Ok(_): print("closed: KHONG DUOC PHEP")
        Err(e): print("closed={e.message}")
"#;

/// **#202** — thay số mili-giây **đo thật** trong thông điệp hết giờ (`… sau 78 ms`) bằng `<N>`.
/// `get_timeout`/`head_timeout` truyền **phần deadline còn lại** xuống lớp đọc, nên interp và VM
/// chạy tuần tự lệch nhau 1 ms là bình thường; phần chữ nghĩa còn lại (tên hàm, handle, đơn vị)
/// vẫn được so **từng byte**.
fn chuan_hoa_ms(s: &str) -> String {
    let mut ra = String::with_capacity(s.len());
    let mut con = s;
    while let Some(i) = con.find(" ms") {
        let b = &con.as_bytes()[..i];
        let mut j = b.len();
        while j > 0 && b[j - 1].is_ascii_digit() {
            j -= 1;
        }
        if j == b.len() {
            // Không phải số trước ` ms`: giữ nguyên rồi đi tiếp.
            ra.push_str(&con[..i + 3]);
            con = &con[i + 3..];
        } else {
            ra.push_str(&con[..j]);
            ra.push_str("<N>");
            con = &con[i..];
        }
    }
    ra.push_str(con);
    ra
}

/// **#202** — bộ chuẩn hoá chỉ thay **số** ngay trước ` ms`; chữ/đơn vị/token khác giữ nguyên.
#[test]
fn chuan_hoa_ms_chi_thay_so_do_thuc() {
    let vao = "slow=quá thời gian chờ dữ liệu từ `C9` sau 78 ms\n\
               slowhead=quá thời gian chờ dữ liệu từ `C10` sau 79 ms\n";
    assert_eq!(
        chuan_hoa_ms(vao),
        "slow=quá thời gian chờ dữ liệu từ `C9` sau <N> ms\n\
         slowhead=quá thời gian chờ dữ liệu từ `C10` sau <N> ms\n"
    );
    // Không có số trước ` ms` ⇒ không đổi; số đứng riêng vẫn bị thay đúng một lần.
    assert_eq!(chuan_hoa_ms("không có số ms"), "không có số ms");
    assert_eq!(chuan_hoa_ms("123 ms"), "<N> ms");
}

/// **ST-315d** — 4 hàm `http` (`get`, `get_timeout`, `head`, `head_timeout`) tới server
/// HTTP/1.1 loopback thật. `head` nhận `Content-Length: 999999` **không body** mà vẫn `Ok`
/// (chứng minh không đọc thân); server trễ 300 ms > timeout 80 ms ⇒ **lỗi dữ liệu** parity;
/// cổng đóng ⇒ `Err` ("bị từ chối"/timeout tuỳ OS), vẫn cùng byte giữa hai engine.
///
/// **#202 (ST-318b)** — stdout của ca này có **số ms đo thật** trong `e.message` hết giờ, nên hai
/// engine lệch 1 ms là flake; [`chuan_hoa_ms`] thay đúng token số, giữ nguyên phần còn lại.
#[test]
fn http_bon_ham_parity() {
    let (addr, stop, h) = http_server(12);
    // Cổng **đóng** thật: bind rồi thả ⇒ `connect` bị từ chối (không dùng cổng cố định).
    let l = TcpListener::bind("127.0.0.1:0").expect("bind cổng đóng");
    let dong_addr = l.local_addr().expect("addr cổng đóng").to_string();
    drop(l);
    let d = tam("http");
    let src = SRC_HTTP
        .replace("__OK__", &format!("http://{addr}/ok"))
        .replace("__HEAD__", &format!("http://{addr}/head"))
        .replace("__FAST__", &format!("http://{addr}/nhanh"))
        .replace("__HEADFAST__", &format!("http://{addr}/head-nhanh"))
        .replace("__SLOW__", &format!("http://{addr}/cham"))
        .replace("__SLOWHEAD__", &format!("http://{addr}/cham-head"))
        .replace("__CLOSED__", &format!("http://{dong_addr}/"));
    let (out, err) = hai_duong_chuan_hoa(&d, "http", &src, || {}, chuan_hoa_ms);
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(dong.len(), 7, "http: {out}\nstderr: {err}");
    assert_eq!(
        &dong[..4],
        &[
            "get=xin chào",
            "head=200 yes",
            "get_timeout=nhanh",
            "head_timeout=200 fast",
        ],
        "http OK/HEAD: {out}"
    );
    assert!(
        dong[4].starts_with("slow=")
            && dong[4].contains("quá thời gian")
            && dong[4].ends_with(" sau <N> ms"),
        "get_timeout phải hết giờ: {}",
        dong[4]
    );
    assert!(
        dong[5].starts_with("slowhead=")
            && dong[5].contains("quá thời gian")
            && dong[5].ends_with(" sau <N> ms"),
        "head_timeout phải hết giờ: {}",
        dong[5]
    );
    assert!(
        dong[6].starts_with("closed=") && dong[6] != "closed: KHONG DUOC PHEP",
        "cổng đóng phải là Err: {}",
        dong[6]
    );
    stop.store(true, Ordering::Relaxed);
    let thay = h.join().expect("join server HTTP");
    assert_eq!(
        thay,
        [
            "GET /ok HTTP/1.1",
            "HEAD /head HTTP/1.1",
            "GET /nhanh HTTP/1.1",
            "HEAD /head-nhanh HTTP/1.1",
            "GET /cham HTTP/1.1",
            "HEAD /cham-head HTTP/1.1",
            "GET /ok HTTP/1.1",
            "HEAD /head HTTP/1.1",
            "GET /nhanh HTTP/1.1",
            "HEAD /head-nhanh HTTP/1.1",
            "GET /cham HTTP/1.1",
            "HEAD /cham-head HTTP/1.1",
        ],
        "server HTTP phải thấy đúng method/đường dẫn của cả hai engine"
    );
    let _ = std::fs::remove_dir_all(&d);
}
