// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296b (VD-281/VD-282)** — `vis-loader`: nạp đa tệp + metadata `vis.lock`/băm nội dung.
//!
//! Crate **thuần**: không OS/TLS/SQLite, `#![forbid(unsafe_code)]`, biên dịch được cho
//! `wasm32-unknown-unknown`/`wasm32-wasip1`. `modules`/`cache`/`json_esc`/`audit_span` chuyển
//! nguyên từ `vis-cli` (0 đổi hành vi); từ ST-310a `Value`/`json`/`json_esc` lấy từ `vis-core`; `lock_meta` chứa `{bam, chuan_hoa, doc_khoa_cua, GoiKhoa}`.
//! `audit_span` chỉ là audit test (trước đây `#[cfg(test)] mod audit_span` trong `vis-cli`).

#![forbid(unsafe_code)]

#[cfg(test)]
pub mod audit_span;
pub mod cache;
pub mod json_esc;
pub mod lock_meta;
pub mod modules;
pub mod semver;
