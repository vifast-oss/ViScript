<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Stage 0 — verify Windows `0d057e5` (`v0.3.6`, R32 khung)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`0d057e5e371998f5181bcb1e5ce996ea2663d0b6`** (`merge ST-289: R32 Stage 0 report
> (4/6, cho ST-285/287)`; version **0.3.6**; Linux gate **3576/3**).
> `psvn` worktree `/c/Users/ps/viscript-moi` **detached đúng `0d057e5`**, `git status --porcelain` **0**.
> **Kết quả**: workspace **3533 / 0 / 3** (354 block, 534 s); P2 **14/14**, P1 **8/8**, P3 **10/10**
> debug + release; default h2 **19/19**; `pkg` **25/25**; `vs3_registry_sig` **10/10**;
> `vs3_default` **22/22**; `vs3_tls` **22/22** (Windows có thêm 1 ca `#[cfg(windows)]`); `net` **9/9**;
> host **54/54**; targets core **18/18**, sqlite **17 + 1 EXPECTED_RED**; release build/CLI OK.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| HEAD | **`0d057e5e…`** = `0d057e5`; dirty **0** |
| Workspace 1 vòng | **354 block; 3533 passed / 0 failed / 3 ignored** — **534 s** |
| Version | `vis 0.3.6`; JSON `{"name":"vis","version":"0.3.6"}`; `doctor` in `vis 0.3.6` |
| P2 / P1 / P3 | **14/14** debug + **14/14** release; P1 **8/8**; P3 **10/10** debug + **10/10** release |
| Targeted | h2 **19/19**; `pkg` **25/25**; `vs3_registry_sig` **10/10**; `vs3_default` **22/22**; `vs3_tls` **22/22**; `net` **9/9** |
| host / targets | **54/54** (1..54, 120 lần chạy, VM 54/54); core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 211 776 B**, sha256 `eede7c57…a49442`; CLI release `0.3.6`; `check`/`run` OK |
| `cargo check --features async-h2` | **rc 101 — EXPECTED** (Unix-only, `net::h2_async` không có trên Windows) |
| Blocker | **Không có** (1 sai lệch kỳ vọng nhỏ: `vs3_tls` Windows 22 ≠ Linux 21 — xem §4) |

## 1. Môi trường

- `psvn`: Windows `MINGW64_NT-10.0-26300`, 6 CPU; cargo/rustc **1.100.0** (MSVC native).
- Repo `/c/Users/ps/viscript-moi`: `git fetch origin main` → checkout `--detach 0d057e5`;
  `git describe --tags --always` = `v0.3.5-16-g0d057e5e`; dirty **0** trước và sau đo.
- `C:` còn **172 G**; `vis-remote` ghi log ngoài repo (`/c/Users/ps/vislogs/`).

## 2. Workspace (1 vòng)

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **354 block; 3533 passed / 0 failed / 3 ignored** | **534 s** |

- `rc=0`; **0** dòng `FAILED`; 3 ignored là các ca đo lường/benchmark có `#[ignore]`
  (`lat256_budget`, `lat257_b_vm_steps`, `lat257_fairness`) — như baseline.
- Log: `/c/Users/ps/vislogs/stage0-win-workspace.log` (sha256 `f2562a39…`).

## 3. Version / CLI

| Lệnh | Kết quả |
|---|---|
| `vis --version` | `vis 0.3.6` |
| `vis version --json` | `{"name":"vis","version":"0.3.6"}` |
| `vis doctor` | `vis 0.3.6`, `os windows`, `arch x86_64`, `sqlite on`, `features sqlite,tls,pkg-sig,http2` |
| `vis check a.vis` | TAP `1..1 ok 1`, rc 0 |
| `vis run a.vis` | `42`, rc 0 |
| `vis run --version` | rc **2** (đúng: cờ lệnh con không bị nuốt) |

## 4. Targeted (debug trừ khi ghi release)

| Suite | Kết quả | Thời gian |
|---|---|---:|
| `vs3_worker_eval_parallel` (P2) debug | **14/14** | 1,49 s |
| `vs3_worker_eval_parallel` (P2) release | **14/14** | 0,52 s (build 41 s) |
| `vs3_worker_eval_local` (P1) | **8/8** | 0,21 s |
| `vs3_worker_eval_vm` (P3) debug | **10/10** | 2,80 s |
| `vs3_worker_eval_vm` (P3) release | **10/10** | 1,01 s (build 3 s) |
| `vs3_http2` (default h2) | **19/19** | 15,35 s |
| `pkg` | **25/25** | 0,47 s |
| `vs3_registry_sig` | **10/10** | 0,25 s |
| `vs3_default` | **22/22** | 2,36 s |
| `vs3_tls` | **22/22** | 6,45 s |
| `net` | **9/9** | 2,63 s |

