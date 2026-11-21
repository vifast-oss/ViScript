// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.strip_prefix)** — bỏ **tiền tố theo thành phần** đường dẫn, **thuần từ vựng
//! POSIX**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra tồn tại,
//! không chuẩn hoá `.`/`..`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | `p` | `base` | kết quả | ghi chú |
//! |---|---|---|---|
//! | bất kỳ | `""` | `Some(p)` | **nguyên văn**, kể cả `/` cuối |
//! | `"a/b"` | `"a/b"` | `Some("")` | bằng nhau |
//! | `"a//"` | `"a//"` | `Some("")` | bằng nhau **nguyên văn**, kể cả `/` cuối |
//! | `"a/b/"` | `"a/b/"` | `Some("")` | `/` cuối của `base` bị bỏ |
//! | `"a/b/c"` | `"a"` | `Some("b/c")` | khớp **trọn** thành phần |
//! | `"a/b/c"` | `"a/b"` | `Some("c")` | bỏ **đúng một** `/` phân cách |
//! | `"abc"` | `"ab"` | `None` | không cắt giữa thành phần |
//! | `"a/b"` | `"a/b/c"` | `None` | `p` ngắn hơn |
//! | `"a/bc"` | `"a/b"` | `None` | thành phần khác |
//! | `"a/b"` | `"a/"`, `"a//"`, `"a///"` | `Some("b")` | bỏ **mọi** `/` cuối của `base` |
//! | `"a/b"` | `"a/b/"`, `"a/b//"` | `Some("")` | như trên |
//! | `"/"` | `"/"` | `Some("")` | gốc |
//! | `"/a/b"` | `"/"` | `Some("a/b")` | gốc, bỏ đúng một `/` đầu |
//! | `"//a"` | `"/"` | `Some("/a")` | giữ `/` dư (khác Rust `a`) |
//! | `"///a"` | `"//"` | `Some("//a")` | `base` toàn `/` = gốc |
//! | `"a"` | `"/"` | `None` | tương đối vs gốc |
//! | `"/a"` | `"a"` | `None` | khác loại tuyệt đối/tương đối |
//! | `"a/../b"` | `"a"` | `Some("../b")` | `..` là thành phần **thường** |
//! | `"a/../b"` | `"a/.."` | `Some("b")` | khớp `..` **nguyên văn** |
//! | `"a/./b"` | `"a"` | `Some("./b")` | giữ `.` (khác Rust `b`) |
//! | `"a/."` | `"a"` | `Some(".")` | giữ `.` cuối (khác Rust `""`) |
//! | `"a//b"` | `"a"` | `Some("/b")` | giữ `/` dư (khác Rust `b`) |
//! | `"a/b"` | `"a//b"` | `Some("")` | khớp thành phần bỏ qua `/` rỗng |
//! | `"a//b//c"` | `"a/b"` | `Some("/c")` | phần còn lại **nguyên văn** |
//! | `"đ😀/x"` | `"đ😀"` | `Some("x")` | Unicode giữ nguyên từng scalar |
//! | `"e\u{0301}/x"` | `"e\u{0300}"` | `None` | so **BINARY**, không NFC/NFD |
//! | `"a\\b/c"` | `"a\\b"` | `Some("c")` | `\` là ký tự **thường** |
//! | `"a\0b/c"` | `"a\0b"` | `Some("c")` | NUL là ký tự **thường** |
//!
//! Tách thành phần theo **đúng** quy tắc `path.components`: bỏ `/` đầu (đánh dấu tuyệt đối) và
//! thành phần rỗng giữa/cuối; `.`/`..` được giữ **nguyên văn** như thành phần thường. So **BINARY**
//! từng byte. Phần còn lại là **hậu tố byte nguyên văn** của `p` ngay sau thành phần cuối khớp,
//! bỏ **đúng một** `/` nếu hậu tố bắt đầu bằng `/`; vì vậy `.`, `..`, `/` lặp bên trong được giữ
//! nguyên — **cố ý** khác Rust `Path::strip_prefix` (Rust trả `Components` đã chuẩn hoá). Với
//! `base` toàn `/`, kết quả là `p[1..]` (bỏ đúng một `/` đầu), nên `//a` bỏ `/` ⇒ `/a`.
//!
//! Không có lỗi dữ liệu/môi trường ⇒ **không trap**. Chữ ký `std.path.strip_prefix(p: str,
//! base: str) -> str?` — dùng `Ty::OptStr` (`Option[str]` **thật**). **Không** sugar (module-only):
//! tên `strip_prefix` không nằm trong bảng `module_sugar`. Lưu ý `str.strip_prefix` đã có từ
//! trước, nên `s.strip_prefix(…)` trên receiver `str` vẫn là **str** sugar — **không** đi qua
//! hàm này; receiver khác `str` (vd `int`) ⇒ `VS0403`. Gọi module sai kiểu ⇒ `VS0412`, sai arity
//! ⇒ `VS0401`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Tách `s` thành các thành phần **từ vựng** (bỏ `/` đầu và thành phần rỗng), trả
/// `(thành_phần, chỉ_số_kết_thúc_byte)`; `.`/`..` là thành phần thường, giữ nguyên văn.
fn thanh_phan(s: &str) -> impl Iterator<Item = (&str, usize)> {
    let than = s.trim_start_matches('/');
    let goc = s.len() - than.len();
    let mut off = goc;
    than.split('/').filter_map(move |tp| {
        if tp.is_empty() {
            // Một `/` phân cách (kể cả liên tiếp) — không phải thành phần.
            off += 1;
            None
        } else {
            let bat_dau = off;
            off = bat_dau + tp.len() + 1;
            Some((tp, bat_dau + tp.len()))
        }
    })
}

