// SPDX-License-Identifier: Apache-2.0
//! **VD-270 (sqlite-database-list)** — unit test host cho `PRAGMA database_list`: probe
//! `sqlite3_stmt_readonly`, authorizer (mở đúng mục đọc), shape thật, và các đường vẫn bị chặn
//! (ghi pragma, `ATTACH`). Tách khỏi `tests.rs` để giữ luật R1 (≤ 1.500 dòng/tệp).

use super::*;

fn mo_db(d: &mut Db) -> String {
    d.db_open(":memory:", true, 5_000).expect("open :memory:")
}

/// Đọc một cột `SqlValue` từ hàng — trả chuỗi nếu là `Text`, số nếu là `Integer`.
fn o_text(rows: &[Vec<Value>], hang: usize, cot: usize) -> String {
    let Some((e, v, args)) = rows[hang][cot].as_enum() else {
        panic!("ô không phải enum: {rows:?}");
    };
    assert_eq!(e, "SqlValue");
    assert_eq!(v, "Text");
    let [Value::Str(s)] = args else {
        panic!("ô không phải Text(str): {rows:?}")
    };
    s.clone()
}

fn o_int(rows: &[Vec<Value>], hang: usize, cot: usize) -> i64 {
    let Some((e, v, args)) = rows[hang][cot].as_enum() else {
        panic!("ô không phải enum: {rows:?}");
    };
    assert_eq!(e, "SqlValue");
    assert_eq!(v, "Integer");
    let [Value::Int(n)] = args else {
        panic!("ô không phải Integer(int): {rows:?}")
    };
    *n
}

#[test]
fn probe_pragma_database_list_readonly_that() {
    // **Probe VD-270 (database-list)**: khác `journal_mode`, pragma này là read-only thật nên
    // `query` không cần ngoại lệ hẹp; chỉ authorizer phải mở mục đọc.
    let c = Connection::open_in_memory().unwrap();
    let stmt = c.prepare("PRAGMA database_list").unwrap();
    assert!(
        stmt.readonly(),
        "probe: `PRAGMA database_list` phải là read-only (SQLite)"
    );
}

#[test]
fn pragma_database_list_doc_qua_query_shape_that() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    let (cot, hang) = d
        .db_query(&h, "PRAGMA database_list", &[])
        .expect("đọc database_list qua query phải được");
    assert_eq!(
        cot,
        vec!["seq".to_string(), "name".to_string(), "file".to_string()],
        "shape cột thật của pragma"
    );
    assert_eq!(
        hang.len(),
        2,
        "probe bundled SQLite: :memory: mới có main + temp"
    );
    assert_eq!(o_int(&hang, 0, 0), 0, "main.seq");
    assert_eq!(o_text(&hang, 0, 1), "main");
    assert_eq!(o_text(&hang, 0, 2), "", "main.file rỗng với :memory:");
    assert_eq!(o_int(&hang, 1, 0), 1, "temp.seq");
    assert_eq!(o_text(&hang, 1, 1), "temp");
    assert_eq!(o_text(&hang, 1, 2), "", "temp.file rỗng");
}

#[test]
fn attach_va_ghi_pragma_van_bi_chan() {
    let mut d = Db::moi(false);
    let h = mo_db(&mut d);
    // Ghi pragma (kể cả dạng không hợp lệ `= value`) bị authorizer từ chối.
    let e = d
        .db_query(&h, "PRAGMA database_list = 1", &[])
        .expect_err("ghi database_list phải bị chặn");
    assert!(e.contains("quyền (authorizer)"), "{e}");
    // ATTACH/DETACH vẫn bị chặn ở mọi chế độ — danh sách không thể tăng qua đường người dùng.
    let e = d
        .db_execute(&h, "ATTACH DATABASE ':memory:' AS aux", &[])
        .expect_err("ATTACH phải bị chặn");
    assert!(e.contains("quyền (authorizer)"), "{e}");
    let (_, hang) = d
        .db_query(&h, "PRAGMA database_list", &[])
        .expect("đọc lại database_list");
    assert_eq!(hang.len(), 2, "ATTACH bị chặn nên vẫn chỉ main + temp");
}

#[test]
fn probe_sqlite_that_co_muc_aux_sau_attach() {
    // Probe **SQLite thật** (không qua host): khi ATTACH thành công, pragma có thêm hàng `aux`
    // với `seq` kế tiếp — hợp đồng `database_list` chỉ trả nguyên hàng pragma, nên lõi xử lý
    // đúng nếu host tương lai cho phép ATTACH.
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch("ATTACH DATABASE ':memory:' AS aux;")
        .expect("raw SQLite cho phép ATTACH");
    // Database `temp` chỉ xuất hiện trong pragma khi schema temp được cấp (host đặt
    // `temp_store=MEMORY` nên thấy `temp` ngay; connection raw cần dùng TEMP trước).
    c.execute_batch("CREATE TEMP TABLE tt (a INTEGER);")
        .expect("tạo bảng TEMP");
    let mut st = c.prepare("PRAGMA database_list").unwrap();
    let mut stt = st.query([]).unwrap();
    let mut dem = 0;
    let mut ten = Vec::new();
    while let Some(row) = stt.next().unwrap() {
        dem += 1;
        let seq = row.get::<_, i64>(0).unwrap();
        assert_eq!(seq, dem - 1, "probe: seq là chỉ số database liên tiếp");
        ten.push(row.get::<_, String>(1).unwrap());
    }
    assert_eq!(dem, 3, "probe: main + temp + aux — thấy {ten:?}");
    assert_eq!(ten, vec!["main", "temp", "aux"], "probe thứ tự seq");
}
