// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-21 — **fuzz đột biến bytecode** (VD-52 #3, §17).
//!
//! Nguồn: **318 module** sinh từ ca sem không lỗi (ST-38: 355 ca − 52 `.err`); mỗi vòng: bit-flip / byte-flip / cắt cụt / chèn.
//! Bất biến (mọi vòng):
//! 1. `decode` **không panic**;
//! 2. `verify` **không panic**;
//! 3. nếu `verify` OK ⇒ VM chạy **không panic** (bắt bằng `catch_unwind`), **kết thúc** (cap 10^7
//!    lệnh), và **độ sâu ngăn xếp ≤ `need`** mà verifier tính (cộng dồn theo hàm vì VM dùng **một**
//!    ngăn xếp giá trị cho mọi khung).
//!
//! Chạy 10 phút: `VBC_FUZZ_PHUT=10 cargo test -p vis-vm --test fuzz_mutate -- --nocapture`.

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;

use vis_bytecode::{decode, encode, need_stack, verify, Module};

/// **VD-241 Gate 1** — hạ một tệp với `std_mods` quét từ chính nguồn (`use std.<tên>`), đúng
/// cách `vis-cli/src/modules.rs` làm: nhờ vậy test/sem chạy được ca `use std.*` (trước Gate 1
/// không có ca nào). Chỉ nhận module std hợp lệ (`vis_types::stdlib::module_ho_tro`).
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

/// Như [`vis_types::lower`] nhưng nạp `std_mods` quét được (mặc định Permissive giữ nguyên).
fn ha_std(cst: &vis_syntax::Cst, lex: &vis_syntax::LexOutput, text: &str) -> vis_types::HirOutput {
    let opts = vis_types::LowerOptions {
        std_mods: pha_std(text),
        ..vis_types::LowerOptions::default()
    };
    vis_types::lower_with(cst, lex, opts)
}

/// Số phút fuzz (mặc định ngắn để cổng nhanh; 10 phút cho log ST-21).
fn so_phut() -> f64 {
    std::env::var("VBC_FUZZ_PHUT")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.02)
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

/// 318 module sinh từ ca sem không `.err` (đúng bộ với differential; ST-41: 370 − 52).
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

/// `need` của module = **max** theo hàm (VM cho **mỗi khung một** ngăn xếp giá trị).
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
        // bit-flip
        0 if !b.is_empty() => {
            let i = rng.duoi(b.len());
            b[i] ^= 1u8 << rng.duoi(8);
        }
        // byte-flip
        1 if !b.is_empty() => {
            let i = rng.duoi(b.len());
            b[i] = rng.next() as u8;
        }
        // cắt cụt
        2 => {
            let n = rng.duoi(b.len());
            b.truncate(n);
        }
        // chèn 1..8 byte
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

#[test]
fn fuzz_dot_bien_bytecode() {
    // VM đệ quy theo khung (cap 2.000) ⇒ cần stack lớn như `vis run`.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(than_fuzz)
        .expect("tạo luồng")
        .join()
        .expect("luồng lỗi");
}

fn than_fuzz() {
    let goc = module_goc();
    // **SỬA KÈM LÝ DO (ST-56)**: 318 → **321** module nguồn (376 ca − 55 `.err`).
    // **SỬA KÈM LÝ DO (ST-56b)**: **325** module nguồn (382 ca − 57 `.err`).
    // **SỬA KÈM LÝ DO (ST-57)**: **329** module nguồn (388 ca − 59 `.err`).
    // **SỬA KÈM LÝ DO (ST-72)**: **364** module nguồn (447 ca − 83 `.err`).
    // **SỬA KÈM LÝ DO (R6, VD-178)**: **368** module nguồn (452 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R17, VD-178)**: **369** module nguồn (453 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R3b, VD-178)**: **370** module nguồn (454 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R3a, VD-178)**: **371** module nguồn (455 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R16, VD-178)**: **372** module nguồn (456 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R12a, VD-178)**: **373** module nguồn (457 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R11, VD-178)**: **374** module nguồn (458 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R7, VD-178)**: **375** module nguồn (459 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R8, VD-178)**: **376** module nguồn (460 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R5a, VD-178)**: **377** module nguồn (461 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R5c, VD-178)**: **378** module nguồn (462 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R20a, VD-178)**: **379** module nguồn (463 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R20b, VD-178)**: **380** module nguồn (464 ca − 84 `.err`).
    // **SỬA KÈM LÝ DO (R19b, VD-178)**: **380** module nguồn (465 ca − 85 `.err`).
    // **SỬA KÈM LÝ DO (R2, VD-178)**: **383** module nguồn (469 ca − 86 `.err`).
    // **SỬA KÈM LÝ DO (VD-240-b, 2026-11-19)**: **387** (473 ca − 86 `.err`, +4 xanh `async/grid-*`).
    // **SỬA KÈM LÝ DO (VD-241 §3, 2026-11-19)**: **394** (483 ca − 89 `.err`, +7 xanh `optional/long-*`/`ban-ghi/field-optional`).
    // **SỬA KÈM LÝ DO (VD-241 §7, 2026-11-19)**: **398** (486 ca − 88 `.err`: `long-reject-e` rời nhóm
    //   lỗi biên dịch, +4 xanh: `long-none-e` + `none-three-states` + `long-none-param` + `long-none-return`).
    // **SỬA KÈM LÝ DO (VD-241 đợt coercion, 2026-11-19)**: **399** (487 ca − 88 `.err`,
    //   +1 xanh `optional/long-option-matrix.vis`).
    // **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: **418** (514 ca − 96 `.err`: +19 `sqlite/*` xanh).
    // **SỬA KÈM LÝ DO (VD-241 pre-merge, 2026-11-19)**: **437** (525 ca − 88 `.err`: +11 `sqlite/*` xanh, 8 `.err`→`.out`).
    assert_eq!(
        goc.len(),
        437,
        "phải đủ 437 module nguồn (ST-41: 525 ca − 88 `.err`)"
    );
    let phut = so_phut();
    let bat_dau = std::time::Instant::now();
    let han = std::time::Duration::from_secs_f64(phut * 60.0);
    let mut rng = Lcg(0x5EED_2026_1121);
    let (mut tong, mut tu_choi_decode, mut tu_choi_verify, mut chay_duoc, mut vi_pham) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut chi_tiet: Vec<String> = Vec::new();
    while bat_dau.elapsed() < han {
        let (ten, bytes) = &goc[rng.duoi(goc.len())];
        let dot = dot_bien(&mut rng, bytes);
        tong += 1;
        // 1) decode không panic
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
        // 2) verify không panic
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
        // 3) verify OK ⇒ VM không panic + không vượt `need`
        let need = need_cua_module(&m);
        let kq = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let mut vm = vis_vm::Vm::new(&m, "");
            let out = vm.run(&[]);
            (out.steps, vm.max_stack_used())
        }));
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
    }
    println!("FUZZ phut={phut} tong={tong} decode_tu_choi={tu_choi_decode} verify_tu_choi={tu_choi_verify} chay_duoc={chay_duoc} vi_pham={vi_pham} giay={:.1}",
        bat_dau.elapsed().as_secs_f64());
    for l in chi_tiet.iter().take(10) {
        println!("VI_PHAM {l}");
    }
    assert_eq!(vi_pham, 0, "phải **0** vi phạm, thật {vi_pham}");
}
