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

use std::collections::HashMap;

use vis_bytecode::Instr;
use vis_interp::builtin;
use vis_interp::frame::{FrameConfig, FrameOutput};
use vis_interp::interp::{Abort, Trap};
use vis_interp::rt::{self, Waiter};
use vis_interp::value::Value;
use vis_syntax::{Diagnostic, Span};

use crate::vm::{Frame, Th, Vm};

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
    let mut vm: Vm<'m> = Vm::new(m, text);
    if let Err(e) = vis_bytecode::verify(m) {
        return FrameOutput {
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
        };
    }
    vm.init_globals();
    let argv = Value::List(args.iter().map(|s| Value::Str(s.clone())).collect());
    let main_th = match main_th(&vm, &argv) {
        Ok(t) => t,
        Err(Abort::Trap(t)) => return frame_output(&vm, None, Some(t)),
        Err(_) => return frame_output(&vm, None, None),
    };

    let mut frames: HashMap<u32, Th<'m>> = HashMap::new();
    frames.insert(0, main_th);
    vm.sched
        .borrow_mut()
        .frame_enable_for_test(config.fake_sources, config.events);

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
                let w: Option<Waiter> = vm.sched.borrow().frame_await(current, &p.task);
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
                        let th = task_th(&vm, t.than, t.args.clone());
                        e.insert(th);
                    }
                }
            }
            None => break,
        }
    }

    frames.clear();
    vm.sched.borrow_mut().frame_drain_events();
    vm.sched.borrow_mut().frame_final_join();
    frame_output(&vm, final_value, trap)
}

fn frame_output(vm: &Vm<'_>, value: Option<Value>, trap: Option<Trap>) -> FrameOutput {
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
        log: vm.sched.borrow().frame.log.clone(),
        snapshot: vm.sched.borrow().diag_snapshot(),
    }
}

/// Dựng `Th` cho thân task async từ **chỉ số hàm bytecode thật**.
fn task_th<'m>(vm: &Vm<'m>, fi: u32, args: Vec<Value>) -> Th<'m> {
    let f = vm.func(fi).expect("hàm task");
    let mut locals = vec![Value::Unit; f.nlocals.max(1) as usize];
    for (i, a) in args.into_iter().enumerate() {
        if i < locals.len() {
            locals[i] = a;
        }
    }
    Th::new(vm.m, fi, locals, Vec::new())
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
