// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-97 (VD-112)** — `--grant` cho `cap[Proc]` = **allowlist lệnh**:
//! lặp `--grant p=<lệnh>`; không grant ⇒ không giới hạn (tương thích). Grant Proc gán cho
//! `cap[Dir]` ⇒ lỗi kiểu rõ. Mọi ca chạy **cả hai đường**, so byte.
//!
//! **ST-123 (VD-130c)**: luật so khớp đổi từ **basename** sang **đường dẫn tuyệt đối đã
//! canonicalize** (giá trị không có `/` phân giải qua `PATH` một lần lúc khởi động). Ca (c)
//! `--grant p=echo` + chạy `{TC} echo` **vẫn xanh** vì `PATH` phân giải `echo` ⇒ `{TC} echo`, không
//! phải vì basename; ca mới cho bản sao trùng tên ở `tests/grant_proc_canon.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

const TC: &str = env!("CARGO_BIN_EXE_test-child");

type Kq = (i32, Vec<u8>, String);

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-procgrant-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn chay_env(d: &Path, ten: &str, args: &[&str], vm: bool, path_prepend: Option<&str>) -> Kq {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d).arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let tc_parent = std::path::Path::new(TC).parent().unwrap().to_path_buf();
    let mut muc: Vec<std::path::PathBuf> = vec![tc_parent];
    if let Some(p) = path_prepend {
        muc.push(std::path::PathBuf::from(p));
    }
    for p in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        muc.push(p);
    }
    c.env("PATH", std::env::join_paths(muc).expect("join PATH"));
    c.args(args).arg(ten);
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ghi tệp + chạy hai đường, đòi **byte y hệt**.
fn ca_env(d: &Path, ten: &str, src: &str, args: &[&str], path_prepend: Option<&str>) -> Kq {
    std::fs::write(d.join(ten), src).expect("ghi");
    let a = chay_env(d, ten, args, false, path_prepend);
    let b = chay(d, ten, args, true);
    assert_eq!(a, b, "hai đường lệch ({ten}):\n interp {a:?}\n vm {b:?}");
    a
}

fn vish(cmd: &str) -> String {
    format!(
        "# SPDX-License-Identifier: MIT OR Apache-2.0\nfn main(p: cap[Proc]):\n    pass\n\n{cmd}\n"
    )
}

/// (a) grant `<test-child>` + chạy `<test-child> echo a` ⇒ chạy.
#[test]
fn grant_tuyet_doi_echo_chay() {
    let d = tam("a");
    let r = ca(
        &d,
        "a.vish",
        &vish(&format!("{} echo a", TC)),
        &["--strict-caps", "--grant", "p=test-child"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) grant `<test-child>` + chạy `tr` ⇒ `VS0604` allowlist, exit 2.
#[test]
fn grant_echo_chan_grep() {
    let d = tam("b");
    let r = ca(
        &d,
        "b.vish",
        &vish("tr a-z A-Z < /etc/hosts"),
        &["--strict-caps", "--grant", "p=test-child"],
    );
    assert_eq!(r.0, 2, "{r:?}");
    let e = &r.2;
    assert!(e.contains("VS0604"), "{e}");
    assert!(
        e.contains("không nằm trong allowlist") && e.contains("cap[Proc]"),
        "{e}"
    );
    assert!(r.1.is_empty(), "không được chạy: {:?}", r.1);
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) grant **basename** của test-child + chạy test-child (basename khớp bá) ⇒ chạy.
#[test]
fn grant_basename_echo_chay() {
    use std::path::Path;
    let d = tam("c");
    let tc_path = Path::new(TC);
    let tc_parent = tc_path.parent().expect("test-child parent").to_path_buf();
    let tc_bn = tc_path
        .file_name()
        .expect("test-child file_name")
        .to_str()
        .expect("test-child utf8")
        .to_string();
    let r = ca_env(
        &d,
        "c.vish",
        &vish(&format!("{} echo a", TC)),
        &["--strict-caps", "--grant", &format!("p={tc_bn}")],
        Some(tc_parent.to_str().unwrap()),
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) **không** grant ⇒ không giới hạn: `test-child grep` chạy được (tương thích).
#[test]
fn khong_grant_khong_gioi_han() {
    let d = tam("d");
    let etc = d.join("hosts.txt");
    std::fs::write(&etc, "127.0.0.1 localhost\n::1 localhost\n").expect("ghi");
    let etc_s = etc.display().to_string().replace('\\', "/");
    let r = ca(
        &d,
        "d.vish",
        &vish(&format!("test-child grep -E \"^\" {etc_s}")),
        &["--strict-caps"],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert!(!r.1.is_empty(), "grep phải in hosts");
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) **hai** grant (echo + grep) + pipeline 2 stage ⇒ chạy, `[a]`.
#[test]
fn hai_grant_pipeline_hai_stage() {
    let d = tam("e");
    let r = ca(
        &d,
        "e.vish",
        &vish("test-child echo a | test-child grep a"),
        &[
            "--strict-caps",
            "--grant",
            "p=test-child",
            "--grant",
            "p=test-child",
        ],
    );
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, b"[a]\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// (f) `--grant x={TC} echo` cho **`cap[Dir]`** ⇒ lỗi rõ “kiểu không khớp”, exit 2.
#[test]
fn grant_proc_cho_cap_dir_la_loi_kieu() {
    let d = tam("f");
    let r = ca(
        &d,
        "f.vis",
        "fn main(x: cap[Dir] @ p\"./\"):\n    print(1)\n",
        &["--strict-caps", "--grant", &format!("x={}", TC)],
    );
    assert_eq!(r.0, 2, "{r:?}");
    assert!(r.2.contains("kiểu không khớp"), "{}", r.2);
    assert!(
        r.2.contains("cap[Proc]") && r.2.contains("cap[Dir]"),
        "{}",
        r.2
    );
    assert!(r.1.is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[allow(dead_code)]
fn chay(d: &Path, ten: &str, args: &[&str], vm: bool) -> Kq {
    chay_env(d, ten, args, vm, None)
}

fn ca(d: &Path, ten: &str, src: &str, args: &[&str]) -> Kq {
    ca_env(d, ten, src, args, None)
}
