// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-58 (VD-94/VD-99)** — `Ty::Any` **xoá hẳn**: chữ ký `std.*` dùng **biến kiểu** (`Var`/`List`/`Map`).
//!
//! Khoá ba việc:
//! 1. **Không còn** `Ty::Any` — *bảo đảm ở mức biên dịch* (biến thể đã xoá khỏi `enum Ty`; test này chỉ
//!    còn khẳng định **mọi** chỗ trong bảng đều hạ được ra `Type` và `Var` chỉ dùng `0..3`).
//! 2. 16 chữ ký đã đổi có **đúng hình dạng** (`list[T]`, `map[K,V]`, trả `T`/`list[K]`/…).
//! 3. `sang_type()` **tổng** (không `Option`): `Var(i)` → `TypeRef` tên `T`/`K`/`V`.

use vis_types::stdlib::{sig_builtin, sig_of, Ty, FUNCS};
use vis_types::Type;

/// Tên biến kiểu theo chỉ số (VD-99 (1): `0=T, 1=K, 2=V`).
const TEN_BIEN: [&str; 3] = ["T", "K", "V"];

fn kiem_var(t: Ty) {
    if let Ty::Var(i) = t {
        assert!((i as usize) < TEN_BIEN.len(), "Ty::Var({i}) ngoài 0..3");
    }
    match t {
        Ty::List(x) => kiem_var(*x),
        Ty::Map(k, v) => {
            kiem_var(*k);
            kiem_var(*v);
        }
        _ => {}
    }
}

#[test]
fn moi_chu_ky_ha_duoc_va_bien_kieu_trong_0_3() {
    for f in &FUNCS {
        for p in f.params {
            kiem_var(*p);
            // `FnKhoa` là **ngoại lệ có ghi** ⇒ hạ thành `{error}` (tầng chạy kiểm, ST-56);
            // còn lại **mọi** tham số phải hạ được (tổng).
            if matches!(p, Ty::FnKhoa) {
                assert_eq!(p.sang_type(), Type::Error);
                continue;
            }
            assert_ne!(
                p.sang_type(),
                Type::Error,
                "{}: tham số không hạ được",
                f.name
            );
        }
        kiem_var(f.ret);
        assert_ne!(
            f.ret.sang_type(),
            Type::Unit,
            "{}: ret hạ thành unit",
            f.name
        );
    }
    // Bảng chữ ký `std.*` mở rộng qua nhiều VD-270; mỗi hàm mới có test chữ ký riêng và mốc
    // chi tiết trong `CHANGELOG.md` / `agent/JOURNAL/vd260-report.md`. Assertion dưới khoá tổng.
    assert_eq!(FUNCS.len(), 167, "bảng chữ ký phải đủ 167 hàm");
}

#[test]
fn fs_mkdir_all_kieu_str_result_va_khong_sugar() {
    // **VD-270 (mkdir-all)**: `(str) -> Result[(), Error]`; fs là module-only nên **không** sugar
    // (`p.mkdir_all()` không được ánh xạ sang `fs`).
    let f = sig_of("fs.mkdir_all").expect("fs.mkdir_all");
    assert_eq!(f.params, &[Ty::Str], "`fs.mkdir_all`: tham số");
    assert_eq!(f.ret, Ty::ResultUnitStr, "`fs.mkdir_all`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Unit), Box::new(Ty::Error.sang_type())),
        "`fs.mkdir_all`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "mkdir_all"), None);
}

#[test]
fn fs_is_symlink_kieu_str_result_va_khong_sugar() {
    // **VD-270 (fs-is-symlink)**: `(str) -> Result[bool, Error]`, tái dùng `Ty::ResultBoolErr`
    // (cùng biến thể của `fs.is_file`/`fs.is_dir`); fs là module-only nên **không** sugar —
    // `p.is_symlink()` bị `VS0403`, không ánh xạ sang `fs`.
    let f = sig_of("fs.is_symlink").expect("fs.is_symlink");
    assert_eq!(f.params, &[Ty::Str], "`fs.is_symlink`: tham số");
    assert_eq!(
        f.ret,
        Ty::ResultBoolErr,
        "`fs.is_symlink`: kiểu trả phải tái dùng `ResultBoolErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Bool), Box::new(Ty::Error.sang_type())),
        "`fs.is_symlink`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_symlink"), None);
}

#[test]
fn fs_read_link_kieu_str_result_va_khong_sugar() {
    // **VD-270 (fs-read-link)**: `(str) -> Result[str, Error]`, tái dùng `Ty::ResultStrStr`
    // (cùng biến thể của `fs.read`); fs là module-only nên **không** sugar — `p.read_link()` bị
    // `VS0403`, không ánh xạ sang `fs`.
    let f = sig_of("fs.read_link").expect("fs.read_link");
    assert_eq!(f.params, &[Ty::Str], "`fs.read_link`: tham số");
    assert_eq!(
        f.ret,
        Ty::ResultStrStr,
        "`fs.read_link`: kiểu trả phải tái dùng `ResultStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Str), Box::new(Ty::Error.sang_type())),
        "`fs.read_link`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "read_link"), None);
}

#[test]
fn fs_current_dir_params_rong_result_va_khong_sugar() {
    // **VD-270 (fs-current-dir)**: **không tham số** (`params` rỗng) và trả
    // `Result[str, Error]` qua **đúng** `Ty::ResultStrErr` (không phải `ResultStrStr` dù hạ
    // cùng kiểu); fs là module-only nên **không** sugar — `p.current_dir()` bị `VS0403`,
    // không ánh xạ sang `fs`.
    let f = sig_of("fs.current_dir").expect("fs.current_dir");
    assert!(f.params.is_empty(), "`fs.current_dir`: params phải rỗng");
    assert_eq!(
        f.ret,
        Ty::ResultStrErr,
        "`fs.current_dir`: kiểu trả phải là biến thể `ResultStrErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Str), Box::new(Ty::Error.sang_type())),
        "`fs.current_dir`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[str, Error]");
    assert_eq!(vis_types::stdlib::module_sugar("str", "current_dir"), None);
}

#[test]
fn fs_canonicalize_kieu_str_result_va_khong_sugar() {
    // **VD-270 (fs-canonicalize)**: `(str) -> Result[str, Error]`, tái dùng `Ty::ResultStrStr`
    // (cùng biến thể của `fs.read`/`fs.read_link`); fs là module-only nên **không** sugar —
    // `p.canonicalize()` bị `VS0403`, không ánh xạ sang `fs`.
    let f = sig_of("fs.canonicalize").expect("fs.canonicalize");
    assert_eq!(f.params, &[Ty::Str], "`fs.canonicalize`: tham số");
    assert_eq!(
        f.ret,
        Ty::ResultStrStr,
        "`fs.canonicalize`: kiểu trả phải tái dùng `ResultStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Str), Box::new(Ty::Error.sang_type())),
        "`fs.canonicalize`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "canonicalize"), None);
}

#[test]
fn splitn_kieu_str_str_int_va_sugar() {
    // **VD-270 (splitn)**: chữ ký `(str, str, int) -> list[str]`; tham số đầu `str` để sugar
    // R5c `s.splitn(sep, n)` nhận, `n` là `int` (không phải `str`).
    let f = sig_of("str.splitn").expect("str.splitn");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str, Ty::Int],
        "`str.splitn`: tham số"
    );
    assert_eq!(f.ret, Ty::ListStr, "`str.splitn`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`str.splitn`: hạ kiểu trả"
    );
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "splitn"), None);
}

