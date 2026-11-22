// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-277 / VD-256**: `connect`/`write` nonblocking trên **đường frame test-only**.
//!
//! Lát 3 đã có readiness thật cho `read`/`accept`. Ở đây dùng **nguyên** các helper đó
//! (`frame_reactor_poll_io`, `frame_reactor_cancel_handle`, `frame_reactor_shutdown`,
//! `diag_snapshot`) — KHÔNG sửa `rt.rs`, không helper mới. Adapter Linux (mio, safe API)
//! nằm hoàn toàn trong test này: nó nhận handle mã hoá `N:<addr>` / `W:<tay>` do
//! `stdlib` phát ra, làm nonblocking connect (kiểm tra SO_ERROR + peer_addr khi readiness)
//! và partial write (lưu offset, resume không gửi lại phần cũ).
//!
//! Mạng chỉ **loopback**, mọi hạn đều rộng, snapshot về baseline sau mỗi ca.

#![cfg(target_os = "linux")]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use mio::net::{TcpListener as MioListener, TcpStream as MioStream};
use mio::{Events, Interest, Poll, Token};

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{
    FrameReactorBackend, ReactorIoOutcome, ReactorOp, ReactorRawEvent, Scheduler, Wait, Waiter,
};
use vis_interp::stdlib::{FRAME_NET_CONNECT, FRAME_NET_WRITE};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

// ---------------------------------------------------------------------------
// Backend thật (test-only). Mọi API gọi đều safe; unsafe FFI epoll nằm trong mio.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op2 {
    Read,
    Connect,
    Write,
}

/// Adapter giới hạn mỗi lần `write` ở `WRITE_CHUNK` byte. Đây là một giới hạn **cố ý**
/// của adapter test: ép nhiều lần `WouldBlock`/resume để kiểm chứng offset tất định,
/// không phụ thuộc vào kích thước send-buffer của kernel (thay đổi theo máy). Ngay cả
/// khi kernel vẫn còn chỗ, adapter trả `WouldBlock` để scheduler chạy frame khác —
/// chính là hành vi cần kiểm chứng (VD-256 §5).
const WRITE_CHUNK: usize = 64 * 1024;

struct WriteState {
    tay: String,
    data: Vec<u8>,
    off: usize,
}

struct Net2 {
    streams: HashMap<String, MioStream>,
    listeners: HashMap<String, MioListener>,
    poll: Poll,
    events: Events,
    /// token → khoá stream/listener ĐÃ đăng ký (dùng để deregister đúng một lần).
    tokens: HashMap<u64, String>,
    /// các khoá `N:<addr>` đã khởi tạo connect (chờ WRITABLE + kiểm tra kết quả).
    connects: HashSet<String>,
    /// `W:<tay>` → (bytes, offset đã gửi) — offset sống ở adapter, KHÔNG ở core.
    writes: HashMap<String, WriteState>,
    next_conn: u32,
    fail_next_wait: Option<String>,
}

