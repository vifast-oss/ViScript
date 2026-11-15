<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R18-viscript — báo cáo vòng 18 (HTTP thật hơn, `--timeout`, tách `cmd.rs`), dòng ViScript

Agent 2 · **2026-11-14/15** · `head` = **`80067ef0`** (commit nền: `QUEUE@head` đủ **121 hàng**, **120
`DONE`** + ST-107 `READY` — báo cáo này). Số đo chạy trên mã **`5d7b1353`** (ST-106; sau đó chỉ
`QUEUE.md` + báo cáo). Bản máy đọc: [`R18-viscript.json`](R18-viscript.json) — **đúng 11 khoá** Khối E
(`project = "viscript"`, `round = 18`). Như tiền lệ R11–R17: `head` = commit **nền** của báo cáo.

**Bàn giao**: [`../ROADMAP.md`](../ROADMAP.md) (55 dòng) — vẫn là gói bàn giao; R18 **không** đổi 8 mức
VS2.

## 1. Tóm tắt (5 dòng)

1. **ST-104 — HTTP thật hơn**: `Transfer-Encoding: chunked` (bỏ `;ext`, dừng chunk `0`), **redirect tối đa
   5** (`Location` tuyệt đối/tương đối, giữ GET), `Content-Length` đọc **đúng N** rồi trả, **timeout đọc
   2 s**; server im lặng lỗi **2,01 s** (trước 3,3 s) — 7 ca, 16 ca http.
2. **ST-105 — `vis test --timeout`**: `--timeout <ms>` ≥50 (mặc định không giới hạn); poll `try_wait` 10
   ms, quá hạn **kill + reap**, `not ok` + `quá hạn <ms> ms`, **tiếp tục** ca sau + JSON; ca treo 300 ms,
   2 ca **0,89 s**, `ps` không còn con.
3. **ST-106 — tách `lower/cmd.rs`**: **1491 → 1060**, `cmd/doc.rs` **235**, `cmd/hop.rs` **213**; thuần
   refactor; **R1 max 1491 → 1482** (`lower.rs`); checksum 1045 test + sha256 diff **trùng**.
4. **Số đo**: **1045 test**, sem **447/22**, `vis test` **447 ok** (`--vm` **447 ok + 0 SKIP**), host
   **54/54 ×2**, parity **398/398 + 49**, differential **100k / 0 lệch** (177,1 s), cache **520×4 / 0
   lệch**, fuzz **2.100.646 ca / 0 vi phạm**, targets **18/18 check**, fmt **497** (+8 cảnh báo).
5. **Trung thực**: vòng 18 vẫn **VS2 không đổi**; chỉ thêm bằng chứng công cụ (HTTP, timeout, R1). Vẫn
   **chưa TLS/HTTP2/keep-alive**, redirect **chỉ GET**, runner **4 nền chưa có**, Landlock **chưa thật**,
   async **một luồng**; nợ **82 → 91**.

## 2. Bảng vòng 18

| Task | Commit | Kết quả (1 dòng, số đo tại lúc đóng) |
|---|---|---|
| ST-104 | `2b55b102` | **chunked + redirect ≤5 + CL/timeout đọc**: `abcdef`/`aabb`, `abs`/`rel`, server im lặng **2,01 s**; 7 ca; **1040 test** |
| ST-105 | `c3afa7b1` | **`test --timeout`**: poll 10 ms + kill/reap, `quá hạn 300 ms`; 2 ca **0,89 s**, không rò; 5 ca; **1045 test** |
| ST-106 | `5d7b1353` | **tách `cmd.rs`**: 1491→1060 + 235 + 213; R1 max **1491→1482**; checksum test/diff trùng; **1045 test** |
| ST-107 | (commit này) | **R18**; `report-check --all` PASS; số liệu chạy lại §3 |

## 3. Số liệu chạy lại (mã `5d7b1353`, đo 2026-11-15, macOS aarch64)

