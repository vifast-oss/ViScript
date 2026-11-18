// SPDX-License-Identifier: MIT OR Apache-2.0
//! **T-01** — lỗi kiểu **trong** nhóm `( … )` của `.vish` phải trỏ đúng biểu thức con (`dòng:cột`)
//! như `.vis` (VD-80 / ST-46: chẩn đoán tuyệt đối trong ngoặc).
//!
//! Gốc lỗi: `lower_bieu_thuc_tai` đệm `bat_dau - len("let __vach = ")` dấu cách; khi biểu thức bắt
//! đầu trước byte 13 (nhóm ở **đầu tệp**) phép trừ bão hoà về 0 ⇒ span lệch sang phải (về cuối
//! dòng / dòng sau). Mỗi ca so `dòng:cột` của `VS0412` ở `vis check`, `vis run` và `vis run --vm`.
//!
//! Tệp `.vish` tạm **không** có dòng SPDX: lỗi chỉ xuất hiện khi nhóm nằm trong 13 byte đầu tệp.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-nvt-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// `dòng:cột` của `ma` trong `vis check` (dòng TAP `# <ma> <dòng:cột> …`).
///
/// `ma` là tham số vì **R9 (VD-178)** tách `VS0412` thành 4 nghĩa: `[1, "a"]` (kiểu phần tử
/// không khớp) vẫn `VS0412`, còn `1 + "a"` (toán tử không hỗ trợ kiểu) nay `VS0414`. PR #27 viết
/// trước thay đổi đó nên bản gốc khoá cứng `VS0412`; ở đây giữ nguyên **ý định** (so vị trí) và
/// truyền mã đúng theo từng ca.
fn vi_tri_check(p: &Path, ma: &str) -> String {
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(p)
        .output()
        .expect("chạy vis check");
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    out.lines()
        .find_map(|l| l.strip_prefix(&format!("# {ma} ")))
        .and_then(|r| r.split_whitespace().next())
        .map(str::to_string)
        .unwrap_or_else(|| format!("<không có {ma} trong stdout: {out}>"))
}

/// `dòng:cột` của `ma` trong `vis run` (`<tệp>.vish:<dòng>:<cột>: error <ma>: …`).
fn vi_tri_run(p: &Path, vm: bool, ma: &str) -> String {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run");
    if vm {
        c.arg("--vm");
    }
    let o = c.arg(p).output().expect("chạy vis run");
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    err.lines()
        .find(|l| l.contains(&format!(": error {ma}:")))
        .and_then(|l| l.split(".vish:").nth(1))
        .and_then(|r| r.split(": error").next())
        .map(str::to_string)
        .unwrap_or_else(|| format!("<không có {ma} trong stderr: {err}>"))
}

/// Ghi `src` (nguyên văn, không thêm gì) vào tệp tạm rồi so `(check, run, run --vm)` với `mong`.
fn kiem(tag: &str, src: &str, ma: &str, mong: &str) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.vish"));
    std::fs::write(&p, src).expect("ghi");
    let thuc = (
        vi_tri_check(&p, ma),
        vi_tri_run(&p, false, ma),
        vi_tri_run(&p, true, ma),
    );
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        thuc,
        (mong.to_string(), mong.to_string(), mong.to_string()),
        "(check, run, run --vm) cho {src:?}"
    );
}

#[test]
fn list_dau_tep_tro_phan_tu_sai() {
    kiem("list", "( [1, \"a\"] )\n", "VS0412", "1:7");
}

#[test]
fn cong_dau_tep_tro_toan_hang_trai() {
    kiem("cong", "( 1 + \"a\" )\n", "VS0414", "1:3");
}

#[test]
fn nhom_roi_take_tro_phan_tu_sai() {
    kiem("take", "( [1, \"a\"] ) | take 1\n", "VS0412", "1:7");
}

#[test]
fn nhom_dong_hai_tro_phan_tu_sai() {
    kiem("dong2", "echo 1\n( [1, \"a\"] )\n", "VS0412", "2:7");
}

#[test]
fn khong_xuong_dong_cuoi_tep() {
    kiem("khongnl", "( [1, \"a\"] )", "VS0412", "1:7");
}

/// Đối chứng: biểu thức bắt đầu từ byte 13 trở đi đã đúng trên `origin/main`; phải **giữ** đúng.
#[test]
fn giu_dung_khi_bieu_thuc_xa_dau_tep() {
    kiem("xa", "(               [1, \"a\"] )\n", "VS0412", "1:21");
}

// ---- Sửa vòng 1 (sau DRIFT): lambda `fn` ở đầu tệp phải vẫn là **biểu thức** ----
// Ở đầu câu lệnh parser hiểu `fn` là khai báo; nhóm ở đầu tệp vẫn phải hạ như `let __vach = fn…`.

/// Mã thoát của `vis <args…> <p>`.
fn ma_thoat(p: &Path, args: &[&str]) -> i32 {
    Command::new(env!("CARGO_BIN_EXE_vis"))
        .args(args)
        .arg(p)
        .output()
        .expect("chạy vis")
        .status
        .code()
        .unwrap_or(-1)
}

#[test]
fn lambda_dau_tep_van_la_bieu_thuc() {
    let d = tam("lam");
    let p = d.join("lam.vish");
    std::fs::write(&p, "(fn(x: int) -> int: x)\n").expect("ghi");
    let thuc = (
        ma_thoat(&p, &["check"]),
        ma_thoat(&p, &["run"]),
        ma_thoat(&p, &["run", "--vm"]),
    );
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        thuc,
        (0, 0, 0),
        "(check, run, run --vm) cho lambda ở đầu tệp"
    );
}

#[test]
fn lambda_dau_tep_tro_toan_hang_trai() {
    kiem(
        "lamcong",
        "(fn(x: int) -> int: x + \"a\")\n",
        "VS0414",
        "1:21",
    );
}

#[test]
fn lambda_dong_hai_tro_toan_hang_trai() {
    kiem(
        "lamdong2",
        "echo 1\n( fn(x: int) -> int: x + \"a\" )\n",
        "VS0414",
        "2:22",
    );
}

#[test]
fn lambda_roi_take_tro_toan_hang_trai() {
    kiem(
        "lamtake",
        "( fn(x: int) -> int: x + \"a\" ) | take 1\n",
        "VS0414",
        "1:22",
    );
}
