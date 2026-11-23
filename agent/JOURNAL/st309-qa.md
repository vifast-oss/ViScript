<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-309 — QA/fuzz dài (parser 1h, verifier/decoder 1h, diff 5×100k, test release, WASM local)

> **Người làm**: subagent ST-309 QA, 2026-11-23. **Nhánh** `plan/st309-qa` từ **`c4337ce`**
> ([ST-300m3] fix race driver WASI). **Local only** — không ssh `mps`/`psvn`, không merge/push/tag.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.
>
> **Worktree riêng** `/home/ps/tmp/st309-qa-wt` vì cây chính `/home/ps/ViScript` đang được một
> subagent khác sửa trực tiếp (`crates/vis-host-native/src/quyen_st295.rs`,
> `crates/vis-plugin-abi/tests/parity.rs`) và chạy `vis-dev gate --fast`. Cây chính đã được trả về
> nhánh `plan/st309-security` + giữ nguyên các thay đổi dở dang đó; QA này **không** đụng vào chúng.
> Mọi số dưới đây đo trên `c4337ce` sạch trong worktree.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Fuzz **parser** 1h, seed mới `20261221` | **PASS** — 286.313.039 ca / 6.497.876.005 token / **0 vi phạm** |
| Fuzz **verifier/decoder** harness `fuzz_mutate` 1h | **KHÔNG HOÀN TẤT** — 1 ca đột biến hợp lệ làm VM chạy bậc hai (repro >31 phút, lần gốc >101 phút); dừng tay, đã lưu artifact. Không panic/UB |
| Fuzz **decode + verify** thuần 1h, seed `20261226` | **PASS** — 2.517.069.629 ca / **0 panic** |
| Differential `VIS_DIFF_N=100000` × **5 seed** | **lech=0** cả 5 (tổng 500.000 chương trình) |
| `cargo test --release --workspace` | **PASS** — 3.754 passed / 0 failed / 8 ignored (130 tập, gồm doc-test) |
| `bin/vis-wasm-sem` (Node local, build lại wasm) | **388/390 eligible = 99,5%**, 2 fail **known gap** (interp stack overflow) |
| `bin/vis-wasm-suite` (Node local) | **28/28** |
| Blocker | **Có 1 phát hiện mới**: bytecode verify-OK có thể chạy hàng giờ (fuel 100M lệnh không phải cap wall-clock) — cần task riêng, **không sửa** trong QA này |

## 1. Môi trường

```text
HEAD    c4337ce402b59d6790cd9b202bf1aef0759399a0  [ST-300m3] fix race driver WASI (thu muc rieng moi invocation)
rustc   1.100.0 (0ef1bc386 2026-11-09)
host    Linux 7.0.0-43-generic x86_64, 12 CPU, 14 GiB RAM
```

- Build/kiểm thử trong `/home/ps/tmp/st309-qa-wt` (worktree), `CARGO_TARGET_DIR=/home/ps/ViScript/target`.
- Không chạy `mps`/`psvn`; WASM chỉ chạy local bằng Node `node:wasi` + oracle native có sẵn
  `/home/ps/vislogs/sem-native-baseline.tsv`
  (sha256 `482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d`).

## 2. Fuzz parser 1 giờ — seed mới `20261221`

```text
vis-parse-fuzz: 286313039 ca, 6497876005 token, 79531 ca/s, 1804966 token/s, 3600.0 s, vi phạm 0
vis-parse-fuzz: PASS (không vi phạm)
```

- Lệnh: `target/release/vis-parse-fuzz --seconds 3600 --seed 20261221` (build release từ `c4337ce`).
- Bất biến của harness: không panic; CST lossless khi parse sạch; span hợp lệ; lỗi ⇒ có chẩn đoán.
- **0 vi phạm**, không có seed/corpus lỗi cần lưu. Tốc độ 79.531 ca/s (máy có tải song song).

## 3. Fuzz verifier/decoder

### 3a. Harness hiện có `vis-vm/tests/fuzz_mutate.rs` — phát hiện ca chạy >101 phút

