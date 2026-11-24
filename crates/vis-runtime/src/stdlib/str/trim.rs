// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `str_e` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod strsplitterm {
    //! **VD-270 (split-terminator)** — `std.str.split_terminator(s, sep) -> list[str]`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM), **đúng** `str::split_terminator` của Rust:
    //! - Như `str.split` nhưng **không** sinh phần rỗng cuối khi `s` kết thúc bằng `sep`:
    //!   `split_terminator("a,b,c", ",") == ["a", "b", "c"]`;
    //!   `split_terminator("a,b,", ",") == ["a", "b"]` (khác `str.split`: `["a", "b", ""]`).
    //! - Chỉ bỏ **một** phần rỗng cuối: `split_terminator(",", ",") == [""]` (len 1),
    //!   `split_terminator(",,", ",") == ["", ""]` (len 2),
    //!   `split_terminator("a,b,,", ",") == ["a", "b", ""]` (len 3).
    //! - Dấu ở **đầu** vẫn sinh phần rỗng: `split_terminator(",a", ",") == ["", "a"]`.
    //! - Không khớp (hoặc `sep` dài hơn `s`) ⇒ `[s]`; `"abc"` cắt `"abc"` ⇒ `[""]` (cả chuỗi là
    //!   dấu phân cách, phần rỗng cuối bị bỏ nên còn đúng một phần tử rỗng).
    //! - **`s` rỗng + `sep` khác rỗng ⇒ `[]`** — probe `rustc 1.100.0` xác nhận
    //!   `"".split_terminator(",")` trả iterator **rỗng** (khác `str.split` của VS trả `[""]`,
    //!   và cũng khác `"".split(",")` của Rust trả `[""]`). Đây là khác biệt **có chủ ý** ngoài
    //!   quy tắc "bỏ phần rỗng cuối"; hợp đồng bám probe.
    //! - `sep` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu, và
    //!   **bỏ** phần rỗng cuối nên `split_terminator("abc", "") == ["", "a", "b", "c"]`,
    //!   `split_terminator("a", "") == ["", "a"]`, `split_terminator("đ😀", "") ==
    //!   ["", "đ", "😀"]`, `split_terminator("", "") == [""]` (len 1). Khác `str.split` của VS
    //!   (rỗng ⇒ tách scalar, **không** sinh phần rỗng đầu).
    //! - Chồng lấn **không** được khớp lại: `split_terminator("aaa", "aa") == ["", "a"]`,
    //!   `split_terminator("abab", "aba") == ["", "b"]`, `split_terminator("banana", "ana") ==
    //!   ["b", "na"]`.
    //! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
    //!   **không bao giờ** cắt giữa scalar: `split_terminator("đ😀x😀", "😀") == ["đ", "x"]`;
    //!   ZWJ trong emoji không phải biên đặc biệt: `split_terminator("👨‍👩‍👧x", "👩") ==
    //!   ["👨‍", "‍👧x"]`, phần còn lại vẫn là UTF-8 hợp lệ.
    //!
    //! Không trap, không có trần riêng: số phần tử bị chặn bởi độ dài `s` + 1 (`sep` rỗng) nên
    //! cấp phát tuyến tính theo `s`; mỗi phần tử được cấp phát đúng một chuỗi kết quả.

    use vis_syntax::Span;

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Tách `s` theo `sep`, **bỏ** phần rỗng cuối khi `s` kết thúc bằng `sep`.
    /// Dùng **đúng** `str::split_terminator` của Rust nên `sep` rỗng và `s` rỗng cũng theo nghĩa
    /// Rust (xem hợp đồng ở đầu tệp).
    pub(crate) fn tach(s: &str, sep: &str) -> Vec<String> {
        s.split_terminator(sep).map(str::to_string).collect()
    }

    /// Dispatch `str.split_terminator`: luôn `list[str]`; không trap.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let s = super::super::super::chuoi(name, &args[0], 0, span)?;
        let sep = super::super::super::chuoi(name, &args[1], 1, span)?;
        Ok(Value::List(
            tach(s, sep).into_iter().map(Value::Str).collect(),
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[&str]) -> Vec<String> {
            xs.iter().map(|x| x.to_string()).collect()
        }

        /// `str::split` của Rust — mốc so sánh trực tiếp với `split_terminator`.
        fn split(s: &str, sep: &str) -> Vec<String> {
            s.split(sep).map(str::to_string).collect()
        }

        #[test]
        fn bang_probe_rust_bo_dung_phan_rong_cuoi() {
            assert_eq!(tach("a,b,c", ","), ds(&["a", "b", "c"]));
            assert_eq!(tach("a,b,", ","), ds(&["a", "b"]));
            assert_eq!(tach(",a", ","), ds(&["", "a"]));
            assert_eq!(tach(",", ","), ds(&[""]));
            assert_eq!(tach(",,", ","), ds(&["", ""]));
            assert_eq!(tach("", ","), ds(&[]));
            assert_eq!(tach("a", ","), ds(&["a"]));
            assert_eq!(tach("a,,b", ","), ds(&["a", "", "b"]));
            assert_eq!(tach("a,b,,", ","), ds(&["a", "b", ""]));
            assert_eq!(tach("a--b--", "--"), ds(&["a", "b"]));
            assert_eq!(tach("abc", "abc"), ds(&[""]));
            assert_eq!(tach("aa", "aa"), ds(&[""]));
        }

        #[test]
        fn chi_khac_split_o_phan_rong_cuoi() {
            // Không kết thúc bằng `sep` (và `s` khác rỗng) ⇒ **giống hệt** `str::split`.
            for (s, sep) in [
                ("a,b,c", ","),
                (",a", ","),
                ("a,,b", ","),
                ("aaa", "aa"),
                ("abab", "aba"),
                ("banana", "ana"),
                ("a,b,c", "x"),
                ("abc", "abcd"),
                ("a", ","),
            ] {
                assert_eq!(
                    tach(s, sep),
                    split(s, sep),
                    "({s:?}, {sep:?}) phải giống split"
                );
            }
            // Kết thúc bằng `sep` ⇒ đúng `split` bỏ **một** phần tử rỗng cuối.
            for (s, sep) in [
                ("a,b,", ","),
                (",", ","),
                (",,", ","),
                ("a,b,,", ","),
                ("a--b--", "--"),
                ("abc", "abc"),
                ("aa", "aa"),
                ("đ😀x😀", "😀"),
            ] {
                let mut mong = split(s, sep);
                assert_eq!(mong.pop(), Some(String::new()), "({s:?}, {sep:?})");
                assert_eq!(tach(s, sep), mong, "({s:?}, {sep:?})");
            }
            // `s` rỗng + `sep` khác rỗng: split ⇒ `[""]`, split_terminator ⇒ `[]` (probe Rust).
            assert_eq!(split("", ","), ds(&[""]));
            assert_eq!(tach("", ","), ds(&[]));
        }

        #[test]
        fn khong_khop_tra_nguyen_s() {
            assert_eq!(tach("abc", ","), ds(&["abc"]));
            assert_eq!(tach("a,b,c", "x"), ds(&["a,b,c"]));
            assert_eq!(tach("abc", "abcd"), ds(&["abc"]), "sep dài hơn s");
            assert_eq!(tach("ab", "abc"), ds(&["ab"]));
            assert_eq!(tach("abc", "ac"), ds(&["abc"]));
        }

        #[test]
        fn chong_lan_khong_khop_lai() {
            assert_eq!(tach("aaa", "aa"), ds(&["", "a"]));
            assert_eq!(tach("aaaa", "aa"), ds(&["", ""]));
            assert_eq!(tach("abab", "aba"), ds(&["", "b"]));
            assert_eq!(tach("banana", "ana"), ds(&["b", "na"]));
            assert_eq!(tach("aaaaa", "aa"), ds(&["", "", "a"]));
        }

        #[test]
        fn sep_rong_theo_nghia_rust() {
            assert_eq!(tach("abc", ""), ds(&["", "a", "b", "c"]));
            assert_eq!(tach("a", ""), ds(&["", "a"]));
            assert_eq!(tach("", ""), ds(&[""]));
            assert_eq!(tach("đ😀", ""), ds(&["", "đ", "😀"]));
            assert_eq!(tach("a\r\nb", ""), ds(&["", "a", "\r", "\n", "b"]));
            // So với `str::split` của Rust: luôn bỏ đúng phần rỗng cuối.
            for s in ["", "a", "abc", "đ😀"] {
                let mut mong = split(s, "");
                assert_eq!(mong.pop(), Some(String::new()), "{s:?}");
                assert_eq!(tach(s, ""), mong, "{s:?}");
            }
        }

        #[test]
        fn s_rong_theo_nghia_rust() {
            // Probe `rustc 1.100.0`: `"".split_terminator(sep)` với `sep` khác rỗng là **rỗng**
            // (khác `str.split` của VS trả `[""]`); đây là hợp đồng **có chủ ý**.
            assert_eq!(tach("", ","), ds(&[]));
            assert_eq!(tach("", "abc"), ds(&[]));
            assert_eq!(tach("", ""), ds(&[""]));
        }

        #[test]
        fn unicode_emoji_zwj_tren_bien_scalar() {
            assert_eq!(tach("đ,😀,x", ","), ds(&["đ", "😀", "x"]));
            assert_eq!(tach("a😀b😀c", "😀"), ds(&["a", "b", "c"]));
            assert_eq!(tach("a🌍b🌍", "🌍"), ds(&["a", "b"]));
            assert_eq!(tach("héllo,🌍,👨‍👩‍👧‍👦", ","), ds(&["héllo", "🌍", "👨‍👩‍👧‍👦"]));
            // ZWJ **không** phải biên đặc biệt: cắt đúng quanh scalar `👩`, phần sau vẫn hợp lệ.
            assert_eq!(tach("👨‍👩‍👧x", "👩"), ds(&["👨‍", "‍👧x"]));
            assert_eq!(tach("👨‍👩‍👧‍👦x", "x"), ds(&["👨‍👩‍👧‍👦"]), "không khớp ⇒ nguyên chuỗi");
            assert_eq!(tach("đ😀x", "😀a"), ds(&["đ😀x"]));
        }

        #[test]
        fn dau_phan_cach_dieu_khien_va_nhieu_byte() {
            assert_eq!(tach("a\r\nb\r\n", "\r\n"), ds(&["a", "b"]));
            assert_eq!(tach("a\0b\0", "\0"), ds(&["a", "b"]));
            assert_eq!(tach("a\tb\t", "\t"), ds(&["a", "b"]));
            assert_eq!(tach("a b ", " "), ds(&["a", "b"]));
        }
    }
}

