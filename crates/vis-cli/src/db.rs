// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-241 (Gate 1, 2026-11-19)** — `DbHost` của CLI: **SQLite bundled** qua `rusqlite` (MIT) +
//! `libsqlite3-sys` (feature `bundled`, ghim version trong `Cargo.lock` — build log ghi phiên bản
//! SQLite thật + compile options qua `PRAGMA compile_options` khi `--explain-perms`).
//!
//! Hợp đồng (agent/VD-SQLITE-MVP-2026-11-19.md §5, §7–#9 + Addendum §13):
//! - **Không** kiểu native nào rò vào `DbHost` trait/`StdFn`/`Value` — trait chỉ thấy `String`/`Value`.
//! - Handle là **chuỗi mờ** `db:N` (`N` là bộ đếm tăng dần, **không tái sử dụng** trong phiên;
//!   tombstone phiên lưu tới khi phiên kết thúc). Chủ sở hữu phiên == host tạo handle.
//! - `sqlite3_set_authorizer` cài **trước khi** prepare SQL người dùng (mọi prepare đều đi qua);
//!   cấm `ATTACH`/`DETACH`/ghi `PRAGMA user_version`/`load_extension` ở **mọi** chế độ; `PRAGMA`
//!   chỉ `table_info` và **đọc** `user_version`. `VACUUM` (cả `VACUUM INTO`) bị chặn gián tiếp —
//!   nó tự **ATTACH** ở đường nội tại, authorizer từ chối ⇒ câu lệnh lỗi.
//! - `temp_store=MEMORY` (tạm mọi câu) + `trusted_schema=OFF` — đặt **trước** authorizer; dữ liệu
//!   tạm không rơi vào đĩa.
//! - Mở file: flags `READ_WRITE` (+`CREATE` khi `create`) — **không** `URI` bao giờ; `:memory:`
//!   là giá trị duy nhất cho in-memory. Tên rỗng / `file:…` / `:memory:…` thừa ký tự ⇒ từ chối.
//! - Giới hạn (vượt ⇒ `Err`/trap, **không** trả dữ liệu cắt): hàng 10_000 · một row ≤ 1 MiB ·
//!   tổng kết quả ≤ 16 MiB · SQL ≤ 64 KiB · params ≤ 1_000 · connections ≤ 16/phiên ·
//!   busy_timeout ≤ 60_000 (mặc định 5_000) · wall-clock 5 s mỗi câu (progress_handler).
//! - 1 câu SQL mỗi `execute`/`query` (multi-statement ⇒ lỗi; cơ chế: rusqlite từ chối tail).

use std::time::{Duration, Instant};

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use vis_interp::builtin::DbHost;
use vis_interp::stdlib::{DAU_LOI_GOI, DAU_QUYEN};
use vis_interp::value::Value;

/// max hàng mỗi query — vượt ⇒ `Err` (không trả dữ liệu cắt).
pub const MAX_ROWS: usize = 10_000;
/// max (text+blob) của **một** row — vượt ⇒ `Err`.
pub const MAX_ROW_BYTES: usize = 1 << 20; // 1 MiB
/// max tổng kích thước kết quả — vượt ⇒ `Err`.
pub const MAX_TOTAL_BYTES: usize = 16 << 20; // 16 MiB
/// max SQL text — vượt ⇒ **lỗi người gọi** (`VS0501`).
pub const MAX_SQL_BYTES: usize = 64 << 10; // 64 KiB
/// max params mỗi câu — vượt ⇒ **lỗi người gọi** (`VS0501`).
pub const MAX_PARAMS: usize = 1_000;
/// max connection mỗi phiên host — vượt ⇒ **lỗi người gọi** (`VS0501`).
pub const MAX_CONNECTIONS: usize = 16;
/// busy_timeout tối đa (hữu hạn).
pub const MAX_BUSY_MS: i64 = 60_000;
/// Wall-clock mỗi câu (progress_handler).
pub const MAX_QUERY_MS: u64 = 5_000;

/// Handle cơ sở dữ liệu trong phiên.
struct Muc {
    /// Bộ đếm toàn cục của phiên — `db:N`.
    id: u64,
    /// `":memory:"` hoặc đường dẫn đã mở (cho diagnostics leak).
    path: String,
    /// `None` khi đã đóng (tombstone — giữ để báo "handle đã đóng").
    conn: Option<Connection>,
}

