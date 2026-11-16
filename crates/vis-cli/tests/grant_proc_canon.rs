// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-123 (VD-130c)** — `--grant` cho `cap[Proc]` so **đường dẫn tuyệt đối ĐÃ CANONICALIZE**,
//! không so **basename**.
//!
//! Đỏ trước (JOURNAL/st123-red.log): tệp thực thi **cùng tên** ở thư mục khác chạy được khi chỉ
//! grant `/bin/echo` (basename khớp). Luật mới: giá trị grant **không có `/`** phân giải qua `PATH`
//! **một lần lúc khởi động** thành đường dẫn tuyệt đối; hai phía (grant và lệnh sẽ chạy) đều
//! `canonicalize` rồi so **bằng nhau**. Grant trỏ tệp không tồn tại/không chạy được ⇒ chẩn đoán rõ,
//! exit 2.
//!
//! Mỗi ca chạy **cả** interp và `--vm --no-cache`, so **byte** (stdout + stderr + exit).

use std::path::{Path, PathBuf};
use std::process::Command;

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

fn chay(d: &Path, ten: &str, args: &[&str], vm: bool) -> Kq {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d).arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    c.args(args).arg(ten);
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ghi tệp + chạy hai đường, đòi **byte y hệt** (interp vs `--vm`).
fn ca(d: &Path, ten: &str, src: &str, args: &[&str]) -> Kq {
    std::fs::write(d.join(ten), src).expect("ghi");
    let a = chay(d, ten, args, false);
    let b = chay(d, ten, args, true);
    assert_eq!(a, b, "hai đường lệch ({ten}):\n interp {a:?}\n vm {b:?}");
    a
}

/// Chương trình `.vish`: `fn main` chỉ khai `cap[Proc]` (thân rỗng), câu lệnh cấp tệp ở dưới.
fn vish(cmd: &str) -> String {
    format!(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\nfn main(p: cap[Proc]):\n    pass\n\n{cmd}\n"
    )
}

/// Tệp thực thi **cùng basename `echo`** trong thư mục tạm (chạy được trên mọi nền POSIX; in lại
/// đối số để ca không-grant chứng minh nó **chạy thật**).
///
/// **Không** dùng bản sao `cp /bin/echo` của macOS: bản sao binary hệ thống (chữ ký nền tảng) bị
/// kernel **SIGKILL (exit 137)** khi chạy — xem `JOURNAL/st123-red.log` §(1). Quyết định quyền chỉ
/// dựa trên **đường dẫn**, nên tệp cùng tên là đủ để chứng minh lỗ hổng basename.
fn dat_echo_trung_ten(d: &Path) -> PathBuf {
    let p = d.join("echo");
    std::fs::write(&p, "#!/bin/sh\necho \"$@\"\n").expect("ghi echo giả");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut m = std::fs::metadata(&p).expect("meta").permissions();
        m.set_mode(0o755);
        std::fs::set_permissions(&p, m).expect("chmod");
    }
    p
}

