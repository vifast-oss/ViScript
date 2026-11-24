// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b/W5 (P1/P2)** — runner W=1 trên caller + runner HIR trên worker.
//!
//! W=1: `chay_than_local` dùng `Host::goi_than`, shared env, không snapshot/thread.
//! W>1: `chay_than_worker` dựng `Interp` **riêng** trong worker từ `Arc<HirImage>`, nạp
//! snapshot top-level (`SendValue::Map`), in vào buffer của task, kiểm cancel/ngân sách và
//! phát hiện ghi top-level để pool báo cross-write.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use vis_hir::hir::{HPat, HStmt};
use vis_syntax::Span;

use crate::interp::{Abort, Interp, Trap, RT};
use crate::rt::fee::{BudgetLease, SharedBudget};
use crate::value::Value;
use vis_runtime::builtin::Host;
use vis_runtime::rt::exec_state::HirImage;
use vis_runtime::worker::{SendValue, ToSend, WorkerCtx};

/// **P2**: gom `sym` của mọi binding cấp tệp (Var/Let, kể cả pattern lồng) — snapshot worker
/// chỉ lấy các binding này, **không** lấy tham số/local của `main`.
pub(crate) fn top_syms(stmts: &[HStmt]) -> Vec<u32> {
    let mut out = Vec::new();
    for s in stmts {
        match s {
            HStmt::Var { sym, .. } => out.push(*sym),
            HStmt::Let { pat, .. } => gom_pat(pat, &mut out),
            _ => {}
        }
    }
    out
}

fn gom_pat(p: &HPat, out: &mut Vec<u32>) {
    match p {
        HPat::Bind { sym, .. } => out.push(*sym),
        HPat::Ctor { args, .. } | HPat::Tuple { items: args, .. } | HPat::Or { alts: args, .. } => {
            for a in args {
                gom_pat(a, out);
            }
        }
        HPat::List { items, rest, .. } => {
            for a in items {
                gom_pat(a, out);
            }
            if let Some(r) = rest {
                gom_pat(r, out);
            }
        }
        HPat::Record { fields, .. } => {
            for (_, f) in fields {
                if let Some(f) = f {
                    gom_pat(f, out);
                }
            }
        }
        HPat::Range { lo, hi, .. } => {
            if let Some(x) = lo {
                gom_pat(x, out);
            }
            if let Some(x) = hi {
                gom_pat(x, out);
            }
        }
        HPat::Wild { .. } | HPat::Lit { .. } | HPat::Rest { .. } => {}
    }
}

/// Snapshot **top-level** của interp thành `SendValue::Map(Int(sym) -> value)`.
pub(crate) fn snapshot_top(it: &Interp<'_>) -> Result<Arc<SendValue>, String> {
    let env = it.env.borrow();
    let mut pairs = Vec::with_capacity(it.top_syms.len());
    for sym in &it.top_syms {
        let Some(v) = env.get(sym) else { continue };
        let sv = v
            .to_send()
            .map_err(|e| format!("biến cấp tệp sym {sym} không Sendable (VS0508): {e}"))?;
        pairs.push((SendValue::Int(*sym as i64), sv));
    }
    Ok(Arc::new(SendValue::Map(pairs)))
}

/// Safe point huỷ hợp tác: kiểm mỗi 1.024 bước đánh giá; `None` ở runtime thường.
impl Interp<'_> {
    pub(crate) fn worker_tick(&mut self, span: Span) -> Option<Abort> {
        if self.steps & 1023 != 0 {
            return None;
        }
        if let Some(d) = self.worker_cancel_deadline {
            if Instant::now() >= d {
                return Some(Abort::Trap(Trap::new(RT, "tác vụ bị huỷ", span)));
            }
        }
        let c = self.worker_cancel.as_ref()?;
        if c.load(Ordering::Acquire) {
            Some(Abort::Trap(Trap::new(RT, "tác vụ bị huỷ", span)))
        } else {
            None
        }
    }
}

/// Dựng `Interp` **trong worker** từ ảnh bất biến + snapshot; không chạy init top-level.
fn tao_interp<'a>(
    image: &'a HirImage,
    snap: &SendValue,
    cancel: Arc<AtomicBool>,
    budget: Option<Arc<SharedBudget>>,
    deadline: Option<Instant>,
) -> Result<Interp<'a>, String> {
    let mut it = Interp::new(&image.hir, &image.symbols, &image.types);
    // Task không chạy khởi tạo cấp tệp: nạp lại tập `var` cấp tệp để gán đúng luật mutable.
    for s in it.stmts {
        if let HStmt::Var { sym, .. } = s {
            it.mutable.insert(*sym);
        }
    }
    let SendValue::Map(pairs) = snap else {
        return Err("snapshot worker không phải Map".to_string());
    };
    for (k, v) in pairs {
        let SendValue::Int(sym) = k else {
            return Err("snapshot worker có khoá không phải Int(sym)".to_string());
        };
        it.env.borrow_mut().insert(*sym as u32, v.to_value());
    }
    it.worker_cancel = Some(cancel);
    it.worker_cancel_deadline = deadline;
    it.worker_budget = budget.map(BudgetLease::new);
    Ok(it)
}

/// **P2 runner HIR trên worker**: trả `SendValue`; output vào `WorkerCtx`; ghi top-level ⇒ dirty.
#[allow(clippy::too_many_arguments)]
pub(crate) fn chay_than_worker(
    image: &HirImage,
    than: u32,
    args: Vec<SendValue>,
    span: Span,
    ctx: &mut WorkerCtx,
    snap: &SendValue,
    budget: Option<Arc<SharedBudget>>,
    deadline: Option<Instant>,
) -> Result<SendValue, String> {
    let cancel = ctx.cancel_flag();
    let mut it = tao_interp(image, snap, cancel, budget, deadline)?;
    let args: Vec<Value> = args.iter().map(SendValue::to_value).collect();
    let before: Vec<(u32, Value)> = {
        let env = it.env.borrow();
        it.top_syms
            .iter()
            .filter_map(|sym| env.get(sym).cloned().map(|v| (*sym, v)))
            .collect()
    };

    let res: Result<Value, String> = match Host::goi_than(&mut it, than, args, span) {
        Ok(v) => Ok(v),
        Err(Abort::Ret(v)) => Ok(v),
        Err(Abort::Trap(t)) => Err(format!("{}: {}", t.code.short, t.msg)),
        Err(Abort::Pending(_)) => Err("pending trong thân tác vụ worker".to_string()),
    };
    // Nested task không ai `await` vẫn phải chạy hết trong worker (như final join của main);
    // output/dirty của cây con nằm chung buffer/lease của task cha (plan D9).
    let res = if res.is_ok() {
        match crate::rt::join_all(&mut it) {
            Ok(()) => res,
            Err(Abort::Trap(t)) => Err(format!("{}: {}", t.code.short, t.msg)),
            Err(_) => Err("join tác vụ lồng trong worker lỗi".to_string()),
        }
    } else {
        res
    };

    let out = it.take_stdout();
    if !out.is_empty() {
        ctx.print(&out);
    }

    // Phát hiện ghi top-level (không cần hook từng đường gán): so ảnh trước/sau.
    {
        let env = it.env.borrow();
        for (sym, old) in &before {
            let Some(new) = env.get(sym) else { continue };
            if !old.eq_value(new) {
                ctx.mark_dirty(format!("s{sym}"));
            }
        }
    }

    res.and_then(|v| {
        v.to_send()
            .map_err(|e| format!("kết quả task không Sendable (VS0508): {e}"))
    })
}
