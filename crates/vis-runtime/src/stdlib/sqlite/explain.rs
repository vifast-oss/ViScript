// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `sqlite_e` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod sqliteschemaver {
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
    use vis_core::value::Value;

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
                "`sqlite.schema_version`: truy vấn hệ thống không trả hàng (cần đúng 1)"
                    .to_string(),
            ),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.schema_version`: hàng có {} ô (cần đúng 1)",
                        hang.len()
                    ));
                };
                match o {
                    Value::Enum(__en)
                        if {
                            let e = &__en.enum_name;
                            let v = &__en.variant;
                            let args = &__en.args;
                            e == "SqlValue"
                                && v == "Integer"
                                && matches!(args.as_slice(), [Value::Int(_)])
                        } =>
                    {
                        let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                        {
                            let [Value::Int(n)] = args.as_slice() else {
                                unreachable!(
                                    "mẫu `matches!` ở trên đã bảo đảm đúng một `Value::Int`"
                                )
                            };
                            Ok(*n)
                        }
                    }
                    _ => Err(
                        "`sqlite.schema_version`: `schema_version` không phải INTEGER".to_string(),
                    ),
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
            Value::enum_val("SqlValue", "Integer", vec![Value::Int(n)])
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
                vec![vec![Value::enum_val("SqlValue", "Text", Vec::new())]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text ⇒ Err");
            let mut h = host(&["schema_version"], vec![vec![Value::Int(1)]]);
            assert!(
                chay(&mut h, "db:x").is_err(),
                "ô không phải `SqlValue` ⇒ Err"
            );
        }
    }
}

pub(crate) mod sqlitetables {
    //! **VD-270 (sqlite-tables)** — lõi **dùng chung** cho `sqlite.tables` (interp ↔ VM
    //! byte-identical).
    //!
    //! Trả **tên** bảng và view người dùng trong **schema chính** của connection, sắp theo **byte**
    //! (`ORDER BY name` — collation `BINARY` của SQLite so UTF-8 theo byte), bỏ đối tượng nội bộ có
    //! tiền tố `sqlite_` (không phân biệt hoa thường như `LIKE`; `_` được escape nên chỉ đúng tiền
    //! tố `sqlite_`, không nuốt tên kiểu `sqliteXtra`).
    //!
    //! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
    //! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
    //! timeout, cache schema/statement và bind params rỗng. `sqlite_master` là bảng hệ thống bình
    //! thường với authorizer (chỉ `dbstat`/trang thô bị chặn) — probe xác nhận đọc được.
    //!
    //! **Bảng TEMP không xuất hiện**: chúng nằm ở schema `temp` (`sqlite_temp_master`), không phải
    //! `sqlite_master` của schema chính; `tables` mô tả **schema chính** của connection (bảng tạm là
    //! dữ liệu phiên, không thuộc tệp DB). Xem test `bang_tam_va_noi_bo` ở
    //! `crates/vis-cli/tests/vd270_sqlite_tables.rs` — đây là hợp đồng **có chủ ý**, không phải sót.
    //!
    //! Mọi lỗi (handle thiếu/đã đóng/sai dạng, SQL/host lỗi, kết quả sai hình dạng) ⇒ `Err(String)`
    //! **dữ liệu** — tầng std biến thành `Err(Error)`, **không** trap. DB rỗng ⇒ `Ok([])`.

    use crate::builtin::DbHost;
    use vis_core::value::Value;

    /// Truy vấn hệ thống: một cột `name` TEXT, chỉ bảng/view người dùng của schema chính.
    ///
    /// `name NOT LIKE 'sqlite\_%' ESCAPE '\'`: `LIKE` của SQLite **không phân biệt hoa thường** với
    /// ASCII và `_` là wildcard một ký tự — escape để loại **đúng** tiền tố `sqlite_` (đối tượng nội
    /// bộ), không loại oan tên người dùng như `sqliteXtra`. `ORDER BY name` dùng collation `BINARY`
    /// mặc định ⇒ thứ tự **byte**, đúng hợp đồng.
    const CAU: &str = "SELECT name FROM sqlite_master \
                   WHERE type IN ('table','view') \
                     AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
                   ORDER BY name";

    /// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên (đã sắp byte lần nữa cho chắc).
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<Vec<String>, String> {
        let (cot, hang) = host.db_query(h, CAU, &[])?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.tables`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
                cot.len()
            ));
        }
        let mut ten = Vec::with_capacity(hang.len());
        for (i, hang) in hang.into_iter().enumerate() {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.tables`: hàng {} có {} ô (cần đúng 1)",
                    i + 1,
                    hang.len()
                ));
            };
            match o {
                Value::Enum(__en)
                    if {
                        let e = &__en.enum_name;
                        let v = &__en.variant;
                        let args = &__en.args;
                        e == "SqlValue" && v == "Text" && args.len() == 1
                    } =>
                {
                    let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                    {
                        match &args[0] {
                            Value::Str(s) => ten.push(s.clone()),
                            _ => {
                                return Err(format!(
                                    "`sqlite.tables`: hàng {} — `SqlValue.Text` không mang `str`",
                                    i + 1
                                ))
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "`sqlite.tables`: hàng {} — tên bảng/view không phải TEXT",
                        i + 1
                    ))
                }
            }
        }
        // SQL đã `ORDER BY name` (BINARY); sắp lại bằng `sort_unstable` (byte lexicographic) để hợp
        // đồng **không phụ thuộc** host/query — rẻ với số bảng thực tế và giữ đúng thứ tự byte.
        ten.sort_unstable();
        Ok(ten)
    }
}

pub(crate) mod sqliteuserver {
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
    use vis_core::value::Value;

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
            [] => Err(
                "`sqlite.user_version`: truy vấn hệ thống không trả hàng (cần đúng 1)".to_string(),
            ),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.user_version`: hàng có {} ô (cần đúng 1)",
                        hang.len()
                    ));
                };
                match o {
                    Value::Enum(__en)
                        if {
                            let e = &__en.enum_name;
                            let v = &__en.variant;
                            let args = &__en.args;
                            e == "SqlValue"
                                && v == "Integer"
                                && matches!(args.as_slice(), [Value::Int(_)])
                        } =>
                    {
                        let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                        {
                            let [Value::Int(n)] = args.as_slice() else {
                                unreachable!(
                                    "mẫu `matches!` ở trên đã bảo đảm đúng một `Value::Int`"
                                )
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
                    }
                    _ => {
                        Err("`sqlite.user_version`: `user_version` không phải INTEGER".to_string())
                    }
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
            Value::enum_val("SqlValue", "Integer", vec![Value::Int(n)])
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
                vec![vec![Value::enum_val("SqlValue", "Text", Vec::new())]],
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
}

pub(crate) mod sqlite_explain {
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
    use vis_core::value::Value;

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
}
