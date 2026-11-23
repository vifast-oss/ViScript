<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310c — Tách guest ABI `vis-plugin-abi` (0 dep) vs host `vis-plugin-host`, xoá `cap_fs.rs`

**Nhánh**: `plan/st310c-plugin-host`, tạo từ **`4746799`** (`ST-310a` core). **Không**
merge/push/tag; không ssh `mps`/`psvn`. Worktree riêng `/home/ps/tmp/st310c-wt` (không checkout
`/home/ps/ViScript`). Môi trường: Rust **1.100.0**, `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0
TMPDIR=/home/ps/tmp`.

## 1. Phạm vi và kết quả

| Hạng mục (`VD-285` ST-310c) | Kết quả |
|---|---|
| `vis-plugin-abi` mỏng **~350 LOC** | **350** dòng (`src/lib.rs`), còn đúng guest ABI: `Error`, `Cap`, `PluginValue`, `Param`/`Signature`/`FuncDef`/`PluginModule`, `PluginCtx` |
| **0 dep workspace** | `[dependencies]` rỗng; `cargo tree -p vis-plugin-abi` chỉ in chính nó; `Cargo.lock` không còn cạnh `rustix`/`vis-interp` |
| Build `wasm32` **< 1 s** | `cargo build -p vis-plugin-abi --target wasm32-wasip1` = **0,22 s** (warm; lần đầu 0,26 s) |
| Tạo `crates/vis-plugin-host` | Có: `wasi/{protocol,runner,sandbox}.rs` + `static_reg.rs` + `convert.rs`; loader `PluginWasi` ở `wasi/mod.rs` |
| Mỗi tệp **< 650 LOC** | lớn nhất **601** (`wasi/protocol.rs`); kế tiếp 376 (`runner`), 307 (`sandbox`), 186 (`wasi/mod`), 164 (`static_reg`), 136 (`convert`) |
| Xoá `cap_fs.rs` trùng lặp | Đã xoá; plugin tĩnh dùng **một** nguồn `vis_host_native::quyen::mo_tep_doc`/`mo_tep_ghi` (nội bộ `mo_goc`) |
| **0 bản sao `openat2`** ngoài `vis-host-native::quyen` | `grep openat2/ResolveFlags` ngoài `crates/vis-host-native` chỉ còn **chú thích** |
| Rà `statat` cho **#194**, không đổi semantics | `quyen_mo::doi_ten` fallback: `statat(fd_b, ten_b, SYMLINK_NOFOLLOW)` trên **fd cha cố định**, không đi theo thành phần; thêm chú thích, **0 đổi mã** |
| Test F1/F2 + `vs3_plugin` xanh Linux | F1 `quyen_st295::goc_bi_thay_bang_symlink_trung_gian…` OK; F2 `parity::cap_ghi_khong_di_theo_symlink_o_ten_cuoi` OK; `vs3_plugin` host 7/7 + CLI 4/4 (Node `node:wasi`) |
| 0 `unsafe` | `#![forbid(unsafe_code)]` ở cả hai crate; R2 xanh |
| `vis-host-wasm` xanh | **12/12** test |
| `./vis-dev gate --fast` | **PASS** — nextest **3761 passed / 0 failed / 4 skipped** (+doc-test riêng OK) |

## 2. Kiến trúc sau tách

### 2.1 `crates/vis-plugin-abi` — guest ABI (0 dep)

- Giữ nguyên `VIS_PLUGIN_ABI_V1 = 1`, `Error` (`VS0604`/`VS0501`), `Cap`, `PluginValue` (kèm
  `TryFrom<PluginValue>`/`From<T>` cho kiểu nguyên thuỷ), `Param`/`Signature`/`FuncDef`/
  `PluginModule` — đúng những gì macro `#[vis::module]` sinh ra cần.
- **Không** còn `vis-interp`: chuyển đổi `Value` ↔ `PluginValue` chuyển sang host (`convert.rs`).
- **Không** còn `wasi.rs`/`cap_fs.rs`/`rustix`: loader + I/O cap chuyển sang host.
- `PluginCtx` giữ **nguyên API** `read_text`/`write_text` cho plugin tĩnh, nhưng thao tác mở tệp
  do host cài qua `PluginIo` (hai hàm `fn(&Path, &Path, …) -> Result<_>`); guest ABI không chứa
  syscall. Không có I/O ⇒ `VS0501` nói rõ cần `PluginCtx::voi_io`.

### 2.2 `crates/vis-plugin-host` — host

