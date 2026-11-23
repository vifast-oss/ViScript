<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R37 / Phase C — nhật ký bằng chứng, lệnh và tiêu chí còn thiếu

> **Phạm vi**: nhánh **`plan/r37-phasec`** tạo từ **`3d85bd2`** (tag annotated **`v0.5.5`**).
> **Docs-only** — không sửa mã/test, **không** ssh `mps`/`psvn`, **không** merge/push/tag.
> Worktree soạn: `/home/ps/tmp/r37-phasec`; cây chính `/home/ps/ViScript` giữ `main` sạch.
> Thời điểm: **2026-11-24** (sáng). Time-box phiên: 1,5 h.
>
> Báo cáo chính: [`agent/reports/R37-viscript.md`](../reports/R37-viscript.md) — **khung, chưa có JSON**.
> Tệp này thay thế ma trận cũ `plan/phasec-qa-plan` @ `bab7a50` cho **nền `v0.5.5`**: ma trận cũ
> giả định `fuzz_decode_only` + `VBC_FUZZ_CASES` + `fuzz_watchdog` đã có trong cây, nhưng tại
> `3d85bd2` **chưa có** (chỉ nằm trên nhánh chưa merge `plan/st309-qa` @ `623f3ba`).

## 0. TL;DR

| Hạng mục | Trạng thái thật @`3d85bd2` |
|---|---|
| Linux fresh QA (`b4db374`) | **ĐẠT** — gate 3773/0/4, release 3778/0/5, diff 5×100k = 0, WASI 28/28 + sem 388/390, host 54/54 |
| Windows full (`e6ca52e`) | **1 lần** — 3720/0/4, host 54/54, targets core 38 PASS + sqlite 37/1, phủ 18/18 crate; ma trận cần **3×** + soak 4 h |
| macOS full (`b4db374`) | **1 lần** — 3764/0/3, host-wasm 29/29, sem 388/390; ma trận cần **3×** + soak 4 h |
| Fuzz parser 8 h | **CHƯA ĐỦ** — bản **2 h** đang chạy (pid 2321124), chưa có verdict |
| Fuzz verifier/decoder 100 M | **CHƯA ĐỦ** — mới **60 M / 0 vi phạm** |
| Fuzz VM + watchdog 2 h | **CHƯA CHẠY** — `#195` mở, harness watchdog không có @`3d85bd2` |
| Soak 24 h Linux | **CHƯA CHẠY** — soak 10/10 (650 s) cũ **không thay thế** |
| `#194` / `#195` | **CHƯA ĐÓNG** — điều kiện cứng, xem §4 |
| `H` = commit `0.6.0` | **CHƯA CHỐT** — mọi số trên là `v0.5.5`, phải chạy lại trên `H` |
| Tag `v0.6.0` / `v1.0.0-rc1` | **CHƯA CHỐT** |

**Không blocker sản phẩm** trong phần đã đo; **blocker điều kiện** là các mục CHƯA ĐỦ/CHƯA ĐÓNG ở trên.

## 1. Mốc, head và môi trường

- Tag **`v0.5.5`** (annotated) trỏ **`3d85bd2c26c46027705224185b9eeb3e27a61416`**.
- Bằng chứng đo tại **`b4db374ac16223ae01d184fdfd3033ba1175b435`** (Linux QA, macOS, fuzz/diff)
  và **`e6ca52e45c72f99bf05700b560f9c20397486d00`** (Windows full). Cả hai là tổ tiên `3d85bd2`.
- `git diff --name-status b4db374..3d85bd2`: **6 tệp** — `agent/ACTIVE.md`, `agent/ROADMAP.md`,
  3 journal mới (`linux-qa-v055`, `mac-full-v055`, `win-full-v055b`), **`bin/vis-targets`**
  (bỏ CRLF khi Windows gọi python). **Không tệp mã sản phẩm nào đổi.**
- Môi trường đo: `ps` Linux x86_64 12 CPU, rustc/cargo **1.100.0**; `psvn` MINGW64 6 CPU;
  `mps` macOS 27.0.1 arm64 12 CPU. `CARGO_BUILD_JOBS=3`, `CARGO_PROFILE_DEV_DEBUG=0`,
  `TMPDIR` ngoài repo. Log ngoài repo theo R7.

