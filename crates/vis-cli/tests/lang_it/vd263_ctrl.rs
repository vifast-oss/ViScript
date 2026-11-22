// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-263 — `break`/`continue` trong biểu thức `if`/`match` phải cùng một ngữ nghĩa ở hai engine**.
//!
//! Trước bản sửa: interp gọi `exec_body` cho nhánh `if`/`match` ở **vị trí giá trị** và nuốt mọi
//! `Ctrl` khác `Normal` thành `()`, nên `let x = if true: break 0 else 0` chạy tiếp vòng lặp;
//! VM phát `Jump` tới cuối vòng nên thoát ngay. Chương trình dưới `vis check` **xanh** ở cả hai
//! đường trước khi sửa (kiểm VD-259 chỉ từ chối `break`/`continue` là **câu cuối** của nhánh cần
//! giá trị — ở đây còn câu `0` phía sau nên hợp lệ).
//!
//! Hợp đồng sau bản sửa:
//! 1. `break`/`continue` phát sinh trong **bất kỳ** biểu thức con nào của thân câu lệnh (nhánh
//!    `if`/`match` giá trị, đối số gọi hàm, toán hạng, …) được truyền tới vòng lặp gần nhất
//!    **trong cùng hàm**, đúng như `Jump` của VM — không bị đổi thành `()`.
//! 2. `return`/`?` trong cùng vị trí vẫn thoát hàm như cũ (không đổi).
//! 3. `break`/`continue` là **câu cuối** của nhánh cần giá trị vẫn bị checker từ chối (`VS0412`,
//!    VD-259) ở **cả hai** engine — không nới lỏng để xanh.
//!
//! Mỗi ca hợp lệ chạy **cả hai engine** trên cùng tệp: `vis run --no-cache` (interp) và
//! `vis run --no-cache --vm`; so byte exit/stdout/stderr. Ca bị từ chối tĩnh chỉ so exit/mã.

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