impl Net2 {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
            poll: Poll::new().expect("mio::Poll::new"),
            events: Events::with_capacity(128),
            tokens: HashMap::new(),
            connects: HashSet::new(),
            writes: HashMap::new(),
            next_conn: 0,
            fail_next_wait: None,
        }
    }

    fn add_stream(&mut self, h: &str, s: std::net::TcpStream) {
        let _ = s.set_nonblocking(true);
        self.streams.insert(h.to_string(), MioStream::from_std(s));
    }

    #[allow(dead_code)]
    fn add_listener(&mut self, h: &str, l: TcpListener) {
        let _ = l.set_nonblocking(true);
        self.listeners
            .insert(h.to_string(), MioListener::from_std(l));
    }

    fn rc(self) -> Rc<RefCell<dyn FrameReactorBackend>> {
        Rc::new(RefCell::new(self))
    }

    fn classify(h: &str) -> Op2 {
        if h.starts_with(FRAME_NET_CONNECT) {
            Op2::Connect
        } else if h.starts_with(FRAME_NET_WRITE) {
            Op2::Write
        } else {
            Op2::Read
        }
    }

    /// Khoá đối tượng kernel cần register: `W:<tay>` → `tay`, còn lại giữ nguyên.
    fn target_of(h: &str) -> &str {
        match Self::classify(h) {
            Op2::Write => &h[FRAME_NET_WRITE.len()..],
            _ => h,
        }
    }

    fn cleanup_connect(&mut self, key: &str) {
        self.connects.remove(key);
        self.streams.remove(key);
    }

    /// connect thật: khởi tạo nonblocking, rồi **CHỈ** tin readiness sau khi đã kiểm
    /// `take_error()` (SO_ERROR) VÀ `peer_addr()` — readiness chỉ là hint (VD-256 §5).
    fn try_connect(&mut self, key: &str) -> ReactorIoOutcome {
        let addr_text = &key[FRAME_NET_CONNECT.len()..];
        if !self.connects.contains(key) {
            let addr: SocketAddr = match addr_text.parse() {
                Ok(a) => a,
                Err(e) => {
                    return ReactorIoOutcome::Err(format!(
                        "địa chỉ `{addr_text}` không hợp lệ: {e}"
                    ))
                }
            };
            return match MioStream::connect(addr) {
                Ok(s) => {
                    self.streams.insert(key.to_string(), s);
                    self.connects.insert(key.to_string());
                    // Chờ WRITABLE; khi đó mới kiểm tra kết quả nối thực tế.
                    ReactorIoOutcome::WouldBlock
                }
                Err(e) => ReactorIoOutcome::Err(format!("nối tới `{addr_text}` thất bại: {e}")),
            };
        }

        // Đã có connect đang treo: ở đây luôn là sau một event WRITABLE (hoặc deadline).
        let Some(s) = self.streams.get(key) else {
            self.connects.remove(key);
            return ReactorIoOutcome::Err(format!("tay nối `{addr_text}` đã đóng"));
        };
        match s.take_error() {
            Ok(Some(e)) => {
                self.cleanup_connect(key);
                return ReactorIoOutcome::Err(format!(
                    "nối tới `{addr_text}` thất bại (SO_ERROR): {e}"
                ));
            }
            Err(e) => {
                self.cleanup_connect(key);
                return ReactorIoOutcome::Err(format!("lỗi kiểm tra nối `{addr_text}`: {e}"));
            }
            Ok(None) => {}
        }
        // take_error() sạch chưa chắc đã nối xong — peer_addr là kiểm chứng cuối.
        match s.peer_addr() {
            Ok(_) => {
                let s = self.streams.remove(key).expect("stream đang nối");
                self.connects.remove(key);
                let tay = format!("C{}", self.next_conn + 1);
                self.next_conn += 1;
                self.streams.insert(tay.clone(), s);
                ReactorIoOutcome::Accepted(tay)
            }
            Err(e)
                if e.kind() == ErrorKind::WouldBlock
                    || e.kind() == ErrorKind::NotConnected
                    || e.raw_os_error() == Some(115) /* EINPROGRESS trên Linux */
            =>
            {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => {
                self.cleanup_connect(key);
                ReactorIoOutcome::Err(format!("nối tới `{addr_text}` thất bại (peer): {e}"))
            }
        }
    }

    /// partial write có offset: bytes lấy từ registry lần đầu, sau đó giữ trong `writes`;
    /// mỗi lần chỉ ghi phần `[off..]`, `off += n`, không bao giờ gửi lại phần đã gửi.
    fn try_write(&mut self, key: &str) -> ReactorIoOutcome {
        let tay = &key[FRAME_NET_WRITE.len()..];
        if !self.writes.contains_key(key) {
            let Some(data) = vis_interp::stdlib::frame_net_take_write(key) else {
                // Tay bị đóng khi đang ghi: registry cũ đã bị stdlib dọn ⇒ Err rõ.
                return ReactorIoOutcome::Err(format!("tay `{tay}` đã đóng khi đang ghi"));
            };
            self.writes.insert(
                key.to_string(),
                WriteState {
                    tay: tay.to_string(),
                    data,
                    off: 0,
                },
            );
        }
        let tay_owned = self
            .writes
            .get(key)
            .expect("write state vừa tạo")
            .tay
            .clone();
        if !self.streams.contains_key(&tay_owned) {
            self.writes.remove(key);
            return ReactorIoOutcome::Err(format!("tay `{tay}` không tồn tại (đã đóng) khi ghi"));
        }
        let (data_len, off) = {
            let ws = self.writes.get(key).unwrap();
            (ws.data.len(), ws.off)
        };
        if off >= data_len {
            self.writes.remove(key);
            return ReactorIoOutcome::Data(Vec::new());
        }
        let res = {
            let writes = &mut self.writes;
            let streams = &mut self.streams;
            let ws = writes.get_mut(key).unwrap();
            let s = streams.get_mut(&tay_owned).unwrap();
            let end = (ws.off + WRITE_CHUNK).min(ws.data.len());
            match s.write(&ws.data[ws.off..end]) {
                Ok(0) => ReactorIoOutcome::WouldBlock,
                Ok(n) => {
                    ws.off += n;
                    if ws.off >= ws.data.len() {
                        ReactorIoOutcome::Data(Vec::new())
                    } else {
                        // Còn bytes ⇒ hỏi WRITABLE tiếp; offset đã lưu lại.
                        ReactorIoOutcome::WouldBlock
                    }
                }
                Err(e)
                    if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted =>
                {
                    ReactorIoOutcome::WouldBlock
                }
                Err(e) => ReactorIoOutcome::Err(format!("lỗi ghi tới `{tay}`: {e}")),
            }
        };
        if !matches!(res, ReactorIoOutcome::WouldBlock) {
            self.writes.remove(key);
        }
        res
    }
}

