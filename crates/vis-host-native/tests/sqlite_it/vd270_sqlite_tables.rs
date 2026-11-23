// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-tables)** — `sqlite.tables(handle) -> Result[list[str], Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **tên** bảng + view **người dùng** của **schema chính** connection, sắp theo **byte**
//!   (`ORDER BY name`, collation `BINARY`; tên giữ nguyên hoa/thường), bỏ đối tượng nội bộ có
//!   tiền tố `sqlite_` (`sqlite_sequence`, `sqlite_stat*`, `sqlite_master`…).
//! - Chạy qua **đúng** đường `sqlite.query` hiện có (`sqlite_master`), không thêm method host:
//!   authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên. `sqlite_master` được authorizer
//!   cho đọc (probe xác nhận; chỉ `dbstat`/trang thô bị chặn).
//! - DB rỗng ⇒ `Ok([])`; handle thiếu/đã đóng/sai dạng hoặc lỗi host ⇒ `Err(Error)` **dữ liệu**,
//!   không trap.
//! - **Bảng TEMP không xuất hiện** (hợp đồng **có chủ ý**): bảng tạm nằm ở schema `temp`
//!   (`sqlite_temp_master`), không thuộc `sqlite_master` của schema chính; `tables` mô tả schema
//!   chính của connection/tệp DB. Test `bang_tam_va_noi_bo` chứng minh bảng tạm vẫn tồn tại và
//!   truy vấn được nhưng **không** có trong kết quả.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary của cây này @ `e17998f`):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 7:19 module `std.sqlite` không có hàm `tables`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:7:19: error VS0301: module `std.sqlite` không có hàm `tables`
//! $ vis run --no-cache --vm probe.vis  ⇒ exit 2 (cùng dòng)
//! ```
//!
//! Sau khi thêm: mọi ca dưới xanh, interp ↔ VM byte-identical.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-sqlite-tables-{tag}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("tạo thư mục tạm");
    d
}

fn ghi(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(format!("{ten}.vis"));
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi chương trình tạm");
    p
}

fn chay(p: &Path, vm: bool) -> (i32, String, String) {
    let mut c = Command::new(crate::common::vis_bin());
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

/// Chạy hai engine trên cùng nguồn và đòi stdout/stderr/exit **byte-identical**; trả kết quả
/// đường interp. `sqlite.tables` chỉ đọc nên không cần tệp DB riêng từng engine.
fn hai_engine(tag: &str, src: &str) -> (i32, String, String) {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let a = chay(&p, false);
    let b = chay(&p, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a, b, "{tag}: interp ≠ `--vm`\n  interp: {a:?}\n  vm: {b:?}");
    a
}

/// `vis check` phải **đỏ** (exit 1) với mã chẩn đoán `ma`; `vis run` cả hai engine từ chối tĩnh
/// (exit 2) cùng mã — không chạy chương trình.
fn tu_choi(tag: &str, src: &str, ma: &str) {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let o = Command::new(crate::common::vis_bin())
        .arg("check")
        .arg(&p)
        .output()
        .expect("chạy vis check");
    let co = String::from_utf8_lossy(&o.stdout).to_string();
    let ce = String::from_utf8_lossy(&o.stderr).to_string();
    assert_eq!(
        o.status.code().unwrap_or(-1),
        1,
        "{tag}: `check` phải đỏ (exit 1)\nstdout: {co}\nstderr: {ce}"
    );
    assert!(
        co.contains(ma) || ce.contains(ma),
        "{tag}: `check` thiếu `{ma}`\nstdout: {co}\nstderr: {ce}"
    );
    for (i, vm) in [false, true].iter().enumerate() {
        let (code, so, se) = chay(&p, *vm);
        assert_eq!(
            code, 2,
            "{tag} engine={i}: phải từ chối tĩnh (exit 2), được {code}\nstdout: {so}\nstderr: {se}"
        );
        assert!(
            so.contains(ma) || se.contains(ma),
            "{tag} engine={i}: thiếu `{ma}`\nstdout: {so}\nstderr: {se}"
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// DB rỗng ⇒ `Ok([])`; 2 bảng + 1 view ⇒ đúng 3 tên, sắp **byte** (hoa trước thường), giữ
/// nguyên chữ hoa/thường (`Alpha`).
#[test]
fn rong_va_bang_view_sap_byte() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.tables(h):
                Ok(xs): print("rong={xs.len()} {xs}")
                Err(e): print("rong loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE zeta (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TABLE Alpha (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_alpha AS SELECT a FROM Alpha", [])
            match sqlite.tables(h):
                Ok(xs):
                    print("n={xs.len()}")
                    for x in xs: print(x)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("rong", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "rong=0 []\nn=3\nAlpha\nv_alpha\nzeta\n",
        "DB rỗng ⇒ Ok([]); 2 bảng + 1 view sắp byte, giữ nguyên hoa/thường"
    );
}

/// Bảng TEMP **không** thuộc schema chính nên không xuất hiện (vẫn tồn tại + truy vấn được);
/// đối tượng nội bộ `sqlite_sequence` bị loại; `sqliteXtra` (người dùng, **không** phải tiền tố
/// `sqlite_`) vẫn xuất hiện — escape `_` chỉ loại đúng tiền tố `sqlite_`.
#[test]
fn bang_tam_va_noi_bo() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE chinh (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tam (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TABLE tang (id INTEGER PRIMARY KEY AUTOINCREMENT)", [])
            let _ = sqlite.execute(h, "CREATE TABLE sqliteXtra (a INTEGER)", [])
            match sqlite.tables(h):
                Ok(xs): print(xs)
                Err(e): print("loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM tam", []):
                Ok(rs): print("tam={rs.rows[0][0]}")
                Err(e): print("tam loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE name = 'sqlite_sequence'", []):
                Ok(rs): print("sequence={rs.rows[0][0]}")
                Err(e): print("sequence loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("tam-noi-bo", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "[chinh, sqliteXtra, tang]\ntam=Integer(0)\nsequence=Integer(1)\n",
        "bảng TEMP + `sqlite_%` không xuất hiện; `sqliteXtra` người dùng vẫn có; bảng tạm vẫn truy vấn được"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0 —
/// không trap, không panic.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.close(h)
            match sqlite.tables(h):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.tables("db:00000000000000000000000000000000"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.tables("khong-phai-handle"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.tables(""):
                Ok(_): print("rong khong duoc phep")
                Err(e): print("rong bi chan")
        Err(e): print("mo loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "lỗi handle phải là Err dữ liệu; stderr: {err}");
    assert_eq!(
        out, "dong bi chan\nla bi chan\nsai dang bi chan\nrong bi chan\n",
        "mọi handle hỏng ⇒ Err(Error), không trap"
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu", "sqlite.tables(1)", "VS0412"),
        ("kieu-bool", "sqlite.tables(true)", "VS0412"),
        ("arity-0", "sqlite.tables()", "VS0401"),
        ("arity-2", "sqlite.tables(\"a\", \"b\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
