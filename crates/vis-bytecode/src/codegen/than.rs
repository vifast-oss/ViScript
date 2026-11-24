// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thân hàm: `HBody::Expr` ⇒ biểu thức; `HBody::Stmts` ⇒ câu lệnh (giá trị cuối giữ lại).
//! Tách khỏi `codegen.rs` để giữ **R1** ≤ 1.500 dòng. **VD-228**: `-> T?` ⇒ tự nâng `Some(…)`
//! ở biên hàm (mirror interp ở `call_hfn_trong`).

use super::*;
use crate::fee::FeeKind;
use vis_hir::Type;

mod infer;

use infer::*;

/// **VD-258** — phân loại sự kiện phí của một call-site (một nguồn cho codegen):
/// `Field` (host method) là `E_BUILTIN`; `Var` builtin không phải hàm tệp cũng vậy.
/// Trả thêm `callee_la_gia_tri` để codegen charge `E_EXPR` đúng một lần khi callee
/// thực sự được đánh giá như giá trị (biến/closure/builtin có `sym`), không charge
/// cho hàm tệp/method/`std.*` gọi thẳng.
pub(super) fn call_fee(g: &Gen, callee: &HExpr) -> (FeeKind, bool) {
    let la_host = match callee {
        HExpr::Field { .. } => true,
        HExpr::Var { name, .. } => !g.fmap_ten.contains_key(name.as_str()) && la_builtin(name),
        _ => false,
    };
    let callee_la_gia_tri = match callee {
        HExpr::Var { sym, .. } => match sym {
            Some(id) => !g.fmap.contains_key(id),
            None => !la_host,
        },
        _ => false,
    };
    (
        if la_host {
            FeeKind::Builtin
        } else {
            FeeKind::Call
        },
        callee_la_gia_tri,
    )
}

/// **ST-248 (VD-242 §3.3)** — gom kiểu trường bản ghi `RecId → (tên trường → kiểu)` từ HIR.
/// Dùng để biết kiểu đích của `u.f = v` mà không cần `TypeTable` (API `compile` giữ nguyên).
///
/// **VD-263**: khi có [`vis_hir::TypeTable`] (`compile_voi_bang`), bảng này là **nguồn đầy đủ** — gồm cả
/// các `RecId` **instantiation** (`Box[int?]`) với trường đã thay `T`; HIR chỉ có bản khai báo.
pub(super) fn collect_rec_fields(
    hir: &Hir,
    types: Option<&vis_hir::TypeTable>,
) -> HashMap<(u32, String), Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        if let HItem::Rec(r) = it {
            for f in &r.fields {
                m.insert((r.id.0, f.name.clone()), f.ty.clone());
            }
        }
    }
    if let Some(types) = types {
        for (i, r) in types.recs().iter().enumerate() {
            for f in &r.fields {
                m.insert((i as u32, f.name.clone()), f.ty.clone());
            }
        }
    }
    m
}

/// **VD-258** — gom kiểu trường biến thể enum `(EnumId, tên biến thể, vị trí) → kiểu`.
/// Dùng để nâng `T → T?` ở đối số dựng enum (interp đọc `TypeTable` trực tiếp).
///
/// **VD-263**: như [`collect_rec_fields`], có `TypeTable` ⇒ gồm cả enum **instantiation**.
pub(super) fn collect_enum_fields(
    hir: &Hir,
    types: Option<&vis_hir::TypeTable>,
) -> HashMap<(u32, String, usize), Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        if let HItem::Enum(e) = it {
            for v in &e.variants {
                for (i, f) in v.fields.iter().enumerate() {
                    m.insert((e.id.0, v.name.clone(), i), f.ty.clone());
                }
            }
        }
    }
    if let Some(types) = types {
        for (i, e) in types.enums().iter().enumerate() {
            for v in &e.variants {
                for (j, f) in v.fields.iter().enumerate() {
                    m.insert((i as u32, v.name.clone(), j), f.ty.clone());
                }
            }
        }
    }
    m
}

fn la_error(t: &Type) -> bool {
    matches!(t, Type::Error)
}

