// SPDX-License-Identifier: MIT OR Apache-2.0
// **VS3-3b (W5, VD-271 B1–B7)** — worker pool **opt-in/test-only**: task affine + heap theo
// luồng, `TaskId`/proxy completion, snapshot Env, print buffer merge theo **spawn order**, budget
// atomic + chunk reservation.
//
// File này được `include!` tạm từ `value.rs` (một dòng, giữ R1) để **không sửa `rt.rs`** (W1 sở
// hữu). Khi W1 tích hợp, chuyển thành `rt/worker.rs` + `mod worker;` trong `rt.rs`, bỏ include.
//
// **Không bật mặc định.** `WorkerPool` chỉ được tạo tường minh bởi test/opt-in; runtime
// `vis run` vẫn dùng `Scheduler` một luồng. `Value::Task(Rc<Task>)` **không đổi** (B5) — `TaskId`
// là handle mới cho worker pool; W1 sẽ thay handle trong `Value` khi tích hợp.
//
// Task chạy dưới dạng closure `FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send`;
// `SendValue` là dạng Sendable của dữ liệu (không có `Rc`/`RefCell`). Đây là mô hình worker
// thật nhưng chưa nối vào evaluator ViScript (W1 hook).

#[doc(hidden)]
pub mod worker {
    use std::collections::{BTreeSet, HashMap};
    use std::fmt;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc};

    use crate::host::TaskOutput;
    use crate::rt::fee::{BudgetError, BudgetLease, BudgetSnapshot, SharedBudget};

    /// Handle **opaque** của một task (u64); không phải `Rc<Task>`/con trỏ heap.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TaskId(u64);

    impl TaskId {
        pub const fn from_raw(raw: u64) -> Self {
            Self(raw)
        }

        pub fn as_u64(self) -> u64 {
            self.0
        }
    }

    impl fmt::Display for TaskId {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "#{}", self.0)
        }
    }

    /// Env top-level dạng Sendable (không `Rc`/`RefCell`), dùng cho snapshot worker.
    pub type EnvMap = HashMap<String, SendValue>;

    /// Dữ liệu **gửi được** qua biên worker (tập con của `Value` theo VD-178 QĐ-1).
    /// `Task`/`Chan`/`Fn`/`Vm`/`Cap` **không** có ở đây — chúng không Sendable.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    pub enum SendValue {
        Unit,
        Int(i64),
        Float(f64),
        Size(i64),
        Duration(i64),
        Percent(f64),
        Str(String),
        Bool(bool),
        Bytes(Vec<u8>),
        Char(char),
        Path(String),
        List(Vec<SendValue>),
        Map(Vec<(SendValue, SendValue)>),
        Record(String, Vec<(String, SendValue)>),
        Enum(String, String, Vec<SendValue>),
    }

    impl SendValue {
        /// Chuyển lại `Value` trong heap của worker/main. Tên enum được nội bộ hoá lại.
        pub fn to_value(&self) -> crate::value::Value {
            match self {
                SendValue::Unit => crate::value::Value::Unit,
                SendValue::Int(i) => crate::value::Value::Int(*i),
                SendValue::Float(f) => crate::value::Value::Float(*f),
                SendValue::Size(i) => crate::value::Value::Size(*i),
                SendValue::Duration(i) => crate::value::Value::Duration(*i),
                SendValue::Percent(f) => crate::value::Value::Percent(*f),
                SendValue::Str(s) => crate::value::Value::Str(s.clone()),
                SendValue::Bool(b) => crate::value::Value::Bool(*b),
                SendValue::Bytes(b) => crate::value::Value::Bytes(b.clone()),
                SendValue::Char(c) => crate::value::Value::Char(*c),
                SendValue::Path(s) => crate::value::Value::Path(s.clone()),
                SendValue::List(xs) => {
                    crate::value::Value::List(xs.iter().map(SendValue::to_value).collect())
                }
                SendValue::Map(kv) => crate::value::Value::Map(
                    kv.iter()
                        .map(|(k, v)| (k.to_value(), v.to_value()))
                        .collect(),
                ),
                SendValue::Record(n, fs) => crate::value::Value::Record(
                    n.clone(),
                    fs.iter().map(|(k, v)| (k.clone(), v.to_value())).collect(),
                ),
                SendValue::Enum(a, b, args) => crate::value::Value::Enum(
                    crate::value::Name::new(a),
                    crate::value::Name::new(b),
                    args.iter().map(SendValue::to_value).collect(),
                ),
            }
        }
    }

    impl crate::value::Value {
        /// Chuyển sang dạng Sendable; closure/task/channel/capability ⇒ lỗi (VS0508).
        #[doc(hidden)]
        pub fn to_send(&self) -> Result<SendValue, String> {
            match self {
                crate::value::Value::Unit => Ok(SendValue::Unit),
                crate::value::Value::Int(i) => Ok(SendValue::Int(*i)),
                crate::value::Value::Float(f) => Ok(SendValue::Float(*f)),
                crate::value::Value::Size(i) => Ok(SendValue::Size(*i)),
                crate::value::Value::Duration(i) => Ok(SendValue::Duration(*i)),
                crate::value::Value::Percent(f) => Ok(SendValue::Percent(*f)),
                crate::value::Value::Str(s) => Ok(SendValue::Str(s.clone())),
                crate::value::Value::Bool(b) => Ok(SendValue::Bool(*b)),
                crate::value::Value::Bytes(b) => Ok(SendValue::Bytes(b.clone())),
                crate::value::Value::Char(c) => Ok(SendValue::Char(*c)),
                crate::value::Value::Path(s) => Ok(SendValue::Path(s.clone())),
                crate::value::Value::List(xs) => Ok(SendValue::List(
                    xs.iter()
                        .map(crate::value::Value::to_send)
                        .collect::<Result<_, _>>()?,
                )),
                crate::value::Value::Map(kv) => Ok(SendValue::Map(
                    kv.iter()
                        .map(|(k, v)| Ok((k.to_send()?, v.to_send()?)))
                        .collect::<Result<_, String>>()?,
                )),
                crate::value::Value::Record(n, fs) => Ok(SendValue::Record(
                    n.clone(),
                    fs.iter()
                        .map(|(k, v)| Ok((k.clone(), v.to_send()?)))
                        .collect::<Result<_, String>>()?,
                )),
                crate::value::Value::Enum(a, b, args) => Ok(SendValue::Enum(
                    a.as_ref().to_string(),
                    b.as_ref().to_string(),
                    args.iter()
                        .map(crate::value::Value::to_send)
                        .collect::<Result<_, _>>()?,
                )),
                crate::value::Value::Range(_, _, _) => {
                    Err("`Range` nội bộ không gửi được sang worker (VS0508)".to_string())
                }
                crate::value::Value::Builtin(_)
                | crate::value::Value::Fn(_)
                | crate::value::Value::Vm(_)
                | crate::value::Value::Task(_)
                | crate::value::Value::Chan(_)
                | crate::value::Value::Cap { .. } => {
                    Err("giá trị không Sendable (VS0508): closure/handle/Task/cap".to_string())
                }
            }
        }
    }

    /// Lỗi của worker pool (không phải lỗi task; task trả `Err(String)` trong outcome).
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum WorkerError {
        InvalidWorkers,
        ChannelClosed,
        /// Có task ghi top-level `var` khi W>1 (không cho ghi chéo worker).
        CrossWrite {
            vars: Vec<String>,
            tasks: Vec<TaskId>,
        },
    }

    /// Kết quả một task, giữ output riêng để merge theo spawn order.
    #[doc(hidden)]
    #[derive(Debug)]
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
        fn new(
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
                        format!("vượt ngân sách worker (còn {remaining})")
                    }
                }),
                None => Ok(()),
            }
        }

        pub fn budget_used(&self) -> i64 {
            self.budget.as_ref().map(BudgetLease::used).unwrap_or(0)
        }

        fn globals(&self) -> &EnvMap {
            &self.globals
        }

        fn into_outcome(self, id: TaskId, result: Result<SendValue, String>) -> TaskOutcome {
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

    type TaskFn = Box<dyn FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send>;

    struct QueuedTask {
        id: TaskId,
        cancel: Arc<AtomicBool>,
        task: TaskFn,
    }

    struct Job {
        id: TaskId,
        cancel: Arc<AtomicBool>,
        snapshot: EnvMap,
        task: TaskFn,
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

        /// Spawn một task; closure phải `Send` và không được giữ `Value`/`Rc`.
        pub fn spawn<F>(&mut self, f: F) -> Result<TaskHandle, WorkerError>
        where
            F: FnOnce(&mut WorkerCtx) -> Result<SendValue, String> + Send + 'static,
        {
            let id = TaskId(self.next);
            self.next += 1;
            let cancel = Arc::new(AtomicBool::new(false));
            let task: TaskFn = Box::new(f);
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
                            id,
                            cancel: Arc::clone(&cancel),
                            snapshot,
                            task,
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
                id,
                cancel,
                snapshot,
                task,
            } = job;
            if cancel.load(Ordering::Acquire) {
                if out_tx.send(cancelled_outcome(id)).is_err() {
                    break;
                }
                continue;
            }
            let mut ctx = WorkerCtx::new(snapshot, false, cancel, budget.clone());
            let result = task(&mut ctx);
            if out_tx.send(ctx.into_outcome(id, result)).is_err() {
                break;
            }
        }
    }
}
