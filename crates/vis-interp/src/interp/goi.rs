// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-63 (VD-105 (6))**: phần **gọi hàm/closure** của interp — tách khỏi `interp.rs` cho luật
//! **R1** (≤1.500 dòng/tệp). Chuyển **nguyên văn**, không đổi một dòng ngữ nghĩa nào.
//!
//! Đây là **mô-đun con** của `interp` nên vẫn thấy `Interp`/`Closure`/`HFn` + `MAX_CALL_DEPTH`
//! như trước; `interp.rs` chỉ còn phần câu lệnh/biểu thức.

use std::collections::HashMap;

use vis_syntax::Span;
use vis_types::{hir::*, lower::Symbols, ty::Type, MethodRet, TypeTable};

use crate::interp::{builtin_name, Abort, Args, Ctrl, Interp, Trap, MAX_CALL_DEPTH, R, RT};
use crate::value::*;

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
pub(crate) fn ty_là_opt(t: Option<&vis_types::ty::Type>) -> bool {
    matches!(t, Some(vis_types::ty::Type::Opt(_)))
}

impl<'a> Interp<'a> {
    // ---------------------------------------------------------------------
    // Gọi hàm
    // ---------------------------------------------------------------------

    pub(crate) fn eval_call(
        &mut self,
        callee: &HExpr,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: Span,
    ) -> R<Value> {
        // 1) Phương thức trên list/str/map: `xs.push(1)`, `s.upper()`.
        if let HExpr::Field {
            base,
            name,
            optional: false,
            ..
        } = callee
        {
            let recv = self.eval(base)?;
            // **ST-61 (VD-96)**: kênh cũng đi **một** nhánh `builtin::method` (như list/str/map) ⇒
            // interp ↔ VM cùng một hiện thực `send`/`recv`/`close`.
            if matches!(
                recv,
                Value::List(_) | Value::Str(_) | Value::Map(_) | Value::Chan(_)
            ) {
                // **VD-258**: `xs.push(v)` với `xs: list[T?]` — nâng `v` plain ở biên phương
                // thức (mirror codegen), tránh list chứa giá trị plain trong ô `T?`.
                let push_depth = if name == "push" {
                    match self.expr_ty(base, &recv) {
                        Some(vis_types::Type::List(el)) if el.is_opt() => opt_depth_ty(&el),
                        _ => 0,
                    }
                } else {
                    0
                };
                let mut vs = Vec::with_capacity(args.len());
                for a in args {
                    let mut vv = self.eval(a)?;
                    if push_depth > 0 {
                        vv = crate::lift::lift_opt(vv, push_depth);
                    }
                    vs.push(vv);
                }
                let kq = crate::builtin::method(self, recv, name, vs, span)?;
                // ST-15b (mục 1, §7.1 ngữ nghĩa giá trị + CoW): `push` trên **`var`** phải **ghi lại**
                // biến (bản thân list đổi), không chỉ trả list mới. `push` trên `let`/biểu thức tạm
                // vẫn "trả list mới" (không có lvalue để ghi) — ghi rõ trong JOURNAL.
                if name == "push" {
                    if let HExpr::Var { sym: Some(id), .. } = &**base {
                        if self.mutable.contains(id) {
                            self.env.borrow_mut().insert(*id, kq.clone());
                        }
                    }
                }
                return Ok(kq);
            }
        }
        // 2) Gọi thẳng hàm khai trong tệp (kể cả đệ quy) — theo `sym`.
        if let HExpr::Var { sym: Some(id), .. } = callee {
            if let Some(f) = self.fns.get(id).copied() {
                let (pos, nm) = self.eval_args(args, named)?;
                // **ST-60 (VD-90)**: gọi `async fn` **không** chạy thân — trả `Task[T]` (cùng một
                // đường với codegen: `$tac_vu` + handle, ở đây handle = `sym`).
                if f.is_async {
                    let t = self.sched.get_mut().tao(f.sym, pos, f.name.clone(), span);
                    return Ok(Value::Task(t));
                }
                return self.call_hfn(f, pos, nm, span);
            }
        }
        // 3) Hàm dựng sẵn.
        if let HExpr::Var {
            name, sym: None, ..
        } = callee
        {
            if builtin_name(name).is_some() {
                let (pos, _nm) = self.eval_args(args, named)?;
                return crate::builtin::call(self, name, pos, span);
            }
        }
        // 4) Còn lại: giá trị hàm (closure/lambda/builtin value).
        let f = self.eval(callee)?;
        let (pos, nm) = self.eval_args(args, named)?;
        self.call_value(f, pos, nm, span)
    }

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

