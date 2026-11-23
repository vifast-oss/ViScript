<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300m3 — macOS arm64 Tier 1 (final6): 3× full + soak 4 h + WASI sem

> **Người chạy**: subagent VS3-W4, 2026-11-23/24 (JST). **Mốc**: `7930e9066d5ce01df038b4c0cc97062b2778ddef`
> (`main`, version **0.5.0**; Linux gate **3.754/4**; `ST-300m3` final6). **Máy**: `mps` — macOS
> **27.0.1** (Darwin 27.0.0) **arm64**, 12 core / 32 GiB, Rust **1.100.0**, Node **26.12.1**; độc quyền.
> **Không merge/push/tag.** Clone đo: `/Users/mps/dvchd/ViScript-v050` (dirty **0** trước/sau).
> `VIS_GATE_MACOS_REPO` + `VIS_REMOTE_MPS_REPO` trỏ clone này; `CARGO_BUILD_JOBS=3
> CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/Users/mps/tmp`, mọi job nặng dưới `bin/vis-lock`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| 3× `bin/vis-gate-macos --full` | **3/3 PASS** — wall **220 s / 216 s / 210 s** (tổng **646 s**) |
| Mỗi lần full | nextest **3.745 passed / 3 skipped / 0 failed** (1 slow); doc-test **5 passed / 0 failed / 1 ignored**; `vis-host-wasm` **29/29**; WASI suite **28/28**; `./vis-dev check` + `gate --fast` PASS |
| Soak 4 h | **1.878 vòng / 14.405 s**, **0 lỗi sản phẩm**; free RAM **15.163–15.854 MB**; RSS REPL **6.272–7.008 kB** (slope **-0,009 kB/vòng**); fd **6 hằng** (first/last **6/6**); dirty cuối **0** |
| `bin/vis-wasm-sem --require-target` | **388/390 = 99,5%** — interp **388/390**, VM **390/390**; oracle native 475/475 |
| Tier 1 | **macOS arm64 (`aarch64-apple-darwin`) — ĐỦ điều kiện công bố**, caveat Landlock/Intel bên dưới |
| Blocker | **không** |

## 1. Đồng bộ clone đo

- `/Users/mps/dvchd/ViScript-v050` đang detached tại `8906c70` (dirty 0); `git fetch origin main`
  → `7930e9066d5ce01df038b4c0cc97062b2778ddef`, `git checkout --detach 7930e90`;
  `git status --porcelain` **0** dòng trước và sau toàn bộ đợt đo.
- `VIS_GATE_MACOS_REPO=/Users/mps/dvchd/ViScript-v050`,
  `VIS_REMOTE_MPS_REPO=/Users/mps/dvchd/ViScript-v050`; log `vis-remote` ghi
  `# cau_hinh: host=mps dirty_policy=check repo=/Users/mps/dvchd/ViScript-v050`.
- `target/gate-macos` tái dùng từ các lần trước; đĩa trống **33 GiB** trong suốt đợt đo.
- **Bối cảnh final5**: tại `8906c70`, `bin/vis-gate-macos --full` đỏ tất định ở libtest
  `lat256_net::t3_peer_reads_slowly_both_engines` (xem `st300m3-final5-macos-blocker.md`).
  `7930e90` đổi bước workspace của cổng `--full` sang **nextest + doc-test** (nextest chạy mỗi ca
  một tiến trình; libtest-only `t3` là khác biệt harness, không phải lỗi sản phẩm). Lần đo này
  xác nhận cổng xanh 3/3 trên HEAD mới.

## 2. 3× `bin/vis-gate-macos --full`

| Lần | UTC (start → end) | Wall | nextest | doc-test | host-wasm | WASI | dirty |
|---|---|---:|---|---|---|---|---:|
| 1 | 15:00:05Z → 15:03:43Z | **220 s** | 3.745 / 3 skip / 0 fail | 5 / 0 / 1 | 29/29 | 28/28 | 0 |
| 2 | 15:03:44Z → 15:07:20Z | **216 s** | 3.745 / 3 skip / 0 fail | 5 / 0 / 1 | 29/29 | 28/28 | 0 |
| 3 | 15:07:20Z → 15:10:50Z | **210 s** | 3.745 / 3 skip / 0 fail | 5 / 0 / 1 | 29/29 | 28/28 | 0 |

- Cả 3 lần **cùng HEAD `7930e90`**, `vis-remote-exit=0`; `./vis-dev check` PASS
  (doc-examples **31 khối** = 26 xanh + 5 ngoại lệ đỏ đúng; readme-examples **20/20**);
  `./vis-dev gate --fast` PASS (L0 nextest + doc-test); không có `FAIL [` trong nextest.
- Log ngoài repo (bản sao cục bộ trên `ps` + log gốc trên `mps`):

