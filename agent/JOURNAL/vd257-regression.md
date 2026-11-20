<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-257 / ST-283 — Regression, fuzz có seed, đo release (tự chủ cuối tuần)

> **Phạm vi**: VD-257 §6 — mọi task được duyệt đã xong ⇒ chuyển regression/fuzz/đo dài **có mục
> tiêu**, không mở tính năng mới, không chạy lại full gate trên cùng commit chỉ để tăng số PASS.
> **Commit đo**: `a6ad45277beec59f203457bf7007c6f44f1bd5fd` (cây sạch, tag `pha1-rc1`).
> **Thời điểm**: 2026-11-20, 19:12–19:xx JST. Toolchain `1.100.0`, Linux x86_64, 12 lõi / 14 GiB.

## 1. Kiểm hồi quy đa target (một làn, tuần tự)

| Cấu hình | Lệnh | Kết quả |
|---|---|---|
| core Linux | `cargo check --workspace --no-default-features` | PASS (0,83 s) |
| core Windows-msvc | `cargo check --workspace --no-default-features --target x86_64-pc-windows-msvc` | PASS (1,18 s) |
| core wasm32 | `cargo check --workspace --no-default-features --target wasm32-unknown-unknown` | PASS (1,13 s) |

Đây là **bằng chứng biên dịch** của cấu hình core; **không** phải bằng chứng runtime Windows. Log:
`/tmp/opencode/vd257-targets.log`.

## 2. Fuzz parser có seed hữu hạn

`target/release/vis-parse-fuzz --seconds 60 --seed <s>` — 3 seed, mỗi 60 s, tổng 180 s:

| Seed | Số ca | Token | Vi phạm | Exit |
|---|---|---|---|---|
| 20261120 | 9.482.304 | 215.172.705 | **0** | 0 |
| 20261121 | 9.493.474 | 215.377.958 | **0** | 0 |
| 20261122 | 9.420.105 | 213.858.975 | **0** | 0 |

Log: `/tmp/opencode/vd257-fuzz-seeds.log`. Không có seed lỗi để lưu. Ngoài ra
`./vis-dev nightly --fuzz-minutes 1` (seed ngày) PASS, vòng tròn fmt 475 tệp bất động.

## 3. Đo **release** (phần B trước đó chỉ đo debug)

Lệnh: `cargo test --release -p vis-cli --test <t> -- --ignored --nocapture`.

### Ngân sách (`lat256_budget::measure_workloads`)

| Workload | Debug (trước) | **Release** | events | cost đề xuất |
|---|---|---|---|---|
| loop-2M | 3,43 s | **374,68 ms** | 18.000.008 | 18.000.008 |
| recursion-200 | 7,1 ms | **0,635 ms** | 2.211 | 2.815 |
| tasks-1000 | 5,5 ms | **0,859 ms** | 14.008 | 17.008 |
| expr-chain | 387 ms | **48,34 ms** | 2.000.008 | 2.000.008 |

Release nhanh hơn debug ~9×; **số sự kiện/chi phí không đổi** (đúng kỳ vọng — catalog là luật ngữ
nghĩa, không phải đồng hồ CPU). Log: `/tmp/opencode/vd257-release-budget.log`.

### Quét Q (`lat257_fairness::quet_q_phan_bo_khong_chon_default`)

| Q | yields | startup (ms) | io_lat_cost | short_lat_cost | thứ tự | snapshot |
|---|---|---|---|---|---|---|
| control | 0 | 0,277 | 450.033 | 450.045 | IO sau CPU (B2) | 0 |
| 1.000.000 | 0 | 0,162 | 450.033 | 450.045 | IO trước CPU (thực tế không yield) | 0 |
| 200.000 | 2 | 0,168 | 200.020 | 200.032 | IO trước CPU | 0 |
| 50.000 | 10 | 0,163 | 50.020 | 50.032 | IO trước CPU | 0 |
| 10.000 | 52 | 0,153 | 10.020 | 10.032 | IO trước CPU | 0 |
| 2.000 | 262 | 0,161 | 2.020 | 2.032 | IO trước CPU | 0 |
| 500 | 1.050 | 0,143 | 520 | 532 | IO trước CPU | 0 |

`DiagSnapshot` = `{live_tasks:0, frames:0, waiters:0, ready:0, regs:0, timers:0}` ở **mọi** Q.
Latency tính bằng **cost units** (đơn điệu, tất định), startup đo riêng không trộn vào workload.
Log: `/tmp/opencode/vd257-release-fairness.log`.

**Không chọn default Q** — A0 phải quyết từ đường cong. Q=1M vô hiệu (tổng cost ~450k); Q=500 latency
thấp nhất nhưng 1.050 yield ⇒ overhead chuyển turn.

## 4. Kết luận regression

- Không có hồi quy biên dịch trên core/Windows-msvc/wasm32 sau khi gộp A/B/C/D.
- Fuzz 28,4 triệu ca qua 3 seed ⇒ 0 vi phạm.
- Số đo release bổ sung cho B; **không đổi** đề xuất `BUDGET`/`FUEL`/`Q` — vẫn chờ A0 chốt.
- Không chạy lại full gate; gate tại `a6ad452` đã **1562 passed / 3 skipped / 0 failed** (báo cáo
  `agent/BAO-CAO-BAN-GIAO-2026-11-20.md`).

