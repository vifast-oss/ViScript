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
//!
//! **ST-309 (kiểm toán bảo mật)**: gốc `cap` cũng được mở **không theo symlink ở mọi thành phần**
//! (`mo_goc`), không chỉ thành phần cuối — đổi một thư mục cha của gốc thành symlink sau khi khởi
//! động không còn mở sang cây khác.

#![cfg(unix)]

use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::io::{self, Read as _, Write as _};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{self, AtFlags, Mode, OFlags};

use vis_core::host::DAU_QUYEN;

use crate::quyen::{Quyen, QuyenCap};

/// **ST-300m1** — `io::Error` từ errno **theo nền** (không hardcode số của Linux).
fn loi_os(e: rustix::io::Errno) -> io::Error {
    io::Error::from_raw_os_error(e.raw_os_error())
}

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
                return Err(loi_os(rustix::io::Errno::INVAL));
            }
        }
    }
    if v.is_empty() {
        return Err(loi_os(rustix::io::Errno::ISDIR)); // EISDIR
    }
    Ok(v)
}

/// **ST-310c** — mở gốc `cap` an toàn; công khai cho `vis-plugin-host` dùng lại (một nguồn
/// primitive, không bản sao `openat2`).
pub fn mo_goc(goc: &Path) -> io::Result<OwnedFd> {
    // **ST-309 (kiểm toán bảo mật)**: `O_NOFOLLOW` chỉ chặn **thành phần cuối**; các thư mục
    // **trung gian** vẫn bị đi theo symlink. Gốc `cap` đã canonicalize lúc khởi động, nhưng nếu
    // một thư mục cha của gốc bị đổi thành symlink **sau đó** thì `open(goc, O_NOFOLLOW)` vẫn mở
    // sang cây khác (đo: ghi được `ngoai/con/tep`). Nay gốc được mở **không theo symlink ở mọi
    // thành phần** (`openat2(RESOLVE_NO_SYMLINKS)` hoặc đi từng thành phần từ `/`).
    if !goc.is_absolute() {
        return Err(loi_os(rustix::io::Errno::INVAL));
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
            Err(e) => return Err(io::Error::from(e)),
        }
    }
    // Fallback (macOS/BSD/kernel < 5.6): mở `/` rồi đi từng thành phần với `O_NOFOLLOW`.
    let mut cur = fs::open(
        Path::new("/"),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io::Error::from)?;
    for c in goc.components() {
        match c {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(x) => {
                // Darwin: O_DIRECTORY|O_NOFOLLOW trên symlink trả ENOTDIR(20), không ELOOP(62).
                // Bỏ O_DIRECTORY để symlink trả ELOOP; thành phần tệp thật vẫn fail ở bước sau.
                let fl = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
                cur = fs::openat(&cur, x, fl, Mode::empty()).map_err(io::Error::from)?;
            }
            Component::ParentDir | Component::Prefix(_) => {
                return Err(loi_os(rustix::io::Errno::INVAL));
            }
        }
    }
    Ok(cur)
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
    // Không O_DIRECTORY: Darwin trả ENOTDIR cho symlink; ELOOP mới ánh xạ VS0604.
    let giua = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
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

/// **ST-310c** — đọc UTF-8 `rel` dưới gốc `goc` cho plugin tĩnh; cùng đường `openat2` với
/// `FsGuard` (`RESOLVE_BENEATH|NO_SYMLINKS`), không có bản sao syscall.
pub fn mo_tep_doc(goc: &Path, rel: &Path) -> io::Result<String> {
    doc(goc, rel)
}

/// **ST-310c** — ghi đè `rel` dưới gốc `goc` cho plugin tĩnh; `O_CREAT|O_TRUNC|O_NOFOLLOW`
/// trong `RESOLVE_BENEATH|NO_SYMLINKS` (F2 ST-309 giữ nguyên).
pub fn mo_tep_ghi(goc: &Path, rel: &Path, data: &[u8]) -> io::Result<()> {
    ghi_de(goc, rel, data)
}

