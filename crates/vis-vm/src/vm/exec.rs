// SPDX-License-Identifier: MIT OR Apache-2.0
//! Vòng lặp thực thi bytecode `Th::exec` tách khỏi `vm.rs` (**ST-311a / VD-286**).
//!
//! Chứa đủ phòng vệ runtime: `MAX_FRAMES`, `MAX_ITER_DEPTH` (`#195`), `safety_fuel`,
//! `deadline` wall-clock monotonic (`#195`), và `pop_frame` trap writeback ngoài miền (`#190`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use vis_bytecode::{Instr, BI_MATCH_NO_ARM, BI_MAU_LIST_GE, BI_MAU_LIST_TAIL, GLOBAL_SLOT_BASE};
use vis_core::trap::{Abort, Trap, R};
use vis_core::value::{self, Value, VmFunc};
use vis_interp::builtin::{self, Host};
use vis_interp::interp::RT;
use vis_syntax::Span;

use super::vm_helpers::*;
use super::{Frame, Th, Vm, VmError, DEADLINE_CODE, MAX_FRAMES, MAX_ITER_DEPTH, SAFETY_FUEL_CODE};
use crate::vm_frame::{MethodKind, MethodLoop};

impl<'m> Th<'m> {
    pub(crate) fn exec(&mut self, vm: &mut Vm<'_>) -> R<Value> {
        loop {
            self.method_poll(vm)?;
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
                self.pop_frame(Span::new(0, 0))?;
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
            let pc = fr.pc;
            let fi = fr.fi;
            let span = f
                .spans
                .get(fr.pc)
                .map(|s| Span::new(s.start as usize, s.end as usize))
                .unwrap_or_else(|| Span::new(0, 0));
            // **ST-276 (VD-256)**: charge bảng phụ TRƯỚC tác dụng phụ; pc đã được tính là điểm
            // sẽ chạy, continuation lưu pc SAU charge ⇒ resume không tính lại cả nhóm.
            vm.budget_charge(fi, pc, span)?;
            self.frames.last_mut().expect("khung").pc += 1;
            vm.steps += 1;
            if vm.steps > vm.safety_fuel {
                // VD-256 D8 / VS3-1a: fuel AN TOÀN VM, chẩn đoán RIÊNG. Ngưỡng mặc định lấy từ
                // hợp đồng dùng chung (`fee::VM_SAFETY_FUEL`), lớn hơn trần ngân sách định cỡ ⇒
                // chương trình còn trong ngân sách không bị fuel chặn trước.
                let cap = vm.safety_fuel;
                return Err(Abort::Trap(Trap::new(
                    SAFETY_FUEL_CODE,
                    format!(
                        "VM safety fuel cạn (>{cap} lệnh) — hàng rào an toàn bytecode, không phải ngân sách ngữ nghĩa"
                    ),
                    span,
                )));
            }
            // **ST-311a (#195)**: hàng rào deadline wall-clock monotonic (kiểm mỗi 4.096 bước,
            // chỉ kích hoạt khi `deadline.is_some()` ⇒ 0 chi phí syscall khi mặc định `None`).
            if vm.steps & 0xFFF == 0 {
                if let Some(dl) = vm.deadline {
                    if std::time::Instant::now() >= dl {
                        return Err(Abort::Trap(Trap::new(
                            DEADLINE_CODE,
                            "VM hết thời gian chạy tối đa (deadline wall-clock)",
                            span,
                        )));
                    }
                }
            }
            match ins {
                Instr::Const(i) => {
                    let v = vm.lit(i, span)?;
                    self.st.push(v);
                }
                Instr::LoadLocal(i) => {
                    // **ST-238 (VD-242, ca 3)**: ô `>= GLOBAL_SLOT_BASE` là biến cấp tệp dùng chung.
                    // **ST-250 (VD-245)**: đọc global luôn qua `doc_global` — trap nếu chưa khởi tạo.
                    // **ST-252 (VD-246)**: đường local cũng qua `doc_local` — trap nếu ô ngoài khung;
                    // `Unit` trong khung vẫn trả bình thường, không đổi sang sentinel.
                    let v = if i >= GLOBAL_SLOT_BASE {
                        vm.doc_global((i - GLOBAL_SLOT_BASE) as usize, span)?
                    } else {
                        let fr = self.frames.last().expect("khung");
                        vm.doc_local(&fr.locals, i, span)?
                    };
                    self.st.push(v);
                }
                Instr::StoreLocal(i) | Instr::Assign(i) | Instr::BindLocal(i) => {
                    let v = self.pop(vm, span)?;
                    if i >= GLOBAL_SLOT_BASE {
                        let k = (i - GLOBAL_SLOT_BASE) as usize;
                        // Kho đã được cấp theo module ở `run()`; chỉ số vượt kho là bytecode hỏng
                        // (đã bị verifier chặn) — trap, tuyệt đối không resize theo chỉ số.
                        if k >= vm.globals.len() {
                            return Err(vm.hong(VmError::BadIdx(i as u32), span));
                        }
                        // **ST-250 (VD-245)**: chỉ đánh dấu đã khởi tạo **sau khi ghi thành công**.
                        vm.globals[k] = Some(v);
                    } else {
                        // **ST-252 (VD-246)**: ghi local có kiểm tra — ô ngoài khung trap, không
                        // `resize` (verifier đã chặn `LocalOverflow`, đây là defence-in-depth).
                        let fr = self.frames.last_mut().expect("khung");
                        vm.ghi_local(&mut fr.locals, i, v, span)?;
                    }
                }
                Instr::LoadCaptured(i) => {
                    // **ST-252 (VD-246)**: đọc capture có kiểm tra với môi trường thực tế của khung;
                    // ngoài miền ⇒ trap (`VS0501`), không trả `Unit` mặc định như trước.
                    let fr = self.frames.last().expect("khung");
                    let v = vm.doc_captured(&fr.captured, i, span)?;
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
                    // **VS3-1d**: `Pipe` (`l |> f`) gọi **giá trị** `f` với `l`, khớp interp
                    // (`eval` nhánh `BinOp::Pipe`). Trước đây rơi vào `builtin::bin` ⇒
                    // `VS0501 "phải xử lý ở tầng biểu thức"`; codegen không đổi opcode.
                    let v = if op == vis_types::BinOp::Pipe {
                        vm.call(&r, vec![l], span)?
                    } else {
                        builtin::bin(op, l, r, span)?
                    };
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
                    let iter_base = self.iters.len();
                    self.frames.push(Frame {
                        fi,
                        pc: 0,
                        locals,
                        captured: Vec::new(),
                        st_cu: Vec::new(),
                        writeback: None,
                        iter_base,
                        loop_ends: HashMap::new(),
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
                    // ST-267/VD-251: hook frame-only (default = `builtin::call` cũ).
                    // **VD-260** — builtin **nội bộ** của pattern list `[a, ..]`/`[a, ..r]`:
                    // `TestList(n)` chỉ đúng `n`; rest cần `len ≥ n` và bản sao `xs[n..]`.
                    // Xử lý ngay tại đây (không thêm opcode/tag/VERSION); interp có bộ khớp
                    // pattern riêng nên chỉ VM chạm hai tên này.
                    if ten == BI_MAU_LIST_GE {
                        let ok = match args.as_slice() {
                            [Value::List(xs), Value::Int(n)] => *n >= 0 && xs.len() >= *n as usize,
                            _ => false,
                        };
                        self.st.push(Value::Bool(ok));
                    } else if ten == BI_MAU_LIST_TAIL {
                        let v = match args.as_slice() {
                            [Value::List(xs), Value::Int(n)] => {
                                let n = (*n).max(0) as usize;
                                Value::List(xs.iter().skip(n).cloned().collect())
                            }
                            _ => Value::Unit,
                        };
                        self.st.push(v);
                    } else {
                        let v = crate::vm_frame::call_builtin_frame(self, vm, &ten, args, span)?;
                        self.st.push(v);
                    }
                }
                Instr::CallMethod(i, n) => {
                    let ten = vm.ten(i, span)?;
                    let args = self.pop_n(vm, n as usize, span)?;
                    let recv = self.pop(vm, span)?;
                    // **VD-258**: `list.map/filter/each` chạy vòng callback trên chính `Th`
                    // (khung callback suspend/resume được). Trước đây `builtin::method` gọi
                    // `host.call` tạo `Th` lồng ⇒ callback `await` mất ngăn xếp/`pc` và lần
                    // resume chạy tiếp instruction sau với ngăn xếp thiếu (`VS0501: ngăn xếp rỗng`).
                    if vm.sched.borrow().frame.enabled {
                        let kind = match (&recv, ten.as_str()) {
                            (Value::List(_), "map") => Some(MethodKind::Map),
                            (Value::List(_), "filter") => Some(MethodKind::Filter),
                            (Value::List(_), "each") => Some(MethodKind::Each),
                            _ => None,
                        };
                        if let (Value::List(items), Some(kind)) = (&recv, kind) {
                            let f = args.into_iter().next().unwrap_or(Value::Unit);
                            self.methods.push(MethodLoop {
                                kind,
                                f,
                                items: items.clone(),
                                idx: 0,
                                acc: Vec::new(),
                                item: None,
                                awaiting: false,
                                base_frames: self.frames.len(),
                                span,
                            });
                            // Điểm nối: `method_poll` ở đầu vòng `exec` chạy callback kế tiếp.
                            continue;
                        }
                    }
                    let v = if vm.sched.borrow().frame.enabled {
                        crate::vm_frame::call_method_frame(self, vm, recv, args, &ten, span)?
                    } else {
                        builtin::method(vm, recv, &ten, args, span)?
                    };
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
                    self.pop_frame(span)?;
                    match self.frames.last_mut() {
                        None => return Ok(v),
                        Some(f) => {
                            // Khôi phục ngăn xếp khung gọi rồi đẩy giá trị trả.
                            self.st = std::mem::take(&mut f.st_cu);
                            self.st.push(v);
                        }
                    }
                }
                Instr::MakeList(n) | Instr::MakeTuple(n) => {
                    let items = self.pop_n(vm, n as usize, span)?;
                    self.st.push(Value::List(items));
                }
                // **ST-239 (VD-242 §4, ca 10)**: `set{…}` là **tập** — giữ phần tử đầu mỗi
                // giá trị, bỏ phần tử trùng (interp đã làm vậy; VM trước đây giữ cả).
                Instr::MakeSet(n) => {
                    let items = self.pop_n(vm, n as usize, span)?;
                    let mut uniq: Vec<Value> = Vec::with_capacity(items.len());
                    for v in items {
                        if !uniq.iter().any(|x| value::eq(x, &v)) {
                            uniq.push(v);
                        }
                    }
                    self.st.push(Value::List(uniq));
                }
                Instr::MakeMap(n) => {
                    // **ST-239 (VD-242 §3.4, ca 170)**: runtime last-wins, khớp interp —
                    // mỗi khoá một entry (giữ giá trị cuối, vị trí chèn đầu).
                    let flat = self.pop_n(vm, 2 * n as usize, span)?;
                    let mut pairs: Vec<(Value, Value)> = Vec::with_capacity(n as usize);
                    let mut it = flat.into_iter();
                    while let (Some(k), Some(v)) = (it.next(), it.next()) {
                        if let Some(e) = pairs.iter_mut().find(|(kk, _)| value::eq(kk, &k)) {
                            e.1 = v;
                        } else {
                            pairs.push((k, v));
                        }
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
                    // **VD-267**: mỗi hằng chỉ tách + nội bộ hoá tên **một lần** (cache theo chỉ
                    // số hằng); các lần dựng sau chỉ clone `Name` — không còn `ten.to_string()`
                    // + hai `String` cho mỗi giá trị enum như trước.
                    let tag = match vm.enum_tags.get(v as usize).and_then(|c| c.get()).cloned() {
                        Some(t) => t,
                        None => {
                            let duong = vm.ten_str(v, span)?;
                            let t = match duong.rsplit_once('.') {
                                Some((a, b)) => (value::Name::new(a), value::Name::new(b)),
                                None => (value::Name::new(""), value::Name::new(duong)),
                            };
                            if let Some(c) = vm.enum_tags.get(v as usize) {
                                let _ = c.set(t.clone());
                            }
                            t
                        }
                    };
                    let args = self.pop_n(vm, n as usize, span)?;
                    self.st.push(Value::Enum(tag.0, tag.1, args));
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
                    // **ST-311a (#195)**: chặn độ sâu `iters` trước khi đẩy để bytecode đột biến
                    // có back-edge nhảy ngược về `IterInit` không thể làm phình bộ nhớ vô hạn;
                    // `loop_ends` nằm theo từng `Frame` (`HashMap<end_pc, base>`) nên tra cứu
                    // `dat_pc()` là O(1) và khử trùng lặp `end_pc` tự nhiên.
                    if self.iters.len() >= MAX_ITER_DEPTH {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!(
                                "vòng lặp `for` lồng quá sâu (>{MAX_ITER_DEPTH} bộ lặp) — bytecode hỏng?"
                            ),
                            span,
                        )));
                    }
                    let base = self.iters.len();
                    let (fi, next_pc) = {
                        let fr = self.frames.last().expect("khung");
                        (fr.fi, fr.pc)
                    };
                    if let Some(Instr::IterNext(d)) = self.m.funcs[fi as usize].code.get(next_pc) {
                        let end_pc = next_pc as i64 + i64::from(*d);
                        if end_pc >= 0 {
                            self.frames
                                .last_mut()
                                .expect("khung")
                                .loop_ends
                                .entry(end_pc as usize)
                                .or_insert(base);
                        }
                    }
                    let it = self.pop(vm, span)?;
                    let items = vis_interp::value::iter_items(it, span)?;
                    self.iters.push(items);
                }
                Instr::IterNext(d) => {
                    let Some(items) = self.iters.last_mut() else {
                        return Err(vm.hong(VmError::StackUnderflow, span));
                    };
                    match items.next() {
                        Some(v) => self.st.push(v),
                        None => {
                            self.iters.pop();
                            let pc = {
                                let fr = self.frames.last().expect("khung");
                                let pc = fr.pc as i64 - 1 + d as i64;
                                self.day(fr, pc, vm, span, "IterNext")?;
                                pc
                            };
                            self.dat_pc(pc as usize);
                        }
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
                Instr::TestTuple(n) => {
                    let v = self.pop(vm, span)?;
                    let ok = match &v {
                        Value::List(xs) => xs.len() == n as usize,
                        // `()` khớp mẫu tuple rỗng.
                        Value::Unit => n == 0,
                        _ => false,
                    };
                    self.st.push(Value::Bool(ok));
                }
                // **VD-263**: `[]` là pattern **list** — chỉ khớp `Value::List`, KHÔNG khớp
                // `Value::Unit` (interp `HPat::List` cũng vậy). `TestTuple(0)` giữ nhánh `Unit`.
                Instr::TestList(n) => {
                    let v = self.pop(vm, span)?;
                    let ok = matches!(&v, Value::List(xs) if xs.len() == n as usize);
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
                    match f {
                        // **VD-257 (stream B)** — gọi hàm bytecode (closure/fn-value) phải mở khung
                        // **trong chính `Th` này** (như `Instr::Call`), KHÔNG dùng `goi_ham_bytecode`
                        // tạo `Th` lồng nhau: `Th` lồng không suspend/resume được. Khi thân closure
                        // `await` trả `Abort::Pending`, driver frame lưu `Th` ngoài; `Th` lồng (cùng
                        // pc/stack của `$await`) bị mất ⇒ resume chạy lại `CallValue` với ngăn xếp
                        // đã pop đối số ⇒ `StackUnderflow`. Đẩy khung vào `self.frames` giữ nguyên
                        // trạng thái suspend được. Không đổi opcode/ngôn ngữ/runtime mặc định.
                        Value::Vm(vf) => {
                            self.push_vm_frame(vm, &vf, args, span)?;
                        }
                        other => {
                            let v = builtin::Host::call(vm, &other, args, span)?;
                            self.st.push(v);
                        }
                    }
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
                            self.pop_frame(span)?;
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
            // VD-257/D — hết hạn ngạch: yield về ĐUÔI ready, KHÔNG trap. Đặt CUỐI
            // thân loop: lệnh vừa charge đã thi hành xong và `pc` đã tăng (D6) ⇒
            // resume ở lệnh kế, KHÔNG chạy lại / charge lại lệnh cũ. `Th` đã có
            // đủ trạng thái (frames/st/locals/captured/writeback) để driver cất.
            // `quota_active = false` (mặc định) ⇒ no-op, runtime thường không đổi.
            if vm.quota_take_hit() {
                let cur = vm.sched.borrow().frame.current;
                vm.sched.borrow_mut().frame.log.push(format!("quota:{cur}"));
                return Err(Abort::Pending(vis_core::value::Pending {
                    task: None,
                    wait: None,
                    span,
                }));
            }
        }
    }
}
