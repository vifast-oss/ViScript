<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Kế hoạch Chạy Xuyên đêm Tối ưu tới 09:00 Sáng 2026-11-23 (`VD-282`)

> **Ban hành**: 2026-11-22 23:55 JST (21:55 ICT) · **Cửa sổ thực thi**: **~9 giờ (00:00 → 09:00 JST)** / **~11 giờ (22:00 → 09:00 ICT)**
> **Baseline**: `origin/main` @ `VD-282` (sau `d53d2588`) · **Mục tiêu 09:00 sáng mai**:
> 1. **Hoàn tất `ST-291` và đẩy `ST-292` xong toàn phần (hoặc ít nhất xong P4a+P4b+SharedBudget sẵn sàng chốt `v0.4.0`)** trên W1 — đường găng duy nhất của `v0.4.0`.
> 2. **Merge xong `ST-296d-tools`** (vá 2 lỗi soak/timing + Luật R7 + `.config/nextest.toml` + 4 script vận hành) để cả đêm chạy êm không sập tải/hết đĩa.
> 3. **Hoàn tất sẵn trên nhánh/workspace cô lập** cả 3 cụm tăng tốc: **`ST-299a`** (`vis-host-wasm/` WasiHost thật trên `wasmtime`), **`ST-296a`** (gom 280 → 13 test binary), và **`ST-296b`** (tách `vis-loader` + `vis-lsp`).
> 4. Nếu W1 chốt kịp `ST-292` trước 07:00 sáng: chạy trọn **final QA `ST-296` + tag `v0.4.0`**, rồi rebase & merge ngay `ST-296a` + `ST-296b` để sáng mai mở mắt đã gần xong luôn `v0.4.1`!

---

## 1. Nguyên tắc Vàng bảo vệ Tài nguyên & Tiến độ Xuyên đêm

1. **Tuyệt đối không gián đoạn W1**: nếu W1 đang chạy `ST-291` (hoặc đang test/build), **đợi xong bước hiện tại**, KHÔNG `kill`, KHÔNG `git reset --hard`, KHÔNG đụng worktree W1.
2. **Ngân sách CPU cứng chống load 27** (tránh lặp lại sự cố run 3 `hieu_nang_cache_5_tep_duoi_2ms` ở `gd1-2h-soak-832787e.md`):
   - Mỗi luồng đặt tường minh:
     `export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`
   - Nếu `sccache` có sẵn: `export RUSTC_WRAPPER=sccache`.
   - **Không chạy cùng lúc 2 vòng `soak` / `diff 100k` / full `gate --fast`** giữa các worktree; xếp lịch chạy full gate lệch pha nhau (nhất là trong khi W1 chạy benchmark `ST-291`).
3. **Kỷ luật `main` sạch trước tag `v0.4.0`**:
   - **Được phép merge thẳng vào `main` trước `v0.4.0`**: `ST-296d-tools` (chỉ tool/test-timing/R7), `ST-299a` (chỉ trong `vis-host-wasm/` ngoài workspace chính), `ST-291`, `ST-292`, và chốt `ST-296`.
   - **Giữ trên nhánh riêng (KHÔNG merge vào `main` trước khi tag `v0.4.0`)**: `plan/w4-st296a-tests` (`ST-296a`) và `plan/w3-st296b-crates` (`ST-296b`).
4. **Kỷ luật R7 & không rác commit**:
   - Không commit `.log`/`.txt` > 50 KB vào `agent/JOURNAL/`; log dài đặt ở `/home/ps/vislogs/` và ghi đường dẫn + `sha256` trong báo cáo `.md` ≤ 15 KB.
   - Không commit `R33-viscript.json` nháp thêm lần nào nữa cho đến bước chốt cuối `ST-296`.
   - Dọn `target/` các worktree cũ trước khi bắt đầu để trống đĩa > 20 GB (ngưỡng cứng không dưới 10 GB).

---

## 2. Lịch trình Tối ưu theo Khung giờ (4 Luồng Song song)