/// Host SQLite của CLI. `--explain-perms` ⇒ log version/compile options + thao tác ra **stderr**;
/// mặc định **không** in (leak warning cũng vậy — chỉ diagnostics, VD-241 §9).
pub struct Db {
    giai_thich: bool,
    /// `db:N` kế tiếp; tăng đơn điệu, **không bao giờ** tái sử dụng trong phiên.
    tiep: u64,
    /// Bảng mọi handle của phiên (kể cả tombstone đã đóng).
    bang: Vec<Muc>,
}

impl Db {
    /// Mới — `giai_thich` = chế độ diagnostics (`--explain-perms`).
    pub fn moi(giai_thich: bool) -> Self {
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
            tiep: 0,
            bang: Vec::new(),
        }
    }

    fn dang_mo(&self) -> usize {
        self.bang.iter().filter(|m| m.conn.is_some()).count()
    }

    /// Tra handle trong **phiên này**; handle lạ/sai loại/đã đóng ⇒ lỗi người gọi.
    fn tim(&mut self, h: &str) -> Result<&mut Muc, String> {
        let n: u64 = h
            .strip_prefix("db:")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                format!("{DAU_LOI_GOI}handle `{h}` không phải handle cơ sở dữ liệu (dạng `db:N`)")
            })?;
        let m = self.bang.iter_mut().find(|m| m.id == n).ok_or_else(|| {
            format!("{DAU_LOI_GOI}handle `{h}` không tồn tại (chưa mở trong phiên này)")
        })?;
        if m.conn.is_none() {
            return Err(format!("{DAU_LOI_GOI}handle `{h}` đã đóng"));
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

/// SQL hợp lệ không? — vượt 64 KiB ⇒ lỗi người gọi; **không có câu SQL nào** (rỗng/chỉ comment)
/// ⇒ lỗi người gọi (cũng để tránh `step(NULL)` ở tầng dưới).
fn kiem_sql(sql: &str) -> Result<(), String> {
    if sql.len() > MAX_SQL_BYTES {
        return Err(format!(
            "{DAU_LOI_GOI}SQL dài {} byte > giới hạn {MAX_SQL_BYTES}",
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
        return Err(format!(
            "{DAU_LOI_GOI}không có câu SQL nào trong văn bản SQL"
        ));
    }
    Ok(())
}

/// Chuỗi path hiển thị an toàn (không lộ biến môi trường/nội bộ — path là dữ liệu).
fn mo_ta(path: &str) -> String {
    if path == ":memory:" {
        ":memory:".to_string()
    } else if path.len() > 120 {
        format!("{}…", &path[..120])
    } else {
        path.to_string()
    }
}

/// Chính sách authorizer (VD-241 §7, lệnh H — **mọi chế độ**, không phải chỉ strict):
/// cấm `ATTACH`/`DETACH`, ghi `PRAGMA user_version`, `load_extension` (kể cả dạng hàm).
/// `PRAGMA` chỉ `table_info` và **đọc** `user_version` (không có giá trị). Mọi action khác
/// (DDL/DML thường, `BEGIN`/`COMMIT`/`ROLLBACK`, savepoint) cho phép — mẫu hợp đồng §7.6.
/// `VACUUM` không có action code riêng: mọi đường `VACUUM` tự `ATTACH` nội tại ⇒ bị từ chối
/// tại bước prepare nội tại ⇒ câu lệnh lỗi (không tạo file).
fn chinh_sach(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Attach { .. } | AuthAction::Detach { .. } => Authorization::Deny,
        AuthAction::Function { function_name } => {
            if function_name.eq_ignore_ascii_case("load_extension")
                || function_name.eq_ignore_ascii_case("sqlite_load_extension")
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
            } else if n == "user_version" && pragma_value.is_none() {
                // **đọc** user_version — ghi (có giá trị) ⇒ Deny.
                Authorization::Allow
            } else {
                Authorization::Deny
            }
        }
        _ => Authorization::Allow,
    }
}

/// Ô SQLite → `Value::Enum("SqlValue", …)`. UTF-8 hỏng ⇒ lỗi (môi trường, không phải dữ liệu cắt).
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

/// Kích thước byte của ô (text/blob là byte thật; int/float/bool tính theo biểu diễn) — cho giới hạn.
fn o_bytes(v: &Value) -> usize {
    match v {
        Value::Str(s) => s.len(),
        Value::Bytes(b) => b.len(),
        Value::Int(_) | Value::Float(_) | Value::Bool(_) => 8,
        // Ô SQLite bọc enum `SqlValue` — tính theo **ruột**.
        Value::Enum(e, _, args) if e == "SqlValue" => args.iter().map(o_bytes).sum(),
        _ => 0,
    }
}

/// Tham số `Value::Enum("SqlValue", …)` → `types::Value` của rusqlite. Ô lạ ⇒ lỗi người gọi.
fn o_nhap(v: &Value) -> Result<rusqlite::types::Value, String> {
    let Value::Enum(e, va, ag) = v else {
        return Err(format!(
            "{DAU_LOI_GOI}tham số phải là `SqlValue`, được `{}`",
            v.show()
        ));
    };
    if e != "SqlValue" {
        return Err(format!(
            "{DAU_LOI_GOI}tham số phải là `SqlValue`, được enum `{e}`"
        ));
    }
    match (va.as_str(), ag.as_slice()) {
        ("Null", []) => Ok(rusqlite::types::Value::Null),
        ("Integer", [Value::Int(n)]) => Ok(rusqlite::types::Value::Integer(*n)),
        ("Real", [Value::Float(f)]) => Ok(rusqlite::types::Value::Real(*f)),
        ("Text", [Value::Str(s)]) => Ok(rusqlite::types::Value::Text(s.clone())),
        ("Blob", [Value::Bytes(b)]) => Ok(rusqlite::types::Value::Blob(b.clone())),
        _ => Err(format!(
            "{DAU_LOI_GOI}`SqlValue.{va}` sai đối số ({} đối số)",
            ag.len()
        )),
    }
}

/// Lỗi SQL từ SQLite: nếu do authorizer từ chối ⇒ lỗi **dữ liệu** với thông điệp rõ (không lộ
/// đường dẫn/cấu trúc nội bộ — chỉ "không được phép" + câu lệnh bị chặn ở mức khái quát).
fn loi_sqlite(ctx: &str, e: rusqlite::Error) -> String {
    match &e {
        rusqlite::Error::SqliteFailure(err, msg)
            if err.code == rusqlite::ffi::ErrorCode::AuthorizationForStatementDenied =>
        {
            let m = msg.as_deref().unwrap_or("");
            format!("{ctx} bị từ chối bởi quyền (authorizer): {m}")
        }
        _ => format!("{ctx}: {e}"),
    }
}

impl DbHost for Db {
    fn db_open(&mut self, path: &str, create: bool, busy_ms: i64) -> Result<String, String> {
        if path.is_empty() {
            return Err(format!(
                "{DAU_LOI_GOI}tên rỗng — cần `\":memory:\"` hoặc đường dẫn tệp"
            ));
        }
        if path != ":memory:" {
            let s = path.to_ascii_lowercase();
            if s.starts_with("file:") {
                return Err(format!(
                    "{DAU_LOI_GOI}URI filename `file:…` không được hỗ trợ (từ chối rõ, §8 VD-241)"
                ));
            }
            if s.starts_with(":memory:") {
                return Err(format!(
                    "{DAU_LOI_GOI}`{path}` bắt đầu bằng `:memory:` nhưng không phải `:memory:` — từ chối"
                ));
            }
        }
        if !(0..=MAX_BUSY_MS).contains(&busy_ms) {
            return Err(format!(
                "{DAU_LOI_GOI}busy_timeout_ms = {busy_ms} ngoài khoảng 0..={MAX_BUSY_MS}"
            ));
        }
        if self.dang_mo() >= MAX_CONNECTIONS {
            return Err(format!(
                "{DAU_LOI_GOI}quá {MAX_CONNECTIONS} connection trong một phiên"
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
                    "không mở được `{}` (create = false, tệp chưa tồn tại?): {e}",
                    mo_ta(path)
                )
            } else {
                format!("không mở được `{}`: {e}", mo_ta(path))
            }
        })?;
        conn.busy_timeout(Duration::from_millis(busy_ms as u64))
            .map_err(|e| format!("không đặt busy_timeout: {e}"))?;
        // Tạm trong bộ nhớ (§8 VD-241) + trusted_schema=OFF — **trước** authorizer.
        let _ = conn.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF;");
        // Authorizer cài **trước khi** prepare SQL người dùng.
        conn.authorizer(Some(chinh_sach));
        let id = self.tiep;
        self.tiep += 1;
        self.bang.push(Muc {
            id,
            path: mo_ta(path),
            conn: Some(conn),
        });
        if self.giai_thich {
            eprintln!("DB open db:{id} {}", mo_ta(path));
        }
        Ok(format!("db:{id}"))
    }

    fn db_close(&mut self, h: &str) -> Result<(), String> {
        let n: u64 = h
            .strip_prefix("db:")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                format!("{DAU_LOI_GOI}handle `{h}` không phải handle cơ sở dữ liệu (dạng `db:N`)")
            })?;
        let Some(i) = self.bang.iter().position(|m| m.id == n) else {
            return Err(format!(
                "{DAU_LOI_GOI}handle `{h}` không tồn tại (chưa mở trong phiên này)"
            ));
        };
        if self.bang[i].conn.is_none() {
            return Err(format!("{DAU_LOI_GOI}handle `{h}` đã đóng"));
        }
        // **Không** auto-commit — `Connection::close` drop; transaction dở bị SQLite huỷ, không commit.
        let c = self.bang[i].conn.take().expect("vừa kiểm is_some");
        match c.close() {
            Ok(()) => {
                if self.giai_thich {
                    eprintln!("DB close {h}");
                }
                Ok(())
            }
            Err((c, e)) => {
                self.bang[i].conn = Some(c);
                Err(format!("không đóng được `{h}`: {e}"))
            }
        }
    }

    fn db_execute(&mut self, h: &str, sql: &str, params: &[Value]) -> Result<i64, String> {
        kiem_sql(sql)?;
        kiem_params(params)?;
        let m = self.tim(h)?;
        let conn = m.conn.as_mut().expect("tim() đã kiểm");
        let (params, _) = chuan_bi_tham_so(params)?;
        let vao = Instant::now();
        conn.progress_handler(
            1_000,
            Some(move || vao.elapsed() > Duration::from_millis(MAX_QUERY_MS)),
        );
        let ket = thuc_thi(conn, sql, &params);
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
        let m = self.tim(h)?;
        let conn = m.conn.as_mut().expect("tim() đã kiểm");
        let (params, _) = chuan_bi_tham_so(params)?;
        let vao = Instant::now();
        conn.progress_handler(
            1_000,
            Some(move || vao.elapsed() > Duration::from_millis(MAX_QUERY_MS)),
        );
        let ket = truy_van(conn, sql, &params);
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
        for m in &mut self.bang {
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
            "{DAU_LOI_GOI}{too} tham số > giới hạn {MAX_PARAMS}",
            too = params.len()
        ));
    }
    for (i, p) in params.iter().enumerate() {
        let Value::Enum(e, _, _) = p else {
            return Err(format!(
                "{DAU_LOI_GOI}tham số {} phải là `SqlValue`, được `{}`",
                i + 1,
                p.show()
            ));
        };
        if e != "SqlValue" {
            return Err(format!(
                "{DAU_LOI_GOI}tham số {} phải là `SqlValue`, được enum `{e}`",
                i + 1
            ));
        }
    }
    Ok(())
}

