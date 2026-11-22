<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Final Windows — verify commit chốt `a1930cb` (`v0.3.6`)

> **Người đo**: subagent VS3-W4 (Gatekeeper phụ), 2026-11-22.
> **Mốc**: `a1930cb` = merge `ST-289 QA: final v0.3.6 4c5dec6` (code `4c5dec6` + journal QA;
> Linux final QA PASS). `psvn` `/c/Users/ps/viscript-moi` **detached `a1930cb`**, dirty **0**;
> `git describe --tags --always` = `v0.3.5-34-ga1930cbe`, **chưa có tag `v0.3.6`**.
> **Kết luận ngắn: CHƯA NÊN TAG** — `vs3_http_cancel` h1 **flake trên Windows** (xem §4);
> workspace lần 2 đạt **3538/0/3** đúng kỳ vọng. Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| HEAD / describe | `a1930cbe84231c2362626dca9bd919a906fab0b9`; `v0.3.5-34-ga1930cbe`; dirty 0 |
| `vis --version` | **`vis 0.3.6`** (debug + release); JSON `{"name":"vis","version":"0.3.6"}`; doctor 0.3.6 |
| Workspace lần 1 | **3533 passed / 5 failed / 3 ignored** (528 s) — 5/5 `vs3_http_cancel` h1 **FAILED** |
| Workspace lần 2 | **3538 passed / 0 failed / 3 ignored** (463 s) — **đúng kỳ vọng 3538/0/3** |
| `vs3_http_cancel` default | lần đầu **5/5**; sau đó **flake**: 2/9 lần suite fail (4/5), 1/5 lần `--test-threads=1` fail; ca riêng ×10 **10/10 pass** |
| `vs3_http_cancel --features async-h2` | **BLOCKED-ENV rc 101** (`E0433 net::h2_async`) — async-h2 **Unix-only**, không build được trên Windows; **không phải 8/8** |
| P2 / P1 / P3 | **14/14**, **8/8**, **10/10** |
| h2 / pkg / sig / default / TLS / net | **19/19**, **25/25**, **10/10**, **22/22**, **22/22**, **9/9** |
| host / targets | **54/54** (120 lần chạy, VM 54/54); core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 228 672 B**, sha256 `fa363043…5d3b08b`; CLI/smoke xanh |
| `cargo check --features async-h2` | **rc 101 EXPECTED** (Unix-only) |
| Blocker | **`vs3_http_cancel::cancel_http_h1_lap_nhieu_vong_khong_ro` flake** (product: task bị huỷ vẫn chạy tiếp) |

## 1. Môi trường / mốc

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC).
- `git fetch origin main` → checkout `--detach a1930cb`; dirty 0 trước và sau đo; prebuild 14 s.
- `C:` còn **172 G**; local `ps` (`/`) còn **9,2 G** (>5 G).
- **Soak 1h tại `f80f774`** (lệnh trước): lệnh dừng không tới được tiến trình detached ⇒ soak vẫn
  chạy đủ **160 vòng / 3606 s, 0 fail, 0 flake**, free RAM **8727–9719 MB**, host **54/54**,
  targets **18/18 + 17/1** (`/c/Users/ps/vislogs/stage0-soak-summary.log`). 10 phút cuối soak
  **chồng lấn** workspace lần 1 (xem §3); soak không chạy `vs3_http_cancel` (mốc `f80f774` trước ST-287).

## 2. Workspace (1 vòng ×2)

| Lần | Lệnh | Kết quả | Thời gian |
|---|---|---|---:|
| 1 | `cargo test --workspace --no-fail-fast` | **355 block; 3533 passed / 5 failed / 3 ignored** | 528 s |
| 2 | `cargo test --workspace --no-fail-fast` | **355 block; 3538 passed / 0 failed / 3 ignored** | 463 s |

