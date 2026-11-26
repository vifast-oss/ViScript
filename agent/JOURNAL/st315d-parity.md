<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-315d (phần 3) — harness parity **66/66 hàm host** (`interp ↔ VM`) — bản vá sau review

> **Lát**: `ST-315d` (W4 · Sóng 4) — phần 3/3, **vòng vá R-Parity** · **Worktree**:
> `/home/ps/ViScript-w4` · nhánh `plan/st315d-parity` · **Nền**: `6e062314` (main, đã `merge
> --ff-only`; chứa `7436d7ee`) · **Ngày**: 2026-11-26
> **Trạng thái**: `READY` — vá F1–F5 + caveat rẻ; **66/66** hàm host (Linux), `gate --fast` PASS.
> **Phạm vi**: `crates/vis-conformance/tests/misc_api_it/host_parity.rs` + tệp nhật ký này.
> **Target**: `/home/ps/ViScript-w4/target` — target **riêng** trong worktree,
> `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=3`. Không dùng target
> chung (xem §6), không dùng `mps`.

## 1. Vì sao có vòng vá này

Gatekeeper `R-Parity` kết luận **FAIL (nhẹ)**: không có lệch engine thật, nhưng hợp đồng phủ lỗi
và assert chưa đạt — `fs` **0 ca lỗi**, `net` bỏ `e.message`, 8 hàm `fs` chỉ gọi mà không assert
giá trị, test khoá 66/66 chỉ tìm chuỗi trong cả tệp. Báo cáo:
`/home/ps/ViScript-rparity/agent/JOURNAL/review-st315d-parity.md`.

| Finding | Trạng thái sau vá |
|---|---|
| **F1** `fs` 0 ca lỗi | **Đóng**: `fs_loi_moi_ham_parity` — **17 ca lỗi** in nguyên văn `e.message` |
| **F2** `net` chỉ so nhãn | **Đóng**: mọi nhánh `Err` in `e.message`; timeout/handle xuất hiện trong chuỗi ghim |
| **F3** 8 hàm `fs` không assert | **Đóng**: cả 20 hàm `fs` assert giá trị; nội dung sau `write`/`write_bytes`/`append`/`copy` được đọc lại và so |
| **F4** khoá 66/66 tìm chuỗi cả tệp | **Đóng**: chỉ xét **script thật** trong `SRC_PHU`, bỏ chú thích `#`; có self-test. **Hợp đồng**: mỗi hàm cần ≥ 1 lời gọi thật; xoá **toàn bộ** lời gọi của một hàm ⇒ đỏ, xoá một bản trùng vẫn xanh nhưng test hành vi `fs_20`/nhóm tương ứng sẽ đỏ nếu mất hành vi |
| **F5** journal khai quá mức | **Đóng**: tệp này thay bản cũ |
| **F7** dọn tempdir khi đỏ | **Đóng**: `struct Tam` + `Drop` (RAII) |
| **F8** cổng cố định `127.0.0.1:1` | **Đóng**: bind `:0` rồi `drop` cho mọi ca "cổng đóng" (net + http) |
| **F9** `vis_bin` profile/target tương đối | **Đóng**: tương đối hiểu theo gốc workspace; profile theo `debug_assertions` |
| **F10** chưa đòi exit 0 | **Đóng**: `hai_duong_*` `assert_eq!(c1, 0)` |
| **F11** mốc đồng hồ tuyệt đối 2023 | **Đóng**: chỉ còn bất biến `now_ms > 0`, `monotonic` không giảm |
| **F6** Windows 64/66 | **Ghi rõ**: `fs_symlink_parity` là `#[cfg(unix)]` |
| **F12** server thread 8 s | Giữ nguyên (12 lượt chạy + gate không flake) |

## 2. Harness hiện tại

- **12 `#[test]`** trong `host_parity`; mọi ca chạy **cả hai đường** `vis run` và
  `vis run --vm --no-cache` trên cùng tệp, reset fixture trước **mỗi** đường. `hai_duong_*` đòi
  **exit 0**, stdout/stderr **từng byte** (sau chuẩn hoá khai báo rõ nếu có).