    fn eval_args(&mut self, args: &[HExpr], named: &[(String, HExpr)]) -> R<Args> {
        let mut pos = Vec::with_capacity(args.len());
        for a in args {
            pos.push(self.eval(a)?);
        }
        let mut nm = Vec::with_capacity(named.len());
        for (n, a) in named {
            nm.push((n.clone(), self.eval(a)?));
        }
        Ok((pos, nm))
    }

    pub(crate) fn call_value(
        &mut self,
        f: Value,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        match f {
            Value::Fn(c) => self.call_closure(&c, pos, span),
            Value::Builtin(name) => crate::builtin::call(self, name, pos, span),
            Value::Record(name, fields) if fields.is_empty() => {
                // `User(name: "An", age: 20)` — khởi tạo bản ghi theo tên kiểu.
                let _ = pos;
                Ok(Value::Record(name, named))
            }
            other => {
                let _ = named;
                Err(Abort::Trap(Trap::new(
                    RT,
                    format!("`{}` không phải hàm", other.show()),
                    span,
                )))
            }
        }
    }

    pub(crate) fn call_closure(&mut self, c: &Closure, pos: Vec<Value>, span: Span) -> R<Value> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "gọi hàm quá sâu (>{} khung) — đệ quy không có điểm dừng?",
                    MAX_CALL_DEPTH
                ),
                span,
            )));
        }
        self.fn_depth += 1;
        let kq = self.call_closure_trong(c, pos, span);
        self.fn_depth -= 1;
        self.depth -= 1;
        kq
    }

    fn call_closure_trong(&mut self, c: &Closure, pos: Vec<Value>, span: Span) -> R<Value> {
        if pos.len() != c.params.len() {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "hàm `{}` cần {} đối số, được {}",
                    if c.name.is_empty() { "<fn>" } else { &c.name },
                    c.params.len(),
                    pos.len()
                ),
                span,
            )));
        }
        let saved_env = self.env.clone();
        self.env = c.env.clone();
        let mut old = Vec::with_capacity(c.params.len());
        for (i, p) in c.params.iter().enumerate() {
            old.push((*p, self.env.borrow().get(p).cloned()));
            self.env.borrow_mut().insert(*p, pos[i].clone());
        }
        let r = match &c.body {
            HBody::Expr(e) => self.eval(e),
            HBody::Stmts(st) => self.exec_body(st),
        };
        for (p, v) in old {
            match v {
                Some(v) => {
                    self.env.borrow_mut().insert(p, v);
                }
                None => {
                    self.env.borrow_mut().remove(&p);
                }
            }
        }
        self.env = saved_env;
        match r {
            Ok(v) => {
                if c.ret_depth > 0 {
                    Ok(opt_lift_maybe(v, c.ret_depth))
                } else {
                    Ok(v)
                }
            }
            Err(Abort::Ret(v)) => {
                if c.ret_depth > 0 {
                    Ok(opt_lift_maybe(v, c.ret_depth))
                } else {
                    Ok(v)
                }
            }
            // **VD-263**: thân closure dạng **biểu thức** (không phải suite) cũng không cho
            // `break`/`continue` xuyên biên hàm — trap thật thay vì để vòng lặp của người gọi
            // bắt nhầm (VD-259).
            Err(t) => match Self::tach_ctrl(t) {
                Ok(_) => Err(Self::trap_nhay(span)),
                Err(t) => Err(t),
            },
        }
    }

    pub(crate) fn call_hfn(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        self.depth += 1;
        if self.depth > MAX_CALL_DEPTH {
            self.depth -= 1;
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "gọi hàm quá sâu (>{} khung) — đệ quy không có điểm dừng?",
                    MAX_CALL_DEPTH
                ),
                span,
            )));
        }
        self.fn_depth += 1;
        let kq = self.call_hfn_trong(f, pos, named, span);
        self.fn_depth -= 1;
        self.depth -= 1;
        kq
    }

    fn call_hfn_trong(
        &mut self,
        f: &'a HFn,
        pos: Vec<Value>,
        named: Vec<(String, Value)>,
        span: Span,
    ) -> R<Value> {
        // ST-27: hàm có `...rest` nhận **thêm** đối số vị trí.
        let bien_di = f.params.last().map(|p| p.variadic).unwrap_or(false);
        if !bien_di && pos.len() + named.len() > f.params.len() {
            return Err(Abort::Trap(Trap::new(
                RT,
                format!(
                    "hàm `{}` nhận {} đối số, được {}",
                    f.name,
                    f.params.len(),
                    pos.len() + named.len()
                ),
                span,
            )));
        }
        let mut old = Vec::with_capacity(f.params.len());
        for (i, p) in f.params.iter().enumerate() {
            old.push((p.sym, self.env.borrow().get(&p.sym).cloned()));
            // ST-27: `...rest` gom **mọi** đối số vị trí còn lại thành `list[str]`.
            if p.variadic {
                let con: Vec<Value> = pos.iter().skip(i).cloned().collect();
                self.env.borrow_mut().insert(p.sym, Value::List(con));
                continue;
            }
            let v = if let Some(v) = pos.get(i) {
                Some(v.clone())
            } else {
                named
                    .iter()
                    .find(|(n, _)| *n == p.name)
                    .map(|(_, v)| v.clone())
            };
            let v = match v {
                Some(v) => v,
                None => match &p.default {
                    Some(d) => self.eval(d)?,
                    None => {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            format!("thiếu đối số `{}` của `{}`", p.name, f.name),
                            span,
                        )))
                    }
                },
            };
            // **VD-228b**: tham số annot `T?` — nâng giá trị plain ở biên hàm (giống `let` + `-> T?`).
            let v = match &p.ty {
                Type::Opt(_) => opt_lift_maybe(v, opt_depth_ty(&p.ty)),
                _ => v,
            };
            self.env.borrow_mut().insert(p.sym, v);
            if p.is_var_self {
                self.mutable.insert(p.sym);
            }
        }
        let r = self.exec_body(&f.body);
        for (p, v) in old {
            match v {
                Some(v) => {
                    self.env.borrow_mut().insert(p, v);
                }
                None => {
                    self.env.borrow_mut().remove(&p);
                }
            }
        }
        match r {
            Ok(v) => match &f.ret {
                Type::Opt(_) => Ok(opt_lift_maybe(v, opt_depth_ty(&f.ret))),
                _ => Ok(v),
            },
            Err(Abort::Ret(v)) => match &f.ret {
                Type::Opt(_) => Ok(opt_lift_maybe(v, opt_depth_ty(&f.ret))),
                _ => Ok(v),
            },
            Err(t) => Err(t),
        }
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
                Value::Record(ten, _) => self.types.rec_by_name(ten).map(Type::Record),
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
    fn field_dich_la_opt(&self, base: &HExpr, name: &str, raw: &Value) -> bool {
        let root = match raw {
            Value::Enum(ea, va, ag) if ea == "Option" && va == "Some" && ag.len() == 1 => &ag[0],
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
                Value::Enum(ea, va, mut ag) if ea == "Option" => match va.as_str() {
                    "None" => return Ok(Value::none()),
                    "Some" if ag.len() == 1 => (ag.pop().expect("Some luôn có một đối số"), true),
                    _ => (Value::Enum(ea, va, ag), false),
                },
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
        rec_id: Option<vis_types::ty::RecId>,
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
        enum_id: Option<vis_types::ty::EnumId>,
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
    pub(crate) fn rec_id_tinh(&self, e: &HExpr, root: &Value) -> Option<vis_types::ty::RecId> {
        match self.expr_ty(e, root) {
            Some(vis_types::Type::Record(id)) => Some(id),
            _ => None,
        }
    }
}

/// **VD-241 §3 (A0, 2026-11-19)** — tự nâng `T → T? → T??…` tại các biên hàm/annot (VD-227/228).
/// Đích `Opt(_)` bọc giá trị tới **ít nhất** đủ lớp của annot. Lõi được giữ đúng (giá trị đã là
/// `Opt`/`Result` thuộc đúng lớp đó không bị bọc thêm). (Checker từ chối từ nguồn lúc < `annot−1`
/// lớp — nên vòng lặp chỉ là kỹ thuật, không còn lớp > 1/chuyển đổi cho các chương trình hợp lệ.)
pub(in crate::interp) fn opt_lift_maybe(v: Value, depth_target: usize) -> Value {
    crate::lift::lift_opt(v, depth_target)
}

/// Số lớp `Opt` của một **đích kiểu** (đếm từ ngoài vào).
pub(in crate::interp) fn opt_depth_ty(t: &vis_types::Type) -> usize {
    let mut d = 0;
    let mut cur = t;
    while let vis_types::Type::Opt(inner) = cur {
        d += 1;
        cur = inner;
    }
    d
}

/// **VD-260** — `sym` hàm/phương thức → kiểu trả về (đủ để suy lớp Option của lời gọi).
pub(crate) fn collect_fn_rets(fns: &HashMap<u32, &HFn>) -> HashMap<u32, Type> {
    fns.iter().map(|(sym, f)| (*sym, f.ret.clone())).collect()
}

fn la_error(t: &Type) -> bool {
    matches!(t, Type::Error)
}

/// **VD-264** — số nút `Error` trong một kiểu (mirror codegen `so_loi`).
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

/// **VD-264** — thay `a` bằng kiểu suy `s` khi `a` là `Error` hoặc `s` có **ít nút `Error`
/// hơn** (mirror codegen `chon_kieu`).
fn chon_kieu(a: &mut Type, s: Type) {
    if la_error(a) || so_loi(&s) < so_loi(a) {
        *a = s;
    }
}

/// **VD-264** — kiểu `tuple` với các phần tử `Error` được suy lại từ biểu thức tương ứng
/// (mirror codegen `than::tuple_ty_suy`). Lower dựng `ty` từ `type_of`, mà lời gọi hàm/`Some`
/// cho `Error`, nên `(Some(1), 2)` mất kiểu `int?` của phần tử đầu.
fn tuple_ty_suy(
    items: &[HExpr],
    ty: &Type,
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Type {
    let Type::TypeRef { name, args } = ty else {
        return ty.clone();
    };
    if name != "tuple" {
        return ty.clone();
    }
    let args = args
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut a = t.clone();
            if let Some(s) = items
                .get(i)
                .and_then(|it| infer_ty(it, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut a, s);
            }
            a
        })
        .collect();
    Type::TypeRef {
        name: name.clone(),
        args,
    }
}

