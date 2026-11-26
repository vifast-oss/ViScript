// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Trạng thái nạp** — overlay buffer LSP + cache nội dung tệp phụ (ST-100, VS3-7, ST-317c).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use super::duong_dan::chuan_hoa;

/// **ST-100**: cache **nội dung tệp phụ** theo `(đường dẫn, mtime, size)` — chỉ dùng cho LSP; tệp chính
/// lấy từ buffer. `mtime` đổi ⇒ đọc lại; `didClose` ⇒ [`xoa_cache_phu`].
struct MucTepPhu {
    mtime: Option<SystemTime>,
    size: u64,
    text: String,
}

/// **ST-317c (#81/#66)** — trạng thái nạp **tiêm được**: overlay buffer tệp phụ + cache nội dung.
///
/// `Default` = rỗng (không overlay, cache rỗng). CLI dùng trạng thái mặc định trong `thread_local!`
/// (một phiên/process, giữ nguyên chữ ký `load_full*` cũ); LSP giữ **một** `LoadState` trong
/// `AnalysisHost` nên hai phiên/test chạy song song không rò trạng thái của nhau.
#[derive(Default)]
pub struct LoadState {
    cache_phu: HashMap<std::path::PathBuf, MucTepPhu>,
    /// **VS3-7 (VD-271)**: buffer LSP đang mở theo đường dẫn **canonical** — dùng làm nội dung tệp
    /// phụ khi phân tích module của tài liệu khác (kể cả chưa ghi đĩa). CLI không bật overlay.
    phu_mo: HashMap<std::path::PathBuf, String>,
}

thread_local! {
    /// Trạng thái mặc định cho CLI/API cũ (`load_full*` không đổi chữ ký).
    static TRANG_THAI: RefCell<LoadState> = RefCell::new(LoadState::default());
}

impl LoadState {
    /// **VS3-7**: `didOpen`/`didChange` của LSP — đăng ký nội dung buffer cho một tệp (canonical
    /// hoá đường dẫn). Lần đọc tệp phụ sau đó dùng buffer thay cho đĩa.
    pub fn dat_noi_dung_mo(&mut self, path: &Path, text: &str) {
        let p = chuan_hoa(path);
        self.phu_mo.insert(p, text.to_string());
    }

    /// **VS3-7**: bỏ overlay buffer của một tệp (LSP `didClose`).
    pub fn xoa_noi_dung_mo(&mut self, path: &Path) {
        let p = chuan_hoa(path);
        // Xoá cả khoá thô (tệp mới chưa có trên đĩa lúc `didOpen`) lẫn canonical.
        self.phu_mo.remove(path);
        self.phu_mo.remove(&p);
    }

    /// **ST-100**: dọn cache tệp phụ (LSP `didClose`/mọi thay đổi văn bản).
    pub fn xoa_cache_phu(&mut self) {
        self.cache_phu.clear();
    }

    pub(super) fn doc_tep_phu(&mut self, path: &Path) -> std::io::Result<String> {
        // **VS3-7**: buffer LSP đang mở thắng đĩa (kể cả tệp chưa tồn tại trên đĩa).
        // **VS3-W4 (Windows)**: `dat_noi_dung_mo` ghi theo `chuan_hoa(path)` (canonical), nên tra
        // thêm khoá canonical — nếu chỉ tra đường dẫn thô, Windows (`\\?\`, hoa/thường) trượt
        // overlay và đọc đĩa ⇒ buffer LSP bị bỏ qua. Tra thô trước để tệp mới (chưa có trên đĩa)
        // vẫn khớp.
        if let Some(t) = self.phu_mo.get(path) {
            return Ok(t.clone());
        }
        let khoa = chuan_hoa(path);
        if khoa != path {
            if let Some(t) = self.phu_mo.get(&khoa) {
                return Ok(t.clone());
            }
        }
        let meta = std::fs::metadata(path)?;
        let mtime = meta.modified().ok();
        let size = meta.len();
        if let Some(m) = self.cache_phu.get(path) {
            if m.mtime == mtime && m.size == size {
                return Ok(m.text.clone());
            }
        }
        let text = std::fs::read_to_string(path)?;
        self.cache_phu.insert(
            path.to_path_buf(),
            MucTepPhu {
                mtime,
                size,
                text: text.clone(),
            },
        );
        Ok(text)
    }
}

/// **VS3-7**: `didOpen`/`didChange` của LSP trên trạng thái mặc định (API cũ).
pub fn dat_noi_dung_mo(path: &Path, text: &str) {
    TRANG_THAI.with(|c| c.borrow_mut().dat_noi_dung_mo(path, text));
}

/// **VS3-7**: bỏ overlay buffer của một tệp trên trạng thái mặc định (API cũ).
pub fn xoa_noi_dung_mo(path: &Path) {
    TRANG_THAI.with(|c| c.borrow_mut().xoa_noi_dung_mo(path));
}

/// **ST-100**: dọn cache tệp phụ trên trạng thái mặc định (API cũ).
pub fn xoa_cache_phu() {
    TRANG_THAI.with(|c| c.borrow_mut().xoa_cache_phu());
}

#[cfg(test)]
fn doc_cache_phu(path: &std::path::Path) -> std::io::Result<String> {
    TRANG_THAI.with(|c| c.borrow_mut().doc_tep_phu(path))
}

/// Chạy `f` với trạng thái mặc định của CLI/API cũ (`thread_local!`).
pub(super) fn voi_trang_thai<R>(f: impl FnOnce(&mut LoadState) -> R) -> R {
    TRANG_THAI.with(|c| f(&mut c.borrow_mut()))
}

#[cfg(test)]
mod tests_overlay {
    use super::{dat_noi_dung_mo, doc_cache_phu, xoa_cache_phu, xoa_noi_dung_mo};

    /// **VS3-7**: buffer đang mở thắng đĩa; `didClose` trả lại nội dung đĩa; tệp **chưa có** trên
    /// đĩa vẫn đọc được từ overlay (module phụ mới mở).
    #[test]
    fn buffer_mo_thang_dia_va_close_tra_lai_dia() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let n = DEM.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("vis-phu-mo-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join("b.vis");
        std::fs::write(&p, "dia\n").unwrap();
        assert_eq!(doc_cache_phu(&p).unwrap(), "dia\n");

        dat_noi_dung_mo(&p, "buffer\n");
        assert_eq!(doc_cache_phu(&p).unwrap(), "buffer\n");
        // Đổi buffer lần nữa cũng thắng cache đĩa.
        dat_noi_dung_mo(&p, "buffer 2\n");
        assert_eq!(doc_cache_phu(&p).unwrap(), "buffer 2\n");

        xoa_noi_dung_mo(&p);
        xoa_cache_phu();
        assert_eq!(doc_cache_phu(&p).unwrap(), "dia\n");

        // Tệp mới chưa ghi đĩa: overlay vẫn đọc được; đóng ⇒ lỗi đọc như cũ.
        let moi = d.join("moi.vis");
        dat_noi_dung_mo(&moi, "moi\n");
        assert_eq!(doc_cache_phu(&moi).unwrap(), "moi\n");
        xoa_noi_dung_mo(&moi);
        assert!(doc_cache_phu(&moi).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
