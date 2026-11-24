// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (sqlite-data-version)** — `sqlite.data_version(handle) -> Result[int, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Trả **`data_version`** — bộ đếm **theo từng connection** của SQLite — đọc bằng
//!   `PRAGMA data_version` qua **đúng** đường `sqlite.query`/`DbHost::db_query` hiện có (không
//!   thêm method host, **không** ghi/DDL): authorizer allow-list **đọc** `user_version`/
//!   `schema_version`/`data_version` (không kèm giá trị), chính sách chỉ đọc
//!   (`sqlite3_stmt_readonly`), trần/timeout/cache áp nguyên.
//! - Giá trị **tăng** khi **connection khác** commit thay đổi vào **cùng** database — cả ghi dữ
//!   liệu lẫn DDL; **không** đổi vì ghi/DDL/commit transaction của **chính** connection. Thay đổi
//!   **chưa commit** (hoặc bị `ROLLBACK`) của connection khác **không** đổi giá trị.
//! - `:memory:`: mỗi connection có database **riêng**, nên connection thứ hai ghi vào database của
//!   nó **không** làm đổi giá trị của connection thứ nhất (muốn quan sát phải mở **cùng tệp** DB).
//! - **Giá trị khởi tạo** của connection mới (kể cả `:memory:`) là `1` — probe bên dưới. Đây là
//!   chi tiết hiện thực, **không** phải hợp đồng: std chỉ hứa `int` **không âm** `0..=i32::MAX`.
//! - **Đường đọc đã chọn**: `PRAGMA data_version` (không phải `SELECT * FROM pragma_data_version`).
//!   **Probe** trên host hiện tại (cả hai engine): `PRAGMA data_version` ⇒ `Ok([[Integer(1)]])`;
//!   `SELECT * FROM pragma_data_version` cũng `Ok` 1 hàng, nhưng sau khi tạo bảng người dùng
//!   `pragma_data_version` thì TVF trả `0` hàng (bị **che**) còn pragma vẫn `Integer(1)`. Vì vậy
//!   chọn pragma — không phụ thuộc schema người dùng. Câu SQL là **hằng**, không bind tham số —
//!   không có bề mặt injection.
//! - Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(Error)` **dữ liệu**, không trap.
//!   Kết quả sai hình dạng/ngoài `0..=i32::MAX` cũng là `Err` dữ liệu — kiểm ở unit test host giả
//!   của `crates/vis-runtime/src/stdlib/sqlite/pragma.rs` (không quan sát được qua tệp DB vì
//!   `data_version` là bộ đếm runtime, không nằm trong header).
//!
//! Mọi ca dương chạy **cả hai** engine (interp + `--vm --no-cache`), so **từng byte**
//! stdout/stderr/exit; ca dùng tệp DB thì mỗi engine một tệp riêng.
//!
//! **RED-BEFORE** (2026-11-21, trước khi thêm hàm — cây `main` @ `7b52f89`):
//!
//! ```text
//! $ cargo test -p vis-cli --test vd270_sqlite_data_version
//! running 6 tests
//! test sai_kieu_sai_arity_bi_chan ... FAILED
//! test handle_dong_va_sai_dang_la_err_du_lieu ... FAILED
//! test tvf_trung_ten_khong_che_pragma ... FAILED
//! test memory_hai_connection_doc_lap ... FAILED
//! test khoi_tao_1_va_ghi_ddl_cua_minh_khong_doi ... FAILED
//! test connection_thu_hai_commit_doi_gia_tri ... FAILED
//! test result: FAILED. 0 passed; 6 failed
//! # mọi ca đỏ vì `VS0301` «module `std.sqlite` không có hàm `data_version`»
//! # (ca sai kiểu/arity cũng đỏ VS0301 vì hàm chưa tồn tại, chưa tới VS0412/VS0401)
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
        "vis-vd270-sqlite-data-version-{tag}-{}-{n}",
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

/// Chạy một nguồn (ghi vào thư mục tạm riêng) trên một engine.
fn chay_nguon(tag: &str, src: &str, vm: bool) -> (i32, String, String) {
    let d = tam(tag);
    let p = ghi(&d, tag, src);
    let r = chay(&p, vm);
    let _ = std::fs::remove_dir_all(&d);
    r
}

