<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-311e — Windows clippy: `JobControl.next` dead_code (chặn soak 4 h)

## 1. Phát hiện

Soak Windows trên `H = 478aa9f` (`bin/vis-soak` → `gate-1` → `./vis-dev check`) đỏ **16 s**:

```
error: field `next` is never read
  --> crates\vish-edit\src\job.rs:46:5
   = note: `-D dead-code` implied by `-D warnings`
--- clippy: FAIL
soak: DỪNG ở gate-1
```

`vis-gate-windows --full` **không** chạy clippy (chỉ `cargo test` + host + targets), nên lỗi này
lọt qua 3× full; soak mới là bước bắt được.

## 2. Nguyên nhân

`JobControl::next` chỉ được đọc trong khối `#[cfg(unix)]` của `spawn()`. Trên Windows khối
`#[cfg(not(unix))]` trả lỗi, nên `next` không có reader ⇒ `dead_code` dưới `-D warnings`.

## 3. Vá

`crates/vish-edit/src/job.rs`:

```rust
#[cfg_attr(not(unix), allow(dead_code))]
next: u32,
```

Không đổi hành vi, không đổi API, không đổi layout trên Unix; chỉ tắt `dead_code` trên nền
không-Unix. `cargo clippy -p vish-edit --all-targets -- -D warnings` xanh trên Linux;
`cargo fmt --check` xanh.

## 4. Hệ quả cho Phase C

`H` **không còn là commit nghiệm thu** cho soak Windows. Commit vá là **`H2`** (ST-311e) rồi **`H3`** (hai vá test); ma trận
3-OS + watchdog + soak phải chạy lại trên `H3`. Các lần chạy parser fuzz đang chạy dùng binary
`vis-syntax` **không đổi** giữa `H` và `H2`; ghi rõ nguồn gốc khi tổng hợp R37.

## 5. Bổ sung cùng đợt (H3)

Chạy `cargo clippy --workspace --all-targets -- -D warnings` trên `psvn` sau H2 lộ thêm hai lỗi
Windows-only (đều là **test**, không phải sản phẩm):

- `crates/vish-edit/tests/completion_job.rs`: `JobControl` chỉ dùng trong 2 test `#[cfg(unix)]`
  ⇒ tách `#[cfg(unix)] use vish_edit::job::JobControl;`.
- `crates/vis-host-native/tests/security_it/st309.rs`: toàn bộ ca là Unix/Linux (symlink +
  Landlock) ⇒ thêm `#![cfg(unix)]` ở đầu module để Windows không còn helper dead_code.

## 6. Việc tiếp


1. Push `H3`; `git checkout -f H3` trên `psvn`/`mps`/clone QA.
2. Chạy lại 3× full Windows/macOS, Linux gate/release/diff/WASI/host/verifier trên `H3`.
3. Restart watchdog 2 h trên `H3`; chạy lại soak 4 h Windows/macOS và soak 24 h Linux trên `H3`.
4. R37 ghi `head = H3`; tag trên commit R37 (docs) như quy ước.
