<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 fresh-clone QA — v0.3.1 candidate `e689c3d`

> **Người chạy**: W3 (QA/release), 2026-11-21 17:45–17:47 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Commit tested | **`e689c3d8dfbfe64a4f69d4f145f51d21c77ae242`** (docs v0.3.1; code `a96841a`) |
| Clone | `/home/ps/vis-qa-v031` (ngoài repo) |
| Target | `/home/ps/vis-qa-v031/target` (riêng, tên `target` để R3 prune) |
| Env | `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` |
| Disk | đầu **29G**; thấp nhất **28G**; clone 1,4G đã xoá → **29G** |

## 1. Kết quả

| # | Lệnh trong clone | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3512 passed / 3 skipped** | 75,82 s (nextest 35,06 s) | `/tmp/opencode/qa-w3-freshclone-v031.log` |
| 2 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 2,83 s | cùng log |

- Checkout `CHECKOUT=e689c3d8dfbfe64a4f69d4f145f51d21c77ae242`.
- Gate nguồn sạch khớp candidate **3512/3**, không hồi quy.
- Host 54/54 + POSIX thật (Linux x86_64).
- Clone/target đã xoá; không để rác trong repo.

## 2. Ý nghĩa cho tag

- Fresh-clone PASS tại docs `e689c3d` (code `a96841a`) — đủ bằng chứng nguồn sạch Linux cho v0.3.1.
- Cùng QA local (gate 3512/3, nightly, diff 100k, host, targets), local side xanh.
- **Chưa thay thế** Windows psvn `a96841a`; W4 cần verify trước khi A0 tag v0.3.1.

## 3. Blocker

1. Windows psvn `a96841a` chưa verify — blocker tag duy nhất.
2. 2 ô EXPECTED_RED #156 (C toolchain msvc/wasm) — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry /home/ps/vis-qa-v031
cd /home/ps/vis-qa-v031
git checkout e689c3d
export CARGO_TARGET_DIR=/home/ps/vis-qa-v031/target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
./vis-dev gate --fast
./bin/vis-host-suite
# xoá /home/ps/vis-qa-v031 sau khi ghi log
```

**Kết luận**: fresh-clone `e689c3d` **PASS** — gate **3512/3**, host **54/54**. Đề xuất tag v0.3.1
sau khi W4 xác nhận Windows `a96841a`.
