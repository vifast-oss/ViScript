// SPDX-License-Identifier: MIT OR Apache-2.0
//! **VD-258 / frame path — hồi quy lỗi nghiêm trọng** (chỉ test/debug, KHÔNG bật mặc định).
//!
//! Mỗi ca dưới đây **tái hiện lỗi cũ** và khoá hành vi đúng trên **cả hai engine**
//! (`run_frame_for_test` của interp + VM). Đường runtime mặc định không đổi.
//!
//! | Ca | Lỗi cũ |
//! |---|---|
//! | `redispatch_*` | resume builtin treo đẩy lại đối số vào `vals` ⇒ hỏng toán hạng |
//! | `vm_map_callback_await_*` | `CallMethod` pop recv/args + tăng `pc` trước Pending ⇒ `VS0501` |
//! | `each_callback_await_*` | `RedispatchMethod` chạy lại cả host method ⇒ lặp side-effect callback |
//! | `deadlock_*` | hết frame sẵn sàng vẫn exit 0, bỏ waiter/task đang chờ |
//! | `self_await_*` | `TrangThai::DangChay` không bao giờ đặt ⇒ await chính mình treo im lặng |
//! | `vm_fake_source_*` | final join chạy thân **thật** của nguồn giả |
//! | `const_default_*` | hàm tên có env rỗng ⇒ không thấy `const` cấp tệp |
//! | `exit_*` | `Result.Err` từ `main` bị bỏ qua (exit 0); `main -> int` **không** phải mã thoát (VD-259) |
//! | `closure_*` | mỗi lần gọi closure dùng chung env ⇒ tham số bị lần gọi khác xoá |

use vis_interp::frame::{run_frame_for_test as run_i, FrameConfig, FrameOutput};
use vis_interp::rt::{fee as ifee, FakeEvent, ReactorEvent, ReactorOutcome};
use vis_interp::Value;
use vis_syntax::{lex, parse, Mode};
use vis_vm::vm_frame::{
    frame_budget_disable_for_test, frame_budget_enable_for_test, run_frame_for_test as run_v,
};

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

/// Chạy cùng nguồn + cùng config trên hai engine, trả `(interp, vm)`.
fn chay(src: &str, c: FrameConfig) -> (FrameOutput, FrameOutput) {
    let (hir, syms, tys) = tien_trinh(src);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let m = vis_bytecode::compile(&hir, &syms, src).expect("compile");
    let v = run_v(&m, src, &[], c);
    (i, v)
}

fn ok_read(seq: u64, key: u64, s: &str) -> ReactorEvent {
    ReactorEvent {
        seq,
        key,
        gen: 0,
        outcome: ReactorOutcome::Ready(Value::ok(Value::Str(s.into()))),
        deadline_ms: None,
    }
}

fn assert_diag_parity(i: &FrameOutput, v: &FrameOutput) {
    assert_eq!(
        i.diagnostics.len(),
        v.diagnostics.len(),
        "số chẩn đoán lệch"
    );
    for (a, b) in i.diagnostics.iter().zip(&v.diagnostics) {
        assert_eq!(a.code.short, b.code.short, "mã chẩn đoán lệch");
        assert_eq!(a.message, b.message, "nội dung chẩn đoán lệch");
        assert_eq!((a.line, a.col), (b.line, b.col), "vị trí chẩn đoán lệch");
    }
}

// ===========================================================================
// 1) `Redispatch` không được chèn đối số đã pop vào ngăn xếp toán hạng
// ===========================================================================

