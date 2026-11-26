<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Review ST-317c — Gatekeeper (Agent 1) độc lập

**Worktree**: `/home/ps/viswt/st317c` · nhánh `plan/st317c` · HEAD `b6fc4cb7` · nền `882b7ffa`.
**Kết luận**: **PASS** — 0 finding P1; 1 finding P2 **có sẵn từ trước, không do ST-317c** (flake `http_bon_ham_parity`); 2 ghi chú P3.

## 0. Phạm vi

- `git status --short`: sạch trước review (sau đó chỉ thêm tệp này).
- `git log --oneline 882b7ffa..HEAD`: `8824a1b2`, `d39f5fce`, `1c814dd9`, `36c77c02`, `b6fc4cb7`.
- `git diff --stat 882b7ffa..HEAD`: 26 tệp, +1.204/−1.928.
- **Không** chạm vùng cấm: `vis-core/src/value/**`, `vis-types/src/tyck/pat.rs`, `vis-pkg/**`,
  `vis-runtime/src/rt/sched.rs`, `vis-host-native/src/db/sqlite/mod.rs` (diff rỗng cho các path này).
- Tệp lạ ngoài phạm vi: không. Chỉ `agent/STATE.md`, `docs/SPEC.md`, `docs/ViScript-thiet-ke.md`,
  `test/fixtures/run_fixtures.sh` + các tệp thuộc 3 mục tiêu.

## 1. StdId registry một nguồn — PASS

| Kiểm | Bằng chứng |
|---|---|
| Số hàng registry | `awk '/^std_registry! \{/,/^\}/' .../registry.rs \| grep -c '=> "'` = **167** |
| `COUNT`/`FUNCS.len()`/`ALL.len()` | test `std_id_dem_that_va_dispatcher_khong_co_nhanh_bat_tat_ca` assert cả 3 = **167** — PASS |
| Tên khớp API-STD | 167 tên registry = **đúng tập** 167 khoá `agent/API-STD.txt` (`sort`+`diff` rỗng) |
| Thứ tự cũ giữ nguyên | 167 tên `FUNCS` mới **trùng thứ tự từng dòng** bảng cũ `funcs_*` (diff rỗng) |
| Biến thể ↦ tên | 167/167 cặp `StdId::X ↦ "..."` **giống hệt** `std_id.rs` cũ (chỉ đổi thứ tự khai) |
| Chữ ký đóng băng | `cargo test -p vis-conformance --test lib_api_it api_std_khop_code` — **1 passed**; `git diff 882b7ffa..HEAD -- agent/API-STD.txt` **rỗng**; blob hai đầu đều `eb8d4b20…` |
| Dispatcher | `call_by_id` không có `_ =>`; test quét mã nguồn đòi đủ 167 nhánh, ca âm `quet_dispatcher_ca_am` PASS |
| Test | `cargo test -p vis-hir` **10 passed**; `cargo test -p vis-runtime stdid` **4 passed** |
| Xoá bảng cũ | 8 tệp `D` (bang, funcs_fs_path, funcs_list_map, funcs_misc, funcs_net_http, funcs_sqlite, funcs_str, std_id); grep toàn repo (trừ `agent/archive`, `JOURNAL`) **0 tham chiếu** |

Ghi chú: thứ tự khai `StdId` đổi từ alphabet (cũ) sang thứ tự module (mới). `index()` chỉ dùng trong
test; không có chỗ nào serialize/ABI hoá chỉ số (`grep 'StdId::COUNT\|\.index()\|as u16'` ngoài
registry/tests = 0). Không phải breaking change.

## 2. R10 `API-CODES.txt` ↔ `SPEC.md` §7.2 — PASS

- `bin/vis-dev:304 rule_api_codes` đối chiếu: cùng tập mã, **mức** `error`/`warning`, trùng hàng,
  và câu `Sổ đóng băng **N mã**` = số hàng thật; chạy trong `run_rules` (R1..R10).
