// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-318c** — dựng **đối số cuối stage** (điền default builtin, kiểm tham số bắt buộc) (nguyên
//! văn từ tail `Lowerer::ha_stage` trong `lower/cmd.rs`; 0 đổi hành vi).

use super::super::*;
use vis_diag::COMMAND_CODES;
use vis_hir::cmdsig::CmdSig;

/// Tên kiểu ngắn để đưa vào thông điệp `VS0602`/`VS0603`.
/// **ST-122 (VD-127)**: giá trị `default` của builtin được dựng **một lần** trong
/// [`vis_hir::cmdsig::BuiltinSigs::mac_dinh`] với `Span::new(0, 0)` (`file = 0`). Khi **chép** nó vào
/// HIR của một tệp, node đó là **node tổng hợp của tệp đang hạ** ⇒ phải mang `file` của tệp ấy.
/// Giữ nguyên `start/end` (giá trị mặc định **không có** vị trí nguồn — chỉ `file` là thông tin đúng).
fn mac_dinh_theo_tep(e: HExpr, file: u16) -> HExpr {
    match e {
        HExpr::Lit { lit, ty, span } => HExpr::Lit {
            lit,
            ty,
            span: span.with_file(file),
        },
        // Các dạng khác chưa có trong bảng builtin; nếu thêm, **audit** (`audit_span`) sẽ bắt.
        other => other,
    }
}

impl<'a> Lowerer<'a> {
    /// Dựng đối số cuối stage: builtin nhận **đối số vị trí** (điền default), còn lại kiểm tham số
    /// **bắt buộc** rồi trả call (nguyên văn tail `ha_stage`).
    pub(super) fn ket_thuc_stage(
        &mut self,
        node: &Node,
        sig: &CmdSig,
        ten: &str,
        mut vi_tri: Vec<Option<HExpr>>,
        mut theo_ten: Vec<(String, HExpr)>,
    ) -> Option<HExpr> {
        // Builtin: interp nhận **đối số vị trí** (không có binding theo tên) ⇒ đặt cờ vào đúng ô,
        // điền default cho các ô trống phía trước.
        if sig.sym.is_none() {
            for (ten_co, gt) in &theo_ten {
                if let Some(i) = sig.params.iter().position(|p| p.named && &p.name == ten_co) {
                    for (j, o) in vi_tri.iter_mut().enumerate().take(i) {
                        if o.is_none() {
                            *o = sig.params[j]
                                .default
                                .clone()
                                .map(|d| mac_dinh_theo_tep(d, self.file));
                        }
                    }
                    vi_tri[i] = Some(gt.clone());
                }
            }
            theo_ten.clear();
            // ST-42: điền default cho **mọi** ô còn trống (kể cả ô **sau** cờ, vd `long` sau `--all`),
            // nếu không `day_du` sai ⇒ cờ bị **bỏ** khỏi call.
            for (j, o) in vi_tri.iter_mut().enumerate() {
                if o.is_none() {
                    *o = sig.params[j]
                        .default
                        .clone()
                        .map(|d| mac_dinh_theo_tep(d, self.file));
                }
            }
            let day_du = vi_tri.iter().all(|x| x.is_some());
            if day_du {
                let args: Vec<HExpr> = vi_tri.into_iter().flatten().collect();
                return Some(HExpr::Call {
                    callee: Box::new(HExpr::Var {
                        name: sig.name.clone(),
                        sym: None,
                        span: node.span,
                    }),
                    args,
                    named: Vec::new(),
                    span: node.span,
                });
            }
        }
        // Thiếu tham số **bắt buộc** (không default) ⇒ VS0602.
        let mut args: Vec<HExpr> = Vec::new();
        for (i, p) in sig.params.iter().enumerate() {
            if p.named {
                continue;
            }
            match &vi_tri[i] {
                Some(e) => args.push(e.clone()),
                None if p.default.is_none() => {
                    self.diag(
                        &COMMAND_CODES[1],
                        format!("`{ten}` thiếu đối số `{}`", p.name),
                        node.span,
                    );
                    return None;
                }
                None => {
                    // Default ở giữa: truyền theo tên để interp tự điền default.
                    break;
                }
            }
        }
        let callee = match sig.sym {
            Some(sym) => HExpr::Var {
                name: sig.name.clone(),
                sym: Some(sym),
                span: node.span,
            },
            None => HExpr::Var {
                name: sig.name.clone(),
                sym: None,
                span: node.span,
            },
        };
        Some(HExpr::Call {
            callee: Box::new(callee),
            args,
            named: theo_ten,
            span: node.span,
        })
    }
}