- Harness: đột biến bit/byte/cắt cụt/chèn trên bytecode của **437 module** `test/sem`; mỗi ca
  `decode` → `verify` → nếu verify OK thì chạy VM (fuel an toàn `VM_SAFETY_FUEL = 100.000.000` lệnh).
  Seed **cố định trong harness** `0x5EED_2026_1121`.
- Lệnh: `VBC_FUZZ_PHUT=60 cargo test --release -p vis-vm --test fuzz_mutate -- --nocapture`.
- **Kết quả**: không có dòng `FUZZ` sau **6.061 s** (1h41m); dừng tay bằng SIGTERM.
  Log chỉ có dòng "running for over 60 seconds" — harness **không in ca đang chạy** nên không tự
  xác định được ca.
- **Truy vết**: chạy lại đúng dãy RNG bằng bản sao harness + watchdog (file tạm, không commit).
  Watchdog bắt **ca `756.892`**, module gốc `test/sem/dieu-khien/for-str.vis`, bytecode đột biến
  **196 byte**. Disassemble cho thấy back-edge bị đổi từ nhảy về `IterNext` thành nhảy về
  `IterInit` (lệnh `Jump(-7)` tại pc 7), nên vòng `for` **khởi tạo lại iterator mỗi vòng** →
  in lặp vô hạn, chỉ dừng khi fuel 100M lệnh cạn.
- Ca này được đem chạy lại riêng (`slowcase_repro`, release): **vẫn chạy sau 31 phút 22 giây**
  (dừng tay), RSS tăng dần tới **~540 MB** — tức fuel 100M lệnh **không phải** giới hạn wall-clock.
- **Không có panic, không UB, không sai lệch ngữ nghĩa**: `decode` + `verify` **chấp nhận** bytecode
  (đây là bytecode hợp lệ về cấu trúc); vấn đề là **thời gian chạy**.

#### 3a-bis. Định lượng: **bậc hai**, không phải chỉ “chậm vì fuel 100M”

Chẩn đoán bằng cách **hạ tạm** `fee_shared::VM_SAFETY_FUEL` trong worktree (đã revert, không commit)
rồi chạy lại cùng artifact (ca 2, `for i in 0..2`):

| Fuel (lệnh) | Thời gian | Steps | stdout | Chẩn đoán |
|---:|---:|---:|---:|---|
| 1.000.000 | **3,03 s** | 1.000.001 | 166.666 B | `VS0501-FUEL` |
| 10.000.000 | **482,72 s** (8m03) | 10.000.001 | 1.666.666 B | `VS0501-FUEL` |
| 100.000.000 (mặc định) | **>1.884 s** (31m22, dừng tay) | — | — | chưa kết thúc |

- 10× số lệnh ⇒ **160× thời gian** ⇒ **siêu tuyến tính**, ngoại suy bậc hai cho 100M là **~8–13 giờ**
  (khớp việc lần chạy gốc không xong sau 101 phút). Vì vậy “chờ fuel cạn” **không** phải biện pháp
  giới hạn thời gian.
- **Nguyên nhân mã** (đọc `crates/vis-vm/src/vm.rs`):
  - `IterInit` (pc ~1034) mỗi lần chạy đều `self.loop_ends.push((fi, end_pc, base))` và
    `self.iters.push(items)`;
  - bình thường `IterNext` khi hết vòng (hoặc `break`) mới pop `iters` + `loop_ends`;
  - bytecode đột biến cho back-edge nhảy về `Const`/`IterInit` ⇒ **không bao giờ hết vòng** ⇒
    `loop_ends` **phình 1 phần tử mỗi vòng**;
  - mỗi back-edge đi qua `dat_pc()` (pc ~542) quét `loop_ends.iter().rposition(...)` **tuyến tính**
    trên vec đang phình ⇒ tổng **O(n²)** + bộ nhớ O(n) (RSS tăng ~540 MB).
- Đây là lỗi **sẵn sàng/DoS trên bytecode đã verify**, không phải panic/UB. Hướng sửa đề xuất cho
  task riêng: không để `loop_ends` phình khi `IterInit` lặp lại cùng `(fi, end_pc)` (dọn/khử trùng
  hoặc chỉ mục theo `end_pc`), và/hoặc thêm deadline wall-clock cho `Vm::run`; harness fuzz cần
  timeout từng ca + in ca đang chạy để tự lưu artifact.
