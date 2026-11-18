// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-228** (proxy A0-uỷ quyền 2026-11-19) — lift `T → T?` tại ranh giới hàm `-> T?` (spec §4.4).
//!
//! Trước VD-228, `fn pick() -> int?: 42` trả plain `Int(42)` trên cả interp lẫn VM — sai spec.
//! Sau VD-228, cả hai engine tự nâng `Value::some(42)` ở biên hàm, đồng nhất byte-for-byte.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd228-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay(tag: &str, src: &str, args: &[&str]) -> (i32, String, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    c.args(args);
    c.arg(&p);
    let o = c.output().expect("chạy vis run");
    let r = (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    );
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn chay_ca_hai(tag: &str, src: &str) -> [(u32, String, String); 2] {
    let a = chay(tag, src, &[]);
    let b = chay(tag, src, &["--vm"]);
    [(a.0 as u32, a.1, a.2), (b.0 as u32, b.1, b.2)]
}

/// (1) `fn pick(i: int) -> int?: 42` — plain `Int` tự nâng ở biên `-> T?`.
#[test]
fn vd228_plain_int_lift() {
    let [a, b] = chay_ca_hai(
        "plain_int",
        "fn pick(i: int) -> int?:\n    42\n\nfn main():\n    print(pick(0))\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(a.1, "Some(42)\n");
    assert_eq!(a.1, b.1);
}

/// (2) `fn k() -> str?:` — plain str lift.
#[test]
fn vd228_plain_str_lift() {
    let [a, b] = chay_ca_hai(
        "plain_str",
        "fn k() -> str?:\n    \"x\"\n\nfn main():\n    print(k())\n",
    );
    assert_eq!(a.1, "Some(x)\n");
    assert_eq!(a.1, b.1);
}

/// (3) `fn h() -> int?: None` — `None` giữ nguyên (đã là `Option`).
#[test]
fn vd228_none_stays() {
    let [a, b] = chay_ca_hai(
        "none",
        "fn h() -> int?:\n    None\n\nfn main():\n    print(h())\n",
    );
    assert_eq!(a.1, "None\n");
    assert_eq!(a.1, b.1);
}

/// (4) `fn pick() -> int?: if i < 1: 42 else: None` — branch join lifts cả hai.
#[test]
fn vd228_branch_join() {
    let [a, b] = chay_ca_hai(
        "branch",
        "fn pick(i: int) -> int?:\n    if i < 1:\n        42\n    else:\n        None\n\nfn main():\n    match pick(0):\n        Some(v): print(\"lifted {v}\")\n        None: print(\"none\")\n",
    );
    assert_eq!(a.1, "lifted 42\n");
    assert_eq!(a.1, b.1);
}

/// (5) `fn ret_early() -> int?: return 99` — return sớm lift (hai engine).
#[test]
fn vd228_early_return_lift() {
    let [a, b] = chay_ca_hai(
        "early",
        "fn ret_early() -> int?:\n    return 99\n\nlet m = ret_early()\nprint(m)\n",
    );
    assert_eq!(a.1, "Some(99)\n");
    assert_eq!(a.1, b.1);
}
