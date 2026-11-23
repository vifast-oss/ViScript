<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final QA `v0.3.6` — `4c5dec6`: fresh-clone, release, 2× soak, diff 100k

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 15:20–15:58 JST (~38 phút; time-box 2 h).
> **Phạm vi**: **chỉ đo**. Không sửa code/docs chung, không push/merge/tag, không GitHub Actions.
> **Nguồn**: `main` = **`4c5dec6`** (`merge ST-289 QA: release 3581/0 + 2x soak 13d184d`;
> Stage 0 **6/6**, version **0.3.6**).

## 0. Mốc đo

| Mục | Giá trị |
|---|---|
| Commit đo | **`4c5dec62069244ed624dd3952b7e593aa2983d06`** |
| Phiên bản | **0.3.6** (`debug/vis --version` và `release/vis --version` đều `vis 0.3.6`) |
| Nhánh QA | `plan/w3-platform` — merge `main` **fast-forward** lên `4c5dec6`; không sửa gì thêm |
| Clone | `/home/ps/vis-qa-v036` (clone từ `/home/ps/ViScript`, checkout `4c5dec6`) |
| Target | clone: `$PWD/target`; soak: `target/soak1`, `target/soak2` — **riêng**, `CARGO_INCREMENTAL=0` |
| Toolchain | `~/.cargo/bin/cargo` **1.100.0**, `rustc 1.100.0 (0ef1bc386 2026-11-09)` |
| Máy | Linux x86_64, 12 CPU; load dao động ~3–11 (thấp hơn các phiên trước) |
| Đĩa | đầu **17 G**; cuối **11 G** (> 5 G) |

## 1. Fresh-clone `4c5dec6` — cổng

```bash
git clone /home/ps/ViScript /home/ps/vis-qa-v036 && cd /home/ps/vis-qa-v036
git checkout 4c5dec6
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
```

| Bước | Kết quả |
|---|---|
| `fmt` / `clippy` / R1..R6 | **OK** |
| self-test + fixtures + doc-examples | **OK** (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD) |
| `report-check` | **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ) |
| L0 `cargo-nextest` | **3581 passed / 3 skipped** (64,32 s) |
| **`gate --fast`** | **PASS** — wall **2:10,99** |
| Log | `/tmp/opencode/final-qa-v036-gate.log` |

## 2. Release workspace — kỳ vọng ~3586/0/3

```bash
cargo test --release --workspace --no-fail-fast
```

| Chỉ số | Giá trị |
|---|---|
| Kết quả | **PASS** (`rc=0`, không `FAILED`) |
| Binary test | **355** |
| **passed** | **3586** |
| **failed** | **0** |
| **ignored** | **3** |
| **blocked** | **0** |
| Thời gian | wall **5:04,88**, max RSS ~622 MB |
| Log | `/tmp/opencode/final-qa-v036-release.log` |

**So với debug 3581/3**: +5 passed = đúng 5 doc-test (`vis_interp` 2 + `vis_syntax` 1 + `vis_types` 2)
mà nextest của gate không đếm; không test nào đổi trạng thái passed/skipped.

## 3. Targeted suites

Tất cả `rc=0`, `0 failed`:

| Suite | Lệnh | Kết quả |
|---|---|---|
| **P2** | `cargo test -p vis-cli --test vs3_worker_eval_parallel` | **14/14** |
| **P1** | `cargo test -p vis-cli --test vs3_worker_eval_local` | **8/8** |
| **P3** | `cargo test -p vis-cli --test vs3_worker_eval_vm` | **10/10** |
| h2 mặc định | `cargo test -p vis-cli --test vs3_http2` | **19/19** |
| h2 feature | `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 1 ignored** |
| `pkg` | `cargo test -p vis-cli --test pkg` | **25/25** |
| `vs3_registry_sig` | `cargo test -p vis-cli --features pkg-sig --test vs3_registry_sig` | **10/10** |
| `vs3_http_cancel` mặc định | `cargo test -p vis-cli --test vs3_http_cancel` | **5/5** |
| `vs3_http_cancel` feature | `cargo test -p vis-cli --features async-h2 --test vs3_http_cancel` | **8/8** |
| host | `./bin/vis-host-suite` | **54/54** + `host_posix_real` PASS |
| phiên bản | `debug/vis --version` / `release/vis --version` | **`vis 0.3.6`** / **`vis 0.3.6`** |

Log: `/tmp/opencode/final-qa-v036-suites.log`.

## 4. Differential 100k — seed mới

```bash
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221700 ./bin/vis-diff
```

| Chỉ số | Giá trị |
|---|---|
| Seed | **202611221700** (mới; khác `20261115`, `202611221318`, `202611221545`) |
| Tổng | **100.000** |
| Lỗi tĩnh | 6.232 |
| Chạy | 93.768 |
| **Lệch interp↔VM** | **0** (`DIFF … lech=0`) |
| Thời gian | 81,5 s test / **86 s wall** (1.227 chương trình/s) |
| Log | `/tmp/opencode/final-qa-v036-diff.log` |

## 5. Hai lần soak 10/10 trên `4c5dec6`

`./bin/vis-soak` (3× `gate --fast`, 3× diff 100k seed `20261115`, 2× host, 2× cache), mỗi lần
target riêng, `CARGO_INCREMENTAL=0`, `debug=0`:

| Lần | Target | Tổng | 10 vòng | Gate | Diff 100k | Host | Cache |
|---|---|---|---|---|---|---|---|
| **1** | `target/soak1` | **593 s** (wall 9:53) | **10/10 OK** | 3× **3581/3** | 3× **lech=0** | 2× **54/54** | 2× **5/5** |
| **2** | `target/soak2` | **602 s** (wall 10:02) | **10/10 OK** | 3× **3581/3** | 3× **lech=0** | 2× **54/54** | 2× **5/5** |

Chi tiết vòng:

| Vòng | Soak 1 | Soak 2 |
|---|---|---|
| gate-1 | 130 s OK | 134 s OK |
| gate-2 | 106 s OK | 107 s OK |
| diff-1 | 81 s OK (`tong=100000 lech=0`) | 79 s OK (`tong=100000 lech=0`) |
| diff-2 | 76 s OK (lech=0) | 76 s OK (lech=0) |
| gate-3 | 101 s OK | 106 s OK |
| diff-3 | 75 s OK (lech=0) | 77 s OK (lech=0) |
| host-1 | 4 s OK (54/54) | 3 s OK (54/54) |
| host-2 | 1 s OK (54/54) | 1 s OK (54/54) |
| cache-1 | 9 s OK (5/5) | 10 s OK (5/5) |
| cache-2 | 10 s OK (5/5) | 9 s OK (5/5) |
| **Tổng** | **593 s — 10/10 OK** | **602 s — 10/10 OK** |

Cả 6 gate trong 2 soak đều **3581 passed / 3 skipped**; cả 6 diff đều **`lech=0`**. Không vòng đỏ,
không retry, không T-INFRA.

## 6. Đánh giá

- Fresh-clone **`4c5dec6`** khớp **toàn bộ** kỳ vọng: gate **3581/3**, release workspace
  **3586/0/3**, P2 **14**, P1 **8**, P3 **10**, h2 **19**, h2 feature **34+1**, `pkg` **25**,
  `sig` **10**, `vs3_http_cancel` **5** + feature **8**, host **54/54**, `vis --version` **0.3.6**.
- Diff 100k seed **202611221700**: **lệch 0**.
- **2 soak × 10/10 vòng OK** (593 s + 602 s = **1195 s**).
- Không flake/T-INFRA; không suite đỏ.

## 7. Kết luận / đề xuất

**TẤT CẢ XANH.** Đề nghị **tag `v0.3.6` @ `4c5dec6`** (việc tag/push do người có thẩm quyền quyết
định; phiên này không tag/push).

## 8. Blocker / ghi chú

1. **Không có blocker**; không test failed/blocked.
2. Windows (`psvn`) và macOS **không đo** trong phiên này (macOS ngoài phạm vi dự án).
3. Không sửa code/docs chung; chỉ thêm journal này.
4. Đĩa cuối **11 G** (> 5 G); có thể xoá `target` (worktree, 3 G) và `/home/ps/vis-qa-v036`
   (2,8 G target) sau khi tag nếu cần thêm chỗ.
5. Log bằng chứng: `/tmp/opencode/final-qa-v036-{gate,release,suites,diff,soak1,soak2}.log`.

## 9. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH" CARGO_INCREMENTAL=0
git clone /home/ps/ViScript /home/ps/vis-qa-v036 && cd /home/ps/vis-qa-v036
git checkout 4c5dec6
export CARGO_TARGET_DIR=$PWD/target CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast                                              # 3581/3
cargo test --release --workspace --no-fail-fast                    # 3586/0/3
for t in vs3_worker_eval_parallel vs3_worker_eval_local vs3_worker_eval_vm vs3_http2 pkg vs3_http_cancel; do
  cargo test -p vis-cli --test "$t"
done
cargo test -p vis-cli --features async-h2 --test vs3_http2         # 34+1
cargo test -p vis-cli --features async-h2 --test vs3_http_cancel   # 8
cargo test -p vis-cli --features pkg-sig --test vs3_registry_sig   # 10
./bin/vis-host-suite                                               # 54/54
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221700 ./bin/vis-diff        # lech=0

cd /home/ps/.local/share/opencode/worktree/cd45e5/vs3-w3-registry
for n in 1 2; do
  CARGO_TARGET_DIR=$PWD/target/soak$n CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
    ./bin/vis-soak --log /tmp/opencode/final-qa-v036-soak$n.log
done
```
