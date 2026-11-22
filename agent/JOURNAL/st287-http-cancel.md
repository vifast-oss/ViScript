<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-287 — Hook huỷ giữa HTTP request cho h1 + h2 (`VS3-1b`, VD-273)

> **Người làm**: subagent VS3-W2, 2026-11-22.
> **Base**: `84915aa` (merge ST-289, version `0.3.6`); nhánh `plan/w2-net-sec`.
> **Phạm vi**: thêm hook huỷ tại safe point; **không** đổi API sync/default, `async-h2` vẫn
> default-off, không CLI/feature/ABI mới cho người dùng.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Hook | `FrameReactorBackend::cancel_task_handles(frame_id)` + 2 hook ghi sổ (`note_task_handle`, `note_task_h2`) |
| Safe point | `Scheduler::frame_cancel_one` đóng tay HTTP async **sau** khi gỡ đăng ký, trước terminal |
| h1 | đóng socket thật (`C<n>`/`N:`), giải phóng `writes`, `dem_tay_dang_mo()==0` |
| h2 | `RST_STREAM` đúng stream + lái session một nhịp để frame ra socket; session dùng chung **không** bị đóng, `h2_users` về 0 |
| Test đỏ-trước | **5/5 h1 fail**, **7/7 (h1+h2) fail** khi bỏ hook |
| Test xanh | `vs3_http_cancel` **5/5** default, **8/8** với `--features async-h2` |
| Hồi quy | `vs3_http2` **19/19** (http2), **34 + 1 ignored** (async-h2), `vs3_tls` **21/21**, `vs3_default` **22/22**, `vs3_cancel` **14/14**, `lat257_c_lifecycle` **11/11**, unit `vis` **196/196**, `quyen` **8/8**, `sqlite` **17/17** |
| `./vis-dev gate --fast` | **PASS** — **3581 passed / 0 failed / 3 skipped** |
| Differential 100k | **lech=0** (seed `20261122`, 94,4 s, 93 770 ca chạy) |
| Blocker | **không** |

## 1. Thiết kế hook

- `FrameReactorBackend` thêm 3 method **default no-op** (không phá adapter scripted/test cũ):
  - `note_task_handle(frame_id, handle)`: chỉ `net/async_http.rs` gọi khi giữ khoá `N:`/`T:` hoặc
    handle `C<n>`/`H<n>#frame` — **không** ghi language handle của `net.read/write`.
  - `note_task_h2(frame_id, handle, id)`: `frame_reactor_h2_send` gọi sau khi `h2_send` trả id.
  - `cancel_task_handles(frame_id)`: safe point đóng mọi tay đã ghi sổ.
- `AsyncNet` giữ `frame_tay: HashMap<u32, Vec<String>>` + `frame_h2: HashMap<u32, Vec<(String,u64)>>`:
  - connect xong (`C`/`H`) thì bỏ khoá chờ `N:`/`T:` tương ứng;
  - `close(handle)`/`h2_cancel(handle,id)` gỡ **đúng** handle khỏi sổ (không đụng `H<n>#frame khác`);
  - `cancel_task_handles`: `RST_STREAM` từng stream h2 rồi `close` từng tay, khử trùng handle.
- `Scheduler::frame_cancel_one` gọi `cancel_task_handles(id)` **sau** bước gỡ đăng ký/write buffer,
  rồi `net::xoa_trang_thai_frame(id)` bỏ state `HttpAsync` — frame id tái dùng không `dong()` lần hai
  (h2 user/`RST_STREAM` đúng một lần).
- `h2_cancel_impl` lái `H2Async::tien()` tối đa 8 nhịp sau `huy_stream` để `RST_STREAM` ra socket
  ngay tại safe point, không đợi request kế tiếp.

Test hook (chỉ test): `frame_cancel_after_turns_for_test(turns,id)` — drain vào
`FrameTest.cancel_on_turn`, đếm trong `frame_next`; production `cancel_on_turn` rỗng ⇒ chỉ một
`is_empty`, **không** tăng bộ đếm.

## 2. Đỏ-trước

Bỏ tạm khối hook trong `frame_cancel_one` (giữ nguyên test), chạy lại:

| Lệnh | Kết quả | Log |
|---|---|---|
| `cargo test --test vs3_http_cancel` | **0 passed / 5 failed** | `st287-red-h1.log` (sha256 `8a6c1fe8a6a65fd2…`) |
| `cargo test --test vs3_http_cancel --features async-h2` | **0 passed / 7 failed** | `st287-red-h2.log` (sha256 `47460e39251c6b98…`) |

Khôi phục hook ⇒ xanh lại toàn bộ.

## 3. Test xanh

| Lệnh | Kết quả |
|---|---|
| `cargo test --test vs3_http_cancel` | **5 passed** (h1: parity+đóng socket, 5 vòng không rò, request sau huỷ, server không trả header, một task không ảnh hưởng task khác) |
| `cargo test --test vs3_http_cancel --features async-h2` | **8 passed** (thêm h2: parity+RST, 3 vòng tái dùng session, hai task chung session huỷ cả hai) |
| `cargo test --test vs3_http2 --features http2` | **19 passed** |
| `cargo test --test vs3_http2 --features async-h2` | **34 passed / 1 ignored** |
| `cargo test --test vs3_tls` | **21 passed** |
| `cargo test --test vs3_default` | **22 passed** |
| `cargo test --test vs3_cancel` | **14 passed** |
| `cargo test --test lat257_c_lifecycle` | **11 passed** |
| `cargo test --bin vis` | **196 passed** (gồm `quyen`, `db::sqlite`, `quyen_st174`) |
| `cargo test --test quyen` / `--test sqlite` | **8/8** / **17/17** |

Đo rò: mỗi ca khẳng định `AsyncNet::dem_tay_dang_mo() == 0` sau huỷ (nguồn sự thật nội bộ thay
cho đếm fd toàn tiến trình — như `vs3_default`), `h2_users` cộng vào tổng này; h2 còn khẳng định
server thấy **≥ 2 `RST_STREAM`**, số connection giữ nguyên (session tái dùng) và stdout parity
interp↔VM.

## 4. Cổng + differential

- `./vis-dev gate --fast`: **PASS**, `exit 0`; L0 **3581 passed / 0 failed / 3 skipped**
  (89,4 s); fmt/clippy/R1..R6/release/self-test/doc-examples/readme-examples OK.
  Log `st287-gate.log` (sha256 `480aa049c318364f…`).
- `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261122 ./bin/vis-diff`:
  `lech=0` — chính xác `DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=94.4`.
  Log `st287-diff-100k.log` (sha256 `7d9beeec61ac144be…`).
- `cargo check -p vis-cli --no-default-features` (+ `--features tls`) xanh; còn 2 cảnh báo
  **có sẵn** ở `async_net_nguon.rs` khi tắt `tls` (không do ST-287).

## 5. Ghi chú API

- Không đổi `Net`/`NetHost`/CLI/feature; `async-h2` vẫn default-off.
- `FrameReactorBackend` là trait `#[doc(hidden)]` nội bộ; 3 method mới có default no-op nên adapter
  cũ (scripted, test script) không phải sửa; không đổi chữ ký method cũ.
- Không `unsafe` mới, không crate mới.
