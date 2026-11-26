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
mod params;
mod proc;
mod worker;
pub(crate) use worker::run_frame_driver;

use expr::{call_value, dispatch, eval_work, exec_work};
use params::{env_for_params, khoi_tao_bien_cap_tep_frame, opt_depth_ty};
use proc::process_work;

pub(crate) use vis_core::frame::{FrameConfig, FrameOutput};

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
    /// `let … else:` — tên trần bóc `Ok`/`Some`; pattern bác được khớp trên toàn giá trị;
    /// không khớp ⇒ hoàn nguyên binding rồi chạy khối `else` (VD-263/VD-265).
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
        base: &'a HExpr,
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
        /// **ST-301**: số lớp `Opt` của annot từng tham số (0 = không nâng).
        param_depths: Vec<usize>,
        ret_depth: usize,
        body: HBody,
    },
    /// Gán biến `var` (đã eval rhs) — dùng cho closure mutation/writeback.
    ApplyAssignVar {
        sym: u32,
        op: AssignOp,
        span: Span,
    },
    /// **VD-270** — đích gán **đường** (`a.b.f`): đi xuống từng bước / ghi sau vế phải.
    AssignPathStep(expr::AssignPathStep<'a>),
    ApplyAssignPath(expr::ApplyAssignPath),

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
                Value::Enum(__en) => {
                    let (_, variant, vargs) = (&__en.enum_name, &__en.variant, &__en.args);
                    {
                        if variant != &name {
                            return false;
                        }
                        bind_all_env(args, vargs, env)
                    }
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
                    let tail = Value::list(xs[items.len()..].to_vec());
                    bind_pat_env(r, &tail, env)
                } else {
                    xs.len() == items.len()
                }
            }
            _ => false,
        },
        HPat::Record { fields, .. } => match v {
            Value::Record(r) => {
                let vfields = &r.fields;
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
        Value::Record(r) => r
            .fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .ok_or_else(|| {
                Abort::Trap(Trap::new(
                    RT,
                    format!("bản ghi không có trường `{name}`"),
                    span,
                ))
            }),
        Value::Enum(e) if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 => {
            field_of(
                e.args.get(0).expect("Some luôn có một đối số").clone(),
                name,
                span,
            )
        }
        Value::Enum(e) => {
            if e.variant.is_empty() && e.args.is_empty() {
                Ok(Value::enum_val(
                    e.enum_name.clone(),
                    Name::new(name),
                    Vec::new(),
                ))
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
        (Value::Map(kv), k) => kv.get(&k).cloned().ok_or_else(|| {
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
fn type_name(it: &Interp<'_>, ty: &vis_hir::Type) -> String {
    match ty {
        vis_hir::Type::Record(id) => it
            .types
            .rec(*id)
            .and_then(|r| r.name.clone())
            .unwrap_or_default(),
        vis_hir::Type::Enum(id) => it
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
