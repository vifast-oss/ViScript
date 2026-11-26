// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-241 (Gate 1 + pre-merge 2026-11-19)** — `DbHost` của CLI: **SQLite bundled** qua
//! `rusqlite` (MIT) + `libsqlite3-sys` (feature `bundled`, ghim version trong `Cargo.lock` —
//! build log ghi phiên bản SQLite thật + compile options qua `PRAGMA compile_options` khi
//! `--explain-perms`).
//!
//! Hợp đồng (agent/VD-SQLITE-MVP-2026-11-19.md §5, §7–#9 + Addendum §13 + đợt pre-merge):
//! - **Không** kiểu native nào rò vào `DbHost` trait/`StdFn`/`Value` — trait chỉ thấy `String`/`Value`.
//! - Handle là **token mờ** `db:<32 hex>` — **128 bit từ OS RNG** (Unix: `/dev/urandom`;
//!   fallback không-crate: `RandomState` của std cũng được seed từ OS) sinh mỗi lần mở; tra cứu
//!   khớp **toàn bộ** token; `db:0`/`db:00`/`db:2`/…/token giả/sai loại/đã đóng đều `Err`. Token
//!   **không** đoán được từ output; **không tái sử dụng** trong phiên; tombstone lưu tới khi phiên
//!   kết thúc; handle đã đóng **không** trỏ sang connection mới. Chủ sở hữu phiên == host tạo handle.
//!   Token là **định danh mờ trong phiên**, không phải bí mật xác thực (host vẫn khớp toàn bộ).
//! - `sqlite3_set_authorizer` cài **trước khi** prepare SQL người dùng (mọi prepare đều đi qua);
//!   **allow-list theo action code** (mặc định từ chối): DDL/CRUD thường, `BEGIN`/`COMMIT`/`ROLLBACK`,
//!   `REINDEX`/`ANALYZE`, `Savepoint`, `Recursive`, `Read` (trừ trang thô), mọi `Function` trừ
//!   `load_extension`/`sqlite_offset`;
//!   `PRAGMA` chỉ `table_info` và **đọc** `user_version`/`schema_version`/`data_version`/
//!   `application_id`/`journal_mode`/`database_list` (không ghi); cấm
//!   `ATTACH`/`DETACH`/ghi `PRAGMA` khác/`load_extension`/**`CREATE VIRTUAL TABLE`** (`SQLITE_CREATE_VTABLE`)
//!   ở **mọi** chế độ. `VACUUM` (cả `VACUUM INTO`) bị chặn gián tiếp — nó tự **ATTACH** ở đường
//!   nội tại, authorizer từ chối ⇒ câu lệnh lỗi.
//! - `temp_store=MEMORY` (tạm mọi câu) + `trusted_schema=OFF` + `cache_size = -2048` (2 MiB page
//!   cache) — đặt **trước** authorizer; dữ liệu tạm không rơi vào đĩa.
//! - Mở file: flags `READ_WRITE` (+`CREATE` khi `create`) — **không** `URI` bao giờ; `:memory:`
//!   là giá trị duy nhất cho in-memory. Tên rỗng / `file:…` / `:memory:…` thừa ký tự ⇒ từ chối.
//! - Giới hạn (vượt ⇒ `Err(Error)`, **không** trả dữ liệu cắt; tên cột **không** tính vào cap
//!   payload nhưng có trần riêng; cộng dồn dùng `checked_add`): hàng 10_000 · **ô (hàng×cột)
//!   100_000** · **cột 2_000** · **tổng byte tên cột 256 KiB** · một row ≤ 1 MiB ·
//!   tổng payload kết quả **+ 64 byte overhead mỗi ô** ≤ 16 MiB · SQL ≤ 64 KiB · params ≤ 1_000 ·
//!   **≤ 1 MiB mỗi param text/blob** · **≤ 16 MiB tổng payload params mỗi lời gọi** ·
//!   **tổng handle 1_024/phiên (kể cả tombstone)** · connections ≤ 16/phiên ·
//!   busy_timeout ≤ 60_000 (mặc định 5_000) · wall-clock 5 s mỗi câu (progress_handler).
//! - **`query` chỉ đọc** (VD-242 §3.6): phân loại **sau** `prepare`, **trước** mọi step bằng
//!   `sqlite3_stmt_readonly`; câu ghi (INSERT/UPDATE/DELETE/DDL…) ⇒ `Err`; câu không trả hàng
//!   (`BEGIN`, `COMMIT`, SAVEPOINT…) ⇒ `Err` — dùng `execute`. Không nhận diện bằng tên/keyword.
//!   **VD-270 (journal-mode)**: `PRAGMA journal_mode` (không giá trị) là truy vấn chỉ đọc nhưng
//!   SQLite đánh dấu không read-only; host chỉ mở **đúng dạng hằng** này qua
//!   [`la_pragma_journal_mode_chi_doc`] (ghi `= …` vẫn bị authorizer chặn).
//! - **Pre-materialization**: `SQLITE_LIMIT_LENGTH` = 17 MiB (`MAX_CELL_HARD_BYTES`) trên **đường
//!   ghi** (`execute`), đặt **trước** SQL người dùng; **đường đọc** (`query`) hạ trần này xuống
//!   `min(trần hàng, trần cứng)` (mặc định 1 MiB) **trước** step, nên cell 1–17 MiB không
//!   materialize rồi mới bị cap hàng từ chối. `zeroblob(512 MiB)` lỗi `TOOBIG` (map thành lỗi cap
//!   hàng) thay vì cấp phát hàng trăm MB.
//! - **Trang thô/freelist bị cấm**: `dbstat` (và `sqlite_dbpage`/`sqlite_dbdata`/`sqlite_dbptr`
//!   nếu có) bị authorizer từ chối ở `SQLITE_READ` — SQL người dùng chỉ thấy dữ liệu logic.
//! - 1 câu SQL mỗi `execute`/`query` (multi-statement ⇒ `Err`).
//! - **VD-270 (db-batch)**: `execute_batch` tách batch trên `;` **ngoài** chuỗi/comment/định
//!   danh (xem [`tach_cau`]); câu `CREATE [TEMP] TRIGGER … BEGIN … END` được quét **nguyên khối**
//!   (không tách `;` trong thân), chạy **từng câu** qua đúng đường `db_execute` (authorizer,
//!   `kiem_sql`, trần `SQLITE_LIMIT_LENGTH`, timeout/progress, cap, map lỗi) — **không** params.
//!   Trả số câu đã chạy; câu lỗi đầu tiên ⇒ `Err`, các câu trước **đã có tác dụng** (không tự
//!   mở transaction/rollback). Batch rỗng/chỉ comment ⇒ `Ok(0)`; `execute` vẫn một câu.
//! - **VD-268 (tối ưu)**: `execute`/`query`/`query_one` dùng **cache statement chuẩn bị sẵn** của
//!   rusqlite (`Connection::prepare_cached`, LRU **16** mỗi connection, key = SQL sau khi trim).
//!   Cache được **xả** khi `schema_version` (main) **hoặc** `temp.schema_version` đổi (xem
//!   [`kiem_schema_neu_doi`]) trước lần prepare kế tiếp — metadata cột (`column_count`/
//!   `column_name`) đọc **trước step** sẽ cũ nếu statement còn sống qua DDL (SQLite chỉ tự
//!   recompile ở `sqlite3_step`). Ba pragma version hằng SQL cũng đi qua cache (SQL không đổi,
//!   giá trị đọc tại step). Authorizer, chính sách `readonly`, `kiem_sql`/`kiem_params`, cap
//!   hàng/ô/cột/byte, `SQLITE_LIMIT_LENGTH`, progress-handler và bind params **giữ nguyên**
//!   từng lời gọi: limit của SQLite áp lúc chạy, không "nướng" vào statement lúc prepare. Không
//!   thêm dependency/feature — bản rusqlite ghim `=0.37.0` có sẵn `prepare_cached` (không có
//!   feature tên `cache`; `hashlink` vốn là dependency bắt buộc của nó).
//! - **Mọi** lỗi host là **lỗi dữ liệu/runtime** ⇒ `Result.Err` (không trap `VS0501`).
//!   Hai lớp ngoài `Result` duy nhất: (a) lỗi tĩnh/checker-time (sai kiểu/sai arity), (b) bất
//!   biến nội bộ bị phá (bug nội bộ).
//! - Thông điệp từ SQLite (`loi_sqlite`/`loi_prepare`) **không** kèm raw SQL/bound values/path:
//!   chỉ **mã SQLite (integer `extended_code`)** + tên lớp lỗi + `offset` (byte, nếu có từ
//!   `Error::SqlInputError`). `--sqlite-debug` (CLI, opt-in) in thông điệp thô.

