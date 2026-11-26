<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Kế hoạch `ST-315b` — `Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười (W2)

> Worktree: `/home/ps/ViScript-w2` · nhánh `plan/st315b` · nền `c2b051c9`.
> **Không** sửa `docs/SPEC*.md`, `agent/DECISIONS.md`, `agent/ACTIVE.md`. **Không** dùng máy `mps`.
> Dùng chung target: `export CARGO_TARGET_DIR=/home/ps/ViScript/target`.

## Hợp đồng đã chốt (`VD-291` §3.2, Q1–Q3)

- `Value::Map(Rc<VisMap>)` — CoW qua `Rc::make_mut`; `size_of::<Value>() <= 64` (test khoá).
- `VisMap { entries: Vec<(Value, Value)>, index: Option<Box<MapIndex>> }`; chỉ mục dựng **lười**
  khi `len >= NGUONG` (đo `N = 1, 4, 8, 16, 79, 316`; nhỏ không alloc phụ). Tái dùng
  `bam_khoa_vo_huong` + `ChiMucKhoa` ở `crates/vis-core/src/value/show_eq.rs`.
- **Q1**: khoá trùng theo `eq_value` (`1` vs `1.0`, `0.5` vs `50pct`, `NaN` fallback quét) cập nhật
  `slot.1 = v` **tại chỗ**, giữ khoá gốc + vị trí chèn đầu.
- **Q2**: `json.decode` last-wins, giữ vị trí chèn đầu, giá trị cuối; bỏ vòng quét O(N²) ở
  `crates/vis-core/src/json/mod.rs`.
- **Q3**: `Show`/`keys()` giữ thứ tự chèn; `remove` bỏ mục, `set` lại chèn **cuối** — byte-for-byte
  như hiện tại.
- Gom **mọi** điểm tra/gán/xoá/gộp/JSON về `VisMap`: `crates/vis-interp/src/gan.rs`,
  `crates/vis-interp/src/interp/eval.rs`, `crates/vis-interp/src/frame/proc_expr.rs`,
  `crates/vis-vm/src/vm/vm_helpers.rs`, `crates/vis-runtime/src/builtin/method.rs`,
  `crates/vis-runtime/src/stdlib.rs`, `crates/vis-runtime/src/iterator.rs`, `json/*`.
  `SendValue::Map` giữ nguyên dạng snapshot (không đổi ABI worker).

## Thứ tự thi công đề nghị

1. Thêm `VisMap` + `MapIndex` (trong `vis-core/src/value/`, tệp mới nếu cần) với API tối thiểu:
   `new`, `len`, `is_empty`, `get`, `get_mut/insert` (last-wins tại chỗ), `remove`, `merge`,
   `keys/values/iter`, `from_vec_last_wins`, `make_mut`.
2. Đổi `Value::Map(Vec<...>)` → `Value::Map(Rc<VisMap>)`; sửa toàn bộ pattern `Value::Map(kv)`
   (55 điểm) theo API mới; compiler sẽ chỉ từng chỗ.
3. Sửa `Show`/`eq_value`/hash/JSON/serde/worker snapshot; giữ nguyên output.
4. Dựng chỉ mục lười + ngưỡng; đo `mapbench` (script có sẵn ở `/tmp/opencode/mapbench` hoặc tạo
   mới, **không** thêm vào repo nếu không cần).
5. Test: `size_of::<Value>() <= 64`; Q1/Q2/Q3; map nhỏ không index; map lớn index; `remove` rồi
   `set`; `eq_value` `1`/`1.0`/`NaN`; JSON trùng khoá; interp↔VM parity (`stdlib_diff`).

## Cổng & KPI

- `cargo fmt --all`; `./vis-dev gate --fast` **PASS**; `cargo clippy --workspace --all-targets -- -D warnings`
  **PASS**; `VIS_DIFF_N=100000 ./bin/vis-diff` **lech=0**; `stdlib_diff` **lech=0**.
- Release `opt-bench` **≤ 120 ms** cả interp & VM; `mapbench N=316` **≥ 10×** so với nền 1.429 ms;
  `N=1` không hồi quy > 5%; ghim sha256 binary trong journal.
- R8: **không** tạo tệp `.rs` test cấp 1 mới; ca mới vào `tests/<domain>/` + `#[path]`.

## Bàn giao

- Ghi `agent/JOURNAL/st315b-map-cow.md` (thiết kế, Q1–Q3, số đo, caveat).
- Cập nhật hàng `ST-315b` trong `agent/QUEUE.md` + `agent/STATE.md` (giữ `REVIEW` nếu chưa đủ KPI).
- Commit trên `plan/st315b` tiền tố `[ST-315b]`; **không** push/merge; báo commit hash + kết quả.
