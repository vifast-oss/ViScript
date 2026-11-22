<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-294 — triage soak cycle 31 `http2-h2` rc101 (tag `v0.3.6`)

> **Người làm**: subagent VS3-W2, 2026-11-22. **Base**: `e35dcf6` (ST-294 default async-h2 +
> ST-295). **TMPDIR**: `/home/ps/soak-tmp`; target riêng `target/w2`.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Kết luận | **Test harness**, không phải product race |
| Root cause | `snapshot_vis()` copy **~108 MB cho mỗi process** vào `std::env::temp_dir()` (`/tmp` = tmpfs 7,2 G) và **không dọn**; soak 31 cycle × 2 binary h2 ⇒ ~62 thư mục > dung lượng tmpfs ⇒ ENOSPC, nhiều ca production h2 rc101 |
| Bằng chứng | lúc triage: **41 thư mục `/tmp/vs3-w2-bin-*`, 4,2 G**; tmpfs **6,0/7,2 G used, 1,3 G free**; lỗi chỉ xảy ra sau nhiều cycle, không có SUMMARY chi tiết |
| Tái hiện với TMPDIR mới (trước fix) | `vs3_http2` default **20/20 pass** nhưng sinh **20 snapshot 108 MB** mới |
| Fix | cache snapshot theo **danh tính binary nguồn** (`path+size+mtime`), ghi file tạm + `rename` atomic; một cache/build thay vì một bản/process |
| Sau fix | `vs3_http2` **20/20 pass**, chỉ **1 cache dir**; `async_h2_soak_env_gated` **2 × 1800 s** pass |
| Soak 30 phút ×2 | **1562 + 1678 vòng**, 0 fail; fd 8→9, RSS 6,0→8,5 MB, threads 6→6 |
| Gate Linux | **3613 passed / 0 failed / 4 skipped** |
| Windows `vs3_http2` @ `fd06d528` | **33/33 ×2**, snapshot cache 5→6 (một bản mới, lần 2 tái dùng) |
| Blocker | **không** |

## 1. Triệu chứng

Soak 2h trước (tag `v0.3.6`) fail ở cycle 31 variant `http2-h2`, rc 101; log chỉ có danh sách
test, nhiều ca production h2 `FAILED`, **không có SUMMARY**. Cùng binary chạy 1 lần sau đó
**34+1 pass** ⇒ nghi tài nguyên môi trường/leak harness.

## 2. Điều tra

- `/tmp` trên host là **tmpfs 7,2 G**, lúc triage chỉ còn **1,3 G**; có **41 thư mục
  `vs3-w2-bin-*` × ~108 MB = 4,2 G**.
- `tests/support/h2_async_spike.rs::snapshot_vis()` cũ:
  ```rust
  let d = std::env::temp_dir().join(format!("vs3-w2-bin-{}", std::process::id()));
  fs::copy(env!("CARGO_BIN_EXE_vis"), &dst)
  ```
  Mỗi process test copy **một** bản binary và không xoá. Mỗi cycle soak chạy 2 binary h2
  (`http2-default`, `http2-h2`) ⇒ 2 × 108 MB/cycle; 31 cycle đủ làm đầy tmpfs. `vis` con
  không exec được/ghi file ⇒ hàng loạt ca production h2 fail rc101; đây là **hệ quả hết chỗ**,
  không phải race HTTP/2.
- Với `TMPDIR=/home/ps/soak-tmp`, cũng 20/20 pass nhưng vẫn sinh 20 bản 108 MB ⇒ nếu soak dài
  tiếp tục đầy (chỉ đổi chỗ đầy).
- Không thấy static/thread-local sản phẩm; `vis`/`h2` chạy đơn lẻ đều xanh.

## 3. Fix test-only

`crates/vis-cli/tests/support/h2_async_spike.rs` — `snapshot_vis()`:

- Băm FNV-1a từ **đường dẫn + size + mtime** của `CARGO_BIN_EXE_vis` ⇒ thư mục
  `vs3-w2-bin-cache-<hash>`; mọi process/run cùng build dùng **một** bản snapshot.
- Copy vào `vis.tmp-<pid>` rồi `fs::rename` (atomic); nếu process khác thắng rename thì dùng bản
  đích — tránh vừa copy vừa exec (**ETXTBSY**), không cần PID dir.
- Không đổi kỳ vọng/ngưỡng test, không che lỗi; snapshot vẫn là bản copy bất biến của `vis`.

## 4. Tái hiện + sau fix

| Pha | Cách chạy | Kết quả |
|---|---|---|
| Trước fix, TMPDIR mới | `vs3_http2` default ×20 | **20/20 pass**, sinh **20** snapshot dir |
| Sau fix | `vs3_http2` default ×20 | **20/20 pass**, **1** cache dir (không tăng) |
| Soak 30 phút #1 | `VS3_W2_SOAK_SECS=1800 VS3_W2_SOAK_CAP_EVERY=5`, exact `async_h2_soak_env_gated` | **1562 vòng, PASS**, `fd=8->9`, `rss=6.0->8.5MB`, `threads=6->6` |
| Soak 30 phút #2 | như trên | **1678 vòng, PASS**, `fd=8->9`, `rss=5.7->8.3MB`, `threads=6->6` |
| Cold/stream trong soak | mỗi vòng cold 1 conn/4 stream; thường 1 conn/4 stream | 0 fail, 0 rò |

Log: `JOURNAL/st294-soak31.log` (soak cũ fail), `JOURNAL/st294-triage-before.txt`,
`JOURNAL/st294-triage-repro20.log`, `JOURNAL/st294-triage-soak30.log`.

## 5. Kiểm sau fix

- `vs3_http2` Linux: **34 passed / 1 ignored**; `vs3_http_cancel` **8/8**.
- `./vis-dev gate --fast` (TMPDIR=`/home/ps/soak-tmp`): **PASS — 3613 passed / 0 failed / 4 skipped**.
- Windows `vs3_http2` tại `fd06d528`: **33/33** hai lần liên tiếp; số dir snapshot
  `%TEMP%/vs3-w2-bin-*` **5 → 6** (một cache mới, lần 2 tái dùng; Windows bỏ 2 ca Unix-only).
- df sau đo: **14 G** trống (>10 G); cache snapshot chỉ 1 bản.

## 6. Kết luận

- Soak cycle 31 **không phải lỗi sản phẩm HTTP/2**; là **rò snapshot test harness** làm đầy tmpfs.
- Fix test-only đã loại nguyên nhân (20/20 + 2×30 phút pass, fd/RSS/threads phẳng).
- Không tag/push/merge. Blocker: **không**.
