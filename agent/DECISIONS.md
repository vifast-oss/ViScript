# DECISIONS — ViScript

Người ghi: **Agent 0**. Agent 2 **không** sửa tệp này; cần đổi thì ghi `BLOCKED.md` + đề xuất.
Nguồn: `docs/viscript/ViScript-thiet-ke.md` §21–§22 và `docs/QUYET-DINH-sau-review.md` (QD) §7.

---

## Phần I — VD-01..VD-10 (chép từ tài liệu ViScript §22)

| Mã | Nội dung |
|---|---|
| **VD-01** | ViScript có kiểu tĩnh bắt buộc, không `any`, không null |
| **VD-02** | Một ngôn ngữ, hai chế độ cú pháp, cùng **HIR**/kiểm kiểu/VM; ViShell là chương trình riêng |
| **VD-03** | Ngữ nghĩa giá trị + RC + copy-on-write; không GC truy vết; chu trình chỉ qua `ref`, phá bằng `weak` |
| **VD-04** | Lõi (syntax, types, bytecode, vm) không phụ thuộc hệ điều hành, build được wasm32 |
| **VD-05** | Mọi truy cập hệ thống qua capability; không hàm toàn cục chạm hệ thống |
| **VD-06** | Interpreter cây là mô hình tham chiếu, giữ mãi để kiểm vi sai VM |
| **VD-07** | Không JIT; không `eval` chuỗi trong chế độ quyền hạn chế |
| **VD-08** | Thay đổi SPEC chỉ qua Agent 0; mỗi tính năng mới phải có ví dụ + test trước |
| **VD-09** | Từ khoá tiếng Anh, định danh Unicode (NFC), thông báo lỗi vi/en/ja |
| **VD-10** | Giấy phép MIT OR Apache-2.0; phụ thuộc phải tương thích |

## Phần II — Q1..Q8 (tóm tắt 1 dòng/mục, kèm chỗ QD §7 đã sửa)

| Mã | Chốt | Ghi chú sau QD §7 |
|---|---|---|
| Q1 | Từ khoá **tiếng Anh**; định danh Unicode cho phép tiếng Việt; thông báo lỗi có tiếng Việt | giữ nguyên |
| Q2 | `int / int` cho `int`; có lint khi chia hai literal | giữ nguyên |
| Q3 | Thụt lề **4 dấu cách bắt buộc, cấm tab** | **sửa bởi VD-12**: đầu vào nhận thụt lề nhất quán bất kỳ (đơn vị suy từ cấp đầu), tab = 4 + VS0107, `vis fmt` luôn ra 4 |
| Q4 | Tên **ViScript** (`.vis`, lệnh `vis`), **ViShell** (`.vish`, lệnh `vish`) | giữ nguyên |
| Q5 | `xs[i]` ngoài phạm vi **panic**; `xs.get(i)` trả `T?` | **thêm bởi VD-18**: có `xs[i]?` trả `T?`; `xs[-1]` panic có thông điệp |
| Q6 | Giấy phép **MIT OR Apache-2.0** | giữ nguyên (= VD-10, D2-21) |
| Q7 | `decimal` **không** trong lõi, là thư viện | giữ nguyên |
| Q8 | Toán tử bit là **hàm** (`bit_or`…), không ký hiệu, để giải phóng `\|` cho pipe | **sửa bởi VD-15**: `\|` **không bao giờ** là toán tử nhị phân trong biểu thức |

## Phần III — VD-11..VD-22: chốt bổ sung từ QD §7 (mỗi dòng một mục)

- **VD-11** Chuẩn hoá: **toàn bộ tệp nguồn được chuẩn hoá NFC trước khi lex** — 'ố' gõ kiểu nào cũng là một scalar.
- **VD-12** Thụt lề: đầu vào nhận thụt lề nhất quán bất kỳ, đơn vị suy từ cấp đầu; đầu ra `vis fmt` luôn **4**. Tab: lexer báo cảnh báo-lỗi **VS0107**, coi tab = 4, **vẫn lex tiếp**; `vis check` coi là lỗi, `vis fmt` sửa.
- **VD-13** Phần trăm: **bỏ literal `50%`**, dùng `50pct`; `%` chỉ là modulo; job spec dùng `job 1`.
- **VD-14** Chú thích: script — `#` ngoài chuỗi luôn là chú thích; lệnh — `#` chỉ là chú thích khi **đứng đầu từ** (sau khoảng trắng hoặc đầu dòng), nên `https://x/y#frag` giữ nguyên.
- **VD-15** `|` **không bao giờ là toán tử nhị phân trong biểu thức**. Pattern: or-pattern dùng `|`; lệnh: pipe ở **độ sâu ngoặc 0**; `|>` là pipe script. (Sửa Q8.)
- **VD-16** Bảng ưu tiên chốt (cao → thấp): postfix (`. ?. () [] ? with`) → `**` → `-` một ngôi → `* / %` → `+ -` → `.. ..=` → `??` → so sánh (không kết hợp) → `not` → `and` → `or` → `|>`. `**` kết hợp **phải**, cao hơn `*`; mũ âm với int panic; có `checked_pow`.
- **VD-17** Float: NaN — `==` trả false; float **không** có `Eq/Ord/Hash`; kiểu chứa float không derive được `Eq/Hash`; dùng `total_cmp`. `percent` so sánh được với float theo tỉ lệ (`50pct == 0.5`); `ps.cpu` có kiểu `percent`.
- **VD-18** Chuỗi: **cấm `s[i]`**; dùng `.chars() .graphemes() .bytes() .byte_at(i)?`; có `byte_len()`, `char_count()`, `grapheme_count()`; so sánh theo dãy scalar. Index: thêm `xs[i]?` trả `T?`; `xs[-1]` panic có thông điệp.
- **VD-19** Optional: tự nâng `T → T?` ở gán, trả, đối số; `T??` là lỗi kiểu; `?` dùng được trên `T?` trong hàm trả `U?`; pattern `none`, `some(x)`, `ok(v)`, `err(e)`.
- **VD-20** Lỗi: `trait Error: Show { message, source, span }`; `AnyError` là kiểu xoá; mọi `E: Error` tự chuyển sang `AnyError` qua `?`; `main -> Result[(), AnyError]`; tên phía plugin Rust là `vis::Error`. Mã thoát: 0 ok, 1 lỗi từ main, 2 lỗi dùng/kiểu, 101 panic, 126 không chạy được, 127 không tìm thấy, 130 bị huỷ.
- **VD-21** Chế độ lệnh (QD §7.2): metachar là khoảng trắng, `( ) [ ] { } ; & $ ' " |`, `#` ở đầu từ; `"..."` có nội suy, `'...'` thô; `$x` biến phiên, `$env.NAME` môi trường; `;` tách pipeline; `&` nền chỉ khi là token cuối có khoảng trắng phía trước; `--` kết thúc cờ; `-5` là số nếu tham số kiểu số, ngược lại là cờ; `.vish`/REPL: `shell_item := use | let | fn | cmd | pipeline`.
- **VD-22** Parse chế độ lệnh **hai pha**: `vis-syntax` sinh `CmdCST` (từ trần chưa định kiểu) → `cmd_lower` trong `vis-types` dùng bảng chữ ký → **HIR**. Tiêu chí "cùng AST" đọc thành **"cùng HIR"**; vòng 1 chỉ làm `CmdCST` + golden, `cmd_lower` là task vòng 2.
- **VD-23** `docs/viscript/SPEC-lex.md` + **Phụ lục A** là nguồn từ vựng/cú pháp duy nhất; Agent 0 duyệt qua errata; ST-02a DONE sau APPROVED.
- **VD-24** **Shell sở hữu tty**; tiến trình con nhận pipe, không nhận `cap[Term]`; chương trình toàn màn hình để sau với tty server.
- **VD-25** `from json` không tham số trả `Json`; `where`/`get` trên `Json` là lỗi kiểu gợi ý `from json[T]`; SIGPIPE/`take` không phải lỗi; `| complete` trả `{stdout,stderr,code,how}`.
- **VD-26** Consent và cache dùng chung **hash closure** (nguồn + phụ thuộc + phiên bản compiler).

- **VD-27** Mã lỗi `VS` + 4 chữ số: `01xx` từ vựng, `02xx` cú pháp, `03xx` tên, `04xx` kiểu, `05xx` runtime. `VS0412` giữ nghĩa "kiểu không khớp" (thiết kế §9). ST-02b được sửa **bảng mã lỗi** trong `SPEC-lex.md`/phụ lục (không sửa quy tắc từ vựng); `AGENTS.md` §4.1 cập nhật cho khớp.