/// `okr` được đẩy lên `vals` TRƯỚC khi vế phải `net.read(...)` treo. Nếu resume đẩy lại
/// đối số (`"C1"`, 8, 1000) thì `==` so nhầm `1000` với `Result` ⇒ sai kết quả (cũ: `false`).
#[test]
fn redispatch_khong_chen_rac_vao_ngan_xep_toan_hang() {
    let src = r#"use std.net
fn main():
    let okr = Ok("DATA")
    print(okr == net.read("C1", 8, 1000))
    print("SAU")
"#;
    let c = FrameConfig {
        reactors: vec![ok_read(0, 0, "DATA")],
        ..Default::default()
    };
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert!(o.diagnostics.is_empty(), "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "true\nSAU\n", "{name}: toán hạng bị hỏng");
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 2) VM `CallMethod` + callback `await` (map/filter/each) — resume đúng
// ===========================================================================

/// `xs.map(f)` với `f` treo ở `net.read`: VM cũ pop recv/args + tăng `pc` rồi Pending
/// ⇒ resume chạy instruction sau với ngăn xếp rỗng (`VS0501: bytecode hỏng: ngăn xếp rỗng`).
/// Nay vòng callback nằm trên `Th`: `idx`/`acc` sống qua suspend.
#[test]
fn vm_map_callback_await_khong_hong_bytecode() {
    let src = r#"use std.net
fn main():
    let xs = [1, 2, 3]
    let f = fn(x: int) -> str:
        let r = net.read("C1", 8, 1000)
        match r:
            Ok(s): "{x}:{s}"
            Err(e): "err"
    print(xs.map(f))
"#;
    let c = FrameConfig {
        reactors: vec![ok_read(0, 0, "A"), ok_read(1, 1, "B"), ok_read(2, 2, "C")],
        ..Default::default()
    };
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "[1:A, 2:B, 3:C]\n", "{name}: map/await sai");
    }
    assert_diag_parity(&i, &v);
}

/// `filter` + `each` với callback treo: mỗi callback in **đúng một lần** (cũ: interp
/// `RedispatchMethod` chạy lại cả phương thức ⇒ in lặp 1,1,1,… rồi deadlock).
#[test]
fn each_callback_await_khong_lap_side_effect() {
    let src = r#"use std.net
fn main():
    let xs = [1, 2, 3, 4]
    let pred = fn(x: int) -> bool:
        let _ = net.read("C1", 8, 1000)
        x > 2
    print(xs.filter(pred))
    let in_ra = fn(x: int) -> ():
        let _ = net.read("C1", 8, 1000)
        print(x)
    let _ = xs.each(in_ra)
"#;
    let evs: Vec<ReactorEvent> = (0..8).map(|k| ok_read(k, k, "X")).collect();
    let c = FrameConfig {
        reactors: evs,
        ..Default::default()
    };
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(
            o.stdout, "[3, 4]\n1\n2\n3\n4\n",
            "{name}: callback bị chạy lặp/thiếu"
        );
        assert_eq!(
            o.stdout.matches("\n1\n").count(),
            1,
            "{name}: side-effect callback lặp"
        );
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 3) Deadlock thật: event không bao giờ tới ⇒ VS0505, KHÔNG exit 0
// ===========================================================================

/// Nguồn giả `src1` không có event nào: task `victim` treo ở `await src1()`.
/// Cũ: interp quay tới guard (VS0501), VM final-join chạy thân thật rồi exit 0.
#[test]
fn deadlock_thieu_event_trap_ro() {
    let src = r#"async fn src1() -> int:
    0
async fn victim() -> int:
    print("V:pre")
    let x = await src1()
    print("V:post")
    99
fn main():
    let a = spawn victim()
    print("main")
"#;
    let c = FrameConfig {
        fake_sources: vec![("src1".into(), 1)],
        ..Default::default()
    };
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 1, "{name}: deadlock không được exit 0");
        assert_eq!(o.diagnostics.len(), 1, "{name}: {:?}", o.diagnostics);
        assert_eq!(
            o.diagnostics[0].code.short, "VS0505",
            "{name}: phải là deadlock: {:?}",
            o.diagnostics
        );
        assert_eq!(o.stdout, "main\nV:pre\n", "{name}: chạy tiếp sau điểm treo");
        assert!(
            !o.stdout.contains("V:post"),
            "{name}: task treo vẫn chạy tiếp"
        );
    }
    assert_diag_parity(&i, &v);
}

