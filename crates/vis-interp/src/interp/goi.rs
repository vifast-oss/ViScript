// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-63 (VD-105 (6))**: phần **gọi hàm/closure** của interp — tách khỏi `interp.rs` cho luật
//! **R1** (≤1.500 dòng/tệp). Chuyển **nguyên văn**, không đổi một dòng ngữ nghĩa nào.
//!
//! Đây là **mô-đun con** của `interp` nên vẫn thấy `Interp`/`Closure`/`HFn` + `MAX_CALL_DEPTH`
//! như trước; `interp.rs` chỉ còn phần câu lệnh/biểu thức.

use std::collections::HashMap;

use vis_hir::hir::*;
use vis_hir::ty::Type;
use vis_hir::Symbols;
use vis_hir::{MethodRet, TypeTable};
use vis_syntax::Span;

use crate::interp::{builtin_name, Abort, Args, Ctrl, Interp, Trap, MAX_CALL_DEPTH, R, RT};
use crate::value::*;

mod goi_ham;

pub(crate) use goi_ham::{loi_arity, loi_khong_phai_ham};
mod infer;
pub(crate) use infer::*;

/// **VD-263** — mã **nội bộ** đánh dấu `break`/`continue` phát ra từ biểu thức (`eval` trả
/// `R<Value>`, không mang được `Ctrl`). Không nằm trong sổ `RUNTIME_CODES`; không bao giờ tới
/// người dùng: `exec_block` mở lại thành `Ctrl`, biên hàm/tệp đổi thành trap `VS0501` thật.
/// `Abort` nằm ở `frame/core.rs` (ngoài phạm vi sửa của VD-263) nên tín hiệu đi qua
/// `Abort::Trap` với mã riêng, nhận lại bằng **danh tính con trỏ**.
static CTRL_BREAK: vis_diag::Code = vis_diag::Code {
    id: 9001,
    short: "VS9001-CTRL",
    message_vi: "tín hiệu nội bộ `break` (VD-263)",
    message_en: "internal `break` signal (VD-263)",
    message_ja: "内部 `break` シグナル（VD-263）",
    severity: vis_diag::Severity::Error,
};
/// **VD-263** — như `CTRL_BREAK`, cho `continue`.
static CTRL_CONTINUE: vis_diag::Code = vis_diag::Code {
    id: 9002,
    short: "VS9002-CTRL",
    message_vi: "tín hiệu nội bộ `continue` (VD-263)",
    message_en: "internal `continue` signal (VD-263)",
    message_ja: "内部 `continue` シグナル（VD-263）",
    severity: vis_diag::Severity::Error,
};

/// `true` khi `let x: T? = …` (chú thích `Optional` — mọi lớp). Chuyển từ `interp.rs` để R1.
pub(crate) fn ty_là_opt(t: Option<&vis_hir::ty::Type>) -> bool {
    matches!(t, Some(vis_hir::ty::Type::Opt(_)))
}

impl<'a> Interp<'a> {
    /// **VD-258** — thân `for`: `E_LOOP` **mỗi lần thử tiến vòng, gồm lần cuối sai**
    /// (VD-252 quy tắc 4). Charge ở đầu vòng, trước khi lấy phần tử; `break` không chạy lượt kế.
    ///
    /// **VD-264**: nhận [`ForIter`] (range duyệt lười) thay cho `Vec<Value>`.
    pub(crate) fn exec_for(
        &mut self,
        pat: &HPat,
        mut items: ForIter,
        body: &[HStmt],
        span: Span,
    ) -> R<Ctrl> {
        let mut ctrl = Ok(Ctrl::Normal);
        loop {
            if let Some(a) = self.charge(crate::rt::fee::FeeKind::Loop, span) {
                ctrl = Err(a);
                break;
            }
            let Some(v) = items.next() else { break };
            self.bind_pat(pat, &v);
            match self.exec_block(body) {
                Ok(Ctrl::Break) => break,
                Ok(Ctrl::Normal) | Ok(Ctrl::Continue) => {}
                Err(e) => {
                    ctrl = Err(e);
                    break;
                }
            }
        }
        ctrl
    }

