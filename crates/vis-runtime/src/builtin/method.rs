// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — bộ phương thức lõi (`list`/`str`/`map`/`chan`), tách khỏi
//! `builtin.rs` để giữ tệp dưới 1.150 dòng. Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use vis_hir::hir::BinOp;
use vis_syntax::Span;

use crate::codes::{OVERFLOW, RT};
use vis_core::trap::Abort;
use vis_core::value::{Value, VisMap};

use super::{bin, call, trap, Host};

pub fn method(
    host: &mut dyn Host,
    recv: Value,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    // **R5c (VD-178)**: `x.f(a) ≡ std.<mod>.f(x, a)` — chỉ khi `f` **không** phải phương thức lõi.
    let (m, loi) = match &recv {
        Value::List(_) => (
            "list",
            "len get first last push map filter each sum join is_empty",
        ),
        Value::Str(_) => ("str", "len upper lower split chars contains is_empty"),
        Value::Map(_) => ("map", "len get has keys values is_empty"),
        _ => ("", ""),
    };
    if !m.is_empty() && !loi.split(' ').any(|x| x == name) {
        // **VD-270 (path.normalize)**: `s.normalize()` là sugar của `path.normalize`.
        let m = vis_hir::stdlib::module_sugar(m, name).unwrap_or(m);
        let ngan = format!("{m}.{name}");
        if vis_hir::stdlib::sig_of(&ngan).is_some() {
            let mut all = vec![recv.clone()];
            all.extend(args.clone());
            return call(host, &format!("std.{ngan}"), all, span);
        }
    }
    match recv {
        Value::List(xs) => list_method(host, std::rc::Rc::unwrap_or_clone(xs), name, args, span),
        Value::Str(s) => str_method(host, s, name, args, span),
        Value::Map(kv) => map_method(host, kv, name, args, span),
        // **ST-61 (VD-96)**: `send`/`recv`/`close` — **một** hiện thực cho cả interp lẫn VM.
        Value::Chan(c) => chan_method(host, c, name, args, span),
        other => Err(trap(
            RT,
            format!("`{}` không có phương thức `{name}`", other.show()),
            span,
        )),
    }
}

/// **ST-61 (VD-96) / VS3-2a/2b**: phương thức kênh — chuyển sang `rt::chan` (wait-set `chan(0)`
/// + `recv_timeout`/`send_timeout` chỉ đường frame test; cap ≥ 1 giữ nguyên run-to-completion
/// + `VS0505` khi hết task).
fn chan_method(
    host: &mut dyn Host,
    c: std::rc::Rc<std::cell::RefCell<crate::rt::Chan>>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    crate::rt::chan::method(host, c, name, args, span)
}

fn list_method(
    host: &mut dyn Host,
    xs: Vec<Value>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    match name {
        "len" => Ok(Value::Int(xs.len() as i64)),
        // **R17 (VD-178)**: `is_empty()` cho list/map/str.
        "is_empty" => Ok(Value::Bool(xs.is_empty())),
        "get" => {
            let Some(Value::Int(i)) = args.first() else {
                return Err(trap(RT, "`get` cần chỉ số `int`", span));
            };
            Ok(Value::opt(match xs.get(*i as usize) {
                Some(v) if *i >= 0 => Some(v.clone()),
                _ => None,
            }))
        }
        "first" => Ok(Value::opt(xs.first().cloned())),
        "last" => Ok(Value::opt(xs.last().cloned())),
        "push" => {
            if args.len() != 1 {
                return Err(trap(RT, "`push` cần 1 đối số", span));
            }
            let mut out = xs;
            out.push(args[0].clone());
            Ok(Value::list(out))
        }
        "map" => {
            let f = args.first().cloned().unwrap_or(Value::Unit);
            let mut out = Vec::with_capacity(xs.len());
            for x in xs {
                out.push(host.call(&f, vec![x], span)?);
            }
            Ok(Value::list(out))
        }
        "filter" => {
            let f = args.first().cloned().unwrap_or(Value::Unit);
            let mut out = Vec::new();
            for x in xs {
                let keep = host.call(&f, vec![x.clone()], span)?;
                if keep.truthy(span)? {
                    out.push(x);
                }
            }
            Ok(Value::list(out))
        }
        "each" => {
            let f = args.first().cloned().unwrap_or(Value::Unit);
            for x in xs {
                host.call(&f, vec![x], span)?;
            }
            Ok(Value::Unit)
        }
        "sum" => {
            let mut acc: Option<Value> = None;
            for x in xs {
                acc = Some(match acc {
                    None => x,
                    Some(a) => bin(BinOp::Add, a, x, span)?,
                });
            }
            Ok(acc.unwrap_or(Value::Int(0)))
        }
        "join" => {
            let sep = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(trap(RT, "`join` cần dấu ngăn `str`", span)),
            };
            let parts: Vec<String> = xs.iter().map(Value::show).collect();
            Ok(Value::Str(parts.join(&sep)))
        }
        // **R20b (VD-178)**: `enumerate`/`any`/`all`/`find` — tách sang `builtin/list.rs`.
        "enumerate" | "any" | "all" | "find" => {
            super::list::duyet_nang_cao(host, xs, name, args, span)
        }
        other => Err(trap(
            RT,
            format!("`list.{other}()` chưa hỗ trợ (danh sách: len get first last push map filter each sum join enumerate any all find)"),
            span,
        )),
    }
}

