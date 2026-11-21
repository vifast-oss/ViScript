// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VS3-1d** — flip default cho chương trình **có `async fn`** sang frame scheduler.
//!
//! Hợp đồng:
//! - chương trình sync (`không` có `async fn`) đi nguyên `Interp::run`/`Vm::run` — byte-identical;
//! - chương trình async đi `run_async_with_*` (frame scheduler) khi HIR có `async fn`;
//! - `chan(0)` là dấu hiệu phân biệt: runtime sync từ chối, async default chạy được;
//! - `Pipe` đã parity trên VM default; frame async cũng chạy;
//! - cache `--vm` warm/cold giữ cùng stdout/exit.
//!
//! Không đổi bytecode/VERSION/cache key: chỉ chọn entry runtime.

use std::path::{Path, PathBuf};
use std::process::Command;

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig};
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::run_frame_for_test as run_v;

fn tien_trinh(src: &str) -> (vis_types::Hir, vis_types::Symbols, vis_types::TypeTable) {
    let p = parse(src, Mode::Script);
    assert!(p.diagnostics.is_empty(), "parse: {:?}", p.diagnostics);
    let l = lex(src, Mode::Script);
    let h = vis_types::lower_with(
        &p.cst,
        &l,
        vis_types::LowerOptions {
            std_mods: vec!["net".into()],
            ..Default::default()
        },
    );
    assert!(h.is_clean(), "lower: {:?}", h.diagnostics);
    let t = vis_types::check(&h.hir, &h.symbols, &h.types);
    assert!(t.is_clean(), "tyck: {:?}", t.diags);
    (h.hir, h.symbols, h.types)
}

const ASYNC_CHAN0: &str = r#"async fn sender(c: Chan[int]) -> int:
    c.send(7)
    1

fn main():
    let c = chan[int](0)
    let t = spawn sender(c)
    let x = c.recv()
    print(x)
"#;

