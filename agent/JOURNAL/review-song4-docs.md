<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Gatekeeper — Kiểm toán tài liệu/trạng thái Sóng 4 (`ST-315a..ST-315d`)

> **Nhánh**: `review/r4docs` · **HEAD/nền**: `987b98b8` · **Ngày**: 2026-11-26
> **Máy**: `ps` (Linux); **không** dùng `mps`. Chỉ đọc mã; **không sửa mã sản phẩm**.
> **Kết luận**: **FAIL (tài liệu)** — bằng chứng kỹ thuật của `ST-315a`/`ST-315c` đứng vững, nhưng
> `ST-315d` và trạng thái `ACTIVE`/`ROADMAP` lệch với HEAD; cần sửa tài liệu trước khi A0 duyệt `DONE`.
> Không có sai lệch nào đòi sửa mã sản phẩm.

## 1. Bảng kiểm

| # | Mục | Kết quả | Bằng chứng | Đề xuất |
|---|---|---|---|---|
| 1 | `QUEUE.md:2228` — `ST-315a` | **PASS** | Trạng thái `REVIEW`; commit `50bb7885` đúng phạm vi (`crates/vis-vm/src/vm.rs:129/132/135` có `budget_idx`/`budget_flat`/`budget_used`; `vm_frame/budget.rs:248` tra `(start,len)` O(1), bỏ quét tuyến tính). Journal `st315a-vm-budget.md` ghi interp **329 ms** / VM **344 ms** / unlimited **342 ms**; `sha256 target/release/vis` = `657458382521907f2c9f3c96f09d7acab1612c2543475d9144445cd33d7cb48c` khớp log `st315a_opt_bench.md`. `lat256_budget` 9 passed + 1 ignored (QUEUE ghi “9/9” — chấp nhận được nhưng nên ghi rõ “9 + 1 ignored”). | Giữ nguyên; có thể thêm “9 passed / 1 ignored” cho chính xác. |
| 2 | `QUEUE.md:2229` — `ST-315b` | **PASS** | Trạng thái `READY`; chưa có commit sản phẩm cho `ST-315b`; `plan/st315b` trỏ đúng `987b98b8`, không có commit riêng (đúng là chưa khởi động). Không có số liệu/claim nào cần đối chiếu. | Giữ nguyên; chỉ tạo commit khi W2 bắt đầu. |
| 3 | `QUEUE.md:2230` — `ST-315c` | **PASS** | Trạng thái `REVIEW`; `fbcbc139` vá `#197` (`vis-host-linux/src/sandbox.rs:123`: `ReadFile+WriteFile+Truncate`) + W3 M2 + `vis-soak`; `987b98b8` bổ sung bằng chứng Windows. Chạy lại `vis-host-linux --test landlock`: **19/19**; `vis-plugin-host --lib`: **10/10**; đếm 18 `#[test]` tích hợp trong `tests/` (7 + 4 + 7). Windows **3.747/0**, 90 suite, `vis-targets` PASS (xem mục 7). | Giữ nguyên; nên ghi rõ Windows chạy tại `c2b051c9` (commit `ST-315d`) nên bằng chứng **bao gồm cả thay đổi `ST-315d`**. |
| 4 | `QUEUE.md:2231` — `ST-315d` | **FAIL** | HEAD có `5f8f8012` **thay DFS 3 màu bằng Tarjan SCC** (`crates/vis-types/src/tyck/const_cycle.rs` dùng `chi_so`/`low`/`tren_ngan_xep`/`ngan_xep`), rồi `ee0dcd8d` + `0ba3e8f2` mở rộng cùng ca `const_chu_trinh_bi_bat_tinh_voi_vs0304` (thêm `if` + hình kim cương, SCC 3 đỉnh, 2 chu trình rời ⇒ 4 chẩn đoán). Nhưng `QUEUE.md:2231` và `JOURNAL/st315d-vs0304.md:15` vẫn ghi “DFS 3 màu”, chỉ mô tả tới `c2b051c9`; trạng thái `READY` không phản ánh 3 commit đã có. Ngoài nhánh review, W4 còn `plan/st315d-parity` @`9356f1c8` (khung parity + 22 fs + 2 time), chưa merge. | Agent 2 cập nhật `QUEUE.md:2231` + journal: “Tarjan SCC”, liệt kê 3 ca mới, ghi rõ parity đang ở `plan/st315d-parity`; đổi trạng thái thành `READY (phần 1 xong)`/`REVIEW` theo A0. Không tuyên bố DFS 3 màu nữa. |
| 5 | `STATE.md:16` ↔ `QUEUE` | **PASS** | `STATE` ghi `ST-315a` `REVIEW`, `ST-315b` `READY`, `ST-315c` `REVIEW`, `ST-315d` “phần 1 xong, còn parity”; khớp `QUEUE` (`2228` REVIEW, `2229` READY, `2230` REVIEW, `2231` READY). `ST-309` vẫn `BLOCKED` (`STATE.md:14`, `QUEUE.md:2191`, `ACTIVE.md:7`, `ROADMAP.md:4`). | Thêm chữ `READY (phần 1)` cho `ST-315d` trong `STATE.md:16` để không phải suy luận trạng thái. |
| 6 | `ACTIVE.md` / `ROADMAP.md` ↔ `STATE`/`QUEUE` | **FAIL** | `ACTIVE.md:10` (“4 luồng `W1..W4` `READY`”), bảng `ACTIVE.md:18` (`ST-315a` `READY`) và `ACTIVE.md:20` (`ST-315c` `READY`), cùng `ROADMAP.md:7` (“4 luồng `W1..W4` `READY`”) chưa cập nhật sau `50bb7885`/`fbcbc139`; trong khi `STATE`/`QUEUE` đã là `REVIEW`. | Agent 0 (hoặc Agent 2 nếu được ủy quyền) cập nhật `ACTIVE` §1/§2 và `ROADMAP.md:7` sang `REVIEW`/“phần 1 xong”; giữ `ST-309` `BLOCKED`. Nếu chủ trương chỉ cập nhật `ACTIVE`/`ROADMAP` tại cổng A0 thì ghi rõ quy ước đó. |
| 7 | `SO-NO.md` — `#197` + §8 + R5 | **PASS** | `SO-NO.md:286` `#197` = **“đóng ở ST-315c”**, mô tả đúng `Truncate` + ca #19; §8 (`402..411`) đếm **78 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng = 198**. Tự đếm 198 dòng bảng chính (1..197, `#109` tách `a/b`) khớp từng trạng thái; `./vis-dev rules` PASS (R5 xanh). | Giữ nguyên. Ghi chú ngoài phạm vi: header `SO-NO.md:83` còn “185 dòng” và `:288` còn “21 mục” (cũ, có từ trước Sóng 4) — nên dọn khi có dịp, không tính FAIL Sóng 4. |
| 8 | `API-CODES.txt` — `VS0304` + 57 mã | **PASS** | `agent/API-CODES.txt:32` có `VS0304	error: chu trình phụ thuộc hằng số (`const`)`; đếm 57 dòng mã, không trùng; `vis-diag/src/lib.rs:900` assert `all_codes().count() == 57` và test lib **10/10**; `api_dong_bang` **3/3**. Diff `3ffd74d0..HEAD` chỉ **thêm** dòng `VS0304`, không đổi chuỗi cũ. | Giữ nguyên. |
| 9 | Commit hygiene (`[ST-00]` tách `[ST-###]`) | **PASS** | `3ffd74d0` là `[ST-00]` duy nhất trong dải; chỉ chạm `agent/*` + `docs/SPEC-lex.md`, `docs/SPEC.md`, `docs/ViScript-thiet-ke.md`, **0 tệp `.rs`**. Các commit mã là `[ST-315a]` (`50bb7885`), `[ST-315c]` (`fbcbc139`, `987b98b8` docs-only), `[ST-315d]` (`c2b051c9`, `ee0dcd8d`, `5f8f8012`, `0ba3e8f2`) — không lẫn nhãn. Log bằng chứng nằm ngoài repo (`/home/ps/vislogs/...`), không có `.log`/`.out` trong commit. Worktree **7 ≤ 8**. `plan/st315d-parity` chưa merge có chủ đích (W4 đang chạy). | Giữ nguyên. `plan/st309-qa` (`623f3ba5`, 23/11) đứng yên 1 commit trước `main` và `ST-309` đang `BLOCKED`; A0 nên chốt “giữ làm bằng chứng” hoặc archive để tránh nợ nhánh. |
| 10 | Windows `ST-315c` — nội dung log | **PASS** | `/home/ps/vislogs/st315c-win-full-c2b051c9.out`: 90 dòng `test result: ok`, tổng **3.747 passed / 0 failed**; HEAD guard `c2b051c91acf`; cuối log `vis-targets: phủ 100% workspace (23 crate)`, core `PASS=47`, sqlite `PASS=46/EXPECTED_RED=1`. Lệnh trong `.log:4` là `cargo clippy --workspace --all-targets -- -D warnings && cargo test ...` ⇒ chạy tới cuối chứng minh clippy exit 0 (không có dòng “clippy PASS” riêng). | Giữ nguyên; có thể tách dòng “clippy: exit 0 (chuỗi `&&`)” cho dễ kiểm. |
| 11 | Windows `ST-315c` — `sha256` khớp journal | **FAIL (nhẹ)** | `JOURNAL/st315c-w3.md:51-52` ghi path `/home/ps/vislogs/st315c-win-full-c2b051c9.log` + “sha256 toàn tệp `11cbf1ac…`”. Thực tế `11cbf1ac…` là `sha256(log_tren_may_dich)` in ở cuối wrapper (log trên `psvn`), **không** phải sha của tệp local: `.log` = `a2c20514cde524d760d75b0b1d636512c8d58de20a1110df682bb04d9dd1d776`, `.out` = `d2cec75750d69c037107f8d569dd4ed8fb7ec7c56f885fd0c731f49bda1c076d`. `.out` lại không chứa dòng sha. | Sửa câu chữ thành “sha256 log trên máy đích = `11cbf1ac…`; sha256 tệp local `.log` = `a2c20514…` / `.out` = `d2cec757…`”. Nội dung 3.747/0 vẫn đúng. |
| 12 | Chạy lại cổng tài liệu/trạng thái | **PASS** | `./bin/vis-dag-check` → `OK — 23 crate / 5 tầng, 0 cạnh cấm`; `./vis-dev rules` → `rules: OK`; `python3 test/report/check_report.py --all` → `PASS (35 báo cáo, 361 hàng QUEUE phủ đủ)`; `git diff --check` → sạch. Bổ sung: `vis-diag --test messages` 12/12, `vis-diag --lib` 10/10, `vis-types types_it const_chu_trinh` 1/1, `vis-host-linux --test landlock` 19/19, `vis-plugin-host --lib` 10/10, `vis-conformance lib_api_it api_dong_bang` 3/3. | Không cần hành động. |
| 13 | `docs/SPEC*.md` / `docs/ViScript-thiet-ke.md` ngoài `[ST-00]` | **PASS** | `git log 3ffd74d0..HEAD -- docs/` rỗng; `git diff 3ffd74d0..HEAD -- docs/` rỗng. Chỉ `[ST-00]` `3ffd74d0` sửa 3 tệp tài liệu (được phép). | Không cần hành động. |

