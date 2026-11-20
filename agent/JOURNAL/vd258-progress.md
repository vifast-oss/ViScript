<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-258 — tiến độ tích hợp (2026-11-20, phiên A0)

## Đã merge vào `main`

| Luồng | Commit nguồn | Merge | Nội dung |
|---|---|---|---|
| C — fee/tooling | `0f733c7`, `7e131b0` | `f1348e5` | `verify_fee` chặn OOB/panic; `for` E_LOOP khớp VM; phân loại call; toolchain 1.100.0; core test skip sqlite |
| B — frame/async | `1ae5cfd` | `f9aaeee` | `Redispatch` stack; `CallMethod`/method loop; `RedispatchMethod`; deadlock `VS0505`; closure env; const env; fake-source final join; exit code |
| D — SQLite/stdlib | `80af3b5`, `1d07072`, `6919da5` | `4638d04` | cell/overhead cap; `SQLITE_LIMIT_LENGTH`; schema UTF-8; handle cap; `json.encode` float; **`std.path` Windows**; LSP header/Interrupted/percent drive |

## Bằng chứng trên main

- `./vis-dev gate --fast`: **1603 passed / 3 skipped / 0 failed** (sau D).
- `cargo test -p vis-cli --no-default-features`: **exit 0** (chạy tuần tự; không chạy song song với gate default vì cùng `target/debug/vis`).
- `cargo check -p vis-cli --no-default-features`: OK.
- `test/toolchain/run.sh`: 12/12 (do C).
- `host_posix_real.rs`: `/bin/echo`, `/usr/bin/grep` thật, interp + VM, 1/1.

## Còn chờ

- **A — checker/type soundness** (worktree `vd258-checker`): chưa commit; đang sửa `vis-types`, `lift.rs`, Option lift/record/None/`?.` + test `vd258_checker.rs`.
- Sau A: merge, chạy gate/core test, xử lý phần còn lại của semantics (or-pattern VM/checker, `()`, `break/continue`, `main -> int` nếu A0 chốt), rồi cập nhật QUEUE/report cuối.

## Quy tắc tích hợp đã áp dụng

- Không merge main vào worktree subagent; merge commit vào main theo thứ tự C → B → D → A (A sau vì chạm nhiều file semantics).
- Mỗi lần merge chạy `./vis-dev gate --fast`; core test chạy **tuần tự**, không song song với gate default.
- Không đổi runtime mặc định, không bật frame/budget. `std.path` Windows là feature mới duy nhất.
