// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `sqlite_d` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod sqliteidxexists {
    //! **VD-270 (sqlite-index-exists)** — lõi **dùng chung** cho `sqlite.index_exists`
    //! (interp ↔ VM byte-identical).
    //!
    //! Trả `true` **khi và chỉ khi** connection có **index** tên **đúng** trong **schema chính**
    //! (`main`). Truy vấn:
    //!
    //! ```sql
    //! SELECT 1 FROM sqlite_master
    //!  WHERE type = 'index' AND name = ? LIMIT 1
    //! ```
    //!
    //! `?` là **tên bind** (`SqlValue.Text`) — tên index là **dữ liệu**, không phải SQL: không nối
    //! chuỗi, nên tên kiểu `i' OR 1=1 --`/`i; DROP INDEX i` chỉ là tên (không có index như vậy ⇒
    //! `Ok(false)`), không có tác dụng phụ. Hàm **chỉ đọc**, đi qua **đúng** đường `DbHost::db_query`
    //! hiện có (không thêm method host): cùng authorizer, chính sách chỉ đọc
    //! (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler timeout, cache
    //! schema/statement. `sqlite_master` là bảng hệ thống bình thường với authorizer (chỉ
    //! `dbstat`/trang thô bị chặn) — `sqlite.table_exists`/`indexes`/`index_columns` đã đi cùng
    //! đường đọc này.
    //!
    //! So khớp **phân biệt hoa/thường**: `name = ?` dùng collation `BINARY` mặc định của
    //! `sqlite_master` (`CREATE INDEX "I_Bang"` ⇒ tra `I_Bang` = `true`, tra `i_bang`/`I_BANG` =
    //! `false` dù SQLite vẫn cho tham chiếu index tên khác hoa/thường trong câu SQL khác). Đây là
    //! hợp đồng **có chủ ý** của truy vấn, không phải sót.
    //!
    //! **Không có hàng ⇒ `Ok(false)`** — gồm **tên thiếu** và **tên rỗng** (không index nào có tên
    //! rỗng nên truy vấn không trả hàng; không cần đặc cách). Chỉ `type = 'index'` nên **bảng**,
    //! **view**, **trigger** cùng tên **không** tính. Index tự động `sqlite_autoindex_%` **không** bị
    //! đặc cách ngoài truy vấn: nó có hàng `type = 'index'` trong `sqlite_master` (tên thật, tham
    //! chiếu được) nên **`true` khi đúng tên**; sai hoa/thường/thiếu hậu tố ⇒ `false`. Index TEMP
    //! nằm ở schema `temp` (`sqlite_temp_master`), **không** thuộc schema chính ⇒ `false` — kể cả
    //! khi trùng tên index chính (index chính vẫn `true`, không bị TEMP che).
    //!
    //! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
    //! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột, quá một
    //! hàng, ô không phải `SqlValue.Integer(1)`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use vis_core::value::Value;

    /// Truy vấn hệ thống: một cột hằng `1`, chỉ `index` của **schema chính** tra theo **tên**.
    ///
    /// - `?` là **tên index** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - `type = 'index'`: đúng hợp đồng "index"; bảng/view/trigger cùng tên không tính.
    /// - `name = ?`: collation `BINARY` mặc định ⇒ so **byte**, **phân biệt hoa/thường**.
    /// - `LIMIT 1`: tối đa một hàng (index name là duy nhất trong một schema).
    const CAU: &str = "SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = ? LIMIT 1";

    /// Chạy [`CAU`] qua `host.db_query`; `true` ⇔ có ít nhất một hàng.
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột; 0 hàng ⇒ `false`; đúng 1 hàng, đúng 1 ô `SqlValue.Integer(1)` ⇒ `true`; lệch ⇒ `Err`
    /// dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str, name: &str) -> Result<bool, String> {
        // Tên index là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn param
        // (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
        let tham_so = [Value::enum_val(
            "SqlValue",
            "Text",
            vec![Value::Str(name.to_string())],
        )];
        let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.index_exists`: truy vấn hệ thống trả {} cột (cần đúng 1)",
                cot.len()
            ));
        }
        match hang.as_slice() {
            // Không có index tên đó trong schema chính (kể cả tên rỗng).
            [] => Ok(false),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.index_exists`: hàng có {} ô (cần đúng 1)",
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
                                "`sqlite.index_exists`: hằng truy vấn là `{}` (cần `1`)",
                                args[0].show()
                            ))
                        }
                    }
                    _ => Err(
                        "`sqlite.index_exists`: `SELECT 1` không trả `SqlValue.Integer(1)`"
                            .to_string(),
                    ),
                }
            }
            _ => Err(format!(
                "`sqlite.index_exists`: truy vấn hệ thống trả {} hàng (cần tối đa 1)",
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
            assert!(chay(&mut h, "db:x", "i").expect("phải Ok"));
            assert_eq!(
                h.sql.as_deref(),
                Some(CAU),
                "phải chạy đúng câu sqlite_master"
            );
            assert!(
                h.sql
                    .as_deref()
                    .is_some_and(|s| s.contains("type = 'index'")),
                "truy vấn phải lọc đúng `type = 'index'`"
            );
            match &h.tham_so.as_deref() {
                Some([o]) => {
                    let Some((e, v, args)) = o.as_enum() else {
                        panic!("param phải là `SqlValue.Text(tên)`, được {o:?}")
                    };
                    assert_eq!(
                        (e.as_str(), v.as_str()),
                        ("SqlValue", "Text"),
                        "tên phải bind `SqlValue.Text(\"i\")`, được {o:?}"
                    );
                    assert!(
                        matches!(args, [Value::Str(s)] if s == "i"),
                        "tên phải bind `SqlValue.Text(\"i\")`, được {o:?}"
                    );
                }
                khac => panic!("param phải là `SqlValue.Text(tên)`, được {khac:?}"),
            }
        }

        #[test]
        fn khong_co_hang_la_false_ke_ca_ten_rong_va_loi_host_duoc_truyen_nguyen() {
            let mut h = host(&["1"], Ok(Vec::new()));
            assert!(!chay(&mut h, "db:x", "thieu").expect("phải Ok"));
            assert!(
                matches!(
                    h.tham_so.as_deref(),
                    Some([o])
                        if matches!(o.as_enum(), Some((e, v, args))
                            if e == "SqlValue" && v == "Text"
                                && matches!(args, [Value::Str(s)] if s == "thieu"))
                ),
                "tên thiếu vẫn bind `SqlValue.Text(\"thieu\")`, được {:?}",
                h.tham_so
            );
            let mut h = host(&["1"], Ok(Vec::new()));
            assert!(!chay(&mut h, "db:x", "").expect("tên rỗng ⇒ Ok(false)"));
            let mut h = host(&["1"], Err("handle đã đóng".to_string()));
            assert_eq!(
                chay(&mut h, "db:x", "i").expect_err("host lỗi ⇒ Err"),
                "handle đã đóng"
            );
        }

        #[test]
        fn hinh_dang_sai_la_err_du_lieu() {
            let mut h = host(&["1", "extra"], Ok(vec![vec![mot(), mot()]]));
            assert!(chay(&mut h, "db:x", "i").is_err(), "hai cột ⇒ Err");
            let mut h = host(&["1"], Ok(vec![vec![]]));
            assert!(chay(&mut h, "db:x", "i").is_err(), "hàng 0 ô ⇒ Err");
            let mut h = host(&["1"], Ok(vec![vec![Value::Int(1)]]));
            assert!(chay(&mut h, "db:x", "i").is_err(), "ô int trần ⇒ Err");
            let mut h = host(
                &["1"],
                Ok(vec![vec![Value::enum_val(
                    "SqlValue",
                    "Integer",
                    vec![Value::Int(0)],
                )]]),
            );
            assert!(chay(&mut h, "db:x", "i").is_err(), "hằng 0 ⇒ Err");
            let mut h = host(&["1"], Ok(vec![vec![mot()], vec![mot()]]));
            assert!(chay(&mut h, "db:x", "i").is_err(), "hai hàng ⇒ Err");
        }
    }
}

