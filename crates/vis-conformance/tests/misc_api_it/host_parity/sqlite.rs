// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315d (VD-291 §3.4)** — parity `interp ↔ VM` cho **28 hàm `sqlite`** (DML/giao dịch +
//! metadata/introspection trên tệp DB thật và `:memory:`).
//!
//! **ST-316d (VD-292 §4.1)** — tách từ `misc_api_it/host_parity.rs` (cận trần R1); nội dung và
//! assertion giữ nguyên 100 %.

use std::path::Path;

use super::{duong, hai_duong, hai_duong_voi, tam};
// ===== sqlite: 28 hàm (VD-291 §3.4) =====

/// Dựng lại thư mục DB rỗng **trước mỗi đường** — nếu không, đường interp để lại bảng/dữ liệu
/// và đường VM sẽ chạy trên trạng thái khác.
fn dat_sqlite(work: &Path) {
    let _ = std::fs::remove_dir_all(work);
    std::fs::create_dir_all(work).expect("mkdir sqlite");
}

/// **ST-315d** — 13 hàm `sqlite` DML/truy vấn/giao dịch trên **tệp DB thật**, reset trước mỗi
/// engine: `open_with`, `close`, `execute`, `execute_batch`, `query`, `query_one`, `explain`,
/// `last_insert_rowid`, `changes`, `begin`, `commit`, `rollback`, `in_transaction`.
/// Mọi lỗi SQL/params/handle là `Result.Err` **dữ liệu** (exit 0), in cả thông điệp để so parity.
pub(super) const SRC_SQLITE_DML: &str = r#"use std.sqlite

fn hien(v: SqlValue) -> str:
    match v:
        Null: "null"
        Integer(n): "int({n})"
        Real(f): "real({f})"
        Text(s): "text({s})"
        Blob(b): "blob({len(b)})"

fn dem(h: str) -> str:
    match sqlite.query(h, "SELECT count(*) FROM t", []):
        Ok(rs): "dem={rs.rows[0][0]}"
        Err(e): "dem loi {e.message}"

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            print("tx0={sqlite.in_transaction(h)}")
            match sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", []):
                Ok(n): print("create n={n}")
                Err(e): print("create loi {e.message}")
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(1), SqlValue.Text("mot")]):
                Ok(n): print("insert n={n}")
                Err(e): print("insert loi {e.message}")
            print("rowid={sqlite.last_insert_rowid(h)}")
            print("changes={sqlite.changes(h)}")
            match sqlite.query(h, "SELECT a, b FROM t", []):
                Ok(rs):
                    print("cols={rs.columns}")
                    print("rows={rs.rows.len()}")
                    print("cell={hien(rs.rows[0][0])} {hien(rs.rows[0][1])}")
                Err(e): print("query loi {e.message}")
            match sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(9)]):
                Ok(rs): print("q0={rs.rows.len()}")
                Err(e): print("q0 loi {e.message}")
            match sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(1)]):
                Ok(rs): print("q1={rs.rows.len()} {hien(rs.rows[0][0])}")
                Err(e): print("q1 loi {e.message}")
            match sqlite.explain(h, "SELECT a FROM t WHERE a = 1", []):
                Ok(rs): print("explain cols={rs.columns} co-plan={rs.rows.len() > 0}")
                Err(e): print("explain loi {e.message}")
            match sqlite.execute_batch(h, "INSERT INTO t (a, b) VALUES (2, 'hai'); INSERT INTO t (a, b) VALUES (3, 'ba');"):
                Ok(n): print("batch n={n}")
                Err(e): print("batch loi {e.message}")
            match sqlite.begin(h):
                Ok(_): print("begin1 ok")
                Err(e): print("begin1 loi {e.message}")
            print("tx1={sqlite.in_transaction(h)}")
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (4, 'bon')", []):
                Ok(n): print("ins-tx n={n}")
                Err(e): print("ins-tx loi {e.message}")
            print("tx1b={sqlite.in_transaction(h)}")
            match sqlite.rollback(h):
                Ok(_): print("rollback ok")
                Err(e): print("rollback loi {e.message}")
            print("tx2={sqlite.in_transaction(h)} {dem(h)}")
            let _ = sqlite.begin(h)
            let _ = sqlite.execute(h, "INSERT INTO t (a, b) VALUES (5, 'nam')", [])
            match sqlite.commit(h):
                Ok(_): print("commit ok")
                Err(e): print("commit loi {e.message}")
            print("tx3={sqlite.in_transaction(h)} {dem(h)}")
            let _ = sqlite.begin(h)
            match sqlite.begin(h):
                Ok(_): print("begin-long: KHONG DUOC PHEP")
                Err(e): print("begin-long bi chan")
            let _ = sqlite.rollback(h)
            match sqlite.commit(h):
                Ok(_): print("commit-ngoai: KHONG DUOC PHEP")
                Err(e): print("commit-ngoai bi chan")
            match sqlite.execute(h, "SELECT * FROM khong_co_bang", []):
                Ok(_): print("sql-sai: KHONG DUOC PHEP")
                Err(e): print("sql-sai {e.message}")
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(7)]):
                Ok(_): print("thieu-param: KHONG DUOC PHEP")
                Err(e): print("thieu-param {e.message}")
            match sqlite.query_one(h, "SELECT 1", [SqlValue.Integer(7)]):
                Ok(_): print("thua-param: KHONG DUOC PHEP")
                Err(e): print("thua-param {e.message}")
            match sqlite.execute(h, "DELETE FROM t", []):
                Ok(n): print("delete n={n}")
                Err(e): print("delete loi {e.message}")
            print("changes-cuoi={sqlite.changes(h)}")
            match sqlite.close(h):
                Ok(_): print("close ok")
                Err(e): print("close loi {e.message}")
            match sqlite.execute(h, "INSERT INTO t (a) VALUES (9)", []):
                Ok(_): print("sau-close: KHONG DUOC PHEP")
                Err(e): print("sau-close {e.message}")
    match sqlite.open_with("__MISS__", false, 5000):
        Ok(_): print("mo-thieu-tep: KHONG DUOC PHEP")
        Err(e): print("mo-thieu-tep bi chan")
