<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — xác nhận tag `v0.1.0` + Windows CI

> **Người thực hiện**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Tag xác nhận**: `v0.1.0` = `50675ddbc861df40b6f4fade3518fc040a9a55c5`
> (`[VS3] release: v0.1.0 — Linux 3448/3, Windows 3404/0, mac ngoai pham vi`).
> **CI**: thêm `.github/workflows/windows.yml` (repo trước đó **chưa có** workflow nào).
> Không push/merge/tag; không đụng `net.rs`/`pkg.rs`/`frame/**`/`value.rs`; không cài dependency.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Tag `v0.1.0` trên `psvn` | checkout đúng `50675dd`, tag exact, cây sạch |
| `cargo test --workspace --no-fail-fast` | **344/344 block; 3404 passed / 0 failed / 3 ignored**; exit **0** |
| `bin/vis-dev check` | **PASS** (toàn bộ bước, gồm clippy `-D warnings`, build, doc/readme examples) |
| `bin/vis-host-suite` | **54/54 PASS** (`host_posix_real` bỏ qua — Windows dùng `test-child`) |
| Code so với `777af63` | **không đổi** — chỉ docs/journal (0 tệp trong `crates/`, `Cargo.toml`, `Cargo.lock`) |
| Windows CI | `.github/workflows/windows.yml`; YAML parse OK; **chưa chạy trên GitHub Actions** (chưa có runner ở đây) |
| Cross-target trên psvn | **BLOCKED-ENV đã gỡ**: core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**, exit **0** |
| Đĩa trước khi làm | `/` còn **21G** (> ngưỡng 5G) |

## 1. Xác nhận tag `v0.1.0`

- `git rev-parse 'v0.1.0^{commit}'` = `50675dd…`; checkout trên `psvn`:
  `git describe --tags --exact-match HEAD` = `v0.1.0`, `git status --porcelain` = 0.
- `cargo test --workspace --no-fail-fast`: **3404 passed / 0 failed / 3 ignored**, 344 block xanh.
- `bin/vis-dev check`: PASS toàn bộ; readme-examples 20/20; doc-examples 24 khối (19 xanh + 5 ngoại lệ đỏ đúng).
- `bin/vis-host-suite`: **54/54**.
- **Tương đương code** với candidate `777af63` (đã verify 3404/0): `git diff --stat 777af63..50675dd`
  chỉ có `CHANGELOG.md`, `agent/ROADMAP.md`, `agent/STATE.md`, `agent/VS3-TIEN-DO.md`,
  `agent/JOURNAL/vs3-w4-release-18e57c3.md` (thay báo cáo cũ); **không** có tệp mã nào:
  ```text
  git diff --stat 777af63..50675dd -- crates/ Cargo.toml Cargo.lock   # rỗng
  ```
  Vì vậy tag `v0.1.0` mang đúng cây code đã verify 3404/0.

## 2. Windows CI

### 2.1. Hiện trạng

- Repo **chưa có** `.github/workflows/` (không có CI Linux/Windows trước đợt này).
- Đã thêm `.github/workflows/windows.yml`; **không** sửa/tạo workflow Linux (không có để đổi).

### 2.2. Nội dung workflow

- Trigger: `push` nhánh `main`/`vs3/**`, `pull_request` cùng nhánh, `workflow_dispatch`.
- `runs-on: windows-latest`, `timeout-minutes: 90`, `permissions: contents: read`, concurrency theo ref.
- Bước:
  1. `actions/checkout@v4`.
  2. **Normalize LF + exec**: `core.autocrlf=false`, `git reset --hard HEAD`, chuẩn hoá `bin/*`
     và `test/**/*.sh`, `chmod +x bin/*` — tránh shebang CRLF và bit exec trên Windows runner.
  3. `dtolnay/rust-toolchain@stable` với `toolchain: 1.100.0`, components `clippy, rustfmt`.
  4. `actions/cache@v4` cho `~/.cargo/registry`, `~/.cargo/git`, `target`; key theo `Cargo.lock`.
  5. `actions/setup-python@v5` (3.12) cho doc/readme examples.
  6. In `cargo --version` / `rustc --version`.
  7. `cargo test --workspace --no-fail-fast` (`shell: bash`, **không** nextest).
  8. `bin/vis-dev check` (Git Bash), kèm shim `python3` tạm trong PATH của riêng bước này vì
     runner Windows có `python` nhưng không chắc có `python3`.
  9. `bin/vis-host-suite` (Windows host conformance).
