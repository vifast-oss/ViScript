// SPDX-License-Identifier: Apache-2.0
//! **ST-317c (#81/#66)** — `AnalysisHost`: trạng thái một phiên LSP, **tiêm được** thay cho
//! `thread_local!` toàn cục.
//!
//! Một `AnalysisHost` sở hữu:
//! - buffer tài liệu đang mở (`uri → text`) — thay `van_ban` cục bộ trong `run`;
//! - cache kết quả phân tích theo `(uri, text)` (ST-283);
//! - cache item module phụ (ST-100) — trước đây là `CACHE_MOD` thread-local trong `module.rs`;
//! - trạng thái nạp của `vis-loader` (overlay buffer + cache nội dung tệp phụ, VS3-7/ST-100) —
//!   trước đây là `CACHE_PHU`/`PHU_MO` thread-local;
//! - `rootUri` của workspace (ST-277) để sẵn cho đa workspace.
//!
//! Nhờ vậy hai phiên (hoặc hai test chạy song song trong cùng process) **không** rò overlay/cache
//! của nhau; `didClose` chỉ xoá trạng thái của host sở hữu. API công khai của binary/server
//! (`vis_lsp::run`) giữ nguyên.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use vis_loader::modules::LoadState;

use super::chan_doan;
use super::module::{self, MucModule};
use super::phan_tich::KetQua;

/// Thống kê cache của một host — dùng cho test cách ly và chẩn đoán `#81`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThongKeCache {
    /// Số tài liệu đang mở.
    pub tai_lieu: usize,
    /// Số mục cache kết quả phân tích.
    pub phan_tich: usize,
    /// Số mục cache module phụ.
    pub module: usize,
}

/// Trạng thái phân tích của **một** phiên LSP (một workspace).
#[derive(Default)]
pub struct AnalysisHost {
    van_ban: HashMap<String, String>,
    kq_cache: HashMap<String, (String, Rc<KetQua>)>,
    mod_cache: HashMap<PathBuf, MucModule>,
    load: LoadState,
    root: Option<PathBuf>,
}

impl AnalysisHost {
    /// Host rỗng (chưa mở tài liệu, chưa có `rootUri`).
    pub fn new() -> Self {
        Self::default()
    }

    /// `rootUri` đã giải thành đường dẫn (ST-277); `None` ⇒ không quét workspace.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// Đặt `rootUri` (đã giải mã) cho `workspace/symbol`.
    pub fn dat_root(&mut self, root: Option<PathBuf>) {
        self.root = root;
    }

    /// `didOpen`/`didChange` (full hoặc sau khi áp incremental): lưu buffer, đăng ký overlay module
    /// và xoá cache phân tích/module/tệp phụ của **host này**.
    pub fn mo(&mut self, uri: &str, text: &str) {
        self.van_ban.insert(uri.to_string(), text.to_string());
        self.dat_overlay(uri, text);
        self.xoa_cache();
    }

    /// `didClose`: bỏ buffer + overlay, xoá cache của host này; trả văn bản vừa đóng (nếu có).
    pub fn dong(&mut self, uri: &str) -> Option<String> {
        let cu = self.van_ban.remove(uri);
        self.xoa_overlay(uri);
        self.xoa_cache();
        cu
    }

    /// Văn bản buffer đang mở của `uri` (LSP đọc lại cho completion/hover/…).
    pub fn van_ban(&self, uri: &str) -> Option<&str> {
        self.van_ban.get(uri).map(String::as_str)
    }

    /// Số tài liệu đang mở.
    pub fn so_tai_lieu(&self) -> usize {
        self.van_ban.len()
    }

    /// Thống kê cache (test cách ly `#81`; không phải API giao thức).
    pub fn thong_ke_cache(&self) -> ThongKeCache {
        ThongKeCache {
            tai_lieu: self.van_ban.len(),
            phan_tich: self.kq_cache.len(),
            module: self.mod_cache.len(),
        }
    }

    /// Notification chẩn đoán cho tài liệu **đang mở**; `None` nếu chưa mở.
    pub fn chan_doan_tai_lieu(&mut self, uri: &str) -> Option<Vec<u8>> {
        let text = self.van_ban.get(uri)?.clone();
        Some(self.chan_doan_voi(uri, Some(&text)))
    }

    /// Notification `diagnostics:[]` khi đóng tài liệu.
    pub fn chan_doan_dong(&mut self, uri: &str) -> Vec<u8> {
        self.chan_doan_voi(uri, None)
    }

    fn chan_doan_voi(&mut self, uri: &str, text: Option<&str>) -> Vec<u8> {
        chan_doan::than(uri, text, super::mode_cua(uri), &mut self.load)
    }

