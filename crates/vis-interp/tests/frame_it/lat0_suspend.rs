// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-264 (VD-250) — LÁT 0, prototype NỘI BỘ.**
//!
//! Đây là **model spike**, KHÔNG phải đường `goi_than` thật của interp/VM.
//! Mục đích: chứng minh hai kiểu **frame trên heap** + vòng lặp scheduler **không đệ quy**
//! có thể dừng/tiếp tục đúng mà **không đổi runtime mặc định hay bytecode phát hành**.
//!
//! # Phạm vi được mô hình hoá (modeled) vs chứng minh (proven)
//!
//! - **Modeled**: một instruction set nhỏ + một HIR-like statement list; frame VM mirror
//!   `vm.rs:124-135` (`fi, pc, locals, captured, st_cu, writeback`); frame interp dùng
//!   **statement index + continuation stack nằm trên heap** (KHÔNG phải Rust call stack).
//! - **Proven (bởi test)**: A→suspend→B→resume đúng vị trí; local/capture giữ giá trị;
//!   resume không lặp side-effect; `Ok`/`Err` đi qua continuation; thứ tự đánh giá trái→phải
//!   với `await` lồng; vòng lặp phẳng, stack bị chặn.
//! - **KHÔNG proven ở đây**: tích hợp với HIR thật, bytecode thật, `goi_than` thật, I/O thật,
//!   đóng gói release, hay thay đổi ngôn ngữ. Đó là **integration gap** trước Lát 1 (xem JOURNAL).
//!
//! Mọi frame được cất trong `Scheduler::frames: Vec<F>`; `ready: VecDeque<usize>` là FIFO;
//! `step()` chạy MỘT frame tới điểm suspend rồi trả control cho vòng lặp. "I/O wait" là
//! cờ sự kiện giả (`Ctx::events`), KHÔNG có socket thật.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

// ===========================================================================
// Common: value, context, scheduler, frame trait
// ===========================================================================
mod common {
    use super::*;

    pub type Log = Rc<RefCell<Vec<String>>>;
    pub type Events = Rc<RefCell<HashMap<String, bool>>>;
    pub type Out = Rc<RefCell<String>>;

    /// Giá trị tối thiểu. `Err` là một **value** để đi qua continuation như `Trap`.
    #[derive(Clone, Debug, PartialEq)]
    pub enum V {
        Int(i64),
        Unit,
        Err(String),
    }

    impl V {
        pub fn is_err(&self) -> bool {
            matches!(self, V::Err(_))
        }
        pub fn as_int(&self) -> i64 {
            match self {
                V::Int(n) => *n,
                _ => panic!("prototype: kỳ vọng Int, nhận {:?}", self),
            }
        }
        pub fn show(&self) -> String {
            match self {
                V::Int(n) => n.to_string(),
                V::Unit => "unit".into(),
                V::Err(e) => format!("err({e})"),
            }
        }
    }

    /// Trạng thái frame (Pending = đang chờ event, Ready = có thể chạy).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum St {
        Ready,
        Pending,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum StepOutcome {
        Done,
        Suspended,
    }

    /// Bối cảnh chia sẻ: log tác dụng phụ, cờ event giả, stdout giả.
    pub struct Ctx {
        pub log: Log,
        pub events: Events,
        pub out: Out,
    }

    impl Ctx {
        pub fn new() -> Self {
            Self {
                log: Rc::new(RefCell::new(Vec::new())),
                events: Rc::new(RefCell::new(HashMap::new())),
                out: Rc::new(RefCell::new(String::new())),
            }
        }
        pub fn record(&self, s: impl Into<String>) {
            self.log.borrow_mut().push(s.into());
        }
        pub fn lines(&self) -> Vec<String> {
            self.log.borrow().clone()
        }
        pub fn fire(&self, ev: &str) {
            self.events.borrow_mut().insert(ev.to_string(), true);
        }
        pub fn fired(&self, ev: &str) -> bool {
            *self.events.borrow().get(ev).unwrap_or(&false)
        }
        pub fn print(&self, v: &V) {
            self.out.borrow_mut().push_str(&format!("{}\n", v.show()));
        }
        pub fn stdout(&self) -> String {
            self.out.borrow().clone()
        }
    }

    /// Scheduler phẳng: heap frame + hàng đợi FIFO. KHÔNG đệ quy, KHÔNG stack Rust.
    pub struct Scheduler<F> {
        pub frames: Vec<F>,
        pub ready: VecDeque<usize>,
    }

    impl<F> Scheduler<F> {
        pub fn new() -> Self {
            Self {
                frames: Vec::new(),
                ready: VecDeque::new(),
            }
        }
        pub fn spawn(&mut self, f: F) -> usize {
            let id = self.frames.len();
            self.frames.push(f);
            self.ready.push_back(id);
            id
        }
    }