use std::collections::HashMap;

use rusqlite::Connection;

mod chuyen_doi;
mod guard;
mod host;
mod kiem;
mod loi;
mod pragma_doc;
mod schema;
mod tach_cau;
mod token;
mod truy_van;

pub use guard::DbGuard;

#[cfg(test)]
use kiem::mo_ta;
use kiem::thong_tin_sqlite;
#[cfg(test)]
use pragma_doc::la_pragma_journal_mode_chi_doc;
#[cfg(test)]
use rusqlite::hooks::{AuthAction, Authorization};
#[cfg(test)]
use schema::{chinh_sach, schema_scans_for_test, HOST_PRAGMAS};
#[cfg(test)]
use std::time::{Duration, Instant};
use token::token_dung_dang;
#[cfg(test)]
use token::{ngau_nhien_du_phong, ngau_nhien_os};
#[cfg(test)]
use vis_core::host::DbHost;
#[cfg(test)]
use vis_core::host::DAU_QUYEN;
#[cfg(test)]
use vis_core::value::Value;

/// max hàng mỗi query — vượt ⇒ `Err` (không trả dữ liệu cắt).
pub const MAX_ROWS: usize = 10_000;
/// max **ô** (hàng × cột) mỗi query — vượt ⇒ `Err`. Chặn trường hợp 10k hàng × 2k cột
/// (20 triệu `Value::Enum`, mỗi ô vài chục byte overhead) làm RSS tăng hàng trăm MB dù payload
/// text/blob bằng 0 (toàn `NULL`). 100k ô đủ cho bảng rộng thông thường.
pub const MAX_CELLS: usize = 100_000;
/// Overhead ước lượng **mỗi ô** đã materialize (`Value::Enum` + 2 `String` + `Vec` + allocator).
/// Cộng vào cap tổng để 100k ô `NULL` không lọt qua cap 16 MiB payload.
pub const CELL_OVERHEAD: usize = 64;
/// max cột mỗi kết quả — vượt ⇒ `Err` (SQLite mặc định cũng chặn 2000 cột).
pub const MAX_COLUMNS: usize = 2_000;
/// max tổng byte **tên cột** mỗi kết quả — vượt ⇒ `Err` (metadata không tính vào cap payload,
/// nhưng vẫn phải có trần riêng để schema ngoài không bắt host cấp phát tên cột khổng lồ).
pub const MAX_COLUMN_NAME_BYTES: usize = 256 << 10; // 256 KiB
/// max (text+blob) của **một** row **kết quả** — vượt ⇒ `Err`.
pub const MAX_ROW_BYTES: usize = 1 << 20; // 1 MiB
/// Trần **cứng** cho một string/blob/row của SQLite (`SQLITE_LIMIT_LENGTH`) trên **đường ghi**
/// (`execute`), đặt trước SQL người dùng để `zeroblob(N)`/text lớn không materialize trước khi
/// host kịp đếm. Phải đủ lớn cho đường ghi hợp lệ (params tối đa 16 MiB + một row 1 MiB).
/// **Đường đọc** (`query`) hạ trần này xuống `min(MAX_ROW_BYTES, trần cứng)` trước mỗi step —
/// mọi ô kết quả > 1 MiB đều bị cap hàng từ chối, nên hạ sớm chỉ chặn materialize vô ích
/// (test: `doc_ha_tran_cung_ve_tran_hang`).
pub const MAX_CELL_HARD_BYTES: usize = MAX_PARAMS_TOTAL_BYTES + MAX_ROW_BYTES; // 17 MiB
/// max tổng kích thước kết quả (text+blob + overhead ô mọi hàng; **không** tính tên cột) —
/// vượt ⇒ `Err`.
pub const MAX_TOTAL_BYTES: usize = 16 << 20; // 16 MiB
/// max SQL text — vượt ⇒ `Err`.
pub const MAX_SQL_BYTES: usize = 64 << 10; // 64 KiB
/// max params mỗi câu — vượt ⇒ `Err`.
pub const MAX_PARAMS: usize = 1_000;
/// max payload **mỗi** param text/blob — vượt ⇒ `Err` (kiểm **trước** clone/bind).
pub const MAX_PARAM_BYTES: usize = 1 << 20; // 1 MiB
/// max tổng payload params (text/blob) mỗi lời gọi — vượt ⇒ `Err` (cộng dồn `checked_add`).
pub const MAX_PARAMS_TOTAL_BYTES: usize = 16 << 20; // 16 MiB
/// max connection **đang mở** mỗi phiên host — vượt ⇒ `Err`.
pub const MAX_CONNECTIONS: usize = 16;
/// max **tổng handle** (kể cả tombstone đã đóng) mỗi phiên — vượt ⇒ `Err`.
///
/// Trước đây `bang` chỉ có cap connection đang mở; mở/đóng lặp lại append tombstone vô hạn
/// (O(n²) tra cứu + RAM tăng mãi). Cap này chặn cả hai; tombstone vẫn được giữ tới hết phiên
/// trong giới hạn đó.
pub const MAX_HANDLES: usize = 1_024;
/// busy_timeout tối đa (hữu hạn).
pub const MAX_BUSY_MS: i64 = 60_000;
/// Wall-clock mỗi câu (progress_handler).
pub const MAX_QUERY_MS: u64 = 5_000;
/// **VD-268** — số statement chuẩn bị sẵn được giữ mỗi connection (LRU của rusqlite). Nhỏ và
/// **bounded**: đủ cho vòng lặp/ít câu SQL lặp lại; SQL tối đa 64 KiB nên bộ nhớ cache bị chặn
/// trên mỗi connection (không phải cache toàn cục, không chia sẻ giữa handle).
const STMT_CACHE_CAP: usize = 16;

