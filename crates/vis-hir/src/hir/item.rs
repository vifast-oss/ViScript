// SPDX-License-Identifier: MIT OR Apache-2.0
//! Khai báo cấp tệp của HIR: [`Hir`], [`HItem`] + struct item (hàm, bản ghi, enum, alias,
//! `const`, `use`, `trait`, `impl`). Tách nguyên văn từ `hir.rs` (ST-318d).

use vis_syntax::Span;

use crate::ty::{EnumId, RecId, Type};

use super::expr::HExpr;
use super::stmt::HStmt;

/// Chương trình đã hạ: **item** (khai báo cấp tệp, được "treo" lên trước — §14.3 "khai báo
/// trước, thân sau") và **câu lệnh cấp tệp** theo đúng thứ tự nguồn.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Hir {
    pub items: Vec<HItem>,
    pub stmts: Vec<HStmt>,
    pub span: Span,
}

/// Khai báo cấp tệp.
#[derive(Debug, Clone, PartialEq)]
pub enum HItem {
    Fn(HFn),
    Rec(HRec),
    Enum(HEnum),
    Alias(HAlias),
    Extern(HSig),
    Const(HConst),
    Use(HUse),
    /// `trait` — khung **VS1** (ST-11 chỉ giữ chữ ký, chưa kiểm).
    Trait(HTrait),
    /// `impl` — khung **VS1**.
    Impl(HImpl),
}

impl HItem {
    /// Tên khai báo (nếu có).
    pub fn name(&self) -> Option<&str> {
        match self {
            HItem::Fn(f) => Some(&f.name),
            HItem::Rec(r) => Some(&r.name),
            HItem::Enum(e) => Some(&e.name),
            HItem::Alias(a) => Some(&a.name),
            HItem::Extern(x) => Some(&x.name),
            HItem::Const(c) => Some(&c.name),
            HItem::Trait(t) => Some(&t.name),
            HItem::Impl(_) | HItem::Use(_) => None,
        }
    }

    /// Mã (chỉ số) trong bảng ký hiệu.
    pub fn sym(&self) -> Option<u32> {
        match self {
            HItem::Fn(f) => Some(f.sym),
            HItem::Rec(r) => Some(r.sym),
            HItem::Enum(e) => Some(e.sym),
            HItem::Alias(a) => Some(a.sym),
            HItem::Extern(x) => Some(x.sym),
            HItem::Const(c) => Some(c.sym),
            HItem::Trait(t) => Some(t.sym),
            HItem::Impl(_) | HItem::Use(_) => None,
        }
    }

    /// Span của khai báo.
    pub fn span(&self) -> Span {
        match self {
            HItem::Fn(f) => f.span,
            HItem::Rec(r) => r.span,
            HItem::Enum(e) => e.span,
            HItem::Alias(a) => a.span,
            HItem::Extern(x) => x.span,
            HItem::Const(c) => c.span,
            HItem::Use(u) => u.span,
            HItem::Trait(t) => t.span,
            HItem::Impl(i) => i.span,
        }
    }

    /// Sexp để test/log.
    pub fn sexp(&self) -> String {
        match self {
            HItem::Fn(f) => f.sexp(),
            HItem::Rec(r) => format!("(rec {})", r.name),
            HItem::Enum(e) => format!("(enum {})", e.name),
            HItem::Alias(a) if a.is_newtype => format!("(newtype {} = {})", a.name, a.ty),
            HItem::Alias(a) => format!("(alias {} = {})", a.name, a.ty),
            HItem::Extern(x) => format!("(extern {})", x.name),
            HItem::Const(c) => format!("(const {} = {})", c.name, c.value.sexp()),
            HItem::Use(u) => format!("(use {})", u.path.join(".")),
            HItem::Trait(t) => format!("(trait {})", t.name),
            HItem::Impl(i) => {
                let head = match &i.trait_name {
                    Some(t) => format!("{t} for {}", i.ty.name()),
                    None => i.ty.name(),
                };
                format!("(impl {head})")
            }
        }
    }
}

/// Một hàm/phương thức/khai báo `cmd`/`test fn`.
#[derive(Debug, Clone, PartialEq)]
pub struct HFn {
    pub name: String,
    pub span: Span,
    pub sym: u32,
    pub kind: FnKind,
    pub is_pub: bool,
    pub is_async: bool,
    /// Tên tham số kiểu (`fn first[T](…)`); **chưa** kiểm (generic là VS1).
    pub generics: Vec<String>,
    pub params: Vec<HParam>,
    /// Kiểu trả về; không ghi ⇒ `()`.
    pub ret: Type,
    pub body: Vec<HStmt>,
}

