// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-259** — ngữ cảnh **luồng điều khiển tĩnh** của checker (tách khỏi `mod.rs` để giữ R1):
//! câu lệnh cuối có cho giá trị không, `break`/`continue` chỉ hợp lệ trong vòng lặp **cùng hàm**
//! và ở **vị trí câu lệnh**.

use vis_syntax::Span;

use crate::hir::{HCond, HStmt};
use crate::ty::Type;

use super::{vd238, Checker, MISMATCH};

impl<'a> Checker<'a> {
    /// Kiểm một **thân** (suite) và ép kiểu giá trị về `want`.
    pub(crate) fn kiem_than(&mut self, body: &[HStmt], want: &Type, span: Span) {
        let at = body.last().map(HStmt::span).unwrap_or(span);
        // **VD-238**: `fn f() -> int: None` — thân cuối là `None` trần ⇒ lỗi tĩnh khi lớp **ngoài cùng** của `want` không phải `Option`.
        let last_is_none =
            matches!(body.last(), Some(crate::hir::HStmt::Expr(e)) if vd238::la_none_bare(e));
        let t = self.than_value(body);
        // **VD-241 §7 (A0, 2026-11-19)**: `None` trả về hợp lệ khi `want` **ngoài cùng** là `Option`.
        if last_is_none && !vd238::la_none_dich_opt(want) {
            self.diag(
                MISMATCH,
                format!(
                    "`return`: `None` chỉ hợp lệ ở `T?`, được `{}` (VD-238)",
                    self.types.show(want)
                ),
                at,
            );
            return;
        }
        if !want.is_error() {
            self.coerce(&t, want, at, "giá trị trả về");
        }
    }

    /// Kiểm thân, ghi rõ **câu cuối có cho giá trị không** (`value`). `in_value` chỉ ảnh hưởng
    /// luật `break`/`continue` ở vị trí cần giá trị (VD-259); kiểu trả vẫn như cũ.
    pub(crate) fn kiem_than_ngu_canh(&mut self, body: &[HStmt], value: bool) -> Type {
        let n = body.len();
        let mut kq = Type::Unit;
        let saved = std::mem::replace(&mut self.in_value, false);
        for (i, s) in body.iter().enumerate() {
            self.in_value = value && i + 1 == n;
            let t = self.kiem_stmt(s);
            if i + 1 == n {
                // §4/§6: câu lệnh cuối của khối **cho giá trị** (`Expr`, `if`/`match` có `else`,
                // khối con); các dạng khác là `()` — VD-47 (sửa lỗ hổng ST-13).
                kq = t;
            }
        }
        self.in_value = saved;
        kq
    }

    /// **VD-259**: `break`/`continue` chỉ hợp lệ trong vòng lặp **cùng hàm** và ở **vị trí câu
    /// lệnh** (không cần giá trị). VD-241 §6/B-A: vẫn phân kỳ ⇒ `Never`.
    pub(crate) fn kiem_nhay(&mut self, la_break: bool, span: Span) -> Type {
        let tu = if la_break { "break" } else { "continue" };
        if self.loop_depth == 0 {
            self.diag(
                MISMATCH,
                format!(
                    "`{tu}` ngoài vòng lặp — chỉ dùng trong `for`/`while`/`loop` của **cùng hàm** (VD-259)"
                ),
                span,
            );
        } else if self.in_value {
            self.diag(
                MISMATCH,
                format!(
                    "`{tu}` ở vị trí cần giá trị — nhánh này không cho giá trị; đặt `{tu}` thành câu lệnh riêng (VD-259)"
                ),
                span,
            );
        }
        Type::Never
    }

    /// `if`/`elif`/`else` dạng **câu lệnh**. Nhánh chỉ ở vị trí cần giá trị khi chính `if`
    /// đang ở vị trí đó (VD-259); kiểu trả/unify giữ nguyên VD-47.
    pub(crate) fn kiem_if_stmt(
        &mut self,
        cond: &HCond,
        then: &[HStmt],
        elifs: &[(HCond, Vec<HStmt>)],
        else_: Option<&[HStmt]>,
        span: Span,
    ) -> Type {
        let value = self.in_value;
        self.kiem_cond(cond, then);
        let mut acc = self.kiem_than_ngu_canh(then, value);
        for (c, body) in elifs {
            self.kiem_cond(c, body);
            let t = self.kiem_than_ngu_canh(body, value);
            self.unify_branch(&mut acc, &t, span);
        }
        match else_ {
            Some(e) if !e.is_empty() => {
                let t = self.kiem_than_ngu_canh(e, value);
                self.unify_branch(&mut acc, &t, span);
                acc
            }
            _ => Type::Unit,
        }
    }
}
