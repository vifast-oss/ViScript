// SPDX-License-Identifier: MIT OR Apache-2.0
//! **ST-290 (VD-273)** — ngân sách ngữ nghĩa **bật mặc định** `40M`, cờ CLI
//! `--max-budget`/`--unlimited-budget`, cache mang bảng phí và bytecode crafted **không né phí**.
//!
//! Hợp đồng:
//! 1. mặc định: interp charge ngay cả khi **không** bật test-hook; giới hạn phiên = `40M`;
//! 2. vượt ngân sách ⇒ `VS0510` (không phải `VS0501`/`VS0501-FUEL`) trên **cả hai** engine;
//! 3. `--unlimited-budget` bỏ giới hạn nhưng vẫn giữ `VM_SAFETY_FUEL = 100M`;
//! 4. cache VM lưu **cả** bảng phí: lần chạy ấm vẫn charge/trap đúng (không né bằng cache);
//! 5. bảng phí bị cắt/rỗng ⇒ `verify_fee` từ chối (bytecode crafted không né phí);
//! 6. 2M vòng, đệ quy sâu, nhiều task: parity interp↔VM dưới ngân sách mặc định.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use vis_runtime::rt::fee as ifee;
use vis_syntax::{lex, parse, Mode};

fn tien_trinh(src: &str) -> (vis_hir::Hir, vis_hir::Symbols, vis_hir::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into(), "str".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

const LOOP_100K: &str =
    "fn main():\n    var n = 0\n    for i in 0..100_000:\n        n = n + 1\n    print(n)\n";

/// Đường **frame/async** (interp frame + VM frame): phải charge cùng ngân sách, không né phí.
const ASYNC_LOOP_100K: &str = "async fn t() -> int:\n    var n = 0\n    for i in 0..100_000:\n        n = n + 1\n    n\nfn main():\n    print(await t())\n";

static DEM: AtomicUsize = AtomicUsize::new(0);

/// Tệp nguồn tạm **riêng** cho mỗi lần gọi (test chạy song song).
fn tep_tam(src: &str, duoi: &str) -> (PathBuf, PathBuf) {
    let n = DEM.fetch_add(1, Ordering::Relaxed);
    let goc = std::env::temp_dir().join(format!("vs3-budget-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&goc).expect("mkdir tạm");
    let tep = goc.join(format!("cau{duoi}"));
    let mut f = std::fs::File::create(&tep).expect("tạo tệp tạm");
    f.write_all(src.as_bytes()).expect("ghi tệp tạm");
    (goc, tep)
}

struct Kq {
    ma: i32,
    stdout: String,
    stderr: String,
}

/// Chạy `vis run` với `XDG_CACHE_HOME` riêng (tránh tranh cache giữa các test).
fn chay_vis(goc: &Path, tep: &Path, co: &[&str]) -> Kq {
    let cache = goc.join("cache");
    let o = Command::new(env!("CARGO_BIN_EXE_vis"))
        .arg("run")
        .args(co)
        .arg(tep)
        .env("XDG_CACHE_HOME", &cache)
        .output()
        .expect("chạy vis");
    Kq {
        ma: o.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

#[test]
fn mac_dinh_bat_ngan_sach_cho_interp_khong_can_hook() {
    // Trạng thái sạch: không hook ⇒ lần chạy kế tiếp nhận ngân sách phiên (mặc định 40M).
    ifee::disable_for_test();
    assert_eq!(ifee::gioi_han_phien(), Some(40_000_000), "mặc định 40M");

    let (hir, syms, tys) = tien_trinh(LOOP_100K);
    let out = vis_interp::run(LOOP_100K, &hir, &syms, &tys, &[]);
    assert_eq!(out.exit, 0, "{:?}", out.diagnostics);
    assert_eq!(out.stdout, "100000\n");
    let dung = ifee::da_dung_phien();
    assert!(dung > 0, "mặc định phải charge thật, dùng={dung}");
    assert!(dung < 40_000_000, "chương trình nhỏ không được chạm trần");
}

#[test]
fn cli_max_budget_trap_vs0510_ca_hai_engine_va_cache_am() {
    let (goc, tep) = tep_tam(LOOP_100K, ".vis");
    for co in [
        vec!["--max-budget", "1000"],
        vec!["--vm", "--max-budget", "1000"],
    ] {
        // Lần 1: biên dịch + ghi cache (VM) / chạy trực tiếp (interp).
        let a = chay_vis(&goc, &tep, &co);
        assert_eq!(a.ma, 101, "{co:?} lần 1: {a:?}", a = (a.ma, &a.stderr));
        assert!(
            a.stderr.contains("VS0510"),
            "{co:?} phải báo VS0510: {}",
            a.stderr
        );
        assert!(
            !a.stderr.contains("VS0501-FUEL"),
            "{co:?} không được báo fuel: {}",
            a.stderr
        );
        assert!(a.stdout.is_empty(), "trap trước print ⇒ stdout rỗng");
        // Lần 2 (VM): cache ấm mang bảng phí; ngân sách vẫn phải trap, không né bằng cache.
        let b = chay_vis(&goc, &tep, &co);
        assert_eq!(b.ma, 101, "{co:?} lần 2 (cache ấm)");
        assert!(b.stderr.contains("VS0510"), "{co:?} cache ấm: {}", b.stderr);
    }
}

#[test]
fn cli_duong_async_frame_cung_trap_vs0510() {
    let (goc, tep) = tep_tam(ASYNC_LOOP_100K, ".vis");
    for co in [
        vec!["--max-budget", "1000"],
        vec!["--vm", "--max-budget", "1000"],
    ] {
        let a = chay_vis(&goc, &tep, &co);
        assert_eq!(a.ma, 101, "{co:?} async: {}", a.stderr);
        assert!(a.stderr.contains("VS0510"), "{co:?}: {}", a.stderr);
        assert!(a.stdout.is_empty(), "{co:?}: trap trước print");
    }
}

#[test]
fn cli_unlimited_budget_bo_gioi_han_nhung_giu_fuel() {
    assert_eq!(vis_vm::vm::SAFETY_FUEL as u64, 100_000_000);
    let (goc, tep) = tep_tam(LOOP_100K, ".vis");
    for co in [
        vec!["--unlimited-budget"],
        vec!["--vm", "--unlimited-budget"],
    ] {
        let a = chay_vis(&goc, &tep, &co);
        assert_eq!(a.ma, 0, "{co:?}: {}", a.stderr);
        assert_eq!(a.stdout, "100000\n", "{co:?}");
    }
}

#[test]
fn cli_budget_sai_cach_dung_la_loi_dung_exit_2() {
    let (goc, tep) = tep_tam(LOOP_100K, ".vis");
    for (co, manh) in [
        (vec!["--max-budget", "0"], "--max-budget"),
        (vec!["--max-budget", "abc"], "--max-budget"),
        (vec!["--max-budget", "10", "--unlimited-budget"], "một"),
    ] {
        let a = chay_vis(&goc, &tep, &co);
        assert_eq!(a.ma, 2, "{co:?} phải là lỗi dùng: {}", a.stderr);
        assert!(a.stderr.contains(manh), "{co:?}: {}", a.stderr);
    }
}
