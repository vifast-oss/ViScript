<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4-fix — sửa lỗi Windows `psvn` sau verify `v0.1.0-rc1`

> **Người sửa**: subagent VS3-W4 (Windows `psvn`), 2026-11-21.
> **Mốc trước**: `0985030` (`v0.1.0-rc1`) — báo cáo `agent/JOURNAL/vs3-w4-windows-verify.md` (`80ed2e5`).
> **Mốc sau**: `ed30ebb` — workspace Windows **xanh**, `vis-dev check` **PASS**.
> **Phạm vi**: chỉ sửa test/helper + một helper quyền (`quyen_chuan`); **không** đụng
> `net.rs`, `pkg.rs`, `frame/**`; **không** cài dependency/target; **không** push/merge.

## 0. TL;DR — trước → sau

| Hạng mục | Trước (`0985030`) | Sau (`ed30ebb`) |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **3218 passed / 38 failed / 3 ignored**; 20/329 block đỏ; exit 101 | **3258 passed / 0 failed / 3 ignored**; 329/329 block xanh; exit 0 |
| `bin/vis-dev check` | exit 1 — `fmt` OK, **clippy FAIL** (`unused variable: marker`) | **PASS** — fmt, clippy, R1..R6, toolchain, release, self-test, explain, report, build, doc-examples, readme-examples |
| `bin/vis-host-suite` | 54/54 PASS | **54/54 PASS** |
| `bin/vis-targets` | exit 1 — thiếu std target wasm32/Linux (BLOCKED-ENV) | vẫn BLOCKED-ENV (không cài target); test `targets_wasm_vis_types` nay **báo rõ và bỏ qua**, không đỏ |
| Cây `psvn` sau đo | `0985030`, sạch | `ed30ebb`, sạch (0 mục) |

**Kết luận**: đủ điều kiện **promote `v0.1.0`** theo nghĩa Windows Tier 2 (workspace + cổng + host suite
xanh trên máy Windows thật), **với một caveat**: ma trận cross-target `vis-targets` **chưa đo được**
trên `psvn` vì thiếu std target; đây là BLOCKED-ENV đã ghi rõ, không phải bằng chứng xanh.

## 1. Commit

| Commit | Nội dung |
|---|---|
| `a7d86e7` | `[VS3][W4-fix]` clippy `cfg(unix)`, cap symlink đích tương đối, 35 ca portability — 25 tệp, +402/−64 |
| `ed30ebb` | `[VS3][W4-fix]` `readme-examples` chọn `vis.exe` trên Windows — 1 tệp, +10/−5 |

Môi trường: `psvn`, `MINGW64_NT-10.0-26300 3.5.4-395fda67.x86_64`, 6 CPU;
cargo/rustc `1.100.0`; chỉ cài `x86_64-pc-windows-msvc`.

## 2. Nguyên nhân gốc và cách sửa

### 2.1. Clippy — `marker` chỉ dùng ở nhánh Unix

- `crates/vis-cli/tests/vd270_vistest_timeout.rs:75`, `don_con(_pid, marker)`:
  `marker` chỉ dùng trong `#[cfg(unix)]` (`pkill -f`); nhánh `#[cfg(windows)]` dùng `taskkill /T`.
- Sửa: thêm `let _ = marker;` trong khối Windows (giữ nguyên chữ ký/hành vi), kèm chú thích.
- Đây là **test-only**, nhưng là **cổng cứng** vì clippy chạy `-D warnings`.

### 2.2. Hai ca cap-confinement — symlink đích **tương đối** trên Windows

- Bằng chứng gốc: `vd270_fs_read_link::quyen_ngoai_root_bi_chan` exit 0/`OK`;
  `vd270_fs_canonicalize::quyen_ngoai_root_bi_chan` exit 0/`ERR`; cả hai phải `VS0604` exit 2.
- **Root cause đo được** (probe Rust ngoài repo, `/c/Users/ps/vislogs/probe/probe.rs`):
  `std::fs::canonicalize` trên Windows trả **`ERROR_INVALID_NAME` (123)** với symlink có **đích
  tương đối** (đích tồn tại), dù `read_link` đọc đích bình thường:
  ```text
  read_link=Ok("../ngoai.txt")
  canon_link=Err(Os { code: 123, kind: InvalidFilename, ... })
  canon_root=Ok("\\\\?\\C:\\...\\root")
  canon_out =Ok("\\\\?\\C:\\...\\ngoai.txt")
  ```
- Vì vậy `quyen_chuan::chuan` rơi vào nhánh "canonicalize thư mục cha rồi ghép tên" ⇒ trả **đường
  dẫn của link** (nằm trong root) ⇒ bước kiểm `starts_with(root)` cho qua. Đích thật không được kiểm.
