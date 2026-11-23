// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-306 / VD-278, VD-283** — ABI tĩnh `VIS_PLUGIN_ABI_V1` cho plugin Rust của ViScript.
//!
//! - `PluginValue` là biên **an toàn** giữa `vis_interp::value::Value` và plugin; chuyển đổi
//!   tường minh, không có quyền ngầm.
//! - `PluginCtx` chỉ cấp quyền qua `Cap` **do host truyền vào**; plugin đọc/ghi bằng
//!   [`PluginCtx::read_text`]/[`PluginCtx::write_text`], không dùng `std::fs` trực tiếp.
//! - `Error` là lỗi plugin (`vis::Error` khi đặt `use vis_plugin_abi as vis;`), mã `VS0604` cho
//!   thiếu quyền, `VS0501` cho lỗi I/O/kiểu.
//! - Crate **0 `unsafe`** (`#![forbid(unsafe_code)]`).
#![forbid(unsafe_code)]

use std::fmt;
use std::path::{Path, PathBuf};

use vis_interp::value::{Name, Value};

/// **ST-307 (VD-278/VD-281/VD-283)** — nạp plugin động WASI/WASM trong sandbox `cap[...]`.
pub mod wasi;

pub use wasi::{ChinhSach, HamPlugin, Manifest, PluginWasi, Runtime, ABI_HANDSHAKE};

/// Phiên bản ABI tĩnh hiện hành. Thay đổi cấu trúc `PluginValue`/`FuncDef` phải tăng số này.
pub const VIS_PLUGIN_ABI_V1: u32 = 1;

/// Lỗi plugin — tên phía Rust của `vis::Error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Mã chẩn đoán ViScript (`VS0604`, `VS0501`, …).
    pub code: &'static str,
    pub message: String,
}

impl Error {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn cap_denied(kind: &str, path: &str) -> Self {
        Self::new(
            "VS0604",
            format!("plugin thiếu `cap[{kind}]` cho `{path}` (không có quyền ngầm)"),
        )
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new("VS0501", message)
    }

    pub fn type_mismatch(want: &str, got: &str) -> Self {
        Self::new("VS0501", format!("plugin cần `{want}` nhưng nhận `{got}`"))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Một quyền host cấp tường minh cho plugin. `kind` hiện hỗ trợ `"Dir"` (và giữ chỗ cho
/// `"File"`/`"Net"` khi có host tương ứng).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cap {
    pub kind: String,
    pub root: PathBuf,
    pub write: bool,
}

impl Cap {
    pub fn dir(root: impl Into<PathBuf>, write: bool) -> Self {
        Self {
            kind: "Dir".to_string(),
            root: root.into(),
            write,
        }
    }
}

/// Ngữ cảnh plugin: **chỉ** chứa các `Cap` host truyền vào. Không đọc biến môi trường, không
/// tự mở tệp ngoài cap.
#[derive(Debug, Clone, Default)]
pub struct PluginCtx {
    caps: Vec<Cap>,
}

impl PluginCtx {
    pub fn new(caps: Vec<Cap>) -> Self {
        Self { caps }
    }

    pub fn caps(&self) -> &[Cap] {
        &self.caps
    }

    fn cap_dir(&self, path: &Path, write: bool) -> Result<&Cap> {
        let p = path.to_string_lossy();
        let cap = self
            .caps
            .iter()
            .find(|c| c.kind == "Dir" && path.starts_with(&c.root) && (!write || c.write))
            .ok_or_else(|| Error::cap_denied(if write { "Dir (write)" } else { "Dir" }, &p))?;
        Ok(cap)
    }

    /// Đọc UTF-8 trong `cap[Dir]`; canonicalize để chặn `..`/symlink ra ngoài root.
    pub fn read_text(&self, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        let cap = self.cap_dir(path, false)?;
        let canon = std::fs::canonicalize(path)
            .map_err(|e| Error::io(format!("không đọc được `{}`: {e}", path.display())))?;
        let root = std::fs::canonicalize(&cap.root).map_err(|e| {
            Error::io(format!(
                "không canonicalize được cap `{}`: {e}",
                cap.root.display()
            ))
        })?;
        if !canon.starts_with(&root) {
            return Err(Error::cap_denied("Dir", &path.to_string_lossy()));
        }
        std::fs::read_to_string(&canon)
            .map_err(|e| Error::io(format!("không đọc được `{}`: {e}", canon.display())))
    }

    /// Ghi UTF-8 trong `cap[Dir]` có cờ `write`.
    pub fn write_text(&self, path: impl AsRef<Path>, noi_dung: &str) -> Result<()> {
        let path = path.as_ref();
        let cap = self.cap_dir(path, true)?;
        let parent = path.parent().unwrap_or(Path::new("."));
        let canon_parent = std::fs::canonicalize(parent).map_err(|e| {
            Error::io(format!(
                "không canonicalize được thư mục `{}`: {e}",
                parent.display()
            ))
        })?;
        let root = std::fs::canonicalize(&cap.root).map_err(|e| {
            Error::io(format!(
                "không canonicalize được cap `{}`: {e}",
                cap.root.display()
            ))
        })?;
        if !canon_parent.starts_with(&root) {
            return Err(Error::cap_denied("Dir (write)", &path.to_string_lossy()));
        }
        let dich = canon_parent.join(path.file_name().unwrap_or_default());
        std::fs::write(&dich, noi_dung)
            .map_err(|e| Error::io(format!("không ghi được `{}`: {e}", dich.display())))
    }
}

/// Giá trị qua biên plugin. Chỉ các dạng dữ liệu thuần; hàm/task/kênh không qua biên.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginValue {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<PluginValue>),
    Map(Vec<(PluginValue, PluginValue)>),
    Option(Option<Box<PluginValue>>),
    Result(std::result::Result<Box<PluginValue>, Box<Error>>),
    Cap {
        kind: String,
        root: String,
        write: bool,
    },
}

impl TryFrom<Value> for PluginValue {
    type Error = Error;

