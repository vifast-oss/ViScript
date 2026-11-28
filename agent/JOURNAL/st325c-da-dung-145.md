<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-325c — repro `#145` `chan(0)`: chú thích đã đúng từ `ST-316b` (`journal: da dung`)

- **Nhánh** `plan/st325c`, nền `main` = `37ae9c35d998ea490d21642fdaca6e610a550c7b`,
  worktree `/home/ps/viswt/st325c`. **Không** đụng repo chính, **không** merge/push (VD-279).
- **Lane**: `crates/vis-runtime/src/rt.rs` (+ journal). **Không đổi code/runtime**, không
  chạy `vis-diff` (không có thay đổi mã sản phẩm).
- **Môi trường**: cargo/rustdoc/rustfmt **1.100.0** (`/home/ps/.cargo/bin` trên PATH,
  `RUSTUP_TOOLCHAIN=1.100.0`, khớp `rust-toolchain.toml`), `CARGO_BUILD_JOBS=2`,
  `TMPDIR=/home/ps/vislogs/st325c-tmp` (đĩa, không tmpfs). Đĩa đầu **29 G**, cuối **24 G**
  (> ngưỡng dừng 5 G). Log `/home/ps/vislogs/st325c-{build,repro,doc,fmt,gate}.log`.

## 1. Repro (binary debug dựng ngay trong worktree)

`cargo build -p vis-cli --bin vis` ⇒ **rc 0** (`st325c-build.log`,
`sha256 4e833dfd…1ee28`). Chạy `target/debug/vis` trên `fn main(): let c = chan[int](0)`:

| Lệnh | Kết quả | Exit |
|---|---|---|
| `vis check c0.vis` | TAP `ok 1 - c0.vis`, **không** có `VS0505` | **0** |
| `vis run c0.vis` (interp) | `error VS0505: chan[T](0): kênh capacity 0 (rendezvous) … dùng capacity ≥ 1` | **101** |
| `vis run --vm --no-cache c0.vis` | cùng `VS0505`, byte y hệt interp | **101** |

Log: `st325c-repro.log`, `sha256 73a25e30…b2e04de`.

**Tầng chặn thật**: tyck **không** kiểm `cap` (`check` xanh), **runtime** từ chối bằng
`VS0505` ở cả hai engine (lỗi chạy có mã ⇒ exit 101 theo R9/C1). Chú thích cũ nói "tyck
chặn, exit 2" là **sai**; mã đúng là `VS0505` (không phải `VS0501`).

## 2. Đã đúng — không sửa nguồn

- `crates/vis-runtime/src/rt.rs` hiện **28 dòng**, một commit duy nhất `aa816e46` (`ST-313a`):
  đây là module re-export, **không có** dòng 128-129 và **không có** chú thích `chan(0)`/`tyck`
  nào để đính chính. Đường dẫn trong sổ nợ `#145` (`rt.rs:128-129`) là **đường cũ trước ST-313a**.
- Chú thích bị nợ đã được `ST-316b` (`da5590e3`, **ancestor của nền này**) sửa tại
  `crates/vis-runtime/src/rt/ops.rs:43-47`; nội dung hiện tại nói đúng sự thật đã repro ở §1:
  tyck không kiểm `cap`, runtime `VS0505` exit 101, riêng đường frame cho phép rendezvous.
  Audit độc lập `agent/JOURNAL/review-st316b.md` §5 **PASS**, A0 đóng `#145` ở VD-293.
- `rt/ops.rs` nằm **ngoài lane** và đã đúng ⇒ theo nguyên tắc "nếu comment cũ vẫn đúng ⇒
  KHÔNG sửa", **không sửa tệp nguồn nào**. `git status` sạch trước khi commit journal.
- **Việc cần A0**: dòng `#145` trong `SO-NO.md:233` đã lỗi thời (đóng ở ST-316b, đường dẫn
  cũ `rt.rs:128-129`). Cần A0 cập nhật sổ; Agent 2 không đụng `SO-NO.md` (ngoài lane).

## 3. Cổng (log `/home/ps/vislogs/st325c-*.log`)

| Kiểm | Kết quả | sha256 log |
|---|---|---|
| `cargo build -p vis-cli --bin vis` | **rc 0** | `4e833dfd…1ee28` |
| Repro `check`/`run`/`run --vm` | tyck 0, runtime 101/101 (`VS0505`) | `73a25e30…b2e04de` |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p vis-runtime --no-deps` | **rc 101 — KHÔNG đạt KPI** | `65398113…1a5a8` |
| `cargo fmt --all` | **rc 0**, không đổi tệp nào | `e3b0c442…b855` (rỗng) |
| `./vis-dev gate --fast` | **rc 0 PASS** (check + L0, doc-test OK) | `ac6a1750…f01b20d` |

### `cargo doc` đỏ — nợ có sẵn, ngoài lane (không phải do lát này)

9 lỗi `private_intra_doc_links`/`broken_intra_doc_links`, **không** liên quan `#145` và có sẵn
trên nền `37ae9c35` (lát này không đổi mã):

- `crates/vis-runtime/src/rt/sched/mod.rs:7-9`: link tới `scheduler`, `frame_api`, `frame_next`,
  `frame_su_kien`, `frame_ket_thuc`, `frame_huy`, `frame_reactor` (đều item private).
- `crates/vis-runtime/src/rt/select.rs:16`: `don_dep` không tồn tại + `huy_frame` private.

Các tệp này **ngoài lane**, nên không vá; cũng không thêm `#![allow(...)]` vào `rt.rs` vì
đó là đổi code và che giấu 9 link hỏng thật. Đề xuất A0 mở task sửa link riêng (tương tự
cách `ST-324b` đã xử lý link trong `vis-http`) rồi yêu cầu lại KPI doc cho crate này.

Không chạy `vis-diff` (doc-only, 0 byte mã sản phẩm đổi).

## 4. Kết luận

Repro xác nhận chú thích cũ sai, nhưng chú thích đã được `ST-316b` đính chính đúng từ trước
(`rt/ops.rs:43-47`) ⇒ **đã đúng, không sửa nguồn**. Commit journal `[ST-325c] journal: da dung`;
chờ Gatekeeper/Agent 0, **không** merge/push. Doc KPI đỏ do nợ có sẵn ngoài lane — báo A0.
