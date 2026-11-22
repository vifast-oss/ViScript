// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a** — test native cho `WasiHost`: preopen/deny, clock/stdout, từ chối net/proc/sqlite,
//! và parity interp↔VM qua runner một tệp.
//!
//! Không cần `wasmtime`: cùng mã `WasiFs`/`runner` chạy native trên thư mục tạm. Phần chạy **wasm
//! thật** do `bin/vis-wasm-suite` đảm nhiệm (xem JOURNAL).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use vis_host_wasm::runner::{chay, Engine, YeuCau};
use vis_host_wasm::{Preopen, Stdio, WasiFs};
use vis_interp::builtin::FsHost;

static DEM: AtomicU64 = AtomicU64::new(0);

struct Tam {
    p: PathBuf,
}

impl Drop for Tam {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.p);
    }
}

fn tam(ten: &str) -> Tam {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("vis-host-wasm-{}-{}-{ten}", std::process::id(), n));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).expect("tạo thư mục tạm");
    Tam { p }
}

fn ghi(p: &Path, s: &str) {
    if let Some(cha) = p.parent() {
        fs::create_dir_all(cha).expect("tạo cha");
    }
    fs::write(p, s).expect("ghi tệp");
}

fn host(root: &Path, ro: bool) -> WasiFs {
    let pre = if ro {
        Preopen::chi_doc("/data", root)
    } else {
        Preopen::moi("/data", root)
    };
    WasiFs::moi(&[pre]).expect("preopen")
}

#[test]
fn doc_ghi_trong_preopen() {
    let d = tam("trong");
    ghi(&d.p.join("root/a.txt"), "xin chào");
    let mut h = host(&d.p.join("root"), false);

    assert_eq!(h.read("/data/a.txt").unwrap(), "xin chào");
    h.write("/data/b.txt", "mới").unwrap();
    assert_eq!(h.read("/data/./b.txt").unwrap(), "mới");
    assert_eq!(h.read("/data/x/../a.txt").unwrap(), "xin chào");
    assert!(h.exists("/data/b.txt"));
    assert!(!h.exists("/data/khong-co.txt"));
    assert_eq!(h.list("/data").unwrap(), ["a.txt", "b.txt"]);
    assert_eq!(h.read_lines("/data/a.txt").unwrap(), ["xin chào"]);
    assert_eq!(h.size("/data/a.txt").unwrap(), "xin chào".len() as i64);
    assert!(h.is_file("/data/a.txt").unwrap());
    assert!(h.is_dir("/data").unwrap());
    assert!(!h.is_symlink("/data/a.txt").unwrap());
    assert_eq!(h.canonicalize("/data/./b.txt").unwrap(), "/data/b.txt");
    assert_eq!(h.current_dir().unwrap(), "/data");
    h.append("/data/a.txt", "!").unwrap();
    assert_eq!(h.read("/data/a.txt").unwrap(), "xin chào!");
    h.mkdir("/data/thu-muc").unwrap();
    h.mkdir_all("/data/a/b/c").unwrap();
    assert!(h.is_dir("/data/a/b/c").unwrap());
    assert_eq!(
        h.walk("/data").unwrap(),
        ["a.txt", "a/", "a/b/", "a/b/c/", "b.txt", "thu-muc/",]
    );
    h.copy("/data/b.txt", "/data/c.txt").unwrap();
    assert_eq!(h.read("/data/c.txt").unwrap(), "mới");
    h.rename("/data/c.txt", "/data/d.txt").unwrap();
    assert!(!h.exists("/data/c.txt"));
    assert_eq!(h.read("/data/d.txt").unwrap(), "mới");
    h.remove("/data/d.txt").unwrap();
    assert!(!h.exists("/data/d.txt"));
    h.write_bytes("/data/raw.bin", &[0, 255, 1]).unwrap();
    assert_eq!(h.read_bytes("/data/raw.bin").unwrap(), [0, 255, 1]);
}

#[test]
fn duong_tuong_doi_theo_preopen_dau() {
    let d = tam("tuong-doi");
    fs::create_dir_all(d.p.join("root")).unwrap();
    let mut h = host(&d.p.join("root"), false);
    h.write("tuong-doi.txt", "ok").unwrap();
    assert_eq!(h.read("tuong-doi.txt").unwrap(), "ok");
    assert!(d.p.join("root/tuong-doi.txt").is_file());
}

