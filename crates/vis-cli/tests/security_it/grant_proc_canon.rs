// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-123 (VD-130c)** — `--grant` cho `cap[Proc]` so **đường dẫn tuyệt đối ĐÃ CANONICALIZE**,
//! không so **basename**.
//!
//! Luật: giá trị grant **không có `/`** phân giải qua `PATH` **một lần lúc khởi động** thành đường
//! dẫn tuyệt đối; hai phía (grant và lệnh sẽ chạy) đều `canonicalize` rồi so **bằng nhau**.
//! Grant trỏ tệp không tồn tại ⇒ chẩn đoán rõ, exit 2. Mọi binary giả trong file này là **bản sao
//! của `test-child`** — xuyên nền (POSIX + Windows), tất định; không dùng shebang/shell ngoài.
//!
//! Mỗi ca chạy **cả** interp và `--vm --no-cache`, so **byte** (stdout + stderr + exit).

use std::path::{Path, PathBuf};
use std::process::Command;

const TC: &str = env!("CARGO_BIN_EXE_test-child");

type Kq = (i32, Vec<u8>, String);

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-grant-canon-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn duong_dan_voi_path(path_them: Option<&Path>) -> std::ffi::OsString {
    let mut muc: Vec<PathBuf> = Vec::new();
    if let Some(p) = path_them {
        muc.push(p.to_path_buf());
    }
    let cur = std::env::var_os("PATH").unwrap_or_default();
    for p in std::env::split_paths(&cur) {
        muc.push(p);
    }
    std::env::join_paths(muc).expect("join PATH")
}

fn chay(d: &Path, ten: &str, args: &[&str], vm: bool, path_them: Option<&Path>) -> Kq {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d).arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    c.args(args).arg(ten);
    c.env("PATH", duong_dan_voi_path(path_them));
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ghi tệp + chạy hai đường, đòi **byte y hệt** (interp vs `--vm`).
fn ca(d: &Path, ten: &str, src: &str, args: &[&str]) -> Kq {
    ca_path(d, ten, src, args, None)
}

fn ca_path(d: &Path, ten: &str, src: &str, args: &[&str], path_them: Option<&Path>) -> Kq {
    std::fs::write(d.join(ten), src).expect("ghi");
    let a = chay(d, ten, args, false, path_them);
    let b = chay(d, ten, args, true, path_them);
    assert_eq!(a, b, "hai đường lệch ({ten}):\n interp {a:?}\n vm {b:?}");
    a
}

/// Chương trình `.vish`: `fn main` chỉ khai `cap[Proc]` (thân rỗng), câu lệnh cấp tệp ở dưới.
fn vish(cmd: &str) -> String {
    format!(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\nfn main(p: cap[Proc]):\n    pass\n\n{cmd}\n"
    )
}

fn ten_file_echo() -> &'static str {
    if cfg!(windows) {
        "echo.exe"
    } else {
        "echo"
    }
}

/// Binary giả tên `echo` = **bản sao `test-child`** (`std::fs::copy` giữ bit thực thi trên Unix;
/// trên Windows `.exe` luôn chạy được). Chạy dưới dạng `<fake> echo TU-DO` in tất định `TU-DO`.
fn dat_echo_trung_ten(d: &Path, thu_muc: Option<&str>) -> PathBuf {
    let muc = thu_muc
        .map(|m| d.join(m))
        .unwrap_or_else(|| d.to_path_buf());
    std::fs::create_dir_all(&muc).expect("thư mục chứa echo giả");
    let p = muc.join(ten_file_echo());
    std::fs::copy(TC, &p).expect("copy test-child làm echo giả");
    p
}

