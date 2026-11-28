// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a** — bảng kết quả + `WorkerPool` (tách từ `rt/worker.rs`, nguyên văn).

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

use crate::rt::fee::{BudgetSnapshot, SharedBudget};

use super::board::{OutcomeBoard, TaskWaitBoard};
use super::ctx::WorkerCtx;
use super::proto::*;

/// **VS3-3b** — buffer `print` của **một task** (worker pool opt-in). Task chỉ ghi vào buffer
/// riêng; worker pool merge theo **thứ tự spawn** để W=1/W>1 byte-identical (VD-271 B3).
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct TaskOutput {
    buf: String,
}

impl TaskOutput {
    pub fn push_str(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    pub fn into_string(self) -> String {
        self.buf
    }
}

/// Kết quả một task, giữ output riêng để merge theo spawn order.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct TaskOutcome {
    pub id: TaskId,
    pub result: Result<SendValue, String>,
    pub output: String,
    pub cancelled: bool,
    /// Top-level `var` task đã ghi (chỉ W>1; W=1 ghi trực tiếp env chung).
    pub dirty: Vec<String>,
    pub budget_used: i64,
}

/// Kết quả `join`: output đã merge, outcome theo spawn order, env cuối (W=1), budget cuối.
#[doc(hidden)]
#[derive(Debug)]
pub struct PoolResult {
    pub output: String,
    pub outcomes: Vec<TaskOutcome>,
    pub env: EnvMap,
    pub budget: Option<BudgetSnapshot>,
}

/// Handle trả cho caller; `cancel()` là **hợp tác** (task phải gọi `ctx.check_cancel()`).
#[doc(hidden)]
#[derive(Clone)]
pub struct TaskHandle {
    id: TaskId,
    cancel: Arc<AtomicBool>,
}

impl TaskHandle {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
}

type TaskFn = Box<dyn FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send>;

/// **ST-291 (P2b-1)** — closure của một job trong lô gửi pool (payload `Arc` + fn).
type SharedFn = Box<dyn FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send>;

/// Số task con tối đa trong một message `Job` (batch). 512 đủ cắt chi phí channel cho
/// 10k task mà vẫn cân tải tốt khi task không đều.
const BATCH_CHUNK: usize = 512;

/// Một task con trong job lô: giữ `TaskId`/cờ huỷ riêng để `await`/cancel từng task.
struct BatchItem {
    id: TaskId,
    cancel: Arc<AtomicBool>,
    payload: Arc<SendValue>,
    f: SharedFn,
}

struct QueuedTask {
    id: TaskId,
    cancel: Arc<AtomicBool>,
    task: TaskFn,
}

/// **ST-291 (P2b-1)**: job đơn (đường cũ) hoặc **một lô** task con. Lô chỉ gửi `workers`
/// message thay vì N message, cắt chi phí channel/wakeup cho 10k task cực nhỏ.
enum JobKind {
    One {
        id: TaskId,
        cancel: Arc<AtomicBool>,
        task: TaskFn,
    },
    Batch(Vec<BatchItem>),
}

struct Job {
    kind: JobKind,
    snapshot: EnvMap,
    wait_board: Option<Arc<TaskWaitBoard>>,
    outcome_board: Option<Arc<OutcomeBoard>>,
}

struct CompletionBoard {
    rx: mpsc::Receiver<TaskOutcome>,
    pending: HashMap<TaskId, TaskOutcome>,
}

impl CompletionBoard {
    fn wait(&mut self, id: TaskId) -> Result<TaskOutcome, WorkerError> {
        if let Some(o) = self.pending.remove(&id) {
            return Ok(o);
        }
        loop {
            match self.rx.recv() {
                Ok(o) if o.id == id => return Ok(o),
                Ok(o) => {
                    self.pending.insert(o.id, o);
                }
                Err(_) => return Err(WorkerError::ChannelClosed),
            }
        }
    }
}

enum PoolMode {
    Sequential {
        queue: Vec<QueuedTask>,
    },
    Parallel {
        txs: Vec<mpsc::SyncSender<Job>>,
        board: CompletionBoard,
        workers: Vec<std::thread::JoinHandle<()>>,
    },
}

