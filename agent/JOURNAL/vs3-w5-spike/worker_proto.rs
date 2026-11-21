// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3b prototype (W5) — KHÔNG production.**
//!
//! Chứng minh 4 điểm A0 đã chốt trong `agent/DECISIONS.md` VD-271 (bổ sung) và
//! `agent/VS3-TIEN-DO.md` §3b:
//!
//! 1. **TaskId/proxy completion**: handle chỉ là `TaskId(u64)`; worker gửi
//!    `Completion` qua channel; không có `Rc<Task>`/`Arc<Mutex<Task>>` chia sẻ heap.
//! 2. **Env snapshot theo worker**: mỗi worker có heap/globals riêng; mỗi `spawn`
//!    nhận snapshot top-level `var`; task chỉ ghi vào env cục bộ. Ghi top-level
//!    `var` bị phát hiện qua `dirty`; chế độ parallel từ chối rõ.
//! 3. **Print buffer + merge tất định**: mỗi task có `String` output riêng; main
//!    merge theo **thứ tự spawn**; W=1/W=2/W=4 cho cùng byte trên chương trình mẫu.
//! 4. **Ngân sách atomic dùng chung**: `AtomicI64` CAS; nhiều task trừ cùng pool,
//!    không reset; đo W=1/2/4, so với budget local (lower bound) và ca cạn budget.
//!
//! Chạy:
//! ```text
//! rustc -O agent/JOURNAL/vs3-w5-spike/worker_proto.rs -o /tmp/vs3w5worker
//! /tmp/vs3w5worker
//! /tmp/vs3w5worker --child 4      # RSS trong tiến trình con
//! ```
//!
//! Không thuộc workspace, không crate ngoài, không đụng `crates/**`/`Cargo.toml`.
//! Đây là mô hình thu nhỏ; không phải runtime `vis`, không phải `Value` thật.

#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Instant;

// ===========================================================================
// 1. Dữ liệu Sendable tối thiểu (mô phỏng Value sau khi đã copy qua biên)
// ===========================================================================

#[derive(Clone, Debug, PartialEq, Eq)]
enum Data {
    Unit,
    Int(i64),
    Str(String),
    List(Vec<Data>),
}

impl Data {
    fn as_int(&self) -> i64 {
        match self {
            Data::Int(i) => *i,
            _ => -1,
        }
    }

    fn deep_bytes(&self) -> usize {
        match self {
            Data::Unit => 0,
            Data::Int(_) => 8,
            Data::Str(s) => s.len(),
            Data::List(xs) => 8 + xs.iter().map(Data::deep_bytes).sum::<usize>(),
        }
    }
}

type EnvSnapshot = HashMap<String, Data>;

/// Env cấp chương trình của **main**. Worker không giữ `Rc` tới đây.
struct RootEnv {
    vars: EnvSnapshot,
}

impl RootEnv {
    fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    fn set_int(&mut self, name: &str, v: i64) {
        self.vars.insert(name.to_string(), Data::Int(v));
    }

    fn set_var(&mut self, name: &str, v: Data) {
        self.vars.insert(name.to_string(), v);
    }

    fn get_int(&self, name: &str) -> i64 {
        match self.vars.get(name) {
            Some(Data::Int(v)) => *v,
            _ => -1,
        }
    }

    /// Snapshot **tại thời điểm spawn**: deep clone toàn bộ top-level `var`.
    fn snapshot(&self) -> EnvSnapshot {
        self.vars.clone()
    }
}

/// Env cục bộ của task. Ghi top-level `var` vào đây và đánh dấu `dirty`.
struct TaskEnv {
    globals: EnvSnapshot,
    dirty: BTreeSet<String>,
}

impl TaskEnv {
    fn new(snapshot: EnvSnapshot) -> Self {
        Self {
            globals: snapshot,
            dirty: BTreeSet::new(),
        }
    }

    fn get_int(&self, name: &str) -> i64 {
        match self.globals.get(name) {
            Some(Data::Int(v)) => *v,
            _ => -1,
        }
    }

    fn list_len(&self, name: &str) -> usize {
        match self.globals.get(name) {
            Some(Data::List(xs)) => xs.len(),
            _ => 0,
        }
    }

