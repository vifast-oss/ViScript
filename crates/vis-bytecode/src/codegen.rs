// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ **HIR** (`vis-types`) xuống [`Module`] của VBC1 (ST-19).
//!
//! Nguyên tắc:
//! - **Không** chép ngữ nghĩa toán tử: `Bin`/`Un` mang **chính** `vis_hir::BinOp`/`UnOp` — VM
//!   (ST-20) sẽ dùng chung bảng ngữ nghĩa với `vis-interp`.
//! - `match` hạ thành **chuỗi test + nhảy** (VM không cần biết pattern).
//! - Closure **chụp snapshot** cả khung hiện tại (VD-51): `MakeClosure(fidx, nfree)` với
//!   `nfree = số ô cục bộ của hàm cha tại điểm tạo`; trong closure đọc bằng `LoadCaptured`.
//! - `max_stack`: đếm **bảo thủ** trong lúc phát (upper bound ≥ độ sâu thật). **ST-21 kiểm lại
//!   độc lập từ byte stream** — không tin giá trị này.

mod builtin_value;
mod cau;
mod expr;
mod gan_long;
mod gia_tri;
mod if_expr;
mod pattern;
mod than;

use std::collections::HashMap;

use vis_hir::MethodRet;
use vis_hir::{HBody, HCond, HExpr, HFn, HItem, HPat, HStmt, Hir, InterpPart, LitKind, Symbols};
use vis_syntax::Span;

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
    sym_ty: HashMap<u32, vis_hir::Type>,
    fn_rets: HashMap<u32, vis_hir::Type>, // VD-260: sym hàm → kiểu trả về (lớp Option)
    rec_fields: HashMap<(u32, String), vis_hir::Type>, // ST-248: kiểu trường bản ghi
    /// **VD-258**: kiểu trường biến thể enum `(EnumId, biến thể, vị trí)`.
    enum_fields: HashMap<(u32, String, usize), vis_hir::Type>,
    mret: vis_hir::MethodRet, // VD-265: kiểu trả phương thức người dùng (theo kiểu nhận + tên)
    /// **ST-60**: chỉ số hàm của các `async fn` — call-site async hạ thành `$tac_vu`.
    async_fns: std::collections::HashSet<u32>,
    /// **ST-238 (VD-242, ca 3)**: `sym` của biến `let`/`var` cấp tệp → ô trong vùng
    /// [`crate::format::GLOBAL_SLOT_BASE`]. Mọi hàm đọc/ghi qua vùng này dùng chung một kho ở VM.
    globals: HashMap<u32, u16>,
    /// **ST-238 (VD-242, ca 4)**: `sym` của `const` cấp tệp → biểu thức giá trị (inline tại
    /// mỗi chỗ dùng, như interp nạp hằng lúc load).
    hconsts: HashMap<u32, HExpr>,
    /// **ST-314a**: các `const` đang inline — chặn chu trình (trước đây đệ quy vô hạn).
    hconsts_dang: std::collections::HashSet<u32>,
    hash: [u8; HASH_LEN],
}

struct Cho {
    idx: u32,
    params: Vec<vis_hir::HParam>,
    body: HBody,
    captured: Vec<(u32, u16)>,
    name: String,
    ret_depth: usize,
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
    compile_kiem_voi_bang(hir, symbols, None, text, chan_doan)
}

/// **VD-263** — như [`compile_kiem`] nhưng nhận thêm [`vis_hir::TypeTable`] để codegen thấy trường của
/// **instantiation** generic (`Box[int?]`) khi nâng `Option` ở `?.`/`with`/gán trường.
pub fn compile_kiem_voi_bang(
    hir: &Hir,
    symbols: &Symbols,
    types: Option<&vis_hir::TypeTable>,
    text: &str,
    chan_doan: &[vis_syntax::Diagnostic],
) -> Result<Module, CodegenError> {
    compile_kiem_voi_bang_co_phi(hir, symbols, types, text, chan_doan).map(|(m, _)| m)
}

