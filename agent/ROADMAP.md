<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ROADMAP bàn giao — dòng ViScript (sau R20 / vòng 20)

Tài liệu **một trang** cho người tiếp nhận: 8 hạng mục VS2, **mức thật đang có** (đo được), bằng chứng,
và **điều kiện đóng 100%**. Không mục nào dưới đây được coi là “xong” nếu chưa đạt cột 4.
Số đo lấy từ `reports/R19-viscript.md` + `R20-viscript.md` (đo lại 2026-11-15, macOS aarch64).

## A. Tám hạng mục VS2

| # | Hạng mục | Mức thật (đo được, **không** vượt mức) | Bằng chứng (task/hash/số) | Điều kiện để đóng 100% |
|---|---|---|---|---|
| 1 | **Async** | `async fn`/`await`/`spawn`/join cuối; scheduler **một luồng, run-to-completion**, **không song song thật**, **không GC**; 10k task interp **316 ms** / VM **168 ms**; drain 100 task đúng thứ tự | ST-60/61/94/98; `async_10k`; `R15`/`R16` §3; `3840d463` | **Song song thật**/work-stealing + suspend-resume; test tăng tốc ≥2 core; `async` method/`cmd`; GC/ownership đo được |
| 2 | **Kênh** | `chan[T](n≥1)` send/recv/close; **`chan(0)` bị từ chối** `VS0505` ngay khi tạo (**không** rendezvous); `recv` sau close `none`; `send` sau close `VS0504`; deadlock `VS0505` không treo; drain 100 task đúng thứ tự | ST-61/94; `kenh.rs` + `kenh_cap0.rs`; `fe6d459b`; soak 10/10 | **Wait-set/chuyển task treo** + rendezvous `chan(0)` đúng nghĩa (hoặc tuyên bố từ chối vĩnh viễn có văn bản); `select`/timeout; test hai đường |
| 3 | **Net/HTTP** | `NetHost` TCP **loopback** thật; **chunked** (+`;ext`, trailer **bỏ**), **redirect GET ≤5** (Location `..`/query-only/fragment/rỗng), `Content-Length` đúng N, **timeout đọc 2 s**, **`http.head`** không đọc thân/không redirect; **không TLS/HTTP2/keep-alive**; cap 8 MiB; 21 ca http | ST-64/70/71/104/109; `net.rs` **463**; `R20` §3 | **TLS** (rustls/webpki — cần duyệt crate), HTTP2, keep-alive, cookie/auth, HEAD redirect; ≥2 kết nối **đồng thời**; test Internet thật |
| 4 | **Quyền** | `cap[Dir]/File/Net/Proc`; `--strict-caps`; `--grant` Dir/File/Net = đường dẫn, **Proc = allowlist** (basename/canonical); `VS0604` exit 2; **chưa siết mặc định**; **chưa Landlock thật**; con **không sandbox** | ST-72b/81/93/97; `quyen.rs`; `cap_proc*`; `0397c5c6` | **Siết mặc định**; **Landlock** (crate/`unsafe` được duyệt) cưỡng chế kernel Linux; sandbox con; test Linux thật |
| 5 | **LSP** | `vis lsp --stdio`: diagnostics **module-aware**, UTF-16; completion sau `.`; **hover trường** + `didClose`; cache module phụ **0,466 ms**; **chưa** hover method/đa tài liệu | ST-91/100; `lsp_std*.rs` 43 ca; `R17` §3 | Hover method + đa tài liệu + generic/`list[P]`; bảng `std.str`; publish tệp phụ; incremental sync; test LSP thật |
| 6 | **Gói** | Registry **local**, manifest `files[]`, lock SHA-256, `.vis/store`, `use pkg.*`, `VS0701/0702`, lock `entry`, `pkg remove`; **`pkg update`** idempotent + **`list --json`**; **chưa** resolver/HTTP/ký | ST-64/66/69/80/90/102; `pkg*.rs`; `R18` §3 | **Semver ranges** + resolver; registry HTTP; chữ ký số; GC store; migration lock cũ |
| 7 | **Đa nền tảng** | **Chỉ `cargo check`**: ma trận **18/18 ô PASS**; **chưa runner** Windows/Linux/wasm | ST-58/73/86; `vis-targets` 18/18; `R20` §3 | **Runner thật**: wasm32-wasip1 + wasmtime; Linux (Landlock) + Windows; CI chạy 54 host trên ≥2 nền |
| 8 | **Hợp chuẩn host** | **54 ca chạy thật macOS**, **2 lần 0 lệch**; **VM 54/54**; cache **520×4 0 lệch**; net **không** nằm trong suite | ST-87/88/89/98; `HOST-BAO-DAM.md`; soak 10/10 | 54 ca × **4 nền thật**; net/TLS trong suite; CI lặp; đo hiệu năng từng nền |

