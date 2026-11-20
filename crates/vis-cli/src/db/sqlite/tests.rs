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

#[test]
fn moi_co_version_va_compile_options_that() {
    let (ver, cc) = thong_tin_sqlite();
    assert!(ver.starts_with("3."), "SQLite version lạ: {ver}");
    assert!(
        cc.iter().any(|o| o.contains("ENABLE_FTS5")),
        "bundled phải có ENABLE_FTS5, thấy: {cc:?}"
    );
}

#[test]
fn token_128_bit_hex_khong_doan_duoc_khong_tai_su_dung() {
    let mut d = Db::moi(false);
    let a = mo_db(&mut d);
    let b = mo_db(&mut d);
    // Format: `db:` + đúng 32 hex thường (128 bit).
    for h in [&a, &b] {
        assert!(h.starts_with("db:"), "{h}");
        assert_eq!(h.len(), 35, "{h}");
        assert!(
            h[3..]
                .bytes()
                .all(|x| matches!(x, b'0'..=b'9' | b'a'..=b'f')),
            "{h}"
        );
    }
    assert_ne!(a, b, "hai lần mở không sinh cùng token");
    d.db_close(&a).unwrap();
    let c = mo_db(&mut d);
    assert_ne!(c, a);
    assert_ne!(c, b);
    // Handle cũ (tombstone) vẫn báo đã đóng, không trỏ nhầm connection mới.
    let e = d.db_execute(&a, "SELECT 1", &[]).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
    // Token cũ không được cứu sống bằng bất kỳ biến thể nào của nó.
    for bien_the in [
        a.to_uppercase(),
        format!("{a} "),
        format!(" {a}"),
        format!("{}0", &a[..34]),
    ] {
        let e = d.db_execute(&bien_the, "SELECT 1", &[]).unwrap_err();
        assert!(!e.is_empty());
    }
    // Handle đã đóng không được trỏ sang connection mới: đóng a, mở mới ⇒ token mới khác a.
    let e = d.db_close(&a).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
}

#[test]
fn token_gia_sai_dang_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let hex32 = format!("db:{}", "0".repeat(32));
    assert!(token_dung_dang(&hex32));
    for bad in [
        "db:0",
        "db:00",
        "db:2",
        "db:99",
        "db:1x",
        "db:",
        "tcp:0",
        "DB:2",
        // hex nhưng hoa — từ chối (chỉ hex thường).
        "db:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        // hex giả — đúng dạng ký tự nhưng chưa mở trong phiên này.
        "db:deadbeefdeadbeefdeadbeefdeadbeef",
        "1",
        "",
    ] {
        let e = d.db_execute(bad, "SELECT 1", &[]).unwrap_err();
        if bad.starts_with("db:")
            && bad.len() == 35
            && bad[3..]
                .bytes()
                .all(|x| matches!(x, b'0'..=b'9' | b'a'..=b'f'))
        {
            assert!(e.contains("không tồn tại"), "handle {bad:?}: {e}");
        } else {
            assert!(e.contains("sai dạng"), "handle {bad:?}: {e}");
        }
    }
    // Token hợp lệ nhưng đóng rồi ⇒ "đã đóng" (không rơi về nhánh khác).
    d.db_close(&h).unwrap();
    let e = d.db_execute(&h, "SELECT 1", &[]).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
    // Không panic, không trap — chỉ `Err`.
}

#[test]
fn uri_ten_rong_memory_thua_bi_tu_choi() {
    let mut d = Db::moi(false);
    for p in [
        "",
        "file:/tmp/x.db",
        "FILE:/tmp/x.db",
        ":memory:x",
        ":MEMORY:",
    ] {
        let e = d.db_open(p, true, 5_000).unwrap_err();
        assert!(!e.is_empty(), "path {p:?}: {e}");
    }
    // `:memory:` thật thì hợp lệ.
    assert!(d.db_open(":memory:", true, 5_000).is_ok());
}

#[test]
fn busy_ngoai_khoang_bi_tu_choi() {
    let mut d = Db::moi(false);
    for ms in [-1, 60_001, i64::MAX] {
        let e = d.db_open(":memory:", true, ms).unwrap_err();
        assert!(e.contains("busy_timeout_ms"), "busy {ms}: {e}");
    }
    assert!(d.db_open(":memory:", true, 60_000).is_ok());
}

#[test]
fn connection_thu_17_bi_tu_choi() {
    let mut d = Db::moi(false);
    for _ in 0..MAX_CONNECTIONS {
        mo_db(&mut d);
    }
    let e = d.db_open(":memory:", true, 5_000).unwrap_err();
    assert!(e.contains("16"), "{e}");
}

