// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `str_d` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod strsplitincl {
    //! **VD-270 (split-inclusive)** — `std.str.split_inclusive(s, sep) -> list[str]`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM), **đúng** `str::split_inclusive` của Rust:
    //! - Tách **trái → phải** tại mỗi lần xuất hiện **không chồng** của `sep`; **mỗi phần tử**
    //!   **giữ luôn dấu phân cách** đã tách (trừ phần cuối khi `s` không kết thúc bằng `sep`):
    //!   `split_inclusive("a,b,c", ",") == ["a,", "b,", "c"]`.
    //! - Không khớp (hoặc `sep` dài hơn `s`) ⇒ `[s]`, **không** rỗng.
    //! - Dấu phân cách ở **cuối** ⇒ phần tử cuối **kết thúc bằng** `sep`, **không** sinh phần rỗng
    //!   thêm: `split_inclusive("a,b,", ",") == ["a,", "b,"]`.
    //! - Dấu phân cách **liên tiếp** ⇒ mỗi dấu là một phần tử riêng (phần tử có thể chỉ là `sep`):
    //!   `split_inclusive("a,b,,c", ",") == ["a,", "b,", ",", "c"]`.
    //! - Chồng lấn **không** được khớp lại: `split_inclusive("aaa", "aa") == ["aa", "a"]`,
    //!   `split_inclusive("abab", "aba") == ["aba", "b"]`.
    //! - `sep` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu/cuối,
    //!   nên `split_inclusive("abc", "") == ["", "a", "b", "c"]` (**không** có phần rỗng cuối);
    //!   `split_inclusive("", "") == [""]`; `split_inclusive("a", "") == ["", "a"]`. Khác
    //!   `str.split` của VS (rỗng ⇒ tách scalar, **không** sinh phần rỗng đầu).
    //! - `s` rỗng + `sep` **khác** rỗng ⇒ **`[]`** — probe `rustc 1.100.0` xác nhận
    //!   `"".split_inclusive(",")` trả iterator **rỗng** (KHÔNG phải `[""]`); giữ đúng nghĩa Rust.
    //! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
    //!   **không bao giờ** cắt giữa scalar: `split_inclusive("đ😀x😀", "😀") == ["đ😀", "x😀"]`;
    //!   ZWJ trong emoji không phải biên đặc biệt: `split_inclusive("👨‍👩‍👧x", "👩") ==
    //!   ["👨‍👩", "‍👧x"]`, phần còn lại vẫn là UTF-8 hợp lệ.
    //!
    //! Không trap, không có trần riêng: số phần tử bị chặn bởi độ dài `s` + 1 (`sep` rỗng) nên
    //! cấp phát tuyến tính theo `s`; mỗi phần tử được cấp phát đúng một chuỗi kết quả.

    use vis_syntax::Span;

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Tách `s` theo `sep`, **giữ** dấu phân cách ở cuối mỗi phần (trừ phần cuối nếu không có dấu).
    /// Dùng **đúng** `str::split_inclusive` của Rust nên `sep` rỗng và `s` rỗng cũng theo nghĩa Rust.
    pub(crate) fn tach(s: &str, sep: &str) -> Vec<String> {
        s.split_inclusive(sep).map(str::to_string).collect()
    }

    /// Dispatch `str.split_inclusive`: luôn `list[str]`; không trap.
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

        #[test]
        fn giu_dau_phan_cach_va_khong_sinh_phan_rong_cuoi() {
            assert_eq!(tach("a,b,c", ","), ds(&["a,", "b,", "c"]));
            assert_eq!(tach("aXbXc", "X"), ds(&["aX", "bX", "c"]));
            assert_eq!(tach("a--b--", "--"), ds(&["a--", "b--"]));
            // Dấu cuối **không** sinh phần rỗng thêm.
            assert_eq!(tach("a,b,", ","), ds(&["a,", "b,"]));
            assert_eq!(tach(",", ","), ds(&[","]));
            assert_eq!(tach(",,", ","), ds(&[",", ","]));
            assert_eq!(tach(",a", ","), ds(&[",", "a"]));
            assert_eq!(
                tach("a,b,,c", ","),
                ds(&["a,", "b,", ",", "c"]),
                "dấu liên tiếp là phần tử riêng"
            );
        }

        #[test]
        fn khong_khop_tra_nguyen_s() {
            assert_eq!(tach("abc", ","), ds(&["abc"]));
            assert_eq!(tach("a,b,c", "x"), ds(&["a,b,c"]));
            assert_eq!(tach("abc", "abcd"), ds(&["abc"]), "sep dài hơn s");
            assert_eq!(tach("ab", "abc"), ds(&["ab"]));
            assert_eq!(tach("abc", "abc"), ds(&["abc"]));
            assert_eq!(tach("abc", "ac"), ds(&["abc"]));
        }

        #[test]
        fn chong_lan_khong_khop_lai() {
            assert_eq!(tach("aaa", "aa"), ds(&["aa", "a"]));
            assert_eq!(tach("aaaa", "aa"), ds(&["aa", "aa"]));
            assert_eq!(tach("abab", "aba"), ds(&["aba", "b"]));
            assert_eq!(tach("banana", "ana"), ds(&["bana", "na"]));
            assert_eq!(tach("aaaaa", "aa"), ds(&["aa", "aa", "a"]));
        }

        #[test]
        fn sep_rong_theo_nghia_rust() {
            assert_eq!(tach("abc", ""), ds(&["", "a", "b", "c"]));
            assert_eq!(tach("a", ""), ds(&["", "a"]));
            assert_eq!(tach("", ""), ds(&[""]));
            assert_eq!(tach("đ😀", ""), ds(&["", "đ", "😀"]));
            assert_eq!(tach("a\r\nb", ""), ds(&["", "a", "\r", "\n", "b"]));
        }

        #[test]
        fn s_rong_theo_nghia_rust() {
            // Probe `rustc 1.100.0`: `"".split_inclusive(sep)` với `sep` khác rỗng là **rỗng**
            // (khác `str.split` của VS trả `[""]`); đây là hợp đồng **có chủ ý**.
            assert_eq!(tach("", ","), ds(&[]));
            assert_eq!(tach("", "abc"), ds(&[]));
            assert_eq!(tach("", ""), ds(&[""]));
        }

        #[test]
        fn unicode_emoji_zwj_tren_bien_scalar() {
            assert_eq!(tach("đ,😀,x", ","), ds(&["đ,", "😀,", "x"]));
            assert_eq!(tach("a😀b😀c", "😀"), ds(&["a😀", "b😀", "c"]));
            assert_eq!(tach("a🌍b🌍", "🌍"), ds(&["a🌍", "b🌍"]));
            assert_eq!(tach("héllo,🌍,👨‍👩‍👧‍👦", ","), ds(&["héllo,", "🌍,", "👨‍👩‍👧‍👦"]));
            // ZWJ **không** phải biên đặc biệt: cắt đúng quanh scalar `👩`, phần sau vẫn hợp lệ.
            assert_eq!(tach("👨‍👩‍👧x", "👩"), ds(&["👨‍👩", "‍👧x"]));
            assert_eq!(tach("👨‍👩‍👧‍👦x", "x"), ds(&["👨‍👩‍👧‍👦x"]), "không khớp ⇒ nguyên chuỗi");
            assert_eq!(tach("đ😀x", "😀a"), ds(&["đ😀x"]));
        }

        #[test]
        fn dau_phan_cach_dieu_khien_va_nhieu_byte() {
            assert_eq!(tach("a\r\nb\r\n", "\r\n"), ds(&["a\r\n", "b\r\n"]));
            assert_eq!(tach("a\0b\0", "\0"), ds(&["a\0", "b\0"]));
            assert_eq!(tach("a\tb\t", "\t"), ds(&["a\t", "b\t"]));
            assert_eq!(tach("a b ", " "), ds(&["a ", "b "]));
        }
    }
}

