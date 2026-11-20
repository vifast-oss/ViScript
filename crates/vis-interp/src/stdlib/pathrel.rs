// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.relative)** — đường dẫn **tương đối** từ `from` tới `to`, **thuần từ vựng
//! POSIX**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra tồn tại.
//!
//! Hai đối số được tách bằng **đúng** helper của `path.normalize` ([`super::pathnorm::tach`]),
//! nên mọi quy tắc chuẩn hoá (`/` lặp, `.`, `..`, `/` cuối) trùng khớp tuyệt đối.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | đầu vào | kết quả | ghi chú |
//! |---|---|---|
//! | cả hai **tuyệt đối** | **tương đối** | bỏ tiền tố chung; mỗi thành phần `from` còn lại ⇒ `..` |
//! | cả hai **tương đối** | tương đối | cùng quy tắc; `..` là thành phần **thường** khi so tiền tố |
//! | bằng nhau sau chuẩn hoá | `"."` | kể cả `"/a/b"` ↔ `"/a/b"`, `""` ↔ `""` |
//! | `"a/b"` → `"a/c"` | `"../c"` | anh em |
//! | `"a/b/c"` → `"a"` | `"../.."` | cơ sở sâu hơn |
//! | `"/a/b"` → `"/"` | `"../.."` | gốc tuyệt đối; kết quả vẫn **tương đối** |
//! | `"a"` → `"/b"` | `"/b"` | **trộn loại** ⇒ trả `to` đã chuẩn hoá, **không** phân giải `cwd` |
//! | `"/a"` → `"b"` | `"b"` | trộn loại, `to` tương đối |
//! | `""` → `"a"` | `"a"` | rỗng theo normalize (`.`) nên không có thành phần |
//! | `"a"` → `""` | `".."` | như trên |
//! | `"../a"` → `"../b"` | `"../b"` | `..` khớp **nguyên văn** ở tiền tố chung |
//!
//! Kết quả **rỗng** (hai bên cùng thành phần) ⇒ `"."`. Khi **cùng loại**, kết quả luôn **tương
//! đối** — kể cả hai đầu vào tuyệt đối (không giữ `/` đầu); chỉ nhánh **trộn loại** mới có thể
//! trả chuỗi bắt đầu bằng `/` (vì đó chính là `to` tuyệt đối đã chuẩn hoá). `\` và NUL là ký tự
//! **thường**, thành phần Unicode giữ nguyên từng scalar.
//!
//! `std.path.relative(from: str, to: str) -> str` — **không** sugar R5c (module-only): tên
//! `relative` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`/`path`.
//! Gọi sai dạng `s.relative(...)` ⇒ `VS0403`; sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

use super::pathnorm::{chuan_hoa, tach};

/// Đường dẫn tương đối **từ** `from` **tới** `to` theo bảng ở đầu tệp.
pub(super) fn tuong_doi(from: &str, to: &str) -> String {
    let (from_tuyet_doi, from_phan) = tach(from);
    let (to_tuyet_doi, to_phan) = tach(to);

    // Một tuyệt đối, một tương đối: không có `cwd` để quy chiếu ⇒ trả `to` đã chuẩn hoá.
    if from_tuyet_doi != to_tuyet_doi {
        return chuan_hoa(to);
    }

    // Bỏ tiền tố chung; `..` được so **nguyên văn** như thành phần thường.
    let chung = from_phan
        .iter()
        .zip(to_phan.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut ra: Vec<&str> = Vec::with_capacity(from_phan.len() - chung + to_phan.len() - chung);
    ra.extend(std::iter::repeat_n("..", from_phan.len() - chung));
    ra.extend_from_slice(&to_phan[chung..]);

    if ra.is_empty() {
        ".".to_string()
    } else {
        ra.join("/")
    }
}

/// Dispatch `std.path.relative`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
/// [`tuong_doi`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let from = super::chuoi("path.relative", &args[0], 0, span)?;
    let to = super::chuoi("path.relative", &args[1], 1, span)?;
    Ok(Value::Str(tuong_doi(from, to)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cung_loai_va_bang_nhau() {
        assert_eq!(tuong_doi("a/b", "a/b"), ".");
        assert_eq!(tuong_doi("/a/b", "/a/b"), ".");
        assert_eq!(tuong_doi("", ""), ".");
        assert_eq!(tuong_doi(".", "./"), ".");
        assert_eq!(tuong_doi("///", "/./"), ".");
    }

    #[test]
    fn con_va_anh_em() {
        assert_eq!(tuong_doi("a", "a/b"), "b");
        assert_eq!(tuong_doi("a/b/c", "a/b/c/d"), "d");
        assert_eq!(tuong_doi("/a", "/a/b/c"), "b/c");
        assert_eq!(tuong_doi("a/b", "a/c"), "../c");
        assert_eq!(tuong_doi("/a/b", "/a/c/d"), "../c/d");
        assert_eq!(tuong_doi("a/b/c", "a/d"), "../../d");
    }

    #[test]
    fn co_so_sau_hon_va_goc() {
        assert_eq!(tuong_doi("a/b/c", "a"), "../..");
        assert_eq!(tuong_doi("a/b/c", "x/y"), "../../../x/y");
        assert_eq!(tuong_doi("/a/b", "/"), "../..");
        assert_eq!(tuong_doi("/", "/a/b"), "a/b");
    }

    #[test]
    fn tron_loai_tra_to_da_chuan_hoa() {
        assert_eq!(tuong_doi("a", "/b"), "/b");
        assert_eq!(tuong_doi("/a", "b"), "b");
        assert_eq!(tuong_doi("a", "/"), "/");
        assert_eq!(tuong_doi("/", "a"), "a");
        assert_eq!(tuong_doi("", "/a/"), "/a");
        assert_eq!(tuong_doi("/x/", "a//b/"), "a/b");
        assert_eq!(tuong_doi("/", ""), ".");
    }

    #[test]
    fn cham_cham_la_thanh_phan_thuong() {
        assert_eq!(tuong_doi("../a", "../b"), "../b");
        assert_eq!(tuong_doi("../../a/b", "../../c"), "../../c");
        assert_eq!(tuong_doi("../a", "x"), "../../x");
        assert_eq!(tuong_doi("..", ".."), ".");
        assert_eq!(tuong_doi("../a/b", "../a/c"), "../c");
    }

    #[test]
    fn rong_unicode_va_ky_tu_thuong() {
        assert_eq!(tuong_doi("", "a"), "a");
        assert_eq!(tuong_doi("a", ""), "..");
        assert_eq!(tuong_doi("a/./b/", "a/b/../c/"), "../c");
        assert_eq!(tuong_doi("/thư/mục", "/thư/tệp.txt"), "../tệp.txt");
        assert_eq!(tuong_doi("đ😀", "đ😀/x"), "x");
        assert_eq!(tuong_doi(r"a\b", r"a\b/c"), "c");
        assert_eq!(tuong_doi("a\0b", "a\0b/c"), "c");
    }
}
