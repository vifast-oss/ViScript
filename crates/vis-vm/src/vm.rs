// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vòng lặp **fetch–decode–execute** trên [`Module`] (ST-20).
//!
//! Ngăn xếp giá trị **động** (`Vec<Value>`), không dựa vào `max_stack` (VD-54 (2)); ngăn xếp
//! khung tách riêng để `return` lồng nhau sạch; iterator của `for` nằm ở `iters` (không lẫn với
//! ngăn xếp giá trị).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vis_bytecode::{Func, Instr, Module, BI_MATCH_NO_ARM, GLOBAL_SLOT_BASE};
use vis_interp::builtin::{self, Host};
use vis_interp::interp::{Abort, Trap, IDX, R, RT};
use vis_interp::value::{self, Value, VmFunc};
use vis_interp::RunOutput;
use vis_syntax::Span;
use vis_types::hir::LitKind;

/// Chạy một module VBC1 (đường tắt tiện dụng).
pub fn run(m: &Module, text: &str, args: &[String]) -> RunOutput {
    Vm::new(m, text).run(args)
}

/// Như [`run`] nhưng gắn **host hệ thống tệp** (ST-33). `None` ⇒ `std.fs.*` ⇒ `VS0604`.
pub fn run_with_fs<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
) -> RunOutput {
    run_with_hosts(m, text, args, fs, None)
}

/// **ST-63 (VD-105)**: như [`run_with_fs`] nhưng gắn **cả host mạng** (`std.net.*`); `None` ở host
/// nào ⇒ `VS0604`. Đây là cửa CLI dùng để hai đường (interp/VM) nhận **cùng** host.
pub fn run_with_hosts<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
) -> RunOutput {
    run_with_hosts_caps(m, text, args, fs, net, &[])
}

/// **ST-72b (VD-103/VD-106)**: như [`run_with_hosts`] nhưng gắn giá trị quyền `cap` cho `main`.
/// **ST-89**: giữ **nguyên chữ ký cũ** — không có host `Ngoai` (wrapper của
/// [`run_with_full_hosts_caps`]).
pub fn run_with_hosts_caps<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
    caps: &[(u32, Value)],
) -> RunOutput {
    run_with_full_hosts_caps(m, text, args, None, fs, net, None, caps)
}

/// **ST-89 (VD-110)**: như [`run_with_hosts_caps`] nhưng nhận **cả** host chương trình ngoài — cùng
/// trait `vis_interp::builtin::Ngoai` mà interp dùng; VM **không** gọi `std::process`.
/// **VD-241 (Gate 1)**: nhận thêm **host cơ sở dữ liệu** (`None` ⇒ `std.sqlite.*` ⇒ `VS0604`).
#[allow(clippy::too_many_arguments)]
pub fn run_with_full_hosts_caps<'a>(
    m: &'a Module,
    text: &'a str,
    args: &[String],
    ngoai: Option<&'a mut dyn vis_interp::builtin::Ngoai>,
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
    db: Option<&'a mut dyn vis_interp::builtin::DbHost>,
    caps: &[(u32, Value)],
) -> RunOutput {
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
    vm.run(args)
}

/// Cap số khung gọi lồng nhau (VD-49).
pub const MAX_FRAMES: usize = 2_000;
/// Cap số **lệnh** đã thực thi — cùng ngưỡng 10^7 với interp (khác đơn vị đếm).
pub const MAX_STEPS: usize = 10_000_000;

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

/// **ST-238 (VD-242, ca 8)**: ô chia sẻ của closure và cặp `(ô cục bộ mirror, chỉ số ô bắt)`.
type ClosureCells = Rc<RefCell<Vec<Value>>>;
type CapturedMirror = Vec<(u16, usize)>;

struct Frame {
    fi: u32,
    pc: usize,
    locals: Vec<Value>,
    captured: Vec<Value>,
    /// Ngăn xếp **của khung gọi** được cất khi gọi hàm (mỗi khung một ngăn xếp riêng).
    st_cu: Vec<Value>,
    /// **ST-238 (VD-242, ca 8)**: nếu khung là một lần gọi closure, chứa ô chia sẻ của closure
    /// và cặp `(ô cục bộ, chỉ số biến bắt)` để ghi ngược giá trị khi khung kết thúc — giúp closure
    /// ghi biến bắt `var` giữ trạng thái **giữa các lần gọi** mà vẫn không đụng biến ngoài.
    writeback: Option<(ClosureCells, CapturedMirror)>,
}

