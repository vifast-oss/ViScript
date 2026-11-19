// SPDX-License-Identifier: MIT OR Apache-2.0
//! ST-20 — **differential**: cùng một chương trình, chạy **interp** và **VM**, phải cho **cùng**
//! `stdout`, `exit`, và `(mã, dòng, cột)` của chẩn đoán.
//!
//! Không so `steps`: interp đếm bước ngữ nghĩa, VM đếm **lệnh** (khác đơn vị — VD-54 (3), Agent 0
//! đã duyệt).

use std::path::PathBuf;

use vis_bytecode::compile;
use vis_interp::RunOutput;

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

fn goc() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../")
}

fn tep_sem() -> Vec<PathBuf> {
    let dir = goc().join("test/sem");
    let mut out = Vec::new();
    for nhom in std::fs::read_dir(&dir).expect("đọc test/sem") {
        let nhom = nhom.expect("entry").path();
        if !nhom.is_dir() {
            continue;
        }
        for f in std::fs::read_dir(&nhom).expect("đọc nhóm") {
            let p = f.expect("entry").path();
            if p.extension()
                .map(|x| x == "vis" || x == "vish")
                .unwrap_or(false)
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// Chạy **interp**.
fn chay_interp(text: &str, mode: vis_syntax::Mode) -> Option<RunOutput> {
    let parsed = vis_syntax::parse(text, mode);
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    let lexed = vis_syntax::lex(text, mode);
    let h = ha_std(&parsed.cst, &lexed, text);
    if !h.is_clean() {
        return None;
    }
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    if !t.is_clean() {
        return None;
    }
    Some(vis_interp::run(text, &h.hir, &h.symbols, &h.types, &[]))
}

/// Chạy **VM**.
fn chay_vm(text: &str, mode: vis_syntax::Mode) -> Option<RunOutput> {
    let parsed = vis_syntax::parse(text, mode);
    if !parsed.diagnostics.is_empty() {
        return None;
    }
    let lexed = vis_syntax::lex(text, mode);
    let h = ha_std(&parsed.cst, &lexed, text);
    if !h.is_clean() {
        return None;
    }
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    if !t.is_clean() {
        return None;
    }
    let m = compile(&h.hir, &h.symbols, text).ok()?;
    Some(vis_vm::run(&m, text, &[]))
}

fn mo_ta(o: &RunOutput) -> String {
    let d: Vec<String> = o
        .diagnostics
        .iter()
        .map(|x| format!("{}:{}:{} {}", x.code_str(), x.line, x.col, x.message))
        .collect();
    format!("stdout={:?} exit={} diag={:?}", o.stdout, o.exit, d)
}

/// **Danh sách ca lệch phải LUÔN RỖNG** (ST-20b; ST-35: so **đủ** 346 ca, kể cả ca `.err` ⇒ 320/320). Nếu ai thêm ca vào đây,
/// test sẽ đỏ ngay (`so_biet == 0`) — mục đích là **không** cho nợ kỹ thuật quay lại.
const KNOWN_LECH: [&str; 0] = [];

#[test]
fn vm_khop_interp_tren_320_ca() {
    // VM đệ quy theo **khung bytecode** (cap 2.000) ⇒ cần stack lớn như `vis run` (256 MiB).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(than_chay)
        .expect("tạo luồng")
        .join()
        .expect("luồng lỗi");
}

fn than_chay() {
    let tat_ca = tep_sem();
    // **SỬA KÈM LÝ DO (ST-35)**: +8 ca `generic/*` (instantiation thật) ⇒ 346 ca (327 `.vis` + 19 `.vish`).
    // **SỬA KÈM LÝ DO (ST-56)**: +6 ca nợ lõi A ⇒ **376 ca** (333 `.vis` + 43 `.vish`).
    // **SỬA KÈM LÝ DO (ST-56b)**: +6 ca phép gán ⇒ **382 ca** (339 `.vis` + 43 `.vish`).
    // **SỬA KÈM LÝ DO (ST-57)**: +6 ca `trait/` ⇒ **388 ca** (345 `.vis` + 43 `.vish`).
    // **SỬA KÈM LÝ DO (ST-59)**: +6 ca `vish/` ⇒ **394 ca** (345 `.vis` + 49 `.vish`).
    // **SỬA KÈM LÝ DO (ST-60)**: +22 ca `async/` ⇒ **416 ca** (367 `.vis` + 49 `.vish`).
    // **SỬA KÈM LÝ DO (ST-72)**: +6 ca `cap/` ⇒ 447 ca.
    // **SỬA KÈM LÝ DO (R9, VD-178)**: +1 ca `loi/range-khong-int` ⇒ 448 ca.
    // **SỬA KÈM LÝ DO (R6, VD-178)**: +4 ca `chuoi/` ⇒ 452 ca.
    // **SỬA KÈM LÝ DO (R17, VD-178)**: +`dieu-khien/is-empty` ⇒ 453 ca.
    // **SỬA KÈM LÝ DO (R3b, VD-178)**: +`result/let-else` ⇒ 454 ca.
    // **SỬA KÈM LÝ DO (R3a, VD-178)**: +`result/if-let-pattern` ⇒ 455 ca.
    // **SỬA KÈM LÝ DO (R16, VD-178)**: +`pham-vi/che-ten-let` ⇒ 456 ca.
    // **SỬA KÈM LÝ DO (R12a, VD-178)**: +`kieu/type-alias-newtype` ⇒ 457 ca.
    // **SỬA KÈM LÝ DO (R11, VD-178)**: +`enum/vi-tri-va-dung-ten` ⇒ 458 ca.
    // **SỬA KÈM LÝ DO (R7, VD-178)**: +`so-hoc/chia-nguyen` ⇒ 459 ca.
    // **SỬA KÈM LÝ DO (R8, VD-178)**: +`list/chi-so-an-toan` ⇒ 460 ca.
    // **SỬA KÈM LÝ DO (R5a, VD-178)**: +`ham/goi-ten` ⇒ 461 ca.
    // **SỬA KÈM LÝ DO (R5c, VD-178)**: +`chuoi-phuong-thuc/std-method` ⇒ 462 ca.
    // **SỬA KÈM LÝ DO (R20a, VD-178)**: +`assert/thong-diep` ⇒ 463 ca.
    // **SỬA KÈM LÝ DO (R20b, VD-178)**: +`list/duyet-nang-cao` ⇒ 464 ca.
    // **SỬA KÈM LÝ DO (R19b, VD-178)**: +`loi/doi-dau-tran` (VS0507) ⇒ 465 ca.
    // **SỬA KÈM LÝ DO (R2, VD-178)**: +4 ca ⇒ **469 ca**.
    // **SỬA KÈM LÝ DO (VD-240-b, 2026-11-19)**: +4 ca grid ⇒ **473 ca**.
    // **SỬA KÈM LÝ DO (VD-241 §3, 2026-11-19)**: +10 ca Option lồng (9 xanh + 2 `.err` lỗi biên dịch,
    //   gồm thêm `long-param-b`, `long-return`, `long-match-nested`, `long-no-pseudo`) ⇒ **483 ca**.
    // **SỬA KÈM LÝ DO (VD-241 §7, 2026-11-19)**: +3 ca `optional/` (`none-three-states`,
    //   `long-none-param`, `long-none-return`); đổi `long-reject-e` (`.err`) → `long-none-e` hợp lệ
    //   ⇒ **486 ca** (**436** `.vis` + 50 `.vish`, **88 `.err`**).
    // **SỬA KÈM LÝ DO (VD-241 đợt coercion, 2026-11-19)**: +1 xanh `optional/long-option-matrix.vis`
    //   ⇒ **487 ca** (**437** `.vis` + 50 `.vish`, **88 `.err`**).
    // **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: +27 ca nhóm mới **`sqlite/`** (19 xanh +
    //   8 `.err` VS0501 — hai engine cùng đường trap ⇒ khớp y hệt) ⇒ **514 ca** (**464** `.vis`
    //   + 50 `.vish`, **96 `.err`**); sqlite/ **bỏ qua** ở harness này (27), còn lại (434 khớp
    //   + 53 lỗi biên dịch) = 487.
    // **SỬA KÈM LÝ DO (VD-241 pre-merge, 2026-11-19)**: +11 ca `sqlite/` (mọi lỗi dữ liệu nay là
    //   `Result.Err`) ⇒ **525 ca** (**475** `.vis` + 50 `.vish`, **88 `.err`**); sqlite/ bỏ qua (38).
    assert_eq!(tat_ca.len(), 525, "phải đủ 525 ca");
    let mut khop = 0;
    let mut lech: Vec<String> = Vec::new();
    let mut bien_dich = 0;
    // **VD-241 Gate 1**: ca `sqlite/` cần **DbHost native** (vis-cli cài rusqlite thật) — harness
    // in-process không gắn host nên hai đường đều `VS0604` (khớp nhưng rỗng nghĩa). Parity thật
    // của sqlite chạy qua `vis test test/sem` và `vis test --vm test/sem` (CLI có host) +
    // `crates/vis-cli/tests/sqlite.rs` (integration, hai engine byte-identical).
    let mut bo_qua_sqlite = 0;
    for p in &tat_ca {
        // **SỬA KÈM LÝ DO (ST-30b)**: **không** bỏ ca `.err` nữa — chẩn đoán lúc chạy của hai đường
        // phải **trùng đủ** `(mã, dòng, cột, thông điệp)`; trước đây bỏ qua nên lọt 6 ca lệch mã
        // (`VS0503` interp ↔ `VS0501` VM).
        if p.starts_with(goc().join("test/sem/sqlite").as_os_str()) {
            bo_qua_sqlite += 1;
            continue;
        }
        let text = std::fs::read_to_string(p).expect("đọc");
        let mode = if p.extension().map(|x| x == "vish").unwrap_or(false) {
            vis_syntax::Mode::Command
        } else {
            vis_syntax::Mode::Script
        };
        let (Some(a), Some(b)) = (chay_interp(&text, mode), chay_vm(&text, mode)) else {
            // **ST-32**: ca lỗi **biên dịch** — không chạy được ở đường nào, nhưng **đường bytecode**
            // vẫn phải được **chạm tới**: `compile()` không panic (codegen tin tyck ⇒ có thể ra module;
            // front end mới là cổng). Đếm riêng để khoá **đủ 346**.
            let lexed = vis_syntax::lex(&text, mode);
            let parsed = vis_syntax::parse(&text, mode);
            let h = ha_std(&parsed.cst, &lexed, text.as_str());
            let kq = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                compile(&h.hir, &h.symbols, &text)
            }));
            assert!(
                kq.is_ok(),
                "{}: `compile()` panic trên HIR lỗi",
                p.display()
            );
            bien_dich += 1;
            continue;
        };
        if mo_ta(&a) == mo_ta(&b) {
            khop += 1;
        } else {
            lech.push(format!(
                "{}: interp[{}] vm[{}]",
                p.strip_prefix(goc()).unwrap_or(p).display(),
                mo_ta(&a),
                mo_ta(&b)
            ));
        }
    }
    // Tách lệch thành "đã biết" và "mới".
    let mut moi: Vec<String> = Vec::new();
    let mut so_biet = 0;
    for l in &lech {
        let duong = l.split(':').next().unwrap_or("");
        if KNOWN_LECH.iter().any(|k| duong.ends_with(k)) {
            so_biet += 1;
        } else {
            moi.push(l.clone());
        }
    }
    assert!(
        moi.is_empty(),
        "{} lệch **mới** (ngoài danh sách đã biết):\n{}",
        moi.len(),
        moi.join("\n")
    );
    assert_eq!(
        so_biet, 0,
        "**không** còn ca lệch nào (danh sách phải rỗng, thật {})",
        so_biet
    );
    // **SỬA KÈM LÝ DO (ST-35)**: **0 ca bỏ qua** — 320 ca **chạy được** so interp ↔ VM, **26** ca lỗi
    // **biên dịch** đi qua **đường bytecode** (`compile()` không panic). Tổng **346**.
    // **SỬA KÈM LÝ DO (ST-89)**: 7 ca host nay khớp thật nhưng vẫn trong **398** — không đổi tổng.
    // **SỬA KÈM LÝ DO (ST-57)**: **388** ca; **356** ca chạy được khớp + **32** ca lỗi biên dịch.
    // **SỬA KÈM LÝ DO (ST-56)**: +6 ca ⇒ **376**; **348** ca chạy được khớp + **28** ca lỗi biên dịch.
    // **SỬA KÈM LÝ DO (ST-56b)**: +6 ca ⇒ **382**; **352** ca chạy được khớp + **30** ca lỗi biên dịch.
    // **SỬA KÈM LÝ DO (VD-241 §7, 2026-11-19)**: +3 ca xanh + đổi `long-reject-e` thành `long-none-e`
    //   (lỗi biên dịch → xanh) ⇒ **(433, 53) = 486**.
    // **SỬA KÈM LÝ DO (VD-241 đợt coercion, 2026-11-19)**: +1 ca xanh (ma trận) ⇒ **(434, 53) = 487**.
    // **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: +27 ca `sqlite/` (nhóm mới, cần DbHost native)
    //   ⇒ **514 ca**; sqlite/ **bỏ qua** (27), còn lại (434 khớp + 53 lỗi biên dịch) = 487.
    // **SỬA KÈM LÝ DO (VD-241 pre-merge, 2026-11-19)**: +11 ca ⇒ sqlite/ bỏ qua **38**, tổng **525**.
    assert_eq!(
        bo_qua_sqlite, 38,
        "phải đúng 38 ca sqlite/ bỏ qua (cần host)"
    );
    assert_eq!(
        khop + so_biet + bien_dich + bo_qua_sqlite,
        525,
        "525 ca (khớp {khop} + lệch {so_biet} + lỗi biên dịch {bien_dich} + sqlite bỏ qua {bo_qua_sqlite})"
    );
    // **SỬA KÈM LÝ DO (ST-72)**: +3 ca cap xanh, +3 ca cap lỗi biên dịch ⇒ (395,46) → (398,49).
    // **SỬA KÈM LÝ DO (R19b, VD-178)**: +`loi/doi-dau-tran` (VS0507, lỗi lúc chạy) ⇒ 465 ca.
    // **SỬA KÈM LÝ DO (R2, VD-178)**: +3 xanh (`optional/`) + `loi/none-bo` (SYNTAX, biên dịch) ⇒
    //   **419 .vis (418 khớp + 1 `none-bo` biên dịch) + 50 .vish lỗi biên dịch ⇒ (418, 51) = 469**.
    // **SỬA KÈM LÝ DO (VD-240-b, 2026-11-19)**: +4 ca grid `async/grid-*` (xanh) ⇒ **(422, 51) = 473**.
    // **SỬA KÈM LÝ DO (VD-241 §3, 2026-11-19)**: +7 ca Option lồng xanh + 3 `.err` (VS0412)
    //   ⇒ **(429, 54) = 483**.
    // **SỬA KÈM LÝ DO (VD-241 §7, 2026-11-19)**: +4 ca xanh (`long-none-e` từ `.err` + 3 ca mới)
    //   − 1 lỗi biên dịch ⇒ **(433, 53) = 486**.
    // **SỬA KÈM LÝ DO (VD-241 đợt coercion, 2026-11-19)**: +1 ca xanh ⇒ **(434, 53) = 487**.
    // **SỬA KÈM LÝ DO (VD-241 Gate 1, 2026-11-19)**: sqlite/ **bỏ qua** ⇒ vẫn **(434, 53) = 487** chạy được.
    assert_eq!(
        (khop, bien_dich),
        (434, 53),
        "434 ca chạy được khớp + 53 ca lỗi biên dịch chạm bytecode (sqlite/ bỏ qua 27), thật {khop}/{bien_dich}"
    );
}