/// **ST-310c** — lỗi mở do symlink/thoát gốc là vi phạm quyền (`VS0604` ở tầng plugin).
/// Dùng errno **theo nền** (Darwin `ELOOP = 62`), như `FsGuard::loi_mo`.
pub fn la_loi_symlink(e: &io::Error) -> bool {
    let ma = e.raw_os_error();
    ma == Some(rustix::io::Errno::LOOP.raw_os_error())
        || ma == Some(rustix::io::Errno::XDEV.raw_os_error())
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
        .ok_or_else(|| loi_os(rustix::io::Errno::INVAL))?;
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
        // Darwin fallback: symlink phải trả ELOOP (VS0604), không ENOTDIR.
        let fl = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
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
        .ok_or_else(|| loi_os(rustix::io::Errno::INVAL))?;
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
        .ok_or_else(|| loi_os(rustix::io::Errno::INVAL))?;
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
        // **#194 (rá soát ST-310c)**: `statat` chỉ là **kiểm tồn tại siêu dữ liệu** trên
        // `(fd_b, ten_b)` — fd cha cố định, `SYMLINK_NOFOLLOW`, không đi theo thành phần nào;
        // nó **không** thay đổi semantics đọc/ghi nội dung (`openat2` vẫn là đường duy nhất).
        // Nợ #194 nằm ở các truy vấn `exists`/`size`/`list`/`walk` theo **đường dẫn** của
        // `FsHost`, không phải nhánh `renameat` dự phòng này.
        match fs::statat(&fd_b, &ten_b, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => return Err(loi_os(rustix::io::Errno::EXIST)), // EEXIST
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

// ===========================================================================
// **ST-311b (VD-286)** — metadata Unix theo fd (`statat`/`openat`/`readlinkat`).
//
// Đóng nợ `#194` (rá soát ST-309 §4): truy vấn siêu dữ liệu của `FsHost` trước đây kiểm-rồi-dùng
// theo **tên** (`canonicalize` rồi `std::fs::*`), có khe đua ở tầng đường dẫn. Ở đây mọi thao tác
// mở **từ fd gốc** `cap` (`openat`/`openat2` + `O_NOFOLLOW`) rồi dùng `statat`/`readlinkat`/`fstat`
// ⇒ không còn cửa sổ "đổi tên/symlink giữa kiểm và dùng". **0 `unsafe`**, cùng nguồn `openat2`
// (R9) với đường đọc/ghi đã có.
// ===========================================================================

/// Lỗi **dữ liệu** (không phải vi phạm quyền) cho metadata — `FsGuard` ghép tiền tố host cũ.
fn loi_data(msg: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// Mở thư mục cho các thành phần `ds` (đã phân giải, không symlink) dưới `goc`.
fn mo_thu_muc_theo(goc: &Path, ds: &[OsString]) -> io::Result<OwnedFd> {
    let mut cur = mo_goc(goc)?;
    for c in ds {
        let fl = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        cur = fs::openat(&cur, c, fl, Mode::empty()).map_err(io::Error::from)?;
    }
    Ok(cur)
}

/// `lstat` `rel` dưới `goc` — mở cha theo fd (`O_NOFOLLOW` mọi thành phần giữa), `statat` thành phần
/// cuối với `SYMLINK_NOFOLLOW` (không đi theo symlink cuối). `rel` rỗng ⇒ chính gốc `cap` (`fstat`).
/// `NOENT` giữ nguyên cho caller xử lý.
fn stat_khong_theo(goc: &Path, rel: &Path) -> io::Result<fs::Stat> {
    if rel.as_os_str().is_empty() {
        return fs::fstat(&mo_goc(goc)?).map_err(io::Error::from);
    }
    let (cha, ten) = mo_cha(goc, rel)?;
    fs::statat(&cha, &ten, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)
}

/// Phân giải `rel` dưới `goc` **theo symlink**, không thoát khỏi `goc`. Trả `(đường dẫn tuyệt đối đã
/// phân giải, day_du)`; `day_du=false` khi gặp đuôi **dangling** (giữ từ vựng như `quyen_chuan::chuan`).
///
/// - symlink trỏ `..`/tuyệt đối **ra ngoài** `goc` ⇒ `Err` `XDEV` (→ `VS0604` ở `FsGuard`);
/// - vòng symlink (số lần đi theo > `MAX_LINK`) ⇒ `Err` `LOOP`;
/// - thành phần giữa thiếu ⇒ `Err` `NOENT`.
pub(crate) fn phan_giai(goc: &Path, rel: &Path) -> io::Result<(PathBuf, bool)> {
    const MAX_LINK: usize = 40;
    if rel.as_os_str().is_empty() {
        return Ok((goc.to_path_buf(), true));
    }
    let ds = thanh(rel)?;
    let mut cur = mo_goc(goc)?;
    let mut out: Vec<OsString> = Vec::new();
    let mut todo: VecDeque<OsString> = ds.into_iter().map(OsStr::to_os_string).collect();
    let mut budget = MAX_LINK;
    let mut day_du = true;
    let fl = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;

    while let Some(c) = todo.pop_front() {
        if c == ".." {
            if out.pop().is_none() {
                return Err(loi_os(rustix::io::Errno::XDEV));
            }
            cur = mo_thu_muc_theo(goc, &out)?;
            continue;
        }
        match fs::openat(&cur, &c, fl, Mode::empty()) {
            Ok(fd) => {
                out.push(c);
                cur = fd;
            }
            // Linux: symlink ⇒ `ELOOP`; macOS/BSD: `O_DIRECTORY|O_NOFOLLOW` trên symlink ⇒ `ENOTDIR`.
            Err(rustix::io::Errno::LOOP) | Err(rustix::io::Errno::NOTDIR) => {
                match fs::statat(&cur, &c, AtFlags::SYMLINK_NOFOLLOW) {
                    Ok(st) if fs::FileType::from_raw_mode(st.st_mode).is_symlink() => {
                        if budget == 0 {
                            return Err(loi_os(rustix::io::Errno::LOOP));
                        }
                        budget -= 1;
                        let tgt = fs::readlinkat(&cur, &c, Vec::new()).map_err(io::Error::from)?;
                        let tp = Path::new(OsStr::from_bytes(tgt.as_bytes()));
                        if tp.is_absolute() {
                            // Đích tuyệt đối phải nằm dưới `goc`; ngoài ⇒ thoát gốc.
                            let suffix = match tp.strip_prefix(goc) {
                                Ok(s) => s.to_path_buf(),
                                Err(_) => return Err(loi_os(rustix::io::Errno::XDEV)),
                            };
                            cur = mo_goc(goc)?;
                            out.clear();
                            push_cac_thanh(suffix.components(), &mut todo, goc)?;
                        } else {
                            push_cac_thanh(tp.components(), &mut todo, goc)?;
                        }
                    }
                    Ok(_) => {
                        // Mục không-phải-thư-mục và không-phải-symlink: chỉ hợp lệ ở thành phần cuối.
                        if !todo.is_empty() {
                            return Err(loi_os(rustix::io::Errno::NOTDIR));
                        }
                        out.push(c);
                    }
                    Err(rustix::io::Errno::NOENT) => {
                        if todo.is_empty() {
                            out.push(c);
                            day_du = false;
                        } else {
                            return Err(loi_os(rustix::io::Errno::NOENT));
                        }
                    }
                    Err(e) => return Err(io::Error::from(e)),
                }
            }
            Err(rustix::io::Errno::NOENT) => {
                if todo.is_empty() {
                    out.push(c);
                    day_du = false;
                } else {
                    return Err(loi_os(rustix::io::Errno::NOENT));
                }
            }
            Err(e) => return Err(io::Error::from(e)),
        }
    }
    let mut p = goc.to_path_buf();
    for c in &out {
        p.push(c);
    }
    Ok((p, day_du))
}

/// Đẩy các thành phần của một symlink target vào **đầu** `todo` (đảo thứ tự), xử lý `.`/`..`.
fn push_cac_thanh<'a>(
    comps: impl Iterator<Item = Component<'a>>,
    todo: &mut VecDeque<OsString>,
    _goc: &Path,
) -> io::Result<()> {
    let mut extra: Vec<OsString> = Vec::new();
    for comp in comps {
        match comp {
            Component::Normal(x) => extra.push(x.to_os_string()),
            Component::CurDir => {}
            Component::ParentDir => extra.push(OsString::from("..")),
            Component::RootDir | Component::Prefix(_) => {
                return Err(loi_os(rustix::io::Errno::INVAL));
            }
        }
    }
    for x in extra.into_iter().rev() {
        todo.push_front(x);
    }
    Ok(())
}

/// `size`: số byte của **tệp thường**, lstat (không theo symlink cuối; symlink ⇒ `Err`).
pub(crate) fn kt_thuoc(goc: &Path, rel: &Path) -> io::Result<i64> {
    let st = stat_khong_theo(goc, rel)?;
    let ft = fs::FileType::from_raw_mode(st.st_mode);
    if ft.is_symlink() {
        // Chặn symlink trỏ **ra ngoài** gốc (như `kiem_fs` cũ) trước khi báo "là symlink".
        phan_giai(goc, rel)?;
        return Err(loi_data("là symlink (không đi theo)"));
    }
    if !ft.is_file() {
        return Err(loi_data("không phải tệp thường"));
    }
    Ok(st.st_size)
}

/// `is_file`: `true` ⇔ tệp thường; thiếu ⇒ `Ok(false)`.
pub(crate) fn la_tep(goc: &Path, rel: &Path) -> io::Result<bool> {
    match stat_khong_theo(goc, rel) {
        Ok(st) => Ok(fs::FileType::from_raw_mode(st.st_mode).is_file()),
        Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NOENT.raw_os_error()) => Ok(false),
        Err(e) => Err(e),
    }
}

/// `is_dir`: `true` ⇔ thư mục; thiếu ⇒ `Ok(false)`.
pub(crate) fn la_thu_muc(goc: &Path, rel: &Path) -> io::Result<bool> {
    match stat_khong_theo(goc, rel) {
        Ok(st) => Ok(fs::FileType::from_raw_mode(st.st_mode).is_dir()),
        Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NOENT.raw_os_error()) => Ok(false),
        Err(e) => Err(e),
    }
}

