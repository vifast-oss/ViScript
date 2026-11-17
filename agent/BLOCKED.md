# BLOCKED — ViScript

Người ghi: **Agent 2**. Mỗi mục: task, lý do, chặn bởi ai/cái gì, cần gì để mở.
Quy tắc 2.5: kẹt **hai lần** cùng một ý ⇒ ghi vào đây, **đổi nhánh**, không thử lần ba.

## 1. REVIEW — chờ Agent 0 duyệt

| Task | Trạng thái | Nội dung chờ | Cần gì để mở |
|---|---|---|---|
| ~~ST-02a~~ | **DONE** | Đã duyệt 2026-11-12 (SPEC-lex + Phụ lục A sau errata). Không còn chặn ST-04..ST-09 | — |

**ST-02a đã DONE**; ST-04..ST-09 mở theo phụ thuộc. `ST-02b`, `ST-03`, `ST-10` độc lập.

## 2. BLOCKED theo bảng `review/PATCH-BRIEF.md` §2

| Task | Chặn bởi | Cần gì để mở |
|---|---|---|
| ST-03 | ST-01 | khung `viscript/` + `./vis-dev check` chạy; 3 case âm cho luật cấu trúc |
| ST-04 | — | ST-02a DONE |
| ST-05 | ST-04 | lexer xanh bảng token vàng + fuzz 10 phút + NFC/NFD |
| ST-06 | ST-05 | parser + CST lossless (100 % fixture tài liệu parse) |
| ST-07 | ST-05 | như trên |
| ST-08 | ST-05 | như trên |
| ST-09 | ST-06, ST-07, ST-08 | CmdCST + `vis-fmt` + runner `--check` xanh |

Không có phụ thuộc vòng. Không có task nào bị chặn vì thiếu quyết định: QD §7 đã chốt từ vựng, chế độ lệnh, ngữ nghĩa lõi (xem `DECISIONS.md` VD-11..VD-22).

## 3. Câu hỏi cho Agent 0

Khi phát sinh (SPEC thiếu/mâu thuẫn): ghi ở đây kèm dẫn chứng (`tệp:dòng`), đề xuất + **ví dụ cụ thể**, rồi chuyển task kế.
**Không mục nào dưới đây chặn ST-04** — ST-04 đã chọn cách đọc bám §1/§3/§8 và ghi rõ trong code + `JOURNAL`.

