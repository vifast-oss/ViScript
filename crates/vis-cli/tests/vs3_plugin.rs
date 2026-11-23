// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307 (VD-278/VD-281/VD-283)** — `vis plugin` trên binary thật: nạp plugin WASI/WASM,
//! kiểm `VIS_PLUGIN_ABI_V1`, sandbox `cap[Dir]`, tắt cứng `--strict-caps`/`--sandbox`.
//!
//! Plugin mẫu dựng bằng `rustc --target wasm32-wasip1` (xem `common::plugin_mau_wasm`); runtime
//! dò `wasmtime` → Python `wasmtime` → Node. Thiếu môi trường ⇒ `BLOCKED-ENV`, không bỏ qua âm thầm.

mod common;

use std::path::PathBuf;
use std::process::{Command, Output};

use common::{plugin_mau_wasm, vis_bin};

fn chay(args: &[&str]) -> Output {
    Command::new(vis_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("chạy `vis {}`: {e}", args.join(" ")))
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn co_runtime() -> bool {
    vis_plugin_abi::wasi::tim_runtime().is_ok()
}

fn thu_muc_tam(ten: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-cli-plugin-{ten}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn chuan_bi(_ten: &str) -> Option<(PathBuf, String)> {
    if !co_runtime() {
        eprintln!("BLOCKED-ENV: không có runtime WASI (wasmtime/python-wasmtime/node)");
        return None;
    }
    match plugin_mau_wasm() {
        Some(p) => Some((p.clone(), p.to_string_lossy().into_owned())),
        None => {
            eprintln!("BLOCKED-ENV: không dựng được wasm32-wasip1 cho plugin mẫu");
            None
        }
    }
}

#[test]
fn check_va_run_plugin_wasi() {
    let Some((_wasm, w)) = chuan_bi("check") else {
        return;
    };
    let o = chay(&["plugin", "check", &w]);
    assert!(o.status.success(), "check: {}", stderr(&o));
    let s = stdout(&o);
    assert!(s.contains("plugin: mau"), "{s}");
    assert!(s.contains("abi: 1"), "{s}");
    assert!(s.contains("hàm: cong(int, int) -> int"), "{s}");
    assert!(s.contains("runtime: "), "{s}");

    let o = chay(&["plugin", "run", &w, "cong", "int:2", "int:3"]);
    assert!(o.status.success(), "run: {}", stderr(&o));
    assert_eq!(stdout(&o).trim(), "5");
}

#[test]
fn tat_cung_strict_caps_va_sandbox_truoc_khi_doc_tep() {
    let khong = thu_muc_tam("khong-co").join("khong-ton-tai.wasm");
    let k = khong.to_string_lossy().into_owned();
    for co in ["--strict-caps", "--sandbox"] {
        let o = chay(&["plugin", "check", &k, co]);
        assert_eq!(o.status.code(), Some(2), "{co}: {}", stderr(&o));
        let e = stderr(&o);
        assert!(e.contains("VS0604"), "{co}: {e}");
        assert!(e.contains("tắt"), "{co}: {e}");
        assert!(
            !e.contains("không đọc được"),
            "{co}: phải chặn trước khi đọc tệp: {e}"
        );
    }
}

#[test]
fn cap_dir_ro_rw_va_ngoai_root() {
    let Some((_wasm, w)) = chuan_bi("cap") else {
        return;
    };
    let d = thu_muc_tam("cap");
    let f = d.join("in.txt");
    std::fs::write(&f, "xin chào wasi").expect("fixture");
    let dstr = d.to_string_lossy().into_owned();

    // Không cap ⇒ plugin không thấy `/cap0`.
    let o = chay(&["plugin", "run", &w, "doc", "str:/cap0/in.txt"]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("VS0604"), "{}", stderr(&o));

    // cap chỉ đọc ⇒ đọc được, ghi bị từ chối, tệp gốc không đổi.
    let o = chay(&[
        "plugin",
        "run",
        &w,
        "doc",
        "str:/cap0/in.txt",
        "--cap-dir",
        &dstr,
    ]);
    assert!(o.status.success(), "doc: {}", stderr(&o));
    assert_eq!(stdout(&o).trim(), "xin chào wasi");

    let o = chay(&[
        "plugin",
        "run",
        &w,
        "ghi",
        "str:/cap0/in.txt",
        "str:moi",
        "--cap-dir",
        &dstr,
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("VS0604"), "{}", stderr(&o));
    assert_eq!(
        std::fs::read_to_string(&f).expect("fixture"),
        "xin chào wasi"
    );

    // cap ghi ⇒ ghi được.
    let rw = format!("{dstr}=rw");
    let o = chay(&[
        "plugin",
        "run",
        &w,
        "ghi",
        "str:/cap0/in.txt",
        "str:moi",
        "--cap-dir",
        &rw,
    ]);
    assert!(o.status.success(), "ghi: {}", stderr(&o));
    assert_eq!(stdout(&o).trim(), "()");
    assert_eq!(std::fs::read_to_string(&f).expect("đọc lại"), "moi");

    // Ngoài preopen ⇒ WASI từ chối.
    let o = chay(&[
        "plugin",
        "run",
        &w,
        "doc",
        "str:/etc/passwd",
        "--cap-dir",
        &dstr,
    ]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("VS0604"), "{}", stderr(&o));
}

#[test]
fn tu_choi_abi_lech_va_plugin_la() {
    let Some((wasm, _)) = chuan_bi("abi") else {
        return;
    };
    let d = thu_muc_tam("abi");
    let goc = std::fs::read(&wasm).expect("đọc wasm");
    let marker = b"vis_plugin_abi_v1";
    let pos = goc
        .windows(marker.len())
        .position(|x| x == marker)
        .expect("marker");
    let mut sai = goc.clone();
    sai[pos + marker.len() - 1] = b'2';
    let p_sai = d.join("abi2.wasm");
    std::fs::write(&p_sai, &sai).expect("ghi");
    let o = chay(&["plugin", "check", p_sai.to_str().expect("utf8")]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    let e = stderr(&o);
    assert!(e.contains("VS0501"), "{e}");
    assert!(e.contains("ABI") && e.contains('2'), "{e}");

    let p_la = d.join("la.bin");
    std::fs::write(&p_la, b"khong-phai-wasm").expect("ghi");
    let o = chay(&["plugin", "check", p_la.to_str().expect("utf8")]);
    assert_eq!(o.status.code(), Some(2), "{}", stderr(&o));
    assert!(stderr(&o).contains("wasm"), "{}", stderr(&o));
}