pub(crate) mod sqlitejournal {
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
    use vis_core::value::Value;

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
            [] => Err(
                "`sqlite.journal_mode`: truy vấn hệ thống không trả hàng (cần đúng 1)".to_string(),
            ),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.journal_mode`: hàng có {} ô (cần đúng 1)",
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
                                && v == "Text"
                                && matches!(args.as_slice(), [Value::Str(_)])
                        } =>
                    {
                        let (_e, _v, args) = (&__en.enum_name, &__en.variant, &__en.args);
                        {
                            let [Value::Str(s)] = args.as_slice() else {
                                unreachable!(
                                    "mẫu `matches!` ở trên đã bảo đảm đúng một `Value::Str`"
                                )
                            };
                            Ok(s.clone())
                        }
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
            Value::enum_val("SqlValue", "Text", vec![Value::Str(s.to_string())])
        }

        fn int(n: i64) -> Value {
            Value::enum_val("SqlValue", "Integer", vec![Value::Int(n)])
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
                vec![vec![Value::enum_val("SqlValue", "Null", Vec::new())]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Null ⇒ Err");
            let mut h = host(&["journal_mode"], vec![vec![Value::Str("delete".into())]]);
            assert!(
                chay(&mut h, "db:x").is_err(),
                "ô không phải `SqlValue` ⇒ Err"
            );
            let mut h = host(
                &["journal_mode"],
                vec![vec![Value::enum_val(
                    "Other",
                    "Text",
                    vec![Value::Str("delete".into())],
                )]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "enum khác ⇒ Err");
            let mut h = host(
                &["journal_mode"],
                vec![vec![Value::enum_val("SqlValue", "Text", Vec::new())]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text thiếu đối số ⇒ Err");
            let mut h = host(
                &["journal_mode"],
                vec![vec![Value::enum_val(
                    "SqlValue",
                    "Text",
                    vec![Value::Int(1)],
                )]],
            );
            assert!(chay(&mut h, "db:x").is_err(), "Text sai kiểu đối số ⇒ Err");
        }
    }
}

pub(crate) mod sqlitepk {
    //! **VD-270 (sqlite-primary-keys)** — lõi **dùng chung** cho `sqlite.primary_keys`
    //! (interp ↔ VM byte-identical).
    //!
    //! Trả **tên cột khoá chính** của một **bảng** trong **schema chính** (`main`) của connection,
    //! theo **thứ tự khoá** (`pk` 1..N — đúng thứ tự khai trong `PRIMARY KEY (…)`, **không** sắp
    //! alphabet), giữ nguyên hoa/thường như định nghĩa. Tên bảng được **bind** làm tham số (`?`) —
    //! không bao giờ nối chuỗi SQL — nên tên chứa dấu nháy/`;`/comment chỉ là tên, không có tác dụng
    //! SQL.
    //!
    //! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
    //! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
    //! timeout, cache schema/statement. `pragma_table_info` là table-valued function dựng sẵn của
    //! SQLite; probe xác nhận authorizer cho đọc (chỉ `dbstat`/trang thô bị chặn). Tham số schema
    //! **cố định** `'main'` nên bảng TEMP trùng tên **không** che bảng chính và không lọt vào kết quả.
    //! **Hạn chế đã biết**: SQLite cho đối tượng schema trùng tên `pragma_table_info` che TVF dựng
    //! sẵn; khi đó truy vấn lỗi ⇒ `Err(Error)` **dữ liệu** (không đọc nhầm/không trả dữ liệu sai).
    //!
    //! **Bảng không có khoá chính, view, hoặc bảng không tồn tại ⇒ `Ok([])`** (pragma không trả
    //! hàng `pk > 0`) — khác hẳn handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)`
    //! **dữ liệu** (tầng std biến thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không
    //! đúng một cột `name`, ô không phải `SqlValue.Text`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use vis_core::value::Value;