- Không dùng `--allow-dirty`, không cài Build Tools/dependency khác; workflow **không** cài target
  wasm/Linux (giữ CI nhẹ), không đổi gate Linux.

### 2.3. Giới hạn / trạng thái kiểm chứng

- **Chưa chạy trên GitHub Actions** trong đợt này (không có runner GitHub ở môi trường W4).
  Đã kiểm:
  - YAML parse hợp lệ (PyYAML; 9 step);
  - các lệnh `cargo test` / `vis-dev check` / `vis-host-suite` chạy đúng trên `psvn` (cùng Git Bash,
    cùng toolchain 1.100.0) — xem log;
  - bước normalize LF/exec chạy thử cục bộ, không đổi nội dung mã.
- **Cross-target không có trong CI**: runner chỉ có `x86_64-pc-windows-msvc`; `targets_wasm_vis_types`
  sẽ tự báo **BLOCKED-ENV** và skip (không phải bằng chứng xanh cho wasm/Linux).
- Lần chạy đầu trên GitHub vẫn cần theo dõi: action versions (`checkout@v4`, `cache@v4`,
  `setup-python@v5`, `dtolnay/rust-toolchain@stable`), quyền cache, và `python3` shim.

### 2.4. Cross-target trên `psvn` — BLOCKED-ENV đã gỡ

- `psvn`: rustup `1.29.2`; `rustup target add wasm32-unknown-unknown x86_64-unknown-linux-gnu`
  thành công (2 component, không cài Build Tools/dependency khác). Log: `/tmp/opencode/psvn-target-add.log`.
- `psvn` @ main `9c789b7` (cây sạch); `bin/vis-targets` **exit 0**:

| Cấu hình | PASS | EXPECTED_RED | FAIL |
|---|---:|---:|---:|
| `core (vis-cli --no-default-features)` | **18** | 0 | **0** |
| `sqlite (vis-cli default)` | **17** | 1 | **0** |

