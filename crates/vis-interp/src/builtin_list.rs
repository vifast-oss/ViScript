// SPDX-License-Identifier: MIT OR Apache-2.0
//! **R20b (VD-178)**: `enumerate`/`any`/`all`/`find` — tách khỏi `builtin.rs` để giữ luật R1.

use crate::builtin::Host;
use crate::interp::R;
use crate::value::Value;
use vis_syntax::Span;

/// `xs.enumerate()` / `xs.any(f)` / `xs.all(f)` / `xs.find(f)`.
pub fn duyet_nang_cao(
    host: &mut dyn Host,
    xs: Vec<Value>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> R<Value> {
    match name {
        "enumerate" => Ok(Value::List(
            xs.iter()
                .enumerate()
                .map(|(i, x)| Value::List(vec![Value::Int(i as i64), x.clone()]))
                .collect(),
        )),
        "any" => {
            let f = args.first().cloned().unwrap_or(Value::Unit);
            for x in xs {
                if host.call(&f, vec![x], span)?.truthy(span)? {
                    return Ok(Value::Bool(true));
                }
            }
            Ok(Value::Bool(false))
        }
        "all" => {
            let f = args.first().cloned().unwrap_or(Value::Unit);
            for x in xs {
                if !host.call(&f, vec![x], span)?.truthy(span)? {
                    return Ok(Value::Bool(false));
                }
            }
            Ok(Value::Bool(true))
        }
        _ => {
            let f = args.first().cloned().unwrap_or(Value::Unit);
            for x in xs {
                if host.call(&f, vec![x.clone()], span)?.truthy(span)? {
                    return Ok(Value::some(x));
                }
            }
            Ok(Value::none())
        }
    }
}
