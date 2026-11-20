// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (path.with_file_name)** — thay **toàn bộ thành phần cuối** (tên tệp, **kể cả** phần
//! mở rộng) của đường dẫn bằng `name`, **giữ nguyên tiền tố** thư mục; **thuần từ vựng**: không
//! chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra tồn tại.
//!
//! Hợp đồng (một hiện thực dùng chung interp + VM):
//!
//! | `p` | `name` | kết quả | ghi chú |
//! |---|---|---|---|
//! | `"a/b.txt"` | `"c"` | `"a/c"` | thay **cả** tên + ext (không giữ `.txt`) |
//! | `"a/b.tar.gz"` | `"c"` | `"a/c"` | không tách stem/ext |
//! | `"a/b"` | `"c"` | `"a/c"` | không ext cũng thay cả tên |
//! | `"a"` | `"c"` | `"c"` | một thành phần, tiền tố rỗng |
//! | `"./a"` | `"x"` | `"./x"` | giữ `./` **nguyên văn** |
//! | `"../a.txt"` | `"x"` | `"../x"` | giữ `../` **nguyên văn** |
//! | `"/a/b.txt"` | `"c"` | `"/a/c"` | giữ `/` đầu |
//! | `"//srv/share/x"` | `"c"` | `"//srv/share/c"` | giữ `//` đầu |
//! | `"a//b"` | `"c"` | `"a//c"` | **không** gộp `/` (khác Rust `a/c`) |
//! | `"a/./b"` | `"c"` | `"a/./c"` | tiền tố **nguyên văn** (khác Rust `a/c`) |
//! | `"a/b/"` | `"c"` | `"a/c"` | `/` cuối chỉ đánh dấu thư mục ⇒ bỏ |
//! | `"a/b//"` | `"c"` | `"a/c"` | bỏ hết `/` cuối |
//! | `"a/"` | `"c"` | `"c"` | thành phần cuối là `a`, tiền tố rỗng |
//! | `"a/b.txt"` | `""` | `"a/"` | tên rỗng ⇒ **bỏ** thành phần cuối, giữ tiền tố + `/` |
//! | `"a/b"` | `""` | `"a/"` | cùng Rust |
//! | `"a"` | `""` | `""` | tiền tố rỗng |
//! | `"/a/b"` | `""` | `"/a/"` | giữ `/` đầu |
//! | `""`, `"/"`, `"///"` | bất kỳ | nguyên văn | không có **tên tệp** ⇒ bỏ qua `name` |
//! | `"."`, `".."`, `"a/.."`, `"a/."` | bất kỳ | nguyên văn | thành phần cuối không phải tên tệp |
//! | `"a/b/."`, `"a/b/.."` | bất kỳ | nguyên văn | **khác** Rust (`a/x`, `a/b/../x`) |
//! | `".gitignore"`, `"foo."` | `"c"` | `"c"` | thay cả tên, không tách ext |
//! | `"a/b.txt"` | `"c.d"` | `"a/c.d"` | `name` dùng **nguyên văn** |
//! | `"a/b.txt"` | `"x/y"` | `"a/x/y"` | `/` trong `name` không bị kiểm |
//! | `"a/b.txt"` | `"/x"` | `"a//x"` | tên tuyệt đối **nối nguyên văn** (khác Rust `/x`) |
//! | `"a/b.txt"` | `"x/"` | `"a/x/"` | `/` cuối trong `name` **giữ** |
//! | `"a/b.txt"` | `"\\"` | `"a/\\"` | `\` là ký tự **thường** trên POSIX |
//! | `"a/b.txt"` | NUL | `"a/\0"` | NUL là ký tự **thường**, không panic/kiểm |
//! | `"thư/tệp.txt"` | `"mới"` | `"thư/mới"` | Unicode giữ nguyên từng scalar |
//!
//! **Tên tệp** nhận bằng **đúng** [`basename_voi`] của `path.basename`; phần bị bỏ là **chính**
//! tên đó, tiền tố là các byte còn lại **trước** nó. Vì vậy:
//!
//! - **Không** dùng `path.dirname`: `dirname` bỏ `/` cuối (`"a/b/"` ⇒ `"a"`), trả `"."` cho tên
//!   trần (`"a"` ⇒ `"."`) và `"/"` cho gốc — sẽ không giữ được tiền tố **nguyên văn**.
//! - `name` **không** bị kiểm tra/chuẩn hoá: mọi byte được nối nguyên văn, kể cả `/`, `.`/`..`,
//!   tên tuyệt đối, `\` và NUL; **không** có trần riêng (kết quả dài tối đa `len(prefix) +
//!   len(name)`).
//! - **Không có tên tệp** — `p` rỗng, chỉ toàn phân cách (`/`, `///`), hoặc thành phần cuối là
//!   `.`/`..` (kể cả `.`/`..` trần, `a/.`, `a/..`, `a/b/.`, `a/b/..`, gốc ổ Windows `C:`/`C:\`)
//!   — ⇒ trả `p` **nguyên văn**, **bỏ qua** `name`. Đây là **cố ý** khác Rust: Rust dùng
//!   `Components` (bỏ `.` bên trong, coi `..` cuối là không có tên) rồi `pop`/`push`, nên `""` +
//!   `x` ⇒ `x`, `/` + `x` ⇒ `/x`, `"."` + `x` ⇒ `./x`, `"a/."` + `x` ⇒ `x`, `"a/.."` + `x` ⇒
//!   `a/../x`. (Gốc UNC Windows theo **đúng** `path.basename`: `\\server\share\` coi `share` là
//!   tên tệp — test `windows_dung_cung_helper_path_basename` ghi rõ.)
//! - Tiền tố giữ **nguyên văn** (khác Rust `PathBuf::pop`/`push`): `a//b` ⇒ `a//c` (Rust
//!   `a/c`), `a/./b` ⇒ `a/./c` (Rust `a/c`), `//a` + `""` ⇒ `//` (Rust `/`).
//! - Tên **tuyệt đối** chỉ là chuỗi được nối: `"a/b.txt"` + `"/x"` ⇒ `"a//x"` (Rust thay **cả**
//!   đường dẫn bằng `/x`). Tên rỗng **bỏ** thành phần cuối và **giữ** `/` của tiền tố
//!   (`"a/b.txt"` ⇒ `"a/"`) — cùng Rust cho các ca có tên tệp.
//!
//! Hai quy tắc trên (dùng `basename_voi`, tiền tố nguyên văn) làm `with_file_name` khớp
//! `path.basename`/`path.with_stem`/`path.with_extension` của repo: phần bị bỏ **luôn** là
//! `path.basename(p)`, còn mọi byte khác của `p` được giữ y nguyên.
//!
//! `std.path.with_file_name(p: str, name: str) -> str` — **không** sugar (module-only): tên
//! `with_file_name` không nằm trong bảng `module_sugar`, tránh nhập nhằng receiver `str`.
//! Gọi sai dạng `s.with_file_name(...)` ⇒ `VS0403`; sai kiểu ⇒ `VS0412`; sai arity ⇒ `VS0401`.

