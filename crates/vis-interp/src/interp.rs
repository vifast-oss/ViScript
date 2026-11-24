// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bộ thông dịch cây trên HIR: môi trường `sym → Value`, câu lệnh, biểu thức, pattern, gọi hàm.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use vis_hir::hir::*;
use vis_hir::Symbols;
use vis_hir::TypeTable;
use vis_syntax::Span;

use crate::gan::{gan_chi_so, gan_truong};
use crate::value::*;

#[path = "frame/core.rs"]
mod core_types;
mod eval;
#[path = "interp/gan_long.rs"]
pub(crate) mod gan_long;
mod lit;
#[path = "interp/opt_method.rs"]
mod opt_method;
mod stmt;
/// **VS3-3b/W5 (P1)**: runner W=1 non-frame (chạy thân trên caller).
#[path = "interp/worker.rs"]
pub(crate) mod worker;
use core_types::goi::{opt_depth_ty, opt_lift_maybe, ty_là_opt};
pub use core_types::*;
pub use lit::*;

/// **VD-267** — khoá cache tên enum theo vị trí nguồn `(file, start, end)`.
type EnumTagKey = (u16, usize, usize);
/// **VD-267** — cặp `(tên enum, tên biến thể)` đã nội bộ hoá.
type EnumTag = (Name, Name);

