// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — bảng chữ ký `std.*` nhóm `funcs_fs_path` (tách khỏi `stdlib.rs`, 0 đổi hành vi).

use super::dinh_nghia::StdFn;
use super::ty::Ty;

pub(super) const FUNCS_FS_PATH: [StdFn; 39] = [
    StdFn {
        name: "path.join",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.join_all",
        params: &[Ty::ListStr],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.ancestors",
        params: &[Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "path.components",
        params: &[Ty::Str],
        ret: Ty::ListStr,
    },
    StdFn {
        name: "path.basename",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.dirname",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.ext",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.stem",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.is_abs",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "path.is_absolute",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "path.is_relative",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "path.normalize",
        params: &[Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.relative",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.strip_prefix",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::OptStr,
    },
    StdFn {
        name: "path.with_extension",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.with_file_name",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "path.with_stem",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::Str,
    },
    StdFn {
        name: "fs.read",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "fs.write",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.exists",
        params: &[Ty::Str],
        ret: Ty::Bool,
    },
    StdFn {
        name: "fs.list",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "fs.read_lines",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "fs.append",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.mkdir",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.mkdir_all",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.remove",
        params: &[Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.copy",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.current_dir",
        params: &[],
        ret: Ty::ResultStrErr,
    },
    StdFn {
        name: "fs.rename",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultUnitStr,
    },
    StdFn {
        name: "fs.walk",
        params: &[Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "fs.glob",
        params: &[Ty::Str, Ty::Str],
        ret: Ty::ResultListStrStr,
    },
    StdFn {
        name: "fs.size",
        params: &[Ty::Str],
        ret: Ty::ResultIntErr,
    },
    StdFn {
        name: "fs.is_file",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "fs.is_dir",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "fs.is_symlink",
        params: &[Ty::Str],
        ret: Ty::ResultBoolErr,
    },
    StdFn {
        name: "fs.read_link",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "fs.canonicalize",
        params: &[Ty::Str],
        ret: Ty::ResultStrStr,
    },
    StdFn {
        name: "fs.read_bytes",
        params: &[Ty::Str],
        ret: Ty::ResultListIntErr,
    },
    StdFn {
        name: "fs.write_bytes",
        params: &[Ty::Str, Ty::List(&Ty::Int)],
        ret: Ty::ResultUnitStr,
    },
];
