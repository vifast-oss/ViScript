// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-242 Q6 / ST-156** — nhánh `core` (`vis-cli --no-default-features`): **không** có
//! backend SQLite. Giữ nguyên chữ ký của `Db`/`DbGuard` và trait `DbHost` để `main.rs` không rẽ
//! nhánh; mọi lời gọi `std.sqlite.*` trả `Err(String)` có chẩn đoán rõ ràng (tầng stdlib đổi
//! thành `Err(Error)` dữ liệu), **không** panic, **không** trả kết quả giả.
#![allow(dead_code)] // chữ ký dựng song song với nhánh `sqlite` (một số hàm chỉ dùng ở tests/debug).

use crate::quyen::Quyen;
use vis_interp::builtin::DbHost;
use vis_interp::value::Value;

/// Chẩn đoán duy nhất, dùng cho **mọi** API `std.sqlite.*` khi dựng core.
pub const THIEU_FEATURE: &str = concat!(
    "`std.sqlite` cần feature `sqlite` của `vis-cli`; ",
    "dựng lại với `--features sqlite`"
);

fn tu_choi<T>() -> Result<T, String> {
    Err(THIEU_FEATURE.to_string())
}

/// Host "SQLite" của bản core — không giữ connection nào, mọi thao tác bị từ chối rõ.
pub struct Db;

impl Db {
    /// Chữ ký giữ nguyên với nhánh `sqlite` (VD-241) — `main.rs` gọi không đổi.
    pub fn moi(_giai_thich: bool) -> Self {
        Db
    }

    /// Chỉ dùng cho tests/debug của nhánh `sqlite`; ở core vẫn tồn tại để giữ chữ ký.
    #[doc(hidden)]
    pub fn moi_voi_query_ms(_giai_thich: bool, _query_ms: u64) -> Self {
        Db
    }

    /// Bật/tắt thông điệp thô từ SQLite — vô nghĩa khi không có backend; giữ để khớp chữ ký.
    pub(crate) fn dat_sqlite_debug(&mut self, _on: bool) {}
}

impl DbHost for Db {
    fn db_open(&mut self, _path: &str, _create: bool, _busy_ms: i64) -> Result<String, String> {
        tu_choi()
    }
    fn db_close(&mut self, _h: &str) -> Result<(), String> {
        tu_choi()
    }
    fn db_execute(&mut self, _h: &str, _sql: &str, _params: &[Value]) -> Result<i64, String> {
        tu_choi()
    }
    fn db_query(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        tu_choi()
    }
    fn db_query_one(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        tu_choi()
    }
    fn db_last_insert_rowid(&mut self, _h: &str) -> Result<i64, String> {
        tu_choi()
    }
    fn db_changes(&mut self, _h: &str) -> Result<i64, String> {
        tu_choi()
    }
    fn db_in_transaction(&mut self, _h: &str) -> Result<bool, String> {
        tu_choi()
    }
    /// Không có connection nào được mở trong bản core ⇒ 0 là số thật, không phải giả.
    fn db_cleanup(&mut self) -> Result<u64, String> {
        Ok(0)
    }
    fn db_path_of(&mut self, _h: &str) -> Result<String, String> {
        tu_choi()
    }
}

/// Guard quyền — cùng trường công khai với nhánh `sqlite` (`main.rs` dựng trực tiếp).
pub struct DbGuard<'a> {
    pub trong: &'a mut Db,
    pub quyen: &'a Quyen,
}

impl DbHost for DbGuard<'_> {
    fn db_open(&mut self, _path: &str, _create: bool, _busy_ms: i64) -> Result<String, String> {
        tu_choi()
    }
    fn db_close(&mut self, _h: &str) -> Result<(), String> {
        tu_choi()
    }
    fn db_execute(&mut self, _h: &str, _sql: &str, _params: &[Value]) -> Result<i64, String> {
        tu_choi()
    }
    fn db_query(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        tu_choi()
    }
    fn db_query_one(
        &mut self,
        _h: &str,
        _sql: &str,
        _params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        tu_choi()
    }
    fn db_last_insert_rowid(&mut self, _h: &str) -> Result<i64, String> {
        tu_choi()
    }
    fn db_changes(&mut self, _h: &str) -> Result<i64, String> {
        tu_choi()
    }
    fn db_in_transaction(&mut self, _h: &str) -> Result<bool, String> {
        tu_choi()
    }
    fn db_cleanup(&mut self) -> Result<u64, String> {
        Ok(0)
    }
    fn db_path_of(&mut self, _h: &str) -> Result<String, String> {
        tu_choi()
    }
}
