<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-324b — sửa link intra-doc `Scheduler::…` trong `vis-http`

- **Nhánh**: `plan/st324b`, nền `main` = `d1077075`; worktree `/home/ps/viswt/st324b`.
  **Không** đụng repo chính, **không** merge/push (VD-279).
- **Lane**: `crates/vis-http/src/async_http.rs` dòng ~5 (doc-only). **Không** thêm dependency
  `vis-runtime` (giữ DAG 5 tầng), **không** đụng code/API, **không** chạy `vis-diff`.
- **Môi trường**: cargo/rustdoc 1.100.0 (`~/.cargo/bin` trên PATH, `rust-toolchain.toml`),
  `CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/tmp-st324b` (đĩa, không tmpfs).
  Đĩa đầu **29 G**, cuối **24 G** (> ngưỡng dừng 5 G).
- **Kết quả**: `cargo doc -p vis-http --no-deps` với `-D warnings` từ **rc 101** → **rc 0,
  0 warning**; `vis-dev gate --fast` **PASS** (3948/3948); clippy `-D warnings` **rc 0**.

## 1. Đỏ-trước (xác lập đúng lane)

`RUSTDOCFLAGS="-D warnings" cargo doc -p vis-http --no-deps` (`st324b-doc-before.log`)
cho **đúng 2 lỗi**, đều ở dòng 5 — không có link hỏng nào khác ngoài lane:

```
error: unresolved link to `Scheduler::frame_reactor_poll_io`  --> async_http.rs:5:7
error: unresolved link to `Scheduler::frame_reactor_poll_bytes` --> async_http.rs:5:44
```

Lý do: `Scheduler` không là item trong scope của crate `vis-http` (`vis-http` không biết
`vis-interp`/`vis-vm`; tầng trên cài trait `AsyncHttpEnv`). Đây là nợ tài liệu, không phải lỗi API.

## 2. Patch (doc-only, 5 dòng)

Thay 2 link intra-doc hỏng bằng **code span**, đồng thời trỏ tới item có thật
`[`FrameReactorBackend`]` (re-export tại `crates/vis-http/src/lib.rs:52` từ `vis_core::reactor`,
và đã `use crate::{…, FrameReactorBackend, …}` trong module):

```rust
//! `Scheduler::frame_reactor_poll_io`/`Scheduler::frame_reactor_poll_bytes` (tầng trên cài qua
//! [`FrameReactorBackend`]) nên khi kernel chưa sẵn sàng thì frame trả `Abort::Pending` và
//! scheduler chạy tác vụ khác. Không gọi `NetHost::connect/read/write` blocking giữa request.
//! TLS dùng handle `T:<addr>` do `AsyncNet` (CLI) cài; HTTP/1.1 trước, HTTP/2 async còn mở.
```

`cargo fmt --all` không đổi gì thêm ngoài đoạn sửa (diff 5 dòng). Không sửa `Cargo.toml`,
không thêm dependency, không đổi chữ ký/`pub use`.

## 3. Cổng (log `/home/ps/vislogs/st324b-*.log`)

| Cổng | Kết quả | sha256 log |
|---|---|---|
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-http --no-deps` (trước) | **rc 101**, 2 link hỏng | `4621b0f5…b04b` |
| `cargo fmt --all` | **rc 0**, không reformat ngoài lane | `e3b0c442…b855` (rỗng) |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-http --no-deps` (sau) | **rc 0**, **0 warning**, sinh `target/doc/vis_http/index.html` | `ff8ef488…88db` |
| `./vis-dev gate --fast` | **rc 0 PASS** — check + L0 **3948/3948 passed, 4 skipped**, doc-test OK | `7472663a…1787` |
| `cargo clippy -p vis-http --all-targets -- -D warnings` | **rc 0**, 0 warning | `f167e111…3353f` |

Không chạy `vis-diff` (doc-only). Không link hỏng ngoài dòng 5 ⇒ không mở rộng lane.

## 4. Kết luận

Lane doc-only hoàn tất, tất cả cổng xanh. Commit `[ST-324b]` trên `plan/st324b`;
**không** merge/push, chờ Gatekeeper/Agent 0.
