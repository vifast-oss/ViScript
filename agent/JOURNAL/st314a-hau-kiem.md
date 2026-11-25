# ST-314a — Hậu-kiểm `VD-290` (W1/W4)

> **Trạng thái**: `REVIEW` · **Ngày**: 2026-11-25 · **Phụ thuộc**: `ST-313d` / `VD-290`
> **Commit lát**: `311d2288` (mã + test) + follow-up `a7cf2533`, `d25f50dc`, `c34e8b44`, `da66418a`, `423b4d3e` · **Phạm vi**: 4 phát hiện kiểm toán của Agent 0
> + 1 lỗi cổng phát sinh khi chạy lại `gate --fast` trên `d8857694`.

## 1. Vá lỗi thật `bin/vis-wasm-conformance` (chạy thật 2 engine)

**Lỗi**: `NODE_DRIVER` destructure `[wasmPath, guest, host, ro, ...args]` nhưng Python không truyền
`ro`; chuỗi `--interp`/`--vm` bị gán vào `ro` rồi bỏ ⇒ harness chạy **cùng engine mặc định
(interp) 2 lần**, chưa từng kiểm `--vm`.

**Vá**:
- Bỏ `ro` khỏi destructure; `VIS_WASM_PREOPENS` chỉ còn `guest` (corpus này không dùng preopen ghi).
- `chay(driver, root, *runner_args)` chuyển tiếp **nguyên văn** đối số runner; mỗi ca chạy
  `--interp /data/prog.vis` và `--vm /data/prog.vis`.
- Thêm **self-check** `tu_kiem`: chạy runner với cờ lạ `--co-khong-ton-tai`; `vis-wasm-run` phải
  từ chối `exit 2`. Nếu cờ bị nuốt như lỗi cũ, self-check fail ngay.
- Thêm `argparse` cấp harness: cờ lạ của chính script cũng `exit 2`.

**Đo thật** (log `/home/ps/vislogs/st314a-wasm-conformance.log`):

```text
# 299 ca hợp lệ trên WASI
# tổng: 299 ca x 2 engine, 0 hỏng
EXIT=0
sha256 567b75c5f4619c18d673157443791ab5673dfae738058fdf3d42f1f983bc701e
```

## 2. Hygiene workspace & R8

- `Cargo.toml`: thêm tường minh `"crates/vis-hir"` vào `[workspace].members`;
  `cargo metadata --no-deps` = **23** workspace member (không đổi tổng, chỉ hết gộp ngầm).
- `crates/vis-core/Cargo.toml`: bỏ `"HTTP client"` khỏi `description` (HTTP đã ở `vis-http`).
- `bin/vis-dev` R8: tổng binary `.rs` cấp 1 = `crates/*/tests/*.rs` **+** `vis-host-linux/tests/*.rs`;
  hiện **38 + 1 = 39 ≤ 40**; `crates/vis-cli/tests` = **8 ≤ 15**.

## 3. `StdId` compile-time exhaustive 100%

- `crates/vis-runtime/src/stdlib.rs`: bỏ hẳn guard `starts_with("path.")` / `starts_with("fs./net./http.")`
  và **bỏ nhánh `_ =>`** cuối `call_by_id`. Match phủ đủ **167/167** biến thể `StdId`;
  thêm biến thể mới mà quên route ⇒ **lỗi biên dịch**.
- `stdlib/path/dispatch.rs` + `stdlib/fs_net/dispatch.rs`: nhận `StdId` (không nhận `&str`),
  match theo biến thể enum; tên chỉ dùng cho thông điệp lỗi (`id.as_str()`). Không còn
  `match name`/`starts_with` ở **đường dispatch vào**; ba helper lá cùng nhóm (`fsbytes`,
  `fsmkdirmall`, `goi_metadata`) nay cũng nhận `StdId`/`MetadataOp` (không còn chọn hàm theo
  chuỗi); `id.as_str()` chỉ còn cho thông điệp lỗi.
