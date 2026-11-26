<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows `--full` trên `main` `347e9cae` — hậu `ST-315d` (parity `cwd-khop`, F-R2) — **FAIL 1 test**

> **Người đo**: Gatekeeper (Agent 1, subagent phiên sạch), 2026-11-26.
> **Commit kiểm**: `347e9caee6b617b2480da0754af9db96880504ac` (`[ST-315d] fix parity: cwd-khop canonicalize ca hai ve`).
> **Máy**: `psvn` (MINGW64_NT-10.0-26300, 6 nhân, cargo/rustc 1.100.0). **Không** dùng `mps`.
> **Worktree điều khiển**: `/home/ps/ViScript-rwin2`, nhánh `review/rwin2` @ `347e9cae` (nền cùng commit).
> **Repo đo**: `/c/Users/ps/viscript-moi`, nhánh `main` @ `347e9cae`, `git status --porcelain` **rỗng**
> trước và sau khi chạy.
> **Kết quả: FAIL** — exit **101**; workspace **3777 passed / 1 failed / 4 ignored** (89 block).
> Thủ phạm **duy nhất**: `host_parity::fs_20_ham_parity_tren_tempdir` — **không phải** `cwd-khop`
> (cả hai engine đều in `cwd-khop=true`), mà là **kỳ vọng `walk-n` hard-code sai** trong harness:
> đòi `9` trên Windows nhưng thực tế (đúng) là `10`. Parity interp↔VM **giữ nguyên** (stdout hai
> engine giống nhau từng byte). Clippy PASS; `vis-targets` **không chạy** do chuỗi `&&` dừng sớm.

## 0. TL;DR

| Hạng mục | Kết quả | Ghi chú |
|---|---|---|
| Đồng bộ `psvn` | `pull --ff-only` OK (`c6724b46..347e9cae`), HEAD khớp `347e9cae…`; dirty 0 | `vis-gate-windows` tự kiểm HEAD, in `HEAD khớp 347e9caee6b6` |
| `bin/vis-gate-windows --full` | **exit 101** | wall **251 s** (`05:19:20Z` → `05:23:31Z`) |
| clippy `--workspace --all-targets -- -D warnings` | **PASS** | tiếp tục sang `cargo test`, không có `error[`/`-D warnings` fail |
| `cargo test --workspace --no-fail-fast` | **3777 passed / 1 failed / 4 ignored**, **89 block** | 1 `FAILED` duy nhất ở `lib_api_it` |
| Host conformance (trong workspace) | **1/1 block**, `host_conformance_suite ... ok` | lượt riêng `--nocapture` **không chạy** (chuỗi `&&`) |
| `bin/vis-targets` | **KHÔNG CHẠY** | chuỗi `&&` dừng sau test fail; chưa có bằng chứng targets lượt này |
| `host_parity` Windows | **11/12 ca hiện diện; 10 passed / 1 FAILED** | `fs_symlink_parity` là `#[cfg(unix)]` ⇒ **64/66 hàm** |
| `cwd-khop` (F-R2) | **`true` ở cả interp và VM** | bản vá parity canonicalize **chạy đúng trên Windows** |
| Blocker | **1 test harness** (`walk-n` 9 vs 10) — test-only, không phải mã sản phẩm | |

## 1. Lệnh và bằng chứng

```bash
# đồng bộ (từ /home/ps/ViScript-rwin2)
bin/vis-remote psvn -- git -C /c/Users/ps/viscript-moi pull --ff-only   # c6724b46..347e9cae
bin/vis-remote psvn -- git -C /c/Users/ps/viscript-moi rev-parse HEAD   # 347e9caee6b6…04ac

VIS_GATE_WINDOWS_LOG=/home/ps/vislogs/st315d-parity-win-full-347e9cae.log \
    ./bin/vis-gate-windows --full                                        # exit 101
```

- Thân lệnh do gate dựng (nguyên văn trong header log):
  `export TMPDIR=/c/Users/ps/tmp …; cd /c/Users/ps/viscript-moi && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --no-fail-fast && cargo test -p vis-cli --test host_conformance -- --nocapture && bash bin/vis-targets`
- `vis-remote` kiểm cây bẩn ST-175: `cay SACH (git status --porcelain rong) — duoc phep chay/do`.
- Mã thoát lấy từ tệp `.exit` trên máy đích: `[exit=101]` (`vis-remote-exit=101`).
- **Không retry mù**: chỉ chạy lại **một** ca đích danh trên Linux để chứng minh kỳ vọng sai (xem §5).

## 2. Log và sha256