    /// Chạy thân lấy **giá trị** + luồng điều khiển còn treo (`break`/`continue` từ biểu thức).
    fn exec_body_ctrl(&mut self, body: &[HStmt]) -> R<(Value, Ctrl)> {
        let c = self.exec_block(body)?;
        match c {
            Ctrl::Normal => Ok((self.last.take().unwrap_or(Value::Unit), Ctrl::Normal)),
            // Không có giá trị: `break`/`continue` thoát khỏi thân trước khi biểu thức cuối chạy.
            c => Ok((Value::Unit, c)),
        }
    }

    /// Chạy thân **hàm/closure** lấy giá trị. `break`/`continue` không xuyên biên hàm
    /// (VD-259): nếu còn treo khi thân kết thúc thì đó là chương trình sai — trap như cấp tệp,
    /// không nuốt thành `()` (VD-263).
    fn exec_body(&mut self, body: &[HStmt]) -> R<Value> {
        let (v, c) = self.exec_body_ctrl(body)?;
        match c {
            Ctrl::Normal => Ok(v),
            _ => Err(Self::trap_nhay(
                body.last().map(HStmt::span).unwrap_or_default(),
            )),
        }
    }

    /// **VD-263** — trap thật cho `break`/`continue` không còn vòng lặp nào trong cùng hàm.
    fn trap_nhay(span: Span) -> Abort {
        Abort::Trap(Trap::new(RT, "`break`/`continue` ngoài vòng lặp", span))
    }

    /// **VD-263** — đóng gói `Ctrl` thành tín hiệu nội bộ để `eval` (vốn trả `R<Value>`) mang
    /// được `break`/`continue` ra khỏi biểu thức. Mã `CTRL_BREAK`/`CTRL_CONTINUE` chỉ để
    /// `tach_ctrl` nhận lại bằng **danh tính con trỏ** — không hiển thị cho người dùng.
    fn ctrl_abort(c: Ctrl) -> Abort {
        let code = match c {
            Ctrl::Break => &CTRL_BREAK,
            Ctrl::Continue => &CTRL_CONTINUE,
            Ctrl::Normal => unreachable!("`Ctrl::Normal` không phải tín hiệu dừng"),
        };
        Abort::Trap(Trap::new(
            code,
            "tín hiệu điều khiển nội bộ (VD-263)",
            Span::default(),
        ))
    }

    /// **VD-263** — nhận diện tín hiệu nội bộ do [`Self::ctrl_abort`] phát; trả nguyên `Abort`
    /// khác (trap thật, `return`, pending).
    pub(crate) fn tach_ctrl(a: Abort) -> Result<Ctrl, Abort> {
        match a {
            Abort::Trap(t) if std::ptr::eq(t.code, &CTRL_BREAK) => Ok(Ctrl::Break),
            Abort::Trap(t) if std::ptr::eq(t.code, &CTRL_CONTINUE) => Ok(Ctrl::Continue),
            other => Err(other),
        }
    }

    /// **VD-258** — đánh giá `if` biểu thức, nâng nhánh đã chọn tới `Opt` chung nếu các nhánh
    /// lệch lớp (`if c: 1 else: None` ⇒ `Some(1)`/`None`, không để `1` plain trong biến `int?`).
    pub(crate) fn eval_if(
        &mut self,
        cond: &HCond,
        then: &[HStmt],
        elifs: &[(HCond, Vec<HStmt>)],
        else_: &[HStmt],
    ) -> R<Value> {
        let (min, max) = do_if(
            then,
            elifs,
            Some(else_),
            &self.sym_ty,
            self.types,
            &self.fn_rets,
            &self.mret,
        );
        let v = if self.eval_cond(cond)? {
            let (v, ctrl) = self.exec_body_ctrl(then)?;
            if ctrl != Ctrl::Normal {
                return Err(Self::ctrl_abort(ctrl));
            }
            v
        } else {
            let mut chon: Option<Value> = None;
            for (c, body) in elifs {
                if self.eval_cond(c)? {
                    let (v, ctrl) = self.exec_body_ctrl(body)?;
                    if ctrl != Ctrl::Normal {
                        return Err(Self::ctrl_abort(ctrl));
                    }
                    chon = Some(v);
                    break;
                }
            }
            match chon {
                Some(v) => v,
                None => {
                    let (v, ctrl) = self.exec_body_ctrl(else_)?;
                    if ctrl != Ctrl::Normal {
                        return Err(Self::ctrl_abort(ctrl));
                    }
                    v
                }
            }
        };
        Ok(boc_nhanh(v, min, max))
    }

