<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-317c — Điểm mở rộng một nguồn (StdId registry / SPEC §7.2 / LSP `AnalysisHost`)

**Worktree**: `/home/ps/viswt/st317c` · nhánh `plan/st317c` · nền `review/song5` (`882b7ffa`) ·
Builder (Agent 2) · 2026-11-26 → 27 (giờ thấp điểm, không fuzz/soak).

## 1. Mục tiêu

1. Gom bảng `StdId` + chữ ký + metadata vào **một nguồn** trong `crates/vis-hir/src/stdlib/`;
   thêm hàm `std.*` mới chỉ chạm **2 chỗ**, vẫn **compile-time exhaustive** (không `_ =>`).
2. Đóng **#75**: kiểm máy `agent/API-CODES.txt` ↔ bảng §7.2 `docs/SPEC.md`, chạy trong
   `./vis-dev check`, có ca âm.
3. Đóng **#81/#66**: tách trạng thái `vis-lsp` khỏi `thread_local!` sang `AnalysisHost` tiêm được;
   `lsp_it` song song không rò trạng thái. **Không** chia nhỏ `vis-lsp/src/lib.rs` (việc `ST-317b`).

## 2. Đã làm

### 2.1 Một nguồn `std.*` (`crates/vis-hir/src/stdlib/registry.rs`, mới)

- Macro `std_registry!` nhận **một bảng** dòng `<BiếnThể> => "<module.tên>": [<tham số>] -> <kiểu trả>,`
  và sinh tại compile-time: `enum StdId` (167 biến thể), `COUNT`, `ALL`, `as_str`, `from_name`,
  metadata `StdId::module()`, `FUNCS: [StdFn; COUNT]`.
- Xoá 8 tệp bảng cũ: `std_id.rs` (707 dòng), `bang.rs`, `funcs_fs_path.rs`, `funcs_list_map.rs`,
  `funcs_misc.rs`, `funcs_net_http.rs`, `funcs_sqlite.rs`, `funcs_str.rs` — **−1.547 dòng**.
- `MODULES`/`MODULES_HOAN` chuyển về `registry.rs` (cùng nguồn với bảng hàm), test khoá
  "mọi module trong FUNCS ↔ MODULES" (không metadata chết).
- **Thêm hàm `std.*` mới = 2 chỗ**: (1) một hàng trong `registry.rs`; (2) một nhánh
  `StdId::<BiếnThể>` trong `vis_runtime::stdlib::call_by_id`. Không còn phải sửa `ALL`/`as_str`/
  `from_name`/`FUNCS` riêng.
- **Số thật: 167** (`assert_eq!(StdId::COUNT, 167)` trong test; `FUNCS.len()`/`ALL.len()` cùng 167).
- Chữ ký sinh lại khớp **byte-for-byte** `agent/API-STD.txt` (test `api_dong_bang::api_std_khop_code`
  PASS — không phải regen tệp đóng băng).

Test mới/cập nhật (`crates/vis-runtime/src/stdlib/stdid_tests.rs`):
- `std_id_dem_that_va_dispatcher_khong_co_nhanh_bat_tat_ca`: đếm 167 + quét khối `match id` của
  `call_by_id` trong mã nguồn, đòi **mọi** `StdId` có nhánh cấp 1 và **không** có `_ =>`.
- `quet_dispatcher_ca_am`: ca âm của bộ quét (`_ =>` bị từ chối; thiếu nhánh bị phát hiện).
- `std_id_phu_dung_funcs_va_khong_trung` bổ sung kiểm `StdId::module()` khớp tiền tố tên.

### 2.2 Đóng `#75` — luật `R10` (`bin/vis-dev`)

- `rule_api_codes`: đối chiếu `agent/API-CODES.txt` ↔ bảng §7.2 `docs/SPEC.md`:
  cùng tập mã, cùng mức `error`/`warning`, không trùng hàng, và câu "Sổ đóng băng **N mã**" bằng
  số hàng thật. Chạy trong `./vis-dev check` (luật R1..R10); thiếu một trong hai tệp ⇒ bỏ qua
  (cây tạm), có `docs/SPEC.md` mà thiếu §7.2 ⇒ đỏ.
- `test/fixtures/run_fixtures.sh`: **18 → 20 ca**; ca 19 âm (lệch mức + thiếu mã), ca 20 dương.
- `docs/SPEC.md` §7.2 ghi thêm luật R10; `docs/SPEC.md` §11.2 + `docs/ViScript-thiet-ke.md` §18.4
  đổi `R1..R9` → `R1..R10` (hành vi cổng đổi, tài liệu phải khớp).

### 2.3 Đóng `#81`/`#66` — `AnalysisHost` (`crates/vis-lsp/src/analysis/host.rs`, mới)

- `vis-loader`: thêm `LoadState` (overlay buffer + cache nội dung tệp phụ) **tiêm được**;
  `Nap` giữ `&mut LoadState`. API cũ `dat_noi_dung_mo`/`xoa_noi_dung_mo`/`xoa_cache_phu`/
  `load_full*` giữ nguyên chữ ký, chạy trên `thread_local!` mặc định cho CLI (một phiên/process).
  Thêm `load_full_voi_text_trang_thai(...)` cho LSP.
