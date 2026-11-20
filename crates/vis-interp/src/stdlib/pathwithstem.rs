// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.with_stem)** — thay **phần thân** (file stem) của **thành phần cuối** đường
//! dẫn, **giữ nguyên phần mở rộng hiện có**, **thuần từ vựng**: không chạm đĩa, không đọc `cwd`,
//! không phân giải symlink, không kiểm tra tồn tại.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | `p` | `stem` | kết quả | ghi chú |
//! |---|---|---|---|
//! | `"a/b.txt"` | `"c"` | `"a/c.txt"` | thay stem, **giữ** ext `.txt` |
//! | `"a/b.tar.gz"` | `"c"` | `"a/c.gz"` | stem `b.tar`, ext `gz` (đúng `path.stem`/`path.ext`) |
//! | `"a/b"` | `"c"` | `"a/c"` | không ext ⇒ thay **toàn bộ** tên |
//! | `"a"` | `"c"` | `"c"` | một thành phần |
//! | `".gitignore"` | `"c"` | `"c"` | stem = cả tên, **không** ext (`path.ext`) |
//! | `"a/.gitignore"` | `"c"` | `"a/c"` | chấm đầu tên không phải ext |
//! | `".a.b"` | `"c"` | `"c"` | **mọi** tên bắt đầu `.` đều không ext (`path.ext`) |
//! | `"foo."` | `"c"` | `"c"` | `path.ext("foo.") = ""` ⇒ coi như **không** ext |
//! | `"a/b.txt"` | `""` | `"a/.txt"` | stem rỗng, vẫn giữ `.txt` |
//! | `"a/b"` | `""` | `"a/"` | stem rỗng, không ext ⇒ tên rỗng |
//! | `".gitignore"` | `""` | `""` | cả tên là stem ⇒ tên rỗng |
//! | `"a/b/"` | `"c"` | `"a/c"` | `/` cuối chỉ đánh dấu thư mục ⇒ bỏ |
//! | `"a/b.txt//"` | `"c"` | `"a/c.txt"` | bỏ hết `/` cuối |
//! | `"/a/b.txt"` | `"c"` | `"/a/c.txt"` | giữ `/` đầu |
//! | `"./a"` | `"c"` | `"./c"` | giữ `./` **nguyên văn** |
//! | `"a//b"` | `"c"` | `"a//c"` | không gộp `/` (khác `path.normalize`) |
//! | `"a/./b.txt"` | `"c"` | `"a/./c.txt"` | tiền tố giữ nguyên văn |
//! | `"thư/tệp.txt"` | `"mới"` | `"thư/mới.txt"` | Unicode giữ nguyên từng scalar |
//! | `"a/b"` | `"x/y"` | `"a/x/y"` | stem **không** bị kiểm; nối nguyên văn |
//! | `"a/b"` | `"x\\y"` | `"a/x\\y"` | `\` là ký tự **thường** trên POSIX |
//! | `"a/b.txt"` | `"c.d"` | `"a/c.d.txt"` | stem chứa dấu chấm dùng nguyên văn |
//! | `""`, `"/"`, `"///"` | bất kỳ | nguyên văn | không có **tên tệp** ⇒ bỏ qua `stem` |
//! | `"."`, `".."`, `"a/.."` | bất kỳ | nguyên văn | thành phần cuối không phải tên tệp |
//! | `"a/."` | bất kỳ | `"a/."` | **khác** Rust (`c`): `.` cuối không phải tên tệp |
//!
//! Phần thân và phần mở rộng hiện có được nhận bằng **đúng** [`stem_voi`]/[`ext_voi`] của
//! `path.stem`/`path.ext`, nên ba hàm không bao giờ lệch quy tắc (đặc biệt tên bắt đầu bằng `.`
//! và dấu chấm cuối). Tiền tố thư mục được giữ **nguyên văn** (kể cả `.`, `..`, `/` lặp, `//`
//! đầu, ổ đĩa/UNC trên Windows); chỉ các phân cách **cuối** bị bỏ khi có tên tệp.
//!
//! **Probe Rust (toolchain ghim 1.100.0)**: `Path::with_stem` **không tồn tại** trên toolchain
//! này — `E0599: no method named with_stem`, và feature `path_with_stem` cũng không có (`E0635`).
//! Vì vậy hợp đồng lấy từ **đúng** `file_stem()`/`extension()` + `with_file_name()` của Rust
//! (thay stem, giữ ext; `"foo.rs"` + `"bar"` ⇒ `"bar.rs"`) rồi **ghi rõ** các ca lệch do quy tắc
//! `path.ext`/`path.stem` của repo: tên bắt đầu `.` (`.a.b` + `c` ⇒ `c`, Rust `c.b`), `foo.`
//! (+ `c` ⇒ `c`, Rust `c.`), không có tên tệp (nguyên văn, Rust sinh tên mới), tiền tố `a//b`/
//! `a/./b` (nguyên văn, Rust `with_file_name` gộp còn `a/c`).
//!
//! `std.path.with_stem(p: str, stem: str) -> str` — **không** sugar (module-only): tên
//! `with_stem` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`.
//! Gọi sai dạng `s.with_stem(...)` ⇒ `VS0403`; sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::builtin_ls::{basename_voi, ext_voi, la_phan_cach, stem_voi};
use crate::interp::R;
use crate::value::Value;

