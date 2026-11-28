// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318e (VD-297)** — phát **biểu thức** (`expr`) của codegen, tách nguyên văn khỏi
//! `codegen.rs` để mỗi tệp ≤ 700 dòng (R1 mềm, 0 đổi hành vi/VBC1).

use super::*;

impl<'g> Emit<'g> {
    // ----- biểu thức -----

    pub(super) fn expr(&mut self, e: &HExpr) -> Result<(), CodegenError> {
        // **ST-276 (VD-256)**: E_EXPR cho nút nguồn thực sự đánh giá. Nút đã có sự kiện riêng
        // (call/spawn/await) không cộng E_EXPR — các nhánh đó tự charge.
        if !matches!(
            e,
            HExpr::Call { .. } | HExpr::Spawn { .. } | HExpr::Await { .. }
        ) {
            self.f.charge(FeeKind::Expr, e.span());
        }
        match e {
            HExpr::Lit { lit, span, .. } => {
                let c = self.g.cst(lit.kind, &lit.text);
                self.f.push(Instr::Const(c), *span, 1);
                Ok(())
            }
            HExpr::Var { sym, name, span } => {
                // **VD-208 (2)**: builtin là **giá trị dựng sẵn** (dùng được như hàm ở cả hai đường).
                if let Some(id) = sym {
                    if self.gia_tri_builtin_var(*id, *span)? {
                        return Ok(());
                    }
                }
                // **R2a (VD-178)**: `None` là **biến thể** của `Option`, không phải biến/hàm.
                if name == "None" && !self.g.fmap.contains_key(&sym.unwrap_or(u32::MAX)) {
                    self.f.push(Instr::MakeNone, *span, 1);
                    return Ok(());
                }
                // **ST-238 (VD-242, ca 4)**: `const` cấp tệp — inline giá trị như interp nạp lúc load.
                if let Some(id) = sym {
                    if let Some(v) = self.g.hconsts.get(id).cloned() {
                        // **ST-314a**: `const A = B + 1` / `const B = A + 1` từng đệ quy vô hạn
                        // (VM stack overflow). Theo dõi đường inline; gặp lại ⇒ lỗi codegen.
                        if !self.g.hconsts_dang.insert(*id) {
                            return Err(CodegenError::ConstCycle(*span));
                        }
                        let kq = self.expr(&v);
                        self.g.hconsts_dang.remove(id);
                        return kq;
                    }
                }
                match sym {
                    Some(id) if self.g.fmap.contains_key(id) => {
                        // Tên hàm dùng như **giá trị** ⇒ closure 0 biến bắt.
                        let fi = self.g.fmap[id];
                        self.f.push(Instr::MakeClosure(fi, 0), *span, 1);
                    }
                    Some(id) => {
                        // **ST-319a (#206)**: closure đã chụp global thành **ô cục bộ riêng**
                        // ⇒ ưu tiên ô cục bộ (đọc theo by-value tại điểm tạo closure); ngoài
                        // closure, biến cấp tệp vẫn dùng ô chung như VD-242.
                        if let Some(s) = self.captured.get(id).copied() {
                            self.f.push(Instr::LoadCaptured(s), *span, 1);
                        } else if let Some(s) = self.f.locals.get(id).copied() {
                            self.f.push(Instr::LoadLocal(s), *span, 1);
                        } else if let Some(s) = self.g.globals.get(id).copied() {
                            self.f.push(Instr::LoadLocal(s), *span, 1);
                        } else {
                            let slot = self.f.slot(*id, *span)?;
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                        }
                    }
                    None => {
                        // Tên dựng sẵn / hàm chưa rõ: gọi theo tên 0 đối số (VM phân giải).
                        let n = self.g.cst_str(name);
                        self.f.push(Instr::CallBuiltin(n, 0), *span, 1);
                    }
                }
                Ok(())
            }
            HExpr::Bin { op, lhs, rhs, span } => {
                use vis_hir::BinOp;
                match op {
                    // `and`/`or`/`??` **đoản mạch** — không gọi bảng ngữ nghĩa (interp cũng vậy).
                    BinOp::And => {
                        self.expr(lhs)?;
                        self.f.push(Instr::Dup, *span, 1);
                        let j = self.f.here();
                        self.f.push(Instr::JumpIfFalse(0), *span, -1);
                        self.f.push(Instr::Pop, *span, -1);
                        self.expr(rhs)?;
                        let end = self.f.here();
                        self.f.patch(j, end);
                        Ok(())
                    }
                    BinOp::Or => {
                        self.expr(lhs)?;
                        self.f.push(Instr::Dup, *span, 1);
                        self.f.push(Instr::Un(vis_hir::UnOp::Not), *span, 0);
                        let j = self.f.here();
                        self.f.push(Instr::JumpIfFalse(0), *span, -1);
                        self.f.push(Instr::Pop, *span, -1);
                        self.expr(rhs)?;
                        let end = self.f.here();
                        self.f.patch(j, end);
                        Ok(())
                    }
                    BinOp::Coalesce => {
                        self.expr(lhs)?;
                        self.f.push(Instr::Dup, *span, 1);
                        let jn = self.f.here();
                        self.f.push(Instr::JumpIfNone(0), *span, -1);
                        self.f.push(Instr::UnwrapOpt, *span, 0);
                        let jend = self.f.here();
                        self.f.push(Instr::Jump(0), *span, 0);
                        let lnone = self.f.here();
                        self.f.patch(jn, lnone);
                        self.f.push(Instr::Pop, *span, -1);
                        self.expr(rhs)?;
                        let end = self.f.here();
                        self.f.patch(jend, end);
                        Ok(())
                    }
                    _ => {
                        self.expr(lhs)?;
                        self.expr(rhs)?;
                        self.f.push(Instr::Bin(*op), *span, -1);
                        Ok(())
                    }
                }
            }
            HExpr::Un { op, expr, span } => {
                self.expr(expr)?;
                self.f.push(Instr::Un(*op), *span, 0);
                Ok(())
            }
            HExpr::Call {
                callee,
                args,
                named,
                span,
            } => {
                let n = (args.len() + named.len()) as u16;
                // **VD-258**: `xs.push(v)` với `xs: list[T?]` — nâng `v` plain ở biên phương thức.
                let push_depth = match &**callee {
                    HExpr::Field { base, name, .. } if name == "push" => self.push_elem_depth(base),
                    _ => 0,
                };
                // **ST-276**: call site có sự kiện riêng, tính **trước** đối số (thứ tự ngôn ngữ).
                // **VD-258**: phân loại `E_BUILTIN`/`E_CALL` + `E_EXPR` của callee nằm ở
                // `than::call_fee` để `codegen.rs` giữ R1 ≤ 1500 dòng.
                let (fee_kind, callee_la_gia_tri) = than::call_fee(self.g, callee);
                self.f.charge(fee_kind, *span);
                if callee_la_gia_tri {
                    self.f.charge(FeeKind::Expr, callee.span());
                }
                match &**callee {
                    HExpr::Field { optional: true, .. } => {
                        return self.goi_phuong_thuc_tuy_chon(callee, args, named, *span);
                    }
                    HExpr::Var { name, sym, .. } => {
                        // Xác định **cách gọi trước** khi đẩy đối số (thứ tự ngăn xếp: hàm ⇒ đối số).
                        let fi = match sym {
                            Some(id) if self.g.fmap.contains_key(id) => Some(self.g.fmap[id]),
                            _ => self.g.fmap_ten.get(name).copied(),
                        };
                        let dong = fi.is_none() && !la_builtin(name);
                        // Gọi bằng **`--tên`**: xếp lại đối số theo **thứ tự tham số** của hàm.
                        if !named.is_empty() {
                            if let Some(ds) = self.g.fparams.get(name).cloned() {
                                let mut vt = 0usize;
                                for (pname, _la_named) in ds.iter() {
                                    // **R5a (VD-178)**: ưu tiên **vị trí**, rồi tới **tên**.
                                    if let Some(a) = args.get(vt) {
                                        self.expr(a)?;
                                        vt += 1;
                                    } else if let Some((_, a)) =
                                        named.iter().find(|(n, _)| n == pname)
                                    {
                                        self.expr(a)?;
                                    } else {
                                        self.f.push(Instr::PushUnit, *span, 1);
                                    }
                                }
                                let fi =
                                    self.g.fmap_ten.get(name).copied().or_else(|| {
                                        sym.and_then(|id| self.g.fmap.get(&id).copied())
                                    });
                                match fi {
                                    Some(fi) => {
                                        let tong = ds.len() as u16;
                                        self.f.push(
                                            Instr::Call(fi, tong),
                                            *span,
                                            1 - i32::from(tong),
                                        );
                                        return self.net_stack(*span);
                                    }
                                    None => {
                                        let c = self.g.cst_str(name);
                                        let tong = ds.len() as u16;
                                        self.f.push(
                                            Instr::CallBuiltin(c, tong),
                                            *span,
                                            1 - i32::from(tong),
                                        );
                                        return self.net_stack(*span);
                                    }
                                }
                            }
                        }
                        // **ST-60 (VD-95 (2))**: callee là `async fn` ⇒ `$tac_vu(handle, args…)`;
                        // **không** thêm tag `VBC1`, verifier/cache **không** đổi.
                        if let Some(fi) = fi {
                            if named.is_empty() && self.g.async_fns.contains(&fi) {
                                let c = self.g.cst(LitKind::Int, &fi.to_string());
                                self.f.push(Instr::Const(c), *span, 1);
                                for a in args {
                                    self.expr(a)?;
                                }
                                let n = (args.len() + 1) as u16;
                                let b = self.g.cst_str("$tac_vu");
                                self.f
                                    .push(Instr::CallBuiltin(b, n), *span, 1 - i32::from(n));
                                return self.net_stack(*span);
                            }
                        }
                        if dong {
                            if let Some(id) = sym {
                                if let Some(c) = self.captured.get(id).copied() {
                                    self.f.push(Instr::LoadCaptured(c), *span, 1);
                                } else {
                                    let slot = self.slot_cua(*id)?;
                                    self.f.push(Instr::LoadLocal(slot), *span, 1);
                                }
                            }
                        }
                        for a in args {
                            self.expr(a)?;
                        }
                        for (_, a) in named {
                            self.expr(a)?;
                        }
                        match fi {
                            Some(fi) => self.f.push(Instr::Call(fi, n), *span, 1 - i32::from(n)),
                            None if dong && sym.is_some() => {
                                self.f.push(Instr::CallValue(n), *span, -i32::from(n))
                            }
                            None => {
                                let c = self.g.cst_str(name);
                                self.f
                                    .push(Instr::CallBuiltin(c, n), *span, 1 - i32::from(n));
                            }
                        }
                    }
                    HExpr::Field { base, name, .. } => {
                        // **ST-319d (#208)**: `push` trên **`var`** phải ghi lại ô ở **mọi** ngữ
                        // cảnh (biểu thức cuối, đối số, `let y = …`), không chỉ câu lệnh; và đọc
                        // **thẳng ô** như `lay_o` của interp (không charge `E_EXPR` cho receiver).
                        let dich = match (&**base, name.as_str()) {
                            (HExpr::Var { sym: Some(id), .. }, "push")
                                if self.g.mutables.contains(id) =>
                            {
                                Some(self.slot_cua(*id)?)
                            }
                            _ => None,
                        };
                        match dich {
                            Some(slot) => self.f.push(Instr::LoadLocal(slot), *span, 1),
                            None => self.expr(base)?,
                        }
                        for a in args {
                            self.expr(a)?;
                            if push_depth > 0 {
                                self.lift_opt(push_depth, *span)?;
                            }
                        }
                        let c = self.g.cst_str(name);
                        self.f.push(Instr::CallMethod(c, n), *span, -i32::from(n));
                        // **Assign NGAY sau `CallMethod`** để `tach_dich` (vm.rs:580-604) tách
                        // đúng ô ⇒ `push` **tại chỗ O(1)** (ST-316a/VD-292); `LoadLocal` giữ hợp
                        // đồng "expr() để lại đúng 1 giá trị".
                        if let Some(slot) = dich {
                            self.f.push(Instr::Assign(slot), *span, -1);
                            self.f.push(Instr::LoadLocal(slot), *span, 1);
                        }
                    }
                    other => {
                        // Gọi **kết quả biểu thức** (vd `f(1)(2)`) ⇒ đẩy hàm trước, rồi đối số.
                        self.expr(other)?;
                        for a in args {
                            self.expr(a)?;
                        }
                        self.f.push(Instr::CallValue(n), *span, -i32::from(n));
                    }
                }
                self.net_stack(*span)
            }
            HExpr::Field {
                base,
                name,
                optional,
                span,
            } => {
                // `E.A` khi `E` là **enum đã khai**: dựng biến thể, không phải truy cập trường.
                if let HExpr::Var { sym: Some(id), .. } = &**base {
                    if let Some(ten) = self.g.enums.get(id).cloned() {
                        let c = self.g.cst_str(&format!("{ten}.{name}"));
                        self.f.push(Instr::MakeEnum(c, 0), *span, 1);
                        return Ok(());
                    }
                }
                if *optional {
                    // **VD-258**: `None` ⇒ `None`; `Some(u)` ⇒ `Some(u.f)` (field optional thật giữ thêm một lớp).
                    self.expr(base)?;
                    self.f.push(Instr::Dup, *span, 1);
                    let jn = self.f.here();
                    self.f.push(Instr::JumpIfNone(0), *span, -1);
                    self.f.push(Instr::UnwrapOpt, *span, 0);
                    let c = self.g.cst_str(name);
                    self.f.push(Instr::Field(c), *span, 0);
                    // **VD-258**: field optional thật (`T?`) ⇒ luôn `MakeSome`; field thường
                    // dùng `lift_opt` như cũ (giá trị hợp lệ không-Option ⇒ `Some(v)`).
                    self.field_optional_boc(base, name, *span)?;
                    let jend = self.f.here();
                    self.f.push(Instr::Jump(0), *span, 0);
                    let lnone = self.f.here();
                    self.f.patch(jn, lnone);
                    self.f.push(Instr::Pop, *span, -1);
                    self.f.push(Instr::MakeNone, *span, 1);
                    let end = self.f.here();
                    self.f.patch(jend, end);
                    return Ok(());
                }
                self.expr(base)?;
                let c = self.g.cst_str(name);
                self.f.push(Instr::Field(c), *span, 0);
                Ok(())
            }
            HExpr::Index { base, index, span } => {
                self.expr(base)?;
                self.expr(index)?;
                self.f.push(Instr::Index, *span, -1);
                Ok(())
            }
            HExpr::List { items, span, .. } => {
                for it in items {
                    self.expr(it)?;
                }
                self.f.push(
                    Instr::MakeList(items.len() as u16),
                    *span,
                    1 - items.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Set { items, span, .. } => {
                for it in items {
                    self.expr(it)?;
                }
                self.f.push(
                    Instr::MakeSet(items.len() as u16),
                    *span,
                    1 - items.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Tuple { items, span, .. } => {
                // **VD-258**: `()` là `unit` duy nhất; empty tuple phát `PushUnit`,
                // không `MakeTuple(0)` (VM sẽ tạo `Value::list([])`).
                if items.is_empty() {
                    self.f.push(Instr::PushUnit, *span, 1);
                    return self.net_stack(*span);
                }
                for it in items {
                    self.expr(it)?;
                }
                self.f.push(
                    Instr::MakeTuple(items.len() as u16),
                    *span,
                    1 - items.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Map { entries, span, .. } => {
                for (k, v) in entries {
                    self.expr(k)?;
                    self.expr(v)?;
                }
                self.f.push(
                    Instr::MakeMap(entries.len() as u16),
                    *span,
                    1 - 2 * entries.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Record { fields, ty, span } => self.record_lit(fields, ty, *span),
            HExpr::EnumLit {
                enum_id,
                path,
                variant,
                args,
                span,
                ..
            } => {
                for (i, a) in args.iter().enumerate() {
                    self.expr(a)?;
                    // **VD-258**: đối số biến thể `T?` nhận plain — nâng ở biên dựng enum.
                    self.lift_enum_arg(*enum_id, variant, i, *span)?;
                }
                // Giữ **cả** tên enum (nếu biết) để `MakeEnum` dựng đúng `Value::Enum`.
                let mut ten = path.join(".");
                if ten.is_empty() {
                    ten = variant.clone();
                } else {
                    ten = format!("{ten}.{variant}");
                }
                let v = self.g.cst_str(&ten);
                self.f.push(
                    Instr::MakeEnum(v, args.len() as u16),
                    *span,
                    1 - args.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Opt { value, span, .. } => {
                match value {
                    Some(v) => {
                        self.expr(v)?;
                        self.f.push(Instr::MakeSome, *span, 0);
                    }
                    None => self.f.push(Instr::MakeNone, *span, 1),
                }
                Ok(())
            }
            HExpr::Try { expr, span } => {
                // **R8 (VD-178)**: `xs[i]?` — chỉ số **có kiểm tra** ⇒ `T?` (không panic).
                if let HExpr::Index { base, index, .. } = &**expr {
                    self.expr(base)?;
                    self.expr(index)?;
                    self.f.push(Instr::IndexOpt, *span, -1);
                    return Ok(());
                }
                self.expr(expr)?;
                self.f.push(Instr::Try, *span, 0);
                Ok(())
            }
            HExpr::With { base, fields, span } => {
                self.expr(base)?;
                let rec_id = match self.base_ty(base) {
                    Some(vis_hir::Type::Record(id)) => Some(id),
                    _ => None,
                };
                for (n, v) in fields {
                    let c = self.g.cst_str(n);
                    self.f.push(Instr::Const(c), *span, 1);
                    self.expr(v)?;
                    // **VD-258**: cập nhật trường `T?` bằng plain ⇒ nâng như biên dựng bản ghi.
                    self.lift_with_field(rec_id, n, *span)?;
                }
                self.f.push(
                    Instr::With(fields.len() as u16),
                    *span,
                    -2 * fields.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::If {
                cond,
                then,
                elifs,
                else_,
                span,
                ..
            } => self.expr_if(cond, then, elifs, else_, *span),
            HExpr::Match {
                subject,
                arms,
                span,
                ..
            } => {
                self.expr(subject)?;
                let (min, max) = than::do_match_emit(self.g, arms);
                let mut jend = Vec::new();
                let mut guardas: Vec<usize> = Vec::new();
                let mut cuoi: Vec<(usize, Vec<usize>)> = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
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
                    self.nhanh_gia_tri(&arm.body)?;
                    self.lift_nhanh(min, max, *span)?;
                    jend.push(self.f.here());
                    self.f.push(Instr::Jump(0), *span, 0);
                    let next = self.f.here();
                    if i + 1 == arms.len() {
                        // Nhánh **cuối** trượt ⇒ phải `Pop` subject trước khi đẩy giá trị dự phòng.
                        cuoi.push((br, guardas.clone()));
                    } else {
                        self.f.patch(br, next);
                        for g in guardas.drain(..) {
                            self.f.patch(g, next);
                        }
                    }
                }
                let lpop = self.f.here();
                self.f.push(Instr::Pop, *span, -1);
                for (br, gs) in cuoi {
                    self.f.patch(br, lpop);
                    for g in gs {
                        self.f.patch(g, lpop);
                    }
                }
                // **ST-325d (nợ #2)**: rơi khỏi mọi nhánh ở **ngữ cảnh giá trị** cũng phải trap
                // `VS0501` — cùng interp (`eval_match`) và nhánh câu lệnh (`cau.rs`). Trước đây
                // đường này đẩy `false` im lặng ⇒ VM exit 0 trong khi interp exit 101.
                // `keep_value = true`: verifier vẫn thấy một giá trị như mỗi nhánh khác.
                self.trap_no_arm(*span, true)?;
                let end = self.f.here();
                for j in jend {
                    self.f.patch(j, end);
                }
                Ok(())
            }
            HExpr::Lambda {
                params,
                body,
                span,
                ret,
                ..
            } => {
                let idx = self.g.funcs.len() as u32;
                self.g.funcs.push(Func {
                    name: "<closure>".to_string(),
                    ..Func::default()
                });
                self.g.fees.push(Vec::new());
                // snapshot **cả khung hiện tại** (VD-51) — thứ tự ô = thứ tự slot.
                let mut pair: Vec<(u32, u16)> =
                    self.f.locals.iter().map(|(k, v)| (*k, *v)).collect();
                // **ST-319a (#206)**: biến cấp tệp **có xuất hiện trong thân** cũng được chụp
                // theo **giá trị tại điểm tạo closure** (SPEC §4.7) — thân closure không còn đọc
                // ô global live. Sym đã nằm trong `f.locals` (closure ngoài đã chụp) giữ ô cục bộ
                // đó, không chụp lại từ kho global.
                let mut tham_chieu = std::collections::BTreeSet::new();
                super::bien_tu_do::quet_body(body, &mut tham_chieu);
                let mut da_co: std::collections::HashSet<u32> =
                    pair.iter().map(|(s, _)| *s).collect();
                for sym in tham_chieu {
                    if let Some(s) = self.g.globals.get(&sym).copied() {
                        if da_co.insert(sym) {
                            pair.push((sym, s));
                        }
                    }
                }
                pair.sort_by_key(|(_, v)| *v);
                for (_sym, slot) in &pair {
                    let s = *slot;
                    self.f.push(Instr::LoadLocal(s), *span, 1);
                }
                self.g.cho.push(Cho {
                    idx,
                    params: params.clone(),
                    body: body.clone(),
                    captured: pair.clone(),
                    name: "<closure>".to_string(),
                    ret_depth: than::opt_depth(ret),
                });
                self.f.push(
                    Instr::MakeClosure(idx, pair.len() as u16),
                    *span,
                    1 - pair.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Interp { parts, span } => {
                for p in parts {
                    match p {
                        InterpPart::Str(s) => {
                            let c = self.g.cst(LitKind::Str, &format!("\"{s}\""));
                            self.f.push(Instr::Const(c), *span, 1);
                        }
                        InterpPart::Expr(x) => self.expr(x)?,
                        // **R6c (VD-178)**: `{expr:spec}` — đẩy giá trị rồi `Format(spec)`.
                        InterpPart::ExprSpec { expr, spec } => {
                            self.expr(expr)?;
                            let c = self.g.cst(LitKind::Str, &format!("\"{spec}\""));
                            self.f.push(Instr::Format(c), *span, 0);
                        }
                    }
                }
                self.f.push(
                    Instr::Interp(parts.len() as u16),
                    *span,
                    1 - parts.len() as i32,
                );
                self.net_stack(*span)
            }
            HExpr::Error { span } => {
                // ST-54: HIR còn nút **lỗi** ⇒ từ chối, không phát chuỗi rỗng cho VM chạy "sạch".
                Err(CodegenError::Unsupported("error-expr", *span))
            }
            // **ST-56 (VD-92)**: `ref(expr)` hạ **identity** (không sinh lệnh) — hết
            // `Unsupported("ref")`; **chưa** có mượn thật (nợ VS2+).
            HExpr::Ref { expr, .. } => self.expr(expr),
            // **ST-61 (VD-96)**: `chan[T](n)` ⇒ `$kenh(n)` — **không** thêm tag `VBC1` (như `$spawn`).
            HExpr::ChanNew { cap, span, .. } => {
                self.expr(cap)?;
                let b = self.g.cst_str("$kenh");
                self.f.push(Instr::CallBuiltin(b, 1), *span, 0);
                self.net_stack(*span)
            }
            // **ST-60 (VD-95 (2))**: `await` = `$await(<task>)` — **không** thêm tag `VBC1`.
            HExpr::Await { expr, span } => {
                // **ST-276**: E_AWAIT cho await **trong nguồn**; final join không tạo await giả.
                self.f.charge(FeeKind::Await, *span);
                self.expr(expr)?;
                let c = self.g.cst_str("$await");
                self.f.push(Instr::CallBuiltin(c, 1), *span, 0);
                self.net_stack(*span)
            }
            // **ST-60**: `spawn f(args)` = `$spawn(handle, args…)`.
            HExpr::Spawn {
                callee,
                args,
                named,
                span,
            } => {
                // **ST-276**: $spawn nội bộ chỉ mang E_SPAWN (không cộng E_BUILTIN).
                self.f.charge(FeeKind::Spawn, *span);
                let fi = match &**callee {
                    HExpr::Var { name, sym, .. } => match sym {
                        Some(id) if self.g.fmap.contains_key(id) => Some(self.g.fmap[id]),
                        _ => self.g.fmap_ten.get(name).copied(),
                    },
                    _ => None,
                };
                let Some(fi) = fi else {
                    return Err(CodegenError::Unsupported("spawn-khong-ten", *span));
                };
                let c = self.g.cst(LitKind::Int, &fi.to_string());
                self.f.push(Instr::Const(c), *span, 1);
                for a in args {
                    self.expr(a)?;
                }
                for (_, a) in named {
                    self.expr(a)?;
                }
                let n = (args.len() + named.len() + 1) as u16;
                let b = self.g.cst_str("$spawn");
                self.f
                    .push(Instr::CallBuiltin(b, n), *span, 1 - i32::from(n));
                self.net_stack(*span)
            }
            HExpr::CmdBlock { span, .. } => Err(CodegenError::Unsupported("cmd-block", *span)),
        }
    }
}
