// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (fs-current-dir)** — `std.fs.current_dir() -> Result[str, Error]`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - **không tham số** (params rỗng), trả **thư mục làm việc hiện tại** dưới dạng **đường dẫn
//!   tuyệt đối** chuỗi UTF-8 — đúng `std::env::current_dir`; kết quả phải **tồn tại** và
//!   `path.is_absolute` ⇒ `true`;
//! - **không** có đường dẫn người dùng ⇒ **không** đi qua `cap[Dir]`/`FsGuard` (không kiểm root,
//!   không cần `(read)`); chỉ đọc môi trường, **không** side effect;
//! - cwd bị **xoá**, **không truy cập được** hoặc **không phải UTF-8** ⇒ `Err(Error)` **dữ liệu**,
//!   **không** trap; chương trình in `err` rồi **tiếp tục** (exit 0);
//! - **không sugar** (module-only, như mọi hàm `fs`): `p.current_dir()` ⇒ `VS0403`; sai kiểu ⇒
//!   `VS0412`; sai arity (`fs.current_dir(1)`) ⇒ `VS0401`;
//! - host thật (`ngoai.rs`), `--fs-mock` (luôn `Err` **tất định** — kịch bản mock không có `cwd`/
//!   môi trường, cùng lý do `canonicalize`/`read_link` luôn `Err`), interp và VM dùng **cùng** một
//!   ngữ nghĩa byte-identical.
//!
//! Ca cwd bị xoá/không UTF-8 dùng `sh` để `chdir` vào thư mục rồi xoá/đặt tên byte không UTF-8
//! **trước** khi `exec vis` — chỉ chạy trên Unix; thiếu `sh` ⇒ bỏ qua mềm, không đỏ. Các ca còn
//! lại chạy mọi nền.
//!
//! RED-BEFORE (ghi lúc làm VD-270, trước khi thêm hàm — cây `17b31cb`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_fs_current_dir
//! running 7 tests
//! test khong_sugar_module_only ... ok
//! test cwd_bi_xoa_la_err_khong_trap ... FAILED
//! test cwd_khong_utf8_la_err_khong_trap ... FAILED
//! test goi_that_tra_tuyet_doi_va_ton_tai ... FAILED
//! test kieu_tra_khop_result_str_error ... FAILED
//! test mock_luon_err_tat_dinh ... FAILED
//! test sai_kieu_sai_arity_bi_chan ... FAILED
//! test result: FAILED. 1 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out
//! # 6 ca đỏ `VS0301` «module `std.fs` không có hàm `current_dir`»
//! # (ca sai kiểu/arity cũng đỏ VS0301 vì hàm chưa tồn tại, chưa tới VS0412/VS0401;
//! #  ca no-sugar đã xanh đúng VS0403 «`str` không có trường `current_dir`»)
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-current-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// Đường dẫn tuyệt đối **đã canonicalize** trên phía test — `std::env::current_dir` của tiến
/// trình con (chạy với `current_dir(d)`) trả đúng dạng này trên Unix (`getcwd` phân giải symlink).
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
/// cwd (để `current_dir` trả cùng một kết quả), đòi byte-identical stdout/stderr/exit;
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
// 1 — gọi thật: tuyệt đối, tồn tại, khớp `std::env::current_dir` của tiến trình con
// ===========================================================================

/// `fs.current_dir()` trả **tuyệt đối**, trỏ tới thư mục **tồn tại** (`fs.is_dir` ⇒ `Ok(true)`),
/// và **đúng** cwd mà test đặt cho tiến trình con (`Command::current_dir`). Chạy cả hai engine,
/// cùng một cwd, kết quả byte-identical.
#[test]
fn goi_that_tra_tuyet_doi_va_ton_tai() {
    let d = tam("that");
    // Windows: `fs.current_dir` chuyển tiếp `std::env::current_dir` — dạng `C:\…` (không verbatim
    // `\\?\`). Unix: canonicalize để so khớp đường dẫn thật (temp có thể qua symlink).
    let mong = if cfg!(windows) {
        d.display().to_string()
    } else {
        canon(&d)
    };
    let src = r#"use std.fs
use std.path

fn main():
    match fs.current_dir():
        Ok(p):
            print(p)
            print(path.is_absolute(p))
            match fs.is_dir(p):
                Ok(b):
                    print(b)
                Err(e):
                    print("err-dir")
        Err(e):
            print("err")
"#;
    let ky_vong = format!("{mong}\ntrue\ntrue\n");
    dung(&d, "that", src, &ky_vong);
    let _ = std::fs::remove_dir_all(&d);
}

