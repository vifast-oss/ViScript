// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-table-exists)** — `sqlite.table_exists(handle, name) -> Result[bool, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - `true` **khi và chỉ khi** schema **chính** (`main`) có **bảng hoặc view** tên đúng:
//!   `SELECT 1 FROM sqlite_master WHERE type IN ('table','view') AND name = ? LIMIT 1`.
//!   Index/trigger không tính (`type` khác); bảng TEMP nằm ở schema `temp`
//!   (`sqlite_temp_master`) nên **không** thuộc schema chính ⇒ `false`.
//! - Tên là **tham số bind** (`SqlValue.Text`), không nối chuỗi SQL: tên kiểu `t' OR 1=1 --`/
//!   `t; DROP TABLE t` chỉ là tên (không có đối tượng như vậy ⇒ `Ok(false)`) và **không** tác
//!   dụng phụ. Hàm **chỉ đọc**, đi qua **đúng** đường `sqlite.query`/`db_query` hiện có (không
//!   thêm method host): authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên.
//! - So khớp **phân biệt hoa/thường** (`name = ?` dùng collation `BINARY` mặc định):
//!   `CREATE TABLE Bang` tra `Bang` ⇒ `true`, tra `bang`/`BANG` ⇒ `false` — hợp đồng **có chủ ý**.
//! - **Không có tên ⇒ `Ok(false)`**. Đối tượng nội bộ `sqlite_%` **không** đặc cách ngoài truy
//!   vấn: `sqlite_sequence` (`AUTOINCREMENT`, `type = 'table'`) ⇒ `true`; `sqlite_master` không
//!   có hàng trong chính nó ⇒ `false`; autoindex `sqlite_autoindex_%` (`type = 'index'`) ⇒ `false`.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary của cây này @ `0eb6b32`):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 8:19 module `std.sqlite` không có hàm `table_exists`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:8:19: error VS0301: module `std.sqlite` không có hàm `table_exists`
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
        "vis-vd270-sqlite-table-exists-{tag}-{}-{n}",
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
/// đường interp. `sqlite.table_exists` chỉ đọc nên không cần tệp DB riêng từng engine.
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

/// Bảng ⇒ `true`, view ⇒ `true`, tên thiếu/rỗng ⇒ `false`; index/trigger cùng tên hoặc khác
/// cũng **không** tính (chỉ `table`/`view`).
#[test]
fn bang_view_true_thieu_index_trigger_false() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_t ON t(a)", [])
            let _ = sqlite.execute(h, "CREATE TRIGGER trg_t AFTER INSERT ON t BEGIN UPDATE t SET a = a; END", [])
            for ten in ["t", "v_t", "khong_co", "idx_t", "trg_t", ""]:
                match sqlite.table_exists(h, ten):
                    Ok(b): print("{ten}={b}")
                    Err(e): print("{ten} loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("bang-view", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "t=true\nv_t=true\nkhong_co=false\nidx_t=false\ntrg_t=false\n=false\n",
        "bảng/view ⇒ true; thiếu/rỗng/index/trigger ⇒ false"
    );
}

/// So khớp **BINARY** — phân biệt hoa/thường: `Bang` ⇒ `true`, `bang`/`BANG` ⇒ `false`.
#[test]
fn phan_biet_hoa_thuong() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE Bang (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE VIEW V_Bang AS SELECT a FROM Bang", [])
            for ten in ["Bang", "bang", "BANG", "V_Bang", "v_bang"]:
                match sqlite.table_exists(h, ten):
                    Ok(b): print("{ten}={b}")
                    Err(e): print("{ten} loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("case", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "Bang=true\nbang=false\nBANG=false\nV_Bang=true\nv_bang=false\n",
        "tên đúng hoa/thường ⇒ true; khác case ⇒ false (BINARY)"
    );
}

/// Bảng TEMP **không** thuộc schema chính ⇒ `false` (vẫn tồn tại + truy vấn được); đối tượng
/// nội bộ theo **đúng truy vấn**: `sqlite_sequence` (`type = 'table'`) ⇒ `true`,
/// `sqlite_master`/autoindex ⇒ `false`; TEMP trùng tên bảng chính không che bảng chính.
#[test]
fn bang_tam_va_noi_bo_theo_truy_van() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE chinh (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TABLE trung (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tam (a INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE trung (b INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE TEMP VIEW v_tam AS SELECT a FROM tam", [])
            let _ = sqlite.execute(h, "CREATE TABLE seq (id INTEGER PRIMARY KEY AUTOINCREMENT)", [])
            let _ = sqlite.execute(h, "CREATE TABLE co_autoindex (a TEXT UNIQUE)", [])
            for ten in ["chinh", "tam", "trung", "v_tam", "sqlite_sequence", "sqlite_master", "sqlite_autoindex_co_autoindex_1"]:
                match sqlite.table_exists(h, ten):
                    Ok(b): print("{ten}={b}")
                    Err(e): print("{ten} loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM tam", []):
                Ok(rs): print("tam={rs.rows[0][0]}")
                Err(e): print("tam loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("tam-noi-bo", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "chinh=true\n\
         tam=false\n\
         trung=true\n\
         v_tam=false\n\
         sqlite_sequence=true\n\
         sqlite_master=false\n\
         sqlite_autoindex_co_autoindex_1=false\n\
         tam=Integer(0)\n",
        "TEMP ⇒ false (vẫn truy vấn được); TEMP trùng tên không che bảng chính; \
         `sqlite_sequence` true, `sqlite_master`/autoindex false"
    );
}

/// Tên **trông như SQL injection** được bind làm tham số: không khớp đối tượng nào ⇒ `false`,
/// và bảng thật + dữ liệu + schema **không** đổi (không tác dụng phụ).
#[test]
fn ten_injection_chi_la_ten() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            for ten in ["t' OR 1=1 --", "t; DROP TABLE t", "t\" OR \"1\"=\"1", "t' UNION SELECT 1 FROM sqlite_master --", "t'; DROP TABLE t; --"]:
                match sqlite.table_exists(h, ten):
                    Ok(b): print("injection={b}")
                    Err(e): print("injection loi {e.message}")
            match sqlite.table_exists(h, "t"):
                Ok(b): print("t={b}")
                Err(e): print("t loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("rows={rs.rows[0][0]}")
                Err(e): print("rows loi {e.message}")
            match sqlite.tables(h):
                Ok(xs): print("tables={xs}")
                Err(e): print("tables loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("injection", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "injection=false\n\
         injection=false\n\
         injection=false\n\
         injection=false\n\
         injection=false\n\
         t=true\n\
         rows=Integer(1)\n\
         tables=[t]\n",
        "mọi tên injection ⇒ false; bảng `t`, dữ liệu và schema không đổi"
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
            match sqlite.table_exists(h, "t"):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.table_exists("db:00000000000000000000000000000000", "t"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.table_exists("khong-phai-handle", "t"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.table_exists("", "t"):
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
        ("kieu-1", "sqlite.table_exists(1, \"t\")", "VS0412"),
        ("kieu-1-bool", "sqlite.table_exists(true, \"t\")", "VS0412"),
        ("kieu-2", "sqlite.table_exists(\"h\", 1)", "VS0412"),
        ("kieu-2-bool", "sqlite.table_exists(\"h\", true)", "VS0412"),
        ("arity-0", "sqlite.table_exists()", "VS0401"),
        ("arity-1", "sqlite.table_exists(\"h\")", "VS0401"),
        (
            "arity-3",
            "sqlite.table_exists(\"h\", \"t\", \"x\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
