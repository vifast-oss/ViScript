# ST-313a/b — Sóng 2 Tái kiến trúc (2026-11-24)

- **Mốc**: `4e6f810` → `aa816e4` (ST-313a) → `c04fee9` (ST-313b). Chế độ VD-289; không soak/fuzz.
- **Máy**: chỉ `ps` + `psvn`; **không dùng `mps`** (đang làm dự án khác theo chỉ đạo).

## ST-313a — tách `vis-runtime`

- Crate mới `crates/vis-runtime`: `Scheduler`, chan/select/fee/reactor_io/frame_test,
  worker pool (`value/worker`), `builtin/`, `stdlib/`, `render`, `lit`
  (`lit_value`/`dinh_dang`), `lift`, `iter`, `codes` (`RT`/`TASK_ERR`/`BUDGET`…),
  adapter `AsyncHttpEnv`.
- `vis-interp` còn **8.820 LOC**: tree-walker, frame evaluator, orchestration worker
  (`rt/exec.rs`), `impl Host`, pat/gan, adapter tối thiểu.
- `vis-vm`: **0 dep `vis-interp`** ở dependencies (chỉ dev-dep parity); phụ thuộc
  `vis-runtime` + `vis-bytecode` + `vis-hir` + `vis-core`.
- `vm_frame.rs` 1.164 → **952 LOC** (`vm_frame/budget.rs`).
- Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` · `async_it` 339/339 ·
  `net_pkg_it` 156/156 · `vis-targets` 22 crate PASS=45/43 FAIL=0.

## ST-313b — `StdId` + chia miền stdlib

- `vis_hir::stdlib::StdId`: 167 biến thể sinh **một nguồn** từ `FUNCS`
  (`COUNT`/`ALL`/`as_str`/`from_name`/`index`).
- `vis-runtime::stdlib::call` tra id **một lần** rồi `call_by_id` `match` vét cạn;
  không còn so chuỗi trong từng nhánh.
- Test song ánh 1–1 `FUNCS ↔ StdId` (phủ đúng COUNT, không trùng, `ALL` đúng thứ tự,
  tập tên trùng khít).
- `stdlib/` chia miền: `str/{scan,parse,replace,split,trim}`,
  `sqlite/{pragma,introspect,index,schema,explain}`,
  `path/{join,normalize,ext,dispatch}`, `list/{math,stats}` — **0 tệp `_a.._e`**.
- Cổng: `gate --fast` PASS · `vis-diff 100k lech=0` · 283 test runtime ·
  `async_it` 339/339 · `net_pkg_it` 156/156.

## Cổng cuối Sóng 2

| Cổng | Kết quả |
|---|---|
| clippy `--workspace --all-targets -D warnings` | Linux PASS · Windows (`psvn`) PASS · **macOS bỏ qua** (mps bận) |
| `bin/vis-targets` | 100% 22 crate · core PASS=45 · sqlite PASS=43 + 2 EXPECTED_RED · FAIL=0 |
| `vis-wasm-suite --build` | **28/28** |
| `vis-wasm-sem` | **388/390** (2 đỏ = nợ `#193` đệ quy sâu) |

## Ghi chú

- `vis-bytecode` chỉ còn nhắc `vis_interp` trong chú thích, không phụ thuộc.
- Sóng 3 (`ST-313c`/`ST-313d`) vẫn `BLOCKED` — chờ A0 duyệt.
