// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-267 / VD-251 — LÁT 1 (chỉ test/debug)**: frame-aware scheduler nối vào
//! **đường VM thật** (`Vm` + `Th` + `Frame` + `pc` hiện hành).
//!
//! VM đã có sẵn ngăn xếp khung **trên heap** (`Th { frames, st, iters }`, mỗi `Frame` giữ `loop_ends`),
//! nên suspend/resume là dừng vòng `Th::exec` ở đúng instruction `$await` và giữ
//! nguyên `Th`. Khác biệt duy nhất với runtime mặc định: `CallBuiltin "$await"` đi
//! qua [`call_builtin_frame`] — khi task chưa xong, trả `Abort::Pending`, driver lưu
//! `Th`, đăng ký wait và chạy frame khác. Bytecode/VERSION/cache-key/opcode **không đổi**.
//! Không reactor, không socket, không `unsafe`, không dependency mới.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vis_bytecode::fee::{FeeEntry, FeeTable};
use vis_bytecode::Instr;
use vis_core::frame::{FrameConfig, FrameOutput};
use vis_core::trap::{Abort, Trap, R};
use vis_core::value::{Value, VmFunc};
use vis_runtime::builtin;
use vis_runtime::builtin::Host;
use vis_runtime::codes::RT;
use vis_runtime::rt::{self, Waiter};
use vis_syntax::{Diagnostic, Span};

use crate::vm::{Frame, Th, Vm, MAX_FRAMES};

mod budget;
pub use budget::*;
use budget::{FRAME_BUDGET_CFG, LAST_QUOTA_YIELDS};

pub const SAFETY_FUEL_CODE: &vis_diag::Code = &vis_diag::Code {
    id: 501,
    short: "VS0501-FUEL",
    message_vi: "VM safety fuel cạn (hàng rào an toàn bytecode, không phải ngân sách ngữ nghĩa)",
    message_en: "VM safety fuel exhausted (bytecode safety barrier, not a semantic budget)",
    message_ja: "VM 安全フェイル枯渇（バイトコード安全装置、意味予算ではない）",
    severity: vis_diag::Severity::Error,
};

/// **ST-311a (#195)**: hàng rào thời gian wall-clock monotonic của VM.
pub const DEADLINE_CODE: &vis_diag::Code = &vis_diag::Code {
    id: 501,
    short: "VS0501-TIME",
    message_vi: "VM hết thời gian chạy tối đa (deadline wall-clock)",
    message_en: "VM execution deadline exceeded (wall-clock safety barrier)",
    message_ja: "VM 実行期限超過（ウォールクロック安全装置）",
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

/// **VD-258**: vòng `list.map/filter/each` của VM — trạng thái sống trên `Th` để
/// callback `await` giữa vòng không mất `idx`/`acc` (trước đây `host.call` chạy callback
/// trong `Th` lồng nhau, suspend làm mất ngăn xếp ⇒ `VS0501: ngăn xếp rỗng`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodKind {
    Map,
    Filter,
    Each,
}

pub(crate) struct MethodLoop {
    pub(crate) kind: MethodKind,
    pub(crate) f: Value,
    pub(crate) items: Vec<Value>,
    pub(crate) idx: usize,
    pub(crate) acc: Vec<Value>,
    /// Item đang được callback xử lý (để `filter` giữ lại đúng phần tử).
    pub(crate) item: Option<Value>,
    /// Đã đẩy khung callback và đang chờ giá trị trả trên `st`.
    pub(crate) awaiting: bool,
    /// Số khung tại thời điểm bắt đầu phương thức — callback xong khi `frames.len()` quay lại.
    pub(crate) base_frames: usize,
    pub(crate) span: Span,
}

