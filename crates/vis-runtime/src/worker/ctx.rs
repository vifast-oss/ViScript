// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-323a** — `WorkerCtx` (env/output/budget/cancel của một task), tách nguyên văn từ `pool.rs`.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::rt::fee::{BudgetError, BudgetLease, SharedBudget};

use super::pool::{TaskOutcome, TaskOutput};
use super::proto::{EnvMap, SendValue, TaskId};

/// Env + output cục bộ của **một task**. Không chia sẻ với task/worker khác.
pub struct WorkerCtx {
    globals: EnvMap,
    locals: EnvMap,
    dirty: BTreeSet<String>,
    out: TaskOutput,
    shared: bool,
    cancel: Arc<AtomicBool>,
    budget: Option<BudgetLease>,
}

impl WorkerCtx {
    pub(super) fn new(
        globals: EnvMap,
        shared: bool,
        cancel: Arc<AtomicBool>,
        budget: Option<Arc<SharedBudget>>,
    ) -> Self {
        Self {
            globals,
            locals: EnvMap::new(),
            dirty: BTreeSet::new(),
            out: TaskOutput::default(),
            shared,
            cancel,
            budget: budget.map(BudgetLease::new),
        }
    }

    /// Đọc top-level `var` (snapshot ở W>1, env chung ở W=1).
    pub fn get(&self, name: &str) -> Option<&SendValue> {
        self.globals.get(name)
    }

    pub fn get_local(&self, name: &str) -> Option<&SendValue> {
        self.locals.get(name)
    }

    /// Ghi top-level `var`. W=1 ghi env chung; W>1 ghi bản sao + đánh dấu `dirty`.
    pub fn set(&mut self, name: impl Into<String>, value: SendValue) {
        let name = name.into();
        self.globals.insert(name.clone(), value);
        if !self.shared {
            self.dirty.insert(name);
        }
    }

    pub fn set_local(&mut self, name: impl Into<String>, value: SendValue) {
        self.locals.insert(name.into(), value);
    }

    /// `print` của task: chỉ vào buffer riêng; pool merge sau.
    pub fn print(&mut self, s: &str) {
        self.out.push_str(s);
    }

    pub fn print_line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push_str("\n");
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    /// **P2**: cờ huỷ để evaluator kiểm tại safe point.
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    /// **P2**: đánh dấu top-level `var` bị task ghi (cross-write W>1).
    pub fn mark_dirty(&mut self, name: impl Into<String>) {
        self.dirty.insert(name.into());
    }

    /// Task tự kiểm huỷ tại safe point; trả lỗi để dừng thân.
    pub fn check_cancel(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err("tác vụ bị huỷ".to_string())
        } else {
            Ok(())
        }
    }

    /// Trừ ngân sách atomic (nếu pool có budget); chunk reservation nằm trong `BudgetLease`.
    pub fn charge(&mut self, weight: i64) -> Result<(), String> {
        match &mut self.budget {
            Some(b) => b.charge(weight).map_err(|e| match e {
                BudgetError::Exhausted { remaining } => {
                    format!("VS0510: vượt ngân sách worker (còn {remaining})")
                }
            }),
            None => Ok(()),
        }
    }

    pub fn budget_used(&self) -> i64 {
        self.budget.as_ref().map(BudgetLease::used).unwrap_or(0)
    }

    pub(super) fn globals(&self) -> &EnvMap {
        &self.globals
    }

    pub(super) fn into_outcome(self, id: TaskId, result: Result<SendValue, String>) -> TaskOutcome {
        let cancelled = self.cancel.load(Ordering::Acquire);
        let used = self.budget.as_ref().map(BudgetLease::used).unwrap_or(0);
        let WorkerCtx {
            dirty, out, budget, ..
        } = self;
        drop(budget);
        TaskOutcome {
            id,
            result,
            output: out.into_string(),
            cancelled,
            dirty: dirty.into_iter().collect(),
            budget_used: used,
        }
    }
}
