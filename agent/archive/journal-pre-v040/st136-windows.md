# ST-136 · Đo trên Windows (`psvn`) qua SSH — **chỉ đo** (VD-136, VD-140)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-136** · hash đang đo **`632e1fba`** (`origin/main` khi đo lần 2 —
số chính trong tệp này) · lần đo 1 ở **`e369698a`** (phụ lục §3.1) · nguồn:
[`KE-HOACH-CHOT-23f.md`](../../../KE-HOACH-CHOT-23f.md) §2 ST-136, [`DECISIONS.md`](../DECISIONS.md) **VD-136**
(máy đo qua SSH; Windows trong VS2 **chỉ đo**), **VD-140** (phạm vi cập nhật công cụ trên `psvn`), **VD-121**
(Windows vẫn "chưa hỗ trợ"), **VD-119**, **VD-143**; hàng QUEUE ST-136; cách đo theo
[`reports/R23-viscript.md`](../reports/R23-viscript.md) §3 (R23 **không** sửa — VD-142).

> **Chỉ đo.** Không sửa mã, không sửa kỳ vọng test, không thêm `cfg` cho Windows. Mọi số dưới đây đo trên máy `psvn`;
> log gốc ở §11. Chỗ nào là suy luận thì ghi rõ **"phỏng đoán"**.

> Vì sao có hai lần đo: lúc bắt đầu đo (07:53:59Z) `origin/main` = `e369698a`; trong lúc đo, **ST-134 land** (commit
> `6e6673bd` + lật QUEUE `7881eb30`; sau đó là các commit dòng OS `6d59c74a`, `76b6e22e`, `f6357d3f`, `632e1fba`,
> **không** chạm `viscript/crates`, `origin/main` → `632e1fba`). Để số liệu phản ánh **code cuối** (đúng ý 23f §2:
> "Windows đo sau cùng"), toàn bộ 4 lệnh được **đo lại** trên `632e1fba`; số của lần 1 giữ nguyên trong phụ lục.

---

## 0. Kết luận ngắn (4 phép đo, đặt cạnh macOS/Linux)

| Phép đo | Windows `psvn` @ **`632e1fba`** | macOS (tham chiếu) | Linux `ps` (tham chiếu) |
|---|---|---|---|
| `cargo test --workspace --no-fail-fast` | **1051 đạt / 102 đỏ** · 129 suite (100 suite xanh toàn bộ) | **1159 / 0** · 129 suite (ST-134 gate, cùng cây) | **1147 / 6** @ `c96506ba` (R23; ST-135 chưa chạy lại) |
| host suite `bin/vis-host-suite` | **39/54** (15 ca đỏ) | **54/54** (`st137-host-suite-macos.log`) | **54/54** (`st137-host-suite-ps.log`) |
| `gate --fast` | **FAIL ở bước 2/10 — `clippy`** (`-D warnings`): `crates\vis-cli\tests\sandbox.rs:61` để `code`/`err` không dùng trên Windows (do ST-134 tách nhánh `cfg`); `fmt` **OK** | **PASS** (L0 **1159/0**; `st134-gate-fast-macos.log`) | PASS 10/11 bước (R23; L0 đỏ 6 ca) |
| `bin/vis-targets` | **8/18 ô PASS** (toàn bộ ô `x86_64-pc-windows-msvc`); 10 ô đỏ **chỉ vì toolchain 1.99.1 thiếu std `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu`** | 18/18 (`st134-vis-targets-macos.log`) | 18/18 (`st134-vis-targets-ps.log`) |

Trạng thái task: **DONE** — đo đủ 4 lệnh. Windows vẫn là **"chưa hỗ trợ"** trong VS2 (VD-121): số đo là *hiện trạng*,
không phải tuyên bố hỗ trợ. **Không có thay đổi mã nào** (diff chỉ có `JOURNAL/`, `infra/windows/`, hàng QUEUE).

Ba điểm **BLOCKED-ENV cục bộ** (không chặn phép đo bắt buộc; chi tiết §6, §7):

1. toolchain `1.99.1` trên `psvn` **thiếu std** của `wasm32-unknown-unknown` và `x86_64-unknown-linux-gnu`
   ⇒ 10/18 ô `bin/vis-targets` + ca `targets_wasm_vis_types` đỏ. Đây là **thành phần toolchain** (rustup target):
   VD-136 cho phép cài qua script, nhưng brief ST-136 cấm "cài gì khác" ⇒ **KHÔNG cài**, chờ Agent 0 quyết.
2. `gate --fast` trên `632e1fba` dừng ở `clippy` vì `crates/vis-cli/tests/sandbox.rs:61` (thay đổi của **ST-134**) để
   `code`/`err` **không dùng** khi cả hai nhánh `cfg(target_os = "macos")`/`cfg(target_os = "linux")` bị loại trên
   Windows. **Lỗi chỉ hiện trên Windows**, macOS/Linux vẫn xanh. Theo luật "chỉ đo": **không sửa**, báo lại.
3. Ở `e369698a` (lần 1), `clippy` **OK** nhưng `gate` dừng ở "self-test case âm": `python3.12` (uv) trên Windows đọc
   ống bằng `cp1252` ⇒ `UnicodeDecodeError` ⇒ 7/12 fixture đỏ. Đây là lỗi **script kiểm**, không phải sản phẩm.

---

## 1. Môi trường đo (đủ mục VD-136)

