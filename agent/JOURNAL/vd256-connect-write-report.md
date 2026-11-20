# ST-277 / VD-256 — connect/write nonblocking trên đường frame TEST

> Phạm vi: **chỉ integration test** (`frame.enabled == true` + có `FrameReactorBackend`).
> Không commit. Không đụng các tệp của luồng ngân sách ST-276 đang chạy song song.

## 1. Contract → evidence

| VD-256 §5 yêu cầu | Cách làm | Evidence |
|---|---|---|
| `connect`: return `Pending`, register, rồi **kiểm tra kết quả nối thực tế** (readiness chỉ là hint) | Adapter khởi tạo `mio::TcpStream::connect` (nonblocking), đăng ký `Interest::WRITABLE`; khi có event mới gọi `take_error()` (SO_ERROR) **và** `peer_addr()`. Chỉ trả tay `C<n>` khi cả hai sạch. | `lat256_net.rs:130` (`try_connect`), `:145` (WRITABLE), `:157` (`take_error`), `:171` (`peer_addr`); test `t1` (từ chối), `t2` (nối xong rồi ghi). |
| `write`: **partial write + lưu offset**, resume không gửi lại phần cũ | State `WriteState{data, off}` nằm trong adapter; mỗi lần ghi `[off..off+chunk]`, `off += n`, trả `WouldBlock` nếu còn bytes ⇒ core re-arm; resume đọc đúng `off`. | `lat256_net.rs:51` (`WRITE_CHUNK`), `:197` (`try_write`), `:240` (`ws.off += n`); test `t3`, `t4` (1 MiB, so sánh từng byte), `t6`. |
| Deadline **không reset** theo readiness; một deadline monotonic từ đầu lời gọi | Tái dùng nguyên `frame_real_poll`/`frame_idle_real` của `rt.rs`: deadline gán lần đăng ký đầu (`rt.rs:1057`), giữ nguyên khi re-arm (`rt.rs:1112–1116`). Không helper mới trong `rt.rs`. | `lat256_net.rs:999` `t9`: so sánh `ReactorReg.deadline` trước/sau re-arm bằng nhau; `t8`: 4 MiB + peer không đọc ⇒ thoát theo hạn 300 ms. |
| Peer không đọc **không giữ scheduler vô hạn**; timeout rõ, cleanup | Hạn write (mặc định `FRAME_WRITE_TIMEOUT_MS = 5000` ms, test override 300 ms) → `frame_real_poll` trả `Err("quá thời gian …")`, `frame_real_done` gỡ đăng ký. | `stdlib.rs:57,86,92`; `lat256_net.rs:954` `t8` (elapsed < 3 s, `Ok`/`Err` đúng, snapshot baseline). |
| Close/timeout/error **deregister đúng một lần**, không wake nhầm task (generation, fd reuse) | Dùng `token_map` + `mint_token` có sẵn (`rt.rs:992, 1197–1225`); `deregister` của adapter theo token, idempotent (`lat256_net.rs:338`); `net.close` huỷ cả khoá `W:<tay>` (`stdlib.rs:695`). | `t5` (đóng giữa chừng → `Err` đúng 1 lần, baseline), `t9` (token 0/1/2 tăng, stale token không wake, `deregistered == [0]` rồi `[0,1]`). |
| Task khác tiến triển khi một task chờ connect/write | Return `Pending` ⇒ driver chạy frame khác; thứ tự in chứng minh interleaving. | `t6` (`A:pre < BG < A:post`), `t7` (connect: cùng thứ tự). |
| Không đổi hợp đồng trả của `net.write` | Vẫn `Result<(), str>`; `frame_unit_result` chỉ đổi payload `Ok(str)` → `Ok(())`, `Err` giữ nguyên. **Không cần proposal riêng.** | `stdlib.rs:103`, `:680`; test `t2/t3/t4/t6` kiểm `Ok(())`. |

## 2. Điểm nối implementation (file:line)

- `crates/vis-interp/src/stdlib.rs`
  - `:48–57` hằng test-only `FRAME_NET_CONNECT = "N:"`, `FRAME_NET_WRITE = "W:"`, `FRAME_WRITE_TIMEOUT_MS = 5000`.
  - `:70–97` kênh thread-local chứa bytes cần gửi + override hạn write (`#[doc(hidden)]`, chỉ test).
  - `:103` `frame_unit_result`: giữ nguyên biểu thức `Result` của ngôn ngữ.
  - `:598–620` `net.connect[_timeout]`: trên frame + có backend ⇒ `frame_reactor_poll_io(ReactorOp::Read, "N:<addr>", …)`; nếu không ⇒ đi `NetHost::connect` y hệt cũ.
  - `:660–682` `net.write`: ghi bytes vào kênh rồi đi cùng helper; resume được `Redispatch` trong `frame.rs` gọi lại với **đối số đã cache** ⇒ không eval lại đối số/không gửi lại bytes.
  - `:689–697` `net.close`: huỷ cả đăng ký `tay` lẫn khoá `W:<tay>`.
- `crates/vis-cli/tests/lat256_net.rs`
  - `:51–64` adapter `Net2` + `WriteState`.
  - `:130–190` connect nonblocking + kiểm chứng SO_ERROR/peer_addr.
  - `:197–259` partial write/offset.
  - `:305–322` `register`: `WRITABLE` cho `N:`/`W:`, `READABLE` cho read thường.
  - `:338–355` `deregister` theo token, idempotent.
  - `:368–395` `close`: dọn stream, write-state, connect-state, token.

