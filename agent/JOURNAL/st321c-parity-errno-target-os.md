<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-321c — normalize errno `ECONNREFUSED` theo `target_os` (test-only)

- **Nhánh**: `plan/st321c`, nền `bbbb99be` (`[REVIEW] merge ST-320a`); worktree
  `/home/ps/viswt/st321c`; máy `ps` (Linux), toolchain rustc/cargo **1.100.0** (phải đưa
  `/home/ps/.cargo/bin` vào `PATH` — `/usr/bin/rustc` hệ thống là 1.93.1, bị `rust-version`
  từ chối).
- **Phạm vi**: **duy nhất** `crates/vis-conformance/tests/misc_api_it/host_parity/net_http.rs`.
  **0 chạm runtime/compiler/VM**, 0 golden khác, 0 tệp test cấp 1 mới (R8 giữ nguyên),
  không `#[ignore]`, không bỏ ca, không đổi 18 assert phía trên.

## 1. Nguyên nhân (audit READ-ONLY)

Audit `/home/ps/vislogs/mps-gate-fail-20261127T011249Z.md` (log nguồn
`peak1-20261127T0100Z/mps-gate-full.log`): chain peak1 FAIL ở `mps` (macOS 27.0.1 /
Darwin 27 arm64) tại `net_loopback_parity`, dòng 148. Khối `#[cfg(unix)]` hard-code
`Connection refused (os error 111)` (= `ECONNREFUSED` **Linux**), trong khi macOS/BSD trả
**61**. `left`/`right` lệch đúng 2 dòng `connect=`/`connect_timeout=`; 18 dòng đầu khớp
hoàn toàn. Cả **hai engine interp lẫn VM** đều in `os error 61` ⇒ **parity interp↔VM nguyên
vẹn**; đây là lỗi portability của **hằng số kỳ vọng trong test**, không phải lỗi engine,
không phải flake (deterministic, lặp 2 lần trong log). Cùng HEAD chạy `ps` xanh (111),
`mps` đỏ (61).

## 2. Cách sửa

Trong `#[cfg(unix)]`, chọn errno kỳ vọng theo đích: `#[cfg(target_os = "linux")]` → **111**,
`#[cfg(not(target_os = "linux"))]` → **61** (bao phủ macOS + các BSD). Dùng `format!` để
dựng đúng 2 chuỗi kỳ vọng, vẫn so **từng byte** `&dong[18..]` như cũ. Không nới assert,
không thay `chuan_hoa_cong` (đang dùng cho cổng ephemeral) — parity interp↔VM vẫn đi qua
`hai_duong_chuan_hoa` và so byte-exact sau khi chuẩn hoá cổng.

Không tái dùng số nợ `#202`: `#202` là flake ms của `http_bon_ham_parity`, đã đóng ở
`ST-318b`. Đây là finding portability mới — **cần A0 mở số nợ mới** khi nghiệm thu.

## 3. Cổng (máy `ps`, Linux)

| Mục | Kết quả | Log / sha256 |
|---|---|---|
| `cargo build -p vis-cli` | OK (1m11s) | `st321c-test.log` |
| `cargo test -p vis-conformance --test lib_api_it -- net_loopback_parity --nocapture` | **PASS** 1/1 (0,21 s) | `/home/ps/vislogs/st321c-test.log`, `f0fd41a1…670df8` |
| `./vis-dev gate --fast` | **PASS** — **3937/3937** passed, 4 skipped (check + L0 + doc-test) | `/home/ps/vislogs/st321c-gate-fast.log`, `23da8b3d…4bcc3251` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** (0 warning) | `/home/ps/vislogs/st321c-clippy.log`, `4663e705…f208f1a` |
| `vis-diff 100k` | **không chạy** — lát chỉ sửa hằng số kỳ vọng trong test, không chạm semantics/compiler/runtime/VM | — |

Môi trường: `PATH=/home/ps/.cargo/bin:$PATH`, `TMPDIR=/home/ps/vislogs/st321c-tmp` (đĩa,
không dùng tmpfs 5,1 G), `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_BUILD_JOBS=2`.

## 4. Còn lại / chuyển giao

- **KHÔNG SSH sang `mps`** trong phiên này. Bắt buộc **re-run `mps-gate-full` sau khi merge**
  để xác nhận macOS PASS (kỳ vọng dòng `connect=`/`connect_timeout=` in `os error 61` và khớp
  nhánh `not(target_os = "linux")`). Chưa có bằng chứng macOS xanh ⇒ **không** tuyên bố
  `ST-316c` đạt 3-OS.
- `psvn` FAIL riêng (`unused import: std::process::Command`, `chain_exit=101`) — **ngoài lane
  này**, cần xử lý riêng nếu muốn mở lại `ST-316c`.
- A0 cập nhật sổ nợ (số mới, không dùng `#202`) và hàng `QUEUE.md`/`ACTIVE.md` khi nghiệm thu;
  Builder không sửa các tệp điều khiển của A0. Không merge, không push.
- **Cảnh báo đĩa**: sau build, `df` còn **5,3 G** (< ngưỡng dừng 6 G) → dừng mọi việc nặng
  tiếp theo (đã bỏ `vis-diff 100k`); chỉ thực hiện journal + commit nhẹ trong phiên này.
