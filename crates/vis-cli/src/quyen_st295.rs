// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-295 (VD-276 §1)** — ca **tất định** cho khe TOCTOU đường ghi (`#109b`/`#140`).
//!
//! Không dựa vào may mắn thời gian: [`chen`] là **điểm chèn chỉ test** nằm đúng khe "đã kiểm
//! quyền, chưa mở". Hook của ca hoán đổi `root/tep` thành **symlink ra ngoài** ngay tại đó.
//!
//! - **Nhánh cũ** (`ho_tro_mo_duoi_goc = false`: kiểm theo tên rồi `std::fs::write`): ghi đi theo
//!   symlink ⇒ tệp ngoài bị sửa ⇒ **ĐỎ 100/100**.
//! - **Nhánh mới** (`openat`/`openat2` + `O_NOFOLLOW`/`RESOLVE_NO_SYMLINKS`): mở gặp symlink ⇒
//!   `ELOOP`/`EXDEV`, tệp ngoài **không đổi** ⇒ **0/100**.
//!
//! Cùng một hook, cùng một thao tác `write`, chỉ khác đường mở ⇒ chứng minh bản vá đóng đúng khe.

#![cfg(all(test, unix))]

use std::cell::{Cell, RefCell};
use std::path::Path;

/// Điểm chèn **chỉ test**, đặt giữa bước kiểm quyền và bước mở/ghi thật.
pub(crate) mod chen {
    use super::*;

    type Hook = Box<dyn FnMut(&Path)>;

    thread_local! {
        static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
        static DEM: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn dat(h: Hook) {
        DEM.with(|d| d.set(0));
        HOOK.with(|c| *c.borrow_mut() = Some(h));
    }

    pub(crate) fn go() {
        HOOK.with(|c| *c.borrow_mut() = None);
        DEM.with(|d| d.set(0));
    }

    pub(crate) fn so_lan() -> usize {
        DEM.with(Cell::get)
    }

    /// No-op khi chưa đăng ký ⇒ không đổi hành vi bất kỳ ca nào khác.
    pub(crate) fn chay(p: &Path) {
        if HOOK.with(|c| c.borrow().is_none()) {
            return;
        }
        DEM.with(|d| d.set(d.get() + 1));
        if let Some(mut h) = HOOK.with(|c| c.borrow_mut().take()) {
            h(p);
            HOOK.with(|c| *c.borrow_mut() = Some(h));
        }
    }
}

use std::path::PathBuf;

use vis_interp::builtin::FsHost;

use crate::quyen::{FsGuard, Quyen, QuyenCap};

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st295-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn quyen_ghi(root: &Path) -> Quyen {
    Quyen {
        moc: vec![QuyenCap {
            pos: 0,
            kind: "Dir".to_string(),
            root: root.display().to_string(),
            write: true,
            allow: Vec::new(),
        }],
        bat: true,
        siet: true,
        goc: root.to_path_buf(),
        allow_proc: None,
    }
}

/// Host thật: `an_toan` quyết định FsGuard dùng `openat` hay đường cũ.
struct HostThat {
    an_toan: bool,
}

impl FsHost for HostThat {
    fn ho_tro_mo_duoi_goc(&self) -> bool {
        self.an_toan
    }
    fn read(&mut self, _p: &str) -> Result<String, String> {
        Err("ca ST-295 chỉ dùng write".to_string())
    }
    fn write(&mut self, p: &str, s: &str) -> Result<(), String> {
        std::fs::write(p, s).map_err(|e| format!("không ghi được `{p}`: {e}"))
    }
    fn exists(&mut self, _p: &str) -> bool {
        false
    }
    fn list(&mut self, _p: &str) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
}

/// Hoán đổi **nguyên tử** `root/tep` thành symlink trỏ `ngoai` (và ngược lại về tệp thật).
fn doi_sang_link(root: &Path, tep: &Path, ngoai: &Path) {
    let tam = root.join(".tam-st295");
    let _ = std::fs::remove_file(&tam);
    std::os::unix::fs::symlink(ngoai, &tam).expect("symlink tạm");
    std::fs::rename(&tam, tep).expect("rename tạm → tep");
}

fn dat_lai_tep(_root: &Path, tep: &Path) {
    let _ = std::fs::remove_file(tep);
    std::fs::write(tep, "TRONG").expect("ghi tệp trong");
}

/// Một lượt: trả `true` nếu nội dung ngoài root bị ghi đè (rò).
fn mot_luot(root: &Path, ngoai: &Path, an_toan: bool, lap: usize) -> (usize, usize) {
    let tep = root.join("tep");
    let mut ro = 0usize;
    let mut dem = 0usize;
    for _ in 0..lap {
        dat_lai_tep(root, &tep);
        std::fs::write(ngoai, "NGOAI").expect("ghi ngoài");
        let q = quyen_ghi(root);
        let mut host = HostThat { an_toan };
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        let r2 = root.to_path_buf();
        let t2 = tep.clone();
        let n2 = ngoai.to_path_buf();
        chen::dat(Box::new(move |_p: &Path| {
            doi_sang_link(&r2, &t2, &n2);
        }));
        let _ = g.write(tep.to_str().unwrap(), "GHI-DE");
        dem += chen::so_lan();
        chen::go();
        if std::fs::read_to_string(ngoai).unwrap_or_default() == "GHI-DE" {
            ro += 1;
        }
    }
    (ro, dem)
}

/// **Đỏ-trước**: nhánh cũ (kiểm-theo-tên → `std::fs::write`) bị hoán đổi symlink ở đúng khe ⇒
/// cả **100/100** lượt ghi ra ngoài.
#[test]
fn cu_100_tren_100_ro_ra_ngoai() {
    let d = tam("cu");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    let (ro, dem) = mot_luot(&root, &ngoai, false, 100);
    println!("ST295-DUA cu: ro={ro}/100 dem={dem}/100 (nhanh kiem-roi-mo cu)");
    assert_eq!(dem, 100, "điểm chèn phải chạy 100 lần");
    assert_eq!(ro, 100, "nhánh cũ phải rò 100/100 (thực tế {ro})");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Sau vá**: nhánh `openat`/`openat2` + `O_NOFOLLOW` gặp symlink ⇒ từ chối; **0/100** lượt rò.
#[test]
fn moi_0_tren_100_ro_ra_ngoai() {
    let d = tam("moi");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    let (ro, dem) = mot_luot(&root, &ngoai, true, 100);
    println!("ST295-DUA moi: ro={ro}/100 dem={dem}/100 (openat/O_NOFOLLOW)");
    assert_eq!(dem, 100, "điểm chèn phải chạy 100 lần");
    assert_eq!(ro, 0, "nhánh mới không được rò (thực tế {ro})");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Đối chứng**: không hoán đổi ⇒ cả hai nhánh ghi đúng tệp trong root.
#[test]
fn doi_chung_khong_hoan_doi_thi_ghi_dung_trong_root() {
    for an_toan in [false, true] {
        let d = tam("doi-chung");
        let root = d.join("root");
        std::fs::create_dir_all(&root).expect("root");
        let tep = root.join("tep");
        dat_lai_tep(&root, &tep);
        let q = quyen_ghi(&root);
        let mut host = HostThat { an_toan };
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        g.write(tep.to_str().unwrap(), "GHI-DE")
            .expect("ghi trong root");
        assert_eq!(std::fs::read_to_string(&tep).unwrap(), "GHI-DE");
        let _ = std::fs::remove_dir_all(&d);
    }
}
