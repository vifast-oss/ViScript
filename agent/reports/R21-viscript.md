<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R21-viscript — báo cáo vòng 21 (file-id, chẩn đoán đa tệp đúng tệp/dòng), dòng ViScript

Agent 2 · **2026-11-15** · `head` = **`cfc68e8c`** (commit nền: `QUEUE@head` đủ **130 hàng**, **129
`DONE`** + ST-115 `READY` — báo cáo này). Số đo chạy trên mã **`614d55e0`** (ST-114b; sau đó chỉ
`QUEUE.md` + báo cáo). Bản máy đọc: [`R21-viscript.json`](R21-viscript.json) — **đúng 11 khoá** Khối E
(`project = "viscript"`, `round = 21`). Như tiền lệ R11–R20: `head` = commit **nền** của báo cáo.

**Bàn giao**: [`../ROADMAP.md`](../ROADMAP.md) **82 dòng** — §C **đã đóng** nợ `Span` file-id; R21
**không** đổi 8 mức VS2, **không** tuyên bố đóng 100%.

## 1. Tóm tắt (5 dòng)

1. **ST-114a — `Span.file`**: thêm `file: u16` (mặc định 0), `new_file`/`with_file`; `lex_file`,
   `parse_file`, `LowerOptions.file` luồn chỉ số tệp; **0** lời gọi `Span::new` cũ phải đổi.
2. **File-id**: `Nap.file_rel` cấp **chính 0, module theo `use`**; quan sát a `let` `file=0`, b `return`
   `file=1`; **1056 test y hệt**, checksum **`6c1ea8bf…` trước = sau**, diff sha **`62bdd699…` trùng**.
3. **ST-114b — trả đúng tệp**: bỏ heuristic `items` làm nguồn chính, dùng `d.span.file` +
   `file_rel`; fallback span **chỉ** khi file-id ngoài bảng + log. JSON/TAP đúng tệp + dòng/cột.
4. **6 ca red-first** (`chan_doan_da_tep.rs`): 5/6 **đỏ** trước, **6/6 xanh** sau; (a) a `2:14`, (b) a
   `3:18`, (c) b `5:5`, (d) a `2:14` + b `5:5`, (e) TAP, (f) b **≥100 dòng** + lỗi a vẫn a.
5. **Số đo**: **1062 test**, sem **447/22**, `--vm` **447 ok + 0 SKIP**, parity **398+49**, host **54/54
   ×2**, diff **100k / 0 lệch**, cache **520×4**, fuzz **2.503.726 ca / 0 vi phạm**, R1 max **1452**.

## 2. Bảng vòng 21

| Task | Commit | Kết quả (1 dòng, số đo tại lúc đóng) |
|---|---|---|
| ST-114a | `bf11a21f` | `Span.file` + `lex_file`/`parse_file`/`LowerOptions.file`; file-id chính 0/module; **0** `Span::new` cũ đổi; 1056 test y hệt; checksum/diff trùng |
| ST-114b | `614d55e0` | map `file_rel[d.span.file]` thay heuristic; 6 ca red-first→xanh; JSON/TAP đúng tệp+dòng; **1062 test**; đóng nợ ST-112 |
| ST-115 | (commit này) | **R21**; `report-check --all` PASS; số liệu chạy lại §3; ROADMAP §C đã đóng |

## 3. Số liệu chạy lại (mã `614d55e0`, đo 2026-11-15, macOS aarch64)

