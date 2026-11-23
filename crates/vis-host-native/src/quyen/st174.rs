// SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg(unix)]
//! **ST-174 (VD-176 §2)** — bằng chứng **TẤT ĐỊNH** cho nợ `#146` (TOCTOU đường đọc macOS).
//!
//! Tệp này **chỉ** được biên dịch khi test (`#[cfg(test)] mod quyen_st174;` ở `main.rs`).
//!
//! **Bối cảnh.** `Quyen::xac_nhan_sau_mo` (đường đọc của nền **không** có fd→path — macOS) làm hai
//! bước: `canonicalize` trả **chuỗi** `sau`, rồi kiểm quyền theo **tên** `sau`. Trước ST-174 bước kiểm
//! dùng `std::fs::metadata` (**đi theo symlink**): kẻ tấn công đổi `root/tep` thành symlink trỏ ra
//! ngoài **đúng khe giữa hai syscall** ⇒ `sau` vẫn là **tên trong root** nhưng metadata trả **inode
//! tệp ngoài** (trùng `(dev, ino)` của handle đã mở) ⇒ `kiem_canon` xanh ⇒ **nội dung ngoài lọt**
//! (ST-163: 4 lần / 8 100 000 vòng trên macOS ⇒ không thể chứng minh bản vá bằng ca đua).
//!
//! **Cách chứng minh (không dựa số vòng).** [`chen`](crate::quyen::chen) — **điểm chèn chỉ test** —
//! nằm đúng giữa `canonicalize` và bước kiểm, cho ca test hoán đổi `root/tep` **đúng khe** đó:
//! - [`ca_a_diem_chen_doi_sang_symlink_ra_ngoai_bi_chan_tat_dinh`] — điểm chèn đổi `root/tep` thành
//!   **symlink ra ngoài** ⇒ phải **bị chặn** (trên mã cũ: **đỏ 100/100**, đọc được `NGOAI`).
//! - [`ca_b_diem_chen_doi_symlink_ve_tep_that_doc_dung_trong_root`] — điểm chèn đổi **symlink (trong
//!   root) về tệp thật** ⇒ phải **đọc đúng** nội dung trong root (`TRONG`), không bị chặn oan.
//! - **Ca âm (mục 6)**: nếu điểm chèn **không chạy** (`chen::so_lan() == 0`, vd. bị bỏ khỏi mã) thì
//!   cả hai ca **báo “không chạy được điểm chèn”** — không thể xanh im lặng.
//!
//! **Host giả mô phỏng đúng phần macOS thiếu**: `std` an toàn không có primitive fd→path ⇒
//! `DuongDanHandle::KhongCo`; phần còn lại là **thật** (mở handle, đọc **từ handle**, `(dev, ino)` của
//! handle). Nhờ vậy cùng một ca chạy được trên **mac** và **ps** (Linux) mà vẫn đi đúng nhánh mã cần vá
//! (`xac_nhan_sau_mo`, không phải nhánh `/proc/self/fd` của Linux).

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use vis_interp::builtin::{DocQuaHandle, DuongDanHandle, FsHost};

use crate::quyen::{chen, FsGuard, Quyen, QuyenCap};

/// Số vòng của ca tất định: **100** (đúng yêu cầu ST-174). Núm `VIS_ST174_LAP` **chỉ** để chạy dài
/// (vd. 6 000 000 vòng ở mục 7 cùng task) — **không** đổi kỳ vọng (`ro_ri == 0`, `ok_trong == lap`).
fn lap() -> usize {
    std::env::var("VIS_ST174_LAP")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(100)
}

/// Thư mục tạm **duy nhất** cho mỗi ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st174-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// Quyền `cap[Dir]` = `root`, **strict-caps** bật, không `cap[Proc]`.
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

/// Thay `root/tep` bằng **symlink ra ngoài** (`ngoai`) hoặc bằng **tệp thật** (`TRONG`) — bằng
/// `rename` nên **nguyên tử** (không có trạng thái nửa vời).
fn dat_tep(root: &Path, tep: &Path, ngoai: &Path, toi_ngoai: bool) {
    let tam = root.join(".tam");
    let _ = std::fs::remove_file(&tam);
    if toi_ngoai {
        std::os::unix::fs::symlink(ngoai, &tam).expect("symlink .tam");
    } else {
        std::fs::write(&tam, "TRONG").expect("ghi .tam");
    }
    std::fs::rename(&tam, tep).expect("rename .tam → tep");
}

