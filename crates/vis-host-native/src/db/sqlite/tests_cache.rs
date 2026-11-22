// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-268** — cache statement chuẩn bị sẵn (`Connection::prepare_cached`) trên host `std.sqlite`.
//!
//! Hợp đồng khoá ở đây:
//! - Cùng một chuỗi SQL lặp lại **không prepare lại** (authorizer đếm `SELECT`: 21 lời gọi cùng
//!   SQL chỉ 1 lần prepare).
//! - `schema_version`/`temp.schema_version` đổi ⇒ cache **bị xả** trước lời gọi kế tiếp, nên
//!   metadata cột (`column_count`/`column_name` đọc **trước** step) không bao giờ là của schema
//!   cũ; đúng cho DDL trên cùng handle, trên handle khác cùng tệp, và bảng TEMP.
//! - Cache **không** đổi kết quả/lỗi: cap hàng/ô/cột/byte, `SQLITE_LIMIT_LENGTH`, chính sách
//!   `query` chỉ đọc, authorizer (ghi qua `query`, ATTACH, ghi PRAGMA) vẫn áp **mỗi** lời gọi.
//! - Statement dùng lại được trong `BEGIN`/`COMMIT`/`ROLLBACK`; offset lỗi prepare giữ đúng
//!   vị trí byte khi SQL có whitespace đầu (cache dùng `sql.trim()`).
//!
//! **Red-before (2026-11-21)**: khi tạm bỏ cache (`conn.prepare(sql)` mỗi lời gọi),
//! `cache_tai_su_dung_khong_prepare_lai` đỏ (21 lần prepare thay vì 1). Khi thiếu xả cache theo
//! `schema_version`, `cache_xa_khi_schema_doi_cung_handle` đỏ vì `SELECT *` vẫn báo số cột cũ
//! trước step; `cache_xa_khi_temp_schema_doi` đỏ nếu chỉ theo dõi main schema.

use super::*;
use rusqlite::hooks::AuthContext;

fn sv_int(n: i64) -> Value {
    Value::Enum("SqlValue".into(), "Integer".into(), vec![Value::Int(n)])
}

fn mo_db(d: &mut Db) -> String {
    d.db_open(":memory:", true, 5_000).expect("open :memory:")
}

fn o_int(rows: &[Vec<Value>]) -> i64 {
    assert_eq!(rows.len(), 1, "phải có đúng 1 hàng: {rows:?}");
    let Value::Enum(e, v, args) = &rows[0][0] else {
        panic!("ô không phải enum: {rows:?}")
    };
    assert_eq!((e.as_str(), v.as_str()), ("SqlValue", "Integer"));
    let Value::Int(n) = &args[0] else {
        panic!("ô không phải int")
    };
    *n
}

