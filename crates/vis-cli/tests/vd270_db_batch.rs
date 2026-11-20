// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (db-batch)** — `sqlite.execute_batch(handle, sql) -> Result[int, Error]`.
//!
//! Hợp đồng khoá ở đây:
//! - Tách batch trên `;` **ở ngoài** chuỗi `'…'`/`"…"`/`` `…` ``/`[…]` và comment
//!   `--…`/`/*…*/`; mỗi mảnh **nguyên văn** đi qua đúng đường `execute` hiện có.
//! - Batch rỗng/chỉ khoảng trắng/chỉ comment (kể cả `;;;`) ⇒ `Ok(0)`.
//! - Lỗi ở câu thứ N ⇒ `Err`; các câu trước **đã có tác dụng** (không tự mở transaction,
//!   không rollback hộ). `execute` vẫn **một** câu.
//! - Authorizer/chính sách, trần SQL 64 KiB **mỗi câu**, timeout 5 s **mỗi câu** giữ nguyên.
//! - Mọi ca chạy **cả hai** engine (interp + `--vm`) trên tệp SQLite tạm riêng từng engine,
//!   so byte stdout/stderr/exit.
//!
//! **RED-BEFORE (2026-11-21)**: trước khi cài, ca migration hỏng với
//! `VS0301: module std.sqlite không có hàm execute_batch` (cả 4 ca đều gọi hàm này nên cùng lỗi
//! checker); sau khi cài cả 4 ca xanh.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!(
        "vis-vd270-db-batch-{tag}-{}-{n}",
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

/// Migration nhiều DDL/DML trong **một** lời gọi; dấu `;` trong chuỗi/comment/định danh không
/// tách câu; batch rỗng/chỉ comment ⇒ 0; `execute` vẫn từ chối multi-statement; mở lại tệp
/// thấy dữ liệu đã ghi.
#[test]
fn migration_nhieu_cau_va_tach_an_toan() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open_with("__DB__", true, 5000):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let sql = r"""
CREATE TABLE t (id INTEGER PRIMARY KEY, note TEXT, "ghi; chu" TEXT);
CREATE INDEX idx_t_note ON t(note);
CREATE TABLE `b;t` (`c;1` TEXT);
CREATE TABLE [a;b] ([c;d] TEXT);
INSERT INTO t (note, "ghi; chu") VALUES ('mot; hai', 'x');
INSERT INTO t (note) VALUES ('O''Clock; x');
-- comment co ; giua
INSERT INTO t (note) VALUES ('sau comment');
/* block ; comment */
INSERT INTO t (note) VALUES ('sau block');
INSERT INTO `b;t` (`c;1`) VALUES ('b');
INSERT INTO [a;b] ([c;d]) VALUES ('a');
-- tail comment; khong phai cau
"""
            match sqlite.execute_batch(h, sql):
                Ok(n): print("batch n={n}")
                Err(e): print("batch loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("count={rs.rows[0][0]}")
                Err(e): print("count loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_t_note'", []):
                Ok(rs): print("index={rs.rows[0][0]}")
                Err(e): print("index loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM `b;t`", []):
                Ok(rs): print("backtick={rs.rows[0][0]}")
                Err(e): print("backtick loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM [a;b]", []):
                Ok(rs): print("bracket={rs.rows[0][0]}")
                Err(e): print("bracket loi {e.message}")
            for s in ["", "   \n\t", "-- chi comment", "/* chi comment */", ";;;"]:
                match sqlite.execute_batch(h, s):
                    Ok(n): print("rong n={n}")
                    Err(e): print("rong loi {e.message}")
            match sqlite.execute(h, "SELECT 1; SELECT 2", []):
                Ok(_): print("execute multi khong duoc phep")
                Err(e): print("execute multi bi chan")
            match sqlite.execute_batch(h, "BEGIN; INSERT INTO t (note) VALUES ('tx'); COMMIT;"):
                Ok(n): print("tx n={n}")
                Err(e): print("tx loi {e.message}")
            match sqlite.in_transaction(h):
                Ok(b): print("tx con mo={b}")
                Err(e): print("tx trang thai loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("count2={rs.rows[0][0]}")
                Err(e): print("count2 loi {e.message}")
            let _ = sqlite.close(h)
    match sqlite.open_with("__DB__", false, 5000):
        Ok(h2):
            match sqlite.query(h2, "SELECT count(*) FROM t", []):
                Ok(rs): print("mo lai count={rs.rows[0][0]}")
                Err(e): print("mo lai loi {e.message}")
            let _ = sqlite.close(h2)
        Err(e): print("mo lai that bai {e.message}")
"#;
    let (code, out, err) = hai_engine("migration", |db| thay_db(SRC, db));
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "batch n=10\n\
         count=Integer(4)\n\
         index=Integer(1)\n\
         backtick=Integer(1)\n\
         bracket=Integer(1)\n\
         rong n=0\n\
         rong n=0\n\
         rong n=0\n\
         rong n=0\n\
         rong n=0\n\
         execute multi bi chan\n\
         tx n=3\n\
         tx con mo=false\n\
         count2=Integer(5)\n\
         mo lai count=Integer(5)\n",
        "batch phải tách đúng 10 câu, giữ `;` trong chuỗi/comment/định danh"
    );
}

