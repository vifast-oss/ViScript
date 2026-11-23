// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311b (VD-286)** — ca TOCTOU/đối chứng của `quyen` (ST-163/ST-137/ST-165/ST-174).
//! Tách khỏi `mod.rs` để giữ luật R1 (< 950 dòng cho `quyen/mod.rs`); **0 đổi hành vi**.
//! Cùng tầm vực `super::*` như khi còn nằm trong `mod.rs`.

use super::*;
use vis_core::host::FsHost;

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-toctou-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
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

/// Thay `root/tep` bằng **symlink trỏ ra ngoài** (`ngoai`) hoặc bằng **tệp trong root**
/// (`TRONG`) — bằng `rename` nên **nguyên tử** (không có trạng thái nửa vời).
#[cfg(unix)]
fn dat_tep(root: &Path, tep: &Path, ngoai: &Path, toi_ngoai: bool) {
    let tam = root.join(".tam");
    let _ = std::fs::remove_file(&tam);
    if toi_ngoai {
        std::os::unix::fs::symlink(ngoai, &tam).expect("symlink");
    } else {
        std::fs::write(&tam, "TRONG").expect("ghi .tam");
    }
    std::fs::rename(&tam, tep).expect("rename");
}

/// Host giả **đổi symlink ngay bên trong `read`** — mô phỏng **tất định** khe thời gian giữa
/// bước KIỂM và bước MỞ (không phụ thuộc thời gian thật, không `sleep`).
#[cfg(unix)]
struct HostDoi {
    root: PathBuf,
    tep: PathBuf,
    ngoai: PathBuf,
}

