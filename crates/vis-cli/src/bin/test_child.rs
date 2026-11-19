// SPDX-License-Identifier: MIT OR Apache-2.0
// test_child.rs — Giải đáp: ỷ ước lượng test binary
//
// Mục tiêu VD-PO-Windows-portability (2026-11-19):
//   - Sub-lệnh echo / cat / grep / printf / true / false / sleep / sh (compat cho các test on-Windows)
//
// Không phải binary chính `vis`; chỉ tồn tại cho các test ngoài.

use std::env;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("test-child: cần ít nhất 1 lệnh");
        exit(127);
    }
    let cmd = args[1].as_str();
    let rest = &args[2..];
    let rc = match cmd {
        "echo" => {
            println!("{}", rest.join(" "));
            0
        }
        "echo-no-nl" => {
            print!("{}", rest.join(" "));
            io::stdout().flush().unwrap_or(());
            0
        }
        "cat" => {
            if rest.is_empty() {
                let stdin = io::stdin();
                let mut handle = stdin.lock();
                let mut line = String::new();
                while handle.read_line(&mut line).unwrap_or(0) > 0 {
                    print!("{line}");
                    line.clear();
                }
                0
            } else if rest.len() == 1 {
                match File::open(&rest[0]) {
                    Ok(mut f) => {
                        let mut s = String::new();
                        use std::io::Read;
                        let _ = f.read_to_string(&mut s);
                        print!("{s}");
                        0
                    }
                    Err(_) => {
                        eprintln!("test-child: không mở được `{}`", &rest[0]);
                        1
                    }
                }
            } else {
                eprintln!("test-child cat: chỉ hỗ trợ 0 hoặc 1 file");
                1
            }
        }
        "grep" => {
            // grep <pattern> [file] — pattern literal substring match (không regex)
            if rest.is_empty() {
                eprintln!("test-child grep: cần pattern");
                return; // err
            }
            let pat = &rest[0];
            let read_to_string = |p: Option<&str>| -> Result<String, ()> {
                if let Some(p) = p {
                    File::open(p).map(|mut f| {
                        let mut s = String::new();
                        use std::io::Read;
                        let _ = f.read_to_string(&mut s);
                        s
                    }).map_err(|_| ())
                } else {
                    let stdin = io::stdin();
                    let mut s = String::new();
                    let mut handle = stdin.lock();
                    use std::io::Read;
                    let _ = handle.read_to_string(&mut s);
                    Ok(s)
                }
            };
            match read_to_string(rest.get(1).map(String::as_str)) {
                Ok(s) => {
                    let mut matched = false;
                    for line in s.lines() {
                        if line.contains(pat) {
                            println!("{line}");
                            matched = true;
                        }
                    }
                    if matched { 0 } else { 1 }
                }
                Err(()) => 1,
            }
        }
        "true" => 0,
        "false" => 1,
        "printf" => {
            // printf <fmt> [args…] — minimal: hỗ trợ %s, %d, %f, %%.
            let mut out = format!("");
            let fmt = match rest.first() {
                Some(s) => s.clone(),
                None => {
                    eprintln!("test-child printf: thiếu format");
                    exit(127);
                }
            };
            let args = &rest[1..];
            let mut ai = 0;
            let mut i = 0;
            let bs = fmt.as_bytes();
            while i < bs.len() {
                let c = bs[i] as char;
                if c == '%' && i + 1 < bs.len() {
                    let spec = bs[i + 1] as char;
                    match spec {
                        's' => {
                            let v = args.get(ai).cloned().unwrap_or_default();
                            ai += 1;
                            out.push_str(&v);
                        }
                        'd' => {
                            let v: i64 = args.get(ai).and_then(|s| s.parse().ok()).unwrap_or(0);
                            ai += 1;
                            out.push_str(&v.to_string());
                        }
                        'f' => {
                            let v: f64 = args.get(ai).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                            ai += 1;
                            out.push_str(&v.to_string());
                        }
                        '%' => { out.push('%'); }
                        _ => {
                            out.push('%');
                            out.push(spec);
                        }
                    }
                    i += 2;
                    continue;
                }
                out.push(c);
                i += 1;
            }
            print!("{out}");
            io::stdout().flush().unwrap_or(());
            0
        }
        "sleep" => {
            let ms: u64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            std::thread::sleep(std::time::Duration::from_millis(ms));
            0
        }
        "sh" => {
            // sh -c "exit N" - return bat
            if rest.len() >= 2 && rest[0] == "-c" {
                let body = &rest[1];
                // Very simple: "exit N"
                if let Some(n_str) = body.trim().strip_prefix("exit ") {
                    n_str.trim().parse::<i32>().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        }
        other => {
            eprintln!("test-child: lệnh `{other}` chưa hỗ trợ");
            127
        }
    };
    exit(rc);
}
