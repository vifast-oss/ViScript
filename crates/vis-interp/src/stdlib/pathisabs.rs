// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.is_absolute)** — `p` có **tuyệt đối** không, theo **đúng** quy tắc từ vựng của
//! `path.is_abs`: tái dùng [`la_tuyet_doi`] và **cùng cờ nền tảng**, nên
//! `is_absolute(p) == is_abs(p)` cho **mọi** đầu vào.
//!
//! **Thuần chuỗi**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra
//! tồn tại.
//!
//! | đầu vào | `is_absolute` | ghi chú |
//! |---|---|---|
//! | `""` | `false` | đường dẫn rỗng **không** tuyệt đối |
//! | `"a"`, `"a/b"`, `"."`, `".."` | `false` | không mở đầu bằng `/` |
//! | `"/"`, `"/a/b"` | `true` | tuyệt đối POSIX |
//! | `"//"`, `"///a"` | `true` | mở đầu bằng `/` ⇒ tuyệt đối (không gộp `/`) |
//! | `"a\\b"`, `"a\0b"` | `false` | `\` và NUL là ký tự **thường**, không phải phân cách |
//! | `"/a\\b"`, `"/a\0b"` | `true` | `/` đầu quyết định |
//! | `"đ😀/x"` | `false` | thành phần Unicode giữ nguyên từng scalar |
//!
//! Không có lỗi dữ liệu/môi trường ⇒ **không trap**. Chữ ký `std.path.is_absolute(p: str) ->
//! bool` — **không** sugar R5c (module-only): tên `is_absolute` không nằm trong bảng
//! `module_sugar`, tránh nhập nhằng receiver `str`/`path`; gọi `s.is_absolute()` ⇒ `VS0403`,
//! sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.
//!
//! **Windows** (`cfg!(windows)`): `/` đơn và `/a` **không** tuyệt đối; `//…`/`\\…` hoặc
//! `<ổ>:` + `/`/`\` mới tuyệt đối (đúng `Path::is_absolute` của Windows). Các dòng `/`, `/\a`
//! trong bảng trên là quy tắc POSIX; test có nhánh Windows riêng.

use vis_syntax::Span;

use crate::builtin_ls::la_tuyet_doi;
use crate::interp::R;
use crate::value::Value;

/// `true` khi [`la_tuyet_doi`] nói `true` — **cùng** quy tắc và cùng cờ nền tảng với
/// `path.is_abs`, nên đây đúng là cùng một hợp đồng.
pub(super) fn tuyet_doi(p: &str) -> bool {
    la_tuyet_doi(p, cfg!(windows))
}

/// Dispatch `std.path.is_absolute`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
/// [`tuyet_doi`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.is_absolute", &args[0], 0, span)?;
    Ok(Value::Bool(tuyet_doi(p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_tuong_doi() {
        assert!(!tuyet_doi(""));
        assert!(!tuyet_doi("a"));
        assert!(!tuyet_doi("a/b"));
        assert!(!tuyet_doi("."));
        assert!(!tuyet_doi(".."));
        assert!(!tuyet_doi("../a"));
        assert!(!tuyet_doi("./a/"));
    }

    #[test]
    fn tuyet_doi_posix() {
        #[cfg(not(windows))]
        {
            assert!(tuyet_doi("/"));
            assert!(tuyet_doi("/a"));
            assert!(tuyet_doi("/a/b"));
            // Không gộp `/`: `//` vẫn mở đầu bằng `/` ⇒ tuyệt đối.
            assert!(tuyet_doi("//"));
            assert!(tuyet_doi("///a"));
        }
        #[cfg(windows)]
        {
            // Windows: `/` đơn không có ổ đĩa/UNC ⇒ KHÔNG tuyệt đối (đúng `Path::is_absolute`).
            assert!(!tuyet_doi("/"));
            assert!(!tuyet_doi("/a"));
            assert!(!tuyet_doi("/a/b"));
            assert!(tuyet_doi("//"));
            assert!(tuyet_doi("///a"));
        }
    }

    #[test]
    fn trung_khit_is_abs() {
        for p in [
            "", "/", "a", "/a", "//", "//a", ".", "..", "../a", "đ😀", "a\\b",
        ] {
            assert_eq!(
                tuyet_doi(p),
                la_tuyet_doi(p, cfg!(windows)),
                "is_absolute phải trùng khít is_abs cho {p:?}"
            );
        }
    }

    #[test]
    fn ky_tu_thuong_unicode_va_nul() {
        // `\` và NUL là ký tự **thường** trên POSIX; trên Windows `\` là phân cách (và `/a\b`
        // không có ổ đĩa ⇒ vẫn tương đối).
        assert!(!tuyet_doi(r"a\b"));
        assert!(!tuyet_doi("a\0b"));
        #[cfg(not(windows))]
        {
            assert!(tuyet_doi(r"/a\b"));
            assert!(tuyet_doi("/a\0b"));
        }
        #[cfg(windows)]
        {
            assert!(!tuyet_doi(r"/a\b"));
            assert!(!tuyet_doi("/a\0b"));
        }
        // Thành phần Unicode giữ nguyên từng scalar, kể cả tổ hợp.
        assert!(!tuyet_doi("thư/mục/tệp.txt"));
        assert!(!tuyet_doi("đ😀"));
        assert!(!tuyet_doi("đ😀/x"));
        #[cfg(not(windows))]
        assert!(tuyet_doi("/thư/mục"));
        #[cfg(windows)]
        assert!(!tuyet_doi("/thư/mục"));
        assert!(!tuyet_doi("e\u{0302}\u{0301}/x"));
    }
}