    /// Hành vi chung của một frame trên heap, cho cả hai mô hình engine.
    pub trait HFrame {
        fn state(&self) -> St;
        fn set_state(&mut self, s: St);
        fn wait_ev(&self) -> Option<String>;
        fn set_resume(&mut self, r: V);
        fn result(&self) -> Option<&V>;
        fn step(&mut self, ctx: &Ctx) -> StepOutcome;
    }

    /// Chạy mọi frame Ready theo FIFO tới khi hàng đợi cạn (mỗi `step` là MỘT lượt).
    pub fn pump<F: HFrame>(s: &mut Scheduler<F>, ctx: &Ctx) {
        let mut guard = 0usize;
        while let Some(id) = s.ready.pop_front() {
            guard += 1;
            assert!(guard < 100_000, "prototype: scheduler không hội tụ");
            let _ = s.frames[id].step(ctx);
        }
    }

    /// Giả lập "I/O xong": bật cờ event, đánh thức mọi frame Pending chờ event đó,
    /// đưa giá trị resume (`Ok` hoặc `Err` dưới dạng `V`) vào frame, rồi xếp lại FIFO.
    pub fn wake<F: HFrame>(s: &mut Scheduler<F>, ctx: &Ctx, ev: &str, r: V) -> usize {
        ctx.fire(ev);
        let ids: Vec<usize> = s
            .frames
            .iter()
            .enumerate()
            .filter(|(_, f)| f.state() == St::Pending && f.wait_ev().as_deref() == Some(ev))
            .map(|(id, _)| id)
            .collect();
        for id in &ids {
            s.frames[*id].set_state(St::Ready);
            s.frames[*id].set_resume(r.clone());
            s.ready.push_back(*id);
        }
        ids.len()
    }
}

// ===========================================================================
// Engine 1 — VM-like: mirror crates/vis-vm/src/vm.rs:124-135 Frame
// ===========================================================================
mod vm_model {
    use super::common::*;
    use super::*;

    #[derive(Clone, Debug)]
    pub enum Instr {
        Push(V),
        EffectVal(String, V), // log rồi đẩy giá trị (side-effect TRƯỚC await)
        Log(String),          // chỉ log, KHÔNG đẩy giá trị (giữ kết quả tính toán)
        LogVal(String),       // log "{label}={top}", giữ top
        Print,                // ghi top + "\n" vào stdout giả
        Await(String),        // tiêu thụ handle trên st_cu, park nếu event chưa bật
        Add,
        LoadLocal(usize),
        StoreLocal(usize),
        LoadCapture(usize),
        /// Ghi `captured[cap]` và `locals[local]`; khi Ret, writeback đẩy ra ô chia sẻ.
        WriteCapture {
            local: usize,
            cap: usize,
        },
        Ret,
    }

    /// Mirror *trực tiếp* kiểu của `vm.rs:120-122`.
    type ClosureCells = Rc<RefCell<Vec<V>>>;
    type CapturedMirror = Vec<(u16, usize)>;

    /// Mirror *trực tiếp* các trường của `vm.rs` Frame (đều nằm trên heap).
    pub struct VmFrame {
        pub fi: u32,
        pub pc: usize,
        pub locals: Vec<V>,
        pub captured: Vec<V>,
        pub st_cu: Vec<V>,
        pub writeback: Option<(ClosureCells, CapturedMirror)>,
        // --- phần prototype: code + trạng thái scheduler ---
        pub code: Rc<Vec<Instr>>,
        pub state: St,
        pub wait_ev: Option<String>,
        pub resume: Option<V>,
        pub result: Option<V>,
        pub max_stack: usize,
    }

    impl VmFrame {
        pub fn new(
            fi: u32,
            code: Vec<Instr>,
            locals: Vec<V>,
            captured: Vec<V>,
            cells: Option<Rc<RefCell<Vec<V>>>>,
        ) -> Self {
            Self {
                fi,
                pc: 0,
                locals,
                captured,
                st_cu: Vec::new(),
                writeback: cells.map(|c| (c, vec![(1u16, 0usize)])),
                code: Rc::new(code),
                state: St::Ready,
                wait_ev: None,
                resume: None,
                result: None,
                max_stack: 0,
            }
        }
        fn push(&mut self, v: V) {
            self.st_cu.push(v);
            self.max_stack = self.max_stack.max(self.st_cu.len());
        }
        fn top_is_err(&self) -> bool {
            self.st_cu.last().map(|v| v.is_err()).unwrap_or(false)
        }
    }

    impl HFrame for VmFrame {
        fn state(&self) -> St {
            self.state
        }
        fn set_state(&mut self, s: St) {
            self.state = s;
        }
        fn wait_ev(&self) -> Option<String> {
            self.wait_ev.clone()
        }
        fn set_resume(&mut self, r: V) {
            self.resume = Some(r);
        }
        fn result(&self) -> Option<&V> {
            self.result.as_ref()
        }

