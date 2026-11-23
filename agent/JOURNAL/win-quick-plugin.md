<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows quick plugin — ST-306 `90b2964` + ST-307 `3d247c4`

> **Người đo**: subagent (context sạch), 2026-11-23. Không merge/push/tag; **không ssh `mps`**.
> **Nguồn**: fetch `origin` GitHub (hai nhánh đã push), worktree **riêng** trên `psvn`:
> `/c/Users/ps/wq-w4-st306` @ `90b2964ffc315b57d7b2f3907f6ddfe2289b11d5` và
> `/c/Users/ps/wq-w4b-st307` @ `3d247c418b03208bbbe02dead22cc243021969bb`.
> `TMPDIR=/c/Users/ps/tmp`, `CARGO_BUILD_JOBS=3`, chạy **tuần tự**, không job `psvn` song song.
> **Kết quả: PASS** cả hai nhánh — 0 test đỏ, 2 cổng quick xanh; kèm **2 phát hiện portability
> test Windows** (không phải lỗi sản phẩm) và 1 sự cố hạ tầng của người đo (đã phân loại).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| ST-306 `cargo test -p vis-plugin-abi -p vis-plugin-macros` | **PASS — 8 passed / 0 failed** (2 unit + 6 parity; macros 0 test, 1 doc-test ignored) |
| ST-306 `bin/vis-gate-windows --quick` | **PASS — 91/91** (pkg 25 + h2 33 + cancel 8 + select 25), exit 0 |
| ST-307 `cargo test -p vis-plugin-abi` (mặc định) | **14/14 pass**, nhưng **4/6 ca WASI tự `BLOCKED-ENV`** (xem §5.1) |
| ST-307 `cargo test -p vis-plugin-abi` + `VIS_RUSTC` | **PASS — 6/6 ca WASI chạy thật** (2,04 s; wasm 2 224 456 B) |
| ST-307 `cargo test -p vis-cli --test vs3_plugin` | **PASS — 4/4** (2,13 s) |
| ST-307 `bin/vis-gate-windows --quick` | **PASS — 91/91**, exit 0 |
| `vis plugin check\|run` mẫu trên `vis.exe` Windows | **PASS** — check đúng manifest, `cong(2,3)=5`, cap `ro`/`rw`/strict đúng |
| Sandbox thật trên Windows (`/cap0/../…`) | **BỊ CHẶN** — `VS0604`, `Capabilities insufficient (os error 76)` |
| Blocker sản phẩm | **không** |
| Blocker tuyên bố “Windows xanh đầy đủ” | **có, nhẹ** — 2 lỗi test-only §5; sửa 2 chỗ là xong |

## 1. Môi trường

- `psvn`: `MINGW64_NT-10.0-26300`, 6 nhân; `cargo/rustc 1.100.0` (2026-11-05/09); Node **v24.19.0**
  (`node:wasi`); **không** có `wasmtime` CLI lẫn Python `wasmtime` ⇒ runtime WASI = **Node**.
- **Thay đổi môi trường có chủ đích**: `rustup target add wasm32-wasip1` (trước đó **thiếu**; chỉ có
  `wasm32-unknown-unknown`). Không có target này thì plugin mẫu không dựng được và toàn bộ ca WASI
  chỉ còn là `BLOCKED-ENV`.
- `CARGO_BUILD_JOBS=3` được áp bằng `.cargo/config.toml` tạm (`[build] jobs = 3`) trong từng
  worktree — `vis-gate-windows` dựng thân lệnh riêng nên biến môi trường ngoài không truyền qua
  `bash -lc`. Tệp tạm **đã xoá**; `git status --porcelain` của cả hai worktree **rỗng** sau cùng.
- Không sửa mã nguồn nhánh; chỉ đọc/compile/test.

## 2. ST-306 — `plan/w4-st306-plugin-abi` @ `90b2964`

```bash
cargo test -p vis-plugin-abi -p vis-plugin-macros
bin/vis-gate-windows --quick   # VIS_GATE_WINDOWS_REPO=/c/Users/ps/wq-w4-st306
```

- Test: **8/8** — `vis-plugin-abi` 2 unit + `tests/parity.rs` 6; `vis-plugin-macros` 0 test,
  1 doc-test `ignored`; `cargo_exit=0`.
- Gate quick: **91/91**, exit 0 (pkg 25, h2 33, cancel 8, select 25); build lần đầu 45,3 s trên
  target riêng.
- Log: `/home/ps/vislogs/wq-st306-cargo-test.log`, `/home/ps/vislogs/wq-st306-gate-quick.log`.

## 3. ST-307 — `plan/w4b-st307-plugin-wasi` @ `3d247c4`

### 3.1 `cargo test -p vis-plugin-abi`

- Mặc định: **14/14 pass** (2 unit + 6 parity + 6 `vs3_plugin`), nhưng 4 ca WASI in
  `BLOCKED-ENV: thiếu runtime WASI hoặc toolchain wasm` / `không dựng được wasm32-wasip1`
  và `return` sớm ⇒ “xanh” nhưng **không đo**. `target/plugin-wasi/` rỗng.
