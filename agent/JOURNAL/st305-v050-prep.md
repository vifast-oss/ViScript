<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-305 — chuẩn bị `v0.5.0`: version, cache, CHANGELOG, trạng thái

> **Người làm**: subagent context sạch, 2026-11-23 · **time-box 1,5 h**.
> **Nguồn**: `main` **`f2d3e73`** (`[ST-305] docs line shift: cap nhat toa do DOC_BLOCKS`).
> **Nhánh**: `plan/v050-prep` từ `f2d3e73` — **local only**; không ssh `mps`/`psvn`;
> không merge/push/tag. `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`,
> `TMPDIR=/home/ps/tmp`.

## 1. Thay đổi

| Tệp | Nội dung |
|---|---|
| `Cargo.toml` | `[workspace.package] version` **0.4.5 → 0.5.0** (một nguồn; mọi crate đã `version.workspace = true`). |
| `Cargo.lock` | 16 crate thành viên đồng bộ **0.5.0** (cargo tự cập nhật). |
| `vis-host-wasm/Cargo.lock` | 7 crate `path` đồng bộ **0.5.0** (`cargo update --workspace` trong workspace riêng). |
| `crates/vis-cli/tests/cli_it/st289_version.rs` | `BAN` **0.5.0** + chú thích đầu tệp. |
| `crates/vis-loader/src/cache.rs` | `PHIEN_BAN` thêm **`+v050-release-2026-11-23`**; ghim khoá mới **`fc20a811…`**; `assert_ne` khoá 0.4.5 **`d92f31c2…`** (và khoá ST-302 `88613f2f…`); `starts_with("0.5.0+")`; assert nhãn `v050-release`. |
| `CHANGELOG.md` | mục **`## v0.5.0`** — ST-302/302b, ST-304/304b, ST-305, ST-306/307, ST-308, `ST-300m2`; macOS Tier 1 **chờ `ST-300m3`**; ghi rõ **chưa tag**. |
| `agent/ACTIVE.md` | `main` `f2d3e73`, gate Linux **3.747/4**, GĐ 3 đã tích hợp (trừ m3), `v0.5.0` chưa tag. |
| `agent/ROADMAP.md` | khối trạng thái hiện hành + khối `v0.5.0` prep: version/cache/khoá, 16 crate, macOS Tier 1 chờ m3, chưa tag/merge. |

## 2. Khoá cache

- `PHIEN_BAN` = `0.5.0+…+v045-release-2026-11-23+st302-derives-json-async-impl-2026-11-23+v050-release-2026-11-23`.
- Khoá mới (test `khoa_cache_ghim`): **`fc20a81123cdb432a976e7d3d1fa15b7f4a2dd3b2276241bd0888e55bda53117`**.
- Khoá cũ 0.4.5 (`d92f31c2…`) và khoá ST-302 (`88613f2f…`) nằm trong `assert_ne` ⇒ cache cũ tự vô hiệu.

## 3. Kiểm chứng

| Lệnh | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3.747 passed / 0 failed / 4 skipped** (nextest 68,97 s; 110 binary); fmt/clippy/R1..R7/self-test/explain/report-check/build/doc-examples/readme 20/20 PASS |
| `report-check` (trong gate) | **PASS — 35 báo cáo, 333 hàng QUEUE phủ đủ** |
| `vis --version` / `vis version --json` | `vis 0.5.0` / `{"name":"vis","version":"0.5.0"}` |
| `cargo check --release --workspace` | **PASS** — `Finished release profile [optimized] target(s) in 1m 09s` (exit 0) |

Log ngoài repo: `/home/ps/tmp/v050-gate-final.log` sha256 `57d03fcfb50ff6a019a053ff2090bcf45e3f6678bfad3bcdefad84d171e3faea`
(lần chạy đầu `v050-gate.log` sha256 `08e69da4…` cũng 3.747/4; lần cuối chạy lại trên **cây chốt** sau khi sửa nhãn `assert_ne`).

## 4. Blocker / treo

- **Không blocker mã**. `v0.5.0` **chưa tag**, nhánh **chưa merge/push**.
- **macOS arm64 Tier 1**: `ST-300m3` (3× `bin/vis-gate-macos --full` + soak 4 h +
  `vis-wasm-sem` ≥ 388/390) **đang chạy / chờ kết quả** trên `mps` — không tuyên bố Tier 1,
  không điền số thay.
- `R36-viscript.{md,json}` vẫn là bản **đang dựng** tại `c3b7290`; chưa chốt trong lượt này.