#[test]
fn split_once_kieu_str_str_va_sugar() {
    // **VD-270 (split-once)**: chữ ký `(str, str) -> list[str]?`; tham số đầu `str` để sugar
    // R5c `s.split_once(sep)` nhận. `Ty` không có `Opt` tổng quát nên dùng `OptListStr`
    // (`list[str]?` = `Option[list[str]]`), không tái dùng `ListStr`/`OptStr`.
    let f = sig_of("str.split_once").expect("str.split_once");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`str.split_once`: tham số");
    assert_eq!(f.ret, Ty::OptListStr, "`str.split_once`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::List(Box::new(Type::Str)))),
        "`str.split_once`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]?", "`str.split_once`: tên hiển thị");
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "split_once"), None);
}

#[test]
fn rsplit_once_kieu_str_str_va_sugar() {
    // **VD-270 (rsplit-once)**: chữ ký `(str, str) -> list[str]?`; tham số đầu `str` để sugar
    // R5c `s.rsplit_once(sep)` nhận. Tái dùng **đúng** `Ty::OptListStr` của `split_once`
    // (`list[str]?` = `Option[list[str]]`), không thêm biến thể `Ty` và không tái dùng
    // `ListStr`/`OptStr`.
    let f = sig_of("str.rsplit_once").expect("str.rsplit_once");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`str.rsplit_once`: tham số");
    assert_eq!(f.ret, Ty::OptListStr, "`str.rsplit_once`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::List(Box::new(Type::Str)))),
        "`str.rsplit_once`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]?", "`str.rsplit_once`: tên hiển thị");
    assert_eq!(f.ret, sig_of("str.split_once").unwrap().ret);
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "rsplit_once"), None);
}

#[test]
fn rsplit_kieu_str_str_list_str_va_sugar() {
    // **VD-270 (rsplit)**: chữ ký `(str, str) -> list[str]`; tham số đầu `str` để sugar R5c
    // `s.rsplit(sep)` nhận. Tái dùng **đúng** `Ty::ListStr` của `str.split`/`str.splitn`,
    // không thêm biến thể `Ty` và không có lớp `Option`.
    let f = sig_of("str.rsplit").expect("str.rsplit");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`str.rsplit`: tham số");
    assert_eq!(f.ret, Ty::ListStr, "`str.rsplit`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`str.rsplit`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]", "`str.rsplit`: tên hiển thị");
    assert_eq!(f.ret, sig_of("str.split").unwrap().ret);
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "rsplit"), None);
}

#[test]
fn split_inclusive_kieu_str_str_list_str_va_sugar() {
    // **VD-270 (split-inclusive)**: chữ ký `(str, str) -> list[str]`; tham số đầu `str` để
    // sugar R5c `s.split_inclusive(sep)` nhận. Tái dùng **đúng** `Ty::ListStr` (cùng biến thể
    // của `str.split`/`str.splitn`), không thêm biến thể `Ty` và không có lớp `Option`.
    let f = sig_of("str.split_inclusive").expect("str.split_inclusive");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`str.split_inclusive`: tham số"
    );
    assert_eq!(f.ret, Ty::ListStr, "`str.split_inclusive`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`str.split_inclusive`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]", "`str.split_inclusive`: tên");
    assert_eq!(f.ret, sig_of("str.split").unwrap().ret);
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "split_inclusive"),
        None
    );
}

#[test]
fn split_terminator_kieu_str_str_list_str_va_sugar() {
    // **VD-270 (split-terminator)**: chữ ký `(str, str) -> list[str]`; tham số đầu `str` để
    // sugar R5c `s.split_terminator(sep)` nhận. Tái dùng **đúng** `Ty::ListStr` (cùng biến thể
    // của `str.split`/`str.splitn`/`str.split_inclusive`), không thêm biến thể `Ty`.
    let f = sig_of("str.split_terminator").expect("str.split_terminator");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`str.split_terminator`: tham số"
    );
    assert_eq!(f.ret, Ty::ListStr, "`str.split_terminator`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`str.split_terminator`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]", "`str.split_terminator`: tên");
    assert_eq!(f.ret, sig_of("str.split").unwrap().ret);
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "split_terminator"),
        None
    );
}

#[test]
fn flatten_kieu_long_mot_cap() {
    // **VD-270 (flatten)**: tham số `list[list[T]]`, trả `list[T]`; chỉ **một** cấp nên
    // `list[list[list[int]]]` ⇒ `list[list[int]]` (phần tử list không bị đệ quy làm phẳng).
    let f = sig_of("list.flatten").expect("list.flatten");
    assert_eq!(
        f.params,
        &[Ty::List(&Ty::List(&Ty::Var(0)))],
        "`list.flatten`: tham số"
    );
    assert_eq!(f.ret, Ty::List(&Ty::Var(0)), "`list.flatten`: kiểu trả");
    let long = Ty::List(&Ty::List(&Ty::Var(0))).sang_type();
    assert_eq!(
        f.params[0].sang_type(),
        long,
        "`list.flatten`: hạ `list[list[T]]`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::TypeRef {
            name: "T".into(),
            args: Vec::new()
        })),
        "`list.flatten`: hạ `list[T]`"
    );
}

