// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — kiểm **`std.*`** và **tên dựng sẵn** (`kiem_std`/`kiem_builtin`, nguyên văn từ
//! `tyck/expr.rs`; 0 đổi hành vi/chẩn đoán).

use vis_hir::hir::HExpr;
use vis_hir::ty::Type;

use super::super::{thay_gen, Checker, ARG_COUNT, MISMATCH};
use super::phu::la_cap;

impl<'a> Checker<'a> {
    /// Chữ ký `std.*` (ST-26): `None` nếu `name` không phải `std.*`.
    ///
    /// **ST-58 (VD-94/VD-99)**: **hợp nhất** đối số thật với **biến kiểu** của chữ ký (trái→phải),
    /// rồi **thay** biến vào kiểu trả. `Ty::FnKhoa` là **ngoại lệ có ghi** duy nhất: lambda chưa chú
    /// thích là `{error}` và `Type` không có union `int|str` ⇒ tầng chạy kiểm (ST-56). Biến **không
    /// gán** ⇒ `{error}` (giữ `json.decode`, VD-66). Không mã mới: `VS0401` (số đối số) / `VS0412`.
    pub(super) fn kiem_std(
        &mut self,
        name: &str,
        args: &[HExpr],
        named: &[(String, HExpr)],
        span: vis_syntax::Span,
    ) -> Option<Type> {
        let ngan = name.strip_prefix("std.")?;
        let sig = vis_hir::stdlib::sig_of(ngan)?;
        // **ST-60b**: `std.*` **chỉ** có đối số **vị trí** — `--desc` (hay bất kỳ `--tên` nào) trước
        // đây bị **bỏ im lặng** (kết quả vẫn tăng dù xin giảm) ⇒ nay **từ chối** `VS0401`, nêu tên.
        if let Some((ten, a)) = named.first() {
            let sp = a.span();
            self.diag(
                ARG_COUNT,
                format!(
                    "`{ngan}` không nhận tham số **tên** `--{ten}` — `std.*` chỉ có đối số vị trí"
                ),
                sp,
            );
        }
        let tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
        if tys.len() != sig.so_doi() {
            self.diag(
                ARG_COUNT,
                format!("`{ngan}` cần {} đối số, được {}", sig.so_doi(), tys.len()),
                span,
            );
        }
        let gens: Vec<String> = (0..3)
            .map(|i| vis_hir::stdlib::ten_bien(i as u8).to_string())
            .collect();
        let mut bang: Vec<(String, Type)> = Vec::new();
        for (i, (that, mong)) in tys.iter().zip(sig.params.iter()).enumerate() {
            // Ngoại lệ **có ghi**: khoá `sort_by` (lambda chưa chú thích) — tầng chạy kiểm (ST-56).
            if matches!(mong, vis_hir::stdlib::Ty::FnKhoa) {
                continue;
            }
            self.hop_nhat(
                &mong.sang_type(),
                that,
                &gens,
                &mut bang,
                &format!("std.{ngan} (đối số {})", i + 1),
                span,
            );
        }
        Some(thay_gen(&sig.ret.sang_type(), &bang))
    }