    /// Ghi top-level `var`: chỉ đổi bản sao của task, ghi `dirty` để main kiểm.
    fn write_global_int(&mut self, name: &str, v: i64) {
        self.globals.insert(name.to_string(), Data::Int(v));
        self.dirty.insert(name.to_string());
    }
}

// ===========================================================================
// 2. TaskId + Completion — proxy, không chia sẻ Task
// ===========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TaskId(u64);

impl TaskId {
    fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug)]
enum Trap {
    CrossWrite { var: String, task: TaskId },
    BudgetExhausted { remaining: i64 },
    Custom(String),
}

#[derive(Clone, Debug)]
enum TaskKind {
    Safe { idx: usize },
    CrossWrite { idx: usize, var: String },
    BudgetOnly,
}

struct TaskSpec {
    kind: TaskKind,
    /// Snapshot top-level `var` tại lúc `spawn`.
    snapshot: EnvSnapshot,
    charge_rounds: usize,
    charge_weight: i64,
    cpu_iters: u64,
}

struct Job {
    id: TaskId,
    spec: TaskSpec,
}

/// Kết quả worker gửi về main. Mọi trường đều Send; không tham chiếu heap worker.
struct Completion {
    id: TaskId,
    worker: usize,
    result: Result<Data, Trap>,
    out: String,
    dirty: Vec<(String, Data)>,
    charged: i64,
    budget_exhausted: bool,
}

// ===========================================================================
// 3. Ngân sách atomic dùng chung
// ===========================================================================

struct SharedBudget {
    limit: i64,
    remaining: AtomicI64,
    used: AtomicI64,
    exhausted: AtomicBool,
    failed_charges: AtomicU64,
}

impl SharedBudget {
    fn new(limit: i64) -> Self {
        assert!(limit >= 0);
        Self {
            limit,
            remaining: AtomicI64::new(limit),
            used: AtomicI64::new(0),
            exhausted: AtomicBool::new(false),
            failed_charges: AtomicU64::new(0),
        }
    }

    /// Trừ **một pool chung**. CAS bảo đảm không âm; không có đường reset.
    fn charge(&self, weight: i64) -> Result<i64, i64> {
        assert!(weight > 0);
        let mut cur = self.remaining.load(Ordering::Relaxed);
        loop {
            if cur < weight {
                self.exhausted.store(true, Ordering::Relaxed);
                self.failed_charges.fetch_add(1, Ordering::Relaxed);
                return Err(cur);
            }
            match self.remaining.compare_exchange_weak(
                cur,
                cur - weight,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.used.fetch_add(weight, Ordering::Relaxed);
                    return Ok(cur - weight);
                }
                Err(actual) => cur = actual,
            }
        }
    }

    fn snapshot(&self) -> BudgetSnapshot {
        BudgetSnapshot {
            limit: self.limit,
            remaining: self.remaining.load(Ordering::Acquire),
            used: self.used.load(Ordering::Acquire),
            exhausted: self.exhausted.load(Ordering::Acquire),
            failed_charges: self.failed_charges.load(Ordering::Acquire),
        }
    }

    fn remaining(&self) -> i64 {
        self.remaining.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy, Debug)]
struct BudgetSnapshot {
    limit: i64,
    remaining: i64,
    used: i64,
    exhausted: bool,
    failed_charges: u64,
}

// ===========================================================================
// 4. Worker pool: heap riêng, queue riêng, completion channel
// ===========================================================================

struct WorkerHeap {
    id: usize,
    /// Globals của worker — deep clone từ root lúc tạo pool; chỉ worker này thấy.
    globals: EnvSnapshot,
}