impl FrameReactorBackend for Net2 {
    fn try_read(&mut self, handle: &str, max: usize) -> ReactorIoOutcome {
        match Self::classify(handle) {
            Op2::Connect => self.try_connect(handle),
            Op2::Write => self.try_write(handle),
            Op2::Read => {
                if max == 0 {
                    return ReactorIoOutcome::Data(Vec::new());
                }
                let Some(s) = self.streams.get_mut(handle) else {
                    return ReactorIoOutcome::Err(format!(
                        "tay mạng `{handle}` không tồn tại (đã đóng)"
                    ));
                };
                let mut buf = vec![0u8; max.min(vis_interp::stdlib::NET_READ_CAP as usize)];
                match s.read(&mut buf) {
                    Ok(0) => ReactorIoOutcome::Data(Vec::new()),
                    Ok(n) => {
                        buf.truncate(n);
                        ReactorIoOutcome::Data(buf)
                    }
                    Err(e)
                        if e.kind() == ErrorKind::WouldBlock
                            || e.kind() == ErrorKind::Interrupted =>
                    {
                        ReactorIoOutcome::WouldBlock
                    }
                    Err(e) => ReactorIoOutcome::Err(format!("lỗi đọc từ `{handle}`: {e}")),
                }
            }
        }
    }

    fn try_accept(&mut self, handle: &str) -> ReactorIoOutcome {
        let Some(l) = self.listeners.get_mut(handle) else {
            return ReactorIoOutcome::Err(format!("tay listener `{handle}` không tồn tại"));
        };
        match l.accept() {
            Ok((s, _)) => {
                let h = format!("C{}", self.next_conn + 1);
                self.next_conn += 1;
                self.streams.insert(h.clone(), s);
                ReactorIoOutcome::Accepted(h)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => ReactorIoOutcome::Err(format!("lỗi `accept` trên `{handle}`: {e}")),
        }
    }

    fn register(&mut self, token: u64, handle: &str, _op: ReactorOp) -> Result<(), String> {
        let interest = match Self::classify(handle) {
            Op2::Read => Interest::READABLE,
            Op2::Connect | Op2::Write => Interest::WRITABLE,
        };
        let key = Self::target_of(handle).to_string();
        let r = if let Some(s) = self.streams.get_mut(&key) {
            self.poll
                .registry()
                .register(s, Token(token as usize), interest)
        } else if let Some(l) = self.listeners.get_mut(&key) {
            self.poll
                .registry()
                .register(l, Token(token as usize), interest)
        } else {
            return Err(format!("không tìm thấy `{key}` để đăng ký readiness"));
        };
        match r {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {} // re-register cùng fd
            Err(e) => return Err(format!("epoll register `{key}` thất bại: {e}")),
        }
        self.tokens.insert(token, key);
        Ok(())
    }

    fn deregister(&mut self, token: u64, _handle: &str, _op: ReactorOp) -> Result<(), String> {
        // Deregister theo token — idempotent (token bị xoá ⇒ lần sau trả Ok ngay).
        let Some(key) = self.tokens.remove(&token) else {
            return Ok(());
        };
        let r = if let Some(s) = self.streams.get_mut(&key) {
            self.poll.registry().deregister(s)
        } else if let Some(l) = self.listeners.get_mut(&key) {
            self.poll.registry().deregister(l)
        } else {
            return Ok(()); // tay đã đóng: kernel tự gỡ
        };
        match r {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("epoll deregister `{key}` thất bại: {e}")),
        }
    }

