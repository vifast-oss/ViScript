// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — `std.fs.copy` và `std.fs.rename` (tệp → tệp).
//!
//! Hợp đồng (đóng băng bằng test):
//! - **chỉ tệp thường**: nguồn/đích là **thư mục** hoặc **symlink** ⇒ `Err(Error)` dữ liệu,
//!   **không** đi theo symlink, **không** trap;
//! - `copy` **từ chối đích đã tồn tại** (kể cả symlink/thư mục) — không ghi đè, không mất dữ liệu;
//!   nội dung chép **từng byte** (không phụ thuộc UTF-8), quyền của tệp mới do umask quyết định;
//! - `rename` **cùng hệ tệp**, cũng **từ chối đích đã tồn tại** (không dựa vào ngữ nghĩa ghi đè
//!   của Unix), do đó không bao giờ thay nội dung đích một cách ngầm;
//! - quyền: `copy` cần **đọc** nguồn + **ghi** đích; `rename` cần **ghi** cả hai (thư mục cha của
//!   đích được canonicalize khi đích chưa tồn tại) ⇒ thiếu là `VS0604` (exit 2);
//! - host thật (`ngoai.rs`), `--fs-mock`, interp và VM dùng **cùng** một ngữ nghĩa.
//!
//! RED-BEFORE (ghi lúc làm VD-269): trước khi thêm hai hàm, chương trình báo lỗi tĩnh «không có
//! hàm `copy`/`rename`» (exit 2) — xem báo cáo bàn giao.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd269-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

fn vis(args: &[&std::ffi::OsStr]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    for a in args {
        c.arg(a);
    }
    let o = c.output().expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn viet_tep(p: &Path, src: &str) {
    std::fs::write(
        p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
}

/// Chạy **cả hai đường** (interp + `--vm --no-cache`) trên **cùng** tệp chương trình, đòi cùng
/// stdout/exit; trả stdout. Chương trình phải **tự dọn** đích để chạy lặp được.
fn hai_duong(d: &Path, tag: &str, src: &str) -> String {
    let p = d.join(format!("{tag}.vis"));
    viet_tep(&p, src);
    let (c1, o1, e1) = vis(&[std::ffi::OsStr::new("run"), p.as_os_str()]);
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        p.as_os_str(),
    ]);
    assert_eq!(c1, 0, "[{tag}] interp exit {c1}: {e1}");
    assert_eq!(c2, 0, "[{tag}] vm exit {c2}: {e2}");
    assert_eq!(o1, o2, "[{tag}] hai đường phải khớp: vm [{e2}]");
    o1
}

/// Như [`hai_duong`] nhưng cả hai đường chạy với `--fs-mock` (mỗi tiến trình nạp kịch bản mới ⇒
/// thao tác ghi trong bộ nhớ không rò sang lần chạy sau).
fn hai_duong_mock(d: &Path, tag: &str, mock: &str, src: &str) -> String {
    let m = d.join(format!("{tag}.json"));
    std::fs::write(&m, mock).expect("ghi kịch bản mock");
    let p = d.join(format!("{tag}.vis"));
    viet_tep(&p, src);
    let (c1, o1, e1) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--fs-mock"),
        m.as_os_str(),
        p.as_os_str(),
    ]);
    let (c2, o2, e2) = vis(&[
        std::ffi::OsStr::new("run"),
        std::ffi::OsStr::new("--vm"),
        std::ffi::OsStr::new("--no-cache"),
        std::ffi::OsStr::new("--fs-mock"),
        m.as_os_str(),
        p.as_os_str(),
    ]);
    assert_eq!(c1, 0, "[{tag}] interp exit {c1}: {e1}");
    assert_eq!(c2, 0, "[{tag}] vm exit {c2}: {e2}");
    assert_eq!(o1, o2, "[{tag}] hai đường phải khớp: vm [{e2}]");
    o1
}

