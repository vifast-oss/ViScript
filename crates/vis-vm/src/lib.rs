// SPDX-License-Identifier: MIT OR Apache-2.0
//! `vis-vm` — máy ảo **VBC1** cho ViScript (ST-20, VD-52/VD-54).
//!
//! # Ngữ nghĩa **trùng** `vis-interp`
//!
//! VM **không** có bảng ngữ nghĩa riêng: toán tử, builtin, phương thức, `Show`, so sánh và vật phẩm
//! `for` đều **gọi** hàm công khai của `vis-interp` (`builtin::{bin,un,call,method}`,
//! `value::{show,eq,cmp,iter_items}`). Nhờ vậy hai đường chạy không thể lệch nhau về ngữ nghĩa.
//!
//! # An toàn với bytecode hỏng
//!
//! Ngăn xếp là `Vec<Value>` **động** — **không** tin `max_stack` trong module (VD-54 (2)); mọi
//! `pop` rỗng ⇒ [`VmError`] ⇒ `VS0501` "bytecode hỏng", **không** panic. Cap khung gọi
//! [`MAX_FRAMES`] = 2.000 (VD-49) và fuel an toàn [`MAX_STEPS`] = `fee::VM_SAFETY_FUEL` (100M,
//! VS3-1a) — fuel là hàng rào bytecode, lớn hơn trần ngân sách định cỡ nên không chặn chương
//! trình còn trong ngân sách trước `VS0510`.
//!
//! # `steps` khác đơn vị
//!
//! `steps` của VM đếm **lệnh đã thực thi**, của interp đếm **bước ngữ nghĩa** ⇒ test differential
//! **không** so `steps` (chỉ `stdout`, `exit`, `(mã, dòng, cột)` của chẩn đoán). Hợp đồng VS3-1a
//! ràng buộc **quan hệ** giữa hai đại lượng qua `fee::VM_STEPS_PER_COST_CEILING`, không đòi bằng nhau.
//!
//! # Builtin làm giá trị — `$value:<tên>` (VD-208 (2), VD-218)
//!
//! Giá trị dựng sẵn (`Some`, `Ok`, …) dùng được **như hàm** ở cả hai đường: `let f = Some; f(5)`,
//! `xs.map(Some)`. VBC1 đã **đóng băng** — không thêm mã `MakeBuiltin` (không thêm opcode nữa) —
//! nên VM **dựng giá trị qua builtin nội bộ `$value:<tên>`** (opcode `CallBuiltin` + tên nội bộ
//! `$value:...`; `:` không hợp lệ trong tên dựng sẵn nên người dùng không viết được). `None` vẫn
//! là **literal** của enum `Option` (`MakeNone`). Tên `$value:<tên>` **không** bao giờ xuất hiện ở
//! bề mặt người dùng: thông báo lỗi và stack trace chặn ở tầng thông điệp (tầng này chỉ in tên
//! dựng sẵn trần qua `Value::Builtin`, ví dụ `<builtin Some>`), LSP hover/`print(f)` giữ nguyên,
//! và disassemble (`vis run --dump-bytecode`) ẩn tiền tố (in `<builtin Some>`).

use vis_core::RunOutput;

pub mod dump;
pub mod vm;
/// **ST-267/VD-251 (chỉ test/debug)** — frame scheduler nối vào đường VM thật.
#[doc(hidden)]
pub mod vm_frame;

/// **VS3-3b/W5 (P3)** — VM non-frame **W=1** (shared globals, cùng đường `Vm::run`).
#[doc(hidden)]
pub fn run_workers_local_vm(m: &vis_bytecode::Module, text: &str, args: &[String]) -> RunOutput {
    vm::Vm::new(m, text).run(args)
}

/// **VS3-3b/W5 (P3)** — VM non-frame `Parallel(n)` CPU-only (test/opt-in).
///
/// Mỗi task dựng `Vm` riêng trên `Arc<ModuleImage>`, globals snapshot, output merge spawn order;
/// budget/cancel opt-in. Không host/`chan(0)`/frame.
#[doc(hidden)]
pub fn run_workers_parallel_vm(
    m: &vis_bytecode::Module,
    text: &str,
    args: &[String],
    workers: usize,
) -> RunOutput {
    run_workers_parallel_vm_mode(m, text, args, workers, None, None)
}

/// Như [`run_workers_parallel_vm`] nhưng dùng `SharedBudget` atomic (caller giữ `Arc` để đọc
/// `snapshot()` sau khi chạy).
#[doc(hidden)]
pub fn run_workers_parallel_vm_with_budget(
    m: &vis_bytecode::Module,
    text: &str,
    args: &[String],
    workers: usize,
    budget: std::sync::Arc<vis_runtime::rt::fee::SharedBudget>,
) -> RunOutput {
    run_workers_parallel_vm_mode(m, text, args, workers, Some(budget), None)
}

/// **ST-292 (P4)** — `Parallel(n)` có đủ host/caps (CLI `vis run --workers`).
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn run_workers_parallel_vm_with_full_hosts_caps<'a>(
    m: &'a vis_bytecode::Module,
    text: &'a str,
    args: &[String],
    ngoai: Option<&'a mut dyn vis_core::host::Ngoai>,
    fs: Option<&'a mut dyn vis_core::host::FsHost>,
    net: Option<&'a mut dyn vis_core::host::NetHost>,
    db: Option<&'a mut dyn vis_core::host::DbHost>,
    caps: &[(u32, vis_core::value::Value)],
    workers: usize,
    budget: Option<std::sync::Arc<vis_runtime::rt::fee::SharedBudget>>,
) -> RunOutput {
    let mut vm = vm::Vm::new(m, text);
    vm::worker::cau_hinh(&mut vm, m, text, workers.max(2), budget, None);
    if let Some(n) = ngoai {
        vm.set_ngoai(n);
    }
    if let Some(f) = fs {
        vm.set_fs(f);
    }
    if let Some(g) = net {
        vm.set_net(g);
    }
    if let Some(d) = db {
        vm.set_db(d);
    }
    if !caps.is_empty() {
        vm.caps = caps.to_vec();
    }
    vm.run(args)
}

/// **Test hook P3**: huỷ task VM đang chạy sau `ms` mili giây (safe point 1.024 lệnh).
#[doc(hidden)]
pub fn run_workers_parallel_vm_cancel_after(
    m: &vis_bytecode::Module,
    text: &str,
    args: &[String],
    workers: usize,
    ms: u64,
) -> RunOutput {
    run_workers_parallel_vm_mode(m, text, args, workers, None, Some(ms))
}

fn run_workers_parallel_vm_mode(
    m: &vis_bytecode::Module,
    text: &str,
    args: &[String],
    workers: usize,
    budget: Option<std::sync::Arc<vis_runtime::rt::fee::SharedBudget>>,
    cancel_after_ms: Option<u64>,
) -> RunOutput {
    assert!(workers >= 2, "P3 cần W>=2; W=1 dùng run_workers_local_vm");
    let mut vm = vm::Vm::new(m, text);
    vm::worker::cau_hinh(&mut vm, m, text, workers, budget, cancel_after_ms);
    vm.run(args)
}

pub use vm::{
    run, run_with_fs, run_with_full_hosts_caps, run_with_hosts, run_with_hosts_caps, Vm, VmError,
    MAX_FRAMES, MAX_STEPS,
};
pub use vm_frame::{
    run_async_with_full_hosts_caps, run_async_with_full_hosts_caps_backend,
    run_async_workers_with_full_hosts_caps,
};
