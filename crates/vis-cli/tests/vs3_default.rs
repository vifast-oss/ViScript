// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d** — flip default cho chương trình **có `async fn`** sang frame scheduler.
//!
//! Hợp đồng:
//! - chương trình sync (`không` có `async fn`) đi nguyên `Interp::run`/`Vm::run` — byte-identical;
//! - chương trình async đi `run_async_with_*` (frame scheduler) khi HIR có `async fn`;
//! - `chan(0)` là dấu hiệu phân biệt: runtime sync từ chối, async default chạy được;
//! - `Pipe` đã parity trên VM default; frame async cũng chạy;
//! - cache `--vm` warm/cold giữ cùng stdout/exit.
//!
//! Không đổi bytecode/VERSION/cache key: chỉ chọn entry runtime.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

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

const ASYNC_CHAN0: &str = r#"async fn sender(c: Chan[int]) -> int:
    c.send(7)
    1

fn main():
    let c = chan[int](0)
    let t = spawn sender(c)
    let x = c.recv()
    print(x)
"#;

#[test]
fn async_default_chay_chan0_ca_hai_engine() {
    let (hir, syms, tys) = tien_trinh(ASYNC_CHAN0);
    let i = vis_interp::run_async_with_hosts_caps(
        ASYNC_CHAN0,
        &hir,
        &syms,
        &tys,
        &[],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(i.exit, 0, "async interp: {:?}", i.diagnostics);
    assert_eq!(i.stdout, "Some(7)\n");

    let m = vis_bytecode::compile(&hir, &syms, ASYNC_CHAN0).expect("compile");
    let v =
        vis_vm::run_async_with_full_hosts_caps(&m, ASYNC_CHAN0, &[], None, None, None, None, &[]);
    assert_eq!(v.exit, 0, "async VM: {:?}", v.diagnostics);
    assert_eq!(v.stdout, i.stdout);

    // Runtime sync (đường cũ) vẫn từ chối `chan(0)` — chứng minh entry đã đổi thật.
    let old_i = vis_interp::run(ASYNC_CHAN0, &hir, &syms, &tys, &[]);
    assert_ne!(old_i.exit, 0);
    assert_eq!(old_i.codes(), vec!["VS0505"]);
    let mut old_vm = vis_vm::Vm::new(&m, ASYNC_CHAN0);
    let old_v = old_vm.run(&[]);
    assert_ne!(old_v.exit, 0);
    assert_eq!(old_v.codes(), vec!["VS0505"]);
}

#[test]
fn sync_khong_doi_chan0_van_tu_choi() {
    let src = "fn main():\n    let c = chan[int](0)\n    let _ = c.send(1)\n";
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_eq!(i.codes(), vec!["VS0505"]);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let mut vm = vis_vm::Vm::new(&m, src);
    let v = vm.run(&[]);
    assert_eq!(v.codes(), vec!["VS0505"]);
}

const PIPE_SYNC: &str = r#"fn id(n: int) -> int:
    n

fn main():
    print(5 |> id)
"#;

const PIPE_ASYNC: &str = r#"async fn noop() -> int:
    1

fn id(n: int) -> int:
    n

fn main():
    print(await spawn noop())
    print(5 |> id)
"#;

#[test]
fn pipe_parity_default_va_async() {
    let (hir, syms, tys) = tien_trinh(PIPE_SYNC);
    let di = vis_interp::run(PIPE_SYNC, &hir, &syms, &tys, &[]);
    let m = vis_bytecode::compile(&hir, &syms, PIPE_SYNC).expect("compile");
    let mut vm = vis_vm::Vm::new(&m, PIPE_SYNC);
    let dv = vm.run(&[]);
    assert_eq!(di.stdout, "5\n");
    assert_eq!(dv.stdout, "5\n");
    assert_eq!(di.exit, dv.exit);

    let (ha, sa, ta) = tien_trinh(PIPE_ASYNC);
    let ai = vis_interp::run_async_with_hosts_caps(
        PIPE_ASYNC,
        &ha,
        &sa,
        &ta,
        &[],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(ai.exit, 0, "{:?}", ai.diagnostics);
    assert_eq!(ai.stdout, "1\n5\n");
    let ma = vis_bytecode::compile(&ha, &sa, PIPE_ASYNC).expect("compile");
    let av =
        vis_vm::run_async_with_full_hosts_caps(&ma, PIPE_ASYNC, &[], None, None, None, None, &[]);
    assert_eq!(av.stdout, "1\n5\n");
}

#[test]
fn async_nhan_args_va_entry_frame() {
    let src = "async fn noop() -> int:\n    1\nfn main(args: list[str]):\n    print(args.len())\n";
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run_async_with_hosts_caps(
        src,
        &hir,
        &syms,
        &tys,
        &["a".into(), "b".into()],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "2\n");
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = vis_vm::run_async_with_full_hosts_caps(
        &m,
        src,
        &["a".into(), "b".into()],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(v.exit, 0, "{:?}", v.diagnostics);
    assert_eq!(v.stdout, "2\n");
}

#[test]
fn guard_scheduler_trap_ro_khong_treo() {
    // `spin` tạo task lồng vô hạn; hạ trần test xuống 100 turn để chứng minh guard có trap rõ
    // (production vẫn 200_000, hook chỉ test).
    let src = "async fn spin() -> int:\n    let t = spawn spin()\n    await t\n\nfn main():\n    print(await spin())\n";
    let (hir, syms, tys) = tien_trinh(src);
    vis_interp::rt::frame_set_max_turns_for_test(Some(100));
    let fi = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    vis_interp::rt::frame_set_max_turns_for_test(None);
    assert_ne!(fi.exit, 0, "guard phải trap");
    assert!(
        fi.diagnostics
            .iter()
            .any(|d| d.message.contains("scheduler quá giới hạn (100 turn)")),
        "{:?}",
        fi.diagnostics
    );

    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    vis_interp::rt::frame_set_max_turns_for_test(Some(100));
    let fv = run_v(&m, src, &[], FrameConfig::default());
    vis_interp::rt::frame_set_max_turns_for_test(None);
    assert_ne!(fv.exit, 0, "guard VM phải trap");
    assert!(
        fv.diagnostics
            .iter()
            .any(|d| d.message.contains("scheduler quá giới hạn (100 turn)")),
        "{:?}",
        fv.diagnostics
    );
}

// ---------------------------------------------------------------------------
// CLI: chọn entry + cache warm/cold
// ---------------------------------------------------------------------------

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-1d-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("cache")).expect("tmp");
    d
}

fn viet(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    p
}

fn chay(d: &Path, args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d)
        .env("XDG_CACHE_HOME", d.join("cache"))
        .args(args);
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn cli_async_chan0_va_sync_byte_identical() {
    let d = tam("cli");
    let a = viet(&d, "a.vis", ASYNC_CHAN0);
    let s = viet(&d, "s.vis", "fn main():\n    print(1 + 2)\n");
    for vm in [false, true] {
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        let mut aa = args.clone();
        aa.push(a.to_str().unwrap());
        let (ca, oa, ea) = chay(&d, &aa);
        assert_eq!(ca, 0, "async vm={vm}: {ea}");
        assert_eq!(oa, "Some(7)\n", "async vm={vm}");

        let mut ss = args;
        ss.push(s.to_str().unwrap());
        let (cs, os, es) = chay(&d, &ss);
        assert_eq!(cs, 0, "sync vm={vm}: {es}");
        assert_eq!(os, "3\n", "sync vm={vm}");
    }
}

const ASYNC_FS: &str = r#"use std.fs

async fn read() -> str:
    match fs.read("x.txt"):
        Ok(s): s
        Err(e): "err"

fn main():
    print(await spawn read())
"#;

#[test]
fn cli_async_noi_host_fs() {
    let d = tam("fs");
    std::fs::write(d.join("x.txt"), "hello").expect("ghi x.txt");
    let p = viet(&d, "fs.vis", ASYNC_FS);
    for vm in [false, true] {
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        args.push(p.to_str().unwrap());
        let (c, out, err) = chay(&d, &args);
        assert_eq!(c, 0, "fs vm={vm}: {err}");
        assert_eq!(out, "hello\n", "fs vm={vm}");
    }
}

// ---------------------------------------------------------------------------
// Async net production (loopback): backend mio trong CLI.
// ---------------------------------------------------------------------------

/// Các ca net dùng port loopback + server thread: serialize trong test binary để tránh
/// tranh port khi `cargo test` chạy song song (không ảnh hưởng sản phẩm).
static NET_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn server_mot_lan(noi_dung: &'static [u8]) -> String {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind server");
    let addr = l.local_addr().expect("addr").to_string();
    thread::spawn(move || {
        for _ in 0..8 {
            if let Ok((mut s, _)) = l.accept() {
                let _ = s.write_all(noi_dung);
            }
        }
    });
    addr
}

const ASYNC_NET_READ: &str = r#"use std.net

async fn doc(addr: str) -> str:
    match net.connect_timeout(addr, 1000):
        Ok(t):
            let r = match net.read(t, 32, 1000):
                Ok(s): s
                Err(e): "loi-doc"
            let _ = net.close(t)
            r
        Err(e): "loi-noi"

fn main(args: list[str]):
    print(await spawn doc(args[0]))
"#;

#[test]
fn cli_async_net_read_loopback_hai_engine() {
    let _net = NET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tam("net");
    let p = viet(&d, "net.vis", ASYNC_NET_READ);
    for vm in [false, true] {
        let addr = server_mot_lan(b"hello");
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        args.push(p.to_str().unwrap());
        args.push("--");
        args.push(&addr);
        let (c, out, err) = chay(&d, &args);
        assert_eq!(c, 0, "net vm={vm}: {err}");
        assert_eq!(out, "hello\n", "net vm={vm}");
    }
}

fn server_http_mot_lan() -> String {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind http");
    let addr = l.local_addr().expect("addr").to_string();
    thread::spawn(move || {
        if let Ok((mut s, _)) = l.accept() {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let _ =
                s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        }
    });
    addr
}

const ASYNC_HTTP: &str = r#"use std.http

async fn lay(url: str) -> str:
    match http.get(url):
        Ok(s): s
        Err(e): "loi-http: {e}"

fn main(args: list[str]):
    print(await spawn lay(args[0]))
"#;

fn server_dem(so_byte: usize) -> (String, thread::JoinHandle<Vec<u8>>) {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind dem");
    let addr = l.local_addr().expect("addr").to_string();
    let h = thread::spawn(move || {
        let (mut s, _) = l.accept().expect("accept dem");
        let mut all = Vec::new();
        let mut buf = [0u8; 8192];
        while all.len() < so_byte {
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => all.extend_from_slice(&buf[..n]),
                Err(e) => panic!("server đọc: {e}"),
            }
        }
        all
    });
    (addr, h)
}

