<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Tích hợp Phase B → nhánh chuẩn bị `v0.5.5` (`plan/v055-integration`)

> **Phạm vi**: hợp nhất **6 nhánh Phase B** (`ST-310a..f`) thành một nhánh tích hợp duy nhất để
> A0/Gatekeeper nghiệm thu trước khi bump/tag `v0.5.5`. Nhánh tạo từ **`main` `e9c62cb`**.
> **Không** merge/push/tag vào `main`; **không** ssh `mps`/`psvn`; worktree riêng
> `/home/ps/tmp/v055-integration` (không checkout `/home/ps/ViScript`).
> Môi trường: Rust **1.100.0**, `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.
> Thời điểm: 2026-11-23 (đêm), time-box tới 09:00.

## 1. Kết quả đo trên nhánh tích hợp

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3773 passed / 0 failed / 4 skipped** (nextest ~70 s) + doc-test riêng OK |
| fmt / clippy / luật **R1..R9** / toolchain / release / self-test / doc-examples / readme-examples | **PASS** |
| `bin/vis-targets` | **PASS** — phủ **18/18** crate workspace; `core` **38 PASS / 0 EXPECTED_RED / 0 FAIL**; `sqlite` **36 PASS / 2 EXPECTED_RED / 0 FAIL** |
| Diff 100k (`bin/vis-diff`, seed `20261115`) | **LECH = 0** |
| `cargo check --workspace --all-targets` | **PASS** (0 error, **0 warning**) |
| `vis-host-wasm`: `cargo check --target wasm32-wasip1` | **PASS** |
| Bump `0.5.5` / tag / push | **KHÔNG** (đúng phạm vi: chỉ chuẩn bị) |

## 2. Thứ tự merge (đúng thứ tự yêu cầu)

| # | Nhánh nguồn | Commit nguồn | Merge commit (`--no-ff`) | Conflict thật |
|---|---|---|---|---|
| 1 | `plan/st310a-core` | `4746799` | `23b91df` | không |
| 2 | `plan/st310b-terminal` | `71138de` | `5219f9b` | không |
| 3 | `plan/st310c-plugin-host` | `bed9582` | `3c919f6` | `agent/QUEUE.md`, `crates/vis-cli/Cargo.toml` |
| 4 | `plan/st310d-deps` | `29b819b` | `f59ebd8` | `Cargo.lock`, `crates/vis-cli/Cargo.toml` |
| 5 | `plan/st310e-tests` | `dfd3bcf` | `c08cea5` | `Cargo.lock`, vị trí tệp `vs3_plugin`/`common` (git "file location") |
| 6 | `plan/st310f-cleanup` | `46a3425` | `2dbf110` | `crates/vis-cli/Cargo.toml`, `test/README.md` |

Mỗi nhánh đều được merge bằng `--no-ff` (giữ nguyên commit gốc của nhánh). Sau mỗi merge đều
chạy `cargo check --workspace --all-targets` trước khi commit merge.

## 3. Xử lý conflict (không đổi semantics)

### 3.1 `ST-310c` — `QUEUE` + `vis-cli/Cargo.toml`

- `QUEUE`: giữ hàng `ST-310b` bản `REVIEW` (từ nhánh b) và hàng `ST-310c` bản `REVIEW` (từ nhánh c);
  không mất ghi chú nào.
- `vis-cli/Cargo.toml`: nhánh c thay `vis-plugin-abi` bằng `vis-plugin-host`, nhánh b thêm
  `vish-edit`/`vish-term`. Hợp nhất **cả ba**: `vis-plugin-host` + `vish-edit` + `vish-term`
  (e2e `vis plugin` nay dùng `vis_plugin_host`, không còn phụ thuộc trực tiếp guest ABI).

### 3.2 `ST-310d` — `Cargo.lock` + `vis-cli/Cargo.toml`

- `vis-cli/Cargo.toml`: lấy cấu trúc **feature-forwarding** của d (`sqlite`/`tls`/`http2`/
  `async-h2`/`pkg-sig` chỉ forward sang `vis-host-native`/`vis-pkg`; 6 dep OS/TLS/SQLite xuống
  `[dev-dependencies]`), **đồng thời** giữ `vis-plugin-host`, `vish-edit`, `vish-term` của c/b.
- `Cargo.lock`: bỏ `webpki-roots` khỏi gói `vis-cli` (không còn dùng trực tiếp); giữ
  `vish-edit`/`vish-term`.

### 3.3 `ST-310e` — gom test + `Cargo.lock`

- `Cargo.lock`: gói `vis-host-native` nhận **cả** `vis-core` (ST-310a) **và** `vis-bytecode`
  (dev-dep của ST-310e).
- Git báo "file location" vì `crates/vis-cli/tests/vs3_plugin.rs` được e chuyển sang
  `vis-plugin-abi/tests/vs3_plugin_cli.rs`, nhưng c thành lập `crates/vis-plugin-host`. Giải:
  đặt suite e2e tại **`crates/vis-plugin-host/tests/vs3_plugin_cli.rs`** (đúng đích QUEUE
  `ST-310e` ghi: "vs3_plugin → vis-plugin-host/tests/"), đổi `vis_plugin_abi::wasi::tim_runtime`
  → `vis_plugin_host::wasi::tim_runtime`; thêm `tests/common/mod.rs` trỏ
  `test/support/vis_test_support.rs`.
- Test `lsp_it` chuyển về `vis-lsp` nhưng còn `use vis_interp::…` — trái mục tiêu 0 dep
  `vis-interp` của ST-310a. Giải: đổi đường dẫn sang **nguồn duy nhất** `vis-core`
  (`vis_core::value::Value`, `vis_core::json::decode`, `vis_core::json::json`) — các hàm này
  vốn là re-export của `vis-interp` (cùng mã), nên **0 đổi hành vi**.
- `sqlite_it`/`tls_it` của `vis-host-native` cần hook test-only của interp
  (`vis_interp::run`, `rt::ReactorIoOutcome`, `stdlib::frame_net_*`) ⇒ thêm **dev-dependency**
  `vis-interp` cho `vis-host-native` (có ghi chú: production vẫn 0 dep; không vào artifact).
- `test/gate/r9-openat2-ngoai-le.txt`: ST-310c đã xoá `vis-plugin-abi/src/cap_fs.rs` ⇒ làm **rỗng**
  danh sách ngoại lệ đúng như ghi chú của chính tệp; R9 vẫn xanh (chỉ còn mention trong doc-comment).

### 3.4 `ST-310f` — `vis-cli/Cargo.toml`, `test/README.md`, `bin/vis-targets`

- `vis-cli/Cargo.toml`: f chỉ sửa comment đường dẫn log `st295-license.log` trong khối `rustix`
  cũ; khối này đã bị d chuyển xuống `[dev-dependencies]`. Giữ cấu trúc của d và **giữ ý** cập nhật
  đường dẫn archive (ghi chú ở dev-dep).
- `test/README.md`: hợp nhất bảng — có **cả** `gen_report_json.py` + `test/wasm/` (f) **và**
  "R1–R9" (e).
- `bin/vis-targets`: f viết danh sách **16 crate** (thời điểm trước a/c), trong khi workspace tích
  hợp có **18** (`vis-core`, `vis-plugin-host`). Bổ sung 2 crate + sửa số ô `18 × 2 + 2 = 38`
  để `check_workspace_coverage` xanh thật (không nới luật).

### 3.5 Sửa lỗi cross-target phát hiện khi tích hợp (wasm32)

`bin/vis-targets` lần đầu **FAIL 2 ô** `vis-plugin-host`/`vis-cli` × `wasm32-unknown-unknown`:
`PluginIo` chỉ có field `doc`/`ghi` khi `not(target_arch = "wasm32")`, còn `static_reg` khởi tạo
vô điều kiện. Sửa **thuần cfg, 0 đổi hành vi nền khác**:

- `ctx_voi_host` chỉ gắn `PluginIo` khi `not(wasm32)` (trên wasm32 guest không có FS trực tiếp —
  đúng nhánh `PluginCtx::read_text` đã trả lỗi).
- `use std::path::Path`, `PluginIo`, `doc_host`/`ghi_host` được cfg theo `not(wasm32)`.
- `vis-host-native` trong `vis-plugin-host` chuyển sang `[target.'cfg(unix)'.dependencies]`
  (chỉ đường `openat2` Unix dùng; Windows/wasm dùng `std::fs`/từ chối) — hết cảnh báo
  "unused dependency" của Cargo 1.100 trên wasm/windows.

Không đổi API, không đổi hành vi Linux/Windows; chỉ làm cho 2 ô check chéo wasm32 xanh.

## 4. Bằng chứng (lệnh + log)

```text
# gate
$ ./vis-dev gate --fast
--- fmt: OK
--- clippy: OK
--- luật R1..R9: OK
--- build: OK
--- doc-examples: OK (31 khối tài liệu: 26 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3)
     Summary: 3773 tests run: 3773 passed, 4 skipped
