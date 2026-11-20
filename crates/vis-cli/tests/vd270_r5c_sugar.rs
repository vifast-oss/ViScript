// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — R5c buộc biến kiểu: sugar phương thức generic trên `list`/`map` phải chạy
//! (không chỉ `windows`), cả interp và VM.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam() -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-r5c-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn chay(p: &PathBuf, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn sugar_generic_list_chay_hai_engine() {
    let d = tam();
    let p = d.join("r5c.vis");
    std::fs::write(
        &p,
        r#"use std.list
fn main():
    let xs = [3, 1, 2]
    print(xs.sort())
    print(xs.take(2))
    print(xs.drop(1))
    print(xs.chunk(2))
    print(xs.unique())
    print(xs.zip([4, 5, 6]))
    print(xs.min())
    print(xs.max())
    print(xs.windows(2))
    print(xs.concat([4]))
"#,
    )
    .expect("ghi");
    let a = chay(&p, false);
    let b = chay(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, 0, "interp exit: {}", a.2);
    assert_eq!(a.2, "", "interp stderr: {}", a.2);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "interp ≠ VM\n interp: {a:?}\n vm: {b:?}"
    );
    let mong = "[1, 2, 3]\n[3, 1]\n[1, 2]\n[[3, 1], [2]]\n[3, 1, 2]\n[[3, 4], [1, 5], [2, 6]]\n1\n3\n[[3, 1], [1, 2]]\n[3, 1, 2, 4]\n";
    assert_eq!(a.1, mong);
}