/// `is_symlink`: `true` ⇔ **chính** `rel` là symlink (dangling vẫn `true`); thiếu ⇒ `Ok(false)`.
pub(crate) fn la_symlink(goc: &Path, rel: &Path) -> io::Result<bool> {
    match stat_khong_theo(goc, rel) {
        Ok(st) => Ok(fs::FileType::from_raw_mode(st.st_mode).is_symlink()),
        Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NOENT.raw_os_error()) => Ok(false),
        Err(e) => Err(e),
    }
}

/// `read_link`: đọc **đích** của symlink (nguyên văn), `readlinkat`; không đi theo link.
/// Trước khi trả đích, kiểm đích phân giải **không thoát** gốc (dangling trong gốc vẫn `Ok`).
pub(crate) fn doc_link(goc: &Path, rel: &Path) -> io::Result<String> {
    let (cha, ten) = mo_cha(goc, rel)?;
    let tgt = fs::readlinkat(&cha, &ten, Vec::new()).map_err(io::Error::from)?;
    phan_giai(goc, rel)?;
    OsStr::from_bytes(tgt.as_bytes())
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| loi_data("đích không phải UTF-8 hợp lệ"))
}

/// `canonicalize`: đường dẫn tuyệt đối đã phân giải (theo symlink, trong gốc). Dangling ⇒ `Err`.
pub(crate) fn chinh_tac(goc: &Path, rel: &Path) -> io::Result<String> {
    let (p, day_du) = phan_giai(goc, rel)?;
    if !day_du {
        return Err(loi_os(rustix::io::Errno::NOENT));
    }
    p.into_os_string()
        .into_string()
        .map_err(|_| loi_data("kết quả không phải UTF-8 hợp lệ"))
}