pub(crate) mod strsplitn {
    //! **VD-270 (splitn)** — `std.str.splitn(s, sep, n) -> list[str]`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Tách **trái → phải**, tối đa `n` phần; **phần cuối** là **phần còn lại nguyên văn** của
    //!   `s`, kể cả các dấu phân cách chưa dùng: `splitn("a,b,c", ",", 2) == ["a", "b,c"]`.
    //! - `n == 0` ⇒ `[]`; `n == 1` ⇒ `[s]`; `n > số phần` ⇒ **toàn bộ** phần như `str.split`.
    //! - Dấu phân cách ở **cuối** `s` sinh phần tử rỗng cuối: `splitn("a,b,", ",", 3) == ["a", "b", ""]`.
    //! - Đơn vị khớp là **chuỗi con UTF-8** (cùng `str.split`/`str.find`), **không bao giờ** cắt giữa
    //!   scalar: `splitn("đ😀x", "😀", 2) == ["đ", "x"]`; kết quả luôn là UTF-8 hợp lệ.
    //! - `sep` rỗng theo **đúng nghĩa Rust `str::splitn`** (khác `str.split` của VS: `str.split` rỗng
    //!   tách theo **scalar** và **không** sinh phần rỗng đầu/cuối). Bảng đã kiểm bằng probe Rust:
    //!
    //!   | biểu thức | kết quả |
    //!   |---|---|
    //!   | `"abc".splitn(2, "")` | `["", "abc"]` |
    //!   | `"abc".splitn(3, "")` | `["", "a", "bc"]` |
    //!   | `"abc".splitn(1, "")` | `["abc"]` |
    //!   | `"abc".splitn(0, "")` | `[]` |
    //!   | `"".splitn(2, "")` | `["", ""]` |
    //!   | `"".splitn(1, "")` | `[""]` |
    //!   | `"a".splitn(3, "")` | `["", "a", ""]` |
    //!   | `"đ😀".splitn(3, "")` | `["", "đ", "😀"]` |
    //!
    //! - `n < 0` là trap `VS0501` nêu `str.splitn` (kiểm **trước** khi tách); `n` là `int`.
    //!
    //! Không có trần riêng: số phần bị chặn bởi `n` **và** độ dài `s` (nhiều nhất `len(s) + 2` khi
    //! `sep` rỗng), nên không cấp phát theo `n`; mọi phép tuyến tính theo `s`.

