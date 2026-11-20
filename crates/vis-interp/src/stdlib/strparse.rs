// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (parse-int)** — `std.str.parse_int`: chuỗi thập phân → `int?`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - Cú pháp **chặt** như `i64::from_str`: dấu `+`/`-` tuỳ chọn rồi **một hoặc nhiều** chữ số
//!   ASCII `0..=9`; **không** khoảng trắng, `_`, `0x`, dấu thập phân, dấu `+`/`-` lặp.
//! - Tràn `i64` (kể cả `i64::MIN`/`MAX` biên) ⇒ `None`; chuỗi rỗng ⇒ `None`.
//! - Không trap, không cấp phát kết quả; sugar R5c `s.parse_int()`.

use crate::value::Value;

/// Parse **chặt** chuỗi thập phân thành `i64`; `None` nếu sai cú pháp/tràn.
pub(super) fn parse_int(s: &str) -> Option<i64> {
    s.parse::<i64>().ok()
}

/// Dispatch chung: `Some(int)` / `None`.
pub(super) fn goi(s: &str) -> Value {
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