fn chay(dir: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

// ---------- copy: chép byte, đích mới ----------

/// Chép **từng byte** (gồm byte không phải UTF-8); đích mới xuất hiện, nội dung trùng khít.
/// Chương trình tự `remove` đích trước nên chạy được cả hai đường trên cùng thư mục.
#[test]
fn copy_tep_sang_tep_moi_giu_nguyen_byte() {
    let d = tam("copy-moi");
    let src = d.join("nguon.bin");
    let dst = d.join("dich.bin");
    let noi_dung: Vec<u8> = vec![0x00, 0x01, 0xff, 0xfe, b'A', b'\n', 0x80];
    std::fs::write(&src, &noi_dung).expect("ghi nguồn nhị phân");

    let o = hai_duong(
        &d,
        "copy-moi",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.remove(\"{}\"):\n        Ok(_):\n            print(\"x\")\n        Err(e):\n            print(\"x\")\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"copy-ok\")\n        Err(e):\n            print(\"copy-err\")\n    print(fs.exists(\"{}\"))\n",
            duong(&dst),
            duong(&src),
            duong(&dst),
            duong(&dst)
        ),
    );
    assert_eq!(o, "x\ncopy-ok\ntrue\n");
    assert_eq!(
        std::fs::read(&dst).expect("đọc đích"),
        noi_dung,
        "copy phải chép đúng từng byte"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Chính sách đích tồn tại**: `copy` **từ chối** (không ghi đè) và nội dung đích cũ **nguyên vẹn**.
#[test]
fn copy_dich_da_ton_tai_bi_tu_choi_va_khong_ghi_de() {
    let d = tam("copy-dich");
    let src = d.join("nguon.txt");
    let dst = d.join("dich.txt");
    std::fs::write(&src, "MOI").expect("ghi nguồn");
    std::fs::write(&dst, "CU").expect("ghi đích cũ");

    let o = hai_duong(
        &d,
        "copy-dich",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"copy-ok\")\n        Err(e):\n            print(\"copy-err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-err\")\n",
            duong(&src),
            duong(&dst),
            duong(&dst)
        ),
    );
    assert_eq!(
        o, "copy-err\nCU\n",
        "đích tồn tại phải bị từ chối, không ghi đè"
    );
    assert_eq!(
        std::fs::read_to_string(&dst).expect("đọc đích"),
        "CU",
        "nội dung đích cũ phải nguyên vẹn"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Nguồn thiếu ⇒ `Err(Error)` dữ liệu (in `err`), **không** trap và **không** tạo đích rỗng.
#[test]
fn copy_nguon_thieu_la_err_khong_tao_dich() {
    let d = tam("copy-thieu");
    let src = d.join("khong-co.txt");
    let dst = d.join("dich.txt");

    let o = hai_duong(
        &d,
        "copy-thieu",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n    print(fs.exists(\"{}\"))\n",
            duong(&src),
            duong(&dst),
            duong(&dst)
        ),
    );
    assert_eq!(
        o, "err\nfalse\n",
        "nguồn thiếu: lỗi dữ liệu, không tạo đích"
    );
    assert!(!dst.exists(), "copy thất bại không được để lại tệp đích");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Chỉ tệp**: nguồn là thư mục ⇒ `Err`; đích đã là thư mục ⇒ `Err`; đích mới không xuất hiện.
#[test]
fn copy_tu_choi_thu_muc_hai_dau() {
    let d = tam("copy-dir");
    let thu_muc = d.join("thu-muc");
    std::fs::create_dir_all(&thu_muc).expect("mkdir");
    let src = d.join("nguon.txt");
    std::fs::write(&src, "noi dung").expect("ghi nguồn");
    let dst_moi = d.join("dich-moi.txt");

    let o = hai_duong(
        &d,
        "copy-dir",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"src-dir-ok\")\n        Err(e):\n            print(\"src-dir-err\")\n    print(fs.exists(\"{}\"))\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"dst-dir-ok\")\n        Err(e):\n            print(\"dst-dir-err\")\n",
            duong(&thu_muc),
            duong(&dst_moi),
            duong(&dst_moi),
            duong(&src),
            duong(&thu_muc)
        ),
    );
    assert_eq!(o, "src-dir-err\nfalse\ndst-dir-err\n");
    assert!(thu_muc.is_dir(), "đích là thư mục phải được giữ nguyên");
    assert!(!dst_moi.exists(), "copy thất bại không được tạo đích");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Không đi theo symlink**: nguồn là symlink ⇒ `Err`, không tạo đích; đích là symlink tới tệp
/// thật ⇒ `Err`, **đích thật không bị ghi**. Chỉ Unix có symlink.
#[cfg(unix)]
#[test]
fn copy_tu_choi_symlink_hai_dau() {
    use std::os::unix::fs::symlink;
    let d = tam("copy-sym");
    let that = d.join("that.txt");
    std::fs::write(&that, "NOI-DUNG-THAT").expect("ghi tệp thật");
    let link_nguon = d.join("link-nguon");
    symlink(&that, &link_nguon).expect("symlink nguồn");
    let src = d.join("nguon.txt");
    std::fs::write(&src, "MOI").expect("ghi nguồn");
    let link_dich = d.join("link-dich");
    symlink(&that, &link_dich).expect("symlink đích");
    let dst_moi = d.join("dich-moi.txt");

    let o = hai_duong(
        &d,
        "copy-sym",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"src-link-ok\")\n        Err(e):\n            print(\"src-link-err\")\n    print(fs.exists(\"{}\"))\n    match fs.copy(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"dst-link-ok\")\n        Err(e):\n            print(\"dst-link-err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-err\")\n",
            duong(&link_nguon),
            duong(&dst_moi),
            duong(&dst_moi),
            duong(&src),
            duong(&link_dich),
            duong(&that)
        ),
    );
    assert_eq!(o, "src-link-err\nfalse\ndst-link-err\nNOI-DUNG-THAT\n");
    assert_eq!(
        std::fs::read_to_string(&that).expect("đọc tệp thật"),
        "NOI-DUNG-THAT",
        "copy qua symlink đích không được ghi vào tệp thật"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- rename: tệp → tệp, đích mới ----------

/// `rename` tệp sang đích mới: nguồn biến mất, đích có nội dung cũ. Chương trình tự tạo lại
/// nguồn + dọn đích nên chạy lặp được.
#[test]
fn rename_tep_sang_dich_moi() {
    let d = tam("rename");
    let src = d.join("nguon.txt");
    let dst = d.join("dich.txt");

    let o = hai_duong(
        &d,
        "rename",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.remove(\"{}\"):\n        Ok(_):\n            print(\"x\")\n        Err(e):\n            print(\"x\")\n    match fs.write(\"{}\", \"noi-dung\"):\n        Ok(_):\n            print(\"w\")\n        Err(e):\n            print(\"w-err\")\n    match fs.rename(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"rename-ok\")\n        Err(e):\n            print(\"rename-err\")\n    print(fs.exists(\"{}\"))\n    print(fs.exists(\"{}\"))\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-err\")\n",
            duong(&dst),
            duong(&src),
            duong(&src),
            duong(&dst),
            duong(&src),
            duong(&dst),
            duong(&dst)
        ),
    );
    assert_eq!(o, "x\nw\nrename-ok\nfalse\ntrue\nnoi-dung\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Chính sách đích tồn tại**: `rename` **từ chối** (không dựa vào ngữ nghĩa ghi đè của Unix);
/// cả nguồn và đích giữ nguyên nội dung.
#[test]
fn rename_dich_da_ton_tai_bi_tu_choi_va_khong_ghi_de() {
    let d = tam("rename-dich");
    let src = d.join("nguon.txt");
    let dst = d.join("dich.txt");
    std::fs::write(&src, "MOI").expect("ghi nguồn");
    std::fs::write(&dst, "CU").expect("ghi đích cũ");

    let o = hai_duong(
        &d,
        "rename-dich",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.rename(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"rename-ok\")\n        Err(e):\n            print(\"rename-err\")\n    print(fs.exists(\"{}\"))\n    print(fs.exists(\"{}\"))\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-src-err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-dst-err\")\n",
            duong(&src),
            duong(&dst),
            duong(&src),
            duong(&dst),
            duong(&src),
            duong(&dst)
        ),
    );
    assert_eq!(o, "rename-err\ntrue\ntrue\nMOI\nCU\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Chỉ tệp**: nguồn là thư mục ⇒ `Err`; đích là thư mục ⇒ `Err`; cả hai vẫn tồn tại.
#[test]
fn rename_tu_choi_thu_muc_hai_dau() {
    let d = tam("rename-dir");
    let thu_muc = d.join("thu-muc");
    std::fs::create_dir_all(&thu_muc).expect("mkdir");
    let src = d.join("nguon.txt");
    std::fs::write(&src, "x").expect("ghi nguồn");
    let dst_moi = d.join("dich-moi.txt");

    let o = hai_duong(
        &d,
        "rename-dir",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.rename(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"src-dir-ok\")\n        Err(e):\n            print(\"src-dir-err\")\n    print(fs.exists(\"{}\"))\n    print(fs.exists(\"{}\"))\n    match fs.rename(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"dst-dir-ok\")\n        Err(e):\n            print(\"dst-dir-err\")\n",
            duong(&thu_muc),
            duong(&dst_moi),
            duong(&thu_muc),
            duong(&dst_moi),
            duong(&src),
            duong(&thu_muc)
        ),
    );
    assert_eq!(o, "src-dir-err\ntrue\nfalse\ndst-dir-err\n");
    assert!(thu_muc.is_dir(), "rename không được xoá/di chuyển thư mục");
    assert!(src.exists(), "rename thất bại không được xoá nguồn");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Không đi theo symlink**: nguồn là symlink ⇒ `Err`, tệp thật không bị di chuyển; đích là
/// symlink tới tệp thật ⇒ `Err`, tệp thật giữ nguyên. Chỉ Unix có symlink.
#[cfg(unix)]
#[test]
fn rename_tu_choi_symlink_hai_dau() {
    use std::os::unix::fs::symlink;
    let d = tam("rename-sym");
    let that = d.join("that.txt");
    std::fs::write(&that, "NOI-DUNG-THAT").expect("ghi tệp thật");
    let link_nguon = d.join("link-nguon");
    symlink(&that, &link_nguon).expect("symlink nguồn");
    let src = d.join("nguon.txt");
    std::fs::write(&src, "MOI").expect("ghi nguồn");
    let link_dich = d.join("link-dich");
    symlink(&that, &link_dich).expect("symlink đích");
    let dst_moi = d.join("dich-moi.txt");

    let o = hai_duong(
        &d,
        "rename-sym",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.rename(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"src-link-ok\")\n        Err(e):\n            print(\"src-link-err\")\n    print(fs.exists(\"{}\"))\n    match fs.rename(\"{}\", \"{}\"):\n        Ok(_):\n            print(\"dst-link-ok\")\n        Err(e):\n            print(\"dst-link-err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-err\")\n",
            duong(&link_nguon),
            duong(&dst_moi),
            duong(&dst_moi),
            duong(&src),
            duong(&link_dich),
            duong(&that)
        ),
    );
    assert_eq!(o, "src-link-err\nfalse\ndst-link-err\nNOI-DUNG-THAT\n");
    assert!(
        that.exists(),
        "tệp thật không được bị di chuyển qua symlink nguồn"
    );
    assert_eq!(
        std::fs::read_to_string(&that).expect("đọc tệp thật"),
        "NOI-DUNG-THAT"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quyền: không lách FsGuard ----------

/// `copy` cần **đọc** nguồn + **ghi** đích. Ba ca: (a) nguồn ngoài root ⇒ `VS0604` khi đọc;
/// (b) đích ngoài root dù có `(write)` ⇒ `VS0604`; (c) đích trong root nhưng cap **không**
/// `(write)` ⇒ `VS0604` nêu `(write)`. Cả hai đường cùng kết quả, không in nội dung.
#[test]
fn copy_quyen_doc_nguon_va_ghi_dich() {
    let d = tam("copy-quyen");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "x").expect("tệp nguồn");
    std::fs::write(d.join("ngoai.txt"), "bi mat").expect("tệp ngoài");
    let r = duong(&root);
    let ca = [
        (
            format!("fn main(r: cap[Dir] @ p\"{r}\"):"),
            format!("fs.copy(\"{r}/../ngoai.txt\", \"{r}/dich.txt\")"),
            "cap[Dir]",
            false,
        ),
        (
            format!("fn main(r: cap[Dir] @ p\"{r}\" (write)):"),
            format!("fs.copy(\"{r}/a.txt\", \"{r}/../dich-ngoai.txt\")"),
            "cap[Dir]",
            false,
        ),
        (
            format!("fn main(r: cap[Dir] @ p\"{r}\"):"),
            format!("fs.copy(\"{r}/a.txt\", \"{r}/dich.txt\")"),
            "(write)",
            true,
        ),
    ];
    for (i, (chu_ky, op, manh, co_write)) in ca.iter().enumerate() {
        let src = format!(
            "use std.fs\n\n{chu_ky}\n    match {op}:\n        Ok(_):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
        );
        viet_tep(&d.join("f.vis"), &src);
        let r1 = chay(&d, &["run", "f.vis"]);
        let r2 = chay(&d, &["run", "--vm", "--no-cache", "f.vis"]);
        assert_eq!(r1.0, 2, "ca {i}: interp phải exit 2: {r1:?}");
        assert_eq!(r2.0, 2, "ca {i}: vm phải exit 2: {r2:?}");
        assert_eq!(r1.1, "", "ca {i}: không được in kết quả");
        assert_eq!(r1.1, r2.1, "ca {i}: stdout lệch");
        for (duong_chay, rr) in [("interp", &r1), ("vm", &r2)] {
            assert!(
                rr.2.contains("VS0604"),
                "ca {i} ({duong_chay}): thiếu VS0604: {rr:?}"
            );
            assert!(
                rr.2.contains(manh),
                "ca {i} ({duong_chay}): thiếu `{manh}`: {rr:?}"
            );
            if *co_write {
                assert!(
                    rr.2.contains("(write)"),
                    "ca {i} ({duong_chay}): thiếu `(write)`: {rr:?}"
                );
            }
        }
    }
    assert!(
        !root.join("dich.txt").exists(),
        "quyền sai không được tạo đích"
    );
    assert!(
        !d.join("dich-ngoai.txt").exists(),
        "quyền sai không được tạo đích ngoài root"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `rename` cần **ghi** cả hai đầu. Hai ca: (a) nguồn trong root nhưng cap **không** `(write)`
/// ⇒ `VS0604` nêu `(write)` trước khi chạm đĩa; (b) nguồn trong root có `(write)`, đích ngoài
/// root ⇒ `VS0604`; nguồn vẫn nguyên.
#[test]
fn rename_quyen_ghi_hai_dau() {
    let d = tam("rename-quyen");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "x").expect("tệp nguồn");
    let r = duong(&root);
    let ca = [
        (
            format!("fn main(r: cap[Dir] @ p\"{r}\"):"),
            format!("fs.rename(\"{r}/a.txt\", \"{r}/b.txt\")"),
            "(write)",
        ),
        (
            format!("fn main(r: cap[Dir] @ p\"{r}\" (write)):"),
            format!("fs.rename(\"{r}/a.txt\", \"{r}/../b-ngoai.txt\")"),
            "cap[Dir]",
        ),
    ];
    for (i, (chu_ky, op, manh)) in ca.iter().enumerate() {
        let src = format!(
            "use std.fs\n\n{chu_ky}\n    match {op}:\n        Ok(_):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
        );
        viet_tep(&d.join("f.vis"), &src);
        let r1 = chay(&d, &["run", "f.vis"]);
        let r2 = chay(&d, &["run", "--vm", "--no-cache", "f.vis"]);
        assert_eq!(r1.0, 2, "ca {i}: interp phải exit 2: {r1:?}");
        assert_eq!(r2.0, 2, "ca {i}: vm phải exit 2: {r2:?}");
        assert_eq!(r1.1, "", "ca {i}: không được in kết quả");
        assert_eq!(r1.1, r2.1, "ca {i}: stdout lệch");
        for (duong_chay, rr) in [("interp", &r1), ("vm", &r2)] {
            assert!(
                rr.2.contains("VS0604"),
                "ca {i} ({duong_chay}): thiếu VS0604: {rr:?}"
            );
            assert!(
                rr.2.contains(manh),
                "ca {i} ({duong_chay}): thiếu `{manh}`: {rr:?}"
            );
        }
    }
    assert!(
        root.join("a.txt").exists(),
        "rename bị chặn không được xoá nguồn"
    );
    assert!(!root.join("b.txt").exists());
    assert!(!d.join("b-ngoai.txt").exists());
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- host giả lập ----------

/// `--fs-mock`: `copy`/`rename` chạy **trong bộ nhớ**, cùng chính sách đích tồn tại + từ chối
/// thư mục; hai đường byte-identical (mỗi tiến trình nạp kịch bản mới).
#[test]
fn mock_host_copy_rename() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "noi-dung" } }"#;
    let o = hai_duong_mock(
        &d,
        "mock",
        mock,
        "use std.fs\nfn main() -> ():\n    match fs.copy(\"a.txt\", \"b.txt\"):\n        Ok(_):\n            print(\"copy-ok\")\n        Err(e):\n            print(\"copy-err\")\n    match fs.read(\"b.txt\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-err\")\n    match fs.copy(\"a.txt\", \"b.txt\"):\n        Ok(_):\n            print(\"copy2-ok\")\n        Err(e):\n            print(\"copy2-err\")\n    match fs.rename(\"b.txt\", \"c.txt\"):\n        Ok(_):\n            print(\"rename-ok\")\n        Err(e):\n            print(\"rename-err\")\n    print(fs.exists(\"b.txt\"))\n    print(fs.exists(\"c.txt\"))\n    match fs.read(\"c.txt\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-c-err\")\n    match fs.rename(\"c.txt\", \"a.txt\"):\n        Ok(_):\n            print(\"rename2-ok\")\n        Err(e):\n            print(\"rename2-err\")\n    match fs.mkdir(\"d\"):\n        Ok(_):\n            print(\"mkdir-ok\")\n        Err(e):\n            print(\"mkdir-err\")\n    match fs.copy(\"d\", \"e\"):\n        Ok(_):\n            print(\"copy-dir-ok\")\n        Err(e):\n            print(\"copy-dir-err\")\n    match fs.rename(\"d\", \"e\"):\n        Ok(_):\n            print(\"rename-dir-ok\")\n        Err(e):\n            print(\"rename-dir-err\")\n    match fs.copy(\"thieu\", \"e\"):\n        Ok(_):\n            print(\"copy-missing-ok\")\n        Err(e):\n            print(\"copy-missing-err\")\n    print(fs.exists(\"e\"))\n",
    );
    assert_eq!(
        o,
        "copy-ok\nnoi-dung\ncopy2-err\nrename-ok\nfalse\ntrue\nnoi-dung\nrename2-err\n\
         mkdir-ok\ncopy-dir-err\nrename-dir-err\ncopy-missing-err\nfalse\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}
