<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R22-viscript — báo cáo vòng 22 (soak cuối 10 vòng, R1 1060, chẩn đoán đa tệp đúng), dòng ViScript

Agent 2 · **2026-11-15** · `head` = **`1be37e6c`** (commit nền: `QUEUE@head` đủ **134 hàng**, **133
`DONE`** + ST-118 `READY` — báo cáo này). Số đo chạy trên mã **`63d27ba4`** (ST-116b; sau đó chỉ
`QUEUE.md` + báo cáo). Bản máy đọc: [`R22-viscript.json`](R22-viscript.json) — **đúng 11 khoá** Khối E
(`project = "viscript"`, `round = 22`). Như tiền lệ R11–R21: `head` = commit **nền** của báo cáo.

**Bàn giao**: [`../ROADMAP.md`](../ROADMAP.md) **64 dòng** — trạng thái cuối 22 vòng; R22 **không** đổi
8 mức VS2, **không** tuyên bố đóng 100%.

## 1. Tóm tắt (5 dòng)

1. **ST-114a/b** — `Span.file` + `file_rel`; **chẩn đoán đa tệp đúng tệp/dòng** (JSON + TAP), **6 ca
   red-first**; nợ correctness ST-112 **đã đóng**.
2. **ST-116/116b** — `parse/mod.rs` 1452→**1069**, `main.rs` 1254→**591** (`cli/check.rs` 335,
   `cli/fmt.rs` 252, `cli/run.rs` 94); checksum/diff trùng; R1 max **1060**.
3. **ST-117** — LSP **4 ca đa tệp**: tài liệu đang mở đúng tệp, không lẫn module; hành vi đã đúng từ
   ST-114b, **không sửa** mã LSP.
4. **Soak cuối** — **10/10 vòng OK, 1297 s** (gate×3, diff×3, host×2, cache×2), load 2.04→1.83.
5. **Số đo cuối**: **1066 test**, sem **447/22**, `--vm` **0 SKIP**, parity **398+49**, diff **0 lệch**,
   cache **520×4**, fuzz **2.506.504 ca / 0 vi phạm**, R1 max **1060**, `FUNCS` **52**, `VBC1` **53**.

## 2. Bảng vòng 21–22

| Task | Commit | Kết quả (1 dòng, số đo tại lúc đóng) |
|---|---|---|
| ST-114a | `bf11a21f` | `Span.file` + `lex_file`/`parse_file`/`LowerOptions.file`; **0** lời gọi `Span::new` cũ đổi; 1056 test y hệt; checksum/diff trùng |
| ST-114b | `614d55e0` | map `file_rel[d.span.file]`; 6 ca red-first→xanh; JSON/TAP đúng tệp+dòng; **1062 test**; đóng nợ ST-112 |
| ST-115 | `6ca45ef0` | **R21**; báo cáo 130 hàng QUEUE, 1062 test; `report-check` PASS |
| ST-116 | `167d7cc4` | `parse/mod.rs` 1452→**1069**, `parse/khai_bao.rs` 391; thuần refactor, checksum/diff trùng; 1062 test |
| ST-116b | `e34a8984` + `63d27ba4` | `main.rs` 1254→**591**, `cli/check.rs` 335, `cli/fmt.rs` 252, `cli/run.rs` 94; 1066 test; R1 max **1060**, ≤1000 toàn cục **chưa** |
| ST-117 | `5183b81a` | LSP 4 ca đa tệp; hành vi đã đúng từ ST-114b, chỉ thêm ca; 1066 test |
| ST-118 | (commit này) | **soak 10/10 (1297 s)** + **R22** + ROADMAP chốt; `report-check --all` PASS |

## 3. Số liệu chạy lại (mã `63d27ba4`, đo 2026-11-15, macOS aarch64)

