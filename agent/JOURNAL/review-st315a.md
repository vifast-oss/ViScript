<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Review ST-315a — Gatekeeper (Agent 1)

> **Lát**: `ST-315a` (W1 · Sóng 4) · **Commit kiểm**: `50bb7885` · **Diff**: `3ffd74d0..50bb7885`
> **Worktree review**: `/home/ps/ViScript-r315a`, nhánh `review/r315a`, nền `987b98b8`
> **Ngày**: 2026-11-26 · **Kết luận**: **PASS** — đề nghị Agent 0 duyệt `DONE`.

## 1. Kết luận ngắn

Không có lỗi chặn. Các tuyên bố của commit (`budget_idx`/`budget_flat` dựng một lần ở
`budget_enable`, `budget_charge` tra O(1), bỏ ghi TLS mỗi sự kiện, `Vm.budget_used` flush một lần
ở mọi lối ra) đều đúng theo mã và được kiểm lại độc lập. Thứ tự `budget_trace_for_test`, điểm trap
`VS0510`, quota và hành vi `budget_fee=None`/async/worker không đổi. KPI hiệu năng tương đối
`VM ≤ interp + 5%` tái lập được (đo engine-only: **+2,7 %**; cửa sổ máy êm qua CLI: **+3,6..4,4 %**).

Hai ghi nhận **không chặn** (chi tiết §6): chỉ mục dày 8 B/lệnh kể cả hàm không có entry; ca test
mới chỉ phủ đường thuận, chưa khoá `MAX_PER_PC`/stale-index/trap giữa nhóm.

## 2. Phạm vi đã đọc

- `crates/vis-vm/src/vm_frame/budget.rs` (toàn bộ 278 dòng), `crates/vis-vm/src/vm.rs`
  (`Vm::run`, khởi tạo trường), `crates/vis-vm/src/vm_frame.rs` (`run_frame_vm_driver`,
  `run_frame_for_test*`), `crates/vis-vm/src/vm/exec.rs` (điểm gọi `budget_charge`).
- Ca mới `crates/vis-conformance/tests/async_api_it/lat256_budget.rs:128-163`; hàng `ST-315a`
  trong `agent/QUEUE.md`; `agent/JOURNAL/st315a-vm-budget.md`; `agent/STATE.md`.
- Đối chiếu thuật toán cũ: `git show 3ffd74d0:crates/vis-vm/src/vm_frame/budget.rs`.
- Xác nhận `50bb7885` là tổ tiên của `HEAD` (`987b98b8`); `git status` sạch trước khi review.

## 3. Đối chiếu tuyên bố với mã

| Tuyên bố | Bằng chứng | Kết quả |
|---|---|---|
| Chỉ mục dựng **một lần** ở `budget_enable` | `budget.rs:101-109` gọi `dung_chi_muc_phi()`; `bat_dau_ngan_sach_vm` chỉ `budget_enable` khi `budget_fee.is_none()` (`budget.rs:61-69`) | PASS |
| `budget_charge` tra O(1), bỏ `[None; 16]` + quét tuyến tính | `budget.rs:246-253` lấy `(start,len)` rồi duyệt `budget_flat[start..start+len]` | PASS |
| Giữ thứ tự entry trong từng pc | Hai lượt: đếm `dem[pc]` rồi rải theo con trỏ pc (`budget.rs:146-177`); thứ tự bảng gốc được bảo toàn | PASS |
| Giữ trần `MAX_PER_PC=16` | `dem` chỉ tăng tới 16 (`budget.rs:150-152`); entry thừa bị bỏ như đường cũ | PASS |
| Giữ điểm trap `VS0510` | Cùng thứ tự kiểm `w > budget_remaining` (`budget.rs:258-270`); test độc lập §4 xác nhận đúng từng mốc | PASS |
| Bỏ ghi TLS mỗi sự kiện | `budget.rs:272` cộng `self.budget_used`; TLS chỉ ghi ở `ket_thuc_ngan_sach` (`budget.rs:185-187`) | PASS |
| Flush một lần ở mọi lối ra `Vm::run` | verify lỗi (`vm.rs:217`), mọi kết quả Ok/Trap/Ret/Pending sau cùng (`vm.rs:288`) | PASS |
| Flush ở mọi lối ra frame | verify lỗi (`vm_frame.rs:488`), `main_th` trap/lỗi (`vm_frame.rs:528,535`), cuối driver (`vm_frame.rs:823`) | PASS |
| Reset `budget_used` theo lần chạy | `vm.rs:214`, `vm_frame.rs:486`, `budget.rs:63,107` | PASS |
| Không đổi quota | `quota_charge(w)` vẫn gọi đúng một lần/sự kiện (`budget.rs:274`); không sửa mã quota | PASS |

