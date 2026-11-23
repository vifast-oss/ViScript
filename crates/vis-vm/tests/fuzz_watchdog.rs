// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-309 QA** — fuzz đột biến bytecode có **watchdog** theo từng ca: phát lại dãy RNG của
//! `fuzz_mutate`, khi một ca chạy quá `VBC_FUZZ_WATCH_S` giây thì ghi module đột biến ra
//! `VBC_FUZZ_ART_DIR` (mặc định thư mục tạm) rồi thoát code 42 — dùng để bắt artifact ca chậm
//! mà harness gốc không in ra.
//!
//! Chạy (mặc định bị `#[ignore]` vì dài):
//! ```text
//! VBC_FUZZ_PHUT=60 VBC_FUZZ_WATCH_S=300 VBC_FUZZ_SEED=20261226 VBC_FUZZ_ART_DIR=/tmp/opencode \
//!   cargo test --release -p vis-vm --test fuzz_watchdog -- --ignored --nocapture
//! ```
//!
//! Bất biến giữ nguyên như `fuzz_mutate`: `decode`/`verify` không panic; verify OK ⇒ VM không panic,
//! kết thúc, và độ sâu ngăn xếp ≤ `need`. Thêm: in tiến độ mỗi 200.000 ca; seed đọc từ env.

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use vis_bytecode::{decode, encode, need_stack, verify, Module};

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

#[derive(Clone, Copy)]
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

/// Thư mục ghi artifact ca chậm — `VBC_FUZZ_ART_DIR`, mặc định thư mục tạm của HĐH.
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

fn need_cua_module(m: &Module) -> usize {
    m.funcs
        .iter()
        .enumerate()
        .map(|(i, f)| need_stack(&f.code, i as u32).unwrap_or(0) as usize)
        .max()
        .unwrap_or(0)
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

/// (chỉ số module, trạng thái RNG ngay trước `dot_bien`, số thứ tự ca)
#[derive(Clone, Copy)]
struct CaDangChay {
    idx: usize,
    rng: u64,
    ca: u64,
}

#[test]
#[ignore = "fuzz dài + watchdog; xem doc đầu tệp để chạy"]
fn watchdog_fuzz() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(than_fuzz)
        .expect("tạo luồng")
        .join()
        .expect("luồng lỗi");
}

fn than_fuzz() {
    let goc = Arc::new(module_goc());
    assert_eq!(goc.len(), 437, "phải đủ 437 module nguồn");
    let phut: f64 = std::env::var("VBC_FUZZ_PHUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60.0);
    let watch_s: u64 = std::env::var("VBC_FUZZ_WATCH_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);

    let art = thu_muc_art();
    std::fs::create_dir_all(&art).ok();
    let cur: Arc<Mutex<Option<CaDangChay>>> = Arc::new(Mutex::new(None));
    let tick = Arc::new(AtomicU64::new(0));
    {
        let cur = Arc::clone(&cur);
        let tick = Arc::clone(&tick);
        let goc = Arc::clone(&goc);
        let art = art.clone();
        std::thread::spawn(move || {
            let mut last = 0u64;
            let mut doi = Instant::now();
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let t = tick.load(Ordering::Acquire);
                if t != last {
                    last = t;
                    doi = Instant::now();
                    continue;
                }
                if doi.elapsed().as_secs() < watch_s {
                    continue;
                }
                let ca = *cur.lock().unwrap();
                if let Some(c) = ca {
                    let (ten, bytes) = &goc[c.idx];
                    let mut r = Lcg(c.rng);
                    let dot = dot_bien(&mut r, bytes);
                    let _ = std::fs::write(art.join("st309-fuzz-vbc-slowcase.bin"), &dot);
                    let _ = std::fs::write(
                        art.join("st309-fuzz-vbc-slowcase.txt"),
                        format!(
                            "ca={} module={} module_len={} dot_len={} watch_s={} tick={}\n",
                            c.ca,
                            ten,
                            bytes.len(),
                            dot.len(),
                            watch_s,
                            t
                        ),
                    );
                    eprintln!(
                        "WATCHDOG ca={} module={} dot_len={} > {} s — đã ghi artifact",
                        c.ca,
                        ten,
                        dot.len(),
                        watch_s
                    );
                } else {
                    eprintln!(
                        "WATCHDOG tick={t} không đổi > {watch_s} s nhưng không có ca đang chạy"
                    );
                }
                std::process::exit(42);
            }
        });
    }

    let bat_dau = Instant::now();
    let han = Duration::from_secs_f64(phut * 60.0);
    let seed: u64 = std::env::var("VBC_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0x5EED_2026_1121);
    eprintln!("SEED={seed} phut={phut} watch_s={watch_s}");
    let mut rng = Lcg(seed);
    let (mut tong, mut tu_choi_decode, mut tu_choi_verify, mut chay_duoc, mut vi_pham) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut chi_tiet: Vec<String> = Vec::new();
    while bat_dau.elapsed() < han {
        let idx = rng.duoi(goc.len());
        let rng_truoc = rng.0;
        let (ten, bytes) = &goc[idx];
        let dot = dot_bien(&mut rng, bytes);
        tong += 1;
        tick.store(tong, Ordering::Release);
        let m = match std::panic::catch_unwind(|| decode(&dot)) {
            Ok(Ok(m)) => m,
            Ok(Err(_)) => {
                tu_choi_decode += 1;
                continue;
            }
            Err(_) => {
                vi_pham += 1;
                chi_tiet.push(format!("{ten}: decode PANIC"));
                continue;
            }
        };
        match std::panic::catch_unwind(AssertUnwindSafe(|| verify(&m))) {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                tu_choi_verify += 1;
                continue;
            }
            Err(_) => {
                vi_pham += 1;
                chi_tiet.push(format!("{ten}: verify PANIC"));
                continue;
            }
        }
        let need = need_cua_module(&m);
        *cur.lock().unwrap() = Some(CaDangChay {
            idx,
            rng: rng_truoc,
            ca: tong,
        });
        let kq = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut vm = vis_vm::Vm::new(&m, "");
            let out = vm.run(&[]);
            (out.steps, vm.max_stack_used())
        }));
        *cur.lock().unwrap() = None;
        match kq {
            Ok((buoc, dinh)) => {
                chay_duoc += 1;
                if dinh > need {
                    vi_pham += 1;
                    chi_tiet.push(format!(
                        "{ten}: ngăn xếp {dinh} > need {need} (steps {buoc})"
                    ));
                }
            }
            Err(_) => {
                vi_pham += 1;
                chi_tiet.push(format!("{ten}: VM PANIC"));
            }
        }
        if tong % 200_000 == 0 {
            eprintln!(
                "TIEN_DO tong={tong} giay={:.1} ca/s={:.0}",
                bat_dau.elapsed().as_secs_f64(),
                tong as f64 / bat_dau.elapsed().as_secs_f64().max(0.001)
            );
        }
    }
    println!(
        "FUZZ phut={phut} tong={tong} decode_tu_choi={tu_choi_decode} verify_tu_choi={tu_choi_verify} chay_duoc={chay_duoc} vi_pham={vi_pham} giay={:.1}",
        bat_dau.elapsed().as_secs_f64()
    );
    for l in chi_tiet.iter().take(10) {
        println!("VI_PHAM {l}");
    }
    assert_eq!(vi_pham, 0, "phải **0** vi phạm, thật {vi_pham}");
}
