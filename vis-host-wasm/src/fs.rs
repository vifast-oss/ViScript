// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-299a (VD-277)** — [`WasiFs`]: `FsHost` chỉ được chạm **trong preopen**.
//!
//! # Mô hình
//!
//! Mỗi [`Preopen`] ánh xạ một đường dẫn **khách** tuyệt đối (`/data`) sang một thư mục **thật**
//! (`PathBuf`). Trên `wasm32-wasip1` hai bên trùng nhau (runtime WASI đã ánh xạ); trên native
//! test chúng khác nhau để kiểm containment bằng thư mục tạm.
//!
//! - Đường dẫn tuyệt đối phải khớp **theo thành phần** với một preopen; dài nhất thắng.
//! - Đường dẫn tương đối giải theo **preopen đầu tiên** (quy ước tất định; chương trình nhiều
//!   preopen nên dùng đường dẫn tuyệt đối).
//! - Preopen `@ro` từ chối mọi thao tác ghi.
//! - Thoát gốc bằng `..`, symlink trỏ ra ngoài, hoặc đường dẫn ngoài preopen ⇒ `Err` bắt đầu
//!   bằng [`DAU_QUYEN`] (`VS0604`), **không panic**.
//!
//! # Giới hạn bằng chứng
//!
//! Đây là guard **trong không gian người dùng** (kiểm theo đường dẫn + canonicalize), không phải
//! Landlock. Trên WASI, hàng rào cứng vẫn là preopen của runtime; guard này **chặt hơn** (thu hẹp
//! thêm), không thay thế kernel. Trên native không có cách ly tiến trình — test chỉ kiểm logic.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use vis_core::host::{chia_dong, FsHost, MucFs};
use vis_core::host::DAU_QUYEN;

use crate::preopen::{chuan_hoa_tuyet_doi, chuan_hoa_yeu_cau, hien, trong_goc};

/// Một thư mục được cấp cho chương trình.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preopen {
    /// Đường dẫn **khách** tuyệt đối (`/data`).
    pub khach: String,
    /// Thư mục thật trên host; trên wasm trùng `khach`.
    pub that: PathBuf,
    /// `true` ⇒ cho ghi; `false` ⇒ chỉ đọc.
    pub ghi: bool,
}

impl Preopen {
    /// Preopen đọc–ghi.
    pub fn moi(khach: impl Into<String>, that: impl Into<PathBuf>) -> Self {
        Self {
            khach: khach.into(),
            that: that.into(),
            ghi: true,
        }
    }

    /// Preopen chỉ đọc.
    pub fn chi_doc(khach: impl Into<String>, that: impl Into<PathBuf>) -> Self {
        Self {
            khach: khach.into(),
            that: that.into(),
            ghi: false,
        }
    }

    /// Phân tích cú pháp CLI: `guest=host`, `guest` (host=guest), hậu tố `@ro`/`@rw`.
    ///
    /// Ví dụ: `--preopen /data=/srv/data@ro`, `--preopen /work`.
    pub fn tu_chuoi(s: &str) -> Result<Self, String> {
        let (than, ghi) = match s.strip_suffix("@ro") {
            Some(x) => (x, false),
            None => (s.strip_suffix("@rw").unwrap_or(s), true),
        };
        if than.is_empty() {
            return Err("preopen rỗng".to_string());
        }
        let (khach, that) = match than.split_once('=') {
            Some((k, t)) => (k, t),
            None => (than, than),
        };
        chuan_hoa_tuyet_doi(khach)?;
        if that.is_empty() {
            return Err(format!("preopen `{s}` thiếu thư mục host"));
        }
        Ok(Self {
            khach: khach.to_string(),
            that: PathBuf::from(that),
            ghi,
        })
    }
}

#[derive(Debug, Clone)]
struct Goc {
    khach: Vec<String>,
    that: PathBuf,
    ghi: bool,
}

/// Tên + metadata của một mục trong thư mục.
type MucThuMuc = Vec<(String, fs::Metadata)>;

