// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `sqlite_a` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod sqliteappid {
    //! **VD-270 (sqlite-application-id)** — lõi **dùng chung** cho `sqlite.application_id`
    //! (interp ↔ VM byte-identical).
    //!
    //! Trả **`application_id`** — số nguyên **32-bit có dấu** lưu trong header tệp database (offset
    //! 68, big-endian) — đọc bằng `PRAGMA application_id`. Đây là metadata **chỉ đọc** dành cho
    //! **ứng dụng tự định danh** (ví dụ công cụ ngoài ghi một magic number): SQLite **không** dùng
    //! số này cho việc gì; DDL (`CREATE`/`DROP`/`ALTER`), ghi dữ liệu và truy vấn chỉ đọc **không**
    //! đổi giá trị. Database `:memory:` **mới** ⇒ `0`; giá trị **lưu theo tệp**, đóng rồi mở lại
    //! vẫn giữ nguyên. Mỗi lời gọi đọc **lại** từ connection — không cache ở tầng std.
    //!
    //! Hợp đồng chỉ nhận **`0..=i32::MAX`** (không âm). Đường std **không có** hàm ghi
    //! `application_id` — authorizer của host từ chối `PRAGMA application_id = …` (ghi là `Deny`,
    //! chỉ đọc không kèm giá trị mới được `Allow`) — nên giá trị âm/ngoài phạm vi chỉ xuất hiện ở
    //! tệp do **công cụ ngoài** tạo; khi đó trả `Err` **dữ liệu**, không trap, thay vì trả một `int`
    //! vi phạm hợp đồng không âm.
    //!
    //! **Đường đọc đã chọn**: `PRAGMA application_id` qua **đúng** `DbHost::db_query` hiện có (không
    //! thêm method host). **Probe** trên host trước khi mở quyền: cả `PRAGMA application_id` **và**
    //! `SELECT * FROM pragma_application_id` đều bị authorizer từ chối (mã SQLite 23) — TVF của
    //! pragma cũng đi qua kiểm tra `SQLITE_PRAGMA` với chính tên `application_id`. Host mở **đúng
    //! một mục đọc** `application_id` (không kèm giá trị) trong allow-list `SQLITE_PRAGMA`; ghi vẫn
    //! bị từ chối. Khi cả hai đường đều được phép, chọn `PRAGMA application_id`: đường thay thế
    //! `SELECT * FROM pragma_application_id` là table-valued function **có thể bị đối tượng schema
    //! người dùng trùng tên che** (probe xác nhận: bảng `pragma_application_id` trả `999`), còn
    //! pragma không có không gian tên đó nên hợp đồng không phụ thuộc schema người dùng. Cùng
    //! authorizer/chính sách chỉ đọc/trần/timeout/cache/statement như mọi lời gọi `db_query` khác;
    //! câu SQL là **hằng**, không bind tham số — không có bề mặt injection.
    //!
    //! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
    //! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, không đúng một
    //! hàng, ô không phải `SqlValue.Integer`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use crate::value::Value;

    /// Truy vấn hệ thống: `application_id` của database **chính** (`main`) — một cột
    /// `application_id`, đúng một hàng, ô INTEGER. Không bind tham số: câu **hằng**, không có đường
    /// nối chuỗi.
    ///
    /// `PRAGMA application_id` (không `temp.`/tên schema khác) đọc số của database chính; pragma chỉ
    /// đọc nên `db_query` chấp nhận. Không dùng `SELECT * FROM pragma_application_id`: TVF có thể bị
    /// bảng người dùng trùng tên che, còn pragma thì không.
    const CAU: &str = "PRAGMA application_id";

    /// Chạy [`CAU`] qua `host.db_query` và trả `application_id` của database chính.
    ///
    /// Hình dạng kết quả được kiểm **chặt**: đúng một cột, đúng một hàng, đúng một ô
    /// `SqlValue.Integer(i64)`; giá trị phải trong `0..=i32::MAX`. Lệch ⇒ `Err` dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<i64, String> {
        let (cot, hang) = host.db_query(h, CAU, &[])?;
        if cot.len() != 1 {
            return Err(format!(
            "`sqlite.application_id`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `application_id`)",
            cot.len()
        ));
        }
        match hang.as_slice() {
            // Pragma `application_id` luôn trả đúng một hàng; 0 hàng nghĩa là host/kết quả sai hình
            // dạng — `Err` dữ liệu, không đoán giá trị.
            [] => Err(
                "`sqlite.application_id`: truy vấn hệ thống không trả hàng (cần đúng 1)"
                    .to_string(),
            ),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.application_id`: hàng có {} ô (cần đúng 1)",
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
                                "`sqlite.application_id`: giá trị {n} ngoài phạm vi 0..={} \
                             (application_id là số 32-bit có dấu)",
                                i32::MAX
                            ));
                        }
                        Ok(*n)
                    }
                    _ => Err(
                        "`sqlite.application_id`: `application_id` không phải INTEGER".to_string(),
                    ),
                }
            }
            _ => Err(format!(
                "`sqlite.application_id`: truy vấn hệ thống trả {} hàng (cần đúng 1)",
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
            let mut h = host(&["application_id"], vec![vec![int(7)]]);
            assert_eq!(chay(&mut h, "db:x").expect("phải Ok"), 7);
            assert_eq!(
                h.sql.as_deref(),
                Some(CAU),
                "phải chạy đúng `PRAGMA application_id`"
            );
            assert_eq!(
                h.tham_so.as_deref().map(<[Value]>::len),
                Some(0),
                "pragma không bind tham số"
            );
            let mut h = host(&["application_id"], vec![vec![int(0)]]);
            assert_eq!(chay(&mut h, "db:x").expect("0 phải Ok"), 0);
            let mut h = host(&["application_id"], vec![vec![int(i64::from(i32::MAX))]]);
            assert_eq!(
                chay(&mut h, "db:x").expect("i32::MAX phải Ok"),
                i64::from(i32::MAX)
            );
            let mut h = host(&["application_id"], vec![vec![int(0x1234_5678)]]);
            assert_eq!(
                chay(&mut h, "db:x").expect("magic number phải Ok"),
                0x1234_5678
            );
        }

        #[test]
        fn hinh_dang_sai_la_err_du_lieu() {
            let mut h = host(&["application_id"], Vec::new());
            assert!(chay(&mut h, "db:x").is_err(), "0 hàng ⇒ Err");
            let mut h = host(&["application_id"], vec![vec![int(1)], vec![int(2)]]);
            assert!(chay(&mut h, "db:x").is_err(), "2 hàng ⇒ Err");
            let mut h = host(&["application_id", "extra"], vec![vec![int(1), int(2)]]);
            assert!(chay(&mut h, "db:x").is_err(), "2 cột ⇒ Err");
            let mut h = host(&["application_id"], vec![vec![int(1), int(2)]]);
            assert!(chay(&mut h, "db:x").is_err(), "2 ô ⇒ Err");
            let mut h = host(
                &["application_id"],
                vec![vec![Value::Enum(
                    "SqlValue".into(),
                    "Text".into(),
                    Vec::new(),
                )]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text ⇒ Err");
            let mut h = host(&["application_id"], vec![vec![Value::Int(1)]]);
            assert!(
                chay(&mut h, "db:x").is_err(),
                "ô không phải `SqlValue` ⇒ Err"
            );
        }

        #[test]
        fn ngoai_pham_vi_khong_am_la_err_du_lieu() {
            for n in [-1, i64::from(i32::MIN), i64::from(i32::MAX) + 1] {
                let mut h = host(&["application_id"], vec![vec![int(n)]]);
                assert!(
                    chay(&mut h, "db:x").is_err(),
                    "{n} ngoài 0..=i32::MAX ⇒ Err"
                );
            }
        }
    }
}

pub(crate) mod sqlitecols {
    //! **VD-270 (sqlite-columns)** — lõi **dùng chung** cho `sqlite.columns` (interp ↔ VM
    //! byte-identical).
    //!
    //! Trả **tên cột** của một **bảng hoặc view** trong **schema chính** (`main`) của connection,
    //! theo **thứ tự khai báo** (`cid` tăng dần), giữ nguyên hoa/thường như định nghĩa. Tên đối tượng
    //! được **bind** làm tham số (`?`) — không bao giờ nối chuỗi SQL — nên tên chứa dấu nháy/`;`/
    //! comment chỉ là tên, không có tác dụng SQL.
    //!
    //! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
    //! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
    //! timeout, cache schema/statement. `pragma_table_info` là table-valued function dựng sẵn của
    //! SQLite; probe xác nhận authorizer cho đọc (chỉ `dbstat`/trang thô bị chặn). Tham số schema
    //! **cố định** `'main'` nên bảng TEMP trùng tên **không** che bảng chính và không lọt vào kết quả.
    //! **Hạn chế đã biết**: SQLite cho đối tượng schema trùng tên `pragma_table_info` che TVF dựng
    //! sẵn; khi đó truy vấn lỗi ⇒ `Err(Error)` **dữ liệu** (không đọc nhầm/không trả dữ liệu sai).
    //!
    //! **Bảng/view không tồn tại ⇒ `Ok([])`** (pragma không trả hàng) — khác hẳn handle thiếu/đã
    //! đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến thành `Err(Error)`,
    //! **không** trap). Kết quả sai hình dạng (không đúng một cột `name`, ô không phải
    //! `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use crate::value::Value;

    /// Truy vấn hệ thống: một cột `name` TEXT theo đúng thứ tự khai báo (`cid`).
    ///
    /// - `?` là **tên bảng/view** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - `'main'` cố định: chỉ schema chính (không phải `temp`/schema gắn ngoài).
    /// - `ORDER BY cid`: thứ tự khai báo, **không** sắp theo tên; view cũng có `cid` theo thứ tự
    ///   cột của câu `SELECT`.
    const CAU: &str = "SELECT name FROM pragma_table_info(?, 'main') ORDER BY cid";

    /// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên cột **đúng thứ tự khai báo**.
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
    /// Bảng/view không tồn tại ⇒ pragma trả 0 hàng ⇒ `Ok(vec![])`.
    pub fn chay(host: &mut dyn DbHost, h: &str, table: &str) -> Result<Vec<String>, String> {
        // Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn
        // param (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
        let tham_so = [Value::Enum(
            "SqlValue".into(),
            "Text".into(),
            vec![Value::Str(table.to_string())],
        )];
        let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.columns`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
                cot.len()
            ));
        }
        let mut ten = Vec::with_capacity(hang.len());
        for (i, hang) in hang.into_iter().enumerate() {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.columns`: hàng {} có {} ô (cần đúng 1)",
                    i + 1,
                    hang.len()
                ));
            };
            match o {
                Value::Enum(e, v, args) if e == "SqlValue" && v == "Text" && args.len() == 1 => {
                    match &args[0] {
                        Value::Str(s) => ten.push(s.clone()),
                        _ => {
                            return Err(format!(
                                "`sqlite.columns`: hàng {} — `SqlValue.Text` không mang `str`",
                                i + 1
                            ))
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "`sqlite.columns`: hàng {} — tên cột không phải TEXT",
                        i + 1
                    ))
                }
            }
        }
        // **Không** sắp lại: `ORDER BY cid` là hợp đồng **thứ tự khai báo**. (Khác `sqlite.tables`,
        // nơi hợp đồng là thứ tự byte.)
        Ok(ten)
    }
}

