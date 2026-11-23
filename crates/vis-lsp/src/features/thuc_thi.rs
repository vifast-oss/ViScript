// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/implementation` cho LSP.
//!
//! Dùng lại [`super::phan_tich::KetQua::phuong`] (phương thức `impl`/`trait` đã gom cho hover).
//! Chỉ trả **phương thức `impl` trùng tên**:
//! - người nhận có kiểu tĩnh (`u.lay`, `self.lay`, `v.hien`) ⇒ lọc `chu` == kiểu đó;
//! - tại khai báo `impl` ⇒ lọc theo kiểu của khối `impl`;
//! - tại khai báo `trait` ⇒ mọi phương thức `impl` cùng tên (không lọc chủ);
//! - không suy được kiểu (gọi tự do/toàn cục) ⇒ mọi phương thức `impl` cùng tên, **trừ** từ là
//!   phương thức `trait`: không rõ chủ thì `null` (không đoán).
//!
//! Không suy quan hệ kiểu/kế thừa ngoài ghép tên trait ↔ impl. Khử trùng theo **span tên**, cap
//! **50**, thứ tự khai; không có kết quả ⇒ `result:null`. Range 0-based **UTF-16** như các method
//! LSP khác (`vi_tri::khoang`).

use std::collections::HashSet;

use vis_core::value::Value;
use vis_syntax::Span;

use super::dinh_nghia::thu_hep_ten;
use super::goi_y::{chu_tinh, tu_tai};
use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::{KetQua, Phuong};
use super::vi_tri::khoang;

/// Số `Location` tối đa (cùng tinh thần cap 20/200 của definition/references).
const CAP: usize = 50;

/// Từ tại con trỏ có nằm **đúng trong tên** khai báo `fn` của phương thức không (span HIR của `fn`
/// phủ cả thân, nên phải thu về tên trước khi so).
fn tai_ten(text: &str, p: &Phuong, start: usize) -> bool {
    let ten = thu_hep_ten(text, &p.ten, p.span);
    start >= ten.start && start < ten.end
}

/// Phản hồi `textDocument/implementation` (`Location[]`) hoặc `result:null`.
pub fn tra_loi_implementation(kq: &KetQua, uri: &str, byte: usize, id: &Value) -> Vec<u8> {
    let (start, end) = tu_tai(&kq.text, byte);
    if start == end {
        return phan_hoi(id, Value::none());
    }
    let tu = &kq.text[start..end];
    let ung: Vec<&Phuong> = kq.phuong.iter().filter(|p| p.ten == tu).collect();
    if ung.is_empty() {
        return phan_hoi(id, Value::none());
    }

    // Khai báo đang đứng (nếu có) quyết định chủ: `impl` ⇒ kiểu khối; `trait` ⇒ không lọc chủ.
    let khai_bao: Vec<&Phuong> = ung
        .iter()
        .copied()
        .filter(|p| tai_ten(&kq.text, p, start))
        .collect();
    let chu = match khai_bao.as_slice() {
        [p] if p.impl_span.is_some() => Some(p.chu.clone()),
        [_] => None,
        _ => chu_tinh(kq, start),
    };

    // Từ là phương thức `trait` nhưng không phải khai báo và không rõ người nhận ⇒ `null` (không
    // đoán implementation nào). Phương thức `impl` thường vẫn trả mọi bản cùng tên.
    let la_trait = ung.iter().any(|p| p.impl_span.is_none());
    if la_trait && khai_bao.is_empty() && chu.is_none() {
        return phan_hoi(id, Value::none());
    }

    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    let mut da: HashSet<(usize, usize)> = HashSet::new();
    let mut locs: Vec<Value> = Vec::new();
    for p in ung {
        // Chỉ `impl` mới là "implementation"; khai báo `trait` không phải đích nhảy.
        if p.impl_span.is_none() {
            continue;
        }
        if let Some(c) = &chu {
            if &p.chu != c {
                continue;
            }
        }
        let ten = thu_hep_ten(&kq.text, &p.ten, p.span);
        if !da.insert((ten.start, ten.end)) {
            continue;
        }
        let ((l0, c0), (l1, c1)) = khoang(&kq.text, Span::new(ten.start, ten.end));
        locs.push(m(vec![
            ("uri", s(uri)),
            (
                "range",
                m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
            ),
        ]));
        if locs.len() >= CAP {
            break;
        }
    }
    if locs.is_empty() {
        return phan_hoi(id, Value::none());
    }
    phan_hoi(id, Value::List(locs))
}