| Lần | Log cục bộ (`/home/ps/vislogs/`) | sha256 cục bộ | Log gốc trên `mps` | sha256 gốc |
|---|---|---|---|---|
| 1 | `st300m3-final6-gate-full-1.log` | `0b755509b56d3623d2fc57d265687b8604ee1636d1cfa7ffa9759a03ff92fcf0` | `/Users/mps/vislogs/vis-remote-20261123T150004Z.log` | `0d9eb91b3b37fa3c94d86af53177724ee685bd885bbac742909847b9f5485d69` |
| 2 | `st300m3-final6-gate-full-2.log` | `0addd02da8d199c9a0b61cb8745d20e38c178aa697957365503f9ab9e7843cfa` | `/Users/mps/vislogs/vis-remote-20261123T150344Z.log` | `3e6565b25e444ac9a404741853be38b0328b4db1566598a8ef38ca8e35315675` |
| 3 | `st300m3-final6-gate-full-3.log` | `804b1460aa048ced74dfc2c3b984c0495194aa612f9fa747b1f05a7cdd9a2e1a` | `/Users/mps/vislogs/vis-remote-20261123T150720Z.log` | `32ea84d8e9922894b99cfe729732db628980ebb08a19d49d786ba1de749c9bee` |

## 3. Soak 4 h

- Script ngoài repo: `/Users/mps/vislogs/st300m3-final6-soak-4h.sh` (kế thừa final4, chỉ đổi
  `EXPECT=7930e90…` và tên log); dưới
  `bin/vis-lock --timeout 600 --log /Users/mps/vislogs/vis-lock-st300m3-final6-soak.log`.
  Start **2026-11-23T15:15:30Z**, kết thúc **2026-11-23T19:15:35Z**, wall **14.405 s**; cùng HEAD `7930e90`.
- Mỗi vòng: P2 `vs3_worker_eval_parallel` **14/14** + P1 `vs3_worker_eval_local` **8/8** +
  P3 `vs3_worker_eval_vm` **10/10** + `async_it vs3_select` **25/25** +
  `cargo test -p vis-plugin-abi` (**17 ca** = 3+7+7, gồm ca chống đua driver WASI
  `tests_driver_rieng`) + 6 tệp `test/sem` so **byte** interp ↔ VM ↔ `.out`.
  (`plugin-wasi=0` trong dòng log là số đếm của **target doc-test cuối** — 0 ca; bộ plugin-abi
  phía trên vẫn chạy đủ 17 ca/lượt và 0 lỗi.)
- Mỗi 10 vòng: `bin/vis-host-suite` (**54/54**), `cargo test -p vis-cli --test http2_it`,
  `bin/vis-wasm-suite --build --runtime node` (**28/28**), kiểm `git status --porcelain` = 0.
- Tổng lượt suite: **P2/P1/P3/select 1.878 mỗi loại**; **host 187/187 lượt** (54/54 mỗi lượt),
  **http2 187/187 lượt** (33/33 mỗi lượt), **WASI suite 187/187 lượt** (28/28 mỗi lượt);
  **0 dòng `FAIL-`** trong log.
- Đo RSS/fd: một tiến trình `vis repl` **thường trú** (stdin FIFO) được lấy mẫu mỗi vòng;
  free RAM lấy từ `vm_stat` (`Pages free + inactive`), trang 16 KiB.
- **Kết quả**: **1.878 vòng**, wall **14.405 s**, **0 fail sản phẩm** (không có tệp
  `st300m3-final6-soak-4h.FAIL`).
  - Free RAM: **15.163–15.854 MB**, first/last **15.687/15.634**,
    slope **-0,0061 MB/vòng** (dao động theo tải, không xu hướng rò rỉ).
  - RSS `vis repl`: **6.272–7.008 kB**, first/last **6.272/6.976**;
    sau 20 vòng khởi động: **6.960–7.008 kB**, slope **-0,009 kB/vòng**
    — **phẳng**.
  - fd `vis repl`: **6–6**; **1.878/1.878** mẫu bằng **6**
    — **phẳng**.
  - Round time: median **6 s**, min/max **5/28 s**.
  - Dirty cuối **0**; HEAD cuối `7930e90`.
- Log: `/Users/mps/vislogs/st300m3-final6-soak-4h.log` (2.919.431 B) sha256
  `94b49f45ac2b2d9d9d2c4f46a7530e94b4c3a9045cacaf05894ec60346e96851`; TSV: `/Users/mps/vislogs/st300m3-final6-soak-4h.tsv` (89.289 B) sha256
  `ff6461064e46ff914ddf16b5b2c47f93a542c1d746afbbe45014e614cc0e3e50`; lock: `/Users/mps/vislogs/vis-lock-st300m3-final6-soak.log` sha256
  `fea56b47f7f83562c7ebb4069b7345f9da3633e17413793db4c4d66df7e45c64`.
- Minh bạch cơ chế dừng: script tự dừng sau `DUR=14400` giây, ghi `FAIL` và thoát khác 0 nếu
  bất kỳ suite nào đỏ / dirty / REPL chết; không retry, không xoá `FAIL`.