/// Bỏ `base` khỏi đầu `p` **theo thành phần**; xem bảng hợp đồng ở đầu tệp.
///
/// `base` rỗng ⇒ `Some(p)`; `base` toàn `/` ⇒ gốc (`p` tuyệt đối ⇒ `Some(&p[1..])`, còn lại
/// `None`); ngược lại khác loại tuyệt đối/tương đối hoặc lệch thành phần ⇒ `None`. Kết quả là
/// hậu tố **nguyên văn** của `p` sau thành phần cuối khớp, bỏ **đúng một** `/` đầu nếu có.
pub(super) fn bo_tien_to<'a>(p: &'a str, base: &str) -> Option<&'a str> {
    if base.is_empty() {
        return Some(p);
    }
    // `p == base` **nguyên văn** ⇒ `Some("")`, kể cả khi cả hai có `/` cuối lặp (`a//`).
    if p == base {
        return Some("");
    }
    let p_tuyet_doi = p.starts_with('/');
    let base_tuyet_doi = base.starts_with('/');

    // `base` toàn `/` ⇒ gốc (kể cả `//`, `///`): bỏ **đúng một** `/` đầu của `p` tuyệt đối.
    if base.bytes().all(|b| b == b'/') {
        return p_tuyet_doi.then(|| &p[1..]);
    }
    if p_tuyet_doi != base_tuyet_doi {
        return None;
    }

    // `/` cuối của `base` chỉ đánh dấu thư mục — bỏ **mọi** `/` cuối trước khi so thành phần.
    let base = base.trim_end_matches('/');
    let mut p_thanh = thanh_phan(p);
    let mut cuoi = 0;
    for (base_tp, _) in thanh_phan(base) {
        let (p_tp, ket_thuc) = p_thanh.next()?;
        if p_tp != base_tp {
            return None;
        }
        cuoi = ket_thuc;
    }

    // Hậu tố **nguyên văn** của `p` (giữ `.`/`..`/`//` bên trong); bỏ đúng một `/` phân cách.
    let du = &p[cuoi..];
    Some(du.strip_prefix('/').unwrap_or(du))
}