| Bản | Đường dẫn | sha256 |
|---|---|---|
| Local (bản sao trên `ps`) | `/home/ps/vislogs/st315d-parity-win-full-347e9cae.log` (245 919 B, 4 449 dòng) | `75070642f9c7ef4d37539b9c2011bf317133f5ff59c239a8ca8253fe48e9f1eb` |
| Remote (log gốc trên `psvn`) | `/c/Users/ps/vislogs/vis-remote-20261126T051919Z-3308756.log` | `7f29e22beb21ae9aa46f367963e070ec80fbf426d486b9c16f4152698cf85e85` |

Hai sha **khác nhau có chủ đích**: bản local được `vis-remote` bọc thêm header `# lenh…`, `# HEADER VD-136`,
khối `# ---------- KET ----------` và dòng `[exit=101]`; phần thân log do `psvn` sinh trùng khớp.
Sha remote đã đọc lại độc lập bằng `sha256sum` trên `psvn` — khớp footer `# sha256(log_tren_may_dich)`.

Tệp test (giống nhau local/remote): `crates/vis-conformance/tests/misc_api_it/host_parity.rs`
sha256 `817604219375e2fb573f23d56d5cbf29d4dd610fc595b853e8ba1b358dcc9f32`.

## 3. Phân bố test (trích từ log)

| Nhóm | Block | Passed | Failed | Ignored |
|---|---:|---:|---:|---:|
| `cargo test --workspace` (66 binary + 23 doc-test) | 89 | 3777 | 1 | 4 |
| `cargo test -p vis-cli --test host_conformance` (lượt riêng) | — | — | — | — |
| **Tổng thực chạy** | **89** | **3777** | **1** | **4** |

- 4 ignored: 3 trong `vis-conformance` (`lat256_budget::measure_workloads`,
  `lat257_b_vm_steps::measure_vm_instruction_counts`, `lat257_fairness::quet_q_phan_bo_khong_chon_default`)
  + 1 doc-test `vis-plugin-macros` (`src/lib.rs - (line 4) ... ignored`).
- `lib_api_it`: `test result: FAILED. 362 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 28.41s`.
- Dòng lỗi duy nhất trong toàn log: `error: 1 target failed: -p vis-conformance --test lib_api_it`.
- `vis-cli cli_it` 522 ok, `async_it` 102 ok, `bytecode_it` 103 ok, `host_conformance_suite` ok;
  không có `FAILED`/`panicked` nào khác.

## 4. `host_parity` trên Windows — chi tiết

11 ca chạy (ca thứ 12 `fs_symlink_parity` bị `#[cfg(unix)]` — không chạy trên Windows):

| Ca | Kết quả |
|---|---|
| `phu_du_66_ham_host` | ok |
| `moi_ham_host_co_loi_goi_trong_harness` | ok |
| `bo_chu_thich_vis_khong_tinh_lenh_bi_comment` | ok |
| `time_hai_ham_parity_bat_bien` | ok |
| `fs_20_ham_parity_tren_tempdir` | **FAILED** |
| `fs_loi_moi_ham_parity` | ok |
| `sqlite_dml_va_giao_dich_parity` | ok |
| `sqlite_metadata_parity` | ok |
| `net_loopback_parity` | ok |
| `net_client_toi_server_rust_parity` | ok |
| `http_bon_ham_parity` | ok |
| `fs_symlink_parity` | *(không chạy — `#[cfg(unix)]`)* |

⇒ **10/11 hiện diện = 64/66 hàm host** trên Windows (2 hàm `fs.is_symlink`/`fs.read_link` chỉ chạy
ở nhánh Unix; bảng `HOST_66` và test khoá script vẫn đủ 66).

**F-R2 `cwd-khop` xác nhận chạy đúng**: stdout cả hai engine đều có `cwd-khong-rong=true` và
`cwd-khop=true`; không có ca nào rơi vào nhánh `cwd-khop loi=…`.

### 4.1. Lỗi nguyên văn

```
thread 'host_parity::fs_20_ham_parity_tren_tempdir' panicked at
crates\vis-conformance\tests\misc_api_it\host_parity.rs:623:5:
assertion `left == right` failed: fs đường thành công: <stdout>

stderr: 
  left:  "…\nwalk-n=10\n…\n"     ← thực tế (interp == VM)
 right:  "…\nwalk-n=9\n…\n"      ← kỳ vọng hard-code
```

Chi tiết: `left`/`right` **giống nhau từng dòng** trừ đúng một dòng `walk-n` (`10` vs `9`);
`canon-a=a.txt`, `canon-dotdot=a.txt`, `cwd-khop=true`, `glob-n=4`, `glob-co-link=false` ở **cả hai**.
Vì `hai_duong_voi` đã assert `o1 == o2` (interp vs VM) **trước**, lỗi này **không** phải lệch engine.

