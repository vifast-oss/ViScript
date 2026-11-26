// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-315b (VD-291 §3.2)** — `VisMap`: bảng khoá → giá trị **giữ thứ tự chèn**, chia sẻ qua
//! `Rc` + copy-on-write (`Value::Map(Rc<VisMap>)`), chỉ mục băm **lười** cho map đủ lớn.
//!
//! # Hợp đồng ngữ nghĩa (chốt Q1–Q3 của `VD-291` §3.2)
//!
//! - **Q1**: khoá trùng theo [`Value::eq_value`] (`1`/`1.0`, `0.5`/`50pct`, `NaN` không bằng
//!   chính nó) ⇒ [`VisMap::insert`] cập nhật **giá trị tại chỗ**, giữ **khoá gốc** và **vị trí
//!   chèn đầu**. `NaN` không khớp chính nó nên mỗi lần chèn `NaN` là một entry mới — y hệt hành vi
//!   quét tuyến tính trước đây.
//! - **Q2**: `json.decode` last-wins — dựng map qua [`VisMap::from_pairs`] nên chỉ còn **một**
//!   entry cho mỗi khoá, giá trị cuối, vị trí đầu; bỏ vòng quét O(N²) ở decoder.
//! - **Q3**: `Show`/`keys()` theo thứ tự chèn; [`VisMap::remove`] bỏ mục, [`VisMap::insert`] lại
//!   chèn **cuối** — byte-for-byte như `Vec<(Value, Value)>` cũ.
//!
//! # Chỉ mục băm
//!
//! [`MapIndex`] chỉ dựng khi `entries.len() >= NGUONG_CHI_MUC` (map nhỏ như JSON-RPC của LSP
//! không cấp phát thêm). Chỉ mục chỉ phủ khoá **vô hướng** — cùng hợp đồng băm với
//! `bam_khoa_vo_huong` ở [`super::show_eq`] (`Int`/`Float`/`Percent` cùng miền số, `-0.0 ==
//! +0.0`, trùng băm do bucket `eq_value` quyết). Khoá phức tạp (`List`/`Map`/`Record`/…) luôn
//! rơi về quét tuyến tính, nên chỉ mục **không thể** trả sai.
//!
//! Đột biến nào cũng giữ chỉ mục đồng bộ: [`VisMap::insert`] nối thêm vào bucket, [`VisMap::remove`]
//! dựng lại (hoặc bỏ) chỉ mục. `Clone` **cố ý bỏ** chỉ mục: `Rc::make_mut` chỉ nhân bản khi map
//! đang chia sẻ, và bản sao dựng lại chỉ mục theo nhu cầu — không nhân bản `HashMap` vô ích.

use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasherDefault;
use std::ops::Deref;

use super::show_eq::bam_khoa_vo_huong;
use super::{Fnv, Value};

/// Ngưỡng dựng chỉ mục băm (số entry). Dưới ngưỡng, quét tuyến tính rẻ hơn cấp phát `HashMap`.
///
/// Chọn `8` sau khi đo `N = 1, 4, 8, 16, 79, 316` (`agent/JOURNAL/st315b-map-cow.md`): map nhỏ
/// (JSON-RPC của LSP, `{}`/`{a: 1}`) không cấp phát phụ; từ 8 khoá trở lên chỉ mục đã thắng rõ.
pub const NGUONG_CHI_MUC: usize = 8;

/// Chỉ mục băm của [`VisMap`]: `dau[h]` = entry **mới nhất** mang băm `h`, `sau[i]` = entry kế
/// tiếp trong cùng bucket (`usize::MAX` = hết chuỗi). `sau` có **một ô cho mỗi entry** (kể cả
/// khoá không vô hướng — ô đó không bao giờ được bucket trỏ tới).
#[derive(Default)]
pub struct MapIndex {
    dau: HashMap<u64, usize, BuildHasherDefault<Fnv>>,
    sau: Vec<usize>,
}

impl MapIndex {
    /// Dựng chỉ mục cho `entries`; `None` khi dưới ngưỡng.
    fn dung(entries: &[(Value, Value)]) -> Option<Box<MapIndex>> {
        if entries.len() < NGUONG_CHI_MUC {
            return None;
        }
        let mut ix = MapIndex {
            dau: HashMap::with_capacity_and_hasher(entries.len(), BuildHasherDefault::default()),
            sau: Vec::with_capacity(entries.len()),
        };
        for i in 0..entries.len() {
            ix.them(entries, i);
        }
        Some(Box::new(ix))
    }

