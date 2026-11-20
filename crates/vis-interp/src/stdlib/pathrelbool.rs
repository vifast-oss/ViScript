// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.is_relative)** — `p` có **tương đối** không, theo **đúng** quy tắc từ vựng của
//! `path.is_abs`: **thuần chuỗi**, không chạm đĩa, không đọc `cwd`, không phân giải symlink,
//! không kiểm tra tồn tại.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM): `is_relative(p) == !is_abs(p)` — cùng
//! [`la_tuyet_doi`] và cùng cờ nền tảng, nên hai hàm **không bao giờ** lệch nhau (kể cả trên
//! Windows, nơi `path.is_abs` nhận ổ đĩa/UNC).
//!
//! | đầu vào | `is_relative` | ghi chú |
//! |---|---|---|
//! | `""` | `true` | đường dẫn rỗng là **tương đối** |
//! | `"a"`, `"a/b"`, `"."`, `".."` | `true` | không mở đầu bằng `/` |
//! | `"//"`, `"///"` | `false` | mở đầu bằng `/` ⇒ tuyệt đối (không gộp `/`) |
//! | `"/"`, `"/a/b"` | `false` | tuyệt đối POSIX |
//! | `"a\\b"`, `"a\0b"` | `true` | `\` và NUL là ký tự **thường**, không phải phân cách |
//! | `"đ😀/x"` | `true` | thành phần Unicode giữ nguyên từng scalar |
//!
//! Không có lỗi dữ liệu/môi trường ⇒ **không trap**. Chữ ký `std.path.is_relative(p: str) ->
//! bool` — **không** sugar R5c (module-only): tên `is_relative` không nằm trong bảng
//! `module_sugar`, tránh nhập nhằng receiver `str`/`path`; gọi `s.is_relative()` ⇒ `VS0403`,
//! sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::builtin_ls::la_tuyet_doi;
use crate::interp::R;
use crate::value::Value;

/// `true` khi [`la_tuyet_doi`] nói `false` — **cùng** quy tắc POSIX và cùng cờ nền tảng với
/// `path.is_abs`, nên đây đúng là phần bù logic của hàm đó.
pub(super) fn la_tuong_doi(p: &str) -> bool {
    !la_tuyet_doi(p, cfg!(windows))
}

/// Dispatch `std.path.is_relative`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
/// [`la_tuong_doi`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.is_relative", &args[0], 0, span)?;
    Ok(Value::Bool(la_tuong_doi(p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_tuong_doi() {
        assert!(la_tuong_doi(""));
        assert!(la_tuong_doi("a"));
        assert!(la_tuong_doi("a/b"));
        assert!(la_tuong_doi("."));
        assert!(la_tuong_doi(".."));
        assert!(la_tuong_doi("../a"));
        assert!(la_tuong_doi("./a/"));
    }

    #[test]
    fn tuyet_doi_posix() {
        assert!(!la_tuong_doi("/"));
        assert!(!la_tuong_doi("/a"));
        assert!(!la_tuong_doi("/a/b"));
        // Không gộp `/`: `//` vẫn mở đầu bằng `/` ⇒ tuyệt đối.
        assert!(!la_tuong_doi("//"));
        assert!(!la_tuong_doi("///a"));
    }

    #[test]
    fn dung_la_phan_bu_cua_is_abs() {
        for p in [
            "", "/", "a", "/a", "//", "//a", ".", "..", "../a", "đ😀", "a\\b",
        ] {
            assert_eq!(
                la_tuong_doi(p),
                !la_tuyet_doi(p, cfg!(windows)),
                "is_relative phải là phần bù của is_abs cho {p:?}"
            );
        }
    }

    #[test]
    fn ky_tu_thuong_unicode_va_nul() {
        // `\` và NUL là ký tự **thường**, không phải phân cách.
        assert!(la_tuong_doi(r"a\b"));
        assert!(la_tuong_doi("a\0b"));
        // Thành phần Unicode giữ nguyên từng scalar, kể cả tổ hợp.
        assert!(la_tuong_doi("thư/mục/tệp.txt"));
        assert!(la_tuong_doi("đ😀"));
        assert!(la_tuong_doi("e\u{0302}\u{0301}/x"));
    }
}