/// **VD-260** — suy kiểu tĩnh của biểu thức HIR (mirror codegen `than::infer_ty`); `None` khi
/// không biết ⇒ không nâng. `types` cho field bản ghi, `fn_rets` cho lời gọi hàm.
pub(crate) fn infer_ty(
    e: &HExpr,
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    match e {
        HExpr::Lit { ty, .. }
        | HExpr::Record { ty, .. }
        | HExpr::Opt { ty, .. }
        | HExpr::EnumLit { ty, .. } => Some(ty.clone()),
        // **VD-264**: xem `tuple_ty_suy`.
        HExpr::Tuple { items, ty, .. } => Some(tuple_ty_suy(items, ty, sym, types, fn_rets, mret)),
        // **VD-260**: list/map literal chưa có kiểu phần tử (lowering để `Error` khi phần tử
        // là `Some(…)`/`None`/lời gọi) — suy từ **phần tử đầu tiên biết kiểu**.
        // **VD-264**: cả khi kiểu phần tử đã có nhưng còn `Error` bên trong
        // (`[(Some(1), 2)]` ⇒ `tuple[Error, int]`), dùng kiểu suy được nếu **ít lỗi hơn**.
        HExpr::List { items, elem, .. } | HExpr::Set { items, elem, .. } => {
            let mut el = elem.clone();
            if let Some(t) = items
                .iter()
                .find_map(|i| infer_ty(i, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut el, t);
            }
            Some(Type::List(Box::new(el)))
        }
        HExpr::Map {
            entries,
            key,
            value,
            ..
        } => {
            let mut k = key.clone();
            if let Some(t) = entries
                .iter()
                .find_map(|(k, _)| infer_ty(k, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut k, t);
            }
            let mut v = value.clone();
            if let Some(t) = entries
                .iter()
                .find_map(|(_, v)| infer_ty(v, sym, types, fn_rets, mret))
            {
                chon_kieu(&mut v, t);
            }
            Some(Type::Map(Box::new(k), Box::new(v)))
        }
        HExpr::Var {
            name, sym: hsym, ..
        } => {
            if name == "None" {
                Some(Type::Opt(Box::new(Type::Error)))
            } else {
                hsym.as_ref().and_then(|id| sym.get(id)).cloned()
            }
        }
        HExpr::Field {
            base,
            name,
            optional,
            ..
        } => {
            let bt = infer_ty(base, sym, types, fn_rets, mret)?;
            let inner = match bt {
                Type::Opt(t) => *t,
                other => other,
            };
            let Type::Record(id) = inner else {
                return None;
            };
            let ft = types.field(id, name)?.ty.clone();
            Some(if *optional {
                Type::Opt(Box::new(ft))
            } else {
                ft
            })
        }
        HExpr::Index { base, .. } => match infer_ty(base, sym, types, fn_rets, mret)? {
            Type::Map(_, v) => Some(*v),
            Type::List(el) => Some(*el),
            _ => None,
        },
        HExpr::Call { callee, args, .. } => match (&**callee, args.first()) {
            (HExpr::Var { name, .. }, Some(a)) if name == "Some" => {
                infer_ty(a, sym, types, fn_rets, mret).map(|t| Type::Opt(Box::new(t)))
            }
            (HExpr::Var { name, .. }, _) if name == "None" => {
                Some(Type::Opt(Box::new(Type::Error)))
            }
            // Hàm **tệp** (kể cả trùng tên builtin) — ưu tiên chữ ký khai báo.
            (HExpr::Var { sym: Some(id), .. }, _) if fn_rets.contains_key(id) => {
                fn_rets.get(id).cloned()
            }
            // **VD-260**: free builtin `first(xs) -> T?` (mirror `cmdsig`/tyck).
            (HExpr::Var { name, .. }, Some(a)) if name == "first" => {
                infer_ty(a, sym, types, fn_rets, mret).map(|t| match t {
                    Type::List(e) => Type::Opt(e),
                    _ => Type::Opt(Box::new(Type::Error)),
                })
            }
            // **VD-260**: method dựng sẵn trả `Option` — `list.get/first/last/find`,
            // `map.get` (mirror tyck `method_ret`). Nhánh `if` gọi các method này cũng nâng.
            //
            // **VD-265**: phương thức **người dùng** còn dạng `Field` (không dispatch tĩnh được)
            // — tra bảng [`MethodRet`] theo kiểu nhận (`Record`/`Enum`/`TypeRef`, bóc `Opt`) để
            // biết `T?`/`list[T?]`/bản ghi; nhờ vậy `?.`/`with`/gán vẫn nâng đúng lớp Option.
            (HExpr::Field { base, name, .. }, _) => {
                let bt = infer_ty(base, sym, types, fn_rets, mret)?;
                match &bt {
                    Type::List(e) if matches!(name.as_str(), "get" | "first" | "last" | "find") => {
                        Some(Type::Opt(e.clone()))
                    }
                    Type::Map(_, v) if name == "get" => Some(Type::Opt(v.clone())),
                    _ => mret.ret(&bt, name, types),
                }
            }
            _ => None,
        },
        HExpr::With { base, .. } => infer_ty(base, sym, types, fn_rets, mret),
        // **VD-264**: `xs[i]?` = chỉ số có kiểm tra ⇒ `T?` (mirror checker `kiem_index_opt`);
        // `expr?` khác bóc một lớp `T?`/`Result[T, E]`.
        HExpr::Try { expr, .. } => {
            if let HExpr::Index { base, .. } = &**expr {
                return match infer_ty(base, sym, types, fn_rets, mret)? {
                    Type::List(el) => Some(Type::Opt(el)),
                    Type::Map(_, v) => Some(Type::Opt(v)),
                    Type::Bytes => Some(Type::Opt(Box::new(Type::Int))),
                    _ => None,
                };
            }
            match infer_ty(expr, sym, types, fn_rets, mret)? {
                Type::Opt(t) | Type::Result(t, _) => Some(*t),
                _ => None,
            }
        }

        HExpr::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, Some(else_), sym, types, fn_rets, mret),
        HExpr::Match { arms, .. } => infer_match(arms, sym, types, fn_rets, mret),
        _ => None,
    }
}

