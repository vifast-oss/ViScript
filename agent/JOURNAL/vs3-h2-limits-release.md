<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Release-mode HTTP/2 limits — 2026-11-21 (W2)

- **Baseline**: `main` `5103da4` (CLI/net flake fix; gate 3530/3), nhánh `vs3/w2-tls`
  fast-forward `58837f9..5103da4`, không conflict. `http2` default-on.
- **Phạm vi**: `crates/vis-cli/src/net/h2.rs` (1 fix production bắt buộc) +
  `crates/vis-cli/tests/vs3_http2.rs` (test/server hermetic). Không dependency mới,
  không GitHub Actions, không push/merge.
- Môi trường: local hermetic TLS + ALPN h2, fixture cert; W1 đang fuzz 4h
  (`vis-parse-fuzz` ~98% CPU, load ~3) nên thời gian test dao động.
- Log thô: [`vs3-h2-limits-release.log`](vs3-h2-limits-release.log).

## 1. Bug production phát hiện bởi test 8 MiB

Test body lớn ban đầu **timeout 30 s** với cả 1 MiB và 8 MiB. Nguyên nhân: `RecvStream` của
`h2` **không tự trả cửa sổ flow-control**; client đọc `Bytes` nhưng không gọi
`release_capacity`, nên sau ~64 KiB server bị chặn và hai bên chờ nhau tới timeout.

Fix tối thiểu trong `PhienH2::doc_phan_hoi` (và prototype test-only `gui_nhieu`):

```rust
let n = b.len();
body.extend_from_slice(&b);
drop(b);
r.flow_control().release_capacity(n)?;
```

Đây là sửa production **cần thiết** để body > 64 KiB hoạt động; không đổi API/hành vi cap.
Sau fix: 1 MiB ~0,3 s; 8 MiB ~0,2 s khi máy rảnh, tới ~10 s khi W1 fuzz nặng — không treo.

## 2. Test mới (release)

Server test được nâng để stream body theo chunk 64 KiB và thêm route:

| Ca | Nội dung | Kỳ vọng |
|---|---|---|
| `h2_body_8mib_va_vuot_cap` | đúng **8.388.608 byte** rồi **8.388.609 byte** | 8 MiB nhận đủ; +1 lỗi cap; request sau mở connection mới |
| `h2_header_sat_cap_va_vuot` | `name+value` đúng **65.536 byte**, rồi **65.537 byte** | sát cap nhận; +1 lỗi; connection lỗi bị bỏ |
| `h2_trailer_body_lon_va_tai_su_dung` | body **1 MiB** + trailer `x-trailer` | nhận đủ body, đọc trailer, tái dùng 1 connection |
| `h2_goaway_giua_body` | header + 3 byte rồi GOAWAY khi stream mở | lỗi rõ, không treo; request sau connection mới |
| `h2_timeout_giua_body_va_cleanup` | header + 3 byte rồi treo 900 ms, client timeout 250 ms | timeout rõ ~1 s; connection mới cho request sau |

Kiểm tra connection:

- body: sau 8 MiB + request vượt cap vẫn **1 connection**; sau lỗi + `/b` → **2**.
- header: sát cap dùng connection đầu; sau lỗi + `/b` → **2**.
- trailer: request sau vẫn **1 connection**.
- GOAWAY/timeout: request sau lỗi mở connection mới (đếm tăng).

Lỗi quan sát:

```text
phản hồi HTTP/2 vượt giới hạn 8388608 byte (8 MiB)
header HTTP/2 quá lớn (> 65536 byte)
quá thời gian HTTP/2
GOAWAY HTTP/2 … / lỗi HTTP/2 khi đọc body …
```

Không panic, không treo quá timeout; cap là **ngưỡng bao gồm** (đúng cap nhận, +1 từ chối).

## 3. Kết quả release

```text
cargo test --release -p vis-cli --test vs3_http2
  19 passed / 0 failed (15,29–15,49 s; 3 vòng full liên tiếp)

cargo test --release -p vis-cli --test vs3_tls
  21 passed / 0 failed (4,27 s)
cargo test --release -p vis-cli --test http
  22 passed / 0 failed (5,01 s)

cargo test -p vis-cli --test vs3_http2   # debug regression
  19 passed / 0 failed (15,51 s)

cargo clippy -p vis-cli --bin vis --lib --test vs3_http2 -- -D warnings  Finished
cargo check -p vis-cli --target wasm32-unknown-unknown --no-default-features --features http2  Finished
./vis-dev rules  OK
```

Thời gian từng test mới (lần chạy riêng, release):

| Test | Thời gian |
|---|---:|
| `h2_body_8mib_va_vuot_cap` | 0,16–9,8 s (dao động do W1 fuzz; 20/20 lần pass) |
| `h2_header_sat_cap_va_vuot` | 0,13 s |
| `h2_trailer_body_lon_va_tai_su_dung` | 0,01–0,09 s |
| `h2_goaway_giua_body` | 0,05 s |
| `h2_timeout_giua_body_va_cleanup` | 0,95 s |

Body test được chạy lặp **20 lần release** dưới tải fuzz: 20/20 pass (trước khi nới timeout
từ 5 s lên 20 s có 1–2 lần vượt 5 s do tải, không phải treo logic). Full release suite
3 vòng liên tiếp đều 19/19.

## 4. Blocker

- **True multiplex production** vẫn chưa mở: API `http_get/head` đồng bộ + `Net` `&mut` một
  luồng; prototype test-only `gui_nhieu` đã chứng minh 4 stream/1 connection. API async h2
  vẫn chờ W1.
- Idle cleanup lazy (request kế tiếp/Net drop), không reaper.
- Windows native h2/release chưa đo; cross-check MSVC dừng ở `ring`/`libsqlite3-sys` (#156).
- Cap sản phẩm: body **8 MiB**, header **64 KiB** (bao gồm name+value); vượt 1 byte bị từ chối.
- Timing test nhạy tải (W1 fuzz); đã nới timeout body lên 20 s/assert 25 s, không nới logic.
- Đĩa sau test: **19G trống** (> ngưỡng 5G). Không dependency mới, không push/merge/tag.
