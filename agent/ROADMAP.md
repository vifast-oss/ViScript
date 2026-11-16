<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ROADMAP bàn giao — dòng ViScript (sau R22 / 22 vòng, 2026-11-15)

Tài liệu một trang cho người tiếp nhận: 8 hạng mục VS2, mức thật **đo được**, điều kiện đóng 100%,
nơi đọc và lệnh chạy. Không mục nào “xong” nếu chưa đạt cột điều kiện. Số đo cuối: `reports/R22-viscript.md`.

## A. Tám hạng mục VS2 — mức thật cuối (không vượt mức)

| # | Hạng mục | Mức thật (đo được) | Bằng chứng | Còn thiếu để đóng 100% |
|---|---|---|---|---|
| 1 | **Async** | `async fn`/`await`/`spawn`/join; scheduler **một luồng, run-to-completion**, **không song song thật**, **không GC**; 10k task interp **316 ms** / VM **168 ms**; drain 100 đúng thứ tự | ST-60/61/94/98; `async_10k`; R22 §3 | **Song song thật**/work-stealing + suspend-resume; test tăng tốc ≥2 core; async method/cmd; GC/ownership |
| 2 | **Kênh** | `chan(n≥1)` send/recv/close; **`chan(0)` bị từ chối** `VS0505` ngay khi tạo (**không** rendezvous); `recv` sau close `none`; `send` sau close `VS0504`; deadlock `VS0505` không treo | ST-61/94; `kenh.rs`/`kenh_cap0.rs`; soak 10/10 | **Wait-set/rendezvous** `chan(0)` (hoặc tuyên bố từ chối vĩnh viễn có văn bản); `select`/timeout; test hai đường |
| 3 | **Net/HTTP** | Loopback TCP thật; **chunked** (+`;ext`, trailer bỏ), **redirect GET ≤5** (Location `..`/query-only/fragment/rỗng), **`http.head` không thân/không redirect**, timeout đọc 2 s, cap 8 MiB; **không TLS/HTTP2/keep-alive**; 21 ca http | ST-64/70/71/104/109; `net.rs` 463; R22 §3 | **TLS** (duyệt crate rustls/webpki), HTTP2, keep-alive, cookie/auth, HEAD redirect; ≥2 kết nối đồng thời; test Internet thật |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; `--strict-caps`; `--grant` Dir/File/Net = đường dẫn, **Proc = allowlist** (basename/canonical); `VS0604` exit 2; **chưa siết mặc định**; **chưa Landlock thật**; con **không sandbox** | ST-72b/81/93/97; `quyen.rs` 460 | **Landlock thật** Linux (duyệt crate `landlock` **hoặc** `unsafe` đóng) + **runner Linux**; siết mặc định; sandbox con |
| 5 | **LSP** | `--stdio`; diagnostics **module-aware**, UTF-16; **4 ca chẩn đoán đa tệp đúng tài liệu mở** (ST-117); completion sau `.`; **hover trường** + `didClose`; cache module phụ **0,466 ms** | ST-91/100/117; `lsp_std*.rs` 43 + `lsp_da_tep.rs` 4; R22 §3 | **Publish chẩn đoán tệp phụ**, hover method, generic/`list[P]`, bảng `std.str`, incremental sync |
| 6 | **Gói** | Registry **local**, manifest `files[]`, lock SHA-256, `.vis/store`, `use pkg.*`, `VS0701/0702`, lock `entry`, `pkg update` idempotent, `list --json`, `remove`; **chưa** resolver/HTTP/ký số | ST-64/66/69/80/90/102; `pkg.rs` 927 | **Semver ranges** + resolver; registry HTTP; chữ ký số; GC store; migration lock cũ |
| 7 | **Đa nền tảng** | **Chỉ `cargo check`**: ma trận **18/18 `PASS`**; **chưa runner** Windows/Linux/wasm | ST-58/73/86; `vis-targets` 18/18; R22 §3 | **Runner thật**: wasm32-wasip1 (wasmtime), Linux (Landlock), Windows; CI 54 host trên ≥2 nền |
| 8 | **Hợp chuẩn host** | **54 ca chạy thật macOS ×2, 0 lệch**; **VM 54/54**; cache **520×4 0 lệch**; net **không** nằm trong suite | ST-87/88/89/98; `HOST-BAO-DAM.md`; soak 10/10 | 54 ca × **4 nền thật**; net/TLS trong suite; CI lặp; đo hiệu năng từng nền |