/// Worker pool **opt-in/test-only**. `workers == 1` chạy tuần tự trên caller (shared env);
/// `workers > 1` chạy N thread, mỗi task nhận snapshot top-level `var`.
pub struct WorkerPool {
    workers: usize,
    root: EnvMap,
    budget: Option<Arc<SharedBudget>>,
    wait_board: Option<Arc<TaskWaitBoard>>,
    outcome_board: Option<Arc<OutcomeBoard>>,
    mode: Option<PoolMode>,
    next: u64,
    order: Vec<TaskId>,
}

impl WorkerPool {
    pub fn new(workers: usize, root: EnvMap) -> Result<Self, WorkerError> {
        Self::new_inner(workers, root, None)
    }

    pub fn with_budget(
        workers: usize,
        root: EnvMap,
        budget: Arc<SharedBudget>,
    ) -> Result<Self, WorkerError> {
        Self::new_inner(workers, root, Some(budget))
    }

    fn new_inner(
        workers: usize,
        root: EnvMap,
        budget: Option<Arc<SharedBudget>>,
    ) -> Result<Self, WorkerError> {
        if workers == 0 {
            return Err(WorkerError::InvalidWorkers);
        }
        let mode = if workers == 1 {
            PoolMode::Sequential { queue: Vec::new() }
        } else {
            let (out_tx, out_rx) = mpsc::channel::<TaskOutcome>();
            let mut txs = Vec::with_capacity(workers);
            let mut handles = Vec::with_capacity(workers);
            for _ in 0..workers {
                let (tx, rx) = mpsc::sync_channel::<Job>(64);
                txs.push(tx);
                let out_tx = out_tx.clone();
                let budget = budget.clone();
                handles.push(std::thread::spawn(move || worker_loop(rx, out_tx, budget)));
            }
            drop(out_tx);
            PoolMode::Parallel {
                txs,
                board: CompletionBoard {
                    rx: out_rx,
                    pending: HashMap::new(),
                },
                workers: handles,
            }
        };
        Ok(Self {
            workers,
            root,
            budget,
            wait_board: None,
            outcome_board: None,
            mode: Some(mode),
            next: 0,
            order: Vec::new(),
        })
    }

    pub fn workers(&self) -> usize {
        self.workers
    }

    /// Env chung của main (W=1 đọc/ghi trực tiếp khi join; W>1 vẫn là mốc snapshot).
    pub fn set_env(&mut self, name: impl Into<String>, value: SendValue) {
        self.root.insert(name.into(), value);
    }

    pub fn get_env(&self, name: &str) -> Option<&SendValue> {
        self.root.get(name)
    }

    /// Gắn **bảng completion proxy** (opt-in) để `await` theo `TaskId`; gọi trước `spawn`.
    /// Một board dùng cho một pool; kết quả đã publish vẫn đọc được sau `join`.
    pub fn attach_wait_board(&mut self, board: Arc<TaskWaitBoard>) {
        self.wait_board = Some(board);
    }

    /// **P2**: gắn board outcome đầy đủ (result/output/dirty) để `await` trước `join`.
    pub fn attach_outcome_board(&mut self, board: Arc<OutcomeBoard>) {
        self.outcome_board = Some(board);
    }

