// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296c** — gom các module `stdlib` miền `path_a` (giữ nguyên nội dung/đường dẫn).

pub(crate) mod pathancestors {
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

    use vis_core::trap::R;
    use vis_core::value::Value;

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
    pub(crate) fn to_tien(p: &str) -> Vec<&str> {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::super::chuoi("path.ancestors", &args[0], 0, span)?;
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
}

pub(crate) mod pathcomponents {
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

    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Tách **từ vựng** `p` thành các thành phần, theo **đúng** bảng ở đầu tệp.
    ///
    /// Tuyệt đối (mở đầu `/`) ⇒ phần tử đầu `"/"`; mọi thành phần rỗng do `/` lặp/đầu/cuối đều bị
    /// bỏ; `.`/`..` được giữ **nguyên văn** (không phân giải/không chuẩn hoá). Trả các mảnh mượn
    /// của `p` nên không cấp phát chuỗi mới.
    pub(crate) fn thanh_phan(p: &str) -> Vec<&str> {
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
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::super::chuoi("path.components", &args[0], 0, span)?;
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
}

pub(crate) mod pathisabs {
    //! **VD-270 (path.is_absolute)** — `p` có **tuyệt đối** không, theo **đúng** quy tắc từ vựng của
    //! `path.is_abs`: tái dùng [`la_tuyet_doi`] và **cùng cờ nền tảng**, nên
    //! `is_absolute(p) == is_abs(p)` cho **mọi** đầu vào.
    //!
    //! **Thuần chuỗi**: không chạm đĩa, không đọc `cwd`, không phân giải symlink, không kiểm tra
    //! tồn tại.
    //!
    //! | đầu vào | `is_absolute` | ghi chú |
    //! |---|---|---|
    //! | `""` | `false` | đường dẫn rỗng **không** tuyệt đối |
    //! | `"a"`, `"a/b"`, `"."`, `".."` | `false` | không mở đầu bằng `/` |
    //! | `"/"`, `"/a/b"` | `true` | tuyệt đối POSIX |
    //! | `"//"`, `"///a"` | `true` | mở đầu bằng `/` ⇒ tuyệt đối (không gộp `/`) |
    //! | `"a\\b"`, `"a\0b"` | `false` | `\` và NUL là ký tự **thường**, không phải phân cách |
    //! | `"/a\\b"`, `"/a\0b"` | `true` | `/` đầu quyết định |
    //! | `"đ😀/x"` | `false` | thành phần Unicode giữ nguyên từng scalar |
    //!
    //! Không có lỗi dữ liệu/môi trường ⇒ **không trap**. Chữ ký `std.path.is_absolute(p: str) ->
    //! bool` — **không** sugar R5c (module-only): tên `is_absolute` không nằm trong bảng
    //! `module_sugar`, tránh nhập nhằng receiver `str`/`path`; gọi `s.is_absolute()` ⇒ `VS0403`,
    //! sai kiểu ⇒ `VS0412`, sai arity ⇒ `VS0401`.
    //!
    //! **Windows** (`cfg!(windows)`): `/` đơn và `/a` **không** tuyệt đối; `//…`/`\\…` hoặc
    //! `<ổ>:` + `/`/`\` mới tuyệt đối (đúng `Path::is_absolute` của Windows). Các dòng `/`, `/\a`
    //! trong bảng trên là quy tắc POSIX; test có nhánh Windows riêng.

    use vis_syntax::Span;

    use crate::builtin::ls::la_tuyet_doi;
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// `true` khi [`la_tuyet_doi`] nói `true` — **cùng** quy tắc và cùng cờ nền tảng với
    /// `path.is_abs`, nên đây đúng là cùng một hợp đồng.
    pub(crate) fn tuyet_doi(p: &str) -> bool {
        la_tuyet_doi(p, cfg!(windows))
    }

    /// Dispatch `std.path.is_absolute`: kiểm kiểu **động** `str` (sai ⇒ `VS0501`), rồi gọi
    /// [`tuyet_doi`].
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let p = super::super::super::chuoi("path.is_absolute", &args[0], 0, span)?;
        Ok(Value::Bool(tuyet_doi(p)))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn rong_va_tuong_doi() {
            assert!(!tuyet_doi(""));
            assert!(!tuyet_doi("a"));
            assert!(!tuyet_doi("a/b"));
            assert!(!tuyet_doi("."));
            assert!(!tuyet_doi(".."));
            assert!(!tuyet_doi("../a"));
            assert!(!tuyet_doi("./a/"));
        }