/// Gửi chính handle của task vào kênh để task `await` chính nó. Cũ: `DangChay` không
/// bao giờ được đặt (interp quay tới guard; VM chạy sai/treo). Nay: lỗi rõ `VS0506`
/// bọc `VS0501` "await chính tác vụ đang chạy", exit 1, không treo.
#[test]
fn self_await_trap_ro_khong_treo_im() {
    let src = r#"async fn f(c: Chan[Task[int]]) -> int:
    let me = c.recv()
    match me:
        Some(t): await t
        None: 0
fn main():
    let c = chan[Task[int]](1)
    let t = spawn f(c)
    let _ = c.send(t)
"#;
    let (i, v) = chay(src, FrameConfig::default());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 1, "{name}: self-await phải lỗi");
        assert_eq!(
            o.diagnostics[0].code.short, "VS0506",
            "{name}: {:?}",
            o.diagnostics
        );
        assert!(
            o.diagnostics[0].message.contains("chính tác vụ đang chạy"),
            "{name}: message: {}",
            o.diagnostics[0].message
        );
        assert!(!o.diagnostics[0].message.contains("quá giới hạn"));
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 4) VM final join: nguồn giả + event stale KHÔNG được chạy thân thật
// ===========================================================================

/// `spawn ext()` (nguồn giả) không ai await + event `gen` lệch ⇒ task không bao giờ xong.
/// Cũ: VM final-join đẩy task vào ready và chạy **thân thật** (`REAL_BODY`). Nay: lọc
/// `external` + deadlock `VS0505`, stdout chỉ còn `main`.
#[test]
fn vm_fake_source_stale_khong_chay_than_that() {
    let src = r#"async fn ext() -> int:
    print("REAL_BODY")
    1
fn main():
    let t = spawn ext()
    print("main")
"#;
    let c = FrameConfig {
        fake_sources: vec![("ext".into(), 1)],
        events: vec![FakeEvent {
            seq: 0,
            key: 1,
            gen: 1, // stale: đăng ký hiện hành là gen 0
            value: Value::Int(9),
        }],
        ..Default::default()
    };
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 1, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "main\n", "{name}: thân nguồn giả bị chạy");
        assert!(!o.stdout.contains("REAL_BODY"));
        assert_eq!(o.diagnostics[0].code.short, "VS0505");
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 5) Hàm tên thấy `const` cấp tệp + default tham số
// ===========================================================================

/// Hàm tên cũ dựng env rỗng ⇒ `const K` trong thân/default báo "chưa định nghĩa".
#[test]
fn const_va_default_tham_so_trong_ham_ten() {
    let src = r#"const K: int = 7
fn g(x: int = K + 1) -> int:
    x + K
fn main():
    print(g())
    print(g(1))
"#;
    let (i, v) = chay(src, FrameConfig::default());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "15\n8\n", "{name}: const/default sai");
    }
    assert_diag_parity(&i, &v);
}

// ===========================================================================
// 6) Exit code theo giá trị `main` (không luôn 0)
// ===========================================================================

/// **VD-259**: chỉ `Result::Err` ⇒ 1, `Ok` ⇒ 0; **không** suy rộng sang `main -> int`
/// (quyết định A0 2026-11-19, mục 7) — `main -> int` trả 3 vẫn thoát **0**.
#[test]
fn exit_theo_gia_tri_main() {
    for (tag, src, want) in [
        ("int-3", "fn main() -> int:\n    3\n", 0),
        ("err", "fn main() -> Result[int, str]:\n    Err(\"x\")\n", 1),
        ("ok", "fn main() -> Result[int, str]:\n    Ok(3)\n", 0),
        ("unit", "fn main():\n    ()\n", 0),
    ] {
        let (i, v) = chay(src, FrameConfig::default());
        for (name, o) in [("interp", &i), ("vm", &v)] {
            assert_eq!(o.exit, want, "{tag}/{name}: exit sai: {:?}", o.diagnostics);
            assert!(
                o.diagnostics.is_empty(),
                "{tag}/{name}: {:?}",
                o.diagnostics
            );
        }
    }
}

// ===========================================================================
// 7) Closure: biến bắt chia sẻ giữa các lần gọi tuần tự, tham số không rò
// ===========================================================================

