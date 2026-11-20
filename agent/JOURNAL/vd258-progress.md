<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258 — tiến độ tích hợp (2026-11-20, phiên A0)

## Đã merge vào `main`

| Luồng | Commit nguồn | Merge | Nội dung |
|---|---|---|---|
| C — fee/tooling | `0f733c7`, `7e131b0` | `f1348e5` | `verify_fee` chặn OOB/panic; `for` E_LOOP khớp VM; phân loại call; toolchain 1.100.0; core test skip sqlite |
| B — frame/async | `1ae5cfd` | `f9aaeee` | `Redispatch` stack; `CallMethod`/method loop; `RedispatchMethod`; deadlock `VS0505`; closure env; const env; fake-source final join; exit code |
| D — SQLite/stdlib | `80af3b5`, `1d07072`, `6919da5` | `4638d04` | cell/overhead cap; `SQLITE_LIMIT_LENGTH`; schema UTF-8; handle cap; `json.encode` float; **`std.path` Windows**; LSP header/Interrupted/percent drive |
| A — checker/soundness | `166bc04` | `0a13b24` | `Result?`; record ctor/default; `None`; `?.` lồng; lift ctor/`with`/push; multi-file builtin trait/ctor |
| tích hợp | `723412b`, `c8e3beb`, `44844d8` | main | giữ R1 (tách `call_fee`); bump cache label; or-pattern VM bind theo alt khớp; `()` = `Unit` |

## Bằng chứng trên main

- `./vis-dev gate --fast`: **1633 passed / 3 skipped / 0 failed** (sau A + tích hợp).
- `cargo test -p vis-cli --no-default-features`: **exit 0** (chạy tuần tự; không chạy song song với gate default vì cùng `target/debug/vis`).
- `cargo check -p vis-cli --no-default-features`: OK.
- `test/toolchain/run.sh`: 12/12 (do C).
- `host_posix_real.rs`: `/bin/echo`, `/usr/bin/grep` thật, interp + VM, 1/1.

## Còn chờ

- **Wave 2** (4 worktree `vd259-*`): semantics leftovers (or-pattern checker/break-continue/main-int), frame leftovers (HBody/lifecycle/timer/defaults), tooling/release (test-child/core-test/host), SQLite hardening (dbstat/token/hard cap).
- Sau wave 2: merge, chạy gate/core test, cập nhật QUEUE/report cuối.

## Quy tắc tích hợp đã áp dụng

- Không merge main vào worktree subagent; merge commit vào main theo thứ tự C → B → D → A (A sau vì chạm nhiều file semantics).
- Mỗi lần merge chạy `./vis-dev gate --fast`; core test chạy **tuần tự**, không song song với gate default.
- Không đổi runtime mặc định, không bật frame/budget. `std.path` Windows là feature mới duy nhất.