- Hai module con giữ nhánh `_ => unreachable!("...")` **phòng vệ** cho biến thể ngoài miền
  (không thể tới từ top-level); đây không phải fallback im lặng.

## 4. Sửa đỏ cổng phát sinh từ đồng bộ tài liệu `VD-290`

`gate --fast` lần 1 đỏ ở `vis-types::types_it::hir::ha_15_khoi_tai_lieu_0_chan_doan_va_dung_so_khai_bao`
(`khối dòng 1480 chưa có số kỳ vọng`): A0 đồng bộ bảng crate §14 gọn **1 dòng** (net −1) làm hai
khối `vis` cuối dịch dòng, trong khi `DOC_BLOCKS` còn ghim 1481/1528.
Đã cập nhật `crates/vis-types/tests/types_it/hir.rs`: **1481 → 1480**, **1528 → 1527**.

## 5. Cổng đo (Chế độ Phát triển `VD-289`)

| Cổng | Kết quả | Bằng chứng |
|---|---|---|
| `./vis-dev gate --fast` | **PASS** — 3804/3804 test (4 skip), 0 fail | `/home/ps/vislogs/st314a-gate-fast-2.log`, sha `5a4ce999126aaaae744e80f10e92af3d2c9dbb1fc78fbb37ea4e041de91df44c` |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** — 100.000 chương trình, 93.768 chạy, 6.232 lỗi tĩnh, seed `3523158054`, 62,7 s | `/home/ps/vislogs/st314a-vis-diff.log`, sha `4d23935231a7bf5a230706e0fae375fff23fbcaf4e7fb344dee25ad994347125` |
| `./bin/vis-targets` | **PASS** — cả hai cấu hình đạt kỳ vọng riêng | `/home/ps/vislogs/st314a-vis-targets.log`, sha `e891b3ed5e5c64320a41a4c320721b1d50ea982ef01a50283782befb029bd593` |
| `vis-wasm-conformance` | **299 × 2 engine, 0 hỏng** (self-check cờ lạ OK) | `/home/ps/vislogs/st314a-wasm-conformance.log`, sha `567b75c5f4619c18d673157443791ab5673dfae738058fdf3d42f1f983bc701e` |
| clippy Linux (trong `gate --fast`) | **PASS** `--workspace --all-targets -D warnings` | gate log |
| clippy Windows `psvn` @`311d2288` | **PASS** — `vis-remote-exit=0` | `/home/ps/vislogs/st314a-clippy-psvn.log` |
| clippy macOS `mps` | **PASS @`311d2288`** (`vis-remote-exit=0`), nhưng **không chạy lại/full gate** theo chỉ đạo người dùng (`mps` bận dự án khác) | `/home/ps/vislogs/st314a-clippy-mps.log`, sha `aa588dd0…` |

## 6. Tệp thay đổi trong `311d2288`

```text
bin/vis-dev
bin/vis-wasm-conformance
Cargo.toml
crates/vis-core/Cargo.toml
crates/vis-runtime/src/stdlib.rs
crates/vis-runtime/src/stdlib/fs_net/dispatch.rs
crates/vis-runtime/src/stdlib/path/dispatch.rs
crates/vis-types/tests/types_it/hir.rs
```

## 7. Bổ sung bằng chứng khung cao điểm (0-LLM, `VD-287`)

Sau khi nộp `REVIEW`, chạy tiếp các việc nặng 0-LLM tới sát giờ thấp điểm:

| Việc | Kết quả | Bằng chứng |
|---|---|---|
| Windows full @`5d857b84` (`vis-gate-windows --full`) | `cargo test --workspace`: **3.729 passed / 0 failed / 4 ignored** (90 binary); `host_conformance` **54 ca / 120 lượt, 0 lệch, VM 54/54**; `vis-targets` **23 crate** PASS (core 47/0/0, sqlite 46/1 EXPECTED_RED/0); `vis-dag-check` OK (guard 7 crate lúc đó) | `/home/ps/vislogs/st314a-win-full.log`, sha `84f1593a7d3bf45454d8b060face61e262dd2b2e2b30a2abf24f958d1fc2cead` |
| 5× `vis-diff 100k` (seed `20261125/101/202/303/404`) | cả 5 **lech=0** (93.767–93.771 chương trình chạy/lượt) | `st314a-diff-<seed>.log` |
| +3× `vis-diff 100k` (seed `505/606/707`) | cả 3 **lech=0** | `st314a-diff-<seed>.log` |
| Fuzz parser 115 phút (`vis-dev nightly --skip-gate`) | **959.027.870 ca / 21.766.452.334 token, 0 vi phạm**; vòng tròn fmt 475 tệp bất động | `/home/ps/vislogs/st314a-fuzz-115m.log`, sha `b145989a698f5cc5da77b0542faceddb0b6bda55f87321179a5c82bc6fec92c4` |

## 8. Hậu-kiểm khung thấp điểm: self-check cờ lạ từng cho lọt đúng lỗi cũ

Kiểm chứng `tu_kiem` bằng cách **tái tạo lỗi cũ** trên bản sao tạm (thêm lại `ro` vào
destructure `NODE_DRIVER`): self-check **PASS sai** (`exit 0`) rồi harness chạy tiếp
**299 ca × 2 engine, 0 hỏng** — đúng triệu chứng "chạy interp 2 lần". Nguyên nhân: thư mục
self-check rỗng nên `vis-wasm-run` thiếu tệp cũng trả `exit 2`, trùng mã đang mong đợi.

**Vá**: self-check tạo `prog.vis` hợp lệ in `SELF-CHECK-LEAK`; yêu cầu đồng thời `rc == 2`,
stdout **không** chứa `SELF-CHECK-LEAK`, và stderr chứa tên cờ lạ.

| Bản | Kết quả | Bằng chứng |
|---|---|---|
| Bản sao tái tạo lỗi `ro` + guard cũ | **LỌT** — `exit 0`, chạy 299 × 2, 0 hỏng | `/home/ps/vislogs/st314a-selfcheck-negative.log` |
| Bản sao tái tạo lỗi `ro` + guard mới | **BẮT** — `exit 2`, `SELF-CHECK … stdout='SELF-CHECK-LEAK\n'` | `/home/ps/vislogs/st314a-selfcheck-negative-fixed.log`, sha `bfe90799a381977b387499d388c6ae6d2692d4e32cd7326617686e6766e0d5b4` |
| Bản thật + guard mới | **299 ca × 2 engine, 0 hỏng**, `EXIT=0` | `/home/ps/vislogs/st314a-lp-wasm-conformance-fixed.log`, sha `0d55130e4c2fe11faf9415f00b959100aba812586576b2c8e1b41602901dfdee` |

Commit vá: `b663d89c`.

## 9. Việc song song khung thấp điểm `2026-11-25 04:00–06:00 UTC`

Chạy song song 1 chuỗi cổng nặng 0-LLM + 3 audit độc lập (subagent), không chồng cargo:

| Việc | Kết quả | Bằng chứng |
|---|---|---|
| `./vis-dev gate --fast` @`dd2b147c` | **3804/3804 passed**, 4 skipped, `EXIT=0` | `/home/ps/vislogs/st314a-lp-gate.log`, sha `fb7fccc872e1ebb04047c92111fafd028188e0433d59fc1db9f983985c151fe0` |
| `bin/vis-wasm-sem` @HEAD | **390/390 eligible, 0 fail, coverage 100%** (525 total, 135 excluded) | `/home/ps/vislogs/st314a-lp-wasm-sem.log`, sha `41777564ef6cf9232076571dd75db09f0fde33ea5552a987cf49596842b4d203` |
| `bin/vis-wasm-suite` @HEAD | **28/28, 0 hỏng** (runtime=node) | `/home/ps/vislogs/st314a-lp-wasm-suite.log`, sha `584a6aa2510897a13a5cbf9464e89c658aaac77d26b6551f419e777de9b881bf` |
| `bin/vis-wasm-conformance` (guard mới) | **299 × 2 engine, 0 hỏng**, `EXIT=0` | `/home/ps/vislogs/st314a-lp-wasm-conformance-fixed.log` |

