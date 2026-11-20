// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-271 / VD-254 — LÁT 3**: readiness THẬT (epoll qua `mio`) cắm vào đường frame
//! test của cả interp lẫn VM. Chỉ `net.read`/`net.accept` là nonblocking trên đường
//! test; `connect`/`write` vẫn đồng bộ ở runtime mặc định (xem report). Không CLI,
//! không API ngôn ngữ, không `unsafe` trong mã dự án.

#![cfg(target_os = "linux")]

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use mio::net::{TcpListener as MioListener, TcpStream as MioStream};
use mio::{Events, Interest, Poll, Token};

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{
    FrameReactorBackend, ReactorIoOutcome, ReactorOp, ReactorRawEvent, Scheduler, Wait, Waiter,
};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

// ---------------------------------------------------------------------------
// Adapter selector thật — nằm hoàn toàn trong test (đường frame test-only).
// Mọi API gọi đều safe; `unsafe` FFI epoll nằm trong `mio`, không nằm ở đây.
// ---------------------------------------------------------------------------
struct Reactor {
    streams: HashMap<String, MioStream>,
    listeners: HashMap<String, MioListener>,
    poll: Poll,
    events: Events,
    /// token → handle. Object lấy lại từ handle hiện hành lúc deregister, nên tay
    /// đóng + fd dùng lại KHÔNG BAO GIỜ deregister nhầm socket mới.
    tokens: HashMap<u64, String>,
    /// Tiêm lỗi selector để chứng minh lỗi lan rõ (không nuốt thành "hết event").
    fail_next_wait: Option<String>,
}

impl Reactor {
    fn new() -> Self {
        Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
            poll: Poll::new().expect("mio::Poll::new"),
            events: Events::with_capacity(64),
            tokens: HashMap::new(),
            fail_next_wait: None,
        }
    }
    fn add_stream(&mut self, h: &str, s: std::net::TcpStream) {
        let _ = s.set_nonblocking(true);
        self.streams.insert(h.to_string(), MioStream::from_std(s));
    }
    fn add_listener(&mut self, h: &str, l: TcpListener) {
        let _ = l.set_nonblocking(true);
        self.listeners
            .insert(h.to_string(), MioListener::from_std(l));
    }
    fn rc(self) -> Rc<RefCell<dyn FrameReactorBackend>> {
        Rc::new(RefCell::new(self))
    }
}