- `./vis-dev rules` → `--- rules: OK`, rc=0.
- `bash test/fixtures/run_fixtures.sh` → **20/20 ok**, `# fixtures: PASS`.
- Số thật: `API-CODES.txt` **57** mã; §7.2 **57** hàng; câu khai **57**.
- Ca âm: fixture 19 (thiếu mã + số khai) bị bắt. Kiểm **độc lập** thêm trên cây tạm:
  - lệch mức (SPEC `error` / API `warning`): `R10 … VS0101: mức SPEC 'error' ≠ API-CODES 'warning'` rc=1;
  - lệch số khai `2` vs `1`: bị bắt; trùng mã trong API: bị bắt.
  ⇒ luật thật, không phải test giả. (P3: fixture 19 gộp "thiếu mã" với "lệch mức" nên **một mình**
  fixture chưa chứng minh nhánh mức — nhánh mức đã được tôi chứng minh thủ công; đề nghị tách ca.)

## 3. `AnalysisHost` / `LoadState` — PASS

- `grep -rn 'thread_local!' crates/vis-lsp/src` → **0 macro thật** (chỉ 3 dòng comment).
- `crates/vis-loader/src/modules.rs:58` **còn** `thread_local! TRANG_THAI` — chỉ cho API CLI cũ
  (`load_full*`, wrapper `dat_noi_dung_mo`…); lane ST-317c chỉ yêu cầu tách **vis-lsp** khỏi
  thread-local. LSP **không** gọi wrapper này: mọi call site trong `vis-lsp` dùng
  `load_full_voi_text_trang_thai(..., &mut load)` / `LoadState` của host (grep xác nhận).
  ⇒ không rò trạng thái giữa host; đây là P3 (dư lượng CLI có chủ đích, đã ghi trong journal Builder),
  không phải P1/P2.
- `cargo test -p vis-lsp --lib`: **29 passed**, gồm `host::tests::hai_host_khong_ro_overlay_va_cache`.
- `cargo test -p vis-lsp --test lsp_it`: **158 passed, 0 failed, 0 ignored**; gồm 2 ca in-process
  `lsp_host::hai_host_cung_uri_khong_ro_buffer_va_chan_doan` và `…hai_host_song_song…`.
- Không `#[ignore]` trong `host.rs` / `lsp_host.rs`.
- API công khai: `vis_lsp::run(args: &[String]) -> ExitCode` **không đổi**; chỉ **thêm**
  `pub use host::{AnalysisHost, ThongKeCache}`. `didOpen/didChange/didClose/completion/hover/…`
  giữ nguyên khung JSON (diff chỉ thay nguồn trạng thái; 158 ca giao thức xanh).
- `didClose` vẫn publish `diagnostics: []` (`chan_doan_dong` → `chan_doan_voi(uri, None)`), và
  `uri_lien_quan` vẫn loại chính URI + đọc overlay đã xoá ⇒ ca `lsp_da_tep` VS3-7 xanh.

## 4. Cổng độc lập — PASS (1 lần flake, đã chứng minh)

- `./vis-dev gate --fast` **lần 1**: FAIL tại `host_parity::net_http::http_bon_ham_parity`
  (1966/3885 chạy, 1965 pass). Nguyên nhân: stdout hai engine khác **1 ms** trong thông điệp
  timeout thật (`…sau 78 ms` vs `…sau 79 ms`); test so **từng byte** sau khi chỉ chuẩn hoá cổng.
- Chạy riêng ca đó **3/3 PASS**; `git diff 882b7ffa..HEAD -- crates/vis-conformance …` **rỗng**
  ⇒ flake **có sẵn**, không do ST-317c (P2 bên dưới).
- `./vis-dev gate --fast` **lần 2**: **3885/3885 passed, 4 skipped**, `--- L0 test: OK`,
  `vis-dev gate --fast: PASS`, exit 0.