pub(crate) mod strstrip {
    //! **VD-270 (strstrip)** — bỏ **tiền tố/hậu tố**: `std.str.strip_prefix` / `strip_suffix`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `strip_prefix(s, p)`: `s` **bắt đầu** bằng `p` ⇒ `Some(s[len(p)..])`, ngược lại ⇒ `None`.
    //! - `strip_suffix(s, p)`: `s` **kết thúc** bằng `p` ⇒ `Some(s[..len(s)-len(p)])`, ngược lại
    //!   ⇒ `None`.
    //! - Chỉ bỏ **một lần**, và **chỉ** khi khớp đúng nguyên `p`; không cắt theo tập ký tự (khác
    //!   `trim_matches`) và không tìm giữa chuỗi (khác `find`/`replace`).
    //! - `p` rỗng ⇒ `Some(s)` nguyên vẹn (nghĩa Rust `str::strip_prefix`/`strip_suffix`).
    //! - Đơn vị khớp là **byte** nhưng biên luôn rơi trên **ranh giới scalar Unicode**: cắt sau
    //!   `len(p)` byte của một tiền tố khớp là an toàn, nên `"đ😀x"` bỏ tiền tố `"đ"` ⇒ `"😀x"`
    //!   và bỏ hậu tố `"😀x"` ⇒ `"đ"`; không bao giờ trả chuỗi UTF-8 hỏng.
    //! - Kiểu trả `str?` = `Option[str]` **thật** (`Some(Str)`/`None`), không phải `{error}`.
    //! - Không có trần riêng: mọi phép là O(độ dài chuỗi), không cấp phát khi không khớp.