/// Host hệ tệp WASI: mọi thao tác đi qua preopen.
#[derive(Debug, Clone)]
pub struct WasiFs {
    goc: Vec<Goc>,
    /// Thư mục khách của preopen đầu tiên (dùng cho đường dẫn tương đối + `current_dir`).
    cwd: Option<String>,
}

impl WasiFs {
    /// Dựng host với danh sách preopen. Mỗi `that` phải **tồn tại** và là thư mục.
    ///
    /// Danh sách rỗng là hợp lệ: mọi thao tác FS bị từ chối rõ (`VS0604`), không panic.
    pub fn moi(preopens: &[Preopen]) -> Result<Self, String> {
        let mut goc: Vec<Goc> = Vec::with_capacity(preopens.len());
        let mut cwd = None;
        for (i, p) in preopens.iter().enumerate() {
            let khach = chuan_hoa_tuyet_doi(&p.khach)
                .map_err(|e| format!("preopen #{} `{}`: {e}", i + 1, p.khach))?;
            let that = fs::canonicalize(&p.that).map_err(|e| {
                format!(
                    "preopen `{}` không canonicalize được: {e}",
                    p.that.display()
                )
            })?;
            let md = fs::metadata(&that).map_err(|e| {
                format!(
                    "preopen `{}` không đọc được metadata: {e}",
                    p.that.display()
                )
            })?;
            if !md.is_dir() {
                return Err(format!("preopen `{}` không phải thư mục", p.that.display()));
            }
            if goc.iter().any(|g| g.khach == khach) {
                return Err(format!("preopen khách trùng: `{}`", p.khach));
            }
            if cwd.is_none() {
                cwd = Some(hien(&khach));
            }
            goc.push(Goc {
                khach,
                that,
                ghi: p.ghi,
            });
        }
        Ok(Self { goc, cwd })
    }

    /// Host không có preopen nào (mọi thao tác FS bị từ chối).
    pub fn rong() -> Self {
        Self {
            goc: Vec::new(),
            cwd: None,
        }
    }

    /// Danh sách đường dẫn khách đang mở (chẩn đoán/tests).
    pub fn ds_khach(&self) -> Vec<String> {
        self.goc.iter().map(|g| hien(&g.khach)).collect()
    }

    fn chan(&self, p: &str, vi_sao: &str) -> String {
        format!("{DAU_QUYEN} wasm preopen: {vi_sao} (`{p}`)")
    }

    /// Ánh xạ đường dẫn khách → `(chỉ số gốc, đường dẫn host)`. Chưa canonicalize.
    fn giai(&self, p: &str, can_ghi: bool) -> Result<(usize, PathBuf), String> {
        if self.goc.is_empty() {
            return Err(self.chan(p, "chưa cấu hình preopen"));
        }
        let (tuyet_doi, parts) = chuan_hoa_yeu_cau(p)
            .map_err(|e| self.chan(p, &format!("đường dẫn không hợp lệ: {e}")))?;
        let idx = if tuyet_doi {
            let mut chon: Option<(usize, usize)> = None;
            for (i, g) in self.goc.iter().enumerate() {
                if trong_goc(&parts, &g.khach) {
                    let do_dai = g.khach.len();
                    if chon.map(|(_, d)| do_dai > d).unwrap_or(true) {
                        chon = Some((i, do_dai));
                    }
                }
            }
            match chon {
                Some((i, _)) => i,
                None => return Err(self.chan(p, "ngoài mọi preopen")),
            }
        } else {
            0
        };
        let g = &self.goc[idx];
        if can_ghi && !g.ghi {
            return Err(self.chan(p, "preopen chỉ đọc"));
        }
        let mut host = g.that.clone();
        let rel = if tuyet_doi {
            &parts[g.khach.len()..]
        } else {
            &parts[..]
        };
        for seg in rel {
            host.push(seg);
        }
        Ok((idx, host))
    }

