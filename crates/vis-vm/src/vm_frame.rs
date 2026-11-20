// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-267 / VD-251 — LÁT 1 (chỉ test/debug)**: frame-aware scheduler nối vào
//! **đường VM thật** (`Vm` + `Th` + `Frame` + `pc` hiện hành).
//!
//! VM đã có sẵn ngăn xếp khung **trên heap** (`Th { frames, st, iters, loop_ends }`),
//! nên suspend/resume là dừng vòng `Th::exec` ở đúng instruction `$await` và giữ
//! nguyên `Th`. Khác biệt duy nhất với runtime mặc định: `CallBuiltin "$await"` đi
//! qua [`call_builtin_frame`] — khi task chưa xong, trả `Abort::Pending`, driver lưu
//! `Th`, đăng ký wait và chạy frame khác. Bytecode/VERSION/cache-key/opcode **không đổi**.
//! Không reactor, không socket, không `unsafe`, không dependency mới.

use std::cell::RefCell;
use std::collections::HashMap;

use vis_bytecode::fee::{FeeEntry, FeeTable};
use vis_bytecode::Instr;
use vis_interp::builtin;
use vis_interp::frame::{FrameConfig, FrameOutput};
use vis_interp::interp::{Abort, Trap, BUDGET, R};
use vis_interp::rt::{self, Waiter};
use vis_interp::value::Value;
use vis_syntax::{Diagnostic, Span};

use crate::vm::{Frame, Th, Vm};

pub const SAFETY_FUEL_CODE: &vis_diag::Code = &vis_diag::Code {
    id: 501,
    short: "VS0501-FUEL",
    message_vi: "VM safety fuel cạn (hàng rào an toàn bytecode, không phải ngân sách ngữ nghĩa)",
    message_en: "VM safety fuel exhausted (bytecode safety barrier, not a semantic budget)",
    message_ja: "VM 安全フェイル枯渇（バイトコード安全装置、意味予算ではない）",
    severity: vis_diag::Severity::Error,
};

/// Lỗi của VM khi bytecode hỏng — **không** panic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    /// `pop` trên ngăn xếp rỗng.
    StackUnderflow,
    /// Đích nhảy ngoài `code`.
    BadJump(i32),
    /// Chỉ số const/hàm ngoài phạm vi.
    BadIdx(u32),
}

impl core::fmt::Display for VmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VmError::StackUnderflow => write!(f, "ngăn xếp rỗng"),
            VmError::BadJump(d) => write!(f, "đích nhảy ngoài code: {d}"),
            VmError::BadIdx(i) => write!(f, "chỉ số ngoài phạm vi: {i}"),
        }
    }
}

// ---------------------------------------------------------------------------
// VD-257 / D — cấu hình budget+quota cho `run_frame_for_test` (test-only). Dùng
// thread-local để KHÔNG đổi chữ ký `run_frame_for_test` / `FrameConfig`, không API
// công, không CLI. Mặc định rỗng ⇒ đường frame VM không charge (test f2 vẫn đúng).
// ---------------------------------------------------------------------------
#[derive(Default)]
struct FrameBudgetCfg {
    fee: Option<FeeTable>,
    budget: u64,
    quota: Option<u64>,
}

thread_local! {
    static FRAME_BUDGET_CFG: RefCell<FrameBudgetCfg> = RefCell::new(FrameBudgetCfg::default());
    /// Số lần quota yield của lần `run_frame_*` gần nhất (test/đo).
    static LAST_QUOTA_YIELDS: RefCell<u64> = const { RefCell::new(0) };
}

/// Số lần quota yield của lần chạy frame VM gần nhất.
#[doc(hidden)]
pub fn frame_quota_yields_for_test() -> u64 {
    LAST_QUOTA_YIELDS.with(|n| *n.borrow())
}

/// Bật instrumentation + (tuỳ chọn) quota cho đường frame VM. `quota = None` ⇒
/// chỉ charge event/budget, không yield.
#[doc(hidden)]
pub fn frame_budget_enable_for_test(fee: FeeTable, budget: u64, quota: Option<u64>) {
    FRAME_BUDGET_CFG.with(|c| {
        *c.borrow_mut() = FrameBudgetCfg {
            fee: Some(fee),
            budget,
            quota,
        }
    });
}

/// Tắt cấu hình — trả về mặc định (không charge/không quota).
#[doc(hidden)]
pub fn frame_budget_disable_for_test() {
    FRAME_BUDGET_CFG.with(|c| *c.borrow_mut() = FrameBudgetCfg::default());
}