    use vis_syntax::Span;

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Tách `s` theo `sep`, tối đa `n` phần; `n <= 0` ⇒ `[]`. Dùng **đúng** `str::splitn` của Rust
    /// nên `sep` rỗng cũng theo nghĩa Rust (khớp tại mọi biên scalar, kể cả đầu/cuối).
    pub(crate) fn phan(s: &str, sep: &str, n: i64) -> Vec<String> {
        if n <= 0 {
            return Vec::new();
        }
        // `n` dương; trên nền 32-bit, `i64` vượt `usize` ⇒ coi như vô hạn (mọi phần).
        let n = usize::try_from(n).unwrap_or(usize::MAX);
        s.splitn(n, sep).map(str::to_string).collect()
    }

    /// Dispatch `str.splitn`; `n < 0` ⇒ trap `VS0501` (cùng mẫu `str.repeat`/`list.windows`).
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let s = super::super::super::chuoi(name, &args[0], 0, span)?;
        let sep = super::super::super::chuoi(name, &args[1], 1, span)?;
        let n = super::super::super::so(name, &args[2], 2, span)?;
        if n < 0 {
            return Err(super::super::super::loi(
                format!("`{name}`: số phần `n` phải ≥ 0, được {n}"),
                span,
            ));
        }
        Ok(Value::List(
            phan(s, sep, n).into_iter().map(Value::Str).collect(),
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[&str]) -> Vec<String> {
            xs.iter().map(|x| x.to_string()).collect()
        }

        #[test]
        fn toi_da_n_phan_va_phan_cuoi_giu_nguyen() {
            assert_eq!(phan("a,b,c", ",", 2), ds(&["a", "b,c"]));
            assert_eq!(phan("a,b,c", ",", 1), ds(&["a,b,c"]));
            assert_eq!(phan("a,b,c", ",", 0), ds(&[]));
            assert_eq!(phan("a,b,c", ",", 3), ds(&["a", "b", "c"]));
            assert_eq!(phan("a,b,c", ",", 99), ds(&["a", "b", "c"]));
            assert_eq!(phan("abc", ",", 2), ds(&["abc"]));
            assert_eq!(phan("", ",", 2), ds(&[""]));
            assert_eq!(phan("", ",", 0), ds(&[]));
        }

        #[test]
        fn dau_phan_cach_cuoi_sinh_phan_rong() {
            assert_eq!(phan("a,b,", ",", 2), ds(&["a", "b,"]));
            assert_eq!(phan("a,b,", ",", 3), ds(&["a", "b", ""]));
            assert_eq!(phan("a,b,", ",", 4), ds(&["a", "b", ""]));
            assert_eq!(phan(",", ",", 1), ds(&[","]));
            assert_eq!(phan(",", ",", 2), ds(&["", ""]));
            assert_eq!(phan(",,", ",", 2), ds(&["", ","]));
            assert_eq!(phan(",,", ",", 3), ds(&["", "", ""]));
        }

        #[test]
        fn sep_rong_theo_nghia_rust() {
            assert_eq!(phan("abc", "", 2), ds(&["", "abc"]));
            assert_eq!(phan("abc", "", 3), ds(&["", "a", "bc"]));
            assert_eq!(phan("abc", "", 1), ds(&["abc"]));
            assert_eq!(phan("abc", "", 0), ds(&[]));
            assert_eq!(phan("abc", "", 9), ds(&["", "a", "b", "c", ""]));
            assert_eq!(phan("", "", 1), ds(&[""]));
            assert_eq!(phan("", "", 2), ds(&["", ""]));
            assert_eq!(phan("", "", 3), ds(&["", ""]));
            assert_eq!(phan("a", "", 2), ds(&["", "a"]));
            assert_eq!(phan("a", "", 3), ds(&["", "a", ""]));
            assert_eq!(phan("a", "", 9), ds(&["", "a", ""]));
        }

        #[test]
        fn unicode_theo_bien_scalar() {
            assert_eq!(phan("đ😀", "", 3), ds(&["", "đ", "😀"]));
            assert_eq!(phan("đ😀", "", 4), ds(&["", "đ", "😀", ""]));
            assert_eq!(phan("đ,😀,x", ",", 2), ds(&["đ", "😀,x"]));
            assert_eq!(phan("a😀b😀c", "😀", 2), ds(&["a", "b😀c"]));
            assert_eq!(phan("đ😀x", "😀", 2), ds(&["đ", "x"]));
            assert_eq!(phan("👨‍👩‍👧x", "👩", 2), ds(&["👨‍", "‍👧x"]));
        }
    }
}

