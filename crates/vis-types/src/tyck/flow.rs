// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-259** — ngữ cảnh **luồng điều khiển tĩnh** của checker (tách khỏi `mod.rs` để giữ R1):
//! câu lệnh cuối có cho giá trị không, `break`/`continue` chỉ hợp lệ trong vòng lặp **cùng hàm**
//! và ở **vị trí câu lệnh**.

use vis_syntax::Span;

use crate::hir::{HCond, HExpr, HStmt};
use crate::ty::Type;

use super::{vd238, Checker, MISMATCH, RANGE_KHONG_INT};

/// **VD-260** — ảnh chụp phạm vi `env`/`mutable` của checker. Binding của `let`/`if let`/
/// `while let`/`for`/nhánh `match` chỉ sống trong khối của nó; ra khỏi khối phải trả lại môi
/// trường trước đó (nếu không, một ký hiệu đã hết phạm vi vẫn còn kiểu trong `env`).
pub(crate) struct EnvSnap {
    env: Vec<Option<Type>>,
    mutable: Vec<bool>,
}

impl<'a> Checker<'a> {
    /// Lưu môi trường hiện tại để khôi phục khi ra khỏi khối.
    pub(crate) fn luu_env(&self) -> EnvSnap {
        EnvSnap {
            env: self.env.clone(),
            mutable: self.mutable.clone(),
        }
    }

    /// Khôi phục môi trường đã lưu — binding trong khối vừa kiểm không còn hiệu lực.
    pub(crate) fn tra_env(&mut self, snap: EnvSnap) {
        self.env = snap.env;
        self.mutable = snap.mutable;
    }

    /// Kiểu phần tử khi `for` duyệt `iter` (tách khỏi `mod.rs` để giữ R1).
    pub(crate) fn elem_of_iter(&mut self, it: &Type, span: Span) -> Type {
        match it {
            Type::List(t) => (**t).clone(),
            Type::Map(k, v) => Type::TypeRef {
                name: "tuple".into(),
                args: vec![(**k).clone(), (**v).clone()],
            },
            Type::Str => Type::Char,
            Type::TypeRef { name, args } if name == "range" => {
                // ST-13b (mục 4): VS0 chỉ duyệt range **`int`**; `size`/`duration`/`char` ⇒ VS0412
                // (tyck và runtime phải khớp — runtime chưa hỗ trợ range đơn vị).
                match args.first() {
                    Some(Type::Int) | None => Type::Int,
                    Some(Type::Error) => Type::Error,
                    Some(other) => {
                        let o = other.clone();
                        self.diag(
                            RANGE_KHONG_INT,
                            format!(
                                "`for` chỉ duyệt được range `int`, được `{}` (§A.7)",
                                self.show(&o)
                            ),
                            span,
                        );
                        Type::Error
                    }
                }
            }
            Type::Error => Type::Error,
            _ => Type::Error,
        }
    }
}

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
        // **VD-260**: binding trong thân khối (`let`/`for`/nhánh) chỉ sống trong khối này.
        let snap = self.luu_env();
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
        self.tra_env(snap);
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
        // **VD-260**: binding của `if let` chỉ sống trong nhánh `then`/`elif` của nó.
        let snap = self.luu_env();
        let value = self.in_value;
        self.kiem_cond(cond, then);
        let mut acc = self.kiem_than_ngu_canh(then, value);
        for (c, body) in elifs {
            self.kiem_cond(c, body);
            let t = self.kiem_than_ngu_canh(body, value);
            self.unify_branch(&mut acc, &t, span);
        }
        let out = match else_ {
            Some(e) if !e.is_empty() => {
                let t = self.kiem_than_ngu_canh(e, value);
                self.unify_branch(&mut acc, &t, span);
                acc
            }
            _ => Type::Unit,
        };
        self.tra_env(snap);
        out
    }
}

/// **VD-263** — khối `else` của `let … else` có **thoát trên mọi nhánh** không?
///
/// Chỉ nhận các dạng phân kỳ **chứng minh được bằng cấu trúc** (không tin một lời gọi khai kiểu
/// `never`): câu cuối là `return`/`break`/`continue`; hoặc `if` có `else` mà **mọi** nhánh thoát;
/// hoặc `match` mà **mọi** nhánh thoát và có ít nhất một nhánh **không guard** (nếu mọi nhánh đều
/// có guard, một tổ hợp guard sai sẽ rơi xuống cuối); hoặc khối/`scope` lồng có câu cuối thoát.
/// `while`/`for`/`loop` không được coi là thoát (vòng có thể không chạy / `break` ẩn).
pub(crate) fn than_luon_thoat(body: &[HStmt]) -> bool {
    body.last().map(stmt_luon_thoat).unwrap_or(false)
}

fn stmt_luon_thoat(s: &HStmt) -> bool {
    match s {
        HStmt::Return { .. } | HStmt::Break { .. } | HStmt::Continue { .. } => true,
        HStmt::If {
            then,
            elifs,
            else_: Some(e),
            ..
        } if !e.is_empty() => {
            than_luon_thoat(then)
                && elifs.iter().all(|(_, b)| than_luon_thoat(b))
                && than_luon_thoat(e)
        }
        HStmt::Match { arms, .. } => match_luon_thoat(arms),
        HStmt::Scope { body, .. } | HStmt::Block { body, .. } => than_luon_thoat(body),
        HStmt::Expr(e) => expr_luon_thoat(e),
        _ => false,
    }
}

fn expr_luon_thoat(e: &HExpr) -> bool {
    match e {
        HExpr::If {
            then, elifs, else_, ..
        } if !else_.is_empty() => {
            than_luon_thoat(then)
                && elifs.iter().all(|(_, b)| than_luon_thoat(b))
                && than_luon_thoat(else_)
        }
        HExpr::Match { arms, .. } => match_luon_thoat(arms),
        _ => false,
    }
}

fn match_luon_thoat(arms: &[crate::hir::HArm]) -> bool {
    !arms.is_empty()
        && arms.iter().all(|a| than_luon_thoat(&a.body))
        && arms.iter().any(|a| a.guard.is_none())
}
