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

## 7. ĐÃ GỠ — ST-171 (tách repo ViScript), 2026-11-17 (gỡ 17/11 21:5x)

Hai mục **ngoài phạm vi tự quyết** của Agent 2 (VD-165(g), VD-179 §4). Cả hai **không** chặn
phần đã làm (repo mới build + gate PASS trên mac/ps); chỉ chặn bước **remote** và chốt **vị trí**
crate host.

### 7.1 Remote của repo `ViScript` — **ĐÃ GỠ**: `https://github.com/vifast-oss/ViScript` (public, tạo + push 17/11)

| | |
|---|---|
| Chặn bởi | (đã gỡ) người điều phối cấp quyền tạo repo |
| Đã làm | Repo local `/Users/mps/dvchd/ViScript`, **459 commit** giữ lịch sử, **không** push (đúng luật "không tự commit/push") |
| Đã mở | `gh repo create vifast-oss/ViScript --public --source=. --remote=origin --push` ⇒ remote `origin` = `https://github.com/vifast-oss/ViScript.git`, nhánh `main` đã đẩy |
| Hệ quả | `psvn` clone được từ URL mới; `ps` đã trỏ `/root/viscript-moi` (cập nhật remote sang GitHub) |

### 7.2 `vis-host-linux` là **path dep trỏ ra ngoài** `viscript/` — **ĐÃ CHỐT**: giữ trong repo `ViScript`; repo cũ đã xoá

| | |
|---|---|
| Bằng chứng | `crates/vis-cli/Cargo.toml:24` — `vis-host-linux = { path = "../../../vis-host-linux" }` (bảng `[target.'cfg(target_os = "linux")'.dependencies]`) |
| Vì sao không tự quyết | VD-179 §4 nói phạm vi tách gồm "mọi đường dẫn khác mà các crate đó phụ thuộc", nhưng luật ST-171 nói path dep ra ngoài `viscript/` ⇒ **BLOCKED-CHO-A0**, không tự quyết |
| Đã làm (tạm) | **Đưa `vis-host-linux/` + `vis-host-wasm/` vào repo mới** (2 workspace riêng, `exclude` trong `Cargo.toml` gốc) và đổi path dep `../../../vis-host-linux` → `../../vis-host-linux`; **không có** tệp nào phải lấy từ repo cũ ⇒ `cargo build --workspace` + gate PASS |
| Đã chốt | (a) **giữ** `vis-host-linux`/`vis-host-wasm` trong repo `ViScript`; (c) repo `ViFastOs` **đã xoá** hai thư mục (`1a6a837c`) ⇒ không còn hai bản |
| Rủi ro nếu A0 chọn (b) | Đổi 1 dòng `Cargo.toml` + 1 dòng `SYNC_DIRS`/`SYNC_FILES` trong `bin/vis-linux` + `sync_list.rs`; không đổi mã sản phẩm |

## 8. BLOCKED-CHO-A0 — enum-hoá `Option`/`Result` **tầng kiểu** (ST-217, VD-204 (1))

**Người ghi**: Agent 2 · **Ngày**: 2026-11-18 · **HEAD khi chặn**: `7c256df` (trước commit ST-217)

**Việc đã làm** (commit ST-217, không cần A0): enum std thật ở **tầng giá trị** (`Value::Opt`/`Value::Res` gộp vào `Value::Enum`; bytecode ISA `UnwrapOpt`/`IndexOpt`/… **không** đổi ⇒ VBC1 nguyên vẹn); mọi đường in xác nhận dùng enum; **0 tệp `.out` đổi** (VD-204 (2)); **ca khoá** (`crates/vis-cli/tests/vd_204.rs`): toàn cục `Option`/`Result`/`Some`/`None`/`Ok`/`Err` ⇒ **VS0302**; match đủ nhánh hồi quy (VS0402); `?` không đổi; cheat-sheet **8 dòng** + luật cổng **R6** (+ca âm 13) + grammar EBNF bỏ token cũ; benchmark (ps): VM **+23 %** · interp **+10 %** ⇒ **nợ #152** (VD-204 (5), không tự tối ưu).