- Lần 1 đỏ đúng **1 target**: `vs3_http_cancel` **0/5** (`cancel_http_h1_*`), cùng thông điệp
  `task bị huỷ phải dừng: … V:post …` — thời điểm chạy **chồng lấn 10 phút cuối soak** `f80f774`
  (cùng máy, cùng target dir) nên bị nghi nhiễu tải/port; **không** đại diện duy nhất cho mã.
- Lần 2 (sau khi soak kết thúc) đạt **3538/0/3** — khớp kỳ vọng (+5 ca ST-287 so 3533).
- 3 ignored vẫn là các ca đo lường có `#[ignore]` (`lat256_budget`, `lat257_b_vm_steps`,
  `lat257_fairness`).

## 3. `vs3_http_cancel` — flake h1 trên Windows

Lệnh: `cargo test -p vis-cli --test vs3_http_cancel` (5 ca h1). Log đỏ:
`/c/Users/ps/vislogs/final-cancel1.log`, `final-cancel-loop1.log`, `final-cancel-st2.log`.

| Lần chạy | Kết quả |
|---|---|
| default lần đầu (isolated) | **5/5 pass** |
| targeted lần 1 | **4/5 FAIL** — `cancel_http_h1_lap_nhieu_vong_khong_ro` |
| targeted lần 2, 3 | 5/5 pass |
| loop1..loop5 | **4/5 FAIL** ở loop1; loop2–5 5/5 pass |
| `--test-threads=1` ×5 | **st2 4/5 FAIL**; st1/st3/st4/st5 5/5 pass |
| ca flake riêng ×10 | **10/10 pass** |

- **Tổng: 3 lần suite đỏ / 14 lần suite** chạy sau soak (≈21 %), **kể cả khi ép tuần tự** ⇒ không
  phải chỉ do libtest chạy song song; ca riêng lẻ 10/10 xanh ⇒ cần tương tác với ca chạy trước.
- Thông điệp đỏ (vs3_http_cancel.rs:244, `kiem_sau_huy`):
  ```
  task bị huỷ phải dừng: "main\nV:pre\nloi: lỗi đọc `C2`: An established connection was aborted
  by the software in your host machine. (os error 10053)\nV:post\n"
  ```
  Tức task đã bị huỷ nhưng nhánh lỗi socket Windows (WSAECONNABORTED 10053) **vẫn chạy tiếp** tới
  `V:post`, thay vì dừng ở safe point. Đây là **hành vi sản phẩm** (ST-287), không chỉ là test.
- Linux final QA đạt 3581/3 và ST-287 journal ghi 5/5 h1 + 8/8 h2 trên Linux; khác biệt nằm ở
  timing/đường lỗi socket Windows.
- **`--features async-h2` 8/8 không thể đo trên Windows**: `cargo test --features async-h2` →
  **rc 101** `E0433: cannot find h2_async in net` (async-h2 Unix-only). Yêu cầu 8/8 chỉ áp dụng Linux.

## 4. Targeted khác (debug, trừ ghi chú)

| Suite | Kết quả |
|---|---|
| `vs3_worker_eval_parallel` (P2) | **14/14** |
| `vs3_worker_eval_local` (P1) | **8/8** |
| `vs3_worker_eval_vm` (P3) | **10/10** |
| `vs3_http2` | **19/19** |
| `pkg` | **25/25** |
| `vs3_registry_sig` (`--features pkg-sig`) | **10/10** |
| `vs3_default` | **22/22** |
| `vs3_tls` | **22/22** (Windows có thêm 1 ca `#[cfg(windows)]`; Linux 21) |
| `net` | **9/9** |

## 5. Host / targets / release / CLI

- host: **54/54** (`1..54`, 120 lần chạy interp+VM, 0 lệch; VM 54/54).
- targets: **rc 0** — core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED, 0 FAIL**.
- async-h2 check: **rc 101 EXPECTED** (`E0433 net::h2_async`).
- release: `cargo build --release -p vis-cli --bin vis` rc 0 (36 s);
  `target/release/vis.exe` **11 228 672 B**,
  sha256 **`fa3630435ef2dfd37e87ff83e4cffd085dbd5eb9e9e1643b075206f8b5d3b08b`**.