        fn step(&mut self, ctx: &Ctx) -> StepOutcome {
            // Resume: event đã bật + có giá trị ⇒ **tiến pc qua Await** rồi đẩy kết quả.
            // Không chạy lại instruction trước Await ⇒ không lặp side-effect.
            if self.state == St::Ready && self.wait_ev.is_some() && self.resume.is_some() {
                let r = self.resume.take().unwrap();
                self.wait_ev = None;
                self.pc += 1;
                self.push(r);
            }

            while self.pc < self.code.len() {
                let ins = self.code[self.pc].clone();
                match ins {
                    Instr::Push(v) => {
                        self.push(v);
                        self.pc += 1;
                    }
                    Instr::EffectVal(label, v) => {
                        if !self.top_is_err() {
                            ctx.record(label);
                            self.push(v);
                        }
                        self.pc += 1;
                    }
                    Instr::Log(label) => {
                        if !self.top_is_err() {
                            ctx.record(label);
                        }
                        self.pc += 1;
                    }
                    Instr::LogVal(label) => {
                        if !self.top_is_err() {
                            let v = self.st_cu.last().cloned().unwrap_or(V::Unit);
                            ctx.record(format!("{label}={}", v.show()));
                        }
                        self.pc += 1;
                    }
                    Instr::Print => {
                        let v = self.st_cu.last().cloned().unwrap_or(V::Unit);
                        if !v.is_err() {
                            ctx.print(&v);
                        }
                        self.pc += 1;
                    }
                    Instr::Await(ev) => {
                        // Operand (handle) đã được đánh giá bởi instruction trước ⇒ tiêu thụ.
                        let _handle = self.st_cu.pop();
                        if !ctx.fired(&ev) {
                            self.state = St::Pending;
                            self.wait_ev = Some(ev);
                            return StepOutcome::Suspended; // pc vẫn ở Await
                        }
                        // Trường hợp event bật sẵn (không đi qua wake): giá trị mặc định.
                        self.push(V::Unit);
                        self.pc += 1;
                    }
                    Instr::Add => {
                        let b = self.st_cu.pop().unwrap_or(V::Unit);
                        let a = self.st_cu.pop().unwrap_or(V::Unit);
                        let r = if a.is_err() {
                            a
                        } else if b.is_err() {
                            b
                        } else {
                            V::Int(a.as_int() + b.as_int())
                        };
                        self.push(r);
                        self.pc += 1;
                    }
                    Instr::LoadLocal(i) => {
                        let v = self.locals.get(i).cloned().unwrap_or(V::Unit);
                        self.push(v);
                        self.pc += 1;
                    }
                    Instr::StoreLocal(i) => {
                        let v = self.st_cu.pop().unwrap_or(V::Unit);
                        if i < self.locals.len() {
                            self.locals[i] = v;
                        }
                        self.pc += 1;
                    }
                    Instr::LoadCapture(i) => {
                        let v = self.captured.get(i).cloned().unwrap_or(V::Unit);
                        self.push(v);
                        self.pc += 1;
                    }
                    Instr::WriteCapture { local, cap } => {
                        let v = self.st_cu.pop().unwrap_or(V::Unit);
                        if cap < self.captured.len() {
                            self.captured[cap] = v.clone();
                        }
                        if local < self.locals.len() {
                            self.locals[local] = v;
                        }
                        self.pc += 1;
                    }
                    Instr::Ret => {
                        let v = self.st_cu.pop().unwrap_or(V::Unit);
                        if let Some((cells, mirror)) = &self.writeback {
                            for &(local, cap) in mirror {
                                let val =
                                    self.locals.get(local as usize).cloned().unwrap_or(V::Unit);
                                if cap < cells.borrow().len() {
                                    cells.borrow_mut()[cap] = val;
                                }
                            }
                        }
                        self.result = Some(v);
                        self.pc += 1;
                        return StepOutcome::Done;
                    }
                }
                self.max_stack = self.max_stack.max(self.st_cu.len());
            }
            StepOutcome::Done
        }
    }
}

// ===========================================================================
// Engine 2 — Interp-like: statement index + continuation stack trên HEAP
// ===========================================================================
mod interp_model {
    use super::common::*;
    use super::*;

    #[derive(Clone, Debug)]
    pub enum Expr {
        Int(i64),
        Var(String),
        Cap(String),
        Add(Box<Expr>, Box<Expr>),
        /// Đánh giá operand (side-effect), rồi suspend; resume trả giá trị vào continuation.
        Await(String, Box<Expr>),
        Effect(String, Box<Expr>),
        LogVal(String, Box<Expr>),
        Print(Box<Expr>),
    }

