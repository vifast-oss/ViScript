// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-295 (VD-276 §1)** — mở **từ fd gốc** cho mọi đường đọc/ghi có `cap[Dir]`/`cap[File]`.
//!
//! Vì sao: `FsGuard` cũ **kiểm theo tên rồi mở theo tên** (`canonicalize` + `std::fs::write`).
//! Giữa hai syscall, thư mục cha có thể bị đổi để tên trỏ thành symlink ra ngoài vùng được cấp
//! (`#109b`/`#140`). Ở đây mỗi thành phần được mở bằng `openat` với `O_NOFOLLOW`/`O_DIRECTORY`;
//! trên Linux ≥ 5.6 ưu tiên `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)`; kernel cũ
//! (`ENOSYS`) rơi về đi từng thành phần. Không `unsafe`, không phụ thuộc OS ngoài `rustix`.
//!
//! Đường dẫn được **chuẩn hoá từ vựng** (`.`/`..`) trước khi đi; `..` ra ngoài gốc bị loại bằng
//! `strip_prefix`, không dựa vào `canonicalize` (mà chính nó là bước kiểm-rồi-mở cũ).

#![cfg(unix)]

use std::ffi::OsStr;
use std::io::{self, Read as _, Write as _};
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{self, AtFlags, Mode, OFlags};

use vis_interp::stdlib::DAU_QUYEN;

use crate::quyen::{Quyen, QuyenCap};

/// Chuẩn hoá **từ vựng**: bỏ `.`, gộp `//`, xử lý `..` bằng `PathBuf::pop` (không chạm đĩa).
/// Trả đường dẫn **tuyệt đối** (mọi caller đã ghép `goc`).
pub(crate) fn chuan_tu_van(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Prefix(_) | Component::RootDir => out.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(x) => out.push(x),
        }
    }
    out
}

/// Tìm `cap[Dir]`/`cap[File]` phủ `p` và trả `(gốc, đường-dẫn-tương-đối)`; `Ok(None)` khi **chưa
/// bật** quyền (chạy không `cap`, giữ hành vi cũ). Khi đã bật mà không khớp ⇒ lỗi `[quyền]` cùng
/// câu chữ với `Quyen::kiem_canon`.
///
/// `can_write` bắt buộc mục khớp có `(write)`; nếu chỉ khớp mục chỉ-đọc thì trả lỗi `(write)`.
pub(crate) fn goc_va_tuong_doi(
    q: &Quyen,
    p: &str,
    can_write: bool,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    if !q.bat {
        return Ok(None);
    }
    let abs = chuan_tu_van(&q.tuyet_doi(p));
    let mut tot_nhat: Option<(usize, &QuyenCap, PathBuf, PathBuf)> = None;
    let mut chi_doc = false;
    for c in &q.moc {
        if c.kind != "Dir" && c.kind != "File" {
            continue;
        }
        let root = chuan_tu_van(Path::new(&c.root));
        let khop = if c.kind == "File" {
            abs == root
        } else {
            abs.starts_with(&root)
        };
        if !khop {
            continue;
        }
        if can_write && !c.write {
            chi_doc = true;
            continue;
        }
        let (goc, tuong_doi) = if c.kind == "File" {
            let ten = match root.file_name() {
                Some(t) => PathBuf::from(t),
                None => continue,
            };
            match root.parent() {
                Some(cha) => (cha.to_path_buf(), ten),
                None => continue,
            }
        } else {
            let rel = match abs.strip_prefix(&root) {
                Ok(r) => r.to_path_buf(),
                Err(_) => continue,
            };
            (root.clone(), rel)
        };
        let do_dai = goc.as_os_str().len();
        if tot_nhat.as_ref().is_none_or(|(n, ..)| do_dai > *n) {
            tot_nhat = Some((do_dai, c, goc, tuong_doi));
        }
    }
    if let Some((_, _, goc, rel)) = tot_nhat {
        return Ok(Some((goc, rel)));
    }
    let co_fs = q.moc.iter().any(|c| c.kind == "Dir" || c.kind == "File");
    if q.siet && !co_fs {
        return Err(format!("{DAU_QUYEN}[quyền] cần khai `cap[Dir]` cho `{p}`"));
    }
    if chi_doc {
        return Err(format!(
            "{DAU_QUYEN}[quyền] thiếu `(write)` trên `cap[Dir]` cho `{p}`"
        ));
    }
    Err(format!("{DAU_QUYEN}[quyền] thiếu `cap[Dir]` cho `{p}`"))
}

