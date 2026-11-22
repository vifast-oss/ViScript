// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-259** — hồi quy ngữ nghĩa còn lại sau VD-258:
//!
//! 1. Or-pattern **lồng** và or-pattern trong `if let`/`while let`/`for`/`let`: tên chỉ bind ở
//!    một số alternative mà được dùng ⇒ `VS0412`; cùng tên nhưng **khác kiểu** ⇒ `VS0412`.
//! 2. `break`/`continue`: ngoài vòng lặp, xuyên biên hàm, hoặc ở **vị trí cần giá trị** ⇒
//!    `VS0412`; dạng câu lệnh bình thường vẫn chạy đúng trên **cả hai engine**.
//! 3. `main -> int`: quyết định A0 2026-11-19 mục 7 — **không** suy rộng mã thoát; chỉ
//!    `Result::Err` ⇒ 1, `Ok` ⇒ 0.
//!
//! Mọi ca hợp lệ chạy cả `vis run` (interp) và `vis run --vm`, so byte stdout/stderr/exit.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd259-{tag}-{}-{n}", std::process::id()));
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

/// Chạy interp + VM, đã assert hai engine giống **byte** stdout/stderr/exit.
fn chay_ca_hai(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a.0, b.0, "{tag}: exit interp↔VM lệch");
    assert_eq!(a.1, b.1, "{tag}: stdout interp↔VM lệch");
    assert_eq!(a.2, b.2, "{tag}: stderr interp↔VM lệch");
    [a, b]
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

/// Chương trình **không hợp lệ**: `vis check` đỏ + đúng mã, và **cả hai engine** `vis run`
/// cũng từ chối tĩnh (exit 2) với cùng mã — không engine nào chạy tiếp.
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

// ===========================================================================
// 1 — or-pattern lồng / ngoài `match`
// ===========================================================================

/// `Some(Some(a) | None)` — `a` chỉ bind ở alternative `Some(a)`; thân dùng `a` ⇒ từ chối.
#[test]
fn or_long_trong_ctor_bi_tu_choi() {
    tu_choi(
        "or_nested",
        r#"fn main():
    let x: int?? = Some(Some(1))
    match x:
        Some(Some(a) | None): print(a)
        None: print("none")
"#,
        "VS0412",
    );
}

/// `if let Some(a) | None = o:` — nhánh `then` dùng `a`; nhánh `None` không bind ⇒ từ chối.
#[test]
fn or_trong_if_let_bi_tu_choi() {
    tu_choi(
        "or_iflet",
        r#"fn main():
    let o: int? = None
    if let Some(a) | None = o:
        print(a)
"#,
        "VS0412",
    );
}

/// `while let Some(a) | None = o:` — thân vòng dùng `a` ⇒ từ chối.
#[test]
fn or_trong_while_let_bi_tu_choi() {
    tu_choi(
        "or_whilelet",
        r#"fn main():
    var o: int? = Some(1)
    while let Some(a) | None = o:
        print(a)
        break
"#,
        "VS0412",
    );
}

/// `let Some(a) | None = o` — câu sau dùng `a` ⇒ từ chối.
#[test]
fn or_trong_let_bi_tu_choi() {
    tu_choi(
        "or_let",
        r#"fn main():
    let o: int? = Some(1)
    let Some(a) | None = o
    print(a)
"#,
        "VS0412",
    );
}

/// `for Some(a) | None in xs:` — thân vòng dùng `a` ⇒ từ chối.
#[test]
fn or_trong_for_bi_tu_choi() {
    tu_choi(
        "or_for",
        r#"fn main():
    let xs: list[int?] = [Some(1), None]
    for Some(a) | None in xs:
        print(a)
"#,
        "VS0412",
    );
}

/// Cùng tên bind ở **mọi** alternative nhưng khác kiểu (`int` ↔ `str`) ⇒ từ chối.
#[test]
fn or_cung_ten_khac_kieu_bi_tu_choi() {
    tu_choi(
        "or_type",
        r#"enum E:
    A(x: int)
    B(x: str)

fn main():
    let e: E = A(1)
    match e:
        A(x) | B(x): print(x)
"#,
        "VS0412",
    );
}

