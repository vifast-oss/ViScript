// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (db-batch)** — tách một chuỗi SQL nhiều câu thành **từng câu nguyên văn** trên `;`
//! nằm **ngoài** chuỗi `'…'`/`"…"`/`` `…` ``/`[…]` và comment `--…`/`/*…*/`.
//!
//! Đây **không** phải parser SQL: chỉ là scanner từ vựng tối thiểu để **không** cắt nhầm dấu `;`
//! trong literal/định danh/comment. Mỗi mảnh trả về (kèm `;` kết thúc nếu có) được chạy qua
//! `db_execute` — nơi `prepare`/authorizer/`SQLITE_LIMIT_LENGTH`/progress-handler/cap/map lỗi
//! quyết định hợp lệ và tác dụng. Mảnh **rỗng/chỉ khoảng trắng/chỉ comment** bị bỏ, nên batch
//! `""`, `" ; -- x"`, `"/* x */"` đều không có câu nào.
//!
//! Quy tắc nháy theo SQLite: nháy đơn `''`, nháy kép `""`, backtick ` `` ` lặp đôi để escape;
//! `[...]` kết thúc ở `]` đầu tiên; `--` tới hết dòng; `/* */` **không** lồng. Chuỗi/comment
//! không đóng được giữ tới hết input (để `db_execute` báo lỗi SQL, không cắt im lặng).
//!
//! **VD-270 (batch-trigger)**: câu bắt đầu bằng `CREATE [TEMP|TEMPORARY] TRIGGER` được quét
//! **nguyên khối** tới `;` sau `END` của thân `BEGIN … END`; dấu `;` trong thân không tách câu.
//! Bộ quét đếm lồng `BEGIN`/`CASE` … `END` (kể cả `CASE` trong `WHEN`) và vẫn bỏ qua chuỗi/
//! comment. Trigger hỏng/thiếu `END` được giữ tới hết input để SQLite báo lỗi, không cắt im lặng.

/// Iterator các câu SQL **nguyên văn** trong `sql`, theo thứ tự xuất hiện.
pub(crate) struct TachCau<'a> {
    sql: &'a str,
    pos: usize,
}

impl<'a> TachCau<'a> {
    pub(crate) fn new(sql: &'a str) -> Self {
        Self { sql, pos: 0 }
    }
}

/// Bỏ qua chuỗi/định danh bắt đầu ở `i` (tại dấu nháy `q`) và trả vị trí **sau** dấu đóng.
/// Cặp nháy lặp đôi (`''`, `""`, ` `` `) là escape; hết input khi chưa đóng ⇒ `b.len()`.
fn bo_qua_nhay(b: &[u8], mut i: usize, q: u8) -> usize {
    i += 1;
    while i < b.len() {
        if b[i] == q {
            if b.get(i + 1) == Some(&q) {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    i
}

/// Bỏ qua khoảng trắng SQLite và comment từ `i`; trả vị trí kế tiếp.
fn bo_qua_trang_comment(b: &[u8], mut i: usize) -> usize {
    loop {
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            i += 1;
        }
        if b.get(i) == Some(&b'-') && b.get(i + 1) == Some(&b'-') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b.get(i) == Some(&b'/') && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = if i + 1 < b.len() { i + 2 } else { b.len() };
            continue;
        }
        return i;
    }
}

/// Đọc một từ ASCII (`[A-Za-z_][A-Za-z0-9_]*`) tại `i`; trả (từ **hoa**, vị trí sau).
fn doc_tu(b: &[u8], i: usize) -> Option<(String, usize)> {
    let first = *b.get(i)?;
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
        j += 1;
    }
    Some((String::from_utf8_lossy(&b[i..j]).to_ascii_uppercase(), j))
}