    #[derive(Clone, Debug)]
    pub enum Stmt {
        SetLocal(String, Expr),
        SetCap(String, Expr),
        Eval(Expr),
        Return(Expr),
    }

    /// Continuation nằm trên **heap** (trường `work` của frame), KHÔNG phải Rust call stack.
    #[derive(Clone, Debug)]
    enum Work {
        Eval(Expr),
        AwaitHold(String),
        EffectHold(String),
        LogHold(String),
        PrintHold,
        AddOp,
        StoreLocal(String),
        StoreCap(String),
        FinishStmt,
    }

    pub struct IFrame {
        pub fi: u32,
        pub state: St,
        pub wait_ev: Option<String>,
        pub resume: Option<V>,
        pub stmts: Rc<Vec<Stmt>>,
        pub pc: usize, // vị trí câu lệnh HIỂN THỰC (explicit statement index)
        pub locals: HashMap<String, V>,
        pub captured: HashMap<String, V>,
        /// Ô capture chia sẻ (mirror `ClosureCells` của VM).
        pub cells: Rc<RefCell<HashMap<String, V>>>,
        work: Vec<Work>,
        vals: Vec<V>,
        pub result: Option<V>,
        pub max_work: usize,
        pub max_vals: usize,
    }

    impl IFrame {
        pub fn new(fi: u32, stmts: Vec<Stmt>, cells: Rc<RefCell<HashMap<String, V>>>) -> Self {
            let captured = cells.borrow().clone();
            Self {
                fi,
                state: St::Ready,
                wait_ev: None,
                resume: None,
                stmts: Rc::new(stmts),
                pc: 0,
                locals: HashMap::new(),
                captured,
                cells,
                work: Vec::new(),
                vals: Vec::new(),
                result: None,
                max_work: 0,
                max_vals: 0,
            }
        }
        fn track(&mut self) {
            self.max_work = self.max_work.max(self.work.len());
            self.max_vals = self.max_vals.max(self.vals.len());
        }
        fn push_work(&mut self, w: Work) {
            self.work.push(w);
            self.track();
        }
        fn eval(&mut self, e: Expr) {
            match e {
                Expr::Int(n) => self.vals.push(V::Int(n)),
                Expr::Var(n) => {
                    let v = self
                        .locals
                        .get(&n)
                        .cloned()
                        .unwrap_or(V::Err(format!("no local {n}")));
                    self.vals.push(v);
                }
                Expr::Cap(n) => {
                    let v = self
                        .captured
                        .get(&n)
                        .cloned()
                        .unwrap_or(V::Err(format!("no capture {n}")));
                    self.vals.push(v);
                }
                Expr::Add(a, b) => {
                    // LIFO: đẩy AddOp, rồi b, rồi a ⇒ Pop theo thứ tự a, b, AddOp (trái→phải).
                    self.push_work(Work::AddOp);
                    self.push_work(Work::Eval(*b));
                    self.push_work(Work::Eval(*a));
                }
                Expr::Await(ev, op) => {
                    self.push_work(Work::AwaitHold(ev));
                    self.push_work(Work::Eval(*op));
                }
                Expr::Effect(label, op) => {
                    self.push_work(Work::EffectHold(label));
                    self.push_work(Work::Eval(*op));
                }
                Expr::LogVal(label, op) => {
                    self.push_work(Work::LogHold(label));
                    self.push_work(Work::Eval(*op));
                }
                Expr::Print(op) => {
                    self.push_work(Work::PrintHold);
                    self.push_work(Work::Eval(*op));
                }
            }
            self.track();
        }
    }

    impl HFrame for IFrame {
        fn state(&self) -> St {
            self.state
        }
        fn set_state(&mut self, s: St) {
            self.state = s;
        }
        fn wait_ev(&self) -> Option<String> {
            self.wait_ev.clone()
        }
        fn set_resume(&mut self, r: V) {
            self.resume = Some(r);
        }
        fn result(&self) -> Option<&V> {
            self.result.as_ref()
        }

