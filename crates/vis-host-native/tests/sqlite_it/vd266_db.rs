// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-266** — ba helper SQLite thực dụng: `sqlite.query_one`, `sqlite.last_insert_rowid`,
//! `sqlite.changes`.
//!
//! Hợp đồng khoá ở đây:
//! - `query_one` trả `RowSet` **0 hoặc 1** hàng; câu trả nhiều hàng chỉ lấy hàng **đầu** và
//!   **dừng** (không materialize/scan phần còn lại — chứng minh bằng CTE đệ quy > trần 10k hàng:
//!   `query` thường sẽ `Err`, `query_one` vẫn `Ok`).
//! - `query_one` **chỉ đọc** y như `query`: câu ghi/DDL/`BEGIN` ⇒ `Err` dữ liệu, không side effect.
//! - `last_insert_rowid`/`changes` là metadata chỉ đọc của connection.
//! - Mọi lỗi SQL/params/handle là `Result.Err` dữ liệu, **không** trap; thiếu host DB ⇒ `VS0604`.
//! - Mọi ca chạy **cả hai** engine (interp + `--vm`) trên **tệp SQLite tạm riêng từng engine**,
//!   so byte stdout/stderr/exit.
//!
//! **Red-before (2026-11-21)**: trước khi cài, `cargo test -p vis-cli --test vd266_db` đỏ **3/3**
//! với `VS0301 module std.sqlite không có hàm query_one/last_insert_rowid/changes`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static DEM: AtomicU32 = AtomicU32::new(0);

fn tam(tag: &str) -> PathBuf {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vis-vd266-db-{tag}-{}-{n}", std::process::id()));
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

fn nguon_chinh(db: &Path) -> String {
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
            let _ = sqlite.execute(h, "CREATE TABLE t (a INTEGER, b TEXT)", [])
            match sqlite.execute(h, "INSERT INTO t (a, b) VALUES (1, 'mot')", []):
                Ok(n): print("insert1 n={{n}}")
                Err(e): print("insert1 loi {{e.message}}")
            match sqlite.last_insert_rowid(h):
                Ok(r): print("rowid1={{r}}")
                Err(e): print("rowid1 loi {{e.message}}")
            let _ = sqlite.execute(h, "INSERT INTO t (a, b) VALUES (2, 'hai')", [])
            match sqlite.last_insert_rowid(h):
                Ok(r): print("rowid2={{r}}")
                Err(e): print("rowid2 loi {{e.message}}")

            match sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(9)]):
                Ok(rs): print("q0 rows={{rs.rows.len()}} cols={{rs.columns.len()}}")
                Err(e): print("q0 loi {{e.message}}")
            match sqlite.query_one(h, "SELECT a, b FROM t WHERE a = ?", [SqlValue.Integer(1)]):
                Ok(rs):
                    print("q1 rows={{rs.rows.len()}} cot0={{rs.columns[0]}}")
                    print("q1 cell={{hien(rs.rows[0][0])}} text={{hien(rs.rows[0][1])}}")
                Err(e): print("q1 loi {{e.message}}")
            match sqlite.query_one(h, "SELECT a FROM t ORDER BY a", []):
                Ok(rs): print("qn rows={{rs.rows.len()}} first={{hien(rs.rows[0][0])}}")
                Err(e): print("qn loi {{e.message}}")
            let big = "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 100000) SELECT x FROM c"
            match sqlite.query_one(h, big, []):
                Ok(rs): print("qbig rows={{rs.rows.len()}} first={{hien(rs.rows[0][0])}}")
                Err(e): print("qbig loi {{e.message}}")

            match sqlite.query_one(h, "INSERT INTO t (a, b) VALUES (3, 'ba')", []):
                Ok(_): print("ghi khong duoc phep")
                Err(e): print("ghi bi chan")
            match sqlite.query_one(h, "BEGIN", []):
                Ok(_): print("begin khong duoc phep")
                Err(e): print("begin bi chan")
            match sqlite.query(h, "SELECT count(*) FROM t", []):
                Ok(rs): print("count={{hien(rs.rows[0][0])}}")
                Err(e): print("count loi {{e.message}}")

            match sqlite.execute(h, "UPDATE t SET b = 'z' WHERE a = ?", [SqlValue.Integer(1)]):
                Ok(n): print("update n={{n}}")
                Err(e): print("update loi {{e.message}}")
            match sqlite.changes(h):
                Ok(n): print("changes={{n}}")
                Err(e): print("changes loi {{e.message}}")
            match sqlite.execute(h, "UPDATE t SET b = 'z' WHERE a = ?", [SqlValue.Integer(99)]):
                Ok(n): print("noop n={{n}}")
                Err(e): print("noop loi {{e.message}}")
            match sqlite.changes(h):
                Ok(n): print("changes noop={{n}}")
                Err(e): print("changes noop loi {{e.message}}")

            match sqlite.query_one(h, "SELECT ?", []):
                Ok(_): print("thieu param khong duoc phep")
                Err(e): print("thieu param bi chan")
            match sqlite.query_one(h, "SELECT 1", [SqlValue.Integer(1)]):
                Ok(_): print("thua param khong duoc phep")
                Err(e): print("thua param bi chan")
            match sqlite.query_one(h, "SELECT * FROM khong_co_bang", []):
                Ok(_): print("sql loi khong duoc phep")
                Err(e): print("sql loi bi chan")
            match sqlite.query_one(h, "SELECT 1; SELECT 2", []):
                Ok(_): print("multi khong duoc phep")
                Err(e): print("multi bi chan")

            let _ = sqlite.close(h)
            match sqlite.query_one(h, "SELECT 1", []):
                Ok(_): print("query_one handle dong khong duoc phep")
                Err(e): print("query_one handle dong bi chan")
            match sqlite.last_insert_rowid(h):
                Ok(_): print("rowid handle dong khong duoc phep")
                Err(e): print("rowid handle dong bi chan")
            match sqlite.changes(h):
                Ok(_): print("changes handle dong khong duoc phep")
                Err(e): print("changes handle dong bi chan")
