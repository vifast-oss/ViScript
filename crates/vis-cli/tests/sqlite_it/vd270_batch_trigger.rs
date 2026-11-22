// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (batch-trigger)** — `sqlite.execute_batch` hiểu thân `BEGIN … END` của trigger.
//!
//! Hợp đồng khoá ở đây:
//! - `CREATE [TEMP|TEMPORARY] TRIGGER … BEGIN <câu>; … END;` là **một** câu batch; dấu `;`
//!   trong thân không tách câu, kể cả `CASE … END` lồng và chuỗi/comment chứa `;`/`END`.
//! - Câu sau trigger vẫn được tách và chạy; số câu trả về đếm trigger là **một**.
//! - Trigger thiếu `END` giữ nguyên tới hết input để SQLite báo lỗi (không cắt im lặng).
//! - Mọi ca chạy **cả hai** engine (interp + `--vm`) trên tệp SQLite tạm riêng từng engine,
//!   so byte stdout/stderr/exit.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-batch-trigger-{tag}-{}-{n}",
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

fn hai_engine<F>(tag: &str, nguon: F) -> (i32, String, String)
where
    F: Fn(&Path) -> String,
{
    let d = tam(tag);
    let db_i = d.join("interp.db");
    let db_v = d.join("vm.db");
    let p_i = ghi(&d, "interp", &nguon(&db_i));
    let p_v = ghi(&d, "vm", &nguon(&db_v));
    let a = chay(&p_i, false);
    let b = chay(&p_v, true);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(a, b, "{tag}: interp ≠ `--vm`\n  interp: {a:?}\n  vm: {b:?}");
    a
}

fn thay_db(src: &str, db: &Path) -> String {
    // Windows: `\` trong đường dẫn phải escape khi nhúng vào chuỗi ViScript (`\\`), nếu không
    // bị `VS0102: escape không hợp lệ`.
    let duong = db.to_str().expect("đường dẫn UTF-8").replace('\\', "\\\\");
    src.replace("__DB__", &duong)
}

/// Trigger nhiều câu trong thân: `;` không tách; `CASE … END` và chuỗi `';'`/`'END;'` vẫn nguyên;
/// câu `INSERT` sau trigger chạy như câu riêng; batch trả đúng số câu (tính trigger là 1).
#[test]
fn trigger_nhieu_cau_case_va_cau_sau() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let sql = r"""
CREATE TABLE log (msg TEXT);
CREATE TABLE t (id INTEGER PRIMARY KEY, a INTEGER);
CREATE TRIGGER tr AFTER INSERT ON t
BEGIN
    INSERT INTO log (msg) VALUES ('mot; hai');
    INSERT INTO log (msg) VALUES (CASE WHEN NEW.a = 1 THEN 'END;' ELSE 'x' END);
END;
INSERT INTO t (a) VALUES (1);
"""
            match sqlite.execute_batch(h, sql):
                Ok(n): print("batch n={n}")
                Err(e): print("batch loi {e.message}")
            match sqlite.query(h, "SELECT msg FROM log ORDER BY rowid", []):
                Ok(rs): print("log={rs.rows.len()} first={rs.rows[0][0]}")
                Err(e): print("log loi {e.message}")
"#;
    let (ma, out, err) = hai_engine("nhieu", |db| thay_db(SRC, db));
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out, "batch n=4\nlog=2 first=Text(mot; hai)\n",
        "stderr: {err}"
    );
}

/// `CREATE TEMP TRIGGER` + định danh chứa `;` + comment trong thân; câu `CREATE VIEW` sau vẫn
/// chạy. Trigger thật sự bắn sau `INSERT` (log temp có 2 hàng).
#[test]
fn trigger_temp_ten_va_comment() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let sql = r"""
CREATE TABLE t (id INTEGER PRIMARY KEY, a INTEGER);
CREATE TEMP TABLE log (msg TEXT);
CREATE TEMP TRIGGER "we;ird" AFTER INSERT ON t
BEGIN
    -- comment ; END gia
    INSERT INTO log (msg) VALUES ('x;y');
    /* block ; END */
    INSERT INTO log (msg) VALUES ('END');
END;
CREATE VIEW v AS SELECT 1 AS x;
INSERT INTO t (a) VALUES (1);
"""
            match sqlite.execute_batch(h, sql):
                Ok(n): print("batch n={n}")
                Err(e): print("batch loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM log", []):
                Ok(rs): print("log={rs.rows[0][0]}")
                Err(e): print("log loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE type = 'view'", []):
                Ok(rs): print("view={rs.rows[0][0]}")
                Err(e): print("view loi {e.message}")
"#;
    let (ma, out, err) = hai_engine("temp", |db| thay_db(SRC, db));
    assert_eq!(ma, 0, "stderr: {err}");
    assert_eq!(
        out, "batch n=5\nlog=Integer(2)\nview=Integer(1)\n",
        "stderr: {err}"
    );
}

/// Trigger thiếu `END` giữ tới hết input ⇒ SQLite báo lỗi qua `Err` dữ liệu, không cắt thành
/// nhiều câu "hợp lệ" giả.
#[test]
fn trigger_thieu_end_tra_err() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER)", [])
            let sql = "CREATE TRIGGER tr AFTER INSERT ON t BEGIN INSERT INTO t VALUES (1);"
            match sqlite.execute_batch(h, sql):
                Ok(n): print("batch n={n}")
                Err(e): print("batch loi={e.message}")
"#;
    let (ma, out, err) = hai_engine("thieu", |db| thay_db(SRC, db));
    assert_eq!(ma, 0, "stderr: {err}");
    assert!(
        out.starts_with("batch loi="),
        "stdout: {out}\nstderr: {err}"
    );
    assert!(!out.starts_with("batch n="), "stdout: {out}\nstderr: {err}");
}
