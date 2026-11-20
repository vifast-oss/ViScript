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

use vis_syntax::{Diagnostic, Span};
use vis_types::hir::*;

use crate::interp::{lit_value, Abort, Interp, Trap, R, RT};
use crate::rt::{self, DiagSnapshot, FakeEvent, FrameReactorBackend, ReactorEvent, Waiter};
use crate::value::{Env, ForIter, Name, Value};

mod expr;
mod params;
mod proc;

use expr::{call_value, dispatch, eval_work, exec_work};
use params::{env_for_params, khoi_tao_bien_cap_tep_frame, opt_depth_ty};
use proc::process_work;

/// Cấu hình frame test — `fake_sources`: `(tên hàm async, khóa event)`; `events`: trace.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub struct FrameConfig {
    pub fake_sources: Vec<(String, u64)>,
    pub events: Vec<FakeEvent>,
    /// **Lát 2**: trace reactor giả (read/accept/timer). `seq` quyết định thứ tự fire.
    #[doc(hidden)]
    pub reactors: Vec<ReactorEvent>,
    /// **Lát 2**: `true` ⇒ khi rảnh, ngủ một lần tới deadline monotonic gần nhất (đo B1).
    #[doc(hidden)]
    pub timer_real_wait: bool,
    /// **Lát 3 (chỉ test)**: adapter selector thật. `None` ⇒ đường reactor giả Lát 2.
    #[doc(hidden)]
    pub backend: Option<std::rc::Rc<std::cell::RefCell<dyn FrameReactorBackend>>>,
}

/// Kết quả frame test — kèm log scheduler (tất định) + snapshot ownership.
#[doc(hidden)]
#[derive(Debug)]
pub struct FrameOutput {
    pub stdout: String,
    pub diagnostics: Vec<Diagnostic>,
    pub value: Option<Value>,
    pub exit: i32,
    pub log: Vec<String>,
    pub snapshot: DiagSnapshot,
}

