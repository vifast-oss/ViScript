<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Phiếu kiểm toán độc lập — `ST-315c` (Gatekeeper · Agent 1)

> **Lát**: `ST-315c` (W3 · Sóng 4) · **Nhánh kiểm**: `review/r315c` · **Nền**: `987b98b8`
> **Phạm vi**: `fbcbc139` (`[ST-315c]`, 8 tệp / +244 −59) + `987b98b8` (docs Windows full)
> **Ngày**: 2026-11-26 · **Máy**: `ps` (Linux 7.0.0-43-generic, Landlock ABI 8) · **Không dùng `mps`**
> **Kết luận**: **KHÔNG ĐẠT (FAIL)** — cả 3 mục chức năng (1)(2)(3) đều đạt và tôi tái lập được,
> nhưng còn **1 lỗi M** trong đúng hàm vừa sửa: panic UTF-8 khi cắt `stderr` 800 byte
> (`runner.rs:412`), trái cam kết “không panic trên dữ liệu người dùng” (`wasi/mod.rs:15`).
> Vá 1 dòng + 1 ca hồi quy là đủ để duyệt lại; các mục còn lại chỉ có finding L.

> **CẬP NHẬT RE-AUDIT (2026-11-26, sau `801719e9`)**: F1–F3 đã vá và tôi kiểm lại độc lập —
> xem **§8**. Kết luận hiện hành: **PASS**; §0–§7 giữ nguyên làm bản gốc.

## 0. Bảng phán quyết

| Hạng mục | Kết quả | Bằng chứng |
|---|---|---|
| `#197` — `Truncate` cho tệp đơn `(write)` | **ĐẠT** | đỏ-trước `EACCES` (cả `O_TRUNC` lẫn `truncate(2)`), xanh-sau `19/19` |
| W3 M2 — trần 8 MiB/luồng + kill + `VS0501` | **ĐẠT** | đúng 8 MiB ⇒ `Ok`; 8 MiB+1 ⇒ `VS0501` + con chết < 3 s (đo 0,19 s cả cụm) |
| W3 M2 — `ThuMucDriver` RAII | **ĐẠT một phần** | 28 test plugin-host để lại **0** thư mục mới; spawn lỗi vẫn dọn; **rò khi `fs::write` lỗi** (F2) |
| W3 M2 — 0 `unsafe` | **ĐẠT** | `#![forbid(unsafe_code)]`; 0 khối `unsafe` |
| `bin/vis-soak` log ngoài repo (R7) | **ĐẠT** | `$HOME`/`VIS_SOAK_LOG_DIR`, `$HOME` rỗng ⇒ `/tmp/vislogs`, chống ghi đè còn nguyên |
| W3 H3 (root symlink/junction) | **ĐẠT (rà mã, còn caveat)** | `kiem_canon` so trên `q.root` đã canonicalize lúc khởi động ⇒ lệch ⇒ từ chối |
| Cổng Linux | **ĐẠT** | landlock 19/19 · plugin-host 28/28 · soak 2/2 · `gate --fast` PASS · clippy `-D warnings` PASS |
| Windows full | **ĐẠT** | 90 suite, tổng **3.747 passed / 0 failed** @ `c2b051c9` (bao gồm mã ST-315c) |
| **Chất lượng mã trong phạm vi vá** | **KHÔNG ĐẠT** | **F1** panic UTF-8 (M) |

## 1. `#197` — kiểm chứng đỏ-trước / xanh-sau (không tin số có sẵn)

Mã sau vá (`vis-host-linux/src/sandbox.rs:123`):

```rust
(true, true) => AccessFs::ReadFile | AccessFs::WriteFile | AccessFs::Truncate,
```

`ap_dung_voi_execute` xử lý `toan_bo(ABI::V3)` nên `Truncate` nằm trong `handle_access` — thiếu
quyền ở **luật** (không phải ở handled-set) đúng là nguyên nhân `EACCES`.

**Đỏ-trước (dựng cây tạm từ nền cũ, không sửa worktree review):**

```bash
git archive 50bb7885 | tar -x -C /tmp/opencode/r315c-red
git show fbcbc139:vis-host-linux/tests/landlock.rs > /tmp/opencode/r315c-red/vis-host-linux/tests/landlock.rs
cd /tmp/opencode/r315c-red && CARGO_TARGET_DIR=/tmp/opencode/target-red \
  cargo test -p vis-host-linux --test landlock tep_don_ghi_truncate_duoc_con_chi_doc_thi_khong
# FAILED — landlock.rs:151: fs::write … Os { code: 13, kind: PermissionDenied }
```