#[test]
fn create_false_thieu_tep_la_loi_du_lieu_khong_phai_du_lieu_cat() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-create-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("khong-ton-tai.db");
    let mut d = Db::moi(false);
    let e = d.db_open(p.to_str().unwrap(), false, 5_000).unwrap_err();
    assert!(!e.starts_with(DAU_QUYEN), "không phải lỗi quyền: {e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn multi_statement_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_execute(&h, "SELECT 1; SELECT 2", &[]).unwrap_err();
    assert!(e.contains("multi-statement"), "{e}");
    let e2 = d.db_query(&h, "SELECT 1; SELECT 2", &[]).unwrap_err();
    assert!(e2.contains("multi-statement"), "{e2}");
}

#[test]
fn bind_thieu_thua_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER, b INTEGER)", &[])
        .unwrap();
    let thieu = d
        .db_execute(&h, "INSERT INTO t VALUES (?, ?)", &[sv_int(1)])
        .unwrap_err();
    assert!(thieu.contains("SQL cần"), "{thieu}");
    let thua = d
        .db_execute(
            &h,
            "INSERT INTO t VALUES (?, ?)",
            &[sv_int(1), sv_int(2), sv_int(3)],
        )
        .unwrap_err();
    assert!(thua.contains("SQL cần"), "{thua}");
    // Thiếu nhưng **đúng với câu SQL** thì ok.
    assert!(d
        .db_execute(&h, "INSERT INTO t VALUES (?, ?)", &[sv_int(1), sv_int(2)])
        .is_ok());
}

#[test]
fn sql_qua_64kib_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let sql = format!("SELECT '{}'", "x".repeat(MAX_SQL_BYTES));
    let e = d.db_query(&h, &sql, &[]).unwrap_err();
    assert!(e.contains("65536"), "{e}");
}

#[test]
fn params_qua_1000_bi_tu_choi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let params: Vec<Value> = (0..(MAX_PARAMS + 1) as i64).map(sv_int).collect();
    let e = d.db_execute(&h, "SELECT 1", &params).unwrap_err();
    assert!(e.contains("1000"), "{e}");
}

#[test]
fn query_vuot_10000_hang_err_khong_tra_du_lieu_cat() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 10001) SELECT x FROM c",
            &[],
        )
        .unwrap_err();
    assert!(e.contains("10000"), "{e}");
}

#[test]
fn row_vuot_1mib_err() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d
        .db_query(&h, "SELECT randomblob(1048577)", &[])
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");
}

#[test]
fn tong_vuot_16mib_err() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    // 200 hàng × 100_000 byte = 20_000_000 byte > 16 MiB; mỗi hàng 100 KB < 1 MiB.
    let e = d
        .db_query(
            &h,
            "SELECT randomblob(100000) FROM (WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 200) SELECT x FROM c)",
            &[],
        )
        .unwrap_err();
    assert!(e.contains("16777216"), "{e}");
}

#[test]
fn giao_dich_rollback_khong_auto_commit() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    d.db_execute(&h, "BEGIN", &[]).unwrap();
    d.db_execute(&h, "INSERT INTO t VALUES (1)", &[]).unwrap();
    // Lỗi trong transaction — không tự commit.
    let e = d.db_execute(&h, "INSERT INTO khong_co_bang VALUES (2)", &[]);
    assert!(e.is_err());
    d.db_execute(&h, "ROLLBACK", &[]).unwrap();
    let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
    assert_eq!(o_int(&rows), 0, "ROLLBACK phải hoàn tác dòng đã insert");
}

#[test]
fn in_transaction_theo_verb() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    assert!(!d.db_in_transaction(&h).unwrap());
    d.db_execute(&h, "BEGIN", &[]).unwrap();
    assert!(d.db_in_transaction(&h).unwrap());
    d.db_execute(&h, "COMMIT", &[]).unwrap();
    assert!(!d.db_in_transaction(&h).unwrap());
    // COMMIT khi không có transaction ⇒ lỗi dữ liệu (không phải panic).
    let e = d.db_execute(&h, "COMMIT", &[]);
    assert!(e.is_err());
    assert!(!e.unwrap_err().starts_with(DAU_QUYEN));
}