## 4. `vis-wasm-sem` trên cùng HEAD

```bash
CARGO_TARGET_DIR=/Users/mps/dvchd/ViScript-v050/target/gate-macos \
  ./bin/vis-wasm-sem --oracle /Users/mps/vislogs/sem-native-baseline.tsv --require-target
```

- **388/390 = 99,5%** — interp **388/390**, VM **390/390**; 2 ca trượt là **stack-overflow interp**
  đã biết từ `ST-300` (`ham/de-quy-1000.vis`, `ham/de-quy-vo-han.vis`; VM pass), không phải hồi quy.
- Oracle: 475/475 native, sha256 `482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d`.
- Runtime WASI: **node 26.12.1** (`node:wasi`), **60,4 s** (vis-lock; harness 59–60 s);
  `--require-target` exit **0** (ngưỡng 85%).
- `CARGO_TARGET_DIR` trỏ target của cổng để dùng đúng wasm đã build ở 3× full cùng HEAD.
- Log gốc trên `mps`: `/Users/mps/vislogs/vis-remote-20261123T191551Z.log` (60.386 B) sha256
  `bec3e66840e4107fad89ba2998b27efa4414db54af639103b47d8fc4226118c6`; bản sao cục bộ
  `/home/ps/vislogs/st300m3-final6-wasm-sem.log` (63.244 B) sha256
  `74a10f470edc5dacf2db44bb5e5ded39741f1090913a002a9a464cdcd5be4432`; lock sem:
  `78062284fdd7cacd5c211a326b7b1512425f124d6e2209036e752fbacb4b1a3b`.
- Lần chạy sớm trước soak (cùng HEAD, cùng target) cũng **388/390**; log gốc
  `/Users/mps/vislogs/vis-remote-20261123T151412Z.log` (60.305 B) sha256
  `fa8af4dd2c59825986bbd7153dab274d891c08efec4b1858b75300a5f7906268`, bản sao cục bộ
  `st300m3-final6-wasm-sem-early.log` sha256
  `efff961619d241a38996487ae079ece916542c46ac38292d378c480536f78da1` — dùng để chặn sớm,
  không thay lần chạy chính thức sau soak.

## 5. Kết luận / tuyên bố

- **Đủ điều kiện công bố macOS arm64 Tier 1** (`aarch64-apple-darwin`): 3× full xanh liên tiếp,
  soak 4 h **1.878 vòng / 0 lỗi sản phẩm**, RSS/fd phẳng, `vis-wasm-sem` 388/390, dirty 0.
- **Caveat bắt buộc**:
  1. **Không có Landlock** trên Darwin (kernel không hỗ trợ); bảo vệ là **cưỡng chế runtime**
     `FsGuard`/`NetGuard` + `VS0604`, không phải sandbox kernel.
  2. **Chứng nhận chỉ cho `aarch64-apple-darwin`** trên macOS 27.0.1 arm64; **chưa đo Intel
     x86_64** (`x86_64-apple-darwin` chỉ `cargo check` nếu có).
  3. Hardlink vẫn là **giới hạn chính sách**; `FsGuard` macOS dùng `openat`/`(dev, ino)` như
     Unix (ST-174/ST-295/ST-300m1).
  4. Tier 1 là **mức hỗ trợ sản phẩm** (compiler/interp/VM/module/LSP/host/plugin/WASI runner),
     không suy rộng thành “sandbox kernel”.
- Docs cập nhật: `agent/HOST-BAO-DAM.md`, `agent/ROADMAP.md`, `agent/ACTIVE.md`; commit **chỉ docs +
  journal** trên nhánh `plan/w4-st300m3-final6` từ `7930e90` — **không merge/push/tag**.

## 6. Lệnh tái lập

```bash
# trên ps
export VIS_GATE_MACOS_REPO=/Users/mps/dvchd/ViScript-v050
export VIS_REMOTE_MPS_REPO=/Users/mps/dvchd/ViScript-v050
for i in 1 2 3; do VIS_GATE_MACOS_LOG=$HOME/vislogs/st300m3-final6-gate-full-$i.log \
  bin/vis-gate-macos --full || break; done

# trên mps (dưới vis-lock)
bin/vis-lock --timeout 600 --log ~/vislogs/vis-lock-st300m3-final6-soak.log -- \
  bash /Users/mps/vislogs/st300m3-final6-soak-4h.sh

# WASI sem
CARGO_TARGET_DIR=/Users/mps/dvchd/ViScript-v050/target/gate-macos \
  ./bin/vis-wasm-sem --oracle /Users/mps/vislogs/sem-native-baseline.tsv --require-target
```

## 7. Blocker

- **Không blocker sản phẩm.** Blocker final5 (`lat256_net::t3` dưới libtest) đã được xử lý ở
  `7930e90` bằng cách đổi cổng `--full` sang nextest + doc-test; 3× full lần này xanh.
  Không tag/push/merge; không GitHub Actions.
