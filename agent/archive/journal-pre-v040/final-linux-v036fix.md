<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Linux QA `v0.3.6` — candidate `0645700` (fix h1 cancel Windows)

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 17:24–17:47 JST (~23 phút; time-box 1 h).
> **Phạm vi**: **chỉ đo**. Không sửa code/test, không push/merge/tag, không GitHub Actions.
> **Nguồn**: `main` = **`0645700`** (`merge ST-287: fix Windows h1 cancel (blocking accept + cancel guard)`).

## 0. Mốc đo

| Mục | Giá trị |
|---|---|
| Commit đo | **`06457003ae3e38bfa682d4ec953157606130849e`** |
| Phiên bản | **0.3.6** (`debug/vis --version`) |
| Nhánh QA | `plan/w3-platform` — merge `main` fast-forward lên **`0645700`**; không sửa gì thêm |
| Clone | `/home/ps/vis-qa-v036fix` (clone từ `/home/ps/ViScript`, checkout `0645700`) |
| Target | clone: `$PWD/target`; soak: `target/soak1` — **riêng**, `CARGO_INCREMENTAL=0` |
| Toolchain | `~/.cargo/bin/cargo` **1.100.0**, `rustc 1.100.0 (0ef1bc386 2026-11-09)` |
| Máy | Linux x86_64, 12 CPU; load ~3–7 |
| Đĩa | đầu **16 G**; cuối **12 G** (> 5 G); clone `.git` + `target` 2,8 G; soak 1,5 G |

## 1. Fresh-clone `0645700` — cổng

```bash
git clone /home/ps/ViScript /home/ps/vis-qa-v036fix && cd /home/ps/vis-qa-v036fix
git checkout 0645700
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
```

| Bước | Kết quả |
|---|---|
| `fmt` / `clippy` / R1..R6 | **OK** |
| self-test + fixtures + doc-examples | **OK** (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD) |
| `report-check` | **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ) |
| L0 `cargo-nextest` | **3581 passed / 3 skipped** (74,25 s) |
| **`gate --fast`** | **PASS** — wall **2:33,66** |
| Log | `/tmp/opencode/final-linux-v036fix-gate.log` |

## 2. Release workspace

```bash
cargo test --release --workspace --no-fail-fast
```

| Chỉ số | Giá trị |
|---|---|
| Kết quả | **PASS** (`rc=0`) |
| Binary test | **355** |
| **passed** | **3586** |
| **failed** | **0** |
| **ignored** | **3** |
| **blocked** | **0** |
| Thời gian | wall **4:53,63** |
| Log | `/tmp/opencode/final-linux-v036fix-release.log` |

So với gate: +5 passed = 5 doc-test mà nextest không đếm.

## 3. `vs3_http_cancel` — fix Windows có hồi quy Linux?

| Pha | Lệnh | Kết quả |
|---|---|---|
| default ×20 | `cargo test -q -p vis-cli --test vs3_http_cancel` | **20/20 suite pass** (0 fail) |
| feature h2 ×5 | `cargo test -q -p vis-cli --features async-h2 --test vs3_http_cancel` | **8/8 ×5** = **40/40 test pass**, 0 ignored |

- Default suite 5 ca ×20 = **100/100 test pass**; feature 5 lần ×8 = **40/40**. Không có log fail.
- Trigger mới theo byte (`frame_cancel_after_bytes_for_test`) + fixture server blocking không làm
  hồi quy Linux.

## 4. Targeted khác (debug)

| Suite | Kết quả |
|---|---|
| P2 `vs3_worker_eval_parallel` | **14/14** |
| P1 `vs3_worker_eval_local` | **8/8** |
| P3 `vs3_worker_eval_vm` | **10/10** |
| host | **54/54** + `host_posix_real` PASS |
| `debug/vis --version` | **`vis 0.3.6`** |

Log: `/tmp/opencode/final-linux-v036fix-suites.log`.

## 5. Soak 10/10 trên `0645700`

`./bin/vis-soak` (3× `gate --fast`, 3× diff 100k seed `20261115`, 2× host, 2× cache), target
`target/soak1`, `CARGO_INCREMENTAL=0`, `debug=0`:

| Vòng | Thời gian | Kết quả |
|---|---:|---|
| gate-1 | 130 s | OK (3581/3) |
| gate-2 | 106 s | OK (3581/3) |
| diff-1 | 72 s | OK (`tong=100000 lech=0`) |
| diff-2 | 72 s | OK (`lech=0`) |
| gate-3 | 103 s | OK (3581/3) |
| diff-3 | 70 s | OK (`lech=0`) |
| host-1 | 4 s | OK (54/54) |
| host-2 | 0 s | OK (54/54) |
| cache-1 | 10 s | OK (5/5) |
| cache-2 | 9 s | OK (5/5) |
| **TỔNG** | **576 s** (wall 9:36) | **10/10 OK** |

Load trước **4,33 / 7,49 / 5,89**, sau **3,31 / 6,01 / 6,18**. Không vòng đỏ, không retry,
không T-INFRA. Log: `/tmp/opencode/final-linux-v036fix-soak.log`.

## 6. Differential 100k — seed mới

```bash
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221730 ./bin/vis-diff
```

| Chỉ số | Giá trị |
|---|---|
| Seed | **202611221730** (mới; khác `20261115`, `202611221318`, `202611221545`, `202611221700`) |
| Tổng | **100.000** |
| Lỗi tĩnh | 6.233 |
| Chạy | 93.767 |
| **Lệch interp↔VM** | **0** (`DIFF … lech=0`) |
| Thời gian | 76,5 s test / **81 s wall** (1.306 ch/s) |
| Log | `/tmp/opencode/final-linux-v036fix-diff.log` |

## 7. Đánh giá

- Fresh-clone **`0645700`** khớp toàn bộ kỳ vọng: gate **3581/3**, release **3586/0/3**,
  h1 cancel default **20/20 suite** + feature **8/8 ×5**, P2 **14**, P1 **8**, P3 **10**,
  host **54/54**, `vis --version` **0.3.6**.
- Soak **10/10** (576 s); diff 100k seed **202611221730** **0 lệch**.
- Không flake/T-INFRA; không suite đỏ.

## 8. Kết luận / đề xuất

**TẤT CẢ XANH.** Kết hợp matrix Windows của W2 (`6b2a2161`: serial **20/20**, workspace
**3538/0/3**) và QA Linux này: **đề xuất tag `v0.3.6` @ `0645700`** (tag/push do người có thẩm
quyền; phiên này không tag/push).

## 9. Blocker / ghi chú

1. **Không có blocker** local; không test failed/blocked.
2. Windows/`psvn` và macOS **không đo lại** trong phiên này; bằng chứng Windows lấy từ
   `JOURNAL/st287-win-cancel-flake.md` @ `6b2a2161` (sau fix) và merge `0645700`.
3. Không sửa code/test; chỉ thêm journal này. Không tag/push/merge.
4. Đĩa cuối **12 G** (> 5 G); có thể xoá `/home/ps/vis-qa-v036fix` (2,8 G target) và
   `target/soak1` (1,5 G) sau khi tag.
5. Log bằng chứng: `/tmp/opencode/final-linux-v036fix-{gate,release,suites,soak,diff}.log`.

## 10. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0
git clone /home/ps/ViScript /home/ps/vis-qa-v036fix && cd /home/ps/vis-qa-v036fix
git checkout 0645700
export CARGO_TARGET_DIR=$PWD/target CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test --release --workspace --no-fail-fast
for i in $(seq 1 20); do cargo test -q -p vis-cli --test vs3_http_cancel; done
for i in $(seq 1 5); do cargo test -q -p vis-cli --features async-h2 --test vs3_http_cancel; done
cargo test -p vis-cli --test vs3_worker_eval_parallel
cargo test -p vis-cli --test vs3_worker_eval_local
cargo test -p vis-cli --test vs3_worker_eval_vm
./bin/vis-host-suite
./target/debug/vis --version
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221730 ./bin/vis-diff

cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry
CARGO_TARGET_DIR=$PWD/target/soak1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  ./bin/vis-soak --log /tmp/opencode/final-linux-v036fix-soak.log
```
