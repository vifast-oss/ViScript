// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307/ST-310c (VD-285)** — dò runtime WASI và chạy plugin trong tiến trình con.
//!
//! Thứ tự dò giữ nguyên VD-283: `wasmtime` CLI → Python `wasmtime` → Node `node:wasi`;
//! `VIS_WASI_RUNTIME` ghim lựa chọn. Mỗi lượt chạy dùng thư mục driver **riêng** (pid + số thứ
//! tự) để hai test song song không đọc driver đang bị truncate; stdout/stderr qua tệp tạm, có
//! hạn chờ và kill.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use vis_plugin_abi::{Error, Result};

use super::GUEST_CAP_PREFIX;

/// Hạn chờ mặc định cho một lượt chạy plugin (ms).
pub const HAN_MAC_DINH_MS: u64 = 20_000;

const DRIVER_MJS: &str = r#"import { WASI } from 'node:wasi';
import { readFile } from 'node:fs/promises';

const av = process.argv.slice(2);
const pre = {};
const args = [];
let wasmPath = '';
for (let i = 0; i < av.length; i++) {
  if (av[i] === '--') { args.push(...av.slice(i + 1)); break; }
  if (av[i] === '--preopen') {
    const spec = av[++i] || '';
    const k = spec.indexOf('::');
    if (k > 0) pre[spec.slice(0, k)] = spec.slice(k + 2);
    continue;
  }
  if (!wasmPath) { wasmPath = av[i]; continue; }
  args.push(av[i]);
}
const wasi = new WASI({
  version: 'preview1',
  args: ['vis-plugin', ...args],
  env: {},
  preopens: pre,
  returnOnExit: true,
});
const wasm = await WebAssembly.compile(await readFile(wasmPath));
const instance = await WebAssembly.instantiate(wasm, wasi.getImportObject());
process.exit(wasi.start(instance));
"#;

const DRIVER_PY: &str = r#"import sys
from wasmtime import Engine, Store, Module, Linker, WasiConfig

av = sys.argv[1:]
pre = []
args = []
wasm = ''
i = 0
while i < len(av):
    if av[i] == '--':
        args.extend(av[i + 1:])
        break
    if av[i] == '--preopen':
        i += 1
        spec = av[i] if i < len(av) else ''
        k = spec.find('::')
        if k > 0:
            pre.append((spec[:k], spec[k + 2:]))
    elif not wasm:
        wasm = av[i]
    else:
        args.append(av[i])
    i += 1

engine = Engine()
store = Store(engine)
wasi = WasiConfig()
wasi.argv = ['vis-plugin'] + args
wasi.env = []
for guest, host in pre:
    wasi.preopen_dir(host, guest)
wasi.inherit_stdout()
wasi.inherit_stderr()
store.set_wasi(wasi)
linker = Linker(engine)
linker.define_wasi()
instance = linker.instantiate(store, Module.from_file(engine, wasm))
try:
    instance.exports(store)['_start'](store)
except Exception as e:  # pragma: no cover - đường lỗi của runtime
    code = getattr(e, 'code', None)
    sys.exit(code if isinstance(code, int) else 1)
"#;

/// Runtime WASI đã dò được.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runtime {
    /// CLI `wasmtime`.
    Wasmtime(PathBuf),
    /// Python package `wasmtime`.
    Python(PathBuf),
    /// Node built-in `node:wasi`.
    Node(PathBuf),
}

impl Runtime {
    /// Tên ngắn để in/ghi log.
    pub fn ten(&self) -> &'static str {
        match self {
            Self::Wasmtime(_) => "wasmtime",
            Self::Python(_) => "python-wasmtime",
            Self::Node(_) => "node",
        }
    }
}

/// Dò runtime theo thứ tự `wasmtime` → Python `wasmtime` → Node.
pub fn tim_runtime() -> Result<Runtime> {
    if let Ok(chon) = std::env::var("VIS_WASI_RUNTIME") {
        let chon = chon.trim();
        if !chon.is_empty() {
            return chon_runtime(chon);
        }
    }
    if let Some(p) = tim_tren_path("wasmtime") {
        return Ok(Runtime::Wasmtime(p));
    }
    if let Some(py) = tim_python_wasmtime() {
        return Ok(Runtime::Python(py));
    }
    if let Some(p) = tim_tren_path("node") {
        return Ok(Runtime::Node(p));
    }
    Err(Error::new(
        "VS0604",
        "không có runtime WASI (cần `wasmtime`, Python `wasmtime` hoặc Node ≥ 20)",
    ))
}

fn chon_runtime(chon: &str) -> Result<Runtime> {
    let lower = chon.to_ascii_lowercase();
    if lower == "node" {
        return tim_tren_path("node")
            .map(Runtime::Node)
            .ok_or_else(|| Error::new("VS0604", "không thấy `node` trên PATH"));
    }
    if lower == "python" || lower == "python-wasmtime" {
        return tim_python_wasmtime()
            .map(Runtime::Python)
            .ok_or_else(|| Error::new("VS0604", "không thấy Python package `wasmtime`"));
    }
    if lower == "wasmtime" {
        return tim_tren_path("wasmtime")
            .map(Runtime::Wasmtime)
            .ok_or_else(|| Error::new("VS0604", "không thấy `wasmtime` trên PATH"));
    }
    let p = PathBuf::from(chon);
    if !p.is_file() {
        return Err(Error::new(
            "VS0604",
            format!("VIS_WASI_RUNTIME không phải tệp chạy được: `{chon}`"),
        ));
    }
    let ten = p
        .file_name()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if ten.contains("node") {
        Ok(Runtime::Node(p))
    } else if ten.contains("python") {
        Ok(Runtime::Python(p))
    } else {
        Ok(Runtime::Wasmtime(p))
    }
}

