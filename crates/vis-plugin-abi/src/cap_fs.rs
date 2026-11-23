// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 (kiểm toán bảo mật)** — mở tệp cho `PluginCtx` **từ fd gốc** thay vì
//! `canonicalize` rồi `std::fs::read/write` theo tên.
//!
//! Lỗ hổng đã vá (ca đỏ `cap_ghi_khong_di_theo_symlink_o_ten_cuoi`): bản cũ canonicalize **thư mục
//! cha** rồi `std::fs::write(canon_parent/ten)`; nếu **tên cuối** là symlink trỏ ra ngoài `cap` thì
//! lệnh ghi đi theo link và sửa tệp ngoài — **tất định**, không cần đua. Đường đọc cũ cũng có khe
//! kiểm-rồi-mở giữa `canonicalize(path)` và `read_to_string(canon)`.
//!
//! Cách làm giống `vis-host-native::quyen_mo` (ST-295/ST-309): mở gốc bằng `openat2(NO_SYMLINKS)`
//! (fallback: đi từng thành phần với `O_NOFOLLOW`), rồi mở đường tương đối bằng
//! `openat2(RESOLVE_BENEATH)` — ghi thêm `NO_SYMLINKS` để không bao giờ ghi xuyên symlink.
//! Mọi lỗi biên đều là [`Error`] có mã, không panic, không `unsafe`.
//!
//! Chỉ biên dịch trên Unix; Windows giữ đường `canonicalize` cũ trong `lib.rs` (mức bảo đảm thấp
//! hơn, đã ghi ở `agent/HOST-BAO-DAM.md`).

use std::ffi::OsStr;
use std::io::{Read as _, Write as _};
use std::os::fd::OwnedFd;
use std::path::{Component, Path};

use rustix::fs::{self, Mode, OFlags};

use crate::{Error, Result};

fn mo_ta(p: &Path) -> String {
    p.to_string_lossy().to_string()
}

/// Lỗi mở: symlink/thoát gốc là **vi phạm quyền** (`VS0604`); còn lại là lỗi I/O (`VS0501`).
fn loi_mo(p: &Path, e: std::io::Error) -> Error {
    let ma = e.raw_os_error();
    if ma == Some(rustix::io::Errno::LOOP.raw_os_error())
        || ma == Some(rustix::io::Errno::XDEV.raw_os_error())
    {
        return Error::cap_denied("Dir", &mo_ta(p));
    }
    Error::io(format!("không mở được `{}`: {e}", p.display()))
}

/// Mở gốc `cap` **không theo symlink ở bất kỳ thành phần nào**.
fn mo_goc(goc: &Path) -> std::io::Result<OwnedFd> {
    if !goc.is_absolute() {
        return Err(std::io::Error::from_raw_os_error(
            rustix::io::Errno::INVAL.raw_os_error(),
        ));
    }
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{openat2, ResolveFlags};
        match openat2(
            rustix::fs::CWD,
            goc,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS,
        ) {
            Ok(fd) => return Ok(fd),
            Err(e) if e == rustix::io::Errno::NOSYS => {}
            Err(e) => return Err(std::io::Error::from(e)),
        }
    }
    let mut cur = fs::open(
        Path::new("/"),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    for c in goc.components() {
        match c {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(x) => {
                let fl = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
                cur = fs::openat(&cur, x, fl, Mode::empty()).map_err(std::io::Error::from)?;
            }
            Component::ParentDir | Component::Prefix(_) => {
                return Err(std::io::Error::from_raw_os_error(
                    rustix::io::Errno::INVAL.raw_os_error(),
                ));
            }
        }
    }
    Ok(cur)
}

/// Thành phần `Normal` của `rel`; từ chối rỗng/`..`/tuyệt đối (phòng thủ sau `strip_prefix`).
fn thanh(rel: &Path) -> std::io::Result<Vec<&OsStr>> {
    let mut v = Vec::new();
    for c in rel.components() {
        match c {
            Component::Normal(x) => v.push(x),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(std::io::Error::from_raw_os_error(
                    rustix::io::Errno::INVAL.raw_os_error(),
                ));
            }
        }
    }
    if v.is_empty() {
        return Err(std::io::Error::from_raw_os_error(
            rustix::io::Errno::ISDIR.raw_os_error(),
        ));
    }
    Ok(v)
}

/// Mở `rel` dưới `goc` với cờ `cuoi`. `chi_beneath = true` (đọc) cho phép symlink **bên trong**
/// gốc miễn không thoát gốc; `false` (ghi) cấm mọi symlink. Fallback luôn đi từng thành phần
/// với `O_NOFOLLOW` ở mọi cấp.
fn mo_duoi_goc(
    goc: &Path,
    rel: &Path,
    cuoi: OFlags,
    mode: Mode,
    chi_beneath: bool,
) -> std::io::Result<OwnedFd> {
    let root = mo_goc(goc)?;
    let ds = thanh(rel)?;
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{openat2, ResolveFlags};
        let mut resolve = ResolveFlags::BENEATH;
        if !chi_beneath {
            resolve |= ResolveFlags::NO_SYMLINKS;
        }
        match openat2(&root, rel, cuoi | OFlags::CLOEXEC, mode, resolve) {
            Ok(fd) => return Ok(fd),
            Err(e) if e == rustix::io::Errno::NOSYS => {}
            Err(e) => return Err(std::io::Error::from(e)),
        }
    }
    let _ = chi_beneath;
    // Fallback (macOS/BSD/kernel < 5.6): mọi thành phần `O_NOFOLLOW`, kể cả thành phần cuối.
    let giua = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut cur = root;
    let n = ds.len();
    for (i, x) in ds.iter().enumerate() {
        let fl = if i + 1 == n {
            cuoi | OFlags::CLOEXEC | OFlags::NOFOLLOW
        } else {
            giua
        };
        cur = fs::openat(&cur, *x, fl, mode).map_err(std::io::Error::from)?;
    }
    Ok(cur)
}

/// Đọc UTF-8 `rel` dưới `goc` (đã kiểm canonical **và** bọc bởi `RESOLVE_BENEATH`).
pub(super) fn doc(goc: &Path, rel: &Path, hien_thi: &Path) -> Result<String> {
    let fd = mo_duoi_goc(goc, rel, OFlags::RDONLY, Mode::empty(), true)
        .map_err(|e| loi_mo(hien_thi, e))?;
    let mut f = std::fs::File::from(fd);
    let mut s = String::new();
    f.read_to_string(&mut s)
        .map_err(|e| Error::io(format!("không đọc được `{}`: {e}", hien_thi.display())))?;
    Ok(s)
}

/// Ghi đè `rel` dưới `goc`: `O_CREAT|O_TRUNC|O_NOFOLLOW` trong `RESOLVE_BENEATH|NO_SYMLINKS`.
pub(super) fn ghi(goc: &Path, rel: &Path, data: &[u8], hien_thi: &Path) -> Result<()> {
    let fd = mo_duoi_goc(
        goc,
        rel,
        OFlags::WRONLY | OFlags::CREATE | OFlags::TRUNC,
        Mode::from_raw_mode(0o666),
        false,
    )
    .map_err(|e| loi_mo(hien_thi, e))?;
    let mut f = std::fs::File::from(fd);
    f.write_all(data)
        .map_err(|e| Error::io(format!("không ghi được `{}`: {e}", hien_thi.display())))
}
