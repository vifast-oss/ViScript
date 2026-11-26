// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `sqlite_b` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod sqlitedblist {
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

    use super::super::super::chung::rowset_record;
    use crate::builtin::DbHost;
    use vis_core::value::Value;

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
            Value::Enum(__en)
                if {
                    let e = &__en.enum_name;
                    let v = &__en.variant;
                    let _args = &__en.args;
                    e == "SqlValue" && v == "Integer"
                } =>
            {
                let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                match args.as_slice() {
            [Value::Int(n)] => Ok(*n),
            _ => Err(format!(
                "`sqlite.database_list`: hàng {} — `seq` (`SqlValue.Integer`) không mang đúng một `int`",
                i + 1
            )),
        }
            }
            _ => Err(format!(
                "`sqlite.database_list`: hàng {} — `seq` không phải INTEGER",
                i + 1
            )),
        }
    }

    /// `name`/`file` phải là `SqlValue.Text(str)`; file rỗng (`""`) hợp lệ. Sai ⇒ `Err` dữ liệu.
    fn kiem_text(o: &Value, i: usize, ten: &str) -> Result<(), String> {
        match o {
            Value::Enum(__en)
                if {
                    let e = &__en.enum_name;
                    let v = &__en.variant;
                    let _args = &__en.args;
                    e == "SqlValue" && v == "Text"
                } =>
            {
                let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                match args.as_slice() {
            [Value::Str(_)] => Ok(()),
            _ => Err(format!(
                "`sqlite.database_list`: hàng {} — `{ten}` (`SqlValue.Text`) không mang đúng một `str`",
                i + 1
            )),
        }
            }
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
            Value::enum_val("SqlValue", "Text", vec![Value::Str(s.to_string())])
        }

        fn int(n: i64) -> Value {
            Value::enum_val("SqlValue", "Integer", vec![Value::Int(n)])
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
                    Value::enum_val("SqlValue", "Integer", Vec::new()),
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
                    Value::enum_val("SqlValue", "Text", Vec::new()),
                    text(""),
                ]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text thiếu đối số ⇒ Err");
            let mut h = host(
                &COT,
                vec![vec![
                    int(0),
                    Value::enum_val("SqlValue", "Text", vec![Value::Int(1)]),
                    text(""),
                ]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text sai kiểu đối số ⇒ Err");
            let mut h = host(
                &COT,
                vec![vec![
                    int(0),
                    Value::enum_val("SqlValue", "Null", Vec::new()),
                    text(""),
                ]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "name Null ⇒ Err");
            let mut h = host(
                &COT,
                vec![vec![
                    int(0),
                    Value::enum_val("Other", "Text", vec![Value::Str("x".into())]),
                    text(""),
                ]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "enum khác ⇒ Err");
        }
    }
}

pub(crate) mod sqliteexists {
    //! **VD-270 (sqlite-table-exists)** — lõi **dùng chung** cho `sqlite.table_exists`
    //! (interp ↔ VM byte-identical).
    //!
    //! Trả `true` **khi và chỉ khi** connection có **bảng hoặc view** tên **đúng** trong **schema
    //! chính** (`main`). Truy vấn:
    //!
    //! ```sql
    //! SELECT 1 FROM sqlite_master
    //!  WHERE type IN ('table','view') AND name = ? LIMIT 1
    //! ```
    //!
    //! `?` là **tên bind** (`SqlValue.Text`) — tên đối tượng là **dữ liệu**, không phải SQL: không
    //! nối chuỗi, nên tên kiểu `t' OR 1=1 --`/`t; DROP TABLE t` chỉ là tên (không có đối tượng như
    //! vậy ⇒ `Ok(false)`), không có tác dụng phụ. Hàm **chỉ đọc**, đi qua **đúng** đường
    //! `DbHost::db_query` hiện có (không thêm method host): cùng authorizer, chính sách chỉ đọc
    //! (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler timeout, cache
    //! schema/statement. `sqlite_master` là bảng hệ thống bình thường với authorizer (chỉ
    //! `dbstat`/trang thô bị chặn) — `sqlite.tables`/`columns`/`schema_sql` đã đi cùng đường đọc này.
    //!
    //! So khớp **phân biệt hoa/thường**: `name = ?` dùng collation `BINARY` mặc định của
    //! `sqlite_master` (`CREATE TABLE Bang` ⇒ tra `Bang` = `true`, tra `bang`/`BANG` = `false` dù
    //! SQLite vẫn cho tham chiếu bảng tên `bang` trong câu SQL khác). Đây là hợp đồng **có chủ ý**
    //! của truy vấn, không phải sót.
    //!
    //! **Không có hàng ⇒ `Ok(false)`**. Đối tượng nội bộ `sqlite_%` **không** bị đặc cách ngoài
    //! truy vấn: `sqlite_sequence` (sinh bởi `AUTOINCREMENT`, `type = 'table'`) ⇒ `true`;
    //! `sqlite_master`/`sqlite_schema` không có hàng trong chính nó ⇒ `false`; index tự động
    //! `sqlite_autoindex_%` có `type = 'index'` nên **không** tính là bảng/view ⇒ `false`. Trigger
    //! cùng tên bảng cũng không tính (`type = 'trigger'`). Đối tượng TEMP nằm ở schema `temp`
    //! (`sqlite_temp_master`), **không** thuộc schema chính ⇒ `false` — kể cả khi trùng tên bảng
    //! chính (bảng chính vẫn `true`).
    //!
    //! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
    //! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, quá một
    //! hàng, ô không phải `SqlValue.Integer(1)`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use vis_core::value::Value;

    /// Truy vấn hệ thống: một cột hằng `1`, chỉ `table`/`view` của **schema chính** tra theo **tên**.
    ///
    /// - `?` là **tên đối tượng** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - `type IN ('table','view')`: đúng hợp đồng "bảng **hoặc view**"; index/trigger không tính.
    /// - `name = ?`: collation `BINARY` mặc định ⇒ so **byte**, **phân biệt hoa/thường**.
    /// - `LIMIT 1`: tối đa một hàng (bảng/view chia sẻ không gian tên nên tên là duy nhất).
    const CAU: &str =
        "SELECT 1 FROM sqlite_master WHERE type IN ('table','view') AND name = ? LIMIT 1";

    /// Chạy [`CAU`] qua `host.db_query`; `true` ⇔ có ít nhất một hàng.
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột; 0 hàng ⇒ `false`; đúng 1 hàng, đúng 1 ô `SqlValue.Integer(1)` ⇒ `true`; lệch ⇒ `Err`
    /// dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str, name: &str) -> Result<bool, String> {
        // Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn
        // param (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
        let tham_so = [Value::enum_val(
            "SqlValue",
            "Text",
            vec![Value::Str(name.to_string())],
        )];
        let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.table_exists`: truy vấn hệ thống trả {} cột (cần đúng 1)",
                cot.len()
            ));
        }
        match hang.as_slice() {
            // Không có bảng/view tên đó trong schema chính.
            [] => Ok(false),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.table_exists`: hàng có {} ô (cần đúng 1)",
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
                                && matches!(args.as_slice(), [Value::Int(1)])
                        } =>
                    {
                        let (_e, _v, _args) = (&__en.enum_name, &__en.variant, &__en.args);
                        {
                            Ok(true)
                        }
                    }
                    Value::Enum(__en)
                        if {
                            let e = &__en.enum_name;
                            let v = &__en.variant;
                            let args = &__en.args;
                            e == "SqlValue" && v == "Integer" && args.len() == 1
                        } =>
                    {
                        let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                        {
                            Err(format!(
                                "`sqlite.table_exists`: hằng truy vấn là `{}` (cần `1`)",
                                args[0].show()
                            ))
                        }
                    }
                    _ => Err(
                        "`sqlite.table_exists`: `SELECT 1` không trả `SqlValue.Integer(1)`"
                            .to_string(),
                    ),
                }
            }
            _ => Err(format!(
                "`sqlite.table_exists`: truy vấn hệ thống trả {} hàng (cần tối đa 1)",
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
            hang: Result<Vec<Vec<Value>>, String>,
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
                self.hang.clone().map(|hang| (self.cot.clone(), hang))
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

        fn host(cot: &[&str], hang: Result<Vec<Vec<Value>>, String>) -> HostGia {
            HostGia {
                cot: cot.iter().map(|s| s.to_string()).collect(),
                hang,
                sql: None,
                tham_so: None,
            }
        }

        fn mot() -> Value {
            Value::enum_val("SqlValue", "Integer", vec![Value::Int(1)])
        }

        #[test]
        fn tra_theo_ten_bind_va_co_hang_la_true() {
            let mut h = host(&["1"], Ok(vec![vec![mot()]]));
            assert!(chay(&mut h, "db:x", "t").expect("phải Ok"));
            assert_eq!(
                h.sql.as_deref(),
                Some(CAU),
                "phải chạy đúng câu sqlite_master"
            );
            match &h.tham_so.as_deref() {
                Some([o]) => {
                    let Some((e, v, args)) = o.as_enum() else {
                        panic!("param phải là `SqlValue.Text(tên)`, được {o:?}")
                    };
                    assert_eq!(
                        (e.as_str(), v.as_str()),
                        ("SqlValue", "Text"),
                        "tên phải bind `SqlValue.Text(\"t\")`, được {o:?}"
                    );
                    assert!(
                        matches!(args, [Value::Str(s)] if s == "t"),
                        "tên phải bind `SqlValue.Text(\"t\")`, được {o:?}"
                    );
                }
                khac => panic!("param phải là `SqlValue.Text(tên)`, được {khac:?}"),
            }
        }

        #[test]
        fn khong_co_hang_la_false_va_loi_host_duoc_truyen_nguyen() {
            let mut h = host(&["1"], Ok(Vec::new()));
            assert!(!chay(&mut h, "db:x", "thieu").expect("phải Ok"));
            let mut h = host(&["1"], Err("handle đã đóng".to_string()));
            assert_eq!(
                chay(&mut h, "db:x", "t").expect_err("host lỗi ⇒ Err"),
                "handle đã đóng"
            );
        }

        #[test]
        fn hinh_dang_sai_la_err_du_lieu() {
            let mut h = host(&["1", "extra"], Ok(vec![vec![mot(), mot()]]));
            assert!(chay(&mut h, "db:x", "t").is_err(), "hai cột ⇒ Err");
            let mut h = host(&["1"], Ok(vec![vec![]]));
            assert!(chay(&mut h, "db:x", "t").is_err(), "hàng 0 ô ⇒ Err");
            let mut h = host(&["1"], Ok(vec![vec![Value::Int(1)]]));
            assert!(chay(&mut h, "db:x", "t").is_err(), "ô int trần ⇒ Err");
            let mut h = host(
                &["1"],
                Ok(vec![vec![Value::enum_val(
                    "SqlValue",
                    "Integer",
                    vec![Value::Int(0)],
                )]]),
            );
            assert!(chay(&mut h, "db:x", "t").is_err(), "hằng 0 ⇒ Err");
            let mut h = host(&["1"], Ok(vec![vec![mot()], vec![mot()]]));
            assert!(chay(&mut h, "db:x", "t").is_err(), "hai hàng ⇒ Err");
        }
    }
}

