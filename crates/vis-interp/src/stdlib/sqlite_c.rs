// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `sqlite_c` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod sqliteidxcols {
    //! **VD-270 (sqlite-index-columns)** — lõi **dùng chung** cho `sqlite.index_columns`
    //! (interp ↔ VM byte-identical).
    //!
    //! Trả **tên cột** của một index trong **schema chính** (`main`) của connection, theo **thứ tự
    //! index** (`seqno` tăng dần: đúng thứ tự các cột đã khai trong `CREATE INDEX`), giữ nguyên
    //! hoa/thường như định nghĩa. Index được tra theo **tên** (index name là **schema-global** —
    //! không phải thuộc tính riêng của bảng), nên index trên **bảng khác** vẫn tìm thấy khi gọi đúng
    //! tên; **không** lọc theo bảng.
    //!
    //! **Đường đọc**: `DbHost::db_query` hiện có (không thêm method host) — cùng authorizer, chính
    //! sách chỉ đọc (`sqlite3_stmt_readonly`), trần SQL/hàng/ô/cột/byte, progress-handler timeout,
    //! cache schema/statement. Tên index được **bind** làm tham số (`?`, `SqlValue.Text`) — không
    //! bao giờ nối chuỗi SQL — nên tên chứa dấu nháy/`;`/comment chỉ là tên.
    //!
    //! **Vì sao không `pragma_index_info(?, 'main')`**: probe trên host hiện tại
    //! (`crates/vis-cli/src/db/sqlite/mod.rs`, allow-list authorizer) cho thấy `SQLITE_PRAGMA` chỉ
    //! cho `table_info` + **đọc** `user_version`/`schema_version`/`data_version`; `pragma_index_info`
    //! và `pragma_index_xinfo` đều bị **từ chối** (`not authorized`, mã SQLite 23). Vì vậy đường
    //! tương đương **được phép** là đọc `sqlite_master.sql` (chỉ đọc, authorizer cho phép — probe
    //! xác nhận) rồi **phân tích thuần chuỗi** danh sách cột của `CREATE INDEX`. Không prepare/step
    //! SQLite trực tiếp, không đọc trang thô, không dùng `dbstat`.
    //!
    //! **Ngữ nghĩa đã chốt**:
    //! - Index **không tồn tại** (kể cả index ở schema `temp`/schema khác — ta chỉ đọc schema chính)
    //!   ⇒ `Ok([])`.
    //! - Index tự động `sqlite_autoindex_%` có `sqlite_master.sql = NULL`; đường đọc được phép
    //!   **không** chứa định nghĩa cột nên trả `Ok([])` — **không** phải lỗi. (Muốn tên cột của
    //!   autoindex, dùng index người dùng tương ứng.)
    //! - **Mục là biểu thức** (`lower(a)`, `a + b`, `(a+0)`, …): `pragma_index_info` tương ứng trả
    //!   `name = NULL`; hợp đồng này **bỏ qua** các mục đó (không trả `""`), giữ thứ tự tương đối
    //!   của các cột thật. Index toàn biểu thức ⇒ `Ok([])`. Literal **trần** `TRUE`/`FALSE`/`NULL`
    //!   cũng là biểu thức (bỏ qua) trừ `TRUE`/`FALSE` khi bảng **thật sự** có cột cùng tên (SQLite
    //!   phân giải chúng thành cột); bare `NULL` luôn là literal, cột tên `null` phải **có nháy**
    //!   (`"null"`) trong `CREATE INDEX` đã lưu để được nhận là cột.
    //! - Tên cột trả về là **tên đã khai** trong bảng (không phải chữ trong `CREATE INDEX`):
    //!   index ghi `b` cho cột khai `"B"` ⇒ trả `B` — cùng quy ước `pragma_index_info` và
    //!   `sqlite.columns`. Để làm việc này (và phân giải `TRUE`/`FALSE`), sau khi phân tích
    //!   `sqlite_master.sql` lõi đọc thêm `SELECT name FROM pragma_table_info(?, 'main')` với **tên
    //!   bảng** (lấy từ chính `CREATE INDEX`) bind làm tham số. **Cột sinh** (generated column)
    //!   không có trong `pragma_table_info` và `pragma_table_xinfo` bị authorizer chặn ⇒ giữ tên
    //!   **viết trong `CREATE INDEX`** (thường trùng tên khai; khác hoa/thường là hạn chế đã ghi).
    //! - Cột có `COLLATE …`/`ASC`/`DESC`/ngoặc dư quanh tên cột vẫn là cột (giống SQLite:
    //!   `ON t((b) DESC)` ⇒ cột `b`).
    //! - Handle thiếu/đã đóng/sai dạng, lỗi host/SQL, `sqlite_master` trả sai hình dạng, hoặc SQL
    //!   định nghĩa index không phân tích được ⇒ `Err(String)` **dữ liệu** (tầng std biến thành
    //!   `Err(Error)`, **không** trap).
    //!
    //! **Hạn chế đã biết**:
    //! - Đường này tin `sqlite_master.sql` (SQLite sinh/giữ từ câu `CREATE INDEX` hợp lệ). Một tệp DB
    //!   bị sửa `writable_schema` có thể có `sql` không khớp index thật; khi đó kết quả phản ánh `sql`
    //!   đã lưu (hoặc `Err` nếu không phân tích được), không panic.
    //! - Như `sqlite.columns`: đối tượng schema trùng tên `pragma_table_info` che TVF dựng sẵn; khi
    //!   đó truy vấn cột lỗi ⇒ `Err(Error)` **dữ liệu** (không đọc nhầm/không trả dữ liệu sai).

    use crate::builtin::DbHost;
    use crate::value::Value;

    /// Truy vấn hệ thống: một cột `sql` (định nghĩa `CREATE INDEX`) của index theo **tên**.
    ///
    /// - `?` là **tên index** bind từ tham số — không nối chuỗi, injection vô hiệu.
    /// - `type = 'index'` + `name = ? COLLATE NOCASE`: đúng đối tượng index, so **không phân biệt
    ///   hoa/thường ASCII** như cách SQLite tra tên đối tượng (NOCASE của SQLite chỉ gập ASCII).
    /// - Không lọc `tbl_name`: index name là **schema-global**, index trên bảng khác vẫn tìm thấy.
    /// - `sqlite_master` của schema chính; index TEMP nằm ở `sqlite_temp_master` nên không lọt.
    const CAU: &str = "SELECT sql FROM sqlite_master \
                   WHERE type = 'index' AND name = ? COLLATE NOCASE";

    /// Truy vấn hệ thống thứ hai: một cột `name` TEXT — cột **đã khai** của bảng đích trong schema
    /// chính. `?` là **tên bảng** (lấy từ `CREATE INDEX`, bind `SqlValue.Text`). `pragma_table_info`
    /// là TVF dựng sẵn được authorizer cho phép (cùng đường `sqlite.columns`); schema cố định
    /// `'main'` nên bảng TEMP trùng tên không che.
    const CAU_COT: &str = "SELECT name FROM pragma_table_info(?, 'main')";

    /// Chạy [`CAU`] qua `host.db_query`, phân tích danh sách cột của `CREATE INDEX`, rồi đọc tên cột
    /// **đã khai** của bảng đích qua [`CAU_COT`] để ánh xạ đúng hoa/thường và phân giải
    /// `TRUE`/`FALSE`.
    ///
    /// Hình dạng kết quả được kiểm **chặt**: đúng một cột, tối đa một hàng, ô là
    /// `SqlValue.Text`/`SqlValue.Null`; lệch ⇒ `Err` dữ liệu (không panic).
    pub fn chay(host: &mut dyn DbHost, h: &str, index: &str) -> Result<Vec<String>, String> {
        // Tên index là **dữ liệu**, không phải SQL: bind `SqlValue.Text`. Host kiểm/giới hạn param
        // (≤ 1 MiB text) và bind qua rusqlite; không có đường nối chuỗi nào ở đây.
        let tham_so = [Value::Enum(
            "SqlValue".into(),
            "Text".into(),
            vec![Value::Str(index.to_string())],
        )];
        let (cot, hang) = host.db_query(h, CAU, &tham_so)?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.index_columns`: truy vấn hệ thống trả {} cột (cần đúng 1 cột `sql`)",
                cot.len()
            ));
        }
        if hang.len() > 1 {
            // Tên index là duy nhất trong schema (SQLite so không phân biệt hoa/thường); nhiều hàng
            // nghĩa là schema bất thường ⇒ Err dữ liệu, không chọn hàng tuỳ tiện.
            return Err(format!(
                "`sqlite.index_columns`: tên index khớp {} hàng (cần tối đa 1)",
                hang.len()
            ));
        }
        let Some(hang) = hang.into_iter().next() else {
            // Không có index nào tên như vậy ⇒ `Ok([])`.
            return Ok(Vec::new());
        };
        let [o] = hang.as_slice() else {
            return Err(format!(
                "`sqlite.index_columns`: hàng 1 có {} ô (cần đúng 1)",
                hang.len()
            ));
        };
        match o {
            // Autoindex không có `sql` ⇒ không có định nghĩa cột trong đường đọc được phép.
            Value::Enum(e, v, args) if e == "SqlValue" && v == "Null" && args.is_empty() => {
                Ok(Vec::new())
            }
            Value::Enum(e, v, args) if e == "SqlValue" && v == "Text" && args.len() == 1 => {
                match &args[0] {
                    Value::Str(sql) => {
                        let (bang, muc) = cot_tu_sql(sql)?;
                        let khai = doc_cot_bang(host, h, &bang)?;
                        if khai.is_empty() {
                            return Err(format!(
                                "`sqlite.index_columns`: bảng `{bang}` của index không có cột đọc \
                             được trong schema chính"
                            ));
                        }
                        Ok(loc_cot(&muc, &khai))
                    }
                    _ => Err(
                        "`sqlite.index_columns`: hàng 1 — `SqlValue.Text` không mang `str`"
                            .to_string(),
                    ),
                }
            }
            _ => Err(
                "`sqlite.index_columns`: hàng 1 — `sql` của index không phải TEXT/NULL".to_string(),
            ),
        }
    }

    /// Phân tích `CREATE INDEX … ON <bảng> (<danh sách cột>) …` và trả tên cột theo thứ tự.
    ///
    /// Mỗi mục danh sách được phân tích bằng một ngữ pháp **nhỏ, tất định**:
    ///
    /// ```text
    /// core      := identifier | '(' core (COLLATE identifier)? (ASC|DESC)? ')'
    /// core_mods := core (COLLATE identifier)? (ASC|DESC)?
    /// entry     := core_mods
    /// ```
    ///
    /// `identifier` nhận tên trần (ASCII/Unicode, `_`/`$`) và tên có nháy `"…"`/`` `…` ``/`[…]`/
    /// `'…'` (nháy đơn là dạng tương thích SQLite cho định danh; nháy lặp đôi là escape). Mục không
    /// khớp `entry` là **biểu thức** ⇒ bỏ qua. `None`/lỗi cú pháp khung `CREATE INDEX` ⇒ `Err`.
    fn cot_tu_sql(sql: &str) -> Result<(String, Vec<Muc>), String> {
        let mut q = Quet::new(sql);
        if !q.an_tu_khoa("CREATE") {
            return Err(err_sql());
        }
        // `UNIQUE` tuỳ chọn (index duy nhất vẫn cùng cú pháp danh sách cột).
        let _ = q.an_tu_khoa("UNIQUE");
        if !q.an_tu_khoa("INDEX") {
            return Err(err_sql());
        }
        // `IF NOT EXISTS` tuỳ chọn (SQLite lưu nguyên văn câu người dùng tạo).
        if q.an_tu_khoa("IF") && (!q.an_tu_khoa("NOT") || !q.an_tu_khoa("EXISTS")) {
            return Err(err_sql());
        }
        q.bo_qua();
        if q.doc_dinh_danh().is_none() {
            return Err(err_sql());
        }
        // Tên index có thể có tiền tố schema (`CREATE INDEX main.i ON …`).
        q.bo_qua();
        if q.nhin() == Some(b'.') {
            q.i += 1;
            q.bo_qua();
            if q.doc_dinh_danh().is_none() {
                return Err(err_sql());
            }
            q.bo_qua();
        }
        if !q.an_tu_khoa("ON") {
            return Err(err_sql());
        }
        q.bo_qua();
        let Some(mut bang) = q.doc_dinh_danh() else {
            return Err(err_sql());
        };
        // Bảng đích cũng có thể có tiền tố schema trong văn bản đã lưu; `pragma_table_info` nhận
        // **tên bảng** (schema là tham số riêng, cố định `'main'`).
        q.bo_qua();
        if q.nhin() == Some(b'.') {
            q.i += 1;
            q.bo_qua();
            let Some(ten_bang) = q.doc_dinh_danh() else {
                return Err(err_sql());
            };
            bang = ten_bang;
            q.bo_qua();
        }
        if q.nhin() != Some(b'(') {
            return Err(err_sql());
        }
        q.i += 1;
        let Some(muc) = tach_muc(sql, q.i) else {
            return Err(err_sql());
        };
        let mut ten = Vec::with_capacity(muc.len());
        for m in muc {
            if let Some(t) = phan_tich_muc(m) {
                ten.push(t);
            }
        }
        Ok((bang, ten))
    }

    fn err_sql() -> String {
        "`sqlite.index_columns`: không phân tích được định nghĩa `CREATE INDEX` trong \
     `sqlite_master.sql`"
            .to_string()
    }

    /// Tách danh sách mục trong ngoặc thành các **lát cắt nguyên văn**, tôn trọng ngoặc lồng,
    /// chuỗi/định danh có nháy và comment `--…`/`/*…*/`. `None` nếu thiếu `)` đóng.
    fn tach_muc(sql: &str, start: usize) -> Option<Vec<&str>> {
        let b = sql.as_bytes();
        let mut i = start;
        let mut dau = start;
        let mut sau: usize = 1;
        let mut ds = Vec::new();
        while i < b.len() {
            match b[i] {
                q @ (b'\'' | b'"' | b'`') => {
                    i += 1;
                    loop {
                        if i >= b.len() {
                            return None;
                        }
                        if b[i] == q {
                            if b.get(i + 1) == Some(&q) {
                                i += 2;
                            } else {
                                i += 1;
                                break;
                            }
                        } else {
                            i += 1;
                        }
                    }
                }
                b'[' => {
                    i += 1;
                    while i < b.len() && b[i] != b']' {
                        i += 1;
                    }
                    if i >= b.len() {
                        return None;
                    }
                    i += 1;
                }
                b'-' if b.get(i + 1) == Some(&b'-') => {
                    i += 2;
                    while i < b.len() && b[i] != b'\n' {
                        i += 1;
                    }
                }
                b'/' if b.get(i + 1) == Some(&b'*') => {
                    i += 2;
                    loop {
                        if i + 1 >= b.len() {
                            return None;
                        }
                        if b[i] == b'*' && b[i + 1] == b'/' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                b'(' => {
                    sau += 1;
                    i += 1;
                }
                b')' => {
                    sau -= 1;
                    if sau == 0 {
                        ds.push(&sql[dau..i]);
                        return Some(ds);
                    }
                    i += 1;
                }
                b',' if sau == 1 => {
                    ds.push(&sql[dau..i]);
                    i += 1;
                    dau = i;
                }
                _ => i += 1,
            }
        }
        None
    }

    /// Một mục đã phân tích: tên viết trong `CREATE INDEX` + cờ **literal trần**
    /// (`TRUE`/`FALSE`/`NULL` không nháy).
    struct Muc {
        ten: String,
        literal: bool,
    }

    /// `TRUE`/`FALSE`/`NULL` trần là **literal** (biểu thức), không phải định danh cột.
    fn la_literal(ten: &str) -> bool {
        ten.eq_ignore_ascii_case("TRUE")
            || ten.eq_ignore_ascii_case("FALSE")
            || ten.eq_ignore_ascii_case("NULL")
    }

    /// Đọc tên cột **đã khai** của bảng đích qua [`CAU_COT`] (cùng hình dạng chặt như
    /// `sqlite.columns`: đúng một cột, mỗi hàng đúng một ô `SqlValue.Text`).
    fn doc_cot_bang(host: &mut dyn DbHost, h: &str, bang: &str) -> Result<Vec<String>, String> {
        let tham_so = [Value::Enum(
            "SqlValue".into(),
            "Text".into(),
            vec![Value::Str(bang.to_string())],
        )];
        let (cot, hang) = host.db_query(h, CAU_COT, &tham_so)?;
        if cot.len() != 1 {
            return Err(format!(
                "`sqlite.index_columns`: truy vấn cột của bảng trả {} cột (cần đúng 1 cột `name`)",
                cot.len()
            ));
        }
        let mut ds = Vec::with_capacity(hang.len());
        for (i, hang) in hang.into_iter().enumerate() {
            let [o] = hang.as_slice() else {
                return Err(format!(
                    "`sqlite.index_columns`: hàng cột {} có {} ô (cần đúng 1)",
                    i + 1,
                    hang.len()
                ));
            };
            match o {
                Value::Enum(e, v, args) if e == "SqlValue" && v == "Text" && args.len() == 1 => {
                    match &args[0] {
                        Value::Str(s) => ds.push(s.clone()),
                        _ => {
                            return Err(format!(
                        "`sqlite.index_columns`: hàng cột {} — `SqlValue.Text` không mang `str`",
                        i + 1
                    ))
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "`sqlite.index_columns`: hàng cột {} — tên cột không phải TEXT",
                        i + 1
                    ))
                }
            }
        }
        Ok(ds)
    }

    /// Ánh xạ mục đã phân tích sang **tên cột đã khai** theo thứ tự index.
    ///
    /// - Khớp cột đã khai (không phân biệt hoa/thường ASCII) ⇒ trả **tên đã khai**.
    /// - Không khớp nhưng **không** phải literal ⇒ có thể là **cột sinh** (generated column —
    ///   `pragma_table_info` không liệt kê, `pragma_table_xinfo` bị authorizer chặn) ⇒ giữ tên
    ///   **viết trong `CREATE INDEX`**. Với SQL hợp lệ, mọi tên không phải biểu thức đều là cột.
    /// - `TRUE`/`FALSE` trần khớp cột cùng tên ⇒ vẫn là cột (SQLite phân giải như vậy); không khớp
    ///   ⇒ literal biểu thức ⇒ bỏ qua.
    /// - `NULL` trần **luôn** là literal (kể cả khi bảng có cột tên `null`) ⇒ bỏ qua.
    fn loc_cot(muc: &[Muc], khai: &[String]) -> Vec<String> {
        let mut ds = Vec::with_capacity(muc.len());
        for m in muc {
            if m.literal && m.ten.eq_ignore_ascii_case("NULL") {
                continue;
            }
            match khai.iter().find(|c| c.eq_ignore_ascii_case(&m.ten)) {
                Some(ten) => ds.push(ten.clone()),
                None if !m.literal => ds.push(m.ten.clone()),
                None => {}
            }
        }
        ds
    }

    /// Phân tích một mục; `Some(Muc)` khi mục là **tham chiếu cột đơn** (kèm `COLLATE`/`ASC`/
    /// `DESC`/ngoặc dư), `None` khi là biểu thức.
    fn phan_tich_muc(muc: &str) -> Option<Muc> {
        let mut q = Quet::new(muc);
        let ten = q.cot_co_mod()?;
        q.bo_qua();
        if q.het() {
            Some(ten)
        } else {
            None
        }
    }

    /// Con trỏ quét byte trên văn bản SQL (UTF-8; mọi ranh giới cắt đều ở ký tự ASCII).
    struct Quet<'a> {
        goc: &'a str,
        b: &'a [u8],
        i: usize,
    }

    impl<'a> Quet<'a> {
        fn new(s: &'a str) -> Self {
            Self {
                goc: s,
                b: s.as_bytes(),
                i: 0,
            }
        }

        fn het(&self) -> bool {
            self.i >= self.b.len()
        }

        fn nhin(&self) -> Option<u8> {
            self.b.get(self.i).copied()
        }

        fn nhin_sau(&self) -> Option<u8> {
            self.b.get(self.i + 1).copied()
        }

        /// Bỏ khoảng trắng ASCII và comment `--…`/`/*…*/` (comment chưa đóng ⇒ tới hết).
        fn bo_qua(&mut self) {
            loop {
                while let Some(c) = self.nhin() {
                    if matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0c) {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                if self.nhin() == Some(b'-') && self.nhin_sau() == Some(b'-') {
                    self.i += 2;
                    while let Some(c) = self.nhin() {
                        if c == b'\n' {
                            break;
                        }
                        self.i += 1;
                    }
                    continue;
                }
                if self.nhin() == Some(b'/') && self.nhin_sau() == Some(b'*') {
                    self.i += 2;
                    while !self.het() {
                        if self.nhin() == Some(b'*') && self.nhin_sau() == Some(b'/') {
                            self.i += 2;
                            break;
                        }
                        self.i += 1;
                    }
                    continue;
                }
                break;
            }
        }

        /// Đọc **từ khoá/tên trần** (không nháy); `None` nếu ký tự hiện tại không mở đầu định danh.
        fn doc_tran(&mut self) -> Option<&'a str> {
            let c = self.nhin()?;
            if !(c.is_ascii_alphabetic() || c == b'_' || c >= 0x80) {
                return None;
            }
            let dau = self.i;
            self.i += 1;
            while let Some(x) = self.nhin() {
                if x.is_ascii_alphanumeric() || x == b'_' || x == b'$' || x >= 0x80 {
                    self.i += 1;
                } else {
                    break;
                }
            }
            Some(&self.goc[dau..self.i])
        }

        /// Ăn một từ khoá **trần** không phân biệt hoa/thường; không khớp ⇒ hoàn vị trí.
        fn an_tu_khoa(&mut self, kw: &str) -> bool {
            let luu = self.i;
            self.bo_qua();
            match self.doc_tran() {
                Some(t) if t.eq_ignore_ascii_case(kw) => true,
                _ => {
                    self.i = luu;
                    false
                }
            }
        }

        /// Đọc một **định danh** (trần hoặc có nháy) và trả nội dung đã bỏ nháy.
        fn doc_dinh_danh(&mut self) -> Option<String> {
            self.doc_dinh_danh_nhay().map(|(ten, _)| ten)
        }

        /// Như [`Self::doc_dinh_danh`] nhưng kèm cờ **có nháy** (để phân biệt literal `TRUE`/
        /// `FALSE`/`NULL` trần với cột cùng tên được quote).
        fn doc_dinh_danh_nhay(&mut self) -> Option<(String, bool)> {
            let c = self.nhin()?;
            match c {
                b'"' | b'\'' | b'`' => self.doc_co_nhay(c).map(|ten| (ten, true)),
                b'[' => {
                    self.i += 1;
                    let dau = self.i;
                    while let Some(x) = self.nhin() {
                        if x == b']' {
                            let ten = self.goc[dau..self.i].to_string();
                            self.i += 1;
                            return Some((ten, true));
                        }
                        self.i += 1;
                    }
                    None
                }
                _ => self.doc_tran().map(|ten| (ten.to_string(), false)),
            }
        }

        fn doc_co_nhay(&mut self, q: u8) -> Option<String> {
            self.i += 1;
            let mut ten = String::new();
            loop {
                let dau = self.i;
                while let Some(x) = self.nhin() {
                    if x == q {
                        break;
                    }
                    self.i += 1;
                }
                if self.het() {
                    return None;
                }
                ten.push_str(&self.goc[dau..self.i]);
                if self.nhin_sau() == Some(q) {
                    ten.push(q as char);
                    self.i += 2;
                } else {
                    self.i += 1;
                    return Some(ten);
                }
            }
        }

        /// `core_mods` của ngữ pháp trong [`cot_tu_sql`]: cột (có thể bọc ngoặc) + `COLLATE`/
        /// `ASC`/`DESC`. Trả mục; `None` nếu không phải tham chiếu cột đơn.
        fn cot_co_mod(&mut self) -> Option<Muc> {
            self.bo_qua();
            let m = if self.nhin() == Some(b'(') {
                self.i += 1;
                let t = self.cot_co_mod()?;
                self.bo_qua();
                if self.nhin() != Some(b')') {
                    return None;
                }
                self.i += 1;
                t
            } else {
                let (ten, co_nhay) = self.doc_dinh_danh_nhay()?;
                Muc {
                    literal: !co_nhay && la_literal(&ten),
                    ten,
                }
            };
            loop {
                let luu = self.i;
                self.bo_qua();
                let Some(t) = self.doc_tran() else {
                    self.i = luu;
                    break;
                };
                if t.eq_ignore_ascii_case("COLLATE") {
                    self.bo_qua();
                    if self.doc_dinh_danh().is_none() {
                        self.i = luu;
                        break;
                    }
                    continue;
                }
                if t.eq_ignore_ascii_case("ASC") || t.eq_ignore_ascii_case("DESC") {
                    continue;
                }
                self.i = luu;
                break;
            }
            Some(m)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{cot_tu_sql, loc_cot, phan_tich_muc};

        /// Tên cột **viết trong CREATE INDEX** sau khi phân tích (chưa ánh xạ tên đã khai).
        fn ten_viet(sql: &str) -> Vec<String> {
            let (_, muc) = cot_tu_sql(sql).expect("phân tích được");
            muc.into_iter().map(|m| m.ten).collect()
        }

        /// Ánh xạ sang tên cột **đã khai** cho trước.
        fn ten_khai(sql: &str, khai: &[&str]) -> Vec<String> {
            let (_, muc) = cot_tu_sql(sql).expect("phân tích được");
            let khai: Vec<String> = khai.iter().map(|s| s.to_string()).collect();
            loc_cot(&muc, &khai)
        }

        #[test]
        fn thu_tu_va_modifier() {
            assert_eq!(ten_viet("CREATE INDEX i ON t(b, a)"), ["b", "a"]);
            assert_eq!(
                ten_viet("create unique index if not exists \"I\" on \"T\" (\"b\" DESC, a ASC)"),
                ["b", "a"]
            );
            assert_eq!(
                ten_viet("CREATE INDEX i ON t(b COLLATE NOCASE DESC, a COLLATE \"x\")"),
                ["b", "a"]
            );
            // Ngoặc dư quanh tên cột vẫn là cột (giống SQLite).
            assert_eq!(
                ten_viet("CREATE INDEX i ON t(((b)), (a) DESC, (b COLLATE NOCASE))"),
                ["b", "a", "b"]
            );
        }

        #[test]
        fn bieu_thuc_bi_bo_qua() {
            for sql in [
                "CREATE INDEX i ON t(lower(b))",
                "CREATE INDEX i ON t(b + 1)",
                "CREATE INDEX i ON t((b + 0))",
                "CREATE INDEX i ON t(-b)",
                "CREATE INDEX i ON t('b' || '')",
                "CREATE INDEX i ON t(b IS NULL)",
                "CREATE INDEX i ON t(case when b then 1 else 2 end)",
            ] {
                assert_eq!(ten_viet(sql), Vec::<String>::new(), "{sql}");
            }
            assert_eq!(ten_viet("CREATE INDEX i ON t(b, lower(a), c)"), ["b", "c"]);
            // `'b'` là định danh cột theo cú pháp tương thích của SQLite, không phải biểu thức.
            assert_eq!(ten_viet("CREATE INDEX i ON t('b')"), ["b"]);
            // Literal trần giữ cờ để bước ánh xạ xử lý; tên có nháy là cột.
            assert_eq!(
                ten_viet("CREATE INDEX i ON t(true, null, false)"),
                ["true", "null", "false"]
            );
            assert_eq!(
                ten_viet("CREATE INDEX i ON t(\"true\", `false`, [null])"),
                ["true", "false", "null"]
            );
        }

        #[test]
        fn anh_xa_ten_khai_va_literal() {
            // Chữ trong index khác hoa/thường với cột đã khai ⇒ trả tên đã khai (như pragma).
            assert_eq!(
                ten_khai("CREATE INDEX i ON t(b, a)", &["B", "A"]),
                ["B", "A"]
            );
            // Cột keyword: bare TRUE/FALSE chỉ là cột khi bảng thật sự có cột đó.
            assert_eq!(
                ten_khai("CREATE INDEX i ON t(true)", &["b"]),
                Vec::<String>::new()
            );
            assert_eq!(ten_khai("CREATE INDEX i ON t(true)", &["true"]), ["true"]);
            assert_eq!(ten_khai("CREATE INDEX i ON t(TRUE)", &["true"]), ["true"]);
            // Bare NULL luôn là literal, kể cả khi có cột tên `null`; có nháy mới là cột.
            assert_eq!(
                ten_khai("CREATE INDEX i ON t(null)", &["null"]),
                Vec::<String>::new()
            );
            assert_eq!(
                ten_khai("CREATE INDEX i ON t(\"null\")", &["null"]),
                ["null"]
            );
            // Không có trong `pragma_table_info` (cột sinh) ⇒ giữ tên viết trong CREATE INDEX.
            assert_eq!(
                ten_khai("CREATE INDEX i ON t(b, sinh, c)", &["b", "c"]),
                ["b", "sinh", "c"]
            );
            // Biểu thức đã bị loại từ bước phân tích.
            assert_eq!(
                ten_khai("CREATE INDEX i ON t(b, lower(a), c)", &["b", "a", "c"]),
                ["b", "c"]
            );
        }

        #[test]
        fn ten_bang_va_dinh_danh() {
            let (bang, muc) = cot_tu_sql("CREATE INDEX i ON t(b, a)").unwrap();
            assert_eq!(bang, "t");
            assert_eq!(muc.len(), 2);
            let (bang, _) = cot_tu_sql("CREATE INDEX main.i ON main.t(a)").unwrap();
            assert_eq!(bang, "t");
            assert_eq!(
                ten_viet("CREATE INDEX i ON t(\"Tên Cột\", `x-y`, [select], \"a\"\"b\")"),
                ["Tên Cột", "x-y", "select", "a\"b"]
            );
            let (bang, muc) =
                cot_tu_sql("CREATE INDEX \"i on t\" ON \"Bảng Kỳ Lạ\" (\"ột\") WHERE 1").unwrap();
            assert_eq!(bang, "Bảng Kỳ Lạ");
            assert_eq!(muc.len(), 1);
            assert_eq!(muc[0].ten, "ột");
        }

        #[test]
        fn comment_trong_danh_sach() {
            assert_eq!(
                ten_viet("CREATE INDEX i ON t(b /* x, ) */, -- , )\n a)"),
                ["b", "a"]
            );
        }

        #[test]
        fn sql_hong_la_err_khong_panic() {
            for sql in [
                "",
                "CREATE TABLE t(a)",
                "CREATE INDEX i t(a)",
                "CREATE INDEX i ON t a)",
                "CREATE INDEX i ON t(a",
                "CREATE INDEX i ON t(\"a)",
            ] {
                assert!(cot_tu_sql(sql).is_err(), "{sql:?}");
            }
            assert_eq!(phan_tich_muc(" b ").map(|m| m.ten), Some("b".to_string()));
            assert_eq!(phan_tich_muc(" b + 1 ").map(|m| m.ten), None);
        }
    }
}