pub(crate) struct Interp<'a> {
    pub(crate) fns: HashMap<u32, &'a HFn>,
    consts: HashMap<u32, &'a HExpr>,
    /// `enum`/`type` khai trong tệp: `sym → (tên, là_enum)` để `E.A`/`User(...)` có "thẻ kiểu".
    pub(crate) tags: HashMap<u32, (String, bool)>,
    main: Option<&'a HFn>,
    stmts: &'a [HStmt],
    pub(crate) env: Env,
    /// **VD-260** — kiểu tĩnh suy từ annot + initializer (mirror codegen `collect_sym_tys`);
    /// dùng cho đích gán `var`/field/index và nâng nhánh `if`/`match`.
    pub(crate) sym_ty: HashMap<u32, vis_hir::Type>,
    /// **VD-260** — `sym` hàm/phương thức → kiểu trả về (suy lớp Option của lời gọi).
    pub(crate) fn_rets: HashMap<u32, vis_hir::Type>,
    /// **VD-265** — bảng kiểu trả của phương thức người dùng theo kiểu nhận + tên (cho lời gọi
    /// còn dạng `HExpr::Field` và cho thay generic ở instantiation).
    pub(crate) mret: vis_hir::MethodRet,
    pub(crate) mutable: HashSet<u32>,
    pub(crate) out: String,
    pub steps: usize,
    /// **VS3-3b/W5 (P2)** — `sym` binding cấp tệp để snapshot worker (không lấy local `main`).
    pub(crate) top_syms: Vec<u32>,
    /// **VS3-3b/W5 (P2)** — cờ huỷ hợp tác do worker pool gắn; `None` ở runtime thường.
    pub(crate) worker_cancel: Option<Arc<AtomicBool>>,
    /// **VS3-3b/W5 (P2)** — hạn huỷ theo thời gian (test hook cancel-after); `None` thường.
    pub(crate) worker_cancel_deadline: Option<std::time::Instant>,
    /// **VS3-3b/W5 (P2)** — lease ngân sách atomic của task worker; `None` ở runtime thường.
    pub(crate) worker_budget: Option<crate::rt::fee::BudgetLease>,
    /// Giá trị của câu lệnh `Expr` **vừa chạy** (để `value` cấp tệp, không eval hai lần).
    last: Option<Value>,
    /// Độ sâu vòng lặp đang chạy (ST-13b mục 8: `break`/`continue` ngoài vòng ⇒ `VS0501`).
    loop_depth: u32,
    /// Độ sâu hàm đang chạy (ST-13b mục 8: `?` ngoài hàm ⇒ `VS0501`).
    fn_depth: u32,
    /// Độ sâu **khung gọi** đang mở (ST-15): vượt [`MAX_CALL_DEPTH`] ⇒ `VS0501`, không tràn stack.
    depth: u32,
    pub(crate) types: &'a TypeTable,
    /// **VD-267** — nhớ `(tên enum, tên biến thể)` theo **vị trí nguồn** của `EnumLit`
    /// (`file,start,end`). Một literal trong vòng lặp chỉ nội bộ hoá tên **một lần**; các lần
    /// sau chỉ clone `Name` (refcount). Khoá theo vị trí nguồn nên không phình khi HIR bị clone
    /// (closure tạo trong vòng lặp vẫn dùng chung span); nội dung được **xác minh** trước khi
    /// dùng để hai literal khác nhau lỡ trùng span không bao giờ nhận nhầm tên.
    enum_tags: RefCell<HashMap<EnumTagKey, EnumTag>>,
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
        let fn_rets = core_types::goi::collect_fn_rets(&fns);
        let mret = vis_hir::MethodRet::from_hir_voi_bang(hir, types);
        let sym_ty = core_types::goi::collect_sym_tys(hir, symbols, types, &fn_rets, &mret);
        Self {
            fns,
            consts,
            tags,
            main,
            stmts: &hir.stmts,
            env: Rc::new(std::cell::RefCell::new(HashMap::new())),
            sym_ty,
            fn_rets,
            mret,
            mutable: HashSet::new(),
            out: String::new(),
            steps: 0,
            top_syms: crate::interp::worker::top_syms(&hir.stmts),
            worker_cancel: None,
            worker_cancel_deadline: None,
            worker_budget: None,
            last: None,
            loop_depth: 0,
            fn_depth: 0,
            depth: 0,
            types,
            enum_tags: RefCell::new(HashMap::new()),
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

    /// **VS3-1d** — đọc caps đã gắn (frame driver dùng để đặt đúng tham số `main`).
    pub(crate) fn caps(&self) -> &[(u32, Value)] {
        &self.caps
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
            self.khoi_tao_bien_cap_tep()?; // VD-262: biến cấp tệp trước `main`.
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
            match self.exec_stmt(s) {
                Ok(Ctrl::Normal) => {}
                Ok(c) => return Ok(c),
                // **VD-263**: `break`/`continue` phát ra từ **biểu thức con** (`if`/`match` ở vị
                // trí giá trị, đối số, toán hạng, …) — đổi lại thành `Ctrl` tại biên câu lệnh.
                Err(e) => {
                    let c = Self::tach_ctrl(e)?;
                    return Ok(c);
                }
            }
        }
        Ok(Ctrl::Normal)
    }

    fn type_name(&self, ty: &vis_hir::Type) -> String {
        match ty {
            vis_hir::Type::Record(id) => self
                .types
                .rec(*id)
                .and_then(|r| r.name.clone())
                .unwrap_or_default(),
            vis_hir::Type::Enum(id) => self
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
        crate::gan::doc_truong(b, name, span)
    }

    fn index_of(&mut self, b: Value, i: Value, span: Span) -> R<Value> {
        crate::gan::doc_chi_so(b, i, span)
    }

    /// **ST-248 (VD-242 §3.3)** — số lớp `Opt` của đích gán trường/chỉ số. Kiểu đích lấy từ
    /// annotation của biến gốc, hoặc (bản ghi không annot) từ tên bản ghi runtime qua `TypeTable`.
    fn assign_depth(&self, t: &HExpr, root: &Value) -> usize {
        self.expr_ty(t, root)
            .map(|ty| opt_depth_ty(&ty))
            .unwrap_or(0)
    }

    // `expr_ty` đã chuyển sang `interp/goi.rs` (VD-264) để dùng `infer_ty` đầy đủ.
    fn iter_items(&mut self, it: Value, span: Span) -> R<ForIter> {
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