/// Thành phần `Normal` của `rel`; từ chối rỗng/`..`/tuyệt đối (phòng thủ sau `strip_prefix`).
fn thanh(rel: &Path) -> io::Result<Vec<&OsStr>> {
    let mut v = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(x) => v.push(x),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(io::Error::from_raw_os_error(22));
            }
        }
    }
    if v.is_empty() {
        return Err(io::Error::from_raw_os_error(21)); // EISDIR
    }
    Ok(v)
}

fn mo_goc(goc: &Path) -> io::Result<OwnedFd> {
    // `NOFOLLOW`: gốc `cap` đã được canonicalize lúc khởi động; nếu tên gốc bị đổi thành symlink
    // sau đó thì **từ chối**, không mở sang cây khác (chống thay gốc).
    fs::open(
        goc,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)
}

/// Mở `rel` dưới `goc`: `openat2` (Linux) trước, rơi về đi từng thành phần khi `ENOSYS`.
fn mo_duoi_goc(goc: &Path, rel: &Path, cuoi: OFlags, mode: Mode) -> io::Result<OwnedFd> {
    let root = mo_goc(goc)?;
    let ds = thanh(rel)?;
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{openat2, ResolveFlags};
        let co = cuoi | OFlags::CLOEXEC | OFlags::NOFOLLOW;
        match openat2(
            &root,
            rel,
            co,
            mode,
            ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
        ) {
            Ok(fd) => return Ok(fd),
            Err(e) if e == rustix::io::Errno::NOSYS => {}
            Err(e) => return Err(io::Error::from(e)),
        }
    }
    // Fallback (macOS/BSD/kernel < 5.6): mở từng thành phần, không đi theo symlink ở bất kỳ cấp.
    let giua = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut cur = root;
    let n = ds.len();
    for (i, x) in ds.iter().enumerate() {
        let fl = if i + 1 == n {
            cuoi | OFlags::CLOEXEC | OFlags::NOFOLLOW
        } else {
            giua
        };
        cur = fs::openat(&cur, *x, fl, mode).map_err(io::Error::from)?;
    }
    Ok(cur)
}