- **Sai lệch kỳ vọng (không phải lỗi)**: yêu cầu ghi `vs3_tls` **21**, thực đo Windows **22**.
  Nguyên nhân: tệp có **22** `#[test]`, trong đó 1 ca `#[cfg(windows)]` (Linux chỉ chạy 21 —
  đã kiểm `cargo test -p vis-cli --test vs3_tls -- --list` trên `ps` = 21 test). Cả 22 ca Windows **pass**.
- Không flake; thời gian build release P2 41 s (P3 dùng lại artifact).

## 5. Host / targets

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test host_conformance` | **1 test suite pass**; với `--nocapture`: `1..54`, **54 ca / 120 lần chạy (interp + VM), 0 lệch; VM 54/54** |
| `bash bin/vis-targets` | **rc 0** — **core 18/18**; **sqlite 17 PASS + 1 EXPECTED_RED, 0 FAIL** |

- `vis-targets` in thêm nhắc: có ô từng `EXPECTED_RED` (#156) nay **xanh** — chỉ là nhắc rà soát nợ
  #156, không đổi kết luận (script vẫn exit 0, sqlite 17/1 đúng cấu hình cũ).

## 6. Release / CLI / async-h2

| Hạng mục | Kết quả |
|---|---|
| `cargo build --release -p vis-cli --bin vis` | rc 0 (artifact đã có từ test release; 1 s) |
| `target/release/vis.exe` | **11 211 776 B**, sha256 **`eede7c57341e6b03d1493c74178b18fa218cd1e85fa56710e4bb69f0c6a49442`** |
| Release CLI | `--version`/`version --json`/`doctor` đều **0.3.6**; `check` TAP ok; `run` → `42` |
| `cargo check -p vis-cli --features async-h2` | **rc 101 EXPECTED** — `E0433: cannot find h2_async in net` (Unix-only) |

## 7. Log / bằng chứng

| Log `psvn` (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `stage0-win-workspace.log` | `f2562a393e736677899b01969c1860b71ecb276e59d5765e843021e84d7c85bf` |
| `stage0-p2.log` / `stage0-p2r.log` | `ddf009a0…21e5` / `a937a4f9…699f` |
| `stage0-p1.log` / `stage0-p3.log` / `stage0-p3r.log` | `35ad94d3…d5d5` / `1b0876b7…9c4b` / `c8c2aeb9…08c7` |
| `stage0-h2.log` / `stage0-pkg.log` / `stage0-sig.log` | `5433c296…f25c` / `804210d9…735c` / `937e1a1b…e97b` |
| `stage0-default.log` / `stage0-tls.log` / `stage0-net.log` | `ff66b3eb…5042` / `d534a313…5189` / `5ded5658…f47c` |
| `stage0-host-nocapture.log` | `1067e58a7194e136033d1c45fe4218b88514fc9a9258efa84738d30008b914a2` |
| `stage0-targets.log` | `8f824cc9e20cd9f217aa8a5c27ca77a0eb712a276814b6e0690001c0bd889df3` |
| `stage0-async-h2.log` | `a34f82df435bfe967d1497047ca603b6443807189f1b9874d0d5bbd2f224b394` |
| `stage0-release.log` | `d6f14fa414f78cc9fa069b992a2381a482fd9b613d9c0482bf01a39931f158e1` |

- `psvn`: HEAD `0d057e5e…`, dirty **0**; `C:` còn **172 G**.
- Local `ps` (`/`): còn **5,3 G** tại lúc ghi (>5 G; theo dõi vì sát ngưỡng).
- Log điều khiển `vis-remote`: `/tmp/psvn-stage0-*.log` (local).

## 8. Kết luận

- **`0d057e5` (`v0.3.6`, R32 khung) đạt trên Windows**: workspace **3533/0/3**, version **0.3.6**
  ở cả debug/release, toàn bộ targeted đúng kỳ vọng (P1/P2/P3, h2 19, pkg 25, sig 10, default 22,
  tls 22, net 9), host **54/54**, targets **18/18 + 17/1**, release build/CLI/smoke xanh.
- `async-h2` vẫn **Unix-only** (rc 101 expected) — đúng thiết kế.
- **Không blocker**. Hai ghi chú: (1) `vs3_tls` Windows **22** vì có ca `#[cfg(windows)]` (Linux 21);
  (2) `vis-targets` nhắc ô #156 từng đỏ nay xanh — cần A0 rà nợ #156 khi có dịp.
- **Chưa** tag/push/merge; Stage 0 vẫn chưa đóng trên `main` (ST-285/ST-287 theo R32 khung).