fn chay_tep(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy `vis run`");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chạy interp + VM trên cùng tệp, trả `[interp, vm]`.
fn hai_duong(tag: &str, src: &str) -> [(i32, String, String); 2] {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi tệp tạm");
    let a = chay_tep(&p, false);
    let b = chay_tep(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    [a, b]
}

/// Ca thành công: đòi exit + stdout + stderr **byte-khớp**, `exit = 0`, stderr rỗng.
fn thanh_cong(tag: &str, src: &str, stdout: &str) {
    let [a, b] = hai_duong(tag, src);
    assert_eq!(
        (a.0, &a.1, &a.2),
        (b.0, &b.1, &b.2),
        "{tag}: interp ≠ VM (exit/stdout/stderr)\n  interp: {a:?}\n  vm: {b:?}"
    );
    assert_eq!(a.0, 0, "{tag}: phải chạy sạch, stderr: {}", a.2);
    assert_eq!(a.1, stdout, "{tag}: stdout");
    assert_eq!(a.2, "", "{tag}: stderr phải rỗng");
}

/// Ca bị **checker** từ chối tĩnh: cả hai engine `vis run` đều exit 2 + mã `VS0412`.
fn tu_choi(tag: &str, src: &str) {
    let [a, b] = hai_duong(tag, src);
    for (i, r) in [a, b].iter().enumerate() {
        let (code, out, err) = r;
        assert_eq!(
            *code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {out}\nstderr: {err}"
        );
        assert!(
            out.contains("VS0412") || err.contains("VS0412"),
            "{tag} engine={i}: thiếu mã `VS0412`\nstdout: {out}\nstderr: {err}"
        );
    }
}

// ===========================================================================
// 1 — Ca gốc của review: `break` trong nhánh `if` ở vị trí giá trị của `let`
// ===========================================================================

#[test]
fn if_break_trong_let_thoat_vong_ngay() {
    thanh_cong(
        "if-break",
        r#"fn main():
    var i = 0
    loop:
        i = i + 1
        if i == 1:
            let x = if true:
                break
                0
            else:
                0
        if i >= 3:
            break
    print(i)
"#,
        "1\n",
    );
}

// ===========================================================================
// 2 — `continue` trong nhánh `if` ở vị trí giá trị: bỏ phần còn lại của vòng
// ===========================================================================

#[test]
fn if_continue_trong_let_bo_phan_con_lai_cua_vong() {
    thanh_cong(
        "if-continue",
        r#"fn main():
    var dem = 0
    var i = 0
    loop:
        i = i + 1
        let x = if i == 2:
            continue
            0
        else:
            0
        dem = dem + 1
        if i >= 5:
            break
    print(dem)
    print(i)
"#,
        "4\n5\n",
    );
}

// ===========================================================================
// 3 — `break` trong nhánh `match` ở vị trí giá trị của `let`
// ===========================================================================

#[test]
fn match_break_trong_let_thoat_vong_ngay() {
    thanh_cong(
        "match-break",
        r#"fn main():
    var i = 0
    loop:
        i = i + 1
        let x = match i:
            1:
                break
                0
            2:
                7
            _:
                0
        if i >= 3:
            break
    print(i)
"#,
        "1\n",
    );
}

// ===========================================================================
// 4 — `continue` trong nhánh `match` ở vị trí giá trị của `let`
// ===========================================================================

#[test]
fn match_continue_trong_let_bo_phan_con_lai_cua_vong() {
    thanh_cong(
        "match-continue",
        r#"fn main():
    var dem = 0
    var i = 0
    loop:
        i = i + 1
        let x = match i:
            2:
                continue
                0
            _:
                0
        dem = dem + 1
        if i >= 4:
            break
    print(dem)
    print(i)
"#,
        "3\n4\n",
    );
}

// ===========================================================================
// 5 — Biểu thức `if`/`match` **không chỉ** ở vế `let`: vế phải phép gán (câu
//     lệnh biểu thức) cũng phải truyền `break` ra vòng lặp, không gán `()`
// ===========================================================================

#[test]
fn if_break_trong_phep_gan_truyen_ra_vong_lap() {
    thanh_cong(
        "assign-break",
        r#"fn main():
    var i = 0
    var z = 0
    loop:
        i = i + 1
        z = if i == 1:
            break
            5
        else:
            2
        if i >= 3:
            break
    print(i)
    print(z)
"#,
        "1\n0\n",
    );
}

/// `break` trong nhánh `match` ở **vế phải phép gán** — cùng đường lan truyền.
#[test]
fn match_break_trong_phep_gan_truyen_ra_vong_lap() {
    thanh_cong(
        "assign-match-break",
        r#"fn main():
    var i = 0
    var z = 0
    loop:
        i = i + 1
        z = match i:
            1:
                break
                0
            _:
                1
        if i >= 3:
            break
    print(i)
    print(z)
"#,
        "1\n0\n",
    );
}

// ===========================================================================
// 6 — `return`/`?` trong biểu thức `if` vẫn thoát **hàm** (không bị đổi thành
//     `break` hay trap) — hành vi cũ giữ nguyên
// ===========================================================================

#[test]
fn return_trong_if_van_thoat_ham() {
    thanh_cong(
        "ret-if",
        r#"fn f(n: int) -> int:
    var i = 0
    loop:
        i = i + 1
        let x = if i == n:
            return i
        else:
            0
        if i >= 10:
            break
    return 0

fn main():
    print(f(3))
"#,
        "3\n",
    );
}

// ===========================================================================
// 8 — Hồi quy: biểu thức `if`/`match` cho giá trị bình thường, `break`/`continue`
//     ở vị trí câu lệnh, và vòng lặp `while` lồng — hành vi **không đổi**
// ===========================================================================

#[test]
fn gia_tri_if_match_va_vong_lap_binh_thuong_khong_doi() {
    thanh_cong(
        "normal-values",
        r#"fn f(n: int) -> int:
    let a = if n > 1:
        10
    else:
        20
    let b = match a:
        10:
            1
        20:
            2
        _:
            3
    return a + b

fn main():
    var i = 0
    var tong = 0
    while i < 5:
        i += 1
        if i == 2:
            continue
        if i == 5:
            break
        tong += i
    print(f(2))
    print(f(0))
    print(tong)
    print(i)
"#,
        "11\n22\n8\n5\n",
    );
}

// ===========================================================================
// 9 — `break`/`continue` là **câu cuối** của nhánh cần giá trị vẫn bị từ chối
//     tĩnh, thống nhất hai engine (VD-259, không nới lỏng)
// ===========================================================================

#[test]
fn break_cuoi_nhanh_gia_tri_bi_tu_choi() {
    tu_choi(
        "brk-tail",
        r#"fn main():
    var i = 0
    loop:
        i = i + 1
        let x = if true:
            break
        else:
            0
    print(i)
"#,
    );
}

#[test]
fn continue_cuoi_nhanh_gia_tri_bi_tu_choi() {
    tu_choi(
        "cont-tail",
        r#"fn main():
    var i = 0
    loop:
        i = i + 1
        let x = match i:
            1:
                continue
            _:
                0
    print(i)
"#,
    );
}
