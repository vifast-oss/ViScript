// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-canonicalize)** — `std.fs.canonicalize(p: str) -> Result[str, Error]`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - trả **đường dẫn tuyệt đối đã phân giải** của `p` bằng `std::fs::canonicalize` — **chạm đĩa**
//!   và đọc `cwd` cho đường dẫn tương đối (khác `path.normalize` thuần từ vựng);
//! - symlink (kể cả chuỗi link) được **phân giải**; `.`/`..` phân giải theo **trạng thái đĩa**;
//! - `p` phải **tồn tại**: rỗng/thiếu/`ENOENT` (gồm **dangling**)/`ENOTDIR` ⇒ `Err(Error)` **dữ
//!   liệu**, không trap; đích không UTF-8 ⇒ `Err` (không cắt cụt bằng lossy); lỗi quyền ⇒ `Err`;
//! - quyền: **chỉ đọc** (`cap[Dir]`/`cap[File]`, không cần `(write)`); path ngoài root hoặc symlink
//!   trỏ **ra ngoài** root ⇒ `VS0604`, exit 2, không in kết quả;
//! - **không sugar** (module-only, như `fs.read`/`fs.read_link`): `p.canonicalize()` ⇒ `VS0403`;
//!   sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`;
//! - host thật (`ngoai.rs`), `--fs-mock` (luôn `Err` — mock không có đĩa/cwd), interp và VM dùng
//!   **cùng** một ngữ nghĩa byte-identical.
//!
//! Ca symlink **bỏ qua mềm** khi nền không tạo được symlink (Windows thiếu quyền, hệ tệp không hỗ
//! trợ): in lý do ra stderr của test và `return`, không đỏ. Linux CI có symlink nên chạy thật.
//!
//! RED-BEFORE (ghi lúc làm VD-270, trước khi thêm hàm):
//!
//! ```text
//! $ vis check probe.vis                    # use std.fs + fs.canonicalize("x")
//! not ok 1 - probe.vis
//! # VS0301 5:11 module `std.fs` không có hàm `canonicalize`
//! $ vis run --no-cache probe.vis           ⇒ exit 2 (cùng lỗi)
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng lỗi)
//! $ vis check sugar.vis                    # "x".canonicalize(), không use
//! not ok 1 - sugar.vis
//! # VS0403 3:11 `str` không có trường `canonicalize`
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-canon-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

/// Đường dẫn tuyệt đối **đã canonicalize** trên phía test — cùng phép của host thật.
fn canon(p: &Path) -> String {
    std::fs::canonicalize(p)
        .unwrap_or_else(|e| panic!("canonicalize {}: {e}", p.display()))
        .display()
        .to_string()
}

fn viet_tep(p: &Path, src: &str) {
    std::fs::write(
        p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
}

/// Tạo symlink tới **tệp** (hoặc đích bất kỳ); `Err` khi nền không hỗ trợ.
#[cfg(unix)]
fn tao_link_tep(goc: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(goc, link)
}

/// Tạo symlink tới **thư mục**; Unix không phân biệt loại đích.
#[cfg(unix)]
fn tao_link_thu_muc(goc: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(goc, link)
}

#[cfg(windows)]
fn tao_link_tep(goc: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(goc, link)
}

#[cfg(windows)]
fn tao_link_thu_muc(goc: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(goc, link)
}

#[cfg(not(any(unix, windows)))]
fn tao_link_tep(_goc: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "nền không hỗ trợ symlink",
    ))
}

#[cfg(not(any(unix, windows)))]
fn tao_link_thu_muc(_goc: &Path, _link: &Path) -> std::io::Result<()> {
    tao_link_tep(_goc, _link)
}

