// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — literal, tên dựng sẵn và định dạng `{expr:.N}`, tách khỏi
//! `interp.rs`. Chuyển **nguyên văn**, không đổi ngữ nghĩa.

use vis_core::trap::{Abort, Trap};
use vis_core::value::{
    parse_bytes_lit, parse_char_lit, parse_duration, parse_percent, parse_size, parse_str_lit,
    Value,
};
use vis_hir::hir::Lit;
use vis_syntax::Span;

use crate::codes::RT;
use vis_hir::hir::LitKind;

/// Đọc literal thành `Value` (lỗi → thông điệp cho `VS0501`).
pub fn lit_value(lit: &Lit) -> std::result::Result<Value, String> {
    match lit.kind {
        LitKind::Int => vis_core::value::parse_int_lit(&lit.text)
            .map(Value::Int)
            .ok_or_else(|| format!("literal int không đọc được: `{}`", lit.text)),
        LitKind::Float => lit
            .text
            .replace('_', "")
            .parse::<f64>()
            .map(Value::Float)
            .map_err(|_| format!("literal float không đọc được: `{}`", lit.text)),
        LitKind::Bool => match lit.text.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            other => Err(format!("literal bool lạ: `{other}`")),
        },
        LitKind::Char => parse_char_lit(&lit.text)
            .map(Value::Char)
            .ok_or_else(|| format!("literal char không đọc được: `{}`", lit.text)),
        LitKind::Str => Ok(Value::Str(parse_str_lit(&lit.text, false))),
        LitKind::Bytes => parse_bytes_lit(&lit.text)
            .map(Value::Bytes)
            .ok_or_else(|| format!("literal bytes không đọc được: `{}`", lit.text)),
        // **R6a (VD-178)**: `p"…"` **không** escape (đường dẫn Windows).
        LitKind::Path => Ok(Value::Path(parse_str_lit(&lit.text, true))),
        LitKind::Size => parse_size(&lit.text)
            .map(Value::Size)
            .ok_or_else(|| format!("literal size không đọc được: `{}`", lit.text)),
        LitKind::Duration => parse_duration(&lit.text)
            .map(Value::Duration)
            .ok_or_else(|| format!("literal duration không đọc được: `{}`", lit.text)),
        LitKind::Percent => parse_percent(&lit.text)
            .map(Value::Percent)
            .ok_or_else(|| format!("literal percent không đọc được: `{}`", lit.text)),
    }
}

pub const MAX_CALL_DEPTH: u32 = 2_000;

pub fn builtin_name(name: &str) -> Option<&'static str> {
    // ST-27: chương trình ngoài — `builtin::call` sẽ hỏi **host** (`None` ⇒ `VS0604`).
    if name.starts_with("exec.") {
        return Some("exec");
    }
    // ST-26: `std.<module>.<fn>` gọi qua bảng `stdlib` (dùng chung với VM, VD-55).
    if name.starts_with("std.") {
        return name
            .strip_prefix("std.")
            .and_then(vis_hir::stdlib::sig_of)
            .map(|f| f.name);
    }
    // **ST-314a hậu-kiểm (low-peak 2026-11-25)**: một nguồn duy nhất với `SIGS` (frame + VM đã
    // dùng). Trước đây danh sách này chép tay và thiếu 4 tên `select_*`, khiến interp ở đường
    // đồng bộ báo ``tên `select_recv` chưa hỗ trợ ở VS0`` trong khi VM báo đúng
    // `` `select_recv` cần đường frame test/async … `` — lệch **thông điệp** giữa hai engine.
    crate::builtin::builtin_name(name)
}
/// **R6c (VD-178)**: định dạng `{expr:spec}` — hiện hỗ trợ `.N` (N chữ số thập phân).
pub fn dinh_dang(v: &Value, spec: &str, span: Span) -> Result<String, Abort> {
    let Some(n) = spec.strip_prefix('.') else {
        return Err(Abort::Trap(Trap::new(
            RT,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N`"),
            span,
        )));
    };
    let n: usize = n.parse().map_err(|_| {
        Abort::Trap(Trap::new(
            RT,
            format!("format spec `{spec}` chưa hỗ trợ — dùng `.N`"),
            span,
        ))
    })?;
    Ok(match v {
        Value::Float(f) => format!("{f:.n$}"),
        Value::Int(i) => format!("{:.n$}", *i as f64),
        Value::Str(s) => s.clone(),
        other => other.show(),
    })
}

#[cfg(test)]
mod tests {
    use super::builtin_name;
    use crate::builtin::SIGS;

    /// **ST-314a hậu-kiểm (low-peak 2026-11-25)** — **một nguồn**: mọi tên trong `SIGS` (gồm 4
    /// `select_*`) phải được `builtin_name` nhận. Trước đây `lit.rs` chép tay 23 tên và thiếu
    /// `select_*`, khiến interp đường đồng bộ báo "tên … chưa hỗ trợ ở VS0" còn VM báo đúng
    /// "cần đường frame". Nếu ai tách danh sách lần nữa, test này đỏ.
    #[test]
    fn moi_sig_deu_la_builtin_name() {
        for s in SIGS {
            assert_eq!(
                builtin_name(s.name),
                Some(s.name),
                "thiếu builtin `{}`",
                s.name
            );
        }
        assert_eq!(builtin_name("khong_co_builtin_nay"), None);
        assert!(
            builtin_name("std.str.upper").is_some(),
            "std.* phải đi qua bảng chữ ký"
        );
    }
}