/// `exists`: `true` ⇔ vật tồn tại (theo symlink, trong gốc); thiếu/dangling ⇒ `Ok(false)`.
pub(crate) fn ton_tai(goc: &Path, rel: &Path) -> io::Result<bool> {
    match phan_giai(goc, rel) {
        Ok((_, day_du)) => Ok(day_du),
        Err(e) if e.raw_os_error() == Some(rustix::io::Errno::NOENT.raw_os_error()) => Ok(false),
        Err(e) => Err(e),
    }
}

/// `list`: tên các mục trong thư mục (không đệ quy), sắp theo byte. Thư mục mở theo fd `O_NOFOLLOW`.
pub(crate) fn ds_muc(goc: &Path, rel: &Path) -> io::Result<Vec<String>> {
    let fd = mo_thu_muc_hoac_goc(goc, rel)?;
    let mut dir = fs::Dir::new(fd)?;
    let mut ten: Vec<String> = Vec::new();
    while let Some(e) = dir.read() {
        let e = e.map_err(io::Error::from)?;
        ten.push(e.file_name().to_string_lossy().to_string());
    }
    ten.sort();
    Ok(ten)
}

/// Mở thư mục `rel` dưới `goc` (rel rỗng ⇒ chính gốc `cap`).
fn mo_thu_muc_hoac_goc(goc: &Path, rel: &Path) -> io::Result<OwnedFd> {
    if rel.as_os_str().is_empty() {
        mo_goc(goc)
    } else {
        mo_duoi_goc(goc, rel, OFlags::RDONLY | OFlags::DIRECTORY, Mode::empty())
    }
}

