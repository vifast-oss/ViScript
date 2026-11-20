// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bộ thông dịch cây trên HIR: môi trường `sym → Value`, câu lệnh, biểu thức, pattern, gọi hàm.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use vis_syntax::Span;
use vis_types::hir::*;
use vis_types::lower::Symbols;
use vis_types::TypeTable;

use crate::gan::{gan_chi_so, gan_truong};
use crate::value::*;

#[path = "frame/core.rs"]
mod core_types;
use core_types::goi::{opt_depth_ty, opt_lift_maybe};
pub use core_types::*;

pub(crate) struct Interp<'a> {
    pub(crate) fns: HashMap<u32, &'a HFn>,
    consts: HashMap<u32, &'a HExpr>,
    /// `enum`/`type` khai trong tệp: `sym → (tên, là_enum)` để `E.A`/`User(...)` có "thẻ kiểu".
    pub(crate) tags: HashMap<u32, (String, bool)>,
    main: Option<&'a HFn>,
    stmts: &'a [HStmt],
    pub(crate) env: Env,
    pub(crate) mutable: HashSet<u32>,
    pub(crate) out: String,
    pub steps: usize,
    /// Giá trị của câu lệnh `Expr` **vừa chạy** (để `value` cấp tệp, không eval hai lần).
    last: Option<Value>,
    /// Độ sâu vòng lặp đang chạy (ST-13b mục 8: `break`/`continue` ngoài vòng ⇒ `VS0501`).
    loop_depth: u32,
    /// Độ sâu hàm đang chạy (ST-13b mục 8: `?` ngoài hàm ⇒ `VS0501`).
    fn_depth: u32,
    /// Độ sâu **khung gọi** đang mở (ST-15): vượt [`MAX_CALL_DEPTH`] ⇒ `VS0501`, không tràn stack.
    depth: u32,
    pub(crate) types: &'a TypeTable,
    #[allow(dead_code)]
    symbols: &'a Symbols,
    /// Host chương trình ngoài (ST-27); `None` ⇒ `VS0604` — interp **không** biết OS.
    pub(crate) ngoai: Option<&'a mut dyn crate::builtin::Ngoai>,
    /// Host hệ thống tệp (ST-33); `None` ⇒ `VS0604` — interp **không** biết `std::fs`.
    pub(crate) fs: Option<&'a mut dyn crate::builtin::FsHost>,
    /// **ST-63 (VD-105)**: host mạng; `None` ⇒ `VS0604` — interp **không** biết `std::net`.
    pub(crate) net: Option<&'a mut dyn crate::builtin::NetHost>,
    /// **VD-241 (Gate 1)**: host cơ sở dữ liệu; `None` ⇒ `std.sqlite.*` ⇒ `VS0604`.
    pub(crate) db: Option<&'a mut dyn crate::builtin::DbHost>,
    /// **ST-60**: hàng đợi tác vụ `async` (một `Scheduler` cho mỗi lần chạy).
    pub(crate) sched: std::cell::RefCell<crate::rt::Scheduler>,
    /// **ST-72b (VD-103/VD-106)**: quyền `main` — `(chỉ số tham số, giá trị)` theo thứ tự khai.
    caps: Vec<(u32, Value)>,
    // **ST-276 / VD-256**: instrumentation ngân sách nằm ở `crate::rt::fee`
    // (thread-local, mặc định tắt) — Interp không giữ trạng thái; hành vi mặc định y hệt cũ.
}

impl<'a> Interp<'a> {
    pub(crate) fn new(hir: &'a Hir, symbols: &'a Symbols, types: &'a TypeTable) -> Self {
        let mut fns = HashMap::new();
        let mut consts = HashMap::new();
        let mut tags = HashMap::new();
        let mut main = None;
        for item in &hir.items {
            match item {
                HItem::Fn(f) => {
                    fns.insert(f.sym, f);
                    if f.name == "main" {
                        main = Some(f);
                    }
                }
                HItem::Const(c) => {
                    consts.insert(c.sym, &c.value);
                }
                HItem::Enum(e) => {
                    tags.insert(e.sym, (e.name.clone(), true));
                }
                // ST-29: phương thức `impl` là hàm thường (dispatch tĩnh trỏ thẳng `sym`).
                HItem::Impl(i) => {
                    for m in &i.methods {
                        fns.insert(m.sym, m);
                    }
                }
                HItem::Rec(r) => {
                    tags.insert(r.sym, (r.name.clone(), false));
                }
                _ => {}
            }
        }
        Self {
            fns,
            consts,
            tags,
            main,
            stmts: &hir.stmts,
            env: Rc::new(std::cell::RefCell::new(HashMap::new())),
            mutable: HashSet::new(),
            out: String::new(),
            steps: 0,
            last: None,
            loop_depth: 0,
            fn_depth: 0,
            depth: 0,
            types,
            symbols,
            ngoai: None,
            fs: None,
            net: None,
            db: None,
            sched: std::cell::RefCell::new(crate::rt::Scheduler::new()),
            caps: Vec::new(),
        }
    }

    /// Gắn host chương trình ngoài (ST-27) — chỉ dùng trong crate (`run_with_ngoai`).
    #[allow(dead_code)]
    pub(crate) fn set_ngoai(&mut self, n: &'a mut dyn crate::builtin::Ngoai) {
        self.ngoai = Some(n);
    }

    /// Gắn host hệ thống tệp (ST-33) — chỉ dùng trong crate (`run_with_hosts`).
    #[allow(dead_code)]
    pub(crate) fn set_fs(&mut self, f: &'a mut dyn crate::builtin::FsHost) {
        self.fs = Some(f);
    }