#[test]
fn chunks_exact_kieu_list_long_va_khong_bien_the_ty_moi() {
    // **VD-270 (chunks-exact)**: `(list[T], int) -> list[list[T]]`; tái dùng đúng hình dạng
    // `Ty::List(&Ty::List(&Ty::Var(0)))` của `list.chunk`/`list.windows` — **không** thêm biến
    // thể `Ty`; `T` buộc từ `xs` nên sugar R5c `xs.chunks_exact(n)` hợp lệ và trả kiểu lồng.
    let f = sig_of("list.chunks_exact").expect("list.chunks_exact");
    assert_eq!(
        f.params,
        &[Ty::List(&Ty::Var(0)), Ty::Int],
        "`list.chunks_exact`: tham số"
    );
    assert_eq!(
        f.ret,
        Ty::List(&Ty::List(&Ty::Var(0))),
        "`list.chunks_exact`: kiểu trả"
    );
    assert_eq!(
        f.params[0].sang_type(),
        Ty::List(&Ty::Var(0)).sang_type(),
        "`list.chunks_exact`: hạ `list[T]`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Ty::List(&Ty::List(&Ty::Var(0))).sang_type(),
        "`list.chunks_exact`: hạ `list[list[T]]`"
    );
    let chunk = sig_of("list.chunk").expect("list.chunk");
    assert_eq!(
        chunk.ret, f.ret,
        "`list.chunks_exact` phải cùng kiểu trả với `list.chunk`"
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "chunks_exact"),
        None,
        "sugar `chunks_exact` thuộc `list`, không phải `str`"
    );
}

#[test]
fn path_ancestors_kieu_str_sang_list_str_va_khong_sugar() {
    // **VD-270 (path.ancestors)**: chữ ký `(str) -> list[str]`, tái dùng `Ty::ListStr`;
    // **không** sugar (`ancestors` không nằm trong `module_sugar`, tránh đoán module nhận).
    let f = sig_of("path.ancestors").expect("path.ancestors");
    assert_eq!(f.params, &[Ty::Str], "`path.ancestors`: tham số");
    assert_eq!(f.ret, Ty::ListStr, "`path.ancestors`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`path.ancestors`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "ancestors"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "ancestors"), None);
}

#[test]
fn path_components_kieu_str_sang_list_str_va_khong_sugar() {
    // **VD-270 (path.components)**: chữ ký `(str) -> list[str]`, tái dùng `Ty::ListStr`;
    // **không** sugar (`components` không nằm trong `module_sugar`, tránh đoán module nhận).
    let f = sig_of("path.components").expect("path.components");
    assert_eq!(f.params, &[Ty::Str], "`path.components`: tham số");
    assert_eq!(f.ret, Ty::ListStr, "`path.components`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`path.components`: hạ kiểu trả"
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "components"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "components"), None);
}

#[test]
fn path_normalize_kieu_str_sang_str_va_sugar() {
    // **VD-270 (path.normalize)**: chữ ký `(str) -> str`; sugar R5c của `str` chỉ mở đúng
    // `normalize` → `path`, không mở các hàm `path.*` khác (tránh đổi nghĩa `s.join(...)`).
    let f = sig_of("path.normalize").expect("path.normalize");
    assert_eq!(f.params, &[Ty::Str], "`path.normalize`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.normalize`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.normalize`: hạ kiểu");
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "normalize"),
        Some("path")
    );
    assert_eq!(vis_types::stdlib::module_sugar("str", "upper"), None);
    assert_eq!(vis_types::stdlib::module_sugar("str", "join"), None);
}

#[test]
fn path_is_absolute_kieu_str_sang_bool_va_khong_sugar() {
    // **VD-270 (path.is_absolute)**: chữ ký `(str) -> bool` — **cùng** quy tắc của
    // `path.is_abs`; **không** sugar (`is_absolute` không nằm trong `module_sugar`, tránh
    // đoán module nhận).
    let f = sig_of("path.is_absolute").expect("path.is_absolute");
    assert_eq!(f.params, &[Ty::Str], "`path.is_absolute`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`path.is_absolute`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`path.is_absolute`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_absolute"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "is_absolute"), None);
}

#[test]
fn path_is_relative_kieu_str_sang_bool_va_khong_sugar() {
    // **VD-270 (path.is_relative)**: chữ ký `(str) -> bool` — phần bù của `path.is_abs`;
    // **không** sugar (`is_relative` không nằm trong `module_sugar`, tránh đoán module nhận).
    let f = sig_of("path.is_relative").expect("path.is_relative");
    assert_eq!(f.params, &[Ty::Str], "`path.is_relative`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`path.is_relative`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`path.is_relative`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_relative"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "is_relative"), None);
}

#[test]
fn path_relative_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.relative)**: chữ ký `(str, str) -> str`; **không** sugar — `relative` không
    // nằm trong `module_sugar`, nên `s.relative(to)` bị từ chối thay vì đoán module nhận.
    let f = sig_of("path.relative").expect("path.relative");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`path.relative`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.relative`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.relative`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "relative"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "relative"), None);
}

#[test]
fn path_with_extension_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.with_extension)**: chữ ký `(str, str) -> str`; **không** sugar — receiver
    // `str` không có trường `with_extension`, tránh đoán module nhận.
    let f = sig_of("path.with_extension").expect("path.with_extension");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`path.with_extension`: tham số"
    );
    assert_eq!(f.ret, Ty::Str, "`path.with_extension`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Str,
        "`path.with_extension`: hạ kiểu"
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "with_extension"),
        None
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("path", "with_extension"),
        None
    );
}

#[test]
fn path_with_stem_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.with_stem)**: chữ ký `(str, str) -> str`; **không** sugar — receiver `str`
    // không có trường `with_stem`, tránh đoán module nhận.
    let f = sig_of("path.with_stem").expect("path.with_stem");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`path.with_stem`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.with_stem`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.with_stem`: hạ kiểu");
    assert_eq!(vis_types::stdlib::module_sugar("str", "with_stem"), None);
    assert_eq!(vis_types::stdlib::module_sugar("path", "with_stem"), None);
}

