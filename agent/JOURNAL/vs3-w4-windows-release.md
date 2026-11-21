<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — Windows release build/check @ main `14310f4`

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc**: main `14310f4` (`merge VS3 W3m: 3x soak 10/10 d1f8f70`; so với tag v0.3.2 chỉ
> test `vs3_http2`/`vs3_worker` đổi + docs — code sản phẩm không đổi).
> **Kết luận**: release binary build/smoke/host/targets **pass**; 2 quan sát CLI (không phải lỗi
> build) ghi cho A0. Không sửa sản phẩm; không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo build --release -p vis-cli` (default: sqlite,tls,pkg-sig,http2) | **PASS**, 62 s, 61 crate compile |
| `target/release/vis.exe` | **10.541.056 B** (~10,05 MiB), sha256 `72013a1c…fd353` |
| `target/debug/vis.exe` (so sánh) | 16.899.584 B (~16,12 MiB), release nhỏ hơn **~37,6%** |
| `vis version` / `--json` | **PASS** — `vis 0.1.0` / `{"name":"vis","version":"0.1.0"}` |
| `vis doctor` / `--json` | **PASS** — 5 dòng / JSON; xem quan sát §3 |
| Smoke `vis run`, `vis check` | **PASS** — `release-smoke-ok`; TAP `ok 1` |
| `bin/vis-host-suite` | **54/54 PASS** (13 s cùng targets) |
| `bin/vis-targets` | core **18/18**; sqlite **17 + 1 EXPECTED_RED**; exit 0 |
| `psvn` cuối | `14310f4`, cây sạch |

## 1. Release build

- Command: `cargo build --release -p vis-cli` (default features, gồm **http2**).
- Thời gian: **62 s** (11:16:30→11:17:32Z); build log có **61 dòng `Compiling`** (không phải cache rỗng).
- Artifact:
  - `target/release/vis.exe`: **10.541.056 B**, sha256 `72013a1c3cd43814b823e8f901abfed8896363ff38b430711c26c72a620fd353`.
  - `target/debug/vis.exe`: 16.899.584 B, sha256 `2e8d8fe1…b9a59` ⇒ release nhỏ hơn ~37,6%.
  - `target/release/test-child.exe`: 226.816 B — `cargo build -p` build cả bin test-support;
    `vis-release`/`test/release` đã có phép kiểm artifact phát hành **chỉ `vis`** (không đổi).

## 2. Chạy release binary

- `target/release/vis.exe version` → `vis 0.1.0` (rc 0).
- `version --json` → `{"name":"vis","version":"0.1.0"}` (rc 0).
- `doctor` → 5 dòng: `vis 0.1.0`, `os windows`, `arch x86_64`, `sqlite on`, `features sqlite` (rc 0).
- `doctor --json` → `{"version":"0.1.0","os":"windows","arch":"x86_64","sqlite":true,"features":["sqlite"]}`.
- Smoke:
  - tạo `C:\Users\ps\vislogs\smoke\hello.vis` (`fn main(): print("release-smoke-ok")`);
  - `vis run` → `release-smoke-ok`, rc 0;
  - `vis check` → TAP `1..1; ok 1 - …hello.vis`, rc 0.

## 3. Quan sát cho A0 (không phải lỗi build)

1. **`vis --version` không được hỗ trợ**: exit **2** + usage (CLI dùng `vis version`). Hành vi này
   giống debug; nếu muốn chuẩn CLI Unix, cần thêm `--version` (product change — W4 **không sửa**).
2. **`vis doctor` chỉ liệt kê feature `sqlite`**, dù default có `tls`, `pkg-sig`, `http2`.
   `FEATURES` trong `cli/doctor.rs` là danh sách cố định chỉ có `sqlite`; nên cập nhật khi thêm
   feature default (product/docs — W4 **không sửa**). Không ảnh hưởng việc binary build/run.

## 4. Host / targets

- `bin/vis-host-suite`: **54/54 PASS** (`host_posix_real` bỏ qua — Windows `test-child`).
- `bin/vis-targets`: core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** (vis-cli × wasm do C
  toolchain sqlite/ring, nợ #156 đã biết); 0 FAIL; exit 0.
- Cùng nhau ~13 s (11:21:51→11:22:04Z).

## 5. Log

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| release build | `/tmp/opencode/psvn-rel-build.log` | `47b690c9…669e` | 0 |
| version/doctor/smoke + host + targets | `/tmp/opencode/psvn-rel-check.log` | `03e61b3a…a8c3` | 0 |
| chi tiết artifact/doctor | `/tmp/opencode/psvn-rel-detail.log` (vis-remote) | — | 0 |

Log trên máy đích: `/c/Users/ps/vislogs/w4rel-{build,version,doctor,smoke-run,smoke-check}.log`;
vis-remote: `vis-remote-20261121T111627Z.log` (build), `…T112149Z.log` (check), `…T112355Z.log` (detail).

## 6. Blocker

- **Không có blocker build/release**. `ps` còn **19G** (>5G); `psvn` còn ~184G lúc build, cây sạch.
- 2 quan sát CLI ở §3 (không sửa trong phạm vi W4); không tag/push/merge; không GitHub Actions.
