<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-325e — sửa 9 link intra-doc hỏng trong `vis-runtime`

- **Nhánh**: `plan/st325e`, nền `main` = `6e8eddc7`; worktree `/home/ps/viswt/st325e`.
  **Không** đụng repo chính, **không** merge/push (VD-279).
- **Lane**: `crates/vis-runtime/src/rt/**` — doc comment only: `rt/sched/mod.rs` dòng 7–9
  và `rt/select.rs` dòng 16. **Không** đổi code/API, **không** dùng `#![allow(...)]`,
  **không** thêm dependency (giữ DAG 5 tầng), **không** chạy `vis-diff` (doc-only).
- **Môi trường**: cargo/rustc/rustdoc 1.100.0 (`~/.cargo/bin` trên PATH + `rust-toolchain.toml`),
  `CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/st325e-tmp` (đĩa, không tmpfs).
  Đĩa đầu **23 G**, cuối **19 G** (> ngưỡng dừng 5 G).

## 1. Đỏ-trước (reproduce trong worktree)

`RUSTDOCFLAGS="-D warnings" cargo doc -p vis-runtime --no-deps -j 2`
(`st325e-doc-before.log`) cho **rc 101** với **đúng 9 lỗi**, khớp ST-325c:

| # | Vị trí | Item/lý do |
|---|---|---|
| 1 | `rt/sched/mod.rs:7:21` | `scheduler` — private item |
| 2 | `rt/sched/mod.rs:7:72` | `frame_api` — private item |
| 3 | `rt/sched/mod.rs:8:17` | `frame_next` — private item |
| 4 | `rt/sched/mod.rs:8:54` | `frame_su_kien` — private item |
| 5 | `rt/sched/mod.rs:9:7` | `frame_ket_thuc` — private item |
| 6 | `rt/sched/mod.rs:9:51` | `frame_huy` — private item |
| 7 | `rt/sched/mod.rs:9:80` | `frame_reactor` — private item |
| 8 | `rt/select.rs:16:9` | `don_dep` — unresolved (không tồn tại; hàm thật là `don_dep_kenh`, `rt/select.rs:486`) |
| 9 | `rt/select.rs:16:21` | `huy_frame` — private item (`pub(crate)`, `rt/select.rs:503`) |

Trường hợp 1–7 và 9 là module/hàm private bị link từ doc công khai; trường hợp 8 là link tới
tên không có thật. Đều là nợ tài liệu, không phải lỗi API.

## 2. Patch (doc-only, 4 dòng / 2 tệp)

Đổi 9 link intra-doc thành **code span**: 7 module private trong `sched/mod.rs` bỏ `[`…`]`;
`[`don_dep`]` → `` `don_dep_kenh` `` (tên hàm có thật); `[`huy_frame`]` → `` `huy_frame` ``.
Văn bản mô tả, hành vi, chữ ký và `pub use` giữ nguyên. `cargo fmt --all` không đổi thêm gì
(diff vẫn 4 dòng). Log `st325e-fmt.log` rỗng.

## 3. Cổng (log `/home/ps/vislogs/st325e-*.log`)

| Cổng | Kết quả | sha256 log |
|---|---|---|
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-runtime --no-deps` (trước) | **rc 101**, 9 link hỏng | `f9be0bf7…6590` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-runtime --no-deps` (sau) | **rc 0**, **0 warning / 0 error**, sinh `target/doc/vis_runtime/index.html` | `d4c2c04f…026f` |
| `cargo fmt --all` | **rc 0**, không reformat ngoài lane (log rỗng) | `e3b0c442…b855` |
| `./vis-dev gate --fast` | **rc 0 PASS** — luật R1..R10 OK, DAG 23 crate/5 tầng, self-test 20/20, doc-examples OK, L0 **3949/3949 passed, 4 skipped** | `74459eff…bb12b` |

Không chạy `vis-diff` (doc-only). Không link hỏng nào ngoài 9 đã liệt kê ⇒ không mở rộng lane.

## 4. Kết luận

Lane doc-only hoàn tất, mọi cổng xanh. Commit `[ST-325e]` trên `plan/st325e`;
**không** merge/push, chờ Gatekeeper/Agent 0.