/// Chạy hai engine trên cùng nguồn và đòi stdout/stderr/exit **byte-identical**; trả kết quả
/// đường interp. `sqlite.data_version` chỉ đọc nên không cần tệp DB riêng từng engine khi nguồn
/// dùng `:memory:`.
fn hai_engine(tag: &str, src: &str) -> (i32, String, String) {
    let a = chay_nguon(tag, src, false);
    let b = chay_nguon(tag, src, true);
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

/// `:memory:` mới ⇒ `1` (probe); ghi dữ liệu, DDL (bảng/index/view/update/delete/drop) và
/// transaction **của chính connection** **không** đổi `data_version`; truy vấn chỉ đọc vẫn chạy.
#[test]
fn khoi_tao_1_va_ghi_ddl_cua_minh_khong_doi() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (a TEXT)", [])
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "INSERT INTO t VALUES ('x')", [])
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master", []):
                Ok(_): print("doc-ok")
                Err(e): print("doc loi {e.message}")
            let _ = sqlite.execute(h, "CREATE INDEX i_t ON t (a)", [])
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "UPDATE t SET a = 'y'", [])
            let _ = sqlite.execute(h, "DELETE FROM t", [])
            let _ = sqlite.execute(h, "DROP VIEW v_t", [])
            let _ = sqlite.execute(h, "DROP TABLE t", [])
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "BEGIN", [])
            let _ = sqlite.execute(h, "CREATE TABLE t2 (b INTEGER)", [])
            let _ = sqlite.execute(h, "COMMIT", [])
            match sqlite.data_version(h):
                Ok(v): print(v)
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("ddl", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        dong,
        ["1", "1", "1", "1", "doc-ok", "1", "1", "1", "1"],
        "`:memory:` mới ⇒ 1; ghi/DDL/transaction của **chính** connection **không** đổi \
         data_version: {dong:?}\nstderr: {err}"
    );
}

/// Bảng người dùng trùng tên TVF `pragma_data_version` **che** TVF nhưng **không** ảnh hưởng
/// đường đọc `PRAGMA data_version`.
#[test]
fn tvf_trung_ten_khong_che_pragma() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.query(h, "SELECT * FROM pragma_data_version", []):
                Ok(rs): print("tvf={rs.rows.len()}")
                Err(e): print("tvf loi {e.message}")
            match sqlite.data_version(h):
                Ok(v): print("truoc={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE pragma_data_version (x INTEGER)", [])
            match sqlite.query(h, "SELECT * FROM pragma_data_version", []):
                Ok(rs): print("tvf-che={rs.rows.len()}")
                Err(e): print("tvf che loi {e.message}")
            match sqlite.data_version(h):
                Ok(v): print("sau={v}")
                Err(e): print("loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("tvf", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "tvf=1\ntruoc=1\ntvf-che=0\nsau=1\n",
        "TVF bị bảng trùng tên che; pragma vẫn đọc đúng 1"
    );
}

/// `:memory:`: mỗi connection một database **riêng** ⇒ connection thứ hai ghi/DDL **không** đổi
/// giá trị của connection thứ nhất (và ghi của chính nó cũng không đổi giá trị của nó).
#[test]
fn memory_hai_connection_doc_lap() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo h1 loi {e.message}")
        Ok(h1):
            match sqlite.open(":memory:"):
                Err(e): print("mo h2 loi {e.message}")
                Ok(h2):
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-truoc={v}")
                        Err(e): print("loi {e.message}")
                    match sqlite.data_version(h2):
                        Ok(v): print("h2-truoc={v}")
                        Err(e): print("loi {e.message}")
                    let _ = sqlite.execute(h2, "CREATE TABLE t (a TEXT)", [])
                    let _ = sqlite.execute(h2, "INSERT INTO t VALUES ('x')", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-sau-h2={v}")
                        Err(e): print("loi {e.message}")
                    match sqlite.data_version(h2):
                        Ok(v): print("h2-sau-ghi={v}")
                        Err(e): print("loi {e.message}")
                    let _ = sqlite.close(h2)
            let _ = sqlite.close(h1)
"#;
    let (code, out, err) = hai_engine("memory", SRC);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "h1-truoc=1\nh2-truoc=1\nh1-sau-h2=1\nh2-sau-ghi=1\n",
        "hai connection `:memory:` có database riêng ⇒ không quan sát được thay đổi của nhau"
    );
}

