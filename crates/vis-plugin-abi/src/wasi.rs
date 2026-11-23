// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307 / VD-278, VD-281, VD-283** — nạp **plugin động WASI/WASM** có kiểm soát quyền.
//!
//! # Hợp đồng
//!
//! - Plugin là **WASI command** (`wasm32-wasip1`, export `_start`) và export marker
//!   `vis_plugin_abi_v1`. Host kiểm marker **trên byte wasm** trước khi chạy, rồi chạy chế độ
//!   `abi` để đối chiếu `VIS_PLUGIN_ABI_V1` — lệch ABI/không phải plugin ⇒ `VS0501` rõ ràng.
//! - **Sandbox tự nhiên**: host chỉ preopen đúng các `cap[Dir]` được trao, tại guest `/cap0`,
//!   `/cap1`… Plugin không có quyền ngầm; đường ngoài preopen bị WASI từ chối (`VS0604`).
//! - **Tắt cứng** khi [`ChinhSach::strict_caps`] hoặc [`ChinhSach::sandbox`] bật ⇒ `VS0604`
//!   trước khi đọc tệp (không có đường vòng).
//! - Runtime WASI tự dò theo thứ tự `wasmtime` CLI → Python `wasmtime` → Node `node:wasi`
//!   (đúng chính sách VD-283); có thể ghim bằng `VIS_WASI_RUNTIME`.
//! - Crate **0 `unsafe`**; mọi lỗi là [`Error`] có mã, không panic trên dữ liệu người dùng.
//!
//! # Giao thức plugin
//!
//! Host chạy runtime với argv `[abi|manifest|call, …]`; plugin trả **stdout**:
//!
//! ```text
//! abi       → VIS_PLUGIN_ABI_V1=1
//! manifest  → module=… / abi=1 / func=ten;p1,p2;ret / cap=Dir;ro|rw / ok
//! call      → ok\n<giá trị mã hoá>   hoặc   err\n<VSxxxx>\n<hex thông điệp>
//! ```
//!
//! Giá trị dùng codec văn bản tự tả: `u:`, `b:1:`, `i:5:`, `f:1.5:`, `s:<n>:<hex>`,
//! `y:<n>:<hex>`, `l:<n>:` + phần tử, `m:<n>:` + cặp, `o:1:`/`o:0:`, `r:1:`/`r:0:<n>:<hex>`,
//! `c:<klen>:<hex>:<rlen>:<hex>:<0|1>:`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::{Cap, Error, PluginValue, Result, VIS_PLUGIN_ABI_V1};

/// Tiền tố export marker ABI trên byte wasm.
pub const ABI_EXPORT_PREFIX: &str = "vis_plugin_abi_v";

/// Dòng handshake mà plugin phải in ở chế độ `abi`.
pub const ABI_HANDSHAKE: &str = "VIS_PLUGIN_ABI_V1=";

/// Tiền tố guest path của các `cap[Dir]` được preopen (`/cap0`, `/cap1`, …).
pub const GUEST_CAP_PREFIX: &str = "/cap";

/// Trần kích thước tệp wasm (64 MiB) — chặn tệp lạ nuốt bộ nhớ.
pub const MAX_WASM_BYTES: usize = 64 * 1024 * 1024;

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

/// Chính sách quyền của phiên chạy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChinhSach {
    /// `--strict-caps`: plugin động **tắt cứng**.
    pub strict_caps: bool,
    /// `--sandbox`: plugin động **tắt cứng**.
    pub sandbox: bool,
}

impl ChinhSach {
    /// Chính sách mặc định (cho phép plugin động, vẫn chỉ trong `cap`).
    pub fn moi() -> Self {
        Self::default()
    }

    /// Dựng từ hai cờ CLI.
    pub fn tu_co(strict_caps: bool, sandbox: bool) -> Self {
        Self {
            strict_caps,
            sandbox,
        }
    }

    /// `true` nếu plugin động bị tắt.
    pub fn tat_plugin_dong(&self) -> bool {
        self.strict_caps || self.sandbox
    }

    /// Chặn sớm, **trước** mọi truy cập tệp.
    pub fn kiem_tra(&self) -> Result<()> {
        if self.strict_caps {
            return Err(Error::new(
                "VS0604",
                "plugin động bị tắt cứng khi bật `--strict-caps` (VD-278/VD-283)",
            ));
        }
        if self.sandbox {
            return Err(Error::new(
                "VS0604",
                "plugin động bị tắt cứng khi bật `--sandbox` (VD-278/VD-283)",
            ));
        }
        Ok(())
    }
}

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