fn chuan_bi_tham_so(params: &[Value]) -> Result<(Vec<rusqlite::types::Value>, Vec<usize>), String> {
    let mut out = Vec::with_capacity(params.len());
    let mut kich_thuoc = Vec::with_capacity(params.len());
    for p in params {
        let v = o_nhap(p)?;
        kich_thuoc.push(match &v {
            rusqlite::types::Value::Blob(b) => b.len(),
            rusqlite::types::Value::Text(s) => s.len(),
            _ => 8,
        });
        out.push(v);
    }
    Ok((out, kich_thuoc))
}

/// `execute`: bind đủ số lượng → chạy → số dòng ảnh hưởng. Trả `Err` **dữ liệu** cho lỗi SQLite;
/// multi-statement / bind sai ⇒ **lỗi người gọi** (`VS0501`).
fn thuc_thi(
    conn: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
) -> Result<i64, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| loi_prepare("execute", e))?;
    kiem_so_luong(&stmt, params.len())?;
    let r = stmt
        .execute(rusqlite::params_from_iter(params.iter()))
        .map_err(|e| loi_sqlite("lỗi khi thực thi", e))?;
    Ok(r as i64)
}

/// `query`: bind đủ số lượng → đọc tới hết (hoặc tới giới hạn) → `(columns, rows)`.
fn truy_van(
    conn: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
    let mut stmt = conn.prepare(sql).map_err(|e| loi_prepare("query", e))?;
    kiem_so_luong(&stmt, params.len())?;
    let col_n = stmt.column_count();
    let mut columns: Vec<String> = Vec::with_capacity(col_n);
    for i in 0..col_n {
        let ten = stmt
            .column_name(i)
            .map_err(|_| "không đọc được tên cột".to_string())?
            .to_string();
        columns.push(ten);
    }
    let mut tong: usize = columns.iter().map(|c| c.len()).sum();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params.iter()))
        .map_err(|e| loi_sqlite("lỗi khi thực thi", e))?;
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    loop {
        let next = rows
            .next()
            .map_err(|e| loi_sqlite("lỗi khi đọc kết quả", e))?;
        let Some(row) = next else { break };
        if out_rows.len() >= MAX_ROWS {
            return Err(format!(
                "kết quả vượt giới hạn {MAX_ROWS} hàng — từ chối, không trả dữ liệu cắt"
            ));
        }
        let mut r = Vec::with_capacity(col_n);
        let mut row_bytes = 0usize;
        for i in 0..col_n {
            let v = o_sqlite(
                row.get_ref(i)
                    .map_err(|e| loi_sqlite("lỗi khi đọc ô dữ liệu", e))?,
            )?;
            row_bytes += o_bytes(&v);
            r.push(v);
        }
        if row_bytes > MAX_ROW_BYTES {
            return Err(format!(
                "một hàng vượt giới hạn {MAX_ROW_BYTES} byte (text+blob) — từ chối"
            ));
        }
        tong += row_bytes;
        if tong > MAX_TOTAL_BYTES {
            return Err(format!(
                "tổng kích thước kết quả vượt giới hạn {MAX_TOTAL_BYTES} byte — từ chối"
            ));
        }
        out_rows.push(r);
    }
    Ok((columns, out_rows))
}

