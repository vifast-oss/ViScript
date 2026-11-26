// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-285 (VD-272)** — phát lệnh cho **câu lệnh** (`cau`) + helper điều khiển
//! (`khoi`/`cond`/`slot_cua`/`net_stack`/`trap_no_arm`), tách nguyên văn khỏi `codegen.rs`
//! (R0 refactor, 0 đổi hành vi). Các phương thức `pub(super)` để `Emit` ở tệp cha và các
//! mô-đun con khác (`than`, `pattern`, `gan_long`, `gia_tri`, `if_expr`) dùng chung.

use super::*;

impl Emit<'_> {
    // ----- câu lệnh -----

    pub(super) fn cau(&mut self, s: &HStmt) -> Result<(), CodegenError> {
        // **ST-276 (VD-256)**: E_STMT một lần khi bắt đầu câu nguồn.
        self.f.charge(FeeKind::Stmt, s.span());
        // **VD-257 (stream B)**: với `while`/`loop`, `start` của vòng trùng PC của E_STMT ⇒ VM
        // charge lại E_STMT mỗi lần nhảy lùi (interp chỉ charge 1 lần). Đẩy header vòng qua một
        // cặp lệnh net-zero (`PushUnit; Pop`, không đổi ngữ nghĩa/opcode) để E_STMT nằm **trước**
        // vòng; back-edge nhảy vào header (sau cặp) ⇒ chỉ E_LOOP lặp lại, đúng interp.
        if matches!(s, HStmt::While { .. } | HStmt::Loop { .. }) {
            self.f.push(Instr::PushUnit, s.span(), 1);
            self.f.push(Instr::Pop, s.span(), -1);
        }
        match s {
            HStmt::Let {
                pat,
                ty,
                init,
                else_,
                span,
            } => {
                self.expr(init)?;
                // **VD-241 §3** — tự nâng theo annot; `$lift-opt` tới đủ lớp `Opt` (§4.4).
                self.lift_to_annot(ty.as_ref(), *span)?;
                self.f.charge(FeeKind::Assign, *span); // ST-276: ghi do nguồn quy định (sau RHS)
                                                       // **R3b (VD-178)**: `let x = r else:` — nhánh `Ok`/`Some` bóc giá trị; ngược lại
                                                       // chạy khối `else` (nhận lỗi nếu có `else err:`).
                if let Some(e) = else_ {
                    self.let_else(pat, e, *span)?;
                } else {
                    self.bind(pat, *span, true)?;
                }
                Ok(())
            }
            HStmt::Var {
                sym,
                init,
                span,
                ty,
                ..
            } => {
                self.expr(init)?;
                self.lift_to_annot(ty.as_ref(), *span)?; // biên khai `var` như `let` (mirror interp)
                self.f.charge(FeeKind::Assign, *span); // ST-276
                                                       // **ST-238 (VD-242, ca 3)**: dùng `slot_cua` (không phải `f.slot` trực tiếp) để
                                                       // `var` cấp tệp ghi vào ô chung — nếu không, khai báo ghi ô 0 nhưng mọi lần đọc
                                                       // lại lấy ô global ⇒ `()` (hồi quy 10k async task).
                let slot = self.slot_cua(*sym)?;
                self.f.push(Instr::StoreLocal(slot), *span, -1);
                // **ST-319d (#208)**: `var` ⇒ `push` trên nó phải ghi lại ô (VD-51).
                if *sym != 0 {
                    self.g.mutables.insert(*sym);
                }
                self.net_stack(*span)
            }
            HStmt::Assign {
                target,
                op,
                value,
                span,
            } => {
                if let HExpr::Var { sym: Some(id), .. } = target {
                    match op {
                        vis_hir::AssignOp::Set => {
                            self.expr(value)?;
                            self.lift_assign_set(target, *span)?; // gán lại `var` annot Optional: nâng một lớp; `None` giữ nguyên
                        }
                        other => {
                            // `x += v` ⇒ `x = x <op> v`.
                            let slot = self.slot_cua(*id)?;
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                            self.expr(value)?;
                            self.f.push(Instr::Bin(assign_binop(*other)?), *span, -1);
                        }
                    }
                    let slot = self.slot_cua(*id)?;
                    self.f.charge(FeeKind::Assign, *span); // ST-276: sau giá trị + đích
                    self.f.push(Instr::Assign(slot), *span, -1);
                    return self.net_stack(*span);
                }
                // **ST-56b (VD-97)** + **VD-268**: đích `trường/phần tử` — gốc `var` (tyck chặn
                // trước). Đường sâu/gán ghép lá qua `gan_long::gan_duong`; dạng một tầng giữ nguyên.
                if !matches!(op, vis_hir::AssignOp::Set) || !gan_long::mot_tang(target) {
                    return self.gan_duong(target, *op, value, *span);
                }
                // `a[i] = v` ⇒ `Set` (3→1) rồi **ghi lại ô** (`Assign`) — trước chỉ `Pop` nên VM
                // **im lặng** bỏ kết quả (interp ↔ VM lệch); VD-97 (2) đòi sửa **tại chỗ**.
                match target {
                    HExpr::Index { base, index, .. } => {
                        if let HExpr::Var { sym: Some(id), .. } = &**base {
                            let slot = self.slot_cua(*id)?;
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                            self.expr(index)?;
                            self.expr(value)?;
                            self.lift_assign_set(target, *span)?; // ST-248: đích `map`/`list` Opt
                            self.f.push(Instr::Set, *span, -2);
                            self.f.push(Instr::Assign(slot), *span, -1);
                            return self.net_stack(*span);
                        }
                        // **ST-60b**: `x.f[i] = v` (một tầng trường, gốc `var`): đọc `x.f`, `Set`,
                        // rồi **ghi lại trường** bằng `With` + `Assign` — cùng ngữ nghĩa
                        // `x = x with f: <mới>` mà interp thi hành cho đích này.
                        if let HExpr::Field {
                            base: goc, name, ..
                        } = &**base
                        {
                            let HExpr::Var { sym: Some(id), .. } = &**goc else {
                                return Err(CodegenError::Unsupported("assign-index", *span));
                            };
                            let slot = self.slot_cua(*id)?;
                            // [x, "f", x.f, i, v] ⇒ Set ⇒ [x, "f", list'] ⇒ With ⇒ [x'] ⇒ Assign.
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                            let nc = self.g.cst_str(name);
                            self.f.push(Instr::Const(nc), *span, 1);
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                            let fc = self.g.cst_str(name);
                            self.f.push(Instr::Field(fc), *span, 0);
                            self.expr(index)?;
                            self.expr(value)?;
                            self.lift_assign_set(target, *span)?; // ST-248: đích `x.f[i]` Opt
                            self.f.push(Instr::Set, *span, -2);
                            self.f.push(Instr::With(1), *span, -2);
                            self.f.push(Instr::Assign(slot), *span, -1);
                            return self.net_stack(*span);
                        }
                        Err(CodegenError::Unsupported("assign-index", *span))
                    }
                    HExpr::Field { base, name, .. } => {
                        if let HExpr::Var { sym: Some(id), .. } = &**base {
                            let slot = self.slot_cua(*id)?;
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                            let n = self.g.cst_str(name);
                            self.f.push(Instr::Const(n), *span, 1);
                            self.expr(value)?;
                            self.lift_assign_set(target, *span)?; // ST-248: đích trường Opt
                            self.f.push(Instr::With(1), *span, -2);
                            self.f.push(Instr::Assign(slot), *span, -1);
                            return self.net_stack(*span);
                        }
                        Err(CodegenError::Unsupported("assign-field", *span))
                    }
                    _ => Err(CodegenError::Unsupported("assign-target", *span)),
                }
            }
            HStmt::Expr(e) => {
                // **ST-319d (#208)**: bỏ đặc cách `push` — ngữ nghĩa `push`/`var` (VD-51) nằm ở
                // `expr()` (một đường cho mọi ngữ cảnh). Nhờ vậy: `var` ghi lại ô ở cả biểu thức
                // cuối; `let` **không** ghi (trước đây ghi sai); `E_BUILTIN` của lời gọi phương
                // thức được charge đúng như interp (ST-276).
                self.expr(e)?;
                self.f.push(Instr::Pop, e.span(), -1);
                self.net_stack(e.span())
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                span,
            } => {
                let mut jend: Vec<usize> = Vec::new();
                let mut jf = self.cond(cond)?;
                // **R3a (VD-178)**: `if let` để **giá trị** trên ngăn xếp cho nhánh `then` bind;
                // nhánh `else`/`elif` phải `Pop` nó trước.
                let mut cond_let = matches!(cond, HCond::Let { .. });
                self.khoi(then)?;
                jend.push(self.f.here());
                self.f.push(Instr::Jump(0), *span, 0);
                let mut prev = jf.take();
                for (c, body) in elifs {
                    let at = self.f.here();
                    if let Some(p) = prev {
                        self.f.patch(p, at);
                    }
                    if cond_let {
                        self.f.push(Instr::Pop, *span, -1);
                    }
                    let mut j = self.cond(c)?;
                    cond_let = matches!(c, HCond::Let { .. });
                    self.khoi(body)?;
                    jend.push(self.f.here());
                    self.f.push(Instr::Jump(0), *span, 0);
                    prev = j.take();
                }
                let at = self.f.here();
                if let Some(p) = prev {
                    self.f.patch(p, at);
                }
                if cond_let {
                    self.f.push(Instr::Pop, *span, -1);
                }
                if let Some(b) = else_ {
                    self.khoi(b)?;
                }
                let end = self.f.here();
                for j in jend {
                    self.f.patch(j, end);
                }
                Ok(())
            }
            HStmt::For {
                pat,
                iter,
                body,
                span,
            } => {
                let sp_iter = iter.span();
                self.expr(iter)?;
                self.f.push(Instr::IterInit, sp_iter, 0);
                let start = self.f.here();
                self.f.charge(FeeKind::Loop, *span); // ST-276: mỗi lần thử tiến vòng (kể cả lần cuối)
                self.f.push(Instr::IterNext(0), *span, 1);
                let next = self.f.code.len() - 1;
                self.bind(pat, *span, true)?;
                self.loops.push(Vong {
                    breaks: Vec::new(),
                    continue_t: start,
                });
                self.khoi(body)?;
                self.f.push(Instr::Jump(0), *span, 0);
                let back = self.f.code.len() - 1;
                self.f.patch(back, start);
                let end = self.f.here();
                self.f.patch(next, end);
                if let Some(v) = self.loops.pop() {
                    for b in v.breaks {
                        self.f.patch(b, end);
                    }
                }
                self.net_stack(*span)
            }
            HStmt::While { cond, body, span } => {
                let start = self.f.here();
                self.f.charge(FeeKind::Loop, *span); // ST-276: mỗi lần thử điều kiện
                                                     // **ST-226 (R18b)**: `while let` dùng chung `cond` với `if let` (MV gán, true-branch Pop).
                let at = self.cond(cond)?;
                let jf = at.expect("cond() luôn trả Some(at)");
                self.loops.push(Vong {
                    breaks: Vec::new(),
                    continue_t: start,
                });
                self.khoi(body)?;
                self.f.push(Instr::Jump(0), *span, 0);
                let back = self.f.code.len() - 1;
                self.f.patch(back, start);
                let end = self.f.here();
                // **ST-238 (VD-242, ca 2)**: `while let` — `cond()` để giá trị pattern trên ngăn
                // xếp ở nhánh sai (`if let` có `Pop` ở caller, `while` thì thiếu ⇒ verify lệch độ
                // sâu 1/0). Nhánh sai phải `Pop` trước khi rời vòng; `break` ( stack đã sạch ) nhảy
                // **qua** `Pop` đó.
                if matches!(cond, HCond::Let { .. }) {
                    self.f.push(Instr::Pop, *span, -1);
                    let sau_pop = self.f.here();
                    self.f.patch(jf, end);
                    if let Some(v) = self.loops.pop() {
                        for b in v.breaks {
                            self.f.patch(b, sau_pop);
                        }
                    }
                } else {
                    self.f.patch(jf, end);
                    if let Some(v) = self.loops.pop() {
                        for b in v.breaks {
                            self.f.patch(b, end);
                        }
                    }
                }
                Ok(())
            }
            HStmt::Loop { body, span } => {
                let start = self.f.here();
                self.f.charge(FeeKind::Loop, *span); // ST-276: trước lần vào thân đầu + mỗi lần lặp
                self.loops.push(Vong {
                    breaks: Vec::new(),
                    continue_t: start,
                });
                self.khoi(body)?;
                self.f.push(Instr::Jump(0), *span, 0);
                let back = self.f.code.len() - 1;
                self.f.patch(back, start);
                let end = self.f.here();
                if let Some(v) = self.loops.pop() {
                    for b in v.breaks {
                        self.f.patch(b, end);
                    }
                }
                Ok(())
            }
            HStmt::Scope { body, span, .. } | HStmt::Block { body, span } => {
                // Khối **có giá trị**: câu lệnh cuối là biểu thức ⇒ giữ lại rồi `Pop`.
                self.nhanh_gia_tri(body)?;
                self.f.push(Instr::Pop, *span, -1);
                self.net_stack(*span)
            }
            HStmt::Return { value, span } => {
                match value {
                    Some(v) => self.expr(v)?,
                    // **ST-60b**: `return` trần ⇒ `()` **thật** (`PushUnit`), không phải `str("")`.
                    None => self.f.push(Instr::PushUnit, *span, 1),
                }
                self.lift_ret_opt(*span)?; // VD-228: nâng ở biên return sớm
                self.f.push(Instr::Return, *span, -1);
                self.net_stack(*span)
            }
            HStmt::Break { span } => {
                let Some(_) = self.loops.last() else {
                    return Err(CodegenError::Unsupported("break", *span));
                };
                let at = self.f.here();
                self.f.push(Instr::Jump(0), *span, 0);
                if let Some(v) = self.loops.last_mut() {
                    v.breaks.push(at);
                }
                Ok(())
            }
            HStmt::Continue { span } => {
                let Some(v) = self.loops.last() else {
                    return Err(CodegenError::Unsupported("continue", *span));
                };
                let t = v.continue_t;
                self.f.push(Instr::Jump(0), *span, 0);
                let at = self.f.code.len() - 1;
                self.f.patch(at, t);
                Ok(())
            }
            HStmt::Match {
                subject,
                arms,
                span,
            } => {
                self.expr(subject)?;
                let mut jend = Vec::new();
                let mut guardas: Vec<usize> = Vec::new();
                for arm in arms.iter() {
                    self.f.push(Instr::Dup, *span, 1);
                    let jf = self.test(&arm.pat, *span)?;
                    let br = self.f.here();
                    self.f.push(Instr::JumpIfFalse(0), *span, -1);
                    if let Some(j) = jf {
                        self.f.patch(j, br + 1);
                    }
                    // Bind **trước** guard (interp bind rồi mới xét guard); giữ subject để `Pop`.
                    self.bind(&arm.pat, *span, false)?;
                    if let Some(gd) = &arm.guard {
                        self.expr(gd)?;
                        let g = self.f.here();
                        self.f.push(Instr::JumpIfFalse(0), *span, -1);
                        guardas.push(g);
                    }
                    self.f.push(Instr::Pop, *span, -1);
                    self.khoi(&arm.body)?;
                    jend.push(self.f.here());
                    self.f.push(Instr::Jump(0), *span, 0);
                    let next = self.f.here();
                    self.f.patch(br, next);
                    for g in guardas.drain(..) {
                        self.f.patch(g, next);
                    }
                }
                self.f.push(Instr::Pop, *span, -1);
                // **ST-238 (VD-242, ca 7)** — rơi khỏi mọi nhánh ⇒ trap, không im lặng đi tiếp.
                self.trap_no_arm(*span, false)?;
                let end = self.f.here();
                for j in jend {
                    self.f.patch(j, end);
                }
                self.net_stack(*span)
            }
        }
    }

    pub(super) fn slot_cua(&mut self, sym: u32) -> Result<u16, CodegenError> {
        // **ST-319a (#206)**: closure chụp global thành ô cục bộ ⇒ đọc/ghi đều đi ô **riêng của
        // closure** (by-value), không xuyên ra kho global. Xét ô cục bộ trước biến cấp tệp; sym
        // là duy nhất nên nhánh này chỉ đổi hành vi đúng các global đã được chụp.
        if let Some(s) = self.captured.get(&sym) {
            return Ok(*s);
        }
        if let Some(s) = self.f.locals.get(&sym).copied() {
            return Ok(s);
        }
        // **ST-238 (VD-242, ca 3)**: biến cấp tệp dùng ô chung (VM ánh xạ tới `Vm::globals`).
        if let Some(s) = self.g.globals.get(&sym).copied() {
            return Ok(s);
        }
        self.f.slot(sym, Span::new(0, 0))
    }

    pub(super) fn net_stack(&mut self, _span: Span) -> Result<(), CodegenError> {
        Ok(())
    }

    /// **ST-238 (VD-242, ca 7)**: phát một lời gọi builtin giả mà VM chặn lại và trap `VS0501`
    /// với đúng thông điệp của interp. `keep_value = true` (ngữ cảnh giá trị) thì để lại một giá trị
    /// trên ngăn xếp cho verifier — các nhánh khác cũng để lại một giá trị.
    pub(super) fn trap_no_arm(&mut self, span: Span, keep_value: bool) -> Result<(), CodegenError> {
        let n = self.g.cst_str(crate::format::BI_MATCH_NO_ARM);
        self.f.push(Instr::CallBuiltin(n, 0), span, 1);
        if !keep_value {
            self.f.push(Instr::Pop, span, -1);
        }
        Ok(())
    }

    /// Khối câu lệnh: giá trị của `HStmt::Expr` cuối **vẫn** nằm trên ngăn xếp (nếu khối "có giá trị").
    pub(super) fn khoi(&mut self, body: &[HStmt]) -> Result<(), CodegenError> {
        for s in body {
            self.cau(s)?;
        }
        Ok(())
    }

    /// Điều kiện: phát biểu thức rồi trả **chỉ số lệnh nhảy-khi-sai** cần vá.
    pub(super) fn cond(&mut self, c: &HCond) -> Result<Option<usize>, CodegenError> {
        match c {
            HCond::Expr(e) => {
                self.expr(e)?;
                let at = self.f.here();
                self.f.push(Instr::JumpIfFalse(0), e.span(), -1);
                Ok(Some(at))
            }
            HCond::Let { pat, value, span } => {
                self.expr(value)?;
                self.f.push(Instr::Dup, *span, 1);
                let _ = self.test(pat, *span)?;
                let at = self.f.here();
                self.f.push(Instr::JumpIfFalse(0), *span, -1);
                // Đúng: gán biến pattern rồi bỏ giá trị.
                self.bind(pat, *span, false)?;
                self.f.push(Instr::Pop, *span, -1);
                Ok(Some(at))
            }
        }
    }
}