/// Một work-item của IFrame phải làm tiếp. `Eval`/`Exec` đẩy continuation,
/// các biến thể `Apply*` là continuation tiêu thụ giá trị trên `vals`.
enum Work<'a> {
    // ---- nền tảng (Lát 1) ----
    Eval(&'a HExpr),
    Exec(&'a HStmt),
    /// Sau khi operand của `await` thành `Value::Task`, gọi `await_task` thật.
    ApplyAwait(Span),
    /// `lhs`/`rhs` đã trên `vals` (theo thứ tự); áp dụng phép toán thật.
    ApplyBin {
        op: BinOp,
        span: Span,
    },
    /// Đánh giá tuần tự đối số (vị trí + tên) rồi chạy `cont`.
    EvalArgsCont {
        args: &'a [HExpr],
        named: &'a [(String, HExpr)],
        idx: usize,
        span: Span,
        cont: Vec<Work<'a>>,
    },
    /// Gán giá trị trên `vals` vào pattern của `let` (kèm độ sâu `Opt` của annot).
    Bind {
        pat: &'a HPat,
        opt_depth: usize,
        /// VD-257/D: span của câu `let` — dùng làm `site` cho E_ASSIGN.
        span: Span,
    },
    /// `let … else:` — bóc `Ok`/`Some`, ngược lại chạy khối `else`.
    ApplyLetElse {
        pat: &'a HPat,
        else_body: &'a [HStmt],
        bind_sym: Option<u32>,
        opt_depth: usize,
        span: Span,
    },
    /// Gán giá trị trên `vals` vào ô `var` (sym), kèm độ sâu `Opt` của annot.
    SetVar {
        sym: u32,
        opt_depth: usize,
        span: Span,
    },
    /// Đặt `last` = giá trị trên `vals`.
    SetLast,
    /// Đặt `last = None` (đầu mỗi câu lệnh / kết thúc khối không có giá trị).
    ResetLast,
    /// Đẩy `last` (hoặc `Unit`) lên `vals` — cuối khối biểu thức.
    PushLast,
    /// `return <value>`: đặt `last` rồi kết thúc thân khung hiện tại.
    DoReturn,

    // ---- Lát 2: retry builtin mà reactor báo Pending ----
    Redispatch {
        callee: Callee<'a>,
        args: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    },

    // ---- VD-255 / Group A: điều khiển ----
    /// Vào khối câu lệnh lồng (nhánh if/arm match/thân hàm/scope).
    ExecBlock(&'a [HStmt]),
    /// Đánh giá khối biểu thức rồi đẩy giá trị (`last`) lên `vals`.
    EvalBlock(&'a [HStmt]),
    /// Đánh giá `HCond`, chạy `on_true` nếu đúng, `on_false` nếu sai.
    EvalCond {
        cond: &'a HCond,
        on_true: Vec<Work<'a>>,
        on_false: Vec<Work<'a>>,
        span: Span,
    },
    /// Áp dụng kết quả bool của `HCond::Expr` (đã nằm trên `vals`).
    ApplyCond {
        on_true: Vec<Work<'a>>,
        on_false: Vec<Work<'a>>,
        span: Span,
    },
    /// Áp dụng `HCond::Let`: khớp pattern, giữ binding nếu khớp, hoàn nguyên nếu không.
    ApplyCondLet {
        pat: &'a HPat,
        on_true: Vec<Work<'a>>,
        on_false: Vec<Work<'a>>,
    },
    /// Khởi tạo vòng lặp: tạo `LoopState`, đẩy `LoopMark` + bước đầu.
    LoopInit {
        kind: LoopKindInit<'a>,
        body: &'a [HStmt],
        span: Span,
    },
    /// Đáywork của một lần lặp: thân chạy xong ⇒ bước tiếp theo.
    LoopMark {
        id: usize,
    },
    /// Vào thân một lần lặp (đẩy LoopMark + các câu lệnh thân).
    LoopEnter {
        id: usize,
    },
    /// Một bước tiến vòng (thử điều kiện / lấy phần tử / vào thân).
    LoopStep {
        id: usize,
    },
    /// Kết thúc vòng lặp (last = Unit, pop LoopState).
    LoopExit {
        id: usize,
    },
    /// `break` — thoát vòng trong cùng.
    DoBreak {
        span: Span,
    },
    /// `continue` — sang lần lặp tiếp của vòng trong cùng.
    DoContinue {
        span: Span,
    },

    // ---- VD-255 / Group B: biểu thức & gọi ----
    ApplyUn {
        op: UnOp,
        span: Span,
    },
    ApplyField {
        name: String,
        optional: bool,
        span: Span,
    },
    ApplyIndex {
        span: Span,
    },
    BuildList {
        n: usize,
    },
    BuildSet {
        n: usize,
    },
    BuildTuple {
        n: usize,
    },
    BuildMap {
        n: usize,
    },
    /// Đánh giá từng giá trị trường của record literal theo thứ tự nguồn.
    EvalRecordFields {
        fields: &'a [(String, HExpr)],
        idx: usize,
        span: Span,
    },
    BuildRecord {
        name: String,
        fields: &'a [(String, HExpr)],
        span: Span,
    },
    BuildEnum {
        name: String,
        variant: String,
        n: usize,
        span: Span,
    },
    ApplySome,
    InterpParts {
        parts: &'a [InterpPart],
        idx: usize,
        out: String,
        span: Span,
    },
    InterpPartVal {
        parts: &'a [InterpPart],
        idx: usize,
        out: String,
        spec: Option<String>,
        span: Span,
    },
    ApplyShortCircuit {
        op: BinOp,
        rhs: &'a HExpr,
        span: Span,
    },
    ApplyBoolRhs {
        span: Span,
    },
    ApplyCoalesce {
        rhs: &'a HExpr,
    },
    ApplyPipe {
        rhs: &'a HExpr,
        span: Span,
    },
    ApplyPipeCall {
        lhs: Value,
        span: Span,
    },
    /// Chọn nhánh `match`: thử từng arm (pattern + guard).
    MatchArm {
        arms: &'a [HArm],
        idx: usize,
        subj: Value,
        stmt_mode: bool,
        span: Span,
    },
    MatchGuard {
        arms: &'a [HArm],
        idx: usize,
        subj: Value,
        stmt_mode: bool,
        snapshot: HashMap<u32, Value>,
        span: Span,
    },
    /// Gọi hàm/builtin tên sau khi đối số đã eval (`names` = tên đối số theo thứ tự).
    ApplyNamedCall {
        callee: Callee<'a>,
        npos: usize,
        names: Vec<String>,
        span: Span,
    },
    /// Gọi một **giá trị hàm** (closure/builtin value) sau khi callee + đối số đã eval.
    ApplyValueCall {
        npos: usize,
        names: Vec<String>,
        span: Span,
    },
    /// Retry một giá trị hàm đã eval đối số mà builtin báo `Pending`.
    RedispatchValue {
        f: Value,
        args: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    },
    /// Gán `subject` đã eval vào work `MatchArm` tương ứng.
    MatchSubj {
        arms: &'a [HArm],
        stmt_mode: bool,
        span: Span,
    },
    /// Thu base + các giá trị trường của `with` rồi dựng bản ghi mới.
    WithCollect {
        names: Vec<String>,
        n: usize,
        span: Span,
    },
    /// Base của `callee.field(...)`: nhận diện method vs field-holding-closure lúc chạy.
    ApplyMethodOrField {
        name: String,
        args: &'a [HExpr],
        named: &'a [(String, HExpr)],
        base: &'a HExpr,
        span: Span,
    },
    /// Gọi phương thức lõi trên list/str/map/chan (recv đã nằm trong work).
    ApplyMethod {
        name: String,
        recv: Value,
        nargs: usize,
        base_var: Option<u32>,
        span: Span,
    },
    /// Tạo closure: chụp **env chia sẻ** hiện tại.
    MakeLambda {
        params: Vec<u32>,
        ret_depth: usize,
        body: HBody,
    },
    /// Gán biến `var` (đã eval rhs) — dùng cho closure mutation/writeback.
    ApplyAssignVar {
        sym: u32,
        op: AssignOp,
        span: Span,
    },
    /// Gán trường/phần tử của biến `var`.
    ApplyAssignField {
        sym: u32,
        name: String,
        op: AssignOp,
        span: Span,
    },
    ApplyAssignIndex {
        sym: u32,
        op: AssignOp,
        span: Span,
    },

    // ---- VD-257 / stream A: `?`, kênh, cmd-block ----
    /// `?` trên `Option`/`Result`: bóc `Some`/`Ok`, hoặc **trả sớm** `None`/`Err`
    /// từ khung hiện tại (short-circuit — mọi work còn lại của khung bị bỏ).
    ApplyTry {
        span: Span,
    },
    /// `xs[i]?` — chỉ số **có kiểm tra** ⇒ `T?` (không trap, VD-178 R8).
    ApplyTryIndex {
        span: Span,
    },
    /// `chan[T](n)` — tạo kênh sau khi dung lượng đã eval (`$kenh` thật).
    BuildChan {
        span: Span,
    },
    /// Retry phương thức host (`send`/`recv`/…) sau `Abort::Pending`: đối số đã eval
    /// được giữ lại trong work ⇒ resume **không eval lại** (không lặp side-effect).
    /// **VD-258**: phương thức không resume được (Pending lần nữa) ⇒ trap rõ, không
    /// chạy lại toàn bộ phương thức (tránh lặp side-effect của callback).
    RedispatchMethod {
        name: String,
        recv: Value,
        args: Vec<Value>,
        base_var: Option<u32>,
        span: Span,
    },
    /// **VD-258**: vòng `list.map/filter/each` **resume được** — giữ `idx` + `acc`
    /// qua suspend, gọi callback tuần tự; `awaiting` ⇒ giá trị callback nằm trên `vals`.
    MethodLoop {
        kind: MethodKind,
        f: Value,
        items: Vec<Value>,
        idx: usize,
        acc: Vec<Value>,
        awaiting: bool,
        span: Span,
    },
}

/// **VD-258**: ba phương thức list nhận callback (trước đây chạy qua `host.call`
/// trong một lần gọi Rust ⇒ không suspend/resume được).
#[derive(Clone, Copy, PartialEq, Eq)]
enum MethodKind {
    Map,
    Filter,
    Each,
}

#[derive(Clone, Copy)]
enum Callee<'a> {
    /// Hàm khai trong tệp — `(sym, là_async, tên)`.
    Named(u32, bool, &'a str),
    /// `spawn f(...)` — tạo + **xếp vào hàng đợi sẵn sàng**.
    Spawn(u32, &'a str),
    /// Builtin (tên dựng sẵn).
    Builtin(&'a str),
}

/// Dữ liệu khởi tạo của một vòng lặp.
enum LoopKindInit<'a> {
    While { cond: &'a HCond },
    For { pat: &'a HPat, iter: &'a HExpr },
    Loop,
}

/// Trạng thái vòng lặp sống trên heap (resume không mất).
struct LoopState<'a> {
    kind: LoopKindState<'a>,
    body: &'a [HStmt],
    span: Span,
}

enum LoopKindState<'a> {
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

struct CallFrame<'a> {
    /// Môi trường **chia sẻ** (ô bắt của closure trỏ vào chính `Rc` này).
    env: Env,
    /// Số lớp `Opt` của kiểu trả về (nâng ở biên hàm như interp).
    ret_depth: usize,
    /// Continuation stack — mọi trạng thái sống qua suspend.
    work: Vec<Work<'a>>,
    /// Giá trị câu lệnh gần nhất (giá trị khối / giá trị trả hàm).
    last: Option<Value>,
    /// Các vòng đang mở (trong cùng nằm cuối).
    loops: Vec<LoopState<'a>>,
    /// Binding tham số cần hoàn nguyên khi khung kết thúc (closure gọi lại nhiều lần).
    restore: Vec<(u32, Option<Value>)>,
    /// **VD-258**: env đích để gộp thay đổi của closure sau khi khung xong (mỗi lần gọi
    /// có env riêng ⇒ tham số không đè lên lần gọi đang treo khác).
    merge: Option<Env>,
}

/// **Frame thật của interp trên heap** (đối trọng với `Th` của VM).
pub struct IFrame<'a> {
    calls: Vec<CallFrame<'a>>,
    vals: Vec<Value>,
    /// Tập `var` dùng chung toàn chương trình (khớp `Interp.mutable` — closure thấy `var` của ngoài).
    mutable: Rc<RefCell<HashSet<u32>>>,
    /// **VD-258**: env cấp tệp (const/hằng) — hàm tên phải thấy như interp thật.
    globals: Env,
}

impl<'a> IFrame<'a> {
    fn new(body: &'a [HStmt], env: Env, globals: &Env, mutable: Rc<RefCell<HashSet<u32>>>) -> Self {
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
    fn expr_frame(env: &Env, mutable: Rc<RefCell<HashSet<u32>>>) -> Self {
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

    fn done(&self) -> bool {
        self.calls.is_empty()
    }

    fn cur(&mut self) -> &mut CallFrame<'a> {
        self.calls.last_mut().expect("khung hiện tại")
    }

    fn push_val(&mut self, v: Value) {
        self.vals.push(v);
    }

    fn pop_val(&mut self) -> R<Value> {
        self.vals.pop().ok_or_else(underflow)
    }
}

/// Trả work của một khối theo **thứ tự thực thi**: ResetLast rồi Exec cho mỗi câu.
/// `append_work` chạy theo đúng thứ tự phần tử trong vec (phần tử đầu chạy trước).
fn block_works<'a>(body: &'a [HStmt]) -> Vec<Work<'a>> {
    let mut w = Vec::with_capacity(body.len() * 2);
    for s in body.iter() {
        w.push(Work::ResetLast);
        w.push(Work::Exec(s));
    }
    w
}

/// Vec work **sẵn sàng gán trực tiếp** vào `cf.work` (LIFO — đảo thứ tự thực thi).
fn initial_work<'a>(body: &'a [HStmt]) -> Vec<Work<'a>> {
    let mut w = block_works(body);
    w.reverse();
    w
}

