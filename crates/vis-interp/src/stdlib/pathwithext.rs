// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.with_extension)** — thay / đặt / bỏ **phần mở rộng** của **thành phần cuối**
//! đường dẫn, **thuần từ vựng**: không chạm đĩa, không đọc `cwd`, không phân giải symlink,
//! không kiểm tra tồn tại.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | `p` | `ext` | kết quả | ghi chú |
//! |---|---|---|---|
//! | `"a/b.txt"` | `"md"` | `"a/b.md"` | thay phần mở rộng **cuối** |
//! | `"a/b.tar.gz"` | `"xz"` | `"a/b.tar.xz"` | chỉ `gz`, không phải `tar.gz` |
//! | `"a/b"` | `"txt"` | `"a/b.txt"` | chưa có ext ⇒ nối `.` + ext |
//! | `"a/b.txt"` | `""` | `"a/b"` | ext rỗng ⇒ xoá **kể cả** dấu chấm |
//! | `"a/b"` | `""` | `"a/b"` | không ext + ext rỗng ⇒ nguyên văn |
//! | `".gitignore"` | `"txt"` | `".gitignore.txt"` | chấm **đầu tên** không phải ext |
//! | `".gitignore"` | `""` | `".gitignore"` | nguyên văn |
//! | `"a/.a.b"` | `"txt"` | `"a/.a.b.txt"` | **mọi** tên bắt đầu `.` đều không ext (`path.ext`) |
//! | `"a.b/c"` | `"txt"` | `"a.b/c.txt"` | chấm trong **thư mục** không phải ext |
//! | `"a/b/"` | `"md"` | `"a/b.md"` | `/` cuối chỉ đánh dấu thư mục ⇒ bỏ |
//! | `"a/b.txt/"` | `""` | `"a/b"` | bỏ `/` cuối rồi xoá ext |
//! | `"/a/b.txt"` | `"md"` | `"/a/b.md"` | giữ `/` đầu |
//! | `"./a"` | `"txt"` | `"./a.txt"` | giữ `./` **nguyên văn** |
//! | `"a//b"` | `"md"` | `"a//b.md"` | không gộp `/` (khác `path.normalize`) |
//! | `"thư/tệp.txt"` | `"md"` | `"thư/tệp.md"` | Unicode giữ nguyên từng scalar |
//! | `""`, `"/"`, `"///"` | bất kỳ | nguyên văn | không có **tên tệp** ⇒ bỏ qua `ext` |
//! | `"."`, `".."`, `"a/.."` | bất kỳ | nguyên văn | thành phần cuối không phải tên tệp |
//! | `"a/."` | bất kỳ | `"a/."` | **khác** Rust (`a.txt`): `.` cuối không phải tên tệp |
//! | `"foo."` | `"txt"` | `"foo..txt"` | `path.ext("foo.")` = `""` ⇒ coi như **không** ext |
//! | `"foo."` | `""` | `"foo."` | nguyên văn (cùng `path.ext`/`path.stem`) |
//! | `"a/b"` | `".md"` | `"a/b..md"` | `ext` dùng **nguyên văn**, không tước dấu chấm |
//!
//! Phần mở rộng hiện có được nhận bằng **đúng** [`ext_voi`], phần thân bằng **đúng**
//! [`stem_voi`] của `path.ext`/`path.stem`, nên ba hàm không bao giờ lệch quy tắc (đặc biệt
//! tên bắt đầu bằng `.` và dấu chấm cuối). Tiền tố thư mục được giữ **nguyên văn** (kể cả `.`,
//! `..`, `/` lặp, `//` đầu, ổ đĩa/UNC trên Windows); chỉ các phân cách **cuối** bị bỏ khi có
//! tên tệp — cùng `Path::with_extension` của Rust. `ext` **không** bị kiểm tra: mọi byte được
//! nối nguyên văn, rỗng nghĩa là **xoá**.
//!
//! `std.path.with_extension(p: str, ext: str) -> str` — **không** sugar (module-only): tên
//! `with_extension` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`.
//! Gọi sai dạng `s.with_extension(...)` ⇒ `VS0403`; sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::builtin_ls::{basename_voi, ext_voi, la_phan_cach, stem_voi};
use crate::interp::R;
use crate::value::Value;