    fn wait(&mut self, timeout: Duration) -> Result<Vec<ReactorRawEvent>, String> {
        if let Some(e) = self.fail_next_wait.take() {
            return Err(e);
        }
        self.poll
            .poll(&mut self.events, Some(timeout))
            .map_err(|e| format!("epoll_wait thất bại: {e}"))?;
        Ok(self
            .events
            .iter()
            .map(|ev| ReactorRawEvent {
                token: ev.token().0 as u64,
                readable: ev.is_readable(),
                hup: ev.is_read_closed() || ev.is_write_closed(),
                error: ev.is_error(),
            })
            .collect())
    }

    fn close(&mut self, handle: &str) -> Result<(), String> {
        self.streams.remove(handle);
        self.listeners.remove(handle);
        if handle.starts_with(FRAME_NET_CONNECT) {
            self.connects.remove(handle);
        }
        let ws: Vec<String> = self
            .writes
            .iter()
            .filter(|(_, w)| w.tay == handle)
            .map(|(k, _)| k.clone())
            .collect();
        for k in ws {
            self.writes.remove(&k);
        }
        let toks: Vec<u64> = self
            .tokens
            .iter()
            .filter(|(_, k)| *k == handle)
            .map(|(t, _)| *t)
            .collect();
        for t in toks {
            let _ = self.deregister(t, handle, ReactorOp::Read);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backend scripted — kiểm chứng bất biến lõi TẤT ĐỊNH (không kernel, không timing).
// ---------------------------------------------------------------------------
struct Scripted {
    events: VecDeque<ReactorRawEvent>,
    registered: Vec<(u64, String, ReactorOp)>,
    deregistered: Vec<u64>,
}
impl Scripted {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            registered: Vec::new(),
            deregistered: Vec::new(),
        }
    }
    #[allow(dead_code)]
    fn rc(self) -> Rc<RefCell<dyn FrameReactorBackend>> {
        Rc::new(RefCell::new(self))
    }
}
impl FrameReactorBackend for Scripted {
    fn try_read(&mut self, _h: &str, _max: usize) -> ReactorIoOutcome {
        ReactorIoOutcome::WouldBlock
    }
    fn try_accept(&mut self, _h: &str) -> ReactorIoOutcome {
        ReactorIoOutcome::WouldBlock
    }
    fn register(&mut self, token: u64, h: &str, op: ReactorOp) -> Result<(), String> {
        self.registered.push((token, h.to_string(), op));
        Ok(())
    }
    fn deregister(&mut self, token: u64, _h: &str, _op: ReactorOp) -> Result<(), String> {
        self.deregistered.push(token);
        Ok(())
    }
    fn wait(&mut self, _timeout: Duration) -> Result<Vec<ReactorRawEvent>, String> {
        Ok(self.events.drain(..).collect())
    }
    fn close(&mut self, _h: &str) -> Result<(), String> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------
fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

fn cfg(b: Rc<RefCell<dyn FrameReactorBackend>>) -> FrameConfig {
    FrameConfig {
        backend: Some(b),
        ..Default::default()
    }
}

fn chay_i(src: &str, b: Rc<RefCell<dyn FrameReactorBackend>>) -> FrameOutput {
    let (hir, syms, tys) = tien_trinh(src);
    run_i(src, &hir, &syms, &tys, &[], cfg(b))
}
fn chay_v(src: &str, b: Rc<RefCell<dyn FrameReactorBackend>>) -> FrameOutput {
    let (hir, syms, _tys) = tien_trinh(src);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    run_v(&m, src, &[], cfg(b))
}
fn chay_duoi_engine(
    src: &str,
    mk: impl Fn() -> Rc<RefCell<dyn FrameReactorBackend>>,
) -> (FrameOutput, FrameOutput) {
    (chay_i(src, mk()), chay_v(src, mk()))
}
fn assert_ok(out: &FrameOutput) {
    assert_eq!(
        out.exit, 0,
        "stdout={:?} diag={:?}",
        out.stdout, out.diagnostics
    );
}
fn baseline() -> vis_interp::rt::DiagSnapshot {
    vis_interp::rt::DiagSnapshot {
        live_tasks: 0,
        frames: 0,
        waiters: 0,
        ready: 0,
        regs: 0,
        timers: 0,
    }
}

// ---------------------------------------------------------------------------
// Server loopback helper — đọc theo chunk nhỏ (có delay) để ép backpressure thật.
// ---------------------------------------------------------------------------
fn reader_server(
    l: TcpListener,
    first_delay_ms: u64,
    chunk: usize,
    gap_ms: u64,
) -> (thread::JoinHandle<()>, mpsc::Receiver<Vec<u8>>) {
    let (tx, rx) = mpsc::channel();
    let h = thread::spawn(move || {
        let (mut s, _) = l.accept().expect("server accept");
        if first_delay_ms > 0 {
            thread::sleep(Duration::from_millis(first_delay_ms));
        }
        let mut buf = vec![0u8; chunk.max(1)];
        let mut all = Vec::new();
        loop {
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    all.extend_from_slice(&buf[..n]);
                    if gap_ms > 0 {
                        thread::sleep(Duration::from_millis(gap_ms));
                    }
                }
                Err(e) => panic!("server read: {e}"),
            }
        }
        let _ = tx.send(all); // receiver có thể đã bị bỏ trong ca không cần bytes
    });
    (h, rx)
}