**Tách riêng `truncate(2)` (chứng minh không phải `WriteFile`):** ca tối giản mở `O_WRONLY`
(không `O_TRUNC`) rồi `set_len(0)`:

| Mã | `open(write)` | `set_len(0)` |
|---|---|---|
| `50bb7885` (cũ) | OK (`WriteFile` đã cấp) | **`EACCES`** |
| `fbcbc139` (mới) | OK | **OK** |

⇒ Thiếu đúng `AccessFs::Truncate`; vá là đúng và đủ. Ca #19 trong cây thật:
`cargo test -p vis-host-linux --test landlock` → **19/19 passed** (1,70 s), gồm
`tep_don_ghi_truncate_duoc_con_chi_doc_thi_khong` (phủ `fs::write`/`O_TRUNC`, `set_len(0)`,
tệp chỉ-đọc `O_TRUNC` ⇒ `PermissionDenied`, nội dung giữ nguyên).

## 2. Runner — trần 8 MiB, kill, deadlock, RAII

### 2.1 Trần và kill (kiểm bằng harness ngoài repo)

Bản sao `987b98b8` tại `/tmp/opencode/r315c-cap`, thêm module test **ngoài repo** gọi thẳng
`chay_mot(&Runtime::Node(<fake>), …)`; `<fake>` là binary ghi N byte ra stdout rồi (nếu N > trần)
ngủ 10 s. Kết quả:

| Ca | Kỳ vọng | Thực đo |
|---|---|---|
| N = `MAX_PLUGIN_OUT` = 8.388.608 | `Ok`, `len == 8 MiB` | **OK** |
| N = 8.388.609 | `VS0501` “vượt trần”, con bị kill | **OK**, cụm 3 test 0,19 s; `/proc/<pid>` đã mất |
| N = 8.388.609, con ngủ 10 s | không chờ hết 10 s | **OK** (< 3 s) |

`doc_co_tran` dùng `Read::take(MAX+1)`, cắt còn `MAX`, `Relaxed` store/load; sau `join` vẫn kiểm
cờ lần cuối ⇒ byte thứ 8 MiB+1 luôn ra `VS0501`, kể cả khi con tự thoát. **Không deadlock** trên
nhánh thường/timeout/flag khi con không sinh tiến trình cháu — đúng như thiết kế.

### 2.2 RAII

- 28 test `cargo test -p vis-plugin-host` chạy lúc 13:03 để lại **0** thư mục
  `/tmp/vis-plugin-wasi-driver-*` mới (`find -newermt 13:00` = 0); 117 thư mục còn lại đều có
  mtime 09:54/12:12 (rác của mã **cũ**, không phải rò mới).
- Spawn lỗi (`Runtime::Node("/khong/co/runtime-nay")`): guard vẫn `Drop` ⇒ thư mục bị dọn (test
  ngoài repo: **PASS**).
- **Rò còn lại:** nếu `fs::write` driver lỗi **bên trong** `thu_muc_driver`, guard chưa được tạo
  ⇒ thư mục ở lại (repro F2, test **FAILED**).

### 2.3 `unsafe`

`crates/vis-plugin-host/src/lib.rs:9` có `#![forbid(unsafe_code)]`; grep `unsafe` trong
`src/` chỉ thấy chú thích/tài liệu. **ĐẠT 0 `unsafe`.**

## 3. `bin/vis-soak` — log ngoài repo (kiểm trực tiếp)

| Kịch bản | Kết quả |
|---|---|
| mặc định, `HOME=/home/ps` | `LOG_DIR=/home/ps/vislogs`, `LOG=…/soak-<UTC>.log` |
| `HOME=` (rỗng) | `LOG_DIR=/tmp/vislogs` (nhờ `${HOME:-/tmp}`) |
| `VIS_SOAK_LOG_DIR=/tmp/opencode/soakdir-vis` | thắng `$HOME` |
| tệp cũ 14 byte + `VIS_TASK_ID=999`, không `--append` | **exit 2**, in “TỪ CHỐI ghi đè”, tệp vẫn 14 byte |
| tệp cũ **rỗng** | cho qua (exit 0) |
| sau các lần chạy | `git status` sạch; không có `*.log` mới trong `agent/JOURNAL/` |