| Khung giờ (JST) | Khung giờ (ICT) | **W1 (`plan/w1-runtime`) — ĐƯỜNG GĂNG** | **W4 (`plan/w4-qa-docs`) — HẠ TẦNG & TEST** | **W3 (`plan/w3-platform`) — TÁCH CRATE** | **W2 (`plan/w2-net-sec`) — WASI ĐỘC LẬP** |
|---|---|---|---|---|---|
| **00:00 – 01:30** *(T+0h..1,5h)* | **22:00 – 23:30** | Tiếp tục hoàn thiện **`ST-291`** (`P2b-1/2`: `PendingJob`/`flush_jobs`, `Vec<Option<TaskProxy>>`), chạy bench release N=10k W=4 ≤ 25–30ms, 20 vòng RSS/fd phẳng | Pull `VD-282`; triển khai & **merge ngay `ST-296d-tools`** (fix `bin/vis-soak` TMPDIR ngoài `target/`, fix `hieu_nang_cache_5_tep_duoi_2ms`, Luật **R7**, `.config/nextest.toml`, `bin/vis-{gc-targets,jobs,status,gate-windows}`) | Pull `VD-282`; khởi tạo nhánh `plan/w3-st296b-crates`; tách 4 hàm thuần sang `vis-loader::lock_meta` + dựng khung `crates/vis-loader` + `[workspace.dependencies]` | Pull `VD-282`; khởi tạo nhánh `plan/w2-st299a-wasi` trong **`vis-host-wasm/`**; kiểm `wasm32-wasip1` target + `wasmtime` trên `ps`; dựng khung `WasiHost` (FS preopened, clock, stdout/stderr, deny net/proc/sqlite) |
| **01:30 – 03:30** *(T+1,5h..3,5h)* | **23:30 – 01:30** | Chốt gate + diff 100k cho **`ST-291`**, ghi `JOURNAL/st291-p2b.md`, **merge `ST-291` vào `main`**; mở ngay **`ST-292` (P4a)** (`OutcomeBoard::wait_any_timeout`, 5 hook `Host`, `frame/worker.rs`) | Tạo nhánh `plan/w4-st296a-tests` từ `main` (sau `ST-296d-tools`); dựng `tests/common/mod.rs`, gom 92 `vd270_*` + 28 `vd26x_*` → `stdlib_str_path_it.rs` + `stdlib_col_fs_it.rs`, chạy xanh 100% test count | Hoàn thiện `crates/vis-loader` (biên dịch xanh cả native lẫn `wasm32-unknown-unknown` / `wasm32-wasip1`), bắt đầu tách `crates/vis-lsp` (25 tệp + 6 điểm gọi `vis_loader::modules`) | Cài xong `WasiHost` + binary `vis-wasm-run` (single-file interp + VM) trên `wasm32-wasip1`; chạy ≥ 10 ca smoke `wasmtime` (gồm đọc/ghi trong preopened dir, chặn thoát gốc, từ chối `net`/`proc`/`sqlite` rõ ràng, 0 panic, 0 `unsafe`) |
| **03:30 – 06:00** *(T+3,5h..6h)* | **01:30 – 04:00** | **`ST-292` (P4b + P4c)**: `vm_frame.rs` pump 50ms, flush output theo thứ tự spawn, cancel worker, từ chối `chan(0)`/host trong worker, `VS0509` cross-write, cờ CLI `--workers <N>` + **nối `gioi_han_phien()` vào `SharedBudget`** (`VD-281`) | Gom nốt các nhóm còn lại trong `ST-296a` (`lang_core_it`, `cli_tools_it`, `async_chan_it`, `security_it`, `sqlite_it`, `net_tls_it`, `http2_it`, `pkg_it`, `pkg_sig_it`; giữ nguyên `vs3_worker*` cho bước rebase cuối); đo bảng so sánh thời gian & `target/` | Chuyển 15 tệp `lsp*.rs` sang `crates/vis-lsp/tests/` (tách `lsp.rs` 1.478 dòng nếu gần R1); chạy `./vis-dev gate --fast` + `./bin/vis-targets` (core 18/18) xanh hoàn toàn trên `plan/w3-st296b-crates`; ghi `JOURNAL/st296b-crates.md` | Hoàn thiện **`ST-299a`** (test parity interp↔VM trên `wasmtime` ≥ 20 ca đại diện, `cargo check --manifest-path vis-host-wasm/Cargo.toml --target wasm32-wasip1`, 0 `unsafe`, ghi `JOURNAL/st299a-wasi-core.md`), **merge `ST-299a` vào `main`** (0% xung đột!) |
| **06:00 – 07:30** *(T+6h..7,5h)* | **04:00 – 05:30** | **`ST-292` Kiểm chứng toàn phần**: viết & chạy `vs3_worker_frame.rs` (≥ 12 ca, gồm `--workers 4 --max-budget 5000` → `VS0510`) + `vs3_worker_frame_vm.rs` ×20 vòng + soak 10 phút + verify `psvn` Windows; ghi `JOURNAL/st292-worker-frame.md`, **merge `ST-292` vào `main`** | Chạy xác nhận `ST-296a` (số test giữ 100%, đo tốc độ + `target/`), ghi `JOURNAL/st296a-tests.md`, giữ nhánh sẵn sàng. **Ngay khi `ST-291`/`ST-292` vào `main`**, chuyển sang **chốt cuối `ST-296`** | Nếu `ST-296b` xong sớm: rà sẵn danh sách điểm sửa `#149` (`ST-297`, chỉ đọc/lập kế hoạch sửa trong `JOURNAL/st297-plan.md`, chưa chạm `main`) để sẵn sàng sau `v0.4.1` | Hỗ trợ W4 chạy Windows `psvn` verify cho `ST-291` và `ST-292` bằng `bin/vis-gate-windows --quick` rồi `--full` (tránh dồn 1 luồng) |
| **07:30 – 09:00** *(T+7,5h..9h)* | **05:30 – 07:00..09:00** | **Nếu `ST-292` đã merge**: nghỉ/chờ `v0.4.0` tag rồi khởi động ngay **`ST-296c`** (gom stdlib + tách 3 tệp R1). **Nếu `ST-292` còn dở**: tập trung chốt dứt điểm `ST-292` + gate xanh trước 09:00 | **Kịch bản A (`ST-292` đã merge)**: chạy final QA `ST-296` (fresh-clone Linux + `bin/vis-gate-windows --full` + 2× soak 10/10 + diff 100k ×3 seed + chốt `R33-viscript.{md,json}` + chuẩn bị sẵn mọi số liệu chờ A0 duyệt tag `v0.4.0`, **KHÔNG tự tag nếu chưa đủ tiêu chí `ST-296`**). **Kịch bản B (`ST-292` đang chốt)**: chạy soak 2h xanh trên `main` (đã có fix `ST-296d-tools`) + gom báo cáo tiến độ sáng 09:00 | Chờ chốt `v0.4.0` để rebase & merge `ST-296b` làm commit đầu tiên của `v0.4.1` | Tổng hợp kiểm tra `vis-gc-targets`, đảm bảo đĩa trống > 15 GB, tất cả job nền sạch sẽ, tóm tắt sáng 09:00 trong `JOURNAL/bao-cao-sang-2026-11-23.md` |