use vis_syntax::Span;

use crate::builtin_ls::{basename_voi, la_phan_cach};
use crate::interp::R;
use crate::value::Value;

/// Thay **toàn bộ** thành phần cuối `p` bằng `ten_moi`, giữ tiền tố **nguyên văn**; xem bảng hợp
/// đồng ở đầu tệp.
///
/// `windows` chỉ đổi **phân cách** và cách nhận tên tệp — cùng cờ `cfg!(windows)` mà
/// `path.basename` dùng, nên trên mỗi nền tảng hai hàm luôn khớp nhau.
pub(super) fn voi_ten_tep(p: &str, ten_moi: &str, windows: bool) -> String {
    // `/` cuối chỉ đánh dấu thư mục, không phải tên tệp: bỏ hết để tìm thành phần cuối.
    let cuoi = p.trim_end_matches(|c| la_phan_cach(c, windows));
    if cuoi.is_empty() {
        // `""` hoặc chỉ toàn phân cách (`/`, `///`) — không có tên tệp để thay.
        return p.to_string();
    }
    let ten = basename_voi(p, windows);
    if ten.is_empty() || ten == "." || ten == ".." {
        // Gốc ổ (`C:`, `C:\`), `.`/`..`: không phải **tên tệp** ⇒ nguyên văn, bỏ qua `ten_moi`.
        return p.to_string();
    }
    // `ten` là **hậu tố** của `cuoi` theo đúng `basename_voi` (kể cả `C:foo` trên Windows);
    // phần còn lại là tiền tố thư mục **nguyên văn** (giữ `.`/`..`/`//` bên trong).
    let tien_to = cuoi.strip_suffix(&ten).unwrap_or("");
    debug_assert!(
        cuoi.ends_with(&ten),
        "path.with_file_name: basename_voi phải là hậu tố của phần cuối ({p:?})"
    );
    format!("{tien_to}{ten_moi}")
}

