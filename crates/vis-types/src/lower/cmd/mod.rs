// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cmd_lower` — hạ **CST chế độ lệnh** (`.vish`, khối `cmd { … }`) xuống **cùng HIR** của script
//! (ST-18, VD-22 pha 2). Nguyên tắc: `cmd` là **hàm** (`FnKind::Cmd`) nên mọi thứ chạy qua máy interp
//! sẵn có; ở đây chỉ thêm **bảng chữ ký** + **khe ống** + 4 mã `06xx`.
//!
//! **ST-318c** — tách tệp theo trách nhiệm (0 đổi hành vi/API): `text` (văn bản/kiểu literal),
//! `stage` (`ha_stage`), `complete` (stage `| complete`), `args` (dựng đối số cuối stage), `doc`/`hop` (đã tách từ ST-106).

use super::*;

mod args;
mod complete;
mod doc;
mod hop;
mod stage;
mod text;

pub(super) fn ten_kieu(t: &Type) -> String {
    match t {
        Type::Int => "int".into(),
        Type::Float => "float".into(),
        Type::Size => "size".into(),
        Type::Duration => "duration".into(),
        Type::Percent => "percent".into(),
        Type::Str => "str".into(),
        Type::Bool => "bool".into(),
        Type::Bytes => "bytes".into(),
        Type::Path => "path".into(),
        Type::Char => "char".into(),
        Type::Unit => "unit".into(),
        Type::List(x) => format!("list[{}]", ten_kieu(x)),
        Type::Map(k, v) => format!("map[{}, {}]", ten_kieu(k), ten_kieu(v)),
        Type::Opt(x) => format!("{}?", ten_kieu(x)),
        Type::Result(a, b) => format!("Result[{}, {}]", ten_kieu(a), ten_kieu(b)),
        other => format!("{other:?}").to_lowercase(),
    }
}
