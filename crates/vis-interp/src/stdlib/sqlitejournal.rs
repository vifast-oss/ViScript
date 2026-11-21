// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (sqlite-journal-mode)** — lõi **dùng chung** cho `sqlite.journal_mode`
//! (interp ↔ VM byte-identical).
//!
//! Trả **chuỗi chế độ journal hiện tại** của connection, đọc bằng `PRAGMA journal_mode` (không
//! tham số). Đây là metadata **chỉ đọc** của SQLite: giá trị là một trong `delete`/`truncate`/
//! `persist`/`memory`/`wal`/`off` tuỳ chế độ hiện tại; database `:memory:` mới ⇒ `memory`,
//! database **tệp mới** ⇒ `delete`. **Đọc lặp không đổi mode** — pragma không tham số không đặt
//! cấu hình. Hợp đồng **không** hứa danh sách đóng: chuỗi lạ vẫn được trả nguyên văn (không ép
//! kiểu/không kiểm tập giá trị), chỉ kiểm **hình dạng** kết quả.
//!
//! **Đường đọc đã chọn**: `PRAGMA journal_mode` qua **đúng** `DbHost::db_query` hiện có (không
//! thêm method host). **Probe** authorizer trước khi mở quyền: cả `PRAGMA journal_mode` **và**
//! `SELECT * FROM pragma_journal_mode` đều bị từ chối (mã SQLite 23) vì đều đi qua kiểm tra
//! `SQLITE_PRAGMA` với chính tên `journal_mode`. Host mở **đúng một mục đọc** `journal_mode`
//! (không kèm giá trị) trong allow-list `SQLITE_PRAGMA`; ghi `PRAGMA journal_mode = …` vẫn bị từ
//! chối. **Probe thêm**: sau khi authorizer cho qua, `PRAGMA journal_mode` (không giá trị) vẫn bị
//! chính sách `query` chỉ-đọc của host từ chối vì SQLite đánh dấu statement này **không**
//! read-only (cùng câu có thể đổi mode khi có giá trị); host chỉ mở **đúng dạng hằng**
//! `PRAGMA journal_mode` qua một phép kiểm hẹp (`la_pragma_journal_mode_chi_doc`) — không thêm
//! method host, không nới bất kỳ dạng ghi/khác. Khi cả hai đường đều được phép, chọn **pragma** vì
//! TVF `pragma_journal_mode` là table-valued function **có thể bị đối tượng schema người dùng
//! trùng tên che** (probe: bảng `pragma_journal_mode` trả `wal`), còn pragma không có không gian
//! tên đó nên hợp đồng không phụ thuộc schema người dùng. Cùng authorizer/chính sách chỉ
//! đọc/trần/timeout/cache/statement như mọi lời gọi `db_query` khác; câu SQL là **hằng**, không
//! bind tham số — không có bề mặt injection.
//!
//! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
//! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, không đúng một
//! hàng, ô không phải `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

use crate::builtin::DbHost;
use crate::value::Value;

/// Truy vấn hệ thống: chế độ journal hiện tại của connection — một cột `journal_mode`, đúng một
/// hàng, ô TEXT. Không bind tham số: câu **hằng**, không có đường nối chuỗi.
///
/// `PRAGMA journal_mode` (không tham số) chỉ **đọc** chế độ hiện tại; authorizer cho qua mục đọc
/// (không giá trị) và host mở **đúng dạng hằng** này cho `db_query` dù SQLite đánh dấu statement
/// không read-only. Ghi `PRAGMA journal_mode = …` vẫn bị authorizer từ chối. Không dùng
/// `SELECT * FROM pragma_journal_mode`: TVF có thể bị bảng người dùng trùng tên che, còn pragma
/// thì không.
const CAU: &str = "PRAGMA journal_mode";

