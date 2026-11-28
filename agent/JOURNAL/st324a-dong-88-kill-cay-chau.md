# ST-324a — đóng #88: `vis test --timeout` kill cây cháu giữ đầu ghi ống

- Ngày: 2026-11-28 · Builder Agent 2 · worktree `/home/ps/viswt/st324a`, nhánh `plan/st324a`, nền `d1077075` (`[ST-00] VD-309`).
- Toolchain 1.100.0 (đường dẫn trực tiếp `/home/ps/.rustup/toolchains/1.100.0-x86_64-unknown-linux-gnu/bin`), `CARGO_BUILD_JOBS=2`, `TMPDIR=/home/ps/vislogs/tmp-st324a` (đĩa). Disk cuối 20 G trống (> 5 G).

## 1. Bệnh / red-before

`chay_co_han` chỉ `child.kill()` (SIGKILL đích danh con `vis run`). Cháu do con sinh ra (ví dụ `sh` → `sleep 30 &`) kế thừa đầu ghi stdout/stderr, nên luồng đọc nền không có EOF; bản hiện hành chỉ chống treo bằng `HAN_GOM_OUTPUT = 500 ms` (cụt output) và **rò tiến trình cháu**.

Repro trên HEAD `d1077075` (log `/home/ps/vislogs/st324a-red-before.log`, sha `b0a1b1b0…1ce`):

```
1..1
not ok 1 - treo.vish
# treo.vish: quá hạn 300 ms
grandchild_pid=230463
 230463  230462  229486 S    sleep 30
RESULT=ALIVE (#88 chua dong)   # vẫn sống sau 500 ms
```

## 2. Sửa

Lane `crates/vis-cli/src/vistest/tien_trinh.rs` (+ comment stale ở `vistest/run.rs:31`):

- **Unix**: `CommandExt::process_group(0)` (API an toàn, R2: không `unsafe`, không crate mới) ⇒ `pgid = pid` của con, cháu kế thừa nhóm. Quá hạn: `/bin/sh -c 'kill -TERM -<pgid>'` → chờ 50 ms → `kill -KILL -<pgid>` (builtin `kill` của dash/BSD **không nhận `--`**, đây là lỗi đã gặp khi thử nghiệm). `child.kill()` + `wait()` giữ nguyên hợp đồng trả mã/`quá_hạn`.
- **Windows**: `taskkill /T /F /PID <pid>` kill cả cây (con + hậu duệ) trong một lệnh.
- `HAN_GOM_OUTPUT` vẫn giữ là lớp an toàn cuối (phòng tiến trình tự tách nhóm), nhưng nay đầu ghi đóng ngay ⇒ output về đủ, không cụt.
- Sửa comment `run.rs` stale: `--timeout` áp cho **cả `test fn` lẫn fixture `.out/.err`** từ VD-270 (không phải "chỉ test fn" như ST-105 cũ).

## 3. Test

- Unit `vistest::tests::chau_giu_ong_khong_lam_treo` (`src/vistest/tests.rs`): ngoài mốc `quá hạn` < 3 s, nay kiểm `ROOT-OK` về đủ (không cụt) **và** PID cháu chết sau 500 ms.
- Tích hợp `cli_it/vd270_test_jobs.rs::timeout_giet_cay_chau_giu_ong` (aggregator có sẵn, R8 **không** thêm `.rs` cấp 1): fixture `.vish` gọi script `/bin/sh` spawn `sleep 30 &`, ghi PID cháu ra tệp; test khẳng định TAP/JSON **byte-identical**, exit 1, trả < 3 s, PID cháu chết sau 500 ms ở cả lần TAP lẫn lần JSON. Chỉ `#[cfg(unix)]`.

| Cổng | Kết quả | sha256 log |
|---|---|---|
| Chạy riêng ca tích hợp **25 lần** (không flake) | **25/25 ok** | `d66d4f5d…e4b` (`st324a-repeat20.log`) |
| `cargo test -p vis-cli --bin vis vistest::tests::chau_giu_ong_khong_lam_treo` | 1/1 xanh | `b087dc52…e6` |
| `cargo test -p vis-cli --test cli_it timeout_giet_cay_chau_giu_ong` | 1/1 xanh | `9ef78e90…cd` |
| `cargo test -p vis-cli` | **rc 0**, 0 fail (toàn suite) | `54203bae…58` |
| `./vis-dev gate --fast` | **PASS**, `vis-dev check: PASS`, L0 OK; 3954 test ok / 0 fail | `32f8d8e9…8e` |
| `./bin/vis test test/conformance` | **573/573 ok**, `1..573`, 0 `not ok` | `1fd665cd…76` |
| `cargo check -p vis-cli --target x86_64-pc-windows-msvc` | **không chạy được ở máy này**: `cc-rs: failed to find tool "lib.exe"` | `4f683858…88` |

Sha đầy đủ:

- `st324a-red-before.log` `b0a1b1b0c4becbf49ff717e2f2a7f8dbc8e1512b5190107c127fbb4d689ab1ce`
- `st324a-unit-green.log` `b087dc52ca7406552187febc7446142744619c657d8595f093b95409c9b246e6`
- `st324a-it-green.log` `9ef78e90c6a433751d61fec9933ca022260d64087a80affd2a188854ad0155cd`
- `st324a-repeat20.log` `d66d4f5df04fdb07e78c52ba6e44167173d40709a5e82eda3a623703dfd79e4b`
- `st324a-cargo-test-vis-cli.log` `54203bae946b5fe0bba7168e7c14ef5f5ce926ace6db01b32c45ee03cd48bc58`
- `st324a-gate-fast.log` `32f8d8e92c9527857a0d9ace85d99ec3fbb6fe2ba5fdfe7ba5b18d9cbe04a8e2`
- `st324a-conformance.log` `1fd665cd8a5229d3cf5e88a5b69c5b0c1a9ffc75621e872378f7821f9c75f376`
- `st324a-win-check.log` `4f683858d460d0fe432776be80c7383130b5ee4a13e061897966f44580500d88`

## 4. Giới hạn / bàn giao

- **Windows**: nhánh `taskkill /T /F /PID` chỉ review nguồn; máy Linux không có `lib.exe`/MSVC linker để cross-`cargo check`, cũng không có `taskkill` để chạy thật. Đề xuất A0/Gatekeeper chạy `psvn` (Windows smoke + gate) trước khi duyệt nhánh Windows. Không có test tích hợp Windows trong ca này (fixture dùng `/bin/sh`).
- Không chạy `vis-diff`: đây là thay đổi test-harness CLI (`vistest`), không chạm compiler/runtime/VM theo VD-289.
- Không merge/push; chờ Agent 1 nghiệm thu, A0 merge theo VD-279. Không tag phát hành.
- Đề xuất A0 chuyển `#88` `mở → đóng ở ST-324a`.
