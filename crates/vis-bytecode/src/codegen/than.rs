// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thân hàm: `HBody::Expr` ⇒ biểu thức; `HBody::Stmts` ⇒ câu lệnh (giá trị cuối giữ lại).
//! Tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng. **VD-228**: `-> T?` ⇒ tự nâng `Some(…)`
//! ở biên hàm (mirror interp ở `call_hfn_trong`).

use super::*;
use vis_types::Type;

/// **ST-248 (VD-242 §3.3)** — gom kiểu trường bản ghi `RecId → (tên trường → kiểu)` từ HIR.
/// Dùng để biết kiểu đích của `u.f = v` mà không cần `TypeTable` (API `compile` giữ nguyên).
pub(super) fn collect_rec_fields(hir: &Hir) -> HashMap<(u32, String), Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        if let HItem::Rec(r) = it {
            for f in &r.fields {
                m.insert((r.id.0, f.name.clone()), f.ty.clone());
            }
        }
    }
    m
}

/// **ST-248** — suy kiểu tĩnh của một biểu thức HIR (chỉ các dạng mang kiểu hiển thị); `None`
/// khi không biết ⇒ không nâng (an toàn, không đoán ngữ nghĩa mới).
fn infer_ty(e: &HExpr, sym: &HashMap<u32, Type>) -> Option<Type> {
    match e {
        HExpr::Lit { ty, .. } | HExpr::Record { ty, .. } => Some(ty.clone()),
        HExpr::Map { key, value, .. } => {
            Some(Type::Map(Box::new(key.clone()), Box::new(value.clone())))
        }
        HExpr::List { elem, .. } | HExpr::Set { elem, .. } => {
            Some(Type::List(Box::new(elem.clone())))
        }
        HExpr::Var { sym: Some(id), .. } => sym.get(id).cloned(),
        HExpr::Call { callee, args, .. } => match (&**callee, args.first()) {
            (HExpr::Var { name, .. }, Some(a)) if name == "Some" => {
                infer_ty(a, sym).map(|t| Type::Opt(Box::new(t)))
            }
            (HExpr::Var { name, .. }, _) if name == "None" => {
                Some(Type::Opt(Box::new(Type::Error)))
            }
            _ => None,
        },
        _ => None,
    }
}

/// **ST-248** — gán kiểu suy được từ pattern `let` đơn (`HPat::Bind`); dạng khác bỏ qua.
fn bind_pat(p: &HPat, ty: &Type, sym: &mut HashMap<u32, Type>) {
    if let HPat::Bind { sym: id, .. } = p {
        sym.insert(*id, ty.clone());
    }
}

/// **ST-248** — đi khối câu lệnh để gom kiểu biến không có annotation (vd `var u = U(…)` ⇒
/// `u: Record(U)`). Chỉ đọc cấu trúc HIR, không gọi mã người dùng (VD-242 §3.4).
fn walk_stmts(sts: &[HStmt], sym: &mut HashMap<u32, Type>) {
    for s in sts {
        match s {
            HStmt::Var {
                sym: id, ty, init, ..
            } => {
                if ty.is_none() {
                    if let Some(t) = infer_ty(init, sym) {
                        sym.insert(*id, t);
                    }
                }
            }
            HStmt::Let { pat, ty, init, .. } => {
                if ty.is_none() {
                    if let Some(t) = infer_ty(init, sym) {
                        bind_pat(pat, &t, sym);
                    }
                }
            }
            HStmt::If {
                then, elifs, else_, ..
            } => {
                walk_stmts(then, sym);
                for (_, b) in elifs {
                    walk_stmts(b, sym);
                }
                if let Some(b) = else_ {
                    walk_stmts(b, sym);
                }
            }
            HStmt::For { body, .. }
            | HStmt::While { body, .. }
            | HStmt::Loop { body, .. }
            | HStmt::Scope { body, .. }
            | HStmt::Block { body, .. } => walk_stmts(body, sym),
            HStmt::Match { arms, .. } => {
                for a in arms {
                    walk_stmts(&a.body, sym);
                }
            }
            _ => {}
        }
    }
}

/// **ST-248** — `sym` → kiểu đầy đủ = annotation + kiểu suy từ initializer (mirror interp).
pub(super) fn collect_sym_tys(hir: &Hir, symbols: &Symbols) -> HashMap<u32, Type> {
    let mut m = crate::util::sym_annot_map(symbols);
    walk_stmts(&hir.stmts, &mut m);
    for it in &hir.items {
        match it {
            HItem::Fn(f) => walk_stmts(&f.body, &mut m),
            HItem::Impl(i) => {
                for meth in &i.methods {
                    walk_stmts(&meth.body, &mut m);
                }
            }
            _ => {}
        }
    }
    m
}

