<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R19-viscript — báo cáo vòng 19 (`lower.rs` ≤1.200, HTTP Location/HEAD, JSON theo tệp), dòng ViScript

Agent 2 · **2026-11-15** · `head` = **`a3368319`** (commit nền: `QUEUE@head` đủ **125 hàng**, **124
`DONE`** + ST-111 `READY` — báo cáo này). Số đo chạy trên mã **`5ae46d04`** (ST-110; sau đó chỉ
`QUEUE.md` + báo cáo). Bản máy đọc: [`R19-viscript.json`](R19-viscript.json) — **đúng 11 khoá** Khối E
(`project = "viscript"`, `round = 19`). Như tiền lệ R11–R18: `head` = commit **nền** của báo cáo.

**Bàn giao**: [`../ROADMAP.md`](../ROADMAP.md) (55 dòng) — vẫn là gói bàn giao; R19 **không** đổi 8 mức
VS2.

## 1. Tóm tắt (5 dòng)

1. **ST-108 — R1 ≤1.200**: tách `lower.rs` **1482 → 993** + `lower/khai_bao.rs` **497** (khai báo item);
   **R1 max 1482 → 1060** (`cmd.rs`), checksum 1045 test + sha256 diff **trùng**.
2. **ST-109 — HTTP**: `Location` xử lý **`..`/`.`**, tương đối, **query-only**, **fragment/rỗng**; thêm
   `http.head` — **không đọc thân**, 2 lần **<1,2 s**, **không** theo redirect; tài liệu HTTP trong
   `HOST-BAO-DAM.md`.
3. **ST-110 — JSON theo tệp**: `check --json` có `files` = **tệp nạp thật** (chính trước, module theo
   `use`) + `severity` (`VS0107` warning, còn lại error); 5 ca; **nợ span file-id** ghi nhận.
4. **Số đo**: **1056 test**, sem **447/22** (interp **27**), `vis test` **447 ok** (`--vm` **447 ok + 0
   SKIP**), host **54/54 ×2**, parity **398/398 + 49**, diff **100k / 0 lệch** (166,3 s), cache **520×4 /
   0 lệch**, fuzz **2.224.775 ca / 0 vi phạm**, `FUNCS` **52**, targets **18/18**, fmt **497**.
5. **Trung thực**: vòng 19 vẫn **VS2 không đổi**; có **1 nợ correctness** mới nổi (span HIR thiếu file-id
   ⇒ chẩn đoán cấp tệp có thể lệch tệp) — phải làm **trước** khi tuyên bố “chẩn đoán đa tệp đúng hoàn
   toàn”; nợ **91 → 96**.

## 2. Bảng vòng 19

| Task | Commit | Kết quả (1 dòng, số đo tại lúc đóng) |
|---|---|---|
| ST-108 | `0cd6d471` | `lower.rs` **1482→993** + `khai_bao.rs` 497; **R1 max 1482→1060**; checksum 1045 test/diff trùng; **1045 test** |
| ST-109 | `28c441cb` | Location **`..`, query-only, fragment/rỗng**; `http.head` không thân **<1,2 s**; 5 ca + 1 unit; `FUNCS` 50→52; **1051 test** |
| ST-110 | `5ae46d04` | `check --json`: `files` theo tệp nạp + `severity`; `VS0107` warning; 5 ca; **1056 test**; nợ span file-id |
| ST-111 | (commit này) | **R19**; `report-check --all` PASS; số liệu chạy lại §3 |

## 3. Số liệu chạy lại (mã `5ae46d04`, đo 2026-11-15, macOS aarch64)