fn tim_tren_path(ten: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for d in std::env::split_paths(&path) {
        let p = d.join(ten);
        if p.is_file() {
            return Some(p);
        }
        #[cfg(windows)]
        {
            let exe = d.join(format!("{ten}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

fn tim_python_wasmtime() -> Option<PathBuf> {
    for ten in ["python3", "python"] {
        if let Some(py) = tim_tren_path(ten) {
            let ok = Command::new(&py)
                .args(["-c", "import wasmtime"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                return Some(py);
            }
        }
    }
    None
}

fn thu_muc_driver() -> Result<PathBuf> {
    // **ST-300m3/QA**: mỗi lần gọi một thư mục riêng. Trước đây dùng chung
    // `vis-plugin-wasi-driver` + ghi đè `driver.mjs`; hai test plugin chạy song song
    // có thể đọc driver đang bị truncate ⇒ stdout rỗng/VS0501. Tên kèm pid + số thứ tự.
    let dem = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-plugin-wasi-driver-{}-{}",
        std::process::id(),
        dem
    ));
    fs::create_dir_all(&d).map_err(|e| {
        Error::io(format!(
            "không tạo được thư mục driver `{}`: {e}",
            d.display()
        ))
    })?;
    for (ten, noi_dung) in [("driver.mjs", DRIVER_MJS), ("driver.py", DRIVER_PY)] {
        let p = d.join(ten);
        fs::write(&p, noi_dung)
            .map_err(|e| Error::io(format!("không ghi được driver `{}`: {e}", p.display())))?;
    }
    Ok(d)
}

/// Chạy một lượt `abi`/`manifest`/`call`; trả stdout, lỗi thoát/hạn chờ là `VS0501`.
pub(crate) fn chay_mot(
    runtime: &Runtime,
    wasm: &Path,
    preopens: &[PathBuf],
    mode: &str,
    args: &[String],
) -> Result<String> {
    let preopens: Vec<(String, String)> = preopens
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                format!("{GUEST_CAP_PREFIX}{i}"),
                p.to_string_lossy().into_owned(),
            )
        })
        .collect();

    let mut cmd = match runtime {
        Runtime::Wasmtime(p) => {
            let mut c = Command::new(p);
            c.arg("run");
            for (guest, host) in &preopens {
                c.arg("--dir").arg(format!("{guest}::{host}"));
            }
            c.arg(wasm).arg(mode);
            c
        }
        Runtime::Node(p) => {
            let d = thu_muc_driver()?;
            let mut c = Command::new(p);
            c.arg(d.join("driver.mjs")).arg(wasm);
            for (guest, host) in &preopens {
                c.arg("--preopen").arg(format!("{guest}::{host}"));
            }
            c.arg("--").arg(mode);
            c
        }
        Runtime::Python(p) => {
            let d = thu_muc_driver()?;
            let mut c = Command::new(p);
            c.arg(d.join("driver.py")).arg(wasm);
            for (guest, host) in &preopens {
                c.arg("--preopen").arg(format!("{guest}::{host}"));
            }
            c.arg("--").arg(mode);
            c
        }
    };
    for a in args {
        cmd.arg(a);
    }

    let dem = DEM.fetch_add(1, Ordering::Relaxed);
    let tam = std::env::temp_dir();
    let out = tam.join(format!("vis-plugin-{}-{dem}.out", std::process::id()));
    let err = tam.join(format!("vis-plugin-{}-{dem}.err", std::process::id()));
    let fout = fs::File::create(&out)
        .map_err(|e| Error::io(format!("không tạo được `{}`: {e}", out.display())))?;
    let ferr = fs::File::create(&err)
        .map_err(|e| Error::io(format!("không tạo được `{}`: {e}", err.display())))?;
    cmd.stdin(Stdio::null()).stdout(fout).stderr(ferr);

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::io(format!("không chạy được runtime {}: {e}", runtime.ten())))?;
    let han = Duration::from_millis(han_tu_moi_truong());
    let bat_dau = Instant::now();
    let trang_thai = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if bat_dau.elapsed() >= han => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&out);
                let _ = fs::remove_file(&err);
                return Err(Error::new(
                    "VS0501",
                    format!("plugin quá hạn {} ms (đã kill)", han.as_millis()),
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => {
                let _ = fs::remove_file(&out);
                let _ = fs::remove_file(&err);
                return Err(Error::io(format!("lỗi chờ plugin: {e}")));
            }
        }
    };

    let stdout = fs::read_to_string(&out).unwrap_or_default();
    let stderr = fs::read_to_string(&err).unwrap_or_default();
    let _ = fs::remove_file(&out);
    let _ = fs::remove_file(&err);
    if !trang_thai.success() {
        let mut thong_diep = stderr.trim().to_string();
        if thong_diep.len() > 800 {
            thong_diep.truncate(800);
            thong_diep.push('…');
        }
        return Err(Error::new(
            "VS0501",
            format!(
                "plugin thoát {} khi chạy `{mode}`{}",
                trang_thai.code().unwrap_or(-1),
                if thong_diep.is_empty() {
                    String::new()
                } else {
                    format!(": {thong_diep}")
                }
            ),
        ));
    }
    Ok(stdout)
}

fn han_tu_moi_truong() -> u64 {
    std::env::var("VIS_PLUGIN_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(HAN_MAC_DINH_MS)
}

static DEM: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
mod tests {
    #[test]
    fn moi_lan_goi_thu_muc_driver_rieng() {
        let a = super::thu_muc_driver().expect("driver a");
        let b = super::thu_muc_driver().expect("driver b");
        assert_ne!(a, b, "driver phải riêng mỗi lần gọi");
        assert!(a.join("driver.mjs").is_file());
        assert!(b.join("driver.mjs").is_file());
    }
}