- **VD-28** `b"…"` **chỉ** có escape `\xNN` (SPEC-lex §1 hàng `BYTES`, §3 dòng 54). Mọi escape khác — kể cả `\\`, `\n`, `\u{…}` — là **VS0102**; muốn một byte `\` thì viết `\x5c`. Chốt câu hỏi Q4 trong `BLOCKED.md` §3 (thiết kế §2.4 xếp `b"…"` vào nhóm đủ bộ escape — SPEC-lex đứng trước). Có test âm ở `crates/vis-syntax/src/lex/lit.rs`.
- **VD-29** `_` **trong phần lẻ** của `FLOAT` theo luật chung của §2: chỉ được **giữa hai chữ số** ⇒ `1.2_3` hợp lệ, `1.2_` và `1.2__3` là **VS0104**. (Chốt Q5: §1 viết phần lẻ là `[0-9]+`; chọn cách đọc của §2 vì nó là luật phát biểu cho literal số nói chung.)
- **VD-30** `char` **không** có escape ở vòng 1 (đúng §1 "`'` scalar `'`" và §3 chỉ định nghĩa escape cho các dạng chuỗi) ⇒ `'\n'` là VS0105 và không viết được literal dấu nháy đơn. Xem lại ở vòng 2 nếu có nhu cầu (VD mới + test trước).
- **VD-31** Phiên giải kind khi §1 và §5–§8 lệch nhau (chốt Q3): `..`/`..=` là **`PUNCT`** (golden §8 gọi `RANGE`/`RANGE_EQ` là bí danh của cùng token); `$`, `^`, `set{` là **kind riêng** `DOLLAR`/`CARET`/`SET_LBRACE` (theo §5–§6); `#` **không bao giờ** là `PUNCT` (chú thích thắng); `##` chỉ là `DOC_COMMENT` ở đầu dòng. `test` là **`IDENT`** (từ khoá theo ngữ cảnh `test fn`): parser điện phân, không phải lexer.
- **VD-32** Ba ví dụ của SPEC-lex §8 là **văn bản minh hoạ**, không phải danh sách đầy đủ: #56 (`\tif`) và #58 (`(1 +\n 2)`) **lược** token ⇒ golden ghi danh sách **đầy đủ** (`!VS0107 INDENT KW(if)`, `( INT(1) + INT(2) )`) và chép nguyên văn ví dụ gốc trong `//`; #26 (`"{x" => !VS0108`) **đúng nguyên văn**, không có errata. Không dòng nào bị bỏ, không kỳ vọng nào bị hạ.
- **VD-33** Ba phụ thuộc Unicode của `vis-syntax` được duyệt theo allowlist QD §9.3: `unicode-normalization` (MIT OR Apache-2.0 AND Unicode-3.0) cho NFC toàn tệp (VD-11), `unicode-ident` (như trên) cho UAX #31, kéo theo `tinyvec` (Zlib OR Apache-2.0 OR MIT). Hệ quả: con số "0 phụ thuộc ngoài" trong `reports/R1-viscript.*` phải ghi lại ở báo cáo vòng sau.
- **VD-34** **30 golden biểu thức** của ST-05 do **Agent 0** soạn tại `viscript/test/expr/golden.expr` (kỳ vọng, **Agent 2 không sửa**: thấy sai thì báo). Định dạng: `<nguồn> => <sexp>` / `=> !ERR` / `=> !ERR "<chuỗi gợi ý>"`; sexp là **dạng ngoặc hoá của CST** (giữ nút `paren`, postfix `field/?field/call/index/try/with`, literal cấu trúc `list/set/map/record/tuple`), kết hợp trái **viết lồng trái** để phân biệt với kết hợp phải (`??`, `**`). Nguồn kỳ vọng: §4.5 (8 hệ quả ưu tiên bắt buộc), §A.5 (12 mức + kết hợp), §A.3 (văn phạm), và ca `50%` của L3 (`docs/DUYET-AGENT0-vong-va.md`). Bất biến kèm mọi ca: **CST lossless** — ghép CST theo thứ tự phải ra nguyên văn nguồn.
- **VD-35** **Mã lỗi cú pháp (02xx)** cho parser ST-05 nằm ở **sổ riêng** trong `vis-diag` (`SYNTAX_CODES`),
  **không** sửa `CODES` — sổ `CODES` vẫn đúng bằng bảng SPEC-lex §7 (10 mã 01xx) để test "CODES khớp §7"
  tiếp tục là hàng rào thật (không nới kỳ vọng cũ). Cấp 2 mã: `VS0201` = lỗi cú pháp chung (token không
  mong đợi/thiếu vế); `VS0202` = `%` dùng như phần trăm ⇒ thông điệp phải chứa `50pct` (L3 của
  `docs/DUYET-AGENT0-vong-va.md`, ca #30 của `viscript/test/expr/golden.expr`). `lookup`/`lookup_str` tra
  **cả hai** sổ, `vis explain` in được cả 02xx; số không tái dùng, đúng 4 chữ số. Mã mới khác phải qua Agent 0.
  Kèm theo: "mỗi tệp lỗi đúng **1** lỗi đầu" = parser báo lỗi đầu tiên rồi **dừng** ở cấp tệp (không báo dồn),
  không panic với mọi chuỗi token.
- **VD-36** **Bộ fixture tài liệu của ST-05** (chốt sau khi kiểm 22 khối ```vis trong `docs/viscript/ViScript-thiet-ke.md`):
  đúng **15 khối đủ thân** phải parse 0 lỗi + lossless; **7 khối loại trừ tường minh** (liệt kê `file:dòng` +
  lý do trong test, máy kiểm đúng con số này):
  - **6 khối có `...`** (dòng 312-315, 436, 450, 613, 663, 739) — QD §7.4 **cấm `...` trong fixture**, nên đây
    là ví dụ lược thân, không phải chương trình; không được thêm `...` vào văn phạm (VD-08).
  - **1 khối dòng 1438 dùng `assert`** — `assert` **không có** trong văn phạm §A.1/A.2; đây là lỗ hổng tài liệu
    (thiết kế dùng mà văn phạm không định nghĩa) ⇒ ghi vào báo cáo R2 để Agent 0/1 vá tài liệu, **không** tự thêm
    cú pháp.
  Khối ```vish-session là pipeline chế độ lệnh ⇒ thuộc ST-06. Nếu để parse một khối **đủ thân** mà phải thêm cú
  pháp ngoài §A.1/A.2 ⇒ dừng, báo Agent 0 (không tự mở rộng ngôn ngữ).
- **VD-37** **Năm chỗ Phụ lục A không tự chứa** khi đối chiếu với chính ví dụ trong tài liệu (chốt cho ST-05,
  mỗi chỗ phải có **test khoá**; đây là mức tối thiểu để tài liệu parse được, **không** mở thêm cú pháp):
  1. `file := (item | stmt)*` — §A.1 viết `item*` nhưng 4 khối tài liệu (dòng 290, 342, 535, 549) đặt **câu lệnh**
     ở cấp tệp; `.vis` là script nên câu lệnh cấp tệp là hợp lệ.
  2. `suite` **một dòng** dừng ở `else`/`elif` **cùng dòng** (`if loud: s.upper() else: s`), không đòi NEWLINE.
  3. Giá trị trường của `with` parse ở mức **`or_expr`** (dưới `|>`), để `u with age: 21 |> f` =
     `(u with age: 21) |> f` (golden #17, §4.5), không nuốt `|>`.
  4. `+=`/`-=` (§A.2) **chưa có token** trong SPEC-lex §1 ⇒ vòng 1 nhận **cặp `+` `=` / `-` `=` dính nhau**
     (span liền kề, không khoảng trắng); `+ =` có khoảng trắng **không** phải `+=`. Vòng 2: thêm token vào §1 +
     lexer (task riêng, không tự làm).
  5. `param := '...'` (§A.1, variadic) chưa có token `...` ⇒ **không** hỗ trợ vòng 1; khối duy nhất dùng nó
     (dòng 663) đã bị loại trừ theo VD-36.
  Kèm theo: `next_block()` trả **301** sau khi 02xx được cấp (VD-35) — test cũ cập nhật kèm lý do, không nới
  kỳ vọng nào khác. Phát hiện chỗ lệch **mới** ⇒ dừng, báo Agent 0 (không tự mở rộng ngôn ngữ, VD-08).
- **VD-38** **Sổ lỗ hổng đã biết, hoãn có ý thức** (ghi để không ai "phát hiện lại" rồi tưởng là hồi quy; mỗi mục
  phải vào báo cáo **R2** khi làm báo cáo vòng 2):
  1. pattern chưa nhận `dotted_path` (`Color.Red`) — §A.4; 2. `lvalue` chưa nhận postfix cho `+=` (`a.b += 1`);
  3. `weak`/`cap` **chưa** là từ khoá ở SPEC-lex §1 nên `cap[Dir]` parse thành type-application (ảnh hưởng ST-06);
  4. tài liệu dùng `Entry(trường: …)` (dòng 1429) và `assert` (dòng 1438) — **không** có trong §A.1/§A.2;
  5. 6 khối tài liệu có `...` (dòng 312-315/436/450/613/663/739) — QD §7.4 cấm `...` trong fixture;
  6. §A.1 có `param := '...'` (variadic) nhưng SPEC-lex §1 **không có** token `...`;
  7. `+=`/`-=` có ở §A.2 nhưng **không có token** ở §1 (vòng 1 nhận cặp dính nhau, VD-37);
  8. "mỗi tệp đúng **1** lỗi đầu" là quy tắc **tầng parser**: tệp có lỗi từ vựng vẫn ra 2 chẩn đoán
     (`VS0101` + `VS0201`) — lọc theo tầng là việc của `vis check` (ST-08).
  Không mục nào chặn ST-06/07/08. Muốn vá ⇒ task riêng + test đỏ trước (VD-08).
- **VD-39** **Phạm vi `vis fmt` vòng 1** (ST-07). Chỉ chuẩn hoá **thụt lề**, không định dạng lại trong dòng:
  mỗi cấp khối = đúng **4 dấu cách** (suy từ cấu trúc CST, không đếm theo tệp gốc); **tab → 4 dấu cách**
  (VS0107); bỏ **khoảng trắng cuối dòng**; bảo đảm **đúng một** newline cuối tệp; **giữ nguyên văn mọi thứ
  trong dòng** (khoảng trắng giữa token, chú thích, thứ tự token); ghi ra **NFC** (vì `LexOutput.text` đã NFC).
  Hệ quả là 4 tiêu chí §18.4 đo được: (a) **lossless** trên tệp "đã đúng chuẩn" — định nghĩa: tệp là **điểm
  bất động** của `fmt`; (b) **idempotent**; (c) sửa tab; (d) tệp **lỗi cú pháp**: không sửa, báo lỗi, mã thoát
  **2**. Định dạng lại biểu thức/danh sách/ngắt dòng dài là **vòng 2** (không làm ở ST-07).
- **VD-40** **Chế độ lệnh vòng 1 (ST-06) — 5 chỗ §A.7/§9.3 cần chốt** (kèm 22 golden ở
  `viscript/test/cmd/golden.cmd`, Agent 0 duyệt trước khi hiện thực):
  1. **`;` và `&`**: §A.7 không có luật sinh, nhưng §9.3 ("`;` tách pipeline") và §9.4 hàng 10 quy định
     nghĩa. Chốt: nội dung **một dòng** là `(items <pipeline> …)`; `&` là **thuộc tính của pipeline**
     (`pipeline := stage ('|' stage)* ['&']`) ⇒ nút `(bg (pipe …))`; `&` **không** ở cuối dòng = lỗi;
     `;` không có vế sau = lỗi. **Tệp** `.vish` = `(shell <item>*)`, mỗi item một dòng.
  2. **`-5`**: §A.7 `flag := '--' IDENT ['=' expr] | '-' IDENT ['=' expr] | '--'` **không** cho chữ số sau
     `-`; §9.3 nói `-5` là số nếu kiểu tham số là số (quyết ở `cmd_lower`). Chốt: parser giữ **`(word -5)`**
     (từ trần chưa định kiểu), **không** nới `flag`; `cmd_lower` (vòng 2, VD-22) phân biệt số/cờ.
  3. **`[ab]` glob char-class**: `[` `]` là **metachar** ở §9.3 nên lexer cắt thành 3 token (ST-04 đã DONE,
     **không** sửa lexer). Chốt: parser **ghép** cụm `[` … `]` **liền kề** (span liền, không khoảng trắng)
     ở vị trí từ thành **một** nút `(word [ab])`; có khoảng trắng bên trong = lỗi; list literal chỉ trong
     `( … )` (chế độ biểu thức).
  4. **`--`**: theo §A.7 `'--'` là một dạng `flag` ⇒ nút `(flag --)`; `cmd_lower` phải hiểu đây là **kết thúc
     cờ** (mọi token sau là từ trần).
  5. **`|>` ở chế độ lệnh**: không phải pipe (§9.4 hàng 2 ghi "—") ⇒ **lỗi**; pipe chỉ là `|` ở độ sâu 0.
  Không chỗ nào ở trên mở rộng ngôn ngữ ngoài §9.3/§9.4; `cmd_lower`/`vis-types` vẫn là vòng 2 (VD-22).
- **VD-41** **Đính chính 3 ca của bộ golden chế độ lệnh** (sau khi hiện thực pha B va vào 2 khối
  ```vish-session của tài liệu — DoD của ST-06):
  1. **`>` `<` `>=` `<=` `==` `!=` ngoài `( … )` KHÔNG phải lỗi và KHÔNG phải redirect**: §9.4 hàng 3 viết
     "so sánh trong tham số biểu thức; **không** redirect" — nghĩa là `>` **không bao giờ** là redirect, và
     2 khối tài liệu dùng `where size > 5mb`, `where age >= 18` **không ngoặc**. Chốt: CmdCST chưa định kiểu
     giữ chúng là nút từ `(word >)`; `cmd_lower` (vòng 2) quyết nghĩa theo chữ ký lệnh; ghi tệp **chỉ** bằng
     `| save`. Ca #13 đổi từ `!ERR` thành ca hợp lệ. (Bài học: `>` redirect kiểu POSIX **không** có trong ngôn ngữ.)
  2. **`^` không ở đầu stage = LỖI**: lexer ST-04 đã bắt bằng `VS0110` (§5/§6) ⇒ ca #22 `ls ^x` là `!ERR`, parser
     không có cơ hội thấy token `^` ở giữa. Không sửa lexer.
  3. **Thêm ca #23** (tổng **23**): `.vish` nhiều dòng `let out = ls | grep foo` + `pwd` ⇒
     `(shell (let out (pipe …)) (pipe …))`, khoá nút `(shell …)` và `(let …)` (VD-40 #1) — vì `vis check` nay
     kiểm được `.vish` và 2 khối ```vish-session là DoD của ST-06.
  Kèm theo (đã ghi `JOURNAL`): `tests/check.rs` của ST-08 đổi ca `.vish` từ `Bail out!` sang "kiểm như tệp lệnh"
  — **có lý do**, không nới kỳ vọng; log đỏ ST-08 giữ nguyên sha256 của harness **thời điểm đó** (là bản ghi lịch
  sử, không phải bằng chứng cho cây hiện tại).
- **VD-42** **Sổ mã lỗi vòng 2** (theo VD-27: `03xx` tên, `04xx` kiểu, `05xx` runtime) — vẫn là **sổ riêng** trong
  `vis-diag` (`NAME_CODES`, `TYPE_CODES`, `RUNTIME_CODES`), **không** sửa `CODES`/`SYNTAX_CODES`. Cấp tối thiểu:
  `VS0301` tên không tồn tại · `VS0302` khai báo trùng; `VS0412` **kiểu không khớp** (giữ đúng nghĩa §9) ·
  `VS0401` sai số/thứ tự đối số · `VS0402` `match` không xét hết · `VS0403` trường/kiểu không tồn tại;
  `VS0501` lỗi runtime chung · `VS0502` chia cho 0 · `VS0503` chỉ số ngoài phạm vi. `lookup`/`lookup_str`/
  `vis explain` tra **cả 5 sổ**; `next_block()` ⇒ **601**; mỗi mã mới phải có `message_vi/en/ja` + snapshot.
- **VD-43** **Phạm vi ngữ nghĩa VS0** (chốt để ST-11..ST-15 có hợp đồng; thiết kế §3–§6 là nguồn, chỗ nào
  thiết kế không nói rõ thì **hỏi Agent 0**, không tự chọn):
  1. **Kiểu**: `int` (i64 có dấu), `size`, `duration`, `percent`, `str`, `bool`, `bytes`, `path`, `char`,
     `list[T]`, `map[K,V]`, bản ghi có tên, enum (biến thể có/không đối số), `T?` (Option), `Result[T,E]`,
     hàm `(A, B) -> R`, `unit`. **Không generic** (VS1).
  2. **Số học**: `+ -` chỉ **cùng kiểu/đơn vị** (`size + duration` = `VS0412`); `size * int`, `size / int`,
     `duration * int`, `duration / int`; `percent` chỉ nhân với `size`/`duration` (`size * percent` ⇒ `size`);
     trộn `int` với đơn vị = `VS0412`; chia/lấy dư cho 0 = `VS0502`; **tràn i64 = `VS0501`** (không wrap).
     Toán tử số học áp lên `int` là `+ - * / % **`; `**` mũ âm trên `int` = `VS0501`.
  3. **So sánh**: chỉ cùng kiểu (số có thể cùng đơn vị); `==`/`!=` cho mọi kiểu **trừ hàm**. Không so sánh
     chuỗi với số.
  4. **Logic**: `and`/`or` ngắn mạch, hai vế `bool`; `not` nhận `bool`; `??` trả kiểu **bỏ Option** của vế trái,
     vế phải cùng kiểu đó.
  5. **`match`**: phải xét hết ⇒ `VS0402` nếu thiếu; được `else`.
  6. **Bản ghi**: bất biến, tạo `{ten: …}`, cập nhật bằng `with`; trường lạ = `VS0403`.
  7. **`print(x)`** in `str(x)` + `\n` ra **stdout**; `str` của list/map/bản ghi/enum/Option/Result theo **một**
     định dạng cố định (ghi vào `JOURNAL` + snapshot để ST-15 dùng).
  8. **Lỗi runtime**: in `<tệp>:<dòng>:<cột>: lỗi VS05xx: …` ra **stderr**, exit **1**; lỗi kiểu/tên ⇒ exit **2**
     (như `vis check`).
- **VD-44** **Hạ CST→HIR phải PERMISSIVE** (ST-11): HIR biểu diễn **mọi** thứ trong Phụ lục A ở mức cú pháp —
  `float`, `cap[Dir]`, `ref`, `async fn`, generic `fn f[T]`, `derives`, `scope s`, `cmd { … }`, `await` — **không**
  báo "tính năng chưa hỗ trợ" ở tầng hạ; giới hạn VS0 chỉ báo ở **kiểm kiểu (ST-12)**. Hệ quả: 15 khối tài liệu
  hạ **0 chẩn đoán**. `Mode::Command` vào `lower` ⇒ lỗi **API** tường minh (VD-22), không panic; không
  `unwrap/todo!` trên dữ liệu người dùng. ST-11 **không** dựng prelude: chỉ 8 tên giá trị builtin (`print`, `len`,
  `str`, `Ok`, `Err`, `Some`, `None`, `ref`) khoá bằng test; `true`/`false` là literal.
- **VD-45** **Chính sách giải tên** (ST-11): `NamePolicy::Permissive` là **mặc định** — dùng tên tự do được phép
  (fragment như 30 golden, khối `match s:`), `VS0301` **chỉ** khi **gán** cho tên chưa khai (báo ở **gốc lvalue**,
  đúng **một** lần cho cả hai policy); `VS0302` khi khai lại **cùng** khung. `NamePolicy::Strict` (mọi lần dùng
  phải phân giải) có test riêng và sẽ là mặc định từ **ST-13/14** cho chương trình đầy đủ. Tên kiểu **đã khai
  trong tệp** ⇒ `Record/Enum`; **chưa khai** ⇒ `TypeRef` (ST-12 phân giải + báo `VS0301` cho kiểu lạ). `HirOutput`
  có 5 trường (`hir, diagnostics, symbols, types, error`). Thứ tự hạ `let`: **kiểu → init → khai `pat`** (§4.1).
- **VD-46** **Chốt 3 câu hỏi của ST-12** (kiểm kiểu VS0):
  1. **Gán cho `let`/tham số** (§4.1 cấm) dùng **`VS0412`** + thông điệp "chỉ `var` mới gán được" — đây là lỗi
     bất biến/kiểu, **không** cấp mã `04xx` mới và **không** mượn `05xx`; nếu sau này cần mã riêng thì cấp sau
     (VD mới).
  2. **Kiểu tham số lambda** để `{error}` ở VS0: văn phạm §A.3 **không** cho `(x: int) =>` nên parser không mở;
     **suy kiểu lambda hai chiều (§3.4) hoãn** ⇒ ghi "lỗ hổng đã biết" cho ST-14/R2.
  3. **Pattern bản ghi trần** `{name, age}` mất `sym` trong HIR ⇒ chỉ dùng được cho xét hết ở VS0; thêm `sym` là
     việc của **ST-14** (hoặc task riêng), không làm trong ST-12.
  Kèm: chẩn đoán của tầng tyck có `line/col = 0` (chỉ có HIR, không có văn bản) — **chấp nhận**, nhưng **ST-14
  bắt buộc** in `line`/`col` thật từ `span` + văn bản nguồn; sửa 2 test cũ (`snapshot_6_ca_so_ten_03xx`,
  `explain_so_ten_03xx`) là **bắt buộc** vì VD-42 đã cấp sổ `04xx` (không phải nới kỳ vọng).
- **VD-47** **Ngữ nghĩa VS0 chốt ở ST-13**: (1) **khối cho giá trị của câu lệnh cuối** — `Expr`, `If`, `Match`, khối
  con đều lấy giá trị; `if` **không** `else` ⇒ `unit`; vòng lặp **không** rò giá trị lần lặp cuối; tyck theo **cùng**
  luật (§4, §6). (2) `Show`: `size`/`duration` in **đơn vị lớn nhất chia hết** (`5000`⇒`5kb`, `1e9 ns`⇒`1s`, không
  chia hết ⇒ gốc); `percent` = `ratio*100` + `pct` bỏ `.0`; `record` ⇒ `Ten(truong: gt)`, `enum` ⇒ `Bien(arg)`,
  `opt` ⇒ `some(x)`/`none`, `Result` ⇒ `Ok/Err`, `char` ⇒ `'a'`, `bytes` ⇒ `b"…"`, `fn` ⇒ `<fn …>` (khoá snapshot).
  (3) `percent * size|int` và `percent * duration` **cắt về 0** (quyết định, không phải mặc định ngầm). (4) **Bảng
  phương thức lõi** khoá bằng test tyck (tên lạ ⇒ `VS0403`): `list` `len get first last push map filter each sum
  join`; `str` `len upper lower split chars contains`; `map` `len get has keys values`. (5) **Nội suy `{expr}` trong
  `MULTI_STRING` hoãn tới ST-14** (HIR phải mang phần nội suy; hiện in nguyên văn).
- **VD-48** **Phạm vi kiểu VS0 (chốt sau review ST-12)**: `AnyError`, `cap`, `ref`, `Json` **không** thuộc VS0 —
  vẫn báo `VS0301` nhưng **thông điệp phải nêu rõ** "không có ở VS0 (thuộc vis-std/vòng 2)". Toán tử VS0:
  `size % int`/`duration % int` ⇒ **giữ đơn vị**; `percent / int` ⇒ `percent`; `percent / percent` ⇒ **`float`**;
  `percent + percent` ⇒ `percent`; còn lại như VD-43. Luật `?` (§4.4): `T?` chỉ khi hàm trả `U?`, `Result[T,E]` chỉ
  khi hàm trả `Result[U,F]` (ràng buộc `E: Error` bỏ qua ở VS0, ghi JOURNAL). `int→float` **chỉ** cho **literal**
  số nguyên (§3.5). `--` chỉ dùng cho tham số **khai `--`** (`VS0401` nếu sai, tên trùng cũng `VS0401`).
  `unit` là kiểu **hữu hạn** khi xét hết `match` (pattern `()`); tuple theo §4.3 nếu mở.
  **Bổ sung (sau ST-12b):** so sánh `percent` ↔ `float` là **hợp lệ về kiểu** (`50pct == 0.6` ⇒ `false` lúc chạy,
  **không** `VS0412` — chỉ khác **giá trị**, không phải lỗi kiểu); ràng buộc `E: Error` của `?` trên `Result`
  **bỏ qua ở VS0** (làm khi có `vis-std`); 4 test cũ đã sửa kèm lý do (3 do VD-42 + 1 kỳ vọng `?` sai ở ST-12).
- **VD-49** **Chốt sau review ST-13** (ST-13b): (1) thêm `Type::Never` — `return`, nhánh `if`/`match` toàn phân kỳ
  cho `Never`, `coerce`/`unify_branch` bỏ qua nó. (2) `..=` với `i64::MAX`: **dừng dải** (không panic, không lỗi);
  dùng `checked_add`. (3) **thân vòng lặp bỏ giá trị** câu lệnh cuối (`for i in 0..1: 1` hợp lệ); `scope` **cho
  giá trị khối** (thống nhất tyck/interp). (4) `for c in "str"` ⇒ `char`; **range ở VS0 chỉ `int`** (`size/duration/
  char` ⇒ `VS0412`). (5) `DUR_UNITS` có **`w`**; `Show` escape ký tự điều khiển; đơn vị **decimal trước binary**
  (`1024kb`). (6) **keyword được dùng sau `.`** (`m.has(..)`, `xs.len()`), sửa ở parser — không đổi tên phương thức.
  (7) `break`/`continue` ngoài vòng lặp, `?` ngoài hàm ⇒ **`VS0501`** (mức runtime).
- **VD-50** **Chốt ST-14**: nội suy `{expr}` nằm ở **HIR** (`HExpr::Interp(Vec<InterpPart>)`), hạ **trong cùng
  ngữ cảnh giải tên** — **không** parse lại lúc chạy; `{{`/`}}` là literal. `vis run` exit **0/1/2** (0 xong,
  1 lỗi runtime `VS05xx`, 2 lỗi cú pháp/tên/kiểu); `--tap` in TAP ra **stderr** (giữ stdout sạch); `.vish` ⇒
  `Bail out!` (chế độ lệnh là VS1). Harness `test/sem/**` chạy **qua binary** `vis run`, so **byte** + `steps > 0`,
  và **khoá cứng số ca + số nhóm** (thêm/bớt ca phải sửa cả khoá — chống âm thầm bỏ ca).














## Phần IV — Duyệt

- Mẫu: `APPROVED <YYYY-MM-DD> Agent 0`, ghi ngay dưới mục **ST-02a** trong `viscript/agent/QUEUE.md`; nếu duyệt làm đổi nội dung SPEC thì thêm một mục VD mới ở tệp này. Không duyệt bằng chat.
- `ST-02a` = **APPROVED 2026-11-12 Agent 0** / DONE: `SPEC-lex.md` + EBNF + bảng ưu tiên + 60 golden token + 30 golden biểu thức; **chỉ Agent 0** sửa sau duyệt (D2-22, VD-08).
- Cổng "mọi khối mã trong tài liệu parse được" là điều kiện duyệt; ngoại lệ phải khai báo trong SPEC (QD §7.4).

## Phần V — Bảng nguồn cho các con số đang có hiệu lực

| Số | Giá trị | Nguồn |
|---|---|---|
| golden token (ST-02a) | **60** | QD §7.5 |
| golden biểu thức (ST-02a) | **30** | QD §7.5 |
| mã lỗi | `VS` + **4 chữ số**, quy tắc cấp (ST-02b) | QD §7.5 |
| thụt lề ra của `vis fmt` | **4** | VD-12 |
| giới hạn tệp | **1.500** dòng/tệp | QD §3.15 áp cho cả hai dòng |
| ca mơ hồ chế độ lệnh (ST-06) | **10** + mỗi hàng bảng 9.4 | QD §7.5, B §10.2 |
| fuzz lexer | **10 phút** | QD §7.5 (ST-04) |
| snapshot `vis-diag` (ST-03) | **20** ca | QD §7.5 |

- **VD-51** **Closure chụp theo snapshot (VS0)** — ST-15b chọn: `HExpr::Lambda` chụp **bản sao** các biến đang sống
  lúc tạo; biến `var` cập nhật **sau** khi tạo closure **không** được closure thấy (nhất quán với ngữ nghĩa giá trị
  của §7.1). Ngữ nghĩa **tham chiếu** (closure thấy giá trị mới) cần env chuỗi frame ⇒ task riêng, đã ghi JOURNAL.
  Kèm: `push` trên `var` **ghi lại lvalue** (CoW); trên `let`/tạm trả list mới (không no-op im lặng). `{x, ..}` hợp
  lệ (§4.3), thiếu trường **không** có `..` ⇒ `VS0402`. Sửa **1 kỳ vọng cũ** `tests/exhaustive.rs` (ca `{name, ..}`
  đang khoá hành vi **trái §4.3**) — ca thứ **5** bị sửa trong dòng ViScript, có comment lý do + ca âm.

- **VD-52** **Phạm vi vòng 3 (khung VS1)** — chốt trước khi làm: (1) **bytecode** tự chứa, có **magic `VBC1`** +
  phiên bản u16 + hash nguồn; `vis-bytecode` **không** phụ thuộc OS, **không** `unsafe`; (2) **VM an toàn** (Rust
  safe, không `unsafe` ở vòng này — thiết kế cho phép `unsafe` trong `vis-vm` nhưng **chưa cần**), stack + frame
  tường minh, giới hạn độ sâu như interp (VD-49: 2.000); (3) **verifier chạy cả bytecode từ cache** (thiết kế
  §14.5), từ chối: magic/phiên bản sai, đích nhảy ngoài, thanh ghi/frame sai, hàm thiếu, hằng số lệch; (4) **cache**
  ở `<dir>/vis/<hash>.vbc`, hash = **hash closure** (nguồn + phụ thuộc + phiên bản compiler) — nạp **luôn** qua
  verifier; (5) **differential** phải so **stdout + mã thoát + mã chẩn đoán**, sinh chương trình **có seed** trong
  subset VS0 (không sinh tính năng VS1b); (6) **mặc định vẫn chạy interp**, `vis run --vm` chọn VM cho tới khi
  differential 100.000 ca xanh (khi đó mới bàn đổi mặc định); (7) **không** làm generic/trait/module/`vis-std`/
  `vis-host-posix` ở vòng này — ghi rõ ở R3 là **VS1b**.

- **VD-53** **Chế độ lệnh chạy được (ST-18)**: (1) **khe ống** = tham số **vị trí đầu tiên theo thứ tự khai báo** của
  stage sau khớp **kiểu ra** của stage trước **và** không được ghi tường minh; khe **giữ chỗ trước** (đối số tường
  minh không chiếm mất); không có khe ⇒ `VS0603`. (2) **fail-fast**: lỗi giữa pipeline dừng, không chạy stage sau.
  (3) `;` ⇒ items tuần tự, `&` cuối dòng / `$x` / `$env` / `^prog` ⇒ **`VS0604`** "chưa hỗ trợ ở VS0" (không dùng
  `VS0602`). (4) **Sổ mã thứ sáu `COMMAND_CODES`**: `VS0601` lệnh không tồn tại · `VS0602` sai đối số/kiểu ·
  `VS0603` pipeline không nối được · `VS0604` tính năng chưa hỗ trợ ở VS0 ⇒ `BOOKS` 6, `all_codes()` **25**,
  `next_block()` **701**. (5) **3 builtin**: `echo(a, b="", c="", --upper:bool=false)`, `lines(s)`, `take(xs, n)`;
  **không variadic** (`echo` ≤ 3 phần) và **chưa** có `--x=v` — hạn chế đã ghi sổ. (6) **Tên lạ KHÔNG** bị coi là
  "chương trình ngoài" ở vòng này (giữ `VS0601`) để `vis check` còn bắt lỗi gõ; `command := … | chuong_trinh_ngoai`
  (§9.3) là việc của **VS1b** cùng `vis-host-posix`. (7) Hai khối `vish-session` §9.7 (dùng `ls/where/sort_by/take`,
  `walk/ps/to json/save`) nằm trong `doc-examples-allow.txt` (**9 mục**) vì là lệnh shell/vis-std VS1.

- **VD-54** **Bytecode VBC1 (ST-19)**: (1) hash nguồn 32 byte = **FNV-1a×4** (không thêm dep) — **không** phải SHA-256;
  doc ghi rõ "chỉ là khoá cache"; **khi làm consent/quyền (§11.3) phải thay bằng SHA-256 thật** (`sha2`, cần duyệt
  allowlist); hàng rào bù: **mọi** bytecode nạp từ cache **luôn** qua verifier (VD-52 #3). (2) `max_stack` chỉ là
  **ước lượng trên** (mô phỏng tuần tự, kẹp 0) — **ST-21 kiểm lại độc lập từ byte stream** và có quyền từ chối.
  (3) `match` hạ thành **chuỗi test + nhảy**; VM **không** biết pattern. (4) `Bin`/`Un` mang **enum của `vis-types`**;
  VM phải **gọi chung** bảng ngữ nghĩa của `vis-interp` (không chép ngữ nghĩa toán tử/`Show`/đơn vị vào bytecode).
  (5) `ref`/`await`/`cmd {}` (VS1) ⇒ `CodegenError::Unsupported` nêu tên + span, **không** cắt ca.

- **VD-55** **VM dùng chung ngữ nghĩa (ST-20/20b)**: (1) `vis-vm` **không** chép ngữ nghĩa — gọi `vis-interp`
  (`ops::{bin,un}`, `builtin::{call,method}`, `value::{show,eq,cmp,iter_items,lit_value,exit_code}`); `Interp` chỉ còn
  là vỏ mỏng. (2) VM dùng **`Vec<Value>` động**, **không** lấy `max_stack` làm giới hạn; `StackUnderflow` ⇒
  `VS0501` "bytecode hỏng"; cap khung **2.000** + cap **10^7 lệnh** như interp. (3) Differential so **`stdout` +
  `exit` + `(mã,dòng,cột)`**, **không** so `steps` (interp đếm bước ngữ nghĩa, VM đếm lệnh — ghi rõ trong doc).
  (4) **19 ca lệch đầu tiên** được khoá trong `KNOWN_LECH` (hai chiều: lệch mới ⇒ đỏ, sửa mà không cập nhật ⇒ đỏ);
  ST-20b đưa về **0 lệch** và để `KNOWN_LECH` **rỗng** — giữ test khoá "rỗng" để không ai thêm lại. (5) Codegen
  được sửa **theo differential** (bind pattern + thứ tự bind→guard, `MakeEnum` cho `E.A`, closure tham số trước
  biến bắt, `CallValue`, `?.` phẳng, `T?` tự nâng) — đây là **hoàn thiện codegen**, không đổi ngữ nghĩa; VBC1 vẫn
  **version 1** vì chưa ghi cache.

- **VD-56** **Chốt VS1-khung (ST-21..ST-23)**: (1) verifier **kiểm lại từ byte stream** và codegen dùng **chính**
  hàm mô phỏng ⇒ `max_stack` khai **bằng** `need`; 13 `VerifyError`; `load_verified` là **cửa duy nhất** để có
  `Module` từ byte (cache cũng đi qua đây). (2) Differential: **cổng `--fast` chạy 1.000** chương trình sinh tự
  động (~1,3 s); **100.000** do `bin/vis-diff` và **ghi log thật** (`st22-diff-100k.log`) — **không** tuyên bố cổng
  chạy 100k. (3) Cache: `$XDG_CACHE_HOME/vifast/vis/<hash>.vbc`, khoá FNV-1a×4 trên (phiên bản compiler, VBC1,
  chế độ, **toàn văn nguồn**), ghi tệp tạm + `rename`, hit thì **bỏ qua frontend**; hỏng/lệch ⇒ biên dịch lại.
  (4) **`vis run` mặc định vẫn INTERP**, cache áp cho đường `--vm` — đổi mặc định sang VM **chưa** làm vì sẽ đổi
  `steps` trong TAP (đơn vị lệnh vs bước ngữ nghĩa); đó là quyết định của vòng sau, có đo lại.
  (5) **Chưa** có generic/trait/module/`vis-std`/`vis-host-posix` ⇒ **VS1b** (ghi rõ trong R3).

- **VD-57** **Phạm vi vòng 4 (VS1b)**: (1) **module** = **mỗi tệp một module** (§7.10); `use ./util` **đường dẫn tương
  đối**, `use std.x` cho thư viện chuẩn, `use pkg.x` **hoãn** (cần registry/`deps`, VS2); **cấm vòng** phụ thuộc;
  mã lỗi khối **07xx**: `VS0701` không tìm thấy module · `VS0702` vòng phụ thuộc · `VS0703` `use` trùng/khoá mơ hồ;
  `next_block()` ⇒ **801**; hash closure (**VD-54**) **gồm mọi phụ thuộc** (sửa file `./util` ⇒ cache mất hiệu lực).
  (2) **`vis-std`** ở vòng này = **prelude trong Rust** (`std.str`, `std.list`, `std.map`, `std.path`, `std.time` mức
  tối thiểu) + `use std.x` phân giải tĩnh; **chưa** có gói ngoài. (3) **`vis-host-posix`**: tên lệnh **lạ** trong
  `.vish` = **chương trình ngoài** (§9.3) ⇒ `VS0601` **chỉ còn** khi `vis check --no-extern`; chương trình ngoài
  chạy thật, **stdout dạng `str`/`list[str]`**, mã thoát, **quyền**: mặc định chạy được nhưng ghi vào nhật ký quyền
  (`vis run --explain-perms`), **consent thật** ⇒ VS2. (4) **generic**: hàm + kiểu `[T]`, **đơn hình hoá**
  (monomorphize) ở codegen — chưa có constraint; **trait** ở ST-29 chỉ `derives Eq/Show/Json` + `impl` tối thiểu.
  (5) **SHA-256** thay FNV cho hash closure/consent khi `sha2` được duyệt allowlist (VD-54 điều kiện (1)).
  (6) **`vis test [đường dẫn]`** chạy bộ `test/sem/**` và in TAP, dùng **cùng** harness đã khoá số ca.

- **VD-58** **Module (ST-25)**: (1) **mỗi tệp một module**; `use ./x` tương đối (thử `.vish` trước nếu tệp chính là
  `.vish`), `./x` và `./x.vis` là **một** module; `..` cho phép nhưng `canonicalize`; `use std.*` ⇒ `VS0701` (ST-26),
  `use pkg.*` ⇒ `VS0701` (VS2). (2) **Cấm vòng**; DFS **hậu duyệt**, mỗi tệp một lần, tệp chính cuối ⇒ item hợp nhất
  tất định. (3) **Tầm nhìn**: chỉ item **`pub`** mới dùng được từ module khác **và** tệp phải `use` module đó — kiểm
  ở **giải tên (`lower`)**, không ở `tyck`; sai ⇒ `VS0301` nêu tên module. `pub` cho `let`/`const` **chưa** có (nợ).
  (4) `lower_into` **append** vào bảng chung ⇒ `SymbolId`/`RecId`/`EnumId` **ổn định** qua các tệp, không remap.
  (5) Sổ `07xx`: `VS0701` không thấy module · `VS0702` vòng · `VS0703` `use` trùng/mơ hồ ⇒ `BOOKS` 7,
  `all_codes()` 28, `next_block()` **801**. (6) Hash closure (VD-54) = FNV-1a×4 trên `"vifast-vis-cache-v2\n"` +
  compiler + VBC1 + mode + **Σ(đường dẫn tương đối + nội dung)** theo hậu duyệt ⇒ sửa phụ thuộc là **mất cache**.

- **VD-59** **vis-std lõi (ST-26)**: (1) tên **đầy đủ theo module** (`str.split`, `path.join`, `time.now_ms`,
  `json.encode`) — không rò tên trần, không đụng `join`/`take` của VS0. (2) **Một hiện thực** ở `vis-interp`
  (`stdlib.rs`), `builtin::call` chuyển tiếp, **VM gọi cùng đường** (VD-55) ⇒ differential 70/70 là bằng chứng.
  (3) `std.list.*`/`std.map.*` nhận **`Any`** (tyck chỉ kiểm số đối số; sai kiểu ⇒ `VS0501` nêu hàm + kiểu nhận) —
  **nợ VS1c**: tổng quát hoá `list[T]`/`map[K,V]` khi có generic. (4) `sort_by` = **closure khoá**, tăng dần,
  **ổn định**; `--desc` = nợ. (5) `json.encode`/`encode_pretty` (map theo **thứ tự chèn**, bản ghi theo thứ tự
  trường, `none` ⇒ `null`, `Result` ⇒ `{"ok":…}`/`{"err":…}`, enum ⇒ `{"variant":…,"value":…}`); `json.decode`
  **không** export, gọi ⇒ `VS0301` **"chưa hỗ trợ (VS1b)"** (bảng `HA_CHUA_HO_TRO`, **không** cấp sổ mới).
  (6) `std.time.*` đọc đồng hồ trong Rust (không thêm `Host`), `std.path.*` **thuần chuỗi**, `std.fs` ⇒ ST-27.

- **VD-60** **Chương trình ngoài (ST-27)**: (1) tên lệnh **lạ** trong `.vish` = **chương trình ngoài** (§9.3); chạy
  bằng `std::process::Command` (**không** shell, đối số mảng, env kế thừa, **cwd** hiện tại); `stdout` ⇒ `list[str]`
  (tách `\n`, bỏ dòng rỗng cuối) hoặc `--raw` ⇒ `str` nguyên văn; `stderr` ⇒ **chẩn đoán**, không bao giờ trộn
  stdout; **`exit != 0` ⇒ `VS0501`** "thoát mã N" (§9.4) — hệ quả đã ghi: `grep` không khớp (exit 1) hiện **là lỗi**
  cho tới khi có `| complete` (VS1b). (2) **Ranh giới OS**: `vis-interp` **không** phụ thuộc OS — chỉ có
  `trait Ngoai` + `Interp.ngoai`; **không** host ⇒ `VS0604`; `vis-cli` cài runner. (3) `vis check` mặc định **cho**
  ngoài, `--no-extern` ⇒ `VS0601`. (4) **variadic `...rest: str`** gom mọi đối số vị trí còn lại thành `list[str]`;
  đối số **sau** variadic ⇒ `VS0601`; **`--x=v`** không lấy khoảng trắng. (5) timeout **10 s**/lần (kill rồi
  `VS0501`), chạy **tuần tự** (cap 1), `--explain-perms` in `EXEC …` ra stderr; consent thật + `cap[Proc]` ⇒ **VS2**.
  (6) Allowlist `doc-examples` **9 → 8**: khối 662 (`extern cmd … ...rest`) **hợp lệ** — chặt hơn, không nới.

- **VD-61** **Generic VS1b (ST-28)**: (1) **xoá kiểu (erase) + hợp nhất ở call-site**, **không** đơn hình hoá —
  để interp/VM/bytecode có **một** hiện thực (differential 0 lệch theo cấu trúc). (2) `FnSig` giữ **kiểu nguyên bản**
  (`raw_params`/`raw_ret` có `TypeRef{"T"}`) để hợp nhất; **thân hàm** dùng kiểu đã xoá (`Type::Error`) ⇒ không
  `VS0301` oan; call-site suy `T` đệ quy qua `list/map/opt/Result`, mâu thuẫn ⇒ **`VS0412`** nêu `T` + hai kiểu;
  kiểu trả thay `T` ⇒ `let n: int = lay("x")` vẫn `VS0412`. (3) Record/enum generic: `Hop[int]` ≡ `Hop[str]`
  (chưa có instantiation thật — **nợ VS1b**), payload đọc ra đúng theo giá trị. (4) Phép toán trên `T` **kiểm động**
  lúc chạy, sai ⇒ **mã hiện có** của toán tử (không bịa mã); constraint/trait ⇒ ST-29. (5) **`vis check` phải báo
  cùng chẩn đoán đầu như `vis run`** (không lọc theo lô); lọc `01xx/02xx/07xx` chỉ còn ở **`--syntax`** — dùng cho
  cổng "khối mã tài liệu **parse được**" (QD §7.4). (6) **VM entry**: tệp có `fn main` ⇒ entry phát `Call(main, 0)`
  (trước đó VM chạy thân cấp tệp và bỏ qua `main` — bug thật, đã khoá bằng test).

- **VD-62** **Trait/impl VS1b (ST-29)**: (1) **dispatch TĨNH ở tầng hạ** — `x.m(…)` với `x` có kiểu **tĩnh**
  `Record`/`Enum` (biến/tham số/trường/`let` có chú thích) và `m` **không** nội tại ⇒ hạ thẳng `Call(impl_fn)` +
  chèn `self` **theo giá trị** đầu tiên ⇒ interp và VM dùng **cùng HIR** (không bảng `impls`, không bump VBC1).
  (2) Thứ tự phân giải: **nội tại** (list/str/map/`std.*`) → **impl tĩnh** → `VS0403`; kiểu nhận **không tĩnh**
  (generic `T`, `{error}`, `Any`) ⇒ `VS0403` "không phân giải được … (VS1b — thêm chú thích kiểu)".
  (3) `VS0403` **mở rộng nghĩa** thành "trường, kiểu hoặc **phương thức** không tồn tại" và dùng cho: không có
  phương thức, **mơ hồ** (≥2 impl cùng tên), impl **thiếu/thừa** so với trait, trait lạ, `derives` lạ, `Json` trên
  kiểu chứa closure — **không** cấp `VS0404`; `TYPE_CODES` vẫn 4, `all_codes()` 28. (4) `derives Eq/Show/Json`
  **dùng ngữ nghĩa có sẵn** (so sánh cấu trúc, `show`, `json.encode`) — **chưa** sinh phương thức riêng (nợ VS1b).
  (5) `self` **theo giá trị**; `&self`/`&mut self` = **nợ VS1**. (6) Mồ côi (§5.4) **mới kiểm trong một tệp**;
  siêu trait chưa kiểm — nợ VS1b.

- **VD-63** **`vis test` + SHA-256 + đồng bộ chẩn đoán (ST-30/30b)**: (1) **SHA-256 thật** (`sha2 0.10`, MIT OR
  Apache-2.0 — duyệt allowlist theo QD §9.3) **thay** FNV-1a×4 cho **hash nguồn `VBC1`** và **`cache::khoa_multi`** —
  gỡ nợ VD-54 (1); vẫn 32 byte, **không** bump version; cache cũ tự miss; nạp **luôn** qua `load_verified`.
  (2) `vis test [--vm] [thư mục|tệp]` in **TAP** (`1..N`, `ok/not ok`, `# <tệp>:<dòng>:<cột>: <mã> <thông điệp>`),
  exit **0/1/2**; `--vm` **so đúng mã** như interp, ca cần **host chương trình ngoài** ⇒ **`ok … # SKIP`** đúng TAP
  (không giả xanh). (3) **Hai đường phải cho CÙNG danh sách chẩn đoán** — differential so **đủ** `(mã, dòng, cột,
  thông điệp)`, không chỉ phần tử đầu (lỗ hổng cũ: bỏ 100% ca `.err`); sửa **bên sai** (VM `index` ⇒ `VS0503` +
  câu interp, thêm `Bytes`, thông điệp đệ quy sâu; interp "tên …") ⇒ **312/312 khớp, 0 lệch**; ca khoá
  `hai_duong_cung_so_chan_doan`. (4) **26 ca bỏ qua** = ca **lỗi biên dịch** (hai đường chung front end) — muốn so
  chúng cần harness riêng cho chẩn đoán biên dịch (**nợ**).

- **VD-64** **Phạm vi vòng 5 (VS1c — đóng nợ `R4` §5)**: (1) **harness chẩn đoán biên dịch**: so **đủ**
  `(mã, dòng, cột, thông điệp)` giữa đường **interp** và đường **bytecode/VM** cho **cả** ca lỗi biên dịch (26 ca
  hiện bị bỏ qua) ⇒ **0 lệch**; nếu hai đường khác nhau thì sửa **bên sai**, **không** bỏ ca. (2) **`json.decode`**:
  đối xứng `encode`, trả `Result[Any, str]`? — chốt: `Result[T, str]` với `T = Any`, lỗi cú pháp ⇒ `Err(msg)` chứ
  **không** panic; `std.fs` **tối thiểu** (`read`, `write`, `exists`, `list`) **đi qua `trait Ngoai`/`Host`**, tuyệt
  đối **không** để `vis-interp`/`vis-vm` phụ thuộc OS; không host ⇒ `VS0604`. (3) **`| complete`**: stage cuối nhận
  `{stdout: list[str], stderr: list[str], code: int}` để `grep`/`exit 1` **không** còn là lỗi; **pipe→stdin** giữa
  hai chương trình ngoài (ghi vào stdin tiến trình sau), cap/timeout giữ như VD-60. (4) **generic thật**:
  `Hop[int]` ≠ `Hop[str]` — instantiation ở `tyck` + `codegen` (nếu buộc phải bump `VBC1` thì **hỏi Agent 0**);
  sai kiểu ⇒ `VS0412`; differential 100k **phải vẫn 0 lệch**. (5) **Quy trình**: từ vòng này **Agent 2 tự commit**
  task của mình (`--only` + push); Agent 0 chỉ commit `QUEUE`/`DECISIONS`/`AGENTS` + `docs/viscript/doc-examples-allow.txt`.

- **VD-65** **Harness chẩn đoán 3 đường (ST-32)**: (1) mọi ca `.vis`/`.vish` được phân loại **cứng** thành 3 tầng —
  **xanh** / **lỗi biên dịch** (`01xx–04xx`, `06xx`, `07xx`) / **lỗi lúc chạy** (`05xx`) — tổng phải bằng số ca sem;
  **không** ca nào bị bỏ qua. (2) Với ca lỗi biên dịch: `vis check` và `vis run --vm --no-cache` phải cho **cùng**
  `(mã, dòng, cột, thông điệp)`; **exit code ánh xạ `1 ↔ 2`** (không đòi bằng) — và `vis_bytecode::compile()` phải
  **được gọi tới** (không panic). (3) Với ca lỗi lúc chạy: `vis check` **rỗng**, `run` = `run --vm`. (4) Một hàm
  **`chuan_hoa`** dùng chung cho cả 3 đường (escape `\n`/`\t`/`\\`) + unit-test riêng — không rải logic chuẩn hoá.
  (5) **Codegen TIN tyck** (không tự kiểm kiểu): hiện **21/26** ca lỗi biên dịch vẫn sinh module chạy được; khoá hằng
  số `CODEGEN_TIN_TYCK = 21` để thay đổi phải có ý thức — đây là **nợ**, không phải hợp đồng.

- **VD-66** **`json.decode` + `std.fs` (ST-33)**: (1) `json.decode(s) -> Result[Any, str]` (tyck dùng `Ty::Any` ⇒ **bỏ
  kiểm tĩnh**, đúng VD-59(3)); object giữ **thứ tự xuất hiện**, trùng khoá ⇒ **giá trị cuối, vị trí đầu**; number có
  `.`/`e` ⇒ `float`, còn lại ⇒ `int`, **tràn i64 ⇒ `Err`** (không bão hoà/không thành float); escape **đủ** gồm
  **surrogate pair** (nửa cặp ⇒ `Err`); `null` ⇒ `none` kiểu `Opt(Error)` — hệ quả **nợ**: `let v: int? = json.decode(…)`
  ⇒ `VS0412` (đóng ở **ST-35**); lỗi ⇒ `Err("JSON không hợp lệ ở byte N: …")` với **byte offset** trong chuỗi gốc; cap
  độ sâu **128**. (2) **`std.fs`**: `read/write/exists/list` khai **có kiểu**, đi qua **`FsHost`** (một hàm `Host::fs()`
  mặc định `None`) ⇒ `vis-interp`/`vis-vm` **không** phụ thuộc OS; **không host ⇒ `VS0604` ở cả hai đường**;
  `vis-cli` impl bằng `std::fs`; `--explain-perms` in `FS read|write|exists|list <p>`; **cho phép** đường dẫn tuyệt
  đối/`..` (không sandbox), **consent thật + `cap[Dir]` ⇒ VS2**; `list` sắp xếp theo byte, **không** đệ quy. (3) **Bộ
  sem giữ hermetic**: không fixture nào cần FS ⇒ **338 ca không đổi**; test FS/JSON nằm trong `crates/vis-cli/tests/`.

- **VD-67** **Pipe→stdin + `| complete` (ST-34)**: (1) `a | b` khi `b` là **chương trình ngoài** ⇒ stdout của `a` được
  ghi vào **stdin** của `b`: `str` ⇒ **nguyên văn** (không thêm newline), `list[str]` ⇒ nối `\n` + **một** newline cuối;
  stdin **đóng** ngay khi ghi xong. (2) **`MAX_STDIN` = 8 MiB** (byte UTF-8) mỗi stage — vượt ⇒ `VS0501` nêu **số byte
  thật** + giới hạn, **trước khi spawn**; ghi stdin bằng thread riêng để tránh deadlock; `EPIPE` sau timeout được bỏ qua.
  (3) **`| complete`** ⇒ record `{stdout: list[str], stderr: list[str], code: int}` — **chỉ** dùng sau stage **ngoài**
  (Vis ⇒ `VS0602`); nhờ đó `grep` không khớp (`exit 1`) **không** còn là lỗi. (4) **Timeout 10 s/stage**, thông điệp
  nêu **stage nào**; **không** cap tổng (chuỗi N stage ⇒ tối đa N×10 s) — ghi JOURNAL. (5) `complete` có kiểu tĩnh
  **`Ty::Any`** (nợ kiểm tĩnh: `r.code` chưa kiểm được; pipeline chưa là biểu thức ở `.vish`).

- **VD-68** **Chữ ký lệnh generic (ST-37)**: (1) `CmdSig` mang **tham số kiểu** `[T]`; so khớp tham số khi hạ
  dùng **hợp nhất** (`unify`, mượn máy ST-28) thay so sánh bằng — khe ống (VD-53 (1)) tìm tham số vị trí **đầu
  tiên** hợp nhất được với kiểu giá trị vào; `take`/`lines` thành `take[T](xs: list[T], n: int) -> list[T]` /
  `lines(s: str) -> list[str]`. (2) **Xoá kiểu ở biên lệnh** như ST-28; **không** bump `VBC1`; **không** đơn
  hình hoá. (3) `CmdSig` thêm `nguon: SigSource { Builtin, FileCmd, ModuleCmd }` — tra chữ ký **không** hard-code
  "builtin ⇒ Rust"; vòng 6 chỉ dùng `Builtin`/`FileCmd`, `ModuleCmd` để dành cho `std.shell` viết bằng ViScript
  sau khi tách repo. (4) Không đổi hành vi `echo/lines/take` hiện có; 338 ca sem **không đổi**.

- **VD-69** **Closure-arg trong chế độ lệnh (ST-38)**: (1) `closure_arg` (§A.7) hạ **trong `cmd_lower`** bằng
  **re-parse span** — parser **không** giữ cây con (chỉ giữ token), chỉ kiểm cú pháp thân; hạ xong thành
  `HExpr::Lambda`; kiểu tham số là `fn(T) -> R`. (2) **Cộng offset** span của thân với vị trí bắt đầu trong
  nguồn ⇒ chẩn đoán bên trong thân báo **đúng** `dòng:cột` của tệp (ca khoá riêng). (3) `it` **chưa** có ở bước
  này (xem VD-72) — ST-38 chỉ nhận lambda tường minh `x => …`. (4) Không hạ closure-arg cho **chương trình
  ngoài** (mọi tham số ngoài vẫn là `str`, VD-60).

- **VD-70** **Ngữ nghĩa verb shell (ST-38)**: (1) **một** ngữ nghĩa, **hai** tên: `where` ≡ `.filter`,
  `each` ≡ `.each`, `first` ≡ `.first`, `count` ≡ `.len` — verb là **alias có chữ ký**, gọi lại bảng phương thức
  (`builtin::method`) để interp và VM dùng **cùng** đường (VD-55) ⇒ differential giữ 0 lệch. (2) `get`/`select`
  **không** phải `.get(i)` mà là **chiếu theo trường** (§9.7 `get name`) ⇒ thuộc nhóm bản ghi (ST-40b).
  (3) Vòng 6 làm trên **`list`**; `stream[T]` lười/SIGPIPE/`take` dừng nguồn sớm để **VSH1b** (§9.5) — ghi rõ
  trong `STATE`/báo cáo, không hứa ngữ nghĩa stream. (4) `first` trả `T?` (Opt); `count` trả `int`; `each` trả
  `()`.

- **VD-71** **`sort_by` (ST-39)**: (1) Mở rộng `list.sort_by` cho khoá `K ∈ {int, size, duration, str}` (hiện
  chỉ `int`, `Ty::FnIntInt`); sắp xếp **ổn định** (§13 `coll`), khoá tính **một lần**/phần tử (giữ như ST-26).
  (2) `--desc` đảo chiều; `--asc` không có. (3) Khoá `bool`/`path` **chưa** sắp được ở vòng 6 ⇒ `VS0412` kèm
  gợi ý. (4) Ca cũ của `list.sort_by` **phải vẫn xanh** (mở rộng, không đổi hành vi).

- **VD-72** **`it` + viết tắt trường (ST-40a)**: (1) Chỉ áp cho **tham số closure** của `cmd` đã biết chữ ký;
  dải token của tham số dừng ở `--flag`/`|`/`;`/`&` **độ sâu ngoặc 0**. (2) Ident trần tra `RecDef` của kiểu
  phần tử ⇒ `it.<field>`; `it` tường minh vẫn hợp lệ; shell **không** có biến trần nên không nhập nhằng với
  `$x`. (3) Trường không tồn tại ⇒ mã mới **`VS0605`** (sổ `COMMAND_CODES` 4 → 5; `all_codes()` tăng 1;
  `next_block()` **không** đổi). (4) Không áp cho chương trình ngoài; `where size > 5mb` trên `list[str]` ⇒
  `VS0602`/`VS0605` (không có trường), không im lặng.

- **VD-73** **Prelude bản ghi + `Host` proc (ST-40a)**: (1) `FileEntry{name: str, path: path, size: size,
  modified: int, is_dir: bool}` — `modified` là **ms** ở vòng 6 (`Type` chưa có `datetime`; §3.1 để VS2);
  `ProcEntry{pid: int, ppid: int, name: str, cpu: percent, mem: size, state: ProcState}`. (2) Bản ghi prelude
  nằm trong `TypeTable` do **ViScript** cấp (không phải tệp người dùng) — vì `sig_builtin()` không biểu diễn
  được `Record(RecId)` của tệp. (3) `ProcEntry` **portable**: không `tcb`/`service`/`cap`; ViFastOs tự ánh xạ.
  (4) `Host::procs`/`list_procs` **chưa** vào hợp đồng ở vòng 6 — chỉ chốt khi có backend; thêm `HOST_API: u32`
  (bắt đầu 1) để backend ngoài `viscript/` biết mình lệch hợp đồng.

- **VD-74** **`Render` (ST-41)**: (1) Giá trị cuối pipeline in qua `Render`: `list[bản ghi]` ⇒ **bảng** (tiêu đề
  = tên trường, canh trái, **một** dấu cách phân cột); bản ghi đơn ⇒ khối `khoá: giá trị`; `list[str]` giữ
  nguyên `Show` cũ (không phá 338 ca). (2) Độ rộng cột đo theo **Unicode scalar** trên chuỗi **NFC** (NFD tiếng
  Việt làm lệch cột; §9.6 + kinh nghiệm ST-03). (3) **Không** màu, **không** cắt theo bề rộng terminal, **không**
  ViTerm ở vòng 6 (VSH1b). (4) `print` của giá trị khác `list[bản ghi]`/bản ghi **không** đổi.

- **VD-75** **`ls`/`open` qua `Host` (ST-42)**: (1) `FsHost::list_entries(dir) -> Result[list[FileEntry], str]`
  — **một** lần gọi, có metadata, thứ tự **đã sắp** theo tên (byte); không đệ quy. (2) `ls(path = p".", --all:
  bool = false, --long: bool = false) -> list[FileEntry]`; `--long` **không đổi kiểu** (chỉ đổi hiển thị, VD-74).
  (3) `vis-interp`/`vis-vm` **không** gọi OS (ca âm khoá lại); thiếu host ⇒ `VS0604`. (4) Cờ **`--fs-mock`** của
  `vis-cli` chỉ để **test tất định** (không phải API); `ls`/`open` test qua mock, không đọc đĩa thật. (5) Cưỡng
  chế quyền (`cap[Dir]`) **không** thuộc vòng 6 — VS2.

- **VD-68** **Generic thật (ST-35)**: (1) instantiation biểu diễn bằng **`RecId`/`EnumId` interned theo tổ hợp**
  (`rec_inst`/`enum_inst` + `rec_pos`/`enum_pos`), **không** thêm biến thể `Type` ⇒ `Hop[int]` ≠ `Hop[str]` mà
  `same`/`coerce`/`tbl.field` **không** phải sửa lan rộng. (2) **Payload runtime vẫn xoá kiểu** ⇒ **không** đụng
  `codegen`/`VBC1`; differential 100k **0 lệch**. (3) `T` **chưa ràng buộc** ⇒ **giữ** hành vi xoá kiểu (không làm đỏ
  ca cũ). (4) Gán chéo hai instantiation ⇒ **`VS0412`**; cùng instantiation ⇒ **sạch**; trường thay `T` ⇒ kiểu cụ thể;
  `match` trên `Or[int,str]` thiếu nhánh ⇒ `VS0402`. (5) **Nợ**: kiểu trả hàm generic còn `{error}` khi `T` chưa suy;
  `Hop[Hop[int]]` chưa có id cho lớp trong (an toàn vì payload xoá kiểu).

- **VD-69** **Phạm vi vòng 6 (VSH1a)**: (1) **khe ống hợp nhất**: khe không còn phụ thuộc so **bằng** kiểu — chọn vị trí
  theo **khả năng nhận** (generic `CmdSig`), nên `( [1,2] ) | take 1` hợp lệ; seam `SigSource` để `vis-std`/prelude cấp
  chữ ký **generic** cho lệnh. (2) **closure-arg** (`CmdClosure`) hạ thành `Lambda` **cộng offset span** để lỗi trong
  thân closure báo đúng `dòng:cột`; thêm `where`/`each`/`first`/`count`. (3) `sort_by` **`--desc`** (giữ **ổn định**),
  khoá `int/size/duration/str`; `bool` ⇒ `VS0412`. (4) **prelude bản ghi** `FileEntry`/`ProcEntry` + `it` + **viết tắt
  trường** ⇒ mã mới **`VS0605`** trong sổ `06xx` (4 → 5 mã; `BOOKS`/`all_codes()`/`next_block()` cập nhật **kèm lý do**).
  (5) `get`/`select` chiếu theo trường. (6) `Render`: bảng `list[bản ghi]` + khối khoá–giá trị, căn cột theo **NFC**;
  `print` cũ cho `list[str]` **không đổi**. (7) `ls`/`open` qua **`FsHost`** (+ `list_entries` metadata), `--all/--long`,
  cờ **`--fs-mock`** cho test tất định; pipeline §9.7 dòng 672 chạy được ⇒ allowlist **8 → 7** (chỉ khi thật xanh).

- **VD-70** **Khe ống hợp nhất + allowlist 8 → 6 (ST-37)**: (1) khe ống chọn tham số vị trí **đầu tiên nhận được**
  kiểu vào, kiểm bằng **hợp nhất một chiều** (`T := int` rồi so phần còn lại) — **không** dùng so `==`; đầu vào
  `{error}` (stage nhóm/lệnh lạ) ⇒ chọn khe **vị trí đầu**; khe **không** vào tham số `--cờ`, **không** đè bind tường
  minh; không khớp ⇒ `VS0603`; **`take 1` không có pipe** vẫn là `VS0602` (đúng VD-53 (4)). (2) `CmdSig`/`SigSource`
  đặt ở **`vis-types`** (`src/cmdsig.rs`) — **một** nguồn chữ ký cho `tyck` **và** khe ống; `vis-std`/prelude cắm
  bằng impl trait. (3) `CmdGroup` **suy kiểu literal** (list/str/int/bool) thay `{error}`. (4) **Allowlist doc-examples
  giảm 8 → 6** vì khối **1451** (`walk`/`where`/`sort_by`/`to json`/`save`) và **672** (`ls | where | sort_by --desc |
  take 10`) **nay hợp lệ** — lệnh lạ = chương trình ngoài + khe hợp nhất; **chặt hơn, không nới**. (5) Nợ **VSH1b**:
  stage đầu là **biểu thức** không ngoặc (`[1,2] | take 1` ⇒ `VS0201`); `( … )` làm **đối số** ⇒ `VS0603`; chẩn đoán
  lúc chạy trong `( … )` dùng vị trí **tương đối** trong ngoặc.

- **VD-71** **Closure-arg + lệnh shell VS1a (ST-38)**: (1) `CmdClosure` hạ thành `Lambda`; **span tuyệt đối** bằng cách
  lex/parse `" ".repeat(offset_byte) + thân` ⇒ lỗi trong thân báo **đúng `dòng:cột` của tệp** (kể cả khi trước đó có
  ký tự nhiều byte). (2) Lệnh mới `where[T](list[T], fn(T)->bool) -> list[T]`, `each[T](list[T], fn(T)->()) -> ()`,
  `first[T](list[T]) -> T?`, `count[T](list[T]) -> int` — khai **hai chỗ** (`cmdsig` cho tyck/khe, `builtin` cho chạy),
  **một** hiện thực dùng **`Host::call`** cho cả interp/VM; **không** thêm vào `BUILTIN_VALUE_NAMES` (chúng là lệnh
  **chế độ shell**, thêm vào sẽ chặn `fn first` của người dùng ⇒ `VS0302`). (3) `each` trả `()` và **là stage cuối**
  (nối tiếp ⇒ `VS0603`); `first` trên list rỗng ⇒ `none`. (4) **Sugar vị từ ngầm `it`**: đối số `where` không phải
  closure ⇒ hạ `it => …`, chỉ đổi **định danh trần chưa khai báo** thành `it.<tên>`; `where 1` ⇒ **`VS0412`**.
  (5) `lower_vach` hạ nhóm **vào bảng chung** (`RecId` ẩn danh phải cùng bảng). Nợ: `where size` (kiểu không `bool`)
  hiện `VS0501` lúc chạy.

- **VD-72** **`sort_by` chế độ shell (ST-39)**: (1) lệnh builtin **mới** `sort_by[T](xs: list[T], f: fn(T)->K,
  --desc: bool = false) -> list[T]`, **một** hiện thực qua `Host::call`; **`std.list.sort_by` giữ nguyên** (khoá
  `fn(int)->int`) — **hai** `sort_by` là **nợ VS2** (hợp nhất khi có generic + trait). (2) `--desc` là **cờ** (khe ống
  không chiếm); `--desc=false` hợp lệ; thân closure **dừng** ở `--…` (parser không nuốt cờ vào thân). (3) So sánh
  khoá: `int`/`float` theo số, **`size` theo BYTE**, **`duration` theo NANOGIÂY**, `percent` theo `f64`, `str`/`path`
  theo **byte UTF-8**; **ổn định cả hai chiều** (đảo comparator, không đảo mảng). (4) Khoá `bool`/`map`/`record`/`list`/
  `set`/`unit` ⇒ **`VS0412`** (kiểm ở lowering cho `.vish` + `kiem_builtin` cho `.vis`; runtime `VS0501` là chốt hai).
  (5) `sort_by` 1 đối số (không closure) vẫn là **chương trình ngoài** (không bị tyck chặn).

- **VD-73** **Prelude bản ghi + `VS0605` (ST-40a)**: (1) `FileEntry { name: str, size: size, modified: duration,
  is_file: bool }` và `ProcEntry { pid: int, name: str, cpu: percent, mem: size }` khai **trong Rust**
  (`vis-types/src/prelude.rs`) và đăng ký **chỉ khi hạ `.vish`** — `.vis` **không** thấy chúng (khai trùng vẫn chạy).
  (2) `it` suy kiểu **thật** từ kiểu phần tử của giá trị vào (`kieu_text`/`kieu_stage`); không suy được (`{error}`/`Any`)
  ⇒ **kiểm động** như cũ, **không** đỏ oan. (3) Viết tắt trường (`size` ≡ `it.size`) giữ luật VD-71. (4) **`VS0605`**
  "trường không có trên `it`" trong sổ `06xx` (4 → 5 mã) ⇒ `all_codes()` **29**; `BOOKS` 7, `next_block()` **801**.
  (5) `.vish` **chưa có** cú pháp khai `cmd`/`type`/`let` có kiểu ⇒ kiểu `it` thật đến từ **literal bản ghi** khớp
  prelude; nợ: khi `.vish` có khai `type` ⇒ phải `VS0302` với tên prelude.

- **VD-74** **`get`/`select` (ST-40b)**: (1) `get(xs, f) -> list[K]` — `… | get name` ⇒ sugar `it => it.name` (VD-71);
  kiểu `K` tính **từ thân lambda** ở `kiem_builtin` (không để `TypeRef` xuống tyck ⇒ không `VS0301` oan). (2)
  `select(xs, k1, k2, …) -> list[record#N ẩn danh]` — hạ thành lambda trả **record literal**, trường kiểu lấy từ `it`,
  **giữ đúng thứ tự đối số**; **không** thêm `Type::Any` (nợ VS2: chưa `to json` được). (3) Trường lạ trên `it` kiểu
  **thật** ⇒ **`VS0605`**; `it` kiểu `{error}`/`Any` ⇒ **kiểm động**, **không** báo oan. (4) `get`/`select` 0 đối số ⇒
  `VS0602`; `get` trên list rỗng ⇒ `[]`. (5) R1 (>1.500 dòng) ⇒ **tách** `lower/chieu.rs`, không nới ngưỡng.

- **VD-75** **`Render` (ST-41)**: (1) giá trị **cuối pipeline** ở chế độ shell được render qua builtin **`render`**
  (`vis-interp/src/render.rs`) — `boc_print` → `boc_render`; **`print` giữ nguyên từng byte**. (2) `list[bản ghi]` ⇒
  **bảng**: tiêu đề = tên trường, căn **trái** `str`, **phải** `int`/`float`/`size`/`duration`/`percent`; bản ghi đơn ⇒
  **khối khoá–giá trị**; `list[bản ghi]` **rỗng** ⇒ `(rỗng)`; **`list[str]`/`list[int]` rỗng và kiểu `{error}` giữ
  `Show` (`[]`)**. (3) Độ rộng cột = **số `char` sau NFC** (không `display_width`) ⇒ NFD/NFC cùng bảng, emoji/CJK = 1.
  (4) **Đổi hành vi có chủ ý**: 7 fixture `.out` chuyển `Show` → bảng (liệt kê trong commit `f2c256f`), ~20 assertion
  Rust theo cùng luật. (5) **Cache**: nhãn `PHIEN_BAN` phải đổi khi ngữ nghĩa render đổi (`+render-st41`) — nếu không,
  `--vm` dùng module cache cũ và in `Show`.

- **VD-76** **`ls`/`open` + `--fs-mock` (ST-42)**: (1) `ls [p] [--all] [--long]` ⇒ `list[FileEntry]` (prelude VD-73),
  **sắp xếp theo tên byte**, `--all` gồm tên bắt đầu `.`, **`--long` no-op** (nợ), **không đối số ⇒ `"."`**;
  `open(p)` ⇒ `str`, lỗi ⇒ `VS0501` nêu **đường dẫn + lỗi OS**. (2) **`FsHost` mở rộng** `MucFs{name,size,modified_ns,
  is_file}` + `list_entries` **có default `Err`** ⇒ impl cũ vẫn biên dịch; `vis-interp`/`vis-vm` **không** biết OS.
  (3) **`--fs-mock <tệp.json>`** (`dirs`/`files`; `size` = literal **theo bảng lexer** ⇒ `2mb` = **2 000 000**, `2mib` =
  2 097 152; `modified` = duration **từ epoch** ⇒ **tất định**); không mock ⇒ `FsHost` thật. (4) **§9.7 khối 672 chạy
  thật end-to-end** với `--fs-mock` (bảng, interp ↔ `--vm` cùng byte). (5) `vis test` **không** có `--fs-mock` ⇒ fixture
  cần FS nằm ở `test/cmd/fs/**` chạy bằng test riêng, **không** tăng `sem` ảo.

- **VD-77** **Phạm vi vòng 7 (VSH1b)**: (1) **`.vish` có khai báo kiểu** — `type`/`enum`/`let x: T`/`cmd f(…) -> T` được
  chấp nhận **trong chế độ lệnh** (trước đây `(` bị hiểu là `CmdGroup`): khai báo là **item** đứng trước stage, không
  trộn với biểu thức; tên trùng prelude ⇒ **`VS0302`**. (2) **`where` kiểm tĩnh đầy đủ**: khi kiểu `it` **biết được**
  mà vị từ không ra `bool` ⇒ lỗi **kiểm tĩnh** (`VS0412`), hết nợ `VS0501` lúc chạy; kiểu `{error}` ⇒ **giữ kiểm động**.
  (3) **`ls --long` thật**: thêm cột thời gian (định dạng **tất định**, UTC) + quyền/kích thước **chỉ đọc** từ `FsHost`
  (`MucFs` mở rộng `mode`/`mtime`), **không** đụng `Render` bảng mặc định (chỉ khi `--long`). (4) **`( … )` làm đối số**:
  nhóm được dùng như biểu thức ở vị trí đối số; **chẩn đoán trong ngoặc** dùng vị trí **tuyệt đối** trong tệp (hết nợ
  "vị trí tương đối"). (5) **Không** thêm mã lỗi mới trừ khi được duyệt; mọi thay đổi hành vi phải liệt kê ca `.out` đổi.

- **VD-78** **Khai báo trong `.vish` (ST-44)**: (1) `.vish` = **vùng item ở đầu tệp** + **vùng stage** phía sau; ranh giới là
  dòng **cột 1** đầu tiên không bắt đầu bằng từ khoá item (`type|enum|cmd|fn|use|const|trait|impl|alias|newtype|extern`)
  và không phải `let` ⇒ từ đó là stage. (2) Tiền tố parse lại **Mode::Script** với **span tuyệt đối** (đệm offset) và hạ
  vào **cùng** `Symbols`/`TypeTable` ⇒ item thấy prelude, stage thấy item. (3) Khai báo **sau** stage ⇒ **`VS0201`** kèm
  gợi ý "khai báo phải đứng trước stage đầu tiên"; **trùng tên** (kể cả `FileEntry`/`ProcEntry`) ⇒ **`VS0302`** (không
  thêm mã). (4) **`f()`** = lời gọi **0 đối số**; nhóm rỗng đứng riêng ⇒ `VS0201`. (5) `let x: T = <có `|`/`;`/`>`/`<`/`&`>`
  ⇒ **shell-let**, ngược lại ⇒ khai báo script. (6) **Nợ VSH1b-2**: thân `cmd` viết bằng **pipeline lệnh** (thụt) chưa
  hỗ trợ — thân hiện là **script**.

- **VD-79** **`where` kiểm tĩnh / `each` không kiểm (ST-45)**: (1) **`where`** kiểm ở **`tyck`** (`coerce` thân lambda,
  khai `fn(T)->bool`) — đã chạy cho cả `.vish`; vị từ sai ⇒ **`VS0412`** nêu **kiểu thật**. (2) **`each`** khai kiểu trả
  lambda = **`{error}` (any)** ⇒ **không** kiểm kiểu trả (giống `for` trong Rust); lỗi khác trong thân **vẫn** báo.
  (3) **`it` không suy được kiểu** (`{error}`/`Any`) ⇒ **giữ kiểm động** (`VS0501` lúc chạy), **không** đỏ oan.
  (4) **Hai mã field-access khác nhau đang tồn tại**: closure tường minh ⇒ **`VS0403`** (tyck), viết tắt `it` ⇒
  **`VS0605`** (hạ) — **cả hai đều đỏ**; **thống nhất** thành một mã ⇒ task riêng (nợ). (5) R1 (>1.500 dòng) ⇒ tách
  `lower/sugar.rs`, không nới ngưỡng.

- **VD-80** **`ls --long` + nhóm-đối-số (ST-46)**: (1) `ls --long` ⇒ **`list[FileEntryLong]`** (`name,size,modified,is_file,
  mode,modified_time`) — **bản ghi riêng**, prelude `FileEntry` **không đổi**; `mode: u32` trong `MucFs` (**0 ⇒ `-`**),
  `modified_time` = **UTC `YYYY-MM-DD HH:MM`** (civil-from-days, **không** thêm crate). (2) **Nhóm là đối số** = biểu
  thức (`take ( 1 + 1 )`), **bỏ `VS0603`** cho trường hợp này, sai kiểu ⇒ `VS0412`; nhóm **rỗng** vẫn là **lời gọi
  0 đối số**; shell-let (VD-78(5)) **không** đổi. (3) **Chẩn đoán tuyệt đối** trong `( … )` bằng **đệm dấu cách**
  (không `\n` ⇒ không sinh `Indent`/`Dedent` giả). (4) Nợ: **nhóm-lệnh có cờ** (`( ls … --long )`), nhóm **nhiều dòng**,
  `where ( x => x )` kiểm **động** (tyck chưa kiểm call `where`).

- **VD-81** **Phạm vi vòng 8 (VSH1c)**: (1) **thân `cmd` bằng pipeline lệnh**: khối thụt sau `cmd f(...) -> T:` parse ở
  **chế độ lệnh** (Indent/Dedent + stage), hạ thành thân hàm trả giá trị cuối pipeline; chạy **cả hai đường**.
  (2) **Nhóm-lệnh có cờ** (`( ls … --long )`) được hạ như stage (không rơi về lệnh ngoài `(`); **nhóm nhiều dòng**
  hợp lệ trong ngoặc. (3) **Thống nhất mã field-access**: một mã duy nhất cho "không có trường" giữa **tyck** và **hạ**
  (hiện `VS0403` vs `VS0605`) — chọn **`VS0605`** (chuyên biệt, đã có vi/en/ja từ ST-40a), đổi phía tyck; snapshot +
  test cũ sửa **kèm lý do**. (4) **`where ( x => x )`** kiểm **tĩnh** (mở rộng `kiem_builtin` cho `where`).
  (5) **Stage đầu là biểu thức** (`[1,2] | take 1`) nếu **không** phá văn phạm §9.5; nếu phá ⇒ **giữ `VS0201`** + ghi nợ.
  (6) Mỗi task ≤ 3 h; VS2 vẫn ngoài phạm vi và **phải** được nêu trong R8.

- **VD-82** **Thân `cmd` bằng pipeline (ST-48)**: (1) Parser **thử `Mode::Script`** trên thân (bỏ thụt): sạch ⇒ **script**
  (raw như VD-78), hỏng ⇒ **thân lệnh** (`CmdKhai` dừng sau `:`; dòng thân do parser lệnh chính parse ⇒ **span tuyệt đối**).
  (2) Tiền tố được thay bằng **filler `()` giữ nguyên độ dài byte** ⇒ offset của **mọi** item sau **không đổi**.
  (3) Thân lệnh = **đúng một pipeline**; rỗng / nhiều pipeline / lẫn `let`-và-lệnh ⇒ **`VS0201`**. (4) Hạ bằng
  `ha_pipeline` trên Lowerer **con** (chung bảng) ⇒ `HItem::Fn.body = [giá trị cuối pipeline]`, **không** `HExpr::Pipeline`,
  **bỏ** `boc_render`. (5) Nợ: thân lệnh **chưa dùng tham số `cmd`**; body "vừa script vừa lệnh" ⇒ `VS0201` (bảo thủ).

- **VD-83** **Nhóm-lệnh + nhóm nhiều dòng (ST-49)**: (1) **một** đường hạ nhóm duy nhất (`lower_nhom_lenh` tự-dựng-Call
  **đã xoá**): nội dung nhóm được **đổi newline thành dấu cách** (1 byte→1 byte ⇒ offset **tuyệt đối** giữ nguyên), parse
  lại (`parse_voi`), hạ **cả pipeline** bằng Lowerer **con** chung `Symbols`/`TypeTable`. (2) Parser **không** còn chặn `|`
  trong nhóm; newline **không** kết thúc nhóm. (3) `kieu_stage` trả kiểu **đúng** của nhóm ⇒ `( ls --long ) | select mode`
  kiểm tĩnh được. (4) Nợ: trong nhóm chỉ **một** pipeline (nhiều pipeline/`;` ⇒ `VS0201`); kiểu tĩnh lấy **stage đầu**.

- **VD-84** **Trường là lỗi kiểu ⇒ `VS0405` (ST-50)**: (1) mã "không có trường" nằm ở sổ **`04xx`** (**`VS0405`**), không ở
  `06xx`; `COMMAND_CODES` **4**, `TYPE_CODES` **5**, `all_codes()` **29**, `BOOKS` 7, `next_block()` **801**. (2) `VS0403`
  giữ nghĩa **khai báo** (literal bản ghi, `with`, pattern, literal enum, phương thức, impl/derive trait). (3) **Tra mã
  theo `id`, không theo chỉ số mảng** (`ma_truong()`) — đổi thứ tự sổ không được làm lệch mã. (4) **`where ( x => … )`**
  kiểm **tĩnh** (nhánh `"where"` trong `kiem_builtin`; thân phải `bool` ⇒ `VS0412`; `{error}` ⇒ kiểm động); ở hạ, lambda
  trong ngoặc được gán kiểu tham số = **kiểu phần tử**. (5) Nợ: `VS0403` còn nghĩa rộng; chưa kiểm kiểu tham số khi lambda
  không chú thích.

- **VD-85** **Stage đầu là biểu thức (ST-51)**: (1) Nhận **hẹp nhất** trong chế độ lệnh: token **đầu dòng** là `[`, `{`, hoặc
  **string literal**; có `|` ở **độ sâu 0**; văn bản tiền tố parse **`Mode::Script` phải 0 chẩn đoán**. (2) CST = `CmdStage`
  bọc `CmdGroup` **không ngoặc** ⇒ **tái dùng** đường hạ `( … )` (không thêm đường mới). (3) `{…}` được **bọc
  `HExpr::List` một phần tử** ⇒ stage hạ nguồn (`where`/`take`/`select`/bảng) hiểu như một dòng. (4) **KHÔNG** nhận:
  số (`1 | …` vốn là lệnh ngoài), **biến** (`x | …` nhập nhằng §9.5), **tên lệnh**; không `|` ⇒ giữ **`VS0201`**.
  (5) Nợ: bọc-list chỉ cho stage **đầu**; biến ⇒ **VSH1c-2**.

- **VD-86** **Phạm vi vòng 9 (VSH2)**: (1) **Bản ghi ẩn danh đi được JSON**: `select`/literal ẩn danh phải `json.encode`
  được (dùng **tên tổng hợp** ổn định hoặc `null` cho tên, **không** cần `Type::Any`). (2) **Một `sort_by`**: shell
  `sort_by` và `std.list.sort_by` dùng **cùng** comparator (khoá `int/float/size/duration/percent/str`; **ổn định**;
  `--desc` chỉ ở shell) — `std.list.sort_by` cũ **không** phá (nhận `fn(int)->int` vẫn chạy). (3) **`CODEGEN_TIN_TYCK`**:
  codegen **không** im lặng sinh module khi HIR còn kiểu `Error` — phải **từ chối** với chẩn đoán rõ **hoặc** chứng minh
  và **hạ hằng số** 21 xuống; differential **0 lệch** bắt buộc. (4) `Ty::Any`/`&self` **chỉ** nếu rẻ, còn lại nợ có chủ đích.
  (5) **VS2** phải có **tiêu chí đo** trong `R9` (async/kênh/net/Landlock/LSP/gói/registry/4 nền tảng).

- **VD-87** **`to json` + hợp nhất `sort_by` (ST-53)**: (1) **`to json`** là **stage lệnh** (hạ `Call(std.json.encode,
  [prev])`, `--pretty` ⇒ `encode_pretty`), **không** cần `use std.json`; bản ghi **ẩn danh** encode **theo trường**
  (serializer **bỏ tên** bản ghi) ⇒ **không** cần `Type::Any`. (2) **Một comparator**: `sap_xep_theo_khoa` — tính khoá
  **một lần**, **sort ổn định**, `desc` chỉ **đảo comparator**; shell `sort_by` = `KieuKhoa::Moi`
  (`int/float/size/duration/percent/str`), `std.list.sort_by` = `KieuKhoa::ChiInt` (**giữ** `FnIntInt` + câu lỗi cũ).
  (3) Nợ: khoá `str` cho `std.list.sort_by`.

- **VD-88** **Codegen từ chối HIR có chẩn đoán (ST-54)**: (1) **Không** chặn bằng soi `Type::Error`/`TypeRef` — walker chứng
  minh HIR **sạch kiểu** kể cả khi tyck báo lỗi. (2) Cửa đúng: `compile_kiem(hir, symbols, text, chan_doan)` — **có**
  chẩn đoán `error` ⇒ `Err(CodegenError::ChanDoan{short, dong, cot, msg})`, **không** sinh module; `compile()` giữ nguyên
  (= `compile_kiem(…, &[])`). (3) Phòng thủ `HExpr::Error` ⇒ `Err(Unsupported("error-expr"))`. (4) Cửa **chỉ chặn
  `Severity::Error`**; cảnh báo **không** chặn (có ca). (5) `CODEGEN_TIN_TYCK` = **0**; `diag_cmp` so **đủ** ba thứ
  (mã, dòng:cột, thông điệp) qua `compile_kiem`.

- **VD-89** **Phạm vi vòng 10**: (1) Sáu task **ST-56..ST-61** + **ST-62 (R10)**, chạy **tuần tự** (đụng chung
  `stdlib.rs`/`tyck`/`codegen`/`lower` ⇒ không song song ghi). (2) Mỗi task **red-first**, commit riêng `[ST-##]`,
  push sau khi cổng xanh. (3) **Không** nới ngưỡng, **không** tăng allowlist (chỉ được giảm). (4) Async/kênh **phải**
  cùng byte interp ↔ `--vm`; nếu một task không đủ thì **tách** thành task nhỏ hơn, **không** hạ tiêu chí.

- **VD-90** **Async — tác vụ chạy tới hoàn tất, một luồng, tất định (ST-60)**: (1) `async fn` khai báo được; **gọi**
  `async fn` **không** chạy thân ngay mà trả `Task[T]`; `spawn f(args)` = tạo tác vụ + đưa vào **hàng đợi FIFO**;
  `await t` chạy `t` **tới hoàn tất** (nếu chưa chạy) rồi trả `T`. (2) Bộ lập lịch **một luồng, tất định**; **không
  nhường giữa thân** (run-to-completion) — đây là **nợ ghi rõ**: chưa có đình chỉ giữa thân nên **không** có song song
  thật; mô hình này để hai đường (interp/VM) ra **cùng byte**. (3) Cuối chương trình: task chưa `await` được **join**
  theo **thứ tự spawn**; thân task lỗi ⇒ `VS0506` (mã mới, sổ 05) kèm tên hàm. (4) `await` lần hai trả **cùng** giá trị
  đã lưu (không chạy lại). (5) 10.000 task dùng **hàng đợi phẳng** (Vec), không đệ quy theo số task ⇒ không tràn.
  (6) Benchmark 10k task là **test thật**, không `#[ignore]`.

- **VD-91** **Kênh — đệm hữu hạn + deadlock có chẩn đoán (ST-61)**: (1) `chan[T](n)`, `n ≥ 0`; `send(v)`,
  `recv() -> T?`, `close()`. (2) `recv` khi rỗng: **chạy tác vụ khác** trong hàng đợi (FIFO, tới hoàn tất) rồi thử lại;
  vẫn rỗng và **không còn task** ⇒ `VS0505` **deadlock** (không treo). (3) `send` khi đầy: như (2); vẫn đầy ⇒ `VS0505`;
  `send` sau `close` ⇒ `VS0504`. (4) `recv` sau `close`: còn hàng ⇒ trả phần còn lại; rỗng ⇒ **`none`**.
  (5) `Chan[T]` là kiểu dựng sẵn; tyck **đòi khớp `T`** khi `send`/`recv`.

- **VD-92** **`&self`/`&expr` — không mượn thật (ST-56)**: (1) `&self` trong `impl` là receiver **không mượn thật**,
  hạ **y như** `self`; `&expr` giữ **nguyên kiểu trong**, hạ **identity** (không tạo giá trị mới). (2) Codegen **hết**
  `HExpr::Ref ⇒ Unsupported`; interp ↔ `--vm` cùng kết quả. (3) Mượn thật/kiểm vòng đời là **nợ VS2+**, phải ghi rõ —
  **không** được mô tả là "đã có borrow".

- **VD-93** **`derives` sinh mã ở `lower` (ST-57)**: (1) Sinh **HIR** trong `lower` (không macro, không sửa parser).
  (2) Tối thiểu: `Show`, `Eq` (thêm `Json` nếu rẻ) — sinh **phương thức thật** gọi được như `p.show()`/`a.eq(b)`, thứ tự
  theo **trường khai báo**, tất định. (3) Trường không derive được (`float` với `Eq`) ⇒ chẩn đoán **tĩnh** `VS04xx`,
  **không** sinh mã. (4) Người dùng viết tay cùng tên ⇒ **người dùng thắng**, không sinh bản trùng.

- **VD-94** **Bỏ `Ty::Any` bằng biến kiểu trong chữ ký std (ST-58)**: (1) `stdlib.rs` dùng **biến kiểu** thay `Any`
  (`list.sort: (list[T]) -> list[T]`; `list.sort_by: (list[T], fn(T) -> int) -> list[T]`; `map.get_or: (map[K,V], K, V) -> V`).
  (2) `tyck` **hợp nhất** kiểu đối số thật với biến kiểu; không khớp ⇒ `VS0401` **tĩnh**. (3) Ngoại lệ **có ghi** (closure
  chưa suy hai chiều) phải viết rõ tại chỗ, **không** để `Any` quay lại. (4) `all_codes()` **không** đổi, trừ mã mới
  của ST-60/ST-61.

- **VD-95** **Hiện thực async — builtin nội bộ, không thêm tag `VBC1` (ST-60)**: (1) `async`/`await` **đã** lex+parse+lower
  sẵn (`HFn::is_async`, `HExpr::Await`); chỉ thiếu tyck + runtime. (2) Hạ `async fn`/`spawn`/`await` thành **lời gọi
  builtin nội bộ** `$tac_vu`, `$spawn`, `$await` (tên có `$` ⇒ người dùng không viết được) + **một** runtime chia sẻ
  `vis-interp/src/rt.rs`; **không** thêm tag `VBC1`, không đổi verifier/cache. (3) `Task[T]`/`Chan[T]` =
  `Type::TypeRef{name,args}` (đã đệ quy sẵn); `FnSig.is_async`; `await` bỏ **một** lớp `Task`. (4) `spawn` chỉ nhận
  **tên hàm `async` cấp tệp**, đối số **vị trí**, **không** `--tên`; `async fn` không dùng làm giá trị; `async` trên
  method/`cmd` ⇒ **`VS0412`** (nợ ghi `R10` §5). (5) Lỗi thân tác vụ: **mọi** lỗi bọc `VS0506` (message kèm mã gốc),
  span = **lúc tạo**, áp **cả** `await` lẫn join — một quy tắc, một kết quả. (6) Join cuối **theo thứ tự spawn**,
  **chỉ khi main thành công**; `await` lần hai trả cache. (7) `MAX_TASK_NEST = 256` cho chặn lồng; 10k task phẳng là
  test thật. Nợ: không nhường giữa thân, không GC (chu trình `Rc`), `await` sâu sát ngưỡng frame.

- **VD-96** **Hiện thực kênh + ba mã runtime mới (ST-61)**: (1) `$kenh` tạo `Chan` (**một** hiện thực trong `builtin`),
  `send/recv/close` đi cùng một nhánh method cho cả interp lẫn VM. (2) Chặn (`send` đầy, `recv` rỗng) ⇒ **drain** hàng
  đợi FIFO (chạy task đầu tới hoàn tất, bỏ task đã xong), thử lại; hết queue ⇒ **`VS0505`** (deadlock, không treo).
  (3) `send` sau `close` ⇒ **`VS0504`**; `recv` sau `close` còn hàng ⇒ trả tiếp, rỗng ⇒ `none`. (4) Sổ `05xx` thêm
  **`VS0504`/`VS0505`/`VS0506`** (giữ index 0..2), `all_codes()` **29 → 32**; sửa **đủ** 6 chỗ khoá số ca. (5) Byte
  giống nhau nhờ: cùng `rt.rs`, cùng thứ tự join, cùng hằng thông điệp, span **nút gọi** — cấm in địa chỉ/thứ tự map.

- **VD-97** **Một ngữ nghĩa cho phép gán (ST-56b)**: (1) `self` là **tham số bất biến** như mọi tham số ⇒ `self.x = v`
  khi `self` (không `var`) là **`VS0412`** ở **cả hai** đường (tyck chặn trước codegen). (2) `var self`/`var x` được gán:
  `x.f = v` = **rebind cục bộ** `x = x with f: v` (đúng codegen hiện có; người gọi **không** thấy đổi — giá trị), `x[i] = v`
  = sửa **tại chỗ**. (3) `let`/tham số thường: giữ `VS0412` (đã có). (4) **Interp** phải làm đúng hai đường gán trên
  (hiện chỉ VM làm) ⇒ interp ↔ VM cùng byte. (5) `vis run --vm` gặp **lỗi codegen** phải **exit ≠ 0** + chẩn đoán ra
  stderr (hiện in rồi exit 0); nếu chương trình đó là lỗi **tĩnh** thì tyck phải chặn trước để parity không lệch.

- **VD-98** **Mồ côi đa tệp + siêu trait + derives sinh mã (ST-57)**: (1) **Mồ côi**: cho `impl` khi **trait hoặc
  kiểu** được khai trong **chương trình** (luật Rust); `impl` inherent trên kiểu nhập **giữ nguyên** (ca đang xanh);
  mồ côi **thật** = cả trait lẫn kiểu ngoài chương trình ⇒ `VS0403` thông điệp riêng. Dữ liệu: `TraitSig{name,
  supertraits, methods}` + `LowerOptions.trait_ngoai/impl_ngoai` (module xuất `pub_traits`). (2) **Siêu trait**: dùng
  **`VS0403`** (không cấp mã mới — tiền lệ ST-29); kiểm **một tầng**, **sau** vòng 1b (thứ tự impl tự do); bắc cầu = nợ.
  (3) **`derives`**: sinh **HIR ở lượt 1c** (sau 1b, trước mọi lượt hạ thân — để `p.show()` trước khai báo vẫn phân giải);
  tên nội bộ **có tiền tố** `<Kiểu>.show`/`<Kiểu>.eq` (không `VS0302`); `show` dựng bằng `HExpr::Interp` để **khớp byte**
  `Value::show`; `eq` = `self == other` (dùng `eq_value` sẵn có); người dùng viết tay **thắng**; generic/`float`+`Eq`/
  `Fn` ⇒ `VS0403` **tĩnh, không sinh mã**; `Json` giữ **kiểm-only**; `enum show` **cắt** (nợ), `enum eq` làm được.
  (4) Ca mới ở **nhóm `trait/`** ⇒ **19 nhóm**; khoá interp=`--vm` tự động.

- **VD-99** **Bỏ `Ty::Any`: biến kiểu trong chữ ký `std.*` (ST-58)**: (1) `Ty` const-friendly giữ `Copy`, thêm
  `Var(u8)` (`0=T,1=K,2=V`), `List(&'static Ty)`, `Map(&'static Ty,&'static Ty)`; **xoá hẳn** `Ty::Any` và `Ty::kieu()`
  (cái `else { continue }` là lỗ làm "Any" lọt im lặng). (2) `sang_type()` **tổng**: `Var(i)` → `Type::TypeRef{name:
  ["T","K","V"][i]}` (tái dùng `TypeRef`, **không** thêm `Type::Param`). (3) `kiem_std` **hợp nhất** bằng `hop_nhat` +
  `thay_gen` sẵn có (trái→phải, `{error}` không gán, biến xuất hiện 2 lần phải `same`); số đối số ⇒ `VS0401`, kiểu ⇒
  `VS0412`; **không** mã mới. (4) **16/38** chữ ký đổi (list 8, map 5, json 3); `json.decode` trả `Var` **không gán** ⇒
  `{error}` (giữ VD-66). (5) **Ngoại lệ có ghi duy nhất**: `list.sort_by` khoá = `Ty::FnKhoa` (lambda chưa chú thích là
  `{error}`; `Type` không có union `int|str` sau ST-56) — tầng chạy vẫn kiểm. (6) `kiem()` của `vis-interp` coi
  `Var/List/Map` là **nhận** (thân hàm tự kiểm như hôm nay) ⇒ 71 ca `test/std` + differential không đổi.

- **VD-100** **Net/HTTP: socket thô ở host, giao thức ở `vis-interp` (ST-63..ST-66, vòng 11)**: (1) Thêm `NetHost` cạnh
  `FsHost` (`connect/listen/local_addr/accept/read/write/close`, handle = `str` mờ); `Host::net` default `None` ⇒
  **`VS0604`** cả hai đường. (2) **HTTP client nằm trong `vis-interp`** (module mới `net.rs`) ⇒ interp ↔ VM **cùng byte**,
  codegen **không** sửa (duyệt `std.*` theo tên). (3) API: `net.connect/connect_timeout/listen/local_addr/accept/
  accept_timeout/read/write/close`, `http.get/get_timeout` — tham số tuỳ chọn **tách tên hàm** (builtin bỏ đối số có tên);
  handle là `str`, **không** thêm variant `Ty`. (4) **Chỉ loopback** (`127.0.0.1`/`localhost`; host khác ⇒ `Err`) trong
  vòng này; **không** thêm `--net-mock` — ca net dựng **server loopback thật** trong test Rust (`bind("127.0.0.1:0")`);
  timeout ≤ 400 ms; coi `WouldBlock` **và** `TimedOut` là hết giờ. (5) **Không mã lỗi mới**: lỗi mạng = `Err(str)`,
  không host = `VS0604`, sai kiểu/arity = `VS04xx`/`VS0501`. (6) R1: `interp.rs` sát 1.500 dòng ⇒ **tách `impl Host`**
  hoặc phần net ra tệp riêng **trước** khi thêm. (7) Ca: ≥13 ca e2e (≥2 lỗi) + `--vm` **có cache và `--no-cache`**.

- **VD-101** **Gói + registry local (VS2 #6, ST-6x)**: (1) **Không mạng**: registry là **thư mục local** (mặc định `./registry`,
  `--registry`/`VIS_REGISTRY`); **không** crate mới, **không** semver range — phụ thuộc ghi **phiên bản chính xác**. (2) Giữ
  tiền tố **`pkg.`**: `use pkg.<tên>[.<mod>]`; tên gói `[a-z][a-z0-9_-]{0,31}` (không chứa `.`); cài vào
  `.vis/pkg/<tên>/<phiên bản>/`. (3) Manifest + lockfile = **JSON** (dùng `vis_interp::json`, không thêm dep); `files[]`
  **tường minh** (không glob), mỗi tệp có `sha256`; `tree_sha` = SHA-256 trên danh sách `(path, file_sha)` **sắp theo byte**,
  chuẩn hoá BOM/CRLF/đúng một LF cuối; **không** timestamp, **không** đường dẫn tuyệt đối trong lock. (4) Cài **tất định**:
  kiểm hết rồi mới ghi; thư mục đích tạo lại từ đầu; ghi tmp + rename. `install` chỉ đọc lock (không cần registry).
  (5) `vis run` kiểm `file_sha` của tệp gói khi nạp ⇒ lệch = **`VS0701`** "gói bị sửa". (6) **Không mã lỗi mới**; lỗi CLI
  `vis pkg` = exit 2 + `vis pkg: …`. (7) Mọi `manifest.json` commit kèm `.license` (R3); `.gitignore` thêm `.vis/`.

- **VD-102** **LSP `vis lsp` (VS2 #5, ST-6x)**: (1) **Tự viết framing** `Content-Length` trên `stdin/stdout` nhị phân; **không**
  `println!`, **không** dùng `xuat_stdout`; thoát khi `exit` **hoặc EOF**. (2) JSON dùng lại `vis_interp::json` (decode) +
  `vis_interp::stdlib::json` (encode) — **không** thêm crate. (3) Nguồn chẩn đoán = `parse`+`lex`+`lower_into`+`check`
  (đúng đường `vis check`), gộp 3 danh sách, sắp `(span.start, code)`; severity Error=1/Warning=2; `source:"vis"`.
  (4) **Vị trí LSP = 0-based, UTF-16, tab=1** — tự viết `vi_tri_utf16`/`byte_tu_vi_tri`, **không** dùng `modules::vi_tri`
  (1-based, scalar, tab=4); tính trên văn bản đã chuẩn hoá NFC/CRLF→LF của `LexOutput::text`, ghi rõ giả định.
  (5) v1 **một tệp**: completion = từ khoá + item + symbol + builtin (khử trùng, sắp byte, cap 200); hover = chữ ký khai
  báo/từ khoá/symbol; **không** module-aware, **không** suy kiểu biểu thức (nợ). (6) Method lạ ⇒ `-32601`; JSON hỏng ⇒
  `-32700` (id `null`); trước `initialize` ⇒ `-32002`; `shutdown` ⇒ `null`; `exit` sau shutdown ⇒ 0, trước ⇒ 1.
  (7) ≥13 frame snapshot tất định (`file:///t.vis`, không timestamp/path tuyệt đối).

- **VD-103** **Quyền: `cap[Dir]`/`cap[Proc]`/`cap[Net]` cưỡng chế ở **biên host** (ST-72, ST-73)**: (1) Ngôn ngữ **đã** có cú
  pháp `@ path` + `(write)` trên tham số và `--explain-perms`; vòng này **cưỡng chế** thay vì chỉ in. (2) Cưỡng chế nằm ở
  **host** (`FsHost`/`NetHost` nhận **tập quyền của phiên**), **không** ở interp/VM ⇒ hai đường vẫn cùng byte và
  `viscript/` không chạm OS. (3) Vi phạm ⇒ **`VS0604`** (chưa hỗ trợ/chưa cấp quyền) với thông điệp nêu **quyền thiếu** +
  đường dẫn; **không** mã mới. (4) `--cap-mock <json>` cho test tất định (giống `--fs-mock`), **không** phải cờ thật để
  "xin quyền". (5) **Landlock thật** chỉ trên Linux ≥6.x: crate `vis-host-linux` (ngoài `viscript/`) `cfg(target_os =
  "linux")`; test chỉ biên dịch/chạy trên Linux và **ghi rõ** trên macOS là **chưa đo** — cấm mô tả là "đã có Landlock".

- **VD-104** **Đa nền tảng: bằng chứng theo **mức** (ST-74, ST-75)**: (1) `x86_64-pc-windows-msvc` + `wasm32-unknown-unknown`
  chỉ đòi **`cargo check` xanh** (biên dịch chéo), **không** tuyên bố "chạy trên Windows/wasm" khi chưa có CI/runner —
  báo cáo phải ghi **mức** (check/biên dịch/chạy). (2) `vis-host-windows`/`vis-host-wasm` là **crate khung** trong
  workspace, **không** nằm trong `viscript/`; phần lõi ngôn ngữ **không** được `cfg(target_os)`. (3) Bộ
  `host-conformance` ≥50 ca chạy **thật** trên macOS trong `viscript/test/host/` + harness Rust; 3 nền còn lại ghi rõ
  "chưa đo" cho tới khi có runner. (4) Không thêm dependency mới cho cross-check.

- **VD-105** **`Host` mở rộng: `NetHost` + quyền của phiên (ST-63..ST-66)**: (1) Thêm `NetHost` cạnh `FsHost` trong
  `vis-interp/src/builtin.rs` (handle = `str` mờ; `connect/listen/local_addr/accept/read/write/close`), `Host::net()`
  default `None` ⇒ **`VS0604`** cả hai đường. (2) **Giao thức HTTP nằm trong `vis-interp`** (`net.rs`) ⇒ interp ↔ VM cùng
  byte, codegen **không** sửa (`std.*` hạ theo tên). (3) **Chỉ loopback** trong vòng này; host khác ⇒ `Err`. (4) **Không**
  mã lỗi mới: lỗi mạng = `Err(str)`, không host = `VS0604`, sai kiểu = `VS04xx`. (5) Tham số tuỳ chọn **tách tên hàm**
  (`connect_timeout`, `get_timeout`) vì builtin **bỏ** đối số có tên. (6) **R1 trước, code sau**: `interp.rs` đang
  **1.486** dòng ⇒ task **ST-63 bắt đầu** bằng việc **tách** (`impl Host` đã ở `host.rs`; tách tiếp `eval_call`/phần
  runtime sang `interp/goi.rs` hoặc tương đương) rồi mới thêm net. (7) Ca net/http nằm ở `crates/vis-cli/tests/net.rs`
  + `http.rs` (server loopback thật trong test Rust), **không** đặt vào `test/sem` (sem không có host net) ⇒ con số
  sem **441** giữ nguyên; ghi rõ trong báo cáo.

- **VD-106** **`cap[...]` là kiểu thật, cưỡng chế **chỉ khi khai báo** (ST-72/ST-72b)**: (1) `cap[Dir|File|Net|Proc|Env|Term]` là
  kiểu **dựng sẵn** (`TypeRef{"cap",[X]}`), **không** người dùng tạo được giá trị — chỉ nhận qua **tham số `main`**; truyền
  qua tham số hàm/record bình thường. (2) `@ p"…"`/`@ "host"` + `(write)` **giữ** ở CST; **`vis run --dry`** in danh sách
  quyền khai báo (dạng `tên: cap[Dir] path (write)`) rồi thoát 0, **không** chạy chương trình. (3) **Cưỡng chế** nằm ở
  `HostFs`/`HostNet` (CLI): thao tác ngoài tập quyền ⇒ **`VS0604`** nêu **quyền thiếu** + đường dẫn/host; `write` cần cờ
  `(write)`. (4) **Chương trình KHÔNG khai `cap` ⇒ không hạn chế** (tương thích 854 test hiện có) — ghi rõ trong tài liệu và
  báo cáo; siết mặc định là **nợ** (phải có `--strict-caps` + cập nhật toàn bộ fixture). (5) `--grant <tên>=<đường dẫn>`
  **ghi đè** đường dẫn của quyền đã khai (không thêm quyền mới). (6) Landlock thật: **chỉ** Linux, crate ngoài `viscript/`;
  trên macOS **không** giả vờ — test `cfg(target_os="linux")`, báo cáo ghi "chưa đo trên macOS".

- **VD-107** **Landlock: chỉ **probe** + khung, **không** giả vờ (ST-73)**: (1) Máy phát triển là macOS ⇒ **không** có Landlock để
  đo; luật hiện hành **cấm `unsafe`** (R2, danh sách crate đóng **rỗng**) và **cấm thêm crate ngoài allowlist** ⇒ **không** thể
  gọi syscall Landlock trong vòng này. (2) Vì vậy ST-73 làm: **`vis run --sandbox-check`** — probe **chỉ đọc**
  (`/sys/kernel/security/lsm` trên Linux; macOS/khác ⇒ "không có"), in kết quả + exit 0, và khung crate `vis-host-linux`
  **ngoài** `viscript/` với `#[cfg(target_os = "linux")]` **chỉ đọc tệp** (không syscall), bằng chứng là
  `cargo check --target x86_64-unknown-linux-gnu` **xanh**. (3) **Cấm** viết trong báo cáo/tài liệu rằng "đã cưỡng chế bằng
  Landlock" — chỉ được nói "**cưỡng chế ở tầng runtime** (ST-72b) + **probe** Landlock (ST-73)". (4) Muốn Landlock thật: cần
  Agent 0 duyệt **một** trong hai: (a) cho phép crate `landlock` (MIT/Apache) trong allowlist, hoặc (b) cho phép `unsafe`
  **đóng** trong `vis-host-linux` + case âm; cả hai phải kèm runner Linux để **đo** — ghi thành **nợ có điều kiện**.

- **VD-108** **Vòng 12 — "nhất quán hoá" tính năng mới (ST-77..ST-82)**: (1) Sau vòng 11, các tính năng VS2 **chạy được** nhưng chưa
  **nhất quán** với phần còn lại của công cụ ⇒ vòng 12 đóng 5 lỗ: **`vis fmt`** cho cú pháp mới (`cap[...]`, `@`/`(write)`,
  `async`, `spawn`, `chan[T]`, `use pkg.*`) — **lossless + idempotent**; **`vis explain`/sổ mã** đủ **3 ngôn ngữ** cho
  `VS0504/0505/0506` + `--list` phủ **32 mã**; **`vis test`** chạy được khối `test` có `async`/`await`/`chan`; **LSP
  module-aware** (giải `use ./x` + `std.*` để **hết chẩn đoán giả**); **`--strict-caps`** (tuỳ chọn, **mặc định không đổi**).
  (2) **Không** đụng ngữ nghĩa đã khoá; mỗi task ≥6 ca, giữ **874 test/447 sem/parity 398/398/diff 0 lệch** (số sẽ tăng).
  (3) Vòng 12 **không** hứa: Landlock thật, runner Windows/Linux/wasm, TLS — đó là **điều kiện môi trường**, ghi ở R12 §5.

- **VD-109** **Vòng 13 — đóng nợ nhỏ + kiểm chứng sâu (ST-83..ST-88)**: (1) Ba nợ nhỏ phải đóng: `fs.exists` ngoài quyền **không**
  được trả `false` im lặng (⇒ `VS0604` như `read`), `assert <đk>` dạng **câu lệnh** (tài liệu dùng dạng này), `usage_explain`
  theo `--lang`. (2) `vis fmt` chuẩn hoá **thụt lề khối `cmd`** trong `.vish` (đổi 68 fixture **có lý do**, không nới luật).
  (3) **Kiểm chứng sâu**: fuzz **differential** mở rộng cho cú pháp mới (20.000 chương trình, **0 lệch**, seed ghim) và **cache không
  che lỗi** — chạy ≥500 ca ở 3 chế độ (`run`, `--vm`, `--vm --no-cache`) so **byte**; lệch ⇒ sửa, không ghi nợ.
  (4) Không thêm hạng mục VS2 mới trong vòng này.

- **VD-110** **Vòng 14 — đóng khoảng lệch cuối (ST-89..ST-92)**: (1) **`Ngoai` cho VM**: đây là **khoảng lệch interp ↔ VM duy nhất
  còn lại** (7 ca `.vish` dùng chương trình ngoài: VM trả `VS0604`) ⇒ phải chạy được ở **cả hai** đường, `--vm` SKIP **7 → 0**,
  parity **398 → 405**. Host vẫn là **CLI** (VM không chạm OS; `vis-vm` chỉ nhận trait). (2) `vis pkg`: lock **lưu `entry`** (bỏ
  quy ước dò tệp) + `vis pkg remove`; **lock cũ vẫn đọc được** (thiếu `entry` ⇒ fallback quy ước, ghi rõ). (3) LSP completion
  **sau `.`** theo **kiểu tĩnh** của biểu thức trước dấu chấm (field bản ghi + phương thức `impl`); không phá 34 ca LSP hiện có.
  (4) Không thêm hạng mục VS2 mới; **không** nới ngưỡng.

- **VD-111** **Vòng 15 — đóng nốt mô hình quyền + ngữ nghĩa kênh (ST-93..ST-95)**: (1) **`cap[Proc]`**: chương trình ngoài (`^cmd`,
  `sh`) chỉ chạy khi chương trình khai `cap[Proc]`; `--strict-caps` ⇒ **`VS0604`** nêu `cap[Proc]` + tên lệnh; **mặc định không
  đổi** (tương thích) — cùng chính sách với `cap[Dir]/cap[Net]`. (2) **Kênh**: `chan[T](0)` = **rendezvous** (gửi chờ tới khi có
  người nhận; người nhận chờ tới khi có người gửi) **không treo** khi lập trình đúng; `recv` sau `close` ⇒ `None`; `send` sau
  `close` ⇒ **`VS0504`**; task `Moi` còn sống khi chương trình kết thúc ⇒ **không** bị bỏ im lặng (chạy hết hoặc báo lỗi rõ).
  (3) **Không** thêm cú pháp mới; không nới ngưỡng.

- **VD-112** **Vòng 16 — nhất quán sổ mã + allowlist `cap[Proc]` + soak + bàn giao (ST-96..ST-99)**: (1) **Bất biến sổ mã**: mọi chuỗi
  `VS0xxx` trong `viscript/crates/**` phải **có** trong `all_codes()`; mọi mã trong sổ phải **được dùng** hoặc được khai là **dự
  phòng** trong danh sách có lý do (test tự động, không quét tay). (2) `--grant <tên>=<lệnh>` cho `cap[Proc]` = **allowlist** lệnh
  (so **đường dẫn tuyệt đối** hoặc **tên**), áp **chỉ** khi cờ được dùng; mặc định **không** đổi. (3) **Soak 30 phút** chạy **tuần
  tự** (không song song với build/cổng khác theo AGENTS §6): cổng `--fast` ×2, diff 100k ×2, host suite ×2, cache ×2 — ghi log
  từng lần; flake ⇒ `T-INFRA` + log, **không** retry để lấy xanh. (4) `ROADMAP.md` là **bàn giao**: 8 hạng mục VS2 × **mức thật**
  × **điều kiện đóng** + **cách chạy tiếp**; **cấm** ngôn từ vượt mức đã đo.

- **VD-113** **Vòng 17 — đầu ra máy đọc + LSP hoàn thiện nhỏ + pkg update (ST-100..ST-103)**: (1) **`--json`** cho `vis check` và `vis test`:
  JSON **ổn định** (khoá cố định: `file`, `diagnostics[{code,line,col,message}]`, `ok`; với `test`: `{total,ok,failed,cases[{name,ok,message}]}`), **chỉ**
  in JSON ra **stdout**, log khác ra stderr; **không** cờ ⇒ TAP/định dạng hiện tại **không đổi**. (2) LSP: **hover trường** (record field),
  `didClose` **dọn** cache, **cache** kết quả nạp module giữa `didChange` (đo < 2 ms sau lần đầu, có ca). (3) `vis pkg update` **không mạng**:
  cài lại từ registry **cục bộ** nếu nguồn không đổi ⇒ **sha không đổi**; `pkg list --json`. (4) Vẫn **không** nới ngưỡng; **không** thêm crate.

- **VD-114** **Vòng 18 — HTTP thật hơn + timeout ca test + R1 headroom (ST-104..ST-107)**: (1) HTTP client (loopback) phải xử lý
  **`Transfer-Encoding: chunked`** (ghép thân đúng, kể cả chunk mở rộng `;ext`), **redirect 3xx** (tối đa **5** lần, giữ phương thức GET,
  `Location` tương đối **và** tuyệt đối, vượt 5 ⇒ lỗi rõ), **`Connection: close`** (đọc tới EOF) — heuristic **không** treo, có timeout.
  (2) `vis test --timeout <ms>` (mặc định **không** đổi): ca vượt hạn ⇒ **`not ok`** với thông điệp "quá hạn", **giết** tiến trình con,
  không rò; áp cho cả hai đường. (3) Tách `lower/cmd.rs` để **R1 max giảm** (mục tiêu ≤1.200 dòng) **không** đổi hành vi (447 sem +
  parity **398+49** bất biến). (4) Không nới ngưỡng; không thêm crate.

- **VD-115** **Vòng 19 — hạ R1 max + HTTP/Location + `--json` chẩn đoán theo tệp (ST-108..ST-111)**: (1) Tách `lower.rs` để **R1 max ≤ 1.200**
  dòng; **thuần refactor**, bằng chứng = **checksum 1045 dòng test** + **sha256 differential** trước = sau. (2) HTTP: resolve `Location`
  **đầy đủ** (đường dẫn tương đối có `..`/`.`; query-only `?x=1`; fragment bỏ), ghi rõ **chunk trailer bị bỏ qua**, hỗ trợ **`HEAD`**
  (không đọc thân). (3) `check --json`: thêm `severity` (`error`/`warning`) và **`file` đúng tệp phụ** cho chẩn đoán module (nợ từ ST-101);
  **khoá cố định**, TAP **không đổi**. (4) Không nới ngưỡng; không thêm crate.

- **VD-116** **Vòng 20 — đóng nợ correctness chẩn đoán đa tệp, rồi soak cuối + R20 (ST-112..ST-113)**: (1) **Chẩn đoán trước, sửa sau**:
  phải **tái hiện** ca "câu lệnh cấp tệp ở tệp chính bị gắn sang module" và **chỉ đúng** dòng mã gây ra (không đoán). (2) Nếu cách sửa
  **≤150 dòng** và không đổi API công khai ⇒ **làm**, với ca cho **cả hai chiều** (lỗi tệp chính ở cấp tệp; lỗi module) và 1056 test
  xanh. (3) Nếu lớn hơn ⇒ **dừng**, viết **kế hoạch ≤30 dòng** (thiết kế `Span` có file-id hoặc offset base + bản đồ), **không** làm
  nửa vời — và ghi thành nợ đầu bảng ROADMAP. (4) Sau đó **soak ≥10 vòng tuần tự** (gate/diff/host/cache) + **R20**, cập nhật
  `ROADMAP.md` theo trạng thái cuối. (5) Không nới ngưỡng; không thêm crate.

- **VD-117** **Vòng 21 — đóng nợ correctness `Span` (ST-114a/b, ST-115)**: theo kế hoạch `st112-ke-hoach.md`. (1) `Span` thêm **`file: u16`**
  **mặc định 0** ⇒ `Span::new(start,end)` cũ **không đổi** hành vi (một tệp = file 0). (2) Luồn `file` qua **lex → parse → lower** bằng
  **một** tham số `file` (không dùng thread-local) — mọi `Span` sinh ra trong tệp thứ i mang `file = i`. (3) Chẩn đoán map **theo
  `file`** (không đoán theo khoảng span) và `vi_tri`/thông điệp dùng **đúng văn bản** của tệp đó; **TAP và `--json`** phải cùng đúng.
  (4) **Bytecode (`VBC1`) không đổi** (span trong một tệp giữ nguyên); LSP dùng `file` để chọn tài liệu. (5) Chia 2 task ≤3 h; task 1
  **không** đổi hành vi (checksum + diff sha256 trùng), task 2 mới đổi chẩn đoán + thêm ca. (6) Không nới ngưỡng; không thêm crate.

- **VD-118** **Vòng 22 — hạ R1 lần cuối, LSP đa tệp, soak chốt + R22 (ST-116..ST-118)**: (1) Tách `parse/mod.rs` (**1.452**) để **R1 max ≤1.200**,
  **thuần refactor**, bằng chứng checksum 1062 test + sha256 differential trước = sau. (2) LSP: xác nhận **chẩn đoán chỉ publish cho tài
  liệu đang mở** và **đúng tệp** nhờ `Span.file`; ca (a) tài liệu mở là tệp chính có module lỗi, (b) tài liệu mở **là** module, (c) hai
  tệp cùng lỗi; nếu sai ⇒ sửa (không "đúng một nửa"). (3) **Soak cuối 10 vòng** tuần tự + **R22** + `ROADMAP.md` chốt trạng thái cuối
  (kèm **điều kiện đóng 100%**). (4) Không nới ngưỡng; không thêm crate.

### VD-119 · Quy trình: Agent 0 soạn, Agent 2 dán (2026-11-16)
Agent 0 không ghi repo trực tiếp. Văn bản trong tệp QUYET-DINH-A0-<ngày>.md được Agent 2 dán nguyên văn vào tệp Agent 0 sở hữu, commit riêng [ST-00] ghi "theo QUYET-DINH-A0-<ngày>". Không diễn giải; không áp được nguyên văn thì BLOCKED. Task xong ghi "CHỜ DUYỆT Agent 0 · <hash> · <số đo>"; chỉ Agent 0 ghi APPROVED qua tệp quyết định kế tiếp. Task phụ thuộc được chạy tiếp khi task trước ở trạng thái CHỜ DUYỆT.

### VD-120 · Runner Linux bằng container + Landlock có điều kiện (2026-11-16)
Runner Linux chạy bằng container trên máy dev. Không tự cài phần mềm hệ thống; thiếu công cụ thì BLOCKED-ENV và báo người dùng. Crate `landlock` được duyệt CÓ ĐIỀU KIỆN: chỉ trong vis-host-linux, chỉ cfg(target_os="linux"), toàn bộ phụ thuộc bắc cầu có giấy phép thuộc allowlist D2-21 (kiểm bằng cargo metadata, ghi log); một phụ thuộc ngoài allowlist thì BLOCKED. Không thêm `unsafe` trong code của ta. Landlock bật bằng cờ `--sandbox` (opt-in trong VS2); nền không hỗ trợ trả exit 3 như --sandbox-check. Ca âm phải chứng minh KERNEL chặn (tiến trình con bỏ qua FsGuard vẫn bị EACCES), không phải runtime chặn. Trước khi có runner: văn bản "cưỡng chế ở tầng runtime; kernel sandbox chưa có".

### VD-121 · wasm và Windows: chỉ cargo check trong VS2 (2026-11-16)
Mức bằng chứng wasm32 và windows-msvc trong VS2 là "biên dịch kiểm tra" (VD-104). Không tuyên bố hỗ trợ. VS3 xét đưa vis-interp/vis-types chạy được trên wasm32-wasip1 như thư viện nhúng.

### VD-122 · HTTP: chưa có HTTPS trong VS2 (2026-11-16)
HTTP client chỉ nói HTTP/1.1 loopback, không TLS, không HTTP/2, không keep-alive. Đây là trạng thái "chưa có", không phải "ngoài phạm vi vĩnh viễn". rustls được xét ở VS3 sau khi có runner Linux.

### VD-123 · Async và chan(0) trong VS2 (2026-11-16)
Ngữ nghĩa VS2: async là task chạy tuần tự, một luồng, run-to-completion khi join; không song song; không suspend giữa thân hàm; không GC. chan(0) bị từ chối lúc tạo bằng VS0505 (exit 1). Cả hai là trạng thái VS2, được văn bản hoá trong thiết kế và ROADMAP. Async đồng thời thật + rendezvous + select/timeout là dự án VS3 có tài liệu thiết kế riêng do Agent 0 viết; không làm trong task 3 h.

### VD-124 · Siết caps mặc định cho chương trình dùng gói (2026-11-16)
Nếu đồ thị import của chương trình có ít nhất một module `pkg.*`, chế độ strict-caps bật mặc định. Thoát bằng cờ tường minh `--no-strict-caps`, khi đó in một dòng cảnh báo ra stderr (không ra stdout). Chương trình không dùng gói: hành vi giữ nguyên. Siết mặc định toàn cục là nợ VS3.

### VD-125 · Semver ranges + resolver tối giản (2026-11-16)
Phiên bản: MAJOR.MINOR.PATCH số nguyên; prerelease/build metadata bị từ chối có chẩn đoán. Ràng buộc: `^`, `~`, `=`, `>`, `>=`, `<`, `<=`; dấu phẩy là AND; không có toán tử thì hiểu là `^` (giống Cargo, kể cả luật ^0.x). Mỗi gói có đúng một phiên bản trong đồ thị (VS2). Resolver tất định: chọn phiên bản cao nhất thoả mọi ràng buộc, quay lui khi xung đột; không thoả được thì mã lỗi MỚI trong khối VS07xx kèm chuỗi ràng buộc gây xung đột. Lock hiện có còn thoả ràng buộc thì giữ nguyên (`pkg install` không đổi lock); `pkg update` mới giải lại. Lock định dạng cũ được đọc và ghi lại định dạng mới; ghi lại hai lần cho ra cùng byte. Không mạng, không chữ ký (VS3).

### VD-126 · LSP publish một tài liệu (2026-11-16)
LSP chỉ publish chẩn đoán cho tài liệu client đang mở; lỗi ở module phụ được báo qua vị trí import trong tài liệu đang mở. Nợ #104 đóng ở trạng thái "không làm, có lý do": publish cho URI client chưa mở gây hiển thị không nhất quán giữa các client.

### VD-127 · Span tổng hợp phải mang file (2026-11-16)
Mọi node HIR do lower sinh ra (không copy từ CST) phải mang file-id của node nguồn sinh ra nó. Có test duyệt HIR của mọi fixture đa tệp: không node nào thuộc module phụ có file = 0.

### VD-128 · R1 chỉ có một ngưỡng: ≤1.500 dòng/tệp (2026-11-16)
Bỏ mục tiêu nội bộ ≤1.000. Không mở task tách tệp chỉ để đạt con số không có trong luật.

### VD-129 · Đóng băng API std và cam kết tương thích (2026-11-16)
Ba danh sách là cam kết tương thích: hàm std (FUNCS), mã chẩn đoán (all_codes), thẻ VBC1. Chúng được ghi thành tệp viscript/agent/API-STD.txt, API-CODES.txt, API-VBC1.txt (Agent 0 sở hữu sau lần tạo đầu). Test so code với tệp. Luật: không xoá, không đổi tên, không đổi ý nghĩa, không tái dùng số mã/số thẻ. Thêm mới chỉ khi có VD nêu đích danh tên được thêm. Đổi ý nghĩa thẻ VBC1 bắt buộc lên định dạng mới (VBC2). Khoá cache bytecode phải đổi khi bảng thẻ đổi. VBC1 không mang file trong Span ở VS2.

### VD-130 · Kết luận review độc lập vòng 10+ (2026-11-16)
(a) Value::Cap không được tạo từ dữ liệu ngoài (json.decode, đọc tệp, đối số) và không lộ quyền khi in/serialize. (b) FsGuard: kiểm TOCTOU; sửa nếu không cần `unsafe`, nếu cần thì ghi giới hạn có văn bản trong HOST-BAO-DAM.md. (c) `--grant` cho Proc so khớp ĐƯỜNG DẪN TUYỆT ĐỐI ĐÃ CANONICALIZE, không so basename. Giá trị grant không có `/` được phân giải qua PATH một lần lúc khởi động thành đường dẫn tuyệt đối. (e) json_esc phải qua fuzz chuỗi ngẫu nhiên 100k, đối chiếu json.decode và python3 json.loads.

### VD-131 · Định nghĩa VS2 đóng + 5 câu bắt buộc (2026-11-16)
VS2 đóng khi ST-119..ST-133 DONE hoặc BLOCKED-ENV có văn bản và R23 xác nhận số đo. Mỗi hạng mục ROADMAP ở một trong ba trạng thái: đã đóng (đo được) / chưa có trong VS2 (chuyển VS3, có văn bản) / chờ môi trường (BLOCKED-ENV, có lệnh tái lập). Mọi báo cáo R## từ R23 phải có nguyên văn 5 câu: "Async không song song." "Kênh không rendezvous." "Net không TLS." "Quyền chỉ siết mặc định khi dùng gói." "LSP chỉ publish tài liệu đang mở." Câu nào hết đúng thì Agent 0 sửa bằng VD mới.

### VD-132 · Ca test phụ thuộc nền (2026-11-16)
Được phép đổi kỳ vọng của một ca test theo nền CHỈ khi sản phẩm đúng hợp đồng trên cả hai nền và ca đang viết cứng cho một nền. Điều kiện: không xoá ca, không ignore, không cfg bỏ ca trên một nền; mỗi nhánh nền khẳng định giá trị cụ thể; diff dán vào báo cáo; mỗi ca được Agent 0 nêu đích danh. Ca được duyệt: kỳ vọng tiền tố tên chương trình trong thông báo grep; kỳ vọng exit của --sandbox-check (macOS 3, Linux có Landlock 0 kèm dòng báo ABI). Mọi ca khác cần VD mới.

### VD-133 · ST-130: chi tiết siết caps khi dùng gói (2026-11-16)
Bổ sung VD-124. --strict-caps cùng --no-strict-caps là lỗi dòng lệnh, exit 2, chẩn đoán nêu tên hai cờ. Khởi tạo quyền chạy sau khi nạp module; thứ tự lỗi mới được khoá bằng một ca. --no-strict-caps khi không dùng gói thì không cảnh báo. Sửa lỗi có sẵn trong lower (module std bị VS0301 khi bật chế độ chặt) được làm trong ST-130, có ca đỏ riêng. Chương trình có use nhưng không dùng pkg.* phải giữ hành vi cũ về caps; có ca khoá. Nợ #115 (vis test và host suite chưa siết) chuyển VS3. Nợ #114 chuyển VS3 chỉ khi ca khoá trên xanh.

### VD-134 · Runner Linux tái lập được (2026-11-16)
Thành phần toolchain (rustup target, component) được cài trong runner, nhưng chỉ qua Dockerfile; digest image ghi vào log mỗi lần build. Không cài tay trong container đang chạy. Hạng mục ROADMAP chỉ được ghi "Linux: chạy thật" khi cargo test --workspace và gate --fast PASS trên runner; host suite xanh một mình không đủ.

### VD-135 · Gói: ghim khi thêm và khi migration (2026-11-16)
pkg add tên@X.Y.Z ghi vào manifest ràng buộc có toán tử tường minh "=X.Y.Z" (không ghi chuỗi trần), để không trái luật "không toán tử là ^" của VD-125. Migration lock cũ ghi "=<version>" để giữ đúng hành vi cũ. Giấy phép unicode-ident đã hợp lệ theo QD §9.3 và VD-33; không cần duyệt thêm.

### VD-136 · Máy đo thật qua SSH: Linux (ps) và Windows (psvn) (2026-11-16)
Sửa VD-134: runner Linux là máy ps truy cập qua SSH từ mps; container không bắt buộc. Tái lập bằng script trong repo (viscript/infra/<nền>/setup.md hoặc .sh) ghi rõ toolchain và target đã cài. Mỗi lần chạy, log đầu tiên ghi: tên máy, phiên bản OS, kernel/build, rustc -Vv, rustup target list --installed, hash commit đang đo. Thành phần toolchain (rustup target, component) được cài qua rustup và ghi vào script; phần mềm hệ thống (Build Tools, gói OS) không tự cài, thiếu thì BLOCKED-ENV.
Windows (psvn) trong VS2 chỉ ĐO: cargo test --workspace, host suite và gate nếu chạy được. Không sửa code, không đổi kỳ vọng test, không thêm cfg cho Windows. Repo clone với core.autocrlf=false; target dir riêng; mã nguồn đồng bộ qua git đúng hash đang đo. Trạng thái Windows trong VS2 vẫn là "chưa có" (VD-121); R23 ghi thêm "đã đo, N lệch" kèm danh sách. Việc sửa cho Windows do Agent 0 mở task ở VS3.

### VD-137 · Phân vai dán văn bản và đối chiếu duyệt (2026-11-16)
Bổ sung VD-119. Agent 1 được dán nguyên văn văn bản quyết định của Agent 0 vào các tệp Agent 0 sở hữu, và đối chiếu hàng CHỜ DUYỆT với tiêu chí của chính hàng đó; commit [ST-00] như VD-119. Agent 1 không sửa mã, test, infra, không chạy cổng. Khi một tệp quyết định gửi cho cả Agent 1 và Agent 2, chỉ Agent 1 dán. Dòng duyệt ghi đúng người đã đối chiếu. Các dòng APPROVED ngày 2026-11-16 ghi "Agent 2 đã đối chiếu" thực tế do Agent 1 đối chiếu; đính chính bằng một dòng trong QUEUE, không sửa dòng cũ.

### VD-138 · Chấp nhận runner Linux qua SSH cho ST-121 (2026-11-16)
ST-121 đạt bằng backend SSH tới máy ps (kernel thật, lsm có landlock, Landlock ABI 8, log thăm dò có sha256), thay cho tiêu chí container. viscript/infra/linux/Dockerfile giữ lại, ghi rõ chưa build, không phải đường đo của VS2. Bằng chứng Landlock của ST-132 tính cho hạng mục 4 sau khi đủ 3 ca âm ở tiến trình con.

### VD-139 · Agent 1 thêm chú thích theo văn bản quyết định (2026-11-16)
Bổ sung VD-137. Agent 1 được thêm dòng chú thích vào tệp không phải mã (Dockerfile, script cấu hình, tài liệu infra) CHỈ khi văn bản quyết định của Agent 0 trích nguyên văn dòng đó; không đổi hành vi, không build, không chạy. Khi câu chữ quyết định xung đột luật có sẵn (ví dụ SPDX dòng đầu), luật có sẵn thắng; Agent 1 đặt ở vị trí gần nhất hợp luật và báo lại. Dòng CHƯA DÙNG ở dòng 2 của viscript/infra/linux/Dockerfile (commit 09c8f999) hợp lệ theo VD này.

### VD-140 · Cập nhật công cụ trên psvn cho ST-136 (2026-11-16)
Người dùng cho phép cập nhật công cụ trên psvn. Phạm vi: (1) Rust toolchain qua rustup, ĐÚNG phiên bản đang đo trên ps (hoặc bản ghim trong rust-toolchain.toml nếu có), không lấy bản mới nhất; ghi vào script tái lập trong viscript/infra/windows/. (2) Visual Studio Build Tools chỉ cập nhật khi build lỗi do phiên bản cũ, có log lỗi trước. Không cập nhật phần mềm khác. Trình cài đòi khởi động lại máy thì dừng, BLOCKED-ENV, báo người dùng; không reboot qua SSH. Log ghi phiên bản trước và sau của mọi thứ đã cập nhật.

### VD-141 · Rò rỉ TOCTOU đường đọc trên Linux là lỗi chặn VS2 (2026-11-16)
R23 đo được rò rỉ 798/50 000 ở quyen::tests::dua_doi_symlink_lap_co_gioi_han trên Linux (macOS 0/50 000). Đây là lỗi sản phẩm trái bất biến đường đọc trong HOST-BAO-DAM.md, sửa trong VS2 bằng ST-137, không chuyển VS3. Hướng ưu tiên: kiểm đường dẫn thật của handle đã mở qua /proc/self/fd, đọc từ chính handle; /proc không dùng được thì từ chối. Không unsafe, không crate mới. Cho tới khi ST-137 APPROVED: hạng mục 4 Quyền ghi "macOS đóng, Linux chưa đóng", và HOST-BAO-DAM.md mang đoạn giới hạn tạm thời.

### VD-142 · Báo cáo đã commit không sửa; đính chính ở báo cáo kế tiếp (2026-11-16)
Báo cáo R## đã commit là bản ghi lịch sử, không sửa nội dung. Sai sót được đính chính bằng một dòng trong QUEUE ngay khi phát hiện, và báo cáo kế tiếp có mục "Đính chính R##" liệt kê từng điểm: câu sai, câu đúng, căn cứ.

### VD-143 · Task chốt không lật DONE trước điều kiện (2026-11-16)
Task chốt (báo cáo R##, chốt VS#) chỉ được lật DONE khi mọi hàng nó phụ thuộc theo thứ tự trong tệp quyết định đã DONE, APPROVED hoặc BLOCKED-ENV có văn bản. Trước khi lật, Agent 2 dán danh sách các hàng đó kèm trạng thái vào báo cáo. ST-133 (R23) lật DONE sớm được giữ, nhưng R23 chỉ là báo cáo chốt đợt 23, không chốt VS2; chốt VS2 là ST-138 (R24).

### VD-144 · Ca symlink_canonicalize_hai_phia không phụ thuộc chương trình hệ thống (2026-11-16)
Trên ps, /usr/bin/echo là coreutils multicall, từ chối chạy khi argv[0] khác tên; ca đỏ vì môi trường. Cho phép sửa CHÍNH CA: đích của symlink là một binary do workspace build (env!("CARGO_BIN_EXE_<tên>") của một binary đã có trong workspace); không có binary phù hợp thì dùng /bin/sh -c. Symlink do ca tự tạo trong thư mục tạm. Giữ nguyên hai chiều khẳng định: grant qua symlink chạy được; grant đường dẫn thật chặn bản sao cùng tên. Không xoá ca, không ignore, không cfg-bỏ, không nới ngưỡng; chú thích ca ghi lý do và VD-144. Không áp cho ca nào khác.

### VD-145 · macOS không có fd→path trong VS2 (2026-11-16)
Trên macOS, std an toàn không lấy được đường dẫn thật của handle đã mở. VS2 giữ bước xác nhận (dev, ino) trước/sau canonicalize của ST-124 trên macOS; đo 0/50 000 nhưng KHÔNG bảo đảm theo cơ chế. Giới hạn ghi trong HOST-BAO-DAM.md. Sửa cần libc hoặc unsafe, là việc VS3, phải có VD riêng. Hạng mục 4 Quyền đóng trong VS2 với giới hạn này.

### VD-146 · Windows trong VS2: chỉ đo, cổng chạy được (2026-11-16)
Giữ VD-121, VD-136: Windows chỉ đo, không tuyên bố hỗ trợ, không sửa sản phẩm cho Windows. Cho phép ba việc hạ tầng: (1) rustup target add wasm32-unknown-unknown và x86_64-unknown-linux-gnu trên psvn, ghi vào viscript/infra/windows/, theo VD-140; (2) viscript/test/doc-examples/doc_examples.py đọc ống bằng encoding="utf-8"; (3) crates/vis-cli/tests/sandbox.rs: xử lý biến không dùng khi mọi nhánh cfg(target_os) bị loại, bằng cách dùng biến trong nhánh mặc định hoặc đặt allow(unused) đúng một biến; không đổi kỳ vọng, không xoá ca, không ignore, không cfg-bỏ ca. Các nhóm lệch /bin/*, escape \, CRLF, tiến trình là việc VS3.

### VD-147 · Quy trình sau đợt 24 (2026-11-16)
(1) Bổ sung VD-137: Agent 1 được chạy các bước kiểm tài liệu chỉ đọc (check_report.py, doc_examples.py --gate) và ghi kết quả vào báo cáo; vẫn không chạy cargo, clippy, L0, gate, soak. (2) Dòng APPROVED phải ghi người đối chiếu và căn cứ. (3) Sổ nợ duy nhất viscript/agent/SO-NO.md: một số một nghĩa, không tái dùng số; cột: số, mô tả, nguồn (task/báo cáo), trạng thái (mở / đóng ở task X / chuyển VS3 / không làm có lý do). Số trùng nghĩa cũ được tách bằng hậu tố (#109a, #109b), không đánh lại số. Báo cáo từ R25 trích nợ bằng số trong sổ này. (4) Mỗi báo cáo ghi hash commit đo cho TỪNG nền. (5) Log bằng chứng không được ghi đè log của task khác; mỗi lần chạy công cụ dài (soak, host suite) ghi tệp mang ID task đang chạy.

### VD-148 · Chốt VS2 (2026-11-16)
R24 chốt VS2 về kỹ thuật cho 7/8 hạng mục. Hạng mục 7 Đa nền tảng: Linux ở trạng thái "chờ" theo VD-134 tới khi cargo test và gate --fast PASS trên ps (ST-140); R25 ghi kết quả. Windows chỉ đo (VD-146), wasm chỉ check (VD-121). VS3 không bắt đầu tính năng cho tới khi Agent 0 ban hành tài liệu thiết kế VS3; trong lúc chờ, Agent 2 chỉ làm việc dọn dẹp, đo và nợ VS2.

### VD-149 · Cấp số nợ 142/143 (2026-11-17)
Nợ #113b (pkg update ghim mọi gói khác; nguồn JOURNAL/st129) nhận số 142. Nợ #114b (NamePolicy::Strict cho mọi tệp có use; nguồn JOURNAL/st130) nhận số 143. Không đánh lại số cũ; SO-NO.md ghi thêm dòng ánh xạ #113b→#142, #114b→#143; các báo cáo sau trích theo số mới, kèm số cũ trong ngoặc.

### VD-150 · Windows: UTF-8 cho mọi lần gọi python của cổng (2026-11-17)
Bổ sung VD-146. Được phép sửa HẠ TẦNG CỔNG để Windows đo được: đặt PYTHONIOENCODING=utf-8 và PYTHONUTF8=1 cho MỌI lần gọi python3 trong bin/vis-dev (gồm doc-examples, check_report, self-test) và trong test/fixtures/run_fixtures.sh nếu cần; không đổi kỳ vọng, không xoá ca, không ignore, không cfg-bỏ ca, không sửa sản phẩm. Điều kiện: gate macOS và Linux vẫn PASS với cùng số bước; trên psvn bước self-test phải 12/12 và cổng phải đi QUA bước 5 (dừng ở bước nào thì ghi đúng bước đó). Mọi ca lệch còn lại của Windows vẫn thuộc VS3.

### VD-151 · VS2 đóng về kỹ thuật (2026-11-17)
VS2 đóng về kỹ thuật với 8/8 hạng mục: (1) Async — xong phạm vi VS2, "không song song" (VD-123); (2) Kênh — xong phạm vi VS2, "không rendezvous" (VD-123); (3) Net/HTTP — xong phạm vi VS2, "không TLS/HTTP2/keep-alive" (VD-122); (4) Quyền — đóng, macOS có giới hạn văn bản (VD-145), Linux có Landlock thật + TOCTOU đường đọc 0/50 000; (5) LSP — đóng (VD-126); (6) Gói — đóng phạm vi VS2, mạng/chữ ký VS3 (VD-125); (7) Đa nền tảng — Linux "chạy thật" (cargo test 1162/0 + gate --fast PASS trên ps tại 02fb7320), Windows "chỉ đo" (VD-146), wasm "chỉ check" (VD-121); (8) Hợp chuẩn host — đóng (54/54 trên macOS và Linux). 5 câu bắt buộc VD-131 vẫn nguyên văn. Từ đây không mở task VS2 mới trừ nợ trong SO-NO.md, đính chính tài liệu và báo cáo vòng. Tính năng VS3 chỉ bắt đầu khi Agent 0 ban hành tài liệu thiết kế VS3.

### VD-152 · Sửa tên gốc khi check để `--syntax` lọc đúng trên Windows (2026-11-17)
`crates/vis-cli/src/cli/check.rs` tách tên gốc của tệp đang check bằng `rsplit('/')`; trên Windows đường dẫn dùng `\` nên `phu` luôn khác rỗng và bộ lọc `--syntax` (QD §7.4) giữ chẩn đoán KIỂU, làm khối tài liệu đỏ. Cho phép sửa TỐI THIỂU: tách tên gốc theo CẢ hai dấu (`/` và `\`) trong MỘT hàm nhỏ có tên rõ nghĩa, dùng ở chỗ cũ; kèm ≥3 ca khoá (đường dẫn Windows `C:\x\a.vis` ⇒ tên gốc `a.vis`; đường dẫn Unix `d/a.vis` ⇒ `a.vis`; tên không có dấu ⇒ nguyên vẹn) chạy được trên MỌI nền. Không đổi hành vi Unix; không sửa ca nào khác; không đổi định dạng chẩn đoán; không `unsafe`/crate mới. Điều kiện đóng: trên psvn self-test **12/12** và cổng đi QUA bước 5 (dừng ở đâu ghi đúng đó); macOS gate PASS 10/10, L0 1159/0; Linux gate PASS 10/10.

### VD-153 · Cấp số #144 + mở điều tra VS3 (2026-11-17)
Vế còn lại của nợ #133 (`read_dir` trên thư mục **có quyền ghi**, đối chứng dương) nhận số **#144**, xử ở ST-154. Cho phép mở **ST-155**: điều tra **CHỈ ĐỌC** để chuẩn bị tài liệu thiết kế VS3 (async · kênh · TLS · quyền ghi · Windows): không sửa mã, không thêm crate, không đổi hành vi; kết quả là một tài liệu trong `agent/JOURNAL/` có dẫn chứng tệp/dòng và câu hỏi mở cho Agent 0. **Không** bắt đầu tính năng VS3 (VD-148 giữ nguyên).

### VD-154 · Đính chính hiến chương VS3 + #143 → VS3 + số #145 (2026-11-17)
(a) Hiến chương VS3 §5 (17) ghi nợ đường GHI là `#142/#109b` — **sai**: `#142` là `pkg update` ghim (VD-149); đúng là **`#140/#109b`**. (b) Nợ `#143` (`NamePolicy::Strict`) **chuyển VS3**: điều kiện VD-133 đã thoả (ca `st130b` xanh từ đợt 23) ⇒ sổ **90 mở / 31 đóng / 21 VS3**. (c) Cấp **#145** = "chú thích `rt.rs` nói sai đường chặn `chan(0)`: tyck không kiểm `cap`, runtime mới chặn (exit 1)"; **mở**, xử khi làm VS3 kênh. R27 đính chính số 91/20 của R26.

### VD-155 · Đính chính số sổ sau ST-154 (2026-11-17)
Sổ sau ST-154 là **146 mục = 91 mở / 32 đóng / 21 VS3 / 1 không làm / 1 điều hướng** (thêm #144 đã đóng và #145 mở). Con số "90 mở/31 đóng/21 VS3" trong VD-154(b) đúng cho sổ **144 mục trước khi thêm hai dòng đó**; R27 trích **146/91/32/21**. Không xoá dòng lịch sử nào; `st154-kiem-trung.log` (0 trùng, dải 1..145) là bằng chứng.

### VD-156 · Đợt 28: parity argv (#110) + một nguồn đếm khối (#129) (2026-11-17)
Cho phép sửa TỐI THIỂU hai nợ: (a) #110 — VM phải đưa `argv` (`list[str]`) vào **tham số không-`cap`** của `main` **như interp** (`interp.rs:221-234`), có ca đỏ-trước chạy cả hai đường và so byte; (b) #129 — gom danh sách **số dòng khối tài liệu** đang chép ở 4 tệp test về **một nguồn** duy nhất (một hằng/module dùng chung), không đổi số kỳ vọng. Không đổi ngữ nghĩa khác, không nới ngưỡng, không `unsafe`/crate mới; gate macOS PASS (L0 1175/0 hoặc số thật nếu thêm ca) là điều kiện commit. Không mở task VS3.

### VD-157 · Hợp thức hoá quyết định đêm 17/11 + giới hạn quyền ban hành thay Agent 0 (2026-11-17)
Công nhận các quyết định 17, 17b, 17c, 17d do Agent 1 ban hành thay Agent 0 trong đêm 17/11. Từ đợt 29, khi Agent 1 đóng vai Agent 0 qua đêm chỉ được ban hành: nợ nhỏ không cần thiết kế, việc sổ, đo đạc, báo cáo. Phải để lại cho Agent 0 (ghi BLOCKED-CHO-A0): thiết kế/API mới, chính sách crate/unsafe/giấy phép, đổi kỳ vọng ca cũ, đổi ngưỡng hoặc cách đo ngưỡng. Việc đổi kỳ vọng ca `cap_khong_lo::doi_so_dong_lenh_la_str` (ST-157) được công nhận hồi tố.

### VD-158 · Ca đua symlink đỏ 1/50 000 KHÔNG xếp T-INFRA (2026-11-17)
Sự cố R28 §5 #1 (`quyen::tests::dua_doi_symlink_lap_co_gioi_han`, đỏ **1/50 000**) là **tín hiệu có thể là lỗi thật** (TOCTOU đường đọc), không phải nhiễu hạ tầng cho tới khi chứng minh được. Đổi loại thành **CHƯA-PHÂN-LOẠI**; mở nợ **#146**; điều tra ở **ST-163** (500 000 vòng trên macOS và `ps`, phân loại (a) lỗi test / (b) lỗi thật; không tái hiện trong 3 h ⇒ thêm chẩn đoán vào thông điệp lỗi, giữ #146 mở, ghi số vòng đã chạy).

### VD-159 · Ca thời gian đo min của 5 lần (2026-11-17)
Ngưỡng **100 ms** của `lsp::module::tests::hieu_nang_nap_module_5_tep` **giữ nguyên**. Đổi phép đo từ 1 lần sang **min của 5 lần chạy** + ghi load average vào log. Agent 0 duyệt rõ: đây là **đổi cách đo**, không tính là nới ngưỡng. Làm ở **ST-164**; điều kiện: 3 lần gate liên tiếp xanh ca này.

### VD-160 · VS3 mục 1: heap riêng mỗi luồng, bộ đếm không atomic (2026-11-17)
Chọn: mỗi luồng worker có **heap riêng**, bộ đếm tham chiếu **không atomic**; **loại** Arc/Mutex toàn cục. Send/Sync: `Value` **không/không**; `Task` **không/không** (chạy trên executor đơn luồng của worker sở hữu); `Chan` **có/có** (Arc + khoá bên trong; đường duy nhất qua luồng); `Host` **có/có** (Arc, bất biến sau khởi tạo). Giá trị qua `Chan` giữa 2 worker: **sao chép sâu** (int/float/bool/str/bytes/list/dict/record của các kiểu này); closure/handle tệp/Task **không truyền được** ⇒ lỗi kiểu nếu biết lúc biên dịch, nếu không thì mã lỗi lúc chạy (Agent 1 đề xuất mã trong `all_codes`). Chia sẻ `bytes`/`str` lớn bất biến qua Arc: **hoãn**, ghi nợ, chỉ làm khi có số đo. Kiểm bắt buộc: doctest **`compile_fail`** chứng minh `Value: !Send` (không thêm crate); ca đỏ-trước: gửi closure qua kênh liên luồng phải lỗi.

### VD-161 · VS3 mục 3/4: chính sách crate, giấy phép, unsafe (2026-11-17)
Giấy phép cho phép: **MIT, Apache-2.0, ISC, BSD-2/3-Clause, Unicode-3.0, Zlib**; **CDLA-Permissive-2.0 chỉ cho dữ liệu chứng chỉ gốc**; ngoài danh sách ⇒ **BLOCKED-CHO-A0**. TLS: **rustls + rustls-platform-verifier** (kho chứng chỉ của HĐH; WASM dùng webpki roots), crypto provider **ring**; không nhúng webpki-roots trừ wasm. `rustls-webpki` phải **≥ 0.103.16** (RUSTSEC-2026-0382); cổng mục TLS phải kiểm **licenses + advisories** (đề xuất `cargo-deny`; Agent 1 ghi cách cài trên mac/ps/psvn và xử lý khi không có mạng). Giữ `#![forbid(unsafe_code)]`, **không** mở ngoại lệ D2-17; `openat`/`openat2` dùng **rustix** (API an toàn). Mỗi crate mới phải có dòng trong **bảng phụ thuộc** (tên, phiên bản, giấy phép, lý do, có unsafe bên trong hay không). Nếu chứng minh **không thể** làm không unsafe ⇒ BLOCKED-CHO-A0 kèm vị trí, Agent 0 duyệt từng chỗ. Agent 1 **xác minh lại bằng `cargo metadata`** tại đúng phiên bản ghim, không dựa trí nhớ.

### VD-162 · VS3 mục 5: định nghĩa "Windows hỗ trợ thật" (2026-11-17)
(1) Chạy **native**, không cần Git Bash/MSYS trong PATH; **loại** "giữ Git Bash" và "đóng gói POSIX". (2) 15 ca host phụ thuộc `/bin/*`: **thay bằng chương trình trợ giúp do repo tự build**, dùng **cùng kỳ vọng trên cả 3 nền**, **không** sửa kỳ vọng. (3) ~100 ca `cargo test` đỏ: phân loại **3 nhóm** — (a) phụ thuộc `/bin/*` ⇒ xử như (2); (b) lỗi thật của viscript trên Windows ⇒ sửa; (c) khác biệt bản chất nền (bit quyền POSIX, signal, symlink…) ⇒ **được kỳ vọng theo nền** (mở rộng VD-132) nhưng phải nằm trong **MỘT tệp danh sách duy nhất** có lý do từng ca, Agent 0 duyệt danh sách. (4) Chỉ tuyên bố "hỗ trợ thật" khi: cổng Windows **10/10**, host **54/54**, và có **CI runner Windows** (self-hosted `psvn` hoặc runner dịch vụ) chạy cổng tự động; trước đó vẫn là "chỉ đo".

### VD-163 · VS3 mục 2: select là hàm std `recv_any` (2026-11-17)
Dùng **hàm std** `recv_any(chans, timeout=None)` trả `(chi_so, gia_tri)`; hết thời gian trả `none` (Agent 1 khớp quy ước kiểu trả về hiện có). **Không** đụng tên `select` (lệnh chiếu cột, `builtin.rs:333-337`). **Không** làm nhánh gửi (send-case) ở VS3; cú pháp riêng chỉ xét lại khi có số liệu dùng thực tế. **#145** (chú thích `rt.rs` về `chan(0)`) xử cùng task này.

### VD-164 · VS3: xác nhận nợ đường GHI và ma trận phụ thuộc (2026-11-17)
**Xác nhận** đường GHI = **`#140/#109b`** (VD-154a). **Ma trận**: mục 1 → mục 2/3/4; mục 3 → mục 6; **mục 5 độc lập**, chạy song song được. Tài liệu VS3 ghi ma trận **kèm TÊN mục**, không chỉ số.

### ĐÍNH CHÍNH SỐ · 2026-11-17 (Agent 1) — mục luật tạm
Bản ghi tạm ở `2468871a` mang số **VD-165** trùng số Agent 0 cấp cho việc **tách repo ViScript**; nội dung luật chuyển sang **VD-166** (Agent 0, thay thế VD-157). Số **VD-165** thuộc về **tách repo**.

### VD-165 · Tách ViScript thành repo riêng "ViScript" (2026-11-17)
Tên repo **"ViScript"** (viết hoa cùng kiểu ViFastOs, ViFastKey, ViFastTts); tên crate/thư mục mã/lệnh giữ **chữ thường** theo quy ước Rust: `viscript`, `vis`, `vis-remote`. **Áp dụng NGAY cho ST-168/ST-169**: mọi trích dẫn `file:dòng` ghi **đường dẫn tính từ gốc ViScript**, không kèm tiền tố thư mục repo hiện tại (ví dụ `src/rt.rs:120`). Mở **ST-171** (Agent 1 điều phối, Agent 2 thực thi), xếp **sau ST-170**, chỉ chạy khi `R29` đã có số đo **3 nền cùng HEAD**, ngân sách **3 h**; không chạy khi một đêm khác đang chạy trên cùng repo. Điều kiện xong: (a) tách bằng `git filter-repo`/`subtree split`, giữ đủ lịch sử; (b) chuyển sổ nợ/QUEUE/JOURNAL/báo cáo/check_report + cập nhật đường dẫn, `check_report --all` PASS ở repo mới; (c) giữ nguyên số nợ và số task (#146, #140, ST-171…), không đánh lại từ 1; (d) cổng repo mới trên mac/ps/psvn **cùng số đo với R29**, lệch thì giải thích từng suite; (e) repo cũ xoá mã ViScript, giữ một dòng trỏ sang, **không** hai bản song song; (f) cập nhật `vis-remote` + đường dẫn clone trên ps/psvn, `git pull` psvn không bị chặn sau một lần chạy cổng; (g) ngoài phạm vi ⇒ **BLOCKED-CHO-A0**, không tự quyết.

### VD-166 · Luật chung: Agent 1 quyết định thay Agent 0 (2026-11-17)
**Thay thế và mở rộng VD-157**, áp dụng **mọi dòng**. Khi A0 vắng và người dùng cho phép, Agent 1 được quyết định thay tới khi A0 online. **ĐƯỢC**: nợ nhỏ không cần thiết kế, việc sổ, đo đạc, báo cáo; thứ tự task trong phạm vi đã duyệt; phân loại sự cố khi có bằng chứng (log, diff, `file:dòng`). **KHÔNG được** (ghi BLOCKED-CHO-A0 rồi chuyển việc): thiết kế/API mới hoặc thay đổi; chính sách crate/giấy phép/unsafe; đổi kỳ vọng ca cũ, đổi ngưỡng, đổi cách đo ngưỡng; hạ phạm vi cổng/mốc đã duyệt hoặc tuyên bố đạt cổng/mốc; mở hàng VS3 khi 6 tài liệu chưa được A0 duyệt (VD-148). Mỗi quyết định thay: **mã riêng**, lý do + bằng chứng, nhãn **“CHỜ A0 XÁC NHẬN”**, liệt kê trong báo cáo kế tiếp để A0 duyệt hồi tố.

### VD-167 · Tài liệu công thái học ViScript + bổ sung VD-165(4b) (2026-11-17)
(1) **Bổ sung VD-165 mục 4b**: tách repo phải chuyển **cả `docs/viscript/`**, không chỉ `viscript/`. (2) Mở **ST-172** (Agent 1, **chỉ đọc**, 0 dòng mã), xếp **sau ST-169**: soạn `JOURNAL/vs-ergo.md` với 7 mục — (a) `str + str` (cho phép hoặc giữ lỗi + gợi ý `"{a}{b}"`); (b) `xs[i]` trả `T?`, `s[i]`/`s[a:b]` trên `str` vẫn không có; (c) pattern nhận cả `Some(v)` lẫn `some(v)`, `none` giữ nguyên; (d) giá trị mặc định của `type` tự áp khi dựng record (nợ, không phải thiết kế mới); (e) `print` nhiều đối số; (f) mọi `std.str.f(s, …)` gọi được dạng `s.f(…)`; (g) format spec `"{n:.2}"`. Mỗi mục: hiện trạng (`file:dòng`), đề xuất, ca đỏ-trước, ảnh hưởng mã/test. (3) Kiểm **VS0412** có dùng cho **2 lỗi khác nhau** (gán lại `let`, `str + str`) không; nếu có, **đề xuất tách mã**. (4) Cheat sheet: đổi nhãn **"VS0 chưa có"** thành **"chưa có tại <commit>"**. (5) **Không** sửa mã; A0 duyệt `vs-ergo.md` cùng đợt 6 tài liệu VS3.

### VD-168 · Mở rộng công thái học ViScript (thay mục 2, 4 của VD-167) (2026-11-17)
VD-167 **giữ nguyên** trong sổ; **VD-168 thay mục 2 và mục 4**; mục 1, 3, 5 của VD-167 giữ nguyên. (1) **Quy tắc chữ hoa** (ghi vào thiết kế): “Hàm dựng mang dữ liệu viết hoa (Some, Ok, Err, Circle); literal không mang dữ liệu là từ khoá viết thường (true, false, none)”. (2) **ST-172** (Agent 1, chỉ đọc, 0 dòng mã) — `JOURNAL/vs-ergo.md`, mỗi mục: hiện trạng (`file:dòng`), đề xuất, ca đỏ-trước, ảnh hưởng mã/test, **mức ưu tiên**: (a) Optional: pattern chỉ `Some(v)`, `none` giữ, tự gói `T→T?`; (b) dựng record theo tên kiểu `U(name: "An", …)` + default; (c) **AnyError** `?` tự chuyển lỗi — **ưu tiên cao**; (d) tham số tên trong `fn` (`f(1, b: 5)`), `--b` chỉ ở `cmd`; (e) `const` suy kiểu khi RHS literal; (f) `panic`/`newtype`: làm xong hoặc bỏ khỏi từ khoá trước 1.0; (g) `str+str`; (h) `xs[i]` trả `T?`, `str` không chỉ số/slice; (i) `print` nhiều đối số; (j) `s.f(…)` cho mọi hàm `str`; (k) format spec `"{n:.2}"`; (l) `duration` in `2h30m`. (3) Cheat sheet: đổi nhãn “VS0 chưa có” → “chưa có tại <commit>” + **ghi chú dùng `r"…"` cho chuỗi JSON**. (4) **Không** sửa mã; Agent 0 duyệt `vs-ergo.md` cùng 6 tài liệu VS3; **các mục được duyệt thực thi TRƯỚC VS3**.

### VD-169 · Chia ST-172 thành 172a (dữ kiện) và 172b (đề xuất) (2026-11-17)
**ST-172a** (Agent 1, chỉ đọc, 0 dòng mã) — `JOURNAL/vs-ergo-facts.md`: **CHỈ dữ kiện**, không đề xuất; mỗi mục ghi trạng thái **[ĐÃ CÓ] / [THIẾT KẾ CÓ, CHƯA LÀM] / [CHƯA CÓ]** kèm `file:dòng` + commit; 8 nhóm (cú pháp; hệ kiểu; ngữ nghĩa giá trị; lỗi + `all_codes`; thư viện chuẩn; tính năng riêng; trải nghiệm thật với 3 script + bản Python; quyết định đã chốt); đầu tệp có mục lục, mỗi nhóm `## N. <tên>`. **ST-172b** — `JOURNAL/vs-ergo.md` theo VD-168, **chỉ bắt đầu sau khi Agent 0 gửi phân tích dựa trên ST-172a**.

### VD-170 · Phân tích vs-ergo-facts, mở ST-172b (2026-11-17)
Căn cứ `vs-ergo-facts.md` (ST-172a). **A. Rút khỏi phạm vi VD-168**: mục (h) `xs[i]` trả `T?` (Q5/VD-18 đã chốt `xs[i]` **panic**, `xs[i]?`/`xs.get(i)` trả `T?` — ST-172b chỉ **xác minh đã làm hay chưa** kèm probe) và phần tự gói `T→T?` của mục (a) (đã có). **B. Chốt (ST-172b đặc tả + ca đỏ-trước, không bàn lại)**: **P0** (1) `newtype` chưa hạ ⇒ **báo lỗi ngay dòng khai báo**, không im lặng; (2) mọi thông điệp có chữ “VS0” ⇒ bỏ, thay “chưa hỗ trợ (nợ #N)”; (3) **cheat sheet thành doc-test** (mọi ví dụ “chạy được” là ca test; sửa theo dữ kiện: `if let`, `else e:` dạng biểu thức, `--help`, `AnyError` chưa có). **P1** (4) `p"…"` không xử lý escape, giữ nội suy `{}`; VS0102 thêm gợi ý `r"…"`/`p"…"`; (5) thông điệp **đóng băng** nhưng ĐƯỢC thêm dòng “gợi ý:” cho 5 mẫu VS0201 (`const A = 1`, `s[a:b]`, `U(n: 1)`, `"{x:.2}"`, `|x|`); (6) **tách mã đa nghĩa** VS0412/VS0501 (mã mới từng nghĩa, mã cũ chỉ phần chung; đề xuất số + bảng chuyển). **P2** (7) viết hoa `Some/Ok/Err` cả dựng lẫn pattern, `none/true/false` thường, **xoá builtin `None`**, pattern thường ⇒ cảnh báo + `vis fmt` sửa, sửa VD-19; (8) **AnyError** theo VD-20 (probe lại `n9`, chép mã; giải thích `json.decode` trả “V” và vì sao `?` fs+json báo VS0412); (9) method-form `std.<mod>.f` khi tham số đầu kiểu `T`; kiểm xung đột tên `std.str`/`str`/builtin `str(x)`; (10) **`str+str` cho phép nối**; (11) `print` nhiều đối số (cách nhau 1 dấu cách), format spec chạy runtime, record `U(name: …)` áp default, `if let`. **P3** (12) `const` suy kiểu từ literal, `duration` in chuẩn hoá (`1h30m`), `cmd` tự sinh `--help`; (13) Q2 giữ chuyển ngầm `int→float` + số học trộn, sửa văn bản Q2. **C. ST-172b** (Agent 1, chỉ đọc, 0 dòng mã) — `vs-ergo.md`: mỗi mục 1–13 có hiện trạng `file:dòng`, đặc tả, ca đỏ-trước, ảnh hưởng mã/test, chia task ≤3 h, thứ tự phụ thuộc; **probe lại ở HEAD mới nhất** (không dùng binary `0df923c2`); viết lại 3 script theo thành ngữ (`filter/map/len`, `cmd`), ghi số dòng ViScript vs Python và lý do nếu dài hơn.

### VD-171 · Bộ quy tắc công thái học ViScript (R1–R10) (2026-11-17)
**Thay thế mục 7, 8, 10, 11, 13 của VD-170**; **sửa VD-19, Q2, §1.6**; các mục khác của VD-170 giữ nguyên. ST-172b soạn đặc tả + ca đỏ-trước + **bảng chuyển đổi mã cũ** cho từng quy tắc; không bàn lại hướng. **R1** chữ hoa: kiểu/trait/biến thể enum **UpperCamel** (`Some/None/Ok/Err/Circle`); biến/hàm/module/trường **snake_case**; hằng **UPPER_SNAKE**; `true/false` thường; dựng và pattern **giống hệt nhau**; pattern thường ⇒ **cảnh báo + `vis fmt` tự sửa**. **R2** `Option`/`Result` là **enum std** (`Option[T]: Some(T), None`; `Result[T,E]: Ok(T), Err(E)`); `T?` = `Option[T]`; **bỏ từ khoá `none`** (`None` là biến thể); cho phép `T??`, tự gói đủ số lớp, `None` lớp ngoài cùng, lớp trong `Some(None)`; ca đỏ-trước `first[T]` với `T=int?` + `map[str, int?].get` phân biệt **không có khoá** vs **khoá = None**. **R3** `?`, `??`, `if let`, `let … else`, `match`; `let x = expr else e: <khối>` (khối **bắt buộc thoát**; Option **không** có `e`); **không** có biểu thức `f() else 0`. **R4** mọi hàm std trả `Result[T, Error]` với **`Error` là record cài trait `Error`** (`message, source, span`) thay `str`; `?` tự chuyển `E: Error → AnyError`; `json.decode -> Result[Json, Error]`, `Json` là enum (`Null/Bool/Num/Str/Arr/Obj`), **bỏ `V`**; `main` có thể trả `Result[(), AnyError]` (in lỗi, thoát **1**); ca chuẩn **đọc tệp + decode 2 dòng bằng `?`**. **R5** `f(1, b: 5)`; `--b` chỉ chế độ lệnh/`cmd`; `User(name: "An", age: 20)` + default bỏ được; **`{…}` chỉ là map**, bỏ dựng record bằng `{}`; `x.f(a) ≡ mod.f(x, a)` khi tham số đầu của `f` có kiểu của `x` (**toàn bộ std**); báo xung đột `std.str`/`str`/builtin `str(x)`. **R6** `p"…"` **không** escape + có nội suy; `str+str` nối; `{expr:spec}` chạy runtime; `print` nhiều đối số cách 1 dấu cách. **R7** chỉ `int→float` chuyển ngầm; **`/` luôn trả `float`**; **`//`** chia lấy phần nguyên; `%` khớp `//`; `const` suy kiểu từ literal; sửa Q2; **liệt kê mọi test dựa vào `int/int=int`**. **R8** giữ Q5/VD-18; **xác minh** `xs[i]?`/`xs.get(i)`. **R9** mỗi mã **một nghĩa** (tách VS0412/VS0501/VS0201/VS0301/VS0604/VS0701; bảng chuyển đổi); lỗi cú pháp hay gặp có dòng “gợi ý:”; bỏ chữ “VS0”; từ khoá chưa làm (`newtype`…) **báo lỗi tại chỗ dùng**. **R10** đơn vị in chuẩn hoá (`1h30m`, `1.5mb`); xác minh trộn đơn vị báo lỗi. **Thứ tự thực thi**: P0 VD-170 → R9 → R6 → R1+R2 → R3+R4 → R5 → R7 → R10, xong **trước VS3**.

### VD-172 · Cú pháp còn lại R11–R21 (bổ sung VD-171) (2026-11-17)
**Thêm vào phạm vi ST-172b**, mỗi mục: hiện trạng (`file:dòng`, probe HEAD), đặc tả, ca đỏ-trước, ảnh hưởng, task ≤3 h. **Giữ nguyên**: thụt lề + `:`, `elif`, `if` là biểu thức, biểu thức cuối khối là giá trị trả, closure `=>` một dòng + `fn(...)` nhiều dòng, generic `[T]`, `..`/`..=`, `and/or/not`, `match` xét hết, `with`, `derives`, `set{}`, `test fn`, `#`/`##`, `^ten`, `| complete`, `extern cmd`. **R11** mọi hàm dựng là **hàm** (record, biến thể enum, kiểu bọc) gọi **theo vị trí hoặc theo tên** (`Circle(2.0)`, `Circle(r: 2.0)`; tên sau vị trí; cùng luật `fn`). **R12** gộp khai báo kiểu: **`type X = T`** thay `alias`; **`type Email(str)`** thay `newtype` (dựng `Email("…")`, mở `let Email(s) = e`); **bỏ `alias`, `newtype`**. **R13** biến thể enum **không cần tiền tố** khi tên duy nhất; trùng tên ⇒ **bắt buộc `Shape.Empty`** (dùng `.`), lỗi biên dịch có gợi ý; `Some/None/Ok/Err` **luôn trong prelude**. **R14** method: **`impl P:`** cho method riêng; `fn f(self)` **chỉ đọc**; `fn f(var self)` **sửa được**, chỉ gọi trên **biến `var`**; `Self` viết hoa; báo hiện trạng. **R15** xuống dòng: **tự nối** trong `() [] {}` và khi dòng mới bắt đầu bằng `.`, `|`, `??`, `and`, `or`; **dấu phẩy cuối** được phép mọi nơi. **R16** `let` **che tên** trong cùng phạm vi (bỏ `VS0302` cho `let`); `var` **không** khai lại. **R17** điều kiện `if`/`while` **chỉ `bool`**; `if xs:` báo lỗi kèm gợi ý `xs.is_empty()`. **R18** pattern: literal, khoảng `1..=5`, tuple, record `User(name, age: a)`, gắn tên `as`; thêm **`while let`**; báo pattern đã có. **R19** `0o755`; tràn i64 runtime **panic**; bit là method std `bit_or/bit_and/bit_xor/shl/shr`. **R20** `assert cond, "msg"`; `use std.fs as f`; `use std.fs.{read, write}`; `list` thêm `enumerate/zip/any/all/find/sort_by/is_empty`; range `.step(n)`. **R21** pipeline `|` **chỉ** chế độ shell + khối `cmd {}`; code thường dùng chuỗi method; hai bên **dùng chung hàm std**; ghi vào thiết kế. **Kiểm thêm**: `f[int](x)` có xung đột với `xs[i](...)` trong parser không. **Thứ tự bổ sung**: … → R5 → R11 → R12 → R13 → R14 → R15 → R16 → R17 → R18 → R7 → R19 → R10 → R20 → R21 (chỉ sửa tài liệu). **Cheat sheet** cập nhật theo **R1–R21** sau khi thực thi; mọi ví dụ là **doc-test** (VD-170 P0 mục 3).

### VD-173 · Chia pha công thái học + R19c + R14 (2026-11-17)
**Căn cứ** `vs-ergo.md` R11–R21 (`2f4e8302`, `80da0c46`). (0) **Mọi báo cáo/tài liệu/commit message viết tiếng Việt**, không tiếng Trung. (1) **Chia 2 pha**: **Pha 1** (TRƯỚC VS3, thay đổi **phá mã cũ**): P0 của VD-170, R9, **R17 chỉ thêm gợi ý**, R6, R1+R2, R3+R4, R5+R11, R12, R16, R7, **R19c**; **Pha 2** (song song/sau VS3, **chỉ thêm**): R14, R18, R19 (`0o755`, phép bit), R20, R10, R15 (nối dòng bằng `.`/`??`/`and`/`or`), R21 (tài liệu). **Quy tắc**: mục mới làm hỏng ≥1 test/ví dụ hiện có ⇒ **pha 1**, ngược lại **pha 2**. (2) **R19c**: float literal **chỉ khi sau `.` là chữ số**; `5.bit_or(3)` = int + gọi method; `5.0` là float; `1.`/`1.e3` **vẫn cấm**; ca đỏ-trước `5.bit_or(3) == 7`. (3) **R14** (method người dùng) là **tính năng mới**: mục riêng trong `vs-ergo.md`, thiết kế đầy đủ (`impl P:`, `self`, `var self`, `Self`, quan hệ trait + R5), task ≤3 h, **đứng đầu pha 2**. (4) **R11**: ghi rõ cú pháp khai báo biến thể enum **có dữ liệu đang chạy được** (cheat sheet dựng `Circle(2.0)` nhưng khai báo `Circle(float)` ⇒ **VS0201**), **chép nguyên mã probe**. (5) Trước khi A0 duyệt `vs-ergo.md`: **bảng tổng ở đầu tệp** (R, pha, số task, giờ ước tính, số test/ví dụ bị ảnh hưởng; tổng riêng pha 1 và pha 2). (6) **Thứ tự task giữ nguyên** ST-164 → ST-165 → … → ST-170 → ST-171; **không** sửa mã R nào cho tới khi A0 duyệt `vs-ergo.md`.

### VD-174 · Nguyên tắc mô hình ViScript (bổ sung R14) (2026-11-17)
Ghi vào **thiết kế như nguyên tắc nền**; ST-172b soạn đặc tả + ca đỏ-trước cho **mục 4–6**. (1) **Không class, không kế thừa** — trừu tượng chỉ qua `type` + `trait` + `impl`; dùng lại bằng **method mặc định trong trait** hoặc **record chứa record**. (2) **Không lifetime/con trỏ/tham chiếu/unsafe/macro người dùng** (trước 1.0); sinh code chỉ qua **derives có sẵn**. (3) **Không tham chiếu dùng chung sửa được** (`Rc<RefCell>`) ở 1.0: trạng thái chung trong một luồng = **map + ID**, giữa luồng = **`Chan`**; truyền theo giá trị ⇒ **không vòng tham chiếu**, đếm tham chiếu đủ. (4) **Trait dùng được làm kiểu**: `list[Shape]` chứa mọi kiểu cài `Shape`; **tự đóng gói**, không cú pháp `Box`/`dyn`; đặc tả **luật an toàn đối tượng**, gọi method **qua bảng ảo**, ca đỏ-trước **2 kiểu cài cùng trait trong 1 list**. (5) **Sửa trường lồng sâu trên `var`**: `u.addr.city = x`, `xs[i].name = x` hợp lệ, **tự sao chép đúng đường dẫn**; **probe hiện trạng**. (6) **Chuỗi method trên list/map/range là lười (iterator)**; tạo collection khi gọi **`.to_list()/.to_map()/.to_set()`** hoặc trong `for`; đặc tả **kiểu trả về** của `filter/map/take/enumerate/zip`; **chốt trước R20**. **Mục 4–6 đứng cùng R14 ở đầu pha 2**; mục 1–3 **chỉ sửa tài liệu**.

### VD-175 · ST-166, landlock, #146 (2026-11-17)
(1) **ST-166 công nhận DONE**; **xác nhận đóng #132** (bỏ nhãn *CHỜ A0 XÁC NHẬN*). `+crt-static` trên mac là **dữ kiện**, **không** đổi chính sách: `+crt-static` **giữ chỉ trên Linux**. (2) **Landlock: DUYỆT cfg-hoá** — `vis-host-linux/Cargo.toml`: `landlock` chuyển vào `[target.'cfg(target_os = "linux")'.dependencies]`; `tests/landlock.rs`: thêm `#![cfg(target_os = "linux")]`; điều kiện xong: **mac `cargo test` không còn lỗi biên dịch do landlock**, **ps số ca landlock KHÔNG đổi** (ghi 2 số), **gate ps PASS**; ghi vào `HOST-BAO-DAM.md` (**landlock chỉ Linux — giới hạn nền**); làm task nhỏ **ST-173** (≤1 h) **sau ST-167, trước ST-170**. (3) **#146** đã phân loại (b) — **TOCTOU thật**: gửi A0 nội dung **BLOCKED-CHO-A0** trong báo cáo kế tiếp (assertion hỏng, đường mã `file:dòng`, chuỗi sự kiện đua; ca đỏ-trước tái hiện được chưa + **tỉ lệ đỏ**; phương án sửa `openat`/`O_NOFOLLOW` qua `rustix`, kiểm lại sau mở; có cần đổi thiết kế không). **Từ nay** gate đỏ do #146 được **chạy lại 1 lần** để phân loại như D2-07, nhưng **mỗi lần phải ghi log** và **đếm số lần trong R29**. (4) Danh sách còn lại **không có ST-168/ST-169**: phải báo rõ **đã xong** (commit + tệp) hay **đã bỏ**; nếu chưa xong ⇒ xếp lại trước ST-170. **Thứ tự**: ST-167 → **ST-173** → (ST-168, ST-169 nếu chưa xong) → ST-170 (R29) → ST-171.

### VD-176 · #146: vá (A) + chứng minh tất định (2026-11-17)
(1) **Chọn (A)**: `quyen.rs:472` đổi `std::fs::metadata` → **`std::fs::symlink_metadata`**; **không** đổi thiết kế/kỳ vọng; **loại (B)** (F_GETPATH/libc/unsafe) và **(C)** (tách kỳ vọng theo nền). (2) **Chứng minh BẮT BUỘC bằng ca tất định** (không dựa số vòng): thêm **điểm chèn chỉ `#[cfg(test)]`** (không có trong build thường) **giữa `canonicalize` (`:465`) và bước kiểm (`:472`)**; **ca đỏ-trước**: điểm chèn đổi `root/tep` thành symlink ra ngoài ⇒ mã cũ đọc nội dung ngoài, **đỏ 100%, chạy 100 lần, ghi 100/100**; **ca thứ hai** đổi symlink về tệp thật ⇒ đúng đặc tả; **sau vá cả hai xanh 100/100 trên mac và ps**; **ca âm cho điểm chèn**: bỏ điểm chèn ⇒ ca phải báo **“không chạy được điểm chèn”**, không xanh im lặng; **bằng chứng phụ**: **6 triệu vòng** ngẫu nhiên mac sau vá, ghi số đỏ. (3) Ghi **giới hạn đã biết** vào `HOST-BAO-DAM.md`: **hardlink** tệp ngoài vào root **được coi là trong root** (giới hạn chính sách, không phải lỗi đua). (4) **Hướng dài hạn** (VS3 cùng `#140/#109b`, **không làm bây giờ**): mở **từng thành phần đường dẫn** bằng **`openat` + `O_NOFOLLOW`** qua **`rustix`** (Linux có thể `openat2` `RESOLVE_BENEATH`), **bỏ hẳn kiểm theo đường dẫn**; **đường đọc và đường ghi dùng chung một cơ chế**; Agent 1 thêm mục này vào `vs3-qd` tương ứng. (5) Task **ST-174** (≤3 h) **sau ST-173, trước ST-170**; **đóng #146 khi mục 2 đạt**; **gate mac 3 lần liên tiếp PASS** (không cần chạy lại); **R29** ghi số lần gate đỏ do #146 **trước khi vá** (theo `st170-dem-gate-do-t146.md`) và **trạng thái sau khi vá**.

### VD-177 · ST-167, nợ #147, cây ps (2026-11-17)
> **§2b ĐÃ BỊ THAY bởi VD-197(4)**: ngưỡng **3×** chỉ áp khi thời gian mac của phép đó **≥ 10 s**; phép dưới 10 s chỉ ghi số, **không mở nợ**.
(1) **ST-167 công nhận DONE** (10/10 cả hai nền, cùng HEAD `795df046`, cây `viscript` `092fd5ab`); **mac 1696 s = CẬN TRÊN**, không dùng làm số chuẩn. (2) **Cấp nợ #147 “mac chậm hơn ps” ngay**, không chờ đo lại; điều tra **a** đo lại soak mac khi **load < 2** và **không có việc dòng OS** (ghi load trước/sau) → **b** nếu tỉ lệ **≤ 3×** thì **đóng #147**, số mới là số chuẩn → **c** nếu vẫn **> 3×** thì đo **từng suite** trong gate cả hai nền, liệt kê **10 suite chênh lớn nhất**, kiểm riêng ca nhiều vòng (vd. `#146` 50 000 vòng) và ca tạo tiến trình; xếp **sau ST-174, trước ST-170**, **2 h**, không sửa mã/ngưỡng. (3) **Khoá máy dùng chung giữa hai dòng**: mọi phép đo thời gian/hiệu năng giữ **`~/.vifast-measure.lock`** (flock); dòng còn lại **không build/không QEMU** khi khoá bị giữ; log ghi **load trước/sau + có giữ khoá không**; phép đo **không giữ khoá chỉ là “cận trên”**. (4) **Cây ps bị xoá hàng nghìn tệp tracked — điều tra TRƯỚC ST-170**: ghi đường dẫn repo, user (`ps`/`root`), `git status --short | wc -l`, đếm tệp bị xoá theo thư mục cấp 1; trả lời các lần gate ps (ST-161/166/167) chạy trên cây nào và **tree hash `viscript` lúc đó có khớp commit trong log** không; **từ nay gate ps chạy trên clone/worktree sạch** (`git status` rỗng ngoài log bị ignore) và **`vis-remote` từ chối chạy nếu cây không sạch**; **không** khôi phục/xoá gì trong cây cũ cho tới khi A0 đọc kết quả; sau ST-171 ps/psvn **clone mới** từ repo ViScript. (5) **Thứ tự**: ST-173 → ST-174 → **cây ps (≤1 h)** → **#147 a-c** → ST-170 (R29) → ST-171 (chỉ sau khi A0 duyệt 6 tài liệu VS3 + `st169` + `vs-ergo.md`).

### VD-178 · Duyệt 6 tài liệu VS3, bảng Windows, vs-ergo.md (17/11)
**Cả 3 mục DUYỆT CÓ SỬA** (**16 chỗ [SỬA]** — **VD-194** đính chính; bản ghi A0 17/11 ghi nhầm "9 chỗ", đếm máy trong `JOURNAL/quyet-dinh-a0-2026-11-17-vd178.md` là **16 dấu** trên 9 nhóm quyết định; ghi trong `JOURNAL/quyet-dinh-a0-2026-11-17-vd178.md`, sha256 `ca7f565e…`). **ST-171 (tách repo) được mở khoá sau ST-170**; các [SỬA] chỉ là **tài liệu**, làm ở task **ST-178** (≤3 h, Agent 1, 0 dòng mã) **ngay sau ST-171, trong repo ViScript mới**. **Thứ tự lớn**: pha 1 công thái học (30 task ≈62 h) chạy **trước VS3**; **xen kẽ được VS3-5a (helper Windows) và VS3-5d (CI Windows)**. **Mọi đổi kỳ vọng test nêu trong VD này là A0 duyệt** (không hỏi lại). **Phần A**: QĐ-1 [SỬA] đo song song bằng task **CPU** (không sleep), ngưỡng `W=min(nproc,N)` nhanh hơn `W=1` ≥ **0,6×min(nproc,N,4)**; **không cấm await task khác worker** — `spawn` gửi được, đối số/giá trị trả **sao chép sâu**, kiểu gửi được liệt kê; **VS0508** = giá trị không gửi được. QĐ-2 [SỬA] **chan(0) hợp lệ = rendezvous**, #145 xoá chú thích sai `rt.rs:128-129` + bỏ chặn runtime (không thêm tyck); `recv_any` trả `(int,T)?`, hết giờ `None`; **công bằng vòng xoay**. QĐ-3 [SỬA] mặc định kho HĐH, **có `http.client(ca: p"…")`** cho CA riêng, **không** tuỳ chọn insecure; mặc định timeout/redirect/body (số do Agent 1 đề xuất, A0 chốt ở ST-178); TLS server ngoài phạm vi (ghi nợ); ring Windows kiểm ở 3a. QĐ-4 [SỬA] **§4b là thiết kế chính** cho Linux+macOS: đi **từng thành phần** từ fd gốc, `O_NOFOLLOW`/`O_DIRECTORY`, đọc+ghi **chung một hàm**; `..` theo từ vựng, ra ngoài gốc ⇒ từ chối; Linux ≥5.6 dùng `openat2(RESOLVE_BENEATH|RESOLVE_NO_SYMLINKS)` + fallback, test cả hai; **không cần fd→path** ⇒ macOS **không còn khe hở bản chất**; sửa §1/§7/HOST-BAO-DAM; **duyệt crate `rustix`** (giấy phép MIT), crate ta giữ `forbid(unsafe_code)`; Windows vẫn kiểm theo đường dẫn, mức bảo đảm thấp hơn. QĐ-5 [SỬA] số liệu **48/15/42=105** (thêm `host_conformance_suite`); **VS3-5b sau R6** (p"…" sửa gốc `\`); luật test: không chèn đường dẫn vào chuỗi thường, golden **tương đối**, mục tiêu danh sách nền **= 0**; helper 5a **chỉ** echo/cat/grep/printf/true/false/sleep, **không** viết lại `sh -c`; nhóm b (15) **mặc định lỗi sản phẩm** (lsp 5 nghi URI Windows; net_server/net điều tra, cổng đóng phải báo “bị từ chối”; json_ra 4: python3 → nhóm a, tên tệp đặc biệt → nhóm c); chuyển b→c cần A0 duyệt. QĐ-6 [SỬA] **mô hình tin cậy khoá**: mỗi registry ghim khoá công khai, `pkg add` ghi khoá vào `vis.lock`, đổi khoá ⇒ **từ chối** trừ `vis pkg trust <gói>`; **VS0707** = chữ ký sai; **#142**: `pkg update X` chỉ X + phụ thuộc bắt buộc, không đối số ⇒ tất cả; cargo-deny trước 6a. **Phần B**: duyệt cách phân loại + “sửa test trước, danh sách nền về 0”. **Phần C**: bảng **mã thoát** (0/1/2/**101** panic) — test cũ mong exit 1 cho VS0502/0503 **đổi sang 101** (duyệt, làm ở R9); R1+R2 **làm chung**, bỏ “xoá builtin None” riêng, `none` ⇒ **VS0203** có gợi ý `None`, fmt tự sửa; R3 **if let/while let nhận pattern** (`if let Some(v) = o:`), bỏ dạng tắt; R5 `{ten: …}` trần ⇒ **VS0205** gợi ý `"ten"`/`Kieu(ten: …)`; method chỉ cho hàm std tham số đầu **kiểu cụ thể**, generic `T` không; giữ `str(x)`; R7 duyệt đổi **4 test** (run.rs:98,100,112; interp_v2.rs:77) + ca `-7 % 2 ⇒ 1`; đơn vị chia nhau ⇒ float, chia số ⇒ đơn vị (làm tròn xuống); R9 VS0203 = “từ khoá đã bỏ”, VS0204 = “const cần kiểu khi RHS không literal”, thêm VS0508/VS0707; R10 [SỬA] cộng **cùng loại** đơn vị hợp lệ (tự quy đổi), khác loại mới lỗi; duration in ghép `2h30m`, size thập phân `1.5mb`, có `.fmt_iec()`; R11–R13 duyệt; R14 [SỬA] **trùng tên method ⇒ lỗi** (không ưu tiên ngầm), `impl P:` chỉ trong module khai P; R15–R18 duyệt; R19 panic 101; R20/R21 duyệt; R22 [SỬA] bỏ “dùng match để ép kiểu ngược”, ép ngược ghi nợ sau 1.0; R23 [SỬA] ca dùng `U(addr: A(city: "A"))`; **R24 [SỬA] đảo VD-174.6**: `filter/map/…` **giữ trả collection ngay**, **`.lazy()`** trả `Iterator[T]`, `range`/`fs.lines` mặc định lười, iterator **duyệt lại được**, R20 không chờ R24. **C3 thứ tự pha 1**: P0 → R9(+C1) → R6 → R17 → R1+R2 → R3 → R8 → R4 → R5 → R11 → R12 → R16 → R7 → R19b → R19c. **Task**: ST-171 (Agent 2, sau ST-170); **ST-178** (Agent 1, sửa tài liệu theo [SỬA] trong repo mới, check_report PASS, mỗi [SỬA] ghi “đã sửa: tệp:dòng”, gửi A0 diff); **ST-179+** (Agent 2, pha 1 theo C3, mỗi task ≤3 h, đỏ-trước, gate PASS).

### VD-179 · Cây ps, phạm vi tách repo, đóng băng (17/11)
(1) **Đóng băng**: **chỉ commit khi dòng OS báo KT-88 xong**; quá 16:27 mà chưa báo ⇒ **hỏi dòng OS trước**, không tự commit theo giờ. (2) **“`viscript/` = 0 tệp bị xoá” CHƯA đủ** — mã ViScript còn ở **`crates/`** (vd. `crates/vis-cli/src/quyen.rs`), mà `crates/` có **93 tệp bị xoá** trên ps; **bổ sung vào báo cáo ST-175** (chỉ đọc, không gate, làm trong lúc đóng băng): **a** liệt kê crate ViScript bằng **`cargo metadata`** (không viết tay); **b** trong 2297 xoá + 25 sửa + 16 untracked, đếm phần thuộc các crate đó + `viscript/`, ghi **danh sách tệp**; **c** tra **reflog** trên ps lúc chạy gate ST-161/166/167 (HEAD hiện `898c6de8`, ST-167 ghi `795df046`). **Kết luận theo bằng chứng**: nếu phần ViScript **sạch và commit khớp** ⇒ **giữ 3 gate**, ghi “**sạch theo kiểm hồi tố**”; nếu không ⇒ gắn nhãn “**không chứng minh đo đúng mã**”. (3) **R29 (ST-170)**: chạy lại **MỘT lần** gate ps trên **clone sạch mới** (đường dẫn mới; **user `ps`** thay `root` nếu được, dùng root thì ghi lý do) **sau khi hết đóng băng**; **số này là số ps chuẩn của R29**. Cây cũ `/root/vifast-linux`: **giữ nguyên**, không xoá/khôi phục, **đổi tên** thành `/root/vifast-linux-BAN-CU-KHONG-DUNG`. **`vis-remote` từ chối cây bẩn: DUYỆT**, kèm **ca âm** (cây có 1 tệp sửa ⇒ từ chối). (4) **Bổ sung VD-165 (ST-171)**: phạm vi tách = **`viscript/` + `docs/viscript/` + mọi crate ViScript theo `cargo metadata`** + mọi đường dẫn khác mà các crate đó phụ thuộc; **kiểm sau tách**: `cargo build` và gate ở repo mới **không cần tệp nào từ repo cũ**. (5) **L0 1184/0** (R29 dự kiến 1182): ghi rõ **2 ca thêm thuộc task nào**. **Thứ tự**: (đóng băng) **mục 2** → [KT-88 xong] commit ST-175 → ST-176 (#147) → ST-170 (R29 + mục 3) → ST-171 → ST-178 → ST-179+.

### VD-180 · Mô hình gói ViScript (thay QĐ-6 mục 1, 2 của VD-178) (17/11)
**Mô hình phi tập trung kiểu Go, KHÔNG có kho tập trung ở 1.0**: (1) **tên gói = đường dẫn** (`github.com/<chủ>/<repo>`, `vifast.org/x/<tên>`, domain riêng); `vis.toml`: `<tên ngắn> = "<đường dẫn>@<semver>"`, code chỉ `use <tên ngắn>`; phiên bản = **tag git semver** (`v1.2.3`), **major ≥ 2 phải có `/v2` trong đường dẫn**; chọn phiên bản bằng **Minimal Version Selection**, **không** bộ giải ràng buộc; `pkg update X` chỉ đổi **X + phụ thuộc bắt buộc của X** (giữ `#142`). (2) **`vifast.org/x/<tên>`** = trang tĩnh trỏ nơi chứa mã (meta tag) để đổi nơi chứa mà không đổi tên gói; gói chính thức dùng tiền tố này. (3) **Tin cậy thay khoá ký registry**: `vis.lock` ghi **sha256 của cây tệp chuẩn hoá** cho mỗi gói@phiên bản; **`sum.vifast.org`** = sổ hash công khai **chỉ ghi thêm** (giai đoạn đầu là repo git có commit ký, vis ghim khoá công khai của sổ); lần đầu hash phải khớp **sổ** (nếu bật), lần sau khớp **vis.lock**; **VS0707** = “hash gói không khớp vis.lock hoặc sổ”; **không có lệnh bỏ qua**, chỉ cấu hình tắt sổ **`VIS_SUMDB=off`** (ghi cảnh báo); gói riêng tư **`VIS_PRIVATE=<mẫu>`** ⇒ không hỏi sổ, không qua proxy. (4) **`proxy.vifast.org`**: thiết kế giao thức trong `vs3-qd-6.md` (GET theo đường dẫn@phiên bản), **CHƯA vận hành ở VS3**; **`VIS_PROXY=direct`** mặc định. (5) **Ca đỏ-trước bắt buộc**: (a) tag bị đổi nội dung sau khi ghi lock ⇒ **VS0707**; (b) hai phụ thuộc cùng cần X `v1.2` và `v1.4` ⇒ **chọn v1.4** (MVS, không chọn bản mới hơn); (c) gói `vifast.org/x/…` trỏ sang GitHub **tải đúng**; (d) **mạng tắt + có cache** ⇒ build chạy. (6) **Ngoài phạm vi VS3**: proxy chạy thật, `pkg.vifast.org` (tìm kiếm/tài liệu), yank/thu hồi phiên bản. **ST-178 sửa `vs3-qd-6.md` theo VD-180.**

### VD-181 · Tên miền cho mô hình gói ViScript (17/11)
**Đường dẫn gói chính thức = `vifast.org/vis/<tên>`** — **path trên một host**, **KHÔNG** subdomain làm tiền tố module; `<tên>` chữ thường `[a-z0-9-]`, không `_`/viết hoa; **không alias `/x/`**, không nhiều tiền tố song song; tài liệu ở `vifast.org/docs/vis/…`. **Dịch vụ ngang hàng, không lồng dưới `vis.`**: `sum.vifast.org` (sổ hash chỉ ghi thêm, khoá ghim trong vis) và `proxy.vifast.org` (chỉ thiết kế ở VS3, `VIS_PROXY=direct` mặc định) — phục vụ **mọi sản phẩm** ⇒ **không** dùng `sum.vis.`/`proxy.vis.`. **`vis.vifast.org`** KHÔNG làm tiền tố module (chỉ website/redirect nếu cần). **`viscript.org`** (nếu mua) chỉ là **tài sản thương hiệu**, không đưa vào đường dẫn gói ở 1.0 (muốn dùng phải là quyết định riêng, phiên bản lớn mới, không di trú). **Bảng host → vai trò**: `vifast.org` = site + namespace module (`/vis/*`, docs `/docs/vis/*`); `sum.`/`proxy.vifast.org` = dịch vụ; `os./tts./key.vifast.org` = dịch vụ độc lập, giữ subdomain. **Traefik/reverse proxy KHÔNG phải hợp đồng**: hợp đồng là **nội dung meta tag** ở `vifast.org/vis/<tên>`; `PathPrefix(/vis/)` → static origin là đủ. **Bất biến**: sau gói đầu tiên, tên/đường dẫn **không đổi** (phá `use`); ST-178 ghi “quyết định không đảo”.

### VD-182 · Tối ưu script dev + đo lại #147 (2026-11-18)
> **ĐÃ BỊ THAY một phần bởi VD-197**: câu bắt buộc *"giới hạn nền macOS (tạo tiến trình 9,6×)"* là **giả thuyết**, **bỏ**; #147 **mở lại** chờ đo đủ theo VD-197.
**Q1 = C\*** (đồng ý, kèm điều kiện ràng buộc). (1) **Chỉ tối ưu script dev** — `bin/vis-dev` và `test/report/check_report.py`; **0 dòng** trong `crates/`. (2) Script mới phải cho **output giống hệt** script cũ trên **toàn cây tại cùng HEAD**; chứng minh bằng `diff` rỗng. (3) **≥ 3 ca âm có chủ đích** phải bị **cả bản cũ và bản mới** bắt: tệp vượt 1.500 dòng, báo cáo thiếu mục bắt buộc, báo cáo sai commit hash. (4) **Đo lại soak 10 vòng trên máy khoá** (`~/.vifast-measure.lock`); nếu tỉ lệ mac/ps **≤ 3×** thì **đóng #147**, số mới là số chuẩn; nếu vẫn **> 3×** thì đóng #147 với câu **"giới hạn nền macOS (tạo tiến trình 9,6×)"**. (5) Làm **ngay sau R2, trước R4**.

### VD-183 · HTTP: mặc định timeout/redirect/body (2026-11-18)
**Q2 = A\*** (đồng ý, kèm điều kiện ràng buộc). (1) Mặc định: `timeout_connect = 10 s`, `timeout_read = 30 s` (**giữa hai lần đọc**, **không** phải tổng), `max_redirect = 5` (**chỉ GET**), `max_body = 8 MiB`. (2) Đếm **byte thật đã nhận**, **không** tin `Content-Length`. (3) **Không** cho HTTPS → HTTP downgrade. (4) Bỏ `Authorization`/`Cookie` khi redirect **khác host**. (5) **Chỉ** tham số tường minh, **không** có giá trị "vô hạn"/"tắt". (6) **Mở nợ mới** cho **hạn chót tổng** của một request (xem #148).

### VD-184 · Ngoại lệ VD-146 cho `check.rs:82` (2026-11-18)
**Q3 = A\*** (đồng ý, kèm điều kiện ràng buộc). (1) Đây là **ngoại lệ có tên** cho VD-146, **chỉ đúng dòng đó**. (2) Sửa bằng **`std::path::Path::new(path).file_name()`**, **KHÔNG** tự tách `'/'` + `'\\'`. (3) **Liệt kê mọi chỗ tách đường dẫn thủ công** thành **nợ mới** (#149), **không sửa** trong đợt này. (4) Tên task **`VS3-5-pre`**, làm **sau khi đóng pha 1**. (5) Ca khoá: tương đối, tuyệt đối `/…`, `#[cfg(windows)]` `C:\…` và `C:/…`, `#[cfg(unix)]` tên tệp chứa `\`; self-test **12/12** trên psvn.

### VD-185 · Mở VS3 chính thức (2026-11-18)
**Q4 = A\*** (đồng ý, kèm điều kiện ràng buộc). (1) **VS3 chính thức mở** (thoả VD-148) nhưng **pha 1 vẫn làm trước**. (2) Agent 1 gửi **trích trước/sau của 4 dấu `[SỬA]`** (QĐ-1 #1, QĐ-4 #2, C2 #3 và #7) **cùng R30**.

### VD-186 · Tài liệu mô tả mã hiện tại (2026-11-18)
**Q5 = B\*** (đồng ý, kèm điều kiện ràng buộc). (1) Tài liệu mô tả **mã hiện tại**, **không** mô tả tương lai. (2) Mọi câu nói về hành vi **chưa triển khai** phải gắn nhãn **`[thiết kế VS3, chưa triển khai]`**. (3) **Mỗi tệp một diff**; **Agent 0 duyệt ở lượt sau**. (4) `HOST-BAO-DAM.md` §4: giữ "macOS còn khe hở" nhưng **thu hẹp vào đường GHI** cho tới khi VS3-QĐ-4 merge; thêm giới hạn **hardlink**; thêm **Windows mức bảo đảm thấp hơn**. (5) `docs/ViScript-thiet-ke.md`: **bỏ** văn bản registry tập trung / `vis.lock` chứa khoá; **thay** bằng mô hình phi tập trung VD-180/VD-181, gắn nhãn `[thiết kế VS3-6, chưa triển khai]`.

### VD-187 · Thứ tự pha 1 + tiêu chí đóng pha 1 (2026-11-18) — **R5b huỷ bởi VD-210 (2026-11-18)**
**Q6 = A**. (1) **Thứ tự**: **R2 → VD-182 → R4 → R5b → R5d**. (2) **R2 và R4 mỗi cái tách ≥ 3 task ≤ 3 h**, **mỗi task một commit**, **gate xanh**. (3) Cú pháp bị bỏ phải **báo lỗi kèm mã + gợi ý** (`none` ⇒ **VS0203**, đăng ký trong `API-CODES.txt`; R5b dùng **VS0205**); **migrate toàn bộ docs/cheatsheet/ví dụ trong cùng task**. (4) **Tiêu chí đóng pha 1**: **30/30 task vs-ergo DONE**; gate **PASS 10/10** trên mac **và** ps, **clone sạch**, **không chạy lại**; docs xanh; **bảng old→new→mã lỗi**.

### VD-188 · Windows "hỗ trợ thật" (2026-11-18)
**Q7 = A\*** (đồng ý, kèm điều kiện ràng buộc). (1) "Hỗ trợ thật" trên Windows = **VS3-5a..5e**. (2) Danh sách kỳ vọng theo nền **phải kèm lý do + số nợ**, **Agent 0 duyệt từng ca**. (3) **Không `#[ignore]`**. (4) 5a/5d **chỉ** làm **lấp khe** trong pha 1. (5) **VD-146 vẫn hiệu lực**, trừ **VD-184**.

### VD-189 · VS3-6 gói (2026-11-18)
**Q8 = A\*** (đồng ý, kèm điều kiện ràng buộc). (1) VS3-6 theo **VD-180/VD-181**, tách **≤ 3 h** theo **4 ca đỏ-trước**: tag đổi nội dung ⇒ **VS0707**; **MVS** `v1.2`/`v1.4` ⇒ chọn `v1.4`; gói `vifast.org/vis/…` trỏ GitHub tải đúng; **mạng tắt + có cache** ⇒ build chạy. (2) **Chỉ** thiết kế sum/proxy + client. (3) **#142**: `pkg update X` **chỉ X + phụ thuộc bắt buộc**, bump tối thiểu, **không** VS0704. (4) `vis.lock` định dạng cũ ⇒ **lỗi rõ + lệnh tạo lại**, **không** di trú ngầm.

### VD-190 · #145 — chan(0) rendezvous (2026-11-18)
**Q9 = A**. Sửa chú thích sai + **bỏ chặn runtime** trong **cùng commit** với **chan(0) rendezvous** + ca khoá.

### VD-191 · #140/#109b — openat (2026-11-18)
**Q10 = A**. Đường **đọc + ghi dùng chung một hàm** đi **từng thành phần từ fd gốc**; **điểm chèn `#[cfg(test)]` tất định** như #146 (đỏ **100/100** trên mã cũ, xanh **100/100** trên mã mới, **mac + Linux**); **cùng commit** cập nhật `HOST-BAO-DAM.md`.

### VD-192 · psvn `audit_span` (2026-11-18)
**Q11 = A**. `audit_span` trên psvn thành **nợ mới** (#150) kèm log **clone sạch**; **không** chặn mac/Linux.

### VD-193 · Khoảng trắng quanh `.` (2026-11-18)
**Q12 = A**. (1) Khoảng trắng quanh `.` **không đổi nghĩa** (`5.bit_or(3)` ≡ `5. bit_or(3)`). (2) Float **cần chữ số sau dấu chấm** (`5.0`, **không** `5.`); nếu `5.` hiện hợp lệ thì **đổi thành lỗi gợi ý `5.0`**. (3) **fmt chuẩn hoá không cách sau `.`**. (4) **Pha 2**.

### VD-194 · Xác nhận 9 nhóm quyết định / 16 `[SỬA]` (2026-11-18)
**Q13 = A\*** (đồng ý, kèm điều kiện ràng buộc). Xác nhận **9 nhóm quyết định / 16 dấu `[SỬA]`**; **sửa lại câu chữ** trong `DECISIONS.md` cho khớp.

### VD-195 · `SO-NO.md` §8 phải khớp §4 (2026-11-18)
**Q14 = A**. (1) Bảng tóm tắt **§8** phải **bằng** số đếm thật của bảng **§4**. (2) Thêm **luật vào `./vis-dev check`**: §8 lệch §4 ⇒ **đỏ**, kèm **ca âm**. (3) **Gộp vào task VD-182** (+ **≤ 30 phút**).

### VD-196 · Thẩm quyền Agent 1 trong đợt này (2026-11-18)
Agent 1 được **tự quyết việc ghi sổ** (số hàng QUEUE, cách tách task, thứ tự trong cùng một VD) nhưng **KHÔNG** được **đổi thứ tự** hay **đổi phạm vi** của VD-182…VD-195. Nếu bị chặn: ghi **`BLOCKED.md`** và **chuyển sang việc lấp khe**, không ngồi chờ.

### VD-197 · Q1 · Nợ #147 → câu chữ **B**, nhưng **mở lại cho tới khi đo đủ** (2026-11-18)
(1) **Sửa VD-182**: **bỏ** câu bắt buộc *"tạo tiến trình 9,6×"* — đó là **giả thuyết**, không phải kết luận đã đo. Kết luận #147 phải viết theo **số đo thật**, **chia theo từng thành phần**. (2) **Mở lại #147** (trạng thái **mở, chờ đo**). Đo lại **đúng cách đo gốc**, trên **mac và ps**, **cùng HEAD**, **máy rảnh**, **không có agent nào khác chạy**: **soak 10 vòng**; **cổng `--fast`**; **cargo test tách riêng thời gian biên dịch và thời gian chạy test**. (3) **Đóng #147** bằng **bảng chia thành phần**: mỗi phần ghi **giây mac, giây ps, tỉ lệ, nguyên nhân đã đo**. Nguyên nhân phải **đo được**, không suy luận — nói "bash 3.2 chậm" thì phải có **phép đo vi mô cùng một đoạn script trên hai bản bash**. (4) **Sửa quy tắc ngưỡng** (thay **VD-177 §2b**): tỉ lệ **3×** chỉ áp khi **thời gian mac của phép đó ≥ 10 s**; phép **dưới 10 s** chỉ ghi số, **không mở nợ** (nên `rules` 1,27 s và `report-check` 0,78 s **không còn là nợ**). (5) Thêm **task đo ≤ 1 h** vào QUEUE, đặt **trước ST-210** để số đo của R30 dùng làm **số chuẩn**.

### VD-198 · Q2 · Nội dung gói → **B: viết thẳng vào §8**, kèm nợ mới (2026-11-18)
(1) **Viết thẳng vào §8**. Sinh lại các tệp kiểm thử tài liệu **một lần chung**, gộp với 2 tệp R2a đã phải sinh lại; đây là **sinh máy**, không phải sửa kỳ vọng nên **được phép**; commit message **liệt kê đủ tên tệp sinh lại**. (2) **Mở nợ mới**: tệp kiểm thử tài liệu đặt tên theo **số dòng** là **thiết kế giòn** — chèn một đoạn văn là phải sinh lại cả loạt, và rất dễ **che giấu một thay đổi kỳ vọng thật** giữa đám tệp đổi tên. Hướng sửa: đặt tên theo **định danh ổn định** (slug tiêu đề mục + số thứ tự khối trong mục, hoặc nhãn tường minh trong khối). Xếp **pha 2**, **không** chen pha 1.

### VD-199 · Q3 · "§4" của `HOST-BAO-DAM.md` → **A: không đổi tiêu đề**, sửa chỗ trỏ (2026-11-18)
(1) Giữ tiêu đề **`### FsGuard`**, **không** thêm số mục. (2) Sửa **mọi chỗ trỏ** "HOST-BAO-DAM.md §4" trong `SO-NO.md`, `DECISIONS.md` (**chỉ phần ghi chú**, **không** sửa nguyên văn VD cũ) và `QUEUE.md` thành **"HOST-BAO-DAM.md mục FsGuard"**; grep đủ, **liệt kê số chỗ đã sửa** trong commit message. (3) **Từ nay** trỏ vào tài liệu bằng **tên mục**, **không** bằng số mục tự đặt.

### VD-200 · Q4 · 3 chỗ sửa ngoài §4 → **giữ cả 3** (2026-11-18)
(1) Đúng nguyên tắc **VD-186**: tài liệu bảo đảm phải mô tả **mã đang chạy**; để bảng nền tảng ghi "Windows: chưa đo" là **sai**. (2) Điều kiện: commit message **liệt kê từng chỗ ngoài phạm vi** kèm **một câu lý do**. (3) Riêng **2 gạch "Sai/Đúng"** ở mục "Cấm ngôn từ": nếu **chỉ sửa ví dụ** cho khớp cú pháp mới thì **giữ**; nếu **đổi quy tắc** ngôn từ thì **tách ra, gửi A0 riêng**.

### VD-201 · Q5 · Câu "macOS còn khe hở" → **xác nhận** (2026-11-18)
Đúng ý A0: giữ **ý** *"còn khe hở ở đường ghi trên Unix (kể cả macOS), cho tới khi VS3-QĐ-4 merge"*. A0 **không** yêu cầu tìm lại câu nguyên văn. Gạch mới **phải ghi số nợ #140/#109b** để người đọc lần ra được.

### VD-202 · Q6 · Số liệu Windows → **giữ số, ghi nguồn**, nhưng **đối chiếu self-test trước** (2026-11-18)
(1) Giữ số, mỗi số ghi kèm **"R26 · HEAD · ngày đo"**. **Không** đo lại hôm nay (Windows vẫn "chỉ đo", không đáng tốn máy psvn lúc này). (2) **Bắt buộc trước khi áp patch**: đối chiếu số **self-test** — báo cáo trước ghi **11/12** (ca 7 đỏ), bản thảo ghi **12/12**. Nếu R26 thật sự 12/12 thì **giải thích vì sao**; nếu không thì **sửa thành 11/12** và trỏ **#149/ST-211**; ghi kết quả đối chiếu vào commit message. (3) Quy tắc chung: **mọi con số trong tài liệu A0 sở hữu phải lần được về một log cụ thể**.

### VD-203 · Q7 · Nhãn `[thiết kế VS3-6]` → **giữ 4 chỗ** (2026-11-18)
**Không** gắn từng đoạn. Bổ sung: **blockquote đầu mục** ghi rõ **"Toàn bộ mục này là thiết kế, chưa triển khai"**, kèm **câu trạng thái hiện tại** của lệnh `pkg`. Khi **VS3-6 merge**, xoá **cả 4 nhãn trong cùng commit** (giống quy tắc VD-186).

### VD-204 · Q8 + Q9 · Hiển thị `None`/`Some` và enum std thật → **đổi trong R2b, khai thật**, **chia R2b** (2026-11-18) — **nghĩa đã được làm rõ bởi VD-208**
(1) **Q9**: `Option`/`Result` khai thành **enum std thật**, đúng như tài liệu thiết kế R2. **Không** chấp nhận để tài liệu nói một đằng, mã làm một nẻo. (2) **Q8**: đổi hiển thị thành **`None`, `Some(1)`, `Ok(…)`, `Err(…)`**. Áp cho **mọi đường in**: `print`/`str(x)`, chuỗi nội suy, thông báo lỗi, REPL, LSP hover. Sinh lại **~6 tệp kết quả chuẩn**, **mỗi tệp chỉ được đổi đúng chuỗi này**; Agent 2 **diff và xác nhận không có thay đổi nào khác**. (3) **R2b quá lớn cho 3 h** ⇒ chia: **R2b-1** (≤3 h) enum thật + đổi hiển thị + sửa cheat-sheet (VD-205); **R2b-2** (≤3 h) `T??` + tự gói nhiều lớp; **R2c** (2 h) `first`/`map.get` (giữ nguyên). (4) **Ca khoá cho R2b-1** (đỏ-trước nếu là hành vi mới, nếu không thì là **ca hồi quy phải xanh trước và sau**): match trên `Option`/`Result` vẫn **bắt buộc đủ nhánh**; người dùng **không được** khai lại hoặc che tên `Option`/`Result`/`Some`/`None`/`Ok`/`Err` ở **phạm vi toàn cục** (lỗi **có mã**); toán tử `?` **hành xử không đổi**. (5) **Hiệu năng**: chạy lại một **benchmark vi mô** hiện có liên quan đến Option (nếu chưa có thì viết một cái nhỏ), **trước và sau**; chậm hơn **> 10 %** thì **ghi nợ và báo A0**, **không** tự tối ưu trong R2b-1. (6) Pha 1 **tăng khoảng 3 h** — chấp nhận.

### VD-205 · Q10 · Cheat-sheet → **sửa ngay** + thêm luật cổng (2026-11-18)
(1) Sửa **7 dòng** trong **commit đầu tiên của R2b-1**. (2) Đây là **chỗ sót của R2a** (VD-187 yêu cầu migrate trong cùng task) — ghi vào báo cáo như một **sai lệch quy trình**, **không** cần làm lại R2a. (3) **Luật cổng mới** (gộp vào R2b-1, **≤ 30 phút**): mỗi cú pháp bị bỏ ở pha 1 có một mục trong **danh sách cấm** (bắt đầu bằng `none` dùng như từ khoá); cổng quét **cả văn xuôi lẫn khối mã** trong `docs/` và cheat-sheet; gặp thì **đỏ**. **Ngoại lệ duy nhất**: **bảng cú pháp cũ → mới → mã lỗi** (tiêu chí đóng pha 1 số 4) và các ví dụ được **đánh dấu tường minh** là "cú pháp sai". **Có ca âm**. (4) **R4, R5b, R5d** khi bỏ cú pháp nào thì **thêm mục vào danh sách cấm trong cùng task**.

### VD-206 · Quy tắc đo hiệu năng (2026-11-18) — **bổ sung bởi VD-217** (đo A/B xen kẽ cho câu hỏi tỉ lệ)
(1) Số dùng làm **chuẩn** (ghi sổ nợ, tài liệu, báo cáo R) phải đo khi **không có agent khác chạy trên cùng máy**, và **ghi rõ điều đó trong log**. (2) Số đo khi máy có tải chỉ được ghi là **"tham khảo"**. (3) **Agent 1 điều phối lịch máy**, để mỗi lần đo chuẩn có một **khoảng máy rảnh**.

### VD-207 · Thứ tự tiếp theo (2026-11-18)
| # | Task | Nội dung | VD | Ước lượng |
|---|---|---|---|---|
| 1 | **R2b-1** | Enum thật + hiển thị + cheat-sheet + luật danh sách cấm | 204, 205 | 3 h |
| 2 | **R2b-2** | `T??` + tự gói nhiều lớp | 204 | 3 h |
| 3 | **R2c** (ST-203) | `first` / `map.get` ba trạng thái | 204 | 2 h |
| 4 | **R4a/b/c** (ST-205…207) | `Error` record + `Json` enum + `AnyError` | 187 | 8 h |
| 5 | **R5b** (ST-208) | `{…}` chỉ map + VS0205 + thêm danh sách cấm | 187, 205 | 2 h |
| 6 | **R5d** (ST-209) | `--b` chỉ ở chế độ lệnh + xung đột tên | 187 | 2,5 h |
| 7 | **Đo #147 chuẩn** | soak + cổng + cargo test tách biên dịch, **máy rảnh** | 197, 206 | 1 h |
| 8 | **ST-210** | Đóng pha 1, R30 | 185, 187 | 1 h |
| 9 | **ST-211** | VS3-5-pre (`check.rs`) | 184 | 1 h |
| 10 | **ST-212** | VS3-QĐ-1 | 185 | theo thiết kế |

**Agent 1 làm song song**: áp 2 patch VD-186 theo **VD-198…VD-203**. Thứ tự: **đối chiếu self-test (VD-202) → viết §8 → sinh lại tệp kiểm thử tài liệu → chạy cổng**. Áp xong gửi A0 **diff cuối** (chỉ phần khác bản thảo) trong báo cáo kế tiếp. **Không cần chờ A0 duyệt lại trước khi commit**, vì nội dung đã được duyệt ở đây.

### VD-208 đến VD-214 (2026-11-18) — dán nguyên văn A0 (Agent 1 chép, commit cùng ST-217)

## 1. CÁC QUYẾT ĐỊNH

### VD-208 · Mục 1 · Option/Result ở tầng kiểu → **A**, kèm tiêu chí "người dùng không phân biệt được"

- Nghĩa của "enum std thật" trong VD-204 (1) được làm rõ như sau: **mọi hành vi người dùng quan sát được**
  phải giống một enum khai trong std. Cách cài đặt bên trong trình kiểm kiểu thì không bắt buộc.
- Tầng chạy (Value::Enum) giữ như ST-217. Tầng kiểu giữ Type::Opt và kiểu dựng sẵn.
- **Không chọn C.** Cờ "std-enum" chỉ làm đẹp mã bên trong, người dùng không được gì, lại thêm một nhánh đặc
  biệt nữa phải bảo trì.
- **Điều kiện đóng ST-217**: có ca khoá, xanh trên cả interp và VM, cho từng điểm dưới đây. Điểm nào đã có ca
  thì trỏ tên ca, không viết lại.
  1. match trên Option/Result bắt buộc đủ nhánh, thiếu nhánh thì báo lỗi.
  2. `Some`, `None`, `Ok`, `Err` dùng được như hàm (truyền vào map, gán vào biến), giống constructor của enum
     người dùng. Nếu enum người dùng làm được mà Option không làm được thì phải ghi rõ trong ca và báo A0.
  3. Không được khai lại hay che tên Option/Result/Some/None/Ok/Err ở phạm vi toàn cục.
  4. Thông báo lỗi và LSP hover vẫn in `int?`, giữ đúng VD-204 (2).
- **Tài liệu**: sửa dòng 818 của docs/ViScript-thiet-ke.md thành ý: "Option/Result hành xử như enum std
  (tầng chạy là enum thật); trình kiểm kiểu xử lý như kiểu dựng sẵn để hiển thị `T?` và suy luận nhanh."
  Gửi A0 diff 1 dòng trong báo cáo kế tiếp. **Không cần chờ duyệt mới commit**, vì nội dung đã chốt ở đây.
- **Phương án B** thành nợ mới, trạng thái "không xếp lịch". Chỉ mở lại khi có tính năng thật sự cần (ví dụ
  người dùng viết `impl` trait cho Option). Ghi luôn chi phí và rủi ro đã ước lượng (1 đến 2 ngày, 10 đến 20
  golden đổi byte, hop_nhat chậm thêm) để lần sau không phải phân tích lại.

### VD-209 · Mục 2 · Nợ #152 chậm hơn 10 % → **(c) đo chuẩn trước, rồi (b) nếu cần**

- Gộp phép đo opt-bench vào **khung đo chuẩn #147** (VD-197, đặt trước ST-210). Máy rảnh, không agent nào
  khác chạy, 10 lần mỗi phép, lấy trung vị và ghi khoảng min đến max.
- Nếu số chuẩn **≤ 10 % trên cả VM và interp** thì đóng #152 với số đo.
- Nếu **> 10 %** thì xếp task tối ưu ctor-enum (dùng chỉ số hằng thay cho so chuỗi tên, ≤ 3 h) **ngay sau khi
  đóng pha 1**, trước VS3-QĐ-1. Không làm trong pha 1.
- Lý do không bỏ qua (a): VM là đường chạy nhanh, nên +23 % ở VM đáng lo hơn +10 % ở interp. Cách so chuỗi tên
  biến thể còn dính tới lỗi ở VD-212, nên rất có thể sửa VD-212 sẽ tự làm giảm phần chậm này. Phải đo lại sau
  VD-212.

### VD-210 · Mục 3 · R5b → **(ii) huỷ R5b**, chốt quy tắc khoá trong `{…}` — **BẢNG THAY BỞI VD-215**

**Lý do**: bản ghi ẩn danh là tính năng cốt lõi cho một ngôn ngữ script. Nó phục vụ JSON, pipeline `select`,
và giá trị trả nhanh từ hàm. R5b sinh ra để tránh nhầm map với record, nhưng đổi lại phải bỏ một tính năng có
ích và phá ca khoá ST-53. Cách tốt hơn là **chốt một quy tắc cú pháp rõ ràng** để không còn nhầm:

| Viết | Nghĩa |
|---|---|
| `{ten: 1}` (khoá là định danh trần) | **bản ghi**, `ten` là tên trường, **không bao giờ** là biến |
| `{"ten": 1}` (khoá là chuỗi literal) | **map** |
| khoá là giá trị của biến hoặc biểu thức | **map**, dùng cú pháp khoá tính toán hiện có của ViScript |
| trộn định danh trần và khoá chuỗi trong cùng `{…}` | **lỗi có mã**, gợi ý chọn một loại |

Thực hiện:
- **Agent 2 kiểm hiện trạng trước**, ghi vào task:
  - cú pháp khoá tính toán của map hiện là gì;
  - `{k: v}` khi `k` là biến trong phạm vi hiện cho ra gì;
  - trộn hai loại khoá hiện có bị báo lỗi không.
  Chỗ nào chưa khớp bảng trên thì sửa trong task này. Nếu chưa có cú pháp khoá tính toán thì **dừng và báo A0**,
  không tự chọn cú pháp.
- **Lỗi có gợi ý**: khi bản ghi ẩn danh được dùng ở chỗ cần map (hoặc ngược lại), thông báo lỗi phải gợi ý cách
  viết đúng. Có ca khoá.
- **ST-53 giữ nguyên**, không đổi ca khoá nào của json_sort.rs.
- **VS0205**: giữ số trong API-CODES.txt, đánh dấu "dành riêng, không dùng", **không** cấp lại cho lỗi khác.
- Sửa vs-ergo §R5 thành "R5b huỷ (VD-210)" và trỏ bảng trên. Cập nhật cheat-sheet với bảng trên trong cùng
  commit.
- ST-208 đổi thành task mới **R5b′** gồm kiểm hiện trạng, sửa theo bảng, lỗi gợi ý và tài liệu. Ước lượng ≤ 2 h.

### VD-211 · Mục 4 (a) · `Foo.A(5) == Bar.A(5)` nay là false → **chấp nhận**, kèm kiểm tầng kiểu

- Hành vi mới đúng: hai enum khác loại thì không bằng nhau.
- Agent 2 kiểm thêm: trình kiểm kiểu hiện **có chấp nhận** biểu thức `==` giữa hai kiểu enum khác nhau không.
  Nếu có thì mở nợ (pha 2) để báo lỗi kiểu lúc biên dịch: so hai kiểu khác nhau gần như luôn là lỗi của người
  viết. Không sửa trong pha 1.
- Thêm một ca khoá cho hành vi mới.

### VD-212 · Mục 4 (b) · VM khớp biến thể **không phân biệt hoa thường** → **PHỦ QUYẾT**

Đây là lỗi thiết kế, không phải sửa lệch nhỏ. Nó **đi ngược trực tiếp R2a**: R2a vừa biến `none` thành lỗi
VS0203 để chỉ còn một cách viết. Nếu pattern `ok(s)` vẫn âm thầm khớp `Ok`, thì `some(x)`, `none`, `err(e)`
viết thường vẫn chạy được trong pattern. Kết quả là có hai cách viết cùng một thứ, đúng điều VD-204 (2) muốn
tránh. Khớp không phân biệt hoa thường còn có thể làm hai biến thể `A` và `a` của enum người dùng va nhau.

Quyết định:
- **Cả interp lẫn VM khớp biến thể phân biệt hoa thường.** Sửa interp cho đúng, không sửa VM theo interp.
- Pattern viết sai hoa thường (ví dụ `ok(s)`, `some(x)`) phải báo **lỗi lúc biên dịch có mã + gợi ý** dạng đúng
  (`Ok(s)`). Dùng VS0203 nếu ngữ nghĩa khớp, nếu không thì đăng ký mã mới.
- Thêm `some`, `ok`, `err` (viết thường, dùng trong pattern) vào **danh sách cấm** của VD-205.
- Ca khoá đỏ-trước, xanh trên **cả interp và VM**: `ok(s)`, `some(x)`, `none` trong pattern; hai biến thể
  `A`/`a` của cùng enum người dùng khớp đúng biến thể.
- Nếu có ca .vis hiện hành nào đang dựa vào khớp không phân biệt hoa thường thì migrate trong cùng commit và
  liệt kê trong commit message.
- Làm **ngay**, gộp vào lúc đóng ST-217, vì đây là phần còn thiếu của R2a/R2b-1. Đo lại opt-bench sau khi sửa
  (VD-209).

### VD-213 · Mục 4 (c) · `let-err` ra VS0412 → **chấp nhận**, kèm gợi ý

Đúng: `let Ctor(…) = …` là pattern. Điều kiện: thông báo VS0412 trong trường hợp này phải có gợi ý cho người
dùng (ví dụ: dùng `match` hoặc `if let` khi pattern có thể không khớp). Có ca khoá kiểm câu gợi ý.

### VD-214 · Thứ tự tiếp theo

| # | Task | Nội dung | VD | Ước lượng |
|---|---|---|---|---|
| 1 | **Đóng ST-217** | 4 ca tiêu chí VD-208 + sửa hoa thường VD-212 + dòng 818 + VD-211, VD-213 | 208, 211–213 | ≤ 3 h |
| 2 | R2b-2 | T?? + tự gói nhiều lớp | 204 | 3 h |
| 3 | R2c | first / map.get ba trạng thái | 204 | 2 h |
| 4 | R4a/b/c | (R4a có thể đã chạy, cứ tiếp) | 187 | 8 h |
| 5 | **R5b′** | Quy tắc khoá `{…}` theo VD-210 | 210 | ≤ 2 h |
| 6 | R5d | --b chỉ ở chế độ lệnh + xung đột tên | 187 | 2,5 h |
| 7 | Đo chuẩn | #147 + opt-bench (#152), máy rảnh | 197, 206, 209 | 1,5 h |
| 8 | ST-210 | Đóng pha 1, R30 (bảng old→new→mã ghi theo VD-208/210/212) | 185, 187 | 1 h |
| 9 | Tối ưu ctor-enum | Chỉ khi #152 vẫn > 10 % | 209 | ≤ 3 h |
| 10 | ST-211, ST-212 | VS3-5-pre, VS3-QĐ-1 | 184, 185 | như cũ |

Nếu Agent 2 đang làm R4a thì làm xong R4a rồi mới chuyển sang bước 1. Không bỏ dở giữa chừng.

---

## 2. VIỆC AGENT 1 CẦN LÀM NGAY

1. Ghi VD-208 … VD-214 vào DECISIONS.md. Ghi chú ở VD-204 (1) rằng nghĩa đã được làm rõ bởi VD-208. Ghi chú ở
   VD-187 rằng R5b đã bị huỷ bởi VD-210.
2. SO-NO.md: mở nợ "tầng kiểu Option/Result (phương án B), không xếp lịch". Mở nợ VD-211 nếu tầng kiểu chấp
   nhận `==` khác kiểu. #152 giữ mở, chờ đo chuẩn.
3. QUEUE.md: bỏ BLOCKED của ST-217, đổi ST-208 thành R5b′, thêm task đo opt-bench vào khung đo chuẩn.
4. API-CODES.txt: đánh dấu VS0205 là dành riêng.
5. Giao Agent 2 theo §1 VD-214.

**Báo cáo kế tiếp gửi kèm**:
- kết quả kiểm hiện trạng `{…}` (VD-210);
- danh sách ca .vis đã migrate vì sửa hoa thường (VD-212);
- opt-bench sau VD-212 (số tham khảo cũng được).

> **Nghĩa của VD-204 (1) đã được làm rõ bởi VD-208** (2026-11-18): Option/Result là enum std ở tầng chạy; tầng kiểu giữ dựng sẵn.

### VD-215 đến VD-219 (2026-11-18) — dán nguyên văn A0 (Agent 1 chép, commit cùng [ST-208/VD-215])

## 1. CÁC QUYẾT ĐỊNH

### VD-215 · Mục 1 · Quy tắc `{…}` và khoá tính toán

**Không thêm ký hiệu mới.** `(expr)` đã chạy được và dễ đọc: dấu ngoặc nghĩa là "tính giá trị này". A0 chốt
`(expr)` là **cú pháp khoá tính toán chính thức**.

Bảng chốt (thay bảng VD-210):

| Viết | Nghĩa |
|---|---|
| `{ten: 1, tuoi: 2}` (mọi khoá là định danh trần) | **bản ghi** |
| `{"ten": 1}` (khoá chuỗi literal) | **map** |
| `{(k): 1}`, `{(k + "_id"): 1}` (khoá trong ngoặc) | **map**, khoá là giá trị của biểu thức |
| trộn chuỗi và `(expr)` | **map**, hợp lệ |
| trộn định danh trần với chuỗi hoặc `(expr)`, **bất kể thứ tự** | **lỗi có mã** |
| `{}` | giữ đúng hiện trạng. Agent 2 ghi hiện trạng vào task, **không đổi** trong task này |

Thực hiện:
1. **Chế độ literal xét toàn bộ khoá**, không chỉ khoá đầu. Parser có thể vẫn đoán theo khoá đầu, nhưng phải
   kiểm hết các khoá trước khi chốt.
2. **Một mã lỗi cho cả hai chiều trộn** (đăng ký mã mới, **không** dùng lại VS0205). Thông báo lỗi nói rõ
   khoá nào lệch, kèm gợi ý theo khoá đầu tiên:
   - khoá đầu là định danh trần: "bản ghi cần mọi khoá là định danh; muốn map thì viết `"b"` hoặc `(b)`";
   - khoá đầu là chuỗi hoặc `(expr)`: "map cần khoá chuỗi hoặc `(expr)`; `b` trần ở đây sẽ không phải biến,
     hãy viết `"b"` hoặc `(b)`".
   - `{a: 1, "b": 2}` hiện là **lỗi cú pháp chung**, phải đổi sang mã lỗi mới có gợi ý.
3. **`{[k]: 1}`**: giữ hợp lệ (danh sách làm khoá vẫn là giá trị đúng), nhưng phát **cảnh báo có mã** kèm gợi ý
   `{(k): 1}`. Lý do: người quen JavaScript sẽ viết đúng như vậy và nhận về một map khoá danh sách mà không
   biết. Tắt cảnh báo bằng cách viết `{([k]): 1}`, ghi cách này trong thông báo.
4. **Ca khoá** (đỏ-trước nếu là hành vi mới), chạy cả interp và VM:
   - `{"a": 1, b: 2}` và `{a: 1, "b": 2}`: cùng mã lỗi;
   - `{(k): 1}` với `k = "a"`: ra map `{"a": 1}`;
   - `{k: 1}` với biến `k` trong phạm vi: ra bản ghi trường `k` (giữ hành vi);
   - `{"a": 1, (k): 2}`: map hợp lệ;
   - `{[k]: 1}`: cảnh báo + vẫn ra map khoá danh sách; `{([k]): 1}`: không cảnh báo;
   - ST-53 và json_sort.rs: xanh, không đổi.
5. **Migrate**: grep toàn repo (test .vis, chuỗi .vis nhúng trong Rust test, docs, ví dụ) tìm literal trộn
   loại khoá. Sửa trong cùng commit, liệt kê trong commit message. Literal trộn có thể đang nằm trong golden,
   đọc kỹ trước khi sinh lại.
6. **Tài liệu**: cheat-sheet và docs ghi bảng trên. Thêm `{[k]: v}` (dạng JavaScript) vào bảng "cú pháp cũ →
   mới → mã" của R30 dưới dạng "bẫy thường gặp".
7. ST-208 (R5b′) ước lượng nâng lên **≤ 3 h**.

### VD-216 · Mục 2 · VS0111

- Sửa SPEC-lex §7, dòng VS0111: ghi "**ngừng phát từ VD-212**; sai hoa thường trong pattern nay báo VS0203".
  **Giữ số**, không cấp lại cho lỗi khác. API-CODES.txt đánh dấu tương tự.
- **Xoá bộ lọc VS0111** trong `vis check`/`--json`. Mã chết nói về một mã lỗi không còn tồn tại sẽ làm người đọc
  sau tưởng nó còn phát. Điều kiện: commit message ghi kết quả grep chứng minh không còn chỗ nào phát VS0111.
  Cổng phải xanh, 0 golden đổi.
- Agent 1 áp cả hai trong một commit, gửi A0 diff SPEC trong báo cáo kế tiếp. Không cần chờ duyệt.

### VD-217 · Mục 3 · #152 và cách đo khi máy không rảnh được → **(c)**, thêm cách đo A/B xen kẽ

VD-206 giả định có máy rảnh. Nếu ps không bao giờ rảnh, quy tắc đó không thực hiện được. A0 sửa VD-206 như sau.

**Với câu hỏi về tỉ lệ** (bản mới chậm hơn bản cũ bao nhiêu %, như #152), số chuẩn được phép đo bằng
**A/B xen kẽ**:
- Build hai binary: commit gốc trước enum-hoá và HEAD.
- Chạy **xen kẽ** A, B, A, B… mỗi bên **≥ 20 lần** trong cùng một phiên. Nhiễu tải khi đó ảnh hưởng gần như
  đều cho cả hai.
- Báo **trung vị tỉ lệ B/A** kèm khoảng min đến max. Ghi rõ "A/B xen kẽ, máy có tải" trong log.
- Nếu khoảng tỉ lệ cắt ngang ngưỡng 10 % (ví dụ 1,06 đến 1,14) thì kết luận "**chưa rõ**". Khi đó vẫn giữ
  "chờ đo chuẩn".

**Với câu hỏi về thời gian tuyệt đối** (#147, so mac với ps): vẫn cần máy rảnh. Agent 1 **xếp một khung 20
phút dừng mọi agent trên ps**, ví dụ đầu hoặc cuối một phiên chạy qua đêm. Ghi giờ khung đó vào QUEUE.

Áp cho #152:
- Agent 2 đo A/B xen kẽ opt-bench (VM và interp) **ngay**. Việc này ≤ 30 phút, không phải chờ máy rảnh.
- Trung vị ≤ 1,10 và cả khoảng ≤ 1,10 thì đóng #152.
- Trung vị > 1,10 và cả khoảng > 1,10 thì giữ lịch tối ưu ctor-enum **sau đóng pha 1** (VD-214).
- Còn lại thì giữ "chờ đo chuẩn" như (c).
- Số tham khảo hiện có (VM 0,19 đến 0,20 s) **không** dùng làm mốc.

### VD-218 · Mục 4 · `$value:<tên>` → **ghi nhận**, thêm điều kiện không lộ tên nội bộ

Cách vá hợp lý: không đụng VBC1, người dùng không gõ được tên này. Điều kiện:
- Tên `$value:…` **không được xuất hiện** ở bất kỳ đâu người dùng nhìn thấy: thông báo lỗi, stack trace, in giá
  trị hàm (`print(Some)`), LSP hover, lệnh disassemble hoặc debug ở chế độ thường.
- Ca khoá: truyền `Some` làm giá trị rồi gây lỗi bên trong hàm nhận. Stack trace và thông báo phải ghi `Some`,
  không ghi `$value:Some`. Kiểm cả `print(Some)` cho interp và VM ra **cùng một chuỗi**.
- Ghi cơ chế này vào tài liệu nội bộ của VM (một đoạn), kèm lý do: VBC1 đóng băng, không thêm opcode.

### VD-219 · Thứ tự tiếp theo

| # | Task | Nội dung | VD | Ước lượng |
|---|---|---|---|---|
| 1 | ST-205 (R4a) | Đang làm, tiếp tục | 187 | như cũ |
| 2 | Đo A/B #152 | opt-bench xen kẽ ≥ 20 lần | 217 | 0,5 h |
| 3 | VS0111 + ca `$value` | SPEC §7, xoá bộ lọc, ca khoá VD-218 | 216, 218 | 1 h |
| 4 | R4b, R4c | | 187 | như cũ |
| 5 | ST-208 (R5b′) | Quy tắc `{…}` theo VD-215 | 215 | ≤ 3 h |
| 6 | R5d | | 187 | 2,5 h |
| 7 | Đo chuẩn #147 | Trong khung 20 phút ps rảnh + mac | 197, 217 | 1 h |
| 8 | ST-210 | Đóng pha 1, R30 | 185, 187 | 1 h |

Các task còn lại sau ST-210 giữ như VD-214.

---

## 2. VIỆC AGENT 1 CẦN LÀM NGAY

1. Ghi VD-215 … VD-219 vào DECISIONS.md. Ghi chú: VD-210 (bảng) đã được thay bởi VD-215; VD-206 đã được bổ sung
   bởi VD-217.
2. API-CODES.txt: đăng ký mã lỗi "trộn loại khoá" và mã cảnh báo "`{[k]: …}`"; đánh dấu VS0111 ngừng phát.
3. QUEUE.md: bỏ BLOCKED của ST-208; thêm task đo A/B #152; xếp khung 20 phút ps rảnh và ghi giờ cụ thể.
4. Giao Agent 2 theo §1 VD-219.

**Báo cáo kế tiếp gửi kèm**: tỉ lệ A/B #152 (trung vị và khoảng), diff SPEC §7, danh sách literal trộn khoá đã
migrate, hiện trạng `{}`.

> **Nghĩa của VD-210** (bảng quy tắc `{…}`) đã được **thay bởi VD-215** (2026-11-18): cú pháp `(expr)` là khoá tính toán chính thức; chế độ literal xét **toàn bộ** khoá; trộn loại khoá (bất kể thứ tự) ⇒ một mã lỗi mới (không dùng lại VS0205); `{[k]: v}` ⇒ cảnh báo có mã + gợi ý.
> **VD-206** (đo khi máy rảnh) đã được **bổ sung bởi VD-217** (2026-11-18): câu hỏi về **tỉ lệ** (#152) được phép đo **A/B xen kẽ** (≥ 20 lần mỗi bên, trung vị tỉ lệ B/A + khoảng min..max; nếu cắt ngưỡng 10 % thì "chưa rõ" ⇒ giữ trạng thái chờ; máy vẫn không rảnh thì #152 xử lý đúng VD-217).

### VD-220 · LỆNH CHẠY QUA ĐÊM (2026-11-18, hiệu lực 19:00 → 09:00 sáng 2026-11-19)

# A0 LỆNH CHẠY QUA ĐÊM: VD-220 (dòng ViScript)

**Ngày**: 2026-11-18 · **Người ra lệnh**: Agent 0 · **Gửi**: Agent 1 (leader) → các sub-agent Agent 2
**Khung chạy**: **19:00 hôm nay → 09:00 sáng mai (2026-11-19)**, chạy liên tục, không chờ người dùng.
**Bổ sung cho**: VD-215 … VD-219. Mọi VD cũ vẫn hiệu lực. Khi lệnh này mâu thuẫn với VD cũ thì **VD cũ thắng**,
ghi vào BLOCKED.md.

---

## 0. TÓM TẮT

- Agent 1 được tạo **tối đa 4 sub-agent Agent 2** chạy song song, chia theo **4 làn** ở §2.
- **Chỉ làn A** sửa mã ngôn ngữ (parser, typeck, interp, VM). Các làn khác không đụng vào mã đó, để không xung
  đột và không phải sinh lại golden hai lần.
- **Chỉ Agent 1 merge** vào nhánh chính, lần lượt từng nhánh, cổng xanh sau **mỗi** lần merge.
- Mục tiêu đêm nay: **đóng pha 1 (ST-210, R30)** nếu kịp. Không kịp thì dừng sạch ở ranh giới task và báo cáo.
- **07:30–07:50**: dừng mọi agent trên ps để đo chuẩn #147 (VD-217).
- **08:30**: đóng băng merge. **09:00**: báo cáo nằm sẵn trong repo.

---

## 1. QUYỀN QUYẾT THAY A0 TRONG ĐÊM

Agent 1 được quyết thay A0 **chỉ** các việc sau:
- chia task, đổi thứ tự **trong cùng một làn**, chuyển việc giữa các làn khi có làn rảnh;
- đăng ký mã lỗi/cảnh báo mới đã được VD cho phép (VD-215: mã "trộn loại khoá", mã cảnh báo `{[k]: …}`);
- câu chữ thông báo lỗi và gợi ý, miễn đúng ý VD;
- revert một merge làm cổng đỏ;
- sổ sách: DECISIONS, SO-NO, QUEUE, BLOCKED.

Agent 1 **không** được quyết, gặp phải thì ghi BLOCKED.md và chuyển việc khác:
- chọn **cú pháp mới** hoặc đổi ngữ nghĩa ngoài những gì VD đã chốt;
- sửa **SPEC** ngoài các dòng VD đã chỉ định (VD-216);
- đổi hoặc xoá **ca khoá của A0** (ST-53, json_sort.rs, các ca VD-2xx);
- hạ ngưỡng, đổi cách đo, dùng `#[ignore]`, sửa kỳ vọng test cho xanh;
- bắt đầu **VS3-QĐ-1** (ST-212). Việc này chờ A0 đọc R30 và kiểm 4 dấu (VD-185).

---

## 2. BỐN LÀN SONG SONG

Mỗi làn có **worktree và nhánh riêng** (`dem/A`, `dem/B`, `dem/C`, `dem/D`). Thư mục target build riêng cho
từng làn để không khoá lẫn nhau.

### Làn A · Mã ngôn ngữ (đường găng, chạy tuần tự)

| # | Task | VD | Ước lượng |
|---|---|---|---|
| A1 | ST-205 R4a (tiếp phần đang làm) | 187 | còn lại |
| A2 | R4b | 187 | ≤ 3 h |
| A3 | R4c (AnyError) | 187 | ≤ 3 h |
| A4 | ST-208 R5b′, quy tắc `{…}` | 215 | ≤ 3 h |
| A5 | R5d (`--b` chỉ ở chế độ lệnh) | 187 | 2,5 h |

- Mỗi task một hoặc nhiều commit, mỗi commit qua cổng xanh trên nhánh trước khi xin merge.
- Làn A là **làn duy nhất được sinh lại golden** (.out/.err). Mỗi lần sinh lại, commit message liệt kê tên tệp
  và lý do, đúng quy tắc cũ.
- Bỏ cú pháp nào thì thêm vào **danh sách cấm** trong cùng task (VD-205).
- Ca đỏ-trước của A4 và A5 lấy từ làn D (xem dưới). Làn A không phải tự viết lại.

### Làn B · Đo đạc, VM nhỏ, dọn mã chết

| # | Task | VD | Ước lượng |
|---|---|---|---|
| B1 | Đo A/B xen kẽ #152 (opt-bench, VM + interp, ≥ 20 lần mỗi bên) | 217 | 0,5 h |
| B2 | VS0111: sửa SPEC §7 + API-CODES + xoá bộ lọc chết | 216 | 0,5 h |
| B3 | Ca khoá `$value:` không lộ ra người dùng + đoạn tài liệu VM | 218 | 0,5 h |
| B4 | Chuẩn bị script đo chuẩn #147 (soak, cổng, cargo test tách biên dịch và chạy) để 07:30 chỉ cần bấm chạy | 197 | 0,5 h |

- **Đo A/B (B1) không được chạy cùng lúc với một lần build lớn của làn khác** trên ps. Agent 1 chọn khoảng giữa
  hai lần build. Ghi số lần build song song (nếu có) vào log.
- B3 đụng VM. Nếu B3 cần sửa mã VM chứ không chỉ thêm test, thì **chuyển B3 sang làn A** sau A1, không làm ở
  làn B.
- Xong B1 đến B4, làn B rảnh thì nhận việc của làn C.

### Làn C · Tài liệu và sổ sách (không sửa mã)

| # | Task | VD |
|---|---|---|
| C1 | Bản nháp **bảng "cú pháp cũ → mới → mã lỗi"** cho R30, cập nhật dần theo task làn A đã merge | 187, 215 |
| C2 | Cheat-sheet và docs: thêm bảng `{…}` của VD-215, mục "bẫy thường gặp" `{[k]: v}` | 215 |
| C3 | Sửa dòng 818 docs/ViScript-thiet-ke.md (nếu chưa làm) | 208 |
| C4 | Kiểm khớp sổ: SO-NO §8, QUEUE, DECISIONS ghi chú VD bị thay | 195, 219 |
| C5 | Danh sách chỗ tách đường dẫn thủ công (chỉ lập danh sách, không sửa) | 184 |

- **Tài liệu có tệp kiểm thử đặt theo số dòng** (nợ VD-198). Vì vậy làn C **không tự sinh lại** các tệp đó.
  Agent 1 gom thay đổi docs, sinh lại **một lần** ngay trước khi merge, liệt kê đủ tên tệp.
- Mô tả cú pháp trong docs chỉ được viết theo **mã đã merge**. Nội dung cho task làn A chưa xong thì để trong
  nhánh, chờ task đó merge.

### Làn D · Viết ca đỏ-trước trước cho làn A, và việc lấp chỗ trống

| # | Task | VD |
|---|---|---|
| D1 | Viết **toàn bộ ca khoá của R5b′** theo VD-215 mục 4 (chỉ test, không sửa mã), xác nhận đỏ trên HEAD hiện tại ở đúng những ca phải đỏ | 215 |
| D2 | Viết ca khoá của R5d | 187 |
| D3 | Grep migrate: liệt kê mọi literal trộn loại khoá trong test, chuỗi .vis nhúng trong Rust, docs, ví dụ | 215 |
| D4 | VS3-5a, VS3-5d (chỉ việc lấp chỗ trống, không đụng mã ngôn ngữ) | 188 |

- D1, D2 giao lại cho làn A **dưới dạng nhánh test**, làn A cherry-pick khi bắt đầu A4, A5.
- Ca nào D thấy đỏ mà **không nằm trong danh sách phải đỏ** thì báo Agent 1 ngay, vì đó có thể là lỗi mới.
- D4 chỉ làm khi D1 đến D3 đã xong.

---

## 3. QUY TẮC MERGE VÀ CỔNG

1. Agent 1 merge **lần lượt**, không gộp nhiều nhánh một lúc. Thứ tự ưu tiên khi nhiều nhánh cùng chờ: A, B, D, C.
2. Sau **mỗi** merge: `gate --fast` trên ps phải PASS. Cuối mỗi task làn A, chạy thêm cổng trên **mac**.
3. Cổng đỏ sau merge: sửa trong **≤ 30 phút**, nếu không được thì **revert merge** và trả nhánh về làn đó.
4. Không merge nhánh nào đang đỏ để "sửa sau".
5. Mỗi làn rebase lên nhánh chính sau mỗi lần merge của làn A.
6. Một task kẹt **quá 45 phút** không tiến triển thì ghi BLOCKED.md (đã thử gì, kẹt ở đâu), chuyển sang task kế
   tiếp trong làn.

---

## 4. KHUNG GIỜ CỐ ĐỊNH

| Giờ | Việc |
|---|---|
| 19:00 | Agent 1 tạo 4 sub-agent, giao làn A đến D, ghi giờ bắt đầu vào QUEUE |
| 23:00 | Mốc kiểm 1: Agent 1 ghi tiến độ 4 làn vào `agent/DEM-2026-11-18.md` (vài dòng mỗi làn) |
| 03:00 | Mốc kiểm 2: như trên. Nếu làn A chậm hơn kế hoạch > 2 h thì quyết sớm task nào bỏ lại cho ngày mai (theo §5) |
| **07:30–07:50** | **Khung đo chuẩn**: dừng mọi sub-agent trên ps, không build, không test. Chạy script B4 trên ps, cùng lúc đo trên mac, cùng HEAD. Ghi log có chữ "máy rảnh" |
| 07:50 | Tiếp tục các làn |
| **08:30** | **Đóng băng merge**. Không bắt đầu task mới. Task đang dở thì commit trên nhánh làn, **không** merge |
| 08:30–09:00 | Agent 1 viết báo cáo §6, cổng cuối trên ps và mac tại HEAD cuối |

---

## 5. NẾU PHA 1 XONG TRƯỚC SÁNG, HOẶC KHÔNG KỊP

**Nếu A5 merge trước khoảng 06:30**:
1. ST-210: chạy tiêu chí đóng pha 1 (VD-187), cổng trên clone sạch 10/10 trên mac và ps, viết R30 kèm 4 dấu kiểm
   ngẫu nhiên (VD-185) và bảng old→new→mã của làn C.
2. ST-211 (VS3-5-pre, sửa check.rs theo VD-184), làm sau khung đo 07:30 nếu còn giờ.
3. **Dừng ở đó.** Không bắt đầu ST-212. Thời gian còn lại dùng cho D4 và nợ nhỏ không đụng mã ngôn ngữ.

**Nếu không kịp**: ưu tiên theo thứ tự **A1 → A2 → A3 → A4 → A5**. Không nhảy cóc để "trông như xong nhiều".
R30 **không** viết khi còn task pha 1 chưa merge. Số đo 07:30 vẫn lấy, ghi rõ HEAD và "pha 1 chưa đóng".

---

## 6. BÁO CÁO 09:00

Tệp `agent/BAO-CAO-A0-2026-11-19-0900.md`, tối đa khoảng 40 KB, gồm:

1. **Một dòng trạng thái**: pha 1 đã đóng chưa, HEAD cuối, cổng ps/mac.
2. **Bảng task**: mã task, làn, trạng thái (merge / trên nhánh / BLOCKED), commit, số test trước và sau.
3. **Số đo**:
   - #152: trung vị và khoảng tỉ lệ B/A, kết luận theo VD-217;
   - #147: bảng chia thành phần (khung 07:30), kết luận theo VD-197.
4. **Danh sách sinh lại**: golden và tệp kiểm thử tài liệu, mỗi lần một dòng kèm lý do.
5. **Quyết định Agent 1 đã làm thay A0** (theo §1), mỗi cái một dòng, để A0 xác nhận hoặc đảo lại.
6. **BLOCKED và câu hỏi cho A0**, đánh số Q1, Q2…, mỗi câu kèm phương án và đề xuất như các lần trước.
7. **Nợ mới** mở trong đêm.
8. Các phụ lục đã hẹn ở VD-219: diff SPEC §7, danh sách literal trộn khoá đã migrate, hiện trạng `{}`.

---

## 7. VIỆC AGENT 1 LÀM NGAY LÚC 19:00

1. Ghi VD-220 vào DECISIONS.md.
2. Tạo 4 worktree, 4 nhánh `dem/A` đến `dem/D`, thư mục target riêng.
3. Tạo 4 sub-agent Agent 2, mỗi sub-agent nhận **đúng §1, §3 và phần làn của mình** trong tệp này.
4. Ghi khung 07:30–07:50 vào QUEUE.md để mọi làn biết trước.
5. Tạo `agent/DEM-2026-11-18.md` để ghi mốc kiểm.

> **Ghi chú B1..B3**: B1 (đo A/B #152), B2 (VS0111 ngừng phát + xoá bộ lọc), B3 (ca `$value:`) đều **đã xong và đã merge** trước 19:00 (commits `8ed1885`, `d50ad39`) — chỉ chạy lại xác nhận; **B4** (script đo chuẩn #147) là việc thật.
> **Ghi chú A1**: ST-205 (R4a) **vẫn BLOCKED-CHO-A0** (BLOCKED.md §9) — VD-220 không trả lời 4 điểm §6.1; **A2/A3 lệ thuộc A1** nên cũng chờ; **A4 (ST-208/VD-215) đã merged** (`8ed1885`); **A5 (R5d, ST-209) — VD-187 vẫn chưa đủ rõ để tự làm**; §5 (không nhảy cóc) áp dụng.

### VD-221 · Uỷ quyền Agent 0 cho Agent 1 trong khung đêm 2026-11-18→19 (proxy A0)

**2026-11-19 (đêm)** · **Người ra lệnh**: người dùng (vận hành) · **Áp dụng**: phần còn lại của VD-220 (19:00 → 09:00 sáng 2026-11-19).

1. Người dùng uỷ quyền **Agent 1 thay mặt Agent 0** ("Agent 0 proxy") để quyết định các điểm đang BLOCKED trong đợt này — gồm **4 điểm §6.1 của R4a (ST-205)** và **ST-209 (R5d)**, cùng các lệnh phụ cần để đóng pha 1.
2. Mọi quyết định trong khung này mang nhãn **"A0-proxy (uỷ quyền 2026-11-18→19)"** và **có thể bị Agent 0 bác bỏ/sửa khi tiếp nhận**; không được ghi là A0 duyệt chính thức.
3. VD-196/§1 vẫn áp dụng cho mọi thứ **ngoài** phạm vi uỷ quyền này (không SPEC ngoại lệ nào ngoài các VD sẽ ghi dưới đây; không tự đổi ngưỡng fixture; không `#[ignore]`; SPEC-lex chỉ sửa khi VD mới có tên).
4. Thứ tự VD-219 giữ nguyên: (1) ST-205 → (2) đo A/B #152 (xong VD-220; không lặp) → (3) VS0111/VD = xong → (4) R4b/R4c → (5) ST-208/R5b′ (xong) → (6) R5d → (7) ST-210 → (8) ST-210.
5. **VD-185**: "4 dấu kiểm ngẫu nhiên của Agent 0" — Agent 1 ghi nhãn **"kiểm ngẫu nhiên proxy — chờ A0 ratify"**; không được xử lý như 4 dấu A0 thật.
6. **07:30–07:50** (khung đo #147) giữ đúng cam kết: dừng mọi làn; `test/vd220-147.sh`; log có chữ `máy rảnh`.
7. Kết thúc khung: **09:00** — báo cáo tổng hợp (BAO-CAO) ghi rõ danh sách quyết định proxy và trạng thái ratify.

### VD-222 · R4a chốt theo **uỷ quyền A0-proxy** (VD-221) — đêm 2026-11-18→19

**Ngày**: 2026-11-19 (đêm) · **Người ra quyết định**: Agent 1 **thay mặt** Agent 0 (VD-221 — uỷ quyền của người dùng) · **Có thể bị A0 bác bỏ/sửa khi tiếp nhận**.

1. **(Điểm 1)** `trait Error: Show` — **(ii)** **ba phương thức mặc định-less**: `message(self) -> str`, `source(self) -> Error?`, `span(self) -> str?`. **Amendment của chính VD-222 (điểm (i) marker loại bỏ) khi thực thi (18/11, về đêm)**: ngữ pháp ViScript **không có khối rỗng** (`implies Impl` với 0 dòng đều bị `VS0201`/`mong khối thụt lề`) — `impl Error for X:` rỗng là **không viết được**, nên marker sẽ không dùng được. Builtin traits `Show` (`fn show(self) -> str`) và `Error` (siêu `Show`; 3 phương thức trên) được biên dịch trước item người dùng; dispatch tĩnh (`x.m()` hạ thẳng `Call(impl_fn)`); `e.message` trên **bản ghi `Error` dựng sẵn** vẫn là **truy cập trường** (bản ghi có đủ 3 trường), còn `T: Error` từ người dùng đi qua `t.message()` như phương thức. **Loại (iii)**: không đưa field-member vào traits trong khung này.
2. **(Điểm 2)** `source: **Error?`** (đệ quy bản ghi thật) — trường lấy kiểu `Option[Error]` dựng sẵn; không đặt giới hạn kích thước tường minh ở tầng kiểu; giá trị `Value::Enum` là cây động nên không có vấn đề kích cỡ vô hạn; **ràng buộc**: không hủy vòng lặp `Error` trong trường khác `source` (chỉ `source` được đệ quy).
3. **(Điểm 3)** `span: **str?`** — định dạng `path:line:col` (khớp chính sách diagnostics hiện có; không cấp kiểu `Span` mới ở pha 1).
4. **(Điểm 4)** `Json` — **(A)** **EnumDef thật, đẩy vào mọi bảng kiểu trước item người dùng** (EnumId `Json` = **0** ổn định, RecId `Error` = **0** ổn định); biến thể `Null() | Bool(v: bool) | Num(v: {error}) | Str(v: str) | Arr(v: list[Json]) | Obj(v: map[str, Json])`; `Num` giữ cả `int` và `float` ở runtime với kiểu khai báo `{error}` (khớp wording VD-204 "Num payload {error}"). `json.decode` → **`Result[Json, Error]`**. `print` trên `Json`/`Error` dùng cú pháp ctor-ish: `Num(1)`, `Arr([Num(1)])`, `Obj({...})`; riêng **bản ghi `Error` in "thanh gọn" = chỉ `{message}`** (định nghĩa riêng ở `Value::Record` khi tên `Error`) — nhờ vậy các ca cũ `Err(e): print(e)` byte-identical với thời `str`-Err; `Span`+`source` vẫn `Show` đầy đủ khi in bằng `{Record}` thường.
   - **(VD-66 giữ hiệu lực)**: dữ liệu không kiểu từ json vẫn phải `match`; `xs` trong `Ok(xs)` có kiểu `Json` — không còn là `{error}` ⇒ **không** gây lỗi tĩnh hàng loạt; các test Rust cũ được migrate thủ công kèm ghi chú.
5. **ST-206 · R4b (thu gọn proxy)**: `AnyError` := **đồng nghĩa `{error}`** ở bảng kiểu (resolve `AnyError` → `Type::Error`); `?` giữ nguyên cơ chế hiện tại (không kiểm biến thể lỗi tại `?` — đã ghi nhận trade-off, A0 ratify sau); `main -> Result[_, AnyError]` compile sạch; `Err` ở `main` ⇒ **exit 1** (đã có, R8). **Không** dựng `dyn Error` machinery ở pha 1 này (không cần tồn kích).
6. **ST-207 · R4c**: mọi std fns chuyển **`Result[T, str]` → `Result[T, Error]`** (builtin record); interp dựng `Value::Record("Error", …)` thay `Value::Str`; docs/bảng std cập nhật; các ca `Err(e): print(e)` byte-identical nhờ (4) ở trên ⇒ **0 golden đổi** cho đường print; `test/` cheatsheet/fixtures đổi được rõ ràng khi in `Json`-enums.
7. **`while let`**: VD-178 hứa nhưng mã merge không có ⇒ **ST-226 mới** thêm `while let` (pha 1, sau R4) — cấu trúc giống `if let` nhưng vòng lặp; A0 ratify hình thức khi review (đánh dấu VD-222 (7)).
8. **Ghi chú ratify**: mọi mục trên là **proxy** — trong 09:00 báo cáo (§6 VD-220) liệt kê lại để A0 ký off; nếu A0 bác bỏ (1)–(4) thì ST-205/206/207 phải revert theo đúng nhánh.
9. **Thứ tự**: ST-205 → ST-206 → ST-207 → R5d (ST-209 dưới mục VD-222 (item R5d)) → ST-210 (R30 + đóng pha 1 nếu mọi cổng xanh và 4 dấu proxy ghi rõ).

> **Amendment (2026-11-19, Agent 2/1 — sau A0 ratify VD-222→điều kiện VD-231)**: VD-222 §5 (AnyError = `{error}`) **ĐÃ SUPERSEDED bởi VD-231** (AnyError = alias bản ghi `Error` RecId(0)). Tên `{error}` bị cấm dùng ở bề mặt người dùng; ở mã nội bộ vẫn ghi chú lịch sử (ST-205..).

### VD-223 · R5d chốt theo **uỷ quyền A0-proxy** (VD-221) — đêm 2026-11-18/19

**Người ra quyết định**: Agent 1 **thay mặt Agent 0** (VD-221) · có thể bị A0 bác bỏ/sửa khi tiếp nhận.

1. **(i)** `--b: bool` trong **hàm thường** ⇒ **lỗi `VS0209`** (Error): "tham số cờ `--b` chỉ hợp lệ trong chế độ lệnh (`cmd`); hàm thường dùng `b: bool`". Chỉ áp dụng cho `FnDecl` thường và `test fn`; `cmd` (và chế độ lệnh `.vish`) giữ nguyên luật `--cờ` theo thiết kế §9.4.
2. **(ii)** **Xung đột tên** (`use std.str` + `let str = …`…):
   - **Use-site ưu tiên**: trong vị trí **gọi thành viên** (`x.m(...)`), nếu `x` là biến đã khai trong phạm vi thì **ưu tiên dispatch trên giá trị** — tệp chỉ fallback sang `std.<module>` khi **không có** binding cho tên đó. (Trước VD-223: `str.len()` nuốt im lặng thành `std.str.len(...)` và báo `VS0301` sai nghĩa.)
   - **Khai báo**: `let`/`var`/tham số/pattern-binding có tên **trùng tên module std đã `use`** ⇒ **cảnh báo `VS0211`** (Warning, không chặn): "tên `str` là module std đã import — `str.xxx(...)` được hiểu là thành viên của giá trị, không phải module".
   - `str(…)` (tên trần) vẫn dẫn tới **builtin** `str(x)` như trước; `let str = "z"; str("a")` ⇒ `VS0412` "str không phải hàm" (giữ nguyên).
3. **(iii)** Mã mới: **`VS0209`** (Error) và **`VS0211`** (Warning) — nhóm 02xx; tổng `all_codes()` **52 → 54**; `API-CODES.txt` regen theo đúng giao thức ST-126 (VD-222/VD-223 đích danh). Không chạm SPEC-lex §7 (không phải từ vựng).
4. **Ghi chú ratify**: proxy; nếu A0 bác bỏ thì revert theo nhánh; `while let` (ST-226) **vẫn** là việc sau pha 1.

### VD-224 · Đảo ngược VD-223 (phần (i)) — uỷ quyền A0-proxy, sửa theo thiết kế đã chốt

**Ngày**: 2026-11-19 · **Cần A0 ratify** (nếu A0 bác bỏ thì revert theo nhánh).

1. **VD-223(i) (VS0209) bị huỷ toàn bộ**: theo thiết kế dòng ViScript đã chốt (**docs §5.1** "Hàm, closure, generic, trait", dòng `pub fn greet(name: str, times: int = 1, --loud: bool = false)` và gọi `greet("An", 2, --loud: true)`), **tham số có tên bắt đầu bằng `--` là hợp lệ ở mọi hàm** — chỉ truyền theo tên; trong chế độ lệnh chúng là cờ. `--b` trong `.vish`/`cmd` giữ vai trò cờ; `--b` trong `fn` thường không bị cấm.
2. **`VS0209` bị xoá khỏi sổ mã; `all_codes()` 54 ⇒ 53**; `API-CODES.txt` regenerate kèm VD-224 đích danh (ST-126).
3. **(ii) (VS0211 + ưu tiên dispatch biến cục bộ) giữ nguyên** — đó là phần R5d có giá trị (xung đột tên đúng nghĩa).
4. **`ST-209 (R5d)`**: phần "xung đột tên std.str/str/builtin" theo VD-224; phần "`--b` chỉ chế độ lệnh" theo thiết kế, **không áp lỗi**.

### VD-225 · Uỷ quyền kết thúc — Agent 1 quyết toàn bộ đợt VD-220/VD-221 (2026-11-18/19)

**Người ra lệnh**: người dùng (vận hành, 2026-11-19) · **Áp dụng**: đóng pha 1 + ST-211 + phần còn lại của khung VD-220.

1. Người dùng cho phép Agent 1 **quyết thay Agent 0 toàn bộ** các điểm còn treo trong khung này, gồm: ratify VD-221..224 (đã áp lên repo), ratify VD-186 (2 patch `DU-THAO-VD186`), đóng pha 1 (§5 VD-220), ST-211 (VS3-5-pre, VD-184) và ST-212 (VS3-QĐ-1, VD-185).
2. Mọi quyết định vẫn ghi nhãn **proxy (VD-225, A0-uỷ-quyền)**; A0 có thể đảo khi tiếp nhận. Không được ghi là A0 duyệt chính thức.
3. **Đóng pha 1 (ST-210)**: **ps-only** — mac không đo được trong khung này (ghi rõ waiver tại BAO-CAO §3.3 + ST-210 + ROADMAP). Điều kiện đóng VD-185 cho mac/A0: giữ nguyên nghĩa, chỉ là A0-proxy chấp nhận ps trước, A0 có thể mở lại khi có mac.
4. **VD-186**: áp hai patch (`HOST-BAO-DAM-4.patch` + `ViScript-thiet-ke.patch`).
5. **ST-211** (VS3-5-pre): làm sau khi đóng pha 1.

### VD-226 · `while let` — ghi bổ sung (backfill, proxy A0 — VD-225)

**Ngày**: 2026-11-18/19 · **Người ra lệnh**: người dùng (VD-225 uỷ quyền) · **Cần A0 ratify** (nếu A0 bác bỏ thì revert theo nhánh).

1. **`while let PAT = EXPR:`** (R18b) — cấu trúc vòng lặp kiểu `if let` với pattern; dùng chung `HCond::Let` với `if let` (parser/`if_cond` tái sử dụng không đổi).
2. **Semantics**: pattern không khớp ⇒ thoát vòng, **không** panic; khớp ⇒ bind rồi chạy thân, lặp lại.
3. `HStmt::While { cond: HCond }` (thay `cond: HExpr`); interp và VM **byte-identical**; `vd_226.rs` 8/8 ca khoá hai đường xanh.
4. **Không** đổi `SPEC-lex.md` trong khung này (§SPEC chỉ sửa qua A0 chính thức) — task **không** chạm EBNF; ghi chú chờ A0 cập nhật SPEC khi ratify.

### VD-227 · Interp tự nâng `T → T?` tại `let` annotate — ghi bổ sung (backfill, proxy A0 — VD-225)

**Ngày**: 2026-11-19 · **Cần A0 ratify** (nếu A0 bác bỏ thì revert theo nhánh).

1. Interp **tự nâng** `T → T?` tại `let x: int? = expr` (annot Optional) — mirror codegen `Instr::MakeSome` (đã có từ VD-178 §4.4).
2. **Chỉ `let` annotate** được nâng; **function-return** và **đối số** không nâng (scope hẹp, có chủ đích; xem VD-228).
3. `vd_227.rs` **6/6** ca byte-identical hai engine; không đổi fixture nào; lệch cũ `Some(42)` vs `42` đóng.

### VD-228 · Function-return `-> T?` lift `T → T?` — chốt proxy A0: Option B (impl) — cần A0 ratify

**Ngày**: 2026-11-19 (đêm) · **Người quyết**: Agent 1 (uỷ quyền `VD-225`) · **Cần A0 ratify** (nếu A0 bác bỏ thì revert theo nhánh; không ghi “A0 đã duyệt”).

1. **Option B được áp** (thay Option A “chờ A0” ghi ở BLOCKED §11 / BAO-CAO-A0 §9-Q10 lúc 01:00). Lý do: corpus chỉ có **1 ca** `-> T?` (`test/sem/optional/tra-bien.vis`), thực thi xong **an toàn, byte-identical, không đổi fixture**; §4.4 (nâng thuộc **trình biên dịch**) đúng cả interp lẫn VM.
2. **Scope** (thực tế, hai engine): mọi `HFn` (cấp tệp, generic, **impl method**) — interp nâng ở biên `call_hfn_trong` (`opt_lift_fn_ret`); VM nâng bằng `ret_opt` trên `Emit` + `Instr::MakeSome` (đã **phẳng Option** nên không bọc hai lớp) tại (a) biên thân hàm `HBody::Expr`/`HBody::Stmts` (`codegen/than.rs::lift_ret_opt`), (b) `return` sớm (`HStmt::Return`). **Closure KHÔNG nâng** (HIR chưa giữ `fn ret` cho closure; cần field mới — task kế tiếp khi A0 yêu cầu). **Đối số truyền vào tham số `int?`/`str?`** — **chưa nâng** (Option-B phần argument vẫn còn mở nếu A0 muốn).
3. **Bằng chứng**: `vd_228.rs` **5/5** byte-identical (plain int/str lift, `None` giữ nguyên, branch join hai nhánh, early return); `vis test` **469/469** ×2; `cargo test --workspace` **147 binary xanh / 0 đỏ**; `vis-dev gate --fast` **PASS 10/10**; R1 giữ (than.rs tách riêng, `codegen.rs` 1492 dòng).
4. **Nếu A0 đảo Option A**: revert commit `1eea7ed` + xoá `codegen/than.rs`, `vd_228.rs`, khôi phục lift bản cũ; spec/cheatsheet chỉnh theo hướng đó. **Option B revert được toàn bộ** không đụng fixture.
**Bổ sung 2026-11-19 (~01:40–01:55, VD-225 amendment)**: từ `8726eee`, `vd_228.rs` mở rộng **7/7** — thêm ca (6) impl-method `-> T?` lift (`impl P: fn pick(self) -> int?: self.x`) và (7) không bọc hai lớp (`fn f() -> int?: Some(42)` ⇒ `Some(42)`). Không đổi kiến trúc lift; chỉ thêm ca khoá.

### VD-229 · Argument-biên lift `T → T?` + closure-biên lift — chốt proxy A0 (VD-225 tiep tuc), 2026-11-19

**Người quyết**: Agent 1 (uỷ quyền A0, tiếp VD-228) · **Cần A0 ratify** (nếu bác bỏ thì revert theo nhánh).

1. **Argument-lift áp dụng** cho tham số annot `Opt(_)` của **mọi HFn** (cấp tệp, generic, impl method, cmd):
   interp `call_hfn_trong` (sau resolve positional/named/default) gói plain→Some; VM phát `LoadLocal/MakeSome/StoreLocal`
   cho mọi tham số annot `Opt` (`codegen/than.rs::lift_params_opt`) **sau** bước default-fill và trước thân hàm.
2. **Closure-biên lift** (`fn p -> int?:` / `fn(x) -> T?:`, bao gồm body `HBody::Expr`): `Closure.ret_opt`/`Cho.ret_opt`
   từ HIR `Lambda.ret`; interp `call_closure_trong` gói; VM `Cho.ret_opt` ⇒ sang closure-drain Emit.
3. **Closure *params* không nâng** — đo thật: lambda params `(x: int?) => x` **không** parse được
   (`VS0201 mong `)`, thật `:`) ⇒ không annot Opt trên closure params nên không có tham số để nâng; **dead code** (không thêm).
4. **Byte-identical** interp ↔ VM; corpus **469/469 ×2**; `vd_228.rs` **13/13** (param plain/None/Some/default
   `(..., --q:"mặc định")` lift; closure-ret lift; cả hai engine byte-identical); workspace **147 binary xanh / 0 đỏ**.
5. **Nếu A0 đảo**: revert từng commit `004b145` (VD-229), sau đó `1eea7ed` (VD-228) về Option A (giữ `let` lift VD-227).

### VD-241 · Uỷ quyền chạy tự động (thay VD-221/VD-225) — Áp dụng từ 2026-11-19

**Ngày**: 2026-11-19 · **Người ra quyết định**: Agent 0 (ratify pha 1 + bàn giao) · **Hiệu lực**: tới khi A0 ra tệp quyết định mới.

#### 1. Ratify

- **PHA 1 ĐÓNG CHÍNH THỨC** tại `pha1-rc1` (`05e42d01483d255987750656fa6cba6c434831c9`), ratify bởi A0.
- Ratify **toàn bộ VD-215..VD-240**, theo đúng nội dung đã chốt trong các tệp quyết định A0 (VD-222 theo dạng sửa của VD-231; VD-229 bỏ phần closure-params; VD-225 chỉ trong phạm vi VD-228/229). Chỗ nào bản ghi STATE/JOURNAL khác các tệp quyết định thì theo tệp quyết định.
- Bug-fix `field_of` @ `1c62858` và VD-238 được ratify.
- **Đính chính**: VD-225 đã hết hiệu lực trước khi đóng pha 1. Sửa dòng "Theo uỷ quyền VD-225" trong STATE/BAO-CAO thành "đóng theo VD-237, A0 ratify ngày 2026-11-19". Mọi VD từ VD-241 trở đi ghi nhãn `[VD-241]`, không ghi "proxy A0".

#### 2. Ba việc dọn trước (P0, tổng 1 h, làm đầu tiên)

- **D1**: Viết rõ trong báo cáo ca `anyerror_none_field_engine_parity` hiện làm gì sau VD-238. Nếu nguồn `let x: AnyError = None` giờ đỏ lúc check thì tách làm hai ca: (a) ca check: `VS0412`; (b) ca runtime parity: tạo giá trị sai bằng con đường check không chặn được (nếu không còn con đường nào thì xoá ca (b) và ghi lý do).
- **D2**: Thử đột biến VD-238: tắt rule ở từng chỗ trong ba chỗ (`coerce_e`, `kiem_than`, Return-arm), ghi ca nào chuyển đỏ. Chỗ nào tắt mà không ca nào đỏ thì thêm ca.
- **D3**: Chép tóm tắt 4 dòng kết quả mac + thông tin máy vào `agent/SO-NO.md` (hoặc `agent/PHA1-DONG.md`). Không trỏ tới `/tmp`.

#### 3. Uỷ quyền chạy tự động (thay VD-221/VD-225)

**Được tự làm và merge main**:
- Sửa lỗi có ca repro đỏ trước, xanh sau, không đổi ngữ nghĩa đã ghi trong spec.
- Thêm test, fuzz, ca khoá; sửa tài liệu cho khớp với mã hiện có.
- Công cụ dev (vis-dev, script, CI), refactor nội bộ không đổi output.
- Regen golden **CHỈ** khi thay đổi nằm trong allowlist và mỗi ca đổi được liệt kê.
- Đo hiệu năng, ghi số vào SO-NO.

**KHÔNG được làm** (ghi `BLOCKED-CHO-A0`, làm trên nhánh `nhap/<task>`, không merge):
- Đổi ngữ nghĩa ngôn ngữ, cú pháp, quy tắc kiểu, nghĩa của mã lỗi.
- Đổi API-STD.txt / API-CODES.txt đã freeze, thêm/xoá mã VS.
- Đổi `Value`, mô hình async/kênh, ABI/định dạng bytecode.
- Tách repo, đổi tên gói/domain, phát hành bản ra ngoài.
- Tự mở rộng quyền này, hoặc đọc "im lặng của A0" là đồng ý.

**Khi gặp ca không rõ thuộc nhóm nào: coi như KHÔNG được làm.**

#### 4. Hàng việc (theo thứ tự)

| # | Việc | Loại | Ngân sách | Điều kiện ĐẠT |
|---|------|------|-----------|---------------|
| 1 | D1, D2, D3 (mục 2) | dọn | 1 h | như mục 2 |
| 2 | **VD-240 (b)**: test grid một luồng cho async/kênh | test | 1 ngày | grid chạy cả interp + VM, byte-identical; 0 golden đổi ngoài allowlist; cheatsheet có ghi chú "async chạy một luồng" |
| 3 | **#152**: đo chuẩn trên máy rảnh (load < 0,3), ps và mac cùng HEAD | đo | 2 h | ghi B/A + khoảng; chỉ đóng nếu khoảng nằm hẳn một phía 1,10; **KHÔNG** tối ưu ctor-enum |
| 4 | **#147-mac**: đo cùng script trên mac | đo | 1 h | số + HEAD vào SO-NO; đóng #147 nếu không vượt 3× ps, nếu vượt thì ghi nợ, không tự tối ưu |
| 5 | Fuzz/nightly hằng ngày | test | tự động | ca crash nào cũng thành ca repro + bug-fix theo mục 3 |
| 6 | Nháp ROADMAP pha 2: liệt kê ứng viên (#151 VS3, #149, #155 lift params closure, lambda annotation, `T??`, async (a)) | tài liệu | 3 h | chỉ ở `agent/ROADMAP-PHA2-NHAP.md`; mỗi mục ghi lợi ích, rủi ro, số golden dự kiến đổi; A0 chọn |
| 7 | Nợ kỹ thuật không đổi ngữ nghĩa có sẵn trong SO-NO | sửa lỗi | còn lại | theo mục 3 |

Hết việc 1..7 thì **DỪNG**, không tự tạo hướng mới.

#### 5. Luật vận hành

- Agent 1: leader, tự soạn tài liệu, ghi sổ. Tối đa 4 sub-agent Agent 2, mỗi cái một worktree; chỉ leader merge.
- Mọi bug-fix và thay đổi checker phải có review chéo bởi sub-agent khác tác giả, có thử đột biến.
- Sau mỗi merge: `gate --fast` PASS 10/10 + sem 469/469 ×2 + workspace xanh. Đỏ thì revert ngay, không sửa chồng lên.
- Mỗi ngày một lần: clone sạch gate trên ps, ghi HEAD.
- `pha1-rc1` không bao giờ bị di chuyển hoặc xoá.

#### 6. Dừng hẳn và chờ A0 khi

- Gate đỏ trên main mà revert không cứu được.
- Hai engine lệch byte mà không xác định được bên nào đúng theo spec.
- Một bug-fix buộc phải đổi ngữ nghĩa hoặc API freeze.
- Hết hàng việc mục 4.

Khi dừng: ghi lý do, HEAD, bước tái hiện vào `agent/DUNG-CHO-A0.md`, không làm tiếp nhánh liên quan.

#### 7. Báo cáo cho A0 khi quay lại

Một tệp `agent/BAO-CAO-A0-<ngày>.md`, mỗi phần ngắn:
1. Một dòng trạng thái: HEAD, gate, sem ×2, số binary, clone sạch tại HEAD nào.
2. Bảng task: mã, trạng thái, commit, test trước → sau.
3. Danh sách BLOCKED-CHO-A0 + nhánh nháp, mỗi mục một câu hỏi có đề xuất.
4. Số đo (#147-mac, #152) kèm HEAD và load máy.
5. Golden đổi (nếu có) + lý do.
6. Nợ mới.

Mỗi số chỉ ghi một lần, kèm HEAD; không lặp số ở nhiều chỗ.

### VD-241 (đính chính Agent 1, 2026-11-19) · §5 dòng "sem 469/469 ×2"

Nội dung VD-241 giữ **nguyên văn** (Agent 0, 2026-11-19). Chỉ đính chính **số** lỗi thời sinh bởi merge VD-240: **469/469 → 473/473** (473 ca, hai engine). Có hiệu lực từ `6295c8e` (VD-240 grid, +4 ca). Không đổi luật, không đổi phạm vi.

### Quyết định Agent 0 · 2026-11-19 (đợt đánh giá công thái học) — ghi theo thứ tự nhận, **chưa gán số VD** (A0 không cấp số trong lượt trả lời này; KHÔNG tự chọn)

**Người ra quyết định**: Agent 0, 2026-11-19 (trả lời đánh giá công thái học + chốt phạm vi) · **Hiệu lực**: thay cho phần "ĐỪNG làm các nhánh" ở `agent/DUNG-CHO-A0.md` — phạm vi mở lại **theo đúng** quyết định dưới đây. `pha1-rc1` (`05e42d0…`) **bảo toàn**; các lỗi mới chỉ được **ghi nhận**, không sửa lịch sử.

1. **Nền giữ**: let/var, bool-only `if`, Option/Result tách, match exhaustive, named args + default, record-vs-map bằng khoá, `if let`/`while let`/`?`/`??`. **Không** được lùi vào bất kỳ phần nào trong số này.
2. **Corpus gap là bằng chứng corpus chưa đủ**: mọi ca hồi quy mới phải có (i) kỳ vọng độc lập suy từ quy tắc đã chốt; (ii) differential hai engine (stdout + stderr phù hợp + exit code + dữ liệu đầu ra). **Không** lấy interp làm chuẩn tự động; **không** sinh lại golden chỉ để xanh.
3. **Option lồng (chốt ngữ nghĩa, KHÔNG phải "chỉ sửa bug VM")**:
   - `T?` = `Option[T]`. `T??` = `Option[Option[T]]`, **không làm phẳng**.
   - `Some(x)` luôn thêm **đúng một** lớp.
   - Pattern khớp **theo cấu trúc giá trị thật**, không tự bọc/bóc trong pattern; `Some(Some(v))` **không** khớp `Some(42)`.
   - Auto-lift chỉ ở chuyển đổi **gán/annot/đối số/return** (biên VD-227/228/229), tối đa **một** lớp mỗi lần; **không** lặp nâng trong một chuyển đổi; **không** nâng ngầm trong pattern.
   - Ví dụ định hướng (đúng theo nguyên văn A0):
     `let a: int? = 42` ✔ (nâng 1 lớp) · `let b: int?? = Some(Some(42))` ✔ (đúng kiểu) · `let c: int?? = Some(42)` ✔ (thêm một lớp ngoài) · `let d: int?? = 42` ✘ (cần nâng hai lớp) · `let e: int?? = None` ✘ (thiếu lớp ngoài) · `let f: int?? = Some(None)` ✘ ("có lớp ngoài, thiếu lớp trong").
   - `agent/QD-OPTION-LONG-2026-11-19.md` là **bảng quy tắc pin** (đọc trước khi triển khai T1).
4. **Pattern ctor lồng (VM) + JSON encode**: được phép sửa. Pattern phải khớp đệ quy theo cấu trúc; giá trị JSON do code dựng và do decode **encode cùng ý nghĩa**; **không** mặc định hai lỗi cùng nguyên nhân — kiểm chứng riêng.
5. **Escape `\u{…}` (SPEC §8 dòng ~57)**: sửa riêng; kiểm tra Unicode scalar hợp lệ + tương tác interpolation + escape không hợp lệ; **không chỉ thêm một ca "A"**.
6. **`vis check`**: sửa tài liệu ngay (README ghi rõ "check xanh ≠ không lỗi runtime") **và mở rộng checker có chọn lọc**: cộng những lỗi **checker đã đủ thông tin để kết luận** phải thành lỗi tĩnh —
   - tên không tồn tại trong phạm vi tĩnh;
   - constructor record thiếu field bắt buộc, hoặc field trùng/không tồn tại;
   - destructure một giá trị đã biết không phải tuple;
   - phép toán sai kiểu trong closure khi kiểu đã xác định;
   - `?` ngoài ngữ cảnh được phép.
   Những cái phụ thuộc dữ liệu runtime (map key, index, deadlock kênh, capacity 0) **giữ runtime**, không ép qua checker.
   **Giữ chính sách strict-caps hiện tại** trong vòng này.
7. **`main -> Err` ⇒ exit 1**; Ok ⇒ exit 0. **Không** suy rộng sang `main -> int` khi chưa có quyết định riêng.
8. **Toolchain**: được phép sửa khả năng tái lập; **không hardcode** ưu tiên `~/.cargo/bin` trong mọi trường hợp; tôn trọng cấu hình chủ động của người dùng; **báo rõ compiler được chọn và yêu cầu phiên bản**; xác định phiên bản tối thiểu **thật** và khai báo nhất quán — **không** dùng con số gần đúng (báo cáo ghi rõ `vis-host-linux` yêu cầu `1.99.1`, không được chốt `"1.99"` cho qua).
9. **Đồng bộ tài liệu**: cho phép, nhưng tách "mô tả hiện trạng" khỏi "quy tắc đích"; **không** sửa spec thành hành vi sai chỉ để khớp code.
10. **Show closure**: chốt `<fn>` cho **cả hai** engine (chuỗi giá trị ổn định).
11. **Channel**: giữ API một giá trị `<kênh>` trong vòng này; tài liệu §742 tuple lỗi thời được sửa; **không** gọi tuple là alias khi chưa có implementation; rendezvous = **khả năng chưa triển khai**, không phải đã hỗ trợ.
12. **Thứ tự pha 2**: tính đúng → kiểu & tính nhất quán → script/process → thư viện thiết yếu → công cụ. **Lambda annotation trước closure lift**. Không mở async runtime lớn lúc này.
13. **Unit và never — sửa trước khi thêm cú pháp**: `()` phải có một ý nghĩa kiểu thống nhất; nhánh kết thúc bằng `return`/`break`/`continue` xử lý như luồng không tiếp tục (never), không ép thành giá trị bình thường; phân biệt biểu thức-cần-giá-trị vs câu-lệnh; **chưa** đổi kiểu trả về `push` trong cùng đợt.
14. **Record — tên trường quan trọng hơn thứ tự**: khởi tạo record bằng tên trường, không phụ thuộc thứ tự; field trùng/lọt/thiếu bắt buộc ⇒ lỗi; default: quy tắc áp dụng rõ ràng (phải bao gồm cả thời điểm và thứ tự đánh giá); **không** parse default rồi bỏ qua như đã hỗ trợ.
15. **Trait & generic ghép module**: ưu tiên sửa truyền thông tin impl/type qua module và biến vòng lặp; **bound chưa hỗ trợ phải enforce hoặc từ chối rõ ràng**, không im lặng chấp nhận.
16. **Script/process API (bài 6 bị chặn)**: xây API process dùng được từ `.vis` — nhận chương trình và danh sách đối số riêng biệt, trả kết quả có stdout/stderr/exit code; lỗi khởi chạy **phân biệt** với tiến trình chạy xong nhưng mã ≠ 0; `.vish` dùng chung nền thực thi, không phát triển nhánh rời. **Báo cáo lại trước khi mở rộng process API hoặc đổi bề mặt thư viện chuẩn.**
17. **Ngôn ngữ & workaround**: không phải "thêm cú pháp" — xử lý: parse số có báo lỗi/overflow; stderr và lỗi CLI bình thường; tìm chuỗi; thao tác chuỗi với quy ước Unicode rõ; JSON constructor/accessor đầy đủ. CSV quoting đúng chỉ sau khi §16 được duyệt.
18. **Trình tự thực thi**: (A) nhóm sửa tính đúng — Option semantics (T1) → pattern VM (T2) → JSON VM (T3) → Unicode escape (T4) → main exit (T5) → toolchain (T6), tích hợp có điều phối nếu cùng chạm codegen; (B) sau nhóm tính đúng: unit/never → checker → record → module/type propagation; (C) **báo cáo lại trước khi** mở rộng process API / đổi bề mặt stdlib.
19. **#152-vế-mac / #147-mac: giữ chờ bằng chứng mac**; không đóng bằng số đo Linux.

**Rủi ro đã ghi nhận (không làm mờ)**: "corpus 473/473 xanh" **không** mâu thuẫn với "chương trình JSON thật chạy sai" — corpus chưa phủ ca lồng. Mọi golden đổi theo ca mới phải **liệt kê từng nhóm**; không regenerate hàng loạt.


### Quyết định Agent 0 · 2026-11-19 (Windows @ `349af67`) — ghi theo thứ tự nhận, **chưa gán số VD** (A0 không cấp số trong lượt trả lời này; KHÔNG tự chọn)

**Người ra quyết định**: Agent 0 (câu trả lời "Quyết định A0 về Windows tại 349af67") · **Base**: `349af67` · **Đối tượng**: Windows/psvn verification + diễn giải kết quả.

1. **Phương án C** — dừng đợt Windows tại `349af67`; **không** port ~101 ca L0 trong đợt đóng pha 1. Windows được xếp **Tier 2 — product verified**: có máy Windows thật để kiểm chứng, có tập gate sản phẩm xác định, lỗi Windows là lỗi cần xử lý; nhưng `cargo test --workspace` **chưa** phải cổng bắt buộc trên Windows. **Không dùng** cụm "Windows fully supported" / "full Windows gate PASS".
2. **`pha1-rc1` (`05e42d0…`) giữ nguyên tag**; các commit `ba11a90` + `349af67` được giữ trong `main`.
3. **Diễn đạt kết luận Windows** thay thế cụm tuyệt đối "không có hồi quy sản phẩm" bằng: *"Không phát hiện hồi quy mã sản phẩm trong các lớp đã được kiểm chứng thành công; 101 ca L0 còn đỏ đã được phân loại sơ bộ là vấn đề portability của test suite hoặc khác biệt nền tảng, nhưng chưa được port và xác nhận độc lập từng ca."*
4. **VD riêng "Windows test-suite portability"** chỉ được mở **sau khi pha 1 đóng chính thức**; base khởi điểm `349af67`; **không gộp** với Phase B-3 / Option-long / lambda-annotation / #147 / #152. Chính sách: (A1) test process đa nền tảng phải dùng **fixture executable do workspace build**, không ánh xạ tùy tiện `/bin/echo → cmd /C echo` v.v.; (A2) test Unix-integration (sh, quoting, executable bit, signal, tiện ích POSIX) mới được `#[cfg(unix)]`, phải ghi rõ là Unix integration, **không tính coverage đa nền tảng**; (A3) test kiểm tra thiếu chương trình dùng fixture executable không tồn tại, không `/bin/...`; **cổng coverage**: mọi API process đa nền tảng phải có ca thành công trên Windows (stdout/stderr/exit code≠0), capability grant/deny phải có coverage Windows, không giảm tổng số test process chỉ để workspace xanh. (B) Path/JSON/CRLF: normalize **theo hợp đồng từng tầng** — (i) fixture dùng path literal chính thức hoặc helper escape dùng chung duy nhất; (ii) JSON assert parse thành cấu trúc thay so byte hiển thị; (iii) CRLF chỉ normalize khi hợp đồng là dòng logic (console/diagnostic), không normalize byte-accuracy tests (span/lexer/serializer/round-trip); helper normalization phải có tên mục đích (vd. `normalize_console_newlines()`), không helper mơ hồ. (D) WinSock: chấp nhận lỗi semantic tương đương (refused/reset/timeout) + timeout hữu hạn; test server do suite tự dựng phải dùng listener port động, đồng bộ server-ready, shutdown có timeout; các ca `server_*` cần xem xét riêng xem có phải race fixture chứ không phải WinSock semantics. (E) Unix mode `#[cfg(unix)]` cho test bit x; **contract của `long` trên Windows trình A0** (ẩn cột mode / placeholder tài liệu hoá / thuộc tính Windows phù hợp); chưa đổi output sản phẩm.
5. **CI**: **không** đưa workspace L0 đỏ thành required CI; tối đa informational/nightly (không `continue-on-error` trình bày như pass; lưu danh sách suite/test đỏ để phát hiện tăng; cảnh báo khi bước sản phẩm đã xanh bị đỏ lại). **Sau VD portability** mới xem xét Tier 1 + required CI (hoặc Windows full gate theo nightly + smoke bắt buộc trên PR). Không yêu cầu Unix-only tests xuất hiện trong Windows count.
6. **#147-mac / #152-vế-mac**: tiếp tục chờ máy mps; **không** dùng Windows/Linux thay bằng chứng mac; VD portability **không được** chặn lịch đo mac. **Phase B-3**: chờ ratify riêng. **Lambda annotation / #155**: quy trình riêng.
7. **Q-OPTION-1/2**: [ĐÍNH CHÍNH 2026-11-19 — lệnh A0 tiếp theo cùng ngày]: cách diễn giải trước của Agent 1 ("thiếu lớp ngoài/trong" ⇒ SAI KIỂU) là **hiểu nhầm chú thích A0**. A0 làm rõ: `let e: int?? = None` **hợp lệ** khi kiểu đích đã xác định ("Option ngoài là None"); `let f: int?? = Some(None)` **hợp lệ** ("Option ngoài là Some, Option trong là None"); `Some(Some(42))` cả hai Some — ba trạng thái giá trị phân biệt được, đều hợp lệ dưới quyết định "lồng thật, không flatten". **Không** VS0412-cho-f, **không** đổi `long-f.vis` thành `.err`. **Nếu engine thực sự từ chối `let e: int?? = None`** (hiện trạng: `VS0412 2:20`, VD-238) ⇒ **finding cần tái lập** — Agent 2 đang kiểm tra tối thiểu ba ca bằng checker/interp/VM; **chưa sửa engine hoặc golden** trước khi xác định chính xác chỗ lệch. Xem đính chính ở `agent/QD-OPTION-LONG-2026-11-19.md` §4 và block đính chính bên dưới.
8. **Quyền truy cập psvn**: ghi rõ trong `agent/PSVN-ACCESS-2026-11-19.md` — chủ sở hữu key, fingerprint, máy được phép, vị trí dòng public key, cách thu hồi (xóa **đúng dòng** `ps-host-to-psvn`, không khôi phục toàn bộ backup một cách mù), ngày xem xét lại quyền truy cập. **Không** commit private key, SSH log nhạy cảm, nội dung đầy đủ `administrators_authorized_keys`.
9. **Không** thay đổi ngữ nghĩa ngôn ngữ / quy tắc kiểu / nghĩa mã lỗi / ABI / API-STD; **không** tạo lại golden; ghi nhận, không rewrite lịch sử.

**Rủi ro đã ghi nhận**: `cargo test --workspace` trên Windows **chưa** xanh (101 ca đỏ / 25 suite — lớp portability/VS3); mọi tuyên bố công khai phải giữ "Tier 2 — product verified", không viết "full Windows gate PASS".


### Đính chính Agent 1 · 2026-11-19 (Option f/e — đính chính cách hiểu A0)

**Người ra lệnh**: Agent 0 (câu trả lời "Quyết định A0 về Windows tại 349af67" — mục (ii) + đính chính ca e/f).

- A0 xác nhận Windows Tier 2 (phương án C) **không đổi**; đợt Windows được đóng ở phạm vi đã báo cáo.
- **Option**: chú thích "thiếu lớp ngoài/trong" **mô tả trạng thái giá trị vắng mặt hợp lệ**, không phải sai kiểu. A0 diễn giải lại nguyên văn (đã ghi trong `agent/QD-OPTION-LONG-2026-11-19.md`):
  `let e: int?? = None` ✔ (Option ngoài là None) · `let f: int?? = Some(None)` ✔ (Option ngoài là Some, Option trong là None) · `let z: int?? = Some(Some(42))` ✔ (cả hai Some).
- **Không** duyệt đề xuất VS0412-cho-f. **Không** đổi `long-f.vis` thành `.err`. `int-qhap-gan.vis` rà theo quy tắc cấu trúc.
- **Agent 2 probe 2026-11-19 ~15:15 (hoàn tất)**: `let e: int?? = None` — check rc=1 `VS0412 2:20` (VD-238) · interp rc=2 · VM rc=2 (cả ba cửa; chỉ ở khai báo `let`; VM/interp nhất quán; không crash). **Finding ca e tái lập** — chưa sửa engine/golden; chờ xác định chính xác chỗ lệch; đề nghị đợt Option riêng. (f `Some(None)` rc=0 cả ba cửa; z `Some(Some(42))` rc=0 cả ba cửa; VM/interp nhất quán toàn bộ.)
- **Sửa hồ sơ sai** (Agent 1 đã gán sai quyết định cho A0): QD-OPTION-LONG §3/§4/§5, DEM-2026-11-18 (~14:35), báo cáo Windows §7, và bullet 7 ở block A0 (Windows Tier 2) phía trên đều đã được đính chính theo nguyên văn này. Không còn bất kỳ tệp nào ghi "f=error theo A0".


### Kết thúc đợt Option (lệnh A0 2026-11-19 — duyệt mở ngay, không chờ pha 1) · `3b2c67e`

**Người thực thi**: Agent 2 (worktree `option-none-fix-2026-11-19`) / **review chéo**: Agent 2 khác tác giả — verdict **SHIP** · **Lead merge**: Agent 1 vào `main` `3b2c67e` (ff-only).

- **Fix `la_none_chep_duoc_opt_1` → `la_none_dich_opt`** (`crates/vis-types/src/tyck/vd238.rs`): `None` trần hợp lệ khi **lớp ngoài cùng** của kiểu đích là `Option` (`T?`/`T??`/`T???`); `Never`/`Error` giữ nguyên; mọi annot ngoài cùng non-Option (kể cả chứa Option bên trong như `list[int?]`/`map[str,int?]`/`Result[..,int?]`) vẫn **VS0412**; alias phân giải về `Type::Opt` trước gate; tham số kiểu mở vẫn `TypeRef` ⇒ VS0412.
- 3 call-site (`coerce_e`, `kiem_than`, Return-arm) nguyên khối một điểm sai; không đụng auto-lift/lift_opt/pattern/flatten/MakeSome/ABI/API-STD/SPEC.
- **Corpus**: `long-reject-e` (.err) → `long-none-e.vis` xanh (`sn`); thêm `none-three-states` (42/sn/none-out), `long-none-param` (1000), `long-none-return` (sn); +3 ca âm `list/map/Result[int?]`; `vd_205::none_dich_ngoai_cung_option_la_ok` (5 ca xanh mới, hai engine byte-identical). **486 ca** (**436 `.vis` + 50 `.vish`**), **`.err` 89→88**; pins cập nhật mọi chỗ kèm lý do.
- **Cổng**: workspace 1297/0 · sem 486/486 ×2 · gate PASS 11/11 (fresh clone `3b2c67e` PASS).
- **Mutation**: revert ⇒ 4 ca khoá đỏ + 9 test pin đỏ; nới-quá ⇒ 3 ca âm mới đỏ.
- **Open questions (theo dõi ở `agent/BAO-CAO-A0-2026-11-19-OPTION-FIX.md` §6, chờ A0)**: (1) 486 chốt; (2) `let x: T?? = None` runtime == `Some(None)` (lift ở biên annot giữ nguyên theo lệnh; `x = None` giữ `None`) — giữ hay VD riêng; (3) bất đối xứng sugar vs spelled-out/alias ở biên `let` (pre-existing, hai engine khớp) — VD riêng hay ghi chú; (4) docs `ViScript-thiet-ke.md` 4 chỗ "T?? là lỗi kiểu" chưa đụng (SPEC A0); (5) Windows/macOS chưa re-verify.

### Đợt coercion Option (lệnh A0 2026-11-19 — tiếp đính chính cùng ngày) · nhánh `rev/vd241-option-coercion-2026-11-19` (chưa push/merge)

**Người thực hiện**: Agent 2 (worktree `option-coercion-fix-2026-11-19`) · HEAD `4cdbad9` (base) · **chưa gán số VD** (A0 không cấp; KHÔNG tự chọn).
**Review chéo**: bắt buộc trước merge — chờ cấp; Agent 2 chỉ chuẩn bị.

- **ĐÍNH CHÍNH (thay thế open question (2)/(3) ở block trước)**: `let x: T?? = None` lúc chạy giờ là **`None` lớp ngoài** (không `Some(None)`) — runtime khớp bảng giá trị A0 ở **mọi biên**: `let` · khai `var` · gán lại (`var` có annot Optional — nay nâng một lớp) · đối số · thân hàm · `return`. `Some(42)` ⇒ **nâng một lớp** ⇒ `Some(Some(42))`; `42` ⇒ lỗi `VS0412` (kể cả spelling-out/alias — `vd_205::vd241_forty_two_opt2_la_loi`); độ sâu ba lớp kiểm riêng.
- **Bất đối xứng spell — SỬA TẠI LOWERING** (`crates/vis-types/src/lower.rs::named_type`): spelled-out `Option[T]`/`option[T]` (1 đối số) phân giải về `Type::Opt` **ngay ở lowering** (như `list`/`map`/`Result`) — trước đây giữ `TypeRef`, khiến biên lift bỏ qua (`None` không nâng — nhưng `Some(42)` cũng không nâng ⇒ sai độ sâu) và bị phân biệt với sugar. Alias `type Y = …` đi cùng đường (HIR store `Type::Opt`). Ba spell giờ **tương đương** cả checker lẫn runtime.
- **Bare None ở biên lift — SỬA TẠI `crates/vis-interp/src/lift.rs::lift_opt`**: `None` trần (lớp ngoài vacant) **trực tiếp hợp lệ** với mọi đích rooted-`Option` ⇒ **không bọc**; `Some(x)`/lõi plain nâng như cũ (đúng một lớp khi checker hợp lệ). `$lift-opt` dùng chung interp+VM nên hai engine khớp.
- **Khai `var` có annot Optional — nâng một lớp** (`HStmt::Var`, cả hai engine) như `let` (trước không nâng ⇒ lưu giá trị thiếu lớp, checker-inconsistent).
- **Gán lại `var` có annot Optional — nâng một lớp** (cả hai engine; đích là `Var` gốc với annot `Option` từ HIR/symbol). Gán lại vào `let` vẫn là lỗi tyck (bất biến). **Chưa** nâng ở đích trường/chỉ số và `var` không annot — xem BAO-CAO addendum §8.6/§10.
- **VM `Instr::Unwrap(i)` không trap trên arm không khớp** (`crates/vis-vm/src/vm.rs`): đẩy `Unit` (test con cho `false`); arm test trở thành **tổng** như interp; bind/`let else` không đổi (chỉ sau `TestCtor(true)`). Trước đây arm ctor `Some(…)` đầu gặp giá trị `None` ⇒ `VS0501` (exit 101) lệch interp.
- **Corpus 487** (+`optional/long-option-matrix.vis`); expected output đổi **4 ca** (`long-none-e` sn→none-out; `long-none-param` 1000→9999; `long-none-return` sn→none-out; `t-qhap-generic` dòng 2 → `None`); pins `sem.rs`/`vis_test.rs`/`diag_cmp.rs` (487/399/53/35)/`roundtrip.rs`/`vs_interp.rs` (434)/`fuzz_mutate.rs` (399)/CHEAT-SHEET/FACTS đồng bộ kèm lý do.
- **Cache** `PHIEN_BAN` bump `+vd241-coercion-2026-11-19` (memory VD-129; `khoa_cache_ghim` ghim lại).
- **Docs đã cập nhật**: `docs/ViScript-thiet-ke.md` (mục Cập nhật 2026-11-19, §3.1, §4.4, §6.1, §18, EBNF Phụ lục A; VD-19 đánh dấu supersede); `agent/BAO-CAO-A0-2026-11-19-OPTION-FIX.md` addendum §8 đính chính (nội dung cũ giữ nguyên); `DEM-2026-11-18`, `FACTS`. `docs/SPEC-lex.md` **không đụng**; SPEC.md (nếu có) vẫn thuộc A0.
- **Cổng (Linux worktree, stable 1.99.1)**: `cargo test --workspace --no-fail-fast` **1299/0** (gồm ca wasm32 `targets_wasm_vis_types`); `vis test test/sem` **487/487** · `vis test --vm` **487/487** byte-identical; mutation theo lệnh A0 (xem báo cáo).
- **A0 cần quyết** (không tự mở): (i) `var` không annot + plain assign vào ô suy-Option, và gán lại đích trường/chỉ số — nâng hay không; (ii) macOS/Windows re-verify; (iii) SPEC.md tham chiếu `int??` (nếu có).


### Windows re-verify bản fix Option — 2026-11-19 (`cc7aef9`..`2c006af`)

- psvn @ `2c006af`: build OK · sem **487/487 ×2** · CRLF 3/3 · **`vd_205` 22/22** (gồm `option_coercion_matrix` exact stdout + engine parity).
- `./vis-dev gate --fast`: mọi bước **product** PASS (fmt/clippy/R1..R6/self-test/explain/report-check/build/doc-examples); dừng như cũ ở L0 test (baseline VS3 — không hồi quy).
- Fix test-harness 2 commit `cc7aef9`+`2c006af`: `option_coercion_matrix` tag chứa `?` (không hợp lệ tên tệp Windows) ⇒ os error 123 ở hai chỗ (tên thư mục tạm + tên `.vis`). Không đụng sản phẩm.
- **Không mở rộng tuyên bố OS hỗ trợ** (Windows vẫn Tier 2 tại `349af67`; bản fix `73780a5`+ test-only được re-verify — ghi nhận vào `agent/WINDOWS-BASELINE-2026-11-19.md` bổ sung).

### 2026-11-19 (chiều muộn) — ghi nhận tổng kết (Agent 2; không phải ratify)

- 2026-11-19 (chiều muộn) — SQLite blockers xong + pending freeze (8fd5134); psvn native build trong check; CSV join program đang được viết; macOS verify pending (máy bận); Windows re-verify done @ 2c006af; RFC typed serialization cần A0 ratify; B-3, Windows portability, Tier 1 — chưa mở.

### Quyết định Agent 0 · 2026-11-20 (phiên 2026-11-20, uỷ quyền "tự làm agent0 quyết định") — ghi nhận, không mở phạm vi

**Người quyết định**: Agent 0 (qua uỷ quyền trực tiếp trong phiên của người dùng).
**Mốc dữ liệu mọi quyết định**: HEAD `c12979f` (docs SYNC, 2026-11-20); Windows verify exit 0 @ `c7cef71`/`2d93101`/`5f4edd7`; Linux gate PASS ×3; nightly PASS @ `4cae3e3` (fuzz 19,073,168 ca, vi phạm 0); fresh-clone gate PASS @ `701d86e` + `4e2bdde`; report-check PASS (30 báo cáo / 243 hàng); `pha1-rc1` `05e42d0…` nguyên vẹn; **freeze** (+9 `std.sqlite.*`) nguyên vẹn.

1. **VD-winport (Windows test-suite portability) = DONE kỹ thu"** — Windows verify **exit 0** toàn workspace @ `c7cef71` / `2d93101` / `5f4edd7` (150 khối `test result: ok`, 0 failed). **Tier 2 giữ nguyên**; Tier 1 **không tuyên bố** (cần VD riêng + required CI Windows + runner).
2. **macOS verify không thực hiện được ở phiên này** — `vis-remote mac --check` báo `nhan: Linux 7.0.0-43-generic x86_64 / Ubuntu 26.04.1` (mapping `mac ⇒ bash -s LOCAL`); **không có máy Darwin/arm64** ở phiên này; "gate macOS" ghi ở DEM/JOURNAL phiên trước là chạy trên **chính máy Linux này**. — **Không tuyên bố macOS**; giữ nguyên "chưa verify; không kết luận" ở tài liệu hiện trạng; lịch sử giữ nguyên.
3. **#146 (TOCTOU)** — đã gỡ bởi ST-174/VD-176 (`symlink_metadata`, `quyen.rs:540–541`, không đổi kỳ vọng/ngưỡng/test); BLOCKED §6 cũ về mặt kỹ thuật đã lỗi thời nhưng **giữ nguyên lịch sử** theo VD-142.
4. **Pha 2: KHÔNG mở trong phiên này** — cần A0/VD riêng + thiết kế; **RFC Typed Serialization: KHÔNG ratify** — tài liệu RFC hiện ghi "SUPERSEDED" (2026-11-19 ~20:30) và nằm ở `(PA-A/PA-B, §3-C, §3-D, §11-A..L)` muôn; **B-3 (module/impl propagation + bound generic): KHÔNG mở** — scope lớn, cần VD riêng; **#147-mac / #152-vế-mac**: máy không có — hoãn; **`#149`**: giữ nguyên (audit đã ghi, sửa khi đụng tệp); **`#151` / VS3**: không mở; **Windows Tier 1**: không tuyên bố.
5. **Freeze (`api_dong_bang.rs`, `API-STD*.txt`, `API-CODES.txt`, `API-VBC1.txt`, `docs/SPEC-lex.md`) KHÔNG đổi** — narrow freeze `+9 std.sqlite.*` là giới hạn đã A0 phê duyệt; không mở rộng.
6. **Fresh-clone gate**: ghi nhận PASS tại `401d86e` (clone `/home/ps/.tmp-clone-gate/verify`, gate toàn L0 xanh) và `4e2bdde` (clone `verify` sau HEAD bump — kết quả PASS).
7. **Phạm vi quyết định**: các quyết định 1–6 chỉ ghi nhận hiện trạng đã đo trong phiên; không thay đổi sản phẩm/ngữ nghĩa/golden/ngưỡng/freeze/API. **Không mở pha 2; không mở B-3; không thay đổi API; không tuyên bố nền không đo được.**
