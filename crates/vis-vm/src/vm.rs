// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vòng lặp **fetch–decode–execute** trên [`Module`] (ST-20).
//!
//! Ngăn xếp giá trị **động** (`Vec<Value>`), không dựa vào `max_stack` (VD-54 (2)); ngăn xếp
//! khung tách riêng để `return` lồng nhau sạch; iterator của `for` nằm ở `iters` (không lẫn với
//! ngăn xếp giá trị).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vis_bytecode::fee::{FeeEntry, FeeTable};
use vis_bytecode::{Func, Instr, Module, GLOBAL_SLOT_BASE};
use vis_interp::builtin::Host;
use vis_interp::interp::{Abort, Trap, IDX, R, RT};
use vis_interp::value::{self, Value, VmFunc};
use vis_interp::RunOutput;
use vis_syntax::Span;
use vis_types::hir::LitKind;

use crate::vm_frame::MethodLoop;

/// **VD-267** — cặp `(tên enum, tên biến thể)` đã nội bộ hoá cho cache `MakeEnum`.
type EnumTag = (value::Name, value::Name);

/// Chạy một module VBC1 (đường tắt tiện dụng).
pub fn run(m: &Module, text: &str, args: &[String]) -> RunOutput {
    Vm::new(m, text).run(args)
}

/// Cap số khung gọi lồng nhau (VD-49).
pub const MAX_FRAMES: usize = 2_000;
/// **ST-311a (#195)**: cap số bộ lặp `for` lồng nhau đồng thời trên ngăn xếp `iters`, ngăn
/// bytecode đột biến nhảy ngược về `IterInit` làm phình bộ nhớ vô hạn.
pub const MAX_ITER_DEPTH: usize = 1_024;
/// **VD-256 D8/D9**, **VS3-1a** — fuel an toàn **chỉ của VM**: không phải ngân sách ngữ nghĩa,
/// không che verifier (chạy trước ở [`Vm::run`]). Một nguồn với interp/compiler:
/// [`vis_bytecode::fee::VM_SAFETY_FUEL`] (100M) — phải lớn hơn trần ngân sách định cỡ × tỷ lệ
/// bước/lệnh trần, nếu không fuel sẽ chặn chương trình hợp lệ trước ngân sách (VS3-1a).
pub const SAFETY_FUEL: usize = vis_bytecode::fee::VM_SAFETY_FUEL as usize;
pub const MAX_STEPS: usize = SAFETY_FUEL;

pub use crate::vm_frame::{VmError, DEADLINE_CODE, SAFETY_FUEL_CODE};
mod exec;
mod host;
pub use host::{run_with_fs, run_with_full_hosts_caps, run_with_hosts, run_with_hosts_caps};
mod vm_helpers;
pub(crate) mod worker;
use vm_helpers::lit_kind;

/// **ST-238 (VD-242, ca 8)**: ô chia sẻ của closure và cặp `(ô cục bộ mirror, chỉ số ô bắt)`.
type ClosureCells = Rc<RefCell<Vec<Value>>>;
type CapturedMirror = Vec<(u16, usize)>;

pub(crate) struct Frame {
    pub(crate) fi: u32,
    pub(crate) pc: usize,
    pub(crate) locals: Vec<Value>,
    pub(crate) captured: Vec<Value>,
    /// Ngăn xếp **của khung gọi** được cất khi gọi hàm (mỗi khung một ngăn xếp riêng).
    pub(crate) st_cu: Vec<Value>,
    /// **ST-238 (VD-242, ca 8)**: nếu khung là một lần gọi closure, chứa ô chia sẻ của closure
    /// và cặp `(ô cục bộ, chỉ số biến bắt)` để ghi ngược giá trị khi khung kết thúc — giúp closure
    /// ghi biến bắt `var` giữ trạng thái **giữa các lần gọi** mà vẫn không đụng biến ngoài.
    pub(crate) writeback: Option<(ClosureCells, CapturedMirror)>,
    /// **ST-311a (#195)**: độ sâu `Th::iters` lúc vào khung để `pop_frame` dọn sạch khi `return`/`?`
    /// sớm từ bên trong vòng `for`.
    pub(crate) iter_base: usize,
    /// **ST-311a (#195)**: ánh xạ `end_pc -> iter_base` trong phạm vi khung này — tra cứu O(1)
    /// trong `dat_pc`, khử trùng lặp tự nhiên và tự giải phóng khi khung pop.
    pub(crate) loop_ends: HashMap<usize, usize>,
}

