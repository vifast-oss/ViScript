// SPDX-License-Identifier: MIT OR Apache-2.0
//! Kiểm/chuyển tham số, chuẩn bị statement (cache LRU, phân loại readonly) và đọc kết quả có
//! trần hàng/ô/cột/byte (tách khỏi `mod.rs` ở `ST-317b`, thuần move).

use rusqlite::types::ValueRef;
use rusqlite::Connection;
use vis_core::value::Value;

use super::chuyen_doi::{o_nhap, o_sqlite, raw_bytes};
use super::loi::{kiem_so_luong, loi_doc_ket_qua, loi_hang_qua_lon, loi_prepare, loi_sqlite};
use super::pragma_doc::la_pragma_journal_mode_chi_doc;
use super::{GioiHan, CELL_OVERHEAD, MAX_PARAMS, MAX_PARAMS_TOTAL_BYTES, MAX_PARAM_BYTES};

pub(super) fn kiem_params(params: &[Value]) -> Result<(), String> {
    if params.len() > MAX_PARAMS {
        return Err(format!(
            "quá {} tham số > giới hạn {MAX_PARAMS} — chia nhỏ lời gọi",
            params.len()
        ));
    }
    for (i, p) in params.iter().enumerate() {
        let Some((e, _, _)) = p.as_enum() else {
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
pub(super) fn chuan_bi_tham_so(params: &[Value]) -> Result<Vec<rusqlite::types::Value>, String> {
    let mut out = Vec::with_capacity(params.len());
    let mut tong: usize = 0;
    for (i, p) in params.iter().enumerate() {
        let payload = match p {
            Value::Enum(en) if en.enum_name == "SqlValue" => {
                match (en.variant.as_str(), en.args.as_slice()) {
                    ("Text", [Value::Str(s)]) => s.len(),
                    ("Blob", [Value::Bytes(b)]) => b.len(),
                    _ => 0,
                }
            }
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

/// **VD-268** — "câu lệnh đã chuẩn bị": lấy từ cache LRU của connection hoặc prepare thường
/// (nhánh thường chỉ dùng cho SQL đặc biệt — xem [`chuan_bi_lenh`]). Cả hai `Deref` về
/// [`rusqlite::Statement`] nên phần còn lại của host không đổi.
enum Lenh<'a> {
    Thuong(rusqlite::Statement<'a>),
    Cache(rusqlite::CachedStatement<'a>),
}

impl<'a> std::ops::Deref for Lenh<'a> {
    type Target = rusqlite::Statement<'a>;

    fn deref(&self) -> &Self::Target {
        match self {
            Lenh::Thuong(s) => s,
            Lenh::Cache(s) => s,
        }
    }
}

impl std::ops::DerefMut for Lenh<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Lenh::Thuong(s) => s,
            Lenh::Cache(s) => s,
        }
    }
}

/// **VD-268** — lấy statement cho `sql` từ **cache statement của connection** (cùng chuỗi SQL),
/// trừ trường hợp `sql.trim()` rỗng nhưng `kiem_sql` vẫn thấy token: whitespace **không-ASCII**
/// (ví dụ U+00A0) không bị `kiem_sql` coi là khoảng trắng, còn `prepare_cached` trim bằng
/// `str::trim` của Rust ⇒ statement rỗng/NULL (khác hành vi prepare nguyên văn). Nhánh đó dùng
/// `prepare` thường để **giữ nguyên** kết quả/lỗi như trước.
///
/// Cache key là `sql.trim()`; caller truyền [`lech_loi_prepare`] vào lỗi prepare để `offset`
/// giữ đúng vị trí byte trong chuỗi người dùng (nhánh `prepare` thường không cộng).
fn chuan_bi_lenh<'a>(conn: &'a Connection, sql: &str) -> Result<Lenh<'a>, rusqlite::Error> {
    if sql.trim().is_empty() {
        return Ok(Lenh::Thuong(conn.prepare(sql)?));
    }
    conn.prepare_cached(sql).map(Lenh::Cache)
}

/// Số **byte** whitespace Unicode bị `trim` ở đầu `sql` — bù `offset` lỗi prepare vì cache dùng
/// `sql.trim()` (xem [`chuan_bi_lenh`]).
pub(super) fn lech_trim_dau(sql: &str) -> usize {
    sql.len() - sql.trim_start().len()
}

/// Bù offset **thực tế** cho lỗi prepare: chỉ có tác dụng khi statement đi qua cache (nhánh
/// `sql.trim()` rỗng dùng `prepare` nguyên văn, offset đã đúng — không cộng thêm).
pub(super) fn lech_loi_prepare(sql: &str) -> usize {
    if sql.trim().is_empty() {
        0
    } else {
        lech_trim_dau(sql)
    }
}

/// `execute`: bind đủ số lượng → chạy → số dòng ảnh hưởng. Câu **trả hàng** (sau `prepare`,
/// **trước** khi execute) ⇒ `Err` **không** bước nào bên trong — không side effect. Mọi lỗi
/// SQLite ⇒ `Result.Err` (VD-241 pre-merge §1).
pub(super) fn thuc_thi(
    conn: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
    sqlite_debug: bool,
) -> Result<i64, String> {
    // Schema UTF-8 đã được `kiem_schema_neu_doi` kiểm trước khi gọi (cache schema_version; VD-268
    // xả cache statement khi schema_version đổi).
    let lech = lech_loi_prepare(sql);
    let mut stmt =
        chuan_bi_lenh(conn, sql).map_err(|e| loi_prepare("execute", e, sqlite_debug, lech))?;
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

/// **VD-266** — chế độ đọc của [`truy_van`]: tất cả hàng (`query`) hoặc dừng sau hàng đầu
/// (`query_one`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ToiDaHang {
    TatCa,
    Mot,
}

/// **VD-270 (sqlite-journal-mode)** — `PRAGMA journal_mode` **không giá trị** là truy vấn **chỉ
/// đọc**, nhưng SQLite đánh dấu statement là **không** read-only ở `sqlite3_stmt_readonly` (cùng
/// một câu có thể đổi mode khi có giá trị). Host chỉ mở **đúng dạng hằng** này cho `db_query`
/// qua [`la_pragma_journal_mode_chi_doc`] (xem `pragma_doc.rs`); dạng có giá trị/khác vẫn rơi về
/// kiểm read-only (và authorizer đã chặn ghi pragma).
///
/// **VD-270 (sqlite-database-list)** — `PRAGMA database_list` là read-only thật
/// (`sqlite3_stmt_readonly` = true), chỉ cần authorizer mở mục đọc; **không** thêm ngoại lệ ở đây.
/// `query`: **chỉ đọc** — phân loại **sau** `prepare`, **trước** mọi step (VD-242 §3.6): câu ghi
/// ⇒ `Err`, câu không trả hàng ⇒ `Err`; không thực thi rồi mới phát hiện ghi. Sau đó bind đủ số
/// lượng → đọc tới hết (hoặc tới giới hạn) → `(columns, rows)`.
/// Giới hạn (mặc định, xem [`GioiHan`]): ≤ [`MAX_ROWS`] hàng, ≤ [`MAX_CELLS`] ô, ≤
/// [`MAX_COLUMNS`] cột, ≤ [`MAX_COLUMN_NAME_BYTES`] tên cột, payload mỗi hàng ≤
/// [`MAX_ROW_BYTES`], tổng payload + overhead ô ≤ [`MAX_TOTAL_BYTES`]. Kiểm **trước** khi
/// clone/allocate ô (cộng dồn `checked_add`). Mọi lỗi SQLite ⇒ `Result.Err`.
///
/// **VD-266**: `toi_da = Mot` dừng ngay sau hàng đầu — **không** gọi `next()` thêm, nên không
/// materialize/scan phần còn lại (cap hàng [`MAX_ROWS`] không áp cho `query_one`).
pub(super) fn truy_van(
    conn: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
    sqlite_debug: bool,
    gh: &GioiHan,
    toi_da: ToiDaHang,
) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
    // Schema UTF-8 đã được `kiem_schema_neu_doi` kiểm trước khi gọi (cache schema_version; VD-268
    // xả cache statement khi schema_version đổi — metadata cột bên dưới không được cũ).
    let lech = lech_loi_prepare(sql);
    let mut stmt =
        chuan_bi_lenh(conn, sql).map_err(|e| loi_prepare("query", e, sqlite_debug, lech))?;
    // **VD-242 §3.6 / VD-259**: `query` chỉ đọc. `sqlite3_stmt_readonly` phân loại statement thật
    // sau prepare — không dựa vào tên/keyword, không chạy thử rồi mới phát hiện ghi. Câu ghi phải
    // đi qua `execute` (nơi có trần ghi 17 MiB); câu không trả hàng (`BEGIN`/`COMMIT`/SAVEPOINT)
    // cũng phải đi qua `execute`.
    // **VD-270 (sqlite-journal-mode)**: `PRAGMA journal_mode` (không giá trị) là truy vấn chỉ đọc
    // nhưng SQLite đánh dấu không read-only — chỉ mở **đúng dạng hằng** qua
    // [`la_pragma_journal_mode_chi_doc`]; dạng có giá trị/khác vẫn rơi về kiểm read-only (và
    // authorizer đã chặn ghi pragma).
    if !stmt.readonly() && !la_pragma_journal_mode_chi_doc(sql) {
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
        if toi_da == ToiDaHang::TatCa && out_rows.len() >= gh.rows {
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
        // **VD-266**: `query_one` dừng ngay — không step thêm hàng nào.
        if toi_da == ToiDaHang::Mot {
            break;
        }
    }
    Ok((columns, out_rows))
}
