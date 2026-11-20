// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (db-explain)** — lõi **dùng chung** cho `sqlite.explain` (interp ↔ VM byte-identical).
//!
//! Hai bước, theo thứ tự:
//! 1. **Lọc trước khi chạm host**: sau khoảng trắng và comment đầu (`--…` tới hết dòng,
//!    `/*…*/`), từ khoá đầu phải là `SELECT` hoặc `WITH` (không phân biệt hoa thường). Rỗng,
//!    chỉ comment, comment chưa đóng hoặc mọi từ khoá khác ⇒ `Err(String)` **dữ liệu** — tầng
//!    std biến thành `Err(Error)`, **không** trap, và **không** prepare/chạy gì.
//! 2. **Chạy qua đúng đường `DbHost::db_query` hiện có** với `EXPLAIN QUERY PLAN <sql>` và
//!    **cùng** params: authorizer, chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần
//!    SQL/hàng/ô/cột/byte, progress-handler timeout, cache schema/statement và bind params giữ
//!    nguyên — không thêm method host, không tự step/đọc trang thô.
//!
//! `EXPLAIN QUERY PLAN` chỉ **chuẩn bị** câu gốc để lấy kế hoạch, không thực thi câu gốc; kết quả
//! là `RowSet` 4 cột `id, parent, notused, detail` (SQLite ≥ 3.24).
//!
//! **Không** phải parser SQL: chỉ kiểm từ khoá đầu theo đúng hợp đồng. Câu `WITH` có phần chính
//! là ghi qua được bước 1 (từ khoá đầu `WITH`), nhưng `EXPLAIN QUERY PLAN` của câu ghi bị
//! `db_query` phân loại **không chỉ đọc** (`sqlite3_stmt_readonly`) ⇒ `Err` trước mọi step —
//! probe xác nhận `WITH … DELETE`/`WITH … INSERT` bị chặn, dữ liệu không đổi. Authorizer/chính
//! sách của `db_query` áp nguyên.

use crate::builtin::DbHost;
use crate::value::Value;

/// Lọc `SELECT`/`WITH` rồi chạy `EXPLAIN QUERY PLAN <sql>` qua `host.db_query` với cùng params.
pub fn chay(
    host: &mut dyn DbHost,
    h: &str,
    sql: &str,
    params: &[Value],
) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
    if !la_truy_van_chi_doc(sql) {
        return Err(
            "`sqlite.explain`: chỉ nhận câu truy vấn chỉ đọc — sau khoảng trắng/comment đầu, \
             từ khoá đầu phải là `SELECT` hoặc `WITH`"
                .to_string(),
        );
    }
    let cau = format!("EXPLAIN QUERY PLAN {sql}");
    host.db_query(h, &cau, params)
}

/// `true` khi từ khoá đầu (sau khoảng trắng/comment đầu) là `SELECT` hoặc `WITH`.
fn la_truy_van_chi_doc(sql: &str) -> bool {
    match tu_khoa_dau(sql) {
        Some(k) => k.eq_ignore_ascii_case("select") || k.eq_ignore_ascii_case("with"),
        None => false,
    }
}

/// Từ khoá ASCII đầu tiên sau khi bỏ khoảng trắng và **các comment đầu** liên tiếp.
///
/// Comment dòng `--…` kéo tới `\n` (hoặc hết chuỗi); comment khối `/*…*/` tới `*/` — khối chưa
/// đóng ⇒ `None` (từ chối). Chỉ nhận chữ cái ASCII liền đầu; ký tự khác (`;`, `(`, số…) ⇒
/// `None`. Không cắt chuỗi/định danh vì chỉ xét **đầu** văn bản, trước mọi token SQL khác.
fn tu_khoa_dau(mut s: &str) -> Option<&str> {
    s = s.trim_start();
    loop {
        if let Some(rest) = s.strip_prefix("--") {
            let i = rest.find('\n')?;
            s = &rest[i + 1..];
        } else if let Some(rest) = s.strip_prefix("/*") {
            let i = rest.find("*/")?;
            s = &rest[i + 2..];
        } else {
            break;
        }
        s = s.trim_start();
    }
    let end = s
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(s.len());
    if end == 0 {
        None
    } else {
        Some(&s[..end])
    }
}
