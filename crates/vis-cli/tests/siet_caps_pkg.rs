// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-130 (VD-124, Q8c)** — **siết caps mặc định khi chương trình dùng gói**:
//!
//! - Đồ thị import có **ít nhất một** module `pkg.*` ⇒ strict-caps bật **mặc định**.
//! - Thoát **tường minh** `--no-strict-caps` ⇒ chạy được + **đúng một** dòng cảnh báo ra **stderr**
//!   (không ra stdout).
//! - Chương trình **không** dùng gói ⇒ hành vi **giữ nguyên** (không siết, không cảnh báo).
//!
//! Mỗi ca chạy **cả** interp và `--vm --no-cache`, so exit + stdout (parity như các ca caps khác).

use std::path::{Path, PathBuf};
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-sietpkg-{tag}-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/pkg/registry")
}

fn ghi(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).expect("mkdir");
    }
    std::fs::write(p, s).expect("ghi");
}

fn chay(d: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(d)
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy **cả hai** đường (interp, `--vm --no-cache`) và đòi exit/stdout **giống nhau**.
fn ca(d: &Path, them: &[&str], tep: &str) -> (i32, String, String) {
    let mut a = vec!["run"];
    a.extend_from_slice(them);
    a.push(tep);
    let mut b = vec!["run", "--vm", "--no-cache"];
    b.extend_from_slice(them);
    b.push(tep);
    let r1 = chay(d, &a);
    let r2 = chay(d, &b);
    assert_eq!(r1.0, r2.0, "exit lệch interp/VM: {r1:?} vs {r2:?}");
    assert_eq!(r1.1, r2.1, "stdout lệch interp/VM: {r1:?} vs {r2:?}");
    r1
}

/// Cài gói `http` từ registry mẫu ⇒ `.vis/pkg/**` + `vis.lock` (để `use pkg.http` nạp được).
fn cai_http(d: &Path) {
    let (c, _o, e) = chay(
        d,
        &[
            "pkg",
            "add",
            "http@1.0.0",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "cài gói mẫu: {e}");
}

/// Đếm số dòng stderr nói về `--no-strict-caps` (cảnh báo phải **đúng một** dòng).
fn dem_canh_bao(err: &str) -> usize {
    err.lines()
        .filter(|l| l.contains("--no-strict-caps"))
        .count()
}

/// (a) Dùng gói + chạm `std.fs` mà **không** khai `cap` ⇒ `VS0604`, exit **2** (mặc định siết).
#[test]
fn co_pkg_thieu_cap_bao_vs0604() {
    let d = tam("a");
    cai_http(&d);
    ghi(&d.join("a.txt"), "x");
    ghi(
        &d.join("f.vis"),
        "use std.fs\nuse pkg.http\n\nfn main():\n    print(fs.exists(\"a.txt\"))\n    print(http_version())\n",
    );
    let (c, out, err) = ca(&d, &[], "f.vis");
    assert_eq!(c, 2, "stderr: {err}");
    assert!(err.contains("VS0604"), "{err}");
    assert!(err.contains("cap[Dir]"), "phải nêu cap cần khai: {err}");
    assert_eq!(out, "", "không được in gì khi bị chặn");
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) Như (a) + `--no-strict-caps` ⇒ **chạy được**, cảnh báo **một** dòng ra **stderr**.
#[test]
fn thoat_bang_no_strict_caps_chay_va_canh_bao_mot_dong() {
    let d = tam("b");
    cai_http(&d);
    ghi(&d.join("a.txt"), "x");
    ghi(
        &d.join("f.vis"),
        "use std.fs\nuse pkg.http\n\nfn main():\n    print(fs.exists(\"a.txt\"))\n    print(http_version())\n",
    );
    let (c, out, err) = ca(&d, &["--no-strict-caps"], "f.vis");
    assert_eq!(c, 0, "stderr: {err}");
    assert_eq!(out, "true\n100\n", "stdout **chỉ** có đầu ra chương trình");
    assert_eq!(
        dem_canh_bao(&err),
        1,
        "cảnh báo phải **đúng một** dòng: {err:?}"
    );
    assert!(err.contains("cảnh báo"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) **Không** dùng gói ⇒ hành vi **giữ nguyên**: không siết, không cảnh báo (kể cả khi có cờ).
#[test]
fn khong_pkg_hanh_vi_cu() {
    let d = tam("c");
    ghi(&d.join("a.txt"), "x");
    ghi(
        &d.join("f.vis"),
        "use std.fs\n\nfn main():\n    print(fs.exists(\"a.txt\"))\n",
    );
    let (c, out, err) = ca(&d, &[], "f.vis");
    assert_eq!(c, 0, "không dùng gói ⇒ không siết: {err}");
    assert_eq!(out, "true\n");
    assert_eq!(dem_canh_bao(&err), 0, "không được cảnh báo: {err:?}");
    // Cờ thoát không có gì để thoát ⇒ không cảnh báo, hành vi y hệt.
    let (c2, out2, err2) = ca(&d, &["--no-strict-caps"], "f.vis");
    assert_eq!(c2, 0, "stderr: {err2}");
    assert_eq!(out2, "true\n");
    assert_eq!(dem_canh_bao(&err2), 0, "{err2:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) Dùng gói + `--strict-caps` **tường minh** ⇒ vẫn `VS0604` (cờ không làm mất chế độ siết).
#[test]
fn strict_caps_tuong_minh_van_siet() {
    let d = tam("d");
    cai_http(&d);
    ghi(&d.join("a.txt"), "x");
    ghi(
        &d.join("f.vis"),
        "use std.fs\nuse pkg.http\n\nfn main():\n    print(fs.exists(\"a.txt\"))\n",
    );
    let (c, out, err) = ca(&d, &["--strict-caps"], "f.vis");
    assert_eq!(c, 2, "stderr: {err}");
    assert!(err.contains("VS0604"), "{err}");
    assert_eq!(out, "");
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) Dùng gói nhưng chương trình **không** chạm fs/net + `--no-strict-caps` ⇒ vẫn chạy và vẫn
/// cảnh báo **một** dòng (cờ là lựa chọn tường minh của người chạy, không phụ thuộc nội dung).
#[test]
fn co_pkg_khong_cham_cap_ma_van_thoat_thi_canh_bao() {
    let d = tam("e");
    cai_http(&d);
    ghi(
        &d.join("f.vis"),
        "use pkg.http\n\nfn main():\n    print(http_version())\n",
    );
    let (c, out, err) = ca(&d, &[], "f.vis");
    assert_eq!(c, 0, "không chạm fs/net ⇒ chạy được: {err}");
    assert_eq!(out, "100\n");
    assert_eq!(
        dem_canh_bao(&err),
        0,
        "không có cờ ⇒ không cảnh báo: {err:?}"
    );
    let (c2, out2, err2) = ca(&d, &["--no-strict-caps"], "f.vis");
    assert_eq!(c2, 0, "stderr: {err2}");
    assert_eq!(out2, "100\n");
    assert_eq!(
        dem_canh_bao(&err2),
        1,
        "cờ ⇒ đúng một dòng cảnh báo: {err2:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (f) Dùng gói + **khai đủ** `cap[Dir]` + đọc **trong** root ⇒ chạy bình thường, **không** cảnh báo.
#[test]
fn co_pkg_va_khai_du_cap_thi_chay() {
    let d = tam("f");
    cai_http(&d);
    let root = d.join("root");
    ghi(&root.join("a.txt"), "x");
    let r = root.canonicalize().expect("root");
    ghi(
        &d.join("f.vis"),
        &format!(
            "use std.fs\nuse pkg.http\n\nfn main(d: cap[Dir] @ p\"{}\"):\n    print(fs.exists(\"{}/a.txt\"))\n    print(http_version())\n",
            r.display(),
            r.display()
        ),
    );
    let (c, out, err) = ca(&d, &[], "f.vis");
    assert_eq!(c, 0, "stderr: {err}");
    assert_eq!(out, "true\n100\n");
    assert_eq!(dem_canh_bao(&err), 0, "không phải cảnh báo: {err:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (g) Hai cờ **mâu thuẫn** ⇒ exit 2, thông điệp rõ, không chạy chương trình.
#[test]
fn hai_co_mau_thuan_exit_2() {
    let d = tam("g");
    cai_http(&d);
    ghi(
        &d.join("f.vis"),
        "use pkg.http\n\nfn main():\n    print(1)\n",
    );
    let (c, out, err) = chay(&d, &["run", "--strict-caps", "--no-strict-caps", "f.vis"]);
    assert_eq!(c, 2, "stderr: {err}");
    assert!(err.contains("mâu thuẫn"), "{err}");
    assert_eq!(out, "", "không được chạy: {out}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (h) **Ca thứ 8 (VD-133, 23b §2)** — chương trình có **`use` module nội bộ** (`use ./util`) +
/// `use std.fs`, **không** `pkg.*`, **không** khai `cap`, đọc một tệp ⇒ chạy được, **không** `VS0604`.
///
/// Đây là ca khoá câu: "chương trình có `use` nhưng **không** dùng `pkg.*` phải giữ hành vi cũ về
/// caps". Sự tồn tại của `use`, kể cả `use` module nội bộ (kéo theo `NamePolicy::Strict`), **không**
/// được bật siết caps.
///
/// **Đối chứng ngay trong ca** (để ca không vô nghĩa): cùng chương trình, chỉ thêm `--strict-caps`
/// ⇒ đường đọc tệp bị siết **thật**, `VS0604` + exit 2.
#[test]
fn use_module_noi_bo_va_std_nhung_khong_pkg_giu_hanh_vi_cu() {
    let d = tam("h");
    ghi(&d.join("a.txt"), "x");
    // Module nội bộ **thật**: `g()` phải trả 7 ⇒ `use ./util` thực sự được nạp, không phải import rỗng.
    ghi(&d.join("util.vis"), "pub fn g() -> int:\n    7\n");
    ghi(
        &d.join("f.vis"),
        "use ./util\nuse std.fs\n\nfn main():\n    match fs.read(\"a.txt\"):\n        ok(s):\n            print(s)\n            print(g())\n        err(e):\n            print(\"err\")\n",
    );
    let (c, out, err) = ca(&d, &[], "f.vis");
    assert_eq!(c, 0, "không dùng gói ⇒ hành vi cũ, không siết: {err}");
    assert_eq!(out, "x\n7\n", "đọc tệp + gọi module nội bộ: {out:?}");
    assert!(
        !err.contains("VS0604"),
        "có `use` mà không `pkg.*` ⇒ **không** được siết caps: {err:?}"
    );
    // **Đối chứng**: cùng chương trình + `--strict-caps` ⇒ siết thật (`VS0604`, exit 2, không in gì).
    let (c2, out2, err2) = ca(&d, &["--strict-caps"], "f.vis");
    assert_eq!(c2, 2, "stderr: {err2}");
    assert!(err2.contains("VS0604"), "{err2}");
    assert!(err2.contains("cap[Dir]"), "phải nêu cap cần khai: {err2}");
    assert_eq!(out2, "", "không được in gì khi bị chặn: {out2}");
    let _ = std::fs::remove_dir_all(&d);
}
