<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R20-viscript — báo cáo vòng 20 (soak cuối 10 vòng, R1 ≤1.200, HTTP/JSON), dòng ViScript

Agent 2 · **2026-11-15** · `head` = **`263c7722`** (commit nền: `QUEUE@head` đủ **127 hàng**, **126
`DONE`** + ST-113 `READY` — báo cáo này). Số đo chạy trên mã **`7c3f5a49`** (ST-112; sau đó chỉ
`QUEUE.md` + báo cáo). Bản máy đọc: [`R20-viscript.json`](R20-viscript.json) — **đúng 11 khoá** Khối E
(`project = "viscript"`, `round = 20`). Như tiền lệ R11–R19: `head` = commit **nền** của báo cáo.

**Bàn giao**: [`../ROADMAP.md`](../ROADMAP.md) **79 dòng** — đã cập nhật trạng thái cuối 20 vòng; R20
**không** đổi 8 mức VS2, **không** tuyên bố đóng 100%.

## 1. Tóm tắt (5 dòng)

1. **ST-108 — R1 ≤1.200**: `lower.rs` **1482 → 993** + `khai_bao.rs` **497**; **R1 max 1482 → 1060**;
   checksum 1045 test + sha256 diff **trùng**.
2. **ST-109 — HTTP**: `Location` **`..`/`.`/query-only/fragment/rỗng**; `http.head` **không đọc thân**,
   **<1,2 s**, **không** redirect; tài liệu `HOST-BAO-DAM.md`.
3. **ST-110 — JSON**: `check --json` `files` theo **tệp nạp thật** + `severity` (`VS0107` warning).
   **ST-112**: nợ **`Span` file-id** tái hiện + kế hoạch (>150 dòng) — **không sửa**, ghi ROADMAP §C.
4. **Số đo cuối**: **1056 test**, sem **447/22** (interp **27**), `--vm` **0 SKIP**, parity **398+49**,
   host **54/54 ×2**, diff **100k / 0 lệch**, cache **520×4 / 0 lệch**, fuzz **2.540.549 ca / 0 vi phạm**,
   **soak 10/10 = 1255 s**, R1 **1060**, `FUNCS` **52**.
5. **Trung thực**: **VS2 vẫn chưa đóng** — còn **96 nợ** + nợ correctness `Span`; Landlock thật/runner 4
   nền/TLS/async song song/rendezvous/siết mặc định **chưa** có.

## 2. Bảng vòng 19–20

| Task | Commit | Kết quả (1 dòng, số đo tại lúc đóng) |
|---|---|---|
| ST-108 | `0cd6d471` | `lower.rs` **1482→993** + `khai_bao.rs` 497; **R1 max 1482→1060**; checksum 1045 test/diff trùng; **1045 test** |
| ST-109 | `28c441cb` | Location **`..`, query-only, fragment/rỗng**; `http.head` không thân **<1,2 s**; 5 ca + 1 unit; `FUNCS` 50→52; **1051 test** |
| ST-110 | `5ae46d04` | `check --json`: `files` theo tệp nạp + `severity`; `VS0107` warning; 5 ca; **1056 test**; nợ span file-id |
| ST-112 | `7c3f5a49` | tái hiện chẩn đoán đa tệp sai tệp; **KHÔNG sửa** (>150 dòng); kế hoạch `Span` file-id; ROADMAP §C |
| ST-113 | (commit này) | **soak 10/10 (1255 s)** + **R20** + ROADMAP; `report-check --all` PASS |

## 3. Số liệu chạy lại (mã `7c3f5a49`, đo 2026-11-15, macOS aarch64)