const ASYNC_NET_WRITE: &str = r#"use std.net
use std.str

async fn ghi(addr: str) -> int:
    let data = str.repeat("x", 300_000)
    match net.connect_timeout(addr, 1000):
        Ok(t):
            let r = net.write(t, data)
            print(r)
            let _ = net.close(t)
        Err(e): print(e)
    1

fn main(args: list[str]):
    print(await spawn ghi(args[0]))
"#;

const ASYNC_NET_ACCEPT: &str = r#"use std.net

async fn serve(addr: str) -> int:
    match net.listen(addr):
        Ok(l):
            match net.accept_timeout(l, 2000):
                Ok(t):
                    let r = net.read(t, 8, 1000)
                    print(r)
                    let _ = net.close(t)
                Err(e): print(e)
            let _ = net.close_listen(l)
        Err(e): print(e)
    1

fn main(args: list[str]):
    print(await spawn serve(args[0]))
"#;

const ASYNC_NET_TIMEOUT: &str = r#"use std.net

async fn cho(addr: str) -> int:
    match net.connect_timeout(addr, 1000):
        Ok(t):
            print(net.read(t, 8, 50))
            let _ = net.close(t)
        Err(e): print(e)
    1

fn main(args: list[str]):
    print(await spawn cho(args[0]))