### 9.1 `vis-targets` @HEAD và vá đường dẫn tài liệu

- Baseline trước vá: `core PASS=47/0/0`; `sqlite PASS=45/2 EXPECTED_RED/0`; `EXIT=0`
  (`/home/ps/vislogs/st314a-lp-vis-targets-before.log`, sha `778aedff…`).
- Ô `vis-cli × x86_64-pc-windows-msvc` **phụ thuộc máy**: trên `psvn` (Windows, có CC) PASS và in
  advisory #156; trên `ps` cross-check vẫn `EXPECTED_RED` (thiếu `lib.exe`/`cc`). Vì vậy **giữ nguyên
  `expected_red()`** (msvc + wasm32), không đổi kỳ vọng cổng.
- Vá thật: `LY_DO` trỏ `agent/TARGET-MATRIX-2026-11-20.md` (không tồn tại) → đúng
  `agent/archive/reports/TARGET-MATRIX-2026-11-20.md`. Commit `6245bf12`.
- Sau vá: `core 47/0/0`, `sqlite 45/2/0`, `EXIT=0`
  (`/home/ps/vislogs/st314a-lp-vis-targets-after.log`, sha `553c6f60…`).

## 10. Audit song song khung thấp điểm & vá phát hiện

### 10.1 `vis-dag-check` — mở rộng từ 7 → 23 crate / 5 tầng

Audit tooling: bảng `CAM` cũ chỉ phủ **7/23** crate; **185 cạnh xuôi tầng** (lower→upper) chưa
được chặn, gồm cả `vis-core → vis-host-native`, `vis-syntax → vis-types`, `vis-interp → vis-vm`.
Đã thay bằng:
- `TANG_CUA`: đủ 23 crate × 5 tầng (`T0` nền → `T4` CLI/conformance);
- `CAM_NGANG`: giữ nguyên các cạnh **cùng tầng** đã có (không nới, không thêm chính sách mới);
- cạnh ngược tầng bị bắt một lần, cạnh cùng tầng báo riêng; thiếu crate trong bảng tầng ⇒ lỗi.

| Kiểm | Kết quả |
|---|---|
| Thật @HEAD | `vis-dag-check: OK — 23 crate / 5 tầng, 0 cạnh cấm`, exit 0 |
| Giả lập 2 cạnh **ngược tầng** (`vis-core→vis-host-native`, `vis-syntax→vis-types`) qua `CARGO` giả (bản đầu; nay self-test có 3 cạnh + positive control — §15) | exit 2, in đúng `vis-core[T1] -> vis-host-native[T3]` và `vis-syntax[T0] -> vis-types[T2]` |
| Giả lập crate thứ 24 chưa khai tầng | exit 2, in `crate `vis-la` chưa khai tầng trong TANG_CUA` |

### 10.2 Self-check `vis-wasm-conformance` — thêm probe engine

- Lỗi F1 (self-check cũ lọt lỗi `ro`) đã vá ở `b663d89c`; audit độc lập xác nhận.
- Bổ sung probe **dương** cho `--interp`/`--vm`: chạy `prog.vis` hợp lệ, đòi `rc=0` + stdout có
  `SELF-CHECK-LEAK`, bắt lỗi caller truyền `engine` thiếu `--`.
- Kiểm âm: tái tạo lỗi `ro` ⇒ **exit 2**; bỏ `--` ở probe engine ⇒ **exit 2**
  (`SELF-CHECK: --interp không tới runner/không chạy`).
- Kiểm dương: **299 ca × 2 engine, 0 hỏng**.

### 10.3 Nit `path` dispatch

`match id` nội bộ nhóm `basename/dirname/ext/stem` bỏ `_ => stem_voi`; nay `StdId::PathStem`
tường minh + `_ => unreachable!` phòng vệ, tránh biến thể mới âm thầm thành `path.stem`.

### 10.4 R8

