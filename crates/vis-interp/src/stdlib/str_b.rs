// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `str_b` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod strlen {
    //! **VD-270 (strlen)** — độ dài chuỗi và rỗng: `std.str.byte_len` / `char_count` / `is_empty`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `byte_len(s)` = độ dài **byte UTF-8** (`str::len` của Rust) — `"đ"` = 2, `"😀"` = 4.
    //! - `char_count(s)` = số **scalar Unicode** (`chars().count()`), **không** phải byte và
    //!   **không** phải grapheme: `"đ😀x"` = 3; `"e\u{301}"` (e + dấu) = **2**.
    //! - `is_empty(s)` = `s` rỗng **byte** (tương đương 0 scalar) ⇒ `bool`.
    //! - Không có trần riêng: mọi phép là O(độ dài chuỗi), không cấp phát.
    //!
    //! **Không** có `grapheme_count`: đếm cụm grapheme cần bảng phân đoạn Unicode
    //! (`unicode-segmentation`) — một dependency mới, ngoài phạm vi VD-270 (zero-dep). Đếm
    //! grapheme sẽ đến khi A0 chốt dependency; `char_count` chỉ đếm scalar.

    use crate::value::Value;

    /// Độ dài **byte UTF-8** của `s` (`"đ😀x"` ⇒ 7: 2 + 4 + 1).
    pub(crate) fn byte_len(s: &str) -> i64 {
        s.len() as i64
    }

    /// Số **scalar Unicode** của `s` (`"đ😀x"` ⇒ 3), cùng đơn vị `chars()`.
    pub(crate) fn char_count(s: &str) -> i64 {
        s.chars().count() as i64
    }

    /// `s` rỗng hay không (0 byte ⇔ 0 scalar).
    pub(crate) fn is_empty(s: &str) -> bool {
        s.is_empty()
    }

    /// Dispatch chung cho ba hàm; `name` là tên đầy đủ `"str.byte_len"`/`"str.char_count"`/
    /// `"str.is_empty"`.
    pub(crate) fn goi(name: &str, s: &str) -> Value {
        match name {
            "str.byte_len" => Value::Int(byte_len(s)),
            "str.char_count" => Value::Int(char_count(s)),
            _ => Value::Bool(is_empty(s)),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn byte_len_la_utf8_khong_phai_scalar() {
            assert_eq!(byte_len(""), 0);
            assert_eq!(byte_len("abc"), 3);
            assert_eq!(byte_len("đ"), 2);
            assert_eq!(byte_len("😀"), 4);
            assert_eq!(byte_len("đ😀x"), 7);
        }

        #[test]
        fn char_count_la_scalar_unicode() {
            assert_eq!(char_count(""), 0);
            assert_eq!(char_count("abc"), 3);
            assert_eq!(char_count("đ"), 1);
            assert_eq!(char_count("😀"), 1);
            assert_eq!(char_count("đ😀x"), 3);
            // Dấu kết hợp là scalar **riêng** — char_count không phải grapheme count.
            assert_eq!(char_count("e\u{301}"), 2);
            assert_eq!(byte_len("e\u{301}"), 3);
        }

        #[test]
        fn is_empty_theo_byte_va_scalar() {
            assert!(is_empty(""));
            assert!(!is_empty(" "));
            assert!(!is_empty("đ"));
            assert!(!is_empty("😀"));
        }
    }
}