/// Chữ ký `() -> Result[str, Error]`: trả thẳng từ hàm người dùng khai `Result[str, Error]` được,
/// `match Ok/Err` chạy sạch — khoá kiểu trả thật, không phải `str` trần.
#[test]
fn kieu_tra_khop_result_str_error() {
    let d = tam("kieu-tra");
    let src = r#"use std.fs

fn lay() -> Result[str, Error]:
    return fs.current_dir()

fn main():
    match lay():
        Ok(p):
            print("ok")
        Err(e):
            print("err")
"#;
    dung(&d, "kieu-tra", src, "ok\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 2 — mock: `--fs-mock` không có cwd/môi trường ⇒ luôn `Err`, tất định
// ===========================================================================

/// Kịch bản `--fs-mock` **không** có `cwd`/môi trường thật ⇒ `fs.current_dir()` **luôn** `Err`
/// dữ liệu, **tất định** (không đọc cwd thật của tiến trình), kể cả khi kịch bản có `files`/`dirs`;
/// `Err` được bắt và chương trình tiếp tục. Hai đường byte-identical.
#[test]
fn mock_luon_err_tat_dinh() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "abc" }, "dirs": { "thu-muc": [] } }"#;
    let m = d.join("mock.json");
    std::fs::write(&m, mock).expect("ghi kịch bản mock");
    let src = r#"use std.fs