## B. Cách chạy tiếp

### B1. Lệnh cổng + soak (tuần tự, **không** chạy song song — AGENTS §6)

```bash
cd viscript
cargo test --workspace                 # 1056 test
cargo test -p vis-cli --test sem       # 447 ca / 22 nhóm
target/debug/vis test test/sem         # 447 ok (interp)
target/debug/vis test --vm test/sem    # 447 ok + 0 SKIP (VM)
../vis-dev gate --fast                 # cổng: fmt, clippy, R1..R4, L0, report-check
bin/vis-soak                           # soak **10 vòng** tuần tự (~21 phút; 1255 s lần cuối)
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff   # 0 lệch
target/release/vis-parse-fuzz --seconds 60 --seed 20261115 # 0 vi phạm
./bin/vis-targets                      # 18/18 check (CHỈ check)
```

### B2. Năm việc nên làm trước (theo thứ tự đòn bẩy)

1. **`Span` file-id** (nợ correctness): xem §C — làm **trước** mọi tuyên bố chẩn đoán đa tệp đúng.
2. **Landlock thật + runner Linux**: cần duyệt crate `landlock` **hoặc** `unsafe` (D2-17); nối
   `vis-host-linux`; chạy 54 ca Linux.
3. **Runner wasm** (wasmtime) + **Windows**; chạy `vis test` + host suite thật.
4. **TLS/HTTP2/keep-alive** (duyệt crate), HEAD redirect, cookie/auth; test Internet thật.
5. **Siết mặc định caps** + sandbox con; **async song song**/wait-set kênh/rendezvous — quyết định dứt
   điểm nếu không làm.

### B3. Nơi đọc

- [`QUEUE.md`](QUEUE.md) — **127 hàng**, trạng thái task.
- [`DECISIONS.md`](DECISIONS.md) — **VD-01..VD-116**.
- `reports/R1..R20` — báo cáo từng vòng (bắt đầu `R19`/`R20`).
- [`HOST-BAO-DAM.md`](HOST-BAO-DAM.md) — hợp chuẩn host 4 nền; HTTP client (ST-104/109).
- [`STATE.md`](STATE.md) — trạng thái hiện tại + việc dở.

## C. Nợ correctness ưu tiên (ST-112)

- **`Span` thiếu file-id** — mỗi tệp lex/parse bắt đầu `start = 0`; gộp HIR làm span chồng lấn ⇒ chẩn
  đoán **cấp tệp** của tệp chính có thể bị gắn sang module (ST-110 chỉ sửa được lỗi trong item/`fn main`).
  Tái hiện + vị trí map: `agent/JOURNAL/st112-chan-doan.log`.
- **Kế hoạch**: `agent/JOURNAL/st112-ke-hoach.md` — thêm `file: u16` vào `Span`; ước lượng **>150 dòng**
  (90 `Span::new`, 13 chỗ cắt text, `modules.rs` map, `interp::pos`, codegen, LSP) ⇒ **chưa sửa**.
- **Không** tuyên bố “chẩn đoán đa tệp đúng hoàn toàn” cho tới khi đóng; ca kiểm: top-level a, fn a,
  module b, cả hai, TAP `a.vis`/`b.vis` + sem/parity/host/diff.

## D. Điều kiện đóng **100%** (bàn giao cuối R20)

1. **`Span` file-id** + 5 ca chẩn đoán đa tệp (ST-112 plan) — correctness.
2. **Landlock thật** trên Linux (crate/`unsafe` được duyệt) + **runner Linux** chạy 54 host.
3. **Runner wasm32-wasip1** (wasmtime) + **Windows**; CI 4 nền.
4. **TLS/HTTP2/keep-alive** + HEAD redirect/cookie; test Internet thật.
5. **Async song song thật** + wait-set/rendezvous `chan(0)` (hoặc tuyên bố vĩnh viễn có văn bản).
6. **Siết mặc định caps** + sandbox tiến trình con; cập nhật host suite.
7. **Resolver/registry HTTP/ký số** cho gói + GC store.
8. **Nợ 96 mục** R19 + nợ mới đóng hết; `R21` xác nhận lại toàn bộ số đo.

**Hiện trạng (R20)**: **1056 test**, sem **447/22**, `--vm` **0 SKIP**, parity **398+49**, diff **0 lệch**,
cache **520×4 0 lệch**, host **54/54 ×2**, soak **10/10 (1255 s)**, fuzz **0 vi phạm**, R1 max **1060**,
`FUNCS` **52**, `VBC1` **53**, **32 mã**, targets **18/18 check**. **VS2 vẫn chưa đóng** — 8 điều kiện trên.
