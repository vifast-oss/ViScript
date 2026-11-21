<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 fresh-clone QA — v0.3.3 candidate `bbf3484`

> **Người chạy**: W3 (QA/release), 2026-11-21 21:52–21:55 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Commit tested | **`bbf3484adac304404717339b2280f113e23e8610`** (docs v0.3.3; code `0f18592`) |
| Clone | `/home/ps/vis-qa-v033` (ngoài repo) |
| Target | `/home/ps/vis-qa-v033/target` (riêng, tên `target` để R3 prune) |
| Env | `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` |
| Disk | đầu **17G**; thấp nhất **16G** (> 5G); clone 1,4G đã xoá → **17G** |

## 1. Kết quả

| # | Lệnh trong clone | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3535 passed / 3 skipped** | 110,72 s (nextest 48,86 s) | `/tmp/opencode/qa-w3-freshclone-v033.log` |
| 2 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 3,46 s | cùng log |

- `CHECKOUT=bbf3484adac304404717339b2280f113e23e8610`.
- Gate nguồn sạch khớp candidate **3535/3**, không hồi quy.
- Host 54/54 + POSIX thật (Linux x86_64).
- Clone/target đã xoá; không để rác trong repo.

## 2. Ý nghĩa cho tag

- Fresh-clone PASS tại docs `bbf3484` (code `0f18592`) — đủ bằng chứng nguồn sạch Linux cho v0.3.3.
- Cùng QA local (gate 3535/3, nightly, diff 100k, host, targets), local side xanh.
- **Chưa thay thế** Windows psvn `0f18592`; W4 cần verify trước khi A0 tag v0.3.3.

## 3. Blocker

1. Windows psvn `0f18592` chưa verify — blocker tag duy nhất.
2. 2 ô EXPECTED_RED #156 (C toolchain msvc/wasm) — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry /home/ps/vis-qa-v033
cd /home/ps/vis-qa-v033
git checkout bbf3484
export CARGO_TARGET_DIR=/home/ps/vis-qa-v033/target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
./vis-dev gate --fast
./bin/vis-host-suite
# xoá /home/ps/vis-qa-v033 sau khi ghi log
```

**Kết luận**: fresh-clone `bbf3484` **PASS** — gate **3535/3**, host **54/54**. Đề xuất tag v0.3.3
sau khi W4 xác nhận Windows `0f18592`.
