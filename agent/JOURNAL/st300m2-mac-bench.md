<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300m2 — đo độc quyền `mps` arm64: đóng `#147` & `#152`

> **Người làm**: VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st300m2-mac-bench` từ **`c3b7290`**.
> **Không merge/push/tag**. Máy `mps` (macOS 27.0.1 / Darwin 27.0.0 arm64, 12 core) độc quyền;
> `bin/vis-lock --timeout 3600`, `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`,
> `TMPDIR=/Users/mps/tmp`; `ps` (Linux) cùng HEAD `c3b7290`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `#147` | **ĐÓNG** — soak 10 vòng: mps **379,7 s** / ps **517,1 s = 0,73×**; gate **103,6/112,0 s = 0,93×**; test **82,3/71,0 s = 1,16×**; compile **22,5/57,0 s = 0,39×**. Mọi phép mac ≥10 s đều **<3×** |
| `#152` | **ĐÓNG** — A/B 24 cặp, load **1,93** (chờ 60 s; không soak/gate): interp **B/A 1,0385**, VM **B/A 1,0626**; cả hai median **<1,10** |
| Soak | **10/10 vòng OK** cả hai nền; diff 100k `lech=0` |
| Gate Linux | **3672/4** (đo lại ở ST-300m1, cùng HEAD) |
| Blocker | **không** |

## 1. Phương pháp

- `#147`: `cargo nextest run --workspace --no-run` (**biên dịch**) → `cargo nextest run
  --workspace --no-fail-fast` (**chạy test**) → `./vis-dev gate --fast` → `bin/vis-soak`
  (**10 vòng**, log ngoài repo). Nextest là đường cổng hiện hành (ST-296a), mỗi test một
  tiến trình; libtest song song làm 2 ca timing flake (xem §4).
- `#152`: build release A=`7c256df` và B=`c3b7290`; chạy `vis run --no-cache [--vm]
  test/bench/opt-bench.vis`, kiểm output **38995450**; 24 cặp xen kẽ **AB/BA** cho từng
  engine; thời gian đồng hồ tường (A không có `--time`); trung vị + min..max.
- Log ngoài repo; sha256 ở §5.

## 2. `#147` — bảng thành phần (cùng HEAD `c3b7290`)

| Phép đo | mps (s) | ps (s) | mps/ps |
|---|---:|---:|---:|
| Biên dịch test (`--no-run`) | **22,481** | **57,048** | **0,39×** |
| Chạy test workspace (nextest) | **82,280** | **70,955** | **1,16×** |
| `gate --fast` | **103,569** | **111,952** | **0,93×** |
| **Soak 10 vòng** | **379,700** | **517,063** | **0,73×** |
| gate-1 / gate-2 / gate-3 | 95 / 96 / 97 | 93 / 89 / 89 | 1,02 / 1,08 / 1,09× |
| diff-1 / diff-2 / diff-3 | 26 / 23 / 24 | 77 / 74 / 72 | 0,34 / 0,31 / 0,33× |
| host-1 / host-2 | 3 / 3 | 2 / 1 | <10 s: chỉ ghi số |
| cache-1 / cache-2 | 6 / 7 | 10 / 10 | <10 s: chỉ ghi số |

- Load: mps **1,80** lúc bắt đầu (sau đó 7,67 do chính soak); ps **1,00** lúc bắt đầu
  (3,54 sau soak). Có `vis-lock` (xem log).
- Ngưỡng **3× chỉ áp khi mac ≥10 s** (VD-197(4)); các phép ≥10 s: compile, test, gate, soak
  và từng gate/diff — **tất cả <3×**. Kể cả so với ps lịch sử **261 s** (ST-176), soak mps
  380 s = **1,46×** <3×.
- `vis-diff` mỗi vòng: `tong=100000 chay=93770 lech=0`; soak log ghi `10/10 vòng OK`.
- Kết luận: **#147 đóng** — macOS arm64 không còn chậm >3×; số mac mới là 380 s soak,
  gate 104 s.

## 3. `#152` — A/B `opt-bench` trên mps

Build: A **8,901 s**, B **28,619 s** (release). Đo riêng sau khi soak đã dừng:
load **2,34 → 1,93** (chờ **60 s**, `heavy=0`; kiểm `pgrep` không có soak/gate). 24 cặp/engine,
xen kẽ AB/BA, đều in `38995450`.