#[test]
fn authorizer_chan_attach_pragma_ghi_load_extension() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    for sql in [
        "ATTACH '/tmp/xx-khong-duoc.db' AS x",
        "DETACH x",
        "PRAGMA user_version=1",
        "PRAGMA user_version = 42",
        "PRAGMA journal_mode=MEMORY",
        "PRAGMA temp_store=FILE",
        "PRAGMA temp_store=OFF",
        "PRAGMA synchronous=OFF",
        "PRAGMA cache_size = -1",
        "VACUUM INTO '/tmp/xx-khong-duoc.db'",
        "VACUUM",
        "CREATE VIRTUAL TABLE tv USING fts5(x)",
    ] {
        let r = d.db_execute(&h, sql, &[]);
        assert!(r.is_err(), "không được phép: {sql}");
    }
    // SELECT load_extension — SQLite tắt sẵn ở tầng C; authorizer chặn `load_extension`.
    let r = d.db_execute(&h, "SELECT load_extension('/tmp/xx.so')", &[]);
    assert!(r.is_err());
    // Authorizer phải chặn **trước** khi SQLite kịp báo "no such module"/"near".
    let e = d
        .db_execute(&h, "CREATE VIRTUAL TABLE tv2 USING fts5(x)", &[])
        .unwrap_err();
    assert!(e.contains("quyền (authorizer)"), "{e}");
    // table_info và đọc user_version thì được phép.
    assert!(d.db_query(&h, "PRAGMA table_info(t)", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA user_version", &[]).is_ok());
    // DDL/CRUD/maintenance thường thì được phép.
    assert!(d.db_execute(&h, "CREATE INDEX idx_t ON t (a)", &[]).is_ok());
    assert!(d.db_execute(&h, "DROP INDEX idx_t", &[]).is_ok());
    assert!(d
        .db_execute(&h, "ALTER TABLE t ADD COLUMN b TEXT", &[])
        .is_ok());
    assert!(d
        .db_execute(&h, "INSERT INTO t (a) VALUES (1)", &[])
        .is_ok());
    assert!(d.db_execute(&h, "ANALYZE t", &[]).is_ok());
    assert!(d.db_execute(&h, "REINDEX t", &[]).is_ok());
    assert!(d.db_execute(&h, "SAVEPOINT sp1", &[]).is_ok());
    assert!(d.db_execute(&h, "RELEASE sp1", &[]).is_ok());
    assert!(d.db_execute(&h, "DROP TABLE t", &[]).is_ok());
    // Commit khi không có transaction ⇒ lỗi **dữ liệu** (không trap, không quyền).
    let e = d.db_execute(&h, "COMMIT", &[]);
    assert!(e.is_err());
    assert!(!e.unwrap_err().starts_with(DAU_QUYEN));
}

#[test]
fn temp_store_bi_ep_memory_va_cache_2mib() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    // Authorizer chặn **chính** các PRAGMA đó trên connection của host (từ chối rõ) —
    // quan sát hiệu lực qua probe connection raw với **đúng** chuỗi `HOST_PRAGMAS`.
    let (ts, trusted, cache) = cau_hinh_temp_store_that();
    assert_eq!(ts, 2, "temp_store phải là MEMORY (2)");
    assert_eq!(trusted, 0, "trusted_schema phải là OFF (0)");
    assert_eq!(cache, -2048, "cache_size phải là -2048 (2 MiB)");
    // Đường SQL người dùng không đụng được (PRAGMA temp_store bị authorizer từ chối).
    let e = d.db_query(&h, "PRAGMA temp_store", &[]);
    assert!(e.is_err());
    let e = d.db_query(&h, "PRAGMA temp_store=FILE", &[]);
    assert!(e.is_err());
    let e = d.db_query(&h, "PRAGMA cache_size = -1", &[]);
    assert!(e.is_err());
}

#[test]
fn cleanup_dong_moi_connection_trong_phien_va_dem() {
    let mut d = Db::moi(false);
    let a = mo_db(&mut d);
    let b = mo_db(&mut d);
    assert_eq!(d.db_cleanup().unwrap(), 2);
    // Handle cũ đã chết.
    let e = d.db_execute(&a, "SELECT 1", &[]).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
    let e2 = d.db_execute(&b, "SELECT 1", &[]).unwrap_err();
    assert!(e2.contains("đã đóng"), "{e2}");
    // Cleanup lần hai không đếm nữa.
    assert_eq!(d.db_cleanup().unwrap(), 0);
}

