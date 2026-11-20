// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-267 / VD-251 — LÁT 1 (chỉ test/debug)**: frame-aware scheduler nối vào
//! **đường interp thật**.
//!
//! Đây **không** phải model/harness độc lập: `run_frame_for_test` dựng đúng [`Interp`]
//! thật (HIR thật sau parse/lower/check), dùng [`crate::builtin::call`], [`crate::ops::bin`],
//! [`crate::value::Value`], [`crate::interp::Trap`] và [`crate::rt::Scheduler`] thật. Điểm khác
//! duy nhất là **chiến lược đánh giá**: thay vì Rust call stack (`eval`/`exec_stmt` đệ quy),
//! frame chạy trên một **work stack nằm heap** (`IFrame`) ⇒ lưu được vị trí giữa biểu thức
//! khi `await` treo. Runtime mặc định (`Interp::run_program`) **không** gọi module này.
//!
//! Giới hạn (ghi rõ, xem báo cáo): evaluator chỉ phủ tập HExpr/HStmt cần cho fixture
//! Lát 1 — literal, `Var`, `Bin`, `Call`/`Await`/`Spawn` với callee là tên hàm/builtin,
//! `Let`/`Var`/`Expr`/`Return`. Các vị trí await checker vẫn chấp nhận (xem SPEC/VD-250
//! §3) như `&&`/`||`, vòng lặp, field/index, callee là closure… **không** được phủ ở
//! đường test này — checker vẫn chấp nhận đầy đủ như trước.

use std::collections::{HashMap, HashSet};

use vis_syntax::{Diagnostic, Span};
use vis_types::hir::*;

use crate::interp::{lit_value, Abort, Interp, Trap, R, RT};
use crate::rt::{self, DiagSnapshot, FakeEvent, ReactorEvent, Waiter};
use crate::value::Value;

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

