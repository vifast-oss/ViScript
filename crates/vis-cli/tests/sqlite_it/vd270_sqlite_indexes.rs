// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-indexes)** — `sqlite.indexes(handle, table) -> Result[list[str], Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **tên index do người dùng tạo** của một **bảng** trong **schema chính** (`main`), sắp
//!   theo **byte** (`ORDER BY name`, collation `BINARY`) — **không** phụ thuộc thứ tự khai báo,
//!   giữ nguyên hoa/thường.
//! - Index tự động của SQLite (`sqlite_autoindex_%`, sinh bởi `UNIQUE`/`PRIMARY KEY`) **bị loại**;
//!   index của **bảng khác** không lọt vào kết quả.
//! - Tên bảng được **bind làm tham số** (`tbl_name = ? COLLATE BINARY`), không nối chuỗi SQL:
//!   tên kiểu `t' OR 1=1 --`/`t; DROP TABLE t` chỉ là tên (không có bảng như vậy ⇒ `Ok([])`) và
//!   **không** tác dụng phụ.
//! - Chạy qua **đúng** đường `sqlite.query`/`db_query` hiện có, không thêm method host:
//!   authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên. `sqlite_master` của schema chính
//!   được authorizer cho đọc (probe xác nhận; chỉ `dbstat`/trang thô bị chặn).
//! - Bảng/view **không tồn tại** ⇒ `Ok([])` (view không thể có index); bảng TEMP nằm ở schema
//!   `temp` nên index của nó **không** thuộc kết quả schema chính. Khác hẳn handle thiếu/đã
//!   đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary của cây này @ `c75c0da`):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 6:21 module `std.sqlite` không có hàm `indexes`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:6:21: error VS0301: module `std.sqlite` không có hàm `indexes`
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
        "vis-vd270-sqlite-indexes-{tag}-{}-{n}",
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

/// Chạy hai engine trên cùng nguồn và đòi stdout/stderr/exit **byte-identical**; trả kết quả
/// đường interp. `sqlite.indexes` chỉ đọc nên không cần tệp DB riêng từng engine.
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
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
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

/// Chưa có bảng ⇒ `Ok([])`; 2 index người dùng khai **ngược alphabet** vẫn trả **thứ tự byte**;
/// autoindex của `UNIQUE` bị loại (dù tồn tại trong `sqlite_master`); index bảng khác không lọt;
/// bảng thiếu ⇒ `Ok([])`.
#[test]
fn sap_byte_autoindex_va_bang_khac() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.indexes(h, "chua_co"):
                Ok(xs): print("chua-co={xs.len()} {xs}")
                Err(e): print("chua-co loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT UNIQUE, b INTEGER, c TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX zeta_idx ON t(b)", [])
            let _ = sqlite.execute(h, "CREATE INDEX Alpha_idx ON t(c)", [])
            let _ = sqlite.execute(h, "CREATE TABLE khac (x INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX khac_idx ON khac(x)", [])
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 't' AND name LIKE 'sqlite_autoindex_%'", []):
                Ok(rs): print("auto={rs.rows[0][0]}")
                Err(e): print("auto loi {e.message}")
            match sqlite.indexes(h, "t"):
                Ok(xs): print("t={xs}")
                Err(e): print("t loi {e.message}")
            match sqlite.indexes(h, "khac"):
                Ok(xs): print("khac={xs}")
                Err(e): print("khac loi {e.message}")
            match sqlite.indexes(h, "khong_co"):
                Ok(xs): print("thieu={xs.len()} {xs}")
                Err(e): print("thieu loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("sap-byte", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "chua-co=0 []\nauto=Integer(1)\nt=[Alpha_idx, zeta_idx]\nkhac=[khac_idx]\nthieu=0 []\n",
        "sắp byte (không theo khai báo), loại autoindex UNIQUE, không lẫn index bảng khác, thiếu bảng ⇒ Ok([])"
    );
}

/// View ⇒ `Ok([])` (không thể có index); index của bảng TEMP nằm ở schema `temp` nên **không**
/// thuộc kết quả schema chính, trong khi bảng chính cùng tên vẫn trả index của nó.
#[test]
fn view_va_schema_chinh_khong_loat_index_temp() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_t ON t(a)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tam (q INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE INDEX tam_idx ON tam(q)", [])
            match sqlite.indexes(h, "v_t"):
                Ok(xs): print("view={xs.len()} {xs}")
                Err(e): print("view loi {e.message}")
            match sqlite.indexes(h, "t"):
                Ok(xs): print("t={xs}")
                Err(e): print("t loi {e.message}")
            match sqlite.indexes(h, "tam"):
                Ok(xs): print("temp={xs.len()} {xs}")
                Err(e): print("temp loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("view-temp", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "view=0 []\nt=[idx_t]\ntemp=0 []\n",
        "view ⇒ Ok([]); index bảng TEMP không thuộc schema chính"
    );
}

/// Tên bảng **trông như SQL injection** được bind làm tham số: không có bảng tương ứng ⇒
/// `Ok([])`, và bảng thật + index của nó **không** đổi (không có tác dụng phụ).
#[test]
fn ten_bang_injection_chi_la_ten() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_t ON t(a)", [])
            for ten in ["t' OR 1=1 --", "t; DROP TABLE t", "t\" OR \"1\"=\"1", "t' UNION SELECT name FROM sqlite_master --", "t'; DROP INDEX idx_t; --"]:
                match sqlite.indexes(h, ten):
                    Ok(xs): print("la={xs.len()}")
                    Err(e): print("la loi {e.message}")
            match sqlite.indexes(h, "t"):
                Ok(xs): print("that={xs}")
                Err(e): print("that loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 't'", []):
                Ok(rs): print("so-index={rs.rows[0][0]}")
                Err(e): print("so-index loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("injection", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "la=0\nla=0\nla=0\nla=0\nla=0\nthat=[idx_t]\nso-index=Integer(1)\n",
        "tên injection chỉ là tham số: Ok([]), bảng thật và index không đổi"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0 —
/// không trap, không panic; khác với bảng thiếu ⇒ `Ok([])`.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_t ON t(a)", [])
            let _ = sqlite.close(h)
            match sqlite.indexes(h, "t"):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.indexes("db:00000000000000000000000000000000", "t"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.indexes("khong-phai-handle", "t"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.indexes("", "t"):
                Ok(_): print("rong khong duoc phep")
                Err(e): print("rong bi chan")
            match sqlite.indexes("db:00000000000000000000000000000000", "khong_co"):
                Ok(_): print("la+thieu khong duoc phep")
                Err(e): print("la+thieu bi chan")
        Err(e): print("mo loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "lỗi handle phải là Err dữ liệu; stderr: {err}");
    assert_eq!(
        out, "dong bi chan\nla bi chan\nsai dang bi chan\nrong bi chan\nla+thieu bi chan\n",
        "mọi handle hỏng ⇒ Err(Error), không trap (kể cả khi tên bảng cũng thiếu)"
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "sqlite.indexes(1, \"t\")", "VS0412"),
        ("kieu-2", "sqlite.indexes(\"h\", 1)", "VS0412"),
        ("kieu-2-bool", "sqlite.indexes(\"h\", true)", "VS0412"),
        ("arity-0", "sqlite.indexes()", "VS0401"),
        ("arity-1", "sqlite.indexes(\"h\")", "VS0401"),
        ("arity-3", "sqlite.indexes(\"h\", \"t\", \"x\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
