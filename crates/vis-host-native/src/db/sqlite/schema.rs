// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-286 (R0c)** — pragma host + chính sách schema của `std.sqlite`: chuỗi `HOST_PRAGMAS`,
//! authorizer allow-list (`chinh_sach`), chặn bảng trang/freelist thô, và kiểm schema UTF-8
//! theo `schema_version`/`data_version` (`kiem_schema_neu_doi`, VD-261/VD-268).
//!
//! Tách khỏi `db/sqlite/mod.rs` để giữ luật R1 (≤ 1500 dòng/tệp). Module con nên vẫn truy cập
//! được item private của module cha. **0 đổi hành vi** — chỉ di chuyển mã nguyên văn (ST-286).

use rusqlite::hooks::{AuthAction, Authorization};
use rusqlite::types::ValueRef;
use rusqlite::Connection;

use super::loi::loi_sqlite;
use super::Muc;

/// PRAGMA host đặt **trước authorizer** (mọi chế độ): tạm trong RAM + `trusted_schema=OFF`
/// và page cache **2 MiB** (N7) — dùng đúng một chuỗi cho cả `db_open` lẫn probe test,
/// không được đổi một chỗ mà quên chỗ kia.
pub(super) const HOST_PRAGMAS: &str =
    "PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF; PRAGMA cache_size = -2048;";

/// Bảng ảo truy cập **trang/freelist thô** của tệp SQLite — chính sách host không cho SQL người
/// dùng thấy layout/byte đã xoá. `dbstat` bật trong bundled build; các tên còn lại là module của
/// SQLite (`sqlite_dbpage`/`sqlite_dbdata`/`sqlite_dbptr`) để chặn sẵn nếu build tương lai bật.
/// So **không phân biệt hoa thường**; bảng thường trùng tên cũng bị từ chối (đổi tên bảng).
fn la_bang_trang_tho(ten: &str) -> bool {
    matches!(
        ten.to_ascii_lowercase().as_str(),
        "dbstat" | "sqlite_dbpage" | "sqlite_dbdata" | "sqlite_dbptr"
    )
}

