// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.normalize)** — chuẩn hoá **từ vựng** đường dẫn POSIX, **thuần chuỗi**:
//! không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra tồn tại.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | đầu vào | kết quả | ghi chú |
//! |---|---|---|
//! | `""` | `"."` | đường dẫn tương đối rỗng ⇒ thư mục hiện tại |
//! | `"/"`, `"//"`, `"///"` | `"/"` | chỉ toàn `/` tuyệt đối ⇒ gốc |
//! | `"a//b"` | `"a/b"` | gộp `/` lặp |
//! | `"a/./b"` | `"a/b"` | bỏ `.` |
//! | `"a/../b"` | `"b"` | `..` phân giải với thành phần **thường** đứng trước |
//! | `"a/.."`, `"."`, `"./"` | `"."` | kết quả tương đối rỗng ⇒ `.` |
//! | `"../a"`, `"../../a"` | giữ nguyên | `..` đầu chuỗi **không** có gì để phân giải |
//! | `"a/../../b"` | `"../b"` | `..` vượt gốc tương đối được **giữ** |
//! | `"/.."`, `"/a/../.."` | `"/"` | gốc tuyệt đối **không** vượt được ⇒ `..` bị bỏ |
//! | `"a/"`, `"a/b//"` | `"a"`, `"a/b"` | bỏ `/` cuối (trừ gốc `/`) |
//!
//! Đây là helper **POSIX**: `\` là ký tự **thường** (không phải phân cách), NUL (`\0`) cũng
//! **thường** — không có trap `VS0501` vì hàm thuần từ vựng không có lỗi môi trường/dữ liệu;
//! mọi thành phần Unicode được giữ nguyên từng scalar.
//!
//! `std.path.normalize(p: str) -> str`, sugar R5c `p.normalize()`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Chuẩn hoá **từ vựng** `p` theo bảng ở đầu tệp; không cấp phát gì ngoài `Vec` thành phần và
/// chuỗi kết quả.
pub(super) fn chuan_hoa(p: &str) -> String {
    let tuyet_doi = p.starts_with('/');
    let mut phan: Vec<&str> = Vec::new();
    for x in p.split('/') {
        match x {
            // `/` lặp và `.` không đóng góp thành phần nào.
            "" | "." => {}
            ".." => match phan.last() {
                // Chỉ phân giải với thành phần **thường**; `..` đứng trước không bị "ăn".
                Some(cuoi) if *cuoi != ".." => {
                    phan.pop();
                }
                // Gốc tuyệt đối không vượt được: `..` bị bỏ.
                _ if tuyet_doi => {}
                // Tương đối: giữ `..` đầu chuỗi (không có gì để phân giải).
                _ => phan.push(".."),
            },
            thuong => phan.push(thuong),
        }
    }
    let noi = phan.join("/");
    if tuyet_doi {
        // `noi` rỗng ⇒ `"/"` (gốc); ngược lại `"/a/b"`.
        format!("/{noi}")
    } else if noi.is_empty() {
        ".".to_string()
    } else {
        noi
    }
}

/// Dispatch `std.path.normalize`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`, như mọi hàm `std.str`
/// nhận `str`), rồi gọi [`chuan_hoa`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.normalize", &args[0], 0, span)?;
    Ok(Value::Str(chuan_hoa(p)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuyet_doi_va_goc() {
        assert_eq!(chuan_hoa("/"), "/");
        assert_eq!(chuan_hoa("//"), "/");
        assert_eq!(chuan_hoa("///"), "/");
        assert_eq!(chuan_hoa("/a//b/./c/../d/"), "/a/b/d");
        assert_eq!(chuan_hoa("/.."), "/");
        assert_eq!(chuan_hoa("/a/../.."), "/");
    }

    #[test]
    fn tuong_doi_va_cham() {
        assert_eq!(chuan_hoa(""), ".");
        assert_eq!(chuan_hoa("."), ".");
        assert_eq!(chuan_hoa("./"), ".");
        assert_eq!(chuan_hoa("a"), "a");
        assert_eq!(chuan_hoa("a//b/"), "a/b");
        assert_eq!(chuan_hoa("a/../b"), "b");
        assert_eq!(chuan_hoa("a/.."), ".");
        assert_eq!(chuan_hoa("../a"), "../a");
        assert_eq!(chuan_hoa("../../a"), "../../a");
        assert_eq!(chuan_hoa("a/../../b"), "../b");
        assert_eq!(chuan_hoa(".."), "..");
    }

    #[test]
    fn ky_tu_thuong_unicode_va_nul() {
        // `\` và NUL là ký tự **thường**, không phải phân cách.
        assert_eq!(chuan_hoa(r"a\b"), r"a\b");
        assert_eq!(chuan_hoa("a\0b"), "a\0b");
        // Thành phần Unicode giữ nguyên từng scalar, kể cả tổ hợp.
        assert_eq!(chuan_hoa("/thư/mục/../tệp.txt"), "/thư/tệp.txt");
        assert_eq!(chuan_hoa("e\u{0302}\u{0301}/x"), "e\u{0302}\u{0301}/x");
    }
}
