# ST-299a — `WasiHost` lõi + runner `vis-wasm-run` (`wasm32-wasip1`)

**Nhánh**: `plan/w2-st299a-wasi` · **Baseline**: `e00ffa1e` (VD-282) · **Ngày**: 2026-11-23
**Phạm vi**: chỉ `vis-host-wasm/` + `bin/vis-wasm-suite`; không chạm workspace chính, không push/merge/tag.

## 1. Đã làm

### 1.1 `WasiHost` thật (0 `unsafe`)

Crate `vis-host-wasm` đặt `#![forbid(unsafe_code)]` (trình biên dịch cưỡng chế) và **không** thêm
dependency ngoài crate lõi (`vis-syntax`, `vis-types`, `vis-bytecode`, `vis-interp`, `vis-vm`,
`vis-diag`). Các phần:

- **`fs::WasiFs`** (`FsHost`): preopen ánh xạ `guest → host`; đường dẫn tuyệt đối khớp **theo thành
  phần** (dài nhất thắng), tương đối giải theo preopen đầu tiên. Kiểm containment bằng
  `canonicalize` + `starts_with` thành phần; thao tác ghi canonicalize **thư mục cha** và từ chối
  symlink ở đích cuối. Hỗ trợ đủ trait: `read/write/exists_quyen/list/list_entries/read_lines/append/
  mkdir/mkdir_all/remove/walk/copy/rename/size/is_file/is_dir/is_symlink/read_link/canonicalize/
  current_dir/read_bytes/write_bytes`.
- **`dong_ho`**: `bay_gio_ms`, `bay_gio_giay`, `don_dieu_ms` — Rust std đi qua WASI
  `clock_time_get`; không `unsafe`, không dependency.
- **`stdio::Stdio`**: `Dich::That` ghi fd 1/2, `Dich::Bo` gom bộ đệm; broken pipe trả `Err`,
  không panic.
- **Từ chối rõ** (`tu_choi`):
  - `TuChoiNet`: mọi cửa (`connect`, TLS, listen, accept, read/write, HTTP/2, `check_net`) ⇒ `Err`
    chứa `net` + `ST-299a`.
  - `TuChoiProc`: `Ngoai::chay`/`chay_day_du` ⇒ trap **`VS0604`** với thông điệp `proc`.
  - `TuChoiDb`: mọi thao tác ⇒ `Err` chứa `sqlite`; `db_cleanup` ⇒ `Ok(0)`.
- **`WasiHost::tach()`** cho mượn rời `fs/net/proc/db` để truyền đồng thời vào runtime, không
  `RefCell`/`unsafe`.

Thông điệp từ chối là hằng ổn định: `LY_DO_MANG`, `LY_DO_PROC`, `LY_DO_SQLITE`.

### 1.2 Runner `vis-wasm-run`

- Pipeline **một tệp**: `parse → lower (std_mods) → tyck → interp | compile+VM`.
- Hỗ trợ `--interp` (mặc định), `--vm`, `--preopen guest[=host][@ro]`, `--stdin`, `-- args…`.
- `.vish` tự chọn `Mode::Command` (giữ luật bỏ `main` khi có câu lệnh cấp tệp — VD-111).
- Async: chọn frame scheduler của interp/VM khi HIR có `async fn`.
- Ánh xạ exit: front end **2**, codegen **1**, trap `VS05xx` **101**, từ chối preopen (`VS0604`) **2**,
  còn lại theo `RunOutput.exit`.
- Không `std::thread` (WASI không có luồng), không cache/đa tệp (để ST-299b nối `vis-loader`).
- Không rò đường dẫn host: `canonicalize`/`current_dir` trả đường dẫn **khách**.

### 1.3 Smoke harness `bin/vis-wasm-suite`

