// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-263 — `let … else` phải có một ngữ nghĩa, hai engine giống nhau.**
//!
//! Review độc lập trên main `6c2c9c2` bắt ba lỗi:
//!
//! 1. **Khối `else` không bắt buộc thoát**: `let x = r else: print("err")` xanh ở checker,
//!    interp trap `VS0501` còn VM in `()` rồi chạy tiếp. Nay checker đòi khối `else` thoát trên
//!    **mọi nhánh** (`return`/`break`/`continue`, hoặc `if`/`match`/khối mà mọi nhánh thoát) —
//!    vi phạm ⇒ `VS0206` ở cả `check`, `run` và `run --vm`.
//! 2. **Tên của pattern rò vào `else`**: `let Some(x) = o else: print("x={x}")` từng thấy `x`.
//!    Nay khối `else` hạ **trước** khi khai pattern và checker kiểm nó bằng môi trường **trước**
//!    binding: `x` trong `else` chỉ có thể là biến **ngoài** (nếu có), không có thì `VS0301`.
//! 3. **Binder lỗi sai kiểu/giá trị**: `else e` nay có kiểu xác định theo pattern —
//!    `let x = r` / `Ok(x)` nhận payload `Err` (E); `Err(e)`, `Some(x)`, `None` nhận `()`;
//!    `let x = o` (Option) nhận `()`. Interp và VM bind **cùng** một giá trị.
//!
//! Mọi ca chạy **cả hai engine** trên cùng một tệp: `vis run --no-cache` (interp) và
//! `vis run --no-cache --vm`, so byte exit/stdout/stderr. Ca từ chối tĩnh còn kiểm `vis check`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd263-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi_tep(tag: &str, src: &str) -> (PathBuf, PathBuf) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    (d, p)
}

fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    c.args(extra);
    c.arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

fn check_src(tag: &str, src: &str) -> (i32, String, String) {
    let (d, p) = ghi_tep(tag, src);
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM, đòi **byte** exit/stdout/stderr khớp nhau.
fn chay_ca_hai(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        a.0, b.0,
        "{tag}: exit interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(
        a.1, b.1,
        "{tag}: stdout interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(
        a.2, b.2,
        "{tag}: stderr interp↔VM lệch\n  interp: {a:?}\n  vm: {b:?}"
    );
    [a, b]
}

/// Ca **phải bị từ chối tĩnh**: `vis check` đỏ + đúng mã, và cả hai engine `vis run`
/// cũng từ chối (exit 2) với cùng mã — không engine nào chạy tiếp.
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_ne!(
        c, 0,
        "{tag}: `vis check` phải đỏ\nstdout: {out}\nstderr: {err}"
    );
    assert!(
        out.contains(ma) || err.contains(ma),
        "{tag}: thiếu mã `{ma}`\nstdout: {out}\nstderr: {err}"
    );
    let (d, p) = ghi_tep(&format!("{tag}-run"), src);
    for (i, extra) in [&[][..], &["--vm"][..]].iter().enumerate() {
        let (code, so, se) = chay_tep(&p, extra);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "{tag} engine={i}: thiếu mã `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Ca **hợp lệ**: `vis check` sạch + hai engine chạy ra cùng stdout.
fn chay_duoc(tag: &str, src: &str, muon: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_eq!(
        c, 0,
        "{tag}: `vis check` phải xanh\nstdout: {out}\nstderr: {err}"
    );
    let [a, b] = chay_ca_hai(tag, src);
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, muon, "{name}: stdout sai\nstderr: {}", o.2);
    }
}

// ===========================================================================
// 1 — khối `else` phải thoát trên mọi nhánh
// ===========================================================================

/// Repro review: `else` in rồi rơi xuống cuối ⇒ `x` không có giá trị. Phải `VS0206`.
#[test]
fn else_roi_xuong_cuoi_bi_tu_choi() {
    tu_choi(
        "else-roi",
        r#"fn main():
    let r: Result[int, str] = Err("bad")
    let x = r else:
        print("err")
    print(x)
"#,
        "VS0206",
    );
}

/// `if` **thiếu `else`** không phải thoát mọi nhánh, dù nhánh `then` có `return`.
#[test]
fn else_if_thieu_else_bi_tu_choi() {
    tu_choi(
        "else-if-thieu",
        r#"fn main():
    let r: Result[int, str] = Err("bad")
    let x = r else:
        if true:
            return
    print(x)
"#,
        "VS0206",
    );
}

/// `if`/`match` mà **mọi** nhánh thoát là hợp lệ.
#[test]
fn else_if_va_match_moi_nhanh_thoat_duoc_chap_nhan() {
    chay_duoc(
        "else-if-match",
        r#"fn f(o: int?) -> int:
    let Some(x) = o else:
        if true:
            return 0
        else:
            return 1
    x

fn g(o: int?) -> int:
    let Some(x) = o else:
        match o:
            None: return 2
            Some(v): return v + 1
    x

fn main():
    print(f(None))
    print(f(Some(7)))
    print(g(None))
    print(g(Some(9)))
"#,
        "0\n7\n2\n9\n",
    );
}