| Phép đo | Kết quả | Lệnh |
|---|---|---|
| `cargo test --workspace` | **1066** = 21 diag + 96 syntax + 27 fmt + 154 types + **27 interp** + 62 bytecode + 9 vm + **670 cli** | `cd viscript && cargo test --workspace` |
| Ca ngữ nghĩa | **447 / 22 nhóm** (397 `.vis` + 50 `.vish`; **83** `.err`) | `cargo test -p vis-cli --test sem` |
| `vis test` / `--vm` | **447 ok** / **447 ok, 0 SKIP, 0 đỏ** | `vis test test/sem` / `vis test --vm test/sem` |
| Parity interp ↔ VM | **398/398** khớp byte + **49** lỗi biên dịch | `cargo test -p vis-vm --test vs_interp` |
| Differential | **`DIFF tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=155 toc_do=645/s seed=20261115`** (3 lần soak: 156/155/155 s) | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| Fuzz parser 60 s | **2.506.504 ca, 57.029.776 token, 41.775 ca/s, 950.496 token/s, 60,0 s, vi phạm 0** | `target/release/vis-parse-fuzz --seconds 60 --seed 20261115` |
| Cache harness | **520 tệp × 4 lần**, **0 lệch**, **25 s / 23 s** (soak) | `cargo test -p vis-cli --test cache_khong_che` |
| Host suite | **54/54 ×2**, 0 lệch; VM 54/54 | `./bin/vis-host-suite` |
| **Soak cuối** | **10/10 vòng OK, 1297 s**; load trước/sau **2.04 / 1.83**; bảng §3b | `bin/vis-soak` |
| `FUNCS` / `VBC1` / `all_codes` | **52** / **53** (không đổi) / **32** | `vis-types/src/stdlib.rs`, `Instr::tag`, `vis explain --list` |
| Allowlist doc-examples | **6** | `grep -v '^#' docs/viscript/doc-examples-allow.txt` |
| Ma trận target | **18/18 `PASS`** — chỉ `cargo check` | `./bin/vis-targets` |
| `fmt --check test/` | **exit 0**, **497 tệp** (**429** `.vis` + **68** `.vish`), **8 cảnh báo** thụt lề | `vis fmt --check test` |
| R1 | **max 1060** (`lower/cmd.rs`); `parse/cmd.rs` **1007**, `parse/mod.rs` **1069**, `main.rs` **591** | `wc -l crates/vis-*` |
| Báo cáo | **22 báo cáo**, **134 hàng QUEUE** phủ đủ; `R22` 11 khoá, `round=22`, `head=1be37e6c` | `python3 test/report/check_report.py --all` |
| Cổng | `gate --fast` **PASS** (3 lần trong soak, không retry) | `./vis-dev gate --fast` |

### 3b. Soak 10 vòng cuối — [`st118-soak.log`](../JOURNAL/st118-soak.log)

| # | Vòng | Giây | Exit | Kết luận |
|---|---|---:|---:|---|
| 1 | gate-1 | 256 | 0 | OK |
| 2 | gate-2 | 253 | 0 | OK |
| 3 | diff-1 | 156 | 0 | OK, 0 lệch |
| 4 | diff-2 | 155 | 0 | OK, 0 lệch |
| 5 | gate-3 | 258 | 0 | OK |
| 6 | diff-3 | 155 | 0 | OK, 0 lệch |
| 7 | host-1 | 8 | 0 | OK 54/54 |
| 8 | host-2 | 8 | 0 | OK 54/54 |
| 9 | cache-1 | 25 | 0 | OK 520×4 |
| 10 | cache-2 | 23 | 0 | OK 520×4 |
| — | **Tổng** | **1297** | 0 | **10/10 OK**; load 2.04→1.83 |

- **Không retry**; log `st118-soak.log` (attempt1 không có vì lần đầu xanh).
- `bin/vis-soak`: **10 vòng** (gate×3, diff×3, host×2, cache×2), đúng danh sách/ngưỡng ST-113.

## 4. §19 — đối chiếu **VS2** sau 22 vòng (8 hạng mục, mức thật **không đổi**)

| # | Hạng mục VS2 | Mức thật | Bằng chứng mới R22 | Còn thiếu |
|---|---|---|---|---|
| 1 | **Async** | Một luồng run-to-completion; 10k **316/168 ms**; drain 100 | giữ | **Song song thật**, async method/cmd, GC |
| 2 | **Kênh** | `chan(n≥1)`; **`chan(0)` bị từ chối**; close/deadlock/drain | giữ | **Wait-set/rendezvous**, `select`/timeout |
| 3 | **Net/HTTP** | Loopback; chunked/redirect/HEAD/timeout; **không TLS/HTTP2/keep-alive** | `net.rs` 463; 21 ca http | **TLS**, HTTP2, keep-alive, cookie, HEAD redirect |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; Proc allowlist; **chưa mặc định**, **chưa Landlock** | `quyen.rs` 460 | **Landlock thật**, mặc định caps, sandbox con |
| 5 | **LSP** | module-aware; **4 ca đa tệp**; hover trường; cache **0,466 ms** | `lsp_da_tep.rs` 4 ca | **Publish tệp phụ**, hover method, generic, incremental |
| 6 | **Gói** | Registry local, lock SHA-256; `update`/`list --json`/`remove` | `pkg.rs` 927 | Resolver/version, HTTP, ký số, GC |
| 7 | **Đa nền tảng** | **Chỉ check 18/18**; chưa runner | `vis-targets` | **Runner thật** 4 nền |
| 8 | **Hợp chuẩn host** | **54/54 macOS ×2**, VM 54/54; cache 520×4 | soak host×2 | Windows/Linux/wasm, net/TLS trong suite |

**Bổ sung bằng chứng**: **chẩn đoán đa tệp đúng tệp/dòng** (JSON/TAP, 6 ca red-first) — correctness
**đã đóng nợ ST-112**, không phải mức VS2 nào lên hạng; hạn chế span tổng hợp ghi §5.

## 5. Lỗ hổng còn mở (R21 **100** + mới **101..104** = **104 mục**)