/// Mỗi lần gọi closure có env riêng nhưng phải **gộp** biến bắt (`var x`) về env gốc
/// ⇒ `11`, `12` như runtime mặc định. (Cô lập tham số giữa hai lần gọi đang treo được
/// kiểm ở unit test `frame::tests::closure_moi_lan_goi_env_rieng`.)
#[test]
fn closure_bien_bat_giu_qua_cac_lan_goi() {
    let src = r#"fn main():
    var x = 10
    let f = fn() -> int:
        x = x + 1
        x
    print(f())
    print(f())
"#;
    let (i, v) = chay(src, FrameConfig::default());
    assert_eq!(i.stdout, "11\n12\n");
    assert_eq!(v.stdout, "11\n12\n");
}

// ===========================================================================
// 8) Event không chết đói sau hàng ready (quota yield)
// ===========================================================================

/// Task I/O treo ở `net.read`, task CPU luôn sẵn sàng. Cũ: event reactor chỉ được fire
/// khi hàng `ready` cạn ⇒ `IO_DONE` sau `CPU_DONE` dù quota đã nhường. Nay event có
/// waiter được giao ngay ở `frame_next`, nên I/O hoàn tất trước khi CPU xong.
#[test]
fn event_reactor_khong_chet_doi_sau_hang_ready() {
    let src = r#"use std.net
async fn cpu(n: int) -> ():
    var i = 0
    while i < n:
        i = i + 1
    print("CPU_DONE")
async fn io() -> ():
    print("IO_START")
    let r = net.read("C1", 8, 1000)
    print("IO_DONE")
fn main():
    let a = spawn io()
    let b = spawn cpu(50000)
    print(await a)
"#;
    let c = FrameConfig {
        reactors: vec![ok_read(0, 0, "A")],
        ..Default::default()
    };
    let (hir, syms, tys) = tien_trinh(src);

    ifee::enable_for_test(40_000_000);
    ifee::frame_emit_enable_for_test();
    ifee::quota_enable_for_test(500);
    let i = run_i(src, &hir, &syms, &tys, &[], c.clone());
    let yi = ifee::quota_yields_for_test();
    ifee::disable_for_test();

    let (m, fee) = vis_bytecode::compile_with_fee(&hir, &syms, src).expect("compile_with_fee");
    frame_budget_enable_for_test(fee, 40_000_000, Some(500));
    let v = run_v(&m, src, &[], c);
    frame_budget_disable_for_test();

    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        let io_done = o.stdout.find("IO_DONE").expect("IO_DONE");
        let cpu_done = o.stdout.find("CPU_DONE").expect("CPU_DONE");
        assert!(
            io_done < cpu_done,
            "{name}: event bị đói sau ready: {:?}",
            o.stdout
        );
    }
    assert!(yi > 0, "interp phải có yield quota để tái hiện starvation");
    assert_diag_parity(&i, &v);
}

/// Ca dương của bộ lọc nguồn giả: event **khớp** ⇒ final join hoàn tất task qua event,
/// KHÔNG chạy thân thật, về baseline.
#[test]
fn vm_fake_source_event_khop_khong_chay_than_that() {
    let src = r#"async fn ext() -> int:
    print("REAL_BODY")
    1
fn main():
    let t = spawn ext()
    print("main")
"#;
    let c = FrameConfig {
        fake_sources: vec![("ext".into(), 1)],
        events: vec![FakeEvent {
            seq: 0,
            key: 1,
            gen: 0,
            value: Value::Int(9),
        }],
        ..Default::default()
    };
    let (i, v) = chay(src, c);
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "main\n", "{name}: in thân nguồn giả");
        assert_eq!(o.snapshot.live_tasks, 0, "{name}: task chưa thu hồi");
    }
    assert_diag_parity(&i, &v);
}

/// `main` `return` sớm vẫn phải final-join task không ai await (không bỏ rơi task).
#[test]
fn main_return_som_van_final_join() {
    let src = r#"async fn lonely() -> int:
    print("lonely")
    9
fn main():
    let t = spawn lonely()
    print("main")
    return
"#;
    let (i, v) = chay(src, FrameConfig::default());
    for (name, o) in [("interp", &i), ("vm", &v)] {
        assert_eq!(o.exit, 0, "{name}: {:?}", o.diagnostics);
        assert_eq!(o.stdout, "main\nlonely\n", "{name}: task bị bỏ rơi");
    }
    assert_diag_parity(&i, &v);
}
