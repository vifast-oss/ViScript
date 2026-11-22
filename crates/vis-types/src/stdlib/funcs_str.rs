// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — bảng chữ ký `std.*` nhóm `funcs_str` (tách khỏi `stdlib.rs`, 0 đổi hành vi).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub(super) const FUNCS_STR: [StdFn; 48] = [
    StdFn {
        name: "str.split",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.split_once",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptListStr,
    },
    StdFn {
        name: "str.rsplit_once",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptListStr,
    },
    StdFn {
        name: "str.split_inclusive",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.split_terminator",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.splitn",
        params: &[Ty::Str, Ty::Str, Ty::Int],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.rsplit",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.lines",
        params: &[Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.split_whitespace",
        params: &[Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.join",
        params: &[Ty::ListStr, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.upper",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.lower",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.title",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.starts_with",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.ends_with",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.contains",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.find",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "str.rfind",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "str.count",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Int,
    },
    StdFn {
        name: "str.matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "str.strip_prefix",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptStr,
    },
    StdFn {
        name: "str.strip_suffix",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptStr,
    },
    StdFn {
        name: "str.repeat",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.replace",
        params: &[Ty::Str, Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.replace_first",
        params: &[Ty::Str, Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.replace_last",
        params: &[Ty::Str, Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.replace_n",
        params: &[Ty::Str, Ty::Str, Ty::Str, Ty::Int],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_start",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_end",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_start_matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.trim_end_matches",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.pad_left",
        params: &[Ty::Str, Ty::Int, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.pad_right",
        params: &[Ty::Str, Ty::Int, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.center",
        params: &[Ty::Str, Ty::Int, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "str.parse_int",
        params: &[Ty::Str],
        ret: Ty::OptInt,
    },
    StdFn {
        name: "str.parse_float",
        params: &[Ty::Str],
        ret: Ty::OptFloat,
    },
    StdFn {
        name: "str.byte_len",
        params: &[Ty::Str],
        ret: Ty::Int,
    },
    StdFn {
        name: "str.char_count",
        params: &[Ty::Str],
        ret: Ty::Int,
    },
    StdFn {
        name: "str.char_at",
        params: &[Ty::Str, Ty::Int],
        ret: Ty::OptStr,
    },
    StdFn {
        name: "str.is_empty",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.is_ascii",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.is_ascii_digit",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.is_ascii_alphabetic",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.is_ascii_alphanumeric",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.is_ascii_whitespace",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "str.is_blank",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
];
