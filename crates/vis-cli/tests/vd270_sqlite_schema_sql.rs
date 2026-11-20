// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-schema-sql)** — `sqlite.schema_sql(handle, name) -> Result[str?, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **văn bản SQL khai báo** của một đối tượng schema trong **schema chính** (`main`) —
//!   `table`/`view`/`index`/`trigger` — tra theo **tên** trong `sqlite_master`:
//!   `SELECT sql FROM sqlite_master WHERE name = ? LIMIT 1`. Không lọc `type`: mọi đối tượng
//!   schema dùng chung không gian tên, tên quyết định đối tượng.
//! - Tên là **tham số bind** (`SqlValue.Text`), không nối chuỗi SQL: tên kiểu `t' OR 1=1 --`/
//!   `t; DROP TABLE t` chỉ là tên (không có đối tượng như vậy ⇒ `Ok(None)`) và **không** tác
//!   dụng phụ. Hàm **chỉ đọc**, đi qua **đúng** đường `sqlite.query`/`db_query` hiện có (không
//!   thêm method host): authorizer/chính sách chỉ đọc/trần/timeout/cache áp nguyên.
//! - **Không có đối tượng** tên đó ⇒ `Ok(None)`. Hàng có `sql = NULL` cũng ⇒ `Ok(None)` — index
//!   tự động `sqlite_autoindex_%` (sinh bởi `UNIQUE`/`PRIMARY KEY`) không có văn bản `CREATE`.
//! - **Phân biệt hoa/thường**: `name = ?` dùng collation `BINARY` mặc định của `sqlite_master`,
//!   nên `CREATE TABLE Bang` tra `Bang` ⇒ `Some`, tra `bang`/`BANG` ⇒ `Ok(None)` (dù SQLite vẫn
//!   cho tham chiếu bảng tên `bang` trong câu SQL khác). Hợp đồng **có chủ ý**.
//! - Đối tượng nội bộ `sqlite_%` **không** bị đặc cách ngoài truy vấn: `sqlite_sequence` (sinh
//!   bởi `AUTOINCREMENT`) có `sql` thật nên trả đúng chuỗi SQLite lưu; `sqlite_master` không có
//!   hàng trong chính nó ⇒ `Ok(None)`. Bảng/đối tượng TEMP nằm ở schema `temp`
//!   (`sqlite_temp_master`) nên **không** thuộc schema chính ⇒ `Ok(None)`.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — binary của cây này @ `5dead58`):
//!
//! ```text
//! $ vis check probe.vis
//! TAP version 13
//! 1..1
//! not ok 1 - probe.vis
//! # VS0301 9:19 module `std.sqlite` không có hàm `schema_sql`
//! $ vis run --no-cache probe.vis       ⇒ exit 2
//! probe.vis:9:19: error VS0301: module `std.sqlite` không có hàm `schema_sql`
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
        "vis-vd270-sqlite-schema-sql-{tag}-{}-{n}",
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
/// đường interp. `sqlite.schema_sql` chỉ đọc nên không cần tệp DB riêng từng engine.
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

/// Đủ bốn loại đối tượng schema (`table`/`view`/`index`/`trigger`): trả **nguyên văn** câu
/// `CREATE` SQLite lưu (không chuẩn hoá thêm); autoindex `UNIQUE` có `sql = NULL` ⇒ `Ok(None)`.
#[test]
fn bang_view_index_trigger_va_autoindex_null() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE Bang (a TEXT UNIQUE, b INTEGER)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_bang AS SELECT a FROM Bang", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_b ON Bang(b)", [])
            let _ = sqlite.execute(h, "CREATE TRIGGER trg_bang AFTER INSERT ON Bang BEGIN UPDATE Bang SET b = b + 1 WHERE rowid = new.rowid; END", [])
            for ten in ["Bang", "v_bang", "idx_b", "trg_bang", "sqlite_autoindex_Bang_1"]:
                match sqlite.schema_sql(h, ten):
                    Ok(s):
                        match s:
                            Some(x): print("{ten}={x}")
                            None: print("{ten}=None")
                    Err(e): print("{ten} loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("bon-loai", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "Bang=CREATE TABLE Bang (a TEXT UNIQUE, b INTEGER)\n\
         v_bang=CREATE VIEW v_bang AS SELECT a FROM Bang\n\
         idx_b=CREATE INDEX idx_b ON Bang(b)\n\
         trg_bang=CREATE TRIGGER trg_bang AFTER INSERT ON Bang BEGIN UPDATE Bang SET b = b + 1 WHERE rowid = new.rowid; END\n\
         sqlite_autoindex_Bang_1=None\n",
        "bốn loại đối tượng trả đúng câu CREATE; autoindex (sql NULL) ⇒ Ok(None)"
    );
}

