// SPDX-License-Identifier: MIT OR Apache-2.0
//! **Host hệ thống tệp** (ST-33, VD-266) — `vis-interp`/`vis-vm` **không** biết `std::fs`;
//! `vis-cli` cài bằng `std::fs`. Tách khỏi `builtin.rs` để giữ luật **R1** (≤ 1.500 dòng/tệp).
//!
//! Thiếu host ⇒ `VS0604` "chưa hỗ trợ ở môi trường này" (cùng mẫu `Ngoai`, VD-60 (2)). Mọi helper
//! mới của VD-266 có **default** trả `Err("chưa hỗ trợ")` ⇒ host cũ/mock vẫn biên dịch.

/// Một mục thư mục **kèm metadata** (ST-42) — cho `ls`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MucFs {
    pub name: String,
    pub size: u64,
    pub modified_ns: i64,
    pub is_file: bool,
    /// ST-46: bit quyền kiểu POSIX (0 = chưa biết ⇒ in `-` trong `ls --long`).
    pub mode: u32,
}

/// **ST-137 (VD-141)**: đường dẫn **thật của chính handle** đã mở — để tầng quyền quyết định trên
/// **vật** đã mở thay vì trên **tên** (đóng cửa sổ TOCTOU của đường đọc).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuongDanHandle {
    /// Host **không có** không gian tên thật (`--fs-mock`) **hoặc** nền **không có** primitive
    /// fd→path trong std an toàn (macOS: không có `/proc`). Tầng quyền giữ bước xác nhận của ST-124.
    KhongCo,
    /// Đường dẫn thật của handle (Linux: `/proc/self/fd/<fd>`), kernel đã giải symlink.
    Co(std::path::PathBuf),
    /// Host **thật** nhưng **không lấy được** đường dẫn của handle (`/proc` lỗi/không mount)
    /// ⇒ khi bật quyền phải **từ chối** (`VS0604`), **không** quay về cách cũ.
    KhongLayDuoc(String),
}

/// **ST-137 (VD-141)**: kết quả **một lần mở** — nội dung **đọc từ handle** + danh tính + đường dẫn
/// thật của chính handle đó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocQuaHandle {
    pub noi_dung: String,
    /// `(dev, ino)` của handle; `(0, 0)` = host không báo (mock).
    pub danh_tinh: (u64, u64),
    pub duong_dan: DuongDanHandle,
}

/// **VD-266**: tách dòng dùng chung cho `std.fs.read_lines` — cùng ngữ nghĩa `lines()`:
/// tách theo `\n`, bỏ `\r` ở **cuối mỗi dòng** (CRLF), và bỏ phần tử rỗng **cuối** do newline
/// kết thúc tệp. Dòng rỗng ở giữa vẫn giữ.
pub fn chia_dong(s: &str) -> Vec<String> {
    let mut ds: Vec<String> = s
        .split('\n')
        .map(|x| x.strip_suffix('\r').unwrap_or(x).to_string())
        .collect();
    if ds.last().is_some_and(|x| x.is_empty()) {
        ds.pop();
    }
    ds
}

pub trait FsHost {
    /// Đọc cả tệp thành chuỗi (UTF-8).
    fn read(&mut self, p: &str) -> Result<String, String>;
    /// Ghi đè cả tệp.
    fn write(&mut self, p: &str, s: &str) -> Result<(), String>;
    /// Tệp/thư mục có tồn tại.
    fn exists(&mut self, p: &str) -> bool;

    /// **ST-83 (VD-109)**: như [`exists`](Self::exists) nhưng **có thể báo vượt quyền** (chuỗi bắt
    /// đầu bằng [`DAU_QUYEN`](crate::stdlib::DAU_QUYEN)) ⇒ `VS0604`. Default: gọi `exists` (host cũ
    /// không đổi chữ ký).
    fn exists_quyen(&mut self, p: &str) -> Result<bool, String> {
        Ok(self.exists(p))
    }
    /// **ST-124 (VD-130b)**: như [`read`](Self::read) nhưng trả **kèm danh tính tệp ĐÃ MỞ**
    /// `(dev, ino)` để tầng quyền (`vis-cli::quyen::FsGuard`) xác nhận **mở-rồi-kiểm** chống TOCTOU
    /// kiểm-rồi-mở. `(0, 0)` = host **không báo** danh tính (mock, hoặc nền không có inode) ⇒ tầng
    /// quyền giữ hành vi cũ. Default: gọi `read` (impl cũ không đổi chữ ký).
    fn read_danh_tinh(&mut self, p: &str) -> Result<(String, u64, u64), String> {
        Ok((self.read(p)?, 0, 0))
    }
    /// **ST-137 (VD-141)**: như [`read_danh_tinh`](Self::read_danh_tinh) nhưng trả kèm **đường dẫn
    /// thật của chính handle đã mở** (Linux: `/proc/self/fd/<fd>`) + nội dung **đọc từ handle**.
    /// Tầng quyền quyết định trên **vật đã mở**, không trên tên ⇒ đóng cửa sổ TOCTOU.
    /// Default: `read_danh_tinh` + [`DuongDanHandle::KhongCo`] (host cũ không đổi chữ ký).
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        let (noi_dung, dev, ino) = self.read_danh_tinh(p)?;
        Ok(DocQuaHandle {
            noi_dung,
            danh_tinh: (dev, ino),
            duong_dan: DuongDanHandle::KhongCo,
        })
    }
    /// Tên các mục trong thư mục, **đã sắp xếp**, **không** đệ quy.
    fn list(&mut self, p: &str) -> Result<Vec<String>, String>;

    /// ST-42: mục thư mục **kèm metadata** (`ls`). Default: **chưa hỗ trợ** ⇒ impl cũ vẫn biên dịch.
    fn list_entries(&mut self, _p: &str) -> Result<Vec<MucFs>, String> {
        Err("môi trường này không cung cấp metadata thư mục".to_string())
    }

    /// **VD-266**: đọc tệp thành **danh sách dòng** — tách `\n`, bỏ `\r` cuối dòng, không sinh
    /// dòng rỗng cuối (xem [`chia_dong`]). Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn read_lines(&mut self, _p: &str) -> Result<Vec<String>, String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-266**: thêm vào **cuối** tệp (tạo nếu thiếu), **không** cắt nội dung cũ.
    /// Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn append(&mut self, _p: &str, _s: &str) -> Result<(), String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-266**: tạo **một** thư mục (như `mkdir`, **không** phải `mkdir -p`); lỗi nếu đã tồn tại
    /// hoặc thiếu thư mục cha. Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn mkdir(&mut self, _p: &str) -> Result<(), String> {
        Err("chưa hỗ trợ".to_string())
    }

    /// **VD-266**: xoá **một tệp** (không đệ quy); **từ chối** thư mục.
    /// Default: **chưa hỗ trợ** ⇒ host cũ/mock vẫn biên dịch.
    fn remove(&mut self, _p: &str) -> Result<(), String> {
        Err("chưa hỗ trợ".to_string())
    }
}
