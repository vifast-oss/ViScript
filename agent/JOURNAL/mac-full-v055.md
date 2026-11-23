<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# macOS `--full` trên tag **`v0.5.5`** @ **`b4db374`**

> **Người đo**: subagent (phiên sạch), 2026-11-24.
> **Mốc**: **`b4db374ac16223ae01d184fdfd3033ba1175b435`** = tag **`v0.5.5`**
> (`[ST-310e] t3: timeout 30s (macOS flake sat 8s, test-only)`).
> Clone đo trên `mps`: **`/Users/mps/dvchd/ViScript-v050`** — detached HEAD @ `b4db374`, **dirty 0**
> trước và sau khi đo (đã ở đúng mốc, không cần fetch/checkout).
> `TMPDIR=/Users/mps/tmp` (ngoài repo), jobs **3**, `CARGO_PROFILE_DEV_DEBUG=0`,
> `CARGO_TARGET_DIR=<repo>/target/gate-macos`. Không ssh `psvn`; không soak; không merge/push/tag.
> **Kết quả: PASS** — nextest **3764 passed / 0 failed / 3 skipped**, host-wasm **29/29**,
> WASI **28/28**, sem **388/390** (99,5%), wall **237 s** (gate) + **61 s** (sem). Không blocker.

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-macos --full` | **exit 0** (`vis-remote-exit=0`, `.exit` máy đích = 0) | **237 s** (20:04:55Z → 20:08:52Z) |
| `./vis-dev check` | fmt OK · clippy OK · luật **R1..R9** OK · self-test OK · doc-examples OK (31 khối: 26 xanh + 5 ngoại lệ; 12 fixture QD) | — |
| `./vis-dev gate --fast` | **PASS** (L0 nextest + doc-test) | — |
| `cargo nextest run --workspace` | **3764 run / 3764 passed / 0 failed / 3 skipped**, 103 binary | **75,6 s** |
| `cargo test --doc` | **5 passed / 0 failed / 1 ignored** (18 binary doc-test) | — |
| host native `vis-host-wasm` | **29/29** (17 lib + 0 bin + 12 `wasi_fs`) | — |
| `bin/vis-wasm-suite --build --runtime node` | **28/28**, 0 hỏng, runtime node | — |
| `bin/vis-wasm-sem --require-target` | **388/390** PASS, 2 FAIL known-gap, coverage **99,5 %** (≥85 %) | **61 s** |
| Dirty cuối | `mps` **0** mục, HEAD không đổi; cây gọi **0** mục | — |
| Blocker | **Không** | — |

## 1. Môi trường

- `mps`: **macOS 27.0.1**, `Darwin 27.0.0 arm64`, **12 nhân**;
  `cargo 1.100.0 (3acc79c0c 2026-11-05)`, `rustc 1.100.0 (0ef1bc386 2026-11-09)`.
- Runtime wasm: **node v26.12.1** (`node:wasi`); **không** có `wasmtime` CLI lẫn Python `wasmtime`
  ⇒ `vis-wasm-sem` tự chọn `runtime=node`.
- Repo: `/Users/mps/dvchd/ViScript-v050`, `origin` = `https://github.com/vifast-oss/ViScript.git`,
  **detached HEAD** @ `b4db374`, `git status --porcelain` = **rỗng** (trước và sau).
  Tag `v0.5.5` (cục bộ tại `ps`) trỏ đúng `b4db374`.
- **Không** ssh `psvn`; **không** chạy soak.

## 2. Full gate

Lệnh đúng như yêu cầu (hai biến `REPO` đều trỏ clone `v050`, vì mặc định của công cụ là
`/Users/mps/dvchd/ViScript` — cả thân chạy **và** phép kiểm cây bẩn `--require-clean`):

```bash
VIS_GATE_MACOS_REPO=/Users/mps/dvchd/ViScript-v050 \
VIS_REMOTE_MPS_REPO=/Users/mps/dvchd/ViScript-v050 \
VIS_GATE_MACOS_LOG=/tmp/opencode/gate-macos-full-v055.log \
bin/vis-gate-macos --full
```

Thân chạy do script sinh (chạy trong `bin/vis-lock`, `pid=92028`, giữ lock **236,6 s**):

```bash
cd /Users/mps/dvchd/ViScript-v050
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0
export TMPDIR="${TMPDIR:-$HOME/tmp}"        # = /Users/mps/tmp
export CARGO_TARGET_DIR=/Users/mps/dvchd/ViScript-v050/target/gate-macos
./vis-dev check
./vis-dev gate --fast
cargo nextest run --workspace
cargo test --doc
cargo test --manifest-path vis-host-wasm/Cargo.toml
bin/vis-wasm-suite --build --runtime node
```

- **exit 0**, `vis-remote-exit=0`, `.exit` trên máy đích = **0**; wall **237 s**
  (`thoi_diem_bat_dau=20:04:55Z`, `thoi_diem_ket_thuc=20:08:52Z`).
- `vis-dev check`: `--- fmt: OK`, `--- clippy: OK`, `--- luật R1..R9: OK`, self-test âm 12/12 OK,
  `doc-examples: OK (31 khối tài liệu: 26 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3)`.