/// Máy ảo: giữ module + văn bản nguồn + `stdout` + số bước.
pub struct Vm<'a> {
    pub(crate) m: &'a Module,
    pub(crate) text: &'a str,
    pub(crate) out: String,
    pub(crate) steps: usize,
    /// **ST-311a (#195)**: fuel an toàn có thể đặt cho phép đo/fuzz (mặc định [`SAFETY_FUEL`]).
    pub(crate) safety_fuel: usize,
    /// **ST-311a (#195)**: deadline wall-clock monotonic tuỳ chọn (mặc định `None`).
    pub(crate) deadline: Option<std::time::Instant>,
    pub(crate) depth: usize,
    /// Độ sâu ngăn xếp giá trị **lớn nhất đã dùng** (bất biến của fuzz ST-21).
    pub(crate) dinh: usize,
    /// Host hệ thống tệp (ST-33); `None` ⇒ `VS0604` — VM **không** biết `std::fs`.
    pub(crate) fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    /// **ST-63 (VD-105)**: host mạng; `None` ⇒ `VS0604` — VM **không** biết `std::net`.
    pub(crate) net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
    /// **ST-89 (VD-110)**: host **chương trình ngoài** — VM **không** gọi `std::process`, chỉ qua
    /// trait của `vis-interp`; `None` ⇒ `exec.*` ⇒ `VS0604`.
    pub(crate) ngoai: Option<&'a mut dyn vis_interp::builtin::Ngoai>,
    /// **VD-241 (Gate 1)**: host **cơ sở dữ liệu** — VM **không** biết backend native; `None` ⇒
    /// `std.sqlite.*` ⇒ `VS0604`.
    pub(crate) db: Option<&'a mut dyn vis_interp::builtin::DbHost>,
    /// **ST-60**: hàng đợi tác vụ `async` — cùng runtime với interp (`vis_interp::rt`).
    pub(crate) sched: std::cell::RefCell<vis_interp::rt::Scheduler>,
    /// **VS3-3b/W5 (P3)**: evaluator worker VM (CPU-only, opt-in); mặc định Off.
    pub(crate) worker: std::cell::RefCell<crate::vm::worker::WorkerState>,
    /// **ST-72b (VD-103/VD-106)**: quyền `main` — `(chỉ số tham số, giá trị)`.
    pub(crate) caps: Vec<(u32, Value)>,
    /// **ST-238 (VD-242, ca 3)**: kho biến cấp tệp, ánh xạ từ ô `>= GLOBAL_SLOT_BASE`.
    ///
    /// **ST-250 (VD-245)**: mỗi ô là `Option<Value>` — `None` = **chưa khởi tạo**, `Some(v)` = đã
    /// ghi (kể cả `v == Value::Unit`). Trạng thái khởi tạo được tách khỏi giá trị ngôn ngữ: `Unit`
    /// hợp lệ không bao giờ bị nhầm với ô chưa có giá trị, và sentinel `None` không lọt vào
    /// `Value`/`show`/phép toán/serialization. Độ dài được cấp **theo module đã verify** ở đầu mỗi
    /// lần chạy, không tự resize theo chỉ số bytecode trong lúc chạy.
    pub(crate) globals: Vec<Option<Value>>,
    /// **ST-238 (VD-242, ca 8)**: ô chia sẻ của từng closure, khoá bằng địa chỉ `Rc<VmFunc>`.
    pub(crate) closure_cells: HashMap<usize, ClosureCells>,
    /// **VD-267** — nhớ `(tên enum, tên biến thể)` theo **chỉ số hằng** của `MakeEnum`. Mỗi hằng
    /// chỉ tách + nội bộ hoá tên một lần; các lần dựng sau chỉ clone `Name` (refcount), hết cảnh
    /// `ten_str → String` + hai `String` mỗi giá trị enum.
    pub(crate) enum_tags: Vec<std::cell::OnceCell<EnumTag>>,
    // ST-276/VD-256 — ngân sách ngữ nghĩa; `None` ⇒ KHÔNG charge (khi `--unlimited-budget`
    // hoặc đường API thấp không kèm bảng phí).
    pub(crate) budget_fee: Option<FeeTable>,
    pub(crate) budget_remaining: i64,
    pub(crate) budget_trace: Vec<FeeEntry>,
    pub(crate) budget_hit: bool,
    /// **ST-290 (VD-273)** — chỉ test-hook mới lưu trace; chạy mặc định (40M) **không** lưu.
    pub(crate) budget_trace_on: bool,
    // VD-257/D — hạn ngạch lượt THỬ NGHIỆM (test-hook), RIÊNG khỏi budget tổng.
    // `quota_active = false` (mặc định) ⇒ mọi thao tác quota là no-op.
    pub(crate) quota_active: bool,
    pub(crate) quota_limit: i64,
    pub(crate) quota_remaining: i64,
    pub(crate) quota_hit: bool,
    pub(crate) quota_yields: u64,
}