    /// Nối entry `i` vào đầu bucket của nó (khoá không vô hướng ⇒ chỉ đẩy ô `usize::MAX`).
    fn them(&mut self, entries: &[(Value, Value)], i: usize) {
        let truoc = bam_khoa_vo_huong(&entries[i].0).and_then(|h| self.dau.insert(h, i));
        self.sau.push(truoc.unwrap_or(usize::MAX));
    }

    /// Vị trí entry khớp `h` + `k` (**cũ nhất** trong bucket — chuỗi là mới→cũ, nên ghi đè lần
    /// khớp cuối). Đúng `find` tuyến tính cũ: `1` và `100pct` không `eq_value` nhau nhưng cùng
    /// khớp `1.0`, nên phải trả entry chèn **trước** để `get`/`insert` không lệch ngữ nghĩa.
    fn tim(&self, entries: &[(Value, Value)], h: u64, k: &Value) -> Option<usize> {
        let mut i = *self.dau.get(&h)?;
        let mut tim = None;
        while i != usize::MAX {
            if entries[i].0.eq_value(k) {
                tim = Some(i);
            }
            i = self.sau[i];
        }
        tim
    }

    /// `true` nếu trong bucket `h` có entry khớp **cả** khoá `k` lẫn giá trị `v` — đúng phép
    /// "có một cặp (k, v)" của `eq_value` map cũ (không chỉ entry đầu tiên khớp khoá).
    fn co_cap(&self, entries: &[(Value, Value)], h: u64, k: &Value, v: &Value) -> bool {
        let Some(mut i) = self.dau.get(&h).copied() else {
            return false;
        };
        while i != usize::MAX {
            let (k2, v2) = &entries[i];
            if k2.eq_value(k) && v2.eq_value(v) {
                return true;
            }
            i = self.sau[i];
        }
        false
    }
}

/// Bảng khoá → giá trị giữ **thứ tự chèn**, chia sẻ qua `Rc` + CoW.
///
/// Bất biến: mỗi khoá (theo [`Value::eq_value`]) xuất hiện **tối đa một** lần. Mọi đường dựng
/// (`Value::map`, `from_pairs`, deserialize) đều đi qua [`VisMap::insert`].
pub struct VisMap {
    entries: Vec<(Value, Value)>,
    index: Option<Box<MapIndex>>,
}

impl Default for VisMap {
    fn default() -> Self {
        VisMap::new()
    }
}

impl Clone for VisMap {
    /// Bản sao cho `Rc::make_mut`: chép entry, **bỏ** chỉ mục (dựng lại khi cần) để không nhân
    /// bản `HashMap` của map lớn.
    fn clone(&self) -> Self {
        VisMap {
            entries: self.entries.clone(),
            index: None,
        }
    }
}

impl fmt::Debug for VisMap {
    /// `Debug` chỉ in entry (không in chỉ mục) để đầu ra tất định như `Vec` cũ.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.entries.iter()).finish()
    }
}

impl Deref for VisMap {
    type Target = [(Value, Value)];

    /// Cho phép mọi thao tác **chỉ đọc** của slice (`iter`, `len`, `is_empty`, `first`, `[i]`…)
    /// trên `Rc<VisMap>` mà không phải sửa 45 điểm khớp mẫu.
    fn deref(&self) -> &[(Value, Value)] {
        &self.entries
    }
}

impl<'a> IntoIterator for &'a VisMap {
    type Item = &'a (Value, Value);
    type IntoIter = std::slice::Iter<'a, (Value, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl VisMap {
    /// Map rỗng, **không** cấp phát.
    pub fn new() -> Self {
        VisMap {
            entries: Vec::new(),
            index: None,
        }
    }

    /// Map rỗng với sức chứa `n` entry.
    pub fn with_capacity(n: usize) -> Self {
        VisMap {
            entries: Vec::with_capacity(n),
            index: None,
        }
    }