/// Lỗi giữa batch: dừng ngay, câu trước **đã có tác dụng**; không tự mở transaction. Nếu người
/// dùng tự `BEGIN` trong batch thì lỗi **không** rollback hộ — transaction vẫn mở để họ quyết định.
#[test]
fn loi_giua_batch_giu_tac_dung_mot_phan() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE p (a INTEGER)", [])
            match sqlite.execute_batch(h, "INSERT INTO p VALUES (1);\nINSERT INTO khong_co_bang VALUES (2);\nINSERT INTO p VALUES (3);"):
                Ok(n): print("khong duoc phep n={n}")
                Err(e): print("chan giua chung")
            match sqlite.query(h, "SELECT count(*) FROM p", []):
                Ok(rs): print("count={rs.rows[0][0]}")
                Err(e): print("count loi {e.message}")
            match sqlite.in_transaction(h):
                Ok(b): print("tx={b}")
                Err(e): print("tx loi {e.message}")
            match sqlite.execute_batch(h, "BEGIN;\nINSERT INTO p VALUES (2);\nINSERT INTO khong_co_bang VALUES (9);"):
                Ok(n): print("khong duoc phep n={n}")
                Err(e): print("chan trong tx")
            match sqlite.in_transaction(h):
                Ok(b): print("tx sau loi={b}")
                Err(e): print("tx sau loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM p", []):
                Ok(rs): print("count tx={rs.rows[0][0]}")
                Err(e): print("count tx loi {e.message}")
            match sqlite.rollback(h):
                Ok(_): print("rollback ok")
                Err(e): print("rollback loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM p", []):
                Ok(rs): print("count sau rollback={rs.rows[0][0]}")
                Err(e): print("count sau rollback loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("loi-giua", |_| SRC.to_string());
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "chan giua chung\n\
         count=Integer(1)\n\
         tx=false\n\
         chan trong tx\n\
         tx sau loi=true\n\
         count tx=Integer(2)\n\
         rollback ok\n\
         count sau rollback=Integer(1)\n",
        "câu trước lỗi phải còn tác dụng; batch không tự rollback transaction người dùng"
    );
}