#[test]
fn file_backed_mac_dinh_mo_duoc_va_ghi_doc_lai() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-back-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("app.db");
    let path = p.to_str().unwrap().to_string();
    {
        let mut d = Db::moi(false);
        let h = d.db_open(&path, true, 5_000).unwrap();
        d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
        d.db_execute(&h, "INSERT INTO t VALUES (1)", &[]).unwrap();
        d.db_close(&h).unwrap();
    }
    {
        let mut d = Db::moi(false);
        let h = d.db_open(&path, true, 5_000).unwrap();
        let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
        assert_eq!(o_int(&rows), 1);
        d.db_cleanup().unwrap();
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn db_da_co_wal_van_mo_duoc() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-wal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("wal.db");
    // Tạo db WAL thật bằng rusqlite trực tiếp (quyết định journal là của người dùng db).
    {
        let c = Connection::open(&p).unwrap();
        c.pragma_update(None, "journal_mode", "WAL").unwrap();
        c.execute("CREATE TABLE t (a INTEGER)", []).unwrap();
        c.execute("INSERT INTO t VALUES (42)", []).unwrap();
        c.close().unwrap();
    }
    // Mở lại qua host: đọc được dữ liệu đã commit.
    let mut d = Db::moi(false);
    let h = d.db_open(p.to_str().unwrap(), true, 5_000).unwrap();
    let (_, rows) = d.db_query(&h, "SELECT a FROM t", &[]).unwrap();
    assert_eq!(o_int(&rows), 42);
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn hai_ket_noi_tranh_khoa_busy_timeout() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-lock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("app.db");
    let path = p.to_str().unwrap().to_string();
    let mut d = Db::moi(false);
    let h1 = d.db_open(&path, true, 300).unwrap();
    let h2 = d.db_open(&path, true, 300).unwrap();
    d.db_execute(&h1, "CREATE TABLE t (a INTEGER)", &[])
        .unwrap();
    d.db_execute(&h1, "BEGIN IMMEDIATE", &[]).unwrap();
    d.db_execute(&h1, "INSERT INTO t VALUES (1)", &[]).unwrap();
    // Kết nối thứ hai chờ ≤ 300 ms rồi `SQLITE_BUSY` ⇒ lỗi, **không** treo vô hạn.
    let t0 = Instant::now();
    let e = d.db_execute(&h2, "INSERT INTO t VALUES (2)", &[]);
    let dt = t0.elapsed();
    assert!(e.is_err(), "ghi tranh chấp phải lỗi");
    assert!(dt < Duration::from_secs(5), "không được treo lâu: {dt:?}");
    d.db_execute(&h1, "ROLLBACK", &[]).unwrap();
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn strict_caps_tu_choi_file_backed_nhung_cho_mem() {
    use crate::quyen::{CapKhai, Quyen};
    let dir = std::env::temp_dir().join(format!("vis-sqlite-strict-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let qk = Quyen {
        moc: vec![crate::quyen::QuyenCap {
            pos: 0,
            kind: "Dir".to_string(),
            root: dir.display().to_string(),
            write: true,
            allow: Vec::new(),
        }],
        bat: true,
        siet: true,
        goc: dir.clone(),
        allow_proc: None,
    };
    let _ = CapKhai {
        name: "d".into(),
        kind: "Dir".into(),
        path: dir.display().to_string(),
        write: true,
        pos: 0,
    };
    let mut trong = Db::moi(false);
    {
        let mut guard = DbGuard {
            trong: &mut trong,
            quyen: &qk,
        };
        let e = guard
            .db_open(dir.join("app.db").to_str().unwrap(), true, 5_000)
            .unwrap_err();
        assert!(e.starts_with(DAU_QUYEN), "phải là lỗi quyền: {e}");
        assert!(e.contains(":memory:"), "thông điệp phải nêu :memory:: {e}");
        assert!(guard.db_open(":memory:", true, 5_000).is_ok());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nan_bi_tu_choi_va_inf_roundtrip() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (r REAL)", &[]).unwrap();
    // NaN ⇒ Err (không NULL, không lưu).
    let e = d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_real(f64::NAN)]);
    let e = e.unwrap_err();
    assert_eq!(
        e, "Real(NaN) — dùng `SqlValue.Null()` nếu muốn giá trị NULL",
        "{e}"
    );
    // ±Inf được phép bind/lưu/đọc lại.
    d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_real(f64::INFINITY)])
        .unwrap();
    d.db_execute(
        &h,
        "INSERT INTO t VALUES (?)",
        &[sv_real(f64::NEG_INFINITY)],
    )
    .unwrap();
    let (_, rows) = d.db_query(&h, "SELECT r FROM t ORDER BY r", &[]).unwrap();
    assert_eq!(rows.len(), 2, "{rows:?}");
    let lay = |row: &Vec<Value>| -> f64 {
        let Value::Enum(_, _, args) = &row[0] else {
            panic!("ô không phải SqlValue: {row:?}")
        };
        let Value::Float(f) = &args[0] else {
            panic!("ô không phải Real: {args:?}")
        };
        *f
    };
    assert_eq!(lay(&rows[0]), f64::NEG_INFINITY);
    assert_eq!(lay(&rows[1]), f64::INFINITY);
}

