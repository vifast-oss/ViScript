<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-307 — Plugin động WASI/WASM có kiểm soát quyền (`vs3_plugin.rs`)

> **Trạng thái: DONE (local, chưa merge).** Nhánh `plan/w4b-st307-plugin-wasi` từ **`90b2964`**
> (ST-306 plugin ABI, chưa merge). Không merge/push/tag; **không ssh mps/psvn** (m2/Windows quick
> đang chạy). `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Nạp plugin động WASI/WASM | **có** — `PluginWasi::nap` kiểm magic/version wasm + export `_start` + marker `vis_plugin_abi_v1` **trước khi chạy** |
| Kiểm khớp `VIS_PLUGIN_ABI_V1` | **có** — marker trên byte wasm + handshake `VIS_PLUGIN_ABI_V1=1` + `manifest abi=1` (ba lớp) |
| ABI lệch / plugin lạ | **từ chối rõ** — `VS0501` kèm ABI/`_start`/magic; test âm đột biến wasm thật |
| Sandbox theo `cap[...]` | **có** — chỉ preopen cap được trao (`/cap0`, `/cap1`, …); ngoài root bị WASI từ chối (`VS0604`) |
| Cap ghi cần `write` | **có** — cap `ro` preopen qua **bản sao chỉ-đọc** (WASI P1 không có cờ ro); ghi bị OS từ chối, dữ liệu gốc không đổi |
| Tắt cứng `--strict-caps`/`--sandbox` | **có** — `ChinhSach::kiem_tra()` chạy **trước** khi đọc tệp (`VS0604`); test dùng tệp không tồn tại để chứng minh |
| Plugin mẫu parity interp↔VM | **có** — `test/plugin-wasi/mau.rs` dựng `wasm32-wasip1`, chạy **Node `node:wasi`**; 3 đường cùng `Int(5)` |
| 0 `unsafe` | **có** — `#![forbid(unsafe_code)]`; plugin mẫu chỉ `std`, không `unsafe` |
| Test plugin | **10/10** (6 `vis-plugin-abi` + 4 `vis-cli`), không `#[ignore]`, không bỏ ca |
| Gate Linux | **PASS — 3690 passed / 0 failed / 4 skipped** (baseline ST-306 3680/4; +10 ca) |
| fmt / clippy / R1..R7 | **PASS** (`vis-dev check`) |
| Log ngoài repo | `/home/ps/vislogs/st307-gate.log` + 2 log phụ, sha256 bên dưới |
| Blocker | **không** |

## 1. Thiết kế

### 1.1 Hợp đồng plugin WASI

Plugin là **WASI command** (`wasm32-wasip1`, export `_start`) và export marker
`vis_plugin_abi_v1`. Host kiểm **byte wasm** trước khi chạy: magic `\0asm`, version 1, export
`_start` (kind 0), đúng một marker `vis_plugin_abi_v<N>`; `N != 1` ⇒ `VS0501` nêu rõ ABI. Sau đó
host chạy chế độ `abi` và đối chiếu `VIS_PLUGIN_ABI_V1=1`, rồi chạy `manifest` để lấy module/hàm/
cap yêu cầu. Ba lớp này chặn cả wasm lạ lẫn wasm đúng định dạng nhưng khai ABI khác.

### 1.2 Sandbox `cap[Dir]`

Host chỉ preopen đúng các `Cap` được trao, guest path `/cap0`, `/cap1`, …; plugin không có quyền
ngầm, không đọc env. Đường ngoài preopen (kể cả `/etc/passwd` và `/data/../outside`) bị WASI từ
chối. Vì WASI Preview 1 trên `wasmtime`/`node:wasi` **không có cờ preopen chỉ-đọc**, cap
`write = false` được preopen qua **bản sao tạm** rồi hạ quyền filesystem (`r-x`/`r--`, tối đa
4096 tệp / 64 MiB / sâu 32); plugin đọc được, ghi bị OS từ chối, dữ liệu gốc không đổi, bản sao
dọn khi `PluginWasi` drop. Cap `write = true` preopen thẳng thư mục gốc.

### 1.3 Tắt cứng

`ChinhSach { strict_caps, sandbox }` được kiểm **đầu tiên** trong `PluginWasi::nap`, trước
`fs::read`; vi phạm ⇒ `VS0604` nêu đúng cờ. CLI `vis plugin` map trực tiếp `--strict-caps` /
`--sandbox`; test âm dùng đường dẫn không tồn tại và khẳng định thông điệp **không** phải lỗi đọc
tệp (chứng minh không có đường vòng).

### 1.4 Runtime & giao thức

Dò runtime theo đúng VD-283: `wasmtime` CLI → Python `wasmtime` → Node `node:wasi`; ghim bằng
`VIS_WASI_RUNTIME`. Giao thức argv `abi|manifest|call <hàm> <tham số…>`, stdout:
`VIS_PLUGIN_ABI_V1=1` / các dòng `module=…`/`abi=…`/`func=…`/`cap=…`/`ok` / `ok\n<giá trị>` hoặc
`err\n<VSxxxx>\n<hex>`. Codec giá trị tự tả (`u:`, `b:1:`, `i:5:`, `f:1.5:`, `s:<n>:<hex>`,
`y:`, `l:`, `m:`, `o:`, `r:`, `c:`) — không cần JSON, không panic trên dữ liệu lạ. Mỗi lượt chạy
có hạn 20 s (`VIS_PLUGIN_TIMEOUT_MS`), stdout/stderr ghi tệp tạm rồi poll `try_wait` để tránh
treo gate.