/// Or-pattern **lồng** hợp lệ (cùng tên, cùng kiểu ở mọi alternative) không bị từ chối oan;
/// VM bind theo alternative khớp như interp.
#[test]
fn or_long_hop_le_van_chay_hai_engine() {
    let [a, b] = chay_ca_hai(
        "or_nested_ok",
        r#"fn main():
    let x: int?? = Some(Some(7))
    match x:
        Some(Some(a) | Some(a)): print(a)
        Some(None): print("inner-none")
        None: print("none")
"#,
    );
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, "7\n", "{name}: stdout sai");
    }
}

/// Or-pattern trong `for` mà **mọi** alternative bind cùng tên/kiểu — hợp lệ, không từ chối oan.
#[test]
fn or_trong_for_hop_le_van_chay_hai_engine() {
    let [a, b] = chay_ca_hai(
        "or_for_ok",
        r#"fn main():
    for a | a in [1, 2, 3]:
        print(a)
"#,
    );
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, "1\n2\n3\n", "{name}: stdout sai");
    }
}

// ===========================================================================
// 2 — break/continue
// ===========================================================================

/// `break` ngoài vòng lặp ⇒ từ chối tĩnh, không để interp trap / VM lỗi codegen.
#[test]
fn break_ngoai_vong_bi_tu_choi() {
    tu_choi("brk_top", "fn main():\n    break\n", "VS0412");
}

/// `continue` ngoài vòng lặp ⇒ từ chối tĩnh.
#[test]
fn continue_ngoai_vong_bi_tu_choi() {
    tu_choi("cont_top", "fn main():\n    continue\n", "VS0412");
}

/// `break` trong hàm được gọi từ vòng lặp **không** được xuyên biên hàm ⇒ từ chối tĩnh.
#[test]
fn break_xuyen_bien_ham_bi_tu_choi() {
    tu_choi(
        "brk_fn",
        r#"fn f():
    break

fn main():
    loop:
        f()
        break
"#,
        "VS0412",
    );
}

/// `continue` ở vị trí cần giá trị (`let x = if …: continue`) ⇒ từ chối tĩnh.
#[test]
fn continue_o_vi_tri_gia_tri_bi_tu_choi() {
    tu_choi(
        "cont_value",
        r#"fn main():
    var i = 0
    loop:
        i += 1
        let x = if i < 3:
            continue
        else:
            i
        print(x)
        if i > 5:
            break
"#,
        "VS0412",
    );
}

/// `break` ở vị trí cần giá trị (`let x = match …: 1: break`) ⇒ từ chối tĩnh.
#[test]
fn break_o_vi_tri_gia_tri_bi_tu_choi() {
    tu_choi(
        "brk_value_match",
        r#"fn main():
    var i = 0
    loop:
        i += 1
        let x = match i:
            1: break
            _: i
        print(x)
        if i > 4:
            break
"#,
        "VS0412",
    );
}

/// `break`/`continue` dạng **câu lệnh** vẫn hợp lệ — kể cả `if …: break` là câu cuối thân vòng.
#[test]
fn break_continue_cau_lenh_van_chay_hai_engine() {
    let [a, b] = chay_ca_hai(
        "brk_ok",
        r#"fn main():
    var i = 0
    var tong = 0
    while i < 10:
        i += 1
        if i % 2 == 0:
            continue
        if i > 7:
            break
        tong += i
    print(tong)
    loop:
        i += 1
        if i >= 20:
            break
    print(i)
"#,
    );
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: {:?}", o.2);
        assert_eq!(o.1, "16\n20\n", "{name}: stdout sai");
    }
}

// ===========================================================================
// 3 — `main -> int` không phải mã thoát
// ===========================================================================

/// Quyết định A0 2026-11-19 mục 7: chỉ `Result::Err` ⇒ 1, `Ok` ⇒ 0; `main -> int` ⇒ 0
/// (không lấy giá trị `int` làm mã thoát). Cả hai engine cùng mã thoát.
#[test]
fn main_int_khong_thanh_ma_thoat() {
    let [a, b] = chay_ca_hai(
        "main_int",
        r#"fn main() -> int:
    print("xong")
    5
"#,
    );
    for (name, o) in [("interp", &a), ("vm", &b)] {
        assert_eq!(o.0, 0, "{name}: `main -> int` phải exit 0; stderr={}", o.2);
        assert_eq!(o.1, "xong\n", "{name}: stdout sai");
    }
}
