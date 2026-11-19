// SPDX-License-Identifier: MIT OR Apache-2.0
//! Tiện ích nhỏ của codegen: `can_test`, tên dựng sẵn, ánh xạ `AssignOp` ⇒ `BinOp`.

use vis_types::hir::HPat;
use vis_types::{AssignOp, BinOp};

use crate::codegen::CodegenError;
use vis_types::Symbols;

/// Chỉ số các ký hiệu dựng sẵn + tên (dùng tại `HExpr::Var` để dựng giá trị bằng `$value:<tên>`).
pub(crate) fn tinh_cac_builtin(symbols: &Symbols) -> std::collections::HashMap<u32, String> {
    let mut m = std::collections::HashMap::new();
    for (i, s) in symbols.syms.iter().enumerate() {
        if s.kind == vis_types::lower::SymKind::Builtin {
            m.insert(i as u32, s.name.clone());
        }
    }
    m
}

/// Pattern con có cần **test** không (literal/ctor/tuple/list/record/range/or).
pub fn can_test(p: &HPat) -> bool {
    matches!(
        p,
        HPat::Lit { .. }
            | HPat::Ctor { .. }
            | HPat::Tuple { .. }
            | HPat::List { .. }
            | HPat::Record { .. }
            | HPat::Range { .. }
            | HPat::Or { .. }
    )
}

/// Tên **dựng sẵn** (phải khớp `vis_interp::builtin::SIGS`; test `codegen` khoá lại).
pub fn la_builtin(name: &str) -> bool {
    // ST-26: `std.<module>.<fn>` (tên đầy đủ trong HIR) — bảng chữ ký ở `vis_types::stdlib`.
    if name.starts_with("exec.") {
        return true;
    }
    if name.starts_with("std.") {
        return vis_types::stdlib::la_std(name.trim_start_matches("std."));
    }
    matches!(
        name,
        "print"
            | "len"
            | "str"
            | "echo"
            | "lines"
            | "take"
            | "Ok"
            | "Err"
            | "Some"
            | "None"
            | "ref"
    )
}

pub fn assign_binop(op: vis_types::AssignOp) -> Result<vis_types::BinOp, CodegenError> {
    Ok(match op {
        AssignOp::Set => BinOp::Add,
        AssignOp::Add => BinOp::Add,
        AssignOp::Sub => BinOp::Sub,
        AssignOp::Mul => BinOp::Mul,
        AssignOp::Div => BinOp::Div,
        AssignOp::FloorDiv => BinOp::FloorDiv,
        AssignOp::Rem => BinOp::Rem,
    })
}

/// **VD-241 §3+ (A0, 2026-11-19)** — `sym` → kiểu annot (chỉ ký hiệu có annot `Some(…)`);
/// dùng ở biên gán lại (mirror interp). Tách khỏi `codegen.rs` để file giữ **R1** ≤ 1.500 dòng.
pub(crate) fn sym_annot_map(symbols: &Symbols) -> std::collections::HashMap<u32, vis_types::Type> {
    symbols
        .syms
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.ty.clone().map(|t| (i as u32, t)))
        .collect()
}

/// Vòng lặp (break/continue đích) — dùng chung `codegen`/`codegen::than`.
pub(crate) struct Vong {
    /// Đích cần vá khi gặp `break` (về cuối vòng).
    pub(crate) breaks: Vec<usize>,
    /// Đích nhảy của `continue` (về đầu vòng).
    pub(crate) continue_t: usize,
}
