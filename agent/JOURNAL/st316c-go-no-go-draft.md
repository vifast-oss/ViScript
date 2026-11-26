<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# `ST-316c` — Hồ sơ Go/No-Go (nháp) + Checklist 3-OS

- **Ngày soạn**: 2026-11-26/27 (UTC) · **Người soạn**: Builder (Agent 2), nhánh `plan/st316c`.
- **Nền**: `main` = **`3e8283930d1e2a9ae4dca4729391403a6b152576`** (`[ST-00] VD-293`); cây mã
  `3e828393` **y hệt** `882b7ffa` (`git diff --stat 882b7ffa 3e828393 -- crates test bin Cargo.toml`
  rỗng — kiểm lại trong lượt này).
- **Trạng thái task**: **`BLOCKED`** (không đổi). Tài liệu này chỉ là **đầu vào để A0 quyết định**;
  không tuyên bố `DONE`, không tuyên bố 3-OS khi chưa có log `mps`.
- **Phạm vi lượt này**: chỉ **đọc** + sửa **phần hiển thị** `bin/vis-targets`; **không** chạy
  build/test/bench/remote/fuzz/soak. Mọi số của `mps`/`psvn` dưới đây là snapshot đã có, ghi rõ
  nguồn; riêng `ps` đo lại bằng `df`/`git` trong lượt.

---

## 0. Snapshot 3 máy & điều kiện cần trước khi chạy

| Máy | HEAD | Repo / SSH | Đĩa | Trạng thái | Cần làm trước khi đo |
|---|---|---|---|---|---|
| **`ps`** (Linux x86_64) | **`3e828393`** (đo 2026-11-26T15:57Z) | `/home/ps/viswt/st316c`; chạy tại chỗ (`vis-remote ps`) | `/` còn **16 GiB** (94 %); `/tmp` tmpfs 7,2 G còn **3,3 G** | sạch (chỉ có thay đổi `bin/vis-targets` của lượt này) | `df` lại; `bin/vis-gc-targets` nếu < 10 GiB; log ra `/home/ps/vislogs/`, **không** `/tmp` |
| **`mps`** (macOS arm64) | **`311d2288`** — *lệch* | `/Users/mps/dvchd/ViScript`; SSH **chỉ** `mps@mps` (`ssh mps` bị tailnet từ chối) | `/` còn **37 GiB** (snapshot lane plan 2026-11-27) | sạch (0 tệp bẩn, snapshot) | fetch `origin/main` → checkout **`3e828393`**; kiểm `bin/vis-lock` rảnh; chạy clippy + full gate |
| **`psvn`** (Windows x86_64, Git Bash) | **`fe3446d2`** — *lệch* | `/c/Users/ps/viscript-moi`; SSH `psvn` (mặc định) | `C:` còn **14 GiB** (98 %, snapshot) | sạch (snapshot) | fetch → checkout **`3e828393`**; dọn `target/` nếu cần; chạy `--full` + build release cho bench |

**Ghi chú nguồn**: trạng thái `mps`/`psvn` lấy từ `/home/ps/vislogs/song6-lane-plan.md` §0
(đo 2026-11-27, subagent READ-ONLY) — **không** ssh lại trong lượt này theo lệnh. `ps` đo trực tiếp
bằng `git rev-parse HEAD`, `git status --porcelain`, `df -h` lúc **2026-11-26T15:57Z**.

### 0.1 Chặn cứng hiện tại (theo thứ tự)

1. `mps` @ `311d2288` ⇒ `bin/vis-gate-macos` sẽ **exit 4** ở HEAD guard. Phải fetch/checkout
   `3e828393` **trước**.
2. `psvn` @ `fe3446d2` ⇒ `bin/vis-gate-windows` sẽ **exit 6** ở HEAD guard. Phải fetch/checkout
   `3e828393` **trước**. `C:` 14 GiB là đủ cho test nhưng **không** dư cho build release + test
   cùng lúc; theo dõi đĩa, đỏ vì đĩa ⇒ ghi `BLOCKED-ENV`, **không** mở task mã.
3. `ps` từng xuống **8,7 GiB** trong lượt soạn (đã lên lại 16 GiB khi đo cuối). Trước
   `bin/vis-targets --wasm`/`cargo build --release`, kiểm `df` và chạy `bin/vis-gc-targets`
   (dry-run trước, `--apply` chỉ khi cần).
4. `mps` **không** có target `wasm32-unknown-unknown` (chỉ `wasm32-wasip1`) ⇒ **không** chạy
   `bin/vis-targets` đầy đủ trên `mps`; gate macOS chỉ cần WASI. Chạy `vis-targets` trên `ps`/`psvn`.
5. `bin/vis-bench` cần **Python 3** trên từng máy; chưa có bằng chứng đã chạy bench trên `mps`/
   `psvn` (xem §5). Kiểm `python3 --version` trước; thiếu Python ⇒ ghi `BLOCKED-ENV` cho bench,
   **không** thay bằng số ước lượng.

### 0.2 Xác minh mốc WASI `379×2` vs `393×2`

Cả hai số đều **có bằng chứng thật**, ở hai thời điểm khác nhau — không phải mâu thuẫn cùng ngày:

| Mốc | Số | Thời điểm đo | Bằng chứng |
|---|---|---|---|
| Cũ | **379×2** = 354 `.out` + 25 `.trap` | 2026-11-25 → r054 (2026-11-25T21:29Z) | `lowpeak-chain-2026-11-25/r006..r054-*.log`; `st314a-lp33-wasm-conformance-trap.log`; `st314a-lp37-targets-wasm.log` |
| Mới | **393×2** = 366 `.out` + 27 `.trap` | **2026-11-26T00:55:01Z → 00:56:50Z** (r072; r060/r066 trước đó) | `lowpeak-chain-2026-11-25/r072-targets.log:112-114`, sha256 **`484809535024051f1906754b33094d052d0b25be34a5b5107ab1c31fffda7486`**; bản sao `st314a-lp53-targets-final.log:112-114`; `chain.log:336-337` ghi `START r072-targets` → `END … rc=0` |
| Sem/suite mới | **390/390**, **28/28** | cùng r072 | `r072-targets.log:112-113` |

Nguyên nhân lệch: `fbb0ae4f` (`[ST-314a] test: +18 ca conformance… 2 trap`, 2026-11-26 07:22 +0900)
thêm ca sau mốc 379; các ca async/kenh không chạy WASI nên `.out` tăng 366−354 = 12, `.trap` tăng
27−25 = 2. `bin/vis-targets` đã cập nhật **ngày 2026-11-26 + số 393×2 + nguồn log**; logic đo
`--wasm` **không đổi**.

---

## 1. Pre-flight chung (chạy một lần, trước mọi phép đo)

```bash
cd /home/ps/viswt/st316c
BASE=3e8283930d1e2a9ae4dca4729391403a6b152576

# 1) Đĩa + tiến trình nặng đang chạy
df -h /home/ps /tmp
uptime
ps -eo pid,etime,cmd | grep -E 'cargo|rustc|vis-gate|vis-diff|fuzz' | grep -v grep || true
bin/vis-gc-targets            # dry-run; chỉ --apply nếu free < 10 GiB và ứng viên an toàn

# 2) Xác nhận HEAD/status cây gọi
git rev-parse HEAD            # phải = $BASE
git status --porcelain        # phải rỗng (trừ thay đổi tài liệu/script đã commit)

# 3) Khoá máy đích (mps dùng vis-lock trong gate; kiểm rảnh trước)
bin/vis-remote mps --check  | sed -n '1,30p'   # chỉ đọc; không chạy test
bin/vis-remote psvn --check | sed -n '1,30p'
```

### 1.1 Đồng bộ `mps` lên đúng nền

```bash
bin/vis-remote mps --require-clean \
  --log /home/ps/vislogs/st316c-mps-sync.log -- bash -lc "
    set -euo pipefail
    cd /Users/mps/dvchd/ViScript
    git fetch origin main
    git switch --detach $BASE
    git rev-parse HEAD
    git status --porcelain
  "
sha256sum /home/ps/vislogs/st316c-mps-sync.log
```

> `vis-remote` mặc định `MPS_SSH=mps@mps` (`bin/vis-remote:91`); **không** gọi `ssh mps` trần.
> `--require-clean` từ chối nếu cây `mps` bẩn (kể cả tệp chưa commit). Nếu repo đổi chỗ, đặt
> `VIS_REMOTE_MPS_REPO` / `VIS_GATE_MACOS_REPO` thay vì sửa script. Git < 2.23: thay
> `git switch --detach` bằng `git checkout --detach`.

### 1.2 Đồng bộ `psvn` lên đúng nền

```bash
bin/vis-remote psvn --require-clean \
  --log /home/ps/vislogs/st316c-psvn-sync.log -- bash -lc "
    set -euo pipefail
    cd /c/Users/ps/viscript-moi
    git fetch origin main
    git switch --detach $BASE
    git rev-parse HEAD
    git status --porcelain
    df -h /c
  "
sha256sum /home/ps/vislogs/st316c-psvn-sync.log
```

> Repo mặc định `/c/Users/ps/viscript-moi` (`bin/vis-remote:99`, `bin/vis-gate-windows:34`);
> đổi bằng `VIS_REMOTE_PSVN_REPO` / `VIS_GATE_WINDOWS_REPO`. `psvn` dùng Git Bash
> `C:\Program Files\Git\bin\bash.exe` (`bin/vis-remote:93`).

### 1.3 Mẫu ghi bằng chứng (mọi log)

Mỗi log phải mở đầu bằng **HEAD + status + OS/arch + rustc/cargo + UTC**. `vis-remote` đã tự ghi
header VD-136; với lệnh chạy trực tiếp trên `ps` thì thêm tay:

```bash
LOG=/home/ps/vislogs/st316c-<ten>.log
{ echo "# HEAD=$(git rev-parse HEAD)"; echo "# dirty=$(git status --porcelain | wc -l)";
  date -u; uname -srm; rustc -V; cargo -V; } | tee "$LOG"
# ... lệnh đo ... 2>&1 | tee -a "$LOG"
echo "# exit=${PIPESTATUS[0]}" | tee -a "$LOG"
sha256sum "$LOG"
```

---

## 2. Checklist lệnh chính xác

### 2.1 macOS — clippy độc lập (bắt buộc có log riêng)

`bin/vis-gate-macos --full` có gọi `./vis-dev check` (trong đó có clippy), nhưng KPI `ST-316c`
cần **một log clippy riêng** ở đúng HEAD. Chạy trước gate:

```bash
cd /home/ps/viswt/st316c
bin/vis-remote mps --require-clean \
  --log /home/ps/vislogs/st316c-macos-clippy.log -- bash -lc '
    set -euo pipefail
    cd /Users/mps/dvchd/ViScript
    export PATH="$HOME/.cargo/bin:$PATH"
    export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0
    export TMPDIR="${TMPDIR:-$HOME/tmp}"; mkdir -p "$TMPDIR"
    export CARGO_TARGET_DIR="$PWD/target/gate-macos"
    git rev-parse HEAD; git status --porcelain
    cargo clippy --workspace --all-targets -- -D warnings
  '
sha256sum /home/ps/vislogs/st316c-macos-clippy.log
```

**PASS**: exit 0, không dòng `warning:`/`error:`; log có `HEAD` đúng `$BASE`; `status` rỗng.
Dùng `CARGO_TARGET_DIR=target/gate-macos` để gate sau tái dùng cache (thư mục này bị gitignore,
không phá `--require-clean`).

### 2.2 macOS — full gate

```bash
cd /home/ps/viswt/st316c
VIS_GATE_MACOS_LOG=/home/ps/vislogs/st316c-macos-full.log \
  bin/vis-gate-macos --full
sha256sum /home/ps/vislogs/st316c-macos-full.log
grep -nE '^--- (fmt|clippy|luật|DAG|toolchain|rules):|Summary \[|gate --fast:|vis-wasm-suite|FAIL' \
  /home/ps/vislogs/st316c-macos-full.log | tail -30
```

Luồng thật của `--full` (`bin/vis-gate-macos:84-95`): `./vis-dev check` → `./vis-dev gate --fast`
→ `cargo nextest run --workspace` → `cargo test --doc` → `cargo test --manifest-path
vis-host-wasm/Cargo.toml` → `bin/vis-wasm-suite --build --runtime node`. Toàn bộ bọc trong
`bin/vis-lock --timeout 7200` trên `mps`; HEAD guard ở `:115` so **full 40 ký tự** với HEAD cây gọi,
lệch ⇒ **exit 4**.

**PASS**: exit 0 + `--- clippy: OK` + `gate --fast: PASS` + nextest `0 failed` + doc-test 0 failed +
WASI suite 0 hỏng. **Không** chấp nhận “exit 0 nhưng thiếu summary”.

### 2.3 Windows — full gate

```bash
cd /home/ps/viswt/st316c
VIS_GATE_WINDOWS_LOG=/home/ps/vislogs/st316c-windows-full.log \
  bin/vis-gate-windows --full
sha256sum /home/ps/vislogs/st316c-windows-full.log
grep -nE 'HEAD khớp|test result:|Summary \[|FAIL|error\[|warning:|PASS=[0-9]+' \
  /home/ps/vislogs/st316c-windows-full.log | tail -40
```

`--full` chạy trên `psvn` (`bin/vis-gate-windows:46`):
`cargo clippy --workspace --all-targets -- -D warnings` → `cargo test --workspace --no-fail-fast`
→ `cargo test -p vis-cli --test host_conformance -- --nocapture` → `bash bin/vis-targets`.

**PASS**: exit 0; clippy không warning; **0 failed**; `vis-targets` core `FAIL=0`, sqlite `FAIL=0`.
HEAD guard (`:57-72`) đọc `head_full` qua `vis-remote --check`; lệch ⇒ **exit 6**; chỉ dùng
`VIS_GATE_WINDOWS_ALLOW_HEAD_MISMATCH=1` khi **cố ý** đo commit khác (không dùng cho nghiệm thu).

> **Đĩa**: `--full` không đặt `CARGO_PROFILE_DEV_DEBUG=0`; với `C:` 14 GiB, theo dõi `df -h /c`
> trong lúc chạy. Nếu đỏ vì hết đĩa ⇒ `BLOCKED-ENV`, dọn `target/` rồi chạy lại; **không** sửa
> script giữa phép đo.

### 2.4 WASI — `vis-targets --wasm` (chạy trên `ps`)

```bash
cd /home/ps/viswt/st316c
LOG=/home/ps/vislogs/st316c-vis-targets-wasm.log
{ echo "# HEAD=$(git rev-parse HEAD)"; echo "# dirty=$(git status --porcelain | wc -l)";
  date -u; } | tee "$LOG"
CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 \
  ./bin/vis-targets --wasm 2>&1 | tee -a "$LOG"
rc=${PIPESTATUS[0]}; echo "# exit=$rc" | tee -a "$LOG"
sha256sum "$LOG"
```

**PASS**: exit 0; có **đủ 3 summary** (thiếu summary = FAIL, `bin/vis-targets:183-190`):
`sem total=… pass=390 fail=0`, `suite 28 ca, 0 hỏng`, `conformance 393 ca x 2 engine
(366 .out + 27 .trap), 0 hỏng`. Nếu corpus đổi trong Sóng 6, **ghi số thật**; điều kiện cứng là
`fail=0`/`0 hỏng`, không phải đúng 393. Ma trận core/sqlite phải `FAIL=0` (trên `ps`, nền này:
core 47 PASS; sqlite 45 PASS + 2 EXPECTED_RED; trên `psvn` có thể khác cấu hình — chỉ cần `FAIL=0`).