**Vì sao đi qua `ReactorOp::Read`?** Trait `FrameReactorBackend` (trong `rt.rs`, **không được sửa**) chỉ có `try_read`/`try_accept` và `ReactorOp` chỉ có `Read`/`Accept`. Handle `N:`/`W:` là kênh điều phối trong adapter test; core vẫn dùng đúng helper `frame_reactor_poll_io`. **Không cần helper mới trong `rt.rs`** — do đó không có yêu cầu báo cáo bổ sung về `rt.rs`.

## 3. Timeout / cleanup / generation — chi tiết

- **Deadline**: `frame_real_poll` tạo deadline ở lần đăng ký đầu và **giữ nguyên** khi re-arm; `t9` so sánh trực tiếp hai `Instant` (bằng nhau). `t8` chứng minh bound thực (300 ms, 4 MiB sinh bằng `str.repeat`, peer không đọc).
- **Cleanup**: mọi nhánh thắng lợi/lỗi/timeout đi `frame_real_done` (deregister + xoá `token_map`); `net.close` đi `frame_reactor_cancel_handle` cho cả hai khoá; cuối frame driver gọi `frame_reactor_shutdown`. `diag_snapshot()` về `baseline()` ở **mọi** test.
- **Generation**: `mint_token` tăng dần, token cũ sau re-arm bị `token_map.remove` ⇒ event stale chỉ log `rstale:…`, không wake (`t9`). Adapter `close` xoá stream nhưng `deregister` theo token nên fd dùng lại không deregister nhầm.

## 4. Kết quả kiểm chứng

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test lat256_net` (3 lần liên tiếp, `--test-threads=1`) | **9 passed / 0 failed** mỗi lần (8.20 s, 8.15 s, 8.17 s). |
| `cargo test -p vis-interp -p vis-cli` | **1058 passed / 0 failed** (110 dòng `test result`, gồm doc-tests). |
| `rustfmt --check` trên 2 tệp thuộc quyền | sạch. |
| `./vis-dev check` | Lần đầu **FAIL tại `--- fmt`** (7 tệp của luồng ST-276 đang sửa song song: `vis-bytecode/{src/codegen.rs, src/fee.rs, tests/fee_side_table.rs}`, `vis-cli/{src/cache.rs, tests/lat256_budget.rs}`, `vis-interp/src/{interp.rs, rt.rs}`); hai tệp của slice này **không** xuất hiện trong diff fmt. Sau khi luồng kia sửa fmt: `--- fmt: OK`, check dừng ở **`--- clippy: FAIL`** do `crates/vis-bytecode/src/fee.rs:333,413` (quá nhiều đối số) và `:476` (assert hằng) — **đều là tệp của luồng ST-276**. Không sửa các tệp đó theo đúng phân quyền; các bước sau clippy chưa tới. |

Các test: `t1` connect refused; `t2` connect→write thật; `t3` peer đọc chậm; `t4` write 1 MiB/backpressure, so khớp từng byte; `t5` đóng giữa chừng; `t6`/`t7` task khác tiến triển; `t8` timeout có hạn; `t9` token/generation/deadline tất định. Cả hai engine (interp + VM) trừ `t8`/`t9` chạy chung harness (t8 chạy cả hai engine, t9 là unit test trên `Scheduler`).

## 5. Không đổi / giới hạn trung thực

- **Runtime mặc định không đổi** (`frame.enabled == false` ⇒ `net.connect`/`net.write` vẫn gọi `NetHost` đồng bộ y hệt trước). **Public API ngôn ngữ không đổi** (không hàm mới, không đổi chữ ký/kiểu trả). **VERSION không đổi** (không đụa format artifact/cache). `crates/vis-cli/src/net.rs` **không cần sửa** (mio là dev-dependency, adapter thuộc test) nên giữ nguyên dù nằm trong phạm vi được phép.
- Đây là **test hook Rust** (`#[doc(hidden)] pub` trong `stdlib.rs`), không phải tính năng ngôn ngữ; không lọt vào CLI hay artifact phát hành.
- Adapter test **giới hạn mỗi lần write ở 64 KiB** (`WRITE_CHUNK`) để partial-write/resume tất định, không phụ thuộc send-buffer của máy; kernel vẫn tham gia (socket nonblocking thật, t8 tạo backpressure thật bằng peer không đọc).
- Ngôn ngữ không có đối số timeout cho `net.write` ⇒ hạn 5 s là **chính sách test**, có thể override bằng hook ẩn; không đổi hợp đồng hàm.
- **Chưa làm**: đóng trực tiếp một `connect` đang bay bằng `net.close` vì trước khi `connect` trả về chưa có tay để đặt tên — phủ bằng (a) test `t7` không wake nhầm khi task khác đóng tay khác, (b) bound deadline, (c) `t9` huỷ đăng ký tất định. `rt.rs`, `interp.rs`, `vm*`, bytecode, cache, DECISIONS/QUEUE **không bị sửa** bởi slice này.
- **Khẳng định**: việc này **chỉ hoàn tất connect/write trên đường test frame**. Nó **không** biến runtime mặc định thành runtime async/full nonblocking; mục tiêu đó vẫn nằm ngoài VD-256 và cần quyết định riêng.