pub(crate) mod sqlitedataver {
    //! **VD-270 (sqlite-data-version)** — lõi **dùng chung** cho `sqlite.data_version`
    //! (interp ↔ VM byte-identical).
    //!
    //! Trả **`data_version`** của connection, đọc bằng `PRAGMA data_version`. Đây là bộ đếm
    //! **theo từng connection** của SQLite: giá trị **tăng** khi **connection khác** commit thay đổi
    //! vào **cùng** database (ghi dữ liệu **hoặc** DDL đều tính — bất cứ thay đổi nào làm đổi tệp),
    //! và **không đổi** vì chính connection này ghi/DDL. Hệ quả quan trọng:
    //!
    //! - **`:memory:`**: mỗi connection mở `:memory:` có database **riêng biệt**, nên connection thứ
    //!   hai ghi vào database của nó **không** làm đổi giá trị của connection thứ nhất. Muốn quan sát
    //!   thay đổi phải mở **cùng một tệp** DB bằng hai connection.
    //! - **Giá trị khởi tạo** của connection mới là `1` (probe trên host hiện tại: `:memory:` mới và
    //!   connection mới trên tệp đều trả `Integer(1)`; đọc lặp lại vẫn `1`). Đây là chi tiết hiện
    //!   thực của SQLite, không phải hợp đồng: std chỉ hứa `int` **không âm**, không hứa mốc cụ thể.
    //! - **Mỗi lần gọi đọc lại** từ connection — không cache ở tầng std.
    //!
    //! **Đường đọc đã chọn**: `PRAGMA data_version` qua **đúng** `DbHost::db_query` hiện có (không
    //! thêm method host, không ghi/DDL). Probe cho thấy authorizer allow-list của host cho **đọc**
    //! `user_version`/`schema_version`/`data_version` (không kèm giá trị) và `db_query` chấp nhận
    //! câu này (`sqlite3_stmt_readonly` = true, trả đúng một hàng). Đường thay thế
    //! `SELECT * FROM pragma_data_version` cũng được phép, nhưng là table-valued function **có thể
    //! bị đối tượng schema người dùng trùng tên che** (probe: tạo bảng `pragma_data_version` ⇒ TVF
    //! trả `0` hàng, còn pragma vẫn `Integer(1)`); `PRAGMA data_version` không có không gian tên đó
    //! nên được chọn để hợp đồng không phụ thuộc schema người dùng. Cùng authorizer/chính sách chỉ
    //! đọc/trần/timeout/cache/statement như mọi lời gọi `db_query` khác; câu SQL là **hằng**, không
    //! bind tham số — không có bề mặt injection.
    //!
    //! Hợp đồng chỉ nhận **`0..=i32::MAX`** (không âm): bộ đếm của SQLite là số nguyên **có dấu**
    //! tăng dần; giá trị âm/ngoài phạm vi (dữ liệu/host lạ) ⇒ `Err` **dữ liệu**, không trap. Handle
    //! thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến thành
    //! `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, không đúng một hàng,
    //! ô không phải `SqlValue.Integer`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use crate::value::Value;

