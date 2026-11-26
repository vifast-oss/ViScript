<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315d (phần 3) — harness parity **66/66 hàm host** (`interp ↔ VM`)

> **Lát**: `ST-315d` (W4 · Sóng 4) — phần 3/3 · **Worktree**: `/home/ps/ViScript-w4` · nhánh
> `plan/st315d-parity` · **Nền**: `9356f1c8` (khung 24 hàm `fs`+`time`) · **Commit**: kế tiếp
> `9356f1c8` (tiền tố `[ST-315d]`, không push) · **Ngày**: 2026-11-26 · **Trạng thái**: `READY` —
> phủ đủ **66/66** hàm host, `gate --fast` PASS.
> **Phạm vi**: chỉ `crates/vis-conformance/tests/misc_api_it/host_parity.rs` (+ tệp nhật ký này).

## 1. Kết quả

- **66/66 hàm host** có ca chạy thật qua **cả hai engine** (`vis run` ↔ `vis run --vm --no-cache`),
  so **stdout + stderr + exit** từng byte; ca lỗi in thêm **mã/thông điệp** để so parity.
- **10 `#[test]`** trong `host_parity` (4 cũ + **6 mới**), tổng thời gian ~1,2 s:
  | Test | Hàm phủ | Số hàm |
  |---|---|---|
  | `fs_20_ham_parity_tren_tempdir` (cũ) | 20 `fs` | 20 |
  | `fs_symlink_parity` (cũ, `#[cfg(unix)]`) | `fs.is_symlink`, `fs.read_link` | 2 |
  | `time_hai_ham_parity_bat_bien` (cũ) | `time.now_ms`, `time.monotonic_ms` | 2 |
  | **`sqlite_dml_va_giao_dich_parity`** | `open_with`, `close`, `execute`, `execute_batch`, `query`, `query_one`, `explain`, `last_insert_rowid`, `changes`, `begin`, `commit`, `rollback`, `in_transaction` | 13 |
  | **`sqlite_metadata_parity`** | `open`, `application_id`, `data_version`, `schema_version`, `user_version`, `journal_mode`, `tables`, `table_exists`, `columns`, `indexes`, `index_exists`, `index_columns`, `primary_keys`, `schema_sql`, `database_list` | 15 |
  | **`net_loopback_parity`** | `listen`, `local_addr`, `connect`, `accept`, `read`, `write`, `close`, `accept_timeout`, `close_listen`, `connect_timeout` | 10 |
  | **`net_client_toi_server_rust_parity`** | `connect`, `connect_timeout`, `read`, `write`, `close` tới server Rust thật | (trong 10) |
  | **`http_bon_ham_parity`** | `get`, `get_timeout`, `head`, `head_timeout` | 4 |
  | **`phu_du_66_ham_host`** (cũ) | bảng `HOST_66` ↔ `StdId::ALL` | khoá 66 |
  | **`moi_ham_host_co_loi_goi_trong_harness`** (mới) | mỗi tên `HOST_66` phải có lời gọi `tên(` trong tệp | khoá 66 |
- **0 binary test cấp 1 mới** (R8 giữ **39 ≤ 40**); tệp vẫn khai `#[path]` ở
  `lib_api_it.rs:81-82`.

## 2. Cách làm

- Giữ nguyên kiến trúc harness của phần trước: chạy binary `vis` thật (đường CLI, có host native
  `FsGuard`/`NetGuard`/`DbGuard`) trên **cùng một tệp**, hai đường `run` và `run --vm --no-cache`;
  `hai_duong_voi` gọi `setup()` **trước mỗi đường** để fixture về đúng trạng thái ban đầu.
- **sqlite**: `:memory:` cho metadata (tất định); **tệp DB thật** cho DML/giao dịch, `setup` xoá
  cả thư mục DB trước mỗi engine. Ca lỗi: SQL sai, thiếu/thừa tham số, `begin` lồng, `commit`
  ngoài transaction, handle sau `close`, `open_with(create = false)` trên tệp thiếu.
  `explain` chỉ assert **chạy được** (`columns` 4 cột + có plan), **không ghim plan**.
- **net**: (a) một chương trình `vis` tự `listen("127.0.0.1:0")` → `local_addr` → `connect` →
  `accept` (dựa backlog TCP của kernel, **không cần canh thời gian**) → `write`/`read` hai chiều
  → `close`; ca lỗi listen trùng cổng, tay đã đóng/tay lạ, cổng đóng, `accept_timeout` 80 ms
  không ai nối. (b) thêm server `TcpListener` **trong thread Rust** để chứng minh `connect`/
  `read`/`write`/`close` của cả hai engine nói chuyện được với tiến trình ngoài; server ghi lại
  đúng `ping`/`bye` client gửi (`["ping","bye","ping","bye"]`).
