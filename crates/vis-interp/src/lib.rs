// SPDX-License-Identifier: MIT OR Apache-2.0
//! **`vis-interp` — thông dịch cây trên HIR (ST-13, VS0).**
//!
//! Chạy chương trình đã hạ (`vis_types::hir::Hir`) theo ngữ nghĩa VD-43: `let` bất biến / `var` gán
//! được; `if/while/for/match`; `return`; `?` (Result/Option, trả sớm); `??`; `|>`; `with`; chỉ số;
//! gọi hàm (tham số tên `--b` + mặc định), closure; 8 tên dựng sẵn + bộ phương thức lõi.
//!
//! # Lỗi runtime — sổ `05xx` (VD-42)
//!
//! `VS0501` chung (tràn `i64`, phép/tính năng chưa hỗ trợ ở VS0), `VS0502` chia/lấy dư cho 0,
//! `VS0503` chỉ số ngoài phạm vi. **Không panic** trên dữ liệu người dùng; chẩn đoán có `span` +
//! `line`/`col` **thật** tính từ `text` + `span` (VD-46 bắt buộc ở tầng này).
//!
//! # API
//!
//! `run(text, &hir, &symbols, &types, &args) -> RunOutput` — chạy `main` nếu có (đối số `args`),
//! không có thì chạy các câu lệnh cấp tệp theo thứ tự. `text` cần cho `line`/`col` của chẩn đoán.
//!
//! ```
//! use vis_interp::run;
//! use vis_syntax::{lex, parse, Mode};
//! use vis_types::lower;
//!
//! let src = "print(1 + 2)";
//! let p = parse(src, Mode::Script);
//! let h = lower(&p.cst, &lex(src, Mode::Script));
//! let out = run(src, &h.hir, &h.symbols, &h.types, &[]);
//! assert!(out.is_clean());
//! assert_eq!(out.stdout, "3\n");
//! assert_eq!(out.exit, 0);
//! ```
//!
//! Chia 0 ⇒ một chẩn đoán `VS0502` (không panic):
//!
//! ```
//! # use vis_interp::run;
//! # use vis_syntax::{lex, parse, Mode};
//! # use vis_types::lower;
//! let src = "print(1 / 0)";
//! let p = parse(src, Mode::Script);
//! let h = lower(&p.cst, &lex(src, Mode::Script));
//! let out = run(src, &h.hir, &h.symbols, &h.types, &[]);
//! assert_eq!(out.codes(), ["VS0502"]);
//! assert_eq!(out.exit, 1);
//! ```

pub mod builtin_assert;
/// **VD-241 (Gate 1)** — trait `DbHost` (host cơ sở dữ liệu).
pub mod builtin_db;
pub mod builtin_list;
/// **ST-46** — `ls --long` helpers.
mod builtin_ls;
mod gan;
mod host;
pub mod json;
pub(crate) mod lift;
pub mod net;
pub mod render;
pub mod rt;
pub mod stdlib;
pub mod value;

/// Tên ngắn cho **bảng ngữ nghĩa dùng chung** (ST-20): `ops::bin`, `ops::un`.
pub mod ops {
    pub use crate::builtin::{bin, un};
}

pub mod builtin;
mod builtin_value;
/// **ST-267/VD-251 (chỉ test/debug)** — frame scheduler nối vào đường interp thật.
#[doc(hidden)]
pub mod frame;
pub mod interp;
mod pat;

pub use builtin::{BuiltinSig, Host, SIGS};
pub use interp::{pos, Abort, Trap, R};
pub use value::{exit_code, show, Closure, Value};

use vis_syntax::Diagnostic;
use vis_types::hir::Hir;
use vis_types::lower::Symbols;
use vis_types::TypeTable;

/// Kết quả một lần chạy.
#[derive(Debug, Clone, Default)]
pub struct RunOutput {
    /// Toàn bộ `stdout` đã in (`print`).
    pub stdout: String,
    /// Chẩn đoán runtime (`VS05xx`), tối đa một cái — lỗi đầu tiên dừng chương trình.
    pub diagnostics: Vec<Diagnostic>,
    /// Giá trị `main` trả (hoặc biểu thức cấp tệp cuối).
    pub value: Option<Value>,
    /// Mã thoát: `0` nếu sạch; nếu `main` trả `int` thì dùng số đó (cắt 8 bit); lỗi ⇒ `1`.
    pub exit: i32,
    /// Số bước đã chạy — **chống xanh-giả** trong test (`out("")` vẫn phải chạy thật).
    pub steps: usize,
}

