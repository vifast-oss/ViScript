// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-303 (VD-283)** — `vis repl`: phiên nhỏ dùng `vis-loader` + `vis-interp`.
//!
//! Mỗi dòng nhập được thử trên **bản nháp** gồm các dòng đã commit; chỉ commit khi nạp +
//! kiểm kiểu + chạy sạch. Lượt lỗi in chẩn đoán ra stderr và **không** làm mất trạng thái trước.
//! Lệnh phiên: `:help`, `:type <expr>`, `:reset`, `:quit`.

use std::io::{self, BufRead, Write};
use std::process::ExitCode;

use vis_loader::modules;
use vis_syntax::Diagnostic;
use vis_types::{check, Type};

fn in_diag(d: &Diagnostic) {
    eprintln!("{}: {}: {}", d.code.short, d.line, d.message);
}

fn in_cac_diag(ds: &[(String, Diagnostic)]) {
    for (f, d) in ds {
        eprintln!("{f}:{}:{}: {}: {}", d.line, d.col, d.code.short, d.message);
    }
}

fn nap_va_kiem(src: &str) -> Option<(modules::Loaded, vis_types::CheckOutput)> {
    let loaded = match modules::load_full_stdin(src, true, false) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("vis repl: {e}");
            return None;
        }
    };
    if !loaded.diags.is_empty() {
        in_cac_diag(&loaded.diags);
        return None;
    }
    let checked = check(&loaded.hir, &loaded.symbols, &loaded.types);
    if !checked.is_clean() {
        for d in &checked.diags {
            in_diag(d);
        }
        return None;
    }
    Some((loaded, checked))
}

fn help() {
    println!(":help          trợ giúp");
    println!(":type <expr>   in kiểu tĩnh của biểu thức");
    println!(":reset         xoá trạng thái phiên");
    println!(":quit          thoát");
}

fn go() -> ExitCode {
    let stdin = io::stdin();
    let mut committed = String::new();
    let mut committed_out = String::new();
    eprintln!("vis repl — gõ :help để xem lệnh");
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            eprintln!("vis repl: lỗi đọc stdin");
            return ExitCode::from(2);
        };
        let line = line.trim_end_matches('\r');
        if let Some(cmd) = line.trim().strip_prefix(':') {
            let (ten, rest) = cmd.split_once(char::is_whitespace).unwrap_or((cmd, ""));
            match ten {
                "quit" | "q" => return ExitCode::SUCCESS,
                "help" | "h" => help(),
                "reset" => {
                    committed.clear();
                    committed_out.clear();
                    println!("(đã xoá trạng thái)");
                }
                "type" | "t" => {
                    let expr = rest.trim();
                    if expr.is_empty() {
                        eprintln!("vis repl: :type cần biểu thức");
                        continue;
                    }
                    let src = format!("{committed}let __repl_type = {expr}\n");
                    if let Some((_l, c)) = nap_va_kiem(&src) {
                        let ty = c.types_of_expr.last().cloned().unwrap_or(Type::Unit);
                        println!("{ty}");
                    }
                }
                _ => eprintln!("vis repl: lệnh lạ `:{ten}` (dùng :help)"),
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let candidate = format!("{committed}{line}\n");
        let Some((loaded, _checked)) = nap_va_kiem(&candidate) else {
            continue;
        };
        let out = vis_interp::run(&candidate, &loaded.hir, &loaded.symbols, &loaded.types, &[]);
        if !out.is_clean() || out.exit != 0 {
            if out.is_clean() {
                eprintln!("vis repl: chương trình thoát {}", out.exit);
            } else {
                for d in &out.diagnostics {
                    in_diag(d);
                }
            }
            continue;
        }
        let extra = out
            .stdout
            .strip_prefix(committed_out.as_str())
            .unwrap_or(out.stdout.as_str());
        print!("{extra}");
        let _ = io::stdout().flush();
        committed = candidate;
        committed_out = out.stdout;
    }
    ExitCode::SUCCESS
}

pub(crate) fn run(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("vis repl: không hỗ trợ đối số");
        eprintln!("dùng: vis repl");
        return ExitCode::from(2);
    }
    go()
}