Không còn đường nào ghi `budget_fee` ngoài `budget_enable` (grep toàn repo), nên chỉ mục không thể
lệch với bảng phí. `Vm.m` là borrow bất biến suốt đời `Vm` ⇒ `n_code` dùng lúc dựng chỉ mục luôn
khớp mã thực thi.

## 4. Kiểm logic độc lập (harness ngoài repo, không commit)

Viết harness riêng tại `/tmp/opencode/review-st315a/audit` (chỉ dùng API public/test-hook), chạy
sau khi target được rebuild sạch — **tất cả PASS**:

1. **Differential 25 bảng phí ngẫu nhiên** (5 seed × 5 mật độ 0/3/12/40/90 entry/hàm, gồm pc vượt
   `code.len()` và chỉ số hàm không tồn tại): dựng dãy `(fi,pc)` thực thi bằng bảng probe 1 entry/pc,
   rồi so **từng entry** (pc/kind/site) với thuật toán cũ mô phỏng lại (lọc theo pc, trần 16, thứ tự
   bảng). Khớp 100 %, kể cả thứ tự nhóm và `da_dung_gan_nhat()`.
2. **Trần `MAX_PER_PC`**: 20 entry tại pc0 + 3 entry pc1 của hàm vỏ ⇒ trace đúng 16 entry đầu theo
   thứ tự + 3 entry pc1; entry pc vượt mã và hàm không tồn tại bị bỏ, không panic.
3. **Index stale**: `budget_enable` lần hai trên **cùng Vm** với bảng khác ⇒ trace chỉ còn entry bảng
   mới (`[222]`), không lẫn entry bảng cũ; `budget_used=4` đúng.
4. **Trap**: 11 mức hạn (0,1,2,3,4,5,total/2,total-2,total-1,total,total+1) trên bảng ngẫu nhiên:
   trace đúng **tiền tố** gồm cả sự kiện gây trap; `da_dung_gan_nhat` = tổng weight trước sự kiện
   trap (không tính sự kiện bị trap); `VS0510` xuất hiện đúng khi trap; `total` và `total+1` không trap.
5. **`budget_fee=None`**: sau một lần chạy có phí (TLS ≠ 0), chạy Vm không bật phí ⇒ TLS = 0 (không
   còn giá trị cũ).
6. **Verify lỗi**: module `version=0` ⇒ exit 1, thông điệp `bytecode hỏng`, TLS = 0.
7. **Nhiều lần chạy cùng Vm**: `budget_used` = 5 ở cả hai lần (reset theo lần, không cộng dồn).
8. **Frame driver**: chương trình `async`/`spawn`/`await` — flush = Σ weight(trace); hạn 1 ⇒ trap
   `VS0510`, flush = Σ trace trừ sự kiện bị trap; chạy frame **không** phí sau đó ⇒ TLS = 0; frame
   verify lỗi ⇒ TLS = 0.
9. **Bảng phí rỗng** (`FeeTable::default()`) ⇒ không charge, không panic.

Ngoài ra chạy lại các suite có sẵn: **38** ca lọc `budget` (gồm `lat256`, `lat257_a/b`,
`vs3_budget*`, `vs3_worker*`, `vs3_worker_eval_*`, `vd260_frame3`) và **7** ca `quota`
(`lat257_fairness`, `vd259_frame2`, `vd260_frame3`) — **45 passed / 0 failed** (1 ignored là
`measure_workloads`). Async/worker dùng
`SharedBudget` riêng, không bị đổi bởi lát này.

## 5. Cổng máy (chạy lại sau khi main rebuild sạch `/home/ps/ViScript/target`)

| Cổng | Kết quả |
|---|---|
| `cargo test -p vis-vm` | **67 passed / 0 failed** (unit 3 + `diff_100k` 7 + `fuzz_mutate` 1 + `vm_it` 56) + doctest 0; `fuzz_mutate` 31,7 s |
| `cargo test -p vis-conformance --test lib_api_it lat256_budget -- --test-threads=1` | **9 passed / 1 ignored** (đúng như journal) |
| `./vis-dev gate --fast` | **PASS** — `3827 tests run: 3827 passed (2 slow), 4 skipped`; luật **R1..R9 OK** (R8 ≤ 40, 0 binary cấp 1 mới) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** (100.000 chương trình; 2.000 ca phụ cũng `lech=0`) |
| Harness review độc lập | **ALL AUDIT CHECKS PASSED** (§4) |