/// Chính sách authorizer — **allow-list theo action code** (mặc định từ chối), **mọi chế độ**,
/// không phải chỉ strict; KHÔNG keyword scan. Cho phép (DDL/CRUD thường, giao dịch, savepoint,
/// REINDEX/ANALYZE, recursive CTE, đọc cột):
///
/// | Action code | Ở đây là | Xử lý |
/// |---|---|---|
/// | `SQLITE_CREATE_TABLE`/`_TEMP_TABLE` · `SQLITE_CREATE_INDEX`/`_TEMP_INDEX` · `SQLITE_CREATE_TRIGGER`/`_TEMP_TRIGGER` · `SQLITE_CREATE_VIEW`/`_TEMP_VIEW` | `CreateTable`/`CreateTempTable`/`CreateIndex`/… | **Allow** |
/// | `SQLITE_DROP_TABLE`/… · `SQLITE_ALTER_TABLE` | `Drop*`/… · `AlterTable` | **Allow** |
/// | `SQLITE_INSERT` · `SQLITE_UPDATE` · `SQLITE_DELETE` | ghi DML | **Allow** |
/// | `SQLITE_SELECT` · `SQLITE_RECURSIVE` | đọc + `WITH RECURSIVE` | **Allow** |
/// | `SQLITE_READ` (bảng thường/view) | đọc cột | **Allow** |
/// | `SQLITE_READ` `dbstat`/`sqlite_dbpage`/`sqlite_dbdata`/`sqlite_dbptr` | trang/freelist thô | **Deny** |
/// | `SQLITE_TRANSACTION` (BEGIN/COMMIT/ROLLBACK) · `SQLITE_SAVEPOINT` | giao dịch | **Allow** (đã duyệt — không tự mở thêm) |
/// | `SQLITE_REINDEX` · `SQLITE_ANALYZE` | maintenance | **Allow** |
/// | `SQLITE_FUNCTION` (trừ `load_extension`/`sqlite_load_extension`/`sqlite_offset`) | hàm SQL | **Allow** |
/// | `SQLITE_PRAGMA` `table_info` · **đọc** `user_version`/`schema_version`/`data_version`/`application_id`/`journal_mode`/`database_list` (không giá trị) | đọc cấu trúc/version/định danh ứng dụng/chế độ journal/danh sách database | **Allow** |
/// | `SQLITE_PRAGMA` khác (kể cả ghi `user_version`, `temp_store`, `journal_mode`…) | đổi cấu hình | **Deny** |
/// | `SQLITE_ATTACH` / `SQLITE_DETACH` | gắn db ngoài | **Deny** |
/// | `SQLITE_CREATE_VTABLE` / `SQLITE_DROP_VTABLE` | virtual table (FTS5…) | **Deny** |
/// | `SQLITE_FUNCTION` `load_extension`/`sqlite_offset` | nạp extension / offset byte trong tệp | **Deny** |
/// | `VACUUM` (cả `VACUUM INTO`) | không có action code riêng — tự `ATTACH` nội tại | **Deny** (qua ATTACH nội tại) |
/// | `Unknown` / mọi action không liệt kê | — | **Deny** |
///
/// **VD-258 (bảo mật)**: schema SQLite (tệp ngoài, `PRAGMA writable_schema`) có thể chứa định
/// danh **không phải UTF-8**. Hook authorizer của rusqlite gọi `CStr::to_str().expect(...)` cho
/// tên bảng/cột/hàm ⇒ panic. Vì vậy host **kiểm schema text trước mọi prepare** (xem
/// [`kiem_schema_utf8`]): byte hỏng ⇒ `Err` dữ liệu, không để rusqlite/`column_name` panic; ma
/// trận authorizer bên dưới giữ **nguyên** chính sách cũ.
///
/// **VD-259 (review độc lập)**: `dbstat` của bundled build (`SQLITE_ENABLE_DBSTAT_VTAB`) đọc
/// **trang/freelist thô** qua `SQLITE_READ` như bảng thường. Chính sách host là không cho SQL
/// người dùng chạm byte/layout tệp ⇒ từ chối `SQLITE_READ` khi tên bảng thuộc nhóm trang thô
/// (xem [`la_bang_trang_tho`]); `sqlite_dbpage`/`sqlite_dbdata`/`sqlite_dbptr` chưa bật trong
/// bundled hiện tại nhưng bị chặn sẵn nếu build tương lai bật.
pub(super) fn chinh_sach(ctx: rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization {
    match ctx.action {
        AuthAction::CreateTable { .. }
        | AuthAction::CreateTempTable { .. }
        | AuthAction::CreateIndex { .. }
        | AuthAction::CreateTempIndex { .. }
        | AuthAction::CreateTrigger { .. }
        | AuthAction::CreateTempTrigger { .. }
        | AuthAction::CreateView { .. }
        | AuthAction::CreateTempView { .. }
        | AuthAction::DropTable { .. }
        | AuthAction::DropTempTable { .. }
        | AuthAction::DropIndex { .. }
        | AuthAction::DropTempIndex { .. }
        | AuthAction::DropTrigger { .. }
        | AuthAction::DropTempTrigger { .. }
        | AuthAction::DropView { .. }
        | AuthAction::DropTempView { .. }
        | AuthAction::AlterTable { .. }
        | AuthAction::Insert { .. }
        | AuthAction::Update { .. }
        | AuthAction::Delete { .. }
        | AuthAction::Select
        | AuthAction::Recursive
        | AuthAction::Transaction { .. }
        | AuthAction::Savepoint { .. }
        | AuthAction::Reindex { .. }
        | AuthAction::Analyze { .. } => Authorization::Allow,
        AuthAction::Read { table_name, .. } => {
            // **VD-259**: không đọc trang/freelist thô qua SQL. So không phân biệt hoa thường;
            // bảng người dùng trùng tên module bị từ chối (đổi tên bảng).
            if la_bang_trang_tho(table_name) {
                Authorization::Deny
            } else {
                Authorization::Allow
            }
        }
        AuthAction::Function { function_name } => {
            // **VD-259**: `sqlite_offset` (nếu build tương lai bật `SQLITE_ENABLE_OFFSET_SQL_FUNC`)
            // trả vị trí byte của cột trong tệp — cùng nhóm "layout trang thô" với `dbstat`.
            if function_name.eq_ignore_ascii_case("load_extension")
                || function_name.eq_ignore_ascii_case("sqlite_load_extension")
                || function_name.eq_ignore_ascii_case("sqlite_offset")
            {
                Authorization::Deny
            } else {
                Authorization::Allow
            }
        }
        AuthAction::Pragma {
            pragma_name,
            pragma_value,
        } => {
            let n = pragma_name.to_ascii_lowercase();
            if n == "table_info" {
                Authorization::Allow
            } else if (n == "user_version"
                || n == "schema_version"
                || n == "data_version"
                || n == "application_id"
                || n == "journal_mode"
                || n == "database_list")
                && pragma_value.is_none()
            {
                // **đọc** user_version/schema_version/data_version/application_id/journal_mode —
                // ghi (có giá trị) ⇒ Deny. schema_version + data_version dùng cache lần quét schema
                // UTF-8 (VD-261); temp.schema_version dùng phát hiện DDL bảng TEMP (VD-268);
                // application_id là metadata định danh ứng dụng 32-bit trong header (VD-270),
                // SQLite không dùng cho việc gì — chỉ đọc, không đổi hành vi ghi/DDL.
                // journal_mode (VD-270) là **đọc** chế độ journal hiện tại cho
                // `sqlite.journal_mode`; ghi `PRAGMA journal_mode = …` vẫn bị từ chối.
                // database_list (VD-270 sqlite-database-list) là **đọc** danh sách database gắn
                // với connection cho `sqlite.database_list`; `ATTACH`/`DETACH` vẫn bị từ chối nên
                // không thể gắn thêm database qua đường người dùng. `= value` (pragma không có
                // dạng ghi hợp lệ) cũng bị từ chối vì có giá trị.
                Authorization::Allow
            } else {
                Authorization::Deny
            }
        }
        AuthAction::Attach { .. }
        | AuthAction::Detach { .. }
        | AuthAction::CreateVtable { .. }
        | AuthAction::DropVtable { .. }
        | AuthAction::Unknown { .. } => Authorization::Deny,
        // `VACUUM` không có action code riêng: mọi đường `VACUUM` tự `ATTACH` nội tại ⇒ bị từ
        // chối tại bước prepare nội tại ⇒ câu lệnh lỗi (không tạo file).
        // Enum `#[non_exhaustive]` — mọi action ngoài allow-list ⇒ từ chối.
        _ => Authorization::Deny,
    }
}

#[cfg(test)]
thread_local! {
    static SCHEMA_SCANS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn schema_scans_for_test() -> u64 {
    SCHEMA_SCANS.with(std::cell::Cell::get)
}

/// Kiểm **toàn bộ** text schema (`sqlite_master.sql`) là UTF-8 hợp lệ **trước** khi prepare SQL
/// người dùng. Cần vì:
/// - hook authorizer của rusqlite panic khi tên bảng/cột không-UTF-8;
/// - `Statement::column_name` cũng panic khi tên cột không-UTF-8.
///
/// Chạy lại **mỗi** prepare (schema có thể đổi từ tiến trình khác); chỉ đọc metadata, không
/// step dữ liệu người dùng. Lỗi ⇒ `Err` dữ liệu, không panic, không echo SQL.
fn kiem_schema_utf8(conn: &Connection, sqlite_debug: bool) -> Result<(), String> {
    #[cfg(test)]
    SCHEMA_SCANS.with(|c| c.set(c.get() + 1));
    let mut stmt = conn
        .prepare("SELECT sql FROM sqlite_master WHERE sql IS NOT NULL")
        .map_err(|e| {
            format!(
                "không đọc được schema SQLite: {}",
                loi_sqlite("lỗi đọc schema", e, sqlite_debug)
            )
        })?;
    let mut rows = stmt
        .query([])
        .map_err(|e| loi_sqlite("lỗi đọc schema", e, sqlite_debug))?;
    while let Some(row) = rows
        .next()
        .map_err(|e| loi_sqlite("lỗi đọc schema", e, sqlite_debug))?
    {
        let bytes = match row
            .get_ref(0)
            .map_err(|e| loi_sqlite("lỗi đọc schema", e, sqlite_debug))?
        {
            ValueRef::Text(t) => t,
            ValueRef::Blob(b) => b,
            _ => continue,
        };
        if std::str::from_utf8(bytes).is_err() {
            return Err(
                "schema SQLite chứa định danh không phải UTF-8 — từ chối (không đọc được tên bảng/cột an toàn)"
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// **VD-261** — quét schema UTF-8 **chỉ khi `schema_version`/`data_version` đổi**.
///
/// Trước đây mỗi `execute`/`query` đều quét toàn bộ `sqlite_master`; với schema 300 bảng,
/// chi phí ~25–45 µs/lời gọi (đo interleaved A/B: chậm 6,8× so với trước). `schema_version`
/// tăng khi DDL của **bất kỳ** connection nào; đọc nó là một pragma đọc rẻ, được allow-list
/// cùng `user_version` (không ghi). Lần đầu `None` ⇒ quét; sau đó chỉ quét lại khi version đổi.
///
/// **VD-268**: `schema_version` đổi cũng là lúc **xả cache statement** của connection. Cache
/// statement của rusqlite giữ `sqlite3_stmt` đã prepare; SQLite chỉ tự recompile ở lần `step`
/// kế tiếp, còn `column_count()`/`column_name()` đọc **trước** step sẽ trả metadata của schema
/// cũ (đã kiểm bằng probe: `SELECT *` sau `ALTER TABLE … ADD COLUMN` vẫn báo 1 cột cho tới khi
/// step). Vì vậy xả cache **trước** prepare của lời gọi kế tiếp.
///
/// - `data_version` đổi (dữ liệu từ connection khác) **không** cần xả — không đổi schema.
/// - DDL trên bảng **TEMP** chỉ đổi schema cookie của database temp (`PRAGMA temp.schema_version`),
///   main `schema_version` giữ nguyên — nên phải đọc thêm cookie temp và xả cache khi nó đổi,
///   nếu không `SELECT *` trên bảng temp vẫn trả metadata cột cũ (probe đã xác nhận).
pub(super) fn kiem_schema_neu_doi(m: &mut Muc, sqlite_debug: bool) -> Result<(), String> {
    let v: (i64, i64, i64) = {
        let conn = m
            .conn
            .as_ref()
            .ok_or_else(|| "handle đã đóng".to_string())?;
        // **VD-268**: ba pragma hằng SQL đi qua cache statement (3 slot LRU). Chúng không phụ
        // thuộc schema và luôn đọc giá trị hiện tại ở step, nên tái dùng an toàn; nhờ vậy chi phí
        // kiểm mỗi lời gọi không tăng dù thêm `temp.schema_version`.
        let schema: i64 = conn
            .prepare_cached("PRAGMA schema_version")
            .and_then(|mut st| st.query_row([], |r| r.get(0)))
            .map_err(|e| loi_sqlite("lỗi đọc schema_version", e, sqlite_debug))?;
        let data: i64 = conn
            .prepare_cached("PRAGMA data_version")
            .and_then(|mut st| st.query_row([], |r| r.get(0)))
            .map_err(|e| loi_sqlite("lỗi đọc data_version", e, sqlite_debug))?;
        // DDL trên bảng TEMP chỉ đổi schema cookie của database temp (main giữ nguyên). Đọc
        // `temp.schema_version` được allow-list cùng `schema_version` (chỉ đọc, không ghi).
        let temp: i64 = conn
            .prepare_cached("PRAGMA temp.schema_version")
            .and_then(|mut st| st.query_row([], |r| r.get(0)))
            .map_err(|e| loi_sqlite("lỗi đọc temp.schema_version", e, sqlite_debug))?;
        (schema, data, temp)
    };
    if m.schema_ver == Some(v) {
        return Ok(());
    }
    let cu = m.schema_ver;
    let conn = m
        .conn
        .as_ref()
        .ok_or_else(|| "handle đã đóng".to_string())?;
    // Quét UTF-8 đúng như VD-261: chỉ khi main `schema_version`/`data_version` đổi (temp schema
    // không nằm trong `sqlite_master`).
    if cu.map(|(s, d, _)| (s, d)) != Some((v.0, v.1)) {
        kiem_schema_utf8(conn, sqlite_debug)?;
    }
    // **VD-268**: schema main **hoặc** temp đổi ⇒ statement đã prepare có thể giữ metadata cột cũ
    // tới lần step kế tiếp. Xả trước khi prepare lời gọi này (an toàn: mọi statement của lời gọi
    // trước đã trả về cache và không còn borrow). Lần đầu (`None`) cache còn rỗng, xả vô hại.
    if cu.map(|(s, _, t)| (s, t)) != Some((v.0, v.2)) {
        conn.flush_prepared_statement_cache();
    }
    m.schema_ver = Some(v);
    Ok(())
}