    use vis_core::value::Value;

    /// Phần **còn lại** sau khi bỏ tiền tố `prefix`; `None` nếu `s` không bắt đầu bằng `prefix`.
    /// `prefix` rỗng ⇒ `Some(s)` (Rust `str::strip_prefix`).
    pub(crate) fn strip_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
        s.strip_prefix(prefix)
    }

    /// Phần **còn lại** sau khi bỏ hậu tố `suffix`; `None` nếu `s` không kết thúc bằng `suffix`.
    /// `suffix` rỗng ⇒ `Some(s)` (Rust `str::strip_suffix`).
    pub(crate) fn strip_suffix<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
        s.strip_suffix(suffix)
    }

    /// Dispatch chung cho hai hàm; `name` là `"str.strip_prefix"` hoặc `"str.strip_suffix"`.
    pub(crate) fn goi(name: &str, s: &str, moc: &str) -> Value {
        let con_lai = match name {
            "str.strip_prefix" => strip_prefix(s, moc),
            _ => strip_suffix(s, moc),
        };
        Value::opt(con_lai.map(|x| Value::Str(x.to_string())))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ascii_khop_va_khong_khop() {
            assert_eq!(strip_prefix("banana", "ban"), Some("ana"));
            assert_eq!(strip_prefix("banana", "x"), None);
            assert_eq!(strip_prefix("banana", "ana"), None, "chỉ khớp ở ĐẦU");
            assert_eq!(strip_suffix("banana", "ana"), Some("ban"));
            assert_eq!(strip_suffix("banana", "x"), None);
            assert_eq!(strip_suffix("banana", "ban"), None, "chỉ khớp ở CUỐI");
            assert_eq!(strip_prefix("abc", "abcd"), None, "tiền tố dài hơn");
            assert_eq!(strip_suffix("abc", "abcd"), None, "hậu tố dài hơn");
        }

        #[test]
        fn moc_rong_tra_nguyen_chuoi() {
            assert_eq!(strip_prefix("abc", ""), Some("abc"));
            assert_eq!(strip_suffix("abc", ""), Some("abc"));
            assert_eq!(strip_prefix("", ""), Some(""));
            assert_eq!(strip_suffix("", ""), Some(""));
            assert_eq!(strip_prefix("", "a"), None);
            assert_eq!(strip_suffix("", "a"), None);
        }

        #[test]
        fn chong_lan_chi_bo_mot_lan() {
            assert_eq!(strip_prefix("aaa", "a"), Some("aa"));
            assert_eq!(strip_suffix("aaa", "a"), Some("aa"));
            assert_eq!(strip_prefix("aaa", "aa"), Some("a"));
            assert_eq!(strip_suffix("aaa", "aa"), Some("a"));
            assert_eq!(strip_prefix("aaa", "aaa"), Some(""));
            assert_eq!(strip_suffix("aaa", "aaa"), Some(""));
            assert_eq!(strip_prefix("aaaa", "aa"), Some("aa"));
            assert_eq!(strip_suffix("aaaa", "aa"), Some("aa"));
        }

        #[test]
        fn unicode_theo_scalar_khong_phai_byte() {
            assert_eq!(strip_prefix("đ😀x", "đ"), Some("😀x"));
            assert_eq!(strip_suffix("đ😀x", "😀x"), Some("đ"));
            assert_eq!(strip_prefix("đ😀x", "đ😀"), Some("x"));
            assert_eq!(strip_suffix("đ😀x", "x"), Some("đ😀"));
            assert_eq!(strip_prefix("😀a", "😀"), Some("a"));
            assert_eq!(strip_suffix("a😀", "😀"), Some("a"));
            // "đ" (U+0111) là **một** scalar 2 byte; bỏ nó phải còn đúng phần sau, không lệch byte.
            assert_eq!(strip_prefix("đx", "đ"), Some("x"));
            assert_eq!(strip_suffix("xđ", "đ"), Some("x"));
        }

        #[test]
        fn goi_tra_option_that() {
            use vis_core::value::eq;
            let some = |s: &str| Value::opt(Some(Value::Str(s.into())));
            assert!(eq(&goi("str.strip_prefix", "abc", "a"), &some("bc")));
            assert!(eq(&goi("str.strip_suffix", "abc", "c"), &some("ab")));
            assert!(eq(&goi("str.strip_prefix", "abc", "b"), &Value::none()));
            assert!(eq(&goi("str.strip_suffix", "abc", "b"), &Value::none()));
        }
    }
}

