// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-310c (VD-285)** — host plugin ViScript, tách khỏi guest ABI `vis-plugin-abi`.
//!
//! - [`convert`]: `vis_core::value::Value` ↔ `PluginValue` (guest ABI không phụ thuộc `vis-core`);
//! - [`static_reg`]: cài I/O `cap[Dir]` cho plugin tĩnh qua `vis_host_native::quyen` + sổ đăng ký;
//! - [`wasi`]: loader/runtime/sandbox cho plugin động WASI/WASM (chia `protocol`/`runner`/`sandbox`).
//!
//! Crate **0 `unsafe`**; **không** có bản sao `openat2` (ST-310c).
#![forbid(unsafe_code)]

pub mod convert;
pub mod static_reg;
pub mod wasi;

pub use vis_plugin_abi::{
    Cap, Error, FuncDef, Param, PluginCtx, PluginFn, PluginIo, PluginModule, PluginValue, Result,
    Signature, VIS_PLUGIN_ABI_V1,
};
pub use wasi::{ChinhSach, HamPlugin, Manifest, PluginWasi, Runtime, ABI_HANDSHAKE};