- **http**: server HTTP/1.1 tối giản trong thread Rust trả theo đường dẫn (`/ok`, `/head`,
  `/nhanh`, `/head-nhanh`, `/cham*`). `HEAD` nhận `Content-Length: 999999` **không body** mà vẫn
  `Ok` (chứng minh không đọc thân). `/cham*` ngủ **300 ms** > `timeout 80 ms` ⇒ `Err` "quá thời
  gian" ở cả hai engine. Cổng đóng (bind rồi thả) ⇒ `Err` dữ liệu. Server trả lời mỗi kết nối ở
  thread riêng để kết nối kế tiếp không bị kernel huỷ khi client kịp thoát.
- **Join có hạn**: mọi server thread dùng accept non-blocking + `stop: AtomicBool` + hạn 8 s;
  test set `stop` rồi `join` ⇒ không treo kể cả khi client lỗi.
- Mỗi nhóm có **≥ 1 ca lỗi** và **≥ 1 assert nội dung thật** (chuỗi stdout đầy đủ, request
  server nhận, byte client gửi) — không xanh rỗng.

## 3. Bằng chứng (chạy trên `ps`, 2026-11-26)

```bash
export PATH=~/.cargo/bin:$PATH
export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0

# Lượt đầu theo chỉ đạo dùng chung /home/ps/ViScript/target: test + clippy + gate đều PASS,
# nhưng worktree chính đang được agent khác build **cùng target dir**; Cargo không phân biệt
# hai cây nguồn ⇒ artifact `lib_api_it` bị ghi đè bởi bản của `main` (42 module, không có
# `misc_api_it`) và `cargo test ... host_parity` chuyển thành "0 tests". Vì vậy **lượt chứng
# cứ cuối** chạy target riêng trong worktree (rẻ: 888 MB, `CARGO_INCREMENTAL=0`):
export CARGO_TARGET_DIR=/home/ps/ViScript-w4/target CARGO_INCREMENTAL=0

cargo fmt --all                                     # OK
cargo test -p vis-conformance --test lib_api_it host_parity -- --test-threads=1
#   running 10 tests ... test result: ok. 10 passed; 0 failed (3 vòng liên tiếp đều xanh)
./vis-dev gate --fast                               # PASS (R1..R9 OK, L0 test OK)
cargo clippy --workspace --all-targets -- -D warnings   # PASS
cargo nextest list -p vis-conformance --test lib_api_it | grep -c host_parity   # 10
```

- `phu_du_66_ham_host`: `StdId` host = **66**, bảng `HOST_66` khớp đúng.
- `moi_ham_host_co_loi_goi_trong_harness`: **0 tên thiếu** lời gọi `tên(`.
- **Cảnh báo target dùng chung**: `main` (hoặc worktree khác) build cùng
  `CARGO_TARGET_DIR=/home/ps/ViScript/target` sẽ ghi đè `lib_api_it` bằng bản của cây đó; Cargo
  không phân biệt hai cây nguồn nên lần chạy sau lặng lẽ dùng binary cũ (triệu chứng: `0 tests`
  hoặc thiếu `host_parity`). Cách chắc ăn: target riêng trong worktree
  (`CARGO_TARGET_DIR=$PWD/target CARGO_INCREMENTAL=0`, ~0,9 GB) hoặc `touch` **ngay trước** mỗi
  lần build/test nếu buộc dùng target chung.

## 4. Caveat / rủi ro

- **Windows**: `fs_symlink_parity` bị `#[cfg(unix)]` (Windows tạo symlink cần đặc quyền) — đúng
  như phần trước. `sqlite`/`http`/`net` không dùng symlink; server thread + loopback chuẩn.
  Thông điệp lỗi mạng/HTTP có thể khác Unix nhưng **parity so giữa hai engine trên cùng máy**,
  nên không phụ thuộc câu chữ; các assert nội dung chỉ đòi `"quá thời gian"` / `Err`.
- **Cổng đóng** trong ca http: bind `127.0.0.1:0` rồi `drop` — lý thuyết có race nếu tiến trình
  khác chiếm đúng cổng trong vài ms (xác suất cực nhỏ, cùng cách các test `http_tu_choi` hiện có).
- **Tệp DB tạm**: mỗi ca một tempdir riêng theo `pid + counter + tag`, tự dọn; chạy song song
  (nextest) an toàn. Không phụ thuộc thứ tự chạy.
- Test cần binary `target/debug/vis` đã build (như phần trước); `gate --fast` build trước khi
  chạy L0 nên không cần bước tay.
- `net.connect("127.0.0.1:1")` trên Windows có thể bị firewall hỏi lần đầu trong môi trường lạ —
  nếu soak Windows gặp chậm, đổi sang cổng "bind rồi thả" như ca http (không đổi ngữ nghĩa).

## 5. Còn lại của `ST-315d`

- Phần (1) `VS0304` + (2) gợi ý `VS0204`: xong ở `JOURNAL/st315d-vs0304.md`.
- Phần (3) này: **66/66** hàm host đã phủ ⇒ `ST-315d` đủ điều kiện `DONE` (chờ Gatekeeper
  nghiệm thu độc lập).
