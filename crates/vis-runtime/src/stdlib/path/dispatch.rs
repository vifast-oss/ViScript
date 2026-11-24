// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — dispatch `std.path.*` (thuần chuỗi, không chạm đĩa), tách khỏi
//! `stdlib.rs` để giữ tệp dưới 1.150 dòng. Chuyển **nguyên văn**, không đổi ngữ nghĩa.

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

/// Gọi một hàm `path.*` theo tên đầy đủ.
pub(crate) fn goi(name: &str, args: &[Value], span: Span) -> R<Value> {
    match name {
        // ----- std.path (thuần chuỗi, không chạm đĩa) -----
        // **VD-258**: Windows nhận ổ đĩa (`C:\`, `C:/`, UNC) và cả `\`/`/`; POSIX giữ nguyên (`builtin_ls`).
        "path.join" => {
            let a = chuoi(name, &args[0], 0, span)?;
            let b = chuoi(name, &args[1], 1, span)?;
            Ok(Value::Str(noi_duong_voi(a, b, cfg!(windows))))
        }
        // **VD-270 (path.join_all)**: nối `list[str]` trái→phải bằng **đúng** quy tắc
        // `path.join` (gấp trái, `[]` ⇒ `""`).
        "path.join_all" => pathjoinall::goi(args, span),
        // **VD-270 (path.ancestors)**: chuỗi **tổ tiên từ vựng** POSIX gồm chính `p`; chỉ bỏ
        // `/` cuối, giữ `.`/`..`, gốc `/` là cuối (không thêm `""`) — thuần chuỗi.
        "path.ancestors" => pathancestors::goi(args, span),
        // **VD-270 (path.components)**: tách **từ vựng** POSIX thành thành phần; tuyệt đối ⇒
        // phần tử đầu `"/"`, bỏ thành phần rỗng, giữ nguyên `.`/`..` (không chuẩn hoá).
        "path.components" => pathcomponents::goi(args, span),
        "path.basename" | "path.dirname" | "path.ext" | "path.stem" => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Str(match name {
                "path.basename" => basename_voi(p, cfg!(windows)),
                "path.dirname" => dirname_voi(p, cfg!(windows)),
                "path.ext" => ext_voi(p, cfg!(windows)),
                _ => stem_voi(p, cfg!(windows)),
            }))
        }
        "path.is_abs" => {
            let p = chuoi(name, &args[0], 0, span)?;
            Ok(Value::Bool(la_tuyet_doi(p, cfg!(windows))))
        }
        // **VD-270 (path.is_absolute)**: cùng quy tắc `path.is_abs` (không đĩa/`cwd`).
        "path.is_absolute" => pathisabs::goi(args, span),
        // **VD-270 (path.is_relative)**: phần bù **từ vựng** của `path.is_abs` (không đĩa/`cwd`).
        "path.is_relative" => pathrelbool::goi(args, span),
        // **VD-270 (path.normalize)**: chuẩn hoá **từ vựng** POSIX (thuần chuỗi, không chạm đĩa).
        "path.normalize" => pathnorm::goi(args, span),
        // **VD-270 (path.relative)**: đường dẫn tương đối **thuần từ vựng** (không `cwd`/đĩa).
        "path.relative" => pathrel::goi(args, span),
        // **VD-270 (path.strip_prefix)**: bỏ tiền tố **theo thành phần** (thuần từ vựng POSIX;
        // phần còn lại **nguyên văn**, bỏ đúng một `/`; không chuẩn hoá `.`/`..`).
        "path.strip_prefix" => pathstripprefix::goi(args, span),
        // **VD-270 (path.with_extension)**: thay/đặt/bỏ phần mở rộng của thành phần cuối
        // (thuần chuỗi; dùng **đúng** `ext_voi`/`stem_voi` của `path.ext`/`path.stem`).
        "path.with_extension" => pathwithext::goi(args, span),
        // **VD-270 (path.with_file_name)**: thay **toàn bộ** thành phần cuối (tên tệp kể cả phần
        // mở rộng) bằng `name` **nguyên văn**, giữ tiền tố nguyên văn (thuần chuỗi).
        "path.with_file_name" => pathwithfilename::goi(args, span),
        // **VD-270 (path.with_stem)**: thay **phần thân** (file stem) của thành phần cuối, giữ
        // nguyên phần mở rộng hiện có (thuần chuỗi; dùng **đúng** `stem_voi`/`ext_voi`).
        "path.with_stem" => pathwithstem::goi(args, span),
        _ => unreachable!("path_d::goi chỉ nhận tên path.*"),
    }
}
