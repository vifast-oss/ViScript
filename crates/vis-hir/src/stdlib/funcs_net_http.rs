// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — bảng chữ ký `std.*` nhóm `funcs_net_http` (tách khỏi `stdlib.rs`, 0 đổi hành vi).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub(super) const FUNCS_NET_HTTP: [StdFn; 14] = [
    StdFn {
        name: "net.connect",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.connect_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.read",
        params: &[Ty::Str, Ty::Int, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.write",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "net.close",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "net.listen",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.local_addr",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.accept",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.accept_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "net.close_listen",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "http.get",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "http.get_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "http.head",
        params: &[Ty::Str],
        ret: Ty::ResultHead,
    },
    StdFn {
        name: "http.head_timeout",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::ResultHead,
    },
];
