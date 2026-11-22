// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-262** — checker phải kiểm **thân phương thức `impl`** với **cùng ngữ cảnh** như hàm cấp
//! tệp: kiểu trả, tham số/`self`, phạm vi cục bộ, độ sâu vòng lặp (`break`/`continue`) và tên
//! chưa khai (`VS0301`).
//!
//! Trước VD-262, `vis check` bỏ qua thân phương thức (chỉ kiểm đích gán) ⇒ `break` ngoài vòng
//! hoặc tên lạ trong method **lọt**; `--vm` lại lỗi codegen ⇒ hai đường lệch nhau.
//!
//! Quy ước mã thoát đã khoá (ST-54): `vis check` = **1** khi có chẩn đoán; `vis run`/`run --vm`
//! từ chối **tĩnh** = **2**. Ca dương chạy **cả hai engine**, so byte stdout/stderr/exit.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd262-{tag}-{}-{n}", std::process::id()));
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

fn chay_tep(p: &Path, extra: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.arg("run").arg("--no-cache").args(extra).arg(p);
    let o = c.output().expect("chạy vis run");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

/// Chương trình **không hợp lệ**: `check` đỏ (exit 1 — ST-54) + đúng mã; **cả hai engine**
/// `run` cũng từ chối tĩnh (exit 2) với cùng mã, không để lệch interp ↔ `--vm`.
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let (c, out, err) = check_src(tag, src);
    assert_eq!(
        c, 1,
        "{tag}: `vis check` phải đỏ (exit 1)\nstdout: {out}\nstderr: {err}"
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

/// Chương trình hợp lệ: chạy interp + `--vm`, đòi exit/stdout/stderr **byte-khớp**.
fn chay_ca_hai(tag: &str, src: &str, muon: &str) {
    let (d, p) = ghi_tep(tag, src);
    let a = chay_tep(&p, &[]);
    let b = chay_tep(&p, &["--vm"]);
    assert_eq!(a.0, b.0, "{tag}: exit interp↔VM lệch");
    assert_eq!(a.1, b.1, "{tag}: stdout interp↔VM lệch");
    assert_eq!(a.2, b.2, "{tag}: stderr interp↔VM lệch");
    assert_eq!(a.0, 0, "{tag}: stderr: {}", a.2);
    assert_eq!(a.1, muon, "{tag}: stdout sai");
    let _ = std::fs::remove_dir_all(&d);
}

const P: &str = "type P:\n    x: int\n\n";

/// `break` ngoài vòng lặp trong **method** ⇒ `VS0412` (trước: `check` xanh, `--vm` lỗi codegen).
#[test]
fn break_ngoai_vong_trong_phuong_thuc_bi_tu_choi() {
    tu_choi(
        "brk_method",
        &format!("{P}impl P:\n    fn f(self):\n        break\n\nfn main():\n    let p: P = {{x: 1}}\n    print(p.x)\n"),
        "VS0412",
    );
}

/// `continue` ngoài vòng lặp trong **method** ⇒ `VS0412`.
#[test]
fn continue_ngoai_vong_trong_phuong_thuc_bi_tu_choi() {
    tu_choi(
        "cont_method",
        &format!("{P}impl P:\n    fn f(self):\n        continue\n\nfn main():\n    let p: P = {{x: 1}}\n    print(p.x)\n"),
        "VS0412",
    );
}

/// `break` ở **vị trí cần giá trị** trong method (nhánh `if` của `let`) ⇒ `VS0412`.
#[test]
fn break_o_vi_tri_gia_tri_trong_phuong_thuc_bi_tu_choi() {
    tu_choi(
        "brk_value_method",
        &format!(
            "{P}impl P:\n    fn f(self) -> int:\n        loop:\n            let x = if true:\n                break\n            else:\n                1\n            return x\n\nfn main():\n    let p: P = {{x: 1}}\n    print(p.f())\n"
        ),
        "VS0412",
    );
}

/// Tên giá trị chưa khai trong **method** ⇒ `VS0301` (trước: `check` xanh, hai engine chạy).
#[test]
fn ten_la_trong_phuong_thuc_bi_tu_choi() {
    tu_choi(
        "ten_method",
        &format!("{P}impl P:\n    fn f(self) -> int:\n        khong_ton_tai\n\nfn main():\n    let p: P = {{x: 1}}\n    print(p.x)\n"),
        "VS0301",
    );
}

/// Kiểu trả của method được dùng làm ngữ cảnh: `-> int` mà trả `str` ⇒ `VS0412`.
#[test]
fn tra_ve_sai_kieu_trong_phuong_thuc_bi_tu_choi() {
    tu_choi(
        "ret_method",
        &format!("{P}impl P:\n    fn f(self) -> int:\n        \"sai\"\n\nfn main():\n    let p: P = {{x: 1}}\n    print(p.x)\n"),
        "VS0412",
    );
}

/// Tham số method + vòng lặp hợp lệ: `continue`/`break` vẫn chạy đúng trên **cả hai engine**.
#[test]
fn vong_lap_trong_phuong_thuc_chay_hai_engine() {
    chay_ca_hai(
        "loop_method",
        &format!(
            "{P}impl P:\n    fn dem(self, n: int) -> int:\n        var i = 0\n        var tong = 0\n        while i < n:\n            i += 1\n            if i % 2 == 0:\n                continue\n            if i > 7:\n                break\n            tong += i\n        return tong\n\nfn main():\n    let p: P = {{x: 1}}\n    print(p.dem(10))\n"
        ),
        "16\n",
    );
}