- Artifact (ngoài repo):
  - `/tmp/opencode/st309-fuzz-vbc-slowcase1.dump.txt` (disassemble ca 1) — sha256
    `2543089b1af86939b9284d2dd2f440f7841cfec3bbd30e64f56fd8900af9d4c1`.
  - Bản `.bin` gốc của ca 1 đã bị lần chạy seed sau ghi đè (lỗi công cụ QA, xem §3b); sha256 bản gốc
    đã ghi nhận: `54ad9fd274bc0e236b7318f4129d4dfeaf57c17f160e168462221dadb2d42756` (196 byte).
- **Phân loại**: availability/DoS trên đường **bytecode đã verify** (bytecode đến từ cache/plugin
  không tin cậy). Không phải panic/UB; không sửa trong task này.

### 3b. Seed mới `20261226` + watchdog 300 s — gặp ca chậm thứ hai

- Cùng harness (file tạm có thêm `VBC_FUZZ_SEED` + watchdog, bất biến giữ nguyên), chạy
  `VBC_FUZZ_PHUT=60 VBC_FUZZ_WATCH_S=300 VBC_FUZZ_SEED=20261226`.
- Sau ~4 phút, watchdog bắt **ca `271.013`**: module gốc dạng
  `let i = 99; for i in 0..2: print(i); print(i)`, bytecode đột biến **279 byte**; disassemble cũng
  cho thấy back-edge nhảy về đầu vòng (`Jump(-11)` tại pc 11) → khởi tạo lại range mỗi vòng.
  Artifact: `/tmp/opencode/st309-fuzz-vbc-slowcase2.bin` (sha256
  `1412719e4565601b6a8339f9d16a21c3d53484ae4dbf5046cea91d9873d42096`) và `.txt`, `.dump.txt`.
- **Hai seed khác nhau đều gặp ca chậm trong vài phút** ⇒ đây là lớp lỗi lặp lại, không phải cá biệt.

### 3c. Fuzz decode + verify thuần 1 giờ — seed `20261226`

- Vì §3a/§3b không thể chạy đủ 1h (VM là nút thắt), chạy thêm harness **chỉ `decode` +
  `verify`** (không chạy VM), cùng phép đột biến byte và 437 module, seed `20261226`, 3.600 s.
- Kết quả:
  ```text
  FUZZ_DECODE giay=3600 seed=20261226 tong=2517069629 decode_tu_choi=1633929435 nhan=805774345     verify_tu_choi=77365849 vi_pham=0 thuc=3600.0s toc_do=699186/s
  ```
  **2.517.069.629 ca**, `decode` từ chối 1.633.929.435, decode OK 805.774.345, `verify` từ chối
  77.365.849, **0 panic/vi phạm**, 699.186 ca/s. Không có artifact vi phạm nào được ghi.

## 4. Differential 100k × 5 seed

`VIS_DIFF_N=100000 VIS_DIFF_SEED=<seed> <binary diff_100k> --nocapture` (release):

| Seed | Tổng | Lỗi tính | Chạy được | **Lệch** | Giây | Tốc độ |
|---|---:|---:|---:|---:|---:|---:|
| 20261221 | 100.000 | 6.233 | 93.767 | **0** | 36,9 | 2.708/s |
| 20261222 | 100.000 | 6.233 | 93.767 | **0** | 36,8 | 2.716/s |
| 20261223 | 100.000 | 6.233 | 93.767 | **0** | 43,3 | 2.310/s |
| 20261224 | 100.000 | 6.233 | 93.767 | **0** | 35,3 | 2.836/s |
| 20261225 | 100.000 | 6.233 | 93.767 | **0** | 39,2 | 2.553/s |

- Tổng **500.000 chương trình**, **lech=0** cả 5 seed; không có dòng `LECH`/`MA` nào.
- (`loi_tinh=6233` giống nhau giữa các seed vì bộ sinh dùng chung phân bố nhóm; khác `20261115`
  của các vòng trước ở 6.230 — không ảnh hưởng kết luận 0 lệch.)

