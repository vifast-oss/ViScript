// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.join_all)** — nối **nhiều** thành phần đường dẫn bằng **đúng** quy tắc
//! [`std.path.join`](crate::stdlib): không chuẩn hoá, không chạm đĩa/`cwd`/symlink/tồn tại.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | đầu vào | kết quả | ghi chú |
//! |---|---|---|
//! | `[]` | `""` | không có thành phần |
//! | `["a"]` | `"a"` | một phần tử ⇒ chính nó, **không** chuẩn hoá |
//! | `["a", "b"]` | `"a/b"` | byte-identical `path.join("a", "b")` |
//! | `["a", "b", "c"]` | `"a/b/c"` | gấp trái `acc = path.join(acc, x)` |
//! | `["", "a"]`, `["a", ""]` | `"a"`, `"a/"` | thành phần rỗng theo `path.join` |
//! | `["a", "", "b"]` | `"a/b"` | `/` cuối của `acc` bị bỏ khi ghép tiếp |
//! | `["a/", "b"]`, `["a//", "b"]` | `"a/b"` | gộp `/` cuối của `acc` |
//! | `["a", "b//"]` | `"a/b//"` | `/` cuối của **phần tử cuối** giữ nguyên |
//! | `["/a", "b", "c"]` | `"/a/b/c"` | tuyệt đối **đầu** giữ `/` đầu |
//! | `["a", "/b", "c"]` | `"/b/c"` | tuyệt đối **giữa** thay phần đã ghép |
//! | `["a", "/b", "/c", "d"]` | `"/c/d"` | tuyệt đối **cuối** thắng |
//! | `["a", "/"]`, `["a", "//b"]` | `"/"`, `"//b"` | thành phần tuyệt đối trả **nguyên văn** |
//!
//! Mỗi bước gọi **đúng** helper của `path.join` ([`crate::builtin_ls::noi_duong_voi`]), nên
//! mọi quy tắc `/` đầu/cuối, thành phần rỗng, ổ đĩa/UNC trên Windows trùng khớp tuyệt đối;
//! bảng trên là hợp đồng POSIX (nền tảng test). `\` và NUL là ký tự **thường** trên POSIX;
//! thành phần Unicode giữ nguyên từng scalar.
//!
//! `std.path.join_all(parts: list[str]) -> str` — **không** sugar R5c (module-only): tên
//! `join_all` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`/`list`.
//! Gọi sai dạng `s.join_all(...)`/`xs.join_all(...)` ⇒ `VS0403`; sai kiểu ⇒ `VS0412`; sai
//! arity ⇒ `VS0401`. Phần tử không phải `str` (dữ liệu không kiểu tới tầng chạy) ⇒ trap
//! `VS0501` nêu vị trí, không panic Rust.

use vis_syntax::Span;

use crate::builtin_ls::noi_duong_voi;
use crate::interp::R;
use crate::value::Value;

/// Gấp trái `parts` bằng **đúng** `path.join`: `acc` khởi đầu `""` (nên `[]` ⇒ `""`, một phần
/// tử ⇒ chính nó), mỗi bước `acc = noi_duong_voi(acc, phần_tử, windows)`.
pub(super) fn noi_tat_ca(parts: &[&str], windows: bool) -> String {
    let mut acc = String::new();
    for p in parts {
        acc = noi_duong_voi(&acc, p, windows);
    }
    acc
}