/// Đọc UTF-8 từ `rel` dưới `goc`; từ chối symlink ở mọi thành phần.
pub(crate) fn doc(goc: &Path, rel: &Path) -> io::Result<String> {
    let fd = mo_duoi_goc(goc, rel, OFlags::RDONLY, Mode::empty())?;
    let mut f = std::fs::File::from(fd);
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

/// Đọc byte thô.
pub(crate) fn doc_byte(goc: &Path, rel: &Path) -> io::Result<Vec<u8>> {
    let fd = mo_duoi_goc(goc, rel, OFlags::RDONLY, Mode::empty())?;
    let mut f = std::fs::File::from(fd);
    let mut b = Vec::new();
    f.read_to_end(&mut b)?;
    Ok(b)
}

/// Ghi đè (tạo nếu thiếu) — `O_CREAT|O_TRUNC|O_NOFOLLOW`.
pub(crate) fn ghi_de(goc: &Path, rel: &Path, data: &[u8]) -> io::Result<()> {
    let fd = mo_duoi_goc(
        goc,
        rel,
        OFlags::WRONLY | OFlags::CREATE | OFlags::TRUNC,
        Mode::from_raw_mode(0o666),
    )?;
    let mut f = std::fs::File::from(fd);
    f.write_all(data)
}

/// Ghi thêm cuối (tạo nếu thiếu) — `O_APPEND`.
pub(crate) fn ghi_them(goc: &Path, rel: &Path, data: &[u8]) -> io::Result<()> {
    let fd = mo_duoi_goc(
        goc,
        rel,
        OFlags::WRONLY | OFlags::CREATE | OFlags::APPEND,
        Mode::from_raw_mode(0o666),
    )?;
    let mut f = std::fs::File::from(fd);
    f.write_all(data)
}

/// Tạo **một** thư mục; cha phải tồn tại (giống `create_dir`).
pub(crate) fn tao_thu_muc(goc: &Path, rel: &Path) -> io::Result<()> {
    let ds = thanh(rel)?;
    let (ten, cha) = ds
        .split_last()
        .ok_or_else(|| io::Error::from_raw_os_error(22))?;
    let cha_rel: PathBuf = cha.iter().collect();
    let fd = if cha.is_empty() {
        mo_goc(goc)?
    } else {
        mo_duoi_goc(
            goc,
            &cha_rel,
            OFlags::RDONLY | OFlags::DIRECTORY,
            Mode::empty(),
        )?
    };
    fs::mkdirat(&fd, *ten, Mode::from_raw_mode(0o777)).map_err(io::Error::from)
}

/// Tạo **đệ quy** (idempotent khi đích đã là thư mục); mọi thành phần đi qua `openat` không symlink.
pub(crate) fn tao_thu_muc_de_quy(goc: &Path, rel: &Path) -> io::Result<()> {
    let ds = thanh(rel)?;
    let mut cur = mo_goc(goc)?;
    for x in &ds {
        let fl = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        match fs::openat(&cur, *x, fl, Mode::empty()) {
            Ok(fd) => cur = fd,
            Err(rustix::io::Errno::NOENT) => {
                fs::mkdirat(&cur, *x, Mode::from_raw_mode(0o777)).map_err(io::Error::from)?;
                cur = fs::openat(&cur, *x, fl, Mode::empty()).map_err(io::Error::from)?;
            }
            Err(e) => return Err(io::Error::from(e)),
        }
    }
    Ok(())
}

/// Xoá **một** tệp (không đệ quy); thư mục ⇒ lỗi như host cũ.
pub(crate) fn xoa_tep(goc: &Path, rel: &Path) -> io::Result<()> {
    let ds = thanh(rel)?;
    let (ten, cha) = ds
        .split_last()
        .ok_or_else(|| io::Error::from_raw_os_error(22))?;
    let cha_rel: PathBuf = cha.iter().collect();
    let fd = if cha.is_empty() {
        mo_goc(goc)?
    } else {
        mo_duoi_goc(
            goc,
            &cha_rel,
            OFlags::RDONLY | OFlags::DIRECTORY,
            Mode::empty(),
        )?
    };
    fs::unlinkat(&fd, *ten, AtFlags::empty()).map_err(io::Error::from)
}

fn mo_cha(goc: &Path, rel: &Path) -> io::Result<(OwnedFd, std::ffi::OsString)> {
    let ds = thanh(rel)?;
    let (ten, cha) = ds
        .split_last()
        .ok_or_else(|| io::Error::from_raw_os_error(22))?;
    let cha_rel: PathBuf = cha.iter().collect();
    let fd = if cha.is_empty() {
        mo_goc(goc)?
    } else {
        mo_duoi_goc(
            goc,
            &cha_rel,
            OFlags::RDONLY | OFlags::DIRECTORY,
            Mode::empty(),
        )?
    };
    Ok((fd, ten.to_os_string()))
}

/// Đổi tên; **từ chối đích đã tồn tại** (giữ ngữ nghĩa `doi_ten_tep` của host cũ).
pub(crate) fn doi_ten(goc_a: &Path, a: &Path, goc_b: &Path, b: &Path) -> io::Result<()> {
    let (fd_a, ten_a) = mo_cha(goc_a, a)?;
    let (fd_b, ten_b) = mo_cha(goc_b, b)?;
    // `RENAME_NOREPLACE` nguyên tử ở nơi hỗ trợ (Linux/apple) ⇒ không còn khe "kiểm đích rồi
    // rename"; nơi khác giữ kiểm + `renameat` (hai fd cha cố định, không thể bị symlink dẫn).
    #[cfg(any(target_os = "linux", target_vendor = "apple"))]
    {
        use rustix::fs::RenameFlags;
        fs::renameat_with(&fd_a, &ten_a, &fd_b, &ten_b, RenameFlags::NOREPLACE)
            .map_err(io::Error::from)
    }
    #[cfg(not(any(target_os = "linux", target_vendor = "apple")))]
    {
        match fs::statat(&fd_b, &ten_b, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => return Err(io::Error::from_raw_os_error(17)), // EEXIST
            Err(rustix::io::Errno::NOENT) => {}
            Err(e) => return Err(io::Error::from(e)),
        }
        fs::renameat(&fd_a, &ten_a, &fd_b, &ten_b).map_err(io::Error::from)
    }
}

/// Chép tệp: nguồn mở **đọc** an toàn, đích `O_CREAT|O_EXCL` (không ghi đè) như host cũ.
pub(crate) fn chep(goc_a: &Path, a: &Path, goc_b: &Path, b: &Path) -> io::Result<()> {
    let fd = mo_duoi_goc(goc_a, a, OFlags::RDONLY, Mode::empty())?;
    let mut nguon = std::fs::File::from(fd);
    let fd = mo_duoi_goc(
        goc_b,
        b,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL,
        Mode::from_raw_mode(0o666),
    )?;
    let mut dich = std::fs::File::from(fd);
    io::copy(&mut nguon, &mut dich)?;
    Ok(())
}