/// **VD-264** — gán kiểu đệ quy cho biến trong pattern (mirror codegen `bind_pat_ty`):
/// `Bind`, `Or`, `List`/`Rest`, `Tuple`, `Record`, `Ctor` (Option/Result + enum). Trước đây chỉ
/// `HPat::Bind` trần của `let`/`for` được gán ⇒ biến bind trong arm `match`/`if let` mất kiểu.
fn bind_pat_ty(p: &HPat, ty: &Type, sym: &mut HashMap<u32, Type>, types: &TypeTable) {
    match p {
        HPat::Bind { sym: id, .. } => {
            sym.insert(*id, ty.clone());
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                bind_pat_ty(a, ty, sym, types);
            }
        }
        HPat::List { items, rest, .. } => {
            let elem = match ty {
                Type::List(el) => (**el).clone(),
                _ => Type::Error,
            };
            for it in items {
                bind_pat_ty(it, &elem, sym, types);
            }
            if let Some(r) = rest {
                bind_pat_ty(r, &Type::List(Box::new(elem)), sym, types);
            }
        }
        HPat::Tuple { items, .. } => {
            let args: &[Type] = match ty {
                Type::TypeRef { name, args } if name == "tuple" => args,
                _ => &[],
            };
            for (i, it) in items.iter().enumerate() {
                let t = args.get(i).unwrap_or(&Type::Error);
                bind_pat_ty(it, t, sym, types);
            }
        }
        HPat::Record { fields, .. } => {
            let inner = match ty {
                Type::Opt(t) => &**t,
                other => other,
            };
            let id = match inner {
                Type::Record(id) => Some(*id),
                _ => None,
            };
            for (name, sub) in fields {
                let Some(sub) = sub else { continue };
                let ft = id
                    .and_then(|id| types.field(id, name))
                    .map(|f| f.ty.clone())
                    .unwrap_or(Type::Error);
                bind_pat_ty(sub, &ft, sym, types);
            }
        }
        HPat::Ctor { path, args, .. } => {
            let ten = path.last().map(String::as_str).unwrap_or("");
            for (i, a) in args.iter().enumerate() {
                let ft = ctor_arg_ty(ten, i, ty, types).unwrap_or(Type::Error);
                bind_pat_ty(a, &ft, sym, types);
            }
        }
        HPat::Wild { .. } | HPat::Lit { .. } | HPat::Range { .. } | HPat::Rest { .. } => {}
    }
}

