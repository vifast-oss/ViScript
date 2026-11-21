<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — CLI hardening: `--version`/`-V` + doctor đủ feature

> **Người làm**: subagent VS3-W4 (`psvn`), 2026-11-21. A0 duyệt sửa nhỏ sau release Windows.
> **Commit code/test**: `c5d2be6` — `[VS3][W4] CLI: --version/-V cap cao nhat + doctor du feature
> (sqlite,tls,pkg-sig,http2)`.
> Không tag/push/merge; không GitHub Actions; không đụng `net.rs`/`pkg.rs`/`frame/**`/`async_net.rs`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `vis --version` / `vis -V` | `vis 0.1.0`, exit 0, stderr rỗng |
| `vis run --version` | vẫn exit 2 + `vis run: không hỗ trợ cờ --version` (không bị nuốt) |
| `vis doctor` | `features sqlite,tls,pkg-sig,http2` (default đủ) |
| `vis doctor --json` | schema giữ nguyên, `features:["sqlite","tls","pkg-sig","http2"]` |
| Test | `vd270_version` **5/5**, `vd270_doctor` **7/7** (local + psvn) |
| Clippy `-D warnings` | PASS (local + psvn) |
| Release rebuild psvn | PASS, 15 s; `vis.exe` **10.550.784 B** |

## 1. Thay đổi

### 1.1. `vis --version` / `-V`
- `crates/vis-cli/src/main.rs`: thêm nhánh **đầu** `match args.first()`:
  ```rust
  Some("--version") | Some("-V") => {
      println!("vis {}", env!("CARGO_PKG_VERSION"));
      ExitCode::SUCCESS
  }
  ```
- Chỉ áp dụng khi là **đối số đầu tiên**; `vis run --version`, `vis version ...` và mọi lệnh con
  giữ nguyên. Usage gốc thêm `vis --version | vis -V`.
- Test mới trong `tests/vd270_version.rs`:
  - `top_level_version_va_v`: cả `--version`/`-V` → stdout `vis <CARGO_PKG_VERSION>\n`, rc 0, stderr rỗng.
  - `top_level_version_khong_anh_huong_lenh_con`: `vis run --version` vẫn rc 2, stdout rỗng,
    stderr `vis run`.

### 1.2. `doctor` đủ feature
- `crates/vis-cli/src/cli/doctor.rs`: bỏ `FEATURES` chỉ `sqlite`; thêm `fn features()` trả theo cfg
  **thứ tự cố định**: `sqlite`, `tls`, `pkg-sig`, `http2` (feature nào bật mới có).
- `--json` giữ nguyên schema/khoá/thứ tự; chỉ giá trị `features` đầy đủ hơn.
- `tests/vd270_doctor.rs`: helper `features()` cập nhật cùng nguồn; assertion usage đổi
  `dùng: vis explain` → `vis explain` (usage mới có tiền tố `vis --version | vis -V`).
- Không đổi `vis version`/`--json` (vẫn `vis 0.1.0` — `CARGO_PKG_VERSION`).

## 2. Test

### 2.1. Local (Linux)
- `cargo test -p vis-cli --test vd270_version --test vd270_doctor`:
  - `vd270_doctor`: **7 passed**;
  - `vd270_version`: **5 passed** (3 cũ + 2 mới).
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- `./target/debug/vis doctor` → `features sqlite,tls,pkg-sig,http2`;
  `doctor --json` → `"features":["sqlite","tls","pkg-sig","http2"]`.
- `vis run --version` → rc 2, stderr `vis run: không hỗ trợ cờ --version`.

### 2.2. `psvn` (Windows)
- `cargo test -p vis-cli --test vd270_version --test vd270_doctor`:
  - `vd270_doctor`: **7/7**; `vd270_version`: **5/5**.
- `cargo clippy -p vis-cli --all-targets -- -D warnings`: PASS.
- `cargo build --release -p vis-cli`: PASS, **15 s** (incremental); `vis.exe` **10.550.784 B**,
  sha256 `e955706f245eef2500c1522d0564cd0dae14670b6b928bcfabc5ecf614ab14bc` (+9.728 B so bản trước).
- Release:
  - `vis --version` / `-V` → `vis 0.1.0`, rc 0;
  - `vis doctor` → `features sqlite,tls,pkg-sig,http2`; JSON đúng schema;
  - `vis run --version` → rc 2 (không đổi).

## 3. Log

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| test + clippy + release + CLI @ psvn | `/tmp/opencode/psvn-cli-check.log` | `a4de1c79…bb8c` | 0 |
| trạng thái cuối psvn | `/tmp/opencode/psvn-cli-final-state.log` | — | 0 |

Log máy đích: `vis-remote-20261121T112733Z.log` (check), `…T112820Z.log` (trạng thái cuối).

## 4. Blocker

- **Không có blocker**. Local `/` còn **19G** (>5G); `psvn` @ `c5d2be6` cây sạch.
- Không tag/push/merge; không GitHub Actions.
