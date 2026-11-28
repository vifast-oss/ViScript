<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-324c — R1 headroom: tách helper ra khỏi `cli/fmt.rs`

- **Ngày**: 2026-11-28 · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st324c`
- **Nền**: `main` @ `d107707537814afe389cf2d9c46cc8e663038c22`
- **Worktree**: `/home/ps/viswt/st324c` — **không** chạm repo chính, **không** merge/push.
- **Trạng thái**: mã + golden + test + cổng **PASS**; chờ Gatekeeper audit.
- **Nguồn đề xuất**: `agent/QUEUE.md` dòng ST-324c (W3 · Sóng 13 / R1 headroom), theo mẫu ST-322a.

## 0. TL;DR

`fmt.rs` đúng **974 dòng**. Tách **nguyên văn** 4 helper đầu ra/định dạng
(`tep_json`, `diff_dong`, `xuat_fmt_diff`, `xuat_fmt_json` — dòng 95–215) sang tệp mới
`cli/fmt/ho_tro.rs`, rồi import lại tại `fmt.rs` bằng `use ho_tro::{…};`.

| | Trước | Sau |
|---|---:|---:|
| `crates/vis-cli/src/cli/fmt.rs` | 974 | **855** (≤ 900) |
| `crates/vis-cli/src/cli/fmt/ho_tro.rs` | — | 130 |

Không đổi stdout/stderr/exit, không đổi CLI/API, không sửa `main.rs`/`check.rs`/`run_cmd.rs`.

## 1. Điều kiện DỪNG (helper dùng chung) — đã kiểm, không kích hoạt

Rà `grep -rlw` toàn `crates/vis-cli/src` cho **19** item do `fmt.rs` định nghĩa (`dem_lech_thut_le`,
`tep_json`, `diff_dong`, `xuat_fmt_diff`, `xuat_fmt_json`, `mode_tu_chuoi`, `FmtMode`, `gom_nguon`,
`gom_theo_doi_so`, `la_input_co_y`, `run_fmt_stdin`, `KetQuaDoc`, `doc_va_dinh_dang`,
`xuat_fmt_json_tep`, `xuat_fmt_list`, `dong_fmt_stat`, `xuat_fmt_stat`, `xuat_fmt_stat_stdin`, …
ngoài `fmt.rs`): **0 tệp** — tất cả là `fn`/`enum` riêng, không lọt ra `check`/`run_cmd`/`main`.
Điều kiện "dùng chung ⇒ DỪNG" **không** xảy ra ⇒ tiếp tục gộp.

`main.rs:211` gọi `cli::fmt::run_fmt(...)` — hàm này **giữ nguyên** trong `fmt.rs`, không di chuyển.

## 2. Thay đổi

- **`cli/fmt/ho_tro.rs`** (tệp mới, 130 dòng): dòng 95–215 của `fmt.rs` cũ chép **byte-for-byte**
  (đã so bằng `difflib` với `git show HEAD:…`). Gồm doc comment đầy đủ của từng helper.
- **`cli/fmt.rs`**: xoá 121 dòng đó, thêm ở đầu:
  ```rust
  mod ho_tro;
  use ho_tro::{diff_dong, tep_json, xuat_fmt_diff, xuat_fmt_json};
  ```
- **Hiệu chỉnh cơ học duy nhất**: `fn` → `pub(super) fn` cho 4 helper. Bắt buộc vì quyền riêng
  Rust là một chiều (module cha không import được item `fn` riêng của module con); `pub(super)`
  giữ các hàm chỉ thấy trong cây `cli::fmt`, **không** lọt ra crate — cùng cách xử lý ST-322a.
  Thân hàm, chữ ký (về ngữ nghĩa), doc comment và kết quả không đổi. `rustfmt` chỉ bọc lại dòng
  chữ ký dài của `tep_json` (thêm `pub(super) ` vượt ngưỡng width) — thuần định dạng.

## 3. Kiểm chứng

Mọi lệnh chạy trong worktree, `PATH=/home/ps/.cargo/bin:$PATH` (cargo/rustc **1.100.0** theo
`rust-toolchain.toml`), `TMPDIR=/home/ps/vislogs/st324c-tmp` (**đĩa** nvme, không tmpfs),
`CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`, **không clean**.
Disk `/` lúc commit: **20 G available** (> ngưỡng dừng 5 G) — không chạm ngưỡng.

| Lệnh | Kết quả | Log |
|---|---|---|
| golden `vis fmt --check/--json/--diff/--stat/--list` + stdin + misuse | **byte-identical** 109/109 artifact (stdout+stderr+rc), hai `MANIFEST.sha256` **cùng** `9bd8d37e5aa84460` | `st324c-golden-{before,after}/` |
| `cargo test -p vis-cli` | **PASS** — 1816 passed / 0 failed (12 suite + doc-test) | `st324c-cargo-test.log` `1c9203d7e004b8dc` |
| `./vis-dev gate --fast` | **PASS rc 0** — fmt/clippy/R1..R10/self-test/build/doc-examples + L0 **3948 passed / 4 skipped** | `st324c-gate-fast.log` `b0287543ceae452f` |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS rc 0**, 0 warning | `st324c-clippy.log` `28a6f3d37cf90a7b` |
| `./bin/vis test test/conformance` | **1..573 — 573 ok / 0 not ok**, rc 0 | `st324c-conformance.log` `3ec6d29ddaf4dde9` |

Golden chạy trên **2 corpus**: (a) `test/fmt/` + `test/check` kiểu fixture bị loại trừ (đường
`--check test/fmt/` ⇒ 0 tệp đúng quy tắc `la_input_co_y`), (b) bản sao 9 tệp tại
`/home/ps/vislogs/st324c-corpus/` (ngoài vùng loại trừ ⇒ 9 tệp, 5 cần sửa, 1 lỗi) — gồm cả
`.vish` để ép chế độ command. Mọi tổ hợp cờ `--json/--diff/--stat/--list`, thứ tự đối số,
đầu vào rỗng/lỗi cú pháp và các ca dùng sai đều nằm trong manifest so sánh.

**Không chạy `vis-diff`**: đây là thay đổi CLI-only (không đụng compiler/runtime/VM) theo chỉ dẫn.

## 4. Luật

- **R1**: 0 tệp > 1.000 dòng trong `crates/vis-cli/src`; `fmt.rs` **855** ≤ 900 (mục tiêu đề bài).
- **R8**: tổng `.rs` cấp 1 trong `crates/*/tests/` + `vis-host-linux/tests/` = **39/40**;
  riêng `crates/vis-cli/tests/` = **8/15**. **0** tệp test cấp 1 mới (helper nằm dưới `src/`).
- **0 `unsafe`**, 0 crate mới, 0 `#[ignore]`, 0 nới ngưỡng, 0 sửa test/golden.
- `git status` trước commit: `M cli/fmt.rs`, `?? cli/fmt/ho_tro.rs`, `??` journal — chỉ 3 tệp.

## 5. Kết luận

Di chuyển thuần tuý, 0 đổi hành vi/CLI/API; cổng máy tái lập được **PASS**. Không merge, không
push, không đụng `agent/QUEUE.md`/`STATE.md`/`DECISIONS.md`. Chờ Gatekeeper (Agent 1) audit rồi
Agent 0 duyệt.
