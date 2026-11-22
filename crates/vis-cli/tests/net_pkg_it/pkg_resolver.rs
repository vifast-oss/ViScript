// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-128 (VD-125)** — `vis pkg add` dùng **resolver tất định**: chọn phiên bản cao nhất thoả,
//! quay lui khi xung đột, mã `VS0704` kèm **chuỗi ràng buộc**, và **hai lần chạy cho cùng byte**.
//!
//! Unit test của resolver ở `crates/vis-cli/src/pkg/resolver.rs` (17 ca); tệp này kiểm **đường CLI**
//! (registry thật trên đĩa + `vis.lock`), hermetic: mỗi ca một thư mục tạm.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tmp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-pkgres-{tag}-{}-{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// Registry mẫu trong repo (chỉ **đọc**): `http` có 1.0.0 / 1.9.0 / 1.10.0, `util` 0.1.0.
fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/pkg/registry")
}

fn vis(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("chạy `vis pkg`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn doc(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("đọc {}: {e}", p.display()))
}

/// Ghi một gói vào registry tạm (manifest băm đúng nội dung).
fn tao_goi(reg: &Path, ten: &str, ver: &str, deps: &[(&str, &str)]) {
    let dir = reg.join(ten).join(ver);
    std::fs::create_dir_all(dir.join("src")).expect("mkdir");
    let noi_dung =
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n\npub fn {ten}() -> int:\n    1\n");
    std::fs::write(dir.join("src/main.vis"), &noi_dung).expect("ghi nguồn");
    let h = vis_bytecode::source_hash(&noi_dung);
    let sha: String = h.iter().map(|b| format!("{b:02x}")).collect();
    let dep_json = deps
        .iter()
        .map(|(n, r)| format!("\"{n}\":\"{r}\""))
        .collect::<Vec<_>>()
        .join(",");
    let m = format!(
        "{{\"deps\":{{{dep_json}}},\"entry\":\"src/main.vis\",\"files\":[{{\"path\":\"src/main.vis\",\"sha256\":\"{sha}\"}}],\"name\":\"{ten}\",\"version\":\"{ver}\"}}"
    );
    std::fs::write(dir.join("manifest.json"), m).expect("ghi manifest");
}

/// Ca 1 — `http@^1.0` chọn **cao nhất** thoả (`1.10.0`) + kéo `util`; `^1` cũng vậy.
#[test]
fn chon_cao_nhat_theo_rang_buoc() {
    let d = tmp_dir("cao-nhat");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@^1.0",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    assert!(
        d.join(".vis/pkg/http/1.10.0/src/main.vis").is_file(),
        "^1.0 ⇒ 1.10.0 (cao nhất thoả)"
    );
    assert!(d.join(".vis/pkg/util/0.1.0/src/main.vis").is_file(), "dep");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 2 — `@1.0.0` (đủ 3 thành phần, không toán tử) giữ nghĩa **ghim đúng** như ST-70.
#[test]
fn ba_thanh_phan_van_la_ghim_dung() {
    let d = tmp_dir("ghim");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@1.0.0",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    assert!(d.join(".vis/pkg/http/1.0.0/src/main.vis").is_file());
    assert!(!d.join(".vis/pkg/http/1.10.0").exists(), "không được nâng");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 3 — khoảng `@<1.9` chọn bản cao nhất **dưới** 1.9 ⇒ 1.0.0.
#[test]
fn khoang_chon_dung_tran() {
    let d = tmp_dir("khoang");
    let (c, _o, e) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@<1.9",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    assert!(d.join(".vis/pkg/http/1.0.0/src/main.vis").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 4 — không phiên bản nào thoả ⇒ exit **2**, **`VS0704`**, nêu ràng buộc + bản **có**;
/// **không** tạo `.vis/`, **không** ghi `vis.lock` (giải trước khi ghi).
#[test]
fn khong_thoa_bao_vs0704_exit_2() {
    let d = tmp_dir("vs0704");
    let (c, out, err) = vis(
        &d,
        &[
            "pkg",
            "add",
            "http@^2",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 2, "stdout: {out} stderr: {err}");
    assert!(err.contains("VS0704"), "phải có mã mới: {err}");
    assert!(err.contains("http") && err.contains("^2"), "{err}");
    assert!(
        err.contains("1.0.0") && err.contains("1.10.0"),
        "phải liệt kê bản **có**: {err}"
    );
    assert!(!d.join("vis.lock").exists(), "KHÔNG được ghi lock");
    assert!(!d.join(".vis").exists(), "KHÔNG được tạo .vis");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 5 — xung đột **qua dep**: `a` cần `b@^2` mà registry chỉ có `b@1.x` ⇒ `VS0704` nêu
/// **chuỗi ràng buộc** (nguồn `a@1.0.0`).
#[test]
fn xung_dot_qua_dep_neu_nguon() {
    let d = tmp_dir("chuoi");
    let reg = d.join("registry");
    tao_goi(&reg, "a", "1.0.0", &[("b", "^2")]);
    tao_goi(&reg, "b", "1.0.0", &[]);
    tao_goi(&reg, "b", "1.4.0", &[]);
    let (c, _o, err) = vis(
        &d,
        &["pkg", "add", "a", "--registry", reg.to_str().unwrap()],
    );
    assert_eq!(c, 2, "stderr: {err}");
    assert!(err.contains("VS0704"), "{err}");
    assert!(err.contains("^2"), "{err}");
    assert!(
        err.contains("a@1.0.0"),
        "phải nêu **nguồn** ràng buộc: {err}"
    );
    assert!(err.contains("1.4.0"), "phải nêu bản có: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca 6 — **tất định byte**: cùng registry + cùng spec ở **hai thư mục** khác nhau ⇒ `vis.lock`
/// **byte y hệt**; thêm lần nữa vào cùng thư mục cũng không đổi byte.
#[test]
fn hai_lan_chay_cung_byte_lock() {
    let d1 = tmp_dir("tat-dinh-1");
    let d2 = tmp_dir("tat-dinh-2");
    for d in [&d1, &d2] {
        let (c, _o, e) = vis(
            d,
            &[
                "pkg",
                "add",
                "http@^1",
                "--registry",
                fixture().to_str().unwrap(),
            ],
        );
        assert_eq!(c, 0, "stderr: {e}");
    }
    let l1 = doc(&d1.join("vis.lock"));
    let l2 = doc(&d2.join("vis.lock"));
    assert_eq!(l1, l2, "hai lần chạy ⇒ cùng byte lock");
    let (c, _o, e) = vis(
        &d1,
        &[
            "pkg",
            "add",
            "http@^1",
            "--registry",
            fixture().to_str().unwrap(),
        ],
    );
    assert_eq!(c, 0, "stderr: {e}");
    assert_eq!(doc(&d1.join("vis.lock")), l1, "add lại không đổi byte");
    let _ = std::fs::remove_dir_all(&d1);
    let _ = std::fs::remove_dir_all(&d2);
}

/// Ca 7 — ràng buộc **sai cú pháp** ⇒ exit 2, thông điệp nêu chuỗi sai, không tạo gì.
#[test]
fn rang_buoc_sai_cu_phap_exit_2() {
    let d = tmp_dir("sai-cu-phap");
    for spec in ["http@^", "http@1.x", "http@>=1.0.0||<2"] {
        let (c, _o, e) = vis(
            &d,
            &[
                "pkg",
                "add",
                spec,
                "--registry",
                fixture().to_str().unwrap(),
            ],
        );
        assert_eq!(c, 2, "{spec}: {e}");
        assert!(!d.join(".vis").exists(), "{spec}: không được tạo .vis");
    }
    let _ = std::fs::remove_dir_all(&d);
}