- **66/66 hàm host** được gọi trong **script thật** (`SRC_PHU`), không tính bảng/chú thích:
  | Test | Hàm phủ | Số hàm |
  |---|---|---|
  | `fs_20_ham_parity_tren_tempdir` | 20 `fs` đường thành công, **assert giá trị** | 20 |
  | `fs_loi_moi_ham_parity` | **17 ca lỗi** `fs` (read/size/remove/mkdir/mkdir_all/copy/rename/read_link/canonicalize/list/walk/glob/append/write/read_bytes/read_lines/write_bytes) | (trong 22) |
  | `fs_symlink_parity` (`#[cfg(unix)]`) | `fs.is_symlink`, `fs.read_link` (+ read/canonicalize qua link) | 2 |
  | `time_hai_ham_parity_bat_bien` | `time.now_ms`, `time.monotonic_ms` | 2 |
  | `sqlite_dml_va_giao_dich_parity` | `open_with`, `close`, `execute`, `execute_batch`, `query`, `query_one`, `explain`, `last_insert_rowid`, `changes`, `begin`, `commit`, `rollback`, `in_transaction` | 13 |
  | `sqlite_metadata_parity` | `open`, `application_id`, `data_version`, `schema_version`, `user_version`, `journal_mode`, `tables`, `table_exists`, `columns`, `indexes`, `index_exists`, `index_columns`, `primary_keys`, `schema_sql`, `database_list` | 15 |
  | `net_loopback_parity` | 10 `net` (listen/local_addr/connect/accept/read/write/close/accept_timeout/close_listen/connect_timeout) + 8 ca lỗi in `e.message` | 10 |
  | `net_client_toi_server_rust_parity` | connect/connect_timeout/read/write/close tới server Rust thật | (trong 10) |
  | `http_bon_ham_parity` | `get`, `get_timeout`, `head`, `head_timeout` | 4 |
  | `phu_du_66_ham_host` | `HOST_66` ↔ `StdId::ALL` | khoá 66 |
  | `moi_ham_host_co_loi_goi_trong_harness` | 66 tên phải có lời gọi trong `SRC_PHU` (đã bỏ chú thích) | khoá 66 |
  | `bo_chu_thich_vis_khong_tinh_lenh_bi_comment` | self-test bộ bóc chú thích | — |
- **Ca lỗi từng nhóm** (trừ `time` — chỉ bất biến `>= 0`, không có nhánh lỗi):
  - `fs`: 17 ca, Unix ghim **toàn bộ** thông điệp sau khi thay tempdir bằng `<W>` (ví dụ
    `read=không đọc được `<W>/khong-co.txt`: No such file or directory (os error 2)`,
    `append=… Is a directory (os error 21)`); nền tảng khác đòi 17 nhãn + thông điệp chứa `<W>`.
  - `net`: **các nhánh lỗi** in **nguyên văn** `e.message` (listen trùng cổng, `accept_timeout` 80 ms,
    đọc/ghi sau `close`, `close_listen` lần 2, `accept`/`local_addr` sau đóng, tay `C99`, connect
    bị từ chối). Cổng ephemeral chuẩn hoá thành `127.0.0.1:<P>`; phần còn lại ghim từng byte.
  - `sqlite`: SQL sai, thiếu/thừa tham số, `begin` lồng, `commit` ngoài transaction, handle sau
    `close`, `open_with(create=false)` thiếu tệp — ghim nguyên văn thông điệp.
  - `http`: timeout 80 ms < server trễ 300 ms; cổng đóng bind-rồi-thả.
- **`explain`**: chỉ assert chạy được — **có** kiểm 4 **tên cột** `[id, parent, notused, detail]`
  và `rows > 0`, **không** ghim nội dung plan (đính chính F5).
- **0 binary cấp 1 mới** (R8 = 39 ≤ 40); tệp khai `#[path]` ở `lib_api_it.rs:81-82`; 0 `unsafe`.

## 3. Bằng chứng (chạy trên `ps`, target riêng)