/// Trần tài nguyên của một `Db` — tách thành struct để test ép cap thấp mà không phải tạo
/// dữ liệu khổng lồ (mặc định = các hằng `MAX_*` ở trên).
#[derive(Clone, Copy, Debug)]
pub(super) struct GioiHan {
    pub(super) rows: usize,
    pub(super) cells: usize,
    pub(super) columns: usize,
    pub(super) ten_cot_bytes: usize,
    pub(super) row_bytes: usize,
    /// Trần cứng `SQLITE_LIMIT_LENGTH` (mặc định [`MAX_CELL_HARD_BYTES`]).
    pub(super) hard_cell_bytes: usize,
    pub(super) total_bytes: usize,
    pub(super) handles: usize,
}

impl Default for GioiHan {
    fn default() -> Self {
        Self {
            rows: MAX_ROWS,
            cells: MAX_CELLS,
            columns: MAX_COLUMNS,
            ten_cot_bytes: MAX_COLUMN_NAME_BYTES,
            row_bytes: MAX_ROW_BYTES,
            hard_cell_bytes: MAX_CELL_HARD_BYTES,
            total_bytes: MAX_TOTAL_BYTES,
            handles: MAX_HANDLES,
        }
    }
}

/// Handle cơ sở dữ liệu trong phiên.
pub(super) struct Muc {
    /// `":memory:"` hoặc đường dẫn đã mở (cho diagnostics leak).
    pub(super) path: String,
    /// `None` khi đã đóng (tombstone — giữ để báo "handle đã đóng").
    pub(super) conn: Option<Connection>,
    /// **VD-261 (tối ưu)** + **VD-268**: `(schema_version, data_version, temp.schema_version)`
    /// tại lần kiểm gần nhất. Chỉ quét `sqlite_master` khi `schema_version` hoặc `data_version`
    /// đổi (như VD-261); cache statement bị xả khi `schema_version` **hoặc temp schema** đổi —
    /// DDL trên bảng TEMP không làm đổi schema cookie của main.
    pub(super) schema_ver: Option<(i64, i64, i64)>,
}
/// Host SQLite của CLI. `--explain-perms` ⇒ log version/compile options + thao tác ra **stderr**;
/// mặc định **không** in (leak warning cũng vậy — chỉ diagnostics, VD-241 §9).
pub struct Db {
    pub(super) giai_thich: bool,
    /// Wall-clock mỗi câu (`execute`/`query`), ms; `progress_handler` ngắt khi vượt.
    /// Nguyên từ `Instant::now()` **trước** `prepare` tới khi đọc xong hàng — không cộng dồn
    /// busy-wait ngoài deadline. Chỉ thay đổi qua [`Db::moi_voi_query_ms`] (tests/debug).
    pub(super) query_ms: u64,
    /// `--sqlite-debug`: in thông điệp thô từ SQLite (có thể chứa SQL fragment/bound values).
    pub(super) sqlite_debug: bool,
    /// Trần hàng/ô/byte/tên cột/handle của phiên (mặc định là các hằng `MAX_*`).
    pub(super) gioi_han: GioiHan,
    /// Bảng mọi handle của phiên (kể cả tombstone đã đóng) — tra cứu O(1) theo token.
    pub(super) bang: HashMap<String, Muc>,
}