"#;

#[test]
fn cli_async_net_read_timeout_loopback_hai_engine() {
    let _net = NET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tam("nettimeout");
    let p = viet(&d, "nettimeout.vis", ASYNC_NET_TIMEOUT);
    for vm in [false, true] {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind timeout");
        let addr = l.local_addr().expect("addr").to_string();
        thread::spawn(move || {
            if let Ok((_s, _)) = l.accept() {
                thread::sleep(std::time::Duration::from_millis(500));
            }
        });
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        args.push(p.to_str().unwrap());
        args.push("--");
        args.push(&addr);
        let (c, out, err) = chay(&d, &args);
        assert_eq!(c, 0, "timeout vm={vm}: {err}");
        assert!(out.contains("Err("), "timeout vm={vm}: {out:?}");
        assert!(out.contains("thời gian"), "timeout vm={vm}: {out:?}");
    }
}

#[test]
fn cli_async_net_accept_loopback_hai_engine() {
    let _net = NET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tam("netaccept");
    let p = viet(&d, "netaccept.vis", ASYNC_NET_ACCEPT);
    for vm in [false, true] {
        let l = TcpListener::bind("127.0.0.1:0").expect("bind port");
        let addr = l.local_addr().expect("addr").to_string();
        drop(l);
        let addr2 = addr.clone();
        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(200));
            if let Ok(mut s) = std::net::TcpStream::connect(&addr2) {
                let _ = s.write_all(b"hi");
            }
        });
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        args.push(p.to_str().unwrap());
        args.push("--");
        args.push(&addr);
        let (c, out, err) = chay(&d, &args);
        assert_eq!(c, 0, "accept vm={vm}: {err}");
        assert_eq!(out, "Ok(hi)\n1\n", "accept vm={vm}: {out:?}");
    }
}

