// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-311d (VD-286)** — kiểm SQL tối thiểu (`kiem_sql`), thông tin phiên bản SQLite
//! (`thong_tin_sqlite`) và nhãn path an toàn (`mo_ta`).
//!
//! Tách **nguyên văn** khỏi `db/sqlite/mod.rs` để giữ mỗi tệp ≤ 1.150 dòng.

use super::*;

/// Phiên bản SQLite thật + compile options — qua SQL thuần trên kết nối `:memory:` tạm
/// (không raw FFI, không chạm connection người dùng).
pub(super) fn thong_tin_sqlite() -> (String, Vec<String>) {
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
pub(super) fn kiem_sql(sql: &str) -> Result<(), String> {
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
pub(super) fn mo_ta(path: &str) -> String {
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
