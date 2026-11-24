// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-312c (VD-289)** — bảng ký hiệu + khung tên (dữ liệu thuần).
//!
//! Chuyển **nguyên văn** từ `vis-types/src/lower.rs` để `vis-bytecode`/`vis-vm` đọc thẳng
//! `vis_hir::Symbols` mà không phải kéo cả tầng hạ/kiểm kiểu.

use vis_syntax::Span;

use crate::ty::Type;

/// Loại khung tên (phạm vi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    File,
    Fn,
    Block,
}

/// Một khung tên. `parent` trỏ khung ngoài.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HFrame {
    pub parent: Option<u32>,
    pub span: Span,
    pub kind: FrameKind,
}

/// Loại khai báo trong bảng ký hiệu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymKind {
    /// Tên giá trị dựng sẵn ([`BUILTIN_VALUE_NAMES`]).
    Builtin,
    /// Khai báo cấp tệp (fn/type/enum/alias/const/…) hoặc phương thức.
    Item,
    Param,
    /// `var` (biến đổi được).
    Var,
    PatBind,
    ElseBind,
    /// Tham số kiểu của hàm (`fn f[T]`).
    Generic,
    /// Nhãn `scope s`.
    ScopeBind,
}

/// Một ký hiệu: tên + loại + khung + vị trí (+ kiểu nếu biết).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HSym {
    pub name: String,
    pub kind: SymKind,
    pub frame: u32,
    pub span: Span,
    pub ty: Option<Type>,
}

/// Thông tin **module** (ST-25): một tệp nguồn trong đơn vị biên dịch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModInfo {
    /// Tên hiển thị (tên tệp, không kèm thư mục).
    pub name: String,
    /// Đường dẫn tương đối tới tệp chính (ổn định khi đổi `cwd`).
    pub path: String,
}

/// **VD-270** — một `impl` đã đăng ký ở phạm vi **toàn chương trình** (xuyên module).
///
/// Mỗi tệp vẫn có bảng `impls` **cục bộ** của riêng nó (dùng khi hạ chính tệp đó); bản ghi ở đây
/// chỉ để module **khác** thấy phương thức của `impl` đã hạ ở tệp phụ thuộc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplDangKy {
    /// Kiểu nhận của `impl` (`Type::Record`/`Enum`/`TypeRef` generic).
    pub ty: Type,
    /// Tên phương thức (`fn` trong thân `impl`).
    pub method: String,
    /// Chỉ số `sym` của phương thức trong HIR toàn chương trình.
    pub sym: u32,
    /// Module khai `impl` (chỉ số trong `Symbols::mods`).
    pub mod_id: u16,
}

/// Bảng ký hiệu của một lượt hạ: các khung + ký hiệu theo thứ tự khai báo.
///
/// ST-25 thêm ba mảng **song song** với `syms` để kiểm tầm nhìn `pub`/`use` **ở tầng giải tên**
/// (`mod_of` = module sở hữu, `pub_of` = có `pub` hay không) và `mods` = danh sách module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Symbols {
    pub frames: Vec<HFrame>,
    pub syms: Vec<HSym>,
    /// Module sở hữu từng ký hiệu; `u16::MAX` = dựng sẵn (luôn thấy).
    pub mod_of: Vec<u16>,
    /// Ký hiệu có `pub` hay không (chỉ có nghĩa với item cấp tệp).
    pub pub_of: Vec<bool>,
    /// Các module của đơn vị biên dịch (chỉ số = `mod_of`).
    pub mods: Vec<ModInfo>,
    /// **VD-270** — `impl` của mọi module đã hạ trong đơn vị (dùng cho dispatch xuyên module).
    pub impls: Vec<ImplDangKy>,
    /// **VD-270** — template trường của `type T[..]` đã hạ (để module khác instantiation được).
    pub rec_tmpl: Vec<(String, Vec<String>, Vec<Type>)>,
    /// **VD-270** — template biến thể của `enum E[..]` đã hạ (để module khác instantiation được).
    pub enum_tmpl: Vec<EnumTmpl>,
}

impl Symbols {
    /// Thêm một module, trả id.
    pub fn add_mod(&mut self, m: ModInfo) -> u16 {
        let id = self.mods.len() as u16;
        self.mods.push(m);
        id
    }
}

impl Symbols {
    pub fn get(&self, id: u32) -> Option<&HSym> {
        self.syms.get(id as usize)
    }

    /// Tên của một ký hiệu.
    pub fn name_of(&self, id: u32) -> Option<&str> {
        self.get(id).map(|s| s.name.as_str())
    }

    /// Ký hiệu theo tên trong **một** khung (lần khai báo cuối trong khung đó).
    pub fn in_frame(&self, frame: u32, name: &str) -> Option<&HSym> {
        self.syms
            .iter()
            .rev()
            .find(|s| s.frame == frame && s.name == name)
    }

    /// Mọi ký hiệu mang `name` (theo thứ tự khai báo).
    pub fn all_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a HSym> {
        self.syms.iter().filter(move |s| s.name == name)
    }

    /// Ký hiệu theo tên + loại (tiện cho test khung).
    pub fn kind_of(&self, id: u32) -> Option<SymKind> {
        self.get(id).map(|s| s.kind)
    }
}

/// Template biến thể enum generic (ST-35; **VD-270**: dùng chung toàn chương trình):
/// `(tên, tham số kiểu, [(biến thể, kiểu trường)])`.
pub type EnumTmpl = (String, Vec<String>, Vec<(String, Vec<Type>)>);