/// Authorizer/chính sách áp **từng câu**; trần 64 KiB áp **mỗi câu** (tổng batch lớn hơn vẫn
/// chạy nếu từng câu dưới trần), câu sau lỗi không xoá tác dụng câu trước.
#[test]
fn chinh_sach_va_cap_moi_cau() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let _ = sqlite.execute(h, "CREATE TABLE s (a TEXT)", [])
            let sqls = [
                "INSERT INTO s VALUES ('b1');\nATTACH DATABASE ':memory:' AS x;",
                "INSERT INTO s VALUES ('b2');\nPRAGMA user_version = 7;",
                "INSERT INTO s VALUES ('b3');\nCREATE VIRTUAL TABLE v USING fts5(x);",
                "INSERT INTO s VALUES ('b4');\nVACUUM;",
                "INSERT INTO s VALUES ('b5');\nSELECT load_extension('/tmp/khong-duoc.so');"
            ]
            for s in sqls:
                match sqlite.execute_batch(h, s):
                    Ok(n): print("khong duoc phep n={n}")
                    Err(e): print("chan chinh sach")
            match sqlite.query(h, "SELECT count(*) FROM s", []):
                Ok(rs): print("count={rs.rows[0][0]}")
                Err(e): print("count loi {e.message}")
            let lon = "INSERT INTO s VALUES ('" + "x".repeat(70000) + "');"
            match sqlite.execute_batch(h, "INSERT INTO s VALUES ('truoc');\n" + lon):
                Ok(n): print("sql lon khong duoc phep n={n}")
                Err(e): print("chan sql lon")
            match sqlite.query(h, "SELECT count(*) FROM s", []):
                Ok(rs): print("count2={rs.rows[0][0]}")
                Err(e): print("count2 loi {e.message}")
            let hai = "INSERT INTO s VALUES ('" + "y".repeat(40000) + "');\nINSERT INTO s VALUES ('" + "z".repeat(40000) + "');"
            match sqlite.execute_batch(h, hai):
                Ok(n): print("hai cau nho n={n}")
                Err(e): print("hai cau nho loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM s", []):
                Ok(rs): print("count3={rs.rows[0][0]}")
                Err(e): print("count3 loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let (code, out, err) = hai_engine("chinh-sach", |_| SRC.to_string());
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "chan chinh sach\n\
         chan chinh sach\n\
         chan chinh sach\n\
         chan chinh sach\n\
         chan chinh sach\n\
         count=Integer(5)\n\
         chan sql lon\n\
         count2=Integer(6)\n\
         hai cau nho n=2\n\
         count3=Integer(8)\n",
        "authorizer + trần 64 KiB phải áp từng câu; tổng batch không có trần riêng"
    );
}

/// Timeout 5 s của **progress handler** áp từng câu: câu ghi chậm bị ngắt (không treo), câu
/// trước đã chạy, handler reset để batch sau chạy bình thường.
#[test]
fn timeout_moi_cau_va_handler_reset() {
    const SRC: &str = r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Err(e): print("mo loi {e.message}")
        Ok(h):
            let slow = "CREATE TABLE slow (x INTEGER);\nINSERT INTO slow SELECT count(*) FROM (WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000000) SELECT x FROM c);"
            match sqlite.execute_batch(h, slow):
                Ok(n): print("khong duoc phep n={n}")
                Err(e): print("timeout chan")
            match sqlite.query(h, "SELECT count(*) FROM slow", []):
                Ok(rs): print("slow count={rs.rows[0][0]}")
                Err(e): print("slow count loi {e.message}")
            match sqlite.execute_batch(h, "INSERT INTO slow VALUES (7);\nINSERT INTO slow VALUES (8);"):
                Ok(n): print("sau timeout n={n}")
                Err(e): print("sau timeout loi {e.message}")
            match sqlite.query(h, "SELECT count(*) FROM slow", []):
                Ok(rs): print("sau count={rs.rows[0][0]}")
                Err(e): print("sau count loi {e.message}")
            let _ = sqlite.close(h)
"#;
    let t0 = Instant::now();
    let (code, out, err) = hai_engine("timeout", |_| SRC.to_string());
    let dt = t0.elapsed();
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "timeout chan\n\
         slow count=Integer(0)\n\
         sau timeout n=2\n\
         sau count=Integer(2)\n",
        "câu chậm phải bị ngắt; câu trước giữ tác dụng; handler reset"
    );
    assert!(
        dt < Duration::from_secs(30),
        "timeout 5 s mỗi engine không được treo: {dt:?}"
    );
}