/// `break`/`continue` trong `else` của `let` nằm trong vòng lặp là thoát hợp lệ.
#[test]
fn else_break_continue_trong_vong_duoc_chap_nhan() {
    chay_duoc(
        "else-break",
        r#"fn main():
    let o: int? = None
    var i = 0
    while i < 3:
        let Some(x) = o else:
            break
        print(x)
        i += 1
    for k in 0..2:
        let Some(x) = o else:
            continue
        print(x)
    print("xong")
"#,
        "xong\n",
    );
}

// ===========================================================================
// 2 — pattern không rò vào `else`
// ===========================================================================

/// `x` trong `else` phải trỏ biến **ngoài** cùng tên, không phải `Some(x)` của pattern.
#[test]
fn else_thay_bien_ngoai_chu_khong_thay_pattern() {
    chay_duoc(
        "else-bien-ngoai",
        r#"fn main():
    let x = 99
    let o: int? = None
    let Some(x) = o else:
        print(x)
        return
    print(x)
"#,
        "99\n",
    );
}

/// Không có biến ngoài ⇒ dùng `x` trong `else` là `VS0301`, không im lặng `{error}`.
#[test]
fn ten_pattern_khong_ton_tai_trong_else_bi_tu_choi() {
    tu_choi(
        "else-khong-ten",
        r#"fn main():
    let o: int? = None
    let Some(x) = o else:
        print("x={x}")
        return
"#,
        "VS0301",
    );
}

// ===========================================================================
// 3 — binder `else e` theo pattern (bốn dạng + tên trần)
// ===========================================================================

/// `let x = o else e` với `Option` ⇒ `e: ()`.
#[test]
fn binder_tren_tron_option_la_unit() {
    chay_duoc(
        "binder-option",
        r#"fn main():
    let o: int? = None
    let x = o else e:
        print(e)
        return
    print(x)
"#,
        "()\n",
    );
}

/// `let x = r else e` với `Result` ⇒ `e` là payload `Err` (E).
#[test]
fn binder_tren_tron_result_la_payload_err() {
    chay_duoc(
        "binder-result",
        r#"fn main():
    let r: Result[int, str] = Err("bad")
    let x = r else e:
        print(e)
        return
    print(x)
"#,
        "bad\n",
    );
}

/// `Ok(x)` thất bại vì `Err` ⇒ binder là payload `Err`.
#[test]
fn binder_ok_la_payload_err() {
    chay_duoc(
        "binder-ok",
        r#"fn main():
    let r: Result[int, str] = Err("hong")
    let Ok(v) = r else e:
        print(e)
        return
    print(v)
"#,
        "hong\n",
    );
}

/// `Err(e)` thất bại vì `Ok` ⇒ binder là `()`, **không** phải payload `Ok`.
#[test]
fn binder_err_la_unit() {
    chay_duoc(
        "binder-err",
        r#"fn main():
    let r: Result[int, int] = Ok(5)
    let Err(e) = r else e:
        print(e)
        return
    print(e)
"#,
        "()\n",
    );
}

/// `Ok(Some(x))` có thể thất bại vì `Err` **hoặc** `Ok(None)` ⇒ binder **luôn** là `()`
/// (không được lấy payload `Err` khi thất bại vì `Ok(None)`).
#[test]
fn binder_nested_ok_some_la_unit() {
    chay_duoc(
        "binder-nested",
        r#"fn main():
    let r: Result[int?, int] = Err(7)
    let Ok(Some(x)) = r else e:
        print(e)
        return
    print(x)
"#,
        "()\n",
    );
}

/// Cùng lý do: `e` kiểu `()` — gán vào `int` phải `VS0412`, không được checker tưởng `int`.
#[test]
fn binder_nested_ok_some_khong_phai_int() {
    tu_choi(
        "binder-nested-int",
        r#"fn main():
    let r: Result[int?, int] = Ok(None)
    let Ok(Some(x)) = r else e:
        let y: int = e
        return
"#,
        "VS0412",
    );
}

/// `Some(x)` thất bại vì `None` ⇒ binder là `()`.
#[test]
fn binder_some_la_unit() {
    chay_duoc(
        "binder-some",
        r#"fn main():
    let o: int? = None
    let Some(x) = o else e:
        print(e)
        return
    print(x)
"#,
        "()\n",
    );
}

/// `None` thất bại vì `Some` ⇒ binder là `()`, không phải payload `Some`.
#[test]
fn binder_none_la_unit() {
    chay_duoc(
        "binder-none",
        r#"fn main():
    let o: int? = Some(5)
    let None = o else e:
        print(e)
        return
    print("khong")
"#,
        "()\n",
    );
}