"#;

#[test]
fn sqlite_dml_va_giao_dich_parity() {
    let d = tam("sqlite-dml");
    let work = d.join("db");
    let db = work.join("p.db");
    let src = SRC_SQLITE_DML
        .replace("__DB__", &duong(&db))
        .replace("__MISS__", &duong(&work.join("khong-co.db")));
    let (out, err) = hai_duong_voi(&d, "sqlite-dml", &src, || dat_sqlite(&work));
    assert_eq!(err, "", "sqlite DML không được có stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        &dong[..],
        &[
            "tx0=Ok(false)",
            "create n=0",
            "insert n=1",
            "rowid=Ok(1)",
            "changes=Ok(1)",
            "cols=[a, b]",
            "rows=1",
            "cell=int(1) text(mot)",
            "q0=0",
            "q1=1 int(1)",
            "explain cols=[id, parent, notused, detail] co-plan=true",
            "batch n=2",
            "begin1 ok",
            "tx1=Ok(true)",
            "ins-tx n=1",
            "tx1b=Ok(true)",
            "rollback ok",
            "tx2=Ok(false) dem=Integer(3)",
            "commit ok",
            "tx3=Ok(false) dem=Integer(4)",
            "begin-long bi chan",
            "commit-ngoai bi chan",
            "sql-sai lỗi SQL (execute) (mã SQLite 1 — SqlError)",
            "thieu-param SQL cần 2 tham số, được 1",
            "thua-param SQL cần 0 tham số, được 1",
            "delete n=4",
            "changes-cuoi=Ok(4)",
            "close ok",
            "sau-close handle đã đóng",
            "mo-thieu-tep bi chan",
        ],
        "sqlite DML/tx: {out}\nstderr: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// **ST-315d** — 15 hàm `sqlite` metadata/introspection trên `:memory:` (DDL tất định):
/// `open`, `application_id`, `data_version`, `schema_version`, `user_version`, `journal_mode`,
/// `tables`, `table_exists`, `columns`, `indexes`, `index_exists`, `index_columns`,
/// `primary_keys`, `schema_sql`, `database_list`; ca lỗi: mọi hàm sau `close` ⇒ `Err` dữ liệu.
pub(super) const SRC_SQLITE_META: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            print("appid={sqlite.application_id(h)}")
            print("userv={sqlite.user_version(h)}")
            let sv0 = match sqlite.schema_version(h):
                Ok(v): v
                Err(e): -1
            let dv0 = match sqlite.data_version(h):
                Ok(v): v
                Err(e): -1
            print("sv0-khong-am={sv0 >= 0}")
            print("datav-duong={dv0 > 0}")
            match sqlite.journal_mode(h):
                Ok(v): print("journal={v}")
                Err(e): print("journal loi {e.message}")
            match sqlite.tables(h):
                Ok(xs): print("tables0={xs}")
                Err(e): print("tables0 loi {e.message}")
            match sqlite.database_list(h):
                Ok(ds):
                    print("dbs={ds.len()}")
                    for rs in ds:
                        print("db={rs.rows[0]}")
                Err(e): print("dbs loi {e.message}")
            let _ = sqlite.execute(h, "CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT UNIQUE, b REAL)", [])
            let _ = sqlite.execute(h, "CREATE INDEX i_ab ON t(b, a)", [])
            let _ = sqlite.execute(h, "CREATE VIEW v_t AS SELECT a FROM t", [])
            match sqlite.tables(h):
                Ok(xs): print("tables={xs}")
                Err(e): print("tables loi {e.message}")
            print("table-t={sqlite.table_exists(h, "t")}")
            print("table-v={sqlite.table_exists(h, "v_t")}")
            print("table-x={sqlite.table_exists(h, "khong_co")}")
            match sqlite.columns(h, "t"):
                Ok(xs): print("cols={xs}")
                Err(e): print("cols loi {e.message}")
            match sqlite.indexes(h, "t"):
                Ok(xs): print("idx={xs}")
                Err(e): print("idx loi {e.message}")
            print("idx-ab={sqlite.index_exists(h, "i_ab")}")
            print("idx-auto={sqlite.index_exists(h, "sqlite_autoindex_t_1")}")
            print("idx-x={sqlite.index_exists(h, "khong_co")}")
            match sqlite.index_columns(h, "i_ab"):
                Ok(xs): print("idxcols={xs}")
                Err(e): print("idxcols loi {e.message}")
            match sqlite.primary_keys(h, "t"):
                Ok(xs): print("pk={xs}")
                Err(e): print("pk loi {e.message}")
            for ten in ["t", "v_t", "i_ab", "sqlite_autoindex_t_1", "khong_co"]:
                match sqlite.schema_sql(h, ten):
                    Ok(s):
                        match s:
                            Some(x): print("sql {ten}={x}")
                            None: print("sql {ten}=None")
                    Err(e): print("sql {ten} loi {e.message}")
            let sv1 = match sqlite.schema_version(h):
                Ok(v): v
                Err(e): -1
            let dv1 = match sqlite.data_version(h):
                Ok(v): v
                Err(e): -1
            print("sv-tang={sv1 > sv0}")
            print("datav-khong-doi={dv1 == dv0}")
            match sqlite.close(h):
                Ok(_): print("close ok")
                Err(e): print("close loi {e.message}")
            match sqlite.application_id(h):
                Ok(_): print("appid-sau-close: KHONG DUOC PHEP")
                Err(e): print("appid-sau-close {e.message}")
            match sqlite.tables(h):
                Ok(_): print("tables-sau-close: KHONG DUOC PHEP")
                Err(e): print("tables-sau-close {e.message}")
            match sqlite.journal_mode(h):
                Ok(_): print("journal-sau-close: KHONG DUOC PHEP")
                Err(e): print("journal-sau-close {e.message}")
"#;

#[test]
fn sqlite_metadata_parity() {
    let d = tam("sqlite-meta");
    let (out, err) = hai_duong(&d, "sqlite-meta", SRC_SQLITE_META);
    assert_eq!(err, "", "sqlite metadata không được có stderr: {err}");
    let dong: Vec<&str> = out.lines().collect();
    assert_eq!(
        &dong[..],
        &[
            "appid=Ok(0)",
            "userv=Ok(0)",
            "sv0-khong-am=true",
            "datav-duong=true",
            "journal=memory",
            "tables0=[]",
            "dbs=2",
            "db=[Integer(0), Text(main), Text()]",
            "db=[Integer(1), Text(temp), Text()]",
            "tables=[t, v_t]",
            "table-t=Ok(true)",
            "table-v=Ok(true)",
            "table-x=Ok(false)",
            "cols=[id, a, b]",
            "idx=[i_ab]",
            "idx-ab=Ok(true)",
            "idx-auto=Ok(true)",
            "idx-x=Ok(false)",
            "idxcols=[b, a]",
            "pk=[id]",
            "sql t=CREATE TABLE t (id INTEGER PRIMARY KEY, a TEXT UNIQUE, b REAL)",
            "sql v_t=CREATE VIEW v_t AS SELECT a FROM t",
            "sql i_ab=CREATE INDEX i_ab ON t(b, a)",
            "sql sqlite_autoindex_t_1=None",
            "sql khong_co=None",
            "sv-tang=true",
            "datav-khong-doi=true",
            "close ok",
            "appid-sau-close handle đã đóng",
            "tables-sau-close handle đã đóng",
            "journal-sau-close handle đã đóng",
        ],
        "sqlite metadata: {out}\nstderr: {err}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
