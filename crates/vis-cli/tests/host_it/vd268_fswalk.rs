// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268** — `std.fs.walk(path) -> Result[list[str], Error]`: liệt kê **đệ quy** dưới `path`.
//!
//! Hợp đồng (đóng băng bằng test):
//! - đường dẫn trả về **tương đối** so với `path`, phân cách `/`;
//! - **thư mục** cũng được trả về, **kèm `/` ở cuối** (tệp không có `/`); mục ẩn vẫn gồm;
//! - **DFS**: tại mỗi thư mục, mục được sắp theo **byte của tên**; thư mục được in **ngay trước**
//!   nội dung của nó (thứ tự **không** phải sắp xếp toàn cục của chuỗi kết quả);
//! - **không đi theo symlink**: mục symlink bị **bỏ hoàn toàn** (không in, không đệ quy) ⇒ vòng
//!   symlink không treo, không lặp; bản thân `path` nếu là symlink thì theo (quyền kiểm trên đích
//!   canonicalize);
//! - lỗi dữ liệu (thiếu `path`, không phải thư mục, không đọc được thư mục con) ⇒ `Err(Error)`,
//!   **không** trap; thiếu `cap[Dir]` ⇒ `VS0604` (exit 2).
//!
//! RED-BEFORE (ghi lúc làm VD-268): trước khi có `fs.walk`, chương trình báo lỗi tĩnh
//! `VS0301`/không có hàm `walk` (exit 2) — xem báo cáo bàn giao.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd268-{tag}-{}-{n}", std::process::id()));
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
/// stdout/exit; trả stdout.
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

/// Như [`hai_duong`] nhưng cả hai đường chạy với `--fs-mock` (mỗi tiến trình nạp kịch bản mới).
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

/// Chương trình in **độ dài + từng mục** của `fs.walk(path)`.
fn ct_walk(p: &Path) -> String {
    format!(
        "use std.fs\nfn main() -> ():\n    match fs.walk(\"{}\"):\n        Ok(xs):\n            print(xs.len())\n            for x in xs:\n                print(x)\n        Err(e):\n            print(\"loi\")\n",
        duong(p)
    )
}

// ---------- cây lồng nhau: thứ tự + tương đối + thư mục kèm `/` + mục ẩn ----------