### Quyết định Agent 0 cho điều kiện đóng (2026-11-16, VD-119..VD-131)

| # | Hạng mục | Trạng thái sau quyết định | Việc |
|---|---|---|---|
| 1 | Async | Chưa có trong VS2: song song, suspend, GC (VD-123) | VS3 thiết kế riêng |
| 2 | Kênh | Chưa có trong VS2: rendezvous, select, timeout (VD-123) | VS3 |
| 3 | Net/HTTP | Chưa có trong VS2: TLS, HTTP/2, keep-alive (VD-122) | VS3 |
| 4 | Quyền | Siết khi dùng gói: ST-130. Landlock: ST-132, chờ runner Linux | ST-130, ST-131, ST-132 |
| 5 | LSP | Một tài liệu là quyết định cuối (VD-126); nợ #104 đóng | không |
| 6 | Gói | Ranges + resolver: ST-127..129. Mạng/chữ ký: VS3 | ST-127..129 |
| 7 | Đa nền tảng | Linux: ST-121, ST-131. Windows, wasm: chỉ check trong VS2 (VD-121) | ST-121, ST-131 |
| 8 | Hợp chuẩn host | Nền thứ hai = Linux qua ST-131 | ST-131 |

R1: ngưỡng duy nhất ≤1.500 dòng/tệp (VD-128); bỏ mục tiêu nội bộ ≤1.000.

## B. Lệnh chạy (tuần tự, **không** song song build/cổng khác — AGENTS §6)

```bash
cd viscript
cargo test --workspace                  # 1066 test
cargo test -p vis-cli --test sem        # 447 ca / 22 nhóm
target/debug/vis test test/sem          # 447 ok (interp)
target/debug/vis test --vm test/sem     # 447 ok + 0 SKIP (VM)
../vis-dev gate --fast                  # fmt, clippy, R1..R4, L0, report-check
bin/vis-soak                            # 10 vòng: gate×3, diff×3, host×2, cache×2 (1297 s lần cuối)
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff   # 0 lệch
target/release/vis-parse-fuzz --seconds 60 --seed 20261115 # 0 vi phạm
./bin/vis-host-suite                    # 54/54 macOS
./bin/vis-targets                       # 18/18 check (CHỈ check)
```

## C. Nợ correctness đa tệp — **ĐÃ ĐÓNG** (ST-112 → ST-114b)

- `Span.file` + `lex_file`/`parse_file`/`LowerOptions.file`; `Nap.file_rel` chính **0**, module theo `use`;
  tyck dùng `d.span.file` (fallback heuristic cũ **chỉ** ngoài bảng + log); `vi_tri` đúng văn bản tệp.
- **6 ca** `chan_doan_da_tep.rs` red-first (5/6 đỏ → 6/6 xanh) + 4 ca LSP `lsp_da_tep.rs`; JSON/TAP đúng
  tệp/dòng cho a/b/cả hai/TAP/b dài.

## D. Điều kiện đóng **100%** (giữ nguyên danh sách; thêm số liệu cuối)

1. **`Span` tổng hợp `file=0`** — audit các node lower sinh; LSP **chưa publish chẩn đoán tệp phụ**.
2. **Landlock thật** Linux (crate `landlock` **hoặc** `unsafe` được duyệt) + **runner Linux** 54 host.
3. **Runner wasm32-wasip1** (wasmtime) + **Windows**; CI 4 nền.
4. **TLS/HTTP2/keep-alive** + HEAD redirect/cookie; test Internet thật.
5. **Async song song thật** + wait-set/rendezvous `chan(0)` (hoặc tuyên bố vĩnh viễn có văn bản).
6. **Siết mặc định caps** + sandbox tiến trình con; cập nhật host suite.
7. **Resolver/registry HTTP/ký số** cho gói + GC store.
8. **R1 nội bộ ≤1.000**: `lower/cmd.rs` **1060**, `parse/cmd.rs` **1007** — mục tiêu nội bộ, **không**
   phải luật R1 (luật là ≤1.500; max hiện tại **1060**); `main.rs` 591, `parse/mod.rs` 1069.
