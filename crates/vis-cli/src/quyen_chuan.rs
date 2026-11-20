// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-is-symlink)** — helper canonicalize **thuần đường dẫn** của `quyen.rs`.
//!
//! Tách khỏi `quyen.rs` để giữ **R1** (≤ 1.500 dòng/tệp) khi thêm `is_symlink`; **không** đổi
//! hành vi: hai hàm dưới đây nguyên văn từ `quyen.rs` (chỉ đổi `Self::` thành hàm tự do).

use std::path::{Path, PathBuf};

/// **ST-124** — canonicalize `p`; nếu `p` **chưa tồn tại** (ghi) thì canonicalize thư mục cha rồi
/// ghép tên; không canonicalize được ⇒ giữ nguyên `p`.
pub(crate) fn chuan(p: &Path) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(p) {
        return c;
    }
    // Tệp **chưa tồn tại** (ghi): canonicalize thư mục cha rồi ghép tên.
    if let (Some(cha), Some(ten)) = (p.parent(), p.file_name()) {
        if let Ok(c) = std::fs::canonicalize(cha) {
            return c.join(ten);
        }
    }
    p.to_path_buf()
}

/// **VD-270 (mkdir-all)**: canonicalize **tổ tiên gần nhất đã tồn tại** rồi ghép phần đuôi
/// **từ vựng** (`.` bỏ, `..` lùi). Cần cho thao tác **đệ quy** có thể thiếu **nhiều** cấp:
/// [`chuan`] chỉ canonicalize được cha **trực tiếp**, nên `root/../ngoai/moi/con` (đuôi chưa
/// tồn tại) sẽ giữ nguyên `..` và lọt qua `Path::starts_with(root)` — một lỗ hổng quyền.
pub(crate) fn chuan_de_quy(p: &Path) -> PathBuf {
    let mut hien = p;
    loop {
        if let Ok(c) = std::fs::canonicalize(hien) {
            let mut ra = c;
            if let Ok(du) = p.strip_prefix(hien) {
                for x in du.components() {
                    match x {
                        std::path::Component::CurDir => {}
                        std::path::Component::ParentDir => {
                            ra.pop();
                        }
                        khac => ra.push(khac.as_os_str()),
                    }
                }
            }
            return ra;
        }
        match hien.parent() {
            Some(cha) => hien = cha,
            None => return p.to_path_buf(),
        }
    }
}