`cargo test -p vis-cli --test cli_it soak_script -- --test-threads=1` → **2/2 passed**.
Lưu ý: ca test hiện **không** khoá hành vi mới (`VIS_SOAK_LOG_DIR`/`$HOME`/đường dẫn ngoài repo);
tôi kiểm bằng `sh -x` + kịch bản tay ở trên.

## 4. Cổng và bằng chứng Windows

```text
cargo test -p vis-host-linux --test landlock                 19 passed / 0 failed
cargo test -p vis-plugin-host                                10 + 7 + 7 + 4 = 28 passed / 0 failed
cargo test -p vis-cli --test cli_it soak_script …            2 passed / 0 failed
./vis-dev gate --fast                                        PASS (R1..R9 OK, clippy OK, L0 OK)
cargo clippy --workspace --all-targets -- -D warnings         rc=0
```

Windows `/home/ps/vislogs/st315c-win-full-c2b051c9.log` (và `.out`):

- `head_full: c2b051c91acf409d7a9dba0c22120d8569d321e1`, `git_status` rỗng, untracked 0.
- 90 dòng `test result: ok.`; tổng `passed` = **3.747**, `failed` = **0**;
  `host_conformance_suite ... ok`; cuối log `vis-targets … vis-remote-exit=0`.
- Chuỗi lệnh là `cargo clippy … && cargo test … && host_conformance && vis-targets` ⇒ test đã
  chạy nghĩa là clippy `-D warnings` đã qua.
- `git diff fbcbc139..c2b051c9` **không chạm** `vis-host-linux/`, `vis-plugin-host/`, `bin/vis-soak`
  ⇒ bằng chứng Windows tại `c2b051c9` (con cháu của `fbcbc139`) **hợp lệ** cho lát này.
- `sha256(log_tren_may_dich)=11cbf1ac…f82124` khớp journal; **hash tệp cục bộ khác**
  (`a2c20514…` cho `.log`, `d2cec757…` cho `.out`) vì `vis-remote` nối thêm trailer sau khi băm
  — journal nên ghi rõ “sha256 tệp trên máy đích” (F6).

## 5. Findings

### F1 — **M (chặn duyệt)** · panic UTF-8 khi cắt `stderr` 800 byte

- **Vị trí**: `crates/vis-plugin-host/src/wasi/runner.rs:412` (`thong_diep.truncate(800)`).
- **Bản chất**: `stderr` là `String` do `from_utf8_lossy` (hoặc `read_to_string` trước đây);
  `String::truncate` **panic** nếu vị trí 800 rơi giữa ký tự nhiều byte.
- **Repro (e2e qua đúng `chay_mot`, không phải logic chép tay)**: runtime giả ghi
  `799 × 'a' + "ệ" (3 byte) + 50 × 'b'` ra stderr rồi `exit 1` ⇒
  `panicked at runner.rs:412: assertion failed: self.is_char_boundary(new_len)`.
  Ca ngoài repo `stderr_800_byte_cat_giua_utf8_thi_panic` tái hiện 100%.
- **Tác động**: plugin (dữ liệu không tin cậy) làm **panic tiến trình host** thay vì `VS0501`;
  `vis` không có `catch_unwind` quanh đường plugin ⇒ thoát 101 kèm backtrace. Trái cam kết
  `crates/vis-plugin-host/src/wasi/mod.rs:15` (“không panic trên dữ liệu người dùng”). Lỗi có
  **trước** lát này nhưng nằm đúng khối mã ST-315c viết lại và là điểm kiểm toán trọng tâm của lát.
- **Vá đề xuất**: cắt theo biên ký tự (lùi `n` tới `is_char_boundary`) + ca hồi quy stderr
  nhiều byte > 800; rẻ hơn là hạ trần theo `char_indices`.

### F2 — **L** · RAII không phủ lỗi bên trong `thu_muc_driver`

- **Vị trí**: `crates/vis-plugin-host/src/wasi/runner.rs:248-258`.
- `create_dir_all` xong, `fs::write(driver.mjs/driver.py)` lỗi ⇒ `?` trả `Err` khi guard
  `ThuMucDriver` **chưa** được tạo ⇒ thư mục `vis-plugin-wasi-driver-*` ở lại.
