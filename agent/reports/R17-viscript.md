<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# R17-viscript — báo cáo vòng 17 (LSP hover trường, `--json` + escape, `pkg update`), dòng ViScript

Agent 2 · **2026-11-14** · `head` = **`7b3aca76`** (commit nền: `QUEUE@head` đủ **117 hàng**, **116
`DONE`** + ST-103 `READY` — báo cáo này). Số đo chạy trên mã **`c20c90cf`** (commit cuối ST-102; sau đó
chỉ `QUEUE.md` + báo cáo). Bản máy đọc: [`R17-viscript.json`](R17-viscript.json) — **đúng 11 khoá**
Khối E (`project = "viscript"`, `round = 17`). Như tiền lệ R11–R16: `head` = commit **nền** của báo cáo.

**Bàn giao**: [`../ROADMAP.md`](../ROADMAP.md) (55 dòng) — vẫn là gói bàn giao; R17 chỉ **thêm bằng
chứng công cụ**, **không** đổi 8 mức VS2.

## 1. Tóm tắt (5 dòng)

1. **ST-100 — LSP**: hover **trường** (`p.x` ⇒ `**x**: int` + “trường của `P`”; `p.ten` ⇒ `str`), dùng
   **cùng** bảng `thanh_vien`; `didClose` dọn cache; cache **item module phụ** theo `(use, mtime,size)`,
   tệp chính luôn đọc từ buffer — **4,862 ms → 0,466 ms** (min 3 vòng, 5 tệp).