/// **VD-260** — `sym` hàm thường/phương thức `impl` → kiểu trả về. `HExpr::Call` chỉ có `sym`
/// của callee nên codegen cần bảng này để suy lớp Option của lời gọi (`f() -> int?`).
pub(super) fn collect_fn_rets(hir: &Hir) -> HashMap<u32, Type> {
    let mut m = HashMap::new();
    for it in &hir.items {
        match it {
            HItem::Fn(f) => {
                m.insert(f.sym, f.ret.clone());
            }
            HItem::Impl(i) => {
                for meth in &i.methods {
                    m.insert(meth.sym, meth.ret.clone());
                }
            }
            _ => {}
        }
    }
    m
}

/// **VD-265** — bảng kiểu trả phương thức người dùng; kèm `TypeTable` (nếu có) để thay generic
/// cho instantiation. Tách hàm để `Gen` khởi tạo một dòng và giữ `codegen.rs` ≤ R1.
pub(super) fn collect_mret(hir: &Hir, types: Option<&vis_hir::TypeTable>) -> MethodRet {
    types
        .map(|t| MethodRet::from_hir_voi_bang(hir, t))
        .unwrap_or_else(|| MethodRet::from_hir(hir))
}

/// **VD-265** — gọi [`do_match`] với các bảng của `Gen` (một dòng ở `codegen.rs`, giữ R1).
pub(super) fn do_match_emit(g: &Gen, arms: &[vis_hir::hir::HArm]) -> (usize, usize) {
    do_match(arms, &g.sym_ty, &g.rec_fields, &g.fn_rets, &g.mret)
}

/// **VD-264** — số nút `Error` trong một kiểu (đệ quy). Dùng để chọn giữa kiểu lower và kiểu
/// suy từ phần tử: kiểu suy chỉ thắng khi **ít lỗi hơn** (không đổi kiểu đã biết đầy đủ).
fn so_loi(t: &Type) -> usize {
    match t {
        Type::Error => 1,
        Type::Opt(x) | Type::List(x) => so_loi(x),
        Type::Map(k, v) | Type::Result(k, v) => so_loi(k) + so_loi(v),
        Type::TypeRef { args, .. } => args.iter().map(so_loi).sum(),
        Type::Fn(params, ret) => params.iter().map(so_loi).sum::<usize>() + so_loi(ret),
        _ => 0,
    }
}

/// **VD-264** — thay `a` bằng kiểu suy `s` khi `a` là `Error` (giữ hành vi VD-260) hoặc `s`
/// có **ít nút `Error` hơn** (suy sâu hơn mà không đổi kiểu đã biết đầy đủ).
fn chon_kieu(a: &mut Type, s: Type) {
    if la_error(a) || so_loi(&s) < so_loi(a) {
        *a = s;
    }
}

/// **VD-260** — kiểu của giá trị cuối một khối (`None` = khối không cho giá trị).
fn infer_than(
    sts: &[HStmt],
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    match sts.last()? {
        HStmt::Expr(e) => infer_ty(e, sym, fn_rets, mret, rec_fields),
        HStmt::If {
            then, elifs, else_, ..
        } => infer_if(
            then,
            elifs,
            else_.as_deref(),
            sym,
            fn_rets,
            mret,
            rec_fields,
        ),
        HStmt::Match { arms, .. } => infer_match(arms, sym, fn_rets, mret, rec_fields),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            infer_than(body, sym, fn_rets, mret, rec_fields)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => None,
    }
}

/// **VD-260** — chọn kiểu nhánh có **nhiều lớp Option nhất** (hoà ⇒ nhánh sau; chỉ độ sâu
/// được dùng). Checker đã chấp nhận chương trình nên không cần hợp nhất đầy đủ ở đây.
///
/// **VD-269**: ưu tiên nhánh **ít `{error}` hơn** (vd `Some(U(…))`/`U(…)` vs `None` ⇒ lấy
/// `U`, không lấy `Option[{error}]`) rồi **nâng** lên đủ độ sâu `Option` của nhánh sâu nhất —
/// đúng biên nâng nhánh VD-258. Nhờ vậy codegen phân giải được phương thức của `recv?.m(…)`.
fn hop_nhanh(ds: Vec<Type>) -> Option<Type> {
    let max = ds.iter().map(opt_depth).max()?;
    let mut best: Option<Type> = None;
    let mut it_loi = usize::MAX;
    for t in ds {
        let loi = so_loi(&t);
        if loi <= it_loi {
            it_loi = loi;
            best = Some(t);
        }
    }
    let mut t = best?;
    while opt_depth(&t) < max {
        t = Type::Opt(Box::new(t));
    }
    Some(t)
}