#[test]
fn tu_choi_ngoai_preopen() {
    let d = tam("ngoai");
    fs::create_dir_all(d.p.join("root")).unwrap();
    ghi(&d.p.join("ngoai.txt"), "bí mật");
    let mut h = host(&d.p.join("root"), false);

    for p in [
        "/etc/passwd",
        "/data/../ngoai.txt",
        "../ngoai.txt",
        "/data2/x",
    ] {
        let e = h.read(p).unwrap_err();
        assert!(e.contains("VS0604"), "{p}: {e}");
        assert!(e.contains("preopen"), "{p}: {e}");
    }
    assert!(!h.exists("/etc/passwd"));
    // Preopen `/data` không được khớp tiền tố byte `/data2`.
    let e = h.write("/data2/x", "x").unwrap_err();
    assert!(e.contains("VS0604"), "{e}");
}

#[test]
fn preopen_chi_doc_tu_choi_ghi() {
    let d = tam("ro");
    ghi(&d.p.join("root/a.txt"), "đọc được");
    let mut h = host(&d.p.join("root"), true);
    assert_eq!(h.read("/data/a.txt").unwrap(), "đọc được");
    for e in [
        h.write("/data/b.txt", "x").unwrap_err(),
        h.append("/data/a.txt", "x").unwrap_err(),
        h.mkdir("/data/x").unwrap_err(),
        h.mkdir_all("/data/x/y").unwrap_err(),
        h.remove("/data/a.txt").unwrap_err(),
        h.copy("/data/a.txt", "/data/b.txt").unwrap_err(),
        h.rename("/data/a.txt", "/data/b.txt").unwrap_err(),
        h.write_bytes("/data/b.bin", &[1]).unwrap_err(),
    ] {
        assert!(e.contains("chỉ đọc"), "{e}");
        assert!(e.contains("VS0604"), "{e}");
    }
}

#[cfg(unix)]
#[test]
fn symlink_ra_ngoai_bi_chan() {
    use std::os::unix::fs::symlink;
    let d = tam("symlink");
    let root = d.p.join("root");
    fs::create_dir_all(&root).unwrap();
    ghi(&d.p.join("ngoai.txt"), "ngoài");
    ghi(&root.join("trong.txt"), "trong");
    symlink(d.p.join("ngoai.txt"), root.join("link-ra")).unwrap();
    symlink(root.join("trong.txt"), root.join("link-vao")).unwrap();
    let mut h = host(&root, false);

    let e = h.read("/data/link-ra").unwrap_err();
    assert!(e.contains("VS0604"), "{e}");
    let e = h.write("/data/link-ra", "x").unwrap_err();
    assert!(e.contains("VS0604"), "{e}");
    assert!(h.is_symlink("/data/link-ra").unwrap());
    assert_eq!(h.read("/data/link-vao").unwrap(), "trong");
    assert!(h.read_link("/data/link-ra").unwrap().contains("ngoai.txt"));
    // walk bỏ symlink hoàn toàn (không in, không đệ quy).
    let ds = h.walk("/data").unwrap();
    assert!(!ds.iter().any(|x| x.contains("link-")), "{ds:?}");
}

#[test]
fn duong_dan_kho_khong_panic() {
    let d = tam("kho");
    fs::create_dir_all(d.p.join("root")).unwrap();
    let mut h = host(&d.p.join("root"), false);
    for p in ["", "/", ".", "..", "/data/..", "/data/", "/data/a\0b"] {
        let _ = h.read(p);
        let _ = h.write(p, "x");
        let _ = h.list(p);
        let _ = h.walk(p);
        let _ = h.exists_quyen(p);
        let _ = h.size(p);
        let _ = h.canonicalize(p);
    }
    // Không có preopen ⇒ mọi thao tác bị từ chối, không panic.
    let mut rong = WasiFs::moi(&[]).unwrap();
    assert!(rong.read("/x").unwrap_err().contains("VS0604"));
    assert!(rong.current_dir().is_err());
}