pub(crate) mod sqliteidx {
    //! **VD-270 (sqlite-indexes)** — lõi **dùng chung** cho `sqlite.indexes` (interp ↔ VM
    //! byte-identical).
    //!
    //! Trả **tên index do người dùng tạo** của một **bảng** trong **schema chính** (`main`) của
    //! connection, sắp theo **byte** (`ORDER BY name`, collation `BINARY` mặc định), giữ nguyên
    //! hoa/thường như đã khai. Index tự động của SQLite (`sqlite_autoindex_%` — sinh bởi `UNIQUE`/
    //! `PRIMARY KEY`) và index của bảng khác **không** xuất hiện.
    //!
    //! Tên bảng được **bind** làm tham số (`?`) — không bao giờ nối chuỗi SQL — nên tên chứa dấu
    //! nháy/`;`/comment chỉ là tên, không có tác dụng SQL. `tbl_name = ? COLLATE BINARY` so **byte**
    //! (phân biệt hoa/thường) với tên đã khai trong `sqlite_master`, đúng như hợp đồng của truy vấn.
    //!
    //! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
    //! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
    //! timeout, cache schema/statement. `sqlite_master` của schema chính là bảng hệ thống bình thường
    //! với authorizer (chỉ `dbstat`/trang thô bị chặn); `sqlite.tables`/`sqlite.columns` đã đi cùng
    //! đường đọc này.
    //!
    //! **Bảng/view không tồn tại ⇒ `Ok([])`** (không có hàng `type='index'`); **view** không thể có
    //! index nên cũng `Ok([])`. Bảng TEMP nằm ở schema `temp` (`sqlite_temp_master`), không thuộc
    //! `sqlite_master` của schema chính nên index của nó không lọt vào kết quả. Khác hẳn handle
    //! thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến thành
    //! `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột `name`, ô không phải
    //! `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use vis_core::value::Value;

