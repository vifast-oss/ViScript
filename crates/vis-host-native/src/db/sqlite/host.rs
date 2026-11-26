// SPDX-License-Identifier: MIT OR Apache-2.0
//! `impl DbHost for Db` — mở/đóng connection, execute/query/batch + metadata (tách khỏi `mod.rs`
//! ở `ST-317b`, thuần move; hợp đồng VD-241/VD-266/VD-268/VD-270 không đổi).

use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};
use vis_core::host::DbHost;
use vis_core::value::Value;

use super::chuyen_doi::{dat_gioi_han_o, tran_o_doc};
use super::kiem::{kiem_sql, mo_ta};
use super::schema::{chinh_sach, kiem_schema_neu_doi, HOST_PRAGMAS};
use super::tach_cau::TachCau;
use super::token::{token_dung_dang, token_ngau_nhien};
use super::truy_van::{chuan_bi_tham_so, kiem_params, thuc_thi, truy_van, ToiDaHang};
use super::{Db, Muc, MAX_BUSY_MS, MAX_CONNECTIONS, STMT_CACHE_CAP};

impl DbHost for Db {
    fn db_open(&mut self, path: &str, create: bool, busy_ms: i64) -> Result<String, String> {
        if path.is_empty() {
            return Err("tên rỗng — cần `\":memory:\"` hoặc đường dẫn tệp".to_string());
        }
        if path != ":memory:" {
            let s = path.to_ascii_lowercase();
            if s.starts_with("file:") {
                return Err(
                    "URI filename `file:…` không được hỗ trợ (từ chối rõ, §8 VD-241)".to_string(),
                );
            }
            if s.starts_with(":memory:") {
                return Err(format!(
                    "`{path}` bắt đầu bằng `:memory:` nhưng không phải `:memory:` — từ chối"
                ));
            }
        }
        if !(0..=MAX_BUSY_MS).contains(&busy_ms) {
            return Err(format!(
                "busy_timeout_ms = {busy_ms} ngoài khoảng 0..={MAX_BUSY_MS}"
            ));
        }
        if self.dang_mo() >= MAX_CONNECTIONS {
            return Err(format!(
                "quá {MAX_CONNECTIONS} connection trong một phiên — đóng bớt (close) trước"
            ));
        }
        if self.bang.len() >= self.gioi_han.handles {
            return Err(format!(
                "quá {} handle trong một phiên (kể cả handle đã đóng) — mở phiên mới nếu cần thêm",
                self.gioi_han.handles
            ));
        }
        let la_memory = path == ":memory:";
        let flags = if create {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        };
        // **Không** SQLITE_OPEN_URI — `file:…` không bao giờ được diễn giải URI dù lọt qua kiểm tra trên.
        let conn = Connection::open_with_flags(path, flags).map_err(|e| {
            if la_memory {
                format!("không tạo được in-memory database: {e}")
            } else if !create && e.to_string().contains("unable to open") {
                format!(
                    "không mở được `{}` (create = false, tệp chưa tồn tại?)",
                    mo_ta(path)
                )
            } else {
                format!("không mở được `{}`", mo_ta(path))
            }
        })?;
        // **VD-268** — cache statement LRU bounded cho **connection này** (mặc định rusqlite
        // cũng 16; đặt tường minh để hợp đồng không phụ thuộc default của crate).
        conn.set_prepared_statement_cache_capacity(STMT_CACHE_CAP);
        conn.busy_timeout(Duration::from_millis(busy_ms as u64))
            .map_err(|e| format!("không đặt busy_timeout: {e}"))?;
        // Trần cứng một string/blob/row **trước** mọi câu SQL người dùng (chặn `zeroblob`/text
        // lớn materialize trước khi host kịp đếm).
        dat_gioi_han_o(&conn, self.gioi_han.hard_cell_bytes)?;
        // Tạm trong bộ nhớ (§8 VD-241) + trusted_schema=OFF + cache trang **2 MiB** (N7 —
        // `PRAGMA cache_size = -2048` nghĩa là 2048 KiB) — **trước** authorizer. Không được nuốt
        // lỗi: nếu PRAGMA này không áp dụng được, từ chối mở để §8 không bị im lặng.
        conn.execute_batch(HOST_PRAGMAS)
            .map_err(|e| format!("không đặt PRAGMA temp_store/trusted_schema/cache_size: {e}"))?;
        // Authorizer cài **trước khi** prepare SQL người dùng. Schema không-UTF-8 đã bị
        // `kiem_schema_utf8` chặn trước mỗi prepare (hook rusqlite panic nếu gặp chuỗi hỏng).
        conn.authorizer(Some(chinh_sach));
        let mut token = token_ngau_nhien();
        while self.bang.contains_key(&token) {
            token = token_ngau_nhien();
        }
        self.bang.insert(
            token.clone(),
            Muc {
                path: mo_ta(path),
                conn: Some(conn),
                schema_ver: None,
            },
        );
        if self.giai_thich {
            eprintln!("DB open {token} {}", mo_ta(path));
        }
        Ok(token)
    }