/// **ST-290 (VD-273)** — như [`compile_kiem_voi_bang`] nhưng trả kèm **bảng phí** để runtime
/// bật ngân sách mặc định. Cùng cổng chẩn đoán front end (chỉ `Severity::Error` chặn).
pub fn compile_kiem_voi_bang_co_phi(
    hir: &Hir,
    symbols: &Symbols,
    types: Option<&vis_hir::TypeTable>,
    text: &str,
    chan_doan: &[vis_syntax::Diagnostic],
) -> Result<(Module, FeeTable), CodegenError> {
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
    compile_with_fee_voi_bang(hir, symbols, types, text)
}

/// Hạ một chương trình HIR xuống module VBC1 (API cũ — **không** kèm chẩn đoán; xem
/// [`compile_kiem`] cho đường có cổng front end).
pub fn compile(hir: &Hir, symbols: &Symbols, text: &str) -> Result<Module, CodegenError> {
    compile_with_fee(hir, symbols, text).map(|(m, _)| m)
}

/// **VD-263** — như [`compile`] nhưng có [`vis_hir::TypeTable`] (instantiation generic).
pub fn compile_voi_bang(
    hir: &Hir,
    symbols: &Symbols,
    types: Option<&vis_hir::TypeTable>,
    text: &str,
) -> Result<Module, CodegenError> {
    compile_with_fee_voi_bang(hir, symbols, types, text).map(|(m, _)| m)
}

/// **ST-276 (VD-256)** — như [`compile`] nhưng trả kèm **bảng phụ S** (sự kiện ngữ nghĩa theo
/// PC). Đường mặc định (`compile`) **không** dùng bảng này; chỉ test-hook ngân sách đóng trailer.
pub fn compile_with_fee(
    hir: &Hir,
    symbols: &Symbols,
    text: &str,
) -> Result<(Module, FeeTable), CodegenError> {
    compile_with_fee_voi_bang(hir, symbols, None, text)
}

/// **VD-263** — như [`compile_with_fee`] nhưng nhận [`vis_hir::TypeTable`] (instantiation generic).
pub fn compile_with_fee_voi_bang(
    hir: &Hir,
    symbols: &Symbols,
    types: Option<&vis_hir::TypeTable>,
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
        hconsts_dang: std::collections::HashSet::new(),
        builtins: crate::util::tinh_cac_builtin(symbols),
        // **VD-263**: `types` bổ sung trường của mọi instantiation generic vào bảng kiểu tĩnh.
        sym_ty: than::collect_sym_tys(hir, symbols, types),
        fn_rets: than::collect_fn_rets(hir), // VD-260: kiểu trả về của hàm
        rec_fields: than::collect_rec_fields(hir, types),
        enum_fields: than::collect_enum_fields(hir, types),
        mret: than::collect_mret(hir, types),
        hash: crate::format::source_hash(text),
    };
    // **VD-263**: `MakeRecord` in tên của cả bản ghi **instantiation** (`Box[int?]` → `Box`).
    if let Some(types) = types {
        for (i, r) in types.recs().iter().enumerate() {
            if let Some(n) = &r.name {
                g.recs.entry(i as u32).or_insert_with(|| n.clone());
            }
        }
    }
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
                    crate::util::pat_syms(pat, &mut syms);
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
            // **VD-262**: khai báo cấp tệp thành global, phát trước `Call(main)`.
            st.khai_bao_cap_tep(hir)?;
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
        // **VD-283/ST-301**: nâng `T → …T?…` cho tham số closure có annotation, mirror interp.
        st.lift_params_opt(&c.params, Span::new(0, 0))?;
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

/// `binop_tag` của lệnh `Bin` — tiện cho test.
pub fn bin_tag(op: vis_hir::BinOp) -> u8 {
    binop_tag(op)
}

/// `unop_tag` của lệnh `Un` — tiện cho test.
pub fn un_tag(op: vis_hir::UnOp) -> u8 {
    unop_tag(op)
}