/// Cái một work-item của IFrame phải làm tiếp. `Eval`/`Exec` đẩy tiếp continuation,
/// các biến thể `Apply*` là continuation tiêu thụ giá trị trên `vals`.
enum Work<'a> {
    Eval(&'a HExpr),
    Exec(&'a HStmt),
    /// Sau khi operand của `await` thành `Value::Task`, gọi `await_task` thật.
    ApplyAwait(Span),
    /// `lhs`/`rhs` đã trên `vals` (theo thứ tự); áp dụng phép toán thật.
    ApplyBin {
        op: BinOp,
        span: Span,
    },
    /// Đánh giá tuần tự đối số của call/spawn, rồi dispatch.
    EvalArgs {
        callee: Callee<'a>,
        args: &'a [HExpr],
        idx: usize,
        span: Span,
    },
    /// Gán giá trị trên `vals` vào pattern của `let`.
    Bind(&'a HPat),
    /// Gán giá trị trên `vals` vào ô `var` (sym).
    SetVar(u32),
    /// Đặt `last` = giá trị trên `vals`.
    SetLast,
    /// `return <value>`: đặt `last` rồi kết thúc thân khung hiện tại.
    DoReturn,
    /// **Lát 2**: retry một builtin đã eval đối số mà reactor báo `Pending` — không eval
    /// lại đối số (tránh lặp side-effect), resume đúng vị trí work stack.
    Redispatch {
        callee: Callee<'a>,
        args: Vec<Value>,
        span: Span,
    },
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

struct CallFrame<'a> {
    body: &'a [HStmt],
    pc: usize,
    env: HashMap<u32, Value>,
    mutable: HashSet<u32>,
    work: Vec<Work<'a>>,
    last: Option<Value>,
}

/// **Frame thật của interp trên heap** (đối trọng với `Th` của VM). Mọi field sống
/// qua suspend: `calls` (ngăn xếp khung gọi + vị trí `pc` + work stack), `vals`.
pub struct IFrame<'a> {
    calls: Vec<CallFrame<'a>>,
    vals: Vec<Value>,
}

impl<'a> IFrame<'a> {
    fn new(body: &'a [HStmt], params: &[(u32, Value)], globals: &HashMap<u32, Value>) -> Self {
        let mut env = globals.clone();
        for (sym, v) in params {
            env.insert(*sym, v.clone());
        }
        Self {
            calls: vec![CallFrame {
                body,
                pc: 0,
                env,
                mutable: HashSet::new(),
                work: Vec::new(),
                last: None,
            }],
            vals: Vec::new(),
        }
    }

    fn done(&self) -> bool {
        self.calls.is_empty()
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
    let _ = symbols; // chỉ cần để giữ lifetime khớp `Interp::new`
                     // Hằng cấp tệp không nằm trong tập phủ của frame test (fixture Lát 1 không dùng).
    let globals = HashMap::new();
    // **VD-252 / ST-268 — từ chối rõ hằng cấp tệp**: đường frame không chạy vòng eval const
    // của `run_program` (`interp.rs:229`), nên `it.env` rỗng. Nhận diện const từ HIR để báo
    // đúng lý do thay vì "biến chưa định nghĩa" gây hiểu lầm. Không đổi checker.
    let mut const_syms: HashSet<u32> = HashSet::new();
    let mut const_names: HashSet<String> = HashSet::new();
    for item in &hir.items {
        if let HItem::Const(c) = item {
            const_syms.insert(c.sym);
            const_names.insert(c.name.clone());
        }
    }
    let mut frames: HashMap<u32, IFrame<'a>> = HashMap::new();
    let main_fn = hir.items.iter().find_map(|i| match i {
        HItem::Fn(f) if f.name == "main" => Some(f),
        _ => None,
    });
    let body: &'a [HStmt] = main_fn.map(|f| f.body.as_slice()).unwrap_or(&hir.stmts);
    let argv: Vec<(u32, Value)> = match main_fn {
        Some(f) => f
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                (
                    p.sym,
                    if i == 0 {
                        Value::List(args.iter().map(|s| Value::Str(s.clone())).collect())
                    } else {
                        Value::Unit
                    },
                )
            })
            .collect(),
        None => vec![(
            u32::MAX,
            Value::List(args.iter().map(|s| Value::Str(s.clone())).collect()),
        )],
    };
    frames.insert(0, IFrame::new(body, &argv, &globals));

    it.sched
        .borrow_mut()
        .frame_enable_for_test(config.fake_sources, config.events);
    it.sched
        .borrow_mut()
        .frame_reactor_load_for_test(config.reactors, config.timer_real_wait);

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
        let outcome = step_frame(
            &mut it,
            current,
            &mut frames,
            body,
            &const_syms,
            &const_names,
        );
        match outcome {
            Ok(v) => {
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
                    trap = Some(t);
                    break;
                }
                let id = current;
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
                if current == 0 {
                    final_value = Some(v);
                    break;
                }
                let id = current;
                if let Some(t) = it.sched.borrow().tasks.iter().find(|t| t.id == id).cloned() {
                    *t.kq.borrow_mut() = Some(Ok(v));
                    t.trang_thai.set(rt::TrangThai::Xong);
                }
                it.sched.borrow_mut().frame_task_done(id, Ok(Value::Unit));
            }
            Err(Abort::Pending(p)) => {
                // Lát 2: reactor mang wait trực tiếp (`p.wait`); Lát 1: `await` mang task.
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
                    // Task đã xong giữa lúc chạy: nối frame lại vào đuôi ready.
                    it.sched.borrow_mut().frame.ready.push_back(current);
                }
            }
        }

        // Chọn frame tiếp theo. Nếu main vừa xong ⇒ **final join**: xếp mọi task còn
        // sống vào đuôi ready theo thứ tự tạo (không auto-cancel — VD-250 §2.3).
        let need_join = frames.get(&0).map(|f| f.done()).unwrap_or(true);
        let next = it.sched.borrow_mut().frame_next();
        let next = match next {
            Some(n) => Some(n),
            None if need_join => {
                let pending: Vec<(u32, u32)> = {
                    let s = it.sched.borrow();
                    s.tasks.iter().map(|t| (t.id, t.id)).collect()
                };
                for (_, id) in pending {
                    let s = it.sched.borrow();
                    let is_moi = s
                        .tasks
                        .iter()
                        .find(|t| t.id == id)
                        .map(|t| t.trang_thai.get() == rt::TrangThai::Moi)
                        .unwrap_or(false);
                    drop(s);
                    if is_moi && !it.sched.borrow().frame.ready.contains(&id) {
                        it.sched.borrow_mut().frame.ready.push_back(id);
                    }
                }
                it.sched.borrow_mut().frame_next()
            }
            None => None,
        };
        match next {
            Some(n) => {
                current = n;
                if let std::collections::hash_map::Entry::Vacant(e) = frames.entry(n) {
                    // Task chưa có frame (lần đầu được chạy): dựng frame thật từ HIR.
                    let setup = build_task_frame(&it, n);
                    if let Some((body, params)) = setup {
                        let f = IFrame::new(body, &params, &globals);
                        e.insert(f);
                    } else {
                        // Nguồn giả: không có thân — bỏ qua (sẽ xong qua event).
                        continue;
                    }
                }
            }
            None => break,
        }
    }

    // Main kết thúc ⇒ mọi env/handle của chương trình biến mất. Bỏ frame trước khi
    // thu hồi để `Rc::strong_count` phản ánh đúng "không còn handle sống".
    frames.clear();
    // Fire nốt trace (duplicate/stale vẫn được log) rồi final join + thu hồi ownership.
    it.sched.borrow_mut().frame_drain_events();
    it.sched.borrow_mut().frame_final_join();
    // Lát 2: sau khi task + handle test thả, reactor về baseline (không cần đóng tay).
    it.sched.borrow_mut().frame_reactor_shutdown();
    let snapshot = it.sched.borrow().diag_snapshot();
    // Gộp log reactor vào log scheduler để hai engine so sánh được TOÀN BỘ trace.
    let mut log = it.sched.borrow().frame.log.clone();
    log.extend(it.sched.borrow().frame.reactor.log.clone());
    let stdout = it.take_stdout();
    let diagnostics = match &trap {
        Some(t) => vec![t.to_diag(text)],
        None => Vec::new(),
    };
    let exit = if trap.is_some() { 1 } else { 0 };
    FrameOutput {
        stdout,
        diagnostics,
        value: final_value,
        exit,
        log,
        snapshot,
    }
}