| # | Từ task | Dẫn chứng | Vấn đề | Cách ST-04 đang làm | Đề xuất cho Agent 0 |
|---|---|---|---|---|---|
| **Q1** | ST-04 | `docs/viscript/SPEC-lex.md:176`, `:178` | Hai **ví dụ** §8 lược token trong danh sách kỳ vọng: `\tif => !VS0107 INDENT` (thiếu `KW(if)`) và `(1 +\n 2) => INT + INT` (thiếu `(` `)`). Đọc nguyên văn thì không thể dùng làm golden tự động | `.tok` ghi **danh sách đầy đủ** `!VS0107 INDENT KW(if)` và `( INT(1) + INT(2) )`, kèm dòng `//` chép nguyên văn ví dụ gốc ⇒ **siết**, không nới | ghi rõ ở §8 "danh sách kỳ vọng là đầy đủ" (hoặc đánh dấu `…` cho chỗ lược) để ST-05 khỏi lặp lại tranh luận |
| **Q2** | ST-04 | `SPEC-lex.md:23` + `ViScript-thiet-ke.md` §2.4 (bảng escape) | `char` **không** có escape: escape chỉ định nghĩa cho `"…"`, `"""…"""`, `b"…"`, `p"…"`. Hệ quả: `'\n'` là VS0105 (2 scalar) và **không viết được** literal dấu nháy đơn | lexer theo đúng §1/§3 (đúng 1 scalar, không escape) | quyết định vòng 2: hoặc thêm escape cho `char` (§1 + §3), hoặc ghi rõ "dùng `'` = U+0027 qua `\u{27}` trong chuỗi" |
| **Q3** | ST-04 | `SPEC-lex.md:33` vs `:129-131`, `:169-170` | §1 xếp `.. ..= $ ^ # ##` vào `PUNCT`, nhưng §5/§6/§8 đòi kind riêng (`RANGE`/`RANGE_EQ`, `DOLLAR`, `CARET`, `SET_LBRACE`) | `..`/`..=` là `Kind::Punct` (golden nhận bí danh `RANGE`/`RANGE_EQ`); `$`/`^`/`set{` là kind riêng; `#` **không bao giờ** thành `PUNCT` (chú thích thắng) | ghi một dòng ở §1: "`.. ..=` là PUNCT, golden §8 gọi RANGE/RANGE_EQ; `$ ^` là DOLLAR/CARET ở chế độ lệnh; `#` luôn là chú thích" |
| **Q4** | ST-04 | `SPEC-lex.md:28` + `:54` vs `:57` và `ViScript-thiet-ke.md` §2.4 | Phạm vi escape của `b"…"` **mâu thuẫn nguồn**: bảng §1 ("ASCII + `\xNN`") và §3 dòng 54 ("chỉ `\xNN`") nói chỉ có `\xNN`, còn danh sách escape chung ở §3 dòng 57 và thiết kế §2.4 lại xếp `b"…"` vào nhóm có đủ `\n \t … \u{…}` | ST-04 chọn cách đọc **chặt** (SPEC-lex đứng trước thiết kế): chỉ `\xNN`; `b"\n"`, `b"\\"`, `b"\u{41}"` ⇒ **VS0102**. Hệ quả: muốn một byte `\` phải viết `b"\x5c"` | chốt một câu ở §3: dạng `b"…"` có những escape nào (nếu cho đủ bộ thì sửa `decode_bytes` — một chỗ, có test) |
| **Q5** | ST-04 | `SPEC-lex.md:22` (`INT '.' [0-9]+`) vs `:42` (`_` chỉ giữa hai chữ số) | `_` **trong phần lẻ** của `FLOAT`: §1 viết phần lẻ là `[0-9]+` (không có `_`), nhưng §2 lại phát biểu luật `_` chung cho literal số | ST-04 áp luật chung của §2: `1.2_` / `1.2__3` ⇒ VS0104; `1.2_3` **hợp lệ** (FLOAT `1.2_3`, một `_` giữa hai chữ số). Nếu đọc §1 theo nghĩa hẹp thì `1.2_3` phải là `FLOAT(1.2)` + `IDENT(_3)` | chốt: hoặc cấm hẳn `_` trong phần lẻ (§1), hoặc cho phép theo §2 như hiện tại — sửa một chỗ trong `scan_number` |

**Phụ thuộc ngoài của ST-04 (cần Agent 0 ghi nhận, không phải BLOCKED):** `vis-syntax` thêm
`unicode-normalization` (MIT OR Apache-2.0 AND Unicode-3.0) + `unicode-ident` (như trên) + `tinyvec`
(Zlib OR Apache-2.0 OR MIT) — tất cả trong allowlist QD §9.3. Dùng cho NFC toàn tệp (VD-11) và UAX #31
(`IDENT`). Hệ quả: `reports/R1-viscript.md` (ST-10, DONE) ghi "0 phụ thuộc ngoài" **không còn đúng**;
đề nghị ghi lại ở báo cáo vòng sau.

## 4. Dòng OS **không** bị chặn bởi mục 1–2

`v2/agent/QUEUE.md` còn KT-05/KT-12 READY (đã xong, chờ lật); KT-09 BLOCKED; KT-04d/T26 DONE ⇒ dòng ViScript được chạy (D2-20). Hai dòng độc lập; KT-04d REVIEW không cản dòng ViScript.

Câu hỏi mở của dòng ViScript: **YC-14** trong `v2/agent/BLOCKED.md` §3 (duyệt ST-02a đúng mẫu, xung đột `04xx`, lật ST-02b/ST-03).

## 5. BLOCKED ngoài phạm vi quyết định (đợt 25, 2026-11-17)

| Task | Chặn bởi | Cần gì để mở |
|---|---|---|
| **ST-147** (**BLOCKED phần Windows**, `fd16d7d1`) | Hạ tầng cổng đã xong theo **VD-150** (`bin/vis-dev` + `test/fixtures/run_fixtures.sh` export `PYTHONIOENCODING=utf-8`, `PYTHONUTF8=1`): gate **macOS PASS** `[exit=0]`, **10/10** bước, L0 **1159/0**. Trên `psvn` self-test **11/12** (ST-144: 7/12 — 6 ca lỗi encoding đã xanh) nhưng cổng **dừng ở bước self-test**, chưa qua bước 5. Ca 7 còn đỏ vì **mã sản phẩm**, không phải encoding: `crates/vis-cli/src/cli/check.rs:82` tách basename bằng `path.rsplit('/')` ⇒ trên Windows `phu` luôn khác rỗng ⇒ nhánh `--syntax` (dòng 97) **giữ** chẩn đoán kiểu ⇒ khối `x = 1` trong ca bị coi là đỏ ⇒ câu `ngoại lệ cũ: …` không bao giờ in. VD-150 chỉ cho hạ tầng cổng; VD-146 cấm sửa sản phẩm cho Windows ⇒ **không sửa**. | Agent 0 cấp **VD + task VS3** cho **một dòng** sửa `check.rs` (dùng `Path::file_name`, hoặc tách cả `'/'` và `'\\'`) **kèm ca khoá** cho cả hai dấu phân cách (vì sửa mà không có ca thì có thể tái phát). Sau đó ca 7 xanh ⇒ self-test **12/12** và cổng đi qua bước 5 (dự kiến dừng ở `doc-examples` hoặc L0 — ~100 ca đỏ Windows thuộc VS3). Bằng chứng: `JOURNAL/st147-gate-psvn.log` (sha256 `25e57182828198e8accdc62f8eb003c1a9156869bf42009cf88e07aa7c94fe46`, khớp hai phía), `JOURNAL/st147-chan-doan-psvn.log` (sha256 `5e2123c27083d8951db2255ff4f0a45adf031be10e53d345070604a062dc613c`, khớp hai phía), `JOURNAL/st147-psvn-env.log` (VD-136). |

## 6. BLOCKED-CHO-A0 — chờ Agent 0 quyết định thiết kế (ST-163, nợ **#146**) — **ĐÃ GỠ theo VD-176**

> **ĐÃ GỠ 2026-11-17 (VD-176)**: Agent 0 chọn **(A)** — `quyen.rs:472` `metadata` → **`symlink_metadata`**, chứng minh **tất định** bằng điểm chèn `#[cfg(test)]` (không dựa số vòng); loại (B)/(C). Việc thi hành ở **ST-174**; đóng `#146` khi mục 2 của VD-176 đạt. Nội dung dưới giữ để tra lịch sử.

