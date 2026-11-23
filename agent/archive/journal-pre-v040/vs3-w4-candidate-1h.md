<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — soak 1 giờ Windows trên candidate `e672df7` (v0.3.5)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `e672df7` (`release: ghi chu v0.3.5`; cancel fix `1edec5d` + net timeout bound
> `20b0987`). `psvn` worktree detached đúng commit, dirty 0.
> **Kết quả**: **386 vòng / 3602 s (1 giờ 00 phút 02 giây)**, **0 fail, 0 flake**; host 54/54;
> targets 18/18 + 17/1. Test-only; không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tổng | **386 vòng / 3602 s** |
| Mỗi vòng | P2 **14/14** (gồm `cancel_pre_run_tra_loi_ro`) + P1 **8/8** + P3 **10/10** (36 test) |
| Mỗi 5 vòng | `default` 22/22 + `TLS` 22/22 + `net` 9/9 (gồm `tcp_het_gio_doc`) — **77 lần** |
| Xen kẽ | h2 sync **debug 3×19/19 + release 2×19/19**; worker **debug 4×14/14 + release 2×14/14** |
| Fail / flake | **0 / 0** |
| Tổng test | **18 156**, 0 fail |
| Process leak | `vis_proc=0` sau **386/386** vòng |
| Bộ nhớ | FreePhysicalMemory **7 732 956 KB → 6 016 276 KB** (giảm ~1,7 GB; REPORT 10–50 phút: 7,53→7,51→7,99→7,89→6,63 GB) |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** (#156) |
| Blocker | Không có (theo dõi bộ nhớ khi soak dài) |

## 1. Phương pháp

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc 1.100.0 (MSVC).
- Warm build debug (P1/P2/P3, default/tls/net, h2, worker) + release (h2, worker).
- Vòng lặp tới 3600 s: P2→P1→P3 mỗi vòng; default/tls/net mỗi 5 vòng; worker debug 30/130/230/330,
  worker release 80/250; h2 debug 50/150/250/350, h2 release 100/300.
- REPORT mỗi 10 phút (vòng, free memory, vis_proc, fail); dừng ngay nếu fail.

## 2. Kết quả

- 386/386 vòng rc 0/0/0; thời gian/vòng 5–9 s (không tăng dần rõ).
- 77 lần default/tls/net: **22/22 + 22/22 + 9/9** mỗi lần; `tcp_het_gio_doc` không fail sau khi
  nới biên 3 s (test-only, commit `20b0987`).
- P2 14/14 mỗi vòng; `cancel_pre_run_tra_loi_ro` ok **386 lần**.
- Không `FAILED`/`panicked`; `vis_proc=0` 386/386.

## 3. Bộ nhớ / process

- Free memory giảm dần về cuối (7,73 GB → 6,02 GB, ~1,7 GB); các REPORT: 7,53 / 7,51 / 7,99 /
  7,89 / 6,63 GB. Không thấy process `vis` sống sót; P2 `muoi_vong_threads_fd_rss_khong_ro` và P3
  `vm_khong_tao_them_thread_sau_khi_join` xanh mỗi vòng.
- Đây có thể là cache/working set của Windows khi chạy hàng trăm test binary; **cần theo dõi** ở
  soak 4h tiếp theo, chưa đủ kết luận rò.

## 4. Cuối soak

- host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**; 0 FAIL.
- HEAD cuối `e672df7c`, dirty 0.

## 5. Log

- `agent/JOURNAL/vs3-w4-candidate-1h.log` (637 dòng),
  sha256 `f08bb9a6fe044fff2436ba6b536dfc51fcaf584934c1fa18a6ba08cfc30ce2d3`.
- Bản gốc `psvn:/c/Users/ps/vislogs/w4r-soak.log` (cùng sha).

## 6. Kết luận

- Candidate `e672df7` ổn định 1 giờ trên Windows: 386 vòng, 0 fail/flake, host/targets xanh.
- `tcp_het_gio_doc` với biên 3 s không còn flake trong soak này.
- Theo dõi free-memory trend khi soak 4h lại trên `v0.3.5`; không blocker cho tag.