/// **Mô phỏng phần macOS**: mở tệp theo **tên**, đọc **từ handle**, báo `(dev, ino)` **thật** của
/// handle, và `KhongCo` cho fd→path (đúng giới hạn `std` an toàn trên macOS — `ngoai.rs`).
fn doc_qua_handle_mac(p: &str) -> Result<DocQuaHandle, String> {
    use std::io::Read as _;
    let loi = |e: std::io::Error| format!("không đọc được `{p}`: {e}");
    let mut f = std::fs::File::open(p).map_err(loi)?;
    let md = f.metadata().map_err(loi)?;
    let mut noi_dung = String::new();
    f.read_to_string(&mut noi_dung).map_err(loi)?;
    Ok(DocQuaHandle {
        noi_dung,
        danh_tinh: (md.dev(), md.ino()),
        duong_dan: DuongDanHandle::KhongCo,
    })
}

/// Host **lành**: đọc đúng tên đã cho (ca B).
struct HostMac;

impl FsHost for HostMac {
    fn read(&mut self, p: &str) -> Result<String, String> {
        Err(format!("ca ST-174 chỉ dùng read_theo_handle (p={p})"))
    }
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        doc_qua_handle_mac(p)
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

/// Host **kẻ tấn công thắng ở bước MỞ**: đổi `root/tep` thành **symlink ra ngoài** ngay trước
/// `File::open` ⇒ handle = tệp **NGOÀI** root; sau đó **trả lại tên** thành tệp thật trong root ⇒
/// `canonicalize` của bước kiểm sau thấy **tên trong root** (đúng chuỗi sự kiện của ST-163).
struct HostThangOMoRoiTraLai {
    root: PathBuf,
    tep: PathBuf,
    ngoai: PathBuf,
}

impl FsHost for HostThangOMoRoiTraLai {
    fn read(&mut self, p: &str) -> Result<String, String> {
        Err(format!("ca ST-174 chỉ dùng read_theo_handle (p={p})"))
    }
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        dat_tep(&self.root, &self.tep, &self.ngoai, true);
        let d = doc_qua_handle_mac(p)?;
        dat_tep(&self.root, &self.tep, &self.ngoai, false);
        Ok(d)
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

/// **ST-174 mục 3 (đỏ-trước) + mục 5**: điểm chèn giữa `canonicalize` và bước kiểm đổi `root/tep`
/// thành **symlink ra ngoài** ⇒ nội dung ngoài root **không được** lọt (mã cũ: đỏ **100/100**).
#[test]
fn ca_a_diem_chen_doi_sang_symlink_ra_ngoai_bi_chan_tat_dinh() {
    let n = lap();
    let d = tam("st174-a");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "NGOAI").expect("ngoai");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut host = HostThangOMoRoiTraLai {
        root: root.clone(),
        tep: tep.clone(),
        ngoai: ngoai.clone(),
    };
    let (r2, t2, n2) = (root.clone(), tep.clone(), ngoai.clone());
    chen::dat(Box::new(move |_sau: &Path, _p: &str| {
        // **ĐIỂM CHÈN**: đổi `root/tep` thành symlink ra ngoài, đúng khe giữa canonicalize và bước kiểm.
        dat_tep(&r2, &t2, &n2, true);
    }));
    let p = tep.to_str().expect("utf8").to_string();
    let (mut ro_ri, mut chan, mut chan_thay_doi, mut khac) = (0usize, 0usize, 0usize, 0usize);
    let mut mau_loi: Vec<String> = Vec::new();
    {
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        for v in 0..n {
            match g.read(&p) {
                Ok(s) if s == "NGOAI" => ro_ri += 1,
                Ok(_) => khac += 1,
                Err(e) => {
                    chan += 1;
                    if e.contains("thay đổi") {
                        chan_thay_doi += 1;
                    }
                    if mau_loi.len() < 2 && !mau_loi.contains(&e) {
                        mau_loi.push(e);
                    }
                }
            }
            assert_eq!(
                chen::so_lan(),
                v + 1,
                "không chạy được điểm chèn (vòng {v}/{n}): điểm chèn chưa chạy — ca tất định vô hiệu"
            );
            // Dựng lại trạng thái đầu mỗi vòng: `root/tep` = tệp thật `TRONG` trong root.
            dat_tep(&root, &tep, &ngoai, false);
        }
    }
    let so_lan = chen::so_lan();
    chen::go();
    eprintln!(
        "ST174 ca-A (điểm chèn đổi root/tep → symlink ra ngoài): lap={n} ro_ri={ro_ri} chan={chan} \
         chan_thay_doi={chan_thay_doi} khac={khac} diem_chen={so_lan}"
    );
    for m in &mau_loi {
        eprintln!(
            "ST174 ca-A err: {}",
            m.chars().take(170).collect::<String>()
        );
    }
    assert_eq!(
        so_lan, n,
        "không chạy được điểm chèn: {so_lan}/{n} lần — ca tất định vô hiệu (mục 6 ST-174)"
    );
    assert_eq!(
        ro_ri, 0,
        "RÒ RỈ (#146): đọc được nội dung NGOÀI root {ro_ri}/{n} lần"
    );
    assert_eq!(
        chan, n,
        "mọi vòng phải bị CHẶN: chan={chan}/{n}, khác={khac}"
    );
    assert_eq!(
        chan_thay_doi, chan,
        "lý do chặn phải là 'bị thay đổi trong khi mở' (chống kiểm-rồi-mở)"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-174 mục 4**: trong khe giữa `canonicalize` và bước kiểm, **đổi symlink (trong root) về tệp
/// thật** ⇒ kết quả **đúng đặc tả**: đọc nội dung **trong root** (`TRONG`), không bị chặn oan.
///
/// Đây là **ca đối chứng** của bản vá: cùng cơ chế điểm chèn, cùng host macOS, nhưng thay đổi trong
/// khe **không** trỏ ra ngoài root ⇒ phải xanh ở **cả** mã cũ và mã mới (chứng minh bản vá **không**
/// “chặn tất”).
#[test]
fn ca_b_diem_chen_doi_symlink_ve_tep_that_doc_dung_trong_root() {
    let n = lap();
    let d = tam("st174-b");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let that = root.join("that");
    std::fs::write(&that, "TRONG").expect("that");
    let tep = root.join("tep");
    // `root/tep` là **symlink TRONG root** ⇒ hợp lệ theo chính sách (canonicalize giải ra `root/that`).
    std::os::unix::fs::symlink("that", &tep).expect("symlink tep → that");
    let q = quyen_dir(&root);
    let mut host = HostMac;
    let t2 = tep.clone();
    chen::dat(Box::new(move |_sau: &Path, _p: &str| {
        // **ĐIỂM CHÈN**: đổi symlink **về tệp thật TRONG root** (không trỏ ra ngoài).
        std::fs::remove_file(&t2).expect("gỡ symlink tep");
        std::fs::write(&t2, "TRONG").expect("ghi tệp thật tep");
    }));
    let p = tep.to_str().expect("utf8").to_string();
    let (mut ok_trong, mut khac, mut loi) = (0usize, 0usize, 0usize);
    let mut mau_loi: Vec<String> = Vec::new();
    {
        let mut g = FsGuard {
            trong: &mut host,
            quyen: &q,
        };
        for v in 0..n {
            match g.read(&p) {
                Ok(s) if s == "TRONG" => ok_trong += 1,
                Ok(_) => khac += 1,
                Err(e) => {
                    loi += 1;
                    if mau_loi.len() < 2 && !mau_loi.contains(&e) {
                        mau_loi.push(e);
                    }
                }
            }
            assert_eq!(
                chen::so_lan(),
                v + 1,
                "không chạy được điểm chèn (vòng {v}/{n}): điểm chèn chưa chạy — ca tất định vô hiệu"
            );
            // Điểm chèn phải **thực sự** đổi symlink về tệp thật (kiểm ngay sau khi đọc).
            assert!(
                !std::fs::symlink_metadata(&tep)
                    .expect("symlink_metadata(tep)")
                    .file_type()
                    .is_symlink(),
                "điểm chèn phải đổi symlink về tệp thật (vòng {v}/{n})"
            );
            // Dựng lại symlink cho vòng kế.
            let _ = std::fs::remove_file(&tep);
            std::os::unix::fs::symlink("that", &tep).expect("dựng lại symlink tep");
        }
    }
    let so_lan = chen::so_lan();
    chen::go();
    eprintln!(
        "ST174 ca-B (điểm chèn đổi symlink → tệp thật trong root): lap={n} ok_trong={ok_trong} \
         loi={loi} khac={khac} diem_chen={so_lan}"
    );
    for m in &mau_loi {
        eprintln!(
            "ST174 ca-B err: {}",
            m.chars().take(170).collect::<String>()
        );
    }
    assert_eq!(
        so_lan, n,
        "không chạy được điểm chèn: {so_lan}/{n} lần — ca tất định vô hiệu (mục 6 ST-174)"
    );
    assert_eq!(
        ok_trong, n,
        "phải đọc ĐÚNG nội dung trong root {ok_trong}/{n} lần (loi={loi}, khác={khac})"
    );
    let _ = std::fs::remove_dir_all(&d);
}