#[test]
fn param_vuot_1mib_bi_tu_choi_va_dung_limit_thi_ok() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (x TEXT, b BLOB)", &[])
        .unwrap();
    // 1 MiB đúng ⇒ ok.
    let dung = "y".repeat(MAX_PARAM_BYTES);
    d.db_execute(
        &h,
        "INSERT INTO t VALUES (?, ?)",
        &[sv_text(&dung), sv_blob(vec![7; MAX_PARAM_BYTES])],
    )
    .unwrap();
    // Vượt 1 MiB ⇒ Err (không cắt im lặng).
    let qua = "y".repeat(MAX_PARAM_BYTES + 1);
    let e = d
        .db_execute(
            &h,
            "INSERT INTO t VALUES (?, ?)",
            &[sv_text(&qua), sv_blob(vec![])],
        )
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");
    let qua_blob = vec![7u8; MAX_PARAM_BYTES + 1];
    let e = d
        .db_execute(
            &h,
            "INSERT INTO t VALUES (?, ?)",
            &[sv_text("ok"), sv_blob(qua_blob)],
        )
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");
}

#[test]
fn params_tong_vuot_16mib_bi_tu_choi_va_dung_limit_thi_ok() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let mi = "x".repeat(MAX_PARAM_BYTES);
    // 16 × 1_048_576 = đúng 16 MiB ⇒ qua được vòng byte-check (không trả hàng to).
    let params: Vec<Value> = (0..16).map(|_| sv_text(&mi)).collect();
    let dieu_kien = (0..16)
        .map(|i| format!("?{} IS NOT NULL", i + 1))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!("SELECT 1 WHERE {dieu_kien}");
    let r = d.db_query(&h, &sql, &params);
    assert!(r.is_ok(), "16 param đúng 16 MiB phải ok: {:?}", r.err());
    // 17 × 1_048_576 > 16 MiB ⇒ Err.
    let params17: Vec<Value> = (0..17).map(|_| sv_text(&mi)).collect();
    let sql17 = format!(
        "SELECT {}",
        (0..17).map(|_| "?").collect::<Vec<_>>().join(",")
    );
    let e = d.db_query(&h, &sql17, &params17).unwrap_err();
    assert!(e.contains("16777216"), "{e}");
}

#[test]
fn execute_tra_hang_phat_hien_truoc_step_khong_side_effect() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (id INTEGER PRIMARY KEY)", &[])
        .unwrap();
    let e = d
        .db_execute(&h, "INSERT INTO t VALUES (1) RETURNING id", &[])
        .unwrap_err();
    assert_eq!(
        e, "câu lệnh trả hàng — dùng `sqlite.query` thay vì `sqlite.execute`",
        "{e}"
    );
    // Bảng phải rỗng — INSERT chưa từng chạy.
    let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
    assert_eq!(o_int(&rows), 0, "không được insert khi bị từ chối");
    // SELECT thường cũng bị chặn như vậy.
    let e = d.db_execute(&h, "SELECT 1", &[]).unwrap_err();
    assert_eq!(
        e, "câu lệnh trả hàng — dùng `sqlite.query` thay vì `sqlite.execute`",
        "{e}"
    );
}

#[test]
fn thoi_gian_chay_nho_bi_ngat_nhanh_khong_phai_doi_5s() {
    let mut d = Db::moi_voi_query_ms(false, 100);
    let h = mo_db(&mut d);
    let t0 = Instant::now();
    let e = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 1000000000) SELECT count(*) FROM c",
            &[],
        )
        .unwrap_err();
    let dt = t0.elapsed();
    assert!(e.contains("OperationInterrupted"), "{e}");
    assert!(dt < Duration::from_secs(5), "quá lâu: {dt:?}");
    // Handler được reset — câu sau chạy bình thường.
    let (_, rows) = d.db_query(&h, "SELECT 1", &[]).unwrap();
    assert_eq!(o_int(&rows), 1);
}

