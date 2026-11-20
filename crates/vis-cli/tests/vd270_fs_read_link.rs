// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-read-link)** — `std.fs.read_link(p: str) -> Result[str, Error]`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - `read_link` trả **đích** của symbolic link **nguyên văn** — tương đối hay tuyệt đối đúng như
//!   lúc tạo — và **không đi theo** link (`std::fs::read_link`);
//! - symlink → đích **thiếu** (dangling) vẫn `Ok(target)` (khác `exists`/`fs.read`); symlink →
//!   tệp và symlink → thư mục cũng `Ok(target)`;
//! - tệp thường, thư mục thường và `p` **thiếu** ⇒ `Err(Error)` **dữ liệu**, không trap;
//! - lỗi khác (vd `tep.txt/con` ⇒ `ENOTDIR`, không đủ quyền OS) ⇒ `Err(Error)` dữ liệu, không trap;
//! - quyền: **chỉ đọc** (`cap[Dir]`/`cap[File]`, không cần `(write)`); path ngoài root ⇒ `VS0604`,
//!   exit 2, không in kết quả; bước kiểm canonicalize như mọi đường đọc nên symlink trỏ **ra
//!   ngoài** root cũng bị `VS0604`;
//! - **không sugar** (module-only, như `fs.read`/`fs.is_symlink`): `p.read_link()` ⇒ `VS0403`;
//!   sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`;
//! - host thật (`ngoai.rs`), `--fs-mock` (luôn `Err` — mock không biểu diễn symlink), interp và
//!   VM dùng **cùng** một ngữ nghĩa byte-identical.
//!
//! Ca symlink **bỏ qua mềm** khi nền không tạo được symlink (Windows thiếu quyền, hệ tệp không
//! hỗ trợ): in lý do ra stderr của test và `return`, không đỏ. Linux CI có symlink nên chạy thật.
//!
//! RED-BEFORE (ghi lúc làm VD-270, trước khi thêm hàm):
//!
//! ```text
//! $ vis check probe.vis                    # use std.fs + fs.read_link("x")
//! not ok 1 - probe.vis
//! # VS0301 module `std.fs` không có hàm `read_link`
//! $ vis run --no-cache probe.vis           ⇒ exit 2 (cùng lỗi)
//! $ vis run --no-cache --vm probe.vis      ⇒ exit 2 (cùng lỗi)
//! $ vis check sugar.vis                    # "x".read_link(), không use
//! not ok 1 - sugar.vis
//! # VS0403 2:11 `str` không có trường `read_link`
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-link-{tag}-{}-{n}", std::process::id()));
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
// 1 — symlink thật: đích tương đối, tuyệt đối, thư mục, dangling
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
    // Dangling: đích cố ý **không** tồn tại.
    if let Err(e) = tao_link_tep(Path::new("khong-ton-tai"), &d.join("link-missing")) {
        eprintln!("bỏ qua: nền không tạo được symlink dangling ({e})");
        return false;
    }
    true
}

