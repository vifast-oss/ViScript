<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — soak cuối trước tag @ `38d3fcf` (Windows, test-only)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`38d3fcf`** (code = `52850ee` + journals; VM thread test fix `eb99c19`;
> Linux gate 3571/3). `psvn` worktree **detached đúng `38d3fcf`**, dirty 0.
> **Kết quả**: **205 vòng / 1816 s (30 phút 16 giây)**, **0 fail, 0 flake**; P3 10/10 + P2 14/14 +
> P1 8/8 mỗi vòng; 41 lần default+tls+net; h2/worker xen kẽ; host 54/54; targets 18/18 + 17/1.
> Không sửa sản phẩm; không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tổng | **205 vòng / 1816 s** (~30,3 phút) |
| Mỗi vòng | P3 `vs3_worker_eval_vm` **10/10** + P2 **14/14** + P1 **8/8** (32 test) |
| Mỗi 5 vòng | thêm `vs3_default` **22/22** + `vs3_tls` **22/22** + `net` **9/9** — **41 lần** |
| Fail / flake | **0 / 0** |
| Xen kẽ | h2 sync **debug 2×19/19 + release 1×19/19**; worker **debug 2×14/14 + release 1×14/14** |
| Tổng test | **8 832**, 0 fail |
| Process leak | `vis.exe` còn sống sau mỗi vòng: **0** (205/205) |
| Bộ nhớ | FreePhysicalMemory **10 073 988 KB → 10 116 040 KB** (+42 052 KB ≈ +41,1 MB) |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** (#156) |
| Blocker | **Không có** |

## 1. Môi trường / phương pháp

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree `/c/Users/ps/viscript-w4l` detached `38d3fcf3`, dirty 0.
- Warm build debug (P1/P2/P3, default/tls/net, h2, worker) + release (h2, worker) trước soak.
- Vòng lặp không nghỉ tới 1800 s:
  - mỗi vòng: P3 → P2 → P1;
  - vòng chia hết 5: thêm default + tls + net;
  - vòng 30/100: worker debug; vòng 60: worker release;
  - vòng 20/120: h2 debug; vòng 80: h2 release.
- Sau mỗi vòng đếm `(Get-Process vis).Count`.

## 2. Kết quả

- **205/205 vòng rc 0/0/0**; thời gian/vòng **5–6 s**, không tăng dần.
- 41 lần default+tls+net: **22/22 + 22/22 + 9/9** mỗi lần (11–12 s).
- 2 lần worker debug **14/14**; 1 lần worker release **14/14**.
- 2 lần h2 debug **19/19**; 1 lần h2 release **19/19**.
- Không `FAILED`/`panicked`; `10053` không tái hiện.
- `vis_proc=0` sau **205/205** vòng.

## 3. RSS / thread / fd

- P2 `muoi_vong_threads_fd_rss_khong_ro` + P3 `vm_khong_tao_them_thread_sau_khi_join`:
  xanh **mỗi vòng** (205 lần).
- FreePhysicalMemory: **10 073 988 KB → 10 116 040 KB** (+41,1 MB), không giảm.
- Không đo handle/thread xuyên tiến trình (mỗi ca process riêng); dùng 2 ca trên + `vis_proc=0`.

## 4. Cuối soak

- host **54/54** (`host_posix_real` bỏ qua — Windows `test-child`).
- targets core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (#156); 0 FAIL.
- HEAD cuối `38d3fcf3`, dirty 0.

## 5. Log / bằng chứng

- Log đầy đủ 410 dòng: `agent/JOURNAL/vs3-w4-tagsoak-38d3fcf.log`,
  sha256 `16b74b469708a1d81714054af4657123741a7985da4e515c39cfbb90a3375e0e`.
- Bản gốc `psvn`: `/c/Users/ps/vislogs/w4l-soak.log` (cùng sha).
- Worktree `psvn` đã xoá sau đo; repo chính @ `73af17bd`, dirty 0; `C:` còn **180G**.
- Local `/` còn **21G** (>5G).

## 6. Kết luận

- Commit **`38d3fcf`** ổn định trên Windows qua 205 vòng / 30 phút: 0 fail/flake, không rò
  process/memory theo các phép đo sẵn có; host/targets xanh.
- Windows async net/TLS/HTTP-1.1 + evaluator P1/P2/P3 đạt phạm vi đo; async-h2 vẫn **Unix-only**.
- **Không blocker** cho tag.