impl RunOutput {
    /// `true` nếu không có chẩn đoán.
    pub fn is_clean(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Mã chẩn đoán theo thứ tự.
    pub fn codes(&self) -> Vec<&'static str> {
        self.diagnostics.iter().map(|d| d.code.short).collect()
    }
}

/// Chạy một chương trình đã hạ.
pub fn run(
    text: &str,
    hir: &Hir,
    symbols: &Symbols,
    types: &TypeTable,
    args: &[String],
) -> RunOutput {
    run_with_ngoai(text, hir, symbols, types, args, None)
}

/// Như [`run`] nhưng gắn **host chương trình ngoài** (ST-27). `None` ⇒ lệnh ngoài ⇒ `VS0604`.
pub fn run_with_ngoai<'a>(
    text: &'a str,
    hir: &'a Hir,
    symbols: &'a Symbols,
    types: &'a TypeTable,
    args: &[String],
    ngoai: Option<&'a mut dyn crate::builtin::Ngoai>,
) -> RunOutput {
    run_with_hosts(text, hir, symbols, types, args, ngoai, None, None)
}

/// Như [`run`] nhưng gắn **cả ba** host: chương trình ngoài (ST-27), hệ thống tệp (ST-33) và mạng
/// (ST-63, VD-105). `None` ở host nào ⇒ tính năng đó ⇒ `VS0604`. **VD-241**: DB host `None`.
#[allow(clippy::too_many_arguments)]
pub fn run_with_hosts<'a>(
    text: &'a str,
    hir: &'a Hir,
    symbols: &'a Symbols,
    types: &'a TypeTable,
    args: &[String],
    ngoai: Option<&'a mut dyn crate::builtin::Ngoai>,
    fs: Option<&'a mut dyn crate::builtin::FsHost>,
    net: Option<&'a mut dyn crate::builtin::NetHost>,
) -> RunOutput {
    run_with_hosts_caps(text, hir, symbols, types, args, ngoai, fs, net, None, &[])
}

/// **ST-72b (VD-103/VD-106)**: như [`run_with_hosts`] nhưng gắn **giá trị quyền** `cap` cho `main`
/// (`(chỉ số tham số, giá trị)`); CLI tính từ CST. Đường cũ giữ nguyên chữ ký qua wrapper trên.
/// **VD-241 (Gate 1)**: nhận thêm **host cơ sở dữ liệu** (`None` ⇒ `std.sqlite.*` ⇒ `VS0604`).
#[allow(clippy::too_many_arguments)]
pub fn run_with_hosts_caps<'a>(
    text: &'a str,
    hir: &'a Hir,
    symbols: &'a Symbols,
    types: &'a TypeTable,
    args: &[String],
    ngoai: Option<&'a mut dyn crate::builtin::Ngoai>,
    fs: Option<&'a mut dyn crate::builtin::FsHost>,
    net: Option<&'a mut dyn crate::builtin::NetHost>,
    db: Option<&'a mut dyn crate::builtin::DbHost>,
    caps: &[(u32, Value)],
) -> RunOutput {
    let mut it = interp::Interp::new(hir, symbols, types);
    if !caps.is_empty() {
        it.set_caps(caps.to_vec());
    }
    if let Some(n) = ngoai {
        it.set_ngoai(n);
    }
    if let Some(f) = fs {
        it.set_fs(f);
    }
    if let Some(g) = net {
        it.set_net(g);
    }
    if let Some(d) = db {
        it.set_db(d);
    }
    let res = it.run_program(args);
    let stdout = it.take_stdout();
    let steps = it.steps;
    match res {
        Ok(value) => {
            // **VD-241 §3/T5 (A0, 2026-11-19)** — `Ok`/`Result::Ok` ⇒ `exit_code` (Err ⇒ 1).
            let exit = match &value {
                Some(v) => crate::value::exit_code(v),
                None => 0,
            };
            RunOutput {
                stdout,
                diagnostics: Vec::new(),
                value,
                exit,
                steps,
            }
        }
        Err(interp::Abort::Ret(value)) => RunOutput {
            stdout,
            diagnostics: Vec::new(),
            value: Some(value.clone()),
            exit: crate::value::exit_code(&value),
            steps,
        },
        Err(interp::Abort::Trap(trap)) => RunOutput {
            stdout,
            diagnostics: vec![trap.to_diag(text)],
            value: None,
            exit: 1,
            steps,
        },
        // Chỉ test/frame: `run_program` mặc định không bật frame ⇒ không xảy ra;
        // driver frame dùng entry point riêng và tự xử lý `Pending`.
        Err(interp::Abort::Pending(_)) => RunOutput {
            stdout,
            diagnostics: Vec::new(),
            value: None,
            exit: 1,
            steps,
        },
    }
}
