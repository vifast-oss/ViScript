<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-311f — soak 4 h Windows/macOS: race shim `vis` ↔ `cache_khong_che`

## 1. Triệu chứng

- **Windows** soak `gate-1` (`H3`): `lat0_suspend::baseline_matches_current_vis` đỏ 15,6 s:
  `error: failed to remove file C:\...\target\debug\vis.exe` — `Access is denied. (os error 5)`.
- **macOS** soak `gate-3` (`H3`): `cache_khong_che::ba_che_do_520_tep_khong_lech` đỏ 4,04 s:
  `chạy vis: Os { code: 2, kind: NotFound }` (ENOENT).

## 2. Nguyên nhân chung

`lat0_suspend::baseline_matches_current_vis` gọi **shim** `repo/vis`; shim chạy
`cargo run -q ... --bin vis`. Trong lúc nextest chạy song song, cargo **relink**
`target/debug/vis`:

- Windows: không xoá được `vis.exe` đang được test khác chạy ⇒ `Access is denied`;
- macOS/Linux: cửa sổ unlink/relink làm `Command::new(target/debug/vis)` của
  `cache_khong_che` nhận ENOENT.

Đây là race **harness/test**, không phải lỗi sản phẩm; full gate (`cargo test`) có thể lọt vì
thứ tự chạy khác, soak mới bắt được.

## 3. Vá

`crates/vis-interp/tests/frame_it/lat0_suspend.rs`: dùng **binary đã build**
(`$CARGO_TARGET_DIR`/`target` + `debug/vis[.exe]`) thay vì shim; bỏ nhánh `bash` trên Windows;
thêm assert báo rõ nếu thiếu binary. Không đổi hành vi sản phẩm, không đổi kỳ vọng `42\n`.

Đã kiểm: `cargo test -p vis-interp --test frame_it baseline_matches_current_vis` → PASS;
`cargo fmt --check` → PASS.

## 4. Hệ quả

`H4` = commit vá; ma trận 3-OS + watchdog + soak phải chạy lại trên `H4`. `H4` chỉ đổi **test**;
parser fuzz đang chạy dùng binary `vis-syntax` không đổi — ghi rõ khi tổng hợp R37.