> `--wasm` chạy **cả** ma trận `cargo check` chéo trước, rồi mới đo WASI; đây là lệnh nặng.
> Kiểm đĩa trước; không chạy trên `mps` (thiếu `wasm32-unknown-unknown`).

### 2.5 `vis-bench` 3-OS — build release + đo

`bin/vis-bench` đo **binary release trực tiếp** (`vis run --time --no-cache`), warmup 1, mặc định
5 mẫu, mỗi chương trình phải exit 0 + stdout **giống hệt** giữa interp và VM. `p90` là
nearest-rank `ceil(0.9*N)`; với `N=5`, **p90 = mẫu lớn nhất**.

#### (a) `ps` (Linux x86_64)

```bash
cd /home/ps/viswt/st316c
df -h /home/ps
CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo build --release -p vis-cli --bin vis
sha256sum target/release/vis
./bin/vis-bench --bin target/release/vis --runs 5 --json \
  > /home/ps/vislogs/song6-baseline-ps.json
echo "exit=$?"; sha256sum /home/ps/vislogs/song6-baseline-ps.json
```

#### (b) `mps` (macOS arm64)

```bash
cd /home/ps/viswt/st316c
# build
bin/vis-remote mps --require-clean --log /home/ps/vislogs/st316c-bench-mps-build.log -- bash -lc '
  set -euo pipefail
  cd /Users/mps/dvchd/ViScript
  export PATH="$HOME/.cargo/bin:$PATH"
  export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0
  export TMPDIR="${TMPDIR:-$HOME/tmp}"; mkdir -p "$TMPDIR"
  cargo build --release -p vis-cli --bin vis
  shasum -a 256 target/release/vis
  python3 --version
'
# đo, ghi JSON NGOÀI repo trên mps
bin/vis-remote mps --require-clean --log /home/ps/vislogs/st316c-bench-mps.log -- bash -lc '
  set -euo pipefail
  cd /Users/mps/dvchd/ViScript
  python3 bin/vis-bench --bin target/release/vis --runs 5 --json \
    > "$HOME/vislogs/song6-baseline-mps.json"
  shasum -a 256 "$HOME/vislogs/song6-baseline-mps.json"
'
# thu về ps (đường dẫn macOS; không dùng scp nếu chưa kiểm)
ssh mps@mps 'cat /Users/mps/vislogs/song6-baseline-mps.json' \
  > /home/ps/vislogs/song6-baseline-mps.json
sha256sum /home/ps/vislogs/song6-baseline-mps.json   # phải khớp shasum ở trên
```

#### (c) `psvn` (Windows x64)

```bash
cd /home/ps/viswt/st316c
bin/vis-remote psvn --require-clean --log /home/ps/vislogs/st316c-bench-psvn-build.log -- bash -lc '
  set -euo pipefail
  cd /c/Users/ps/viscript-moi
  export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0
  export TMPDIR=/c/Users/ps/tmp TMP=/c/Users/ps/tmp TEMP=/c/Users/ps/tmp
  cargo build --release -p vis-cli --bin vis
  sha256sum target/release/vis.exe
  (python3 --version || py -3 --version)
'
bin/vis-remote psvn --require-clean --log /home/ps/vislogs/st316c-bench-psvn.log -- bash -lc '
  set -euo pipefail
  cd /c/Users/ps/viscript-moi
  python3 bin/vis-bench --bin target/release/vis.exe --runs 5 --json \
    > /c/Users/ps/vislogs/song6-baseline-psvn.json
  sha256sum /c/Users/ps/vislogs/song6-baseline-psvn.json
'
# thu về: dùng scp/ssh tương ứng; KHÔNG để lẫn header vis-remote vào JSON
```

> Nếu `psvn` không có `python3` mà có `py -3`, thay lệnh đầu bằng `py -3 bin/vis-bench …` và ghi
> rõ interpreter trong journal. Không có Python ⇒ bench Windows = `BLOCKED-ENV`, không bịa số.

#### (d) JSON phải giữ (schema_version = 1)

```json
{
  "schema_version": 1,
  "tool": "vis-bench",
  "generated_at": "YYYY-MM-DDTHH:MM:SSZ",
  "runs": 5,
  "warmup": 1,
  "host": {"system": "...", "machine": "...", "release": "...", "python": "...", "cpu_count": 0},
  "binary": {"path": "...", "sha256": "...", "version": "vis 0.6.0"},
  "programs": [
    {
      "name": "opt-bench", "path": "test/bench/opt-bench.vis",
      "sha256": "<sha256 chương trình>", "stdout": "...", "stdout_sha256": "...",
      "engines": {
        "interp": {"samples_ms": [], "median_ms": 0.0, "p90_ms": 0.0, "min_ms": 0.0, "max_ms": 0.0,
                   "wall_samples_ms": [], "wall_median_ms": 0.0, "wall_p90_ms": 0.0,
                   "warmup_runtime_ms": 0.0, "warmup_wall_ms": 0.0},
        "vm": {"...": "cùng khoá"}
      }
    }
  ]
}
```

Bắt buộc lưu: **`binary.sha256`**, **`programs[].sha256`**, **`stdout_sha256`**, `host`, `runs`,
`warmup`, `generated_at`. So sánh chỉ có nghĩa khi **cùng máy/OS + cùng `runs` + cùng chương trình
sha256**; đổi chương trình ⇒ baseline mới.

