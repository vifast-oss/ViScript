// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-is-symlink)** — helper canonicalize **thuần đường dẫn** của `quyen.rs`.
//!
//! Tách khỏi `quyen.rs` để giữ **R1** (≤ 1.500 dòng/tệp) khi thêm `is_symlink`; **không** đổi
//! hành vi: hai hàm dưới đây nguyên văn từ `quyen.rs` (chỉ đổi `Self::` thành hàm tự do).

use std::path::{Path, PathBuf};

/// Số lần đi theo symlink tối đa trước khi bỏ cuộc (cùng tinh thần `MAXSYMLINKS`).
const MAX_LINK: usize = 40;

/// **ST-124** — canonicalize `p`; nếu `p` **chưa tồn tại** (ghi) thì canonicalize thư mục cha rồi
/// ghép tên; không canonicalize được ⇒ giữ nguyên `p`.
///
/// **VS3-W4 (Windows)**: `std::fs::canonicalize` trên Windows trả `ERROR_INVALID_NAME` (123) khi
/// `p` là symlink có đích **tương đối** (đích tồn tại, nhưng API không phân giải được). Nếu rơi
/// thẳng vào nhánh "canonicalize cha rồi ghép tên" thì kết quả là **đường dẫn của link** — vẫn
/// nằm trong `cap[Dir]` — nên bước kiểm quyền bỏ qua đích thật và cho lọt symlink trỏ **ra ngoài**
/// root. Vì vậy khi gặp symlink mà canonicalize lỗi, tự đọc đích và đi tiếp (đệ quy có giới hạn).
pub(crate) fn chuan(p: &Path) -> PathBuf {
    chuan_sau(p, 0)
}

fn chuan_sau(p: &Path, sau: usize) -> PathBuf {
    if let Ok(c) = std::fs::canonicalize(p) {
        return c;
    }
    if sau < MAX_LINK {
        if let Ok(md) = std::fs::symlink_metadata(p) {
            if md.file_type().is_symlink() {
                if let Ok(dich) = std::fs::read_link(p) {
                    let dich = if dich.is_absolute() {
                        dich
                    } else {
                        match p.parent() {
                            Some(cha) => cha.join(dich),
                            None => dich,
                        }
                    };
                    return chuan_sau(&dich, sau + 1);
                }
            }
        }
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
/// `root/../ngoai/moi/con` (đuôi chưa tồn tại) không được lọt qua `Path::starts_with(root)` — một
/// lỗ hổng quyền.
///
/// **VS3-W4 (Windows)**: dùng [`chuan`] cho tổ tiên thay vì `std::fs::canonicalize` trần, để
/// symlink có đích tương đối vẫn được phân giải (xem [`chuan`]); nếu không, `root/link-ra-ngoai/moi`
/// sẽ được kiểm trên đường dẫn của link (trong root) và lọt ra ngoài khi tạo thư mục.
pub(crate) fn chuan_de_quy(p: &Path) -> PathBuf {
    let mut hien = p;
    loop {
        if std::fs::symlink_metadata(hien).is_ok() {
            let mut ra = chuan(hien);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static DEM: AtomicU32 = AtomicU32::new(0);

    fn tam(tag: &str) -> PathBuf {
        let n = DEM.fetch_add(1, Ordering::Relaxed);
        let d =
            std::env::temp_dir().join(format!("vis-quyen-chuan-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("root")).expect("tạo thư mục tạm");
        d
    }

    #[cfg(unix)]
    fn tao_link(goc: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(goc, link)
    }

    #[cfg(windows)]
    fn tao_link(goc: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_file(goc, link)
    }

    #[cfg(not(any(unix, windows)))]
    fn tao_link(_goc: &Path, _link: &Path) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "nền không hỗ trợ symlink",
        ))
    }

    /// Symlink **đích tương đối** trỏ ra ngoài root: `chuan` phải trả đích thật, không phải link.
    /// Trên Windows ca này là đường sửa chính (`canonicalize` lỗi 123); trên Unix vẫn xanh.
    #[test]
    fn symlink_dich_tuong_doi_ra_ngoai_duoc_phan_giai() {
        let d = tam("ra-ngoai");
        std::fs::write(d.join("ngoai.txt"), "x").expect("tệp ngoài");
        let link = d.join("root").join("link");
        if tao_link(Path::new("../ngoai.txt"), &link).is_err() {
            eprintln!("bỏ qua: nền không tạo được symlink");
            return;
        }
        let c = chuan(&link);
        let r = chuan(&d.join("root"));
        assert!(
            !c.starts_with(&r),
            "symlink ra ngoài root phải phân giải ra ngoài: c={} r={}",
            c.display(),
            r.display()
        );
        assert_eq!(c, chuan(&d.join("ngoai.txt")));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Symlink **dangling** đích tương đối: vẫn phải trả đích tuyệt đối (ngoài root), không trả
    /// đường dẫn của link. Trên Unix ca này đi đúng nhánh dự phòng mới (canonicalize thất bại).
    #[test]
    fn symlink_dangling_dich_tuong_doi_van_ra_ngoai() {
        let d = tam("dangling");
        let link = d.join("root").join("link-missing");
        if tao_link(Path::new("../khong-co"), &link).is_err() {
            eprintln!("bỏ qua: nền không tạo được symlink");
            return;
        }
        let c = chuan(&link);
        let r = chuan(&d.join("root"));
        assert!(
            !c.starts_with(&r),
            "symlink dangling ra ngoài root phải phân giải ra ngoài: c={} r={}",
            c.display(),
            r.display()
        );
        assert_eq!(c, chuan(&d).join("khong-co"));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// `chuan_de_quy` phải phân giải symlink ở **giữa** đường dẫn chưa tồn tại (mkdir_all).
    #[test]
    fn chuan_de_quy_symlink_giua_duong_dan_ra_ngoai() {
        let d = tam("de-quy");
        let link = d.join("root").join("link-ra-ngoai");
        if tao_link(Path::new("../ngoai"), &link).is_err() {
            eprintln!("bỏ qua: nền không tạo được symlink");
            return;
        }
        let c = chuan_de_quy(&d.join("root").join("link-ra-ngoai").join("moi").join("con"));
        let r = chuan(&d.join("root"));
        assert!(
            !c.starts_with(&r),
            "mkdir_all qua symlink ra ngoài phải bị kiểm ngoài root: c={} r={}",
            c.display(),
            r.display()
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
