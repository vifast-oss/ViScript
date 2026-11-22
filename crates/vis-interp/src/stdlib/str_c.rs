// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `str_c` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod strreplace {
    //! **VD-270 (replace-first)** — thay **một lần** đầu tiên: `std.str.replace_first`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `replace_first(s, from, to)` thay **đúng lần xuất hiện đầu tiên** của `from` trong `s`
    //!   bằng `to`; mọi lần xuất hiện sau giữ nguyên. Cùng đơn vị khớp với `str.replace`/`find`
    //!   (chuỗi con theo **scalar Unicode**, không phải byte).
    //! - Không khớp ⇒ `s` nguyên vẹn.
    //! - `from` rỗng ⇒ **chèn** `to` một lần ở **đầu** (Rust `str::replacen(…, 1)`):
    //!   `replace_first("abc", "", "-") == "-abc"`, `replace_first("", "", "-") == "-"`.
    //! - `s` rỗng hoặc `to` rỗng theo cùng quy tắc; `to` rỗng nghĩa là **xoá** lần khớp đầu.
    //! - `to` được phép dài tùy ý: không có trần riêng (như `str.replace`), vì chỉ một phép thay
    //!   và kết quả nhiều nhất là `s` + `to` (không nhân theo số lần khớp).
    //!
    //! Không panic Rust: mọi đầu vào `str` hợp lệ đều có kết quả; `replacen` bảo toàn biên scalar.

    /// Thay lần xuất hiện **đầu tiên** của `from` bằng `to`; không khớp ⇒ bản sao nguyên vẹn của `s`.
    /// `from` rỗng ⇒ `to + s` (chèn một lần ở đầu), đúng nghĩa Rust `s.replacen(from, to, 1)`.
    pub(crate) fn replace_first(s: &str, from: &str, to: &str) -> String {
        s.replacen(from, to, 1)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn chi_thay_lan_dau() {
            assert_eq!(replace_first("aaa", "a", "b"), "baa");
            assert_eq!(replace_first("banana", "na", "X"), "baXna");
            assert_eq!(replace_first("ababab", "ab", "X"), "Xabab");
            assert_eq!(
                replace_first("hello world hello", "hello", "hi"),
                "hi world hello"
            );
        }

        #[test]
        fn khong_khop_tra_nguyen_ven() {
            assert_eq!(replace_first("abc", "z", "X"), "abc");
            assert_eq!(replace_first("abc", "abcd", "X"), "abc");
            assert_eq!(replace_first("", "a", "b"), "");
        }

        #[test]
        fn from_rong_chen_dau_mot_lan() {
            assert_eq!(replace_first("abc", "", "-"), "-abc");
            assert_eq!(replace_first("", "", "-"), "-");
            assert_eq!(replace_first("abc", "", ""), "abc");
            assert_eq!(replace_first("đ", "", "😀"), "😀đ");
        }

        #[test]
        fn s_rong_va_to_rong() {
            assert_eq!(replace_first("abc", "b", ""), "ac");
            assert_eq!(replace_first("abc", "abc", ""), "");
            assert_eq!(replace_first("a", "a", ""), "");
            assert_eq!(replace_first("đ😀", "😀", ""), "đ");
        }

        #[test]
        fn unicode_theo_scalar_khong_phai_byte() {
            assert_eq!(replace_first("đ😀x😀", "😀", "Z"), "đZx😀");
            assert_eq!(replace_first("đađ", "đ", "Đ"), "Đađ");
            assert_eq!(replace_first("😀😀😀", "😀", "🙂"), "🙂😀😀");
            assert_eq!(replace_first("đ😀x", "đ😀", "Q"), "Qx");
            assert_eq!(replace_first("đ😀x", "x", "đ"), "đ😀đ");
        }

        #[test]
        fn chong_lan_chi_mot_lan() {
            assert_eq!(replace_first("aaa", "aa", "b"), "ba");
            assert_eq!(replace_first("aaaa", "aa", "b"), "baa");
            assert_eq!(replace_first("aaa", "a", "aa"), "aaaa");
            assert_eq!(replace_first("aaaa", "aaa", "b"), "ba");
        }
    }
}

