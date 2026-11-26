// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-269** — gọi phương thức qua **một** lớp `Option`: `recv?.m(…)`.
//!
//! Lowering không dispatch được callee `HExpr::Field { optional: true }`, nên interp tự:
//! 1. đánh giá base **một lần**;
//! 2. `None` ⇒ trả `None` **trước khi đánh giá đối số** (đoản mạch);
//! 3. `Some(v)` ⇒ gọi phương thức lõi (`list`/`str`/`map`/`Chan`) hoặc phương thức `impl`
//!    (bảng [`vis_hir::MethodRet`]) với `v` làm `self`;
//! 4. nâng kết quả **một lớp** ([`crate::lift::lift_opt`]): plain → `Some`, `Option` giữ
//!    nguyên (không bọc hai lớp), `()` giữ `()` (xem [`vis_hir::nang_optional`]).
//!
//! Tách khỏi `goi.rs` để giữ luật R1 (≤ 1.500 dòng/tệp).

use vis_hir::hir::HExpr;
use vis_hir::Type;
use vis_syntax::Span;

use crate::interp::{Abort, Interp, R, RT};
use crate::value::Value;

impl<'a> Interp<'a> {
    pub(crate) fn goi_tuy_chon(
        &mut self,
        base: &HExpr,
        name: &str,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: Span,
    ) -> R<Value> {
        let raw = self.eval(base)?;
        let v = match raw {
            Value::Enum(e) if e.enum_name == "Option" && e.variant == "None" => {
                return Ok(Value::none());
            }
            Value::Enum(e)
                if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 =>
            {
                e.args.get(0).expect("Some luôn có một đối số").clone()
            }
            Value::Enum(e) if e.enum_name == "Option" => {
                return Err(Abort::Trap(crate::interp::Trap::new(
                    RT,
                    "giá trị `Option` không hợp lệ ở `?.` (VD-269)",
                    span,
                )))
            }
            other => {
                return Err(Abort::Trap(crate::interp::Trap::new(
                    RT,
                    format!(
                        "`?.` cần `Option`, gặp `{}` (VD-269 — checker phải chặn trước)",
                        other.show()
                    ),
                    span,
                )))
            }
        };
        // Phương thức lõi: đối số chỉ được đánh giá **sau** khi biết base là `Some`.
        if matches!(
            v,
            Value::List(_) | Value::Str(_) | Value::Map(_) | Value::Chan(_)
        ) {
            let (pos, _nm) = self.eval_args(args, named)?;
            let kq = crate::builtin::method(self, v, name, pos, span)?;
            return Ok(nang_ket_qua(kq));
        }
        let (pos, nm) = self.eval_args(args, named)?;
        let sym = self
            .expr_ty(base, &v)
            .and_then(|t| {
                let inner = match t {
                    Type::Opt(x) => *x,
                    other => other,
                };
                self.mret.sym(&inner, name, self.types)
            })
            .or_else(|| {
                let ten: &str = match &v {
                    Value::Record(r) => r.name.as_str(),
                    Value::Enum(e) => e.enum_name.as_ref(),
                    _ => return None,
                };
                self.mret.sym_theo_ten(ten, name)
            });
        let Some(sym) = sym else {
            return Err(Abort::Trap(crate::interp::Trap::new(
                RT,
                format!("không có phương thức `{name}` cho `{}` (VD-269)", v.show()),
                span,
            )));
        };
        let Some(f) = self.fns.get(&sym).copied() else {
            return Err(Abort::Trap(crate::interp::Trap::new(
                RT,
                format!("phương thức `{name}` không có thân (VD-269)"),
                span,
            )));
        };
        let mut ds = Vec::with_capacity(pos.len() + 1);
        ds.push(v);
        ds.extend(pos);
        let kq = self.call_hfn(f, ds, nm, span)?;
        Ok(nang_ket_qua(kq))
    }
}

/// **VD-269** — lift kết quả phương thức: `()` giữ `()`, còn lại `lift_opt(1)` (plain → `Some`,
/// `Option` có sẵn giữ nguyên, `None` giữ `None`).
fn nang_ket_qua(v: Value) -> Value {
    if matches!(v, Value::Unit) {
        v
    } else {
        crate::lift::lift_opt(v, 1)
    }
}
