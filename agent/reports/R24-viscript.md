# R24 — ViScript, vòng 24 (chốt VS2 theo VD-131, 2026-11-16)

Báo cáo chốt **sau đợt 23f**. Nguồn quyết định: `KE-HOACH-CHOT-23f.md`, `viscript/agent/DECISIONS.md`
**VD-131** (định nghĩa VS2 đóng), **VD-141** (TOCTOU Linux là lỗi chặn), **VD-142** (báo cáo đã commit
không sửa — đính chính ở đây), **VD-143** (task chốt không lật DONE sớm).

## 0. Sáu hàng phụ thuộc — trạng thái tại thời điểm chốt

| Hàng | Nội dung | Trạng thái | Bằng chứng chính |
|---|---|---|---|
| **ST-130** | ca thứ 8: `use` module nội bộ + `std.fs`, **không** `pkg.*`, **không** `cap` ⇒ chạy được, không `VS0604` | **DONE** `eab366d7` | `siet_caps_pkg` **8/8** `#[test]`; đối chứng `--strict-caps` ⇒ `VS0604` exit 2; sem **447/22** không đổi |
| **ST-132b** | ca âm Landlock thứ 3: con `read_dir` ngoài phạm vi ⇒ `EACCES` | **DONE** `a47a1ffe` | **13/13** ca trên `ps`; 3 ca âm tiến trình con (đúng VD-138); đỏ-trước `exit=0 stdout="bi-mat.txt"` |
| **ST-134** | vá runner (2 target vào script) + 2 ca theo VD-132 + điều tra ca (iv) | **DONE** `6e6673bd` | `vis-targets` **18/18** trên `ps` **và** macOS; 2 ca xanh hai nền; ca (iv) **lỗi sản phẩm**, sửa **68 dòng** |
| **ST-135** | chạy lại đầy đủ trên Linux (chỉ đo) | **DONE** (số ở §2) | `cargo test` **1161/1**, host suite **54/54**, gate đỏ **chỉ** ở 1 ca môi trường |
| **ST-136** | đo trên Windows (`psvn`), chỉ đo | ****DONE** `042cf378`** | Windows **1051/102** (129 suite), host **39/54**, targets **8/18**, gate FAIL ở clippy (hệ quả ST-134, chỉ Windows) |
| **ST-137** | sửa rò rỉ TOCTOU đường đọc trên Linux | **DONE** `e47cfb2f` | đỏ-trước **92/50 000** → **0/50 000** trên **`ps` và macOS**; host **54/54** hai nền; gate PASS; không `unsafe` |

## 1. Đính chính R23 (VD-142: không sửa R23, chỉ đính chính ở đây)

| # | R23 nói | Thực tế (R24) | Căn cứ |
|---|---|---|---|
| 1 | "hạng mục **Quyền** chưa đóng trên Linux vì rò rỉ TOCTOU 798/50 000" | **Đã sửa.** Cửa sổ thật **không** ở "kiểm-rồi-mở" (ST-124 đã vá) mà ở bước xác nhận sau khi mở: `canonicalize(p)` trả **chuỗi**, rồi `metadata(chuỗi)` và quyết định quyền trên **chuỗi cũ** ⇒ kẻ tấn công chen giữa hai syscall. Sau sửa: quyết định trên **đường dẫn thật của chính handle** (`/proc/self/fd/<fd>`) và **đọc từ handle** | `JOURNAL/st137-phantich.md`, `st137-sua.md`, log `st137-*` |
| 2 | "798/50 000" như một hằng số | **Không phải hằng số**: đo lại đỏ-trước cùng commit được **92/50 000**; tỉ lệ phụ thuộc lịch trình. Cơ chế mới chứng minh lọt **tất định** bằng một ca (trật tự A bị chặn, B lọt) | `st137-red-ps.log`, `st137-cuaso-that.log` |
| 3 | "ca (iv) `.vish` + `/tmp`: chưa đủ bằng chứng kết luận" | **Là lỗi sản phẩm**, không phải lỗi ca kiểm: thân `cmd` **một dòng** bắt đầu bằng đường dẫn tuyệt đối bị phân loại **script** ⇒ `/tmp` thành phép chia ⇒ `Var("tmp")` ⇒ `VS0501`. macOS "xanh may mắn" vì `/var` là từ khoá. Đã sửa **68 dòng** (phân loại theo tên lệnh đã biết) | `st134-2c-do-truoc-ps.log`, `st134-runner-ca-nen.md` |
| 4 | "hạng mục Quyền cần **3** ca âm tiến trình con (mới có 2)" | **Đã đủ 3** (thêm ca `read_dir` ở ST-132b), đúng VD-138 | `st132b-read-dir.md` |
| 5 | "`ST-132b` do Agent 2 tự thêm, chưa duyệt (nợ #131)" | **Sai**: hàng do **Agent 0 mở** trong quyết định 23d; nợ #131 **đóng** | `QUEUE.md` đính chính dưới ST-133 |
| 6 | "nợ #126: giấy phép `unicode-ident` cần Agent 0 quyết" | **Đã đóng** bằng **VD-135** (hợp lệ theo QD §9.3 + VD-33) | `DECISIONS.md` VD-135 |
| 7 | "Linux **1147/6**" | Sau ST-134 + ST-137: **1161/1** — 5 ca đỏ đã hết; còn **1** ca do **môi trường** | §2.2 |
| 8 | "nợ #121/#122 là lỗi ca kiểm" | Đúng, và **VD-132** đã cho phép sửa; ST-134 đã sửa xong, diff dán trong journal | `st134-runner-ca-nen.md` |

