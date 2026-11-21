<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 fresh-clone QA — v0.3.0 final `0d26527`

> **Người chạy**: W3 (QA/release), 2026-11-21 17:00–17:03 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Commit tested | **`0d26527e0bd0fd54795e6298b351235d4b0a194e`** (docs v0.3.0; code `6f0ea95`) |
| Clone | `/home/ps/vis-qa-v030-final` (ngoài repo, local clone) |
| Target | `/home/ps/vis-qa-v030-final/target` (riêng, **tên `target`** để R3 prune) |
| Env tiết kiệm đĩa | `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **6,8G**; thấp nhất **5,5G** (> 5G); clone 1,4G đã xoá, còn **6,8G** |

## 1. Kết quả

| # | Lệnh trong clone | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3496 passed / 3 skipped** | 86,52 s (nextest 45,38 s) | `/tmp/opencode/qa-w3-freshclone-v030-final.log` |
| 2 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 2,83 s | cùng log (nối tiếp) |

- `CHECKOUT=0d26527e0bd0fd54795e6298b351235d4b0a194e` — đúng commit final.
- Gate nguồn sạch khớp candidate **3496/3**, không hồi quy.
- Host suite 54/54 + POSIX thật (Linux x86_64).
- Clone/target đã xoá (`rm -rf /home/ps/vis-qa-v030-final`), không để rác trong repo.
- Target đặt tên `target` (đúng bài học v0.2.0: `target-qa` bị R3 quét artifact).

## 2. Ý nghĩa cho tag

- Fresh-clone **PASS** tại `0d26527` (code `6f0ea95`): đủ bằng chứng nguồn sạch Linux cho v0.3.0.
- Cùng với QA local (gate 3496/3, nightly, diff 100k, host, targets), local side đã xanh.
- **Chưa thay thế** Windows psvn `6f0ea95` — W4 cần verify trước khi A0 tag.

## 3. Blocker / còn lại

1. **Windows `6f0ea95`** chưa verify — blocker tag duy nhất.
2. **Đĩa thấp** (low-water 5,5G, sát ngưỡng 5G); nếu chạy thêm fresh-clone/Windows cần dọn target cũ.
3. Không GitHub Actions; Windows do W4 psvn thủ công.
4. Tàn dư sản phẩm đã biết (async net production, worker evaluator, `select` VS3-2c) — ghi release
   notes, ngoài phạm vi tag.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry /home/ps/vis-qa-v030-final
cd /home/ps/vis-qa-v030-final
git checkout 0d26527
export CARGO_TARGET_DIR=/home/ps/vis-qa-v030-final/target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
./vis-dev gate --fast
./bin/vis-host-suite
# xoá /home/ps/vis-qa-v030-final sau khi ghi log
```

**Kết luận**: fresh-clone `0d26527` **PASS** — gate **3496/3**, host **54/54**. Đề xuất tag `v0.3.0`
sau khi W4 xác nhận Windows trên code `6f0ea95`.
