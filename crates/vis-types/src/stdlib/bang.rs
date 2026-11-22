// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — ghép các bảng chữ ký nhóm thành `FUNCS` (một nguồn, đúng thứ tự cũ).

use super::dinh_nghia::StdFn;
use super::funcs_fs_path::FUNCS_FS_PATH;
use super::funcs_list_map::FUNCS_LIST_MAP;
use super::funcs_misc::FUNCS_MISC;
use super::funcs_net_http::FUNCS_NET_HTTP;
use super::funcs_sqlite::FUNCS_SQLITE;
use super::funcs_str::FUNCS_STR;
use super::ty::Ty;

/// Ghép các mảng const thành một mảng tại **const-eval** (không cấp phát runtime).
macro_rules! noi_bang {
    ($($m:ident),+ $(,)?) => {{
        const TONG: usize = 0 $(+ $m.len())+;
        let mut out = [StdFn { name: "", params: &[], ret: Ty::Int }; TONG];
        let mut i = 0usize;
        $(
            let mut j = 0usize;
            while j < $m.len() {
                out[i] = $m[j];
                i += 1;
                j += 1;
            }
        )+
        out
    }};
}

/// Toàn bộ hàm `std.*` (167).
pub const FUNCS: [StdFn; 167] = noi_bang!(
    FUNCS_FS_PATH,
    FUNCS_LIST_MAP,
    FUNCS_MISC,
    FUNCS_NET_HTTP,
    FUNCS_SQLITE,
    FUNCS_STR
);
