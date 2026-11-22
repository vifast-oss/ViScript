<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — fix cancel-before `1edec5d` trên Windows (+ dừng soak 4h v0.3.4)

> **Người đo**: subagent VS3-W4, 2026-11-22.
> **Bối cảnh**: soak 4h trên tag `v0.3.4` (`8fa1f96`) đã **dừng ở vòng 1150** vì flake
> test-only `net.rs::tcp_het_gio_doc` (timeout ≤400 ms), không phải cancel race. Main
> **`1edec5d`** sửa race cancel-before (`WorkerPool::spawn_inner` đặt cờ huỷ trước enqueue)
> ⇒ chuẩn bị `v0.3.5`. Verify trên `psvn`, time-box 1h.
> **Kết quả**: P2 **10× 14/14** (gồm `cancel_pre_run_tra_loi_ro`), P1 8/8, P3 10/10 debug+release,
> workspace **3528/0/3**, targeted 3 vòng, host/targets, release/CLI đạt.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Mốc | **`1edec5d`** (`merge VS3 W5ai: fix cancel-before race`), worktree sạch |
| P2 `vs3_worker_eval_parallel` ×10 | **10/10 lần 14/14**, `cancel_pre_run_tra_loi_ro` ok mỗi lần |
| P1 `vs3_worker_eval_local` | **8/8** |
| P3 `vs3_worker_eval_vm` debug / release | **10/10** / **10/10** |
| Workspace | **353 block; 3528 passed / 0 failed / 3 ignored** — 548 s |
| Targeted default/tls/net ×3 | **22/22 + 22/22 + 9/9** mỗi vòng |
| host / targets | **54/54**; core **18/18**, sqlite **17 + 1 EXPECTED_RED** |
| Release | `vis.exe` **11 205 120 B**, sha256 `c8010cfa…bf1ca` |
| CLI | `--version` `vis 0.1.0`; doctor 4 feature; check TAP ok; run `42`; run --version rc 2 |
| async-h2 Windows | **rc 101 — EXPECTED** (Unix-only) |
| Blocker | **Không có** |

## 1. Dừng soak 4h trên tag `v0.3.4`

- Chạy **1150 vòng / 10 189 s (~2h50m)** thì dừng đúng luật "fail là dừng":
  - Extra `default_tls_net_1150` rc **101**; `net` 8/9, `tcp_het_gio_doc` FAILED tại
    `crates/vis-cli/tests/net.rs:210` — assertion `dt.as_millis() <= 400` ("**treo** … phải ≤ 400 ms").
- **Phân loại: test-only timing**, không phải cancel-before race:
  - Ca đo thời gian đọc timeout trên Windows dưới tải soak dài; không liên quan `WorkerPool`.
  - 1150 vòng trước đó `vis_proc=0`; free memory các REPORT dao động ~9,75–10,08 GB (không đơn điệu);
    cuối ghi 7,79 GB sau khi cây test dừng (dọn dẹp/OS), không có bằng chứng rò sản phẩm.
  - host **54/54**, targets **18/18 + 17/1** vẫn xanh ngay sau đó.
- Log trích: `agent/JOURNAL/vs3-w4-tag4h-v034-partial.log`; full
  `psvn:/c/Users/ps/vislogs/w4o-soak.log` (sha256 `70639c90…63c59`).
- Tag `v0.3.4` vẫn chứa race **cancel-before** (đã biết) ⇒ cần `v0.3.5`; flake timeout ở trên là
  nợ test-only riêng, nên theo dõi khi soak 4h lại.

## 2. P2 10 lần (`vs3_worker_eval_parallel`)

| Lần | Kết quả | `cancel_pre_run_tra_loi_ro` | Thời gian |
|---|---|---|---:|
| 1 | **14/14** | ok | 44 s (build) |
| 2 | **14/14** | ok | 3 s |
| 3 | **14/14** | ok | 3 s |
| 4 | **14/14** | ok | 2 s |
| 5 | **14/14** | ok | 2 s |
| 6 | **14/14** | ok | 2 s |
| 7 | **14/14** | ok | 3 s |
| 8 | **14/14** | ok | 2 s |
| 9 | **14/14** | ok | 2 s |
| 10 | **14/14** | ok | 2 s |

- Không lần nào fail/timeout; đặc biệt ca pre-cancel xanh cả 10 lần.
- P1 **8/8**; P3 debug **10/10**; P3 release **10/10** (build 73 s).

## 3. Workspace + targeted

| Lệnh | Kết quả | Thời gian |
|---|---|---:|
| `cargo test --workspace --no-fail-fast` | **353 block; 3528 passed / 0 failed / 3 ignored** | 548 s |
| `vs3_default + vs3_tls + net` ×3 | **22/22 + 22/22 + 9/9** mỗi vòng | 11–17 s/vòng |

- Không `FAILED`/`panicked`; `10053` không tái hiện.

## 4. Host / targets / release / CLI

- host **54/54**; targets core **18/18**, sqlite **17 PASS + 1 EXPECTED_RED** (#156).
- `target/release/vis.exe`: **11 205 120 B**,
  sha256 `c8010cfad518930373f41dfb7e5052a2b6a407c2567edff2366f3a9e752bf1ca`.
- `vis --version` → `vis 0.1.0`; `doctor` → `os windows`, `arch x86_64`, `sqlite on`,
  `features sqlite,tls,pkg-sig,http2`.
- Smoke: `check` → TAP `1..1 ok 1`; `run` → `42`; `run --version` → rc 2.

## 5. `--features async-h2` trên Windows

`cargo check -p vis-cli --features async-h2` → **rc 101** (`h2_async.rs:15` `cfg(…, unix)`) —
**EXPECTED**: async-h2 vẫn Unix-only/default-off.

## 6. Log / bằng chứng

| Log `psvn` | sha256 |
|---|---|
| `/c/Users/ps/vislogs/w4p-targeted.log` | `4ba847c97ec21e4dcc7eb1f9d89dc35e6b7d07c748a3f6a4ed2b8a0296c8b87d` |
| `/c/Users/ps/vislogs/w4p-workspace.log` | `b4fffb0718504831ef91c194c25840ea7045429ae064ee87381e6d14b7af2f9c` |
| `/c/Users/ps/vislogs/w4p-release.log` | `b6d7877a46b602db64bf6c4f2be11729b8ad9b148c5c103706e98c411ba66b24` |
| `/c/Users/ps/vislogs/w4o-soak.log` (4h partial) | `70639c9003fa83ded3b4fb99777a7bf7e82ec2c84e808a3f91cfe98f5fc63c59` |

- Worktree `w4p` và `w4o` đã xoá sau khi thu log; flake 4h lưu đầy đủ trong log trích + bản gốc `psvn`.
- repo chính `psvn` @ `73af17bd`, dirty 0; `C:` còn **171G**.
- Local `/` còn **21G** (>5G).

## 7. Kết luận

- **`1edec5d` sửa cancel-before đạt trên Windows**: P2 10/10 lần 14/14 (pre-cancel xanh),
  workspace 3528/0/3, targeted 3 vòng, host/targets, release/CLI đạt.
- Soak 4h trên `v0.3.4` dừng vì **flake timeout test-only** `tcp_het_gio_doc`, không phải race;
  cần theo dõi/nới biên hoặc sửa test khi soak 4h lại trên `v0.3.5`.
- **Không blocker** cho `v0.3.5`; async-h2 vẫn Unix-only.