---

## 3. Ma trận quyết định — Go Sóng 6 vs mở lại `ST-309`

> `VD-293` §5: `ST-309` **vẫn `BLOCKED` vô thời hạn** trong chế độ `VD-289`; hồ sơ này **không**
> tự mở `ST-309`. Mọi ngưỡng dưới đây là **điều kiện cần**; quyền quyết định vẫn là A0/người dùng.

### 3.1 Tiêu chí định lượng

| # | Tiêu chí | Ngưỡng PASS | Bằng chứng tối thiểu |
|---|---|---|---|
| G1 | Linux gate | `./vis-dev gate --fast` PASS, **0 failed** (nền `3e828393`: 3.880/3.880, 4 skipped); `VIS_DIFF_N=100000 ./bin/vis-diff` **`lech=0`** nếu chạm compiler/runtime/VM | log + HEAD + sha256 |
| G2 | macOS clippy | `cargo clippy --workspace --all-targets -- -D warnings` **exit 0**, 0 warning, @`3e828393` | `st316c-macos-clippy.log` + sha256 |
| G3 | macOS full | `bin/vis-gate-macos --full` **exit 0**, đủ `vis-dev check` + `gate --fast` + nextest workspace + doc + WASI suite | `st316c-macos-full.log` + sha256 |
| G4 | Windows full | `bin/vis-gate-windows --full` **exit 0**, clippy 0 warning, test **0 failed**, `vis-targets` `FAIL=0` | `st316c-windows-full.log` + sha256 |
| G5 | WASI | `bin/vis-targets --wasm` **exit 0**, đủ 3 summary: sem **390/390**, suite **28/28**, conformance **393×2 (366+27), 0 hỏng** — hoặc số hiện hành nếu corpus đổi, miễn `fail=0` | `st316c-vis-targets-wasm.log` + sha256 |
| G6 | Bench 3-OS | Mỗi chương trình/engine: **median hồi quy ≤ 10 %**, **p90 ≤ 15 %** so baseline **cùng máy/OS**; stdout interp = VM (công cụ tự chặn); không có mẫu exit ≠ 0 | 3 JSON baseline + 3 JSON sau + `binary.sha256` mỗi OS |
| G7 | R8/R1/R2 | R8 tổng **≤ 40** (hiện 39/40), `crates/vis-cli/tests` **≤ 15** (hiện 8); R1 max **≤ 1.500** dòng (hiện 1.189); R2 **0 `unsafe`** | `vis-dev check` log / `bin/vis-dev rules` |
| G8 | Nợ & quyền | `#200`/`#201` có trạng thái rõ trong `SO-NO`; **mở lại `ST-309` cần người dùng cho phép** | `agent/SO-NO.md` + quyết định A0 |

**Công thức hồi quy** (dùng `median_ms`, đơn vị ms, làm tròn 3 chữ số):

```
regression_% = (median_after - median_baseline) / median_baseline * 100
```

- `median_baseline < 5 ms` (writebench ~4 ms): dùng **dung sai tuyệt đối ±1 ms** thay vì %,
  vì đồng hồ `# time:` thô và nhiễu máy.
- So sánh **cùng engine, cùng chương trình, cùng OS**; không so `ps` với `mps`/`psvn`.
- Ghi kèm `uptime`/load average trước–sau; nếu load trung bình > `nproc/2`, đo lại (không kết luận
  hồi quy từ máy đang bận).

### 3.2 Bảng quyết định

| Tình huống | Go Sóng 6 | Mở lại `ST-309` | Hành động |
|---|---|---|---|
| **A. Đủ G1–G7**, bench không hồi quy, `#200` **còn mở** | **GO** (tiếp 317a/b/c; giữ `ST-309` BLOCKED) | **NO-GO** | vá `#200` bằng lát VS3 sau 317b; **chưa** tag |
| **B. Đủ G1–G7**, bench không hồi quy, `#200` **đã đóng/waiver rõ** | **GO** | **GO có điều kiện** — chỉ khi người dùng cho phép | trước khi đóng băng `H`: clippy 3-OS + **smoke soak 1 chu kỳ 3-OS** + tách `[ST-00]` docs khỏi `H_code` (`AGENTS.md` §6.2) |
| **C. macOS/WASM/bench thiếu log, Linux xanh** | **GO hẹp (Linux-only)**; **không** tuyên bố 3-OS | **NO-GO** | tiếp Sóng 6 trên Linux; `ST-316c` giữ `BLOCKED` phần `mps`; ghi `STATE` “3-OS chưa hợp lệ” |
| **D. Bất kỳ G1–G4 đỏ, WASI `fail>0`, bench hồi quy > 10 % không giải thích, R8/R1/R2 vi phạm** | **NO-GO (dừng Sóng 6)** | **NO-GO** | dừng merge; mở nợ hồi quy; sửa trên nhánh; chạy lại đủ cổng |
| **E. `mps` bận/lệch > 3 ngày** | **GO hẹp** (Linux) | **NO-GO** | giữ `ST-309` BLOCKED; ghi rõ 3-OS chưa hợp lệ; không hạ chuẩn bằng chứng |

