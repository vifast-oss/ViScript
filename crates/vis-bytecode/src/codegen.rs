// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ **HIR** (`vis-types`) xuống [`Module`] của VBC1 (ST-19).
//!
//! Nguyên tắc:
//! - **Không** chép ngữ nghĩa toán tử: `Bin`/`Un` mang **chính** `vis_types::BinOp`/`UnOp` — VM
//!   (ST-20) sẽ dùng chung bảng ngữ nghĩa với `vis-interp`.
//! - `match` hạ thành **chuỗi test + nhảy** (VM không cần biết pattern).
//! - Closure **chụp snapshot** cả khung hiện tại (VD-51): `MakeClosure(fidx, nfree)` với
//!   `nfree = số ô cục bộ của hàm cha tại điểm tạo`; trong closure đọc bằng `LoadCaptured`.
//! - `max_stack`: đếm **bảo thủ** trong lúc phát (upper bound ≥ độ sâu thật). **ST-21 kiểm lại
//!   độc lập từ byte stream** — không tin giá trị này.

mod builtin_value;
mod gia_tri;
mod if_expr;
mod pattern;
mod than;

use std::collections::HashMap;

use vis_syntax::Span;
use vis_types::{HBody, HCond, HExpr, HFn, HItem, HPat, HStmt, Hir, InterpPart, LitKind, Symbols};

pub use crate::cerr::CodegenError;
use crate::fee::{FeeEntry, FeeKind, FeeTable};
use crate::format::{binop_tag, unop_tag};
use crate::format::{Const, Func, Instr, Module, HASH_LEN};
use crate::util::{assign_binop, can_test, la_builtin, Vong};

#[path = "codegen/f.rs"]
mod f_mod;
use f_mod::F;

/// Bộ phát mã.
struct Gen {
    consts: Vec<Const>,
    funcs: Vec<Func>,
    /// **ST-276**: bảng phụ S của từng hàm (cùng chỉ số với `funcs`).
    fees: Vec<Vec<FeeEntry>>,
    /// `sym` hàm → chỉ số trong `funcs`.
    fmap: HashMap<u32, u32>,
    /// `sym` enum → tên enum (để `E.A` thành `MakeEnum`, không phải `Field`).
    enums: HashMap<u32, String>,
    /// **R11 (VD-178)**: `RecId` → tên bản ghi (để `U(n: 1)` in `U(...)`).
    recs: HashMap<u32, String>,
    /// Tên hàm → chỉ số (dự phòng khi `sym` của lời gọi là `None`).
    fmap_ten: HashMap<String, u32>,
    /// Tên hàm → danh sách `(tên tham số, là named)` để xếp đối số theo **tên**.
    fparams: HashMap<String, Vec<(String, bool)>>,
    /// Closure chờ phát: (chỉ số hàm, thân, tham số, snapshot locals của cha).
    cho: Vec<Cho>,
    /// `sym` dựng sẵn → tên dựng sẵn (`print`, `Ok`, …).
    builtins: HashMap<u32, String>,
    sym_ty: HashMap<u32, vis_types::Type>,
    rec_fields: HashMap<(u32, String), vis_types::Type>, // ST-248: kiểu trường bản ghi
    /// **ST-60**: chỉ số hàm của các `async fn` — call-site async hạ thành `$tac_vu`.
    async_fns: std::collections::HashSet<u32>,
    /// **ST-238 (VD-242, ca 3)**: `sym` của biến `let`/`var` cấp tệp → ô trong vùng
    /// [`crate::format::GLOBAL_SLOT_BASE`]. Mọi hàm đọc/ghi qua vùng này dùng chung một kho ở VM.
    globals: HashMap<u32, u16>,
    /// **ST-238 (VD-242, ca 4)**: `sym` của `const` cấp tệp → biểu thức giá trị (inline tại
    /// mỗi chỗ dùng, như interp nạp hằng lúc load).
    hconsts: HashMap<u32, HExpr>,
    hash: [u8; HASH_LEN],
}

struct Cho {
    idx: u32,
    params: Vec<vis_types::HParam>,
    body: HBody,
    captured: Vec<(u32, u16)>,
    name: String,
    ret_depth: usize,
}

