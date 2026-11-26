// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — đánh giá biểu thức (`eval_cond`/`eval`), tách khỏi `interp.rs`.
//! Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use super::*;
impl<'a> Interp<'a> {
    /// **ST-314a (hậu-kiểm 2026-11-26)** — giá trị một `const` cấp tệp, **lazy + memo**:
    /// đánh giá phụ thuộc trước, chèn vào `env`, và bắt **chu trình** (`const A = B`, `const B = A`)
    /// bằng `const_dang` thay vì để interp chạy vào trạng thái không tất định.
    pub(crate) fn gia_tri_const(&mut self, sym: u32, span: Span) -> R<Value> {
        if let Some(v) = self.env.borrow().get(&sym).cloned() {
            return Ok(v);
        }
        if !self.const_dang.insert(sym) {
            let ten = self.symbols.name_of(sym).unwrap_or("<const>").to_string();
            return Err(Abort::Trap(Trap::new(
                RT,
                format!("chu trình `const` tại `{ten}`"),
                span,
            )));
        }
        let Some(e) = self.consts.get(&sym).copied() else {
            self.const_dang.remove(&sym);
            return Err(Abort::Trap(Trap::new(RT, "hằng chưa có giá trị", span)));
        };
        let kq = self.eval(e);
        self.const_dang.remove(&sym);
        let v = kq?;
        self.env.borrow_mut().insert(sym, v.clone());
        Ok(v)
    }

    pub(crate) fn eval_cond(&mut self, c: &HCond) -> R<bool> {
        match c {
            HCond::Expr(e) => {
                let v = self.eval(e)?;
                v.truthy(e.span())
            }
            HCond::Let { pat, value, .. } => {
                let v = self.eval(value)?;
                // **VD-241 §3 (A0, 2026-11-19)** — `if let Some(x) = o:`/`if let Ok(x) = r:` khớp
                // theo cấu trúc giá trị thật; KHÔNG bọc ngầm giá trị trần. Giá trị không khớp
                // pattern ⇒ không vào nhánh (và không bọc/bóc phụ).
                let snapshot = self.env.borrow().clone();
                if !self.bind_pat_bool(pat, &v) {
                    *self.env.borrow_mut() = snapshot;
                    return Ok(false);
                }
                Ok(true)
            }
        }
    }

    // ---------------------------------------------------------------------
    // Biểu thức
    // ---------------------------------------------------------------------