impl<'m> Th<'m> {
    /// **VD-258**: đẩy khung gọi một `Value::Vm` vào **chính `Th`** này (suspend/resume
    /// được), dùng chung cho `CallValue` và callback của `list.map/filter/each`.
    pub(crate) fn push_vm_frame(
        &mut self,
        vm: &mut Vm<'_>,
        vf: &Rc<VmFunc>,
        args: Vec<Value>,
        span: Span,
    ) -> R<()> {
        let fi = vf.idx;
        let (nlocals, mirror) = {
            let fc = vm.func(fi)?;
            let nlocals = fc.nlocals.max(1) as usize;
            let mut mirror: Vec<(u16, usize)> = Vec::new();
            let mut k = 0;
            while k + 1 < fc.code.len() {
                if let (Instr::LoadCaptured(i), Instr::Assign(slot)) = (fc.code[k], fc.code[k + 1])
                {
                    mirror.push((slot, i as usize));
                    k += 2;
                } else {
                    break;
                }
            }
            (nlocals, mirror)
        };
        if self.frames.len() + vm.depth + 1 >= MAX_FRAMES {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("gọi hàm quá sâu (>{MAX_FRAMES} khung) — đệ quy không có điểm dừng?"),
                span,
            )));
        }
        let ptr = Rc::as_ptr(vf) as usize;
        let cells = match vm.closure_cells.get(&ptr) {
            Some(c) => c.clone(),
            None => {
                let c = Rc::new(RefCell::new(vf.captured.clone()));
                vm.closure_cells.insert(ptr, c.clone());
                c
            }
        };
        let captured = cells.borrow().clone();
        let mut locals = vec![Value::Unit; nlocals];
        for (i, a) in args.into_iter().enumerate() {
            if i < locals.len() {
                locals[i] = a;
            }
        }
        let cu = std::mem::take(&mut self.st);
        if let Some(fr) = self.frames.last_mut() {
            fr.st_cu = cu;
        }
        let iter_base = self.iters.len();
        self.frames.push(Frame {
            fi,
            pc: 0,
            locals,
            captured,
            st_cu: Vec::new(),
            writeback: Some((cells, mirror)),
            iter_base,
            loop_ends: HashMap::new(),
        });
        Ok(())
    }

    /// **VD-258**: chạy tiếp/khởi động vòng callback `list.map/filter/each`. Được gọi ở
    /// đầu mỗi vòng `exec`: nếu khung callback vừa trả (số khung quay lại `base_frames`)
    /// thì tiêu thụ giá trị trên `st`; nếu còn phần tử thì đẩy khung callback kế tiếp.
    pub(crate) fn method_poll(&mut self, vm: &mut Vm<'_>) -> R<()> {
        loop {
            let Some(mut m) = self.methods.pop() else {
                return Ok(());
            };
            let span = m.span;
            if m.awaiting {
                // Khung callback còn sống (kể cả đang treo ở `$await`) ⇒ để `exec` chạy tiếp.
                if self.frames.len() > m.base_frames {
                    self.methods.push(m);
                    return Ok(());
                }
                let v = self
                    .st
                    .pop()
                    .ok_or_else(|| vm.hong(VmError::StackUnderflow, span))?;
                m.awaiting = false;
                match m.kind {
                    MethodKind::Map => m.acc.push(v),
                    MethodKind::Filter => {
                        if v.truthy(span)? {
                            if let Some(it) = &m.item {
                                m.acc.push(it.clone());
                            }
                        }
                    }
                    MethodKind::Each => {}
                }
                m.item = None;
            }
            if m.idx >= m.items.len() {
                let out = match m.kind {
                    MethodKind::Each => Value::Unit,
                    MethodKind::Map | MethodKind::Filter => Value::list(std::mem::take(&mut m.acc)),
                };
                self.st.push(out);
                continue;
            }
            let item = m.items[m.idx].clone();
            m.idx += 1;
            m.awaiting = true;
            m.item = Some(item.clone());
            match m.f.clone() {
                Value::Vm(vf) => {
                    self.methods.push(m);
                    self.push_vm_frame(vm, &vf, vec![item], span)?;
                    return Ok(());
                }
                Value::Builtin(n) => {
                    let v = match builtin::call(vm, n, vec![item], span) {
                        Ok(v) => v,
                        // Builtin không có continuation trong `Th` ⇒ không thể treo giữa vòng.
                        Err(Abort::Pending(_)) => {
                            return Err(Abort::Trap(Trap::new(
                                RT,
                                "frame test: callback dạng builtin không resume được trong `map`/`filter`/`each`",
                                span,
                            )))
                        }
                        Err(e) => return Err(e),
                    };
                    self.st.push(v);
                    self.methods.push(m);
                }
                other => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!("`{}` không phải hàm", other.show()),
                        span,
                    )))
                }
            }
        }
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
) -> vis_core::R<Value> {
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

/// **VS3-2a** — hook `CallMethod` của VM khi frame bật: nếu method trả `Abort::Pending`, khôi
/// phục **cả receiver + đối số** và `pc` để instruction `CallMethod` chạy lại sau resume. Trước
/// đây `CallMethod` pop operands rồi để `Pending` lan ra ⇒ resume chạy instruction kế với ngăn
/// xếp thiếu; wait-set `chan(0)` là đường đầu tiên cần resume method thật.
pub(crate) fn call_method_frame(
    th: &mut Th<'_>,
    vm: &mut Vm<'_>,
    recv: Value,
    args: Vec<Value>,
    ten: &str,
    span: Span,
) -> vis_core::R<Value> {
    match builtin::method(vm, recv.clone(), ten, args.clone(), span) {
        Ok(v) => Ok(v),
        Err(Abort::Pending(p)) => {
            // Khôi phục đúng thứ tự ngăn xếp trước `CallMethod`: receiver rồi đối số.
            th.st.push(recv);
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

/// **VD-283/ST-301 (#192)** — test-only: frame + `Parallel(workers)` với `FrameConfig` đầy đủ
/// (fake_sources/events/cancel trigger) để khoá cancel worker qua trigger frame.
#[doc(hidden)]
pub fn run_frame_workers_for_test<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    workers: usize,
    config: FrameConfig,
) -> FrameOutput {
    let mut vm = Vm::new(m, text);
    crate::vm::worker::cau_hinh(&mut vm, m, text, workers.max(2), None, None);
    run_frame_vm_driver(&mut vm, m, text, args, config, None).0
}

/// **VS3-1d** — chạy chương trình **có async** bằng frame scheduler thật trên VM, cùng host/
/// args/caps với `vis_vm::run_with_full_hosts_caps`. Caller chỉ chọn khi HIR có `async fn`.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_async_with_full_hosts_caps<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    ngoai: Option<&'m mut dyn vis_core::host::Ngoai>,
    fs: Option<&'m mut dyn vis_core::host::FsHost>,
    net: Option<&'m mut dyn vis_core::host::NetHost>,
    db: Option<&'m mut dyn vis_core::host::DbHost>,
    caps: &[(u32, Value)],
) -> vis_core::RunOutput {
    run_async_with_full_hosts_caps_backend(m, text, args, ngoai, fs, net, db, caps, None)
}

/// **VS3-1d/W1f** — như [`run_async_with_full_hosts_caps`] nhưng gắn reactor backend production.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_async_with_full_hosts_caps_backend<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    ngoai: Option<&'m mut dyn vis_core::host::Ngoai>,
    fs: Option<&'m mut dyn vis_core::host::FsHost>,
    net: Option<&'m mut dyn vis_core::host::NetHost>,
    db: Option<&'m mut dyn vis_core::host::DbHost>,
    caps: &[(u32, Value)],
    backend: Option<std::rc::Rc<std::cell::RefCell<dyn vis_core::reactor::FrameReactorBackend>>>,
) -> vis_core::RunOutput {
    let mut vm = Vm::new(m, text);
    if let Some(n) = ngoai {
        vm.set_ngoai(n);
    }
    if let Some(f) = fs {
        vm.set_fs(f);
    }
    if let Some(g) = net {
        vm.set_net(g);
    }
    if let Some(d) = db {
        vm.set_db(d);
    }
    if !caps.is_empty() {
        vm.caps = caps.to_vec();
    }
    let (fo, _) = run_frame_vm_driver(
        &mut vm,
        m,
        text,
        args,
        FrameConfig {
            backend,
            ..Default::default()
        },
        None,
    );
    vis_core::RunOutput {
        stdout: fo.stdout,
        diagnostics: fo.diagnostics,
        value: fo.value,
        exit: fo.exit,
        steps: vm.steps,
    }
}

/// **ST-292 (P4)** — frame + evaluator **`Parallel(workers)`** cho VM (main frame spawn ⇒ pool).
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_async_workers_with_full_hosts_caps<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    ngoai: Option<&'m mut dyn vis_core::host::Ngoai>,
    fs: Option<&'m mut dyn vis_core::host::FsHost>,
    net: Option<&'m mut dyn vis_core::host::NetHost>,
    db: Option<&'m mut dyn vis_core::host::DbHost>,
    caps: &[(u32, Value)],
    workers: usize,
    budget: Option<std::sync::Arc<vis_runtime::rt::fee::SharedBudget>>,
    backend: Option<std::rc::Rc<std::cell::RefCell<dyn vis_core::reactor::FrameReactorBackend>>>,
) -> vis_core::RunOutput {
    let mut vm = Vm::new(m, text);
    crate::vm::worker::cau_hinh(&mut vm, m, text, workers.max(2), budget, None);
    if let Some(n) = ngoai {
        vm.set_ngoai(n);
    }
    if let Some(f) = fs {
        vm.set_fs(f);
    }
    if let Some(g) = net {
        vm.set_net(g);
    }
    if let Some(d) = db {
        vm.set_db(d);
    }
    if !caps.is_empty() {
        vm.caps = caps.to_vec();
    }
    let (fo, _) = run_frame_vm_driver(
        &mut vm,
        m,
        text,
        args,
        FrameConfig {
            backend,
            ..Default::default()
        },
        None,
    );
    vis_core::RunOutput {
        stdout: fo.stdout,
        diagnostics: fo.diagnostics,
        value: fo.value,
        exit: fo.exit,
        steps: vm.steps,
    }
}

