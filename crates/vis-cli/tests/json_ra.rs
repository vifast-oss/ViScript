// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-101 (VD-113)** — đầu ra máy đọc: `vis check --json` + `vis test --json`.
//! Hợp đồng: **một** object JSON nén/dòng, khoá + thứ tự cố định; `check` dùng `line/col` **1-based**;
//! `test` message một dòng ≤200 ký tự; không cờ ⇒ TAP y hệt; stderr rỗng.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-json-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tmp");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    std::fs::write(&p, src).expect("ghi");
    p
}

fn vis(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .output()
        .expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn ca_test(d: &Path, them: &[&str]) -> (i32, String, String) {
    let mut args = vec!["test", "--json", d.to_str().unwrap()];
    args.extend_from_slice(them);
    vis(&args)
}

/// (a) check 1 tệp sạch ⇒ `ok:true`, diagnostics rỗng, exit 0, stderr rỗng.
#[test]
fn check_json_tep_sach() {
    let d = tam("sach");
    let p = ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    let f = p.to_str().unwrap();
    let (c, o, e) = vis(&["check", "--json", f]);
    assert_eq!(c, 0, "{e}");
    assert_eq!(e, "");
    assert_eq!(
        o,
        format!("{{\"ok\":true,\"files\":[{{\"file\":\"{f}\",\"diagnostics\":[]}}]}}\n")
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// (b) check 1 tệp **2 lỗi** ⇒ 2 chẩn đoán, đúng mã + line/col 1-based, exit 1, stderr rỗng.
#[test]
fn check_json_hai_loi() {
    let d = tam("loi");
    let p = ghi(
        &d,
        "b.vis",
        "fn main():\n    let x: int = \"s\"\n    let y: str = 1\n",
    );
    let f = p.to_str().unwrap();
    let (c, o, e) = vis(&["check", "--json", f]);
    assert_eq!(c, 1, "{e}");
    assert_eq!(e, "");
    assert_eq!(o.matches(r#""code":"VS0412""#).count(), 2, "{o}");
    assert!(o.contains(r#""line":2,"col":18"#), "{o}");
    assert!(o.contains(r#""line":3,"col":18"#), "{o}");
    assert_eq!(o.lines().count(), 1, "phải **một dòng** JSON: {o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (c) check **3 tệp** ⇒ `files` đúng thứ tự đối số (sạch, lỗi, sạch), `ok:false`.
#[test]
fn check_json_ba_tep_dung_thu_tu() {
    let d = tam("ba");
    let a = ghi(&d, "a.vis", "fn main():\n    print(1)\n");
    let b = ghi(&d, "b.vis", "fn main():\n    let x: int = \"s\"\n");
    let c = ghi(&d, "c.vis", "fn main():\n    print(2)\n");
    let (f1, f2, f3) = (
        a.to_str().unwrap(),
        b.to_str().unwrap(),
        c.to_str().unwrap(),
    );
    let (ma, o, e) = vis(&["check", "--json", f1, f2, f3]);
    assert_eq!(ma, 1, "{e}");
    assert_eq!(e, "");
    let i1 = o.find(&format!("\"file\":\"{f1}\"")).expect("thiếu f1");
    let i2 = o.find(&format!("\"file\":\"{f2}\"")).expect("thiếu f2");
    let i3 = o.find(&format!("\"file\":\"{f3}\"")).expect("thiếu f3");
    assert!(i1 < i2 && i2 < i3, "sai thứ tự files: {o}");
    assert!(o.starts_with("{\"ok\":false,"), "{o}");
    let _ = std::fs::remove_dir_all(&d);
}

/// (d) test 2/3 ok ⇒ total/ok/failed + cases đúng thứ tự + message một dòng; exit 1; VM cùng JSON.
#[test]
fn test_json_hai_tren_ba() {
    let d = tam("test");
    ghi(&d, "a.vis", "test fn a():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "test fn b():\n    assert 1 == 2\n");
    ghi(&d, "c.vis", "test fn c():\n    assert 1 == 1\n");
    let mong = "{\"total\":3,\"ok\":2,\"failed\":1,\"cases\":[{\"name\":\"a\",\"ok\":true,\"message\":\"\"},{\"name\":\"b\",\"ok\":false,\"message\":\"VS0501 assert thất bại\"},{\"name\":\"c\",\"ok\":true,\"message\":\"\"}]}\n";
    let (c1, o1, e1) = ca_test(&d, &[]);
    assert_eq!(c1, 1, "{e1}");
    assert_eq!(e1, "");
    assert_eq!(o1, mong);
    let (c2, o2, e2) = ca_test(&d, &["--vm"]);
    assert_eq!(c2, 1, "{e2}");
    assert_eq!(o2, mong, "VM phải cùng JSON");
    assert_eq!(e2, "");
    let _ = std::fs::remove_dir_all(&d);
}

/// (e) test **tất cả ok** ⇒ exit 0, `failed:0`; VM cùng JSON.
#[test]
fn test_json_tat_ca_ok() {
    let d = tam("ok");
    ghi(&d, "a.vis", "test fn a():\n    assert 1 == 1\n");
    let mong = "{\"total\":1,\"ok\":1,\"failed\":0,\"cases\":[{\"name\":\"a\",\"ok\":true,\"message\":\"\"}]}\n";
    for them in [&[][..], &["--vm"][..]] {
        let (ma, o, e) = ca_test(&d, them);
        assert_eq!(ma, 0, "{e}");
        assert_eq!(e, "");
        assert_eq!(o, mong, "them={them:?}");
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// (f) **không** `--json` ⇒ TAP **byte y hệt** (không hồi quy), stderr rỗng.
#[test]
fn khong_json_tap_nguyen_ven() {
    let d = tam("tap");
    ghi(&d, "a.vis", "test fn a():\n    assert 1 == 1\n");
    ghi(&d, "b.vis", "test fn b():\n    assert 1 == 2\n");
    ghi(&d, "c.vis", "test fn c():\n    assert 1 == 1\n");
    let mong =
        "TAP version 13\n1..3\nok 1 - a\nnot ok 2 - b\n# b:5:1: VS0501 assert thất bại\nok 3 - c\n";
    let (ma, o, e) = vis(&["test", d.to_str().unwrap()]);
    assert_eq!(ma, 1, "{e}");
    assert_eq!(e, "");
    assert_eq!(o, mong);
    let _ = std::fs::remove_dir_all(&d);
}
