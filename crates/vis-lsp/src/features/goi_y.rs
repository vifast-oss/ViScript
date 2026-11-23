// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-69 (VD-102 (5))** — `textDocument/completion` + `textDocument/hover`.
//!
//! Nguồn gợi ý (khử trùng theo `label`, ưu tiên **item > biến > builtin > từ khoá**, sắp theo **byte**
//! của label, lọc theo **tiền tố** tại con trỏ, cap **200**):
//! item cấp tệp (`fn` 3 / record 22 / enum 13 / alias 25 / const 21 / trait 11), biến-tham số (6),
//! `vis_types::BUILTIN_VALUE_NAMES` (3), `vis_syntax::KEYWORDS` (14).
//!
//! Hover: tra item → biến → trường → ký hiệu → builtin → từ khoá → phương thức (`impl`/`trait`)
//! → phương thức lõi (`list`/`map`/`str`); không có ⇒ `result:null`. `range` là **đúng từ** tại
//! con trỏ (0-based **UTF-16**, tab = 1 — dùng lại `phan_tich::khoang`).

use std::collections::HashSet;

use vis_core::value::Value;
use vis_syntax::{Span, KEYWORDS};
use vis_types::BUILTIN_VALUE_NAMES;

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::{GoiY, KetQua, Phuong};
use super::vi_tri::khoang;

fn la_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Vị trí bắt đầu từ + **tiền tố** kết thúc tại `byte` (dùng cho completion).
pub fn tien_to(text: &str, byte: usize) -> (usize, String) {
    let b = byte.min(text.len());
    let mut start = b;
    for (i, c) in text[..b].char_indices().rev() {
        if la_ident(c) {
            start = i;
        } else {
            break;
        }
    }
    (start, text[start..b].to_string())
}

/// Biên **byte** của từ tại `byte` (rỗng nếu con trỏ không nằm trong/trên từ).
pub(crate) fn tu_tai(text: &str, byte: usize) -> (usize, usize) {
    let b = byte.min(text.len());
    let mut start = b;
    for (i, c) in text[..b].char_indices().rev() {
        if la_ident(c) {
            start = i;
        } else {
            break;
        }
    }
    let mut end = b;
    for (i, c) in text[b..].char_indices() {
        if la_ident(c) {
            end = b + i + c.len_utf8();
        } else {
            break;
        }
    }
    (start, end)
}

/// Tên biến **ngay trước dấu chấm** cuối cùng trước con trỏ (chỉ `a.` / `a.xy`, không biểu thức).
/// `None` ⇒ không phải truy cập thành viên đơn giản ⇒ dùng fallback toàn bộ.
fn nhan_sau_cham(text: &str, byte: usize) -> Option<String> {
    let b = byte.min(text.len());
    let before = &text[..b];
    let dot = before.rfind('.')?;
    let sau = &before[dot + 1..];
    if !sau.chars().all(la_ident) {
        return None;
    }
    if dot == 0 {
        return None;
    }
    let mut start = dot;
    for (i, c) in text[..dot].char_indices().rev() {
        if la_ident(c) {
            start = i;
        } else {
            break;
        }
    }
    if start == dot {
        return None;
    }
    Some(text[start..dot].to_string())
}

/// **ST-91**: gợi ý **thành viên** theo kiểu tĩnh của biến trước dấu chấm; `None` = không xác định
/// kiểu ⇒ fallback. Kiểu xác định nhưng **không** có trường/phương thức (vd `str`) ⇒ trả **rỗng**.
fn goi_y_thanh_vien(kq: &KetQua, byte: usize, tien_to: &str) -> Option<Vec<GoiY>> {
    let ten = nhan_sau_cham(&kq.text, byte)?;
    // `p.` một mình là cú pháp dở ⇒ HIR gốc có thể rỗng; phân tích lại với sentinel.
    let tv = super::thanh_vien::tu_van_ban_sua(&kq.text, byte);
    let kieu = tv
        .kieu_bien
        .iter()
        .rev()
        .find(|(n, _)| n == &ten)
        .map(|(_, k)| k.clone())?;
    let mut ds: Vec<GoiY> = tv
        .truong
        .iter()
        .filter(|(k, _)| k == &kieu)
        .map(|(_, g)| GoiY {
            label: g.label.clone(),
            kind: g.kind,
            detail: g.detail.clone(),
        })
        .chain(
            tv.phuong
                .iter()
                .filter(|(k, _)| k == &kieu)
                .map(|(_, g)| GoiY {
                    label: g.label.clone(),
                    kind: g.kind,
                    detail: g.detail.clone(),
                }),
        )
        .collect();
    ds.retain(|g| g.label.starts_with(tien_to));
    ds.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
    ds.truncate(200);
    Some(ds)
}