```bash
export PATH=~/.cargo/bin:$PATH
export CARGO_TARGET_DIR=/home/ps/ViScript-w4/target
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=3

cargo fmt --all && cargo fmt --all -- --check          # OK
for i in 1..5: cargo test -p vis-conformance --test lib_api_it host_parity -- --test-threads=1
#   cả 5 lượt: 12 passed / 0 failed (1,46–1,53 s)
for i in 1..2: cargo test -p vis-conformance --test lib_api_it host_parity -- --test-threads=8
#   cả 2 lượt: 12 passed / 0 failed (0,66–0,68 s)
./vis-dev gate --fast                                  # PASS (check + L0: 3854 test, 12/12 host_parity)
cargo clippy --workspace --all-targets -- -D warnings  # PASS
```

Kiểm **hàng rào F4** (mutation tạm, đã khôi phục):

- Bọc **cả 3** lời gọi `fs.read_bytes(` bằng chú thích `#` trong script ⇒
  `moi_ham_host_co_loi_goi_trong_harness` **đỏ** (`thiếu lời gọi host … ["fs.read_bytes"]`).
- Đổi `net.accept_timeout(l, 80)` → `100` và `net.read("C99", …)` → `"C98"` ⇒
  `net_loopback_parity` **đỏ** (chuỗi ghim `sau 80 ms` / `C99` khác đi) — chứng minh F2 bắt được
  lệch tham số timeout/handle.
- Dọn tempdir: sau 1 lượt `--test-threads=8`, `/tmp/vis-host-parity-*` còn **0** (RAII).

## 4. Ảnh hưởng / tương thích

- Không đổi mã sản phẩm; chỉ test + journal. Không sửa `docs/SPEC*`, `DECISIONS.md`, `ACTIVE.md`,
  `QUEUE.md`, `STATE.md`.
- Không push/merge.

## 5. Caveat

- **Windows chỉ 64/66**: `fs_symlink_parity` là `#[cfg(unix)]` (tạo symlink cần đặc quyền), nên
  `fs.is_symlink`/`fs.read_link` không chạy trên Windows. Bảng `HOST_66` và test khoá script vẫn
  đủ 66 (script có mặt, chỉ test chạy bị cfg). Các nhóm còn lại chạy đủ.
- **Thông điệp lỗi theo OS**: thông điệp `fs`/`net`/`http` có đuôi OS (`os error N` /
  “Connection refused”); phần ghim **đầy đủ** chỉ bật `#[cfg(unix)]`. Trên nền tảng khác, harness
  vẫn so **từng byte giữa hai engine** + đòi nhãn/đường dẫn/cổng thật, nên vẫn bắt lệch engine.
- **Cổng đóng** dùng bind `:0` rồi `drop`: race lý thuyết nếu tiến trình khác chiếm đúng cổng
  trong vài ms (xác suất cực nhỏ, cùng cách test `http_tu_choi` hiện có).
- **Đồng hồ**: chỉ kiểm `now_ms() > 0`, `monotonic_ms()` không giảm; không ghim mốc 2023.
- **Server thread**: hạn 8 s + `stop`; 5×1 + 2×8 + gate nextest không flake. Nếu máy cực tải,
  ưu tiên nới hạn (không đổi hợp đồng).
- **`vis_bin`** tìm `<CARGO_TARGET_DIR>/<debug|release>/vis`; `cargo test` trực tiếp cần binary đã
  build (`gate --fast` build trước). Profile lấy theo `debug_assertions`.
- **Target riêng là bắt buộc**: hai worktree dùng chung `CARGO_TARGET_DIR` bị Cargo ghi đè
  `lib_api_it` (bản của cây này đè cây kia; triệu chứng `0 tests`). Dùng
  `CARGO_TARGET_DIR=/home/ps/ViScript-w4/target`; nếu buộc dùng target chung thì `touch` tệp test
  ngay trước mỗi lần build/test.

## 6. Kết luận

- **66/66** hàm host (Linux) có ca chạy thật; **12 test**; mỗi nhóm có ca lỗi in thông điệp thật;
  8 hàm `fs` trước đây chỉ gọi nay đã assert giá trị/nội dung.
- Sẵn sàng để Gatekeeper nghiệm thu lại; phần còn lại của `ST-315d` (mã `VS0304`, gợi ý `VS0204`)
  đã xong ở `JOURNAL/st315d-vs0304.md`.
