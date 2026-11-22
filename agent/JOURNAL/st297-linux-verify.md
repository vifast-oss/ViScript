<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-297 — xác minh lặp trên Linux (LSP URI/UNC + path `--grant`)

> **Người chạy**: W3, 2026-11-23 · nhánh **`plan/w3-st297-linux-verify`** từ **`f4cb136`**
> (ST-297 đã merge). Không push/merge/tag; không sửa code; không đụng `SO-NO.md`/`QUEUE.md`.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`, target trong worktree.

## 1. `vis-lsp` × 10

`cargo test -p vis-lsp`, load đầu **3,10**:

| Lần | Kết quả |
|---:|---|
| 1–10 | **29 passed / 0 failed** mỗi lần (0,02–0,04 s) |

**10/10 lần xanh, 29/29 ca/lần**, không flake.

## 2. `path_impl` trong `vis-cli --bin vis` × 10

`cargo test -p vis-cli --bin vis path_impl::tests`:

| Lần | Kết quả |
|---:|---|
| 1–10 | **2 passed / 0 failed** mỗi lần (143 filtered out) |

**10/10 lần xanh, 2/2 ca/lần**, không flake.

## 3. Tên 4 ca mới (một lần `--nocapture`)

`cargo test -p vis-lsp -- --nocapture` → **29/29** (0,03 s). Bốn ca mới của ST-297:

1. `vd258_tests::uri_st297_unc_mixed_percent_roundtrip` — UNC `file://server/share`,
   `file:////…`, mixed `\`/`/`, percent UTF-8, round-trip URI ↔ path.
2. `workspace::tests::symbol_uri_chuan_hoa_percent_utf8` — URI `workspace/symbol` có space/UTF-8
   được percent-encode đúng.
3. `quyen::path_impl::tests::phan_giai_lenh_windows_khong_do_path` — `--grant` path `C:\…`/UNC
   không bị dò `PATH`.
4. `quyen::path_impl::tests::phan_loai_path_windows_va_unix` — phân loại path Windows/Unix
   (ổ đĩa, UNC, `/`, `\`).

## 4. Log ngoài repo

| Log | sha256 |
|---|---|
| `/home/ps/vislogs/st297-lsp-10.log` | `d425b46714ce71f53f76cbfa03948d66459b1ac30ceb0168706ff59f82879e5d` |
| `/home/ps/vislogs/st297-path-10.log` | `a4e5c1ee388f53280b3f0776b8822a705c9236e4d1238dd2ff544fed350ed773` |
| `/home/ps/vislogs/st297-lsp-nocapture.log` | `32e7be4cf136eb602a13c05a443ce5c6b589b825beef77f708e88fb376f12ecd` |

Không chạy gate/soak/diff (theo lệnh; W4 đang soak Windows).

## 5. Blocker

Không. Không sửa `SO-NO.md`/`QUEUE.md`; không push/merge/tag.