Giới hạn đúng: `vis-cli` **8/15**, tổng **38 + 1 = 39/40**. Ghi rõ trong comment: `vis-host-wasm`
là **workspace riêng** nên ngoài trần AGENTS §6.3; `tests/<domain>/*.rs` là module `#[path]`, không
phải binary cấp 1; `[[test]] path=…` lồng nhau (chưa có) là điểm mù đã biết.

### 10.5 `vis-targets`

Giữ nguyên `expected_red` (msvc + wasm32) vì kết quả **phụ thuộc máy**: `psvn` có CC ⇒ msvc PASS
(in advisory), `ps` cross-check ⇒ msvc `EXPECTED_RED`. Chỉ vá đường dẫn tài liệu cũ (commit
`6245bf12`); baseline/sau vá: `core 47/0/0`, `sqlite 45/2/0`, `EXIT=0`. Trên `psvn`: sqlite **46 PASS + 1 EXPECTED_RED** (msvc PASS, wasm32 đỏ vì `getrandom`); trên `ps`: **45 + 2** (msvc đỏ vì thiếu C toolchain, wasm32 đỏ) — càng khẳng định phải giữ cả hai ô trong `expected_red`.

### 10.6 Cổng batch low-peak 2 (@HEAD sau vá)

| Cổng | Kết quả | Bằng chứng |
|---|---|---|
| `bin/vis-dag-check` | **OK — 23 crate / 5 tầng, 0 cạnh cấm**; negative giả lập exit 2 | log phiên |
| `vis-wasm-conformance` (guard + engine probe) | **299 ca × 2 engine, 0 hỏng**, `EXIT=0` | `/home/ps/vislogs/st314a-lp2-wasm-conformance.log`, sha `a2ddd230fd86b53760ddcd2eae0a5dcd94ae6ec97fea60ad37cd8d041078cf75` |
| `./vis-dev gate --fast` | **3804/3804 passed**, 4 skipped, `EXIT=0` | `/home/ps/vislogs/st314a-lp2-gate.log`, sha `0c90814f5fffc3f118bd6a01bf21ebefdbb610fddf571ff4a37e1c5acaf4b078` |
| `bin/vis-wasm-sem` @`a7cf2533` | **390/390, 0 fail, coverage 100%** (525 total) | `/home/ps/vislogs/st314a-lp2-wasm-sem.log`, sha `41777564ef6cf9232076571dd75db09f0fde33ea5552a987cf49596842b4d203` |
| `bin/vis-wasm-suite` @`a7cf2533` | **28/28, 0 hỏng** (runtime=node) | `/home/ps/vislogs/st314a-lp2-wasm-suite.log`, sha `53e789db050a355b3aba9629f8412f8281acd66384267000c93497f9fe22a3ff` |
| `bin/vis-targets` @`a7cf2533` (gồm `vis-dag-check` mới) | **PASS** — core `47/0/0`, sqlite `45/2/0`; dag-check `OK — 23 crate / 5 tầng` | `/home/ps/vislogs/st314a-lp2-vis-targets.log`, sha `bd8309ddcbf5b29579394adb804b44def1f8238522cdc87bf09c40a203fb5afe` |
| `./vis-dev core-test` (no-default-features) | **PASS** — 1.804/0 (12 binary), target riêng | `/home/ps/vislogs/st314a-lp2-core-test.log`, sha `ec5ab3afdaa212d38f60712586799607c99e7c70262e05b6714966f6d8808834` |
| Windows full @`a7cf2533` (`psvn`) | **3.729/0/4 ignored** (90 binary); host conformance **54 ca/120 lượt, 0 lệch**; `vis-targets` core `47/0/0`, sqlite `46/1/0`; `vis-dag-check OK` | `/home/ps/vislogs/st314a-lp-win-full.log`, sha `31237fe79645729ac3f6af9169de6e901c9d3594a89ac29b2bbd318003e807e6` (sha log máy đích `6eb165cd…`) |
| Fuzz parser 20 phút @`psvn` | **92.966.091 ca / 2.109.956.381 token, 0 vi phạm** | `/home/ps/vislogs/st314a-lp3-fuzz-psvn.log` |
| `vis-wasm-conformance` @`da66418a` (sau leaf refactor) | **299 × 2 engine, 0 hỏng**, `EXIT=0` | `/home/ps/vislogs/st314a-lp4-wasm-conformance.log`, sha `3ad77dc556e0ec78b6088ba1564f6198c81ee187bb1e4abb61055ced6a867fdd` |

