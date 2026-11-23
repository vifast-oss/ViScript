// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-286 (R0c)** — [`super::FsGuard`]: bọc `FsHost` để **kiểm `cap[...]` trước** mọi thao tác
//! (`mở-rồi-kiểm` chống TOCTOU, ST-124/ST-137/ST-174). Tách khỏi `quyen.rs` để giữ luật R1
//! (≤ 1500 dòng/tệp); **0 đổi hành vi** — chỉ di chuyển mã nguyên văn.

#[cfg(all(test, unix))]
use vis_interp::builtin::{DocQuaHandle, DuongDanHandle};
use vis_interp::builtin::{FsHost, MucFs};
#[cfg(unix)]
use vis_interp::stdlib::DAU_QUYEN;

use super::Quyen;

#[cfg(all(test, unix))]
use super::vet;
#[cfg(all(test, unix))]
use crate::quyen_chuan::chuan;

/// **ST-163**: mô tả **trạng thái quan sát được ngay sau** một lần đọc **thành công** — để phân
/// biệt "nội dung tệp ngoài root lọt ra" với "đọc nhầm một tệp trong root". Chỉ dùng cho chẩn đoán
/// (`cfg(test)`), không ảnh hưởng quyết định quyền.
#[cfg(all(test, unix))]
fn mo_ta_lan_doc(p: &str, d: &DocQuaHandle) -> String {
    use std::os::unix::fs::MetadataExt;
    let duong_dan = match &d.duong_dan {
        DuongDanHandle::Co(dd) => format!("Co({})", dd.display()),
        DuongDanHandle::KhongLayDuoc(e) => format!("KhongLayDuoc({e})"),
        DuongDanHandle::KhongCo => "KhongCo".to_string(),
    };
    let symlink = match std::fs::symlink_metadata(p) {
        Ok(m) => format!(
            "symlink={},({},{})",
            m.file_type().is_symlink(),
            m.dev(),
            m.ino()
        ),
        Err(e) => format!("symlink=LỖI({e})"),
    };
    let theo_ten = match std::fs::metadata(p) {
        Ok(m) => format!("({},{})", m.dev(), m.ino()),
        Err(e) => format!("LỖI({e})"),
    };
    format!(
        "doc(dai={}, danh_tinh_handle=({},{}), duong_dan={duong_dan}) sau_doc: symlink_metadata={symlink}, metadata={theo_ten}, chuan(p)={}",
        d.noi_dung.len(),
        d.danh_tinh.0,
        d.danh_tinh.1,
        chuan(std::path::Path::new(p)).display()
    )
}

/// Bọc `FsHost` để **kiểm quyền trước** mọi thao tác.
pub struct FsGuard<'a> {
    pub trong: &'a mut dyn FsHost,
    pub quyen: &'a Quyen,
}

impl FsGuard<'_> {
    /// **ST-295 (VD-276 §1)** — với host hệ tệp thật trên Unix, trả `(fd gốc, đường tương đối)`
    /// để mở bằng `openat` **thay vì** kiểm-rồi-mở theo tên. `Ok(None)` khi host không hỗ trợ
    /// (mock/test) hoặc chưa bật `cap` ⇒ caller giữ đường cũ nguyên vẹn.
    #[cfg(unix)]
    fn goc_an_toan(
        &self,
        p: &str,
        can_write: bool,
    ) -> Result<Option<(std::path::PathBuf, std::path::PathBuf)>, String> {
        if !self.trong.ho_tro_mo_duoi_goc() {
            return Ok(None);
        }
        crate::quyen_mo::goc_va_tuong_doi(self.quyen, p, can_write)
    }

    /// Lỗi `openat`: symlink/thoát gốc là **vi phạm quyền** (`VS0604`); còn lại giữ câu chữ host cũ.
    #[cfg(unix)]
    fn loi_mo(p: &str, e: std::io::Error, ghi: bool) -> String {
        // **ST-300m1**: dùng errno **của nền** (Darwin `ELOOP = 62`, không phải 40).
        let ma = e.raw_os_error();
        if ma == Some(rustix::io::Errno::LOOP.raw_os_error())
            || ma == Some(rustix::io::Errno::XDEV.raw_os_error())
            || ma == Some(rustix::io::Errno::NOTDIR.raw_os_error())
        {
            return format!(
                "{DAU_QUYEN}[quyền] `{p}` chứa symlink hoặc thoát khỏi gốc `cap` (ST-295 · chống kiểm-rồi-mở)"
            );
        }
        if ghi {
            format!("không ghi được `{p}`: {e}")
        } else {
            format!("không đọc được `{p}`: {e}")
        }
    }
}