/// **ST-30b**: hai đường phải cho **cùng SỐ** chẩn đoán (không chỉ cùng nội dung phần tử đầu) — quét
/// **toàn bộ** 346 ca, kể cả `.err`; ít nhất một ca có chẩn đoán để phép so không rỗng nghĩa.
#[test]
fn hai_duong_cung_so_chan_doan() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let tat_ca = tep_sem();
            let mut so_ca = 0;
            let mut so_ca_co_chan_doan = 0;
            for p in &tat_ca {
                // **VD-241 Gate 1**: sqlite/ cần DbHost native ⇒ hai đường đều `VS0604` ở harness
                // này (không có host) — bỏ qua; parity thật qua `vis test --vm test/sem`.
                if p.starts_with(goc().join("test/sem/sqlite").as_os_str()) {
                    continue;
                }
                let text = std::fs::read_to_string(p).expect("đọc");
                let mode = if p.extension().map(|x| x == "vish").unwrap_or(false) {
                    vis_syntax::Mode::Command
                } else {
                    vis_syntax::Mode::Script
                };
                let (Some(a), Some(b)) = (chay_interp(&text, mode), chay_vm(&text, mode)) else {
                    continue;
                };
                so_ca += 1;
                assert_eq!(
                    a.diagnostics.len(),
                    b.diagnostics.len(),
                    "{}: interp {} chẩn đoán, vm {}",
                    p.display(),
                    a.diagnostics.len(),
                    b.diagnostics.len()
                );
                for (x, y) in a.diagnostics.iter().zip(b.diagnostics.iter()) {
                    assert_eq!(
                        (x.code_str(), x.line, x.col, &x.message),
                        (y.code_str(), y.line, y.col, &y.message),
                        "{}",
                        p.display()
                    );
                }
                if !a.diagnostics.is_empty() {
                    so_ca_co_chan_doan += 1;
                }
            }
            // **SỬA KÈM LÝ DO (ST-35)**: 320 ca chạy được (26 ca lỗi biên dịch bị bỏ qua), trong đó
            // **30** ca có chẩn đoán lúc chạy (6 ca chỉ số/map + `.vish` không host + phạm vi) — mỗi ca
            // đúng **1** chẩn đoán ở **cả hai** đường.
            // **SỬA KÈM LÝ DO (ST-56)**: 344 → **348** ca chạy được; **31** ca có chẩn đoán lúc
            // chạy (+`ban-ghi/self-amp-thieu-truong` VS0501).
            // **SỬA KÈM LÝ DO (ST-56b)**: **352** ca chạy được (+4 ca xanh); chẩn đoán lúc chạy
            // vẫn **31** (2 ca `.err` mới là lỗi **biên dịch**, không tới lúc chạy).
            // **ST-89**: 398 ca chạy được **gồm 7 ca host** (nay khớp thật); tổng vẫn 398 + 49 lỗi biên dịch.
            // **R6 (VD-178)**: +4 ca `chuoi/` ⇒ **402** ca chạy được.
            // **R19b (VD-178)**: +1 ca `loi/doi-dau-tran` (VS0507, lỗi **lúc chạy**) ⇒ **415** ca chạy được.
            // **R2 (VD-178)**: +3 ca `optional/` chạy được ⇒ **418**.
            // **VD-240-b (2026-11-19)**: +4 ca grid `async/grid-*` ⇒ **422**.
            // **VD-241 §3 (2026-11-19)**: +7 ca Option lồng ⇒ **429**.
            // **VD-241 §7 (2026-11-19)**: +4 ca xanh ⇒ **433**.
            // **VD-241 đợt coercion (2026-11-19)**: +1 ca xanh (ma trận) ⇒ **434**.
            // **VD-241 Gate 1 (2026-11-19)**: sqlite/ **bỏ qua** (27 ca cần host) ⇒ vẫn **434**.
            assert_eq!(so_ca, 434, "phải so đủ 434 ca chạy được");
            // **SỬA KÈM LÝ DO (ST-60)**: 32 → **34** (+2 ca `VS0506` lúc chạy).
            // **SỬA KÈM LÝ DO (ST-61)**: 34 → **38** (+4 ca kênh: 3 `VS0505` + 1 `VS0504`).
            // **SỬA KÈM LÝ DO (R19b, VD-178)**: **38 → 39** (+`loi/doi-dau-tran` VS0507).
            assert_eq!(
                so_ca_co_chan_doan, 39,
                "39 ca có chẩn đoán lúc chạy (hai đường cùng số)"
            );
        })
        .expect("tạo luồng")
        .join()
        .expect("luồng lỗi");
}
