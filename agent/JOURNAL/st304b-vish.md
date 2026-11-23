<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-304b — completion theo kiểu + POSIX job control (ViShell MVP)

> **Người làm**: W3, 2026-11-23 · nhánh **`plan/w3-st304b-vish`** từ **`6e0675a`** (ST-304).
> **Local only**, không ssh `mps`; không merge/push/tag. `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.

## 1. Completion (`completion.rs`)

API:
- `CompletionContext` — metadata `records`, `enums`, `cmds`, `vars`, `cwd`; builder `add_record/add_enum/add_cmd/add_var`;
  `from_source(src)` trích từ **CST `vis-syntax`** (TypeDecl/EnumDecl/CmdDecl/Param + `let/var`).
- `complete(src, cursor, &ctx)` / `ctx.complete(...)` trả `Candidate { text, kind }` với
  `CandidateKind::{Path, RecordField, EnumVariant, CmdFlag}`.
- Ngữ cảnh: `u.na` → field record; `Color.Gr` → variant enum; `build --re`/`test --fi` →
  cờ `cmd`/`async cmd`; `./a` → path (đọc thư mục, giữ tiền tố gõ).

Test: record/enum/cmd/async cmd/path/from_source/param hàm — **6/6**.

## 2. POSIX job control (`job.rs`)

- `parse_command`: `cmd &` → `Spawn`, `jobs`, `fg [id]`, `bg [id]`, `kill [-SIG] [id]`.
- `JobControl`: bảng job, `jobs()`, `handle()`; Unix `sh -c` spawn, `fg` wait, `bg` trạng thái,
  `kill` dừng/reap; Windows compile cfg (spawn trả lỗi rõ, parser/state vẫn dùng được).
- Test: parse + spawn/jobs/kill + bg/fg (Unix) — **3/3**.

## 3. Test / cổng

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vish-edit` | **17/17 pass** (8 editor + 6 completion + 3 job) |
| `./vis-dev gate --fast` | **PASS — 3688 passed / 4 skipped** (100 binary; wall **71,17 s**) |
| fmt / clippy / R1..R7 | **PASS** |
| 0 `unsafe` | Có (`#![forbid(unsafe_code)]`) |
| License | Không thêm dependency; `agent/JOURNAL/st304-license.log` (ST-304) vẫn hiệu lực |

Log gate ngoài repo: `/home/ps/vislogs/st304b-gate.log`
sha256 `5883a1e2de338ba727c38375b2963bf4f85695234170f928e90d24d9aeed55f2`.

## 4. Còn lại / blocker

- `fg`/`bg` hiện là MVP trong bảng job (fg wait, bg trạng thái); **process-group/terminal handoff
  đầy đủ** để dành cho ViShell hoàn chỉnh (`ST-305`/sau), không phải blocker completion.
- macOS/Windows quick để sau khi W2 merge `ST-300m1`; không ssh `mps`.
- Không merge/push/tag. Đĩa còn **29 G** (> 10 G).
