// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-267 / R10** — `Show` của `size` và `duration` (dùng chung `print`, `str()`, nội suy
//! `{x}` và **cả hai engine**).
//!
//! Quy tắc khoá ở đây:
//! - `size`: thang **thập phân** `b kb mb gb tb`; chọn đơn vị lớn nhất có **phần nguyên ≥ 1**;
//!   tối đa **2 chữ số lẻ**, **làm tròn nửa lên**, bỏ số 0 thừa; âm giữ dấu; `0` ⇒ `0b`.
//!   `kib`/`mib` chỉ còn là thang **literal** (`1kib` = 1024 byte) — hiển thị vẫn thập phân.
//! - `duration`: **ghép** `w d h m s ms us ns` từ lớn đến nhỏ, bỏ thành phần 0, **không cách**;
//!   phân rã **nguyên chính xác** (không `f64`); âm giữ dấu; `0` ⇒ `0ns`.
//! - Biên `i64::MIN` dùng độ lớn không dấu, không tràn.
//!
//! Mọi ca chạy `vis check` rồi `vis run` **và** `vis run --vm --no-cache`, so **byte**
//! stdout/stderr/exit giữa hai engine.
//!
//! **Đỏ-trước** (main `2109c10`): các kỳ vọng mới đều đỏ vì `show_size` cũ in `1024` ⇒ `1kib`,
//! `1500` ⇒ `1500b`; `show_duration` cũ in `1500` ⇒ `1500ns`, `90s` ⇒ `90s`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd267-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

fn chay(args: &[&OsStr]) -> (i32, String, String) {
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

fn chay_run(p: &Path, vm: bool) -> (i32, String, String) {
    let mut a: Vec<&OsStr> = vec![OsStr::new("run"), OsStr::new("--no-cache")];
    if vm {
        a.push(OsStr::new("--vm"));
    }
    a.push(p.as_os_str());
    chay(&a)
}

/// Chương trình hợp lệ: `check` sạch, hai engine exit 0 + stderr rỗng + **byte-identical**,
/// stdout đúng nguyên văn.
fn chay_sach(tag: &str, src: &str, want: &str) {
    let (d, p) = ghi(tag, src);
    let (cc, co, ce) = chay(&[OsStr::new("check"), p.as_os_str()]);
    assert_eq!(
        cc, 0,
        "{tag}: `vis check` phải sạch\nstdout: {co}\nstderr: {ce}"
    );
    assert!(ce.is_empty(), "{tag}: `vis check` stderr phải rỗng: {ce}");

    let a = chay_run(&p, false);
    let b = chay_run(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a, b,
        "{tag}: interp↔VM lệch byte (exit/stdout/stderr)\ninterp: {a:?}\nvm: {b:?}"
    );
    assert_eq!(
        a.0, 0,
        "{tag}: phải chạy sạch\nstdout: {}\nstderr: {}",
        a.1, a.2
    );
    assert!(a.2.is_empty(), "{tag}: stderr phải rỗng: {}", a.2);
    assert_eq!(a.1, want, "{tag}: stdout sai");
}

/// `size`: biên chọn đơn vị, làm tròn 2 chữ số lẻ, âm, `0`, `i64` hai đầu; kèm `str()` + `{x}`.
#[test]
fn size_bien_am_zero_va_ba_duong() {
    let src = r#"fn main():
    print(0b)
    print(999b)
    print(1000b)
    print(1024b)
    print(1200b)
    print(1234b)
    print(1235b)
    print(1500b)
    print(5000b)
    print(999999b)
    print(1000000b)
    print(1500000b)
    print(1000000000b)
    print(1234567890b)
    print(999999999999b)
    print(1000000000000b)
    print(1234567890123b)
    print(9223372036854775807b)
    print(0b - 999b)
    print(0b - 1000b)
    print(0b - 1500b)
    print(0b - 9223372036854775807b - 1b)
    print(str(1500b))
    print("{1500b}")
    let s = 1234b
    print(s)
    print("{s}")
    print(1500b, 90000000000ns)
    print([1500b, 1000b])
    print({a: 1500b})
"#;
    let want = "\
0b
999b
1kb
1.02kb
1.2kb
1.23kb
1.24kb
1.5kb
5kb
1000kb
1mb
1.5mb
1gb
1.23gb
1000gb
1tb
1.23tb
9223372.04tb
-999b
-1kb
-1.5kb
-9223372.04tb
1.5kb
1.5kb
1.23kb
1.23kb
1.5kb 1m30s
[1.5kb, 1kb]
{a: 1.5kb}
";
    chay_sach("size", src, want);
}

/// `duration`: ghép đủ 8 đơn vị, bỏ thành phần 0, âm, `0`, `i64` hai đầu; kèm `str()` + `{x}`.
#[test]
fn duration_ghep_bien_am_zero_va_ba_duong() {
    let src = r#"fn main():
    print(0ns)
    print(1ns)
    print(999ns)
    print(1000ns)
    print(1500ns)
    print(1000000ns)
    print(150000000ns)
    print(1000000000ns)
    print(1500000000ns)
    print(1000000001ns)
    print(90000000000ns)
    print(5400000000000ns)
    print(86400000000000ns)
    print(604800000000000ns)
    print(691200000000000ns)
    print(9223372036854775807ns)
    print(0ns - 1500000000ns)
    print(0ns - 9223372036854775807ns - 1ns)
    print(str(90000000000ns))
    print("{5400000000000ns}")
    let d = 90000000000ns
    print(d)
    print("{d}")
    print([90000000000ns, 5400000000000ns])
"#;
    let want = "\
0ns
1ns
999ns
1us
1us500ns
1ms
150ms
1s
1s500ms
1s1ns
1m30s
1h30m
1d
1w
1w1d
15250w1d23h47m16s854ms775us807ns
-1s500ms
-15250w1d23h47m16s854ms775us808ns
1m30s
1h30m
1m30s
1m30s
[1m30s, 1h30m]
";
    chay_sach("duration", src, want);
}
