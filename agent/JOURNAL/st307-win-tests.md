<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-307 W4b — Windows quick: sửa 2 phát hiện test-only (`vs3_plugin`)

> **Người làm**: subagent W4b, 2026-11-23. **Nhánh** `plan/w4b-st307-win-tests` từ **`3d247c4`**
> (ST-307). Chỉ sửa **test**; không đổi `src`, `SPEC`, API, golden hay ngưỡng cổng.
> `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`; **không ssh `mps`**; không merge/push/tag.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| `cargo test -p vis-plugin-abi --test vs3_plugin` trên **psvn**, **không** đặt `VIS_RUSTC` | **6/6 chạy thật**, 0 `BLOCKED-ENV`, 1,57 s |
| Ép dựng lại wasm (xoá `target/plugin-wasi/mau.wasm`) rồi chạy lại | **6/6**, wasm dựng lại bằng `rustc.exe` lúc 14:48 (chứng minh finder mới) |
| `cargo test -p vis-cli --test vs3_plugin` trên psvn | **4/4** |
| `vis plugin check` mẫu | `plugin: mau`, `abi: 1`, **`runtime: node`**, 3 hàm, cap `Dir` |
| `vis plugin run … cong int:2 int:3` | **`5`**, exit 0 |
| Cap demo trên psvn | đọc `/cap0/in.txt` = `xin chao wasi`; tệp ngoài cap ⇒ `VS0604`; `..` ⇒ `VS0604` + `os error 76` (`Capabilities insufficient`) |
| `bin/vis-gate-windows --quick` (psvn, commit cuối) | **exit 0** — pkg **25**, h2 **33**, cancel **8**, select **25**; 0 failed |
| Gate Linux `./vis-dev gate --fast` (commit cuối) | **PASS — 3690 passed / 0 failed / 4 skipped** (68,3 s) |
| fmt / clippy / R1..R7 | **OK** (cùng log gate cuối, dòng 3/7/8) |
| Blocker | **không** |

## 1. Phát hiện 1 — `tim_rustc()` không thấy `rustc.exe`

**Hiện tượng (Windows quick ST-307)**: `VIS_RUSTC` không đặt; finder chỉ thử `rustc` (không đuôi)
trong `$HOME/.cargo/bin` và `PATH`; Windows chỉ có `rustc.exe` ⇒ `None`. Bốn ca phụ thuộc runtime
in `BLOCKED-ENV` rồi `return` **âm thầm** ⇒ suite vẫn báo xanh dù **không chạy WASI**.

**Sửa** (`crates/vis-plugin-abi/tests/vs3_plugin.rs`):

- Thứ tự dò: **`VIS_RUSTC`** → `CARGO_HOME/bin` → `$HOME/.cargo/bin` →
  `%USERPROFILE%\.cargo\bin` → `PATH`; mỗi nơi thử **cả `rustc` lẫn `rustc.exe`**.
  `VIS_RUSTC` sai vẫn in `BLOCKED-ENV` rồi dò tiếp (không im lặng).
- `goi_duoc_rustc()` là nguồn thứ hai: gọi `rustc --version` qua `PATH`.
- `thieu_moi_truong(ly_do)`: in `BLOCKED-ENV`; nếu **Windows/macOS có `rustc`** thì **panic** —
  máy đủ toolchain không được “xanh giả” bằng cách bỏ qua. Linux vẫn cho phép bỏ qua khi
  container thiếu runtime/target (giữ nguyên hành vi CI tối giản).
- `plugin_mau()` dùng `thieu_moi_truong` cho cả ba nhánh: không thấy rustc, không chạy được
  rustc, `rustc --target wasm32-wasip1` thất bại.

## 2. Phát hiện 2 — fixture “ngoài preopen” dùng `/etc/passwd`

**Hiện tượng**: trên Windows không có `/etc/passwd` ⇒ lỗi là **ENOENT**, không chứng minh được
sandbox (tệp không tồn tại thì runtime nào cũng từ chối).

**Sửa** (abi + CLI test):

- Fixture chính là **tệp thật** trong `temp_dir()` nhưng **ngoài** thư mục cap; host đọc lại được
  sau khi plugin bị từ chối ⇒ chứng minh tệp tồn tại.
- Thêm đường **`/cap0/../<tên tệp>`** trỏ đúng tệp thật đó ⇒ trên Windows cho
  `Capabilities insufficient (os error 76)`, tức **ENOTCAPABLE** — đây mới là bằng chứng sandbox
  thật (đường tuyệt đối ngoài preopen trên Node WASI vẫn trả ENOENT, xem §5).
- Giữ ca **Linux/macOS** với `/etc/passwd` trong `#[cfg(unix)]`.
- `crates/vis-cli/tests/vs3_plugin.rs` thay `/etc/passwd` bằng tệp thật + giữ ca Unix.

## 3. Bằng chứng Windows (psvn)

Worktree riêng `/c/Users/ps/wq-w4b-st307` (đã có sẵn từ ST-307), checkout
`plan/w4b-st307-win-tests` @ **`f2911528`**. `VIS_RUSTC` **unset** trong mọi lệnh dưới đây;
`TMPDIR=TMP=TEMP=/c/Users/ps/tmp`.