/// Máy ảo: giữ module + văn bản nguồn + `stdout` + số bước.
pub struct Vm<'a> {
    m: &'a Module,
    text: &'a str,
    out: String,
    steps: usize,
    depth: usize,
    /// Độ sâu ngăn xếp giá trị **lớn nhất đã dùng** (bất biến của fuzz ST-21).
    dinh: usize,
    /// Host hệ thống tệp (ST-33); `None` ⇒ `VS0604` — VM **không** biết `std::fs`.
    fs: Option<&'a mut dyn vis_interp::builtin::FsHost>,
    /// **ST-63 (VD-105)**: host mạng; `None` ⇒ `VS0604` — VM **không** biết `std::net`.
    net: Option<&'a mut dyn vis_interp::builtin::NetHost>,
    /// **ST-89 (VD-110)**: host **chương trình ngoài** — VM **không** gọi `std::process`, chỉ qua
    /// trait của `vis-interp`; `None` ⇒ `exec.*` ⇒ `VS0604`.
    ngoai: Option<&'a mut dyn vis_interp::builtin::Ngoai>,
    /// **VD-241 (Gate 1)**: host **cơ sở dữ liệu** — VM **không** biết backend native; `None` ⇒
    /// `std.sqlite.*` ⇒ `VS0604`.
    db: Option<&'a mut dyn vis_interp::builtin::DbHost>,
    /// **ST-60**: hàng đợi tác vụ `async` — cùng runtime với interp (`vis_interp::rt`).
    sched: std::cell::RefCell<vis_interp::rt::Scheduler>,
    /// **ST-72b (VD-103/VD-106)**: quyền `main` — `(chỉ số tham số, giá trị)`.
    caps: Vec<(u32, Value)>,
    /// **ST-238 (VD-242, ca 3)**: kho biến cấp tệp, ánh xạ từ ô `>= GLOBAL_SLOT_BASE`.
    globals: Vec<Value>,
    /// **ST-238 (VD-242, ca 8)**: ô chia sẻ của từng closure, khoá bằng địa chỉ `Rc<VmFunc>`.
    closure_cells: HashMap<usize, ClosureCells>,
}

