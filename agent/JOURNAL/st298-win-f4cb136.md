<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-298 — Windows `f4cb136`: 3× full + soak 4h (Tier 1 candidate)

> **Người đo**: subagent VS3-W4, 2026-11-23 · deadline nội bộ 3× full <04:50 JST, soak <08:50 JST.
> **Mốc**: `f4cb136da45c18f19671b50641b48696f981e8ff` (`merge ST-297`; gate Linux 3660/4).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-st298-f4cb136`, dirty **0**; không job psvn khác;
> `TMPDIR=/c/Users/ps/tmp`; log ngoài repo. Không tag/push/merge; không GitHub Actions.
> **Kết quả**: **3/3 full PASS**, **soak 985 vòng / 4h00m12s, 0 fail sản phẩm** (1 env flake).
> **Đề xuất: ĐỦ điều kiện tuyên bố Windows Tier 1** (A0 quyết định).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| A targeted ST-297 | `vis-lsp` **29/29**, `path_impl::tests` **2/2**, `vis-loader` **29/29** |
| B 3× `vis-gate-windows --full` | **3/3 PASS** — mỗi lần **3611 passed / 0 failed / 3 ignored**, host 54/54, targets 18/18 + 17/1, dirty **0** |
| Wall 3× full | **447 s / 335 s / 333 s** |
| C soak 4h | **985 vòng / 14 412 s (4h00m12s)**, **0 fail sản phẩm**, **1 env flake** R1 p1 `rc=127` (rerun 8/0) |
| Free RAM | **8204–8766 MB** (end 8580) |
| fd/handle | **không đo được sau soak** (`NO_PROC` — không còn `vis`/`cargo`/`rustc`); dùng **proc/threads/wsMB** max **2/14/18,1 MB** + P2/P3 tests nội bộ |
| Kết thúc | HEAD `f4cb136`, **dirty_end 0**, C: còn **67 G** |

## 1. A — Targeted ST-297 (Windows)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-lsp` | **29/29 passed** (0,06 s; log `st298-lsp.log`, sha256 `0a2b686d…2ec9e7`) |
| `cargo test --bin vis path_impl::tests` | **2/2 passed** (2,73 s; `st298-pathimpl.log`, sha256 `983da034…868d7f58`) |
| `cargo test -p vis-loader` | **29/29 passed** (4,21 s; `st298-loader.log`, sha256 `845ef8a3…17bc4e12`) |

## 2. B — 3× full liên tiếp

| Lần | Wall | Workspace | host | targets | dirty sau |
|---|---:|---|---|---|---:|
| 1 | **447 s** | 3611/0/3 | 54/54 | 18/18 + 17/1 | **0** |
| 2 | **335 s** | 3611/0/3 | 54/54 | 18/18 + 17/1 | **0** |
| 3 | **333 s** | 3611/0/3 | 54/54 | 18/18 + 17/1 | **0** |

- Kỳ vọng đề bài 3607; thực tế **3611** — **+4 ca ST-297** (UNC/mixed/percent + `--grant`), không fail.
- Log local sha256:
  - FULL1 `8315b85ffc86f456bb24eb22624993794688f7596c0575acbc43db3ae288f01e`
  - FULL2 `129aaa214c8988622cfc06f8744bebf760b17c3432e9135e8a8f7fbff3c6c593`
  - FULL3 `d0afe2017c91d5a4e8b4aa057e04342b9c2d28bd4098ea51ee6da62ec952c080`

## 3. C — Soak 4h

- Worktree `f4cb136`, script `/c/Users/ps/vislogs/st298-soak.sh`, start **04:31:30 JST**,
  end **08:31:42 JST**; **985 vòng**; tổng **14 412 s**.
- Mỗi vòng: P2 14/14 + P1 8/8 + P3 10/10 + `vs3_select` 25/25 — **985/985 vòng**.
- Mỗi 10 vòng: `http2_it` **33/33** (98 lần); `cancel_http_` default **8/8** + serial **8/8**
  (98 vòng ×2 = **196 lượt suite**), tất cả xanh.
- **Fail sản phẩm: 0.** Duy nhất **1 env flake**: `R1 p1 rc=127` (test chưa chạy, rerun ngay `8/0`).
  Ghi nhận đúng, không retry mù; không có lỗi sản phẩm nào khác.
- Free RAM **8204–8766 MB** (min/max trên 985 mẫu; end 8580) — phẳng, không xu hướng rò rỉ.
- **fd/handle**: sau soak `Get-Process` không còn tiến trình `vis`/`cargo`/`rustc` ⇒ **không đo được
  handle**. Ghi đúng: **“fd/handle không đo được sau soak; dùng RSS/proc/threads + freeMB”**.
  Mẫu proc/threads/wsMB tại mốc vòng: max **2/14/18,1 MB** (R199), còn lại 0 — test P2/P3 có kiểm
  fd/RSS/thread nội bộ.
- dirty_end **0**; HEAD_END `f4cb136`; C: còn **67 G**.
- Summary: `/c/Users/ps/vislogs/st298-soak-summary.log`, sha256
  `14409ef41d4f02655cb855dc62b19854092c8a344baac4ca5e13d1ff2c86f3b4`.

## 4. Kết luận / đề xuất Tier 1

- **Đủ bằng chứng tuyên bố Windows Tier 1**: 3× full liên tiếp **3611/0/3**, host 54/54,
  targets đạt, dirty 0; soak **4h/985 vòng, 0 fail sản phẩm**, RAM phẳng, cancel/h2/select ổn định.
- Caveat phải giữ: **Windows FsGuard chỉ kiểm theo đường dẫn** (không danh tính tệp), **không có
  Landlock**; `async-h2`/proc/sqlite có ràng buộc riêng; **handle count không đo được sau soak**
  (dùng RSS/proc/threads); macOS ngoài phạm vi.
- Docs `HOST-BAO-DAM`/`ROADMAP`/`ACTIVE` trên nhánh `plan/v045-prep` đã điền số soak nhưng vẫn ghi
  **CHƯA TUYÊN BỐ** — A0 quyết định tuyên bố chính thức khi tag `v0.4.5`.

## 5. Blocker

- **Không blocker sản phẩm.** Không tag/push/merge; không GitHub Actions.