### 1.5 CLI

`vis plugin check <wasm>` in module/ABI/runtime/hàm/cap; `vis plugin run <wasm> <hàm> [giá trị…]`
gọi hàm; `--cap-dir <path>[=ro|rw]` lặp được. Exit 0 sạch, 2 lỗi dùng/quyền/ABI (cùng quy ước
`vis run`).

## 2. Tệp chạm

- `crates/vis-plugin-abi/src/wasi.rs` (mới, 1.354 dòng ≤ R1 1.500): loader + codec + runtime +
  parser export wasm + bản sao cap ro.
- `crates/vis-plugin-abi/src/lib.rs`: `pub mod wasi` + re-export.
- `crates/vis-plugin-abi/tests/vs3_plugin.rs` (mới): 6 ca loader/WASI.
- `crates/vis-cli/src/cli/plugin.rs` (mới): `vis plugin check|run`.
- `crates/vis-cli/src/{main.rs,cli/mod.rs}`, `crates/vis-cli/Cargo.toml` (+`vis-plugin-abi` path).
- `crates/vis-cli/tests/vs3_plugin.rs` (mới): 4 ca CLI; `tests/common/mod.rs` thêm helper dựng
  plugin mẫu.
- `test/plugin-wasi/mau.rs` (mới): plugin mẫu `wasm32-wasip1`, chỉ `std`.
- `Cargo.lock`: chỉ thêm cạnh `vis-plugin-abi` trong gói `vis-cli` (không thêm crate ngoài).

Không đổi `SPEC`, `API-CODES.txt`, ngữ nghĩa ngôn ngữ, golden, hay ngưỡng cổng. Dùng mã đã cấp:
`VS0604` (quyền/tính năng), `VS0501` (ABI/runtime/kiểu).

## 3. Test đỏ trước (bằng chứng)

Lần chạy đầu 6 ca loader: **5 passed / 1 failed** — đúng lý do thiếu cưỡng chế ghi:

```text
---- cap_dir_la_sandbox_tu_nhien stdout ----
thread 'cap_dir_la_sandbox_tu_nhien' panicked at crates/vis-plugin-abi/tests/vs3_plugin.rs:193:
cap chỉ đọc không được ghi: Unit
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.74s
```

Nguyên nhân: `node:wasi` preopen cấp quyền ghi dù host muốn `ro`. Sửa bằng bản sao chỉ-đọc +
hạ quyền filesystem; chạy lại **6/6**, rồi thêm 4 ca CLI **4/4**.

## 4. Bằng chứng máy

- `./vis-dev check`: **PASS** — fmt, clippy `-D warnings`, R1..R7, toolchain, release, báo cáo,
  self-test âm, build, doc-examples 24 khối, readme 20/20. Log:
  `/home/ps/vislogs/st307-check.log`,
  sha256 `918c483bb26b19cbe46a2c5b271799ba072a2fd14aac72499a78ea42f4219fca`.
- `./vis-dev gate --fast`: **PASS — 3690 passed / 0 failed / 4 skipped** (71,0 s nextest +
  doc-test). Log: `/home/ps/vislogs/st307-gate.log`,
  sha256 `e89c8076ab61ef1931d2872b566d9550fdaaed28ab48baf562895d81f317f5ba`.
- Test plugin: `cargo test -p vis-plugin-abi --test vs3_plugin -p vis-cli --test vs3_plugin`
  ⇒ **6/6 + 4/4**. Log: `/home/ps/vislogs/st307-plugin-tests.log`,
  sha256 `9de91921de2bf52e47ae65e53d095b35b93d26b8cf1f9bf10f738f21a0e6228f`.
- Demo CLI: `/home/ps/vislogs/st307-cli-demo.log`,
  sha256 `2cc032e3c8ca9c025b03c4d1e985e5fbfed3a89dd35a988700d4468c3362eb4d`.
- `wc -l crates/vis-plugin-abi/src/wasi.rs` = **1354** (< 1.500); `grep unsafe` chỉ thấy câu
  doc “0 `unsafe`”, không có mã.

## 5. Giới hạn có chủ đích

- Bản này chỉ nhận `cap[Dir]`; `cap[Net]`/`cap[Proc]` cần host WASI tương ứng (ngoài phạm vi
  ST-307).
- Cap `ro` dùng bản sao tạm (giới hạn 64 MiB/4096 tệp); cây lớn hơn bị từ chối rõ thay vì cấp
  quyền ghi. Đây là hệ quả của WASI P1 không có preopen chỉ-đọc, không phải nới quyền.
- Runtime là tiến trình ngoài (`wasmtime`/Python/Node), không nhúng wasmtime vào `vis` — đúng
  chính sách VD-283 và giữ 0 `unsafe`/không thêm crate nặng.
- Windows: logic `cfg(not(unix))` dùng `readonly` của std; **đã chạy** ở W4b — xem
  [`st307-win-tests.md`](st307-win-tests.md): 6/6 WASI thật + CLI 4/4 trên `psvn`.

## 6. Việc còn lại

- Chờ A0 merge `ST-306` rồi `ST-307`; không merge/push/tag.
- W4b: **xong** — `vs3_plugin` 6/6 + CLI 4/4 trên Windows; chi tiết ở
  [`st307-win-tests.md`](st307-win-tests.md). Còn nhánh runtime `wasmtime` CLI (nếu có máy cài)
  để xác nhận thứ tự dò; `psvn` hiện chỉ có Node.
