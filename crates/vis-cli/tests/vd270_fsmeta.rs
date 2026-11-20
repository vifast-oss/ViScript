// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — metadata **chỉ đọc** của `std.fs`: `size`, `is_file`, `is_dir`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - `fs.size(path) -> Result[int, Error]`: số **byte** của **tệp thường**; thư mục, symlink,
//!   thiếu path (và loại không phải tệp thường) ⇒ `Err(Error)` dữ liệu, **không** trap;
//! - `fs.is_file(path)` / `fs.is_dir(path) -> Result[bool, Error]`: **đúng loại** mới `true`; path
//!   **thiếu** ⇒ `Ok(false)`; **symlink** ⇒ `false`/`false` ở **cả hai** (dùng `symlink_metadata`,
//!   **không** đi theo symlink), `size` của symlink ⇒ `Err`;
//! - quyền: cả ba là thao tác **đọc** ⇒ chỉ cần `cap[Dir]`/`cap[File]` (không cần `(write)`); ngoài
//!   root ⇒ `VS0604`, exit 2, không in kết quả;
//! - host thật (`ngoai.rs`), `--fs-mock`, interp và VM dùng **cùng** một ngữ nghĩa.
//!
//! RED-BEFORE (ghi lúc làm VD-270): trước khi thêm ba hàm, chương trình báo lỗi tĩnh
//! «không có hàm `size`/`is_file`/`is_dir`» (`VS0301`, exit 2) — xem báo cáo bàn giao.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd270-{tag}-{}-{n}", std::process::id()));
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
/// stdout/exit; trả stdout. Chương trình phải **chỉ đọc** (chạy lặp được).
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

/// Như [`hai_duong`] nhưng cả hai đường chạy với `--fs-mock`.
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

// ---------- size: byte, biên, đúng loại ----------

