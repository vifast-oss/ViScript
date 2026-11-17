<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS-ERGO · Công thái học ViScript — 12 điểm + VS0412
**Trạng thái**: **CHỜ AGENT 0 DUYỆT** (cùng đợt 6 tài liệu VS3) · **ST-172** · **Chỉ đọc, 0 dòng mã**
**Phạm vi**: **VD-168** thay **mục 2 và mục 4** của VD-167; mục 1, 3, 5 của VD-167 giữ nguyên. (Bản trước chỉ có 7 mục a–g; bản này **bổ sung** theo VD-168.)
**Căn cứ**: VD-167/VD-168; probe bằng `target/debug/vis` (cổng L0 **1179/0 @ `0df923c2`**) ngày 2026-11-17.
**Quy ước đường dẫn (VD-165 §2)**: tính từ **gốc ViScript**. **Mức ưu tiên**: **P0** chặn/đau nhất · **P1** nên làm trước 1.0 · **P2** để sau.

## §0. Phương pháp
Mỗi mục: **hiện trạng** (`file:dòng` + probe tái lập) · **đề xuất** (phương án + rủi ro) · **ca đỏ-trước** · **ảnh hưởng** mã/test/tài liệu · **ưu tiên**.
Probe ở `/tmp/ergo/*.vis`, chạy `vis run`; kết quả là **thật**, không suy đoán. Đây là **đề xuất**, không phải quyết định thiết kế (VD-166).

## §1. Quy tắc chữ hoa (ghi vào thiết kế — Agent 0)
> **Hàm dựng mang dữ liệu viết hoa (`Some`, `Ok`, `Err`, `Circle`); literal không mang dữ liệu là từ khoá viết thường (`true`, `false`, `none`).**

**Hiện trạng**: đúng một phần — `Some/Ok/Err` viết hoa; `none` viết thường đúng; **`some(v)` viết thường vẫn được nhận làm pattern** (probe `c8`: `some(v)` khớp) trong khi **constructor** `some(1)` là VS0501. Tức quy tắc **chưa được thi hành** cho pattern.
**Đề xuất**: chấp nhận quy tắc; **pattern chỉ nhận `Some(v)`** (bỏ `some(v)`), `none` giữ; constructor giữ `Some`. Cần Agent 0 duyệt vì đây là **đổi ngôn ngữ** (phá mã đang dùng `some(v)` nếu có).
**Ca đỏ-trước**: `some(v)` ⇒ lỗi rõ (“dùng `Some`”); `Some(v)` xanh; `none` xanh.
**Ảnh hưởng**: `crates/vis-types/src/tyck/pat.rs:131`, parser pattern, docs/cheatsheet. **P1**

## §2. (a) Optional: pattern `Some(v)`; tự gói `T` thành `T?`
**Hiện trạng**: pattern **nhận cả `Some(v)` và `some(v)`** (probe `c6`,`c8`); `none` khớp. **Tự gói đã có**: `let x: int? = 5` chạy, in `5` (probe `n1`) — không cần `Some(5)`.
**Đề xuất**: giữ tự gói; chốt pattern **chỉ `Some(v)`** (theo §1); bổ sung **ca khoá** cho tự gói và cho pattern.
**Ca đỏ-trước**: `let x: int? = 5` ⇒ `some(5)` khi in; `match x: Some(v): …`; `some(v)` ⇒ lỗi (sau khi chốt).
**Ảnh hưởng**: `tyck/pat.rs`, tyck coerce (`tyck/mod.rs`), docs. **P1**

## §3. (b) Dựng record theo tên kiểu + giá trị mặc định
**Hiện trạng**: **chưa có** cú pháp `U(name: "An", age: 20)` ⇒ **VS0201** (probe `n2`); hiện chỉ có `{name: "An", age: 20}` sau khi khai `type U:` (`crates/vis-types/tests/exhaustive.rs:151`). Trường **chưa có** giá trị mặc định (nợ — không phải thiết kế mới).
**Đề xuất** (theo thiết kế §3.2): cho phép `U(field: value, …)`; cho phép `field: T = expr` trong `type`; dựng thiếu ⇒ áp default; thiếu trường không default ⇒ lỗi rõ. Đây là **thiết kế mới** ⇒ **Agent 0 quyết** + cấp số nợ.
**Ca đỏ-trước**: `type U: n: int = 0` + `U(name: "a")` ⇒ `n == 0`; `U()` thiếu trường bắt buộc ⇒ lỗi.
**Ảnh hưởng**: parser type/record, tyck, lower/codegen, exhaustive, docs §3.2. **P1**