- Nguyên nhân: `tim_rustc()` của test không thấy `rustc.exe` trên Windows (§5.1), **không phải**
  thiếu target (đã cài) hay thiếu runtime (Node có).
- Ép bằng đúng override có sẵn của test:
  `VIS_RUSTC=C:/Users/ps/.cargo/bin/rustc.exe cargo test -p vis-plugin-abi --test vs3_plugin -- --nocapture`
  ⇒ **6/6 chạy thật**, 2,04 s, 0 dòng `BLOCKED-ENV`; wasm mẫu **2 224 456 B**
  (sha256 `1b2e02c59ec9c49c9a786a3c614c3ae374552d395038a4dadde1eaaf86c7abc1`).
- `cargo test -p vis-cli --test vs3_plugin` (thêm, cùng override): **4/4**, 2,13 s.

### 3.2 `vis plugin check|run` trên binary thật

`target/debug/vis.exe` — `vis 0.4.5`, 18 840 064 B, sha256
`ff4c5bdcc910516258a5bc16e2fca1d34e82c806d123121236b8aa1427e174f6`.

```text
$ vis plugin check target/plugin-wasi/mau.wasm
plugin: mau
abi: 1
runtime: node
hàm: cong(int, int) -> int
hàm: doc(str) -> str
hàm: ghi(str, str) -> unit
cap yêu cầu: Dir (đọc)
```

| Ca | Lệnh | Kết quả |
|---|---|---|
| Gọi hàm | `plugin run … cong int:2 int:3` | `5`, exit 0 |
| Cap `ro` đọc | `plugin run … doc str:/cap0/in.txt --cap-dir D=ro` | `xin chào wasi`, exit 0 |
| Cap `ro` ghi | `plugin run … ghi … --cap-dir D=ro` | `VS0604` “Operation not permitted (os error 63)”, exit 2; tệp gốc **không đổi** |
| Cap `rw` ghi | `… ghi … --cap-dir D=rw` | `()`, exit 0; nội dung đổi `da-ghi` |
| Thoát cap `..` | `doc str:/cap0/../wq-outside/secret.txt --cap-dir D=ro` | `VS0604` “Capabilities insufficient (os error 76)”, exit 2 |
| Ngoài preopen | `doc str:/etc/passwd` | `VS0604`, exit 2 — **nhưng** trên Windows `/etc/passwd` không tồn tại (xem §5.2) |
| Tắt cứng | `plugin check <không tồn tại> --strict-caps` | `VS0604` “bị tắt cứng…”, exit 2 (chặn **trước** khi đọc tệp) |

### 3.3 `bin/vis-gate-windows --quick`

- **91/91**, exit 0 (pkg 25, h2 33, cancel 8, select 25); build 40,7 s trên target riêng.
- `--quick` **không** gồm `vs3_plugin` (mới của ST-307); đã chạy riêng: 4/4 (§3.1).
- Log: `/home/ps/vislogs/wq-st307-gate-quick.log`.

## 4. Sự cố hạ tầng của người đo (đã phân loại — **không phải lỗi nhánh**)

Lần chạy ST-307 **đầu tiên** đỏ khi compile test:

```text
error[E0432]: unresolved imports `vis_plugin_abi::ChinhSach`, `vis_plugin_abi::PluginWasi`
error[E0433]: cannot find `wasi` in `vis_plugin_abi`
```

- Nguồn đã commit **có** `pub mod wasi;` + re-export (`lib.rs:19,21`); worktree `3d247c4` sạch.
- Nguyên nhân: người đo dựng **một target dir chia sẻ** cho hai worktree (junction). Cargo 1.100
  xếp unit theo `target/debug/build/<pkg>/<hash>`; dep-info của lần build ST-307 trỏ
  `wq-w4-st306` (đã kiểm bằng `grep -o "wq-w4[a-z0-9-]*" *.d`), và artifact ST-306 (mtime 14:21)
  được coi là “mới” so với nguồn ST-307 (14:20) ⇒ **tái dùng artifact cũ không có module `wasi`**.
- Xử lý: `cmd rmdir` gỡ junction, tạo `target/` thật **riêng từng worktree**, chạy lại ⇒ 14/14.
  Không retry mù; kết luận dựa trên dep-info + nội dung `lib.rs` của commit.
- Bài học: **không** chia sẻ `target/` giữa hai worktree với Cargo 1.100 (hoặc phải `cargo clean`
  / `touch` nguồn trước khi đổi cây). Log lần đỏ giữ lại: `/home/ps/vislogs/wq-st307-cargo-test.log`.

## 5. Phát hiện portability test trên Windows (test-only, cần sửa nhỏ)

### 5.1 `tim_rustc()` không thấy `rustc.exe` ⇒ 4/6 ca WASI **tự bỏ qua**

- `crates/vis-plugin-abi/tests/vs3_plugin.rs`: `tim_rustc()` thử `VIS_RUSTC`,
  `HOME/.cargo/bin/rustc`, rồi `PATH/*/rustc` — **không** thử `rustc.exe`. Trên Windows tiến trình
  native nhận `HOME=C:\Users\ps` và chỉ có `rustc.exe`; probe xác nhận
  `home_candidate is_file=false`, `path_has_rustc_exe=true` ⇒ hàm trả `None` ⇒ `plugin_mau()`
  trả `None` ⇒ 4 ca `eprintln!("BLOCKED-ENV…"); return;` **pass mà không đo**.
