// SPDX-License-Identifier: MIT OR Apache-2.0
//! Xét hết `match` (`VS0402`, VD-43 #5) — tách khỏi `pat.rs` ở `ST-317b`.

use std::collections::BTreeSet;

use vis_syntax::Span;

use vis_hir::hir::{HArm, HPat};
use vis_hir::ty::Type;

use super::phu::{bool_names, ctor_names, phu_tat_ca, record_cov, tuple_phu_het};
use crate::tyck::{Checker, NON_EXHAUSTIVE};

impl<'a> Checker<'a> {
    /// `match` phải **xét hết** — `guard` không tính (thiết kế §4.3).
    pub(crate) fn xet_het(&mut self, subj: &Type, arms: &[HArm], span: Span) {
        if arms.is_empty() {
            self.diag(NON_EXHAUSTIVE, "`match` không có nhánh nào", span);
            return;
        }
        // Chỉ nhánh **không guard** mới chứng minh phủ.
        let plain: Vec<&HPat> = arms
            .iter()
            .filter(|a| a.guard.is_none())
            .map(|a| &a.pat)
            .collect();
        if plain.iter().any(|p| phu_tat_ca(p)) {
            return;
        }
        match subj {
            Type::Error => {}
            Type::Enum(id) => {
                let Some(def) = self.types.enum_def(*id) else {
                    return;
                };
                let mut cov = BTreeSet::new();
                for p in &plain {
                    ctor_names(p, &mut cov);
                }
                let missing: Vec<String> = def
                    .variants
                    .iter()
                    .filter(|v| !cov.contains(&v.name.to_ascii_lowercase()))
                    .map(|v| v.name.clone())
                    .collect();
                if !missing.is_empty() {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!("`match` thiếu nhánh: {}", missing.join(", ")),
                        span,
                    );
                }
            }
            Type::Opt(_) | Type::Result(..) => {
                let mut cov = BTreeSet::new();
                for p in &plain {
                    ctor_names(p, &mut cov);
                }
                let need: &[&str] = match subj {
                    // `ctor_names` so khớp **thường**; thông điệp in **hoa** chữ đầu (UpperCamel).
                    Type::Opt(_) => &["some", "none"],
                    _ => &["ok", "err"],
                };
                let hoa = |n: &str| -> String {
                    let mut c = n.chars();
                    match c.next() {
                        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        None => n.to_string(),
                    }
                };
                let missing: Vec<String> = need
                    .iter()
                    .filter(|n| !cov.contains(**n))
                    .map(|n| hoa(n))
                    .collect();
                if !missing.is_empty() {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!("`match` thiếu nhánh: {}", missing.join(", ")),
                        span,
                    );
                }
            }
            Type::Bool => {
                let mut cov = BTreeSet::new();
                for p in &plain {
                    bool_names(p, &mut cov);
                }
                let missing: Vec<&str> = ["true", "false"]
                    .into_iter()
                    .filter(|n| !cov.contains(*n))
                    .collect();
                if !missing.is_empty() {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!("`match` thiếu nhánh: {}", missing.join(", ")),
                        span,
                    );
                }
            }
            Type::Record(id) => {
                let Some(def) = self.types.rec(*id) else {
                    return;
                };
                let all: BTreeSet<String> = def.fields.iter().map(|f| f.name.clone()).collect();
                // Một nhánh **không `..`** phủ đủ mọi trường ⇒ xét hết.
                for p in &plain {
                    let mut cov = BTreeSet::new();
                    let mut open = false;
                    record_cov(p, &mut cov, &mut open);
                    // ST-15b (mục 3): nhánh c�� `..` (open) **phủ mọi trường còn lại** (§4.3).
                    if open || all.is_subset(&cov) {
                        return;
                    }
                }
                let mut cov = BTreeSet::new();
                let mut open = false;
                for p in &plain {
                    record_cov(p, &mut cov, &mut open);
                }
                let missing: Vec<String> = all.difference(&cov).cloned().collect();
                self.diag(
                    NON_EXHAUSTIVE,
                    format!(
                        "`match` trên bản ghi phải có đủ mọi trường (không `..`), thiếu: {}",
                        missing.join(", ")
                    ),
                    span,
                );
            }
            // ST-12b MAJOR #7: `unit` và tuple là kiểu **hữu hạn** — không đòi `_` oan.
            Type::Unit => {
                let covered = plain
                    .iter()
                    .any(|p| matches!(p, HPat::Tuple { items, .. } if items.is_empty()));
                if !covered {
                    self.diag(
                        NON_EXHAUSTIVE,
                        "`match` trên `()` cần nhánh `()` hoặc `_` (kiểu `unit` chỉ có một giá trị)",
                        span,
                    );
                }
            }
            Type::TypeRef { name, args } if name == "tuple" => {
                let n = args.len();
                let covered = plain.iter().any(|p| tuple_phu_het(p, n));
                if !covered {
                    self.diag(
                        NON_EXHAUSTIVE,
                        format!(
                            "`match` trên tuple {n} thành phần phải phủ **mọi** thành phần bằng `_`/tên trần (hoặc dùng `_` cho cả tuple)"
                        ),
                        span,
                    );
                }
            }
            other => {
                self.diag(
                    NON_EXHAUSTIVE,
                    format!(
                        "`{}` là kiểu vô hạn — `match` bắt buộc có nhánh `_` (thiết kế §4.3)",
                        self.show(other)
                    ),
                    span,
                );
            }
        }
    }
}
