// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 QA** — fuzz `decode` + `verify` (không chạy VM) trên bytecode đột biến từ 437 module
//! `test/sem`. Bất biến: `decode`/`verify` không panic; vi phạm ⇒ ghi artifact (seed, ca, module,
//! bytes) vào `VBC_FUZZ_ART_DIR` (mặc định thư mục tạm) rồi in `VI_PHAM`.
//!
//! Chạy (mặc định bị `#[ignore]` vì dài):
//! ```text
//! VBC_DECODE_FUZZ_S=3600 VBC_FUZZ_SEED=20261226 VBC_FUZZ_ART_DIR=/tmp/opencode \
//!   cargo test --release -p vis-vm --test fuzz_decode_only -- --ignored --nocapture
//! ```

use std::panic::catch_unwind;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use vis_bytecode::{decode, encode, verify};

fn pha_std(text: &str) -> Vec<String> {
    let mut ds = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix("use ") else {
            continue;
        };
        let mut seg = rest.split('.').map(str::trim);
        if seg.next() != Some("std") {
            continue;
        }
        let m = seg.next().unwrap_or_default();
        if vis_types::stdlib::module_ho_tro(m) && !ds.contains(&m.to_string()) {
            ds.push(m.to_string());
        }
    }
    ds
}

fn ha_std(cst: &vis_syntax::Cst, lex: &vis_syntax::LexOutput, text: &str) -> vis_types::HirOutput {
    let opts = vis_types::LowerOptions {
        std_mods: pha_std(text),
        ..vis_types::LowerOptions::default()
    };
    vis_types::lower_with(cst, lex, opts)
}

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

fn goc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../")
}

/// Thư mục ghi artifact vi phạm — `VBC_FUZZ_ART_DIR`, mặc định thư mục tạm của HĐH.
fn thu_muc_art() -> PathBuf {
    std::env::var_os("VBC_FUZZ_ART_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

fn module_goc() -> Vec<(String, Vec<u8>)> {
    let mut ds = Vec::new();
    let dir = goc().join("test/sem");
    let mut tep = Vec::new();
    for nhom in std::fs::read_dir(&dir).expect("đọc test/sem") {
        let p = nhom.expect("entry").path();
        if !p.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&p).expect("đọc nhóm") {
            let q = f.expect("entry").path();
            if q.extension()
                .map(|x| x == "vis" || x == "vish")
                .unwrap_or(false)
                && !q.with_extension("err").exists()
            {
                tep.push(q);
            }
        }
    }
    tep.sort();
    for p in tep {
        let text = std::fs::read_to_string(&p).expect("đọc");
        let mode = if p.extension().map(|x| x == "vish").unwrap_or(false) {
            vis_syntax::Mode::Command
        } else {
            vis_syntax::Mode::Script
        };
        let parsed = vis_syntax::parse(&text, mode);
        let lexed = vis_syntax::lex(&text, mode);
        let h = ha_std(&parsed.cst, &lexed, &text);
        if let Ok(m) = vis_bytecode::compile(&h.hir, &h.symbols, &text) {
            ds.push((text, encode(&m)));
        }
    }
    ds
}

fn dot_bien(rng: &mut Lcg, goc: &[u8]) -> Vec<u8> {
    let mut b = goc.to_vec();
    match rng.duoi(4) {
        0 if !b.is_empty() => {
            let i = rng.duoi(b.len());
            b[i] ^= 1u8 << rng.duoi(8);
        }
        1 if !b.is_empty() => {
            let i = rng.duoi(b.len());
            b[i] = rng.next() as u8;
        }
        2 => {
            let n = rng.duoi(b.len());
            b.truncate(n);
        }
        _ => {
            let i = rng.duoi(b.len() + 1);
            let k = 1 + rng.duoi(8);
            let them: Vec<u8> = (0..k).map(|_| rng.next() as u8).collect();
            for (j, x) in them.into_iter().enumerate() {
                b.insert(i + j, x);
            }
        }
    }
    b
}

fn ghi_vi_pham(seed: u64, ca: u64, idx: usize, ten: &str, dot: &[u8], loai: &str) {
    let art = thu_muc_art();
    std::fs::create_dir_all(&art).ok();
    let _ = std::fs::write(art.join("st309-fuzz-decode-violation.bin"), dot);
    let _ = std::fs::write(
        art.join("st309-fuzz-decode-violation.txt"),
        format!(
            "seed={seed} ca={ca} idx={idx} module={ten} len={} loai={loai}\n",
            dot.len()
        ),
    );
    eprintln!("VI_PHAM seed={seed} ca={ca} module={ten} loai={loai} — đã ghi artifact");
}

#[test]
#[ignore = "fuzz dài; xem doc đầu tệp để chạy"]
fn fuzz_decode_verify() {
    let giay: u64 = std::env::var("VBC_DECODE_FUZZ_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    let seed: u64 = std::env::var("VBC_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20261226);
    let goc = module_goc();
    assert_eq!(goc.len(), 437, "phải đủ 437 module nguồn");
    let bat_dau = Instant::now();
    let han = Duration::from_secs(giay);
    let mut rng = Lcg(seed);
    let (mut tong, mut tu_choi_decode, mut tu_choi_verify, mut nhan, mut vi_pham) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    eprintln!("SEED={seed} giay={giay} module={}", goc.len());
    while bat_dau.elapsed() < han {
        let idx = rng.duoi(goc.len());
        let (ten, bytes) = &goc[idx];
        let dot = dot_bien(&mut rng, bytes);
        tong += 1;
        let m = match catch_unwind(|| decode(&dot)) {
            Ok(Ok(m)) => m,
            Ok(Err(_)) => {
                tu_choi_decode += 1;
                continue;
            }
            Err(_) => {
                vi_pham += 1;
                ghi_vi_pham(seed, tong, idx, ten, &dot, "decode PANIC");
                continue;
            }
        };
        match catch_unwind(std::panic::AssertUnwindSafe(|| verify(&m))) {
            Ok(Ok(())) => nhan += 1,
            Ok(Err(_)) => tu_choi_verify += 1,
            Err(_) => {
                vi_pham += 1;
                ghi_vi_pham(seed, tong, idx, ten, &dot, "verify PANIC");
            }
        }
        if tong % 5_000_000 == 0 {
            eprintln!(
                "TIEN_DO tong={tong} nhan={nhan} giay={:.1} ca/s={:.0}",
                bat_dau.elapsed().as_secs_f64(),
                tong as f64 / bat_dau.elapsed().as_secs_f64().max(0.001)
            );
        }
    }
    println!(
        "FUZZ_DECODE giay={giay} seed={seed} tong={tong} decode_tu_choi={tu_choi_decode} nhan={nhan} verify_tu_choi={tu_choi_verify} vi_pham={vi_pham} thuc={:.1}s toc_do={:.0}/s",
        bat_dau.elapsed().as_secs_f64(),
        tong as f64 / bat_dau.elapsed().as_secs_f64().max(0.001)
    );
    assert_eq!(vi_pham, 0, "phải **0** vi phạm, thật {vi_pham}");
}
