// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis doctor [--json]`: thông tin chẩn đoán **tất định** của bản `vis` đang chạy.
//!
//! Hợp đồng:
//! - chế độ người đọc (stdout, đúng 5 dòng theo thứ tự): `vis <phiên bản>`, `os <OS>`,
//!   `arch <ARCH>`, `sqlite <on|off>`, `features <danh sách, phẩy>` (rỗng ⇒ đúng chữ `features`);
//! - `--json` (stdout, đúng **một** dòng + newline): khoá theo thứ tự cố định
//!   `version`/`os`/`arch`/`sqlite`/`features`;
//! - cờ/đối số lạ, hoặc `--json` kèm đối số khác ⇒ exit 2, usage trên stderr, **stdout rỗng**;
//! - **không** đọc môi trường, thư mục hiện tại, tệp hay mạng: mọi trường là hằng biên dịch
//!   (`CARGO_PKG_VERSION`, `std::env::consts`, `cfg!(feature = "sqlite")`) nên test tất định.
//!
//! Bằng chứng **đỏ trước** (main `5dead58`, trước khi thêm lệnh):
//! ```text
//! $ vis doctor
//! dùng: vis explain ... | vis pkg add|install|list     # usage gốc
//! $ echo $?
//! 2
//! ```
//! stdout rỗng; `vis doctor --json` cũng vậy vì `doctor` rơi vào nhánh `_ => usage()` của
//! `main`. Sau khi thêm lệnh, cùng lời gọi cho exit 0 + 5 dòng ở trên.

use crate::json_esc;
use std::process::ExitCode;

/// Danh sách feature **cố định lúc biên dịch** — không dò runtime, không đọc tệp.
/// Thứ tự cố định: `sqlite`, `tls`, `pkg-sig`, `http2` (feature nào bật mới xuất hiện).
fn features() -> Vec<&'static str> {
    let mut ds = Vec::new();
    if cfg!(feature = "sqlite") {
        ds.push("sqlite");
    }
    if cfg!(feature = "tls") {
        ds.push("tls");
    }
    if cfg!(feature = "pkg-sig") {
        ds.push("pkg-sig");
    }
    if cfg!(feature = "http2") {
        ds.push("http2");
    }
    ds
}

/// `true` khi feature `sqlite` bật — **cùng nguồn** với [`features`].
fn co_sqlite() -> bool {
    cfg!(feature = "sqlite")
}

fn usage_doctor() -> ExitCode {
    eprintln!("dùng: vis doctor [--json]");
    eprintln!(
        "  --json: in đúng một object JSON nén trên stdout (khoá version,os,arch,sqlite,features); không đọc môi trường/tệp/mạng"
    );
    ExitCode::from(2)
}

/// Năm dòng người đọc, theo thứ tự cố định.
fn in_thuong() {
    println!("vis {}", env!("CARGO_PKG_VERSION"));
    println!("os {}", std::env::consts::OS);
    println!("arch {}", std::env::consts::ARCH);
    println!("sqlite {}", if co_sqlite() { "on" } else { "off" });
    let features = features();
    if features.is_empty() {
        println!("features");
    } else {
        println!("features {}", features.join(","));
    }
}

/// Một object JSON nén, một dòng; thứ tự khoá cố định như hợp đồng.
fn in_json() {
    let features = features()
        .iter()
        .map(|f| format!("\"{}\"", json_esc::esc(f)))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"version\":\"{}\",\"os\":\"{}\",\"arch\":\"{}\",\"sqlite\":{},\"features\":[{}]}}",
        json_esc::esc(env!("CARGO_PKG_VERSION")),
        json_esc::esc(std::env::consts::OS),
        json_esc::esc(std::env::consts::ARCH),
        co_sqlite(),
        features
    );
}

/// Chỉ nhận đúng `[]` hoặc `["--json"]`; mọi tổ hợp khác là lỗi dùng (exit 2, stdout rỗng).
pub(crate) fn run_doctor(args: &[String]) -> ExitCode {
    match args {
        [] => {
            in_thuong();
            ExitCode::SUCCESS
        }
        [co] if co == "--json" => {
            in_json();
            ExitCode::SUCCESS
        }
        [co] if co.starts_with('-') => {
            eprintln!("vis doctor: không hỗ trợ cờ `{co}`");
            usage_doctor()
        }
        [arg] => {
            eprintln!("vis doctor: không hỗ trợ đối số `{arg}`");
            usage_doctor()
        }
        _ => {
            eprintln!("vis doctor: quá nhiều đối số (dùng `vis doctor [--json]`)");
            usage_doctor()
        }
    }
}