/// Dispatch `std.path.join_all`: kiểm **động** `list` (sai ⇒ `VS0501`), kiểm từng phần tử `str`,
/// rồi gọi [`noi_tat_ca`] với `cfg!(windows)` — cùng nền tảng `path.join`.
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let parts = super::danh_sach("path.join_all", &args[0], 0, span)?;
    let mut ds: Vec<&str> = Vec::with_capacity(parts.len());
    for (i, v) in parts.iter().enumerate() {
        match v {
            Value::Str(s) => ds.push(s),
            other => {
                return Err(super::loi(
                    format!(
                        "`path.join_all`: phần tử {i} cần `str`, được `{}`",
                        other.show()
                    ),
                    span,
                ));
            }
        }
    }
    Ok(Value::Str(noi_tat_ca(&ds, cfg!(windows))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_mot_phan_tu() {
        assert_eq!(noi_tat_ca(&[], false), "");
        assert_eq!(noi_tat_ca(&["a"], false), "a");
        assert_eq!(noi_tat_ca(&[""], false), "");
        assert_eq!(noi_tat_ca(&["/a"], false), "/a");
        assert_eq!(noi_tat_ca(&["a/"], false), "a/");
    }

    #[test]
    fn nhieu_phan_tu_giong_path_join_tung_buoc() {
        assert_eq!(noi_tat_ca(&["a", "b"], false), "a/b");
        assert_eq!(noi_tat_ca(&["a", "b", "c"], false), "a/b/c");
        assert_eq!(noi_tat_ca(&["a", "b", "c", "d"], false), "a/b/c/d");
        assert_eq!(noi_tat_ca(&["/a", "b", "c"], false), "/a/b/c");
    }

    #[test]
    fn thanh_phan_rong_theo_path_join() {
        assert_eq!(noi_tat_ca(&["", ""], false), "");
        assert_eq!(noi_tat_ca(&["", "a"], false), "a");
        assert_eq!(noi_tat_ca(&["a", ""], false), "a/");
        assert_eq!(noi_tat_ca(&["a", "", "b"], false), "a/b");
        assert_eq!(noi_tat_ca(&["", "a", ""], false), "a/");
        assert_eq!(noi_tat_ca(&["a", "b", ""], false), "a/b/");
    }

    #[test]
    fn slash_dau_cuoi() {
        assert_eq!(noi_tat_ca(&["a/", "b"], false), "a/b");
        assert_eq!(noi_tat_ca(&["a//", "b"], false), "a/b");
        assert_eq!(noi_tat_ca(&["/", "a"], false), "/a");
        assert_eq!(noi_tat_ca(&["//", "a"], false), "/a");
        assert_eq!(noi_tat_ca(&["a", "b/", "c"], false), "a/b/c");
        assert_eq!(noi_tat_ca(&["a", "b//"], false), "a/b//");
        assert_eq!(noi_tat_ca(&["/a/", "b/"], false), "/a/b/");
        assert_eq!(noi_tat_ca(&["a", "//b"], false), "//b");
        assert_eq!(noi_tat_ca(&["a", "//b", "c"], false), "//b/c");
    }

    #[test]
    fn tuyet_doi_giua_va_cuoi_thay_phan_truoc() {
        assert_eq!(noi_tat_ca(&["a", "/b", "c"], false), "/b/c");
        assert_eq!(noi_tat_ca(&["a/b", "/c/d", "e"], false), "/c/d/e");
        assert_eq!(noi_tat_ca(&["a", "/b", "/c", "d"], false), "/c/d");
        assert_eq!(noi_tat_ca(&["a", "/", "b"], false), "/b");
        assert_eq!(noi_tat_ca(&["a", "b", "/c"], false), "/c");
        assert_eq!(noi_tat_ca(&["a", "/b/"], false), "/b/");
    }

    #[test]
    fn unicode_va_ky_tu_thuong() {
        assert_eq!(
            noi_tat_ca(&["thư", "mục", "tệp.txt"], false),
            "thư/mục/tệp.txt"
        );
        assert_eq!(noi_tat_ca(&["đ😀", "x"], false), "đ😀/x");
        assert_eq!(
            noi_tat_ca(&["e\u{0302}\u{0301}", "x"], false),
            "e\u{0302}\u{0301}/x"
        );
        // POSIX: `\` là ký tự thường, không phải phân cách.
        assert_eq!(noi_tat_ca(&[r"a\b", "c"], false), r"a\b/c");
    }

    #[test]
    fn windows_dung_cung_helper_path_join() {
        // Cùng `noi_duong_voi(.., true)`: ổ đĩa tuyệt đối/rooted theo đúng `path.join`.
        assert_eq!(noi_tat_ca(&["C:\\a", "b"], true), "C:\\a\\b");
        assert_eq!(noi_tat_ca(&["C:\\a", "D:\\b", "c"], true), "D:\\b\\c");
        assert_eq!(noi_tat_ca(&["C:\\a", "\\b"], true), "C:\\b");
    }
}
