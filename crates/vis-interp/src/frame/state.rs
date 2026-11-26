// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318h (VD-298)** — trạng thái sống trên heap: `CallFrame`, `IFrame`,
//! `LoopState`/`LoopKindState` + constructor/truy cập `vals`.
//!
//! Tách **nguyên văn** từ `frame.rs` (R0, 0 đổi hành vi); API `IFrame` giữ nguyên.

use super::*;

/// Trạng thái vòng lặp sống trên heap (resume không mất).
pub(super) struct LoopState<'a> {
    pub(super) kind: LoopKindState<'a>,
    pub(super) body: &'a [HStmt],
    pub(super) span: Span,
}

pub(super) enum LoopKindState<'a> {
    While {
        cond: &'a HCond,
    },
    For {
        pat: &'a HPat,
        /// **VD-264**: range duyệt lười (không còn `Vec<Value>` N phần tử).
        items: ForIter,
    },
    Loop,
}

pub(super) struct CallFrame<'a> {
    /// Môi trường **chia sẻ** (ô bắt của closure trỏ vào chính `Rc` này).
    pub(super) env: Env,
    /// Số lớp `Opt` của kiểu trả về (nâng ở biên hàm như interp).
    pub(super) ret_depth: usize,
    /// Continuation stack — mọi trạng thái sống qua suspend.
    pub(super) work: Vec<Work<'a>>,
    /// Giá trị câu lệnh gần nhất (giá trị khối / giá trị trả hàm).
    pub(super) last: Option<Value>,
    /// Các vòng đang mở (trong cùng nằm cuối).
    pub(super) loops: Vec<LoopState<'a>>,
    /// Binding tham số cần hoàn nguyên khi khung kết thúc (closure gọi lại nhiều lần).
    pub(super) restore: Vec<(u32, Option<Value>)>,
    /// **VD-258**: env đích để gộp thay đổi của closure sau khi khung xong (mỗi lần gọi
    /// có env riêng ⇒ tham số không đè lên lần gọi đang treo khác).
    pub(super) merge: Option<Env>,
}

/// **Frame thật của interp trên heap** (đối trọng với `Th` của VM).
pub struct IFrame<'a> {
    pub(super) calls: Vec<CallFrame<'a>>,
    pub(super) vals: Vec<Value>,
    /// Tập `var` dùng chung toàn chương trình (khớp `Interp.mutable` — closure thấy `var` của ngoài).
    pub(super) mutable: Rc<RefCell<HashSet<u32>>>,
    /// **VD-258**: env cấp tệp (const/hằng) — hàm tên phải thấy như interp thật.
    pub(super) globals: Env,
}

impl<'a> IFrame<'a> {
    pub(super) fn new(
        body: &'a [HStmt],
        env: Env,
        globals: &Env,
        mutable: Rc<RefCell<HashSet<u32>>>,
    ) -> Self {
        Self {
            calls: vec![CallFrame {
                env,
                ret_depth: 0,
                work: initial_work(body),
                last: None,
                loops: Vec::new(),
                restore: Vec::new(),
                merge: None,
            }],
            vals: Vec::new(),
            mutable,
            globals: globals.clone(),
        }
    }

    /// Khung phụ dùng để đánh giá một biểu thức đơn (default của tham số / hằng).
    pub(super) fn expr_frame(env: &Env, mutable: Rc<RefCell<HashSet<u32>>>) -> Self {
        Self {
            calls: vec![CallFrame {
                env: env.clone(),
                ret_depth: 0,
                work: Vec::new(),
                last: None,
                loops: Vec::new(),
                restore: Vec::new(),
                merge: None,
            }],
            vals: Vec::new(),
            mutable,
            globals: env.clone(),
        }
    }

    pub(super) fn done(&self) -> bool {
        self.calls.is_empty()
    }

    pub(super) fn cur(&mut self) -> &mut CallFrame<'a> {
        self.calls.last_mut().expect("khung hiện tại")
    }

    pub(super) fn push_val(&mut self, v: Value) {
        self.vals.push(v);
    }

    pub(super) fn pop_val(&mut self) -> R<Value> {
        self.vals.pop().ok_or_else(underflow)
    }
}
