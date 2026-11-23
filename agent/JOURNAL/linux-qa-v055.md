<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# QA Linux `v0.5.5` — tag `v0.5.5` @ `b4db374` (fresh clone, 5 seed)

> **Phạm vi**: xác minh độc lập mốc phát hành **`v0.5.5`** (tag annotated trỏ
> **`b4db374ac16223ae01d184fdfd3033ba1175b435`**) trên **clone mới** `/home/ps/vis-qa-v055`,
> nhánh `plan/linux-qa-v055` tạo từ chính commit đó. **Không** merge/push/tag; **không** ssh
> `mps`/`psvn`; không sửa mã/test (chỉ thêm tệp journal này).
> Môi trường: `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`, target trong clone.
> Toolchain: rustc/cargo **1.100.0** (`~/.cargo/bin`, ghim bởi `rust-toolchain.toml`).
> Thời điểm: 2026-11-24 05:04–05:22 +09:00, máy `ps` local (Linux x86_64, 12 CPU).
> **Verdict: PASS, không blocker** — trừ **Phase C soak 24 h CHƯA chạy** (ngoài thời gian còn lại).

## 1. Tổng hợp

| Hạng mục | Kết quả | Log ngoài repo |
|---|---|---|
| `./vis-dev gate --fast` | **PASS** — nextest **3773 passed / 0 failed / 4 skipped** (103 binary, 81,569 s); fmt/clippy/R1..R9/toolchain/release/self-test/explain/báo cáo/build/doc-examples/readme-examples/L0 đều OK | `v055-gate-fast.log` |
| `cargo test --release --workspace --no-fail-fast` | **PASS** — **3778 passed / 0 failed / 5 ignored**, 121 binary test (103 unit+IT, 18 doc-test), exit 0 | `v055-release-test.log` |
| Diff 100k × **5 seed** (`20261221..25`) | **`lech=0` cả 5** (`tong=100000 loi_tinh=6233 chay=93767`) | `v055-diff-5seed.log` + `v055-diff-seed-*.log` |
| `bin/vis-wasm-suite --require-runtime` | **28/28**, runtime `node` (Node 26.5.0), exit 0 | `v055-wasm-suite.log` |
| `bin/vis-wasm-sem --require-target` | **388/390 = 99,5 %** (interp 388, **VM 390/390**) — đúng nợ **#193** đã biết | `v055-wasm-sem.log` |
| `bin/vis-host-suite` | **54/54** ca xanh + `host_posix_real` PASS, nền `Linux x86_64` | `v055-host-suite.log` |
| `vis --version` | **`vis 0.5.5`** (cả `target/debug` và `target/release`) | — |
| Phase C soak 24 h | **CHƯA CHẠY** (ngoài time-box) | — |

## 2. Gate `--fast`

- Exit 0, `vis-dev gate --fast: PASS`; log 388.518 byte (ghi cả từng dòng PASS của nextest).
- `Starting 3773 tests across 103 binaries (4 tests skipped)` →
  `Summary [ 81.569s] 3773 tests run: 3773 passed (1 slow), 4 skipped` — khớp kỳ vọng **3773/4**.
- Doc-examples: `31 khối tài liệu: 26 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3`.
- Bẫy môi trường (không phải lỗi mã): `/usr/bin/cargo` là **1.93.1** < `rust-version = "1.100.0"`;
  `bin/_vis-toolchain.sh` tự rơi về `~/.cargo/bin/cargo` **1.100.0** và cổng xanh. Nếu gọi
  `cargo` trần (ngoài `vis-dev`) phải đặt `PATH=/home/ps/.cargo/bin:$PATH`.

## 3. Release workspace

- `cargo test --release --workspace --no-fail-fast`, exit 0, ~6 phút (05:16 → 05:21:54).
- Tổng theo `test result:`: **121 binary**, **3778 passed**, **0 failed**, **5 ignored**;
  không có dòng `error`/`FAILED` nào.
- `target/release/vis --version` ⇒ `vis 0.5.5`.

## 4. Diff 100k — 5 seed

Mỗi lần chạy kèm bước 2.000 chương trình (seed cố định `20261115` của test): `lech=0` cả 5 lần
(`loi_tinh=126 chay=1874`).

| Seed | Dòng 100k | Giây |
|---|---|---:|
| 20261221 | `DIFF tong=100000 loi_tinh=6233 chay=93767 lech=0 giay=86.1 toc_do=1162/s` | 86,1 |
| 20261222 | `DIFF tong=100000 loi_tinh=6233 chay=93767 lech=0 giay=104.3 toc_do=958/s` | 104,3 |
| 20261223 | `DIFF tong=100000 loi_tinh=6233 chay=93767 lech=0 giay=97.1 toc_do=1030/s` | 97,1 |
| 20261224 | `DIFF tong=100000 loi_tinh=6233 chay=93767 lech=0 giay=83.3 toc_do=1201/s` | 83,3 |
| 20261225 | `DIFF tong=100000 loi_tinh=6233 chay=93767 lech=0 giay=99.2 toc_do=1008/s` | 99,2 |

