<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315a — VM `budget_charge` O(1) + bỏ ghi TLS mỗi sự kiện

> **Lát**: `ST-315a` (W1 · Sóng 4) · **Nền**: `3ffd74d0` (`VD-291`) · **Ngày**: 2026-11-26
> **Trạng thái**: `REVIEW` (Agent 2 hoàn tất; chờ Agent 1/Agent 0 nghiệm thu)
> **Phạm vi**: `crates/vis-vm/src/vm.rs`, `crates/vis-vm/src/vm_frame.rs`,
> `crates/vis-vm/src/vm_frame/budget.rs`, `crates/vis-conformance/tests/async_api_it/lat256_budget.rs`

## 1. Thay đổi

1. **Chỉ mục phí dựng một lần** (`budget_enable`):
   - Thêm `Vm.budget_idx: Vec<Vec<(u32, u32)>>` (`hàm → pc → (start, len)`) và
     `Vm.budget_flat: Vec<FeeEntry>`.
   - `dung_chi_muc_phi()` đếm theo `pc`, chặn đúng trần `MAX_PER_PC = 16`, cấp vùng phẳng và đặt
     entry theo con trỏ `pc` ⇒ **giữ nguyên thứ tự gốc trong từng pc**.
   - `budget_charge` còn: tra `(start, len)` O(1) + xử lý đúng nhóm; bỏ mảng `[None; 16]` và quét
     tuyến tính toàn bộ entry của hàm mỗi lệnh.
2. **Bộ đếm ngân sách trên `Vm`**:
   - Thêm `Vm.budget_used: i64`; `budget_charge` cộng biến này thay vì ghi TLS `DUNG_GAN_NHAT`
     mỗi sự kiện.
   - `ket_thuc_ngan_sach()` flush TLS **một lần** ở mọi lối ra của `Vm::run` và
     `run_frame_vm_driver` (gồm cả nhánh verify lỗi/trap sớm); `bat_dau_ngan_sach_vm` reset cả
     `Vm.budget_used` lẫn TLS.
3. **Ngữ nghĩa giữ nguyên**: cùng sự kiện, cùng thứ tự `budget_trace_for_test`, cùng điểm trap
   `VS0510`, cùng hợp đồng quota (`quota_charge` vẫn trừ cùng `w`).

## 2. Test

- Ca mới `chi_muc_phi_giu_thu_tu_giua_cac_pc` (trong `lat256_budget.rs`, không tạo binary cấp 1):
  bảng phí xen kẽ `pc0/pc1` ⇒ trace đúng `[202, 204, 201, 203]`; đồng thời khoá
  `da_dung_gan_nhat() == Σ weight(trace)` (flush TLS đúng một lần).
- `cargo test -p vis-conformance --test lib_api_it lat256_budget`: **9 passed / 1 ignored**.
- `cargo test -p vis-vm`: **56 passed / 0 failed**.
- `./vis-dev gate --fast`: **PASS** (fmt, clippy, R1..R9, DAG, L0 test).
- `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `VIS_DIFF_N=100000 ./bin/vis-diff`: **lech=0** (100.000 chương trình, 58,6 s).

## 3. Đo release (`opt-bench`, n = 15, `# time:`)

- Binary `target/release/vis`, sha256 `657458382521907f2c9f3c96f09d7acab1612c2543475d9144445cd33d7cb48c`.
- interp **329 ms** · VM **344 ms** · VM `--unlimited-budget` **342 ms**.
- Trước vá: VM **610 ms** (cùng máy, `de-xuat-truc-ke-tiep-2026-11-25.md` §1b) ⇒ giảm **44 %**;
  overhead charge còn **~2 ms** so với `--unlimited-budget`.
- KPI `VM ≤ interp + 5%` (≤ 345,45 ms): **PASS** (344 ms). Log: `/home/ps/vislogs/st315a_opt_bench.md`.

## 4. Hậu-kiểm R-A (`review/r315a`, `96c6ad45`)

- **PASS** — differential 25 bảng phí ngẫu nhiên so thuật toán cũ khớp 100 %; 11 mốc trap
  (`VS0510`, trace đúng tiền tố, `budget_used` loại đúng sự kiện bị trap); `MAX_PER_PC=16`;
  pc vượt `code.len()`; stale index khi `budget_enable` lại; frame/async `spawn`/`await`;
  `budget_fee=None`; verify lỗi; nhiều lần chạy cùng Vm.
- Engine-only 45 cặp xen kẽ: interp 312,3 / VM 320,1 ms ⇒ **+2,7 %**, overhead charge +3,9 ms.
- **F2 (đã bổ sung `54597d30`)**: ca `chi_muc_phi_chan_max_per_pc_va_trap_giua_nhom`.
- F1 (bộ nhớ `budget_idx` 8 B/lệnh) và F3 (báo cáo 0 cho lần không phí) là ghi nhận **không chặn**.
