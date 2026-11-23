// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 QA** — decode + verify + disassemble một artifact bytecode (thường do `fuzz_watchdog`
//! ghi ra) để phân loại ca chậm mà không phải chạy VM. Không đặt `SLOWCASE` thì test tự bỏ qua.
//!
//! ```text
//! SLOWCASE=/tmp/opencode/st309-fuzz-vbc-slowcase2.bin \
//!   cargo test --release -p vis-vm --test dump_only -- --nocapture
//! ```
//!
//! `SLOWCASE_OUT` đổi tệp dump (mặc định `dump-slowcase.txt` trong thư mục tạm).

use std::path::PathBuf;

use vis_bytecode::{decode, need_stack, verify};

#[test]
fn dump_slowcase() {
    let Ok(path) = std::env::var("SLOWCASE") else {
        eprintln!("dump_only: bỏ qua — đặt SLOWCASE=<tệp .bin> để chạy");
        return;
    };
    let path = PathBuf::from(path);
    if !path.is_file() {
        eprintln!("dump_only: bỏ qua — không thấy {}", path.display());
        return;
    }
    let out = std::env::var_os("SLOWCASE_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("dump-slowcase.txt"));
    let bytes = std::fs::read(&path).expect("đọc artifact");
    let m = decode(&bytes).expect("decode");
    verify(&m).expect("verify");
    let need = m
        .funcs
        .iter()
        .enumerate()
        .map(|(i, f)| need_stack(&f.code, i as u32).unwrap_or(0) as usize)
        .max()
        .unwrap_or(0);
    let dump = vis_vm::dump::dump(&m);
    std::fs::write(&out, &dump).expect("ghi dump");
    println!(
        "MODULE bytes={} version={} consts={} funcs={} need={need} dump={}",
        bytes.len(),
        m.version,
        m.consts.len(),
        m.funcs.len(),
        out.display()
    );
}