impl<'a> Vm<'a> {
    /// VM cho một module + văn bản nguồn (để đổi span byte → `dòng:cột`).
    pub fn new(m: &'a Module, text: &'a str) -> Self {
        Self {
            m,
            text,
            out: String::new(),
            steps: 0,
            depth: 0,
            dinh: 0,
            fs: None,
            net: None,
            db: None,
            ngoai: None,
            sched: std::cell::RefCell::new(vis_interp::rt::Scheduler::new()),
            caps: Vec::new(),
            globals: Vec::new(),
            closure_cells: HashMap::new(),
        }
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
        let argv = Value::List(args.iter().map(|s| Value::Str(s.clone())).collect());
        // **ST-60 (VD-90/VD-95)**: join mọi tác vụ **theo thứ tự tạo**, **chỉ khi main thành công**
        // (và **trước** `take_stdout` ⇒ thân tác vụ in vẫn vào stdout, đúng thứ tự).
        let kq = match self.chay_chuong_trinh(&argv) {
            Ok(v) => match vis_interp::rt::join_all(self) {
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

    fn func(&self, i: u32) -> R<&Func> {
        self.m
            .funcs
            .get(i as usize)
            .ok_or_else(|| self.hong(VmError::BadIdx(i), Span::new(0, 0)))
    }

    fn hong(&self, e: VmError, span: Span) -> Abort {
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
        let c = self
            .m
            .consts
            .get(i as usize)
            .ok_or_else(|| self.hong(VmError::BadIdx(i), span))?;
        Ok(match c.kind {
            5 => c.text.trim_matches('"').to_string(),
            _ => c.text.clone(),
        })
    }
}

impl<'a> Vm<'a> {
    /// Gắn host hệ thống tệp (ST-33) — chỉ dùng qua [`crate::run_with_fs`].
    pub fn set_fs(&mut self, f: &'a mut dyn vis_interp::builtin::FsHost) {
        self.fs = Some(f);
    }

    /// **ST-63 (VD-105)**: gắn host mạng — chỉ dùng qua [`crate::run_with_hosts`].
    pub fn set_net(&mut self, n: &'a mut dyn vis_interp::builtin::NetHost) {
        self.net = Some(n);
    }

    /// **VD-241 (Gate 1)**: gắn host cơ sở dữ liệu — VM chỉ gọi qua trait, không biết native.
    pub fn set_db(&mut self, d: &'a mut dyn vis_interp::builtin::DbHost) {
        self.db = Some(d);
    }

    /// **ST-89 (VD-110)**: gắn host chương trình ngoài — VM chỉ gọi qua trait, không chạm OS.
    pub fn set_ngoai(&mut self, n: &'a mut dyn vis_interp::builtin::Ngoai) {
        self.ngoai = Some(n);
    }
}

impl Host for Vm<'_> {
    fn out(&mut self) -> &mut String {
        &mut self.out
    }

    fn fs(&mut self) -> Option<&mut dyn vis_interp::builtin::FsHost> {
        self.fs
            .as_mut()
            .map(|f| &mut **f as &mut dyn vis_interp::builtin::FsHost)
    }

    /// **ST-63 (VD-105)**: host mạng; `None` ⇒ `std.net.*` ⇒ `VS0604`.
    fn net(&mut self) -> Option<&mut dyn vis_interp::builtin::NetHost> {
        self.net
            .as_mut()
            .map(|n| &mut **n as &mut dyn vis_interp::builtin::NetHost)
    }

    /// **VD-241 (Gate 1)**: host cơ sở dữ liệu; `None` ⇒ `std.sqlite.*` ⇒ `VS0604`.
    fn db(&mut self) -> Option<&mut dyn vis_interp::builtin::DbHost> {
        self.db
            .as_mut()
            .map(|d| &mut **d as &mut dyn vis_interp::builtin::DbHost)
    }

    /// **ST-89 (VD-110)**: chương trình ngoài; `None` ⇒ `VS0604`.
    fn ngoai(
        &mut self,
        ten: &str,
        args: &[String],
        raw: bool,
        stdin: &str,
        span: Span,
    ) -> Option<R<Value>> {
        self.ngoai
            .as_mut()
            .map(|n| (**n).chay(ten, args, raw, stdin, span))
    }

    /// **ST-89 (VD-110)**: `| complete` — đủ `stdout/stderr/code` từ host.
    fn ngoai_day_du(
        &mut self,
        ten: &str,
        args: &[String],
        stdin: &str,
        span: Span,
    ) -> Option<Result<vis_interp::builtin::KetQua, vis_interp::interp::Abort>> {
        self.ngoai
            .as_mut()
            .map(|n| (**n).chay_day_du(ten, args, stdin, span))
    }

    fn sched(&self) -> &std::cell::RefCell<vis_interp::rt::Scheduler> {
        &self.sched
    }

    /// **ST-60**: handle của VM là **chỉ số hàm bytecode** — chạy bằng `goi_ham_bytecode`.
    fn goi_than(&mut self, fi: u32, args: Vec<Value>, span: Span) -> R<Value> {
        let f = Rc::new(VmFunc {
            idx: fi,
            captured: Vec::new(),
        });
        self.goi_ham_bytecode(&f, args, span)
    }

    fn ten_than(&self, fi: u32) -> String {
        self.m
            .funcs
            .get(fi as usize)
            .map(|f| f.name.clone())
            .unwrap_or_default()
    }

    fn call(&mut self, f: &Value, args: Vec<Value>, span: Span) -> R<Value> {
        match f {
            Value::Vm(v) => self.goi_ham_bytecode(v, args, span),
            Value::Builtin(n) => builtin::call(self, n, args, span),
            other => Err(Abort::Trap(Trap::new(
                RT,
                format!("`{}` không phải hàm", other.show()),
                span,
            ))),
        }
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
struct Th<'m> {
    m: &'m Module,
    frames: Vec<Frame>,
    st: Vec<Value>,
    iters: Vec<(Vec<Value>, usize)>,
    /// **ST-238 (VD-242, ca 1)**: `(fi, pc_kết_thúc, độ_sâu_iters_trước_vòng)` cho mỗi `for`
    /// đang mở — để `break` nhảy tới cuối vòng thì cũng pop iterator như `IterNext` hết vòng.
    loop_ends: Vec<(u32, usize, usize)>,
}

impl<'m> Th<'m> {
    fn new(m: &'m Module, fi: u32, locals: Vec<Value>, captured: Vec<Value>) -> Self {
        Self {
            m,
            frames: vec![Frame {
                fi,
                pc: 0,
                locals,
                captured,
                st_cu: Vec::new(),
                writeback: None,
            }],
            st: Vec::new(),
            iters: Vec::new(),
            loop_ends: Vec::new(),
        }
    }

    fn f(&self) -> &Func {
        // `fi` do `Vm::func` đã kiểm trước khi dựng khung.
        &self.m.funcs[self.frames.last().expect("khung").fi as usize]
    }

    fn pop(&mut self, vm: &Vm<'_>, span: Span) -> R<Value> {
        self.st
            .pop()
            .ok_or_else(|| vm.hong(VmError::StackUnderflow, span))
    }

    fn pop_n(&mut self, vm: &Vm<'_>, n: usize, span: Span) -> R<Vec<Value>> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.pop(vm, span)?);
        }
        v.reverse();
        Ok(v)
    }

    /// **ST-238 (VD-242, ca 1)**: đặt `pc` cho khung hiện tại, đồng thời nếu đích là `pc` kết
    /// thúc của một vòng `for` đang mở thì pop iterator của vòng đó — `break` nhảy tới cuối vòng
    /// phải có cùng hiệu ứng với `IterNext` khi hết phần tử, nếu không iterator vòng trong còn
    /// nằm trên chồng và vòng ngoài sẽ đọc nhầm ⇒ lặp vô hạn.
    fn dat_pc(&mut self, pc: usize) {
        let fi = self.frames.last().expect("khung").fi;
        if let Some(idx) = self
            .loop_ends
            .iter()
            .rposition(|(f, end, _)| *f == fi && *end == pc)
        {
            let base = self.loop_ends[idx].2;
            if self.iters.len() > base {
                self.iters.truncate(base);
            }
            self.loop_ends.remove(idx);
        }
        self.frames.last_mut().expect("khung").pc = pc;
    }

    /// **ST-238 (VD-242, ca 8)**: pop khung + ghi ngược ô mirror của closure vào ô chia sẻ.
    fn pop_frame(&mut self) -> Option<Frame> {
        let fr = self.frames.pop()?;
        if let Some((cells, mirror)) = &fr.writeback {
            let mut c = cells.borrow_mut();
            for (slot, i) in mirror {
                if let (Some(dst), Some(src)) = (c.get_mut(*i), fr.locals.get(*slot as usize)) {
                    *dst = src.clone();
                }
            }
        }
        Some(fr)
    }

    fn day(&self, fr: &Frame, pc: i64, vm: &Vm<'_>, span: Span, what: &str) -> R<()> {
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

    fn exec(&mut self, vm: &mut Vm<'_>) -> R<Value> {
        loop {
            // Ghi nhận độ sâu ngăn xếp **lớn nhất** (bất biến fuzz ST-21: `dinh ≤ need`).
            if self.st.len() > vm.dinh {
                vm.dinh = self.st.len();
            }
            let Some(fr) = self.frames.last() else {
                return Ok(Value::Unit);
            };
            let f = self.f();
            if fr.pc >= f.code.len() {
                // Hết mã mà không `Return`/`Halt` — trả `()`.
                let v = Value::Unit;
                self.pop_frame();
                match self.frames.last_mut() {
                    None => return Ok(v),
                    Some(f) => {
                        self.st = std::mem::take(&mut f.st_cu);
                        self.st.push(v);
                    }
                }
                continue;
            }
            let ins = f.code[fr.pc];
            let span = f
                .spans
                .get(fr.pc)
                .map(|s| Span::new(s.start as usize, s.end as usize))
                .unwrap_or_else(|| Span::new(0, 0));
            self.frames.last_mut().expect("khung").pc += 1;
            vm.steps += 1;
            if vm.steps > MAX_STEPS {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    "chạy quá lâu (>10^7 lệnh)",
                    span,
                )));
            }
            match ins {
                Instr::Const(i) => {
                    let v = vm.lit(i, span)?;
                    self.st.push(v);
                }
                Instr::LoadLocal(i) => {
                    // **ST-238 (VD-242, ca 3)**: ô `>= GLOBAL_SLOT_BASE` là biến cấp tệp dùng chung.
                    let v = if i >= GLOBAL_SLOT_BASE {
                        vm.globals
                            .get((i - GLOBAL_SLOT_BASE) as usize)
                            .cloned()
                            .unwrap_or(Value::Unit)
                    } else {
                        let fr = self.frames.last().expect("khung");
                        fr.locals.get(i as usize).cloned().unwrap_or(Value::Unit)
                    };
                    self.st.push(v);
                }
                Instr::StoreLocal(i) | Instr::Assign(i) | Instr::BindLocal(i) => {
                    let v = self.pop(vm, span)?;
                    if i >= GLOBAL_SLOT_BASE {
                        let k = (i - GLOBAL_SLOT_BASE) as usize;
                        if vm.globals.len() <= k {
                            vm.globals.resize(k + 1, Value::Unit);
                        }
                        vm.globals[k] = v;
                    } else {
                        let fr = self.frames.last_mut().expect("khung");
                        if (i as usize) < fr.locals.len() {
                            fr.locals[i as usize] = v;
                        } else {
                            fr.locals.resize(i as usize + 1, Value::Unit);
                            fr.locals[i as usize] = v;
                        }
                    }
                }
                Instr::LoadCaptured(i) => {
                    let fr = self.frames.last().expect("khung");
                    let v = fr.captured.get(i as usize).cloned().unwrap_or(Value::Unit);
                    self.st.push(v);
                }
                Instr::Pop => {
                    self.pop(vm, span)?;
                }
                Instr::Dup => {
                    let v = self
                        .st
                        .last()
                        .cloned()
                        .ok_or_else(|| vm.hong(VmError::StackUnderflow, span))?;
                    self.st.push(v);
                }
                Instr::Bin(op) => {
                    let r = self.pop(vm, span)?;
                    let l = self.pop(vm, span)?;
                    let v = builtin::bin(op, l, r, span)?;
                    self.st.push(v);
                }
                Instr::Un(op) => {
                    let v = self.pop(vm, span)?;
                    let v = builtin::un(op, v, span)?;
                    self.st.push(v);
                }
                Instr::Call(fi, n) => {
                    let f = vm.func(fi)?;
                    let args = self.pop_n(vm, n as usize, span)?;
                    let mut locals = vec![Value::Unit; f.nlocals.max(1) as usize];
                    for (i, a) in args.into_iter().enumerate() {
                        if i < locals.len() {
                            locals[i] = a;
                        }
                    }
                    if self.frames.len() + vm.depth + 1 >= MAX_FRAMES {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!("gọi hàm quá sâu (>{MAX_FRAMES} khung) — đệ quy không có điểm dừng?"),
                            span,
                        )));
                    }
                    // Mỗi khung một ngăn xếp: cất ngăn xếp khung gọi, mở ngăn xếp mới.
                    let cu = std::mem::take(&mut self.st);
                    if let Some(f) = self.frames.last_mut() {
                        f.st_cu = cu;
                    }
                    self.frames.push(Frame {
                        fi,
                        pc: 0,
                        locals,
                        captured: Vec::new(),
                        st_cu: Vec::new(),
                        writeback: None,
                    });
                }
                Instr::CallBuiltin(i, n) => {
                    let ten = vm.ten(i, span)?;
                    let args = self.pop_n(vm, n as usize, span)?;
                    // **ST-238 (VD-242, ca 7)**: builtin giả do codegen phát ở nhánh `match` rơi —
                    // trap `VS0501` với cùng thông điệp/span của interp, không im lặng đi tiếp.
                    if ten == BI_MATCH_NO_ARM {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "`match` không khớp nhánh nào (không xét hết bị lọt)",
                            span,
                        )));
                    }
                    let v = builtin::call(vm, &ten, args, span)?;
                    self.st.push(v);
                }
                Instr::CallMethod(i, n) => {
                    let ten = vm.ten(i, span)?;
                    let args = self.pop_n(vm, n as usize, span)?;
                    let recv = self.pop(vm, span)?;
                    let v = builtin::method(vm, recv, &ten, args, span)?;
                    self.st.push(v);
                }
                Instr::Jump(d) => {
                    let pc = {
                        let fr = self.frames.last().expect("khung");
                        let pc = fr.pc as i64 - 1 + d as i64;
                        self.day(fr, pc, vm, span, "Jump")?;
                        pc
                    };
                    self.dat_pc(pc as usize);
                }
                Instr::JumpIfFalse(d) => {
                    let v = self.pop(vm, span)?;
                    let ok = v.truthy(span)?;
                    if !ok {
                        let pc = {
                            let fr = self.frames.last().expect("khung");
                            let pc = fr.pc as i64 - 1 + d as i64;
                            self.day(fr, pc, vm, span, "JumpIfFalse")?;
                            pc
                        };
                        self.dat_pc(pc as usize);
                    }
                }
                Instr::JumpIfNotTrue(d) => {
                    let v = self
                        .st
                        .last()
                        .cloned()
                        .ok_or_else(|| vm.hong(VmError::StackUnderflow, span))?;
                    if !v.truthy(span)? {
                        let pc = {
                            let fr = self.frames.last().expect("khung");
                            let pc = fr.pc as i64 - 1 + d as i64;
                            self.day(fr, pc, vm, span, "JumpIfNotTrue")?;
                            pc
                        };
                        self.dat_pc(pc as usize);
                    }
                }
                Instr::JumpIfNone(d) => {
                    // Luôn **pop** (1→0); nhảy khi đúng điều kiện.
                    let v = self.pop(vm, span)?;
                    if v.is_none() {
                        let pc = {
                            let fr = self.frames.last().expect("khung");
                            let pc = fr.pc as i64 - 1 + d as i64;
                            self.day(fr, pc, vm, span, "JumpIfNone")?;
                            pc
                        };
                        self.dat_pc(pc as usize);
                    }
                }
                Instr::JumpIfSome(d) => {
                    // Luôn **pop** (1→0); nhảy khi đúng điều kiện.
                    let v = self.pop(vm, span)?;
                    if v.is_some() {
                        let pc = {
                            let fr = self.frames.last().expect("khung");
                            let pc = fr.pc as i64 - 1 + d as i64;
                            self.day(fr, pc, vm, span, "JumpIfSome")?;
                            pc
                        };
                        self.dat_pc(pc as usize);
                    }
                }
                Instr::Return | Instr::Halt => {
                    // **VD-241 §3/T5 (A0, 2026-11-19)** — `Halt` giữ nguyên giá trị trên ngăn xếp
                    // (không ép về `()`); `Result::Err` của `main` đi thẳng ra `RunOutput.exit` ⇒ CLI exit 1.
                    let v = self.st.pop().unwrap_or(Value::Unit);
                    self.pop_frame();
                    match self.frames.last_mut() {
                        None => return Ok(v),
                        Some(f) => {
                            // Khôi phục ngăn xếp khung gọi rồi đẩy giá trị trả.
                            self.st = std::mem::take(&mut f.st_cu);
                            self.st.push(v);
                        }
                    }
                }
                Instr::MakeList(n) | Instr::MakeTuple(n) | Instr::MakeSet(n) => {
                    let items = self.pop_n(vm, n as usize, span)?;
                    self.st.push(Value::List(items));
                }
                Instr::MakeMap(n) => {
                    let flat = self.pop_n(vm, 2 * n as usize, span)?;
                    let mut pairs = Vec::with_capacity(n as usize);
                    let mut it = flat.into_iter();
                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                        pairs.push((k, v));
                    }
                    self.st.push(Value::Map(pairs));
                }
                Instr::MakeRecord(t, n) => {
                    let ten = vm.ten(t, span)?;
                    let flat = self.pop_n(vm, 2 * n as usize, span)?;
                    let mut fields = Vec::with_capacity(n as usize);
                    let mut it = flat.into_iter();
                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                        fields.push((k.show(), v));
                    }
                    self.st.push(Value::Record(ten, fields));
                }
                Instr::MakeEnum(v, n) => {
                    let duong = vm.ten(v, span)?;
                    let args = self.pop_n(vm, n as usize, span)?;
                    let (ten, bien_the) = match duong.rsplit_once('.') {
                        Some((a, b)) => (a.to_string(), b.to_string()),
                        None => (String::new(), duong),
                    };
                    self.st.push(Value::Enum(ten, bien_the, args));
                }
                Instr::MakeSome => {
                    let v = self.pop(vm, span)?;
                    // **VD-241 §3 (A0, 2026-11-19)**: `Some(x)` luôn thêm **đúng một lớp** —
                    // không giữ phẳng, không bọc chồng người dùng đã `Some(..)`; guard chọn
                    // vị trí này nằm ở emit-site (`opt_wrap`), không phải tại đây.
                    self.st.push(Value::some(v));
                }
                Instr::MakeNone => self.st.push(Value::none()),
                Instr::MakeOk => {
                    let v = self.pop(vm, span)?;
                    self.st.push(Value::ok(v));
                }
                Instr::MakeErr => {
                    let v = self.pop(vm, span)?;
                    self.st.push(Value::err(v));
                }
                Instr::MakeClosure(fi, n) => {
                    let cap = self.pop_n(vm, n as usize, span)?;
                    let func = Rc::new(VmFunc {
                        idx: fi,
                        captured: cap.clone(),
                    });
                    // **ST-238 (VD-242, ca 8)**: tạo ô chia sẻ ngay lúc tạo closure. Mọi lần gọi
                    // cùng closure dùng chung ô này ⇒ ghi biến bắt `var` không bị reset.
                    let ptr = Rc::as_ptr(&func) as usize;
                    vm.closure_cells.insert(ptr, Rc::new(RefCell::new(cap)));
                    self.st.push(Value::Vm(func));
                }
                Instr::Field(i) => {
                    let ten = vm.ten(i, span)?;
                    let v = self.pop(vm, span)?;
                    let v = field(&v, &ten, span)?;
                    self.st.push(v);
                }
                Instr::Index => {
                    let idx = self.pop(vm, span)?;
                    let base = self.pop(vm, span)?;
                    let v = index(&base, &idx, span)?;
                    self.st.push(v);
                }
                // **R8 (VD-178)**: `xs[i]?` — chỉ số **có kiểm tra**, trả `T?`.
                Instr::IndexOpt => {
                    let idx = self.pop(vm, span)?;
                    let base = self.pop(vm, span)?;
                    let v = index_opt(&base, &idx);
                    self.st.push(v);
                }
                Instr::Set => {
                    let v = self.pop(vm, span)?;
                    let idx = self.pop(vm, span)?;
                    let base = self.pop(vm, span)?;
                    let v = set(&base, &idx, v, span)?;
                    self.st.push(v);
                }
                Instr::With(n) => {
                    let flat = self.pop_n(vm, 2 * n as usize, span)?;
                    let mut pairs = Vec::with_capacity(n as usize);
                    let mut it = flat.into_iter();
                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                        pairs.push((k.show(), v));
                    }
                    let base = self.pop(vm, span)?;
                    let v = with(&base, pairs, span)?;
                    self.st.push(v);
                }
                Instr::Interp(n) => {
                    let parts = self.pop_n(vm, n as usize, span)?;
                    let mut s = String::new();
                    for p in parts {
                        s.push_str(&p.show());
                    }
                    self.st.push(Value::Str(s));
                }
                // **R6c (VD-178)**: định dạng đỉnh ngăn xếp theo spec `consts[i]` (`.N`).
                Instr::Format(i) => {
                    let v = self.pop(vm, span)?;
                    let spec = match vm.lit(i, span)? {
                        Value::Str(s) => s,
                        _ => return Err(vm.hong(VmError::BadIdx(i), span)),
                    };
                    let s = vis_interp::interp::dinh_dang(&v, &spec, span)?;
                    self.st.push(Value::Str(s));
                }
                Instr::IterInit => {
                    // **ST-238 (VD-242, ca 1)**: `IterInit` ngay trước `IterNext`; tính `pc` kết
                    // thúc vòng từ offset của `IterNext` để nhận diện `break` nhảy tới đó.
                    let fi = self.frames.last().expect("khung").fi;
                    let base = self.iters.len();
                    let next_pc = self.frames.last().expect("khung").pc;
                    if let Some(Instr::IterNext(d)) = self.m.funcs[fi as usize].code.get(next_pc) {
                        let end_pc = next_pc as i64 + i64::from(*d);
                        if end_pc >= 0 {
                            self.loop_ends.push((fi, end_pc as usize, base));
                        }
                    }
                    let it = self.pop(vm, span)?;
                    let items = value::iter_items(it, span)?;
                    self.iters.push((items, 0));
                }
                Instr::IterNext(d) => {
                    let Some((items, i)) = self.iters.last_mut() else {
                        return Err(vm.hong(VmError::StackUnderflow, span));
                    };
                    if *i >= items.len() {
                        self.iters.pop();
                        let pc = {
                            let fr = self.frames.last().expect("khung");
                            let pc = fr.pc as i64 - 1 + d as i64;
                            self.day(fr, pc, vm, span, "IterNext")?;
                            pc
                        };
                        self.dat_pc(pc as usize);
                    } else {
                        let v = items[*i].clone();
                        *i += 1;
                        self.st.push(v);
                    }
                }
                Instr::TestLit(i) => {
                    let want = vm.lit(i, span)?;
                    let v = self.pop(vm, span)?;
                    self.st.push(Value::Bool(value::eq(&v, &want)));
                }
                Instr::TestCtor(i, n) => {
                    let ten = vm.ten(i, span)?;
                    let v = self.pop(vm, span)?;
                    let var_muon = ten.rsplit('.').next().unwrap_or(&ten).to_string();
                    // **VD-212 (A0)**: `Some`/`None`/`Ok`/`Err` là biến thể dựng sẵn của enum
                    // `Option`/`Result` — so khớp biến thể **phân biệt hoa thường** (interp cũng vậy;
                    // pattern viết sai hoa thường là lỗi lúc biên dịch, không bao giờ chạm VM).
                    let ok = match &v {
                        Value::Enum(_, var, args) => *var == var_muon && args.len() == n as usize,
                        _ => false,
                    };
                    self.st.push(Value::Bool(ok));
                }
                Instr::TestTuple(n) | Instr::TestList(n) => {
                    let v = self.pop(vm, span)?;
                    let ok = match &v {
                        Value::List(xs) => xs.len() == n as usize,
                        // `()` khớp mẫu tuple rỗng.
                        Value::Unit => n == 0,
                        _ => false,
                    };
                    self.st.push(Value::Bool(ok));
                }
                Instr::TestRecord(_, n, open) => {
                    let v = self.pop(vm, span)?;
                    let ok =
                        matches!(&v, Value::Record(_, fs) if fs.len() == n as usize || open != 0);
                    self.st.push(Value::Bool(ok));
                }
                Instr::TestRange(a, b, incl) => {
                    let lo = vm.lit(a, span)?;
                    let hi = vm.lit(b, span)?;
                    let v = self.pop(vm, span)?;
                    let ok = match (&v, &lo, &hi) {
                        (Value::Int(x), Value::Int(l), Value::Int(h)) => {
                            if incl != 0 {
                                x >= l && x <= h
                            } else {
                                x >= l && x < h
                            }
                        }
                        _ => false,
                    };
                    self.st.push(Value::Bool(ok));
                }
                Instr::CallValue(n) => {
                    let args = self.pop_n(vm, n as usize, span)?;
                    let f = self.pop(vm, span)?;
                    let v = builtin::Host::call(vm, &f, args, span)?;
                    self.st.push(v);
                }
                Instr::IsUnit => {
                    let v = self.pop(vm, span)?;
                    self.st.push(Value::Bool(v.is_unit()));
                }
                Instr::PushUnit => self.st.push(Value::Unit),
                Instr::Swap => {
                    let b = self.pop(vm, span)?;
                    let a = self.pop(vm, span)?;
                    self.st.push(b);
                    self.st.push(a);
                }
                Instr::BoolAnd => {
                    let b = self.pop(vm, span)?;
                    let a = self.pop(vm, span)?;
                    self.st
                        .push(Value::Bool(a.truthy(span)? && b.truthy(span)?));
                }
                Instr::BoolOr => {
                    let b = self.pop(vm, span)?;
                    let a = self.pop(vm, span)?;
                    self.st
                        .push(Value::Bool(a.truthy(span)? || b.truthy(span)?));
                }
                Instr::UnwrapOpt => {
                    let v = self.pop(vm, span)?;
                    match v {
                        Value::Enum(ea, va, ag)
                            if ea == "Option" && va == "Some" && ag.len() == 1 =>
                        {
                            self.st
                                .push(ag.into_iter().next().expect("Some luôn có một đối số"));
                        }
                        other => self.st.push(other),
                    }
                }
                Instr::Unwrap(i) => {
                    let v = self.pop(vm, span)?;
                    let x = match &v {
                        Value::Enum(_, _, args) => args.get(i as usize).cloned(),
                        Value::List(xs) => xs.get(i as usize).cloned(),
                        _ => None,
                    };
                    // **VD-241 §3+ (A0, 2026-11-19)** — `pattern::test` phát `Unwrap` để **tính
                    // thử** đối số của arm trước khi arm được chọn: giá trị hỏng hình (ví dụ
                    // `None` gặp arm ctor `Some(…)`) **không được trap** — arm đó phải bị loại
                    // (test con ra `false`, `BoolAnd` giữ `false`), như interp khớp cấu trúc
                    // thật. Đẩy `Unit` khi hỏng hình; mọi test (`TestCtor`/`TestLit`/`TestRange`/
                    // `TestTuple`/`TestList`/`TestRecord`/or) đều cho `false` với `Unit`. Các chỗ
                    // bind/`let else` phát `Unwrap` **sau** `TestCtor(true)` nên không bao giờ
                    // chạm nhánh hỏng hình trong chương trình hợp lệ.
                    self.st.push(x.unwrap_or(Value::Unit));
                }
                Instr::Try => {
                    let v = self.pop(vm, span)?;
                    match &v {
                        Value::Enum(ea, va, _)
                            if (ea == "Option" && va == "None")
                                || (ea == "Result" && va == "Err") =>
                        {
                            self.pop_frame();
                            match self.frames.last_mut() {
                                None => return Ok(v),
                                Some(f) => {
                                    self.st = std::mem::take(&mut f.st_cu);
                                    self.st.push(v);
                                }
                            }
                        }
                        Value::Enum(ea, va, ag)
                            if (ea == "Option" && va == "Some" || ea == "Result" && va == "Ok")
                                && ag.len() == 1 =>
                        {
                            self.st.push(ag[0].clone());
                        }
                        other => self.st.push(other.clone()),
                    }
                }
            }
        }
    }
}