impl<'a> Vm<'a> {
    /// VM cho một module + văn bản nguồn (để đổi span byte → `dòng:cột`).
    pub fn new(m: &'a Module, text: &'a str) -> Self {
        Self {
            m,
            text,
            out: String::new(),
            steps: 0,
            safety_fuel: SAFETY_FUEL,
            deadline: None,
            depth: 0,
            dinh: 0,
            fs: None,
            net: None,
            db: None,
            ngoai: None,
            sched: std::cell::RefCell::new(vis_interp::rt::Scheduler::new()),
            worker: std::cell::RefCell::new(Default::default()),
            caps: Vec::new(),
            globals: Vec::new(),
            closure_cells: HashMap::new(),
            enum_tags: (0..m.consts.len())
                .map(|_| std::cell::OnceCell::new())
                .collect(),
            budget_fee: None,
            budget_remaining: 0,
            budget_trace: Vec::new(),
            budget_hit: false,
            budget_trace_on: false,
            quota_active: false,
            quota_limit: 0,
            quota_remaining: 0,
            quota_hit: false,
            quota_yields: 0,
        }
    }

    /// **ST-311a (#195)**: đặt deadline wall-clock monotonic cho lần chạy VM.
    pub fn set_deadline(&mut self, deadline: Option<std::time::Instant>) {
        self.deadline = deadline;
    }

    /// **ST-311a (#195)**: đặt thời lượng chạy tối đa (wall-clock monotonic) theo kiểu builder.
    pub fn with_timeout(mut self, dur: std::time::Duration) -> Self {
        self.deadline = Some(std::time::Instant::now() + dur);
        self
    }

    /// **ST-311a (#195)**: đặt giới hạn safety fuel cho đo lường/fuzz (mặc định [`SAFETY_FUEL`]).
    pub fn set_safety_fuel(&mut self, fuel: usize) {
        self.safety_fuel = fuel;
    }

    /// Độ sâu ngăn xếp **lớn nhất** đã dùng trong lần chạy.
    pub fn max_stack_used(&self) -> usize {
        self.dinh
    }

    /// Lấy `stdout` đã in.
    pub fn take_stdout(&mut self) -> String {
        std::mem::take(&mut self.out)
    }