impl FrameReactorBackend for Reactor {
    fn try_read(&mut self, handle: &str, max: usize) -> ReactorIoOutcome {
        if max == 0 {
            return ReactorIoOutcome::Data(Vec::new());
        }
        let Some(s) = self.streams.get_mut(handle) else {
            return ReactorIoOutcome::Err(format!("tay mạng `{handle}` không tồn tại (đã đóng)"));
        };
        let mut buf = vec![0u8; max.min(vis_interp::stdlib::NET_READ_CAP as usize)];
        match s.read(&mut buf) {
            Ok(0) => ReactorIoOutcome::Data(Vec::new()), // EOF
            Ok(n) => {
                buf.truncate(n);
                ReactorIoOutcome::Data(buf)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => ReactorIoOutcome::Err(format!("lỗi đọc từ `{handle}`: {e}")),
        }
    }

    fn try_accept(&mut self, handle: &str) -> ReactorIoOutcome {
        let Some(l) = self.listeners.get_mut(handle) else {
            return ReactorIoOutcome::Err(format!("tay listener `{handle}` không tồn tại"));
        };
        match l.accept() {
            Ok((s, _)) => {
                let h = format!("C{}", self.streams.len() + 1);
                self.streams.insert(h.clone(), s);
                ReactorIoOutcome::Accepted(h)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                ReactorIoOutcome::WouldBlock
            }
            Err(e) => ReactorIoOutcome::Err(format!("lỗi `accept` trên `{handle}`: {e}")),
        }
    }

    fn register(&mut self, token: u64, handle: &str, op: ReactorOp) -> Result<(), String> {
        let tok = Token(token as usize);
        let r = match op {
            ReactorOp::Read => match self.streams.get_mut(handle) {
                Some(s) => self.poll.registry().register(s, tok, Interest::READABLE),
                None => return Err(format!("không tìm thấy `{handle}` để đăng ký read")),
            },
            ReactorOp::Accept => match self.listeners.get_mut(handle) {
                Some(l) => self.poll.registry().register(l, tok, Interest::READABLE),
                None => return Err(format!("không tìm thấy `{handle}` để đăng ký accept")),
            },
        };
        match r {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {} // re-register cùng fd
            Err(e) => return Err(format!("epoll register `{handle}` thất bại: {e}")),
        }
        self.tokens.insert(token, handle.to_string());
        Ok(())
    }

    fn deregister(&mut self, token: u64, _handle: &str, op: ReactorOp) -> Result<(), String> {
        let Some(h) = self.tokens.remove(&token) else {
            return Ok(()); // idempotent
        };
        let r = match op {
            ReactorOp::Read => match self.streams.get_mut(&h) {
                Some(s) => self.poll.registry().deregister(s),
                None => return Ok(()), // tay đã đóng: kernel tự gỡ; fd có thể đã dùng lại
            },
            ReactorOp::Accept => match self.listeners.get_mut(&h) {
                Some(l) => self.poll.registry().deregister(l),
                None => return Ok(()),
            },
        };
        match r {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("epoll deregister `{h}` thất bại: {e}")),
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
        self.tokens.retain(|_, h| h != handle);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Backend scripted — kiểm chứng bất biến lõi TẤT ĐỊNH (không timing, không kernel).
// ---------------------------------------------------------------------------
struct Scripted {
    events: VecDeque<ReactorRawEvent>,
    reads: VecDeque<ReactorIoOutcome>,
    registered: Vec<(u64, String, ReactorOp)>,
    deregistered: Vec<u64>,
    fail_next_wait: Option<String>,
}
impl Scripted {
    fn new() -> Self {
        Self {
            events: VecDeque::new(),
            reads: VecDeque::new(),
            registered: Vec::new(),
            deregistered: Vec::new(),
            fail_next_wait: None,
        }
    }
    fn rc(self) -> Rc<RefCell<dyn FrameReactorBackend>> {
        Rc::new(RefCell::new(self))
    }
    fn ev(token: u64) -> ReactorRawEvent {
        ReactorRawEvent {
            token,
            readable: true,
            hup: false,
            error: false,
        }
    }
}
impl FrameReactorBackend for Scripted {
    fn try_read(&mut self, _h: &str, _max: usize) -> ReactorIoOutcome {
        self.reads
            .pop_front()
            .unwrap_or(ReactorIoOutcome::WouldBlock)
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
        if let Some(e) = self.fail_next_wait.take() {
            return Err(e);
        }
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
            std_mods: vec!["net".into()],
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

/// Chạy cùng nguồn trên cả hai engine (mỗi engine một backend riêng).
fn chay_duoi_engine(
    src: &str,
    mk: impl Fn() -> Rc<RefCell<dyn FrameReactorBackend>>,
) -> (FrameOutput, FrameOutput) {
    (chay_i(src, mk()), chay_v(src, mk()))
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

/// Server loopback: mỗi kết nối được phục vụ bởi MỘT luồng riêng ⇒ delay chồng lấp.
fn server(
    l: TcpListener,
    n: usize,
    delay_ms: u64,
    payload: &'static [u8],
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut kids = Vec::new();
        for _ in 0..n {
            let (mut s, _) = l.accept().expect("server accept");
            kids.push(thread::spawn(move || {
                if delay_ms > 0 {
                    thread::sleep(Duration::from_millis(delay_ms));
                }
                s.write_all(payload).expect("server write");
            }));
        }
        for k in kids {
            k.join().expect("server child");
        }
    })
}

fn program_read(n: usize, timeout: u64) -> String {
    let mut src = String::from("use std.net\n");
    for i in 1..=n {
        src.push_str(&format!(
            "async fn r{i}() -> int:\n    let x = net.read(\"C{i}\", 64, {timeout})\n    print(x)\n    {i}\n\n"
        ));
    }
    src.push_str("fn main():\n");
    for i in 1..=n {
        src.push_str(&format!("    let a{i} = spawn r{i}()\n"));
    }
    for i in 1..=n {
        src.push_str(&format!("    print(await a{i})\n"));
    }
    src
}

/// Cài `n` kết nối loopback vào backend + trả server handle.
fn setup_reads(
    n: usize,
    delay_ms: u64,
    payload: &'static [u8],
) -> (Reactor, thread::JoinHandle<()>) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = l.local_addr().expect("local_addr");
    let srv = server(l, n, delay_ms, payload);
    let mut r = Reactor::new();
    for i in 1..=n {
        r.add_stream(
            &format!("C{i}"),
            std::net::TcpStream::connect(addr).expect("connect"),
        );
    }
    (r, srv)
}

// ---------------------------------------------------------------------------
// (1) Đọc thật: dữ liệu, EOF, lỗi, timeout — cả hai engine.
// ---------------------------------------------------------------------------
const READ_ONE: &str = r#"use std.net

async fn r() -> int:
    print("PRE")
    let x = net.read("C1", 64, 2000)
    print(x)
    7

fn main():
    print(await spawn r())
"#;

#[test]
fn t1_read_real_data_both_engines() {
    let src = program_read(2, 2000);
    let (i, v) = chay_duoi_engine(&src, || {
        let (r, srv) = setup_reads(2, 60, b"PONG");
        srv.join().unwrap();
        r.rc()
    });
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(
            out.stdout.matches("Ok(PONG)").count(),
            2,
            "{:?}",
            out.stdout
        );
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

#[test]
fn t2_read_eof_removes_waiter_once() {
    // Peer đóng ngay ⇒ read trả Ok("") (EOF); snapshot về baseline.
    let (i, v) = chay_duoi_engine(READ_ONE, || {
        let (r, srv) = setup_reads(1, 0, b"");
        srv.join().unwrap();
        r.rc()
    });
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert!(out.stdout.contains("Ok()"), "{:?}", out.stdout);
        assert_eq!(out.stdout.matches("PRE").count(), 1, "không re-run operand");
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

#[test]
fn t3_read_error_propagates_as_result_err() {
    // Không có tay C1 ⇒ adapter trả Err rõ (không trap, không treo).
    let (i, v) = chay_duoi_engine(READ_ONE, || Reactor::new().rc());
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert!(out.stdout.contains("Err("), "{:?}", out.stdout);
        assert!(out.stdout.contains("C1"), "{:?}", out.stdout);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

#[test]
fn t4_read_timeout_wakes_once() {
    // Có socket nhưng không có dữ liệu: selector chờ tới deadline rồi trả Err.
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = l.local_addr().unwrap();
    let _keep = std::net::TcpStream::connect(addr).unwrap(); // peer giữ kết nối, không ghi
    let src = r#"use std.net

async fn r() -> int:
    let x = net.read("C1", 8, 80)
    print(x)
    1

fn main():
    print(await spawn r())
"#;
    let mk = || {
        let mut r = Reactor::new();
        r.add_stream("C1", std::net::TcpStream::connect(addr).unwrap());
        r.rc()
    };
    let (i, v) = chay_duoi_engine(src, mk);
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert!(out.stdout.contains("quá thời gian"), "{:?}", out.stdout);
        assert_eq!(
            out.stdout.matches("quá thời gian").count(),
            1,
            "timeout đúng 1 lần"
        );
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
    drop(_keep);
    drop(l);
}

// ---------------------------------------------------------------------------
// (2) accept thật: chưa có client rồi client nối; nhiều waiter đồng thời.
// ---------------------------------------------------------------------------
const ACCEPT_ONE: &str = r#"use std.net

async fn server() -> int:
    print("S:pre")
    let c = net.accept_timeout("L1", 2000)
    print(c)
    1

async fn bg() -> int:
    print("BG")
    2

fn main():
    let a = spawn server()
    let b = spawn bg()
    print(await a)
    print(await b)
"#;

#[test]
fn t5_accept_no_client_then_client_connects() {
    let mk = || {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(80));
            let _c = std::net::TcpStream::connect(addr).unwrap();
            thread::sleep(Duration::from_millis(200)); // giữ handle sống tới khi chạy xong
        });
        let mut r = Reactor::new();
        r.add_listener("L1", l);
        r.rc()
    };
    let (i, v) = chay_duoi_engine(ACCEPT_ONE, mk);
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert!(out.stdout.contains("Ok(C"), "{:?}", out.stdout);
        assert!(out.stdout.contains("BG"), "{:?}", out.stdout);
        assert_eq!(out.stdout.matches("S:pre").count(), 1, "không re-run");
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

const ACCEPT_TWO: &str = r#"use std.net

async fn s1() -> int:
    let c = net.accept_timeout("L1", 2000)
    print(c)
    1

async fn s2() -> int:
    let c = net.accept_timeout("L2", 2000)
    print(c)
    2

fn main():
    let a = spawn s1()
    let b = spawn s2()
    print(await a)
    print(await b)
"#;

#[test]
fn t6_multiple_simultaneous_accepts() {
    let mk = || {
        let mut r = Reactor::new();
        for (h, delay) in [("L1", 60u64), ("L2", 120)] {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = l.local_addr().unwrap();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(delay));
                let _c = std::net::TcpStream::connect(addr).unwrap();
                thread::sleep(Duration::from_millis(200));
            });
            r.add_listener(h, l);
        }
        r.rc()
    };
    let (i, v) = chay_duoi_engine(ACCEPT_TWO, mk);
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.stdout.matches("Ok(C").count(), 2, "{:?}", out.stdout);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ---------------------------------------------------------------------------
// (3) close-while-waiting: đóng tay đang chờ ⇒ stale event vô hiệu, Err đúng một lần.
// ---------------------------------------------------------------------------
const CLOSE_WAIT: &str = r#"use std.net

async fn r() -> int:
    print("R:pre")
    let x = net.read("C1", 16, 2000)
    print(x)
    1

async fn killer() -> int:
    net.close("C1")
    print("CLOSED")
    2

fn main():
    let a = spawn r()
    let b = spawn killer()
    print(await a)
    print(await b)
"#;

#[test]
fn t7_close_while_waiting_invalidates_old_events() {
    let mk = || {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let _peer = std::net::TcpStream::connect(addr).unwrap();
        let mut r = Reactor::new();
        r.add_stream("C1", std::net::TcpStream::connect(addr).unwrap());
        r.rc()
    };
    let (i, v) = chay_duoi_engine(CLOSE_WAIT, mk);
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(out.stdout.matches("R:pre").count(), 1, "không re-run");
        assert_eq!(out.stdout.matches("CLOSED").count(), 1);
        assert!(out.stdout.contains("Err("), "{:?}", out.stdout);
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ---------------------------------------------------------------------------
// (4) Lỗi selector lan rõ (không nuốt thành "không có event").
// ---------------------------------------------------------------------------
#[test]
fn t8_selector_error_propagates_clearly() {
    let mk = || {
        let mut s = Scripted::new();
        s.fail_next_wait = Some("epoll giả hỏng".into());
        s.rc()
    };
    let (i, v) = chay_duoi_engine(READ_ONE, mk);
    for out in [&i, &v] {
        assert_eq!(out.exit, 1, "{:?}", out.stdout);
        assert!(
            out.diagnostics
                .iter()
                .any(|d| d.message.contains("epoll giả hỏng")),
            "{:?}",
            out.diagnostics
        );
    }
}

// ---------------------------------------------------------------------------
// (5) Bất biến lõi TẤT ĐỊNH: token phân biệt generation; stale/fd-reuse không wake nhầm.
// ---------------------------------------------------------------------------
#[test]
fn t9_token_generation_stale_and_reuse_deterministic() {
    let b = Rc::new(RefCell::new(Scripted::new()));
    let mut s = Scheduler::new();
    s.frame_enable_for_test(vec![], vec![]);
    s.frame_install_backend_for_test(b.clone());
    s.frame.current = 1;
    let span = vis_syntax::Span::new(0, 0);

    // Lần đăng ký đầu: token 0, key 0, gen 0.
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 16, 1000, span)
        .unwrap_err();
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 0 },
        gen: 0,
    });
    assert_eq!(
        b.borrow().registered,
        vec![(0, "C1".into(), ReactorOp::Read)]
    );

    // Event token lạ (fd/tay khác) ⇒ stale, KHÔNG wake.
    b.borrow_mut().events.push_back(Scripted::ev(999));
    assert_eq!(s.frame_next(), None, "stale event không wake");
    assert!(s.frame.reactor.log.iter().any(|l| l == "rstale:token:999"));

    // Đúng token 0 ⇒ wake frame 1.
    b.borrow_mut().events.push_back(Scripted::ev(0));
    assert_eq!(s.frame_next(), Some(1));
    assert_eq!(s.frame.log.iter().filter(|l| *l == "wake:1").count(), 1);

    // Retry vẫn WouldBlock ⇒ re-arm: token MỚI (1), gen MỚI (1). Token 0 cũ thành stale.
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 16, 1000, span)
        .unwrap_err();
    assert_eq!(
        b.borrow().registered,
        vec![
            (0, "C1".into(), ReactorOp::Read),
            (1, "C1".into(), ReactorOp::Read)
        ]
    );
    s.frame_register(Waiter {
        frame: 1,
        wait: Wait::Reactor { key: 0, gen: 1 },
        gen: 1,
    });
    b.borrow_mut().events.push_back(Scripted::ev(0)); // event cũ
    assert_eq!(s.frame_next(), None, "token cũ sau re-arm phải stale");
    b.borrow_mut().events.push_back(Scripted::ev(1)); // event mới
    assert_eq!(s.frame_next(), Some(1));

    // Đóng tay: cancel dereg token mới + wake; mở lại C1 (fd có thể trùng) ⇒ token MỚI (2).
    s.frame.current = 2;
    s.frame_reactor_cancel_handle("C1");
    let _ = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 16, 1000, span)
        .unwrap_err();
    assert_eq!(
        b.borrow().registered.last().unwrap().0,
        2,
        "token tăng, không tái dùng"
    );
    s.frame_register(Waiter {
        frame: 2,
        wait: Wait::Reactor { key: 1, gen: 0 },
        gen: 0,
    });
    b.borrow_mut().events.push_back(Scripted::ev(1)); // event của tay cũ
    assert_eq!(s.frame_next(), None, "fd/tay dùng lại không wake nhầm task");

    b.borrow_mut()
        .reads
        .push_back(ReactorIoOutcome::Data(b"OK".to_vec()));
    b.borrow_mut().events.push_back(Scripted::ev(2));
    assert_eq!(s.frame_next(), Some(2));
    let v = s
        .frame_reactor_poll_io(ReactorOp::Read, "C1", 16, 1000, span)
        .unwrap();
    assert!(format!("{v:?}").contains("OK"), "{v:?}");
    assert!(
        s.frame.reactor.regs.is_empty(),
        "cleanup sau EOF/thành công"
    );
    assert!(s.frame.token_map.is_empty(), "sổ token sạch sau khi gỡ");
    s.frame_reactor_shutdown();
}

