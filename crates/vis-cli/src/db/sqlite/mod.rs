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
//!   `PRAGMA` chỉ `table_info` và **đọc** `user_version`/`schema_version` (không ghi); cấm
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
//! - **Pre-materialization**: `SQLITE_LIMIT_LENGTH` = 17 MiB (`MAX_CELL_HARD_BYTES`) trên **đường
//!   ghi** (`execute`), đặt **trước** SQL người dùng; **đường đọc** (`query`) hạ trần này xuống
//!   `min(trần hàng, trần cứng)` (mặc định 1 MiB) **trước** step, nên cell 1–17 MiB không
//!   materialize rồi mới bị cap hàng từ chối. `zeroblob(512 MiB)` lỗi `TOOBIG` (map thành lỗi cap
//!   hàng) thay vì cấp phát hàng trăm MB.
//! - **Trang thô/freelist bị cấm**: `dbstat` (và `sqlite_dbpage`/`sqlite_dbdata`/`sqlite_dbptr`
//!   nếu có) bị authorizer từ chối ở `SQLITE_READ` — SQL người dùng chỉ thấy dữ liệu logic.
//! - 1 câu SQL mỗi `execute`/`query` (multi-statement ⇒ `Err`).
//! - **Mọi** lỗi host là **lỗi dữ liệu/runtime** ⇒ `Result.Err` (không trap `VS0501`).
//!   Hai lớp ngoài `Result` duy nhất: (a) lỗi tĩnh/checker-time (sai kiểu/sai arity), (b) bất
//!   biến nội bộ bị phá (bug nội bộ).
//! - Thông điệp từ SQLite (`loi_sqlite`/`loi_prepare`) **không** kèm raw SQL/bound values/path:
//!   chỉ **mã SQLite (integer `extended_code`)** + tên lớp lỗi + `offset` (byte, nếu có từ
//!   `Error::SqlInputError`). `--sqlite-debug` (CLI, opt-in) in thông điệp thô.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rusqlite::hooks::{AuthAction, Authorization};
use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use vis_interp::builtin::DbHost;
use vis_interp::stdlib::DAU_QUYEN;
use vis_interp::value::Value;

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

