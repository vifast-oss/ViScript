# ST-126 · Đóng băng API + cam kết tương thích (Q14/Q15, VD-129)

Agent 2 dòng ViScript · **2026-11-16** · task **ST-126** · ngân sách 2,5 h · nguồn sự thật:
[`quyet-dinh-a0-2026-11-16.md`](quyet-dinh-a0-2026-11-16.md) §2 Q14/Q15 + §6 ST-126,
[`DECISIONS.md`](../DECISIONS.md) **VD-129**, hàng ST-126 trong [`QUEUE.md`](../QUEUE.md).

## 1. Mục tiêu (nguyên văn + cách đo)

Ba danh sách là **cam kết tương thích**, sinh **từ code**, mỗi mục **một dòng**, sắp xếp **ổn định**:

| tệp | nguồn | một dòng | số mục **đo được** |
|---|---|---|---|
| `viscript/agent/API-STD.txt` | `vis_types::stdlib::FUNCS` | `tên<TAB>(tham) -> trả` | **52** |
| `viscript/agent/API-CODES.txt` | `vis_diag::all_codes()` | `mã<TAB>mức: câu ngắn (vi)` | **32** |
| `viscript/agent/API-VBC1.txt` | `vis_bytecode::format::TEN_THE` + `Instr::tag()` | `số thẻ 2 chữ số<TAB>tên` | **53** |

Test chặn: `crates/vis-cli/tests/api_dong_bang.rs` (đọc được **cả ba** nguồn). Đỏ khi code thêm/xoá/
đổi tên/đổi nghĩa một mục mà tệp không đổi, và ngược lại; thông điệp **nêu đích danh** mục và nhắc
`VD-129`.

**Phần khó — khoá cache bytecode:** phải chứng minh bằng **ca kiểm** rằng **đổi bảng thẻ `VBC1` ⇒
bytecode cũ không bị dùng nhầm**, và trả lời **đo, không đoán** câu "khoá cache có đổi khi bảng thẻ
đổi hay không".

## 2. Việc đã làm

1. `crates/vis-bytecode/src/format.rs`: thêm `TEN_THE: [&str; 53]` (tên từng thẻ theo **số thẻ**),
   `SO_THE`, `ten_the()`, `bang_the_van_ban_voi()`/`bang_the_van_ban()`,
   `bang_the_hash_voi()`/`bang_the_hash()` (SHA-256 của bảng thẻ). Đây là **nguồn chuẩn** cho
   `API-VBC1.txt` **và** cho khoá cache.
2. `crates/vis-cli/src/cache.rs`: khoá cache phủ thêm **hash bảng thẻ** (`khoa_multi_voi(files, mode,
   the)`; `khoa_multi` truyền `TEN_THE` thật), nhãn khoá `vifast-vis-cache-v2` → `-v3`.
   Sửa luôn dòng doc sai ("FNV-1a 64 bit × 4") — code đã dùng **SHA-256** từ ST-30.
3. Ca kiểm khoá cache (unit test trong `cache.rs`, xem §4 vì sao **không** đặt ở `tests/`).
4. Ba tệp cam kết + ba sidecar `.license` (luật **R3** của `bin/vis-dev`: `.txt` phải có sidecar).

## 3. Red-first (log đỏ thật + sha256)

| # | Ca | Log | sha256 | Nội dung đỏ |
|---|---|---|---|---|
| 1 | Ba tệp cam kết **chưa tồn tại** | [`st126-red-api.log`](st126-red-api.log) | `e25c99b6242c5b50b6b08c81ad6362e657d3d0a7c867d839b2a19e9cd642aa73` | 3 test `FAILED`, mỗi test panic `THIẾU tệp cam kết 'viscript/agent/API-…'` (đúng ca "chưa khai") |
| 2 | **Chứng minh test chặn**: sửa tay 3 tệp (xoá 1 mục, thêm 1 mục, đổi 1 chữ ký, đổi tên 1 thẻ) | [`st126-do-red-khai.log`](st126-do-red-khai.log) | `8ef8c7572ac310e01f963958e29bb8165f5e0c483794e34f31ae005d53fc0f2b` | bắt **đúng**: `THÊM (1): [str.split]…`, `XOÁ (1): [moi.them]…`, `ĐỔI (1): [51] "SwapMoi" → "Swap"`, `ĐỔI (1): [fs.exists] "(str, int) -> bool" → "(str) -> bool"`, `THÊM (1): [VS0701]…`; thông điệp nhắc `VD-129` (3 lần) |
| 3 | **Đo khoá cache TRƯỚC khi sửa** (ghim hex, cố ý sai) | [`st126-red-khoa.log`](st126-red-khoa.log) | `05d0bdcaa2b41760178789a9ebe20d6f29f2d8132ff6617da7221331a6744c41` | `left: "799713b8bd33729d75c798547c3c5038782f99b48fd55b5e5849268b043b7cd5"` |
| 4 | **Đo lại SAU khi thêm bảng thẻ vào khoá** (ghim vẫn sai) | [`st126-red-khoa-sau.log`](st126-red-khoa-sau.log) | `b8ac9b6ec445023b15381b569e1b995c55827281e427ac548cb83cd0eee63429` | `left: "6d11672ccddf93ea56afa3d8c1bc3a47981e999be3dae418308127f623384ccf"` |