/// Dispatch `std.path.with_file_name`: kiểm kiểu **động** `str` cho **cả hai** tham số (sai ⇒
/// `VS0501`, như mọi hàm `std.path` nhận `str`), rồi gọi [`voi_ten_tep`].
pub(super) fn goi(args: &[Value], span: Span) -> R<Value> {
    let p = super::chuoi("path.with_file_name", &args[0], 0, span)?;
    let name = super::chuoi("path.with_file_name", &args[1], 1, span)?;
    Ok(Value::Str(voi_ten_tep(p, name, cfg!(windows))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thay_toan_bo_ten_giu_tien_to() {
        assert_eq!(voi_ten_tep("a/b.txt", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a/b.tar.gz", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a/b", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a", "c", false), "c");
        assert_eq!(voi_ten_tep("./a", "x", false), "./x");
        assert_eq!(voi_ten_tep("../a.txt", "x", false), "../x");
        assert_eq!(voi_ten_tep("/a/b.txt", "c", false), "/a/c");
        assert_eq!(
            voi_ten_tep("//srv/share/x.txt", "c", false),
            "//srv/share/c"
        );
        assert_eq!(voi_ten_tep("thư/mục/tệp.txt", "mới", false), "thư/mục/mới");
    }

    #[test]
    fn ten_nguyen_van() {
        assert_eq!(voi_ten_tep("a/b.txt", "c.d", false), "a/c.d");
        assert_eq!(voi_ten_tep("a/b.txt", "x.y.z", false), "a/x.y.z");
        assert_eq!(voi_ten_tep("a/b.txt", "x/y", false), "a/x/y");
        assert_eq!(voi_ten_tep("a/b.txt", "/x", false), "a//x");
        assert_eq!(voi_ten_tep("a/b.txt", "x/", false), "a/x/");
        assert_eq!(voi_ten_tep("a/b.txt", ".", false), "a/.");
        assert_eq!(voi_ten_tep("a/b.txt", "..", false), "a/..");
        assert_eq!(voi_ten_tep("a/b.txt", r"x\y", false), r"a/x\y");
        assert_eq!(voi_ten_tep("a/b.txt", "\0", false), "a/\0");
        assert_eq!(voi_ten_tep("a/b.txt", " ", false), "a/ ");
        assert_eq!(voi_ten_tep("a/b.txt", "tệp😀", false), "a/tệp😀");
    }

    #[test]
    fn ten_rong_bo_thanh_phan_cuoi() {
        assert_eq!(voi_ten_tep("a/b.txt", "", false), "a/");
        assert_eq!(voi_ten_tep("a/b", "", false), "a/");
        assert_eq!(voi_ten_tep("a", "", false), "");
        assert_eq!(voi_ten_tep("/a/b", "", false), "/a/");
        assert_eq!(voi_ten_tep("a/b/", "", false), "a/");
        assert_eq!(voi_ten_tep("a/", "", false), "");
        assert_eq!(voi_ten_tep("/", "", false), "/");
        assert_eq!(voi_ten_tep("", "", false), "");
    }

    #[test]
    fn slash_cuoi_va_tien_to_nguyen_van() {
        assert_eq!(voi_ten_tep("a/b/", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a/b//", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a/b.txt//", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a/", "c", false), "c");
        assert_eq!(voi_ten_tep("/a/b/", "c", false), "/a/c");
        assert_eq!(voi_ten_tep("//a/b/", "c", false), "//a/c");
        assert_eq!(voi_ten_tep("a//b", "c", false), "a//c");
        assert_eq!(voi_ten_tep("a/./b", "c", false), "a/./c");
        assert_eq!(voi_ten_tep("a//b", "", false), "a//");
        assert_eq!(voi_ten_tep("a/./b", "", false), "a/./");
        assert_eq!(voi_ten_tep("//a", "", false), "//");
        assert_eq!(voi_ten_tep("/a", "x", false), "/x");
    }

    #[test]
    fn khong_co_ten_tep_thi_nguyen_van() {
        for p in ["", "/", "///", ".", "..", "a/..", "a/.", "a/b/.", "a/b/.."] {
            assert_eq!(voi_ten_tep(p, "c", false), p, "p={p:?}");
            assert_eq!(voi_ten_tep(p, "", false), p, "p={p:?}");
        }
    }

    #[test]
    fn cham_dau_va_cham_cuoi_khong_dac_biet() {
        assert_eq!(voi_ten_tep(".gitignore", "c", false), "c");
        assert_eq!(voi_ten_tep("a/.gitignore", "c", false), "a/c");
        assert_eq!(voi_ten_tep("a/.a.b", "c.txt", false), "a/c.txt");
        assert_eq!(voi_ten_tep("foo.", "c", false), "c");
        assert_eq!(voi_ten_tep("foo.", "", false), "");
        assert_eq!(voi_ten_tep("a/b.", "", false), "a/");
        assert_eq!(voi_ten_tep("a/b.", "x.", false), "a/x.");
    }

    #[test]
    fn unicode_giu_nguyen_scalar() {
        assert_eq!(voi_ten_tep("thư/mục/tệp.txt", "mới", false), "thư/mục/mới");
        assert_eq!(voi_ten_tep("đ😀.txt", "x", false), "x");
        assert_eq!(voi_ten_tep("e\u{0302}\u{0301}.txt", "m", false), "m");
        assert_eq!(
            voi_ten_tep("a/b.txt", "e\u{0302}\u{0301}", false),
            "a/e\u{0302}\u{0301}"
        );
    }

    #[test]
    fn windows_dung_cung_helper_path_basename() {
        assert_eq!(voi_ten_tep(r"C:\a\b.txt", "c", true), r"C:\a\c");
        assert_eq!(voi_ten_tep("C:/a/b.txt", "c", true), "C:/a/c");
        assert_eq!(voi_ten_tep(r"C:\a\b.txt\", "c", true), r"C:\a\c");
        assert_eq!(voi_ten_tep("C:foo.txt", "c", true), "C:c");
        assert_eq!(
            voi_ten_tep(r"\\server\share\x.txt", "c", true),
            r"\\server\share\c"
        );
        // Gốc ổ không có tên tệp ⇒ nguyên văn.
        assert_eq!(voi_ten_tep(r"C:\", "c", true), r"C:\");
        assert_eq!(voi_ten_tep("C:", "c", true), "C:");
        // UNC root: `path.basename` coi `share` là tên ⇒ thay nó (ghi rõ, khác Rust).
        assert_eq!(voi_ten_tep(r"\\server\share\", "c", true), r"\\server\c");
    }
}
