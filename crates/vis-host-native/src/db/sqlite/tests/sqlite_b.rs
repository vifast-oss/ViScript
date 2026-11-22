// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-296c — nhóm ca sqlite (sqlite_b); tách nguyên văn, không đổi tên ca.

use super::sqlite_c::db_gioi_han;
use super::*;

/// **VD-266** — `query_one` chỉ đọc **y như `query`**, không side effect.
#[test]
fn query_one_tu_choi_ghi_khong_side_effect() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    let e = d
        .db_query_one(&h, "INSERT INTO t (a) VALUES (1)", &[])
        .unwrap_err();
    assert!(e.contains("chỉ đọc"), "{e}");
    let e = d.db_query_one(&h, "BEGIN", &[]).unwrap_err();
    assert!(e.contains("không trả hàng") || e.contains("chỉ đọc"), "{e}");
    let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
    assert_eq!(o_int(&rows), 0, "ghi qua query_one không được chạy");
}

#[test]
fn query_one_kiem_params_va_handle_dong() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d.db_query_one(&h, "SELECT ?", &[]).unwrap_err();
    assert!(e.contains("1 tham số"), "{e}");
    let e = d.db_query_one(&h, "SELECT 1", &[sv_int(1)]).unwrap_err();
    assert!(e.contains("0 tham số"), "{e}");
    d.db_close(&h).unwrap();
    let e = d.db_query_one(&h, "SELECT 1", &[]).unwrap_err();
    assert!(e.contains("đã đóng"), "{e}");
}

/// **VD-266** — `last_insert_rowid`/`changes` là metadata của connection, không đổi dữ liệu.
/// **VD-266** — `query_one` dùng **cùng** cap hàng/cap ô với `query`, không nới chính sách.
#[test]
fn query_one_cung_cap_hang_va_cap_o() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let e = d
        .db_query_one(&h, "SELECT randomblob(1048577)", &[])
        .unwrap_err();
    assert!(e.contains("1048576"), "{e}");

    let mut d2 = db_gioi_han(|g| g.cells = 1);
    let h2 = mo_db(&mut d2);
    let e = d2.db_query_one(&h2, "SELECT 1, 2", &[]).unwrap_err();
    assert!(e.contains("ô"), "{e}");
}

#[test]
fn last_insert_rowid_va_changes_metadata() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER, b TEXT)", &[])
        .unwrap();
    assert_eq!(d.db_last_insert_rowid(&h).unwrap(), 0);
    d.db_execute(&h, "INSERT INTO t (a, b) VALUES (1, 'x')", &[])
        .unwrap();
    assert_eq!(d.db_last_insert_rowid(&h).unwrap(), 1);
    d.db_execute(&h, "INSERT INTO t (a, b) VALUES (2, 'y')", &[])
        .unwrap();
    assert_eq!(d.db_last_insert_rowid(&h).unwrap(), 2);
    d.db_execute(&h, "UPDATE t SET b = 'z' WHERE a = ?", &[sv_int(1)])
        .unwrap();
    assert_eq!(d.db_changes(&h).unwrap(), 1);
    d.db_execute(&h, "UPDATE t SET b = 'z' WHERE a = ?", &[sv_int(99)])
        .unwrap();
    assert_eq!(d.db_changes(&h).unwrap(), 0);
    d.db_close(&h).unwrap();
    assert!(d.db_last_insert_rowid(&h).is_err());
    assert!(d.db_changes(&h).is_err());
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
        "PRAGMA application_id=1",
        "PRAGMA application_id = 42",
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
    // table_info và đọc user_version/schema_version/data_version/application_id/journal_mode/
    // database_list thì được phép.
    assert!(d.db_query(&h, "PRAGMA table_info(t)", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA user_version", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA schema_version", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA data_version", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA application_id", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA journal_mode", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA database_list", &[]).is_ok());
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

/// **VD-242 §3.6 / VD-259** — `query` **chỉ đọc**: phân loại sau prepare, trước step; câu ghi
/// hoặc câu không trả hàng bị từ chối, không side effect. `PRAGMA table_info` (trả hàng) đọc
/// được qua `query`, bị `execute` từ chối; PRAGMA ngoài allow-list bị authorizer chặn.
#[test]
fn query_chi_doc_tu_choi_lenh_ghi_va_lenh_khong_tra_hang() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    for sql in [
        "INSERT INTO t VALUES (1)",
        "INSERT INTO t VALUES (2) RETURNING a",
        "CREATE TABLE t2 (x INTEGER)",
        "UPDATE t SET a = 1",
        "DELETE FROM t",
        "WITH x AS (SELECT 1) INSERT INTO t SELECT * FROM x",
        "DROP TABLE t",
    ] {
        let e = d.db_query(&h, sql, &[]).unwrap_err();
        assert!(
            e.contains("chỉ đọc") && e.contains("sqlite.execute"),
            "{sql}: {e}"
        );
    }
    // Không side effect nào chạy: bảng rỗng, t2/t không bị đổi.
    let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
    assert_eq!(o_int(&rows), 0);
    assert!(
        d.db_query(&h, "SELECT * FROM t2", &[]).is_err(),
        "CREATE TABLE không được chạy qua query"
    );
    assert!(
        d.db_query(&h, "SELECT * FROM t", &[]).is_ok(),
        "bảng t vẫn còn"
    );
    // Câu không trả hàng (kể cả transaction) phải dùng `execute`.
    for sql in ["BEGIN", "COMMIT", "SAVEPOINT sp", "RELEASE sp"] {
        let e = d.db_query(&h, sql, &[]).unwrap_err();
        assert!(e.contains("không trả hàng"), "{sql}: {e}");
    }
    // `PRAGMA table_info` trả hàng ⇒ query được; execute từ chối trước step.
    assert!(d.db_query(&h, "PRAGMA table_info(t)", &[]).is_ok());
    let e = d.db_execute(&h, "PRAGMA table_info(t)", &[]).unwrap_err();
    assert_eq!(
        e, "câu lệnh trả hàng — dùng `sqlite.query` thay vì `sqlite.execute`",
        "{e}"
    );
    // Chỉ `table_info` + **đọc** `user_version`/`schema_version`/`data_version`/`application_id`/
    // `journal_mode`/`database_list` được phép.
    assert!(d.db_query(&h, "PRAGMA user_version", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA schema_version", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA data_version", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA application_id", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA journal_mode", &[]).is_ok());
    assert!(d.db_query(&h, "PRAGMA database_list", &[]).is_ok());
    for sql in [
        "PRAGMA index_list(t)",
        "PRAGMA table_xinfo(t)",
        "PRAGMA user_version = 1",
        "PRAGMA schema_version = 1",
        "PRAGMA data_version = 1",
        "PRAGMA application_id = 1",
        "PRAGMA journal_mode = WAL",
        "PRAGMA journal_mode = DELETE",
        "PRAGMA database_list = 1",
    ] {
        let e = d.db_query(&h, sql, &[]).unwrap_err();
        assert!(e.contains("quyền (authorizer)"), "{sql}: {e}");
    }
}
