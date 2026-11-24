# ST-312a..d — Sóng 0–1 Tái kiến trúc (2026-11-24)

- **Mốc gốc**: `origin/main` @ `5967c9d` (VD-289). **Kết thúc Sóng 0–1**: `4e6f810`.
- **Chế độ**: Phát triển Kiến trúc (VD-289) — không soak/fuzz; cổng nhẹ mỗi lát.

## 0. Dừng pha phát hành & dọn kho (VD-288 §4)

- Đã dừng có kiểm soát soak Windows `win-h4-soak-13-b` (psvn) + wrapper `vis-remote` (ps);
  không còn tiến trình soak/fuzz nào trên `ps`/`psvn`/`mps`.
- WIP `R37` (json + patch) lưu `/home/ps/vislogs/cancel-r37-h4/`; không tạo `R37` JSON, không tag.
- Dọn nhánh: 144 nhánh đã merge (`git branch -d`) + 7 nhánh chưa merge nhưng nội dung đã hấp thụ
  (patch lưu `/home/ps/vislogs/cleanup-289/`). Còn **2** nhánh local (`main`, `plan/st309-qa`),
  **3** ref remote (`main`, `plan/st309-qa`, `HEAD`). Xóa 3 worktree còn lại → **1** worktree.
- `bin/vis-gc-targets`: OK, 23 GB trống (> 10 GB).

## 1. ST-312a — dọn 9 shim `vis_interp → vis_core` (`65ea2a0`)

- Xóa `vis-interp/src/{cookie,json}.rs`; 7 điểm còn lại chuyển `pub(crate)`
  (`value/mod.rs`, `rt.rs`, `builtin/mod.rs`, `stdlib.rs`, `stdlib/helper.rs`,
  `frame/core.rs`, `net/mod.rs`).
- `RunOutput` → `vis-core/src/run.rs`; `FrameConfig`/`FrameOutput` + `FakeEvent`/
  `ReactorEvent`/`ReactorOutcome`/`DiagSnapshot` → `vis-core/src/frame.rs`.
- `vis-vm` thêm dep `vis-core`; workspace dùng đường trực tiếp `vis_core::*`.

## 2. ST-312b — `vis-host-linux` vào workspace + `#196` (`02e881e`)

- `members` 19 crate (sau đó 20/21); `landlock` vào `[workspace.dependencies]`; xóa lock riêng;
  `bin/vis-targets` phủ 100%; `vis-gate-windows --full` thêm `clippy --all-targets -D warnings`.
- **#196**: `parse_duration` trả `None` khi không có cặp `<số><đơn vị>` (`"sai"`, `""`, `"250"`).

## 3. ST-312c — `crates/vis-hir` (`cd007f9`)

- Chuyển `hir`, `ty`, `symbols` (từ `lower.rs`), `cmdsig`, `method_ret`, `prelude`, `stdlib`.
- `vis-types` chỉ còn `lower` + `tyck`; `vis-core`/`vis-bytecode`/`vis-vm` **0 dep `vis-types`**
  (bytecode/vm chỉ dev-dep cho test hạ HIR).
- Test `MethodRet` dời `vis-types/tests/types_it/method_ret.rs` (tránh dev-cycle).

## 4. ST-312d — `crates/vis-http` + cắt `vis-pkg → vis-host-native` (`4e6f810`)

- `vis-core` 0 HTTP protocol (chỉ còn `cookie` cho trait `NetHost`).
- `vis-http`: `http_get`/`http_head`, URL/header/chunked/redirect, `async_http` state machine.
- Async: trait `AsyncHttpEnv`; adapter `vis-interp/src/net/mod.rs` (~110 dòng) nối
  `Host`/`Scheduler`; không đổi hành vi.
- `vis-pkg`: `tai_va_giai(..., net: &mut dyn NetHost)`; CLI tiêm `Net::moi`; unit test mock
  `NetHost` (không tự mở mạng). Feature `vis-pkg/tls` bỏ.
- `vis-interp`: `vis-types` xuống dev-dependencies.

## 5. Cổng đo (Linux `ps`)

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` (sau mỗi lát) | PASS |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | `lech=0` · 100 000 tổng · 58,6 s · 1705/s |
| `cargo check --workspace --all-targets` | sạch (rustc 1.100.0) |
| `bin/vis-targets` | 100% 21 crate · core PASS=43 · sqlite PASS=41 + 2 EXPECTED_RED · FAIL=0 |
| `net_pkg_it` | 156/156 |
| `async_it` | 339/339 (3 ignored) |
| `vis-pkg --lib` | 28/28 |
| `cargo clippy --workspace --all-targets -- -D warnings` | **Linux PASS · macOS (`mps`) PASS · Windows (`psvn`) PASS** |
| `bin/vis-wasm-suite --build` | **28/28** · runtime=node |
| `bin/vis-wasm-sem` | **388/390** PASS — 2 đỏ là nợ `#193` (đệ quy sâu: `ham/de-quy-1000`, `de-quy-vo-han`), đúng sàn A0 |

## 6. Ghi chú cho A0

- Sóng 2 (`ST-313a`/`ST-313b`) vẫn `BLOCKED` chờ A0 duyệt mở sau khi REVIEW Sóng 0–1.
- Clippy 3 OS chạy trên cây `4e6f810`: `psvn` `/c/Users/ps/viscript-moi` (log
  `/home/ps/vislogs/clippy-psvn-312.log`), `mps` `/Users/mps/dvchd/ViScript` (log
  `/home/ps/vislogs/clippy-mps-312.log`) — cả hai exit 0.
- `vis-wasm-suite` lần đầu đỏ 2 ca `da_tep` vì dùng **binary wasm cũ**; `--build` lại từ
  `vis-host-wasm` @ `4e6f810` ⇒ 28/28, xác nhận không phải hồi quy.
- `docs/ViScript-thiet-ke.md:927` còn câu "vis-host-linux ngoài workspace chính" — tài liệu
  thiết kế thuộc quyền A0, chưa sửa; đề nghị A0 cập nhật khi chốt kiến trúc.
