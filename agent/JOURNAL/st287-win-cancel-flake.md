<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-287 — triage flake `vs3_http_cancel` trên Windows (trước tag `v0.3.6`)

> **Người làm**: subagent VS3-W2, 2026-11-22. **Base**: `92e381b` (merge final Windows
> `a1930cb`). Không push/merge/tag; không GitHub Actions. **Không sửa mã sản phẩm.**

## 0. TL;DR

| Hạng mục | Trước | Sau |
|---|---|---|
| Windows `vs3_http_cancel` full suite `--test-threads=1` | **2/10 pass** (80 % fail) | **20/20 pass** |
| Windows full suite default (song song) | **8/10 pass** (20 % fail) | **10/10 pass** |
| Windows ca riêng `lap_nhieu_vong` ×20 | 20/20 pass | **20/20 pass** |
| Windows ca riêng `server_khong_tra_header` ×20 | 3/10 fail (W4) | **20/20 pass** |
| Windows ca riêng `dong_socket` | 1/10 fail (W4) | chạy trong 20 suite serial — 0 fail |
| Linux `vs3_http_cancel` default / `--features async-h2` | — | **10/10** / **10/10** |
| `./vis-dev gate --fast` Linux | 3581/3 | **3581/3 PASS** |
| Nguyên nhân | — | **test fixture**: socket `accept` trên Windows thừa hưởng nonblocking |
| Quyết định tag | — | **đủ điều kiện tag theo matrix này**; không tag trong phiên |

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

## 3. Sửa tối thiểu (test-only)

`crates/vis-cli/tests/vs3_http_cancel.rs` — `phuc_vu_h1`:

```rust
tcp.set_nonblocking(false).expect("server blocking");
```

kèm chú thích Windows; thông điệp assertion `V:post` in thêm `out.log` để lần sau thấy ngay
`cancelreq`/`cancel:` (không đổi kỳ vọng/ngưỡng). Không đụng `FrameReactorBackend`,
`frame_cancel_one`, `AsyncNet` hay bất kỳ mã sản phẩm nào.

## 4. Matrix sau sửa

| Nền / cách chạy | Kết quả |
|---|---|
| Windows full suite `--test-threads=1` ×20 | **20/20 pass** |
| Windows full suite default ×10 | **10/10 pass** |
| Windows ca riêng `lap_nhieu_vong` ×20 `--test-threads=1` | **20/20 pass** |
| Windows ca riêng `server_khong_tra_header` ×20 `--test-threads=1` | **20/20 pass** |
| Windows workspace 1 vòng | xem §5 (chạy tại commit sửa) |
| Linux default ×10 | **10/10 pass** |
| Linux `--features async-h2` ×10 | **10/10 pass** |
| Linux `./vis-dev gate --fast` | **PASS — 3581 passed / 0 failed / 3 skipped** |

## 5. Windows workspace tại commit sửa

(chèn kết quả `cargo test --workspace --no-fail-fast` tại commit `[ST-287]` sau khi chạy;
target riêng `/c/Users/ps/vt-st287-target`.)

## 6. Quyết định

- Flake **không phải lỗi sản phẩm**; sửa test fixture là đủ và tối thiểu, không che lỗi: trigger
  huỷ vẫn chạy sau khi request đã đi, các assertion `cancel:1`/`V:post`/`dem_tay` giữ nguyên.
- **Đủ điều kiện tag `v0.3.6`** theo matrix Windows + Linux + gate 3581/3; phiên này **không tag**,
  không push/merge.
