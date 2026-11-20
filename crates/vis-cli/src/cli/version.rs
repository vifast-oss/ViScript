// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-270** — `vis version [--json]`: in phiên bản `vis` (một dòng) hoặc object JSON.

use std::process::ExitCode;

pub(crate) fn run(args: &[String]) -> ExitCode {
    for a in args {
        if a != "--json" {
            eprintln!("vis version: không hỗ trợ đối số `{a}`");
            eprintln!("dùng: vis version [--json]");
            return ExitCode::from(2);
        }
    }
    if args.iter().any(|a| a == "--json") {
        println!(
            "{{\"name\":\"vis\",\"version\":\"{}\"}}",
            env!("CARGO_PKG_VERSION")
        );
    } else {
        println!("vis {}", env!("CARGO_PKG_VERSION"));
    }
    ExitCode::SUCCESS
}