| Phép đo | Kết quả | Lệnh |
|---|---|---|
| `cargo test --workspace` | **1056** = 21 diag + 96 syntax + 27 fmt + 154 types + **27 interp** + 62 bytecode + 9 vm + **660 cli** | `cd viscript && cargo test --workspace` |
| Ca ngữ nghĩa | **447 / 22 nhóm** (397 `.vis` + 50 `.vish`; **83** `.err`) | `cargo test -p vis-cli --test sem` |
| `vis test` / `--vm` | **447 ok**, 0 `not ok` / **447 ok, 0 SKIP, 0 đỏ** | `vis test test/sem` / `vis test --vm test/sem` |
| Parity interp ↔ VM | **398/398** khớp byte + **49** lỗi biên dịch (447) | `cargo test -p vis-vm --test vs_interp` |
| `diag_cmp` ba tầng | **364 xanh / 49 biên dịch / 34 lúc chạy** | `cargo test -p vis-cli --test diag_cmp` |
| Differential | **`tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=166.3 toc_do=601/s seed=20261115`** | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| Fuzz parser 60 s | **2.224.775 ca, 50.617.245 token, 37.080 ca/s, 843.621 token/s, 60,0 s, vi phạm 0** | `target/release/vis-parse-fuzz --seconds 60 --seed 20261115` |
| Cache harness | **520 tệp × 4 lần**, **520 ca so đủ 4 đường, 0 lệch**, **24,56 s** | `cargo test -p vis-cli --test cache_khong_che -- --nocapture` |
| Host suite | **54/54 × 2 lần**, 0 lệch; VM 54/54 | `./bin/vis-host-suite` (hai lần) |
| Soak | **8/8 vòng OK, 818 s** — lần cuối ST-98, **không** chạy lại | `bin/vis-soak` |
| `FUNCS` / `VBC1` | **52** hàm chữ ký / **53** tag (không bump) | `vis-types/src/stdlib.rs` / `Instr::tag` |
| Sổ mã | **7 sổ / 32 mã** | `vis explain --list` |
| Allowlist doc-examples | **6** mục | `grep -v '^#' docs/viscript/doc-examples-allow.txt` |
| Ma trận target | **18/18 `PASS`** — chỉ `cargo check` | `./bin/vis-targets` |
| `fmt --check test/` | **exit 0**, **497 tệp** (**429** `.vis` + **68** `.vish`), **8 cảnh báo** thụt lề | `vis fmt --check test` |
| `--indent-report` | **8 tệp / 10 dòng lệch** — không sửa | `vis fmt --indent-report test` |
| R1 | **max 1482 → 1060**; `lower.rs` **993**, `khai_bao.rs` **497**, `cmd.rs` **1060**, `net.rs` **463** | `wc -l crates/vis-*` |
| Báo cáo | **19 báo cáo**, **125 hàng QUEUE** phủ đủ; `R19` 11 khoá, `round=19`, `head=a3368319` | `python3 viscript/test/report/check_report.py --all` |
| Cổng | `gate --fast` **PASS** (không retry) | `./vis-dev gate --fast` |

### 3b. Bằng chứng công cụ **mới** vòng 19

- **R1**: `lower.rs` **1482 → 993**, `khai_bao.rs` **497**; **R1 max 1482 (`lower.rs`) → 1060
  (`cmd.rs`)** ≤1.200; checksum 1045 test `5c194bb8…` + sha256 diff `c40b9a84…` **trước = sau**.
- **Location**: `/a/b/../c?x=1` ⇒ **`/a/c?x=1`**; `?y=2` ⇒ **giữ path + query mới**; `#f`/rỗng ⇒
  **dừng**, trả `HTTP 3xx`; tương đối + `..` chuẩn hoá.
- **HEAD**: `http.head` trả `{status, headers}`, server gửi `Content-Length: 999999` + thân giả, **2 lần
  liên tiếp <1,2 s**, không đọc thân; `HEAD` + 302 ⇒ **302 + Location**, không theo.
- **JSON tệp phụ**: `{"files":[{"file":"a.vis","diagnostics":[]},{"file":"b.vis","diagnostics":[{"code":
  "VS0412","line":2,"col":5,"severity":"error","message":"…"}]}]}`; tab `.vish` ⇒
  `{"code":"VS0107","severity":"warning"}`.
- **Nợ correctness**: span HIR mỗi tệp bắt đầu từ 0 ⇒ **câu lệnh cấp tệp chính** có thể bị gắn sang
  module; ví dụ top-level `let` + module lỗi vẫn lệch — **chưa** sửa.

## 4. §19 — đối chiếu **VS2** sau 19 vòng: mức thật (không đổi; thêm bằng chứng)

| # | Hạng mục VS2 | Mức thật | Bằng chứng mới R19 | Còn thiếu |
|---|---|---|---|---|
| 1 | **Async** | Một luồng run-to-completion; 10k **316/168 ms**; drain 100 | 100 task đúng thứ tự (ST-94) giữ | **Song song thật**, async method/cmd, GC |
| 2 | **Kênh** | `chan(n≥1)`; **`chan(0)` bị từ chối**; close/deadlock/drain | giữ | **Wait-set/rendezvous**, `select`/timeout |
| 3 | **Net/HTTP** | Loopback; **Location đầy đủ + HEAD** (ST-109); **không TLS/HTTP2/keep-alive**, redirect chỉ GET | `../c`, query-only, `#f`; HEAD **<1,2 s** không thân | **TLS**, HTTP2, keep-alive, cookie/auth, HEAD redirect |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; `--strict-caps`; Proc allowlist; **chưa mặc định**, **chưa Landlock** | giữ | **Landlock thật**, mặc định caps, sandbox con |
| 5 | **LSP** | diagnostics module-aware; completion; hover trường; cache **0,466 ms** | giữ (R17) | Hover method, đa tài liệu, generic |
| 6 | **Gói** | Registry local, lock SHA-256; `update`/`list --json` (ST-102) | giữ | Resolver/version, HTTP, ký số, GC |
| 7 | **Đa nền tảng** | **Chỉ check 18/18**; chưa runner Windows/Linux/wasm | giữ | **Runner thật** 4 nền |
| 8 | **Hợp chuẩn host** | **54/54 macOS ×2**, VM 54/54; cache 520×4 | host **54/54 ×2** | Windows/Linux/wasm, net/TLS trong suite |