### 4.2. Nguyên nhân gốc (đã xác minh)

- Harness đặt `let walk_n = if cfg!(unix) { 10 } else { 9 };` với chú thích
  “Không có `link.txt` trên Windows ⇒ `walk` ít hơn đúng 1 mục”.
- Nhưng `fs.walk` **bỏ hoàn toàn symlink** (doc + mã `crates/vis-host-native/src/ngoai.rs:695-712`,
  `duyet_de_quy` “không đi theo symlink”), nên trên Unix `link.txt` **không được đếm**.
- Fixture `dat_fixture` chỉ tạo symlink `link.txt` dưới `#[cfg(unix)]`; các mục còn lại (kể cả
  `w.txt`, `w.bin`, `mv.txt`, `newdir`, `deep/x` do script sinh) là **10 mục trên cả hai HĐH**.
- **Bằng chứng đối chứng**: chạy đúng một ca này trên Linux tại cùng commit
  (`cargo test -p vis-conformance --test lib_api_it host_parity::fs_20_ham_parity_tren_tempdir -- --exact`)
  ⇒ **ok** với kỳ vọng `walk_n = 10`. Vậy Windows in `10` là **đúng**; kỳ vọng `9` mới sai.
- Đây là lỗi **test-only** ở nhánh `cfg!(unix)` — không chạm mã sản phẩm, không chạm `cwd-khop`.

### 4.3. Đề xuất (cho Builder, **không** sửa trong lượt Gatekeeper này)

Đổi `let walk_n = if cfg!(unix) { 10 } else { 9 };` → hằng `10` cho cả hai nền tảng (hoặc tính
`walk_n` từ chính fixture) và cập nhật chú thích: `walk` bỏ symlink nên `link.txt` không bao giờ
làm tăng số mục. Sau đó chạy lại `vis-gate-windows --full` — dự kiến 3777/0/4 + targets.

## 5. Clippy / host conformance / targets

```
cargo clippy --workspace --all-targets -- -D warnings     → PASS (đi tiếp sang cargo test)
test host_conformance_suite ... ok                        → PASS (1/1 block trong workspace)
bin/vis-targets                                           → KHÔNG CHẠY (chuỗi && dừng ở exit 101)
```

- Cảnh báo duy nhất đáng ghi: `unused dependency vis-runtime` ở `crates/vis-host-native/Cargo.toml:54`
  (lint `cargo::unused_dependencies`, mức `warn` của cargo — **không** làm clippy `-D warnings` đỏ,
  và **đã có từ trước**, không phải hồi quy của ST-315d).
- Vì `vis-targets` chưa chạy, lượt này **không** có bằng chứng ma trận targets; cần chạy lại sau khi
  vá harness.

## 6. Caveat

1. Wall **251 s** ngắn hơn khung 10–20 phút vì `target/` trên `psvn` đã có artifact (chỉ recompile
   phần đổi); vẫn là `--full` đầy đủ (clippy + toàn workspace + host).
2. Không chạy `--wasm`, soak, fuzz dài — ngoài phạm vi `--full`; không dùng `mps`.
3. Không sửa mã nguồn/test, không tag/push/merge; chỉ ghi journal này trên nhánh `review/rwin2`.
4. Bản local của log nằm ngoài repo (`/home/ps/vislogs/`) theo luật R7; sha local khác remote như §2.
5. Đã kiểm repo `psvn` sạch và HEAD không đổi sau lượt đo (`347e9caee`).

## 7. Kết luận

**FAIL (1/3778 test)** — `main` @ `347e9cae` **không** xanh trên Windows full. Không phải lỗi sản phẩm
và **không phải** `cwd-khop`: bản vá F-R2 cho `cwd-khop` chạy đúng trên Windows (`true` ở cả hai
engine), 10/11 ca `host_parity` xanh, parity interp↔VM giữ nguyên. Thủ phạm là **kỳ vọng hard-code
`walk-n=9`** trong `host_parity::fs_20_ham_parity_tren_tempdir` (đúng phải là `10` vì `walk` bỏ
symlink trên mọi nền tảng). Đề nghị Agent 0: chuyển Builder vá 1 dòng test-only, rồi Gatekeeper chạy
lại `--full` để lấy mốc xanh đầy đủ (kèm `vis-targets`).

Blocker: **1 test harness** — test-only, không chặn kiến trúc; **chưa** đủ điều kiện nghiệm thu Windows.