    /// Canonicalize **đầy đủ** và kiểm vẫn trong gốc; lỗi IO giữ nguyên là lỗi dữ liệu.
    fn that_day_du(&self, idx: usize, p: &str, host: &Path) -> Result<PathBuf, String> {
        match fs::canonicalize(host) {
            Ok(that) => {
                if !that.starts_with(&self.goc[idx].that) {
                    return Err(self.chan(p, "symlink/đường dẫn trỏ ra ngoài preopen"));
                }
                Ok(that)
            }
            Err(e) => {
                // WASI trả `ENOENT` cho symlink trỏ ra ngoài preopen (không rò tồn tại). Nếu
                // chính đích cuối là symlink ⇒ đây là nỗ lực thoát gốc, báo quyền rõ ràng.
                let la_symlink = fs::symlink_metadata(host)
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false);
                if la_symlink {
                    Err(self.chan(p, "symlink/đường dẫn trỏ ra ngoài preopen"))
                } else {
                    Err(format!("không truy cập được `{p}`: {e}"))
                }
            }
        }
    }

    /// Canonicalize **thư mục cha** (đích có thể chưa tồn tại) và kiểm containment.
    fn that_cha(&self, idx: usize, p: &str, host: &Path) -> Result<PathBuf, String> {
        let ten = host
            .file_name()
            .ok_or_else(|| format!("đường dẫn không có tên tệp: `{p}`"))?;
        let cha = host
            .parent()
            .ok_or_else(|| format!("đường dẫn không có thư mục cha: `{p}`"))?;
        let cha_that = fs::canonicalize(cha)
            .map_err(|e| format!("không truy cập được thư mục cha của `{p}`: {e}"))?;
        if !cha_that.starts_with(&self.goc[idx].that) {
            return Err(self.chan(p, "thư mục cha trỏ ra ngoài preopen"));
        }
        Ok(cha_that.join(ten))
    }

    /// Từ chối ghi/xoá/đổi tên nếu đích cuối là symlink (không đi theo link).
    fn khong_symlink_cuoi(&self, p: &str, host: &Path) -> Result<(), String> {
        match fs::symlink_metadata(host) {
            Ok(md) if md.file_type().is_symlink() => {
                Err(self.chan(p, "từ chối thao tác qua symlink"))
            }
            Ok(_) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("không đọc được metadata `{p}`: {e}")),
        }
    }

    fn metadata_opt(
        &self,
        idx: usize,
        p: &str,
        host: &Path,
    ) -> Result<Option<fs::Metadata>, String> {
        if host == self.goc[idx].that {
            return match fs::symlink_metadata(host) {
                Ok(md) => Ok(Some(md)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(format!("không đọc được metadata `{p}`: {e}")),
            };
        }
        let h = self.that_cha(idx, p, host)?;
        match fs::symlink_metadata(&h) {
            Ok(md) => Ok(Some(md)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("không đọc được metadata `{p}`: {e}")),
        }
    }

    fn metadata_khong_theo(
        &self,
        idx: usize,
        p: &str,
        host: &Path,
    ) -> Result<fs::Metadata, String> {
        self.metadata_opt(idx, p, host)?
            .ok_or_else(|| format!("không tìm thấy `{p}`"))
    }

    fn doc_byte(&self, p: &str) -> Result<Vec<u8>, String> {
        let (idx, host) = self.giai(p, false)?;
        let that = self.that_day_du(idx, p, &host)?;
        fs::read(&that).map_err(|e| format!("không đọc được `{p}`: {e}"))
    }

    fn ghi_byte(&self, p: &str, data: &[u8]) -> Result<(), String> {
        let (idx, host) = self.giai(p, true)?;
        let dich = self.that_cha(idx, p, &host)?;
        self.khong_symlink_cuoi(p, &dich)?;
        fs::write(&dich, data).map_err(|e| format!("không ghi được `{p}`: {e}"))
    }

    fn hien_lai(&self, idx: usize, that: &Path) -> Option<String> {
        let g = &self.goc[idx];
        let rel = that.strip_prefix(&g.that).ok()?;
        let mut parts = g.khach.clone();
        for c in rel.components() {
            if let std::path::Component::Normal(s) = c {
                parts.push(s.to_string_lossy().to_string());
            }
        }
        Some(hien(&parts))
    }

    fn liet_ke(&self, p: &str) -> Result<(usize, PathBuf, MucThuMuc), String> {
        let (idx, host) = self.giai(p, false)?;
        let that = self.that_day_du(idx, p, &host)?;
        let rd = fs::read_dir(&that).map_err(|e| format!("không đọc được thư mục `{p}`: {e}"))?;
        let mut ds = Vec::new();
        for e in rd {
            let e = e.map_err(|x| format!("lỗi đọc mục trong `{p}`: {x}"))?;
            let md = e
                .metadata()
                .map_err(|x| format!("lỗi đọc metadata của mục trong `{p}`: {x}"))?;
            ds.push((e.file_name().to_string_lossy().to_string(), md));
        }
        ds.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
        Ok((idx, that, ds))
    }
}