- Sửa `crates/vis-cli/src/quyen_chuan.rs`:
  - `chuan` → `chuan_sau`: khi `canonicalize` lỗi, nếu `symlink_metadata` nói là symlink thì
    `read_link`, ghép đích tương đối với thư mục cha, đệ quy có giới hạn `MAX_LINK = 40`.
    Với symlink có đích tồn tại trên Linux, `canonicalize` thành công nên **không đổi hành vi**;
    nhánh mới chỉ chạy khi lỗi (gồm cả symlink dangling — trước đây rơi vào nhánh cha+tên).
  - `chuan_de_quy`: dùng `chuan(hien)` cho **tổ tiên gần nhất đã tồn tại** thay vì
    `std::fs::canonicalize` trần ⇒ `mkdir_all` qua symlink đích tương đối cũng bị kiểm đúng.
  - Thêm 3 unit test: symlink tương đối ra ngoài, symlink dangling ra ngoài, `mkdir_all` qua symlink.
- Kết quả: các ca `quyen_ngoai_root_bi_chan` của `vd270_fs_read_link` và `vd270_fs_canonicalize`
  đều **exit 2 + `VS0604`** (interp và VM); không còn đọc đích ngoài root.

### 2.3. 35 ca portability/test-only

| Nhóm | Tệp | Sửa |
|---|---|---|
| Path host semantics | `vd270_path_is_absolute`, `vd270_path_is_relative` | thêm `ky_vong(posix, windows)`; nhánh Windows theo `Path::is_absolute` (nền) |
| Join host semantics | `vd270_path_join_all` | 7 kỳ vọng Windows (`\`, `//`, `///`…) |
| With-stem | `vd270_with_stem` | Windows coi `a\b` là `a` + `b` ⇒ `a\c` |
| Rust probe | `vd270_path_strip_prefix` | `khop_rust`/`probe_rust_*` chỉ chạy `#[cfg(not(windows))]` (POSIX probe; sản phẩm vẫn POSIX) |
| FS kỳ vọng | `vd270_fs_current_dir`, `vd270_fs_read_link`, `vd270_fs_is_symlink` | không verbatim `\\?\`; đích native `\`; `NotFound` ⇒ `Ok(false)` trên Windows |
| Std/cheat | `stdlib.rs` + `test/std/*.out`, `cheatsheet.rs` + `09-stdlib.vis`, `st240_std_cli` | token `{SEP}`, `{IS_ABS_ROOT}`; root `\` |
| Unit test | `pathisabs.rs`, `pathrelbool.rs` | nhánh `cfg(windows)` cho `/`; giữ hợp đồng `cfg!(windows)` |
| Fixture escape | `vd270_batch_trigger`, `vd270_db_batch`, `vd270_db_explain`, `vd270_sqlite_database_list` | escape `\` → `\\` khi nhúng đường dẫn Windows vào chuỗi ViScript |
| OS error text | `vd270_fmt_list`, `vd270_fmt_stat` | không so nguyên văn; đòi path + `os error 2` + newline |
| Target thiếu | `targets.rs` | dò sysroot; thiếu std ⇒ in **BLOCKED-ENV** và bỏ qua, không đỏ |

Không nới/không xoá test: các kỳ vọng POSIX vẫn giữ cho Unix; ca Windows có nhánh riêng.

### 2.4. Cổng `readme-examples` — lỗi Windows ẩn sau clippy

- Sau khi clippy xanh, `bin/vis-dev check` mới chạy tới `readme-examples` và đỏ:
  `OSError: [WinError 193] %1 is not a valid Win32 application`.
- Nguyên nhân: `test/readme-examples.py` tìm `target/debug/vis` (không có `.exe`), rơi xuống shim
  bash `REPO/vis`; `CreateProcess` không chạy được script bash.
- Sửa: chọn `vis.exe` khi `os.name == "nt"`; trên Unix giữ nguyên `vis`/shim. Kết quả: **20/20 khối
  README xanh** trên `psvn`.

## 3. Lệnh và log sau sửa (mốc `ed30ebb`)

| Lệnh | Kết quả | Exit | Log cục bộ |
|---|---|---|---|
| `cargo test --workspace --no-fail-fast` | 329/329 block xanh; **3258 passed / 0 failed / 3 ignored** | 0 | `/tmp/opencode/psvn-fix-cargo-final.log` |
| `bin/vis-dev check` | **PASS** toàn bộ 13 bước | 0 | `/tmp/opencode/psvn-fix-visdev-check2.log` |
| `bin/vis-host-suite` | **54/54 PASS**; `host_posix_real` bỏ qua (Windows) | 0 | `/tmp/opencode/psvn-fix-host-suite.log` |
| `cargo test -p vis-cli --test targets -- --nocapture` | `BLOCKED-ENV` + `1 passed` (bỏ qua ô wasm) | 0 | `/tmp/opencode/psvn-fix-targets-nocapture.log` |
| `bin/vis-targets` | MSVC 8/8 PASS mỗi cấu hình; wasm/Linux FAIL vì thiếu std | 1 | `/tmp/opencode/psvn-fix-targets-matrix.log` |
| `git status --porcelain` cuối | 0 mục; HEAD `ed30ebb` | 0 | `/tmp/opencode/psvn-fix-final-state.log` |

Log máy đích: `vis-remote-20261121T031531Z.log` (cargo @ `a7d86e7`),
`…T032209Z.log` (check lần 1 — đỏ readme), `…T032410Z.log` (check lần 2 — PASS),
`…T032522Z.log` (host), `…T032533Z.log` (cargo @ `ed30ebb`), `…T032929Z.log` (targets test),
`…T032938Z.log` (vis-targets), `…T033004Z.log` (trạng thái cuối).

sha256 log cục bộ:

```text
412fa303ec55e30046aade763bd0b54c4ce2d2512e995ba82fc67f4a9e056283  psvn-fix-cargo-final.log
aa462cb59ded682ec024b56b90d29f04bbccc594f5d5175136c4de3763650f50  psvn-fix-visdev-check2.log
55efbaebbad7e0bca698387d1c2fad267a3a5b9c611121de23e77b22f3c0a39a  psvn-fix-host-suite.log
82c34ced8f704d271f6aee5c80eadef520afa78514384bcea2d184fe4a30cdc3  psvn-fix-targets-nocapture.log
bce5d161228fb00b1755e2fa7781352499ddce1a906c98fb87ae7a712226b9df  psvn-fix-targets-matrix.log
```

## 4. Kiểm chứng không hồi quy Linux

Trước khi chuyển sang `psvn`, trên worktree Linux:

- `cargo clippy --workspace --all-targets -- -D warnings` (cargo 1.100.0): **PASS**.
- `cargo test --workspace --no-fail-fast` (cargo 1.100.0): **329 block / 0 failed**, exit 0
  (`/tmp/opencode/w4fix-local-cargo.log`).
- Các test đích: `quyen_chuan` unit (184 test bin `vis`), path/fs/fmt/db/stdlib/cheatsheet/st240
  đều xanh — bao gồm 2 ca cap trên Linux (hành vi cũ giữ nguyên).

## 5. BLOCKED-ENV còn lại

- `bin/vis-targets` vẫn đỏ vì `psvn` **không cài** `wasm32-unknown-unknown` và
  `x86_64-unknown-linux-gnu` (chỉ có `x86_64-pc-windows-msvc`). Không cài theo lệnh.
- Test `targets_wasm_vis_types` nay tự dò sysroot và in:
  ```text
  BLOCKED-ENV: thiếu std target `wasm32-unknown-unknown` trong sysroot của C:/Users/ps/.cargo/bin/rustc
  — bỏ qua ô check này, KHÔNG phải bằng chứng xanh (cài target rồi chạy lại để đo thật)
  ```
  rồi `1 passed` — không giả xanh, nhưng cũng không chặn cổng vì lý do môi trường.
- Ghi nhận thêm: ở bảng `sqlite`, ô `vis-cli × x86_64-pc-windows-msvc` từng `EXPECTED_RED` (nợ #156)
  nay **PASS** — script đã nhắc rà soát nợ; không phải lỗi của đợt này.

## 6. Kết luận và khuyến nghị

**Khuyến nghị promote `v0.1.0`** từ `ed30ebb` (hoặc commit kế thừa), vì:

1. Windows `cargo test --workspace` **0 đỏ** — 3258 test pass, 329/329 block xanh.
2. `bin/vis-dev check` **PASS** trên Windows thật (gồm clippy `-D warnings`, build, doc/readme examples).
3. `bin/vis-host-suite` **54/54**.
4. Hai lỗi cap-confinement đã sửa tận gốc, có unit test và integration test cho cả interp/VM.
5. Linux không hồi quy (clippy + full workspace xanh).

**Caveat phải ghi khi cắt tag**: Windows Tier 2 xác nhận trên **host MSVC**; ma trận cross-target
`vis-targets` **chưa đo** trên `psvn` (BLOCKED-ENV). Nếu A0 muốn ma trận là điều kiện promote, cần
cài `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu` trên `psvn` (cần phê duyệt) rồi chạy lại;
không nên coi lần skip BLOCKED-ENV là bằng chứng xanh.

## 7. Tái lập

```bash
# Lấy commit sửa (bundle cục bộ, không push)
git bundle create /tmp/opencode/w4fix.bundle a7d86e7^..ed30ebb
scp /tmp/opencode/w4fix.bundle psvn:C:/Users/ps/vislogs/w4fix.bundle
bin/vis-remote psvn -- bash -lc \
  'cd /c/Users/ps/viscript-moi && git fetch /c/Users/ps/vislogs/w4fix.bundle refs/heads/vs3/w4-windows:refs/heads/w4fix-test && git switch --detach w4fix-test'

bin/vis-remote psvn --log /tmp/opencode/psvn-fix-cargo-final.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && cargo test --workspace --no-fail-fast'
bin/vis-remote psvn --log /tmp/opencode/psvn-fix-visdev-check2.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-dev check'
bin/vis-remote psvn --log /tmp/opencode/psvn-fix-host-suite.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-host-suite'
```
