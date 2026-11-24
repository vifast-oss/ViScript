// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — `HostFs` của `vis run`: hệ tệp thật hoặc `--fs-mock` tất định.
//!
//! Tách **nguyên văn** khỏi `cli/run_cmd.rs` để giữ mỗi tệp ≤ 1.150 dòng.

use super::*;

/// **FsHost** của CLI (ST-42): `--fs-mock` ⇒ mock tất định, ngược lại ⇒ hệ tệp thật.
pub(super) enum HostFs {
    That(ngoai::Runner),
    Mock(fsmock::FsMock),
}

impl vis_core::host::FsHost for HostFs {
    /// **ST-295 (VD-276)**: chỉ host **hệ tệp thật** mới mở được bằng `openat` từ fd gốc; mock
    /// không có không gian tên ⇒ giữ đường cũ (và mọi test mock vẫn tất định).
    #[cfg(unix)]
    fn ho_tro_mo_duoi_goc(&self) -> bool {
        matches!(self, HostFs::That(_))
    }
    fn read(&mut self, p: &str) -> Result<String, String> {
        match self {
            HostFs::That(r) => r.read(p),
            HostFs::Mock(m) => m.read(p),
        }
    }
    /// **ST-124 (VD-130b)**: chỉ host **hệ tệp thật** báo được danh tính tệp đã mở; mock giữ
    /// `(0, 0)` (default của trait) ⇒ tầng quyền giữ hành vi cũ.
    fn read_danh_tinh(&mut self, p: &str) -> Result<(String, u64, u64), String> {
        match self {
            HostFs::That(r) => r.read_danh_tinh(p),
            HostFs::Mock(m) => m.read_danh_tinh(p),
        }
    }
    /// **ST-137 (VD-141)**: host **thật** báo được đường dẫn thật của handle (Linux `/proc`);
    /// mock không có không gian tên ⇒ default của trait (`KhongCo`) ⇒ tầng quyền giữ bước ST-124.
    fn read_theo_handle(&mut self, p: &str) -> Result<vis_core::host::DocQuaHandle, String> {
        match self {
            HostFs::That(r) => r.read_theo_handle(p),
            HostFs::Mock(m) => m.read_theo_handle(p),
        }
    }
    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.write(p, s),
            HostFs::Mock(m) => m.write(p, s),
        }
    }
    /// **VD-266**: bốn helper mới — host thật (OS) hoặc mock, cùng chữ ký.
    fn read_lines(&mut self, p: &str) -> Result<Vec<String>, String> {
        match self {
            HostFs::That(r) => r.read_lines(p),
            HostFs::Mock(m) => m.read_lines(p),
        }
    }
    fn append(&mut self, p: &str, s: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.append(p, s),
            HostFs::Mock(m) => m.append(p, s),
        }
    }
    fn mkdir(&mut self, p: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.mkdir(p),
            HostFs::Mock(m) => m.mkdir(p),
        }
    }
    /// **VD-270 (mkdir-all)**: host thật (OS) hoặc mock, cùng chữ ký.
    fn mkdir_all(&mut self, p: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.mkdir_all(p),
            HostFs::Mock(m) => m.mkdir_all(p),
        }
    }
    fn remove(&mut self, p: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.remove(p),
            HostFs::Mock(m) => m.remove(p),
        }
    }
    /// **VD-269**: chép/đổi tên tệp — host thật (OS) hoặc mock, cùng chữ ký.
    fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.copy(from, to),
            HostFs::Mock(m) => m.copy(from, to),
        }
    }
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.rename(from, to),
            HostFs::Mock(m) => m.rename(from, to),
        }
    }
    /// **VD-270**: metadata chỉ đọc — host thật (OS) hoặc mock, cùng chữ ký.
    fn size(&mut self, p: &str) -> Result<i64, String> {
        match self {
            HostFs::That(r) => r.size(p),
            HostFs::Mock(m) => m.size(p),
        }
    }
    fn is_file(&mut self, p: &str) -> Result<bool, String> {
        match self {
            HostFs::That(r) => r.is_file(p),
            HostFs::Mock(m) => m.is_file(p),
        }
    }
    fn is_dir(&mut self, p: &str) -> Result<bool, String> {
        match self {
            HostFs::That(r) => r.is_dir(p),
            HostFs::Mock(m) => m.is_dir(p),
        }
    }
    fn is_symlink(&mut self, p: &str) -> Result<bool, String> {
        match self {
            HostFs::That(r) => r.is_symlink(p),
            HostFs::Mock(m) => m.is_symlink(p),
        }
    }
    /// **VD-270 (fs-read-link)**: đọc đích symlink — host thật (OS) hoặc mock, cùng chữ ký.
    fn read_link(&mut self, p: &str) -> Result<String, String> {
        match self {
            HostFs::That(r) => r.read_link(p),
            HostFs::Mock(m) => m.read_link(p),
        }
    }
    /// **VD-270 (fs-canonicalize)**: đường dẫn tuyệt đối đã phân giải — host thật (OS) hoặc mock,
    /// cùng chữ ký.
    fn canonicalize(&mut self, p: &str) -> Result<String, String> {
        match self {
            HostFs::That(r) => r.canonicalize(p),
            HostFs::Mock(m) => m.canonicalize(p),
        }
    }
    /// **VD-270 (fs-current-dir)**: thư mục làm việc hiện tại — host thật (môi trường OS) hoặc
    /// mock (luôn `Err` tất định); cùng chữ ký.
    fn current_dir(&mut self) -> Result<String, String> {
        match self {
            HostFs::That(r) => r.current_dir(),
            HostFs::Mock(m) => m.current_dir(),
        }
    }
    /// **VD-270 (fsbytes)**: byte thô — host thật (OS) hoặc mock, cùng chữ ký.
    fn read_bytes(&mut self, p: &str) -> Result<Vec<u8>, String> {
        match self {
            HostFs::That(r) => r.read_bytes(p),
            HostFs::Mock(m) => m.read_bytes(p),
        }
    }
    fn write_bytes(&mut self, p: &str, data: &[u8]) -> Result<(), String> {
        match self {
            HostFs::That(r) => r.write_bytes(p, data),
            HostFs::Mock(m) => m.write_bytes(p, data),
        }
    }
    /// **VD-268**: host thật (OS) hoặc mock, cùng chữ ký.
    fn walk(&mut self, p: &str) -> Result<Vec<String>, String> {
        match self {
            HostFs::That(r) => r.walk(p),
            HostFs::Mock(m) => m.walk(p),
        }
    }
    fn exists(&mut self, p: &str) -> bool {
        match self {
            HostFs::That(r) => r.exists(p),
            HostFs::Mock(m) => m.exists(p),
        }
    }
    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        match self {
            HostFs::That(r) => r.list(p),
            HostFs::Mock(m) => m.list(p),
        }
    }
    fn list_entries(&mut self, p: &str) -> Result<Vec<vis_core::host::MucFs>, String> {
        match self {
            HostFs::That(r) => r.list_entries(p),
            HostFs::Mock(m) => m.list_entries(p),
        }
    }
}

impl HostFs {
    pub(super) fn tao(giai_thich: bool, mock: Option<&fsmock::FsMock>) -> Self {
        match mock {
            Some(m) => HostFs::Mock(m.clone()),
            None => HostFs::That(ngoai::Runner {
                giai_thich,
                ..ngoai::Runner::default()
            }),
        }
    }
}
