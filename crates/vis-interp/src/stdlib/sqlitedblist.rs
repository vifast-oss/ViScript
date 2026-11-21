// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-database-list)** — lõi **dùng chung** cho `sqlite.database_list`
//! (interp ↔ VM byte-identical).
//!
//! Trả **danh sách database** đang gắn với connection, đọc bằng `PRAGMA database_list` — câu
//! **hằng**, **không** bind tham số. Mỗi hàng của pragma thành **một `RowSet`** cùng hình dạng
//! `sqlite.query` (`columns` + `rows`): `columns = [seq, name, file]`, `rows` đúng **một** hàng
//! theo cùng thứ tự cột, ô giữ nguyên `SqlValue` (`Integer(seq)`, `Text(name)`, `Text(file)`).
//! Danh sách sắp theo `seq` **tăng dần** (sắp lại bằng `sort_by_key`, không tin thứ tự host).
//!
//! **Probe shape thật** (bundled SQLite 3.50.2, cả hai engine): `:memory:` **mới** trả **hai**
//! hàng — `main` (`seq = 0`, `file = ""`) và `temp` (`seq = 1`, `file = ""`); database **tệp**
//! trả `main` với `file` = đường dẫn tệp và `temp` với `file = ""`. SQLite luôn kèm schema `temp`
//! trong pragma này, nên hợp đồng **không** lọc `temp`. Hợp đồng ban đầu kỳ vọng `:memory:` mới
//! chỉ một mục `main`; probe cho thấy điều đó không đúng với bản SQLite bundled — test khoá shape
//! thật.
//!
//! **Đường đọc đã chọn**: `PRAGMA database_list` qua **đúng** `DbHost::db_query` hiện có (không
//! thêm method host). **Probe authorizer** trước khi mở quyền: cả `PRAGMA database_list` **và**
//! TVF `pragma_database_list` đều bị từ chối (mã SQLite 23) vì cùng đi qua kiểm tra
//! `SQLITE_PRAGMA`; host mở **đúng một mục đọc** `database_list` (không kèm giá trị) trong
//! allow-list. Ghi `PRAGMA database_list = …` (không hợp lệ) và `ATTACH`/`DETACH` vẫn bị từ chối
//! — hàm này **không** mở thêm quyền ghi/gắn DB. `sqlite3_stmt_readonly` của pragma là **true**
//! nên **không** cần nới chính sách chỉ đọc như `journal_mode`. Chọn **pragma** thay TVF vì TVF
//! có thể bị bảng người dùng trùng tên che, còn pragma thì không.
//!
//! **Hệ quả ATTACH (gap có chủ ý)**: vì host cấm `ATTACH` ở mọi chế độ, `sqlite.execute` không
//! thể gắn database mới; danh sách từ đường người dùng chỉ có `main` + `temp`. Lõi vẫn xử lý
//! đúng hàng `aux` khi host trả về (unit test host giả). Không nới chính sách bảo mật chỉ để làm
//! test xanh.
//!
//! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
//! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng 3 cột
//! `seq,name,file`, hàng không đúng 3 ô, `seq` không phải INTEGER, `name`/`file` không phải
//! TEXT) cũng là `Err` dữ liệu, không panic. Host trả **0 hàng** ⇒ `Ok([])` (không có database
//! nào để mô tả); hình dạng cột vẫn được kiểm.

use super::chung::rowset_record;
use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: danh sách database gắn với connection. Câu **hằng**, không bind tham số —
/// không có bề mặt injection. `PRAGMA database_list` (không giá trị) là metadata **chỉ đọc**.
const CAU: &str = "PRAGMA database_list";

/// Tên cột cố định của `PRAGMA database_list` (SQLite ≥ 3.x): `seq`, `name`, `file`.
const COT: [&str; 3] = ["seq", "name", "file"];