- `vis-lsp`: `AnalysisHost` sở hữu buffer tài liệu, cache phân tích `(uri,text)`, cache module phụ,
  `LoadState`, `rootUri`. `run()` dùng **một** host; `analysis/module.rs` **không còn** `CACHE_MOD`
  thread-local; `chan_doan`/`module` nhận `&mut LoadState` tường minh.
- API công khai ổn định: `vis_lsp::run` không đổi; thêm `vis_lsp::AnalysisHost` +
  `vis_lsp::ThongKeCache` (nhúng/test). JSON LSP công khai không đổi (158 ca `lsp_it` PASS).
- Test khoá:
  - `host::tests::hai_host_khong_ro_overlay_va_cache` — hai host **cùng thread**, cùng đường dẫn
    `util.vis` nhưng overlay khác: mỗi host chỉ thấy item của mình; `didClose` host A không xoá
    cache host B. (Trước ST-317c `PHU_MO` thread-local ⇒ đỏ.)
  - `tests/lsp_it/lsp_host.rs` — 2 ca in-process: cùng URI khác buffer (chẩn đoán riêng) và hai
    host dựng **song song** mỗi thread. Tệp nằm trong `tests/lsp_it/` + `#[path]` ⇒ **R8 không đổi**.
- Kích thước: `vis-lsp/src/lib.rs` 1.144 → **1.058** dòng (vẫn > 1.000; việc chia nhỏ là `ST-317b`,
  không làm ở đây theo lane). `analysis/host.rs` 213 dòng; `vis-loader/modules.rs` 986 → 1.034.

## 3. Lane điều chỉnh (BỔ SUNG từ phân tích xung đột)

- Được phép chạm `crates/vis-loader/src/modules.rs` để hoàn tất #81/#66 — **đã chạm** (LoadState),
  không luồng khác sở hữu tệp này.
- `crates/vis-runtime/src/stdlib.rs` đang **976 dòng**: ST-317c **không thêm nhánh dispatch nào**
  (167 hàm giữ nguyên) nên vẫn 976 dòng < 1.000; không tách tệp (việc `ST-317b`).
- Không chạm các vùng cấm: `crates/vis-core/src/value/**`, `crates/vis-types/src/tyck/pat.rs`,
  `crates/vis-pkg/**`, `crates/vis-runtime/src/rt/sched.rs`,
  `crates/vis-host-native/src/db/sqlite/mod.rs`.

## 4. Cổng

| Cổng | Kết quả |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo test -p vis-hir` | 10/10 |
| `cargo test -p vis-runtime stdid` | 4/4 |
| `cargo test -p vis-conformance --test lib_api_it api_dong_bang` | 3/3 (API-STD/CODES/VBC1) |
| `cargo test -p vis-lsp --lib` | 29/29 |
| `cargo test -p vis-lsp --test lsp_it` | 158/158 |
| `cargo test -p vis-loader` | PASS |
| `cargo clippy` (crate chạm, `-D warnings`) | PASS |
| `test/fixtures/run_fixtures.sh` | 20/20 PASS |
| `./vis-dev gate --fast` | xem §5 (log `/home/ps/vislogs/st317c-gate-fast.log`) |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | xem §5 |
| R8 (tệp `.rs` cấp 1 trong tests) | 39/40 (không tạo tệp cấp 1 mới) |

## 5. Kết quả cổng cuối

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` (`CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`) | **PASS**, exit 0 — nextest **3885/3885 passed, 4 skipped** + doc-test OK; `check` gồm fmt/clippy/R1..R10/DAG/toolchain/release/fixtures 20/report/build/doc-examples/readme-examples đều OK. Log `/home/ps/vislogs/st317c-gate-fast.log`, sha256 `0ca31f3ed9924d8cb0bcbc92fb779671d8c7cf4656393fe2756b4b6c25d0eefe`. |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **PASS** — `DIFF tong=100000 loi_tinh=6232 chay=93768 lech=0 giay=126.7` (+ 2k: `lech=0`); log `/home/ps/vislogs/st317c-diff100k.log`, sha256 `74af9ecc782af8505c5fc52bebcc5accf9ea55d067769230e8812ee25b1c38dc`. |
| R8 | 38 `crates/*/tests/*.rs` cấp 1 + 1 `vis-host-linux/tests/landlock.rs` = **39/40** (không tạo tệp cấp 1 mới). |
| R1 | `vis-lsp/src/lib.rs` 1.058 · `analysis/host.rs` 213 · `vis-loader/modules.rs` 1.034 · `vis-runtime/src/stdlib.rs` 976 (không đổi) — mọi tệp ≤ 1.500. |

## 6. Việc còn lại / bàn giao

- `ST-317b`: chia `vis-lsp/src/lib.rs` (1.058) + 5 tệp sản phẩm khác về < 1.000 dòng — **không**
  làm trong ST-317c.
- `#50`/`#65` (hover field, receiver phức tạp, bảng phương thức `std.str`) vẫn mở: `AnalysisHost`
  chỉ là hạ tầng trạng thái, không mở rộng tính năng hover/completion.
- `#66` (cache `entry`/thành viên theo phiên) vẫn một phần mở: host đã có cache theo `(uri,text)`
  và cache module, nhưng completion sau `.` vẫn dựng sentinel `__vis` mỗi lần (ngoài phạm vi lát).