    /// Chữ ký 8 tên dựng sẵn (VD-44 #3): đối số đúng số lượng + đúng kiểu.
    pub(super) fn kiem_builtin(
        &mut self,
        name: &str,
        args: &[HExpr],
        span: vis_syntax::Span,
    ) -> Option<Type> {
        // **ST-293 (VD-274/VD-280)**: `select_recv*`/`select_send*` — kiểu kênh + phần tử
        // suy từ `list[Chan[T]]`; logic ở `expr/kenh.rs` (R1), không đổi parser/EBNF.
        if matches!(
            name,
            "select_recv" | "select_recv_timeout" | "select_send" | "select_send_timeout"
        ) {
            return Some(self.kiem_select(name, args, span));
        }
        // ST-42: `ls` ⇒ `list[FileEntry]` (tra bản ghi **theo tên** trong bảng kiểu); `open` ⇒ `str`.
        if name == "ls" || name == "open" {
            let mut tys: Vec<Type> = Vec::new();
            for a in args {
                tys.push(self.kiem_expr(a));
            }
            if name == "open" {
                if tys.len() != 1 {
                    self.diag(
                        ARG_COUNT,
                        format!("`open` cần 1 đối số, được {}", tys.len()),
                        span,
                    );
                }
                return Some(Type::Str);
            }
            if tys.len() > 3 {
                self.diag(
                    ARG_COUNT,
                    format!("`ls` cần 0–3 đối số, được {}", tys.len()),
                    span,
                );
            }
            let el = match self.types.rec_by_name("FileEntry") {
                Some(id) => Type::Record(id),
                None => Type::Error,
            };
            return Some(Type::List(Box::new(el)));
        }
        // ST-40b: `get`/`select` — kiểu ra = `list[<kiểu thân lambda>]` (một lần kiểm thân).
        if name == "get" || name == "select" {
            let xs_ty = args
                .first()
                .map(|a| self.kiem_expr(a))
                .unwrap_or(Type::Error);
            let mut body_ty = Type::Error;
            if let Some(f) = args.get(1) {
                body_ty = match f {
                    HExpr::Lambda { body, .. } => match body {
                        vis_hir::hir::HBody::Expr(e) => self.kiem_expr(e),
                        vis_hir::hir::HBody::Stmts(_) => Type::Unit,
                    },
                    other => self.kiem_expr(other),
                };
            }
            if args.len() != 2 {
                self.diag(
                    ARG_COUNT,
                    format!("`{name}` cần 2 đối số, được {}", args.len()),
                    span,
                );
            }
            let _ = xs_ty;
            return Some(Type::List(Box::new(body_ty)));
        }
        // ST-39: `sort_by` (chế độ shell) — kiểm **khoá** **một lần** trên thân lambda (không kiểm lại
        // qua `tys`), từ chối khoá không so được (`bool`, `map`, `record`, `list`) ⇒ `VS0412`.
        // Runtime có chốt thứ hai (`VS0501`). **Chỉ** khi có ≥ 2 đối số: pipeline `.vis` cũ dùng
        // `sort_by` như **chương trình ngoài** 1 đối số phải giữ nguyên hành vi (không báo arity).
        if name == "sort_by" && args.len() >= 2 {
            let xs_ty = args
                .first()
                .map(|a| self.kiem_expr(a))
                .unwrap_or(Type::Error);
            if let Some(f) = args.get(1) {
                let kt = match f {
                    HExpr::Lambda { body, .. } => match body {
                        vis_hir::hir::HBody::Expr(e) => self.kiem_expr(e),
                        vis_hir::hir::HBody::Stmts(_) => Type::Unit,
                    },
                    other => self.kiem_expr(other),
                };
                let khong_so = matches!(&kt, Type::Bool | Type::Unit)
                    || matches!(&kt, Type::Map(..) | Type::Record(..) | Type::List(..));
                if khong_so {
                    self.diag(
                        MISMATCH,
                        format!("`sort_by`: khoá không so được: `{}`", self.show(&kt)),
                        span,
                    );
                }
            }
            if let Some(d) = args.get(2) {
                let dt = self.kiem_expr(d);
                if !matches!(dt, Type::Bool | Type::Error) {
                    self.diag(
                        MISMATCH,
                        format!("`sort_by`: `--desc` cần `bool`, được `{}`", self.show(&dt)),
                        span,
                    );
                }
            }
            if args.len() > 3 {
                self.diag(
                    ARG_COUNT,
                    format!("`sort_by` cần 2–3 đối số, được {}", args.len()),
                    span,
                );
            }
            return Some(match xs_ty {
                Type::List(e) => Type::List(e),
                other => Type::List(Box::new(other)),
            });
        }
        // ST-50: `where` — kiểm **thân vị từ** ra `bool` **kể cả** lambda trong ngoặc
        // (`where ( x => 1 )` ⇒ `VS0412`); `{error}` ⇒ kiểm động (không đỏ oan).
        if name == "where" && args.len() >= 2 {
            let xs_ty = args
                .first()
                .map(|a| self.kiem_expr(a))
                .unwrap_or(Type::Error);
            let el = match xs_ty {
                Type::List(e) => *e,
                Type::Error => Type::Error,
                _ => Type::Error,
            };
            let kt = match &args[1] {
                HExpr::Lambda { body, params, .. } => {
                    for p in params {
                        let t = self.resolve_ann(&p.ty, p.span);
                        self.set_ty(p.sym, t);
                    }
                    match body {
                        vis_hir::hir::HBody::Expr(e) => self.kiem_expr(e),
                        vis_hir::hir::HBody::Stmts(_) => Type::Unit,
                    }
                }
                f => self.kiem_expr(f),
            };
            if !matches!(kt, Type::Bool | Type::Error | Type::Fn(..)) {
                self.diag(
                    MISMATCH,
                    format!("`where`: thân vị từ phải `bool`, được `{}`", self.show(&kt)),
                    args[1].span(),
                );
            }
            if args.len() != 2 {
                self.diag(
                    ARG_COUNT,
                    format!("`where` cần 2 đối số, được {}", args.len()),
                    span,
                );
            }
            return Some(Type::List(Box::new(el)));
        }
        let tys: Vec<Type> = args.iter().map(|a| self.kiem_expr(a)).collect();
        let arity = |n: usize| args.len() == n;
        let sai_so = format!("`{name}` cần {} đối số, được {}", 0, args.len());
        let _ = sai_so;
        let t0 = || tys.first().cloned().unwrap_or(Type::Error);
        match name {
            "print" => {
                // **ST-72 (VD-106 (1))**: `cap` chỉ dùng làm tham số — không in được.
                // **R6d (VD-178)**: `print(a, b, …)` hợp lệ (cách nhau 1 dấu cách); `print()` ⇒ `\n`.
                for t in &tys {
                    if la_cap(t) {
                        self.diag(
                            MISMATCH,
                            format!("`cap` không in được (`{}`)", self.show(t)),
                            span,
                        );
                    }
                }
                Some(Type::Unit)
            }
            "str" => {
                if !arity(1) {
                    self.diag(
                        ARG_COUNT,
                        format!("`str` cần 1 đối số, được {}", args.len()),
                        span,
                    );
                }
                Some(Type::Str)
            }
            "assert" => {
                if !arity(1) {
                    self.diag(ARG_COUNT, "`assert` cần 1 đối số", span);
                } else if !matches!(t0(), Type::Bool | Type::Error) {
                    self.diag(
                        MISMATCH,
                        format!("`assert` cần `bool`, được `{}`", self.show(&t0())),
                        span,
                    );
                }
                Some(Type::Unit)
            }
            // ST-18: 3 lệnh builtin chế độ lệnh — chữ ký như `sig_builtin()` ở `lower/cmd.rs`.
            "echo" => {
                if tys.len() > 4 {
                    self.diag(
                        ARG_COUNT,
                        format!(
                            "`echo` nhận tối đa 4 đối số (a, b, c, --sep), được {}",
                            tys.len()
                        ),
                        span,
                    );
                }
                Some(Type::Str)
            }
            "lines" => {
                if !arity(1) {
                    self.diag(ARG_COUNT, "`lines` cần 1 đối số", span);
                }
                Some(Type::List(Box::new(Type::Str)))
            }
            "take" => {
                if !arity(2) {
                    self.diag(ARG_COUNT, "`take` cần 2 đối số", span);
                } else {
                    if let Some(t) = tys.first() {
                        if !matches!(t, Type::List(_) | Type::Error) {
                            self.diag(MISMATCH, "`take` cần `list[str]` ở đối số 1", span);
                        }
                    }
                    if let Some(t) = tys.get(1) {
                        if !matches!(t, Type::Int | Type::Error) {
                            self.diag(MISMATCH, "`take` cần `int` ở đối số 2", span);
                        }
                    }
                }
                Some(Type::List(Box::new(Type::Str)))
            }
            "len" => {
                if !arity(1) {
                    self.diag(
                        ARG_COUNT,
                        format!("`len` cần 1 đối số, được {}", args.len()),
                        span,
                    );
                } else if let Some(t) = tys.first() {
                    let ok = matches!(
                        t,
                        Type::Str | Type::List(_) | Type::Map(..) | Type::Bytes | Type::Error
                    );
                    if !ok {
                        let t = t.clone();
                        self.diag(
                            MISMATCH,
                            format!(
                                "`len` không nhận `{}` (chỉ `str`/`list`/`map`/`bytes`)",
                                self.show(&t)
                            ),
                            span,
                        );
                    }
                }
                Some(Type::Int)
            }
            "Some" => {
                if !arity(1) {
                    self.diag(ARG_COUNT, "`Some` cần 1 đối số", span);
                }
                Some(Type::Opt(Box::new(t0())))
            }
            "None" => {
                if !args.is_empty() {
                    self.diag(ARG_COUNT, "`None` không có đối số", span);
                }
                Some(Type::Opt(Box::new(Type::Error)))
            }
            "Ok" => {
                if !arity(1) {
                    self.diag(ARG_COUNT, "`Ok` cần 1 đối số", span);
                }
                Some(Type::Result(Box::new(t0()), Box::new(Type::Error)))
            }
            "Err" => {
                if !arity(1) {
                    self.diag(ARG_COUNT, "`Err` cần 1 đối số", span);
                }
                Some(Type::Result(Box::new(Type::Error), Box::new(t0())))
            }
            "ref" => {
                if !arity(1) {
                    self.diag(ARG_COUNT, "`ref` cần 1 đối số", span);
                }
                Some(Type::Error)
            }
            _ => None,
        }
    }
}
