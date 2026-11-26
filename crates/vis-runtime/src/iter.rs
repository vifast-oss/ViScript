// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — bộ lặp `for` dùng chung interp/VM, tách khỏi `vis-core::value` vì
//! cần hằng trần vòng lặp + trap runtime của `vis-interp`. Chuyển **nguyên văn** từ `value.rs`.

use vis_core::value::Value;

/// Bộ lặp của `for` trên **mọi** kiểu lặp được (list/range/str/map) — dùng **chung** cho interp,
/// VM (ST-20) và frame evaluator.
///
/// **VD-264**: `Range` duyệt **lười** bằng con trỏ `i64` (không dựng `Vec<Value>` N phần tử như
/// trước — `for i in 0..2_000_000` từng tốn ~146 MB). Các kiểu khác vẫn vật chất hoá đúng như cũ.
pub struct ForIter {
    kind: ForIterKind,
}

enum ForIterKind {
    /// `a..b` / `a..=b`: `cur` là phần tử kế, `done` chặn sau khi chạm cận/`i64::MAX`.
    Range {
        cur: i64,
        end: i64,
        incl: bool,
        done: bool,
    },
    /// list/map/str — đã vật chất hoá (ngữ nghĩa các kiểu này không đổi).
    Items(std::vec::IntoIter<Value>),
}

impl Iterator for ForIter {
    type Item = Value;

    fn next(&mut self) -> Option<Value> {
        match &mut self.kind {
            ForIterKind::Range {
                cur,
                end,
                incl,
                done,
            } => {
                if *done {
                    return None;
                }
                let het = if *incl { *cur > *end } else { *cur >= *end };
                if het {
                    *done = true;
                    return None;
                }
                let v = *cur;
                // ST-13b (mục 2): **không** cộng tràn — `..=` với `i64::MAX` từng panic.
                match cur.checked_add(1) {
                    Some(n) => *cur = n,
                    None => *done = true, // chạm `i64::MAX` ⇒ hết dải, **không** panic
                }
                Some(Value::Int(v))
            }
            ForIterKind::Items(xs) => xs.next(),
        }
    }
}

/// Vật phẩm của `for` trên **mọi** kiểu lặp được (list/range/str/map) — dùng **chung** cho interp
/// và VM (ST-20). Khoảng quá dài ⇒ `VS0501` (ST-13b: không cộng tràn `i64`).
///
/// **VD-264**: trả [`ForIter`]; range **không** cấp phát N phần tử. Trần `10^7` được kiểm **trước**
/// khi vào vòng (đếm bằng `i128`, không tràn) để giữ đúng thời điểm trap của bản vật chất hoá cũ:
/// trap trước mọi thân vòng và trước `E_LOOP` đầu tiên.
pub fn iter_items(it: Value, span: vis_syntax::Span) -> vis_core::trap::R<ForIter> {
    let kind = match it {
        Value::List(xs) => ForIterKind::Items(std::rc::Rc::unwrap_or_clone(xs).into_iter()),
        Value::Range(a, b, incl) => {
            let n: i128 = if incl {
                if a > b {
                    0
                } else {
                    i128::from(b) - i128::from(a) + 1
                }
            } else if a >= b {
                0
            } else {
                i128::from(b) - i128::from(a)
            };
            if n > i128::from(crate::rt::fee::MAX_LOOP_ITERATIONS) {
                return Err(vis_core::trap::Abort::Trap(vis_core::trap::Trap::new(
                    crate::codes::RT,
                    "khoảng quá dài (>10^7 phần tử)",
                    span,
                )));
            }
            ForIterKind::Range {
                cur: a,
                end: b,
                incl,
                done: false,
            }
        }
        // ST-13b (mục 4): duyệt chuỗi cho **`char`** (không phải `str`).
        Value::Str(s) => {
            ForIterKind::Items(s.chars().map(Value::Char).collect::<Vec<_>>().into_iter())
        }
        Value::Map(kv) => {
            // **ST-315b**: `Rc<VisMap>` — bóc khi không chia sẻ để khỏi chép entry, còn lại
            // chép N cặp **một lần** cho vòng `for` (như `Vec` cũ).
            let entries = match std::rc::Rc::try_unwrap(kv) {
                Ok(m) => m.into_entries(),
                Err(rc) => rc.iter().cloned().collect(),
            };
            ForIterKind::Items(
                entries
                    .into_iter()
                    .map(|(k, v)| Value::list(vec![k, v]))
                    .collect::<Vec<_>>()
                    .into_iter(),
            )
        }
        other => {
            return Err(vis_core::trap::Abort::Trap(vis_core::trap::Trap::new(
                crate::codes::RT,
                format!("`for` không duyệt được `{}`", other.show()),
                span,
            )))
        }
    };
    Ok(ForIter { kind })
}