/// Nối `works` vào đỉnh `cf.work` sao cho chạy theo thứ tự trong `works`.
fn append_work<'a>(cf: &mut CallFrame<'a>, mut works: Vec<Work<'a>>) {
    while let Some(w) = works.pop() {
        cf.work.push(w);
    }
}

/// Khớp pattern và ghi binding vào `env` (mirror `pat.rs` nhưng dùng env của khung).
fn bind_pat_env(pat: &HPat, v: &Value, env: &Env) -> bool {
    match pat {
        HPat::Wild { .. } | HPat::Rest { .. } => true,
        HPat::Bind { sym, .. } => {
            env.borrow_mut().insert(*sym, v.clone());
            true
        }
        HPat::Lit { lit, .. } => lit_value(lit).map(|l| l.eq_value(v)).unwrap_or(false),
        HPat::Or { alts, .. } => {
            for a in alts {
                let snap = env.borrow().clone();
                if bind_pat_env(a, v, env) {
                    return true;
                }
                *env.borrow_mut() = snap;
            }
            false
        }
        HPat::Ctor { path, args, .. } => {
            let name = path.last().cloned().unwrap_or_default();
            let low = name.to_ascii_lowercase();
            match v {
                Value::Enum(_, variant, vargs) => {
                    if variant != &name {
                        return false;
                    }
                    bind_all_env(args, vargs, env)
                }
                Value::Bool(b) => match low.as_str() {
                    "true" => *b,
                    "false" => !*b,
                    _ => false,
                },
                _ => false,
            }
        }
        HPat::Tuple { items, .. } => match v {
            Value::Unit if items.is_empty() => true,
            Value::List(xs) if xs.len() == items.len() => items
                .iter()
                .zip(xs.iter())
                .all(|(p, x)| bind_pat_env(p, x, env)),
            _ => false,
        },
        HPat::List { items, rest, .. } => match v {
            Value::List(xs) if xs.len() >= items.len() => {
                for (p, x) in items.iter().zip(xs.iter()) {
                    if !bind_pat_env(p, x, env) {
                        return false;
                    }
                }
                if let Some(r) = rest {
                    let tail = Value::List(xs[items.len()..].to_vec());
                    bind_pat_env(r, &tail, env)
                } else {
                    xs.len() == items.len()
                }
            }
            _ => false,
        },
        HPat::Record { fields, .. } => match v {
            Value::Record(_, vfields) => {
                for (n, sub) in fields {
                    if n.is_empty() {
                        continue;
                    }
                    let Some((_, val)) = vfields.iter().find(|(f, _)| f == n) else {
                        return false;
                    };
                    if let Some(p) = sub {
                        if !bind_pat_env(p, val, env) {
                            return false;
                        }
                    }
                }
                true
            }
            _ => false,
        },
        HPat::Range {
            lo, hi, inclusive, ..
        } => {
            let lit_of = |p: &HPat| -> Option<Value> {
                if let HPat::Lit { lit, .. } = p {
                    lit_value(lit).ok()
                } else {
                    None
                }
            };
            let (Some(a0), Some(b0)) = (
                lo.as_ref().and_then(|p| lit_of(p)),
                hi.as_ref().and_then(|p| lit_of(p)),
            ) else {
                return false;
            };
            let (Some(a), Some(b)) = (v.cmp_value(&a0), v.cmp_value(&b0)) else {
                return false;
            };
            let ok_lo = a != std::cmp::Ordering::Less;
            let ok_hi = if *inclusive {
                b != std::cmp::Ordering::Greater
            } else {
                b == std::cmp::Ordering::Less
            };
            ok_lo && ok_hi
        }
    }
}

