// SPDX-License-Identifier: MIT OR Apache-2.0
//! **R20a (VD-178)**: `assert cond, "thông điệp"` — tách khỏi `builtin.rs` để giữ luật R1.

use crate::builtin::trap;
use crate::interp::R;
use crate::interp::RT;
use crate::value::Value;
use vis_syntax::Span;

/// `assert cond[, "thông điệp"]` — 1 hoặc 2 đối số; sai ⇒ trap `VS0501`.
pub fn assert_builtin(args: &[Value], span: Span) -> R<Value> {
    if args.is_empty() || args.len() > 2 {
        return Err(trap(RT, "`assert` cần 1 hoặc 2 đối số", span));
    }
    let msg = match args.get(1) {
        Some(Value::Str(m)) => format!("assert thất bại: {m}"),
        Some(v) => {
            return Err(trap(
                RT,
                format!("`assert` thông điệp phải là `str`, được `{}`", v.show()),
                span,
            ))
        }
        None => "assert thất bại".to_string(),
    };
    match &args[0] {
        Value::Bool(true) => Ok(Value::Unit),
        Value::Bool(false) => Err(trap(RT, &msg, span)),
        v => Err(trap(
            RT,
            format!("`assert` cần `bool`, được `{}`", v.show()),
            span,
        )),
    }
}