## 11. Audit docs `SPEC.md` + thiết kế (subagent) — đề xuất cho Agent 0

Audit đối chiếu cây thật tại `aaf4fa83`: **6 dòng `docs/SPEC.md`** và **16 dòng
`docs/ViScript-thiet-ke.md`** còn lệch (phiên bản CLI `0.5.0`, tên backend cũ
`vis-host-posix/-windows/-mock`, danh sách `unsafe` cũ, bảng lệnh `vis-dev`, đường dẫn
`viscript/...`, `crates/vis-cli/src/pkg.rs`). Theo `AGENTS.md` §1, `SPEC.md` và tài liệu thiết kế
**không sửa trực tiếp** ở đây; đề xuất đầy đủ (file:line + thay thế) tại
[`de-xuat-dong-bo-docs-2026-11-25.md`](de-xuat-dong-bo-docs-2026-11-25.md).

Đã xác minh tại chỗ: `docs/doc-examples-allow.txt` tồn tại; `crates/vis-cli/src/pkg.rs` không còn
(`crates/vis-pkg/src/pkg.rs`); R2 danh sách đóng **rỗng**; `vish-term` có dep `rustix`/`crossterm`
theo target. Các dòng `0.4.5` là mốc lịch sử Windows Tier 1, giữ nguyên.

## 12. Đưa DAG guard vào `check` + bổ sung README công cụ

- `vis-dev check` (và do đó `gate --fast`) nay gọi `bin/vis-dag-check` sau R1..R9 — trước đây chỉ
  `bin/vis-targets` gọi, nên lỗi kiến trúc có thể lọt qua gate thường.
- `bin/README.md` bổ sung hai công cụ bị thiếu: `vis-dag-check` và `vis-wasm-conformance`.
- Thêm `test/gate/dag-check-self-test.sh` — case âm thường trực (metadata giả: **3 cạnh cấm** +
  crate thứ 24 chưa khai tầng), gọi trong `vis-dev check`/`gate --fast`. Chạy trực tiếp:
  `ok - vis-dag-check bắt 3 cạnh cấm + crate chưa khai tầng; metadata sạch xanh`, exit 0.
- `bin/vis-dag-check` header ghi rõ nay được gọi từ cả `vis-targets` và `vis-dev check`.
- `test/fixtures/run_fixtures.sh` thêm **case 18** (TAP 1..18): tổng > 40 binary **gồm
  `vis-host-linux/tests`** phải bị R8 bắt; chạy `bash test/fixtures/run_fixtures.sh` → `ok 18`, PASS.

- **Bằng chứng tích hợp**: `./vis-dev check` @`d25f50dc` → `--- DAG 5 tầng: OK`,
  `--- DAG self-test: OK`, `vis-dev check: PASS`, `EXIT=0`
  (`/home/ps/vislogs/st314a-lp3-check-dag.log`, sha `30c648e5103c73c5cbc65df260bdaf8c042d1455033b48fdea5230a7671d858c`).
- **Windows/Git Bash**: `test/gate/dag-check-self-test.sh` @`d25f50dc` trên `psvn` → `ok …`,
  `self_test_exit=0` (log phiên `vis-remote-20261125T042248Z`).


## 13. Leaf dispatch `StdId` hoá (F3 — audit review mã)

- `fs_net/fs.rs`: `fsbytes::goi(host, StdId, …)` match `FsReadBytes`/`FsWriteBytes`;
  `fsmkdirmall::goi(host, StdId, …)` match `FsMkdir`/`FsMkdirAll`.
