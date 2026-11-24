// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `path_b` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod pathnorm {
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

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Tách **từ vựng** `p` thành `(tuyệt_đối, thành_phần)` — cùng một hiện thực cho
    /// [`chuan_hoa`] và `path.relative` (`super::super::pathrel`), nên hai hàm không bao giờ lệch quy tắc.
    ///
    /// Thành phần đã bỏ `/` lặp và `.`; `..` đã phân giải với thành phần **thường** đứng trước (hoặc
    /// bị bỏ ở gốc tuyệt đối, hoặc được giữ ở đầu chuỗi tương đối). Đầu vào rỗng ⇒ `(false, [])`.
    pub(crate) fn tach(p: &str) -> (bool, Vec<&str>) {
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
        (tuyet_doi, phan)
    }

    /// Chuẩn hoá **từ vựng** `p` theo bảng ở đầu tệp; không cấp phát gì ngoài `Vec` thành phần và
    /// chuỗi kết quả.
    pub(crate) fn chuan_hoa(p: &str) -> String {
        let (tuyet_doi, phan) = tach(p);
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi("path.normalize", &args[0], 0, span)?;
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
}

pub(crate) mod pathrel {
    //! **VD-270 (path.relative)** — đường dẫn **tương đối** từ `from` tới `to`, **thuần từ vựng
    //! POSIX**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra tồn tại.
    //!
    //! Hai đối số được tách bằng **đúng** helper của `path.normalize` ([`super::super::pathnorm::tach`]),
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

    use vis_core::trap::R;
    use vis_core::value::Value;

    use super::super::pathnorm::{chuan_hoa, tach};

    /// Đường dẫn tương đối **từ** `from` **tới** `to` theo bảng ở đầu tệp.
    pub(crate) fn tuong_doi(from: &str, to: &str) -> String {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let from = super::super::chuoi("path.relative", &args[0], 0, span)?;
        let to = super::super::chuoi("path.relative", &args[1], 1, span)?;
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
}

pub(crate) mod pathrelbool {
    //! **VD-270 (path.is_relative)** — `p` có **tương đối** không, theo **đúng** quy tắc từ vựng của
    //! `path.is_abs`: **thuần chuỗi**, không chạm đĩa, không đọc `cwd`, không phân giải symlink,
    //! không kiểm tra tồn tại.
    //!
    //! Hợp đồng (một hiện thực dùng chung interp + VM): `is_relative(p) == !is_abs(p)` — cùng
    //! [`la_tuyet_doi`] và cùng cờ nền tảng, nên hai hàm **không bao giờ** lệch nhau (kể cả trên
    //! Windows, nơi `path.is_abs` nhận ổ đĩa/UNC).
    //!
    //! | đầu vào | `is_relative` | ghi chú |
    //! |---|---|---|
    //! | `""` | `true` | đường dẫn rỗng là **tương đối** |
    //! | `"a"`, `"a/b"`, `"."`, `".."` | `true` | không mở đầu bằng `/` |
    //! | `"//"`, `"///"` | `false` | mở đầu bằng `/` ⇒ tuyệt đối (không gộp `/`) |
    //! | `"/"`, `"/a/b"` | `false` | tuyệt đối POSIX |
    //! | `"a\\b"`, `"a\0b"` | `true` | `\` và NUL là ký tự **thường**, không phải phân cách |
    //! | `"đ😀/x"` | `true` | thành phần Unicode giữ nguyên từng scalar |
    //!
    //! Không có lỗi dữ liệu/môi trường ⇒ **không trap**. Chữ ký `std.path.is_relative(p: str) ->
    //! bool` — **không** sugar R5c (module-only): tên `is_relative` không nằm trong bảng
    //! `module_sugar`, tránh nhập nhằng receiver `str`/`path`; gọi `s.is_relative()` ⇒ `VS0403`,
    //! sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.
    //!
    //! **Windows** (`cfg!(windows)`): `/` đơn là **tương đối** (thiếu ổ đĩa/UNC), nên
    //! `is_relative("/") == true`; `//…` hoặc `<ổ>:` + phân cách mới tuyệt đối. Bảng trên là quy
    //! tắc POSIX; test có nhánh Windows riêng.

    use vis_syntax::Span;

    use crate::builtin::ls::la_tuyet_doi;
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// `true` khi [`la_tuyet_doi`] nói `false` — **cùng** quy tắc POSIX và cùng cờ nền tảng với
    /// `path.is_abs`, nên đây đúng là phần bù logic của hàm đó.
    pub(crate) fn la_tuong_doi(p: &str) -> bool {
        !la_tuyet_doi(p, cfg!(windows))
    }

    /// Dispatch `std.path.is_relative`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
    /// [`la_tuong_doi`].
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi("path.is_relative", &args[0], 0, span)?;
        Ok(Value::Bool(la_tuong_doi(p)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn rong_va_tuong_doi() {
            assert!(la_tuong_doi(""));
            assert!(la_tuong_doi("a"));
            assert!(la_tuong_doi("a/b"));
            assert!(la_tuong_doi("."));
            assert!(la_tuong_doi(".."));
            assert!(la_tuong_doi("../a"));
            assert!(la_tuong_doi("./a/"));
        }

        #[test]
        fn tuyet_doi_posix() {
            #[cfg(not(windows))]
            {
                assert!(!la_tuong_doi("/"));
                assert!(!la_tuong_doi("/a"));
                assert!(!la_tuong_doi("/a/b"));
                // Không gộp `/`: `//` vẫn mở đầu bằng `/` ⇒ tuyệt đối.
                assert!(!la_tuong_doi("//"));
                assert!(!la_tuong_doi("///a"));
            }
            #[cfg(windows)]
            {
                // Windows: `/` đơn không có ổ đĩa/UNC ⇒ tương đối (bù của `is_absolute`).
                assert!(la_tuong_doi("/"));
                assert!(la_tuong_doi("/a"));
                assert!(la_tuong_doi("/a/b"));
                assert!(!la_tuong_doi("//"));
                assert!(!la_tuong_doi("///a"));
            }
        }

        #[test]
        fn dung_la_phan_bu_cua_is_abs() {
            for p in [
                "", "/", "a", "/a", "//", "//a", ".", "..", "../a", "đ😀", "a\\b",
            ] {
                assert_eq!(
                    la_tuong_doi(p),
                    !la_tuyet_doi(p, cfg!(windows)),
                    "is_relative phải là phần bù của is_abs cho {p:?}"
                );
            }
        }

        #[test]
        fn ky_tu_thuong_unicode_va_nul() {
            // `\` và NUL là ký tự **thường**, không phải phân cách.
            assert!(la_tuong_doi(r"a\b"));
            assert!(la_tuong_doi("a\0b"));
            // Thành phần Unicode giữ nguyên từng scalar, kể cả tổ hợp.
            assert!(la_tuong_doi("thư/mục/tệp.txt"));
            assert!(la_tuong_doi("đ😀"));
            assert!(la_tuong_doi("e\u{0302}\u{0301}/x"));
        }
    }
}

pub(crate) mod pathstripprefix {
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

    use vis_core::trap::R;
    use vis_core::value::Value;

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
    pub(crate) fn bo_tien_to<'a>(p: &'a str, base: &str) -> Option<&'a str> {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::chuoi("path.strip_prefix", &args[0], 0, span)?;
        let base = super::super::chuoi("path.strip_prefix", &args[1], 1, span)?;
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
            use vis_core::value::eq;
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
}
