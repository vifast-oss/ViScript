// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268** — `std.fs.glob(root, pattern) -> Result[list[str], Error]`: **lọc** kết quả
//! `fs.walk(root)` bằng mẫu glob **thuần** (cùng ngữ nghĩa cho host thật, `--fs-mock`, interp, VM).
//!
//! Hợp đồng (đóng băng bằng test):
//! - kết quả giữ **nguyên dạng** của `walk`: tương đối so với `root`, **thư mục kèm `/`**;
//! - **sắp theo byte** của cả đường dẫn (không theo thứ tự DFS của `walk`), **không** trùng;
//! - mẫu **không có** `/` ⇒ khớp **basename** ở mọi tầng (`*.txt` khớp cả `b/c.txt`); mục ẩn vẫn
//!   khớp (`*` khớp cả `.an.txt`);
//! - mẫu **có** `/` ⇒ khớp **toàn bộ** đường dẫn tương đối. Mục được thử ở dạng **không** có `/`
//!   cuối; **thư mục** còn được thử dạng **có** `/` cuối **chỉ khi** mẫu kết thúc bằng `/` (chọn
//!   thư mục) hoặc bằng `/**` (`**` cuối khớp rỗng, gồm chính thư mục: `b/**` ⇒ có `b/`). Nhờ vậy
//!   `b/*` chọn **con** của `b` chứ không chọn chính `b/`; kết quả vẫn in thư mục kèm `/`;
//! - `*` = không/nhiều ký tự trừ `/`; `?` = **một ký tự** (Unicode) trừ `/`; `**` = không/nhiều
//!   ký tự **gồm** `/` và **chỉ** hợp lệ khi là một **thành phần đầy đủ** của mẫu:
//!   `**/` khớp **không hoặc nhiều** thư mục (`**/*.txt` khớp cả `a.txt` ở gốc), `**` ở cuối khớp
//!   **mọi thứ còn lại** (kể cả rỗng ⇒ `b/**` khớp chính `b/`); `a**b`, `***` ⇒ `Err`;
//! - `[`, `]`, `{`, `}`, `\` là **ký tự thường** (không lớp ký tự, không escape);
//! - mẫu rỗng, mẫu tuyệt đối (bắt đầu `/`), mẫu quá dài ⇒ `Err(Error)` **dữ liệu** (không trap);
//! - **không** có đường tắt hệ thống tệp: `root` vẫn đi qua `walk`/`FsGuard` trước ⇒ root ngoài
//!   `cap[Dir]` là `VS0604` (exit 2), symlink vẫn bị `walk` bỏ hoàn toàn.
//!
//! RED-BEFORE (ghi lúc làm VD-268 trên HEAD `a76c945`): `fs.glob` chưa có ⇒ chương trình báo lỗi
//! tĩnh `VS0301: module std.fs không có hàm glob`, exit 2, ở **cả** interp và `--vm --no-cache`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

/// Thư mục tạm riêng cho từng ca (test chạy song song).
fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd268-glob-{tag}-{}-{n}", std::process::id()));
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

/// Chương trình in **độ dài + từng mục** của `fs.glob(root, pattern)`.
fn ct_glob(root: &Path, mau: &str) -> String {
    format!(
        "use std.fs\nfn main() -> ():\n    match fs.glob(\"{}\", \"{mau}\"):\n        Ok(xs):\n            print(xs.len())\n            for x in xs:\n                print(x)\n        Err(e):\n            print(\"loi\")\n",
        duong(root)
    )
}

/// Cây dùng chung cho các ca lọc:
/// `.an.txt`, `a.txt`, `đ.txt` (hai byte UTF-8), `x`, `b/c.txt`, `b/rong/`, `zz/y.txt`.
fn cay(d: &Path) -> PathBuf {
    let root = d.join("goc");
    std::fs::create_dir_all(root.join("b").join("rong")).expect("mkdir b/rong");
    std::fs::create_dir_all(root.join("zz")).expect("mkdir zz");
    std::fs::write(root.join(".an.txt"), "an").expect("ghi .an.txt");
    std::fs::write(root.join("a.txt"), "a").expect("ghi a.txt");
    std::fs::write(root.join("đ.txt"), "d").expect("ghi đ.txt");
    std::fs::write(root.join("x"), "x").expect("ghi x");
    std::fs::write(root.join("b").join("c.txt"), "c").expect("ghi b/c.txt");
    std::fs::write(root.join("zz").join("y.txt"), "y").expect("ghi zz/y.txt");
    root
}