fn chay(dir: &Path, args: &[&std::ffi::OsStr]) -> (i32, String, String) {
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

/// Chạy **cả hai đường** (interp + `--vm --no-cache`) trên **cùng** tệp chương trình, từ **cùng**
/// cwd (để đường dẫn tương đối + `cwd` phân giải giống nhau), đòi byte-identical stdout/stderr/exit;
/// trả `(exit, stdout, stderr)`.
fn hai_duong(d: &Path, tag: &str, src: &str) -> (i32, String, String) {
    let ten = format!("{tag}.vis");
    viet_tep(&d.join(&ten), src);
    let (c1, o1, e1) = chay(
        d,
        &[std::ffi::OsStr::new("run"), std::ffi::OsStr::new(&ten)],
    );
    let (c2, o2, e2) = chay(
        d,
        &[
            std::ffi::OsStr::new("run"),
            std::ffi::OsStr::new("--vm"),
            std::ffi::OsStr::new("--no-cache"),
            std::ffi::OsStr::new(&ten),
        ],
    );
    assert_eq!(
        c1, c2,
        "[{tag}] exit hai đường lệch: interp {c1}, vm {c2}\n[e1] {e1}\n[e2] {e2}"
    );
    assert_eq!(o1, o2, "[{tag}] stdout hai đường phải khớp byte: vm [{e2}]");
    assert_eq!(e1, e2, "[{tag}] stderr hai đường phải khớp byte");
    (c1, o1, e1)
}

/// Chương trình hợp lệ: hai đường cùng chạy sạch, stdout đúng kỳ vọng.
fn dung(d: &Path, tag: &str, src: &str, stdout: &str) {
    let (code, out, err) = hai_duong(d, tag, src);
    assert_eq!(
        code, 0,
        "[{tag}] phải chạy sạch (exit 0), được {code}\nstdout: {out}\nstderr: {err}"
    );
    assert_eq!(out, stdout, "[{tag}] stdout sai");
}

/// Lỗi **tĩnh**: `vis check` đỏ (exit 1, mã trong TAP) và **cả hai** engine từ chối (exit 2).
fn tu_choi(d: &Path, tag: &str, src: &str, ma: &str) {
    let ten = format!("{tag}.vis");
    viet_tep(&d.join(&ten), src);
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(["check", &ten])
        .current_dir(d)
        .output()
        .expect("chạy vis check");
    let co = String::from_utf8_lossy(&o.stdout).to_string();
    let ce = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(
        o.status.code().unwrap_or(-1),
        1,
        "[{tag}] `check` phải đỏ (exit 1)\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        co.contains(ma) || ce.contains(ma),
        "[{tag}] `check` thiếu `{ma}`\nstdout: {co}\nstderr: {ce}"
    );
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let mut args = vec![std::ffi::OsStr::new("run")];
        args.extend(extra.iter().map(std::ffi::OsStr::new));
        args.push(std::ffi::OsStr::new(&ten));
        let (code, so, se) = chay(d, &args);
        assert_eq!(
            code, 2,
            "[{tag}] engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "[{tag}] engine={i}: thiếu `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
}

// ===========================================================================
// 1 — tệp/thư mục thật: tương đối + tuyệt đối, `.`/`..`, kết quả tuyệt đối
// ===========================================================================

/// `canonicalize` trả **cùng một** đường dẫn tuyệt đối cho mọi cách viết của cùng vật:
/// tương đối (`tep.txt`, `./tep.txt`), có `.`/`..` (`thu-muc/../tep.txt`), `.`/`thư mục/..`
/// (⇒ chính cwd) và tuyệt đối. Kết quả luôn **tuyệt đối** (`path.is_absolute` = `true`).
#[test]
fn tep_thu_muc_that_tuong_doi_tuyet_doi_va_cham_dia() {
    let d = tam("that");
    std::fs::write(d.join("tep.txt"), "NOI-DUNG").expect("ghi tệp");
    std::fs::create_dir_all(d.join("thu-muc")).expect("mkdir");
    let tep = canon(&d.join("tep.txt"));
    let thu_muc = canon(&d.join("thu-muc"));
    let goc = canon(&d);
    let abs_tep = duong(&d.join("tep.txt"));
    let abs_thu_muc = duong(&d.join("thu-muc"));

    let src = format!(
        r#"use std.fs
use std.path

fn main():
    for p in ["tep.txt", "./tep.txt", "thu-muc/../tep.txt", "thu-muc", ".", "thu-muc/..", "{abs_tep}", "{abs_thu_muc}"]:
        match fs.canonicalize(p):
            Ok(t):
                print(t)
                print(path.is_absolute(t))
            Err(e):
                print("err")
"#
    );
    let ky_vong = format!(
        "{tep}\ntrue\n{tep}\ntrue\n{tep}\ntrue\n{thu_muc}\ntrue\n{goc}\ntrue\n{goc}\ntrue\n{tep}\ntrue\n{thu_muc}\ntrue\n"
    );
    dung(&d, "that", &src, &ky_vong);
    let _ = std::fs::remove_dir_all(&d);
}

/// Khác `path.normalize` (**thuần từ vựng**): `normalize` trả `b` cho `a/../b` kể cả khi `b`
/// không tồn tại, còn `canonicalize` **chạm đĩa** — trả đường dẫn tuyệt đối đã phân giải khi vật
/// tồn tại, và `Err` khi thiếu. Đây là ranh giới ngữ nghĩa giữa hai hàm.
#[test]
fn khac_path_normalize_thuan_tu_vung() {
    let d = tam("normalize");
    std::fs::create_dir_all(d.join("a")).expect("mkdir a");
    std::fs::write(d.join("b"), "x").expect("ghi b");
    let b = canon(&d.join("b"));

    let src = r#"use std.fs
use std.path

fn main():
    print(path.normalize("a/../b"))
    match fs.canonicalize("a/../b"):
        Ok(t):
            print(t)
        Err(e):
            print("err")
    print(path.normalize("khong-co/../x"))
    match fs.canonicalize("khong-co/../x"):
        Ok(t):
            print(t)
        Err(e):
            print("err")
"#;
    let ky_vong = format!("b\n{b}\nx\nerr\n");
    dung(&d, "normalize", src, &ky_vong);
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 2 — symlink: trong root được **phân giải** (kể cả chuỗi link), dangling ⇒ Err
// ===========================================================================

/// Dựng bộ fixture symlink; `false` = nền không tạo được symlink ⇒ ca test bỏ qua mềm.
fn fixture_symlink(d: &Path) -> bool {
    if let Err(e) = tao_link_tep(Path::new("tep.txt"), &d.join("link-tep")) {
        eprintln!("bỏ qua: nền không tạo được symlink tệp ({e})");
        return false;
    }
    if let Err(e) = tao_link_thu_muc(Path::new("thu-muc"), &d.join("link-thu-muc")) {
        eprintln!("bỏ qua: nền không tạo được symlink thư mục ({e})");
        return false;
    }
    // Chuỗi `link2 → link1 → tep.txt`.
    if let Err(e) = tao_link_tep(Path::new("tep.txt"), &d.join("link1")) {
        eprintln!("bỏ qua: nền không tạo được symlink chuỗi ({e})");
        return false;
    }
    if let Err(e) = tao_link_tep(Path::new("link1"), &d.join("link2")) {
        eprintln!("bỏ qua: nền không tạo được symlink chuỗi ({e})");
        return false;
    }
    // Dangling: đích cố ý **không** tồn tại.
    if let Err(e) = tao_link_tep(Path::new("khong-ton-tai"), &d.join("link-missing")) {
        eprintln!("bỏ qua: nền không tạo được symlink dangling ({e})");
        return false;
    }
    true
}

/// Symlink tới tệp/thư mục và **chuỗi** link đều trả đích **đã phân giải** (không phải tên link);
/// dangling ⇒ `Err` dữ liệu (khác `read_link`), không trap.
#[test]
fn symlink_trong_root_duoc_phan_giai_va_dangling_la_err() {
    let d = tam("symlink");
    std::fs::write(d.join("tep.txt"), "NOI-DUNG").expect("ghi tệp");
    std::fs::create_dir_all(d.join("thu-muc")).expect("mkdir");
    if !fixture_symlink(&d) {
        return;
    }
    let tep = canon(&d.join("tep.txt"));
    let thu_muc = canon(&d.join("thu-muc"));

    let src = r#"use std.fs

fn main():
    for p in ["link-tep", "link-thu-muc", "link1", "link2", "link-missing"]:
        match fs.canonicalize(p):
            Ok(t):
                print(t)
            Err(e):
                print("err")
"#;
    let ky_vong = format!("{tep}\n{thu_muc}\n{tep}\n{tep}\nerr\n");
    dung(&d, "symlink", src, &ky_vong);
    let _ = std::fs::remove_dir_all(&d);
}

/// Đích canonicalize **không phải UTF-8** ⇒ `Err(Error)` dữ liệu (không cắt cụt bằng lossy).
/// Unix-only; bỏ qua mềm nếu nền không tạo được symlink.
#[cfg(unix)]
#[test]
fn dich_khong_utf8_la_err() {
    use std::os::unix::ffi::OsStrExt;

    let d = tam("khong-utf8");
    let ten = std::ffi::OsStr::from_bytes(b"tep-\xff.txt");
    std::fs::write(d.join(ten), "x").expect("ghi tệp tên không UTF-8");
    if let Err(e) = tao_link_tep(Path::new(ten), &d.join("link-khong-utf8")) {
        eprintln!("bỏ qua: nền không tạo được symlink ({e})");
        return;
    }
    let src = r#"use std.fs

fn main():
    match fs.canonicalize("link-khong-utf8"):
        Ok(t):
            print("ok")
        Err(e):
            print("err")
    print("xong")
"#;
    dung(&d, "khong-utf8", src, "err\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 3 — rỗng/thiếu/ENOTDIR ⇒ Err(Error) dữ liệu, không trap
// ===========================================================================

/// `p` rỗng, thiếu, có thư mục cha thiếu, hoặc `tep.txt/con` (`ENOTDIR`) ⇒ `Err` dữ liệu;
/// chương trình in `err` rồi **tiếp tục**, không trap/exit khác 0.
#[test]
fn rong_thieu_enotdir_la_err_khong_trap() {
    let d = tam("thieu");
    std::fs::write(d.join("tep.txt"), "x").expect("ghi tệp");
    let src = r#"use std.fs

fn main():
    for p in ["", "khong-co", "khong/co", "tep.txt/con"]:
        match fs.canonicalize(p):
            Ok(t):
                print("ok")
            Err(e):
                print("err")
    print("xong")
"#;
    dung(&d, "thieu", src, "err\nerr\nerr\nerr\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 4 — quyền: chỉ đọc trong cap[Dir]; ngoài root / symlink ra ngoài ⇒ VS0604
// ===========================================================================

/// `canonicalize` là thao tác **đọc** ⇒ chạy trong `cap[Dir]` **không** `(write)`; tệp, thư mục
/// và symlink **trong** root đều `Ok` với đích canonicalize.
#[test]
fn quyen_doc_trong_root_khong_can_write() {
    let d = tam("quyen-doc");
    let root = d.join("root");
    std::fs::create_dir_all(root.join("thu-muc")).expect("root");
    std::fs::write(root.join("tep.txt"), "x").expect("tệp");
    let mut co_link = true;
    if let Err(e) = tao_link_tep(Path::new("tep.txt"), &root.join("link-trong")) {
        eprintln!("bỏ qua symlink trong root: nền không tạo được ({e})");
        co_link = false;
    }
    let r = duong(&root);
    let tep = canon(&root.join("tep.txt"));
    let thu_muc = canon(&root.join("thu-muc"));
    let danh_sach = if co_link {
        format!("\"{r}/tep.txt\", \"{r}/thu-muc\", \"{r}/link-trong\"")
    } else {
        format!("\"{r}/tep.txt\", \"{r}/thu-muc\"")
    };
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    for p in [{danh_sach}]:\n        match fs.canonicalize(p):\n            Ok(t):\n                print(t)\n            Err(e):\n                print(\"err\")\n    print(\"xong\")\n"
    );
    let ky_vong = if co_link {
        format!("{tep}\n{thu_muc}\n{tep}\nxong\n")
    } else {
        format!("{tep}\n{thu_muc}\nxong\n")
    };
    viet_tep(&d.join("f.vis"), &src);
    for args in [
        vec!["run", "f.vis"],
        vec!["run", "--vm", "--no-cache", "f.vis"],
    ] {
        let r1 = chay(
            &d,
            &args.iter().map(std::ffi::OsStr::new).collect::<Vec<_>>(),
        );
        assert_eq!(r1.0, 0, "{args:?} phải chạy được: {r1:?}");
        assert_eq!(r1.1, ky_vong, "{args:?}: stdout lệch");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Path ngoài `cap[Dir]` ⇒ `VS0604` (exit 2), không in kết quả; hai đường cùng kết quả. Symlink
/// trong root trỏ **ra ngoài** cũng bị chặn vì bước kiểm quyền canonicalize theo **đích**
/// (cùng chính sách `FsGuard` với mọi đường đọc).
#[test]
fn quyen_ngoai_root_bi_chan() {
    let d = tam("quyen-ngoai");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(d.join("ngoai.txt"), "bi mat").expect("tệp ngoài");
    let mut chuong_trinh = vec![format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{root_disp}\"):\n    match fs.canonicalize(\"{root_disp}/../ngoai.txt\"):\n        Ok(t):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n",
        root_disp = duong(&root)
    )];
    if tao_link_tep(Path::new("../ngoai.txt"), &root.join("link-ra-ngoai")).is_ok() {
        chuong_trinh.push(format!(
            "use std.fs\n\nfn main(r: cap[Dir] @ p\"{root_disp}\"):\n    match fs.canonicalize(\"{root_disp}/link-ra-ngoai\"):\n        Ok(t):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n",
            root_disp = duong(&root)
        ));
    } else {
        eprintln!("bỏ qua symlink ra ngoài: nền không tạo được symlink");
    }
    for (i, src) in chuong_trinh.iter().enumerate() {
        viet_tep(&d.join("f.vis"), src);
        let r1 = chay(
            &d,
            &[std::ffi::OsStr::new("run"), std::ffi::OsStr::new("f.vis")],
        );
        let r2 = chay(
            &d,
            &[
                std::ffi::OsStr::new("run"),
                std::ffi::OsStr::new("--vm"),
                std::ffi::OsStr::new("--no-cache"),
                std::ffi::OsStr::new("f.vis"),
            ],
        );
        assert_eq!(r1.0, 2, "ca {i}: interp phải exit 2: {r1:?}");
        assert_eq!(r2.0, 2, "ca {i}: vm phải exit 2: {r2:?}");
        assert_eq!(r1.1, "", "ca {i}: không được in kết quả");
        assert_eq!(r1.1, r2.1, "ca {i}: stdout lệch");
        for (duong_chay, rr) in [("interp", &r1), ("vm", &r2)] {
            assert!(
                rr.2.contains("VS0604"),
                "ca {i} {duong_chay}: thiếu VS0604: {rr:?}"
            );
            assert!(
                rr.2.contains("cap[Dir]"),
                "ca {i} {duong_chay}: thiếu `cap[Dir]`: {rr:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 5 — host giả lập: mock không có đĩa/cwd ⇒ luôn Err
// ===========================================================================

/// `--fs-mock` không có đĩa/cwd thật ⇒ `canonicalize` luôn `Err` cho tệp/thư mục/thiếu/rỗng;
/// hai đường byte-identical.
#[test]
fn mock_host_luon_err() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "abc" }, "dirs": { "thu-muc": [] } }"#;
    let m = d.join("mock.json");
    std::fs::write(&m, mock).expect("ghi kịch bản mock");
    let src = r#"use std.fs

fn main():
    for p in ["a.txt", "thu-muc", "thieu", ""]:
        match fs.canonicalize(p):
            Ok(t):
                print(t)
            Err(e):
                print("err")
"#;
    viet_tep(&d.join("m.vis"), src);
    let m = std::ffi::OsStr::new(&m);
    let (c1, o1, e1) = chay(
        &d,
        &[
            std::ffi::OsStr::new("run"),
            std::ffi::OsStr::new("--fs-mock"),
            m,
            std::ffi::OsStr::new("m.vis"),
        ],
    );
    let (c2, o2, e2) = chay(
        &d,
        &[
            std::ffi::OsStr::new("run"),
            std::ffi::OsStr::new("--vm"),
            std::ffi::OsStr::new("--no-cache"),
            std::ffi::OsStr::new("--fs-mock"),
            m,
            std::ffi::OsStr::new("m.vis"),
        ],
    );
    assert_eq!(c1, 0, "interp: {e1}");
    assert_eq!(c2, 0, "vm: {e2}");
    assert_eq!(o1, o2, "stdout hai đường lệch");
    assert_eq!(o1, "err\nerr\nerr\nerr\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 6 — **không** sugar: module-only
// ===========================================================================

/// `p.canonicalize()` **không** phải sugar (giống `fs.read`/`fs.read_link`): `str` không có trường
/// `canonicalize` ⇒ `VS0403`, kể cả khi đã `use std.fs`.
#[test]
fn khong_sugar_module_only() {
    let d = tam("sugar");
    for (tag, src) in [
        (
            "sugar-literal",
            "fn main():\n    print(\"a\".canonicalize())\n",
        ),
        (
            "sugar-bien",
            "fn main():\n    let p = \"a\"\n    print(p.canonicalize())\n",
        ),
        (
            "sugar-co-use",
            "use std.fs\n\nfn main():\n    print(\"a\".canonicalize())\n",
        ),
    ] {
        tu_choi(&d, tag, src, "VS0403");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 7 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `int`/`bool` thay `str` ⇒ `VS0412`; arity 0/2 ⇒ `VS0401`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    let d = tam("kieu");
    for (tag, call, ma) in [
        ("kieu-int", "fs.canonicalize(1)", "VS0412"),
        ("kieu-bool", "fs.canonicalize(true)", "VS0412"),
        ("arity-0", "fs.canonicalize()", "VS0401"),
        ("arity-2", "fs.canonicalize(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.fs\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(&d, tag, &src, ma);
    }
    let _ = std::fs::remove_dir_all(&d);
}