/// Thay stem của **thành phần cuối** `p` bằng `stem_moi`, giữ ext hiện có; xem bảng hợp đồng ở
/// đầu tệp.
///
/// `windows` chỉ đổi **phân cách** và cách nhận tên tệp — cùng cờ `cfg!(windows)` mà
/// `path.ext`/`path.stem` dùng, nên trên mỗi nền tảng ba hàm luôn khớp nhau.
pub(super) fn voi_than(p: &str, stem_moi: &str, windows: bool) -> String {
    // `/` cuối chỉ đánh dấu thư mục, không phải tên tệp: bỏ hết để tìm thành phần cuối.
    let cuoi = p.trim_end_matches(|c| la_phan_cach(c, windows));
    if cuoi.is_empty() {
        // `""` hoặc chỉ toàn phân cách (`/`, `///`) — không có tên tệp để đặt stem.
        return p.to_string();
    }
    let ten = basename_voi(p, windows);
    if ten.is_empty() || ten == "." || ten == ".." {
        // Gốc ổ/UNC (`C:`), `.`/`..`: không phải **tên tệp** ⇒ nguyên văn, bỏ qua `stem`.
        return p.to_string();
    }
    // `stem_voi`/`ext_voi` là **đúng** hiện thực của `path.stem`/`path.ext`; gộp lại phải bằng
    // `basename_voi` — nếu không, `with_stem` sẽ lệch `path.stem`/`path.ext`.
    let than = stem_voi(p, windows);
    let ext = ext_voi(p, windows);
    let duoi = if ext.is_empty() {
        String::new()
    } else {
        format!(".{ext}")
    };
    debug_assert_eq!(
        ten,
        format!("{than}{duoi}"),
        "path.with_stem: stem_voi/ext_voi phải khớp basename_voi ({p:?})"
    );
    let ten_moi = format!("{stem_moi}{duoi}");
    // `ten` là **hậu tố** của `cuoi` theo đúng `basename_voi` (kể cả `C:foo` trên Windows);
    // phần còn lại là tiền tố thư mục **nguyên văn** (giữ `.`/`..`/`//` bên trong).
    let tien_to = cuoi.strip_suffix(&ten).unwrap_or("");
    format!("{tien_to}{ten_moi}")
}

