<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300m3 final5 — macOS arm64 Tier 1: **CHẶN** (cổng đỏ tất định ở `lat256_net::t3`)

> **Người chạy**: subagent VS3-W4, 2026-11-23/24 (JST). **Mốc đo**: `8906c70ffa341e1496d4f55ebb49b81c389a1be6`
> (`main`, version **0.5.0**; R36 final2 + ST-309 fallback Darwin `ELOOP/NOTDIR → VS0604`).
> **Máy**: `mps` — macOS **27.0.1** (Darwin 27.0.0) **arm64**, 12 core / 32 GiB, Rust **1.100.0**,
> Node **26.12.1**; độc quyền. **Không merge/push/tag.** Clone đo: `/Users/mps/dvchd/ViScript-v050`
> (dirty **0** trước và sau), `VIS_GATE_MACOS_REPO` + `VIS_REMOTE_MPS_REPO` trỏ clone này.
> `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0`, `TMPDIR=/Users/mps/tmp`.
> **Không cập nhật `HOST-BAO-DAM.md` / `ROADMAP.md` / `ACTIVE.md`** — không có cơ sở công bố Tier 1.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| 3× `bin/vis-gate-macos --full` | **0/3 PASS** — cả 3 lần **rc=101**, cùng một ca: `vis-cli --test async_it` → `lat256_net::t3_peer_reads_slowly_both_engines` (`server bytes i: Timeout`) |
| Wall mỗi lần | **329 s / 316 s / 316 s** (dừng ở bước `cargo test --workspace --no-fail-fast`) |
| Phần xanh trong mỗi lần | `./vis-dev check` PASS; `./vis-dev gate --fast` PASS — **nextest 3.745/3.745** (3 skip, 82 s, **t3 PASS 13,3 s**), doc-test PASS |
| Phần đỏ | libtest `cargo test --workspace`: `async_it` **338 passed / 1 failed / 3 ignored**, đúng `t3` |
| Soak 4 h | **KHÔNG chạy** (điều kiện tiên quyết 3× full chưa đạt; time-box 5 h không còn đủ) |
| `bin/vis-wasm-sem` | **KHÔNG chạy** (cùng lý do) |
| Tier 1 | **KHÔNG công bố lại** — giữ nguyên trạng thái đã công bố ở `d511763`/`c4337ce` (không có bằng chứng mới cho `8906c70`) |
| Blocker | **Có** — xem §5 |

## 1. Đồng bộ clone đo

- `/Users/mps/dvchd/ViScript-v050` đã có sẵn đúng **`8906c70`**, `git status --porcelain` **0** dòng
  (kiểm lại sau toàn bộ chẩn đoán: vẫn **0**).
- `VIS_GATE_MACOS_REPO=/Users/mps/dvchd/ViScript-v050`,
  `VIS_REMOTE_MPS_REPO=/Users/mps/dvchd/ViScript-v050`; log `vis-remote` ghi
  `# cau_hinh: host=mps dirty_policy=check repo=/Users/mps/dvchd/ViScript-v050`.
- `target/gate-macos` tái dùng từ lần trước (3,9 GiB); đĩa trống **25–33 GiB** trong suốt đợt đo.

## 2. Ba lần `bin/vis-gate-macos --full`

| Lần | UTC | Wall | `vis-dev check` | nextest (L0) | doc-test | libtest `async_it` | rc |
|---|---|---:|---|---|---|---|---:|
| 1 | 14:17Z | **329 s** | PASS | **3.745/3.745** (3 skip) | PASS | **338/1/3 — t3 FAILED** | 101 |
| 2 | 14:25Z | **316 s** | PASS | **3.745/3.745** (3 skip) | PASS | **338/1/3 — t3 FAILED** | 101 |
| 3 | 14:38Z | **316 s** | PASS | **3.745/3.745** (3 skip) | PASS | **338/1/3 — t3 FAILED** | 101 |

- Lỗi y hệt nhau ở cả 3 lần:
  ```
  test lat256_net::t3_peer_reads_slowly_both_engines ... FAILED
  thread '...' panicked at crates/vis-cli/tests/async_it/lat256_net.rs:570:10:
  server bytes i: Timeout
  ```
  (`lat256_net.rs:570` = `rx1.recv_timeout(Duration::from_secs(8)).expect("server bytes i")`.)
- Bước sau (`cargo test --manifest-path vis-host-wasm/Cargo.toml`, `bin/vis-wasm-suite --build
  --runtime node`) **không tới được** vì `set -euo pipefail` dừng ở lỗi trên.
- **Lưu ý bằng chứng**: `vis-remote` **ghi tiếp** vào bản sao log cục bộ, nên tệp
  `st300m3-final5-gate-full-2-3.log` chứa **2 lần chạy** (lần 2 từ dòng 1–8.662, lần 3 từ 8.663–17.329).
  Lần 1 để riêng ở `st300m3-final5-gate-full-attempt1.log` (đặt tên cũ trước khi phân loại;
  nội dung là lần chạy 14:17Z, **không** phải một “flake” ngẫu nhiên — xem §3).

