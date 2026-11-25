# ST-314a — Hậu-kiểm `VD-290` (W1/W4)

> **Trạng thái**: `REVIEW` · **Ngày**: 2026-11-25 · **Phụ thuộc**: `ST-313d` / `VD-290`
> **Commit lát**: `311d2288` (mã + test) · **Phạm vi**: 4 phát hiện kiểm toán của Agent 0
> + 1 lỗi cổng phát sinh khi chạy lại `gate --fast` trên `d8857694`.

## 1. Vá lỗi thật `bin/vis-wasm-conformance` (chạy thật 2 engine)

**Lỗi**: `NODE_DRIVER` destructure `[wasmPath, guest, host, ro, ...args]` nhưng Python không truyền
`ro`; chuỗi `--interp`/`--vm` bị gán vào `ro` rồi bỏ ⇒ harness chạy **cùng engine mặc định
(interp) 2 lần**, chưa từng kiểm `--vm`.

**Vá**:
- Bỏ `ro` khỏi destructure; `VIS_WASM_PREOPENS` chỉ còn `guest` (corpus này không dùng preopen ghi).
- `chay(driver, root, *runner_args)` chuyển tiếp **nguyên văn** đối số runner; mỗi ca chạy
  `--interp /data/prog.vis` và `--vm /data/prog.vis`.
- Thêm **self-check** `tu_kiem`: chạy runner với cờ lạ `--co-khong-ton-tai`; `vis-wasm-run` phải
  từ chối `exit 2`. Nếu cờ bị nuốt như lỗi cũ, self-check fail ngay.
- Thêm `argparse` cấp harness: cờ lạ của chính script cũng `exit 2`.

**Đo thật** (log `/home/ps/vislogs/st314a-wasm-conformance.log`):

```text
# 299 ca hợp lệ trên WASI
# tổng: 299 ca x 2 engine, 0 hỏng
EXIT=0
sha256 567b75c5f4619c18d673157443791ab5673dfae738058fdf3d42f1f983bc701e
```

## 2. Hygiene workspace & R8

- `Cargo.toml`: thêm tường minh `"crates/vis-hir"` vào `[workspace].members`;
  `cargo metadata --no-deps` = **23** workspace member (không đổi tổng, chỉ hết gộp ngầm).
- `crates/vis-core/Cargo.toml`: bỏ `"HTTP client"` khỏi `description` (HTTP đã ở `vis-http`).
- `bin/vis-dev` R8: tổng binary `.rs` cấp 1 = `crates/*/tests/*.rs` **+** `vis-host-linux/tests/*.rs`;
  hiện **38 + 1 = 39 ≤ 40**; `crates/vis-cli/tests` = **8 ≤ 15**.

## 3. `StdId` compile-time exhaustive 100%

- `crates/vis-runtime/src/stdlib.rs`: bỏ hẳn guard `starts_with("path.")` / `starts_with("fs./net./http.")`
  và **bỏ nhánh `_ =>`** cuối `call_by_id`. Match chính phủ đủ **167/167** biến thể `StdId`;
  thêm biến thể mới mà quên route ⇒ **lỗi biên dịch**.
- `stdlib/path/dispatch.rs` + `stdlib/fs_net/dispatch.rs`: nhận `StdId` (không nhận `&str`),
  match theo biến thể enum; tên chỉ dùng cho thông điệp lỗi (`id.as_str()`). Không còn
  `match name` chọn hàm, không còn `starts_with`.
- Hai module con giữ nhánh `_ => unreachable!("...")` **phòng vệ** cho biến thể ngoài miền
  (không thể tới từ top-level); đây không phải fallback im lặng.

## 4. Sửa đỏ cổng phát sinh từ đồng bộ tài liệu `VD-290`

`gate --fast` lần 1 đỏ ở `vis-types::types_it::hir::ha_15_khoi_tai_lieu_0_chan_doan_va_dung_so_khai_bao`
(`khối dòng 1480 chưa có số kỳ vọng`): A0 đồng bộ bảng crate §14 gọn **1 dòng** (net −1) làm hai
khối `vis` cuối dịch dòng, trong khi `DOC_BLOCKS` còn ghim 1481/1528.
Đã cập nhật `crates/vis-types/tests/types_it/hir.rs`: **1481 → 1480**, **1528 → 1527**.