9. **100 nợ R21** + nợ mới đóng hết; **R23** xác nhận lại toàn bộ số đo.

**Hiện trạng R22**: **1066 test**, sem **447/22**, `--vm` **0 SKIP**, parity **398+49**, diff **0 lệch**,
cache **520×4 0 lệch**, host **54/54 ×2**, soak **10/10 (1297 s)**, fuzz **0 vi phạm**, `FUNCS` **52**,
`all_codes` **32**, `VBC1` **53**, targets **18/18 check**, R1 max **1060**. **VS2 chưa đóng** — 9 điều kiện.

## E. Nơi đọc

- [`QUEUE.md`](QUEUE.md) **134 hàng**; [`DECISIONS.md`](DECISIONS.md) VD-01..VD-118.
- `reports/R1..R22` (bắt đầu R21/R22); [`HOST-BAO-DAM.md`](HOST-BAO-DAM.md) — host/HTTP.
- [`STATE.md`](STATE.md) — trạng thái hiện tại.

### Trạng thái cuối sau R23 (đo ở `c96506ba`, soak 10/10 = 1447 s)

| # | Hạng mục | Trạng thái (đúng 3 loại của VD-131) | Ghi chú |
|---|---|---|---|
| 1 | Async | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | Kênh | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 |
| 3 | Net/HTTP | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback; không TLS/HTTP2/keep-alive (VD-122) |
| 4 | Quyền | **macOS đã đóng · Linux CHƯA đóng** | Landlock thật 12/12 ca trên Linux (ST-132); **rò rỉ TOCTOU 798/50 000 trên Linux** ⇒ nợ #128 |
| 5 | LSP | **đã đóng** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | Gói | **đã đóng** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration; mạng/chữ ký = VS3 |
| 7 | Đa nền tảng | **một phần** | Linux **chạy thật** (ST-131); Windows/wasm **chỉ check** (VD-121) |
| 8 | Hợp chuẩn host | **đã đóng** | 54/54 ca trên **cả** macOS và Linux, 0 lệch |

**Số khoá R23**: 1153 test (129 suite) macOS · 1147/6 Linux · sem 447/22 · `vis test` 447 ok · `--vm` 447 ok + 0 SKIP ·
differential 100k `lech=0` (3 lần: 174/162/181 s) · cache 520×4 `lech=0` · host 54/54 hai nền · targets 18/18 `check` ·
R1 max **1498** · FUNCS **52** · all_codes **33** · VBC1 **53** · allowlist **6** · soak **10/10 = 1447 s**.

### Trạng thái cuối **sau đợt 25** — R26, `head` `b1c07802` (2026-11-17)

> Khối này **thêm** sau khối R23; số cũ **không xoá** (VD-142). Nguồn: `reports/R26-viscript.md`.

**VS2 đóng về kỹ thuật 8/8 hạng mục** (**VD-151**, 2026-11-17). Từ đây **không** mở task VS2 mới trừ: nợ đã có
số trong [`SO-NO.md`](SO-NO.md), đính chính tài liệu, và báo cáo vòng. **Không** bắt đầu tính năng **VS3** cho tới
khi Agent 0 ban hành **tài liệu thiết kế VS3** (VD-148 giữ nguyên; hiến chương VS3 = `JOURNAL/quyet-dinh-a0-2026-11-17.md` §5).