---

## 3. Cây Quyết định Tự động (Dành cho Agent 2 khi chạy không giám sát)

- **Nếu gặp fail khi chạy soak/gate**: tuân thủ luật cũ — **dừng vòng đó ngay, phân loại rõ (lỗi sản phẩm vs T-INFRA/timing), KHÔNG retry lấy xanh**, ghi log ở `/home/ps/vislogs/` + tóm tắt `.md`.
- **Nếu `ST-291` hoặc `ST-292` chạm ngưỡng thời gian/phức tạp**: ưu tiên **chất lượng & đúng bất biến (`SharedBudget`, `VS0509`, byte-identical W=1/2/4)** hơn là chạy ép gắn tag `v0.4.0` trong đêm. Thà 09:00 sáng có **`ST-291` DONE + `ST-292` REVIEW sạch + `ST-296d-tools` DONE + `ST-299a` DONE + `ST-296a`/`ST-296b` READY-TO-MERGE**, còn hơn gắn tag `v0.4.0` vội khi thiếu bằng chứng Windows/soak!
- **Nếu thiếu `wasmtime` trên `ps` cho `ST-299a`**: vẫn triển khai đầy đủ `vis-host-wasm/` + `cargo check --target wasm32-wasip1` + unit test logic `WasiHost` trên native, đồng thời ghi `BLOCKED` cho bước chạy thực thi `wasmtime` (không tự tải binary lạ ngoài nguồn chính thức).
- **Mốc báo cáo sáng 09:00**: tạo duy nhất **1 tệp tóm tắt** `agent/JOURNAL/bao-cao-sang-2026-11-23.md` (≤ 80 dòng) liệt kê: commit hash của `ST-291`, `ST-292`, `ST-296d-tools`, `ST-299a`, trạng thái nhánh `ST-296a`/`ST-296b`, số đo `gate --fast` Linux + Windows, dung lượng đĩa trống, và danh sách mục cần A0 duyệt lúc 09:00.