/// Thay/đặt/bỏ phần mở rộng của **thành phần cuối** `p`; xem bảng hợp đồng ở đầu tệp.
///
/// `windows` chỉ đổi **phân cách** và cách nhận tên tệp — cùng cờ `cfg!(windows)` mà
/// `path.ext`/`path.stem` dùng, nên trên mỗi nền tảng ba hàm luôn khớp nhau.
pub(super) fn voi_phan_mo_rong(p: &str, ext_moi: &str, windows: bool) -> String {
    // `/` cuối chỉ đánh dấu thư mục, không phải tên tệp: bỏ hết để tìm thành phần cuối.
    let cuoi = p.trim_end_matches(|c| la_phan_cach(c, windows));
    if cuoi.is_empty() {
        // `""` hoặc chỉ toàn phân cách (`/`, `///`) — không có tên tệp để đặt ext.
        return p.to_string();
    }
    let ten = basename_voi(p, windows);
    if ten.is_empty() || ten == "." || ten == ".." {
        // Gốc ổ/UNC (`C:`), `.`/`..`: không phải **tên tệp** ⇒ nguyên văn, bỏ qua `ext`.
        return p.to_string();
    }
    // `ext_voi`/`stem_voi` là **đúng** hiện thực của `path.ext`/`path.stem` — không tự tách
    // dấu chấm ở đây, nếu không `.gitignore`/`foo.` sẽ lệch ba hàm.
    let ten_moi = if ext_voi(p, windows).is_empty() {
        if ext_moi.is_empty() {
            ten.clone()
        } else {
            format!("{ten}.{ext_moi}")
        }
    } else if ext_moi.is_empty() {
        stem_voi(p, windows)
    } else {
        format!("{}.{ext_moi}", stem_voi(p, windows))
    };
    // `ten` là **hậu tố** của `cuoi` theo đúng `basename_voi` (kể cả `C:foo` trên Windows);
    // phần còn lại là tiền tố thư mục **nguyên văn** (giữ `.`/`..`/`//` bên trong).
    let tien_to = cuoi.strip_suffix(&ten).unwrap_or("");
    format!("{tien_to}{ten_moi}")
}