13 ca × 2 đường = **26 lần chạy wasm**, gồm: số học/chuỗi/argv, đọc–ghi–list trong preopen,
preopen `@ro`, thoát gốc `..`, symlink ra ngoài, từ chối `net`/`sqlite`/`proc`, đồng hồ.
Chọn runtime: `wasmtime` CLI → Python `wasmtime` → Node `node:wasi`; thiếu cả ba ⇒ `BLOCKED-ENV`
(exit 0, trừ `--require-runtime`). Không tự tải binary nguồn lạ.

## 2. Bằng chứng máy

| Bước | Kết quả |
|---|---|
| `rustup target add wasm32-wasip1` | **OK** (`rustup target list --installed` có `wasm32-wasip1`) |
| `cargo build --release --target wasm32-wasip1` | **OK** — `vis-wasm-run.wasm` **3.430.132 byte** |
| `cargo check --target wasm32-wasip1` | **OK** |
| `cargo test` (native) | **27/27** (17 unit + 10 integration `tests/wasi_fs.rs`) |
| `cargo clippy --all-targets -- -D warnings` | **OK** |
| `cargo fmt --all -- --check` | **OK** |
| `grep -rn unsafe vis-host-wasm/src vis-host-wasm/tests` | chỉ có `#![forbid(unsafe_code)]` + chú thích; **không** có khối `unsafe` |
| `./vis-dev rules` (R1..R6) | **OK** |
| `vis-wasm-suite --runtime python` (wasmtime **51.0.0** official wheel) | **26/26 OK** |
| `vis-wasm-suite --runtime node` (Node **26.5.0**, WASI preview1) | **26/26 OK** |

Log đầy đủ (ngoài repo): `/home/ps/tmp/st299a/suite-python-venv.log`, `suite-node.log`.

### Trạng thái `wasmtime`

- **CLI `wasmtime` KHÔNG có trên `ps`** (`which wasmtime` rỗng) ⇒ nhánh CLI của harness **chưa chạy
  thật** — ghi nhận là **BLOCKED-ENV cho CLI**, không tuyên bố đã kiểm.
- Đã cài **official Python package `wasmtime 51.0.0`** trong venv tạm `/home/ps/tmp/venv-wt`
  (9,5 MB, không commit, không thêm vào Cargo.toml) và chạy **đúng engine wasmtime/Cranelift**:
  26/26 ca xanh. Đây là bằng chứng chạy wasm thật, không phải mô phỏng native.
- Node 26.5.0 (`node:wasi`, WASI preview1) là runtime độc lập thứ hai: 26/26 xanh.
- Không tải binary lạ; không cài `wasmtime-cli` bằng cargo (tránh build/đĩa lớn trong time-box).

## 3. Giới hạn & bàn giao

- Guard preopen là **user-space** (canonicalize + kiểm thành phần), **không** phải Landlock; hàng
  rào cứng trên WASI vẫn là preopen của runtime. Không tuyên bố cách ly kernel.
- Symlink mà `canonicalize` không phân giải được (WASI trả `ENOENT` cho đích ngoài preopen) bị
  **từ chối như nỗ lực thoát gốc** (`VS0604`) — fail-closed; đã kiểm trên wasmtime/Node.
- Chưa nối `vis-loader`/đa tệp/`use` — thuộc **ST-299b**.
- Chưa có bộ `test/sem` đầy đủ trên wasm — thuộc **ST-300**.
- `WasiHost::tu_moi_truong` đọc `VIS_WASM_PREOPENS`; trên wasm còn quét argv `guest::host` do
  wasmtime chèn. Chương trình nhiều preopen nên dùng đường dẫn tuyệt đối.

## 4. Tệp

- `vis-host-wasm/Cargo.toml`, `Cargo.lock` (workspace riêng, path deps crate lõi)
- `vis-host-wasm/src/{lib,preopen,fs,stdio,dong_ho,tu_choi,runner}.rs`
- `vis-host-wasm/src/bin/vis-wasm-run.rs`
- `vis-host-wasm/tests/wasi_fs.rs`
- `vis-host-wasm/README.md`
- `bin/vis-wasm-suite`
