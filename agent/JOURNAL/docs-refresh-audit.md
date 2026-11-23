<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-305 — rà & cập nhật tài liệu mô tả hiện trạng (docs-only)

> **Nhánh**: `plan/docs-refresh` từ `c3b7290` (merge `ST-300m1`). **Không** merge/push/tag, **không**
> ssh `mps`/`psvn`, **không** chạy build/gate nặng (nhường `ps` cho `ST-300m2`).
> `TMPDIR=/home/ps/tmp`. Phạm vi: tài liệu **mô tả hiện trạng**; **không** sửa lịch sử/`reports`/JSON/code.

## 1. Nguồn đối chiếu (đã kiểm)

| Dữ kiện | Bằng chứng |
|---|---|
| Tag `v0.4.5` @ `3daff62`, `v0.4.1` @ `90844bc`, `v0.4.0` @ `c2accdf` | `git rev-parse v0.4.x^{commit}` (tag annotated; object tag khác commit) |
| Version workspace **0.4.5** | `Cargo.toml` `[workspace.package] version = "0.4.5"`; crate dùng `version.workspace = true` |
| 12 crate trên `main` | `ls crates/`: `vis-diag`, `vis-syntax`, `vis-types`, `vis-interp`, `vis-bytecode`, `vis-vm`, `vis-fmt`, `vis-cli`, `vis-host-native`, `vis-pkg`, `vis-loader`, `vis-lsp` |
| 4 crate chưa merge | `git ls-tree` các nhánh: `vish-edit` (`plan/w3-st304-vish`, `plan/w3-st304b-vish`), `vish-term` (`plan/w3b-st308-term`), `vis-plugin-abi` + `vis-plugin-macros` (`plan/w4-st306-plugin-abi`, `plan/w4b-st307-plugin-wasi`, `plan/w4b-st307-win-tests`) |
| Windows Tier 1 | `agent/reports/R35-viscript.md` §2 (`ST-298`: 3× full 3611/0/3, soak 985/0); `HOST-BAO-DAM` khối `VD-283` |
| macOS verify `v0.4.5`, **chưa** Tier 1 | `agent/JOURNAL/mac-v045-verify.md` (gate 3631/3, workspace 3636/0/3, host 54/54, WASI 28/28, wasm-sem 388/390); `JOURNAL/quyet-dinh-a0-2026-11-23-vd284.md` §3 (m3 mới công bố) |
| `ST-300m1` đã merge; m2/m3 chờ | merge `c3b7290`; `JOURNAL/st300m1-macos.md`; nhánh `plan/w2-st300m2-mac-bench` trỏ đúng `c3b7290` (chưa có commit riêng) |
| WASI runtime | `bin/vis-wasm-sem`, `bin/vis-wasm-suite`; `bin/vis-targets` dòng `wasm32-wasip1 (runtime)` 388/390 + 28/28 |
| CLI hiện có | `crates/vis-cli/src/main.rs` (`run`, `check`, `fmt`, `test`, `doc`, `explain`, `lsp`, `pkg`, `new`, `repl`, `doctor`, `cache`, `version`) |
| `vis-cli` ~4,8k LOC | `wc -l crates/vis-cli/src/main.rs` 228 + `lib.rs` 15 + `vistest.rs` 1249 + `cli/*.rs` 3325 = 4817 (không có 21k trong tài liệu đã rà) |
| `SO-NO` §8 hiện tại | 194 = 85 mở / 90 đóng / 16 VS3 / 2 không làm / 1 điều hướng (khớp `ST-300m1` đóng `#137`) |

## 2. Tệp đã sửa & đoạn chính

### `README.md`
- Bảng “Trạng thái ngôn ngữ”: thay dòng “Windows hỗ trợ đầy đủ 🔜 VS3” bằng 3 dòng
  **Windows x86_64 Tier 1** (`v0.4.5`, caveat `FsGuard`), **macOS arm64 đã verify `v0.4.5` nhưng
  chưa công bố Tier 1**, **WASI Preview 1 có runner** (388/390 eligible, suite 28/28).
- Thêm “Bản hiện tại: `v0.4.5` (tag @ `3daff62`)”.
- Bảng lệnh: thêm `--json-out`, `--slowest` cho `vis test`; thêm `implementation` cho LSP; thêm
  `vis new`, `vis doc`, `vis repl`, `vis doctor`, `vis cache`; mở rộng `vis pkg` đủ
  `add|install|update|list|remove|gc|migrate`.