        fn step(&mut self, ctx: &Ctx) -> StepOutcome {
            // Resume: đẩy kết quả I/O vào `vals`; continuation cha đang ở đỉnh `work`
            // sẽ tiêu thụ nó — operand KHÔNG bị đánh giá lại.
            if self.state == St::Ready && self.wait_ev.is_some() && self.resume.is_some() {
                let r = self.resume.take().unwrap();
                self.wait_ev = None;
                self.vals.push(r);
                self.track();
            }

            while self.pc < self.stmts.len() {
                if self.work.is_empty() {
                    let s = self.stmts[self.pc].clone();
                    match s {
                        Stmt::SetLocal(n, e) => {
                            self.push_work(Work::FinishStmt);
                            self.push_work(Work::StoreLocal(n));
                            self.push_work(Work::Eval(e));
                        }
                        Stmt::SetCap(n, e) => {
                            self.push_work(Work::FinishStmt);
                            self.push_work(Work::StoreCap(n));
                            self.push_work(Work::Eval(e));
                        }
                        Stmt::Eval(e) | Stmt::Return(e) => {
                            self.push_work(Work::FinishStmt);
                            self.push_work(Work::Eval(e));
                        }
                    }
                }
                let w = self.work.pop().expect("prototype: work rỗng sau khi fetch");
                match w {
                    Work::Eval(e) => self.eval(e),
                    Work::AwaitHold(ev) => {
                        let handle = self.vals.pop().unwrap_or(V::Unit);
                        if handle.is_err() {
                            self.vals.push(handle);
                        } else if !ctx.fired(&ev) {
                            self.state = St::Pending;
                            self.wait_ev = Some(ev);
                            return StepOutcome::Suspended;
                        } else {
                            let r = self.resume.take().unwrap_or(V::Unit);
                            self.vals.push(r);
                        }
                    }
                    Work::EffectHold(label) => {
                        let v = self.vals.pop().unwrap_or(V::Unit);
                        if !v.is_err() {
                            ctx.record(label.clone());
                        }
                        self.vals.push(v);
                    }
                    Work::LogHold(label) => {
                        let v = self.vals.pop().unwrap_or(V::Unit);
                        if !v.is_err() {
                            ctx.record(format!("{label}={}", v.show()));
                        }
                        self.vals.push(v);
                    }
                    Work::PrintHold => {
                        let v = self.vals.pop().unwrap_or(V::Unit);
                        if !v.is_err() {
                            ctx.print(&v);
                        }
                        self.vals.push(v);
                    }
                    Work::AddOp => {
                        let b = self.vals.pop().unwrap_or(V::Unit);
                        let a = self.vals.pop().unwrap_or(V::Unit);
                        let r = if a.is_err() {
                            a
                        } else if b.is_err() {
                            b
                        } else {
                            V::Int(a.as_int() + b.as_int())
                        };
                        self.vals.push(r);
                    }
                    Work::StoreLocal(n) => {
                        let v = self.vals.pop().unwrap_or(V::Unit);
                        self.locals.insert(n, v);
                    }
                    Work::StoreCap(n) => {
                        let v = self.vals.pop().unwrap_or(V::Unit);
                        self.captured.insert(n.clone(), v.clone());
                        self.cells.borrow_mut().insert(n, v);
                    }
                    Work::FinishStmt => {
                        self.result = self.vals.pop();
                        self.vals.clear();
                        self.pc += 1;
                    }
                }
                self.track();
            }
            StepOutcome::Done
        }
    }
}

// ===========================================================================
// KỊCH BẢN DÙNG CHUNG + TESTS
// ===========================================================================
use common::*;
use interp_model::{Expr, IFrame, Stmt};
use vm_model::{Instr, VmFrame};

fn new_cells_v(n: usize) -> Rc<RefCell<Vec<V>>> {
    Rc::new(RefCell::new(vec![V::Int(0); n]))
}
fn new_cells_i(keys: &[&str]) -> Rc<RefCell<HashMap<String, V>>> {
    Rc::new(RefCell::new(
        keys.iter().map(|k| (k.to_string(), V::Int(0))).collect(),
    ))
}

// ------------------------- ENGINE VM --------------------------------------

/// Tiêu chí 1+3: A chạy→suspend→B chạy→A resume đúng vị trí; side-effect không lặp.
#[test]
fn vm_a_suspends_b_runs_a_resumes() {
    let ctx = Ctx::new();
    let mut s = Scheduler::new();
    s.spawn(VmFrame::new(
        1,
        vec![
            Instr::EffectVal("A:pre".into(), V::Int(1)),
            Instr::Await("ioA".into()),
            Instr::EffectVal("A:post".into(), V::Int(3)),
            Instr::Ret,
        ],
        vec![],
        vec![],
        None,
    ));
    s.spawn(VmFrame::new(
        2,
        vec![Instr::EffectVal("B:run".into(), V::Int(2)), Instr::Ret],
        vec![],
        vec![],
        None,
    ));

    pump(&mut s, &ctx);
    assert_eq!(
        ctx.lines(),
        vec!["A:pre", "B:run"],
        "A phải suspend trước khi B chạy"
    );
    assert_eq!(s.frames[0].state(), St::Pending);
    assert_eq!(s.frames[1].state(), St::Ready);

    assert_eq!(wake(&mut s, &ctx, "ioA", V::Int(9)), 1);
    pump(&mut s, &ctx);
    assert_eq!(
        ctx.lines(),
        vec!["A:pre", "B:run", "A:post"],
        "A:post phải chạy SAU B; A:pre không được lặp"
    );
    assert_eq!(s.frames[0].result(), Some(&V::Int(3)));
}