pub(crate) mod strtitle {
    //! **VD-270 (str-title)** — `std.str.title(s) -> str`: chữ hoa đầu **từ**, phần còn lại chữ thường.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Tách **từ** theo **khoảng trắng Unicode** (`char::is_whitespace`), **không** theo dấu câu:
    //!   `"hello-world"` ⇒ `"Hello-world"` (gạch nối là ký tự thường **trong** từ), còn
    //!   `"hello, world!"` ⇒ `"Hello, World!"` (dấu cách mới là biên).
    //! - Với mỗi từ, **scalar đầu** đổi sang **chữ hoa** (`char::to_uppercase`) và **mọi scalar sau**
    //!   đổi sang **chữ thường** (`char::to_lowercase`). Đơn vị là **scalar** Unicode, không phải
    //!   byte/grapheme; phép đổi có thể **nở** thành nhiều scalar (`ß` ⇒ `SS`, `İ` ⇒ `i` + dấu chấm).
    //! - Khoảng trắng **giữ nguyên từng scalar, đúng thứ tự** (`"a  \tb\nc"` ⇒ `"A  \tB\nC"`); chuỗi
    //!   rỗng hoặc **chỉ** khoảng trắng ⇒ trả **nguyên vẹn**.
    //! - Không trap, không cấp phát thừa ngoài chuỗi kết quả; O(số scalar của `s`).

