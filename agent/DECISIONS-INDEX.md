<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# DECISIONS-INDEX — chỉ mục quyết định `VD`

> **Sinh tự động** từ `agent/DECISIONS.md` (ST-296d, cập nhật tới `VD-308`, 2026-11-28). Không sửa nội dung `VD` ở đây;
> nguồn sự thật vẫn là `agent/DECISIONS.md`. Mỗi `VD` một dòng: tiêu đề 1 dòng + trạng thái.
> Có **298** mục định nghĩa; dải **VD-230..VD-240** không có mục định nghĩa riêng (một số số chỉ được nhắc trong văn bản, không bịa).

| VD | Tiêu đề 1 dòng | Trạng thái | Dòng |
|---|---|---|---:|
| VD-01 | ViScript có kiểu tĩnh bắt buộc, không `any`, không null | đã chốt | 12 |
| VD-02 | Một ngôn ngữ, hai chế độ cú pháp, cùng **HIR**/kiểm kiểu/VM; ViShell là chương trình riêng | đã chốt | 13 |
| VD-03 | Ngữ nghĩa giá trị + RC + copy-on-write; không GC truy vết; chu trình chỉ qua `ref`, phá bằng `weak` | đã chốt | 14 |
| VD-04 | Lõi (syntax, types, bytecode, vm) không phụ thuộc hệ điều hành, build được wasm32 | đã chốt | 15 |
| VD-05 | Mọi truy cập hệ thống qua capability; không hàm toàn cục chạm hệ thống | đã chốt | 16 |
| VD-06 | Interpreter cây là mô hình tham chiếu, giữ mãi để kiểm vi sai VM | đã chốt | 17 |
| VD-07 | Không JIT; không `eval` chuỗi trong chế độ quyền hạn chế | đã chốt | 18 |
| VD-08 | Thay đổi SPEC chỉ qua Agent 0; mỗi tính năng mới phải có ví dụ + test trước | đã chốt | 19 |
| VD-09 | Từ khoá tiếng Anh, định danh Unicode (NFC), thông báo lỗi vi/en/ja | đã chốt | 20 |
| VD-10 | Giấy phép MIT OR Apache-2.0; phụ thuộc phải tương thích | đã chốt | 21 |
| VD-11 | Chuẩn hoá: **toàn bộ tệp nguồn được chuẩn hoá NFC trước khi lex** — 'ố' gõ kiểu nào cũng là một scalar. | đã chốt | 38 |
| VD-12 | Thụt lề: đầu vào nhận thụt lề nhất quán bất kỳ, đơn vị suy từ cấp đầu; đầu ra `vis fmt` luôn **4**. Tab: lexer báo cảnh báo-lỗi **VS0107**, coi tab = 4, **vẫn lex tiếp**; `vis c… | đã chốt | 39 |
| VD-13 | Phần trăm: **bỏ literal `50%`**, dùng `50pct`; `%` chỉ là modulo; job spec dùng `job 1`. | đã chốt | 40 |
| VD-14 | Chú thích: script — `#` ngoài chuỗi luôn là chú thích; lệnh — `#` chỉ là chú thích khi **đứng đầu từ** (sau khoảng trắng hoặc đầu dòng), nên `https://x/y#frag` giữ nguyên. | đã chốt | 41 |
| VD-15 | `\|` **không bao giờ là toán tử nhị phân trong biểu thức**. Pattern: or-pattern dùng `\|`; lệnh: pipe ở **độ sâu ngoặc 0**; `\|>` là pipe script. (Sửa Q8.) | đã chốt | 42 |
| VD-16 | Bảng ưu tiên chốt (cao → thấp): postfix (`. ?. () [] ? with`) → `**` → `-` một ngôi → `* / %` → `+ -` → `.. ..=` → `??` → so sánh (không kết hợp) → `not` → `and` → `or` → `\|>`.… | đã chốt | 43 |
| VD-17 | Float: NaN — `==` trả false; float **không** có `Eq/Ord/Hash`; kiểu chứa float không derive được `Eq/Hash`; dùng `total_cmp`. `percent` so sánh được với float theo tỉ lệ (`50pct… | đã chốt | 44 |
| VD-18 | Chuỗi: **cấm `s[i]`**; dùng `.chars() .graphemes() .bytes() .byte_at(i)?`; có `byte_len()`, `char_count()`, `grapheme_count()`; so sánh theo dãy scalar. Index: thêm `xs[i]?` trả… | đã chốt | 45 |
| VD-19 | Optional: tự nâng `T → T?` ở gán, trả, đối số; `T??` là lỗi kiểu; `?` dùng được trên `T?` trong hàm trả `U?`; pattern `none`, `some(x)`, `ok(v)`, `err(e)`. | đã chốt | 46 |
| VD-20 | Lỗi: `trait Error: Show { message, source, span }`; `AnyError` là kiểu xoá; mọi `E: Error` tự chuyển sang `AnyError` qua `?`; `main -> Result[(), AnyError]`; tên phía plugin Rus… | đã chốt | 47 |
| VD-21 | Chế độ lệnh (QD §7.2): metachar là khoảng trắng, `( ) [ ] { } ; & $ ' " \|`, `#` ở đầu từ; `"..."` có nội suy, `'...'` thô; `$x` biến phiên, `$env.NAME` môi trường; `;` tách pip… | đã chốt | 48 |
| VD-22 | Parse chế độ lệnh **hai pha**: `vis-syntax` sinh `CmdCST` (từ trần chưa định kiểu) → `cmd_lower` trong `vis-types` dùng bảng chữ ký → **HIR**. Tiêu chí "cùng AST" đọc thành **"c… | đã chốt | 49 |
| VD-23 | `docs/viscript/SPEC-lex.md` + **Phụ lục A** là nguồn từ vựng/cú pháp duy nhất; Agent 0 duyệt qua errata; ST-02a DONE sau APPROVED. | đã chốt | 50 |
| VD-24 | **Shell sở hữu tty**; tiến trình con nhận pipe, không nhận `cap[Term]`; chương trình toàn màn hình để sau với tty server. | đã chốt | 51 |
| VD-25 | `from json` không tham số trả `Json`; `where`/`get` trên `Json` là lỗi kiểu gợi ý `from json[T]`; SIGPIPE/`take` không phải lỗi; `\| complete` trả `{stdout,stderr,code,how}`. | đã chốt | 52 |
| VD-26 | Consent và cache dùng chung **hash closure** (nguồn + phụ thuộc + phiên bản compiler). | đã chốt | 53 |
| VD-27 | Mã lỗi `VS` + 4 chữ số: `01xx` từ vựng, `02xx` cú pháp, `03xx` tên, `04xx` kiểu, `05xx` runtime. `VS0412` giữ nghĩa "kiểu không khớp" (thiết kế §9). ST-02b được sửa **bảng mã lỗ… | đã chốt | 55 |
| VD-28 | `b"…"` **chỉ** có escape `\xNN` (SPEC-lex §1 hàng `BYTES`, §3 dòng 54). Mọi escape khác — kể cả `\\`, `\n`, `\u{…}` — là **VS0102**; muốn một byte `\` thì viết `\x5c`. Chốt câu… | đã chốt | 57 |
| VD-29 | `_` **trong phần lẻ** của `FLOAT` theo luật chung của §2: chỉ được **giữa hai chữ số** ⇒ `1.2_3` hợp lệ, `1.2_` và `1.2__3` là **VS0104**. (Chốt Q5: §1 viết phần lẻ là `[0-9]+`;… | đã chốt | 58 |
| VD-30 | `char` **không** có escape ở vòng 1 (đúng §1 "`'` scalar `'`" và §3 chỉ định nghĩa escape cho các dạng chuỗi) ⇒ `'\n'` là VS0105 và không viết được literal dấu nháy đơn. Xem lại… | đã chốt | 59 |
| VD-31 | Phiên giải kind khi §1 và §5–§8 lệch nhau (chốt Q3): `..`/`..=` là **`PUNCT`** (golden §8 gọi `RANGE`/`RANGE_EQ` là bí danh của cùng token); `$`, `^`, `set{` là **kind riêng** `… | đã chốt | 60 |
| VD-32 | Ba ví dụ của SPEC-lex §8 là **văn bản minh hoạ**, không phải danh sách đầy đủ: #56 (`\tif`) và #58 (`(1 +\n 2)`) **lược** token ⇒ golden ghi danh sách **đầy đủ** (`!VS0107 INDEN… | đã chốt | 61 |
| VD-33 | Ba phụ thuộc Unicode của `vis-syntax` được duyệt theo allowlist QD §9.3: `unicode-normalization` (MIT OR Apache-2.0 AND Unicode-3.0) cho NFC toàn tệp (VD-11), `unicode-ident` (n… | đã chốt | 62 |
| VD-34 | **30 golden biểu thức** của ST-05 do **Agent 0** soạn tại `viscript/test/expr/golden.expr` (kỳ vọng, **Agent 2 không sửa**: thấy sai thì báo). Định dạng: `<nguồn> => <sexp>` / `… | đã chốt | 63 |
| VD-35 | **Mã lỗi cú pháp (02xx)** cho parser ST-05 nằm ở **sổ riêng** trong `vis-diag` (`SYNTAX_CODES`), | đã chốt | 64 |
| VD-36 | **Bộ fixture tài liệu của ST-05** (chốt sau khi kiểm 22 khối ```vis trong `docs/viscript/ViScript-thiet-ke.md`): | đã chốt | 72 |
| VD-37 | **Năm chỗ Phụ lục A không tự chứa** khi đối chiếu với chính ví dụ trong tài liệu (chốt cho ST-05, | đã chốt | 82 |
| VD-38 | **Sổ lỗ hổng đã biết, hoãn có ý thức** (ghi để không ai "phát hiện lại" rồi tưởng là hồi quy; mỗi mục | đã chốt | 96 |
| VD-39 | **Phạm vi `vis fmt` vòng 1** (ST-07). Chỉ chuẩn hoá **thụt lề**, không định dạng lại trong dòng: | đã chốt | 107 |
| VD-40 | **Chế độ lệnh vòng 1 (ST-06) — 5 chỗ §A.7/§9.3 cần chốt** (kèm 22 golden ở | đã chốt | 114 |
| VD-41 | **Đính chính 3 ca của bộ golden chế độ lệnh** (sau khi hiện thực pha B va vào 2 khối | đã chốt | 131 |
| VD-42 | **Sổ mã lỗi vòng 2** (theo VD-27: `03xx` tên, `04xx` kiểu, `05xx` runtime) — vẫn là **sổ riêng** trong | đã chốt | 146 |
| VD-43 | **Phạm vi ngữ nghĩa VS0** (chốt để ST-11..ST-15 có hợp đồng; thiết kế §3–§6 là nguồn, chỗ nào | đã chốt | 152 |
| VD-44 | **Hạ CST→HIR phải PERMISSIVE** (ST-11): HIR biểu diễn **mọi** thứ trong Phụ lục A ở mức cú pháp — | đã chốt | 171 |
| VD-45 | **Chính sách giải tên** (ST-11): `NamePolicy::Permissive` là **mặc định** — dùng tên tự do được phép | đã chốt | 177 |
| VD-46 | **Chốt 3 câu hỏi của ST-12** (kiểm kiểu VS0): | đã chốt | 183 |
| VD-47 | **Ngữ nghĩa VS0 chốt ở ST-13**: (1) **khối cho giá trị của câu lệnh cuối** — `Expr`, `If`, `Match`, khối | đã chốt | 194 |
| VD-48 | **Phạm vi kiểu VS0 (chốt sau review ST-12)**: `AnyError`, `cap`, `ref`, `Json` **không** thuộc VS0 — | đã chốt | 203 |
| VD-49 | **Chốt sau review ST-13** (ST-13b): (1) thêm `Type::Never` — `return`, nhánh `if`/`match` toàn phân kỳ | đã chốt | 213 |
| VD-50 | **Chốt ST-14**: nội suy `{expr}` nằm ở **HIR** (`HExpr::Interp(Vec<InterpPart>)`), hạ **trong cùng | đã chốt | 220 |
| VD-51 | **Closure chụp theo snapshot (VS0)** — ST-15b chọn: `HExpr::Lambda` chụp **bản sao** các biến đang sống | đã chốt | 258 |
| VD-52 | **Phạm vi vòng 3 (khung VS1)** — chốt trước khi làm: (1) **bytecode** tự chứa, có **magic `VBC1`** + | đã chốt | 265 |
| VD-53 | **Chế độ lệnh chạy được (ST-18)**: (1) **khe ống** = tham số **vị trí đầu tiên theo thứ tự khai báo** của | đã chốt | 276 |
| VD-54 | **Bytecode VBC1 (ST-19)**: (1) hash nguồn 32 byte = **FNV-1a×4** (không thêm dep) — **không** phải SHA-256; | đã chốt | 288 |
| VD-55 | **VM dùng chung ngữ nghĩa (ST-20/20b)**: (1) `vis-vm` **không** chép ngữ nghĩa — gọi `vis-interp` | đã chốt | 296 |
| VD-56 | **Chốt VS1-khung (ST-21..ST-23)**: (1) verifier **kiểm lại từ byte stream** và codegen dùng **chính** | đã chốt | 307 |
| VD-57 | **Phạm vi vòng 4 (VS1b)**: (1) **module** = **mỗi tệp một module** (§7.10); `use ./util` **đường dẫn tương | đã chốt | 317 |
| VD-58 | **Module (ST-25)**: (1) **mỗi tệp một module**; `use ./x` tương đối (thử `.vish` trước nếu tệp chính là | đã chốt | 330 |
| VD-59 | **vis-std lõi (ST-26)**: (1) tên **đầy đủ theo module** (`str.split`, `path.join`, `time.now_ms`, | đã chốt | 340 |
| VD-60 | **Chương trình ngoài (ST-27)**: (1) tên lệnh **lạ** trong `.vish` = **chương trình ngoài** (§9.3); chạy | đã chốt | 350 |
| VD-61 | **Generic VS1b (ST-28)**: (1) **xoá kiểu (erase) + hợp nhất ở call-site**, **không** đơn hình hoá — | đã chốt | 361 |
| VD-62 | **Trait/impl VS1b (ST-29)**: (1) **dispatch TĨNH ở tầng hạ** — `x.m(…)` với `x` có kiểu **tĩnh** | đã chốt | 372 |
| VD-63 | **`vis test` + SHA-256 + đồng bộ chẩn đoán (ST-30/30b)**: (1) **SHA-256 thật** (`sha2 0.10`, MIT OR | đã chốt | 384 |
| VD-64 | **Phạm vi vòng 5 (VS1c — đóng nợ `R4` §5)**: (1) **harness chẩn đoán biên dịch**: so **đủ** | đã chốt | 395 |
| VD-65 | **Harness chẩn đoán 3 đường (ST-32)**: (1) mọi ca `.vis`/`.vish` được phân loại **cứng** thành 3 tầng — | đã chốt | 407 |
| VD-66 | **`json.decode` + `std.fs` (ST-33)**: (1) `json.decode(s) -> Result[Any, str]` (tyck dùng `Ty::Any` ⇒ **bỏ | đã chốt | 416 |
| VD-67 | **Pipe→stdin + `\| complete` (ST-34)**: (1) `a \| b` khi `b` là **chương trình ngoài** ⇒ stdout của `a` được | đã chốt | 427 |
| VD-68 | **Chữ ký lệnh generic (ST-37)**: (1) `CmdSig` mang **tham số kiểu** `[T]`; so khớp tham số khi hạ | đã chốt (+1 mục) | 436 |
| VD-69 | **Closure-arg trong chế độ lệnh (ST-38)**: (1) `closure_arg` (§A.7) hạ **trong `cmd_lower`** bằng | đã chốt (+1 mục) | 444 |
| VD-70 | **Ngữ nghĩa verb shell (ST-38)**: (1) **một** ngữ nghĩa, **hai** tên: `where` ≡ `.filter`, | đã chốt (+1 mục) | 451 |
| VD-71 | **`sort_by` (ST-39)**: (1) Mở rộng `list.sort_by` cho khoá `K ∈ {int, size, duration, str}` (hiện | đã chốt (+1 mục) | 459 |
| VD-72 | **`it` + viết tắt trường (ST-40a)**: (1) Chỉ áp cho **tham số closure** của `cmd` đã biết chữ ký; | đã chốt (+1 mục) | 464 |
| VD-73 | **Prelude bản ghi + `Host` proc (ST-40a)**: (1) `FileEntry{name: str, path: path, size: size, | đã chốt (+1 mục) | 471 |
| VD-74 | **`Render` (ST-41)**: (1) Giá trị cuối pipeline in qua `Render`: `list[bản ghi]` ⇒ **bảng** (tiêu đề | đã chốt (+1 mục) | 479 |
| VD-75 | **`ls`/`open` qua `Host` (ST-42)**: (1) `FsHost::list_entries(dir) -> Result[list[FileEntry], str]` | đã chốt (+1 mục) | 485 |
| VD-76 | **`ls`/`open` + `--fs-mock` (ST-42)**: (1) `ls [p] [--all] [--long]` ⇒ `list[FileEntry]` (prelude VD-73), | đã chốt | 566 |
| VD-77 | **Phạm vi vòng 7 (VSH1b)**: (1) **`.vish` có khai báo kiểu** — `type`/`enum`/`let x: T`/`cmd f(…) -> T` được | đã chốt | 575 |
| VD-78 | **Khai báo trong `.vish` (ST-44)**: (1) `.vish` = **vùng item ở đầu tệp** + **vùng stage** phía sau; ranh giới là | đã chốt | 584 |
| VD-79 | **`where` kiểm tĩnh / `each` không kiểm (ST-45)**: (1) **`where`** kiểm ở **`tyck`** (`coerce` thân lambda, | đã chốt | 593 |
| VD-80 | **`ls --long` + nhóm-đối-số (ST-46)**: (1) `ls --long` ⇒ **`list[FileEntryLong]`** (`name,size,modified,is_file, | đã chốt | 601 |
| VD-81 | **Phạm vi vòng 8 (VSH1c)**: (1) **thân `cmd` bằng pipeline lệnh**: khối thụt sau `cmd f(...) -> T:` parse ở | đã chốt | 609 |
| VD-82 | **Thân `cmd` bằng pipeline (ST-48)**: (1) Parser **thử `Mode::Script`** trên thân (bỏ thụt): sạch ⇒ **script** | đã chốt | 618 |
| VD-83 | **Nhóm-lệnh + nhóm nhiều dòng (ST-49)**: (1) **một** đường hạ nhóm duy nhất (`lower_nhom_lenh` tự-dựng-Call | đã chốt | 625 |
| VD-84 | **Trường là lỗi kiểu ⇒ `VS0405` (ST-50)**: (1) mã "không có trường" nằm ở sổ **`04xx`** (**`VS0405`**), không ở | đã chốt | 631 |
| VD-85 | **Stage đầu là biểu thức (ST-51)**: (1) Nhận **hẹp nhất** trong chế độ lệnh: token **đầu dòng** là `[`, `{`, hoặc | đã chốt | 639 |
| VD-86 | **Phạm vi vòng 9 (VSH2)**: (1) **Bản ghi ẩn danh đi được JSON**: `select`/literal ẩn danh phải `json.encode` | đã chốt | 646 |
| VD-87 | **`to json` + hợp nhất `sort_by` (ST-53)**: (1) **`to json`** là **stage lệnh** (hạ `Call(std.json.encode, | đã chốt | 654 |
| VD-88 | **Codegen từ chối HIR có chẩn đoán (ST-54)**: (1) **Không** chặn bằng soi `Type::Error`/`TypeRef` — walker chứng | đã chốt | 661 |
| VD-89 | **Phạm vi vòng 10**: (1) Sáu task **ST-56..ST-61** + **ST-62 (R10)**, chạy **tuần tự** (đụng chung | đã chốt | 668 |
| VD-90 | **Async — tác vụ chạy tới hoàn tất, một luồng, tất định (ST-60)**: (1) `async fn` khai báo được; **gọi** | đã chốt | 673 |
| VD-91 | **Kênh — đệm hữu hạn + deadlock có chẩn đoán (ST-61)**: (1) `chan[T](n)`, `n ≥ 0`; `send(v)`, | đã chốt | 682 |
| VD-92 | **`&self`/`&expr` — không mượn thật (ST-56)**: (1) `&self` trong `impl` là receiver **không mượn thật**, | đã chốt | 688 |
| VD-93 | **`derives` sinh mã ở `lower` (ST-57)**: (1) Sinh **HIR** trong `lower` (không macro, không sửa parser). | đã chốt | 693 |
| VD-94 | **Bỏ `Ty::Any` bằng biến kiểu trong chữ ký std (ST-58)**: (1) `stdlib.rs` dùng **biến kiểu** thay `Any` | đã chốt | 698 |
| VD-95 | **Hiện thực async — builtin nội bộ, không thêm tag `VBC1` (ST-60)**: (1) `async`/`await` **đã** lex+parse+lower | đã chốt | 704 |
| VD-96 | **Hiện thực kênh + ba mã runtime mới (ST-61)**: (1) `$kenh` tạo `Chan` (**một** hiện thực trong `builtin`), | đã chốt | 715 |
| VD-97 | **Một ngữ nghĩa cho phép gán (ST-56b)**: (1) `self` là **tham số bất biến** như mọi tham số ⇒ `self.x = v` | đã chốt | 722 |
| VD-98 | **Mồ côi đa tệp + siêu trait + derives sinh mã (ST-57)**: (1) **Mồ côi**: cho `impl` khi **trait hoặc | đã chốt | 729 |
| VD-99 | **Bỏ `Ty::Any`: biến kiểu trong chữ ký `std.*` (ST-58)**: (1) `Ty` const-friendly giữ `Copy`, thêm | đã chốt | 740 |
| VD-100 | **Net/HTTP: socket thô ở host, giao thức ở `vis-interp` (ST-63..ST-66, vòng 11)**: (1) Thêm `NetHost` cạnh | đã chốt | 750 |
| VD-101 | **Gói + registry local (VS2 #6, ST-6x)**: (1) **Không mạng**: registry là **thư mục local** (mặc định `./registry`, | đã chốt | 761 |
| VD-102 | **LSP `vis lsp` (VS2 #5, ST-6x)**: (1) **Tự viết framing** `Content-Length` trên `stdin/stdout` nhị phân; **không** | đã chốt | 771 |
| VD-103 | **Quyền: `cap[Dir]`/`cap[Proc]`/`cap[Net]` cưỡng chế ở **biên host** (ST-72, ST-73)**: (1) Ngôn ngữ **đã** có cú | đã chốt | 782 |
| VD-104 | **Đa nền tảng: bằng chứng theo **mức** (ST-74, ST-75)**: (1) `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown` | đã chốt | 790 |
| VD-105 | **`Host` mở rộng: `NetHost` + quyền của phiên (ST-63..ST-66)**: (1) Thêm `NetHost` cạnh `FsHost` trong | đã chốt | 797 |
| VD-106 | **`cap[...]` là kiểu thật, cưỡng chế **chỉ khi khai báo** (ST-72/ST-72b)**: (1) `cap[Dir\|File\|Net\|Proc\|Env\|Term]` là | đã chốt | 808 |
| VD-107 | **Landlock: chỉ **probe** + khung, **không** giả vờ (ST-73)**: (1) Máy phát triển là macOS ⇒ **không** có Landlock để | đã chốt | 818 |
| VD-108 | **Vòng 12 — "nhất quán hoá" tính năng mới (ST-77..ST-82)**: (1) Sau vòng 11, các tính năng VS2 **chạy được** nhưng chưa | đã chốt | 828 |
| VD-109 | **Vòng 13 — đóng nợ nhỏ + kiểm chứng sâu (ST-83..ST-88)**: (1) Ba nợ nhỏ phải đóng: `fs.exists` ngoài quyền **không** | đã chốt | 836 |
| VD-110 | **Vòng 14 — đóng khoảng lệch cuối (ST-89..ST-92)**: (1) **`Ngoai` cho VM**: đây là **khoảng lệch interp ↔ VM duy nhất | đã chốt | 843 |
| VD-111 | **Vòng 15 — đóng nốt mô hình quyền + ngữ nghĩa kênh (ST-93..ST-95)**: (1) **`cap[Proc]`**: chương trình ngoài (`^cmd`, | đã chốt | 850 |
| VD-112 | **Vòng 16 — nhất quán sổ mã + allowlist `cap[Proc]` + soak + bàn giao (ST-96..ST-99)**: (1) **Bất biến sổ mã**: mọi chuỗi | đã chốt | 857 |
| VD-113 | **Vòng 17 — đầu ra máy đọc + LSP hoàn thiện nhỏ + pkg update (ST-100..ST-103)**: (1) **`--json`** cho `vis check` và `vis test`: | đã chốt | 865 |
| VD-114 | **Vòng 18 — HTTP thật hơn + timeout ca test + R1 headroom (ST-104..ST-107)**: (1) HTTP client (loopback) phải xử lý | đã chốt | 871 |
| VD-115 | **Vòng 19 — hạ R1 max + HTTP/Location + `--json` chẩn đoán theo tệp (ST-108..ST-111)**: (1) Tách `lower.rs` để **R1 max ≤ 1.200** | đã chốt | 878 |
| VD-116 | **Vòng 20 — đóng nợ correctness chẩn đoán đa tệp, rồi soak cuối + R20 (ST-112..ST-113)**: (1) **Chẩn đoán trước, sửa sau**: | đã chốt | 884 |
| VD-117 | **Vòng 21 — đóng nợ correctness `Span` (ST-114a/b, ST-115)**: theo kế hoạch `st112-ke-hoach.md`. (1) `Span` thêm **`file: u16`** | đã chốt | 891 |
| VD-118 | **Vòng 22 — hạ R1 lần cuối, LSP đa tệp, soak chốt + R22 (ST-116..ST-118)**: (1) Tách `parse/mod.rs` (**1.452**) để **R1 max ≤1.200**, | đã chốt | 898 |
| VD-119 | Quy trình: Agent 0 soạn, Agent 2 dán (2026-11-16) | đã chốt | 904 |
| VD-120 | Runner Linux bằng container + Landlock có điều kiện (2026-11-16) | đã chốt | 907 |
| VD-121 | wasm và Windows: chỉ cargo check trong VS2 (2026-11-16) | đã chốt | 910 |
| VD-122 | HTTP: chưa có HTTPS trong VS2 (2026-11-16) | đã chốt | 913 |
| VD-123 | Async và chan(0) trong VS2 (2026-11-16) | đã chốt | 916 |
| VD-124 | Siết caps mặc định cho chương trình dùng gói (2026-11-16) | đã chốt | 919 |
| VD-125 | Semver ranges + resolver tối giản (2026-11-16) | đã chốt | 922 |
| VD-126 | LSP publish một tài liệu (2026-11-16) | đã chốt | 925 |
| VD-127 | Span tổng hợp phải mang file (2026-11-16) | đã chốt | 928 |
| VD-128 | R1 chỉ có một ngưỡng: ≤1.500 dòng/tệp (2026-11-16) | đã chốt | 931 |
| VD-129 | Đóng băng API std và cam kết tương thích (2026-11-16) | đã chốt | 934 |
| VD-130 | Kết luận review độc lập vòng 10+ (2026-11-16) | đã chốt | 937 |
| VD-131 | Định nghĩa VS2 đóng + 5 câu bắt buộc (2026-11-16) | đã chốt | 940 |
| VD-132 | Ca test phụ thuộc nền (2026-11-16) | đã chốt | 943 |
| VD-133 | ST-130: chi tiết siết caps khi dùng gói (2026-11-16) | đã chốt | 946 |
| VD-134 | Runner Linux tái lập được (2026-11-16) | đã chốt | 949 |
| VD-135 | Gói: ghim khi thêm và khi migration (2026-11-16) | đã chốt | 952 |
| VD-136 | Máy đo thật qua SSH: Linux (ps) và Windows (psvn) (2026-11-16) | đã chốt | 955 |
| VD-137 | Phân vai dán văn bản và đối chiếu duyệt (2026-11-16) | đã chốt | 959 |
| VD-138 | Chấp nhận runner Linux qua SSH cho ST-121 (2026-11-16) | đã chốt | 962 |
| VD-139 | Agent 1 thêm chú thích theo văn bản quyết định (2026-11-16) | đã chốt | 965 |
| VD-140 | Cập nhật công cụ trên psvn cho ST-136 (2026-11-16) | đã chốt | 968 |
| VD-141 | Rò rỉ TOCTOU đường đọc trên Linux là lỗi chặn VS2 (2026-11-16) | đã chốt | 971 |
| VD-142 | Báo cáo đã commit không sửa; đính chính ở báo cáo kế tiếp (2026-11-16) | đã chốt (đính chính) | 974 |
| VD-143 | Task chốt không lật DONE trước điều kiện (2026-11-16) | đã chốt | 977 |
| VD-144 | Ca symlink_canonicalize_hai_phia không phụ thuộc chương trình hệ thống (2026-11-16) | đã chốt | 980 |
| VD-145 | macOS không có fd→path trong VS2 (2026-11-16) | đã chốt | 983 |
| VD-146 | Windows trong VS2: chỉ đo, cổng chạy được (2026-11-16) | đã chốt | 986 |
| VD-147 | Quy trình sau đợt 24 (2026-11-16) | đã chốt | 989 |
| VD-148 | Chốt VS2 (2026-11-16) | đã chốt | 992 |
| VD-149 | Cấp số nợ 142/143 (2026-11-17) | đã chốt | 995 |
| VD-150 | Windows: UTF-8 cho mọi lần gọi python của cổng (2026-11-17) | đã chốt | 998 |
| VD-151 | VS2 đóng về kỹ thuật (2026-11-17) | đã chốt | 1001 |
| VD-152 | Sửa tên gốc khi check để `--syntax` lọc đúng trên Windows (2026-11-17) | đã chốt | 1004 |
| VD-153 | Cấp số #144 + mở điều tra VS3 (2026-11-17) | đã chốt | 1007 |
| VD-154 | Đính chính hiến chương VS3 + #143 → VS3 + số #145 (2026-11-17) | đã chốt | 1010 |
| VD-155 | Đính chính số sổ sau ST-154 (2026-11-17) | đã chốt | 1013 |
| VD-156 | Đợt 28: parity argv (#110) + một nguồn đếm khối (#129) (2026-11-17) | đã chốt | 1016 |
| VD-157 | Hợp thức hoá quyết định đêm 17/11 + giới hạn quyền ban hành thay Agent 0 (2026-11-17) | đã chốt | 1019 |
| VD-158 | Ca đua symlink đỏ 1/50 000 KHÔNG xếp T-INFRA (2026-11-17) | đã chốt | 1022 |
| VD-159 | Ca thời gian đo min của 5 lần (2026-11-17) | đã chốt | 1025 |
| VD-160 | VS3 mục 1: heap riêng mỗi luồng, bộ đếm không atomic (2026-11-17) | đã chốt | 1028 |
| VD-161 | VS3 mục 3/4: chính sách crate, giấy phép, unsafe (2026-11-17) | đã chốt | 1031 |
| VD-162 | VS3 mục 5: định nghĩa "Windows hỗ trợ thật" (2026-11-17) | đã chốt | 1034 |
| VD-163 | VS3 mục 2: select là hàm std `recv_any` (2026-11-17) | đã chốt | 1037 |
| VD-164 | VS3: xác nhận nợ đường GHI và ma trận phụ thuộc (2026-11-17) | đã chốt | 1040 |
| VD-165 | Tách ViScript thành repo riêng "ViScript" (2026-11-17) | đã chốt | 1046 |
| VD-166 | Luật chung: Agent 1 quyết định thay Agent 0 (2026-11-17) | đã chốt | 1049 |
| VD-167 | Tài liệu công thái học ViScript + bổ sung VD-165(4b) (2026-11-17) | đã chốt (bổ sung) | 1052 |
| VD-168 | Mở rộng công thái học ViScript (thay mục 2, 4 của VD-167) (2026-11-17) | đã chốt | 1055 |
| VD-169 | Chia ST-172 thành 172a (dữ kiện) và 172b (đề xuất) (2026-11-17) | đã chốt | 1058 |
| VD-170 | Phân tích vs-ergo-facts, mở ST-172b (2026-11-17) | đã chốt | 1061 |
| VD-171 | Bộ quy tắc công thái học ViScript (R1–R10) (2026-11-17) | đã chốt | 1064 |
| VD-172 | Cú pháp còn lại R11–R21 (bổ sung VD-171) (2026-11-17) | đã chốt (bổ sung) | 1067 |
| VD-173 | Chia pha công thái học + R19c + R14 (2026-11-17) | đã chốt | 1070 |
| VD-174 | Nguyên tắc mô hình ViScript (bổ sung R14) (2026-11-17) | đã chốt (bổ sung) | 1073 |
| VD-175 | ST-166, landlock, #146 (2026-11-17) | đã chốt | 1076 |
| VD-176 | #146: vá (A) + chứng minh tất định (2026-11-17) | đã chốt | 1079 |
| VD-177 | ST-167, nợ #147, cây ps (2026-11-17) | đã chốt | 1082 |
| VD-178 | Duyệt 6 tài liệu VS3, bảng Windows, vs-ergo.md (17/11) | đã chốt | 1086 |
| VD-179 | Cây ps, phạm vi tách repo, đóng băng (17/11) | đã chốt | 1089 |
| VD-180 | Mô hình gói ViScript (thay QĐ-6 mục 1, 2 của VD-178) (17/11) | đã chốt | 1092 |
| VD-181 | Tên miền cho mô hình gói ViScript (17/11) | đã chốt | 1095 |
| VD-182 | Tối ưu script dev + đo lại #147 (2026-11-18) | đã chốt | 1098 |
| VD-183 | HTTP: mặc định timeout/redirect/body (2026-11-18) | đã chốt | 1102 |
| VD-184 | Ngoại lệ VD-146 cho `check.rs:82` (2026-11-18) | đã chốt | 1105 |
| VD-185 | Mở VS3 chính thức (2026-11-18) | đã chốt | 1108 |
| VD-186 | Tài liệu mô tả mã hiện tại (2026-11-18) | đã chốt | 1111 |
| VD-187 | Thứ tự pha 1 + tiêu chí đóng pha 1 (2026-11-18) | đã chốt | 1114 |
| VD-188 | Windows "hỗ trợ thật" (2026-11-18) | đã chốt | 1117 |
| VD-189 | VS3-6 gói (2026-11-18) | đã chốt | 1120 |
| VD-190 | #145 | đã chốt | 1123 |
| VD-191 | #140/#109b | đã chốt | 1126 |
| VD-192 | psvn `audit_span` (2026-11-18) | đã chốt | 1129 |
| VD-193 | Khoảng trắng quanh `.` (2026-11-18) | đã chốt | 1132 |
| VD-194 | Xác nhận 9 nhóm quyết định / 16 `[SỬA]` (2026-11-18) | đã chốt | 1135 |
| VD-195 | `SO-NO.md` §8 phải khớp §4 (2026-11-18) | đã chốt | 1138 |
| VD-196 | Thẩm quyền Agent 1 trong đợt này (2026-11-18) | đã chốt | 1141 |
| VD-197 | Q1 · Nợ #147 → câu chữ **B**, nhưng **mở lại cho tới khi đo đủ** (2026-11-18) | đã chốt | 1144 |
| VD-198 | Q2 · Nội dung gói → **B: viết thẳng vào §8**, kèm nợ mới (2026-11-18) | đã chốt | 1147 |
| VD-199 | Q3 · "§4" của `HOST-BAO-DAM.md` → **A: không đổi tiêu đề**, sửa chỗ trỏ (2026-11-18) | đã chốt | 1150 |
| VD-200 | Q4 · 3 chỗ sửa ngoài §4 → **giữ cả 3** (2026-11-18) | đã chốt | 1153 |
| VD-201 | Q5 · Câu "macOS còn khe hở" → **xác nhận** (2026-11-18) | đã chốt | 1156 |
| VD-202 | Q6 · Số liệu Windows → **giữ số, ghi nguồn**, nhưng **đối chiếu self-test trước** (2026-11-18) | đã chốt | 1159 |
| VD-203 | Q7 · Nhãn `[thiết kế VS3-6]` → **giữ 4 chỗ** (2026-11-18) | đã chốt | 1162 |
| VD-204 | Q8 + Q9 · Hiển thị `None`/`Some` và enum std thật → **đổi trong R2b, khai thật**, **chia R2b** (2026-11-18) | đã chốt | 1165 |
| VD-205 | Q10 · Cheat-sheet → **sửa ngay** + thêm luật cổng (2026-11-18) | đã chốt | 1168 |
| VD-206 | Quy tắc đo hiệu năng (2026-11-18) | đã chốt (bổ sung) | 1171 |
| VD-207 | Thứ tự tiếp theo (2026-11-18) | đã chốt | 1174 |
| VD-208 | đến VD-214 (2026-11-18) | đã chốt (+1 mục) | 1190 |
| VD-209 | Mục 2 · Nợ #152 chậm hơn 10 % → **(c) đo chuẩn trước, rồi (b) nếu cần** | đã chốt | 1215 |
| VD-210 | Mục 3 · R5b → **(ii) huỷ R5b**, chốt quy tắc khoá trong `{…}` | đã chốt | 1226 |
| VD-211 | Mục 4 (a) · `Foo.A(5) == Bar.A(5)` nay là false → **chấp nhận**, kèm kiểm tầng kiểu | đã chốt | 1254 |
| VD-212 | Mục 4 (b) · VM khớp biến thể **không phân biệt hoa thường** → **PHỦ QUYẾT** | đã chốt | 1262 |
| VD-213 | Mục 4 (c) · `let-err` ra VS0412 → **chấp nhận**, kèm gợi ý | đã chốt | 1281 |
| VD-214 | Thứ tự tiếp theo | đã chốt | 1286 |
| VD-215 | đến VD-219 (2026-11-18) | đã chốt (+1 mục) | 1322 |
| VD-216 | Mục 2 · VS0111 | đã chốt | 1368 |
| VD-217 | Mục 3 · #152 và cách đo khi máy không rảnh được → **(c)**, thêm cách đo A/B xen kẽ | đã chốt | 1377 |
| VD-218 | Mục 4 · `$value:<tên>` → **ghi nhận**, thêm điều kiện không lộ tên nội bộ | đã chốt | 1400 |
| VD-219 | Thứ tự tiếp theo | đã chốt | 1409 |
| VD-220 | LỆNH CHẠY QUA ĐÊM (2026-11-18, hiệu lực 19:00 → 09:00 sáng 2026-11-19) | đã chốt | 1440 |
| VD-221 | Uỷ quyền Agent 0 cho Agent 1 trong khung đêm 2026-11-18→19 (proxy A0) | đã chốt | 1614 |
| VD-222 | R4a chốt theo **uỷ quyền A0-proxy** (VD-221) | đã chốt | 1626 |
| VD-223 | R5d chốt theo **uỷ quyền A0-proxy** (VD-221) | đã chốt | 1643 |
| VD-224 | Đảo ngược VD-223 (phần (i)) | đã chốt | 1655 |
| VD-225 | Uỷ quyền kết thúc | đã chốt | 1664 |
| VD-226 | `while let` | đã chốt (bổ sung) | 1674 |
| VD-227 | Interp tự nâng `T → T?` tại `let` annotate | đã chốt (bổ sung) | 1683 |
| VD-228 | Function-return `-> T?` lift `T → T?` | đã chốt | 1691 |
| VD-229 | Argument-biên lift `T → T?` + closure-biên lift | đã chốt | 1701 |
| VD-241 | Uỷ quyền chạy tự động (thay VD-221/VD-225) | đã chốt (+1 mục) | 1716 |
| VD-242 | Mở đợt “Ổn định sau pha 1” | đã chốt | 1922 |
| VD-243 | Số học `percent`: lỗi có kiểm soát, không bão hoà | đã chốt (bổ sung) | 2026 |
| VD-244 | Bytecode/cache tương thích: tăng VERSION, phân biệt cache | đã chốt | 2083 |
| VD-245 | Global chưa khởi tạo: runtime trap, không trả Unit | đã chốt | 2139 |
| VD-246 | #189 capture/local ngoài miền: verifier + runtime trap | đã chốt | 2195 |
| VD-247 | Nghiệm thu kỹ thuật có phạm vi cho `dbae70a` | đã chốt | 2236 |
| VD-248 | Tạm dừng phát hành; đánh giá mức sẵn sàng sử dụng thực tế | đã chốt | 2303 |
| VD-249 | Đầu tư cơ chế async thực dụng; hợp đồng ngân sách; #148; đo vòng đời | đã chốt | 2343 |
| VD-250 | Duyệt frame heap + Lát 0 có kiểm soát; sửa 3 bất biến; #148 chọn A | đã chốt | 2433 |
| VD-251 | Nghiệm thu checkpoint; duyệt Lát 1 nối runtime thật có kiểm soát | đã chốt | 2536 |
| VD-252 | Nhận Lát 1 có giới hạn; duyệt Lát 2 reactor; chốt 5 quy tắc tính phí; ghim toolchain | đã chốt | 2619 |
| VD-253 | Override VD-252 §6 | đã chốt | 2704 |
| VD-254 | Duyệt `mio` và Lát 3 | đã chốt | 2719 |
| VD-255 | Nhận Lát 3 chức năng; chưa đóng checkpoint tích hợp; khép coverage frame | đã chốt | 2787 |
| VD-256 | Khép checkpoint Lát 3; chốt D1–D9; mở ngân sách, connect/write, sửa final-join | đã chốt | 2865 |
| VD-257 | Ủy quyền tự chủ cuối tuần trên máy `ps` | đã chốt | 2952 |
| VD-258 | Nhận stream A (checker/record/None) | đã chốt | 3053 |
| VD-259 | Nhận stream A (frame/async) | đã chốt | 3059 |
| VD-260 | Nhận stream A (rest pattern + refutable) | đã chốt | 3065 |
| VD-261 | SQLite: cache schema + chính sách đọc | đã chốt | 3071 |
| VD-262 | Method body + globals + exit code | đã chốt | 3077 |
| VD-263 | Let-else/with/`[]`/generic + method-return | đã chốt | 3082 |
| VD-264 | Range lười + suy kiểu pattern/call | đã chốt | 3090 |
| VD-265 | Duyệt 5 helper stdlib thực dụng | đã chốt | 3096 |
| VD-266 | Duyệt pexpr/intlit/fs/db/method-ret | đã chốt | 3105 |
| VD-267 | Duyệt test filter + format size/duration + fmt stdin | đã chốt (+2 mục) | 3121 |
| VD-268 | `std.fs.walk` | đã chốt (+3 mục) | 3141 |
| VD-269 | `std.str.pad_*` + trim một phía | đã chốt (+3 mục) | 3167 |
| VD-270 | `list.windows` + R5c buộc biến kiểu; `vis run --stdin` | đã chốt (+41 mục) | 3215 |
| VD-271 | Mở chương trình VS3 | đã chốt (+4 mục) | 3660 |
| VD-272 | Lộ trình dài hạn hậu `v0.3.5` & cổng chặn cứng R0 Refactor | đã chốt | 3756 |
| VD-273 | Chốt Ngân sách mặc định (`VS3-1a`), 3 mã lỗi mới & Hook Huỷ HTTP (`VS3-1b`) | đã chốt | 3771 |
| VD-274 | Kênh `select` (`VS3-2c`): API hàm tĩnh trước, không đổi EBNF ở `v0.4.0` | đã chốt | 3780 |
| VD-275 | Chốt triển khai Worker Pool `P2b` + `P4` & cờ CLI `--workers` (`VS3-3b`) | đã chốt | 3790 |
| VD-276 | Đóng khe hở TOCTOU đường ghi Unix (`VS3-QĐ-4`, `#109b`/`#140`) & Landlock `#125` | đã chốt | 3800 |
| VD-277 | Tiêu chí công nhận Windows Tier 1 & WASI Runtime (`VS3-6`, `v0.4.5`) | đã chốt | 3809 |
| VD-278 | Phạm vi Gói, Ngôn ngữ Pha 2, ViShell & Plugin ABI tới `v1.0` | đã chốt | 3822 |
| VD-279 | Chuẩn hoá phân vai theo Quyền hạn (độc lập môi trường, mở rộng `VD-119`/`VD-137`) | đã chốt | 3835 |
| VD-280 | Duyệt đóng Giai đoạn 0 (`v0.3.6`) & Mở Giai đoạn 1 (`v0.4.0`) kèm 6 tinh chỉnh điều phối | đã chốt | 3852 |
| VD-281 | Duyệt `ST-290`/`ST-294`, Chèn Giai đoạn 1.5 (`v0.4.1` Tối ưu Repo & DevEx) & Tinh chỉnh GĐ 1–4 | đã chốt | 3869 |
| VD-282 | Duyệt `ST-293`/`ST-295`, Đóng 12 Nợ Sổ, Luật R7, Cơ chế Zero-Idle & Kế hoạch Chạy Đêm 22–23/11 | đã chốt | 3887 |
| VD-283 | Phê chuẩn `v0.4.0`/`v0.4.1`, Chốt `P2b-3/4 NO-GO` & WASI Runtime, Điều kiện Tag `v0.4.5` (`R35`), & Mở Zero-Idle GĐ 3 (`v0.5.0`) | đã chốt | 3917 |
| VD-284 | Phê chuẩn `v0.4.5` & `ST-300b/301/303`, Mở Chính thức Hỗ trợ macOS arm64 (`mps` Tier 1) & Điều phối GĐ 3 (`v0.5.0` — 3-OS Tier 1) | đã chốt | 3957 |
| VD-285 | Phê chuẩn Đợt tính năng GĐ 3–4, Mở Đợt Refactor Kiến trúc & Cấu trúc Kho `v0.5.5` (`ST-310a..ST-310f`) & Trình tự `v0.5.0 → v0.5.5 → v1.0.0-rc1` | đã chốt | 3984 |
| VD-286 | Phê chuẩn `v0.5.0` & `v0.5.5`, Đăng ký `#195` (VM O(n²)) & Mở Đợt Hoàn thiện Tiền-`v0.6.0` (`ST-311a..ST-311d`) | đã chốt | 4002 |
| VD-287 | Quy trình Điều độ Công việc theo Khung giờ DeepSeek (Cao điểm = Chạy nặng 0-LLM · Thấp điểm = Phát triển) | đã chốt | 4017 |
| VD-288 | Phê chuẩn `ST-311b/d/e/f`, Chốt `H4 = db67eab` (`0.6.0`), Tối ưu Quy trình Đóng băng/QA 3-OS & Chính sách Dọn Nhánh/Worktree | đã chốt | 4029 |
| VD-289 | Chuyển sang Chế độ Phát triển Kiến trúc Tối ưu (Hủy Kế hoạch Phát hành Cũ & Mở Đợt Tái kiến trúc `ST-312a..ST-313d`) | đã chốt | 4047 |
| VD-290 | Phê chuẩn Sóng 0–3 Tái kiến trúc 5 Tầng DAG (`ST-312a..ST-313d`), Chốt 5 Quyết định Gói Duyệt & Mở Lát Hậu-kiểm `ST-314a` | đã chốt | 4064 |
| VD-291 | Phê chuẩn `ST-314a` (`DONE`), Đồng bộ SPEC/Thiết kế, Đăng ký `#197` & Mở Chương trình Tối ưu – Cứng hoá Sóng 4–5 (`ST-315a..ST-316c`) | đã chốt | 4081 |
| VD-292 | Phê chuẩn Sóng 4 (`ST-315a..ST-315d` `DONE`), Phán quyết F1/F-R1, Đăng ký `#198`/`#199` & Tối ưu Kế hoạch Sóng 5–6 hướng Kiến trúc Sạch – Dễ Bảo trì – Dễ Mở rộng | đã chốt | 4096 |
| VD-293 | Phê chuẩn Sóng 5 (`ST-316a`/`ST-316b`/`ST-316d` `DONE`), Đăng ký `#200`/`#201` & Mở Sóng 6 (`ST-317a`/`ST-317c` song song, `ST-317b` chờ tệp) | đã chốt | 4118 |
| VD-294 | Phê chuẩn `ST-317a`/`ST-317c` `DONE`, Giữ `ST-317b` `READY` (P2b đang chạy), `ST-316c` `BLOCKED` & Đăng ký `#202` | đã chốt | 4135 |
| VD-295 | Phê chuẩn `ST-317b` `DONE`, Chốt Sóng 6 (`ST-317a`/`ST-317b`/`ST-317c`) `DONE`, Giữ `ST-316c` `BLOCKED` & Đăng ký `#203`/`#204` | đã chốt | 4147 |
| VD-296 | Mở Sóng 7 — Đóng nợ Correctness (`#202`/`#203`/`#204`) & R1 mềm (`ST-318a..d`) | đã chốt | 4159 |
| VD-297 | Mở đợt 2 Sóng 7 — R1 mềm (`ST-318e`/`ST-318f`/`ST-318g`) | đã chốt | 4172 |
| VD-298 | Mở đợt 3 Sóng 7 — `ST-318h` (R1 `vis-interp/frame.rs`) & `ST-318i` (rustdoc `vis-hir`) | đã chốt | 4184 |
| VD-299 | Phê chuẩn `ST-318a`/`ST-318h` `DONE`, Chốt Sóng 7 9/9, Đăng ký `#206`/`#207`/`#208` | đã chốt | 4195 |
| VD-300 | Mở Sóng 8 — Đóng nợ Parity/Rustdoc hậu Sóng 7 (`ST-319a`/`ST-319b`/`ST-319c`) | đã chốt | 4208 |
| VD-301 | Mở `ST-319d` (đóng `#208` theo thiết kế codegen) & Đăng ký `#210` | đã chốt | 4221 |
| VD-302 | Mở `ST-319e` — đóng `#210` (closure gọi hàm tên, interp non-frame) | đã chốt | 4232 |
| VD-303 | Phê chuẩn `ST-319d`/`ST-319e` `DONE`, Chốt Sóng 8 5/5, Đóng `#208`/`#210` | đã chốt | 4242 |
| VD-304 | Mở Sóng 9 nhỏ — `ST-320a` (`var self`+`push`) & `ST-320b` (frame env module) | đã chốt | 4253 |
| VD-305 | Chốt Sóng 9 (`ST-320a`/`ST-320b` `DONE`, gate 3940, 38 test), numbering nợ `#211`..`#215` theo R1, giữ `ST-316c` `BLOCKED` (chain exit=1) | đã chốt | 4263 |
| VD-306 | Chốt Sóng 10 — 4 merge fix (`ST-321a..d` `DONE`, HEAD `0b32c5dc`, gate 3942, 40 test), đóng `#200`/`#201` (sổ 82/114), giữ `ST-316c` `BLOCKED` chờ re-run full-gate | đã chốt | 4274 |
| VD-307 | Chốt bằng chứng 3-OS (`ps` gate 3942 + `psvn` peak3 + `mps` peak4), mở `ST-316c` `DONE` (sổ giữ 215 = 82/114; `ST-309` vẫn BLOCKED; không tag) | đã chốt | 4284 |

## Ghi chú

- `VD-01`..`VD-10`: bảng Phần I; `VD-11`..`VD-118`: bullet Phần III; `VD-119` trở đi: mục `###`.
- Nhiều mục cùng số (đặc biệt `VD-270`, `VD-271`) là **bổ sung/đính chính** cùng một quyết định;
  index giữ tiêu đề mục đầu và ghi số mục bổ sung trong cột trạng thái.
- Khoảng trống `VD-230`..`VD-240` không xuất hiện trong `DECISIONS.md`; không tự sinh nội dung.
| VD-308 | Chốt Sóng 12 (ST-323a/b/c/d) — A0, 2026-11-28 · 4284+ |
| VD-309 | Mở Sóng 13 (ST-324a/b/c/d) — A0, 2026-11-28 · 4284+ |