Tổng **500.000 chương trình**, **0 lệch** interp↔VM; `test result: ok. 7 passed` mỗi lần.

## 5. WASI (workspace riêng `vis-host-wasm`)

### 5.1 Smoke suite — 28/28

- Build `wasm32-wasip1 --release`: `vis-wasm-run.wasm` **3.829.701 byte**.
- `--require-runtime` (không chấp nhận BLOCKED-ENV): chọn runtime **`node`**
  (`node:wasi`, Node **26.5.0**; máy không có `wasmtime` CLI/Python wasmtime).
- `# tổng: 28 ca, 0 hỏng, runtime=node`, exit 0 — gồm preopen đọc/ghi, preopen `@ro`, chặn
  thoát gốc/symlink, từ chối `net`/`proc`/`sqlite`, đồng hồ, argv, đa tệp.

### 5.2 `vis-wasm-sem` — 388/390 (đúng nợ #193)

- `# sem total=525 eligible=390 excluded=135`, oracle
  `/home/ps/vislogs/sem-native-baseline.tsv` sha256 **khớp** baseline
  `482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d`.
- `# SUMMARY … pass=388 fail=2 coverage=99,5% target=85%`; exit 0 (`--require-target`).
- **2 ca hỏng đều là interp, VM đúng** (không có dòng lỗi `# wasm vm`):
  - `test/sem/ham/de-quy-1000.vis` — interp wasm `rc=1`, native `0`:
    `RuntimeError: memory access out of bounds` tại `vis_interp::interp::Interp::eval`.
  - `test/sem/ham/de-quy-vo-han.vis` — interp wasm `rc=1`, native `101`.
  - Đây **đúng** nợ **#193** (`agent/SO-NO.md`): đệ quy sâu tràn stack interp trên
    `wasm32-wasip1`, VM **390/390 = 100 %**, coverage tổng 388/390. **Không phải hồi quy mới.**

## 6. Host

- `bin/vis-host-suite`: **54/54** ca xanh, `host_posix_real PASS` (lệnh POSIX thật), exit 0,
  nền `Linux x86_64`.

## 7. Log ngoài repo + sha256 (R7)

Tất cả trong `/home/ps/vislogs/`, **không** nằm trong repo:

```text
891aa03d908b8d5862bbd518a7d620961791451393e7cfe447d8ef93cb0dff32  v055-gate-fast.log         (388518 B)
1b971cf25eff34d8a27025d3c9c3bab5b3bc97302d86040645f28fcb3b2796dc  v055-release-test.log      (244556 B)
0a62c2469272e91f6afcf446e9e9b8135cd0c5f923cd2d908a03a44a8bd07fd7  v055-diff-5seed.log          (1557 B)
d94431080e690721b0dd853f047f2b4b6761d24ffba3b1ede783369c66490715  v055-diff-seed-20261221.log   (263 B)
7414f4307160cd8299031f7333647af1eddc272acf4225eb1086665275ef7849  v055-diff-seed-20261222.log   (264 B)
5ee3e0667a6e5da926dd94ef187e6cad9d078ad30b88ddb4b1121a06dea0e557  v055-diff-seed-20261223.log   (263 B)
225c1095c92dd5389f31c7c97443d1f8ced25a6d7c169cef6c156f344b6f40a5  v055-diff-seed-20261224.log   (263 B)
7548f6b08acce44f8ebd935157345cc1e7b3fb71ceee4bff43d9a5b3d3cde9ec  v055-diff-seed-20261225.log   (264 B)
a664405e9b731e300367961ec9694238201c86b8a7943c49ab90960f765861d8  v055-wasm-suite.log         (2118 B)
5d018ca86ea30fac94ce4cd4cf16fd68635ee53ed50734296be64c2d666a9878  v055-wasm-sem.log          (60184 B)
3ced5d536710a39d5260932b1a3502b3ed74561bc177a2242ac4bb4a7e461db0  v055-host-suite.log         (2033 B)
```

## 8. Blocker / việc chưa làm

1. **Không blocker sản phẩm.** Mọi cổng bắt buộc xanh; WASI sem đạt đúng ngưỡng ≥388/390.
2. **Phase C soak 24 h CHƯA chạy** — ngoài time-box 2 h của phiên này; cần chạy riêng trước khi
   tuyên bố đủ điều kiện Pha C.
3. Hai ca WASI đỏ là nợ **#193** đã mở (stack interp trên wasm), không phải hồi quy `v0.5.5`.
4. Không chạy Windows/macOS (không ssh `mps`/`psvn` theo yêu cầu); số liệu này chỉ cho Linux.
5. Không merge/push/tag; clone giữ nguyên mã nguồn, chỉ thêm journal này.

## 9. Kết luận

Tại tag **`v0.5.5` @ `b4db374`**, trên fresh clone Linux: gate **3773/4**, release
**3778 passed / 0 failed / 5 ignored**, diff 100k **5 seed lệch 0** (500.000 chương trình),
WASI suite **28/28**, WASI sem **388/390** (VM 390/390; nợ #193), host **54/54** + POSIX PASS,
`vis 0.5.5`. **PASS** cho phạm vi đã đo; **Phase C soak 24 h còn treo**.
