// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-312c (VD-289)** — bảng ký hiệu + khung tên (dữ liệu thuần).
//!
//! Chuyển **nguyên văn** từ `vis-types/src/lower.rs` để `vis-bytecode`/`vis-vm` đọc thẳng
//! `vis_hir::Symbols` mà không phải kéo cả tầng hạ/kiểm kiểu.

use std::collections::HashMap;

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

/// **ST-316b (`#199`)** — chỉ mục **dẫn xuất** `khung → tên → chỉ số ký hiệu cuối cùng`.
///
/// Dựng tăng dần trong [`Symbols::push_sym`] nên tra `(khung, tên)` là `O(1)` thay cho quét
/// tuyến tính `O(N)` của `in_frame`/`resolve`. `so_sym` ghi số phần tử `syms` đã lập chỉ mục:
/// nếu ai đó thêm thẳng vào `syms` (không qua `push_sym`), chỉ mục bị coi là cũ và mọi phép tra
/// quay về đường quét tuyến tính cũ (kết quả y hệt, chỉ mất tốc độ).
///
/// Chỉ mục là dữ liệu **dẫn xuất** từ `syms` ⇒ [`PartialEq`] bỏ qua nó (hai bảng cùng dữ liệu
/// luôn bằng nhau dù chỉ mục dựng khác đường).
#[derive(Debug, Clone, Default)]
pub struct ChiMuc {
    theo_khung: HashMap<u32, HashMap<String, u32>>,
    so_sym: usize,
}

impl PartialEq for ChiMuc {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for ChiMuc {}

impl ChiMuc {
    /// Dựng lại từ đầu (dùng khi phát hiện `syms` bị sửa ngoài `push_sym`).
    fn dung_lai(&mut self, syms: &[HSym]) {
        self.theo_khung.clear();
        for (i, s) in syms.iter().enumerate() {
            self.ghi(s.frame, s.name.clone(), i as u32);
        }
        self.so_sym = syms.len();
    }

    fn ghi(&mut self, frame: u32, name: String, id: u32) {
        self.theo_khung.entry(frame).or_default().insert(name, id);
        self.so_sym += 1;
    }
}

/// Bảng ký hiệu của một lượt hạ: các khung + ký hiệu theo thứ tự khai báo.
///
/// ST-25 thêm ba mảng **song song** với `syms` để kiểm tầm nhìn `pub`/`use` **ở tầng giải tên**
/// (`mod_of` = module sở hữu, `pub_of` = có `pub` hay không) và `mods` = danh sách module.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Symbols {
    pub frames: Vec<HFrame>,
    pub syms: Vec<HSym>,
    /// **ST-316b (`#199`)** — chỉ mục tra cứu `(khung, tên)` dựng tăng dần (dữ liệu dẫn xuất).
    pub chi_muc: ChiMuc,
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

    /// **ST-316b (`#199`)** — thêm ký hiệu **qua chỉ mục** (đường chuẩn cho mọi chỗ đẩy `syms`).
    ///
    /// Trả chỉ số của ký hiệu vừa thêm. Nếu `syms` từng bị sửa ngoài đường này, chỉ mục được
    /// dựng lại một lần trước khi ghi.
    pub fn push_sym(&mut self, sym: HSym) -> u32 {
        if !self.chi_muc_khop() {
            let syms = &self.syms;
            self.chi_muc.dung_lai(syms);
        }
        let id = self.syms.len() as u32;
        self.chi_muc.ghi(sym.frame, sym.name.clone(), id);
        self.syms.push(sym);
        id
    }

    /// **ST-316b (`#199`)** — đổi **khung** của ký hiệu `id` và cập nhật chỉ mục.
    ///
    /// `vis-types` tái dùng sym tham số `cmd` trong khung con của thân (xem
    /// `lower_than_khai_lenh`); nếu chỉ sửa `HSym::frame` thì chỉ mục `(khung, tên)` sẽ trỏ sai.
    pub fn doi_khung(&mut self, id: u32, frame: u32) {
        let Some(s) = self.syms.get(id as usize) else {
            return;
        };
        let old_frame = s.frame;
        let name = s.name.clone();
        if old_frame == frame {
            return;
        }
        if !self.chi_muc_khop() {
            let syms = &self.syms;
            self.chi_muc.dung_lai(syms);
        }
        if let Some(s) = self.syms.get_mut(id as usize) {
            s.frame = frame;
        }
        // Khoá cũ: chỉ sửa nếu đang trỏ đúng `id`; nếu còn bản trước cùng `(khung, tên)` thì lùi về nó.
        let con_ban_truoc = self
            .syms
            .iter()
            .enumerate()
            .rev()
            .find(|(i, s)| *i as u32 != id && s.frame == old_frame && s.name == name)
            .map(|(i, _)| i as u32);
        if let Some(m) = self.chi_muc.theo_khung.get_mut(&old_frame) {
            if m.get(&name) == Some(&id) {
                match con_ban_truoc {
                    Some(prev) => {
                        m.insert(name.clone(), prev);
                    }
                    None => {
                        m.remove(&name);
                    }
                }
            }
        }
        // Khoá mới: chỉ ghi nếu `id` là bản **cuối** (khai báo sau ⇒ id lớn hơn).
        let hien_tai = self
            .chi_muc
            .theo_khung
            .get(&frame)
            .and_then(|m| m.get(&name))
            .copied();
        if hien_tai.is_none_or(|cur| cur < id) {
            self.chi_muc
                .theo_khung
                .entry(frame)
                .or_default()
                .insert(name, id);
        }
    }