    fn try_from(v: Value) -> Result<Self> {
        Ok(match v {
            Value::Unit => Self::Unit,
            Value::Bool(b) => Self::Bool(b),
            Value::Int(i) | Value::Size(i) | Value::Duration(i) => Self::Int(i),
            Value::Float(f) | Value::Percent(f) => Self::Float(f),
            Value::Str(s) => Self::Str(s),
            Value::Char(c) => Self::Str(c.to_string()),
            Value::Path(s) => Self::Str(s),
            Value::Bytes(b) => Self::Bytes(b),
            Value::List(xs) => {
                Self::List(xs.into_iter().map(Self::try_from).collect::<Result<_>>()?)
            }
            Value::Map(kv) => Self::Map(
                kv.into_iter()
                    .map(|(k, v)| Ok((Self::try_from(k)?, Self::try_from(v)?)))
                    .collect::<Result<_>>()?,
            ),
            Value::Record(_, fs) => Self::Map(
                fs.into_iter()
                    .map(|(k, v)| Ok((Self::Str(k), Self::try_from(v)?)))
                    .collect::<Result<_>>()?,
            ),
            Value::Enum(ten, bien, args) => match (ten.as_ref(), bien.as_ref()) {
                ("Option", "Some") if args.len() == 1 => Self::Option(Some(Box::new(
                    Self::try_from(args.into_iter().next().unwrap())?,
                ))),
                ("Option", "None") if args.is_empty() => Self::Option(None),
                ("Result", "Ok") if args.len() == 1 => Self::Result(Ok(Box::new(Self::try_from(
                    args.into_iter().next().unwrap(),
                )?))),
                ("Result", "Err") if args.len() == 1 => {
                    Self::Result(Err(Box::new(Error::io(vis_interp::value::show(&args[0])))))
                }
                _ => {
                    return Err(Error::type_mismatch(
                        "giá trị thuần (không enum tuỳ ý)",
                        &format!("{ten}::{bien}"),
                    ))
                }
            },
            Value::Cap { kind, root, write } => Self::Cap { kind, root, write },
            other => {
                return Err(Error::type_mismatch(
                    "giá trị thuần",
                    &vis_interp::value::show(&other),
                ))
            }
        })
    }
}

impl TryFrom<PluginValue> for Value {
    type Error = Error;