impl Db {
    /// Mới — `giai_thich` = chế độ diagnostics (`--explain-perms`); budget mặc định [`MAX_QUERY_MS`].
    pub fn moi(giai_thich: bool) -> Self {
        Self::moi_voi_query_ms(giai_thich, MAX_QUERY_MS)
    }

    /// Mới với budget wall-clock **tuỳ ý** (ms) — **chỉ dùng cho tests/debug**; không phải API
    /// ổn định, không lộ ra `std.sqlite`.
    #[doc(hidden)]
    pub fn moi_voi_query_ms(giai_thich: bool, query_ms: u64) -> Self {
        if giai_thich {
            let (ver, cc) = thong_tin_sqlite();
            eprintln!(
                "SQLITE version={} (libsqlite3-sys bundled) compile_options={}",
                ver,
                cc.join(",")
            );
        }
        Db {
            giai_thich,
            query_ms,
            sqlite_debug: false,
            gioi_han: GioiHan::default(),
            bang: HashMap::new(),
        }
    }

    /// Bật/tắt thông điệp thô từ SQLite (`--sqlite-debug`) — opt-in, chỉ CLI.
    pub fn dat_sqlite_debug(&mut self, on: bool) {
        self.sqlite_debug = on;
    }

    pub(super) fn dang_mo(&self) -> usize {
        self.bang.values().filter(|m| m.conn.is_some()).count()
    }

    /// Tra handle trong **phiên này**; handle lạ/sai loại/sai dạng/đã đóng ⇒ `Err`.
    /// Khớp **toàn bộ** token (không phần nào được diễn giải); handle đã đóng **không** trỏ sang
    /// connection mới (tombstone + token một lần).
    pub(super) fn tim(&mut self, h: &str) -> Result<&mut Muc, String> {
        if !token_dung_dang(h) {
            return Err("handle sai dạng — cần `db:<32 hex thường>`".to_string());
        }
        let m = self
            .bang
            .get_mut(h)
            .ok_or_else(|| "handle không tồn tại (chưa mở trong phiên này)".to_string())?;
        if m.conn.is_none() {
            return Err("handle đã đóng".to_string());
        }
        Ok(m)
    }
}
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_batch;
#[cfg(test)]
mod tests_cache;
#[cfg(test)]
mod tests_db_list;
#[cfg(test)]
mod tests_journal;