    /// Chỉ mục `(khung, tên)` còn khớp với `syms` không.
    pub fn chi_muc_khop(&self) -> bool {
        self.chi_muc.so_sym == self.syms.len()
    }

    /// **ST-316b (`#199`)** — chỉ số ký hiệu **cuối cùng** mang `(frame, name)`, `O(1)`.
    ///
    /// Kết quả **y hệt** `syms.iter().rposition(|s| s.frame == frame && s.name == name)` mà
    /// `in_frame` dùng trước đây; khi chỉ mục cũ/không đồng bộ thì quét tuyến tính như trước.
    pub fn last_id_in_frame(&self, frame: u32, name: &str) -> Option<u32> {
        if self.chi_muc_khop() {
            let id = *self.chi_muc.theo_khung.get(&frame)?.get(name)?;
            if let Some(s) = self.syms.get(id as usize) {
                if s.frame == frame && s.name == name {
                    return Some(id);
                }
            }
        }
        self.syms
            .iter()
            .rposition(|s| s.frame == frame && s.name == name)
            .map(|i| i as u32)
    }

    /// Ký hiệu theo tên trong **một** khung (lần khai báo cuối trong khung đó).
    pub fn in_frame(&self, frame: u32, name: &str) -> Option<&HSym> {
        self.last_id_in_frame(frame, name)
            .and_then(|i| self.syms.get(i as usize))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(ten: &str, frame: u32, kind: SymKind) -> HSym {
        HSym {
            name: ten.to_string(),
            kind,
            frame,
            span: Span::new(0, 0),
            ty: None,
        }
    }

    /// **ST-316b (`#199`)** — chỉ mục `(khung, tên)` cho **cùng** kết quả với quét tuyến tính:
    /// bản khai **cuối** trong khung thắng; khung khác/không có tên ⇒ `None`.
    #[test]
    fn chi_muc_khop_quet_tuyen_tinh() {
        let mut s = Symbols::default();
        for i in 0..1_000 {
            s.push_sym(sym(&format!("x{i}"), 0, SymKind::Item));
        }
        s.push_sym(sym("x1", 0, SymKind::Param)); // trùng tên ⇒ bản cuối
        s.push_sym(sym("x1", 3, SymKind::Var)); // cùng tên, khác khung
        assert_eq!(s.in_frame(0, "x1").map(|x| x.kind), Some(SymKind::Param));
        assert_eq!(s.in_frame(3, "x1").map(|x| x.kind), Some(SymKind::Var));
        assert_eq!(s.in_frame(0, "khong_co"), None);
        assert_eq!(s.in_frame(9, "x2"), None);
        // Kết quả phải trùng quét tuyến tính trên **mọi** tên.
        for i in 0..1_000 {
            let ten = format!("x{i}");
            let tuyen = s
                .syms
                .iter()
                .enumerate()
                .rev()
                .find(|(_, x)| x.frame == 0 && x.name == ten)
                .map(|(k, _)| k as u32);
            assert_eq!(s.last_id_in_frame(0, &ten), tuyen, "lệch ở {ten}");
        }
    }

    /// Chỉ mục **cũ** (ai đó đẩy thẳng vào `syms`) ⇒ `in_frame` quay về quét tuyến tính, không
    /// bỏ sót ký hiệu mới.
    #[test]
    fn chi_muc_cu_van_tra_dung() {
        let mut s = Symbols::default();
        s.push_sym(sym("a", 0, SymKind::Item));
        s.syms.push(sym("b", 0, SymKind::Item)); // không qua push_sym ⇒ chỉ mục cũ
        assert!(!s.chi_muc_khop());
        assert_eq!(s.in_frame(0, "b").map(|x| x.name.as_str()), Some("b"));
        assert_eq!(s.in_frame(0, "a").map(|x| x.name.as_str()), Some("a"));
        // push_sym kế tiếp dựng lại chỉ mục rồi mới ghi.
        s.push_sym(sym("c", 0, SymKind::Item));
        assert!(s.chi_muc_khop());
        assert_eq!(s.in_frame(0, "c").map(|x| x.name.as_str()), Some("c"));
    }
    /// **ST-316b (`#199`)** — đổi khung phải cập nhật chỉ mục: `vis-types` tái dùng sym tham số
    /// `cmd` trong khung con (trước: sửa `HSym::frame` trực tiếp làm `resolve` mất tên).
    #[test]
    fn doi_khung_cap_nhat_chi_muc() {
        let mut s = Symbols::default();
        s.push_sym(sym("x", 0, SymKind::Param));
        s.push_sym(sym("x", 0, SymKind::Item)); // bản sau cùng trong khung 0
        s.doi_khung(0, 5);
        assert_eq!(s.in_frame(0, "x").map(|x| x.kind), Some(SymKind::Item));
        assert_eq!(s.in_frame(5, "x").map(|x| x.kind), Some(SymKind::Param));
        assert_eq!(s.last_id_in_frame(5, "x"), Some(0));
        s.doi_khung(0, 0);
        assert_eq!(s.in_frame(5, "x"), None);
        assert_eq!(s.in_frame(0, "x").map(|x| x.kind), Some(SymKind::Item));
    }
}
