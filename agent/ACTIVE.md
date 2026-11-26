# agent/ACTIVE.md — Bảng điều khiển tác chiến hiện hành (`VD-289` / `VD-291`, 2026-11-26)

> **Quyết định mới nhất**: `agent/JOURNAL/quyet-dinh-a0-2026-11-26-vd291.md` · `agent/DECISIONS.md` (`VD-291`) · Kiến trúc 5 tầng DAG: `agent/JOURNAL/quyet-dinh-a0-2026-11-24-vd289.md` · Gói duyệt Sóng 0–3: `VD-290`.

## 1. CHẾ ĐỘ HIỆN HÀNH — PHÁT TRIỂN KIẾN TRÚC TỐI ƯU (`VD-289` / `VD-291`)

- **TIẾP TỤC TẠM DỪNG VÔ THỜI HẠN** kế hoạch phát hành cũ (`ST-309` = `BLOCKED`, **không** soak 24h/4h, **không** fuzz 8h, **không** 3× full lặp lại, **không** `R37` JSON, **không** tag mới).
- **ĐÃ DUYỆT `DONE` (`VD-291`, 2026-11-26, nền `6ea63df` / mã `577405f`)**: đóng **`ST-314a`** (khép trọn chuỗi tái kiến trúc 5 tầng DAG + 4 đợt audit `W1..W4`) — `vis-wasm-conformance` **393 × 2, 0 hỏng** (`366 .out + 27 .trap`), `vis-wasm-sem` **390/390**, `vis-wasm-suite` **28/28**; DAG **23 crate / 5 tầng** + self-test trong `vis-dev check`; R8 **39 ≤ 40**; `StdId` **167/167** compile-time exhaustive; corpus **563/563** (so cả message); vá `const` + 6 lỗi bảo mật (`C1/C2/H1/H2/M1/M3`) + 18 lỗi `bin/*`; Linux gate **3817/3817**, Windows full **3.742/0 @`577405f1`**, 7,2M + 1M stdlib + 100k method diff **0 lệch**, 1,54 tỷ+ fuzz **0 vi phạm**; macOS clippy **PASS @`311d2288`** *(full gate & clippy hậu-security `mps` chuyển sang `ST-316c` khi máy rảnh)*.
- **Sổ nợ & Tài liệu đồng bộ ở `VD-291`**: đóng **`#108`** (PATH TOCTOU), thu hẹp `#127` (`ABI::V3` xong, còn `IOCTL_DEV`), **đăng ký `#197` (`mở` — A0 audit: `vis-host-linux/src/sandbox.rs:121` tệp đơn `(write)` thiếu `AccessFs::Truncate`)**; đồng bộ `docs/SPEC-lex.md`, `docs/SPEC.md` (gồm đính chính `doctor --json` = 5 khoá) và `docs/ViScript-thiet-ke.md` (0 dịch `DOC_BLOCKS`).
- **TRỤC HIỆN HÀNH — SÓNG 4 TỐI ƯU HIỆU NĂNG, BẢO MẬT & NGỮ NGHĨA (`ST-315a..ST-315d`)**: `ST-315a`/`ST-315c` đã **`REVIEW`**; `ST-315b`/`ST-315d` **đang thi công song song** (W2 `Value::Map` CoW, W4 `VS0304` xong + parity host).
  - **Hiệu chỉnh kiến trúc quyết định ở `VD-291`**: `ST-315b` **bắt buộc dùng `Value::Map(Rc<VisMap>)` CoW** + chỉ mục băm lười (`Option<Box<MapIndex>>`), vì `m.get(k)` hiện deep-clone toàn `Value::Map` mỗi lần đọc; `ST-315a` bỏ thêm TLS `DUNG_GAN_NHAT` per-event; `ST-315d` được cấp mã chẩn đoán mới **`VS0304`** (`VD-129`).

## 2. Bảng Trạng thái Sóng 4–5 (`ST-315a..ST-316c`)

