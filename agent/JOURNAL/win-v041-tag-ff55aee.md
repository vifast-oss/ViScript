<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows — xác nhận tag `v0.4.1` @ `ff55aee` (lock + version + quick)

> **Người đo**: subagent VS3-W4, 2026-11-23 · time-box 45 phút.
> **Mốc**: `main` **`ff55aeeea144bc3717124413928d63dc6a7bd494`**
> (`[ST-296d] R34: final QA v0.4.1 + lock fix`).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-v041-tag-ff55aee` @ `ff55aee`, dirty **0**;
> `TMPDIR=/c/Users/ps/tmp`; không job psvn khác. Không tag/push/merge; không GitHub Actions.
> **Kết quả**: lock **sạch sau `vis-targets`**, version **0.4.1**, quick **pkg 25 / h2 33 /
> cancel 8 / select 25**. **Sản phẩm đủ điều kiện tag `v0.4.1` @ `ff55aee`.**
> **Lưu ý công cụ**: `bin/vis-gate-windows --quick` **tại `ff55aee` còn tên test cũ** (ST-296a đã
> gom binary) ⇒ fail ngay; đã vá 1 dòng trên nhánh này và chạy lại PASS. Nếu muốn quick hoạt động
> trên tag, merge fix này trước (khuyến nghị); nếu tag `ff55aee` ngay thì ghi known-issue cho quick
> (full không bị ảnh hưởng).

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `bash bin/vis-targets` | **rc 0**, 18/18 + 17/1; sau đó `git status --porcelain` **rỗng** (lock fix OK) |
| `vis --version` (release) | **`0.4.1`**; JSON đúng |
| `check` / `run` | TAP `1..1 ok 1`; `run` → `42` |
| Release build | `vis.exe` **11 718 144 B**, sha256 `d9ff1ab1…a1962bc`; dirty sau build **0** |
| `--quick` (tool gốc) | **FAIL rc 101** — `no test target named vs3_http_cancel` (tên cũ) |
| `--quick` (sau vá 1 dòng) | **PASS 21 s** — pkg **25/25**, h2 **33/33**, cancel **8/8**, select **25/25** |
| Blocker sản phẩm | **Không**; chỉ có defect tooling `--quick` (đã vá trên nhánh) |

## 1. Lock fix — `vis-targets` không còn làm bẩn cây

- Worktree sạch lúc đầu; chạy `bash bin/vis-targets`:
  - **rc 0**, **64 s**; core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED**.
  - `git status --porcelain` sau đó: **rỗng** — `vis-host-wasm/Cargo.lock` đã đúng 0.4.1
    (không còn `M vis-host-wasm/Cargo.lock`).
- Log `v041tag-targets.log` (trên psvn).

## 2. Release build / version

- `cargo build --release -p vis-cli` rc 0, **69 s**;
  `target/release/vis.exe` **11 718 144 B**,
  sha256 **`d9ff1ab13da9f109a751cabdae7e289ee48d0a85bd2709afdecef81d0a1962bc`**.
- `vis --version` → **`vis 0.4.1`**; `version --json` đúng.
- `check` TAP `1..1 ok 1`; `run` → `42`; sau build `git status` vẫn **sạch**.

## 3. `vis-gate-windows --quick`

- **Lần 1 (tool tại `ff55aee`)**: **FAIL rc 101** —
  `error: no test target named 'vs3_http_cancel'` (ST-296a đã gom thành `net_pkg_it`,
  `http2_it`, `async_it`). Đây là defect **tooling**, không phải sản phẩm.
- **Vá trên nhánh** `bin/vis-gate-windows`: quick dùng binary nhóm:
  `net_pkg_it pkg:: -- --skip siet_caps_pkg` + `http2_it` + `net_pkg_it cancel_http_` +
  `async_it vs3_select::`.
- **Lần 2 (sau vá)**: **PASS**, **21 s**:
  - `pkg` **25/25**;
  - `http2_it` **33/33**;
  - `cancel_http_` **8/8**;
  - `select` **25/25**.
- `vis-remote-exit=0`, `[exit=0]`.

## 4. Kết luận / đề xuất tag

- **Sản phẩm:** xác nhận **đủ điều kiện tag `v0.4.1` @ `ff55aee`** — lock sạch, version 0.4.1,
  check/run/quick (sau vá tool) đạt; full/release trước đó tại `5d7791c` đã PASS
  (`3607/0/3`, `3606/0/3`).
- **Tooling:** `--quick` tại đúng `ff55aee` còn broken do tên test cũ. Khuyến nghị **merge fix 1 dòng
  này rồi tag** (commit `[ST-296d]` trên nhánh); nếu A0 tag `ff55aee` ngay, ghi known-issue cho
  `--quick` (không ảnh hưởng `--full`/sản phẩm).
- Không tag/push/merge; không GitHub Actions. `psvn` `C:` còn **74 G**; local `/` **20 G** (>10 G).
