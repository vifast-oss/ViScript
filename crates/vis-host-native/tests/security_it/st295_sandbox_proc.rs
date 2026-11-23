// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-295 (VD-276 §3)** — `vis run --sandbox` + `cap[Proc]` + `--grant`: chương trình ngoài
//! chạy được nhờ luật **chỉ đọc + Execute** cho thư viện chuẩn và đúng binary; ghi vào `/lib*`
//! vẫn bị chặn.
//!
//! Chỉ Linux (Landlock). Nền khác: `--sandbox` trả 3 theo VD-132 — ca giữ nhánh riêng.

#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::common::TcBin;

const TC: TcBin = TcBin;

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st295-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn chay(d: &Path, ten: &str, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(crate::common::vis_bin())
        .current_dir(d)
        .arg("run")
        .arg("--no-cache")
        .args(args)
        .arg(ten)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// `--sandbox` + grant `cap[Proc]` ⇒ lệnh ngoài trong allowlist chạy được (Execute + libs).
#[cfg(target_os = "linux")]
#[test]
fn sandbox_grant_proc_chay_duoc_binary() {
    let d = tam("proc");
    std::fs::write(
        d.join("chay.vish"),
        format!(
            "# SPDX-License-Identifier: MIT OR Apache-2.0\nfn main(p: cap[Proc]):\n    pass\n\n{TC} echo a\n"
        ),
    )
    .expect("ghi .vish");
    let grant = format!("p={TC}");
    let (ma, out, err) = chay(
        &d,
        "chay.vish",
        &["--sandbox", "--strict-caps", "--grant", &grant],
    );
    assert_eq!(ma, 0, "sandbox + grant Proc phải chạy được: {err}");
    assert_eq!(out, "[a]\n", "stdout");
    assert!(
        err.contains("sandbox: landlock ABI=") && err.contains("mức=đầy đủ"),
        "phải áp Landlock đầy đủ: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **Ca âm**: `cap[Dir]` ghi `/usr/lib` + `--sandbox` ⇒ kernel (hoặc quyền OS) chặn `fs.write`;
/// chương trình in `err`, không ghi được. Kèm: không `--sandbox` cũng không ghi được (root-owned)
/// — ca này chỉ khẳng định **không có đường ghi thành công**, không nhận là bằng chứng duy nhất
/// (bằng chứng kernel mạnh nằm ở `vis-host-linux/tests/landlock.rs::luat_thu_vien_chi_doc_chan_ghi`).
#[cfg(target_os = "linux")]
#[test]
fn sandbox_khong_ghi_duoc_vao_usr_lib() {
    let d = tam("lib");
    std::fs::write(
        d.join("ghi.vis"),
        "# SPDX-License-Identifier: MIT OR Apache-2.0\nuse std.fs\nfn main(d: cap[Dir] @ p\"/usr/lib\" (write)):\n    match fs.write(\"/usr/lib/vis-st295-khong-duoc\", \"x\"):\n        Ok(_):\n            print(\"ok\")\n        Err(e):\n            print(\"err\")\n",
    )
    .expect("ghi .vish");
    let (ma, out, err) = chay(&d, "ghi.vis", &["--sandbox", "--strict-caps"]);
    assert_eq!(ma, 0, "chương trình phải bắt Err dữ liệu: {err}");
    assert_eq!(out, "err\n", "không được ghi /usr/lib: {err}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **Ca âm runtime**: Landlock cấp `Execute` trên `/usr/bin` (thư viện chuẩn) nhưng allowlist
/// `cap[Proc]` **vẫn** chặn binary ngoài grant (`VS0604`, exit 2) trước khi `execve`.
#[cfg(target_os = "linux")]
#[test]
fn sandbox_grant_proc_van_chan_binary_ngoai_allowlist() {
    let d = tam("proc-am");
    std::fs::write(
        d.join("chan.vish"),
        "# SPDX-License-Identifier: MIT OR Apache-2.0\nfn main(p: cap[Proc]):\n    pass\n\n/usr/bin/true\n",
    )
    .expect("ghi .vish");
    let grant = format!("p={TC}");
    let (ma, out, err) = chay(
        &d,
        "chan.vish",
        &["--sandbox", "--strict-caps", "--grant", &grant],
    );
    assert_eq!(ma, 2, "binary ngoài grant phải bị runtime chặn: {err}");
    assert!(out.is_empty(), "không được chạy: {out}");
    assert!(
        err.contains("VS0604") && err.contains("không nằm trong allowlist"),
        "phải là VS0604 allowlist: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