## 2. Bằng chứng đã có — lệnh và log

### 2.1 Linux fresh clone `/home/ps/vis-qa-v055` @`b4db374`

```bash
./vis-dev gate --fast
cargo test --release --workspace --no-fail-fast
for s in 20261221 20261222 20261223 20261224 20261225; do
  VIS_DIFF_N=100000 VIS_DIFF_SEED="$s" ./bin/vis-diff
done
bin/vis-wasm-suite --require-runtime
bin/vis-wasm-sem --require-target
bin/vis-host-suite
```

| Log (`/home/ps/vislogs/`) | sha256 | Kết quả |
|---|---|---|
| `v055-gate-fast.log` (388.518 B) | `891aa03d908b8d5862bbd518a7d620961791451393e7cfe447d8ef93cb0dff32` | **3773/0/4**, 81,569 s |
| `v055-release-test.log` (244.556 B) | `1b971cf25eff34d8a27025d3c9c3bab5b3bc97302d86040645f28fcb3b2796dc` | **3778/0/5**, exit 0 |
| `v055-diff-5seed.log` | `0a62c2469272e91f6afcf446e9e9b8135cd0c5f923cd2d908a03a44a8bd07fd7` | 5 seed, **lech=0** |
| `v055-wasm-suite.log` | `a664405e9b731e300367961ec9694238201c86b8a7943c49ab90960f765861d8` | **28/28** |
| `v055-wasm-sem.log` | `5d018ca86ea30fac94ce4cd4cf16fd68635ee53ed50734296be64c2d666a9878` | **388/390**, VM 390/390 |
| `v055-host-suite.log` | `3ced5d536710a39d5260932b1a3502b3ed74561bc177a2242ac4bb4a7e461db0` | **54/54** + POSIX |

Chi tiết + 5 log seed: [`linux-qa-v055.md`](linux-qa-v055.md) §7.

### 2.2 Windows `psvn` @`e6ca52e`

```bash
bin/vis-gate-windows --full
```

- **3720/0/4** (workspace 3719 + host 1), 122 block, wall **552 s**; host **54/54**; targets core
  **38 PASS**, sqlite **37 + 1 EXPECTED_RED** (`#156`); dirty **0**.
- Log local `vis-gate-windows-full-v055b.log` —
  `2349d428a280bc4836b9049167293e056e5203c65a50ead4c966dfe7251d447e`;
  bản `psvn` `vis-remote-20261123T202505Z.log` — `8686d0800e43444fe267b86191839aacb8cabde06773e76f1c27aaf6b334ed8e`.
- Chi tiết: [`win-full-v055b.md`](win-full-v055b.md).

### 2.3 macOS `mps` @`b4db374`

```bash
VIS_GATE_MACOS_REPO=/Users/mps/dvchd/ViScript-v050 \
VIS_REMOTE_MPS_REPO=/Users/mps/dvchd/ViScript-v050 \
bin/vis-gate-macos --full
bin/vis-wasm-sem --require-target
```

- Full **exit 0**, wall **237 s**; nextest **3764/0/3** (75,6 s); doc 5/0/1; host-wasm **29/29**;
  WASI suite **28/28**; sem **388/390** (61 s); dirty **0**.
- Log gate `27e0fd91ffdad5ffc2ce760ce20711575d001bafdf87d0806f183e363c81d897`
  (máy đích `45e8489ba0b1bb07195bbfd8ce6b3bd89c7d286169ff0e808ac26fb180d91f5b`);
  log sem `91b58b6b4ca3429a6e7159f3042d2880678c1e192da90d532498c3efd110f16d`.
- Chi tiết: [`mac-full-v055.md`](mac-full-v055.md).

### 2.4 Fuzz/diff bổ sung @`b4db374` (nhánh `plan/fuzz-v055`)

```bash
# verifier/decoder theo SỐ CA (harness có ở tag; không có fuzz_decode_only)
VBC_FUZZ_CA=60000000 cargo test --release -p vis-bytecode --test st309_fuzz_verify -- --nocapture

# differential 100k seed mới
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611240550 \
  cargo test --release -p vis-vm --test diff_100k -- --nocapture
```