**Việc còn lại (đây là chặn)**: ở **tầng kiểu**, `Option`/`Result` vẫn là kiểu dựng sẵn (`Type::Opt`/`Type::Result`; 82 chỗ `Type::Opt` + 102 chỗ `Value::Opt` tiền-ST-217 trên 12 tệp). Enum-hoá trọn đòi **xoá `Type::Opt`** và dựng `Option`/`Result` thành enum trong bảng kiểu (`EnumDef`), tức:

1. **Checker bất biến** — `tyck::check(hir, symbols, types: &TypeTable)`: mọi instantiation `Option[…]` do **tyck tự dựng tại chỗ** (đường `coerce` tự nâng `T → T?` và đường `erase_gen`/`thay_gen` cho hàm generic) không thể intern khi bảng kiểu là `&` bất biến ⇒ phải đổi Checker sang sở hữu/mutate bảng (`RefCell<TypeTable>` hoặc trả `TypeTable` mới), rà toàn bộ ~50 chỗ `self.types.enum_def/field/rec/find_*`, và cẩn thận phép hợp nhất kiểu (`hop_nhat`) khi hai instantiation "cùng args" nhưng khác id.
2. **`Type::show` giữ `int?`** — VD-204 (2) khoá mọi chẩn đoán in `int?` (82 chỗ) + LSP hover dòng đó; bất kỳ sai sót in ra `Option[int]` là đổi byte hàng loạt golden + hover, xung đột trực tiếp VD-204 (2) ("mỗi tệp chỉ đổi đúng chuỗi này" — tức **không** được đổi).
3. **VD-204 (1) không nói rõ** có muốn xoá `Type::Opt` hay chỉ muốn "tài liệu nói thật". Tài liệu thiết kế hiện hành (A0 soạn) gọi `Option`/`Result` là **kiểu dựng sẵn** của `core` (dòng 818 `docs/ViScript-thiet-ke.md`) — mâu thuẫn văn bản với VD-204 (1) ("khai thật ở R2b"), cần A0 chốt một trong ba:
   - **(A)** Chấp nhận **tầng chạy thật + tầng kiểu dựng sẵn** là đáp án cuối (bảng `core` giữ nguyên; VD-204 (1) thoả ở mức tương đương ngữ nghĩa — không còn "kiểu giả" trong ngữ nghĩa chạy/hiển thị).
   - **(B)** Refactor **tầng kiểu** đầy đủ (bảng kiểu nội hàm + intern lazy); ước lượng **1–2 ngày**, rủi ro **10–20 chẩn đoán/golden** đổi byte, cần **VD + task riêng** (không gộp pha 1), và benchmark trước vì `hop_nhat` tốn thêm.
   - **(C)** Phương án trung: giữ `Type::Opt`, gắn cờ "std-enum" vào `EnumDef` dựng sẵn để hợp nhất logic với enum (giống tầng chạy đã làm); ít rủi ro nhất nhưng không xoá được `Type::Opt` ⇒ vẫn "kiểu dựng sẵn" về mặt loại.

**Không tự quyết** — VD-196 cấm Agent 2 đổi phạm vi VD-204; SPEC/ngữ nghĩa ngôn ngữ thuộc Agent 0 (`AGENTS.md`). **Cần gì để mở**: A0 chọn **A / B / C**; nếu B thì duyệt **VD mới + task mới** (ước lượng, mốc đo, chiến lược golden); nếu A thì sửa 1 dòng `docs/ViScript-thiet-ke.md` §8 (bảng module core) cho khớp VD-204 — A0 duyệt diff đó; nếu C thì cấp VD cho cờ `std-enum`.

**Không chặn việc khác**: pha 1 tiếp theo (R4 / R5b / R5d theo VD-207) **không phụ thuộc** quyết định này; chỉ chặn "R2b-1 hoàn tất 100 %" và tiêu chí đóng pha 1 ở phần nhắc `Option`/`Result` (bảng old→new→mã sẽ ghi rõ theo A / B / C khi ST-210 dựng bảng).