// **ST-276 (VD-256)** — test-hook ngân sách của VM; tách sang tệp này để giữ R1 ≤ 1500 dòng
// cho `vm.rs`. Mặc định `budget_fee == None` ⇒ `budget_charge` là no-op.
impl<'a> Vm<'a> {
    /// Bật ngân sách ngữ nghĩa bằng bảng phụ S (internal test hook).
    pub fn budget_enable_for_test(&mut self, fee: FeeTable, limit: u64) {
        self.budget_fee = Some(fee);
        self.budget_remaining = limit as i64;
        self.budget_trace.clear();
        self.budget_hit = false;
    }

    /// Dãy sự kiện **có thứ tự** đã charge (so với trace của compiler/interp).
    pub fn budget_trace_for_test(&self) -> &[FeeEntry] {
        &self.budget_trace
    }

    // ---- VD-257/D — quota lượt (RIÊNG khỏi budget tổng, test-only) ----

    /// Bật quota với hạn `limit`. Không đụng `budget_*`.
    pub(crate) fn quota_setup_for_test(&mut self, limit: u64) {
        self.quota_active = true;
        self.quota_limit = limit as i64;
        self.quota_remaining = limit as i64;
        self.quota_hit = false;
        self.quota_yields = 0;
    }

    /// Reset quota **đầu mỗi turn** (không reset budget).
    pub(crate) fn quota_reset_for_test(&mut self) {
        if self.quota_active {
            self.quota_remaining = self.quota_limit;
            self.quota_hit = false;
        }
    }

    /// Trừ quota bởi CÙNG `w` với budget (một sự kiện, hai bộ đếm). Không trap.
    pub(crate) fn quota_charge(&mut self, w: i64) {
        if self.quota_active {
            self.quota_remaining -= w;
            if self.quota_remaining <= 0 {
                self.quota_hit = true;
            }
        }
    }

    /// Lấy (và xoá) cờ hết quota tại continuation point.
    pub(crate) fn quota_take_hit(&mut self) -> bool {
        if !self.quota_active || !self.quota_hit {
            return false;
        }
        self.quota_hit = false;
        self.quota_yields += 1;
        true
    }

    /// Số lần yield do quota (test/đo).
    #[doc(hidden)]
    pub fn quota_yields_for_test(&self) -> u64 {
        self.quota_yields
    }

    /// Charge **mọi** sự kiện tại `(hàm, pc)` theo thứ tự bảng phụ. D6: pc lưu sau charge ⇒
    /// resume không charge lại nhóm cũ.
    pub(crate) fn budget_charge(&mut self, fi: u32, pc: usize, span: Span) -> R<()> {
        // Copy ra Vec để kết thúc mượn immutable của `self.budget_fee` trước khi
        // gọi `self.quota_charge(w)` (cần mượn mutable). `FeeEntry` là Copy.
        let events: Vec<FeeEntry> = match self
            .budget_fee
            .as_ref()
            .and_then(|f| f.funcs.get(fi as usize))
        {
            Some(v) => v.iter().filter(|e| e.pc as usize == pc).copied().collect(),
            None => return Ok(()),
        };
        for e in events {
            let w = vis_bytecode::fee::standard_weight(e.kind) as i64;
            self.budget_trace.push(e);
            if w > self.budget_remaining {
                self.budget_hit = true;
                return Err(Abort::Trap(Trap::new(
                    BUDGET,
                    format!(
                        "vượt ngân sách ngữ nghĩa VD-256 ({}): cần {}, còn {}",
                        e.kind.name(),
                        w,
                        self.budget_remaining
                    ),
                    span,
                )));
            }
            self.budget_remaining -= w;
            // VD-257/D — MỘT sự kiện trừ CẢ HAI bộ đếm; quota KHÔNG trap.
            self.quota_charge(w);
        }
        Ok(())
    }
}