**Bổ sung bằng chứng**: HTTP hiểu **Location/HEAD**, `check --json` **gắn tệp phụ + severity**, R1 max
**≤1.200** — nhưng **không** hạng mục nào lên mức; chẩn đoán đa tệp còn nợ **span file-id**.

## 5. Lỗ hổng còn mở (R18 **91** + mới **92..96** = **96 mục**)

**1..91** — giữ nguyên [`R18-viscript.md`](R18-viscript.md) §5. Năm mục mới vòng 19:

92. **`span` HIR chưa có file-id (correctness — ưu tiên)**: mỗi tệp hạ với span bắt đầu 0; tyck toàn cục
    gắn chẩn đoán theo khoảng span ⇒ **câu lệnh cấp tệp chính** có thể bị quy sang module (top-level `let`
    + module lỗi). Cần offset/`file_id` trước khi tuyên bố chẩn đoán đa tệp “đúng hoàn toàn”.
93. **`HEAD` không theo redirect**: trả 302 + `Location`, không đọc thân (đã chọn, ghi tài liệu); chưa
    `HEAD` keep-alive/HTTP2.
94. **`FUNCS` 52 cần review**: thêm `http.head`/`head_timeout` và `Ty::ResultHead` (`map[str, {error}]`);
    test đếm 50→52 đã sửa kèm lý do — cần chốt API trước VS2.
95. **`cmd.rs` 1060 vẫn max R1**: dưới 1.200 nhưng `ha_stage` **782** dòng chưa tách; tách tiếp nếu muốn
    <900.
96. **`check --json` nhiều đối số có thể lặp module**; `files` chưa có metadata `main`/`phu`; severity chỉ
    `error`/`warning`; TAP vẫn báo chẩn đoán **đầu**.

## 6. VD-115

**VD-115 — Vòng 19: R1 ≤1.200 + HTTP Location/HEAD + JSON theo tệp (ST-108..ST-111)**: (1) **R1** —
tách `lower.rs` 1482→993 + `khai_bao.rs` 497; max **1482→1060**; checksum test/diff trùng; (2) **HTTP** —
Location `..`/`.`/query-only/fragment/rỗng; `http.head` chỉ header, không thân, **<1,2 s**, không redirect,
tài liệu `HOST-BAO-DAM`; (3) **JSON** — `files` theo tệp nạp thật + `severity` từ `Code.severity`; (4)
**nợ correctness** span file-id phải làm trước. **Đính chính**: VS2 **không đổi**; đây là bằng chứng công
cụ, không nới mức.

## 7. Ghi chú máy kiểm + kết luận

- `python3 test/report/check_report.py --all`: **PASS** — **19 báo cáo**, **125 hàng QUEUE** phủ đủ;
  `R19-viscript.json` đúng **11 khoá**, `round=19`, `head=a3368319` (tồn tại + là tổ tiên HEAD).
- Cổng: `./vis-dev gate --fast` **PASS**, không retry; `fmt`/`clippy -D warnings`/R1..R4 OK; report-check
  trong cổng PASS; differential **0 lệch**; fuzz **0 vi phạm**; cache **0 lệch**.
- Không thêm `#[ignore]`, không nới ngưỡng, không sửa golden/allowlist; `VBC1` **53**; **32 mã**; không
  sửa `R1..R18`; không sửa `docs/`, `v2/`, `spec/`, QUEUE/DECISIONS.
- **Kết luận trung thực**: sau **19 vòng**, đo được **1056 test**, **447 sem** (interp **27**), `--vm` **0
  SKIP**, parity **398+49**, diff **0 lệch**, cache **520×4 0 lệch**, host **54/54 ×2**, soak **8/8**,
  fuzz **0 vi phạm**, R1 max **1060**; thêm **Location/HEAD**, **JSON theo tệp**. Nhưng **VS2 không đổi**
  và còn **nợ correctness `span file-id`** — **không** được tuyên bố “chẩn đoán đa tệp đúng hoàn toàn”
  trước khi sửa; nợ **96 mục** (1..96).