pub(crate) mod strreplacelast {
    //! **VD-270 (replace-last)** — thay **một lần** cuối cùng: `std.str.replace_last`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `replace_last(s, from, to)` thay **đúng lần xuất hiện cuối cùng** của `from` trong `s`
    //!   bằng `to`; mọi lần xuất hiện trước giữ nguyên. Cùng đơn vị khớp với
    //!   `str.replace`/`replace_first`/`rfind` (chuỗi con theo **scalar Unicode**, không phải byte).
    //! - Không khớp ⇒ `s` nguyên vẹn.
    //! - `from` rỗng ⇒ **nối** `to` một lần ở **cuối** (lần khớp rỗng cuối ở vị trí `len(s)`,
    //!   đúng nghĩa Rust `s.rfind("")`): `replace_last("abc", "", "-") == "abc-"`,
    //!   `replace_last("", "", "-") == "-"`.
    //! - `s` rỗng hoặc `to` rỗng theo cùng quy tắc; `to` rỗng nghĩa là **xoá** lần khớp cuối.
    //! - `to` được phép dài tùy ý: không có trần riêng (như `str.replace`), vì chỉ một phép thay
    //!   và kết quả nhiều nhất là `s` + `to` (không nhân theo số lần khớp).
    //!
    //! Không panic Rust: `rfind` chỉ trả offset trên biên scalar, nên phép cắt `s[..i]` /
    //! `s[i + from.len()..]` luôn là UTF-8 hợp lệ.