thread_local! {
    /// Số lần authorizer thấy `SELECT` — tăng **chỉ khi prepare** (cache hit không gọi
    /// authorizer). `thread_local` để các test chạy song song không giẫm nhau.
    static DEM_PREPARE_T: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Authorizer đếm + **giữ nguyên** chính sách host (uỷ quyền [`chinh_sach`]). Đếm
/// `AuthAction::Select` — mỗi **lần prepare** một câu SELECT gọi đúng một lần, còn `READ` có thể
/// gọi nhiều lần cho cùng một cột (SELECT + WHERE). Cài **trước** lần prepare đầu tiên cần đo:
/// thay authorizer lần nữa sẽ làm SQLite expire mọi statement đã prepare
/// (`sqlite3_set_authorizer` → `sqlite3ExpirePreparedStatements`), nên không được cài lại giữa
/// chừng khi đang đo cache hit.
fn dem_prepare(ctx: AuthContext<'_>) -> Authorization {
    if matches!(&ctx.action, AuthAction::Select) {
        DEM_PREPARE_T.with(|c| c.set(c.get() + 1));
    }
    chinh_sach(ctx)
}

fn dat_authorizer(d: &mut Db, h: &str, f: fn(AuthContext<'_>) -> Authorization) {
    let conn = d
        .bang
        .get(h)
        .and_then(|m| m.conn.as_ref())
        .expect("connection đang mở");
    conn.authorizer(Some(f));
}

fn dat_lai_dem() {
    DEM_PREPARE_T.with(|c| c.set(0));
}

fn dem_hien_tai() -> usize {
    DEM_PREPARE_T.with(|c| c.get())
}

/// Cùng chuỗi SQL lặp lại ⇒ **không prepare lại**. Sau lần chạy đầu, cài authorizer từ chối đọc
/// bảng `t`: lời gọi y hệt phải thành công (cache hit), SQL **mới** phải bị từ chối (chứng minh
/// authorizer đang có hiệu lực, không phải test vô nghĩa).
#[test]
fn cache_tai_su_dung_khong_prepare_lai() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    d.db_execute(&h, "INSERT INTO t VALUES (1)", &[]).unwrap();
    // Cài **trước** khi đo: prepare xong mới cài sẽ expire statement (xem [`dem_prepare`]).
    dat_authorizer(&mut d, &h, dem_prepare);
    dat_lai_dem();

    let (cols, rows) = d.db_query(&h, "SELECT a FROM t", &[]).unwrap();
    assert_eq!(cols, vec!["a".to_string()]);
    assert_eq!(o_int(&rows), 1);
    assert_eq!(
        dem_hien_tai(),
        1,
        "lần đầu phải prepare (authorizer thấy SELECT)"
    );
    for _ in 0..20 {
        let (c, r) = d.db_query(&h, "SELECT a FROM t", &[]).unwrap();
        assert_eq!((c, o_int(&r)), (vec!["a".to_string()], 1));
    }
    assert_eq!(
        dem_hien_tai(),
        1,
        "21 lần cùng SQL chỉ được prepare **một** lần — cache hit không gọi authorizer"
    );
    // SQL khác ⇒ prepare mới thật sự (đếm tăng), kết quả vẫn đúng.
    let (_, rows) = d.db_query(&h, "SELECT a FROM t WHERE a = 1", &[]).unwrap();
    assert_eq!(o_int(&rows), 1);
    assert_eq!(dem_hien_tai(), 2, "SQL mới phải prepare");
    dat_authorizer(&mut d, &h, chinh_sach);
}

/// `ALTER TABLE … ADD COLUMN` trên **cùng handle** ⇒ `SELECT *` lặp lại phải trả **đủ cột mới**.
/// Không xả cache, `column_count()` trước step vẫn là 1 (metadata cũ) và cột `b` biến mất khỏi
/// kết quả dù dữ liệu đúng.
#[test]
fn cache_xa_khi_schema_doi_cung_handle() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    d.db_execute(&h, "INSERT INTO t VALUES (1)", &[]).unwrap();
    let (truoc, rows) = d.db_query(&h, "SELECT * FROM t", &[]).unwrap();
    assert_eq!(truoc, vec!["a".to_string()]);
    assert_eq!(o_int(&rows), 1);

    d.db_execute(&h, "ALTER TABLE t ADD COLUMN b TEXT", &[])
        .unwrap();
    let (sau, rows) = d
        .db_query(&h, "SELECT * FROM t", &[])
        .expect("query sau DDL");
    assert_eq!(
        sau,
        vec!["a".to_string(), "b".to_string()],
        "cache phải bị xả khi schema_version đổi — không trả metadata cột cũ"
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
    assert!(
        matches!(&rows[0][1], Value::Enum(e, v, _) if e == "SqlValue" && v == "Null"),
        "cột mới NULL: {:?}",
        rows[0][1]
    );
    // Lưu ý: chỉ dựa vào SQLite tự expire statement là **không đủ** — nó chỉ recompile ở
    // `sqlite3_step`, sau khi host đã đọc `column_count`/`column_name` (xem assertion trên).
}

/// DDL từ **handle khác trên cùng tệp** cũng phải làm handle này thấy cột mới (invalidated bằng
/// `PRAGMA schema_version`, không phải bằng state nội bộ của một handle).
#[test]
fn cache_xa_khi_schema_doi_tu_handle_khac() {
    let dir = std::env::temp_dir().join(format!(
        "vis-vd268-cache-schema-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("chung.db");
    let path = path.to_str().expect("đường dẫn UTF-8").to_string();

    let mut d = Db::moi(false);
    let h1 = d.db_open(&path, true, 5_000).expect("mở h1");
    d.db_execute(&h1, "CREATE TABLE t (a INTEGER)", &[])
        .unwrap();
    d.db_execute(&h1, "INSERT INTO t VALUES (1)", &[]).unwrap();
    let h2 = d.db_open(&path, false, 5_000).expect("mở h2");
    let (truoc, _) = d.db_query(&h2, "SELECT * FROM t", &[]).unwrap();
    assert_eq!(truoc, vec!["a".to_string()]);

    d.db_execute(&h1, "ALTER TABLE t ADD COLUMN b TEXT", &[])
        .expect("DDL qua h1");
    let (sau, rows) = d
        .db_query(&h2, "SELECT * FROM t", &[])
        .expect("h2 query sau DDL của h1");
    assert_eq!(
        sau,
        vec!["a".to_string(), "b".to_string()],
        "h2 phải thấy schema mới, không dùng cache cũ"
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
    d.db_cleanup().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

/// Statement cache dùng lại được trong `BEGIN`/`COMMIT`/`ROLLBACK` (cùng SQL INSERT/SELECT).
#[test]
fn cache_trong_giao_dich_commit_rollback() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    let dem = |d: &mut Db| {
        let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
        o_int(&rows)
    };

    d.db_execute(&h, "BEGIN", &[]).unwrap();
    d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_int(1)])
        .unwrap();
    d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_int(2)])
        .unwrap();
    assert_eq!(dem(&mut d), 2);
    d.db_execute(&h, "ROLLBACK", &[]).unwrap();
    assert_eq!(dem(&mut d), 0, "rollback phải huỷ cả hai insert");

    d.db_execute(&h, "BEGIN", &[]).unwrap();
    for i in 3..5 {
        d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_int(i)])
            .unwrap();
    }
    d.db_execute(&h, "COMMIT", &[]).unwrap();
    assert_eq!(dem(&mut d), 2, "commit giữ hai insert");
    d.db_execute(&h, "BEGIN", &[]).unwrap();
    assert!(d.db_in_transaction(&h).unwrap());
    d.db_execute(&h, "ROLLBACK", &[]).unwrap();
    assert!(!d.db_in_transaction(&h).unwrap());
}

