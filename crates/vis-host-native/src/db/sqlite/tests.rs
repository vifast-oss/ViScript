// SPDX-License-Identifier: MIT OR Apache-2.0
//! Unit tests cho [`super`] (tách theo luật R1 ≤ 1.500 dòng/tệp).

use super::*;
use rusqlite::hooks::AuthContext;

fn sv_int(n: i64) -> Value {
    Value::Enum("SqlValue".into(), "Integer".into(), vec![Value::Int(n)])
}

fn sv_real(f: f64) -> Value {
    Value::Enum("SqlValue".into(), "Real".into(), vec![Value::Float(f)])
}

fn sv_text(s: &str) -> Value {
    Value::Enum(
        "SqlValue".into(),
        "Text".into(),
        vec![Value::Str(s.to_string())],
    )
}

fn sv_blob(b: Vec<u8>) -> Value {
    Value::Enum("SqlValue".into(), "Blob".into(), vec![Value::Bytes(b)])
}

fn mo_db(d: &mut Db) -> String {
    d.db_open(":memory:", true, 5_000).expect("open :memory:")
}

fn o_int(rows: &[Vec<Value>]) -> i64 {
    assert_eq!(rows.len(), 1, "phải có đúng 1 hàng: {rows:?}");
    let row = &rows[0];
    assert_eq!(row.len(), 1, "phải có đúng 1 ô: {row:?}");
    let Value::Enum(e, v, args) = &row[0] else {
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

/// PRAGMA host set (trước authorizer) — **cùng chuỗi** mà [`DbHost::db_open`] dùng
/// (`HOST_PRAGMAS`), đọc back trên connection raw cùng cấu hình: authorizer (đã cài trên
/// connection của host) chặn **chính** các PRAGMA đó với người dùng — xem
/// `temp_store_bi_ep_memory_va_cache_2mib`.
fn cau_hinh_temp_store_that() -> (i64, i64, i64) {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(HOST_PRAGMAS).unwrap();
    let ts: i64 = c
        .pragma_query_value(None, "temp_store", |r| r.get(0))
        .expect("đọc pragma temp_store");
    let trusted: i64 = c
        .pragma_query_value(None, "trusted_schema", |r| r.get(0))
        .expect("đọc pragma trusted_schema");
    let cache: i64 = c
        .pragma_query_value(None, "cache_size", |r| r.get(0))
        .expect("đọc pragma cache_size");
    (ts, trusted, cache)
}

mod sqlite_a;
mod sqlite_b;
mod sqlite_c;
