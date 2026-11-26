<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# `ST-318e` — R1 mềm `vis-bytecode` + `vis-cli` (Builder, 2026-11-27)

- **Vai**: Builder (Agent 2) · **Worktree**: `/home/ps/viswt/st318e` · **Nhánh**: `plan/st318e`
- **Nền**: `main` @ `baa2dbd3` (`VD-297`) · **Lane**: `crates/vis-bytecode/src/codegen*`,
  `crates/vis-cli/src/vistest*`
- **Luật move**: nguyên văn từng item, chỉ thêm `pub(super)`/import; 0 đổi hành vi/API/VBC1.

## Kết quả LOC

| Tệp | Trước | Sau |
|---|---:|---:|
| `crates/vis-bytecode/src/codegen.rs` | 1069 | **447** |
| `crates/vis-bytecode/src/codegen/expr.rs` (mới) | — | **628** |
| `crates/vis-cli/src/vistest.rs` | 1061 | **201** |
| `crates/vis-cli/src/vistest/run.rs` (mới) | — | **465** |
| `crates/vis-cli/src/vistest/ca.rs` (mới) | — | **255** |
| `crates/vis-cli/src/vistest/tien_trinh.rs` (mới) | — | **174** |

- `codegen.rs` giữ entry `compile*`, `Gen`, `Emit`; `expr` (622 dòng, băm trước/sau khớp sau khi
  bỏ chữ `pub(super)`) sang `codegen/expr.rs`. Các trách nhiệm lệnh/khai báo/pattern đã ở
  `cau.rs`/`than.rs`/`pattern.rs` từ trước.
- `vistest.rs` còn doc + helper nhỏ (`esc_json`/`mot_dong`/`CaJson`/`bo_spdx`/`tim_mac_dinh`/
  `gom`/`ds_test`/`thanh_main`/`json_body`); `run` + `usage` sang `run.rs`; spawn/timeout sang
  `tien_trinh.rs`; `Kq`/`chay_mot_lan`/tuần tự/song song sang `ca.rs`; `vis test` giữ
  `crate::vistest::run`.

## Cổng (log `/home/ps/vislogs/st318e-*.log`)

| Cổng | Kết quả |
|---|---|
| `cargo test -p vis-bytecode -p vis-cli` | **PASS** (bytecode 103+1; cli 523+254+496+125+287+3) |
| `./vis-dev gate --fast` | **PASS** 3902/3902, clippy/fmt/rules OK |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **`lech=0`** (93.768 chạy, 90,5 s) |
| `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** |

- `agent/API-VBC1.txt`, `API-CODES.txt`, `API-STD.txt`: **không đổi**.
- R8: **39/40** (`vis-cli/tests` 8/15) — không thêm tệp test cấp 1; 0 `unsafe` mới.
- Commit: `22e81371` (codegen) · `70f8ab29` (vistest) · `832bbfee` (bỏ import thừa).
