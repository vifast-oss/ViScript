<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-321d — cfg-gate import `Command` trong `vistest/tests.rs` cho Windows

- **Ngày**: 2026-11-27 (giờ thấp điểm) · **Vai**: Builder (Agent 2)
- **Nhánh**: `plan/st321d` · **Worktree**: `/home/ps/viswt/st321d`
- **Nền**: `main` @ `bbbb99be` (`[REVIEW] merge ST-320a`)
- **Trạng thái**: fix 1 dòng; gate Linux xanh. **Cần re-run `psvn-gate-full` sau merge** — chưa tự nghiệm thu Windows.

## 0. TL;DR

Chain peak1 `psvn-gate-full` (ST-316c, HEAD `ff585af8`) fail `exit=101`:
`error: unused import: std::process::Command` tại `crates\vis-cli\src\vistest\tests.rs:5`
với `cargo clippy --workspace --all-targets -- -D warnings` trên **psvn (MINGW64)**.

Nguyên nhân: `use std::process::Command;` (dòng 5) **chỉ** được dùng bởi hàm `sh()` —
mà hàm này đã bị `#[cfg(unix)]` (ST-139/Agent 0: `/bin/sh` không có trên Windows).
Trên Windows hàm `sh()` bị loại ⇒ `Command` thành unused ⇒ `-D warnings` đỏ.

Sửa tối thiểu: gate chính import bằng `#[cfg(unix)]`. Không xoá hẳn (vẫn cần trên Unix).

## 1. Điều tra

`grep -n '\bCommand\b' crates/vis-cli/src/vistest/`:

| Tệp | Dùng `Command` | Có `#[cfg]`? |
|---|---|---|
| `tests.rs:5,34,35` | import + `sh()` + `Command::new("/bin/sh")` | `sh()` **có** `#[cfg(unix)]` ⇒ import thừa trên Windows |
| `ca.rs:6,64` | import + `Command::new(current_exe())` | không — dùng mọi nền, xanh |
| `tien_trinh.rs:6,25,37,62,131` | import + `chay_co_han`/`dat_path_tro_giup` | không — dùng mọi nền, xanh |

⇒ chỉ `tests.rs` cần vá. `run.rs` không import `Command`.

## 2. Patch

```diff
 use std::path::{Path, PathBuf};
+#[cfg(unix)]
 use std::process::Command;
```

Một dòng, không đổi hành vi Unix (`sh()` + 2 ca test `#[cfg(unix)]` vẫn biên dịch/chạy),
loại bỏ unused-import trên Windows.

## 3. Gate Linux (đều PASS)

Chạy trong worktree, `PATH=/home/ps/.cargo/bin:$PATH` (rustup pin **1.100.0**; `/usr/bin/cargo`
là 1.93.1 ⇒ bị từ chối MSRV), `CARGO_BUILD_JOBS=2`,
`TMPDIR=/home/ps/tmp`. Log: `/home/ps/vislogs/st321d-20261127T1014Z-*.log`.

| Lệnh | RC | Ghi chú |
|---|---|---|
| `cargo fmt --all` | 0 | không đổi tệp ngoài patch |
| `cargo test -p vis-cli` | 0 | unit **287/0** + `worker_it` **3/3** + doc-tests 0 |
| `./vis-dev gate --fast` | 0 | `PASS (L1 chưa có tới khi có sim ViScript)`; L0 OK |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | `Finished dev ... 1m 04s`, không warning |

## 4. Điều kiện nghiệm thu / chuyển giao

- **Không SSH sang psvn**, không merge/push — Builder only.
- **Bắt buộc**: sau khi merge nhánh này vào `main` và đồng bộ sang `psvn`, chạy lại
  `bin/vis-gate-windows --full` (`psvn-gate-full`) để xác nhận clippy `-D warnings` xanh
  trên MINGW64. Log fail gốc: `/home/ps/vislogs/peak1-20261127T0100Z/psvn-gate-full.log`.
- Disk lúc chạy: `/` 13 G available (> ngưỡng dừng 6 G); tmpfs `/tmp` 5.1 G — đã dùng
  `TMPDIR` đĩa `/home/ps/tmp` để tránh tràn tmpfs.