    /// **VD-258** — đánh giá `match` biểu thức, nâng nhánh khớp tới `Opt` chung.
    ///
    /// **VD-263** — nhánh chạy có thể phát `break`/`continue` (nằm trong biểu thức con); chuyển
    /// tiếp dưới dạng tín hiệu nội bộ do `Interp::ctrl_abort` phát để `exec_block` trả lại cho vòng lặp
    /// gần nhất.
    pub(crate) fn eval_match(&mut self, subject: &HExpr, arms: &[HArm], span: Span) -> R<Value> {
        let (min, max) = do_match(arms, &self.sym_ty, self.types, &self.fn_rets, &self.mret);
        let subj = self.eval(subject)?;
        match self.pick_arm(arms, &subj)? {
            Some((arm, binds)) => {
                for (sym, v) in binds {
                    self.env.borrow_mut().insert(sym, v);
                }
                let (v, ctrl) = self.exec_body_ctrl(&arm.body)?;
                if ctrl != Ctrl::Normal {
                    return Err(Self::ctrl_abort(ctrl));
                }
                Ok(boc_nhanh(v, min, max))
            }
            None => Err(Abort::Trap(Trap::new(
                RT,
                "`match` không khớp nhánh nào (không xét hết bị lọt)",
                span,
            ))),
        }
    }

    /// **ST-248** — kiểu tĩnh của gốc/target; `None` ⇒ không nâng. **VD-264**: dùng chung
    /// [`infer_ty`] (Var/Field/Index/Call/Try/With/If/Match/tuple) rồi mới fallback theo
    /// **tên bản ghi runtime** cho biến không annot; trước đây chỉ Var/Field/Index nên
    /// `make().push(x)`/`lay()?.b` mù kiểu. Ở `goi.rs` để `interp.rs` giữ R1 ≤ 1.500 dòng.
    pub(crate) fn expr_ty(&self, e: &HExpr, root: &Value) -> Option<Type> {
        if let Some(t) = infer_ty(e, &self.sym_ty, self.types, &self.fn_rets, &self.mret) {
            return Some(t);
        }
        match e {
            HExpr::Var { sym: Some(_), .. } => match root {
                Value::Record(r) => self.types.rec_by_name(&r.name).map(Type::Record),
                _ => None,
            },
            HExpr::Field { base, name, .. } => {
                let bt = self.expr_ty(base, root)?;
                let inner = match bt {
                    Type::Opt(t) => *t,
                    other => other,
                };
                match inner {
                    Type::Record(id) => self.types.field(id, name).map(|f| f.ty.clone()),
                    _ => None,
                }
            }
            HExpr::Index { base, .. } => match self.expr_ty(base, root) {
                Some(Type::Map(_, v)) => Some(*v),
                Some(Type::List(el)) => Some(*el),
                Some(Type::Bytes) => Some(Type::Int),
                _ => None,
            },
            _ => None,
        }
    }

    /// **VD-258** — field của `?.` có kiểu `T?` không (theo kiểu tĩnh của base, hoặc tên bản
    /// ghi runtime khi base không annot). Quyết định `Some(None)` (field optional thật) khác
    /// `None` (field thường nhưng giá trị bất thường — giữ tương thích `lift_opt`).
    pub(crate) fn field_dich_la_opt(&self, base: &HExpr, name: &str, raw: &Value) -> bool {
        let root = match raw {
            Value::Enum(e)
                if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 =>
            {
                e.args.get(0).expect("Some luôn có một đối số")
            }
            other => other,
        };
        let Some(bt) = self.expr_ty(base, root) else {
            return false;
        };
        let inner = match bt {
            Type::Opt(t) => *t,
            other => other,
        };
        let Type::Record(id) = inner else {
            return false;
        };
        self.types
            .field(id, name)
            .map(|f| f.ty.is_opt())
            .unwrap_or(false)
    }