- `convert.rs`: `tu_value`/`sang_value` (chuyển nguyên quy tắc ST-306/ST-309: hàm/task/kênh
  không qua biên; `Option`/`Result` chỉ đúng enum chuẩn).
- `static_reg.rs`: `ctx_voi_host(caps)` cài I/O; `SoDangKy` đăng ký module tĩnh (chống trùng
  tên). Unix dùng `vis_host_native::quyen::{mo_tep_doc, mo_tep_ghi}` + `la_loi_symlink`
  (errno theo nền: Darwin `ELOOP = 62`); Windows giữ đường `canonicalize` cũ.
- `wasi/protocol.rs`: codec văn bản, manifest, kiểm ABI trên byte wasm (magic/`_start`/marker).
- `wasi/runner.rs`: dò `wasmtime` → Python `wasmtime` → Node; driver riêng mỗi lượt; hạn chờ/kill.
- `wasi/sandbox.rs`: `ChinhSach` (tắt cứng `--strict-caps`/`--sandbox` trước khi đọc tệp), bản
  sao chỉ-đọc (bỏ symlink, trần 4096 tệp/64 MiB/32 cấp).
- `wasi/mod.rs`: `PluginWasi::nap`/`goi`, guest path `/cap0`, `/cap1`…; hành vi giữ nguyên.
- `vis-cli` chỉ còn dep `vis-plugin-host` (không dep trực tiếp `vis-plugin-abi`); `convert::sang_value`
  thay `TryFrom<Value>` ở `cli/plugin.rs`.

### 2.3 Xoá `cap_fs.rs`, thống nhất `openat2`

- Đường ghi F2 đi qua `vis-host-native::quyen::mo_tep_ghi` → `quyen_mo::ghi_de` →
  `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)` (fallback `O_NOFOLLOW` từng thành phần).
- Đường đọc dùng `mo_tep_doc` → `quyen_mo::doc` cùng cờ. Trên Linux bản cũ cho phép symlink
  **bên trong** root ở nhánh `openat2` (BENEATH không NO_SYMLINKS); nay siết `NO_SYMLINKS` cho
  khớp `FsGuard` và fallback macOS — không đổi hành vi quan sát được (đường đọc đã canonicalize
  trước khi mở; symlink xuất hiện ở đó chỉ là đua và phải bị chặn).
- **#194**: nhánh `doi_ten` dự phòng (BSD/không `renameat_with`) vẫn dùng `statat` trên fd cha +
  `SYMLINK_NOFOLLOW`; đây là kiểm tồn tại siêu dữ liệu **đã neo fd**, không phải truy vấn theo
  đường dẫn. Nợ #194 (`exists`/`size`/`list`/`walk` theo tên ở `FsHost`) **vẫn mở**, không đổi
  semantics trong lát này.

## 3. Kiểm chứng

- `cargo test -p vis-plugin-abi -p vis-plugin-host`: **24/24** (ABI 3 unit + host 7 unit +
  `parity` 7 + `vs3_plugin` 7).
- `cargo test -p vis-cli --test vs3_plugin --test security_it`: **83/83** (4 + 79), gồm F2 CLI
  `st309::symlink_ten_cuoi_ra_ngoai_root_bi_chan_ca_doc_va_ghi`.
- `cargo test -p vis-host-native`: **27/27**, gồm F1
  `quyen_st295::goc_bi_thay_bang_symlink_trung_gian_khong_duoc_ghi_ra_ngoai`.
- `vis-host-wasm`: **12/12**.
- `cargo fmt --all --check` OK; `cargo clippy -p vis-plugin-abi -p vis-plugin-host -p
  vis-host-native -p vis-cli --all-targets -- -D warnings` OK; clippy `wasm32-wasip1` cho
  `vis-plugin-abi` OK.
- `./vis-dev rules`: OK (R1..R7).
- `./vis-dev gate --fast`: **PASS** — nextest **3761 passed / 0 failed / 4 skipped**; fmt/clippy/R1..R7,
  toolchain, release, self-test, doc-examples, readme-examples đều OK.

## 4. Việc còn lại / blocker

- **Windows/macOS quick để sau** (mps bận): mã đường Windows dùng fallback `canonicalize` như
  trước; macOS đi nhánh `unix` (`openat2` fallback `O_NOFOLLOW`, errno Darwin đã dùng helper
  `rustix::io::Errno`). Chưa chạy `bin/vis-gate-windows/macos` trong lát này.
- Không có blocker kỹ thuật trên Linux. Không merge/push/tag; không GitHub Actions.