/// Server nhận một kết nối rồi giữ nó không đọc trong `ms` ms ⇒ ép backpressure.
fn hold_server(l: TcpListener, ms: u64) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        if let Ok((s, _)) = l.accept() {
            thread::sleep(Duration::from_millis(ms));
            drop(s);
        }
    })
}

/// Chạy cả hai engine, **mỗi engine một listener/server riêng** do `mk` tạo, rồi thu
/// bytes server nhận được — tránh ca sau nối nhầm vào server đã kết thúc của ca trước.
fn run_both_with_server(
    src: &str,
    mut mk: impl FnMut() -> (
        Rc<RefCell<dyn FrameReactorBackend>>,
        mpsc::Receiver<Vec<u8>>,
        thread::JoinHandle<()>,
    ),
) -> (FrameOutput, FrameOutput, Vec<Vec<u8>>) {
    let (b1, rx1, s1) = mk();
    let o1 = chay_i(src, b1);
    let (b2, rx2, s2) = mk();
    let o2 = chay_v(src, b2);
    let g1 = rx1
        .recv_timeout(Duration::from_secs(8))
        .expect("server bytes i");
    let g2 = rx2
        .recv_timeout(Duration::from_secs(8))
        .expect("server bytes v");
    s1.join().unwrap();
    s2.join().unwrap();
    (o1, o2, vec![g1, g2])
}

fn payload(n: usize) -> String {
    // Dữ liệu xác định, chỉ gồm ký tự an toàn trong literal ViScript
    // (không có `"`, `\`, newline) nhưng vẫn đủ độ dài để ép partial write.
    let unit = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-";
    let mut v = Vec::with_capacity(n);
    while v.len() < n {
        v.extend_from_slice(unit);
    }
    v.truncate(n);
    String::from_utf8(v).expect("utf8")
}

// ===========================================================================
// (1) Connection refused / failed connect — readiness KHÔNG được tin khi connect lỗi.
// ===========================================================================
#[test]
fn t1_connect_refused_both_engines() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    drop(l); // không còn ai listen ⇒ connect bị từ chối

    let src = format!(
        r#"use std.net

async fn c() -> int:
    let r = net.connect_timeout("127.0.0.1:{port}", 1500)
    print(r)
    1

fn main():
    print(await spawn c())
"#
    );
    let (i, v) = chay_duoi_engine(&src, || Net2::new().rc());
    for out in [&i, &v] {
        assert_ok(out);
        assert!(out.stdout.contains("Err("), "{:?}", out.stdout);
        assert!(
            out.stdout.contains("thất bại") || out.stdout.contains("từ chối"),
            "{:?}",
            out.stdout
        );
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
        // Không được trả tay C1 giả khi connect lỗi.
        assert!(!out.stdout.contains("Ok(C"), "{:?}", out.stdout);
    }
}

// ===========================================================================
// (2) Connect THÀNH CÔNG rồi write — kiểm chứng SO_ERROR/peer_addr trước khi trả tay.
// ===========================================================================
fn run_connect_write(is_i: bool) -> (FrameOutput, Vec<u8>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let (srv, rx) = reader_server(l, 0, 1024, 0);
    let src = format!(
        r#"use std.net

async fn c() -> int:
    print("C:pre")
    let h = net.connect_timeout("{addr}", 2000)
    print(h)
    let w = net.write("C1", "PONG")
    print(w)
    1

fn main():
    print(await spawn c())
"#
    );
    let out = if is_i {
        chay_i(&src, Net2::new().rc())
    } else {
        chay_v(&src, Net2::new().rc())
    };
    let got = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("bytes từ server");
    srv.join().unwrap();
    (out, got)
}