    /// Chạy `main` của module.
    pub fn run(&mut self, args: &[String]) -> RunOutput {
        // **Bắt buộc** xác minh trước khi chạy (VD-52 #3): bytecode có thể đến từ cache.
        if let Err(e) = vis_bytecode::verify(self.m) {
            return RunOutput {
                stdout: String::new(),
                diagnostics: vec![vis_syntax::Diagnostic {
                    code: &vis_diag::RUNTIME_CODES[0],
                    message: format!("bytecode hỏng: {e}"),
                    span: Span::new(0, 0),
                    line: 1,
                    col: 1,
                }],
                value: None,
                exit: 1,
                steps: 0,
            };
        }
        self.steps = 0; // VD-256 D8: fuel reset mỗi lần chạy, không cộng dồn.
                        // **ST-290 (VD-273)** — ngân sách phiên (mặc định 40M): lấy bảng phí
                        // CLI đặt cho đúng lần chạy này; không có bảng phí ⇒ không charge (API thấp).
        crate::vm_frame::bat_dau_ngan_sach_vm(self);
        // **ST-250 (VD-245)**: reset + cấp phát kho global **theo module** ở đầu mỗi lần chạy.
        // Mọi ô bắt đầu `None` (chưa khởi tạo) — lần chạy trước không thể che lỗi lần sau, và kích
        // thước bị chặn bởi miền đã verify (không grow không giới hạn theo chỉ số bytecode).
        self.init_globals();
        let argv = Value::List(args.iter().map(|s| Value::Str(s.clone())).collect());
        // **ST-60 (VD-90/VD-95)**: join mọi tác vụ **theo thứ tự tạo**, **chỉ khi main thành công**
        // (và **trước** `take_stdout` ⇒ thân tác vụ in vẫn vào stdout, đúng thứ tự).
        let kq = match self.chay_chuong_trinh(&argv) {
            Ok(v) => match self.join_tasks() {
                Ok(()) => Ok(v),
                Err(e) => Err(e),
            },
            Err(e) => Err(e),
        };
        match kq {
            Ok(v) => {
                let exit = value::exit_code(&v);
                RunOutput {
                    stdout: self.take_stdout(),
                    diagnostics: Vec::new(),
                    value: Some(v),
                    exit,
                    steps: self.steps,
                }
            }
            Err(Abort::Trap(t)) => RunOutput {
                stdout: self.take_stdout(),
                diagnostics: vec![t.to_diag(self.text)],
                value: None,
                exit: 1,
                steps: self.steps,
            },
            Err(Abort::Ret(v)) => {
                // **VD-241 §3/T5 (A0, 2026-11-19)** — `Result::Err` ⇒ exit 1; dùng chung `value::exit_code`.
                let exit = value::exit_code(&v);
                RunOutput {
                    stdout: self.take_stdout(),
                    diagnostics: Vec::new(),
                    value: Some(v),
                    exit,
                    steps: self.steps,
                }
            }
            // Chỉ đường test/frame — `run()` mặc định không bật frame; driver có entry riêng.
            Err(Abort::Pending(_)) => RunOutput {
                stdout: self.take_stdout(),
                diagnostics: Vec::new(),
                value: None,
                exit: 1,
                steps: self.steps,
            },
        }
    }

    fn chay_chuong_trinh(&mut self, argv: &Value) -> R<Value> {
        let entry = self.m.entry;
        // **ST-157 (nợ #110, VD-156)**: codegen bọc `fn main` của người dùng trong một hàm "vỏ"
        // `Call(main, 0); Halt` có **0 tham số** (`vis-bytecode/src/codegen.rs`, mục `main`) ⇒ vỏ
        // không nhận được `argv`. Khi `main` có tham số, chạy **thẳng** `main` để đặt `argv`/`cap`
        // vào đúng ô tham số như interp (`interp.rs:221-234`); `main` **không** tham số giữ nguyên
        // đường cũ (chạy vỏ) nên không đổi cách gọi.
        let vo = self.func(entry)?;
        let fi = match vo.code.as_slice() {
            [Instr::Call(fi, 0), Instr::Halt] if vo.nparams == 0 => {
                let g = self.func(*fi)?;
                if g.name == "main" && g.nparams > 0 {
                    *fi
                } else {
                    entry
                }
            }
            _ => entry,
        };
        let f = self.func(fi)?;
        let mut locals = vec![Value::Unit; f.nlocals.max(1) as usize];
        let nparams = f.nparams as usize;
        let mut la_cap = vec![false; nparams];
        // **ST-72b**: đặt `Value::Cap` đúng **chỉ số** tham số.
        for (pos, v) in &self.caps {
            if let Some(l) = locals.get_mut(*pos as usize) {
                *l = v.clone();
                if let Some(c) = la_cap.get_mut(*pos as usize) {
                    *c = true;
                }
            }
        }
        // **ST-157 (nợ #110)**: tham số **không** `cap` của `main` nhận `argv` (`list[str]`) **như
        // interp** — trước đây điều kiện `arity == 0` khiến `main(args: list[str])` nhận `()` trên
        // VM. Không có `cap` ⇒ interp truyền **một** đối số vị trí `argv` (tham số sau lấy mặc
        // định): VM giữ đúng cách gọi đó ở tham số 0.
        if self.caps.is_empty() {
            if nparams > 0 {
                if let Some(l) = locals.first_mut() {
                    *l = argv.clone();
                }
            }
        } else {
            for i in 0..nparams.min(locals.len()) {
                if !la_cap[i] {
                    locals[i] = argv.clone();
                }
            }
        }
        let mut th = Th::new(self.m, fi, locals, Vec::new());
        let v = th.exec(self)?;
        Ok(v)
    }