    /// Truy vấn hệ thống: một cột `name` TEXT, chỉ index người dùng của bảng trong schema chính.
    ///
    /// - `?` là **tên bảng** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - `type = 'index'`: chỉ index; bảng/view/trigger không lọt.
    /// - `tbl_name = ? COLLATE BINARY`: đúng bảng đích, so **byte** (phân biệt hoa/thường).
    /// - `name NOT LIKE 'sqlite_autoindex_%'`: loại index tự động của SQLite. Tiền tố `sqlite_` là
    ///   **dành riêng** cho SQLite nên không thể là index người dùng.
    /// - `ORDER BY name`: collation `BINARY` mặc định ⇒ thứ tự **byte**.
    const CAU: &str = "SELECT name FROM sqlite_master \
                   WHERE type = 'index' \
                     AND tbl_name = ? COLLATE BINARY \
                     AND name NOT LIKE 'sqlite_autoindex_%' \
                   ORDER BY name";

    /// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên index (đã sắp byte lần nữa cho chắc).
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str, table: &str) -> Result<Vec<String>, String> {
        // Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn
        // param (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
        let tham_so = [Value::enum_val(
            "SqlValue",
            "Text",
            vec![Value::Str(table.to_string())],
        )];
        let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.indexes`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
                cot.len()
            ));
        }
        let mut ten = Vec::with_capacity(hang.len());
        for (i, hang) in hang.into_iter().enumerate() {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.indexes`: hàng {} có {} ô (cần đúng 1)",
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
                                    "`sqlite.indexes`: hàng {} — `SqlValue.Text` không mang `str`",
                                    i + 1
                                ))
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "`sqlite.indexes`: hàng {} — tên index không phải TEXT",
                        i + 1
                    ))
                }
            }
        }
        // SQL đã `ORDER BY name` (BINARY); sắp lại bằng `sort_unstable` (byte lexicographic) để hợp
        // đồng **không phụ thuộc** host/query — rẻ với số index thực tế và giữ đúng thứ tự byte.
        ten.sort_unstable();
        Ok(ten)
    }
}