- **Repro đã chạy**: tạo sẵn đúng tên thư mục kế tiếp với mode `0555` ⇒ `thu_muc_driver()` trả
  `Err`, thư mục vẫn tồn tại (test ngoài repo `loi_ghi_driver_ro_ri_thu_muc_tam` **FAILED**).
- **Vá**: tạo `ThuMucDriver(d)` ngay sau `create_dir_all`, ghi driver qua `guard.duong()`.

### F3 — **L (lý thuyết)** · lỗi `stdout.take()/stderr.take()` không kill con

- **Vị trí**: `runner.rs:343-350`. `Stdio::piped()` bảo đảm `Some`, nên gần như không xảy ra;
  nếu xảy ra, con đang chạy bị bỏ mặc (và thread đọc còn lại bị detach). Nên kill+wait trong
  nhánh lỗi cho khép kín.

### F4 — **L (lý thuyết)** · `kill()` chỉ giết tiến trình trực tiếp

- **Vị trí**: `runner.rs:377-397`. Nếu runtime sinh tiến trình cháu giữ ống, `join` chờ EOF có
  thể treo sau khi kill. WASI preview1 không có `proc_spawn`, các driver hiện tại không sinh con
  ⇒ chưa phải lỗi thực; ghi lại để không quên nếu đổi runtime.

### F5 — **L** · lỗ hổng test

- Không có ca **e2e** cho nhánh vượt trần/kill (`VS0501`) và không có ca đúng **biên**
  8 MiB / 8 MiB+1 (unit hiện dùng 1 KiB và +10 byte) — tôi đã kiểm ngoài repo, logic đúng,
  nhưng CI sẽ không bắt hồi quy.
- Không có ca RAII cho nhánh lỗi (F2).
- Không có ca truncate **tệp có sẵn trong `cap[Dir]` chỉ-đọc`** (W3 C1 nêu trong doc
  `sandbox.rs:11-12`); ca #19 chỉ phủ luật **tệp đơn**.

### F6 — **L** · câu chữ bằng chứng

- `agent/JOURNAL/st315c-w3.md:51-52` ghi “sha256 toàn tệp” cho `11cbf1ac…`; đó là hash **trên
  máy đích trước khi nối trailer**. Tệp cục bộ có hash khác. Sửa thành “sha256 tệp log trên máy
  đích” để người sau không tưởng lệch bằng chứng.

### F7 — **L** · doc lệch mã

- `vis-host-linux/src/sandbox.rs:14-15` vẫn ghi tệp chỉ xin “`ReadFile` (+`WriteFile` khi có
  quyền ghi)”, thiếu `Truncate` — chính là quyền vừa vá.

## 6. Việc cần làm để duyệt lại

1. **Bắt buộc**: vá F1 theo biên ký tự + 1 ca hồi quy (stderr tiếng Việt > 800 byte, thoát ≠ 0).
2. Nên gộp luôn (rẻ, cùng tệp): F2 (tạo guard sớm), F3 (kill+wait ở nhánh `take` lỗi), F5
   (thêm ca biên `MAX`/`MAX+1`), F7 (doc).
3. Chạy lại: `vis-host-linux` 19/19, `vis-plugin-host` 28/28, `soak_script` 2/2,
   `gate --fast`, clippy. Windows không cần lặp nếu chỉ sửa nhánh format lỗi (không chạm
   `quyen`/hành vi WASI), nhưng nếu sửa `runner.rs` quá nhánh trên thì nên chạy lại full.

## 7. Ghi chú

- Không chạy `vis-diff` — lát này không chạm compiler/VM/runtime ngôn ngữ; hợp lý theo `VD-289`.
- Không sửa mã, không push/merge; mọi thí nghiệm chạy trong bản sao tạm `/tmp/opencode/r315c-*`.
- 117 thư mục `vis-plugin-wasi-driver-*` rác có sẵn trong `/tmp` (mtime 09:54/12:12) là di sản
  của mã **trước** vá; có thể dọn một lần bằng tay, không phải lỗi của lát này.

## 8. Re-audit sau vá (2026-11-26)

> **Bối cảnh**: Agent 2 vá F1–F3 tại `801719e9` trên `main`; báo cáo review cũ merge ở `86df7ea6`.
> Worktree `review/r315c` fast-forward `d2bab2fa → c9f28526` (main tip; `801719e9` là tổ tiên).
> §0–§7 giữ nguyên. **Kết luận re-audit: PASS.**

### 8.1 Đồng bộ và phạm vi vá

```text
$ git -C /home/ps/ViScript-r315c fetch /home/ps/ViScript main
$ git merge --ff-only FETCH_HEAD
Updating d2bab2fa..c9f28526   (Fast-forward, 12 tệp)
```

`801719e9` chỉ chạm `runner.rs` (+58/−15), doc `sandbox.rs`, `st315c-w3.md`, `QUEUE.md`; **không**
đổi hành vi Landlock/`#197` hay `bin/vis-soak` (kiểm lại bên dưới).