fn run_frame_with_budget<'m>(
    m: &'m vis_bytecode::Module,
    text: &'m str,
    args: &[String],
    config: FrameConfig,
    budget: Option<(FeeTable, u64)>,
) -> (FrameOutput, Vec<FeeEntry>) {
    let mut vm: Vm<'m> = Vm::new(m, text);
    run_frame_vm_driver(&mut vm, m, text, args, config, budget)
}

/// **VS3-1d** — driver frame dùng chung test/runtime; `vm` đã có host/caps do caller gắn.
fn run_frame_vm_driver<'m>(
    vm: &mut Vm<'m>,
    m: &'m vis_bytecode::Module,
    _text: &'m str,
    args: &[String],
    config: FrameConfig,
    budget: Option<(FeeTable, u64)>,
) -> (FrameOutput, Vec<FeeEntry>) {
    // **ST-315a (VD-291)**: bộ đếm ngân sách theo lần chạy; flush TLS một lần ở mọi lối ra.
    vm.budget_used = 0;
    if let Err(e) = vis_bytecode::verify(m) {
        vm.ket_thuc_ngan_sach();
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
    } else {
        // **ST-290 (VD-273)** — không có hook test ⇒ dùng bảng phí + ngân sách phiên của CLI.
        crate::vm_frame::bat_dau_ngan_sach_vm(vm);
    }
    if let Some(q) = cfg.2 {
        vm.quota_setup_for_test(q);
    }
    LAST_QUOTA_YIELDS.with(|n| *n.borrow_mut() = 0);
    vm.init_globals();
    let argv = Value::list(args.iter().map(|s| Value::Str(s.clone())).collect());
    let main_th = match main_th(vm, &argv) {
        Ok(t) => t,
        Err(Abort::Trap(t)) => {
            vm.ket_thuc_ngan_sach();
            return (
                frame_output(vm, None, Some(t), Default::default()),
                vm.budget_trace_for_test().to_vec(),
            );
        }
        Err(_) => {
            vm.ket_thuc_ngan_sach();
            return (
                frame_output(vm, None, None, Default::default()),
                vm.budget_trace_for_test().to_vec(),
            );
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
    let max_turns = vis_runtime::rt::frame_max_turns_for_test().unwrap_or(200_000);
    loop {
        guard += 1;
        if guard > max_turns {
            trap = Some(Trap::new(
                vis_runtime::codes::RT,
                format!(
                    "vòng lặp scheduler quá giới hạn ({max_turns} turn) — kiểm tra await vòng tròn/deadlock"
                ),
                Span::new(0, 0),
            ));
            break;
        }
        vm.sched.borrow_mut().frame.current = current;
        // **ST-292 (P4b)**: chuyển cancel của scheduler tới handle worker (idempotent).
        let huy: Vec<u32> = vm.sched.borrow().frame.cancelled.iter().copied().collect();
        for id in huy {
            vm.worker_cancel(id);
        }
        // **VD-258**: task đang chạy mang `DangChay` ⇒ `await` chính nó bị phát hiện.
        if current != 0 {
            if let Some(t) = vm
                .sched
                .borrow()
                .tasks
                .iter()
                .find(|t| t.id == current)
                .cloned()
            {
                t.trang_thai.set(vis_core::value::TrangThai::DangChay);
            }
        }
        // VD-257/D — reset hạn ngạch ĐẦU MỖI TURN (không đụng budget tổng).
        vm.quota_reset_for_test();
        let mut th = frames.remove(&current).expect("frame hiện tại");
        let outcome = th.exec(vm);
        match outcome {
            Ok(v) => {
                if current == 0 {
                    final_value = Some(v);
                    // Không `break`: main xong ⇒ final join các task không ai await.
                } else {
                    let id = current;
                    if let Some(t) = vm.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(v));
                        t.trang_thai.set(vis_core::value::TrangThai::Xong);
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
                    tk.trang_thai.set(vis_core::value::TrangThai::Xong);
                }
                vm.sched.borrow_mut().frame_task_done(id, Err(wrapped));
            }
            Err(Abort::Ret(v)) => {
                if current == 0 {
                    // `return`/`?` ở `main` vẫn phải final-join các task còn lại (không bỏ).
                    final_value = Some(v);
                } else {
                    let id = current;
                    if let Some(t) = vm.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(v));
                        t.trang_thai.set(vis_core::value::TrangThai::Xong);
                    }
                    vm.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
                }
            }
            Err(Abort::Pending(p)) => {
                let quota_yield = p.task.is_none() && p.wait.is_none();
                // **VD-258**: `await` chính tác vụ đang chạy ⇒ deadlock rõ, không exit 0.
                if let Some(t) = &p.task {
                    if t.id == current || t.trang_thai.get() == vis_core::value::TrangThai::DangChay
                    {
                        trap = Some(Trap::new(
                            vis_runtime::codes::DEADLOCK,
                            "deadlock: tác vụ đang chờ chính nó (await chính tác vụ đang chạy)",
                            p.span,
                        ));
                        break;
                    }
                }
                if current != 0 {
                    if let Some(t) = vm
                        .sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == current)
                        .cloned()
                    {
                        t.trang_thai.set(vis_core::value::TrangThai::Pending);
                    }
                }
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
                if quota_yield {
                    // Yield quota là tiến độ hợp lệ, không phải scheduler quay vô ích.
                    guard = guard.saturating_sub(1);
                }
                // `Th` được cất lại **nguyên vẹn** (pc/locals/st/captured/writeback).
                frames.insert(current, th);
            }
        }

        let need_join = true; // final join luôn được phép sau khi main xong
        let mut next = vm.sched.borrow_mut().frame_next();
        // Lát 3: lỗi selector là điểm dừng — KHÔNG final-join rồi chạy lại vô hạn.
        if vm.sched.borrow().frame.selector_error.is_some() {
            break;
        }
        // **ST-292 (P4)**: pool còn in-flight ⇒ pump 50 ms/lần rồi thử frame_next lại.
        if next.is_none() && vm.worker_in_flight() {
            for _ in 0..400 {
                match vm.worker_pump(true) {
                    Ok(_) => {}
                    Err(Abort::Trap(t)) => {
                        trap = Some(t);
                        break;
                    }
                    Err(_) => {}
                }
                next = vm.sched.borrow_mut().frame_next();
                if next.is_some() || !vm.worker_in_flight() {
                    break;
                }
            }
        }
        let next = match next {
            Some(n) => Some(n),
            None if need_join => {
                // **VD-258**: chỉ join task **Moi** (chưa chạy). Task `Pending` đang chờ
                // event KHÔNG được đẩy lại ready (quay vô ích tới guard). Nguồn giả
                // (`external`) cũng bị loại — thân thật của nó KHÔNG được chạy.
                let moi: Vec<u32> = {
                    let s = vm.sched.borrow();
                    s.tasks
                        .iter()
                        .filter(|t| t.trang_thai.get() == vis_core::value::TrangThai::Moi)
                        .filter(|t| !s.frame.external.contains_key(&t.id))
                        .filter(|t| !vm.worker_is_task(t.id))
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
        // **VD-260**: bỏ `Th` của task đã `Xong`/`Huy` ngay tại safe point kế tiếp — không
        // giữ frame + captured/locals tới cuối chương trình. `next` luôn còn sống nên giữ.
        let purged = purge_terminal_frames(&mut frames, &vm.sched.borrow(), next);
        if !purged.is_empty() {
            let mut s = vm.sched.borrow_mut();
            for id in &purged {
                s.frame.log.push(format!("purge:{}", id));
            }
            s.frame.live_frames = frames.len();
        }
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
                        match task_th(vm, t.than, t.args.clone()) {
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
            trap = Some(Trap::new(vis_runtime::codes::RT, e, Span::new(0, 0)));
        }
    }
    // **VD-259/VD-260**: bỏ `Th` của task đã `Xong`/`Huy` khỏi map trước khi đo `live_frames`
    // — frame terminal/huỷ không phải trạng thái treo cần giữ (đã bỏ ngay trong vòng lặp).
    let purged = purge_terminal_frames(&mut frames, &vm.sched.borrow(), None);
    if !purged.is_empty() {
        let mut s = vm.sched.borrow_mut();
        for id in &purged {
            // Hậu tố `-end`: frame chỉ được dọn ở cuối (không kịp safe point).
            s.frame.log.push(format!("purge-end:{}", id));
        }
    }
    vm.sched.borrow_mut().frame.live_frames = frames.len();
    frames.clear();
    vm.sched.borrow_mut().frame_drain_events();
    // **ST-292 (P4)**: chờ worker pool, cache task chưa await, flush output theo spawn order.
    if trap.is_none() {
        if let Err(Abort::Trap(t)) = vm.worker_final_join() {
            trap = Some(t);
        }
    }
    let final_join_err = vm.sched.borrow_mut().frame_final_join();
    // **ST-278 (VD-256)**: lỗi task ở final join phải **surfaced** — không exit 0. Ưu tiên trap
    // đã có (main lỗi); nếu chưa có thì dùng lỗi `VS0506` đã bọc sẵn trong `Task.kq`.
    if trap.is_none() {
        trap = final_join_err;
    }
    // **VD-258**: hết frame sẵn sàng mà còn waiter/task chưa terminal/đăng ký reactor
    // ⇒ deadlock thật, không được trả exit 0.
    if trap.is_none() && !vm.worker_in_flight() {
        trap = vm.sched.borrow().frame_stuck_trap();
    }
    // **VD-259**: snapshot **trước** shutdown — nếu không, regs/timers luôn 0.
    let snapshot = vm.sched.borrow().diag_snapshot();
    // Lát 2: reactor về baseline sau khi handle test thả.
    vm.sched.borrow_mut().frame_reactor_shutdown();
    LAST_QUOTA_YIELDS.with(|n| *n.borrow_mut() = vm.quota_yields_for_test());
    vm.ket_thuc_ngan_sach();
    (
        frame_output(vm, final_value, trap, snapshot),
        vm.budget_trace_for_test().to_vec(),
    )
}

/// **VD-260**: bỏ `Th` của task đã terminal (`Xong`/`Huy`) hoặc không còn task tương ứng.
/// `keep` là frame vừa được chọn chạy — không bao giờ bỏ. Main (id 0) luôn giữ. Trả danh
/// sách id đã bỏ để driver ghi log tất định (hai engine cùng chuỗi).
fn purge_terminal_frames<V>(
    frames: &mut HashMap<u32, V>,
    s: &rt::Scheduler,
    keep: Option<u32>,
) -> Vec<u32> {
    let dead: Vec<u32> = frames
        .keys()
        .copied()
        .filter(|id| *id != 0 && Some(*id) != keep)
        .filter(|id| {
            s.tasks
                .iter()
                .find(|t| t.id == *id)
                .map(|t| {
                    matches!(
                        t.trang_thai.get(),
                        vis_core::value::TrangThai::Xong | vis_core::value::TrangThai::Huy
                    )
                })
                .unwrap_or(true)
        })
        .collect();
    for id in &dead {
        frames.remove(id);
    }
    dead
}

fn frame_output(
    vm: &Vm<'_>,
    value: Option<Value>,
    trap: Option<Trap>,
    snapshot: vis_core::frame::DiagSnapshot,
) -> FrameOutput {
    let mut log = vm.sched.borrow().frame.log.clone();
    log.extend(vm.sched.borrow().frame.reactor.log.clone());
    let stdout = vm.out.clone();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(vm.text)],
        None => Vec::new(),
    };
    // **VD-258**: exit theo giá trị cuối của `main` (`int`/`Result.Err`), không luôn 0.
    let exit = match (&trap, &value) {
        (Some(_), _) => 1,
        (None, Some(v)) => vis_core::value::exit_code(v),
        (None, None) => 0,
    };
    FrameOutput {
        stdout,
        diagnostics,
        value,
        exit,
        log,
        snapshot,
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
fn task_th<'m>(vm: &Vm<'m>, fi: u32, args: Vec<Value>) -> vis_core::R<Th<'m>> {
    let f = vm.func(fi)?;
    if task_requires_capture(vm.m, fi) {
        return Err(Abort::Trap(Trap::new(
            vis_runtime::codes::RT,
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
fn main_th<'m>(vm: &Vm<'m>, argv: &Value) -> vis_core::R<Th<'m>> {
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