#[test]
fn sqlite_debug_in_thong_diep_tho_ro_fragment_chi_khi_bat() {
    // Câu lỗi có fragment nhạy cảm trong SQL.
    let sql = "INSERT INTO khong_co_bang VALUES ('O'Clock; SELECT secret')";
    // Mặc định: thông điệp **không** chứa fragment.
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_execute(&h, sql, &[]).unwrap_err();
    assert!(
        !e.contains("O'Clock"),
        "thông điệp mặc định lộ fragment: {e}"
    );
    assert!(
        !e.contains("secret"),
        "thông điệp mặc định lộ fragment: {e}"
    );
    assert!(e.contains("mã SQLite"), "phải có mã SQLite: {e}");
    // `--sqlite-debug`: thông điệp thô được hiển thị (user chủ động bật).
    let mut d2 = Db::moi(false);
    d2.dat_sqlite_debug(true);
    let h2 = mo_db(&mut d2);
    let e2 = d2.db_execute(&h2, sql, &[]).unwrap_err();
    assert!(
        e2.contains("O'Clock") || e2.contains("secret"),
        "--sqlite-debug phải thấy nội dung thô: {e2}"
    );
}

#[test]
fn loi_offset_duoc_giu_tu_loi_prepare() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_execute(&h, "SELEC 1", &[]).unwrap_err();
    assert!(e.contains("mã SQLite"), "{e}");
    assert!(e.contains("offset"), "phải giữ offset: {e}");
    assert!(!e.contains("SELEC"), "không được lộ SQL text: {e}");
}

fn chinh_sach_tai(action: AuthAction<'_>) -> Authorization {
    let ctx = AuthContext {
        action,
        database_name: None,
        accessor: None,
    };
    chinh_sach(ctx)
}

#[test]
fn chinh_sach_action_code_ma_tran() {
    use rusqlite::hooks::TransactionOperation::*;
    use AuthAction::*;
    let allow = [
        CreateTable { table_name: "t" },
        DropTable { table_name: "t" },
        AlterTable {
            database_name: "main",
            table_name: "t",
        },
        CreateIndex {
            index_name: "i",
            table_name: "t",
        },
        DropIndex {
            index_name: "i",
            table_name: "t",
        },
        Insert { table_name: "t" },
        Update {
            table_name: "t",
            column_name: "a",
        },
        Delete { table_name: "t" },
        Select,
        Read {
            table_name: "t",
            column_name: "a",
        },
        Recursive,
        Transaction { operation: Begin },
        Transaction { operation: Release },
        Transaction {
            operation: Rollback,
        },
        // `COMMIT` từ SQLite map về `Unknown` — vẫn phải Allow.
        Transaction { operation: Unknown },
        Savepoint {
            operation: Begin,
            savepoint_name: "sp",
        },
        Savepoint {
            operation: Release,
            savepoint_name: "sp",
        },
        Reindex { index_name: "i" },
        Analyze { table_name: "t" },
        CreateView { view_name: "v" },
        DropView { view_name: "v" },
        CreateTrigger {
            trigger_name: "tg",
            table_name: "t",
        },
        DropTrigger {
            trigger_name: "tg",
            table_name: "t",
        },
        CreateTempTable { table_name: "t" },
        DropTempTable { table_name: "t" },
        Function {
            function_name: "count",
        },
        Function {
            function_name: "randomblob",
        },
        Function {
            function_name: "sqlite_version",
        },
        Pragma {
            pragma_name: "table_info",
            pragma_value: None,
        },
        Pragma {
            pragma_name: "user_version",
            pragma_value: None,
        },
    ];
    for a in allow {
        assert_eq!(
            chinh_sach_tai(a),
            Authorization::Allow,
            "phải cho phép: {a:?}"
        );
    }
    let deny = [
        Attach {
            filename: "/tmp/x.db",
        },
        Detach { database_name: "x" },
        CreateVtable {
            table_name: "t",
            module_name: "fts5",
        },
        DropVtable {
            table_name: "t",
            module_name: "fts5",
        },
        Function {
            function_name: "load_extension",
        },
        Function {
            function_name: "sqlite_load_extension",
        },
        Function {
            function_name: "LOAD_EXTENSION",
        },
        Pragma {
            pragma_name: "user_version",
            pragma_value: Some("42"),
        },
        Pragma {
            pragma_name: "user_version",
            pragma_value: Some(""),
        },
        Pragma {
            pragma_name: "temp_store",
            pragma_value: Some("FILE"),
        },
        Pragma {
            pragma_name: "journal_mode",
            pragma_value: Some("WAL"),
        },
        Pragma {
            pragma_name: "synchronous",
            pragma_value: Some("OFF"),
        },
        Pragma {
            pragma_name: "cache_size",
            pragma_value: Some("-1"),
        },
        Pragma {
            pragma_name: "temp_store",
            pragma_value: None,
        },
        AuthAction::Unknown {
            code: 999,
            arg1: None,
            arg2: None,
        },
    ];
    for a in deny {
        assert_eq!(
            chinh_sach_tai(a),
            Authorization::Deny,
            "phải từ chối: {a:?}"
        );
    }
}

