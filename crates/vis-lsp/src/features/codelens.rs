// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/codeLens` cho LSP.
//!
//! Mỗi `test fn <tên>` **cấp tệp** ⇒ một lens chạy test qua command `vis.test.run`; `range` là
//! **đúng token tên** (UTF-16, 0-based). Không sinh lens cho `fn` thường, `cmd`, hay phương thức
//! `impl`/`trait`; giữ **thứ tự nguồn**, tối đa **200** lens.

use vis_core::value::Value;
use vis_hir::hir::{HItem, Hir};
use vis_hir::FnKind;
use vis_syntax::Span;

use super::jsonrpc::{m, phan_hoi, s};
use super::phan_tich::KetQua;
use super::vi_tri::khoang;

/// Tối đa số lens trong một phản hồi.
const TOI_DA: usize = 200;

/// Một `test fn` cấp tệp: tên + span **cả khai báo** (span tên tính khi dựng phản hồi).
#[derive(Clone)]
pub struct TestFn {
    pub ten: String,
    pub span: Span,
}

/// Gom `test fn` từ HIR theo thứ tự khai; bỏ `fn`/`cmd`/phương thức và tên rỗng (khai hỏng).
pub fn gom(hir: &Hir) -> Vec<TestFn> {
    hir.items
        .iter()
        .filter_map(|it| match it {
            HItem::Fn(f) if f.kind == FnKind::Test && !f.name.is_empty() => Some(TestFn {
                ten: f.name.clone(),
                span: f.span,
            }),
            _ => None,
        })
        .collect()
}

/// Span **đúng token tên** trong `test fn <tên>`: bỏ qua từ khoá `test`/`fn` nên tên trùng
/// `test` hay `fn` vẫn đúng. Không thấy tên trọn từ ⇒ trả span khai báo.
fn span_ten(text: &str, t: &TestFn) -> Span {
    let seg = &text[t.span.start..t.span.end];
    let from = seg.find("fn").map_or(0, |i| i + 2);
    let sau_fn = Span::new(t.span.start + from, t.span.end);
    let tim = super::dinh_nghia::thu_hep_ten(text, &t.ten, sau_fn);
    if tim == sau_fn {
        t.span
    } else {
        tim
    }
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

fn thanh_value(text: &str, uri: &str, t: &TestFn) -> Value {
    m(vec![
        ("range", vi_tri(text, span_ten(text, t))),
        (
            "command",
            m(vec![
                ("title", s(&format!("Chạy test {}", t.ten))),
                ("command", s("vis.test.run")),
                ("arguments", Value::list(vec![s(uri), s(&t.ten)])),
            ]),
        ),
    ])
}

/// Phản hồi `textDocument/codeLens` (mảng `CodeLens`, tối đa [`TOI_DA`]).
pub fn tra_loi_code_lens(kq: &KetQua, uri: &str, id: &Value) -> Vec<u8> {
    let ds: Vec<Value> = kq
        .test_fn
        .iter()
        .take(TOI_DA)
        .map(|t| thanh_value(&kq.text, uri, t))
        .collect();
    phan_hoi(id, Value::list(ds))
}
