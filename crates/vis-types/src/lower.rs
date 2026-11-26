// SPDX-License-Identifier: MIT OR Apache-2.0
//! Hạ **CST → HIR** + giải tên cho chế độ script (ST-11).
//!
//! # Hợp đồng (VD-44)
//!
//! 1. **Permissive ở tầng hạ**: biểu diễn được **mọi** thứ trong Phụ lục A ở mức cú pháp —
//!    `float`, `cap[Dir]`, `ref(0)`, `async fn`, `fn f[T](…)`, `derives`, `scope s`,
//!    `cmd { … }`, `await`… Hạ **không** báo "tính năng chưa hỗ trợ"; giới hạn VS0 chỉ báo ở
//!    tầng kiểm kiểu (ST-12). 15 khối ```vis đủ thân của tài liệu phải hạ **0 chẩn đoán**.
//! 2. **Giải tên chỉ cho tên GIÁ TRỊ** (biến, tham số, hàm, bản ghi/enum đã khai). Khai lại
//!    **cùng khung** ⇒ `VS0302`. Tên **kiểu** chưa phân giải giữ dạng [`Type::TypeRef`] — ST-11
//!    **không** báo `VS0301` cho kiểu (ST-12 làm).
//! 3. **Không prelude**: đúng 8 tên giá trị dựng sẵn [`BUILTIN_VALUE_NAMES`] (khoá bằng test).
//! 4. `VS0301` (**tên giá trị chưa khai**): mặc định [`NamePolicy::Permissive`] (VD-45) **chỉ**
//!    báo khi **gán** cho tên chưa khai — chỗ chắc chắn sai; [`NamePolicy::Strict`] báo cho **mọi**
//!    lần dùng chưa phân giải và sẽ là mặc định từ ST-13/14. Lý do: 30 golden biểu thức và nhiều
//!    khối tài liệu dùng tên tự do (`a`, `b`, `xs`, `user`, `json`…) — xem `JOURNAL` ST-11.
//! 5. **Không panic** trên dữ liệu người dùng (CST của tệp parse lỗi, 7 khối loại trừ, rác):
//!    thiếu node ⇒ [`HExpr::Error`]/bỏ qua, không `unwrap`/`todo!`/`unimplemented!`.
//! 6. `Mode::Command` **không** hạ bằng `lower` (VD-22 — `cmd_lower` là VS1): trả
//!    [`LowerError::CommandMode`] tường minh trong [`HirOutput::error`], không panic.

use vis_diag::NAME_CODES;
use vis_syntax::{Cst, Diagnostic, Elem, Kind, LexOutput, Node, NodeKind, Span, Token};

use vis_hir::cmdsig::SigSource;
use vis_hir::hir::{
    BinOp, FnKind, HAlias, HConst, HEnum, HExpr, HField, HFn, HImpl, HItem, HParam, HRec, HSig,
    HStmt, HTrait, HUse, HVariant, Hir, LitKind, UnOp,
};

mod generic;
mod trait_impl;
pub use trait_impl::TraitSig;
pub(crate) use vis_hir::symbols::{EnumTmpl, HFrame, HSym, ImplDangKy, SymKind, Symbols};
use vis_hir::ty::{EnumDef, EnumId, FieldDef, RecDef, RecId, Type, TypeTable, VariantDef};
pub(crate) use vis_hir::FrameKind;

mod chieu;
mod cmd;
mod co_so;
mod expr;
mod khai;
mod khai_bao;
mod kieu;
mod pat;
mod quyen;
mod sig;
mod stmt;
mod sugar;
mod than;
pub use than::is_builtin_value;
pub(crate) use than::{lower_nhom, lower_than, ThanCtx};

/// **VD-267**: thêm `panic` (builtin phân kỳ, `panic(str) -> never`) ⇒ **12** tên.
/// Danh sách **khoá bằng test** (VD-44 #3: thêm/bớt phải qua Agent 0). `true`/`false` là
/// **literal**; `none` đã **bỏ** (R2, `VS0203`).
pub const BUILTIN_VALUE_NAMES: [&str; 12] = [
    "print", "len", "str", "echo", "lines", "take", "Ok", "Err", "Some", "None", "ref", "panic",
];

