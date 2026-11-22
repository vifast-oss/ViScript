<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296 pre-QA — fix `vd270_test_slowest`, `TMPDIR` cho `bin/vis-soak`, nightly & #191

> **Người làm**: subagent VS3-W4, 2026-11-22 · **VD-281(2)** · time-box 2,5 h.
> **Baseline**: `main` **`f6939721475b064726a11733ed2a7efbe76962ad`** (`merge ST-289: Windows soak
> 4h tag v0.3.6 1361 vong`); làm trên `plan/w4-qa-docs`, target riêng của worktree,
> `TMPDIR=/home/ps/tmp`. Không tag/push/merge; không GitHub Actions.
> **Kết quả**: gate/nightly **PASS** (3638/4), #191 `cargo nextest` trên `psvn`
> **3583/3583 passed, 3 skipped, 0 fail/flake**. Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `vd270_test_slowest` | nới ca chậm **50 → 300 ms**, ngưỡng **40 → 250 ms**; **không `#[ignore]`** |
| Tải ≥20 | **3/3 lần pass** ở load **21,75 / 26,02 / 26,02** (thêm 5 lần pass ở load 14,8–18,35) |
| `bin/vis-soak` | ép `TMPDIR/TMP/TEMP` xuống `target/vis-soak-tmp-<pid>`, dọn khi thoát; `VIS_SOAK_TMPDIR` đổi gốc |
| `./vis-dev nightly` | **PASS** — gate **3638 passed / 4 skipped**, fuzz 2 phút **9 717 129 ca / 0 vi phạm**, fmt 475 tệp bất động |
| #191 (Windows nextest) | **3583/3583 passed, 3 skipped**, 350 binaries, test 213,4 s / tổng 481 s, **0 fail/flake** |
| Gate / fmt / clippy | `gate --fast` PASS (trong nightly); fmt + clippy OK |
| Đĩa | local `/` còn **14 G** (>10 G); `psvn` `C:` còn **130 G** |

## 1. `vd270_test_slowest` — độ chịu tải

**Vấn đề**: ca chậm busy-loop ~50 ms, ca nhanh `assert true`; dưới tải cao, thời gian spawn/đo
của ca nhanh có thể tiến sát 50 ms ⇒ thứ tự `cham`/`nhanh` và ngưỡng 40 ms có thể lật, dù
`--slowest` vẫn đúng.

**Sửa** (`crates/vis-cli/tests/vd270_test_slowest.rs`, không `#[ignore]`):
- `CHAM`: busy loop `time.monotonic_ms()` **50 → 300 ms**.
- Ngưỡng ca chậm: **40 000 → 250 000 us** (2 chỗ: test 1 và test 2) — biên 50 ms dưới 300 ms.
- Giữ nguyên kiểm regression: `--slowest 1` phải là `cham`; `--slowest 2` phải giảm dần
  (`us0 >= us1`), `cham` trước `nhanh`, `us1 < us0`; repeat/shard/filter/json/compose không đổi.
- Cập nhật doc-comment nêu căn cứ (load ≥20: spawn ca nhanh vẫn « 250 ms).

**Đo dưới tải** (40–70 tiến trình `yes`, load average lấy từ `/proc/loadavg`):

| Lần | Load | Kết quả |
|---|---:|---|
| 3 lần (sau sửa) | **21,75 / 26,02 / 26,02** | **7/7 pass** mỗi lần (1,44–1,53 s) |
| 5 lần (sau sửa) | 14,80–18,35 | 7/7 pass mỗi lần (1,37–1,55 s) |

- Trước sửa, 5 lần ở load ~16 và 10 lần ở load ~19,4–25,3 cũng pass trên máy này — không tái hiện
  đỏ cục bộ, nhưng biên 50 ms quá mỏng theo VD-281; bản sửa tăng biên tuyệt đối mà vẫn giữ kiểm
  thứ tự/ngưỡng.

## 2. `bin/vis-soak` — `TMPDIR` ra ngoài `/tmp` tmpfs

Thêm sau bước kiểm ghi đè log / `--dry-run` (không đổi 10 vòng, không đổi log semantics):

```sh
SOAK_TMP_GOC="${VIS_SOAK_TMPDIR:-$PWD/target}"
SOAK_TMP="$SOAK_TMP_GOC/vis-soak-tmp-$$"
export TMPDIR="$SOAK_TMP" TMP="$SOAK_TMP" TEMP="$SOAK_TMP"
trap 'rm -rf "$SOAK_TMP"' EXIT INT TERM HUP
```

- Mặc định **dưới repo** (`target/vis-soak-tmp-<pid>`), tránh tmpfs `/tmp` 7,2 GB.
- `VIS_SOAK_TMPDIR` cho phép đổi gốc; luôn tạo thư mục con riêng và **dọn khi thoát**
  (kể cả lỗi/bị ngắt).
- `--dry-run` thoát trước khi tạo thư mục ⇒ không side effect; `sh -n` OK.
- Test hiện có: `cargo test -p vis-cli --test soak_script` **2/2 pass**.

## 3. Nightly (local, trên `f693972` + bản sửa)

`PATH=... TMPDIR=/home/ps/tmp CARGO_INCREMENTAL=0 ./vis-dev nightly`:

| Bước | Kết quả |
|---|---|
| `gate --fast` | **PASS** — **3638 passed / 0 failed / 4 skipped** (fmt, clippy, R1..R6, self-test, doc-examples, L0 nextest) |
| fuzz parser 2 phút | **PASS** — **9 717 129 ca / 220 594 221 token / 0 vi phạm** (80 976 ca/s) |
| vòng tròn fmt | **PASS** — 475 tệp bất động, bỏ qua 78 ca lỗi cú pháp |
| Kết luận | `vis-dev nightly: PASS` (exit 0) |

## 4. #191 — `cargo nextest` trên Windows (worktree riêng)

- `psvn`: worktree **riêng** `/c/Users/ps/viscript-w4n191-f693972` @ `f6939721`, dirty 0;
  `describe = v0.3.6-28-gf6939721`; không dùng repo chung `/c/Users/ps/viscript-moi`.
- `cargo nextest run --workspace --no-fail-fast`:
  - **350 binaries; 3583 tests run: 3583 passed (1 slow), 3 skipped**;
  - test **213,4 s**, tổng (gồm build) **481 s**; **rc 0**; **0 fail, 0 flake**.
  - Log: `/c/Users/ps/vislogs/n191-nextest.log`, sha256
    `412a4e43cd91565557b82f41d6092bf5a57026220283faa6a26e2bec31dfb47b`.
- So với finding #191 cũ (`dbae70a`: 1416 passed / 1 failed — `tcp_explain_tong`, WSAECONNABORTED),
  lần này **không còn ca đỏ**. Đây là **bằng chứng đo sớm**; việc đóng #191 vẫn thuộc A0/ST-296
  chính thức (chưa sửa sổ nợ trong phiên này).

## 5. Ghi chú / blocker

- Không sửa logic sản phẩm; chỉ test + script soak (0 đổi ABI/hành vi `vis`).
- `bin/vis-soak` chưa chạy full 10 vòng trong phiên này (không cần cho pre-QA; `--dry-run` +
  `soak_script` xanh, nightly/gate xanh).
- **Không blocker**. Đĩa local còn **14 G**, `psvn` còn **130 G**.
- Không tag/push/merge; không GitHub Actions.
