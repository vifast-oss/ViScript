// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-3a (W5) — spike worker pool/heap: benchmark độc lập.**
//!
//! Không thuộc workspace, không thêm crate, không đụng `Cargo.toml`. Chạy bằng:
//!
//! ```text
//! rustc -O agent/JOURNAL/vs3-w5-spike/bench.rs -o /tmp/vs3w5bench
//! /tmp/vs3w5bench
//! ```
//!
//! Mục đích: đo **chi phí dữ liệu** (deep copy list/map/record/enum, move) và
//! **chi phí đồng bộ** (spawn + channel 2–4 luồng) để trả lời câu hỏi của VS3-3a:
//! mô hình “task affine + heap theo luồng + copy/move tại biên” có đủ rẻ để làm
//! VS3-3b không, hay phải chuyển sang `Arc`/COW, hay dừng ở single-thread.
//!
//! **Đây là bản mô phỏng, không phải `Value` thật.** Hình dạng biến thể bám
//! `crates/vis-interp/src/value.rs:319-374`; `size_of::<Value>()` được in ra và
//! kỳ vọng **64 byte** (VD-267). Các số đo chỉ để so sánh tương đối.
//!
//! Ghi chú kỹ thuật:
//! - `Value` dùng `Rc`/`RefCell` ⇒ `!Send` (mô phỏng heap theo luồng hiện tại).
//! - `Wire` là dạng “đã sao chép sâu, gửi được” (Send) — mô phỏng payload qua
//!   biên worker/channel theo VD-160/VD-178 QĐ-1.
//! - `ArcValue` là phương án (b): `Arc` + COW, đo clone O(1) và COW write O(n).
//! - RSS đọc từ `/proc/self/status` trong **tiến trình con** (mỗi cảnh báo một
//!   tiến trình sạch) để tránh allocator giữ bộ nhớ giữa các cảnh báo.

#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hint::black_box;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Barrier};
use std::time::Instant;

// ===========================================================================
// 1. Bản mô phỏng `Value` hiện tại (Rc, !Send)
// ===========================================================================

type Env = Rc<RefCell<HashMap<u32, Value>>>;

struct ClosureStub {
    name: String,
    env: Env,
}

struct VmFuncStub {
    idx: u32,
    captured: Vec<Value>,
}

struct TaskStub {
    id: u32,
    args: Vec<Value>,
    ten: String,
    kq: RefCell<Option<Result<Value, String>>>,
}

struct ChanStub {
    cap: usize,
    buf: VecDeque<Value>,
    closed: bool,
}