mod options;
pub use options::{LowerOptions, NamePolicy};

/// Lỗi **cấp API** (không phải chẩn đoán ngôn ngữ): đầu vào ngoài phạm vi `lower`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowerError {
    /// CST sinh từ `parse(.., Mode::Command)` — cần `cmd_lower` (VD-22, VS1).
    CommandMode,
}

impl LowerError {
    /// Thông điệp ổn định cho người dùng/log.
    pub const fn message(self) -> &'static str {
        match self {
            LowerError::CommandMode => {
                "chế độ lệnh (.vish) cần `cmd_lower` (VD-22, VS1) — `lower` chỉ hạ chế độ script"
            }
        }
    }
}

/// Kết quả một lượt hạ.
#[derive(Debug, Clone)]
pub struct HirOutput {
    pub hir: Hir,
    /// Chẩn đoán **ngôn ngữ** (`VS03xx`) — không chứa lỗi cấp API.
    pub diagnostics: Vec<Diagnostic>,
    pub symbols: Symbols,
    pub types: TypeTable,
    /// Lỗi cấp API (vd hạ CST chế độ lệnh); `None` khi hạ bình thường.
    pub error: Option<LowerError>,
}

impl HirOutput {
    /// `true` nếu không có chẩn đoán và không có lỗi cấp API.
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty() && self.error.is_none()
    }

    /// Mã chẩn đoán theo thứ tự (tiện cho log/test).
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code.short).collect()
    }
}

/// Hạ một CST chế độ script với tuỳ chọn mặc định ([`LowerOptions::default`]).
pub fn lower(cst: &Cst, lex: &LexOutput) -> HirOutput {
    lower_with(cst, lex, LowerOptions::default())
}

/// Hạ một CST chế độ script với tuỳ chọn cho trước (bảng ký hiệu/kiểu **mới**).
pub fn lower_with(cst: &Cst, lex: &LexOutput, opts: LowerOptions) -> HirOutput {
    let mut symbols = Symbols::default();
    let mut types = TypeTable::new();
    lower_into(&mut symbols, &mut types, cst, lex, opts)
}

/// Hạ với **nguồn chữ ký lệnh** tự chọn (ST-37): `vis-std`/prelude (ST-40/41) cắm `SigSource`
/// riêng mà **không** phải sửa `lower`/`tyck`. Nguồn mặc định là 3 builtin (`BuiltinSigs`).
pub fn lower_with_sigs(
    cst: &Cst,
    lex: &LexOutput,
    opts: LowerOptions,
    src: &dyn SigSource,
) -> HirOutput {
    let mut symbols = Symbols::default();
    let mut types = TypeTable::new();
    let mut l = Lowerer::new(cst, lex, opts, &mut symbols, &mut types);
    l.cmd_sigs = src.sigs().to_vec();
    l.run();
    l.finish()
}

/// Hạ một tệp **vào bảng chung** (ST-25): ký hiệu và kiểu được **append** ⇒ id ổn định giữa các tệp.
///
/// `symbols.mods` do nơi gọi khai bằng [`Symbols::add_mod`] trước khi hạ tệp tương ứng; tên item
/// cấp tệp của tệp mới bắt đầu từ `symbols.syms.len()`.
pub fn lower_into(
    symbols: &mut Symbols,
    types: &mut TypeTable,
    cst: &Cst,
    lex: &LexOutput,
    opts: LowerOptions,
) -> HirOutput {
    let mut l = Lowerer::new(cst, lex, opts, symbols, types);
    l.run();
    l.finish()
}

