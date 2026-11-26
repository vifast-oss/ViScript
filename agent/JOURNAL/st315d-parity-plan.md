<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Kế hoạch `ST-315d` phần 3 — parity 66 hàm host (W4)

> Worktree: `/home/ps/ViScript-w4` · nhánh `plan/st315d-parity` · nền `c2b051c9`.
> **Không** sửa `docs/SPEC*.md`, `agent/DECISIONS.md`, `agent/ACTIVE.md`. **Không** dùng máy `mps`.
> Dùng chung target để tiết kiệm đĩa: `export CARGO_TARGET_DIR=/home/ps/ViScript/target`.

## Mục tiêu

Thêm harness parity (`interp ↔ VM`) phủ **66 hàm host** còn ngoài corpus, so **stdout + exit +
mã chẩn đoán + thông điệp**; đặt trong `crates/vis-conformance/tests/misc_api_it/host_parity.rs`
và khai `#[path = "misc_api_it/host_parity.rs"] mod host_parity;` trong
`crates/vis-conformance/tests/lib_api_it.rs` (**0 binary cấp 1 mới — R8 giữ `39 ≤ 40`**).

## 66 hàm (nguồn: `JOURNAL/st314a-hau-kiem.md` §23.2)

- **fs (22)**: `append`, `canonicalize`, `copy`, `current_dir`, `exists`, `glob`, `is_dir`,
  `is_file`, `is_symlink`, `list`, `mkdir`, `mkdir_all`, `read`, `read_bytes`, `read_lines`,
  `read_link`, `remove`, `rename`, `size`, `walk`, `write`, `write_bytes`.
- **sqlite (28)**: `application_id`, `begin`, `changes`, `close`, `columns`, `commit`,
  `database_list`, `data_version`, `execute`, `execute_batch`, `explain`, `index_columns`,
  `indexes`, `index_exists`, `in_transaction`, `journal_mode`, `last_insert_rowid`, `open`,
  `open_with`, `primary_keys`, `query`, `query_one`, `rollback`, `schema_sql`, `schema_version`,
  `table_exists`, `tables`, `user_version`.
- **net (10)**: `accept`, `accept_timeout`, `close`, `close_listen`, `connect`, `connect_timeout`,
  `listen`, `local_addr`, `read`, `write`.
- **http (4)**: `get`, `get_timeout`, `head`, `head_timeout`.
- **time (2)**: `now_ms`, `monotonic_ms` (so **kiểu + bất biến** `>= 0`, không so giá trị tuyệt đối).

## Cách làm đề nghị

1. **Harness trong process** (ưu tiên "parity thư viện"): dùng `vis_interp::run_with_hosts_caps`
   và `vis_vm::run_with_full_hosts_caps`; host thật lấy từ `vis-cli` (nếu thêm dev-dep không tạo
   vòng) hoặc dựng `FsGuard`/`NetGuard`/`DbGuard` từ `vis-host-native` như
   `crates/vis-cli/src/cli/run_cmd.rs:500-550`. Nếu bế tắc, dùng binary
   `env!("CARGO_MANIFEST_DIR")/../../target/debug/vis` làm phương án B, nhưng phải chứng minh
   binary tồn tại dưới `cargo test --workspace`.
2. Mỗi nhóm một `#[test]` (hoặc vài test) chạy **bảng script**; mỗi script in kết quả tất định
   (đường dẫn tempdir đã chuẩn hoá, thứ tự `list`/`walk` sắp xếp, thời gian chỉ `>= 0`).
3. **fs**: tempdir riêng mỗi ca; tạo fixture; ca lỗi (thiếu tệp, quyền) so mã + message.
4. **sqlite**: DB tempdir; phủ DDL/DML/query/metadata/transaction; `open_with`+`journal_mode`;
   `explain` chỉ assert chạy được + parity, không ghim plan.
5. **net**: listener loopback `127.0.0.1:0` trong thread Rust; script `listen`/`accept`/`connect`/
   `read`/`write`/`local_addr`/timeout/close. Tránh treo: timeout ngắn, join có hạn.
6. **http**: server HTTP/1.1 tối giản trong thread Rust (200 + header + body; HEAD không body);
   phủ `get`/`head`/timeout (server trì hoãn > timeout ⇒ lỗi dữ liệu, so parity).
7. **time**: `now_ms()`/`monotonic_ms() >= 0`, kiểu `int`, hai engine cùng bất biến.
8. Ca âm: mỗi nhóm ít nhất một lỗi (file thiếu, SQL sai, connect bị từ chối, HTTP timeout) —
   so **exit + mã + message**.

## Cổng & bàn giao

- `cargo fmt --all`; `CARGO_BUILD_JOBS=3 ./vis-dev gate --fast` **PASS**;
  `cargo clippy --workspace --all-targets -- -D warnings` **PASS**;
  `cargo test -p vis-conformance --test lib_api_it host_parity -- --test-threads=1` xanh.
- Ghi bằng chứng vào `agent/JOURNAL/st315d-parity.md` (số ca, 66/66 phủ, lệnh, exit).
- Commit trên `plan/st315d-parity` với tiền tố `[ST-315d]`; **không** push/merge.
- Báo lại: commit hash, test count, gate, rủi ro/caveat (Windows `.skip` nếu cần).