### 8.2 F1 — đúng repro cũ, qua `chay_mot`

Harness độc lập **ngoài repo**: `/tmp/opencode/r315c-reaudit` = `git archive HEAD` + module test
nối thêm; runtime giả ghi `799×'a' + "ệ" + 50×'b'` ra stderr rồi `exit 1`:

| Kiểm | Kết quả |
|---|---|
| `catch_unwind(chay_mot(…))` | **không panic** |
| mã lỗi | `VS0501` |
| thông điệp | `plugin thoát 1 …` + `…`; **không** còn đuôi `bbbb` |
| biên 800 ASCII + `ệ`×3 | `800×'a' + '…'` (803 byte), không panic |
| toàn `ệ` (900 byte) | cắt về 798 + `…` (801 byte) |

`cat_thong_diep` (`runner.rs:285-296`) lùi tới `is_char_boundary` trước khi `truncate`; đúng,
không đổi thông điệp với chuỗi ngắn hoặc đúng 800 byte.

### 8.3 F2 / F3

- **F2**: guard tạo ngay sau `create_dir_all` (`runner.rs:256`). Test độc lập: ép thư mục kế
  tiếp mode `0555` ⇒ `thu_muc_driver()` trả `Err` và **thư mục biến mất** (ca này đỏ trước vá).
- **F3**: cả hai nhánh `stdout.take()`/`stderr.take()` `None` đều `kill()+wait()`
  (`runner.rs:361-376`). Nhánh **không thể kích hoạt từ ngoài** (`Stdio::piped()` bảo đảm
  `Some`) ⇒ xác minh bằng đọc mã.

### 8.4 Biên 8 MiB (đúng +1, không chỉ +10)

| Đầu vào | Kết quả |
|---|---|
| đúng `MAX_PLUGIN_OUT` = 8.388.608 | `Ok`, `len == 8 MiB`, không bật cờ |
| 8.388.609 (= trần + 1) | `VS0501` “vượt trần”, kill < 3 s, `/proc/<pid>` mất |
| trong repo: `MAX + 10` | vẫn xanh |

### 8.5 Cổng (target dùng chung đã rebuild)

```text
cargo test -p vis-plugin-host                 lib 12 + parity 7 + vs3 7 + cli 4 = 30/30
cargo test -p vis-host-linux --test landlock  19/19
./vis-dev gate --fast                         PASS (clippy OK, R1..R9, DAG, L0), rc=0
```

Ghi chú: `cargo build` còn in `warning: unused dependency 'vis-runtime'` tại
`crates/vis-host-native/Cargo.toml:54` — có từ `aa816e46` (ST-313a), **ngoài** ST-315c, không
làm cổng đỏ.

### 8.6 Tồn đọng không chặn

- Câu chữ `801719e9`/`st315c-w3.md` nói “test đúng biên 8 MiB **và 8 MiB+1**”, nhưng test
  trong repo là đúng 8 MiB và **+10**; chưa có ca `MAX+1` chính xác. Hành vi đã được tôi kiểm
  ngoài repo; muốn khớp câu chữ thì đổi `MAX+10` thành `MAX+1` (1 dòng) — không bắt buộc.
- F4 (kill chỉ giết con trực tiếp) và F5 (chưa có e2e cap/kill trong CI; chưa có ca truncate
  tệp có sẵn trong `cap[Dir]` chỉ-đọc) vẫn như cũ, mức **L**.

> **Phán quyết cuối: PASS** — đủ điều kiện để A0 duyệt `ST-315c`; F1–F3 đã đóng, các mục L
> ghi nợ/không chặn.