| Phép đo | Kết quả | Lệnh |
|---|---|---|
| `cargo test --workspace` | **1045** = 21 diag + 96 syntax + 27 fmt + 154 types + 26 interp + 62 bytecode + 9 vm + **650 cli** | `cd viscript && cargo test --workspace` |
| Ca ngữ nghĩa | **447 / 22 nhóm** (397 `.vis` + 50 `.vish`; **83** `.err`) | `cargo test -p vis-cli --test sem` |
| `vis test` / `--vm` | **447 ok**, 0 `not ok` / **447 ok, 0 SKIP, 0 đỏ** | `vis test test/sem` / `vis test --vm test/sem` |
| Parity interp ↔ VM | **398/398** khớp byte + **49** lỗi biên dịch (447) | `cargo test -p vis-vm --test vs_interp` |
| `diag_cmp` ba tầng | **364 xanh / 49 biên dịch / 34 lúc chạy** | `cargo test -p vis-cli --test diag_cmp` |
| Differential | **`tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=177.1 toc_do=565/s seed=20261115`** | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| Fuzz parser 60 s | **2.100.646 ca, 47.798.025 token, 35.011 ca/s, 796.634 token/s, 60,0 s, vi phạm 0** | `target/release/vis-parse-fuzz --seconds 60 --seed 20261115` |
| Cache harness | **520 tệp × 4 lần**, **520 ca so đủ 4 đường, 0 lệch**, **23,92 s** | `cargo test -p vis-cli --test cache_khong_che -- --nocapture` |
| Host suite | **54/54 × 2 lần**, 0 lệch; VM 54/54 | `./bin/vis-host-suite` (hai lần) |
| Soak | **8/8 vòng OK, 818 s** — lần cuối ST-98, **không** chạy lại | `bin/vis-soak` |
| `VBC1` | **53 tag** (không bump) | `Instr::tag` |
| Sổ mã | **7 sổ / 32 mã** | `vis explain --list` |
| Allowlist doc-examples | **6** mục | `grep -v '^#' docs/viscript/doc-examples-allow.txt` |
| Ma trận target | **18/18 `PASS`** — chỉ `cargo check` | `./bin/vis-targets` |
| `fmt --check test/` | **exit 0**, **497 tệp** (**429** `.vis` + **68** `.vish`), **8 cảnh báo** thụt lề | `vis fmt --check test` |
| `--indent-report` | **8 tệp / 10 dòng lệch** — không sửa | `vis fmt --indent-report test` |
| R1 | **max 1491 → 1482**; `cmd.rs` **1491 → 1060**, `doc.rs` **235**, `hop.rs` **213** | `wc -l crates/vis-types/src/lower*` |
| Báo cáo | **18 báo cáo**, **121 hàng QUEUE** phủ đủ; `R18` 11 khoá, `round=18`, `head=80067ef0` | `python3 viscript/test/report/check_report.py --all` |
| Cổng | `gate --fast` **PASS** (không retry) | `./vis-dev gate --fast` |

### 3b. Bằng chứng công cụ **mới** vòng 18

- **Chunked**: server trả 3 chunk `"a"`,`"bc"`,`"def"` ⇒ thân **`abcdef`**; chunk mở rộng `3;ext=1` ⇒ bỏ
  ext, thân **`aabb`**; `Content-Length` bị **bỏ qua** khi có chunked.
- **Redirect**: `/a` → `http://addr/b` ⇒ **`abs`**; `/a`→`/b`→`/c` ⇒ **`rel`**; vượt 5 ⇒
  **`quá 5 lần chuyển hướng HTTP (tối đa 5)`**; thiếu `Location` ⇒ mã gốc.
- **Timeout đọc**: server nhận request nhưng không trả lời ⇒ **`quá thời gian chờ dữ liệu … sau 2000 ms`**,
  wall-clock **2,01 s** (trước: chờ server đóng **3,3 s**); `Content-Length` + không đóng ⇒ trả `abc` ngay.
- **`test --timeout 300`**: TAP
  `not ok 1 - treo` / `# treo: quá hạn 300 ms` / `ok 2 - nhanh`; JSON
  `{"ok":false,"message":"quá hạn 300 ms"}`; 2 ca **0,89 s**; không còn tiến trình con (`ps` ppid).
- **Refactor `cmd.rs`**: checksum 1045 dòng test **`5c194bb8…` trước = sau**; sha256 dòng bất biến diff
  **`62bdd699…` trước = sau**; `cmd.rs` **1491 → 1060**; **R1 max 1491 → 1482**.

## 4. §19 — đối chiếu **VS2** sau 18 vòng: mức thật (không đổi; thêm bằng chứng)

| # | Hạng mục VS2 | Mức thật | Bằng chứng mới R18 | Còn thiếu |
|---|---|---|---|---|
| 1 | **Async** | Một luồng run-to-completion; 10k **316/168 ms**; drain 100 | 100 task đúng thứ tự (ST-94) giữ | **Song song thật**, async method/cmd, GC |
| 2 | **Kênh** | `chan(n≥1)`; **`chan(0)` bị từ chối** (`VS0505`); close/deadlock/drain | giữ | **Wait-set/rendezvous**, `select`/timeout |
| 3 | **Net/HTTP** | Loopback thật; **chunked + redirect ≤5 + CL/timeout** (ST-104); **không TLS/HTTP2/keep-alive**, redirect chỉ GET | body chunked `abcdef`, redirect `abs`/`rel`, lỗi **2,01 s** | **TLS**, HTTP2, keep-alive, cookie/auth, chunk trailer |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; `--strict-caps`; Proc allowlist; **chưa mặc định**, **chưa Landlock** | giữ | **Landlock thật**, mặc định caps, sandbox con |
| 5 | **LSP** | diagnostics module-aware; completion; **hover trường** (ST-100); cache **0,466 ms** | giữ (R17) | Hover method, đa tài liệu, generic |
| 6 | **Gói** | Registry local, lock SHA-256; **`update` + `list --json`** (ST-102) | giữ | Resolver/version, HTTP, ký số, GC |
| 7 | **Đa nền tảng** | **Chỉ check 18/18**; chưa runner Windows/Linux/wasm | giữ | **Runner thật** 4 nền |
| 8 | **Hợp chuẩn host** | **54/54 macOS ×2**, VM 54/54; cache 520×4 | host **54/54 ×2** | Windows/Linux/wasm, net/TLS trong suite |