- “Cấu trúc repo”: 8 → **12 crate**; liệt kê thêm `vis-gate-{macos,windows}`,
  `vis-wasm-{sem,suite}`, `vis-lock`; ghi chú 4 crate đang ở nhánh riêng, chưa merge.

### `bin/README.md`
- Thêm `vis-gate-macos` (`--quick|--full|--dry-run|--self-test`), `vis-wasm-sem`, `vis-wasm-suite`.
- Cập nhật mô tả `vis-targets` (dòng `wasm32-wasip1 (runtime)`, `aarch64-apple-darwin (native)`).
- Thêm helper nội bộ `_vis-toolchain.sh` (trước đây thiếu trong bảng).

### `AGENTS.md`
- `./vis-dev check` ghi **R1..R7** (khớp header `bin/vis-dev`; trước là R1..R4).

### `agent/ACTIVE.md`
- Cập nhật mốc `ST-300m1` merge `c3b7290`; GĐ 3 **4/7** (thêm m1: errno Darwin, `VS0604` symlink
  APFS, `vis-gate-macos`, release/targets arm64, gate Linux 3672/4, đóng `#137`).
- macOS: giữ số verify `v0.4.5` 3631/3…, thêm `vis-gate-macos --quick` PASS; **chưa công bố Tier 1**
  (chờ m3).
- W2: m1 **DONE/merged**, m2 **READY**; W4: **BLOCKED chờ m2** (trước ghi chờ m1).
- `SO-NO`: 194 = 85/90/16/2/1 (khớp §8; trước ghi 85/89/17).

### `agent/STATE.md`
- Báo cáo mới nhất `R33` → **`R35`**.
- Khối mốc: `v0.4.5` @ `3daff62` (Windows Tier 1 + WASI), `v0.4.1` @ `90844bc`, `v0.4.0` @ `c2accdf`;
  macOS đã verify `v0.4.5`, **Tier 1 chưa công bố**; m1 đã merge, m2/m3 chờ; 4 crate còn ở nhánh.

### `agent/ROADMAP.md`
- Thêm khối **“TRẠNG THÁI HIỆN HÀNH — 2026-11-23”** ngay sau tiêu đề: tag/version, Windows Tier 1,
  macOS verify nhưng **chưa Tier 1**, WASI runner, 12 crate + 4 crate nhánh, công cụ mới.
- Ghi rõ các khối dated bên dưới là **lịch sử** (VD-142).
- Chú thích tại chỗ cho 2 câu “macOS ngoài phạm vi” (VD-271) rằng `VD-284` đã đưa macOS vào phạm vi;
  không xoá nội dung lịch sử.

### `agent/HOST-BAO-DAM.md`
- Thêm **ĐÍNH CHÍNH 2026-11-23** đầu tệp: macOS verify `v0.4.5` (chưa Tier 1), Windows Tier 1,
  WASI runner, `vis-host-native`; nói rõ các câu “macOS không có máy/chưa verify”, “wasm chưa có
  runner” bên dưới **hết hiệu lực**.
- Sửa các khối còn mang tính hiện trạng: ST-295 (Windows Tier 1), ST-289 (đánh dấu lịch sử),
  đoạn giới thiệu 20/11 (đánh dấu lịch sử), bảng nền tảng (tách Windows/wasm, Windows Tier 1,
  WASI runtime), ma trận `cargo check` (thêm runner runtime), bộ host suite (Linux/Windows/macOS
  54/54; wasm ngoài 54 ca), “Cấm ngôn từ”, bảng FsGuard (macOS đã verify; sửa đường dẫn
  `vis-host-native`), mục Windows (Tier 1 nhưng caveat `FsGuard`), bảng Landlock (macOS verify,
  Windows có runner), kết luận macOS (Tier 1 chờ m3).
- Giữ nguyên các khối/đoạn dated là lịch sử; không sửa số cũ.

### `agent/DECISIONS-INDEX.md`
- Số mục định nghĩa **271 → 273** (đã có `VD-283`/`VD-284`); ghi “cập nhật tới `VD-284`”.
- Ghi rõ dải `VD-230..VD-240` không có mục định nghĩa riêng (một số số chỉ được nhắc trong văn bản).

