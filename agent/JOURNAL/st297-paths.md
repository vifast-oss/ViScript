<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-297 — path-split Windows/UNC/mixed-slash (3 điểm theo plan)

> **Người làm**: subagent VS3-W2, 2026-11-23. **Nhánh** `plan/w2-st297-paths` (ban đầu từ
> `1b27cb8`, **rebase lên `a1730c5`** sau ST-296c/296d). Không merge/push/tag; v0.4.1 mới merge.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`, target riêng `target/w3b`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| #1 `uri_thanh_duong_voi` | nhận **UNC**, drive `C:`/`C:\`, `%3A`/`%5C`, percent UTF-8, mixed slash; scheme khác `None` |
| #2 `workspace/symbol` + `definition` | helper `duong_thanh_uri` chuẩn `file:///C:/…` / `file://server/share/…`, encode space/UTF-8/`#`/`%`/`?` |
| #3 `phan_giai_lenh` | phân loại path theo `/` + `\` + drive/UNC trên Windows; tên trần vẫn dò `PATH`; Unix `\` là ký tự thường |
| Test mới | LSP **29** (thêm UNC/mixed/round-trip + workspace URI); `quyen_path` **2** unit (Windows trên Linux) |
| Gate Linux | **3660 passed / 0 failed / 4 skipped** — PASS |
| R1..R7 / fmt / clippy | PASS |
| `cargo check --no-default-features --tests` | PASS |
| `bin/vis-targets` core | **18/18** |
| Blocker | **không** |

## 1. Thay đổi

- `crates/vis-lsp/src/lib.rs`:
  - `uri_thanh_duong_voi`: decode percent trước; trên Windows chuẩn hoá `\`→`/` rồi nhận drive
    hoặc UNC (`file://server/share` ⇒ `//server/share`), giữ `file:////…`; non-Windows vẫn từ chối
    authority và chỉ nhận `/…`.
  - thêm `pub(crate) duong_thanh_uri(path, windows)` — encode percent (space/UTF-8/`#`/`%`/`?`),
    giữ `/` và `:` drive; drive ⇒ `file:///C:/…`, UNC ⇒ `file://server/share/…`.
- `crates/vis-lsp/src/workspace.rs`: thay 2 `format!("file://{}", display())` bằng helper; thêm test
  `symbol_uri_chuan_hoa_percent_utf8` (file `a b-é.vis` ⇒ `a%20b-%C3%A9.vis`, không backslash).
- `crates/vis-cli/src/quyen_path.rs`: `la_duong_dan(v, windows)` (cả `/`, `\`, drive/UNC);
  `phan_giai_lenh` dùng `cfg!(windows)`, test `phan_giai_lenh_voi` với `windows=true` trên Linux:
  `C:\…`/`\\server\…` đi nhánh path (không dò PATH), tên trần vẫn dò PATH.

## 2. Test

- `cargo test -p vis-lsp`: **29/29**.
- `cargo test -p vis-cli --bin vis path_impl::tests`: **2/2**.
- Test mới LSP: UNC (`file://server/...`), `file:////...`, `%20`/`%C3%A9`/`%3A`/`%5C`/`%25`,
  mixed `C:/a\b`, non-Windows authority `None`, path→URI/round-trip POSIX/Windows/UNC,
  workspace symbol URI có space/UTF-8.
- Test mới `quyen_path`: phân loại Windows/Unix, `C:\`/`\\server` không dò `PATH`, tên trần vẫn
  dò `PATH`.

## 3. Cổng / targets

- `./vis-dev gate --fast`: **PASS — 3660 passed / 0 failed / 4 skipped**; fmt/clippy/R1..R7 OK
  (R7 self-test 15/15). Base trước task **3656/4**; +4 ca test mới (LSP + quyen_path).
- `cargo check -p vis-cli --no-default-features --tests`: PASS.
- `bin/vis-targets`: core **18 PASS / 0 EXPECTED_RED / 0 FAIL**; sqlite 16 PASS + 2 EXPECTED_RED
  (wasm/msvc C toolchain #156), exit 0 đúng kỳ vọng.
- Log ngoài repo (R7):
  - `/home/ps/vislogs/st297-gate.log` — sha256 `feb6c849cfb66d29b235d900273a6ea4baf68d1cc6a020128d84ff1ada039f1e`.
  - `/home/ps/vislogs/st297-targets.log` — sha256 `7debefd1a86aeff25d13a5407b2e5a537739658c77331fd713543c74d6dbef38`.

## 4. Windows

Theo chỉ đạo: **không chạy psvn song song** khi W4 chưa full gate. Các ca `#[cfg(windows)]`/UNC
thật để W4 chạy targeted `cargo test -p vis-lsp` + `quyen_path` trên psvn; phần logic đã test được
trên Linux qua cờ `windows: bool`.

## 5. Kết luận

- 3 điểm đã sửa đúng phạm vi plan, không đổi API công khai.
- Gate Linux xanh; targets core 18/18. **Không merge** — chờ v0.4.1.
- Blocker: **không**.


## 6. Rebase lên `a1730c5`

- `git rebase a1730c5` trên `plan/w2-st297-paths`: **sạch, 0 xung đột** (vis-lsp/quyen_path/
  Cargo.lock đều merge tự động); 1 commit replay thành rebase commit.
- Sau rebase: `cargo test -p vis-lsp` **29/29**; `path_impl::tests` **2/2**;
  `cargo check -p vis-cli --no-default-features --tests` PASS; `bin/vis-targets` core **18/18**;
  `./vis-dev gate --fast` **PASS — 3660 passed / 0 failed / 4 skipped** (253 s), R1..R7/fmt/clippy OK.
- Log ngoài repo (R7): `/home/ps/vislogs/st297-rebase-gate.log` —
  sha256 `6ceb4d836b4f6975f83c87b0b9f3aa7f9f9f57823d070fac386ed6549810c8aa`;
  `/home/ps/vislogs/st297-rebase-targets.log` — sha256
  `7debefd1a86aeff25d13a5407b2e5a537739658c77331fd713543c74d6dbef38`.
- Không đổi API/hành vi ngoài 3 điểm ST-297; **chưa merge** — chờ tag v0.4.1.