    /// Chữ hoa đầu từ, chữ thường phần còn lại; khoảng trắng Unicode giữ nguyên từng scalar.
    ///
    /// `dau_tu` bật lại **chỉ** khi gặp scalar `is_whitespace` — dấu câu **không** mở từ mới, nên
    /// `"hello-world"` ⇒ `"Hello-world"` (chỉ `h` được viết hoa). Scalar đầu từ được đổi hoa bằng
    /// `char::to_uppercase` (có thể nở: `ß` ⇒ `SS`), các scalar sau bằng `char::to_lowercase`.
    pub(crate) fn title(s: &str) -> String {
        let mut ra = String::with_capacity(s.len());
        let mut dau_tu = true;
        for c in s.chars() {
            if c.is_whitespace() {
                ra.push(c);
                dau_tu = true;
            } else if dau_tu {
                ra.extend(c.to_uppercase());
                dau_tu = false;
            } else {
                ra.extend(c.to_lowercase());
            }
        }
        ra
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn ascii_va_tron_hoa_thuong() {
            assert_eq!(title("hello world"), "Hello World");
            assert_eq!(title("hello"), "Hello");
            assert_eq!(title("a b c"), "A B C");
            assert_eq!(title("hELLo wORLD"), "Hello World");
            assert_eq!(title("HELLO"), "Hello");
            assert_eq!(title("mIxEd CaSe TeXt"), "Mixed Case Text");
        }

        #[test]
        fn khoang_trang_giu_nguyen_tung_scalar() {
            assert_eq!(title("a  b\tc\nd"), "A  B\tC\nD");
            assert_eq!(title("  a\r\n b  "), "  A\r\n B  ");
            // Unicode whitespace: NBSP (U+00A0) và ideographic space (U+3000) cũng là biên từ.
            assert_eq!(
                title("a\u{A0}b\u{3000}c"),
                "A\u{A0}B\u{3000}C",
                "khoảng trắng Unicode phải giữ nguyên và tách từ"
            );
        }

        #[test]
        fn rong_va_chi_khoang_trang_nguyen_ven() {
            assert_eq!(title(""), "");
            assert_eq!(title("   "), "   ");
            assert_eq!(title("\t\n\r"), "\t\n\r");
            assert_eq!(title("\u{A0}\u{3000}"), "\u{A0}\u{3000}");
        }

        #[test]
        fn dau_cau_khong_mo_tu_moi() {
            assert_eq!(title("hello-world"), "Hello-world");
            assert_eq!(title("hello, world!"), "Hello, World!");
            assert_eq!(title("don't STOP"), "Don't Stop");
            // Dấu câu **đầu** từ là scalar đầu tiên: nó được "viết hoa" (không đổi) rồi các scalar
            // sau thành chữ thường — đúng hợp đồng "biên từ chỉ là khoảng trắng".
            assert_eq!(title("(hello)"), "(hello)");
            assert_eq!(title("-Hello"), "-hello");
        }

        #[test]
        fn viet_va_astral_theo_scalar() {
            assert_eq!(title("đẶNG văn hÙNG"), "Đặng Văn Hùng");
            assert_eq!(title("XIN CHÀO THẾ GIỚI"), "Xin Chào Thế Giới");
            assert_eq!(title("😀abc 😀dEF"), "😀abc 😀def");
            assert_eq!(title("👨‍👩‍👧 FAMILY"), "👨‍👩‍👧 Family");
            // Emoji là **một** scalar astral: không bị tách byte, char_count không đổi.
            assert_eq!(title("😀ABC").chars().count(), 4);
        }

        #[test]
        fn ss_no_thanh_hai_scalar() {
            assert_eq!(title("ßig ß"), "SSig SS");
            assert_eq!(title("ẞIG"), "ẞig");
            assert_eq!(title("aß"), "Aß", "ß không phải scalar đầu thì giữ nguyên");
        }
    }
}