#[test]
fn path_strip_prefix_hai_tham_so_optstr_va_khong_sugar() {
    // **VD-270 (path.strip_prefix)**: chữ ký `(str, str) -> str?` dùng `Ty::OptStr`; **không**
    // sugar **path** — `module_sugar` không ánh xạ `strip_prefix` sang `path`. Lưu ý
    // `str.strip_prefix` đã có từ trước nên receiver `str` vẫn đi `str.*` (không phải path).
    let f = sig_of("path.strip_prefix").expect("path.strip_prefix");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`path.strip_prefix`: tham số"
    );
    assert_eq!(f.ret, Ty::OptStr, "`path.strip_prefix`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Str)),
        "`path.strip_prefix`: hạ kiểu `str?`"
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "strip_prefix"),
        None,
        "`strip_prefix` không được ánh xạ sang `path`"
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("path", "strip_prefix"),
        None
    );
    // `str.strip_prefix` vẫn là hàm `str` độc lập (sugar receiver `str` không đổi).
    assert!(sig_of("str.strip_prefix").is_some());
}

#[test]
fn path_with_file_name_hai_tham_so_va_khong_sugar() {
    // **VD-270 (path.with_file_name)**: chữ ký `(str, str) -> str`; **không** sugar — receiver
    // `str` không có trường `with_file_name`, tránh đoán module nhận.
    let f = sig_of("path.with_file_name").expect("path.with_file_name");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`path.with_file_name`: tham số"
    );
    assert_eq!(f.ret, Ty::Str, "`path.with_file_name`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Str,
        "`path.with_file_name`: hạ kiểu"
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("str", "with_file_name"),
        None
    );
    assert_eq!(
        vis_types::stdlib::module_sugar("path", "with_file_name"),
        None
    );
}

#[test]
fn path_join_all_list_str_sang_str_va_khong_sugar() {
    // **VD-270 (path.join_all)**: chữ ký `(list[str]) -> str`; **không** sugar — `join_all`
    // không nằm trong `module_sugar`, nên `s.join_all(...)`/`xs.join_all()` bị từ chối thay vì
    // đoán module nhận (receiver `str`/`list` đều nhập nhằng với `path`).
    let f = sig_of("path.join_all").expect("path.join_all");
    assert_eq!(f.params, &[Ty::ListStr], "`path.join_all`: tham số");
    assert_eq!(f.ret, Ty::Str, "`path.join_all`: kiểu trả");
    assert_eq!(
        f.params[0].sang_type(),
        Type::List(Box::new(Type::Str)),
        "`path.join_all`: hạ kiểu tham số"
    );
    assert_eq!(f.ret.sang_type(), Type::Str, "`path.join_all`: hạ kiểu trả");
    assert_eq!(vis_types::stdlib::module_sugar("str", "join_all"), None);
    assert_eq!(vis_types::stdlib::module_sugar("list", "join_all"), None);
}

#[test]
fn parse_int_tra_int_option_va_sugar() {
    // **VD-270 (parse-int)**: `(str) -> int?`; tham số đầu `str` để sugar R5c nhận.
    let f = sig_of("str.parse_int").expect("str.parse_int");
    assert_eq!(f.params, &[Ty::Str], "`str.parse_int`: tham số");
    assert_eq!(f.ret, Ty::OptInt, "`str.parse_int`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Int)),
        "`str.parse_int`: hạ `int?`"
    );
}

#[test]
fn parse_float_tra_float_option_va_sugar() {
    // **VD-270 (parse-float)**: `(str) -> float?`; tham số đầu `str` để sugar R5c nhận.
    let f = sig_of("str.parse_float").expect("str.parse_float");
    assert_eq!(f.params, &[Ty::Str], "`str.parse_float`: tham số");
    assert_eq!(f.ret, Ty::OptFloat, "`str.parse_float`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Float)),
        "`str.parse_float`: hạ `float?`"
    );
    assert_eq!(f.ret.ten(), "float?", "`str.parse_float`: tên hiển thị");
}

#[test]
fn strlen_kieu_int_va_bool() {
    // **VD-270 (strlen)**: `byte_len`/`char_count` trả `int`; `str.is_empty`/`list.is_empty`/
    // `map.is_empty` trả `bool`; tham số đầu đúng kiểu để R5c nhận sugar `s.byte_len()`, v.v.
    for ten in ["str.byte_len", "str.char_count"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::Int, "`{ten}`: kiểu trả");
        assert_eq!(f.ret.sang_type(), Type::Int, "`{ten}`: hạ kiểu");
    }
    let s = sig_of("str.is_empty").expect("str.is_empty");
    assert_eq!(s.params, &[Ty::Str]);
    assert_eq!(s.ret, Ty::Bool);
    assert_eq!(s.ret.sang_type(), Type::Bool);
    let l = sig_of("list.is_empty").expect("list.is_empty");
    assert_eq!(l.params, &[Ty::List(&Ty::Var(0))]);
    assert_eq!(l.ret, Ty::Bool);
    let m = sig_of("map.is_empty").expect("map.is_empty");
    assert_eq!(m.params, &[Ty::Map(&Ty::Var(1), &Ty::Var(2))]);
    assert_eq!(m.ret, Ty::Bool);
}

#[test]
fn map_contains_key_kieu_map_k_bool() {
    // **VD-270 (map-contains-key)**: `(map[K, V], K) -> bool`; `K` buộc từ map nên sai kiểu
    // khoá ⇒ `VS0412`; tham số đầu `map` để sugar R5c `m.contains_key(k)` nhận.
    let f = sig_of("map.contains_key").expect("map.contains_key");
    assert_eq!(
        f.params,
        &[Ty::Map(&Ty::Var(1), &Ty::Var(2)), Ty::Var(1)],
        "`map.contains_key`: tham số"
    );
    assert_eq!(f.ret, Ty::Bool, "`map.contains_key`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`map.contains_key`: hạ kiểu");
    assert_eq!(f.so_doi(), 2, "`map.contains_key`: arity");
    assert_eq!(vis_types::stdlib::module_sugar("map", "contains_key"), None);
}

#[test]
fn str_is_ascii_kieu_str_bool() {
    // **VD-270 (str-is-ascii)**: `(str) -> bool`; tham số đầu `str` để R5c nhận sugar
    // `s.is_ascii()` (không nằm trong bảng `module_sugar`, chỉ dùng đúng module `str`).
    let f = sig_of("str.is_ascii").expect("str.is_ascii");
    assert_eq!(f.params, &[Ty::Str], "`str.is_ascii`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`str.is_ascii`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`str.is_ascii`: hạ kiểu");
    assert_eq!(f.so_doi(), 1, "`str.is_ascii`: arity");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_ascii"), None);
}