/// `kind` số của const pool → `LitKind`.
fn lit_kind(k: u8) -> Option<LitKind> {
    Some(match k {
        0 => LitKind::Int,
        1 => LitKind::Float,
        2 => LitKind::Size,
        3 => LitKind::Duration,
        4 => LitKind::Percent,
        5 => LitKind::Str,
        6 => LitKind::Bytes,
        7 => LitKind::Path,
        8 => LitKind::Char,
        9 => LitKind::Bool,
        _ => return None,
    })
}

fn field(v: &Value, name: &str, span: Span) -> R<Value> {
    // `.` trên `Some(x)` tự bóc (interp làm vậy) — `?.` mới là đường có kiểm `None`.
    // `.` trên `Some(x)` tự bóc (interp làm vậy) — `?.` mới là đường có kiểm `None`.
    if let Some(x) = v.opt_payload() {
        return field(x, name, span);
    }
    match v {
        Value::Record(_, fs) => fs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi không có trường `{name}`"),
                    span,
                ))
            }),
        Value::Enum(_, _, args) => {
            let i: usize = name.parse().map_err(|_| {
                Abort::Trap(Trap::new(RT, format!("`{name}` không phải trường"), span))
            })?;
            args.get(i)
                .cloned()
                .ok_or_else(|| Abort::Trap(Trap::new(RT, format!("không có đối số thứ {i}"), span)))
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`{}` không có trường `{name}`", other.show()),
            span,
        ))),
    }
}