    pub(crate) fn func(&self, i: u32) -> R<&Func> {
        self.m
            .funcs
            .get(i as usize)
            .ok_or_else(|| self.hong(VmError::BadIdx(i), Span::new(0, 0)))
    }

    /// **ST-250 (VD-245)**: đặt lại kho global cho **một lần chạy** — mọi ô `None` (chưa khởi tạo).
    ///
    /// Kích thước lấy từ chính module đã verify: `1 + max(s - GLOBAL_SLOT_BASE)` trên mọi lệnh ghi
    /// vùng global. Vì `verify()` đã chặn `s - BASE >= nglobals`, số ô này bị giới hạn bởi số global
    /// thật của module; VM **không** resize bộ nhớ theo một chỉ số bytecode tùy ý lúc chạy.
    pub(crate) fn init_globals(&mut self) {
        let mut n = 0usize;
        for f in &self.m.funcs {
            for ins in &f.code {
                let off = match *ins {
                    Instr::StoreLocal(s) | Instr::Assign(s) | Instr::BindLocal(s)
                        if s >= GLOBAL_SLOT_BASE =>
                    {
                        s - GLOBAL_SLOT_BASE
                    }
                    _ => continue,
                };
                n = n.max(off as usize + 1);
            }
        }
        self.globals = vec![None; n];
    }

    /// **ST-250 (VD-245)**: **mọi đường đọc global** đi qua đây — ô được ghi (`Some(v)`, kể cả
    /// `v == Unit`) mới trả giá trị; còn lại (chưa khởi tạo, kể cả ô ngoài kho hiện tại) ⇒ trap
    /// có chẩn đoán, không trả giá trị mặc định. Dùng mã runtime hiện hành `RT` (`VS0501`), không
    /// thêm mã chẩn đoán mới.
    fn doc_global(&self, k: usize, span: Span) -> R<Value> {
        match self.globals.get(k) {
            Some(Some(v)) => Ok(v.clone()),
            _ => Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "global chưa được khởi tạo tại ô {} (chỉ số bytecode {})",
                    k,
                    k + GLOBAL_SLOT_BASE as usize
                ),
                span,
            ))),
        }
    }

    /// **ST-252 (VD-246)**: mọi đường đọc ô cục bộ (non-global) đi qua đây — chỉ số phải nằm
    /// trong **khung thực tế**, nếu không trap có chẩn đoán bằng mã runtime hiện hành (`RT`),
    /// không trả `Value::Unit` mặc định. `Unit` là giá trị **hợp lệ** đã được khởi tạo sẵn ở dựng
    /// khung, nên một ô trong khung chứa `Unit` vẫn trả `Unit` bình thường (không nhầm với lỗi).
    fn doc_local(&self, locals: &[Value], i: u16, span: Span) -> R<Value> {
        locals.get(i as usize).cloned().ok_or_else(|| {
            Abort::Trap(Trap::new(
                RT,
                format!("biến cục bộ {i} ngoài khung ({} ô)", locals.len()),
                span,
            ))
        })
    }

    /// **ST-252 (VD-246)**: mọi đường đọc biến bắt đi qua đây — chỉ số phải nằm trong **môi
    /// trường capture thực tế** của khung, nếu không trap có chẩn đoán (`RT`). Capture chứa `Unit`
    /// là hợp lệ ⇒ trả `Unit` bình thường; chỉ có truy cập ngoài miền mới là lỗi.
    fn doc_captured(&self, captured: &[Value], i: u16, span: Span) -> R<Value> {
        captured.get(i as usize).cloned().ok_or_else(|| {
            Abort::Trap(Trap::new(
                RT,
                format!(
                    "biến bắt {i} ngoài môi trường capture ({} ô)",
                    captured.len()
                ),
                span,
            ))
        })
    }

    /// **ST-252 (VD-246)**: ghi ô cục bộ có kiểm tra — chỉ số ngoài khung ⇒ trap, **không**
    /// `resize` khung (cùng bản chất với đọc fallback: ô ngoài miền không được âm thầm mở rộng).
    fn ghi_local(&self, locals: &mut [Value], i: u16, v: Value, span: Span) -> R<()> {
        let n = locals.len();
        let slot = locals.get_mut(i as usize).ok_or_else(|| {
            Abort::Trap(Trap::new(
                RT,
                format!("ghi biến cục bộ {i} ngoài khung ({n} ô)"),
                span,
            ))
        })?;
        *slot = v;
        Ok(())
    }

    pub(crate) fn hong(&self, e: VmError, span: Span) -> Abort {
        Abort::Trap(Trap::new(RT, format!("bytecode hỏng: {e}"), span))
    }

    fn lit(&self, i: u32, span: Span) -> R<Value> {
        let c = self
            .m
            .consts
            .get(i as usize)
            .ok_or_else(|| self.hong(VmError::BadIdx(i), span))?;
        let kind = lit_kind(c.kind).ok_or_else(|| self.hong(VmError::BadIdx(i), span))?;
        let lit = vis_types::hir::Lit {
            kind,
            text: c.text.clone(),
        };
        vis_interp::interp::lit_value(&lit)
            .map_err(|e| Abort::Trap(Trap::new(&vis_diag::TYPE_CODES[0], e, span)))
    }

    fn ten(&self, i: u32, span: Span) -> R<String> {
        Ok(self.ten_str(i, span)?.to_string())
    }

    /// **VD-267** — như [`Self::ten`] nhưng **mượn** chuỗi trong bảng hằng, không cấp phát
    /// `String`. Dùng cho chỗ chỉ cần đọc tên (vd. `MakeEnum` nội bộ hoá [`Name`]).
    fn ten_str(&self, i: u32, span: Span) -> R<&str> {
        let c = self
            .m
            .consts
            .get(i as usize)
            .ok_or_else(|| self.hong(VmError::BadIdx(i), span))?;
        Ok(match c.kind {
            5 => c.text.trim_matches('"'),
            _ => c.text.as_str(),
        })
    }
}