- `vis-core`: thêm `MetadataOp { Size, IsFile, IsDir }`; `goi_metadata(host, op, p)`; dispatch
  `vis-runtime` map `StdId::FsSize/FsIsFile/FsIsDir` → op. Không còn `match ten`/`match name`
  trong các đường này.
- Đo cục bộ: `cargo check -p vis-runtime` OK; `cargo test -p vis-runtime -p vis-core` →
  **283 + 20 + 3 passed, 0 failed**; `cargo fmt --all -- --check` OK.
- Test tích hợp đích: `host_it vd270_fsbytes` **7/7**, `vd270_mkdir_all` **7/7**, `vd266_fs` **11/11**.
- `./vis-dev core-test` sau refactor: **1.804/0** (12 binary), PASS
  (`/home/ps/vislogs/st314a-lp4-core-test.log`, sha ghi trong log `ff54f3bb672b948a0a976734ecbb1bb8adf8762723f44c6e5580655330997b55`; xem quy ước §18).
- `./vis-dev check` @working tree: **PASS**, `EXIT=0`
  (`/home/ps/vislogs/st314a-lp3-check-leaf.log`, sha `9240367d135324948d465180fe1f9e40cfca26d73cf38519bfd47f713a7c4572`).
- `gate --fast` cuối @HEAD: **PASS 3804/3804**, DAG + self-test OK — §17.

## 14. Audit ma trận host (subagent) — sửa `agent/HOST-BAO-DAM.md`

- Xác nhận **47 ô/cấu hình** = 23 crate × 2 target + `vis-host-wasm` × wasm32; `core` 47 PASS cả
  `ps`/`psvn`; `sqlite` **45+2 trên `ps`**, **46+1 trên `psvn`** (ô `vis-cli` × msvc phụ thuộc C
  toolchain của host).
- Đã sửa: bỏ **dòng Linux trùng**; `vis-host-linux` không còn `— | —` (check PASS cả 2 target,
  0 ca vì crate rỗng trên host khác); đoạn "47 ô" dẫn log hiện hành `st314a-lp2-vis-targets.log`
  (`ps`) + `st314a-lp-win-full.log` (`psvn`), log 18 ô cũ ghi rõ là **lịch sử**.
- Số Windows `3611/0/3 @f4cb136` nay ghi rõ là **chứng nhận `v0.4.5`**, kèm **đo lại
  @`a7cf2533c`: 3729/0/4**; các dòng `18/18 + 17/1` cũ đánh dấu "mốc cũ; nay 47 ô".
- `docs/SPEC.md:683` (macOS caveat `0.6.0`) vẫn là đề xuất A0-only trong
  `de-xuat-dong-bo-docs-2026-11-25.md`.

## 15. Audit DAG self-test (subagent) — vá các lỗ hổng

- **High**: self-test cũ chỉ có 2 cạnh **ngược tầng** nên không chứng minh `CAM_NGANG`. Nay thêm
  cạnh **cùng tầng** `vis-vm → vis-interp`; tự kiểm đòi đủ 3 chẩn đoán + crate chưa khai tầng,
  và thêm **positive control**: metadata sạch 23 crate phải `OK`.
- **Medium**: `CAM_NGANG` nay được kiểm khoá/đích lạ; thêm chính sách cùng tầng cho `vis-types`
  (`→ vis-bytecode/runtime/interp/vm`); sửa cascade báo sai "cùng tầng" khi đích chưa khai tầng.
- **Portability**: dò `python3.12/3.11/3` như `vis-dev`; truyền metadata qua **argv** thay
  `VIS_DAG_META` (an toàn Windows); self-test dùng `PYTHON` tương tự.
- Bằng chứng: `./test/gate/dag-check-self-test.sh` → `ok - … 3 cạnh cấm + crate chưa khai tầng;
  metadata sạch xanh`; `./bin/vis-dag-check` → `OK — 23 crate / 5 tầng, 0 cạnh cấm`.

## 16. Audit wasm conformance (subagent) — vá F1..F8