/// **VD-264** — kiểu đối số thứ `i` của ctor `ten` khi chủ thể có kiểu `subj` (mirror codegen
/// `ctor_arg_ty`; interp đọc thẳng `TypeTable` thay vì bảng enum_fields).
fn ctor_arg_ty(ten: &str, i: usize, subj: &Type, types: &TypeTable) -> Option<Type> {
    match (ten, subj) {
        ("Some", Type::Opt(t)) if i == 0 => Some((**t).clone()),
        ("Ok", Type::Result(t, _)) if i == 0 => Some((**t).clone()),
        ("Err", Type::Result(_, e)) if i == 0 => Some((**e).clone()),
        (_, Type::Enum(id)) => types
            .variant(*id, ten)
            .and_then(|v| v.fields.get(i))
            .map(|f| f.ty.clone()),
        _ => None,
    }
}

/// **VD-260** — kiểu của giá trị cuối một khối (`None` = khối không cho giá trị).
fn infer_than(
    sts: &[HStmt],
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    match sts.last()? {
        HStmt::Expr(e) => infer_ty(e, sym, types, fn_rets, mret),
        HStmt::If {
            then, elifs, else_, ..
        } => infer_if(then, elifs, else_.as_deref(), sym, types, fn_rets, mret),
        HStmt::Match { arms, .. } => infer_match(arms, sym, types, fn_rets, mret),
        HStmt::Block { body, .. } | HStmt::Scope { body, .. } => {
            infer_than(body, sym, types, fn_rets, mret)
        }
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => None,
        _ => None,
    }
}