// ---------- basename: mẫu không có `/` khớp mọi tầng, gồm mục ẩn, sắp theo byte ----------

/// `*.txt` khớp **basename** ở mọi tầng: gồm mục ẩn `.an.txt` và file trong thư mục con; kết quả
/// sắp theo **byte** (`.` < chữ ASCII < `đ` UTF-8), không theo DFS của `walk`.
#[test]
fn sao_txt_khop_basename_moi_tang() {
    let d = tam("ten");
    let root = cay(&d);
    let o = hai_duong(&d, "ten", &ct_glob(&root, "*.txt"));
    assert_eq!(
        o, "5\n.an.txt\na.txt\nb/c.txt\nzz/y.txt\nđ.txt\n",
        "basename mọi tầng + mục ẩn + sắp theo byte"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `?` = **một ký tự** Unicode trừ `/`: `?.txt` khớp mọi basename **một ký tự** + `.txt`, gồm
/// `a.txt`, `b/c.txt`, `zz/y.txt` và `đ.txt` (một `char`, không phải một byte); mẫu `?` khớp
/// basename một ký tự, gồm **thư mục** `b/`.
#[test]
fn hoi_cham_mot_ky_tu_unicode() {
    let d = tam("hoi");
    let root = cay(&d);
    let o = hai_duong(&d, "hoi1", &ct_glob(&root, "?.txt"));
    assert_eq!(
        o, "4\na.txt\nb/c.txt\nzz/y.txt\nđ.txt\n",
        "`?` phải khớp một ký tự Unicode"
    );

    let o = hai_duong(&d, "hoi2", &ct_glob(&root, "?"));
    assert_eq!(o, "2\nb/\nx\n", "`?` khớp basename một ký tự, có thư mục");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- mẫu có `/`: khớp toàn đường dẫn tương đối ----------

/// `b/*.txt` chỉ trong `b/`; `b/*` chọn cả tệp **và thư mục con** (`b/rong/` — thử thêm dạng
/// không có `/` cuối, kết quả vẫn in kèm `/`).
#[test]
fn mau_co_slash_khop_duong_dan() {
    let d = tam("slash");
    let root = cay(&d);
    let o = hai_duong(&d, "slash1", &ct_glob(&root, "b/*.txt"));
    assert_eq!(o, "1\nb/c.txt\n");

    let o = hai_duong(&d, "slash2", &ct_glob(&root, "b/*"));
    assert_eq!(
        o, "2\nb/c.txt\nb/rong/\n",
        "`b/*` chọn được thư mục con, kết quả vẫn kèm `/`"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `**/` khớp **không hoặc nhiều** thư mục ⇒ `**/*.txt` khớp cả tệp ở gốc lẫn trong thư mục con.
#[test]
fn sao_sao_slash_khop_khong_hoac_nhieu_thu_muc() {
    let d = tam("sau");
    let root = cay(&d);
    let o = hai_duong(&d, "sau", &ct_glob(&root, "**/*.txt"));
    assert_eq!(o, "5\n.an.txt\na.txt\nb/c.txt\nzz/y.txt\nđ.txt\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `**` ở **cuối** khớp mọi thứ còn lại, **kể cả rỗng**: `b/**` gồm chính `b/` + toàn bộ cây con.
#[test]
fn sao_sao_cuoi_khop_moi_thu_con_lai() {
    let d = tam("cuoi");
    let root = cay(&d);
    let o = hai_duong(&d, "cuoi", &ct_glob(&root, "b/**"));
    assert_eq!(
        o, "3\nb/\nb/c.txt\nb/rong/\n",
        "`**` cuối khớp cả rỗng (chính `b/`)"
    );

    let o = hai_duong(&d, "cuoi2", &ct_glob(&root, "**"));
    assert_eq!(
        o, "9\n.an.txt\na.txt\nb/\nb/c.txt\nb/rong/\nx\nzz/\nzz/y.txt\nđ.txt\n",
        "`**` không có `/` khớp basename mọi mục (gồm thư mục)"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `*/` chỉ chọn **thư mục** ở gốc — dấu `/` cuối là ký tự thường phải khớp.
#[test]
fn slash_cuoi_chi_chon_thu_muc() {
    let d = tam("dirmau");
    let root = cay(&d);
    let o = hai_duong(&d, "dirmau", &ct_glob(&root, "*/"));
    assert_eq!(o, "2\nb/\nzz/\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// Không khớp ⇒ `Ok([])` (in `0`), không phải lỗi.
#[test]
fn khong_khop_la_ok_rong() {
    let d = tam("rong");
    let root = cay(&d);
    let o = hai_duong(&d, "rong", &ct_glob(&root, "*.md"));
    assert_eq!(o, "0\n");
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- symlink: walk bỏ hoàn toàn, glob không thấy ----------

/// Symlink tới tệp/thư mục và vòng `loop -> .` đều không xuất hiện và không nhân bản kết quả.
#[cfg(unix)]
#[test]
fn symlink_bi_bo_qua() {
    use std::os::unix::fs::symlink;
    let d = tam("sym");
    let root = cay(&d);
    symlink(".", root.join("loop")).expect("symlink loop");
    symlink("b", root.join("link-thu-muc")).expect("symlink thư mục");
    symlink("a.txt", root.join("link.txt")).expect("symlink tệp");

    let o = hai_duong(&d, "sym1", &ct_glob(&root, "*.txt"));
    assert_eq!(
        o, "5\n.an.txt\na.txt\nb/c.txt\nzz/y.txt\nđ.txt\n",
        "symlink phải bị bỏ hoàn toàn"
    );
    let o = hai_duong(&d, "sym2", &ct_glob(&root, "**"));
    assert_eq!(
        o, "9\n.an.txt\na.txt\nb/\nb/c.txt\nb/rong/\nx\nzz/\nzz/y.txt\nđ.txt\n",
        "không có mục symlink nào trong kết quả"
    );
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- lỗi dữ liệu: Err(Error), không trap ----------

/// Mẫu sai ⇒ `Err(Error)` (in `loi`, exit 0): rỗng, `**` không phải thành phần đầy đủ, ba sao,
/// tuyệt đối, quá dài. **Không** mẫu nào làm trap/panic.
#[test]
fn mau_khong_hop_le_la_err_du_lieu() {
    let d = tam("mau");
    let root = cay(&d);
    let dai = "*".repeat(4097);
    for (tag, mau) in [
        ("rong", ""),
        ("giua", "a**b"),
        ("ba", "***"),
        ("tuyetdoi", "/tmp/*"),
        ("dai", dai.as_str()),
    ] {
        let o = hai_duong(&d, tag, &ct_glob(&root, mau));
        assert_eq!(o, "loi\n", "[{tag}] mẫu `{mau}` phải là Err dữ liệu");
    }
    let _ = std::fs::remove_dir_all(&d);
}

// ---------- arity / kiểu: lỗi tĩnh, không vào host ----------

/// Sai kiểu đối số ⇒ `VS0412`; sai arity ⇒ `VS0401`; exit 2 ở cả hai đường (lỗi tĩnh trước khi chạy).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    let d = tam("sig");
    for (tag, call, ma) in [
        ("kieu", "fs.glob(1, \"*\")", "VS0412"),
        ("thieu", "fs.glob(\".\")", "VS0401"),
        ("arity", "fs.glob(\".\", \"*\", 1)", "VS0401"),
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

/// `cap[Dir]` chỉ cấp `root`; `glob(root/../ngoai, "*")` ⇒ `VS0604`, exit 2, **không** in tên nào
/// (glob không được lách quyền của `walk`).
#[test]
fn glob_ngoai_quyen_bi_chan() {
    let d = tam("quyen");
    let root = d.join("root");
    let ngoai = d.join("ngoai");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::create_dir_all(&ngoai).expect("ngoai");
    std::fs::write(ngoai.join("bi-mat.txt"), "x").expect("tệp ngoài");
    let src = format!(
        "use std.fs\n\nfn main(r: cap[Dir] @ p\"{}\"):\n    match fs.glob(\"{}/../ngoai\", \"*.txt\"):\n        Ok(xs):\n            print(xs.len())\n        Err(e):\n            print(\"ERR\")\n",
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

/// `--fs-mock`: cùng ngữ nghĩa lọc, hai đường byte-identical.
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
    let o = hai_duong_mock(&d, "mock", mock, &ct_glob(Path::new("."), "**/*.txt"));
    assert_eq!(o, "2\na/c.txt\nb.txt\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `--fs-mock`: root không có trong kịch bản ⇒ `Err` dữ liệu (cùng lỗi `walk`), không trap.
#[test]
fn mock_host_thieu_root_la_err() {
    let d = tam("mock2");
    let o = hai_duong_mock(
        &d,
        "mock2",
        r#"{ "dirs": { ".": [] } }"#,
        &ct_glob(Path::new("khong-co"), "*"),
    );
    assert_eq!(o, "loi\n");
    let _ = std::fs::remove_dir_all(&d);
}
