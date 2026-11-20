// SPDX-License-Identifier: MIT OR Apache-2.0
// Cấu hình core (`--no-default-features`) không có host `std.sqlite` ⇒ bỏ cả tệp test.
#![cfg(feature = "sqlite")]
//! **VD-270 (db-explain)** — `sqlite.explain(handle, sql, params) -> Result[RowSet, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Chỉ nhận **truy vấn chỉ đọc**: sau khoảng trắng và comment đầu (`--…`, `/*…*/`), từ khoá
//!   đầu phải là `SELECT` hoặc `WITH` (không phân biệt hoa thường). Mọi thứ khác ⇒ `Err(Error)`
//!   **dữ liệu** (không trap), **trước** khi chạm handle/SQLite.
//! - Chạy `EXPLAIN QUERY PLAN <sql>` qua **đúng** đường `sqlite.query` hiện có: cùng authorizer
//!   (dbstat/trang thô/`load_extension` vẫn bị chặn), chính sách chỉ đọc, trần SQL/hàng/ô/byte,
//!   progress-handler 5 s, cache schema/statement và bind params (cùng params với câu gốc).
//! - Trả `RowSet` của kế hoạch (`id, parent, notused, detail`), không trả dữ liệu câu gốc.
//! - Mọi ca chạy **cả hai** engine (interp + `--vm`) trên tệp SQLite tạm riêng từng engine, so
//!   byte stdout/stderr/exit.
//!
//! **RED-BEFORE (2026-11-21)**: trước khi cài, cả 3 ca đỏ với
//! `VS0301: module std.sqlite không có hàm explain` (checker chặn trước khi chạy); sau khi cài
//! cả 3 ca xanh.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-db-explain-{tag}-{}-{n}",
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

/// Chạy hai engine trên **hai tệp DB tạm riêng** (cùng nguồn, khác đường dẫn DB) và đòi
/// stdout/stderr/exit **byte-identical**; trả kết quả đường interp.
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
    src.replace("__DB__", db.to_str().expect("đường dẫn UTF-8"))
}

/// Plan của `SELECT` đơn giản, params và `WITH` đều trả `RowSet` 4 cột của
/// `EXPLAIN QUERY PLAN` (**không** trả dữ liệu câu gốc); thiếu/thừa params ⇒ `Err`.
#[test]
fn plan_select_tham_so_va_with() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", [])
            let _ = sqlite.execute(h, "CREATE INDEX idx_t_a ON t(a)", [])
            match sqlite.explain(h, "SELECT b FROM t WHERE a = 1", []):
                Err(e): print("plan loi {e.message}")
                Ok(rs):
                    print("cot={rs.columns.len()} ten0={rs.columns[0]} ten3={rs.columns[3]}")
                    print("hang={rs.rows.len()} duong={rs.rows[0][3]}")
            match sqlite.explain(h, "SELECT b FROM t WHERE a = ?", [SqlValue.Integer(1)]):
                Ok(rs): print("param ok hang={rs.rows.len()} duong={rs.rows[0][3]}")
                Err(e): print("param loi {e.message}")
            match sqlite.explain(h, "WITH c AS (SELECT 1 AS x) SELECT x FROM c", []):
                Ok(rs): print("with ok hang={rs.rows.len()}")
                Err(e): print("with loi {e.message}")
            match sqlite.explain(h, "SELECT b FROM t WHERE a = ?", []):
                Ok(_): print("thieu param khong duoc phep")
                Err(e): print("thieu param bi chan")
            match sqlite.explain(h, "SELECT 1", [SqlValue.Integer(1)]):
                Ok(_): print("thua param khong duoc phep")
                Err(e): print("thua param bi chan")
            match sqlite.explain(h, "SELECT * FROM khong_co_bang", []):
                Ok(_): print("sql loi khong duoc phep")
                Err(e): print("sql loi bi chan")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("plan", |db| thay_db(SRC, db));
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "cot=4 ten0=id ten3=detail\nhang=1 duong=Text(SEARCH t USING INDEX idx_t_a (a=?))\nparam ok hang=1 duong=Text(SEARCH t USING INDEX idx_t_a (a=?))\nwith ok hang=3\nthieu param bi chan\nthua param bi chan\nsql loi bi chan\n",
        "plan phải có 4 cột id/parent/notused/detail; params/with dùng đúng đường query"
    );
}

