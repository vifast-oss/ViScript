// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (map-contains-key)** — `std.map.contains_key(m, key) -> bool`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//! - `true` khi `m` có **ít nhất một** cặp khoá `==` `key`; `false` khi không có hoặc `m` rỗng
//!   (không trap).
//! - So bằng [`crate::value::eq`] — **đúng** phép so của `map.has`/`map.get`
//!   (`Value::eq_value`: cấu trúc, list/map/record/enum lồng nhau; `int` ↔ `float` theo tỉ lệ
//!   như `==`), nên `contains_key(m, k)` khớp `m.has(k)`.
//! - Quét tuyến tính, dừng ở khoá đầu tiên bằng; không cấp phát.
//!
//! `key` **không** bị ép kiểu ở tầng chạy: checker đã buộc `K` từ `m`; lời gọi thủ công vẫn
//! so **cấu trúc** (khác kiểu ⇒ thường `false`, không trap). `m` không phải map ⇒ lỗi dữ liệu
//! `VS0501` qua [`super::chung::ban_do`].

use vis_syntax::Span;

use super::chung::ban_do;
use crate::interp::R;
use crate::value::{eq, Value};

/// `true` iff có khoá `==` `key`; map rỗng ⇒ `false`.
pub(super) fn co(m: &[(Value, Value)], key: &Value) -> bool {
    m.iter().any(|(k, _)| eq(k, key))
}

/// Điểm vào `std.map.contains_key` — kiểm `m` là map rồi so khoá.
pub(super) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    let m = ban_do(name, &args[0], 0, span)?;
    Ok(Value::Bool(co(m, &args[1])))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bd(kv: &[(&str, i64)]) -> Vec<(Value, Value)> {
        kv.iter()
            .map(|(k, v)| (Value::Str((*k).into()), Value::Int(*v)))
            .collect()
    }

    #[test]
    fn co_va_khong_co() {
        let m = bd(&[("a", 1), ("b", 2)]);
        assert!(co(&m, &Value::Str("a".into())));
        assert!(co(&m, &Value::Str("b".into())));
        assert!(!co(&m, &Value::Str("c".into())));
    }

    #[test]
    fn map_rong_la_false() {
        assert!(!co(&[], &Value::Str("a".into())));
        assert!(!co(&[], &Value::Int(1)));
    }

    #[test]
    fn cau_truc_long_nhau_va_khac_kieu() {
        // List làm khoá: cùng cấu trúc, khác thứ tự ⇒ false.
        let m = vec![(
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(1),
        )];
        assert!(co(&m, &Value::List(vec![Value::Int(1), Value::Int(2)])));
        assert!(!co(&m, &Value::List(vec![Value::Int(2), Value::Int(1)])));

        // Bản ghi: so theo trường, không theo thứ tự khai báo.
        let m = vec![(
            Value::Record(
                "P".into(),
                vec![("x".into(), Value::Int(1)), ("y".into(), Value::Int(2))],
            ),
            Value::Int(1),
        )];
        let k = Value::Record(
            "P".into(),
            vec![("y".into(), Value::Int(2)), ("x".into(), Value::Int(1))],
        );
        assert!(co(&m, &k));

        // Khác kiểu (`int` vs `str`) chỉ là `false`, không trap.
        let m = vec![(Value::Int(1), Value::Int(1))];
        assert!(!co(&m, &Value::Str("1".into())));
    }
}
