<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 fresh-clone QA — v0.2.0 candidate `42b700c`

> **Người chạy**: W3 (QA/release), 2026-11-21 16:31–16:37 JST.
> **Phạm vi**: chỉ đo; **không sửa code/docs chung**, không tag/push/merge.
> **Local only — không dùng GitHub Actions.**

## 0. Mốc

| Mục | Giá trị |
|---|---|
| Commit tested | **`42b700c2eef560e8eb3033071a521e5cec5fb379`** (`42b700c`, merge W5e timeout kênh) |
| Docs candidate | `28c359b` không đổi code — QA dùng thẳng `42b700c` |
| Clone | `/home/ps/vis-qa-v020` (ngoài repo, full local clone) |
| Target | `/home/ps/vis-qa-v020/target` (riêng, trong clone — tên `target` để R3 prune) |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |
| Disk | đầu **15G**; thấp nhất **6,4G** (> 5G); clone chiếm 8,4G, đã xoá sau QA → **15G** |

## 1. Kết quả

| # | Lệnh trong clone | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3480 passed / 3 skipped** | 283,41 s (nextest 54,05 s) | `/tmp/opencode/qa-w3-freshclone-v020b.log` |
| 2 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 1,66 s | cùng log (nối tiếp) |

- Checkout xác nhận `CHECKOUT=42b700c2eef560e8eb3033071a521e5cec5fb379`.
- Gate nguồn sạch khớp candidate: **3480/3480 + 3 skipped**, `vis-dev check` PASS, không hồi quy.
- Host suite: 54/54 trên Linux x86_64 + bằng chứng POSIX thật.
- Clone + target đã xoá (`rm -rf /home/ps/vis-qa-v020`), không để lại rác trong repo.

## 2. Ghi chú lần chạy đầu (procedure, không phải lỗi sản phẩm)

Lần 1 dùng `CARGO_TARGET_DIR=/home/ps/vis-qa-v020/target-qa` (tên không phải `target`) ⇒ R3 quét
`target-qa/**` và báo thiếu sidecar `.license`, gate **FAIL sau 18,51 s** (log
`/tmp/opencode/qa-w3-freshclone-v020.log`). Đã xoá clone, chạy lại với target tên **`target`**
(R3 prune `*/target/*`) ⇒ PASS như bảng trên.

> Bài học: khi target nằm **trong clone**, phải đặt tên `target` (hoặc ngoài clone) để R3 không
> quét artifact build.

## 3. Ý nghĩa cho release

- Fresh-clone tại code candidate `42b700c` **xanh**: gate 3480/3 + host 54/54.
- Đây là bằng chứng nguồn sạch cho v0.2.0, **không** thay thế yêu cầu Windows/psvn tại cùng commit.
- `v0.1.0`/`v0.2.0` tag vẫn do A0 quyết; W3 không tag.

## 4. Blocker / còn lại

1. **Windows psvn** tại `42b700c` vẫn cần W4 verify (QA này chỉ Linux local).
2. A0 cần chốt tàn dư 1d (`cap>0`/net async) và 3b (“chưa nối evaluator”) trước khi tag v0.2.0.
3. HTTP/2 default-on (`959d1aa`) vẫn chưa merge; chỉ sau v0.2.0 + Windows h2.
4. Đĩa từng xuống 6,4G; nếu chạy thêm fresh-clone/Windows cần dọn target cũ.
5. Không dùng GitHub Actions; mọi kết quả là local Linux.

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry /home/ps/vis-qa-v020
cd /home/ps/vis-qa-v020
git checkout 42b700c
CARGO_TARGET_DIR=/home/ps/vis-qa-v020/target ./vis-dev gate --fast
CARGO_TARGET_DIR=/home/ps/vis-qa-v020/target ./bin/vis-host-suite
# xoá /home/ps/vis-qa-v020 sau khi ghi log
```

**Kết luận**: fresh-clone `42b700c` **PASS** — gate **3480/3**, host **54/54**. Không có blocker kỹ
thuật mới; còn Windows psvn + A0 sign-off trước tag v0.2.0.