fn bind_all_env(pats: &[HPat], vals: &[Value], env: &Env) -> bool {
    if pats.len() != vals.len() {
        return false;
    }
    pats.iter()
        .zip(vals.iter())
        .all(|(p, v)| bind_pat_env(p, v, env))
}

/// Truy cập trường (mirror `Interp::field_of`).
fn field_of(b: Value, name: &str, span: Span) -> R<Value> {
    match b {
        Value::Record(_, fields) => fields
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi không có trường `{name}`"),
                    span,
                ))
            }),
        Value::Enum(ea, va, args) if ea == "Option" && va == "Some" && args.len() == 1 => {
            field_of(args.into_iter().next().unwrap(), name, span)
        }
        Value::Enum(en, variant, args) => {
            if variant.is_empty() && args.is_empty() {
                Ok(Value::Enum(en, Name::new(name), Vec::new()))
            } else {
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("`{name}` không phải trường"),
                    span,
                )))
            }
        }
        other => Err(Abort::Trap(Trap::new(
            RT,
            format!("`{}` không có trường `{name}`", other.show()),
            span,
        ))),
    }
}

/// Chỉ số (mirror `Interp::index_of`).
fn index_of(b: Value, i: Value, span: Span) -> R<Value> {
    match (b, i) {
        (Value::List(xs), Value::Int(n)) => {
            let len = xs.len() as i64;
            if n < 0 {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::IDX,
                    format!("chỉ số âm `{n}` (dùng `.last()` hoặc `xs[i]?`)"),
                    span,
                )));
            }
            xs.get(n as usize).cloned().ok_or_else(|| {
                Abort::Trap(Trap::new(
                    crate::interp::IDX,
                    format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                    span,
                ))
            })
        }
        (Value::Map(kv), k) => kv
            .iter()
            .find(|(kk, _)| kk.eq_value(&k))
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    crate::interp::IDX,
                    format!("khoá `{}` không có trong map", k.show()),
                    span,
                ))
            }),
        (Value::Bytes(bs), Value::Int(n)) => {
            let len = bs.len() as i64;
            if n < 0 || n >= len {
                return Err(Abort::Trap(Trap::new(
                    crate::interp::IDX,
                    format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                    span,
                )));
            }
            Ok(Value::Int(bs[n as usize] as i64))
        }
        (other, idx) => Err(Abort::Trap(Trap::new(
            RT,
            format!("không chỉ số được `{}` bằng `{}`", other.show(), idx.show()),
            span,
        ))),
    }
}

