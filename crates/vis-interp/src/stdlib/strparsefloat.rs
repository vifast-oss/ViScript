// SPDX-License-Identifier: MIT OR Apache-2.0
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
pub(super) fn parse_float(s: &str) -> Option<f64> {
    let f = s.parse::<f64>().ok()?;
    if f.is_finite() {
        Some(f)
    } else {
        None
    }
}

/// Dispatch chung: `Some(float)` / `None`.
pub(super) fn goi(s: &str) -> Value {
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
