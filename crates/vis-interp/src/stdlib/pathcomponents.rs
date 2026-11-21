// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.components)** — tách đường dẫn **POSIX** thành **thành phần từ vựng**,
//! **thuần chuỗi**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra
//! tồn tại, **không chuẩn hoá** `.`/`..`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | đầu vào | kết quả | ghi chú |
//! |---|---|---|
//! | `""` | `[]` | không có thành phần nào |
//! | `"/"`, `"//"`, `"///"` | `["/"]` | chỉ toàn `/` tuyệt đối ⇒ gốc |
//! | `"a//b/"` | `["a","b"]` | chia theo `/`, **bỏ** thành phần rỗng giữa/cuối |
//! | `"a/"` | `["a"]` | `/` cuối không sinh thành phần rỗng |
//! | `"/a//b"`, `"///a"` | `["/","a","b"]`, `["/","a"]` | tuyệt đối ⇒ `"/"` đầu, gộp `/` đầu |
//! | `"//srv/share"` | `["/","srv","share"]` | hai `/` đầu gộp thành root `/` (như `path.normalize`) |
//! | `"./a"`, `"a/./b"` | `[".","a"]`, `["a",".","b"]` | `.` là thành phần **thường**, giữ nguyên văn |
//! | `"a/../b"`, `"a/.."` | `["a","..","b"]`, `["a",".."]` | `..` **không** phân giải với phần trước |
//! | `".."`, `"."` | `[".."]`, `["."]` | giữ nguyên văn |
//! | `"a/../.."` | `["a","..",".."]` | giữ **đủ** số `..` |
//!
//! `\` và NUL là ký tự **thường** trên POSIX (không phải phân cách); thành phần Unicode giữ
//! nguyên **từng scalar**, không chuẩn hoá NFC/NFD. Không có lỗi dữ liệu/môi trường ⇒ **không
//! trap**.
//!
//! `std.path.components(p: str) -> list[str]` — **không** sugar R5c (module-only): tên
//! `components` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`/`path`;
//! gọi `p.components()` ⇒ `VS0403`, sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tách **từ vựng** `p` thành các thành phần, theo **đúng** bảng ở đầu tệp.
///
/// Tuyệt đối (mở đầu `/`) ⇒ phần tử đầu `"/"`; mọi thành phần rỗng do `/` lặp/đầu/cuối đều bị
/// bỏ; `.`/`..` được giữ **nguyên văn** (không phân giải/không chuẩn hoá). Trả các mảnh mượn
/// của `p` nên không cấp phát chuỗi mới.
pub(super) fn thanh_phan(p: &str) -> Vec<&str> {
    let mut ra: Vec<&str> = Vec::new();
    if p.starts_with('/') {
        // Root duy nhất: `//srv/share` ⇒ `["/","srv","share"]` (gộp mọi `/` đầu).
        ra.push("/");
    }
    // `split('/')` sinh mảnh rỗng cho `/` đầu/lặp/cuối — bỏ hết; phần còn lại giữ **nguyên văn**.
    ra.extend(p.split('/').filter(|x| !x.is_empty()));
    ra
}

/// Dispatch `std.path.components`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
/// [`thanh_phan`] và đóng gói thành `list[str]`.
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.components", &args[0], 0, span)?;
    let ds = thanh_phan(p)
        .into_iter()
        .map(|x| Value::Str(x.to_string()))
        .collect();
    Ok(Value::List(ds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_goc() {
        assert_eq!(thanh_phan(""), Vec::<&str>::new());
        assert_eq!(thanh_phan("/"), ["/"]);
        assert_eq!(thanh_phan("//"), ["/"]);
        assert_eq!(thanh_phan("///"), ["/"]);
        assert_eq!(thanh_phan("////"), ["/"]);
    }

    #[test]
    fn slash_lap_va_cuoi() {
        assert_eq!(thanh_phan("a"), ["a"]);
        assert_eq!(thanh_phan("a/b"), ["a", "b"]);
        assert_eq!(thanh_phan("a//b/"), ["a", "b"]);
        assert_eq!(thanh_phan("a/"), ["a"]);
        assert_eq!(thanh_phan("a///b//c///"), ["a", "b", "c"]);
        assert_eq!(thanh_phan("/a//b"), ["/", "a", "b"]);
        assert_eq!(thanh_phan("///a"), ["/", "a"]);
        assert_eq!(thanh_phan("//srv/share"), ["/", "srv", "share"]);
        assert_eq!(thanh_phan("/a/b/"), ["/", "a", "b"]);
    }

    #[test]
    fn cham_va_cham_cham_nguyen_van() {
        assert_eq!(thanh_phan("."), ["."]);
        assert_eq!(thanh_phan(".."), [".."]);
        assert_eq!(thanh_phan("./a"), [".", "a"]);
        assert_eq!(thanh_phan("../a"), ["..", "a"]);
        assert_eq!(thanh_phan("a/./b"), ["a", ".", "b"]);
        assert_eq!(thanh_phan("a/../b"), ["a", "..", "b"]);
        assert_eq!(thanh_phan("a/.."), ["a", ".."]);
        assert_eq!(thanh_phan("a/../.."), ["a", "..", ".."]);
        assert_eq!(thanh_phan("/./a"), ["/", ".", "a"]);
        assert_eq!(thanh_phan("/../a"), ["/", "..", "a"]);
        assert_eq!(thanh_phan("//../x"), ["/", "..", "x"]);
    }

    #[test]
    fn unicode_giu_nguyen_scalar() {
        assert_eq!(thanh_phan("thư/mục/tệp.txt"), ["thư", "mục", "tệp.txt"]);
        assert_eq!(thanh_phan("đ😀/x"), ["đ😀", "x"]);
        assert_eq!(thanh_phan("/đ😀//x/"), ["/", "đ😀", "x"]);
        assert_eq!(
            thanh_phan("e\u{0302}\u{0301}/x"),
            ["e\u{0302}\u{0301}", "x"]
        );
    }

    #[test]
    fn ky_tu_thuong_va_nul() {
        // `\` và NUL là ký tự **thường**, không phải phân cách.
        assert_eq!(thanh_phan(r"a\b"), [r"a\b"]);
        assert_eq!(thanh_phan(r"\a\b/c"), [r"\a\b", "c"]);
        assert_eq!(thanh_phan("a\0b"), ["a\0b"]);
        assert_eq!(thanh_phan("/a\0b/c"), ["/", "a\0b", "c"]);
    }
}