#[test]
fn str_is_blank_kieu_str_bool() {
    // **VD-270 (is-blank)**: `(str) -> bool`; tham số đầu `str` để R5c nhận sugar
    // `s.is_blank()` (không nằm trong bảng `module_sugar`, chỉ dùng đúng module `str`).
    let f = sig_of("str.is_blank").expect("str.is_blank");
    assert_eq!(f.params, &[Ty::Str], "`str.is_blank`: tham số");
    assert_eq!(f.ret, Ty::Bool, "`str.is_blank`: kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "`str.is_blank`: hạ kiểu");
    assert_eq!(f.so_doi(), 1, "`str.is_blank`: arity");
    assert_eq!(vis_types::stdlib::module_sugar("str", "is_blank"), None);
}

#[test]
fn str_is_ascii_preds_kieu_str_bool() {
    // **VD-270 (ascii-preds)**: bốn hàm `(str) -> bool`; tham số đầu `str` để R5c nhận sugar
    // `s.is_ascii_*()` (không nằm trong bảng `module_sugar`, chỉ dùng đúng module `str`).
    for ten in [
        "str.is_ascii_digit",
        "str.is_ascii_alphabetic",
        "str.is_ascii_alphanumeric",
        "str.is_ascii_whitespace",
    ] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu `{ten}`"));
        assert_eq!(f.params, &[Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::Bool, "`{ten}`: kiểu trả");
        assert_eq!(f.ret.sang_type(), Type::Bool, "`{ten}`: hạ kiểu");
        assert_eq!(f.so_doi(), 1, "`{ten}`: arity");
        let (m, n) = ten.split_once('.').expect("tên có dấu chấm");
        assert_eq!(
            vis_types::stdlib::module_sugar(m, n),
            None,
            "`{ten}`: sugar"
        );
    }
}

#[test]
fn panic_la_builtin_tra_never() {
    // **VD-267**: `panic` là builtin **trần** (không `std.`) với kiểu trả `never`; chữ ký nằm ở
    // `BUILTIN_FUNCS`, không trộn vào `FUNCS` (freeze API `std.*`).
    assert!(sig_of("panic").is_none(), "`panic` không phải hàm `std.*`");
    let f = sig_builtin("panic").expect("phải có chữ ký builtin `panic`");
    assert_eq!(f.params, &[Ty::Str]);
    assert_eq!(f.ret, Ty::Never);
    assert_eq!(f.ret.sang_type(), Type::Never);
    assert_eq!(f.ret.ten(), "never");
    assert_eq!(f.so_doi(), 1);
}

#[test]
fn strfind_tra_int_option_va_int() {
    // **VD-270 (strfind)**: `find`/`rfind` trả `int?` = `Option[int]` **thật**, `count` trả `int`;
    // cùng tham số `(str, str)` nên sugar `s.find(...)`/`s.count(...)` thoả R5c.
    for ten in ["str.find", "str.rfind"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::Str, Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::OptInt, "`{ten}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Int)),
            "`{ten}`: phải hạ thành `Option[int]`"
        );
        assert_eq!(f.ret.ten(), "int?", "`{ten}`: tên hiển thị");
    }
    let c = sig_of("str.count").expect("str.count");
    assert_eq!(c.params, &[Ty::Str, Ty::Str]);
    assert_eq!(c.ret, Ty::Int);
    assert_eq!(c.ret.sang_type(), Type::Int);
}

#[test]
fn strstrip_tra_str_option() {
    // **VD-270 (strstrip)**: `strip_prefix`/`strip_suffix` trả `str?` = `Option[str]` **thật**
    // (`Ty::OptStr`), tham số `(str, str)` nên sugar `s.strip_prefix(…)` thoả R5c.
    for ten in ["str.strip_prefix", "str.strip_suffix"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::Str, Ty::Str], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::OptStr, "`{ten}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Str)),
            "`{ten}`: phải hạ thành `Option[str]`"
        );
        assert_eq!(f.ret.ten(), "str?", "`{ten}`: tên hiển thị");
        assert_eq!(f.so_doi(), 2, "`{ten}`: số đối số");
    }
}

#[test]
fn replace_first_kieu_str_sang_str() {
    // **VD-270 (replace-first)**: `(str, str, str) -> str`; cùng tham số đầu `str` nên sugar
    // R5c `s.replace_first(…)` chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.replace_first").expect("str.replace_first");
    assert_eq!(f.params, &[Ty::Str, Ty::Str, Ty::Str], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 3, "số đối số");
}

#[test]
fn replace_last_kieu_str_sang_str() {
    // **VD-270 (replace-last)**: `(str, str, str) -> str`; cùng tham số đầu `str` nên sugar
    // R5c `s.replace_last(…)` chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.replace_last").expect("str.replace_last");
    assert_eq!(f.params, &[Ty::Str, Ty::Str, Ty::Str], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 3, "số đối số");
}

#[test]
fn replace_n_kieu_str_sang_str() {
    // **VD-270 (replace-n)**: `(str, str, str, int) -> str`; cùng tham số đầu `str` nên sugar
    // R5c `s.replace_n(…)` chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.replace_n").expect("str.replace_n");
    assert_eq!(f.params, &[Ty::Str, Ty::Str, Ty::Str, Ty::Int], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 4, "số đối số");
}

#[test]
fn char_at_kieu_str_int_sang_str_option() {
    // **VD-270 (char-at)**: `(str, int) -> str?`; cùng tham số đầu `str` nên sugar
    // R5c `s.char_at(i)` chạy không cần `use`, và kết quả là `Option[str]` thật.
    let f = sig_of("str.char_at").expect("str.char_at");
    assert_eq!(f.params, &[Ty::Str, Ty::Int], "tham số");
    assert_eq!(f.ret, Ty::OptStr, "kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Str)),
        "hạ kiểu trả"
    );
    assert_eq!(f.so_doi(), 2, "số đối số");
}

#[test]
fn title_kieu_str_sang_str() {
    // **VD-270 (str-title)**: `(str) -> str`; cùng tham số đầu `str` nên sugar R5c `s.title()`
    // chạy không cần `use`, và kết quả là `str` (không phải `str?`).
    let f = sig_of("str.title").expect("str.title");
    assert_eq!(f.params, &[Ty::Str], "tham số");
    assert_eq!(f.ret, Ty::Str, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Str, "hạ kiểu trả");
    assert_eq!(f.so_doi(), 1, "số đối số");
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_types::stdlib::module_sugar("str", "title"), None);
}