#[test]
fn async_default_chay_chan0_ca_hai_engine() {
    let (hir, syms, tys) = tien_trinh(ASYNC_CHAN0);
    let i = vis_interp::run_async_with_hosts_caps(
        ASYNC_CHAN0,
        &hir,
        &syms,
        &tys,
        &[],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(i.exit, 0, "async interp: {:?}", i.diagnostics);
    assert_eq!(i.stdout, "Some(7)\n");

    let m = vis_bytecode::compile(&hir, &syms, ASYNC_CHAN0).expect("compile");
    let v =
        vis_vm::run_async_with_full_hosts_caps(&m, ASYNC_CHAN0, &[], None, None, None, None, &[]);
    assert_eq!(v.exit, 0, "async VM: {:?}", v.diagnostics);
    assert_eq!(v.stdout, i.stdout);

    // Runtime sync (đường cũ) vẫn từ chối `chan(0)` — chứng minh entry đã đổi thật.
    let old_i = vis_interp::run(ASYNC_CHAN0, &hir, &syms, &tys, &[]);
    assert_ne!(old_i.exit, 0);
    assert_eq!(old_i.codes(), vec!["VS0505"]);
    let mut old_vm = vis_vm::Vm::new(&m, ASYNC_CHAN0);
    let old_v = old_vm.run(&[]);
    assert_ne!(old_v.exit, 0);
    assert_eq!(old_v.codes(), vec!["VS0505"]);
}

#[test]
fn sync_khong_doi_chan0_van_tu_choi() {
    let src = "fn main():\n    let c = chan[int](0)\n    let _ = c.send(1)\n";
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run(src, &hir, &syms, &tys, &[]);
    assert_eq!(i.codes(), vec!["VS0505"]);
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let mut vm = vis_vm::Vm::new(&m, src);
    let v = vm.run(&[]);
    assert_eq!(v.codes(), vec!["VS0505"]);
}

const PIPE_SYNC: &str = r#"fn id(n: int) -> int:
    n

fn main():
    print(5 |> id)
"#;

const PIPE_ASYNC: &str = r#"async fn noop() -> int:
    1

fn id(n: int) -> int:
    n

fn main():
    print(await spawn noop())
    print(5 |> id)
"#;

#[test]
fn pipe_parity_default_va_async() {
    let (hir, syms, tys) = tien_trinh(PIPE_SYNC);
    let di = vis_interp::run(PIPE_SYNC, &hir, &syms, &tys, &[]);
    let m = vis_bytecode::compile(&hir, &syms, PIPE_SYNC).expect("compile");
    let mut vm = vis_vm::Vm::new(&m, PIPE_SYNC);
    let dv = vm.run(&[]);
    assert_eq!(di.stdout, "5\n");
    assert_eq!(dv.stdout, "5\n");
    assert_eq!(di.exit, dv.exit);

    let (ha, sa, ta) = tien_trinh(PIPE_ASYNC);
    let ai = vis_interp::run_async_with_hosts_caps(
        PIPE_ASYNC,
        &ha,
        &sa,
        &ta,
        &[],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(ai.exit, 0, "{:?}", ai.diagnostics);
    assert_eq!(ai.stdout, "1\n5\n");
    let ma = vis_bytecode::compile(&ha, &sa, PIPE_ASYNC).expect("compile");
    let av =
        vis_vm::run_async_with_full_hosts_caps(&ma, PIPE_ASYNC, &[], None, None, None, None, &[]);
    assert_eq!(av.stdout, "1\n5\n");
}

#[test]
fn async_nhan_args_va_entry_frame() {
    let src = "async fn noop() -> int:\n    1\nfn main(args: list[str]):\n    print(args.len())\n";
    let (hir, syms, tys) = tien_trinh(src);
    let i = vis_interp::run_async_with_hosts_caps(
        src,
        &hir,
        &syms,
        &tys,
        &["a".into(), "b".into()],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(i.exit, 0, "{:?}", i.diagnostics);
    assert_eq!(i.stdout, "2\n");
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = vis_vm::run_async_with_full_hosts_caps(
        &m,
        src,
        &["a".into(), "b".into()],
        None,
        None,
        None,
        None,
        &[],
    );
    assert_eq!(v.exit, 0, "{:?}", v.diagnostics);
    assert_eq!(v.stdout, "2\n");
}

#[test]
fn guard_scheduler_trap_ro_khong_treo() {
    // `spin` tạo task lồng vô hạn; hạ trần test xuống 100 turn để chứng minh guard có trap rõ
    // (production vẫn 200_000, hook chỉ test).
    let src = "async fn spin() -> int:\n    let t = spawn spin()\n    await t\n\nfn main():\n    print(await spin())\n";
    let (hir, syms, tys) = tien_trinh(src);
    vis_interp::rt::frame_set_max_turns_for_test(Some(100));
    let fi = run_i(src, &hir, &syms, &tys, &[], FrameConfig::default());
    vis_interp::rt::frame_set_max_turns_for_test(None);
    assert_ne!(fi.exit, 0, "guard phải trap");
    assert!(
        fi.diagnostics
            .iter()
            .any(|d| d.message.contains("scheduler quá giới hạn (100 turn)")),
        "{:?}",
        fi.diagnostics
    );

    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    vis_interp::rt::frame_set_max_turns_for_test(Some(100));
    let fv = run_v(&m, src, &[], FrameConfig::default());
    vis_interp::rt::frame_set_max_turns_for_test(None);
    assert_ne!(fv.exit, 0, "guard VM phải trap");
    assert!(
        fv.diagnostics
            .iter()
            .any(|d| d.message.contains("scheduler quá giới hạn (100 turn)")),
        "{:?}",
        fv.diagnostics
    );
}

// ---------------------------------------------------------------------------
// CLI: chọn entry + cache warm/cold
// ---------------------------------------------------------------------------

fn tam(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DEM: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!(
        "vis-1d-{tag}-{}-{}",
        std::process::id(),
        DEM.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("cache")).expect("tmp");
    d
}

fn viet(d: &Path, ten: &str, src: &str) -> PathBuf {
    let p = d.join(ten);
    std::fs::write(
        &p,
        format!("# SPDX-License-Identifier: MIT OR Apache-2.0\n{src}"),
    )
    .expect("ghi");
    p
}

fn chay(d: &Path, args: &[&str]) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_vis"));
    c.current_dir(d)
        .env("XDG_CACHE_HOME", d.join("cache"))
        .args(args);
    let o = c.output().expect("chạy vis");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).to_string(),
        String::from_utf8_lossy(&o.stderr).to_string(),
    )
}

#[test]
fn cli_async_chan0_va_sync_byte_identical() {
    let d = tam("cli");
    let a = viet(&d, "a.vis", ASYNC_CHAN0);
    let s = viet(&d, "s.vis", "fn main():\n    print(1 + 2)\n");
    for vm in [false, true] {
        let mut args = vec!["run"];
        if vm {
            args.extend(["--vm", "--no-cache"]);
        }
        let mut aa = args.clone();
        aa.push(a.to_str().unwrap());
        let (ca, oa, ea) = chay(&d, &aa);
        assert_eq!(ca, 0, "async vm={vm}: {ea}");
        assert_eq!(oa, "Some(7)\n", "async vm={vm}");

        let mut ss = args;
        ss.push(s.to_str().unwrap());
        let (cs, os, es) = chay(&d, &ss);
        assert_eq!(cs, 0, "sync vm={vm}: {es}");
        assert_eq!(os, "3\n", "sync vm={vm}");
    }
}

#[test]
fn cli_cache_warm_cold_async_vm() {
    let d = tam("cache");
    let a = viet(&d, "a.vis", ASYNC_CHAN0);
    let a = a.to_str().unwrap();
    let (c1, o1, e1) = chay(&d, &["run", "--vm", a]);
    assert_eq!((c1, o1.as_str()), (0, "Some(7)\n"), "{e1}");
    let (c2, o2, e2) = chay(&d, &["run", "--vm", a]);
    assert_eq!((c2, o2.as_str()), (0, "Some(7)\n"), "{e2}");
    let (c3, o3, e3) = chay(&d, &["run", "--vm", "--no-cache", a]);
    assert_eq!((c3, o3.as_str()), (0, "Some(7)\n"), "{e3}");
    assert_eq!(o1, o2);
    assert_eq!(o1, o3);
}
