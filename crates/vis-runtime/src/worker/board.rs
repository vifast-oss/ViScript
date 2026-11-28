// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-323a** — bảng completion/wait (`TaskWaitBoard`/`OutcomeBoard`), tách nguyên văn từ `pool.rs`.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use super::pool::TaskOutcome;
use super::proto::{SendValue, TaskId};

/// **Prototype proxy completion (VS3-3b/W5)** — bảng kết quả theo `TaskId`, để evaluator
/// `await` mà không cần `Rc<Task>`. Worker công bố kết quả ngay khi task xong (W>1);
/// `join` công bố nốt phần còn lại (W=1, task bị huỷ trước khi chạy). Bảng chỉ chứa
/// `SendValue`/`String` — `Send + Sync`, không shared mutable heap của ngôn ngữ.
///
/// **Chưa nối evaluator.** Một board dùng cho một pool; `wait_timeout` chỉ dành cho
/// **main thread** (worker không được block chờ worker khác — evaluator phải `yield`).
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct TaskWaitBoard {
    state: Mutex<WaitState>,
    cv: Condvar,
}

#[derive(Debug, Default)]
struct WaitState {
    done: HashMap<TaskId, Result<SendValue, String>>,
    closed: bool,
}

/// Lỗi chờ completion; tách khỏi `WorkerError` để không đổi API cũ.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitError {
    /// Hết thời gian chờ.
    Timeout,
    /// Bảng đã đóng trước khi có kết quả.
    Closed,
}

impl TaskWaitBoard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Công bố kết quả **một lần**; gọi lại cùng `id` là no-op (first wins).
    pub fn publish(&self, id: TaskId, result: Result<SendValue, String>) {
        let mut st = self.lock();
        st.done.entry(id).or_insert(result);
        self.cv.notify_all();
    }

    /// Lấy kết quả nếu đã xong (không chờ); lấy xong thì bỏ khỏi bảng.
    pub fn try_take(&self, id: TaskId) -> Option<Result<SendValue, String>> {
        self.lock().done.remove(&id)
    }

    /// Chờ completion của `id`; **không** chờ vô hạn nên không thể treo cứng.
    pub fn wait_timeout(
        &self,
        id: TaskId,
        timeout: Duration,
    ) -> Result<Result<SendValue, String>, WaitError> {
        let mut st = self.lock();
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(r) = st.done.remove(&id) {
                return Ok(r);
            }
            if st.closed {
                return Err(WaitError::Closed);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(WaitError::Timeout);
            }
            let (guard, _) = self
                .cv
                .wait_timeout(st, left)
                .unwrap_or_else(|e| e.into_inner());
            st = guard;
        }
    }

    /// Đóng bảng: đánh thức waiter còn lại bằng `WaitError::Closed`.
    pub fn close(&self) {
        self.lock().closed = true;
        self.cv.notify_all();
    }

    pub fn len(&self) -> usize {
        self.lock().done.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WaitState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// **VS3-3b/W5 (P2)** — board outcome **đầy đủ** (result + output + dirty + budget) để
/// evaluator `await` một task trước `join` mà vẫn merge output đúng thứ tự tiêu thụ.
/// Worker publish ngay khi task xong; `join` vẫn giữ bản trên completion channel để final
/// join không phụ thuộc việc caller đã `await`. Chỉ `Send`; không shared heap ngôn ngữ.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct OutcomeBoard {
    state: Mutex<OutcomeState>,
    cv: Condvar,
}

#[derive(Debug, Default)]
struct OutcomeState {
    done: HashMap<TaskId, TaskOutcome>,
    closed: bool,
}

impl OutcomeBoard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Công bố outcome một lần; gọi lại cùng `id` là no-op (first wins).
    pub fn publish(&self, outcome: TaskOutcome) {
        let mut st = self.lock();
        st.done.entry(outcome.id).or_insert(outcome);
        self.cv.notify_all();
    }

    pub fn try_take(&self, id: TaskId) -> Option<TaskOutcome> {
        self.lock().done.remove(&id)
    }

    /// **ST-292 (P4)** — chờ **một trong** `ids` có outcome; trả `(id, outcome)` đầu tiên.
    /// Dùng cho pump frame: không giữ `RefCell` borrow của scheduler khi chờ.
    pub fn wait_any_timeout(
        &self,
        ids: &[TaskId],
        timeout: Duration,
    ) -> Result<(TaskId, TaskOutcome), WaitError> {
        let mut st = self.lock();
        let deadline = Instant::now() + timeout;
        loop {
            for id in ids {
                if let Some(o) = st.done.remove(id) {
                    return Ok((*id, o));
                }
            }
            if st.closed {
                return Err(WaitError::Closed);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(WaitError::Timeout);
            }
            let (guard, _) = self
                .cv
                .wait_timeout(st, left)
                .unwrap_or_else(|e| e.into_inner());
            st = guard;
        }
    }

    pub fn wait_timeout(&self, id: TaskId, timeout: Duration) -> Result<TaskOutcome, WaitError> {
        let mut st = self.lock();
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(o) = st.done.remove(&id) {
                return Ok(o);
            }
            if st.closed {
                return Err(WaitError::Closed);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(WaitError::Timeout);
            }
            let (guard, _) = self
                .cv
                .wait_timeout(st, left)
                .unwrap_or_else(|e| e.into_inner());
            st = guard;
        }
    }

    pub fn close(&self) {
        self.lock().closed = true;
        self.cv.notify_all();
    }

    pub fn len(&self) -> usize {
        self.lock().done.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, OutcomeState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}
