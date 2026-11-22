<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296 — `vis-gate-windows --full` trên `5a38680` (baseline trước ST-292)

> **Người đo**: subagent VS3-W4, 2026-11-23 · time-box 1,5 h.
> **Mốc**: `main` **`5a38680545141e501c513ab2f174c7634d962083`**
> (`merge ST-296: ACTIVE docs ST-291/299a/296d`; Linux gate 3638/4).
> `psvn` worktree **riêng** `/c/Users/ps/viscript-w4full-5a38680` @ `5a38680`, dirty **0**.
> `TMPDIR=/c/Users/ps/tmp` (ngoài `/tmp`). Không tag/push/merge; không GitHub Actions.
> **Kết quả**: `--full` **PASS** (workspace debug **3589/0/3**, host 54/54, targets 18/18 + 17/1);
> script full **không chạy release** ⇒ đã chạy release targeted riêng: **tất cả xanh**.
> Đây là **baseline trước ST-292**, không phải final tag.

## 0. TL;DR

| Hạng mục | Kết quả | Thời gian |
|---|---|---:|
| `bin/vis-gate-windows --full` | **exit 0** — toàn bộ debug + host + targets | **682 s (11m22s)** |
| Workspace debug | **359 block; 3589 passed / 0 failed / 3 ignored** | trong 682 s |
| host (debug, trong full) | **54/54** (1..54, 120 lần chạy, VM 54/54) | 3,8 s |
| targets | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** | — |
| Release targeted (riêng) | P2 **14/14**, P1 **8/8**, P3 **10/10**, select **25/25**, h2 **33/33**, cancel **8/8** | ~98 s tổng |
| Release build/CLI | `vis.exe` **11 468 288 B**, sha256 `73ee2534…f57358`; `vis 0.3.6` | 0 s (artifact sẵn) |
| Blocker | **Không** | — |

## 1. Chạy đúng script `--full`

Lệnh (env trỏ worktree riêng, tránh repo chung):

```bash
VIS_REMOTE_PSVN_REPO=/c/Users/ps/viscript-w4full-5a38680 \
VIS_GATE_WINDOWS_REPO=/c/Users/ps/viscript-w4full-5a38680 \
VIS_GATE_WINDOWS_TMPDIR=/c/Users/ps/tmp \
bin/vis-gate-windows --full
```

- Script chạy: `cargo test --workspace --no-fail-fast` + `host_conformance` + `vis-targets`
  trên worktree sạch (`--require-clean` thoả).
- **exit 0**, tổng **682 s**; `vis-remote-exit=0`.
- Workspace: **359 block; 3589 passed / 0 failed / 3 ignored** (không dòng `FAILED`).
- host: `1..54`, **54 ca / 120 lần chạy**, 0 lệch, VM **54/54**.
- targets: **core 18/18**, **sqlite 17 PASS + 1 EXPECTED_RED**, 0 FAIL.
- Log local `/tmp/vis-gate-windows-full-5a38680.log` sha256
  `c8595adc53e792db66bd386e5015c39d1a7a0f24c6c4c6d2973c04f65d78eb03`;
  bản máy đích sha256 `f588c3880939293d66b41f3986853fec6acca8a71a28a77e59887cba8b7e521d`.

## 2. Script `--full` KHÔNG chạy release

Hiện `bin/vis-gate-windows --full` chỉ chạy **debug** workspace + host + targets. Theo yêu cầu,
chạy bổ sung **release targeted** trên cùng worktree:

| Suite (release) | Kết quả |
|---|---|
| `vs3_worker_eval_parallel` (P2) | **14/14** |
| `vs3_worker_eval_local` (P1) | **8/8** |
| `vs3_worker_eval_vm` (P3) | **10/10** |
| `vs3_select` | **25/25** |
| `vs3_http2` | **33/33** |
| `vs3_http_cancel` | **8/8** |

- Tổng thời gian release targeted ~**98 s** (P2 70 s gồm build; các suite còn lại 2–20 s).
- Release build: `cargo build --release -p vis-cli --bin vis` rc 0 (artifact tái dùng);
  `target/release/vis.exe` **11 468 288 B**, sha256
  **`73ee2534233f403609af48fbf886625dafd9d95d893d53661157a0bc8fa57358`**.
- CLI release: `vis 0.3.6`; `version --json` đúng; `check` TAP `ok 1`, `run` → `42`.

## 3. Trạng thái cuối

- Worktree HEAD `5a38680` không đổi, dirty **0**; `psvn` `C:` còn **105 G**.
- Local `/` còn **17 G** (>10 G).
- Không tag/push/merge; không GitHub Actions.

## 4. Bằng chứng (log trên `psvn`)

| Log (`/c/Users/ps/vislogs/`) | sha256 |
|---|---|
| `fullrel-p2.log` | `f993683a…8fdac817` |
| `fullrel-p1.log` | `955b9cbb…0af23259cf` |
| `fullrel-p3.log` | `6ab96bec…8b58afaeec` |
| `fullrel-select.log` | `f2f167cb…6a40fd286a` |
| `fullrel-h2.log` | `d0c78470…ff4ebef` |
| `fullrel-cancel.log` | `061f5cbf…cf5d3dfc` |
| `fullrel-build.log` | `1d4a4590…6c48d277` |
| bản remote của full log | `f588c388…8b7e521d` |

## 5. Kết luận / blocker

- `5a38680` — **baseline trước ST-292** — đạt `--full` trên Windows (debug) + host/targets +
  release targeted; version **0.3.6** nhất quán.
- **Không blocker.** Chưa tag/push/merge; đây **không phải** final tag `v0.4.0` (còn ST-292).
