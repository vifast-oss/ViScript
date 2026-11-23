<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310a — Tách `crates/vis-core`, gỡ coupling `vis-interp`, đóng #112

**Nhánh**: `plan/st310a-core`, tạo từ **`e9c62cb`** (`main`, VD-285). **Không** merge/push/tag;
không ssh `mps`/`psvn`. Worktree riêng
`/home/ps/.local/share/opencode/worktree/cd45e5/st310a-core` (không checkout `/home/ps/ViScript`).
Môi trường: Rust **1.100.0**, `CARGO_BUILD_JOBS=3 CARGO_PROFILE_DEV_DEBUG=0 TMPDIR=/home/ps/tmp`.

## 1. Phạm vi và kết quả

| Hạng mục (`VD-285` ST-310a) | Kết quả |
|---|---|
| Tạo `crates/vis-core` | Có: `value/` (+ `Task`/`Chan`/`Pending`/`Wait`), `json/` (+ `esc`, `derive`), `host/` (traits + fs/db), `reactor/`, `http_client/` (+ cookie), `trap.rs` |
| Một bộ `json_esc` duy nhất (đóng **#112**) | `vis_core::json::esc::esc`; `vis-loader::json_esc` chỉ re-export; encoder `json_tai` dùng cùng hàm |
| 4 crate **0 dep** `vis-interp` | `vis-lsp`, `vis-loader`, `vis-host-native`, `vis-pkg` chuyển sang `vis-core` (kiểm bằng `cargo tree`/`Cargo.toml`) |
| `vis-interp/src/interp.rs` < 900 | **1.460 → 423** LOC |
| `vis-interp/src/rt.rs` < 800 | **1.338 → 152** LOC |
| Tệp lớn nhất `vis-interp` < 1.150 | **1.073** (`rt/sched.rs`); kế tiếp `frame.rs` 1.045, `builtin/mod.rs` 1.002 |
| 0 `unsafe` | `#![forbid(unsafe_code)]` ở `vis-core`; R2 xanh |
| Diff 100k = 0 | xem §3 |
| `vis-host-wasm` xanh | xem §3 |

## 2. Kiến trúc và thay đổi chính (0 đổi hành vi)

### 2.1 `crates/vis-core` — tầng đáy DAG

- `value/`: chuyển nguyên văn `Value`, `Name`, `Closure`, `VmFunc`, `Show`/`eq`/`cmp`, parse
  literal. `Task`/`Chan`/`Pending`/`Wait`/`TrangThai` (hình dạng dữ liệu từ `rt.rs`) chuyển vào
  `value/runtime.rs`; field từ `pub(crate)` đổi `pub` + `#[doc(hidden)]` (hành vi vẫn ở interp).
  `Value::truthy` chuyển từ inherent impl của interp sang `vis-core` (cùng mã).
- `json/`: decoder/encoder chuyển nguyên văn; `esc.rs` là **nguồn duy nhất** (kèm 100k fuzz cũ);
  `derive.rs` là runtime `derives Json`.
- `host/`: `Ngoai`, `FsHost`, `MucFs`, `DocQuaHandle`, `DuongDanHandle`, `chia_dong`,
  `goi_metadata`, `NetHost`, `KetQua`, `KetQuaHttp2`, `DbHost`, `DAU_QUYEN`. Trait `Host` nội bộ
  (có `Scheduler`) **giữ ở `vis-interp`** — không phải bề mặt host.
- `reactor/`: `FrameReactorBackend`, `ReactorOp/H2*/Io*/RawEvent`, `NET_READ_CAP`, `FRAME_*`,
  trạng thái `net.write` test-only.
- `http_client/`: HTTP/1.1 đồng bộ (`http_get`/`http_head`, keep-alive/redirect/chunked cap 8 MiB)
  + `CookieJar`. Helper giao thức dùng chung cho đường async đổi `pub` + `#[doc(hidden)]`.
- `trap.rs`: `Trap`/`Abort`/`R`/`pos` (chuyển từ `frame/core.rs` + `interp.rs`) để `Value::Task`
  và trait host không phụ thuộc ngược.

`Cargo.toml` workspace thêm `crates/vis-core`; `vis-interp` thêm `vis-core`; 4 crate chuyển
`vis-interp` → `vis-core`.

### 2.2 Chia tệp `vis-interp` (R1 + mục tiêu < 1.150)

- `interp.rs`: tách `interp/stmt.rs` (`exec_stmt`), `interp/eval.rs` (`eval`/`eval_cond`),
  `interp/lit.rs` (literal/builtin name/`dinh_dang`); `pos` chuyển `vis-core::trap`.
- `rt.rs`: `rt/frame_test.rs` (kiểu frame + trigger huỷ), `rt/reactor_io.rs` (máy reactor/selector),
  `rt/ops.rs` (tạo kênh/await/join); `Task`/`Chan`/reactor trait chuyển `vis-core`.
- `builtin.rs` → `builtin/mod.rs` + `builtin/method.rs` (bộ phương thức lõi).
- `value/`: wrapper re-export `vis-core`, `value/iter.rs` (`iter_items` cần trap/trần vòng),
  `value/worker/{mod,proto,pool}.rs` thay `include!("rt/worker.rs")` (cùng đường dẫn
  `vis_interp::value::worker::*`).
- `json.rs`/`cookie.rs`/`net/mod.rs`: wrapper re-export `vis-core`; `net/async_http.rs` giữ ở
  interp (cần `Host`/scheduler frame) và dùng helper công khai của `vis-core`.
- `stdlib.rs`: dispatch `path.*` → `stdlib/path_d.rs`, `fs.*`/`net.*` → `stdlib/fs_net_d.rs`;
  hằng `DAU_QUYEN`/`FRAME_*`/`NET_READ_CAP` re-export từ `vis-core`; test R1 → `stdlib/st238_tests.rs`.
- `interp/goi.rs`: type inference → `interp/infer.rs`.

### 2.3 Tương thích API

- `vis_interp::{Value, Closure, Abort, Trap, R, pos}` và các đường dẫn cũ
  `vis_interp::{value,json,net,cookie}` vẫn biên dịch nhờ re-export.
- `vis_interp::value::worker::{TaskId,SendValue,...,ProtoValue}` giữ nguyên; `Value::to_send`
  chuyển thành **trait** `ToSend` (Rust cấm inherent impl trên kiểu ngoại crate); `ToSend` được
  re-export ở `vis_interp::value::worker`, test/example cập nhật import.
- `vis-loader::json_esc::esc` giữ chữ ký; CLI `check/test/doctor/fmt/doc` không đổi.

## 3. Kiểm chứng

- `cargo fmt --all` + `cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- `./vis-dev rules`: **R1..R7 PASS**.
- `./vis-dev gate --fast` trên Linux: **PASS 3754 / 0 failed / 4 skipped** (chi tiết §4).
- Diff 100k (`bin/vis-diff`, seed 20261115): **LECH = 0** (kết quả thật ở §4).
- `vis-host-wasm`: **xanh** (kết quả lệnh ở §4).
- Đếm LOC: `interp.rs` 423; `rt.rs` 152; max `vis-interp` 1.073; `vis-core` max 1.152
  (`http_client/mod.rs`).
- 4 crate không còn `vis-interp` trong `Cargo.toml`/`use` (grep 0).

## 4. Nhật ký lệnh (bằng chứng)

```text
$ cargo check --workspace --all-targets          # PASS, 0 warning
$ cargo fmt --all -- --check                     # PASS
$ cargo clippy --workspace --all-targets -- -D warnings   # PASS
$ ./vis-dev rules                                 # --- rules: OK (R1..R7)
$ ./vis-dev gate --fast
  Summary [77.493s] 3754 tests run: 3754 passed (1 slow), 4 skipped
  --- fmt: OK / clippy: OK / luật R1..R7: OK / toolchain: OK / release: OK
  --- self-test case âm: OK / doc-examples: OK / --- L0 test: OK
  vis-dev gate --fast: PASS (L1 chưa có tới khi có sim ViScript)

$ VIS_DIFF_N=100000 ./bin/vis-diff                 # seed 20261115 (script), seed ca 3523158054
  DIFF tong=2000   loi_tinh=126  chay=1874   lech=0 giay=2.6   seed=20261115
  DIFF tong=100000 loi_tinh=6232 chay=93768  lech=0 giay=84.2  seed=3523158054
  test result: ok. 7 passed; 0 failed

$ cargo test --manifest-path vis-host-wasm/Cargo.toml
  test result: ok. 17 passed; 0 failed ... (lib)
  test result: ok. 12 passed; 0 failed ... (wasi_fs)
$ cargo check --manifest-path vis-host-wasm/Cargo.toml --target wasm32-wasip1 --all-targets
  Finished `dev` profile [unoptimized] target(s) in 7.34s

$ cargo tree -p vis-lsp / vis-loader / vis-host-native / vis-pkg | grep vis-interp
  (không có) — 4 crate 0 dep `vis-interp`
```

**Sự cố bắt được trong gate (đã sửa)**: lần chạy đầu, `async_h2_cold_multiplex_20_vong` đỏ vì
khối `std.fs`/`std.net` chuyển sang `stdlib/fs_net_d.rs` còn chứa cả `http.get/head` nhưng điều
kiện dispatch chỉ mở cho tiền tố `fs.`/`net.`. Sửa: mở thêm tiền tố `http.`; test đơn và gate đầy
đủ xanh lại. Đây là lỗi **điều kiện dispatch do tách tệp**, không phải đổi ngữ nghĩa.

## 5. Quyết định / lưu ý cho Agent 0

1. **`#112` đóng** bằng một hàm `vis_core::json::esc::esc`; encoder đổi escape `U+0008`/`U+000C`
   từ `\u0008`/`\u000c` sang `\b`/`\f` (cùng giá trị JSON sau parse, hợp lệ RFC 8259). Không có
   golden nào phụ thuộc byte cũ (gate xanh).
2. `Value` mang `Rc<Task>`/`Rc<RefCell<Chan>>` nên `vis-core` phải giữ **hình dạng** `Task`/`Chan`;
   mọi hành vi lập lịch/kênh vẫn ở `vis-interp`. Không thêm dependency OS/TLS vào `vis-core`.
3. Trait `Host` nội bộ **không** chuyển sang `vis-core` vì nó gắn `Scheduler`/frame — chỉ các trait
   host thật (`Ngoai`/`FsHost`/`NetHost`/`DbHost`) chuyển.
4. `ToSend` là thay đổi API **nhỏ** (trait thay inherent method) nhưng mọi call site trong repo đã
   cập nhật; đường dẫn cũ `value::worker::ToSend` có re-export.
5. Không merge/push/tag; không đụng `mps`/`psvn`.

## 6. Phần còn lại

- Full gate/diff 100k/wasm đã chạy trong phạm vi ST-310a (xem §3/§4). Không còn phần bắt buộc.
- Các lát sau (`ST-310b..f`) **không** thuộc phạm vi nhánh này.
