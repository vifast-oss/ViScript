# Ma trận target core / SQLite — 2026-11-20

**Căn cứ**: VD-242 Q6 (`agent/DECISIONS.md`) + §4 “SQLite/target”; ST-231/#156
(`agent/QUEUE.md`, `agent/SO-NO.md`). **Người ghi**: Agent 2 (ST-156).

SQLite là **feature tùy chọn** của `vis-cli`. Hai cấu hình dưới đây được đặt tên riêng;
**không** đổi đỏ thành xanh bằng cách đổi cấu hình rồi giữ nguyên nhãn. Kết quả lịch sử
**16/18** vẫn thuộc cấu hình **sqlite (cũ)**; cấu hình **core** được đo riêng.

## 1. Hai cấu hình

| Cấu hình (nhãn in ra) | Cargo flags | Kết quả đo (18 ô) | Ghi chú |
|---|---|---|---|
| `core (vis-cli --no-default-features)` | `--no-default-features` | **PASS=18 · EXPECTED_RED=0 · FAIL=0** | Không kéo `rusqlite`/`libsqlite3-sys`; msvc + wasm32 đều CHECK xanh. |
| `sqlite (vis-cli default, cấu hình cũ)` | mặc định | **PASS=16 · EXPECTED_RED=2 · FAIL=0** | Giữ nguyên 16/18 lịch sử; 2 ô `vis-cli` × (`x86_64-pc-windows-msvc`, `wasm32-unknown-unknown`) đỏ đúng nợ #156. |

Mỗi cấu hình gồm 8 crate × 2 target + 2 crate phụ (`vis-host-wasm`, `vis-host-linux`).
`./bin/vis-targets` in hai bảng có tên tương ứng, exit **0** khi cả hai đạt kỳ vọng riêng.

## 2. Lệnh chạy

```bash
export PATH="$HOME/.cargo/bin:$PATH"

./bin/vis-targets                                  # in cả hai bảng; exit 0 ở baseline này
cargo check --workspace --no-default-features       # core: biên dịch không SQLite
cargo check -p vis-cli                              # sqlite: cấu hình cũ (mặc định)
./vis-dev check && ./vis-dev gate --fast            # cổng luật + L0
./vis test test/sem && ./vis test test/sem --vm     # hai engine
```

Dựng tay một bản core để thử chẩn đoán:

```bash
cargo build -p vis-cli --no-default-features
./target/debug/vis run test/sem/sqlite/mo-dong.vis
# => loi `std.sqlite` cần feature `sqlite` của `vis-cli`; dựng lại với `--features sqlite`
```

## 3. Phạm vi bằng chứng: CHECK ≠ chạy

- Bảng chỉ là **`cargo check --target` biên dịch chéo**; **không** chạy chương trình trên
  Windows hay wasm32. Ô xanh của `std::process`/`std::net`/`std::fs` trên wasm32 chỉ có nghĩa
  là **biên dịch được** (`wasm32-unknown-unknown` không có tiến trình/socket/FS).
- macOS **chưa verify** (VD-242 Q7); Windows giữ **Tier 2**. Không suy ra hỗ trợ runtime từ
  bảng này.
- Hai ô đỏ của cấu hình sqlite được in là `EXPECTED_RED` kèm lý do, không được gộp vào FAIL
  và cũng không được đổi nhãn thành “PASS core”.

## 4. Nợ #156 — trạng thái sau ST-156

| Trước | Sau |
|---|---|
| `vis-targets` 16/18, chỉ đo + ghi nợ, không sửa mã/script/kỳ vọng (ST-231). | Vẫn **16/18 ở cấu hình sqlite (cũ)**; script giờ in `EXPECTED_RED` + lý do thay vì chỉ `FAIL`, và **bổ sung** bảng core 18/18. |

- **#156 vẫn mở** (không đóng): nguyên nhân C toolchain của `libsqlite3-sys` (bundled) trên
  msvc/wasm32 chưa được giải quyết; đợt này chỉ **tách cấu hình + công bố ma trận** theo Q6.
- Giới hạn/zero-params/read-only/authorizer/tag… của SQLite MVP (VD-241, VD-242 Q16) **không đổi**;
  `rusqlite = "=0.37.0"`, `bundled`, `hooks` giữ nguyên. Không đổi VERSION/opcode/freeze API.

## 5. Từ chối `std.sqlite` khi tắt feature

`crates/vis-cli/src/db/` tách thành hai nhánh cùng chữ ký: `sqlite/` (nguyên vẹn VD-241) và
`disabled.rs` (core). Ở core, `Db`/`DbGuard` vẫn triển khai `DbHost` nhưng **mọi** API trả một
`Err(String)` có nội dung:

```text
`std.sqlite` cần feature `sqlite` của `vis-cli`; dựng lại với `--features sqlite`
```

`hop_nhat_fs` của stdlib đổi thông điệp đó thành `Result.Err(Error)` dữ liệu — **không** panic,
**không** trả handle giả hay kết quả giả; `main.rs` không phải rẽ nhánh. Cả đường interp và
`--vm` đều đã kiểm chứng bằng `test/sem/sqlite/mo-dong.vis`.

Integration test `crates/vis-cli/tests/sqlite.rs` gọi trực tiếp `rusqlite::Connection` để tạo
WAL thật — đã khai báo `[[test]] required-features = ["sqlite"]` trong `Cargo.toml` để
`cargo test --no-default-features` không cố biên dịch nó (các integration test khác giữ nguyên
chế độ tự phát hiện). Cấu hình mặc định vẫn chạy target này như cũ.