#[test]
fn indexof_tra_int_option() {
    // **VD-270 (indexof)**: `index_of`/`last_index_of` là `(list[T], T) -> int?`; `T` buộc từ
    // `xs` nên sugar `xs.index_of(v)` thoả R5c và `value` cùng kiểu phần tử.
    // **VD-270 (binary-search)**: `binary_search` dùng **đúng** hình dạng đó (biên dưới).
    for ten in ["list.index_of", "list.last_index_of", "list.binary_search"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(
            f.params,
            &[Ty::List(&Ty::Var(0)), Ty::Var(0)],
            "`{ten}`: tham số"
        );
        assert_eq!(f.ret, Ty::OptInt, "`{ten}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Int)),
            "`{ten}`: phải hạ thành `Option[int]`"
        );
        assert_eq!(f.ret.ten(), "int?", "`{ten}`: tên hiển thị");
    }
}

#[test]
fn list_contains_tra_bool_va_sugar() {
    // **VD-270 (list-contains)**: `(list[T], T) -> bool`; `T` buộc từ `xs` nên `value` cùng
    // kiểu phần tử và sugar `xs.contains(v)` thoả R5c; khác `index_of` ở kiểu trả `bool`.
    let f = sig_of("list.contains").expect("list.contains");
    assert_eq!(f.params, &[Ty::List(&Ty::Var(0)), Ty::Var(0)], "tham số");
    assert_eq!(f.ret, Ty::Bool, "kiểu trả");
    assert_eq!(f.ret.sang_type(), Type::Bool, "hạ kiểu trả");
    assert_eq!(f.ret.ten(), "bool", "tên hiển thị");
    assert_eq!(f.so_doi(), 2, "số đối số");
}

#[test]
fn list_sum_product_chi_list_int_tra_int() {
    // **VD-270 (list-sum-product)**: cả hai **chỉ** `list[int]` (không `list[T]`), trả `int`;
    // sugar `xs.product()` đi R5c; `xs.sum()` vẫn là phương thức lõi (cùng ngữ nghĩa cho
    // `list[int]`, còn `list[float]` do lõi xử lý).
    for ten in ["list.sum", "list.product"] {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, &[Ty::List(&Ty::Int)], "`{ten}`: tham số");
        assert_eq!(f.ret, Ty::Int, "`{ten}`: kiểu trả");
        assert_eq!(f.ret.sang_type(), Type::Int, "`{ten}`: phải hạ thành `int`");
        assert_eq!(f.ret.ten(), "int", "`{ten}`: tên hiển thị");
        assert_eq!(f.so_doi(), 1, "`{ten}`: số đối số");
    }
}

#[test]
fn list_mean_chi_list_float_tra_opt_float() {
    // **VD-270 (list-mean)**: **chỉ** `list[float]` (không `list[T]`, không `list[int]`), trả
    // `float?` = `Option[float]`; `mean` không phải phương thức lõi nên sugar `xs.mean()` R5c.
    let f = sig_of("list.mean").expect("list.mean");
    assert_eq!(f.params, &[Ty::List(&Ty::Float)], "`list.mean`: tham số");
    assert_eq!(f.ret, Ty::OptFloat, "`list.mean`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Float)),
        "`list.mean`: phải hạ thành `Option[float]`"
    );
    assert_eq!(f.ret.ten(), "float?", "`list.mean`: tên hiển thị");
    assert_eq!(f.so_doi(), 1, "`list.mean`: số đối số");
}

#[test]
fn list_median_chi_list_float_tra_opt_float() {
    // **VD-270 (list-median)**: **chỉ** `list[float]` (không `list[T]`, không `list[int]`), trả
    // `float?` = `Option[float]`; `median` không phải phương thức lõi nên sugar `xs.median()` R5c.
    let f = sig_of("list.median").expect("list.median");
    assert_eq!(f.params, &[Ty::List(&Ty::Float)], "`list.median`: tham số");
    assert_eq!(f.ret, Ty::OptFloat, "`list.median`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Opt(Box::new(Type::Float)),
        "`list.median`: phải hạ thành `Option[float]`"
    );
    assert_eq!(f.ret.ten(), "float?", "`list.median`: tên hiển thị");
    assert_eq!(f.so_doi(), 1, "`list.median`: số đối số");
}

#[test]
fn list_sum_float_chi_list_float_tra_float() {
    // **VD-270 (list-sum-float)**: **chỉ** `list[float]`, trả `float` trần (không `Option`);
    // `sum_float` không phải phương thức lõi nên sugar `xs.sum_float()` R5c.
    let f = sig_of("list.sum_float").expect("list.sum_float");
    assert_eq!(
        f.params,
        &[Ty::List(&Ty::Float)],
        "`list.sum_float`: tham số"
    );
    assert_eq!(f.ret, Ty::Float, "`list.sum_float`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::Float,
        "`list.sum_float`: phải hạ thành `float`"
    );
    assert_eq!(f.ret.ten(), "float", "`list.sum_float`: tên hiển thị");
    assert_eq!(f.so_doi(), 1, "`list.sum_float`: số đối số");
}