- Ô `EXPECTED_RED` duy nhất: `vis-cli × wasm32-unknown-unknown` — `libsqlite3-sys`/`ring` cần C
  toolchain cho wasm (lý do #156/VS3-4a); **không** phải lỗi mới, script ghi rõ lý do.
- Ghi nhận: `vis-cli × x86_64-pc-windows-msvc` (sqlite) nay **PASS** — từng `EXPECTED_RED` nợ #156;
  script nhắc rà soát nợ. Các ô còn lại (8 crate × MSVC, wasm, `vis-host-wasm`, `vis-host-linux`
  × Linux) đều PASS.
- Vậy **`psvn` không còn BLOCKED-ENV cross-target**. CI workflow vẫn không cài target (giữ nhẹ),
  nên trên GitHub `targets_wasm_vis_types` vẫn skip; nếu muốn CI đo cross-target, thêm
  `rustup target add …` + `bin/vis-targets` (chưa làm trong đợt này).

## 3. Merge main trên branch

- Trước khi làm CI: `git merge main` (`77b71fb`) vào `vs3/w4-windows` — merge `4db96b2`, **không conflict**;
  thay đổi chỉ ở `CHANGELOG.md`, `agent/DECISIONS.md`, `agent/ROADMAP.md`, `agent/STATE.md`,
  `agent/VS3-TIEN-DO.md` (docs), **không** đụng mã.
- Commit workflow: **`67e23f2`** `[VS3][W4] ci: workflow windows-latest ...`.
- Sau đó branch **fast-forward** lên main `9c789b7` (chỉ `agent/VS3-TIEN-DO.md`, docs);
  không merge conflict, không đổi mã.

## 4. Log / bằng chứng

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| `cargo test` @ `v0.1.0` (`50675dd`) | `/tmp/opencode/psvn-v010-cargo.log` | `28ca2d56…dc20c` | **0** |
| `vis-dev check` + host suite @ tag | `/tmp/opencode/psvn-v010-gate.log` | `23608534…f33d` | 0 |
| `rustup target add` | `/tmp/opencode/psvn-target-add.log` | `fd75fde5…3fed` | 0 |
| `bin/vis-targets` @ `9c789b7` | `/tmp/opencode/psvn-targets-matrix.log` | `0db74913…224` | **0** |
| trạng thái cuối @ tag / sau targets | `/tmp/opencode/psvn-v010-final-state.log`, `/tmp/opencode/psvn-targets-final-state.log` | — | 0 |

Log máy đích: `vis-remote-20261121T064026Z.log` (cargo tag),
`…T064618Z.log` (gate + host), `…T064733Z.log` (trạng thái cuối tag),
`…T065449Z.log` (target add), `…T065523Z.log` (vis-targets), `…T065610Z.log` (trạng thái cuối).

## 5. Kết luận

- Tag `v0.1.0` (`50675dd`) **đã xác nhận xanh trên Windows**: 3404/0, gate PASS, host 54/54;
  code trùng khít candidate `777af63` (chỉ khác docs).
- **BLOCKED-ENV cross-target trên `psvn` đã gỡ**: `vis-targets` exit 0 (core 18/18;
  sqlite 17 PASS + 1 EXPECTED_RED do C toolchain wasm). CI workflow vẫn không cài target,
  nên trên GitHub `targets_wasm_vis_types` vẫn skip — đã ghi rõ.
- Windows CI đã có workflow tối thiểu, bám đúng đường chuẩn Windows; **chưa chạy trên GitHub**
  nên cần theo dõi lần đầu. Không có blocker sản phẩm.
- Không tag, không push, không merge ngược.

## 6. Checklist verify Windows cho `v0.2.0` (khi W1/W2/W5 merge)

- [ ] `git merge main` vào `vs3/w4-windows`; xử lý conflict; **không** sửa `net.rs`/`pkg.rs`/
      `frame/**`/`value.rs` trừ khi được phân công.
- [ ] `df` còn > 5G trên `ps` và `psvn`; dừng nếu dưới ngưỡng.
- [ ] `psvn`: fetch/checkout **đúng commit merge** (hoặc tag ứng viên), `git status --porcelain` = 0.
- [ ] `cargo test --workspace --no-fail-fast` → **0 failed**; ghi pass/failed/ignored + block.
- [ ] `bin/vis-dev check` → **PASS**.
- [ ] `bin/vis-host-suite` → **54/54**.
- [ ] `bin/vis-targets` → core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED**; exit **0**
      (psvn đã cài `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu`).
- [ ] Xác nhận `targets_wasm_vis_types` **chạy thật** (không skip) vì psvn đã có target.
- [ ] Kiểm mảng mới của v0.2.0 (W1/W2/W5): TLS/keepalive/sig/chan0/LSP incremental + test mới;
      đỏ thì phân loại product/portability/test-only/BLOCKED-ENV, sửa test Windows nếu trong phạm vi.
- [ ] Log mới: `/tmp/opencode/psvn-<mốc>-{cargo,gate,host,targets}.log` + sha256; log đích
      `/c/Users/ps/vislogs/vis-remote-*.log`.
- [ ] Báo cáo `agent/JOURNAL/vs3-w4-...`; commit `[VS3][W4] ...`; **không tag/push/merge**.
- [ ] Chỉ A0 tag sau khi đọc báo cáo.