/// Hook nằm ngay tại nhánh `CallBuiltin` thật của VM (vm.rs). Khi `frame.enabled`,
/// `Abort::Pending` được **khôi phục nguyên vẹn** để instruction `$await` chạy lại
/// sau resume (lúc đó task đã cache ⇒ không lặp side-effect). Khi không bật frame,
/// hành vi đúng bytelength bằng `builtin::call` cũ.
pub(crate) fn call_builtin_frame(
    th: &mut Th<'_>,
    vm: &mut Vm<'_>,
    ten: &str,
    args: Vec<Value>,
    span: Span,
) -> vis_interp::R<Value> {
    if !vm.sched.borrow().frame.enabled {
        return builtin::call(vm, ten, args, span);
    }
    match builtin::call(vm, ten, args.clone(), span) {
        Ok(v) => Ok(v),
        Err(Abort::Pending(p)) => {
            // `CallBuiltin` đã pop `n` đối số và `exec` đã tăng `pc`. Khôi phục cả hai.
            for a in args {
                th.st.push(a);
            }
            if let Some(fr) = th.frames.last_mut() {
                fr.pc -= 1;
            }
            Err(Abort::Pending(p))
        }
        Err(e) => Err(e),
    }
}

/// Chạy bytecode thật qua frame scheduler — gọi từ integration test sau compile.
#[doc(hidden)]
pub fn run_frame_for_test<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    config: FrameConfig,
) -> FrameOutput {
    run_frame_with_budget(m, text, args, config, None).0
}

/// **VD-257 / stream A** — cùng đường frame, nhưng bật hook ngân sách VD-256 có sẵn và
/// trả kèm dãy sự kiện để integration test kiểm chứng **resume không charge lại** nhóm cũ.
/// `None` ⇒ hook tắt (mặc định), hành vi y hệt [`run_frame_for_test`].
#[doc(hidden)]
pub fn run_frame_for_test_with_budget<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    config: FrameConfig,
    fee: FeeTable,
    limit: u64,
) -> (FrameOutput, Vec<FeeEntry>) {
    run_frame_with_budget(m, text, args, config, Some((fee, limit)))
}