#[test]
fn list_minmax_float_chi_list_float_tra_opt_float() {
    // **VD-270 (list-minmax-float)**: **chỉ** `list[float]` (không `list[T]`, không
    // `list[int]`), trả `float?` = `Option[float]`; `min_float`/`max_float` không phải phương
    // thức lõi nên sugar `xs.min_float()`/`xs.max_float()` R5c.
    for name in ["list.min_float", "list.max_float"] {
        let f = sig_of(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(f.params, &[Ty::List(&Ty::Float)], "`{name}`: tham số");
        assert_eq!(f.ret, Ty::OptFloat, "`{name}`: kiểu trả");
        assert_eq!(
            f.ret.sang_type(),
            Type::Opt(Box::new(Type::Float)),
            "`{name}`: phải hạ thành `Option[float]`"
        );
        assert_eq!(f.ret.ten(), "float?", "`{name}`: tên hiển thị");
        assert_eq!(f.so_doi(), 1, "`{name}`: số đối số");
    }
}

#[test]
fn sang_type_dung_ten_bien_kieu() {
    assert_eq!(
        Ty::Var(0).sang_type(),
        Type::TypeRef {
            name: "T".into(),
            args: Vec::new()
        }
    );
    assert_eq!(
        Ty::List(&Ty::Var(1)).sang_type(),
        Type::List(Box::new(Type::TypeRef {
            name: "K".into(),
            args: Vec::new()
        }))
    );
    // `list[list[T]]` — promotion trong `const FUNCS` đã chứng minh; đây khoá **hình dạng**.
    assert_eq!(
        Ty::List(&Ty::List(&Ty::Var(0))).sang_type(),
        Type::List(Box::new(Type::List(Box::new(Type::TypeRef {
            name: "T".into(),
            args: Vec::new()
        }))))
    );
    assert_eq!(
        Ty::Map(&Ty::Var(1), &Ty::Var(2)).sang_type(),
        Type::Map(
            Box::new(Type::TypeRef {
                name: "K".into(),
                args: Vec::new()
            }),
            Box::new(Type::TypeRef {
                name: "V".into(),
                args: Vec::new()
            })
        )
    );
}

/// 16/38 chữ ký đổi (VD-99 (4)): `std.list` 8 + `std.map` 5 + `std.json` 3.
#[test]
fn muoi_sau_chu_ky_generic_dung_hinh_dang() {
    let t = Ty::Var(0);
    let list_t = Ty::List(&Ty::Var(0));
    let k = Ty::Var(1);
    let v = Ty::Var(2);
    let map_kv = Ty::Map(&Ty::Var(1), &Ty::Var(2));
    let ca: [(&str, &[Ty], Ty); 16] = [
        ("list.sort", &[list_t], list_t),
        ("list.sort_by", &[list_t, Ty::FnKhoa], list_t),
        ("list.reverse", &[list_t], list_t),
        ("list.unique", &[list_t], list_t),
        ("list.concat", &[list_t, list_t], list_t),
        (
            "list.chunk",
            &[list_t, Ty::Int],
            Ty::List(&Ty::List(&Ty::Var(0))),
        ),
        ("list.take", &[list_t, Ty::Int], list_t),
        ("list.drop", &[list_t, Ty::Int], list_t),
        ("map.keys", &[map_kv], Ty::List(&Ty::Var(1))),
        ("map.values", &[map_kv], Ty::List(&Ty::Var(2))),
        ("map.merge", &[map_kv, map_kv], map_kv),
        ("map.get_or", &[map_kv, Ty::Str, v], v),
        ("map.remove", &[map_kv, Ty::Str], map_kv),
        ("json.encode", &[t], Ty::Str),
        ("json.encode_pretty", &[t], Ty::Str),
        ("json.decode", &[Ty::Str], Ty::ResultJsonErr),
    ];
    for (ten, params, ret) in ca {
        let f = sig_of(ten).unwrap_or_else(|| panic!("thiếu chữ ký `{ten}`"));
        assert_eq!(f.params, params, "`{ten}`: tham số sai hình dạng");
        assert_eq!(f.ret, ret, "`{ten}`: kiểu trả sai hình dạng");
    }
    let _ = (k,);
}

/// `list.sort_by` **giữ** `Ty::FnKhoa` (ngoại lệ **có ghi** duy nhất, VD-94 (3)/VD-99 (5)):
/// lambda chưa chú thích là `{error}`; `Type` không có union `int|str` (ST-56) ⇒ tầng chạy kiểm.
#[test]
fn sort_by_giu_ngoai_le_fn_khoa() {
    let f = sig_of("list.sort_by").expect("list.sort_by");
    assert!(matches!(f.params[1], Ty::FnKhoa), "phải giữ `FnKhoa`");
    assert!(
        matches!(f.params[0], Ty::List(_)),
        "tham số 1 phải là `list[T]`"
    );
}

/// **VD-270 (sqlite-indexes)**: chữ ký `(str, str) -> Result[list[str], Error]` — tái dùng
/// `Ty::ResultListStrStr` (không thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_indexes_kieu_str_str_va_result_list_str() {
    let f = sig_of("sqlite.indexes").expect("sqlite.indexes");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.indexes`: tham số (handle, table)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListStrStr,
        "`sqlite.indexes`: kiểu trả phải tái dùng `ResultListStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.indexes`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[list[str], Error]");
}

/// **VD-270 (sqlite-schema-version)**: chữ ký `(str) -> Result[int, Error]` — tái dùng **đúng**
/// `Ty::ResultIntErr` (như `sqlite.changes`/`sqlite.last_insert_rowid`), không thêm biến thể.
#[test]
fn sqlite_schema_version_kieu_str_va_result_int() {
    let f = sig_of("sqlite.schema_version").expect("sqlite.schema_version");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.schema_version`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultIntErr,
        "`sqlite.schema_version`: kiểu trả phải là `ResultIntErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Int), Box::new(Ty::Error.sang_type())),
        "`sqlite.schema_version`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[int, Error]");
}

/// **VD-270 (sqlite-user-version)**: chữ ký `(str) -> Result[int, Error]` — tái dùng **đúng**
/// `Ty::ResultIntErr` (như `sqlite.changes`/`sqlite.last_insert_rowid`/`sqlite.schema_version`),
/// không thêm biến thể.
#[test]
fn sqlite_user_version_kieu_str_va_result_int() {
    let f = sig_of("sqlite.user_version").expect("sqlite.user_version");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.user_version`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultIntErr,
        "`sqlite.user_version`: kiểu trả phải là `ResultIntErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Int), Box::new(Ty::Error.sang_type())),
        "`sqlite.user_version`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[int, Error]");
}

/// **VD-270 (sqlite-application-id)**: chữ ký `(str) -> Result[int, Error]` — tái dùng **đúng**
/// `Ty::ResultIntErr` (như `sqlite.changes`/`sqlite.last_insert_rowid`/`sqlite.schema_version`/
/// `sqlite.user_version`), không thêm biến thể.
#[test]
fn sqlite_application_id_kieu_str_va_result_int() {
    let f = sig_of("sqlite.application_id").expect("sqlite.application_id");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.application_id`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultIntErr,
        "`sqlite.application_id`: kiểu trả phải là `ResultIntErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Int), Box::new(Ty::Error.sang_type())),
        "`sqlite.application_id`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[int, Error]");
}

