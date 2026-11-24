// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — bảng chữ ký `std.*` nhóm `funcs_list_map` (tách khỏi `stdlib.rs`, 0 đổi hành vi).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub(super) const FUNCS_LIST_MAP: [StdFn; 33] = [
    StdFn {
        name: "list.is_empty",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::Bool,
    },
    StdFn {
        name: "list.sort",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.sort_by",
        params: &[Ty::List(&Ty::Var(0)), Ty::FnKhoa],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.reverse",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.unique",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.concat",
        params: &[Ty::List(&Ty::Var(0)), Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.flatten",
        params: &[Ty::List(&Ty::List(&Ty::Var(0)))],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.chunk",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    StdFn {
        name: "list.chunks_exact",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    StdFn {
        name: "list.windows",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    StdFn {
        name: "list.index_of",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "list.last_index_of",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "list.contains",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::Bool,
    },
    StdFn {
        name: "list.binary_search",
        params: &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "list.take",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.drop",
        params: &[Ty::List(&Ty::Var(0)), Ty::Int],
        ret: Ty::List(&Ty::Var(0)),
    },
    StdFn {
        name: "list.zip",
        params: &[Ty::List(&Ty::Var(0)), Ty::List(&Ty::Var(0))],
        ret: Ty::List(&Ty::List(&Ty::Var(0))),
    },
    StdFn {
        name: "list.min",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::Var(0),
    },
    StdFn {
        name: "list.max",
        params: &[Ty::List(&Ty::Var(0))],
        ret: Ty::Var(0),
    },
    StdFn {
        name: "list.sum",
        params: &[Ty::List(&Ty::Int)],
        ret: Ty::Int,
    },
    StdFn {
        name: "list.product",
        params: &[Ty::List(&Ty::Int)],
        ret: Ty::Int,
    },
    StdFn {
        name: "list.mean",
        params: &[Ty::List(&Ty::Float)],
        ret: Ty::OptFloat,
    },
    StdFn {
        name: "list.median",
        params: &[Ty::List(&Ty::Float)],
        ret: Ty::OptFloat,
    },
    StdFn {
        name: "list.sum_float",
        params: &[Ty::List(&Ty::Float)],
        ret: Ty::Float,
    },
    StdFn {
        name: "list.min_float",
        params: &[Ty::List(&Ty::Float)],
        ret: Ty::OptFloat,
    },
    StdFn {
        name: "list.max_float",
        params: &[Ty::List(&Ty::Float)],
        ret: Ty::OptFloat,
    },
    StdFn {
        name: "map.is_empty",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2))],
        ret: Ty::Bool,
    },
    StdFn {
        name: "map.contains_key",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Var(1)],
        ret: Ty::Bool,
    },
    StdFn {
        name: "map.keys",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2))],
        ret: Ty::List(&Ty::Var(1)),
    },
    StdFn {
        name: "map.values",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2))],
        ret: Ty::List(&Ty::Var(2)),
    },
    StdFn {
        name: "map.merge",
        params: &[
            Ty::Map(&Ty::Var(1), &Ty::Var(2)),
            Ty::Map(&Ty::Var(1), &Ty::Var(2)),
        ],
        ret: Ty::Map(&Ty::Var(1), &Ty::Var(2)),
    },
    StdFn {
        name: "map.get_or",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Str, Ty::Var(2)],
        ret: Ty::Var(2),
    },
    StdFn {
        name: "map.remove",
        params: &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Str],
        ret: Ty::Map(&Ty::Var(1), &Ty::Var(2)),
    },
];
