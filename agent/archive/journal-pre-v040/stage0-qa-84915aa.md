<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Stage 0 QA — fresh-clone `84915aa` (`v0.3.6`): gate, 10 suite, diff 100k

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 14:36–14:56 JST (~20 phút; time-box 2 h).
> **Phạm vi**: **chỉ đo**. Không sửa code/docs chung, không push/merge/tag, không GitHub Actions.
> **Nguồn**: `main` = **`84915aa`** (`merge ST-289: version 0.3.6 + dong #111/#151 + khung R32`).

## 0. Mốc đo

| Mục | Giá trị |
|---|---|
| Commit đo | **`84915aa137b2f8754a933ecb42bd9b1de90bbee7`** |
| Phiên bản | **0.3.6** (`./target/debug/vis --version` → `vis 0.3.6`) |
| Nhánh QA | `plan/w3-platform` — merge `main` fast-forward lên **`84915aa`** (ST-288 đã ở main qua `545a38d`); không sửa gì thêm |
| Clone | `/home/ps/vis-qa-stage0` (clone từ `/home/ps/ViScript`, checkout `84915aa`) |
| Target | **trong clone** (`$PWD/target`), `CARGO_INCREMENTAL=0`, `debug=0` |
| Toolchain | `~/.cargo/bin/cargo` **1.100.0**, `rustc 1.100.0 (0ef1bc386 2026-11-09)` |
| Máy | Linux x86_64, 12 CPU; load average **48,92 / 27,44 / 26,33** lúc đo (các worktree khác build song song) |
| Đĩa | đầu **33 G**; cuối **16 G** (> 5 G); clone `.git` 56 M + `target` 1,5 G |

## 1. Fresh-clone `84915aa` — cổng

```bash
git clone /home/ps/ViScript /home/ps/vis-qa-stage0
cd /home/ps/vis-qa-stage0 && git checkout 84915aa
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
```

| Bước | Kết quả |
|---|---|
| `fmt` | **OK** |
| `clippy` | **OK** |
| luật R1..R6 | **OK** (R1: mọi tệp nguồn ≤ 1.500 dòng) |
| self-test case âm + fixtures | **OK** |
| `report-check` | **PASS** (32 báo cáo, 322 hàng QUEUE phủ đủ) |
| doc-examples | **OK** (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3) |
| toolchain / release | **PASS** |
| L0 `cargo-nextest` | **3576 passed / 3 skipped** (346 binary, 108,04 s) |
| **`gate --fast`** | **PASS** — wall **3:16,43** |
| Log | `/tmp/opencode/stage0-qa-gate.log` |

## 2. Các suite đích

Tất cả chạy trong clone, cùng target, **0 failed**:

| Suite | Lệnh | Kết quả | Thời gian test |
|---|---|---|---|
| **P2** | `cargo test -p vis-cli --test vs3_worker_eval_parallel` | **14/14** | 1,79 s |
| **P1** | `cargo test -p vis-cli --test vs3_worker_eval_local` | **8/8** | 0,45 s |
| **P3** | `cargo test -p vis-cli --test vs3_worker_eval_vm` | **10/10** | 3,82 s |
| h2 mặc định | `cargo test -p vis-cli --test vs3_http2` | **19/19** | 15,57 s |
| `vs3_default` | `cargo test -p vis-cli --test vs3_default` | **22/22** | 1,94 s |
| `vs3_tls` | `cargo test -p vis-cli --test vs3_tls` | **21/21** | 6,21 s |
| `pkg` | `cargo test -p vis-cli --test pkg` | **25/25** | 0,05 s |
| `vs3_registry_sig` | `cargo test -p vis-cli --features pkg-sig --test vs3_registry_sig` | **10/10** | 0,06 s |
| host | `./bin/vis-host-suite` | **54/54** + `host_posix_real` PASS | — |
| h2 feature | `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **34 passed / 1 ignored** | 15,96 s |
| phiên bản | `./target/debug/vis --version` | **`vis 0.3.6`** | — |

Log gộp: `/tmp/opencode/stage0-qa-suites.log`.

## 3. Differential 100k — **seed mới**

```bash
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221545 ./bin/vis-diff
```

| Chỉ số | Giá trị |
|---|---|
| Seed | **202611221545** (mới, không trùng `20261115`/`202611221318` của các lần trước) |
| Tổng | **100.000** |
| Lỗi tĩnh | 6.228 |
| Chạy | 93.772 |
| **Lệch interp↔VM** | **0** (`DIFF … lech=0`) |
| Thời gian | 136,8 s test / **139 s wall** (731 chương trình/s) |
| Log | `/tmp/opencode/stage0-qa-diff.log` |

Dòng `DIFF` ghi nhận nguyên văn:

```
DIFF tong=2000 loi_tinh=126 chay=1874 lech=0 giay=1.2 toc_do=1695/s seed=20261115
DIFF tong=100000 loi_tinh=6228 chay=93772 lech=0 giay=136.8 toc_do=731/s seed=202611221545
```

## 4. Đánh giá

- Fresh-clone **`84915aa`** khớp **toàn bộ** kỳ vọng Stage 0: gate **3576/3**, P2 **14**, P1 **8**,
  P3 **10**, h2 mặc định **19**, h2 feature **34+1**, `vs3_default` **22**, `vs3_tls` **21**,
  `pkg` **25**, `vs3_registry_sig` **10**, host **54/54**, `vis --version` **0.3.6**.
- Diff 100k seed **202611221545**: **lệch 0**.
- Không thấy flake/T-INFRA; chạy dưới tải cao (load ~49) vẫn xanh.

## 5. Blocker / ghi chú

1. **Không có blocker** local; không có suite đỏ.
2. `vs3_registry_sig` cần `--features pkg-sig` (mặc định tệp rỗng theo `cfg`) — đúng như VS3-5b/5c.
3. Windows/`psvn` và macOS **không đo** trong phiên này (ngoài phạm vi Stage 0 QA local; macOS ngoài
   phạm vi dự án).
4. Không sửa code/docs chung; chỉ thêm journal này. Không tag/push/merge.
5. Clone `/home/ps/vis-qa-stage0` giữ lại làm bằng chứng (1,5 G); đĩa còn **16 G** (> 5 G).

## 6. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
git clone /home/ps/ViScript /home/ps/vis-qa-stage0
cd /home/ps/vis-qa-stage0 && git checkout 84915aa
export CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
./vis-dev gate --fast
cargo test -p vis-cli --test vs3_worker_eval_parallel   # 14
cargo test -p vis-cli --test vs3_worker_eval_local      # 8
cargo test -p vis-cli --test vs3_worker_eval_vm         # 10
cargo test -p vis-cli --test vs3_http2                  # 19
cargo test -p vis-cli --test vs3_default                # 22
cargo test -p vis-cli --test vs3_tls                    # 21
cargo test -p vis-cli --test pkg                        # 25
cargo test -p vis-cli --features pkg-sig --test vs3_registry_sig  # 10
./bin/vis-host-suite                                    # 54/54
cargo test -p vis-cli --features async-h2 --test vs3_http2        # 34 + 1 ignored
./target/debug/vis --version                            # vis 0.3.6
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611221545 ./bin/vis-diff       # lech=0
```