    /// **VD-258** — đánh giá `base.name` / `base?.name`: field optional thật luôn bọc thêm một
    /// `Some` (`f: int?` + `None` ⇒ `Some(None)`); field thường dùng `lift_opt(1)` như cũ.
    pub(crate) fn eval_field(
        &mut self,
        base: &HExpr,
        name: &str,
        optional: bool,
        span: Span,
    ) -> R<Value> {
        let raw = self.eval(base)?;
        let field_la_opt = self.field_dich_la_opt(base, name, &raw);
        let (b, nang) = if optional {
            match raw {
                Value::Enum(e) if e.enum_name == "Option" && e.variant == "None" => {
                    return Ok(Value::none());
                }
                Value::Enum(e)
                    if e.enum_name == "Option" && e.variant == "Some" && e.args.len() == 1 =>
                {
                    (
                        e.args.get(0).expect("Some luôn có một đối số").clone(),
                        true,
                    )
                }
                other => (other, false),
            }
        } else {
            (raw, false)
        };
        let v = self.field_of(b, name, span)?;
        Ok(if nang {
            if field_la_opt {
                Value::some(v)
            } else {
                crate::lift::lift_opt(v, 1)
            }
        } else {
            v
        })
    }

    /// **VD-258** — nâng giá trị vào trường `T?` của bản ghi theo `RecId` đã biết.
    pub(crate) fn lift_truong_ban_ghi(
        &self,
        rec_id: Option<vis_hir::ty::RecId>,
        name: &str,
        v: Value,
    ) -> Value {
        if let Some(id) = rec_id {
            if let Some(f) = self.types.field(id, name) {
                if f.ty.is_opt() {
                    return crate::lift::lift_opt(v, opt_depth_ty(&f.ty));
                }
            }
        }
        v
    }

    /// **VD-258** — nâng đối số vào trường `T?` của biến thể enum.
    pub(crate) fn lift_doi_so_bien_the(
        &self,
        enum_id: Option<vis_hir::ty::EnumId>,
        variant: &str,
        i: usize,
        v: Value,
    ) -> Value {
        let ft = enum_id
            .and_then(|id| self.types.variant(id, variant))
            .and_then(|vd| vd.fields.get(i))
            .map(|f| f.ty.clone());
        match ft {
            Some(t) if t.is_opt() => crate::lift::lift_opt(v, opt_depth_ty(&t)),
            _ => v,
        }
    }

    /// **VD-263** — `RecId` **tĩnh** của biểu thức gốc (`Box[int?]` từ annot), để `with` nâng
    /// trường theo instantiation thay vì bản khai báo đã xoá `T`.
    pub(crate) fn rec_id_tinh(&self, e: &HExpr, root: &Value) -> Option<vis_hir::ty::RecId> {
        match self.expr_ty(e, root) {
            Some(vis_hir::Type::Record(id)) => Some(id),
            _ => None,
        }
    }
}

/// **VD-241 §3 (A0, 2026-11-19)** — tự nâng `T → T? → T??…` tại các biên hàm/annot (VD-227/228).
/// Đích `Opt(_)` bọc giá trị tới **ít nhất** đủ lớp của annot. Lõi được giữ đúng (giá trị đã là
/// `Opt`/`Result` thuộc đúng lớp đó không bị bọc thêm). (Checker từ chối từ nguồn lúc < `annot−1`
/// lớp — nên vòng lặp chỉ là kỹ thuật, không còn lớp > 1/chuyển đổi cho các chương trình hợp lệ.)
pub(crate) fn opt_lift_maybe(v: Value, depth_target: usize) -> Value {
    crate::lift::lift_opt(v, depth_target)
}

