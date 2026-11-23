// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277/VD-282)** — host WASI thật cho ViScript trên `wasm32-wasip1`.
//!
//! # Phạm vi
//!
//! - **Hệ tệp**: chỉ trong **preopen** ([`Preopen`]) — đường dẫn tuyệt đối khớp theo thành phần,
//!   đường dẫn tương đối giải theo preopen đầu tiên; preopen `@ro` từ chối ghi; thoát gốc bằng
//!   `..`/symlink ⇒ lỗi `VS0604` rõ ràng ([`fs::WasiFs`]).
//! - **Đồng hồ**: wall clock + monotonic qua Rust std (WASI `clock_time_get`) — [`dong_ho`].
//! - **stdout/stderr**: ghi thật hoặc gom bộ đệm cho test — [`stdio::Stdio`].
//! - **Từ chối rõ**: `net`/`proc`/`sqlite` — [`tu_choi`] (`TuChoiNet`/`TuChoiProc`/`TuChoiDb`).
//!
//! Không `unsafe` (crate đặt `#![forbid(unsafe_code)]`), không panic trên dữ liệu người dùng,
//! không dependency ngoài crate lõi ViScript.
//!
//! # Bằng chứng
//!
//! `WasiFs` test được **native** (thư mục tạm) vì logic containment thuần `std::fs`; phần chạy
//! thật cần `wasmtime` — xem `bin/vis-wasm-suite` và `agent/archive/journal-pre-v040/st299a-wasi-core.md`.
//! Mức bằng chứng **không** vượt quá thứ chạy được: nếu thiếu `wasmtime`, bước chạy wasm ghi
//! `BLOCKED-ENV`, không tuyên bố đã chạy.

#![forbid(unsafe_code)]

pub mod dong_ho;
pub mod fs;
pub mod preopen;
pub mod runner;
pub mod stdio;
pub mod tu_choi;

#[cfg(target_arch = "wasm32")]
use std::path::PathBuf;

pub use fs::{Preopen, WasiFs};
pub use stdio::{Dich, Stdio};
pub use tu_choi::{TuChoiDb, TuChoiNet, TuChoiProc, LY_DO_MANG, LY_DO_PROC, LY_DO_SQLITE};

/// Câu mô tả **mức bảo đảm** của host (giữ API cũ của khung ST-74).
pub const MO_TA: &str =
    "wasm32-wasip1: FS preopen + clock + stdout/stderr; từ chối net/proc/sqlite; 0 mã nguy hiểm";

/// Phiên bản hợp đồng host (đổi khi thêm/bớt khả năng).
pub const PHIEN_BAN: &str = "ST-299a/1";

/// Probe tĩnh: trả câu mô tả mức bảo đảm (không truy cập OS).
pub fn probe() -> &'static str {
    MO_TA
}

/// Host WASI tổng: gom hệ tệp, stdio và ba host từ chối.
///
/// Ba host từ chối là **trường riêng** để [`WasiHost::tach`] cho mượn rời từng phần — cùng lúc
/// truyền được `&mut dyn FsHost`, `&mut dyn NetHost`, `&mut dyn Ngoai`, `&mut dyn DbHost` vào
/// runtime mà không cần `RefCell`/`unsafe`.
#[derive(Debug)]
pub struct WasiHost {
    pub fs: WasiFs,
    pub stdio: Stdio,
    pub net: TuChoiNet,
    pub proc: TuChoiProc,
    pub db: TuChoiDb,
}

/// Các mượn rời từ [`WasiHost`].
pub struct ThanhPhan<'a> {
    pub fs: &'a mut WasiFs,
    pub stdio: &'a mut Stdio,
    pub net: &'a mut TuChoiNet,
    pub proc: &'a mut TuChoiProc,
    pub db: &'a mut TuChoiDb,
}

impl WasiHost {
    /// Dựng host với preopen cho trước (xem [`WasiFs::moi`]).
    pub fn moi(preopens: &[Preopen]) -> Result<Self, String> {
        Ok(Self {
            fs: WasiFs::moi(preopens)?,
            stdio: Stdio::that(),
            net: TuChoiNet,
            proc: TuChoiProc,
            db: TuChoiDb,
        })
    }

