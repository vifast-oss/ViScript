// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — API tra cứu chữ ký (`sig_of`/`sig_builtin`/`la_std`/`hoan`/…).

use super::bang::FUNCS;
use super::builtin::BUILTIN_FUNCS;
use super::dinh_nghia::{StdFn, MODULES};

/// Tra chữ ký **builtin trần** theo tên (`panic`).
pub fn sig_builtin(name: &str) -> Option<&'static StdFn> {
    BUILTIN_FUNCS.iter().find(|f| f.name == name)
}

/// Tên **đã biết nhưng chưa hỗ trợ** ở VS1c — dùng để báo `VS0301` với thông điệp riêng.
/// (ST-33: `json.decode` **đã có** ⇒ bảng **rỗng**; giữ API cho các tên hoãn sau này.)
pub const HA_CHUA_HO_TRO: [&str; 0] = [];

/// Tra chữ ký theo tên đầy đủ.
pub fn sig_of(name: &str) -> Option<&'static StdFn> {
    FUNCS.iter().find(|f| f.name == name)
}

/// `true` nếu `name` là hàm `std.*` đã cấp.
pub fn la_std(name: &str) -> bool {
    sig_of(name).is_some()
}

/// `true` nếu `name` là tên std **đã biết nhưng hoãn** (báo thông điệp riêng).
pub fn hoan(name: &str) -> bool {
    HA_CHUA_HO_TRO.contains(&name)
}

/// `true` nếu `m` là module std được hỗ trợ.
pub fn module_ho_tro(m: &str) -> bool {
    MODULES.contains(&m)
}

/// **VD-270 (path.normalize)** — module đích của sugar R5c khi tên **không** thuộc module lõi
/// của kiểu nhận: `s.normalize()` ≡ `path.normalize(s)` (đường dẫn POSIX là chuỗi). Chỉ trả
/// `Some` cho cặp đã được duyệt; caller giữ module mặc định khi `None`.
pub fn module_sugar(module: &str, ten: &str) -> Option<&'static str> {
    match (module, ten) {
        ("str", "normalize") => Some("path"),
        _ => None,
    }
}

/// Mọi hàm thuộc module `m` (tên đầy đủ), theo thứ tự khai.
pub fn funcs_cua(m: &str) -> impl Iterator<Item = &'static StdFn> {
    let tien = format!("{m}.");
    FUNCS.iter().filter(move |f| f.name.starts_with(&tien))
}
