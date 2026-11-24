// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310a (VD-285)** — lõi dùng chung của dòng ViScript.
//!
//! Tầng đáy của DAG: `vis-core` → (`vis-interp`, `vis-loader`, `vis-lsp`, `vis-host-native`,
//! `vis-pkg`). Nó **không** biết OS, không socket, không `unsafe`; chỉ giữ hình dạng dữ liệu và
//! trait để các tầng trên cài đặt:
//!
//! - [`value`] — `Value` + `Show`/so sánh + đọc literal, và dữ liệu runtime `Task`/`Chan`;
//! - [`json`] — decoder/encoder + **một** bộ `json_esc` duy nhất (đóng nợ #112);
//! - [`host`] — trait `Ngoai`/`FsHost`/`NetHost`/`DbHost`;
//! - [`reactor`] — trait adapter selector (`FrameReactorBackend`) + trạng thái `net.write`;
//! - [`http_client`] — HTTP/1.1 đồng bộ trên [`host::NetHost`] + cookie jar;
//! - [`trap`] — `Trap`/`Abort`/`R`/`pos` dùng chung.

#![forbid(unsafe_code)]

pub mod frame;
pub mod host;
pub mod http_client;
pub mod json;
pub mod reactor;
pub mod run;
pub mod trap;
pub mod value;

pub use frame::{DiagSnapshot, FakeEvent, FrameConfig, FrameOutput, ReactorEvent, ReactorOutcome};
pub use run::RunOutput;
pub use trap::{pos, Abort, Trap, R};
pub use value::{exit_code, show, Closure, Name, Value};