/// Trần tài nguyên của một `Db` — tách thành struct để test ép cap thấp mà không phải tạo
/// dữ liệu khổng lồ (mặc định = các hằng `MAX_*` ở trên).
#[derive(Clone, Copy, Debug)]
struct GioiHan {
    rows: usize,
    cells: usize,
    columns: usize,
    ten_cot_bytes: usize,
    row_bytes: usize,
    /// Trần cứng `SQLITE_LIMIT_LENGTH` (mặc định [`MAX_CELL_HARD_BYTES`]).
    hard_cell_bytes: usize,
    total_bytes: usize,
    handles: usize,
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

/// PRAGMA host đặt **trước authorizer** (mọi chế độ): tạm trong RAM + `trusted_schema=OFF`
/// và page cache **2 MiB** (N7) — dùng đúng một chuỗi cho cả `db_open` lẫn probe test,
/// không được đổi một chỗ mà quên chỗ kia.
const HOST_PRAGMAS: &str =
    "PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF; PRAGMA cache_size = -2048;";

/// Handle cơ sở dữ liệu trong phiên.
struct Muc {
    /// `":memory:"` hoặc đường dẫn đã mở (cho diagnostics leak).
    path: String,
    /// `None` khi đã đóng (tombstone — giữ để báo "handle đã đóng").
    conn: Option<Connection>,
    /// **VD-261 (tối ưu)**: `PRAGMA schema_version` tại lần quét schema gần nhất. Chỉ quét
    /// `sqlite_master` khi version đổi; schema không đổi ⇒ không quét lại mỗi prepare.
    schema_ver: Option<i64>,
}

/// Token ngẫu nhiên 128 bit (`db:` + 32 hex thường). Hai nửa 64-bit lấy từ **nguồn OS**:
/// Unix đọc 16 byte `/dev/urandom`; nền tảng khác/không đọc được ⇒ fallback **không-crate**
/// dùng `RandomState` của std (cũng được seed từ OS) băm SipHash hai miền tách biệt, kèm bộ đếm.
/// Token chỉ là **handle mờ trong phiên**, không phải bí mật xác thực: host luôn khớp toàn bộ
/// token trong `bang` của chính phiên, tombstone chặn tái dùng; không dùng làm mật khẩu/capability
/// liên phiên. Không đoán được từ output, không tái sử dụng trong phiên.
fn token_ngau_nhien() -> String {
    static BO_DEM: AtomicU64 = AtomicU64::new(0);
    let dem = BO_DEM.fetch_add(1, Ordering::Relaxed);
    let (a, b) = ngau_nhien_os().unwrap_or_else(|| ngau_nhien_du_phong(dem));
    let token128 = ((a as u128) << 64) | (b as u128);
    format!("db:{token128:032x}")
}

/// 16 byte entropy từ OS trên Unix (`/dev/urandom`). Không dùng `unsafe`/crate ngoài; lỗi mở/đọc
/// ⇒ `None` để caller dùng fallback (không panic, không chặn mở db).
#[cfg(unix)]
fn ngau_nhien_os() -> Option<(u64, u64)> {
    use std::io::Read;
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .ok()?
        .read_exact(&mut b)
        .ok()?;
    Some((
        u64::from_le_bytes(b[0..8].try_into().ok()?),
        u64::from_le_bytes(b[8..16].try_into().ok()?),
    ))
}

/// Không có `/dev/urandom` (Windows/nền tảng khác) — fallback trong `token_ngau_nhien`.
#[cfg(not(unix))]
fn ngau_nhien_os() -> Option<(u64, u64)> {
    None
}

/// Fallback **không thêm dependency**: `RandomState` của std được seed từ OS cho mỗi thread
/// (Unix: getrandom; Windows: OS entropy), rồi SipHash hai miền tách biệt lấy 2×64 bit. Không
/// phải CSPRNG và không hứa 128 bit entropy độc lập tuyệt đối — chấp nhận được vì token là
/// handle mờ trong phiên (xem [`token_ngau_nhien`]); bộ đếm bảo đảm không lặp trong cùng tiến
/// trình ngay cả khi nguồn OS lỗi.
fn ngau_nhien_du_phong(dem: u64) -> (u64, u64) {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id() as u64;
    let chao: u64 = 0;
    let dia_chi = &chao as *const u64 as usize as u64;
    let rs = RandomState::new();
    let bam = |mien: u8| {
        let mut h = rs.build_hasher();
        h.write_u8(mien);
        h.write_u64(t);
        h.write_u64(pid);
        h.write_u64(dem);
        h.write_u64(dia_chi);
        h.finish()
    };
    (bam(0xA1), bam(0x5C))
}

/// Token đúng dạng? `db:` + đúng 32 ký tự hex thường — mọi dạng khác (`db:0`, `db:00`, `db:2`,
/// `db:1x`, `db:`…) đều bị từ chối **trước** khi tra bảng.
fn token_dung_dang(h: &str) -> bool {
    match h.strip_prefix("db:") {
        Some(hex) => hex.len() == 32 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
        None => false,
    }
}

/// Host SQLite của CLI. `--explain-perms` ⇒ log version/compile options + thao tác ra **stderr**;
/// mặc định **không** in (leak warning cũng vậy — chỉ diagnostics, VD-241 §9).
pub struct Db {
    giai_thich: bool,
    /// Wall-clock mỗi câu (`execute`/`query`), ms; `progress_handler` ngắt khi vượt.
    /// Nguyên từ `Instant::now()` **trước** `prepare` tới khi đọc xong hàng — không cộng dồn
    /// busy-wait ngoài deadline. Chỉ thay đổi qua [`Db::moi_voi_query_ms`] (tests/debug).
    query_ms: u64,
    /// `--sqlite-debug`: in thông điệp thô từ SQLite (có thể chứa SQL fragment/bound values).
    sqlite_debug: bool,
    /// Trần hàng/ô/byte/tên cột/handle của phiên (mặc định là các hằng `MAX_*`).
    gioi_han: GioiHan,
    /// Bảng mọi handle của phiên (kể cả tombstone đã đóng) — tra cứu O(1) theo token.
    bang: HashMap<String, Muc>,
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
    pub(crate) fn dat_sqlite_debug(&mut self, on: bool) {
        self.sqlite_debug = on;
    }

    fn dang_mo(&self) -> usize {
        self.bang.values().filter(|m| m.conn.is_some()).count()
    }

