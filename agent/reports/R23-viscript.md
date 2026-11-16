# R23 — ViScript, vòng 23 (đợt 23 theo quyết định Agent 0, 2026-11-16)

Báo cáo chốt đợt **ST-119..ST-133** (quyết định `QUYET-DINH-A0-ViScript-2026-11-16.md`, VD-119..VD-131).
Số đo dưới đây **do máy sinh**, không viết tay: mỗi con số có lệnh tái lập ở §3.

## 1. Mục tiêu và phạm vi

Đợt 23 nhận **15 hàng QUEUE** (ST-119..ST-133) với một mục tiêu duy nhất: **đóng VS2 theo định nghĩa
của VD-131** — mỗi hạng mục trong 8 hạng mục phải ở đúng một trong ba trạng thái *đã đóng (đo được)* /
*chưa có trong VS2 (có văn bản, chuyển VS3)* / *chờ môi trường (BLOCKED-ENV, có lệnh tái lập)*, và
**không còn trạng thái "dở dang không tên"**.

Bốn việc lớn đã làm: (1) **văn bản hoá** ngữ nghĩa thật (async tuần tự, kênh không rendezvous,
HTTP không TLS, LSP một tài liệu) và **bỏ mọi số đếm khỏi tệp luật**; (2) **mở khoá môi trường Linux**
(host thật qua tailscale: Ubuntu 26.04, rustc 1.99.1, kernel có `landlock` ABI 8); (3) **đóng băng
API** (52 hàm std / 33 mã chẩn đoán / 53 thẻ VBC1) và làm **gói có ngữ nghĩa semver**; (4) **Landlock
cưỡng chế thật** bằng kernel, có ca âm ở tiến trình con.

## 2. Mười lăm task — hash và số đo chính

| Task | Hash | Số đo chính |
|---|---|---|
| ST-119 | `af69a4ae` (+`324d1da9`) | `VD-88` = **0** trong cả hai `AGENTS.md`; header `STATE.md` **35** dòng; luật không còn số đếm |
| ST-120 | `cfeee4c5` (+`26e5f82c` ST-120b, `6fb8c3de` ST-120c) | mục "Mức hiện thực VS2" + **4** dòng trỏ trong thiết kế; `doc-examples` OK (**24** khối: 18 xanh + **6** ngoại lệ) |
| ST-121 | `627c698a` (và `3f89b207`) | runner Linux: `lsm` **có** `landlock`, syscall **ABI=8**, rustc **1.99.1**; vá **2** lỗi portability của harness |
| ST-122 | `b0189956` | audit HIR **23** đơn vị đa tệp / **321** node: node sai `file` **65 → 0**; **+6** test (**1066 → 1072**); diff 100k `lech=0` |
| ST-123 | `4c87cd59` | `--grant` Proc so **đường dẫn canonicalize**; **7** ca × interp/`--vm`; bản sao trùng tên ⇒ `VS0604` exit 2; **0** ca cũ phải đổi kỳ vọng |
| ST-124 | `706dba00` | TOCTOU tái hiện **1661/50 000** (3,32 %) → **0/50 000** sau khi **mở-rồi-kiểm** (không `unsafe`); **7** ca `Value::Cap` (0 đường tạo, 0 đường lộ) |
| ST-125 | `2a1ff16a` | fuzz `json_esc`: seed **20261115**, **100 000** chuỗi × 4 phép, **lech = 0**; +ca đối chứng escaper hỏng bị bắt |
| ST-126 | `1a1614cd` | `API-STD` **52**, `API-CODES` **32** (lúc tạo), `API-VBC1` **53** mục; khoá cache **không** phủ bảng thẻ trước sửa → **có** sau sửa (**+40** dòng) |
| ST-127 | `769331ee` | semver: **7** toán tử, `,` = AND, không toán tử ⇒ `^` (kể cả `^0.x`); **19** test / **114** assert |
| ST-128 | `e20eca56` | resolver tất định + **`VS0704`** (3 ngôn ngữ); **all_codes 32 → 33**; **17** ca resolver + **7** ca CLI; 2 thư mục ⇒ lock **byte y hệt** |
| ST-129 | `13aa18f2` | lock cũ **510 B** → định dạng 2 **568 B**; `install` 2 lần cùng `sha256`; **8** ca (giữ byte, migrate, nâng trong `^1.0`, `list --json`) |
| ST-130 | `742971f2` | **37** dòng thêm / **9** bớt; **7** ca × interp/`--vm`; sem **447** không đổi; `--no-strict-caps` ⇒ 1 dòng cảnh báo **stderr** |
| ST-131 | `c9096552` | đo trên Linux: **1062/4** (macOS lúc đó **1066/0**), host suite **54/54 = 0 lệch**, 4 lệch đã liệt kê từng ca |
| ST-132 | `f9ced6c6` (+`4f785f10` lock) | `vis-host-linux`: **12/12** ca trên runner, có **2 ca âm ở tiến trình con** (kernel chặn, không qua `FsGuard`); macOS `--sandbox` **exit 3** |
| ST-133 | (chính báo cáo này) | `gate --fast` **PASS**; **1153** test / **129** suite / 0 đỏ; soak **10/10 OK, 1447 s**; macOS vs Linux: §3 |

