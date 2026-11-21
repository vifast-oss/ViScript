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
- Không dùng `--allow-dirty`, không cài target wasm/Linux, không đổi gate Linux.

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

## 3. Merge main trên branch

- Trước khi làm CI: `git merge main` (`77b71fb`) vào `vs3/w4-windows` — merge `4db96b2`, **không conflict**;
  thay đổi chỉ ở `CHANGELOG.md`, `agent/DECISIONS.md`, `agent/ROADMAP.md`, `agent/STATE.md`,
  `agent/VS3-TIEN-DO.md` (docs), **không** đụng mã.
- Commit workflow: **`67e23f2`** `[VS3][W4] ci: workflow windows-latest ...`.

## 4. Log / bằng chứng

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| `cargo test` @ `v0.1.0` (`50675dd`) | `/tmp/opencode/psvn-v010-cargo.log` | `28ca2d56…dc20c` | **0** |
| `vis-dev check` + host suite @ tag | `/tmp/opencode/psvn-v010-gate.log` | `23608534…f33d` | 0 |
| trạng thái cuối @ tag | `/tmp/opencode/psvn-v010-final-state.log` | — | 0 |

Log máy đích: `vis-remote-20261121T064026Z.log` (cargo tag),
`…T064618Z.log` (gate + host), `…T064733Z.log` (trạng thái cuối).

## 5. Kết luận

- Tag `v0.1.0` (`50675dd`) **đã xác nhận xanh trên Windows**: 3404/0, gate PASS, host 54/54;
  code trùng khít candidate `777af63` (chỉ khác docs).
- Windows CI đã có workflow tối thiểu, bám đúng đường chuẩn Windows; **chưa chạy trên GitHub**
  nên cần theo dõi lần đầu. Không có blocker sản phẩm; blocker duy nhất là cross-target
  wasm/Linux không đo được trên `psvn`/CI Windows (BLOCKED-ENV đã ghi rõ).
- Không tag, không push, không merge ngược.