- CLI debug + release: `vis 0.3.6`; `version --json` đúng; `doctor` `os windows`, `arch x86_64`,
  `features sqlite,tls,pkg-sig,http2`; smoke `check` TAP `ok 1`, `run` → `42`, `run --version` rc 2.

## 6. Log / bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `final-workspace.log` (đỏ lần 1) | `269d79fd1c621ea0494f53ec9001571bf3c3220631c61fc4c3bfff76b5fbc67e` |
| `final-workspace2.log` (xanh 3538/0/3) | `eff5889829b027ff3fda0372d01a1cdb0d7500e8c840151e65e41b72294e44fb` |
| `final-cancel-default.log` (5/5) | `3794a23dca8f98d49ed0990e551f979fb81e794c2431300d3c8a70defb59db0b` |
| `final-cancel1.log` (4/5 fail) | `ead4a6ec439665b58ba76e4b2c2541ebc907f103f01e1d9cf298fed339790f5b` |
| `final-cancel-loop1.log` (4/5 fail) | `fc3cb918486d304d3c33c07d80f6574862433a7e7f21d101e592ec604db3a216` |
| `final-cancel-st1.log` / `final-cancel-st2.log` (fail) | `2ecc2d57…c35419` / `dfb333c8…a0be1c` |
| `final-flake1.log` (ca riêng 1/1) | `71e7975a0f70450d599f2f99395b914ce66e55ba24b3dfca89ebf56ab488705b` |
| `final-host.log` | `5dfa98cc72c395f3105a8b66d180d9f5a21caa6a5dc455d12576f622a015fe3f` |
| `final-targets.log` | `8f824cc9e20cd9f217aa8a5c27ca77a0eb712a276814b6e0690001c0bd889df3` |
| `final-release.log` | `fc1faf754b555d2d925019086625d4b00f6e34a3f445c677d6656c705ce1cf7c` |
| `final-async-h2.log` | `e041f0f0e70a58474df7274661dd530750958ea8fff13605b4afe4890e16c7a8` |

- `psvn` HEAD `a1930cb`, dirty 0 sau đo; `C:` còn 172 G; local `/` còn 9,2 G.

## 7. Đề xuất tag `v0.3.6`

**KHÔNG tag ngay** ở trạng thái hiện tại, vì:

1. `vs3_http_cancel` h1 — tính năng mới của ST-287 — **flake ~21 % số lần chạy suite trên Windows**,
   kể cả `--test-threads=1`; lỗi là **task bị huỷ vẫn chạy tiếp** (in `V:post`) sau lỗi socket
   Windows `10053`. Không thể tuyên bố "Windows xanh" cho commit chốt.
2. Workspace chỉ đạt 3538/0/3 ở **lần chạy thứ 2**; lần 1 đỏ 5 ca (dù bị nhiễu bởi soak chồng lấn,
   vẫn cho thấy cổng chưa tất định).

**Đường hợp lệ để tag**: sửa/triage flake h1 Windows (kiểm tra thứ tự safe-point vs nhánh lỗi
socket trong `frame_cancel_one`/`AsyncNet`) rồi chạy lại `vs3_http_cancel` ×10 + workspace 1 vòng.
**Nếu A0 buộc tag** vì Windows chỉ là Tier 2/không hỗ trợ: chỉ tag sau khi (a) ghi rõ known-issue
`cancel_http_h1_lap_nhieu_vong_khong_ro` flake trên Windows trong release notes, (b) **không** tuyên
bố Windows xanh, (c) giữ Linux final QA 3581/3 làm bằng chứng chính.

- Không tag/push/merge trong phiên này; không GitHub Actions.