/// Tiêu chí 2: locals + captures giữ giá trị qua suspend; writeback ra ô chia sẻ.
#[test]
fn vm_locals_captures_survive_suspend() {
    let ctx = Ctx::new();
    let cells = new_cells_v(1);
    let mut s = Scheduler::new();
    s.spawn(VmFrame::new(
        1,
        vec![
            Instr::Push(V::Int(10)),
            Instr::StoreLocal(0), // x = 10 (trước suspend)
            Instr::Push(V::Int(7)),
            Instr::WriteCapture { local: 1, cap: 0 }, // c = 7 (trước suspend)
            Instr::Await("ioC".into()),
            Instr::LoadLocal(0),
            Instr::LoadCapture(0),
            Instr::Add,
            Instr::Ret,
        ],
        vec![V::Unit, V::Unit],
        vec![V::Int(0)],
        Some(cells.clone()),
    ));

    pump(&mut s, &ctx);
    assert_eq!(wake(&mut s, &ctx, "ioC", V::Unit), 1);
    pump(&mut s, &ctx);
    assert_eq!(
        s.frames[0].result(),
        Some(&V::Int(17)),
        "10 + 7 phải sống qua suspend"
    );
    assert_eq!(cells.borrow()[0], V::Int(7), "writeback capture phải đúng");
    assert!(s.frames[0].max_stack <= 4, "stack phải nhỏ, không phình");
}

/// Tiêu chí 4: Ok VÀ Err đều đi qua continuation (Add → Ret).
#[test]
fn vm_result_and_error_through_continuation() {
    let code = || {
        vec![
            Instr::EffectVal("wait-io".into(), V::Int(0)),
            Instr::Await("ioR".into()),
            Instr::Push(V::Int(1)),
            Instr::Add,
            Instr::Log("after-await".into()),
            Instr::Ret,
        ]
    };

    let ctx_ok = Ctx::new();
    let mut s_ok = Scheduler::new();
    s_ok.spawn(VmFrame::new(1, code(), vec![], vec![], None));
    pump(&mut s_ok, &ctx_ok);
    wake(&mut s_ok, &ctx_ok, "ioR", V::Int(41));
    pump(&mut s_ok, &ctx_ok);
    assert_eq!(s_ok.frames[0].result(), Some(&V::Int(42)));
    assert!(ctx_ok.lines().contains(&"after-await".to_string()));

    let ctx_err = Ctx::new();
    let mut s_err = Scheduler::new();
    s_err.spawn(VmFrame::new(1, code(), vec![], vec![], None));
    pump(&mut s_err, &ctx_err);
    wake(&mut s_err, &ctx_err, "ioR", V::Err("io that fail".into()));
    pump(&mut s_err, &ctx_err);
    assert_eq!(
        s_err.frames[0].result(),
        Some(&V::Err("io that fail".into()))
    );
    assert!(
        !ctx_err.lines().contains(&"after-await".to_string()),
        "Err phải short-circuit"
    );
}

/// Tiêu chí 5: await trong BIỂU THỨC LỒNG — thứ tự trái→phải được bảo toàn.
#[test]
fn vm_nested_expression_order() {
    let ctx = Ctx::new();
    let mut s = Scheduler::new();
    s.spawn(VmFrame::new(
        1,
        vec![
            Instr::EffectVal("operand-eval".into(), V::Int(0)), // operand await
            Instr::Await("ioN".into()),                         // suspend SAU operand
            Instr::EffectVal("g".into(), V::Int(2)),            // vế phải
            Instr::Add,
            Instr::LogVal("call".into()),
            Instr::Ret,
        ],
        vec![],
        vec![],
        None,
    ));
    pump(&mut s, &ctx);
    assert_eq!(ctx.lines(), vec!["operand-eval"]);
    wake(&mut s, &ctx, "ioN", V::Int(40));
    pump(&mut s, &ctx);
    assert_eq!(
        ctx.lines(),
        vec!["operand-eval", "g", "call=42"],
        "trái→phải: operand, rồi g, rồi outer call"
    );
}

// ------------------------- ENGINE INTERP ----------------------------------