/// Số lớp `Opt` của một **đích kiểu** (đếm từ ngoài vào).
pub(crate) fn opt_depth_ty(t: &vis_hir::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_hir::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

/// **VD-260** — `sym` hàm/phương thức → kiểu trả về (đủ để suy lớp Option của lời gọi).
pub(crate) fn collect_fn_rets(fns: &HashMap<u32, &HFn>) -> HashMap<u32, Type> {
    fns.iter().map(|(sym, f)| (*sym, f.ret.clone())).collect()
}

fn bind_cond_let(
    c: &HCond,
    sym: &mut HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) {
    if let HCond::Let { pat, value, .. } = c {
        if let Some(t) = infer_ty(value, sym, types, fn_rets, mret) {
            bind_pat_ty(pat, &t, sym, types);
        }
    }
}

/// **VD-260** — đi khối câu lệnh để gom kiểu biến (mirror codegen `than::walk_stmts`).
///
/// **VD-264**: dùng `bind_pat_ty` đệ quy cho `let`/`for`/**arm `match`**/`if let`/`while let`
/// (Tuple/List/Record/Ctor/Or) — trước đây chỉ `Bind` trần nên biến bind trong pattern mất kiểu.
fn walk_stmts(
    sts: &[HStmt],
    sym: &mut HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) {
    for s in sts {
        match s {
            HStmt::Var {
                sym: id, ty, init, ..
            } => {
                let t = ty
                    .clone()
                    .filter(|t| !la_error(t))
                    .or_else(|| infer_ty(init, sym, types, fn_rets, mret));
                if let Some(t) = t {
                    sym.insert(*id, t);
                }
            }
            HStmt::Let { pat, ty, init, .. } => {
                let t = ty
                    .clone()
                    .filter(|t| !la_error(t))
                    .or_else(|| infer_ty(init, sym, types, fn_rets, mret));
                if let Some(t) = t {
                    bind_pat_ty(pat, &t, sym, types);
                }
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                bind_cond_let(cond, sym, types, fn_rets, mret);
                walk_stmts(then, sym, types, fn_rets, mret);
                for (c, b) in elifs {
                    bind_cond_let(c, sym, types, fn_rets, mret);
                    walk_stmts(b, sym, types, fn_rets, mret);
                }
                if let Some(b) = else_ {
                    walk_stmts(b, sym, types, fn_rets, mret);
                }
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                // **VD-264**: kiểu pattern = **phần tử** của iter (list/map/str/range), như
                // checker `elem_of_iter` — trước đây truyền cả `list[T]` nên pattern tuple mất kiểu.
                if let Some(t) =
                    infer_ty(iter, sym, types, fn_rets, mret).and_then(|t| elem_of_iter_ty(&t))
                {
                    bind_pat_ty(pat, &t, sym, types);
                }
                walk_stmts(body, sym, types, fn_rets, mret);
            }
            HStmt::While { cond, body, .. } => {
                bind_cond_let(cond, sym, types, fn_rets, mret);
                walk_stmts(body, sym, types, fn_rets, mret);
            }
            HStmt::Loop { body, .. } | HStmt::Scope { body, .. } | HStmt::Block { body, .. } => {
                walk_stmts(body, sym, types, fn_rets, mret)
            }
            HStmt::Match { subject, arms, .. } => {
                // **VD-264**: biến bind trong arm lấy kiểu từ chủ thể (kể cả chủ thể là lời gọi).
                let subj = infer_ty(subject, sym, types, fn_rets, mret);
                for a in arms {
                    if let Some(t) = &subj {
                        bind_pat_ty(&a.pat, t, sym, types);
                    }
                    walk_stmts(&a.body, sym, types, fn_rets, mret);
                }
            }
            _ => {}
        }
    }
}

/// **VD-260** — kiểu đầy đủ của biến = annotation + suy từ initializer (mirror codegen);
/// `self` trong method `impl` lấy kiểu bản ghi của `impl` kể cả khi không annot.
pub(crate) fn collect_sym_tys(
    hir: &Hir,
    symbols: &Symbols,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> HashMap<u32, Type> {
    let mut m: HashMap<u32, Type> = symbols
        .syms
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.ty.clone().filter(|t| !la_error(t)).map(|t| (i as u32, t)))
        .collect();
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
    walk_stmts(&hir.stmts, &mut m, types, fn_rets, mret);
    for it in &hir.items {
        match it {
            HItem::Fn(f) => walk_stmts(&f.body, &mut m, types, fn_rets, mret),
            HItem::Impl(i) => {
                for meth in &i.methods {
                    walk_stmts(&meth.body, &mut m, types, fn_rets, mret);
                }
            }
            _ => {}
        }
    }
    m
}

/// **VD-258** — số lớp `Option` của biểu thức nếu biết **theo cú pháp** (`None`, `Some(…)`,
/// `if`/`match` lồng); dùng để nâng nhánh `if`/`match` lệch lớp. Không đoán kiểu biến.
fn do_bieu_thuc(
    e: &HExpr,
    sym_ty: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> usize {
    match e {
        HExpr::Opt { ty, .. } => opt_depth_ty(ty),
        HExpr::Var { name, .. } if name == "None" => 1,
        HExpr::Var { sym: Some(id), .. } => sym_ty.get(id).map(opt_depth_ty).unwrap_or(0),
        HExpr::Field {
            base,
            name,
            optional: true,
            ..
        } => {
            // **VD-264**: suy kiểu đầy đủ trước (base có thể là lời gọi/`?`/biến thể);
            // fallback cũ chỉ nhìn `Var` nên `a?.f` với `a` bind từ pattern/call bị đoán 1 lớp.
            if let Some(t) = infer_ty(e, sym_ty, types, fn_rets, mret) {
                return opt_depth_ty(&t);
            }
            if let HExpr::Var { sym: Some(id), .. } = &**base {
                if let Some(Type::Opt(inner)) = sym_ty.get(id) {
                    if let Type::Record(rid) = &**inner {
                        if let Some(f) = types.field(*rid, name) {
                            return 1 + opt_depth_ty(&f.ty);
                        }
                    }
                }
            }
            1
        }
        // **VD-264**: `xs[i]?` ⇒ `list[T]` cho `T?`; giữ đúng lớp khi `T` đã là Option.
        HExpr::Try { expr, .. } if matches!(&**expr, HExpr::Index { .. }) => {
            infer_ty(e, sym_ty, types, fn_rets, mret)
                .map(|t| opt_depth_ty(&t))
                .unwrap_or(1)
        }
        HExpr::Call { callee, args, .. } => match &**callee {
            HExpr::Var { name, .. } if name == "Some" => {
                1 + args
                    .first()
                    .map(|a| do_bieu_thuc(a, sym_ty, types, fn_rets, mret))
                    .unwrap_or(0)
            }
            HExpr::Var { name, .. } if name == "None" => 1,
            // **VD-260**: lời gọi hàm tệp/phương thức — kiểu trả về quyết định lớp Option.
            HExpr::Var { sym: Some(id), .. } => fn_rets.get(id).map(opt_depth_ty).unwrap_or(0),
            // Method dựng sẵn (`list.get/first/…`, `map.get`) — dùng chung `infer_ty`.
            _ => infer_ty(e, sym_ty, types, fn_rets, mret)
                .map(|t| opt_depth_ty(&t))
                .unwrap_or(0),
        },
        HExpr::If {
            then, elifs, else_, ..
        } => do_if(then, elifs, Some(else_), sym_ty, types, fn_rets, mret).1,
        HExpr::Match { arms, .. } => do_match(arms, sym_ty, types, fn_rets, mret).1,
        // **VD-260**: các dạng còn lại (method `get`/`first`/…, index, `with`, tuple) — dùng
        // chung suy kiểu đích để nhánh plain được nâng đúng lớp.
        _ => infer_ty(e, sym_ty, types, fn_rets, mret)
            .map(|t| opt_depth_ty(&t))
            .unwrap_or(0),
    }
}

/// **VD-258** — số lớp `Option` của giá trị cuối một khối; `None` = khối không cho giá trị
/// (kết thúc `return`/`break`/`continue`).
fn do_than(
    stmts: &[HStmt],
    sym_ty: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<usize> {
    match stmts.last()? {
        HStmt::Expr(e) => Some(do_bieu_thuc(e, sym_ty, types, fn_rets, mret)),
        HStmt::If {
            then, elifs, else_, ..
        } => Some(do_if(then, elifs, else_.as_deref(), sym_ty, types, fn_rets, mret).1),
        HStmt::Match { arms, .. } => Some(do_match(arms, sym_ty, types, fn_rets, mret).1),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            do_than(body, sym_ty, types, fn_rets, mret)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => Some(0),
    }
}

/// **VD-258** — `(min, max)` lớp `Option` của các nhánh `if`; không `else` ⇒ `(0, 0)`.
pub(crate) fn do_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym_ty: &HashMap<u32, Type>,
    types: &TypeTable,
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
        if let Some(d) = do_than(body, sym_ty, types, fn_rets, mret) {
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
pub(crate) fn do_match(
    arms: &[HArm],
    sym_ty: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> (usize, usize) {
    let ds: Vec<usize> = arms
        .iter()
        .filter_map(|a| do_than(&a.body, sym_ty, types, fn_rets, mret))
        .collect();
    if ds.is_empty() {
        return (0, 0);
    }
    let min = *ds.iter().min().unwrap_or(&0);
    let max = *ds.iter().max().unwrap_or(&0);
    (min, max)
}

/// **VD-258** — nâng giá trị nhánh đã chọn tới `max` khi các nhánh lệch lớp.
pub(crate) fn boc_nhanh(v: Value, min: usize, max: usize) -> Value {
    if max > min {
        crate::lift::lift_opt(v, max)
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ha(src: &str) -> vis_types::HirOutput {
        let src = format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}");
        let lex = vis_syntax::lex(&src, vis_syntax::Mode::Script);
        let out = vis_syntax::parse(&src, vis_syntax::Mode::Script);
        vis_types::lower(&out.cst, &lex)
    }

    /// Đổi `Call(Var("R.io"), [r])` (dispatch tĩnh) thành `Call(Field(r, "io"), [])` — dạng HIR mà
    /// **VD-265** phải suy được khi lowering không dispatch (callee `Field`).
    fn doi_thanh_field(e: &mut HExpr) -> bool {
        if let HExpr::Call {
            callee,
            args,
            named,
            span,
        } = e
        {
            if let HExpr::Var { name, .. } = &**callee {
                if name == "R.io" && !args.is_empty() {
                    let base = args.remove(0);
                    **callee = HExpr::Field {
                        base: Box::new(base),
                        name: "io".into(),
                        optional: false,
                        span: *span,
                    };
                    named.clear();
                    return true;
                }
            }
            for a in args.iter_mut() {
                if doi_thanh_field(a) {
                    return true;
                }
            }
            return doi_thanh_field(callee);
        }
        false
    }

    fn doi(sts: &mut [HStmt]) -> bool {
        for s in sts {
            if let HStmt::Let { init, .. } | HStmt::Var { init, .. } = s {
                if doi_thanh_field(init) {
                    return true;
                }
            }
        }
        false
    }

    /// **VD-265 (đỏ-trước ở tầng này)** — trước bản sửa, `infer_ty` của `Call(Field(Record, m))`
    /// chỉ biết method dựng sẵn (`list`/`map`) nên trả `None`; nay tra [`MethodRet`] và biết `int?`.
    #[test]
    fn infer_ty_phuong_thuc_nguoi_dung_callee_field() {
        let h = ha("type R:\n    f: int?\n\nimpl R:\n    fn io(self) -> int?:\n        None\n\nfn main():\n    let r: R = R(f: None)\n    let x = r.io()\n");
        let mut h = h;
        let mut found = false;
        for it in h.hir.items.iter_mut() {
            if let HItem::Fn(f) = it {
                if f.name == "main" {
                    found = doi(&mut f.body);
                }
            }
        }
        assert!(found, "phải đổi được lời gọi `r.io()` thành callee Field");
        let mret = vis_hir::MethodRet::from_hir_voi_bang(&h.hir, &h.types);
        let fn_rets = HashMap::new(); // cố ý rỗng: chỉ bảng phương thức quyết định
        let sym = collect_sym_tys(&h.hir, &h.symbols, &h.types, &fn_rets, &mret);
        let got = h
            .hir
            .items
            .iter()
            .find_map(|it| match it {
                HItem::Fn(f) if f.name == "main" => f.body.iter().find_map(|s| match s {
                    HStmt::Let { init, .. } if matches!(init, HExpr::Call { .. }) => {
                        infer_ty(init, &sym, &h.types, &fn_rets, &mret)
                    }
                    _ => None,
                }),
                _ => None,
            })
            .expect("phải suy được kiểu lời gọi phương thức");
        assert_eq!(got, Type::Opt(Box::new(Type::Int)));
    }
}
