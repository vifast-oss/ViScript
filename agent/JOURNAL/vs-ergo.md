<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS-ERGO · Đặc tả 13 mục công thái học (ST-172b, theo VD-170)
**Trạng thái**: **CHỜ AGENT 0 DUYỆT** (cùng đợt 6 tài liệu VS3) · **Chỉ đọc, 0 dòng mã** · Căn cứ: `vs-ergo-facts.md` (ST-172a, APPROVED) + VD-170.
**HEAD khi soạn**: `efa78295`. **Probe**: `target/debug/vis`; cây mã **không đổi** từ `0df923c2` (`git diff --stat 0df923c2..HEAD -- crates/` rỗng) nên kết quả probe của ST-172a vẫn đúng; các mục then chốt đã **probe lại** (`n9`, `r1`–`r4`, `c2`–`c7`, `t1`–`t10`).
**Quy ước đường dẫn**: từ **gốc ViScript**. **Mức**: P0 sửa trước · P1 kế · P2 · P3 sau.

## §0. Phạm vi
Theo **VD-170**: **bỏ** mục (h) `xs[i]→T?` (Q5/VD-18 đã chốt `xs[i]` panic, `xs[i]?`/`xs.get(i)` trả `T?` — chỉ **xác minh**) và **bỏ** phần tự gói `T→T?` của (a) (đã có, probe `n1`). 13 mục dưới đây là **đặc tả đã chốt hướng**, không bàn lại.

## §1. [P0] `newtype` chưa hạ phải **báo lỗi ngay dòng khai báo**
**Hiện trạng**: `newtype Met = float` **bị bỏ qua im lặng** — chương trình chạy, exit 0 (probe `n11`), nhưng dùng `Met` ⇒ `VS0301` tại **dòng dùng**, không phải dòng khai (`n8`: `t6.vis:3:18`). `newtype` có trong `KEYWORDS` (`token.rs:185`) nhưng parser không sinh khai báo.
**Đặc tả**: gặp `newtype` ⇒ chẩn đoán **tại dòng khai**, mã mới (đề xuất **VS0203** “`newtype` chưa hỗ trợ”), kèm “gợi ý: dùng `type`/`alias`”; **không** được im lặng; vẫn tiếp tục phân tích để báo các lỗi khác.
**Ca đỏ-trước**: `newtype Met = float` ⇒ đúng 1 chẩn đoán, `file:dòng` = dòng 1, exit 2; có `main` dùng `Met` ⇒ **không** thêm VS0301 trùng.
**Ảnh hưởng**: parser/khaibao (`parse/khai_bao.rs`), `vis-diag` (thêm mã), golden, `all_codes` + `API-CODES.txt`.
**Task**: **172b-1** (1 h) thêm mã + bắt lỗi; **172b-1b** (1 h) test âm/dương + API-CODES. **Phụ thuộc**: —.