Ghi chú hạ tầng (không phải lỗi mã): lần chạy đầu trước khi main rebuild sạch target gặp
`doctest` báo thiếu rlib do tranh chấp `CARGO_TARGET_DIR` dùng chung; chạy lại một mình PASS.
Sau rebuild sạch, toàn bộ cổng trên chạy lại và xanh; số test gate tăng 3823 → **3827** so với lần
chạy lẫn artifact trước đó — con số **3827** mới là số đúng của cây `review/r315a`.

## 6. Đo hiệu năng

Máy dùng chung, tải dao động mạnh (load average 3–20 do các worktree khác chạy test/build), nên
**thời gian tuyệt đối qua CLI dao động ~2×** và không dùng để kết luận. Đo hai cách độc lập:

**(a) Engine-only, `CLOCK_THREAD_CPUTIME_ID`, in-process, 45 cặp xen kẽ** (nguồn `opt-bench.vis`,
cùng cây `r315a`, release):

| Chế độ | p50 CPU | min |
|---|---:|---:|
| interp | 312,3 ms | 293,6 ms |
| VM (ngân sách 40M) | 320,1 ms | 308,3 ms |
| VM `--unlimited-budget` | 315,8 ms | 305,6 ms |

⇒ trung vị **VM/interp = 1,0267 (+2,7 %)**; chênh trung vị **+8,6 ms**; overhead charge
**VM − unlimited = +3,9 ms**. **KPI `VM ≤ interp + 5%`: PASS.**

**(b) CLI `# time:` n=15 mỗi tổ hợp, cửa sổ máy êm (load ≈ 3,3)** — binary build từ `review/r315a`,
sha256 `5eb2973aed13f13be0fb361a18c61eea4c64535b92f16465fa436374e1833ceb`:

| Tổ hợp | min | p50 |
|---|---:|---:|
| interp | 334 ms | 368 ms |
| VM | 346 ms | 384 ms |
| VM `--unlimited-budget` | 343 ms | 398 ms |

⇒ p50 **VM/interp = 1,0435**, min = **1,036** (ngưỡng 1,05). Journal ghi `interp 329 / VM 344`
(tỉ lệ **1,0456**) — **tỉ lệ tái lập**, chênh tuyệt đối ~12 % do trạng thái máy.

Đã giữ và kiểm binary mà journal ghim: sha256
`657458382521907f2c9f3c96f09d7acab1612c2543475d9144445cd33d7cb48c` (khớp log
`/home/ps/vislogs/st315a_opt_bench.md`);
đo lại trên máy nhiễu cho tỉ lệ min 1,01–1,09 tuỳ cửa sổ, không mâu thuẫn KPI. Binary `HEAD` của
worktree review có sha khác (`5eb2973a…`) vì `review/r315a` đứng sau `50bb7885` (các commit
ST-315c/ST-315d), không phải sai lệch số đo.

## 7. Ghi nhận & khuyến nghị (không chặn)

- **F1 — thấp, bộ nhớ**: `budget_idx` dày `(u32,u32)` cho **mọi pc**, kể cả hàm không có entry
  (`budget.rs:142-145` và `173-177`) ⇒ ~8 B/lệnh; hàm 1M lệnh tốn ~8 MB dù bảng phí rỗng. Không ảnh
  hưởng đúng đắn/KPI; nếu muốn tối ưu bộ nhớ sau này có thể dùng dạng thưa (`Option<Box<[…]>>`).
- **F2 — thấp, độ phủ test**: ca mới `chi_muc_phi_giu_thu_tu_giua_cac_pc` chỉ phủ đường thuận.
  Đề nghị thêm **một** ca nhỏ khoá `MAX_PER_PC=16` (hoặc trap giữa nhóm) vào `lat256_budget.rs` để
  chống hồi quy lâu dài; không tạo binary cấp 1 nên R8 không đổi. Hành vi này hiện đã được kiểm
  ngoài repo (§4) nhưng sẽ không được giữ trong CI.
- **F3 — thông tin**: `da_dung_gan_nhat()` giờ trả **0** cho lần chạy không bật phí hoặc verify lỗi
  (trước giữ giá trị cũ trong TLS). Đây là cải thiện; consumer duy nhất ngoài test là báo cáo
  `VIS_BUDGET_REPORT` (`run_cmd.rs:898`) đọc **sau** khi run trả về nên không bị ảnh hưởng.

## 8. Đề nghị

**PASS** — duyệt `ST-315a` = `DONE`. Khi cập nhật `QUEUE.md`/`STATE.md`, có thể ghi kèm hai ghi
nhận F1/F2 làm nợ tối ưu nhỏ (không cần chặn Sóng 4). Không phát hiện lỗi cần sửa mã.
