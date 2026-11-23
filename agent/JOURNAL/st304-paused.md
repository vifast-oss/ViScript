<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-304 — TẠM DỪNG do cao điểm (chưa có mã `crates/vish-edit`)

> **Người làm**: W3, 2026-11-23 · nhánh **`plan/w3-st304-vish`** từ main **`700bb66`**.
> **Trạng thái: PAUSED theo chỉ đạo người dùng** (cao điểm CPU); tiếp tục **13:00–15:00**.
> Không merge/push/tag; không ssh `mps`; **không** chạy build/test/gate mới trong lượt dừng này.

## 1. Đã làm (chỉ khảo sát, chưa tạo tệp)

- Đọc `VD-284`/`VD-283` làn 4: `crates/vish-edit` (ViShell MVP phần A) cần raw mode đa nền,
  nhập nhiều dòng theo thụt lề, tô màu CST, lịch sử append-only + tìm mờ, Ctrl-C 130, panic giữ
  phiên, khởi động ấm < 50 ms, test PTY giả Linux/macOS.
- Khảo sát API: `rustix =1.1.6` hiện chỉ bật `fs`+`std`; `rustix::termios` cần thêm feature
  `termios` (Unix/macOS). Windows cần raw-mode an toàn; `crossterm =0.29.0` đã có trong cache
  crate (MIT) và đã được VD-283 duyệt có điều kiện — dự kiến dùng **rustix termios cho Unix** +
  **crossterm Windows** (cfg target), kèm log license `cargo metadata`.
- Chưa tạo `crates/vish-edit/`, chưa sửa `Cargo.toml`, chưa chạy `cargo`/gate.

## 2. Trạng thái git

- `git status --short` **rỗng** trước khi thêm journal này.
- Commit này chỉ thêm journal; **không** có mã `vish-edit` để commit (partial = rỗng).

## 3. Việc tiếp theo (13:00–15:00)

1. Tạo `crates/vish-edit` + `Cargo.toml` (workspace member), bật `rustix/termios`; Windows
   `crossterm` pinned `=0.29.0`, `default-features = false`.
2. API tối thiểu: buffer/cursor, `Key`/`feed_key`, history append-only + fuzzy, render ANSI theo
   token CST; `run()` raw-mode + `Drop` restore; Ctrl-C → `ExitCode 130`.
3. Test PTY giả Linux/macOS + cfg Windows; đo warm release < 50 ms.
4. `cargo metadata` → `agent/JOURNAL/st304-license.log` (allowlist MIT/Apache-2.0/Zlib/Unicode-3.0).
5. `cargo test -p vish-edit`; gate `./vis-dev gate --fast` ≥ **3671/4**; R1 ≤1500; 0 unsafe.

## 4. Blocker

Không có blocker kỹ thuật; chỉ tạm dừng vì cao điểm CPU. Không merge/push/tag.