/// Tên kiểu record/enum từ `TypeTable` (mirror `Interp::type_name`).
fn type_name(it: &Interp<'_>, ty: &vis_types::Type) -> String {
    match ty {
        vis_types::Type::Record(id) => it
            .types
            .rec(*id)
            .and_then(|r| r.name.clone())
            .unwrap_or_default(),
        vis_types::Type::Enum(id) => it
            .types
            .enum_def(*id)
            .map(|e| e.name.clone())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// Chạy chương trình thật qua frame scheduler. Gọi từ integration test sau khi
/// parse/lower/check — **không** có CLI flag/API ngôn ngữ nào gọi tới đây.
#[doc(hidden)]
pub fn run_frame_for_test<'a>(
    text: &str,
    hir: &'a vis_types::hir::Hir,
    symbols: &'a vis_types::lower::Symbols,
    types: &'a vis_types::TypeTable,
    args: &[String],
    config: FrameConfig,
) -> FrameOutput {
    let mut it = Interp::new(hir, symbols, types);
    let _ = symbols;

    // **VD-255 / Group B**: hằng cấp tệp được đánh giá trước main (khớp `interp.rs:229`).
    // Dùng chung một `Env` để const sau thấy const trước; mỗi frame con clone env làm gốc.
    let globals: Env = Rc::new(RefCell::new(HashMap::new()));
    let mutable: Rc<RefCell<HashSet<u32>>> = Rc::new(RefCell::new(HashSet::new()));
    let consts: Vec<(u32, &'a HExpr)> = hir
        .items
        .iter()
        .filter_map(|item| match item {
            HItem::Const(c) => Some((c.sym, &c.value)),
            _ => None,
        })
        .collect();
    for (sym, e) in consts {
        match eval_expr_in(&mut it, e, &globals, &mutable) {
            Ok(v) => {
                globals.borrow_mut().insert(sym, v);
            }
            Err(Abort::Trap(t)) => return frame_output_early(&mut it, text, None, Some(t)),
            Err(_) => return frame_output_early(&mut it, text, None, None),
        }
    }

    let mut frames: HashMap<u32, IFrame<'a>> = HashMap::new();
    let main_fn = hir.items.iter().find_map(|i| match i {
        HItem::Fn(f) if f.name == "main" => Some(f),
        _ => None,
    });
    // **VD-262**: biến cấp tệp là global — khai báo `let`/`var` chạy trước `main`; không có
    // `main` thì để thân cấp tệp tự chạy như cũ (tránh thi hành hai lần).
    if main_fn.is_some() {
        if let Some(t) = khoi_tao_bien_cap_tep_frame(&mut it, hir, &globals, &mutable) {
            return frame_output_early(&mut it, text, None, Some(t));
        }
    }
    let body: &'a [HStmt] = main_fn.map(|f| f.body.as_slice()).unwrap_or(&hir.stmts);
    let argv = Value::List(args.iter().map(|s| Value::Str(s.clone())).collect());
    // **VD-259**: `main` dùng **cùng** helper tham số/default với `dispatch`. Bản cũ chỉ đưa
    // argv cho tham số 1 và `Unit` cho mọi tham số còn lại ⇒ `b: int = 7` bị `Unit`.
    let main_env = match main_fn {
        Some(f) => {
            let pos = if f.params.is_empty() {
                Vec::new()
            } else {
                vec![argv]
            };
            match env_for_params(
                &mut it,
                &f.params,
                &pos,
                &[],
                &globals,
                &mutable,
                &f.name,
                f.span,
            ) {
                Ok(env) => env,
                Err(Abort::Trap(t)) => return frame_output_early(&mut it, text, None, Some(t)),
                Err(_) => return frame_output_early(&mut it, text, None, None),
            }
        }
        // **VD-259**: không có `main` ⇒ chạy câu lệnh cấp tệp như runtime mặc định, KHÔNG
        // gắn argv vào một ô `u32::MAX` (fallback cũ khiến mọi tên lạ nhận danh sách args).
        None => Rc::new(RefCell::new(globals.borrow().clone())),
    };
    frames.insert(0, IFrame::new(body, main_env, &globals, mutable.clone()));

    it.sched
        .borrow_mut()
        .frame_enable_for_test(config.fake_sources, config.events);
    it.sched
        .borrow_mut()
        .frame_reactor_load_for_test(config.reactors, config.timer_real_wait);
    if let Some(b) = config.backend {
        it.sched.borrow_mut().frame_install_backend_for_test(b);
    }

    let mut current: u32 = 0;
    let mut final_value: Option<Value> = None;
    let mut trap: Option<Trap> = None;
    let mut guard = 0usize;
    loop {
        guard += 1;
        if guard > 200_000 {
            trap = Some(Trap::new(
                RT,
                "frame test: vòng lặp scheduler quá giới hạn",
                Span::new(0, 0),
            ));
            break;
        }
        it.sched.borrow_mut().frame.current = current;
        // **VD-258**: task đang chạy phải mang `DangChay` (trước đây không bao giờ đặt)
        // ⇒ `await` chính nó bị phát hiện thay vì treo im lặng.
        if current != 0 {
            if let Some(t) = it
                .sched
                .borrow()
                .tasks
                .iter()
                .find(|t| t.id == current)
                .cloned()
            {
                t.trang_thai.set(rt::TrangThai::DangChay);
            }
        }
        // VD-257/D — reset hạn ngạch ĐẦU MỖI TURN (không đụng ngân sách tổng).
        crate::rt::fee::quota_reset_for_test();
        let outcome = step_frame(&mut it, current, &mut frames);
        match outcome {
            Ok(v) => {
                // **VD-259**: frame đã terminal ⇒ bỏ khỏi map ngay (không giữ `IFrame`/env
                // của nó tới cuối chương trình).
                frames.remove(&current);
                if current == 0 {
                    final_value = v;
                    // Không `break`: main xong ⇒ final join các task không ai await.
                } else {
                    let id = current;
                    let val = v.unwrap_or(Value::Unit);
                    if let Some(t) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(val));
                        t.trang_thai.set(rt::TrangThai::Xong);
                    }
                    it.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
                }
            }
            Err(Abort::Trap(t)) => {
                if current == 0 {
                    frames.remove(&current);
                    trap = Some(t);
                    break;
                }
                let id = current;
                frames.remove(&id);
                let wrapped = rt::wrap_task_error_for_test(
                    &it.sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.ten.clone())
                        .unwrap_or_default(),
                    t,
                    it.sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.span)
                        .unwrap_or_else(|| Span::new(0, 0)),
                );
                if let Some(tk) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                    *tk.kq.borrow_mut() = Some(Err(wrapped.clone()));
                    tk.trang_thai.set(rt::TrangThai::Xong);
                }
                it.sched.borrow_mut().frame_task_done(id, Err(wrapped));
            }
            Err(Abort::Ret(v)) => {
                frames.remove(&current);
                if current == 0 {
                    // `return`/`?` ở `main` vẫn phải final-join các task còn lại (không bỏ).
                    final_value = Some(v);
                } else {
                    let id = current;
                    if let Some(t) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                        *t.kq.borrow_mut() = Some(Ok(v));
                        t.trang_thai.set(rt::TrangThai::Xong);
                    }
                    it.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
                }
            }
            Err(Abort::Pending(p)) => {
                // **VD-258**: wait không có task/wait = yield quota; frame vẫn sẵn sàng.
                let quota_yield = p.task.is_none() && p.wait.is_none();
                // **VD-258**: `await` chính tác vụ đang chạy (hoặc một task `DangChay`)
                // không bao giờ có event đánh thức ⇒ trap rõ thay vì treo/exit 0.
                if let Some(t) = &p.task {
                    if t.id == current || t.trang_thai.get() == rt::TrangThai::DangChay {
                        trap = Some(Trap::new(
                            crate::interp::DEADLOCK,
                            "deadlock: tác vụ đang chờ chính nó (await chính tác vụ đang chạy)",
                            p.span,
                        ));
                        break;
                    }
                }
                if current != 0 {
                    if let Some(t) = it
                        .sched
                        .borrow()
                        .tasks
                        .iter()
                        .find(|t| t.id == current)
                        .cloned()
                    {
                        t.trang_thai.set(rt::TrangThai::Pending);
                    }
                }
                let w: Option<Waiter> = if let Some(w) = p.wait {
                    let gen = w.gen_of(it.sched.borrow().frame.gen);
                    Some(Waiter {
                        frame: current,
                        wait: w,
                        gen,
                    })
                } else {
                    p.task
                        .as_ref()
                        .and_then(|t| it.sched.borrow().frame_await(current, t))
                };
                if let Some(w) = w {
                    it.sched.borrow_mut().frame_register(w);
                } else {
                    it.sched.borrow_mut().frame.ready.push_back(current);
                }
                if quota_yield {
                    // Yield quota là tiến độ hợp lệ, không phải scheduler quay vô ích.
                    guard = guard.saturating_sub(1);
                }
            }
        }

        // Lên lịch task `Moi` khi không còn frame sẵn sàng — kể cả task tạo bằng `tao`
        // (gọi `async fn` trực tiếp, không `spawn`) đang được `await`. Khớp VM; nếu không,
        // main treo khi await một task thường. **VD-258**: CHỈ `Moi` (chưa chạy) — task
        // `Pending` đang chờ event không được đẩy lại ready (quay vô ích tới guard).
        let need_join = true;
        let next = it.sched.borrow_mut().frame_next();
        if it.sched.borrow().frame.selector_error.is_some() {
            break;
        }
        let next = match next {
            Some(n) => Some(n),
            None if need_join => {
                let pending: Vec<u32> = {
                    let s = it.sched.borrow();
                    s.tasks
                        .iter()
                        .filter(|t| t.trang_thai.get() == rt::TrangThai::Moi)
                        .filter(|t| !s.frame.external.contains_key(&t.id))
                        .map(|t| t.id)
                        .collect()
                };
                for id in pending {
                    if !it.sched.borrow().frame.ready.contains(&id) {
                        it.sched.borrow_mut().frame.ready.push_back(id);
                    }
                }
                it.sched.borrow_mut().frame_next()
            }
            None => None,
        };
        // **VD-260**: bỏ frame của task đã `Xong`/`Huy` ngay tại safe point kế tiếp —
        // không giữ `IFrame` + env tới cuối chương trình. `next` (nếu có) luôn còn sống
        // nên được giữ lại; frame mồ côi (task đã bị thu hồi) cũng bị bỏ.
        let purged = purge_terminal_frames(&mut frames, &it.sched.borrow(), next);
        if !purged.is_empty() {
            let mut s = it.sched.borrow_mut();
            for id in &purged {
                s.frame.log.push(format!("purge:{}", id));
            }
            s.frame.live_frames = frames.len();
        }
        match next {
            Some(n) => {
                match frames.entry(n) {
                    std::collections::hash_map::Entry::Occupied(_) => {}
                    std::collections::hash_map::Entry::Vacant(e) => {
                        match build_task_frame(&mut it, n, &globals, &mutable) {
                            Ok(Some((body, env))) => {
                                e.insert(IFrame::new(body, env, &globals, mutable.clone()));
                            }
                            Ok(None) => {
                                // Nguồn giả / task ngoài — không có thân để dựng frame. Bỏ qua,
                                // GIỮ NGUYÊN `current` (không gán rồi `continue` như cũ, vốn
                                // gây panic `step_frame` ở vòng sau vì frame không tồn tại).
                                continue;
                            }
                            Err(Abort::Trap(t)) => {
                                trap = Some(t);
                                break;
                            }
                            Err(e) => {
                                // Default tham số cần suspend (hoặc `return`) — chưa có
                                // continuation cho bước dựng frame; báo rõ thay vì chạy sai.
                                let span = match &e {
                                    Abort::Pending(p) => p.span,
                                    _ => Span::new(0, 0),
                                };
                                trap = Some(Trap::new(
                                    RT,
                                    "frame test: không dựng được tham số/default của task",
                                    span,
                                ));
                                break;
                            }
                        }
                    }
                }
                current = n;
            }
            None => break,
        }
    }

    if trap.is_none() {
        if let Some(e) = it.sched.borrow_mut().frame_take_selector_error() {
            trap = Some(Trap::new(RT, e, Span::new(0, 0)));
        }
    }
    // **VD-259/VD-260**: frame của task đã `Xong`/`Huy` không còn là trạng thái treo — bỏ
    // trước khi đo `live_frames` (và đã bỏ ngay tại safe point trong vòng lặp). Chỉ frame
    // thật sự Pending mới còn trong `frames`.
    let purged = purge_terminal_frames(&mut frames, &it.sched.borrow(), None);
    if !purged.is_empty() {
        let mut s = it.sched.borrow_mut();
        for id in &purged {
            // Hậu tố `-end`: frame chỉ được dọn ở cuối (không kịp safe point) — khác với
            // `purge:<id>` giữa vòng lặp, để test phân biệt được retention.
            s.frame.log.push(format!("purge-end:{}", id));
        }
    }
    it.sched.borrow_mut().frame.live_frames = frames.len();
    frames.clear();
    it.sched.borrow_mut().frame_drain_events();
    // **ST-278 (VD-256)**: lỗi task cache ở final join phải surfaced (cùng hợp đồng VS0506).
    let final_join_err = it.sched.borrow_mut().frame_final_join();
    if trap.is_none() {
        trap = final_join_err;
    }
    // **VD-258**: không còn frame sẵn sàng mà vẫn có waiter/task chưa terminal ⇒ deadlock
    // thật (event không bao giờ tới), KHÔNG được trả exit 0 như thành công.
    if trap.is_none() {
        trap = it.sched.borrow().frame_stuck_trap();
    }
    // **VD-259**: đo **trước** `frame_reactor_shutdown` — nếu đo sau, regs/timers luôn 0 và
    // snapshot không còn là bằng chứng vòng đời.
    let snapshot = it.sched.borrow().diag_snapshot();
    it.sched.borrow_mut().frame_reactor_shutdown();
    let mut log = it.sched.borrow().frame.log.clone();
    log.extend(it.sched.borrow().frame.reactor.log.clone());
    let stdout = it.take_stdout();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(text)],
        None => Vec::new(),
    };
    // **VD-258**: exit theo giá trị cuối của `main` (`int`/`Result.Err`), không luôn 0.
    let exit = match (&trap, &final_value) {
        (Some(_), _) => 1,
        (None, Some(v)) => crate::value::exit_code(v),
        (None, None) => 0,
    };
    FrameOutput {
        stdout,
        diagnostics,
        value: final_value,
        exit,
        log,
        snapshot,
    }
}

