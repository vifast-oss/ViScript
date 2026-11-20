// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-270** — `textDocument/documentSymbol` cho LSP.
//!
//! Cây ký hiệu theo HIR: item cấp tệp (fn/type/enum/alias/const/trait/extern) + trường/variant/
//! phương thức. `range` phủ cả khai báo; `selectionRange` thu về **đúng tên** (client chọn tên).
//! Không có HIR (tệp chưa mở/hỏng) ⇒ mảng rỗng.

use vis_interp::value::Value;
use vis_syntax::Span;
use vis_types::hir::{HField, HItem, Hir};

use super::dinh_nghia::thu_hep_ten;
use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::khoang;

/// Một ký hiệu (đệ quy) — đủ để dựng `DocumentSymbol`.
#[derive(Clone)]
pub struct KyHieu {
    pub ten: String,
    pub kind: u8,
    pub span: Span,
    pub con: Vec<KyHieu>,
}

fn kh(ten: &str, kind: u8, span: Span) -> KyHieu {
    KyHieu {
        ten: ten.to_string(),
        kind,
        span,
        con: Vec::new(),
    }
}

fn truong(f: &HField) -> KyHieu {
    kh(&f.name, 8, f.span) // Field
}

/// Gom cây ký hiệu từ HIR (giữ thứ tự khai).
pub fn gom(hir: &Hir) -> Vec<KyHieu> {
    let mut out = Vec::new();
    for it in &hir.items {
        match it {
            HItem::Fn(f) => out.push(kh(&f.name, 12, f.span)), // Function
            HItem::Rec(r) => {
                let mut k = kh(&r.name, 23, r.span); // Struct
                k.con = r.fields.iter().map(truong).collect();
                out.push(k);
            }
            HItem::Enum(e) => {
                let mut k = kh(&e.name, 10, e.span); // Enum
                k.con = e
                    .variants
                    .iter()
                    .map(|v| {
                        let mut k = kh(&v.name, 22, v.span); // EnumMember
                        k.con = v.fields.iter().map(truong).collect();
                        k
                    })
                    .collect();
                out.push(k);
            }
            HItem::Alias(a) => out.push(kh(&a.name, 26, a.span)), // TypeParameter
            HItem::Const(c) => out.push(kh(&c.name, 14, c.span)), // Constant
            HItem::Trait(t) => {
                let mut k = kh(&t.name, 11, t.span); // Interface
                k.con = t.methods.iter().map(|f| kh(&f.name, 6, f.span)).collect();
                out.push(k);
            }
            HItem::Impl(i) => {
                out.extend(i.methods.iter().map(|f| kh(&f.name, 6, f.span))); // Method
            }
            HItem::Extern(x) => out.push(kh(&x.name, 12, x.span)),
            HItem::Use(_) => {}
        }
    }
    out
}

fn vi_tri(text: &str, span: Span) -> Value {
    let ((l0, c0), (l1, c1)) = khoang(text, span);
    m(vec![
        (
            "start",
            m(vec![
                ("line", Value::Int(l0 as i64)),
                ("character", Value::Int(c0 as i64)),
            ]),
        ),
        (
            "end",
            m(vec![
                ("line", Value::Int(l1 as i64)),
                ("character", Value::Int(c1 as i64)),
            ]),
        ),
    ])
}

fn thanh_value(text: &str, k: &KyHieu) -> Value {
    let sel = thu_hep_ten(text, &k.ten, k.span);
    let mut kv = vec![
        ("name", s(&k.ten)),
        ("kind", Value::Int(k.kind as i64)),
        ("range", vi_tri(text, k.span)),
        ("selectionRange", vi_tri(text, sel)),
    ];
    if !k.con.is_empty() {
        kv.push((
            "children",
            Value::List(k.con.iter().map(|c| thanh_value(text, c)).collect()),
        ));
    }
    m(kv)
}

/// Phản hồi `textDocument/documentSymbol` (mảng `DocumentSymbol`).
pub fn tra_loi_document_symbol(kq: &KetQua, id: &Value) -> Vec<u8> {
    let ds: Vec<Value> = kq
        .ky_hieu
        .iter()
        .map(|k| thanh_value(&kq.text, k))
        .collect();
    phan_hoi(id, Value::List(ds))
}