/// Một hàm plugin do manifest khai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HamPlugin {
    pub name: String,
    pub params: Vec<String>,
    pub ret: String,
}

/// Manifest plugin đọc từ chế độ `manifest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub module: String,
    pub abi: u32,
    pub funcs: Vec<HamPlugin>,
    pub caps: Vec<(String, bool)>,
}

impl Manifest {
    /// Tìm hàm theo tên.
    pub fn tim(&self, ten: &str) -> Option<&HamPlugin> {
        self.funcs.iter().find(|f| f.name == ten)
    }
}

/// Plugin WASI đã nạp: manifest + cap + runtime.
#[derive(Debug)]
pub struct PluginWasi {
    path: PathBuf,
    manifest: Manifest,
    /// Cap **gốc** host đã trao (để hiển thị/kiểm toán).
    caps: Vec<Cap>,
    /// Đường preopen thật (cap `ro` là bản sao chỉ-đọc).
    preopens: Vec<PathBuf>,
    runtime: Runtime,
    /// Bản sao chỉ-đọc của các cap `write = false` (dọn khi drop).
    tam_ro: Vec<PathBuf>,
}

impl PluginWasi {
    /// Nạp plugin, kiểm ABI + chính sách quyền. **Không** chạy hàm nào ngoài handshake/manifest.
    pub fn nap(path: impl AsRef<Path>, caps: Vec<Cap>, chinh_sach: ChinhSach) -> Result<Self> {
        // Tắt cứng **trước** khi đọc tệp: chế độ hạn chế không có đường vòng.
        chinh_sach.kiem_tra()?;

        let path = path.as_ref().to_path_buf();
        let bytes = fs::read(&path)
            .map_err(|e| Error::io(format!("không đọc được plugin `{}`: {e}", path.display())))?;
        if bytes.len() > MAX_WASM_BYTES {
            return Err(Error::io(format!(
                "plugin `{}` quá lớn ({} byte > {} byte)",
                path.display(),
                bytes.len(),
                MAX_WASM_BYTES
            )));
        }
        kiem_wasm_abi(&bytes)?;

        let runtime = tim_runtime()?;
        let (caps, preopens, tam_ro) = chuan_hoa_caps(caps)?;
        // Nếu các bước sau lỗi, `giu` dọn bản sao chỉ-đọc; thành công thì chuyển cho `PluginWasi`.
        let mut giu = GiuTam(tam_ro);

        let abi = chay_mot(&runtime, &path, &preopens, "abi", &[])?;
        let dong = abi.lines().next().unwrap_or("").trim();
        if dong != format!("{ABI_HANDSHAKE}{VIS_PLUGIN_ABI_V1}") {
            return Err(Error::new(
                "VS0501",
                format!(
                    "handshake plugin sai: cần `{ABI_HANDSHAKE}{VIS_PLUGIN_ABI_V1}`, nhận `{dong}`"
                ),
            ));
        }

        let ra = chay_mot(&runtime, &path, &preopens, "manifest", &[])?;
        let manifest = doc_manifest(&ra)?;
        if manifest.abi != VIS_PLUGIN_ABI_V1 {
            return Err(Error::new(
                "VS0501",
                format!(
                    "ABI plugin `{}` không khớp `VIS_PLUGIN_ABI_V1` ({VIS_PLUGIN_ABI_V1})",
                    manifest.abi
                ),
            ));
        }

        Ok(Self {
            path,
            manifest,
            caps,
            preopens,
            runtime,
            tam_ro: std::mem::take(&mut giu.0),
        })
    }

    /// Đường dẫn tệp wasm.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Manifest đã kiểm.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// Cap host đã trao (đã canonicalize).
    pub fn caps(&self) -> &[Cap] {
        &self.caps
    }

    /// Runtime đang dùng.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Guest path của cap thứ `i` (`/cap0`, `/cap1`, …).
    pub fn guest_cap(i: usize) -> String {
        format!("{GUEST_CAP_PREFIX}{i}")
    }

