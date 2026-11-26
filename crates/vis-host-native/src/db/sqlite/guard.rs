// SPDX-License-Identifier: MIT OR Apache-2.0
//! `DbGuard` — chặn `std.sqlite` file-backed khi phiên có `cap[...]`/`--strict-caps`
//! (VD-241 §8). Tách khỏi `mod.rs` ở `ST-317b` (thuần move).

use vis_core::host::DbHost;
use vis_core::host::DAU_QUYEN;
use vis_core::value::Value;

use super::Db;

/// Path **có** `cap[...]` (hoặc `--strict-caps`) trong phiên — file-backed bị từ chối (chỉ
/// `:memory:`): I/O của SQLite (journal/WAL/temp) chưa được kiểm soát đủ (VD-241 §8).
pub struct DbGuard<'a> {
    pub trong: &'a mut Db,
    pub quyen: &'a crate::quyen::Quyen,
}

impl DbHost for DbGuard<'_> {
    fn db_open(&mut self, path: &str, create: bool, busy_ms: i64) -> Result<String, String> {
        if path != ":memory:" && self.quyen.bat {
            return Err(format!(
                "{DAU_QUYEN}[quyền] std.sqlite file-backed chưa có bằng chứng kiểm soát I/O (journal/WAL/temp) \u{2014} từ chối khi có `cap` khai báo hoặc `--strict-caps`; chỉ `:memory:` được phép (VD-241 §8)"
            ));
        }
        self.trong.db_open(path, create, busy_ms)
    }
    fn db_close(&mut self, h: &str) -> Result<(), String> {
        self.trong.db_close(h)
    }
    fn db_execute(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<i64, String> {
        self.trong.db_execute(h, sql, params)
    }
    fn db_execute_batch(&mut self, h: &str, sql: &str) -> Result<i64, String> {
        self.trong.db_execute_batch(h, sql)
    }
    fn db_query(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        self.trong.db_query(h, sql, params)
    }
    fn db_query_one(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        self.trong.db_query_one(h, sql, params)
    }
    fn db_last_insert_rowid(&mut self, h: &str) -> Result<i64, String> {
        self.trong.db_last_insert_rowid(h)
    }
    fn db_changes(&mut self, h: &str) -> Result<i64, String> {
        self.trong.db_changes(h)
    }
    fn db_in_transaction(&mut self, h: &str) -> Result<bool, String> {
        self.trong.db_in_transaction(h)
    }
    fn db_cleanup(&mut self) -> Result<u64, String> {
        self.trong.db_cleanup()
    }
    fn db_path_of(&mut self, h: &str) -> Result<String, String> {
        self.trong.db_path_of(h)
    }
}