/// Dựng frame cho task `id` từ HIR thật (thân `async fn`).
#[allow(clippy::type_complexity)]
fn build_task_frame<'a>(it: &Interp<'a>, id: u32) -> Option<(&'a [HStmt], Vec<(u32, Value)>)> {
    let s = it.sched.borrow();
    let t = s.tasks.iter().find(|t| t.id == id)?;
    if s.frame.external.contains_key(&id) {
        return None;
    }
    let f = it.fns.get(&t.than).copied()?;
    let mut params = Vec::new();
    for (i, p) in f.params.iter().enumerate() {
        params.push((p.sym, t.args.get(i).cloned().unwrap_or(Value::Unit)));
    }
    Some((&f.body, params))
}

/// Một bước chạy frame: tiêu thụ work/statement cho tới khi frame xong, treo (`Pending`),
/// hoặc lỗi. Trạng thái nằm hoàn toàn trong `IFrame` trên heap ⇒ resume không lặp side effect.
fn step_frame<'a>(
    it: &mut Interp<'a>,
    frame_id: u32,
    frames: &mut HashMap<u32, IFrame<'a>>,
    _main_body: &'a [HStmt],
    const_syms: &HashSet<u32>,
    const_names: &HashSet<String>,
) -> R<Option<Value>> {
    // Mượn frame ra khỏi map để không giữ RefCell/map borrow qua các lệnh gọi host.
    let mut frame = frames.remove(&frame_id).expect("frame hiện tại");
    let res = step_frame_owned(it, frame_id, &mut frame, const_syms, const_names);
    frames.insert(frame_id, frame);
    res
}

