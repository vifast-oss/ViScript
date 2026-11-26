// SPDX-License-Identifier: MIT OR Apache-2.0
//! Ánh xạ lỗi SQLite → `Err(String)` **không lộ** raw SQL/bound values/path (VD-241 §9).
//! Tách khỏi `mod.rs` ở `ST-317b` (thuần move).

use super::GioiHan;

/// Lỗi từ SQLite: **không** kèm raw message/SQL fragment/bound values/path. Trả về **mã SQLite
/// (integer `extended_code`)** + tên **lớp** lỗi (`ErrorCode` — không phải văn bản SQL) +
/// `offset` (byte, chỉ có từ `Error::SqlInputError` ở đường prepare). Chi tiết đầy đủ chỉ khi
/// người dùng chủ động bật `--sqlite-debug` (`sqlite_debug = true`).
pub(super) fn loi_sqlite(ctx: &str, e: rusqlite::Error, sqlite_debug: bool) -> String {
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

/// `true` nếu lỗi SQLite là `SQLITE_TOOBIG` — cell vượt `SQLITE_LIMIT_LENGTH` (đã đặt bằng
/// [`dat_gioi_han_o`]). Trả thông điệp **cùng dạng** với cap hàng để caller thấy giới hạn ổn định.
pub(super) fn la_toobig(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(err, _)
            if err.code == rusqlite::ffi::ErrorCode::TooBig
    )
}

pub(super) fn loi_hang_qua_lon(max: usize) -> String {
    format!(
        "một hàng vượt giới hạn {max} byte (text+blob; metadata không tính vào giới hạn) — chọn ít cột hơn hoặc cắt nhỏ dữ liệu"
    )
}

/// Lỗi khi step/đọc ô: `TOOBIG` (cell > limit) ⇒ thông điệp cap hàng, còn lại ⇒ `loi_sqlite`.
pub(super) fn loi_doc_ket_qua(e: rusqlite::Error, gh: &GioiHan, sqlite_debug: bool) -> String {
    if la_toobig(&e) {
        loi_hang_qua_lon(gh.row_bytes)
    } else {
        loi_sqlite("lỗi khi đọc kết quả", e, sqlite_debug)
    }
}

/// Map lỗi prepare: multi-statement ⇒ `Err`; còn lại ⇒ `loi_sqlite` (mã SQLite + offset,
/// không raw fragment). `lech_dau` bù số byte whitespace bị cache trim ở đầu SQL để `offset`
/// giữ đúng vị trí trong chuỗi người dùng (VD-268).
pub(super) fn loi_prepare(
    ctx: &str,
    mut e: rusqlite::Error,
    sqlite_debug: bool,
    lech_dau: usize,
) -> String {
    if lech_dau > 0 {
        if let rusqlite::Error::SqlInputError { offset, .. } = &mut e {
            *offset = offset.saturating_add(i32::try_from(lech_dau).unwrap_or(i32::MAX));
        }
    }
    match e {
        rusqlite::Error::MultipleStatement => {
            format!("{ctx}: chỉ được **một** câu SQL mỗi lời gọi (multi-statement bị từ chối)")
        }
        other => loi_sqlite(&format!("lỗi SQL ({ctx})"), other, sqlite_debug),
    }
}

/// Số tham số phải khớp chính xác — thiếu/thừa ⇒ `Err`.
pub(super) fn kiem_so_luong(stmt: &rusqlite::Statement, n: usize) -> Result<(), String> {
    let can = stmt.parameter_count();
    if n != can {
        return Err(format!("SQL cần {can} tham số, được {n}"));
    }
    Ok(())
}
