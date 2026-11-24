// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 — `vis run`/`--vm` không hồi quy** cho gán lồng trên đường frame.
//!
//! Phần API thư viện (17 test) đã sang
//! `vis-conformance/tests/async_api_it/vd270_frame_nested_lib.rs` (ST-313c/VD-289).

use std::path::{Path, PathBuf};
use std::process::Command;

const P: &str = "type P:\n    x: int\n\ntype Q:\n    p: P\n\n";

// ===========================================================================
// 6 — `vis run` / `vis run --vm` không hồi quy
// ===========================================================================

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-vd270-frame-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn chay_cli(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run");
    if vm {
        c.arg("--vm").arg("--no-cache");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn vis_run_va_vm_khong_hoi_quy() {
    let d = tam("cli");
    let src = format!(
        "{P}fn main():\n    var q: Q = {{p: {{x: 1}}}}\n    q.p.x = 2\n    var xs: list[P] = [{{x: 3}}]\n    xs[0].x = 4\n    var m: map[str, P] = {{\"k\": {{x: 5}}}}\n    m[\"k\"].x = 6\n    print(q.p.x)\n    print(xs[0].x)\n    print(m[\"k\"].x)\n"
    );
    let p = d.join("nested.vis");
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    let a = chay_cli(&p, false);
    let b = chay_cli(&p, true);
    assert_eq!(a.0, 0, "interp stderr: {}", a.2);
    assert_eq!(b.0, 0, "vm stderr: {}", b.2);
    assert_eq!(a.1, "2\n4\n6\n");
    assert_eq!((a.0, &a.1), (b.0, &b.1), "interp ≠ --vm");
    let _ = std::fs::remove_dir_all(&d);
}