/// `walk`: duyệt đệ quy dưới `rel` (thư mục), bỏ symlink, DFS + sắp byte mỗi cấp, thư mục kèm `/`.
pub(crate) fn duyet(goc: &Path, rel: &Path) -> io::Result<Vec<String>> {
    let fd = mo_thu_muc_hoac_goc(goc, rel)?;
    let mut ra = Vec::new();
    duyet_de_quy(fd.as_fd(), "", &mut ra)?;
    Ok(ra)
}

fn duyet_de_quy(dirfd: BorrowedFd<'_>, tuong_doi: &str, ra: &mut Vec<String>) -> io::Result<()> {
    let mut dir = fs::Dir::read_from(dirfd)?;
    let mut muc: Vec<(String, bool, bool)> = Vec::new();
    while let Some(e) = dir.read() {
        let e = e.map_err(io::Error::from)?;
        let ft = e.file_type();
        muc.push((
            e.file_name().to_string_lossy().to_string(),
            ft.is_dir(),
            ft.is_symlink(),
        ));
    }
    muc.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    for (ten, la_thu_muc, la_symlink) in muc {
        if la_symlink {
            continue;
        }
        let rel = if tuong_doi.is_empty() {
            ten.clone()
        } else {
            format!("{tuong_doi}/{ten}")
        };
        if la_thu_muc {
            ra.push(format!("{rel}/"));
            let fl = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            let sub = fs::openat(dirfd, &ten, fl, Mode::empty()).map_err(io::Error::from)?;
            duyet_de_quy(sub.as_fd(), &rel, ra)?;
        } else {
            ra.push(rel);
        }
    }
    Ok(())
}