    /// Spawn một task; closure phải `Send` và không được giữ `Value`/`Rc`.
    pub fn spawn<F>(&mut self, f: F) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn_inner(f, false)
    }

    /// **P2b/test**: spawn với cờ huỷ **bật trước khi job vào queue**. Dùng cho
    /// `cancel_before`: worker kiểm cờ trước khi chạy nên không có race
    /// “check false → main cancel → chạy task”.
    pub fn spawn_cancelled<F>(&mut self, f: F) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn_inner(f, true)
    }

    fn spawn_inner<F>(&mut self, f: F, cancelled: bool) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send + 'static,
    {
        let id = TaskId(self.next);
        self.next += 1;
        let cancel = Arc::new(AtomicBool::new(cancelled));
        let wait_board = self.wait_board.clone();
        let task: TaskFn = Box::new(move |ctx| {
            let result = f(ctx);
            if let Some(board) = &wait_board {
                board.publish(id, result.clone());
            }
            result
        });
        let snapshot = self.root.clone();
        let mode = self.mode.as_mut().ok_or(WorkerError::ChannelClosed)?;
        match mode {
            PoolMode::Sequential { queue } => queue.push(QueuedTask {
                id,
                cancel: Arc::clone(&cancel),
                task,
            }),
            PoolMode::Parallel { txs, .. } => {
                let idx = (self.next as usize - 1) % txs.len();
                txs[idx]
                    .send(Job {
                        kind: JobKind::One {
                            id,
                            cancel: Arc::clone(&cancel),
                            task,
                        },
                        snapshot,
                        wait_board: self.wait_board.clone(),
                        outcome_board: self.outcome_board.clone(),
                    })
                    .map_err(|_| WorkerError::ChannelClosed)?;
            }
        }
        self.order.push(id);
        Ok(TaskHandle { id, cancel })
    }

    /// **Move payload**: chuyển hẳn `payload` vào task, không copy trên producer.
    /// Dùng khi caller không còn cần giá trị (payload dựng mới hoặc đã tách ownership);
    /// handler nhận payload trong heap của worker (W=1: ngay trên caller).
    pub fn spawn_with_payload<F>(
        &mut self,
        payload: SendValue,
        f: F,
    ) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx, SendValue) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn(move |ctx| f(ctx, payload))
    }

    /// **Worker-copy**: gửi `Arc<SendValue>` (O(1)); worker tự clone payload vào heap
    /// của mình khi chạy task. Tránh copy tuần tự trên producer cho payload lớn
    /// (list10k/map1k); `Arc` chỉ là template bất biến, không shared mutable heap.
    pub fn spawn_worker_copy<F>(
        &mut self,
        payload: Arc<SendValue>,
        f: F,
    ) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx, SendValue) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn(move |ctx| {
            let payload = (*payload).clone();
            f(ctx, payload)
        })
    }

    /// **Shared read-only**: task giữ `Arc<SendValue>` và chỉ nhận `&SendValue`; **không**
    /// deep clone payload. Producer clone `Arc` O(1), worker đọc trực tiếp.
    ///
    /// Chỉ dùng khi task **không sửa** payload (kiểu `&` chặn mutation). Cần sở hữu/sửa
    /// payload thì dùng [`Self::spawn_worker_copy`] hoặc [`Self::spawn_with_payload`].
    /// Opt-in/test-only; W=1/W>1, print merge, cross-write, budget giữ nguyên như `spawn`.
    pub fn spawn_shared<F>(
        &mut self,
        payload: Arc<SendValue>,
        f: F,
    ) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn(move |ctx| f(ctx, &payload))
    }

    /// Như [`Self::spawn_shared`] nhưng cờ huỷ **true từ trước khi gửi job**.
    pub fn spawn_shared_cancelled<F>(
        &mut self,
        payload: Arc<SendValue>,
        f: F,
    ) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn_shared_with_cancel(payload, f, true)
    }

    /// Như [`Self::spawn_shared`] với cờ huỷ chọn trước (`cancelled=true` ⇒ pre-cancelled).
    pub fn spawn_shared_with_cancel<F>(
        &mut self,
        payload: Arc<SendValue>,
        f: F,
        cancelled: bool,
    ) -> Result<TaskHandle, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send + 'static,
    {
        self.spawn_inner(move |ctx| f(ctx, &payload), cancelled)
    }

    /// **ST-291 (P2b-1)** — gửi **một lô** job `(payload, f)` xuống pool, trả handle đúng thứ tự.
    ///
    /// Gộp phần dựng `TaskId`/cờ huỷ/board vào một lần gọi; W=1 vẫn chạy tuần tự trong
    /// `join` như `spawn_shared`. Không đổi semantics `spawn`: task chưa chạy tới `await`/join.
    pub fn spawn_shared_batch<F>(
        &mut self,
        jobs: Vec<(Arc<SendValue>, F)>,
        cancelled: bool,
    ) -> Result<Vec<TaskHandle>, WorkerError>
    where
        F: FnOnce(&mut WorkerCtx, &SendValue) -> Result<SendValue, String> + Send + 'static,
    {
        let mut handles = Vec::with_capacity(jobs.len());
        match self.mode.as_mut().ok_or(WorkerError::ChannelClosed)? {
            PoolMode::Sequential { queue } => {
                let wait_board = self.wait_board.clone();
                for (payload, f) in jobs {
                    let id = TaskId(self.next);
                    self.next += 1;
                    let cancel = Arc::new(AtomicBool::new(cancelled));
                    let wb = wait_board.clone();
                    let task: TaskFn = Box::new(move |ctx| {
                        let result = f(ctx, &payload);
                        if let Some(board) = &wb {
                            board.publish(id, result.clone());
                        }
                        result
                    });
                    queue.push(QueuedTask {
                        id,
                        cancel: Arc::clone(&cancel),
                        task,
                    });
                    self.order.push(id);
                    handles.push(TaskHandle { id, cancel });
                }
            }
            PoolMode::Parallel { txs, .. } => {
                let n = txs.len();
                let wait_board = self.wait_board.clone();
                let outcome_board = self.outcome_board.clone();
                // **ST-291 (P2b-1)**: gộp tối đa `BATCH_CHUNK` task/message, xoay vòng kênh
                // để cân tải; N task cực nhỏ chỉ tốn ~N/BATCH_CHUNK lần gửi channel.
                let mut chunks: Vec<Vec<BatchItem>> = (0..n).map(|_| Vec::new()).collect();
                for (i, (payload, f)) in jobs.into_iter().enumerate() {
                    let id = TaskId(self.next);
                    self.next += 1;
                    let cancel = Arc::new(AtomicBool::new(cancelled));
                    let f: SharedFn = Box::new(f);
                    let chunk = (i / BATCH_CHUNK) % n;
                    chunks[chunk].push(BatchItem {
                        id,
                        cancel: Arc::clone(&cancel),
                        payload,
                        f,
                    });
                    self.order.push(id);
                    handles.push(TaskHandle { id, cancel });
                }
                for (ci, items) in chunks.into_iter().enumerate() {
                    if items.is_empty() {
                        continue;
                    }
                    txs[ci]
                        .send(Job {
                            kind: JobKind::Batch(items),
                            snapshot: self.root.clone(),
                            wait_board: wait_board.clone(),
                            outcome_board: outcome_board.clone(),
                        })
                        .map_err(|_| WorkerError::ChannelClosed)?;
                }
            }
        }
        Ok(handles)
    }

    /// Join tất cả task; merge output theo **spawn order**. W>1 có ghi chéo ⇒ `CrossWrite`.
    pub fn join(mut self) -> Result<PoolResult, WorkerError> {
        let mode = self.mode.take().ok_or(WorkerError::ChannelClosed)?;
        let mut outcomes = Vec::with_capacity(self.order.len());
        let mut env = self.root.clone();
        match mode {
            PoolMode::Sequential { queue } => {
                for q in queue {
                    if q.cancel.load(Ordering::Acquire) {
                        outcomes.push(cancelled_outcome(q.id));
                        continue;
                    }
                    let mut ctx = WorkerCtx::new(
                        env.clone(),
                        true,
                        Arc::clone(&q.cancel),
                        self.budget.clone(),
                    );
                    let result = (q.task)(&mut ctx);
                    env = ctx.globals().clone();
                    outcomes.push(ctx.into_outcome(q.id, result));
                }
            }
            PoolMode::Parallel {
                txs,
                mut board,
                workers,
            } => {
                drop(txs);
                for id in &self.order {
                    outcomes.push(board.wait(*id)?);
                }
                for h in workers {
                    h.join().map_err(|_| WorkerError::ChannelClosed)?;
                }
            }
        }

        // Công bố nốt kết quả cho proxy (W=1, task huỷ trước khi chạy, hoặc task đã xong
        // trước khi caller kịp chờ); `publish` là first-wins nên không ghi đè.
        if let Some(board) = &self.wait_board {
            for o in &outcomes {
                board.publish(o.id, o.result.clone());
            }
        }
        if let Some(board) = &self.outcome_board {
            for o in &outcomes {
                board.publish(o.clone());
            }
        }

        // Parallel: bất kỳ dirty nào ⇒ từ chối, main env không đổi.
        if self.workers > 1 {
            let mut vars = BTreeSet::new();
            let mut tasks = BTreeSet::new();
            for o in &outcomes {
                if !o.dirty.is_empty() {
                    tasks.insert(o.id);
                    vars.extend(o.dirty.iter().cloned());
                }
            }
            if !vars.is_empty() {
                return Err(WorkerError::CrossWrite {
                    vars: vars.into_iter().collect(),
                    tasks: tasks.into_iter().collect(),
                });
            }
        }

        let output = outcomes.iter().map(|o| o.output.as_str()).collect();
        Ok(PoolResult {
            output,
            outcomes,
            env,
            budget: self.budget.as_ref().map(|b| b.snapshot()),
        })
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        if let Some(PoolMode::Parallel {
            txs,
            workers,
            board,
        }) = self.mode.take()
        {
            drop(txs);
            drop(board);
            for h in workers {
                let _ = h.join();
            }
        }
        // Không để waiter treo nếu pool bị drop mà chưa join.
        if let Some(board) = &self.wait_board {
            board.close();
        }
        if let Some(board) = &self.outcome_board {
            board.close();
        }
    }
}