## 3. Chẩn đoán ca đỏ (phân loại trước khi kết luận)

### 3.1 Không phải lỗi sản phẩm do diff `c4337ce..8906c70`

`git diff --stat c4337ce..8906c70`: **không** tệp nào thuộc đường `net`/`async`/`frame` runtime.
Thay đổi mã gồm ST-309 (`vis-host-native/src/quyen_mo.rs` fallback Darwin, `vis-plugin-abi/cap_fs.rs`,
test bảo mật mới, `st309_fuzz_verify`) + tài liệu/R36. `lat256_net.rs` **không đổi** so với mốc
`c4337ce` (nơi `ST-300m3` final3 đã PASS 3/3).

### 3.2 Không phải “flake do tải” — tái hiện **tất định**, kể cả khi máy rảnh

| Thí nghiệm (cùng HEAD, cùng target) | Kết quả |
|---|---|
| `t3` **một mình**, `--exact`, 5 lần, 14:23Z (load 2,6) | **PASS 5/5** (~16 s/lần) |
| `t3` **một mình**, `--exact`, từ 14:45Z (load 1,3–4,7) | **FAIL 12/12** (8,15 s/lần) |
| `async_it` đầy đủ, `--test-threads=12` | 1 PASS / 3 FAIL |
| `async_it` đầy đủ, `--test-threads=4` | **FAIL 3/3** |
| `lat256_net` (9 ca), `--test-threads=1/2/3` | **FAIL 3/3** mỗi mức |
| `t3` sau khi **buộc biên dịch lại** (`touch lat256_net.rs`) | **FAIL 3/3** |
| nextest (mỗi ca một tiến trình), 3 lần trong cổng | **PASS 3/3** (13,3 s) |

- `8,15 s ≈ 8 s` của `recv_timeout` ⇒ hai engine `chay_i`/`chay_v` trả về **trước khi ghi byte nào**,
  rồi test chờ đủ 8 s.
- `sample` tiến trình test (khi đang chờ): luồng test nằm trong
  `std::sync::mpsc::Receiver::recv_timeout`; **hai luồng server** nằm trong `__recvfrom` — tức
  đã `accept()` xong và **không nhận được byte nào**. Không có luồng nào quay cuồng/CPU-bound.
  (`st300m3-final5-t3-direct.sample.txt`, 284 dòng.)
- Vì `nextest` (cùng mã, cùng binary, mỗi ca một tiến trình) **PASS**, còn libtest **FAIL** — kể cả
  khi chỉ chạy đúng ca đó — khác biệt nằm ở **môi trường/tiến trình của libtest**, không ở logic
  `net.write`. **Chưa khoanh được nguyên nhân gốc trong time-box** (xem §5).

### 3.3 Môi trường máy đo: một dịch vụ crash-loop đã được xử lý

- Phát hiện `com.deepseek.dsh.web` (launchd) **crash-loop**: PID cũ **77868** (sống 6 ngày, orphan,
  PPID 1) vẫn giữ cổng `100.69.232.75:3080`, nên instance do launchd quản lý **không bind được**,
  chết và được spawn lại mỗi ~2 s ở ~95–100 % CPU (quan sát 23:34–23:36 JST).
- **Đã xử lý**: `kill 77868` lúc **14:35Z**. Sau đó **đúng một** tiến trình `dsh web` (PID 46265)
  listen trên 3080, không còn vòng lặp; dịch vụ của người dùng **hoạt động lại bình thường**.
  Không tắt dịch vụ nào khác. Load 1 phút giảm 4,8 → **1,3**.
- `XProtectRemediatorPirrit` chạy 14:31–14:53Z (~100 % CPU lúc đầu) rồi kết thúc; **sau khi nó xong
  và máy rảnh (load 1,3), ca `t3` vẫn FAIL 3/3** ⇒ không quy được cho tải nền.
- Không có soak/cổng nào khác chạy song song; `bin/vis-lock` được dùng trong mọi lần cổng.

## 4. Vì sao không chạy soak 4 h / `vis-wasm-sem`

- Điều kiện của ST-300m3 là **3× full PASS liên tiếp** rồi mới soak; cổng đỏ **tất định** ở cùng một
  ca nên soak 4 h sẽ không chứng minh được gì cho tuyên bố Tier 1 (và tốn 4 h của time-box).
- Time-box **5 h** (14:17Z → 19:17Z): sau ~50 phút chẩn đoán, kể cả nếu cổng xanh ngay thì
  3 cổng (~16 phút) + soak 4 h + `vis-wasm-sem` vẫn vượt hạn. **Dừng theo luật “fail dừng”.**

## 5. Blocker & khuyến nghị