    /// **ST-63 (VD-105)**: gắn host mạng — chỉ dùng trong crate (`run_with_hosts`).
    #[allow(dead_code)]
    pub(crate) fn set_net(&mut self, n: &'a mut dyn crate::builtin::NetHost) {
        self.net = Some(n);
    }

    /// **VD-241 (Gate 1)**: gắn host cơ sở dữ liệu — chỉ dùng trong crate (`run_with_hosts…`).
    #[allow(dead_code)]
    pub(crate) fn set_db(&mut self, d: &'a mut dyn crate::builtin::DbHost) {
        self.db = Some(d);
    }

    /// **ST-72b**: gắn giá trị quyền cho các tham số `cap` của `main`.
    pub(crate) fn set_caps(&mut self, caps: Vec<(u32, Value)>) {
        self.caps = caps;
    }

    pub(crate) fn take_stdout(&mut self) -> String {
        std::mem::take(&mut self.out)
    }

    pub(crate) fn run_program(&mut self, args: &[String]) -> R<Option<Value>> {
        let v = self.run_program_than(args)?;
        // **ST-60 (VD-90/VD-95)**: join mọi tác vụ **theo thứ tự tạo** — chỉ khi main **thành công**
        // (`?` ở trên đã chặn đường lỗi).
        crate::rt::join_all(self)?;
        Ok(v)
    }