struct Lowerer<'a> {
    /// **ST-114a**: chỉ số tệp đang hạ (0 = một tệp).
    file: u16,
    cst: &'a Cst,
    text: &'a str,
    opts: LowerOptions,
    diags: Vec<Diagnostic>,
    error: Option<LowerError>,
    symbols: &'a mut Symbols,
    /// Bảng kiểu **chung** của đơn vị biên dịch (ST-25: id ổn định, append theo tệp).
    tbl: &'a mut TypeTable,
    /// Chồng khung hiện tại; đáy luôn là khung tệp `0`.
    scope: Vec<u32>,
    items: Vec<HItem>,
    stmts: Vec<HStmt>,
    /// Tham số kiểu đang có hiệu lực (`type Hop[T]`, `enum Or[A, B]`) — ST-28: xoá kiểu.
    generic_scope: Vec<String>,
    impls: Vec<(Type, String, u32)>,
    /// ST-29: trait nội tệp để kiểm `impl` khớp trait; **ST-57**: kèm **siêu trait**.
    traits: Vec<TraitSig>,
    /// **ST-57**: trait `pub` nhập từ module khác (mồ côi đa tệp + siêu trait).
    traits_ngoai: Vec<TraitSig>,
    /// **ST-57**: `(kiểu, trait)` của mọi `impl Trait for Type` trong tệp (kiểm siêu trait **một tầng**).
    impl_pairs: Vec<(Type, String, Span)>,
    /// **ST-57**: tên kiểu **nhập** từ module khác — mồ côi **thật** = trait nhập *và* kiểu nhập.
    kieu_ngoai: Vec<String>,
    /// **ST-57**: `impl` **sinh** từ `derives` (nối cuối `items` ở cuối `run()`).
    derived: Vec<HItem>,
    /// ST-29: kiểu tĩnh của biến (chỉ từ **chú thích** tường minh: tham số + `let`/`var`).
    type_env: std::collections::HashMap<u32, Type>,
    /// ST-29: tên kiểu của `impl` đang hạ (để tái dùng sym đã khai ở lượt thu thập).
    cur_impl: Option<String>,
    /// **VD-265**: kiểu của `impl` đang hạ — chỉ dùng cho [`Lowerer::static_ty`] của `self`
    /// (dispatch `self.m()` và `self.f` **field** vẫn giữ `{error}` như VD-262 để không đổi
    /// chẩn đoán runtime hiện có).
    cur_impl_ty: Option<Type>,
    /// **VD-270** — tham số kiểu của `impl` đang hạ (`impl Hop[T]:`) để **hợp nhất** vào
    /// `generics` của từng phương thức: chữ ký giữ `Hop[T]`/`T` dạng ký hiệu, thân xoá như
    /// hàm generic. Rỗng với `impl` không generic.
    cur_impl_generics: Vec<String>,
    /// **VD-265**: `sym` tham số `self` của các phương thức `impl` → kiểu nhận. `static_ty`
    /// tra bảng này để `self.m()` hạ thành `Call(Var(sym))` như `p.m()` với `p` có annot.
    self_ty: std::collections::HashMap<u32, Type>,
    /// ST-35: **template** trường của `type T[..]` — kiểu còn **ký hiệu** `TypeRef{"T"}` để thay khi
    /// instantiation (`Hop[int]`), thay vì xoá kiểu như ST-28.
    rec_tmpl: Vec<(String, Vec<String>, Vec<Type>)>,
    /// ST-35: template biến thể của `enum E[..]` — `(tên enum, tham số, [(biến thể, kiểu trường)])`.
    enum_tmpl: Vec<EnumTmpl>,
    /// ST-35: cache instantiation `(tên gốc, đối số) → id`.
    inst_cache: Vec<((String, Vec<Type>), u32)>,
    /// ST-35: `true` khi đang dựng **template** ⇒ tham số kiểu giữ dạng `TypeRef{"T"}`.
    sym_generic: bool,
    /// ST-37: chữ ký lệnh chế độ shell (builtin + `cmd`) — **một** nguồn cho khe ống và kiểm gọi.
    cmd_sigs: Vec<vis_hir::cmdsig::CmdSig>,
    /// ST-38: bộ đếm tên tạm cho **sugar `it`** (tránh trùng `__c` giữa nhiều vị từ trong tệp).
    tam_n: u32,
    /// ST-46: `RecId` của `FileEntryLong` (prelude) — `ls --long` trả `list` kiểu này.
    file_long: Option<RecId>,
    /// ST-41: kiểu (tĩnh) của **stage cuối** vừa hạ — để `boc_render` biết phần tử có phải **bản ghi**
    /// (⇒ bảng / `(rỗng)`) hay không (`list[str]`/`list[int]` rỗng giữ `[]`).
    last_stage_ty: Type,
    /// ST-40a: bản ghi **prelude** của `.vish` (`FileEntry`, `ProcEntry`) — để `kieu_text` nhận literal
    /// bản ghi có đúng bộ trường và cho `it` **kiểu thật**.
    prelude_recs: Vec<(String, RecId)>,
    rec_names: Vec<(String, RecId)>,
    enum_names: Vec<(String, EnumId)>,
    /// Tên biến thể trần → (enum, chỉ số biến thể).
    variant_index: Vec<(String, EnumId, usize)>,
    aliases: Vec<(String, Type)>,
    /// **R12a (VD-178)**: tên `type Email(str)` — newtype **trong suốt** (dựng/mở = chính giá trị).
    newtypes: Vec<String>,
    /// Tên item cấp tệp → chỉ số ký hiệu (khai ở lượt 1).
    ///
    /// **ST-316b (`#199`)**: `HashMap` thay `Vec<(String, u32)>` + quét ngược — tra `item_sym`
    /// `O(1)`, bản ghi **sau** ghi đè bản trước (đúng "lần khai cuối" như cũ).
    item_syms: std::collections::HashMap<String, u32>,
    /// Đang hạ vế sau của or-pattern: khai tên đã có thì **dùng lại**, không báo `VS0302`.
    suppress_dup: bool,
    /// Đang hạ **đích gán**: hoãn `VS0301` của [`Lowerer::lower_var_use`] để `lower_assign` báo
    /// **một** lần ở gốc lvalue (tránh 2 chẩn đoán cho cùng một `x = 1`).
    suppress_name_diag: bool,
    /// **VD-263**: kiểu **mong đợi** của biểu thức đang hạ (từ annot `let`/`var`, `Some(...)`,
    /// biến thể enum…). Chỉ dùng để chọn **instantiation** cho constructor/record literal —
    /// `None` ⇒ hạ như cũ.
    expect: Option<Type>,
    /// **VD-263**: kiểu trả về của hàm đang hạ (để `return`/biểu thức cuối mang instantiation).
    cur_ret: Option<Type>,
    /// **VD-263**: `sym` hàm → danh sách `(tên, kiểu)` tham số (để đối số call-site mang
    /// instantiation như tham số khai báo). **VD-265**: pre-pass nạp **mọi** chữ ký trước khi hạ
    /// thân ⇒ hàm khai **sau** call-site vẫn có.
    fn_params: std::collections::HashMap<u32, Vec<(String, Type)>>,
    /// **VD-265**: `sym` hàm → kiểu trả về (dựng `fn(…) -> …` cho **giá trị hàm** ở call-site).
    fn_ret: std::collections::HashMap<u32, Type>,
}

