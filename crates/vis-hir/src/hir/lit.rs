// SPDX-License-Identifier: MIT OR Apache-2.0
//! Literal HIR: [`Lit`] + [`LitKind`]. Tách nguyên văn từ `hir.rs` (ST-318d).

use std::fmt;

use crate::ty::Type;

/// Literal: **nguyên văn nguồn** + loại (để `Display`/chẩn đoán in lại đúng như người viết).
#[derive(Debug, Clone, PartialEq)]
pub struct Lit {
    pub kind: LitKind,
    /// Nguyên văn token (đã NFC) — `1`, `"a"`, `50pct`, `p"./x"`, `true`.
    pub text: String,
}

impl Lit {
    /// Kiểu của literal (VD-43).
    pub fn ty(&self) -> Type {
        match self.kind {
            LitKind::Int => Type::Int,
            LitKind::Float => Type::Float,
            LitKind::Size => Type::Size,
            LitKind::Duration => Type::Duration,
            LitKind::Percent => Type::Percent,
            LitKind::Str => Type::Str,
            LitKind::Bytes => Type::Bytes,
            LitKind::Path => Type::Path,
            LitKind::Char => Type::Char,
            LitKind::Bool => Type::Bool,
        }
    }
}

impl fmt::Display for Lit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Loại literal mà HIR phân biệt (mỗi loại ⇒ đúng một [`Type`] ở [`Lit::ty`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LitKind {
    Int,
    Float,
    Size,
    Duration,
    Percent,
    Str,
    Bytes,
    Path,
    Char,
    Bool,
}