/// Tiêu chí 1+3 (interp): A suspend → B → A resume; continuation nằm heap.
#[test]
fn interp_a_suspends_b_runs_a_resumes() {
    let ctx = Ctx::new();
    let cells = new_cells_i(&[]);
    let mut s = Scheduler::new();
    s.spawn(IFrame::new(
        1,
        vec![
            Stmt::Eval(Expr::Effect("A:pre".into(), Box::new(Expr::Int(1)))),
            Stmt::Eval(Expr::Await("ioA".into(), Box::new(Expr::Int(0)))),
            Stmt::Return(Expr::Effect("A:post".into(), Box::new(Expr::Int(3)))),
        ],
        cells.clone(),
    ));
    s.spawn(IFrame::new(
        2,
        vec![Stmt::Return(Expr::Effect(
            "B:run".into(),
            Box::new(Expr::Int(2)),
        ))],
        cells,
    ));

    pump(&mut s, &ctx);
    assert_eq!(ctx.lines(), vec!["A:pre", "B:run"]);
    assert_eq!(s.frames[0].state(), St::Pending);
    wake(&mut s, &ctx, "ioA", V::Int(9));
    pump(&mut s, &ctx);
    assert_eq!(ctx.lines(), vec!["A:pre", "B:run", "A:post"]);
    assert_eq!(s.frames[0].result(), Some(&V::Int(3)));
}

/// Tiêu chí 2 (interp): local/capture sống qua suspend; cells chia sẻ được ghi.
#[test]
fn interp_locals_captures_survive_suspend() {
    let ctx = Ctx::new();
    let cells = new_cells_i(&["c"]);
    let mut s = Scheduler::new();
    s.spawn(IFrame::new(
        1,
        vec![
            Stmt::SetLocal("x".into(), Expr::Int(10)),
            Stmt::SetCap("c".into(), Expr::Int(7)),
            Stmt::Eval(Expr::Await("ioC".into(), Box::new(Expr::Int(0)))),
            Stmt::Return(Expr::Add(
                Box::new(Expr::Var("x".into())),
                Box::new(Expr::Cap("c".into())),
            )),
        ],
        cells.clone(),
    ));
    pump(&mut s, &ctx);
    wake(&mut s, &ctx, "ioC", V::Unit);
    pump(&mut s, &ctx);
    assert_eq!(s.frames[0].result(), Some(&V::Int(17)));
    assert_eq!(cells.borrow().get("c"), Some(&V::Int(7)));
    assert!(s.frames[0].max_work <= 8, "continuation stack phải nhỏ");
}

/// Tiêu chí 4 (interp): Ok VÀ Err đi qua continuation.
/// Dùng await lồng trong Add để kết quả resume chảy thẳng vào continuation cha.
#[test]
fn interp_result_and_error_through_continuation() {
    let stmts = || {
        vec![Stmt::Return(Expr::Add(
            Box::new(Expr::Await(
                "ioR".into(),
                Box::new(Expr::Effect("wait-io".into(), Box::new(Expr::Int(0)))),
            )),
            Box::new(Expr::Int(1)),
        ))]
    };

    let ctx_ok = Ctx::new();
    let mut s_ok = Scheduler::new();
    s_ok.spawn(IFrame::new(1, stmts(), new_cells_i(&[])));
    pump(&mut s_ok, &ctx_ok);
    assert_eq!(ctx_ok.lines(), vec!["wait-io"]);
    wake(&mut s_ok, &ctx_ok, "ioR", V::Int(41));
    pump(&mut s_ok, &ctx_ok);
    assert_eq!(s_ok.frames[0].result(), Some(&V::Int(42)));

    let ctx_err = Ctx::new();
    let mut s_err = Scheduler::new();
    s_err.spawn(IFrame::new(1, stmts(), new_cells_i(&[])));
    pump(&mut s_err, &ctx_err);
    wake(&mut s_err, &ctx_err, "ioR", V::Err("boom".into()));
    pump(&mut s_err, &ctx_err);
    assert_eq!(s_err.frames[0].result(), Some(&V::Err("boom".into())));
    assert_eq!(
        ctx_err.lines(),
        vec!["wait-io"],
        "Err không tạo thêm side-effect"
    );
}

/// Tiêu chí 5 (interp): await lồng trong expression, thứ tự trái→phải.
#[test]
fn interp_nested_expression_order() {
    let ctx = Ctx::new();
    let mut s = Scheduler::new();
    let expr = Expr::LogVal(
        "call".into(),
        Box::new(Expr::Add(
            Box::new(Expr::Await(
                "ioN".into(),
                Box::new(Expr::Effect("operand-eval".into(), Box::new(Expr::Int(0)))),
            )),
            Box::new(Expr::Effect("g".into(), Box::new(Expr::Int(2)))),
        )),
    );
    s.spawn(IFrame::new(1, vec![Stmt::Return(expr)], new_cells_i(&[])));
    pump(&mut s, &ctx);
    assert_eq!(ctx.lines(), vec!["operand-eval"]);
    wake(&mut s, &ctx, "ioN", V::Int(40));
    pump(&mut s, &ctx);
    assert_eq!(ctx.lines(), vec!["operand-eval", "g", "call=42"]);
}