/// Cap hàng và `SQLITE_LIMIT_LENGTH` vẫn áp **mỗi** lời gọi trên statement lấy từ cache.
#[test]
fn cache_van_ap_cap_hang_va_tran_o() {
    let mut d = Db::moi(false);
    d.gioi_han.rows = 2;
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    for i in 0..3 {
        d.db_execute(&h, "INSERT INTO t VALUES (?)", &[sv_int(i)])
            .unwrap();
    }
    for lan in 0..2 {
        let e = d
            .db_query(&h, "SELECT a FROM t", &[])
            .expect_err("phải vượt cap 2 hàng");
        assert!(e.contains("vượt giới hạn 2 hàng"), "lần {lan}: {e}");
    }
    // 2 MB > trần đọc 1 MiB (`SQLITE_LIMIT_LENGTH`) — lỗi ở **mỗi** lần, không cache kết quả.
    for lan in 0..2 {
        let e = d.db_query(&h, "SELECT zeroblob(2000000)", &[]).unwrap_err();
        assert!(
            e.contains("vượt giới hạn 1048576 byte") || e.contains("vượt giới hạn"),
            "lần {lan}: {e}"
        );
    }
    // Cùng SQL nhưng **prepare lần đầu** qua `execute` (trần ghi 17 MiB) rồi bị từ chối vì trả
    // hàng; lần `query` sau lấy statement từ cache vẫn phải chịu trần đọc 1 MiB (limit áp lúc
    // chạy, không "nướng" vào statement).
    let _ = d
        .db_execute(&h, "SELECT zeroblob(2000001)", &[])
        .expect_err("execute câu trả hàng phải bị từ chối");
    let e = d.db_query(&h, "SELECT zeroblob(2000001)", &[]).unwrap_err();
    assert!(
        e.contains("vượt giới hạn"),
        "statement prepare dưới trần ghi vẫn phải bị trần đọc chặn: {e}"
    );
}