    /// Truy vấn hệ thống: một cột `name` TEXT của các cột khoá chính, theo đúng thứ tự khoá.
    ///
    /// - `?` là **tên bảng** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - `'main'` cố định: chỉ schema chính (không phải `temp`/schema gắn ngoài).
    /// - `pk > 0` chỉ giữ cột thuộc khoá chính (`pk` = 1..N); `ORDER BY pk` là thứ tự khai trong
    ///   `PRIMARY KEY`, **không** phải thứ tự cột (`cid`) hay alphabet.
    const CAU: &str = "SELECT name FROM pragma_table_info(?, 'main') WHERE pk > 0 ORDER BY pk";

    /// Chạy [`CAU`] qua `host.db_query` và trả danh sách tên cột khoá chính **đúng thứ tự khoá**.
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**: đúng
    /// một cột, mỗi hàng đúng một ô `SqlValue.Text(str)`; lệch ⇒ `Err` dữ liệu (không panic).
    /// Bảng không có khoá chính/view/không tồn tại ⇒ pragma trả 0 hàng ⇒ `Ok(vec![])`.
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
                "`sqlite.primary_keys`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `name`)",
                cot.len()
            ));
        }
        let mut ten = Vec::with_capacity(hang.len());
        for (i, hang) in hang.into_iter().enumerate() {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.primary_keys`: hàng {} có {} ô (cần đúng 1)",
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
                                "`sqlite.primary_keys`: hàng {} — `SqlValue.Text` không mang `str`",
                                i + 1
                            ))
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "`sqlite.primary_keys`: hàng {} — tên cột không phải TEXT",
                        i + 1
                    ))
                }
            }
        }
        // **Không** sắp lại: `ORDER BY pk` là hợp đồng **thứ tự khoá**, khác `sqlite.columns`
        // (`ORDER BY cid`) và `sqlite.tables`/`sqlite.indexes` (sắp byte).
        Ok(ten)
    }
}

pub(crate) mod sqliteschema {
    //! **VD-270 (sqlite-schema-sql)** — lõi **dùng chung** cho `sqlite.schema_sql` (interp ↔ VM
    //! byte-identical).
    //!
    //! Trả **văn bản SQL khai báo** của một đối tượng schema trong **schema chính** (`main`) của
    //! connection — `table`/`view`/`index`/`trigger` — tra theo **tên** trong `sqlite_master`. Tên
    //! không phân biệt loại: cùng một danh sách tên của schema chính, nên tra `t` trả câu `CREATE`
    //! tương ứng bất kể đối tượng là loại nào.
    //!
    //! So khớp tên là **phân biệt hoa/thường** (`=` trên cột `name` dùng collation `BINARY` mặc
    //! định của `sqlite_master`): `CREATE TABLE Bang` lưu đúng `Bang`, nên tra `bang` ⇒ `Ok(None)`
    //! dù SQLite vẫn cho tham chiếu bảng tên `bang` trong câu SQL khác. Đây là hợp đồng **có chủ ý**
    //! của truy vấn, không phải sót.
    //!
    //! Không có tham số **loại** đối tượng: SQLite cho phép **trigger** trùng tên với bảng (hai
    //! không gian tên khác nhau), nên nếu một tên có hai hàng trong `sqlite_master`, `LIMIT 1` trả
    //! hàng mà truy vấn duyệt trước — hợp đồng **không hứa** loại nào; đây là hệ quả trực tiếp của
    //! "tra theo tên", không phải lỗi. Bảng/view/index chia sẻ không gian tên nên không trùng nhau.
    //!
    //! Đi qua **đúng** đường `DbHost::db_query` hiện có (không thêm method host): cùng authorizer,
    //! chính sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler
    //! timeout, cache schema/statement. `sqlite_master` của schema chính là bảng hệ thống bình thường
    //! với authorizer (chỉ `dbstat`/trang thô bị chặn); `sqlite.tables`/`columns`/`indexes` đã đi
    //! cùng đường đọc này.
    //!
    //! **Không có hàng ⇒ `Ok(None)`** (tên không tồn tại). Hàng có `sql = NULL` cũng ⇒ `Ok(None)`:
    //! SQLite để `sql` NULL cho đối tượng **do nó tự sinh** như index tự động `sqlite_autoindex_%`
    //! (từ `UNIQUE`/`PRIMARY KEY`) — không có văn bản `CREATE` để trả. Đối tượng nội bộ `sqlite_%`
    //! **không** bị đặc cách ngoài những gì truy vấn trả: ví dụ `sqlite_sequence` (sinh bởi
    //! `AUTOINCREMENT`) có `sql` thật nên trả đúng chuỗi SQLite lưu; `sqlite_master`/`sqlite_schema`
    //! không có hàng trong chính nó nên tra ⇒ `Ok(None)`. TEMP object nằm ở schema `temp`
    //! (`sqlite_temp_master`) nên **không** thuộc kết quả schema chính.
    //!
    //! Tên đối tượng là **dữ liệu**, không phải SQL: bind `SqlValue.Text` làm tham số `?` — không
    //! bao giờ nối chuỗi, nên tên kiểu `t' OR 1=1 --`/`t; DROP TABLE t` chỉ là tên (không có đối
    //! tượng như vậy ⇒ `Ok(None)`), không có tác dụng phụ. Hàm **chỉ đọc**.
    //!
    //! Handle thiếu/đã đóng/sai dạng hoặc lỗi SQL/host ⇒ `Err(String)` **dữ liệu** (tầng std biến
    //! thành `Err(Error)`, **không** trap). Kết quả sai hình dạng (không đúng một cột `sql`, quá một
    //! hàng, ô không phải `SqlValue.Text`/`SqlValue.Null`) cũng là `Err` dữ liệu, không panic.

    use crate::builtin::DbHost;
    use vis_core::value::Value;

    /// Truy vấn hệ thống: một cột `sql` (TEXT hoặc NULL), tra theo **tên** trong schema chính.
    ///
    /// - `?` là **tên đối tượng** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - Không lọc `type`: mọi đối tượng schema (table/view/index/trigger) dùng chung một không gian
    ///   tên trong `sqlite_master`; tên quyết định đối tượng, không phải loại.
    /// - `name = ?`: collation `BINARY` mặc định ⇒ so **byte**, **phân biệt hoa/thường**.
    /// - `LIMIT 1`: tối đa một hàng (bình thường tên là duy nhất trong schema chính).
    const CAU: &str = "SELECT sql FROM sqlite_master WHERE name = ? LIMIT 1";

    /// Chạy [`CAU`] qua `host.db_query` và trả văn bản SQL của đối tượng.
    ///
    /// Không tự prepare/step SQLite, không đọc trang thô. Hình dạng kết quả được kiểm **chặt**:
    /// đúng một cột, tối đa một hàng, ô là `SqlValue.Text(str)` ⇒ `Some`, `SqlValue.Null` ⇒ `None`;
    /// lệch ⇒ `Err` dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str, name: &str) -> Result<Option<String>, String> {
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
                "`sqlite.schema_sql`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `sql`)",
                cot.len()
            ));
        }
        match hang.as_slice() {
            // Không có đối tượng tên đó trong schema chính.
            [] => Ok(None),
            [hang] => {
                let [o] = hang.as_slice() else {
                    return Err(format!(
                        "`sqlite.schema_sql`: hàng có {} ô (cần đúng 1)",
                        hang.len()
                    ));
                };
                match o {
                    // `sql` NULL (index tự động, đối tượng SQLite tự sinh) ⇒ không có văn bản.
                    Value::Enum(__en)
                        if {
                            let e = &__en.enum_name;
                            let v = &__en.variant;
                            let args = &__en.args;
                            e == "SqlValue" && v == "Null" && args.is_empty()
                        } =>
                    {
                        let (_e, _v, _args) = (&__en.enum_name, &__en.variant, &__en.args);
                        {
                            Ok(None)
                        }
                    }
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
                                Value::Str(s) => Ok(Some(s.clone())),
                                _ => Err("`sqlite.schema_sql`: `SqlValue.Text` không mang `str`"
                                    .to_string()),
                            }
                        }
                    }
                    _ => Err("`sqlite.schema_sql`: `sql` không phải TEXT/NULL".to_string()),
                }
            }
            _ => Err(format!(
                "`sqlite.schema_sql`: truy vấn hệ thống trả {} hàng (cần tối đa 1)",
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
            Value::enum_val("SqlValue", "Text", vec![Value::Str(s.to_string())])
        }

        fn null() -> Value {
            Value::enum_val("SqlValue", "Null", Vec::new())
        }

        #[test]
        fn tra_theo_ten_bind_va_tra_text() {
            let mut h = host(&["sql"], vec![vec![text("CREATE TABLE t (a TEXT)")]]);
            assert_eq!(
                chay(&mut h, "db:x", "t").expect("phải Ok"),
                Some("CREATE TABLE t (a TEXT)".to_string())
            );
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
        fn khong_co_hang_hoac_null_deu_none() {
            let mut h = host(&["sql"], Vec::new());
            assert_eq!(chay(&mut h, "db:x", "thieu").expect("phải Ok"), None);
            let mut h = host(&["sql"], vec![vec![null()]]);
            assert_eq!(
                chay(&mut h, "db:x", "sqlite_autoindex_t_1").expect("phải Ok"),
                None
            );
        }

        #[test]
        fn hinh_dang_sai_la_err_du_lieu() {
            let mut h = host(&["sql", "extra"], vec![vec![text("x"), text("y")]]);
            assert!(chay(&mut h, "db:x", "t").is_err(), "hai cột ⇒ Err");
            let mut h = host(&["sql"], vec![vec![]]);
            assert!(chay(&mut h, "db:x", "t").is_err(), "hàng 0 ô ⇒ Err");
            let mut h = host(&["sql"], vec![vec![Value::Int(1)]]);
            assert!(chay(&mut h, "db:x", "t").is_err(), "ô int ⇒ Err");
            let mut h = host(&["sql"], vec![vec![text("a")], vec![text("b")]]);
            assert!(chay(&mut h, "db:x", "t").is_err(), "hai hàng ⇒ Err");
        }
    }
}
