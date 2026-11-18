// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-227** (proxy A0-uỷ quyền, 2026-11-19) — interp **auto-lift** `T → T?` tại `let`.
//!
//! **Bẫy**: trước VD-227, `let m: int? = 42` giữ nguyên `42` (Int) trên interp — từ đó
//! `match m: Some(v)` không khớp nhánh nào (chỉ `_`), vô tình hiển thị `m = 42` trở lên thiếu.
//! VM đã đúng §4.4 từ VD-208. Lệch này **chặn date-order** kế hoạch rule that structs
//! (`VD-186`, VD-220 §3).
//!
//! Mọi ca chạy **cả hai đường** (interp + VM) — byte stdout/stderr và exit code khớp.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd227-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay_ca_hai(tag: &str, src: &str) -> [(u32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let mo = |extra: &[&str]| {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
        c.args(["run"]).arg("--no-cache");
        c.args(extra);
        c.arg(&p);
        let o = c.output().expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1) as u32,
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = mo(&[]);
    let b = mo(&["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// (1) `let x: int? = 42` — interp tự nâng thành `Some(42)`; VM trước đó đã là `Some(42)`; cả hai giống.
#[test]
fn vd227_let_opt_lift() {
    let [a, b] = chay_ca_hai(
        "lift",
        "fn main():\n    let x: int? = 42\n    match x:\n        Some(v): print(\"lifted {v}\")\n        _: print(\"other\")\n",
    );
    assert_eq!(a.0, 0, "interp: {}", a.2);
    assert_eq!(b.0, 0, "vm: {}", b.2);
    assert_eq!(a.1, "lifted 42\n");
    assert_eq!(a.1, b.1);
}

/// (2) `let x: int? = Some(99)` — đã là `Some` — không bị bọc lần nữa; cả hai in `Some(99)`-then-match thứ nhất.
#[test]
fn vd227_already_some() {
    let [a, b] = chay_ca_hai(
        "some",
        "fn main():\n    let x: int? = Some(99)\n    print(x)\n",
    );
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert_eq!(r.1, "Some(99)\n");
    }
}

/// (3) `let x: int? = None` — giữ nguyên `None`; cả hai in `None`.
#[test]
fn vd227_none_literal() {
    let [a, b] = chay_ca_hai("none", "fn main():\n    let x: int? = None\n    print(x)\n");
    for r in [&a, &b] {
        assert_eq!(r.0, 0, "{r:?}");
        assert_eq!(r.1, "None\n");
    }
}

/// (4) `let x: int? = pick()` — giá trị plain không wrap (function-ret **chưa** auto-wrap): cả hai engine `42`.
///     VD-227 ghi scope: chỉ **`let` nhấc lên kiểu Annotated** lip; function-return đặt
///     trong `§4.4` phụ thuộc vấn đề A0 ratify; chưa áp ở dòng này.
#[test]
fn vd227_fn_ret_raw() {
    let [a, b] = chay_ca_hai(
        "fn_ret",
        "fn pick() -> int?:\n    42\nfn main():\n    let m = pick()\n    print(m)\n",
    );
    assert_eq!(a.1, b.1, "hai engine khác: {:?} / {:?}", a, b);
}

/// (6) `let x: int? = 3 + 4` — vế phải biểu thức tự nhấc lên `Some`.
#[test]
fn vd227_expr_into_ann() {
    let [a, b] = chay_ca_hai(
        "expr",
        "fn main():\n    let x: int? = 3 + 4\n    match x:\n        Some(v): print(\"sum {v}\")\n        None: print(\"none\")\n",
    );
    assert_eq!(a.1, "sum 7\n");
    assert_eq!(a.0, 0);
    assert_eq!(a.1, b.1);
}

/// (7) `match` miss vẫn fail sạch — annotated `int?` mà vế phải là giá trị thô qua tự lift.
#[test]
fn vd227_mixed_lit_ann() {
    let [a, b] = chay_ca_hai(
        "mixed",
        "fn main():\n    let x: int? = 5\n    match x:\n        Some(1): print(\"one\")\n        Some(v): print(\"got {v}\")\n        None: print(\"none\")\n",
    );
    assert_eq!(a.1, "got 5\n");
    assert_eq!(b.1, "got 5\n");
}
