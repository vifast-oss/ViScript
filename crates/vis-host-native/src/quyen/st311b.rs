// SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg(all(test, unix))]
//! **ST-311b (VD-286)** — ca đua 100 000 vòng cho metadata theo fd (đóng nợ `#194`).
//!
//! Trước ST-311b, các truy vấn siêu dữ liệu (`exists`/`size`/`list`/`walk`/`read_link`/`is_*`/
//! `canonicalize`) kiểm-rồi-dùng theo **tên**: `kiem_fs` (canonicalize + kiểm trong root) rồi
//! `std::fs::*` trên **đường dẫn gốc**. Kẻ tấn công đổi **thành phần giữa** (`root/sub`) thành
//! symlink trỏ ra ngoài đúng khe giữa hai syscall ⇒ `std::fs` đi theo symlink **giữa** và đọc
//! metadata của vật ngoài root (rò "tồn tại"/kích thước ngoài root).
//!
//! Sau ST-311b, mọi thao tác mở **từ fd gốc** `cap` (`openat`/`openat2` + `O_NOFOLLOW`) rồi
//! `statat`/`readlinkat`/`fstat` ⇒ thành phần giữa là symlink ⇒ `ELOOP`/`EXDEV`/`ENOTDIR` ⇒
//! `VS0604`, không rò.
//!
//! Núm `VIS_ST311B_LAP` (mặc định 100 000) **chỉ** để chạy dài hơn; **không** đổi kỳ vọng
//! (`ro == 0`, `ok_trong > 0`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use vis_core::host::FsHost;

use crate::quyen::{FsGuard, Quyen, QuyenCap};

fn lap() -> usize {
    std::env::var("VIS_ST311B_LAP")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(100_000)
}

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::AtomicUsize;
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st311b-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn quyen_dir(root: &Path) -> Quyen {
    Quyen {
        moc: vec![QuyenCap {
            pos: 0,
            kind: "Dir".to_string(),
            root: root.display().to_string(),
            write: false,
            allow: Vec::new(),
        }],
        bat: true,
        siet: true,
        goc: root.to_path_buf(),
        allow_proc: None,
    }
}

/// Host **thật** (`ho_tro_mo_duoi_goc = true`) ⇒ `FsGuard` đi đường fd; các thao tác nội dung
/// không dùng trong ca này.
struct HostThat;

impl FsHost for HostThat {
    fn ho_tro_mo_duoi_goc(&self) -> bool {
        true
    }
    fn read(&mut self, _p: &str) -> Result<String, String> {
        Err("ca ST-311b chỉ dùng metadata".to_string())
    }
    fn write(&mut self, _p: &str, _s: &str) -> Result<(), String> {
        Ok(())
    }
    fn exists(&mut self, _p: &str) -> bool {
        false
    }
    fn list(&mut self, _p: &str) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

/// Lật `root/sub` giữa **thư mục thật** (chứa `tep` = `TRONG`, 5 byte) và **symlink** trỏ `ngoai`
/// (chứa `tep` = 41 byte). `secret` **chỉ** tồn tại trong `ngoai` — dùng làm vết rò của `exists`.
fn dat_sub(root: &Path, ngoai: &Path, toi_ngoai: bool) {
    let sub = root.join("sub");
    let _ = std::fs::remove_dir_all(&sub);
    let _ = std::fs::remove_file(&sub);
    if toi_ngoai {
        std::os::unix::fs::symlink(ngoai, &sub).expect("symlink sub → ngoài");
    } else {
        std::fs::create_dir_all(&sub).expect("mkdir sub");
        std::fs::write(sub.join("tep"), "TRONG").expect("ghi trong");
    }
}

/// **Ca đua 100 000 vòng**: luồng phụ lật `root/sub` (thư mục ↔ symlink ra ngoài) liên tục, luồng
/// chính gọi `FsGuard::exists_quyen("root/sub/secret")` — `secret` **chỉ** có ngoài root. Bất biến:
/// **không lần nào** trả `Ok(true)` (vật ngoài root); và **phải có** lần `Ok(false)` (thư mục thật
/// trong root) để ca không "xanh vì chặn tất".
#[test]
fn dua_metadata_100k_khong_ro() {
    let lap = lap();
    let d = tam("dua");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&ngoai).expect("ngoai");
    std::fs::write(
        ngoai.join("tep"),
        "NGOAI-NGOAI-NGOAI-NGOAI-NGOAI-NGOAI-NGOAI",
    )
    .expect("ghi ngoài"); // 41 byte
    std::fs::write(ngoai.join("secret"), "BI-MAT").expect("secret ngoài"); // vết rò của exists
    dat_sub(&root, &ngoai, false);