/// `size` trả **số byte** (không phải số ký tự): tệp rỗng `0`, một byte `1`, `"ế"` (UTF-8) `3`,
/// và một tệp nhị phân 4 byte. Cả hai đường khớp.
#[test]
fn size_dem_byte_va_bien_rong_mot() {
    let d = tam("size");
    let rong = d.join("rong.txt");
    std::fs::write(&rong, "").expect("ghi rỗng");
    let mot = d.join("mot.txt");
    std::fs::write(&mot, "A").expect("ghi một byte");
    let utf8 = d.join("utf8.txt");
    std::fs::write(&utf8, "ế").expect("ghi UTF-8");
    let nhi = d.join("nhi.bin");
    std::fs::write(&nhi, [0x00, 0xff, 0x80, b'\n']).expect("ghi nhị phân");

    let o = hai_duong(
        &d,
        "size",
        &format!(
            "use std.fs\nfn main() -> ():\n    for p in [\"{}\", \"{}\", \"{}\", \"{}\"]:\n        match fs.size(p):\n            Ok(n):\n                print(n)\n            Err(e):\n                print(\"err\")\n",
            duong(&rong),
            duong(&mot),
            duong(&utf8),
            duong(&nhi)
        ),
    );
    assert_eq!(o, "0\n1\n3\n4\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `size` chỉ nhận **tệp thường**: thư mục ⇒ `Err`, path thiếu ⇒ `Err`; không trap (chương trình
/// in `err` rồi tiếp tục).
#[test]
fn size_thu_muc_va_thieu_la_err() {
    let d = tam("size-err");
    let thu_muc = d.join("thu-muc");
    std::fs::create_dir_all(&thu_muc).expect("mkdir");
    let thieu = d.join("khong-co.txt");

    let o = hai_duong(
        &d,
        "size-err",
        &format!(
            "use std.fs\nfn main() -> ():\n    match fs.size(\"{}\"):\n        Ok(n):\n            print(\"dir-ok\")\n        Err(e):\n            print(\"dir-err\")\n    match fs.size(\"{}\"):\n        Ok(n):\n            print(\"thieu-ok\")\n        Err(e):\n            print(\"thieu-err\")\n    print(\"xong\")\n",
            duong(&thu_muc),
            duong(&thieu)
        ),
    );
    assert_eq!(o, "dir-err\nthieu-err\nxong\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- predicate: đúng loại, thiếu ⇒ false ----------

/// `is_file`/`is_dir` phân biệt **tệp thường** và **thư mục**; path thiếu ⇒ `false`/`false`
/// (không phải `Err`).
#[test]
fn predicate_tep_thu_muc_va_thieu() {
    let d = tam("pred");
    let tep = d.join("tep.txt");
    std::fs::write(&tep, "x").expect("ghi tệp");
    let thu_muc = d.join("thu-muc");
    std::fs::create_dir_all(&thu_muc).expect("mkdir");
    let thieu = d.join("khong-co");

    let o = hai_duong(
        &d,
        "pred",
        &format!(
            "use std.fs\nfn main() -> ():\n    for p in [\"{tep}\", \"{dir}\", \"{thieu}\"]:\n        match fs.is_file(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n        match fs.is_dir(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n",
            tep = duong(&tep),
            dir = duong(&thu_muc),
            thieu = duong(&thieu)
        ),
    );
    assert_eq!(o, "true\nfalse\nfalse\ntrue\nfalse\nfalse\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Không đi theo symlink**: symlink tới tệp ⇒ `is_file=false`, `is_dir=false`, `size=Err`;
/// symlink tới thư mục ⇒ `is_file=false`, `is_dir=false`, `size=Err`. Đích thật vẫn đọc được
/// (chứng minh symlink hợp lệ). Chỉ Unix có symlink.
#[cfg(unix)]
#[test]
fn symlink_khong_phai_tep_cung_khong_phai_thu_muc() {
    use std::os::unix::fs::symlink;
    let d = tam("sym");
    let tep = d.join("tep.txt");
    std::fs::write(&tep, "NOI-DUNG").expect("ghi tệp");
    let thu_muc = d.join("thu-muc");
    std::fs::create_dir_all(&thu_muc).expect("mkdir");
    let link_tep = d.join("link-tep");
    symlink(&tep, &link_tep).expect("symlink tệp");
    let link_thu_muc = d.join("link-thu-muc");
    symlink(&thu_muc, &link_thu_muc).expect("symlink thư mục");

    let o = hai_duong(
        &d,
        "sym",
        &format!(
            "use std.fs\nfn main() -> ():\n    for p in [\"{}\", \"{}\"]:\n        match fs.is_file(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n        match fs.is_dir(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n        match fs.size(p):\n            Ok(n):\n                print(n)\n            Err(e):\n                print(\"size-err\")\n    match fs.read(\"{}\"):\n        Ok(s):\n            print(s)\n        Err(e):\n            print(\"read-err\")\n",
            duong(&link_tep),
            duong(&link_thu_muc),
            duong(&link_tep)
        ),
    );
    assert_eq!(
        o, "false\nfalse\nsize-err\nfalse\nfalse\nsize-err\nNOI-DUNG\n",
        "symlink không được đi theo: cả hai predicate false, size Err"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- host giả lập ----------

/// `--fs-mock`: `size` = số **byte** của chuỗi trong kịch bản; `is_file`/`is_dir` theo `files`/
/// `dirs`; thư mục/thiếu ⇒ `size=Err`, thiếu ⇒ predicate `false`; hai đường byte-identical.
#[test]
fn mock_host_metadata() {
    let d = tam("mock");
    let mock = r#"{ "files": { "a.txt": "abc", "utf8.txt": "ế" }, "dirs": { "thu-muc": [] } }"#;
    let o = hai_duong_mock(
        &d,
        "mock",
        mock,
        "use std.fs\nfn main() -> ():\n    for p in [\"a.txt\", \"utf8.txt\", \"thu-muc\", \"thieu\"]:\n        match fs.is_file(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n        match fs.is_dir(p):\n            Ok(b):\n                print(b)\n            Err(e):\n                print(\"err\")\n        match fs.size(p):\n            Ok(n):\n                print(n)\n            Err(e):\n                print(\"size-err\")\n",
    );
    assert_eq!(
        o,
        "true\nfalse\n3\ntrue\nfalse\n3\nfalse\ntrue\nsize-err\nfalse\nfalse\nsize-err\n"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quyền: chỉ đọc, ngoài root ⇒ VS0604 ----------

/// Ba helper **chỉ đọc** hoạt động trong `cap[Dir]` **không** `(write)`: không đòi quyền ghi.
#[test]
fn quyen_doc_trong_root_khong_can_write() {
    let d = tam("quyen-doc");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "abc").expect("tệp");
    let r = duong(&root);
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match fs.size(\"{r}/a.txt\"):\n        Ok(n):\n            print(n)\n        Err(e):\n            print(\"err\")\n    match fs.is_file(\"{r}/a.txt\"):\n        Ok(b):\n            print(b)\n        Err(e):\n            print(\"err\")\n    match fs.is_dir(\"{r}\"):\n        Ok(b):\n            print(b)\n        Err(e):\n            print(\"err\")\n"
    );
    viet_tep(&d.join("f.vis"), &src);
    for args in [
        vec!["run", "f.vis"],
        vec!["run", "--vm", "--no-cache", "f.vis"],
    ] {
        let r1 = chay(&d, &args);
        assert_eq!(r1.0, 0, "{args:?} phải chạy được: {r1:?}");
        assert_eq!(r1.1, "3\ntrue\ntrue\n", "{args:?}: stdout lệch");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Path **ngoài** `cap[Dir]` ⇒ `VS0604` (exit 2) cho **cả ba** helper, không in kết quả; hai đường
/// cùng kết quả. Đây là "permission denied" của tầng quyền, không phải `Err` dữ liệu.
#[test]
fn quyen_ngoai_root_bi_chan() {
    let d = tam("quyen-ngoai");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(d.join("ngoai.txt"), "bi mat").expect("tệp ngoài");
    let r = duong(&root);
    let ops = [
        format!("fs.size(\"{r}/../ngoai.txt\")"),
        format!("fs.is_file(\"{r}/../ngoai.txt\")"),
        format!("fs.is_dir(\"{r}/..\")"),
    ];
    for (i, op) in ops.iter().enumerate() {
        let src = format!(
            "use std.fs\n\nfn main(r: cap[Dir] @ p\"{r}\"):\n    match {op}:\n        Ok(v):\n            print(\"OK\")\n        Err(e):\n            print(\"ERR\")\n"
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
                rr.2.contains("cap[Dir]"),
                "ca {i} ({duong_chay}): thiếu `cap[Dir]`: {rr:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}