- `VIS_DIFF_N=100000 ./bin/vis-diff`: `tong=100000 … chay=93768 lech=0`, exit 0.
- Log ngoài repo (`/home/ps/vislogs/`):
  - `review-st317c-gate.log` (lần 1, FAIL) sha256 `78003163380032f4ddd2e1909723c878cac0c3eee36e0a9f8e2e0cfd25768e5b`;
  - `review-st317c-gate-run2.log` (PASS) sha256 `6af9d77669678eab0e7b1181f3bdbc7f6120962e8a9a3b09f3e6bafcb67835da`;
  - `review-st317c-diff100k.log` sha256 `c9c325d8c2346ea7e671ef72d34be3d0f1c900e9f3bb8ac6206fb5629ca4b7e2`.

## 5. R8 / R1 — PASS

- R8: `crates/*/tests/*.rs` cấp 1 = **38** + `vis-host-linux/tests` = **1** ⇒ **39/40**.
  Tệp mới `crates/vis-lsp/tests/lsp_it/lsp_host.rs` nằm trong thư mục con + `#[path]` ⇒ không tăng.
- R1: không tệp sản phẩm nào > 1.500 dòng (max toàn cây 1.275 `vis-types/tests/...`; sản phẩm lớn
  nhất 1.189 `rt/sched.rs`). `vis-lsp/src/lib.rs` **1.056** (chưa < 1.000 — đúng lane `ST-317b`,
  không tính FAIL); `vis-loader/src/modules.rs` 1.043; `registry.rs` 304; `host.rs` 231.

## 6. Finding / rủi ro

| Mức | Nội dung | Tái lập |
|---|---|---|
| **P2** (có sẵn, không do ST-317c) | `host_parity::net_http::http_bon_ham_parity` flake: so byte stdout chứa **số ms đo thật** của timeout, hai engine lệch 1 ms ⇒ đỏ oan. Đã có ghi chú flake cùng test trong `JOURNAL/review-st315d-parity.md`; chưa thấy trong sổ nợ. | `./vis-dev gate --fast` lần 1 (log trên); chạy riêng 3/3 xanh. Đề nghị chuẩn hoá `sau \d+ ms` → `sau <ms> ms` trong test. |
| P3 | Fixture R10 số 19 gộp "thiếu mã" + "lệch mức"; nếu ai xoá nhánh so mức, fixture vẫn đỏ vì thiếu mã ⇒ không khoá riêng nhánh mức. | Đã chứng minh nhánh mức hoạt động thủ công (mục 2); đề nghị thêm 1 fixture chỉ lệch mức. |
| P3 | `vis-loader` còn `thread_local!` cho API CLI cũ (không dùng trong LSP). | `grep -n thread_local crates/vis-loader/src/modules.rs`; lane chỉ yêu cầu tách `vis-lsp`. |

Không tìm thấy lỗi P1. Không có rò cache/overlay giữa 2 host; `didClose` không làm mất chẩn đoán
của host khác; registry không sai thứ tự `FUNCS`/không khác `API-STD.txt`.

## 7. Tổng hợp

| Mục | Kết quả |
|---|---|
| 1. Phạm vi diff | **PASS** |
| 2. StdId 167 / dispatcher / API-STD | **PASS** |
| 3. R10 + 20 fixture + ca âm | **PASS** |
| 4. AnalysisHost + 158 `lsp_it` | **PASS** |
| 5. `gate --fast` + `vis-diff 100k` | **PASS** (gate lần 2; lần 1 flake P2) |
| 6. R8 39/40, R1 ≤ 1.500 | **PASS** |
| 7. Finding | P1: 0 · P2: 1 (có sẵn) · P3: 2 |

**Duyệt PASS** cho `ST-317c`; đề nghị A0 ghi nợ flake `http_bon_ham_parity` (P2) và không chặn merge.