## 2. Lệnh đã chạy lại (máy `ps`, `CARGO_TARGET_DIR=/home/ps/ViScript/target`)

```text
$ ./bin/vis-dag-check
vis-dag-check: OK — 23 crate / 5 tầng, 0 cạnh cấm

$ ./vis-dev rules
--- rules: OK

$ python3 test/report/check_report.py --all
report-check: PASS (35 báo cáo, 361 hàng QUEUE phủ đủ)

$ git diff --check
(không lỗi)
```

## 3. Việc cần làm trước khi A0 duyệt `DONE`

1. **Agent 2**: cập nhật `QUEUE.md:2231` + `JOURNAL/st315d-vs0304.md` theo HEAD — Tarjan SCC thay
   DFS 3 màu; mở rộng ca `const_chu_trinh_bi_bat_tinh_voi_vs0304` theo 3 commit
   `ee0dcd8d`/`5f8f8012`/`0ba3e8f2` (if/kim cương, SCC 3 đỉnh, 2 chu trình rời); ghi rõ parity còn
   ở `plan/st315d-parity` (`9356f1c8`) và trạng thái thật (`READY (phần 1)` hoặc `REVIEW`).
2. **Agent 0/Agent 2**: cập nhật `ACTIVE.md:10,18,20` và `ROADMAP.md:7` — `ST-315a`/`ST-315c` là
   `REVIEW`, không còn “4 luồng `READY`”; `ST-309` giữ `BLOCKED`.
3. **Agent 2**: sửa câu `sha256` trong `JOURNAL/st315c-w3.md:51-52` cho đúng “log trên máy đích”
   (hoặc ghi sha tệp local tương ứng); không cần chạy lại Windows.
4. **Tùy chọn**: dọn header cũ `SO-NO.md:83`/`:288`; A0 chốt số phận `plan/st309-qa`.

## 4. Kết luận

- **FAIL (tài liệu)** do mục **4**, **6**, **11**; các mục còn lại **PASS**.
- Không phát hiện sai lệch sản phẩm: `ST-315a`/`ST-315c` có mã + log khớp; `#197` đóng đúng;
  `VS0304`/57 mã khớp; `[ST-00]` sạch; `docs/SPEC*` không bị sửa ngoài `[ST-00]`.