/// Chạy [`CAU`] qua `host.db_query` và trả **mỗi hàng** thành một `RowSet` (đã sắp theo `seq`).
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**:
/// đúng ba cột `seq,name,file`; mỗi hàng đúng ba ô; `seq` là `SqlValue.Integer(i64)`, `name` và
/// `file` là `SqlValue.Text(str)` (file rỗng hợp lệ). Lệch ⇒ `Err` dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<Vec<Value>, String> {
    let (cot, hang) = host.db_query(h, CAU, &[])?;
    if cot.as_slice() != COT {
        return Err(format!(
            "`sqlite.database_list`: truy vấn hệ thống trả cột {:?} (cần đúng `seq,name,file`)",
            cot
        ));
    }
    // Giữ `(seq, hàng)` để sắp ổn định theo `seq`; `seq` bằng nhau giữ thứ tự host (sort ổn định).
    let mut ds: Vec<(i64, Vec<Value>)> = Vec::with_capacity(hang.len());
    for (i, hang) in hang.into_iter().enumerate() {
        let [o_seq, o_ten, o_file] = hang.as_slice() else {
            return Err(format!(
                "`sqlite.database_list`: hàng {} có {} ô (cần đúng 3: seq,name,file)",
                i + 1,
                hang.len()
            ));
        };
        let seq = so_seq(o_seq, i)?;
        kiem_text(o_ten, i, "name")?;
        kiem_text(o_file, i, "file")?;
        ds.push((seq, hang));
    }
    // Sắp theo `seq` (hợp đồng không phụ thuộc host/pragma); với số database thực tế (main, temp,
    // các aux) chi phí không đáng kể. `sort_by_key` ổn định nên hàng cùng `seq` giữ thứ tự host.
    ds.sort_by_key(|(seq, _)| *seq);
    Ok(ds
        .into_iter()
        .map(|(_, hang)| rowset_record(cot.clone(), vec![hang]))
        .collect())
}

/// `seq` phải là `SqlValue.Integer(i64)`; trả giá trị để sắp. Sai ⇒ `Err` dữ liệu.
fn so_seq(o: &Value, i: usize) -> Result<i64, String> {
    match o {
        Value::Enum(e, v, args) if e == "SqlValue" && v == "Integer" => match args.as_slice() {
            [Value::Int(n)] => Ok(*n),
            _ => Err(format!(
                "`sqlite.database_list`: hàng {} — `seq` (`SqlValue.Integer`) không mang đúng một `int`",
                i + 1
            )),
        },
        _ => Err(format!(
            "`sqlite.database_list`: hàng {} — `seq` không phải INTEGER",
            i + 1
        )),
    }
}

