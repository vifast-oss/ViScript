// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-user-version)** — lõi **dùng chung** cho `sqlite.user_version`
//! (interp ↔ VM byte-identical).
//!
//! Trả **`user_version`** — số nguyên **ứng dụng tự định nghĩa** lưu trong header tệp database
//! (offset 60, 32-bit có dấu) — đọc bằng `PRAGMA user_version`. Đây là metadata **chỉ đọc**:
//! SQLite **không** dùng số này cho việc gì; DDL (`CREATE`/`DROP`/`ALTER`), ghi dữ liệu và
//! `VACUUM` **không** đổi giá trị. Database `:memory:` **mới** ⇒ `0`; giá trị **lưu theo tệp**,
//! đóng rồi mở lại vẫn giữ nguyên. Mỗi lời gọi đọc **lại** từ connection — không cache ở tầng
//! std.
//!
//! Hợp đồng chỉ nhận **`0..=i32::MAX`** (không âm). Đường std **không có** hàm ghi
//! `user_version` — authorizer của host từ chối `PRAGMA user_version = …` (ghi là `Deny`, chỉ
//! đọc không kèm giá trị mới được `Allow`) — nên giá trị âm/ngoài phạm vi chỉ xuất hiện ở tệp do
//! **công cụ ngoài** tạo; khi đó trả `Err` **dữ liệu**, không trap, thay vì trả một `int` vi phạm
//! hợp đồng không âm.
//!
//! **Đường đọc đã chọn**: `PRAGMA user_version` qua **đúng** `DbHost::db_query` hiện có (không
//! thêm method host). Probe cho thấy authorizer allow-list của host cho **đọc**
//! `user_version`/`schema_version`/`data_version` (không kèm giá trị) và `db_query` chấp nhận
//! câu này (`sqlite3_stmt_readonly` = true, trả đúng một hàng). Đường thay thế
//! `SELECT * FROM pragma_user_version` cũng được phép, nhưng là table-valued function **có thể
//! bị đối tượng schema người dùng trùng tên che**; `PRAGMA user_version` không có không gian tên
//! đó nên được chọn để hợp đồng không phụ thuộc schema người dùng. Cùng authorizer/chính sách
//! chỉ đọc/trần/timeout/cache/statement như mọi lời gọi `db_query` khác; câu SQL là **hằng**,
//! không bind tham số — không có bề mặt injection.
//!
//! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
//! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, không đúng một
//! hàng, ô không phải `SqlValue.Integer`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: `user_version` của database **chính** (`main`) — một cột `user_version`,
/// đúng một hàng, ô INTEGER. Không bind tham số: câu **hằng**, không có đường nối chuỗi.
///
/// `PRAGMA user_version` (không `temp.`/tên schema khác) đọc số của database chính; pragma chỉ
/// đọc nên `db_query` chấp nhận. Không dùng `SELECT * FROM pragma_user_version`: TVF có thể bị
/// bảng người dùng trùng tên che, còn pragma thì không.
const CAU: &str = "PRAGMA user_version";

/// Chạy [`CAU`] qua `host.db_query` và trả `user_version` của database chính.
///
/// Hình dạng kết quả được kiểm **chặt**: đúng một cột, đúng một hàng, đúng một ô
/// `SqlValue.Integer(i64)`; giá trị phải trong `0..=i32::MAX`. Lệch ⇒ `Err` dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<i64, String> {
    let (cot, hang) = host.db_query(h, CAU, &[])?;
    if cot.len() != 1 {
        return Err(format!(
            "`sqlite.user_version`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `user_version`)",
            cot.len()
        ));
    }
    match hang.as_slice() {
        // Pragma `user_version` luôn trả đúng một hàng; 0 hàng nghĩa là host/kết quả sai hình
        // dạng — `Err` dữ liệu, không đoán giá trị.
        [] => {
            Err("`sqlite.user_version`: truy vấn hệ thống không trả hàng (cần đúng 1)".to_string())
        }
        [hang] => {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.user_version`: hàng có {} ô (cần đúng 1)",
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
                    if !(0..=i64::from(i32::MAX)).contains(n) {
                        return Err(format!(
                            "`sqlite.user_version`: giá trị {n} ngoài phạm vi 0..={} \
                             (user_version là số 32-bit có dấu)",
                            i32::MAX
                        ));
                    }
                    Ok(*n)
                }
                _ => Err("`sqlite.user_version`: `user_version` không phải INTEGER".to_string()),
            }
        }
        _ => Err(format!(
            "`sqlite.user_version`: truy vấn hệ thống trả {} hàng (cần đúng 1)",
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
        let mut h = host(&["user_version"], vec![vec![int(7)]]);
        assert_eq!(chay(&mut h, "db:x").expect("phải Ok"), 7);
        assert_eq!(
            h.sql.as_deref(),
            Some(CAU),
            "phải chạy đúng `PRAGMA user_version`"
        );
        assert_eq!(
            h.tham_so.as_deref().map(<[Value]>::len),
            Some(0),
            "pragma không bind tham số"
        );
        let mut h = host(&["user_version"], vec![vec![int(0)]]);
        assert_eq!(chay(&mut h, "db:x").expect("0 phải Ok"), 0);
        let mut h = host(&["user_version"], vec![vec![int(i64::from(i32::MAX))]]);
        assert_eq!(
            chay(&mut h, "db:x").expect("i32::MAX phải Ok"),
            i64::from(i32::MAX)
        );
    }

    #[test]
    fn hinh_dang_sai_la_err_du_lieu() {
        let mut h = host(&["user_version"], Vec::new());
        assert!(chay(&mut h, "db:x").is_err(), "0 hàng ⇒ Err");
        let mut h = host(&["user_version"], vec![vec![int(1)], vec![int(2)]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 hàng ⇒ Err");
        let mut h = host(&["user_version", "extra"], vec![vec![int(1), int(2)]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 cột ⇒ Err");
        let mut h = host(&["user_version"], vec![vec![int(1), int(2)]]);
        assert!(chay(&mut h, "db:x").is_err(), "2 ô ⇒ Err");
        let mut h = host(
            &["user_version"],
            vec![vec![Value::Enum(
                "SqlValue".into(),
                "Text".into(),
                Vec::new(),
            )]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Text ⇒ Err");
        let mut h = host(&["user_version"], vec![vec![Value::Int(1)]]);
        assert!(
            chay(&mut h, "db:x").is_err(),
            "ô không phải `SqlValue` ⇒ Err"
        );
    }

    #[test]
    fn ngoai_pham_vi_khong_am_la_err_du_lieu() {
        for n in [-1, i64::from(i32::MIN), i64::from(i32::MAX) + 1] {
            let mut h = host(&["user_version"], vec![vec![int(n)]]);
            assert!(
                chay(&mut h, "db:x").is_err(),
                "{n} ngoài 0..=i32::MAX ⇒ Err"
            );
        }
    }
}