    fn run_program_than(&mut self, args: &[String]) -> R<Option<Value>> {
        // Hằng cấp tệp trước (mọi khai báo item là "hoisted").
        let consts: Vec<(u32, &'a HExpr)> = self.consts.iter().map(|(k, v)| (*k, *v)).collect();
        for (sym, e) in consts {
            let v = self.eval(e)?;
            self.env.borrow_mut().insert(sym, v);
        }
        if let Some(f) = self.main {
            let argv = Value::List(args.iter().map(|s| Value::Str(s.clone())).collect());
            let v = if f.params.is_empty() {
                self.call_hfn(f, Vec::new(), Vec::new(), f.span)?
            } else if self.caps.is_empty() {
                // Hành vi cũ: `main` có tham số không `cap` ⇒ nhận **list args** ở tham số 1.
                self.call_hfn(f, vec![argv], Vec::new(), f.span)?
            } else {
                // **ST-72b**: đặt `Value::Cap` đúng **chỉ số tham số**; tham số khác giữ `argv`
                // (tương thích đường cũ). `caps` do CLI tính từ CST `main`.
                let mut gt: Vec<Value> = Vec::with_capacity(f.params.len());
                for i in 0..f.params.len() {
                    match self.caps.iter().find(|(pos, _)| *pos as usize == i) {
                        Some((_, v)) => gt.push(v.clone()),
                        None => gt.push(argv.clone()),
                    }
                }
                self.call_hfn(f, gt, Vec::new(), f.span)?
            };
            return Ok(Some(v));
        }
        let n = self.stmts.len();
        let mut last = None;
        for (i, s) in self.stmts.iter().enumerate() {
            self.last = None;
            match self.exec_stmt(s)? {
                Ctrl::Normal => {
                    if i + 1 == n {
                        last = self.last.take();
                    }
                }
                Ctrl::Break | Ctrl::Continue => {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`break`/`continue` ngoài vòng lặp",
                        s.span(),
                    )))
                }
            }
        }
        Ok(last)
    }

    // ---------------------------------------------------------------------
    // Câu lệnh
    // ---------------------------------------------------------------------

    fn exec_block(&mut self, body: &[HStmt]) -> R<Ctrl> {
        for s in body {
            // VD-47: giá trị của khối = giá trị câu lệnh **cuối** (`Expr`/`if`/`match`/khối con).
            self.last = None;
            match self.exec_stmt(s)? {
                Ctrl::Normal => {}
                c => return Ok(c),
            }
        }
        Ok(Ctrl::Normal)
    }

    /// Chạy thân lấy **giá trị**: biểu thức cuối cùng (nếu có), còn lại `()`.
    fn exec_body(&mut self, body: &[HStmt]) -> R<Value> {
        match self.exec_block(body)? {
            Ctrl::Normal => Ok(self.last.take().unwrap_or(Value::Unit)),
            _ => Ok(Value::Unit),
        }
    }

    fn exec_stmt(&mut self, s: &HStmt) -> R<Ctrl> {
        self.steps += 1;
        if let Some(a) = self.charge(crate::rt::fee::FeeKind::Stmt, s.span()) {
            return Err(a);
        }
        match s {
            HStmt::Let {
                pat,
                ty,
                init,
                else_,
                span,
            } => {
                let v = self.eval(init)?;
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span) {
                    return Err(a);
                }
                // **VD-226 §TEST (A0-proxy)**: tự nâng `T → T?` ở vế phải `let` khi chú thích
                // là `T?` — đồng nhất với codegen (codegen.rs) và spec §4.4 ("tự nâng ở ba vị trí:
                // gán let x: int? = …, trả về → int? …, đối số"). `Some(_)`/`None` giữ nguyên,
                // `Result` giữ nguyên; khác đi thì bọc `Value::some`.
                // **VD-241 §3 (A0, 2026-11-19)** — tự nâng `T → T? → …` tới đủ lớp của annot
                // (một lớp/biên chuyển đổi; không bọc thêm giá trị đã Option/Result).
                let v = if ty_là_opt(ty.as_ref()) {
                    let target = ty.as_ref().map(opt_depth_ty).unwrap_or(0);
                    opt_lift_maybe(v, target)
                } else {
                    v
                };
                // **R3b (VD-178)**: `let x = r else:` — bóc `Ok`/`Some`; ngược lại chạy khối `else`.
                if let Some(e) = else_ {
                    match v {
                        Value::Enum(ea, va, _) if ea == "Option" && va == "None" => {
                            return self.exec_block(&e.body);
                        }
                        Value::Enum(ea, va, ag)
                            if ea == "Result" && va == "Ok" && ag.len() == 1 =>
                        {
                            let mut ag = ag;
                            self.bind_pat(pat, &ag.pop().expect("Ok luôn có một đối số"))
                        }
                        Value::Enum(ea, va, ag)
                            if ea == "Option" && va == "Some" && ag.len() == 1 =>
                        {
                            let mut ag = ag;
                            self.bind_pat(pat, &ag.pop().expect("Some luôn có một đối số"))
                        }
                        Value::Enum(ea, va, ag)
                            if ea == "Result" && va == "Err" && ag.len() == 1 =>
                        {
                            let mut ag = ag;
                            if let Some(sym) = e.bind_sym {
                                self.env
                                    .borrow_mut()
                                    .insert(sym, ag.pop().expect("Err luôn có một đối số"));
                            }
                            return self.exec_block(&e.body);
                        }
                        other => self.bind_pat(pat, &other),
                    }
                } else {
                    self.bind_pat(pat, &v);
                }
                Ok(Ctrl::Normal)
            }
            HStmt::Var {
                sym,
                init,
                ty,
                span,
                ..
            } => {
                let v = self.eval(init)?;
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span) {
                    return Err(a);
                }
                // **VD-241 §3+ (A0, 2026-11-19)** — biên **khai `var`** có annot Optional nâng
                // giá trị như `let` (mirror codegen `HStmt::Var`): `var x: int?? = Some(7)` ⇒ `x`
                // bằng `Some(Some(7))`. `None` trần giữ nguyên (bước 1 — `lift_opt`).
                let v = if ty_là_opt(ty.as_ref()) {
                    let target_d = ty.as_ref().map(opt_depth_ty).unwrap_or(0);
                    opt_lift_maybe(v, target_d)
                } else {
                    v
                };
                self.env.borrow_mut().insert(*sym, v);
                self.mutable.insert(*sym);
                Ok(Ctrl::Normal)
            }
            HStmt::Assign {
                target,
                op,
                value,
                span,
            } => {
                // **ST-56b (VD-97)**: gán **trường/phần tử** của biến `var` — **cùng** ngữ nghĩa
                // codegen (xem `gan_truong`/`gan_chi_so`): thứ tự đánh giá **giống** codegen
                // (nạp ô → chỉ số → giá trị) và kết quả **ghi lại ô** (giá trị, không aliasing).
                // Chỉ `=`; gán ghép trên trường/phần tử đã bị tyck chặn trước.
                if matches!(op, AssignOp::Set) {
                    match target {
                        HExpr::Field { base, name, .. } => {
                            if let HExpr::Var { sym: Some(id), .. } = &**base {
                                let cu = self.doc_o(*id, target.span())?;
                                let dep = self.assign_depth(target, &cu); // ST-248: đích trường Opt
                                let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
                                let moi = gan_truong(cu, name, rhs, *span)?;
                                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span)
                                {
                                    return Err(a);
                                }
                                self.ghi_o(*id, moi, *span)?;
                                return Ok(Ctrl::Normal);
                            }
                        }
                        HExpr::Index { base, index, .. } => {
                            if let HExpr::Var { sym: Some(id), .. } = &**base {
                                let cu = self.doc_o(*id, target.span())?;
                                let dep = self.assign_depth(target, &cu); // ST-248: đích phần tử Opt
                                let i = self.eval(index)?;
                                let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
                                let moi = gan_chi_so(cu, i, rhs, *span)?;
                                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span)
                                {
                                    return Err(a);
                                }
                                self.ghi_o(*id, moi, *span)?;
                                return Ok(Ctrl::Normal);
                            }
                            // **ST-60b**: `p.xs[0] = v` (một tầng trường, gốc `var`) — đọc trường,
                            // gán phần tử, rồi **ghi lại trường** rồi ghi lại biến: đúng chuỗi
                            // `p = p with xs: (p.xs[i] = v)` mà codegen phát (`Field`/`Set`/`With`).
                            if let HExpr::Field {
                                base: goc, name, ..
                            } = &**base
                            {
                                if let HExpr::Var { sym: Some(id), .. } = &**goc {
                                    let cu = self.doc_o(*id, target.span())?;
                                    let dep = self.assign_depth(target, &cu); // ST-248: `x.f[i]`
                                    let truong = self.field_of(cu, name, *span)?;
                                    let i = self.eval(index)?;
                                    let rhs = crate::lift::lift_opt(self.eval(value)?, dep);
                                    let moi_truong = gan_chi_so(truong, i, rhs, *span)?;
                                    let cu2 = self.doc_o(*id, target.span())?;
                                    let moi = gan_truong(cu2, name, moi_truong, *span)?;
                                    if let Some(a) =
                                        self.charge(crate::rt::fee::FeeKind::Assign, *span)
                                    {
                                        return Err(a);
                                    }
                                    self.ghi_o(*id, moi, *span)?;
                                    return Ok(Ctrl::Normal);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let cur = self.lvalue_get(target)?;
                let rhs = self.eval(value)?;
                // **VD-241 §3+ (A0, 2026-11-19)** — biên **gán lại `=`** nâng một lớp khi đích là
                // biến `var` có annot Optional (`x = Some(7)` với `x: int??` ⇒ `x` bằng
                // `Some(Some(7))`). Đích `let` không gán được (tyck chặn trước). `None` trần giữ
                // nguyên (bước 1). **Chưa** nâng ở đích trường/chỉ số (`x.f = …`, `x[i] = …`) —
                // xem báo cáo.
                let rhs = match op {
                    AssignOp::Set => match target {
                        HExpr::Var { sym: Some(id), .. } => {
                            let want = self
                                .symbols
                                .get(*id)
                                .and_then(|s| s.ty.clone())
                                .filter(|t| matches!(t, vis_types::Type::Opt(_)));
                            match want {
                                Some(t) => opt_lift_maybe(rhs, opt_depth_ty(&t)),
                                None => rhs,
                            }
                        }
                        _ => rhs,
                    },
                    _ => rhs,
                };
                let new = match op {
                    AssignOp::Set => rhs,
                    _ => {
                        let bop = match op {
                            AssignOp::Add => BinOp::Add,
                            AssignOp::Sub => BinOp::Sub,
                            AssignOp::Mul => BinOp::Mul,
                            AssignOp::Div => BinOp::Div,
                            // **R7 (VD-178)**: `//=` — chia lấy phần nguyên.
                            AssignOp::FloorDiv => BinOp::FloorDiv,
                            _ => BinOp::Rem,
                        };
                        crate::builtin::bin(bop, cur, rhs, *span)?
                    }
                };
                if let Some(a) = self.charge(crate::rt::fee::FeeKind::Assign, *span) {
                    return Err(a);
                }
                self.lvalue_set(target, new, *span)?;
                Ok(Ctrl::Normal)
            }
            HStmt::Expr(e) => {
                let v = self.eval(e)?;
                self.last = Some(v);
                Ok(Ctrl::Normal)
            }
            HStmt::If {
                cond,
                then,
                elifs,
                else_,
                ..
            } => {
                // VD-47: `if` **có** `else` cho giá trị của nhánh đã chạy; **không** `else` ⇒ `()`.
                let has_else = matches!(else_, Some(b) if !b.is_empty());
                let mut ctrl = Ctrl::Normal;
                let mut ran = false;
                if self.eval_cond(cond)? {
                    ctrl = self.exec_block(then)?;
                    ran = true;
                }
                if !ran {
                    for (c, body) in elifs {
                        if self.eval_cond(c)? {
                            ctrl = self.exec_block(body)?;
                            ran = true;
                            break;
                        }
                    }
                }
                if !ran {
                    if let Some(b) = else_ {
                        if !b.is_empty() {
                            ctrl = self.exec_block(b)?;
                        }
                    }
                }
                if !has_else {
                    self.last = None;
                }
                Ok(ctrl)
            }
            HStmt::For {
                pat, iter, body, ..
            } => {
                let it = self.eval(iter)?;
                let items = self.iter_items(it, iter.span())?;
                self.loop_depth += 1;
                let mut ctrl = Ok(Ctrl::Normal);
                for v in items {
                    if let Some(a) = self.charge(crate::rt::fee::FeeKind::Loop, s.span()) {
                        ctrl = Err(a);
                        break;
                    }
                    self.bind_pat(pat, &v);
                    match self.exec_block(body) {
                        Ok(Ctrl::Break) => break,
                        Ok(Ctrl::Normal) => {}
                        Ok(Ctrl::Continue) => {}
                        Err(e) => {
                            ctrl = Err(e);
                            break;
                        }
                    }
                }
                self.loop_depth -= 1;
                self.last = None;
                ctrl
            }
            HStmt::While { cond, body, .. } => {
                let mut guard = 0usize;
                self.loop_depth += 1;
                // **ST-226 (R18b)**: `while let` dùng chung bộ máy `eval_cond` với `if let`.
                loop {
                    if let Some(a) = self.charge(crate::rt::fee::FeeKind::Loop, s.span()) {
                        self.loop_depth -= 1;
                        return Err(a);
                    }
                    if !self.eval_cond(cond)? {
                        break;
                    }
                    guard += 1;
                    if guard > 10_000_000 {
                        return Err(Abort::Trap(Trap::new(
                            RT,
                            "vòng lặp quá lâu (>10^7 bước)",
                            s.span(),
                        )));
                    }
                    let c = self.exec_block(body)?;
                    if c == Ctrl::Break {
                        break;
                    }
                }
                self.loop_depth -= 1;
                self.last = None;
                Ok(Ctrl::Normal)
            }
            HStmt::Loop { body, .. } => {
                self.loop_depth += 1;
                for _ in 0..10_000_000usize {
                    if let Some(a) = self.charge(crate::rt::fee::FeeKind::Loop, s.span()) {
                        self.loop_depth -= 1;
                        return Err(a);
                    }
                    if self.exec_block(body)? == Ctrl::Break {
                        self.loop_depth -= 1;
                        self.last = None;
                        return Ok(Ctrl::Normal);
                    }
                }
                self.loop_depth -= 1;
                Err(Abort::Trap(Trap::new(
                    RT,
                    "`loop` quá lâu (>10^7 bước)",
                    s.span(),
                )))
            }
            HStmt::Scope { body, .. } => self.exec_block(body),
            HStmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Unit,
                };
                Err(Abort::Ret(v))
            }
            HStmt::Break { .. } => {
                if self.loop_depth == 0 {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`break` ngoài vòng lặp (chỉ dùng trong `for`/`while`/`loop`)",
                        s.span(),
                    )));
                }
                Ok(Ctrl::Break)
            }
            HStmt::Continue { .. } => {
                if self.loop_depth == 0 {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`continue` ngoài vòng lặp",
                        s.span(),
                    )));
                }
                Ok(Ctrl::Continue)
            }
            HStmt::Match { subject, arms, .. } => {
                let subj = self.eval(subject)?;
                let Some((arm, binds)) = self.pick_arm(arms, &subj)? else {
                    return Err(Abort::Trap(Trap::new(
                        RT,
                        "`match` không khớp nhánh nào (không xét hết bị lọt)",
                        s.span(),
                    )));
                };
                for (sym, v) in binds {
                    self.env.borrow_mut().insert(sym, v);
                }
                self.exec_block(&arm.body)
            }
            HStmt::Block { body, .. } => self.exec_block(body),
        }
    }

    fn eval_cond(&mut self, c: &HCond) -> R<bool> {
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

    fn eval(&mut self, e: &HExpr) -> R<Value> {
        self.steps += 1;
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
                    if let Some(f) = self.fns.get(id) {
                        return Ok(Value::Fn(Rc::new(Closure {
                            name: f.name.clone(),
                            params: f.params.iter().map(|p| p.sym).collect(),
                            body: HBody::Stmts(f.body.clone()),
                            env: self.env.clone(),
                            ret_depth: opt_depth_ty(&f.ret),
                        })));
                    }
                    // `enum`/`type` khai trong tệp: thẻ kiểu để `E.A` / `User(...)` chạy được.
                    if let Some((n, is_enum)) = self.tags.get(id).cloned() {
                        return Ok(if is_enum {
                            Value::Enum(n, String::new(), Vec::new())
                        } else {
                            Value::Record(n, Vec::new())
                        });
                    }
                    // `print`/`len`/`str`/`Ok`/... **có** `sym` (ST-11 khai 8 tên dựng sẵn) ⇒ tra tên.
                    if self.symbols.kind_of(*id) == Some(vis_types::lower::SymKind::Builtin) {
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
            } => {
                let b = self.eval(base)?;
                // **ST-239 (VD-242 §3.2, ca 5)**: VM đúng, interp thiếu nâng một lớp `Opt`.
                // `None` ⇒ `None`; `Some(u)` ⇒ `Some(u.field)` — **không flatten**. Dùng chung
                // `lift_opt(v, 1)` với `$lift-opt` của VM để field đã `Opt` không bị bọc chồng.
                // Checker đã chặn base thường (`?.` chỉ trên `Option[U]`), nhưng giữ nhánh phòng vệ.
                let (b, nang) = if *optional {
                    match b {
                        Value::Enum(ea, va, mut ag) if ea == "Option" => match va.as_str() {
                            "None" => return Ok(Value::none()),
                            "Some" if ag.len() == 1 => {
                                (ag.pop().expect("Some luôn có một đối số"), true)
                            }
                            _ => (Value::Enum(ea, va, ag), false),
                        },
                        other => (other, false),
                    }
                } else {
                    (b, false)
                };
                let v = self.field_of(b, name, *span)?;
                Ok(if nang { crate::lift::lift_opt(v, 1) } else { v })
            }
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
                let mut kv: Vec<(Value, Value)> = Vec::new();
                for (k, v) in entries {
                    let kk = self.eval(k)?;
                    let vv = self.eval(v)?;
                    if let Some(e) = kv.iter_mut().find(|(kk2, _)| kk2.eq_value(&kk)) {
                        e.1 = vv;
                    } else {
                        kv.push((kk, vv));
                    }
                }
                Ok(Value::Map(kv))
            }
            HExpr::Record { fields, ty, span } => {
                let name = self.type_name(ty);
                let mut out = Vec::with_capacity(fields.len());
                for (n, v) in fields {
                    out.push((n.clone(), self.eval(v)?));
                    let _ = span;
                }
                Ok(Value::Record(name, out))
            }
            HExpr::EnumLit {
                path,
                variant,
                args,
                ty,
                span,
                ..
            } => {
                let mut vs = Vec::with_capacity(args.len());
                for a in args {
                    vs.push(self.eval(a)?);
                }
                let name = self.type_name(ty);
                let name = if name.is_empty() {
                    path.first().cloned().unwrap_or_default()
                } else {
                    name
                };
                let _ = span;
                Ok(Value::Enum(name, variant.clone(), vs))
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
                    (Value::Map(kv), k) => Value::opt(
                        kv.iter()
                            .find(|(kk, _)| kk.eq_value(k))
                            .map(|(_, v)| v.clone()),
                    ),
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
                match b {
                    Value::Record(name, mut cur) => {
                        for (n, v) in fields {
                            let nv = self.eval(v)?;
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
            } => {
                if self.eval_cond(cond)? {
                    return self.exec_body(then);
                }
                for (c, body) in elifs {
                    if self.eval_cond(c)? {
                        return self.exec_body(body);
                    }
                }
                self.exec_body(else_)
            }
            HExpr::Match {
                subject,
                arms,
                span,
                ..
            } => {
                let subj = self.eval(subject)?;
                match self.pick_arm(arms, &subj)? {
                    Some((arm, binds)) => {
                        for (sym, v) in binds {
                            self.env.borrow_mut().insert(sym, v);
                        }
                        self.exec_body(&arm.body)
                    }
                    None => Err(Abort::Trap(Trap::new(
                        RT,
                        "`match` không khớp nhánh nào (không xét hết bị lọt)",
                        *span,
                    ))),
                }
            }
            HExpr::Lambda {
                params, body, ret, ..
            } => Ok(Value::Fn(Rc::new(Closure {
                name: String::new(),
                params: params.iter().map(|p| p.sym).collect(),
                body: body.clone(),
                ret_depth: opt_depth_ty(ret),
                // ST-15b (mục 2): **chụp snapshot** môi trường tại điểm tạo closure — interp dùng
                // một env phẳng và khôi phục tham số khi hàm trả về, nên nếu giữ `Rc` chung thì
                // closure thoát khỏi hàm sẽ mất biến đã bắt. Chọn snapshot (ghi JOURNAL): biến
                // `var` cập nhật **sau** khi tạo closure thì closure vẫn thấy giá trị lúc tạo.
                env: Rc::new(std::cell::RefCell::new(self.env.borrow().clone())),
            }))),
            HExpr::Tuple { items, .. } => {
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
                let t = self
                    .sched
                    .get_mut()
                    .spawn(f.sym, pos, f.name.clone(), *span);
                Ok(Value::Task(t))
            }
            // ST-14: chuỗi nội suy — đánh giá từng mảnh rồi ghép (không parse lại lúc chạy).
            HExpr::Interp { parts, .. } => {
                let mut out = String::new();
                for p in parts {
                    match p {
                        vis_types::InterpPart::Str(t) => out.push_str(t),
                        vis_types::InterpPart::Expr(e) => {
                            let v = self.eval(e)?;
                            out.push_str(&v.show());
                        }
                        // **R6c (VD-178)**: `{expr:spec}` — định dạng lúc chạy.
                        vis_types::InterpPart::ExprSpec { expr, spec } => {
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

    fn type_name(&self, ty: &vis_types::Type) -> String {
        match ty {
            vis_types::Type::Record(id) => self
                .types
                .rec(*id)
                .and_then(|r| r.name.clone())
                .unwrap_or_default(),
            vis_types::Type::Enum(id) => self
                .types
                .enum_def(*id)
                .map(|e| e.name.clone())
                .unwrap_or_default(),
            _ => String::new(),
        }
    }

    // ---------------------------------------------------------------------
    // Lấy/đặt giá trị (lvalue), trường, chỉ số, vòng lặp
    // ---------------------------------------------------------------------

    /// **ST-56b** — nạp **giá trị ô** của biến (dùng cho gán trường/phần tử; đúng thứ tự codegen).
    fn doc_o(&mut self, id: u32, span: Span) -> R<Value> {
        self.env
            .borrow()
            .get(&id)
            .cloned()
            .ok_or_else(|| Abort::Trap(Trap::new(RT, "tên chưa có giá trị", span)))
    }

    /// **ST-56b** — ghi lại **ô** của biến; chỉ `var`/`var self` (tyck đã chặn trước, đây là chốt 2).
    fn ghi_o(&mut self, id: u32, v: Value, span: Span) -> R<()> {
        if !self.mutable.contains(&id) {
            return Err(Abort::Trap(Trap::new(
                MISMATCH,
                "không gán được trường của `let`/tham số — chỉ `var` mới gán được (§4.1)",
                span,
            )));
        }
        self.env.borrow_mut().insert(id, v);
        Ok(())
    }

    fn lvalue_get(&mut self, target: &HExpr) -> R<Value> {
        match target {
            HExpr::Var { sym: Some(id), .. } => self
                .env
                .borrow()
                .get(id)
                .cloned()
                .ok_or_else(|| Abort::Trap(Trap::new(RT, "tên chưa có giá trị", target.span()))),
            HExpr::Var { span, .. } => Err(Abort::Trap(Trap::new(
                RT,
                "gán cho tên chưa phân giải được",
                *span,
            ))),
            other => self.eval(other),
        }
    }

    fn lvalue_set(&mut self, target: &HExpr, v: Value, span: Span) -> R<()> {
        match target {
            HExpr::Var { sym: Some(id), .. } => {
                if !self.mutable.contains(id) {
                    return Err(Abort::Trap(Trap::new(
                        MISMATCH,
                        "không gán được cho `let`/tham số (chỉ `var` mới gán được, §4.1)",
                        span,
                    )));
                }
                self.env.borrow_mut().insert(*id, v);
                Ok(())
            }
            HExpr::Field { .. } | HExpr::Index { .. } => Err(Abort::Trap(Trap::new(
                RT,
                "gán vào trường/phần tử chưa hỗ trợ (dùng `with` cho bản ghi)",
                span,
            ))),
            _ => Err(Abort::Trap(Trap::new(RT, "đích gán không hợp lệ", span))),
        }
    }

    fn field_of(&mut self, b: Value, name: &str, span: Span) -> R<Value> {
        match b {
            Value::Record(_, fields) => fields
                .into_iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v)
                .ok_or_else(|| {
                    Abort::Trap(Trap::new(
                        RT,
                        format!("bản ghi không có trường `{name}`"),
                        span,
                    ))
                }),
            // **VD-204 (1)**: `Some(v)` không có trường; truy cập trường lên `Option`
            // bóc `Some(v)` rồi lấy trường của `v` (tương thích hành vi cũ với `Value::Opt`).
            Value::Enum(ea, va, args) if ea == "Option" && va == "Some" && args.len() == 1 => {
                let mut args = args;
                self.field_of(args.pop().expect("Some luôn có một đối số"), name, span)
            }
            // **Parity interp ↔ VM**: field access trên giá trị `Enum` không được định nghĩa ở
            // ViScript ⇒ `VS0501`. Ngoại lệ hợp lệ duy nhất: **atom biến thể rỗng** `E.A` —
            // `E` evaluate về `Enum(en, "", [])` (tag atom marker) và `x.Type` dựng biến thể rỗng.
            Value::Enum(en, variant, args) => {
                if variant.is_empty() && args.is_empty() {
                    Ok(Value::Enum(
                        en,
                        name.to_string(),
                        Vec::new(), /* zero-width */
                    ))
                } else {
                    Err(Abort::Trap(Trap::new(
                        RT,
                        format!("`{name}` không phải trường"),
                        span,
                    )))
                }
            }
            other => Err(Abort::Trap(Trap::new(
                RT,
                format!("`{}` không có trường `{name}`", other.show()),
                span,
            ))),
        }
    }

    fn index_of(&mut self, b: Value, i: Value, span: Span) -> R<Value> {
        match (b, i) {
            (Value::List(xs), Value::Int(n)) => {
                let len = xs.len() as i64;
                if n < 0 {
                    return Err(Abort::Trap(Trap::new(
                        IDX,
                        format!("chỉ số âm `{n}` (dùng `.last()` hoặc `xs[i]?`)"),
                        span,
                    )));
                }
                xs.get(n as usize).cloned().ok_or_else(|| {
                    Abort::Trap(Trap::new(
                        IDX,
                        format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                        span,
                    ))
                })
            }
            (Value::Map(kv), k) => kv
                .iter()
                .find(|(kk, _)| kk.eq_value(&k))
                .map(|(_, v)| v.clone())
                .ok_or_else(|| {
                    Abort::Trap(Trap::new(
                        IDX,
                        format!("khoá `{}` không có trong map", k.show()),
                        span,
                    ))
                }),
            (Value::Bytes(bs), Value::Int(n)) => {
                let len = bs.len() as i64;
                if n < 0 || n >= len {
                    return Err(Abort::Trap(Trap::new(
                        IDX,
                        format!("chỉ số {n} ngoài phạm vi (độ dài {len})"),
                        span,
                    )));
                }
                Ok(Value::Int(bs[n as usize] as i64))
            }
            (other, idx) => Err(Abort::Trap(Trap::new(
                RT,
                format!("không chỉ số được `{}` bằng `{}`", other.show(), idx.show()),
                span,
            ))),
        }
    }

    /// **ST-248 (VD-242 §3.3)** — số lớp `Opt` của đích gán trường/chỉ số. Kiểu đích lấy từ
    /// annotation của biến gốc, hoặc (bản ghi không annot) từ tên bản ghi runtime qua `TypeTable`.
    fn assign_depth(&self, t: &HExpr, root: &Value) -> usize {
        self.expr_ty(t, root)
            .map(|ty| opt_depth_ty(&ty))
            .unwrap_or(0)
    }

    /// **ST-248** — kiểu tĩnh của gốc/target (`Var`/`Field`/`Index`); `None` ⇒ không nâng.
    fn expr_ty(&self, e: &HExpr, root: &Value) -> Option<vis_types::Type> {
        match e {
            HExpr::Var { sym: Some(id), .. } => {
                if let Some(t) = self.symbols.get(*id).and_then(|s| s.ty.clone()) {
                    return Some(t);
                }
                match root {
                    Value::Record(ten, _) => {
                        self.types.rec_by_name(ten).map(vis_types::Type::Record)
                    }
                    _ => None,
                }
            }
            HExpr::Field { base, name, .. } => match self.expr_ty(base, root) {
                Some(vis_types::Type::Record(id)) => {
                    self.types.field(id, name).map(|f| f.ty.clone())
                }
                _ => None,
            },
            HExpr::Index { base, .. } => match self.expr_ty(base, root) {
                Some(vis_types::Type::Map(_, v)) => Some(*v),
                Some(vis_types::Type::List(el)) => Some(*el),
                _ => None,
            },
            _ => None,
        }
    }

    fn iter_items(&mut self, it: Value, span: Span) -> R<Vec<Value>> {
        crate::value::iter_items(it, span)
    }

    /// Chọn nhánh `match`: thử pattern rồi guard; trả binding của nhánh khớp.
    fn pick_arm<'s>(&mut self, arms: &'s [HArm], subj: &Value) -> R<Option<Picked<'s>>> {
        for arm in arms {
            let snapshot = self.env.borrow().clone();
            if !self.bind_pat_bool(&arm.pat, subj) {
                *self.env.borrow_mut() = snapshot;
                continue;
            }
            if let Some(g) = &arm.guard {
                let gv = self.eval(g)?;
                if !gv.truthy(g.span())? {
                    *self.env.borrow_mut() = snapshot;
                    continue;
                }
            }
            // Binding của nhánh: các sym vừa thêm/đổi so với snapshot.
            let now = self.env.borrow().clone();
            let mut binds = Vec::new();
            for (k, v) in now {
                binds.push((k, v));
            }
            *self.env.borrow_mut() = snapshot;
            return Ok(Some((arm, binds)));
        }
        Ok(None)
    }

    // ---------------------------------------------------------------------
    // Mã lỗi chung
    // ---------------------------------------------------------------------
}

/// Đọc literal thành `Value` (lỗi → thông điệp cho `VS0501`).
pub fn lit_value(lit: &Lit) -> std::result::Result<Value, String> {
    match lit.kind {
        LitKind::Int => parse_int_lit(&lit.text)
            .map(Value::Int)
            .ok_or_else(|| format!("literal int không đọc được: `{}`", lit.text)),
        LitKind::Float => lit
            .text
            .replace('_', "")
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("literal float không đọc được: `{}`", lit.text)),
        LitKind::Bool => match lit.text.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(format!("literal bool lạ: `{other}`")),
        },
        LitKind::Char => parse_char_lit(&lit.text)
            .map(Value::Char)
            .ok_or_else(|| format!("literal char không đọc được: `{}`", lit.text)),
        LitKind::Str => Ok(Value::Str(parse_str_lit(&lit.text, false))),
        LitKind::Bytes => parse_bytes_lit(&lit.text)
            .map(Value::Bytes)
            .ok_or_else(|| format!("literal bytes không đọc được: `{}`", lit.text)),
        // **R6a (VD-178)**: `p"…"` **không** escape (đường dẫn Windows).
        LitKind::Path => Ok(Value::Path(parse_str_lit(&lit.text, true))),
        LitKind::Size => parse_size(&lit.text)
            .map(Value::Size)
            .ok_or_else(|| format!("literal size không đọc được: `{}`", lit.text)),
        LitKind::Duration => parse_duration(&lit.text)
            .map(Value::Duration)
            .ok_or_else(|| format!("literal duration không đọc được: `{}`", lit.text)),
        LitKind::Percent => parse_percent(&lit.text)
            .map(Value::Percent)
            .ok_or_else(|| format!("literal percent không đọc được: `{}`", lit.text)),
    }
}

fn parse_int_lit(text: &str) -> Option<i64> {
    let t = text.replace('_', "");
    if let Some(h) = t.strip_prefix("0x") {
        return i64::from_str_radix(h, 16).ok();
    }
    if let Some(b) = t.strip_prefix("0b") {
        return i64::from_str_radix(b, 2).ok();
    }
    if let Some(o) = t.strip_prefix("0o") {
        return i64::from_str_radix(o, 8).ok();
    }
    t.parse().ok()
}

/// Vị trí `dòng:cột` (cột đếm theo **Unicode scalar**, bắt đầu từ 1 — §2.4) từ `span` + văn bản.
pub fn pos(text: &str, span: Span) -> (u32, u32) {
    let mut line = 1u32;
    let mut col = 1u32;
    for (i, c) in text.char_indices() {
        if i >= span.start {
            break;
        }
        match c {
            '\n' => {
                line += 1;
                col = 1;
            }
            // ST-13b (mục 7): tab = **4 cột** (SPEC/VD-12), không phải 1.
            '\t' => col += 4,
            _ => col += 1,
        }
    }
    (line, col)
}

/// 8 tên dựng sẵn (VD-44 #3) mà bộ thông dịch gọi trực tiếp.
/// Cap **độ sâu khung gọi** (ST-15): **2.000** khung (đo 10^4 tràn stack cả trên luồng 256 MiB) —
/// vượt ⇒ `VS0501` thay vì tràn stack process.
pub(crate) const MAX_CALL_DEPTH: u32 = 2_000;

fn builtin_name(name: &str) -> Option<&'static str> {
    // ST-27: chương trình ngoài — `builtin::call` sẽ hỏi **host** (`None` ⇒ `VS0604`).
    if name.starts_with("exec.") {
        return Some("exec");
    }
    // ST-26: `std.<module>.<fn>` gọi qua bảng `stdlib` (dùng chung với VM, VD-55).
    if name.starts_with("std.") {
        return name
            .strip_prefix("std.")
            .and_then(vis_types::stdlib::sig_of)
            .map(|f| f.name);
    }
    match name {
        "print" => Some("print"),
        "assert" => Some("assert"),
        "len" => Some("len"),
        "str" => Some("str"),
        "Ok" => Some("Ok"),
        "Err" => Some("Err"),
        "Some" => Some("Some"),
        "None" => Some("None"),
        "ref" => Some("ref"),
        // ST-18: 3 lệnh builtin của chế độ lệnh (VS0) — chữ ký ở `vis-types/lower/cmd.rs`.
        "echo" => Some("echo"),
        "lines" => Some("lines"),
        "take" => Some("take"),
        // ST-38: closure-arg của chế độ lệnh.
        "sort_by" => Some("sort_by"),
        "render" => Some("render"),
        "ls" => Some("ls"),
        "open" => Some("open"),
        "get" => Some("get"),
        "select" => Some("select"),
        "where" => Some("where"),
        "each" => Some("each"),
        "first" => Some("first"),
        "count" => Some("count"),
        _ => None,
    }
}

impl Value {
    /// Chân trị cho `if`/`while`/`and`/`or`/`not`/guard.
    pub fn truthy(&self, span: Span) -> R<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Enum(ea, va, ag) if ea == "Option" => {
                Ok(matches!((va.as_str(), ag.len()), ("Some", 1)))
            }
            other => Err(Abort::Trap(Trap::new(
                RT,
                format!("điều kiện cần `bool`, được `{}`", other.show()),
                span,
            ))),
        }
    }
}

/// **R6c (VD-178)**: định dạng `{expr:spec}` — hiện hỗ trợ `.N` (N chữ số thập phân).
pub fn dinh_dang(v: &Value, spec: &str, span: Span) -> Result<String, Abort> {
    let Some(n) = spec.strip_prefix('.') else {
        return Err(Abort::Trap(Trap::new(
            RT,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N`"),
            span,
        )));
    };
    let n: usize = n.parse().map_err(|_| {
        Abort::Trap(Trap::new(
            RT,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N`"),
            span,
        ))
    })?;
    Ok(match v {
        Value::Float(f) => format!("{f:.n$}"),
        Value::Int(i) => format!("{:.n$}", *i as f64),
        Value::Str(s) => s.clone(),
        other => other.show(),
    })
}

/// `true` khi `let x: T? = …` (chú thích `Optional` — mọi lớp).
fn ty_là_opt(t: Option<&vis_types::ty::Type>) -> bool {
    matches!(t, Some(vis_types::ty::Type::Opt(_)))
}