- Bằng chứng: `target/plugin-wasi/` rỗng; `--nocapture` in 4 dòng `BLOCKED-ENV`; đặt
  `VIS_RUSTC` trỏ `rustc.exe` ⇒ wasm được dựng, 6/6 chạy thật 2,04 s.
- Đề xuất (chưa sửa, ngoài phạm vi đo): trên Windows thử thêm `rustc.exe`, hoặc bỏ điều kiện
  `is_file()` và để `Command::new("rustc")` tự dò; giữ `VIS_RUSTC` làm override. Tương tự cho
  `crates/vis-cli/tests/common/mod.rs` (ở đó có fallback `Command::new("rustc")` nên nhiều khả năng
  vẫn chạy, nhưng nên thống nhất).

### 5.2 Ca “ngoài preopen” dùng `/etc/passwd` — vô hiệu trên Windows

- Cả `vis-plugin-abi/tests/vs3_plugin.rs` và `vis-cli/tests/vs3_plugin.rs` dùng `/etc/passwd` làm
  đường ngoài cap. Trên Windows tệp này **không tồn tại**, nên lý do từ chối là `ENOENT`
  (“No such file or directory”), không chứng minh sandbox.
- Người đo kiểm tay đường thoát thật: `/cap0/../wq-outside/secret.txt` (tệp **có thật** ngoài cap)
  ⇒ `VS0604` “Capabilities insufficient (os error 76)” — sandbox **đúng** trên Windows.
- Đề xuất: dùng tệp ngoài thật + `..` (hoặc đường tuyệt đối của host) thay `/etc/passwd`.

## 6. Bằng chứng log ngoài repo + sha256

| Log local (`/home/ps/vislogs/`) | sha256 |
|---|---|
| `wq-st306-cargo-test.log` | `8ae0b7705c4dcfd30ccce65cf2c5501b917bc24facac8cd2115dc69190e34644` |
| `wq-st306-gate-quick.log` | `83677195fb05621b880912bb244f88f76aaddc58c9cf6a85a931ad2187cdc655` |
| `wq-st307-cargo-test.log` (lần đỏ do target chia sẻ) | `eb73474dcabde2b8e271bd7fe1aa637beacbf4410cf9b0d117e5d4dc72d21dbf` |
| `wq-st307-cargo-test-r2.log` (14/14) | `32704376dc1cb6d000eaa4a6647fc425f1fafc98a5047129da2a7c944c273040` |
| `wq-st307-gate-quick.log` (91/91) | `ff2b160f4c8b3149f8c15adce74fb46fa24d5074b8176efebf7ed8d37768f99b` |
| `wq-st307-vis-rustc.log` (6/6 thật) | `bd0afbc8b06912772139de3f094c44bf016c0ea3223434786048b8bac95aff0a` |
| `wq-st307-cli-demo.log` (4/4 + demo) | `a287e7fe60a565fc9cfe2ac769121fd55fba506099228e00ba9e9d695c484a4b` |
| `wq-st307-escape.log` (thoát cap) | `7679a9ffcfdb121027ae28a49e6a9a8f7c7c37645d229c89216a8fcc0df3269d` |

- Bản trên `psvn` ở `/c/Users/ps/vislogs/` (đường dẫn ghi trong header từng log); ví dụ gate
  ST-306 `vis-remote-20261123T052214Z.log` (sha256 trên máy đích
  `30cb423ba6779e3a6795516a0e8e9756cc7f031a9d9b002edd117bc8181b58b0`), gate ST-307
  `vis-remote-20261123T052647Z.log` (`b1fb60c1ff41c76c357b197490caf25d90fbf4ba5c1ba64f69a0b6f94ed778d5`).
- Log chứng minh skip (không có bản local): `/c/Users/ps/vislogs/vis-remote-20261123T052943Z.log`
  (`--nocapture`, 4 dòng `BLOCKED-ENV`).

## 7. Kết luận / blocker

- **ST-306**: PASS đầy đủ (8/8 + quick 91/91).
- **ST-307**: PASS sản phẩm — 14/14 + CLI 4/4 + quick 91/91 + demo `check|run` + sandbox `..`
  đúng; **cần `VIS_RUSTC`** để 6/6 ca WASI chạy thật trên Windows.
- **Blocker**: không có blocker sản phẩm. Còn **2 việc test-only** trước khi tuyên bố “Windows
  xanh đầy đủ không cần override”: (1) `tim_rustc()` nhận `rustc.exe`; (2) ca ngoài preopen dùng
  tệp thật thay `/etc/passwd`.
- Không merge/push/tag; không ssh `mps`; worktree `psvn` để lại
  `/c/Users/ps/wq-w4-st306` + `/c/Users/ps/wq-w4b-st307` (sạch, target riêng 0 / 1,9 GB).
