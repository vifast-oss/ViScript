// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 (kiểm toán bảo mật)** — fuzz **decoder + verifier** bytecode, 0 panic trên dữ liệu
//! hỏng. Bổ sung cho `vis-vm::fuzz_mutate` (harness cũ chạy theo thời gian và có cả VM, ~5 nghìn
//! ca/phút ở debug); tệp này chạy **theo số ca** nên đo được mốc 1–2 triệu.
//!
//! Mặc định **20 000 ca** để cổng nhanh vẫn có phủ; chạy mốc kiểm toán bằng biến môi trường:
//!
//! ```text
//! VBC_FUZZ_CA=1500000 cargo test -p vis-bytecode --test st309_fuzz_verify -- --nocapture
//! ```
//!
//! Bất biến mỗi ca: `decode` không panic; nếu decode OK thì `verify` + `need_stack` không panic.
//! Không chạy VM ở đây (đã có `fuzz_mutate`); mục tiêu là số ca lớn trên hai bề mặt nhận dữ liệu
//! không tin cậy.

use std::panic::{catch_unwind, AssertUnwindSafe};

use vis_bytecode::{decode, encode, need_stack, verify};

/// Mẩu nguồn nhỏ, đa dạng opcode (số học, nhánh, vòng lặp, hàm, list/map, chuỗi).
const NGUON: &[&str] = &[
    "fn main() -> int:\n    1 + 2 * 3\n",
    "fn main() -> int:\n    let x = 0\n    let i = 0\n    while i < 10:\n        x = x + i\n        i = i + 1\n    x\n",
    "fn cong(a: int, b: int) -> int:\n    a + b\n\nfn main() -> int:\n    cong(2, 3)\n",
    "fn main() -> bool:\n    if 1 < 2:\n        true\n    else:\n        false\n",
    "fn main() -> list[int]:\n    [1, 2, 3]\n",
    "fn main() -> str:\n    \"xin chào\"\n",
    "fn f(n: int) -> int:\n    if n <= 1:\n        n\n    else:\n        n * f(n - 1)\n\nfn main() -> int:\n    f(5)\n",
    "fn main() -> int:\n    let m = map{\"a\": 1}\n    m[\"a\"]\n",
];

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn duoi(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn so_ca() -> u64 {
    std::env::var("VBC_FUZZ_CA")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(20_000)
}

fn seed() -> Vec<Vec<u8>> {
    let mut ra = Vec::new();
    for text in NGUON {
        let parsed = vis_syntax::parse(text, vis_syntax::Mode::Script);
        let lexed = vis_syntax::lex(text, vis_syntax::Mode::Script);
        let h = vis_types::lower(&parsed.cst, &lexed);
        if let Ok(m) = vis_bytecode::compile(&h.hir, &h.symbols, text) {
            ra.push(encode(&m));
        }
    }
    assert!(
        ra.len() >= 6,
        "phải dựng được ≥6 module mẫu, thật {}",
        ra.len()
    );
    ra
}

fn dot_bien(rng: &mut Lcg, goc: &[u8]) -> Vec<u8> {
    let mut b = goc.to_vec();
    // 1..3 đột biến mỗi ca (nhiều hơn harness cũ) để chạm cả header/độ dài/chỉ số.
    for _ in 0..1 + rng.duoi(3) {
        match rng.duoi(5) {
            0 if !b.is_empty() => {
                let i = rng.duoi(b.len());
                b[i] ^= 1u8 << rng.duoi(8);
            }
            1 if !b.is_empty() => {
                let i = rng.duoi(b.len());
                b[i] = rng.next() as u8;
            }
            2 => {
                let n = rng.duoi(b.len() + 1);
                b.truncate(n);
            }
            3 => {
                let i = rng.duoi(b.len() + 1);
                let k = 1 + rng.duoi(8);
                let them: Vec<u8> = (0..k).map(|_| rng.next() as u8).collect();
                for (j, x) in them.into_iter().enumerate() {
                    b.insert(i + j, x);
                }
            }
            _ if !b.is_empty() => {
                let i = rng.duoi(b.len());
                b.remove(i);
            }
            _ => {}
        }
    }
    b
}

#[test]
fn decoder_verifier_khong_panic() {
    let mau = seed();
    let tong = so_ca();
    let bat_dau = std::time::Instant::now();
    let mut rng = Lcg(0x05EE_D309_2026_1123);
    let (mut decode_ok, mut verify_ok, mut vi_pham) = (0u64, 0u64, 0u64);
    for _ in 0..tong {
        let goc = &mau[rng.duoi(mau.len())];
        let dot = dot_bien(&mut rng, goc);
        let m = match catch_unwind(AssertUnwindSafe(|| decode(&dot))) {
            Ok(Ok(m)) => {
                decode_ok += 1;
                m
            }
            Ok(Err(_)) => continue,
            Err(_) => {
                vi_pham += 1;
                continue;
            }
        };
        match catch_unwind(AssertUnwindSafe(|| {
            let kq = verify(&m);
            if kq.is_ok() {
                for (i, f) in m.funcs.iter().enumerate() {
                    let _ = need_stack(&f.code, i as u32);
                }
                true
            } else {
                false
            }
        })) {
            Ok(true) => verify_ok += 1,
            Ok(false) => {}
            Err(_) => vi_pham += 1,
        }
    }
    let giay = bat_dau.elapsed().as_secs_f64();
    println!(
        "ST309-FUZZ ca={tong} decode_ok={decode_ok} verify_ok={verify_ok} vi_pham={vi_pham} giay={giay:.2} ca_giay={:.0}",
        tong as f64 / giay.max(1e-9)
    );
    assert_eq!(vi_pham, 0, "decoder/verifier phải 0 panic, thật {vi_pham}");
    assert!(decode_ok > 0, "phải có ca decode được để phủ verifier");
    assert!(verify_ok > 0, "phải có ca verify xanh để phủ đường hợp lệ");
}