/// Dispatch `std.path.with_extension`: kiểm kiểu **động** `str` cho **cả hai** tham số (sai ⇒
/// `VS0501`, như mọi hàm `std.path` nhận `str`), rồi gọi [`voi_phan_mo_rong`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.with_extension", &args[0], 0, span)?;
    let ext = super::chuoi("path.with_extension", &args[1], 1, span)?;
    Ok(Value::Str(voi_phan_mo_rong(p, ext, cfg!(windows))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thay_va_them_phan_mo_rong() {
        assert_eq!(voi_phan_mo_rong("a/b.txt", "md", false), "a/b.md");
        assert_eq!(voi_phan_mo_rong("a/b.tar.gz", "xz", false), "a/b.tar.xz");
        assert_eq!(voi_phan_mo_rong("a/b", "txt", false), "a/b.txt");
        assert_eq!(voi_phan_mo_rong("a", "txt", false), "a.txt");
        assert_eq!(voi_phan_mo_rong("./a", "txt", false), "./a.txt");
        assert_eq!(voi_phan_mo_rong("../a", "md", false), "../a.md");
    }

    #[test]
    fn ext_rong_xoa_hoac_nguyen_van() {
        assert_eq!(voi_phan_mo_rong("a/b.txt", "", false), "a/b");
        assert_eq!(voi_phan_mo_rong("a/b.tar.gz", "", false), "a/b.tar");
        assert_eq!(voi_phan_mo_rong("a/b", "", false), "a/b");
        assert_eq!(voi_phan_mo_rong("a", "", false), "a");
    }

    #[test]
    fn cham_dau_ten_va_nhieu_cham() {
        assert_eq!(
            voi_phan_mo_rong(".gitignore", "txt", false),
            ".gitignore.txt"
        );
        assert_eq!(voi_phan_mo_rong(".gitignore", "", false), ".gitignore");
        assert_eq!(
            voi_phan_mo_rong("a/.gitignore", "txt", false),
            "a/.gitignore.txt"
        );
        // Mọi tên bắt đầu `.` không có ext theo `path.ext` — kể cả `.a.b`.
        assert_eq!(voi_phan_mo_rong(".a.b", "txt", false), ".a.b.txt");
        assert_eq!(voi_phan_mo_rong(".a.b", "", false), ".a.b");
        assert_eq!(voi_phan_mo_rong("a.b/c", "txt", false), "a.b/c.txt");
        assert_eq!(
            voi_phan_mo_rong("a.b.c/d.tar.gz", "xz", false),
            "a.b.c/d.tar.xz"
        );
        assert_eq!(voi_phan_mo_rong("a/b.c.d", "e", false), "a/b.c.e");
    }

    #[test]
    fn slash_cuoi_va_tien_to_nguyen_van() {
        assert_eq!(voi_phan_mo_rong("a/b/", "md", false), "a/b.md");
        assert_eq!(voi_phan_mo_rong("a/b.txt/", "", false), "a/b");
        assert_eq!(voi_phan_mo_rong("a/b.txt//", "md", false), "a/b.md");
        assert_eq!(voi_phan_mo_rong("/a/b/", "md", false), "/a/b.md");
        assert_eq!(voi_phan_mo_rong("a//b", "md", false), "a//b.md");
        assert_eq!(voi_phan_mo_rong("a/./b.txt", "md", false), "a/./b.md");
        assert_eq!(
            voi_phan_mo_rong("//srv/share/x.txt", "md", false),
            "//srv/share/x.md"
        );
    }

    #[test]
    fn khong_co_ten_tep_thi_nguyen_van() {
        for p in ["", "/", "///", ".", "..", "a/..", "a/."] {
            assert_eq!(voi_phan_mo_rong(p, "txt", false), p, "p={p:?}");
            assert_eq!(voi_phan_mo_rong(p, "", false), p, "p={p:?}");
        }
    }

    #[test]
    fn ext_nguyen_van_va_dau_cham_cuoi() {
        assert_eq!(voi_phan_mo_rong("a/b", "tar.gz", false), "a/b.tar.gz");
        assert_eq!(voi_phan_mo_rong("a/b.txt", ".md", false), "a/b..md");
        // `path.ext("foo.")` = `""` ⇒ coi như **không** có ext rồi nối thêm.
        assert_eq!(voi_phan_mo_rong("foo.", "txt", false), "foo..txt");
        assert_eq!(voi_phan_mo_rong("foo.", "", false), "foo.");
        // POSIX: `\` là ký tự thường, không phải phân cách.
        assert_eq!(voi_phan_mo_rong(r"a\b.txt", "md", false), r"a\b.md");
    }

    #[test]
    fn unicode_giu_nguyen_scalar() {
        assert_eq!(
            voi_phan_mo_rong("thư/mục/tệp.txt", "md", false),
            "thư/mục/tệp.md"
        );
        assert_eq!(voi_phan_mo_rong("đ😀.txt", "md", false), "đ😀.md");
        assert_eq!(
            voi_phan_mo_rong("e\u{0302}\u{0301}.txt", "md", false),
            "e\u{0302}\u{0301}.md"
        );
    }

    #[test]
    fn windows_dung_cung_helper_path_ext_stem() {
        assert_eq!(voi_phan_mo_rong(r"C:\a\b.txt", "md", true), r"C:\a\b.md");
        assert_eq!(voi_phan_mo_rong("C:/a/b.txt", "md", true), "C:/a/b.md");
        assert_eq!(voi_phan_mo_rong(r"C:\a\b.txt\", "md", true), r"C:\a\b.md");
        // Drive-relative: `C:foo.txt` giữ `C:` (không mất ổ đĩa).
        assert_eq!(voi_phan_mo_rong("C:foo.txt", "md", true), "C:foo.md");
        assert_eq!(
            voi_phan_mo_rong(r"\\server\share\x.txt", "md", true),
            r"\\server\share\x.md"
        );
        // Gốc ổ không có tên tệp ⇒ nguyên văn.
        assert_eq!(voi_phan_mo_rong(r"C:\", "txt", true), r"C:\");
    }
}