fn execute_job(job: Job, worker: &WorkerHeap, budget: &SharedBudget) -> Completion {
    let Job { id, spec } = job;
    let TaskSpec {
        kind,
        snapshot,
        charge_rounds,
        charge_weight,
        cpu_iters,
    } = spec;

    let mut env = TaskEnv::new(snapshot);
    let mut out = String::new();
    let mut result = Data::Unit;
    let mut trap = None;

    match kind {
        TaskKind::Safe { idx } => {
            let base = env.get_int("base");
            let data_len = env.list_len("data");
            // Snapshot: task thấy base tại lúc spawn, không thấy main đổi sau đó.
            out.push_str(&format!("t{idx}:base={base}\n"));
            out.push_str(&format!("t{idx}:data_len={data_len}\n"));
            if cpu_iters > 0 {
                black_box_cpu(cpu_iters);
            }
            result = Data::Int(base * 100 + idx as i64);
        }
        TaskKind::CrossWrite { idx, ref var } => {
            let cur = env.get_int(&var);
            env.write_global_int(var, cur + 1);
            out.push_str(&format!("t{idx}:cw-counter={}\n", cur + 1));
            if cpu_iters > 0 {
                black_box_cpu(cpu_iters);
            }
            result = Data::Int(cur + 1);
        }
        TaskKind::BudgetOnly => {
            if cpu_iters > 0 {
                black_box_cpu(cpu_iters);
            }
        }
    }

    let mut charged = 0i64;
    let mut budget_exhausted = false;
    for _ in 0..charge_rounds {
        match budget.charge(charge_weight) {
            Ok(_) => charged += charge_weight,
            Err(_) => {
                budget_exhausted = true;
                break;
            }
        }
    }
    if budget_exhausted && trap.is_none() && !matches!(kind, TaskKind::BudgetOnly) {
        trap = Some(Trap::BudgetExhausted {
            remaining: budget.remaining(),
        });
    }

    let TaskEnv { globals, dirty } = env;
    let dirty: Vec<(String, Data)> = dirty
        .into_iter()
        .map(|k| {
            let v = globals.get(&k).cloned().unwrap_or(Data::Unit);
            (k, v)
        })
        .collect();

    Completion {
        id,
        worker: worker.id,
        result: match trap {
            Some(t) => Err(t),
            None => Ok(result),
        },
        out,
        dirty,
        charged,
        budget_exhausted,
    }
}

fn worker_loop(
    worker: WorkerHeap,
    rx: mpsc::Receiver<Job>,
    out_tx: mpsc::Sender<Completion>,
    budget: Arc<SharedBudget>,
) {
    while let Ok(job) = rx.recv() {
        let completion = execute_job(job, &worker, &budget);
        if out_tx.send(completion).is_err() {
            break;
        }
    }
}

struct CompletionBoard {
    rx: mpsc::Receiver<Completion>,
    pending: HashMap<TaskId, Completion>,
    arrival: Vec<TaskId>,
}

impl CompletionBoard {
    fn wait(&mut self, id: TaskId) -> Completion {
        if let Some(c) = self.pending.remove(&id) {
            return c;
        }
        loop {
            let c = self
                .rx
                .recv()
                .expect("completion channel đóng trước khi đủ handle");
            self.arrival.push(c.id);
            if c.id == id {
                return c;
            }
            self.pending.insert(c.id, c);
        }
    }
}

struct WorkerPool {
    worker_txs: Vec<mpsc::SyncSender<Job>>,
    board: CompletionBoard,
    handles: Vec<TaskId>,
    next_id: u64,
    budget: Arc<SharedBudget>,
    workers: Vec<std::thread::JoinHandle<()>>,
}

impl WorkerPool {
    fn new(w: usize, root: &RootEnv, budget: Arc<SharedBudget>, heap_pad: usize) -> Self {
        assert!(w >= 1);
        let (out_tx, out_rx) = mpsc::channel::<Completion>();
        let mut worker_txs = Vec::with_capacity(w);
        let mut workers = Vec::with_capacity(w);
        for id in 0..w {
            let (tx, rx) = mpsc::sync_channel::<Job>(64);
            worker_txs.push(tx);
            let out_tx = out_tx.clone();
            let mut globals = root.snapshot();
            if heap_pad > 0 {
                // Heap riêng của worker: cố ý cấp phát thêm để đo RSS theo W.
                globals.insert(
                    "__heap_pad".to_string(),
                    Data::List((0..heap_pad).map(|i| Data::Int(i as i64)).collect()),
                );
            }
            let heap = WorkerHeap { id, globals };
            let budget = Arc::clone(&budget);
            workers.push(std::thread::spawn(move || {
                worker_loop(heap, rx, out_tx, budget)
            }));
        }
        drop(out_tx);
        Self {
            worker_txs,
            board: CompletionBoard {
                rx: out_rx,
                pending: HashMap::new(),
                arrival: Vec::new(),
            },
            handles: Vec::new(),
            next_id: 0,
            budget,
            workers,
        }
    }