pub(crate) mod strtrim {
    //! **VD-270 (strtrim)** — cắt theo **tập scalar**: `std.str.trim_matches` /
    //! `trim_start_matches` / `trim_end_matches`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `chars` là **tập** scalar Unicode: thứ tự **không** quan trọng, lặp **không** sao.
    //! - Bỏ **lặp lại** scalar ở đầu/cuối (hoặc cả hai) khi scalar đó thuộc `chars`, tới scalar
    //!   đầu tiên (từ phía tương ứng) **không** thuộc tập — như Rust `trim*_matches` với closure.
    //! - `chars` rỗng ⇒ trả `s` **nguyên vẹn**; mọi scalar đều thuộc tập ⇒ `""`.
    //! - Đơn vị là **scalar** (`char`), **không** phải byte và **không** phải grapheme: emoji
    //!   nhiều scalar (vd. `👨‍👩‍👧`) chỉ bị cắt từng scalar có mặt trong `chars`.
    //!
    //! **Tối ưu (VD-270 trim-perf)**: `chars` ≤ [`NGUONG_BANG_TRA`] scalar ⇒ duyệt tuyến tính
    //! (không cấp phát); lớn hơn ⇒ sắp xếp `Vec<char>` **một lần** rồi tìm nhị phân
    //! O(log |chars|) mỗi scalar. Kết quả không đổi: bảng tra vẫn là **tập** scalar (lặp/bỏ
    //! trùng như cũ), và ngưỡng chỉ chọn cách tra.

