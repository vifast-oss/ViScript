// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-67 (VD-102)** — dựng/giải JSON-RPC 2.0 trên [`Value`] của `vis-interp`.
//!
//! **Không** thêm crate: dùng lại `vis_core::json::decode` và `vis_core::json::json`
//! (cùng encoder với `json.encode` của ngôn ngữ ⇒ snapshot tất định, thứ tự khoá = thứ tự chèn).

use vis_core::json::json;
use vis_core::value::Value;

/// Dựng `Value::Map` từ danh sách khoá–giá trị (giữ thứ tự).
pub fn m(kv: Vec<(&str, Value)>) -> Value {
    Value::Map(
        kv.into_iter()
            .map(|(k, v)| (Value::Str(k.to_string()), v))
            .collect(),
    )
}

/// `Value::Str` ngắn gọn.
pub fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

/// Một cặp khoá–giá trị của `Value::Map` (giữ **thứ tự chèn** — encoder tôn trọng thứ tự này).
pub fn kv(k: &str, v: Value) -> (Value, Value) {
    (Value::Str(k.to_string()), v)
}

/// Mã hoá compact (không pretty) — đúng byte mà frame mang.
pub fn so_json(v: &Value) -> String {
    let mut s = String::new();
    json(v, false, &mut s);
    s
}

/// `id` của lỗi "không xác định được request" — JSON `null`.
pub fn id_null() -> Value {
    Value::none()
}

/// Phản hồi thành công: `{"jsonrpc":"2.0","id":<id>,"result":<result>}`.
pub fn phan_hoi(id: &Value, result: Value) -> Vec<u8> {
    let m = Value::Map(vec![
        kv("jsonrpc", Value::Str("2.0".to_string())),
        kv("id", id.clone()),
        kv("result", result),
    ]);
    so_json(&m).into_bytes()
}

/// Phản hồi lỗi: `{"jsonrpc":"2.0","id":<id>,"error":{"code":…,"message":…,"data":…}}`.
pub fn loi(id: &Value, code: i64, message: &str, data: Value) -> Vec<u8> {
    let err = Value::Map(vec![
        kv("code", Value::Int(code)),
        kv("message", Value::Str(message.to_string())),
        kv("data", data),
    ]);
    let m = Value::Map(vec![
        kv("jsonrpc", Value::Str("2.0".to_string())),
        kv("id", id.clone()),
        kv("error", err),
    ]);
    so_json(&m).into_bytes()
}

/// Notification (không `id`): `{"jsonrpc":"2.0","method":…,"params":…}`.
pub fn thong_bao(method: &str, params: Value) -> Vec<u8> {
    let m = Value::Map(vec![
        kv("jsonrpc", Value::Str("2.0".to_string())),
        kv("method", Value::Str(method.to_string())),
        kv("params", params),
    ]);
    so_json(&m).into_bytes()
}

/// Lấy trường `khoa` của object JSON (map giữ thứ tự) — `None` nếu không phải map/không có khoá.
pub fn lay<'a>(kv: &'a [(Value, Value)], khoa: &str) -> Option<&'a Value> {
    kv.iter().find_map(|(k, v)| match k {
        Value::Str(s) if s == khoa => Some(v),
        _ => None,
    })
}

/// Lấy chuỗi từ một `Value` (chỉ `Value::Str`).
pub fn chuoi(v: &Value) -> Option<&str> {
    match v {
        Value::Str(s) => Some(s),
        _ => None,
    }
}

/// `params` của request.
pub fn lay_params(kv: &[(Value, Value)]) -> Option<&Value> {
    lay(kv, "params")
}

/// `params.textDocument` (object).
pub fn text_doc(kv: &[(Value, Value)]) -> Option<&[(Value, Value)]> {
    let p = lay_params(kv)?;
    let Value::Map(p) = p else {
        return None;
    };
    let td = lay(p, "textDocument")?;
    let Value::Map(td) = td else {
        return None;
    };
    Some(td)
}

/// `params.textDocument.uri`.
pub fn uri_cua(kv: &[(Value, Value)]) -> Option<String> {
    chuoi(lay(text_doc(kv)?, "uri")?).map(str::to_string)
}

/// `didOpen`: `(uri, text)`.
pub fn van_ban_mo(kv: &[(Value, Value)]) -> Option<(String, String)> {
    let td = text_doc(kv)?;
    let uri = chuoi(lay(td, "uri")?)?.to_string();
    let text = chuoi(lay(td, "text")?)?.to_string();
    Some((uri, text))
}

/// Một `contentChanges[]` của `didChange`: `range = None` ⇒ thay **toàn** tài liệu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThayDoi {
    pub range: Option<((u32, u32), (u32, u32))>,
    pub text: String,
}

fn vi_tri_cua(v: &Value) -> Option<(u32, u32)> {
    let Value::Map(kv) = v else {
        return None;
    };
    Some((
        so_nguyen(lay(kv, "line")?)?,
        so_nguyen(lay(kv, "character")?)?,
    ))
}

/// `didChange` **incremental/full**: `(uri, contentChanges)` — giữ **mọi** thay đổi theo thứ tự.
///
/// `range` vắng/null ⇒ full replace; có `range` ⇒ áp lên trạng thái sau thay đổi trước (LSP).
pub fn thay_doi_van_ban(kv: &[(Value, Value)]) -> Option<(String, Vec<ThayDoi>)> {
    let uri = uri_cua(kv)?;
    let p = lay_params(kv)?;
    let Value::Map(p) = p else {
        return None;
    };
    let Value::List(xs) = lay(p, "contentChanges")? else {
        return None;
    };
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        let Value::Map(c) = x else {
            return None;
        };
        let text = chuoi(lay(c, "text")?)?.to_string();
        let range = match lay(c, "range") {
            None => None,
            Some(v) if v.is_none() => None,
            Some(Value::Map(r)) => {
                Some((vi_tri_cua(lay(r, "start")?)?, vi_tri_cua(lay(r, "end")?)?))
            }
            // `range` sai kiểu: từ chối cả sự kiện thay vì đoán thành full replace.
            Some(_) => return None,
        };
        out.push(ThayDoi { range, text });
    }
    Some((uri, out))
}

fn so_nguyen(v: &Value) -> Option<u32> {
    match v {
        Value::Int(n) if *n >= 0 => Some(*n as u32),
        _ => None,
    }
}

/// `params.textDocument.uri` + `params.positions[].{line,character}` (0-based, giữ thứ tự).
pub fn vi_tri_nhieu(kv: &[(Value, Value)]) -> Option<(String, Vec<(u32, u32)>)> {
    let uri = uri_cua(kv)?;
    let p = lay_params(kv)?;
    let Value::Map(p) = p else {
        return None;
    };
    let Value::List(xs) = lay(p, "positions")? else {
        return None;
    };
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        let Value::Map(pos) = x else {
            return None;
        };
        out.push((
            so_nguyen(lay(pos, "line")?)?,
            so_nguyen(lay(pos, "character")?)?,
        ));
    }
    Some((uri, out))
}

/// `params.textDocument.uri` + `params.position.{line,character}` (0-based).
pub fn vi_tri_van_ban(kv: &[(Value, Value)]) -> Option<(String, u32, u32)> {
    let uri = uri_cua(kv)?;
    let p = lay_params(kv)?;
    let Value::Map(p) = p else {
        return None;
    };
    let Value::Map(pos) = lay(p, "position")? else {
        return None;
    };
    Some((
        uri,
        so_nguyen(lay(pos, "line")?)?,
        so_nguyen(lay(pos, "character")?)?,
    ))
}
