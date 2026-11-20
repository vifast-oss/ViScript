// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pattern của `match`/`let` (ST-19/ST-20b/ST-21): hằng đầu mút, test (kể cả **lồng**), bind.
//!
//! Là **module con** của `codegen` để dùng được `Emit` + trường riêng mà không phải nới visibility.

use super::*;

impl Emit<'_> {
    /// **VD-262**: phát **mọi** `let`/`var` cấp tệp theo thứ tự nguồn (trước `Call(main)`).
    /// Nhờ đó ô global có lệnh ghi — verifier suy đúng `nglobals` — và giá trị tồn tại trước khi
    /// `main` đọc. Câu lệnh cấp tệp không phải khai báo **không** chạy khi có `main`.
    pub(super) fn khai_bao_cap_tep(&mut self, hir: &Hir) -> Result<(), CodegenError> {
        for s in &hir.stmts {
            if matches!(s, HStmt::Let { .. } | HStmt::Var { .. }) {
                self.cau(s)?;
            }
        }
        Ok(())
    }

    pub(super) fn cst_pat(&mut self, p: Option<&HPat>) -> u32 {
        match p {
            Some(HPat::Lit { lit, .. }) => self.g.cst(lit.kind, &lit.text),
            _ => self.g.cst(LitKind::Int, "0"),
        }
    }

    pub(super) fn test(&mut self, p: &HPat, span: Span) -> Result<Option<usize>, CodegenError> {
        // Bất biến: `test` **tiêu thụ** giá trị đỉnh và để lại **một** `bool`.
        // Pattern lồng: test con trước (mỗi con dùng một bản `Dup`), test ngoài sau, rồi gộp
        // bằng `BoolAnd` (giao hoán nên thứ tự gộp không quan trọng).
        match p {
            HPat::Wild { .. } | HPat::Bind { .. } => {
                // `test` **tiêu thụ** giá trị (hợp đồng 1→1); Wild/Bind luôn khớp.
                self.f.push(Instr::Pop, span, -1);
                let c = self.g.cst(LitKind::Bool, "true");
                self.f.push(Instr::Const(c), span, 1);
                Ok(None)
            }
            HPat::Lit { lit, .. } => {
                let c = self.g.cst(lit.kind, &lit.text);
                self.f.push(Instr::TestLit(c), span, 0);
                Ok(None)
            }
            HPat::Ctor { path, args, .. } => {
                // **VD-241 §3 (A0, 2026-11-19)**: kiểm ctor + từng đối số vào ô tạm `Bool`, rồi gộp;
                // giá trị gốc giữ trên ngăn xếp đến cuối rồi `Swap`+`Pop` — luôn đúng thứ tự, tránh
                // nhầm `Unwrap` lên `Bool` (vòng trước) cũng như outer-test tiêu thụ `Bool` (bản đầu).
                let ten = path.last().cloned().unwrap_or_default();
                let n = self.g.cst_str(&ten);
                self.f.push(Instr::Dup, span, 1);
                self.f.push(Instr::TestCtor(n, args.len() as u16), span, 0);
                let slot = self.f.nlocals;
                self.f.nlocals += 1;
                self.f.push(Instr::StoreLocal(slot), span, -1);
                let mut tmps = vec![slot];
                for (i, sub) in args.iter().enumerate() {
                    if !can_test(sub) {
                        continue;
                    }
                    self.f.push(Instr::Dup, span, 1);
                    self.f.push(Instr::Unwrap(i as u16), span, 0);
                    let _ = self.test(sub, span)?;
                    let s2 = self.f.nlocals;
                    self.f.nlocals += 1;
                    self.f.push(Instr::StoreLocal(s2), span, -1);
                    tmps.push(s2);
                }
                for t in &tmps {
                    self.f.push(Instr::LoadLocal(*t), span, 1);
                }
                for _ in 0..tmps.len().saturating_sub(1) {
                    self.f.push(Instr::BoolAnd, span, -1);
                }
                // `[value, bool]` ⇒ `[bool]` (giá trị của arm bị bỏ, như cũ).
                self.f.push(Instr::Swap, span, 0);
                self.f.push(Instr::Pop, span, -1);
                Ok(None)
            }
            HPat::Tuple { items, .. } | HPat::List { items, .. } => {
                // **VD-241 §3 (A0, 2026-11-19)**: kiểm Tuple/List + phần tử vào ô tạm `Bool`, rồi gộp;
                // giá trị gốc giữ trên ngăn xếp đến cuối rồi `Swap`+`Pop` (như Ctor ở trên).
                self.f.push(Instr::Dup, span, 1);
                let n = items.len() as u16;
                if matches!(p, HPat::Tuple { .. }) {
                    self.f.push(Instr::TestTuple(n), span, 0);
                } else if matches!(p, HPat::List { rest: Some(_), .. }) {
                    // **VD-260**: `[a, ..]`/`[a, ..r]` — dài **≥** số tiền tố, không phải đúng
                    // bằng. Không có opcode `TestListMin` (không đổi VERSION 2) nên gọi builtin
                    // nội bộ thuần, trả `false` cho giá trị không phải `list` (cùng interp).
                    let c = self.g.cst(LitKind::Int, &items.len().to_string());
                    self.f.push(Instr::Const(c), span, 1);
                    let b = self.g.cst_str(crate::format::BI_MAU_LIST_GE);
                    self.f.push(Instr::CallBuiltin(b, 2), span, -1);
                } else {
                    self.f.push(Instr::TestList(n), span, 0);
                }
                let slot = self.f.nlocals;
                self.f.nlocals += 1;
                self.f.push(Instr::StoreLocal(slot), span, -1);
                let mut tmps = vec![slot];
                for (i, sub) in items.iter().enumerate() {
                    if !can_test(sub) {
                        continue;
                    }
                    self.f.push(Instr::Dup, span, 1);
                    // **VD-260**: `Unwrap` (không `Index`) để hình dạng **sai/ngắn** trả `Unit`
                    // cho test con ⇒ `false`, không trap `VS0503`. Interp khớp cấu trúc thật
                    // cũng trả `false`; `Index` vẫn dùng lúc **bind** (sau khi test đã qua).
                    self.f.push(Instr::Unwrap(i as u16), span, 0);
                    let _ = self.test(sub, span)?;
                    let s2 = self.f.nlocals;
                    self.f.nlocals += 1;
                    self.f.push(Instr::StoreLocal(s2), span, -1);
                    tmps.push(s2);
                }
                for t in &tmps {
                    self.f.push(Instr::LoadLocal(*t), span, 1);
                }
                for _ in 0..tmps.len().saturating_sub(1) {
                    self.f.push(Instr::BoolAnd, span, -1);
                }
                self.f.push(Instr::Swap, span, 0);
                self.f.push(Instr::Pop, span, -1);
                Ok(None)
            }
            HPat::Record { fields, open, .. } => {
                // **VD-241 §3 (A0, 2026-11-19)**: kiểm Record + trường vào ô tạm `Bool`, rồi gộp;
                // giá trị gốc giữ trên ngăn xếp đến cuối rồi `Swap`+`Pop` (như Ctor ở trên).
                self.f.push(Instr::Dup, span, 1);
                let t = self.g.cst_str("");
                self.f.push(
                    Instr::TestRecord(t, fields.len() as u16, u8::from(*open)),
                    span,
                    0,
                );
                let slot = self.f.nlocals;
                self.f.nlocals += 1;
                self.f.push(Instr::StoreLocal(slot), span, -1);
                let mut tmps = vec![slot];
                for (name, sub) in fields {
                    let Some(sub) = sub else { continue };
                    if !can_test(sub) {
                        continue;
                    }
                    self.f.push(Instr::Dup, span, 1);
                    let c = self.g.cst_str(name);
                    self.f.push(Instr::Field(c), span, 0);
                    let _ = self.test(sub, span)?;
                    let s2 = self.f.nlocals;
                    self.f.nlocals += 1;
                    self.f.push(Instr::StoreLocal(s2), span, -1);
                    tmps.push(s2);
                }
                for t in &tmps {
                    self.f.push(Instr::LoadLocal(*t), span, 1);
                }
                for _ in 0..tmps.len().saturating_sub(1) {
                    self.f.push(Instr::BoolAnd, span, -1);
                }
                self.f.push(Instr::Swap, span, 0);
                self.f.push(Instr::Pop, span, -1);
                Ok(None)
            }
            HPat::Range {
                lo, hi, inclusive, ..
            } => {
                let a = self.cst_pat(lo.as_deref());
                let b = self.cst_pat(hi.as_deref());
                self.f
                    .push(Instr::TestRange(a, b, u8::from(*inclusive)), span, 0);
                Ok(None)
            }
            HPat::Or { alts, .. } => {
                // Giá trị gốc ở đỉnh; mỗi nhánh test trên **bản sao**, cất `bool` vào ô tạm.
                let mut o: Vec<u16> = Vec::new();
                for alt in alts {
                    self.f.push(Instr::Dup, span, 1);
                    let _ = self.test(alt, span)?;
                    let slot = self.f.nlocals;
                    self.f.nlocals += 1;
                    self.f.push(Instr::StoreLocal(slot), span, -1);
                    o.push(slot);
                }
                for (i, slot) in o.iter().enumerate() {
                    self.f.push(Instr::LoadLocal(*slot), span, 1);
                    if i > 0 {
                        self.f.push(Instr::BoolOr, span, -1);
                    }
                }
                // `[giá trị, bool]` ⇒ `[bool]` (tiêu thụ giá trị cho đúng hợp đồng).
                self.f.push(Instr::Swap, span, 0);
                self.f.push(Instr::Pop, span, -1);
                Ok(None)
            }
            HPat::Rest { .. } => Err(CodegenError::Unsupported("rest-pattern", span)),
        }
    }

    pub(super) fn bind(&mut self, p: &HPat, span: Span, pop: bool) -> Result<(), CodegenError> {
        if !pop {
            self.f.push(Instr::Dup, span, 1);
        }
        self.gan_tu_dinh(p, span)
    }

    pub(super) fn gan_tu_dinh(&mut self, p: &HPat, span: Span) -> Result<(), CodegenError> {
        match p {
            HPat::Wild { .. } | HPat::Lit { .. } | HPat::Range { .. } => {
                self.f.push(Instr::Pop, span, -1);
                Ok(())
            }
            HPat::Bind { sym, .. } => {
                let slot = self.slot_cua(*sym)?;
                self.f.push(Instr::StoreLocal(slot), span, -1);
                Ok(())
            }
            HPat::Tuple { items, .. } => {
                for (i, it) in items.iter().enumerate() {
                    self.f.push(Instr::Dup, span, 1);
                    let c = self.g.cst(LitKind::Int, &i.to_string());
                    self.f.push(Instr::Const(c), span, 1);
                    self.f.push(Instr::Index, span, -1);
                    self.gan_tu_dinh(it, span)?;
                }
                self.f.push(Instr::Pop, span, -1);
                Ok(())
            }
            HPat::List { items, rest, .. } => {
                for (i, it) in items.iter().enumerate() {
                    self.f.push(Instr::Dup, span, 1);
                    let c = self.g.cst(LitKind::Int, &i.to_string());
                    self.f.push(Instr::Const(c), span, 1);
                    self.f.push(Instr::Index, span, -1);
                    self.gan_tu_dinh(it, span)?;
                }
                // **VD-260**: đuôi là **bản sao** `xs[n..]` (COW như interp: `Value::List` clone);
                // `..` trần không tạo list — `HPat::Rest` chỉ bỏ giá trị.
                if let Some(r) = rest {
                    if !matches!(r.as_ref(), HPat::Rest { .. }) {
                        self.f.push(Instr::Dup, span, 1);
                        let c = self.g.cst(LitKind::Int, &items.len().to_string());
                        self.f.push(Instr::Const(c), span, 1);
                        let b = self.g.cst_str(crate::format::BI_MAU_LIST_TAIL);
                        self.f.push(Instr::CallBuiltin(b, 2), span, -1);
                        self.gan_tu_dinh(r, span)?;
                    }
                }
                self.f.push(Instr::Pop, span, -1);
                Ok(())
            }
            HPat::Ctor { args, .. } => {
                for (i, it) in args.iter().enumerate() {
                    self.f.push(Instr::Dup, span, 1);
                    self.f.push(Instr::Unwrap(i as u16), span, 0);
                    self.gan_tu_dinh(it, span)?;
                }
                self.f.push(Instr::Pop, span, -1);
                Ok(())
            }
            HPat::Record { fields, .. } => {
                for (name, sub) in fields {
                    let c = self.g.cst_str(name);
                    self.f.push(Instr::Dup, span, 1);
                    self.f.push(Instr::Field(c), span, 0);
                    match sub {
                        Some(it) => self.gan_tu_dinh(it, span)?,
                        None => {
                            self.f.push(Instr::Pop, span, -1);
                        }
                    }
                }
                self.f.push(Instr::Pop, span, -1);
                Ok(())
            }
            HPat::Or { alts, .. } => {
                // **VD-258**: bind theo **alternative khớp**, không phải alternative đầu.
                // Giá trị gốc ở đỉnh; mỗi alternative không-cuối được test trên bản sao:
                //   [value] → Dup → test(alt) → JumpIfFalse(next) → bind(alt) → Jump(end)
                //   next: [value] (bool đã bị JumpIfFalse tiêu thụ) → thử alt kế.
                // Alternative cuối là fallback: outer match chỉ vào arm này khi đã có alt khớp.
                if alts.is_empty() {
                    self.f.push(Instr::Pop, span, -1);
                    return Ok(());
                }
                let mut ends = Vec::new();
                for (i, alt) in alts.iter().enumerate() {
                    if i + 1 == alts.len() {
                        self.gan_tu_dinh(alt, span)?;
                        break;
                    }
                    self.f.push(Instr::Dup, span, 1);
                    let _ = self.test(alt, span)?;
                    let jf = self.f.here();
                    self.f.push(Instr::JumpIfFalse(0), span, -1);
                    self.gan_tu_dinh(alt, span)?;
                    let jend = self.f.here();
                    self.f.push(Instr::Jump(0), span, 0);
                    ends.push(jend);
                    let next = self.f.here();
                    self.f.patch(jf, next);
                }
                let end = self.f.here();
                for j in ends {
                    self.f.patch(j, end);
                }
                Ok(())
            }
            HPat::Rest { .. } => Err(CodegenError::Unsupported("rest-pattern", span)),
        }
    }
}