#[test]
fn t2_connect_then_write_real_both_engines() {
    for (out, got) in [run_connect_write(true), run_connect_write(false)] {
        assert_eq!(got, b"PONG", "server nhận đúng bytes");
        assert_ok(&out);
        assert_eq!(out.stdout.matches("C:pre").count(), 1, "không re-run");
        assert!(out.stdout.contains("Ok(C"), "{:?}", out.stdout);
        assert!(
            out.stdout.contains("Ok(())"),
            "net.write trả Ok(()) — {:?}",
            out.stdout
        );
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ===========================================================================
// (3) Peer đọc chậm (small chunks) — write nhỏ vẫn tới đủ, không duplicate.
// ===========================================================================
#[test]
fn t3_peer_reads_slowly_both_engines() {
    let expected = payload(4096);
    let src = format!(
        r#"use std.net

async fn w() -> int:
    let r = net.write("C1", "{data}")
    print(r)
    1

fn main():
    print(await spawn w())
"#,
        data = expected
    );
    for rep in 1..=2 {
        let (i, v, gots) = run_both_with_server(&src, || {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = l.local_addr().unwrap();
            let (srv, rx) = reader_server(l, 40, 7, 2);
            let mut b = Net2::new();
            b.add_stream(
                "C1",
                std::net::TcpStream::connect(addr).expect("client connect"),
            );
            (b.rc(), rx, srv)
        });
        for got in gots {
            assert_eq!(got.len(), expected.len(), "rep={rep} tổng số byte");
            assert_eq!(
                got,
                expected.as_bytes(),
                "rep={rep} không duplicate/mất byte"
            );
        }
        for out in [&i, &v] {
            assert_ok(out);
            assert!(out.stdout.contains("Ok(())"), "{:?}", out.stdout);
            assert_eq!(out.snapshot, baseline(), "rep={rep} {:?}", out.log);
        }
    }
}

// ===========================================================================
// (4) LARGE write + backpressure — offset được lưu, resume KHÔNG gửi lại phần cũ.
// ===========================================================================
#[test]
fn t4_large_write_backpressure_no_duplicate_both_engines() {
    const N: usize = 1_048_576; // 1 MiB — vượt xa send buffer ⇒ bắt buộc partial write
    let expected = payload(N);
    let src = format!(
        r#"use std.net

async fn w() -> int:
    print("W:pre")
    let r = net.write("C1", "{data}")
    print("W:post")
    print(r)
    1

fn main():
    print(await spawn w())
"#,
        data = expected
    );
    for rep in 1..=3 {
        let (i, v, gots) = run_both_with_server(&src, || {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = l.local_addr().unwrap();
            // Peer KHÔNG đọc trong 90 ms đầu ⇒ buffer đầy, write trả WouldBlock ⇒ re-arm.
            let (srv, rx) = reader_server(l, 90, 8192, 1);
            let mut b = Net2::new();
            b.add_stream(
                "C1",
                std::net::TcpStream::connect(addr).expect("client connect"),
            );
            (b.rc(), rx, srv)
        });
        for got in gots {
            assert_eq!(got.len(), N, "rep={rep} tổng byte nhận được");
            assert_eq!(got, expected.as_bytes(), "rep={rep} byte bị duplicate/mất");
        }
        for out in [&i, &v] {
            assert_ok(out);
            assert_eq!(out.stdout.matches("W:pre").count(), 1, "không re-run");
            assert_eq!(out.stdout.matches("W:post").count(), 1);
            assert!(out.stdout.contains("Ok(())"), "{:?}", out.stdout);
            // Bằng chứng partial-write thật: có ít nhất một lần re-arm sau WouldBlock.
            assert!(
                out.log.iter().any(|l| l.starts_with("rrearm:")),
                "không có re-arm (không chứng minh được partial write): {:?}",
                &out.log[..out.log.len().min(20)]
            );
            assert_eq!(out.snapshot, baseline(), "rep={rep} {:?}", out.log);
        }
    }
}

// ===========================================================================
// (5) Close giữa chừng — Err đúng một lần, cleanup về baseline, không late wake.
// ===========================================================================
#[test]
fn t5_close_while_write_in_flight_both_engines() {
    let expected = payload(1_048_576);
    let src = format!(
        r#"use std.net

async fn w() -> int:
    print("W:pre")
    let r = net.write("C1", "{data}")
    print(r)
    1

async fn killer() -> int:
    net.close("C1")
    print("CLOSED")
    2

fn main():
    let a = spawn w()
    let b = spawn killer()
    print(await a)
    print(await b)
"#,
        data = expected
    );
    let mk = || {
        let mut b = Net2::new();
        // Peer nhận rồi KHÔNG đọc ⇒ write chắc chắn bị treo trước khi killer chạy.
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let srv = hold_server(l, 2000);
        std::mem::forget(srv);
        b.add_stream(
            "C1",
            std::net::TcpStream::connect(addr).expect("client connect"),
        );
        b.rc()
    };
    let (i, v) = chay_duoi_engine(&src, mk);
    for out in [&i, &v] {
        assert_ok(out);
        assert_eq!(out.stdout.matches("W:pre").count(), 1, "không re-run");
        assert_eq!(out.stdout.matches("CLOSED").count(), 1);
        assert!(out.stdout.contains("Err("), "{:?}", out.stdout);
        assert_eq!(
            out.stdout.matches("Err(").count(),
            1,
            "Err đúng một lần: {:?}",
            out.stdout
        );
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ===========================================================================
// (6) Task KHÁC tiến triển khi một task đang chờ connect/write (ý chính của slice).
// ===========================================================================
#[test]
fn t6_other_task_progresses_during_write_both_engines() {
    let expected = payload(1_048_576);
    let src = format!(
        r#"use std.net

async fn a() -> int:
    print("A:pre")
    let r = net.write("C1", "{data}")
    print("A:post")
    print(r)
    1

async fn b() -> int:
    print("BG")
    2

fn main():
    let x = spawn a()
    let y = spawn b()
    print(await x)
    print(await y)
"#,
        data = expected
    );
    let mk = || {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let (srv, _rx) = reader_server(l, 80, 8192, 1);
        // Giữ server sống tới khi chạy xong (thread tự kết thúc khi client đóng).
        std::mem::forget(srv);
        let mut b = Net2::new();
        b.add_stream(
            "C1",
            std::net::TcpStream::connect(addr).expect("client connect"),
        );
        b.rc()
    };
    let (i, v) = chay_duoi_engine(&src, mk);
    for out in [&i, &v] {
        assert_ok(out);
        let s = &out.stdout;
        let pa = s.find("A:pre").expect("A:pre");
        let pb = s.find("BG").expect("BG");
        let pp = s.find("A:post").expect("A:post");
        assert!(
            pa < pb && pb < pp,
            "task B phải chạy khi A đang treo: {s:?}"
        );
        assert!(s.contains("Ok(())"), "{s:?}");
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

#[test]
fn t7_other_task_progresses_during_connect_both_engines() {
    fn src_for(addr: SocketAddr) -> String {
        format!(
            r#"use std.net

async fn a() -> int:
    print("A:pre")
    let h = net.connect_timeout("{addr}", 2000)
    print("A:post")
    print(h)
    1

async fn b() -> int:
    print("BG")
    2

fn main():
    let x = spawn a()
    let y = spawn b()
    print(await x)
    print(await y)
"#
        )
    }

    // Mỗi engine một listener/server riêng: connect thật, server chấp nhận sau 70 ms.
    fn one(is_i: bool) -> FrameOutput {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let srv = hold_server(l, 400);
        let src = src_for(addr);
        let out = if is_i {
            chay_i(&src, Net2::new().rc())
        } else {
            chay_v(&src, Net2::new().rc())
        };
        let _ = srv.join();
        out
    }

    for out in [one(true), one(false)] {
        assert_ok(&out);
        let s = &out.stdout;
        let pa = s.find("A:pre").expect("A:pre");
        let pb = s.find("BG").expect("BG");
        let pp = s.find("A:post").expect("A:post");
        assert!(
            pa < pb && pb < pp,
            "B phải chạy khi connect đang treo: {s:?}"
        );
        assert!(s.contains("Ok(C"), "{s:?}");
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ===========================================================================
// (7) Deadline KHÔNG reset theo readiness + peer không đọc ⇒ timeout có hạn, cleanup.
// ===========================================================================
#[test]
fn t8_write_timeout_is_bounded_and_cleans_up() {
    vis_interp::stdlib::frame_net_set_write_timeout(Some(300));
    // 4 MiB sinh bởi `str.repeat` ⇒ chắc chắn vượt send buffer dù peer không đọc.
    let src = r#"use std.net
use std.str

async fn w() -> int:
    let data = str.repeat("0123456789", 400000)
    let r = net.write("C1", data)
    print(r)
    1

fn main():
    print(await spawn w())
"#;
    let mk = || {
        let mut b = Net2::new();
        // Peer nhận rồi KHÔNG BAO GIỜ đọc ⇒ write dừng ở partial + chờ WRITABLE.
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let srv = hold_server(l, 2000);
        std::mem::forget(srv);
        b.add_stream("C1", std::net::TcpStream::connect(addr).unwrap());
        b.rc()
    };
    let t0 = std::time::Instant::now();
    let (i, v) = chay_duoi_engine(src, mk);
    let elapsed = t0.elapsed();
    vis_interp::stdlib::frame_net_set_write_timeout(None);
    assert!(
        elapsed < Duration::from_secs(3),
        "phải thoát theo deadline, không treo vô hạn: {elapsed:?}"
    );
    for out in [&i, &v] {
        assert_ok(out);
        assert!(out.stdout.contains("quá thời gian"), "{:?}", out.stdout);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ===========================================================================
// (8) Bất biến tất định: token generation / deregister đúng một lần / stale wake.
//     Dùng `N:`/`W:` handle đi thẳng qua helper core — KHÔNG cần kernel.
// ===========================================================================
#[test]
fn t9_token_generation_connect_write_deterministic() {
    let b = Rc::new(RefCell::new(Scripted::new()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    s.frame.current = 1;
    let span = vis_syntax::Span::new(0, 0);

    // connect: lần đầu WouldBlock ⇒ register token 0.
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "N:127.0.0.1:9", 0, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    assert_eq!(
        b.borrow().registered,
        vec![(0, "N:127.0.0.1:9".into(), ReactorOp::Read)]
    );
    let deadline0 = s
        .frame
        .reactor
        .regs
        .iter()
        .find(|r| r.key == 0)
        .unwrap()
        .deadline;
    assert!(deadline0.is_some(), "connect có deadline riêng");

    // write: handle KHÁC ⇒ key 1, token 1 — không dùng lại token của connect.
    vis_interp::stdlib::frame_net_set_write("W:C1", b"X");
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "W:C1", 1, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 2,
        wait: Wait::Reactor { key: 1, gen: 0 },
        gen: 0,
    });
    assert_eq!(b.borrow().registered.last().unwrap().0, 1);

    // stale token (fd/tay khác) ⇒ không wake ai.
    b.borrow_mut().events.push_back(ReactorRawEvent {
        token: 999,
        readable: true,
        hup: false,
        error: false,
    });
    assert_eq!(s.frame_next(), None);
    assert!(s.frame.reactor.log.iter().any(|l| l == "rstale:token:999"));

    // event đúng token 0 ⇒ wake frame 1 (connect).
    b.borrow_mut().events.push_back(ReactorRawEvent {
        token: 0,
        readable: true,
        hup: false,
        error: false,
    });
    assert_eq!(s.frame_next(), Some(1));

    // retry vẫn WouldBlock ⇒ re-arm token MỚI (2), gen MỚI; token 0 thành stale.
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "N:127.0.0.1:9", 0, 1000, span)
        .unwrap_err();
    assert_eq!(b.borrow().registered.last().unwrap().0, 2);
    assert_eq!(b.borrow().deregistered, vec![0], "gỡ đúng token cũ 1 lần");
    assert_eq!(
        s.frame
            .reactor
            .regs
            .iter()
            .find(|r| r.key == 0)
            .unwrap()
            .deadline,
        deadline0,
        "deadline KHÔNG được reset sau mỗi readiness/re-arm (VD-256 §5)"
    );
    b.borrow_mut().events.push_back(ReactorRawEvent {
        token: 0,
        readable: true,
        hup: false,
        error: false,
    });
    assert_eq!(s.frame_next(), None, "token cũ sau re-arm phải stale");

    // huỷ tay đang chờ: wake + deregister token hiện hành đúng một lần.
    s.frame_reactor_cancel_handle("W:C1");
    assert_eq!(b.borrow().deregistered, vec![0, 1]);

    s.frame_reactor_shutdown();
    assert!(s.frame.token_map.is_empty());
    assert!(s.frame.reactor.regs.is_empty());
    let _ = vis_interp::stdlib::frame_net_take_write("W:C1");
}
