// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-schema-version)** — lõi **dùng chung** cho `sqlite.schema_version`
//! (interp ↔ VM byte-identical).
//!
//! Trả **`schema_version`** (schema cookie) của **schema chính** (`main`) của connection, đọc
//! bằng `PRAGMA schema_version`. Đây là metadata **chỉ đọc** của SQLite: con số SQLite tăng mỗi
//! khi **schema chính** đổi (DDL `CREATE`/`DROP`/`ALTER` table/view/index/trigger); **không** đổi
//! khi chỉ đọc/ghi dữ liệu hay chạy truy vấn. Mỗi lời gọi đọc **lại** từ connection nên DDL trên
//! **cùng connection** làm giá trị **tăng** ngay ở lời gọi kế tiếp — không cache ở tầng std.
//!
//! Giá trị khởi tạo của database `:memory:` mới là `0`; database trên tệp có thể bắt đầu ở bất kỳ
//! số **không âm** nào tuỳ lịch sử DDL của tệp — hợp đồng chỉ hứa `int` **không âm** theo SQLite,
//! không hứa một mốc cụ thể. DDL trên bảng **TEMP** chỉ đổi `temp.schema_version`, **không** đổi
//! cookie `main` — nên hàm này (schema chính) **không** tăng vì bảng tạm. Đây là hợp đồng **có
//! chủ ý**: hàm mô tả **schema chính** của connection.
//!
//! **Đường đọc đã chọn**: `PRAGMA schema_version` qua **đúng** `DbHost::db_query` hiện có (không
//! thêm method host). Probe cho thấy authorizer allow-list của host cho **đọc**
//! `user_version`/`schema_version`/`data_version` (không kèm giá trị) và `db_query` chấp nhận
//! câu này (`sqlite3_stmt_readonly` = true, trả đúng một hàng); host cũng dùng chính pragma này
//! cho cache schema. Đường thay thế `SELECT * FROM pragma_schema_version` cũng được phép, nhưng
//! là table-valued function **có thể bị đối tượng schema người dùng trùng tên che**; `PRAGMA
//! schema_version` không có không gian tên đó nên được chọn để hợp đồng không phụ thuộc schema
//! người dùng. Cùng authorizer/chính sách chỉ đọc/trần/timeout/cache/statement như mọi lời gọi
//! `db_query` khác.
//!
//! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
//! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, không đúng một
//! hàng, ô không phải `SqlValue.Integer`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: schema cookie của **schema chính** (`main`) — một cột `schema_version`,
/// đúng một hàng, ô INTEGER. Không bind tham số: câu **hằng**, không có đường nối chuỗi.
///
/// `PRAGMA schema_version` (không `temp.`/tên schema khác) đọc cookie của schema chính; pragma
/// chỉ đọc nên `db_query` chấp nhận. Không dùng `SELECT * FROM pragma_schema_version`: TVF có
/// thể bị bảng người dùng trùng tên che, còn pragma thì không.
const CAU: &str = "PRAGMA schema_version";

/// Chạy [`CAU`] qua `host.db_query` và trả `schema_version` của schema chính.
///
/// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
/// một cột, đúng một hàng, đúng một ô `SqlValue.Integer(i64)`; lệch ⇒ `Err` dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<i64, String> {
    let (cot, hang) = host.db_query(h, CAU, &[])?;
    if cot.len() != 1 {
        return Err(format!(
            "`sqlite.schema_version`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `schema_version`)",
            cot.len()
        ));
    }
    match hang.as_slice() {
        // Pragma `schema_version` luôn trả đúng một hàng; 0 hàng nghĩa là host/kết quả sai hình
        // dạng — `Err` dữ liệu, không đoán giá trị.
        [] => Err(
            "`sqlite.schema_version`: truy vấn hệ thống không trả hàng (cần đúng 1)".to_string(),
        ),
        [hang] => {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.schema_version`: hàng có {} ô (cần đúng 1)",
                    hang.len()
                ));
            };
            match o {
                Value::Enum(e, v, args)
                    if e == "SqlValue"
                        && v == "Integer"
                        && matches!(args.as_slice(), [Value::Int(_)]) =>
                {
                    let [Value::Int(n)] = args.as_slice() else {
                        unreachable!("mẫu `matches!` ở trên đã bảo đảm đúng một `Value::Int`")
                    };
                    Ok(*n)
                }
                _ => {
                    Err("`sqlite.schema_version`: `schema_version` không phải INTEGER".to_string())
                }
            }
        }
        _ => Err(format!(
            "`sqlite.schema_version`: truy vấn hệ thống trả {} hàng (cần đúng 1)",
            hang.len()
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

    fn int(n: i64) -> Value {
        Value::Enum("SqlValue".into(), "Integer".into(), vec![Value::Int(n)])
    }

    #[test]
    fn doc_pragma_va_tra_integer() {
        let mut h = host(&["schema_version"], vec![vec![int(7)]]);
        assert_eq!(chay(&mut h, "db:x").expect("phải Ok"), 7);
        assert_eq!(
            h.sql.as_deref(),
            Some(CAU),
            "phải chạy đúng `PRAGMA schema_version`"
        );
        assert_eq!(
            h.tham_so.as_deref().map(<[Value]>::len),
            Some(0),
            "pragma không bind tham số"
        );
    }

    #[test]
    fn hinh_dang_sai_la_err_du_lieu() {
        let mut h = host(&["schema_version"], Vec::new());
        assert!(chay(&mut h, "db:x").is_err(), "0 hàng ⇒ Err");
        let mut h = host(&["schema_version"], vec![vec![int(1)], vec![int(2)]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 hàng ⇒ Err");
        let mut h = host(&["schema_version", "extra"], vec![vec![int(1), int(2)]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 cột ⇒ Err");
        let mut h = host(&["schema_version"], vec![vec![int(1), int(2)]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 ô ⇒ Err");
        let mut h = host(
            &["schema_version"],
            vec![vec![Value::Enum(
                "SqlValue".into(),
                "Text".into(),
                Vec::new(),
            )]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Text ⇒ Err");
        let mut h = host(&["schema_version"], vec![vec![Value::Int(1)]]);
        assert!(
            chay(&mut h, "db:x").is_err(),
            "ô không phải `SqlValue` ⇒ Err"
        );
    }
}