fn infer_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    let else_ = else_?;
    let mut ds = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(t) = infer_than(body, sym, fn_rets, mret, rec_fields) {
            ds.push(t);
        }
    }
    hop_nhanh(ds)
}

fn infer_match(
    arms: &[vis_hir::hir::HArm],
    sym: &HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
) -> Option<Type> {
    let ds: Vec<Type> = arms
        .iter()
        .filter_map(|a| infer_than(&a.body, sym, fn_rets, mret, rec_fields))
        .collect();
    hop_nhanh(ds)
}

/// **VD-264** — kiểu phần tử của biểu thức `for` (mirror checker `elem_of_iter`).
fn elem_of_iter_ty(t: &Type) -> Option<Type> {
    match t {
        Type::List(x) => Some((**x).clone()),
        Type::Map(k, v) => Some(Type::TypeRef {
            name: "tuple".into(),
            args: vec![(**k).clone(), (**v).clone()],
        }),
        Type::Str => Some(Type::Char),
        Type::TypeRef { name, args } if name == "range" => {
            Some(args.first().cloned().unwrap_or(Type::Int))
        }
        _ => None,
    }
}

/// **VD-263** — bind biến của `if let` / `elif let` theo kiểu giá trị vế phải (nếu biết).
fn bind_cond_let(
    c: &HCond,
    sym: &mut HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
    enum_fields: &HashMap<(u32, String, usize), Type>,
) {
    if let HCond::Let { pat, value, .. } = c {
        if let Some(t) = infer_ty(value, sym, fn_rets, mret, rec_fields) {
            bind_pat_ty(pat, &t, sym, rec_fields, enum_fields);
        }
    }
}

/// **ST-248 / VD-260** — đi khối câu lệnh để gom kiểu biến không có annotation (vd
/// `var u = U(…)`/`var u = pick(true)` ⇒ `u: Record(U)`) và biến bind trong pattern
/// (`match`/`if let`/`for`). Chỉ đọc cấu trúc HIR, không gọi mã người dùng (VD-242 §3.4).
fn walk_stmts(
    sts: &[HStmt],
    sym: &mut HashMap<u32, Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
    rec_fields: &HashMap<(u32, String), Type>,
    enum_fields: &HashMap<(u32, String, usize), Type>,
) {
    for s in sts {
        match s {
            HStmt::Var {
                sym: id, ty, init, ..
            } => {
                // **VD-260**: annotation `Error` (không có/không phân giải) vẫn cho phép suy
                // từ initializer — trước đây chỉ `ty.is_none()` nên `var` thiếu kiểu bị bỏ.
                let t = ty
                    .clone()
                    .filter(|t| !la_error(t))
                    .or_else(|| infer_ty(init, sym, fn_rets, mret, rec_fields));
                if let Some(t) = t {
                    sym.insert(*id, t);
                }
            }
            HStmt::Let { pat, ty, init, .. } => {
                // **VD-258**: annotation của `let` (`let u: U? = …`) trước đây không vào
                // `sym_ty` (chỉ `var`/tham số có), nên `u?.f` không biết kiểu field để lift.
                let t = ty
                    .clone()
                    .filter(|t| !la_error(t))
                    .or_else(|| infer_ty(init, sym, fn_rets, mret, rec_fields));
                if let Some(t) = t {
                    bind_pat_ty(pat, &t, sym, rec_fields, enum_fields);
                }
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                bind_cond_let(cond, sym, fn_rets, mret, rec_fields, enum_fields);
                walk_stmts(then, sym, fn_rets, mret, rec_fields, enum_fields);
                for (c, b) in elifs {
                    bind_cond_let(c, sym, fn_rets, mret, rec_fields, enum_fields);
                    walk_stmts(b, sym, fn_rets, mret, rec_fields, enum_fields);
                }
                if let Some(b) = else_ {
                    walk_stmts(b, sym, fn_rets, mret, rec_fields, enum_fields);
                }
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                // **VD-260**: biến vòng lặp lấy kiểu phần tử của list (nếu biết).
                // **VD-264**: dùng `elem_of_iter` như checker (list/map/str/range) — trước đây
                // truyền **cả** kiểu iter nên pattern tuple/ctor trong `for` mất kiểu.
                if let Some(t) =
                    infer_ty(iter, sym, fn_rets, mret, rec_fields).and_then(|t| elem_of_iter_ty(&t))
                {
                    bind_pat_ty(pat, &t, sym, rec_fields, enum_fields);
                }
                walk_stmts(body, sym, fn_rets, mret, rec_fields, enum_fields);
            }
            HStmt::While { cond, body, .. } => {
                // **VD-264**: `while let Some(x) = …` — biến của pattern phải có kiểu tĩnh
                // trong thân như `if let` (interp/codegen cùng luật).
                bind_cond_let(cond, sym, fn_rets, mret, rec_fields, enum_fields);
                walk_stmts(body, sym, fn_rets, mret, rec_fields, enum_fields);
            }
            HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
                walk_stmts(body, sym, fn_rets, mret, rec_fields, enum_fields)
            }
            HStmt::Match { subject, arms, .. } => {
                // **VD-263**: biến bind trong arm (`Some(u)`, `[u]`, `{f}`, ctor enum) lấy
                // kiểu từ chủ thể — trước đây arm chỉ được đi thân, không bind kiểu.
                let subj = infer_ty(subject, sym, fn_rets, mret, rec_fields);
                for a in arms {
                    if let Some(t) = &subj {
                        bind_pat_ty(&a.pat, t, sym, rec_fields, enum_fields);
                    }
                    walk_stmts(&a.body, sym, fn_rets, mret, rec_fields, enum_fields);
                }
            }
            _ => {}
        }
    }
}