    /// Dựng host theo môi trường:
    ///
    /// 1. `VIS_WASM_PREOPENS` — danh sách ngăn bởi `:`, mỗi mục `guest` hoặc `guest@ro`
    ///    (host = guest; trên wasm runtime đã ánh xạ).
    /// 2. Trên wasm: quét `argv` tìm mục `guest::host` do wasmtime chèn cho `--dir` (lấy `guest`).
    /// 3. Không có mục nào ⇒ host FS **đóng** (mọi thao tác `std.fs` bị từ chối rõ).
    pub fn tu_moi_truong() -> Result<Self, String> {
        let ds = preopen_moi_truong()?;
        Self::moi(&ds)
    }

    /// Mượn rời từng host để truyền vào interp/VM.
    pub fn tach(&mut self) -> ThanhPhan<'_> {
        ThanhPhan {
            fs: &mut self.fs,
            stdio: &mut self.stdio,
            net: &mut self.net,
            proc: &mut self.proc,
            db: &mut self.db,
        }
    }

    /// Ghi stdout qua [`Stdio`].
    pub fn ghi_out(&mut self, s: &str) -> Result<(), String> {
        self.stdio.ghi_out(s)
    }

    /// Ghi stderr qua [`Stdio`].
    pub fn ghi_err(&mut self, s: &str) -> Result<(), String> {
        self.stdio.ghi_err(s)
    }
}

/// Danh sách preopen theo môi trường (xem [`WasiHost::tu_moi_truong`]).
///
/// Tách khỏi [`WasiHost`] để binary lấy được **cùng** danh sách trước khi dựng host.
pub fn preopen_moi_truong() -> Result<Vec<Preopen>, String> {
    let mut ds: Vec<Preopen> = Vec::new();
    if let Ok(raw) = std::env::var("VIS_WASM_PREOPENS") {
        for entry in raw.split(':') {
            let e = entry.trim();
            if e.is_empty() {
                continue;
            }
            let p = Preopen::tu_chuoi(e)?;
            if !ds.iter().any(|x| x.khach == p.khach) {
                ds.push(p);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        for arg in std::env::args().skip(1) {
            if let Some((khach, _host)) = preopen_tu_argv(&arg) {
                if !ds.iter().any(|x| x.khach == khach) {
                    ds.push(Preopen {
                        khach: khach.to_string(),
                        that: PathBuf::from(khach),
                        ghi: true,
                    });
                }
            }
        }
    }
    Ok(ds)
}

/// Nhận diện mục `guest::host` do wasmtime chèn cho `--dir` — dùng để **bỏ qua** khi parse argv
/// chương trình và (trên wasm) làm preopen dự phòng.
pub fn preopen_tu_argv(arg: &str) -> Option<(&str, &str)> {
    let (khach, host) = arg.split_once("::")?;
    if khach.starts_with('/') && !host.is_empty() {
        Some((khach, host))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mo_ta_va_probe() {
        assert!(MO_TA.contains("wasm32-wasip1"));
        assert!(MO_TA.contains("0 mã nguy hiểm"));
        assert_eq!(probe(), MO_TA);
    }

    #[test]
    fn nhan_dien_preopen_argv() {
        assert_eq!(
            preopen_tu_argv("/data::/home/ps/data"),
            Some(("/data", "/home/ps/data"))
        );
        assert_eq!(preopen_tu_argv("--vm"), None);
        assert_eq!(preopen_tu_argv("a::b"), None);
        assert_eq!(preopen_tu_argv("/data::"), None);
    }

    #[test]
    fn preopen_tu_chuoi() {
        let p = Preopen::tu_chuoi("/data=/srv/data@ro").unwrap();
        assert_eq!(p.khach, "/data");
        assert_eq!(p.that, std::path::PathBuf::from("/srv/data"));
        assert!(!p.ghi);
        assert!(Preopen::tu_chuoi("data").is_err());
    }
}
