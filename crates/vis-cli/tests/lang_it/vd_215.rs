// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-208 / VD-215** (A0 2026-11-18) — quy tắc `{…}`: `(expr)` là khoá tính toán chính thức.
//! Chế độ literal quyết theo **toàn bộ** khoá; trộn loại khoá — bất kể thứ tự — là lỗi **VS0207**;
//! `{[k]: …}` (khoá danh sách) ⇒ **cảnh báo VS0208** + vẫn ra map; `{([k]): …}` tắt cảnh báo.
//!
//! Chạy cả interp và VM (bằng so-sánh đầu ra); ca lỗi chạy qua `vis check`.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd215-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

/// `vis check` trên `src`; trả về (exit, stdout). Lỗi cú pháp ⇒ stdout chứa TAP + mã lỗi.
fn check(tag: &str, src: &str) -> (i32, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    let _ = std::fs::remove_dir_all(&d);
    (o.status.code().unwrap_or(-1), out)
}

/// Chạy cả hai đường; đòi cùng stdout/exit; trả về (exit, stdout).
fn hai_duong(tag: &str, src: &str) -> (i32, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let chay = |args: &[&std::ffi::OsStr]| -> (i32, String, String) {
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .arg("run")
            .arg("--no-cache")
            .args(args)
            .arg(&p)
            .output()
            .expect("chạy vis run");
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
        )
    };
    let a = chay(&[]);
    let b = chay(&[std::ffi::OsStr::new("--vm")]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, b.0, "[{tag}] hai đường lệch exit");
    assert_eq!(a.1, b.1, "[{tag}] hai đường lệch stdout");
    assert_eq!(a.2, b.2, "[{tag}] hai đường lệch stderr");
    (a.0, a.1)
}

/// stderr của hai đường (kiểm cảnh báo); đòi hai đường **giống nhau**.
fn stderr_hai_duong(tag: &str, src: &str) -> String {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let chay = |args: &[&std::ffi::OsStr]| -> String {
        let o = Command::new(env!("CARGO_BIN_EXE_vis"))
            .arg("run")
            .arg("--no-cache")
            .args(args)
            .arg(&p)
            .output()
            .expect("chạy vis run");
        String::from_utf8_lossy(&o.stderr).to_string()
    };
    let a = chay(&[]);
    let b = chay(&[std::ffi::OsStr::new("--vm")]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a, b, "[{tag}] hai đường lệch stderr cảnh báo");
    a
}

#[test]
fn tron_khoa_ca_hai_thu_tu_ra_vs0207() {
    let src_a = "fn main():\n    let m = {\"a\": 1, b: 2}\n    print(m)\n";
    let src_b = "fn main():\n    let m = {a: 1, \"b\": 2}\n    print(m)\n";
    for (tag, src) in [("mix-a", src_a), ("mix-b", src_b)] {
        let (code, out) = check(tag, src);
        assert_ne!(code, 0, "[{tag}] phải bị từ chối: {out}");
        assert!(out.contains("VS0207"), "[{tag}] phải báo VS0207: {out}");
    }
    // Gợi ý theo khoá đầu tiên
    let (_, out_a) = check("mix-a-hint", src_a);
    assert!(out_a.contains("map"), "hint khoá đầu là chuỗi: {out_a}");
    let (_, out_b) = check("mix-b-hint", src_b);
    assert!(
        out_b.contains("bản ghi"),
        "hint khoá đầu là định danh: {out_b}"
    );
}

#[test]
fn khoa_tinh_toan_ngoac_in_ra_map() {
    let (code, out) = hai_duong(
        "calc-key",
        "fn main():\n    let k = \"e\"\n    let m = {(k): 1}\n    print(m)\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "{\"e\": 1}\n");
}

#[test]
fn bien_k_trong_ngoac_van_la_ban_ghi() {
    // `{k: 1}` với biến `k` trong phạm vi ⇒ **bản ghi** trường `k` (giữ hành vi).
    let (code, out) = hai_duong(
        "record-k",
        "fn main():\n    let k = \"e\"\n    let m = {k: 1}\n    print(m)\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "{k: 1}\n");
}

#[test]
fn chuoi_cong_ngoac_la_map() {
    let (code, out) = hai_duong(
        "string-plus-paren",
        "fn main():\n    let m = {\"a\": 1, (\"b\"): 2}\n    print(m)\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "{\"a\": 1, \"b\": 2}\n");
}

#[test]
fn list_key_canh_bao_vs0208() {
    let e = stderr_hai_duong(
        "list-key",
        "fn main():\n    let k = \"a\"\n    let m = {[k]: 1}\n    print(m)\n",
    );
    assert!(e.contains("VS0208"), "phải cảnh báo VS0208: {e}");
    // Tắt cảnh báo bằng `{([k]): …}` — không có VS0208
    let e2 = stderr_hai_duong(
        "list-key-silenced",
        "fn main():\n    let k = \"a\"\n    let m = {([k]): 1}\n    print(m)\n",
    );
    assert!(
        !e2.contains("VS0208"),
        "`{{([k]): …}}` phải tắt cảnh báo: {e2}"
    );
}

#[test]
fn list_key_chay_va_ra_map_khoa_danh_sach() {
    let (code, out) = hai_duong(
        "list-key-run",
        "fn main():\n    let k = \"a\"\n    let m = {[k]: 1}\n    print(m)\n",
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out, "{[a]: 1}\n");
}