/// **ST-238 (VD-242, ca 3)**: gom mọi `sym` được bind bởi một pattern (dùng để đăng ký biến
/// cấp tệp). Không gọi mã người dùng (VD-242 §3.4) — chỉ đi cấu trúc HIR.
fn pat_syms(p: &HPat, out: &mut Vec<u32>) {
    match p {
        HPat::Bind { sym, .. } => out.push(*sym),
        HPat::Ctor { args, .. } => {
            for a in args {
                pat_syms(a, out);
            }
        }
        HPat::Tuple { items, .. } | HPat::List { items, .. } => {
            for it in items {
                pat_syms(it, out);
            }
            if let HPat::List { rest: Some(r), .. } = p {
                pat_syms(r, out);
            }
        }
        HPat::Record { fields, .. } => {
            for (_, sub) in fields {
                if let Some(sub) = sub {
                    pat_syms(sub, out);
                }
            }
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                pat_syms(a, out);
            }
        }
        HPat::Range { lo, hi, .. } => {
            if let Some(lo) = lo {
                pat_syms(lo, out);
            }
            if let Some(hi) = hi {
                pat_syms(hi, out);
            }
        }
        HPat::Wild { .. } | HPat::Lit { .. } | HPat::Rest { .. } => {}
    }
}

impl Gen {
    fn cst(&mut self, kind: LitKind, text: &str) -> u32 {
        let kind = match kind {
            LitKind::Int => 0,
            LitKind::Float => 1,
            LitKind::Size => 2,
            LitKind::Duration => 3,
            LitKind::Percent => 4,
            LitKind::Str => 5,
            LitKind::Bytes => 6,
            LitKind::Path => 7,
            LitKind::Char => 8,
            LitKind::Bool => 9,
        };
        let c = Const {
            kind,
            text: text.to_string(),
        };
        if let Some(i) = self.consts.iter().position(|x| *x == c) {
            return i as u32;
        }
        self.consts.push(c);
        (self.consts.len() - 1) as u32
    }

    fn cst_str(&mut self, s: &str) -> u32 {
        self.cst(LitKind::Str, s)
    }
}

/// **ST-54**: cửa codegen **duy nhất** — từ chối khi front end đã có lỗi.
///
/// HIR do bộ hạ sinh **không** mang dấu `Type::Error` (đo 370 ca: 0 nút `Error`), nên codegen
/// **không thể** tự suy ra lỗi tyck; phải nhận **chẩn đoán front end** (parse/hạ/tyck) ở cửa này.
/// Chỉ chẩn đoán **`Severity::Error`** chặn; cảnh báo (nếu có) vẫn cho compile.
pub fn compile_kiem(
    hir: &Hir,
    symbols: &Symbols,
    text: &str,
    chan_doan: &[vis_syntax::Diagnostic],
) -> Result<Module, CodegenError> {
    if let Some(d) = chan_doan
        .iter()
        .find(|d| d.code.severity == vis_diag::Severity::Error)
    {
        return Err(CodegenError::ChanDoan {
            short: d.code.short,
            dong: d.line,
            cot: d.col,
            msg: d.message.clone(),
        });
    }
    compile(hir, symbols, text)
}

/// Hạ một chương trình HIR xuống module VBC1 (API cũ — **không** kèm chẩn đoán; xem
/// [`compile_kiem`] cho đường có cổng front end).
pub fn compile(hir: &Hir, symbols: &Symbols, text: &str) -> Result<Module, CodegenError> {
    compile_with_fee(hir, symbols, text).map(|(m, _)| m)
}