    use vis_syntax::Span;

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Tối đa số scalar của `chars` để còn duyệt tuyến tính; vượt ngưỡng thì dựng bảng tra
    /// (sắp xếp một lần, đổi lại mỗi phép tra thành O(log |chars|)). Không ảnh hưởng ngữ nghĩa.
    const NGUONG_BANG_TRA: usize = 8;

    /// `true` nếu `c` thuộc **tập** scalar `chars` (duyệt tuyến tính, không cấp phát).
    fn thuoc(chars: &str, c: char) -> bool {
        chars.chars().any(|x| x == c)
    }

    /// Cắt `s` theo `name` với predicate thành viên `thuoc`; dùng chung cho cả hai đường tra.
    fn cat_voi(name: &str, s: &str, thuoc: impl Fn(char) -> bool) -> String {
        match name {
            "str.trim_start_matches" => s.trim_start_matches(thuoc).to_string(),
            "str.trim_end_matches" => s.trim_end_matches(thuoc).to_string(),
            _ => s.trim_matches(thuoc).to_string(),
        }
    }

    /// Cắt `s` theo `name` (`"str.trim_matches"` / `"str.trim_start_matches"` /
    /// `"str.trim_end_matches"`); `chars` rỗng ⇒ `s` nguyên vẹn, toàn bộ thuộc tập ⇒ `""`.
    pub(crate) fn cat(name: &str, s: &str, chars: &str) -> String {
        if chars.is_empty() {
            return s.to_string();
        }
        if chars.chars().count() > NGUONG_BANG_TRA {
            let mut tap: Vec<char> = chars.chars().collect();
            tap.sort_unstable();
            tap.dedup();
            return cat_voi(name, s, |c| tap.binary_search(&c).is_ok());
        }
        cat_voi(name, s, |c| thuoc(chars, c))
    }

