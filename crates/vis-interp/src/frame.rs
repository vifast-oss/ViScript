// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-267 / VD-251 — LÁT 1 (chỉ test/debug)**, mở rộng **ST-274 / VD-255**:
//! frame-aware scheduler nối vào **đường interp thật**.
//!
//! Đây **không** phải model/harness độc lập: `run_frame_for_test` dựng đúng [`Interp`]
//! thật (HIR thật sau parse/lower/check), dùng [`crate::builtin::call`], [`crate::ops::bin`],
//! [`crate::value::Value`], [`crate::interp::Trap`] và [`crate::rt::Scheduler`] thật. Điểm khác
//! duy nhất là **chiến lược đánh giá**: thay vì Rust call stack (`eval`/`exec_stmt` đệ quy),
//! frame chạy trên một **work stack nằm heap** (`IFrame`) ⇒ lưu được vị trí giữa biểu thức
//! khi `await` treo. Runtime mặc định (`Interp::run_program`) **không** gọi module này.
//!
//! **VD-255** mở rộng work stack để phủ: `if`/`match`, vòng lặp (`while`/`loop`/`for`/
//! `while let`), `break`/`continue`, short-circuit (`&&`/`||`/`??`/`|>`), field/index,
//! record/tuple/list/set/map/enum, toán tử một ngôi, đối số tên, hằng cấp tệp, và
//! closure với **ô bắt chia sẻ** + ghi ngược (writeback) qua suspend/resume.
//! Mọi cấu trúc KHÔNG phủ vẫn bị **từ chối rõ** (Trap có `Span`) ngay tại điểm gặp,
//! trước khi đẩy operand — không fallback âm thầm sang đường đồng bộ.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use vis_hir::hir::*;
use vis_syntax::Span;

use crate::interp::{lit_value, Abort, Interp, Trap, R, RT};
use crate::rt;
use crate::value::{Env, ForIter, Name, Value};

mod charge;
pub(crate) use charge::frame_charge;

mod expr;
mod helper;
mod params;
mod proc;
mod state;
mod step;
mod work;
mod worker;
pub(crate) use worker::run_frame_driver;

use expr::{call_value, dispatch, eval_work, exec_work};
use helper::{
    append_work, args_works, bind_pat_env, block_works, field_of, if_work, index_of, initial_work,
    pop_n, run_arm_body, take_call_args_names, type_name, underflow,
};
use params::{env_for_params, khoi_tao_bien_cap_tep_frame, opt_depth_ty};
use proc::process_work;
pub use state::IFrame;
use state::{CallFrame, LoopKindState, LoopState};
use step::eval_expr_in;
pub use step::run_frame_for_test;
#[cfg(test)]
use step::step_frame_owned;
use work::{Callee, LoopKindInit, MethodKind, Work};

pub(crate) use vis_core::frame::{FrameConfig, FrameOutput};

#[cfg(test)]
mod tests {
    use super::*;

    /// **VD-258**: mỗi lần gọi closure phải có env **riêng** — tham số của lần gọi đang
    /// treo không bị lần gọi khác ghi đè/xoá. Trước đây mọi lần gọi dùng chung `Closure.env`.
    #[test]
    fn closure_moi_lan_goi_env_rieng() {
        let hir = vis_hir::Hir {
            items: Vec::new(),
            stmts: Vec::new(),
            span: Span::new(0, 0),
        };
        let symbols = vis_hir::Symbols::default();
        let types = vis_hir::TypeTable::default();
        let mut it = Interp::new(&hir, &symbols, &types);

        let env: Env = Rc::new(RefCell::new(HashMap::new()));
        env.borrow_mut().insert(99, Value::Int(0)); // biến bắt tượng trưng
        let mk = || {
            Value::Fn(Rc::new(crate::value::Closure {
                name: "f".into(),
                params: vec![7],
                param_depths: vec![0],
                body: HBody::Stmts(Vec::new()),
                env: env.clone(),
                ret_depth: 0,
                frame_body: std::cell::OnceCell::new(),
            }))
        };
        let mut frame = IFrame::expr_frame(&env, Rc::new(RefCell::new(HashSet::new())));
        call_value(
            &mut it,
            &mut frame,
            mk(),
            vec![Value::Int(1)],
            Vec::new(),
            Span::new(0, 0),
        )
        .expect("call A");
        call_value(
            &mut it,
            &mut frame,
            mk(),
            vec![Value::Int(2)],
            Vec::new(),
            Span::new(0, 0),
        )
        .expect("call B");

        assert_eq!(frame.calls.len(), 3, "khung gốc + 2 lần gọi");
        let a = &frame.calls[1];
        let b = &frame.calls[2];
        assert!(!Rc::ptr_eq(&a.env, &b.env), "hai lần gọi phải khác env");
        assert!(!Rc::ptr_eq(&a.env, &env), "không dùng chung env closure");
        assert!(
            matches!(a.env.borrow().get(&7), Some(Value::Int(1))),
            "lần gọi A giữ tham số riêng"
        );
        assert!(
            matches!(b.env.borrow().get(&7), Some(Value::Int(2))),
            "lần gọi B giữ tham số riêng"
        );
        assert!(
            env.borrow().get(&7).is_none(),
            "tham số không được rò vào env gốc của closure"
        );
        assert!(matches!(env.borrow().get(&99), Some(Value::Int(0))));
    }

    /// **VD-259**: tên chưa phân giải **không** được rơi vào ô argv (`u32::MAX`) của chế độ
    /// không có `main` — phải trap "chưa định nghĩa" như runtime mặc định.
    #[test]
    fn ten_khong_ton_tai_khong_nhan_argv() {
        let hir = vis_hir::Hir {
            items: Vec::new(),
            stmts: Vec::new(),
            span: Span::new(0, 0),
        };
        let symbols = vis_hir::Symbols::default();
        let types = vis_hir::TypeTable::default();
        let mut it = Interp::new(&hir, &symbols, &types);
        let env: Env = Rc::new(RefCell::new(HashMap::new()));
        env.borrow_mut()
            .insert(u32::MAX, Value::list(vec![Value::Str("argv".into())]));
        let mut frame = IFrame::expr_frame(&env, Rc::new(RefCell::new(HashSet::new())));
        let e = HExpr::Var {
            name: "khong_ton_tai".into(),
            sym: None,
            span: Span::new(0, 0),
        };
        frame.cur().work.push(Work::SetLast);
        frame.cur().work.push(Work::Eval(&e));
        let err = step_frame_owned(&mut it, &mut frame).expect_err("phải trap");
        match err {
            Abort::Trap(t) => assert!(t.msg.contains("chưa định nghĩa"), "message: {}", t.msg),
            other => panic!("không phải trap: {other:?}"),
        }
    }
}