#[test]
fn stdio_bo_dem_va_that() {
    let mut s = Stdio::bo();
    s.ghi_out("một").unwrap();
    s.ghi_err("hai").unwrap();
    assert_eq!(s.out(), "một".as_bytes());
    assert_eq!(s.err(), "hai".as_bytes());
    // `That` ghi thật nhưng không panic (nội dung đi ra ngoài test harness).
    let mut t = Stdio::that();
    t.ghi_err("").unwrap();
}

fn chay_voi_preopen(src: &str, engine: Engine, root: &Path) -> vis_host_wasm::runner::KetQua {
    chay(&YeuCau {
        nguon: src.to_string(),
        ten: "test.vis".to_string(),
        mode: vis_syntax::Mode::Script,
        args: Vec::new(),
        engine,
        preopens: vec![Preopen::moi("/data", root)],
    })
}

#[test]
fn runner_parity_doc_ghi_fs() {
    let src = "use std.fs\nfn main():\n    match fs.write(\"/data/ra.txt\", \"nội dung\"):\n        Ok(_):\n            match fs.read(\"/data/ra.txt\"):\n                Ok(s):\n                    print(s)\n                Err(e):\n                    print(\"ERR-DOC\")\n        Err(e):\n            print(\"ERR-GHI\")\n";
    let a_dir = tam("parity-a");
    let b_dir = tam("parity-b");
    fs::create_dir_all(&a_dir.p).unwrap();
    fs::create_dir_all(&b_dir.p).unwrap();
    let a = chay_voi_preopen(src, Engine::Interp, &a_dir.p);
    let b = chay_voi_preopen(src, Engine::Vm, &b_dir.p);
    assert_eq!(a.exit, 0, "interp: {}", a.stderr);
    assert_eq!(b.exit, 0, "vm: {}", b.stderr);
    assert_eq!(a.stdout, "nội dung\n", "interp: {}", a.stderr);
    assert_eq!(b.stdout, a.stdout, "VM lệch interp: {}", b.stderr);
}

#[test]
fn runner_dong_ho_va_tu_choi_sqlite() {
    let clock = "use std.time\nfn main():\n    if time.now_ms() > 0 and time.monotonic_ms() >= 0:\n        print(\"CLOCK\")\n    else:\n        print(\"BAD\")\n";
    for engine in [Engine::Interp, Engine::Vm] {
        let kq = chay_voi_preopen(clock, engine, Path::new("/tmp"));
        assert_eq!(kq.exit, 0, "{engine:?}: {}", kq.stderr);
        assert_eq!(kq.stdout, "CLOCK\n", "{engine:?}: {}", kq.stderr);
    }
    let sqlite = "use std.sqlite\nfn main():\n    match sqlite.open(\":memory:\"):\n        Ok(h):\n            print(\"NO\")\n        Err(e):\n            print(\"CHAN\")\n";
    for engine in [Engine::Interp, Engine::Vm] {
        let kq = chay_voi_preopen(sqlite, engine, Path::new("/tmp"));
        assert_eq!(kq.exit, 0, "{engine:?}: {}", kq.stderr);
        assert_eq!(kq.stdout, "CHAN\n", "{engine:?}: {}", kq.stderr);
    }
}

#[test]
fn runner_tu_choi_ghi_ngoai_preopen() {
    let src = "use std.fs\nfn main():\n    match fs.write(\"/etc/x\", \"x\"):\n        Ok(_):\n            print(\"GHI\")\n        Err(e):\n            print(\"CHAN\")\n";
    let d = tam("runner-ngoai");
    fs::create_dir_all(&d.p).unwrap();
    for engine in [Engine::Interp, Engine::Vm] {
        let kq = chay_voi_preopen(src, engine, &d.p);
        // Host trả dấu quyền ⇒ trap VS0604 ⇒ exit 2, không phải `Err` dữ liệu.
        assert_eq!(
            kq.exit, 2,
            "{engine:?}: stdout={:?} {}",
            kq.stdout, kq.stderr
        );
        assert!(kq.stderr.contains("VS0604"), "{engine:?}: {}", kq.stderr);
        assert!(kq.stdout.is_empty(), "{engine:?}: {:?}", kq.stdout);
    }
}