/// **Cùng một tệp** DB, hai connection: DDL/ghi của **h2** làm **tăng** `data_version` của
/// **h1** (2, 3, 4); thay đổi **chưa commit**/`ROLLBACK` của h2 **không** đổi; ghi của **chính**
/// h1 **không** đổi; h2 (connection thực hiện) luôn `1`; mở lại connection mới ⇒ `1`.
#[test]
fn connection_thu_hai_commit_doi_gia_tri() {
    fn mot_luot(nhan: &str, vm: bool) -> (i32, String, String) {
        let d = tam(&format!("tep-{nhan}"));
        let db = d.join("d.db");
        // Dùng `/` trong literal VS (SQLite nhận cả `\` trên Windows); đường dẫn tệp tạm không
        // chứa dấu `"` nên không cần escape thêm.
        let db = db.display().to_string().replace('\\', "/");
        let src = format!(
            r#"use std.sqlite

fn main():
    let db = "{db}"
    match sqlite.open(db):
        Err(e): print("mo h1 loi {{e.message}}")
        Ok(h1):
            match sqlite.data_version(h1):
                Ok(v): print("h1={{v}}")
                Err(e): print("loi {{e.message}}")
            match sqlite.open(db):
                Err(e): print("mo h2 loi {{e.message}}")
                Ok(h2):
                    match sqlite.data_version(h2):
                        Ok(v): print("h2={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.execute(h2, "CREATE TABLE t (a TEXT)", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-sau-ddl-h2={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.execute(h2, "INSERT INTO t VALUES ('b')", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-sau-insert-h2={{v}}")
                        Err(e): print("loi {{e.message}}")
                    match sqlite.data_version(h2):
                        Ok(v): print("h2-sau-ghi-minh={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.execute(h1, "INSERT INTO t VALUES ('a')", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-sau-ghi-minh={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.execute(h2, "BEGIN", [])
                    let _ = sqlite.execute(h2, "INSERT INTO t VALUES ('c')", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-khi-h2-chua-commit={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.execute(h2, "ROLLBACK", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-sau-rollback={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.execute(h2, "BEGIN", [])
                    let _ = sqlite.execute(h2, "INSERT INTO t VALUES ('d')", [])
                    let _ = sqlite.execute(h2, "COMMIT", [])
                    match sqlite.data_version(h1):
                        Ok(v): print("h1-sau-commit-h2={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.close(h2)
            match sqlite.data_version(h1):
                Ok(v): print("h1-sau-dong-h2={{v}}")
                Err(e): print("loi {{e.message}}")
            let _ = sqlite.close(h1)
            match sqlite.open(db):
                Err(e): print("mo h3 loi {{e.message}}")
                Ok(h3):
                    match sqlite.data_version(h3):
                        Ok(v): print("h3={{v}}")
                        Err(e): print("loi {{e.message}}")
                    let _ = sqlite.close(h3)
"#
        );
        let r = chay_nguon(&format!("tep-{nhan}"), &src, vm);
        let _ = std::fs::remove_dir_all(&d);
        r
    }

    let a = mot_luot("interp", false);
    let b = mot_luot("vm", true);
    assert_eq!(
        a, b,
        "interp ≠ `--vm` trên tệp DB thật\n  interp: {a:?}\n  vm: {b:?}"
    );
    let (code, out, err) = a;
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "h1=1\nh2=1\nh1-sau-ddl-h2=2\nh1-sau-insert-h2=3\nh2-sau-ghi-minh=1\n\
         h1-sau-ghi-minh=3\nh1-khi-h2-chua-commit=3\nh1-sau-rollback=3\n\
         h1-sau-commit-h2=4\nh1-sau-dong-h2=4\nh3=1\n",
        "connection thứ hai commit (DDL/ghi) phải làm tăng data_version của h1; \
         chưa commit/rollback/ghi của chính mình thì không; connection mới ⇒ 1\nstderr: {err}"
    );
}

/// Handle đã đóng/sai dạng/không tồn tại/rỗng ⇒ `Err(Error)` **dữ liệu** ở cả hai engine, exit 0
/// — không trap, không panic.
#[test]
fn handle_dong_va_sai_dang_la_err_du_lieu() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.close(h)
            match sqlite.data_version(h):
                Ok(_): print("dong khong duoc phep")
                Err(_): print("dong bi chan")
            match sqlite.data_version("db:00000000000000000000000000000000"):
                Ok(_): print("la khong duoc phep")
                Err(_): print("la bi chan")
            match sqlite.data_version("khong-phai-handle"):
                Ok(_): print("sai dang khong duoc phep")
                Err(_): print("sai dang bi chan")
            match sqlite.data_version(""):
                Ok(_): print("rong khong duoc phep")
                Err(_): print("rong bi chan")
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
        ("kieu-int", "sqlite.data_version(1)", "VS0412"),
        ("kieu-bool", "sqlite.data_version(true)", "VS0412"),
        ("arity-0", "sqlite.data_version()", "VS0401"),
        ("arity-2", "sqlite.data_version(\"h\", \"x\")", "VS0401"),
    ] {
        let src = format!("use std.sqlite\n\nfn main():\n    let x = {call}\n    print(x)\n");
        tu_choi(tag, &src, ma);
    }
}
