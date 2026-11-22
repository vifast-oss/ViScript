// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: `std.*`: str/list/map/số/path thuần và sugar.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "stdlib_it/sort_by_str.rs"]
mod sort_by_str;
#[path = "stdlib_it/stdlib.rs"]
mod stdlib;
#[path = "stdlib_it/vd270_ascii_preds.rs"]
mod vd270_ascii_preds;
#[path = "stdlib_it/vd270_binary_search.rs"]
mod vd270_binary_search;
#[path = "stdlib_it/vd270_char_at.rs"]
mod vd270_char_at;
#[path = "stdlib_it/vd270_chunks_exact.rs"]
mod vd270_chunks_exact;
#[path = "stdlib_it/vd270_flatten.rs"]
mod vd270_flatten;
#[path = "stdlib_it/vd270_indexof.rs"]
mod vd270_indexof;
#[path = "stdlib_it/vd270_is_blank.rs"]
mod vd270_is_blank;
#[path = "stdlib_it/vd270_list_contains.rs"]
mod vd270_list_contains;
#[path = "stdlib_it/vd270_list_mean.rs"]
mod vd270_list_mean;
#[path = "stdlib_it/vd270_list_median.rs"]
mod vd270_list_median;
#[path = "stdlib_it/vd270_list_sum_float.rs"]
mod vd270_list_sum_float;
#[path = "stdlib_it/vd270_list_sum_product.rs"]
mod vd270_list_sum_product;
#[path = "stdlib_it/vd270_map_contains_key.rs"]
mod vd270_map_contains_key;
#[path = "stdlib_it/vd270_minmax_float.rs"]
mod vd270_minmax_float;
#[path = "stdlib_it/vd270_parse_float.rs"]
mod vd270_parse_float;
#[path = "stdlib_it/vd270_parse_int.rs"]
mod vd270_parse_int;
#[path = "stdlib_it/vd270_replace_first.rs"]
mod vd270_replace_first;
#[path = "stdlib_it/vd270_replace_last.rs"]
mod vd270_replace_last;
#[path = "stdlib_it/vd270_replace_n.rs"]
mod vd270_replace_n;
#[path = "stdlib_it/vd270_rsplit.rs"]
mod vd270_rsplit;
#[path = "stdlib_it/vd270_rsplit_once.rs"]
mod vd270_rsplit_once;
#[path = "stdlib_it/vd270_split_inclusive.rs"]
mod vd270_split_inclusive;
#[path = "stdlib_it/vd270_split_once.rs"]
mod vd270_split_once;
#[path = "stdlib_it/vd270_split_terminator.rs"]
mod vd270_split_terminator;
#[path = "stdlib_it/vd270_splitn.rs"]
mod vd270_splitn;
#[path = "stdlib_it/vd270_str_is_ascii.rs"]
mod vd270_str_is_ascii;
#[path = "stdlib_it/vd270_str_strip.rs"]
mod vd270_str_strip;
#[path = "stdlib_it/vd270_str_title.rs"]
mod vd270_str_title;
#[path = "stdlib_it/vd270_str_trim.rs"]
mod vd270_str_trim;
#[path = "stdlib_it/vd270_strfind.rs"]
mod vd270_strfind;
#[path = "stdlib_it/vd270_strlen.rs"]
mod vd270_strlen;