/// **ST-276 (VD-256)** — như [`compile`] nhưng trả kèm **bảng phụ S** (sự kiện ngữ nghĩa theo
/// PC). Đường mặc định (`compile`) **không** dùng bảng này; chỉ test-hook ngân sách đóng trailer.
pub fn compile_with_fee(
    hir: &Hir,
    symbols: &Symbols,
    text: &str,
) -> Result<(Module, FeeTable), CodegenError> {
    let mut g = Gen {
        consts: Vec::new(),
        funcs: Vec::new(),
        fees: Vec::new(),
        fmap: HashMap::new(),
        enums: HashMap::new(),
        recs: HashMap::new(),
        fmap_ten: HashMap::new(),
        fparams: HashMap::new(),
        cho: Vec::new(),
        async_fns: std::collections::HashSet::new(),
        globals: HashMap::new(),
        hconsts: HashMap::new(),
        builtins: crate::util::tinh_cac_builtin(symbols),
        sym_ty: than::collect_sym_tys(hir, symbols), // ST-248: annot + kiểu suy từ initializer
        rec_fields: than::collect_rec_fields(hir),
        hash: crate::format::source_hash(text),
    };
    // Lượt 1: cấp chỉ số cho mọi hàm cấp tệp + ghi tên enum.
    for it in &hir.items {
        // **ST-238 (VD-242, ca 4)**: gom `const` cấp tệp để inline tại chỗ dùng.
        if let HItem::Const(c) = it {
            g.hconsts.insert(c.sym, c.value.clone());
        }
        if let HItem::Enum(e) = it {
            g.enums.insert(e.sym, e.name.clone());
        }
        // **R11 (VD-178)**: ghi tên bản ghi để `MakeRecord` in đúng `U(...)`.
        if let HItem::Rec(r) = it {
            g.recs.insert(r.id.0, r.name.clone());
        }
        // ST-29: hàm `impl` cũng là hàm thường (dispatch tĩnh trỏ thẳng `sym`).
        let ham: Vec<&HFn> = match it {
            HItem::Fn(f) => vec![f],
            HItem::Impl(i) => i.methods.iter().collect(),
            _ => Vec::new(),
        };
        for f in ham {
            g.fmap.insert(f.sym, g.funcs.len() as u32);
            g.fmap_ten.insert(f.name.clone(), g.funcs.len() as u32);
            if f.is_async {
                g.async_fns.insert(g.funcs.len() as u32);
            }
            g.fparams.insert(
                f.name.clone(),
                f.params.iter().map(|p| (p.name.clone(), p.named)).collect(),
            );
            g.funcs.push(Func::default());
            g.fees.push(Vec::new());
        }
    }
    // **ST-238 (VD-242, ca 3)**: đăng ký biến cấp tệp (`let`/`var` ở thân tệp) vào vùng ô chung.
    // Cấp phát **trước** khi phát thân hàm để mọi hàm thấy cùng một `sym` → cùng một ô.
    {
        let mut syms: Vec<u32> = Vec::new();
        for s in &hir.stmts {
            match s {
                HStmt::Let { pat, else_, .. } => {
                    pat_syms(pat, &mut syms);
                    if let Some(e) = else_ {
                        if let Some(sym) = e.bind_sym {
                            syms.push(sym);
                        }
                    }
                }
                HStmt::Var { sym, .. } => syms.push(*sym),
                _ => {}
            }
        }
        for sym in syms {
            if !g.globals.contains_key(&sym) {
                let slot = crate::format::GLOBAL_SLOT_BASE + g.globals.len() as u16;
                g.globals.insert(sym, slot);
            }
        }
    }
    // `main` — câu lệnh cấp tệp.
    let main_idx = g.funcs.len() as u32;
    g.funcs.push(Func::default());
    g.fees.push(Vec::new());
    // Lượt 2: thân hàm cấp tệp.
    for it in &hir.items {
        let ham: Vec<&HFn> = match it {
            HItem::Fn(f) => vec![f],
            HItem::Impl(i) => i.methods.iter().collect(),
            _ => Vec::new(),
        };
        for f in ham {
            let idx = g.fmap[&f.sym] as usize;
            let mut st = Emit {
                g: &mut g,
                f: F::new(f.name.clone()),
                loops: Vec::new(),
                captured: HashMap::new(),
                ret_depth: than::opt_depth(&f.ret),
            };
            st.f.arity = f.params.iter().filter(|p| !p.named).count() as u16;
            st.f.nparams = f.params.len() as u16;
            for p in &f.params {
                st.f.slot(p.sym, p.span)?;
            }
            // Tham số có `default`: nếu ô còn `()` (không được truyền) ⇒ gán default.
            for p in &f.params {
                if let Some(d) = &p.default {
                    let slot = st.f.slot(p.sym, p.span)?;
                    st.f.push(Instr::LoadLocal(slot), p.span, 1);
                    st.f.push(Instr::IsUnit, p.span, 0);
                    let j = st.f.here();
                    st.f.push(Instr::JumpIfFalse(0), p.span, -1);
                    st.expr(d)?;
                    st.f.push(Instr::StoreLocal(slot), p.span, -1);
                    let end = st.f.here();
                    st.f.patch(j, end);
                }
            }
            // **VD-228b**: `int?`/`str?` annot trên tham số — nâng ngay trong biên hàm.
            st.lift_params_opt(&f.params, f.span)?;
            st.than(&HBody::Stmts(f.body.clone()), f.params.len() as u16, f.span)?;
            st.f.push(Instr::Halt, f.span, 0);
            let (done, fee) = st.f.finish()?;
            st.g.funcs[idx] = done;
            st.g.fees[idx] = fee;
        }
    }
    // `main`.
    {
        let mut st = Emit {
            g: &mut g,
            f: F::new("main".to_string()),
            loops: Vec::new(),
            captured: HashMap::new(),
            ret_depth: 0,
        };
        // ST-28: **cùng** ngữ nghĩa với interp — nếu tệp khai `fn main` thì chạy **hàm đó**
        // (trước đây VM bỏ qua `main` ⇒ lệch interp; phát hiện khi làm generic).
        if let Some(fi) = st.g.fmap_ten.get("main").copied() {
            st.f.push(Instr::Call(fi, 0), hir.span, 1);
            st.f.push(Instr::Halt, hir.span, 0);
        } else {
            // Giá trị chương trình: interp trả giá trị **câu lệnh cuối nếu là biểu thức**, còn lại `()`.
            match hir.stmts.split_last() {
                Some((last, rest)) => {
                    for s in rest {
                        st.cau(s)?;
                    }
                    if let HStmt::Expr(e) = last {
                        // ST-276: câu Expr cuối đi thẳng `expr()` (không qua `cau()`) ⇒ charge
                        // E_STMT tại đây để khớp interp.
                        st.f.charge(FeeKind::Stmt, e.span());
                        st.expr(e)?;
                        st.f.push(Instr::Return, e.span(), 0);
                    } else {
                        st.cau(last)?;
                        st.f.push(Instr::Halt, hir.span, 0);
                    }
                }
                None => st.f.push(Instr::Halt, hir.span, 0),
            }
        }
        let (done, fee) = st.f.finish()?;
        st.g.funcs[main_idx as usize] = done;
        st.g.fees[main_idx as usize] = fee;
    }
    // Closure đã gom trong lúc phát.
    while let Some(c) = g.cho.pop() {
        let mut st = Emit {
            g: &mut g,
            f: F::new(c.name.clone()),
            loops: Vec::new(),
            captured: HashMap::new(),
            ret_depth: c.ret_depth,
        };
        st.f.arity = c.params.iter().filter(|p| !p.named).count() as u16;
        st.f.nparams = c.params.len() as u16;
        // **Tham số trước** (ô 0..n — VM đổ đối số vào đầu khung), biến bắt giữ ở `captured`.
        for p in &c.params {
            let s = st.f.nlocals;
            st.f.nlocals += 1;
            st.f.locals.insert(p.sym, s);
        }
        // **ST-60b (hồi quy `dd04afb`)**: biến bắt **theo giá trị** ⇒ cấp **ô cục bộ riêng** cho
        // từng biến bắt và **copy** ở đầu thân (`LoadCaptured(i)` + `Assign(slot)`). Nhờ vậy **đọc
        // và ghi** trong closure đều đi ô cục bộ: ghi **không** đụng biến ngoài, đọc lại **thấy** giá
        // trị vừa ghi (đúng interp) — không cần tag `VBC1` mới, verifier/cache không đổi.
        for (i, (sym, _s)) in c.captured.iter().enumerate() {
            let slot = st.f.nlocals;
            st.f.nlocals += 1;
            st.f.locals.insert(*sym, slot);
            st.f.push(Instr::LoadCaptured(i as u16), Span::new(0, 0), 1);
            st.f.push(Instr::Assign(slot), Span::new(0, 0), -1);
        }
        let base = c.params.len() as u16;
        st.than(&c.body, base, Span::new(0, 0))?;
        st.f.push(Instr::Return, Span::new(0, 0), 0);
        let (done, fee) = st.f.finish()?;
        let i = c.idx as usize;
        if i < st.g.funcs.len() {
            st.g.funcs[i] = done;
            st.g.fees[i] = fee;
        }
    }
    let module = Module {
        version: crate::format::VERSION,
        source_hash: g.hash,
        consts: g.consts,
        funcs: g.funcs,
        entry: main_idx,
    };
    if module.funcs.is_empty() {
        return Err(CodegenError::NoEntry);
    }
    g.fees.resize(module.funcs.len(), Vec::new());
    Ok((module, FeeTable { funcs: g.fees }))
}