/// Chỉ nhận `SELECT`/`WITH` sau khoảng trắng + comment đầu; `INSERT`/`UPDATE`/`DELETE`/DDL/
/// `PRAGMA`/`EXPLAIN`/`BEGIN`/rỗng/chỉ comment/multi-statement đều `Err` **trước** khi chạy —
/// không tạo bảng, không đổi dữ liệu.
#[test]
fn chi_select_hoac_with_va_comment_dau() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", [])
            let _ = sqlite.execute(h, "INSERT INTO t VALUES (1, 'mot')", [])
            let nhan = [
                "  SELECT 1",
                "\n\tselect a from t",
                "WITH c AS (SELECT 1 AS x) SELECT x FROM c",
                "with c(x) as (select 1) select x from c",
                "-- ghi chu dau\nSELECT a FROM t",
                "/* khoi\n   nhieu dong */\nSELECT a FROM t",
                "  -- mot\n  /* hai */  SELECT a FROM t"
            ]
            for s in nhan:
                match sqlite.explain(h, s, []):
                    Ok(rs): print("nhan hang={rs.rows.len() > 0}")
                    Err(e): print("nhan loi")
            let chan = [
                "",
                "   \n\t",
                "-- chi comment",
                "/* chi comment */",
                "/* chua dong SELECT 1",
                "INSERT INTO t VALUES (2, 'hai')",
                "UPDATE t SET b = 'x'",
                "DELETE FROM t",
                "CREATE TABLE moi (x INTEGER)",
                "DROP TABLE t",
                "ALTER TABLE t ADD COLUMN c INTEGER",
                "PRAGMA user_version",
                "EXPLAIN SELECT 1",
                "WITH x AS (SELECT 1) DELETE FROM t",
                "WITH x AS (SELECT 1) INSERT INTO t VALUES (2, 'hai')",
                "BEGIN",
                "ATTACH DATABASE ':memory:' AS x",
                "SELECT 1; SELECT 2"
            ]
            for s in chan:
                match sqlite.explain(h, s, []):
                    Ok(_): print("chan khong duoc phep")
                    Err(e): print("chan")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("count={rs.rows[0][0]}")
                Err(e): print("count loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE name = 'moi'", []):
                Ok(rs): print("bang moi={rs.rows[0][0]}")
                Err(e): print("bang moi loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("loc", |_| SRC.to_string());
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "nhan hang=true\nnhan hang=true\nnhan hang=true\nnhan hang=true\nnhan hang=true\nnhan hang=true\nnhan hang=true\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\nchan\ncount=Integer(1)\nbang moi=Integer(0)\n",
        "chỉ SELECT/WITH sau khoảng trắng/comment đầu; ghi/DDL phải bị chặn không side effect"
    );
}

/// Authorizer/chính sách và trần giữ nguyên: `dbstat`/`load_extension` bị chặn, SQL > 64 KiB bị
/// chặn, handle đóng/params sai kiểu/multi-statement ⇒ `Err` dữ liệu (không trap).
#[test]
fn authorizer_tran_va_handle_giu_nguyen() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            match sqlite.explain(h, "SELECT * FROM dbstat", []):
                Ok(_): print("dbstat khong duoc phep")
                Err(e): print("dbstat bi chan")
            match sqlite.explain(h, "SELECT load_extension('/tmp/khong-duoc.so')", []):
                Ok(_): print("load_extension khong duoc phep")
                Err(e): print("load_extension bi chan")
            let lon = "SELECT '" + "x".repeat(70000) + "'"
            match sqlite.explain(h, lon, []):
                Ok(_): print("sql lon khong duoc phep")
                Err(e): print("sql lon bi chan")
            match sqlite.explain(h, "SELECT 1; SELECT 2", []):
                Ok(_): print("multi khong duoc phep")
                Err(e): print("multi bi chan")
            let _ = sqlite.close(h)
            match sqlite.explain(h, "SELECT 1", []):
                Ok(_): print("handle dong khong duoc phep")
                Err(e): print("handle dong bi chan")
            match sqlite.explain("db:00000000000000000000000000000000", "SELECT 1", []):
                Ok(_): print("handle la khong duoc phep")
                Err(e): print("handle la bi chan")
"#;
    let (code, out, err) = hai_engine("authorizer", |_| SRC.to_string());
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "dbstat bi chan\nload_extension bi chan\nsql lon bi chan\nmulti bi chan\nhandle dong bi chan\nhandle la bi chan\n",
        "authorizer/trần/handle phải đi qua đúng đường query; mọi lỗi là Err dữ liệu"
    );
}
