// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268** — e2e `std.sqlite` khi host dùng **cache statement** (`Connection::prepare_cached`
//! cho cùng chuỗi SQL): hành vi phải **không đổi** so với trước.
//!
//! Phủ: query/query_one/execute lặp lại cùng SQL + tham số; `ALTER TABLE … ADD COLUMN` giữa hai
//! lần query `SELECT *` (metadata cột không được cũ); `BEGIN`/`COMMIT`/`ROLLBACK`; các ca âm
//! chính sách (ghi qua `query`, `BEGIN` qua `query`, `SELECT` qua `execute`, ATTACH, ghi PRAGMA,
//! `query_one` ghi); đóng/mở lại handle trên tệp. Mọi ca chạy **cả hai** engine (interp và
//! `--vm --no-cache`), mỗi engine một tệp DB tạm riêng, so **từng byte** stdout/stderr/exit.
//!
//! **Red-before (2026-11-21)**: bản không cache vẫn **xanh** ở đây (cache không được đổi kết
//! quả); phần chứng minh cache thật sự được dùng nằm ở unit test
//! `db::sqlite::tests_cache::cache_tai_su_dung_khong_prepare_lai` (đỏ 21/1 khi bỏ cache). Ca
//! `ALTER` là lưới hồi quy cho lỗi metadata cột cũ nếu thiếu xả cache theo `schema_version`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd268-dbcache-{tag}-{}-{n}",
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

/// Hai engine trên **hai tệp DB tạm riêng** (cùng nguồn, khác path) — stdout/stderr/exit phải
/// **byte-identical**; trả kết quả đường interp.
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