    /// Thay lần xuất hiện **cuối cùng** của `from` bằng `to`; không khớp ⇒ bản sao nguyên vẹn của
    /// `s`. `from` rỗng ⇒ `s + to` (nối một lần ở cuối), đúng nghĩa Rust `s.rfind("") == len(s)`.
    pub(crate) fn replace_last(s: &str, from: &str, to: &str) -> String {
        // `rfind` là đúng phép khớp của `str.rfind` (scalar Unicode, biên UTF-8); `from` rỗng
        // ⇒ `Some(s.len())` — lần khớp rỗng **cuối** — nên kết quả là `s + to`.
        match s.rfind(from) {
            Some(i) => {
                let mut out = String::with_capacity(s.len() + to.len());
                out.push_str(&s[..i]);
                out.push_str(to);
                out.push_str(&s[i + from.len()..]);
                out
            }
            None => s.to_string(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn chi_thay_lan_cuoi() {
            assert_eq!(replace_last("aaa", "a", "b"), "aab");
            assert_eq!(replace_last("banana", "na", "X"), "banaX");
            assert_eq!(replace_last("ababab", "ab", "X"), "ababX");
            assert_eq!(
                replace_last("hello world hello", "hello", "hi"),
                "hello world hi"
            );
        }

        #[test]
        fn khong_khop_tra_nguyen_ven() {
            assert_eq!(replace_last("abc", "z", "X"), "abc");
            assert_eq!(replace_last("abc", "abcd", "X"), "abc");
            assert_eq!(replace_last("", "a", "b"), "");
        }

        #[test]
        fn from_rong_noi_cuoi_mot_lan() {
            assert_eq!(replace_last("abc", "", "-"), "abc-");
            assert_eq!(replace_last("", "", "-"), "-");
            assert_eq!(replace_last("abc", "", ""), "abc");
            assert_eq!(replace_last("đ", "", "😀"), "đ😀");
        }

        #[test]
        fn s_rong_va_to_rong() {
            assert_eq!(replace_last("abc", "b", ""), "ac");
            assert_eq!(replace_last("abc", "abc", ""), "");
            assert_eq!(replace_last("a", "a", ""), "");
            assert_eq!(replace_last("đ😀", "😀", ""), "đ");
        }

        #[test]
        fn unicode_theo_scalar_khong_phai_byte() {
            assert_eq!(replace_last("đ😀x😀", "😀", "Z"), "đ😀xZ");
            assert_eq!(replace_last("đađ", "đ", "Đ"), "đaĐ");
            assert_eq!(replace_last("😀😀😀", "😀", "🙂"), "😀😀🙂");
            assert_eq!(replace_last("đ😀x", "đ😀", "Q"), "Qx");
            assert_eq!(replace_last("đ😀x", "x", "đ"), "đ😀đ");
            assert_eq!(replace_last("đ😀xđ😀y", "đ😀", "Q"), "đ😀xQy");
        }

        #[test]
        fn chong_lan_chon_lan_cuoi() {
            assert_eq!(replace_last("aaa", "aa", "b"), "ab");
            assert_eq!(replace_last("aaaa", "aa", "b"), "aab");
            assert_eq!(replace_last("aaa", "a", "aa"), "aaaa");
            assert_eq!(replace_last("aaaa", "aaa", "b"), "ab");
            assert_eq!(replace_last("ababa", "aba", "X"), "abX");
        }
    }
}

pub(crate) mod strreplacen {
    //! **VD-270 (replace-n)** — thay **tối đa `n` lần**: `std.str.replace_n`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - `replace_n(s, from, to, n)` thay **`n` lần khớp KHÔNG chồng** đầu tiên của `from` trong
    //!   `s` bằng `to`, quét **trái → phải**, đúng nghĩa Rust `str::replacen(from, to, n)`.
    //!   Cùng đơn vị khớp với `str.replace`/`replace_first`/`replace_last`/`find`
    //!   (chuỗi con theo **scalar Unicode**, không phải byte).
    //! - `n == 0` ⇒ `s` **nguyên vẹn**; `n` lớn hơn số lần khớp ⇒ thay **tất cả** (như
    //!   `str.replace`); không khớp ⇒ `s` nguyên vẹn. Chồng lấn: sau mỗi lần thay, phép quét tiếp
    //!   tục **sau** phần `from` vừa khớp (nên `replace_n("aaaa", "aa", "b", 2) == "bb"`).
    //! - `from` rỗng theo **đúng nghĩa Rust `replacen`**: khớp rỗng tại **mọi biên scalar** kể cả
    //!   đầu và cuối chuỗi; `n` lần khớp rỗng đầu tiên được chèn `to`:
    //!
    //!   | biểu thức | kết quả |
    //!   |---|---|
    //!   | `"abc".replacen("", "-", 0)` | `"abc"` |
    //!   | `"abc".replacen("", "-", 1)` | `"-abc"` |
    //!   | `"abc".replacen("", "-", 2)` | `"-a-bc"` |
    //!   | `"abc".replacen("", "-", 3)` | `"-a-b-c"` |
    //!   | `"abc".replacen("", "-", 4)` | `"-a-b-c-"` |
    //!   | `"abc".replacen("", "-", 9)` | `"-a-b-c-"` |
    //!   | `"".replacen("", "-", 1)` | `"-"` |
    //!   | `"".replacen("", "-", 9)` | `"-"` |
    //!   | `"đ".replacen("", "😀", 2)` | `"😀đ😀"` |
    //!
    //!   (bảng kiểm bằng probe Rust 1.100.0 — `str::replacen` của std.)
    //! - `s` rỗng hoặc `to` rỗng theo cùng quy tắc; `to` rỗng nghĩa là **xoá** các lần khớp.
    //! - `n < 0` là trap `VS0501` nêu `str.replace_n` (kiểm **trước** khi thay, cùng mẫu
    //!   `str.splitn`/`str.repeat`).
    //! - Không có trần riêng (như `str.replace`): số lần thay bị chặn bởi `n` **và** độ dài `s`
    //!   (nhiều nhất `len(s) + 1` khi `from` rỗng), nên không cấp phát theo `n`.
    //!
    //! Không panic Rust: `replacen` chỉ cắt trên biên scalar (UTF-8) nên kết quả luôn hợp lệ.

    /// Thay tối đa `n` lần khớp **không chồng** đầu tiên của `from` bằng `to`.
    /// `n == 0` ⇒ bản sao nguyên vẹn của `s`; dùng **đúng** `str::replacen` của Rust nên
    /// `from` rỗng cũng theo nghĩa Rust (chèn tại mọi biên scalar, kể cả đầu/cuối).
    pub(crate) fn replace_n(s: &str, from: &str, to: &str, n: usize) -> String {
        s.replacen(from, to, n)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn toi_da_n_lan_trai_sang_phai() {
            assert_eq!(replace_n("aaaa", "a", "b", 0), "aaaa");
            assert_eq!(replace_n("aaaa", "a", "b", 1), "baaa");
            assert_eq!(replace_n("aaaa", "a", "b", 2), "bbaa");
            assert_eq!(replace_n("aaaa", "a", "b", 3), "bbba");
            assert_eq!(replace_n("aaaa", "a", "b", 4), "bbbb");
            assert_eq!(replace_n("banana", "na", "X", 1), "baXna");
            assert_eq!(replace_n("banana", "na", "X", 2), "baXX");
            assert_eq!(replace_n("ababab", "ab", "X", 2), "XXab");
            assert_eq!(replace_n("x.y.z", ".", "/", 1), "x/y.z");
        }

        #[test]
        fn n_lon_hon_so_lan_khop_thay_tat_ca() {
            assert_eq!(replace_n("aaaa", "a", "b", 99), "bbbb");
            assert_eq!(replace_n("banana", "na", "X", 99), "baXX");
            assert_eq!(replace_n("ababab", "ab", "X", 99), "XXX");
            assert_eq!(replace_n("x.y.z", ".", "/", 99), "x/y/z");
            assert_eq!(replace_n("abc", "z", "X", 99), "abc");
        }

        #[test]
        fn khong_khop_tra_nguyen_ven() {
            assert_eq!(replace_n("abc", "z", "X", 2), "abc");
            assert_eq!(replace_n("abc", "abcd", "X", 5), "abc");
            assert_eq!(replace_n("", "a", "b", 3), "");
        }

        #[test]
        fn from_rong_theo_nghia_rust() {
            assert_eq!(replace_n("abc", "", "-", 0), "abc");
            assert_eq!(replace_n("abc", "", "-", 1), "-abc");
            assert_eq!(replace_n("abc", "", "-", 2), "-a-bc");
            assert_eq!(replace_n("abc", "", "-", 3), "-a-b-c");
            assert_eq!(replace_n("abc", "", "-", 4), "-a-b-c-");
            assert_eq!(replace_n("abc", "", "-", 9), "-a-b-c-");
            assert_eq!(replace_n("", "", "-", 1), "-");
            assert_eq!(replace_n("", "", "-", 9), "-");
            assert_eq!(replace_n("abc", "", "", 2), "abc");
            assert_eq!(replace_n("đ", "", "😀", 2), "😀đ😀");
        }

        #[test]
        fn s_rong_va_to_rong() {
            assert_eq!(replace_n("abc", "b", "", 1), "ac");
            assert_eq!(replace_n("abc", "b", "", 2), "ac");
            assert_eq!(replace_n("banana", "na", "", 2), "ba");
            assert_eq!(replace_n("a", "a", "", 1), "");
            assert_eq!(replace_n("đ😀", "😀", "", 5), "đ");
        }

        #[test]
        fn chong_lan_khong_chong_va_quet_tiep_sau_khop() {
            assert_eq!(replace_n("aaaa", "aa", "b", 1), "baa");
            assert_eq!(replace_n("aaaa", "aa", "b", 2), "bb");
            assert_eq!(replace_n("aaaa", "aa", "b", 3), "bb");
            assert_eq!(replace_n("aaa", "aa", "b", 2), "ba");
            assert_eq!(replace_n("ababa", "aba", "X", 2), "Xba");
            assert_eq!(replace_n("aaa", "a", "aa", 2), "aaaaa");
            assert_eq!(replace_n("aaa", "a", "aa", 9), "aaaaaa");
        }

        #[test]
        fn unicode_theo_scalar_khong_phai_byte() {
            assert_eq!(replace_n("đ😀x😀", "😀", "Z", 1), "đZx😀");
            assert_eq!(replace_n("đ😀x😀", "😀", "Z", 2), "đZxZ");
            assert_eq!(replace_n("đađ", "đ", "Đ", 2), "ĐaĐ");
            assert_eq!(replace_n("😀😀😀", "😀", "🙂", 2), "🙂🙂😀");
            assert_eq!(replace_n("đ😀x", "đ😀", "Q", 1), "Qx");
            assert_eq!(replace_n("đ😀x", "x", "đ", 1), "đ😀đ");
            assert_eq!(
                replace_n("👨‍👩‍👧x", "👩", "M", 1),
                "👨‍M‍👧x",
                "chỉ thay scalar giữa, không cắt ZWJ"
            );
            assert_eq!(replace_n("đ😀xđ😀y", "đ😀", "Q", 2), "QxQy");
        }
    }
}

pub(crate) mod strrsplit {
    //! **VD-270 (rsplit)** — `std.str.rsplit(s, sep) -> list[str]`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Tách **phải → trái**, trả các phần theo **thứ tự ngược** (phần **cuối** trước), **đúng**
    //!   nghĩa Rust `str::rsplit`: `rsplit("a,b,c", ",") == ["c", "b", "a"]`.
    //! - Không khớp/`sep` dài hơn `s` ⇒ `[s]`; `s` rỗng + `sep` khác rỗng ⇒ `[""]` (`len` = 1).
    //! - Dấu phân cách ở **cuối** sinh phần tử rỗng **đầu**: `rsplit("a,b,", ",") == ["", "b", "a"]`;
    //!   ở **đầu** sinh phần tử rỗng **cuối**: `rsplit(",a", ",") == ["a", ""]`; dấu **liên tiếp**
    //!   là phần tử rỗng riêng: `rsplit("a,,b", ",") == ["b", "", "a"]`.
    //! - Chồng lấn **không** khớp lại (khớp không chồng, từ phải): `rsplit("aaa", "aa") == ["", "a"]`,
    //!   `rsplit("aaaa", "aa") == ["", "", ""]`, `rsplit("abab", "aba") == ["b", ""]`,
    //!   `rsplit("banana", "ana") == ["", "ban"]`.
    //! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
    //!   **không bao giờ** cắt giữa scalar: `rsplit("đ😀x😀", "😀") == ["", "x", "đ"]`; ZWJ trong
    //!   emoji không phải biên đặc biệt: `rsplit("👨‍👩‍👧x", "👩") == ["‍👧x", "👨‍"]`.
    //! - `sep` rỗng theo **đúng nghĩa Rust `str::rsplit`** (khác `str.split` của VS: rỗng ⇒ tách
    //!   theo scalar và **không** sinh phần rỗng đầu/cuối). Bảng đã kiểm bằng probe Rust:
    //!
    //!   | biểu thức | kết quả |
    //!   |---|---|
    //!   | `"abc".rsplit("")` | `["", "c", "b", "a", ""]` |
    //!   | `"".rsplit("")` | `["", ""]` |
    //!   | `"a".rsplit("")` | `["", "a", ""]` |
    //!   | `"đ😀".rsplit("")` | `["", "😀", "đ", ""]` |
    //!   | `"".rsplit(",")` | `[""]` |
    //!   | `"abc".rsplit(",")` | `["abc"]` |
    //!
    //! - Không trap; chỉ cấp phát **các phần** kết quả (mỗi phần một `String`), không cấp phát theo
    //!   độ dài `s`; số phần bị chặn bởi `len(s) + 1` khi `sep` rỗng.

    use vis_syntax::Span;

    use crate::interp::R;
    use crate::value::Value;

    /// Tách `s` theo `sep` từ **phải → trái**; dùng **đúng** `str::rsplit` của Rust nên `sep` rỗng
    /// cũng theo nghĩa Rust (khớp tại mọi biên scalar, kể cả đầu/cuối).
    pub(crate) fn phan(s: &str, sep: &str) -> Vec<String> {
        s.rsplit(sep).map(str::to_string).collect()
    }

    /// Dispatch `str.rsplit`; không trap, không kiểm `sep` rỗng.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
        let s = super::super::chuoi(name, &args[0], 0, span)?;
        let sep = super::super::chuoi(name, &args[1], 1, span)?;
        Ok(Value::List(
            phan(s, sep).into_iter().map(Value::Str).collect(),
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn ds(xs: &[&str]) -> Vec<String> {
            xs.iter().map(|x| x.to_string()).collect()
        }

        #[test]
        fn bang_probe_rust_thu_tu_nguoc() {
            assert_eq!(phan("a,b,c", ","), ds(&["c", "b", "a"]));
            assert_eq!(phan("a,b,", ","), ds(&["", "b", "a"]));
            assert_eq!(phan(",a", ","), ds(&["a", ""]));
            assert_eq!(phan(",", ","), ds(&["", ""]));
            assert_eq!(phan(",,", ","), ds(&["", "", ""]));
            assert_eq!(phan("a,,b", ","), ds(&["b", "", "a"]));
            assert_eq!(phan("a::b::c", "::"), ds(&["c", "b", "a"]));
            assert_eq!(phan("aXbXc", "X"), ds(&["c", "b", "a"]));
        }

        #[test]
        fn khong_khop_va_s_rong() {
            assert_eq!(phan("abc", ","), ds(&["abc"]));
            assert_eq!(phan("abc", "abcd"), ds(&["abc"]));
            assert_eq!(phan("", ","), ds(&[""]));
            assert_eq!(phan("", "::"), ds(&[""]));
        }

        #[test]
        fn sep_rong_theo_nghia_rust() {
            assert_eq!(phan("abc", ""), ds(&["", "c", "b", "a", ""]));
            assert_eq!(phan("", ""), ds(&["", ""]));
            assert_eq!(phan("a", ""), ds(&["", "a", ""]));
            assert_eq!(phan("đ😀", ""), ds(&["", "😀", "đ", ""]));
        }

        #[test]
        fn chong_lan_khong_khop_lai() {
            assert_eq!(phan("aaa", "aa"), ds(&["", "a"]));
            assert_eq!(phan("aaaa", "aa"), ds(&["", "", ""]));
            assert_eq!(phan("abab", "aba"), ds(&["b", ""]));
            assert_eq!(phan("banana", "ana"), ds(&["", "ban"]));
        }

        #[test]
        fn unicode_emoji_theo_chuoi_con() {
            assert_eq!(phan("đ😀x😀", "😀"), ds(&["", "x", "đ"]));
            assert_eq!(phan("a😀b😀c", "😀"), ds(&["c", "b", "a"]));
            assert_eq!(phan("đ,😀,x", ","), ds(&["x", "😀", "đ"]));
            assert_eq!(
                phan("👨‍👩‍👧x", "👩"),
                ds(&["‍👧x", "👨‍"]),
                "ZWJ là scalar thường; cắt quanh 👩 vẫn hợp lệ UTF-8"
            );
            assert_eq!(phan("👨‍👩‍👧", "👨‍👩‍👧"), ds(&["", ""]));
        }

        #[test]
        fn goi_tra_list_that() {
            use crate::value::eq;
            let args = |s: &str, sep: &str| vec![Value::Str(s.into()), Value::Str(sep.into())];
            let span = Span::new(0, 0);
            let ky_vong = Value::List(vec![
                Value::Str("c".into()),
                Value::Str("b".into()),
                Value::Str("a".into()),
            ]);
            assert!(eq(
                &goi("str.rsplit", &args("a,b,c", ","), span).unwrap(),
                &ky_vong
            ));
            let rong = Value::List(vec![Value::Str("".into())]);
            assert!(eq(&goi("str.rsplit", &args("", ","), span).unwrap(), &rong));
        }
    }
}

pub(crate) mod strrsplitonce {
    //! **VD-270 (rsplit-once)** — `std.str.rsplit_once(s, sep) -> list[str]?`.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM):
    //! - Tách tại **lần xuất hiện cuối cùng** của `sep`, **không** tách các lần trước:
    //!   `rsplit_once("a,b,c", ",") == Some(["a,b", "c"])`, `rsplit_once("aXbXc", "X") ==
    //!   Some(["aXb", "c"])`.
    //! - Không có `sep` trong `s` (hoặc `sep` dài hơn `s`) ⇒ `None`.
    //! - `sep` rỗng ⇒ `Some([s, ""])` — **biên cuối** của `s` (khác `split_once`: `Some(["", s])`);
    //!   `s` rỗng + `sep` rỗng ⇒ `Some(["", ""])`; `s` rỗng + `sep` khác rỗng ⇒ `None`. Đúng nghĩa
    //!   Rust `str::rsplit_once` (probe xác nhận; **không** trap).
    //! - Dấu phân cách ở **đầu** ⇒ phần đầu rỗng: `rsplit_once("abc", "a") == Some(["", "bc"])`;
    //!   ở **cuối** ⇒ phần sau rỗng: `rsplit_once("abc", "c") == Some(["ab", ""])`.
    //! - Đơn vị khớp là **chuỗi con UTF-8** trên **biên scalar** (cùng `str.split`/`str.find`),
    //!   **không bao giờ** cắt giữa scalar: `rsplit_once("đ😀x", "😀") == Some(["đ", "x"])`;
    //!   ZWJ trong emoji không phải biên đặc biệt: `rsplit_once("👨‍👩‍👧x", "👩")` cắt đúng
    //!   quanh `👩` **cuối** và phần còn lại vẫn là UTF-8 hợp lệ.
    //! - `Some` luôn có **đúng 2** phần tử; chỉ cấp phát **hai** chuỗi kết quả (không cắt thêm,
    //!   không cấp phát theo độ dài `s`).

    use vis_syntax::Span;

    use crate::value::Value;

    /// Tách `s` tại lần khớp **cuối** của `sep`; `None` nếu không khớp. Dùng **đúng**
    /// `str::rsplit_once` của Rust nên `sep` rỗng cũng theo nghĩa Rust.
    pub(crate) fn tach<'a>(s: &'a str, sep: &str) -> Option<(&'a str, &'a str)> {
        s.rsplit_once(sep)
    }

    /// Dispatch `str.rsplit_once`: `Some(list)` với **đúng 2** phần, hoặc `None`; không trap.
    pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> crate::interp::R<Value> {
        let s = super::super::chuoi(name, &args[0], 0, span)?;
        let sep = super::super::chuoi(name, &args[1], 1, span)?;
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
        fn chi_tach_lan_cuoi() {
            assert_eq!(hai("a,b,c", ","), Some(("a,b".into(), "c".into())));
            assert_eq!(hai("a::b::c", "::"), Some(("a::b".into(), "c".into())));
            assert_eq!(hai("aXbXc", "X"), Some(("aXb".into(), "c".into())));
            assert_eq!(hai("aaaa", "aa"), Some(("aa".into(), "".into())));
            assert_eq!(hai("aaa", "aa"), Some(("a".into(), "".into())));
            assert_eq!(hai("banana", "na"), Some(("bana".into(), "".into())));
        }

        #[test]
        fn dau_va_cuoi_sinh_phan_rong() {
            assert_eq!(hai("abc", "a"), Some(("".into(), "bc".into())));
            assert_eq!(hai("abc", "c"), Some(("ab".into(), "".into())));
            assert_eq!(hai("đ", "đ"), Some(("".into(), "".into())));
            assert_eq!(hai(",", ","), Some(("".into(), "".into())));
            assert_eq!(hai(",a,", ","), Some((",a".into(), "".into())));
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
            assert_eq!(hai("abc", ""), Some(("abc".into(), "".into())));
            assert_eq!(hai("", ""), Some(("".into(), "".into())));
            assert_eq!(hai("đ😀", ""), Some(("đ😀".into(), "".into())));
        }

        #[test]
        fn unicode_emoji_theo_chuoi_con() {
            assert_eq!(hai("đ😀x", "😀"), Some(("đ".into(), "x".into())));
            assert_eq!(hai("ađb", "đ"), Some(("a".into(), "b".into())));
            assert_eq!(hai("đ,😀,x", ","), Some(("đ,😀".into(), "x".into())));
            assert_eq!(hai("a😀b😀c", "😀"), Some(("a😀b".into(), "c".into())));
            assert_eq!(
                hai("👨‍👩‍👧x", "👩"),
                Some(("👨‍".into(), "‍👧x".into())),
                "ZWJ là scalar thường; cắt quanh 👩 vẫn hợp lệ UTF-8"
            );
            assert_eq!(hai("👨‍👩‍👧", "👨‍👩‍👧"), Some(("".into(), "".into())));
        }

        #[test]
        fn goi_tra_option_that_hai_phan() {
            use crate::value::eq;
            let some = |a: &str, b: &str| {
                Value::opt(Some(Value::List(vec![
                    Value::Str(a.into()),
                    Value::Str(b.into()),
                ])))
            };
            let args = |s: &str, sep: &str| vec![Value::Str(s.into()), Value::Str(sep.into())];
            let span = Span::new(0, 0);
            assert!(eq(
                &goi("str.rsplit_once", &args("a,b,c", ","), span).unwrap(),
                &some("a,b", "c")
            ));
            assert!(eq(
                &goi("str.rsplit_once", &args("abc", ""), span).unwrap(),
                &some("abc", "")
            ));
            assert!(eq(
                &goi("str.rsplit_once", &args("abc", "x"), span).unwrap(),
                &Value::none()
            ));
        }
    }
}