    /// URI các tài liệu **đang mở** phụ thuộc tài liệu vừa đổi (VS3-7) — sắp theo URI.
    pub fn uri_lien_quan(&mut self, uri: &str) -> Vec<String> {
        chan_doan::uri_lien_quan(uri, &self.van_ban, &mut self.load)
    }

    /// Kết quả phân tích đã cache theo `(uri, text)` (ST-283) cho tài liệu đang mở.
    pub(crate) fn phan_tich(&mut self, uri: &str) -> Option<Rc<KetQua>> {
        let text = self.van_ban.get(uri)?.clone();
        if let Some((cu, kq)) = self.kq_cache.get(uri) {
            if cu == &text {
                return Some(kq.clone());
            }
        }
        let kq = Rc::new(module::phan_tich_uri(
            uri,
            &text,
            super::mode_cua(uri),
            &mut self.mod_cache,
            &mut self.load,
        ));
        self.kq_cache.insert(uri.to_string(), (text, kq.clone()));
        Some(kq)
    }

    /// Mọi thay đổi văn bản ⇒ xoá cache phân tích + module + tệp phụ của host này.
    fn xoa_cache(&mut self) {
        self.kq_cache.clear();
        self.mod_cache.clear();
        self.load.xoa_cache_phu();
    }

    /// Đăng ký buffer làm nội dung module phụ (thắng đĩa tới khi `didClose`).
    fn dat_overlay(&mut self, uri: &str, text: &str) {
        if let Some(p) = super::uri_thanh_duong(uri) {
            self.load.dat_noi_dung_mo(Path::new(&p), text);
        }
    }

    fn xoa_overlay(&mut self, uri: &str) {
        if let Some(p) = super::uri_thanh_duong(uri) {
            self.load.xoa_noi_dung_mo(Path::new(&p));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(nhan: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEM: AtomicUsize = AtomicUsize::new(0);
        let n = DEM.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("vis-host-{nhan}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        d
    }

    /// **#81/#66** — hai host dùng **cùng** đường dẫn `util.vis` nhưng overlay khác nhau: mỗi host
    /// chỉ thấy item của buffer mình. Trước ST-317c (`PHU_MO` thread-local) host B ghi đè host A
    /// trong cùng thread ⇒ test này đỏ.
    #[test]
    fn hai_host_khong_ro_overlay_va_cache() {
        let d = temp_dir("cach-ly");
        std::fs::write(d.join("util.vis"), "pub fn goc():\n    return 0\n").expect("ghi util");
        std::fs::write(
            d.join("main.vis"),
            "use ./util\n\nfn main():\n    print(1)\n",
        )
        .expect("ghi main");
        let main_uri = super::super::duong_thanh_uri(&d.join("main.vis"), false);
        let util_uri = super::super::duong_thanh_uri(&d.join("util.vis"), false);

        let mut a = AnalysisHost::new();
        a.mo(&util_uri, "pub fn cua_a():\n    return 1\n");
        a.mo(&main_uri, "use ./util\n\nfn main():\n    print(1)\n");
        let kq_a = a.phan_tich(&main_uri).expect("phân tích A");
        let nhan_a: Vec<&str> = kq_a.items.iter().map(|g| g.label.as_str()).collect();
        assert!(
            nhan_a.contains(&"cua_a"),
            "A thiếu item buffer mình: {nhan_a:?}"
        );
        assert!(!nhan_a.contains(&"cua_b"), "A thấy item của B: {nhan_a:?}");
        assert!(
            !nhan_a.contains(&"goc"),
            "A phải dùng overlay, không đọc đĩa"
        );

        let mut b = AnalysisHost::new();
        b.mo(&util_uri, "pub fn cua_b():\n    return 2\n");
        b.mo(&main_uri, "use ./util\n\nfn main():\n    print(1)\n");
        let kq_b = b.phan_tich(&main_uri).expect("phân tích B");
        let nhan_b: Vec<&str> = kq_b.items.iter().map(|g| g.label.as_str()).collect();
        assert!(
            nhan_b.contains(&"cua_b"),
            "B thiếu item buffer mình: {nhan_b:?}"
        );
        assert!(!nhan_b.contains(&"cua_a"), "B thấy item của A: {nhan_b:?}");

        assert_eq!(a.thong_ke_cache().module, 1, "cache module của A");
        assert_eq!(b.thong_ke_cache().module, 1, "cache module của B");
        // didClose ở A không được xoá cache của B.
        assert!(a.dong(&util_uri).is_some());
        assert_eq!(
            a.thong_ke_cache(),
            ThongKeCache {
                tai_lieu: 1,
                phan_tich: 0,
                module: 0
            }
        );
        assert_eq!(b.thong_ke_cache().module, 1, "B giữ cache riêng");
        assert!(b.van_ban(&util_uri).is_some(), "B vẫn mở util");

        let _ = std::fs::remove_dir_all(&d);
    }
}
