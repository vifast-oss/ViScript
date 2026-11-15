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