## 2. Số đo theo từng nền

### 2.1 macOS (máy dev, arm64)

| Phép đo | Kết quả |
|---|---|
| `gate --fast` | **PASS** (fmt, clippy `-D warnings`, R1..R4, self-test âm, `vis explain`, report-check, doc-examples **24 khối = 18 xanh + 6 ngoại lệ**, L0) |
| `cargo test --workspace` | ****1159 passed / 0 failed**, 129 suite** |
| `vis test test/sem` / `--vm` | **447 ok** / **447 ok, 0 SKIP** |
| differential 100k seed `20261115` | **lech = 0** |
| host suite (54 ca) | **54/54** |
| `bin/vis-targets` | **18/18 `check`** |
| soak 10 vòng | ****10/10 OK = 1352 s** (lần đo hợp lệ; xem §6)** |
| R1 max | ****1498** (`crates/vis-types/src/tyck/expr.rs`)** |
| `FUNCS` / `all_codes` / `VBC1` | **52 / 33 / 53** |

### 2.2 Linux (runner `ps` — Ubuntu 26.04.1, kernel 7.0.0-43, rustc 1.99.1, Landlock ABI 8)

| Phép đo | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1161 passed / 1 failed**, 129 suite (tổng 1162; macOS 1159 ⇒ chênh 3 ca do ca gắn `cfg(target_os)`) |
| `bin/vis-host-suite` | **54/54** |
| `bin/vis-targets` | **18/18** (sau `infra/linux/setup.sh`) |
| `vis-host-linux` (Landlock) | **13/13** ca, trong đó **3** ca âm tiến trình con |
| `gate --fast` | 10/11 bước PASS; **L0 đỏ đúng 1 ca** (xem dưới) |
| ca đua TOCTOU | **0/50 000** (`ok_trong = 34142 > 0`) |

**Ca đỏ duy nhất trên Linux** — `crates/vis-cli/tests/grant_proc_canon.rs::symlink_canonicalize_hai_phia`:
trên `ps`, `/usr/bin/echo` là **symlink vào coreutils multicall** (`/usr/lib/cargo/bin/coreutils/echo`),
binary multicall **từ chối** chạy khi `argv[0]` khác tên nó. **Sản phẩm đúng** (đã canonicalize và exec
đúng đường dẫn thật); đây là **khác biệt môi trường**. Chưa sửa: VD-132 mới cho phép **2** ca, đây là ca
thứ **3** ⇒ cần Agent 0 mở một VD (hoặc cài `echo` không-multicall trên runner).

### 2.3 Windows (`psvn`, Windows 10.0.26300, x86_64-pc-windows-msvc) — **chỉ đo**

**Môi trường**: `PSVN`, Windows 11 Home `10.0.26300`, `x86_64-pc-windows-msvc`, rustc **1.99.1
`58cf8f9c3`** (khớp `ps`), `CARGO_TARGET_DIR=C:\vis-target`, repo `C:\Users\ps\vifast-windows\repo`,
`core.autocrlf=false` (`git ls-files --eol` = `i/lf w/lf`). **Hash đo: `632e1fba`**.