## 5. `cargo test --release --workspace`

- Lệnh: `cargo test --release --workspace` (worktree `c4337ce`, target dùng chung).
- **Lần 1** (trước khi thêm 4 test chẩn đoán): **PASS** 3.753 passed / **0 failed** / 5 ignored;
  126 tập kết quả (unit + integration + doc-test); log `/tmp/opencode/st309-workspace-test.log`.
- **Lần 2** (sau khi thêm test chẩn đoán, xác nhận chúng không phá cổng): **PASS**
  **3.754 passed / 0 failed / 8 ignored**; 130 tập; exit 0; log
  `/tmp/opencode/st309-workspace-test-final.log`. 3 test dài bị `#[ignore]`; `dump_only` tự bỏ qua
  khi thiếu `SLOWCASE`.

## 6. WASM local (Node `node:wasi`, không cần `mps`)

- `bin/vis-wasm-sem --build --runtime node --require-target`:
  ```text
  # sem total=525 eligible=390 excluded=135 runtime=node
  # oracle_sha256=482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d
  # SUMMARY total=525 eligible=390 excluded=135 pass=388 fail=2 coverage=99.5% target=85%
  ```
  2 fail **known gap** (đã ghi ở `agent/JOURNAL/st300-main-verify.md`): `ham/de-quy-1000.vis`,
  `ham/de-quy-vo-han.vis` — interp WASI stack overflow; VM vẫn pass rc/hash. **Không phải hồi quy.**
- `bin/vis-wasm-suite --runtime node`: **28 ca, 0 hỏng**, exit 0.

## 7. Log ngoài repo + sha256

| Tệp | Byte | sha256 |
|---|---:|---|
| `/tmp/opencode/st309-fuzz-parser-1h.log` | 139 | `89796b609357ac1fe42298820be1c6f7d090483276b8663bc4e3501bf59af248` |
| `/tmp/opencode/st309-fuzz-vbc-1h.log` | 522 | `916ef9752591deb970dc8f54fa02554dd78be99f9032e9d08396482d404d0092` |
| `/tmp/opencode/st309-fuzz-vbc-1h-seed20261226.log` | 268 | `3c9c48c1c0a5940bb7b4e4942bffba46c72c7026ea0e8c503313e8b63c952023` |
| `/tmp/opencode/st309-fuzz-decode-1h.log` | 12.888 | `a924fbb5f193eb21560ecb73a1d8a8c3907be7c5bfbc4936aa258b9b7961d7f4` |
| `/tmp/opencode/st309-diff-20261221.log` | 625 | `0b9c4ad53bcc709a423c8e0accedcd969cb45756469a899b9fda6c38b653f511` |
| `/tmp/opencode/st309-diff-20261222.log` | — | `eec81b08251aee99eb3a49caa148c432b3ee4152d2e7270895be9a97582c5135` |
| `/tmp/opencode/st309-diff-20261223.log` | — | `87b95890c308fae9cd135a0c9e0551af63546f65f68f0cfddfa27fd2c1fdf6e9` |
| `/tmp/opencode/st309-diff-20261224.log` | — | `c916acb9fe16b0ca23ed77b7b2f53aeec2ffeeb2c3d215f12dff0f7d7159bf8c` |
| `/tmp/opencode/st309-diff-20261225.log` | — | `ee6aea4042c5ced419a588fb8b70a0ea7fd0e1aa77c2ad702563797cc150b4a5` |
| `/tmp/opencode/st309-workspace-test.log` | 241.291 | `3c3b3ecf1af8ef8572764460618019be80ddc9a7ad90b2ccc6bd827de14cd71d` |
| `/tmp/opencode/st309-workspace-test-final.log` | 245.543 | `ac0b0f21e9cda0641407571defee43fc87d9834ba90f8a989b8715456b7d8dcf` |
| `/tmp/opencode/st309-wasm-sem.log` | 61.338 | `dae04acdb6db437ffc6bdff36e2288023950c195b297b4e3e713f076f54b7d63` |
| `/tmp/opencode/st309-wasm-suite.log` | 947 | `d9d1766f888f815796e07c0d15e390ad4a8e5f86e20fbe2b33e534f22dda1104` |
| `/tmp/opencode/st309-fuzz-vbc-slowcase1.dump.txt` | 444 | `2543089b1af86939b9284d2dd2f440f7841cfec3bbd30e64f56fd8900af9d4c1` |
| `/tmp/opencode/st309-fuzz-vbc-slowcase2.bin` | 279 | `1412719e4565601b6a8339f9d16a21c3d53484ae4dbf5046cea91d9873d42096` |
| `/tmp/opencode/st309-fuzz-vbc-slowcase2.txt` | 162 | `248bf459fbb63dd555aff52261e9882591e752ee78b661b87b84608ad2fbc10c` |
| `/tmp/opencode/st309-fuzz-vbc-slowcase2.dump.txt` | 702 | `cfa50e6b92833912ebcce5364ca3abe491bc68cde041f5cc12739b4cc7bd9c50` |
| `/tmp/opencode/st309-fuzz-vbc-slowcase.run.log` | 124 | `eb2ee48de623cf3e48cddce4ab0deb4d8a104e2e1d3565dff3c8864dcc9c296f` |

