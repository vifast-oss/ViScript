// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270 (matches)** — khoá chữ ký `std.str.matches` trong bảng `FUNCS`.
//!
//! Tách khỏi `stdlib_ty.rs` để tệp đó giữ dưới luật R1 (1500 dòng).

use vis_hir::stdlib::{sig_of, Ty};
use vis_hir::Type;

#[test]
fn strmatches_kieu_str_str_list_str_va_sugar() {
    // **VD-270 (matches)**: chữ ký `(str, str) -> list[str]`; tham số đầu `str` để sugar R5c
    // `s.matches(pat)` nhận. Tái dùng **đúng** `Ty::ListStr` của `str.split`/`str.rsplit`,
    // không thêm biến thể `Ty` và không có lớp `Option`.
    let f = sig_of("str.matches").expect("str.matches");
    assert_eq!(f.params, &[Ty::Str, Ty::Str], "`str.matches`: tham số");
    assert_eq!(f.ret, Ty::ListStr, "`str.matches`: kiểu trả");
    assert_eq!(
        f.ret.sang_type(),
        Type::List(Box::new(Type::Str)),
        "`str.matches`: hạ kiểu trả"
    );
    assert_eq!(f.ret.ten(), "list[str]", "`str.matches`: tên hiển thị");
    assert_eq!(f.so_doi(), 2, "`str.matches`: số đối số");
    assert_eq!(f.ret, sig_of("str.split").unwrap().ret);
    // Không cần bảng `module_sugar`: module mặc định của người nhận `str` đã là `str`.
    assert_eq!(vis_hir::stdlib::module_sugar("str", "matches"), None);
}