#[test]
fn cli_async_net_write_partial_loopback_hai_engine() {
    let _net = NET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tam("netwrite");
    let p = viet(&d, "netwrite.vis", ASYNC_NET_WRITE);
    for vm in [false, true] {
        let (addr, h) = server_dem(300_000);
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        args.push(p.to_str().unwrap());
        args.push("--");
        args.push(&addr);
        let (c, out, err) = chay(&d, &args);
        assert_eq!(c, 0, "write vm={vm}: {err}");
        assert!(out.contains("Ok(())"), "write vm={vm}: {out:?}");
        let data = h.join().expect("server write");
        assert_eq!(data.len(), 300_000, "write vm={vm}: thiếu byte");
    }
}

#[test]
fn cli_async_http_get_loopback_hai_engine() {
    let _net = NET_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let d = tam("http");
    let p = viet(&d, "http.vis", ASYNC_HTTP);
    for vm in [false, true] {
        let addr = server_http_mot_lan();
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        args.push(p.to_str().unwrap());
        let url = format!("http://{addr}/");
        args.extend(["--", &url]);
        let (c, out, err) = chay(&d, &args);
        assert_eq!(c, 0, "http vm={vm}: {err}");
        assert_eq!(out, "ok\n", "http vm={vm}");
    }
}

#[test]
fn cli_cache_warm_cold_async_vm() {
    let d = tam("cache");
    let a = viet(&d, "a.vis", ASYNC_CHAN0);
    let a = a.to_str().unwrap();
    let (c1, o1, e1) = chay(&d, &["run", "--vm", a]);
    assert_eq!((c1, o1.as_str()), (0, "Some(7)\n"), "{e1}");
    let (c2, o2, e2) = chay(&d, &["run", "--vm", a]);
    assert_eq!((c2, o2.as_str()), (0, "Some(7)\n"), "{e2}");
    let (c3, o3, e3) = chay(&d, &["run", "--vm", "--no-cache", a]);
    assert_eq!((c3, o3.as_str()), (0, "Some(7)\n"), "{e3}");
    assert_eq!(o1, o2);
    assert_eq!(o1, o3);
}
