// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-journal-mode)** — unit test host cho `PRAGMA journal_mode` **không giá trị**:
//! probe `sqlite3_stmt_readonly`, phép kiểm hẹp mở đúng dạng hằng, và đường đọc qua `db_query`.
//! Tách khỏi `tests.rs` để giữ luật R1 (≤ 1.500 dòng/tệp).

use super::*;

fn mo_db(d: &mut Db) -> String {
    d.db_open(":memory:", true, 5_000).expect("open :memory:")
}

fn o_text(rows: &[Vec<Value>]) -> String {
    assert_eq!(rows.len(), 1, "phải có đúng 1 hàng: {rows:?}");
    let row = &rows[0];
    assert_eq!(row.len(), 1, "phải có đúng 1 ô: {row:?}");
    let Some((e, v, args)) = row[0].as_enum() else {
        panic!("ô không phải enum: {row:?}")
    };
    assert_eq!(e, "SqlValue");
    assert_eq!(v, "Text");
    let [Value::Str(s)] = args else {
        panic!("ô không phải Text(str): {row:?}")
    };
    s.clone()
}

#[test]
fn probe_pragma_journal_mode_khong_readonly_con_tvf_thi_doc_duoc() {
    // **Probe VD-270 (journal-mode)**: SQLite đánh dấu `PRAGMA journal_mode` (không giá trị) là
    // **không** read-only, nên `query` từ chối trước step; TVF `pragma_journal_mode` là SELECT nên
    // read-only nhưng **có thể bị bảng người dùng trùng tên che** (xem test CLI). Vì vậy host chỉ
    // mở **đúng dạng hằng** pragma qua `la_pragma_journal_mode_chi_doc`.
    let c = Connection::open_in_memory().unwrap();
    let stmt = c.prepare("PRAGMA journal_mode").unwrap();
    assert!(
        !stmt.readonly(),
        "probe: `PRAGMA journal_mode` phải là không read-only (SQLite)"
    );
    let tvf = c.prepare("SELECT * FROM pragma_journal_mode").unwrap();
    assert!(tvf.readonly(), "probe: TVF là SELECT nên read-only");
}

#[test]
fn nhan_dung_dang_pragma_journal_mode_chi_doc() {
    for sql in [
        "PRAGMA journal_mode",
        " pragma   JOURNAL_MODE ",
        "PRAGMA journal_mode;",
        "PRAGMA\njournal_mode",
    ] {
        assert!(
            la_pragma_journal_mode_chi_doc(sql),
            "phải nhận đúng dạng chỉ đọc: {sql:?}"
        );
    }
    for sql in [
        "PRAGMA journal_mode = WAL",
        "PRAGMA journal_mode=DELETE",
        "PRAGMA main.journal_mode",
        "PRAGMA journal_mode -- x",
        "PRAGMA journal_mode; -- x",
        "PRAGMA journal_modes",
        "SELECT 1",
        "",
    ] {
        assert!(
            !la_pragma_journal_mode_chi_doc(sql),
            "không được nhận: {sql:?}"
        );
    }
}

#[test]
fn pragma_journal_mode_doc_qua_query_va_ghi_bi_chan() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let (cot, hang) = d
        .db_query(&h, "PRAGMA journal_mode", &[])
        .expect("đọc journal_mode qua query phải được");
    assert_eq!(cot, vec!["journal_mode".to_string()], "đúng một cột");
    assert_eq!(o_text(&hang), "memory", "`:memory:` mới ⇒ Text(memory)");
    // Đọc lặp (kể cả hoa/thường/khoảng trắng) không đổi mode.
    let (_, hang2) = d
        .db_query(&h, " pragma   JOURNAL_MODE ", &[])
        .expect("đọc lặp phải được");
    assert_eq!(o_text(&hang2), "memory", "đọc lặp không đổi mode");
    // Ghi pragma vẫn bị authorizer từ chối; mode không đổi.
    for sql in ["PRAGMA journal_mode = WAL", "PRAGMA journal_mode=DELETE"] {
        let e = d.db_query(&h, sql, &[]).unwrap_err();
        assert!(e.contains("quyền (authorizer)"), "{sql}: {e}");
    }
    let (_, hang3) = d
        .db_query(&h, "PRAGMA journal_mode", &[])
        .expect("đọc sau khi ghi bị chặn");
    assert_eq!(o_text(&hang3), "memory", "ghi bị chặn không đổi mode");
}