fn run_frame_with_budget<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    config: FrameConfig,
    budget: Option<(FeeTable, u64)>,
) -> (FrameOutput, Vec<FeeEntry>) {
    let mut vm: Vm<'m> = Vm::new(m, text);
    if let Err(e) = vis_bytecode::verify(m) {
        return (
            FrameOutput {
                stdout: String::new(),
                diagnostics: vec![Diagnostic {
                    code: &vis_diag::RUNTIME_CODES[0],
                    message: format!("bytecode hỏng: {e}"),
                    span: Span::new(0, 0),
                    line: 1,
                    col: 1,
                }],
                value: None,
                exit: 1,
                log: Vec::new(),
                snapshot: Default::default(),
            },
            Vec::new(),
        );
    }
    // VD-257/D — lấy cấu hình budget/quota từ thread-local hook nếu có.
    let cfg = FRAME_BUDGET_CFG.with(|c| {
        let c = c.borrow();
        (c.fee.clone(), c.budget, c.quota)
    });
    let budget = budget.or_else(|| cfg.0.map(|f| (f, cfg.1)));
    if let Some((fee, limit)) = budget {
        vm.budget_enable_for_test(fee, limit);
    }
    if let Some(q) = cfg.2 {
        vm.quota_setup_for_test(q);
    }
    LAST_QUOTA_YIELDS.with(|n| *n.borrow_mut() = 0);
    vm.init_globals();
    let argv = Value::List(args.iter().map(|s| Value::Str(s.clone())).collect());
    let main_th = match main_th(&vm, &argv) {
        Ok(t) => t,
        Err(Abort::Trap(t)) => {
            return (
                frame_output(&vm, None, Some(t)),
                vm.budget_trace_for_test().to_vec(),
            )
        }
        Err(_) => {
            return (
                frame_output(&vm, None, None),
                vm.budget_trace_for_test().to_vec(),
            )
        }
    };

    let mut frames: HashMap<u32, Th<'m>> = HashMap::new();
    frames.insert(0, main_th);
    vm.sched
        .borrow_mut()
        .frame_enable_for_test(config.fake_sources, config.events);
    vm.sched
        .borrow_mut()
        .frame_reactor_load_for_test(config.reactors, config.timer_real_wait);
    if let Some(b) = config.backend {
        vm.sched.borrow_mut().frame_install_backend_for_test(b);
    }

    let mut current: u32 = 0;
    let mut final_value: Option<Value> = None;
    let mut trap: Option<Trap> = None;
    let mut guard = 0usize;
    loop {
        guard += 1;
        if guard > 200_000 {
            trap = Some(Trap::new(
                vis_interp::interp::RT,
                "frame test: vòng lặp scheduler quá giới hạn",
                Span::new(0, 0),
            ));
            break;
        }
        vm.sched.borrow_mut().frame.current = current;
        // VD-257/D — reset hạn ngạch ĐẦU MỖI TURN (không đụng budget tổng).
        vm.quota_reset_for_test();
        let mut th = frames.remove(&current).expect("frame hiện tại");
        let outcome = th.exec(&mut vm);
        match outcome {
            Ok(v) => {
                if current == 0 {
                    final_value = Some(v);
                    // Không `break`: main xong ⇒ final join các task không ai await.
                } else {
                    let id = current;
                    if let Some(t) = vm.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(v));
                        t.trang_thai.set(rt::TrangThai::Xong);
                    }
                    vm.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
                }
            }
            Err(Abort::Trap(t)) => {
                if current == 0 {
                    trap = Some(t);
                    break;
                }
                let id = current;
                let (ten, span) = {
                    let s = vm.sched.borrow();
                    s.tasks
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| (t.ten.clone(), t.span))
                        .unwrap_or_default()
                };
                let wrapped = rt::wrap_task_error_for_test(&ten, t, span);
                if let Some(tk) = vm.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                    *tk.kq.borrow_mut() = Some(Err(wrapped.clone()));
                    tk.trang_thai.set(rt::TrangThai::Xong);
                }
                vm.sched.borrow_mut().frame_task_done(id, Err(wrapped));
            }
            Err(Abort::Ret(v)) => {
                if current == 0 {
                    final_value = Some(v);
                    break;
                }
                let id = current;
                if let Some(t) = vm.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                    *t.kq.borrow_mut() = Some(Ok(v));
                    t.trang_thai.set(rt::TrangThai::Xong);
                }
                vm.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
            }
            Err(Abort::Pending(p)) => {
                // Lát 2: reactor mang wait trực tiếp (`p.wait`); Lát 1: task.
                let w: Option<Waiter> = if let Some(w) = p.wait {
                    let gen = w.gen_of(vm.sched.borrow().frame.gen);
                    Some(Waiter {
                        frame: current,
                        wait: w,
                        gen,
                    })
                } else {
                    p.task
                        .as_ref()
                        .and_then(|t| vm.sched.borrow().frame_await(current, t))
                };
                if let Some(w) = w {
                    vm.sched.borrow_mut().frame_register(w);
                } else {
                    vm.sched.borrow_mut().frame.ready.push_back(current);
                }
                // `Th` được cất lại **nguyên vẹn** (pc/locals/st/captured/writeback).
                frames.insert(current, th);
            }
        }

        let need_join = true; // final join luôn được phép sau khi main xong
        let next = vm.sched.borrow_mut().frame_next();
        // Lát 3: lỗi selector là điểm dừng — KHÔNG final-join rồi chạy lại vô hạn.
        if vm.sched.borrow().frame.selector_error.is_some() {
            break;
        }
        let next = match next {
            Some(n) => Some(n),
            None if need_join => {
                let moi: Vec<u32> = {
                    let s = vm.sched.borrow();
                    s.tasks
                        .iter()
                        .filter(|t| t.trang_thai.get() == rt::TrangThai::Moi)
                        .map(|t| t.id)
                        .collect()
                };
                for id in moi {
                    if !vm.sched.borrow().frame.ready.contains(&id) {
                        vm.sched.borrow_mut().frame.ready.push_back(id);
                    }
                }
                vm.sched.borrow_mut().frame_next()
            }
            None => None,
        };
        match next {
            Some(n) => {
                current = n;
                if let std::collections::hash_map::Entry::Vacant(e) = frames.entry(n) {
                    let t = {
                        let s = vm.sched.borrow();
                        s.tasks.iter().find(|t| t.id == n).cloned()
                    };
                    if let Some(t) = t {
                        // VD-252/ST-268: task cần capture bị từ chối RÕ (không âm thầm dựng
                        // `captured = Vec::new()`/`writeback = None` — kết quả sai mà vẫn pass).
                        match task_th(&vm, t.than, t.args.clone()) {
                            Ok(th) => {
                                e.insert(th);
                            }
                            Err(Abort::Trap(t)) => {
                                trap = Some(t);
                                break;
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
            None => break,
        }
    }

    // Lát 3: lỗi selector là trap RÕ — không được hiểu nhầm là "không có event".
    if trap.is_none() {
        if let Some(e) = vm.sched.borrow_mut().frame_take_selector_error() {
            trap = Some(Trap::new(vis_interp::interp::RT, e, Span::new(0, 0)));
        }
    }
    frames.clear();
    vm.sched.borrow_mut().frame_drain_events();
    let final_join_err = vm.sched.borrow_mut().frame_final_join();
    // **ST-278 (VD-256)**: lỗi task ở final join phải **surfaced** — không exit 0. Ưu tiên trap
    // đã có (main lỗi); nếu chưa có thì dùng lỗi `VS0506` đã bọc sẵn trong `Task.kq`.
    if trap.is_none() {
        trap = final_join_err;
    }
    // Lát 2: reactor về baseline sau khi handle test thả.
    vm.sched.borrow_mut().frame_reactor_shutdown();
    LAST_QUOTA_YIELDS.with(|n| *n.borrow_mut() = vm.quota_yields_for_test());
    (
        frame_output(&vm, final_value, trap),
        vm.budget_trace_for_test().to_vec(),
    )
}

fn frame_output(vm: &Vm<'_>, value: Option<Value>, trap: Option<Trap>) -> FrameOutput {
    let mut log = vm.sched.borrow().frame.log.clone();
    log.extend(vm.sched.borrow().frame.reactor.log.clone());
    let stdout = vm.out.clone();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(vm.text)],
        None => Vec::new(),
    };
    let exit = if trap.is_some() { 1 } else { 0 };
    FrameOutput {
        stdout,
        diagnostics,
        value,
        exit,
        log,
        snapshot: vm.sched.borrow().diag_snapshot(),
    }
}

/// **VD-252/ST-268**: `true` khi hàm bytecode `fi` dùng biến bắt (`Instr::LoadCaptured`) —
/// tức là cần `captured`/`writeback` mà đường frame test không mô phỏng được. `#[doc(hidden)]`
/// để integration test có ca âm tường minh (xem report, mục coverage/explicit-rejection).
#[doc(hidden)]
pub fn task_requires_capture(m: &vis_bytecode::Module, fi: u32) -> bool {
    m.funcs
        .get(fi as usize)
        .map(|f| vis_bytecode::verify::captured_used(&f.code) > 0)
        .unwrap_or(false)
}

/// Dựng `Th` cho thân task async từ **chỉ số hàm bytecode thật**.
/// Từ chối rõ (Trap có span) nếu thân cần capture — không dựng rỗng rồi chạy sai.
fn task_th<'m>(vm: &Vm<'m>, fi: u32, args: Vec<Value>) -> vis_interp::R<Th<'m>> {
    let f = vm.func(fi)?;
    if task_requires_capture(vm.m, fi) {
        return Err(Abort::Trap(Trap::new(
            vis_interp::interp::RT,
            format!(
                "frame test: task `{}` cần capture biến tự do chưa được đường frame test hỗ trợ",
                f.name
            ),
            Span::new(0, 0),
        )));
    }
    let mut locals = vec![Value::Unit; f.nlocals.max(1) as usize];
    for (i, a) in args.into_iter().enumerate() {
        if i < locals.len() {
            locals[i] = a;
        }
    }
    Ok(Th::new(vm.m, fi, locals, Vec::new()))
}