// ---------------------------------------------------------------------------
// (5b) Resume builtin KHÔNG eval lại đối số ⇒ không lặp side-effect.
// ---------------------------------------------------------------------------
const ARG_SIDE_EFFECT: &str = r#"use std.net

fn tay() -> str:
    print("ARG")
    "C1"

async fn r() -> int:
    let x = net.read(tay(), 64, 2000)
    print(x)
    1

fn main():
    print(await spawn r())
"#;

#[test]
fn t11_builtin_resume_does_not_reevaluate_args() {
    let (i, v) = chay_duoi_engine(ARG_SIDE_EFFECT, || {
        let (r, srv) = setup_reads(1, 60, b"ONCE");
        srv.join().unwrap();
        r.rc()
    });
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(
            out.stdout.matches("ARG").count(),
            1,
            "đối số bị eval lại: {:?}",
            out.stdout
        );
        assert!(out.stdout.contains("Ok(ONCE)"), "{:?}", out.stdout);
        assert_eq!(out.snapshot, baseline());
    }
}

// ---------------------------------------------------------------------------
// (5c) Nhiều waiter trên CÙNG một listener: mỗi client được phục vụ, không mất wake.
// ---------------------------------------------------------------------------
const ACCEPT_SAME: &str = r#"use std.net

async fn s() -> int:
    let c = net.accept_timeout("L1", 2000)
    print(c)
    1

