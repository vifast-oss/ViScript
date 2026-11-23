<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — soak Windows ~30 phút @ `83b4002` (test-only)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`83b4002`** (code = `fdd9e88` + docs/infra; Linux gate **3547/3**).
> `psvn` worktree **detached đúng `83b4002`**, dirty 0.
> **Kết quả**: **137 vòng / 1811 s (~30,2 phút)**, **0 fail, 0 flake**; kèm 5 vòng h2 debug
> 19/19, 1 h2 release 19/19, worker debug+release 14/14; host 54/54 + targets 18/18 + 17/1.
> Không sửa sản phẩm; không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tổng thời gian soak | **1811 s** (30 phút 11 giây), 137 vòng |
| Mỗi vòng | **net 9/9 + vs3_default 22/22 + vs3_tls 22/22 + eval_local 8/8** (61 test) |
| Fail / flake | **0 / 0**; `10053` không tái hiện; không vòng nào quá 14 s |
| `vs3_http2` (sync h2) | **5 vòng debug 19/19** + **1 release 19/19** |
| `vs3_worker` | **debug 14/14** + **release 14/14** |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** (#156) |
| fd/thread | ca fd-leak 200 vòng nằm trong `vs3_default` — xanh **mỗi vòng**; không đo OS handle/thread xuyên tiến trình (mỗi ca process riêng) |
| Blocker | **Không có** |

## 1. Môi trường / phương pháp

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Worktree `/c/Users/ps/viscript-w4e` detached `83b40020`, dirty 0.
- Warm build trước soak (debug 4 suite + h2/worker debug + h2/worker release), rồi vòng lặp
  **không nghỉ** cho tới khi chạm 1800 s. Mỗi vòng chạy **một** lệnh cargo với 4 test target để
  giảm overhead:
  `cargo test -p vis-cli --test vs3_default --test vs3_tls --test net --test vs3_worker_eval_local`.
- Xen kẽ theo lịch cố định: worker debug (vòng 3), h2 debug #1 (vòng 5), worker release (vòng 6),
  h2 release (vòng 8), h2 debug #2..#5 (vòng 10/20/30/40).
- Dừng ngay nếu bất kỳ lệnh nào rc ≠ 0 (không xảy ra).

## 2. Kết quả từng vòng

- **137/137 vòng rc 0**; mỗi vòng đúng `9 + 22 + 22 + 8 = 61` test pass.
- Thời gian/vòng: vòng 1 **14 s** (khởi động), các vòng còn lại **11–12 s**; không vòng nào tăng
  dần hay treo.
- Tổng test đã chạy: **137 × 61 = 8 357** (4 suite) + h2 **6 × 19 = 114** + worker **2 × 14 = 28**
  ⇒ **8 499 test**, **0 fail**.
- Không có `FAILED`/`panicked` trong bất kỳ log vòng/extra nào.

## 3. Xen kẽ

| Lần | Lệnh | Kết quả | Thời gian |
|---|---|---|---:|
| worker debug (vòng 3) | `cargo test -p vis-cli --test vs3_worker` | **14/14** | 27 s |
| h2 debug #1 (vòng 5) | `--test vs3_http2` | **19/19** | 16 s |
| worker release (vòng 6) | `--release … --test vs3_worker` | **14/14** | 31 s |
| h2 release (vòng 8) | `--release … --test vs3_http2` | **19/19** | 16 s |
| h2 debug #2–#5 (vòng 10/20/30/40) | `--test vs3_http2` | **19/19 mỗi lần** | 16 s/lần |

- `vs3_default` mỗi vòng gồm `async_net_khong_ro_ri_tay_sau_200_vong` (fd/handle nội bộ = 0) và
  `host_tls_tren_socket_reactor_tra_loi_ro` ⇒ bằng chứng fd-leak/invariant lặp 137 lần.
- Không đo OS handle/thread count xuyên tiến trình: mỗi ca test là process riêng, đóng cùng
  process; `AsyncNet::dem_tay_dang_mo()` là nguồn đo trực tiếp (200 vòng/ca, luôn 0 sau close).

## 4. Cuối soak

- `bin/vis-host-suite`: **54/54** (`host_posix_real` bỏ qua — Windows `test-child`).
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (#156); 0 FAIL.
- HEAD cuối `83b40020`, dirty 0.

## 5. Log / bằng chứng

- Log đầy đủ 137 vòng + extras + host/targets:
  `agent/JOURNAL/vs3-w4-winsoak-83b4002.log` (302 dòng),
  sha256 `0aacc82d232b3c69fc4cdb571316fde407588a29a4021a290bb5a64f788ababb`.
- Bản gốc `psvn`: `/c/Users/ps/vislogs/w4e-soak.log` (cùng sha).
- Worktree `psvn` đã xoá sau đo; repo chính `psvn` @ `73af17bd`, dirty 0; `C:` còn **181G**.
- Local `/` còn **13G** (>5G).

## 6. Kết luận

- Windows async net/TLS/HTTP-1.1 + evaluator P1 ổn định qua **137 vòng / 30 phút**, 0 flake;
  sync h2 và worker không hồi quy.
- `10053` đã hết; không có flake mới.
- Async-h2 vẫn **Unix-only/default-off** (không nằm trong soak này).
- **Không blocker.**
