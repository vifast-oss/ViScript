// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-is-symlink)** — `std.fs.is_symlink(p: str) -> Result[bool, Error]`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - `true` ⇔ **chính** `p` là **symbolic link**, dùng `symlink_metadata` nên **không** đi theo
//!   link: symlink → tệp, symlink → thư mục, và symlink → đích **thiếu** (dangling) đều `true`;
//! - tệp thường, thư mục thường và `p` **thiếu** ⇒ `false`;
//! - **dangling symlink** là khác biệt then chốt với `exists` (theo link ⇒ `false`);
//! - lỗi metadata khác (vd `tep.txt/con` ⇒ `ENOTDIR`) ⇒ `Err(Error)` **dữ liệu**, không trap;
//! - quyền: **chỉ đọc** (`cap[Dir]`/`cap[File]`, không cần `(write)`); ngoài root ⇒ `VS0604`,
//!   exit 2, không in kết quả;
//! - **không sugar** (module-only, như `fs.is_file`/`fs.is_dir`): `p.is_symlink()` ⇒ `VS0403`;
//!   sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`;
//! - host thật (`ngoai.rs`), `--fs-mock`, interp và VM dùng **cùng** một ngữ nghĩa byte-identical.
//!
//! Ca symlink **bỏ qua mềm** khi nền không tạo được symlink (Windows thiếu quyền, hệ tệp không
//! hỗ trợ): in lý do ra stderr của test và `return`, không đỏ. Linux CI có symlink nên chạy thật.
//!
//! RED-BEFORE (ghi lúc làm VD-270, trước khi thêm hàm):
//!
//! ```text
//! $ vis check probe.vis                    # use std.fs + fs.is_symlink("x")
//! not ok 1 - probe.vis
//! # VS0301 module `std.fs` không có hàm `is_symlink`
//! $ vis run --no-cache probe.vis           ⇒ exit 2 (cùng lỗi)
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng lỗi)
//! $ vis check sugar.vis                    # "x".is_symlink(), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `is_symlink`
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-sym-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn duong(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
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
/// cwd (để đường dẫn tương đối phân giải giống nhau), đòi byte-identical stdout/stderr/exit;
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
// 1 — symlink thật: tệp, thư mục, đích thiếu (dangling)
// ===========================================================================

/// Dựng bộ fixture symlink; `None` = nền không tạo được symlink ⇒ ca test bỏ qua mềm.
fn fixture_symlink(d: &Path) -> bool {
    if let Err(e) = tao_link_tep(Path::new("tep.txt"), &d.join("link-tep")) {
        eprintln!("bỏ qua: nền không tạo được symlink tệp ({e})");
        return false;
    }
    if let Err(e) = tao_link_thu_muc(Path::new("thu-muc"), &d.join("link-thu-muc")) {
        eprintln!("bỏ qua: nền không tạo được symlink thư mục ({e})");
        return false;
    }
    // Dangling: đích cố ý **không** tồn tại.
    if let Err(e) = tao_link_tep(Path::new("khong-ton-tai"), &d.join("link-missing")) {
        eprintln!("bỏ qua: nền không tạo được symlink dangling ({e})");
        return false;
    }
    true
}

/// Symlink → tệp, symlink → thư mục, symlink dangling đều `Ok(true)`; tệp/thư mục thường và
/// đường dẫn thiếu đều `Ok(false)`. `exists` đi theo link nên dangling ⇒ `false` — đúng điểm
/// khác biệt then chốt giữa hai hàm.
#[test]
fn symlink_tep_thu_muc_dangling() {
    let d = tam("that");
    std::fs::write(d.join("tep.txt"), "NOI-DUNG").expect("ghi tệp");
    std::fs::create_dir_all(d.join("thu-muc")).expect("mkdir");
    if !fixture_symlink(&d) {
        return;
    }

    let src = r#"use std.fs

fn main():
    for p in ["tep.txt", "thu-muc", "link-tep", "link-thu-muc", "link-missing", "khong-co"]:
        match fs.is_symlink(p):
            Ok(b):
                print(p)
                print(b)
            Err(e):
                print(p)
                print("err")
    print(fs.exists("link-missing"))
    print(fs.exists("link-tep"))
"#;
    dung(
        &d,
        "symlink",
        src,
        "tep.txt\nfalse\nthu-muc\nfalse\nlink-tep\ntrue\nlink-thu-muc\ntrue\nlink-missing\ntrue\nkhong-co\nfalse\nfalse\ntrue\n",
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Symlink **không** bị đi theo: `is_file`/`is_dir` của link vẫn `false` (cùng `symlink_metadata`
/// với `is_symlink`), trong khi `is_symlink` của chính link là `true`; đích thật vẫn được đọc.
#[test]
fn symlink_khong_bi_di_theo_boi_isfile_isdir() {
    let d = tam("khong-theo");
    std::fs::write(d.join("tep.txt"), "NOI-DUNG").expect("ghi tệp");
    std::fs::create_dir_all(d.join("thu-muc")).expect("mkdir");
    if !fixture_symlink(&d) {
        return;
    }

    let src = r#"use std.fs

fn main():
    match fs.is_file("link-tep"):
        Ok(b):
            print(b)
        Err(e):
            print("file-err")
    match fs.is_dir("link-thu-muc"):
        Ok(b):
            print(b)
        Err(e):
            print("dir-err")
    match fs.is_symlink("link-tep"):
        Ok(b):
            print(b)
        Err(e):
            print("sym-err")
    match fs.read("tep.txt"):
        Ok(s):
            print(s)
        Err(e):
            print("read-err")
"#;
    dung(&d, "khong-theo", src, "false\nfalse\ntrue\nNOI-DUNG\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 2 — tương đối và tuyệt đối: cùng một đối tượng, cùng kết quả
// ===========================================================================

/// `link-tep` gọi bằng **tên tương đối** (cwd = thư mục fixture) và bằng **đường dẫn tuyệt đối**
/// đều `true`; tệp thường cũng vậy nhưng `false`.
#[test]
fn tuong_doi_va_tuyet_doi() {
    let d = tam("tuyet-doi");
    std::fs::write(d.join("tep.txt"), "x").expect("ghi tệp");
    if let Err(e) = tao_link_tep(&d.join("tep.txt"), &d.join("link-tuyet-doi")) {
        eprintln!("bỏ qua: nền không tạo được symlink tuyệt đối ({e})");
        return;
    }
    let tep = duong(&d.join("tep.txt"));
    let link = duong(&d.join("link-tuyet-doi"));
    let src = format!(
        "use std.fs\n\nfn main():\n    for p in [\"link-tuyet-doi\", \"{link}\", \"tep.txt\", \"{tep}\"]:\n        match fs.is_symlink(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n"
    );
    // Trong `for`, hai đầu tương đối là 2 phần tử đầu; hai đầu tuyệt đối là 2 phần tử cuối.
    dung(&d, "tuyet-doi", &src, "true\ntrue\nfalse\nfalse\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 3 — lỗi metadata ⇒ Err(Error) dữ liệu, không trap
// ===========================================================================

/// `tep.txt/con` là `ENOTDIR` (không phải `NotFound`) ⇒ `Err(Error)` dữ liệu; chương trình in
/// `err` rồi **tiếp tục**, không trap/exit khác 0.
#[test]
fn loi_metadata_la_err_khong_trap() {
    let d = tam("metadata-err");
    std::fs::write(d.join("tep.txt"), "x").expect("ghi tệp");
    let src = r#"use std.fs

fn main():
    match fs.is_symlink("tep.txt/con"):
        Ok(b):
            print("ok")
        Err(e):
            print("err")
    print("xong")
"#;
    dung(&d, "metadata-err", src, "err\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 4 — quyền: chỉ đọc trong cap[Dir]; ngoài root ⇒ VS0604
// ===========================================================================

/// `is_symlink` là thao tác **đọc** ⇒ chạy trong `cap[Dir]` **không** `(write)`; symlink nội bộ
/// và dangling trong root đều quan sát được.
#[test]
fn quyen_doc_trong_root_khong_can_write() {
    let d = tam("quyen-doc");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("tep.txt"), "x").expect("tệp");
    if let Err(e) = tao_link_tep(Path::new("tep.txt"), &root.join("link-tep")) {
        eprintln!("bỏ qua: nền không tạo được symlink ({e})");
        return;
    }
    if let Err(e) = tao_link_tep(Path::new("khong-co"), &root.join("link-missing")) {
        eprintln!("bỏ qua: nền không tạo được symlink dangling ({e})");
        return;
    }
    let r = duong(&root);
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    for p in [\"{r}/tep.txt\", \"{r}/link-tep\", \"{r}/link-missing\"]:\n        match fs.is_symlink(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n"
    );
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
        assert_eq!(r1.1, "false\ntrue\ntrue\n", "{args:?}: stdout lệch");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Path **ngoài** `cap[Dir]` ⇒ `VS0604` (exit 2), không in kết quả; hai đường cùng kết quả.
#[test]
fn quyen_ngoai_root_bi_chan() {
    let d = tam("quyen-ngoai");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(d.join("ngoai.txt"), "bi mat").expect("tệp ngoài");
    let r = duong(&root);
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match fs.is_symlink(\"{r}/../ngoai.txt\"):\n        Ok(b):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
    );
    viet_tep(&d.join("f.vis"), &src);
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
    assert_eq!(r1.0, 2, "interp phải exit 2: {r1:?}");
    assert_eq!(r2.0, 2, "vm phải exit 2: {r2:?}");
    assert_eq!(r1.1, "", "không được in kết quả");
    assert_eq!(r1.1, r2.1, "stdout lệch");
    for (duong_chay, rr) in [("interp", &r1), ("vm", &r2)] {
        assert!(
            rr.2.contains("VS0604"),
            "{duong_chay}: thiếu VS0604: {rr:?}"
        );
        assert!(
            rr.2.contains("cap[Dir]"),
            "{duong_chay}: thiếu `cap[Dir]`: {rr:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 5 — host giả lập: mock không biểu diễn symlink ⇒ luôn false
// ===========================================================================

/// `--fs-mock` không có khái niệm symlink ⇒ `is_symlink` luôn `Ok(false)` cho tệp/thư mục/thiếu;
/// hai đường byte-identical.
#[test]
fn mock_host_khong_bieu_dien_symlink() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "abc" }, "dirs": { "thu-muc": [] } }"#;
    let m = d.join("mock.json");
    std::fs::write(&m, mock).expect("ghi kịch bản mock");
    let src = r#"use std.fs

fn main():
    for p in ["a.txt", "thu-muc", "thieu"]:
        match fs.is_symlink(p):
            Ok(b):
                print(b)
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
    assert_eq!(o1, "false\nfalse\nfalse\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 6 — **không** sugar: module-only
// ===========================================================================

/// `p.is_symlink()` **không** phải sugar (giống `is_file`/`is_dir`): `str` không có trường
/// `is_symlink` ⇒ `VS0403`, kể cả khi đã `use std.fs`.
#[test]
fn khong_sugar_module_only() {
    let d = tam("sugar");
    for (tag, src) in [
        (
            "sugar-literal",
            "fn main():\n    print(\"a\".is_symlink())\n",
        ),
        (
            "sugar-bien",
            "fn main():\n    let p = \"a\"\n    print(p.is_symlink())\n",
        ),
        (
            "sugar-co-use",
            "use std.fs\n\nfn main():\n    print(\"a\".is_symlink())\n",
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
        ("kieu-int", "fs.is_symlink(1)", "VS0412"),
        ("kieu-bool", "fs.is_symlink(true)", "VS0412"),
        ("arity-0", "fs.is_symlink()", "VS0401"),
        ("arity-2", "fs.is_symlink(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.fs\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(&d, tag, &src, ma);
    }
    let _ = std::fs::remove_dir_all(&d);
}
