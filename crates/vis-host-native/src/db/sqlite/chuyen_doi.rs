// SPDX-License-Identifier: MIT OR Apache-2.0
//! Chuyển đổi **ô SQLite ↔ `Value`**, param `SqlValue` → rusqlite, và đặt trần
//! `SQLITE_LIMIT_LENGTH` (tách khỏi `mod.rs` ở `ST-317b`, thuần move).

use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use vis_core::value::Value;

use super::GioiHan;

/// Trần `SQLITE_LIMIT_LENGTH` cho **đường đọc** (`query`/`query_one`): không cần lớn hơn trần một
/// hàng kết quả vì ô đơn > trần hàng chắc chắn bị [`truy_van`] từ chối; cũng không vượt trần cứng.
pub(super) fn tran_o_doc(gh: &GioiHan) -> usize {
    gh.row_bytes.min(gh.hard_cell_bytes)
}

/// Đặt `SQLITE_LIMIT_LENGTH` = trần cứng một string/blob/row **trước** SQL người dùng.
///
/// `zeroblob(N)`/text lớn được SQLite cấp phát **trước** khi host kịp thấy `ValueRef` để đếm
/// `raw_bytes`; không có limit này, `zeroblob(536870912)` làm RSS tăng ~500 MB rồi mới bị cap
/// hàng từ chối. Limit cũng chặn text/blob > trần ngay tại bước step (lỗi dữ liệu `TOOBIG`,
/// không cắt im lặng). `Connection::set_limit` chỉ hạ được trần (không nâng quá compile default).
pub(super) fn dat_gioi_han_o(conn: &Connection, max_cell_bytes: usize) -> Result<(), String> {
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

/// Ô SQLite → `Value::enum_val("SqlValue", …)`. UTF-8 hỏng ⇒ lỗi (dữ liệu, không phải dữ liệu cắt).
pub(super) fn o_sqlite(v: ValueRef<'_>) -> Result<Value, String> {
    let mk = |variant: &str, args: Vec<Value>| Value::enum_val("SqlValue", variant, args);
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
pub(super) fn raw_bytes(v: ValueRef<'_>) -> usize {
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
pub(super) fn o_nhap(v: &Value) -> Result<rusqlite::types::Value, String> {
    let Some((e, va, ag)) = v.as_enum() else {
        return Err(format!("tham số phải là `SqlValue`, được `{}`", v.show()));
    };
    if e != "SqlValue" {
        return Err(format!("tham số phải là `SqlValue`, được enum `{e}`"));
    }
    match (va.as_str(), ag) {
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
