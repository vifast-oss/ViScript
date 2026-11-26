// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a** — kiểu/handle worker pool (tách từ `rt/worker.rs`, nguyên văn).

#[cfg(feature = "taskid-proto")]
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
#[cfg(feature = "taskid-proto")]
use std::rc::Rc;
/// Handle **opaque** của một task (u64); không phải `Rc<Task>`/con trỏ heap.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub(crate) u64);

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

/// **Prototype VS3-3b TaskId trong `Value`** — hình dạng đề xuất: mọi biến thể của
/// `Value` giữ nguyên, riêng `Task` là `TaskId` (u64) thay `Rc<Task>`; completion/proxy
/// nằm ở `WorkerPool` (không `Arc<Mutex<Task>>`).
///
/// **Chưa phải `Value` production**: `interp.rs`/`vm.rs`/`frame/**` vẫn khớp
/// `Value::Task(Rc<Task>)`; đổi enum cần W1/A0 duyệt. Default `Value`/ABI **không đổi**.
/// Type này để test chuyển đổi/`Show`/eq và làm hợp đồng patch W1.
#[cfg(feature = "taskid-proto")]
#[doc(hidden)]
#[derive(Clone, Debug)]
pub enum ProtoValue {
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
    List(Vec<ProtoValue>),
    Map(Vec<(ProtoValue, ProtoValue)>),
    Record(String, Vec<(String, ProtoValue)>),
    Enum(String, String, Vec<ProtoValue>),
    Range(i64, i64, bool),
    Builtin(&'static str),
    Fn(Rc<vis_core::value::Closure>),
    Vm(Rc<vis_core::value::VmFunc>),
    Task(TaskId),
    Chan(Rc<RefCell<crate::rt::Chan>>),
    Cap {
        kind: String,
        root: String,
        write: bool,
    },
}

#[cfg(feature = "taskid-proto")]
impl ProtoValue {
    /// Chuyển `Value` → prototype. `Value::Task(Rc<Task>)` **không** tự map được: W1 phải
    /// có registry `Rc<Task> ↔ TaskId`; trả lỗi rõ thay vì bịa id.
    pub fn from_value(v: &vis_core::value::Value) -> Result<Self, String> {
        use vis_core::value::Value as V;
        Ok(match v {
            V::Unit => Self::Unit,
            V::Int(i) => Self::Int(*i),
            V::Float(f) => Self::Float(*f),
            V::Size(i) => Self::Size(*i),
            V::Duration(i) => Self::Duration(*i),
            V::Percent(f) => Self::Percent(*f),
            V::Str(s) => Self::Str(s.clone()),
            V::Bool(b) => Self::Bool(*b),
            V::Bytes(b) => Self::Bytes(b.clone()),
            V::Char(c) => Self::Char(*c),
            V::Path(s) => Self::Path(s.clone()),
            V::List(xs) => Self::List(
                xs.iter()
                    .map(Self::from_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            V::Map(kv) => Self::Map(
                kv.iter()
                    .map(|(k, v)| Ok((Self::from_value(k)?, Self::from_value(v)?)))
                    .collect::<Result<Vec<_>, String>>()?,
            ),
            V::Record(n, fs) => Self::Record(
                n.clone(),
                fs.iter()
                    .map(|(k, v)| Ok((k.clone(), Self::from_value(v)?)))
                    .collect::<Result<Vec<_>, String>>()?,
            ),
            V::Enum(a, b, args) => Self::Enum(
                a.as_ref().to_string(),
                b.as_ref().to_string(),
                args.iter()
                    .map(Self::from_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            V::Range(a, b, c) => Self::Range(*a, *b, *c),
            V::Builtin(n) => Self::Builtin(n),
            V::Fn(c) => Self::Fn(Rc::clone(c)),
            V::Vm(f) => Self::Vm(Rc::clone(f)),
            V::Task(_) => {
                return Err(
                    "prototype: `Value::Task(Rc<Task>)` cần registry map sang TaskId".to_string(),
                )
            }
            V::Chan(c) => Self::Chan(Rc::clone(c)),
            V::Cap { kind, root, write } => Self::Cap {
                kind: kind.clone(),
                root: root.clone(),
                write: *write,
            },
        })
    }

    /// Chuyển prototype → `Value`; `Task(TaskId)` cần registry proxy của W1, nên trả lỗi.
    pub fn to_value(&self) -> Result<vis_core::value::Value, String> {
        use vis_core::value::Value as V;
        Ok(match self {
            Self::Unit => V::Unit,
            Self::Int(i) => V::Int(*i),
            Self::Float(f) => V::Float(*f),
            Self::Size(i) => V::Size(*i),
            Self::Duration(i) => V::Duration(*i),
            Self::Percent(f) => V::Percent(*f),
            Self::Str(s) => V::Str(s.clone()),
            Self::Bool(b) => V::Bool(*b),
            Self::Bytes(b) => V::Bytes(b.clone()),
            Self::Char(c) => V::Char(*c),
            Self::Path(s) => V::Path(s.clone()),
            Self::List(xs) => V::list(
                xs.iter()
                    .map(Self::to_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Map(kv) => V::map(
                kv.iter()
                    .map(|(k, v)| Ok((Self::to_value(k)?, Self::to_value(v)?)))
                    .collect::<Result<Vec<_>, String>>()?,
            ),
            Self::Record(n, fs) => V::Record(
                n.clone(),
                fs.iter()
                    .map(|(k, v)| Ok((k.clone(), Self::to_value(v)?)))
                    .collect::<Result<Vec<_>, String>>()?,
            ),
            Self::Enum(a, b, args) => V::Enum(
                vis_core::value::Name::new(a),
                vis_core::value::Name::new(b),
                args.iter()
                    .map(Self::to_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Self::Range(a, b, c) => V::Range(*a, *b, *c),
            Self::Builtin(n) => V::Builtin(n),
            Self::Fn(c) => V::Fn(Rc::clone(c)),
            Self::Vm(f) => V::Vm(Rc::clone(f)),
            Self::Task(_) => {
                return Err(
                    "prototype: `TaskId` cần registry/proxy để dựng `Value::Task`".to_string(),
                )
            }
            Self::Chan(c) => V::Chan(Rc::clone(c)),
            Self::Cap { kind, root, write } => V::Cap {
                kind: kind.clone(),
                root: root.clone(),
                write: *write,
            },
        })
    }

    pub fn task_id(&self) -> Option<TaskId> {
        match self {
            Self::Task(id) => Some(*id),
            _ => None,
        }
    }

    /// `Show`: cây **không có** `Task` dùng đúng `Value::show` (một nguồn sự thật);
    /// cây có `Task` render tại chỗ với `<tác vụ>` (không dựng được `Value`).
    pub fn show(&self) -> String {
        if let Ok(v) = self.to_value() {
            return v.show();
        }
        match self {
            Self::Task(_) => "<tác vụ>".to_string(),
            Self::List(xs) => {
                let ds: Vec<String> = xs.iter().map(Self::show).collect();
                format!("[{}]", ds.join(", "))
            }
            Self::Map(kv) => {
                let ds: Vec<String> = kv
                    .iter()
                    .map(|(k, v)| {
                        let key = match k {
                            Self::Str(s) => format!("\"{s}\""),
                            other => other.show(),
                        };
                        format!("{key}: {}", v.show())
                    })
                    .collect();
                format!("{{{}}}", ds.join(", "))
            }
            Self::Record(n, fs) => {
                let ds: Vec<String> = fs
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.show()))
                    .collect();
                if n.is_empty() {
                    format!("{{{}}}", ds.join(", "))
                } else {
                    format!("{n}({})", ds.join(", "))
                }
            }
            Self::Enum(_, b, args) => {
                if args.is_empty() {
                    b.clone()
                } else {
                    let ds: Vec<String> = args.iter().map(Self::show).collect();
                    format!("{b}({})", ds.join(", "))
                }
            }
            other => other
                .to_value()
                .map(|v| v.show())
                .unwrap_or_else(|_| "<giá trị>".to_string()),
        }
    }

    /// Task không bằng bất kỳ giá trị nào (kể cả Task/TaskId khác); còn lại dùng `Value::eq_value`.
    pub fn eq_value(&self, other: &Self) -> bool {
        if matches!(self, Self::Task(_)) || matches!(other, Self::Task(_)) {
            return false;
        }
        match (self.to_value(), other.to_value()) {
            (Ok(a), Ok(b)) => a.eq_value(&b),
            _ => false,
        }
    }
}

impl SendValue {
    /// Chuyển lại `Value` trong heap của worker/main. Tên enum được nội bộ hoá lại.
    pub fn to_value(&self) -> vis_core::value::Value {
        match self {
            SendValue::Unit => vis_core::value::Value::Unit,
            SendValue::Int(i) => vis_core::value::Value::Int(*i),
            SendValue::Float(f) => vis_core::value::Value::Float(*f),
            SendValue::Size(i) => vis_core::value::Value::Size(*i),
            SendValue::Duration(i) => vis_core::value::Value::Duration(*i),
            SendValue::Percent(f) => vis_core::value::Value::Percent(*f),
            SendValue::Str(s) => vis_core::value::Value::Str(s.clone()),
            SendValue::Bool(b) => vis_core::value::Value::Bool(*b),
            SendValue::Bytes(b) => vis_core::value::Value::Bytes(b.clone()),
            SendValue::Char(c) => vis_core::value::Value::Char(*c),
            SendValue::Path(s) => vis_core::value::Value::Path(s.clone()),
            SendValue::List(xs) => {
                vis_core::value::Value::list(xs.iter().map(SendValue::to_value).collect())
            }
            SendValue::Map(kv) => vis_core::value::Value::map(
                kv.iter()
                    .map(|(k, v)| (k.to_value(), v.to_value()))
                    .collect(),
            ),
            SendValue::Record(n, fs) => vis_core::value::Value::Record(
                n.clone(),
                fs.iter().map(|(k, v)| (k.clone(), v.to_value())).collect(),
            ),
            SendValue::Enum(a, b, args) => vis_core::value::Value::Enum(
                vis_core::value::Name::new(a),
                vis_core::value::Name::new(b),
                args.iter().map(SendValue::to_value).collect(),
            ),
        }
    }
}

/// **ST-310a** — mở rộng `vis_core::value::Value` (kiểu ngoại crate) bằng trait địa phương
/// thay cho inherent impl (Rust cấm impl inherent trên kiểu ngoại crate).
pub trait ToSend {
    /// Chuyển sang dạng Sendable; closure/task/channel/capability ⇒ lỗi (VS0508).
    fn to_send(&self) -> Result<SendValue, String>;
}

impl ToSend for vis_core::value::Value {
    fn to_send(&self) -> Result<SendValue, String> {
        match self {
            vis_core::value::Value::Unit => Ok(SendValue::Unit),
            vis_core::value::Value::Int(i) => Ok(SendValue::Int(*i)),
            vis_core::value::Value::Float(f) => Ok(SendValue::Float(*f)),
            vis_core::value::Value::Size(i) => Ok(SendValue::Size(*i)),
            vis_core::value::Value::Duration(i) => Ok(SendValue::Duration(*i)),
            vis_core::value::Value::Percent(f) => Ok(SendValue::Percent(*f)),
            vis_core::value::Value::Str(s) => Ok(SendValue::Str(s.clone())),
            vis_core::value::Value::Bool(b) => Ok(SendValue::Bool(*b)),
            vis_core::value::Value::Bytes(b) => Ok(SendValue::Bytes(b.clone())),
            vis_core::value::Value::Char(c) => Ok(SendValue::Char(*c)),
            vis_core::value::Value::Path(s) => Ok(SendValue::Path(s.clone())),
            vis_core::value::Value::List(xs) => Ok(SendValue::List(
                xs.iter().map(ToSend::to_send).collect::<Result<_, _>>()?,
            )),
            vis_core::value::Value::Map(kv) => Ok(SendValue::Map(
                kv.iter()
                    .map(|(k, v)| Ok((k.to_send()?, v.to_send()?)))
                    .collect::<Result<_, String>>()?,
            )),
            vis_core::value::Value::Record(n, fs) => Ok(SendValue::Record(
                n.clone(),
                fs.iter()
                    .map(|(k, v)| Ok((k.clone(), v.to_send()?)))
                    .collect::<Result<_, String>>()?,
            )),
            vis_core::value::Value::Enum(a, b, args) => Ok(SendValue::Enum(
                a.as_ref().to_string(),
                b.as_ref().to_string(),
                args.iter().map(ToSend::to_send).collect::<Result<_, _>>()?,
            )),
            vis_core::value::Value::Range(_, _, _) => {
                Err("`Range` nội bộ không gửi được sang worker (VS0508)".to_string())
            }
            vis_core::value::Value::Builtin(_)
            | vis_core::value::Value::Fn(_)
            | vis_core::value::Value::Vm(_)
            | vis_core::value::Value::Task(_)
            | vis_core::value::Value::Chan(_)
            | vis_core::value::Value::Cap { .. } => {
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