/// **ST-248 / VD-260** — `sym` → kiểu đầy đủ = annotation + kiểu suy từ initializer (mirror
/// interp). `self` trong method `impl` lấy kiểu bản ghi của `impl` (kể cả khi không annot).
pub(super) fn collect_sym_tys(
    hir: &Hir,
    symbols: &Symbols,
    types: Option<&vis_hir::TypeTable>,
) -> HashMap<u32, Type> {
    let mut m: HashMap<u32, Type> = crate::util::sym_annot_map(symbols)
        .into_iter()
        .filter(|(_, t)| !la_error(t))
        .collect();
    let fn_rets = collect_fn_rets(hir);
    let mret = types
        .map(|t| MethodRet::from_hir_voi_bang(hir, t))
        .unwrap_or_else(|| MethodRet::from_hir(hir));
    let rec_fields = collect_rec_fields(hir, types);
    let enum_fields = collect_enum_fields(hir, types);
    // `self` của `impl U` có kiểu `U` dù tham số không annot (lowering để `Error`).
    for it in &hir.items {
        if let HItem::Impl(i) = it {
            if la_error(&i.ty) {
                continue;
            }
            for meth in &i.methods {
                for p in &meth.params {
                    if p.is_self {
                        m.insert(p.sym, i.ty.clone());
                    }
                }
            }
        }
    }
    walk_stmts(
        &hir.stmts,
        &mut m,
        &fn_rets,
        &mret,
        &rec_fields,
        &enum_fields,
    );
    for it in &hir.items {
        match it {
            HItem::Fn(f) => walk_stmts(&f.body, &mut m, &fn_rets, &mret, &rec_fields, &enum_fields),
            HItem::Impl(i) => {
                for meth in &i.methods {
                    walk_stmts(
                        &meth.body,
                        &mut m,
                        &fn_rets,
                        &mret,
                        &rec_fields,
                        &enum_fields,
                    );
                }
            }
            _ => {}
        }
    }
    m
}