| Engine | A median (ms) | B median (ms) | B/A | cặp median | min..max |
|---|---:|---:|---:|---:|---|
| interp | 113,597 | 117,965 | **1,0385** | 1,0419 | 1,0067..1,7178 |
| VM | 119,416 | 126,892 | **1,0626** | 1,0638 | 1,0301..1,1249 |

- Cả hai **median B/A <1,10** ⇒ enum-hoá `Option`/`Result` **không** chậm >10 % trên
  macOS arm64; khớp ps (1,00/1,02). VM có 1 cặp 1,125; interp có 1 outlier 1,718 —
  không hệ thống, median/median-cặp vẫn <1,10.
- Kết luận: **#152 đóng**.

## 4. Fail đã gặp & phân loại (không retry mù)

1. **Libtest song song** (`cargo test --workspace`): ps đỏ `test_timeout::treo_bi_giet_trong_3s`
   vì `con_cua_minh()` thấy tiến trình của test khác; mps đỏ
   `lat256_net::t3_peer_reads_slowly_both_engines` timeout dưới tải. Đây là **cô lập test**,
   không phải lỗi sản phẩm (gate dùng nextest, mỗi test một tiến trình).
2. **Nextest CLI**: `--no-fail-fast` không dùng được với `--no-run` ⇒ bỏ cờ đó ở bước biên dịch.
3. **A/B lần 1** nằm trong job #147, bắt đầu khi load còn **5,78** ⇒ **không dùng**; chạy lại
   A/B riêng sau khi chờ load <2 (mục 3).
4. **Ghi summary A/B**: lỗi Python (biến `a` bị đổi thành list) sau khi đã đo xong; **CSV 24
   dòng/engine còn nguyên**, median được tính lại từ CSV — **không đo lại**.

## 5. Log + sha256 (ngoài repo)

| Tệp | sha256 |
|---|---|
| `vislogs/st300m2-mps-20261123T0601Z/compile.log` | `29278da5c2487fa8c3547c7112efb11a6880a5b7ae735ca032e1c3c9a9446797` |
| `…/test.log` | `d6e812515df1c587882c0b7e210014278082fcd92c06bf6deb41ceeb70cc5601` |
| `…/gate.log` | `6f34d9c55135dbcf879f24ef1620dbb4ffe5bb00837162bb72b20fc5ea4b4733` |
| `…/soak-20261123T060339Z.log` | `740b5dbfcc4a9bbea7b5cc4b7bad6dbebdd9cd859ff8e37521627d4e6e7fa79c` |
| `vislogs/st300m2-ps-20261123T0601Z/soak-20261123T060413Z.log` | `9308a43110d8f4f5123e489666831d3af2c314d173a0302d3c44e829d2718469` |
| `…/ps/gate.log` | `bb63aeccacf5d17aa32de3e82a2b7907a4c7fd7af1607fa01b4c00fea0b09ad9` |
| `vislogs/st300m2-ab-20261123T0731Z/ab-INTERP.csv` | `1c73618306fa3c948ddfb66acddd17591e8dc2425d3b1a10f44919edf9937068` |
| `…/ab-VM.csv` | `63f54cdc91182c42e14c262bd06a135d83126e82d8f00b1e00c3f739284d29f5` |
| `…/load-ab.log` | `8cff944f32a3ee9ceb4e55e89d277177110cceb7f57311c7f212c63d109c3b05` |
| `vislogs/st300m2-mps-summary-final.txt` | `d9af20a88cc0bbbda1d5faa62aa194fcf522149fe647543ffa0ce4b7dec4cb78` |
| `vislogs/st300m2-ps-20261123T0601Z/summary.txt` | `d3115ff6c7cd18b131bb0d2ee319b20938baa18e742a83a9565ec0b25c0461ea` |

`SO-NO.md`: `#147`, `#152` → **đóng ở ST-300m2**; §8 **194 = 83 mở + 92 đóng +
16 VS3 + 2 không làm + 1 điều hướng**; R5 PASS.

## 6. Kết luận

- `#147` đóng: mac arm64 **không** còn >3×; soak 380 s, gate 104 s, test 82 s.
- `#152` đóng: A/B 24 cặp, interp **+3,85 %**, VM **+6,26 %** — dưới ngưỡng 10 %.
- **Không merge/push/tag**; không chạy `ST-300m2` lần nữa, không benchmark/soak ngoài phạm vi.