fn index(base: &Value, idx: &Value, span: Span) -> R<Value> {
    // ST-30b: **trùng đúng** chẩn đoán của interp (cùng mã `VS0503`, cùng thông điệp) — hai đường
    // phải cho **cùng** danh sách `(mã, dòng, cột, thông điệp)`; trước đây VM dùng `VS0501` + câu ngắn.
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) => {
            let len = xs.len() as i64;
            if *i < 0 {
                return Err(Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số âm `{i}` (dùng `.last()` hoặc `xs[i]?`)"),
                    span,
                )));
            }
            xs.get(*i as usize).cloned().ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số {i} ngoài phạm vi (độ dài {len})"),
                    span,
                ))
            })
        }
        (Value::Bytes(bs), Value::Int(i)) => {
            let len = bs.len() as i64;
            if *i < 0 || *i >= len {
                return Err(Abort::Trap(Trap::new(
                    IDX,
                    format!("chỉ số {i} ngoài phạm vi (độ dài {len})"),
                    span,
                )));
            }
            Ok(Value::Int(bs[*i as usize] as i64))
        }
        (Value::Map(kv), k) => kv
            .iter()
            .find(|(kk, _)| value::eq(kk, k))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    IDX,
                    format!("khoá `{}` không có trong map", k.show()),
                    span,
                ))
            }),
        (other, idx) => Err(Abort::Trap(Trap::new(
            RT,
            format!("không chỉ số được `{}` bằng `{}`", other.show(), idx.show()),
            span,
        ))),
    }
}