/// Dựng `Th` cho `main` — cùng logic chọn entry với `Vm::chay_chuong_trinh` (VD-156).
fn main_th<'m>(vm: &Vm<'m>, argv: &Value) -> vis_interp::R<Th<'m>> {
    let entry = vm.m.entry;
    let vo = vm.func(entry)?;
    let fi = match vo.code.as_slice() {
        [Instr::Call(fi, 0), Instr::Halt] if vo.nparams == 0 => {
            let g = vm.func(*fi)?;
            if g.name == "main" && g.nparams > 0 {
                *fi
            } else {
                entry
            }
        }
        _ => entry,
    };
    let f = vm.func(fi)?;
    let mut locals = vec![Value::Unit; f.nlocals.max(1) as usize];
    for (pos, v) in &vm.caps {
        if let Some(l) = locals.get_mut(*pos as usize) {
            *l = v.clone();
        }
    }
    if vm.caps.is_empty() && f.nparams > 0 {
        if let Some(l) = locals.first_mut() {
            *l = argv.clone();
        }
    }
    Ok(Th::new(vm.m, fi, locals, Vec::new()))
}

/// Giữ `Frame` được dùng (tránh cảnh báo dead_code khi module con không dựng trực tiếp).
#[allow(dead_code)]
fn _frame_touch(f: &Frame) -> usize {
    f.pc
}