## §4. (c) `AnyError`: `?` tự chuyển các kiểu lỗi khác nhau — **ưu tiên cao**
**Hiện trạng**: `?` **buộc** hàm trả đúng `Result`/`Option`: `fs.read(...)?` trong hàm trả `str?` ⇒ **VS0412** “`?` trên `Result` chỉ dùng được khi hàm trả `Result[…, …]`, hàm này trả `str?`” (probe `n9`; `crates/vis-types/src/tyck/expr.rs:177,194`). Không có kiểu lỗi chung ⇒ không viết được hàm “dùng nhiều nguồn lỗi”.
**Đề xuất**: `AnyError` (kiểu lỗi tổng) + `?` tự bọc `E → AnyError`; hiển thị giữ mã/ngữ cảnh gốc; `try/e catch` giữ nguyên. Đây là **thiết kế/API mới** ⇒ **Agent 0 quyết** (ưu tiên cao theo A0).
**Ca đỏ-trước**: hàm trả `T?` (hoặc `Result[T, AnyError]`) gọi `fs.read(...)?` và `json.decode(...)?` ⇒ chạy, lỗi giữ mã gốc; lỗi hai nguồn khác nhau đều bọc được.
**Ảnh hưởng**: `tyck/expr.rs:177,194`, `ty.rs` (kiểu lỗi), runtime (`?`), docs §4.4. **P0**

## §5. (d) Tham số tên trong `fn` thường: `f(1, b: 5)`
**Hiện trạng**: **VS0201** (probe `n3`); chỉ hỗ trợ tham số tên kiểu `--flag` theo thiết kế (`docs/viscript/ViScript-thiet-ke.md:412`), và `--b` chỉ dùng trong `cmd`.
**Đề xuất**: cho phép `f(1, b: 5)` ở `fn` thường (bắt buộc sau tham số vị trí); **không** đổi cú pháp `--b` của `cmd`; trùng tên/thiếu/sai thứ tự ⇒ lỗi rõ.
**Ca đỏ-trước**: `f(1, b: 5)` chạy; `f(b: 5, 1)` ⇒ lỗi; `f(1, x: 2)` ⇒ lỗi tên.
**Ảnh hưởng**: parser call args, tyck call, lower, docs. **P1**

## §6. (e) `const` suy kiểu khi vế phải là literal
**Hiện trạng**: `const A = 1` ⇒ **VS0201** (probe `n4`); hiện phải ghi `const A: int = 1`.
**Đề xuất**: cho phép bỏ kiểu khi RHS là **literal** (số/chuỗi/bool); không suy từ biểu thức phức tạp (giữ tường minh).
**Ca đỏ-trước**: `const A = 1` ⇒ `A` kiểu `int`; `const S = "x"`; `const B = 1 + 1` ⇒ lỗi (cần kiểu).
**Ảnh hưởng**: parser const, tyck, docs. **P2**

## §7. (f) `panic(...)` và `newtype`: làm xong hoặc bỏ khỏi danh sách từ khoá trước 1.0
**Hiện trạng**: `panic("hỏng")` ⇒ **VS0501** “tên `panic` chưa hỗ trợ ở VS0” (probe `n5`). `newtype Met = float` **không báo lỗi, bị bỏ qua im lặng** (probe `n11` chạy `print(1)`, exit 0), nhưng dùng `Met` ⇒ **VS0301** “kiểu `Met` không có trong VS0 (VD-43)” (probe `n8`) ⇒ **từ khoá `newtype` được nhận nhưng không hạ** — nguy hiểm vì **im lặng**.
**Đề xuất**: hoặc (i) làm `panic(...)` (dừng chương trình, mã thoát riêng) + `newtype` (kiểu mới có kiểm); hoặc (ii) **xoá khỏi danh sách từ khoá** trước 1.0 và báo lỗi rõ khi gặp. Việc “im lặng bỏ qua” phải bị **cấm** dù chọn hướng nào.
**Ca đỏ-trước**: `newtype` + dùng kiểu ⇒ hoặc chạy đúng, hoặc **lỗi rõ** (không im lặng); `panic("x")` ⇒ exit/mã xác định.
**Ảnh hưởng**: lexer/parser keyword, lower, diag, docs. **P1**