/// `name`/`file` phải là `SqlValue.Text(str)`; file rỗng (`""`) hợp lệ. Sai ⇒ `Err` dữ liệu.
fn kiem_text(o: &Value, i: usize, ten: &str) -> Result<(), String> {
    match o {
        Value::Enum(e, v, args) if e == "SqlValue" && v == "Text" => match args.as_slice() {
            [Value::Str(_)] => Ok(()),
            _ => Err(format!(
                "`sqlite.database_list`: hàng {} — `{ten}` (`SqlValue.Text`) không mang đúng một `str`",
                i + 1
            )),
        },
        _ => Err(format!(
            "`sqlite.database_list`: hàng {} — `{ten}` không phải TEXT",
            i + 1
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Host giả: trả đúng `(cột, hàng)` đã định; ghi lại SQL/params để kiểm hợp đồng.
    struct HostGia {
        cot: Vec<String>,
        hang: Vec<Vec<Value>>,
        sql: Option<String>,
        tham_so: Option<Vec<Value>>,
    }

    impl DbHost for HostGia {
        fn db_open(&mut self, _: &str, _: bool, _: i64) -> Result<String, String> {
            unreachable!()
        }
        fn db_close(&mut self, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn db_execute(&mut self, _: &str, _: &str, _: &[Value]) -> Result<i64, String> {
            unreachable!()
        }
        fn db_query(
            &mut self,
            _: &str,
            sql: &str,
            params: &[Value],
        ) -> Result<(Vec<String>, Vec<Vec<Value>>), String> {
            self.sql = Some(sql.to_string());
            self.tham_so = Some(params.to_vec());
            Ok((self.cot.clone(), self.hang.clone()))
        }
        fn db_in_transaction(&mut self, _: &str) -> Result<bool, String> {
            unreachable!()
        }
        fn db_cleanup(&mut self) -> Result<u64, String> {
            unreachable!()
        }
        fn db_path_of(&mut self, _: &str) -> Result<String, String> {
            unreachable!()
        }
    }

    fn host(cot: &[&str], hang: Vec<Vec<Value>>) -> HostGia {
        HostGia {
            cot: cot.iter().map(|s| s.to_string()).collect(),
            hang,
            sql: None,
            tham_so: None,
        }
    }

    fn text(s: &str) -> Value {
        Value::Enum(
            "SqlValue".into(),
            "Text".into(),
            vec![Value::Str(s.to_string())],
        )
    }

    fn int(n: i64) -> Value {
        Value::Enum("SqlValue".into(), "Integer".into(), vec![Value::Int(n)])
    }

    fn hang_mot(seq: i64, ten: &str, file: &str) -> Vec<Value> {
        vec![int(seq), text(ten), text(file)]
    }

    #[test]
    fn doc_pragma_va_moi_hang_thanh_rowset_sap_seq() {
        // Host trả **ngược** thứ tự seq để chứng minh hàm tự sắp, kèm một `aux` (ATTACH) mà đường
        // người dùng không tạo được vì authorizer cấm ATTACH.
        let mut h = host(
            &COT,
            vec![
                hang_mot(2, "aux", "/tmp/aux.db"),
                hang_mot(0, "main", "/tmp/chinh.db"),
                hang_mot(1, "temp", ""),
            ],
        );
        let ds = chay(&mut h, "db:x").expect("phải Ok");
        assert_eq!(
            h.sql.as_deref(),
            Some(CAU),
            "phải chạy đúng `PRAGMA database_list`"
        );
        assert_eq!(
            h.tham_so.as_deref().map(<[Value]>::len),
            Some(0),
            "pragma không bind tham số"
        );
        assert_eq!(ds.len(), 3, "mỗi hàng thành một RowSet");
        let ky_vong = [
            (0, "main", "/tmp/chinh.db"),
            (1, "temp", ""),
            (2, "aux", "/tmp/aux.db"),
        ];
        for (v, (seq, ten, file)) in ds.iter().zip(ky_vong) {
            assert_eq!(
                v.show(),
                format!(
                    "RowSet(columns: [seq, name, file], rows: [[Integer({seq}), Text({ten}), Text({file})]])"
                ),
                "mỗi phần tử là RowSet 3 cột đúng một hàng, SqlValue nguyên văn"
            );
        }
    }

    #[test]
    fn host_tra_rong_la_ok_rong() {
        let mut h = host(&COT, Vec::new());
        let ds = chay(&mut h, "db:x").expect("0 hàng vẫn Ok (không có database để mô tả)");
        assert!(ds.is_empty(), "phải trả danh sách rỗng");
    }

    #[test]
    fn hinh_dang_sai_la_err_du_lieu() {
        // Sai số cột / sai tên cột.
        let mut h = host(&["seq", "name"], vec![]);
        assert!(chay(&mut h, "db:x").is_err(), "2 cột ⇒ Err");
        let mut h = host(&["seq", "file", "name"], vec![]);
        assert!(chay(&mut h, "db:x").is_err(), "sai thứ tự cột ⇒ Err");
        // Sai số ô.
        let mut h = host(&COT, vec![vec![int(0), text("main")]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 ô ⇒ Err");
        let mut h = host(&COT, vec![vec![int(0), text("main"), text(""), int(1)]]);
        assert!(chay(&mut h, "db:x").is_err(), "4 ô ⇒ Err");
        // `seq` sai kiểu/thiếu đối số/không phải SqlValue.
        let mut h = host(&COT, vec![vec![text("0"), text("main"), text("")]]);
        assert!(chay(&mut h, "db:x").is_err(), "seq Text ⇒ Err");
        let mut h = host(&COT, vec![vec![int(0), text("main"), text("")]]);
        assert!(chay(&mut h, "db:x").is_ok(), "hàng chuẩn ⇒ Ok");
        let mut h = host(
            &COT,
            vec![vec![
                Value::Enum("SqlValue".into(), "Integer".into(), Vec::new()),
                text("main"),
                text(""),
            ]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Integer thiếu đối số ⇒ Err");
        let mut h = host(
            &COT,
            vec![vec![Value::Str("0".into()), text("main"), text("")]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "ô không phải SqlValue ⇒ Err");
        // `name`/`file` sai kiểu/thiếu đối số/Null.
        let mut h = host(&COT, vec![vec![int(0), int(1), text("")]]);
        assert!(chay(&mut h, "db:x").is_err(), "name Integer ⇒ Err");
        let mut h = host(&COT, vec![vec![int(0), text("main"), int(0)]]);
        assert!(chay(&mut h, "db:x").is_err(), "file Integer ⇒ Err");
        let mut h = host(
            &COT,
            vec![vec![
                int(0),
                Value::Enum("SqlValue".into(), "Text".into(), Vec::new()),
                text(""),
            ]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Text thiếu đối số ⇒ Err");
        let mut h = host(
            &COT,
            vec![vec![
                int(0),
                Value::Enum("SqlValue".into(), "Text".into(), vec![Value::Int(1)]),
                text(""),
            ]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Text sai kiểu đối số ⇒ Err");
        let mut h = host(
            &COT,
            vec![vec![
                int(0),
                Value::Enum("SqlValue".into(), "Null".into(), Vec::new()),
                text(""),
            ]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "name Null ⇒ Err");
        let mut h = host(
            &COT,
            vec![vec![
                int(0),
                Value::Enum("Other".into(), "Text".into(), vec![Value::Str("x".into())]),
                text(""),
            ]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "enum khác ⇒ Err");
    }
}