/// Số lớp `Opt` của một **đích kiểu** — đếm từ ngoài vào (mirror vis-interp `opt_depth_ty`).
pub(super) fn opt_depth(t: &vis_hir::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_hir::Type::Opt(inner) = cur {
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
        let d = self.g.cst(vis_hir::LitKind::Int, &depth.to_string());
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
        params: &[vis_hir::HParam],
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
        ty: Option<&vis_hir::Type>,
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
    ///
    /// **VD-263**: gốc không phải `Var`/`Field`/`Index` (literal bản ghi, lời gọi hàm/phương thức,
    /// `if`/`match`, `with` lồng, `expr?`) vẫn suy được bằng [`infer_ty`] — nhờ đó `with` lấy đúng
    /// `RecId` và nâng trường `T?` như interp (interp tra tên bản ghi lúc chạy; VM **không** dựa
    /// vào tên lúc chạy).
    /// **VD-264**: mọi dạng biểu thức đều đi qua [`infer_ty`] — `Var`, `Field` (kể cả `?.` trên
    /// `Opt(Record)`), `Index`, `Call` trả về, `Try`, `With`, `If`/`Match`, tuple/list literal.
    /// Trước đây nhánh `Var`/`Field`/`Index` riêng không bóc `Opt` cho `Field`, và `Call` là
    /// fallback nên `make().push(x)`/`lay()?.b` mù kiểu.
    pub(super) fn base_ty(&self, e: &vis_hir::HExpr) -> Option<Type> {
        match e {
            vis_hir::HExpr::Var { sym: Some(id), .. } => self.g.sym_ty.get(id).cloned(),
            other => infer_ty(
                other,
                &self.g.sym_ty,
                &self.g.fn_rets,
                &self.g.mret,
                &self.g.rec_fields,
            ),
        }
    }
}

/// **VD-258** — số lớp `Option` của biểu thức nếu biết **theo cú pháp** (`None`, `Some(…)`,
/// `if`/`match` lồng); dùng để nâng nhánh `if`/`match` lệch lớp.
fn do_bieu_thuc(
    e: &HExpr,
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> usize {
    match e {
        HExpr::Opt { ty, .. } => opt_depth(ty),
        HExpr::Var { name, .. } if name == "None" => 1,
        HExpr::Var { sym: Some(id), .. } => sym_ty.get(id).map(opt_depth).unwrap_or(0),
        HExpr::Field {
            base,
            name,
            optional: true,
            ..
        } => {
            // **VD-264**: dùng suy kiểu đầy đủ trước (base có thể là lời gọi/`?`/biến thể);
            // fallback cũ chỉ nhìn `Var` nên `a?.f` với `a` bind từ pattern/call bị đoán 1 lớp.
            if let Some(t) = infer_ty(e, sym_ty, fn_rets, mret, rec_fields) {
                return opt_depth(&t);
            }
            if let HExpr::Var { sym: Some(id), .. } = &**base {
                if let Some(Type::Opt(inner)) = sym_ty.get(id) {
                    if let Type::Record(rid) = &**inner {
                        if let Some(ft) = rec_fields.get(&(rid.0, name.clone())) {
                            return 1 + opt_depth(ft);
                        }
                    }
                }
            }
            1
        }
        // **VD-264**: `xs[i]?` ⇒ `list[T]` cho `T?`; giữ đúng lớp khi `T` đã là Option.
        HExpr::Try { expr, .. } if matches!(&**expr, HExpr::Index { .. }) => {
            infer_ty(e, sym_ty, fn_rets, mret, rec_fields)
                .map(|t| opt_depth(&t))
                .unwrap_or(1)
        }
        HExpr::Call { callee, args, .. } => match &**callee {
            HExpr::Var { name, .. } if name == "Some" => {
                1 + args
                    .first()
                    .map(|a| do_bieu_thuc(a, sym_ty, rec_fields, fn_rets, mret))
                    .unwrap_or(0)
            }
            HExpr::Var { name, .. } if name == "None" => 1,
            // **VD-260**: lời gọi hàm tệp/phương thức — kiểu trả về quyết định lớp Option.
            HExpr::Var { sym: Some(id), .. } => fn_rets.get(id).map(opt_depth).unwrap_or(0),
            // Method dựng sẵn (`list.get/first/…`, `map.get`) — dùng chung `infer_ty`.
            _ => infer_ty(e, sym_ty, fn_rets, mret, rec_fields)
                .map(|t| opt_depth(&t))
                .unwrap_or(0),
        },
        HExpr::If {
            then, elifs, else_, ..
        } => do_if(then, elifs, Some(else_), sym_ty, rec_fields, fn_rets, mret).1,
        HExpr::Match { arms, .. } => do_match(arms, sym_ty, rec_fields, fn_rets, mret).1,
        // **VD-260**: các dạng còn lại (method `get`/`first`/…, index, `with`, tuple) — dùng
        // chung suy kiểu đích để nhánh plain được nâng đúng lớp.
        _ => infer_ty(e, sym_ty, fn_rets, mret, rec_fields)
            .map(|t| opt_depth(&t))
            .unwrap_or(0),
    }
}

/// **VD-258** — số lớp `Option` của giá trị cuối một khối (`None` = khối không cho giá trị).
fn do_than(
    stmts: &[HStmt],
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<usize> {
    match stmts.last()? {
        HStmt::Expr(e) => Some(do_bieu_thuc(e, sym_ty, rec_fields, fn_rets, mret)),
        HStmt::If {
            then, elifs, else_, ..
        } => Some(
            do_if(
                then,
                elifs,
                else_.as_deref(),
                sym_ty,
                rec_fields,
                fn_rets,
                mret,
            )
            .1,
        ),
        HStmt::Match { arms, .. } => Some(do_match(arms, sym_ty, rec_fields, fn_rets, mret).1),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            do_than(body, sym_ty, rec_fields, fn_rets, mret)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => Some(0),
    }
}

/// **VD-258** — `(min, max)` lớp `Option` của các nhánh `if`; không `else` ⇒ `(0, 0)`.
pub(super) fn do_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> (usize, usize) {
    let Some(else_) = else_ else {
        return (0, 0);
    };
    let mut ds: Vec<usize> = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(d) = do_than(body, sym_ty, rec_fields, fn_rets, mret) {
            ds.push(d);
        }
    }
    if ds.is_empty() {
        return (0, 0);
    }
    let min = *ds.iter().min().unwrap_or(&0);
    let max = *ds.iter().max().unwrap_or(&0);
    (min, max)
}

/// **VD-258** — `(min, max)` lớp `Option` của các nhánh `match`.
pub(super) fn do_match(
    arms: &[vis_hir::hir::HArm],
    sym_ty: &HashMap<u32, Type>,
    rec_fields: &HashMap<(u32, String), Type>,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> (usize, usize) {
    let ds: Vec<usize> = arms
        .iter()
        .filter_map(|a| do_than(&a.body, sym_ty, rec_fields, fn_rets, mret))
        .collect();
    if ds.is_empty() {
        return (0, 0);
    }
    let min = *ds.iter().min().unwrap_or(&0);
    let max = *ds.iter().max().unwrap_or(&0);
    (min, max)
}

impl Emit<'_> {
    /// **VD-258** — nâng giá trị nhánh đã chọn tới `max` khi các nhánh lệch lớp.
    pub(super) fn lift_nhanh(
        &mut self,
        min: usize,
        max: usize,
        span: Span,
    ) -> Result<(), CodegenError> {
        if max > min {
            self.lift_opt(max, span)?;
        }
        Ok(())
    }

    /// **VD-258** — field của `?.` có kiểu `T?` không. Chỉ dựa vào kiểu tĩnh đã biết
    /// (`base_ty`); field optional thật ⇒ `MakeSome` vô điều kiện, field thường ⇒ `lift_opt` cũ.
    pub(super) fn field_la_opt(&self, base: &vis_hir::HExpr, name: &str) -> bool {
        let Some(bt) = self.base_ty(base) else {
            return false;
        };
        let inner = match bt {
            Type::Opt(t) => *t,
            other => other,
        };
        let Type::Record(id) = inner else {
            return false;
        };
        self.g
            .rec_fields
            .get(&(id.0, name.to_string()))
            .map(|t| t.is_opt())
            .unwrap_or(false)
    }

    /// **VD-258** — sau `Field` của `?.`: field `T?` ⇒ `MakeSome` vô điều kiện; field thường
    /// ⇒ `lift_opt(1)` như cũ (giá trị hợp lệ không-Option).
    pub(super) fn field_optional_boc(
        &mut self,
        base: &vis_hir::HExpr,
        name: &str,
        span: Span,
    ) -> Result<(), CodegenError> {
        if self.field_la_opt(base, name) {
            self.f.push(Instr::MakeSome, span, 0);
        } else {
            self.lift_opt(1, span)?;
        }
        Ok(())
    }

    /// **VD-258** — số lớp `Opt` của phần tử `list` khi gọi `push` (`0` nếu không phải list/Opt).
    pub(super) fn push_elem_depth(&self, base: &vis_hir::HExpr) -> usize {
        match self.base_ty(base) {
            Some(Type::List(el)) if el.is_opt() => opt_depth(&el),
            _ => 0,
        }
    }

    /// **VD-258** — nâng đối số `i` của biến thể enum tới đủ lớp `Opt` của trường khai báo.
    pub(super) fn lift_enum_arg(
        &mut self,
        enum_id: Option<vis_hir::EnumId>,
        variant: &str,
        i: usize,
        span: Span,
    ) -> Result<(), CodegenError> {
        let Some(id) = enum_id else {
            return Ok(());
        };
        if let Some(ft) = self
            .g
            .enum_fields
            .get(&(id.0, variant.to_string(), i))
            .cloned()
        {
            let depth = opt_depth(&ft);
            if depth > 0 {
                self.lift_opt(depth, span)?;
            }
        }
        Ok(())
    }

    /// **VD-258** — nâng giá trị cập nhật `with` vào trường `T?` của bản ghi đã biết.
    pub(super) fn lift_with_field(
        &mut self,
        rec_id: Option<vis_hir::RecId>,
        name: &str,
        span: Span,
    ) -> Result<(), CodegenError> {
        if let Some(id) = rec_id {
            if let Some(ft) = self.g.rec_fields.get(&(id.0, name.to_string())).cloned() {
                let depth = opt_depth(&ft);
                if depth > 0 {
                    self.lift_opt(depth, span)?;
                }
            }
        }
        Ok(())
    }

    /// **ST-248 (VD-242 §3.3)** — số lớp `Opt` của **đích gán** (0 ⇒ giữ nguyên).
    /// `Var` dùng kiểu biến (không đổi theo từng lần gán); `Field`/`Index` dùng kiểu khai báo
    /// của trường/phần tử. `lift_opt` tự dừng ở đủ lớp ⇒ không nâng nhiều lớp, `None` giữ nguyên.
    pub(super) fn assign_target_depth(&self, t: &vis_hir::HExpr) -> usize {
        let ty = match t {
            vis_hir::HExpr::Var { sym: Some(id), .. } => self.g.sym_ty.get(id).cloned(),
            other => self.base_ty(other),
        };
        ty.as_ref().map(opt_depth).unwrap_or(0)
    }

    /// **VD-241 §3+ / ST-248** — nâng giá trị ở biên gán (đích `var`, `field`, `index`) tới đủ lớp
    /// `Opt` của kiểu đích; đích không-Opt giữ nguyên.
    pub(super) fn lift_assign_set(
        &mut self,
        target: &vis_hir::HExpr,
        span: Span,
    ) -> Result<(), CodegenError> {
        let depth = self.assign_target_depth(target);
        if depth > 0 {
            self.lift_opt(depth, span)?;
        }
        Ok(())
    }

    /// **VD-269** — `recv?.m(args)`: base `Option` → `Dup`/`JumpIfNone` (đoản mạch **trước**
    /// đối số) → gọi phương thức `impl` (nếu có `sym`) hoặc phương thức lõi → lift một lớp.
    ///
    /// Ngăn xếp: `[opt]` → `Dup` `[opt,opt]` → `JumpIfNone` pop bản sao (`None` để nguyên
    /// `[opt]` làm kết quả) → `UnwrapOpt` `[v]` → đối số → gọi `[kq]` → lift `[kq']`. Đường
    /// `None` nhảy thẳng tới cuối, không đánh giá đối số; hai đường đều còn **một** giá trị.
    pub(super) fn goi_phuong_thuc_tuy_chon(
        &mut self,
        callee: &vis_hir::HExpr,
        args: &[vis_hir::HExpr],
        named: &[(String, vis_hir::HExpr)],
        span: Span,
    ) -> Result<(), CodegenError> {
        let vis_hir::HExpr::Field {
            base,
            name,
            optional: true,
            ..
        } = callee
        else {
            return Err(CodegenError::Unsupported("opt-method-callee", span));
        };
        let inner = match self.base_ty(base) {
            Some(Type::Opt(t)) => Some(*t),
            _ => None,
        };
        // **VD-270**: kiểu nhận **suy ra** có thể vẫn chưa chốt (lambda/generic/kênh) —
        // `sym_voi_types` trả `None`. Khi đó fallback theo **tên phương thức duy nhất** trên
        // mọi `impl` (interp vốn fallback theo tên `Value`; VM không có bảng tên lúc chạy nên
        // phải chốt ở codegen). Chỉ fallback khi kiểu nhận **không phải** kiểu lõi đã biết
        // (`list`/`str`/`map`/...), để không cướp `xs?.get/first/...` của phương thức lõi;
        // `{error}` cũng **không** fallback (giữ hành vi cũ, tránh gọi nhầm phương thức của
        // kiểu khác trong thân generic chưa chốt).
        let co_the_theo_ten = matches!(
            &inner,
            None | Some(Type::Record(_)) | Some(Type::Enum(_)) | Some(Type::TypeRef { .. })
        );
        let sym = inner
            .as_ref()
            .and_then(|t| self.g.mret.sym_voi_types(t, name))
            .or_else(|| {
                if co_the_theo_ten {
                    self.g.mret.sym_duy_nhat(name)
                } else {
                    None
                }
            });
        let fi = sym.and_then(|s| self.g.fmap.get(&s).copied());
        self.expr(base)?;
        self.f.push(Instr::Dup, span, 1);
        let jn = self.f.here();
        self.f.push(Instr::JumpIfNone(0), span, -1);
        self.f.push(Instr::UnwrapOpt, span, 0);
        // **VD-269**: đối số có tên của phương thức `impl` phải xếp lại theo **thứ tự tham số**
        // (mirror `HExpr::Var`); không có tên thì đẩy nguyên thứ tự viết.
        let ps = if named.is_empty() {
            None
        } else if fi.is_some() {
            self.g.fparams.get(name).cloned()
        } else {
            None
        };
        match (fi, ps) {
            (Some(fi), Some(ps)) => {
                let mut vt = 0usize;
                for (pname, _) in ps.iter().skip(1) {
                    if let Some(a) = args.get(vt) {
                        self.expr(a)?;
                        vt += 1;
                    } else if let Some((_, a)) = named.iter().find(|(n, _)| n == pname) {
                        self.expr(a)?;
                    } else {
                        self.f.push(Instr::PushUnit, span, 1);
                    }
                }
                let tong = ps.len() as u16;
                self.f
                    .push(Instr::Call(fi, tong), span, 1 - i32::from(tong));
            }
            (Some(fi), None) => {
                for a in args {
                    self.expr(a)?;
                }
                for (_, a) in named {
                    self.expr(a)?;
                }
                let n = (args.len() + named.len()) as u16;
                self.f
                    .push(Instr::Call(fi, n + 1), span, 1 - i32::from(n + 1));
            }
            (None, _) => {
                for a in args {
                    self.expr(a)?;
                }
                for (_, a) in named {
                    self.expr(a)?;
                }
                let n = (args.len() + named.len()) as u16;
                let c = self.g.cst_str(name);
                self.f.push(Instr::CallMethod(c, n), span, -i32::from(n));
            }
        }
        // **VD-269**: lift một lớp, **trừ** `()` (giữ `()` như checker) — quyết định theo *giá
        // trị* để hai engine khớp kể cả khi kiểu trả của phương thức chưa chốt (`{error}`).
        self.f.push(Instr::Dup, span, 1);
        self.f.push(Instr::IsUnit, span, 0);
        self.f.push(Instr::Un(vis_hir::UnOp::Not), span, 0);
        let jskip = self.f.here();
        self.f.push(Instr::JumpIfFalse(0), span, -1);
        self.lift_opt(1, span)?;
        let end = self.f.here();
        self.f.patch(jskip, end);
        self.f.patch(jn, end);
        self.net_stack(span)
    }
}

#[cfg(test)]
mod tests;