## 9. BLOCKED-CHO-A0 — ST-205 (R4a) `Error` record + `Json` enum: §6.1 cần chốt (2026-11-18)

**Người ghi**: Agent 2 · **Ngày**: 2026-11-18 · **HEAD khi chặn**: `d50ad39` (VD-219 thứ tự §1)

**Việc đã làm**: khảo sát hiện trạng §6.1 — **ViScript traits không có phương thức mang dữ liệu** (`trait Error { message, … }` ⇒ lỗi cú pháp dòng đó, `VS0201`; chỉ tồn tại `fn` items); **`Span` không tồn tại ở tầng ngôn ngữ** (chỉ là tham số Rust nội bộ); **`AnyError` không tồn tại ở cả tầng giá trị lẫn tầng kiểu** (chỉ là `TypeRef` placeholder trong chữ ký `main -> Result[(), AnyError]`); `json.decode` hiện `Result[V, String]`, phá: `null` ⇒ `None`, lỗi ⇒ **chuỗi** và **in thẳng ra `print`**.

**Bị chặn**: §6.1 có **ba** điểm thiết kế VD-187 không đủ để tự quyết (và mỗi điểm đổi ý thì đổi bảng chuyển đổi của hàng chục test treo trên `json.decode`):

1. **`trait Error` thành viên thế nào?** Candidates:
   - **(i)** marker trait `trait Error: Show` **rỗng** + `Error` là **record** với trường `message`/`source`/`span` truy cập trực tiếp (`e.message`) — gần nhật với §6.1 về field syntax nhưng không có trait ĐA PHƯƠNG THỨC nào (message/show riêng);
   - **(ii)** trait `Error: Show` với **ba phương thức** `message(self) -> str`, `source(self) -> …`, `span(self) -> …` (phương thức mặc định hoặc khai báo abstract);
   - **(iii)** space-shifting: A0 duyệt struct-level trait items trong ViScript (**thay đổi ngạo**ngôn ngữ; SPEC) trước khi ST-205 chạy.
   Đề xuất: **(ii)** hiện thựcc thể (không đổi spec, không tiêu chí trait mới, chạy được ngay với traits hiện có).
2. **`source` kiểu gì?** Candidates: **(a)** `Error?` (recursive record thật — compiler chịu được kiểu đệ quy bản ghi); **(b)** `AnyError?` (cần quyết `AnyError` có tồn tại là kiểu không); **(c)** `str?` (một dòng mô tả nguyên nhân lồng). Đề xuất: **(a)** (phù hợp §6.4 "nguyên nhân lồng"; `Error?` recursion tự nhiên).
3. **`span` kiểu gì?** Candidates: **(a)** `str?` (tệp:dòng:cột dưới dạng chuỗi — không cần kiểu Span); **(b)** bỏ field ra khỏi record và giữ trong diagnostics (span không vào user-facing record); **(c)** kiểu `Span?` mới (cần SPEC, không khả thi pha 1). Đề xuất: **(a)**.
4. **`Json` enum chạy năng lượng gì?** Candidates:
   - **(A)** enum thật **làm compile-time + runtime** — EnumDef đăng ký trong bảng kiểu mọi lượt hạ (7 biến thể... 6: `Null Bool Num Str Arr Obj`); `Num` payload `{error}` (bọc int và {float}cùng dòng {error}); `match` trong ngôn ngữ chấp nhận `j: Null:…`; ĐỔI output của `json.decode` và docs; **~40 test Rust treo** migrate.
   - **(B)** `TypeRef Json` tag **không EnumDef** — runtime cây giá trị như hiện tại (map/list/scalars); tyck không kiểm variants; `match j: Null:` **báo lỗi**, không usable cho người dùng; giá như "error path works: `match` works on Result arms (`Ok`/`Err`) chỉ trên vầng Ok — tức là giải pháp nửa vời.
   - **(C)** JSON encode/decode tôn `derives Json` và giữ cây giá trị (status quo) + chỉ đổi chữ ký SIG → `Result[Json, Error]`.
   Đề xuất: **(A)** (đây là mục tiêu thật của VD-187 "json decode được pattern match"): EnumDef `Json` đẩy vào **mọi** bảng kiểu (Builtin như Opt/Result).