## §8. (g) `str + str`
**Hiện trạng**: `"a" + "b"` ⇒ **VS0412** “toán tử `+`: `str` và `str` không cùng kiểu/đơn vị” (`crates/vis-types/src/tyck/expr.rs:501`).
**Đề xuất**: **giữ lỗi + gợi ý `"{a}{b}"`** (khuyến nghị, 1 dòng) hoặc cho phép nối (đổi ngôn ngữ).
**Ca đỏ-trước**: `"a"+"b"` ⇒ VS0412 **và** chuỗi chứa `{a}{b}`.
**Ảnh hưởng**: `tyck/expr.rs:501`, golden. **P1**

## §9. (h) `xs[i]` trả `T?`; `str` không có chỉ số/slice
**Hiện trạng**: `xs[i]` trả **`T`** (không `T?`): `let y: int = xs[0]`, `xs[0]+1` OK (probe `b1`,`b6`); ngoài phạm vi ⇒ **VS0503** runtime (`b4`). `s[0]` ⇒ VS0412 có gợi ý; `s[0:2]` ⇒ **VS0201** (`b2`,`b3`).
**Đề xuất**: **giữ** `xs[i]: T` + VS0503 (đổi `T?` là **đổi thiết kế**, phá mọi chỗ dùng); **giữ** `str` không có chỉ số/slice (VD-18); chỉ cải thiện thông điệp `s[a:b]` thành gợi ý `.chars()`.
**Ca đỏ-trước**: `xs[5]` ⇒ VS0503; `s[0]` ⇒ VS0412 gợi ý; `s[0:2]` ⇒ VS0201 (hoặc gợi ý mới).
**Ảnh hưởng**: tyck index, parser slice, docs. **P2**

## §10. (i) `print` nhiều đối số
**Hiện trạng**: `print("a","b")` ⇒ **VS0401** “cần 1 đối số, được 2”; `print()` ⇒ VS0401 (`crates/vis-types/src/tyck/expr.rs:1303-1319`).
**Đề xuất**: cho phép `print(a, b, …)` cách nhau **một dấu cách**, newline cuối; `print()` ⇒ newline (rủi ro thấp — nới arity một builtin).
**Ca đỏ-trước**: `print("a","b")` ⇒ `a b\n`; `print()` ⇒ `\n`.
**Ảnh hưởng**: `tyck/expr.rs:1303`, interp, golden. **P1**

## §11. (j) `s.f(…)` cho mọi hàm `str`
**Hiện trạng**: `s.upper()/split()/len()` **chạy** (`f1`,`f2`); `trim/replace` ⇒ **VS0403** (“… (ST-13: len upper lower split chars contains)”, `crates/vis-types/src/tyck/expr.rs:1004`); `use std.str` + `upper(...)` ⇒ **VS0501** (“tên `std` chưa hỗ trợ ở VS0”) — **module `std` chưa mở ở VS0**.
**Đề xuất**: với bộ hàm **đang có**, `s.f()` đã đúng ⇒ chỉ **khoá ca**; thêm `trim/replace` là tính năng mới (Agent 0); **đừng hứa** `std.str.f` cho tới khi có module `std`.
**Ca đỏ-trước**: `s.trim()` ⇒ VS0403 (hoặc xanh nếu mở); `std.str.upper("x")` ⇒ VS0501.
**Ảnh hưởng**: `tyck/expr.rs:1004`, builtin method, docs. **P2**