- **F1 exit code**: self-check/argparse sai ⇒ `exit 1`; `2` chỉ còn BLOCKED-ENV (build/node).
- **F2 argv trace**: `NODE_DRIVER` in `ARGV=<json>` khi `VIS_WASM_TRACE_ARGV=1`; `tu_kiem` đòi
  **đúng** `["--interp","/data/prog.vis"]` / `["--vm","/data/prog.vis"]`, bắt cả token thừa.
- **F3 engine trace**: `vis-wasm-run` in `engine=Interp|Vm` khi `VIS_WASM_TRACE_ENGINE=1`
  (env được driver chuyển vào WASI); `tu_kiem` đòi đúng engine ⇒ **chứng minh `--vm` chọn VM**,
  không chỉ "chạy được".
- **F4** marker `[USAGE-UNKNOWN]` thay phụ thuộc nguyên văn thông điệp.
- **F5** kiểm `node -e "require('node:wasi')"` trước khi chạy ⇒ BLOCKED-ENV rõ.
- **F6** bỏ ép `CARGO_TARGET_DIR` trong `bin/vis-wasm-sem` (tôn trọng env như suite).
- **F7** `tu_kiem` nằm trong `try` ⇒ không rò thư mục tạm khi self-check đỏ.
- **F8** in số ca bỏ qua (`bỏ qua 0 ca host/async`).

Bằng chứng:
| Kiểm | Kết quả |
|---|---|
| Positive `vis-wasm-conformance` | **299 × 2 engine, 0 hỏng**, `EXIT=0`, sha `4b49d823…` |
| Negative tái tạo `ro` | `exit 1`, trace `ARGV=["/data/prog.vis"]`, `engine=Interp` |
| Negative bỏ `--vm` trong driver | `exit 1`, `--vm không tới runner/không chọn Vm` |
| `vis-wasm-sem` sau F6 | **390/390, 0 fail, 100%** | `/home/ps/vislogs/st314a-lp6-wasm-sem.log`, sha `41777564ef6cf9232076571dd75db09f0fde33ea5552a987cf49596842b4d203` |
| `vis-wasm-suite` sau F6 | **28/28, 0 hỏng** | `/home/ps/vislogs/st314a-lp6-wasm-suite.log`, sha `858670cd5224ea6d87a8287426e41bdbfacb109b526355a7fd69f497d11af002` |
| Windows full @`da66418a` (`psvn`, sau leaf refactor) | **3.729/0/4 ignored** (90 binary); host **54/120, 0 lệch**; targets core `47/0/0`, sqlite `46/1/0` | `/home/ps/vislogs/st314a-lp4-win-full.log`, sha `0d7f99c9dddd23aabd0377ecc92abd78774c7f1779c007c44b33fd05746cbecb` (máy đích `d3a27fc4…`) |

## 17. Cổng cuối @HEAD (sau toàn bộ hardening)

| Cổng | Kết quả | Bằng chứng |
|---|---|---|
| `./vis-dev gate --fast` | **PASS** — 3804/3804 (4 skip); `DAG 5 tầng: OK`; `DAG self-test: OK` | `/home/ps/vislogs/st314a-lp6-gate.log`, sha `e5aa183e7c2eb48048f800daf82f558a5718629e75372c864bbbacd78b8fdc05` |
| `vis-wasm-sem` sau F6 | **390/390, 0 fail, 100%** | `/home/ps/vislogs/st314a-lp6-wasm-sem.log`, sha `41777564…` |
| `vis-wasm-suite` sau F6 | **28/28, 0 hỏng** | `/home/ps/vislogs/st314a-lp6-wasm-suite.log`, sha `858670cd…` |

## 18. Quy ước sha log

Các dòng `sha256sum` ở **cuối log** được tính trên nội dung log **trước khi** nối chính dòng đó
(vì vậy `sha256sum` lại cả tệp sẽ khác). Journal trích **trị ghi trong log**; ngoại lệ whole-file
được ghi rõ. Không có số liệu tự đặt.
