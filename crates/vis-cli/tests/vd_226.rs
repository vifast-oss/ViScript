// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-226 / VD-226 (uỷ quyền A0-proxy, 2026-11-19)** — `while let PAT = EXPR:`.
//!
//! VD-222 (điểm 7) + R18b (VD-178). Mỗi ca chạy **cả hai đường** (interp và VM) — byte-identical.
//! Ca kiểm chú trị; **không** dựa vào thứ mã hôm nay nâng ngầm giá trị vào `Optional` ở ranh giới
//! hàm — lặp `while let` chỉ thỏa mượn ở `let` annotation / local Option.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd226-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn chay(tag: &str, src: &str, vm: bool) -> (i32, String, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.args(["run"]).arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
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

fn chay_ca_hai(tag: &str, src: &str, mong: &str) {
    let a = chay(tag, src, false);
    let b = chay(tag, src, true);
    assert_eq!(a.0, 0, "interp exit={} stderr={}", a.0, a.2);
    assert_eq!(a.1, mong, "interp stdout khác kỳ vọng: {}", a.1);
    assert_eq!(a.1, b.1, "VM khác byte stdout: vm={:?}", b.1);
    assert_eq!(a.0, b.0, "VM khác exit code: vm={:?}", b.0);
}

/// (1) `while let Some(v) = xs.get(i):` — concept `Optional<int>` op xs.
#[test]
fn vd226_xs_get_consume() {
    let src = "fn main():\n    let xs = [10, 20, 30, 40]\n    var i = 0\n    while let Some(v) = xs.get(i):\n        print(v)\n        i = i + 1\n";
    chay_ca_hai("xs_get", src, "10\n20\n30\n40\n");
}

/// (2) `Some(..)` vào body và `None` làm thoát vòng.
#[test]
fn vd226_one_shot_some() {
    chay_ca_hai(
        "oneshot_some",
        "fn main():\n    var x: int? = Some(11)\n    while let Some(v) = x:\n        print(v)\n        x = None\n",
        "11\n",
    );
}

/// (3) `None` ngay từ đầu — body không chạy.
#[test]
fn vd226_none_entry_skip() {
    chay_ca_hai(
        "none_entry",
        "fn main():\n    var x: int? = None\n    while let Some(v) = x:\n        print(v)\n",
        "",
    );
}

/// (4) while-let với pattern enum người dùng — danh sách `Shape` hữu hạn với sentinel `Empty`.
#[test]
fn vd226_tuple_pattern_wrapped() {
    let src = "enum Shape:\n    Circle(r: int, s: str)\n    Empty\n\nfn main():\n    var xs: list[Shape] = [Shape.Circle(1, \"a\"), Shape.Circle(2, \"b\"), Shape.Empty]\n    var i = 0\n    while let Circle(r, s) = xs[i]:\n        print(\"{r} {s}\")\n        i = i + 1\n";
    chay_ca_hai("tuple_pat", src, "1 a\n2 b\n");
}

/// (5) Vòng chạy hết — `done` marker cho thấy bằng `print`.
#[test]
fn vd226_exhaust_marker() {
    let src = "fn main():\n    let xs = [1, 2, 3]\n    var i = 0\n    while let Some(v) = xs.get(i):\n        print(v)\n        i = i + 1\n    print(\"done\")\n";
    chay_ca_hai("exhaust", src, "1\n2\n3\ndone\n");
}

/// (6) Khi `None` ở đầu, while-let thoát ngay, không treo, in marker sau vòng, exit 0.
#[test]
fn vd226_terminates_and_continues_after() {
    let src = "fn main():\n    var x: int? = None\n    while let Some(v) = x:\n        print(v)\n    print(\"after\")\n";
    chay_ca_hai("terminates", src, "after\n");
}

/// (7) While-let trên **Some-typed plain variable** — lặp lại làm logic ping-pong no larger.
#[test]
fn vd226_plain_some_int_typed() {
    chay_ca_hai(
        "plain_some",
        "fn main():\n    var x: int? = Some(1)\n    let v = match x:\n        Some(v): v\n        None: -1\n    print(v)\n",
        "1\n",
    );
}

/// (8) while-let với pattern mà giá trị không khớp (lấy `None` thay vì `Some`) ⇒ body không chạy.
#[test]
fn vd226_pattern_mismatch_skips() {
    chay_ca_hai(
        "mismatch",
        "fn main():\n    var xs: list[int?] = [None, Some(9)]\n    var i = 0\n    while let Some(v) = xs[i]:\n        print(\"x\")\n        i = i + 1\n    match i:\n        0: print(\"none-at-start\")\n        _: print(\"unexpected\")\n",
        "none-at-start\n",
    );
}