#[cfg(unix)]
impl FsHost for HostDoi {
    fn read(&mut self, p: &str) -> Result<String, String> {
        dat_tep(&self.root, &self.tep, &self.ngoai, true);
        std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
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

/// **Ca tất định**: đường dẫn đã KIỂM (trong root) bị đổi thành symlink ra ngoài **đúng lúc**
/// mở ⇒ must be blocked; previously read outside content.
#[cfg(unix)]
#[test]
fn doi_symlink_dung_luc_mo_bi_chan() {
    let d = tam("bien");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "NGOAI").expect("ngoai");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut host = HostDoi {
        root: root.clone(),
        tep: tep.clone(),
        ngoai,
    };
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let kq = g.read(tep.to_str().expect("utf8"));
    assert!(
        kq.is_err(),
        "phải CHẶN đọc tệp ngoài root, nhưng đọc được: {kq:?}"
    );
    assert!(
        !kq.unwrap_or_default().contains("NGOAI"),
        "nội dung ngoài root đã lọt ra"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Ca đua thật**: luồng phụ đổi symlink liên tục, luồng chính đọc `VIS_TOCTOU_LAP` lần
/// (mặc định **50 000**).
/// Không phụ thuộc **thời gian tuyệt đối** (chỉ đếm vòng), **lặp có giới hạn**.
/// Bất biến: **không lần nào** trả về nội dung của tệp ngoài root; và **phải có** lần đọc
/// thành công (`TRONG`) để ca không "xanh vì chặn tất".
///
/// **ST-163 — núm chẩn đoán, KHÔNG đổi kỳ vọng**: số vòng lấy từ biến môi trường
/// `VIS_TOCTOU_LAP` khi có (mặc định **50 000** như ca gốc) — chỉ để chạy **dài** hơn nhằm bắt
/// ca đua hiếm (`1/50 000` trên macOS, nợ `#146`). Biến này **không** đổi ngưỡng, **không** đổi
/// `assert_eq!(ro_ri, 0, …)`, **không** `#[ignore]`/cfg-bỏ. Khi có bất thường, ca in ra **vòng
/// thứ mấy**, **nội dung thật đọc được** (che 40 ký tự), **độ dài** ký tự/byte và mẫu `err`;
/// đặt `VIS_TOCTOU_TRACE=1` để in thêm **vết** đường đi của chính lần đọc đó.
#[cfg(unix)]
#[test]
fn dua_doi_symlink_lap_co_gioi_han() {
    let lap: usize = std::env::var("VIS_TOCTOU_LAP")
        .ok()
        .and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(50_000);
    let d = tam("dua");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "NGOAI").expect("ngoai");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut runner = crate::ngoai::Runner::default();
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let st = stop.clone();
    let (r2, t2, n2) = (root.clone(), tep.clone(), ngoai.clone());
    let phu = std::thread::spawn(move || {
        let mut vong = 0usize;
        while !st.load(std::sync::atomic::Ordering::Relaxed) {
            dat_tep(&r2, &t2, &n2, vong.is_multiple_of(2));
            vong += 1;
        }
        vong
    });
    let (mut ok_trong, mut ro_ri, mut chan) = (0usize, 0usize, 0usize);
    let mut ca_ro_ri: Vec<String> = Vec::new();
    let mut mau_loi: Vec<String> = Vec::new();
    let p = tep.to_str().expect("utf8");
    let dong_ho = std::time::Instant::now();
    {
        let mut g = FsGuard {
            trong: &mut runner,
            quyen: &q,
        };
        for vong in 0..lap {
            match g.read(p) {
                Ok(s) if s == "TRONG" => ok_trong += 1,
                Ok(s) => {
                    ro_ri += 1;
                    let vet = vet::lay();
                    if ca_ro_ri.len() < 8 {
                        let che: String = s.chars().take(40).collect();
                        ca_ro_ri.push(format!(
                            "RO_RI vong={vong} dai_byte={} dai_ky_tu={} noi_dung={:?} vet={}",
                            s.len(),
                            s.chars().count(),
                            che,
                            vet.as_deref()
                                .unwrap_or("(tắt — đặt VIS_TOCTOU_TRACE=1 để in vết)")
                        ));
                    }
                }
                Err(e) => {
                    chan += 1;
                    let gon: String = e.chars().take(90).collect();
                    if mau_loi.len() < 4 && !mau_loi.contains(&gon) {
                        mau_loi.push(gon);
                    }
                }
            }
        }
    }
    let giay = dong_ho.elapsed().as_secs_f64();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let vong = phu.join().expect("luồng phụ");
    eprintln!(
            "TOCTOU dua: lap={lap} ok_trong={ok_trong} ro_ri={ro_ri} chan={chan} vong_doi={vong} giay={giay:.1}"
        );
    for dong in &ca_ro_ri {
        eprintln!("{dong}");
    }
    for gon in &mau_loi {
        eprintln!("TOCTOU dua err: {gon}");
    }
    assert_eq!(ro_ri, 0, "RÒ RỈ: đọc được nội dung ngoài root {ro_ri} lần");
    assert!(
        ok_trong > 0,
        "ca vô nghĩa: không lần nào đọc được tệp trong root"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-137 (VD-141)** — host mô phỏng **kẻ tấn công thắng ở bước MỞ**: đổi `root/tep` thành
/// **symlink ra ngoài** ngay trước `File::open` ⇒ handle = tệp NGOÀI root. Đường dẫn thật của
/// handle (`/proc/self/fd`) phải chỉ ra ngoài root ⇒ tầng quyền **từ chối**; không được trả
/// nội dung `NGOAI`. Đây là ca **tất định** cho đúng trật tự đã lọt trước ST-137.
#[cfg(target_os = "linux")]
struct HostThangOMo {
    root: PathBuf,
    tep: PathBuf,
    ngoai: PathBuf,
}

#[cfg(target_os = "linux")]
impl FsHost for HostThangOMo {
    fn read(&mut self, p: &str) -> Result<String, String> {
        std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
    }
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        dat_tep(&self.root, &self.tep, &self.ngoai, true);
        crate::ngoai::doc_theo_handle(p, std::path::Path::new("/proc/self/fd"))
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

#[cfg(target_os = "linux")]
#[test]
fn handle_ngoai_root_bi_chan_tat_dinh() {
    let d = tam("st137-thang");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "NGOAI").expect("ngoai");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut host = HostThangOMo {
        root: root.clone(),
        tep: tep.clone(),
        ngoai,
    };
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let kq = g.read(tep.to_str().expect("utf8"));
    match &kq {
        Err(e) => {
            assert!(e.contains("VS0604"), "phải là lỗi quyền VS0604: {e}");
            assert!(
                e.contains("ĐÃ MỞ") || e.contains("đã mở"),
                "thông điệp phải nêu tệp ĐÃ MỞ: {e}"
            );
        }
        Ok(s) => panic!("đọc được tệp ngoài root (handle ra ngoài): {s:?}"),
    }
    assert!(
        !kq.unwrap_or_default().contains("NGOAI"),
        "nội dung ngoài root đã lọt ra"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-137 (VD-141)** — host đổi tên **sau khi đã mở+đọc**: nội dung phải là nội dung **của
/// handle** (`TRONG`), quyết định quyền theo **đường dẫn thật của handle**, không theo tên đã
/// đổi (tên lúc này trỏ ra ngoài). Trước ST-137 ca này trả `VS0604` oan (hoặc lọt `NGOAI`).
#[cfg(target_os = "linux")]
struct HostDoiSauKhiMo {
    root: PathBuf,
    tep: PathBuf,
    ngoai: PathBuf,
}

#[cfg(target_os = "linux")]
impl FsHost for HostDoiSauKhiMo {
    fn read(&mut self, p: &str) -> Result<String, String> {
        std::fs::read_to_string(p).map_err(|e| format!("không đọc được `{p}`: {e}"))
    }
    fn read_theo_handle(&mut self, p: &str) -> Result<DocQuaHandle, String> {
        let d = crate::ngoai::doc_theo_handle(p, std::path::Path::new("/proc/self/fd"))?;
        dat_tep(&self.root, &self.tep, &self.ngoai, true);
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

#[cfg(target_os = "linux")]
#[test]
fn doc_lay_tu_handle_khi_ten_da_bi_doi() {
    let d = tam("st137-sau");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "NGOAI").expect("ngoai");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut host = HostDoiSauKhiMo {
        root: root.clone(),
        tep: tep.clone(),
        ngoai,
    };
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let kq = g.read(tep.to_str().expect("utf8"));
    assert_eq!(
        kq.as_deref(),
        Ok("TRONG"),
        "phải trả nội dung CỦA HANDLE (TRONG), không theo tên đã đổi"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-137 (VD-141)** — host **thật** nhưng không lấy được đường dẫn thật của handle
/// (`/proc` lỗi/không mount) ⇒ tầng quyền **từ chối** `VS0604`, **không** quay về kiểm theo tên.
/// Host giả ở đây trả đúng thứ tự: trước ST-137 nó đã đọc được `NGOAI` rồi; nay phải bị chặn.
struct HostKhongProc;

impl FsHost for HostKhongProc {
    fn read(&mut self, _p: &str) -> Result<String, String> {
        Ok("NGOAI".to_string())
    }
    fn read_theo_handle(&mut self, _p: &str) -> Result<DocQuaHandle, String> {
        Ok(DocQuaHandle {
            noi_dung: "NGOAI".to_string(),
            danh_tinh: (7, 7),
            duong_dan: DuongDanHandle::KhongLayDuoc(
                "không đọc được `/proc/self/fd/3`: No such file or directory".to_string(),
            ),
        })
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

#[test]
fn proc_khong_doc_duoc_thi_tu_choi() {
    let d = tam("st137-proc");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut host = HostKhongProc;
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let kq = g.read(tep.to_str().expect("utf8"));
    let e = kq.expect_err("phải TỪ CHỐI khi không đọc được /proc/self/fd");
    assert!(e.contains("VS0604"), "phải là VS0604: {e}");
    assert!(e.contains("/proc"), "thông điệp phải nêu /proc: {e}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-137** — kiểm **thẳng** host thật: `/proc` không có (tham số `goc_proc` trỏ thư mục
/// không tồn tại) ⇒ `KhongLayDuoc`; `/proc` thật ⇒ `Co(đường dẫn trong root)`.
#[cfg(target_os = "linux")]
#[test]
fn doc_theo_handle_bao_dung_duong_dan_cua_proc() {
    let d = tam("st137-doc");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let root_c = std::fs::canonicalize(&root).expect("canon root");
    let p = tep.to_str().expect("utf8");
    // /proc "không đọc được": trỏ vào một thư mục không tồn tại (không cần unmount procfs).
    let khong_proc = d.join("khong-co-proc");
    let d1 = crate::ngoai::doc_theo_handle(p, &khong_proc).expect("mở được tệp");
    assert!(
        matches!(&d1.duong_dan, DuongDanHandle::KhongLayDuoc(m) if m.contains("khong-co-proc")),
        "phải báo KhongLayDuoc khi không đọc được /proc: {:?}",
        d1.duong_dan
    );
    // /proc thật: đường dẫn handle phải nằm trong root và là tệp đã mở.
    let d2 = crate::ngoai::doc_theo_handle(p, std::path::Path::new("/proc/self/fd"))
        .expect("mở được tệp");
    match &d2.duong_dan {
        DuongDanHandle::Co(dd) => {
            assert!(
                dd.starts_with(&root_c),
                "đường dẫn handle phải trong root: {}",
                dd.display()
            );
        }
        khac => panic!("phải là Co(đường dẫn handle), gặp {khac:?}"),
    }
    assert_eq!(d2.noi_dung, "TRONG");
    assert!(d2.danh_tinh.0 != 0, "phải báo danh tính (dev, ino)");
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-165 (nợ #138)** — từ chối vì tên đã cho **phân giải ra đích khác** (vd
/// `/proc/self/fd/0` ⇒ `/dev/null`) thì thông điệp phải nêu **đích thật**: nếu chỉ nói
/// "thiếu `cap[Dir]` cho `/proc/self/fd/0`", người khai `cap[Dir]` cho `/proc` tưởng cap của
/// mình sai. Đỏ-trước: thông điệp cũ không có đích đã phân giải.
#[cfg(unix)]
#[test]
fn thong_diep_tu_choi_neu_dich_that_khac_ten() {
    let d = tam("st165-dich");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let ngoai = d.join("ngoai.txt");
    std::fs::write(&ngoai, "NGOAI").expect("ngoai");
    let link = root.join("link");
    std::os::unix::fs::symlink(&ngoai, &link).expect("symlink");
    let q = quyen_dir(&root);
    let e = q
        .kiem_fs(link.to_str().expect("utf8"), false)
        .expect_err("phải TỪ CHỐI đích ngoài root");
    assert!(e.contains("VS0604"), "phải là VS0604: {e}");
    assert!(
        e.contains("phân giải tới"),
        "thông điệp phải nói tên đã phân giải: {e}"
    );
    let dich = std::fs::canonicalize(&ngoai).expect("canon đích");
    assert!(
        e.contains(&dich.display().to_string()),
        "thông điệp phải nêu đích thật {}: {e}",
        dich.display()
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-165 (nợ #138)** — `/proc/self/fd/1` trỏ tới `pipe:[…]`: vật đã mở **không có đường
/// dẫn** ⇒ phải nói rõ "không phải đường dẫn", **không** báo nhầm "thiếu `cap[Dir]`".
/// (Trước ST-165: rơi vào nhánh "ngoài root" ⇒ thông điệp sai chẩn đoán.)
#[test]
fn doi_tuong_khong_phai_duong_dan_bao_ro_ly_do() {
    let d = tam("st165-pipe");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let q = quyen_dir(&root);
    let e = q
        .kiem_canon(Path::new("pipe:[123]"), "/proc/self/fd/1", false)
        .expect_err("phải TỪ CHỐI đối tượng không phải đường dẫn");
    assert!(e.contains("VS0604"), "phải là VS0604: {e}");
    assert!(
        e.contains("không phải đường dẫn"),
        "phải nói rõ vật không có đường dẫn: {e}"
    );
    assert!(e.contains("pipe:[123]"), "phải nêu đích: {e}");
    assert!(
        !e.contains("thiếu `cap[Dir]`"),
        "không được báo nhầm thiếu cap[Dir]: {e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-165 (nợ #138)** — `/proc` **không đọc được** (host thật) thì người vận hành phải biết
/// **cần procfs** (không chỉ thấy "từ chối"): thông điệp phải nêu `procfs` + `/proc/self/fd`.
#[test]
fn thong_diep_khong_doc_duoc_proc_co_huong_dan_van_hanh() {
    let d = tam("st165-khong-proc");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    let tep = root.join("tep");
    std::fs::write(&tep, "TRONG").expect("tep");
    let q = quyen_dir(&root);
    let mut host = HostKhongProc;
    let mut g = FsGuard {
        trong: &mut host,
        quyen: &q,
    };
    let e = g
        .read(tep.to_str().expect("utf8"))
        .expect_err("phải TỪ CHỐI khi không đọc được /proc/self/fd");
    assert!(e.contains("VS0604"), "phải là VS0604: {e}");
    assert!(
        e.contains("procfs") && e.contains("/proc/self/fd"),
        "phải hướng dẫn kiểm/mount procfs: {e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-165 (nợ #138)** — khi `/proc` **đọc được** (Linux thật), đọc `/proc/self/status` dưới
/// `cap[Dir] /proc` **không** bị `VS0604`; bước kiểm theo tên + quyết định trên handle
/// (`/proc/<pid>/status`) đều nằm trong root. Đây là ca "xanh-sau" cho đúng yêu cầu của nợ:
/// `/proc` đọc được ⇒ đọc được thật.
#[cfg(target_os = "linux")]
#[test]
fn doc_proc_status_khi_proc_doc_duoc_khong_bi_vs0604() {
    let q = quyen_dir(Path::new("/proc"));
    let c = q
        .kiem_fs_canon("/proc/self/status", false)
        .expect("phải CHO đọc /proc/self/status khi cap[Dir] /proc");
    let c = c.expect("quyền bật ⇒ phải trả đường dẫn đã canonicalize");
    assert!(
        c.starts_with("/proc"),
        "đường dẫn canonicalize phải trong /proc: {}",
        c.display()
    );
}
