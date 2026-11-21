// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.ancestors)** — chuỗi **tổ tiên từ vựng POSIX** của `p`, **gồm chính `p`**,
//! **thuần chuỗi**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra
//! tồn tại, **không chuẩn hoá** `.`/`..`.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! 1. Chuẩn hoá **chỉ** `/` cuối (trừ gốc): bỏ **mọi** `/` cuối khi phần còn lại **không
//!    rỗng**; `"/"`/`"//"`/`"///"` giữ `"/"`. Phần tử **đầu** = `p` đã bỏ `/` cuối.
//! 2. Lặp: bỏ **thành phần cuối** (đoạn sau `/` cuối) **kể cả** dấu `/` phân cách; phần còn
//!    lại bỏ `/` cuối (trừ gốc). Dừng khi `cur == "/"` (gốc là cuối, **không** thêm `""`)
//!    hoặc `cur` **không còn** `/` (thêm `""`); `p` rỗng ⇒ chỉ `[""]`.
//!
//! | đầu vào | kết quả | ghi chú |
//! |---|---|---|
//! | `""` | `[""]` | đúng **một** phần tử rỗng |
//! | `"/"`, `"//"`, `"///"` | `["/"]` | chỉ toàn `/` tuyệt đối ⇒ gốc |
//! | `"a"`, `"a/"`, `"a//"` | `["a",""]` | bỏ `/` cuối rồi hết `/` ⇒ thêm `""` |
//! | `"a/b/c"` | `["a/b/c","a/b","a",""]` | bỏ thành phần cuối từng bước |
//! | `"a/b/"` | `["a/b","a",""]` | `/` cuối bị bỏ **trước** khi lặp |
//! | `"/a"`, `"/a/"` | `["/a","/"]` | gốc `/` là cuối |
//! | `"/a/b"` | `["/a/b","/a","/"]` | dừng ở gốc, **không** thêm `""` |
//! | `"."`, `".."` | `[".",""]`, `["..",""]` | `.`/`..` là thành phần **thường** |
//! | `"./a"` | `["./a",".",""]` | giữ **nguyên văn**, không phân giải |
//! | `"a//b"` | `["a//b","a",""]` | bỏ `b`, prefix `a/` → bỏ `/` cuối ⇒ `a` |
//! | `"a///b//"` | `["a///b","a",""]` | `/` lặp giữa thành phần, `/` cuối bị bỏ |
//! | `"//srv/share"` | `["//srv/share","//srv","/"]` | `//` đầu **không** gộp |
//! | `"//srv"` | `["//srv","/"]` | prefix `//` → gốc `/` |
//! | `"///srv/share"` | `["///srv/share","///srv","/"]` | giữ nguyên số `/` đầu |
//! | `"a/."`, `"a/.."` | `["a/.","a",""]`, `["a/..","a",""]` | **không** chuẩn hoá |
//! | `"../a/b"` | `["../a/b","../a","..",""]` | `..` giữ nguyên từng bước |
//!
//! Quy tắc **nhất quán** cho `//` đầu: phần tử đầu giữ **nguyên văn** `p` đã bỏ `/` cuối;
//! mỗi bước lấy prefix **tính cả** `/` phân cách rồi bỏ mọi `/` cuối (trừ khi prefix toàn
//! `/` ⇒ `"/"`), nên `"//srv/share"` ⇒ `["//srv/share","//srv","/"]` và `"///srv/share"`
//! ⇒ `["///srv/share","///srv","/"]` — **không** có `""` sau gốc.
//!
//! `\` và NUL là ký tự **thường** trên POSIX (không phải phân cách); thành phần Unicode giữ
//! nguyên **từng scalar**, không chuẩn hoá NFC/NFD. Không có lỗi dữ liệu/môi trường ⇒ **không
//! trap**.
//!
//! `std.path.ancestors(p: str) -> list[str]` — **không** sugar R5c (module-only): tên
//! `ancestors` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`/`path`;
//! gọi `p.ancestors()` ⇒ `VS0403`, sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::interp::R;
use crate::value::Value;

/// Bỏ **mọi** `/` cuối; chuỗi **toàn** `/` (không rỗng) co lại thành `"/"`; `""` giữ `""`.
fn chuan(p: &str) -> &str {
    let t = p.trim_end_matches('/');
    if t.is_empty() && !p.is_empty() {
        "/"
    } else {
        t
    }
}