**Bằng chứng tối thiểu theo từng nhánh**:
- **Go Sóng 6**: G1 + G5 + G7 + (G2/G3 nếu muốn nói “macOS xanh”) + G6 baseline/after; tối thiểu
  có **một** log Linux gate + **một** log WASI + JSON bench `ps`; các máy khác là điều kiện để
  **tuyên bố 3-OS**, không phải để bắt đầu Sóng 6.
- **Mở lại `ST-309`**: **đủ G1–G8** + quyết định người dùng + `#200` đóng/waiver + kế hoạch
  Pre-Freeze (`H_code` tách `[ST-00]`; smoke soak 1 chu kỳ 3-OS; clippy 3-OS).
- **No-Go**: log đỏ + lệnh tái lập + HEAD + `sha256`; không dùng “máy bận” thay cho log đỏ.

### 3.3 Thứ tự chạy đề xuất (tối thiểu thời gian máy bận)

1. `ps`: pre-flight + **baseline bench** (`song6-baseline-ps.json`) **trước** khi 317a merge.
   Nếu 317a đã merge trước khi kịp đo, dùng `st317a-bench-before.json` (xem §4) làm baseline thay
   thế và ghi rõ ngoại lệ.
2. `mps`: sync → clippy → full gate (nặng nhất, chạy khi `vis-lock` rảnh).
3. `psvn`: sync → full gate (theo dõi `C:`); build release + bench sau khi test xong (dọn `target/`
   nếu đĩa < 5 GiB).
4. `ps`: `vis-targets --wasm` (sau khi `bin/vis-gc-targets` đủ đĩa).
5. `mps`/`psvn`: build release + bench; thu JSON + sha256.
6. Điền bảng §3.1; A0 quyết định theo §3.2.

---

## 4. Baseline `vis-bench` tạm thời trên `ps` (chưa chạy lại trong lượt này)

Có sẵn **baseline hợp lệ về mặt mã** tại `882b7ffa` (= `3e828393` phần mã):

- Tệp: `/home/ps/vislogs/st317a-bench-before.json`
- `generated_at`: **2026-11-26T15:35:05Z**; `runs=5`, `warmup=1`; host Linux x86_64, 12 CPU,
  Python 3.14.4; binary `/home/ps/viswt/st317a/target/release/vis`, sha256
  **`77e64bc091f358f02464c17a472613aefd88bb080cb939fa74390a577d26b96b`**, `vis 0.6.0`.
- Reflog `plan/st317a` tại thời điểm đo chỉ có `882b7ffa` (reset 00:30, build 00:34, JSON 00:35);
  `git diff --stat 882b7ffa 3e828393 -- crates test bin Cargo.toml` **rỗng**.

| Chương trình | interp median / p90 (ms) | VM median / p90 (ms) |
|---|---:|---:|
| `opt-bench` | 50 / 52 | 40 / 42 |
| `trim-matches` | 33 / 36 | 29 / 29 |
| `mapbench` | 47 / 53 | 46 / 57 |
| `listbench` | 29 / 30 | 20 / 24 |
| `writebench` | 4 / 5 | 3 / 3 |

> **Trạng thái**: `provisional` — đo trên worktree `st317a` ở `882b7ffa`, **chưa** chạy lại trên
> `3e828393`; dùng được để so hồi quy nếu A0 chấp nhận “cùng mã”, nhưng **phải** ghi rõ nguồn và
> ưu tiên đo lại trên `3e828393` trước khi merge 317a. Không dùng
> `st316a-bench-final.json` làm baseline: đo giữa nhánh 316a, `opt-bench` 68 ms do tải khác.

---

## 5. Soát script gate — giả định, chỗ hỏng khi HEAD lệch, lỗi đã sửa

### 5.1 `bin/vis-targets`

| Vị trí | Giả định / hành vi | Rủi ro & cách xử lý |
|---|---|---|
| `:197-204` | Mốc “lần cuối” in khi **không** có `--wasm` | **Đã sửa** trong lượt: 379×2 (25/11) → **393×2 (366+27), 2026-11-26**, nguồn `lowpeak-chain-2026-11-25/r072-targets.log:112-114` (`chain.log` 00:55:01Z→00:56:50Z, rc=0); thêm dòng `nguồn-wasm`. Không đổi logic đo. |
| `:167-191` | `--wasm` đo thật; thiếu summary ⇒ FAIL | Đúng; không được “vá” số hiển thị để khớp log đỏ. |
| `:45-51` | Ma trận cần **cả** `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown` | **Không chạy trên `mps`** (thiếu `wasm32-unknown-unknown`); chạy `ps`/`psvn`. |
| `:212` | Câu kết “chưa chạy trên Windows/wasm” | Chỉ là nhãn mức bằng chứng; khi chạy trên `psvn` câu này vẫn in — **không** dùng làm bằng chứng runtime Windows. Không sửa (ngoài phạm vi hiển thị “lần cuối”). |

### 5.2 `bin/vis-gate-macos`