/// (1) Đường dẫn tuyệt đối **khớp** ⇒ chạy (không đổi so trước).
#[test]
fn tuyet_doi_khop_thi_chay() {
    let d = tam("khop");
    let r = ca(
        &d,
        "a.vish",
        &vish("/bin/echo a"),
        &["--strict-caps", "--grant", "p=/bin/echo"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (2) **Bản sao trùng tên** ở thư mục khác ⇒ `VS0604` exit 2 (đỏ trước: chạy được).
#[test]
fn sao_chep_trung_ten_bi_chan() {
    let d = tam("trung-ten");
    let gia = dat_echo_trung_ten(&d);
    let r = ca(
        &d,
        "b.vish",
        &vish(&format!("{} SAO", gia.display())),
        &["--strict-caps", "--grant", "p=/bin/echo"],
    );
    assert_eq!(r.0, 2, "{r:?}");
    let e = &r.2;
    assert!(e.contains("VS0604"), "{e}");
    assert!(e.contains("không nằm trong allowlist"), "{e}");
    assert!(e.contains("cap[Proc]"), "{e}");
    assert!(r.1.is_empty(), "không được chạy bản sao: {:?}", r.1);
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) **Symlink** tới binary được grant ⇒ chạy (canonicalize **hai phía**); grant chính symlink
/// rồi chạy đích cũng khớp (cùng đường dẫn canonicalize).
///
/// **ST-140 — vì sao đích symlink là `/bin/sh -c` chứ không phải `/bin/echo` (VD-144).** Trên runner
/// Linux `ps`, `/bin/echo` là symlink vào **coreutils multicall** (`/usr/lib/cargo/bin/coreutils/echo`);
/// binary multicall **từ chối** chạy khi `argv[0]` khác tên nó:
/// `Security violation: Requested utility 'ln-echo' does not match executable name` ⇒ `VS0501`, exit 1.
/// Ca đỏ vì **môi trường**, không phải vì quyền: sản phẩm canonicalize rồi exec **đúng đường dẫn
/// thật**, chính là điều ca này kiểm (đỏ-trước: `JOURNAL/st140-red-ps.log`).
///
/// VD-144 cho phép đổi **đích của symlink** sang binary do workspace build
/// (`env!("CARGO_BIN_EXE_<tên>")`); không có binary phù hợp thì `/bin/sh -c`. Gói `vis-cli` chỉ lộ
/// **một** binary qua `CARGO_BIN_EXE_` là `vis`, và `vis` **không** in lại đối số một cách tất định
/// (`vis` trần in usage; `vis` không có `--version`) ⇒ dùng nhánh `/bin/sh -c` của VD-144.
/// `/bin/sh` trên `ps` là `/usr/bin/dash` (binary thật, **không** multicall) nên chạy được qua symlink
/// mang tên bất kỳ; `sh -c "echo a"` in tất định `a\n` ⇒ khẳng định stdout **giữ nguyên** `[a]\n`.
/// Symlink vẫn do ca **tự tạo trong thư mục tạm**; hai chiều khẳng định và parity interp/`--vm` giữ nguyên.
#[cfg(unix)]
#[test]
fn symlink_canonicalize_hai_phia() {
    let d = tam("symlink");
    let ln = d.join("ln-sh");
    std::os::unix::fs::symlink("/bin/sh", &ln).expect("symlink");
    let r = ca(
        &d,
        "c.vish",
        &vish(&format!("{} -c \"echo a\"", ln.display())),
        &["--strict-caps", "--grant", "p=/bin/sh"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    // Chiều ngược: grant **symlink**, chạy **binary thật** ⇒ vẫn khớp sau canonicalize.
    let r2 = ca(
        &d,
        "c2.vish",
        &vish("/bin/sh -c \"echo b\""),
        &["--strict-caps", "--grant", &format!("p={}", ln.display())],
    );
    assert_eq!(r2.0, 0, "{r2:?}");
    assert_eq!(r2.1, b"[b]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (4) Grant **không có `/`** ⇒ phân giải qua `PATH` một lần lúc khởi động, rồi so **bằng** đường
/// dẫn tuyệt đối: `/bin/echo` và `/usr/bin/printf` khớp grant `echo`/`printf`; lệnh khác bị chặn.
///
/// **Không** dùng `echo` trần làm *lệnh sẽ chạy*: trong chế độ lệnh `echo` là **builtin**, không
/// spawn tiến trình ⇒ không đi qua allowlist (đúng ngữ nghĩa: quyền `cap[Proc]` chỉ áp cho tiến
/// trình ngoài). Vì vậy ca dương dùng đường dẫn tuyệt đối / `printf` trần (lệnh ngoài thật).
#[test]
fn grant_ten_tran_phan_giai_path() {
    let d = tam("path");
    let r = ca(
        &d,
        "d1.vish",
        &vish("/bin/echo a"),
        &["--strict-caps", "--grant", "p=echo"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    // `printf` **không** phải builtin ⇒ chạy thật; grant tên trần phải phân giải qua `PATH`.
    let r2 = ca(
        &d,
        "d2.vish",
        &vish("printf abc"),
        &["--strict-caps", "--grant", "p=printf"],
    );
    assert_eq!(r2.0, 0, "{r2:?}");
    assert_eq!(r2.1, b"[abc]\n");
    // Cùng đường dẫn tuyệt đối của `printf` cũng khớp grant tên trần.
    let r2b = ca(
        &d,
        "d2b.vish",
        &vish("/usr/bin/printf abc"),
        &["--strict-caps", "--grant", "p=printf"],
    );
    assert_eq!(r2b.0, 0, "{r2b:?}");
    assert_eq!(r2b.1, b"[abc]\n");
    // Lệnh ngoài **khác** grant ⇒ chặn.
    let r3 = ca(
        &d,
        "d3.vish",
        &vish("/usr/bin/printf x"),
        &["--strict-caps", "--grant", "p=/bin/cat"],
    );
    assert_eq!(r3.0, 2, "{r3:?}");
    assert!(r3.2.contains("VS0604"), "{}", r3.2);
    let _ = std::fs::remove_dir_all(&d);
}

/// (5) Grant **đường dẫn tương đối** ⇒ ghép `cwd` lúc khởi động rồi canonicalize; chạy đúng đường
/// dẫn đó thì được, chạy tệp cùng tên ở chỗ khác thì bị chặn.
#[test]
fn grant_duong_dan_tuong_doi() {
    let d = tam("tuong-doi");
    std::fs::create_dir_all(d.join("bin")).expect("bin");
    let trong = d.join("bin").join("echo");
    std::fs::write(&trong, "#!/bin/sh\necho TUONG-DOI\n").expect("ghi");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut m = std::fs::metadata(&trong).expect("meta").permissions();
        m.set_mode(0o755);
        std::fs::set_permissions(&trong, m).expect("chmod");
    }
    let r = ca(
        &d,
        "e.vish",
        &vish("./bin/echo"),
        &["--strict-caps", "--grant", "p=./bin/echo"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    // `.vish` in kết quả cấp tệp theo dạng danh sách ⇒ `[...]`.
    assert_eq!(r.1, b"[TUONG-DOI]\n");
    // Cùng tên, khác thư mục ⇒ chặn.
    let gia = dat_echo_trung_ten(&d);
    let r2 = ca(
        &d,
        "e2.vish",
        &vish(&format!("{} SAO", gia.display())),
        &["--strict-caps", "--grant", "p=./bin/echo"],
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
        &vish("/bin/echo KHONG"),
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
        &vish("/bin/echo KHONG"),
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
    let gia = dat_echo_trung_ten(&d);
    let r = ca(
        &d,
        "g.vish",
        &vish(&format!("{} TU-DO", gia.display())),
        &["--strict-caps"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[TU-DO]\n");
    let _ = std::fs::remove_dir_all(&d);
}