## §2. [P0] Bỏ chữ “VS0” khỏi mọi thông điệp
**Hiện trạng**: nhiều thông điệp còn “VS0”: `VS0501` “tên `some` chưa hỗ trợ ở VS0” (`t7`), “`newtype`/`panic` chưa hỗ trợ ở VS0”, `VS0301` “… không có trong VS0 (VD-43)”, `VS0604`/`VS0701` tương tự.
**Đặc tả**: thay bằng **“chưa hỗ trợ (nợ #N)”** khi có số nợ; không có số ⇒ “chưa hỗ trợ ở bản này”. Không đổi **mã**; phần lớn thông điệp **giữ nguyên** (luật đóng băng §5), chỉ thay cụm “ở VS0”.
**Ca đỏ-trước**: `some(1)` ⇒ `VS0501: tên `some` chưa hỗ trợ (nợ #…)`, **không** chứa “VS0”; `all_codes` không còn chuỗi “VS0”.
**Ảnh hưởng**: mọi `format!` trong `crates/vis-types`, `crates/vis-diag`, test so chuỗi thông điệp, `API-CODES.txt`.
**Task**: **172b-2** (1,5 h) rà + thay; **172b-2b** (1 h) test so khớp. **Phụ thuộc**: —.

## §3. [P0] Cheat sheet thành **doc-test**
**Hiện trạng**: `CHEAT-SHEET-ViScript.md` (161 dòng, **không track**) có ví dụ đánh dấu ✖/✔ nhưng **không ca nào được chạy**; nhiều ví dụ đã sai theo dữ kiện: `if let` (chưa có), `else e:` dạng biểu thức (chưa có, probe `z2`), `--help` (chưa sinh, probe `c2`), `AnyError` (chỉ thiết kế).
**Đặc tả**: mọi ví dụ nhãn “chạy được” ⇒ **một ca test** (fixture + `vis run`/`vis check`) trong bộ test; ví dụ hỏng ⇒ **cổng đỏ**. Ví dụ chưa hỗ trợ ⇒ nhãn **“chưa hỗ trợ (nợ #N)”** + **không** chạy. Sửa nội dung theo dữ kiện (bỏ `if let`, `else`, `--help`, `AnyError` khỏi nhóm “chạy được”).
**Ca đỏ-trước**: chạy **toàn bộ** ví dụ “chạy được” ⇒ 0 đỏ; cố tình sửa 1 ví dụ ⇒ cổng đỏ.
**Ảnh hưởng**: `CHEAT-SHEET-ViScript.md`, bộ test mới (`crates/vis-cli/tests/cheatsheet.rs`), `vis-dev`/`vis script` (đường dẫn), cổng.
**Task**: **172b-3** (2 h) tách ví dụ + sinh test; **172b-3b** (1 h) sửa nhãn sai. **Phụ thuộc**: §1, §2 (để nhãn “chưa hỗ trợ” đúng).

## §4. [P1] `p"..."`: **không xử lý escape**, vẫn nội suy `{}`
**Hiện trạng**: `p"C:\x"` ⇒ **VS0102** escape không hợp lệ (probe `r1`) tức p-string **đang** xử lý escape; SPEC-lex:29 ghi “có escape, có nội suy”; thiết kế:257 nói path literal tường minh.
**Đặc tả** (VD-170#4): `p"..."` **không** xử lý escape (như raw) nhưng **giữ** nội suy `{expr}`; `\` là ký tự thường; **VS0102** thêm gợi ý “dùng `r"..."` cho chuỗi thô hoặc `p"..."` cho đường dẫn”. Cần **sửa SPEC-lex §3** — Agent 0 sở hữu (ST-02a), Agent 1 không sửa.
**Ca đỏ-trước**: `p"C:\x"` ⇒ in nguyên `C:\x`; `p"a{b}"` ⇒ nội suy; `"a\q"` ⇒ VS0102 **kèm gợi ý**.
**Ảnh hưởng**: lexer `lit.rs` (p-string), `vis-diag` thông điệp VS0102, golden, docs.
**Task**: **172b-4** (1,5 h) lexer + hint. **Phụ thuộc**: —.

## §5. [P1] Luật thông điệp đóng băng + **dòng “gợi ý:”**
**Hiện trạng**: 5 mẫu VS0201 **không gợi ý**: `const A = 1` ⇒ “mong `:`, thật `=`”; `s[a:b]` ⇒ “mong `]`, thật `:`”; `U(n: 1)` ⇒ “mong `)`, thật `:`”; `"{x:.2}"` ⇒ “mong xuống dòng…, thật `:`”; `|x|` ⇒ “mong biểu thức, thật `|`” (probe `t5`,`t8`,`t9`,`t10`,`p7`).
**Đặc tả**: **giữ mã + thông điệp chính**; **được** thêm dòng thứ hai `gợi ý: …` (không đổi mã, không đổi câu chính):
- `const A = 1` → `gợi ý: const cần kiểu: const A: int = 1`.
- `s[a:b]` → `gợi ý: str không cắt theo chỉ số — dùng s.chars()/graphemes()`.
- `U(n: 1)` → `gợi ý: dựng record bằng {n: 1}` (tới khi có §11 record-theo-tên).
- `"{x:.2}"` → `gợi ý: format spec chưa hỗ trợ (nợ #…)` (tới khi §11).
- `|x|` → `gợi ý: closure ViScript dùng x => expr`.
**Ca đỏ-trước**: 5 ca so **chính xác** dòng chính (không đổi) + **có** dòng `gợi ý:`; 1 ca khác không có gợi ý (để không phá).
**Ảnh hưởng**: `tyck`/`vis-diag` (thêm dòng), test so khớp, docs.
**Task**: **172b-5** (2 h). **Phụ thuộc**: §2 (chuỗi thông điệp thống nhất trước).

## §6. [P1] Tách mã đa nghĩa VS0412 / VS0501
**Hiện trạng**: `VS0412` phủ 4 nhóm (kiểu; **gán `let`** `tyck/mod.rs:1125`; **toán tử** `tyck/expr.rs:501`; async/range `mod.rs:818,1067`); `VS0501` phủ tính năng chưa hỗ trợ **và** tràn i64 lúc chạy (probe `p2`) **và** `some(1)` (probe `t7`).
**Đặc tả**: mã cũ **chỉ còn phần chung**; mã mới:
| Nghĩa | Mã mới (đề xuất) | Ghi chú |
|---|---|---|
| Gán cho ràng buộc bất biến (`let`/tham số) | **VS0413** | thay chỗ `mod.rs:1125` |
| Toán tử không hỗ trợ cho kiểu (`str+str`, đơn vị) | **VS0414** | thay `expr.rs:501` (trừ khi §10 cho phép nối) |
| `async` đặt sai chỗ | **VS0415** | `mod.rs:818` |
| Range không `int` | **VS0416** | `mod.rs:1067` |
| Tràn i64 lúc chạy | **VS0507** | tách khỏi VS0501 |
| Tính năng chưa hỗ trợ ở runtime | **VS0501** giữ | |
**Bảng chuyển đổi** đầy đủ trong tài liệu `API-CODES.txt` (Agent 0 sở hữu) — Agent 1 **không** sửa tệp đó.
**Ca đỏ-trước**: mỗi ca mã mới đúng 1 mã; ca cũ chuyển sang mã mới; `all_codes` có đủ.
**Ảnh hưởng**: `vis-diag`, mọi test so mã, `API-CODES.txt`, docs; **không** đổi số mã cũ.
**Task**: **172b-6** (2,5 h) + test. **Phụ thuộc**: §5 (thông điệp) — nếu đụng cùng tệp.

## §7. [P2] Viết hoa `Some/Ok/Err`; `none/true/false` thường; **xoá builtin `None`**
**Hiện trạng**: pattern nhận **cả** `Some(v)` và `some(v)` (probe `c6`,`c8`); constructor chỉ `Some`; `None` là builtin in `<builtin None>` (probe `r2`); `lower.rs:53` có `None`.
**Đặc tả**: `Some/Ok/Err` viết hoa **ở cả dựng và pattern**; `none/true/false` thường; **xoá `None`** khỏi builtin; `some(v)` pattern ⇒ **cảnh báo** (mã mới, đề xuất **VS0111**) + `vis fmt` **tự sửa** thành `Some(v)`; sửa văn bản **VD-19** cho khớp (Agent 0).
**Ca đỏ-trước**: `some(v)` ⇒ cảnh báo + fmt sửa; `None` ⇒ VS0301 (không còn builtin); `Some(1)`/`none` xanh.
**Ảnh hưởng**: `tyck/pat.rs`, `lower.rs:53`, fmt, `all_codes`, test, docs.
**Task**: **172b-7** (2 h). **Phụ thuộc**: §2.

## §8. [P2] `AnyError` theo VD-20
**Hiện trạng**: probe `n9`/`r4` (chép nguyên): `r4.vis:4:13: error VS0412: `?` trên `Result` chỉ dùng được khi hàm trả `Result[…, …]`, hàm này trả `str?` (§4.4)`. `json.decode(str)->V` (`API-STD.txt:15`) — **“V” là kiểu biến** (generic) không phải `any` (VD-01 cấm `any`); `?` giữa `fs.read` (Result) và `json.decode` (Option/`T?`) báo VS0412 vì **hai hàm trả hai kiểu khác nhau** (`Result` vs `T?`) — cần `AnyError` để dung hợp.
**Đặc tả** (VD-20): `trait Error { message, source, span }`; `AnyError` xoá kiểu; **mọi `E: Error` tự chuyển** sang `AnyError` qua `?`; `main -> Result[(), AnyError]`; `json.decode` trả `Result[V, AnyError]` (hoặc giữ `V` + lỗi riêng — **Agent 0 chốt**).
**Ca đỏ-trước**: hàm trả `Result[T, AnyError]` dùng `fs.read(...)?` + `json.decode(...)?` ⇒ **chạy**; lỗi giữ mã gốc; hai nguồn lỗi khác nhau bọc được.
**Ảnh hưởng**: `tyck/expr.rs:177,194`, `ty.rs` (thêm `AnyError`), runtime, `stdlib.rs` (chữ ký), docs §4.4.
**Task**: **172b-8a** (3 h) kiểu + `?`; **172b-8b** (3 h) chuyển chữ ký std + test; **172b-8c** (2 h) `main -> Result` + exit code. **Phụ thuộc**: §6 (mã lỗi).

## §9. [P2] Gọi dạng phương thức cho `std.<mod>.f`
**Hiện trạng**: `s.upper()/split()/len()` chạy; `s.trim()/replace` ⇒ VS0403 (`tyck/expr.rs:1004`); `use std.str` + `str.trim(" x ")` **chạy** (probe `z1`) — tức hàm **có**, chỉ thiếu dạng phương thức. Xung đột tên: kiểu `str` vs module `std.str` vs builtin `str(x)` (probe `r3`: `str(5)` ⇒ `5`).
**Đặc tả**: hàm `std.<mod>.f` có **tham số đầu kiểu `T`** ⇒ gọi được `x.f(...)` (đường phương thức ưu tiên inherent; nếu không có thì module). **Kiểm xung đột**: `str(x)` giữ là **ép kiểu** (builtin), `str.upper` là **module**, `x.upper()` là **phương thức** — ba không gian tên riêng, ghi rõ trong docs.
**Ca đỏ-trước**: `"  x ".trim()` ⇒ `x`; `[3,1].sort()` ⇒ `[1,3]`; `str(5)` vẫn `5`; tên trùng (`str.len`) không mơ hồ.
**Ảnh hưởng**: `tyck/expr.rs` (method lookup), `stdlib.rs`, docs, thông điệp VS0403.
**Task**: **172b-9** (2,5 h). **Phụ thuộc**: §8 (chữ ký std ổn định).

## §10. [P2] `str + str` ⇒ **cho phép nối chuỗi**
**Hiện trạng**: `"a"+"b"` ⇒ VS0412 “toán tử `+`: `str` và `str` không cùng kiểu/đơn vị” (`tyck/expr.rs:501`; probe `t1`).
**Đặc tả**: `+` trên `str` ⇒ nối; giữ VS0412 cho **trộn đơn vị**; cân nhắc `+=`? (không có). **Không** đổi `+` số.
**Ca đỏ-trước**: `"a"+"b"` ⇒ `ab`; `"a" + 1` ⇒ VS0412/VS0414 (không nối ngầm số); `1+"a"` cũng lỗi.
**Ảnh hưởng**: `tyck/expr.rs:501` + `tyck/num.rs`, interp/VM, golden, docs.
**Task**: **172b-10** (1,5 h). **Phụ thuộc**: §6 (mã lỗi).

## §11. [P2] `print` nhiều đối số · format spec · record-theo-tên + default · `if let`
**Hiện trạng**: `print("a","b")` ⇒ VS0401 (`expr.rs:1303-1319`); `"{x:.2}"` ⇒ VS0201 (probe `t10`); `U(n: 1)` ⇒ VS0201 (`t9`); `if let` **không có** trong từ khoá.
**Đặc tả**: `print(a, b, …)` in **cách nhau 1 dấu cách**, newline cuối; `print()` ⇒ `\n`. `{expr:spec}` chạy runtime (`.N` cho float, làm tròn Agent 0 chốt). `U(name: …)` dựng record theo tên kiểu, **áp default** (cần default trường `field: T = expr` — nợ riêng). `if let` **chưa** làm (P2, ghi nợ).
**Ca đỏ-trước**: `print("a","b")` ⇒ `a b`; `"{3.14159:.2}"` ⇒ `3.14`; `U(n: 1)` với `type U: n: int = 0` ⇒ `n==0`; `if let` ⇒ VS0201 (tới khi làm).
**Ảnh hưởng**: `expr.rs:1303`, parser nội suy, parser record, tyck record, docs.
**Task**: **172b-11a** (1 h print) · **172b-11b** (2,5 h format) · **172b-11c** (3 h record+default) · **172b-11d** (2 h if let). **Phụ thuộc**: §5.

## §12. [P3] `const` suy kiểu · `duration` in chuẩn hoá · `cmd --help`
**Hiện trạng**: `const A = 1` ⇒ VS0201 (`t5`); `150m` in `150m`, `1h+30m` in `90m` (probe `n7`,`p9`); `greet --help` ⇒ VS0602 “không có cờ `--help`” (probe `c2`).
**Đặc tả**: `const` suy kiểu khi RHS **literal**; `duration` in `1h30m`/`45s`/`1s500ms` (quy tắc Agent 0 chốt); `cmd` tự sinh `--help` (mô tả tham số, cờ).
**Ca đỏ-trước**: `const A = 1` ⇒ int; `150m` ⇒ `2h30m`; `dem --help` ⇒ văn bản trợ giúp, exit 0.
**Ảnh hưởng**: parser const, Display duration, `cmdsig`/CLI.
**Task**: **172b-12a** (1 h) · **172b-12b** (1,5 h) · **172b-12c** (2,5 h). **Phụ thuộc**: —.

## §13. [P3] Q2: giữ chuyển ngầm `int→float` + số học trộn
**Hiện trạng**: `1 + 1.5` ⇒ `2.5`; `let f: float = 1` ⇒ `1` (probe `p3`,`p4`); văn bản Q2 (`ViScript-thiet-ke.md:1104`) nói “không chuyển đổi ngầm” **mâu thuẫn mã**.
**Đặc tả**: **giữ** hành vi mã (đã chốt VD-170#13); sửa **văn bản Q2** cho khớp (Agent 0 sở hữu thiết kế); thêm ca khoá.
**Ca đỏ-trước**: `1+1.5` ⇒ `2.5`; `let f: float = 1` ⇒ `1.0`; `let n: int = 1.5` ⇒ lỗi.
**Ảnh hưởng**: docs (Agent 0), test.
**Task**: **172b-13** (0,5 h test + đề nghị sửa docs). **Phụ thuộc**: §2.

## §14. Ba script theo thành ngữ (so số dòng với Python)
**Script 1 — log** (`filter/len`, dùng closure `=>` — đã kiểm):
```
use std.fs
fn main():
    match fs.read("/tmp/ergo/app.log"):
        ok(noi):
            let dong = noi.split("\n").filter(l => l.len() > 0)
            print("tong dong: {dong.len()}")
            print("ERROR: {dong.filter(l => l.contains("ERROR")).len()}")
            print("WARN : {dong.filter(l => l.contains("WARN")).len()}")
        err(x):
            print("loi doc: {x}")
```
**12 dòng** vs Python **6 dòng** — dài hơn vì **bắt buộc** xử lý `Result` (`match ok/err`) và `use std.fs`; Python ném exception. Ghi nguyên nhân: **an toàn kiểu + lỗi tường minh**, không phải cú pháp rườm.

**Script 2 — JSON**: (như ST-172a, **12 dòng** vs Python **8**) — dài hơn do `match` cho `fs.write`/`fs.read`; có thể rút khi có `?`/`AnyError` (§8).

**Script 3 — CLI có cờ**: `fn main(args)` **25 dòng** vs Python **8**; `cmd` **chưa dùng được** cho logic: `--help` chưa sinh (`c2`), thân `cmd` không trộn script/lệnh (`cf`: VS0201), tham số `cmd` **không thay được** vào pipeline (`c6`/`c7`: `open tep` đọc tên literal; `$tep` ⇒ VS0604). Khi §12c xong, script 3 rút còn ~10 dòng.

## §15. Thứ tự thực thi đề nghị
**P0**: §1 → §2 → §3 (3 tuần tự, ước 5–6 h). **P1**: §4 → §5 → §6 (5–6 h). **P2**: §7 → §9 → §10 → §11a → §11b → §11c → §8 (a/b/c) → §11d. **P3**: §12a/b/c → §13. Mọi task ≤ 3 h; mục ≥3 h đã tách (8a/8b/8c, 11a–d).
