<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 fresh-clone — `5103da4` (CLI/doctor + net flake fix)

> **Người chạy**: W3 (QA/hardening), 2026-11-21 21:22–21:26 JST.
> **Phạm vi**: chỉ đo; không sửa code/docs chung, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main`/commit tested | **`5103da4c276683aea0cc19245593dc4d28f9ffd2`** (`5103da4`) |
| Nội dung | CLI `--version`/`doctor` + Windows net flake fix (W4 psvn 3/3) |
| Clone | `/home/ps/vis-qa-main5103` (ngoài repo) |
| Target | `/home/ps/vis-qa-main5103/target` (riêng, tên `target` để R3 prune) |
| Env | `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0` |
| Disk | đầu **19G**; trong **17G** (> 5G); sau xoá **19G** |

## 1. Kết quả

| # | Lệnh trong clone | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3530 passed / 3 skipped** | 125,31 s (nextest 59,94 s) | `/tmp/opencode/qa-w3-freshclone-5103da4.log` |
| 2 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 3,77 s | cùng log |

- `CHECKOUT=5103da4c276683aea0cc19245593dc4d28f9ffd2`.
- Gate nguồn sạch khớp main **3530/3**, không hồi quy.
- Host 54/54 + POSIX thật (Linux x86_64).
- Clone/target đã xoá; không để rác trong repo.

## 2. Đánh giá

- Fresh-clone **PASS** tại `5103da4`: nguồn sạch Linux ổn định sau CLI/doctor + net flake fix.
- Không có test đỏ, không T-INFRA.
- Không cần tag mới; đây là hardening sau `v0.3.2`.

## 3. Blocker / còn lại

1. Không có blocker local.
2. 2 ô `EXPECTED_RED` msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
3. Không GitHub Actions; Windows do W4 thủ công (commit này có net flake fix, psvn 3/3 theo docs).
4. Tàn dư đã biết: worker evaluator, `select` VS3-2c, async HTTP Unix — release notes.

## 4. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry /home/ps/vis-qa-main5103
cd /home/ps/vis-qa-main5103
git checkout 5103da4
export CARGO_TARGET_DIR=/home/ps/vis-qa-main5103/target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
./vis-dev gate --fast
./bin/vis-host-suite
# xoá /home/ps/vis-qa-main5103 sau khi ghi log
```

**Kết luận**: fresh-clone `5103da4` **PASS** — gate **3530/3**, host **54/54**. Không có blocker kỹ
thuật local.