| Log (`/home/ps/tmp/fuzz-v055-logs/`) | sha256 | Kết quả |
|---|---|---|
| `verify-60M.log` | `b6a1a299e0f4476ce90d79fc807d1c1e74fede86a14c433f855d949ad5adf7ab` | **60.000.000 ca**, `vi_pham=0`, 40,66 s |
| `diff-100k.log` | `27e019b79f14b5e33190acb5ee7186f00eabb02c541e5071f7bc90d93854914a` | `lech=0`, 93.768 chạy, 41,7 s |
| `parser-2h.log` | chưa chốt (đang ghi) | **ĐANG CHẠY** — pid **2321124**, bắt đầu `2026-11-24T05:05:25+09:00`, seed `202611240550` |

- Binary fuzz verify sha256 `945a18cac24d2c0df3e46fd090a4dbdd33a8cbb853a8f2431c73cda12edf4767`.
- **Tuyệt đối không kill** pid 2321124; chờ hết 7.200 s rồi lấy dòng cuối + sha256.
- Nháp journal fuzz (chưa commit): `/home/ps/tmp/fuzz-v055-logs/journal-draft.md`.

## 3. Việc còn thiếu — lệnh, tiêu chí PASS, thời lượng

> Thứ tự đề xuất: (a) để parser 2 h chạy hết → (b) 100 M verify (~70 s) → (c) đóng `#194`/`#195`
> → (d) chốt `H` bump `0.6.0` → (e) chạy lại ma trận trên `H` (gồm 24 h soak) → (f) R37 JSON + tag.
> Không chạy 24 h soak trước khi chốt `H`, tránh đo lại.

### 3.1 Fuzz parser **8 h**

```bash
cd /home/ps/tmp/r37-linux-${H:0:7}
cargo build --release -p vis-syntax --bin vis-parse-fuzz
"${CARGO_TARGET_DIR:-$PWD/target}/release/vis-parse-fuzz" \
  --seconds 28800 --seed 20261221 \
  | tee /home/ps/vislogs/r37/linux-fuzz-parser-8h-${H:0:7}.log
```

- **PASS**: exit 0; dòng cuối `vis-parse-fuzz: PASS (không vi phạm)`; `vi phạm 0`; có số ca/token/giây.
- **FAIL**: exit 1 / `vi phạm > 0` ⇒ lưu seed + nguồn escape, mở nợ, **không tag**.
- **Đã có**: 2 h (đang chạy) — **không tính** thay 8 h. Thời lượng: **8 h**.
- Bẫy: dùng seed mới, không trùng `20261221` của lần 8 h nếu muốn phủ mới; ghi seed thật vào log.

### 3.2 Fuzz verifier/decoder **100 M ca**

```bash
VBC_FUZZ_CA=100000000 \
  cargo test --release -p vis-bytecode --test st309_fuzz_verify -- --nocapture \
  | tee /home/ps/vislogs/r37/linux-fuzz-verify-100m-${H:0:7}.log
```

- **PASS**: `ST309-FUZZ ca=100000000 … vi_pham=0`; `test decoder_verifier_khong_panic ... ok`; exit 0.
- **Đã có**: 60 M / 40,66 s ⇒ 100 M ước **~68 s** (1,475 M ca/s). Chỉ là **ước lượng**, phải đo thật.
- **Lưu ý harness**: tại `3d85bd2` **không có** `fuzz_decode_only`/`VBC_FUZZ_CASES`; dùng
  `st309_fuzz_verify` với `VBC_FUZZ_CA` (đúng bề mặt decode+verify, không chạy VM).
  Không chép harness từ nhánh chưa merge vào cây QA.

### 3.3 Fuzz có VM + watchdog **2 h**

```bash
# CHỈ sau khi #195 đóng và watchdog đã vào cây; tại 3d85bd2 KHÔNG chạy được đúng tiêu chí.
VBC_FUZZ_PHUT=120 cargo test --release -p vis-vm --test fuzz_mutate -- --nocapture \
  | tee /home/ps/vislogs/r37/linux-fuzz-mutate-2h-${H:0:7}.log
```

- **PASS**: 2 h, 0 vi phạm bất biến (decode/verify/VM không panic, kết thúc trong fuel, stack ≤ `need`),
  **0 ca > 300 s**.
