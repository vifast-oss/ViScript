// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-217 / VD-211** (A0 2026-11-18) — `Enum` ở tầng chạy so **cả tên enum**:
//! hai enum khác tên nhưng cùng biến thể + cùng đối số **không** bằng nhau.
//! (`==` khác kiểu đã bị tầng kiểu chặn bằng VS0412 ⇒ không mở nợ pha 2 theo VD-211.)

use vis_interp::value::Value;

fn e(en: &str, va: &str, args: Vec<Value>) -> Value {
    Value::Enum(en.to_string(), va.to_string(), args)
}

#[test]
fn enum_khac_ten_khong_bang() {
    let a = e("A", "A", vec![Value::Int(5)]);
    let b = e("B", "A", vec![Value::Int(5)]);
    assert!(!vis_interp::value::eq(&a, &b));
    let a2 = e("A", "A", vec![Value::Int(5)]);
    assert!(vis_interp::value::eq(&a, &a2));
    // Option và Result cùng biến thể tên gần như — không lẫn.
    assert!(!vis_interp::value::eq(
        &Value::some(Value::Int(5)),
        &Value::ok(Value::Int(5))
    ));
    assert!(vis_interp::value::eq(
        &Value::some(Value::some(Value::Int(5))),
        &Value::some(Value::some(Value::Int(5)))
    ));
    // Enum cùng tên, khác biến thể ⇒ khác nhau
    assert!(!vis_interp::value::eq(
        &Value::some(Value::Int(5)),
        &Value::none()
    ));
}
