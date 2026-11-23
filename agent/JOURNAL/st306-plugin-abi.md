<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-306 — Plugin ABI tĩnh `VIS_PLUGIN_ABI_V1` + macro + parity interp↔VM

> **Trạng thái: DONE (local, chưa merge).** Nhánh `plan/w4-st306-plugin-abi` từ `c3b7290`
> (merge `ST-300m1`). Không merge/push/tag; **không ssh mps** (W2 độc quyền `m2`). Time-box 3h.

## 1. `crates/vis-plugin-abi`

- `VIS_PLUGIN_ABI_V1 = 1`; `PluginValue` ↔ `vis_interp::value::Value` chuyển đổi **tường minh**
  (`TryFrom` cả hai chiều). Hàm/task/kênh/closure không qua biên → `VS0501` (không đoán).
- `Error` = tên phía Rust của `vis::Error` (`code`, `message`); `VS0604` thiếu quyền,
  `VS0501` lỗi I/O/kiểu/số tham số.
- `PluginCtx` **chỉ** chứa `Cap` host truyền vào; `read_text`/`write_text` canonicalize root và
  đường dẫn để chặn `..`/symlink ra ngoài. `Cap::dir(root, write)`; ghi cần cờ `write`.
- Không ambient authority: không đọc env, không `std::fs` ngoài `PluginCtx`.
- **0 `unsafe`** (`#![forbid(unsafe_code)]`).

## 2. `crates/vis-plugin-macros`

- `#[vis::module(name = "...")]` + `#[vis::func]`: sinh `FuncDef`/`Signature`/`Param` và wrapper
  chuyển `PluginValue` → kiểu Rust bằng `TryInto` (**kiểm kiểu lúc build**), gọi hàm, chuyển kết
  quả về `PluginValue`.
- `ctx: &PluginCtx` là tham số đầu **không** lộ vào chữ ký ViScript; quyền chỉ đến từ host.
- Hàm trả `Result<T>` được `?` rồi đóng gói `Ok`; hàm thường đóng gói trực tiếp.
- Sai số tham số → `VS0501`, không panic.
- **0 `unsafe`** (`#![forbid(unsafe_code)]`).

## 3. Plugin mẫu + test

`crates/vis-plugin-abi/tests/parity.rs` (6 ca) + 2 unit trong `src/lib.rs`:

- `cong(a: int, b: int) -> int` — parity **interp↔VM qua biên ABI**: cùng chương trình
  `fn cong` chạy bằng interp và VM, chuyển cả hai sang `PluginValue`, so với kết quả plugin;
  ba đường bằng nhau (`Int(5)`).
- `doc(ctx, p) -> str`: không `cap[Dir]` ⇒ `VS0604`; có cap đọc đúng nội dung.
- `ghi(ctx, p, noi_dung) -> ()`: cap chỉ đọc ⇒ `VS0604`; cap ghi ⇒ ghi được; đường dẫn ngoài
  root ⇒ `VS0604`.
- Chữ ký macro: `ctx` không lộ; `a/b: int`, trả `int`/`str`.
- Chuyển đổi `Value::Builtin` → `VS0501` (không nuốt kiểu lạ).

Kết quả `cargo test -p vis-plugin-abi`: **8/8 pass** (2 unit + 6 integration).

## 4. Cổng Linux

- `./vis-dev gate --fast`: **PASS — 3680 passed / 0 failed / 4 skipped** (baseline `c3b7290`
  3672/4; +8 ca ST-306).
- `fmt: OK`, `clippy: OK` (kể cả `-D warnings` cho hai crate mới), `luật R1..R7: OK`,
  `report-check: PASS (34 báo cáo, 333 hàng QUEUE)`, `doc-examples: OK (24 khối)`.
- `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`; không ssh mps.

## 5. Việc còn lại / blocker

- Chờ A0 merge; `ST-300m3` (mps) vẫn là ưu tiên khi W2 báo `m2` xong — **chưa chạy**.
- Không blocker kỹ thuật. Không merge/push/tag, không GitHub Actions.