fn step_frame_owned<'a>(
    it: &mut Interp<'a>,
    _frame_id: u32,
    frame: &mut IFrame<'a>,
    const_syms: &HashSet<u32>,
    const_names: &HashSet<String>,
) -> R<Option<Value>> {
    loop {
        if frame.done() {
            // Giá trị frame = giá trị đã eval cuối cùng nằm trên `vals`.
            return Ok(frame.vals.pop());
        }

        let work = {
            let cf = frame.calls.last_mut().expect("khung");
            cf.work.pop()
        };
        let Some(w) = work else {
            let cf = frame.calls.last_mut().expect("khung");
            if cf.pc >= cf.body.len() {
                // Hết work **và** hết câu lệnh ⇒ kết thúc khung con: đẩy giá trị trả
                // vào `vals` rồi pop. (Không được pop sớm: work cuối có thể còn nằm lại.)
                let cf = frame.calls.pop().expect("khung");
                let ret = cf.last.unwrap_or(Value::Unit);
                frame.vals.push(ret);
                continue;
            }
            // Không còn work ⇒ chạy câu lệnh tiếp theo.
            let s = &cf.body[cf.pc];
            cf.pc += 1;
            cf.work.push(Work::Exec(s));
            continue;
        };

        match w {
            Work::Exec(s) => {
                let cf = frame.calls.last_mut().expect("khung");
                match s {
                    HStmt::Let { pat, init, .. } => {
                        cf.work.push(Work::Bind(pat));
                        cf.work.push(Work::Eval(init));
                    }
                    HStmt::Var { sym, init, .. } => {
                        cf.work.push(Work::SetVar(*sym));
                        cf.work.push(Work::Eval(init));
                    }
                    HStmt::Expr(e) => {
                        cf.work.push(Work::SetLast);
                        cf.work.push(Work::Eval(e));
                    }
                    HStmt::Return { value, .. } => {
                        cf.work.push(Work::DoReturn);
                        if let Some(e) = value {
                            cf.work.push(Work::Eval(e));
                        } else {
                            frame.vals.push(Value::Unit);
                        }
                    }
                    other => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!(
                                "frame test: HStmt {:?} chưa được phủ ở đường test Lát 1",
                                std::mem::discriminant(other)
                            ),
                            s.span(),
                        )))
                    }
                }
            }
            Work::Bind(pat) => {
                let v = frame.vals.pop().ok_or_else(underflow)?;
                let cf = frame.calls.last_mut().expect("khung");
                match pat {
                    HPat::Bind { sym, .. } => {
                        cf.env.insert(*sym, v);
                    }
                    HPat::Wild { .. } => {}
                    other => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!(
                                "frame test: pattern {:?} chưa được phủ",
                                std::mem::discriminant(other)
                            ),
                            pat.span(),
                        )))
                    }
                }
            }
            Work::SetVar(sym) => {
                let v = frame.vals.pop().ok_or_else(underflow)?;
                let cf = frame.calls.last_mut().expect("khung");
                cf.env.insert(sym, v);
                cf.mutable.insert(sym);
            }
            Work::SetLast => {
                let v = frame.vals.pop().ok_or_else(underflow)?;
                frame.calls.last_mut().expect("khung").last = Some(v);
            }
            Work::DoReturn => {
                let v = frame.vals.pop().ok_or_else(underflow)?;
                {
                    let cf = frame.calls.last_mut().expect("khung");
                    cf.last = Some(v);
                    cf.pc = cf.body.len();
                }
            }
            Work::Eval(e) => {
                let cf = frame.calls.last_mut().expect("khung");
                match e {
                    HExpr::Lit { lit, .. } => {
                        let v =
                            lit_value(lit).map_err(|m| Abort::Trap(Trap::new(RT, m, e.span())))?;
                        frame.vals.push(v);
                    }
                    HExpr::Var { sym, name, .. } => {
                        let v = sym
                            .and_then(|s| cf.env.get(&s).cloned())
                            .or_else(|| {
                                cf.env
                                    .iter()
                                    .find(|(s, _)| **s == u32::MAX)
                                    .map(|(_, v)| v.clone())
                            })
                            .or_else(|| it.env.borrow().get(&sym.unwrap_or(u32::MAX)).cloned())
                            .ok_or_else(|| {
                                // VD-252/ST-268: hằng cấp tệp là cấu trúc KHÔNG được đường frame
                                // test phủ — báo đúng lý do, không đổ lỗi "biến chưa định nghĩa".
                                let is_const = sym.map(|s| const_syms.contains(&s)).unwrap_or(false)
                                    || const_names.contains(name);
                                let msg = if is_const {
                                    format!(
                                        "frame test: hằng cấp tệp `{name}` chưa được đường frame test hỗ trợ"
                                    )
                                } else {
                                    format!("biến `{name}` chưa định nghĩa trong frame test")
                                };
                                Abort::Trap(Trap::new(RT, msg, e.span()))
                            })?;
                        frame.vals.push(v);
                    }
                    HExpr::Bin { op, lhs, rhs, .. } => {
                        // VD-252/ST-268: `&&`/`||`/`??`/`|>` là short-circuit — `ops::bin` từ chối
                        // chúng, nhưng nếu đẩy operand trước thì vế phải (có side-effect) chạy mất
                        // rồi mới lỗi. Từ chối TẠI ĐÂY, trước khi đẩy operand nào.
                        if matches!(op, BinOp::And | BinOp::Or | BinOp::Coalesce | BinOp::Pipe) {
                            return Err(Abort::Trap(Trap::new(
                                RT,
                                format!(
                                    "frame test: toán tử `{}` (short-circuit) chưa được đường frame test hỗ trợ",
                                    op.text()
                                ),
                                e.span(),
                            )));
                        }
                        cf.work.push(Work::ApplyBin {
                            op: *op,
                            span: e.span(),
                        });
                        cf.work.push(Work::Eval(rhs));
                        cf.work.push(Work::Eval(lhs));
                    }
                    HExpr::Un { .. } => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "frame test: Un chưa được phủ",
                            e.span(),
                        )))
                    }
                    HExpr::Call {
                        callee,
                        args,
                        named,
                        ..
                    } => {
                        if !named.is_empty() {
                            return Err(Abort::Trap(Trap::new(
                                RT,
                                "frame test: đối số tên chưa được phủ",
                                e.span(),
                            )));
                        }
                        if let Some(c) = classify_callee(it, callee) {
                            cf.work.push(Work::EvalArgs {
                                callee: c,
                                args,
                                idx: 0,
                                span: e.span(),
                            });
                        } else {
                            return Err(Abort::Trap(Trap::new(
                                RT,
                                "frame test: callee không phải tên hàm/builtin",
                                e.span(),
                            )));
                        }
                    }
                    HExpr::Spawn {
                        callee,
                        args,
                        named,
                        ..
                    } => {
                        if !named.is_empty() {
                            return Err(Abort::Trap(Trap::new(
                                RT,
                                "frame test: spawn đối số tên chưa được phủ",
                                e.span(),
                            )));
                        }
                        match &**callee {
                            HExpr::Var {
                                sym: Some(id),
                                name,
                                ..
                            } => {
                                let f = it.fns.get(id).copied().ok_or_else(|| {
                                    Abort::Trap(Trap::new(
                                        RT,
                                        "frame test: spawn hàm không tồn tại",
                                        e.span(),
                                    ))
                                })?;
                                let _ = name;
                                let c = Callee::Spawn(*id, &f.name);
                                cf.work.push(Work::EvalArgs {
                                    callee: c,
                                    args,
                                    idx: 0,
                                    span: e.span(),
                                });
                            }
                            _ => {
                                return Err(Abort::Trap(Trap::new(
                                    RT,
                                    "frame test: spawn cần tên hàm async cấp tệp",
                                    e.span(),
                                )))
                            }
                        }
                    }
                    HExpr::Await { expr, .. } => {
                        cf.work.push(Work::ApplyAwait(e.span()));
                        cf.work.push(Work::Eval(expr));
                    }
                    other => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!(
                                "frame test: HExpr {:?} chưa được phủ ở đường test Lát 1",
                                std::mem::discriminant(other)
                            ),
                            other.span(),
                        )))
                    }
                }
            }
            Work::ApplyBin { op, span } => {
                let r = frame.vals.pop().ok_or_else(underflow)?;
                let l = frame.vals.pop().ok_or_else(underflow)?;
                let v = crate::ops::bin(op, l, r, span)?;
                frame.vals.push(v);
            }
            Work::ApplyAwait(span) => {
                let v = frame.vals.pop().ok_or_else(underflow)?;
                let t = match v {
                    Value::Task(t) => t,
                    _ => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "`await` cần một `Task[T]`",
                            span,
                        )))
                    }
                };
                match crate::rt::await_task(it, &t, span) {
                    Ok(r) => frame.vals.push(r),
                    Err(Abort::Pending(p)) => {
                        // Khôi phục **nguyên vẹn** vị trí + operand: resume không eval lại
                        // operand của `await`, không lặp side effect.
                        frame
                            .calls
                            .last_mut()
                            .expect("khung")
                            .work
                            .push(Work::ApplyAwait(span));
                        frame.vals.push(Value::Task(t));
                        return Err(Abort::Pending(p));
                    }
                    Err(e) => return Err(e),
                }
            }
            Work::EvalArgs {
                callee,
                args,
                idx,
                span,
            } => {
                if idx < args.len() {
                    let cf = frame.calls.last_mut().expect("khung");
                    cf.work.push(Work::EvalArgs {
                        callee,
                        args,
                        idx: idx + 1,
                        span,
                    });
                    cf.work.push(Work::Eval(&args[idx]));
                    continue;
                }
                let mut got = Vec::with_capacity(args.len());
                for _ in 0..args.len() {
                    got.push(frame.vals.pop().ok_or_else(underflow)?);
                }
                got.reverse();
                match dispatch(it, frame, callee, got.clone(), span) {
                    Ok(()) => {}
                    Err(Abort::Pending(p)) => {
                        // Reactor chưa sẵn: lưu continuation với đối số ĐÃ eval (không eval lại).
                        frame
                            .calls
                            .last_mut()
                            .expect("khung")
                            .work
                            .push(Work::Redispatch {
                                callee,
                                args: got,
                                span,
                            });
                        return Err(Abort::Pending(p));
                    }
                    Err(e) => return Err(e),
                }
            }
            Work::Redispatch { callee, args, span } => {
                for a in &args {
                    frame.vals.push(a.clone());
                }
                match dispatch(it, frame, callee, args.clone(), span) {
                    Ok(()) => {}
                    Err(Abort::Pending(p)) => {
                        frame
                            .calls
                            .last_mut()
                            .expect("khung")
                            .work
                            .push(Work::Redispatch { callee, args, span });
                        return Err(Abort::Pending(p));
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

/// Tên builtin được frame evaluator chấp nhận: 8 tên ngắn (`SIGS`) **và** mọi hàm
/// `std.*` đã đăng ký (`vis_types::stdlib`) — trong đó có `std.net.read`/`accept`.
/// Chỉ mở rộng nhận diện ở đường test; checker/runtime mặc định không đổi.
fn is_builtin_name(name: &str) -> bool {
    crate::builtin::SIGS.iter().any(|s| s.name == name)
        || name
            .strip_prefix("std.")
            .and_then(vis_types::stdlib::sig_of)
            .is_some()
}

fn classify_callee<'a>(it: &Interp<'a>, callee: &'a HExpr) -> Option<Callee<'a>> {
    match callee {
        HExpr::Var {
            sym: Some(id),
            name,
            ..
        } => it
            .fns
            .get(id)
            .copied()
            .map(|f| {
                if f.is_async {
                    Callee::Named(*id, true, &f.name)
                } else {
                    Callee::Named(*id, false, &f.name)
                }
            })
            .or_else(|| {
                if is_builtin_name(name) {
                    Some(Callee::Builtin(name))
                } else {
                    None
                }
            }),
        HExpr::Var {
            sym: None, name, ..
        } => {
            if is_builtin_name(name) {
                Some(Callee::Builtin(name))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn dispatch<'a>(
    it: &mut Interp<'a>,
    frame: &mut IFrame<'a>,
    callee: Callee<'a>,
    args: Vec<Value>,
    span: Span,
) -> R<()> {
    match callee {
        Callee::Builtin(name) => {
            let v = crate::builtin::call(it, name, args, span)?;
            frame.vals.push(v);
            Ok(())
        }
        Callee::Spawn(sym, ten) => {
            let t = it
                .sched
                .borrow_mut()
                .spawn(sym, args, ten.to_string(), span);
            frame.vals.push(Value::Task(t));
            Ok(())
        }
        Callee::Named(sym, is_async, ten) => {
            if is_async {
                let t = it.sched.borrow_mut().tao(sym, args, ten.to_string(), span);
                frame.vals.push(Value::Task(t));
                return Ok(());
            }
            let f =
                it.fns.get(&sym).copied().ok_or_else(|| {
                    Abort::Trap(Trap::new(RT, "frame test: hàm không tồn tại", span))
                })?;
            if args.len() != f.params.len() {
                return Err(Abort::Trap(Trap::new(
                    RT,
                    format!(
                        "hàm `{ten}` cần {} đối số, được {}",
                        f.params.len(),
                        args.len()
                    ),
                    span,
                )));
            }
            let mut env = HashMap::new();
            for (p, v) in f.params.iter().zip(args) {
                env.insert(p.sym, v);
            }
            frame.calls.push(CallFrame {
                body: &f.body,
                pc: 0,
                env,
                mutable: HashSet::new(),
                work: Vec::new(),
                last: None,
            });
            Ok(())
        }
    }
}

fn underflow() -> Abort {
    Abort::Trap(Trap::new(
        RT,
        "frame test: ngăn xếp giá trị rỗng",
        Span::new(0, 0),
    ))
}