/// **ST-276**: gợi ý hàm `std.<module>` khi con trỏ sau `module.` (vd `fs.`, `sqlite.`).
/// Chỉ dùng tiền tố module từ bảng `FUNCS`; biến cùng tên có kiểu đã được `goi_y_thanh_vien`
/// xử lý trước nên không bị ghi đè.
fn goi_y_module(byte: usize, tien_to: &str, text: &str) -> Option<Vec<GoiY>> {
    let ten = nhan_sau_cham(text, byte)?;
    // **ST-305**: `std.` ⇒ gợi ý **module** (`fs`, `str`, `list`, …), không chỉ hàm sau `fs.`.
    if ten == "std" {
        let mut ds: Vec<GoiY> = Vec::new();
        let mut da = HashSet::new();
        for f in vis_types::stdlib::FUNCS {
            let Some((module, _)) = f.name.split_once('.') else {
                continue;
            };
            if !module.starts_with(tien_to) || !da.insert(module.to_string()) {
                continue;
            }
            ds.push(GoiY {
                label: module.to_string(),
                kind: 9, // CompletionItemKind::Module
                detail: Some(format!("std.{module}")),
            });
        }
        ds.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
        ds.truncate(200);
        return (!ds.is_empty()).then_some(ds);
    }
    let mut ds: Vec<GoiY> = vis_types::stdlib::FUNCS
        .iter()
        .filter_map(|f| {
            let (module, ham) = f.name.split_once('.')?;
            if module != ten || !ham.starts_with(tien_to) {
                return None;
            }
            let ps: Vec<&str> = f.params.iter().map(|t| t.ten()).collect();
            Some(GoiY {
                label: ham.to_string(),
                kind: 3,
                detail: Some(format!("{}({}) -> {}", f.name, ps.join(", "), f.ret.ten())),
            })
        })
        .collect();
    if ds.is_empty() {
        return None;
    }
    ds.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
    ds.truncate(200);
    Some(ds)
}

/// Danh sách gợi ý tại `byte` — đã khử trùng, lọc tiền tố, sắp theo byte, cap 200.
pub fn ds_goi_y(kq: &KetQua, byte: usize) -> Vec<GoiY> {
    let (_, tien_to) = tien_to(&kq.text, byte);
    if let Some(ds) = goi_y_thanh_vien(kq, byte, &tien_to) {
        return ds;
    }
    if let Some(ds) = goi_y_module(byte, &tien_to, &kq.text) {
        return ds;
    }
    let mut ds: Vec<GoiY> = Vec::new();
    let mut da_co: HashSet<String> = HashSet::new();
    for it in &kq.items {
        if da_co.insert(it.label.clone()) {
            ds.push(GoiY {
                label: it.label.clone(),
                kind: it.kind,
                detail: it.detail.clone(),
            });
        }
    }
    for (ten, _ty) in &kq.bien {
        if da_co.insert(ten.clone()) {
            ds.push(GoiY {
                label: ten.clone(),
                kind: 6,
                detail: None,
            });
        }
    }
    for ten in BUILTIN_VALUE_NAMES {
        if da_co.insert(ten.to_string()) {
            ds.push(GoiY {
                label: ten.to_string(),
                kind: 3,
                detail: None,
            });
        }
    }
    for ten in KEYWORDS {
        if da_co.insert(ten.to_string()) {
            ds.push(GoiY {
                label: ten.to_string(),
                kind: 14,
                detail: None,
            });
        }
    }
    ds.retain(|g| g.label.starts_with(&tien_to));
    ds.sort_by(|a, b| a.label.as_bytes().cmp(b.label.as_bytes()));
    ds.truncate(200);
    ds
}