    /// Tra handle trong **phiên này**; handle lạ/sai loại/sai dạng/đã đóng ⇒ `Err`.
    /// Khớp **toàn bộ** token (không phần nào được diễn giải); handle đã đóng **không** trỏ sang
    /// connection mới (tombstone + token một lần).
    fn tim(&mut self, h: &str) -> Result<&mut Muc, String> {
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

/// Phiên bản SQLite thật + compile options — qua SQL thuần trên kết nối `:memory:` tạm
/// (không raw FFI, không chạm connection người dùng).
fn thong_tin_sqlite() -> (String, Vec<String>) {
    let ver = rusqlite::version().to_string();
    let mut cc = Vec::new();
    if let Ok(c) = Connection::open_in_memory() {
        if let Ok(mut st) = c.prepare("PRAGMA compile_options") {
            if let Ok(mut rows) = st.query([]) {
                while let Ok(Some(row)) = rows.next() {
                    match row.get::<_, String>(0) {
                        Ok(s) => cc.push(s),
                        Err(_) => break,
                    }
                }
            }
        }
    } else {
        return ("không rõ".to_string(), Vec::new());
    }
    (ver, cc)
}

/// SQL hợp lệ không? — vượt 64 KiB ⇒ `Err`; **không có câu SQL nào** (rỗng/chỉ comment)
/// ⇒ `Err` (cũng để tránh `step(NULL)` ở tầng dưới). Mọi lỗi ở tầng này là dữ liệu/runtime ⇒
/// `Result.Err` (VD-241 pre-merge §1), không trap.
fn kiem_sql(sql: &str) -> Result<(), String> {
    if sql.len() > MAX_SQL_BYTES {
        return Err(format!(
            "SQL dài {} byte > giới hạn {MAX_SQL_BYTES} — cắt nhỏ câu SQL",
            sql.len()
        ));
    }
    // Quét tối thiểu: ký tự nằm ngoài (a) chuỗi `'…'`/`""`/`[…]`, (b) comment `--…`/`/*…*/`.
    let b = sql.as_bytes();
    let mut i = 0usize;
    let mut co_token = false;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'\'' => {
                co_token = true;
                i += 1;
                while i < b.len() {
                    if b[i] == b'\'' {
                        if b.get(i + 1) == Some(&b'\'') {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            b'"' | b'`' => {
                co_token = true;
                let q = b[i];
                i += 1;
                while i < b.len() {
                    if b[i] == q {
                        if b.get(i + 1) == Some(&q) {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            b'[' => {
                co_token = true;
                i += 1;
                while i < b.len() && b[i] != b']' {
                    i += 1;
                }
                i = (i + 1).min(b.len());
            }
            b'-' if b.get(i + 1) == Some(&b'-') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(b.len());
            }
            _ => {
                co_token = true;
                i += 1;
            }
        }
    }
    if !co_token {
        return Err("không có câu SQL nào trong văn bản SQL".to_string());
    }
    Ok(())
}

/// Chuỗi path hiển thị an toàn (không lộ biến môi trường/nội bộ — path là dữ liệu).
fn mo_ta(path: &str) -> String {
    if path == ":memory:" {
        ":memory:".to_string()
    } else if path.len() > 120 {
        // **F-A (2026-11-19)**: không cắt giữa codepoint UTF-8 — lùi về ranh giới ký tự gần nhất ≤120.
        let end = (0..=120)
            .rev()
            .find(|&i| path.is_char_boundary(i))
            .unwrap_or(0);
        format!("{}…", &path[..end])
    } else {
        path.to_string()
    }
}

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
/// | `SQLITE_PRAGMA` `table_info` · **đọc** `user_version`/`schema_version` (không giá trị) | đọc cấu trúc/version | **Allow** |
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
fn chinh_sach(ctx: rusqlite::hooks::AuthContext<'_>) -> rusqlite::hooks::Authorization {
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
            } else if (n == "user_version" || n == "schema_version") && pragma_value.is_none() {
                // **đọc** user_version/schema_version — ghi (có giá trị) ⇒ Deny.
                // `schema_version` chỉ dùng để cache lần quét schema UTF-8 (VD-261).
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

/// Trần `SQLITE_LIMIT_LENGTH` cho **đường đọc** (`query`): không cần lớn hơn trần một hàng kết
/// quả vì ô đơn > trần hàng chắc chắn bị [`truy_van`] từ chối; cũng không vượt trần cứng.
fn tran_o_doc(gh: &GioiHan) -> usize {
    gh.row_bytes.min(gh.hard_cell_bytes)
}

/// Đặt `SQLITE_LIMIT_LENGTH` = trần cứng một string/blob/row **trước** SQL người dùng.
///
/// `zeroblob(N)`/text lớn được SQLite cấp phát **trước** khi host kịp thấy `ValueRef` để đếm
/// `raw_bytes`; không có limit này, `zeroblob(536870912)` làm RSS tăng ~500 MB rồi mới bị cap
/// hàng từ chối. Limit cũng chặn text/blob > trần ngay tại bước step (lỗi dữ liệu `TOOBIG`,
/// không cắt im lặng). `Connection::set_limit` chỉ hạ được trần (không nâng quá compile default).
fn dat_gioi_han_o(conn: &Connection, max_cell_bytes: usize) -> Result<(), String> {
    let m = i32::try_from(max_cell_bytes)
        .map_err(|_| "giới hạn ô vượt i32 — từ chối cấu hình".to_string())?;
    conn.set_limit(Limit::SQLITE_LIMIT_LENGTH, m)
        .map_err(|e| format!("không đặt được SQLITE_LIMIT_LENGTH: {e}"))?;
    let hien = conn
        .limit(Limit::SQLITE_LIMIT_LENGTH)
        .map_err(|e| format!("không đọc lại được SQLITE_LIMIT_LENGTH: {e}"))?;
    if hien != m {
        return Err(format!(
            "SQLITE_LIMIT_LENGTH không áp dụng được (muốn {m}, hiện {hien}) — từ chối mở"
        ));
    }
    Ok(())
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

/// **VD-261** — quét schema UTF-8 **chỉ khi `schema_version` đổi**.
///
/// Trước đây mỗi `execute`/`query` đều quét toàn bộ `sqlite_master`; với schema 300 bảng,
/// chi phí ~25–45 µs/lời gọi (đo interleaved A/B: chậm 6,8× so với trước). `schema_version`
/// tăng khi DDL của **bất kỳ** connection nào; đọc nó là một pragma đọc rẻ, được allow-list
/// cùng `user_version` (không ghi). Lần đầu `None` ⇒ quét; sau đó chỉ quét lại khi version đổi.
fn kiem_schema_neu_doi(m: &mut Muc, sqlite_debug: bool) -> Result<(), String> {
    let v: i64 = {
        let conn = m
            .conn
            .as_ref()
            .ok_or_else(|| "handle đã đóng".to_string())?;
        conn.query_row("PRAGMA schema_version", [], |r| r.get(0))
            .map_err(|e| loi_sqlite("lỗi đọc schema_version", e, sqlite_debug))?
    };
    if m.schema_ver == Some(v) {
        return Ok(());
    }
    let conn = m
        .conn
        .as_ref()
        .ok_or_else(|| "handle đã đóng".to_string())?;
    kiem_schema_utf8(conn, sqlite_debug)?;
    m.schema_ver = Some(v);
    Ok(())
}

/// Ô SQLite → `Value::Enum("SqlValue", …)`. UTF-8 hỏng ⇒ lỗi (dữ liệu, không phải dữ liệu cắt).
fn o_sqlite(v: ValueRef<'_>) -> Result<Value, String> {
    let mk = |variant: &str, args: Vec<Value>| Value::Enum("SqlValue".into(), variant.into(), args);
    match v {
        ValueRef::Null => Ok(mk("Null", Vec::new())),
        ValueRef::Integer(i) => Ok(mk("Integer", vec![Value::Int(i)])),
        // REAL không finite (NaN/±Inf) — SQLite chấp nhận doubles; giữ nguyên (ghi nhận trong báo cáo).
        ValueRef::Real(f) => Ok(mk("Real", vec![Value::Float(f)])),
        ValueRef::Text(t) => match std::str::from_utf8(t) {
            Ok(s) => Ok(mk("Text", vec![Value::Str(s.to_string())])),
            Err(_) => Err("TEXT không phải UTF-8 hợp lệ — không đọc được".to_string()),
        },
        ValueRef::Blob(b) => Ok(mk("Blob", vec![Value::Bytes(b.to_vec())])),
    }
}

/// Kích thước byte thô của ô (text/blob là byte thật; int/float/bool tính theo biểu diễn) —
/// cho giới hạn; đọc từ raw ref **trước** khi clone/allocate.
fn raw_bytes(v: ValueRef<'_>) -> usize {
    match v {
        ValueRef::Null => 0,
        ValueRef::Integer(_) | ValueRef::Real(_) => 8,
        ValueRef::Text(t) => t.len(),
        ValueRef::Blob(b) => b.len(),
    }
}

/// Tham số `Value::Enum("SqlValue", …)` → `types::Value` của rusqlite.
/// `Real(NaN)` ⇒ `Err` (**không** map vào NULL — người dùng muốn NULL thì dùng `SqlValue.Null()`);
/// `±Inf` **được phép** bind/lưu/đăng nhập lại. Ô lạ ⇒ `Err`.
fn o_nhap(v: &Value) -> Result<rusqlite::types::Value, String> {
    let Value::Enum(e, va, ag) = v else {
        return Err(format!("tham số phải là `SqlValue`, được `{}`", v.show()));
    };
    if e != "SqlValue" {
        return Err(format!("tham số phải là `SqlValue`, được enum `{e}`"));
    }
    match (va.as_str(), ag.as_slice()) {
        ("Null", []) => Ok(rusqlite::types::Value::Null),
        ("Integer", [Value::Int(n)]) => Ok(rusqlite::types::Value::Integer(*n)),
        ("Real", [Value::Float(f)]) => {
            if f.is_nan() {
                return Err("Real(NaN) — dùng `SqlValue.Null()` nếu muốn giá trị NULL".to_string());
            }
            Ok(rusqlite::types::Value::Real(*f))
        }
        ("Text", [Value::Str(s)]) => Ok(rusqlite::types::Value::Text(s.clone())),
        ("Blob", [Value::Bytes(b)]) => Ok(rusqlite::types::Value::Blob(b.clone())),
        _ => Err(format!("`SqlValue.{va}` sai đối số ({} đối số)", ag.len())),
    }
}

/// Lỗi từ SQLite: **không** kèm raw message/SQL fragment/bound values/path. Trả về **mã SQLite
/// (integer `extended_code`)** + tên **lớp** lỗi (`ErrorCode` — không phải văn bản SQL) +
/// `offset` (byte, chỉ có từ `Error::SqlInputError` ở đường prepare). Chi tiết đầy đủ chỉ khi
/// người dùng chủ động bật `--sqlite-debug` (`sqlite_debug = true`).
fn loi_sqlite(ctx: &str, e: rusqlite::Error, sqlite_debug: bool) -> String {
    if sqlite_debug {
        return format!("{ctx}: {e}");
    }
    match &e {
        rusqlite::Error::SqliteFailure(err, _) => {
            // `ErrorCode::Unknown` là `SQLITE_ERROR` (1) — mã lỗi SQL chung, không phải "không rõ".
            let ten = if matches!(err.code, rusqlite::ffi::ErrorCode::Unknown) {
                "SqlError".to_string()
            } else {
                format!("{:?}", err.code)
            };
            if err.code == rusqlite::ffi::ErrorCode::AuthorizationForStatementDenied {
                // Authorizer từ chối — thông điệp **tĩnh** (không lấy msg thô của SQLite).
                format!(
                    "{ctx} bị từ chối bởi quyền (authorizer) (mã SQLite {})",
                    err.extended_code
                )
            } else {
                format!("{ctx} (mã SQLite {} — {ten})", err.extended_code)
            }
        }
        rusqlite::Error::SqlInputError { error, offset, .. } => {
            let ten = if matches!(error.code, rusqlite::ffi::ErrorCode::Unknown) {
                "SqlError".to_string()
            } else {
                format!("{:?}", error.code)
            };
            format!(
                "{ctx} (mã SQLite {} — {ten}, offset {offset})",
                error.extended_code
            )
        }
        rusqlite::Error::ExecuteReturnedResults => {
            "câu lệnh trả hàng — dùng `sqlite.query` thay vì `sqlite.execute`".to_string()
        }
        rusqlite::Error::MultipleStatement => {
            format!("{ctx}: chỉ được **một** câu SQL mỗi lời gọi (multi-statement bị từ chối)")
        }
        rusqlite::Error::InvalidParameterCount(n, can) => {
            format!("SQL cần {can} tham số, được {n}")
        }
        _ => format!("{ctx} (lỗi SQLite ngoài lớp đã biết)"),
    }
}

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
        let ket = truy_van(conn, sql, &params, debug, &gh);
        conn.progress_handler(1_000, None::<fn() -> bool>);
        ket
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

fn kiem_params(params: &[Value]) -> Result<(), String> {
    if params.len() > MAX_PARAMS {
        return Err(format!(
            "quá {} tham số > giới hạn {MAX_PARAMS} — chia nhỏ lời gọi",
            params.len()
        ));
    }
    for (i, p) in params.iter().enumerate() {
        let Value::Enum(e, _, _) = p else {
            return Err(format!(
                "tham số {} phải là `SqlValue`, được `{}`",
                i + 1,
                p.show()
            ));
        };
        if e != "SqlValue" {
            return Err(format!(
                "tham số {} phải là `SqlValue`, được enum `{e}`",
                i + 1
            ));
        }
    }
    Ok(())
}

/// Kiểm + chuyển tham số. Giới hạn byte (**text/blob payload**, metadata **không** tính):
/// ≤ [`MAX_PARAM_BYTES`] mỗi tham số, ≤ [`MAX_PARAMS_TOTAL_BYTES`] cộng dồn mỗi lời gọi —
/// kiểm **trước** khi clone/bind; cộng dồn dùng `checked_add` (tràn ⇒ `Err`, không im lặng).
fn chuan_bi_tham_so(params: &[Value]) -> Result<Vec<rusqlite::types::Value>, String> {
    let mut out = Vec::with_capacity(params.len());
    let mut tong: usize = 0;
    for (i, p) in params.iter().enumerate() {
        let payload = match p {
            Value::Enum(e, va, ag) if e == "SqlValue" => match (va.as_str(), ag.as_slice()) {
                ("Text", [Value::Str(s)]) => s.len(),
                ("Blob", [Value::Bytes(b)]) => b.len(),
                _ => 0,
            },
            _ => 0, // sai kiểu — `kiem_params`/`o_nhap` báo lỗi cụ thể
        };
        if payload > MAX_PARAM_BYTES {
            return Err(format!(
                "tham số {} vượt giới hạn {MAX_PARAM_BYTES} byte mỗi tham số (text/blob; metadata không tính vào giới hạn) — cắt nhỏ dữ liệu hoặc lưu qua đường khác",
                i + 1
            ));
        }
        tong = tong
            .checked_add(payload)
            .ok_or_else(|| "tổng payload params tràn — từ chối".to_string())?;
        if tong > MAX_PARAMS_TOTAL_BYTES {
            return Err(format!(
                "tổng payload params vượt giới hạn {MAX_PARAMS_TOTAL_BYTES} byte mỗi lời gọi (text/blob; metadata không tính vào giới hạn) — chia nhỏ lời gọi"
            ));
        }
        out.push(o_nhap(p)?);
    }
    Ok(out)
}

/// `execute`: bind đủ số lượng → chạy → số dòng ảnh hưởng. Câu **trả hàng** (sau `prepare`,
/// **trước** khi execute) ⇒ `Err` **không** bước nào bên trong — không side effect. Mọi lỗi
/// SQLite ⇒ `Result.Err` (VD-241 pre-merge §1).
fn thuc_thi(
    conn: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
    sqlite_debug: bool,
) -> Result<i64, String> {
    // Schema UTF-8 đã được `kiem_schema_neu_doi` kiểm trước khi gọi (cache schema_version).
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| loi_prepare("execute", e, sqlite_debug))?;
    // **Trước** mọi side effect: câu có cột kết quả ⇒ không execute, không insert/update nào chạy.
    if stmt.column_count() > 0 {
        return Err("câu lệnh trả hàng — dùng `sqlite.query` thay vì `sqlite.execute`".to_string());
    }
    kiem_so_luong(&stmt, params.len())?;
    let r = stmt
        .execute(rusqlite::params_from_iter(params.iter()))
        .map_err(|e| loi_sqlite("lỗi khi thực thi", e, sqlite_debug))?;
    Ok(r as i64)
}

/// `query`: **chỉ đọc** — phân loại **sau** `prepare`, **trước** mọi step (VD-242 §3.6): câu ghi
/// ⇒ `Err`, câu không trả hàng ⇒ `Err`; không thực thi rồi mới phát hiện ghi. Sau đó bind đủ số
/// lượng → đọc tới hết (hoặc tới giới hạn) → `(columns, rows)`.
/// Giới hạn (mặc định, xem [`GioiHan`]): ≤ [`MAX_ROWS`] hàng, ≤ [`MAX_CELLS`] ô, ≤
/// [`MAX_COLUMNS`] cột, ≤ [`MAX_COLUMN_NAME_BYTES`] tên cột, payload mỗi hàng ≤
/// [`MAX_ROW_BYTES`], tổng payload + overhead ô ≤ [`MAX_TOTAL_BYTES`]. Kiểm **trước** khi
/// clone/allocate ô (cộng dồn `checked_add`). Mọi lỗi SQLite ⇒ `Result.Err`.
fn truy_van(
    conn: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
    sqlite_debug: bool,
    gh: &GioiHan,
) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
    // Schema UTF-8 đã được `kiem_schema_neu_doi` kiểm trước khi gọi (cache schema_version).
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| loi_prepare("query", e, sqlite_debug))?;
    // **VD-242 §3.6 / VD-259**: `query` chỉ đọc. `sqlite3_stmt_readonly` phân loại statement thật
    // sau prepare — không dựa vào tên/keyword, không chạy thử rồi mới phát hiện ghi. Câu ghi phải
    // đi qua `execute` (nơi có trần ghi 17 MiB); câu không trả hàng (`BEGIN`/`COMMIT`/SAVEPOINT)
    // cũng phải đi qua `execute`.
    if !stmt.readonly() {
        return Err(
            "câu lệnh ghi — `sqlite.query` chỉ đọc, dùng `sqlite.execute` thay vì `sqlite.query`"
                .to_string(),
        );
    }
    if stmt.column_count() == 0 {
        return Err(
            "câu lệnh không trả hàng — dùng `sqlite.execute` thay vì `sqlite.query`".to_string(),
        );
    }
    kiem_so_luong(&stmt, params.len())?;
    let col_n = stmt.column_count();
    // Tên cột: schema đã được kiểm UTF-8 ở trên nên `column_name` không panic vì encoding;
    // vẫn map `Err` (index ngoài miền) thành lỗi dữ liệu. Cap số cột + tổng byte tên cột.
    if col_n > gh.columns {
        return Err(format!(
            "kết quả có {col_n} cột > giới hạn {} cột — chọn ít cột hơn",
            gh.columns
        ));
    }
    let mut columns: Vec<String> = Vec::with_capacity(col_n);
    let mut tong_ten: usize = 0;
    for i in 0..col_n {
        let ten = stmt
            .column_name(i)
            .map_err(|_| "không đọc được tên cột".to_string())?;
        tong_ten = tong_ten
            .checked_add(ten.len())
            .ok_or_else(|| "tổng tên cột tràn — từ chối".to_string())?;
        if tong_ten > gh.ten_cot_bytes {
            return Err(format!(
                "tổng tên cột vượt giới hạn {} byte — chọn ít cột hơn",
                gh.ten_cot_bytes
            ));
        }
        columns.push(ten.to_string());
    }
    // **Metadata** (tên cột) **không** tính vào cap byte payload (có trần riêng ở trên).
    let mut tong: usize = 0;
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params.iter()))
        .map_err(|e| loi_doc_ket_qua(e, gh, sqlite_debug))?;
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    // Overhead **mỗi ô** (Enum + 2 String + Vec + allocator) — cố định theo số cột, cộng vào
    // cap tổng mỗi hàng. Nếu chỉ đếm payload, 100k ô `NULL` lọt cap 16 MiB nhưng vẫn tốn RAM.
    let overhead_moi_hang = col_n
        .checked_mul(CELL_OVERHEAD)
        .ok_or_else(|| "overhead ô tràn — từ chối".to_string())?;
    loop {
        let next = rows
            .next()
            .map_err(|e| loi_doc_ket_qua(e, gh, sqlite_debug))?;
        let Some(row) = next else { break };
        if out_rows.len() >= gh.rows {
            return Err(format!(
                "kết quả vượt giới hạn {} hàng — cắt nhỏ kết quả (LIMIT), không trả dữ liệu cắt",
                gh.rows
            ));
        }
        // Cap **ô** (hàng × cột): chặn 10k × 2k ô NULL (mỗi ô vẫn có overhead `Value::Enum`).
        let da_co = out_rows.len().saturating_mul(col_n);
        if da_co.saturating_add(col_n) > gh.cells {
            return Err(format!(
                "kết quả vượt giới hạn {} ô (hàng × cột) — chọn ít cột hơn hoặc cắt nhỏ kết quả (LIMIT)",
                gh.cells
            ));
        }
        // Lượt 1 (thô, không clone): giữ `ValueRef` của cả hàng (Copy, trỏ vào bộ nhớ SQLite)
        // để đếm kích thước **và** chuyển đổi mà không gọi `get_ref` hai lần.
        let mut o: Vec<ValueRef<'_>> = Vec::with_capacity(col_n);
        let mut row_bytes: usize = 0;
        for i in 0..col_n {
            let v = row
                .get_ref(i)
                .map_err(|e| loi_doc_ket_qua(e, gh, sqlite_debug))?;
            row_bytes = row_bytes
                .checked_add(raw_bytes(v))
                .ok_or_else(|| "một hàng tràn kích thước — từ chối".to_string())?;
            o.push(v);
        }
        if row_bytes > gh.row_bytes {
            return Err(loi_hang_qua_lon(gh.row_bytes));
        }
        tong = tong
            .checked_add(row_bytes)
            .and_then(|x| x.checked_add(overhead_moi_hang))
            .ok_or_else(|| "tổng kích thước kết quả tràn — từ chối".to_string())?;
        if tong > gh.total_bytes {
            return Err(format!(
                "tổng kích thước kết quả vượt giới hạn {} byte (text+blob + overhead ô; tên cột không tính) — cắt nhỏ kết quả (LIMIT)",
                gh.total_bytes
            ));
        }
        // Lượt 2 (allocate): kích thước đã kiểm ⇒ clone an toàn.
        let mut r = Vec::with_capacity(col_n);
        for v in o {
            r.push(o_sqlite(v)?);
        }
        out_rows.push(r);
    }
    Ok((columns, out_rows))
}

/// `true` nếu lỗi SQLite là `SQLITE_TOOBIG` — cell vượt `SQLITE_LIMIT_LENGTH` (đã đặt bằng
/// [`dat_gioi_han_o`]). Trả thông điệp **cùng dạng** với cap hàng để caller thấy giới hạn ổn định.
fn la_toobig(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ffi::ErrorCode::TooBig
    )
}

fn loi_hang_qua_lon(max: usize) -> String {
    format!(
        "một hàng vượt giới hạn {max} byte (text+blob; metadata không tính vào giới hạn) — chọn ít cột hơn hoặc cắt nhỏ dữ liệu"
    )
}

/// Lỗi khi step/đọc ô: `TOOBIG` (cell > limit) ⇒ thông điệp cap hàng, còn lại ⇒ `loi_sqlite`.
fn loi_doc_ket_qua(e: rusqlite::Error, gh: &GioiHan, sqlite_debug: bool) -> String {
    if la_toobig(&e) {
        loi_hang_qua_lon(gh.row_bytes)
    } else {
        loi_sqlite("lỗi khi đọc kết quả", e, sqlite_debug)
    }
}

/// Map lỗi prepare: multi-statement ⇒ `Err`; còn lại ⇒ `loi_sqlite` (mã SQLite + offset,
/// không raw fragment).
fn loi_prepare(ctx: &str, e: rusqlite::Error, sqlite_debug: bool) -> String {
    match e {
        rusqlite::Error::MultipleStatement => {
            format!("{ctx}: chỉ được **một** câu SQL mỗi lời gọi (multi-statement bị từ chối)")
        }
        other => loi_sqlite(&format!("lỗi SQL ({ctx})"), other, sqlite_debug),
    }
}

/// Số tham số phải khớp chính xác — thiếu/thừa ⇒ `Err`.
fn kiem_so_luong(stmt: &rusqlite::Statement, n: usize) -> Result<(), String> {
    let can = stmt.parameter_count();
    if n != can {
        return Err(format!("SQL cần {can} tham số, được {n}"));
    }
    Ok(())
}

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
    fn db_query(
        &mut self,
        h: &str,
        sql: &str,
        params: &[Value],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
        self.trong.db_query(h, sql, params)
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

#[cfg(test)]
mod tests;