/// **VD-260**: bỏ frame của task đã terminal (`Xong`/`Huy`) hoặc không còn task tương ứng
/// (frame mồ côi). `keep` là frame vừa được chọn chạy — không bao giờ bỏ. Main (id 0) luôn
/// giữ. Trả danh sách id đã bỏ để driver ghi log tất định.
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
                .map(|t| matches!(t.trang_thai.get(), rt::TrangThai::Xong | rt::TrangThai::Huy))
                .unwrap_or(true)
        })
        .collect();
    for id in &dead {
        frames.remove(id);
    }
    dead
}

/// Trả `FrameOutput` sớm khi đánh giá hằng cấp tệp lỗi (trước khi scheduler chạy).
fn frame_output_early(
    it: &mut Interp<'_>,
    text: &str,
    value: Option<Value>,
    trap: Option<Trap>,
) -> FrameOutput {
    let stdout = it.take_stdout();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(text)],
        None => Vec::new(),
    };
    FrameOutput {
        stdout,
        diagnostics,
        value,
        exit: if trap.is_some() { 1 } else { 0 },
        log: Vec::new(),
        snapshot: Default::default(),
    }
}

/// Đánh giá một biểu thức trong `env` cho trước (dùng cho hằng/default tham số).
fn eval_expr_in<'a>(
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