"#
    )
}

#[test]
fn query_one_va_metadata_tren_tep_tam() {
    let (code, out, err) = hai_engine("chinh", nguon_chinh);
    assert_eq!(code, 0, "phải chạy sạch; stderr: {err}");
    assert_eq!(
        out,
        "insert1 n=1\n\
         rowid1=1\n\
         rowid2=2\n\
         q0 rows=0 cols=1\n\
         q1 rows=1 cot0=a\n\
         q1 cell=int(1) text=text(mot)\n\
         qn rows=1 first=int(1)\n\
         qbig rows=1 first=int(1)\n\
         ghi bi chan\n\
         begin bi chan\n\
         count=int(2)\n\
         update n=1\n\
         changes=1\n\
         noop n=0\n\
         changes noop=0\n\
         thieu param bi chan\n\
         thua param bi chan\n\
         sql loi bi chan\n\
         multi bi chan\n\
         query_one handle dong bi chan\n\
         rowid handle dong bi chan\n\
         changes handle dong bi chan\n",
        "stdout phải đúng ngữ nghĩa 0/1/nhiều hàng + metadata + lỗi dữ liệu"
    );
}

/// Lỗi kiểu/arity là **lỗi tĩnh** (checker), không phải `Result.Err` chạy được.
#[test]
fn query_one_kiem_tra_kieu_tinh() {
    let d = tam("kieu");
    let sai_kieu = ghi(
        &d,
        "sai-kieu",
        r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.query_one(h, "SELECT ?", [1])
        Err(e): print(e.message)
"#,
    );
    let o = Command::new(crate::common::vis_bin())
        .arg("check")
        .arg(&sai_kieu)
        .output()
        .expect("chạy `vis check`");
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert_ne!(o.status.code(), Some(0), "sai kiểu phải bị chặn: {out}");
    assert!(
        out.contains("VS0412") && out.contains("SqlValue"),
        "phải báo `VS0412` cần `SqlValue`, được: {out:?}"
    );

    let sai_arity = ghi(
        &d,
        "sai-arity",
        r#"use std.sqlite

fn main():
    match sqlite.open(":memory:"):
        Ok(h):
            let _ = sqlite.query_one(h, "SELECT 1")
        Err(e): print(e.message)
"#,
    );
    let o = Command::new(crate::common::vis_bin())
        .arg("check")
        .arg(&sai_arity)
        .output()
        .expect("chạy `vis check`");
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert_ne!(o.status.code(), Some(0), "sai arity phải bị chặn: {out}");
    assert!(
        out.contains("VS0401") && out.contains("3 đối số"),
        "phải báo `VS0401` cần 3 đối số, được: {out:?}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// Thiếu host DB ⇒ `VS0604` (trap), **không** `Err` dữ liệu; hai engine cùng byte.
#[test]
fn query_one_khong_host_la_vs0604() {
    let src = "use std.sqlite\n\
               fn main():\n\
               \x20   match sqlite.query_one(\"db:00000000000000000000000000000000\", \"SELECT 1\", []):\n\
               \x20       Ok(_): print(\"khong duoc phep\")\n\
               \x20       Err(e): print(e.message)\n";
    let text = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}");
    let p = vis_syntax::parse(&text, vis_syntax::Mode::Script);
    let lx = vis_syntax::lex(&text, vis_syntax::Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &lx,
        vis_types::LowerOptions {
            std_mods: vec!["sqlite".into()],
            ..Default::default()
        },
    );
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(
        t.is_clean(),
        "phải sạch kiểu: {:?}",
        t.diags
            .iter()
            .map(|d| (d.code.short, d.line, d.col, d.message.clone()))
            .collect::<Vec<_>>()
    );
    let a = vis_interp::run(&text, &h.hir, &h.symbols, &h.types, &[]);
    assert_eq!(a.codes(), ["VS0604"], "interp: {:?}", a.codes());
    let m = vis_bytecode::compile(&h.hir, &h.symbols, &text).expect("compile");
    let b = vis_vm::run(&m, &text, &[]);
    assert_eq!(b.codes(), ["VS0604"], "vm: {:?}", b.codes());
    assert_eq!(
        (a.stdout.clone(), a.exit, a.codes()),
        (b.stdout.clone(), b.exit, b.codes()),
        "hai đường phải cùng byte khi thiếu host"
    );
    assert_eq!(
        a.diagnostics.first().map(|d| d.message.clone()),
        b.diagnostics.first().map(|d| d.message.clone()),
        "cùng thông điệp `VS0604`"
    );
}
