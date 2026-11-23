// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-307/ST-310c (VD-285)** — nạp **plugin động WASI/WASM** có kiểm soát quyền.
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
//! Codec và phân tích phản hồi ở [`protocol`]; dò/chạy runtime ở [`runner`]; chính sách cap ở
//! [`sandbox`]. Tách ba phần để mỗi tệp < 650 dòng (ST-310c) mà không đổi hành vi.

pub mod protocol;
pub mod runner;
pub mod sandbox;

use std::fs;
use std::path::{Path, PathBuf};

use vis_plugin_abi::{Cap, Error, PluginValue, Result, VIS_PLUGIN_ABI_V1};

pub use protocol::{giai_ma, ma_hoa};
pub use protocol::{HamPlugin, Manifest, ABI_EXPORT_PREFIX, ABI_HANDSHAKE, MAX_WASM_BYTES};
pub use runner::{tim_runtime, Runtime, HAN_MAC_DINH_MS};
pub use sandbox::ChinhSach;

/// Tiền tố guest path của các `cap[Dir]` được preopen (`/cap0`, `/cap1`, …).
pub const GUEST_CAP_PREFIX: &str = "/cap";

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
        protocol::kiem_wasm_abi(&bytes)?;

        let runtime = tim_runtime()?;
        let (caps, preopens, tam_ro) = sandbox::chuan_hoa_caps(caps)?;
        // Nếu các bước sau lỗi, `giu` dọn bản sao chỉ-đọc; thành công thì chuyển cho `PluginWasi`.
        let mut giu = sandbox::GiuTam(tam_ro);

        let abi = runner::chay_mot(&runtime, &path, &preopens, "abi", &[])?;
        let dong = abi.lines().next().unwrap_or("").trim();
        if dong != format!("{ABI_HANDSHAKE}{VIS_PLUGIN_ABI_V1}") {
            return Err(Error::new(
                "VS0501",
                format!(
                    "handshake plugin sai: cần `{ABI_HANDSHAKE}{VIS_PLUGIN_ABI_V1}`, nhận `{dong}`"
                ),
            ));
        }

        let ra = runner::chay_mot(&runtime, &path, &preopens, "manifest", &[])?;
        let manifest = protocol::doc_manifest(&ra)?;
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
        protocol::doc_ket_qua(&ra)
    }

    fn chay(&self, mode: &str, args: &[String]) -> Result<String> {
        runner::chay_mot(&self.runtime, &self.path, &self.preopens, mode, args)
    }
}

impl Drop for PluginWasi {
    fn drop(&mut self) {
        sandbox::don_tam(&mut self.tam_ro);
    }
}
