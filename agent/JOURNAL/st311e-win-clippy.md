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

`H` **không còn là commit nghiệm thu** cho soak Windows. Commit vá là **`H2`** (ST-311e); ma trận
3-OS + watchdog + soak phải chạy lại trên `H2`. Các lần chạy parser fuzz đang chạy dùng binary
`vis-syntax` **không đổi** giữa `H` và `H2`; ghi rõ nguồn gốc khi tổng hợp R37.

## 5. Việc tiếp

1. Push `H2`; `git checkout -f H2` trên `psvn`/`mps`/clone QA.
2. Chạy lại 3× full Windows/macOS, Linux gate/release/diff/WASI/host/verifier trên `H2`.
3. Restart watchdog 2 h trên `H2`; chạy lại soak 4 h Windows/macOS và soak 24 h Linux trên `H2`.
4. R37 ghi `head = H2`; tag trên commit R37 (docs) như quy ước.
