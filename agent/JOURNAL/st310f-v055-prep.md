<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310f — chuẩn bị `v0.5.5`: version, cache, CHANGELOG, trạng thái

> **Phạm vi**: nhánh **`plan/v055-prep`** tạo từ **`b591e4d`** (`plan/v055-integration2`: Phase B
> `ST-310a..f` + hop `main` `8906c70`). **Không** merge/push/tag, không ssh `mps`/`psvn`.
> Worktree riêng `/home/ps/tmp/v055-prep` (không checkout `/home/ps/ViScript`).
> Môi trường: Rust **1.100.0**, `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/home/ps/tmp`.
> Thời điểm: 2026-11-23 (đêm).

## 1. Thay đổi

| Tệp | Nội dung |
|---|---|
| `Cargo.toml` | `[workspace.package] version` **0.5.0 → 0.5.5** (một nguồn; mọi crate đã `version.workspace = true`). |
| `Cargo.lock` | **18 crate** thành viên đồng bộ **0.5.5** (`cargo metadata`, không đổi dependency ngoài). |
| `vis-host-wasm/Cargo.lock` | **9 crate** `path` đồng bộ **0.5.5** (`cargo metadata` trong workspace riêng). |
| `crates/vis-cli/tests/cli_it/st289_version.rs` | `BAN = "0.5.5"`; đổi tên test `cli_in_dung_ban_0_5_5`; cập nhật doc-comment. |
| `crates/vis-loader/src/cache.rs` | `PHIEN_BAN` thêm **`+v055-release-2026-11-23`**; ghim khoá mới **`903995a7…`**; `assert_ne` khoá 0.5.0 **`fc20a811…`**; `starts_with("0.5.5+")`; assert nhãn `v055-release`. |
| `CHANGELOG.md` | Mục `## v0.5.5`: `ST-310a..f`, `#194` mở, `#195` đề xuất, version/cache, macOS Tier 1 chờ tag `v0.5.0`. |
| `agent/ACTIVE.md` | Khối trạng thái hiện hành: `v0.5.5` chưa tag, Phase B integration `b591e4d`, chưa merge `main`. |
| `agent/ROADMAP.md` | Khối trạng thái hiện hành + khối `v0.5.5` prep; `v0.5.0` vẫn chưa tag (macOS Tier 1 chờ `ST-300m3`). |

`PHIEN_BAN` = `0.5.5+…+st302-derives-json-async-impl-2026-11-23+v050-release-2026-11-23+v055-release-2026-11-23`.

## 2. Khoá cache (VD-129)

| Mốc | Khoá (`khoa_cache_ghim`) |
|---|---|
| **0.5.5 (mới)** | **`903995a78ed8585f360005df18a81a0be3a970c5fdaf2a3f7401cc90ec2100d9`** |
| 0.5.0 (cũ, `assert_ne`) | `fc20a81123cdb432a976e7d3d1fa15b7f4a2dd3b2276241bd0888e55bda53117` |

Cache 0.5.0 tự vô hiệu; `VERSION` VBC1/ABI/định dạng **không đổi**. Test
`vis-loader::cache::tests::khoa_cache_ghim` PASS.

## 3. Kiểm chứng

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3773 passed / 0 failed / 4 skipped** (nextest 68,7 s) |
| fmt / clippy / luật R1..R9 / toolchain / release / self-test | **PASS** |
| `report-check` (`check_report.py --all`) | **PASS — 35 báo cáo, 339 hàng QUEUE** |
| `bin/vis-targets` | **PASS — phủ 100% workspace (18 crate)**; core **38 PASS / 0 FAIL**; sqlite **36 PASS / 2 EXPECTED_RED (#156) / 0 FAIL** |
| doc-examples / readme-examples | **PASS** (31 khối: 26 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3) |
| `vis --version` / `vis version --json` | `vis 0.5.5` / `{"name":"vis","version":"0.5.5"}` |

Log: `/home/ps/tmp/v055-prep-logs/gate-fast.log` (rc 0, 0 dòng `FAIL`).

**Ghi nhận flake**: lần chạy gate đầu tiên đỏ 1 ca
`vis-cli::cli_it repl_term::format_la_la_loi_dung_exit_2` — `ghi stdin: BrokenPipe`: test ghi
`:quit\n` vào `vis repl --format xml`, nhưng tiến trình đã thoát (exit 2) trước khi `write_all`
kịp. Chạy riêng **PASS**; chạy lại toàn gate **PASS 3773/4**. Đây là race sẵn có của test (không
do bump 0.5.5, không sửa mã trong đợt này) — ghi nhận để xử lý ở đợt sau.

## 4. Còn lại / blocker

- Nhánh **local only**: **chưa merge `main`**, chưa push, chưa tag `v0.5.5`.
- **`v0.5.0` vẫn chưa tag** — macOS arm64 Tier 1 chờ `ST-300m3` (3× `vis-gate-macos --full` +
  soak 4 h + `vis-wasm-sem` ≥ 388/390); không tuyên bố trước bằng chứng.
- **`#194` mở** (race metadata-only FS); **`#195` đề xuất** (VM back-edge O(n²)/fuel, chưa cấp số).
- Không có blocker mã.
