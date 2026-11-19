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
/// **128 bit ngẫu nhiên** sinh mỗi lần mở; tra cứu khớp **toàn bộ** chuỗi; token **không** đoán
/// được từ output, **không tái sử dụng** trong phiên; chủ sở hữu phiên == host tạo handle.
/// Mọi lỗi host (sai dạng/không tồn tại/đã đóng/SQL/params/giới hạn/authorizer/timeout/lỗi
/// SQLite) ⇒ `Err(String)` — tầng `std` chuyển thành `Result.Err` dữ liệu (không phải trap).
pub trait DbHost {
    /// Mở db `path` (`":memory:"` hoặc đường dẫn tệp — **không** URI `file:…`; host từ chối rõ).
    /// `create` = cho phép tạo mới (read-write); `busy_ms` = busy_timeout (đã kẹp hữu hạn ≤ 60_000).
    /// Trả **handle mờ**.
    fn db_open(&mut self, path: &str, create: bool, busy_ms: i64) -> Result<String, String>;
    /// Đóng handle; handle lạ/đã đóng ⇒ `Err` (không panic); **không** auto-commit.
    fn db_close(&mut self, h: &str) -> Result<(), String>;
    /// Chạy **một** câu SQL (multi-statement ⇒ host từ chối), params **positional** đã bind đủ
    /// số lượng. Trả số dòng bị ảnh hưởng.
    fn db_execute(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<i64, String>;
    /// Như [`Self::db_execute`] nhưng trả `(columns, rows)` — mỗi ô là `Value::Enum("SqlValue", …)`.
    fn db_query(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String>;
    /// `true` khi connection **đang** trong transaction (SQLite: `sqlite3_get_autocommit() == 0`).
    fn db_in_transaction(&mut self, h: &str) -> Result<bool, String>;
    /// Đóng **mọi** connection của phiên; trả số connection đã đóng (cho diagnostics). Gọi khi
    /// phiên Host kết thúc — **không** auto-commit.
    fn db_cleanup(&mut self) -> Result<u64, String>;
    /// `":memory:"` hoặc đường dẫn đã mở của handle (cho diagnostics rò); handle lạ ⇒ `Err`.
    fn db_path_of(&mut self, h: &str) -> Result<String, String>;
}