2. **ST-101/101b — `--json`**: `vis check --json` + `vis test --json` **một object nén/dòng**, khoá cố
   định; escape **một nguồn** (`"` `\` `\b` `\f` `\n` `\r` `\t` control, **UTF-8 thô**) chứng minh bằng
   `json.decode` + `python3 json.load`; TAP không hồi quy.
3. **ST-102 — `pkg update`**: đọc lại registry **cùng version**; nguồn không đổi ⇒ **lock byte-identical**
   + `không đổi`; nguồn đổi ⇒ cài lại + cập nhật `entry/sha256/files` (không resolver); `pkg list --json`
   sắp tên byte-order.
4. **Số đo**: **1033 test**, sem **447/22**, `vis test` **447 ok** (`--vm` **447 ok + 0 SKIP**), host
   **54/54 ×2**, parity **398/398 + 49**, differential **100k / 0 lệch** (171,4 s), cache **520×4 / 0
   lệch**, fuzz parser **2.247.596 ca / 0 vi phạm**, soak **8/8 818 s** (ST-98), targets **18/18**, fmt
   **497 tệp** (+8 cảnh báo).
5. **Trung thực**: 17 vòng, mức **VS2 không đổi** so R16 — chỉ thêm bằng chứng công cụ (hover/JSON/pkg);
   vẫn **chưa Landlock thật**, **chưa runner** Windows/Linux/wasm, net **không TLS**, async **một luồng**,
   `chan(0)` **bị từ chối**; nợ tăng **77 → 82**.

## 2. Bảng vòng 17

| Task | Commit | Kết quả (1 dòng, số đo tại lúc đóng) |
|---|---|---|
| ST-100 | `45334bfc` | **LSP hover trường** (+`didClose` cache, cache module phụ **0,466 ms**); 5 ca; **1015 test**; `lsp/*` ≤300 dòng |
| ST-101 | `e2f25945` | **`check --json` / `test --json`** hợp đồng nén một dòng; check đủ diagnostics **1-based**, test cases thứ tự + message ≤200; 6 ca; **1021 test** |
| ST-101b | `494788bc` | **Escape JSON** đúng chuẩn một nguồn; roundtrip `json.decode` + python; UTF-8 thô; 6 ca; **1027 test** |
| ST-102 | `c20c90cf` | **`pkg update`** idempotent (lock byte-identical), cập nhật khi nguồn đổi; **`pkg list --json`**; 6 ca; **1033 test** |
| ST-103 | (commit này) | **R17**; `report-check --all` PASS; số liệu chạy lại §3 |

## 3. Số liệu chạy lại (mã `c20c90cf`, đo 2026-11-14, macOS aarch64)

| Phép đo | Kết quả | Lệnh |
|---|---|---|
| `cargo test --workspace` | **1033** = 21 diag + 96 syntax + **27 fmt** + 154 types + 26 interp + 62 bytecode + 9 vm + **638 cli** | `cd viscript && cargo test --workspace` |
| Ca ngữ nghĩa | **447 / 22 nhóm** (397 `.vis` + 50 `.vish`; **83** `.err`) — không đổi | `cargo test -p vis-cli --test sem` |
| `vis test` / `--vm` | **447 ok**, 0 `not ok` / **447 ok, 0 SKIP, 0 đỏ** | `vis test test/sem` / `vis test --vm test/sem` |
| Parity interp ↔ VM | **398/398** khớp byte + **49** lỗi biên dịch (tổng 447) | `cargo test -p vis-vm --test vs_interp` |
| `diag_cmp` ba tầng | **364 xanh / 49 biên dịch / 34 lúc chạy** | `cargo test -p vis-cli --test diag_cmp` |
| Differential | **`tong=100000 loi_tinh=6230 chay=93770 lech=0 giay=171.4 toc_do=583/s seed=20261115`** | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` |
| Fuzz parser 60 s | **2.247.596 ca, 51.138.137 token, 37.460 ca/s, 852.302 token/s, 60,0 s, vi phạm 0** | `target/release/vis-parse-fuzz --seconds 60 --seed 20261115` |
| Cache harness | **520 tệp × 4 lần**, **520 ca so đủ 4 đường, 0 lệch**, **0 loại trừ**, **24,48 s** | `cargo test -p vis-cli --test cache_khong_che -- --nocapture` |
| Host suite | **54/54 × 2, 120 lần, 0 lệch; VM 54/54** | `./bin/vis-host-suite` |
| Soak | **8/8 vòng OK, 818 s** — **lần cuối ST-98**, không chạy lại trong task báo cáo | `bin/vis-soak` |
| `VBC1` | **53 tag** (không bump ở vòng 17) | `Instr::tag` (`vis-bytecode/src/format.rs`) |
| Sổ mã | **7 sổ / 32 mã** (không mã mới) | `vis explain --list` |
| Allowlist doc-examples | **6** mục | `grep -v '^#' docs/viscript/doc-examples-allow.txt` |
| Ma trận target | **18/18 `PASS`** — **chỉ `cargo check`**, chưa chạy | `./bin/vis-targets` |
| `fmt --check test/` | **exit 0**, **497 tệp** (**429** `.vis` + **68** `.vish`), 136 input bố trí bỏ; **8 cảnh báo** thụt lề `.vish` (không đỏ) | `vis fmt --check test` |
| `--indent-report` | **8 tệp / 10 dòng lệch** — không sửa | `vis fmt --indent-report test` |
| Báo cáo | **17 báo cáo**, **117 hàng QUEUE** phủ đủ; `R17` 11 khoá, `round=17`, `head=7b3aca76` | `python3 viscript/test/report/check_report.py --all` |
| Cổng | `gate --fast` **PASS** (không retry) | `./vis-dev gate --fast` |

### 3b. Bằng chứng công cụ **mới** vòng 17

- **`vis check --json`** (một dòng; `line/col` **1-based**):
  `{"ok":true,"files":[{"file":"a.vis","diagnostics":[]}]}`
- **`vis test --json`**:
  `{"total":3,"ok":2,"failed":1,"cases":[{"name":"a","ok":true,"message":""},{"name":"b","ok":false,"message":"VS0501 assert thất bại"},{"name":"c","ok":true,"message":""}]}`
- **Escape**: `"`→`\"`, `\`→`\\`, `\b/\f/\n/\r/\t`, `<0x20`→`\u00XX`, **UTF-8 thô**; roundtrip
  `a"b\c` + newline/tab/CR/`0x01` qua `json.decode`; tên tệp `"`/`\` và tên ca `kiểm_tra` qua
  `python3 json.load`.
- **Hover trường**: `p.x` ⇒ `**x**: int` + “trường của `P`”; `p.ten` ⇒ `str`; `didClose` → mở lại
  chẩn đoán **VS0412 vẫn đúng**; 5 `didChange` ⇒ hover **byte y hệt** lần đầu.
- **Cache LSP**: 5 tệp phụ, **min 3 vòng = 0,466 ms** (< 2 ms; trước 4,862 ms).
- **`pkg update`**: `không đổi a 1.0.0` (lock byte-identical) / `đã cập nhật a 1.0.0` (cùng version,
  sha/files đổi); lần 2 idempotent. **`pkg list --json`**: `{"packages":[{"name",...}]}` tên byte-order.

## 4. §19 — đối chiếu **VS2** sau 17 vòng: mức thật (không đổi; thêm bằng chứng)

| # | Hạng mục VS2 | Mức thật | Bằng chứng mới R17 | Còn thiếu |
|---|---|---|---|---|
| 1 | **Async** | Một luồng run-to-completion, không song song/GC; 10k **316/168 ms**; drain 100 task | 100 task đúng thứ tự (ST-94) giữ | **Song song thật**, async method/cmd, GC |
| 2 | **Kênh** | `chan(n≥1)`; **`chan(0)` bị từ chối** (`VS0505`, không rendezvous); close/deadlock/drain đủ | 4 ca cap0 + close/drain (ST-94) giữ | **Wait-set/rendezvous**, `select`/timeout |
| 3 | **Net/HTTP** | Loopback thật, `http.get`; **không TLS/redirect/keep-alive/chunked**, một luồng | không đổi | **TLS**, redirect/chunked, đồng thời |
| 4 | **Quyền** | `cap[Dir]/File/Net/Proc`; `--strict-caps`; `--grant` Proc **allowlist** (basename/canonical); **chưa mặc định**, **chưa Landlock**, con không sandbox | Proc allowlist (ST-97) giữ | **Landlock thật**, mặc định caps, sandbox con |
| 5 | **LSP** | diagnostics module-aware; completion sau `.`; **hover trường + didClose cache** (ST-100); **chưa** hover method/đa tài liệu | **hover `p.x`/`p.ten`**, cache **0,466 ms**, 43+1 ca | Hover method, đa tài liệu, generic, `std.str` |
| 6 | **Gói** | Registry local, lock SHA-256, entry/remove; **`update` idempotent + `list --json`** (ST-102); **chưa** semver/HTTP/ký | **update lock byte-identical**, list JSON | **Resolver/version**, registry HTTP, ký số, GC |
| 7 | **Đa nền tảng** | **Chỉ check 18/18**; chưa runner Windows/Linux/wasm | không đổi | **Runner thật** 4 nền |
| 8 | **Hợp chuẩn host** | **54/54 macOS ×2**, VM 54/54; cache 520×4 | host 54/54 ×2 | Windows/Linux/wasm, net/TLS trong suite |

## 5. Lỗ hổng còn mở (R16 **77** + mới **78..82** = **82 mục**)

**1..77** — giữ nguyên [`R16-viscript.md`](R16-viscript.md) §5 (R14 67 + R15 68..74 + R16 75..77).
Năm mục mới vòng 17:

78. **JSON thiếu `severity`** (ST-101): `check --json` chỉ `code/line/col/message`; chưa phân biệt
    warning/error; chưa có `source`/`range` UTF-16 như LSP.
79. **Chẩn đoán tệp phụ dưới `file` chính** (ST-101): `files[].diagnostics` gộp theo **tệp đối số**, không
    có trường `phu`/`file` gốc của chẩn đoán; người đọc JSON không biết lỗi nằm ở module nào.
80. **Cắt message 200 ký tự** (ST-101): `test --json` cắt theo **ký tự** (không byte), thêm “…”;
    message dài mất phần đuôi, không có trường `truncated`/độ dài gốc.
81. **Cache LSP global + tệp `pkg` luôn nạp lại** (ST-100): cache `thread_local` toàn tiến trình; key dùng
    `use ./…` văn bản nên tệp dùng `pkg` **không** hưởng cache; `didClose` xoá **toàn bộ** cache; hover
    khai báo chỉ khi tên duy nhất.
82. **`pkg update` chưa đủ resolver/blob** (ST-102): **không** đổi version/không resolver; chỉ ghi gói
    đích, **không cập nhật phụ thuộc**; blob cũ **không GC**; registry **cục bộ** (`registry`/
    `VIS_REGISTRY`), chưa mạng/ký số; lock cũ thiếu `entry` ⇒ `""` trong JSON.

## 6. VD-113

**VD-113 — Vòng 17: công cụ máy đọc + LSP hover + pkg update (ST-100..ST-103)**: (1) **LSP** — hover
trường dùng chung bảng `thanh_vien` (`p.x`→`**x**: int` + “trường của `P`”); `didClose` dọn cache; cache
**item module phụ** theo `(use, mtime,size)`, tệp chính luôn từ buffer, **4,862 → 0,466 ms**; (2) **JSON**
— `check/test --json` một object nén/dòng, check đủ diagnostics **1-based**, test cases thứ tự/message
≤200; **escape một nguồn** (`"` `\` `\b` `\f` `\n` `\r` `\t` control, UTF-8 thô), roundtrip
`json.decode` + `python3 json.load`, TAP không hồi quy; (3) **pkg** — `update` đọc lại registry cùng
version: không đổi ⇒ **lock byte-identical**, đổi ⇒ cài lại + cập nhật entry/sha/files (**không**
resolver/version), `list --json` tên byte-order. **Đính chính**: mức VS2 **không đổi** so R16; đây là
bằng chứng công cụ, không phải nới mức.

## 7. Ghi chú máy kiểm + kết luận

- `python3 test/report/check_report.py --all`: **PASS** — **17 báo cáo**, **117 hàng QUEUE** phủ đủ;
  `R17-viscript.json` đúng **11 khoá**, `round=17`, `head=7b3aca76` (tồn tại + là tổ tiên HEAD).
- Cổng: `./vis-dev gate --fast` **PASS**, không retry; `fmt`/`clippy -D warnings`/R1..R4 OK; report-check
  trong cổng PASS; differential 100k **0 lệch**; fuzz parser 60 s **0 vi phạm**; cache **0 lệch**.
- Không thêm `#[ignore]`, không nới ngưỡng, không sửa golden/allowlist; `VBC1` giữ **53**; **32 mã**;
  không sửa `R1..R16`; không sửa `docs/`, `v2/`, `spec/`, QUEUE/DECISIONS.
- **Kết luận trung thực**: sau **17 vòng**, phần đo được là **1033 test**, **447 sem**, `--vm` **0 SKIP**,
  parity **398+49**, differential **0 lệch**, cache **520×4 0 lệch**, host **54/54 ×2**, soak **8/8**,
  fuzz **0 vi phạm**; thêm **JSON hợp lệ**, **hover trường**, **pkg update idempotent**. Nhưng **VS2
  không đổi**: Landlock thật, runner 4 nền, TLS, async song song, rendezvous, quyền mặc định vẫn mở —
  nợ **82 mục** (1..82) ghi rõ, không có “đã xong”.