/// Thiếu tên ⇒ `Ok(None)`; **phân biệt hoa/thường** (`Bang` ≠ `bang`/`BANG`); `sqlite_%` không
/// đặc cách (`sqlite_sequence` có SQL thật, `sqlite_master` không có hàng); TEMP không thuộc
/// schema chính.
#[test]
fn thieu_ten_case_va_noi_bo_khong_dac_cach() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE Bang (a TEXT)", [])
            let _ = sqlite.execute(h, "CREATE TABLE seq (x INTEGER PRIMARY KEY AUTOINCREMENT)", [])
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tam (q INTEGER)", [])
            for ten in ["khong_co", "bang", "BANG", "Bang", "sqlite_sequence", "sqlite_master", "tam"]:
                match sqlite.schema_sql(h, ten):
                    Ok(s):
                        match s:
                            Some(x): print("{ten}={x}")
                            None: print("{ten}=None")
                    Err(e): print("{ten} loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("case-noi-bo", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "khong_co=None\n\
         bang=None\n\
         BANG=None\n\
         Bang=CREATE TABLE Bang (a TEXT)\n\
         sqlite_sequence=CREATE TABLE sqlite_sequence(name,seq)\n\
         sqlite_master=None\n\
         tam=None\n",
        "thiếu/case khác ⇒ Ok(None); sqlite_sequence trả SQL thật; sqlite_master/TEMP ⇒ Ok(None)"
    );
}

/// Tên **trông như SQL injection** được bind làm tham số: không khớp đối tượng nào ⇒ `Ok(None)`,
/// và bảng thật + schema **không** đổi (không tác dụng phụ).
#[test]
fn ten_injection_chi_la_ten() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_t ON t(a)", [])
            for ten in ["t' OR 1=1 --", "t; DROP TABLE t", "t\" OR \"1\"=\"1", "t' UNION SELECT sql FROM sqlite_master --", "t'; DROP INDEX idx_t; --"]:
                match sqlite.schema_sql(h, ten):
                    Ok(s):
                        match s:
                            Some(_): print("la=Some")
                            None: print("la=None")
                    Err(e): print("la loi {e.message}")
            match sqlite.schema_sql(h, "t"):
                Ok(s):
                    match s:
                        Some(x): print("that={x}")
                        None: print("that=None")
                Err(e): print("that loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master", []):
                Ok(rs): print("so-do={rs.rows[0][0]}")
                Err(e): print("so-do loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("injection", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "la=None\nla=None\nla=None\nla=None\nla=None\n\
         that=CREATE TABLE t (a TEXT)\nso-do=Integer(2)\n",
        "tên injection chỉ là tham số: Ok(None); bảng thật + schema không đổi"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0 —
/// không trap, không panic; khác với đối tượng thiếu ⇒ `Ok(None)`.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            let _ = sqlite.close(h)
            match sqlite.schema_sql(h, "t"):
                Ok(_): print("dong khong duoc phep")
                Err(e): print("dong bi chan")
            match sqlite.schema_sql("db:00000000000000000000000000000000", "t"):
                Ok(_): print("la khong duoc phep")
                Err(e): print("la bi chan")
            match sqlite.schema_sql("khong-phai-handle", "t"):
                Ok(_): print("sai dang khong duoc phep")
                Err(e): print("sai dang bi chan")
            match sqlite.schema_sql("", "t"):
                Ok(_): print("rong khong duoc phep")
                Err(e): print("rong bi chan")
            match sqlite.schema_sql("db:00000000000000000000000000000000", "khong_co"):
                Ok(_): print("la+thieu khong duoc phep")
                Err(e): print("la+thieu bi chan")
        Err(e): print("mo loi {e.message}")
"#;
    let (code, out, err) = hai_engine("handle", SRC);
    assert_eq!(code, 0, "lỗi handle phải là Err dữ liệu; stderr: {err}");
    assert_eq!(
        out, "dong bi chan\nla bi chan\nsai dang bi chan\nrong bi chan\nla+thieu bi chan\n",
        "mọi handle hỏng ⇒ Err(Error), không trap (kể cả khi tên cũng thiếu)"
    );
}

/// Sai kiểu/sai số đối số bị checker chặn **trước khi chạy** (`VS0412`/`VS0401`, exit 2 ở cả
/// hai engine).
#[test]
fn sai_kieu_sai_arity_bi_chan() {
    for (tag, call, ma) in [
        ("kieu-1", "sqlite.schema_sql(1, \"t\")", "VS0412"),
        ("kieu-2", "sqlite.schema_sql(\"h\", 1)", "VS0412"),
        ("kieu-2-bool", "sqlite.schema_sql(\"h\", true)", "VS0412"),
        ("arity-0", "sqlite.schema_sql()", "VS0401"),
        ("arity-1", "sqlite.schema_sql(\"h\")", "VS0401"),
        (
            "arity-3",
            "sqlite.schema_sql(\"h\", \"t\", \"x\")",
            "VS0401",
        ),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