/// Dựng frame cho task `id` từ HIR thật (thân `async fn`).
#[allow(clippy::type_complexity)]
fn build_task_frame<'a>(
    it: &mut Interp<'a>,
    id: u32,
    globals: &Env,
    mutable: &Rc<RefCell<HashSet<u32>>>,
) -> R<Option<(&'a [HStmt], Env)>> {
    let (f, args, span, ten) = {
        let s = it.sched.borrow();
        let Some(t) = s.tasks.iter().find(|t| t.id == id) else {
            return Ok(None);
        };
        if s.frame.external.contains_key(&id) {
            return Ok(None);
        }
        let Some(f) = it.fns.get(&t.than).copied() else {
            return Ok(None);
        };
        (f, t.args.clone(), t.span, t.ten.clone())
    };
    // **VD-259**: cùng helper tham số/default với `dispatch` — task `async` có default không
    // còn nhận `Unit` khi gọi thiếu đối số.
    let env = env_for_params(it, &f.params, &args, &[], globals, mutable, &ten, span)?;
    Ok(Some((&f.body, env)))
}

/// Một bước chạy frame: tiêu thụ work cho tới khi frame xong, treo (`Pending`), hoặc lỗi.
fn step_frame<'a>(
    it: &mut Interp<'a>,
    frame_id: u32,
    frames: &mut HashMap<u32, IFrame<'a>>,
) -> R<Option<Value>> {
    let mut frame = frames.remove(&frame_id).expect("frame hiện tại");
    let res = step_frame_owned(it, &mut frame);
    frames.insert(frame_id, frame);
    res
}