/// Trả **tổ tiên từ vựng** của `p`, theo **đúng** bảng ở đầu tệp.
///
/// Phần tử đầu là `p` đã bỏ `/` cuối; mỗi bước bỏ thành phần cuối **kể cả** `/` phân cách,
/// bỏ `/` cuối của prefix (trừ gốc), dừng ở `"/"` hoặc thêm `""` khi hết `/`. Trả các mảnh
/// mượn của `p` nên không cấp phát chuỗi mới.
pub(super) fn to_tien(p: &str) -> Vec<&str> {
    let dau = chuan(p);
    let mut ra = vec![dau];
    if dau.is_empty() {
        return ra;
    }
    let mut cur = dau;
    while cur != "/" {
        match cur.rfind('/') {
            // Prefix **tính cả** dấu `/` phân cách: `"/a"` ⇒ `"/"`, `"a/b"` ⇒ `"a"`,
            // `"a//b"` ⇒ `"a"`, `"//srv"` ⇒ `"/"` (mọi `/` cuối bị bỏ, trừ gốc).
            Some(i) => {
                let pre = chuan(&cur[..=i]);
                ra.push(pre);
                cur = pre;
            }
            // Không còn `/` ⇒ tổ tiên cuối là đường dẫn rỗng.
            None => {
                ra.push("");
                break;
            }
        }
    }
    ra
}

/// Dispatch `std.path.ancestors`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
/// [`to_tien`] và đóng gói thành `list[str]`.
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.ancestors", &args[0], 0, span)?;
    let ds = to_tien(p)
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
        assert_eq!(to_tien(""), [""]);
        assert_eq!(to_tien("/"), ["/"]);
        assert_eq!(to_tien("//"), ["/"]);
        assert_eq!(to_tien("///"), ["/"]);
        assert_eq!(to_tien("////"), ["/"]);
    }

    #[test]
    fn tuong_doi_va_slash_cuoi() {
        assert_eq!(to_tien("a"), ["a", ""]);
        assert_eq!(to_tien("a/"), ["a", ""]);
        assert_eq!(to_tien("a//"), ["a", ""]);
        assert_eq!(to_tien("a/b/c"), ["a/b/c", "a/b", "a", ""]);
        assert_eq!(to_tien("a/b/"), ["a/b", "a", ""]);
        assert_eq!(to_tien("a//b"), ["a//b", "a", ""]);
        assert_eq!(to_tien("a///b//"), ["a///b", "a", ""]);
        assert_eq!(to_tien("a//b//c"), ["a//b//c", "a//b", "a", ""]);
    }

    #[test]
    fn tuyet_doi_goc_la_cuoi() {
        assert_eq!(to_tien("/a"), ["/a", "/"]);
        assert_eq!(to_tien("/a/"), ["/a", "/"]);
        assert_eq!(to_tien("/a/b"), ["/a/b", "/a", "/"]);
        assert_eq!(to_tien("/a/b//"), ["/a/b", "/a", "/"]);
        assert_eq!(to_tien("/a//b"), ["/a//b", "/a", "/"]);
        assert_eq!(to_tien("//srv"), ["//srv", "/"]);
        assert_eq!(to_tien("//srv/share"), ["//srv/share", "//srv", "/"]);
        assert_eq!(to_tien("///srv/share"), ["///srv/share", "///srv", "/"]);
    }

    #[test]
    fn cham_va_cham_cham_nguyen_van() {
        assert_eq!(to_tien("."), [".", ""]);
        assert_eq!(to_tien(".."), ["..", ""]);
        assert_eq!(to_tien("./a"), ["./a", ".", ""]);
        assert_eq!(to_tien("a/."), ["a/.", "a", ""]);
        assert_eq!(to_tien("a/.."), ["a/..", "a", ""]);
        assert_eq!(to_tien("../a/b"), ["../a/b", "../a", "..", ""]);
        assert_eq!(to_tien("/a/.."), ["/a/..", "/a", "/"]);
        assert_eq!(to_tien("/../a"), ["/../a", "/..", "/"]);
    }

    #[test]
    fn unicode_giu_nguyen_scalar() {
        assert_eq!(
            to_tien("thư/mục/tệp.txt"),
            ["thư/mục/tệp.txt", "thư/mục", "thư", ""]
        );
        assert_eq!(to_tien("đ😀/x"), ["đ😀/x", "đ😀", ""]);
        assert_eq!(to_tien("/đ😀//x/"), ["/đ😀//x", "/đ😀", "/"]);
        assert_eq!(
            to_tien("e\u{0302}\u{0301}/x"),
            ["e\u{0302}\u{0301}/x", "e\u{0302}\u{0301}", ""]
        );
    }

    #[test]
    fn ky_tu_thuong_va_nul() {
        // `\` và NUL là ký tự **thường**, không phải phân cách.
        assert_eq!(to_tien(r"a\b"), [r"a\b", ""]);
        assert_eq!(to_tien(r"\a\b/c"), [r"\a\b/c", r"\a\b", ""]);
        assert_eq!(to_tien("a\0b"), ["a\0b", ""]);
        assert_eq!(to_tien("/a\0b/c"), ["/a\0b/c", "/a\0b", "/"]);
    }
}