pub(crate) mod strmatches {
    //! **VD-270 (matches)** — `std.str.matches(s, pat) -> list[str]`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM), **đúng** `str::matches` của Rust:
    //! - Trả **các chuỗi con** khớp `pat`, theo thứ tự **trái → phải**, khớp **không chồng** (sau
    //!   mỗi lần khớp, vị trí tìm tiếp nằm **sau** phần đã khớp): `matches("banana", "ana")` ⇒
    //!   `["ana"]`; `matches("aaaa", "aa")` ⇒ `["aa", "aa"]`; `matches("aaa", "aa")` ⇒ `["aa"]`.
    //! - Không khớp/`pat` dài hơn `s`/`s` rỗng + `pat` khác rỗng ⇒ `[]` (khác `str.split` trả
    //!   `[s]`).
    //! - `pat` rỗng theo **đúng nghĩa Rust**: khớp rỗng tại **mọi biên scalar** kể cả đầu/cuối ⇒
    //!   `matches("abc", "")` ⇒ `["", "", "", ""]` (`len` = 4), `matches("", "")` ⇒ `[""]`
    //!   (`len` = 1), `matches("đ😀x😀", "")` ⇒ 5 phần tử rỗng.
    //! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.find`/`str.split`),
    //!   **không bao giờ** cắt giữa scalar: `matches("đ😀x😀", "😀")` ⇒ `["😀", "😀"]`. ZWJ trong
    //!   emoji không phải biên đặc biệt: `matches("👨‍👩‍👧x", "👩")` ⇒ `["👩"]`; cả cụm ZWJ vẫn
    //!   khớp nguyên vẹn.
    //! - Không trap, không có trần riêng: mỗi lần khớp cấp phát **đúng một** chuỗi kết quả; số
    //!   phần tử bị chặn bởi `len_scalar(s) + 1` khi `pat` rỗng.
    //!
    //! Bảng probe `rustc 1.100.0` (toolchain ghim; chạy lại trong unit test bên dưới):
    //!
    //! | biểu thức | kết quả |
    //! |---|---|
    //! | `"banana".matches("ana")` | `["ana"]` |
    //! | `"aaaa".matches("aa")` | `["aa", "aa"]` |
    //! | `"abab".matches("aba")` | `["aba"]` |
    //! | `"abc".matches("abcd")` | `[]` |
    //! | `"abc".matches("x")` | `[]` |
    //! | `"".matches("")` | `[""]` |
    //! | `"abc".matches("")` | `["", "", "", ""]` |
    //! | `"đ😀x😀".matches("😀")` | `["😀", "😀"]` |
    //! | `"👨‍👩‍👧‍👦x👨‍👩‍👧‍👦".matches("👨‍👩‍👧‍👦")` | `["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]` |
    //! | `"👨‍👩‍👧x".matches("👩")` | `["👩"]` |
    //! | `"đ😀x😀".matches("")` | `["", "", "", "", ""]` |
    //! | `"aaa".matches("aa")` | `["aa"]` |
    //! | `"ababa".matches("aba")` | `["aba"]` |
    //! | `"abcabc".matches("abc")` | `["abc", "abc"]` |
    //! | `"😀😀😀".matches("😀😀")` | `["😀😀"]` |

    use vis_syntax::Span;

    use crate::interp::R;
    use crate::value::Value;

    /// Các chuỗi con khớp `pat` theo **đúng** `str::matches` của Rust (không chồng, trái → phải);
    /// `pat` rỗng khớp tại mọi biên scalar.
    pub(crate) fn khop(s: &str, pat: &str) -> Vec<String> {
        s.matches(pat).map(str::to_string).collect()
    }