#[derive(Clone)]
enum Value {
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
    List(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Record(String, Vec<(String, Value)>),
    Enum(Rc<str>, Rc<str>, Vec<Value>),
    Range(i64, i64, bool),
    Builtin(&'static str),
    Fn(Rc<ClosureStub>),
    Vm(Rc<VmFuncStub>),
    Task(Rc<TaskStub>),
    Chan(Rc<RefCell<ChanStub>>),
    Cap {
        kind: String,
        root: String,
        write: bool,
    },
}

impl Default for Value {
    fn default() -> Self {
        Value::Unit
    }
}

// Bảng tên thread-local — bản đơn giản của `Name`/`ENUM_NAMES` (value.rs:57-133).
thread_local! {
    static NAMES: RefCell<HashSet<Rc<str>>> = RefCell::new(HashSet::new());
}

fn name(s: &str) -> Rc<str> {
    NAMES.with(|set| {
        let mut set = set.borrow_mut();
        if let Some(n) = set.get(s) {
            return Rc::clone(n);
        }
        let n: Rc<str> = Rc::from(s);
        set.insert(Rc::clone(&n));
        n
    })
}

// ===========================================================================
// 2. `Wire` — payload Sendable (deep copy tại biên) và `ArcValue` — phương án Arc/COW
// ===========================================================================

#[derive(Clone)]
enum Wire {
    Unit,
    Int(i64),
    Float(f64),
    Size(i64),
    Duration(i64),
    Percent(f64),
    Str(Box<str>),
    Bool(bool),
    Bytes(Vec<u8>),
    Char(char),
    Path(Box<str>),
    List(Vec<Wire>),
    Map(Vec<(Wire, Wire)>),
    Record(Box<str>, Vec<(Box<str>, Wire)>),
    Enum(Box<str>, Box<str>, Vec<Wire>),
    Range(i64, i64, bool),
}

#[derive(Clone)]
enum ArcValue {
    Unit,
    Int(i64),
    Float(f64),
    Size(i64),
    Duration(i64),
    Percent(f64),
    Str(Arc<str>),
    Bool(bool),
    Bytes(Arc<[u8]>),
    Char(char),
    Path(Arc<str>),
    List(Arc<Vec<ArcValue>>),
    Map(Arc<Vec<(ArcValue, ArcValue)>>),
    Record(Arc<str>, Arc<Vec<(Arc<str>, ArcValue)>>),
    Enum(Arc<str>, Arc<str>, Arc<Vec<ArcValue>>),
    Range(i64, i64, bool),
}

fn assert_send_sync<T: Send + Sync>() {}

// ===========================================================================
// 3. Chuyển đổi Value ↔ Wire; thống kê cấp phát logic
// ===========================================================================

fn to_wire(v: &Value) -> Wire {
    match v {
        Value::Unit => Wire::Unit,
        Value::Int(i) => Wire::Int(*i),
        Value::Float(f) => Wire::Float(*f),
        Value::Size(i) => Wire::Size(*i),
        Value::Duration(i) => Wire::Duration(*i),
        Value::Percent(f) => Wire::Percent(*f),
        Value::Str(s) => Wire::Str(s.as_str().into()),
        Value::Bool(b) => Wire::Bool(*b),
        Value::Bytes(b) => Wire::Bytes(b.clone()),
        Value::Char(c) => Wire::Char(*c),
        Value::Path(s) => Wire::Path(s.as_str().into()),
        Value::List(xs) => Wire::List(xs.iter().map(to_wire).collect()),
        Value::Map(kv) => Wire::Map(kv.iter().map(|(k, v)| (to_wire(k), to_wire(v))).collect()),
        Value::Record(n, fs) => Wire::Record(
            n.as_str().into(),
            fs.iter()
                .map(|(k, v)| (k.as_str().into(), to_wire(v)))
                .collect(),
        ),
        Value::Enum(a, b, args) => Wire::Enum(
            a.as_ref().into(),
            b.as_ref().into(),
            args.iter().map(to_wire).collect(),
        ),
        Value::Range(a, b, c) => Wire::Range(*a, *b, *c),
        // VD-178 QĐ-1: closure/handle/Task/Cap không gửi được ⇒ VS0508.
        Value::Builtin(_)
        | Value::Fn(_)
        | Value::Vm(_)
        | Value::Task(_)
        | Value::Chan(_)
        | Value::Cap { .. } => panic!("payload không Sendable (mô phỏng VS0508)"),
    }
}

fn wire_to_value(w: &Wire) -> Value {
    match w {
        Wire::Unit => Value::Unit,
        Wire::Int(i) => Value::Int(*i),
        Wire::Float(f) => Value::Float(*f),
        Wire::Size(i) => Value::Size(*i),
        Wire::Duration(i) => Value::Duration(*i),
        Wire::Percent(f) => Value::Percent(*f),
        Wire::Str(s) => Value::Str(s.to_string()),
        Wire::Bool(b) => Value::Bool(*b),
        Wire::Bytes(b) => Value::Bytes(b.clone()),
        Wire::Char(c) => Value::Char(*c),
        Wire::Path(s) => Value::Path(s.to_string()),
        Wire::List(xs) => Value::List(xs.iter().map(wire_to_value).collect()),
        Wire::Map(kv) => Value::Map(
            kv.iter()
                .map(|(k, v)| (wire_to_value(k), wire_to_value(v)))
                .collect(),
        ),
        Wire::Record(n, fs) => Value::Record(
            n.to_string(),
            fs.iter()
                .map(|(k, v)| (k.to_string(), wire_to_value(v)))
                .collect(),
        ),
        Wire::Enum(a, b, args) => Value::Enum(
            name(a.as_ref()),
            name(b.as_ref()),
            args.iter().map(wire_to_value).collect(),
        ),
        Wire::Range(a, b, c) => Value::Range(*a, *b, *c),
    }
}

/// Chuyển **tiêu thụ** `Value` → `Wire` (move Vec/Box, không copy container).
/// Dùng khi giá trị nguồn không còn ai giữ — ví dụ đối số `spawn`/`send` vừa eval.
fn into_wire(v: Value) -> Wire {
    match v {
        Value::Unit => Wire::Unit,
        Value::Int(i) => Wire::Int(i),
        Value::Float(f) => Wire::Float(f),
        Value::Size(i) => Wire::Size(i),
        Value::Duration(i) => Wire::Duration(i),
        Value::Percent(f) => Wire::Percent(f),
        Value::Str(s) => Wire::Str(s.into_boxed_str()),
        Value::Bool(b) => Wire::Bool(b),
        Value::Bytes(b) => Wire::Bytes(b),
        Value::Char(c) => Wire::Char(c),
        Value::Path(s) => Wire::Path(s.into_boxed_str()),
        Value::List(xs) => Wire::List(xs.into_iter().map(into_wire).collect()),
        Value::Map(kv) => Wire::Map(
            kv.into_iter()
                .map(|(k, v)| (into_wire(k), into_wire(v)))
                .collect(),
        ),
        Value::Record(n, fs) => Wire::Record(
            n.into_boxed_str(),
            fs.into_iter()
                .map(|(k, v)| (k.into_boxed_str(), into_wire(v)))
                .collect(),
        ),
        Value::Enum(a, b, args) => Wire::Enum(
            a.as_ref().into(),
            b.as_ref().into(),
            args.into_iter().map(into_wire).collect(),
        ),
        Value::Range(a, b, c) => Wire::Range(a, b, c),
        Value::Builtin(_)
        | Value::Fn(_)
        | Value::Vm(_)
        | Value::Task(_)
        | Value::Chan(_)
        | Value::Cap { .. } => panic!("payload không Sendable (mô phỏng VS0508)"),
    }
}

/// Chuyển **tiêu thụ** `Wire` → `Value` (move Vec/Box; tên enum phải nội bộ hoá lại).
fn wire_into_value(w: Wire) -> Value {
    match w {
        Wire::Unit => Value::Unit,
        Wire::Int(i) => Value::Int(i),
        Wire::Float(f) => Value::Float(f),
        Wire::Size(i) => Value::Size(i),
        Wire::Duration(i) => Value::Duration(i),
        Wire::Percent(f) => Value::Percent(f),
        Wire::Str(s) => Value::Str(s.into_string()),
        Wire::Bool(b) => Value::Bool(b),
        Wire::Bytes(b) => Value::Bytes(b),
        Wire::Char(c) => Value::Char(c),
        Wire::Path(s) => Value::Path(s.into_string()),
        Wire::List(xs) => Value::List(xs.into_iter().map(wire_into_value).collect()),
        Wire::Map(kv) => Value::Map(
            kv.into_iter()
                .map(|(k, v)| (wire_into_value(k), wire_into_value(v)))
                .collect(),
        ),
        Wire::Record(n, fs) => Value::Record(
            n.into_string(),
            fs.into_iter()
                .map(|(k, v)| (k.into_string(), wire_into_value(v)))
                .collect(),
        ),
        Wire::Enum(a, b, args) => Value::Enum(
            name(a.as_ref()),
            name(b.as_ref()),
            args.into_iter().map(wire_into_value).collect(),
        ),
        Wire::Range(a, b, c) => Value::Range(a, b, c),
    }
}

/// (số cấp phát logic, số byte payload logic) khi `Value::clone()`.
fn value_stats(v: &Value) -> (u64, u64) {
    match v {
        Value::Unit
        | Value::Int(_)
        | Value::Float(_)
        | Value::Size(_)
        | Value::Duration(_)
        | Value::Percent(_)
        | Value::Bool(_)
        | Value::Char(_)
        | Value::Range(_, _, _)
        | Value::Builtin(_) => (0, 0),
        Value::Str(s) | Value::Path(s) => (1, s.len() as u64),
        Value::Bytes(b) => (1, b.len() as u64),
        Value::List(xs) => {
            let mut a = 1;
            let mut b = (xs.len() * std::mem::size_of::<Value>()) as u64;
            for x in xs {
                let (aa, bb) = value_stats(x);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        Value::Map(kv) => {
            let mut a = 1;
            let mut b = (kv.len() * std::mem::size_of::<(Value, Value)>()) as u64;
            for (k, v) in kv {
                let (aa, bb) = value_stats(k);
                a += aa;
                b += bb;
                let (aa, bb) = value_stats(v);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        Value::Record(n, fs) => {
            let mut a = 2; // Vec + tên
            let mut b = n.len() as u64
                + (fs.len() * std::mem::size_of::<(String, Value)>()) as u64;
            for (k, v) in fs {
                a += 1;
                b += k.len() as u64;
                let (aa, bb) = value_stats(v);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        Value::Enum(_, _, args) => {
            let mut a = 1;
            let mut b = (args.len() * std::mem::size_of::<Value>()) as u64;
            for x in args {
                let (aa, bb) = value_stats(x);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        // Rc bump, không cấp phát nội dung.
        Value::Fn(_) | Value::Vm(_) | Value::Task(_) | Value::Chan(_) => (0, 0),
        Value::Cap { kind, root, .. } => (2, (kind.len() + root.len()) as u64),
    }
}

fn wire_stats(w: &Wire) -> (u64, u64) {
    match w {
        Wire::Unit
        | Wire::Int(_)
        | Wire::Float(_)
        | Wire::Size(_)
        | Wire::Duration(_)
        | Wire::Percent(_)
        | Wire::Bool(_)
        | Wire::Char(_)
        | Wire::Range(_, _, _) => (0, 0),
        Wire::Str(s) | Wire::Path(s) => (1, s.len() as u64),
        Wire::Bytes(b) => (1, b.len() as u64),
        Wire::List(xs) => {
            let mut a = 1;
            let mut b = (xs.len() * std::mem::size_of::<Wire>()) as u64;
            for x in xs {
                let (aa, bb) = wire_stats(x);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        Wire::Map(kv) => {
            let mut a = 1;
            let mut b = (kv.len() * std::mem::size_of::<(Wire, Wire)>()) as u64;
            for (k, v) in kv {
                let (aa, bb) = wire_stats(k);
                a += aa;
                b += bb;
                let (aa, bb) = wire_stats(v);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        Wire::Record(n, fs) => {
            let mut a = 2;
            let mut b =
                n.len() as u64 + (fs.len() * std::mem::size_of::<(Box<str>, Wire)>()) as u64;
            for (k, v) in fs {
                a += 1;
                b += k.len() as u64;
                let (aa, bb) = wire_stats(v);
                a += aa;
                b += bb;
            }
            (a, b)
        }
        Wire::Enum(ten, bien_the, args) => {
            let mut allocs = 3;
            let mut bytes = (ten.len() + bien_the.len()) as u64
                + (args.len() * std::mem::size_of::<Wire>()) as u64;
            for x in args {
                let (aa, bb) = wire_stats(x);
                allocs += aa;
                bytes += bb;
            }
            (allocs, bytes)
        }
    }
}

// ===========================================================================
// 4. Bộ dựng dữ liệu
// ===========================================================================

fn list_ints(n: usize) -> Value {
    Value::List((0..n).map(|i| Value::Int(i as i64)).collect())
}

fn map_value(n: usize) -> Value {
    Value::Map(
        (0..n)
            .map(|i| (Value::Str(format!("k{i}")), Value::Int(i as i64)))
            .collect(),
    )
}

fn record_value(nfields: usize, list_len: usize) -> Value {
    let mut fs = Vec::with_capacity(nfields);
    for i in 0..nfields {
        let v = if i == 0 {
            list_ints(list_len)
        } else {
            Value::Int(i as i64)
        };
        fs.push((format!("f{i}"), v));
    }
    Value::Record("R".to_string(), fs)
}

fn enum_value(list_len: usize) -> Value {
    Value::Enum(name("Result"), name("Ok"), vec![list_ints(list_len)])
}

fn enum_values(n: usize) -> Value {
    Value::List(
        (0..n)
            .map(|i| {
                Value::Enum(
                    name("Option"),
                    name("Some"),
                    vec![Value::Int(i as i64)],
                )
            })
            .collect(),
    )
}

fn wire_list_ints(n: usize) -> Wire {
    Wire::List((0..n).map(|i| Wire::Int(i as i64)).collect())
}

fn arc_list_ints(n: usize) -> ArcValue {
    ArcValue::List(Arc::new(
        (0..n).map(|i| ArcValue::Int(i as i64)).collect(),
    ))
}

fn arc_map_value(n: usize) -> ArcValue {
    ArcValue::Map(Arc::new(
        (0..n)
            .map(|i| {
                (
                    ArcValue::Str(Arc::from(format!("k{i}"))),
                    ArcValue::Int(i as i64),
                )
            })
            .collect(),
    ))
}

fn arc_record_value(nfields: usize, list_len: usize) -> ArcValue {
    let mut fs = Vec::with_capacity(nfields);
    for i in 0..nfields {
        let v = if i == 0 {
            arc_list_ints(list_len)
        } else {
            ArcValue::Int(i as i64)
        };
        fs.push((Arc::from(format!("f{i}")), v));
    }
    ArcValue::Record(Arc::from("R"), Arc::new(fs))
}

fn arc_enum_value(list_len: usize) -> ArcValue {
    ArcValue::Enum(
        Arc::from("Result"),
        Arc::from("Ok"),
        Arc::new(vec![arc_list_ints(list_len)]),
    )
}

// ===========================================================================
// 5. Đo thời gian
// ===========================================================================

fn bench<F: FnMut()>(iters: u64, runs: usize, mut f: F) -> (f64, f64) {
    let warm = std::cmp::max(1, iters / 10);
    for _ in 0..warm {
        f();
    }
    let mut samples = Vec::with_capacity(runs);
    for _ in 0..runs {
        let t = Instant::now();
        for _ in 0..iters {
            f();
        }
        samples.push(t.elapsed().as_nanos() as f64 / iters as f64);
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    (samples[runs / 2], samples[0])
}

fn report_clone(label: &str, v: &Value, stats: (u64, u64), iters: u64) {
    let (med, min) = bench(iters, 5, || {
        black_box(v.clone());
    });
    report_line(label, med, min, stats);
}

fn report_line(label: &str, med: f64, min: f64, stats: (u64, u64)) {
    let (allocs, bytes) = stats;
    let mib_s = if bytes == 0 {
        0.0
    } else {
        bytes as f64 / med / 1024.0 / 1024.0 * 1e9
    };
    println!(
        "CLONE\t{label}\t{med:.2}\t{min:.2}\t{mib_s:.1}\t{allocs}\t{:.1}",
        bytes as f64 / 1024.0
    );
}

/// Đo chuyển **tiêu thụ** `Value → Wire` trên lô dựng sẵn (dựng ngoài vùng đo).
fn report_move_value<FB, FC>(
    label: &str,
    k: usize,
    build: FB,
    mut convert: FC,
    stats: (u64, u64),
) where
    FB: Fn() -> Value,
    FC: FnMut(Value) -> Wire,
{
    let mut samples = Vec::new();
    for _ in 0..3 {
        let values: Vec<Value> = (0..k).map(|_| build()).collect();
        let t = Instant::now();
        for v in values {
            black_box(convert(v));
        }
        samples.push(t.elapsed().as_nanos() as f64 / k as f64);
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    report_line(label, samples[1], samples[0], stats);
}

/// Cùng phép đo batch nhưng **mượn + clone** (đối chứng công bằng với move).
fn report_copy_value<FB>(
    label: &str,
    k: usize,
    build: FB,
    convert: fn(&Value) -> Wire,
    stats: (u64, u64),
) where
    FB: Fn() -> Value,
{
    let mut samples = Vec::new();
    for _ in 0..3 {
        let values: Vec<Value> = (0..k).map(|_| build()).collect();
        let t = Instant::now();
        for v in &values {
            black_box(convert(v));
        }
        samples.push(t.elapsed().as_nanos() as f64 / k as f64);
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    report_line(label, samples[1], samples[0], stats);
}

/// Đo chuyển **tiêu thụ** `Wire → Value` trên lô dựng sẵn.
fn report_move_wire<FB, FC>(
    label: &str,
    k: usize,
    build: FB,
    mut convert: FC,
    stats: (u64, u64),
) where
    FB: Fn() -> Wire,
    FC: FnMut(Wire) -> Value,
{
    let mut samples = Vec::new();
    for _ in 0..3 {
        let wires: Vec<Wire> = (0..k).map(|_| build()).collect();
        let t = Instant::now();
        for w in wires {
            black_box(convert(w));
        }
        samples.push(t.elapsed().as_nanos() as f64 / k as f64);
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    report_line(label, samples[1], samples[0], stats);
}

/// Batch mượn + clone `Wire → Value` (đối chứng với move).
fn report_copy_wire<FB>(
    label: &str,
    k: usize,
    build: FB,
    convert: fn(&Wire) -> Value,
    stats: (u64, u64),
) where
    FB: Fn() -> Wire,
{
    let mut samples = Vec::new();
    for _ in 0..3 {
        let wires: Vec<Wire> = (0..k).map(|_| build()).collect();
        let t = Instant::now();
        for w in &wires {
            black_box(convert(w));
        }
        samples.push(t.elapsed().as_nanos() as f64 / k as f64);
    }
    samples.sort_by(|a, b| a.total_cmp(b));
    report_line(label, samples[1], samples[0], stats);
}

// ===========================================================================
// 6. CPU work + worker pool
// ===========================================================================

#[inline(never)]
fn cpu_work(iters: u64) -> u64 {
    let mut x = 0x9e37_79b9_7f4a_7c15u64;
    for i in 0..iters {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i ^ 0x5deece66d);
        x ^= x >> 29;
    }
    x
}

fn calibrate_cpu(target_ns: f64) -> (u64, f64) {
    let mut iters = 10_000u64;
    for _ in 0..24 {
        let t = Instant::now();
        black_box(cpu_work(iters));
        let ns = t.elapsed().as_nanos() as f64;
        if ns >= target_ns {
            return (iters, ns / 1000.0);
        }
        let scale = (target_ns / ns.max(1.0)).clamp(1.2, 4.0);
        iters = ((iters as f64) * scale) as u64 + 1;
    }
    let t = Instant::now();
    black_box(cpu_work(iters));
    (iters, t.elapsed().as_nanos() as f64 / 1000.0)
}

/// Đốt CPU ~`ms` mili-giây trước khi đo — máy laptop tăng xung rất chậm
/// (đo được 31% xung lúc rảnh), nếu không sẽ đo nhầm “W=4 nhanh hơn 4×”。
fn warm_cpu(ms: u64) {
    let start = Instant::now();
    let mut acc = 0u64;
    while start.elapsed().as_millis() < ms as u128 {
        acc ^= cpu_work(100_000);
    }
    black_box(acc);
}

/// Pool “copy”: producer deep-copy payload vào job; worker deep-copy kết quả về.
/// Trả (giây, số kết quả).
fn run_pool_copy(
    workers: usize,
    tasks: usize,
    cpu_iters: u64,
    template: &Wire,
) -> (f64, usize) {
    let workers = workers.max(1);
    let mut job_txs = Vec::with_capacity(workers);
    let mut handles = Vec::with_capacity(workers);
    let (res_tx, res_rx) = mpsc::sync_channel::<(u64, Wire)>(64);

    for _ in 0..workers {
        let (tx, rx) = mpsc::sync_channel::<Wire>(64);
        job_txs.push(tx);
        let res_tx = res_tx.clone();
        // Setup copy nằm ngoài vùng đo.
        let result_template = template.clone();
        handles.push(std::thread::spawn(move || {
            while let Ok(arg) = rx.recv() {
                black_box(&arg);
                if cpu_iters > 0 {
                    black_box(cpu_work(cpu_iters));
                }
                let result = result_template.clone();
                if res_tx.send((0, result)).is_err() {
                    break;
                }
            }
        }));
    }
    drop(res_tx);

    let start = Instant::now();
    let producer_template = template.clone();
    let producer = std::thread::spawn(move || {
        for i in 0..tasks {
            let arg = producer_template.clone();
            if job_txs[i % workers].send(arg).is_err() {
                break;
            }
        }
    });

    let mut got = 0;
    while got < tasks {
        match res_rx.recv() {
            Ok((_, r)) => {
                black_box(&r);
                got += 1;
            }
            Err(_) => break,
        }
    }
    producer.join().unwrap();
    for h in handles {
        h.join().unwrap();
    }
    (start.elapsed().as_secs_f64(), got)
}

fn run_pool_arc(
    workers: usize,
    tasks: usize,
    cpu_iters: u64,
    template: &ArcValue,
) -> (f64, usize) {
    let workers = workers.max(1);
    let mut job_txs = Vec::with_capacity(workers);
    let mut handles = Vec::with_capacity(workers);
    let (res_tx, res_rx) = mpsc::sync_channel::<(u64, ArcValue)>(64);

    for _ in 0..workers {
        let (tx, rx) = mpsc::sync_channel::<ArcValue>(64);
        job_txs.push(tx);
        let res_tx = res_tx.clone();
        let result_template = template.clone();
        handles.push(std::thread::spawn(move || {
            while let Ok(arg) = rx.recv() {
                black_box(&arg);
                if cpu_iters > 0 {
                    black_box(cpu_work(cpu_iters));
                }
                let result = result_template.clone();
                if res_tx.send((0, result)).is_err() {
                    break;
                }
            }
        }));
    }
    drop(res_tx);

    let start = Instant::now();
    let producer_template = template.clone();
    let producer = std::thread::spawn(move || {
        for i in 0..tasks {
            let arg = producer_template.clone();
            if job_txs[i % workers].send(arg).is_err() {
                break;
            }
        }
    });

    let mut got = 0;
    while got < tasks {
        match res_rx.recv() {
            Ok((_, r)) => {
                black_box(&r);
                got += 1;
            }
            Err(_) => break,
        }
    }
    producer.join().unwrap();
    for h in handles {
        h.join().unwrap();
    }
    (start.elapsed().as_secs_f64(), got)
}

/// Cùng phép copy như pool nhưng không có luồng/channel — mốc so sánh.
fn run_local_copy(tasks: usize, cpu_iters: u64, template: &Wire) -> f64 {
    let start = Instant::now();
    for _ in 0..tasks {
        let arg = template.clone();
        black_box(&arg);
        if cpu_iters > 0 {
            black_box(cpu_work(cpu_iters));
        }
        let result = template.clone();
        black_box(&result);
    }
    start.elapsed().as_secs_f64()
}

fn run_local_arc(tasks: usize, cpu_iters: u64, template: &ArcValue) -> f64 {
    let start = Instant::now();
    for _ in 0..tasks {
        let arg = template.clone();
        black_box(&arg);
        if cpu_iters > 0 {
            black_box(cpu_work(cpu_iters));
        }
        let result = template.clone();
        black_box(&result);
    }
    start.elapsed().as_secs_f64()
}

// ===========================================================================
// 7. Channel
// ===========================================================================

fn run_channel_copy(producers: usize, per: usize, template: &Wire, decode: bool) -> (f64, usize) {
    let (tx, rx) = mpsc::sync_channel::<Wire>(1024);
    let start = Instant::now();
    let mut handles = Vec::with_capacity(producers);
    for _ in 0..producers {
        let tx = tx.clone();
        let t = template.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..per {
                if tx.send(t.clone()).is_err() {
                    break;
                }
            }
        }));
    }
    drop(tx);

    let mut got = 0;
    while let Ok(w) = rx.recv() {
        if decode {
            black_box(wire_to_value(&w));
        } else {
            black_box(&w);
        }
        got += 1;
    }
    for h in handles {
        h.join().unwrap();
    }
    (start.elapsed().as_secs_f64(), got)
}

fn run_channel_move(messages: Vec<Wire>) -> (f64, usize) {
    let n = messages.len();
    let (tx, rx) = mpsc::sync_channel::<Wire>(1024);
    let start = Instant::now();
    let h = std::thread::spawn(move || {
        for w in messages {
            if tx.send(w).is_err() {
                break;
            }
        }
    });
    let mut got = 0;
    while let Ok(w) = rx.recv() {
        black_box(&w);
        got += 1;
    }
    h.join().unwrap();
    (start.elapsed().as_secs_f64(), got.min(n))
}

fn run_channel_arc(producers: usize, per: usize, template: &ArcValue) -> (f64, usize) {
    let (tx, rx) = mpsc::sync_channel::<ArcValue>(1024);
    let start = Instant::now();
    let mut handles = Vec::with_capacity(producers);
    for _ in 0..producers {
        let tx = tx.clone();
        let t = template.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..per {
                if tx.send(t.clone()).is_err() {
                    break;
                }
            }
        }));
    }
    drop(tx);

    let mut got = 0;
    while let Ok(v) = rx.recv() {
        black_box(&v);
        got += 1;
    }
    for h in handles {
        h.join().unwrap();
    }
    (start.elapsed().as_secs_f64(), got)
}

include!("bench_extra.rs");