- **Hiện trạng**: `fuzz_mutate` chạy theo **thời gian**, **không có** timeout từng ca; với `#195`,
  một ca verify-OK có thể chạy **>101 phút** (đã gặp ở `plan/st309-qa`). Vì vậy mục này **bị chặn**
  bởi `#195` + harness watchdog (`fuzz_watchdog`, `VBC_FUZZ_WATCH_S`) — cả hai **chưa có** @`3d85bd2`.
- Không dùng `timeout` ngoài để "đạt" tiêu chí: nếu ca bị cắt, đó là **FAIL** của `#195`, không phải PASS.

### 3.4 Soak **24 h** Linux

```bash
export CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp
LOG=/home/ps/vislogs/r37/linux-soak-24h-${H:0:7}.log
: > "$LOG"                      # rỗng; --append cho phép ghi tiếp, không truncate
BAT_DAU=$(date +%s)
while [ $(( $(date +%s) - BAT_DAU )) -lt 86400 ]; do
  bin/vis-soak --log "$LOG" --append || { echo "SOAK_FAIL rc=$?" | tee -a "$LOG"; break; }
done
```

- Mỗi chu kỳ `vis-soak` = **10 vòng** (gate-1..3, diff-1..3, host-1..2, cache-1..2), ~650–760 s
  ⇒ 24 h cần **~120–135 chu kỳ**.
- **PASS**: wall ≥ 86.400 s; **mọi** chu kỳ `10/10 vòng OK`, `SOAK_EXIT=0`; 0 fail sản phẩm,
  **không retry**; RSS/fd phẳng; `git status --porcelain` cuối = **0**; không job nặng song song.
- **FAIL**: bất kỳ vòng đỏ do mã sản phẩm ⇒ dừng, lưu log, mở nợ. Đỏ do tải máy (T-INFRA) phải
  ghi rõ và chạy lại chu kỳ đó, không tính là PASS.
- Dùng `bin/vis-lock` giữ máy độc quyền; `bin/vis-gc-targets` trước khi bắt đầu; đĩa trống > 10 GB.
- **Đã có**: soak 10/10 **650 s** @`e9c62cb` — **không** thay thế 24 h.

### 3.5 Windows **3× full** + soak 4 h; macOS **3× full** + soak 4 h + sem

```bash
# psvn, worktree riêng /c/Users/ps/viscript-r37-${H:0:7}
bin/vis-gate-windows --full        # ×3, mỗi lần dirty 0
# mps, clone riêng + VIS_GATE_MACOS_REPO/VIS_REMOTE_MPS_REPO
bin/vis-gate-macos --full          # ×3, mỗi lần dirty 0
bin/vis-wasm-sem --require-target  # ≥ 388/390
```

- **PASS**: mỗi lần full **0 failed**, skipped đúng danh sách, host **54/54** (Windows) / host-wasm
  **29/29** (macOS), targets core **38 PASS** + sqlite **37/1** (`#156`, Windows), WASI **28/28**, sem **≥ 388/390** (macOS);
  dirty **0** sau mỗi lần; cùng `H`.
- **Soak 4 h** mỗi nền: 0 fail sản phẩm, RSS/fd (hoặc handle) phẳng tới mức đo được, dirty 0.
- **Đã có**: Windows **1×** @`e6ca52e`, macOS **1×** @`b4db374` — **chưa đủ 3×**, chưa soak 4 h trên `v0.5.5`.

### 3.6 Chốt `H` = commit `0.6.0` rồi chạy lại toàn bộ

- Bump `[workspace.package] version` **0.6.0**; đồng bộ `Cargo.lock` + `vis-host-wasm/Cargo.lock`;
  `PHIEN_BAN` + khoá cache mới; test version `st289_version.rs`; `CHANGELOG.md`; `vis --version`.
- **Sau `H` không đổi mã**. Mọi số Phase C phải đo lại trên `H`: version đổi ⇒ cache key đổi ⇒
  test kỳ vọng đổi; số `v0.5.5` chỉ là **pre-evidence**, không dùng làm số chốt R37.
- Ước lượng: Linux fresh (1–2 h) + 24 h soak + 8 h parser (song song được với soak? **không** —
  `vis-soak` cấm chạy song song build/cổng khác, AGENTS §6) ⇒ lịch tuần tự tối thiểu **~36–40 h**
  cho Linux, cộng 3× Windows + 3× macOS + soak 4 h mỗi nền (có thể chạy song song 3 máy).