## 5. Ranh giới parity sự kiện (phát hiện khi audit trace)

Test mới `crates/vis-cli/tests/lat257_trace_audit.rs` (2/2) dùng chương trình có `match`/`with`/`?`
và ghi nhận: `fee.trace()` của compiler là **bảng tĩnh gồm mọi hàm/nhánh**, không phải trace thực
thi. Bằng chứng "dãy sự kiện byte-identical" của `lat256_budget.rs` chỉ đúng cho chương trình
**một hàm, không nhánh**; với chương trình nhiều hàm/nhánh không được suy ra parity thứ tự đầy đủ.
Kiểm chứng thay thế: mọi sự kiện thực thi đều nằm trong bảng tĩnh (tập con), và 4 đường chạy
(mặc định interp/VM, frame interp/VM) cùng stdout/exit. **Không hạ tiêu chí test cũ**, không đổi mã
sản phẩm; giới hạn được ghi thành test `ghi_nhan_gioi_han_parity`.

## 6. Soak hữu hạn bổ sung

- **Fuzz parser 5 seed mới** (`20261123`..`20261127`, 60 s/seed): ~47,4 triệu ca, **0 vi phạm**,
  exit 0 cả 5. Cộng 3 seed ở §2 ⇒ **~75,8 triệu ca, 0 vi phạm**. Log `/tmp/opencode/vd257-fuzz-more.log`.
- **Lặp test vòng đời release 20 lần** (`cargo test --release -p vis-cli --test lat257_c_lifecycle`):
  **20/20 OK**, mỗi lần 11 passed / 0 failed, không flake/drift. Log `/tmp/opencode/vd257-lifecycle-*.log`.
- Seed và cấu hình được lưu; không có seed lỗi. Không chạy OOM/không `cargo clean` toàn bộ.

## 7. B1-real-frame ở release

Lệnh `cargo test --release -p vis-cli --test lat3_real t10_b1_real_frame_overlap_measure -- --nocapture`
(server loopback trễ 200 ms, N=1/5/20 × 3 lần × 2 engine):

| N | workload interp (ms) | workload VM (ms) | startup (ms) | ok |
|---|---|---|---|---|
| 1 | 200,83 / 200,99 / 201,11 | 201,15 / 200,75 / 201,08 | 0,42–0,93 | 3/3 |
| 5 | 201,52 / 201,15 / 201,01 | 201,41 / 201,87 / 200,76 | 1,31–2,15 | 5/5 |
| 20 | 201,43 / 201,76 / 202,41 | 201,32 / 201,13 / 201,58 | 1,71–2,50 | 20/20 |

**Chồng lấp** rõ: 20 kết nối × 200 ms vẫn ~201 ms, không phải ~4 s. Startup đo riêng, không trộn
vào workload. Log `/tmp/opencode/vd257-b1-release.log`. Không đổi baseline mặc định (4,03–4,04 s).

## 8. Khép thêm một ô coverage thật (`E.A` trên đường frame)

Test `lat257_coverage_extra.rs` (source có `set{...}`, `map`, tuple, enum `E.A`, interpolation)
ban đầu **đỏ** trên frame interp: `E` bị lấy nhầm thành argv fallback `u32::MAX` ⇒ `Value::List([])`
rồi `field_of` báo "`[]` không có trường `A`". Đã sửa `frame/expr.rs`: ưu tiên tra `it.tags` (atom
marker `Value::Enum(n,"",[])` / `Value::Record(n,[])`) **trước** fallback argv, mirror `interp.rs:651`;
`interp.rs::tags` chuyển `pub(crate)`. Giờ 4 đường chạy cùng stdout. Gate sau sửa: **1565/1565**.
Đây là sửa lỗi coverage, **không** đổi checker hay runtime mặc định.

## 9. Ma trận target sau tích hợp

`./bin/vis-targets`: exit 0; **core** `PASS=18 EXPECTED_RED=0 FAIL=0`; **sqlite** `PASS=16
EXPECTED_RED=2 FAIL=0` (2 ô `vis-cli` × Windows-msvc/wasm32 — lỗi `libsqlite3-sys` có sẵn).
Không có ô mới chuyển sang FAIL sau khi gộp A/B/C/D. Log `/tmp/opencode/vd257-targets-matrix.log`.

## 10. Cross-review phạm vi diff

`git diff --stat fc297ba..HEAD`: **43 tệp**, tất cả nằm trong `crates/`, `agent/`, hoặc các tệp cấu
hình đã được ủy quyền — **không có tệp ngoài danh sách dự kiến**. `rt.rs` (−566) và `rt/sched.rs`
(+694), `vm.rs` (−263) và `vm/vm_helpers.rs` (+174) là tách module để qua luật R1, không đổi API.
Không thay đổi cú pháp/API công khai, không dependency/lock, không runtime mặc định.