/// Nếu câu bắt đầu bằng `CREATE [TEMP|TEMPORARY] TRIGGER`, quét tới `;` kết thúc **ngoài** thân
/// `BEGIN … END` (tôn trọng `CASE … END` lồng và chuỗi/comment) và trả vị trí sau `;`.
/// Không phải trigger ⇒ `None` để scanner thường xử lý.
fn quet_trigger(b: &[u8], start: usize) -> Option<usize> {
    let mut i = bo_qua_trang_comment(b, start);
    let (tu, ni) = doc_tu(b, i)?;
    if tu != "CREATE" {
        return None;
    }
    i = bo_qua_trang_comment(b, ni);
    if let Some((t2, n2)) = doc_tu(b, i) {
        if t2 == "TEMP" || t2 == "TEMPORARY" {
            i = bo_qua_trang_comment(b, n2);
        }
    }
    let (t3, n3) = doc_tu(b, i)?;
    if t3 != "TRIGGER" {
        return None;
    }
    i = n3;
    let mut case_hdr = 0u32;
    let mut body = 0u32;
    loop {
        i = bo_qua_trang_comment(b, i);
        if i >= b.len() {
            return Some(b.len());
        }
        match b[i] {
            b';' => {
                if body == 0 && case_hdr == 0 {
                    return Some(i + 1);
                }
                i += 1;
            }
            q @ (b'\'' | b'"' | b'`') => i = bo_qua_nhay(b, i, q),
            b'[' => {
                i += 1;
                while i < b.len() && b[i] != b']' {
                    i += 1;
                }
                if i < b.len() {
                    i += 1;
                }
            }
            _ => {
                if let Some((tu, ni)) = doc_tu(b, i) {
                    if body > 0 {
                        match tu.as_str() {
                            "BEGIN" | "CASE" => body += 1,
                            "END" => body = body.saturating_sub(1),
                            _ => {}
                        }
                    } else {
                        match tu.as_str() {
                            "BEGIN" if case_hdr == 0 => body = 1,
                            "CASE" => case_hdr += 1,
                            "END" if case_hdr > 0 => case_hdr -= 1,
                            _ => {}
                        }
                    }
                    i = ni;
                } else {
                    i += 1;
                }
            }
        }
    }
}