/// (1) Đường dẫn tuyệt đối **khớp** (sau canonicalize) ⇒ chạy.
#[test]
fn tuyet_doi_khop_thi_chay() {
    let d = tam("khop");
    let g = format!("p={TC}");
    let r = ca(
        &d,
        "a.vish",
        &vish(&format!("{TC} echo a")),
        &["--strict-caps", "--grant", g.as_str()],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (2) **Bản sao trùng tên** ở thư mục khác ⇒ `VS0604` exit 2 (grant trỏ test-child, chạy bản sao).
#[test]
fn sao_chep_trung_ten_bi_chan() {
    let d = tam("trung-ten");
    let gia = dat_echo_trung_ten(&d, None);
    let g = format!("p={TC}");
    let r = ca(
        &d,
        "b.vish",
        &vish(&format!("{} echo SAO", gia.display())),
        &["--strict-caps", "--grant", g.as_str()],
    );
    assert_eq!(r.0, 2, "{r:?}");
    let e = &r.2;
    assert!(e.contains("VS0604"), "{e}");
    assert!(e.contains("không nằm trong allowlist"), "{e}");
    assert!(e.contains("cap[Proc]"), "{e}");
    assert!(r.1.is_empty(), "không được chạy bản sao: {:?}", r.1);
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) **Symlink** tới binary được grant ⇒ chạy (canonicalize **hai phía**).
#[cfg(unix)]
#[test]
fn symlink_canonicalize_hai_phia() {
    let d = tam("symlink");
    let ln = d.join("ln-tc");
    std::os::unix::fs::symlink(TC, &ln).expect("symlink");
    let g = format!("p={TC}");
    let r = ca(
        &d,
        "c.vish",
        &vish(&format!("{} echo a", ln.display())),
        &["--strict-caps", "--grant", g.as_str()],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    // Chiều ngược: grant **symlink**, chạy **binary thật** ⇒ vẫn khớp sau canonicalize.
    let g2 = format!("p={}", ln.display());
    let r2 = ca(
        &d,
        "c2.vish",
        &vish(&format!("{TC} echo b")),
        &["--strict-caps", "--grant", g2.as_str()],
    );
    assert_eq!(r2.0, 0, "{r2:?}");
    assert_eq!(r2.1, b"[b]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (4) Grant **không có `/`** ⇒ phân giải qua `PATH` một lần lúc khởi động, rồi so **bằng** đường
/// dẫn tuyệt đối. Tạo `PATH` giả trong thư mục tạm: `tc-echo` (bản sao test-child) — hermetic,
/// không phụ thuộc `/usr/bin/echo` của nền.
#[test]
fn grant_ten_tran_phan_giai_path() {
    let d = tam("path");
    let bin = d.join("bindir");
    let gia = dat_echo_trung_ten(&d, Some("bindir"));
    let ten_gia = gia.file_name().unwrap().to_str().unwrap().to_string();
    let g1 = format!("p={ten_gia}");
    let r = ca_path(
        &d,
        "d1.vish",
        &vish(&format!("{tc_echo} echo a", tc_echo = gia.display())),
        &["--strict-caps", "--grant", g1.as_str()],
        Some(&bin),
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    // test-child thật **không** khớp grant tên trần (phân giải ra bản sao trong PATH) ⇒ chặn.
    let g2 = format!("p={ten_gia}");
    let r2 = ca_path(
        &d,
        "d2.vish",
        &vish(&format!("{TC} echo abc")),
        &["--strict-caps", "--grant", g2.as_str()],
        Some(&bin),
    );
    assert_eq!(r2.0, 2, "{r2:?}");
    assert!(r2.2.contains("VS0604"), "{}", r2.2);
    // Tên khác grant (test-child trần trong PATH giả cũng là `ten_gia` — không có) ⇒ chặn.
    let r3 = ca_path(
        &d,
        "d3.vish",
        &vish(&format!("{tc_echo} echo x", tc_echo = gia.display())),
        &["--strict-caps", "--grant", "p=khong_co_lenh_nay"],
        Some(&bin),
    );
    // Grant trỏ lệnh không có trong PATH ⇒ chẩn đoán rõ, exit 2 (kiểm cả nhánh chẩn đoán).
    assert_eq!(r3.0, 2, "{r3:?}");
    assert!(
        r3.2.contains("không thấy lệnh") && r3.2.contains("PATH"),
        "{}",
        r3.2
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (5) Grant **đường dẫn tương đối** ⇒ ghép `cwd` lúc khởi động rồi canonicalize; chạy đúng đường
/// dẫn đó thì được, chạy tệp cùng tên ở chỗ khác thì bị chặn.
#[test]
fn grant_duong_dan_tuong_doi() {
    let d = tam("tuong-doi");
    let _trong = dat_echo_trung_ten(&d, Some("bin"));
    let cwd_rel = Path::new("bin").join(ten_file_echo());
    // Lệnh .vish dùng `./bin/echo echo TUONG-DOI` (có `/` ⇒ không đi qua allowlist PATH mà qua
    // canonicalize từ cwd).
    let cmd = format!(
        "./{} echo TUONG-DOI",
        cwd_rel.display().to_string().replace('\\', "/")
    );
    let g = format!("p=./{}", cwd_rel.display());
    let r = ca(
        &d,
        "e.vish",
        &vish(&cmd),
        &["--strict-caps", "--grant", g.as_str()],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[TUONG-DOI]\n");
    // Cùng tên, khác thư mục ⇒ chặn.
    let gia = dat_echo_trung_ten(&d, None);
    let g2 = format!("p=./{}", cwd_rel.display());
    let r2 = ca(
        &d,
        "e2.vish",
        &vish(&format!("{} echo KHAC", gia.display())),
        &["--strict-caps", "--grant", g2.as_str()],
    );
    assert_eq!(r2.0, 2, "{r2:?}");
    assert!(r2.2.contains("VS0604"), "{}", r2.2);
    let _ = std::fs::remove_dir_all(&d);
}

/// (6) Grant trỏ binary **không tồn tại lúc khởi động** ⇒ chẩn đoán rõ, exit 2, **không** chạy gì.
#[test]
fn grant_binary_khong_ton_tai() {
    let d = tam("khong-co");
    let r = ca(
        &d,
        "f.vish",
        &vish(&format!("{TC} echo KHONG")),
        &["--strict-caps", "--grant", "p=/khong/co/lenh"],
    );
    assert_eq!(r.0, 2, "{r:?}");
    assert!(r.1.is_empty(), "không được chạy: {:?}", r.1);
    assert!(
        r.2.contains("--grant p=/khong/co/lenh") && r.2.contains("không phân giải được"),
        "{}",
        r.2
    );
    // Tên trần không có trong `PATH` ⇒ chẩn đoán khác, cùng exit 2.
    let r2 = ca(
        &d,
        "f2.vish",
        &vish(&format!("{TC} echo KHONG")),
        &["--strict-caps", "--grant", "p=khong_co_lenh_nay_trong_path"],
    );
    assert_eq!(r2.0, 2, "{r2:?}");
    assert!(
        r2.2.contains("không thấy lệnh") && r2.2.contains("PATH"),
        "{}",
        r2.2
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (7) **Không** grant ⇒ **không** giới hạn (hành vi cũ giữ nguyên, không siết oan).
#[test]
fn khong_grant_khong_gioi_han() {
    let d = tam("khong-grant");
    let gia = dat_echo_trung_ten(&d, None);
    let r = ca(
        &d,
        "g.vish",
        &vish(&format!("{} echo TU-DO", gia.display())),
        &["--strict-caps"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[TU-DO]\n");
    let _ = std::fs::remove_dir_all(&d);
}