    let q = quyen_dir(&root);
    let stop = Arc::new(AtomicBool::new(false));
    let st = stop.clone();
    let (r2, n2) = (root.clone(), ngoai.clone());
    let phu = std::thread::spawn(move || {
        let mut vong = 0usize;
        while !st.load(Ordering::Relaxed) {
            dat_sub(&r2, &n2, vong.is_multiple_of(2));
            vong += 1;
        }
        vong
    });

    let mut host = HostThat;
    let p = root
        .join("sub")
        .join("secret")
        .to_str()
        .expect("utf8")
        .to_string();
    let (mut ok_trong, mut ro_ri, mut chan) = (0usize, 0usize, 0usize);
    {
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        for _ in 0..lap {
            match g.exists_quyen(&p) {
                Ok(true) => ro_ri += 1,
                Ok(false) => ok_trong += 1,
                Err(_) => chan += 1,
            }
        }
    }
    let giay = std::time::Instant::now();
    stop.store(true, Ordering::Relaxed);
    let vong = phu.join().expect("luồng phụ");
    eprintln!(
        "ST311B-DUA metadata(exists): lap={lap} ok_trong={ok_trong} ro_ri={ro_ri} chan={chan} vong_doi={vong} giay={:.1}",
        giay.elapsed().as_secs_f64()
    );
    assert_eq!(
        ro_ri, 0,
        "RÒ RỈ: `exists` báo vật ngoài root tồn tại {ro_ri} lần"
    );
    assert!(
        ok_trong > 0,
        "ca vô nghĩa: không lần nào thấy thư mục thật trong root"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Đối chứng tĩnh (không đua)**: khi `root/sub` **đã là** symlink trỏ ra ngoài, `size` của
/// `root/sub/tep` phải là `VS0604` (không trả `41`). Khoá hành vi mới; không dựa số vòng.
#[test]
fn symlink_giua_ra_ngoai_size_la_vs0604() {
    let d = tam("tinh");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&ngoai).expect("ngoai");
    std::fs::write(
        ngoai.join("tep"),
        "NGOAI-NGOAI-NGOAI-NGOAI-NGOAI-NGOAI-NGOAI",
    )
    .expect("ghi ngoài");
    dat_sub(&root, &ngoai, true); // sub = symlink → ngoai

    let q = quyen_dir(&root);
    let mut host = HostThat;
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let p = root
        .join("sub")
        .join("tep")
        .to_str()
        .expect("utf8")
        .to_string();
    let e = g
        .size(&p)
        .expect_err("symlink giữa trỏ ra ngoài phải bị chặn");
    assert!(e.contains("VS0604"), "phải là VS0604: {e:?}");
    assert!(e.contains("cap[Dir]"), "phải nêu cap[Dir]: {e:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Đối chứng tĩnh**: symlink giữa trỏ ra ngoài ⇒ `exists` của vật ngoài là `VS0604`.
#[test]
fn symlink_giua_ra_ngoai_exists_la_vs0604() {
    let d = tam("tinh-exists");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&ngoai).expect("ngoai");
    std::fs::write(ngoai.join("secret"), "BI-MAT").expect("secret ngoài");
    dat_sub(&root, &ngoai, true); // sub = symlink → ngoai

    let q = quyen_dir(&root);
    let mut host = HostThat;
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let p = root
        .join("sub")
        .join("secret")
        .to_str()
        .expect("utf8")
        .to_string();
    let e = g
        .exists_quyen(&p)
        .expect_err("symlink giữa trỏ ra ngoài phải bị chặn");
    assert!(e.contains("VS0604"), "phải là VS0604: {e:?}");
    assert!(e.contains("cap[Dir]"), "phải nêu cap[Dir]: {e:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Đối chứng dương**: `root/sub/tep` là tệp thật trong root ⇒ `size` trả `5`.
#[test]
fn size_trong_root_tra_dung() {
    let d = tam("duong");
    let root = d.join("root");
    std::fs::create_dir_all(root.join("sub")).expect("root/sub");
    std::fs::write(root.join("sub").join("tep"), "TRONG").expect("ghi trong");
    let q = quyen_dir(&root);
    let mut host = HostThat;
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let p = root
        .join("sub")
        .join("tep")
        .to_str()
        .expect("utf8")
        .to_string();
    assert_eq!(g.size(&p).expect("size trong root"), 5);
    let _ = std::fs::remove_dir_all(&d);
}