impl<'a> Iterator for TachCau<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let b = self.sql.as_bytes();
        loop {
            if self.pos >= b.len() {
                return None;
            }
            let start = self.pos;
            if let Some(end) = quet_trigger(b, start) {
                self.pos = end;
                return Some(&self.sql[start..end]);
            }
            let mut i = self.pos;
            let mut co_token = false;
            // `Some(end)` = gặp `;` ở mức ngoài cùng (end = sau `;`); `None` = hết input.
            let mut het_cau: Option<usize> = None;
            while i < b.len() {
                if b[i] == b';' {
                    i += 1;
                    het_cau = Some(i);
                    break;
                }
                match b[i] {
                    // Khoảng trắng SQLite (space, tab, LF, VT, FF, CR) không phải token.
                    b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r' => i += 1,
                    b'\'' => {
                        co_token = true;
                        i = bo_qua_nhay(b, i, b'\'');
                    }
                    q @ (b'"' | b'`') => {
                        co_token = true;
                        i = bo_qua_nhay(b, i, q);
                    }
                    b'[' => {
                        co_token = true;
                        i += 1;
                        while i < b.len() && b[i] != b']' {
                            i += 1;
                        }
                        if i < b.len() {
                            i += 1;
                        }
                    }
                    b'-' if b.get(i + 1) == Some(&b'-') => {
                        while i < b.len() && b[i] != b'\n' {
                            i += 1;
                        }
                    }
                    b'/' if b.get(i + 1) == Some(&b'*') => {
                        i += 2;
                        while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                            i += 1;
                        }
                        i = if i + 1 < b.len() { i + 2 } else { b.len() };
                    }
                    _ => {
                        co_token = true;
                        i += 1;
                    }
                }
            }
            match het_cau {
                Some(end) => {
                    self.pos = end;
                    if co_token {
                        return Some(&self.sql[start..end]);
                    }
                    // Mảnh rỗng giữa hai dấu `;` — bỏ, quét tiếp.
                }
                None => {
                    self.pos = b.len();
                    return co_token.then(|| &self.sql[start..]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TachCau;

    fn tach(sql: &str) -> Vec<&str> {
        TachCau::new(sql).collect()
    }

    #[test]
    fn tach_co_ban_va_giu_nguyen_van() {
        let sql = "CREATE TABLE t (a INTEGER);\nINSERT INTO t VALUES (1);\n";
        assert_eq!(
            tach(sql),
            vec!["CREATE TABLE t (a INTEGER);", "\nINSERT INTO t VALUES (1);"]
        );
        // Không có `;` cuối vẫn là một câu; khoảng trắng thừa không sinh câu.
        assert_eq!(tach("SELECT 1\n\n"), vec!["SELECT 1\n\n"]);
    }

    #[test]
    fn khong_tach_trong_chuoi_va_dinh_danh() {
        let sql = "INSERT INTO t VALUES ('a;b');\
                   INSERT INTO t VALUES (\"c;d\");\
                   INSERT INTO `x;y` VALUES (1);\
                   SELECT * FROM [a;b];\
                   INSERT INTO t VALUES ('O''Clock; x');\
                   INSERT INTO t VALUES (\"a\"\";b\");";
        let cau = tach(sql);
        assert_eq!(cau.len(), 6, "{cau:?}");
        assert!(cau[0].contains("'a;b'"), "{cau:?}");
        assert!(cau[1].contains("\"c;d\""), "{cau:?}");
        assert!(cau[2].contains("`x;y`"), "{cau:?}");
        assert!(cau[3].contains("[a;b]"), "{cau:?}");
        assert!(cau[4].contains("'O''Clock; x'"), "{cau:?}");
        assert!(cau[5].contains("\"a\"\";b\""), "{cau:?}");
    }

    #[test]
    fn khong_tach_trong_comment() {
        let sql = "-- a; b\nCREATE TABLE t (a INTEGER); /* c; d */ INSERT INTO t VALUES (1); -- tail; comment";
        let cau = tach(sql);
        assert_eq!(cau.len(), 2, "{cau:?}");
        assert_eq!(cau[0], "-- a; b\nCREATE TABLE t (a INTEGER);");
        assert_eq!(cau[1], " /* c; d */ INSERT INTO t VALUES (1);");
    }

    #[test]
    fn rong_chi_khoang_trang_chi_comment_va_dau_cham_phay() {
        for sql in [
            "",
            " \n\t\r",
            "-- chi comment",
            "/* chi comment */",
            "/* nhieu\n dong; khong co cau */",
            ";;;",
            " ; -- x\n ; /* y */ ",
        ] {
            assert_eq!(tach(sql), Vec::<&str>::new(), "sql={sql:?}");
        }
    }

    #[test]
    fn chuoi_comment_khong_dong_giu_toi_het_input() {
        // Một câu (dù hỏng) — để `db_execute` báo lỗi SQL, không cắt im lặng.
        assert_eq!(
            tach("INSERT INTO t VALUES ('a;b)"),
            vec!["INSERT INTO t VALUES ('a;b)"]
        );
        // Comment chưa đóng ở cuối batch chỉ là comment ⇒ không có câu.
        assert_eq!(
            tach("CREATE TABLE t (a); /* chua dong ;"),
            vec!["CREATE TABLE t (a);"]
        );
        // `--` không có newline nuốt hết phần còn lại của dòng.
        assert_eq!(
            tach("SELECT 1; -- khong newline ; SELECT 2"),
            vec!["SELECT 1;"]
        );
    }

    /// **VD-270 (batch-trigger)**: `;` trong thân `BEGIN … END` không tách câu.
    #[test]
    fn trigger_begin_end_duoc_hieu() {
        let sql = "CREATE TRIGGER tr AFTER INSERT ON t BEGIN INSERT INTO log VALUES (1); \
                   INSERT INTO log VALUES (2); END;";
        assert_eq!(tach(sql), vec![sql]);
    }

    /// `CASE … END` lồng trong thân (và trong `WHEN`) không làm mất dấu `;` phân cách;
    /// chuỗi chứa `;`/`END` vẫn nguyên; câu sau trigger tách đúng.
    #[test]
    fn trigger_case_chuoi_va_cau_sau() {
        let sql = "CREATE TRIGGER tr AFTER INSERT ON t WHEN (CASE WHEN 1 THEN 1 ELSE 0 END) \
                   BEGIN INSERT INTO log VALUES (CASE WHEN 1 THEN ';END;' ELSE 'x' END); \
                   UPDATE t SET a = a + 1; END; \
                   INSERT INTO t VALUES (1);";
        let cau = tach(sql);
        assert_eq!(cau.len(), 2, "{cau:?}");
        assert!(cau[0].contains("CASE WHEN 1 THEN ';END;'"), "{cau:?}");
        assert!(cau[0].ends_with("END;"), "{cau:?}");
        assert!(cau[1].contains("INSERT INTO t"), "{cau:?}");
    }

    /// `CREATE TEMP TRIGGER` + định danh có dấu `;`; comment trong thân.
    #[test]
    fn trigger_temp_ten_va_comment() {
        let sql = "CREATE TEMP TRIGGER \"we;ird\" AFTER INSERT ON t BEGIN \
                   -- comment ; END gia
                   SELECT 1; /* block ; END */ SELECT 2; END;";
        assert_eq!(tach(sql), vec![sql]);
    }

    /// Trigger thiếu `END` giữ tới hết input (để SQLite báo lỗi), không cắt im lặng.
    #[test]
    fn trigger_thieu_end_giu_toi_het() {
        let sql = "CREATE TRIGGER tr AFTER INSERT ON t BEGIN SELECT 1;";
        assert_eq!(tach(sql), vec![sql]);
    }

    /// Không nhầm `CREATE TABLE`/`CREATE VIEW`/tên chứa `trigger`.
    #[test]
    fn khong_nham_cac_create_khac() {
        let sql = "CREATE TABLE t (a); CREATE VIEW v AS SELECT 1; \
                   CREATE TABLE trigger_log (a); CREATE INDEX i ON t(a);";
        assert_eq!(tach(sql).len(), 4, "{:?}", tach(sql));
    }
}