| Sóng | Lát | Luồng | Trạng thái | Mục tiêu & KPI nghiệm thu |
|---|---|---|---|---|
| **Hậu-kiểm** | **`ST-314a`** | **W1/W4** | **`DONE`** | APPROVED `VD-291` (`6ea63df`/`577405f`): WASI 393×2, DAG 23 crate/5 tầng trong `check`, `StdId` 167/167, corpus 563, vá `const`/security/bin, đóng `#108`, Linux 3817/3817, Win 3742/0, macOS clippy @`311d2288` |
| **4** | **`ST-315a`** | **W1** | **`REVIEW`** | **VM `budget_charge` O(1) + bỏ TLS nóng**: chỉ mục `pc → (start, len)` phẳng dựng 1 lần ở `budget_enable`, bỏ `[None; 16]`, chuyển `DUNG_GAN_NHAT` thành `Vm.budget_used` (flush 1 lần lúc thoát). KPI: release `opt-bench` VM ≤ interp + 5% (`≤ 345 ms`), `vis-diff 100k lech=0` |
| **4** | **`ST-315b`** | **W2** | **`READY`** (đang thi công) | **`Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười**: `entries` + `Option<Box<MapIndex>>` (ngưỡng 8/16, tái dùng `bam_khoa_vo_huong`), khoá `size_of::<Value>() <= 64`, gom 1 nguồn map (`gan`/`vm_helpers`/`method`/`stdlib`/`json`), giữ nguyên Q1–Q3. KPI: release `opt-bench` ≤ 120 ms cả 2 engine, `mapbench N=316` ≥ 10× |
| **4** | **`ST-315c`** | **W3** | **`REVIEW`** | **Vá `#197` + đóng W3 M2 + tooling**: `sandbox.rs:121` thêm `AccessFs::Truncate` + Landlock test #19; `vis-plugin-host` dùng `Stdio::piped()` + trần 8 MiB (0 `unsafe`) + RAII dọn `thu_muc_driver`; rà W3 H3 Windows + `vis-soak` log ngoài repo (R7) |
| **4** | **`ST-315d`** | **W4** | **`READY`** (VS0304 xong, parity đang thi công) | **Chu trình `const` tĩnh (`VS0304`) + 66 hàm host**: cấp mã `VS0304` (`VD-129`, vi/en/ja/`API-CODES.txt`) bắt chu trình `const` ở `vis-types`; sửa gợi ý `VS0204` ở `khai_bao.rs:303`; thêm harness parity 66 hàm host vào `crates/vis-conformance/tests/misc_api_it/` (**R8 giữ 39 ≤ 40**) |
| **5** | **`ST-316a`** | **W1+W2** | `BLOCKED` | *(chờ `ST-315a/b`)* Mở rộng `Rc` CoW cho `Value::List(Rc<Vec<Value>>)` + `bin/vis-bench` (release, sha256, JSON) |
| **5** | **`ST-316b`** | **W3** | `BLOCKED` | *(chờ `ST-315d`)* Quét nợ correctness/LSP: đóng `#186`, `#7`, `#48` (`is_pub` HIR), `#145` |
| **5** | **`ST-316c`** | **W4** | `BLOCKED` | *(chờ Sóng 4 & `mps` rảnh)* Bù 3-OS macOS `mps` (`clippy` + `vis-gate-macos --full`), đối chuẩn `vis-bench` 3-OS & hồ sơ Go/No-Go mở lại `ST-309` (`#195` watchdog 2h) hay mở Sóng 6 |

## 3. Cổng Kiểm tra Gọn (Chế độ Phát triển — KHÔNG soak/fuzz nặng)

- **Cuối mỗi lát**: `./vis-dev gate --fast` PASS + `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0` cho lát chạm compiler/runtime/VM) + số đo release trực tiếp có sha256 (`ST-315a`/`ST-315b`).
- **Cuối Sóng 4**: 1× `cargo clippy --workspace --all-targets -- -D warnings` trên Linux + Windows (`psvn`, và `mps` khi rảnh) + 1× `bin/vis-targets --wasm` (gồm `vis-dag-check` + 3 harness WASI).
- **Luật cứng**: 0 `unsafe` (R2), R1 ≤ 1.500 (mục tiêu mềm ≤ 1.150), R5 (`SO-NO`), R7 (log ≤ 50 KB), **R8 ĐANG CHẠM 39/40 — CẤM TẠO TỆP `.rs` CẤP 1 MỚI** (mọi test mới vào `tests/<domain>/` + `#[path]`), R9, không `.github/workflows/`.
- **Quản trị nhánh (`AGENTS.md` §6)**: merge xong xóa worktree + nhánh local (`-d`) + `origin` trong 24h; ≤ 8 worktree; `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, đĩa trống > 10 GB.

## 4. Lịch Tác chiến theo Khung giờ DeepSeek (`VD-287`, chuẩn `AGENTS.md` §5)

| Khung giờ | UTC | JST (UTC+9) | ICT (UTC+7) | Tác chiến ưu tiên (Sóng 4 `ST-315a..d`) |
|---|---|---|---|---|
| **Cao điểm 1 & 2** *(2×)* | `01:00–04:00` · `06:00–10:00` | `10:00–13:00` · `15:00–19:00` | `08:00–11:00` · `13:00–17:00` | **0-LLM**: chạy kiểm cổng (`vis-diff 100k`, `stdlib_diff`, release bench, Windows full, `vis-targets --wasm`, `vis-gc-targets`) hoặc nghỉ gọi LLM |
| **Thấp điểm trưa** *(0,5×)* | `04:00–06:00` | `13:00–15:00` | `11:00–13:00` | Triển khai nhanh `ST-315a` (0,5 ngày) & khởi động `ST-315c` (`#197` + W3 M2) |
| **Thấp điểm chính & Cuối tuần** *(0,5×)* | `10:00–01:00` (+ T7–CN) | `19:00–10:00` | `17:00–08:00` | Triển khai song song `ST-315b` (`Rc<VisMap>` CoW), `ST-315d` (`VS0304` + 66 hàm host), hoàn tất Sóng 4 & nộp `REVIEW` |
