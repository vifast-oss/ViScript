// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — stage giả `| complete` (ST-34): đổi tên callee stage trước thành
//! `exec.complete.<ten>` (nguyên văn từ `lower/cmd.rs`; 0 đổi hành vi).

use super::super::*;
use vis_diag::COMMAND_CODES;
use vis_syntax::NodeKind;

impl<'a> Lowerer<'a> {
    /// ST-34: `| complete` — stage **giả**, chỉ dùng sau chương trình ngoài; nó **đổi tên** callee
    /// của stage trước thành `exec.complete.<ten>` (không thêm biến thể HIR ⇒ codegen không đổi).
    pub(super) fn ha_complete(
        &mut self,
        node: &Node,
        truoc: &Option<(HExpr, Type)>,
    ) -> Option<HExpr> {
        if node.child(NodeKind::CmdGroup).is_some() || node.elems.len() > 1 {
            // `complete` không nhận đối số.
            self.diag(&COMMAND_CODES[1], "`complete` không nhận đối số", node.span);
            return None;
        }
        let Some((up, _)) = truoc.clone() else {
            self.diag(
                &COMMAND_CODES[1],
                "complete chỉ dùng sau chương trình ngoài",
                node.span,
            );
            return None;
        };
        if let HExpr::Call {
            callee,
            args,
            named,
            span,
        } = up
        {
            if let HExpr::Var { name, sym, .. } = *callee {
                if let Some(con) = name.strip_prefix("exec.") {
                    return Some(HExpr::Call {
                        callee: Box::new(HExpr::Var {
                            name: format!("exec.complete.{con}"),
                            sym,
                            span,
                        }),
                        args,
                        named,
                        span,
                    });
                }
            }
        }
        self.diag(
            &COMMAND_CODES[1],
            "complete chỉ dùng sau chương trình ngoài",
            node.span,
        );
        None
    }
}