- `gate --fast`: `--- L0 test: OK`, `vis-dev gate --fast: PASS`.
- nextest: **3764 tests run / 3764 passed (1 slow) / 3 skipped**, 103 binary, **75,6 s** —
  chạy **hai lần** (L0 trong `gate --fast` 75,720 s; lệnh tường minh 75,635 s), cả hai cùng số.
- doc-test: 18 binary, **5 passed / 0 failed / 1 ignored**
  (`crates/vis-plugin-macros/src/lib.rs - (line 4) ... ignored` — có chủ đích).
- host `vis-host-wasm`: lib **17**, bin **0**, `tests/wasi_fs.rs` **12** ⇒ **29/29**, 0 failed.
- WASI suite: `# tổng: 28 ca, 0 hỏng, runtime=node` ⇒ **28/28**.
- **0 dòng `warning:`**, **0 `error`**.

### 2.1 Đối chiếu kỳ vọng

| Chỉ số | Kỳ vọng | Thực đo | Lệch |
|---|---|---|---|
| nextest | ~3760 / 3 skipped | **3764 / 3 skipped** | +4 passed (cùng 3 skip), không fail |
| host-wasm | 29/29 | **29/29** | 0 |
| WASI | 28/28 | **28/28** | 0 |
| sem | ≥388/390 | **388/390** (99,5 %) | 0 (đúng sàn) |
| dirty cuối | 0 | **0** | 0 |

## 3. WASI sem (`bin/vis-wasm-sem`)

Dùng lại đúng wasm mà gate vừa build (cùng `CARGO_TARGET_DIR`, không build lại):

```bash
cd /Users/mps/dvchd/ViScript-v050
export TMPDIR=/Users/mps/tmp
export CARGO_TARGET_DIR=/Users/mps/dvchd/ViScript-v050/target/gate-macos
python3 bin/vis-wasm-sem \
  --oracle /Users/mps/vislogs/sem-native-baseline.tsv \
  --require-target
```

- wasm: `target/gate-macos/wasm32-wasip1/release/vis-wasm-run.wasm` (3.831.442 B, 05:08 local).
- `# sem total=525 eligible=390 excluded=135 runtime=node
  oracle_sha256=482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d`
  — **khớp baseline W1** đã biết (không cảnh báo).
- `# SUMMARY total=525 eligible=390 excluded=135 pass=388 fail=2 coverage=99.5% target=85%`
  ⇒ **exit 0** (99,5 % ≥ 85 %), wall **61 s**.
- 2 FAIL **known-gap** (không phải hồi quy; VM đều xanh, chỉ interp wasm tràn bộ nhớ):

| Ca | native rc | wasm interp | wasm VM | Ghi chú |
|---|---|---|---|---|
| `test/sem/ham/de-quy-1000.vis` | 0/0 | **1** | 0 | `RuntimeError: memory access out of bounds` tại `vis_interp::interp::eval` |
| `test/sem/ham/de-quy-vo-han.vis` | 101/101 | **1** | 101 | cùng lỗi; đệ quy vô hạn làm interp wasm hết bộ nhớ thay vì trả 101 |

  Đây đúng là 2 stack-overflow interp đã ghi nhận ở các lượt Windows/Linux trước
  (`agent/JOURNAL/win-full-f9e7503.md` §2), không xuất hiện thêm ca mới.

## 4. Bằng chứng và dirty

- Log cục bộ gate: `/tmp/opencode/gate-macos-full-v055.log` — **8190 dòng**,
  `sha256=27e0fd91ffdad5ffc2ce760ce20711575d001bafdf87d0806f183e363c81d897`.
  Bản sao trên máy đích: `/Users/mps/vislogs/vis-remote-20261123T200455Z.log`,
  `sha256=45e8489ba0b1bb07195bbfd8ce6b3bd89c7d286169ff0e808ac26fb180d91f5b` (theo KET của `vis-remote`).
- Log sem: `/tmp/opencode/sem-run.out` — **533 dòng**,
  `sha256=91b58b6b4ca3429a6e7159f3042d2880678c1e192da90d532498c3efd110f16d`;
  bản máy đích `/Users/mps/vislogs/vis-remote-20261123T200904Z.log`.
- Sau đo: `mps` `HEAD=b4db374ac16223ae01d184fdfd3033ba1175b435`, `git status --porcelain` = **0**;
  cây gọi `ps` cũng sạch, HEAD `b4db374`, tag `v0.5.5` nguyên vẹn.

## 5. Blocker

**Không.** Hai điểm cần biết khi đọc số (không phải blocker):

1. nextest **3764** thay vì ~3760 — nhiều hơn 4 ca đã pass, cùng **3 skipped**, **0 failed**;
   nhiều khả năng do test thêm giữa các mốc, không phải nới lỏng.
2. sem **388/390** chứ không 390/390 — đúng 2 known-gap đệ quy sâu của interp wasm
   (mục 3), `--require-target` vẫn đạt vì coverage 99,5 %.