## 3. Số khoá — macOS và Linux (cùng một commit `c96506ba`)

| Phép đo | macOS (máy dev) | Linux (runner `ps`) | Lệnh |
|---|---|---|---|
| `gate --fast` | **PASS** (soak đo 3 lần: 286 / 288 / 286 s) | **FAIL** chỉ ở `L0` (xem §4.4) | `bash viscript/bin/vis-dev gate --fast` |
| `cargo test --workspace` | **1153 / 129 suite / 0 đỏ** | **1147 / 129 suite / 6 đỏ** | `cargo test --workspace --no-fail-fast` |
| `vis test test/sem` (interp) | **447 ok** | chưa đo | `./target/debug/vis test test/sem` |
| `vis test --vm test/sem` | **447 ok, 0 SKIP** | chưa đo | `./target/debug/vis test --vm test/sem` |
| differential 100k (seed `20261115`) | **lech = 0**, **155 s** (soak: 3 lần 174,3 / 162,1 / 180,5 s) | chưa đo | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| host suite (54 ca) | **54/54** (soak 2 lần: 8 s, 9 s) | **54/54** | `bash bin/vis-host-suite` |
| ma trận nền tảng | **18/18 `check` PASS** | thiếu `wasm32` target (xem §4.4) | `bash bin/vis-targets` |
| `fmt --check` | **OK** | **OK** | `cargo fmt --all -- --check` |
| `R1` max (dòng/tệp) | **1498** (`crates/vis-types/src/tyck/expr.rs`) | — | `find … \| xargs wc -l \| sort -rn` |
| `FUNCS` / `all_codes` / `VBC1` | **52 / 33 / 53** | — | `wc -l agent/API-*.txt` (trừ dòng `#`) |
| allowlist `doc-examples` | **6** | — | `grep -vc '^#' ../docs/viscript/doc-examples-allow.txt` |
| soak 10 vòng | **10/10 OK, 1447 s** | — | `bash bin/vis-soak` |

## 4. Tám hạng mục VS2 — trạng thái cuối (theo đúng ba loại của VD-131)