## 4. **Trả lời câu hỏi chính: khoá cache có đổi khi bảng thẻ đổi không? — ĐO**

**Trước ST-126: KHÔNG.** Khoá cache là `vis_bytecode::format::source_hash` trên chuỗi
`"vifast-vis-cache-v2\n" + PHIEN_BAN + VERSION + chế độ + (rel, text)…` — **không** có byte nào của
bảng thẻ. Đo bằng ca ghim: `khoá = 799713b8bd33729d75c798547c3c5038782f99b48fd55b5e5849268b043b7cd5`
(log #3). Verifier **không** phải hàng rào: `load_verified` chỉ giải mã + kiểm cấu trúc
(`crates/vis-bytecode/src/verify.rs:416`), **không** biết ngữ nghĩa thẻ, nên tệp `.vbc` ghi bằng bảng
thẻ khác vẫn qua được vòng xác minh ⇒ **bytecode cũ bị dùng nhầm**. Đây là **thiếu sót thật** so với
VD-129.

**Sau ST-126: CÓ.** Khoá phủ thêm `bang_the_hash()` (SHA-256 của `số\tTên\n…` toàn bảng thẻ):
`khoá = 6d11672ccddf93ea56afa3d8c1bc3a47981e999be3dae418308127f623384ccf` (log #4) — **đổi** so với
khoá cũ (cùng đầu vào tệp/chế độ). Sửa **≈ 40 dòng** < 150 ⇒ làm luôn, không cần kế hoạch VD-116.

Ca kiểm `cache::tests::khoa_cache_doi_khi_bang_the_doi` đo 6 chiều (đều **xanh**):

| chiều | đo |
|---|---|
| (a) khoá thật = khoá tính với `bang_the_hash()` của bảng thật | khẳng định bảng thẻ **thật sự** là đầu vào, không phải nhánh chết |
| (b) đổi **tên** 1 thẻ (`Pop` → `PopDoiTen`) | hash bảng đổi **và** khoá cache đổi |
| (c) **thêm** 1 thẻ ở cuối | khoá đổi |
| (d) **hoán vị** số thẻ (đổi 5↔6) | khoá đổi |
| (e) `duong_dan()` của hai bảng | **hai tệp cache khác nhau** ⇒ tệp cũ **không** được tìm thấy |
| (f) đổi nguồn / đổi chế độ | khoá **vẫn** đổi (không mất tính chất cũ) |

Khoá cache cũng bị **ghim** bằng hex (`cache::tests::khoa_cache_ghim`) để lần sau đổi khoá là **đỏ**
(không đổi khoá im lặng).

**Vì sao unit test, không phải `tests/`:** `cache::khoa_multi` nằm trong **bin crate** `vis` (không
phải lib) ⇒ integration test không gọi được; ba danh sách thì ở `tests/api_dong_bang.rs` như brief.
Đây là chỗ **khác brief** (brief chỉ nói "ví dụ" `tests/api_dong_bang.rs`).

## 5. Lệnh + số đo

```
cd viscript
~/.cargo/bin/cargo test -p vis-cli --test api_dong_bang          → 3 passed  (52/32/53 mục khớp)
~/.cargo/bin/cargo test -p vis-cli --bin vis cache::tests        → 2 passed
~/.cargo/bin/cargo test -p vis-cli --test cache_fastpath \
                        --test cache_khong_che --test api_dong_bang
                                                                 → 3 + 5 + 5 passed, 24,45 s (cache 520 tệp ×4 / 0 lệch)
~/.cargo/bin/cargo test -p vis-bytecode                          → 5+3+12+26 passed
bash bin/vis-dev gate --fast                                     → xem §7 (đỏ do việc đang chạy của agent khác)
```

Số khoá **không đổi**: FUNCS **52**, `all_codes` **32** (7 khối), `VBC1` **53**, sem 447/22, parity
398+49, allowlist doc-examples 6. Không thêm crate, không `unsafe`.

## 6. Tệp đổi

| tệp | loại |
|---|---|
| `crates/vis-bytecode/src/format.rs` | + bảng tên thẻ + hash bảng thẻ (nguồn chuẩn) |
| `crates/vis-cli/src/cache.rs` | khoá cache phủ bảng thẻ + 2 ca unit |
| `crates/vis-cli/tests/api_dong_bang.rs` | **mới** — test chặn 3 danh sách |
| `agent/API-STD.txt`, `agent/API-CODES.txt`, `agent/API-VBC1.txt` | **mới** — 3 tệp cam kết |
| `agent/API-*.txt.license` | **mới** — sidecar SPDX (luật R3) |
| `agent/JOURNAL/st126-*.log`, `agent/JOURNAL/st126-dong-bang-api.md` | log + nhật ký |
| `agent/STATE.md` (thân), `agent/QUEUE.md` (hàng ST-126) | tiến độ |

## 7. Chỗ **lệch** so với quyết định / quan sát được (kèm dòng thực tế)

1. **Brief nói** "`format.rs` ghi hash nguồn dùng **cả** khoá cache **và** trường `hash nguồn` của
   VBC1" ⇒ **đo được là hai giá trị khác nhau, không dùng chung**:
   `crates/vis-bytecode/src/codegen.rs:217` `hash: crate::format::source_hash(text)` (chỉ **văn bản
   tệp chính**) vào trường `hash nguồn` của VBC1; còn `crates/vis-cli/src/cache.rs:28` dùng chuỗi
   closure (nhiều tệp + phiên bản + chế độ + **bảng thẻ**) làm **khoá/tên tệp cache**. Hai giá trị
   **không** được đối chiếu ở đâu (verifier đọc `source_hash` rồi bỏ qua). Hệ quả: đổi bảng thẻ ⇒
   **khoá** đổi (đã sửa) nhưng **trường hash nguồn** trong tệp cũ vẫn là hash văn bản — vô hại vì tệp
   cũ không còn được tìm thấy; ghi thành **nợ #106** (loader chưa đối chiếu hash nguồn).
2. **R3 của `bin/vis-dev`** đòi `.txt` phải có sidecar `.license` ⇒ thêm 3 sidecar (brief không nhắc).
3. `cargo fmt -- --check` trong `gate --fast` quét **cả** `vis-host-linux/` (đường phụ thuộc `path` mà
   agent khác vừa thêm vào `vis-cli/Cargo.toml`) ⇒ cổng đỏ ở tệp **ngoài** `viscript/`, không do
   ST-126. Số đo riêng của tôi: `cargo fmt -p vis-cli -p vis-bytecode -- --check` **sạch**.
4. `next_block()` (`crates/vis-diag/src/lib.rs:439`) hiện trả **801** (7 khối đã cấp) — không ảnh
   hưởng ST-126, nhưng ST-128 sẽ phải cấp mã trong khối **`07xx`** theo VD-125, tức **không** dùng
   `next_block()`; ghi ở JOURNAL ST-128.

## 8. Nợ mới (đánh số tiếp từ **105**)

| # | Nợ | Mức |
|---|---|---|
| **105** | `TEN_THE` là nguồn chuẩn nhưng **không có ràng buộc lúc biên dịch**: thêm biến thể `Instr` mới + số thẻ mới mà **quên** thêm tên vào `TEN_THE` thì test chỉ đỏ khi `SO_THE` bị đổi; cần một danh sách "mọi biến thể" sinh tự động (macro/`variant_count` chưa ổn định) hoặc script kiểm `match` của `tag()` | VS3 |
| **106** | Trường `hash nguồn` của `VBC1` (`codegen.rs:217`) là `source_hash(văn bản tệp chính)`, **không** phải khoá cache và **không** được loader/verifier đối chiếu ⇒ nếu ai đó đặt tệp `.vbc` vào đúng tên khoá, nội dung vẫn được xác minh **cấu trúc** chứ không theo nguồn. Cân nhắc: ghi khoá closure vào header (VBC2) hoặc đối chiếu hash nguồn khi nạp | VS3 |