#[test]
fn chinh_sach_ham_load_extension_bi_cam() {
    for f in ["load_extension", "sqlite_load_extension", "LOAD_EXTENSION"] {
        assert_eq!(
            chinh_sach_tai(AuthAction::Function { function_name: f }),
            Authorization::Deny,
            "{f}"
        );
    }
}

/// **F-A (2026-11-19)** — đường dẫn cắt ngang codepoint đa byte ở byte 120 không được panic.
#[test]
fn mo_ta_path_utf8_split_tai_byte_120_khong_panic() {
    // 118 'a' (byte 0..117) + 'é' (byte 118..119) + tiếp byte 120 là giữa dấu nhọn
    let path = format!("{}étail.db", "a".repeat(118));
    let d = super::mo_ta(&path);
    assert!(d.ends_with('…'), "kỳ vọng giữ tới ranh giới byte + dấu …");
    // đường open (tạo file trong /tmp) cũng không được panic
    let d_tmp = std::env::temp_dir().join(format!("vis-f-a-{}.db", std::process::id()));
    let mut db = super::Db::moi_voi_query_ms(false, 5_000);
    let _ = db.db_open(&d_tmp.to_string_lossy(), true, 5_000);
    let _ = std::fs::remove_file(&d_tmp);
}

// ===========================================================================
// VD-258 — cap bộ nhớ SQLite (ô/overhead/tên cột), pre-materialization,
// schema không-UTF-8 và cap tổng handle.
// ===========================================================================

/// `Db` với trần tuỳ ý — test ép cap thấp, không cần dữ liệu khổng lồ.
fn db_gioi_han(f: impl FnOnce(&mut GioiHan)) -> Db {
    let mut d = Db::moi(false);
    f(&mut d.gioi_han);
    d
}