    /// Gọi một hàm plugin qua biên ABI.
    pub fn goi(&self, ten: &str, args: &[PluginValue]) -> Result<PluginValue> {
        let ham = self.manifest.tim(ten).ok_or_else(|| {
            Error::new(
                "VS0501",
                format!(
                    "plugin `{}` không có hàm `{ten}` (có: {})",
                    self.manifest.module,
                    self.manifest
                        .funcs
                        .iter()
                        .map(|f| f.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })?;
        if ham.params.len() != args.len() {
            return Err(Error::new(
                "VS0501",
                format!(
                    "hàm `{ten}` cần {} tham số, nhận {}",
                    ham.params.len(),
                    args.len()
                ),
            ));
        }
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(ten.to_string());
        argv.extend(args.iter().map(ma_hoa));
        let ra = self.chay("call", &argv)?;
        doc_ket_qua(&ra)
    }

    fn chay(&self, mode: &str, args: &[String]) -> Result<String> {
        chay_mot(&self.runtime, &self.path, &self.preopens, mode, args)
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

/// Chuẩn hoá cap và dựng **bản sao chỉ-đọc** cho cap `write = false`.
///
/// WASI Preview 1 (wasmtime CLI/Node `node:wasi`) không có cờ "preopen chỉ đọc"; nếu preopen
/// thẳng thư mục gốc thì runtime cấp quyền ghi. Để giữ đúng `cap[...]`, cap chỉ đọc được preopen
/// qua **bản sao tạm** rồi hạ quyền filesystem (`r-x`/`r--`): plugin đọc được, ghi bị OS từ chối,
/// dữ liệu gốc không đổi. Cap `write = true` preopen thẳng thư mục gốc.
fn chuan_hoa_caps(caps: Vec<Cap>) -> Result<(Vec<Cap>, Vec<PathBuf>, Vec<PathBuf>)> {
    let mut ra = Vec::with_capacity(caps.len());
    let mut preopens = Vec::with_capacity(caps.len());
    let mut tam = Vec::new();
    for (i, c) in caps.into_iter().enumerate() {
        if c.kind != "Dir" {
            don_tam(&mut tam);
            return Err(Error::new(
                "VS0604",
                format!(
                    "plugin WASI chỉ nhận `cap[Dir]` ở bản này (nhận `cap[{}]`)",
                    c.kind
                ),
            ));
        }
        let root = match fs::canonicalize(&c.root) {
            Ok(r) => r,
            Err(e) => {
                don_tam(&mut tam);
                return Err(Error::io(format!(
                    "không canonicalize được cap `{}`: {e}",
                    c.root.display()
                )));
            }
        };
        if !root.is_dir() {
            don_tam(&mut tam);
            return Err(Error::new(
                "VS0604",
                format!("cap `{}` không phải thư mục", c.root.display()),
            ));
        }
        if c.write {
            preopens.push(root.clone());
            ra.push(Cap {
                kind: c.kind,
                root,
                write: true,
            });
            continue;
        }
        let dich = std::env::temp_dir().join(format!(
            "vis-plugin-ro-{}-{i}-{}.d",
            std::process::id(),
            DEM.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dich);
        let mut dem = DemRo::default();
        if let Err(e) = sao_chep_ro(&root, &dich, &mut dem) {
            let _ = fs::remove_dir_all(&dich);
            don_tam(&mut tam);
            return Err(e);
        }
        if let Err(e) = ha_quyen_ro(&dich) {
            let _ = fs::remove_dir_all(&dich);
            don_tam(&mut tam);
            return Err(e);
        }
        preopens.push(dich.clone());
        tam.push(dich);
        ra.push(Cap {
            kind: c.kind,
            root,
            write: false,
        });
    }
    Ok((ra, preopens, tam))
}

impl Drop for PluginWasi {
    fn drop(&mut self) {
        don_tam(&mut self.tam_ro);
    }
}

/// Giữ bản sao chỉ-đọc trong lúc `nap` chạy; lỗi giữa chừng vẫn dọn sạch.
struct GiuTam(Vec<PathBuf>);

impl Drop for GiuTam {
    fn drop(&mut self) {
        don_tam(&mut self.0);
    }
}

fn don_tam(ds: &mut Vec<PathBuf>) {
    for d in ds.drain(..) {
        let _ = khoi_phuc_quyen(&d);
        let _ = fs::remove_dir_all(&d);
    }
}

#[derive(Default)]
struct DemRo {
    files: usize,
    bytes: u64,
}

const MAX_RO_FILES: usize = 4096;
const MAX_RO_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RO_SAU: usize = 32;

fn sao_chep_ro(goc: &Path, dich: &Path, dem: &mut DemRo) -> Result<()> {
    fs::create_dir_all(dich).map_err(|e| {
        Error::io(format!(
            "không tạo được bản sao chỉ đọc `{}`: {e}",
            dich.display()
        ))
    })?;
    sao_chep_vao(goc, dich, dem, 0)
}

fn sao_chep_vao(goc: &Path, dich: &Path, dem: &mut DemRo, sau: usize) -> Result<()> {
    if sau > MAX_RO_SAU {
        return Err(Error::new(
            "VS0604",
            format!("cap chỉ đọc `{}` quá sâu (> {MAX_RO_SAU})", goc.display()),
        ));
    }
    let ds = fs::read_dir(goc).map_err(|e| {
        Error::io(format!(
            "không đọc được cap chỉ đọc `{}`: {e}",
            goc.display()
        ))
    })?;
    for muc in ds {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc cap chỉ đọc: {e}")))?;
        let loai = muc
            .file_type()
            .map_err(|e| Error::io(format!("lỗi kiểu tệp cap chỉ đọc: {e}")))?;
        if loai.is_symlink() {
            // Không đi theo symlink: bản sao không được phép trỏ ra ngoài root.
            continue;
        }
        let dich_con = dich.join(muc.file_name());
        if loai.is_dir() {
            fs::create_dir_all(&dich_con)
                .map_err(|e| Error::io(format!("không tạo được `{}`: {e}", dich_con.display())))?;
            sao_chep_vao(&muc.path(), &dich_con, dem, sau + 1)?;
        } else if loai.is_file() {
            let meta = muc
                .metadata()
                .map_err(|e| Error::io(format!("lỗi metadata cap chỉ đọc: {e}")))?;
            dem.files += 1;
            dem.bytes = dem.bytes.saturating_add(meta.len());
            if dem.files > MAX_RO_FILES || dem.bytes > MAX_RO_BYTES {
                return Err(Error::new(
                    "VS0604",
                    format!(
                        "cap chỉ đọc `{}` quá lớn để sao chép (>{MAX_RO_FILES} tệp hoặc >{MAX_RO_BYTES} byte)",
                        goc.display()
                    ),
                ));
            }
            fs::copy(muc.path(), &dich_con).map_err(|e| {
                Error::io(format!(
                    "không sao chép được `{}`: {e}",
                    muc.path().display()
                ))
            })?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn ha_quyen_ro(d: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        let loai = muc
            .file_type()
            .map_err(|e| Error::io(format!("lỗi kiểu tệp `{}`: {e}", p.display())))?;
        if loai.is_dir() {
            ha_quyen_ro(&p)?;
            fs::set_permissions(&p, fs::Permissions::from_mode(0o555))
                .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", p.display())))?;
        } else {
            fs::set_permissions(&p, fs::Permissions::from_mode(0o444))
                .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", p.display())))?;
        }
    }
    fs::set_permissions(d, fs::Permissions::from_mode(0o555))
        .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", d.display())))?;
    Ok(())
}

#[cfg(not(unix))]
fn ha_quyen_ro(d: &Path) -> Result<()> {
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        if muc.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            ha_quyen_ro(&p)?;
        }
        let mut q = fs::metadata(&p)
            .map_err(|e| Error::io(format!("lỗi metadata `{}`: {e}", p.display())))?
            .permissions();
        q.set_readonly(true);
        fs::set_permissions(&p, q)
            .map_err(|e| Error::io(format!("không hạ quyền `{}`: {e}", p.display())))?;
    }
    Ok(())
}

#[cfg(unix)]
fn khoi_phuc_quyen(d: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    if !d.exists() {
        return Ok(());
    }
    fs::set_permissions(d, fs::Permissions::from_mode(0o755))
        .map_err(|e| Error::io(format!("không mở quyền `{}`: {e}", d.display())))?;
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        if muc.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            khoi_phuc_quyen(&p)?;
        } else {
            let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o644));
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn khoi_phuc_quyen(d: &Path) -> Result<()> {
    if !d.exists() {
        return Ok(());
    }
    for muc in fs::read_dir(d).map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?
    {
        let muc = muc.map_err(|e| Error::io(format!("lỗi đọc `{}`: {e}", d.display())))?;
        let p = muc.path();
        if muc.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            khoi_phuc_quyen(&p)?;
        } else if let Ok(meta) = fs::metadata(&p) {
            let mut q = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            q.set_readonly(false);
            let _ = fs::set_permissions(&p, q);
        }
    }
    Ok(())
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

fn chay_mot(
    runtime: &Runtime,
    wasm: &Path,
    preopens: &[PathBuf],
    mode: &str,
    args: &[String],
) -> Result<String> {
    let preopens: Vec<(String, String)> = preopens
        .iter()
        .enumerate()
        .map(|(i, p)| (PluginWasi::guest_cap(i), p.to_string_lossy().into_owned()))
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

fn kiem_wasm_abi(wasm: &[u8]) -> Result<()> {
    let exports = doc_export(wasm)?;
    let mut abi: Option<u32> = None;
    let mut co_start = false;
    for (ten, kind) in exports {
        if ten == "_start" && kind == 0 {
            co_start = true;
        }
        if let Some(rest) = ten.strip_prefix(ABI_EXPORT_PREFIX) {
            if let Ok(n) = rest.parse::<u32>() {
                if abi.is_some() {
                    return Err(Error::new(
                        "VS0501",
                        format!("plugin có nhiều export `{ABI_EXPORT_PREFIX}*` — không rõ ABI"),
                    ));
                }
                abi = Some(n);
            }
        }
    }
    if !co_start {
        return Err(Error::new(
            "VS0501",
            "không phải plugin WASI: thiếu export `_start` (WASI command)",
        ));
    }
    let abi = abi.ok_or_else(|| {
        Error::new(
            "VS0501",
            format!("không phải plugin ViScript: thiếu export `{ABI_EXPORT_PREFIX}1`"),
        )
    })?;
    if abi != VIS_PLUGIN_ABI_V1 {
        return Err(Error::new(
            "VS0501",
            format!("ABI plugin v{abi} không khớp `VIS_PLUGIN_ABI_V1` ({VIS_PLUGIN_ABI_V1})"),
        ));
    }
    Ok(())
}

fn doc_export(wasm: &[u8]) -> Result<Vec<(String, u8)>> {
    if wasm.len() < 8 || &wasm[0..4] != b"\0asm" {
        return Err(Error::new(
            "VS0501",
            "không phải tệp wasm (thiếu magic `\\0asm`)",
        ));
    }
    let version = u32::from_le_bytes([wasm[4], wasm[5], wasm[6], wasm[7]]);
    if version != 1 {
        return Err(Error::new(
            "VS0501",
            format!("phiên bản wasm {version} không hỗ trợ (cần 1)"),
        ));
    }
    let mut i = 8usize;
    let mut ra = Vec::new();
    while i < wasm.len() {
        let id = wasm[i];
        i += 1;
        let (size, ni) = leb(wasm, i)?;
        i = ni;
        let end = i
            .checked_add(size)
            .ok_or_else(|| Error::new("VS0501", "wasm hỏng: kích thước section tràn"))?;
        if end > wasm.len() {
            return Err(Error::new("VS0501", "wasm hỏng: section vượt quá tệp"));
        }
        if id == 7 {
            let mut j = i;
            let (count, nj) = leb(wasm, j)?;
            j = nj;
            for _ in 0..count {
                let (nlen, nj2) = leb(wasm, j)?;
                j = nj2;
                let ten_end = j
                    .checked_add(nlen)
                    .ok_or_else(|| Error::new("VS0501", "wasm hỏng: tên export tràn"))?;
                if ten_end > end {
                    return Err(Error::new("VS0501", "wasm hỏng: tên export vượt section"));
                }
                let ten = std::str::from_utf8(&wasm[j..ten_end])
                    .map_err(|_| Error::new("VS0501", "wasm hỏng: tên export không UTF-8"))?
                    .to_string();
                j = ten_end;
                if j >= end {
                    return Err(Error::new("VS0501", "wasm hỏng: export thiếu kind"));
                }
                let kind = wasm[j];
                j += 1;
                let (_, nj3) = leb(wasm, j)?;
                j = nj3;
                ra.push((ten, kind));
            }
        }
        i = end;
    }
    Ok(ra)
}

fn leb(b: &[u8], mut i: usize) -> Result<(usize, usize)> {
    let mut ra = 0usize;
    let mut shift = 0u32;
    loop {
        let x = *b
            .get(i)
            .ok_or_else(|| Error::new("VS0501", "wasm hỏng: LEB128 cụt"))?;
        i += 1;
        ra |= ((x & 0x7f) as usize) << shift;
        if x & 0x80 == 0 {
            return Ok((ra, i));
        }
        shift += 7;
        if shift >= usize::BITS {
            return Err(Error::new("VS0501", "wasm hỏng: LEB128 quá dài"));
        }
    }
}

fn doc_manifest(ra: &str) -> Result<Manifest> {
    let mut module = None;
    let mut abi = None;
    let mut funcs = Vec::new();
    let mut caps = Vec::new();
    let mut ok = false;
    for dong in ra.lines().map(str::trim) {
        if dong.is_empty() {
            continue;
        }
        if dong == "ok" {
            ok = true;
            continue;
        }
        let (khoa, gia) = dong.split_once('=').ok_or_else(|| {
            Error::new(
                "VS0501",
                format!("manifest plugin sai: dòng `{dong}` không có `=`"),
            )
        })?;
        match khoa {
            "module" => module = Some(gia.to_string()),
            "abi" => {
                abi = Some(gia.parse::<u32>().map_err(|_| {
                    Error::new("VS0501", format!("manifest plugin sai abi `{gia}`"))
                })?)
            }
            "func" => {
                let mut phan = gia.split(';');
                let ten = phan.next().unwrap_or("").trim();
                let params = phan.next().unwrap_or("");
                let ret = phan.next().unwrap_or("unit").trim();
                if ten.is_empty() {
                    return Err(Error::new("VS0501", "manifest plugin: hàm không tên"));
                }
                funcs.push(HamPlugin {
                    name: ten.to_string(),
                    params: if params.trim().is_empty() {
                        Vec::new()
                    } else {
                        params.split(',').map(|s| s.trim().to_string()).collect()
                    },
                    ret: ret.to_string(),
                });
            }
            "cap" => {
                let (kind, quyen) = gia.split_once(';').unwrap_or((gia, "ro"));
                let write = match quyen.trim() {
                    "rw" | "write" => true,
                    "ro" | "read" => false,
                    khac => {
                        return Err(Error::new(
                            "VS0501",
                            format!("manifest plugin: quyền cap lạ `{khac}`"),
                        ))
                    }
                };
                caps.push((kind.trim().to_string(), write));
            }
            khac => {
                return Err(Error::new(
                    "VS0501",
                    format!("manifest plugin: khoá lạ `{khac}`"),
                ))
            }
        }
    }
    if !ok {
        return Err(Error::new("VS0501", "manifest plugin thiếu dòng kết `ok`"));
    }
    let module = module.ok_or_else(|| Error::new("VS0501", "manifest plugin thiếu `module=`"))?;
    let abi = abi.ok_or_else(|| Error::new("VS0501", "manifest plugin thiếu `abi=`"))?;
    Ok(Manifest {
        module,
        abi,
        funcs,
        caps,
    })
}

fn doc_ket_qua(ra: &str) -> Result<PluginValue> {
    let mut dong = ra.lines();
    match dong.next().map(str::trim) {
        Some("ok") => {
            let gia = dong
                .next()
                .ok_or_else(|| Error::new("VS0501", "plugin trả `ok` thiếu giá trị"))?;
            giai_ma(gia.trim())
        }
        Some("err") => {
            let code = dong.next().unwrap_or("VS0501").trim();
            let ma = match code {
                "VS0604" => "VS0604",
                _ => "VS0501",
            };
            let hex_msg = dong.next().unwrap_or("").trim();
            let msg = giai_hex(hex_msg)
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or_else(|| "plugin báo lỗi không rõ".to_string());
            Err(Error::new(ma, msg))
        }
        Some(khac) => Err(Error::new(
            "VS0501",
            format!("phản hồi plugin sai: `{khac}` (cần `ok`/`err`)"),
        )),
        None => Err(Error::new("VS0501", "plugin không trả gì")),
    }
}

/// Mã hoá một [`PluginValue`] sang codec văn bản của giao thức.
pub fn ma_hoa(v: &PluginValue) -> String {
    let mut s = String::new();
    ma_hoa_vao(v, &mut s);
    s
}

fn ma_hoa_vao(v: &PluginValue, s: &mut String) {
    match v {
        PluginValue::Unit => s.push_str("u:"),
        PluginValue::Bool(b) => s.push_str(if *b { "b:1:" } else { "b:0:" }),
        PluginValue::Int(i) => {
            s.push_str("i:");
            s.push_str(&i.to_string());
            s.push(':');
        }
        PluginValue::Float(f) => {
            s.push_str("f:");
            s.push_str(&f.to_string());
            s.push(':');
        }
        PluginValue::Str(x) => {
            s.push_str("s:");
            s.push_str(&x.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(x.as_bytes()));
        }
        PluginValue::Bytes(x) => {
            s.push_str("y:");
            s.push_str(&x.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(x));
        }
        PluginValue::List(xs) => {
            s.push_str("l:");
            s.push_str(&xs.len().to_string());
            s.push(':');
            for x in xs {
                ma_hoa_vao(x, s);
            }
        }
        PluginValue::Map(kv) => {
            s.push_str("m:");
            s.push_str(&kv.len().to_string());
            s.push(':');
            for (k, v) in kv {
                ma_hoa_vao(k, s);
                ma_hoa_vao(v, s);
            }
        }
        PluginValue::Option(None) => s.push_str("o:0:"),
        PluginValue::Option(Some(x)) => {
            s.push_str("o:1:");
            ma_hoa_vao(x, s);
        }
        PluginValue::Result(Ok(x)) => {
            s.push_str("r:1:");
            ma_hoa_vao(x, s);
        }
        PluginValue::Result(Err(e)) => {
            s.push_str("r:0:");
            s.push_str(&e.message.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(e.message.as_bytes()));
        }
        PluginValue::Cap { kind, root, write } => {
            s.push_str("c:");
            s.push_str(&kind.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(kind.as_bytes()));
            s.push_str(&root.len().to_string());
            s.push(':');
            s.push_str(&ma_hex(root.as_bytes()));
            s.push(if *write { '1' } else { '0' });
            s.push(':');
        }
    }
}

/// Giải mã codec văn bản; lỗi ⇒ `VS0501`, không panic.
pub fn giai_ma(s: &str) -> Result<PluginValue> {
    let mut d = Doc {
        b: s.as_bytes(),
        i: 0,
    };
    let v = d.gia_tri()?;
    if d.i != d.b.len() {
        return Err(Error::new(
            "VS0501",
            format!("giá trị plugin còn thừa `{}`", &s[d.i..]),
        ));
    }
    Ok(v)
}

struct Doc<'a> {
    b: &'a [u8],
    i: usize,
}

impl Doc<'_> {
    fn ky_tu(&mut self) -> Result<u8> {
        let x = *self
            .b
            .get(self.i)
            .ok_or_else(|| Error::new("VS0501", "giá trị plugin cụt"))?;
        self.i += 1;
        Ok(x)
    }

    fn cham(&mut self) -> Result<()> {
        if self.ky_tu()? != b':' {
            return Err(Error::new("VS0501", "giá trị plugin thiếu `:`"));
        }
        Ok(())
    }

    fn so_den_cham(&mut self) -> Result<usize> {
        let bat_dau = self.i;
        while self.i < self.b.len() && self.b[self.i] != b':' {
            self.i += 1;
        }
        let s = std::str::from_utf8(&self.b[bat_dau..self.i])
            .map_err(|_| Error::new("VS0501", "giá trị plugin không UTF-8"))?;
        self.cham()?;
        s.parse::<usize>()
            .map_err(|_| Error::new("VS0501", format!("giá trị plugin: số `{s}` sai")))
    }

    fn hex_n(&mut self, n: usize) -> Result<Vec<u8>> {
        let can = n
            .checked_mul(2)
            .ok_or_else(|| Error::new("VS0501", "giá trị plugin: độ dài tràn"))?;
        let end = self
            .i
            .checked_add(can)
            .ok_or_else(|| Error::new("VS0501", "giá trị plugin: độ dài tràn"))?;
        if end > self.b.len() {
            return Err(Error::new("VS0501", "giá trị plugin: hex cụt"));
        }
        let s = std::str::from_utf8(&self.b[self.i..end])
            .map_err(|_| Error::new("VS0501", "giá trị plugin: hex không UTF-8"))?;
        let ra = giai_hex(s).ok_or_else(|| Error::new("VS0501", "giá trị plugin: hex sai"))?;
        self.i = end;
        Ok(ra)
    }

    fn chuoi(&mut self) -> Result<String> {
        let n = self.so_den_cham()?;
        let b = self.hex_n(n)?;
        String::from_utf8(b).map_err(|_| Error::new("VS0501", "giá trị plugin: chuỗi không UTF-8"))
    }

    fn gia_tri(&mut self) -> Result<PluginValue> {
        let tag = self.ky_tu()?;
        self.cham()?;
        match tag {
            b'u' => Ok(PluginValue::Unit),
            b'b' => {
                let x = self.ky_tu()?;
                self.cham()?;
                match x {
                    b'1' => Ok(PluginValue::Bool(true)),
                    b'0' => Ok(PluginValue::Bool(false)),
                    _ => Err(Error::new("VS0501", "giá trị plugin: bool sai")),
                }
            }
            b'i' => {
                let bat_dau = self.i;
                while self.i < self.b.len() && self.b[self.i] != b':' {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[bat_dau..self.i])
                    .map_err(|_| Error::new("VS0501", "giá trị plugin: int không UTF-8"))?;
                self.cham()?;
                s.parse::<i64>()
                    .map(PluginValue::Int)
                    .map_err(|_| Error::new("VS0501", format!("giá trị plugin: int `{s}` sai")))
            }
            b'f' => {
                let bat_dau = self.i;
                while self.i < self.b.len() && self.b[self.i] != b':' {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[bat_dau..self.i])
                    .map_err(|_| Error::new("VS0501", "giá trị plugin: float không UTF-8"))?;
                self.cham()?;
                s.parse::<f64>()
                    .map(PluginValue::Float)
                    .map_err(|_| Error::new("VS0501", format!("giá trị plugin: float `{s}` sai")))
            }
            b's' => self.chuoi().map(PluginValue::Str),
            b'y' => {
                let n = self.so_den_cham()?;
                self.hex_n(n).map(PluginValue::Bytes)
            }
            b'l' => {
                let n = self.so_den_cham()?;
                let mut xs = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    xs.push(self.gia_tri()?);
                }
                Ok(PluginValue::List(xs))
            }
            b'm' => {
                let n = self.so_den_cham()?;
                let mut kv = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    let k = self.gia_tri()?;
                    let v = self.gia_tri()?;
                    kv.push((k, v));
                }
                Ok(PluginValue::Map(kv))
            }
            b'o' => {
                let x = self.ky_tu()?;
                self.cham()?;
                match x {
                    b'0' => Ok(PluginValue::Option(None)),
                    b'1' => Ok(PluginValue::Option(Some(Box::new(self.gia_tri()?)))),
                    _ => Err(Error::new("VS0501", "giá trị plugin: option sai")),
                }
            }
            b'r' => {
                let x = self.ky_tu()?;
                self.cham()?;
                match x {
                    b'1' => Ok(PluginValue::Result(Ok(Box::new(self.gia_tri()?)))),
                    b'0' => {
                        let n = self.so_den_cham()?;
                        let b = self.hex_n(n)?;
                        let msg = String::from_utf8(b)
                            .map_err(|_| Error::new("VS0501", "giá trị plugin: lỗi không UTF-8"))?;
                        Ok(PluginValue::Result(Err(Box::new(Error::io(msg)))))
                    }
                    _ => Err(Error::new("VS0501", "giá trị plugin: result sai")),
                }
            }
            b'c' => {
                let kind = self.chuoi()?;
                let root = self.chuoi()?;
                let w = self.ky_tu()?;
                self.cham()?;
                let write = match w {
                    b'1' => true,
                    b'0' => false,
                    _ => return Err(Error::new("VS0501", "giá trị plugin: cap sai")),
                };
                Ok(PluginValue::Cap { kind, root, write })
            }
            _ => Err(Error::new(
                "VS0501",
                format!("giá trị plugin: tag lạ `{}`", tag as char),
            )),
        }
    }
}

fn ma_hex(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 0x0f) as usize] as char);
    }
    s
}

fn giai_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let b = s.as_bytes();
    let mut ra = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        ra.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(ra)
}

#[cfg(test)]
mod tests_driver_rieng {
    #[test]
    fn moi_lan_goi_thu_muc_driver_rieng() {
        let a = super::thu_muc_driver().expect("driver a");
        let b = super::thu_muc_driver().expect("driver b");
        assert_ne!(a, b, "driver phải riêng mỗi lần gọi");
        assert!(a.join("driver.mjs").is_file());
        assert!(b.join("driver.mjs").is_file());
    }
}