| Vị trí | Giả định | Hỏng khi HEAD lệch / cách xử lý |
|---|---|---|
| `:34` | `REMOTE=mps` | `vis-remote:91` đổi thành `mps@mps`; **không** dùng `ssh mps` trần (tailnet từ chối user `ps`). |
| `:37` | `REPO=/Users/mps/dvchd/ViScript` | Repo đổi chỗ ⇒ `VIS_GATE_MACOS_REPO`; nếu sai, probe `head_full: ?` ⇒ `vis-remote` exit 3 (đúng). |
| `:115` | HEAD guard **full 40 ký tự** | Lệch ⇒ **exit 4** trước khi chạy test; đây là hành vi đúng, không phải lỗi. |
| `:107` | `CARGO_TARGET_DIR=<repo>/target/gate-macos` | Nằm trong repo nhưng gitignore; `--require-clean` không chặn. Nếu `target/` đầy ⇒ dọn trước. |
| `:117` | `bin/vis-lock` trên `mps` | Lock bận ⇒ chờ tới 7200 s; kiểm `vis-lock` trước khi hẹn giờ. |
| `:82` | `nextest --no-tests=fail` cho `lat` | Đúng: 0 ca = đỏ, tránh PASS giả. |
| `:94` | `vis-wasm-suite --build --runtime node` trên `mps` | Cần `node` + `wasm32-wasip1` (snapshot có); thiếu ⇒ exit ≠ 0, ghi `BLOCKED-ENV`, không bỏ bước. |

### 5.3 `bin/vis-gate-windows`

| Vị trí | Giả định | Hỏng khi HEAD lệch / cách xử lý |
|---|---|---|
| `:34` | `DEST=/c/Users/ps/viscript-moi` | Repo đổi chỗ ⇒ `VIS_GATE_WINDOWS_REPO`; sai path ⇒ `vis-remote` exit 3. |
| `:36` | Log mặc định `/tmp/...` | `/tmp` là tmpfs 3,3 G trên `ps`; đặt `VIS_GATE_WINDOWS_LOG=/home/ps/vislogs/...` để log bền, ngoài repo. |
| `:57-72` | HEAD guard qua `vis-remote --check` | Lệch ⇒ **exit 6**; đúng. `ALLOW_HEAD_MISMATCH=1` chỉ để chẩn đoán, **không** dùng nghiệm thu. |
| `:46` | `--full` không đặt `CARGO_PROFILE_DEV_DEBUG=0` | `C:` 14 GiB: theo dõi đĩa; nếu đỏ vì đĩa, `BLOCKED-ENV` + dọn `target/`, không sửa script giữa phép đo. |
| `:44` | Quick guard đòi `test result: ok. N passed` | Đúng, chống PASS giả khi filter 0 ca. |
| `:46` | `bash bin/vis-targets` trên Windows | In snapshot WASI (đã sửa); **không** chạy `--wasm` trên Windows. |

### 5.4 `bin/vis-remote`

| Vị trí | Giả định | Ghi chú |
|---|---|---|
| `:91` | `MPS_SSH=mps@mps` | Bắt buộc; `ssh mps` trần fail tailnet. |
| `:92-93` | `PSVN_SSH=psvn`; Git Bash `C:\Program Files\Git\bin\bash.exe` | Git cài chỗ khác ⇒ `VIS_REMOTE_PSVN_BASH`. |
| `:98-99` | Repo `mps`/`psvn` | Override `VIS_REMOTE_*_REPO`. |
| `:260-290` | `psvn` mặc định kiểm cây bẩn; `mps` chỉ khi `--require-clean` | Gate dùng `--require-clean` cho cả hai. |
| `:292-297` | Ghi `.exit` + đọc lại từ tệp | Không lấy `$?` của PowerShell; đúng. |
| `:346-347` | Probe phải trả `head_full` hợp lệ | Sai repo/ssh hỏng ⇒ exit 3 (`BLOCKED-ENV`). |
| `:410` | `sha256(log trên máy đích)` | Lưu cùng log cục bộ để đối chiếu. |
| `~/.ssh/config` (đọc trong lượt) | `Host psvn` → `User ps`, `IdentityFile ~/.ssh/psvn_ed25519`; **không** có alias `mps` | `ssh mps` trần dùng user `ps` ⇒ tailnet từ chối; luôn `mps@mps`. Nếu đổi user `psvn`, đặt `VIS_REMOTE_PSVN_SSH=ps@psvn` (mặc định `psvn` đang đúng). |

### 5.5 `bin/vis-bench`

| Vị trí | Giả định | Ghi chú |
|---|---|---|
| `:71-82` | `--bin` > `$VIS_BENCH_BIN` > `target/release/vis` | Windows phải là `target/release/vis.exe`; binary phải `+x` (macOS/Linux). |
| `:110-143` | `vis run --time --no-cache`; đọc `# time:` từ stderr | Thiếu dòng `# time:` ⇒ exit 1; không tự bịa. |
| `:146-158` | p90 nearest-rank; N=5 ⇒ p90 = max | Ghi rõ khi so p90; đừng coi p90 là “ổn định”. |
| `:206-210` | interp và VM phải in **giống hệt** | Lệch ⇒ exit 1; đây là cổng đúng, không nới. |
| `:347-366` | JSON có `binary.sha256` + `programs[].sha256` | Bắt buộc lưu; đổi chương trình ⇒ baseline mới. |
| — | Cần Python 3 trên từng máy | Chưa có bằng chứng bench trên `mps`/`psvn`; kiểm trước, thiếu ⇒ `BLOCKED-ENV`. |

### 5.6 Kiểm tra đã chạy trong lượt (không build, không ssh)

