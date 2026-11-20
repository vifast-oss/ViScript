// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-241 (Gate 1, 2026-11-19)** — trait `DbHost` (host cơ sở dữ liệu) — tách khỏi `builtin.rs`
//! để giữ luật **R1** (≤ 1.500 dòng/tệp). Trait này **không** biết backend native (rusqlite chỉ ở
//! `vis-cli`): path/handle/params/ô dữ liệu đều là **chuỗi** và [`Value`]; thiếu host ⇒ `VS0604`
//! (cùng mẫu `FsHost`/`NetHost`).

use crate::value::Value;

/// **Host cơ sở dữ liệu** (VD-241, Gate 1) — `vis-interp`/`vis-vm` **không** biết backend native;
/// `vis-cli` cài (rusqlite bundled). **Không** kiểu native nào rò vào trait: path/handle là **chuỗi**,
/// params/ô dữ liệu là [`Value`] (`Enum("SqlValue", …)`), kết quả `(columns, rows)` là `Value`.
/// Thiếu host ⇒ `VS0604` (cùng mẫu [`FsHost`]/[`NetHost`]). Handle là **token mờ** `db:<32 hex>` —
/// **128 bit từ OS RNG** (Unix: `/dev/urandom`; fallback std `RandomState` được seed từ OS) sinh
/// mỗi lần mở; tra cứu khớp **toàn bộ** chuỗi; token **không** đoán được từ output, **không tái sử
/// dụng** trong phiên; chủ sở hữu phiên == host tạo handle. Token là định danh mờ trong phiên,
/// **không** phải bí mật xác thực liên phiên.
/// Mọi lỗi host (sai dạng/không tồn tại/đã đóng/SQL/params/giới hạn/authorizer/timeout/lỗi
/// SQLite) ⇒ `Err(String)` — tầng `std` chuyển thành `Result.Err` dữ liệu (không phải trap).
pub trait DbHost {
    /// Mở db `path` (`":memory:"` hoặc đường dẫn tệp — **không** URI `file:…`; host từ chối rõ).
    /// `create` = cho phép tạo mới (read-write); `busy_ms` = busy_timeout (đã kẹp hữu hạn ≤ 60_000).
    /// Trả **handle mờ**.
    fn db_open(&mut self, path: &str, create: bool, busy_ms: i64) -> Result<String, String>;
    /// Đóng handle; handle lạ/đã đóng ⇒ `Err` (không panic); **không** auto-commit.
    fn db_close(&mut self, h: &str) -> Result<(), String>;
    /// Chạy **một** câu SQL ghi/không-trả-hàng (multi-statement ⇒ host từ chối), params
    /// **positional** đã bind đủ số lượng. Trả số dòng bị ảnh hưởng. Câu trả hàng ⇒ `Err`
    /// (**trước** step, không side effect) — dùng [`Self::db_query`].
    fn db_execute(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<i64, String>;
    /// **VD-270 (db-batch)** — chạy **nhiều** câu SQL (migration) **không** params. Host tách
    /// trên `;` **ngoài** chuỗi/comment/định danh rồi chạy **từng câu** qua đúng đường
    /// [`Self::db_execute`] (authorizer, `kiem_sql`, trần `SQLITE_LIMIT_LENGTH`, timeout/progress,
    /// cap, map lỗi). Trả **số câu đã chạy thành công**; câu lỗi đầu tiên ⇒ `Err` và các câu
    /// trước **đã có tác dụng** (không tự mở transaction, không rollback). Batch rỗng/chỉ
    /// comment ⇒ `Ok(0)`. Host cũ ⇒ `Err` dữ liệu (mặc định), không panic.
    fn db_execute_batch(&mut self, _h: &str, _sql: &str) -> Result<i64, String> {
        Err("host chưa cài `sqlite.execute_batch` (cần host DB mới)".to_string())
    }
    /// **Chỉ đọc** (VD-242 §3.6): câu ghi hoặc câu không trả hàng ⇒ `Err` (phân loại sau prepare,
    /// trước step). Trả `(columns, rows)` — mỗi ô là `Value::Enum("SqlValue", …)`.
    fn db_query(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String>;
    /// **VD-266** — như [`Self::db_query`] (cùng chính sách chỉ đọc/params/schema/timeout/cap
    /// hàng) nhưng chỉ trả **0 hoặc 1** hàng: dừng ngay sau hàng đầu, **không** materialize/scan
    /// phần còn lại. Host cũ không cài ⇒ `Err` dữ liệu (mặc định), không panic.
    fn db_query_one(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        Err("host chưa cài `sqlite.query_one` (cần host DB mới)".to_string())
    }
    /// **VD-266** — `rowid` của lần INSERT gần nhất (metadata chỉ đọc). Host cũ ⇒ `Err` dữ liệu.
    fn db_last_insert_rowid(&mut self, _h: &str) -> Result<i64, String> {
        Err("host chưa cài `sqlite.last_insert_rowid` (cần host DB mới)".to_string())
    }
    /// **VD-266** — số hàng bị INSERT/UPDATE/DELETE gần nhất (metadata chỉ đọc). Host cũ ⇒ `Err`.
    fn db_changes(&mut self, _h: &str) -> Result<i64, String> {
        Err("host chưa cài `sqlite.changes` (cần host DB mới)".to_string())
    }
    /// `true` khi connection **đang** trong transaction (SQLite: `sqlite3_get_autocommit() == 0`).
    fn db_in_transaction(&mut self, h: &str) -> Result<bool, String>;
    /// Đóng **mọi** connection của phiên; trả số connection đã đóng (cho diagnostics). Gọi khi
    /// phiên Host kết thúc — **không** auto-commit.
    fn db_cleanup(&mut self) -> Result<u64, String>;
    /// `":memory:"` hoặc đường dẫn đã mở của handle (cho diagnostics rò); handle lạ ⇒ `Err`.
    fn db_path_of(&mut self, h: &str) -> Result<String, String>;
}
