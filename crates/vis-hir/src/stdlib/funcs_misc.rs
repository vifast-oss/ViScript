// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — bảng chữ ký `std.*` nhóm `funcs_misc` (tách khỏi `stdlib.rs`, 0 đổi hành vi).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub(super) const FUNCS_MISC: [StdFn; 5] = [
    StdFn {
        name: "time.now_ms",
        params: &[],
        ret: Ty::Int,
    },
    StdFn {
        name: "time.monotonic_ms",
        params: &[],
        ret: Ty::Int,
    },
    StdFn {
        name: "json.encode",
        params: &[Ty::Var(0)],
        ret: Ty::Str,
    },
    StdFn {
        name: "json.encode_pretty",
        params: &[Ty::Var(0)],
        ret: Ty::Str,
    },
    StdFn {
        name: "json.decode",
        params: &[Ty::Str],
        ret: Ty::ResultJsonErr,
    },
];