/// **VD-260** — chọn kiểu nhánh có **nhiều lớp Option nhất** (join theo kiểu đích).
fn hop_nhanh(ds: Vec<Type>) -> Option<Type> {
    ds.into_iter().max_by_key(opt_depth_ty)
}

fn infer_if(
    then: &[HStmt],
    elifs: &[(HCond, Vec<HStmt>)],
    else_: Option<&[HStmt]>,
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    let else_ = else_?;
    let mut ds = Vec::new();
    for body in std::iter::once(then)
        .chain(elifs.iter().map(|(_, b)| b.as_slice()))
        .chain(std::iter::once(else_))
    {
        if let Some(t) = infer_than(body, sym, types, fn_rets, mret) {
            ds.push(t);
        }
    }
    hop_nhanh(ds)
}

fn infer_match(
    arms: &[HArm],
    sym: &HashMap<u32, Type>,
    types: &TypeTable,
    fn_rets: &HashMap<u32, Type>,
    mret: &MethodRet,
) -> Option<Type> {
    let ds: Vec<Type> = arms
        .iter()
        .filter_map(|a| infer_than(&a.body, sym, types, fn_rets, mret))
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

/// **VD-264** — bind biến của `if let` / `elif let` / `while let` theo kiểu vế phải (nếu biết).
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
        let mret = vis_types::MethodRet::from_hir_voi_bang(&h.hir, &h.types);
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