| # | Hạng mục | Trạng thái (3 loại của VD-131) | Ghi chú |
|---|---|---|---|
| 1 | Async | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | Kênh | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 |
| 3 | Net/HTTP | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback; không TLS/HTTP2/keep-alive (VD-122) |
| 4 | Quyền | **ĐÓNG** (macOS giới hạn VD-145) | Linux Landlock thật **14/14** ca (thêm ca âm **GHI** ở ST-150) + TOCTOU **0/50 000**; macOS bảo đảm theo **đo** |
| 5 | LSP | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | Gói | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2; mạng/chữ ký = VS3 |
| 7 | Đa nền tảng | **ĐÓNG theo nghĩa kỹ thuật** | Linux **"chạy thật"** (`ps` gate PASS L0 **1169/0**) · Windows **"chỉ đo"** · wasm **"chỉ check"** |
| 8 | Hợp chuẩn host | **ĐÓNG** | 54/54 trên **macOS và Linux**; `vis-host-linux` **14/14**; Windows **39/54** (chỉ đo) |

**Số khoá sau đợt 25** (R26 §3; log trong `JOURNAL/`, hash đo từng nền theo VD-147(4)):

```
macOS (@ b1c07802) : gate --fast PASS · L0 **1175/0** (**130** suite; = 1166 + 9 ca ST-151) · 10/10 bước
        vis test 447/447 ×2 (0 SKIP) · diff 100k lech=0 · cache 520×4 0 lệch · host 54/54 · targets 18/18
        soak 10/10 = 1352 s (**đo ở HEAD cũ** 3079dede — không chạy lại) · R1 1498 · FUNCS 52 · all_codes 33 · VBC1 53
Linux ps (@ 32b79d4a/4a805115): gate --fast PASS · L0 **1169/0** (129 suite) — **số TRƯỚC 9 ca ST-151, CHƯA đo lại**
        · cargo test 1162/0 (ST-140) · `vis-host-linux` **14/14** (@ 9eeb936b) · vis test 447/447 ×2 · diff lech=0
        · cache 0 lệch · host 54/54 · targets 18/18 · soak 10/10 = 250 s (@ 2866f8c2) · TOCTOU 0/50 000
Windows psvn (@ 3dc13342; cổng @ 4a805115) — **CHỈ ĐO, "chưa hỗ trợ"**:
        self-test **12/12** (7/12 → 11/12 → 12/12) · cổng đi **qua 9/10 bước**, dừng ở **L0** (fail-fast `cap_proc`
        **150/4** vì `/bin/echo` — VS3) · cargo test **1053/100** · host **39/54** · targets **18/18**
Sổ nợ : **144 mục** = **91 mở** / **31 đóng** / **20 VS3** / 1 không làm / 1 điều hướng (`SO-NO.md`; §8)
```

**Đợt 25 đã đóng**: **6/6 hàng** (ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-153) — nợ **#136**, **#133**,
**#135** và **ca 7 self-test của ST-147** (không cấp số mới); thêm số **#142/#143** (VD-149).
**Còn lại**: nợ trong sổ (**91 mở** / **20 VS3**) và **Windows** (hỗ trợ thật = **VS3**).

### Trạng thái cuối **sau R26** — đêm 17/11 (2026-11-17), `head` `dd088322`

> Khối này **thêm** sau khối R26; số cũ **không xoá** (VD-142). Nguồn: `reports/R27-viscript.md`.

**Phần còn lại sau R26, đêm 17/11 đóng nốt** (ST-152 = R26 · **ST-154** · **ST-155** · **ST-156** = R27):

| Việc | Nội dung | Trạng thái |
|---|---|---|
| Nợ **#144** | Ca **dương** `read_dir` trên thư mục **có quyền ghi** (`vis-host-linux` ca 15): `exit 0` + `co.txt`, đối chứng ghi `exit 0`, âm cùng ca `exit 7` + `EACCES` | **ĐÓNG** (ST-154) — `ps` **15/15** |
| Nợ **#143** | `NamePolicy::Strict` cho mọi tệp có `use` — điều kiện VD-133 đã thoả (ca `st130b` xanh) | **chuyển VS3** (VD-154(b)) |
| Nợ **#145** | Chú thích `rt.rs` nói **sai** đường chặn `chan(0)`: tyck **không** kiểm `cap`, runtime mới chặn (`exit 1`) | **mở** — xử khi có task VS3 kênh |
| **Bộ đầu vào VS3** | `JOURNAL/st155-vs3-dieu-tra.md` **222 dòng**: 6 mục × 5 khối, **57** dẫn chứng `tệp:dòng`, `[ƯL]` **19–28 task ≈ 57–84 h**, §8 Top 5 câu hỏi, §9 6 việc đề nghị KHÔNG làm | **ĐÃ CÓ** (ST-155) — **chưa** có tài liệu thiết kế ⇒ **chưa** mở tính năng VS3 (VD-148) |