**Blocker (T-INFRA/harness, chưa phải lỗi sản phẩm đã chứng minh):** tại `8906c70`, trên `mps`,
`lat256_net::t3_peer_reads_slowly_both_engines` **đỏ tất định dưới libtest** (kể cả chạy một mình,
sau rebuild, máy rảnh), trong khi **nextest xanh**. Vì `bin/vis-gate-macos --full` bắt buộc bước
`cargo test --workspace`, cổng **không thể xanh** ở HEAD này ⇒ **không thể tái chứng nhận Tier 1**.

Khuyến nghị cho lượt kế (cần A0/Agent 2 quyết, **không tự sửa mã trong lượt đo**):

1. **Khoanh nguyên nhân gốc** bằng cách so môi trường tiến trình nextest vs libtest (biến môi trường,
   thứ tự khởi tạo, `mio::Poll`/kqueue, `FrameConfig`), và in `FrameOutput.stdout/diagnostics` của
   `chay_i` trong ca `t3` (hiện test chỉ báo timeout ở `recv_timeout`, che mất lỗi thật của chương trình).
2. Nếu là lỗi **cô lập test** (state tĩnh/`mio` dùng chung): sửa test hoặc tách ca; nếu là lỗi
   **sản phẩm** ở đường frame in-process: mở nợ riêng và đánh giá lại phạm vi Tier 1 (CLI vẫn xanh).
3. Chỉ sau khi cổng xanh **3/3** mới chạy soak 4 h + `vis-wasm-sem` và cập nhật
   `HOST-BAO-DAM.md` / `ROADMAP.md` / `ACTIVE.md`.

**Không tuyên bố gì mới về Tier 1 cho `8906c70`.** Trạng thái đã công bố trước đó (macOS arm64 Tier 1
@ `c4337ce`, `d511763`) **giữ nguyên**, kèm ghi chú rằng lần tái chứng nhận tại `8906c70` **chưa đạt**.

## 6. Bằng chứng ngoài repo (sha256)

| Tệp | Vị trí | sha256 |
|---|---|---|
| Cổng lần 1 (14:17Z) | `ps:/home/ps/vislogs/st300m3-final5-gate-full-attempt1.log` | `56ff19d8620924c19c34f59903a9834d256f34861db0c6df4dd1e1c50270d83f` |
| Cổng lần 2+3 (14:25Z, 14:38Z) | `ps:/home/ps/vislogs/st300m3-final5-gate-full-2-3.log` | `8f94d606443f10dfa0ac8a97cf1a499626e11502ff8eabfc3256d9c9858a763e` |
| `t3` một mình (log) | `mps:/Users/mps/vislogs/st300m3-final5-t3-alone-run1.log` | `8d92bc9066cf41835bea441d11c80dae743093f5b1753bfc8d539ef7ac20f80a` |
| `t3` một mình (sample) | `mps:/Users/mps/vislogs/st300m3-final5-t3-alone-run1.sample.txt` | `7833b31e842bf167c448e858c79d4b9430e3c85245cd18af71b89f11433d3887` |
| `t3` chạy binary trực tiếp (log) | `mps:/Users/mps/vislogs/st300m3-final5-t3-direct.log` | `d40bd0a43b55ebd329b7f31c3203326ac33125e17520cf7fde792288df47f280` |
| `t3` chạy binary trực tiếp (sample) | `mps:/Users/mps/vislogs/st300m3-final5-t3-direct.sample.txt` | `03e79268c49818fe473deb5e1fe69164d62c57e1bf62cbe05ab0b26db182d9aa` |
| Log gốc `vis-remote` lần 1 | `mps:/Users/mps/vislogs/vis-remote-20261123T141706Z.log` | `41ae6c980cfa9ae383ea7efb6c70ec76a666d4c2c03bb05a3b3f20d3c90204f0` |
| Log gốc `vis-remote` lần 2 | `mps:/Users/mps/vislogs/vis-remote-20261123T142516Z.log` | `2fa59850d290904efceb9fd972f7a049fc652a96e1ab16c590d23a8e2341e576` |
| Log gốc `vis-remote` lần 3 | `mps:/Users/mps/vislogs/vis-remote-20261123T143824Z.log` | `96144d524cb0127b206f02c522798e8b57c42aa8d03ad7658012c6fb27c7f247` |

## 7. Lệnh tái lập

```bash
# trên ps
export VIS_GATE_MACOS_REPO=/Users/mps/dvchd/ViScript-v050
export VIS_REMOTE_MPS_REPO=/Users/mps/dvchd/ViScript-v050
VIS_GATE_MACOS_LOG=$HOME/vislogs/st300m3-final5-gate-full-1.log bin/vis-gate-macos --full
# ⇒ rc=101 tại vis-cli --test async_it / lat256_net::t3_peer_reads_slowly_both_engines

# trên mps — ca đỏ, một mình
cd /Users/mps/dvchd/ViScript-v050
CARGO_TARGET_DIR=$PWD/target/gate-macos cargo test -p vis-cli --test async_it \
  -- lat256_net::t3_peer_reads_slowly_both_engines --exact --test-threads=1
# ⇒ FAILED (8,15 s) trên máy rảnh; nextest cùng ca: PASS
```