/// Tiêu chí 6: hai engine cho CÙNG trace cho ca lồng ⇒ parity + stack bị chặn.
#[test]
fn both_engines_parity_and_flat_stack() {
    let expect = vec![
        "operand-eval".to_string(),
        "g".to_string(),
        "call=42".to_string(),
    ];

    let cv = Ctx::new();
    let mut sv = Scheduler::new();
    sv.spawn(VmFrame::new(
        1,
        vec![
            Instr::EffectVal("operand-eval".into(), V::Int(0)),
            Instr::Await("ioN".into()),
            Instr::EffectVal("g".into(), V::Int(2)),
            Instr::Add,
            Instr::LogVal("call".into()),
            Instr::Ret,
        ],
        vec![],
        vec![],
        None,
    ));
    pump(&mut sv, &cv);
    wake(&mut sv, &cv, "ioN", V::Int(40));
    pump(&mut sv, &cv);
    assert_eq!(cv.lines(), expect);
    assert_eq!(sv.frames[0].fi, 1, "frame id mirror fi");
    assert!(sv.frames[0].max_stack <= 4);
    assert!(
        sv.frames.len() == 1 && sv.ready.is_empty(),
        "scheduler phẳng, không đệ quy"
    );

    let ci = Ctx::new();
    let mut si = Scheduler::new();
    si.spawn(IFrame::new(
        1,
        vec![Stmt::Return(Expr::LogVal(
            "call".into(),
            Box::new(Expr::Add(
                Box::new(Expr::Await(
                    "ioN".into(),
                    Box::new(Expr::Effect("operand-eval".into(), Box::new(Expr::Int(0)))),
                )),
                Box::new(Expr::Effect("g".into(), Box::new(Expr::Int(2)))),
            )),
        ))],
        new_cells_i(&[]),
    ));
    pump(&mut si, &ci);
    wake(&mut si, &ci, "ioN", V::Int(40));
    pump(&mut si, &ci);
    assert_eq!(ci.lines(), expect);
    assert_eq!(si.frames[0].fi, 1, "frame id mirror fi");
    assert!(si.frames[0].max_work <= 8);
}

// ------------------------- BASELINE SO VỚI `vis` HIỆN TẠI ------------------

/// Chạy chương trình KHÔNG suspend bằng `vis run` hiện tại (default + `--vm --no-cache`)
/// và so stdout với prototype. File .vis nằm trong /tmp — KHÔNG thêm file vào repo.
#[test]
fn baseline_matches_current_vis() {
    use std::process::Command;

    let path = std::env::temp_dir().join(format!("lat0_baseline_{}.vis", std::process::id()));
    std::fs::write(&path, "let a = 40\nlet b = 2\nprint(a + b)\n").expect("ghi file .vis tạm");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/ps".into());
    let path_env = format!(
        "{}/.cargo/bin:{}",
        home,
        std::env::var("PATH").unwrap_or_default()
    );
    // **SỬA KÈM LÝ DO (verify mps/psvn)**: đường dẫn tuyệt đối `/home/ps/...` chỉ có trên
    // máy ps ⇒ ca này đỏ oan trên mps/psvn. Dò gốc repo từ CARGO_MANIFEST_DIR (có trên
    // mọi nền); trên Windows gọi shim `vis` (bash script) qua `bash`.
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let vis = repo.join("vis");

    for extra in [vec![], vec!["--vm", "--no-cache"]] {
        let mut cmd = if cfg!(windows) {
            let mut c = Command::new("bash");
            c.arg(&vis);
            c
        } else {
            Command::new(&vis)
        };
        cmd.arg("run");
        for e in &extra {
            cmd.arg(e);
        }
        cmd.arg(&path).env("PATH", &path_env);
        let out = cmd.output().expect("chạy vis run");
        assert!(
            out.status.success(),
            "vis run {:?} thất bại: {}",
            extra,
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&out.stdout), "42\n");
    }

    // Prototype: cùng phép tính, KHÔNG suspend ⇒ stdout phải giống hệt.
    let cv = Ctx::new();
    let mut sv = Scheduler::new();
    sv.spawn(VmFrame::new(
        1,
        vec![
            Instr::Push(V::Int(40)),
            Instr::Push(V::Int(2)),
            Instr::Add,
            Instr::Print,
            Instr::Ret,
        ],
        vec![],
        vec![],
        None,
    ));
    pump(&mut sv, &cv);
    assert_eq!(cv.stdout(), "42\n");

    let ci = Ctx::new();
    let mut si = Scheduler::new();
    si.spawn(IFrame::new(
        1,
        vec![Stmt::Return(Expr::Print(Box::new(Expr::Add(
            Box::new(Expr::Int(40)),
            Box::new(Expr::Int(2)),
        ))))],
        new_cells_i(&[]),
    ));
    pump(&mut si, &ci);
    assert_eq!(ci.stdout(), "42\n");

    let _ = std::fs::remove_file(&path);
}