| Mục | Giá trị | Nguồn |
|---|---|---|
| Máy | `PSVN` — Microsoft **Windows 11 Home** 64-bit, version **10.0.26300**, build **26300** (`ver` = 10.0.26300.9974) | `st136-env.log` |
| Truy cập | `ssh psvn` (User `ps`, `~/.ssh/psvn_ed25519`); shell mặc định **PowerShell 5.1.26100.9818** | |
| rustup | `rustup 1.29.2 (d8c92c676 2026-10-21)` | |
| **rustc đang đo** | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` · host `x86_64-pc-windows-msvc` · LLVM 23.1.1 — **khớp `ps` và macOS** (`58cf8f9c3`) | `st136-env.log` |
| cargo | `cargo 1.99.1 (5f94df478 2026-08-27)` | |
| Toolchain có trên máy | `stable` (**1.100.0**), `nightly`, `1.90.0`, `1.96`, **`1.99.1`** (được ghim làm active khi đo) | |
| Target của 1.99.1 | `x86_64-pc-windows-msvc` (**chỉ một**) | |
| Component 1.99.1 | cargo, clippy, rust-std, rustc, rustfmt | |
| **Hash đang đo (chính)** | **`632e1fba82ba7cd4a6675394b27594dce4bd0813`** = `origin/main` lúc 08:12Z; đo 08:12–08:14Z | `st136b-run.log` |
| Hash lần đo 1 | `e369698adf28c4f8aa3b5883fec9af5569a93920` (= `origin/main` lúc 07:53:59Z); đo 07:57–08:04Z | phụ lục §3.1 |
| Repo trên máy đo | `C:\Users\ps\vifast-windows\repo` — clone mới, `git config core.autocrlf false`, `git checkout --detach <hash>`; `git status` sạch; `git ls-files --eol` của các tệp kiểm = **`i/lf w/lf`** | §11 |
| **Target dir riêng** | `C:\vis-target` (qua `CARGO_TARGET_DIR`) | |
| Git / bash | `git 2.47.1.windows.1`; Git Bash `GNU bash 5.2.37(1)` tại `C:\Program Files\Git\bin\bash.exe` (bash mặc định của Windows là **WSL**, không dùng) | |
| Python | `python 3.13.3` (PATH); `python3` = uv `cpython-3.12.10` — đây là bản `vis-dev` chọn | |
| Build Tools | VS 2022 **Community 17.14.8**, MSVC **14.44.35207**, Windows SDK 10.0.22621.0 + 10.0.26100.0 | |
| Ổ đĩa | C: trống **104 662 540 288 B** (~97 GiB) | |

Mọi lệnh đo đều ghim toolchain: `RUSTUP_TOOLCHAIN=1.99.1-x86_64-pc-windows-msvc` + `CARGO_TARGET_DIR=C:\vis-target`,
vì `rust-toolchain.toml` ở gốc repo khai `channel = "stable"` và trên `psvn` "stable" **không** phải 1.99.1.

---

## 2. Những gì đã cài/cập nhật trên `psvn` (trước/sau)

| Thành phần | Trước | Sau | Ghi chú |
|---|---|---|---|
| `1.99.1-x86_64-pc-windows-msvc` | **đã có sẵn** (thư mục tạo 2026-10-31 02:58Z) | **không đổi** | Brief cho rằng máy thiếu 1.99.1 — **thực tế đã có** ⇒ **không cài gì**; đã kiểm `rustc 1.99.1 (58cf8f9c3)` khớp `ps`/macOS |
| `stable-x86_64-pc-windows-msvc` | **1.96.0** (`ac68faa20`, 2026-05-25) | **1.100.0** (`0ef1bc386`, 2026-11-09); thêm `rust-src`, `rust-docs`, std `x86_64-unknown-none`, `aarch64-unknown-none`, `riscv64gc-unknown-none-elf`, `wasm32-unknown-unknown` | **Ngoài ý muốn**, xảy ra 2026-11-16T07:55:32Z: lệnh `rustup -V` chạy ở **gốc repo** (có `rust-toolchain.toml` channel `stable`) ⇒ rustup tự cập nhật "stable" + cài target/component khai trong tệp. **Không ảnh hưởng số đo** (mọi lệnh đo ghim `RUSTUP_TOOLCHAIN=1.99.1`). Ghi ở đây cho đúng VD-140 "log phiên bản trước/sau" |
| Visual Studio Build Tools | 17.14.8 / MSVC 14.44.35207 | **không đổi** | Build + link chạy được ngay ⇒ **không có log lỗi** ⇒ **không** cập nhật (đúng VD-140) |
| Phần mềm khác | — | **không cài gì** | Không `sudo`, không gói OS, **không reboot** |
| Toolchain `1.90.0`, `1.96`, `nightly` | có | có, không đổi | |

> Bài học: **luôn** đặt `RUSTUP_TOOLCHAIN` trước khi chạy lệnh rustup/cargo trong cây repo, nếu không rustup sẽ tự cài
> theo `rust-toolchain.toml`. Script tái lập ở [`infra/windows/setup.ps1`](../../infra/windows/setup.ps1) làm đúng như vậy
> (đã chạy thử `-Check` trên `psvn`: exit 1 đúng thiết kế, báo thiếu target, **không** cài gì).

---

## 3. Bảng kết quả từng lệnh

### 3.0 Lần đo 2 — hash **`632e1fba`** (số chính; `st136b-run.log`)

| # | Lệnh (chạy trên `psvn`) | exit | Thời gian | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | `cargo test --workspace --no-fail-fast` | **101** | **127 s** | **1051 đạt / 102 đỏ**; 129 suite; 100 suite xanh toàn bộ | `st136-cargo-test.log` |
| 2 | `bash bin/vis-host-suite` (Git Bash) | **1** | **3 s** | **39/54** | `st136-host-suite.log` |
| 3 | `bash viscript/bin/vis-dev gate --fast` (Git Bash) | **1** | **4 s** | `fmt: OK` → **`clippy: FAIL`** (`sandbox.rs:61`, xem §6) ⇒ dừng | `st136-gate-fast.log` |
| 4 | `bash bin/vis-targets` (Git Bash) | **1** | **5 s** | **8/18 PASS**; 10 ô đỏ vì thiếu std target | `st136-vis-targets.log` |

Đối chiếu hai lần đo: **cùng 102 tên ca đỏ** (diff danh sách theo tên = rỗng), host suite cùng **39/54**,
`vis-targets` cùng **8/18**; chỉ **bước `gate` dừng ở chỗ khác** (lần 1: fixture encoding; lần 2: clippy — vì ST-134
thêm `sandbox.rs` chưa xanh trên Windows).

### 3.1 Phụ lục — lần đo 1, hash **`e369698a`** (trước khi ST-134 land)

| # | Lệnh | exit | Thời gian | Kết quả | Log |
|---|---|---|---|---|---|
| 1 | `cargo test --workspace --no-fail-fast` | 101 | 160 s | 1051 đạt / **102 đỏ**; 129 suite | `st136-lan1-cargo-test.log` |
| 2 | `bash bin/vis-host-suite` | 1 | 3 s | 39/54 | `st136-lan1-host-suite.log` |
| 3 | `bash viscript/bin/vis-dev gate --fast` | 1 | 84 s | `fmt: OK`, **`clippy: OK`**, `R1..R4: OK`, **self-test case âm: FAIL (7/12)** ⇒ dừng trước `build`/`L0`/`doc-examples` | `st136-lan1-gate-fast.log` |
| 4 | `bash bin/vis-targets` | 1 | 12 s | 8/18 | `st136-lan1-vis-targets.log` |

Mốc thời gian (UTC): lần 1 — clone/checkout ~07:53Z, env log **07:54:52Z**, cargo test **07:57:28Z → 08:00:08Z**,
host suite **08:02:04Z → 08:02:07Z**, gate **08:02:25Z → 08:03:48Z**, vis-targets **08:04:10Z → 08:04:22Z**;
lần 2 — **08:12:00Z → 08:14:19Z**.

Chi tiết `cargo test`: 119 tệp test, 129 dòng `test result:`; **1153 ca** được biên dịch/chạy trên Windows so với
**1159** trên macOS — **6 ca ít hơn** (phỏng đoán: các ca gắn `cfg(unix)`/`cfg(target_os)` không được biên dịch trên
Windows; chưa đối chiếu từng ca).

---

## 4. Mỗi ca lệch: tên, hai đầu ra, nhóm nguyên nhân phỏng đoán

Phân bố **102 ca đỏ** theo nhóm (ca "gộp" như `sem`, `host_conformance`, `diag_cmp`, `span_tep` chỉ tính **một** dòng
dù bên trong liệt kê nhiều ca con — số ca con ghi trong ngoặc):

| Nhóm phỏng đoán | Số ca | Ví dụ |
|---|---|---|
| **Đường dẫn** — thiếu chương trình POSIX (`/bin/echo`, `/bin/sh`, `/bin/cat`, `/usr/bin/grep`, `/usr/bin/printf`, `/usr/bin/false`, `/usr/bin/true`, `/bin/sleep`) | **42** (gồm `sem` 4 ca con + `host_conformance` gộp) | `extern::*` (10), `pipeline::*` (14), `cap_proc::*` (4), `cap_proc_grant::*` (6) |
| **Đường dẫn** — dấu `\` của Windows đi vào **literal** ViScript/JSON ⇒ `VS0102 escape không hợp lệ` | **20** | `quyen::*` (8), `std_fs_json::*` (7), `st83_no_nho` (2), `strict_caps` (2), `siet_caps_pkg` (1) |
| **Đường dẫn** — URI/JSON có `\` ⇒ sản phẩm trả `JSON không hợp lệ`, chẩn đoán **rỗng** | **10** | `lsp::*` (6), `lsp_da_tep::*` (4) |
| **Đường dẫn** — dạng đường dẫn trong JSON/TAP (tuyệt đối vs tương đối; hậu tố `(a.vis:2:18)`) | **14** | `json_ra::*` (6), `ngoai_vm::*` (3), `vis_test::*` (3), `span_tep` (5 ca con), `diag_cmp` (4 ca con) |
| **Đường dẫn + CRLF** — sản phẩm in `\`, `python3` trên Windows ghi stdout text-mode (`\n`→`\r\n`) | **5** | `chan_doan_da_tep::*` (5) |
| **Tiến trình** — lệnh `ls` cần chương trình POSIX / không spawn được chương trình / chạy script bash trực tiếp | **5** | `vsh1b`, `vish_cmd_body`, `ls_open`, `test_timeout`, `soak_script` |
| **Tiến trình/mạng** — con không ghi tệp địa chỉ trong 5 s; nối bị timeout thay vì bị từ chối | **4** | `net_server::*` (3), `net::tcp_tu_choi` |
| **Khác** — cần điều tra (`grant` trỏ thư mục nhưng không báo lỗi kiểu) | **1** | `cap_proc_grant::grant_proc_cho_cap_dir_la_loi_kieu` |
| **Môi trường** — toolchain 1.99.1 thiếu std `wasm32-unknown-unknown` | **1** | `targets::targets_wasm_vis_types` |
| **Khác** — tên tệp không hợp lệ trên Windows (`os error 123`) | **1** | `json_ra::check_json_ten_tep_dac_biet_parse_duoc` |
| **Tổng** | **102** | |

**Không ca nào đỏ vì sai logic ngôn ngữ.** 101/102 ca quy về bốn nguyên nhân máy: (a) Windows không có `/bin/*`;
(b) đường dẫn `C:\…` đi vào literal/URI/JSON; (c) EOL do `python3` text mode; (d) thiếu thành phần toolchain.
Riêng `grant_proc_cho_cap_dir_la_loi_kieu` cần điều tra thêm. **Đây là phỏng đoán** từ thông điệp lỗi — chưa mổ từng ca;
việc sửa cho Windows thuộc VS3 (VD-136: "Việc sửa cho Windows do Agent 0 mở task ở VS3").

Bảng đầy đủ 102 ca (thứ tự trong log lần 2; "actual" = giá trị `left`/thông điệp, "mong đợi" = `right`/kỳ vọng macOS):


| # | ca | suite | đầu ra Windows (actual) | mong đợi (macOS) | nhóm phỏng đoán |
|---|---|---|---|---|---|
| 1 | `khong_strict_khong_cap_van_chay` | cap_proc | exit 1 · interp stderr: c.vish:1:1: error VS0501: không chạy được chương trình ngoài `/bin/echo`: The sy… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 2 | `strict_cap_proc_sh_exit3` | cap_proc | exit 1 · stderr: d.vish:4:1: error VS0501: không chạy được chương trình ngoài `/bin/sh`: The system cann… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 3 | `strict_co_cap_proc_thi_chay_hai_duong` | cap_proc | exit 1 · interp stderr: main.vish:4:1: error VS0501: không chạy được chương trình ngoài `/bin/echo`: The… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 4 | `strict_cap_proc_pipeline_hai_stage` | cap_proc | exit 1 · stderr: e.vish:4:1: error VS0501: không chạy được chương trình ngoài `/bin/echo`: The system ca… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 5 | `grant_basename_echo_chay` | cap_proc_grant | exit 2 · (2, [], "vis run: `--grant p=echo`: không thấy lệnh `echo` trong `PATH` — cần đường dẫn tuyệt đ… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 6 | `grant_proc_cho_cap_dir_la_loi_kieu` | cap_proc_grant | exit 0, stdout `1\n` (không báo lỗi kiểu) | exit 2 (lỗi kiểu: `grant` trỏ thư mục) | **khác** — cần điều tra (phỏng đoán: kiểm kiểu `cap` cho thư mục khác trên Windows) |
| 7 | `grant_tuyet_doi_echo_chay` | cap_proc_grant | exit 2 · (2, [], "vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot … | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 8 | `grant_echo_chan_grep` | cap_proc_grant | vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot find the path specified.… | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 9 | `khong_grant_khong_gioi_han` | cap_proc_grant | exit 1 · (1, [], "d.vish:5:1: error VS0501: không chạy được chương trình ngoài `/usr/bin/grep`: The syst… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 10 | `hai_grant_pipeline_hai_stage` | cap_proc_grant | exit 2 · (2, [], "vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot … | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 11 | `loi_cap_tep_a` | chan_doan_da_tep | "C:\\Users\\ps\\AppData\\Local\\Temp\\vis-da-tep-a-11352-1\\a.vis\|1\|VS0412\|2\|14\|error\r\nb.vis\|0\|\|\|\r… · C:\Users\ps\AppData\Local\Temp\vis-da-tep-a-11352-1\a.v… | "a.vis\|1\|VS0412\|2\|14\|error\nb.vis\|0\|\|\|\n" | đường dẫn + CRLF |
| 12 | `loi_trong_b_dong_sau` | chan_doan_da_tep | "C:\\Users\\ps\\AppData\\Local\\Temp\\vis-da-tep-b-11352-3\\a.vis\|0\|\|\|\r\nb.vis\|1\|VS0412\|5\|5\|error\r\… · C:\Users\ps\AppData\Local\Temp\vis-da-tep-b-11352-3\a.v… | "a.vis\|0\|\|\|\nb.vis\|1\|VS0412\|5\|5\|error\n" | đường dẫn + CRLF |
| 13 | `loi_ca_hai_tep` | chan_doan_da_tep | "C:\\Users\\ps\\AppData\\Local\\Temp\\vis-da-tep-hai-11352-2\\a.vis\|1\|VS0412\|2\|14\|error\r\nb.vis\|1\|VS04… · C:\Users\ps\AppData\Local\Temp\vis-da-tep-hai-11352-2\a… | "a.vis\|1\|VS0412\|2\|14\|error\nb.vis\|1\|VS0412\|5\|5\|error\n" | đường dẫn + CRLF |
| 14 | `loi_trong_fn_a` | chan_doan_da_tep | "C:\\Users\\ps\\AppData\\Local\\Temp\\vis-da-tep-fn-11352-4\\a.vis\|1\|VS0412\|3\|18\|error\r\nb.vis\|0\|\|\|\… · C:\Users\ps\AppData\Local\Temp\vis-da-tep-fn-11352-4\a.… | "a.vis\|1\|VS0412\|3\|18\|error\nb.vis\|0\|\|\|\n" | đường dẫn + CRLF |
| 15 | `b_dai_van_dung_a` | chan_doan_da_tep | "C:\\Users\\ps\\AppData\\Local\\Temp\\vis-da-tep-dai-11352-0\\a.vis\|1\|VS0412\|2\|14\|error\r\nb.vis\|0\|\|\|… · C:\Users\ps\AppData\Local\Temp\vis-da-tep-dai-11352-0\a… | "a.vis\|1\|VS0412\|2\|14\|error\nb.vis\|0\|\|\|\n" | đường dẫn + CRLF |
| 16 | `ca_xanh_ba_duong_khop_stdout` | diag_cmp | 4 ca lệch: | (theo ca — dòng `not ok` trong log) | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 17 | `explain_perms_in_nhat_ky` | extern | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 18 | `duong_dan_hermetic_co_that` | extern | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 19 | `echo_raw_giu_newline` | extern | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 20 | `echo_ra_list_str` | extern | thiếu chương trình hermetic `/bin/echo` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 21 | `cat_doc_stdin_rong_la_rong` | extern | thiếu chương trình hermetic `/bin/cat` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 22 | `false_thoat_ma_1_la_vs0501` | extern | thiếu chương trình hermetic `/usr/bin/false` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 23 | `printf_nhieu_dong_bo_newline_cuoi` | extern | thiếu chương trình hermetic `/usr/bin/printf` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 24 | `stderr_khong_tron_stdout` | extern | thiếu chương trình hermetic `/bin/cat` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 25 | `timeout_la_vs0501` | extern | thiếu chương trình hermetic `/bin/sleep` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 26 | `true_khong_in_gi` | extern | thiếu chương trình hermetic `/usr/bin/true` — test phải đỏ, không fallback PATH | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 27 | `grant_ten_tran_phan_giai_path` | grant_proc_canon | exit 2 · (2, [], "vis run: `--grant p=echo`: không thấy lệnh `echo` trong `PATH` — cần đường dẫn tuyệt đ… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 28 | `tuyet_doi_khop_thi_chay` | grant_proc_canon | exit 2 · (2, [], "vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot … | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 29 | `khong_grant_khong_gioi_han` | grant_proc_canon | exit 1 · (1, [], "g.vish:5:1: error VS0501: không chạy được chương trình ngoài `C:\\Users\\ps\\AppData\\… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 30 | `grant_duong_dan_tuong_doi` | grant_proc_canon | exit 1 · (1, [], "e.vish:5:1: error VS0501: không chạy được chương trình ngoài `./bin/echo`: %1 is not a… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 31 | `sao_chep_trung_ten_bi_chan` | grant_proc_canon | vis run: `--grant p=/bin/echo`: không phân giải được `C:/bin/echo`: The system cannot find the path specified.… | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 32 | `host_conformance_suite` | host_conformance | 30 lỗi / 54 ca (120 lần chạy): | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX + `\` (15/54 ca con) |
| 33 | `check_json_ten_tep_dac_biet_parse_duoc` | json_ra | ghi: Os { code: 123, kind: InvalidFilename, message: "The filename, directory name, or volume label syntax is … | (theo ca — dòng `not ok` trong log) | khác — tên tệp không hợp lệ trên Windows |
| 34 | `check_json_khong_co_tap_khong_doi` | json_ra | "TAP version 13\n1..1\nnot ok 1 - C:\\Users\\ps\\AppData\\Local\\Temp\\vis-json-tap-check-336-3\\a.vis (a.vis:… · assertion `left == right` failed | "TAP version 13\n1..1\nnot ok 1 - C:\\Users\\ps\\AppData\\Local\\Temp\\vis-json-tap-check-336-3\\a.vis\n# VS04… | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 35 | `check_json_ba_tep_dung_thu_tu` | json_ra | thiếu f1 | (theo ca — dòng `not ok` trong log) | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 36 | `check_json_tep_sach` | json_ra | "{\"ok\":true,\"files\":[{\"file\":\"C:\\\\Users\\\\ps\\\\AppData\\\\Local\\\\Temp\\\\vis-json-sach-336-8\\\\a… · assertion `left == right` failed | "{\"ok\":true,\"files\":[{\"file\":\"C:\\Users\\ps\\AppData\\Local\\Temp\\vis-json-sach-336-8\\a.vis\",\"diagn… | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 37 | `check_json_loi_hai_tep` | json_ra | "2\r\nC:\\Users\\ps\\AppData\\Local\\Temp\\vis-json-hai-phu-336-1\\a.vis 1\r\nb.vis 1\r\nerror error\r\n" · assertion `left == right` failed | "2\na.vis 1\nb.vis 1\nerror error\n" | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 38 | `check_json_loi_trong_tep_phu` | json_ra | ["2", "C:\\Users\\ps\\AppData\\Local\\Temp\\vis-json-phu-336-4\\a.vis", "0", "b.vis", "VS0412", "error", "2 5"… · {"ok":false,"files":[{"file":"C:\\Users\\ps\\AppData\\L… | ["2", "a.vis", "0", "b.vis", "VS0412", "error", "2 5"] | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 39 | `check_json_severity_warning_tab` | json_ra | python3 lỗi: Traceback (most recent call last): | (theo ca — dòng `not ok` trong log) | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 40 | `khong_mock_thi_dung_fs_that` | ls_open | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-ls-real-11588\real.vish:2:1: error VS0501: `ls C:\Us… | exit 0 | tiến trình — `ls` cần chương trình POSIX |
| 41 | `lsp_ba_tep_bang_vis_check` | lsp | [] · lệch ở C:\Users\ps\AppData\Local\Temp\vis-lsp-so-11192-… | [("VS0301", 3, 13)] | đường dẫn — URI/JSON có `\` |
| 42 | `lsp_completion_thay_std_da_use` | lsp | {"jsonrpc":"2.0","id":2,"result":[]} | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 43 | `lsp_completion_thay_ham_module` | lsp | exit 0 · {"jsonrpc":"2.0","id":2,"result":[]} | exit 1 | đường dẫn — URI/JSON có `\` |
| 44 | `lsp_hover_ham_module` | lsp | {"jsonrpc":"2.0","id":2,"result":null} | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 45 | `lsp_module_std_sai_giong_vis_check` | lsp | [] · LSP khác vis check | [("VS0301", 3, 13)] | đường dẫn — URI/JSON có `\` |
| 46 | `lsp_module_thieu_giong_vis_check` | lsp | LSP thiếu VS0701: [] | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 47 | `a_sach_rong_b_co_loi` | lsp_da_tep | Content-Length: 199 | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 48 | `did_change_van_dung_a` | lsp_da_tep | Content-Length: 199 | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 49 | `mo_a_chi_chan_doan_a` | lsp_da_tep | Content-Length: 199 | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 50 | `mo_b_chan_doan_b` | lsp_da_tep | Content-Length: 199 | (theo ca — dòng `not ok` trong log) | đường dẫn — URI/JSON có `\` |
| 51 | `tcp_tu_choi` | net | stdout: "lỗi: quá thời gian chờ nối tới `127.0.0.1:54919`\n" | (theo ca — dòng `not ok` trong log) | quyền/mạng — nối bị timeout |
| 52 | `server_hai_ket_noi_lien_tiep` | net_server | quá 5 s chưa thấy địa chỉ trong C:\Users\ps\AppData\Local\Temp\vis-srv-hai-ket-noi-17180\addr-false.txt | (theo ca — dòng `not ok` trong log) | tiến trình/mạng — con không ghi tệp địa chỉ trong 5 s |
| 53 | `server_mot_lan` | net_server | quá 5 s chưa thấy địa chỉ trong C:\Users\ps\AppData\Local\Temp\vis-srv-mot-17180\addr.txt | (theo ca — dòng `not ok` trong log) | tiến trình/mạng — con không ghi tệp địa chỉ trong 5 s |
| 54 | `server_hai_tien_trinh` | net_server | quá 5 s chưa thấy địa chỉ trong C:\Users\ps\AppData\Local\Temp\vis-srv-hai-17180\addr-0.txt | (theo ca — dòng `not ok` trong log) | tiến trình/mạng — con không ghi tệp địa chỉ trong 5 s |
| 55 | `cat_pipe_vm_cung_byte` | ngoai_vm | exit 1 · [67, 58, 92, 85, 115, 101, 114, 115, 92, 112, 115, 92, 65, 112, 112, 68, 97, 116, 97, 92, 76, 1… | exit 0 | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 56 | `complete_ma_thoat_vm_cung_byte` | ngoai_vm | exit 1 · [67, 58, 92, 85, 115, 101, 114, 115, 92, 112, 115, 92, 65, 112, 112, 68, 97, 116, 97, 92, 76, 1… | exit 0 | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 57 | `sh_exit3_vm_cung_byte` | ngoai_vm | exit 1 · [67, 58, 92, 85, 115, 101, 114, 115, 92, 112, 115, 92, 65, 112, 112, 68, 97, 116, 97, 92, 76, 1… | exit 0 | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 58 | `chuong_trinh_ngoai_co_that` | pipeline | thiếu chương trình ngoài `/bin/echo` (test giả định POSIX có nó) | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 59 | `complete_grep_khong_khop_het_la_loi` | pipeline | exit 1 · complete ⇒ **không** còn VS0501: C:\Users\ps\AppData\Local\Temp\vis-pipe-cmp-16624\cmp.vish:2:1… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 60 | `complete_grep_khop_code_0` | pipeline | exit 1 · assertion `left == right` failed | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 61 | `complete_giu_stderr_va_code_khac_0` | pipeline | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-cmp3-16624\cmp3.vish:2:1: error VS0501: không c… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 62 | `echo_qua_cat_nhan_stdin` | pipeline | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-cat-16624\cat.vish:2:1: error VS0501: không chạ… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 63 | `echo_qua_grep_tim_thay` | pipeline | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-grep-16624\grep.vish:2:1: error VS0501: không c… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 64 | `fixture_vish_trong_test_cmd_pipe` | pipeline | exit 1 · fixture cat: C:\Users\ps\vifast-windows\repo\viscript\crates\vis-cli\../../test/cmd/pipe\cat.vi… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 65 | `grep_khong_khop_van_la_loi_khi_khong_complete` | pipeline | stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-grep0-16624\grep0.vish:2:1: error VS0501: không chạy được chươ… | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 66 | `list_nhieu_dong_qua_grep_loc_dung_dong` | pipeline | exit 1 · assertion `left == right` failed | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 67 | `printf_hai_dong_qua_cat_ra_hai_dong` | pipeline | exit 1 · assertion `left == right` failed | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 68 | `printf_khong_newline_qua_cat_khong_them_dong_rong` | pipeline | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-printf-16624\printf.vish:2:1: error VS0501: khô… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 69 | `raw_str_di_nguyen_van_khong_them_newline` | pipeline | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-pipe-raw-16624\raw.vish:2:1: error VS0501: không chạ… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 70 | `timeout_neu_ro_stage` | pipeline | thông điệp timeout phải nêu stage: C:\Users\ps\AppData\Local\Temp\vis-pipe-to-16624\to.vish:2:1: error VS0501:… | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX |
| 71 | `vm_co_host_chay_cung_interp` | pipeline | exit 1 · assertion `left == right` failed | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 72 | `ghi_thieu_co` | quyen | stderr: f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 73 | `doc_trong_quyen` | quyen | exit 2 · stderr: f.vis:3:23: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 74 | `ghi_co_co` | quyen | exit 2 · stderr: f.vis:3:23: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 75 | `doc_ngoai_quyen` | quyen | stderr: f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 76 | `grant_doi_duong_dan` | quyen | stderr: f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 77 | `hai_cap_dir` | quyen | stderr: f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 78 | `khong_khai_cap_thi_khong_han_che` | quyen | exit 2 · stderr: f.vis:4:19: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 79 | `net_thieu_cap` | quyen | stderr: f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 80 | `sem_323_ca_qua_binary` | sem | 4/447 ca sai: | (theo ca — dòng `not ok` trong log) | đường dẫn — thiếu chương trình POSIX (4 ca con) |
| 81 | `co_pkg_va_khai_du_cap_thi_chay` | siet_caps_pkg | exit 2 · stderr: f.vis:4:23: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 82 | `vis_soak_hop_le_va_in_du_10_vong` | soak_script | chạy --dry-run: Os { code: 193, kind: Uncategorized, message: "%1 is not a valid Win32 application." } | (theo ca — dòng `not ok` trong log) | tiến trình — chạy script bash trực tiếp |
| 83 | `json_bao_dung_tep_va_dong_cot_moi_ca` | span_tep | 5 ca sai: | (theo ca — dòng `not ok` trong log) | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 84 | `exists_ngoai_quyen_trap` | st83_no_nho | f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 85 | `exists_trong_quyen_tra_bool` | st83_no_nho | exit 2 · stderr: f.vis:3:23: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 86 | `explain_perms_in_fs_nhat_ky` | std_fs_json | exit 2 · stderr: log.vis:4:20: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 87 | `differential_fs_va_json_khop_hai_duong` | std_fs_json | exit 2 · [diff] interp exit 2: diff.vis:5:20: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 88 | `fs_duong_dan_tuyet_doi_duoc_phep` | std_fs_json | exit 2 · [abs] interp exit 2: abs.vis:4:20: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 89 | `fs_exists_va_read_thieu_la_err` | std_fs_json | exit 2 · [ex] interp exit 2: ex.vis:4:21: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 90 | `fs_ghi_vao_duong_dan_sai_la_err` | std_fs_json | exit 2 · [bad] interp exit 2: bad.vis:4:20: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 91 | `fs_list_sap_xep_khong_de_quy` | std_fs_json | exit 2 · [ls] interp exit 2: ls.vis:4:19: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 92 | `fs_write_read_round_trip` | std_fs_json | exit 2 · [rw] interp exit 2: rw.vis:4:20: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 93 | `strict_cap_dir_trong_root` | strict_caps | exit 2 · stderr: f.vis:3:23: error VS0102: escape không hợp lệ | exit 0 | đường dẫn — `\` trong literal ⇒ VS0102 |
| 94 | `strict_cap_dir_ngoai_root` | strict_caps | f.vis:3:23: error VS0102: escape không hợp lệ | (theo ca — dòng `not ok` trong log) | đường dẫn — `\` trong literal ⇒ VS0102 |
| 95 | `targets_wasm_vis_types` | targets | vis-types × wasm32 phải CHECK xanh; stderr: | (theo ca — dòng `not ok` trong log) | môi trường — thiếu std wasm32 trong toolchain 1.99.1 |
| 96 | `treo_bi_giet_trong_3s` | test_timeout | ps: Error { kind: NotFound, message: "program not found" } | (theo ca — dòng `not ok` trong log) | tiến trình — không spawn được chương trình |
| 97 | `vm_khop_interp_va_khong_con_skip` | vis_test | exit 1 · VM: exit phải 0 (ca cần host thì SKIP): | exit 0 | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 98 | `tap_hop_le_va_du_so_ca` | vis_test | exit 1 · exit phải 0 khi toàn xanh: | exit 0 | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 99 | `cac_dong_ok_dung_thu_tu_khong_lap` | vis_test | dòng thứ 423 sai thứ tự: ok 426 - vish\ngoai-thoat-ma.vish | (theo ca — dòng `not ok` trong log) | đường dẫn — dạng đường dẫn trong JSON/TAP |
| 100 | `than_lenh_tren_thu_muc_that` | vish_cmd_body | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-cmdbody-that-7344\l.vish:3:3: error VS0501: `ls C:\U… | exit 0 | tiến trình — `ls` cần chương trình POSIX |
| 101 | `main_rong_va_pipeline_hop_le` | vish_main | exit 1 · interp stderr: a.vish:4:1: error VS0501: không chạy được chương trình ngoài `/bin/echo`: The sy… | exit 0 | đường dẫn — thiếu chương trình POSIX |
| 102 | `long_tren_thu_muc_that` | vsh1b | exit 1 · stderr: C:\Users\ps\AppData\Local\Temp\vis-vsh1b-that-9548\l.vish:2:1: error VS0501: `ls C:\Use… | exit 0 | tiến trình — `ls` cần chương trình POSIX |

---

## 5. Host suite: **39/54** — 15 ca đỏ

`bash bin/vis-host-suite` (Git Bash) chạy bộ hợp chuẩn host qua harness Rust, mỗi ca **cả interp và `--vm --no-cache`**.
Trên Windows (`632e1fba`): **39/54 xanh, 15 đỏ** — **giống hệt** lần đo `e369698a` (macOS **54/54**,
Linux `ps` **54/54** — `st137-host-suite-{macos,ps}.log`).

| Ca | Windows (actual) | Mong đợi (macOS) | Nhóm phỏng đoán |
|---|---|---|---|
| `fs_duong_dan_tuyet_doi` | exit 2, stderr `c.vis:3:19: error VS0102: escape không hợp lệ` | exit 0, stdout `xin chào\n` | đường dẫn (`\` trong literal) |
| `ngoai_echo` | exit 1, `VS0501 … /bin/echo: The system cannot find the path specified. (os error 3)` | exit 0, `[xin chào]\n` | đường dẫn (thiếu `/bin/echo`) |
| `ngoai_cat` | exit 1, `VS0501 … /bin/echo` (stage đầu) | exit 0, `[a]\n` | đường dẫn |
| `ngoai_grep_match` | exit 1, `VS0501 … /bin/echo` | exit 0, `[a]\n` | đường dẫn |
| `ngoai_complete_code1` | exit 1, `VS0501 … /bin/echo` | exit 0, `stdout  []\nstderr  []\ncode    1\n` | đường dẫn |
| `ngoai_complete_code0` | exit 1, `VS0501 … /bin/echo` | exit 0, `stdout  [a]\nstderr  []\ncode    0\n` | đường dẫn |
| `ngoai_dashdash` | exit 1, `VS0501 … /bin/echo` | exit 0, `[-- -n]\n` | đường dẫn |
| `ngoai_dau_cach` | exit 1, `VS0501 … /bin/echo` | exit 0, `[a b]\n` | đường dẫn |
| `ngoai_nhieu_stage` | exit 1, `VS0501 … /bin/echo` | exit 0, `[a]\n` | đường dẫn |
| `ngoai_printf` | exit 1, `VS0501 … /usr/bin/printf` | exit 0, `[abc]\n` | đường dẫn |
| `cap_trong` | exit 2, `c.vis:2:23 VS0102` + `c.vis:3:19 VS0102` + `VS0201: @ phải theo sau là literal đường dẫn` | exit 0, `xin chào\n` | đường dẫn (`\` trong literal) |
| `cap_ngoai` | exit 2 (đúng mã) nhưng stderr là `VS0102`/`VS0201` thay vì chẩn đoán quyền | exit 2, chẩn đoán quyền | đường dẫn |
| `cap_thieu_write` | exit 2 (đúng mã) nhưng stderr `VS0102`/`VS0201` | exit 2, chẩn đoán quyền | đường dẫn |
| `cap_grant` | exit 2, `VS0102`/`VS0201` | exit 0, `B\n` | đường dẫn |
| `cap_khong_khai` | exit 2, `c.vis:3:19 VS0102` | exit 0, `ngoai\n` | đường dẫn |

Đối chứng: `fs_read_crlf`, `fs_ten_unicode`, `fs_big_64k`, `ls_mock_*`, `shell_*`, `det_*`… đều **xanh** — nghĩa là
I/O tệp và ngôn ngữ chạy được trên Windows; chỗ đỏ tập trung ở **chương trình ngoài** và **đường dẫn tuyệt đối**.

---

## 6. `gate --fast` — hai lần dừng ở hai bước khác nhau

**Lần 2, `632e1fba` (số chính) — FAIL ở `clippy` sau 4 s:**

```
== vis-dev check ==
--- fmt: OK
error: unused variable: `code`
  --> crates\vis-cli\tests\sandbox.rs:61:10
error: unused variable: `err`
  --> crates\vis-cli\tests\sandbox.rs:61:21
   = note: `-D unused-variables` implied by `-D warnings`
--- clippy: FAIL
```

Nguyên nhân: **ST-134** đổi `sandbox_check_luon_in_cuong_che` thành
`let (code, out, err) = chay(&d, &["run", "--sandbox-check", "f.vis"]);` rồi chỉ dùng `code`/`err` **bên trong**
`#[cfg(target_os = "macos")]` và `#[cfg(target_os = "linux")]`. Trên Windows **cả hai nhánh bị loại** ⇒ `code` và `err`
không dùng ⇒ `clippy -D warnings` đỏ. Trên macOS/`ps` không thấy (mỗi nền còn một nhánh) — xem
`st134-gate-fast-macos.log`: `clippy: OK`, `gate --fast: PASS`, L0 **1159/0**.

⇒ Đây là **lỗi chỉ hiện trên Windows** do một thay đổi đã xanh trên hai nền kia. **Chỉ đo — không sửa.**

**Lần 1, `e369698a` — `clippy` OK, FAIL ở "self-test case âm" sau 84 s:**

```
--- fmt: OK
--- clippy: OK
--- luật R1..R4: OK
1..12  (ok 1..5, not ok 6..12 = 7 fixture doc-examples)
# fixtures: FAIL (7)
--- self-test case âm: FAIL
```

Nguyên nhân (traceback nguyên văn trong `st136-lan1-gate-fast.log`): `vis-dev` chọn `python3.12`
(uv `cpython-3.12.10-windows-x86_64-none`); `doc_examples.py` gọi `subprocess.run(...)` không đặt `encoding` nên Python
trên Windows giải mã stdout của `vis` bằng **`cp1252`**, gặp byte `0x81` của tiếng Việt ⇒
`UnicodeDecodeError: 'charmap' codec can't decode byte 0x81 in position 875` ⇒ `proc.stdout = None` ⇒
`AttributeError: 'NoneType' object has no attribute 'splitlines'` (`doc_examples.py:180`).
⇒ **Lỗi script kiểm trên Windows**, không phải lỗi sản phẩm; theo luật "chỉ đo": **không sửa**.

Bù lại `L0`: đo riêng bằng `cargo test --workspace` (§3) — cùng bộ test mà `gate` sẽ chạy ở bước cuối.

---

## 7. `bin/vis-targets`: **8/18 ô PASS**

| crate | `x86_64-pc-windows-msvc` | `wasm32-unknown-unknown` / khác |
|---|---|---|
| vis-diag, vis-syntax, vis-types, vis-bytecode, vis-fmt, vis-interp, vis-vm, vis-cli | **PASS** (8/8) | **FAIL** — `error[E0463]: can't find crate for std/core` (8) |
| vis-host-wasm × `wasm32-unknown-unknown` | — | **FAIL** (E0463 `std`) |
| vis-host-linux × `x86_64-unknown-linux-gnu` | — | **FAIL** (E0463 `core`) |

⇒ **8/18**, toàn bộ 10 ô đỏ là **thiếu rust-std của target**, không phải lỗi biên dịch mã nguồn. Toolchain `1.99.1`
trên `psvn` chỉ cài `x86_64-pc-windows-msvc`; `ps` cùng phiên bản 1.99.1 có `wasm32-unknown-unknown` +
`x86_64-pc-windows-msvc` (+ host linux), macOS có `wasm32-unknown-unknown` (+ host).
Đây là **thành phần toolchain**; VD-136 cho phép cài qua rustup **trong script**, nhưng brief ST-136 cấm "cài gì khác"
⇒ **không cài trong ST-136**. Lệnh cần Agent 0 cho phép (chưa chạy):

```
RUSTUP_TOOLCHAIN=1.99.1-x86_64-pc-windows-msvc rustup target add wasm32-unknown-unknown x86_64-unknown-linux-gnu
```

---

## 8. So sánh ba nền

| Phép đo | macOS (cùng cây) | Linux `ps` | Windows `psvn` `632e1fba` |
|---|---|---|---|
| rustc | 1.99.1 `58cf8f9c3` | 1.99.1 `58cf8f9c3` | **1.99.1 `58cf8f9c3`** (khớp) |
| `cargo test --workspace` | **1159 / 0** (129 suite; ST-134 gate) | 1147 / 6 @ `c96506ba` (R23) | **1051 / 102** (129 suite) |
| host suite | 54/54 | 54/54 | **39/54** |
| `gate --fast` | **PASS** (fmt/clippy/rules/fixtures OK, L0 1159/0) | PASS 10/11 bước (L0 đỏ 6 ca) | **FAIL ở `clippy`** (2 biến không dùng trong `sandbox.rs:61`) |
| `bin/vis-targets` | 18/18 | 18/18 | **8/18** (thiếu 2 std target) |

Ghi chú: **ST-135 chưa chạy lại Linux** trên cây hiện tại (hàng QUEUE đang READY) ⇒ số Linux vẫn là của R23
(`c96506ba`). Số macOS lấy từ log ST-134 (cùng cây `632e1fba`) và log host suite ST-137 (`a85193bd`; host suite không
bị ảnh hưởng bởi 2 ca `cargo test` mà ST-134 đổi).

---

## 9. Kết luận và việc đề nghị (không tự làm)

1. **Windows trong VS2 vẫn "chưa hỗ trợ"** (VD-121). Con số để R24 ghi: *"Windows `632e1fba`: **cargo test 1051/102**
   (129 suite), **host suite 39/54**, `gate --fast` **chưa qua `clippy`** vì `sandbox.rs:61` (ST-134), `vis-targets`
   **8/18** vì thiếu std target"* — trong đó **91/102 lệch thuộc nhóm đường dẫn** (phỏng đoán), **không có lệch logic
   ngôn ngữ**.
2. Không có thay đổi mã/kỳ vọng/`cfg` nào cho Windows. Đề nghị (VS3, do Agent 0 mở task):
   (a) cho phép `rustup target add wasm32-unknown-unknown x86_64-unknown-linux-gnu` cho 1.99.1 trên `psvn`;
   (b) sửa `doc_examples.py` đọc ống bằng `encoding="utf-8", errors="replace"` (lỗi script kiểm, 1 dòng);
   (c) xử lý `code`/`err` không dùng trong `sandbox.rs` khi chạy trên Windows (hệ quả ST-134 — cần VD mới nếu phải
       thêm `cfg`/underscore);
   (d) nhóm ca dùng `/bin/*` chuyển sang chương trình hermetic của nền hoặc `cfg(windows)` **theo VD mới**;
   (e) nhóm ca nhúng đường dẫn tuyệt đối vào literal/URI/JSON cần escape theo nền.
3. Đã ghi script tái lập: [`infra/windows/setup.ps1`](../../infra/windows/setup.ps1) + [`setup.md`](../../infra/windows/setup.md)
   (đã chạy thử `-Check` trên `psvn`: exit **1** đúng thiết kế, in toolchain 1.99.1 + cảnh báo thiếu target, không cài).

---

## 10. Tái lập (đúng lệnh đã chạy)

```powershell
# 1) toolchain: ĐÚNG bản của ps (1.99.1) — không update stable, không đổi default
$env:RUSTUP_TOOLCHAIN = "1.99.1-x86_64-pc-windows-msvc"
$env:CARGO_TARGET_DIR = "C:\vis-target"
$env:CARGO_TERM_COLOR = "never"

# 2) nguồn đúng hash đang đo
git -c core.autocrlf=false clone --config core.autocrlf=false https://github.com/vifast-oss/ViFastOs.git C:\Users\ps\vifast-windows\repo
cd C:\Users\ps\vifast-windows\repo
git config core.autocrlf false
git fetch origin; git checkout --detach 632e1fba82ba7cd4a6675394b27594dce4bd0813

# 3) đo
cd C:\Users\ps\vifast-windows\repo\viscript
cargo test --workspace --no-fail-fast                     # 1051/102, exit 101, 127 s
& "C:\Program Files\Git\bin\bash.exe" -lc "cd /c/Users/ps/vifast-windows/repo/viscript && bash bin/vis-host-suite"     # 39/54
& "C:\Program Files\Git\bin\bash.exe" -lc "cd /c/Users/ps/vifast-windows/repo && bash viscript/bin/vis-dev gate --fast" # FAIL ở clippy
& "C:\Program Files\Git\bin\bash.exe" -lc "cd /c/Users/ps/vifast-windows/repo/viscript && bash bin/vis-targets"        # 8/18
```

---

## 11. Log (đo trên `psvn`, 2026-11-16)

**Lần đo 2 — `632e1fba` (số chính):**

| Tệp | Nội dung |
|---|---|
| [`st136-env.log`](st136-env.log) | log môi trường đủ mục VD-136 (máy, OS, rustc `-Vv`, rustup, toolchain, target, component, hash, repo) |
| [`st136-cargo-test.log`](st136-cargo-test.log) | `cargo test --workspace --no-fail-fast` — 129 dòng `test result:`, 102 ca đỏ |
| [`st136-host-suite.log`](st136-host-suite.log) | host suite qua Git Bash — 15 dòng `not ok` + kết luận HỎNG |
| [`st136-gate-fast.log`](st136-gate-fast.log) | `gate --fast` — `fmt: OK`, `clippy: FAIL` (`sandbox.rs:61`) |
| [`st136-vis-targets.log`](st136-vis-targets.log) | ma trận 18 ô — 8 PASS / 10 FAIL (E0463) |

**Lần đo 1 — `e369698a` (phụ lục):**

| Tệp | Nội dung |
|---|---|
| [`st136-lan1-cargo-test.log`](st136-lan1-cargo-test.log) | `cargo test --workspace --no-fail-fast` — cùng 102 tên ca đỏ |
| [`st136-lan1-host-suite.log`](st136-lan1-host-suite.log) | host suite — cùng 15 ca `not ok` |
| [`st136-lan1-gate-fast.log`](st136-lan1-gate-fast.log) | `gate --fast` — `clippy: OK`, 7/12 fixture đỏ (encoding `cp1252`) kèm traceback |
| [`st136-lan1-vis-targets.log`](st136-lan1-vis-targets.log) | ma trận 18 ô — 8 PASS / 10 FAIL (E0463) |

Ghi chú về log: console Windows dùng `cp1252` nên `st136-lan1-gate-fast.log` có **2 byte không hợp lệ UTF-8**, đã bị
loại khi chép sang repo (`iconv -c`); các tệp còn lại nguyên văn.