    /// Truy vấn hệ thống: `data_version` của connection — một cột `data_version`, đúng một hàng, ô
    /// INTEGER. Không bind tham số: câu **hằng**, không có đường nối chuỗi.
    ///
    /// `PRAGMA data_version` đọc bộ đếm theo connection; pragma chỉ đọc nên `db_query` chấp nhận.
    /// Không dùng `SELECT * FROM pragma_data_version`: TVF có thể bị bảng người dùng trùng tên che,
    /// còn pragma thì không.
    const CAU: &str = "PRAGMA data_version";

    /// Chạy [`CAU`] qua `host.db_query` và trả `data_version` của connection.
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột, đúng một hàng, đúng một ô `SqlValue.Integer(i64)`; giá trị phải trong
    /// `0..=i32::MAX`. Lệch ⇒ `Err` dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str) -> Result<i64, String> {
        let (cot, hang) = host.db_query(h, CAU, &[])?;
        if cot.len() != 1 {
            return Err(format!(
            "`sqlite.data_version`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `data_version`)",
            cot.len()
        ));
        }
        match hang.as_slice() {
            // Pragma `data_version` luôn trả đúng một hàng; 0 hàng nghĩa là host/kết quả sai hình
            // dạng — `Err` dữ liệu, không đoán giá trị.
            [] => Err(
                "`sqlite.data_version`: truy vấn hệ thống không trả hàng (cần đúng 1)".to_string(),
            ),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.data_version`: hàng có {} ô (cần đúng 1)",
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
                                "`sqlite.data_version`: giá trị {n} ngoài phạm vi 0..={} \
                             (bộ đếm data_version là số 32-bit có dấu)",
                                i32::MAX
                            ));
                        }
                        Ok(*n)
                    }
                    _ => {
                        Err("`sqlite.data_version`: `data_version` không phải INTEGER".to_string())
                    }
                }
            }
            _ => Err(format!(
                "`sqlite.data_version`: truy vấn hệ thống trả {} hàng (cần đúng 1)",
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
            let mut h = host(&["data_version"], vec![vec![int(1)]]);
            assert_eq!(chay(&mut h, "db:x").expect("phải Ok"), 1);
            assert_eq!(
                h.sql.as_deref(),
                Some(CAU),
                "phải chạy đúng `PRAGMA data_version`"
            );
            assert_eq!(
                h.tham_so.as_deref().map(<[Value]>::len),
                Some(0),
                "pragma không bind tham số"
            );
            let mut h = host(&["data_version"], vec![vec![int(0)]]);
            assert_eq!(chay(&mut h, "db:x").expect("0 phải Ok"), 0);
            let mut h = host(&["data_version"], vec![vec![int(i64::from(i32::MAX))]]);
            assert_eq!(
                chay(&mut h, "db:x").expect("i32::MAX phải Ok"),
                i64::from(i32::MAX)
            );
        }

        #[test]
        fn hinh_dang_sai_la_err_du_lieu() {
            let mut h = host(&["data_version"], Vec::new());
            assert!(chay(&mut h, "db:x").is_err(), "0 hàng ⇒ Err");
            let mut h = host(&["data_version"], vec![vec![int(1)], vec![int(2)]]);
            assert!(chay(&mut h, "db:x").is_err(), "2 hàng ⇒ Err");
            let mut h = host(&["data_version", "extra"], vec![vec![int(1), int(2)]]);
            assert!(chay(&mut h, "db:x").is_err(), "2 cột ⇒ Err");
            let mut h = host(&["data_version"], vec![vec![int(1), int(2)]]);
            assert!(chay(&mut h, "db:x").is_err(), "2 ô ⇒ Err");
            let mut h = host(
                &["data_version"],
                vec![vec![Value::Enum(
                    "SqlValue".into(),
                    "Text".into(),
                    Vec::new(),
                )]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text ⇒ Err");
            let mut h = host(&["data_version"], vec![vec![Value::Int(1)]]);
            assert!(
                chay(&mut h, "db:x").is_err(),
                "ô không phải `SqlValue` ⇒ Err"
            );
        }

        #[test]
        fn ngoai_pham_vi_khong_am_la_err_du_lieu() {
            for n in [-1, i64::from(i32::MIN), i64::from(i32::MAX) + 1, i64::MAX] {
                let mut h = host(&["data_version"], vec![vec![int(n)]]);
                assert!(
                    chay(&mut h, "db:x").is_err(),
                    "{n} ngoài 0..=i32::MAX ⇒ Err"
                );
            }
        }
    }
}
