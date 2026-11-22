# vis-host-wasm — host WASI cho ViScript (ST-299a)

Workspace **riêng** (không thuộc workspace chính; `exclude` từ ST-171) chứa host WASI thật cho
`wasm32-wasip1` và runner một tệp `vis-wasm-run`.

## Phạm vi

| Thành phần | Việc làm |
|---|---|
| `fs::WasiFs` | `FsHost` chỉ trong **preopen**: đọc/ghi/list/walk/copy/rename/metadata; preopen `@ro` từ chối ghi; chặn `..`, symlink trỏ ra ngoài, đường dẫn ngoài preopen bằng `VS0604` |
| `dong_ho` | wall clock + monotonic qua `SystemTime`/`Instant` (WASI `clock_time_get`) |
| `stdio::Stdio` | ghi thật ra stdout/stderr, hoặc gom bộ đệm để test; broken pipe là lỗi trả về |
| `tu_choi::{TuChoiNet,TuChoiProc,TuChoiDb}` | từ chối rõ `net` / `proc` / `sqlite`, không panic |
| `runner::chay` | pipeline một tệp `parse → lower → tyck → (compile) → interp\|VM` |
| `src/bin/vis-wasm-run.rs` | binary chạy trên native và `wasm32-wasip1` |

Crate đặt `#![forbid(unsafe_code)]`; không dependency ngoài crate lõi ViScript (`vis-syntax`,
`vis-types`, `vis-bytecode`, `vis-interp`, `vis-vm`, `vis-diag`).

## Dùng

```bash
# native
cargo run --bin vis-wasm-run -- --interp --preopen /data=$PWD/data data/prog.vis
cargo run --bin vis-wasm-run -- --vm     --preopen /data=$PWD/data data/prog.vis

# wasm (khi có wasmtime CLI)
cargo build --release --target wasm32-wasip1
wasmtime run --dir "$PWD/data::/data" --env VIS_WASM_PREOPENS=/data \
  target/wasm32-wasip1/release/vis-wasm-run.wasm -- --interp /data/prog.vis
```

Cú pháp preopen: `guest[=host][@ro]`, lặp lại được. Không có `--preopen` ⇒ đọc
`VIS_WASM_PREOPENS` (`guest[:guest@ro…]`) và (trên wasm) mục `guest::host` do wasmtime chèn.
Không có preopen nào ⇒ `std.fs` bị từ chối **rõ** (`VS0604`), không panic.

Đường dẫn tương đối giải theo **preopen đầu tiên**; chương trình nhiều preopen nên dùng đường dẫn
tuyệt đối. Guard là **không gian người dùng** (canonicalize + kiểm thành phần), không phải Landlock;
hàng rào cứng trên WASI vẫn là preopen của runtime.

## Kiểm

```bash
cargo test                 # 27 test native: preopen/deny/symlink/ro/clock/stdio/refuse/parity
cargo clippy --all-targets -- -D warnings
cargo check --target wasm32-wasip1
bin/vis-wasm-suite --build  # 13 ca × interp/VM trên runtime WASI có sẵn
```

`bin/vis-wasm-suite` chọn runtime theo thứ tự `wasmtime` CLI → Python `wasmtime` → Node `node:wasi`;
thiếu cả ba ⇒ in `BLOCKED-ENV` (exit 0, trừ `--require-runtime`). Không tự tải binary nguồn lạ.
