// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-321a (`#200`/`#201`)** — chữ ký `main` ngoài đặc tả bị chặn **tĩnh**, không còn lệch
//! interp↔VM: trước bản vá `fn main(a: int, b: int)` làm interp trap `VS0501` (exit 101) trong
//! khi VM chạy tiếp với `b = ()` (exit 0), còn `fn main(a: int)` nhận `argv: list[str]` vào tham
//! số `int` ở cả hai engine. Nay `vis check` đỏ TAP và `vis run` (cả hai đường) dừng ở `exit 2`
//! với **cùng** chẩn đoán `VS0412`/`VS0401`, stdout rỗng.

use std::path::{Path, PathBuf};
use std::process::Command;

type Kq = (i32, Vec<u8>, String);

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-st321a-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

fn chay(d: &Path, args: &[&str]) -> Kq {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .current_dir(d)
        .args(args)
        .output()
        .expect("chạy `vis`");
    (
        o.status.code().unwrap_or(-1),
        o.stdout,
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn viet(d: &Path, ten: &str, src: &str) -> String {
    let f = d.join(format!("{ten}.vis"));
    std::fs::write(
        &f,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình");
    f.to_string_lossy().into_owned()
}

/// `vis check` phải đỏ TAP với mã cho sẵn (in ở TAP `# VSxxxx …` hoặc stderr).
fn kiem_check(d: &Path, p: &str, ma: &[&str]) {
    let (exit, out, err) = chay(d, &["check", p]);
    let text = format!("{}{}", String::from_utf8_lossy(&out), err);
    assert_eq!(exit, 1, "check phải exit 1: {text}");
    assert!(
        String::from_utf8_lossy(&out).starts_with("TAP version 13"),
        "check phải in TAP: {text}"
    );
    assert!(
        String::from_utf8_lossy(&out).contains("not ok"),
        "check: {text}"
    );
    for m in ma {
        assert!(text.contains(m), "check thiếu `{m}`: {text}");
    }
}

/// `vis run` interp và `--vm --no-cache` cùng exit 2, stdout rỗng, stderr **byte bằng nhau**.
fn kiem_run(d: &Path, p: &str, ma: &[&str]) {
    let i = chay(d, &["run", p, "--", "a", "b"]);
    let v = chay(d, &["run", "--vm", "--no-cache", p, "--", "a", "b"]);
    assert_eq!(
        i.0, 2,
        "interp phải exit 2 (lỗi kiểu trước khi chạy): {i:?}"
    );
    assert_eq!(v.0, 2, "vm phải exit 2 (lỗi kiểu trước khi chạy): {v:?}");
    assert!(i.1.is_empty(), "interp stdout phải rỗng: {i:?}");
    assert!(v.1.is_empty(), "vm stdout phải rỗng: {v:?}");
    assert_eq!(i.2, v.2, "interp ↔ VM lệch stderr: interp {i:?}\nvm {v:?}");
    for m in ma {
        assert!(i.2.contains(m), "stderr thiếu `{m}`: {}", i.2);
    }
}

/// `#201` — `fn main(a: int)`: `argv` không được lọt vào tham số `int`.
#[test]
fn main_mot_int_bi_tu_choi() {
    let d = tam("mot-int");
    let p = viet(&d, "f", "fn main(a: int):\n    print(a)\n");
    kiem_check(&d, &p, &["VS0412"]);
    kiem_run(&d, &p, &["VS0412"]);
    let _ = std::fs::remove_dir_all(&d);
}

/// `#200` — `fn main(a: int, b: int)`: vừa sai kiểu tham số đầu, vừa thiếu default ở sau.
#[test]
fn main_hai_int_bi_tu_choi() {
    let d = tam("hai-int");
    let p = viet(
        &d,
        "g",
        "fn main(a: int, b: int):\n    print(a)\n    print(b)\n",
    );
    // `vis check` in **đúng một** dòng `# VSxxxx` cho mỗi tệp (VD-38) ⇒ chỉ thấy mã đầu;
    // đường `vis run` in đủ cả hai chẩn đoán.
    kiem_check(&d, &p, &["VS0412"]);
    kiem_run(&d, &p, &["VS0412", "VS0401"]);
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-323d (`#214`) ca 1** — closure gọi hàm tên có default param: tham số phải được ghi
/// và khôi phục sạch, hai engine cùng in `3`, cùng exit 0. Không rò ra env module.
#[test]
fn st323d_closure_goi_ham_ten_default() {
    let d = tam("closure-default");
    let p = viet(
        &d,
        "d",
        "fn h(--n: int = 3) -> int:\n    n\n\nfn main():\n    let f = fn() -> int: h()\n    print(f())\n",
    );
    let i = chay(&d, &["run", &p]);
    let v = chay(&d, &["run", "--vm", "--no-cache", &p]);
    assert_eq!(i.0, 0, "interp phải exit 0: {i:?}");
    assert_eq!(v.0, 0, "vm phải exit 0: {v:?}");
    assert_eq!(i.1, b"3\n", "interp stdout: {i:?}");
    assert_eq!(i.1, v.1, "interp ↔ VM lệch stdout: {i:?}\nvm {v:?}");
    assert_eq!(i.2, v.2, "interp ↔ VM lệch stderr: {i:?}\nvm {v:?}");
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-323d (`#214`) ca 2** — default `1 / 0` trap **trước** vòng restore của hàm tên gọi
/// qua closure: hai engine cùng exit 101, stdout rỗng, cùng `VS0502`. Phần chứng minh env
/// module sạch sau trap/Ret nằm ở unit test `vis-interp/src/interp/goi_ham.rs` (R8: không tệp
/// test cấp 1 mới).
#[test]
fn st323d_closure_goi_ham_ten_trap_som() {
    let d = tam("closure-trap");
    let p = viet(
        &d,
        "t",
        "fn h(a: int, --n: float = 1 / 0) -> float:\n    1.0\n\nfn main() -> float:\n    let f = fn() -> float: h(7)\n    f()\n",
    );
    let i = chay(&d, &["run", &p]);
    let v = chay(&d, &["run", "--vm", "--no-cache", &p]);
    assert_eq!(i.0, 101, "interp phải exit 101 (trap): {i:?}");
    assert_eq!(v.0, 101, "vm phải exit 101 (trap): {v:?}");
    assert!(i.1.is_empty(), "interp stdout phải rỗng: {i:?}");
    assert_eq!(i.1, v.1, "interp ↔ VM lệch stdout: {i:?}\nvm {v:?}");
    assert!(
        i.2.contains("VS0502"),
        "interp stderr thiếu VS0502: {}",
        i.2
    );
    assert_eq!(i.2, v.2, "interp ↔ VM lệch stderr: {i:?}\nvm {v:?}");
    let _ = std::fs::remove_dir_all(&d);
}
