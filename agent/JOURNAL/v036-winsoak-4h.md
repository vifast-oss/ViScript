<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Soak Windows 4h — tag `v0.3.6` (`8fa2c09`)

> **Người đo**: subagent VS3-W4, 2026-11-22 · **test-only**, time-box 4,5 h.
> **Tag**: `v0.3.6` (annotated `e27a5ed`) → commit **`8fa2c097bcdf2eb1eace583ac7fae016a90647b6`**;
> `git describe --tags --exact-match` = `v0.3.6`; cây sạch lúc bắt đầu mỗi chặng.
> **Kết quả**: **≈4 h wall (~3h56m vòng đo), 1361 vòng, 0 fail sản phẩm** — chia **2 chặng** vì `psvn`
> (repo dùng chung) bị tác nhân khác đổi HEAD giữa soak. Cancel: **272 lượt suite, 0 fail**;
> host 54/54, targets 18/18 + 17/1 ở chặng cách ly. Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Chặng | Môi trường | Thời gian | Vòng | Fail sản phẩm | Flake |
|---|---|---|---:|---:|---:|
| **1** | `psvn` `/c/Users/ps/viscript-moi` @ tag | 08:58:09Z → ~10:52Z (**1h51m** hợp lệ) | **660** (R1–R660) | **0** | **0** |
| **2** | worktree cách ly `/c/Users/ps/viscript-w4soak-v036` @ tag | 11:00:56Z → 13:07:34Z (**2h05m**) | **701** | **0** | **1** (env R1 p3 `rc=127`, rerun pass) |
| **Tổng** | 2 môi trường, cùng tag | **≈3h56m vòng đo (~4h wall)** | **1361** | **0** | **1 env** |

- **Cancel h1**: mỗi 10 vòng chạy default 5/5 + `--test-threads=1` 5/5 ⇒ chặng 1: **66 vòng ×2 =
  132 lượt suite**; chặng 2: **70 vòng ×2 = 140 lượt** ⇒ **272 lượt, tất cả 5/5**, 0 fail.
- **Batch mỗi 5 vòng** (default 22 + TLS 22 + net 9 + pkg 25): chặng 1 **132 vòng**, chặng 2
  **140 vòng** — tất cả xanh.
- **P2 14/14 + P1 8/8 + P3 10/10** mỗi vòng: **1361/1361 vòng** xanh (trừ R1 p3 chặng 2 là lỗi môi
  trường `rc=127` trước khi chạy test, rerun ngay sau đó 10/0).
- Free RAM: chặng 1 **7184–9611 MB**; chặng 2 **6706–8839 MB** — không có xu hướng rò rỉ.
- Cuối chặng 2: host **54/54**; targets core **18/18**, sqlite **17 + 1 EXPECTED_RED**;
  `cargo check --features async-h2` **rc 101 EXPECTED** (Unix-only).

## 1. Chặng 1 — `psvn` dùng chung (bị nhiễu giữa chừng)

- Checkout `v0.3.6` (`8fa2c09`), dirty 0, free 9499 MB; soak bắt đầu **08:58:09Z**.
- **R1–R660**: mọi vòng P2/P1/P3 xanh; 132 batch xanh; 66 vòng cancel (132 lượt) đều `5/0`;
  free RAM 7184–9611 MB; **0 fail, 0 flake**. Tổng thời lượng vòng ≈ **6683 s (1h51m)**.
- **Nhiễu**: khoảng R661–R670, một tác nhân khác chuyển `psvn` sang HEAD **`a48d5b87…`** và để
  cây bẩn **4 tệp** (`Cargo.toml`, `async_net_conn.rs`, `async_net_nguon.rs`, `net/h2_async.rs`).
  - Từ R670, `vs3_http_cancel` có **8 ca** (thay vì 5) ⇒ các dòng `FAIL ... exp=5 got=8/0` là do
    **mã đã đổi**, không phải test fail; rerun vẫn `8/0` (không tính flake sản phẩm).
  - Kiểm dirty định kỳ mỗi 30 vòng phát hiện tại **R690** ⇒ soak **tự dừng** để bảo toàn bằng chứng
    (`rounds=690`, `dirty_stop=1`, `dirty_end=4`, `HEAD_END=a48d5b87…`).
  - Host/targets cuối chặng 1 chạy trên HEAD bẩn ⇒ **không dùng** làm bằng chứng tag.
- Kết luận chặng 1: **660 vòng hợp lệ trên tag, 0 fail**.

## 2. Chặng 2 — worktree cách ly (bù 2h05m)

- Tạo `git worktree add --detach /c/Users/ps/viscript-w4soak-v036 v0.3.6` (HEAD `8fa2c09`,
  `describe --exact-match = v0.3.6`, dirty 0), prebuild 42 s.
- Soak **11:00:56Z → 13:07:34Z** = **7505 s (2h05m)**, **701 vòng** (R1–R701):
  - P2/P1/P3 xanh 701/701 vòng (R1 p3 lần đầu `rc=127` — lỗi môi trường trước khi chạy test,
    rerun ngay `10/0`; tính **1 env flake**, không phải fail sản phẩm).
  - Batch 140 vòng (default/TLS/net/pkg) — tất cả xanh.
  - Cancel 70 vòng × (default + serial) = **140 lượt suite 5/5**, 0 fail.
  - Free RAM 6706–8839 MB (start 8176, end 8298); dirty_end 0; HEAD_END `8fa2c09`.
- Cuối chặng: host **54/54** (`1..54`, 120 lần chạy, VM 54/54); targets **18/18 + 17/1**;
  async-h2 check **rc 101 EXPECTED**.

## 3. Tổng hợp

- **1361 vòng**, **14 188 s (~3h56m) thời gian vòng đo** trên đúng tag (`8fa2c09`),
  xấp xỉ **4h wall**; **0 fail sản phẩm**.
- **Cancel**: **272 lượt suite** (default + serial), tất cả **5/5** — tiếp tục xác nhận fix
  `0645700` ổn định qua thời gian dài.
- **1 flake môi trường** (R1 p3 chặng 2, `rc=127` trước khi chạy test; rerun pass) — không phải
  flake sản phẩm.
- Bộ nhớ phẳng trong cả hai chặng, không dấu hiệu rò rỉ theo thời gian.
- **Blocker**: `psvn` là môi trường dùng chung, bị tác nhân khác chiếm giữa soak (HEAD/dirty đổi).
  Đã xử lý bằng worktree cách ly. Đề xuất: **luôn dùng worktree/clone riêng cho soak dài** trên
  `psvn`; không chạy soak 4h trực tiếp trong repo chung.

## 4. Bằng chứng

- Raw summaries: `agent/JOURNAL/v036-winsoak-4h.log` (chặng 1 + chặng 2).
- `psvn` logs: `/c/Users/ps/vislogs/soak4h-summary.log`,
  `/c/Users/ps/vislogs/soak4b-summary.log`, `soak4b-host.log`, `soak4b-targets.log`,
  `soak4b-async-h2.log`; worktree cách ly `C:` còn **147 G**.
- Local `/`: còn **15 G** (>5 G); không tag/push/merge; không GitHub Actions.