/// `read_link` trả đích **nguyên văn**: tương đối (`tep.txt`, `thu-muc`, `khong-ton-tai`) và
/// tuyệt đối (đúng chuỗi lúc tạo); dangling vẫn `Ok` dù `exists` là `false`. Tệp/thư mục thường
/// và path thiếu đều `Err`.
#[test]
fn dich_tuong_doi_tuyet_doi_va_dangling() {
    let d = tam("dich");
    std::fs::write(d.join("tep.txt"), "NOI-DUNG").expect("ghi tệp");
    std::fs::create_dir_all(d.join("thu-muc")).expect("mkdir");
    if !fixture_symlink(&d) {
        return;
    }
    let tep = duong(&d.join("tep.txt"));
    if let Err(e) = tao_link_tep(&d.join("tep.txt"), &d.join("link-tuyet-doi")) {
        eprintln!("bỏ qua: nền không tạo được symlink tuyệt đối ({e})");
        return;
    }

    let src = r#"use std.fs

fn main():
    for p in ["link-tep", "link-thu-muc", "link-missing", "link-tuyet-doi", "tep.txt", "thu-muc", "khong-co"]:
        match fs.read_link(p):
            Ok(t):
                print(p)
                print(t)
            Err(e):
                print(p)
                print("err")
    print(fs.exists("link-missing"))
"#;
    let ky_vong = format!(
        "link-tep\ntep.txt\nlink-thu-muc\nthu-muc\nlink-missing\nkhong-ton-tai\nlink-tuyet-doi\n{tep}\ntep.txt\nerr\nthu-muc\nerr\nkhong-co\nerr\nfalse\n"
    );
    dung(&d, "dich", src, &ky_vong);
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 2 — **không** đi theo link (đọc đích của chính link, không phải của chuỗi link)
// ===========================================================================

/// Chuỗi `link2 → link1 → tep.txt`: `read_link("link2")` trả `link1` (đích **trực tiếp**), không
/// trả `tep.txt`; trong khi `fs.read("link2")` vẫn đi theo cả chuỗi và đọc nội dung tệp.
#[test]
fn khong_di_theo_link() {
    let d = tam("khong-theo");
    std::fs::write(d.join("tep.txt"), "NOI-DUNG").expect("ghi tệp");
    if let Err(e) = tao_link_tep(Path::new("tep.txt"), &d.join("link1")) {
        eprintln!("bỏ qua: nền không tạo được symlink ({e})");
        return;
    }
    if let Err(e) = tao_link_tep(Path::new("link1"), &d.join("link2")) {
        eprintln!("bỏ qua: nền không tạo được symlink chuỗi ({e})");
        return;
    }

    let src = r#"use std.fs

fn main():
    match fs.read_link("link2"):
        Ok(t):
            print(t)
        Err(e):
            print("err")
    match fs.read("link2"):
        Ok(s):
            print(s)
        Err(e):
            print("read-err")
    print(fs.is_symlink("link2"))
"#;
    dung(&d, "khong-theo", src, "link1\nNOI-DUNG\nOk(true)\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 3 — tệp/thư mục thường/thiếu ⇒ Err(Error) dữ liệu, không trap
// ===========================================================================

/// `read_link` chỉ hợp lệ trên symlink: tệp thường, thư mục thường, path thiếu ⇒ `Err` dữ liệu;
/// chương trình in `err` rồi **tiếp tục**, không trap/exit khác 0.
#[test]
fn tep_thu_muc_thieu_la_err_khong_trap() {
    let d = tam("khong-phai-link");
    std::fs::write(d.join("tep.txt"), "x").expect("ghi tệp");
    std::fs::create_dir_all(d.join("thu-muc")).expect("mkdir");
    let src = r#"use std.fs

fn main():
    for p in ["tep.txt", "thu-muc", "khong-co"]:
        match fs.read_link(p):
            Ok(t):
                print("ok")
            Err(e):
                print("err")
    print("xong")
"#;
    dung(&d, "khong-phai-link", src, "err\nerr\nerr\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `tep.txt/con` là `ENOTDIR` (không phải `NotFound`) ⇒ `Err(Error)` dữ liệu; in `err` rồi
/// **tiếp tục**, không trap.
#[test]
fn enotdir_la_err_khong_trap() {
    let d = tam("enotdir");
    std::fs::write(d.join("tep.txt"), "x").expect("ghi tệp");
    let src = r#"use std.fs

fn main():
    match fs.read_link("tep.txt/con"):
        Ok(t):
            print("ok")
        Err(e):
            print("err")
    print("xong")
"#;
    dung(&d, "enotdir", src, "err\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 4 — quyền: chỉ đọc trong cap[Dir]; ngoài root ⇒ VS0604
// ===========================================================================

/// `read_link` là thao tác **đọc** ⇒ chạy trong `cap[Dir]` **không** `(write)`; symlink và
/// dangling trong root đều đọc được, tệp thường vẫn `Err`.
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
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    for p in [\"{r}/link-tep\", \"{r}/link-missing\", \"{r}/tep.txt\"]:\n        match fs.read_link(p):\n            Ok(t):\n                print(t)\n            Err(e):\n                print(\"err\")\n    print(\"xong\")\n"
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
        assert_eq!(
            r1.1, "tep.txt\nkhong-co\nerr\nxong\n",
            "{args:?}: stdout lệch"
        );
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
    if let Err(e) = tao_link_tep(Path::new("../ngoai.txt"), &root.join("link-ra-ngoai")) {
        eprintln!("bỏ qua: nền không tạo được symlink ra ngoài ({e})");
        return;
    }
    let r = duong(&root);
    let mut chuong_trinh = Vec::new();
    chuong_trinh.push(format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match fs.read_link(\"{r}/../ngoai.txt\"):\n        Ok(t):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
    ));
    chuong_trinh.push(format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match fs.read_link(\"{r}/link-ra-ngoai\"):\n        Ok(t):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
    ));
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
// 5 — host giả lập: mock không biểu diễn symlink ⇒ luôn Err
// ===========================================================================

/// `--fs-mock` không có khái niệm symlink ⇒ `read_link` luôn `Err` cho tệp/thư mục/thiếu (không
/// có link để đọc); hai đường byte-identical.
#[test]
fn mock_host_luon_err() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "abc" }, "dirs": { "thu-muc": [] } }"#;
    let m = d.join("mock.json");
    std::fs::write(&m, mock).expect("ghi kịch bản mock");
    let src = r#"use std.fs

fn main():
    for p in ["a.txt", "thu-muc", "thieu"]:
        match fs.read_link(p):
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
    assert_eq!(o1, "err\nerr\nerr\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 6 — **không** sugar: module-only
// ===========================================================================

/// `p.read_link()` **không** phải sugar (giống `fs.read`): `str` không có trường `read_link` ⇒
/// `VS0403`, kể cả khi đã `use std.fs`.
#[test]
fn khong_sugar_module_only() {
    let d = tam("sugar");
    for (tag, src) in [
        (
            "sugar-literal",
            "fn main():\n    print(\"a\".read_link())\n",
        ),
        (
            "sugar-bien",
            "fn main():\n    let p = \"a\"\n    print(p.read_link())\n",
        ),
        (
            "sugar-co-use",
            "use std.fs\n\nfn main():\n    print(\"a\".read_link())\n",
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
        ("kieu-int", "fs.read_link(1)", "VS0412"),
        ("kieu-bool", "fs.read_link(true)", "VS0412"),
        ("arity-0", "fs.read_link()", "VS0401"),
        ("arity-2", "fs.read_link(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.fs\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(&d, tag, &src, ma);
    }
    let _ = std::fs::remove_dir_all(&d);
}