/// Chính sách **không** bị cache làm mất hiệu lực: ghi qua `query`, `BEGIN` qua `query`,
/// `SELECT` qua `execute`, ATTACH, ghi `PRAGMA` — đều `Err` ở **cả hai** lần lặp.
#[test]
fn cache_khong_bo_qua_chinh_sach() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TABLE t (a INTEGER)", &[]).unwrap();
    for lan in 0..2 {
        let e = d.db_query(&h, "INSERT INTO t VALUES (1)", &[]).unwrap_err();
        assert!(e.contains("chỉ đọc"), "lần {lan}: {e}");
        assert!(
            d.db_query(&h, "BEGIN", &[]).is_err(),
            "lần {lan}: BEGIN qua query phải bị chặn"
        );
        assert!(
            d.db_execute(&h, "SELECT 1", &[]).is_err(),
            "lần {lan}: câu trả hàng qua execute phải bị chặn"
        );
        assert!(
            d.db_execute(&h, "ATTACH DATABASE ':memory:' AS x", &[])
                .is_err(),
            "lần {lan}: ATTACH phải bị chặn"
        );
        assert!(
            d.db_execute(&h, "PRAGMA user_version = 7", &[]).is_err(),
            "lần {lan}: ghi PRAGMA phải bị chặn"
        );
    }
    let (_, rows) = d.db_query(&h, "SELECT count(*) FROM t", &[]).unwrap();
    assert_eq!(o_int(&rows), 0, "không insert nào được chạy");
}

/// Offset lỗi prepare giữ đúng vị trí byte khi SQL có whitespace đầu (cache dùng `sql.trim()`).
#[test]
fn cache_giu_offset_loi_prepare_voi_ws_dau() {
    fn offset(s: &str) -> i64 {
        let sau = s.split("offset ").nth(1).expect("phải có offset");
        let so: String = sau.chars().take_while(|c| c.is_ascii_digit()).collect();
        so.parse().expect("offset là số")
    }
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let a = d.db_query(&h, "SELEC 1", &[]).unwrap_err();
    let b = d.db_query(&h, "   SELEC 1", &[]).unwrap_err();
    assert!(b.contains("offset "), "lỗi vẫn phải giữ offset: {b}");
    assert_eq!(
        offset(&b),
        offset(&a) + 3,
        "offset phải tính trên SQL người dùng (cộng 3 byte whitespace bị cache trim):\n{a}\n{b}"
    );
}

/// SQL chỉ gồm whitespace **không-ASCII** (U+00A0) không được cache-trim thành rỗng — giữ đúng
/// hành vi `prepare` nguyên văn (Err dữ liệu), không panic/không đổi thành công.
#[test]
fn cache_khong_trim_unicode_whitespace_thanh_rong() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    for _ in 0..2 {
        let e = d.db_query(&h, "\u{00A0}", &[]).unwrap_err();
        assert!(!e.is_empty(), "phải là lỗi dữ liệu");
    }
}

/// DDL trên bảng **TEMP** không làm đổi `PRAGMA schema_version` của main — vẫn phải xả cache
/// nhờ `PRAGMA temp.schema_version`, nếu không `SELECT *` trả metadata cột cũ.
#[test]
fn cache_xa_khi_temp_schema_doi() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    d.db_execute(&h, "CREATE TEMP TABLE tt (a INTEGER)", &[])
        .unwrap();
    d.db_execute(&h, "INSERT INTO tt VALUES (1)", &[]).unwrap();
    let (truoc, rows) = d.db_query(&h, "SELECT * FROM tt", &[]).unwrap();
    assert_eq!(truoc, vec!["a".to_string()]);
    assert_eq!(o_int(&rows), 1);
    d.db_execute(&h, "ALTER TABLE tt ADD COLUMN b TEXT", &[])
        .unwrap();
    let (sau, rows) = d
        .db_query(&h, "SELECT * FROM tt", &[])
        .expect("query bảng temp sau DDL");
    assert_eq!(
        sau,
        vec!["a".to_string(), "b".to_string()],
        "DDL bảng TEMP phải xả cache (temp.schema_version), không trả metadata cũ"
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].len(), 2);
}
