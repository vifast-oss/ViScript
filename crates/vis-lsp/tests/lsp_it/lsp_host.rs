// SPDX-License-Identifier: Apache-2.0
//! **ST-317c (#81/#66)** — khoá cách ly trạng thái `AnalysisHost` chạy **trong process** (các ca
//! còn lại của `lsp_it` spawn binary nên đã cách ly theo process; ca này bắt hồi quy về
//! `thread_local!` toàn cục — hai host cùng thread/cùng process phải không rò buffer/cache).

use std::path::{Path, PathBuf};

use vis_lsp::AnalysisHost;

fn thu_muc(nhan: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-lsp-host-{nhan}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn uri_cua(p: &Path) -> String {
    format!("file://{}", p.display())
}

fn thanh_chuoi(v: Vec<u8>) -> String {
    String::from_utf8(v).expect("notification UTF-8")
}

/// Hai host cùng URI nhưng buffer khác nhau: mỗi host giữ **đúng** văn bản + chẩn đoán của mình;
/// `didClose` ở host A không đóng tài liệu của host B.
#[test]
fn hai_host_cung_uri_khong_ro_buffer_va_chan_doan() {
    let d = thu_muc("buffer");
    let p = d.join("x.vis");
    std::fs::write(&p, "fn main():\n    print(1)\n").expect("ghi x.vis");
    let u = uri_cua(&p);
    let sach = "fn main():\n    print(1)\n";
    let loi = "fn main():\n    khong_ton_tai()\n";

    let mut a = AnalysisHost::new();
    let mut b = AnalysisHost::new();
    a.mo(&u, sach);
    b.mo(&u, loi);
    assert_eq!(a.van_ban(&u), Some(sach), "buffer host A");
    assert_eq!(b.van_ban(&u), Some(loi), "buffer host B");

    let ta = thanh_chuoi(a.chan_doan_tai_lieu(&u).expect("A đang mở"));
    assert!(!ta.contains("VS0301"), "A không được thấy lỗi của B: {ta}");
    let tb = thanh_chuoi(b.chan_doan_tai_lieu(&u).expect("B đang mở"));
    assert!(tb.contains("VS0301"), "B phải báo tên không tồn tại: {tb}");

    assert_eq!(a.so_tai_lieu(), 1);
    assert_eq!(b.so_tai_lieu(), 1);
    assert!(a.dong(&u).is_some(), "A đóng tài liệu của mình");
    assert_eq!(a.so_tai_lieu(), 0);
    assert_eq!(b.so_tai_lieu(), 1, "B không bị đóng theo");
    assert_eq!(b.van_ban(&u), Some(loi));
    let _ = std::fs::remove_dir_all(&d);
}

/// Hai host dựng **song song** (mỗi thread một host): không host nào thấy buffer/chẩn đoán của
/// host kia. Chạy chung process với các ca `lsp_it` khác (libtest song song) — khoá không rò
/// trạng thái toàn cục.
#[test]
fn hai_host_song_song_moi_host_giu_trang_thai_rieng() {
    let d = thu_muc("song-song");
    let p = d.join("y.vis");
    std::fs::write(&p, "fn main():\n    print(1)\n").expect("ghi y.vis");
    let u = uri_cua(&p);

    let chay = |text: &'static str| {
        let u = u.clone();
        std::thread::spawn(move || {
            let mut h = AnalysisHost::new();
            h.mo(&u, text);
            (
                h.van_ban(&u).map(str::to_string),
                thanh_chuoi(h.chan_doan_tai_lieu(&u).expect("đang mở")),
            )
        })
    };
    let a = chay("fn main():\n    print(1)\n");
    let b = chay("fn main():\n    khong_ton_tai()\n");
    let (ba, ta) = a.join().expect("thread A");
    let (bb, tb) = b.join().expect("thread B");
    assert_eq!(ba.as_deref(), Some("fn main():\n    print(1)\n"));
    assert!(!ta.contains("VS0301"), "A: {ta}");
    assert_eq!(bb.as_deref(), Some("fn main():\n    khong_ton_tai()\n"));
    assert!(tb.contains("VS0301"), "B: {tb}");
    let _ = std::fs::remove_dir_all(&d);
}