| # | Hạng mục | Trạng thái | Bằng chứng / việc còn lại |
|---|---|---|---|
| 1 | **Async** | **chưa có trong VS2** (chuyển VS3, có văn bản) | VD-123: task tuần tự, một luồng, run-to-completion; không song song/suspend/GC. VS3 mới có thiết kế riêng |
| 2 | **Kênh** | **chưa có trong VS2** (chuyển VS3) | VD-123: `chan(n≥1)` đủ send/recv/close; `chan(0)` **từ chối** `VS0505`; rendezvous/select/timeout là VS3 |
| 3 | **Net/HTTP** | **chưa có trong VS2** (chuyển VS3) | VD-122: HTTP/1.1 **loopback**, không TLS/HTTP2/keep-alive; "chưa có", không phải "vĩnh viễn ngoài phạm vi" |
| 4 | **Quyền** | **đã đóng trên macOS; CHƯA đóng trên Linux** | macOS: runtime `FsGuard`/`NetGuard` + `--strict-caps` + ST-123 + ST-124 (**0/50 000** rò rỉ). Linux: Landlock thật **12/12** ca (ST-132) **nhưng** ca đua TOCTOU rò rỉ **798/50 000** ⇒ chưa đóng (xem §4.4 #2) |
| 5 | **LSP** | **đã đóng** (quyết định cuối, VD-126) | publish **chỉ** tài liệu đang mở; nợ #104 đóng ở trạng thái "không làm, có lý do" |
| 6 | **Gói** | **đã đóng** (trong phạm vi VS2) | ST-127..129: semver + resolver tất định + lock định dạng 2 + migration; **chưa** có registry mạng/chữ ký (VS3, có văn bản) |
| 7 | **Đa nền tảng** | **một phần**: Linux **đã chạy thật**, Windows/wasm **chỉ check** | ST-121/131/132: Ubuntu 26.04 chạy `cargo test`/gate/host suite; Windows/wasm vẫn `cargo check` (VD-121) |
| 8 | **Hợp chuẩn host** | **đã đóng** (nền thứ hai thật) | ST-131: **54/54** ca trên Linux, **0 lệch** so với macOS ⇒ bằng chứng nền thứ hai không còn là "chưa có runner" |

### 4.4 Sáu lệch Linux (đo ở `c96506ba`) — phải đọc trước khi tin "xanh toàn bộ"

Linux **1147/6**: sáu ca đỏ, trong đó **một ca là lỗi sản phẩm thật**:

1. `complete_giu_stderr_va_code_khac_0` — GNU grep in `argv[0]` (`/usr/bin/grep:`), BSD in `grep:`;
   record **vẫn đúng** (`stderr […]`, `code 2`). Lỗi **ca kiểm** (nợ #108 cũ → nay #121).
2. **`quyen::tests::dua_doi_symlink_lap_co_gioi_han` — RÒ RỈ 798/50 000 (1,6 %) trên Linux** trong khi
   macOS **0/50 000**. Đây là **lỗi thật**: cách "mở-rồi-kiểm" của ST-124 chưa đủ trên Linux ⇒
   hạng mục **Quyền chưa đóng trên Linux**; cần task sửa (`openat2`/`O_NOFOLLOW` hoặc Landlock bắt
   buộc khi `--sandbox`) trước khi tuyên bố.
3. `sandbox_check_luon_in_cuong_che` — kỳ vọng **macOS** (exit 3) trong khi Linux trả **0** (đúng hợp đồng).
4. `symlink_canonicalize_hai_phia` — `echo` trên runner là symlink vào coreutils multicall, từ chối
   `argv[0]` khác ⇒ **môi trường**, không phải sản phẩm.
5. `targets_wasm_vis_types` — runner **thiếu** `wasm32-unknown-unknown` (thành phần toolchain; cần Agent 0 cho phép cài).
6. `than_lenh_tren_thu_muc_that` — `.vish` trên đường dẫn `/tmp/…` (Linux) vs `/var/folders/…` (macOS);
   chưa đủ bằng chứng kết luận, đã thành nợ.

## 5. Năm câu bắt buộc (nguyên văn, VD-131)

Mọi báo cáo từ R23 phải có nguyên văn năm câu sau; câu nào hết đúng thì Agent 0 sửa bằng VD mới.

> **Async không song song.**
> **Kênh không rendezvous.**
> **Net không TLS.**
> **Quyền chỉ siết mặc định khi dùng gói.**
> **LSP chỉ publish tài liệu đang mở.**

### 6.1 Hàng QUEUE phát sinh ngoài 15 hàng của đợt

Trong đợt có **một** hàng được Agent 2 tự thêm vào QUEUE (ngoài ủy quyền của VD-119):
`ST-132b` — "ca âm Landlock thứ 3: tiến trình con mở **thư mục** ngoài phạm vi (`read_dir`)".
Hàng này **chưa được Agent 0 duyệt**, đang `READY`, và **không** nằm trong 15 hàng ST-119..ST-133
⇒ **không** ảnh hưởng định nghĩa "VS2 đóng" của VD-131, nhưng phải được Agent 0 quyết ở đợt 24
(nhận thành task thật, hoặc xoá). Đây là chỗ **quy trình bị vượt**: QUEUE thuộc Agent 0.

## 6. Nợ còn lại và việc của đợt 24

Ba agent làm song song trong đợt 23 **đánh số nợ trùng nhau** (mỗi người bắt đầu từ 105). R23 **chuẩn
hoá lại một dãy duy nhất** theo thứ tự hợp nhất, ghi rõ nguồn:

| # | Nguồn | Nội dung |
|---|---|---|
| 105 | ST-122 | `HSym` builtin vẫn `Span::new(0,0)` — bảng ký hiệu dùng chung, builtin không thuộc tệp nào; cần chốt `Option<Span>`/`u16::MAX` |
| 106 | ST-122 | `RecDef`/`FieldDef` prelude + `cmdsig` vẫn `file=0`, audit chưa phủ (`TypeTable`) |
| 107 | ST-122 | `test/sem` **không** chứa được ca đa tệp (5 harness một tầng + `vistest::gom` đệ quy) |
| 108 | ST-123 | `Command::new(tên)` để OS phân giải `PATH` lần nữa sau khi kiểm |
| 109 | ST-124 | **đường GHI** vẫn kiểm-rồi-mở ⇒ cần một dòng giới hạn trong `HOST-BAO-DAM.md` (Agent 0 ghi) |
| 110 | ST-124 | VM **không** đưa `argv` vào tham số không-`cap` của `main` (interp `list[str]`, VM `()`) |
| 111 | ST-124 | tyck chấp nhận `let e: cap[Dir] = json.decode(...)` |
| 112 | ST-124 | thực tế có **2** bộ escape JSON (`json_esc` + `json_tai`), VD-130e "nguồn duy nhất" chưa đúng |
| 113 | ST-126 | `TEN_THE` không ràng buộc lúc biên dịch |
| 114 | ST-126 | hash nguồn VBC1 ≠ khoá cache (hai giá trị khác nhau, không đối chiếu ở đâu) |
| 115 | ST-128 | quy ước "điền 0" khi so sánh thiếu thành phần phiên bản |
| 116 | ST-128 | `next_block()` trả 801 trong khi khối 07xx còn chỗ ⇒ cấp số mã mới phải chọn tay |
| 117 | ST-128 | `@1.2.3` = ghim, khác "không toán tử ⇒ `^`" |
| 118 | ST-128/129 | yanked, registry mạng, chữ ký (VS3) |
| 119 | ST-129 | lock không ghi quan hệ phụ thuộc giữa gói |
| 120 | ST-130 | `vis test`/host suite chưa áp strict-caps |
| 121 | ST-131 | `pipeline.rs` bám tiền tố thông điệp grep (lệch 1 §4.4) |
| 122 | ST-131 | `sandbox.rs:59` chưa `cfg` theo nền (lệch 3) |
| 123 | ST-131 | runner thiếu `wasm32` target (lệch 5) |
| 124 | ST-131 | `vish_cmd_body.rs:312` đường dẫn thư mục tạm (lệch 6) |
| 125 | ST-132 | `--sandbox` chưa cấp `EXECUTE` cho `/lib*` ⇒ lệnh ngoài không chạy được dưới sandbox |
| 126 | ST-132 | `unicode-ident` (`(MIT OR Apache-2.0) AND Unicode-3.0`, build-time) **ngoài allowlist hẹp** — **cần Agent 0 quyết** |
| 127 | ST-132 | `ap_dung` mới dùng `ABI::V1`; ABI 8 có thêm `REFER`/`TRUNCATE`/`IOCTL_DEV` chưa xét |
| 128 | ST-133 | **RÒ RỈ TOCTOU trên Linux 798/50 000** — lỗi sản phẩm, ưu tiên cao nhất đợt 24 |
| 129 | ST-133 | cùng danh sách **số dòng khối tài liệu** bị chép ở **4** tệp test (`check.rs`, `fmt.rs`, `doc_fixtures.rs`, `hir.rs`) — nên gom một nguồn |
| 130 | ST-133 | Linux chưa chạy `vis test`/differential/cache (mới có `cargo test` + host suite) |
| 131 | ST-133 | hàng `ST-132b` do Agent 2 tự thêm vào QUEUE (ngoài ủy quyền) — cần Agent 0 nhận hoặc xoá |

### Việc đợt 24 (đề xuất, theo thứ tự ưu tiên)

1. Sửa **rò rỉ TOCTOU trên Linux** (#128) — đây là chỗ duy nhất phá cam kết "quyền được cưỡng chế".
2. Quyết **giấy phép `unicode-ident`** (#126) — chấp nhận / gỡ `landlock` / hoãn.
3. Chạy **nốt** `vis test`, differential, cache trên Linux (#130) để hạng mục 7 có số đủ hai nền.
4. Siết `cfg(target_os)` cho các ca kiểm đang giả định macOS (#121..#124).
5. Gom danh sách dòng-khối về một nguồn (#129).
6. Duyệt hàng loạt 15 hàng `CHỜ DUYỆT` và cập nhật `ROADMAP.md` + `docs/` theo quyết định mới.