## 5. Cổng đo (Chế độ Phát triển `VD-289`)

| Cổng | Kết quả | Bằng chứng |
|---|---|---|
| `./vis-dev gate --fast` | **PASS** — 3804/3804 test (4 skip), 0 fail | `/home/ps/vislogs/st314a-gate-fast-2.log`, sha `5a4ce999126aaaae744e80f10e92af3d2c9dbb1fc78fbb37ea4e041de91df44c` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** — 100.000 chương trình, 93.768 chạy, 6.232 lỗi tĩnh, seed `3523158054`, 62,7 s | `/home/ps/vislogs/st314a-vis-diff.log`, sha `4d23935231a7bf5a230706e0fae375fff23fbcaf4e7fb344dee25ad994347125` |
| `./bin/vis-targets` | **PASS** — cả hai cấu hình đạt kỳ vọng riêng | `/home/ps/vislogs/st314a-vis-targets.log`, sha `e891b3ed5e5c64320a41a4c320721b1d50ea982ef01a50283782befb029bd593` |
| `vis-wasm-conformance` | **299 × 2 engine, 0 hỏng** (self-check cờ lạ OK) | `/home/ps/vislogs/st314a-wasm-conformance.log`, sha `567b75c5f4619c18d673157443791ab5673dfae738058fdf3d42f1f983bc701e` |
| clippy Linux (trong `gate --fast`) | **PASS** `--workspace --all-targets -D warnings` | gate log |
| clippy Windows `psvn` @`311d2288` | **PASS** — `vis-remote-exit=0` | `/home/ps/vislogs/st314a-clippy-psvn.log` |
| clippy macOS `mps` | **BỎ QUA theo chỉ đạo người dùng** — `mps` đang dùng cho dự án khác | ghi nợ bằng chứng macOS |

## 6. Tệp thay đổi

```text
bin/vis-dev
bin/vis-wasm-conformance
Cargo.toml
crates/vis-core/Cargo.toml
crates/vis-runtime/src/stdlib.rs
crates/vis-runtime/src/stdlib/fs_net/dispatch.rs
crates/vis-runtime/src/stdlib/path/dispatch.rs
crates/vis-types/tests/types_it/hir.rs
```

## 7. Bổ sung bằng chứng khung cao điểm (0-LLM, `VD-287`)

Sau khi nộp `REVIEW`, chạy tiếp các việc nặng 0-LLM tới sát giờ thấp điểm:

| Việc | Kết quả | Bằng chứng |
|---|---|---|
| Windows full @`5d857b84` (`vis-gate-windows --full`) | `cargo test --workspace`: **3.729 passed / 0 failed / 4 ignored** (90 binary); `host_conformance` **54 ca / 120 lượt, 0 lệch, VM 54/54**; `vis-targets` **23 crate** PASS (core 47/0/0, sqlite 46/1 EXPECTED_RED/0); `vis-dag-check` OK | `/home/ps/vislogs/st314a-win-full.log`, sha `84f1593a7d3bf45454d8b060face61e262dd2b2e2b30a2abf24f958d1fc2cead` |
| 5× `vis-diff 100k` (seed `20261125/101/202/303/404`) | cả 5 **lech=0** (93.767–93.771 chương trình chạy/lượt) | `st314a-diff-<seed>.log` |
| +3× `vis-diff 100k` (seed `505/606/707`) | cả 3 **lech=0** | `st314a-diff-<seed>.log` |
| Fuzz parser 115 phút (`vis-dev nightly --skip-gate`) | **959.027.870 ca / 21.766.452.334 token, 0 vi phạm**; vòng tròn fmt 475 tệp bất động | `/home/ps/vislogs/st314a-fuzz-115m.log`, sha `b145989a698f5cc5da77b0542faceddb0b6bda55f87321179a5c82bc6fec92c4` |
