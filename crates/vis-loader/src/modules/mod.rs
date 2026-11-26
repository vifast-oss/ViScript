// SPDX-License-Identifier: MIT OR Apache-2.0
//! Nạp **đa tệp** (ST-25, §7.10): giải `use`, cấm vòng, hợp nhất HIR, giữ `dòng:cột` theo **tệp thật**.
//!
//! Mô hình: mỗi tệp là **một module**; `use ./util` nhập item `pub` của `util.vis` cùng thư mục;
//! `use std.x` ⇒ `VS0701` ("std chưa có — ST-26"); `use pkg.<tên>[.<module>]` (ST-71) đọc gói đã cài
//! từ `.vis/pkg/` **theo `vis.lock`**, kiểm **sha** nội dung lúc nạp — lệch ⇒ `VS0701` "gói bị sửa".
//!
//! Thứ tự **tất định**: hậu duyệt DFS theo thứ tự `use` trong nguồn (không phụ thuộc `read_dir`), mỗi
//! tệp một lần, tệp chính **cuối**. Hậu duyệt ⇒ khi hạ một tệp, mọi kiểu `pub` của phụ thuộc đã nằm
//! trong bảng kiểu chung nên `RecId`/`EnumId` **ổn định** giữa các tệp.

mod duong_dan;
mod kieu;
mod nap;
mod state;

pub use duong_dan::vi_tri;
pub use kieu::{FileUnit, LoadError, Loaded};
pub use state::{dat_noi_dung_mo, xoa_cache_phu, xoa_noi_dung_mo, LoadState};

/// Nạp `main` + toàn bộ phụ thuộc (ST-25).
#[allow(dead_code)] // giữ API `load` (không cờ) cho test/đọc; sản phẩm dùng `load_with`.
pub fn load(main: &str) -> Result<Loaded, LoadError> {
    load_with(main, true)
}

/// Như [`load`] nhưng `cho_ngoai = false` ⇒ lệnh lạ báo `VS0601` (`vis check --no-extern`).
pub fn load_with(main: &str, cho_ngoai: bool) -> Result<Loaded, LoadError> {
    load_full(main, cho_ngoai, true)
}

/// Như [`load_with`] nhưng `kiem_kieu = false` ⇒ **chỉ cú pháp/hạ**, bỏ chẩn đoán `tyck`
/// (`vis check --syntax`, dùng cho cổng "khối mã trong tài liệu **parse** được" — QD §7.4).
pub fn load_full(main: &str, cho_ngoai: bool, kiem_kieu: bool) -> Result<Loaded, LoadError> {
    state::voi_trang_thai(|t| nap::load_full_voi(main, None, cho_ngoai, kiem_kieu, false, t))
}

/// **ST-80 (VD-108)**: như [`load_full`] nhưng **văn bản tệp chính** lấy từ `text` (buffer LSP)
/// thay vì đọc đĩa; tệp phụ vẫn đọc đĩa. **Không** đổi chữ ký `load_full`/`load_with`.
pub fn load_full_voi_text(
    main: &str,
    text: &str,
    cho_ngoai: bool,
    kiem_kieu: bool,
) -> Result<Loaded, LoadError> {
    state::voi_trang_thai(|t| nap::load_full_voi(main, Some(text), cho_ngoai, kiem_kieu, false, t))
}

/// **ST-317c (#81/#66)** — như [`load_full_voi_text`] nhưng dùng **trạng thái nạp tiêm được**
/// (overlay + cache) thay cho `thread_local!` toàn cục. LSP giữ một `LoadState` trong
/// `AnalysisHost` để hai phiên/test song song không rò trạng thái.
pub fn load_full_voi_text_trang_thai(
    main: &str,
    text: &str,
    cho_ngoai: bool,
    kiem_kieu: bool,
    trang_thai: &mut LoadState,
) -> Result<Loaded, LoadError> {
    nap::load_full_voi(main, Some(text), cho_ngoai, kiem_kieu, false, trang_thai)
}

/// **VD-269**: `vis check --stdin` — nạp **script mode** từ bộ đệm stdin dưới tên ảo `-`
/// (không đọc/ghi đĩa). Cùng đường [`load_full`] (lex → parse → hạ → tyck) nên luật chẩn
/// đoán/mức không đổi; tên `-` không có đuôi ⇒ `Mode::Script`. `use ./x` tương đối giải
/// theo **thư mục hiện hành** (như tệp chính nằm ở đó).
pub fn load_full_stdin(text: &str, cho_ngoai: bool, kiem_kieu: bool) -> Result<Loaded, LoadError> {
    state::voi_trang_thai(|t| nap::load_full_voi("-", Some(text), cho_ngoai, kiem_kieu, true, t))
}

/// In cây phụ thuộc (cho `vis run --dump-deps`), thứ tự hậu duyệt đã sắp.
pub fn dump_deps(l: &Loaded) -> String {
    let mut s = String::new();
    for (rel, con) in &l.deps {
        if con.is_empty() {
            s.push_str(&format!("{rel}\n"));
        } else {
            s.push_str(&format!("{rel} -> {}\n", con.join(", ")));
        }
    }
    s
}
