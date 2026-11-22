<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-287 — triage flake `vs3_http_cancel` trên Windows (trước tag `v0.3.6`)

> **Người làm**: subagent VS3-W2, 2026-11-22. **Base**: `92e381b` (merge final Windows
> `a1930cb`). Không push/merge/tag; không GitHub Actions. Sửa test fixture + P0/P1 sản phẩm tối thiểu.

## 0. TL;DR

| Hạng mục | Trước | Sau |
|---|---|---|
| Windows `vs3_http_cancel` full suite `--test-threads=1` | **2/10 pass** (80 % fail) | **20/20 pass** |
| Windows full suite default (song song) | **8/10 pass** (20 % fail) | **10/10 pass** |
| Windows ca riêng `lap_nhieu_vong` ×20 | 20/20 pass | **20/20 pass** |
| Windows ca riêng `server_khong_tra_header` ×20 | 3/10 fail (W4) | **20/20 pass** |
| Windows ca riêng `dong_socket` | 1/10 fail (W4) | chạy trong 20 suite serial — 0 fail |
| Linux `vs3_http_cancel` default / `--features async-h2` | — | **10/10** / **10/10** |
| `./vis-dev gate --fast` / `./vis-dev check` Linux | 3581/3 | **3581/3 PASS** / **check PASS** (report-check 32/322) |
| Nguyên nhân | — | **test fixture**: socket `accept` trên Windows thừa hưởng nonblocking |
| Quyết định tag | — | **đủ điều kiện tag theo matrix này**; không tag trong phiên |
| Commit | — | `e43ca43` (fixture) + `6b2a216` (P0/P1 + bytes trigger) |

## 1. Triệu chứng W4

`a1930cb` trên `psvn`: `vs3_http_cancel` h1 flake ~21 % (3/14 lần suite), kể cả
`--test-threads=1`; ca riêng ×10 pass. Thông điệp: task bị huỷ vẫn in `V:post` sau lỗi đọc
`WSAECONNABORTED (10053)`; có lần task anh em cũng bị đóng. `--features async-h2` rc 101 là
expected (Unix-only), không tính.

Matrix W4 bổ sung @ `a1930cb`:

| Cách chạy | Kết quả |
|---|---|
| ca riêng `lap_nhieu_vong` ×20, `--test-threads=1` | **20/20 pass** |
| full suite default ×10 | **8/10 pass** |
| 2 process song song ×5 | **10/10 process pass** |
| full suite `--test-threads=1` ×10 | **2/10 pass** |
| ca riêng `server_khong_tra_header` ×10 | **3/10 fail** |
| ca riêng `dong_socket` ×10 | **1/10 fail** |

## 2. Tái hiện + chẩn đoán

- Tái hiện `92e381b`/target riêng trên `psvn` (`/c/Users/ps/vt-st287-target`): suite
  `--test-threads=1` **7/20 fail**; ca fail luân phiên (`lap`, `mot_task`, `server_khong_tra`).
- Log scheduler của ca fail **không có `cancelreq`/`cancel:1`**, chỉ có `cancelonturn:20:1`
  (đăng ký trigger) rồi `rerr:C1: ... 10053` và `rshutdown` — trigger turn 20 **chưa bao giờ
  chạy** vì request đã lỗi trước ~190 ms.
- Instrument server (`H1SRV`): `read` request có lần trả **`WSAEWOULDBLOCK (10035)`**; handler
  `phuc_vu_h1` dùng API blocking + timeout nên coi `Err` là kết thúc và đóng socket; các lần khác
  `write_all` body trả **10053** sau ~189 ms. Client đọc thấy 10053/EOF ⇒ `http.get_timeout` trả
  `Err(Error)` ⇒ source in `V:post` **hợp lệ**; đây không phải task bị huỷ chạy tiếp.
- **Gốc**: listener test đặt `set_nonblocking(true)`; trên Windows socket do `accept` **thừa hưởng
  nonblocking** (khác Linux, nơi accepted socket blocking). Handler giả định blocking ⇒
  `WSAEWOULDBLOCK`/abort ngẫu nhiên tuỳ thời điểm request/data. Chạy serial trong cùng process làm
  thời điểm đến của request/data khác đi ⇒ flake tăng; 2 process song song ít chồng lấn hơn.