/// Dispatch `std.path.strip_prefix`: kiểm kiểu **động** `str` cho **cả hai** tham số (sai ⇒
/// `VS0501`, như mọi hàm `std.path` nhận `str`), rồi gọi [`bo_tien_to`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.strip_prefix", &args[0], 0, span)?;
    let base = super::chuoi("path.strip_prefix", &args[1], 1, span)?;
    Ok(Value::opt(
        bo_tien_to(p, base).map(|x| Value::Str(x.to_string())),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rong_va_bang_nhau() {
        assert_eq!(bo_tien_to("a/b", ""), Some("a/b"));
        assert_eq!(bo_tien_to("a/", ""), Some("a/"));
        assert_eq!(bo_tien_to("", ""), Some(""));
        assert_eq!(bo_tien_to("/", ""), Some("/"));
        assert_eq!(bo_tien_to("a/b", "a/b"), Some(""));
        assert_eq!(bo_tien_to("a/b/", "a/b/"), Some(""));
        assert_eq!(bo_tien_to("a//", "a//"), Some(""));
        assert_eq!(bo_tien_to("a//b//", "a//b//"), Some(""));
        assert_eq!(bo_tien_to("//", "//"), Some(""));
        assert_eq!(bo_tien_to("/a//", "/a//"), Some(""));
        assert_eq!(bo_tien_to("a", "a/"), Some(""));
        assert_eq!(bo_tien_to("a/", "a"), Some(""));
        assert_eq!(bo_tien_to("a//", "a"), Some("/"));
    }

    #[test]
    fn bien_thanh_phan_va_slash_cuoi() {
        assert_eq!(bo_tien_to("a/b/c", "a"), Some("b/c"));
        assert_eq!(bo_tien_to("a/b/c", "a/b"), Some("c"));
        assert_eq!(bo_tien_to("a/b/c", "a/b/c"), Some(""));
        assert_eq!(bo_tien_to("a/b/c", "a/bb"), None);
        assert_eq!(bo_tien_to("a/b", "a/b/c"), None);
        assert_eq!(bo_tien_to("abc", "ab"), None);
        assert_eq!(bo_tien_to("ab/c", "a"), None);
        assert_eq!(bo_tien_to("a/bc", "a/b"), None);
        for base in ["a/", "a//", "a///"] {
            assert_eq!(bo_tien_to("a/b", base), Some("b"), "base={base:?}");
        }
        for base in ["a/b/", "a/b//"] {
            assert_eq!(bo_tien_to("a/b", base), Some(""), "base={base:?}");
        }
    }

    #[test]
    fn goc_va_tuyet_doi_tuong_doi() {
        assert_eq!(bo_tien_to("/", "/"), Some(""));
        assert_eq!(bo_tien_to("/a", "/"), Some("a"));
        assert_eq!(bo_tien_to("/a/b", "/"), Some("a/b"));
        assert_eq!(bo_tien_to("//a", "/"), Some("/a"));
        assert_eq!(bo_tien_to("///a", "//"), Some("//a"));
        assert_eq!(bo_tien_to("//", "/"), Some("/"));
        assert_eq!(bo_tien_to("a", "/"), None);
        assert_eq!(bo_tien_to("", "/"), None);
        assert_eq!(bo_tien_to("/a", "a"), None);
        assert_eq!(bo_tien_to("a", "/a"), None);
        assert_eq!(bo_tien_to("/a/b", "a/b"), None);
        assert_eq!(bo_tien_to("a/b", "/a/b"), None);
    }

    #[test]
    fn cham_cham_va_hai_slash() {
        assert_eq!(bo_tien_to("a/../b", "a"), Some("../b"));
        assert_eq!(bo_tien_to("a/../b", "a/.."), Some("b"));
        assert_eq!(bo_tien_to("a/..", "a"), Some(".."));
        assert_eq!(bo_tien_to("a/./b", "a"), Some("./b"));
        assert_eq!(bo_tien_to("a/./b", "a/."), Some("b"));
        assert_eq!(bo_tien_to("a/.", "a"), Some("."));
        assert_eq!(bo_tien_to("a//b", "a"), Some("/b"));
        assert_eq!(bo_tien_to("a/b", "a//b"), Some(""));
        assert_eq!(bo_tien_to("a//b//c", "a/b"), Some("/c"));
        assert_eq!(bo_tien_to("a//b//", "a//b"), Some("/"));
        assert_eq!(bo_tien_to("a//./b", "a"), Some("/./b"));
    }

    #[test]
    fn unicode_va_ky_tu_thuong() {
        assert_eq!(bo_tien_to("thư/mục/tệp.txt", "thư"), Some("mục/tệp.txt"));
        assert_eq!(bo_tien_to("đ😀/x", "đ😀"), Some("x"));
        assert_eq!(bo_tien_to("đ😀/x", "đ"), None);
        assert_eq!(bo_tien_to("e\u{0301}/x", "e\u{0300}"), None);
        assert_eq!(bo_tien_to("e\u{0301}/x", "e"), None);
        assert_eq!(bo_tien_to(r"a\b/c", r"a\b"), Some("c"));
        assert_eq!(bo_tien_to("a\0b/c", "a\0b"), Some("c"));
        assert_eq!(bo_tien_to("a\0b/c", "a"), None);
        assert_eq!(bo_tien_to("/a\\b/c", "/a\\b"), Some("c"));
    }

    #[test]
    fn goi_tra_option_that() {
        use crate::value::eq;
        let some = |s: &str| Value::opt(Some(Value::Str(s.into())));
        assert!(eq(
            &goi(&[v("a/b"), v("a")], span()).expect("strip_prefix"),
            &some("b")
        ));
        assert!(eq(
            &goi(&[v("a/b"), v("a/b")], span()).expect("strip_prefix"),
            &some("")
        ));
        assert!(eq(
            &goi(&[v("a/b"), v("x")], span()).expect("strip_prefix"),
            &Value::none()
        ));
        assert!(eq(
            &goi(&[v("a"), v("")], span()).expect("strip_prefix"),
            &some("a")
        ));
    }

    fn v(s: &str) -> Value {
        Value::Str(s.into())
    }

    fn span() -> Span {
        Span::new(0, 0)
    }
}