    /// Dispatch `str.matches`: luôn `list[str]`; không trap.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let s = super::super::chuoi(name, &args[0], 0, span)?;
        let pat = super::super::chuoi(name, &args[1], 1, span)?;
        Ok(Value::List(
            khop(s, pat).into_iter().map(Value::Str).collect(),
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[&str]) -> Vec<String> {
            xs.iter().map(|x| x.to_string()).collect()
        }

        #[test]
        fn bang_probe_rust() {
            assert_eq!(khop("banana", "ana"), ds(&["ana"]));
            assert_eq!(khop("aaaa", "aa"), ds(&["aa", "aa"]));
            assert_eq!(khop("abab", "aba"), ds(&["aba"]));
            assert_eq!(khop("abc", "abcd"), ds(&[]));
            assert_eq!(khop("abc", "x"), ds(&[]));
            assert_eq!(khop("", ""), ds(&[""]));
            assert_eq!(khop("abc", ""), ds(&["", "", "", ""]));
            assert_eq!(khop("đ😀x😀", "😀"), ds(&["😀", "😀"]));
            assert_eq!(khop("👨‍👩‍👧‍👦x👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"), ds(&["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]));
            assert_eq!(khop("👨‍👩‍👧x", "👩"), ds(&["👩"]));
            assert_eq!(khop("đ😀x😀", ""), ds(&["", "", "", "", ""]));
            assert_eq!(khop("aaa", "aa"), ds(&["aa"]));
            assert_eq!(khop("ababa", "aba"), ds(&["aba"]));
            assert_eq!(khop("abcabc", "abc"), ds(&["abc", "abc"]));
            assert_eq!(khop("😀😀😀", "😀😀"), ds(&["😀😀"]));
        }

        #[test]
        fn khong_chong_va_khong_quay_lai() {
            assert_eq!(khop("aaaaa", "aa"), ds(&["aa", "aa"]));
            assert_eq!(khop("banana", "ana"), ds(&["ana"]));
            assert_eq!(khop("banana", "na"), ds(&["na", "na"]));
            assert_eq!(khop("aaaa", "aaa"), ds(&["aaa"]));
            assert_eq!(khop("ababab", "ab"), ds(&["ab", "ab", "ab"]));
        }

        #[test]
        fn khong_khop_va_s_rong() {
            assert_eq!(khop("abc", "z"), ds(&[]));
            assert_eq!(khop("abc", "abcd"), ds(&[]));
            assert_eq!(khop("", "a"), ds(&[]));
            assert_eq!(khop("", "abc"), ds(&[]));
            // `pat` rỗng là ca duy nhất `s` rỗng vẫn khớp (một phần tử rỗng).
            assert_eq!(khop("", ""), ds(&[""]));
        }

        #[test]
        fn pat_rong_theo_nghia_rust() {
            assert_eq!(khop("abc", ""), ds(&["", "", "", ""]));
            assert_eq!(khop("a", ""), ds(&["", ""]));
            assert_eq!(khop("", ""), ds(&[""]));
            assert_eq!(khop("đ😀", ""), ds(&["", "", ""]));
            assert_eq!(khop("a\r\nb", ""), ds(&["", "", "", "", ""]));
        }

        #[test]
        fn unicode_emoji_zwj_tren_bien_scalar() {
            assert_eq!(khop("đ😀x😀", "😀"), ds(&["😀", "😀"]));
            assert_eq!(khop("a😀b😀c", "😀"), ds(&["😀", "😀"]));
            assert_eq!(khop("héllo,🌍,👨‍👩‍👧‍👦", ","), ds(&[",", ","]));
            // ZWJ không phải biên đặc biệt; phần khớp vẫn là UTF-8 hợp lệ.
            assert_eq!(khop("👨‍👩‍👧x", "👩"), ds(&["👩"]));
            assert_eq!(khop("👨‍👩‍👧x", "👨‍"), ds(&["👨‍"]));
            assert_eq!(khop("👨‍👩‍👧‍👦x👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"), ds(&["👨‍👩‍👧‍👦", "👨‍👩‍👧‍👦"]));
            assert_eq!(khop("😀😀😀", "😀😀"), ds(&["😀😀"]));
        }

        #[test]
        fn khong_cat_giua_scalar() {
            // `pat` luôn là chuỗi UTF-8 hợp lệ nên mọi lần khớp bắt đầu/kết thúc trên biên scalar;
            // dấu kết hợp là scalar **riêng** (không phải grapheme) nên khớp được một mình.
            assert_eq!(khop("e\u{301}", "\u{301}"), ds(&["\u{301}"]));
            assert_eq!(khop("đ", "đ"), ds(&["đ"]));
            assert_eq!(khop("😀", "\u{1F600}"), ds(&["😀"]));
            assert_eq!(khop("xđy", "đ"), ds(&["đ"]));
            assert_eq!(khop("đ", "x"), ds(&[]));
        }

        #[test]
        fn goi_tra_list_that() {
            let v = goi(
                "str.matches",
                &[Value::Str("abcabc".into()), Value::Str("abc".into())],
                Span::new(0, 0),
            )
            .expect("không trap");
            match v {
                Value::List(xs) => {
                    assert_eq!(xs.len(), 2);
                    assert!(matches!(&xs[0], Value::Str(s) if s == "abc"));
                    assert!(matches!(&xs[1], Value::Str(s) if s == "abc"));
                }
                other => panic!("phải là list, được {}", other.show()),
            }
        }
    }
}

pub(crate) mod strpad {
    //! **VD-269** — đệm chuỗi `std.str.pad_left` / `pad_right` / `center` theo **ký tự Unicode**
    //! (scalar), không theo byte.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `width` = số **scalar** (`s.chars().count()`); `"đ"` và `"😀"` đều là **một** ký tự.
    //! - `width <= 0` hoặc `width <= len(s)` ⇒ trả `s` **nguyên vẹn**.
    //! - `fill` phải **khác rỗng** — kiểm **trước** cả khi không cần đệm; rỗng ⇒ trap `VS0501`.
    //! - phần đệm = `fill` **lặp rồi cắt** đúng số scalar cần (fill nhiều ký tự không bị cắt thô).
    //! - `center` chia đôi phần đệm; scalar **dư nằm bên phải**; mỗi phía **lặp `fill` từ đầu**.
    //! - trần phần đệm 10^7 scalar (cùng mức `str.repeat`) ⇒ `width` khổng lồ không OOM; vượt trần
    //!   là trap `VS0501`, **không** panic.

    use vis_syntax::Span;

    use super::super::loi;
    use crate::interp::Abort;

    /// Trần số scalar **thêm vào** (không tính `s`) — như `str.repeat` (10^7).
    pub(crate) const TRAN_DEM: i64 = 10_000_000;

    /// Đệm `s` lên `width` scalar bằng `fill`; `name` chọn phía (`str.pad_left` / `pad_right` /
    /// `center`). Trả `s` khi `width <= 0` hoặc `width <= len(s)`.
    pub(crate) fn dem(
        name: &str,
        s: &str,
        width: i64,
        fill: &str,
        span: Span,
    ) -> Result<String, Abort> {
        if fill.is_empty() {
            return Err(loi(format!("`{name}`: `fill` phải khác rỗng"), span));
        }
        let dai = s.chars().count() as i64;
        if width <= 0 || width <= dai {
            return Ok(s.to_string());
        }
        let can = width - dai;
        if can > TRAN_DEM {
            return Err(loi(
                format!("`{name}`: phần đệm quá lớn ({can} ký tự > 10^7)"),
                span,
            ));
        }
        let can = can as usize;
        let (trai, phai) = match name {
            "str.pad_left" => (can, 0),
            "str.pad_right" => (0, can),
            _ => (can / 2, can - can / 2),
        };
        let mut ra = String::new();
        ra.extend(fill.chars().cycle().take(trai));
        ra.push_str(s);
        ra.extend(fill.chars().cycle().take(phai));
        Ok(ra)
    }
}

pub(crate) mod strparse {
    //! **VD-270 (parse-int)** — `std.str.parse_int`: chuỗi thập phân → `int?`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Cú pháp **chặt** như `i64::from_str`: dấu `+`/`-` tuỳ chọn rồi **một hoặc nhiều** chữ số
    //!   ASCII `0..=9`; **không** khoảng trắng, `_`, `0x`, dấu thập phân, dấu `+`/`-` lặp.
    //! - Tràn `i64` (kể cả `i64::MIN`/`MAX` biên) ⇒ `None`; chuỗi rỗng ⇒ `None`.
    //! - Không trap, không cấp phát kết quả; sugar R5c `s.parse_int()`.

    use crate::value::Value;

    /// Parse **chặt** chuỗi thập phân thành `i64`; `None` nếu sai cú pháp/tràn.
    pub(crate) fn parse_int(s: &str) -> Option<i64> {
        s.parse::<i64>().ok()
    }

    /// Dispatch chung: `Some(int)` / `None`.
    pub(crate) fn goi(s: &str) -> Value {
        Value::opt(parse_int(s).map(Value::Int))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn co_ban_va_dau() {
            assert_eq!(parse_int("42"), Some(42));
            assert_eq!(parse_int("+7"), Some(7));
            assert_eq!(parse_int("-0"), Some(0));
            assert_eq!(parse_int("007"), Some(7));
        }

        #[test]
        fn bien_i64() {
            assert_eq!(parse_int("9223372036854775807"), Some(i64::MAX));
            assert_eq!(parse_int("-9223372036854775808"), Some(i64::MIN));
            assert_eq!(parse_int("9223372036854775808"), None);
            assert_eq!(parse_int("-9223372036854775809"), None);
        }

        #[test]
        fn sai_cu_phap() {
            for s in [
                "", " ", " 42", "42 ", "0x10", "1_000", "4.2", "abc", "+-1", "--1", "+", "-",
            ] {
                assert_eq!(parse_int(s), None, "s={s:?}");
            }
        }

        #[test]
        fn ket_qua_la_option_that() {
            let v = goi("5");
            assert!(v.is_some());
            assert!(matches!(v.opt_payload(), Some(Value::Int(5))));
            assert!(goi("x").is_none());
        }
    }
}

pub(crate) mod strparsefloat {
    //! **VD-270 (parse-float)** — `std.str.parse_float`: chuỗi → `float?`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Cú pháp **thập phân của Rust** (`f64::from_str`): dấu `+`/`-` tuỳ chọn, phần nguyên/thập
    //!   phân có thể khuyết **một** phía (`.5`, `5.`), mũ `e`/`E` kèm dấu; **không** khoảng trắng,
    //!   `_`, `0x`; chuỗi rỗng/sai cú pháp ⇒ `None`.
    //! - Sau parse **từ chối mọi giá trị không hữu hạn**: `inf`/`-inf`/`NaN` (literal của Rust) và
    //!   **tràn** (`1e400` ⇒ `inf`) ⇒ `None` — API này không biểu diễn non-finite.
    //! - **Underflow** (`1e-400` ⇒ `0.0`, `-1e-400` ⇒ `-0.0`) vẫn `Some` (hữu hạn, giữ dấu `-0`).
    //! - Không trap, không cấp phát kết quả; sugar R5c `s.parse_float()`.

    use crate::value::Value;

    /// Parse `f64` theo cú pháp Rust rồi **từ chối non-finite** (inf/NaN, kể cả do tràn).
    pub(crate) fn parse_float(s: &str) -> Option<f64> {
        let f = s.parse::<f64>().ok()?;
        if f.is_finite() {
            Some(f)
        } else {
            None
        }
    }

    /// Dispatch chung: `Some(float)` / `None`.
    pub(crate) fn goi(s: &str) -> Value {
        Value::opt(parse_float(s).map(Value::Float))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn hop_le_thap_phan_va_mu() {
            assert_eq!(parse_float("42"), Some(42.0));
            assert_eq!(parse_float("+1.5"), Some(1.5));
            assert_eq!(parse_float(".5"), Some(0.5));
            assert_eq!(parse_float("5."), Some(5.0));
            assert_eq!(parse_float("1e3"), Some(1000.0));
            assert_eq!(parse_float("1E-3"), Some(0.001));
            assert_eq!(parse_float("+.5"), Some(0.5));
            assert_eq!(parse_float("-.5"), Some(-0.5));
            assert_eq!(parse_float("5.e2"), Some(500.0));
            assert_eq!(parse_float(".5e2"), Some(50.0));
        }

        #[test]
        fn am_khong_va_underflow_giu_dau() {
            let z = parse_float("-0.0").expect("-0.0 hợp lệ");
            assert_eq!(z, 0.0);
            assert!(z.is_sign_negative(), "`-0.0` phải giữ dấu âm");
            let u = parse_float("-1e-400").expect("underflow vẫn hữu hạn");
            assert_eq!(u, 0.0);
            assert!(u.is_sign_negative(), "`-1e-400` ⇒ `-0.0`");
            assert_eq!(parse_float("1e-400"), Some(0.0));
        }

        #[test]
        fn khong_huu_han_va_tran_tra_none() {
            for s in [
                "inf",
                "-inf",
                "+inf",
                "infinity",
                "-infinity",
                "NaN",
                "nan",
                "-NaN",
                "1e400",
                "-1e400",
                "1e999999",
            ] {
                assert_eq!(parse_float(s), None, "s={s:?}");
            }
        }

        #[test]
        fn sai_cu_phap() {
            for s in [
                "", " ", " 42", "42 ", "0x10", "1_000", "1,5", "4.2.3", ".", "+", "-", "e3", "1e",
                "1e+", "++1", "--1", "abc", "1.5f64",
            ] {
                assert_eq!(parse_float(s), None, "s={s:?}");
            }
        }

        #[test]
        fn ket_qua_la_option_that() {
            let v = goi("1.5");
            assert!(v.is_some());
            assert!(matches!(v.opt_payload(), Some(Value::Float(f)) if *f == 1.5));
            assert!(goi("x").is_none());
            assert!(goi("inf").is_none());
        }
    }
}