    fn spawn(&mut self, spec: TaskSpec) -> TaskId {
        self.next_id += 1;
        let id = TaskId(self.next_id);
        let idx = (self.next_id as usize - 1) % self.worker_txs.len();
        self.worker_txs[idx]
            .send(Job { id, spec })
            .expect("worker queue đóng");
        self.handles.push(id);
        id
    }

    fn await_task(&mut self, id: TaskId) -> Completion {
        self.board.wait(id)
    }

    fn await_all(&mut self) -> Vec<Completion> {
        let ids = self.handles.clone();
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            out.push(self.board.wait(id));
        }
        out
    }

    fn arrival(&self) -> Vec<TaskId> {
        self.board.arrival.clone()
    }

    fn handles(&self) -> Vec<TaskId> {
        self.handles.clone()
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.worker_txs.clear();
        for h in self.workers.drain(..) {
            let _ = h.join();
        }
    }
}

// ===========================================================================
// 5. Chương trình mẫu: snapshot, output merge, cross-write
// ===========================================================================

struct RunResult {
    output: String,
    results: Vec<Data>,
    main_base: i64,
    main_counter: i64,
    handles: Vec<TaskId>,
    arrival: Vec<TaskId>,
    dirty: Vec<(TaskId, String, Data)>,
    budget: BudgetSnapshot,
    startup_ms: f64,
    run_ms: f64,
    err: Option<RunError>,
}

#[derive(Clone, Debug)]
enum RunError {
    CrossWrite {
        vars: Vec<String>,
        tasks: Vec<TaskId>,
    },
    Trap(String),
}

fn run_safe(
    w: usize,
    n: usize,
    base_len: usize,
    cpu_iters: u64,
    charges: usize,
) -> RunResult {
    run_safe_pad(w, n, base_len, cpu_iters, charges, 0)
}

fn run_safe_pad(
    w: usize,
    n: usize,
    base_len: usize,
    cpu_iters: u64,
    charges: usize,
    heap_pad: usize,
) -> RunResult {
    let weight = 1i64;
    let limit = (n * charges) as i64 * weight + 10_000;
    let mut root = RootEnv::new();
    root.set_int("base", 10);
    root.set_int("shared_counter", 0);
    root.set_var(
        "data",
        Data::List((0..base_len).map(|i| Data::Int(i as i64)).collect()),
    );

    let budget = Arc::new(SharedBudget::new(limit));
    let t0 = Instant::now();
    let mut pool = WorkerPool::new(w, &root, Arc::clone(&budget), heap_pad);
    let startup_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    for idx in 0..n {
        // Snapshot **tại spawn** — độc lập cho từng task.
        let snapshot = root.snapshot();
        pool.spawn(TaskSpec {
            kind: TaskKind::Safe { idx },
            snapshot,
            charge_rounds: charges,
            charge_weight: weight,
            cpu_iters,
        });
    }
    // Main đổi top-level var **sau khi spawn**: task phải giữ snapshot cũ.
    root.set_int("base", 99);

    let comps = pool.await_all();
    let run_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let handles = pool.handles();
    let arrival = pool.arrival();

    let mut output = String::from("main:start\n");
    for c in &comps {
        output.push_str(&c.out);
    }
    output.push_str("main:end\n");
    output.push_str(&format!("main:base={}\n", root.get_int("base")));
    output.push_str(&format!(
        "main:shared_counter={}\n",
        root.get_int("shared_counter")
    ));
    let vals: Vec<String> = comps
        .iter()
        .map(|c| c.result.as_ref().map(Data::as_int).unwrap_or(-1).to_string())
        .collect();
    output.push_str(&format!("main:results=[{}]\n", vals.join(",")));

    let results: Vec<Data> = comps
        .iter()
        .map(|c| c.result.clone().unwrap_or(Data::Unit))
        .collect();
    let dirty: Vec<(TaskId, String, Data)> = comps
        .iter()
        .flat_map(|c| {
            let id = c.id;
            c.dirty
                .iter()
                .map(move |(var, val)| (id, var.clone(), val.clone()))
        })
        .collect();
    let err = comps.iter().find_map(|c| match &c.result {
        Err(t) => Some(RunError::Trap(format!("{t:?}"))),
        Ok(_) => None,
    });

    RunResult {
        output,
        results,
        main_base: root.get_int("base"),
        main_counter: root.get_int("shared_counter"),
        handles,
        arrival,
        dirty,
        budget: budget.snapshot(),
        startup_ms,
        run_ms,
        err,
    }
}