fn is_item(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::UseDecl
            | NodeKind::FnDecl
            | NodeKind::CmdDecl
            | NodeKind::ExternDecl
            | NodeKind::TypeDecl
            | NodeKind::EnumDecl
            | NodeKind::AliasDecl
            | NodeKind::NewtypeDecl
            | NodeKind::TraitDecl
            | NodeKind::ImplDecl
            | NodeKind::ConstDecl
            | NodeKind::TestDecl
    )
}

/// Loại literal theo kind token (SPEC-lex §1).
pub(crate) fn lit_kind(kind: Kind) -> Option<LitKind> {
    Some(match kind {
        Kind::Int => LitKind::Int,
        Kind::Float => LitKind::Float,
        Kind::Size => LitKind::Size,
        Kind::Duration => LitKind::Duration,
        Kind::Percent => LitKind::Percent,
        Kind::Char => LitKind::Char,
        Kind::Str | Kind::RawStr | Kind::MultiStr | Kind::RawMulti => LitKind::Str,
        Kind::Bytes => LitKind::Bytes,
        Kind::Path => LitKind::Path,
        _ => return None,
    })
}

/// Toán tử nhị phân từ chữ viết token.
pub(crate) fn bin_op(text: &str) -> Option<BinOp> {
    BinOp::parse(text)
}

/// Toán tử một ngôi từ chữ viết token.
pub(crate) fn un_op(text: &str) -> Option<UnOp> {
    match text {
        "-" => Some(UnOp::Neg),
        "not" => Some(UnOp::Not),
        _ => None,
    }
}