fn set(base: &Value, idx: &Value, v: Value, span: Span) -> R<Value> {
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) => {
            let mut xs = xs.clone();
            if (*i as usize) < xs.len() {
                xs[*i as usize] = v;
                Ok(Value::List(xs))
            } else {
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("chỉ số {i} ngoài phạm vi"),
                    span,
                )))
            }
        }
        (Value::Map(kv), k) => {
            let mut kv = kv.clone();
            if let Some(e) = kv.iter_mut().find(|(kk, _)| value::eq(kk, k)) {
                e.1 = v;
            } else {
                kv.push((k.clone(), v));
            }
            Ok(Value::Map(kv))
        }
        _ => Err(Abort::Trap(Trap::new(RT, "gán chỉ số không hợp lệ", span))),
    }
}

fn with(base: &Value, pairs: Vec<(String, Value)>, span: Span) -> R<Value> {
    match base {
        Value::Record(ten, fs) => {
            let mut fs = fs.clone();
            for (k, v) in pairs {
                if let Some(e) = fs.iter_mut().find(|(kk, _)| *kk == k) {
                    e.1 = v;
                } else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        format!("bản ghi `{ten}` không có trường `{k}`"),
                        span,
                    )));
                }
            }
            Ok(Value::Record(ten.clone(), fs))
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`with` không dùng được cho `{}`", other.show()),
            span,
        ))),
    }
}

/// **R8 (VD-178)**: `xs[i]?` — trả `Some(v)` nếu trong phạm vi, `None` nếu ngoài (không panic).
fn index_opt(base: &Value, idx: &Value) -> Value {
    match (base, idx) {
        (Value::List(xs), Value::Int(i)) if *i >= 0 => Value::opt(xs.get(*i as usize).cloned()),
        (Value::Bytes(bs), Value::Int(i)) if *i >= 0 => {
            Value::opt(bs.get(*i as usize).map(|b| Value::Int(*b as i64)))
        }
        (Value::Map(kv), k) => Value::opt(
            kv.iter()
                .find(|(kk, _)| kk.eq_value(k))
                .map(|(_, v)| v.clone()),
        ),
        _ => Value::none(),
    }
}
