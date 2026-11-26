<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows `--full` trên `main` `c6724b46` — hậu-merge `ST-315b` (`Value::Map(Rc<VisMap>)`)

> **Người đo**: Gatekeeper (Agent 1, subagent phiên sạch), 2026-11-26.
> **Commit kiểm**: `c6724b466a10f52857f950c82bbc38ec5e8bfe3a` (`[ST-315a] docs: ghi nhan hau-kiem R-A PASS + test F2`,
> đỉnh `main` sau merge `ST-315b`).
> **Máy**: `psvn` (MINGW64_NT-10.0-26300, 6 nhân, cargo/rustc 1.100.0). **Không** dùng `mps`.
> **Worktree điều khiển**: `/home/ps/ViScript-rwin`, nhánh `review/rwin` @ `c6724b46` (nền cùng commit).
> **Repo đo**: `/c/Users/ps/viscript-moi`, nhánh `main` @ `c6724b46`, `git status --porcelain` **rỗng**
> trước và sau khi chạy.
> **Kết quả: PASS** — exit 0; workspace **3776 passed / 0 failed / 4 ignored** (89 block), thêm lượt
> host riêng **1/1** ⇒ tổng **3777/0/4** (90 block); host conformance **54/54**; targets
> **core 47 PASS**, **sqlite 46 PASS + 1 EXPECTED_RED**; clippy `--workspace --all-targets -D warnings`
> sạch. Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả | Ghi chú |
|---|---|---|
| Đồng bộ `psvn` | `pull --ff-only` OK → HEAD `c6724b46…` khớp local; dirty 0 | `vis-gate-windows` tự kiểm HEAD, in `HEAD khớp c6724b46` |
| `bin/vis-gate-windows --full` | **exit 0** | wall **407 s** (`05:05:13Z` → `05:12:00Z`) |
| clippy `--workspace --all-targets -- -D warnings` | **PASS** | `Finished dev profile … 14.45s`; 0 dòng `warning:` của cargo (2 dòng WARNING còn lại là banner SSH post-quantum) |
| `cargo test --workspace --no-fail-fast` | **3776 passed / 0 failed / 4 ignored**, **89 block** (66 binary + 23 doc-test) | không có `FAILED`, không `failures:` |
| Host conformance (riêng, `--nocapture`) | **1/1 block**, suite **54/54** (120 lần chạy interp+VM, 0 lệch; VM 54/54) | chạy **hai** lần: trong workspace và lần riêng — cả hai 54/54 |
| Tổng test-result | **3777 passed / 0 failed / 4 ignored**, **90 block** | |
| `bin/vis-targets` | core **PASS=47 EXPECTED_RED=0 FAIL=0**; sqlite **PASS=46 EXPECTED_RED=1 FAIL=0**; phủ **100% workspace (23 crate)**; `vis-dag-check` OK (23 crate / 5 tầng) | wasm32-wasip1 **chưa đo lượt này** |
| Test mới `st315b_map_cow` | **6/6 ok** trong `vis-interp --test interp_it` | gồm chia sẻ/COW, băm lưới, Q1/Q2/Q3, `eq_value_map` |
| Dirty cuối trên `psvn` | **0** mục; HEAD không đổi | |
| Blocker | **Không** | |

## 1. Lệnh và bằng chứng

```bash
# đồng bộ (từ /home/ps/ViScript-rwin)
bin/vis-remote psvn -- git -C /c/Users/ps/viscript-moi pull --ff-only
bin/vis-remote psvn -- git -C /c/Users/ps/viscript-moi rev-parse HEAD   # c6724b466a10f5…fe3a

VIS_GATE_WINDOWS_LOG=/home/ps/vislogs/st315b-win-full-c6724b46.log \
    ./bin/vis-gate-windows --full                                        # exit 0
```

- Thân lệnh do gate dựng (nguyên văn trong header log):
  `export TMPDIR=/c/Users/ps/tmp …; cd /c/Users/ps/viscript-moi && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --no-fail-fast && cargo test -p vis-cli --test host_conformance -- --nocapture && bash bin/vis-targets`
- `vis-remote` kiểm cây bẩn ST-175: `cay SACH (git status --porcelain rong) — duoc phep chay/do`.
- Mã thoát lấy từ tệp `.exit` trên máy đích: `[exit=0]` (`vis-remote-exit=0`).

## 2. Log và sha256