```text
cargo test -p vis-plugin-abi --test vs3_plugin -- --nocapture
running 6 tests
test tat_cung_khi_strict_caps_hoac_sandbox ... ok
test codec_gia_tri_tron_vong_va_sai_thi_loi ... ok
test tu_choi_abi_lech_va_plugin_la ... ok
test nap_plugin_va_goi_cong ... ok
test parity_interp_vm_qua_wasi ... ok
test cap_dir_la_sandbox_tu_nhien ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
```

- Ép dựng lại wasm: xoá `target/plugin-wasi/mau.wasm`, chạy lại ⇒ **6/6**, tệp wasm mới
  `2224456 B` lúc **14:48** — `tim_rustc()` đã tìm ra `rustc.exe` và dựng thành công.
- `cargo test -p vis-cli --test vs3_plugin` ⇒ **4/4** (2,07 s).
- `vis plugin check target/plugin-wasi/mau.wasm`:

```text
plugin: mau
abi: 1
runtime: node
hàm: cong(int, int) -> int
hàm: doc(str) -> str
hàm: ghi(str, str) -> unit
cap yêu cầu: Dir (đọc)
```

- `vis plugin run … cong int:2 int:3` ⇒ **`5`**, exit 0.
- Cap demo: `doc str:/cap0/in.txt --cap-dir <cap>` ⇒ `xin chao wasi`; tệp thật ngoài cap ⇒
  `vis plugin: VS0604: không đọc được … No such file or directory (os error 44)`;
  `/cap0/../ngoai.txt` ⇒ `VS0604: … Capabilities insufficient (os error 76)` — cả hai exit 2.

## 4. Cổng

| Cổng | Kết quả | Log / sha256 |
|---|---|---|
| `bin/vis-gate-windows --quick` @ `f2911528` | **exit 0**; pkg 25/25, h2 33/33, cancel 8/8, select 25/25 | `/home/ps/vislogs/st307-win-tests-gate-windows-quick-final.log` — `07bb053b…f3d4da6` |
| `./vis-dev gate --fast` @ `f2911528` | **PASS 3690/0/4**; fmt/clippy/R1..R7 OK | `/home/ps/vislogs/st307-win-tests-gate-final.log` — `21068e39…b11b3c1` |
| `./vis-dev check` (trước khi thêm 3 assert `os error`) | **PASS** | `/home/ps/vislogs/st307-win-tests-check.log` — `4508ee8f…96db28e` |
| Test plugin psvn (commit cuối) | 6/6 + 4/4 | `/home/ps/vislogs/st307-win-tests-psvn-final.log` — `93ab3b25…9dc76d2` |
| Ép dựng lại wasm | 6/6 | `/home/ps/vislogs/st307-win-tests-psvn-rebuild.log` — `68beb9d6…491260` |
| `vis plugin check/run` | runtime node, `5` | `/home/ps/vislogs/st307-win-tests-psvn-cli.log` — `3a2d9953…8dd05e` |
| CLI test + cap demo | 4/4; `xin chao wasi` | `/home/ps/vislogs/st307-win-tests-psvn-cli-test.log` — `cbe67324…ec0f2b` |
| Ngoài cap (native + `..`) | `VS0604` os error 44 / 76 | `/home/ps/vislogs/st307-win-tests-psvn-demo-ngoai.log` — `b21134fa…6fd8f46c` |

Ghi chú: `vis-dev check` chạy trên `b6dec60` (bản trước khi thêm ba assert `os error`); commit cuối
`f2911528` chỉ khác ba dòng assert và đã chạy lại **6/6 + 4/4** + **gate đầy đủ** (gate có fmt,
clippy, R1..R7).

## 5. Ghi chú kỹ thuật — vì sao vẫn cần nhánh `..`

Trên Node WASI (Windows), đường **tuyệt đối** ngoài preopen (`C:/…`) vẫn trả **ENOENT** dù tệp có
thật; ENOENT ở đây là cách runtime báo “không có preopen nào chứa đường này”, không phân biệt được
với tệp thiếu. Vì vậy test giữ **hai lớp**:

1. tệp thật ngoài cap + host đọc lại được (chứng minh fixture tồn tại), và
2. `..` từ `/cap0` tới đúng tệp đó ⇒ **ENOTCAPABLE** (`os error 76`).

Lớp 2 là bằng chứng sandbox thật trên Windows; lớp 1 giữ đúng tinh thần ca cũ và chạy được cả khi
runtime báo ENOENT. Trên Unix, `/etc/passwd` vẫn là ca bổ sung (tệp hệ thống có thật, ngoài cap).

## 6. Tệp chạm

- `crates/vis-plugin-abi/tests/vs3_plugin.rs`: finder `rustc.exe` + chính sách `BLOCKED-ENV`,
  fixture ngoài cap thật, `..`, `#[cfg(unix)] /etc/passwd`.
- `crates/vis-cli/tests/vs3_plugin.rs`: fixture ngoài cap thật, `#[cfg(unix)] /etc/passwd`.
- Không đổi mã nguồn, `SPEC`, `API-CODES.txt`, golden, ngưỡng cổng.

## 7. Việc còn lại

- A0 merge `ST-306` rồi `ST-307` + commit W4b này; **không** merge/push/tag ở nhánh này.
- Không còn blocker Windows cho `vs3_plugin`: 6/6 WASI chạy thật, `vis plugin check/run` xanh.