- Kiểm state toàn cục: `frame_cancel_clear_triggers_for_test` clear cả trigger `seq` + `turn`;
  `cancel_on_turn` thuộc `FrameTest` của scheduler mới mỗi `run_*`; `AsyncNet`/server/port đều tạo
  mới mỗi ca. Không thấy static/thread-local rò giữa ca. Nguyên nhân là fixture, không phải
  safe-point sản phẩm.

## 3. Sửa tối thiểu

**Test fixture (P0, commit `e43ca43`)**: `phuc_vu_h1` thêm

```rust
tcp.set_nonblocking(false).expect("server blocking");
```

**Test trigger (P0, bám dữ liệu)**: `frame_cancel_after_bytes_for_test("C", 8192, id)` — huỷ khi
`try_read` thật của frame đã trả đủ 8192 byte trên handle `C…` (giữa body), không phụ thuộc
`SO_TURN=20`; turn chỉ còn là fallback cho h2 (`H`) và `/im` (chưa có header). Log scheduler có
`cancelonbytes:C:8192:1` rồi `cancel:1`.

**Product P0**: driver frame bỏ qua frame đã nằm trong `frame.cancelled` **trước** khi set
`DangChay`/chạy `step_frame`; không ghi `Xong`/cache cho task đã huỷ.

**Product P1 (Windows)**: `AsyncNet::close` deregister token + dọn `writes` **trước** khi drop
`MioStream`/`TlsChan` (drop/`closesocket` trên Windows huỷ I/O đang chờ; deregister sau có thể
trả 10053). `cancel_task_handles`/`RST_STREAM` giữ nguyên.

**P2**: chưa thêm `clear_all_for_test` — không thấy state thread-local rò giữa ca; xem §2.

## 4. Matrix sau sửa

| Nền / cách chạy | Kết quả |
|---|---|
| Windows full suite `--test-threads=1` ×20 @ `6b2a2161` | **20/20 pass** (trước 7/20 fail) |
| Windows full suite default ×10 @ `6b2a2161` | **10/10 pass** |
| Windows ca riêng `lap_nhieu_vong` ×20 @ `6b2a2161` | **20/20 pass** |
| Windows ca riêng `server_khong_tra_header` ×20 (fixture fix) | **20/20 pass** (trước 3/10 fail) |
| Windows workspace @ `6b2a2161` | **3538/0/3**, 355 block, dirty 0 |
| Linux default ×10 | **10/10 pass** |
| Linux `--features async-h2` ×10 | **10/10 pass** |
| Linux `./vis-dev gate --fast` | **PASS — 3581 passed / 0 failed / 3 skipped** |

## 5. Windows workspace tại commit `6b2a2161`

- `psvn` detached `6b2a2161`, dirty 0, `CARGO_TARGET_DIR=/c/Users/ps/vt-st287-target`.
- `cargo test --workspace --no-fail-fast` — **355 block; 3538 passed / 0 failed / 3 ignored**,
  `rc=0` (532 s). Log `JOURNAL/st287-win-cancel-workspace.log`.
- Suite `vs3_http_cancel` tại đúng commit: serial **20/20**, default **10/10**, exact lap
  **20/20** (`JOURNAL/st287-win-cancel-final-matrix.log`).

## 6. Quyết định

- Flake **không phải race scheduler thuần**: fixture nonblocking tạo lỗi 10053 trước safe point.
  Vẫn áp P0/P1 phòng vệ (không chạy/cache task đã huỷ; deregister trước drop) và trigger theo
  byte để test bám dữ liệu thật, không che lỗi: assertion `cancel:1` kiểm **trước** `V:post`,
  `dem_tay`/`snapshot` giữ nguyên.
- **Đủ điều kiện tag `v0.3.6`** theo matrix Windows + Linux + gate 3581/3; phiên này **không tag**,
  không push/merge.