/// Dispatch `std.path.with_stem`: kiểm kiểu **động** `str` cho **cả hai** tham số (sai ⇒
/// `VS0501`, như mọi hàm `std.path` nhận `str`), rồi gọi [`voi_than`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.with_stem", &args[0], 0, span)?;
    let stem = super::chuoi("path.with_stem", &args[1], 1, span)?;
    Ok(Value::Str(voi_than(p, stem, cfg!(windows))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thay_than_giu_phan_mo_rong() {
        assert_eq!(voi_than("a/b.txt", "c", false), "a/c.txt");
        assert_eq!(voi_than("a/b.tar.gz", "c", false), "a/c.gz");
        assert_eq!(voi_than("a/b.tar.gz", "d.e", false), "a/d.e.gz");
        assert_eq!(voi_than("a", "c", false), "c");
        assert_eq!(voi_than("./a", "c", false), "./c");
        assert_eq!(voi_than("../a.txt", "c", false), "../c.txt");
        assert_eq!(voi_than("/a/b.txt", "c", false), "/a/c.txt");
        assert_eq!(
            voi_than("//srv/share/x.txt", "c", false),
            "//srv/share/c.txt"
        );
        assert_eq!(voi_than("thư/mục/tệp.txt", "mới", false), "thư/mục/mới.txt");
    }

    #[test]
    fn khong_ext_thay_toan_bo_ten() {
        assert_eq!(voi_than("a/b", "c", false), "a/c");
        assert_eq!(voi_than("b", "c", false), "c");
        assert_eq!(voi_than("a/b/", "c", false), "a/c");
        assert_eq!(voi_than("/a/b/", "c", false), "/a/c");
        assert_eq!(voi_than("a//b", "c", false), "a//c");
        assert_eq!(voi_than("a/./b", "c", false), "a/./c");
        // POSIX: `\` là ký tự thường, cả tên là stem.
        assert_eq!(voi_than(r"a\b", "c", false), "c");
    }

    #[test]
    fn cham_dau_ten_va_dau_cham_cuoi() {
        assert_eq!(voi_than(".gitignore", "c", false), "c");
        assert_eq!(voi_than("a/.gitignore", "c", false), "a/c");
        // Mọi tên bắt đầu `.` không có ext theo `path.ext` — kể cả `.a.b`.
        assert_eq!(voi_than(".a.b", "c", false), "c");
        assert_eq!(voi_than("a/.a.b", "c", false), "a/c");
        assert_eq!(voi_than("a/.a.b", "z", false), "a/z");
        assert_eq!(voi_than("foo.", "c", false), "c");
        assert_eq!(voi_than("...", "c", false), "c");
    }

    #[test]
    fn stem_rong_giu_duoi() {
        assert_eq!(voi_than("a/b.txt", "", false), "a/.txt");
        assert_eq!(voi_than("a/b.tar.gz", "", false), "a/.gz");
        assert_eq!(voi_than("/a/b.txt", "", false), "/a/.txt");
        assert_eq!(voi_than("a/b", "", false), "a/");
        assert_eq!(voi_than(".gitignore", "", false), "");
        assert_eq!(voi_than("foo.", "", false), "");
    }

    #[test]
    fn slash_cuoi_va_khong_ten_tep() {
        assert_eq!(voi_than("a/b.txt/", "c", false), "a/c.txt");
        assert_eq!(voi_than("a/b.txt//", "c", false), "a/c.txt");
        assert_eq!(voi_than("a/b.txt/", "", false), "a/.txt");
        for p in ["", "/", "///", ".", "..", "a/..", "a/."] {
            assert_eq!(voi_than(p, "c", false), p, "p={p:?}");
            assert_eq!(voi_than(p, "", false), p, "p={p:?}");
        }
    }

    #[test]
    fn stem_nguyen_van_va_unicode() {
        assert_eq!(voi_than("a/b.txt", "c.d", false), "a/c.d.txt");
        assert_eq!(voi_than("a/b.txt", "x.", false), "a/x..txt");
        assert_eq!(voi_than("a/b.txt", "..", false), "a/...txt");
        assert_eq!(voi_than("a/b.txt", ".x", false), "a/.x.txt");
        assert_eq!(voi_than("a/b", "x/y", false), "a/x/y");
        assert_eq!(voi_than("a/b", r"x\y", false), r"a/x\y");
        assert_eq!(voi_than("a/b", " ", false), "a/ ");
        assert_eq!(voi_than("a/b", "\0", false), "a/\0");
        assert_eq!(voi_than("đ😀.txt", "x", false), "x.txt");
        assert_eq!(voi_than("e\u{0302}\u{0301}.txt", "m", false), "m.txt");
        assert_eq!(voi_than("a/b.txt", "tệp😀", false), "a/tệp😀.txt");
    }

    #[test]
    fn windows_dung_cung_helper_path_ext_stem() {
        assert_eq!(voi_than(r"C:\a\b.txt", "c", true), r"C:\a\c.txt");
        assert_eq!(voi_than("C:/a/b.txt", "c", true), "C:/a/c.txt");
        assert_eq!(voi_than(r"C:\a\b.txt\", "c", true), r"C:\a\c.txt");
        assert_eq!(voi_than("C:foo.txt", "c", true), "C:c.txt");
        assert_eq!(
            voi_than(r"\\server\share\x.txt", "c", true),
            r"\\server\share\c.txt"
        );
        // Gốc ổ không có tên tệp ⇒ nguyên văn.
        assert_eq!(voi_than(r"C:\", "c", true), r"C:\");
    }
}