    fn db_close(&mut self, h: &str) -> Result<(), String> {
        if !token_dung_dang(h) {
            return Err("handle sai dạng — cần `db:<32 hex thường>`".to_string());
        }
        let Some(m) = self.bang.get_mut(h) else {
            return Err("handle không tồn tại (chưa mở trong phiên này)".to_string());
        };
        if m.conn.is_none() {
            return Err("handle đã đóng".to_string());
        }
        // **Không** auto-commit — `Connection::close` drop; transaction dở bị SQLite huỷ, không commit.
        let c = m.conn.take().expect("vừa kiểm is_some");
        match c.close() {
            Ok(()) => {
                if self.giai_thich {
                    eprintln!("DB close {h}");
                }
                Ok(())
            }
            Err((c, e)) => {
                if let Some(m) = self.bang.get_mut(h) {
                    m.conn = Some(c);
                }
                // **F-B (2026-11-19)**: thông điệp lỗi không được echo full token 128-bit.
                Err(format!(
                    "không đóng được connection SQL (chi tiết đã định vị tại phiên chạy): {e}"
                ))
            }
        }
    }

    fn db_execute(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<i64, String> {
        kiem_sql(sql)?;
        kiem_params(params)?;
        let debug = self.sqlite_debug;
        let ngan_sach = Duration::from_millis(self.query_ms);
        let tran_cung = self.gioi_han.hard_cell_bytes;
        let m = self.tim(h)?;
        kiem_schema_neu_doi(m, debug)?;
        let conn = m.conn.as_mut().expect("tim() đã kiểm");
        // **Đường ghi**: trần cứng 17 MiB (đủ params 16 MiB + row 1 MiB). Đặt lại mỗi lời gọi vì
        // `query` trước đó có thể đã hạ xuống trần đọc.
        dat_gioi_han_o(conn, tran_cung)?;
        let params = chuan_bi_tham_so(params)?;
        // Progress handler cài **trước** `prepare` — từ đó tới hết đọc hàng; reset trên cả
        // đường thành công lẫn lỗi.
        let vao = Instant::now();
        conn.progress_handler(1_000, Some(move || vao.elapsed() > ngan_sach));
        let ket = thuc_thi(conn, sql, &params, debug);
        conn.progress_handler(1_000, None::<fn() -> bool>);
        ket
    }

    /// **VD-270 (db-batch)** — chạy batch nhiều câu, **không** params. Handle được kiểm **kể cả**
    /// batch rỗng (không im lặng nuốt handle sai/đã đóng). Mỗi câu đi qua [`Self::db_execute`]
    /// nguyên vẹn — `TachCau` chỉ tách từ vựng, không thay đổi văn bản. Dừng ở lỗi đầu tiên:
    /// các câu trước đã thực thi (**không** transaction ngầm, **không** rollback hộ).
    fn db_execute_batch(&mut self, h: &str, sql: &str) -> Result<i64, String> {
        self.tim(h)?;
        let mut n: i64 = 0;
        for cau in TachCau::new(sql) {
            self.db_execute(h, cau, &[])?;
            n += 1;
        }
        Ok(n)
    }

    fn db_query(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        kiem_sql(sql)?;
        kiem_params(params)?;
        let debug = self.sqlite_debug;
        let ngan_sach = Duration::from_millis(self.query_ms);
        let gh = self.gioi_han;
        let m = self.tim(h)?;
        kiem_schema_neu_doi(m, debug)?;
        let conn = m.conn.as_mut().expect("tim() đã kiểm");
        // **Đường đọc**: hạ trần cứng về trần hàng (mặc định 1 MiB) **trước** prepare/step để
        // cell 1–17 MiB không materialize rồi mới bị cap hàng từ chối. `execute` đặt lại 17 MiB.
        dat_gioi_han_o(conn, tran_o_doc(&gh))?;
        let params = chuan_bi_tham_so(params)?;
        let vao = Instant::now();
        conn.progress_handler(1_000, Some(move || vao.elapsed() > ngan_sach));
        let ket = truy_van(conn, sql, &params, debug, &gh, ToiDaHang::TatCa);
        conn.progress_handler(1_000, None::<fn() -> bool>);
        ket
    }

    /// **VD-266** — y hệt [`Self::db_query`] (cùng `kiem_sql`/`kiem_params`/schema/authorizer/
    /// read-only/timeout/cap hàng) nhưng dừng sau **hàng đầu**; không materialize/scan phần còn
    /// lại. 0 hàng ⇒ `RowSet` rỗng; nhiều hàng ⇒ đúng 1 hàng.
    fn db_query_one(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        kiem_sql(sql)?;
        kiem_params(params)?;
        let debug = self.sqlite_debug;
        let ngan_sach = Duration::from_millis(self.query_ms);
        let gh = self.gioi_han;
        let m = self.tim(h)?;
        kiem_schema_neu_doi(m, debug)?;
        let conn = m.conn.as_mut().expect("tim() đã kiểm");
        // Cùng trần đọc như `query` — ô > trần hàng bị từ chối **trước** khi materialize.
        dat_gioi_han_o(conn, tran_o_doc(&gh))?;
        let params = chuan_bi_tham_so(params)?;
        let vao = Instant::now();
        conn.progress_handler(1_000, Some(move || vao.elapsed() > ngan_sach));
        let ket = truy_van(conn, sql, &params, debug, &gh, ToiDaHang::Mot);
        conn.progress_handler(1_000, None::<fn() -> bool>);
        ket
    }

    /// **VD-266** — `rowid` của lần INSERT gần nhất; metadata chỉ đọc, không prepare SQL.
    fn db_last_insert_rowid(&mut self, h: &str) -> Result<i64, String> {
        let m = self.tim(h)?;
        let conn = m.conn.as_ref().expect("tim() đã kiểm");
        Ok(conn.last_insert_rowid())
    }

    /// **VD-266** — số hàng bị INSERT/UPDATE/DELETE gần nhất; metadata chỉ đọc.
    fn db_changes(&mut self, h: &str) -> Result<i64, String> {
        let m = self.tim(h)?;
        let conn = m.conn.as_ref().expect("tim() đã kiểm");
        i64::try_from(conn.changes()).map_err(|_| "số hàng thay đổi vượt i64 — từ chối".to_string())
    }

    fn db_in_transaction(&mut self, h: &str) -> Result<bool, String> {
        let m = self.tim(h)?;
        let conn = m.conn.as_mut().expect("tim() đã kiểm");
        Ok(!conn.is_autocommit())
    }

    fn db_cleanup(&mut self) -> Result<u64, String> {
        let mut n = 0u64;
        for m in self.bang.values_mut() {
            if let Some(c) = m.conn.take() {
                // Đóng best-effort; **không** auto-commit (SQLite huỷ transaction dở khi đóng).
                let _ = c.close();
                n += 1;
            }
        }
        Ok(n)
    }

    fn db_path_of(&mut self, h: &str) -> Result<String, String> {
        let m = self.tim(h)?;
        Ok(m.path.clone())
    }
}