| Phép đo | Kết quả | Lệnh |
|---|---|---|
| `cargo test --workspace` | **1056** = 21 diag + 96 syntax + 27 fmt + 154 types + **27 interp** + 62 bytecode + 9 vm + **660 cli** | `cd viscript && cargo test --workspace` |
| Ca ngữ nghĩa | **447 / 22 nhóm** (397 `.vis` + 50 `.vish`; **83** `.err`) | `cargo test -p vis-cli --test sem` |
| `vis test` / `--vm` | **447 ok**, 0 `not ok` / **447 ok, 0 SKIP, 0 đỏ** | `vis test test/sem` / `vis test --vm test/sem` |
| Parity interp ↔ VM | **398/398** khớp byte + **49** lỗi biên dịch (447) | `cargo test -p vis-vm --test vs_interp` |
| `diag_cmp` ba tầng | **364 xanh / 49 biên dịch / 34 lúc chạy** | `cargo test -p vis-cli --test diag_cmp` |
| Differential | **`tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=154.0 toc_do=649/s seed=20261115`** (3 lần trong soak: 154/153/156 s, đều 0 lệch) | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| Fuzz parser 60 s | **2.540.549 ca, 57.800.097 token, 42.342 ca/s, 963.335 token/s, 60,0 s, vi phạm 0** | `target/release/vis-parse-fuzz --seconds 60 --seed 20261115` |
| Cache harness | **520 tệp × 4 lần**, **520 ca đủ 4 đường, 0 lệch**, **25 s ×2** | `cargo test -p vis-cli --test cache_khong_che` |
| Host suite | **54/54 ×2**, 0 lệch; VM 54/54 | `./bin/vis-host-suite` |
| **Soak cuối** | **10/10 vòng OK, 1255 s**, load trước 2.18 → sau 2.94; bảng §3b | `bin/vis-soak` |
| `FUNCS` / `VBC1` | **52** hàm chữ ký / **53** tag (không bump) | `vis-types/src/stdlib.rs` / `Instr::tag` |
| Sổ mã | **7 sổ / 32 mã** | `vis explain --list` |
| Allowlist doc-examples | **6** mục | `grep -v '^#' docs/viscript/doc-examples-allow.txt` |
| Ma trận target | **18/18 `PASS`** — chỉ `cargo check` | `./bin/vis-targets` |
| `fmt --check test/` | **exit 0**, **497 tệp** (**429** `.vis` + **68** `.vish`), **8 cảnh báo** thụt lề | `vis fmt --check test` |
| R1 | **max 1060**; `lower.rs` **993**, `khai_bao.rs` **497**, `cmd.rs` **1060**, `net.rs` **463** | `wc -l crates/vis-*` |
| Báo cáo | **20 báo cáo**, **127 hàng QUEUE** phủ đủ; `R20` 11 khoá, `round=20`, `head=263c7722` | `python3 viscript/test/report/check_report.py --all` |
| Cổng | `gate --fast` **PASS** (3 lần trong soak, không retry) | `./vis-dev gate --fast` |

### 3b. Soak 10 vòng (`JOURNAL/st113-soak.log`, attempt2) + bài học attempt1

| # | Vòng | Giây | Exit | Kết luận |
|---|---|---:|---:|---|
| 1 | gate-1 | 249 | 0 | OK |
| 2 | gate-2 | 239 | 0 | OK |
| 3 | diff-1 | 154 | 0 | OK, 0 lệch |
| 4 | diff-2 | 153 | 0 | OK, 0 lệch |
| 5 | gate-3 | 238 | 0 | OK |
| 6 | diff-3 | 156 | 0 | OK, 0 lệch |
| 7 | host-1 | 8 | 0 | OK 54/54 |
| 8 | host-2 | 8 | 0 | OK 54/54 |
| 9 | cache-1 | 25 | 0 | OK 520×4 |
| 10 | cache-2 | 25 | 0 | OK 520×4 |
| — | **Tổng** | **1255** | 0 | **10/10 OK**; load trước/sau 2.18/2.94 |

- **Attempt1** (lưu `JOURNAL/st113-soak-attempt1.log`): `gate-1` đỏ vì `soak_script` còn kỳ vọng **8**
  vòng, không phải lỗi môi trường/sản phẩm. Đã sửa test (8→10, kèm lý do ST-113) rồi chạy lại **một**
  lần attempt2; **không** T-INFRA, **không** retry để che lỗi.
- `bin/vis-soak`: **10 vòng** (gate×3, diff 100k×3, host×2, cache×2); log `st113-soak.log`.

## 4. §19 — đối chiếu **VS2** sau 20 vòng: mức thật (không đổi; thêm bằng chứng)

| # | Hạng mục VS2 | Mức thật | Bằng chứng mới R20 | Còn thiếu |
|---|---|---|---|---|
| 1 | **Async** | Một luồng run-to-completion; 10k **316/168 ms**; drain 100 | giữ | **Song song thật**, async method/cmd, GC |
| 2 | **Kênh** | `chan(n≥1)`; **`chan(0)` bị từ chối**; close/deadlock/drain | giữ | **Wait-set/rendezvous**, `select`/timeout |
| 3 | **Net/HTTP** | Loopback; **Location + HEAD**; chunked/redirect GET ≤5; timeout 2 s; **không TLS/HTTP2/keep-alive** | 21 ca http; `net.rs` 463; HEAD **<1,2 s** | **TLS**, HTTP2, keep-alive, cookie, HEAD redirect |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; Proc allowlist; **chưa mặc định**, **chưa Landlock** | giữ | **Landlock thật**, mặc định caps, sandbox con |
| 5 | **LSP** | module-aware; hover trường; cache **0,466 ms** | giữ (R17) | Hover method, đa tài liệu, generic |
| 6 | **Gói** | Registry local, lock SHA-256; `update`/`list --json` | giữ | Resolver/version, HTTP, ký số, GC |
| 7 | **Đa nền tảng** | **Chỉ check 18/18**; chưa runner | giữ | **Runner thật** 4 nền |
| 8 | **Hợp chuẩn host** | **54/54 macOS ×2**, VM 54/54; cache 520×4 | host **54/54 ×2** trong soak | Windows/Linux/wasm, net/TLS trong suite |