fn mode_cua(md: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        md.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = md;
        0
    }
}

fn modified_ns(md: &fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

impl FsHost for WasiFs {
    fn read(&mut self, p: &str) -> Result<String, String> {
        let bytes = self.doc_byte(p)?;
        String::from_utf8(bytes).map_err(|e| format!("`{p}` không phải UTF-8: {e}"))
    }

    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        self.ghi_byte(p, s.as_bytes())
    }

    fn exists(&mut self, p: &str) -> bool {
        self.exists_quyen(p).unwrap_or(false)
    }

    fn exists_quyen(&mut self, p: &str) -> Result<bool, String> {
        let (idx, host) = self.giai(p, false)?;
        match fs::canonicalize(&host) {
            Ok(that) => {
                if !that.starts_with(&self.goc[idx].that) {
                    return Err(self.chan(p, "symlink/đường dẫn trỏ ra ngoài preopen"));
                }
                Ok(true)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let la_symlink = fs::symlink_metadata(&host)
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false);
                if la_symlink {
                    Err(self.chan(p, "symlink/đường dẫn trỏ ra ngoài preopen"))
                } else {
                    Ok(false)
                }
            }
            Err(e) => Err(format!("không truy cập được `{p}`: {e}")),
        }
    }

    fn list(&mut self, p: &str) -> Result<Vec<String>, String> {
        let (_, _, ds) = self.liet_ke(p)?;
        Ok(ds.into_iter().map(|(n, _)| n).collect())
    }

    fn list_entries(&mut self, p: &str) -> Result<Vec<MucFs>, String> {
        let (_, _, ds) = self.liet_ke(p)?;
        Ok(ds
            .into_iter()
            .map(|(name, md)| MucFs {
                name,
                size: md.len(),
                modified_ns: modified_ns(&md),
                is_file: md.is_file(),
                mode: mode_cua(&md),
            })
            .collect())
    }

    fn read_lines(&mut self, p: &str) -> Result<Vec<String>, String> {
        self.doc_byte(p)
            .and_then(|b| String::from_utf8(b).map_err(|e| format!("`{p}` không phải UTF-8: {e}")))
            .map(|s| chia_dong(&s))
    }

    fn append(&mut self, p: &str, s: &str) -> Result<(), String> {
        use std::io::Write;
        let (idx, host) = self.giai(p, true)?;
        let dich = self.that_cha(idx, p, &host)?;
        self.khong_symlink_cuoi(p, &dich)?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&dich)
            .map_err(|e| format!("không mở được `{p}` để ghi nối: {e}"))?;
        f.write_all(s.as_bytes())
            .map_err(|e| format!("không ghi nối được `{p}`: {e}"))
    }

    fn mkdir(&mut self, p: &str) -> Result<(), String> {
        let (idx, host) = self.giai(p, true)?;
        let dich = self.that_cha(idx, p, &host)?;
        self.khong_symlink_cuoi(p, &dich)?;
        fs::create_dir(&dich).map_err(|e| format!("không tạo được thư mục `{p}`: {e}"))
    }

    fn mkdir_all(&mut self, p: &str) -> Result<(), String> {
        let (idx, host) = self.giai(p, true)?;
        let goc = &self.goc[idx].that;
        let rel = host
            .strip_prefix(goc)
            .map_err(|_| self.chan(p, "đường dẫn ngoài preopen"))?;
        let mut cur = goc.clone();
        for c in rel.components() {
            let std::path::Component::Normal(seg) = c else {
                continue;
            };
            cur.push(seg);
            match fs::symlink_metadata(&cur) {
                Ok(md) => {
                    if md.file_type().is_symlink() {
                        return Err(self.chan(p, "từ chối đi qua symlink trong mkdir_all"));
                    }
                    if !md.is_dir() {
                        return Err(format!(
                            "không tạo được `{p}`: `{}` không phải thư mục",
                            cur.display()
                        ));
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    fs::create_dir(&cur)
                        .map_err(|x| format!("không tạo được thư mục `{p}`: {x}"))?;
                }
                Err(e) => return Err(format!("không truy cập được `{p}`: {e}")),
            }
        }
        Ok(())
    }

    fn remove(&mut self, p: &str) -> Result<(), String> {
        let (idx, host) = self.giai(p, true)?;
        let md = self.metadata_khong_theo(idx, p, &host)?;
        if md.is_dir() {
            return Err(format!("`{p}` là thư mục; chỉ xoá tệp"));
        }
        let dich = self.that_cha(idx, p, &host)?;
        fs::remove_file(&dich).map_err(|e| format!("không xoá được `{p}`: {e}"))
    }

    fn walk(&mut self, p: &str) -> Result<Vec<String>, String> {
        const TOI_DA_MUC: usize = 100_000;
        const TOI_DA_SAU: usize = 256;
        let (idx, host) = self.giai(p, false)?;
        let goc = self.that_day_du(idx, p, &host)?;
        let mut ra = Vec::new();
        let mut stack = vec![(goc, String::new(), 0usize)];
        while let Some((dir, tien_to, sau)) = stack.pop() {
            if sau > TOI_DA_SAU {
                return Err(format!("`{p}` quá sâu (> {TOI_DA_SAU})"));
            }
            let rd = fs::read_dir(&dir)
                .map_err(|e| format!("không đọc được `{}`: {e}", dir.display()))?;
            let mut ds = Vec::new();
            for e in rd {
                let e = e.map_err(|x| format!("lỗi đọc mục trong `{}`: {x}", dir.display()))?;
                ds.push(e);
            }
            ds.sort_by(|a, b| {
                a.file_name()
                    .to_string_lossy()
                    .as_bytes()
                    .cmp(b.file_name().to_string_lossy().as_bytes())
            });
            let mut con: Vec<(PathBuf, String, usize)> = Vec::new();
            for e in ds.into_iter().rev() {
                let ten = e.file_name().to_string_lossy().to_string();
                let duong = if tien_to.is_empty() {
                    ten.clone()
                } else {
                    format!("{tien_to}/{ten}")
                };
                let ft = e
                    .file_type()
                    .map_err(|x| format!("lỗi metadata `{}`: {x}", e.path().display()))?;
                if ft.is_symlink() {
                    continue;
                }
                if ft.is_dir() {
                    ra.push(format!("{duong}/"));
                    con.push((e.path(), duong, sau + 1));
                } else {
                    ra.push(duong);
                }
                if ra.len() > TOI_DA_MUC {
                    return Err(format!("`{p}` có quá {TOI_DA_MUC} mục"));
                }
            }
            stack.extend(con);
        }
        ra.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        Ok(ra)
    }

    fn copy(&mut self, from: &str, to: &str) -> Result<(), String> {
        use std::io::Write;
        let (idx_n, host_n) = self.giai(from, false)?;
        let nguon = self.that_cha(idx_n, from, &host_n)?;
        let md = fs::symlink_metadata(&nguon)
            .map_err(|e| format!("không đọc được nguồn `{from}`: {e}"))?;
        if md.file_type().is_symlink() {
            return Err(format!("nguồn `{from}` là symlink; từ chối"));
        }
        if !md.is_file() {
            return Err(format!("nguồn `{from}` không phải tệp thường"));
        }
        let (idx_d, host_d) = self.giai(to, true)?;
        let dich = self.that_cha(idx_d, to, &host_d)?;
        if fs::symlink_metadata(&dich).is_ok() {
            return Err(format!("đích `{to}` đã tồn tại; từ chối ghi đè"));
        }
        let mut vao =
            fs::File::open(&nguon).map_err(|e| format!("không mở nguồn `{from}`: {e}"))?;
        let mut ra = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dich)
            .map_err(|e| format!("không tạo đích `{to}`: {e}"))?;
        let kq = io::copy(&mut vao, &mut ra).and_then(|_| ra.flush());
        if let Err(e) = kq {
            let _ = fs::remove_file(&dich);
            return Err(format!("chép `{from}` → `{to}` lỗi: {e}"));
        }
        Ok(())
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        let (idx_n, host_n) = self.giai(from, true)?;
        let (idx_d, host_d) = self.giai(to, true)?;
        let nguon = self.that_cha(idx_n, from, &host_n)?;
        let dich = self.that_cha(idx_d, to, &host_d)?;
        let md = fs::symlink_metadata(&nguon)
            .map_err(|e| format!("không đọc được nguồn `{from}`: {e}"))?;
        if md.file_type().is_symlink() || !md.is_file() {
            return Err(format!("nguồn `{from}` không phải tệp thường"));
        }
        if fs::symlink_metadata(&dich).is_ok() {
            return Err(format!("đích `{to}` đã tồn tại; từ chối ghi đè"));
        }
        fs::rename(&nguon, &dich).map_err(|e| format!("đổi tên `{from}` → `{to}` lỗi: {e}"))
    }

    fn size(&mut self, p: &str) -> Result<i64, String> {
        let (idx, host) = self.giai(p, false)?;
        let md = self.metadata_khong_theo(idx, p, &host)?;
        if !md.is_file() {
            return Err(format!("`{p}` không phải tệp thường"));
        }
        i64::try_from(md.len()).map_err(|_| format!("`{p}` quá lớn để biểu diễn"))
    }

    fn is_file(&mut self, p: &str) -> Result<bool, String> {
        let (idx, host) = self.giai(p, false)?;
        Ok(self
            .metadata_opt(idx, p, &host)?
            .map(|m| m.is_file())
            .unwrap_or(false))
    }

    fn is_dir(&mut self, p: &str) -> Result<bool, String> {
        let (idx, host) = self.giai(p, false)?;
        Ok(self
            .metadata_opt(idx, p, &host)?
            .map(|m| m.is_dir())
            .unwrap_or(false))
    }

    fn is_symlink(&mut self, p: &str) -> Result<bool, String> {
        let (idx, host) = self.giai(p, false)?;
        Ok(self
            .metadata_opt(idx, p, &host)?
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false))
    }

    fn read_link(&mut self, p: &str) -> Result<String, String> {
        let (idx, host) = self.giai(p, false)?;
        let that_cha = self.that_cha(idx, p, &host)?;
        let md =
            fs::symlink_metadata(&that_cha).map_err(|e| format!("không đọc được `{p}`: {e}"))?;
        if !md.file_type().is_symlink() {
            return Err(format!("`{p}` không phải symlink"));
        }
        let dich =
            fs::read_link(&that_cha).map_err(|e| format!("không đọc được link `{p}`: {e}"))?;
        if let Ok(rel) = dich.strip_prefix(&self.goc[idx].that) {
            let mut parts = self.goc[idx].khach.clone();
            for c in rel.components() {
                if let std::path::Component::Normal(s) = c {
                    parts.push(s.to_string_lossy().to_string());
                }
            }
            return Ok(hien(&parts));
        }
        Ok(dich.to_string_lossy().to_string())
    }

    fn canonicalize(&mut self, p: &str) -> Result<String, String> {
        let (idx, host) = self.giai(p, false)?;
        let that = self.that_day_du(idx, p, &host)?;
        self.hien_lai(idx, &that)
            .ok_or_else(|| self.chan(p, "không ánh xạ ngược được về đường dẫn khách"))
    }

    fn current_dir(&mut self) -> Result<String, String> {
        self.cwd
            .clone()
            .ok_or_else(|| self.chan(".", "chưa cấu hình preopen"))
    }

    fn read_bytes(&mut self, p: &str) -> Result<Vec<u8>, String> {
        self.doc_byte(p)
    }

    fn write_bytes(&mut self, p: &str, data: &[u8]) -> Result<(), String> {
        self.ghi_byte(p, data)
    }
}
