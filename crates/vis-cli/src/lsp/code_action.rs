// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `textDocument/codeAction` cho LSP.
//!
//! Phạm vi: **một** action toàn tài liệu `source.fixAll` — "Định dạng tài liệu" — chứa bản đã
//! format của **đúng** formatter CLI (`vis_fmt::format_mode`, cùng hàm `textDocument/formatting`
//! dùng) theo chế độ script/command suy từ đuôi URI. Tài liệu **đã đúng** format (formatter là
//! điểm bất động) hoặc lỗi cú pháp ⇒ `[]`, không bịa bản sửa. Tài liệu **rỗng** đi theo đúng
//! formatter: `vis fmt` thêm newline cuối ⇒ action `newText: "\n"`; tệp `"\n"` ⇒ `[]`.
//!
//! `range` và `context` của request (kể cả `context.only`) **được bỏ qua có chủ đích**: action là
//! full-document nên không phụ thuộc vùng chọn/lý do client hỏi. URI chưa mở/URI lạ do dispatch
//! trả lỗi chuẩn `-32602` (InvalidParams), giống `semanticTokens/full`.

use vis_interp::value::Value;
use vis_syntax::Mode;

use super::dinh_dang::edit_toan_tep;
use super::jsonrpc::{kv, m, phan_hoi, s};
use super::phan_tich::KetQua;

/// Tiêu đề action (người dùng thấy trong menu "Quick Fix"/"Source Action").
const TIEU_DE: &str = "Định dạng tài liệu";
/// `CodeActionKind` của LSP — action sửa cả tệp.
const LOAI: &str = "source.fixAll";

/// `codeActionProvider` của `initialize`: chỉ quảng cáo đúng loại action mình sinh.
pub fn provider() -> Value {
    m(vec![("codeActionKinds", Value::List(vec![s(LOAI)]))])
}

/// Phản hồi `textDocument/codeAction` (`CodeAction[]`).
pub fn tra_loi(kq: &KetQua, uri: &str, mode: Mode, id: &Value) -> Vec<u8> {
    let Some(edit) = edit_toan_tep(kq, mode) else {
        return phan_hoi(id, Value::List(Vec::new()));
    };
    let action = m(vec![
        ("title", s(TIEU_DE)),
        ("kind", s(LOAI)),
        (
            "edit",
            m(vec![(
                "changes",
                Value::Map(vec![kv(uri, Value::List(vec![edit]))]),
            )]),
        ),
    ]);
    phan_hoi(id, Value::List(vec![action]))
}
