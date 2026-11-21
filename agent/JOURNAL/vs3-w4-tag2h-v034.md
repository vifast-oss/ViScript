<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — soak Windows 2 giờ trên tag `v0.3.4` (`8fa1f96`)

> **Người đo**: subagent VS3-W4, 2026-11-22/23.
> **Tag**: **`v0.3.4`** → commit **`8fa1f96720b5cb37613226e7713beb50527b3f71`**;
> `psvn` worktree **detached đúng tag**, `git describe --tags --exact-match` = `v0.3.4`, dirty 0.
> **Kết quả**: **830 vòng / 7215 s (2 giờ 00 phút 15 giây)**, **0 fail, 0 flake**; P3 10/10 +
> P2 14/14 + P1 8/8 mỗi vòng; 166 lần default+tls+net; h2/worker xen kẽ; host 54/54;
> targets 18/18 + 17/1. Test-only; không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tổng | **830 vòng / 7215 s** (~120,25 phút) |
| Mỗi vòng | P3 `vs3_worker_eval_vm` **10/10** + P2 **14/14** + P1 **8/8** (32 test) |
| Mỗi 5 vòng | thêm `vs3_default` **22/22** + `vs3_tls` **22/22** + `net` **9/9** — **166 lần** |
| Fail / flake | **0 / 0**; `10053` không tái hiện |
| Xen kẽ | h2 sync **debug 3×19/19 + release 2×19/19**; worker **debug 8×14/14 + release 2×14/14** |
| Tổng test | **35 593**, 0 fail |
| Process leak | `vis.exe` còn sống sau mỗi vòng: **0** (830/830) |
| Bộ nhớ | FreePhysicalMemory đầu **10 027 524 KB**, cuối **9 924 024 KB**; dao động 9,92–10,14 GB, **không đơn điệu** (cuối thấp hơn đầu ~101 MB) |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** (#156) |
| Blocker | **Không có** |

## 1. Môi trường / phương pháp

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree `/c/Users/ps/viscript-w4n` detached `v0.3.4` (`8fa1f967…`), dirty 0.
- Warm build debug (P1/P2/P3, default/tls/net, h2, worker) + release (h2, worker) trước soak.
- Vòng lặp không nghỉ tới 7200 s:
  - mỗi vòng: P3 → P2 → P1, đếm `(Get-Process vis).Count`;
  - vòng chia hết 5: thêm default + tls + net;
  - worker debug: vòng 50/150/250/350/450/550/650/750; worker release: 200/500;
  - h2 debug: 100/300/500/700; h2 release: 400/800;
  - mỗi ≥600 s ghi `REPORT` (vòng, thời gian, free memory, vis_proc, fail).
- Dừng ngay nếu bất kỳ lệnh nào rc ≠ 0 (không xảy ra).

## 2. Kết quả

- **830/830 vòng rc 0/0/0**; thời gian/vòng **5–6 s**, không tăng dần.
- 166 lần default+tls+net: **22/22 + 22/22 + 9/9** mỗi lần.
- 8 lần worker debug **14/14**; 2 lần worker release **14/14**.
- 3 lần h2 debug **19/19**; 2 lần h2 release **19/19**.
- Không `FAILED`/`panicked`; `10053` không tái hiện.
- `vis_proc=0` sau **830/830** vòng.
- `REPORT` mỗi 10 phút (11 mốc 10→111 phút): tất cả `fail=0`, `vis_proc=0`.

## 3. RSS / thread / fd

- P2 `muoi_vong_threads_fd_rss_khong_ro` + P3 `vm_khong_tao_them_thread_sau_khi_join`:
  xanh **mỗi vòng** (830 lần).
- FreePhysicalMemory dao động **9 917 760–10 137 816 KB**; đầu 10 027 524 KB, cuối 9 924 024 KB
  (cuối thấp hơn ~101 MB, không đơn điệu; các mốc 10 phút có lúc tăng) — không thấy xu hướng rò.
- Không đo handle/thread xuyên tiến trình (mỗi ca process riêng); dùng các ca trên +
  `vis_proc=0` làm bằng chứng.

## 4. Cuối soak

- host **54/54** (`host_posix_real` bỏ qua — Windows `test-child`).
- targets core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (#156); 0 FAIL.
- Tag/HEAD cuối `v0.3.4` / `8fa1f967`, dirty 0.

## 5. Log / bằng chứng

- Log đầy đủ 1180 dòng (từng vòng + extras + 11 REPORT + host/targets):
  `agent/JOURNAL/vs3-w4-tag2h-v034.log`,
  sha256 `ea7b00a89f259d6a02f46927fb71105fbe2cc2b3224886338547ff2f8830524f`.
- Bản gốc `psvn`: `/c/Users/ps/vislogs/w4n-soak.log` (cùng sha).
- Worktree `psvn` đã xoá sau đo; repo chính @ `73af17bd`, dirty 0; `C:` còn **181G**.
- Local `/` còn **22G** (>5G).

## 6. Kết luận

- Tag **`v0.3.4`** ổn định trên Windows qua **830 vòng / 2 giờ**, 0 fail/flake; không rò process
  theo `vis_proc=0` và các ca thread/fd/RSS; free memory dao động trong biên ~220 MB, không
  đơn điệu.
- Async net/TLS/net xen kẽ 166 lần; h2 sync + worker debug/release nhiều lần — đều xanh.
- Async-h2 vẫn **Unix-only/default-off** (không nằm trong soak).
- **Không blocker.**