impl Vm<'_> {
    fn goi_ham_bytecode(&mut self, f: &Rc<VmFunc>, args: Vec<Value>, span: Span) -> R<Value> {
        if self.depth + 1 >= MAX_FRAMES {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("gọi hàm quá sâu (>{MAX_FRAMES} khung) — đệ quy không có điểm dừng?"),
                span,
            )));
        }
        let m = self.m;
        let fc = m
            .funcs
            .get(f.idx as usize)
            .ok_or_else(|| self.hong(VmError::BadIdx(f.idx), span))?;
        let nlocals = fc.nlocals.max(1) as usize;
        // **ST-238 (VD-242, ca 8)**: nhận diện prologue `LoadCaptured(i); Assign(slot)` — các ô
        // cục bộ này là **mirror** của ô chia sẻ; khi khung kết thúc phải ghi ngược vào ô đó.
        let mut mirror: Vec<(u16, usize)> = Vec::new();
        let mut k = 0;
        while k + 1 < fc.code.len() {
            if let (Instr::LoadCaptured(i), Instr::Assign(slot)) = (fc.code[k], fc.code[k + 1]) {
                mirror.push((slot, i as usize));
                k += 2;
            } else {
                break;
            }
        }
        let mut locals = vec![Value::Unit; nlocals];
        for (i, a) in args.into_iter().enumerate() {
            if i < locals.len() {
                locals[i] = a;
            }
        }
        // **ST-238 (VD-242, ca 8)**: ô chia sẻ theo địa chỉ closure — giá trị khởi tạo lấy từ
        // `MakeClosure` (hoặc từ `f.captured` nếu closure dựng sẵn không qua `MakeClosure`).
        let ptr = Rc::as_ptr(f) as usize;
        let cells = match self.closure_cells.get(&ptr) {
            Some(c) => c.clone(),
            None => {
                let c = Rc::new(RefCell::new(f.captured.clone()));
                self.closure_cells.insert(ptr, c.clone());
                c
            }
        };
        let captured = cells.borrow().clone();
        let mut th = Th::new(m, f.idx, locals, captured);
        if let Some(fr) = th.frames.first_mut() {
            fr.writeback = Some((cells, mirror));
        }
        self.depth += 1;
        let v = th.exec(self);
        self.depth -= 1;
        v
    }
}