| Phép đo | Kết quả | Lệnh |
|---|---|---|
| `cargo test --workspace` | **1062** = 21 diag + 96 syntax + 27 fmt + 154 types + **27 interp** + 62 bytecode + 9 vm + **666 cli** | `cd viscript && cargo test --workspace` |
| Ca ngữ nghĩa | **447 / 22 nhóm** (397 `.vis` + 50 `.vish`; **83** `.err`) | `cargo test -p vis-cli --test sem` |
| `vis test` / `--vm` | **447 ok**, 0 `not ok` / **447 ok, 0 SKIP, 0 đỏ** | `vis test test/sem` / `vis test --vm test/sem` |
| Parity interp ↔ VM | **398/398** khớp byte + **49** lỗi biên dịch (447) | `cargo test -p vis-vm --test vs_interp` |
| Differential | **`tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=163.5 toc_do=612/s seed=20261115`** | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| Fuzz parser 60 s | **2.503.726 ca, 56.966.444 token, 41.729 ca/s, 949.441 token/s, 60,0 s, vi phạm 0** | `target/release/vis-parse-fuzz --seconds 60 --seed 20261115` |
| Cache harness | **520 tệp × 4 lần**, **520 ca đủ 4 đường, 0 lệch**, **24,62 s** | `cargo test -p vis-cli --test cache_khong_che` |
| Host suite | **54/54 × 2 lần**, 0 lệch; VM 54/54 | `./bin/vis-host-suite` (hai lần) |
| Soak | **10/10 vòng OK, 1255 s** — ST-113, **không** chạy lại | `bin/vis-soak` |
| `FUNCS` / `VBC1` | **52** hàm chữ ký / **53** tag (**không đổi**) | `vis-types/src/stdlib.rs` / `Instr::tag` |
| Sổ mã | **7 sổ / 32 mã** (`all_codes` **32**) | `vis explain --list` |
| Allowlist doc-examples | **6** mục | `grep -v '^#' docs/viscript/doc-examples-allow.txt` |
| Ma trận target | **18/18 `PASS`** — chỉ `cargo check` | `./bin/vis-targets` |
| `fmt --check test/` | **exit 0**, **497 tệp** (**429** `.vis` + **68** `.vish`), **8 cảnh báo** thụt lề | `vis fmt --check test` |
| R1 | **max 1452** (`parse/mod.rs`); `main.rs` **1254**, `cmd.rs` **1060**, `modules.rs` **866** | `wc -l crates/vis-*` |
| Báo cáo | **21 báo cáo**, **130 hàng QUEUE** phủ đủ; `R21` 11 khoá, `round=21`, `head=cfc68e8c` | `python3 test/report/check_report.py --all` |
| Cổng | `gate --fast` **PASS** (không retry) | `./vis-dev gate --fast` |

### 3b. Bằng chứng mới vòng 21

- **`Span.file`**: `Span { start, end, file: u16 }`; `new` giữ `file=0`; **0** lời gọi `Span::new` cũ bị
  sửa; chỉ thêm `new_file` (4 chỗ nội bộ). `Span::new_file` + `lex_file`/`parse_file` + `LowerOptions.file`.
- **File-id quan sát**: a top-level `let` span **21..24 `file=0`**; b `return` span **23..33 `file=1`**;
  debug tạm đã gỡ, log `st114a-truoc-sau.log`.
- **Bằng chứng hành vi không đổi 114a**: checksum 1056 dòng test **`6c1ea8bf…` trước = sau**; sha256 dòng
  `DIFF tong=100000…` **`62bdd699…` trùng**; sem/parity/host như §3.
- **4 kịch bản JSON trước/sau** (`st114b-truoc-json.log` / `st114b-sau-json.log`): (a) `b.vis 2:3` →
  **`a.vis 2:14`**; (b) `a.vis 3:18` giữ; (c) `b.vis 5:5` giữ; (d) cả hai dồn b → **`a 2:14` + `b 5:5`**.
- **TAP**: (a) `not ok 1 - a.vis` + `# VS0412 2:14`, **không** `(b.vis:`; (d) TAP in `a.vis 2:14`.
- **6 ca red-first**: `st114b-red-test.log` **5/6 đỏ** với mã cũ; sau sửa **6/6 xanh**; ca (f) b dài
  ≥100 dòng vẫn trả a — đúng ca bắt lỗi heuristic cũ.
- **Nợ correctness ST-112 đã đóng**: map theo **file-id**, không còn heuristic làm nguồn chính.

## 4. §19 — đối chiếu **VS2** sau 21 vòng: mức thật (không đổi; thêm bằng chứng)

| # | Hạng mục VS2 | Mức thật | Bằng chứng mới R21 | Còn thiếu |
|---|---|---|---|---|
| 1 | **Async** | Một luồng run-to-completion; 10k **316/168 ms**; drain 100 | giữ | **Song song thật**, async method/cmd, GC |
| 2 | **Kênh** | `chan(n≥1)`; **`chan(0)` bị từ chối**; close/deadlock/drain | giữ | **Wait-set/rendezvous**, `select`/timeout |
| 3 | **Net/HTTP** | Loopback; **Location/HEAD**; chunked/redirect ≤5; timeout 2 s; **không TLS/HTTP2/keep-alive** | giữ | **TLS**, HTTP2, keep-alive, cookie, HEAD redirect |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; Proc allowlist; **chưa mặc định**, **chưa Landlock** | giữ | **Landlock thật**, mặc định caps, sandbox con |
| 5 | **LSP** | module-aware; hover trường; cache **0,466 ms** | giữ (R17) | Hover method, đa tài liệu, generic, publish tệp phụ |
| 6 | **Gói** | Registry local, lock SHA-256; `update`/`list --json` | giữ | Resolver/version, HTTP, ký số, GC |
| 7 | **Đa nền tảng** | **Chỉ check 18/18**; chưa runner | giữ | **Runner thật** 4 nền |
| 8 | **Hợp chuẩn host** | **54/54 macOS ×2**, VM 54/54; cache 520×4 | host **54/54 ×2** | Windows/Linux/wasm, net/TLS trong suite |