fn str_method(
    _host: &mut dyn Host,
    s: String,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    match name {
        "len" => Ok(Value::Int(s.chars().count() as i64)),
        "is_empty" => Ok(Value::Bool(s.is_empty())),
        "upper" => Ok(Value::Str(s.to_uppercase())),
        "lower" => Ok(Value::Str(s.to_lowercase())),
        "chars" => Ok(Value::list(
            s.chars().map(|c| Value::Str(c.to_string())).collect(),
        )),
        "contains" => {
            let Some(Value::Str(sub)) = args.first() else {
                return Err(trap(RT, "`contains` cần `str`", span));
            };
            Ok(Value::Bool(s.contains(sub.as_str())))
        }
        "split" => {
            let Some(Value::Str(sep)) = args.first() else {
                return Err(trap(RT, "`split` cần dấu ngăn `str`", span));
            };
            if sep.is_empty() {
                return Err(trap(RT, "`split` cần dấu ngăn khác rỗng", span));
            }
            Ok(Value::list(
                s.split(sep.as_str())
                    .map(|p| Value::Str(p.to_string()))
                    .collect(),
            ))
        }
        other => Err(trap(
            RT,
            format!(
                "`str.{other}()` chưa hỗ trợ (danh sách: len upper lower split chars contains)"
            ),
            span,
        )),
    }
}

fn map_method(
    _host: &mut dyn Host,
    kv: std::rc::Rc<VisMap>,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_core::trap::R<Value> {
    match name {
        "len" => Ok(Value::Int(kv.len() as i64)),
        "is_empty" => Ok(Value::Bool(kv.is_empty())),
        "get" => {
            let Some(k) = args.first() else {
                return Err(trap(RT, "`get` cần 1 khoá", span));
            };
            // **ST-315b**: tra chỉ mục băm O(1) khi map đủ lớn (không chép map, không quét N).
            Ok(Value::opt(kv.get(k).cloned()))
        }
        "has" => {
            let Some(k) = args.first() else {
                return Err(trap(RT, "`has` cần 1 khoá", span));
            };
            Ok(Value::Bool(kv.contains_key(k)))
        }
        "keys" => Ok(Value::list(kv.keys().cloned().collect())),
        "values" => Ok(Value::list(kv.values().cloned().collect())),
        other => Err(trap(
            RT,
            format!("`map.{other}()` chưa hỗ trợ (danh sách: len get has keys values)"),
            span,
        )),
    }
}
/// **ST-240 / VD-243 #172**: nhân `f64` rồi kiểm **kết quả cuối**, không `checked_mul` máy móc;
/// `100pct * i64::MAX = 2^63` vẫn là biên hợp lệ. Mọi tạo `Percent` mới cũng kiểm miền.
pub(crate) fn pct_nhan_f64(p: f64, n: f64, span: Span) -> vis_core::trap::R<i64> {
    let v = p * n;
    if !v.is_finite() || v > vis_core::value::PCT_GIOI_I64 || v < -vis_core::value::PCT_GIOI_I64 {
        return Err(trap(
            OVERFLOW,
            "tràn miền biểu diễn khi nhân percent với số (VD-243 #172)".to_string(),
            span,
        ));
    }
    Ok(v as i64)
}

/// **ST-240 / VD-243 #172**: chặn tại nơi tạo giá trị — không chỉ sửa chuỗi `show`.
pub(crate) fn pct_moi(ratio: f64, span: Span) -> vis_core::trap::R<f64> {
    if !vis_core::value::pct_hop_le(ratio) {
        return Err(trap(
            OVERFLOW,
            "kết quả percent vượt miền biểu diễn (VD-243 #172)",
            span,
        ));
    }
    Ok(ratio)
}

/// Tràn `i64` — `VS0507` (VD-43 #2/R9).
pub(crate) fn overflow(span: Span) -> Abort {
    // **R9 (VD-178)**: tràn i64 ⇒ `VS0507` (tách khỏi `VS0501`).
    trap(OVERFLOW, "tràn i64 lúc chạy (VD-43 #2)", span)
}
