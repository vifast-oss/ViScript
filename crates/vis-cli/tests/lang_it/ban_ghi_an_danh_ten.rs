// SPDX-License-Identifier: MIT OR Apache-2.0
//! **T-02** — chẩn đoán in bản ghi **ẩn danh** theo cú pháp kiểu bản ghi `{a: int}` thay vì mã
//! nội bộ `record#N` (số đổi theo prelude: `.vis` ra `record#0`, `.vish` ra `record#3`).
//!
//! Khoá: `.vis` và `.vish` cho **cùng** chữ; bản ghi lồng in đệ quy; trường theo thứ tự khai báo;
//! bản ghi **có tên** (`type P:`) vẫn in tên; mã lỗi `VS0412` và phần chữ khác không đổi.

use std::path::PathBuf;
use std::process::Command;

fn tam(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vis-t02-ban-ghi-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("thư mục tạm");
    d
}

/// Ghi `src` vào `<tag>.<duoi>` rồi chạy `vis check`; trả `(exit, stdout + stderr)`.
fn check(tag: &str, duoi: &str, src: &str) -> (i32, String) {
    let d = tam(tag);
    let p = d.join(format!("{tag}.{duoi}"));
    std::fs::write(&p, src).expect("ghi");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy `vis check`");
    let _ = std::fs::remove_dir_all(&d);
    (
        o.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

/// Đòi: thoát 1, có `VS0412`, có đúng chuỗi `can`, **không** còn `record#`.
fn doi_loi(tag: &str, duoi: &str, src: &str, can: &str) {
    let (c, out) = check(tag, duoi, src);
    assert_eq!(c, 1, "phải báo lỗi kiểu, ra:\n{out}");
    assert!(out.contains("VS0412"), "thiếu mã `VS0412`, ra:\n{out}");
    assert!(out.contains(can), "thiếu `{can}`, ra:\n{out}");
    assert!(
        !out.contains("record#"),
        "còn mã nội bộ `record#`, ra:\n{out}"
    );
}

const LIST_HAI_BAN_GHI: &str = "let xs = [{a: 1}, {b: 2}]\n";
const CHU_LIST: &str = "phần tử `list`: cần `{a: int}`, được `{b: int}`";

#[test]
fn an_danh_trong_list_vis() {
    doi_loi("list-vis", "vis", LIST_HAI_BAN_GHI, CHU_LIST);
}

#[test]
fn an_danh_trong_list_vish_cung_chu() {
    // `.vish` nạp prelude khác ⇒ id bản ghi khác (`record#3`/`record#4` trước đây); chữ phải **y hệt** `.vis`.
    doi_loi("list-vish", "vish", LIST_HAI_BAN_GHI, CHU_LIST);
}

#[test]
fn let_chu_thich_kieu_an_danh() {
    doi_loi(
        "let-chu-thich",
        "vis",
        "let r: {a: int} = {b: 2}\n",
        "khai báo `let`: cần `{a: int}`, được `{b: int}`",
    );
}

#[test]
fn ban_ghi_long_in_de_quy() {
    doi_loi(
        "long",
        "vis",
        "let xs = [{p: {q: \"x\"}}, {p: {q: 1}}]\n",
        "phần tử `list`: cần `{p: {q: str}}`, được `{p: {q: int}}`",
    );
}

#[test]
fn truong_theo_thu_tu_khai_bao() {
    doi_loi(
        "thu-tu",
        "vis",
        "let xs = [{b: 1, a: \"s\"}, {a: 2}]\n",
        "phần tử `list`: cần `{b: int, a: str}`, được `{a: int}`",
    );
}

#[test]
fn ban_ghi_co_ten_van_in_ten() {
    doi_loi(
        "co-ten",
        "vis",
        "type P:\n    a: int\nlet x: P = {b: 2}\n",
        "khai báo `let`: cần `P`, được `{b: int}`",
    );
}