impl Emit<'_> {
    /// **R3b (VD-178)**: `let x = r else:` — nhánh `Ok`/`Some` bóc giá trị; ngược lại chạy khối
    /// `else` (nhận lỗi nếu có `else err:`).
    pub(super) fn let_else(
        &mut self,
        pat: &HPat,
        e: &vis_types::hir::HElse,
        span: Span,
    ) -> Result<(), CodegenError> {
        // **VD-260**: pattern không phải tên trần khớp trên **kiểu đầy đủ**; chỉ `let x = … else`
        // mới bóc một lớp `Ok`/`Some` (R3b).
        if !matches!(pat, HPat::Bind { .. }) {
            self.f.push(Instr::Dup, span, 1);
            let _ = self.test(pat, span)?;
            let j_else = self.f.here();
            self.f.push(Instr::JumpIfFalse(0), span, -1);
            self.bind(pat, span, true)?;
            let j_end = self.f.here();
            self.f.push(Instr::Jump(0), span, 0);
            let le = self.f.here();
            self.f.patch(j_else, le);
            if let Some(sym) = e.bind_sym {
                self.f.push(Instr::Unwrap(0), span, 0);
                let slot = self.slot_cua(sym)?;
                self.f.push(Instr::BindLocal(slot), span, -1);
            } else {
                self.f.push(Instr::Pop, span, -1);
            }
            self.khoi(&e.body)?;
            let lend = self.f.here();
            self.f.patch(j_end, lend);
            return Ok(());
        }
        let c_ok = self.g.cst_str("Ok");
        let c_some = self.g.cst_str("Some");
        self.f.push(Instr::Dup, span, 1);
        self.f.push(Instr::TestCtor(c_ok, 1), span, 0);
        self.f.push(Instr::Un(vis_types::hir::UnOp::Not), span, 0);
        let j_ok = self.f.here();
        self.f.push(Instr::JumpIfFalse(0), span, -1);
        self.f.push(Instr::Dup, span, 1);
        self.f.push(Instr::TestCtor(c_some, 1), span, 0);
        let j_else = self.f.here();
        self.f.push(Instr::JumpIfFalse(0), span, -1);
        // nhánh khớp: bóc đối số rồi bind
        let ok = self.f.here();
        self.f.patch(j_ok, ok);
        self.f.push(Instr::Unwrap(0), span, 0);
        self.bind(pat, span, true)?;
        let j_end = self.f.here();
        self.f.push(Instr::Jump(0), span, 0);
        // nhánh `else`
        let le = self.f.here();
        self.f.patch(j_else, le);
        if let Some(sym) = e.bind_sym {
            self.f.push(Instr::Unwrap(0), span, 0);
            // **ST-238 (VD-242, ca 3)**: `slot_cua` để `let … else err:` cấp tệp ghi đúng ô chung.
            let slot = self.slot_cua(sym)?;
            self.f.push(Instr::BindLocal(slot), span, -1);
        } else {
            self.f.push(Instr::Pop, span, -1);
        }
        self.khoi(&e.body)?;
        let lend = self.f.here();
        self.f.patch(j_end, lend);
        Ok(())
    }
}