fn nguon(db: &Path) -> String {
    format!(
        r#"use std.sqlite

fn hien(v: SqlValue) -> str:
    match v:
        Null: "null"
        Integer(n): "int({{n}})"
        Real(f): "real"
        Text(s): "text({{s}})"
        Blob(b): "blob({{len(b)}})"

fn main():
    match sqlite.open_with({db:?}, true, 5000):
        Err(e): print("mo loi {{e.message}}")
        Ok(h):
            match sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", []):
                Ok(n): print("create={{n}}")
                Err(e): print("create loi {{e.message}}")
            var so_insert = 0
            for i in 0..200:
                match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(i), SqlValue.Text("x")]):
                    Ok(n): so_insert += n
                    Err(e): print("insert loi {{e.message}}")
            print("inserts={{so_insert}}")

            # 200 lần cùng SQL + tham số qua query_one.
            var dem_one = 0
            for i in 0..200:
                match sqlite.query_one(h, "SELECT b FROM t WHERE a = ?", [SqlValue.Integer(i)]):
                    Ok(rs):
                        if rs.rows.len() == 1:
                            dem_one += 1
                    Err(e): print("qone loi {{e.message}}")
            print("qone={{dem_one}}")

            # 200 lần cùng SQL qua query; tổng phải không đổi.
            var tong = 0
            for i in 0..200:
                match sqlite.query(h, "SELECT a FROM t WHERE a >= ? ORDER BY a", [SqlValue.Integer(0)]):
                    Ok(rs):
                        for row in rs.rows:
                            match row[0]:
                                Integer(n): tong += n
                                _: print("cell la")
                    Err(e): print("q loi {{e.message}}")
            print("tong={{tong}}")

            # Cùng câu `SELECT *` trước/sau ALTER: cache không được trả metadata cột cũ.
            match sqlite.query(h, "SELECT * FROM t WHERE a = 0", []):
                Ok(rs): print("truoc cols={{rs.columns.len()}} ten0={{rs.columns[0]}}")
                Err(e): print("truoc loi {{e.message}}")
            match sqlite.execute(h, "ALTER TABLE t ADD COLUMN c TEXT", []):
                Ok(n): print("alter={{n}}")
                Err(e): print("alter loi {{e.message}}")
            match sqlite.query(h, "SELECT * FROM t WHERE a = 0", []):
                Ok(rs):
                    print("sau cols={{rs.columns.len()}} ten2={{rs.columns[2]}} c={{hien(rs.rows[0][2])}}")
                Err(e): print("sau loi {{e.message}}")

            # Bảng TEMP: DDL temp không đổi main schema_version — vẫn phải invalidate cache.
            let _ = sqlite.execute(h, "CREATE TEMP TABLE tt (z INTEGER)", [])
            let _ = sqlite.execute(h, "INSERT INTO tt VALUES (1)", [])
            match sqlite.query(h, "SELECT * FROM tt", []):
                Ok(rs): print("temp truoc cols={{rs.columns.len()}} ten0={{rs.columns[0]}}")
                Err(e): print("temp truoc loi {{e.message}}")
            let _ = sqlite.execute(h, "ALTER TABLE tt ADD COLUMN w TEXT", [])
            match sqlite.query(h, "SELECT * FROM tt", []):
                Ok(rs):
                    print("temp sau cols={{rs.columns.len()}} ten1={{rs.columns[1]}} w={{hien(rs.rows[0][1])}}")
                Err(e): print("temp sau loi {{e.message}}")

            # Giao dịch: statement cache dùng lại được qua BEGIN/ROLLBACK/BEGIN/COMMIT.
            let _ = sqlite.execute(h, "BEGIN", [])
            let _ = sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(1000), SqlValue.Text("tx")])
            match sqlite.query_one(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("tx count={{hien(rs.rows[0][0])}}")
                Err(e): print("tx loi {{e.message}}")
            let _ = sqlite.execute(h, "ROLLBACK", [])
            match sqlite.query_one(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("rollback count={{hien(rs.rows[0][0])}}")
                Err(e): print("rollback loi {{e.message}}")
            let _ = sqlite.execute(h, "BEGIN", [])
            let _ = sqlite.execute(h, "INSERT INTO t (a, b) VALUES (?, ?)", [SqlValue.Integer(1001), SqlValue.Text("tx2")])
            let _ = sqlite.execute(h, "COMMIT", [])
            match sqlite.query_one(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("commit count={{hien(rs.rows[0][0])}}")
                Err(e): print("commit loi {{e.message}}")

            # Ca âm chính sách lặp **hai lần** — cache không được bỏ qua kiểm tra.
            for i in 0..2:
                match sqlite.query(h, "INSERT INTO t (a, b) VALUES (3, 'z')", []):
                    Ok(_): print("ghi qua query khong bi chan")
                    Err(e): print("ghi qua query bi chan")
                match sqlite.query(h, "BEGIN", []):
                    Ok(_): print("begin qua query khong bi chan")
                    Err(e): print("begin qua query bi chan")
                match sqlite.query_one(h, "INSERT INTO t (a, b) VALUES (4, 'w')", []):
                    Ok(_): print("query_one ghi khong bi chan")
                    Err(e): print("query_one ghi bi chan")
                match sqlite.execute(h, "SELECT 1", []):
                    Ok(_): print("execute tra hang khong bi chan")
                    Err(e): print("execute tra hang bi chan")
                match sqlite.execute(h, "ATTACH DATABASE ':memory:' AS x", []):
                    Ok(_): print("attach khong bi chan")
                    Err(e): print("attach bi chan")
                match sqlite.execute(h, "PRAGMA user_version = 1", []):
                    Ok(_): print("pragma ghi khong bi chan")
                    Err(e): print("pragma ghi bi chan")
            match sqlite.query_one(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("count cuoi={{hien(rs.rows[0][0])}}")
                Err(e): print("count cuoi loi {{e.message}}")

            let _ = sqlite.close(h)
            match sqlite.query_one(h, "SELECT 1", []):
                Ok(_): print("handle dong khong bi chan")
                Err(e): print("handle dong bi chan")
            match sqlite.open_with({db:?}, false, 5000):
                Ok(h2):
                    match sqlite.query_one(h2, "SELECT count(*) FROM t", []):
                        Ok(rs): print("reopen={{hien(rs.rows[0][0])}}")
                        Err(e): print("reopen loi {{e.message}}")
                Err(e): print("reopen mo loi {{e.message}}")
"#
    )
}

#[test]
fn cache_khong_doi_ket_qua_tren_tep_tam() {
    let (code, out, err) = hai_engine("chinh", nguon);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    let mong_doi = "\
create=0\n\
inserts=200\n\
qone=200\n\
tong=3980000\n\
truoc cols=2 ten0=a\n\
alter=1\n\
sau cols=3 ten2=c c=null\n\
temp truoc cols=1 ten0=z\n\
temp sau cols=2 ten1=w w=null\n\
tx count=int(201)\n\
rollback count=int(200)\n\
commit count=int(201)\n\
ghi qua query bi chan\n\
begin qua query bi chan\n\
query_one ghi bi chan\n\
execute tra hang bi chan\n\
attach bi chan\n\
pragma ghi bi chan\n\
ghi qua query bi chan\n\
begin qua query bi chan\n\
query_one ghi bi chan\n\
execute tra hang bi chan\n\
attach bi chan\n\
pragma ghi bi chan\n\
count cuoi=int(201)\n\
handle dong bi chan\n\
reopen=int(201)\n";
    assert_eq!(
        out, mong_doi,
        "stdout phải đúng ngữ nghĩa (cache không đổi kết quả/lỗi)"
    );
}

/// Phép đếm/so khớp độc lập với cache: `query_one` lặp lại trả 0/1 hàng đúng, và `changes`
/// vẫn phản ánh **lần gần nhất** dù INSERT lấy từ cache statement.
#[test]
fn cache_query_one_va_changes_lap_lai() {
    let (code, out, err) = hai_engine("qone", |db| {
        format!(
            r#"use std.sqlite

fn hien(v: SqlValue) -> str:
    match v:
        Integer(n): "int({{n}})"
        _: "khac"

fn main():
    match sqlite.open_with({db:?}, true, 5000):
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER)", [])
            for i in 0..30:
                let _ = sqlite.execute(h, "INSERT INTO t (a) VALUES (?)", [SqlValue.Integer(i)])
            var dem = 0
            for i in 0..30:
                match sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(i)]):
                    Ok(rs):
                        if rs.rows.len() == 1:
                            dem += 1
                    Err(e): print("loi")
            print("dem={{dem}}")
            let _ = sqlite.execute(h, "UPDATE t SET a = a WHERE a = 0", [])
            match sqlite.changes(h):
                Ok(n): print("changes={{n}}")
                Err(e): print("changes loi")
            match sqlite.query_one(h, "SELECT a FROM t WHERE a = 999", []):
                Ok(rs): print("khong={{rs.rows.len()}}")
                Err(e): print("loi")
        Err(e): print("mo loi")
"#
        )
    });
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out, "dem=30\nchanges=1\nkhong=0\n",
        "query_one/changes không đổi khi statement lấy từ cache"
    );
}