fn cancelled_outcome(id: TaskId) -> TaskOutcome {
    TaskOutcome {
        id,
        result: Err("tác vụ bị huỷ".to_string()),
        output: String::new(),
        cancelled: true,
        dirty: Vec::new(),
        budget_used: 0,
    }
}

fn worker_loop(
    rx: mpsc::Receiver<Job>,
    out_tx: mpsc::Sender<TaskOutcome>,
    budget: Option<Arc<SharedBudget>>,
) {
    while let Ok(job) = rx.recv() {
        let Job {
            kind,
            snapshot,
            wait_board,
            outcome_board,
        } = job;
        match kind {
            JobKind::One { id, cancel, task } => {
                if cancel.load(Ordering::Acquire) {
                    let outcome = cancelled_outcome(id);
                    if let Some(board) = &wait_board {
                        board.publish(id, outcome.result.clone());
                    }
                    if let Some(board) = &outcome_board {
                        board.publish(outcome.clone());
                    }
                    if out_tx.send(outcome).is_err() {
                        break;
                    }
                    continue;
                }
                let mut ctx = WorkerCtx::new(snapshot, false, cancel, budget.clone());
                let result = task(&mut ctx);
                let outcome = ctx.into_outcome(id, result);
                if let Some(board) = &outcome_board {
                    board.publish(outcome.clone());
                }
                if out_tx.send(outcome).is_err() {
                    break;
                }
            }
            // **ST-291 (P2b-1)**: chạy cả lô task con trên worker này; mỗi task vẫn có
            // `WorkerCtx`/cờ huỷ/outcome riêng → semantics từng task không đổi.
            JobKind::Batch(items) => {
                for item in items {
                    let BatchItem {
                        id,
                        cancel,
                        payload,
                        f,
                    } = item;
                    if cancel.load(Ordering::Acquire) {
                        let outcome = cancelled_outcome(id);
                        if let Some(board) = &wait_board {
                            board.publish(id, outcome.result.clone());
                        }
                        if let Some(board) = &outcome_board {
                            board.publish(outcome.clone());
                        }
                        if out_tx.send(outcome).is_err() {
                            return;
                        }
                        continue;
                    }
                    let mut ctx = WorkerCtx::new(snapshot.clone(), false, cancel, budget.clone());
                    let result = f(&mut ctx, &payload);
                    if let Some(board) = &wait_board {
                        board.publish(id, result.clone());
                    }
                    let outcome = ctx.into_outcome(id, result);
                    if let Some(board) = &outcome_board {
                        board.publish(outcome.clone());
                    }
                    if out_tx.send(outcome).is_err() {
                        return;
                    }
                }
            }
        }
    }
}
