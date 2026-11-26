// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests cho **VD-270 (db-batch)** — [`DbHost::db_execute_batch`] trên host SQLite.
//! Tách khỏi `tests.rs` để giữ luật R1 (≤ 1.500 dòng/tệp).

use super::*;

fn mo_db(d: &mut Db) -> String {
    d.db_open(":memory:", true, 5_000).expect("open :memory:")
}

fn o_int(rows: &[Vec<Value>]) -> i64 {
    assert_eq!(rows.len(), 1, "phải có đúng 1 hàng: {rows:?}");
    let row = &rows[0];
    assert_eq!(row.len(), 1, "phải có đúng 1 ô: {row:?}");
    let Some((e, v, args)) = row[0].as_enum() else {
        panic!("ô không phải enum: {row:?}")
    };
    assert_eq!(e, "SqlValue");
    assert_eq!(v, "Integer");
    assert_eq!(args.len(), 1);
    let Value::Int(n) = &args[0] else {
        panic!("ô không phải int")
    };
    *n
}

fn dem(d: &mut Db, h: &str, bang: &str) -> i64 {
    let (_, rows) = d
        .db_query(h, &format!("SELECT count(*) FROM {bang}"), &[])
        .expect("đếm hàng");
    o_int(&rows)
}

#[test]
fn batch_nhieu_cau_ddl_dml_tra_so_cau() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let n = d
        .db_execute_batch(
            &h,
            "CREATE TABLE t (a INTEGER);\n\
             INSERT INTO t VALUES (1);\n\
             INSERT INTO t VALUES (2);\n",
        )
        .expect("batch hợp lệ");
    assert_eq!(n, 3, "phải trả số câu đã chạy");
    assert_eq!(dem(&mut d, &h, "t"), 2);
    // `execute` vẫn **một** câu — không âm thầm nhận multi.
    let e = d
        .db_execute(
            &h,
            "INSERT INTO t VALUES (3); INSERT INTO t VALUES (4);",
            &[],
        )
        .unwrap_err();
    assert!(e.contains("multi-statement"), "{e}");
    assert_eq!(dem(&mut d, &h, "t"), 2);
}

#[test]
fn batch_rong_chi_comment_va_handle() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    for sql in [
        "",
        "   \n\t\r",
        "-- chi comment",
        "/* chi comment */",
        ";;;",
        " ; -- x\n ; /* y */ ",
    ] {
        assert_eq!(
            d.db_execute_batch(&h, sql).expect("batch rỗng phải Ok"),
            0,
            "sql={sql:?}"
        );
    }
    // Handle vẫn được kiểm kể cả batch rỗng — không im lặng nuốt handle sai/đã đóng.
    let e = d.db_execute_batch("db:0", "").unwrap_err();
    assert!(e.contains("sai dạng"), "{e}");
    let gia = format!("db:{}", "0".repeat(32));
    let e = d.db_execute_batch(&gia, "").unwrap_err();
    assert!(e.contains("không tồn tại"), "{e}");
    d.db_close(&h).unwrap();
    let e = d.db_execute_batch(&h, "").unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
}

#[test]
fn batch_dung_o_loi_dau_tien_giu_tac_dung() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE p (a INTEGER)", &[]).unwrap();
    let e = d
        .db_execute_batch(
            &h,
            "INSERT INTO p VALUES (1);\n\
             INSERT INTO khong_co_bang VALUES (2);\n\
             INSERT INTO p VALUES (3);\n",
        )
        .unwrap_err();
    assert!(!e.is_empty());
    assert_eq!(dem(&mut d, &h, "p"), 1, "chỉ câu trước lỗi được chạy");
    assert!(!d.db_in_transaction(&h).unwrap(), "không tự mở transaction");
}

#[test]
fn batch_khong_tu_rollback_transaction_nguoi_dung() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE p (a INTEGER)", &[]).unwrap();
    let e = d
        .db_execute_batch(
            &h,
            "BEGIN;\nINSERT INTO p VALUES (1);\nINSERT INTO khong_co_bang VALUES (9);\n",
        )
        .unwrap_err();
    assert!(!e.is_empty());
    assert!(
        d.db_in_transaction(&h).unwrap(),
        "lỗi giữa batch không được rollback hộ transaction người dùng"
    );
    assert_eq!(dem(&mut d, &h, "p"), 1);
    d.db_execute(&h, "ROLLBACK", &[]).unwrap();
    assert_eq!(dem(&mut d, &h, "p"), 0);
}

#[test]
fn batch_chinh_sach_authorizer_ap_tung_cau() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE s (a TEXT)", &[]).unwrap();
    let am = [
        "ATTACH DATABASE ':memory:' AS x",
        "PRAGMA user_version = 7",
        "CREATE VIRTUAL TABLE v USING fts5(x)",
        "VACUUM",
        "SELECT load_extension('/tmp/khong-duoc.so')",
    ];
    for (i, xau) in am.iter().enumerate() {
        let sql = format!("INSERT INTO s VALUES ('b{i}');\n{xau};");
        let e = d.db_execute_batch(&h, &sql).unwrap_err();
        assert!(!e.is_empty(), "{xau}");
    }
    assert_eq!(
        dem(&mut d, &h, "s"),
        5,
        "câu INSERT trước mỗi câu bị chặn vẫn chạy"
    );
}

#[test]
fn batch_cap_sql_theo_tung_cau_khong_co_tran_tong() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE s (a TEXT)", &[]).unwrap();
    let lon = format!("INSERT INTO s VALUES ('{}');", "x".repeat(MAX_SQL_BYTES));
    let e = d
        .db_execute_batch(&h, &format!("INSERT INTO s VALUES ('truoc');\n{lon}"))
        .unwrap_err();
    assert!(e.contains("65536"), "{e}");
    assert_eq!(dem(&mut d, &h, "s"), 1, "câu trước câu quá dài vẫn chạy");
    // Tổng batch > 64 KiB nhưng **mỗi câu** dưới trần ⇒ hợp lệ (trần áp từng câu).
    let hai = format!(
        "INSERT INTO s VALUES ('{}');\nINSERT INTO s VALUES ('{}');",
        "y".repeat(40_000),
        "z".repeat(40_000)
    );
    assert_eq!(d.db_execute_batch(&h, &hai).unwrap(), 2);
    assert_eq!(dem(&mut d, &h, "s"), 3);
}

#[test]
fn batch_timeout_tung_cau_va_handler_reset() {
    let mut d = Db::moi_voi_query_ms(false, 100);
    let h = mo_db(&mut d);
    let t0 = Instant::now();
    let e = d
        .db_execute_batch(
            &h,
            "CREATE TABLE slow (x INTEGER);\n\
             INSERT INTO slow SELECT count(*) FROM (WITH RECURSIVE c(x) AS \
             (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000000) SELECT x FROM c);\n",
        )
        .unwrap_err();
    let dt = t0.elapsed();
    assert!(e.contains("OperationInterrupted"), "{e}");
    assert!(
        dt < Duration::from_secs(5),
        "timeout 100 ms không được treo: {dt:?}"
    );
    assert_eq!(dem(&mut d, &h, "slow"), 0, "câu ghi bị ngắt phải rollback");
    // Handler được reset sau lỗi: batch sau chạy bình thường.
    assert_eq!(
        d.db_execute_batch(
            &h,
            "INSERT INTO slow VALUES (7); INSERT INTO slow VALUES (8);"
        )
        .unwrap(),
        2
    );
    assert_eq!(dem(&mut d, &h, "slow"), 2);
}
