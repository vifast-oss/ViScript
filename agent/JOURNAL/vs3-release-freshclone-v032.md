<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 fresh-clone QA — v0.3.2 candidate `0ff2cac`

> **Người chạy**: W3 (QA/release), 2026-11-21 18:40–18:42 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Commit tested | **`0ff2cacc3139c20dad6723ba3f3d716d95e9e7c0`** (docs v0.3.2; code `5c13741`) |
| Clone | `/home/ps/vis-qa-v032` (ngoài repo) |
| Target | `/home/ps/vis-qa-v032/target` (riêng, tên `target` để R3 prune) |
| Env | `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` |
| Disk | đầu **9,9G**; thấp nhất **8,5G** (> 5G); clone 1,4G đã xoá → **9,9G** |

## 1. Kết quả

| # | Lệnh trong clone | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3528 passed / 3 skipped** | 95,77 s (nextest 47,05 s) | `/tmp/opencode/qa-w3-freshclone-v032.log` |
| 2 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 2,71 s | cùng log |

- `CHECKOUT=0ff2cacc3139c20dad6723ba3f3d716d95e9e7c0`.
- Gate nguồn sạch khớp candidate **3528/3**, không hồi quy.
- Host 54/54 + POSIX thật (Linux x86_64).
- Clone/target đã xoá; không để rác trong repo.

## 2. Ý nghĩa cho tag

- Fresh-clone PASS tại docs `0ff2cac` (code `5c13741`) — đủ bằng chứng nguồn sạch Linux cho v0.3.2.
- Cùng QA local (gate 3528/3, nightly, diff 100k, host, targets), local side xanh.
- **Chưa thay thế** Windows psvn `5c13741`; W4 cần verify trước khi A0 tag v0.3.2.

## 3. Blocker

1. Windows psvn `5c13741` chưa verify — blocker tag duy nhất.
2. 2 ô EXPECTED_RED #156 (C toolchain msvc/wasm) — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry /home/ps/vis-qa-v032
cd /home/ps/vis-qa-v032
git checkout 0ff2cac
export CARGO_TARGET_DIR=/home/ps/vis-qa-v032/target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
./vis-dev gate --fast
./bin/vis-host-suite
# xoá /home/ps/vis-qa-v032 sau khi ghi log
```

**Kết luận**: fresh-clone `0ff2cac` **PASS** — gate **3528/3**, host **54/54**. Đề xuất tag v0.3.2
sau khi W4 xác nhận Windows `5c13741`.
