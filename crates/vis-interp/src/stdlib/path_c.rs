// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `path_c` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod pathwithext {
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
    pub(crate) fn voi_phan_mo_rong(p: &str, ext_moi: &str, windows: bool) -> String {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi("path.with_extension", &args[0], 0, span)?;
        let ext = super::super::chuoi("path.with_extension", &args[1], 1, span)?;
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
}

pub(crate) mod pathwithfilename {
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
    pub(crate) fn voi_ten_tep(p: &str, ten_moi: &str, windows: bool) -> String {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi("path.with_file_name", &args[0], 0, span)?;
        let name = super::super::chuoi("path.with_file_name", &args[1], 1, span)?;
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
}

pub(crate) mod pathwithstem {
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
    pub(crate) fn voi_than(p: &str, stem_moi: &str, windows: bool) -> String {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi("path.with_stem", &args[0], 0, span)?;
        let stem = super::super::chuoi("path.with_stem", &args[1], 1, span)?;
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
}
