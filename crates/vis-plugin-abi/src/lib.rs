// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-306 / ST-310c (VD-278, VD-283, VD-285)** — ABI tĩnh `VIS_PLUGIN_ABI_V1` (**guest ABI**).
//!
//! Crate này là **mặt tiếp xúc duy nhất** cho plugin Rust và cho host:
//!
//! - `PluginValue` là biên **an toàn** giữa giá trị ViScript và plugin; hàm/task/kênh không qua
//!   biên. Chuyển đổi với `vis_core::value::Value` nằm ở **host** (`vis-plugin-host::convert`) để
//!   crate này **0 phụ thuộc** (kể cả nội bộ workspace) và build `wasm32` trong < 1 s.
//! - `PluginCtx` chỉ chứa `Cap` **do host truyền vào**; **không** đọc biến môi trường, **không**
//!   tự mở tệp ngoài cap. Việc mở tệp Unix (`openat2`/`RESOLVE_BENEATH`) do host cài qua
//!   [`PluginIo`] — crate này không chứa bản sao syscall nào.
//! - `Error` là lỗi plugin (`vis::Error` khi đặt `use vis_plugin_abi as vis;`), mã `VS0604` cho
//!   thiếu quyền, `VS0501` cho lỗi I/O/kiểu.
//! - Crate **0 `unsafe`** (`#![forbid(unsafe_code)]`).
#![forbid(unsafe_code)]

use std::fmt;
use std::path::{Path, PathBuf};

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

/// Hai thao tác tệp tối thiểu host cài cho [`PluginCtx`] (**ST-310c**).
///
/// Chữ ký nhận `(gốc cap đã canonicalize, đường tương đối, đường hiển thị)` để mọi kiểm tra
/// quyền nằm ở host (`vis-plugin-host::static_reg` → `vis_host_native::quyen`) — crate guest
/// ABI không chứa `openat2`/`rustix`.
#[derive(Debug, Clone, Copy)]
pub struct PluginIo {
    #[cfg(not(target_arch = "wasm32"))]
    pub doc: fn(goc: &Path, rel: &Path, hien_thi: &Path) -> Result<String>,
    #[cfg(not(target_arch = "wasm32"))]
    pub ghi: fn(goc: &Path, rel: &Path, data: &[u8], hien_thi: &Path) -> Result<()>,
}

/// Ngữ cảnh plugin: **chỉ** chứa các `Cap` host truyền vào. Không đọc biến môi trường, không
/// tự mở tệp ngoài cap.
#[derive(Debug, Clone, Default)]
pub struct PluginCtx {
    caps: Vec<Cap>,
    io: Option<PluginIo>,
}

impl PluginCtx {
    /// Ngữ cảnh chỉ có cap (chưa gắn I/O của host).
    pub fn new(caps: Vec<Cap>) -> Self {
        Self { caps, io: None }
    }

    /// Gắn I/O do host cài (**ST-310c**); plugin tĩnh dùng
    /// `vis_plugin_host::static_reg::ctx_voi_host`.
    pub fn voi_io(mut self, io: PluginIo) -> Self {
        self.io = Some(io);
        self
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

    /// Đọc UTF-8 trong `cap[Dir]`; canonicalize để chặn `..`/symlink ra ngoài root, rồi nhờ
    /// host mở **từ fd gốc** (`RESOLVE_BENEATH|NO_SYMLINKS`) — không còn khe kiểm-rồi-mở.
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            let rel = canon
                .strip_prefix(&root)
                .map_err(|_| Error::cap_denied("Dir", &path.to_string_lossy()))?;
            let io = self.io.as_ref().ok_or_else(|| {
                Error::io("plugin tĩnh chưa được host cài I/O (`PluginCtx::voi_io`)")
            })?;
            (io.doc)(&root, rel, path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = &root;
            Err(Error::io(
                "guest wasm32 không có filesystem trực tiếp: dùng preopen WASI `/capN`",
            ))
        }
    }

    /// Ghi UTF-8 trong `cap[Dir]` có cờ `write`.
    ///
    /// **ST-309/ST-310c**: trên Unix host mở bằng `openat` từ fd gốc với `O_NOFOLLOW` +
    /// `RESOLVE_BENEATH|NO_SYMLINKS`; tên cuối là symlink (kể cả trỏ ra ngoài) ⇒ `VS0604`,
    /// không ghi xuyên link.
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ten = path.file_name().ok_or_else(|| {
                Error::io(format!(
                    "không xác định được tên tệp để ghi: `{}`",
                    path.display()
                ))
            })?;
            let rel = canon_parent
                .strip_prefix(&root)
                .map_err(|_| Error::cap_denied("Dir (write)", &path.to_string_lossy()))?;
            let io = self.io.as_ref().ok_or_else(|| {
                Error::io("plugin tĩnh chưa được host cài I/O (`PluginCtx::voi_io`)")
            })?;
            (io.ghi)(&root, &rel.join(ten), noi_dung.as_bytes(), path)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = &root;
            let _ = &canon_parent;
            let _ = noi_dung;
            Err(Error::io(
                "guest wasm32 không có filesystem trực tiếp: dùng preopen WASI `/capN`",
            ))
        }
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
        assert_eq!(PluginValue::from(5i64), PluginValue::Int(5));
        assert_eq!(i64::try_from(PluginValue::Int(5)).expect("int"), 5);
        assert!(i64::try_from(PluginValue::Bool(true)).is_err());
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

    #[test]
    fn cap_du_nhung_thieu_io_bao_ro() {
        let d = std::env::temp_dir().join(format!("vis-plugin-cap-io-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp");
        let f = d.join("a.txt");
        std::fs::write(&f, "x").expect("fixture");
        let ctx = PluginCtx::new(vec![Cap::dir(&d, false)]);
        let e = ctx.read_text(&f).expect_err("thiếu I/O host phải lỗi rõ");
        assert_eq!(e.code, "VS0501");
        assert!(e.message.contains("voi_io"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