```text
bash -n bin/vis-targets                         -> OK
bash -n bin/vis-gate-windows                    -> OK
python3 compile(bin/vis-gate-macos)             -> OK
./bin/vis-targets --help                        -> exit 0
./bin/vis-gate-macos --full --dry-run           -> exit 0 (in đúng HEAD guard $BASE, REPO, lock, 6 bước)
./bin/vis-gate-macos --self-test                -> OK (13 kiểm)
./bin/vis-gate-windows --full --dry-run         -> exit 0
./bin/vis-gate-windows --quick --dry-run        -> exit 0
./bin/vis-remote --hosts                        -> ps/mps/psvn đúng repo/cargo/dirty policy
./bin/vis-remote --self-test                    -> 5 ok, 0 fail
```

Log phiên (ngoài repo): `/home/ps/vislogs/st316c-prep-20261126T1557Z.log`,
sha256 **`ba0ec3d0b000ce0f292ccc53b83576c212f74b9d7c189944ff4d2404490f623e`** — gồm HEAD/`df`,
sha256 các log nguồn `379×2`/`393×2`, và kết quả các kiểm script ở trên.

**Không có lỗi hard-code rõ ràng** trong 3 script gate; các đường dẫn/user đều có biến môi trường
override và đã khớp snapshot. **Không sửa** logic gate. Thay đổi duy nhất: hiển thị “lần cuối”
của `bin/vis-targets` (mục 1).

---

## 6. Rủi ro đã biết & cách né

| # | Rủi ro | Tác động | Cách né / bằng chứng |
|---|---|---|---|
| R1 | `mps` lệch HEAD `311d2288` | `vis-gate-macos` exit 4; clippy cũ chỉ @`311d2288`, không dùng cho `3e828393` | sync §1.1; log clippy mới; ghi HEAD trong log |
| R2 | `psvn` lệch `fe3446d2`, `C:` 14 GiB | gate exit 6; full test/build release có thể đầy đĩa | sync §1.2; `df` trước/sau; dọn `target/`; đỏ đĩa = `BLOCKED-ENV` |
| R3 | `ps` từng còn 8,7 GiB (< 10 GB), `/tmp` tmpfs 3,3 G | `vis-targets --wasm`/release build có thể fail giữa chừng | `bin/vis-gc-targets`; log ra `/home/ps/vislogs`; `TMPDIR` ngoài `/tmp` nếu cần |
| R4 | Số `vis-targets` cũ 379×2 | Báo cáo sai mốc “lần cuối” | **đã sửa** 393×2 + nguồn log; `--wasm` vẫn là nguồn duy nhất khi đo thật |
| R5 | corpus đổi trong Sóng 6 | 393×2 không còn đúng | điều kiện là `fail=0` + đủ summary; ghi số thật, không ép 393 |
| R6 | `bin/vis-bench` cần Python trên `mps`/`psvn`; chưa từng chạy remote | bench 3-OS có thể `BLOCKED-ENV` | kiểm `python3`/`py -3`; ghi interpreter; thiếu ⇒ No-Go 3-OS, không bịa |
| R7 | p90 N=5 = max, máy không ghim CPU | kết luận hồi quy sai do nhiễu | median là chính; p90 phụ; ghi load; đo lại nếu load cao |
| R8 | `mps` thiếu `wasm32-unknown-unknown` | `vis-targets` đầy đủ fail trên `mps` | không chạy `vis-targets` trên `mps`; chỉ `vis-gate-macos` (WASI) |
| R9 | `ssh mps` trần bị tailnet từ chối | script “treo”/fail khó hiểu | luôn dùng `mps@mps` (mặc định `vis-remote`); không sửa thành `mps` trần |
| R10 | `bin/vis-gate-windows` log mặc định `/tmp` | mất log khi reboot/đầy tmpfs | đặt `VIS_GATE_WINDOWS_LOG` ngoài `/tmp` |
| R11 | `ST-309` bị hiểu nhầm là “tự mở khi PASS” | vi phạm `VD-289`/`VD-293` | §3.2: PASS chỉ là điều kiện cần; cần quyết định người dùng + `#200` |

---

## 7. Việc còn chờ máy (để `ST-316c` chuyển `DONE`)

- [ ] `mps` fetch/checkout `3e828393` + `cargo clippy --workspace --all-targets -- -D warnings`
      exit 0 (log + sha256).
- [ ] `mps` `bin/vis-gate-macos --full` exit 0 (log + sha256).
- [ ] `psvn` fetch/checkout `3e828393` + `bin/vis-gate-windows --full` exit 0 (log + sha256).
- [ ] `ps` `bin/vis-targets --wasm` exit 0, đủ 3 summary, 0 hỏng (log + sha256).
- [ ] `bin/vis-bench --runs 5 --json` trên **3 OS**, cùng chương trình sha256; baseline + after;
      median/p90 + `binary.sha256`; so hồi quy theo §3.1.
- [ ] Điền bảng §3.1 + quyết định §3.2; A0 duyệt; **chưa** tag, **chưa** mở `ST-309` nếu chưa
      có quyết định người dùng.

> Khi có đủ log, thay tiêu đề “nháp” bằng “báo cáo cuối” và cập nhật `QUEUE` **qua A0** (Builder
> không sửa `QUEUE.md`/`DECISIONS.md`/`ACTIVE.md`).