--- L0 test: OK
vis-dev gate --fast: PASS
# log: /tmp/opencode/gate-v055-final.log

# ma trận target
$ ./bin/vis-targets
vis-targets: phủ 100% workspace (18 crate)
core (vis-cli --no-default-features): PASS=38 EXPECTED_RED=0 FAIL=0
sqlite (vis-cli default, cấu hình cũ): PASS=36 EXPECTED_RED=2 FAIL=0
vis-targets: cả hai cấu hình đạt kỳ vọng riêng
# log: /tmp/opencode/targets-v055-final.log

# diff 100k
$ env VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff
DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=76.9 toc_do=1301/s seed=20261115
test result: ok. 7 passed; 0 failed
# log: /tmp/opencode/diff-v055-final.log

# host wasm (workspace riêng)
$ cd vis-host-wasm && cargo check --target wasm32-wasip1
Finished `dev` profile … in 7.07s
# log: /tmp/opencode/wasm-v055-final.log
```

Ghi chú môi trường: `/tmp/vrepfx.out` tồn tại do người dùng khác (`root`) làm bước
`test/report/run_fixtures.sh` in "Permission denied"; đã xoá tệp rác và chạy lại độc lập —
`1..5 ok` thật (không còn cảnh báo). Không sửa mã nguồn cho việc này.

## 5. Blocker / việc chưa làm (chuyển A0)

1. **Chưa bump `0.5.5`, chưa tag, chưa push** — đúng phạm vi tích hợp. Việc bump + tag thuộc
   `ST-310f`/Pha C sau khi A0 duyệt nhánh này.
2. **Chưa chạy Windows/macOS quick** trên nhánh tích hợp (không ssh `mps`/`psvn`; `mps` đang bận
   `ST-300m3`). `vis-targets` chỉ là **check chéo**, không phải bằng chứng chạy 3 OS.
3. **macOS Tier 1** vẫn chờ `ST-300m3` (3× `vis-gate-macos --full` + soak 4 h) — độc lập với
   nhánh này, không tuyên bố thay.
4. **`ST-309` Pha C** (fresh-clone 3-OS + WASI suite + soak 24 h + fuzz dài + `R37`) vẫn là bước
   sau, cần nhánh này được duyệt trước.
5. Nợ **#194** (statat theo tên ở `FsHost`) vẫn mở — ST-310c không đổi semantics, chỉ thống nhất
   `openat2`; không đóng nợ trong đợt này.
6. `agent/QUEUE.md`: 6 hàng `ST-310a..f` đã đổi `BLOCKED → REVIEW` và ghi merge commit tương ứng;
   **không** đánh dấu `DONE` (chờ A0/Gatekeeper nghiệm thu).
7. **`main` tiến 1 commit trong lúc tích hợp**: `main` nay ở `8906c70` (`[ST-309] fix Darwin fallback
   O_NOFOLLOW: ELOOP/NOTDIR -> VS0604`, 2026-11-23 23:14) sửa `crates/vis-host-native/src/quyen_guard.rs`
   và `quyen_mo.rs`. Nhánh này giữ đúng căn cứ **`e9c62cb`** theo yêu cầu và **chưa** gộp commit đó;
   ST-310f đã đổi tên hai tệp trên thành `quyen/guard.rs`/`quyen/mo.rs`, nên khi gộp sẽ có conflict
   rename-vs-modify (xử lý cơ học: giữ cả vá Darwin lẫn `mo_tep_doc`/`mo_tep_ghi` của ST-310c).
   **A0 quyết định** gộp `8906c70` trước hay sau nghiệm thu nhánh này.