## §12. (k) Format spec `"{n:.2}"`
**Hiện trạng**: `"{n}"` chạy (in `3`); `"{n:.2}"`/`"{n:.2f}"` ⇒ **VS0201** (chưa có spec) (`g1`,`g3`).
**Đề xuất**: `{expr:spec}` với `.N` cho `float` (làm tròn — Agent 0 chốt half-even), spec sai ⇒ **mã lỗi riêng** (không dùng VS0201 chung).
**Ca đỏ-trước**: `"{3.14159:.2}"` ⇒ `3.14`; `"{3:.2}"`/`"{n:xx}"` ⇒ lỗi rõ.
**Ảnh hưởng**: parser nội suy (vis-syntax), interp, docs. **P1**

## §13. (l) `duration` in `2h30m` thay vì `150m`
**Hiện trạng**: `let d = 150m; print(d)` ⇒ **`150m`** (probe `n7`), `"{d}"` cũng `150m` (`n10`). Kiểu `duration` có ở `crates/vis-types/src/ty.rs:10,35,87,108`.
**Đề xuất**: chuẩn hoá hiển thị: `≥1h` ⇒ `2h30m`; `<1h` ⇒ `30m`; `<1m` ⇒ `30s`; `<1s` ⇒ `500ms`; `0` ⇒ `0s`. Quy tắc làm tròn/đơn vị nhỏ nhất — Agent 0 chốt. Giữ **giá trị** nguyên (chỉ đổi in).
**Ca đỏ-trước**: `150m` in `2h30m`; `90m` ⇒ `1h30m`; `45s` ⇒ `45s`; `1500ms` ⇒ `1s500ms` (hoặc `1.5s` — Agent 0 chốt).
**Ảnh hưởng**: `Display`/format của `Type::Duration`, golden, docs. **P1**

## §14. VS0412 dùng cho **4 nhóm lỗi khác nhau** — **CÓ**
| Nhóm | Bằng chứng | Vị trí |
|---|---|---|
| Kiểu không khớp chung | `let n: int = "x"` | `tyck/mod.rs:42` |
| **Gán cho ràng buộc bất biến** | `let x = 1; x = 2` (probe `r1`) | `tyck/mod.rs:1125` |
| **Toán tử không hỗ trợ cho kiểu** | `"a"+"b"`, trộn đơn vị (probe `a`) | `tyck/expr.rs:501`, `tyck/num.rs:5,21` |
| `async`/range sai chỗ | `async` trên method; range không `int` | `tyck/mod.rs:818,1067` |

**Đề xuất tách** (Agent 0 chốt số; 04xx trống 0404, 0406–0411): **VS0413** = gán cho ràng buộc bất biến; **VS0414** = toán tử không hỗ trợ cho kiểu; **VS0415** = `async` sai chỗ; **VS0416** = range không `int`.
**Ảnh hưởng**: `crates/vis-diag/src/lib.rs:207-260`, `agent/API-CODES.txt`, mọi test so mã. **P2** (không làm đêm nay).

## §15. Cheat sheet
- `CHEAT-SHEET-ViScript.md:12`: nhãn ✖ đã đổi **“VS0 chưa có” → “chưa có tại `<commit>`”** (tệp đang **không track**; khi tách repo phải đưa vào).
- **Thêm ghi chú**: dùng **`r"…"`** cho chuỗi JSON/thô (tránh `\` bị hiểu là escape — đúng lỗi đang thấy trên Windows, xem `JOURNAL/st169-bang-phan-loai-windows.md` nhóm **c**).

## §16. Tổng hợp ưu tiên
| Ưu tiên | Mục | Lý do |
|---|---|---|
| **P0** | (c) **AnyError** | Chặn viết hàm thực tế dùng nhiều nguồn lỗi; A0 đánh ưu tiên cao |
| **P1** | (g) gợi ý `{a}{b}` · (i) print nhiều đối số · (l) duration `2h30m` · (k) format spec · (a) pattern `Some` · (b) record theo tên + default · (d) tham số tên | Rẻ, giảm ma sát hằng ngày |
| **P1** | (f) `panic`/`newtype`: **cấm im lặng** (đặc biệt `newtype`) | Im lặng bỏ qua là bẫy |
| **P2** | (h) index · (j) `std.str` · (e) `const` suy kiểu · §14 tách VS0412 | Đổi thiết kế/lớn hơn, để sau 1.0 hoặc khi có số liệu |