fn step_frame_owned<'a>(it: &mut Interp<'a>, frame: &mut IFrame<'a>) -> R<Option<Value>> {
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

/// Dựng danh sách work cho if/elif/else (dùng cho cả stmt lẫn expr).
fn if_work<'a>(
    cond: &'a HCond,
    then: &'a [HStmt],
    elifs: &'a [(HCond, Vec<HStmt>)],
    else_: Option<&'a [HStmt]>,
    span: Span,
    stmt_mode: bool,
) -> Vec<Work<'a>> {
    let branch = |body: &'a [HStmt]| -> Vec<Work<'a>> {
        if stmt_mode {
            block_works(body)
        } else {
            vec![Work::EvalBlock(body)]
        }
    };
    let mut on_false: Vec<Work<'a>> = match else_ {
        Some(b) => branch(b),
        None => {
            if stmt_mode {
                vec![Work::ResetLast]
            } else {
                vec![Work::PushLast] // else rỗng ⇒ Unit
            }
        }
    };
    for (c, body) in elifs.iter().rev() {
        let on_true = branch(body);
        on_false = vec![Work::EvalCond {
            cond: c,
            on_true,
            on_false,
            span,
        }];
    }
    let on_true = branch(then);
    vec![Work::EvalCond {
        cond,
        on_true,
        on_false,
        span,
    }]
}

fn args_works<'a>(
    args: &'a [HExpr],
    named: &'a [(String, HExpr)],
    span: Span,
    cont: Vec<Work<'a>>,
) -> Work<'a> {
    Work::EvalArgsCont {
        args,
        named,
        idx: 0,
        span,
        cont,
    }
}
fn run_arm_body<'a>(frame: &mut IFrame<'a>, arm: &'a HArm, stmt_mode: bool) {
    if stmt_mode {
        append_work(frame.cur(), block_works(&arm.body));
    } else {
        frame.cur().work.push(Work::EvalBlock(&arm.body));
    }
}

fn pop_n(frame: &mut IFrame<'_>, n: usize) -> R<Vec<Value>> {
    let mut vs = Vec::with_capacity(n);
    for _ in 0..n {
        vs.push(frame.pop_val()?);
    }
    Ok(vs)
}

/// Lấy `(pos, named)` đã eval từ `vals` theo thứ tự nguồn; `names` là tên đối số theo thứ tự.
#[allow(clippy::type_complexity)]
fn take_call_args_names(
    frame: &mut IFrame<'_>,
    npos: usize,
    names: &[String],
) -> R<(Vec<Value>, Vec<(String, Value)>)> {
    let mut all = Vec::with_capacity(npos + names.len());
    for _ in 0..(npos + names.len()) {
        all.push(frame.pop_val()?);
    }
    all.reverse();
    let pos = all[..npos].to_vec();
    let named = all[npos..]
        .iter()
        .cloned()
        .zip(names.iter().cloned())
        .map(|(v, n)| (n, v))
        .collect();
    Ok((pos, named))
}

fn underflow() -> Abort {
    Abort::Trap(Trap::new(
        RT,
        "frame test: ngăn xếp giá trị rỗng",
        Span::new(0, 0),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **VD-258**: mỗi lần gọi closure phải có env **riêng** — tham số của lần gọi đang
    /// treo không bị lần gọi khác ghi đè/xoá. Trước đây mọi lần gọi dùng chung `Closure.env`.
    #[test]
    fn closure_moi_lan_goi_env_rieng() {
        let hir = vis_types::Hir {
            items: Vec::new(),
            stmts: Vec::new(),
            span: Span::new(0, 0),
        };
        let symbols = vis_types::Symbols::default();
        let types = vis_types::TypeTable::default();
        let mut it = Interp::new(&hir, &symbols, &types);

        let env: Env = Rc::new(RefCell::new(HashMap::new()));
        env.borrow_mut().insert(99, Value::Int(0)); // biến bắt tượng trưng
        let mk = || {
            Value::Fn(Rc::new(crate::value::Closure {
                name: "f".into(),
                params: vec![7],
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
        let hir = vis_types::Hir {
            items: Vec::new(),
            stmts: Vec::new(),
            span: Span::new(0, 0),
        };
        let symbols = vis_types::Symbols::default();
        let types = vis_types::TypeTable::default();
        let mut it = Interp::new(&hir, &symbols, &types);
        let env: Env = Rc::new(RefCell::new(HashMap::new()));
        env.borrow_mut()
            .insert(u32::MAX, Value::List(vec![Value::Str("argv".into())]));
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
