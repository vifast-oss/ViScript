<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-309 (phần SPEC) — `docs/SPEC.md` tự chứa

**Nhánh**: `plan/st309-spec` từ **`c4337ce`** (`main`, sau `ST-300m3`). **Không** merge/push/tag,
không ssh `mps`/`psvn`. Worktree riêng `/home/ps/tmp/st309-spec-wt` vì worktree chính
`/home/ps/ViScript` đang được nhánh ST-309 security dùng đồng thời; 0 đụng mã sản phẩm.

## 1. Phạm vi

Chỉ **phần SPEC** của ST-309: xuất `docs/SPEC.md` hoàn chỉnh, tự chứa, mô tả **hành vi đã merge
trên `main` tại `c4337ce`**; không mô tả nhánh chưa merge, không làm kiểm toán bảo mật/soak/tag.
Các mục chưa đặc tả được ghi tường minh ở §12 của SPEC (không suy đoán).

## 2. Nội dung đã xuất

`docs/SPEC.md` (1.177 dòng) gồm:

1. Tổng quan, thứ tự nguồn sự thật, quy ước vị trí (byte/scalar/NFC).
2. Từ vựng: bảng token 25 kind, 40 từ khoá, 39 dấu câu, literal, escape/nội suy, NEWLINE/INDENT/
   DEDENT, chế độ lệnh.
3. EBNF đầy đủ: file/item/khai báo, khối/câu lệnh, biểu thức + 12 mức ưu tiên, pattern, kiểu,
   chế độ lệnh; ghi chú lookahead và chỗ parser/tyck khác thiết kế.
4. Hệ kiểu/ngữ nghĩa tĩnh: kiểu dựng sẵn, record/enum/alias/newtype, suy kiểu, optional `T??`/
   `T???`, `Result`/`?`, `match` phủ, bảng số học R7/R10, hàm/closure/generic/trait/derives,
   phạm vi/bất biến, ngữ nghĩa giá trị.
5. Async/`Task`/`await`/`spawn`/kênh, ngân sách 40M (`VS0510`), worker `--workers` (`VS0509`).
6. Quyền `cap[...]`: mô hình, `@`/`(write)`/`--grant`, `--strict-caps`, `--sandbox`,
   Landlock/FsGuard, bảng mức bảo đảm Linux/Windows/macOS/WASI.
7. Chẩn đoán: định dạng, **toàn bộ 56 mã** `VSxxxx` (đối chiếu `agent/API-CODES.txt`), mã thoát.
8. Module/`vis pkg`/10 module std (167 hàm đóng băng, liệt kê theo nhóm).
9. CLI `run/check/test/fmt/pkg/cache/plugin/new/doc/repl` + `lsp/doctor/version/explain`.
10. ABI plugin tĩnh `VIS_PLUGIN_ABI_V1` và động WASI (`wasm32-wasip1`, handshake, manifest,
    codec, sandbox, giới hạn).
11. Nền tảng + bộ kiểm + cách kiểm chứng bản SPEC.
12. **Danh sách phần chưa đặc tả đầy đủ / chỉ có thiết kế** (không bịa): `ref`/`weak`, `set`,
    `stream`/`Future`, `dyn`, `datetime`/`instant`, `scope` thật, `chan(0)`, chuyển đổi số,
    `std.*` thiếu, `vis.toml`/MVS/sumdb, `cmd --help`, enum pattern có tiền tố, `if let`
    irrefutable, `main` chữ ký, phạm vi `vis fmt`, plugin tĩnh trong bản phát hành, LSP.

Phụ lục A (tương thích cú pháp), B (7 ví dụ tổng hợp đã chạy `vis check`), C (bảng đối chiếu test).

## 3. Cross-check

- **Từ vựng**: `crates/vis-syntax/src/lex/token.rs` — `Kind::ALL` 25, `KEYWORDS` 40, `PUNCTS` 39;
  khớp `docs/SPEC-lex.md`.
- **Cú pháp**: đọc trực tiếp `parse/{mod,expr,khai_bao,cmd}.rs`; mọi luật EBNF có luật sinh tương
  ứng. Các chỗ parser/tyck khác thiết kế được ghi ở §12, không nâng thành chuẩn.
- **Ngữ nghĩa**: đối chiếu `tyck/**`, `interp/**`, `bytecode/**`, `test/sem/**`; các ca then chốt
  được chạy tay (`T??`, `T???`, auto-lift, `//`/`%`, `set{}` là list, `ref(1)` là identity,
  `if let v = x`, `main` chữ ký, `impl` method không `pub`, `async cmd`).
- **Doc-examples**: trước khi thêm SPEC `24 khối: 19 xanh + 5 ngoại lệ`; sau khi thêm 7 khối
  `Phụ lục B` (đều `vis check` xanh), nâng ngưỡng **24 → 31** trong
  `test/doc-examples/doc_examples.py` và sinh **7 fixture** `doc-spec-*`. Đây là thay đổi ngưỡng
  **có chủ ý** của ST-309 (mọi ngưỡng khác giữ nguyên); A0 rà trong commit này.
  Kết quả: `doc-examples: OK (31 khối: 26 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD)`.
- **report-check**: `python3 test/report/check_report.py --all` → **PASS (35 báo cáo, 333 hàng
  QUEUE phủ đủ)**; không sửa báo cáo.
- **git diff --check**: sạch (0 lỗi khoảng trắng).
- Build `vis 0.5.0` bằng Rust **1.100.0**, target riêng `/home/ps/tmp/vis-target-st309`; 7/7 khối
  `vis` trong SPEC qua `vis check`.

## 4. Quyết định / thay đổi cần A0 biết

1. Thêm `docs/SPEC.md` + link ở `docs/README.md`.
2. Nâng `BLOCK_COUNT` 24 → 31 và thêm 7 fixture doc-examples (do SPEC có 7 khối `vis` hợp lệ).
   Không đổi `ALLOW_COUNT`, `VISH_COUNT`, `QD_COUNT`, không đổi allowlist.
3. Không sửa `agent/QUEUE.md`/`DECISIONS.md`/reports: ST-309 còn phần security/soak/tag; tránh
   giẫm worktree ST-309 security đang chạy.

## 5. Blocker / tồn

- Không có blocker kỹ thuật cho phần SPEC.
- Mốc macOS **chưa Tier 1** (chờ ST-300m3) và các gap ở §12 vẫn là hành vi hiện tại — SPEC ghi
  đúng trạng thái, không tuyên bố quá.
- Full `./vis-dev gate --fast` không chạy trong worktree này (chỉ chạy doc-examples/report-check/
  diff-check theo phạm vi tài liệu); mã sản phẩm không đổi.