/// **VD-270 (sqlite-data-version)**: chữ ký `(str) -> Result[int, Error]` — tái dùng **đúng**
/// `Ty::ResultIntErr` (như `sqlite.changes`/`sqlite.last_insert_rowid`/`sqlite.schema_version`/
/// `sqlite.user_version`), không thêm biến thể.
#[test]
fn sqlite_data_version_kieu_str_va_result_int() {
    let f = sig_of("sqlite.data_version").expect("sqlite.data_version");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.data_version`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultIntErr,
        "`sqlite.data_version`: kiểu trả phải là `ResultIntErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Int), Box::new(Ty::Error.sang_type())),
        "`sqlite.data_version`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[int, Error]");
}

/// **VD-270 (sqlite-journal-mode)**: chữ ký `(str) -> Result[str, Error]` — biến thể **riêng**
/// `Ty::ResultStrErr` (theo mẫu `ResultIntErr`; `ResultStrStr` cũng hạ ra cùng kiểu nhưng tên cũ
/// thuộc thời `Result[str, str]`), hạ ra `Result[Str, Error]`.
#[test]
fn sqlite_journal_mode_kieu_str_va_result_str() {
    let f = sig_of("sqlite.journal_mode").expect("sqlite.journal_mode");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.journal_mode`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultStrErr,
        "`sqlite.journal_mode`: kiểu trả phải là biến thể `ResultStrErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Str), Box::new(Ty::Error.sang_type())),
        "`sqlite.journal_mode`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[str, Error]");
}

/// **VD-270 (sqlite-database-list)**: chữ ký `(str) -> Result[list[RowSet], Error]` — biến thể
/// **riêng** `Ty::ListRowSet` (`list[RowSet]`) + `Ty::ResultListRowSetErr`, theo mẫu
/// `RowSet`/`ResultRowSetErr`; hạ ra `Result[list[Record(RecId(1))], Error]`.
#[test]
fn sqlite_database_list_kieu_str_va_result_list_rowset() {
    let f = sig_of("sqlite.database_list").expect("sqlite.database_list");
    assert_eq!(
        f.params,
        &[Ty::Str],
        "`sqlite.database_list`: tham số (handle)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListRowSetErr,
        "`sqlite.database_list`: kiểu trả phải là `ResultListRowSetErr`"
    );
    assert_eq!(
        Ty::ListRowSet.sang_type(),
        Type::List(Box::new(Ty::RowSet.sang_type())),
        "`list[RowSet]` hạ ra list của bản ghi `RowSet`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Record(vis_types::ty::RecId(1))))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.database_list`: hạ kiểu trả"
    );
    assert_eq!(Ty::ListRowSet.ten(), "list[RowSet]");
    assert_eq!(f.ret.ten(), "Result[list[RowSet], Error]");
}

/// **VD-270 (sqlite-schema-sql)**: chữ ký `(str, str) -> Result[str?, Error]` — biến thể **riêng**
/// `Ty::ResultOptStrErr` (không tái dùng `ResultStrStr`/`OptStr`, vì phải giữ **cả** lớp
/// `Result` **và** lớp `Option`), hạ ra `Result[Option[str], Error]`.
#[test]
fn sqlite_schema_sql_kieu_str_str_va_result_opt_str() {
    let f = sig_of("sqlite.schema_sql").expect("sqlite.schema_sql");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.schema_sql`: tham số (handle, name)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultOptStrErr,
        "`sqlite.schema_sql`: kiểu trả phải là biến thể riêng `ResultOptStrErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::Opt(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.schema_sql`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[str?, Error]");
}

/// **VD-270 (sqlite-index-columns)**: chữ ký `(str, str) -> Result[list[str], Error]` — tái
/// dùng `Ty::ResultListStrStr` (không thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi
/// `Error`.
#[test]
fn sqlite_index_columns_kieu_str_str_va_result_list_str() {
    let f = sig_of("sqlite.index_columns").expect("sqlite.index_columns");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.index_columns`: tham số (handle, index)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListStrStr,
        "`sqlite.index_columns`: kiểu trả phải tái dùng `ResultListStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.index_columns`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[list[str], Error]");
}

/// **VD-270 (sqlite-primary-keys)**: chữ ký `(str, str) -> Result[list[str], Error]` — tái dùng
/// `Ty::ResultListStrStr` (không thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_primary_keys_kieu_str_str_va_result_list_str() {
    let f = sig_of("sqlite.primary_keys").expect("sqlite.primary_keys");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.primary_keys`: tham số (handle, table)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultListStrStr,
        "`sqlite.primary_keys`: kiểu trả phải tái dùng `ResultListStrStr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(
            Box::new(Type::List(Box::new(Type::Str))),
            Box::new(Ty::Error.sang_type())
        ),
        "`sqlite.primary_keys`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[list[str], Error]");
}

/// **VD-270 (sqlite-table-exists)**: chữ ký `(str, str) -> Result[bool, Error]` — tái dùng
/// `Ty::ResultBoolErr` (cùng biến thể của `sqlite.in_transaction`, không thêm biến thể `Ty`),
/// hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_table_exists_kieu_str_str_va_result_bool() {
    let f = sig_of("sqlite.table_exists").expect("sqlite.table_exists");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.table_exists`: tham số (handle, name)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultBoolErr,
        "`sqlite.table_exists`: kiểu trả phải tái dùng `ResultBoolErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Bool), Box::new(Ty::Error.sang_type())),
        "`sqlite.table_exists`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[bool, Error]");
}

/// **VD-270 (sqlite-index-exists)**: chữ ký `(str, str) -> Result[bool, Error]` — tái dùng
/// `Ty::ResultBoolErr` (cùng biến thể của `sqlite.in_transaction`/`sqlite.table_exists`, không
/// thêm biến thể `Ty`), hạ ra `Result` với lỗi là bản ghi `Error`.
#[test]
fn sqlite_index_exists_kieu_str_str_va_result_bool() {
    let f = sig_of("sqlite.index_exists").expect("sqlite.index_exists");
    assert_eq!(
        f.params,
        &[Ty::Str, Ty::Str],
        "`sqlite.index_exists`: tham số (handle, name)"
    );
    assert_eq!(
        f.ret,
        Ty::ResultBoolErr,
        "`sqlite.index_exists`: kiểu trả phải tái dùng `ResultBoolErr`"
    );
    assert_eq!(
        f.ret.sang_type(),
        Type::Result(Box::new(Type::Bool), Box::new(Ty::Error.sang_type())),
        "`sqlite.index_exists`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "Result[bool, Error]");
}