    fn try_from(v: PluginValue) -> Result<Self> {
        Ok(match v {
            PluginValue::Unit => Value::Unit,
            PluginValue::Bool(b) => Value::Bool(b),
            PluginValue::Int(i) => Value::Int(i),
            PluginValue::Float(f) => Value::Float(f),
            PluginValue::Str(s) => Value::Str(s),
            PluginValue::Bytes(b) => Value::Bytes(b),
            PluginValue::List(xs) => {
                Value::List(xs.into_iter().map(Value::try_from).collect::<Result<_>>()?)
            }
            PluginValue::Map(kv) => Value::Map(
                kv.into_iter()
                    .map(|(k, v)| Ok((Value::try_from(k)?, Value::try_from(v)?)))
                    .collect::<Result<_>>()?,
            ),
            PluginValue::Option(None) => {
                Value::Enum(Name::from("Option"), Name::from("None"), Vec::new())
            }
            PluginValue::Option(Some(x)) => Value::Enum(
                Name::from("Option"),
                Name::from("Some"),
                vec![Value::try_from(*x)?],
            ),
            PluginValue::Result(Ok(x)) => Value::Enum(
                Name::from("Result"),
                Name::from("Ok"),
                vec![Value::try_from(*x)?],
            ),
            PluginValue::Result(Err(e)) => Value::Enum(
                Name::from("Result"),
                Name::from("Err"),
                vec![Value::Str(e.message)],
            ),
            PluginValue::Cap { kind, root, write } => Value::Cap { kind, root, write },
        })
    }
}

macro_rules! tu_so {
    ($ty:ty, $bien:ident, $ten:literal) => {
        impl TryFrom<PluginValue> for $ty {
            type Error = Error;
            fn try_from(v: PluginValue) -> Result<Self> {
                match v {
                    PluginValue::$bien(x) => Ok(x),
                    other => Err(Error::type_mismatch($ten, &format!("{other:?}"))),
                }
            }
        }
        impl From<$ty> for PluginValue {
            fn from(x: $ty) -> Self {
                PluginValue::$bien(x)
            }
        }
    };
}

tu_so!(i64, Int, "int");
tu_so!(f64, Float, "float");
tu_so!(bool, Bool, "bool");
tu_so!(String, Str, "str");
tu_so!(Vec<u8>, Bytes, "Bytes");

impl From<()> for PluginValue {
    fn from((): ()) -> Self {
        Self::Unit
    }
}

/// Một tham số trong chữ ký ViScript do macro sinh.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    pub name: &'static str,
    pub ty: &'static str,
}

/// Chữ ký ViScript của một hàm plugin.
#[derive(Debug, Clone, Copy)]
pub struct Signature {
    pub name: &'static str,
    pub params: &'static [Param],
    pub ret: &'static str,
}

pub type PluginFn = fn(&PluginCtx, &[PluginValue]) -> Result<PluginValue>;

/// Một hàm plugin đã đăng ký; `call` chuyển `PluginValue` ↔ kiểu Rust **có kiểm lúc build**.
#[derive(Clone, Copy)]
pub struct FuncDef {
    pub sig: Signature,
    pub call: PluginFn,
}

impl FuncDef {
    /// Gọi hàm plugin qua biên ABI (wrapper do macro sinh).
    pub fn call(&self, ctx: &PluginCtx, args: &[PluginValue]) -> Result<PluginValue> {
        (self.call)(ctx, args)
    }
}

/// Module plugin tĩnh.
#[derive(Clone, Copy)]
pub struct PluginModule {
    pub name: &'static str,
    pub funcs: &'static [FuncDef],
}

impl PluginModule {
    pub fn tim(&self, ten: &str) -> Option<&FuncDef> {
        self.funcs.iter().find(|f| f.sig.name == ten)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_va_chuyen_doi_co_ban() {
        assert_eq!(VIS_PLUGIN_ABI_V1, 1);
        let v = Value::List(vec![Value::Int(1), Value::Str("x".into())]);
        let p = PluginValue::try_from(v.clone()).expect("chuyển sang PluginValue");
        let lai = Value::try_from(p).expect("chuyển lại");
        assert_eq!(vis_interp::value::show(&lai), vis_interp::value::show(&v));
    }

    #[test]
    fn cap_am_tu_choi_doc_ngoai_root() {
        let d = std::env::temp_dir().join(format!("vis-plugin-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        let ctx = PluginCtx::new(vec![Cap::dir(&d, false)]);
        let ngoai = std::env::temp_dir().join("vis-plugin-ngoai-root.txt");
        let e = ctx.read_text(&ngoai).expect_err("phải từ chối");
        assert_eq!(e.code, "VS0604");
        let _ = std::fs::remove_dir_all(&d);
    }
}
