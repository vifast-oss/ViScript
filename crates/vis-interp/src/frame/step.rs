// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318h (VD-298)** — vòng step của frame (`step_frame_owned`), đánh giá biểu
//! thức trong env cho trước, và entry `run_frame_for_test`.
//!
//! Tách **nguyên văn** từ `frame.rs` (R0, 0 đổi hành vi); API `run_frame_for_test` giữ nguyên.

use super::*;

/// Chạy chương trình thật qua frame scheduler. Gọi từ integration test sau khi
/// parse/lower/check — **không** có CLI flag/API ngôn ngữ nào gọi tới đây.
#[doc(hidden)]
pub fn run_frame_for_test<'a>(
    text: &str,
    hir: &'a vis_hir::hir::Hir,
    symbols: &'a vis_hir::Symbols,
    types: &'a vis_hir::TypeTable,
    args: &[String],
    config: FrameConfig,
) -> FrameOutput {
    let mut it = Interp::new(hir, symbols, types);
    run_frame_driver(&mut it, text, hir, symbols, types, args, config)
}

/// Đánh giá một biểu thức trong `env` cho trước (dùng cho hằng/default tham số).
pub(super) fn eval_expr_in<'a>(
    it: &mut Interp<'a>,
    e: &'a HExpr,
    env: &Env,
    mutable: &Rc<RefCell<HashSet<u32>>>,
) -> R<Value> {
    let mut f = IFrame::expr_frame(env, mutable.clone());
    // `Eval` đẩy giá trị vào `vals`; `SetLast` chuyển nó vào `last` để khi khung kết
    // thúc, giá trị trả về chính là giá trị biểu thức (không phải Unit).
    f.cur().work.push(Work::SetLast);
    f.cur().work.push(Work::Eval(e));
    step_frame_owned(it, &mut f).map(|v| v.unwrap_or(Value::Unit))
}

pub(super) fn step_frame_owned<'a>(
    it: &mut Interp<'a>,
    frame: &mut IFrame<'a>,
) -> R<Option<Value>> {
    loop {
        // VD-257/D — hết hạn ngạch: yield về ĐUÔI ready, KHÔNG trap. Điểm này là
        // continuation point: `frame` đang chứa toàn bộ `calls/work/vals/loops/
        // restore` và sẽ được `step_frame` cất lại nguyên vẹn (xem :1057). Work-item
        // trước đó đã charge + thi hành xong ⇒ resume không charge lại / không lặp
        // side-effect. Chỉ phát `Pending{task:None,wait:None}` — nhánh driver có sẵn
        // (`frame.ready.push_back(current)`), không cơ chế hàng đợi mới.
        if frame.done() {
            return Ok(frame.vals.pop());
        }
        // Không yield một frame ĐÃ xong (tránh turn rỗng, khớp VM ở loop-bottom).
        if crate::rt::fee::quota_take_hit() {
            let cur = it.sched.borrow().frame.current;
            it.sched.borrow_mut().frame.log.push(format!("quota:{cur}"));
            return Err(Abort::Pending(crate::rt::Pending {
                task: None,
                wait: None,
                span: Span::new(0, 0),
            }));
        }
        let w = frame.cur().work.pop();
        match w {
            Some(w) => process_work(it, frame, w)?,
            None => {
                // Hết work ⇒ kết thúc khung con: hoàn nguyên tham số, nâng ret, đẩy giá trị.
                let cf = frame.calls.pop().expect("khung");
                let mut v = cf.last.unwrap_or(Value::Unit);
                if cf.ret_depth > 0 {
                    v = crate::lift::lift_opt(v, cf.ret_depth);
                }
                // **VD-258**: closure có env riêng cho lần gọi này — gộp các biến bắt
                // về env gốc của closure (tham số bị loại ⇒ lần gọi khác không bị đè).
                if let Some(dich) = &cf.merge {
                    let params: Vec<u32> = cf.restore.iter().map(|(s, _)| *s).collect();
                    let snapshot = cf.env.borrow().clone();
                    let mut den = dich.borrow_mut();
                    for (sym, val) in snapshot {
                        if !params.contains(&sym) {
                            den.insert(sym, val);
                        }
                    }
                }
                for (sym, old) in cf.restore {
                    match old {
                        Some(old) => {
                            cf.env.borrow_mut().insert(sym, old);
                        }
                        None => {
                            cf.env.borrow_mut().remove(&sym);
                        }
                    }
                }
                frame.vals.push(v);
            }
        }
    }
}
