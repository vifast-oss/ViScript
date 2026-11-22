// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — `BUILTIN_FUNCS` (builtin trần `panic`).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub const BUILTIN_FUNCS: [StdFn; 1] = [StdFn {
    name: "panic",
    // Nhận `str`; bản ghi dựng sẵn `Error` cũng được nhận ở checker/runtime (VD-267 —
    // payload `AnyError` của thiết kế §6.4), nhưng `Ty` không có union nên chữ ký ghi `str`.
    params: &[Ty::Str],
    ret: Ty::Never,
}];
