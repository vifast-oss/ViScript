// SPDX-License-Identifier: MIT OR Apache-2.0
//! Helper phủ pattern dùng chung (`_`/tên trần, tên ctor, bản ghi, tuple, `cap_dau`) — tách khỏi
//! `pat.rs` ở `ST-317b`.

use std::collections::BTreeSet;

use vis_hir::hir::{HPat, LitKind};

/// Pattern phủ **mọi** giá trị (`_`, tên trần, hoặc or-pattern có một vế như vậy).
pub(super) fn phu_tat_ca(p: &HPat) -> bool {
    match p {
        HPat::Wild { .. } | HPat::Bind { .. } => true,
        HPat::Or { alts, .. } => alts.iter().any(phu_tat_ca),
        _ => false,
    }
}

/// **VD-212 (A0)**: viết thường chữ cái đầu (`SOME` ⇒ `Some`) — dùng để nhận ra biến thể viết sai
/// hoa thường và gợi ý dạng đúng. Giữ nguyên phần còn lại (không `title case`).
pub(super) fn cap_dau(s: &str) -> String {
    let low = s.to_ascii_lowercase();
    let mut it = low.chars();
    match it.next() {
        Some(f) => f.to_uppercase().collect::<String>() + it.as_str(),
        None => low,
    }
}

pub(super) fn ctor_names(p: &HPat, out: &mut BTreeSet<String>) {
    match p {
        HPat::Ctor { path, .. } => {
            if let Some(n) = path.last() {
                out.insert(n.to_ascii_lowercase());
            }
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                ctor_names(a, out);
            }
        }
        _ => {}
    }
}

pub(super) fn bool_names(p: &HPat, out: &mut BTreeSet<String>) {
    match p {
        HPat::Lit { lit, .. } if lit.kind == LitKind::Bool => {
            out.insert(lit.text.clone());
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                bool_names(a, out);
            }
        }
        _ => {}
    }
}

pub(super) fn record_cov(p: &HPat, out: &mut BTreeSet<String>, open: &mut bool) {
    match p {
        HPat::Record {
            fields, open: o, ..
        } => {
            for (n, _) in fields {
                if !n.is_empty() {
                    out.insert(n.clone());
                }
            }
            if *o {
                *open = true;
            }
        }
        HPat::Or { alts, .. } => {
            for a in alts {
                record_cov(a, out, open);
            }
        }
        _ => {}
    }
}

/// Tuple pattern phủ hết tuple `n` thành phần: đúng số thành phần và **mọi** thành phần bất khả bác
/// (`_`, tên trần, hoặc tuple con như vậy).
pub(super) fn tuple_phu_het(p: &HPat, n: usize) -> bool {
    match p {
        HPat::Tuple { items, .. } => items.len() == n && items.iter().all(bat_kha_bac),
        _ => false,
    }
}

pub(super) fn bat_kha_bac(p: &HPat) -> bool {
    match p {
        HPat::Wild { .. } | HPat::Bind { .. } => true,
        HPat::Tuple { items, .. } => items.iter().all(bat_kha_bac),
        HPat::Or { alts, .. } => alts.iter().any(bat_kha_bac),
        _ => false,
    }
}