pub(crate) mod strsplitonce {
    //! **VD-270 (split-once)** — `std.str.split_once(s, sep) -> list[str]?`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Tách tại **lần xuất hiện đầu tiên** của `sep`, **không** tách các lần sau:
    //!   `split_once("a,b,c", ",") == Some(["a", "b,c"])`, `split_once("aXbXc", "X") ==
    //!   Some(["a", "bXc"])`.
    //! - Không có `sep` trong `s` (hoặc `sep` dài hơn `s`) ⇒ `None`.
    //! - `sep` rỗng ⇒ `Some(["", s])`; `s` rỗng + `sep` rỗng ⇒ `Some(["", ""])`; `s` rỗng + `sep`
    //!   khác rỗng ⇒ `None`. Đúng nghĩa Rust `str::split_once` (probe xác nhận; **không** trap).
    //! - Dấu phân cách ở **đầu** ⇒ phần đầu rỗng: `split_once("abc", "a") == Some(["", "bc"])`;
    //!   ở **cuối** ⇒ phần sau rỗng: `split_once("abc", "c") == Some(["ab", ""])`.
    //! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
    //!   **không bao giờ** cắt giữa scalar: `split_once("đ😀x", "😀") == Some(["đ", "x"])`;
    //!   ZWJ trong emoji không phải biên đặc biệt: `split_once("👨‍👩‍👧x", "👩")` cắt đúng
    //!   quanh `👩` và phần còn lại vẫn là UTF-8 hợp lệ.
    //! - `Some` luôn có **đúng 2** phần tử; chỉ cấp phát **hai** chuỗi kết quả (không cắt thêm,
    //!   không cấp phát theo độ dài `s`).

    use vis_syntax::Span;

    use vis_core::value::Value;