fn main():
    for i in [0, 1]:
        match fs.current_dir():
            Ok(p):
                print(p)
            Err(e):
                print("err")
    print("xong")
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
    assert_eq!(e1, e2, "stderr hai đường lệch");
    assert_eq!(o1, "err\nerr\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 3 — **không** sugar: module-only (params rỗng, không receiver)
// ===========================================================================

/// `current_dir` **không** phải sugar (giống mọi hàm `fs`): `str` không có trường `current_dir`
/// ⇒ `VS0403`, kể cả khi đã `use std.fs`; gọi đúng dạng `fs.current_dir()` mới là hàm.
#[test]
fn khong_sugar_module_only() {
    let d = tam("sugar");
    for (tag, src) in [
        (
            "sugar-literal",
            "fn main():\n    print(\"a\".current_dir())\n",
        ),
        (
            "sugar-bien",
            "fn main():\n    let p = \"a\"\n    print(p.current_dir())\n",
        ),
        (
            "sugar-co-use",
            "use std.fs\n\nfn main():\n    print(\"a\".current_dir())\n",
        ),
    ] {
        tu_choi(&d, tag, src, "VS0403");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 4 — ca âm tĩnh: sai kiểu/sai arity
// ===========================================================================

/// `fs.current_dir(1)` (và 2 đối số) ⇒ `VS0401` — **params rỗng**; gán kết quả `Result[str, Error]`
/// cho `str` ⇒ `VS0412`.
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    let d = tam("kieu");
    for (tag, body, ma) in [
        (
            "arity-1",
            "let x = fs.current_dir(1)\n    print(x)",
            "VS0401",
        ),
        (
            "arity-2",
            "let x = fs.current_dir(\"a\", \"b\")\n    print(x)",
            "VS0401",
        ),
        (
            "kieu-str",
            "let x: str = fs.current_dir()\n    print(x)",
            "VS0412",
        ),
    ] {
        let src = format!("use std.fs\n\nfn main():\n    {body}\n");
        tu_choi(&d, tag, &src, ma);
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ===========================================================================
// 5 — cwd bị xoá / không phải UTF-8 ⇒ Err dữ liệu, không trap (Unix, `sh`)
// ===========================================================================

/// `sh` có sẵn? Dùng để `chdir` vào thư mục rồi xoá/đặt tên byte lạ **trước** khi `exec vis`.
#[cfg(unix)]
fn co_sh() -> bool {
    Command::new("sh")
        .args(["-c", "exit 0"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Chạy `vis` qua `sh -c 'cd "$1" && <bien_doi> && exec "$2" run --no-cache "$3"'` — `bien_doi`
/// chạy **sau** `cd`, **trước** `exec`, nên tiến trình `vis` khởi động với cwd đã bị xoá/đổi tên.
#[cfg(unix)]
fn chay_sau_khi_doi_cwd(
    bien_doi: &str,
    cwd: &std::ffi::OsStr,
    prog: &Path,
    vm: bool,
) -> (i32, String, String) {
    let vis = env!("CARGO_BIN_EXE_vis");
    let lenh = format!(
        "cd \"$1\" && {bien_doi} && exec \"$2\" run --no-cache {} \"$3\"",
        if vm { "--vm" } else { "" }
    );
    let o = Command::new("sh")
        .args([
            std::ffi::OsStr::new("-c"),
            std::ffi::OsStr::new(&lenh),
            std::ffi::OsStr::new("sh"),
            cwd,
            std::ffi::OsStr::new(vis),
            prog.as_os_str(),
        ])
        .output()
        .expect("chạy sh");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// cwd bị **xoá** sau `chdir`, trước `exec vis` ⇒ `std::env::current_dir` lỗi (`ENOENT`) ⇒
/// `Err(Error)` dữ liệu, chương trình in `err` rồi **tiếp tục** (exit 0), không trap. Hai engine
/// byte-identical.
#[cfg(unix)]
#[test]
fn cwd_bi_xoa_la_err_khong_trap() {
    if !co_sh() {
        eprintln!("bỏ qua: không có `sh`");
        return;
    }
    let d = tam("cwd-xoa");
    let cwd1 = d.join("cwd-interp");
    let cwd2 = d.join("cwd-vm");
    std::fs::create_dir_all(&cwd1).expect("mkdir cwd interp");
    std::fs::create_dir_all(&cwd2).expect("mkdir cwd vm");
    let prog = d.join("p.vis");
    viet_tep(
        &prog,
        r#"use std.fs

fn main():
    match fs.current_dir():
        Ok(p):
            print("ok")
        Err(e):
            print("err")
    print("xong")
"#,
    );
    let (c1, o1, e1) = chay_sau_khi_doi_cwd("rmdir \"$1\"", cwd1.as_os_str(), &prog, false);
    let (c2, o2, e2) = chay_sau_khi_doi_cwd("rmdir \"$1\"", cwd2.as_os_str(), &prog, true);
    assert_eq!(c1, 0, "interp: exit {c1}\nstderr: {e1}");
    assert_eq!(c2, 0, "vm: exit {c2}\nstderr: {e2}");
    assert_eq!(o1, "err\nxong\n", "interp stdout: {o1}\nstderr: {e1}");
    assert_eq!(o2, o1, "vm stdout lệch\nstderr: {e2}");
    assert_eq!(e1, e2, "stderr hai đường lệch");
    let _ = std::fs::remove_dir_all(&d);
}

/// cwd **không phải UTF-8** (tên thư mục chứa byte `0xFF` trên Unix) ⇒ host **không** cắt cụt
/// bằng lossy mà trả `Err(Error)` dữ liệu, chương trình tiếp tục. Hai engine byte-identical.
#[cfg(unix)]
#[test]
fn cwd_khong_utf8_la_err_khong_trap() {
    use std::os::unix::ffi::OsStrExt;

    if !co_sh() {
        eprintln!("bỏ qua: không có `sh`");
        return;
    }
    let d = tam("cwd-khong-utf8");
    let ten = std::ffi::OsStr::from_bytes(b"cwd-\xff");
    let cwd = d.join(ten);
    std::fs::create_dir_all(&cwd).expect("mkdir cwd không UTF-8");
    let prog = d.join("p.vis");
    viet_tep(
        &prog,
        r#"use std.fs

fn main():
    match fs.current_dir():
        Ok(p):
            print("ok")
        Err(e):
            print("err")
    print("xong")
"#,
    );
    let (c1, o1, e1) = chay_sau_khi_doi_cwd("true", cwd.as_os_str(), &prog, false);
    let (c2, o2, e2) = chay_sau_khi_doi_cwd("true", cwd.as_os_str(), &prog, true);
    assert_eq!(c1, 0, "interp: exit {c1}\nstderr: {e1}");
    assert_eq!(c2, 0, "vm: exit {c2}\nstderr: {e2}");
    assert_eq!(o1, "err\nxong\n", "interp stdout: {o1}\nstderr: {e1}");
    assert_eq!(o2, o1, "vm stdout lệch\nstderr: {e2}");
    assert_eq!(e1, e2, "stderr hai đường lệch");
    let _ = std::fs::remove_dir_all(&d);
}