## 8. Kết luận / khuyến nghị

- Parser, differential, test release, WASM local: **đạt**; không hồi quy mới.
- **Phát hiện mới (blocker cho “fuzz verifier 1h sạch”)**: bytecode **verify-OK** có thể chạy
  **>101 phút** cho một module 196 byte; nguyên nhân đã khoanh vùng: `IterInit` lặp lại làm
  `loop_ends` phình, `dat_pc()` quét tuyến tính ⇒ **O(n²)** thời gian + O(n) bộ nhớ (fuel 100M lệnh
  không phải cap wall-clock). Đề nghị **nợ mới `#195`** (chưa thêm vào `SO-NO.md`, chờ A0 duyệt
  số): “VM chạy bậc hai khi `IterInit` bị lặp qua back-edge trên bytecode verify-OK — cần dọn
  `loop_ends`/chỉ mục theo `end_pc` + deadline wall-clock; harness fuzz cần timeout từng ca và in
  ca đang chạy”.
- **Không sửa sản phẩm** trong task QA này; không merge/push/tag.
- Các file test chẩn đoán **được commit** (đều `#[ignore]`/tự bỏ qua khi thiếu env, không làm chậm
  cổng): `crates/vis-vm/tests/fuzz_watchdog.rs`, `fuzz_decode_only.rs`, `fuel_probe.rs`, `dump_only.rs`.
  `slowcase_repro.rs` **không commit** vì chỉ là bản chạy lại một artifact cụ thể (đã thay bằng
  `fuel_probe` + `dump_only`), chạy VM không giới hạn nên không phù hợp test thường trực.

## 9. Test chẩn đoán kèm commit (ST-309)

| Tệp | Vai trò | Chạy |
|---|---|---|
| `crates/vis-vm/tests/fuzz_watchdog.rs` | fuzz verifier/decoder + watchdog từng ca, seed env, ghi artifact ca chậm | `VBC_FUZZ_PHUT=60 VBC_FUZZ_WATCH_S=300 VBC_FUZZ_SEED=… VBC_FUZZ_ART_DIR=… cargo test --release -p vis-vm --test fuzz_watchdog -- --ignored --nocapture` |
| `crates/vis-vm/tests/fuzz_decode_only.rs` | fuzz `decode`+`verify` thuần, ghi artifact vi phạm | `VBC_DECODE_FUZZ_S=3600 VBC_FUZZ_SEED=… cargo test --release -p vis-vm --test fuzz_decode_only -- --ignored --nocapture` |
| `crates/vis-vm/tests/fuel_probe.rs` | đo thời gian/steps/stdout của artifact nghi chậm (dùng để lấy đường cong bậc hai) | `SLOWCASE=<bin> cargo test --release -p vis-vm --test fuel_probe -- --ignored --nocapture` |
| `crates/vis-vm/tests/dump_only.rs` | decode/verify/disassemble artifact, không chạy VM | `SLOWCASE=<bin> cargo test --release -p vis-vm --test dump_only -- --nocapture` |
