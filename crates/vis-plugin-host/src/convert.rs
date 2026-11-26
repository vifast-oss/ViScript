// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-306/ST-310c (VD-285)** — chuyển đổi tường minh `Value` ↔ `PluginValue` ở **biên host**.
//!
//! Tách khỏi `vis-plugin-abi` để guest ABI **0 phụ thuộc**: plugin chỉ thấy `PluginValue`;
//! host (CLI, registry tĩnh) là nơi duy nhất biết `vis_core::value::Value`.
//!
//! Quy tắc giữ nguyên ST-306/ST-309:
//! - hàm/task/kênh/closure **không** qua biên ⇒ `VS0501` (không đoán kiểu);
//! - `Option`/`Result` chỉ nhận đúng enum chuẩn `Option::{Some,None}` / `Result::{Ok,Err}`;
//! - `Result::Err` phía ViScript chỉ mang thông điệp (mã lỗi không qua biên ngược).

use vis_core::value::{show, Name, Value};
use vis_plugin_abi::{Error, PluginValue, Result};

/// Chuyển `Value` của ViScript sang `PluginValue`; kiểu không qua biên ⇒ `VS0501`.
pub fn tu_value(v: Value) -> Result<PluginValue> {
    Ok(match v {
        Value::Unit => PluginValue::Unit,
        Value::Bool(b) => PluginValue::Bool(b),
        Value::Int(i) | Value::Size(i) | Value::Duration(i) => PluginValue::Int(i),
        Value::Float(f) | Value::Percent(f) => PluginValue::Float(f),
        Value::Str(s) => PluginValue::Str(s),
        Value::Char(c) => PluginValue::Str(c.to_string()),
        Value::Path(s) => PluginValue::Str(s),
        Value::Bytes(b) => PluginValue::Bytes(b),
        Value::List(xs) => PluginValue::List(xs.into_iter().map(tu_value).collect::<Result<_>>()?),
        Value::Map(kv) => {
            let entries = match std::rc::Rc::try_unwrap(kv) {
                Ok(m) => m.into_entries(),
                Err(rc) => rc.iter().cloned().collect(),
            };
            PluginValue::Map(
                entries
                    .into_iter()
                    .map(|(k, v)| Ok((tu_value(k)?, tu_value(v)?)))
                    .collect::<Result<_>>()?,
            )
        }
        Value::Record(_, fs) => PluginValue::Map(
            fs.into_iter()
                .map(|(k, v)| Ok((PluginValue::Str(k), tu_value(v)?)))
                .collect::<Result<_>>()?,
        ),
        Value::Enum(ten, bien, args) => match (ten.as_ref(), bien.as_ref()) {
            ("Option", "Some") if args.len() == 1 => PluginValue::Option(Some(Box::new(tu_value(
                args.into_iter().next().expect("len == 1"),
            )?))),
            ("Option", "None") if args.is_empty() => PluginValue::Option(None),
            ("Result", "Ok") if args.len() == 1 => PluginValue::Result(Ok(Box::new(tu_value(
                args.into_iter().next().expect("len == 1"),
            )?))),
            ("Result", "Err") if args.len() == 1 => {
                PluginValue::Result(Err(Box::new(Error::io(show(&args[0])))))
            }
            _ => {
                return Err(Error::type_mismatch(
                    "giá trị thuần (không enum tuỳ ý)",
                    &format!("{ten}::{bien}"),
                ))
            }
        },
        Value::Cap { kind, root, write } => PluginValue::Cap { kind, root, write },
        other => return Err(Error::type_mismatch("giá trị thuần", &show(&other))),
    })
}

/// Chuyển `PluginValue` về `Value`; dữ liệu plugin hỏng ⇒ `VS0501`.
pub fn sang_value(v: PluginValue) -> Result<Value> {
    Ok(match v {
        PluginValue::Unit => Value::Unit,
        PluginValue::Bool(b) => Value::Bool(b),
        PluginValue::Int(i) => Value::Int(i),
        PluginValue::Float(f) => Value::Float(f),
        PluginValue::Str(s) => Value::Str(s),
        PluginValue::Bytes(b) => Value::Bytes(b),
        PluginValue::List(xs) => {
            Value::List(xs.into_iter().map(sang_value).collect::<Result<_>>()?)
        }
        PluginValue::Map(kv) => Value::map(
            kv.into_iter()
                .map(|(k, v)| Ok((sang_value(k)?, sang_value(v)?)))
                .collect::<Result<_>>()?,
        ),
        PluginValue::Option(None) => {
            Value::Enum(Name::from("Option"), Name::from("None"), Vec::new())
        }
        PluginValue::Option(Some(x)) => Value::Enum(
            Name::from("Option"),
            Name::from("Some"),
            vec![sang_value(*x)?],
        ),
        PluginValue::Result(Ok(x)) => Value::Enum(
            Name::from("Result"),
            Name::from("Ok"),
            vec![sang_value(*x)?],
        ),
        PluginValue::Result(Err(e)) => Value::Enum(
            Name::from("Result"),
            Name::from("Err"),
            vec![Value::Str(e.message)],
        ),
        PluginValue::Cap { kind, root, write } => Value::Cap { kind, root, write },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tron_vong_qua_bien_giu_gia_tri() {
        let v = Value::List(vec![Value::Int(1), Value::Str("x".into())]);
        let p = tu_value(v.clone()).expect("sang PluginValue");
        let lai = sang_value(p).expect("về Value");
        assert_eq!(show(&lai), show(&v));
    }

    #[test]
    fn khong_nuot_kieu_la() {
        let e = tu_value(Value::Builtin("in")).expect_err("builtin không qua biên");
        assert_eq!(e.code, "VS0501");
    }

    #[test]
    fn option_va_result_dung_enum_chuan() {
        let some = Value::Enum(
            Name::from("Option"),
            Name::from("Some"),
            vec![Value::Int(2)],
        );
        let ve = sang_value(tu_value(some).expect("option")).expect("về Value");
        assert_eq!(
            show(&ve),
            show(&Value::Enum(
                Name::from("Option"),
                Name::from("Some"),
                vec![Value::Int(2)]
            ))
        );
        let la = Value::Enum(Name::from("Khac"), Name::from("Some"), vec![Value::Int(2)]);
        assert!(tu_value(la).is_err());
    }
}
