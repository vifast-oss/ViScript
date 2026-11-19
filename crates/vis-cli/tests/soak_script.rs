// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-98/ST-113 (VD-112/VD-116)** — `bin/vis-soak` chỉ **mô tả** **10** vòng tuần tự (không chạy
//! soak trong test): tồn tại, có SPDX, `set -u`, không dùng `--ignore`/`|| true`, và `--dry-run` in
//! **đúng 10 tên vòng** (ST-113 thêm `gate-3` + `diff-3`).

use std::path::PathBuf;
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bin/vis-soak")
}

/// `bash` thật cho script (`vis-soak`); trên Windows PATH trả WSL stub ⇒ Git Bash tuyệt đối.
fn bash() -> PathBuf {
    #[cfg(windows)]
    {
        for c in [
            "C:/Program Files/Git/bin/bash.exe",
            "C:/Program Files (x86)/Git/bin/bash.exe",
        ] {
            if PathBuf::from(c).is_file() {
                return PathBuf::from(c);
            }
        }
    }
    PathBuf::from("bash")
}

const VONG: [&str; 10] = [
    "gate-1", "gate-2", "gate-3", "diff-1", "diff-2", "diff-3", "host-1", "host-2", "cache-1",
    "cache-2",
];

#[test]
fn vis_soak_hop_le_va_in_du_10_vong() {
    let p = script();
    assert!(p.exists(), "thiếu {}", p.display());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&p)
            .expect("metadata")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "bin/vis-soak phải +x (mode {mode:o})");
    }
    let s = std::fs::read_to_string(&p).expect("đọc script");
    assert!(
        s.lines()
            .any(|l| l.starts_with("# SPDX-License-Identifier: MIT OR Apache-2.0")),
        "thiếu dòng SPDX `#`"
    );
    assert!(s.contains("set -u"), "thiếu `set -u`");
    assert!(!s.contains("--ignore"), "không được né bằng --ignore");
    assert!(!s.contains("|| true"), "không được `|| true`");
    assert!(
        !s.contains("#[ignore]") && !s.contains("#[should_panic]"),
        "không được tắt/ẩn ca"
    );

    // `--dry-run` chỉ in danh sách, không chạy soak.
    let o = Command::new(bash())
        .arg(script())
        .arg("--dry-run")
        .output()
        .expect("chạy --dry-run");
    assert!(
        o.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let out = String::from_utf8_lossy(&o.stdout);
    let dong: Vec<&str> = out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(dong, VONG, "danh sách vòng: {out:?}");
}

/// **ST-233 (hồi quy sau tách repo ST-171)**: `vis-soak` `cd` về **gốc repo** rồi chạy `./vis-dev`.
/// Bản cũ gọi `../vis-dev` (đường của cây `viscript/` trước tách) ⇒ `exit 127` ngay `gate-1`.
///
/// Dựng cây giả: `vis-dev` ở **gốc** ghi marker mỗi lần gọi; `bin/vis-diff`/`bin/vis-host-suite` stub.
/// Đòi `vis-dev` được gọi **3 lần** (gate-1..3); vòng `cache-*` dùng `cargo` thật trong cây không có
/// manifest nên script dừng ở đó — không ảnh hưởng phép kiểm.
#[test]
fn vis_soak_goi_vis_dev_o_goc_repo() {
    let d = std::env::temp_dir().join(format!("vis-soak-goc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).expect("tạo cây giả");
    std::fs::copy(script(), d.join("bin/vis-soak")).expect("chép vis-soak");
    let marker = d.join("marker");
    let viet = |p: &std::path::Path, noi: &str| {
        std::fs::write(p, format!("#!/bin/sh\n{noi}\n")).expect("ghi stub");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut q = std::fs::metadata(p).expect("metadata").permissions();
            q.set_mode(0o755);
            std::fs::set_permissions(p, q).expect("chmod stub");
        }
    };
    viet(
        &d.join("vis-dev"),
        &format!("echo GATE >> '{}'\nexit 0", marker.display()),
    );
    viet(&d.join("bin/vis-diff"), "exit 0");
    viet(&d.join("bin/vis-host-suite"), "exit 0");

    let o = Command::new(bash())
        .arg(d.join("bin/vis-soak"))
        .arg("--log")
        .arg(d.join("soak.log"))
        .output()
        .expect("chạy soak giả");
    let m = std::fs::read_to_string(&marker).unwrap_or_default();
    let so = m.lines().filter(|l| l.trim() == "GATE").count();
    assert_eq!(
        so,
        3,
        "vis-dev ở GỐC phải được gọi 3 lần (gate-1..3); stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
}