    /// Tách `s` tại lần khớp **đầu** của `sep`; `None` nếu không khớp. Dùng **đúng**
    /// `str::split_once` của Rust nên `sep` rỗng cũng theo nghĩa Rust.
    pub(crate) fn tach<'a>(s: &'a str, sep: &str) -> Option<(&'a str, &'a str)> {
        s.split_once(sep)
    }

    /// Dispatch `str.split_once`: `Some(list)` với **đúng 2** phần, hoặc `None`; không trap.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> vis_core::trap::R<Value> {
        let s = super::super::super::chuoi(name, &args[0], 0, span)?;
        let sep = super::super::super::chuoi(name, &args[1], 1, span)?;
        Ok(Value::opt(tach(s, sep).map(|(dau, sau)| {
            Value::List(vec![
                Value::Str(dau.to_string()),
                Value::Str(sau.to_string()),
            ])
        })))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn hai(a: &str, b: &str) -> Option<(String, String)> {
            tach(a, b).map(|(x, y)| (x.to_string(), y.to_string()))
        }

        #[test]
        fn chi_tach_lan_dau() {
            assert_eq!(hai("a,b,c", ","), Some(("a".into(), "b,c".into())));
            assert_eq!(hai("a::b::c", "::"), Some(("a".into(), "b::c".into())));
            assert_eq!(hai("aXbXc", "X"), Some(("a".into(), "bXc".into())));
            assert_eq!(hai("aaaa", "aa"), Some(("".into(), "aa".into())));
            assert_eq!(hai("aaa", "aa"), Some(("".into(), "a".into())));
        }

        #[test]
        fn dau_va_cuoi_sinh_phan_rong() {
            assert_eq!(hai("abc", "a"), Some(("".into(), "bc".into())));
            assert_eq!(hai("abc", "c"), Some(("ab".into(), "".into())));
            assert_eq!(hai("đ", "đ"), Some(("".into(), "".into())));
            assert_eq!(hai(",", ","), Some(("".into(), "".into())));
            assert_eq!(hai(",a,", ","), Some(("".into(), "a,".into())));
        }

        #[test]
        fn khong_khop_tra_none() {
            assert_eq!(hai("a,b,c", "x"), None);
            assert_eq!(hai("abc", "abcd"), None, "sep dài hơn s");
            assert_eq!(hai("", "a"), None, "s rỗng + sep khác rỗng");
            assert_eq!(hai("", "ab"), None);
            assert_eq!(hai("abc", "ac"), None);
            assert_eq!(hai("đ😀x", "😀a"), None);
        }

        #[test]
        fn sep_rong_theo_nghia_rust() {
            assert_eq!(hai("abc", ""), Some(("".into(), "abc".into())));
            assert_eq!(hai("", ""), Some(("".into(), "".into())));
            assert_eq!(hai("đ😀", ""), Some(("".into(), "đ😀".into())));
        }

        #[test]
        fn unicode_emoji_theo_chuoi_con() {
            assert_eq!(hai("đ😀x", "😀"), Some(("đ".into(), "x".into())));
            assert_eq!(hai("ađb", "đ"), Some(("a".into(), "b".into())));
            assert_eq!(hai("đ,😀,x", ","), Some(("đ".into(), "😀,x".into())));
            assert_eq!(
                hai("👨‍👩‍👧x", "👩"),
                Some(("👨‍".into(), "‍👧x".into())),
                "ZWJ là scalar thường; cắt quanh 👩 vẫn hợp lệ UTF-8"
            );
            assert_eq!(hai("👨‍👩‍👧", "👨‍👩‍👧"), Some(("".into(), "".into())));
        }

        #[test]
        fn goi_tra_option_that_hai_phan() {
            use vis_core::value::eq;
            let some = |a: &str, b: &str| {
                Value::opt(Some(Value::List(vec![
                    Value::Str(a.into()),
                    Value::Str(b.into()),
                ])))
            };
            let args = |s: &str, sep: &str| vec![Value::Str(s.into()), Value::Str(sep.into())];
            let span = Span::new(0, 0);
            assert!(eq(
                &goi("str.split_once", &args("a,b,c", ","), span).unwrap(),
                &some("a", "b,c")
            ));
            assert!(eq(
                &goi("str.split_once", &args("abc", ""), span).unwrap(),
                &some("", "abc")
            ));
            assert!(eq(
                &goi("str.split_once", &args("abc", "x"), span).unwrap(),
                &Value::none()
            ));
        }
    }
}