fn main():
    let a = spawn s()
    let b = spawn s()
    print(await a)
    print(await b)
"#;

#[test]
fn t12_multiple_waiters_same_listener() {
    let (i, v) = chay_duoi_engine(ACCEPT_SAME, || {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        thread::spawn(move || {
            for d in [60u64, 120] {
                thread::sleep(Duration::from_millis(d));
                let _c = std::net::TcpStream::connect(addr).unwrap();
                thread::sleep(Duration::from_millis(200));
            }
        });
        let mut r = Reactor::new();
        r.add_listener("L1", l);
        r.rc()
    });
    for out in [&i, &v] {
        assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
        assert_eq!(
            out.stdout.matches("Ok(C").count(),
            2,
            "{:?}\n{:?}",
            out.stdout,
            out.log
        );
        assert_eq!(out.snapshot, baseline(), "{:?}", out.log);
    }
}

// ---------------------------------------------------------------------------
// (6) B1-real-frame — số đo (1/5/20, lặp 3 lần), startup vs workload riêng.
//     ~0,3 s là mục tiêu QUAN SÁT, không phải ngưỡng CI cứng.
// ---------------------------------------------------------------------------
#[test]
fn t10_b1_real_frame_overlap_measure() {
    for &n in &[1usize, 5, 20] {
        for rep in 1..=3 {
            let src = program_read(n, 2000);
            let t0 = Instant::now();
            let (hir, syms, tys) = tien_trinh(&src);
            let (r, srv) = setup_reads(n, 200, b"X");
            let startup = t0.elapsed();
            let t1 = Instant::now();
            let out = run_i(&src, &hir, &syms, &tys, &[], cfg(r.rc()));
            let workload_i = t1.elapsed();
            srv.join().unwrap();
            assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
            assert_eq!(out.stdout.matches("Ok(X)").count(), n, "{:?}", out.stdout);
            println!(
                "B1-real-frame interp n={n} rep={rep}: startup={:.3?} workload={:.3?} ok={n}",
                startup, workload_i
            );
            // Chỉ chống regressions chồng lấp thô khi N≥5 (N=1 không có gì để chồng).
            // Đây KHÔNG phải ngưỡng ~0,3 s — số đo thực được in ở trên.
            if n >= 5 {
                assert!(
                    workload_i < Duration::from_millis((n as u64 * 200) / 2),
                    "read tuần tự (N×200 ms) thay vì chồng lấp: {workload_i:?}"
                );
            }

            let (r2, srv2) = setup_reads(n, 200, b"X");
            let t2 = Instant::now();
            let outv = chay_v(&src, r2.rc());
            let workload_v = t2.elapsed();
            srv2.join().unwrap();
            assert_eq!(outv.exit, 0, "{:?}", outv.diagnostics);
            assert_eq!(outv.stdout.matches("Ok(X)").count(), n, "{:?}", outv.stdout);
            println!(
                "B1-real-frame vm     n={n} rep={rep}: workload={:.3?} ok={n}",
                workload_v
            );
        }
    }
}