struct Emit<'g> {
    g: &'g mut Gen,
    f: F,
    loops: Vec<Vong>,
    captured: HashMap<u32, u16>,
    /// **VD-228/VD-241 §3**: số lớp `Opt` của annot return (0 = không nâng).
    ret_depth: usize,
}

impl<'g> Emit<'g> {
    // ----- câu lệnh -----

    fn cau(&mut self, s: &HStmt) -> Result<(), CodegenError> {
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
                        vis_types::AssignOp::Set => {
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
                // **ST-56b (VD-97)**: gán **trường/phần tử** chỉ hợp lệ khi đích gốc là biến
                // `var` (tyck đã chặn trước); gán ghép trên trường/phần tử **chưa** hỗ trợ.
                if !matches!(op, vis_types::AssignOp::Set) {
                    return Err(CodegenError::Unsupported("assign-op", *span));
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
                // `x.push(v)` — interp ghi **trả lại** ô (CoW, VD-51): `x = List([...])`.
                if let HExpr::Call {
                    callee,
                    args,
                    named,
                    span,
                } = e
                {
                    if let HExpr::Field { base, name, .. } = &**callee {
                        if name == "push" {
                            if let HExpr::Var { sym: Some(id), .. } = &**base {
                                let slot = self.slot_cua(*id)?;
                                self.f.push(Instr::LoadLocal(slot), *span, 1);
                                for a in args {
                                    self.expr(a)?;
                                }
                                let c = self.g.cst_str(name);
                                self.f.push(
                                    Instr::CallMethod(c, args.len() as u16),
                                    *span,
                                    -i32::from(args.len() as u16),
                                );
                                let _ = named;
                                self.f.push(Instr::Assign(slot), *span, -1);
                                return self.net_stack(*span);
                            }
                        }
                    }
                }
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

    fn slot_cua(&mut self, sym: u32) -> Result<u16, CodegenError> {
        // **ST-238 (VD-242, ca 3)**: biến cấp tệp dùng ô chung (VM ánh xạ tới `Vm::globals`).
        if let Some(s) = self.g.globals.get(&sym).copied() {
            return Ok(s);
        }
        if let Some(s) = self.captured.get(&sym) {
            return Ok(*s);
        }
        self.f.slot(sym, Span::new(0, 0))
    }

    fn net_stack(&mut self, _span: Span) -> Result<(), CodegenError> {
        Ok(())
    }

    /// **ST-238 (VD-242, ca 7)**: phát một lời gọi builtin giả mà VM chặn lại và trap `VS0501`
    /// với đúng thông điệp của interp. `keep_value = true` (ngữ cảnh giá trị) thì để lại một giá trị
    /// trên ngăn xếp cho verifier — các nhánh khác cũng để lại một giá trị.
    fn trap_no_arm(&mut self, span: Span, keep_value: bool) -> Result<(), CodegenError> {
        let n = self.g.cst_str(crate::format::BI_MATCH_NO_ARM);
        self.f.push(Instr::CallBuiltin(n, 0), span, 1);
        if !keep_value {
            self.f.push(Instr::Pop, span, -1);
        }
        Ok(())
    }

    /// Khối câu lệnh: giá trị của `HStmt::Expr` cuối **vẫn** nằm trên ngăn xếp (nếu khối "có giá trị").
    fn khoi(&mut self, body: &[HStmt]) -> Result<(), CodegenError> {
        for s in body {
            self.cau(s)?;
        }
        Ok(())
    }

    /// Điều kiện: phát biểu thức rồi trả **chỉ số lệnh nhảy-khi-sai** cần vá.
    fn cond(&mut self, c: &HCond) -> Result<Option<usize>, CodegenError> {
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

    // ----- biểu thức -----

    fn expr(&mut self, e: &HExpr) -> Result<(), CodegenError> {
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
                        return self.expr(&v);
                    }
                }
                match sym {
                    Some(id) if self.g.fmap.contains_key(id) => {
                        // Tên hàm dùng như **giá trị** ⇒ closure 0 biến bắt.
                        let fi = self.g.fmap[id];
                        self.f.push(Instr::MakeClosure(fi, 0), *span, 1);
                    }
                    Some(id) => {
                        if let Some(s) = self.g.globals.get(id).copied() {
                            self.f.push(Instr::LoadLocal(s), *span, 1);
                        } else if let Some(s) = self.captured.get(id) {
                            let s = *s;
                            self.f.push(Instr::LoadCaptured(s), *span, 1);
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
                use vis_types::BinOp;
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
                        self.f.push(Instr::Un(vis_types::UnOp::Not), *span, 0);
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
                // **ST-276**: call site có sự kiện riêng, tính **trước** đối số (thứ tự ngôn ngữ).
                let ten_goi = match &**callee {
                    HExpr::Var { name, .. } => Some(name.as_str()),
                    _ => None,
                };
                let la_host = ten_goi
                    .map(|t| !self.g.fmap_ten.contains_key(t) && la_builtin(t))
                    .unwrap_or(false);
                // Interp đánh giá callee builtin như một giá trị ⇒ có E_EXPR riêng; codegen
                // phát thẳng theo tên nên phải charge thêm ở đây để cùng dãy sự kiện.
                self.f.charge(
                    if la_host {
                        FeeKind::Builtin
                    } else {
                        FeeKind::Call
                    },
                    *span,
                );
                if la_host {
                    self.f.charge(FeeKind::Expr, callee.span());
                }
                match &**callee {
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
                        self.expr(base)?;
                        for a in args {
                            self.expr(a)?;
                        }
                        let c = self.g.cst_str(name);
                        self.f.push(Instr::CallMethod(c, n), *span, -i32::from(n));
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
                    // `x?.y`: `None` ⇒ `None`; `Some(v)` ⇒ `Some(v.y)`.
                    self.expr(base)?;
                    self.f.push(Instr::Dup, *span, 1);
                    let jn = self.f.here();
                    self.f.push(Instr::JumpIfNone(0), *span, -1);
                    self.f.push(Instr::UnwrapOpt, *span, 0);
                    let c = self.g.cst_str(name);
                    self.f.push(Instr::Field(c), *span, 0);
                    // **VD-241 §3/VD-227** — `?.f` tới đủ `Opt` không bọc chồng field `B?`.
                    self.lift_opt(1, *span)?;
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
                path,
                variant,
                args,
                span,
                ..
            } => {
                for a in args {
                    self.expr(a)?;
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
                for (n, v) in fields {
                    let c = self.g.cst_str(n);
                    self.f.push(Instr::Const(c), *span, 1);
                    self.expr(v)?;
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
                {
                    let c = self.unit_cst_x();
                    self.f.push(Instr::Const(c), *span, 1);
                }
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

    fn unit_cst_x(&mut self) -> u32 {
        self.g.cst(LitKind::Bool, "false")
    }
}

/// `binop_tag` của lệnh `Bin` — tiện cho test.
pub fn bin_tag(op: vis_types::BinOp) -> u8 {
    binop_tag(op)
}

/// `unop_tag` của lệnh `Un` — tiện cho test.
pub fn un_tag(op: vis_types::UnOp) -> u8 {
    unop_tag(op)
}