/// Tên **ngay trước dấu chấm** cuối cùng trước `start` (chỉ `a.` / `a.xy`, không biểu thức).
fn ten_truoc_cham(text: &str, start: usize) -> Option<&str> {
    let goc = text[..start].trim_end().strip_suffix('.')?.trim_end();
    let i = goc
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map(|i| i + 1)
        .unwrap_or(0);
    let recv = &goc[i..];
    if recv.is_empty() {
        None
    } else {
        Some(recv)
    }
}

/// Kiểu tĩnh của tên **ngay trước dấu chấm** cuối cùng trước `start`. `None` ⇒ không xác định được
/// kiểu (không phải truy cập thành viên, hoặc tên không có trong `bien`).
fn kieu_truoc_cham(kq: &KetQua, text: &str, start: usize) -> Option<String> {
    let recv = ten_truoc_cham(text, start)?;
    kq.bien
        .iter()
        .find(|(n, _)| n == recv)
        .map(|(_, k)| k.clone())
}

/// **VD-270**: chủ sở hữu (kiểu tĩnh) của từ tại `start` khi từ nằm sau dấu chấm — `u.lay` ⇒ kiểu
/// của `u`, `self.lay` trong `impl` ⇒ kiểu của khối `impl` chứa con trỏ. `None` ⇒ không xác định.
pub(crate) fn chu_tinh(kq: &KetQua, start: usize) -> Option<String> {
    match ten_truoc_cham(&kq.text, start) {
        Some("self") => kq
            .phuong
            .iter()
            .find(|p| {
                p.impl_span
                    .is_some_and(|s| start >= s.start && start < s.end)
            })
            .map(|p| p.chu.clone()),
        _ => kieu_truoc_cham(kq, &kq.text, start),
    }
}

/// **ST-100**: hover **trường** — `p.x` (theo kiểu `p`) hoặc khai báo duy nhất `x: int` trong `type`.
fn hover_truong(kq: &KetQua, text: &str, start: usize, tu: &str) -> Option<String> {
    let chu_p = kieu_truoc_cham(kq, text, start);
    let mut ung: Vec<(&str, &GoiY)> = kq
        .truong
        .iter()
        .filter(|(_, g)| g.label == tu)
        .map(|(k, g)| (k.as_str(), g))
        .collect();
    if let Some(p) = &chu_p {
        ung.retain(|(k, _)| k == p);
    }
    let (chu, g) = match ung.as_slice() {
        [(chu, g)] => (*chu, *g),
        _ => return None,
    };
    let than = g.detail.as_deref()?;
    let kieu = than.strip_prefix(&format!("{tu}: "))?;
    Some(format!("**{tu}**: {kieu}\n\ntrường của `{chu}`"))
}

/// **VD-270**: hover **phương thức** — `u.lay`, `self.lay`, hoặc đúng tên khai báo trong
/// `impl`/`trait`. Thứ tự chọn: (1) từ nằm trong **span khai báo** của chính phương thức; (2) lọc
/// theo kiểu tĩnh của tên trước dấu chấm; (3) không rõ chủ ⇒ chỉ trả khi tên **duy nhất** trên mọi
/// kiểu/trait (trùng tên ⇒ `None`, không đoán).
fn hover_phuong(kq: &KetQua, start: usize, tu: &str) -> Option<String> {
    let ung: Vec<&Phuong> = kq.phuong.iter().filter(|p| p.ten == tu).collect();
    let tai_khai_bao: Vec<&Phuong> = ung
        .iter()
        .copied()
        .filter(|p| start >= p.span.start && start < p.span.end)
        .collect();
    let chon = match tai_khai_bao.as_slice() {
        [p] => *p,
        _ => {
            // `self.m` trong `impl` ⇒ chủ là kiểu của khối `impl` chứa con trỏ.
            let chu_p = chu_tinh(kq, start);
            let mut loc = ung;
            if let Some(chu) = &chu_p {
                loc.retain(|p| &p.chu == chu);
            }
            match loc.as_slice() {
                [p] => *p,
                _ => return None,
            }
        }
    };
    Some(format!(
        "```vis\n{}\n```\n\nphương thức của `{}`",
        chon.chu_ky, chon.chu
    ))
}

