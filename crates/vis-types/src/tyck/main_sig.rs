// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-321a (`#200`/`#201`)** — kiểm **tĩnh chữ ký `main`**, một nguồn cho cả interp lẫn VM.
//!
//! SPEC §9.2 chỉ định nghĩa nghĩa `argv` cho `main(args: list[str])`. Chương trình có chữ ký
//! khác (`main(a: int, b: int)`, `main(a: int)`, …) trước đây lọt qua `vis check`: interp trap
//! runtime (`VS0501`, exit 101) trong khi VM điền `()` và exit 0, hoặc cả hai cùng nhận
//! `list[str]` vào tham số khai `int`. Luật ở đây chặn **trước khi chạy** (`exit 2`, TAP đỏ ở
//! `vis check`) bằng đúng mã đã có `VS0412`/`VS0401` — không mã mới, không đụng engine/bytecode.
//!
//! Luật (chỉ áp cho hàm entry `main` của tệp chính, `span.file == 0`):
//!
//! - tham số `cap[...]` được **bỏ qua** (quyền do loader/host cấp, không phải `argv`);
//! - **không có `cap`**: tham số non-`cap` đầu tiên nhận `argv` ⇒ phải `list[str]`;
//!   mọi tham số non-`cap` sau phải có `default` (không nhận `argv`);
//! - **có `cap`**: tối đa **một** tham số non-`cap`, và nó phải `list[str]`.
//!
//! Dùng chữ ký đã resolve trong [`Checker::sigs`] — **không** gọi lại `resolve_ann` (tránh báo
//! trùng `VS0301`) và chạy sau `const_cycle::kiem` trong [`super::check`].

use vis_hir::hir::{FnKind, HItem, Hir};
use vis_hir::ty::Type;

use super::{Checker, ARG_COUNT, MISMATCH};

/// Thêm chẩn đoán chữ ký `main` vào `c`, theo **thứ tự nguồn** của tham số.
pub(super) fn kiem(hir: &Hir, c: &mut Checker) {
    // Chỉ tệp chính (`file = 0`, ST-114a) — không đụng `fn main` trong module nạp sau.
    let Some(f) = hir.items.iter().find_map(|it| match it {
        HItem::Fn(f) if f.name == "main" && f.kind == FnKind::Fn && f.span.file == 0 => Some(f),
        _ => None,
    }) else {
        return;
    };
    // Chữ ký đã resolve nằm trong `khai_bao_item`; nếu không có (HIR lạ) thì im lặng, không
    // đoán kiểu (nguyên tắc `Type::Error` lan truyền im lặng của tyck).
    let Some(sig) = c.sig_of(f.sym) else {
        return;
    };
    if sig.params.len() != f.params.len() {
        return;
    }
    // Chụp lại kiểu + cờ default (chữ ký `main` rất nhỏ) để không giữ borrow của `c` khi ghi
    // chẩn đoán qua `c.diag`.
    let params: Vec<(Type, bool)> = sig
        .params
        .iter()
        .map(|(_, t, has_default, _)| (t.clone(), *has_default))
        .collect();

    // `cap[Dir]`/… được `resolve_cap` hạ thành `TypeRef { name: "cap", .. }`.
    let la_cap = |i: usize| matches!(&params[i].0, Type::TypeRef { name, .. } if name == "cap");
    let non_cap: Vec<usize> = (0..f.params.len()).filter(|&i| !la_cap(i)).collect();
    if non_cap.is_empty() {
        return;
    }

    let co_cap = non_cap.len() != f.params.len();
    let dau = non_cap[0];
    let la_list_str = matches!(&params[dau].0, Type::List(t) if matches!(**t, Type::Str));
    if !la_list_str {
        c.diag(
            MISMATCH,
            format!(
                "`main`: tham số `{}` nhận `argv` phải là `list[str]`",
                f.params[dau].name
            ),
            f.params[dau].span,
        );
    }

    for &i in &non_cap[1..] {
        let p = &f.params[i];
        // Không có cap: các tham số sau không nhận argv nhưng phải có giá trị mặc định để VM
        // và interp cùng điền được (ca dương `main(a: list[str], b: int = K + 2)`).
        // Có cap: mọi tham số non-cap đều được engine gán argv ⇒ chỉ được đúng một.
        if co_cap || !params[i].1 {
            let msg = if co_cap {
                format!(
                    "`main` chỉ nhận một tham số `list[str]` ngoài `cap`; tham số `{}` thừa",
                    p.name
                )
            } else {
                format!(
                    "`main` chỉ nhận một tham số `list[str]` (argv); tham số `{}` thiếu giá trị mặc định",
                    p.name
                )
            };
            c.diag(ARG_COUNT, msg, p.span);
        }
    }
}