    /// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.trim*_matches"`.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let s = super::super::super::chuoi(name, &args[0], 0, span)?;
        let chars = super::super::super::chuoi(name, &args[1], 1, span)?;
        Ok(Value::Str(cat(name, s, chars)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn tap_scalar_hai_phia() {
            assert_eq!(cat("str.trim_matches", "--x--", "-"), "x");
            assert_eq!(cat("str.trim_start_matches", "--x--", "-"), "x--");
            assert_eq!(cat("str.trim_end_matches", "--x--", "-"), "--x");
            assert_eq!(cat("str.trim_matches", "abbaHELLOabba", "ab"), "HELLO");
            assert_eq!(cat("str.trim_matches", "abbaHELLOabba", "ba"), "HELLO");
            assert_eq!(cat("str.trim_matches", "a-b", "-a"), "b");
        }

        #[test]
        fn rong_va_toan_bo() {
            assert_eq!(cat("str.trim_matches", "abc", ""), "abc");
            assert_eq!(cat("str.trim_matches", "xyx", "xy"), "");
            assert_eq!(cat("str.trim_start_matches", "xxx", "x"), "");
            assert_eq!(cat("str.trim_end_matches", "xxx", "x"), "");
        }

        #[test]
        fn scalar_khong_phai_grapheme() {
            assert_eq!(cat("str.trim_matches", "đ😀đxđ😀đ", "đ😀"), "x");
            assert_eq!(cat("str.trim_start_matches", "👨‍👩‍👧", "👨").chars().count(), 4);
            assert_eq!(cat("str.trim_matches", "👨‍👩‍👧", "👨👧").chars().count(), 3);
        }

        /// Bản tham chiếu **trước tối ưu** — duyệt tuyến tính mọi scalar của `chars`.
        fn cat_tuyen_tinh(name: &str, s: &str, chars: &str) -> String {
            if chars.is_empty() {
                return s.to_string();
            }
            let trong = |c: char| chars.chars().any(|x| x == c);
            match name {
                "str.trim_start_matches" => s.trim_start_matches(trong).to_string(),
                "str.trim_end_matches" => s.trim_end_matches(trong).to_string(),
                _ => s.trim_matches(trong).to_string(),
            }
        }

        const TEN: [&str; 3] = [
            "str.trim_matches",
            "str.trim_start_matches",
            "str.trim_end_matches",
        ];

        #[test]
        fn tap_lon_trung_va_astral() {
            // 22 scalar > ngưỡng: có lặp ('x' 20 lần) và astral ('😀', 'đ').
            let mut bo = String::new();
            for _ in 0..20 {
                bo.push('x');
            }
            bo.push('😀');
            bo.push('đ');
            assert!(bo.chars().count() > NGUONG_BANG_TRA);
            assert_eq!(cat("str.trim_matches", "xx😀đXđ😀xx", &bo), "X");
            assert_eq!(cat("str.trim_matches", "xđ😀x", &bo), "");
            assert_eq!(cat("str.trim_start_matches", "x😀X", &bo), "X");
            assert_eq!(cat("str.trim_end_matches", "X😀x", &bo), "X");
            // Tập lớn nhưng rỗng ⇒ nguyên vẹn (nhánh sớm, không dựng bảng).
            assert_eq!(cat("str.trim_matches", "xx😀", ""), "xx😀");
        }

        #[test]
        fn tap_lon_tuong_duong_ban_tuyen_tinh() {
            // 300 scalar thường + trùng + astral ⇒ chắc chắn đi nhánh bảng tra nhị phân.
            let mut bo = String::new();
            for c in (0x100u32..0x100 + 300).filter_map(char::from_u32) {
                bo.push(c);
            }
            for c in ['đ', '😀', 'đ', '😀', '\u{10FFFF}', '\u{10FFFF}'] {
                bo.push(c);
            }
            assert!(bo.chars().count() > NGUONG_BANG_TRA);
            let ca = [
                "",
                "x",
                "đ😀x😀đ",
                "đđ😀😀x",
                "xđ😀",
                "abcđ😀xyz",
                "\u{100}\u{12b}MID\u{12b}\u{100}",
                "\u{10FFFF}x\u{10FFFF}",
                "😀😀😀",
            ];
            for name in TEN {
                for s in ca {
                    assert_eq!(
                        cat(name, s, &bo),
                        cat_tuyen_tinh(name, s, &bo),
                        "lệch kết quả: {name}({s:?}, tập lớn)"
                    );
                }
            }
        }

        #[test]
        fn nguong_bang_tra_bien() {
            // 8 scalar ⇒ tuyến tính; 9 scalar ⇒ bảng tra — cả hai phải cùng kết quả.
            let tam = "abcdefgh";
            let chin = "abcdefghi";
            assert_eq!(tam.chars().count(), NGUONG_BANG_TRA);
            for (bo, goc, mong) in [
                (tam, "abcdefghXhgfedcba", ["X", "Xhgfedcba", "abcdefghX"]),
                (
                    chin,
                    "abcdefghiXihgfedcba",
                    ["X", "Xihgfedcba", "abcdefghiX"],
                ),
            ] {
                assert_eq!(cat("str.trim_matches", goc, bo), mong[0]);
                assert_eq!(cat("str.trim_start_matches", goc, bo), mong[1]);
                assert_eq!(cat("str.trim_end_matches", goc, bo), mong[2]);
                assert_eq!(cat("str.trim_matches", "X", bo), "X");
            }
        }
    }
}