### 3.7 Đóng `#194`, `#195` và R37 JSON

- `#194`: API theo **fd** (`statat`/`listat`/`readlinkat`/`openat2`) trong `vis-host-native::quyen`;
  ca đua `rename`/`symlink` **100.000 vòng** Linux + macOS, **0 rò** metadata; cập nhật
  `HOST-BAO-DAM.md`; ghi đóng `SO-NO.md`. Windows giữ caveat `FsGuard`.
- `#195`: **đăng ký dòng `SO-NO` trước** (hiện chưa có); dọn/khử trùng `loop_ends` hoặc chỉ mục
  `(fi, end_pc)`; **deadline wall-clock** cho `Vm::run`; watchdog từng ca; nghiệm thu slowcase
  < 300 s ở fuel 100 M, 1M→10M→100M mỗi bậc ≤ 15×, watchdog 2 h 0 ca > 300 s.
- R37 JSON: `gen_report_json.py` hiện chỉ có cấu hình round 35/36 — phải **thêm round 37** (head `H`,
  overrides `ST-309` nếu QUEUE chưa lật) hoặc script tương đương; sinh `.license`; chạy
  `python3 test/report/check_report.py --n 37` và `--all`.

## 4. Điều kiện cứng và nợ đã biết

| Mục | Trạng thái | Ảnh hưởng Phase C |
|---|---|---|
| **`#194`** race metadata-only FS | **mở** (`SO-NO`, `st309-security.md` §4) | **Chặn tag** — phải đóng bằng API theo fd + ca đua 100k |
| **`#195`** VM back-edge O(n²)/fuel | **đề xuất**, **chưa có dòng `SO-NO`** | **Chặn tag** — phải đăng ký + đóng + watchdog |
| **`#193`** WASI interp stack 2/390 | mở (known gap) | **Không chặn** — ngưỡng ≥ 388/390; VM 390/390 |
| **`#156`** sqlite wasm32 EXPECTED_RED | mở (known) | **Không chặn** — đúng danh sách targets |

Không waiver mặc định. Nếu A0 quyết định khác, phải ghi quyết định riêng trong `DECISIONS.md`.

## 5. Điều kiện dừng / blocker của phiên soạn

- **Không blocker sản phẩm** từ phần đã đo: mọi cổng bắt buộc xanh, WASI sem đạt ngưỡng.
- **Blocker điều kiện**: `#194`/`#195` chưa đóng; soak 24 h/fuzz 8 h/fuzz 100 M/watchdog chưa đủ;
  `H` chưa chốt; chưa có 3× full Windows/macOS trên `v0.5.5`.
- **Bất biến vận hành**: **không kill** pid fuzz 2321124; không chạy soak/fuzz song song với
  build/cổng khác; không ssh `mps`/`psvn` trong phiên này.
- Nếu fuzz parser 2 h kết thúc với `vi phạm > 0`: dừng, lưu seed/nguồn, mở nợ — **không** tiến tới tag.

## 6. Những gì phiên này KHÔNG làm

- Không sửa mã/test; không merge/push/tag; không ssh `mps`/`psvn`.
- Không tạo `R37-viscript.json` (đúng chủ đích: chỉ sinh khi đủ bằng chứng + chốt `H`).
- Không chạy lại `./vis-dev check`/`gate --fast` trên nhánh docs-only này (theo tiền lệ R36/R34);
  cổng bắt buộc của phiên là `python3 test/report/check_report.py --all`.
- Không dùng số R35/R36 hay soak 650 s cũ để thay số Phase C.

## 7. Kết luận

Tại tag **`v0.5.5` @ `3d85bd2`**, Phase C đã có **nền bằng chứng 3-OS + WASI + diff** đáng tin
(1 lần mỗi nền), nhưng **chưa đủ điều kiện tag `v0.6.0`/`v1.0.0-rc1`**. Việc còn lại theo thứ tự:
parser 8 h → verify 100 M → đóng `#194`/`#195` (+ watchdog) → chốt `H` `0.6.0` → chạy lại ma trận
(gồm soak 24 h + 3× full mỗi nền + soak 4 h) → R37 JSON → A0 duyệt tag.
