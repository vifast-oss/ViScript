// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-157 (nợ #110, VD-156)** — `main` phải nhận `argv` ở **tham số không-`cap`** trên **cả hai**
//! đường: interp (`vis run`) và VM (`vis run --vm --no-cache`). Trước ST-157, VM chỉ đặt `argv` khi
//! `arity == 0` và không có `cap` ⇒ `main(args: list[str])` nhận `()` trên VM trong khi interp nhận
//! `["a","b"]` (lệch interp↔VM). Ca chạy **cả hai đường**, so **byte** stdout/stderr/exit và khẳng
//! định giá trị tuyệt đối `[a, b]`; `Value::Cap` vẫn đặt đúng **chỉ số** tham số (ST-72b).

use std::path::{Path, PathBuf};
use std::process::Command;

type Kq = (i32, Vec<u8>, String);

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st157-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// Chạy một đường: `vis run [--vm --no-cache] <tệp> -- a b`.
fn chay(d: &Path, ten: &str, vm: bool) -> Kq {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d).arg("run");
    if vm {
        c.arg("--vm").arg("--no-cache");
    }
    let o = c
        .arg(ten)
        .arg("--")
        .arg("a")
        .arg("b")
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Ghi chương trình rồi chạy **cả hai** đường; trả `(interp, vm)`.
fn ca(d: &Path, ten: &str, src: &str) -> (Kq, Kq) {
    let f = d.join(format!("{ten}.vis"));
    std::fs::write(
        &f,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
    (
        chay(d, &format!("{ten}.vis"), false),
        chay(d, &format!("{ten}.vis"), true),
    )
}

/// (1) `main(args: list[str])` — **không** `cap`: interp và VM cùng in `[a, b]`, exit 0.
#[test]
fn main_khong_cap_nhan_argv() {
    let d = tam("thuong");
    let (a, b) = ca(&d, "f", "fn main(args: list[str]):\n    print(args)\n");
    assert_eq!(a.0, 0, "interp exit: {a:?}");
    assert_eq!(b.0, 0, "vm exit: {b:?}");
    assert_eq!(String::from_utf8_lossy(&a.1), "[a, b]\n", "interp: {a:?}");
    assert_eq!(a, b, "hai đường lệch (nợ #110):\n interp {a:?}\n vm {b:?}");
    assert_eq!(String::from_utf8_lossy(&b.1), "[a, b]\n", "vm: {b:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (2) `main(d: cap[Dir], args: list[str])` — `Value::Cap` đúng chỉ số 0, `argv` ở 1.
#[test]
fn cap_dau_argv_sau() {
    let d = tam("cap-dau");
    let (a, b) = ca(
        &d,
        "g",
        "fn main(d: cap[Dir] @ p\".\", args: list[str]):\n    print(args)\n",
    );
    assert_eq!(
        a, b,
        "hai đường lệch (cap + argv):\n interp {a:?}\n vm {b:?}"
    );
    assert_eq!(a.0, 0, "{a:?}");
    assert_eq!(String::from_utf8_lossy(&a.1), "[a, b]\n", "{a:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (3) `main(args: list[str], d: cap[Dir])` — tham số không-`cap` ở chỉ số 0 vẫn nhận `argv`,
/// `cap` ở chỉ số 1 **không** bị ghi đè (dùng được `fs.read` qua cap ⇒ bằng chứng cap còn nguyên).
#[test]
fn argv_dau_cap_sau() {
    let d = tam("cap-sau");
    let root = d.join("root");
    std::fs::create_dir_all(&root).expect("root");
    std::fs::write(root.join("a.txt"), "ok").expect("ghi a.txt");
    let (a, b) = ca(
        &d,
        "h",
        &format!(
            "use std.fs\n\nfn main(args: list[str], d: cap[Dir] @ p\"{r}\"):\n    print(args)\n    match fs.read(\"{r}/a.txt\"):\n        ok(s):\n            print(s)\n        err(e):\n            print(\"ERR\")\n",
            r = root.display()
        ),
    );
    assert_eq!(
        a, b,
        "hai đường lệch (argv + cap):\n interp {a:?}\n vm {b:?}"
    );
    assert_eq!(a.0, 0, "{a:?}");
    assert_eq!(String::from_utf8_lossy(&a.1), "[a, b]\nok\n", "{a:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (4) `main()` **không tham số** giữ nguyên hành vi cũ trên cả hai đường (không bị đổi cách gọi).
#[test]
fn main_khong_tham_so_giu_nguyen() {
    let d = tam("khong-ts");
    let (a, b) = ca(&d, "k", "fn main():\n    print(1)\n");
    assert_eq!(
        a, b,
        "hai đường lệch (main rỗng):\n interp {a:?}\n vm {b:?}"
    );
    assert_eq!(a.0, 0, "{a:?}");
    assert_eq!(a.1, b"1\n");
    let _ = std::fs::remove_dir_all(&d);
}
