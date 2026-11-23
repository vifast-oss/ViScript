<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — soak Windows P2 ~30 phút @ `ffe202b` (test-only)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`ffe202b`** (P2 + P2 soak docs; Linux gate **3561/3**).
> `psvn` worktree **detached đúng `ffe202b`**, dirty 0.
> **Kết quả**: **233 vòng / 1802 s (30 phút 02 giây)**, **0 fail, 0 flake**; mỗi vòng
> `eval_parallel` debug **14/14** + release **14/14** + `eval_local` **8/8**; xen kẽ
> `default+net` 46 lần, `worker` debug 23 lần, `worker` release 1 lần; host 54/54; targets
> 18/18 + 17/1. Không sửa sản phẩm; không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tổng | **233 vòng / 1802 s** (~30,0 phút) |
| Mỗi vòng | `vs3_worker_eval_parallel` debug **14/14** + release **14/14** + `vs3_worker_eval_local` **8/8** (36 test) |
| Fail / flake | **0 / 0**; dừng ngay nếu fail (không xảy ra) |
| Xen kẽ | `vs3_default + net` **46 lần** (22/22 + 9/9); `vs3_worker` debug **23 lần** (14/14); release **1 lần** (14/14) |
| Tổng test | 8 388 (vòng) + 1 426 (default+net) + 322 (worker debug) + 14 (worker release) = **10 150**, 0 fail |
| Process leak | `vis.exe` còn sống sau mỗi vòng: **0** (233/233 lần) |
| Bộ nhớ | FreePhysicalMemory **10 088 336 KB → 10 125 896 KB** (+37 MB, không giảm) |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Blocker | **Không có** |

## 1. Môi trường / phương pháp

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree `/c/Users/ps/viscript-w4g` detached `ffe202b8`, dirty 0.
- Warm build debug (parallel/local/default/net/worker) + release (parallel/worker) trước soak.
- Vòng lặp **không nghỉ** tới 1800 s:
  - mỗi vòng: `cargo test -p vis-cli --test vs3_worker_eval_parallel` (debug),
    `cargo test --release -p vis-cli --test vs3_worker_eval_parallel`,
    `cargo test -p vis-cli --test vs3_worker_eval_local`;
  - vòng chia hết 5: thêm `--test vs3_default --test net`;
  - vòng chia hết 10: thêm `--test vs3_worker` (debug);
  - vòng 3: thêm `vs3_worker` release.
- Sau mỗi vòng đếm `(Get-Process vis).Count` — kỳ vọng 0 (không rò tiến trình).

## 2. Kết quả

- **233/233 vòng rc 0/0/0**; thời gian/vòng 3–4 s (vòng 1 4 s), không tăng dần.
- `eval_parallel` debug 1,4–1,5 s; release 0,48–0,55 s; `eval_local` 0,20–0,23 s.
- 46 lần `default+net`: **22/22 + 9/9** mỗi lần.
- 23 lần `worker` debug: **14/14** mỗi lần (30,6–31,7 s/lần).
- 1 lần `worker` release: **14/14**.
- Không có `FAILED`/`panicked` trong bất kỳ log nào; `10053` không tái hiện.
- Không có `vis.exe` nào còn sống sau vòng nào (`vis_proc=0` 233/233) ⇒ không rò process.

## 3. RSS / thread / fd

- P2 có ca `muoi_vong_threads_fd_rss_khong_ro` chạy **mỗi vòng** (debug + release) — xanh
  233 lần ⇒ thread/fd/RSS trong worker không rò theo ca kiểm của test.
- FreePhysicalMemory toàn máy: **10 088 336 KB** đầu soak → **10 125 896 KB** cuối soak
  (tăng ~37 MB; không giảm theo thời gian) — không có dấu hiệu rò bộ nhớ.
- Không đo handle/thread count xuyên tiến trình (mỗi ca là process riêng); dùng
  `vis_proc=0` + ca P2 làm bằng chứng thay thế.

## 4. Cuối soak

- `bin/vis-host-suite`: **54/54** (`host_posix_real` bỏ qua — Windows `test-child`).
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (#156); 0 FAIL.
- HEAD cuối `ffe202b8`, dirty 0.

## 5. Log / bằng chứng

- Log đầy đủ 458 dòng: `agent/JOURNAL/vs3-w4-p2winsoak.log`,
  sha256 `91c85711d952673a373bd87745130b779fd329e14d7dc9c1bcfe9feb6ea7594d`.
- Bản gốc `psvn`: `/c/Users/ps/vislogs/w4g-soak.log` (cùng sha); wrapper
  `sha256=deb35029…91fba`.
- Worktree `psvn` đã xoá sau đo; repo chính `psvn` @ `73af17bd`, dirty 0; `C:` còn **181G**.
- Local `/` còn **22G** (>5G).

## 6. Kết luận

- Evaluator P2 trên Windows ổn định qua **233 vòng / 30 phút**, 0 fail/flake; không rò
  process/memory theo các phép đo sẵn có.
- `vs3_default`/`net`/`vs3_worker` xen kẽ đều xanh; host/targets cuối soak xanh.
- Async-h2 vẫn **Unix-only/default-off** (không nằm trong soak).
- **Không blocker.**