| Lệnh | Kết quả | So với macOS / Linux |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **1051 đạt / 102 đỏ**, 129 suite (100 suite xanh), 127 s | 1159/0 · 1161/1 |
| `bin/vis-host-suite` (Git Bash) | **39/54**, exit 1 | 54/54 · 54/54 |
| `gate --fast` (Git Bash) | **FAIL ở `clippy`**: `crates/vis-cli/tests/sandbox.rs:61` — biến không dùng khi cả hai nhánh `cfg(target_os)` bị loại trên Windows (**hệ quả ST-134**, chỉ hiện trên Windows) | PASS · PASS 10/11 |
| `bin/vis-targets` | **8/18** (toàn bộ ô `x86_64-pc-windows-msvc` xanh); 10 ô đỏ vì **thiếu std** `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu` | 18/18 · 18/18 |

**102 ca lệch — nhóm nguyên nhân (phỏng đoán, bảng đủ 102 dòng ở `JOURNAL/st136-windows.md` §4)**:
42 ca thiếu chương trình POSIX (`/bin/echo|sh|cat`, `/usr/bin/grep|printf`); 20 ca `\` đi vào literal
⇒ `VS0102`; 10 ca URI/JSON có `\`; 14 ca dạng đường dẫn JSON/TAP (` (a.vis:2:18)`); 5 ca + CRLF;
5 ca tiến trình; 4 ca mạng (`net_server`, `tcp_tu_choi`); 1 ca môi trường; 1 ca tên tệp không hợp lệ;
1 ca cần điều tra. **Không ca nào lệch logic ngôn ngữ** theo thông điệp.

**Hai điểm BLOCKED-ENV (không tự sửa)**:
1. Toolchain 1.99.1 trên `psvn` thiếu std `wasm32-unknown-unknown` + `x86_64-unknown-linux-gnu` ⇒ 10/18 ô
   `vis-targets` đỏ. Lệnh chờ duyệt: `rustup target add …` với `RUSTUP_TOOLCHAIN=1.99.1-x86_64-pc-windows-msvc`.
2. `gate --fast` **không thể PASS trên Windows** với cây hiện tại: (a) `clippy` đỏ do `sandbox.rs:61`
   (cần VD nếu thêm `cfg`/`_`), (b) `doc_examples.py` đọc ống bằng `cp1252` ⇒ `UnicodeDecodeError`
   (sửa 1 dòng `encoding="utf-8"`), (c) nhóm `/bin/*` và escape `\` thuộc **VS3**.

**Đã cập nhật trên `psvn`**: **không cài gì mới** (1.99.1 đã có sẵn từ 2026-10-31). `stable` bị rustup tự
nâng 1.96.0 → 1.100.0 khi chạy `rustup -V` ở gốc repo có `rust-toolchain.toml` — **ngoài ý muốn**, không
ảnh hưởng số đo (mọi lệnh ghim `RUSTUP_TOOLCHAIN=1.99.1`). Build Tools **không đổi** (17.14.8 / MSVC
14.44.35207 — build/link chạy ngay, không có log lỗi). **Không reboot**. Script tái lập:
`viscript/infra/windows/setup.ps1` + `setup.md` (đã chạy `-Check`: exit 1 đúng thiết kế).

Windows trong VS2 **vẫn là "chưa hỗ trợ"** (VD-121) — đây là **số đo**, không phải tuyên bố hỗ trợ.

## 3. Năm câu bắt buộc (nguyên văn, VD-131)

> **Async không song song.**
> **Kênh không rendezvous.**
> **Net không TLS.**
> **Quyền chỉ siết mặc định khi dùng gói.**
> **LSP chỉ publish tài liệu đang mở.**

## 4. Tám hạng mục VS2 — trạng thái chốt

| # | Hạng mục | Trạng thái | Căn cứ |
|---|---|---|---|
| 1 | **Async** | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | **Kênh** | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 |
| 3 | **Net/HTTP** | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback, không TLS/HTTP2/keep-alive (VD-122) |
| 4 | **Quyền** | **ĐÓNG về kỹ thuật — chờ Agent 1 dán `APPROVED` (VD-131/VD-143)** | macOS: runtime + `--strict-caps` + ST-123/124; Linux: Landlock thật **13/13** (3 ca âm tiến trình con) + **TOCTOU 0/50 000** sau ST-137. Điều kiện VD-131 "ST-137 APPROVED": **chưa dán** tại thời điểm viết (Agent 1 đối chiếu sau) — mọi số đo đã có |
| 5 | **LSP** | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | **Gói** | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration (ST-127..129); mạng/chữ ký = VS3 |
| 7 | **Đa nền tảng** | **một phần, có số ba nền** | Linux **chạy thật** (ST-131/134/135), Windows **đã đo** (ST-136), wasm **chỉ check** (VD-121) |
| 8 | **Hợp chuẩn host** | **ĐÓNG** | 54/54 ca trên **macOS và Linux**, 0 lệch; `vis-host-linux` 13/13 |

## 5. Nợ còn lại và việc đợt 25

Nợ đánh số tiếp **sau 131** (ST-132b/134/137 đã dùng 132..136; ST-135/136/138 dùng tiếp):

| # | Nguồn | Nội dung |
|---|---|---|
| 132 | ST-132b | 3 ca âm dựng helper bằng `rustc`; thiếu `crt-static` ⇒ đỏ vì môi trường |
| 133 | ST-132b | ca `read_dir` mới kiểm luật chỉ-đọc, chưa kiểm luật ghi |
| 134 | ST-134 | `setup.sh` cài thêm `x86_64-pc-windows-msvc` (ngoài `wasm32`) để đạt 18/18 — cần Agent 0 chuẩn thuận |
| 135 | ST-134 | `vis-linux` phải sync cả `vis-host-wasm/`; chưa có kiểm tự động danh sách sync |
| 136 | ST-134 | phân loại thân `cmd` theo tên lệnh: biến **trùng tên lệnh** muốn làm biểu thức bị hiểu là lệnh (đường thoát `( … )`), chưa có ca khoá |
| 137 | ST-137 | **macOS** vẫn dùng bước xác nhận hai-syscall của ST-124 (không có `/proc`, std an toàn không có fd→path) — đo 0/50 000 nhưng không bảo đảm theo cơ chế |
| 138 | ST-137 | `/proc` phải đọc được trên Linux, nếu không ⇒ `VS0604` (hành vi đúng, nhưng người vận hành phải biết) |
| 139 | ST-135 | ca `symlink_canonicalize_hai_phia` đỏ trên Linux vì coreutils multicall — cần VD thứ 3 theo VD-132 hoặc cài `echo` thật trên runner |
| 140 | R24 | nợ cũ **#109** (đường **ghi** vẫn kiểm-rồi-mở) vẫn mở; cần một dòng giới hạn trong `HOST-BAO-DAM.md` (Agent 0) |
| 141 | R24 | nợ cũ **#121..#124**, **#128**, **#129**, **#130** đã xử lý hoặc chuyển VS3 — xem R23 §6 để đối chiếu |

### Việc đợt 25 (đề xuất)

1. **Duyệt** ST-137 (và ST-130/132b/134/136 nếu Agent 1 chưa dán) để hạng mục 4 chính thức đóng.
2. Quyết **giới hạn đường ghi** (#140) và **macOS fd→path** (#137).
3. Mở **VD thứ 3** cho ca coreutils (#139) hoặc cài `echo` không-multicall trên runner.
4. Chạy `vis test`/differential/cache **trên Linux** (R23 nợ #130) để nền Linux có đủ bộ số.
5. Windows: quyết định có mở **VS3** cho Windows hay giữ "chỉ đo" (VD-121).

## 6. Soak: hai lần dừng và một lần 10/10 — ghi đủ, không giấu

| Lần | Kết quả | Phân loại | Xử lý |
|---|---|---|---|
| 1 | DỪNG ở `gate-3` — `-p vis-cli --test net`, ca `tcp_explain_tong` | **T-INFRA** (so byte interp/VM qua server loopback; chạy lại **đúng một lần** ca đó ⇒ `1 passed` 0,59 s) | Lưu log `st138-soak-attempt1-infra.log`; **không** nới ngưỡng, **không** sửa ca |
| 2 | DỪNG ở `gate-1` trong 47 s — `R3` báo `st138-phan-loai-tcp-explain.txt` thiếu sidecar `.license` | **Lỗi của chính tôi**: tệp bằng chứng tôi thêm vi phạm R3 | Thêm `…txt.license` (SPDX); `vis-dev rules` ⇒ OK |
| 3 | **10/10 vòng OK, 1352 s**, load 3,96 | — | Đây là số dùng trong R24 |

Ba lần chạy **không** phải "retry để lấy xanh": lần 1 là flake đã phân loại, lần 2 là vi phạm luật do
chính báo cáo viên gây ra và đã sửa bằng tệp sidecar (không đụng ngưỡng/ca kiểm).