**Bổ sung bằng chứng**: HTTP **xử lý chunked/redirect/CL** thay vì chỉ 200; `vis test` có **timeout + kill**;
`cmd.rs` tách làm **R1 max giảm** — nhưng **không** hạng mục nào lên mức.

## 5. Lỗ hổng còn mở (R17 **82** + mới **83..91** = **91 mục**)

**1..82** — giữ nguyên [`R17-viscript.md`](R17-viscript.md) §5. Chín mục mới vòng 18:

83. **Chưa TLS/HTTP2/keep-alive**: `http.get` chỉ `http://`, một request/kết nối, không nén.
84. **Redirect chỉ GET/không cookie-auth**: 303 không đổi POST, không giữ `Cookie`/`Authorization`, không
    giới hạn host đích ngoài loopback (vẫn `tach_url` chặn).
85. **Chunk trailer bị bỏ qua**: dừng ở chunk `0`, không đọc/không kiểm trailer.
86. **`Location` tương đối chưa resolve `..`/query-only**: chỉ ghép thư mục hiện tại; `../x`/`?q` chưa chuẩn.
87. **`--timeout` chỉ `test fn`**: fixture `.out/.err` không có timeout; không timeout theo ca/bộ.
88. **`--timeout` không kill cây cháu**: chỉ kill `vis run`; cháu (nếu có) có thể sống.
89. **Pipe stdout/stderr có thể nghẽn**: `wait_with_output` sau khi con kết thúc; ca in quá buffer có thể treo.
90. **`lower.rs` 1482 vẫn max R1**: ST-106 chỉ tách `cmd.rs`; muốn max ≤1200 phải tách `lower.rs`.
91. **`ha_stage` 782 vẫn lớn**: chưa tách tiếp; `cmd.rs` còn 1060.

## 6. VD-114

**VD-114 — Vòng 18: HTTP client thật hơn + test timeout + tách cmd (ST-104..ST-107)**: (1) **HTTP** —
chunked (bỏ `;ext`, dừng `0`, không đọc CL), redirect **tối đa 5** (tuyệt đối/tương đối, giữ GET, thiếu
`Location` trả mã gốc, lần 6 lỗi rõ), CL đọc đúng N, mỗi lần đọc cap **2 s** ⇒ server im lặng **2,01 s**;
(2) **`vis test --timeout`** — poll `try_wait` 10 ms, kill + reap, `not ok` + `quá hạn <ms> ms`, tiếp tục
ca sau + JSON, **chỉ `test fn`**, không rò tiến trình; (3) **refactor** — `cmd.rs` **1491 → 1060**,
`cmd/doc.rs` 235, `cmd/hop.rs` 213, R1 max **1491 → 1482**, checksum test/diff trùng. **Đính chính**: mức
VS2 **không đổi**; đây là bằng chứng công cụ, không nới mức.

## 7. Ghi chú máy kiểm + kết luận

- `python3 test/report/check_report.py --all`: **PASS** — **18 báo cáo**, **121 hàng QUEUE** phủ đủ;
  `R18-viscript.json` đúng **11 khoá**, `round=18`, `head=80067ef0` (tồn tại + là tổ tiên HEAD).
- Cổng: `./vis-dev gate --fast` **PASS**, không retry; `fmt`/`clippy -D warnings`/R1..R4 OK; report-check
  trong cổng PASS; differential 100k **0 lệch**; fuzz **0 vi phạm**; cache **0 lệch**.
- Không thêm `#[ignore]`, không nới ngưỡng, không sửa golden/allowlist; `VBC1` **53**; **32 mã**; không
  sửa `R1..R17`; không sửa `docs/`, `v2/`, `spec/`, QUEUE/DECISIONS.
- **Kết luận trung thực**: sau **18 vòng**, phần đo được là **1045 test**, **447 sem**, `--vm` **0 SKIP**,
  parity **398+49**, differential **0 lệch**, cache **520×4 0 lệch**, host **54/54 ×2**, soak **8/8**,
  fuzz **0 vi phạm**; thêm **HTTP chunked/redirect**, **test timeout có kill**, **R1 max giảm**. Nhưng
  **VS2 không đổi**: TLS, runner 4 nền, Landlock thật, async song song, rendezvous, quyền mặc định vẫn mở
  — nợ **91 mục** (1..91), không có “đã xong”.