impl FsHost for FsGuard<'_> {
    /// **ST-124 (VD-130b) → ST-137 (VD-141)**: **mở-rồi-kiểm**, nhưng quyết định quyền trên
    /// **đường dẫn thật của handle** (khi host báo được — Linux `/proc/self/fd`), không trên tên.
    ///
    /// 1. Kiểm **trước** (giữ nguyên thông điệp lỗi cũ, và **không mở** tệp đã biết là ngoài quyền).
    /// 2. Mở tệp **một lần** qua host, lấy `(nội dung đọc TỪ handle, (dev,ino), đường dẫn thật)`.
    /// 3. Xác nhận **sau khi mở** trên đường dẫn của **chính handle** ⇒ tên bị đổi giữa các bước
    ///    không thể làm nội dung trả về khác vật mà quyết định quyền đã áp. Host **thật** mà
    ///    không lấy được đường dẫn handle ⇒ **từ chối** (`VS0604`).
    fn read(&mut self, p: &str) -> Result<String, String> {
        #[cfg(all(test, unix))]
        vet::bat_dau();
        // **ST-295 (VD-276 §1)**: host thật + `cap` ⇒ mở **từ fd gốc** (openat/O_NOFOLLOW);
        // không còn "canonicalize rồi mở theo tên" cho đường đọc có quyền.
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, false)? {
            return crate::quyen_mo::doc(&goc, &rel).map_err(|e| Self::loi_mo(p, e, false));
        }
        let da_kiem = self.quyen.kiem_fs_canon(p, false)?;
        let d = self.trong.read_theo_handle(p)?;
        if let Some(truoc) = da_kiem {
            self.quyen.xac_nhan_handle(p, &truoc, &d)?;
        }
        #[cfg(all(test, unix))]
        if vet::dang_bat() {
            vet::them(mo_ta_lan_doc(p, &d));
        }
        Ok(d.noi_dung)
    }
    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        // **ST-295 (VD-276 §1)**: `openat` từ fd gốc, `O_NOFOLLOW`/`openat2` — đóng khe
        // kiểm-rồi-mở của đường ghi (`#109b`/`#140`).
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, true)? {
            #[cfg(all(test, unix))]
            crate::quyen_st295::chen::chay(std::path::Path::new(p));
            return crate::quyen_mo::ghi_de(&goc, &rel, s.as_bytes())
                .map_err(|e| Self::loi_mo(p, e, true));
        }
        self.quyen.kiem_fs(p, true)?;
        #[cfg(all(test, unix))]
        crate::quyen_st295::chen::chay(std::path::Path::new(p));
        self.trong.write(p, s)
    }
    /// **VD-266**: đọc dòng **qua** [`read`](Self::read) ⇒ giữ nguyên **mở-rồi-kiểm** (chống TOCTOU)
    /// của đường đọc; tách dòng bằng cùng helper với host thật/mock.
    fn read_lines(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.read(p).map(|s| vis_interp::builtin::chia_dong(&s))
    }
    /// **VD-266**: thao tác **ghi** ⇒ cùng kiểm quyền như [`write`](Self::write) (`(write)` trên root).
    fn append(&mut self, p: &str, s: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, true)? {
            return crate::quyen_mo::ghi_them(&goc, &rel, s.as_bytes())
                .map_err(|e| Self::loi_mo(p, e, true));
        }
        self.quyen.kiem_fs(p, true)?;
        self.trong.append(p, s)
    }
    /// **VD-266**: tạo thư mục là thao tác **ghi** ⇒ cùng kiểm quyền như `write`.
    fn mkdir(&mut self, p: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, true)? {
            return crate::quyen_mo::tao_thu_muc(&goc, &rel)
                .map_err(|e| format!("không tạo được thư mục `{p}`: {e}"));
        }
        self.quyen.kiem_fs(p, true)?;
        self.trong.mkdir(p)
    }
    /// **VD-270 (mkdir-all)**: tạo **đệ quy** là thao tác **ghi** ⇒ `(write)` trên root; kiểm trên
    /// **tổ tiên gần nhất đã tồn tại** (đuôi có thể thiếu nhiều cấp) trước khi chạm đĩa.
    fn mkdir_all(&mut self, p: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, true)? {
            return crate::quyen_mo::tao_thu_muc_de_quy(&goc, &rel)
                .map_err(|e| format!("không tạo được thư mục `{p}`: {e}"));
        }
        self.quyen.kiem_fs_de_quy(p, true)?;
        self.trong.mkdir_all(p)
    }
    /// **VD-266**: xoá tệp là thao tác **ghi** ⇒ cùng kiểm quyền như `write`.
    fn remove(&mut self, p: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, true)? {
            return crate::quyen_mo::xoa_tep(&goc, &rel).map_err(|e| {
                if e.raw_os_error() == Some(rustix::io::Errno::ISDIR.raw_os_error()) {
                    format!("không xoá được `{p}`: là thư mục (chỉ xoá tệp)")
                } else {
                    format!("không xoá được `{p}`: {e}")
                }
            });
        }
        self.quyen.kiem_fs(p, true)?;
        self.trong.remove(p)
    }
    /// **VD-269**: `copy` cần **đọc** nguồn + **ghi** đích — kiểm **cả hai** trước khi chạm đĩa.
    /// Nguồn kiểm `can_write=false`, đích kiểm `true` (đích chưa tồn tại ⇒ canonicalize thư mục cha).
    fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let (Some((ga, ra)), Some((gb, rb))) =
            (self.goc_an_toan(from, false)?, self.goc_an_toan(to, true)?)
        {
            return crate::quyen_mo::chep(&ga, &ra, &gb, &rb)
                .map_err(|e| format!("không chép được `{from}` → `{to}`: {e}"));
        }
        self.quyen.kiem_fs(from, false)?;
        self.quyen.kiem_fs(to, true)?;
        self.trong.copy(from, to)
    }
    /// **VD-269**: `rename` cần **ghi** ở **cả hai** đầu (nguồn bị xoá khỏi tên cũ, đích được tạo).
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let (Some((ga, ra)), Some((gb, rb))) =
            (self.goc_an_toan(from, true)?, self.goc_an_toan(to, true)?)
        {
            return crate::quyen_mo::doi_ten(&ga, &ra, &gb, &rb)
                .map_err(|e| format!("không đổi tên được `{from}` → `{to}`: {e}"));
        }
        self.quyen.kiem_fs(from, true)?;
        self.quyen.kiem_fs(to, true)?;
        self.trong.rename(from, to)
    }
    /// **VD-270**: metadata **chỉ đọc** ⇒ kiểm `cap[Dir]`/`cap[File]` trên path như `exists`
    /// (`can_write=false`), không đòi `(write)`. Host dùng `symlink_metadata` (không đi theo
    /// symlink) cho **thao tác**; bước kiểm quyền vẫn canonicalize như mọi đường đọc khác.
    fn size(&mut self, p: &str) -> Result<i64, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.size(p)
    }
    /// **VD-270**: predicate **chỉ đọc** ⇒ cùng kiểm quyền như [`size`](Self::size).
    fn is_file(&mut self, p: &str) -> Result<bool, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.is_file(p)
    }
    /// **VD-270**: predicate **chỉ đọc** ⇒ cùng kiểm quyền như [`size`](Self::size).
    fn is_dir(&mut self, p: &str) -> Result<bool, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.is_dir(p)
    }
    /// **VD-270 (fs-is-symlink)**: predicate **chỉ đọc** ⇒ cùng kiểm quyền như
    /// [`size`](Self::size); bước kiểm quyền vẫn canonicalize như mọi đường đọc (symlink trỏ
    /// ngoài root ⇒ `VS0604`), còn **thao tác** dùng `symlink_metadata` (không đi theo link).
    fn is_symlink(&mut self, p: &str) -> Result<bool, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.is_symlink(p)
    }
    /// **VD-270 (fs-read-link)**: thao tác **chỉ đọc** ⇒ cùng kiểm quyền như [`size`](Self::size);
    /// bước kiểm canonicalize như mọi đường đọc (symlink tồn tại trỏ ngoài root ⇒ `VS0604`), còn
    /// **thao tác** dùng `read_link` (không đi theo link) nên đọc được cả symlink dangling.
    fn read_link(&mut self, p: &str) -> Result<String, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.read_link(p)
    }
    /// **VD-270 (fs-canonicalize)**: thao tác **chỉ đọc** ⇒ cùng kiểm quyền như [`size`](Self::size)
    /// (`cap[Dir]`/`cap[File]`, **không** cần `(write)`). Kiểm quyền canonicalize **trước** khi
    /// chạm đĩa nên symlink trỏ **ra ngoài** root ⇒ `VS0604`; bước kiểm dùng helper `chuan` của
    /// `quyen_chuan` (**không** gọi lại `canonicalize` của host) nên **không** đệ quy. Host thật
    /// canonicalize lần nữa để trả kết quả; đua giữa hai bước là cùng cửa sổ kiểm-rồi-dùng như mọi
    /// đường đọc khác.
    fn canonicalize(&mut self, p: &str) -> Result<String, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.canonicalize(p)
    }
    /// **VD-270 (fs-current-dir)**: **không** có đường dẫn người dùng ⇒ **không** kiểm `cap`;
    /// chỉ chuyển tiếp host (đọc môi trường). Hàm không-path duy nhất phải chuyển tiếp ở đây vì
    /// `FsGuard` luôn bọc host thật trong `vis run`; **không** gọi `kiem_fs`/`kiem_fs_canon`.
    fn current_dir(&mut self) -> Result<String, String> {
        self.trong.current_dir()
    }
    /// **VD-270 (fsbytes)**: đọc byte thô là thao tác **đọc** ⇒ kiểm `cap[Dir]`/`cap[File]`
    /// (`can_write=false`) trước khi chạm đĩa; lỗi host giữ nguyên `Err` dữ liệu.
    fn read_bytes(&mut self, p: &str) -> Result<Vec<u8>, String> {
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, false)? {
            return crate::quyen_mo::doc_byte(&goc, &rel).map_err(|e| Self::loi_mo(p, e, false));
        }
        self.quyen.kiem_fs(p, false)?;
        self.trong.read_bytes(p)
    }
    /// **VD-270 (fsbytes)**: ghi byte thô là thao tác **ghi** ⇒ cùng kiểm quyền như
    /// [`write`](Self::write) (`(write)` trên root).
    fn write_bytes(&mut self, p: &str, data: &[u8]) -> Result<(), String> {
        #[cfg(unix)]
        if let Some((goc, rel)) = self.goc_an_toan(p, true)? {
            return crate::quyen_mo::ghi_de(&goc, &rel, data).map_err(|e| Self::loi_mo(p, e, true));
        }
        self.quyen.kiem_fs(p, true)?;
        self.trong.write_bytes(p, data)
    }
    /// **VD-268**: duyệt là thao tác **đọc** ⇒ kiểm `cap[Dir]` trên **root** trước khi chạm đĩa.
    /// Host chỉ đi xuống thư mục con thật và **bỏ mọi symlink**, nên mọi đường dẫn trả về nằm dưới
    /// root đã kiểm (không thể thoát qua symlink); kiểm root là đủ, giống `list`/`list_entries`.
    fn walk(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.walk(p)
    }
    /// **ST-83**: ngoài quyền ⇒ **trap `VS0604`** (không còn `false` im lặng).
    fn exists(&mut self, p: &str) -> bool {
        self.exists_quyen(p).unwrap_or(false)
    }
    fn exists_quyen(&mut self, p: &str) -> Result<bool, String> {
        self.quyen.kiem_fs(p, false)?;
        Ok(self.trong.exists(p))
    }
    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.list(p)
    }
    fn list_entries(&mut self, p: &str) -> Result<Vec<MucFs>, String> {
        self.quyen.kiem_fs(p, false)?;
        self.trong.list_entries(p)
    }
}