**Bổ sung bằng chứng**: **chẩn đoán đa tệp nay đúng tệp + dòng/cột** (JSON và TAP), nợ **ST-112 đã
đóng** — đây là **correctness**, không phải mức VS2 nào lên hạng; hạn chế span tổng hợp ghi §5.

## 5. Lỗ hổng còn mở (R20 **96** + mới **97..100** = **100 mục**)

**1..96** — giữ nguyên [`R20-viscript.md`](R20-viscript.md) §5. Bốn mục mới vòng 21:

97. **Span tổng hợp `file=0`** — vài node do lower sinh dùng `Span::new` mặc định; nếu chẩn đoán tyck rơi
    đúng node tổng hợp của module, nó vẫn có thể về tệp chính. Chưa thấy ca thực tế; cần audit.
98. **LSP chưa publish chẩn đoán tệp phụ theo file-id** — hành vi LSP không đổi; mới có module-aware.
99. **Bytecode `Span` không mang file** — `VBC1` giữ **53**; nếu cần debug đa tệp trong VM phải bump format.
100. **`parse/mod.rs` 1452 dòng** gần trần R1 **1500**; thêm chức năng parse đa tệp dễ vượt — cần tách
     trước khi mở rộng.

## 6. VD-117

**VD-117 — Vòng 21: `Span.file` + chẩn đoán đa tệp đúng tệp/dòng (ST-114a..ST-115)**: (1) **Span** —
thêm `file: u16` mặc định 0; `new` cũ không đổi, `new_file`; **0** lời gọi cũ phải sửa; (2) **luồn** —
`lex_file`/`parse_file`/`LowerOptions.file`, `Nap.file_rel` chính 0/module theo `use`; (3) **map** —
tyck dùng `d.span.file`, fallback heuristic **chỉ** ngoài bảng + log; JSON/TAP đúng; (4) **ca** — 6 ca
red-first (5/6 đỏ → 6/6 xanh), gồm b dài ≥100 dòng. **Đính chính**: VS2 **không đổi**; nợ correctness
ST-112 **đã đóng**, trừ span tổng hợp.

## 7. Ghi chú máy kiểm

- `python3 test/report/check_report.py --all`: **PASS** — **21 báo cáo**, **130 hàng QUEUE** phủ đủ;
  `R21-viscript.json` đúng **11 khoá**, `round=21`, `head=cfc68e8c` (tồn tại + là tổ tiên HEAD).
- Cổng: `./vis-dev gate --fast` **PASS**, không retry; `fmt`/`clippy -D warnings`/R1..R4 OK;
  report-check trong cổng PASS; diff **0 lệch**; fuzz **0 vi phạm**; cache **0 lệch**.
- Không thêm `#[ignore]`, không nới ngưỡng, không sửa golden/allowlist; `VBC1` **53**; **32 mã**; không
  sửa `R1..R20`; không sửa `docs/`, `v2/`, `spec/`, QUEUE/DECISIONS.

## 8. Kết luận trung thực (3–5 dòng)

Sau **21 vòng**, đo được **1062 test**, sem **447/22**, `--vm` **0 SKIP**, parity **398+49**, diff **0
lệch**, cache **520×4**, host **54/54 ×2**, fuzz **0 vi phạm**, R1 max **1452**, `FUNCS` **52**, `VBC1`
**53**. **Lần đầu tiên** không còn nợ correctness **đã biết** trong chẩn đoán đa tệp: file-id + 6 ca
red-first xác nhận đúng tệp/dòng cho a/b/cả hai/TAP/b dài. **VS2 vẫn không đổi**; còn span tổng hợp
`file=0`, LSP tệp phụ, bytecode, R1 `parse/mod.rs` 1452 — **chưa** đóng 100%.