/// Map lỗi prepare: multi-statement ⇒ **lỗi người gọi** (`VS0501`); còn lại ⇒ lỗi dữ liệu.
fn loi_prepare(ctx: &str, e: rusqlite::Error) -> String {
    match e {
        rusqlite::Error::MultipleStatement => {
            format!("{DAU_LOI_GOI}{ctx}: chỉ được **một** câu SQL mỗi lời gọi (multi-statement bị từ chối)")
        }
        other => loi_sqlite(&format!("lỗi SQL ({ctx})"), other),
    }
}

/// Số tham số phải khớp chính xác — thiếu/thừa ⇒ **lỗi người gọi** (`VS0501`).
fn kiem_so_luong(stmt: &rusqlite::Statement, n: usize) -> Result<(), String> {
    let can = stmt.parameter_count();
    if n != can {
        return Err(format!("{DAU_LOI_GOI}SQL cần {can} tham số, được {n}"));
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
mod tests {
    use super::*;

    fn sv_int(n: i64) -> Value {
        Value::Enum("SqlValue".into(), "Integer".into(), vec![Value::Int(n)])
    }

    fn mo_db(d: &mut Db) -> String {
        d.db_open(":memory:", true, 5_000).expect("open :memory:")
    }

    fn o_int(rows: &[Vec<Value>]) -> i64 {
        assert_eq!(rows.len(), 1, "phải có đúng 1 hàng: {rows:?}");
        let row = &rows[0];
        assert_eq!(row.len(), 1, "phải có đúng 1 ô: {row:?}");
        let Value::Enum(e, v, args) = &row[0] else {
            panic!("ô không phải enum: {row:?}")
        };
        assert_eq!(e, "SqlValue");
        assert_eq!(v, "Integer");
        assert_eq!(args.len(), 1);
        let Value::Int(n) = &args[0] else {
            panic!("ô không phải int")
        };
        *n
    }

    /// Xác minh trình tự mở của host (cùng các lệnh, cùng thứ tự) đặt `temp_store=MEMORY`
    /// trước khi authorizer — đọc lại bằng connection raw với **đúng** các lệnh host dùng.
    fn cau_hinh_temp_store_that() -> i64 {
        let c = Connection::open_in_memory().unwrap();
        let _ = c.execute_batch("PRAGMA temp_store=MEMORY; PRAGMA trusted_schema=OFF;");
        c.pragma_query_value(None, "temp_store", |r| r.get(0))
            .expect("đọc pragma temp_store")
    }

    #[test]
    fn moi_co_version_va_compile_options_that() {
        let (ver, cc) = thong_tin_sqlite();
        assert!(ver.starts_with("3."), "SQLite version lạ: {ver}");
        assert!(
            cc.iter().any(|o| o.contains("ENABLE_FTS5")),
            "bundled phải có ENABLE_FTS5, thấy: {cc:?}"
        );
    }

    #[test]
    fn open_ids_tang_don_dieu_khong_tai_su_dung() {
        let mut d = Db::moi(false);
        let a = mo_db(&mut d);
        let b = mo_db(&mut d);
        assert_eq!(a, "db:0");
        assert_eq!(b, "db:1");
        d.db_close(&a).unwrap();
        let c = mo_db(&mut d);
        assert_eq!(c, "db:2", "id không tái sử dụng sau close");
        // Handle cũ (tombstone) vẫn báo đã đóng, không trỏ nhầm connection mới.
        let e = d.db_execute("db:0", "SELECT 1", &[]).unwrap_err();
        assert!(e.contains("đã đóng"), "{e}");
        assert!(e.starts_with(DAU_LOI_GOI));
    }

    #[test]
    fn handle_la_sai_loai_bi_tu_choi() {
        let mut d = Db::moi(false);
        for h in ["db:99", "tcp:0", "db:", "1", "db:1x"] {
            let e = d.db_execute(h, "SELECT 1", &[]).unwrap_err();
            assert!(e.starts_with(DAU_LOI_GOI), "handle {h:?}: {e}");
        }
    }

    #[test]
    fn uri_ten_rong_memory_thua_bi_tu_choi() {
        let mut d = Db::moi(false);
        for p in [
            "",
            "file:/tmp/x.db",
            "FILE:/tmp/x.db",
            ":memory:x",
            ":MEMORY:",
        ] {
            let e = d.db_open(p, true, 5_000).unwrap_err();
            assert!(e.starts_with(DAU_LOI_GOI), "path {p:?}: {e}");
        }
        // `:memory:` thật thì hợp lệ.
        assert!(d.db_open(":memory:", true, 5_000).is_ok());
    }

    #[test]
    fn busy_ngoai_khoang_bi_tu_choi() {
        let mut d = Db::moi(false);
        for ms in [-1, 60_001, i64::MAX] {
            let e = d.db_open(":memory:", true, ms).unwrap_err();
            assert!(e.starts_with(DAU_LOI_GOI), "busy {ms}: {e}");
        }
        assert!(d.db_open(":memory:", true, 60_000).is_ok());
    }

    #[test]
    fn connection_thu_17_bi_tu_choi() {
        let mut d = Db::moi(false);
        for _ in 0..MAX_CONNECTIONS {
            mo_db(&mut d);
        }
        let e = d.db_open(":memory:", true, 5_000).unwrap_err();
        assert!(e.starts_with(DAU_LOI_GOI));
        assert!(e.contains("16"), "{e}");
    }

    #[test]
    fn create_false_thieu_tep_la_loi_du_lieu_khong_phai_du_lieu_cat() {
        let dir = std::env::temp_dir().join(format!("vis-sqlite-create-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("khong-ton-tai.db");
        let mut d = Db::moi(false);
        let e = d.db_open(p.to_str().unwrap(), false, 5_000).unwrap_err();
        assert!(!e.starts_with(DAU_LOI_GOI), "phải là lỗi dữ liệu: {e}");
        assert!(!e.starts_with(DAU_QUYEN), "không phải lỗi quyền: {e}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn multi_statement_bi_tu_choi() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        let e = d.db_execute(&h, "SELECT 1; SELECT 2", &[]).unwrap_err();
        assert!(e.starts_with(DAU_LOI_GOI), "{e}");
        let e2 = d.db_query(&h, "SELECT 1; SELECT 2", &[]).unwrap_err();
        assert!(e2.starts_with(DAU_LOI_GOI), "{e2}");
    }

    #[test]
    fn bind_thieu_thua_bi_tu_choi() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        d.db_execute(&h, "CREATE TABLE t (a INTEGER, b INTEGER)", &[])
            .unwrap();
        let thieu = d
            .db_execute(&h, "INSERT INTO t VALUES (?, ?)", &[sv_int(1)])
            .unwrap_err();
        assert!(thieu.starts_with(DAU_LOI_GOI));
        let thua = d
            .db_execute(
                &h,
                "INSERT INTO t VALUES (?, ?)",
                &[sv_int(1), sv_int(2), sv_int(3)],
            )
            .unwrap_err();
        assert!(thua.starts_with(DAU_LOI_GOI));
        // Thiếu nhưng **đúng với câu SQL** thì ok.
        assert!(d
            .db_execute(&h, "INSERT INTO t VALUES (?, ?)", &[sv_int(1), sv_int(2)])
            .is_ok());
    }

    #[test]
    fn sql_qua_64kib_bi_tu_choi() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        let sql = format!("SELECT '{}'", "x".repeat(MAX_SQL_BYTES));
        let e = d.db_query(&h, &sql, &[]).unwrap_err();
        assert!(e.starts_with(DAU_LOI_GOI));
        assert!(e.contains("65536"), "{e}");
    }

    #[test]
    fn params_qua_1000_bi_tu_choi() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        let params: Vec<Value> = (0..(MAX_PARAMS + 1) as i64).map(sv_int).collect();
        let e = d.db_execute(&h, "SELECT 1", &params).unwrap_err();
        assert!(e.starts_with(DAU_LOI_GOI));
        assert!(e.contains("1000"), "{e}");
    }

    #[test]
    fn query_vuot_10000_hang_err_khong_tra_du_lieu_cat() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        let e = d
            .db_query(
                &h,
                "WITH RECURSIVE c(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM c WHERE x < 10001) SELECT x FROM c",
                &[],
            )
            .unwrap_err();
        assert!(
            !e.starts_with(DAU_LOI_GOI),
            "phải là lỗi giới hạn (dữ liệu): {e}"
        );
        assert!(e.contains("10000"), "{e}");
    }

    #[test]
    fn row_vuot_1mib_err() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        let e = d
            .db_query(&h, "SELECT randomblob(1048577)", &[])
            .unwrap_err();
        assert!(!e.starts_with(DAU_LOI_GOI));
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
        assert!(!e.starts_with(DAU_LOI_GOI));
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
        assert!(!e.unwrap_err().starts_with(DAU_LOI_GOI));
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
            "PRAGMA journal_mode=MEMORY",
            "VACUUM INTO '/tmp/xx-khong-duoc.db'",
            "VACUUM",
        ] {
            let r = d.db_execute(&h, sql, &[]);
            assert!(r.is_err(), "không được phép: {sql}");
        }
        // SELECT load_extension — SQLite tắt sẵn ở tầng C; authorizer chặn `load_extension`.
        let r = d.db_execute(&h, "SELECT load_extension('/tmp/xx.so')", &[]);
        assert!(r.is_err());
        // table_info và đọc user_version thì được phép.
        assert!(d.db_query(&h, "PRAGMA table_info(t)", &[]).is_ok());
        assert!(d.db_query(&h, "PRAGMA user_version", &[]).is_ok());
    }

    #[test]
    fn temp_store_bi_ep_memory() {
        let mut d = Db::moi(false);
        let h = mo_db(&mut d);
        assert_eq!(
            cau_hinh_temp_store_that(),
            2,
            "temp_store phải là MEMORY (2)"
        );
        // Đường SQL người dùng không đụng được (PRAGMA temp_store bị authorizer từ chối).
        let e = d.db_query(&h, "PRAGMA temp_store", &[]);
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
        assert!(e.starts_with(DAU_LOI_GOI));
        let e2 = d.db_execute(&b, "SELECT 1", &[]).unwrap_err();
        assert!(e2.starts_with(DAU_LOI_GOI));
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
}