fn run_cross_write(w: usize, n: usize) -> RunResult {
    let weight = 1i64;
    let charges = 100usize;
    let limit = (n * charges) as i64 * weight + 10_000;
    let mut root = RootEnv::new();
    root.set_int("base", 10);
    root.set_int("shared_counter", 0);

    let budget = Arc::new(SharedBudget::new(limit));
    let t0 = Instant::now();
    let mut pool = WorkerPool::new(w, &root, Arc::clone(&budget), 0);
    let startup_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let mut comps = Vec::with_capacity(n);
    let mut seq_out = String::new();
    if w == 1 {
        // W=1/sequential: mỗi task thấy top-level var **hiện tại**, chạy xong thì
        // main áp dirty — đúng ngữ nghĩa tuần tự hiện tại. Đây là chế độ được phép.
        for idx in 0..n {
            let snapshot = root.snapshot();
            let id = pool.spawn(TaskSpec {
                kind: TaskKind::CrossWrite {
                    idx,
                    var: "shared_counter".to_string(),
                },
                snapshot,
                charge_rounds: charges,
                charge_weight: weight,
                cpu_iters: 0,
            });
            let c = pool.await_task(id);
            for (var, val) in &c.dirty {
                root.set_var(var, val.clone());
            }
            seq_out.push_str(&c.out);
            comps.push(c);
        }
    } else {
        // W>1: mọi task nhận cùng snapshot; task ghi heap cục bộ, main phát hiện dirty.
        for idx in 0..n {
            let snapshot = root.snapshot();
            pool.spawn(TaskSpec {
                kind: TaskKind::CrossWrite {
                    idx,
                    var: "shared_counter".to_string(),
                },
                snapshot,
                charge_rounds: charges,
                charge_weight: weight,
                cpu_iters: 0,
            });
        }
        comps = pool.await_all();
    }
    let run_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let handles = pool.handles();
    let arrival = pool.arrival();

    let dirty: Vec<(TaskId, String, Data)> = comps
        .iter()
        .flat_map(|c| {
            let id = c.id;
            c.dirty
                .iter()
                .map(move |(var, val)| (id, var.clone(), val.clone()))
        })
        .collect();

    let parallel = w > 1;
    let (output, err, main_counter) = if parallel && !dirty.is_empty() {
        // Task chỉ ghi heap của nó; main phát hiện qua completion và từ chối.
        let mut vars: Vec<String> = dirty.iter().map(|(_, v, _)| v.clone()).collect();
        vars.sort();
        vars.dedup();
        let mut tasks: Vec<TaskId> = dirty.iter().map(|(t, _, _)| *t).collect();
        tasks.sort();
        tasks.dedup();
        let msg = format!(
            "error: PROTO_CROSS_WRITE top-level var(s) {:?} written by task(s) {:?}; \
             parallel mode từ chối ghi chéo worker\n",
            vars, tasks
        );
        (
            msg,
            Some(RunError::CrossWrite { vars, tasks }),
            root.get_int("shared_counter"),
        )
    } else {
        let mut output = String::from("main:start\n");
        output.push_str(&seq_out);
        for c in &comps {
            if w > 1 {
                output.push_str(&c.out);
            }
        }
        output.push_str("main:end\n");
        output.push_str(&format!(
            "main:shared_counter={}\n",
            root.get_int("shared_counter")
        ));
        (output, None, root.get_int("shared_counter"))
    };

    RunResult {
        output,
        results: comps
            .iter()
            .map(|c| c.result.clone().unwrap_or(Data::Unit))
            .collect(),
        main_base: root.get_int("base"),
        main_counter,
        handles,
        arrival,
        dirty,
        budget: budget.snapshot(),
        startup_ms,
        run_ms,
        err,
    }
}

// ===========================================================================
// 6. Đo ngân sách: shared atomic vs local (lower bound) vs cạn pool
// ===========================================================================

#[derive(Clone, Copy, Debug)]
struct BudgetRun {
    w: usize,
    ms: f64,
    snapshot: BudgetSnapshot,
    total_attempts: usize,
}

