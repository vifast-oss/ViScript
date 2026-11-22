// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-296a** — binary gộp miền: Capability/--grant/sandbox/Landlock + cấm ghi ngoài.
//!
//! Các tệp ca gốc nằm trong thư mục cùng tên; `#[path]` giữ nguyên nội dung, không copy test.
//! Tổng số ca/assertion không đổi — chỉ đổi crate test binary.

mod common;

#[path = "security_it/cap.rs"]
mod cap;
#[path = "security_it/cap_khong_lo.rs"]
mod cap_khong_lo;
#[path = "security_it/cap_proc.rs"]
mod cap_proc;
#[path = "security_it/cap_proc_grant.rs"]
mod cap_proc_grant;
#[path = "security_it/gan_hai_duong.rs"]
mod gan_hai_duong;
#[path = "security_it/grant_proc_canon.rs"]
mod grant_proc_canon;
#[path = "security_it/quyen.rs"]
mod quyen;
#[path = "security_it/sandbox.rs"]
mod sandbox;
#[path = "security_it/st295_sandbox_proc.rs"]
mod st295_sandbox_proc;
#[path = "security_it/st83_no_nho.rs"]
mod st83_no_nho;
#[path = "security_it/strict_caps.rs"]
mod strict_caps;