/// Số lớp `Opt` của một **đích kiểu** — đếm từ ngoài vào (mirror vis-interp `opt_depth_ty`).
pub(super) fn opt_depth(t: &vis_types::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_types::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

impl Emit<'_> {
    /// **VD-241 §3 (A0, 2026-11-19)** — nâng giá trị Opt tới **≥ `depth` lớp** (hàm. nội bộ
    /// `$lift-opt`, interp/VM dùng chung; người dùng không viết được tên `$…`).
    /// Stack trước: [v] → sau: [v'] với `v'` có tối thiểu `depth` lớp `Opt` từ ngoài vào.
    pub(super) fn lift_opt(&mut self, depth: usize, span: Span) -> Result<(), CodegenError> {
        let d = self.g.cst(vis_types::LitKind::Int, &depth.to_string());
        self.f.push(Instr::Const(d), span, 1);
        let n = self.g.cst_str("$lift-opt");
        self.f.push(Instr::CallBuiltin(n, 2), span, -1);
        Ok(())
    }

    /// **VD-228** (siết theo VD-241 §3) — tự nâng `T → …T?…` ở biên hàm khi annot return là Optional.
    pub(super) fn lift_ret_opt(&mut self, span: Span) -> Result<(), CodegenError> {
        if self.ret_depth > 0 {
            self.lift_opt(self.ret_depth, span)?;
        }
        Ok(())
    }

    /// **VD-228b** — tự nâng `T → T?` ở biên **tham số** annot Optional: giá trị plain vào
    /// ô cục bộ tham số ⇒ gói `Some(…)` (mirror interp ở `call_hfn_trong`).
    pub(super) fn lift_params_opt(
        &mut self,
        params: &[vis_types::HParam],
        span: Span,
    ) -> Result<(), CodegenError> {
        for p in params {
            let depth = opt_depth(&p.ty);
            if depth > 0 {
                let slot = self
                    .f
                    .locals
                    .get(&p.sym)
                    .copied()
                    .ok_or(CodegenError::Unsupported(
                        "tham số không có ô cục bộ",
                        p.span,
                    ))?;
                self.f.push(Instr::LoadLocal(slot), span, 1);
                self.lift_opt(depth, span)?; // VD-241 §3: nâng tới đủ lớp tham số.
                self.f.push(Instr::StoreLocal(slot), span, -1);
            }
        }
        Ok(())
    }

    pub(super) fn than(&mut self, b: &HBody, _base: u16, span: Span) -> Result<(), CodegenError> {
        match b {
            HBody::Expr(e) => {
                // **VD-257 (stream B)** — HIR coi thân `Expr` là MỘT câu nguồn; interp charge
                // `E_STMT` ở đây (`interp.rs::exec_stmt`). VD-256 đã vá đường tệp/top-level nhưng
                // sót đường thân hàm (`fn f(): expr`) ⇒ parity compiler↔interp lệch đúng 1 sự kiện.
                //charge E_STMT để dãy sự kiện khớp; không đổi opcode/ngữ nghĩa thực thi.
                self.f.charge(FeeKind::Stmt, e.span());
                self.expr(e)?;
                self.lift_ret_opt(span)?; // VD-228 (siết VD-241 §3): biên hàm `-> T?`
                Ok(())
            }
            HBody::Stmts(st) => {
                // Câu lệnh cuối là biểu thức ⇒ **giữ** giá trị làm kết quả hàm (giống interp).
                self.nhanh_gia_tri(st)?;
                self.lift_ret_opt(span)?; // VD-228 (siết VD-241 §3): biên hàm `-> T?`
                self.f.push(Instr::Return, span, 0);
                self.net_stack(span)
            }
        }
    }

    /// **VD-241 §3+ (A0, 2026-11-19)** — nâng giá trị ở biên **annot `Option`** (`let`/`var`):
    /// giá trị đã có đủ lớp giữ nguyên; thiếu thì bọc `Some(…)` tới đủ lớp. `None` trần được
    /// giữ nguyên bởi `lift_opt` (bước 1 — không bọc).
    pub(super) fn lift_to_annot(
        &mut self,
        ty: Option<&vis_types::Type>,
        span: Span,
    ) -> Result<(), CodegenError> {
        if let Some(t) = ty {
            let depth = opt_depth(t);
            if depth > 0 {
                self.lift_opt(depth, span)?;
            }
        }
        Ok(())
    }

    /// **ST-248 (VD-242 §3.3)** — kiểu tĩnh của biểu thức **gốc** (`Var`/`Field`/`Index`), dùng để
    /// suy kiểu đích của phép gán trường/chỉ số. `None` ⇒ không biết ⇒ không nâng.
    fn base_ty(&self, e: &vis_types::HExpr) -> Option<Type> {
        match e {
            vis_types::HExpr::Var { sym: Some(id), .. } => self.g.sym_ty.get(id).cloned(),
            vis_types::HExpr::Field { base, name, .. } => match self.base_ty(base) {
                Some(Type::Record(id)) => self.g.rec_fields.get(&(id.0, name.clone())).cloned(),
                _ => None,
            },
            vis_types::HExpr::Index { base, .. } => match self.base_ty(base) {
                Some(Type::Map(_, v)) => Some(*v),
                Some(Type::List(el)) => Some(*el),
                _ => None,
            },
            _ => None,
        }
    }

    /// **ST-248 (VD-242 §3.3)** — số lớp `Opt` của **đích gán** (0 ⇒ giữ nguyên).
    /// `Var` dùng kiểu biến (không đổi theo từng lần gán); `Field`/`Index` dùng kiểu khai báo
    /// của trường/phần tử. `lift_opt` tự dừng ở đủ lớp ⇒ không nâng nhiều lớp, `None` giữ nguyên.
    pub(super) fn assign_target_depth(&self, t: &vis_types::HExpr) -> usize {
        let ty = match t {
            vis_types::HExpr::Var { sym: Some(id), .. } => self.g.sym_ty.get(id).cloned(),
            other => self.base_ty(other),
        };
        ty.as_ref().map(opt_depth).unwrap_or(0)
    }

    /// **VD-241 §3+ / ST-248** — nâng giá trị ở biên gán (đích `var`, `field`, `index`) tới đủ lớp
    /// `Opt` của kiểu đích; đích không-Opt giữ nguyên.
    pub(super) fn lift_assign_set(
        &mut self,
        target: &vis_types::HExpr,
        span: Span,
    ) -> Result<(), CodegenError> {
        let depth = self.assign_target_depth(target);
        if depth > 0 {
            self.lift_opt(depth, span)?;
        }
        Ok(())
    }
}