impl HFn {
    fn sexp(&self) -> String {
        let params: Vec<String> = self.params.iter().map(|p| p.name.clone()).collect();
        format!("(fn {}({}) -> {})", self.name, params.join(" "), self.ret)
    }
}

/// Loại hàm — dùng chung một `HFn` cho cả 4 dạng khai báo (ít trùng lặp, dễ duyệt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnKind {
    Fn,
    Cmd,
    Test,
    TraitMethod,
    ImplMethod,
}

/// Tham số hình thức (vị trí, `--tên`, `self`, `var self`).
#[derive(Debug, Clone, PartialEq)]
pub struct HParam {
    pub name: String,
    pub span: Span,
    pub sym: u32,
    pub ty: Type,
    /// `--ten` — chỉ truyền theo tên / là cờ ở chế độ lệnh.
    pub named: bool,
    pub is_self: bool,
    pub is_var_self: bool,
    pub default: Option<HExpr>,
    /// `...rest: str` — gom **mọi** đối số vị trí còn lại thành `list[str]` (ST-27).
    pub variadic: bool,
}

/// `extern cmd name(params)` — chưa có thân.
#[derive(Debug, Clone, PartialEq)]
pub struct HSig {
    pub name: String,
    pub span: Span,
    pub sym: u32,
    pub params: Vec<HParam>,
}

/// Trường của bản ghi/enum (đã hạ kiểu + giá trị mặc định).
#[derive(Debug, Clone, PartialEq)]
pub struct HField {
    pub name: String,
    pub span: Span,
    pub ty: Type,
    pub default: Option<HExpr>,
    pub has_default: bool,
}

/// `type Name derives …: field+`.
#[derive(Debug, Clone, PartialEq)]
pub struct HRec {
    pub id: RecId,
    pub name: String,
    pub span: Span,
    pub sym: u32,
    /// **ST-316b (`#48`)** — có `pub` không (LSP chỉ gợi ý item `pub` của module khác).
    pub is_pub: bool,
    /// Tham số kiểu (`type Hop[T]`) — **xoá kiểu** ở VS0 (ST-28), chưa instantiation thật.
    pub generics: Vec<String>,
    pub fields: Vec<HField>,
    pub derives: Vec<String>,
}

/// `enum Name derives …: variant+`.
#[derive(Debug, Clone, PartialEq)]
pub struct HEnum {
    pub id: EnumId,
    pub name: String,
    pub span: Span,
    pub sym: u32,
    /// **ST-316b (`#48`)** — có `pub` không (LSP chỉ gợi ý item `pub` của module khác).
    pub is_pub: bool,
    /// Tham số kiểu (`enum Or[A, B]`) — xoá kiểu ở VS0 (ST-28).
    pub generics: Vec<String>,
    pub variants: Vec<HVariant>,
    pub derives: Vec<String>,
}

/// Một biến thể enum.
#[derive(Debug, Clone, PartialEq)]
pub struct HVariant {
    pub name: String,
    pub span: Span,
    pub fields: Vec<HField>,
}

/// `alias Name = type` (`newtype` cùng cấu trúc, `is_newtype = true`).
#[derive(Debug, Clone, PartialEq)]
pub struct HAlias {
    pub name: String,
    pub span: Span,
    pub sym: u32,
    /// **ST-316b (`#48`)** — có `pub` không (LSP chỉ gợi ý item `pub` của module khác).
    pub is_pub: bool,
    pub ty: Type,
    pub is_newtype: bool,
}

/// `const Name: type = expr`.
#[derive(Debug, Clone, PartialEq)]
pub struct HConst {
    pub name: String,
    pub span: Span,
    pub sym: u32,
    /// **ST-316b (`#48`)** — có `pub` không (LSP chỉ gợi ý item `pub` của module khác).
    pub is_pub: bool,
    pub ty: Type,
    pub value: HExpr,
}

/// `use a.b.c` (module là VS1 — ST-11 chỉ ghi lại đường dẫn).
#[derive(Debug, Clone, PartialEq)]
pub struct HUse {
    pub path: Vec<String>,
    pub span: Span,
}

/// Khung `trait` (VS1): chữ ký phương thức, thân có thể rỗng.
#[derive(Debug, Clone, PartialEq)]
pub struct HTrait {
    pub name: String,
    pub span: Span,
    pub sym: u32,
    /// **ST-316b (`#48`)** — có `pub` không (LSP chỉ gợi ý item `pub` của module khác).
    pub is_pub: bool,
    pub supertraits: Vec<String>,
    pub methods: Vec<HFn>,
}

/// Khung `impl` (VS1).
#[derive(Debug, Clone, PartialEq)]
pub struct HImpl {
    pub trait_name: Option<String>,
    pub ty: Type,
    pub span: Span,
    pub methods: Vec<HFn>,
}