### `docs/ViScript-thiet-ke.md`
- §14.2: thêm ghi chú **hiện trạng crate** (12 crate trên `main`, 2 workspace `vis-host-*`, 4 crate
  chưa merge và nhánh tương ứng).
- §16: thêm ghi chú **lệnh hiện có ở `v0.4.5`** (chưa có `vis add`/binary `vish`).
- Phụ lục C.2: cập nhật ngày; lỗi chữ ký nay là **`VS0708`** (ST-288), bỏ câu “đề xuất/chưa cấp”;
  sửa danh sách “chưa có” (chữ ký Ed25519 đã có; còn thiếu sổ hash/proxy vận hành).
- “Mức hiện thực VS2”: đồng bộ **0.4.5 / HEAD `c3b7290`**, macOS chưa Tier 1, 4 crate chưa merge;
  sửa bullet **Quyền** (đường ghi đã đóng bằng `openat`/`openat2`, macOS verify) và bullet **Nền tảng**
  (Windows Tier 1, macOS verify chưa Tier 1, WASI runner).

### `docs/SPEC-lex.md`
- **Không sửa.** Đặc tả từ vựng đã APPROVED; các tính năng đang ở nhánh `ST-302`/`ST-304`/`ST-305`
  không đổi từ vựng. Không thấy câu mô tả trạng thái bị cũ trong tệp.

## 3. Kiểm nhẹ

- `python3 test/report/check_report.py --all` → **PASS (34 báo cáo, 333 hàng QUEUE phủ đủ)**.
- `git diff --check` → **sạch**.
- **Không chạy** `./vis-dev gate --fast`/`check`/build: `ps` cần rảnh cho `ST-300m2`; thay đổi
  docs-only, không đụng code/test.

## 4. Còn nghi ngờ / cố ý không sửa

1. **`agent/QUEUE.md`**: `ST-300m1` vẫn ghi `READY`, `m2`/`m3` ghi `BLOCKED` chờ m1 — đã lỗi thời
   (m1 merge `c3b7290`). Theo lệnh, **không sửa QUEUE**; cần Agent 0/Agent 1 cập nhật trạng thái.
2. **`agent/SO-NO.md`**: §8 đúng 194 = 85/90/16/2/1, nhưng dòng header “Cập nhật 2026-11-23” còn
   ghi **192 = 84/87/18**; không sửa (lệnh cấm), chỉ ghi nhận.
3. **`VD-284`** có tiêu đề “Mở Chính thức Hỗ trợ macOS arm64 (`mps` Tier 1)”, trong khi §3 yêu cầu
   `ST-300m3` mới tuyên bố Tier 1. Tài liệu đã sửa theo **hiện trạng**: đã verify nhưng **chưa**
   công bố Tier 1. Cần A0 xác nhận cách gọi nếu muốn tránh lệch tiêu đề quyết định.
4. **`agent/ROADMAP.md`** còn nhiều khối lịch sử ghi “macOS chưa verify/không có máy” (R27, VD-270…),
   và dòng `macOS : CHƯA verify` trong khối số đo 21/11. Giữ nguyên theo VD-142; khối đầu tệp đã
   nói rõ các khối dưới là lịch sử.
5. **`agent/HOST-BAO-DAM.md`** còn đoạn ST-289 ghi `version = "0.3.6"`, và một số câu lịch sử
   20/11; đã đánh dấu “khối này là lịch sử”/đính chính đầu tệp, không xoá số.
6. **`docs/ViScript-thiet-ke.md` §11.4** là bảng **thiết kế** (`macOS: Runtime kiểm [GT]`), không phải
   mô tả hiện trạng — không sửa.
7. `README.md` không có câu “vis-cli 21k LOC”; con số 4,8k trong `ACTIVE.md` khớp `wc -l` phần
   `src` không tính trùng. Không tìm thấy “21k” trong các tệp thuộc phạm vi rà.
8. Số dòng trong `DECISIONS-INDEX` là số của lần sinh tự động; không sinh lại toàn bộ (ngoài phạm vi
   docs-only), chỉ sửa **số lượng** và ghi chú cập nhật.

> **Đính chính 2026-11-23**: AGENTS.md giữ **R1..R4** theo chỉ đạo hiện hành; dòng R1..R7 đã trả lại R1..R4.