        #[test]
        fn tuyet_doi_posix() {
            #[cfg(not(windows))]
            {
                assert!(tuyet_doi("/"));
                assert!(tuyet_doi("/a"));
                assert!(tuyet_doi("/a/b"));
                // Không gộp `/`: `//` vẫn mở đầu bằng `/` ⇒ tuyệt đối.
                assert!(tuyet_doi("//"));
                assert!(tuyet_doi("///a"));
            }
            #[cfg(windows)]
            {
                // Windows: `/` đơn không có ổ đĩa/UNC ⇒ KHÔNG tuyệt đối (đúng `Path::is_absolute`).
                assert!(!tuyet_doi("/"));
                assert!(!tuyet_doi("/a"));
                assert!(!tuyet_doi("/a/b"));
                assert!(tuyet_doi("//"));
                assert!(tuyet_doi("///a"));
            }
        }

        #[test]
        fn trung_khit_is_abs() {
            for p in [
                "", "/", "a", "/a", "//", "//a", ".", "..", "../a", "đ😀", "a\\b",
            ] {
                assert_eq!(
                    tuyet_doi(p),
                    la_tuyet_doi(p, cfg!(windows)),
                    "is_absolute phải trùng khít is_abs cho {p:?}"
                );
            }
        }

        #[test]
        fn ky_tu_thuong_unicode_va_nul() {
            // `\` và NUL là ký tự **thường** trên POSIX; trên Windows `\` là phân cách (và `/a\b`
            // không có ổ đĩa ⇒ vẫn tương đối).
            assert!(!tuyet_doi(r"a\b"));
            assert!(!tuyet_doi("a\0b"));
            #[cfg(not(windows))]
            {
                assert!(tuyet_doi(r"/a\b"));
                assert!(tuyet_doi("/a\0b"));
            }
            #[cfg(windows)]
            {
                assert!(!tuyet_doi(r"/a\b"));
                assert!(!tuyet_doi("/a\0b"));
            }
            // Thành phần Unicode giữ nguyên từng scalar, kể cả tổ hợp.
            assert!(!tuyet_doi("thư/mục/tệp.txt"));
            assert!(!tuyet_doi("đ😀"));
            assert!(!tuyet_doi("đ😀/x"));
            #[cfg(not(windows))]
            assert!(tuyet_doi("/thư/mục"));
            #[cfg(windows)]
            assert!(!tuyet_doi("/thư/mục"));
            assert!(!tuyet_doi("e\u{0302}\u{0301}/x"));
        }
    }
}

pub(crate) mod pathjoinall {
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
    //! Mỗi bước gọi **đúng** helper của `path.join` ([`crate::builtin::ls::noi_duong_voi`]), nên
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

    use crate::builtin::ls::noi_duong_voi;
    use vis_core::trap::R;
    use vis_core::value::Value;

    /// Gấp trái `parts` bằng **đúng** `path.join`: `acc` khởi đầu `""` (nên `[]` ⇒ `""`, một phần
    /// tử ⇒ chính nó), mỗi bước `acc = noi_duong_voi(acc, phần_tử, windows)`.
    pub(crate) fn noi_tat_ca(parts: &[&str], windows: bool) -> String {
        let mut acc = String::new();
        for p in parts {
            acc = noi_duong_voi(&acc, p, windows);
        }
        acc
    }

    /// Dispatch `std.path.join_all`: kiểm **động** `list` (sai ⇒ `VS0501`), kiểm từng phần tử `str`,
    /// rồi gọi [`noi_tat_ca`] với `cfg!(windows)` — cùng nền tảng `path.join`.
    pub(crate) fn goi(args: &[Value], span: Span) -> R<Value> {
        let parts = super::super::super::danh_sach("path.join_all", &args[0], 0, span)?;
        let mut ds: Vec<&str> = Vec::with_capacity(parts.len());
        for (i, v) in parts.iter().enumerate() {
            match v {
                Value::Str(s) => ds.push(s),
                other => {
                    return Err(super::super::super::loi(
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
}
