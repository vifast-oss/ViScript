<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Linux gate + soak `v0.5.0` — `e9c62cb` (10/10, 650 s)

> **Bản repo là tóm tắt (R7)**; log đầy đủ nằm ngoài repo:
> `/home/ps/vislogs/final-linux-e9c62cb.log` (1.149.650 byte, sha256
> `4aed4c203a11a3184e07f1cdc8dc9810ea38de29392ee8b2185a1f2f50b7db42`);
> log điều khiển `/home/ps/vislogs/final-linux-e9c62cb.driver.log` (1.797 byte, sha256
> `8dd6538caa793b821ce7eea4619e1d073819293001521869e8638d160f228e55`).
> **Chưa có** `/home/ps/vislogs/final-linux-e9c62cb.md` — tệp này là bản tóm tắt thay thế,
> ghi rõ số đọc trực tiếp từ log.

- **Mốc**: `e9c62cb11eae2725a024a833ec9aa3de257cb248` — `[ST-00] VD-285` (version `0.5.0`;
  gồm security `5902887`, `SPEC.md` `51e937d`/`f9e7503`).
- **Máy**: `ps` local (Linux x86_64, 12 CPU); **không** ssh `mps`/`psvn`.
- **Môi trường**: `TMPDIR=/home/ps/tmp` (soak tự tạo thư mục tạm ngoài repo), `CARGO_BUILD_JOBS=3`.
- **Lệnh**: `./bin/vis-soak --log /home/ps/vislogs/final-linux-e9c62cb.log`
- **Kết quả**: **10/10 vòng OK**, `SOAK_EXIT=0`, **650 s** (`13:24:46Z` → `13:35:36Z`),
  không flake, không retry.
- **Trạng thái R7**: số liệu đầy đủ trong log; **thiếu `.md` gốc** — A0 cần chốt trước khi
  tuyên bố điều kiện 3 của tag `v0.5.0`.

## 1. Tổng hợp vòng

| Vòng | Thời gian | Kết quả | Bằng chứng chính |
|---|---:|---|---|
| gate-1 | 196 s | OK | nextest **3754 passed / 4 skipped** |
| gate-2 | 92 s | OK | nextest **3754 passed / 4 skipped** |
| diff-1 | 84 s | OK | `tong=100000 … lech=0` |
| diff-2 | 79 s | OK | `tong=100000 … lech=0` |
| gate-3 | 92 s | OK | nextest **3754 passed / 4 skipped** |
| diff-3 | 77 s | OK | `tong=100000 … lech=0` |
| host-1 | 7 s | OK | **54/54** + `host_posix_real` PASS |
| host-2 | 1 s | OK | **54/54** + `host_posix_real` PASS |
| cache-1 | 11 s | OK | **5/5** |
| cache-2 | 11 s | OK | **5/5** |

Tổng theo script: `650s | 10/10 vòng OK`; load trước `7,97 / 7,70 / 6,97`, sau
`3,15 / 6,13 / 6,98`.

## 2. Gate `./vis-dev gate --fast`

Cả 3 lần exit 0, `vis-dev gate --fast: PASS`; `check` (fmt/clippy/R1..R7/toolchain/release/
fixtures/readme-examples) xanh; doc-test toàn bộ `ok`.

| Lần | nextest | Thời lượng nextest |
|---|---|---:|
| gate-1 | **3754 passed, 4 skipped** | 100,969 s |
| gate-2 | **3754 passed, 4 skipped** | 70,450 s |
| gate-3 | **3754 passed, 4 skipped** | 68,886 s |

- **3754/4** là mốc tại `e9c62cb`: +6 ca so với **3748/4** tại `c4337ce` (security hotfix,
  `SPEC.md`, `f9e7503`).
- Ca hồi quy `vis-plugin-abi wasi::tests_driver_rieng::moi_lan_goi_thu_muc_driver_rieng`
  PASS cả 3 gate; ca từng flake `cap_dir_la_sandbox_tu_nhien` không tái hiện.

## 3. Diff 100k

Mỗi lần chạy kèm bước 2.000 chương trình; dòng 100k:

| Lần | Kết quả | Giây |
|---|---|---:|
| diff-1 | `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 … seed=20261115` | 77,8 |
| diff-2 | `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 … seed=20261115` | 79,7 |
| diff-3 | `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 … seed=20261115` | 76,6 |

`lech=0` cả 3 lần; bước 2.000 cũng `lech=0` (`loi_tinh=126 chay=1874`). Ngoài soak, QA
`623f3ba` đã chạy **5 seed × 100k = 500.000 chương trình**, `lech=0` cả 5.

## 4. Host và cache

- `bin/vis-host-suite`: **54/54** ca xanh + `host_posix_real` PASS, nền `Linux x86_64` — cả 2 lần.
- `cargo test -p vis-cli --test cache_khong_che`: **5 passed / 0 failed** — cả 2 lần
  (10,19 s và 10,54 s).

## 5. Log, sha256, trạng thái

```text
4aed4c203a11a3184e07f1cdc8dc9810ea38de29392ee8b2185a1f2f50b7db42  final-linux-e9c62cb.log
8dd6538caa793b821ce7eea4619e1d073819293001521869e8638d160f228e55  final-linux-e9c62cb.driver.log
```

- Log chính: 1.149.650 byte; log điều khiển: 1.797 byte; cả hai **ngoài repo**.
- **Không có** `final-linux-e9c62cb.md` gốc tại thời điểm chốt; bản này là tóm tắt R7 thay thế.
- Không merge/push/tag; không sửa mã/test; `HEAD = e9c62cb` khi bắt đầu soak.
- Thư mục tạm soak (`/home/ps/tmp/vis-soak-tmp-*`) đã được script dọn khi thoát.

## 6. Kết luận

**10/10 PASS theo log**, 650 s, không blocker. Điều kiện 3 của tag `v0.5.0` đã có đủ số
(gate ×3, diff 100k ×3, host ×2, cache ×2); phần còn thiếu chỉ là **bản `.md` tổng kết R7**
để A0 chốt, không phải số đo.