**Trạng thái**: `#146` đã **phân loại (b) — TOCTOU THẬT, CHỈ macOS, khớp `VD-145`** (không phải lỗi test:
nội dung lọt là `"NGOAI"` thật). Ca **không** bị sửa, kỳ vọng `ro_ri == 0` **không** đổi, `#146` **giữ mở**.

**Cơ chế (đo được, có vết)**. Trên macOS `duong_dan_cua_handle` trả `KhongCo` (`crates/vis-cli/src/ngoai.rs:286-289`),
nên `xac_nhan_handle` rơi về `xac_nhan_sau_mo` (`crates/vis-cli/src/quyen.rs:459-485`) — xác nhận **theo tên**:
`sau = chuan(p)` (canonicalize) rồi **sau đó** đối chiếu `std::fs::metadata(&sau)` (`quyen.rs:472`) với
`(dev, ino)` của handle. `std::fs::metadata` **đi theo symlink**, còn `sau` là **chuỗi lấy từ syscall trước**:
kẻ đổi `root/tep` xen **giữa hai syscall** làm `metadata` trả inode của **tệp ngoài root** trên một **tên vẫn
nằm trong root** ⇒ `kiem_canon(sau)` xanh ⇒ **trả nội dung ngoài**. Vết thật (`JOURNAL/st163-toctou-macos.log`,
vòng 21 627):

```
xac_nhan_sau_mo(truoc=/…/root/tep, sau=/…/root/tep,
                danh_tinh_handle=(16777229,57186948), metadata(sau)=(16777229,57186948))
 | doc(dai=5, danh_tinh_handle=(16777229,57186948), duong_dan=KhongCo)
   sau_doc: symlink_metadata=symlink=true,(16777229,57195256), metadata=(16777229,57186948),
            chuan(p)=/…/ngoai.txt
```

Đo: mac **4 lần đỏ / 8 100 000 vòng** (nội dung `"NGOAI"`, 5 byte); `ps` **0 / 4 100 000** (Linux quyết định
trên `/proc/self/fd`). **`VD-145` đã ghi đúng** ("đo 0/50 000 nhưng **KHÔNG** bảo đảm theo cơ chế") — ST-163 đo
được **cơ chế hỏng** và **cửa sổ cụ thể**, không chỉ "đo được 0".

| # | Phương án | Việc phải làm | Điều kiện / rủi ro | Ước lượng |
|---|---|---|---|---|
| **A** (đề xuất) | **Vá 1 dòng trong tầm VS2**: `quyen.rs:472` `std::fs::metadata(&sau)` → `std::fs::symlink_metadata(&sau)` | Khi tên vừa bị đổi thành symlink, `symlink_metadata` trả inode **của symlink** ≠ inode handle ⇒ **từ chối**; cửa sổ còn lại (giữa `chuan` và `symlink_metadata`) không còn tạo được "tên trong root + vật ngoài root" | Không đổi API/kỳ vọng/ca; phải chứng minh bằng **≥4M vòng** mac + 3 nền gate; vẫn là "kiểm theo tên" nên **không** thay `VD-145` | 0,5 h |
| **B** | **Chờ VS3 `#137`** (fd→path trên macOS: `fcntl(F_GETPATH)`) — bỏ hẳn xác nhận theo tên | Đúng hướng thiết kế VS3 (mục 4 trong `JOURNAL/st155-vs3-dieu-tra.md` §7) | Cần `libc`/ngoại lệ `unsafe` ⇒ **ngoài VS2** (D2-17) | 2–3 task VS3 |
| **C** | Giữ ca + **tách kỳ vọng theo nền** (`cfg(target_os = "macos")` cho phép `ro_ri` nhỏ) | Sửa **kỳ vọng ca cũ** | **Chỉ A0 được duyệt** (AGENTS §4.2); che lỗi thật; **không** đề xuất | 0,25 h |

**Câu hỏi cho Agent 0.** (1) Chọn **A / B / C** (đề xuất **A**, rồi **B** ở VS3). (2) Nếu **A**: có cho phép sửa
`xac_nhan_sau_mo` (đường đọc macOS) **trong VS2** dù `VD-145` đã hoãn fd→path sang VS3 không? (3) `#146` giữ **mở**
tới khi có quyết định; ca **không** đổi, **không** `#[ignore]`, **không** đổi ngưỡng.