**Bổ sung bằng chứng**: R1 max **1060**, HTTP hiểu **Location/HEAD**, JSON **tệp phụ + severity**, `test
--timeout`, soak **10/10** — nhưng **không** hạng mục nào lên mức.

## 5. Lỗ hổng còn mở (R19 **96**; R20 không thêm nợ mới)

**1..96** — giữ nguyên [`R19-viscript.md`](R19-viscript.md) §5, gồm **92 `Span` file-id** (correctness),
93 HEAD không redirect, 94 `FUNCS` 52 cần review, 95 `cmd.rs` 1060 max, 96 JSON nhiều đối số.
ST-112 **không sửa** nợ 92; kế hoạch >150 dòng tại `JOURNAL/st112-ke-hoach.md`.

## 6. VD-116

**VD-116 — Vòng 20: soak cuối 10 vòng + R20 + ROADMAP (ST-112..ST-113)**: (1) **soak** — 10 vòng tuần
tự (gate×3, diff 100k×3, host×2, cache×2), **10/10 OK, 1255 s**, load trước/sau; attempt1 đỏ do test
`soak_script` còn 8 vòng, sửa test rồi chạy lại một lần; (2) **ROADMAP** — 79 dòng, cập nhật R1 **1060**,
`FUNCS` **52**, test **1056**, soak 10, HTTP Location/HEAD, JSON tệp phụ/severity, `test --timeout`,
§C nợ correctness `Span`, §D 8 điều kiện đóng 100%; (3) **R20** — 127 hàng QUEUE, 11 khoá. **Đính chính**:
VS2 **không đổi**; không tuyên bố đóng 100%.

## 7. Ghi chú máy kiểm

- `python3 test/report/check_report.py --all`: **PASS** — **20 báo cáo**, **127 hàng QUEUE** phủ đủ;
  `R20-viscript.json` đúng **11 khoá**, `round=20`, `head=263c7722` (tồn tại + là tổ tiên HEAD).
- Cổng: `./vis-dev gate --fast` **PASS** 3 lần trong soak; `fmt`/`clippy -D warnings`/R1..R4 OK;
  report-check trong cổng PASS; diff **0 lệch**; fuzz **0 vi phạm**; cache **0 lệch**.
- Không thêm `#[ignore]`, không nới ngưỡng, không sửa golden/allowlist; `VBC1` **53**; **32 mã**; không
  sửa `R1..R19`; không sửa `docs/`, `v2/`, `spec/`, QUEUE/DECISIONS.
- **Sửa test `soak_script` 8→10** (kèm lý do ST-113) là cần thiết cho script mới; không nới bất kỳ
  ngưỡng chất lượng nào.

## 8. Kết luận trung thực

Sau **20 vòng**, phần đo được: **1056 test**, **447 sem** (interp **27**), `--vm` **0 SKIP**, parity
**398+49**, diff **0 lệch**, cache **520×4 0 lệch**, host **54/54 ×2**, soak **10/10 1255 s**, fuzz **0 vi
phạm**, R1 max **1060**, `FUNCS` **52**, `VBC1` **53**, **32 mã**. Nhưng **VS2 vẫn chưa đóng**: Landlock
thật, runner 4 nền, TLS, async song song, rendezvous, siết mặc định caps **chưa**; nợ **96** + **`Span`
file-id** (correctness) **chưa sửa** ⇒ **không** được nói “chẩn đoán đa tệp đúng hoàn toàn”.

## 9. Bàn giao cuối — điều kiện đóng **100%**

1. **`Span` file-id** + 5 ca chẩn đoán đa tệp (ST-112 plan) — correctness trước tiên.
2. **Landlock thật** Linux (crate/`unsafe` được duyệt) + **runner Linux** 54 host.
3. **Runner wasm32-wasip1** (wasmtime) + **Windows**; CI 4 nền.
4. **TLS/HTTP2/keep-alive** + HEAD redirect/cookie; test Internet thật.
5. **Async song song thật** + wait-set/rendezvous `chan(0)` (hoặc tuyên bố vĩnh viễn có văn bản).
6. **Siết mặc định caps** + sandbox tiến trình con; cập nhật host suite.
7. **Resolver/registry HTTP/ký số** cho gói + GC store.
8. **96 nợ** R19 + nợ mới đóng hết; **R21** xác nhận lại toàn bộ số đo.

**Hiện trạng**: **1056 test**, 20 vòng, soak **10/10**, R1 **1060** — nhưng **8 điều kiện trên chưa đạt**;
không có mục nào được coi là “xong” nếu chưa đo lại ở `R21`.