    /// Dựng từ danh sách cặp, **last-wins** tại chỗ (giữ khoá gốc + vị trí đầu) — nguồn duy nhất
    /// cho literal `{}`, `Instr::MakeMap` của VM, `json.decode`, snapshot worker.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (Value, Value)>) -> Self {
        let it = pairs.into_iter();
        let (lo, hi) = it.size_hint();
        let mut m = VisMap::with_capacity(hi.unwrap_or(lo));
        for (k, v) in it {
            m.insert(k, v);
        }
        m
    }

    /// Số entry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Không có entry nào.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Toàn bộ entry theo thứ tự chèn.
    pub fn as_slice(&self) -> &[(Value, Value)] {
        &self.entries
    }

    /// Entry theo thứ tự chèn (tường minh — `Deref` cũng cho `iter()`).
    pub fn iter(&self) -> std::slice::Iter<'_, (Value, Value)> {
        self.entries.iter()
    }

    /// Khoá theo thứ tự chèn.
    pub fn keys(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(k, _)| k)
    }

    /// Giá trị theo thứ tự chèn.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(_, v)| v)
    }

    /// `true` nếu map đang có chỉ mục băm (chỉ để test/đo).
    #[doc(hidden)]
    pub fn co_chi_muc(&self) -> bool {
        self.index.is_some()
    }

    /// Vị trí entry khớp `k` theo [`Value::eq_value`]; dùng chỉ mục khi có và khoá vô hướng.
    fn vi_tri(&self, k: &Value) -> Option<usize> {
        if let Some(ix) = &self.index {
            if let Some(h) = bam_khoa_vo_huong(k) {
                return ix.tim(&self.entries, h, k);
            }
        }
        self.entries.iter().position(|(k2, _)| k2.eq_value(k))
    }

    /// Giá trị theo khoá (so [`Value::eq_value`]).
    pub fn get(&self, k: &Value) -> Option<&Value> {
        self.vi_tri(k).map(|i| &self.entries[i].1)
    }

    /// Có khoá `k` không.
    pub fn contains_key(&self, k: &Value) -> bool {
        self.vi_tri(k).is_some()
    }

    /// Có **cặp** khoá `k` + giá trị `v` không (dùng cho so map: `eq_value` cũ xét *một* cặp
    /// bất kỳ khớp cả khoá lẫn giá trị, không chỉ entry đầu khớp khoá).
    pub fn co_cap(&self, k: &Value, v: &Value) -> bool {
        if let Some(ix) = &self.index {
            if let Some(h) = bam_khoa_vo_huong(k) {
                return ix.co_cap(&self.entries, h, k, v);
            }
        }
        self.entries
            .iter()
            .any(|(k2, v2)| k2.eq_value(k) && v2.eq_value(v))
    }

    /// Chèn/cập nhật **last-wins**: khoá trùng ⇒ giá trị tại chỗ, giữ khoá gốc + vị trí đầu.
    /// Trả giá trị cũ nếu có.
    pub fn insert(&mut self, k: Value, v: Value) -> Option<Value> {
        // **ST-315b review F3**: sau `Rc::make_mut` bản sao mất chỉ mục; nếu đang cập nhật khoá
        // **đã có**, dựng lại chỉ mục trước khi tra để các lần `get` sau không quét N.
        if self.index.is_none() && self.entries.len() >= NGUONG_CHI_MUC {
            self.index = MapIndex::dung(&self.entries);
        }
        if let Some(i) = self.vi_tri(&k) {
            return Some(std::mem::replace(&mut self.entries[i].1, v));
        }
        self.entries.push((k, v));
        let i = self.entries.len() - 1;
        match self.index.as_mut() {
            Some(ix) => ix.them(&self.entries, i),
            None => self.index = MapIndex::dung(&self.entries),
        }
        None
    }

    /// Xoá khoá `k`, trả giá trị cũ. Các entry còn lại **giữ nguyên thứ tự**; `set` lại sau đó
    /// chèn **cuối** (Q3).
    pub fn remove(&mut self, k: &Value) -> Option<Value> {
        let i = self.vi_tri(k)?;
        let (_, v) = self.entries.remove(i);
        self.index = MapIndex::dung(&self.entries);
        Some(v)
    }

    /// Gộp `other` vào `self` (last-wins, giữ vị trí của `self` cho khoá đã có, nối cuối cho
    /// khoá mới) — đúng `map.merge` cũ.
    pub fn merge(&mut self, other: &VisMap) {
        if self.index.is_none() {
            self.index = MapIndex::dung(&self.entries);
        }
        for (k, v) in &other.entries {
            self.insert(k.clone(), v.clone());
        }
    }

    /// Bọc thành `Value::Map` chia sẻ.
    pub fn thanh_gia_tri(self) -> Value {
        Value::Map(std::rc::Rc::new(self))
    }

    /// Bóc toàn bộ entry (dùng khi đã sở hữu `VisMap`, ví dụ `Rc::try_unwrap`).
    pub fn into_entries(self) -> Vec<(Value, Value)> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::show;

    fn s(x: &str) -> Value {
        Value::Str(x.into())
    }

    #[test]
    fn giu_thu_tu_va_last_wins_tai_cho() {
        let mut m = VisMap::new();
        m.insert(s("a"), Value::Int(1));
        m.insert(s("b"), Value::Int(2));
        m.insert(s("a"), Value::Int(3));
        assert_eq!(m.len(), 2);
        assert!(m.get(&s("a")).is_some_and(|v| v.eq_value(&Value::Int(3))));
        assert_eq!(m.keys().map(|k| k.show()).collect::<Vec<_>>(), ["a", "b"]);
        assert_eq!(show(&m.clone().thanh_gia_tri()), "{\"a\": 3, \"b\": 2}");
    }

    #[test]
    fn khoa_trung_eq_value_int_float_percent() {
        let mut m = VisMap::new();
        m.insert(Value::Int(1), Value::Int(10));
        m.insert(Value::Float(1.0), Value::Int(20));
        assert_eq!(m.len(), 1);
        assert!(m
            .get(&Value::Int(1))
            .is_some_and(|v| v.eq_value(&Value::Int(20))));
        assert!(m
            .get(&Value::Float(1.0))
            .is_some_and(|v| v.eq_value(&Value::Int(20))));

        let mut m = VisMap::new();
        m.insert(Value::Percent(0.5), Value::Int(1));
        m.insert(Value::Float(0.5), Value::Int(2));
        assert_eq!(m.len(), 1);
        assert!(m
            .get(&Value::Percent(0.5))
            .is_some_and(|v| v.eq_value(&Value::Int(2))));
    }

    #[test]
    fn nan_khong_khop_chinh_no() {
        let mut m = VisMap::new();
        m.insert(Value::Float(f64::NAN), Value::Int(1));
        m.insert(Value::Float(f64::NAN), Value::Int(2));
        assert_eq!(m.len(), 2);
        assert!(m.get(&Value::Float(f64::NAN)).is_none());
    }

    #[test]
    fn get_float_khi_co_int_va_percent_tra_entry_cu_nhat() {
        // `eq_value(Int, Percent)` **sai** (không bắc cầu qua Float) nên map giữ **hai** entry;
        // `Float(1.0)` khớp cả hai ⇒ `get` phải trả entry **chèn trước** như `find` tuyến tính cũ.
        let mut m = VisMap::new();
        m.insert(Value::Int(1), Value::Int(10));
        m.insert(Value::Percent(1.0), Value::Int(20));
        assert_eq!(m.len(), 2);
        assert_eq!(
            m.get(&Value::Float(1.0)).map(Value::show),
            Some("10".to_string())
        );
        assert_eq!(
            m.get(&Value::Int(1)).map(Value::show),
            Some("10".to_string())
        );
        assert_eq!(
            m.get(&Value::Percent(1.0)).map(Value::show),
            Some("20".to_string())
        );
        // `insert(Float(1.0))` cập nhật entry **cũ nhất** (Int) như quét tuyến tính cũ.
        m.insert(Value::Float(1.0), Value::Int(99));
        assert_eq!(m.len(), 2);
        assert_eq!(
            m.get(&Value::Int(1)).map(Value::show),
            Some("99".to_string())
        );
    }

    #[test]
    fn remove_roi_set_lai_chen_cuoi() {
        let mut m = VisMap::new();
        m.insert(s("a"), Value::Int(1));
        m.insert(s("b"), Value::Int(2));
        m.insert(s("c"), Value::Int(3));
        assert!(m
            .remove(&s("b"))
            .is_some_and(|v| v.eq_value(&Value::Int(2))));
        m.insert(s("b"), Value::Int(9));
        assert_eq!(show(&m.thanh_gia_tri()), "{\"a\": 1, \"c\": 3, \"b\": 9}");
    }

    #[test]
    fn chi_muc_dung_tu_nguong_va_khoa_phuc_tap_fallback() {
        let nho = VisMap::from_pairs(
            (0..NGUONG_CHI_MUC - 1).map(|i| (Value::Int(i as i64), Value::Int(0))),
        );
        assert!(!nho.co_chi_muc());
        let lon =
            VisMap::from_pairs((0..NGUONG_CHI_MUC).map(|i| (Value::Int(i as i64), Value::Int(0))));
        assert!(lon.co_chi_muc());

        let mut m =
            VisMap::from_pairs((0..NGUONG_CHI_MUC).map(|i| (Value::Int(i as i64), Value::Int(0))));
        m.insert(Value::List(vec![Value::Int(1)]), Value::Int(7));
        assert!(m
            .get(&Value::List(vec![Value::Int(1)]))
            .is_some_and(|v| v.eq_value(&Value::Int(7))));
        assert_eq!(m.len(), NGUONG_CHI_MUC + 1);
    }
}
