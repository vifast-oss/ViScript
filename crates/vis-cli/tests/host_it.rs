// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Host FS/path/walk/canonicalize + `--fs-mock`.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "host_it/std_fs_json.rs"]
mod std_fs_json;
#[path = "host_it/vd266_fs.rs"]
mod vd266_fs;
#[path = "host_it/vd268_fswalk.rs"]
mod vd268_fswalk;
#[path = "host_it/vd268_glob.rs"]
mod vd268_glob;
#[path = "host_it/vd269_fscopy.rs"]
mod vd269_fscopy;
#[path = "host_it/vd270_fs_canonicalize.rs"]
mod vd270_fs_canonicalize;
#[path = "host_it/vd270_fs_current_dir.rs"]
mod vd270_fs_current_dir;
#[path = "host_it/vd270_fs_is_symlink.rs"]
mod vd270_fs_is_symlink;
#[path = "host_it/vd270_fs_read_link.rs"]
mod vd270_fs_read_link;
#[path = "host_it/vd270_fsbytes.rs"]
mod vd270_fsbytes;
#[path = "host_it/vd270_fsmeta.rs"]
mod vd270_fsmeta;
#[path = "host_it/vd270_mkdir_all.rs"]
mod vd270_mkdir_all;
#[path = "host_it/vd270_path_ancestors.rs"]
mod vd270_path_ancestors;
#[path = "host_it/vd270_path_components.rs"]
mod vd270_path_components;
#[path = "host_it/vd270_path_is_absolute.rs"]
mod vd270_path_is_absolute;
#[path = "host_it/vd270_path_is_relative.rs"]
mod vd270_path_is_relative;
#[path = "host_it/vd270_path_join_all.rs"]
mod vd270_path_join_all;
#[path = "host_it/vd270_path_normalize.rs"]
mod vd270_path_normalize;
#[path = "host_it/vd270_path_relative.rs"]
mod vd270_path_relative;
#[path = "host_it/vd270_path_strip_prefix.rs"]
mod vd270_path_strip_prefix;
#[path = "host_it/vd270_path_with_extension.rs"]
mod vd270_path_with_extension;
#[path = "host_it/vd270_windows.rs"]
mod vd270_windows;
#[path = "host_it/vd270_with_file_name.rs"]
mod vd270_with_file_name;
#[path = "host_it/vd270_with_stem.rs"]
mod vd270_with_stem;
