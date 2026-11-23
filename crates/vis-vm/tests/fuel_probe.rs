// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 QA** — đo thời gian chạy một artifact bytecode nghi chậm (thường do
//! `fuzz_watchdog` ghi ra). In `steps`, độ dài stdout, chẩn đoán và thời gian; hữu ích để xác nhận
//! **bậc hai** của đường back-edge/`loop_ends` khi fuel bị hạ tạm.
//!
//! Chạy (mặc định bị `#[ignore]`; **có thể chạy hàng giờ** ở fuel 100M):
//! ```text
//! SLOWCASE=/tmp/opencode/st309-fuzz-vbc-slowcase2.bin \
//!   cargo test --release -p vis-vm --test fuel_probe -- --ignored --nocapture
//! ```
//!
//! Để đo đường cong, hạ tạm `vis_bytecode::fee::VM_SAFETY_FUEL` (đã làm trong ST-309: 1M → 3,03 s;
//! 10M → 482,72 s; 100M → >31 phút) rồi chạy lại — **revert trước khi commit**.

use std::path::PathBuf;
use std::time::Instant;

use vis_bytecode::{decode, verify};

#[test]
#[ignore = "chạy VM trên artifact nghi chậm; đặt SLOWCASE để dùng"]
fn fuel_probe() {
    let Ok(path) = std::env::var("SLOWCASE") else {
        eprintln!("fuel_probe: bỏ qua — đặt SLOWCASE=<tệp .bin> để chạy");
        return;
    };
    let path = PathBuf::from(path);
    if !path.is_file() {
        eprintln!("fuel_probe: bỏ qua — không thấy {}", path.display());
        return;
    }
    let bytes = std::fs::read(&path).expect("đọc artifact");
    let m = decode(&bytes).expect("decode");
    verify(&m).expect("verify");
    println!("FUEL={}", vis_vm::vm::SAFETY_FUEL);
    let t = Instant::now();
    let mut vm = vis_vm::Vm::new(&m, "");
    let out = vm.run(&[]);
    let giay = t.elapsed().as_secs_f64();
    println!(
        "PROBE giay={giay:.2} steps={} stdout_len={} exit={} chan_doan={} max_stack={}",
        out.steps,
        out.stdout.len(),
        out.exit,
        out.diagnostics
            .first()
            .map(|d| format!("{} {}", d.code.short, d.message))
            .unwrap_or_default(),
        vm.max_stack_used()
    );
}