impl Emit<'_> {
    /// **R11 (VD-178)**: dựng bản ghi — tên kiểu lấy từ `recs` (rỗng nếu ẩn danh).
    pub(super) fn record_lit(
        &mut self,
        fields: &[(String, HExpr)],
        ty: &vis_types::Type,
        span: Span,
    ) -> Result<(), CodegenError> {
        for (n, v) in fields {
            let c = self.g.cst_str(n);
            self.f.push(Instr::Const(c), span, 1);
            self.expr(v)?;
            // **VD-258**: trường `T?` nhận giá trị plain ⇒ nâng ở biên dựng bản ghi.
            if let vis_types::Type::Record(id) = ty {
                if let Some(ft) = self.g.rec_fields.get(&(id.0, n.clone())).cloned() {
                    let depth = super::than::opt_depth(&ft);
                    if depth > 0 {
                        self.lift_opt(depth, span)?;
                    }
                }
            }
        }
        let ten = match ty {
            vis_types::Type::Record(id) => self.g.recs.get(&id.0).cloned().unwrap_or_default(),
            _ => String::new(),
        };
        let t = self.g.cst_str(&ten);
        self.f.push(
            Instr::MakeRecord(t, fields.len() as u16),
            span,
            1 - 2 * fields.len() as i32,
        );
        self.net_stack(span)
    }
}