/// Trạng thái thực thi của **một** lần gọi (khung + ngăn xếp giá trị + iterator).
pub(crate) struct Th<'m> {
    pub(crate) m: &'m Module,
    pub(crate) frames: Vec<Frame>,
    pub(crate) st: Vec<Value>,
    /// **VD-264**: bộ lặp dùng chung với interp — range duyệt lười (không còn `Vec<Value>`).
    pub(crate) iters: Vec<value::ForIter>,
    /// **VD-258**: các vòng callback đang mở (chỉ khác rỗng trên đường frame test).
    pub(crate) methods: Vec<MethodLoop>,
}

impl<'m> Th<'m> {
    pub(crate) fn new(m: &'m Module, fi: u32, locals: Vec<Value>, captured: Vec<Value>) -> Self {
        Self {
            m,
            frames: vec![Frame {
                fi,
                pc: 0,
                locals,
                captured,
                st_cu: Vec::new(),
                writeback: None,
                iter_base: 0,
                loop_ends: HashMap::new(),
            }],
            st: Vec::new(),
            iters: Vec::new(),
            methods: Vec::new(),
        }
    }

    pub(super) fn f(&self) -> &Func {
        // `fi` do `Vm::func` đã kiểm trước khi dựng khung.
        &self.m.funcs[self.frames.last().expect("khung").fi as usize]
    }

    pub(super) fn pop(&mut self, vm: &Vm<'_>, span: Span) -> R<Value> {
        self.st
            .pop()
            .ok_or_else(|| vm.hong(VmError::StackUnderflow, span))
    }

    pub(super) fn pop_n(&mut self, vm: &Vm<'_>, n: usize, span: Span) -> R<Vec<Value>> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.pop(vm, span)?);
        }
        v.reverse();
        Ok(v)
    }

    /// **ST-238 (VD-242, ca 1) / ST-311a (#195)**: đặt `pc` cho khung hiện tại, đồng thời nếu
    /// đích là `pc` kết thúc của một vòng `for` đang mở thì pop iterator của vòng đó — tra cứu
    /// O(1) trên `Frame::loop_ends`, không quét tuyến tính O(n).
    pub(super) fn dat_pc(&mut self, pc: usize) {
        let fr = self.frames.last_mut().expect("khung");
        if let Some(base) = fr.loop_ends.remove(&pc) {
            if self.iters.len() > base {
                self.iters.truncate(base);
            }
        }
        fr.pc = pc;
    }

    /// **ST-238 (VD-242, ca 8) / ST-311a (#190)**: pop khung, dọn iterator bị bỏ lại nếu hàm
    /// `return`/`?` sớm bên trong vòng `for`, và ghi ngược ô mirror của closure vào ô chia sẻ —
    /// trap `VS0501` nếu ô cục bộ hoặc chỉ số biến bắt ngoài miền (không bỏ qua im lặng).
    pub(super) fn pop_frame(&mut self, span: Span) -> R<Option<Frame>> {
        let Some(fr) = self.frames.pop() else {
            return Ok(None);
        };
        if self.iters.len() > fr.iter_base {
            self.iters.truncate(fr.iter_base);
        }
        if let Some((cells, mirror)) = &fr.writeback {
            let mut c = cells.borrow_mut();
            for (slot, i) in mirror {
                let Some(src) = fr.locals.get(*slot as usize) else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!(
                            "bytecode hỏng: writeback closure ô cục bộ {} ngoài khung ({} ô)",
                            slot,
                            fr.locals.len()
                        ),
                        span,
                    )));
                };
                let Some(dst) = c.get_mut(*i) else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!(
                            "bytecode hỏng: writeback closure biến bắt {} ngoài môi trường ({} ô)",
                            i,
                            c.len()
                        ),
                        span,
                    )));
                };
                *dst = src.clone();
            }
        }
        Ok(Some(fr))
    }

    pub(super) fn day(&self, fr: &Frame, pc: i64, vm: &Vm<'_>, span: Span, what: &str) -> R<()> {
        let len = vm.func(fr.fi)?.code.len() as i64;
        if pc < 0 || pc > len {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("bytecode hỏng: {} nhảy ngoài code ({pc}/{len})", what),
                span,
            )));
        }
        Ok(())
    }
}

// ST-252 (VD-246): test nội bộ cho hàng rào runtime (helper private, không gọi được từ integration test).
#[cfg(test)]
mod tests;