/// Cây: `.an.txt`, `a.txt`, `b/c.txt`, `b/rong/` (rỗng), `zz/y.txt`. Thứ tự DFS theo byte tên
/// từng cấp; đường dẫn **tương đối**; thư mục kèm `/`; mục ẩn vẫn có.
#[test]
fn cay_long_thu_tu_tuong_doi_va_thu_muc_kem_slash() {
    let d = tam("cay");
    let root = d.join("goc");
    std::fs::create_dir_all(root.join("b").join("rong")).expect("mkdir b/rong");
    std::fs::create_dir_all(root.join("zz")).expect("mkdir zz");
    std::fs::write(root.join(".an.txt"), "an").expect("ghi .an.txt");
    std::fs::write(root.join("a.txt"), "a").expect("ghi a.txt");
    std::fs::write(root.join("b").join("c.txt"), "c").expect("ghi b/c.txt");
    std::fs::write(root.join("zz").join("y.txt"), "y").expect("ghi zz/y.txt");

    let o = hai_duong(&d, "cay", &ct_walk(&root));
    assert_eq!(
        o, "7\n.an.txt\na.txt\nb/\nb/c.txt\nb/rong/\nzz/\nzz/y.txt\n",
        "thứ tự DFS + tương đối + `/` cuối thư mục + mục ẩn"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Thư mục gốc rỗng ⇒ `Ok([])`; thư mục con rỗng vẫn xuất hiện một mục `rong/`.
#[test]
fn thu_muc_rong() {
    let d = tam("rong");
    let root = d.join("goc");
    std::fs::create_dir_all(&root).expect("mkdir goc");
    let o = hai_duong(&d, "rong", &ct_walk(&root));
    assert_eq!(o, "0\n", "walk thư mục rỗng phải trả danh sách rỗng");

    std::fs::create_dir_all(root.join("rong")).expect("mkdir rong");
    let o = hai_duong(&d, "rong2", &ct_walk(&root));
    assert_eq!(o, "1\nrong/\n", "thư mục con rỗng vẫn có mục kèm `/`");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- symlink: bỏ hoàn toàn, không lặp ----------

/// Vòng symlink (`loop -> .`) + symlink tới thư mục + symlink tới tệp đều bị **bỏ**; kết quả
/// đúng bằng cây thật, không treo. Chỉ Unix có symlink.
#[cfg(unix)]
#[test]
fn symlink_loop_bi_bo_qua() {
    use std::os::unix::fs::symlink;
    let d = tam("sym");
    let root = d.join("goc");
    std::fs::create_dir_all(root.join("a")).expect("mkdir a");
    std::fs::write(root.join("a").join("f.txt"), "f").expect("ghi f");
    symlink(".", root.join("loop")).expect("symlink loop");
    symlink("a", root.join("link-thu-muc")).expect("symlink thư mục");
    symlink("a/f.txt", root.join("link-tep")).expect("symlink tệp");

    let o = hai_duong(&d, "sym", &ct_walk(&root));
    assert_eq!(
        o, "2\na/\na/f.txt\n",
        "symlink phải bị bỏ hoàn toàn (không in, không đệ quy)"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- lỗi dữ liệu: Err, không trap ----------

/// Đường dẫn thiếu ⇒ `Err(Error)` (in `loi`, exit 0); tệp (không phải thư mục) ⇒ `Err`.
#[test]
fn thieu_duong_dan_va_khong_phai_thu_muc_la_err() {
    let d = tam("loi");
    let thieu = d.join("khong-co");
    let o = hai_duong(&d, "thieu", &ct_walk(&thieu));
    assert_eq!(o, "loi\n", "thiếu path phải là Err dữ liệu");

    let tep = d.join("tep.txt");
    std::fs::write(&tep, "x").expect("ghi tệp");
    let o = hai_duong(&d, "tep", &ct_walk(&tep));
    assert_eq!(o, "loi\n", "walk trên tệp phải là Err dữ liệu");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- arity / kiểu: lỗi tĩnh, không vào host ----------

/// Sai kiểu đối số ⇒ `VS0412`; sai arity ⇒ `VS0401`; exit 2 ở cả hai đường (lỗi tĩnh trước khi chạy).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    let d = tam("sig");
    for (tag, call, ma) in [
        ("kieu", "fs.walk(1)", "VS0412"),
        ("arity", "fs.walk()", "VS0401"),
    ] {
        let src = format!("use std.fs\nfn main() -> ():\n    let x = {call}\n    print(\"x\")\n");
        let p = d.join(format!("{tag}.vis"));
        viet_tep(&p, &src);
        let r1 = vis(&[std::ffi::OsStr::new("run"), p.as_os_str()]);
        let r2 = vis(&[
            std::ffi::OsStr::new("run"),
            std::ffi::OsStr::new("--vm"),
            std::ffi::OsStr::new("--no-cache"),
            p.as_os_str(),
        ]);
        assert_eq!(r1.0, 2, "[{tag}] interp: {r1:?}");
        assert_eq!(r2.0, 2, "[{tag}] vm: {r2:?}");
        assert_eq!(r1.1, "", "[{tag}] không được chạy");
        assert_eq!(r1.1, r2.1, "[{tag}] stdout lệch");
        assert!(r1.2.contains(ma), "[{tag}] interp thiếu {ma}: {r1:?}");
        assert!(r2.2.contains(ma), "[{tag}] vm thiếu {ma}: {r2:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- quyền: root ngoài cap ⇒ VS0604 ----------
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

/// `cap[Dir]` chỉ cấp `root`; `walk(root/../ngoai)` ⇒ `VS0604`, exit 2, **không** in tên nào.
#[test]
fn walk_ngoai_quyen_bi_chan() {
    let d = tam("quyen");
    let root = d.join("root");
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::create_dir_all(&ngoai).expect("ngoai");
    std::fs::write(ngoai.join("bi-mat.txt"), "x").expect("tệp ngoài");
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{}\"):\n    match fs.walk(\"{}/../ngoai\"):\n        Ok(xs):\n            print(xs.len())\n        Err(e):\n            print(\"ERR\")\n",
        duong(&root),
        duong(&root)
    );
    viet_tep(&d.join("f.vis"), &src);
    let r1 = chay(&d, &["run", "f.vis"]);
    let r2 = chay(&d, &["run", "--vm", "--no-cache", "f.vis"]);
    assert_eq!(r1.0, 2, "interp phải exit 2: {r1:?}");
    assert_eq!(r2.0, 2, "vm phải exit 2: {r2:?}");
    assert_eq!(r1.1, "", "không được in kết quả ngoài quyền");
    assert_eq!(r1.1, r2.1, "stdout lệch");
    for (duong_chay, r) in [("interp", &r1), ("vm", &r2)] {
        assert!(r.2.contains("VS0604"), "{duong_chay}: thiếu VS0604: {r:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- host giả lập ----------

/// `--fs-mock`: cùng hợp đồng (DFS, tương đối, thư mục kèm `/`), hai đường byte-identical.
#[test]
fn mock_host_tat_dinh() {
    let d = tam("mock");
    let mock = r#"{ "dirs": {
        ".": [
            {"name": "b.txt", "is_file": true},
            {"name": "a", "is_file": false}
        ],
        "./a": [
            {"name": "c.txt", "is_file": true},
            {"name": "rong", "is_file": false}
        ]
    }, "files": { "b.txt": "", "./a/c.txt": "" } }"#;
    let o = hai_duong_mock(&d, "mock", mock, &ct_walk(Path::new(".")));
    assert_eq!(o, "4\na/\na/c.txt\na/rong/\nb.txt\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--fs-mock`: root không có trong kịch bản ⇒ `Err` dữ liệu, không trap.
#[test]
fn mock_host_thieu_root_la_err() {
    let d = tam("mock2");
    let o = hai_duong_mock(
        &d,
        "mock2",
        r#"{ "dirs": { ".": [] } }"#,
        &ct_walk(Path::new("khong-co")),
    );
    assert_eq!(o, "loi\n");
    let _ = std::fs::remove_dir_all(&d);
}