/// Chạy [`CAU`] qua `host.db_query` và trả chế độ journal hiện tại.
///
/// Hình dạng kết quả được kiểm **chặt**: đúng một cột, đúng một hàng, đúng một ô
/// `SqlValue.Text(str)`; chuỗi trả **nguyên văn** (không hạ hoa/thường, không kiểm tập giá trị —
/// hợp đồng không hứa danh sách đóng). Lệch ⇒ `Err` dữ liệu (không panic).
pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<String, String> {
    let (cot, hang) = host.db_query(h, CAU, &[])?;
    if cot.len() != 1 {
        return Err(format!(
            "`sqlite.journal_mode`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `journal_mode`)",
            cot.len()
        ));
    }
    match hang.as_slice() {
        // Pragma `journal_mode` luôn trả đúng một hàng; 0 hàng nghĩa là host/kết quả sai hình
        // dạng — `Err` dữ liệu, không đoán giá trị.
        [] => {
            Err("`sqlite.journal_mode`: truy vấn hệ thống không trả hàng (cần đúng 1)".to_string())
        }
        [hang] => {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.journal_mode`: hàng có {} ô (cần đúng 1)",
                    hang.len()
                ));
            };
            match o {
                Value::Enum(e, v, args)
                    if e == "SqlValue"
                        && v == "Text"
                        && matches!(args.as_slice(), [Value::Str(_)]) =>
                {
                    let [Value::Str(s)] = args.as_slice() else {
                        unreachable!("mẫu `matches!` ở trên đã bảo đảm đúng một `Value::Str`")
                    };
                    Ok(s.clone())
                }
                _ => Err("`sqlite.journal_mode`: `journal_mode` không phải TEXT".to_string()),
            }
        }
        _ => Err(format!(
            "`sqlite.journal_mode`: truy vấn hệ thống trả {} hàng (cần đúng 1)",
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

    #[test]
    fn doc_pragma_va_tra_text_nguyen_van() {
        for mode in ["delete", "truncate", "persist", "memory", "wal", "off"] {
            let mut h = host(&["journal_mode"], vec![vec![text(mode)]]);
            assert_eq!(
                chay(&mut h, "db:x").expect("phải Ok"),
                mode,
                "phải trả nguyên văn mode `{mode}`"
            );
            assert_eq!(
                h.sql.as_deref(),
                Some(CAU),
                "phải chạy đúng `PRAGMA journal_mode`"
            );
            assert_eq!(
                h.tham_so.as_deref().map(<[Value]>::len),
                Some(0),
                "pragma không bind tham số"
            );
        }
        // Hợp đồng **không** hứa danh sách đóng: chuỗi lạ vẫn trả nguyên văn, không ép kiểu.
        let mut h = host(&["journal_mode"], vec![vec![text("custom-mode")]]);
        assert_eq!(
            chay(&mut h, "db:x").expect("chuỗi lạ vẫn Ok"),
            "custom-mode"
        );
        // Không hạ hoa/thường/trim.
        let mut h = host(&["journal_mode"], vec![vec![text(" WAL ")]]);
        assert_eq!(chay(&mut h, "db:x").expect("phải Ok"), " WAL ");
    }

    #[test]
    fn hinh_dang_sai_la_err_du_lieu() {
        let mut h = host(&["journal_mode"], Vec::new());
        assert!(chay(&mut h, "db:x").is_err(), "0 hàng ⇒ Err");
        let mut h = host(
            &["journal_mode"],
            vec![vec![text("delete")], vec![text("delete")]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "2 hàng ⇒ Err");
        let mut h = host(
            &["journal_mode", "extra"],
            vec![vec![text("delete"), text("x")]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "2 cột ⇒ Err");
        let mut h = host(
            &["journal_mode"],
            vec![vec![text("delete"), text("delete")]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "2 ô ⇒ Err");
        let mut h = host(&["journal_mode"], vec![vec![int(1)]]);
        assert!(chay(&mut h, "db:x").is_err(), "Integer ⇒ Err");
        let mut h = host(
            &["journal_mode"],
            vec![vec![Value::Enum(
                "SqlValue".into(),
                "Null".into(),
                Vec::new(),
            )]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Null ⇒ Err");
        let mut h = host(&["journal_mode"], vec![vec![Value::Str("delete".into())]]);
        assert!(
            chay(&mut h, "db:x").is_err(),
            "ô không phải `SqlValue` ⇒ Err"
        );
        let mut h = host(
            &["journal_mode"],
            vec![vec![Value::Enum(
                "Other".into(),
                "Text".into(),
                vec![Value::Str("delete".into())],
            )]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "enum khác ⇒ Err");
        let mut h = host(
            &["journal_mode"],
            vec![vec![Value::Enum(
                "SqlValue".into(),
                "Text".into(),
                Vec::new(),
            )]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Text thiếu đối số ⇒ Err");
        let mut h = host(
            &["journal_mode"],
            vec![vec![Value::Enum(
                "SqlValue".into(),
                "Text".into(),
                vec![Value::Int(1)],
            )]],
        );
        assert!(chay(&mut h, "db:x").is_err(), "Text sai kiểu đối số ⇒ Err");
    }
}