    pub(crate) fn eval(&mut self, e: &HExpr) -> R<Value> {
        self.steps += 1;
        if let Some(a) = self.worker_tick(e.span()) {
            return Err(a);
        }
        // ST-276: E_EXPR cho nút nguồn; call/spawn/await có sự kiện riêng.
        if !matches!(
            e,
            HExpr::Call { .. } | HExpr::Spawn { .. } | HExpr::Await { .. }
        ) {
            if let Some(a) = self.charge(crate::rt::fee::FeeKind::Expr, e.span()) {
                return Err(a);
            }
        }
        match e {
            HExpr::Lit { lit, span, .. } => {
                lit_value(lit).map_err(|m| Abort::Trap(Trap::new(RT, m, *span)))
            }
            HExpr::Var { name, sym, span } => match sym {
                Some(id) => {
                    if let Some(v) = self.env.borrow().get(id).cloned() {
                        return Ok(v);
                    }
                    // **ST-314a**: `const` chưa nạp ⇒ nạp lazy (xử lý tham chiếu phía trước).
                    if self.consts.contains_key(id) {
                        return self.gia_tri_const(*id, *span);
                    }
                    if let Some(f) = self.fns.get(id) {
                        return Ok(Value::Fn(Rc::new(Closure {
                            name: f.name.clone(),
                            params: f.params.iter().map(|p| p.sym).collect(),
                            param_depths: f.params.iter().map(|p| opt_depth_ty(&p.ty)).collect(),
                            body: HBody::Stmts(f.body.clone()),
                            env: self.env.clone(),
                            ret_depth: opt_depth_ty(&f.ret),
                            frame_body: std::cell::OnceCell::new(),
                        })));
                    }
                    // `enum`/`type` khai trong tệp: thẻ kiểu để `E.A` / `User(...)` chạy được.
                    if let Some((n, is_enum)) = self.tags.get(id).cloned() {
                        return Ok(if is_enum {
                            Value::Enum(n.into(), Name::new(""), Vec::new())
                        } else {
                            Value::Record(n, Vec::new())
                        });
                    }
                    // `print`/`len`/`str`/`Ok`/... **có** `sym` (ST-11 khai 8 tên dựng sẵn) ⇒ tra tên.
                    if self.symbols.kind_of(*id) == Some(vis_hir::SymKind::Builtin) {
                        let n = self.symbols.name_of(*id).unwrap_or_default();
                        // **R2a (VD-178)**: `None` là **biến thể** của `Option`, không phải hàm.
                        if n == "None" {
                            return Ok(Value::none());
                        }
                        if let Some(b) = builtin_name(n) {
                            return Ok(Value::Builtin(b));
                        }
                    }
                    Err(Abort::Trap(Trap::new(
                        RT,
                        format!("tên `{name}` chưa có giá trị lúc chạy"),
                        *span,
                    )))
                }
                None => match builtin_name(name) {
                    // **R2a (VD-178)**: `None` là **biến thể** của `Option`, không phải hàm.
                    Some("None") => Ok(Value::none()),
                    Some(b) => Ok(Value::Builtin(b)),
                    None => Err(Abort::Trap(Trap::new(
                        RT,
                        format!("tên `{name}` chưa hỗ trợ ở VS0"),
                        *span,
                    ))),
                },
            },
            HExpr::Bin { op, lhs, rhs, span } => match op {
                BinOp::Coalesce => {
                    let l = self.eval(lhs)?;
                    if l.is_some() {
                        Ok(l.into_some().expect("vừa kiểm is_some"))
                    } else if l.is_none() {
                        self.eval(rhs)
                    } else {
                        // Không phải `Option` — `??` chỉ trên `T?`; giữ nguyên hành vi cũ (giá trị thô).
                        Ok(l)
                    }
                }
                BinOp::And => {
                    let l = self.eval(lhs)?;
                    if l.truthy(lhs.span())? {
                        let r = self.eval(rhs)?;
                        Ok(Value::Bool(r.truthy(rhs.span())?))
                    } else {
                        Ok(Value::Bool(false))
                    }
                }
                BinOp::Or => {
                    let l = self.eval(lhs)?;
                    if l.truthy(lhs.span())? {
                        Ok(Value::Bool(true))
                    } else {
                        let r = self.eval(rhs)?;
                        Ok(Value::Bool(r.truthy(rhs.span())?))
                    }
                }
                BinOp::Pipe => {
                    let l = self.eval(lhs)?;
                    let f = self.eval(rhs)?;
                    self.call_value(f, vec![l], Vec::new(), *span)
                }
                _ => {
                    let l = self.eval(lhs)?;
                    let r = self.eval(rhs)?;
                    crate::builtin::bin(*op, l, r, *span)
                }
            },
            HExpr::Un { op, expr, span } => {
                let v = self.eval(expr)?;
                crate::builtin::un(*op, v, *span)
            }
            HExpr::Call {
                callee,
                args,
                named,
                span,
            } => {
                // ST-276: call charge trước đối số (f(g(1)) ⇒ E_CALL(f), E_CALL(g), E_EXPR(1)).
                let host = match &**callee {
                    HExpr::Field { .. } => true, // method trên giá trị = host
                    HExpr::Var { name, sym, .. } => {
                        sym.and_then(|id| self.fns.get(&id)).is_none()
                            && builtin_name(name).is_some()
                    }
                    _ => false,
                };
                let kind = if host {
                    crate::rt::fee::FeeKind::Builtin
                } else {
                    crate::rt::fee::FeeKind::Call
                };
                if let Some(a) = self.charge(kind, *span) {
                    return Err(a);
                }
                self.eval_call(callee, args, named, *span)
            }
            HExpr::Field {
                base,
                name,
                optional,
                span,
            } => self.eval_field(base, name, *optional, *span),
            HExpr::Index { base, index, span } => {
                let b = self.eval(base)?;
                let i = self.eval(index)?;
                self.index_of(b, i, *span)
            }
            HExpr::List { items, .. } => {
                let mut vs = Vec::with_capacity(items.len());
                for it in items {
                    vs.push(self.eval(it)?);
                }
                Ok(Value::List(vs))
            }
            HExpr::Set { items, .. } => {
                let mut vs: Vec<Value> = Vec::new();
                for it in items {
                    let v = self.eval(it)?;
                    if !vs.iter().any(|x| x.eq_value(&v)) {
                        vs.push(v);
                    }
                }
                Ok(Value::List(vs))
            }
            HExpr::Map { entries, .. } => {
                // **ST-239 (VD-242 §3.4, ca 170)**: runtime last-wins — mỗi khoá một entry,
                // giữ giá trị cuối và vị trí chèn đầu. Khoá trùng **tĩnh** đã bị checker chặn.
                // **ST-315b**: gộp last-wins **một nguồn** ở `VisMap::from_pairs` (bỏ quét O(N²)).
                let mut kv: Vec<(Value, Value)> = Vec::with_capacity(entries.len());
                for (k, v) in entries {
                    kv.push((self.eval(k)?, self.eval(v)?));
                }
                Ok(Value::map(kv))
            }
            HExpr::Record { fields, ty, span } => {
                let name = self.type_name(ty);
                let rec_id = match ty {
                    vis_hir::Type::Record(id) => Some(*id),
                    _ => None,
                };
                let mut out = Vec::with_capacity(fields.len());
                for (n, v) in fields {
                    let vv = self.eval(v)?;
                    // **VD-258**: trường `T?` nhận plain — nâng ở biên dựng bản ghi.
                    out.push((n.clone(), self.lift_truong_ban_ghi(rec_id, n, vv)));
                    let _ = span;
                }
                Ok(Value::Record(name, out))
            }
            HExpr::EnumLit {
                enum_id,
                path,
                variant,
                args,
                ty,
                span,
                ..
            } => {
                let mut vs = Vec::with_capacity(args.len());
                for (i, a) in args.iter().enumerate() {
                    let vv = self.eval(a)?;
                    // **VD-258**: đối số biến thể `T?` nhận plain — nâng ở biên dựng enum.
                    vs.push(self.lift_doi_so_bien_the(*enum_id, variant, i, vv));
                }
                // **VD-267**: `EnumLit` nóng — nhớ tên theo **vị trí nguồn** (span), xác minh
                // nội dung rồi chỉ clone `Name`; không tra bảng tên mỗi vòng. Giữ **đúng**
                // fallback cũ: tên kiểu rỗng/không có ⇒ `type_name` rồi `path.first()`.
                if let vis_hir::Type::Enum(id) = ty {
                    if let Some(e) = self.types.enum_def(*id) {
                        if !e.name.is_empty() {
                            let key = (span.file, span.start, span.end);
                            let hit = self.enum_tags.borrow().get(&key).cloned();
                            if let Some((n, v)) = hit {
                                // Hai `EnumLit` khác nhau lỡ trùng span ⇒ xác minh nội dung.
                                if n.as_str() == e.name && v.as_str() == variant.as_str() {
                                    return Ok(Value::Enum(n, v, vs));
                                }
                            }
                            let ten = Name::new(&e.name);
                            let bien_the = Name::new(variant);
                            self.enum_tags
                                .borrow_mut()
                                .insert(key, (ten.clone(), bien_the.clone()));
                            return Ok(Value::Enum(ten, bien_the, vs));
                        }
                    }
                }
                let name = self.type_name(ty);
                let name = if name.is_empty() {
                    Name::new(path.first().map(String::as_str).unwrap_or(""))
                } else {
                    Name::new(&name)
                };
                let _ = span;
                Ok(Value::Enum(name, Name::new(variant), vs))
            }
            HExpr::Opt { value, .. } => match value {
                None => Ok(Value::none()),
                Some(v) => {
                    let inner = self.eval(v)?;
                    Ok(Value::some(inner))
                }
            },
            HExpr::Try { expr, .. } if self.fn_depth == 0 => {
                // ST-13b (mục 8): `?` ngoài hàm là lỗi (§4.4) — trước đây bị nuốt thành `return`.
                Err(Abort::Trap(Trap::new(
                    RT,
                    "`?` chỉ dùng **trong hàm** (§4.4)",
                    e.span(),
                )))
            }
            // **R8 (VD-178)**: `xs[i]?` — chỉ số **có kiểm tra** ⇒ `T?` (không panic).
            HExpr::Try { expr, .. } if matches!(&**expr, HExpr::Index { .. }) => {
                let HExpr::Index { base, index, .. } = &**expr else {
                    unreachable!()
                };
                let b = self.eval(base)?;
                let i = self.eval(index)?;
                Ok(match (&b, &i) {
                    (Value::List(xs), Value::Int(n)) if *n >= 0 => {
                        Value::opt(xs.get(*n as usize).cloned())
                    }
                    (Value::Bytes(bs), Value::Int(n)) if *n >= 0 => {
                        Value::opt(bs.get(*n as usize).map(|x| Value::Int(*x as i64)))
                    }
                    (Value::Map(kv), k) => Value::opt(kv.get(k).cloned()),
                    _ => Value::none(),
                })
            }
            HExpr::Try { expr, .. } => match self.eval(expr)? {
                // **VD-204 (1)**: `Option`/`Result` thật — `?` bóc `Some(v)`/`Ok(v)`, đẩy `None`/`Err(e)`
                // về làm giá trị trả sớm; `?` trên kiểu khác ⇒ lỗi như cũ.
                Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => {
                    Ok(ag.into_iter().next().expect("Some luôn có một đối số"))
                }
                Value::Enum(ea, va, ag) if ea == "Option" && va == "None" => {
                    Err(Abort::Ret(Value::Enum(ea, va, ag)))
                }
                Value::Enum(ea, va, ag) if ea == "Result" && va == "Ok" && ag.len() == 1 => {
                    Ok(ag.into_iter().next().expect("Ok luôn có một đối số"))
                }
                Value::Enum(ea, va, ag) if ea == "Result" && va == "Err" && ag.len() == 1 => {
                    Err(Abort::Ret(Value::Enum(ea, va, ag)))
                }
                other => Err(Abort::Trap(Trap::new(
                    RT,
                    format!("`?` cần `T?`/`Result`, được `{}`", other.show()),
                    e.span(),
                ))),
            },
            HExpr::With { base, fields, span } => {
                let b = self.eval(base)?;
                // **VD-263**: ưu tiên `RecId` instantiation từ kiểu tĩnh của gốc (annot).
                let rec_id_tinh = self.rec_id_tinh(base, &b);
                match b {
                    Value::Record(name, mut cur) => {
                        // **VD-258**: `with` nâng trường `T?` như biên dựng bản ghi.
                        let rec_id = rec_id_tinh.or_else(|| self.types.rec_by_name(&name));
                        for (n, v) in fields {
                            let nv = self.eval(v)?;
                            let nv = self.lift_truong_ban_ghi(rec_id, n, nv);
                            match cur.iter_mut().find(|(f, _)| f == n) {
                                Some(slot) => slot.1 = nv,
                                None => {
                                    return Err(Abort::Trap(Trap::new(
                                        RT,
                                        format!("bản ghi không có trường `{n}`"),
                                        *span,
                                    )))
                                }
                            }
                        }
                        Ok(Value::Record(name, cur))
                    }
                    other => Err(Abort::Trap(Trap::new(
                        RT,
                        format!("`with` cần bản ghi, được `{}`", other.show()),
                        *span,
                    ))),
                }
            }
            HExpr::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => self.eval_if(cond, then, elifs, else_),
            HExpr::Match {
                subject,
                arms,
                span,
                ..
            } => self.eval_match(subject, arms, *span),
            HExpr::Lambda {
                params, body, ret, ..
            } => Ok(Value::Fn(Rc::new(Closure {
                name: String::new(),
                params: params.iter().map(|p| p.sym).collect(),
                param_depths: params.iter().map(|p| opt_depth_ty(&p.ty)).collect(),
                body: body.clone(),
                ret_depth: opt_depth_ty(ret),
                // ST-15b (mục 2): **chụp snapshot** môi trường tại điểm tạo closure — interp dùng
                // một env phẳng và khôi phục tham số khi hàm trả về, nên nếu giữ `Rc` chung thì
                // closure thoát khỏi hàm sẽ mất biến đã bắt. Chọn snapshot (ghi JOURNAL): biến
                // `var` cập nhật **sau** khi tạo closure thì closure vẫn thấy giá trị lúc tạo.
                env: Rc::new(std::cell::RefCell::new(self.env.borrow().clone())),
                frame_body: std::cell::OnceCell::new(),
            }))),
            HExpr::Tuple { items, .. } => {
                // **VD-258**: `()` là `unit` duy nhất — empty tuple không được là `Value::List([])`
                // (trước đây `()` in `[]`, khác `return` trần in `()`).
                if items.is_empty() {
                    return Ok(Value::Unit);
                }
                let mut vs = Vec::with_capacity(items.len());
                for it in items {
                    vs.push(self.eval(it)?);
                }
                Ok(Value::List(vs))
            }
            // **ST-61 (VD-96)**: `chan[T](n)` ⇒ `$kenh(n)` (một hiện thực ở `builtin`, dùng chung
            // cho cả interp lẫn VM).
            HExpr::ChanNew { cap, span, .. } => {
                let c = self.eval(cap)?;
                crate::builtin::call(self, "$kenh", vec![c], *span)
            }
            // **ST-56 (VD-92)**: `ref(expr)` giữ **nguyên kiểu trong**, hạ **identity** — **không**
            // tạo giá trị mới, **không** mượn thật (borrow/vòng đời là nợ **VS2+**).
            HExpr::Ref { expr, .. } => self.eval(expr),
            // **ST-60 (VD-90)**: `await t` chạy tác vụ **tới hoàn tất** (hoặc trả cache).
            HExpr::Await { expr, span } => {
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Await, *span) {
                    return Err(a);
                }
                let v = self.eval(expr)?;
                match v {
                    Value::Task(t) => crate::rt::await_task(self, &t, *span),
                    _ => Err(Abort::Trap(Trap::new(
                        RT,
                        "`await` cần một `Task[T]`",
                        *span,
                    ))),
                }
            }
            // **ST-60**: `spawn f(args)` = tạo tác vụ + đưa vào hàng đợi FIFO.
            HExpr::Spawn {
                callee,
                args,
                named,
                span,
            } => {
                // **ST-276**: $spawn nội bộ chỉ mang E_SPAWN, không cộng E_BUILTIN.
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Spawn, *span) {
                    return Err(a);
                }
                let _ = named;
                let id = match &**callee {
                    HExpr::Var { sym: Some(id), .. } => Some(*id),
                    _ => None,
                };
                let Some(id) = id else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`spawn` cần **tên** hàm `async` cấp tệp",
                        *span,
                    )));
                };
                let Some(f) = self.fns.get(&id).copied() else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`spawn` cần **tên** hàm `async` cấp tệp",
                        *span,
                    )));
                };
                let mut pos = Vec::with_capacity(args.len());
                for a in args {
                    pos.push(self.eval(a)?);
                }
                // P1/P2: `Off` giữ nguyên đường cũ; `Local` chạy caller; `Parallel` đẩy pool.
                let mode = self.sched.borrow().exec.mode;
                match mode {
                    crate::rt::exec::WorkerMode::Off => {
                        let t = self
                            .sched
                            .get_mut()
                            .spawn(f.sym, pos, f.name.clone(), *span);
                        Ok(Value::Task(t))
                    }
                    crate::rt::exec::WorkerMode::Local => {
                        crate::rt::exec::spawn_local(self, f.sym, pos, f.name.clone(), *span)
                    }
                    crate::rt::exec::WorkerMode::Parallel(_) => {
                        crate::rt::exec::spawn_parallel(self, f.sym, pos, f.name.clone(), *span)
                    }
                }
            }
            // ST-14: chuỗi nội suy — đánh giá từng mảnh rồi ghép (không parse lại lúc chạy).
            HExpr::Interp { parts, .. } => {
                let mut out = String::new();
                for p in parts {
                    match p {
                        vis_hir::InterpPart::Str(t) => out.push_str(t),
                        vis_hir::InterpPart::Expr(e) => {
                            let v = self.eval(e)?;
                            out.push_str(&v.show());
                        }
                        // **R6c (VD-178)**: `{expr:spec}` — định dạng lúc chạy.
                        vis_hir::InterpPart::ExprSpec { expr, spec } => {
                            let v = self.eval(expr)?;
                            out.push_str(&dinh_dang(&v, spec, expr.span())?);
                        }
                    }
                }
                Ok(Value::Str(out))
            }
            HExpr::CmdBlock { span, .. } => Err(Abort::Trap(Trap::new(
                RT,
                "khối `cmd` là VS1 — chưa hỗ trợ ở VS0",
                *span,
            ))),
            HExpr::Error { span } => Err(Abort::Trap(Trap::new(
                RT,
                "biểu thức không hạ được (HIR `error`)",
                *span,
            ))),
        }
    }
}