#[test]
fn cap_o_chan_bang_rong_nhieu_cot() {
    let mut d = db_gioi_han(|g| g.cells = 10);
    let h = mo_db(&mut d);
    // 3 hàng × 4 cột = 12 ô > 10 ⇒ Err (không trả dữ liệu cắt).
    let e = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 3) \
             SELECT x, x, x, x FROM c",
            &[],
        )
        .unwrap_err();
    assert!(e.contains("10") && e.contains("ô"), "{e}");
    // 2 hàng × 4 cột = 8 ô ⇒ vẫn qua.
    let (_, rows) = d
        .db_query(
            &h,
            "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 2) \
             SELECT x, x, x, x FROM c",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn overhead_o_null_tinh_vao_tong_byte() {
    // Một hàng 2 ô NULL: payload 0 nhưng overhead 2×64 = 128 > 100 ⇒ phải bị chặn.
    let mut d = db_gioi_han(|g| g.total_bytes = 100);
    let h = mo_db(&mut d);
    let e = d.db_query(&h, "SELECT NULL, NULL", &[]).unwrap_err();
    assert!(e.contains("100") && e.contains("overhead"), "{e}");
    // 1 ô NULL: 64 ≤ 100 ⇒ qua (và chứng minh cap không chặn oan payload nhỏ).
    let (_, rows) = d.db_query(&h, "SELECT NULL", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn tran_cung_chan_zeroblob_truoc_khi_cap_phat() {
    let mut d = db_gioi_han(|g| {
        g.row_bytes = 4 << 10;
        g.hard_cell_bytes = 8 << 10;
    });
    let h = mo_db(&mut d);
    // 1 MiB > trần cứng 8 KiB ⇒ SQLite báo TOOBIG, không materialize 1 MiB; host map thành
    // lỗi cap hàng (không trả blob cắt).
    let t0 = Instant::now();
    let e = d.db_query(&h, "SELECT zeroblob(1048576)", &[]).unwrap_err();
    assert!(e.contains("4096"), "{e}");
    assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    // Trong trần cứng và trần hàng ⇒ đọc được.
    let (_, rows) = d.db_query(&h, "SELECT zeroblob(4096)", &[]).unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn zeroblob_512m_bi_chan_voi_rss_huu_han() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    #[cfg(target_os = "linux")]
    let truoc = vm_hwm_kb();
    let t0 = Instant::now();
    let e = d
        .db_query(&h, "SELECT zeroblob(536870912)", &[])
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");
    assert!(t0.elapsed() < Duration::from_secs(2), "{:?}", t0.elapsed());
    // Trước đây `zeroblob(512 MiB)` materialize ~519 MB. Với trần cứng, mức tăng HWM phải nhỏ.
    #[cfg(target_os = "linux")]
    if let (Some(a), Some(b)) = (truoc, vm_hwm_kb()) {
        let tang_kb = b.saturating_sub(a);
        assert!(
            tang_kb < 256 * 1024,
            "RSS tăng {tang_kb} KiB — trần cứng không chặn materialize?"
        );
    }
}

#[cfg(target_os = "linux")]
fn vm_hwm_kb() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    for d in s.lines() {
        if let Some(rest) = d.strip_prefix("VmHWM:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

#[test]
fn tong_handle_cap_ke_ca_tombstone() {
    let mut d = db_gioi_han(|g| g.handles = 4);
    for _ in 0..4 {
        let h = mo_db(&mut d);
        d.db_close(&h).unwrap();
    }
    let e = d.db_open(":memory:", true, 5_000).unwrap_err();
    assert!(e.contains('4') && e.contains("handle"), "{e}");
}

/// Ghi tệp SQLite, thay **nguyên văn** chuỗi `tim` trong byte schema bằng `thay` (cùng độ dài,
/// byte không-UTF-8). Dùng để tái hiện schema ngoài bị hỏng encoding — không qua host.
fn tao_db_schema_hong(
    dir: &std::path::Path,
    ten: &str,
    sql: &str,
    tim: &str,
    thay: &[u8],
) -> String {
    assert_eq!(tim.len(), thay.len(), "thay phải cùng độ dài byte");
    let p = dir.join(ten);
    {
        let c = Connection::open(&p).unwrap();
        c.execute_batch(sql).unwrap();
        c.close().unwrap();
    }
    let mut b = std::fs::read(&p).unwrap();
    let cu = tim.as_bytes();
    let mut n = 0usize;
    let mut i = 0usize;
    while i + cu.len() <= b.len() {
        if &b[i..i + cu.len()] == cu {
            b[i..i + cu.len()].copy_from_slice(thay);
            n += 1;
            i += cu.len();
        } else {
            i += 1;
        }
    }
    assert!(n > 0, "không tìm thấy {tim:?} trong schema để thay");
    std::fs::write(&p, &b).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn schema_khong_utf8_tra_loi_du_lieu_khong_panic() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-schema-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // "cột" = 5 byte UTF-8; thay bằng 5 byte 0xFF (không-UTF-8, không NUL).
    let path = tao_db_schema_hong(
        &dir,
        "bad.db",
        "CREATE TABLE t (\"cột\" TEXT); INSERT INTO t VALUES ('x');",
        "cột",
        &[0xff; 5],
    );
    let mut d = Db::moi(false);
    let h = d.db_open(&path, false, 5_000).expect("mở được tệp");
    let e = d.db_query(&h, "SELECT * FROM t", &[]).unwrap_err();
    assert!(
        e.contains("UTF-8"),
        "phải là lỗi schema không-UTF-8 (không panic), được: {e}"
    );
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ten_cot_khong_utf8_tra_loi_du_lieu_khong_panic() {
    let dir = std::env::temp_dir().join(format!("vis-sqlite-cot-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // View alias không-UTF-8: authorizer có thể chỉ thấy cột gốc `a` (hợp lệ); đường
    // `Statement::column_name` của rusqlite sẽ panic nếu host dùng nó. Host phải trả Err.
    let path = tao_db_schema_hong(
        &dir,
        "bad-view.db",
        "CREATE TABLE t (a TEXT); CREATE VIEW v AS SELECT a AS \"cột\" FROM t;",
        "cột",
        &[0xff; 5],
    );
    let mut d = Db::moi(false);
    let h = d.db_open(&path, false, 5_000).expect("mở được tệp");
    let r = d.db_query(&h, "SELECT * FROM v", &[]);
    let e = r.expect_err("tên cột không-UTF-8 phải là lỗi dữ liệu");
    assert!(
        e.contains("UTF-8") || e.contains("quyền (authorizer)") || e.contains("mã SQLite"),
        "phải là lỗi dữ liệu rõ, được: {e}"
    );
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}
