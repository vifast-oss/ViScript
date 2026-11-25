// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — dispatch `std.path.*` (thuần chuỗi, không chạm đĩa), tách khỏi
//! `stdlib.rs` để giữ tệp dưới 1.150 dòng. Chuyển **nguyên văn**, không đổi ngữ nghĩa.
//!
//! **ST-314a (VD-290)**: nhận `StdId` thay tên chuỗi — nhánh dispatch là biến thể enum nên
//! sai tên/thiếu nhánh bị Rust bắt lúc biên dịch, không còn `match name`.

use vis_hir::stdlib::StdId;
use vis_syntax::Span;

use crate::builtin::ls::{
    basename_voi, dirname_voi, ext_voi, la_tuyet_doi, noi_duong_voi, stem_voi,
};
use vis_core::trap::R;
use vis_core::value::Value;

use super::super::chung::chuoi;
use super::ext::{pathwithext, pathwithfilename, pathwithstem};
use super::join::{pathancestors, pathcomponents, pathisabs, pathjoinall};
use super::normalize::{pathnorm, pathrel, pathrelbool, pathstripprefix};

/// Gọi một hàm `path.*` theo `StdId`.
pub(crate) fn goi(id: StdId, args: &[Value], span: Span) -> R<Value> {
    let name = id.as_str();
    match id {
        // ----- std.path (thuần chuỗi, không chạm đĩa) -----
        // **VD-258**: Windows nhận ổ đĩa (`C:\`, `C:/`, UNC) và cả `\`/`/`; POSIX giữ nguyên (`builtin_ls`).
        StdId::PathJoin => {
            let a = chuoi(name, &args[0], 0, span)?;
            let b = chuoi(name, &args[1], 1, span)?;
            Ok(Value::Str(noi_duong_voi(a, b, cfg!(windows))))
        }
        // **VD-270 (path.join_all)**: nối `list[str]` trái→phải bằng **đúng** quy tắc
        // `path.join` (gấp trái, `[]` ⇒ `""`).
        StdId::PathJoinAll => pathjoinall::goi(args, span),
        // **VD-270 (path.ancestors)**: chuỗi **tổ tiên từ vựng** POSIX gồm chính `p`; chỉ bỏ
        // `/` cuối, giữ `.`/`..`, gốc `/` là cuối (không thêm `""`) — thuần chuỗi.
        StdId::PathAncestors => pathancestors::goi(args, span),
        // **VD-270 (path.components)**: tách **từ vựng** POSIX thành thành phần; tuyệt đối ⇒
        // phần tử đầu `"/"`, bỏ thành phần rỗng, giữ nguyên `.`/`..` (không chuẩn hoá).
        StdId::PathComponents => pathcomponents::goi(args, span),
        StdId::PathBasename | StdId::PathDirname | StdId::PathExt | StdId::PathStem => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(match id {
                StdId::PathBasename => basename_voi(p, cfg!(windows)),
                StdId::PathDirname => dirname_voi(p, cfg!(windows)),
                StdId::PathExt => ext_voi(p, cfg!(windows)),
                _ => stem_voi(p, cfg!(windows)),
            }))
        }
        StdId::PathIsAbs => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Bool(la_tuyet_doi(p, cfg!(windows))))
        }
        // **VD-270 (path.is_absolute)**: cùng quy tắc `path.is_abs` (không đĩa/`cwd`).
        StdId::PathIsAbsolute => pathisabs::goi(args, span),
        // **VD-270 (path.is_relative)**: phần bù **từ vựng** của `path.is_abs` (không đĩa/`cwd`).
        StdId::PathIsRelative => pathrelbool::goi(args, span),
        // **VD-270 (path.normalize)**: chuẩn hoá **từ vựng** POSIX (thuần chuỗi, không chạm đĩa).
        StdId::PathNormalize => pathnorm::goi(args, span),
        // **VD-270 (path.relative)**: đường dẫn tương đối **thuần từ vựng** (không `cwd`/đĩa).
        StdId::PathRelative => pathrel::goi(args, span),
        // **VD-270 (path.strip_prefix)**: bỏ tiền tố **theo thành phần** (thuần từ vựng POSIX;
        // phần còn lại **nguyên văn**, bỏ đúng một `/`; không chuẩn hoá `.`/`..`).
        StdId::PathStripPrefix => pathstripprefix::goi(args, span),
        // **VD-270 (path.with_extension)**: thay/đặt/bỏ phần mở rộng của thành phần cuối
        // (thuần chuỗi; dùng **đúng** `ext_voi`/`stem_voi` của `path.ext`/`path.stem`).
        StdId::PathWithExtension => pathwithext::goi(args, span),
        // **VD-270 (path.with_file_name)**: thay **toàn bộ** thành phần cuối (tên tệp kể cả phần
        // mở rộng) bằng `name` **nguyên văn**, giữ tiền tố nguyên văn (thuần chuỗi).
        StdId::PathWithFileName => pathwithfilename::goi(args, span),
        // **VD-270 (path.with_stem)**: thay **phần thân** (file stem) của thành phần cuối, giữ
        // nguyên phần mở rộng hiện có (thuần chuỗi; dùng **đúng** `stem_voi`/`ext_voi`).
        StdId::PathWithStem => pathwithstem::goi(args, span),
        _ => unreachable!("path_d::goi chỉ nhận StdId::Path*"),
    }
}
