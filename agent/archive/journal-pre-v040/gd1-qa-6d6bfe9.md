<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# QA GĐ1 (ST-296 partial) — fresh-clone `6d6bfe9`

> **Người chạy**: W3 (Gatekeeper QA), 2026-11-22 20:54–21:50 JST (~56 phút; time-box 2 h).
> **Phạm vi**: **chỉ đo**. Không sửa code, không push/merge/tag, không GitHub Actions.
> **Mốc**: `main` **`6d6bfe9`** (ST-290/293/294/295; gate **3638/4**).

## 0. Mốc đo

| Mục | Giá trị |
|---|---|
| Commit đo | **`6d6bfe9c249de74e956567554ccbcc0bd1cc95c3`** |
| Phiên bản | **0.3.6** (`debug/vis --version`) |
| Nhánh QA | `plan/w3-platform` — merge `main` fast-forward lên `6d6bfe9`; không sửa gì thêm |
| Clone | `/home/ps/vis-qa-gd1` (clone `/home/ps/ViScript`, checkout `6d6bfe9`) |
| Target | clone: `$PWD/target`; soak: `target/soak1`, `target/soak2` — riêng |
| TMPDIR | **`/home/ps/tmp`** (tránh tmpfs `/tmp` như sự cố soak trước) |
| Toolchain | cargo/rustc **1.100.0**; Linux x86_64 12 CPU |
| Đĩa | đầu 21 G; cuối **12 G** (> 10 G yêu cầu); clone/target 5,9 G |

## 1. Fresh-clone `6d6bfe9` — gate

```bash
git clone /home/ps/ViScript /home/ps/vis-qa-gd1 && cd /home/ps/vis-qa-gd1
git checkout 6d6bfe9
export TMPDIR=/home/ps/tmp CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0
./vis-dev gate --fast
```

| Bước | Kết quả |
|---|---|
| fmt / clippy / R1..R6 | **OK** |
| self-test + fixtures + doc-examples | **OK** (24 khối; 12 fixture QD) |
| report-check | **PASS** (32 báo cáo, 327 hàng QUEUE phủ đủ) |
| L0 nextest | **3638 passed (2 slow) / 4 skipped** (97,46 s) |
| **gate --fast** | **PASS** — wall **3:24,33** |
| Log | `/tmp/opencode/gd1-gate.log` |

## 2. Release workspace — **ĐỎ 8 ca test-only**

```bash
cargo test --release --workspace --no-fail-fast
```

| Chỉ số | Giá trị |
|---|---|
| Binary test | **358** |
| **passed** | **3635** |
| **failed** | **8** |
| **ignored** | **4** |
| Thời gian | wall **6:44,12** |
| Log | `/tmp/opencode/gd1-release.log` |

8 ca đỏ đều thuộc **`vs3_http2::h2_async_spike`** (release):
`async_h2_production_cap_body`, `async_h2_production_bon_stream_cold`,
`async_h2_cold_multiplex_20_vong`, `async_h2_production_redirect`,
`async_h2_production_timeout_va_goaway`, `async_h2_production_bon_stream_mot_connection`,
`async_h2_soak_env_gated`, `async_h2_production_cookie_redirect`.

- Lỗi chung: `lỗi TLS handshake tới 127.0.0.1:<port>: invalid peer certificate: UnknownIssuer`.
- Nguyên nhân **test harness/release**, không phải core runtime: các test này chạy binary `vis`
  con với `VIS_TLS_TEST_ROOT_DER`; hook đọc biến này **chỉ biên dịch khi `debug_assertions`**
  (`crates/vis-cli/src/net/tls.rs:51-53`), nên binary **release** bỏ qua root fixture ⇒ TLS
  self-signed không tin được. Các ca còn lại (kể cả `vs3_http_cancel`, `vs3_select`, h2 sync)
  đều xanh.
- **Blocker QA**: release-workspace chưa xanh vì 8 ca fixture TLS debug-only này; cần W2/W4
  quyết định release-gate (bật hook test ở release có kiểm soát, hoặc `#[ignore]`/`cfg` riêng
  cho `h2_async_spike` ở release). Không sửa code trong phiên này.

## 3. Targeted (debug) — xanh toàn bộ

| Suite | Kết quả |
|---|---|
| `vs3_select` | **25/25** |
| P2 `vs3_worker_eval_parallel` | **14/14** |
| P1 `vs3_worker_eval_local` | **8/8** |
| P3 `vs3_worker_eval_vm` | **10/10** |
| `vs3_http2` | **34 passed / 1 ignored** (35 tổng) |
| `vs3_http_cancel` | **8/8** |
| `pkg` | **25/25** |
| host | **54/54** + `host_posix_real` PASS |
| `debug/vis --version` | **`vis 0.3.6`** |

Log: `/tmp/opencode/gd1-suites.log`.

## 4. Hai soak 10/10 (`6d6bfe9`)

Mỗi lần: 3× `gate --fast`, 3× diff 100k seed `20261115`, 2× host, 2× cache; target riêng,
`TMPDIR=/home/ps/tmp`:

| Lần | Tổng | Kết quả | Gate | Diff 100k | Host | Cache | Log |
|---|---:|---|---|---|---|---|---|
| 1 | **999 s** (wall 16:38) | **10/10 OK** | 3× **3638/4** | 3× **lech=0** | 2× **54/54** | 2× **5/5** | `/tmp/opencode/gd1-soak1.log` |
| 2 | **1027 s** (wall 17:06) | **10/10 OK** | 3× **3638/4** | 3× **lech=0** | 2× **54/54** | 2× **5/5** | `/tmp/opencode/gd1-soak2.log` |

Không vòng đỏ, không retry, không T-INFRA; load 6–14. Tổng **20/20 vòng**, **2026 s**.

## 5. Differential 100k — seed mới

```bash
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611222200 ./bin/vis-diff
```

| Chỉ số | Giá trị |
|---|---|
| Seed | **202611222200** (mới) |
| Tổng | **100.000** |
| Lỗi tĩnh | 6.233 |
| Chạy | 93.767 |
| **Lệch** | **0** (`DIFF … lech=0`) |
| Thời gian | 188,1 s test / **194 s wall** |
| Log | `/tmp/opencode/gd1-diff.log` |

## 6. Kết luận / blocker

- Gate, targeted debug, 2× soak và diff đều **xanh**; `vs3_select` 25/25, cancel 8/8.
- **Blocker duy nhất**: release workspace **3635 passed / 8 failed** do 8 ca
  `h2_async_spike` dùng root TLS fixture qua env **chỉ có ở debug** (`net/tls.rs:51-53`);
  đây là **test harness release-mode**, không phải lỗi sản phẩm đã thấy trong gate/debug.
- Đề xuất: W2/W4 gate các ca `h2_async_spike` theo `debug_assertions`/feature test-root, hoặc
  cung cấp hook root test an toàn cho release; sau đó chạy lại release workspace.
- Đĩa cuối **12 G** (> 10 G). Không sửa code; không tag/push/merge; không GitHub Actions.