**Không tự quyết** — SPEC/biểu diễn ngôn ngữ (VD-196; `AGENTS.md`). **Cần gì để mở**: A0 chọn **(1, 2, 3, 4)** với các câu trả lời số; nếu A0 muốn (4-A), duyệt VD mới + phương án migration ~40 tests + cập nhật docs §3.2 và bảng `core` trong §13 nội dung "dữ liệu động".

**Không chặn việc khác**: **ST-211 (VS3-5-pre, VD-184)** và **ST-212 (VS3-QĐ-1, VD-185)** **không** phụ thuộc quyết định này; **R4b/R4c** phụ thuộc câu trả lời 1; **R4a chỉ**.

## 10. GỠ proxy 2026-11-19 (VD-221..VD-224, uỷ quyền A0-proxy — chờ A0 ratify)

| Hạng mục | Trạng thái | Bằng chứng |
|---|---|---|
| **ST-205 (R4a) — khối §9** | **GỠ (proxy VD-222, A0 ratify)** | trait Error = (ii) ba phương thức mặc định-less; `source: Error?` đệ quy; `span: str?`; `Json` enum dựng sẵn `EnumId(0)`; bản ghi `Error` dựng sẵn `RecId(0)`; `json.decode` → `Result[Json, Error]`; **R4a/b/c merge** (`7471152`/`a6c200e`); `vd_205.rs` 10/10; cả hai đường byte-identical |
| **ST-209 (R5d) — §ASK** | **GỠ (proxy VD-223→VD-224)** | `--b` hợp lệ ở mọi hàm (docs §5.1 — đảo ngược VD-223(i)); **VS0211** (Warning) ở khai báo trùng tên module std đã import; giữ `str(…)` sau bind ⇒ VS0412; std module call giữ tĩnh-std; `vd_223.rs` 5/5 |
| **VD-186 (host patches)** | **CHỜ A0 duyệt diff** | `agent/DU-THAO-VD186/` — 7 tệp nhận diện; VD-221 không mở rộng tới VD-186 (không uỷ quyền patches cá nhân) |
| **ST-210/Pha 1** | **ps-done; mac chờ; A0 rate-limit** | VD-185: 4 dấu kiểm proxy là **proxy**, không phải A0 thật; báo cáo `BAO-CAO-A0-2026-11-19-0900.md` Q1..Q9 |
| **#152** | **giữ "chờ đo chuẩn"** | VM 1,129 / interp 1,109 (VD-220 lượt B); tối ưu ctor-enum chưa xếp lịch; **#147** mac vẫn mở (ps đo xong lượt 2) |

## 11. RESOLVED (impl) — VD-228 (proxy A0, Option B) — cần A0 ratify, không còn BLOCKED

**Người ghi**: Agent 1 (uỷ quyền A0) · **Ngày**: 2026-11-19 (sáng ~01:40) · **Trạng thái**: **IMPLEMENTED** — xem `DECISIONS.md` mục VD-228; chờ A0 ratify/revert

**Cập nhật so với bản 01:00**: đề xuất Option A ("không tự động hoá") đã **bị huỷ** sau khi đo corpus thật: `test/sem` chỉ có **1 ca** `-> T?` (`test/sem/optional/tra-bien.vis`) — thực thi Option B an toàn, byte-identical, không đổi fixture. Commit `1eea7ed`:

- interp: `opt_lift_fn_ret` ở `call_hfn_trong` — `None`/`Some`/`Ok`/`Err` giữ nguyên; còn lại gói `Value::some`.
- VM: `ret_opt: bool` trên `Emit`; `codegen/than.rs::lift_ret_opt` chèn `Instr::MakeSome` ở (a) biên thân hàm (`HBody::Expr`/`HBody::Stmts`), (b) `HStmt::Return` sớm. `MakeSome` đã flat Option (không bọc hai lớp). `codegen/than.rs` tách để giữ **R1 ≤ 1.500** dòng.
- closure-biên & argument-lift: **đã đóng ở VD-229** (`004b145`); xem addendum ngay dưới.
- **Bằng chứng**: `vd_228.rs` **5/5** byte-identical; `vis test` **469/469** hai engine; workspace **147 binary xanh / 0 đỏ**; gate **PASS 10/10**; `codegen.rs` 1492 dòng.



**Cập nhật tiếp (VD-229 @ `004b145`, ~03:55 JST)**: phần **argument** (`fn g(x: int?)` nhận `42` ⇒ `Some(42)`) và **closure-biên** (`fn(x) -> T?` suite) đều **đã áp**, byte-identical interp ↔ VM; corpus **469/469 ×2**; `vd_228.rs` **13/13**. **Closure params** không nâng (lambda params **không** parse annotation — `VS0201 mong `)`, thật `:`). BLOCKED không còn nội dung trên máy — chỉ chờ A0 ratify/revert.
**Đề nghị cho A0** (tương ứng §9-Q10 BAO-CAO-A0): giữ Option B như hiện tại (đề xuất), hoặc revert toàn bộ `1eea7ed` về Option A, hoặc chuyển sang Option C (explicit `Some(…)`).

## 12. CHỜ A0 — ST-237 / nợ #172: số học `percent` bão hoà hay lỗi?

**Người ghi**: Agent 1 (từ kết quả ST-237, luồng D) · **Ngày**: 2026-11-20 · **VD liên quan**: VD-242 (chưa chốt riêng `percent`).

**Đã đo**: 28/28 phát hiện reproduced, trong đó #172 tái hiện với `vis run` trên baseline `31b51d2`:
- `200pct * i64::MAX` ⇒ `9223372036854775807` (bão hoà im lặng);
- `1e300pct` khi `show` ⇒ `9223372036854775807pct`;
- cả hai exit 0, không lỗi.

**Vì sao chặn**: VD-242 §3.5 chốt JSON/sort, nhưng **không chốt riêng** hành vi số học `percent` khi vượt miền. `VS0507` hiện có hợp đồng cho tràn `i64` thường; chưa rõ có áp cho `percent` hay không. Đây là lựa chọn ngữ nghĩa ⇒ Agent 2 phải dừng, không tự sửa (VD-242 §6 guardrail).

**Câu hỏi cho A0** (trả lời một trong hai):
1. **Bão hoà có kiểm soát**: giữ `i64::MAX`/`i64::MIN`, nhưng phải ghi rõ hợp đồng bão hoà + sửa `show` để không in số giả; hoặc
2. **Lỗi**: `VS0507` cho mọi phép tính `percent` vượt miền (nhân/chia/show), exit ≠ 0, không trả giá trị bão hoà.

**Bằng chứng**: `agent/JOURNAL/st237-ma-tran-28.md` dòng #172; `agent/JOURNAL/st235-audit-std-cli.md` #172; `builtin.rs:419-455`, `value.rs:421-428`.

**Không chặn** các task khác: ST-238/239/240/241/242 vẫn chạy; riêng phần sửa #172 trong ST-240 phải đợi câu trả lời này.

### Gỡ blocker quyết định — VD-243 (A0, 2026-11-20)

A0 đã chốt **phương án 2 — báo lỗi**: `percent` vượt miền ⇒ `VS0507` + exit ≠ 0; **không** bão hoà. Chi tiết hợp đồng, test bắt buộc và hai điểm “không được hiểu sai” ở `DECISIONS.md` **VD-243**.

- **Blocker QUYẾT ĐỊNH cho #172: ĐÃ GỠ.** ST-240 được phép chuẩn bị repro/test/inventory và sửa song song khi không xung đột quyền tệp với ST-238; không gián đoạn ST-238.
- **Blocker TRIỂN KHAI vẫn mở**: chỉ đóng sau khi có bản sửa + bằng chứng (repro trước/sau, test hồi quy, kết quả trên đúng commit) theo lệnh A0. #172 giữ trạng thái **mở** trong `SO-NO.md`.
- Không đổi biểu diễn/thang đo/quy tắc làm tròn `percent`; chặn tại nơi tạo/chuyển đổi giá trị (không chỉ sửa chuỗi `show`); trung gian lớn mà kết quả cuối hợp lệ thì không được từ chối máy móc.

## 13. BLOCKER — Tương thích bytecode/cache sau ST-245 (VD-244), CHƯA đóng

**Người ghi**: Agent 1 theo A0 2026-11-20 · **Trạng thái**: **mở** · **VD**: VD-244 · **Task**: ST-246.

**Bối cảnh**: ST-245 chặn được repro `LoadLocal(0xFFFF)`, nhưng xác nhận toán hạng `>= GLOBAL_SLOT_BASE` có **ngữ nghĩa mới** mà `VERSION` và khoá cache vẫn giữ cũ. VM cũ chạy verifier cũ ⇒ có thể chấp nhận bytecode mới rồi diễn giải sai; verifier mới không bảo vệ được đường chạy đó.

**Phải đạt trước khi đóng** (bằng chứng VD-244 §2):
- Runtime mới từ chối artifact phiên bản cũ; runtime cũ từ chối artifact phiên bản mới.
- Cache cũ không tái sử dụng dưới ngữ nghĩa mới; source được biên dịch lại, chạy đúng interp + VM.
- Bytecode cũ không có source bị từ chối rõ (trừ khi có đường tương thích đã kiểm chứng).
- Cập nhật đúng phần hợp đồng VBC1 liên quan; không tái sinh API, không đổi opcode ngoài nhu cầu.

**Điều kiện chặn khác**: nếu runtime cũ hoàn toàn không kiểm `VERSION`, **dừng và báo A0** trước khi coi việc tăng `VERSION` là đủ.

**Không được** gộp với ST-245 (đã DONE trong phạm vi hẹp) hay tuyên bố “global-slot đã khép” — xem §14.

## 14. CẦN LÀM RÕ — Miền global suy từ lệnh ghi (VD-244 §3), CHƯA đóng

**Task**: ST-247 · **VD**: VD-244. Không phải blocker phát hành nếu không phát hiện sai, nhưng **phải có câu trả lời bằng test + mô tả ngắn** trước khi kết luận global-slot ổn.

Năm câu: (1) `StoreLocal` chỉ số rất cao có tự tăng miền chấp nhận không; (2) chặn chỉ số vượt miền mã hóa `u16` không; (3) slot thưa / chưa khởi tạo có bị đọc thành `Unit` thành công không; (4) ghi trong nhánh không chạy / hàm chưa gọi ⇒ đọc-trước-ghi xử lý sao; (5) miền verifier có khớp miền runtime cấp phát/truy cập không.

Phân biệt **hợp lệ về chỉ số** và **hợp lệ về khởi tạo**. `catch_unwind` không panic **chưa đủ**. Nếu phát hiện verifier chấp nhận truy cập ngoài miền hoặc đọc sai thành công ⇒ nâng blocker ngay.

## 15. CẦN XỬ LÝ — Option field/index assignment chưa nâng (ST-248), KHÔNG phải VD mới

**Trạng thái**: mở · **Luật**: đã có trong **VD-242 §3.3** (không cần A0 quyết thêm).

ST-239 xác nhận checker chấp nhận nhưng runtime hai engine lưu giá trị thô: `u.f = 5` (`f: int?`) ⇒ `5` thay vì `Some(5)`; `m["a"] = 7` (`map[str,int?]`) ⇒ `7` thay vì `Some(7)`. Phải chèn `$lift-opt` tại đích field/index trong codegen + mirror interp (ST-248). **Chưa được coi là hợp đồng Option đã khớp** khi xét điều kiện mở pha 2. Không có ngữ nghĩa chưa có luật; không đổi `Unwrap`/bytecode.