fn run_budget_shared(
    w: usize,
    tasks: usize,
    charges: usize,
    weight: i64,
    limit: i64,
) -> BudgetRun {
    let budget = Arc::new(SharedBudget::new(limit));
    let start = Instant::now();
    let mut hs = Vec::with_capacity(w);
    for k in 0..w {
        let b = Arc::clone(&budget);
        hs.push(std::thread::spawn(move || {
            let mut i = k;
            while i < tasks {
                for _ in 0..charges {
                    if b.charge(weight).is_err() {
                        break;
                    }
                }
                i += w;
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    BudgetRun {
        w,
        ms: start.elapsed().as_secs_f64() * 1000.0,
        snapshot: budget.snapshot(),
        total_attempts: tasks * charges,
    }
}

/// Budget **local** mỗi worker (không atomic) — chỉ là lower bound, KHÔNG phải pool chung.
fn run_budget_local(
    w: usize,
    tasks: usize,
    charges: usize,
    weight: i64,
    total_limit: i64,
) -> BudgetRun {
    let per = total_limit / w as i64;
    let start = Instant::now();
    let mut hs = Vec::with_capacity(w);
    for k in 0..w {
        hs.push(std::thread::spawn(move || {
            let mut remaining = per;
            let mut used = 0i64;
            let mut failed = 0u64;
            let mut i = k;
            while i < tasks {
                for _ in 0..charges {
                    if remaining >= weight {
                        remaining -= weight;
                        used += weight;
                    } else {
                        failed += 1;
                        break;
                    }
                }
                i += w;
            }
            (used, remaining, failed)
        }));
    }
    let mut used = 0;
    let mut remaining = 0;
    let mut failed = 0;
    for h in hs {
        let (u, r, f) = h.join().unwrap();
        used += u;
        remaining += r;
        failed += f;
    }
    BudgetRun {
        w,
        ms: start.elapsed().as_secs_f64() * 1000.0,
        snapshot: BudgetSnapshot {
            limit: per * w as i64,
            remaining,
            used,
            exhausted: failed > 0,
            failed_charges: failed,
        },
        total_attempts: tasks * charges,
    }
}

fn exhaustion_probe(w: usize, tasks: usize, charges: usize, weight: i64, limit: i64) -> BudgetRun {
    let budget = Arc::new(SharedBudget::new(limit));
    let start = Instant::now();
    let mut hs = Vec::with_capacity(w);
    for k in 0..w {
        let b = Arc::clone(&budget);
        hs.push(std::thread::spawn(move || {
            let mut i = k;
            while i < tasks {
                for _ in 0..charges {
                    if b.charge(weight).is_err() {
                        break;
                    }
                }
                i += w;
            }
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    let snap = budget.snapshot();
    // Không reset: charge sau khi cạn vẫn Err, remaining vẫn 0.
    let after_err = budget.charge(weight).is_err();
    let after_remaining = budget.remaining();
    assert!(snap.exhausted, "exhaustion_probe phải cạn pool");
    assert_eq!(snap.remaining, 0, "pool cạn phải còn đúng 0");
    assert_eq!(snap.used, snap.limit, "used phải bằng limit");
    assert!(after_err, "charge sau khi cạn phải Err");
    assert_eq!(after_remaining, 0, "không được reset remaining");
    BudgetRun {
        w,
        ms: start.elapsed().as_secs_f64() * 1000.0,
        snapshot: snap,
        total_attempts: tasks * charges,
    }
}

// ===========================================================================
// 7. CPU work + RSS
// ===========================================================================

#[inline(never)]
fn black_box_cpu(iters: u64) -> u64 {
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    for i in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i ^ 0x5deece66d);
        x ^= x >> 29;
    }
    std::hint::black_box(x)
}

fn status_kb(key: &str) -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        }
    }
    0
}

fn child(w: usize) {
    let r = run_safe_pad(w, 64, 1_000, 50_000, 200, 50_000);
    let rss = status_kb("VmRSS:") as f64 / 1024.0;
    let hwm = status_kb("VmHWM:") as f64 / 1024.0;
    assert_eq!(
        r.budget.used,
        64 * 200,
        "child: budget dùng đúng số lần charge"
    );
    println!(
        "CHILD\t{w}\t{rss:.1}\t{hwm:.1}\t{}\t{}\t{}",
        r.budget.used,
        r.budget.remaining,
        r.output.len()
    );
}

// ===========================================================================
// 8. Main: self-test + đo
// ===========================================================================

fn assert_send<T: Send>() {}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn out_of_order(handles: &[TaskId], arrival: &[TaskId]) -> usize {
    // arrival chỉ chứa các completion đến trước handle được await; so vị trí.
    let mut pos = HashMap::new();
    for (i, id) in arrival.iter().enumerate() {
        pos.insert(*id, i);
    }
    let mut count = 0;
    let mut last = 0usize;
    for h in handles {
        if let Some(p) = pos.get(h) {
            if *p < last {
                count += 1;
            }
            last = *p;
        }
    }
    count
}

fn median_budget(mut runs: Vec<BudgetRun>) -> BudgetRun {
    assert!(!runs.is_empty());
    runs.sort_by(|a, b| a.ms.total_cmp(&b.ms));
    runs[runs.len() / 2]
}

fn run_rss_children() -> Vec<(usize, f64, f64)> {
    let exe = std::env::current_exe().expect("current_exe");
    let mut out = Vec::new();
    for w in [1usize, 2, 4] {
        let child = std::process::Command::new(&exe)
            .arg("--child")
            .arg(w.to_string())
            .output()
            .expect("chạy child RSS");
        assert!(
            child.status.success(),
            "child W={w} lỗi: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let text = String::from_utf8_lossy(&child.stdout);
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("CHILD\t") {
                let p: Vec<&str> = rest.split('\t').collect();
                assert_eq!(p.len(), 6, "dòng CHILD không hợp lệ: {line}");
                let rss: f64 = p[1].parse().unwrap();
                let hwm: f64 = p[2].parse().unwrap();
                out.push((w, rss, hwm));
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--child" {
        child(args[2].parse().expect("W"));
        return;
    }

    assert_send::<Data>();
    assert_send::<TaskSpec>();
    assert_send::<Job>();
    assert_send::<Completion>();
    assert_send::<TaskId>();

    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    println!(
        "ENV\tnproc={nproc}\tarch={}\tos={}",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    println!(
        "TASKID\thandle=TaskId(u64)\tRc<Task>=khong\tArc<Mutex<Task>>=khong\tcompletion=channel"
    );

    // ---- 1+2+3: TaskId/proxy, snapshot, print merge ----
    let s1 = run_safe(1, 8, 1000, 20_000, 100);
    let s2 = run_safe(2, 8, 1000, 20_000, 100);
    let s4 = run_safe(4, 8, 1000, 20_000, 100);
    assert_eq!(s1.output, s2.output, "W=1 vs W=2 output phải giống");
    assert_eq!(s1.output, s4.output, "W=1 vs W=4 output phải giống");
    assert_eq!(s1.results, s2.results);
    assert_eq!(s1.results, s4.results);
    assert_eq!(s4.main_base, 99, "main đổi base sau spawn");
    assert!(
        s4.output.contains("t0:base=10"),
        "task phải thấy snapshot base=10, không thấy 99"
    );
    assert_eq!(s4.main_counter, 0, "safe program không ghi top-level var");
    assert_eq!(s4.budget.used, 8 * 100, "budget dùng đúng");
    assert_eq!(
        s4.budget.used + s4.budget.remaining,
        s4.budget.limit,
        "budget: used + remaining == limit"
    );
    println!("PARITY\tsafe W=1/W=2/W=4\tOK\t{} byte", s4.output.len());
    println!("SNAPSHOT\tmain_base={}\ttask_base=10\tOK", s4.main_base);
    println!(
        "PROXY\thandles={}\tarrived_out_of_order={}\tOK",
        s4.handles.len(),
        out_of_order(&s4.handles, &s4.arrival)
    );
    println!("OUTPUT_BEGIN");
    print!("{}", s4.output);
    println!("OUTPUT_END");

    // ---- cross-write: W=1 cho phép (mô phỏng tuần tự), W>1 từ chối ----
    let cw1 = run_cross_write(1, 8);
    assert!(cw1.err.is_none(), "W=1 sequential phải cho phép");
    assert_eq!(cw1.main_counter, 8, "W=1: dirty áp theo thứ tự spawn");
    assert!(cw1.output.contains("main:shared_counter=8"));
    println!(
        "CROSS_WRITE\tW=1\tallowed\tmain_counter={}\tOK",
        cw1.main_counter
    );
    print!("{}", cw1.output);

    let cw4 = run_cross_write(4, 8);
    assert!(cw4.err.is_some(), "W=4 parallel phải từ chối");
    assert_eq!(cw4.main_counter, 0, "parallel: main không bị ghi chéo");
    assert!(cw4.output.starts_with("error: PROTO_CROSS_WRITE"));
    println!(
        "CROSS_WRITE\tW=4\trefused\tmain_counter={}",
        cw4.main_counter
    );
    print!("{}", cw4.output);

    // ---- 4: budget atomic ----
    let tasks = 64usize;
    let charges = 5_000usize;
    let total_work = (tasks * charges) as i64;
    let mut shared_runs = Vec::new();
    for w in [1usize, 2, 4] {
        let runs: Vec<BudgetRun> = (0..3)
            .map(|_| run_budget_shared(w, tasks, charges, 1, total_work + 10_000))
            .collect();
        for r in &runs {
            assert_eq!(r.snapshot.used, total_work, "shared W={w}: phải dùng đủ");
            assert_eq!(
                r.snapshot.used + r.snapshot.remaining,
                r.snapshot.limit,
                "shared W={w}: used + remaining == limit"
            );
        }
        shared_runs.push(median_budget(runs));
    }
    let local_runs: Vec<BudgetRun> = [1usize, 2, 4]
        .iter()
        .copied()
        .map(|w| {
            let runs: Vec<BudgetRun> = (0..3)
                .map(|_| run_budget_local(w, tasks, charges, 1, total_work + 10_000))
                .collect();
            median_budget(runs)
        })
        .collect();
    println!(
        "BUDGET\tmode\tW\ttasks\tcharges\tms\tns_per_charge\tused\tremaining\texhausted\tfailed"
    );
    for r in &shared_runs {
        println!(
            "BUDGET\tshared\t{}\t{tasks}\t{charges}\t{:.2}\t{:.1}\t{}\t{}\t{}\t{}",
            r.w,
            r.ms,
            r.ms * 1e6 / r.total_attempts as f64,
            r.snapshot.used,
            r.snapshot.remaining,
            r.snapshot.exhausted,
            r.snapshot.failed_charges
        );
    }
    for r in &local_runs {
        println!(
            "BUDGET\tlocal\t{}\t{tasks}\t{charges}\t{:.2}\t{:.1}\t{}\t{}\t{}\t{}",
            r.w,
            r.ms,
            r.ms * 1e6 / r.total_attempts as f64,
            r.snapshot.used,
            r.snapshot.remaining,
            r.snapshot.exhausted,
            r.snapshot.failed_charges
        );
    }
    let ex = exhaustion_probe(4, tasks, charges, 1, total_work / 2);
    println!(
        "BUDGET\texhaust\t{}\t{tasks}\t{charges}\t{:.2}\t{:.1}\t{}\t{}\t{}=no-reset\tfailed={}",
        ex.w,
        ex.ms,
        ex.ms * 1e6 / ex.total_attempts as f64,
        ex.snapshot.used,
        ex.snapshot.remaining,
        ex.snapshot.exhausted,
        ex.snapshot.failed_charges
    );

    // ---- perf W=1/2/4 ----
    println!("PERF\tW\trun_ms_med\tstartup_ms\tspeedup");
    let mut base = 0.0;
    for w in [1usize, 2, 4] {
        let mut runs = Vec::new();
        let mut starts = Vec::new();
        for _ in 0..3 {
            let r = run_safe_pad(w, 64, 1_000, 50_000, 200, 50_000);
            assert_eq!(r.budget.used, 64 * 200);
            assert!(r.err.is_none(), "perf safe không được trap");
            runs.push(r.run_ms);
            starts.push(r.startup_ms);
        }
        let med = median(runs);
        if w == 1 {
            base = med;
        }
        println!(
            "PERF\t{w}\t{med:.2}\t{:.2}\t{:.2}",
            median(starts),
            base / med
        );
    }

    // ---- RSS: tiến trình con sạch ----
    let rss = run_rss_children();
    println!("RSS\tW\tRSS_MB\tHWM_MB");
    for (w, rss_mb, hwm) in rss {
        println!("RSS\t{w}\t{rss_mb:.1}\t{hwm:.1}");
    }

    println!("DONE\tprototype only — not production, not VS3-3b done");
}