| Bản | Đường dẫn | sha256 |
|---|---|---|
| Local (bản sao trên `ps`) | `/home/ps/vislogs/st315b-win-full-c6724b46.log` (254 319 B) | `a24d7b916981aaef89194d9fb49751610840ef207b868f4d99548fec245a0496` |
| Remote (log gốc trên `psvn`) | `/c/Users/ps/vislogs/vis-remote-20261126T050511Z-3170059.log` | `71f6bd6172f94d8d73be08a01515c1cd2972ea84545b6e8d6faa2b048ab805fe` |

Hai sha **khác nhau có chủ đích**: bản local được `vis-remote` bọc thêm header `# lenh…`, `# HEADER VD-136`,
khối `# ---------- KET ----------` và dòng `[exit=0]`; phần thân log do `psvn` sinh trùng khớp.
Sha remote đã đọc lại độc lập bằng `sha256sum` trên `psvn` — khớp giá trị `# sha256(log_tren_may_dich)`.

## 3. Phân bố test (trích từ log)

| Nhóm | Block | Passed | Failed | Ignored |
|---|---:|---:|---:|---:|
| `cargo test --workspace` (66 binary + 23 doc-test) | 89 | 3776 | 0 | 4 |
| `cargo test -p vis-cli --test host_conformance` (lượt riêng) | 1 | 1 | 0 | 0 |
| **Tổng** | **90** | **3777** | **0** | **4** |

- 4 ignored: 3 trong `vis-conformance` (`test result: ok. 361 passed; 0 failed; 3 ignored` — ca
  `#[ignore]` sẵn có) + 1 doc-test `vis-plugin-macros` (`src/lib.rs - (line 4) ... ignored`).
- Các binary lớn đều xanh: `vis-cli async_it` 522, `vis-conformance` 493, `interp_it` 361,
  `vis-runtime` unittests 284, `vis-types types_it` 216, `vis-cli net_pkg_it`…; `vis-vm vm_it` 56,
  `vis-vm diff_100k` 7, `fuzz_mutate` 1 (28,98 s).
- Không có dòng `FAILED`, `failures:`, `error[`, `panicked` trong toàn log.

## 4. Host conformance

```
1..54
# 54 ca, 120 lần chạy (interp + vm), 0 lệch; VM 54/54
test host_conformance_suite ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.53s
```

Chạy hai lần (một lần như test target của workspace, một lần theo lệnh riêng của gate) — cả hai
**54/54**, 0 lệch interp/VM.

## 5. `vis-targets`

```
vis-targets: phủ 100% workspace (23 crate)
vis-dag-check: OK — 23 crate / 5 tầng, 0 cạnh cấm
core   (vis-cli --no-default-features): PASS=47 EXPECTED_RED=0 FAIL=0
sqlite (vis-cli default, cấu hình cũ) : PASS=46 EXPECTED_RED=1 FAIL=0
```

- 47/47 ô `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown` của cấu hình core PASS.
- Cấu hình sqlite: `vis-cli` × `wasm32-unknown-unknown` vẫn `EXPECTED_RED` (nợ **#156**: getrandom/js,
  libsqlite3-sys/ring cần C toolchain) — đúng kỳ vọng, **không** phải hồi quy.
  Ghi chú của công cụ: ô `vis-cli` × `x86_64-pc-windows-msvc` **từng** EXPECTED_RED **nay xanh** —
  cần rà soát lại nợ #156 (không chặn cổng này).
- `wasm32-wasip1` và `aarch64-apple-darwin`: **không đo** trong lượt này (đúng thiết kế
  `vis-gate-windows --full`; mốc wasip1 lần cuối 2026-11-25: 390/390).

## 6. Caveat

1. **Thời gian 407 s** ngắn hơn khung 15–30 phút dự kiến vì `target/` trên `psvn` đã có artifact
   từ lượt trước (chỉ recompile phần đổi của `ST-315b`: 58 s cho test profile); đây vẫn là
   `--full` đầy đủ (clippy + toàn workspace + host + targets), không phải `--quick`.
2. Không chạy `--wasm`, không soak, không fuzz dài — ngoài phạm vi `--full`.
3. Không sửa mã nguồn, không tag/push/merge; chỉ ghi journal này trên nhánh `review/rwin`.
4. Bản local của log nằm ngoài repo (`/home/ps/vislogs/`) theo luật R7; sha local khác remote như §2.

## 7. Kết luận

**PASS** — `main` @ `c6724b46` xanh trên Windows full sau merge `ST-315b`: 3777 passed / 0 failed,
host 54/54, targets đúng kỳ vọng, clippy sạch. Đề nghị Agent 0 ghi nhận cổng Windows cho `ST-315b`.
Blocker: **không**.