/// **VD-270**: hover **phương thức lõi** dựng sẵn — `xs.get`, `s.upper`, `m.get`, và các sugar
/// `std.list/map/str.*` (R5c). Chỉ tra khi từ nằm sau dấu chấm và người nhận có **kiểu tĩnh** trong
/// `kq.bien`; không rõ kiểu/tên ⇒ `None` (không đoán). Bảng/format nằm ở [`super::phuong_loi`].
fn hover_phuong_loi(kq: &KetQua, start: usize, tu: &str) -> Option<String> {
    let kieu = kieu_truoc_cham(kq, &kq.text, start)?;
    super::phuong_loi::hover(&kieu, tu)
}

/// **ST-274**: hover biến thể enum / trường từ cây ký hiệu (bổ sung cho `hover_truong`).
fn hover_ky_hieu(ds: &[super::ky_hieu::KyHieu], tu: &str, parent: Option<&str>) -> Option<String> {
    for k in ds {
        if k.ten == tu {
            match k.kind {
                22 => {
                    return Some(format!(
                        "**{tu}**: biến thể của `{}`",
                        parent.unwrap_or("?")
                    ))
                }
                8 => return Some(format!("**{tu}**: trường của `{}`", parent.unwrap_or("?"))),
                _ => {}
            }
        }
        if let Some(v) = hover_ky_hieu(&k.con, tu, Some(&k.ten)) {
            return Some(v);
        }
    }
    None
}

/// Giá trị hover (markdown) cho từ tại `byte` + **span** của từ; `None` ⇒ `result:null`.
fn gia_tri_hover(kq: &KetQua, byte: usize) -> Option<(Span, String)> {
    let (start, end) = tu_tai(&kq.text, byte);
    if start == end {
        return None;
    }
    let tu = &kq.text[start..end];
    let value = if let Some(it) = kq.items.iter().find(|g| g.label == tu) {
        let than = it.detail.clone().unwrap_or_else(|| it.label.clone());
        format!("```vis\n{than}\n```")
    } else if let Some((_ten, kieu)) = kq.bien.iter().find(|(ten, _)| ten == tu) {
        format!("```vis\n{tu}: {kieu}\n```")
    } else if let Some(v) = hover_truong(kq, &kq.text, start, tu) {
        v
    } else if let Some(v) = hover_ky_hieu(&kq.ky_hieu, tu, None) {
        v
    } else if BUILTIN_VALUE_NAMES.contains(&tu) {
        format!("```vis\n<builtin {tu}>\n```")
    } else if KEYWORDS.binary_search(&tu).is_ok() {
        format!("từ khoá: {tu}")
    } else if let Some(v) = hover_phuong(kq, start, tu) {
        // **VD-270** — phương thức người dùng thắng fallback lõi.
        v
    } else {
        // **VD-270** — cuối cùng: phương thức lõi (`list`/`map`/`str`); không có ⇒ `null`.
        hover_phuong_loi(kq, start, tu)?
    };
    Some((Span::new(start, end), value))
}

/// Phản hồi `textDocument/completion` (result = mảng `CompletionItem`).
pub fn tra_loi_completion(kq: &KetQua, byte: usize, id: &Value) -> Vec<u8> {
    let items: Vec<Value> = ds_goi_y(kq, byte)
        .into_iter()
        .map(|g| {
            let mut kv = vec![("label", s(&g.label)), ("kind", Value::Int(g.kind as i64))];
            if let Some(d) = g.detail {
                kv.push(("detail", s(&d)));
            }
            m(kv)
        })
        .collect();
    phan_hoi(id, Value::List(items))
}

/// Phản hồi `textDocument/hover` (`contents` markdown + `range`), hoặc `result:null`.
pub fn tra_loi_hover(kq: &KetQua, byte: usize, id: &Value) -> Vec<u8> {
    let Some((span, value)) = gia_tri_hover(kq, byte) else {
        return phan_hoi(id, Value::none());
    };
    let ((l0, c0), (l1, c1)) = khoang(&kq.text, span);
    let pos = |line: u32, character: u32| {
        m(vec![
            ("line", Value::Int(line as i64)),
            ("character", Value::Int(character as i64)),
        ])
    };
    phan_hoi(
        id,
        m(vec![
            (
                "contents",
                m(vec![("kind", s("markdown")), ("value", s(&value))]),
            ),
            (
                "range",
                m(vec![("start", pos(l0, c0)), ("end", pos(l1, c1))]),
            ),
        ]),
    )
}