| # | Hạng mục | Trạng thái (3 loại của VD-131) | Ghi chú |
|---|---|---|---|
| 1 | Async | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | Kênh | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3; **#145** ghi lệch tài liệu–mã |
| 3 | Net/HTTP | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback; không TLS/HTTP2/keep-alive (VD-122) |
| 4 | Quyền | **ĐÓNG** (macOS giới hạn VD-145) | Linux Landlock thật **15/15** (thêm ca **dương** `read_dir` — đóng #144) + TOCTOU **0/50 000**; macOS bảo đảm theo **đo** |
| 5 | LSP | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | Gói | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2; mạng/chữ ký = VS3 |
| 7 | Đa nền tảng | **ĐÓNG theo nghĩa kỹ thuật** | Linux **"chạy thật"** (`ps` gate PASS L0 **1169/0**) · Windows **"chỉ đo"** (**9/10 bước**, self-test **12/12**, dừng **L0**) · wasm **"chỉ check"** |
| 8 | Hợp chuẩn host | **ĐÓNG** | 54/54 trên **macOS và Linux**; `vis-host-linux` **15/15**; Windows **39/54** (chỉ đo) |

**Số khoá sau đêm 17/11** (R27 §3; log trong `JOURNAL/`, hash đo từng nền theo VD-147(4)):

```
macOS (@ dd088322) : gate --fast PASS · L0 **1175/0** (**130** suite) · 10/10 bước · check_report **27** báo cáo / **174** hàng
        vis test 447/447 ×2 (0 SKIP) · diff 100k lech=0 (157,4 s) · cache 520×4 0 lệch (25,43 s) · host 54/54 · targets 18/18
        soak 10/10 = 1352 s (**đo ở HEAD cũ** 3079dede — không chạy lại) · R1 1498 · FUNCS 52 · all_codes 33 · VBC1 53
Linux ps (@ 32b79d4a; Landlock @ d203b664; vis test @ ee58a518; soak @ 2866f8c2): gate --fast PASS · L0 **1169/0**
        (129 suite) — **số TRƯỚC 9 ca ST-151, CHƯA đo lại** · `vis-host-linux` **15/15** · vis test 447/447 ×2
        · diff lech=0 (40,9 s) · cache 0 lệch (6,33 s) · host 54/54 · targets 18/18 · soak 10/10 = 250 s · TOCTOU 0/50 000
Windows psvn (@ 3dc13342; cổng @ 4a805115) — **CHỈ ĐO, "chưa hỗ trợ"**:
        self-test **12/12** · cổng đi **qua 9/10 bước**, dừng ở **L0** (`cap_proc` **150/4** vì `/bin/echo` — VS3)
        · cargo test **1053/100** · host **39/54** · targets **18/18**
Sổ nợ : **146 mục** = **91 mở** / **32 đóng** / **21 VS3** / 1 không làm / 1 điều hướng (`SO-NO.md` §8; VD-155)
```

**Đêm 17/11 đã đóng**: **8 hàng công việc** (ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-153 · **ST-154** ·
**ST-155**) + **2 hàng báo cáo vòng** (ST-152 · ST-156); **không** hàng nào `BLOCKED`.
**Còn lại**: nợ trong sổ (**91 mở** / **21 VS3**) và **Windows** (hỗ trợ thật = **VS3**).
**Việc kế**: **Agent 0** trả lời **5 câu hỏi** (`st155` §8) rồi ban hành **tài liệu thiết kế VS3** (6 quyết định riêng).
