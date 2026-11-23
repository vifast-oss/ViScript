// SPDX-License-Identifier: MIT OR Apache-2.0
//! Plugin WASI mẫu cho ST-307 — WASI command `wasm32-wasip1`, ABI `VIS_PLUGIN_ABI_V1`.
//!
//! Không phụ thuộc crate nào (chỉ `std`), để `rustc --target wasm32-wasip1` dựng trực tiếp.
//! Quyền chỉ đến từ preopen host trao (`/cap0`, `/cap1`, …); plugin không đọc env, không mở
//! đường ngoài preopen. Giao thức: xem `crates/vis-plugin-host/src/wasi/protocol.rs`.

use std::io::Write;

/// Marker ABI host kiểm trên byte wasm trước khi chạy.
#[no_mangle]
pub extern "C" fn vis_plugin_abi_v1() -> u32 {
    1
}

fn giai_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let b = s.as_bytes();
    let mut ra = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        ra.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(ra)
}

fn ma_hex(b: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push(HEX[(x >> 4) as usize] as char);
        s.push(HEX[(x & 0x0f) as usize] as char);
    }
    s
}

/// Giải một tham số codec (`i:5:` / `s:<n>:<hex>`); trả chuỗi thô để hàm tự ép kiểu.
fn doc_tham_so(s: &str) -> Option<String> {
    if let Some(rest) = s.strip_prefix("i:") {
        return Some(rest.strip_suffix(':')?.to_string());
    }
    if let Some(rest) = s.strip_prefix("s:") {
        let (n, hex) = rest.split_once(':')?;
        let n: usize = n.parse().ok()?;
        let b = giai_hex(hex)?;
        if b.len() != n {
            return None;
        }
        return String::from_utf8(b).ok();
    }
    None
}

fn ok(v: &str) {
    println!("ok");
    println!("{v}");
}

fn err(code: &str, msg: &str) {
    println!("err");
    println!("{code}");
    println!("{}", ma_hex(msg.as_bytes()));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str).unwrap_or("");
    match mode {
        "abi" => println!("VIS_PLUGIN_ABI_V1=1"),
        "manifest" => {
            println!("module=mau");
            println!("abi=1");
            println!("func=cong;int,int;int");
            println!("func=doc;str;str");
            println!("func=ghi;str,str;unit");
            println!("cap=Dir;ro");
            println!("ok");
        }
        "call" => {
            let ten = args.get(2).map(String::as_str).unwrap_or("");
            match ten {
                "cong" => {
                    let a = args
                        .get(3)
                        .and_then(|s| doc_tham_so(s))
                        .and_then(|s| s.parse::<i64>().ok());
                    let b = args
                        .get(4)
                        .and_then(|s| doc_tham_so(s))
                        .and_then(|s| s.parse::<i64>().ok());
                    match (a, b) {
                        (Some(a), Some(b)) => ok(&format!("i:{}:", a + b)),
                        _ => err("VS0501", "cong cần hai int"),
                    }
                }
                "doc" => match args.get(3).and_then(|s| doc_tham_so(s)) {
                    Some(p) => match std::fs::read_to_string(&p) {
                        Ok(s) => ok(&format!("s:{}:{}", s.len(), ma_hex(s.as_bytes()))),
                        Err(e) => err("VS0604", &format!("không đọc được `{p}`: {e}")),
                    },
                    None => err("VS0501", "doc cần str"),
                },
                "ghi" => {
                    let p = args.get(3).and_then(|s| doc_tham_so(s));
                    let n = args.get(4).and_then(|s| doc_tham_so(s));
                    match (p, n) {
                        (Some(p), Some(n)) => match std::fs::write(&p, n.as_bytes()) {
                            Ok(()) => ok("u:"),
                            Err(e) => err("VS0604", &format!("không ghi được `{p}`: {e}")),
                        },
                        _ => err("VS0501", "ghi cần str, str"),
                    }
                }
                _ => err("VS0501", &format!("hàm lạ `{ten}`")),
            }
        }
        _ => {
            let _ = std::io::stderr().write_all(b"usage: mau abi|manifest|call <func> [args]\n");
            std::process::exit(2);
        }
    }
}