**1..100** — giữ nguyên [`R21-viscript.md`](R21-viscript.md) §5. Bốn mục mới vòng 22:

101. **`lower/cmd.rs` 1060** — dưới luật R1 1500 nhưng trên mục tiêu nội bộ ≤1000; `parse/cmd.rs` **1007**
     cũng vậy; cần task tách riêng nếu muốn ≤1000 toàn cục.
102. **`main.rs` 591** nhưng `run_run` vẫn 352 dòng trong main (giữ `cache::khoa_multi` cho ca
     `cache_fastpath`); muốn đưa sang `cli/run.rs` phải cập nhật ca kiểm tra vị trí.
103. **Span tổng hợp `file=0`** — node do lower sinh vẫn mặc định tệp chính; chưa audit hết.
104. **LSP chưa publish chẩn đoán tệp phụ** cho tài liệu chính; mới bảo đảm tài liệu đang mở đúng tệp.

## 6. VD-118

**VD-118 — Vòng 22: soak cuối + R22 + ROADMAP chốt (ST-116..ST-118)**: (1) **soak** — 10 vòng tuần tự
(gate×3, diff×3, host×2, cache×2), **10/10 OK, 1297 s**, load 2.04→1.83, **không retry**; (2) **R1** —
`parse/mod.rs` **1069**, `main.rs` **591**, max **1060** (`lower/cmd.rs`) ≤ luật 1500; mục tiêu nội bộ
≤1000 **chưa đạt** (ngoài phạm vi 2 task tách); (3) **ROADMAP** 64 dòng, 8 mức thật + 9 điều kiện đóng
100%; (4) **R22** 134 hàng QUEUE, 11 khoá. **Đính chính**: VS2 **không đổi**; không tuyên bố đóng 100%.

## 7. Ghi chú máy kiểm

- `python3 test/report/check_report.py --all`: **PASS** — **22 báo cáo**, **134 hàng QUEUE** phủ đủ;
  `R22-viscript.json` đúng **11 khoá**, `round=22`, `head=1be37e6c` (tồn tại + là tổ tiên HEAD).
- Cổng: `./vis-dev gate --fast` **PASS** 3 lần trong soak; `fmt`/`clippy -D warnings`/R1..R4 OK;
  report-check trong cổng PASS; diff **0 lệch**; fuzz **0 vi phạm**; cache **0 lệch**.
- Không thêm `#[ignore]`, không nới ngưỡng, không sửa golden/allowlist; `VBC1` **53**; **32 mã**; không
  sửa `R1..R21`; không sửa `docs/`, `v2/`, `spec/`, QUEUE/DECISIONS.

## 8. Kết luận trung thực (3–5 dòng)

Sau **22 vòng**, đo được **1066 test**, sem **447/22**, `--vm` **0 SKIP**, parity **398+49**, diff **0
lệch**, cache **520×4**, host **54/54 ×2**, soak **10/10 (1297 s)**, fuzz **0 vi phạm**, R1 max **1060**,
`FUNCS` **52**, `all_codes` **32**, `VBC1` **53**. **Lần đầu tiên** chẩn đoán đa tệp đúng tệp/dòng (JSON
và TAP, 6 ca red-first) — nợ correctness ST-112 **đã đóng**, trừ span tổng hợp. **VS2 vẫn không đổi**:
Landlock/runner/TLS/async song song/rendezvous/mặc định caps **chưa**, nên **chưa** đóng 100%.

## 9. Bàn giao cuối — 22 vòng

**Trạng thái**: 22 vòng, **1066 test**, R1 max **1060**, soak **10/10 = 1297 s**, 8 hạng mục VS2 **không
đổi mức**. **Điều kiện đóng 100%** (giữ danh sách): (1) `Span` tổng hợp `file=0` + LSP publish tệp phụ;
(2) **Landlock thật** (crate `landlock` hoặc `unsafe` được duyệt) + **runner Linux**; (3) runner wasm
(wasmtime) + Windows, CI 4 nền; (4) **TLS/HTTP2/keep-alive** + HEAD redirect; (5) **async song song
thật** + wait-set/rendezvous `chan(0)`; (6) **siết mặc định caps** + sandbox con; (7) resolver/registry
HTTP/ký số + GC; (8) `lower/cmd.rs` **1060** / `parse/cmd.rs` **1007** (mục tiêu nội bộ ≤1000, **không**
phải luật R1); (9) **104 nợ** đóng hết, **R23** đo lại.

**Nơi đọc**: [`ROADMAP.md`](../ROADMAP.md) · [`QUEUE.md`](../QUEUE.md) · [`DECISIONS.md`](../DECISIONS.md)
(VD-01..VD-118) · [`HOST-BAO-DAM.md`](../HOST-BAO-DAM.md) · `reports/R1..R22` · [`STATE.md`](../STATE.md).

**Lệnh chạy**: `./vis-dev gate --fast` · `bin/vis-diff` · `bin/vis-host-suite` · `bin/vis-soak` ·
`bin/vis-targets`.
