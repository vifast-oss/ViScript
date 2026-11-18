<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS-ERGO · Bộ quy tắc công thái học ViScript (R1–R24) — ST-172b
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · đã áp [SỬA] tại **ST-178** · **ST-172b** · **Chỉ đọc, 0 dòng mã**
**Căn cứ**: **VD-171** (thay mục 7, 8, 10, 11, 13 của VD-170; sửa VD-19/Q2/§1.6) + **VD-172** (R11–R21) + **VD-174** (R14/R22/R23/R24) + **VD-178** (duyệt CÓ SỬA: C1 mã thoát, C2 từng R, C3 thứ tự) + `vs-ergo-facts.md` (ST-172a, APPROVED) + probe tại HEAD `6d01d8d3`.
**Mỗi quy tắc**: Hiện trạng (probe + `file:dòng`) → Đặc tả → Ca đỏ-trước → Bảng chuyển đổi mã (nếu có) → Ảnh hưởng → Task ≤ 3 h → Phụ thuộc. **Không bàn lại hướng.**

## C1 · Bảng mã thoát (VD-178 — áp cho **mọi** R)
| Mã thoát | Nghĩa |
|---|---|
| 0 | thành công |
| 1 | `main` trả `Err` (lỗi dự kiến) |
| 2 | lỗi biên dịch/kiểm tra |
| 101 | panic: lỗi lập trình (tràn số, chỉ số ngoài phạm vi, chia 0, assert, gửi sai worker lúc chạy) |

- Test hiện mong exit 1 cho **VS0502/VS0503** phải đổi sang **101** — **Agent 0 duyệt đổi kỳ vọng**, làm trong **R9**.
- Lỗi chạy **có mã** vẫn in mã, nhưng thoát **101** (không phải 1).
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:7` (ST-178) — thêm bảng mã thoát C1 (0/1/2/**101**).

## Bảng tổng (VD-173 — pha, task, giờ, ảnh hưởng)
| R | Nội dung | Pha | Task | Giờ | Test/ví dụ hiện có bị ảnh hưởng |
|---|---|---|---|---|---|
| P0 | newtype báo lỗi · bỏ “VS0” · cheat sheet doc-test | **1** | 3 | 5 | 8 (golden + `API-CODES`) |
| R1 | Chữ hoa enum + cảnh báo pattern thường (**gộp R2**: bỏ `none`) | **1** | 3 | 3,5 | 6 (golden, fmt) |
| R2 | Option/Result enum std, `None` là biến thể, bỏ từ khoá `none`, `T??` | **1** | 3 | 8 | 8 (option/kenh) |
| R3 | `if let`/`while let` nhận **pattern**, `let else` khối thoát | **1** | 2 | 2,5 | 6 |
| R4 | `Error` record + `Json` enum + `AnyError` | **1** | 3 | 8 | 30 (toàn bộ test std) |
| R5 | tên đối số, record theo tên, method toàn std | **1** | 4 | 8,5 | 10 (record/gọi hàm) |
| R6 | `p"…"` thô, `str+str`, format spec, `print` nhiều đối số | **1** | 4 | 6,5 | 5 (str/golden) |
| R7 | `/`→float, `//`, `%`, const suy kiểu | **1** | 3 | 4,5 | **4** (run.rs:98,100,112; interp_v2.rs:77) |
| R8 | xác minh `xs[i]?`/`xs.get` | **1** | 1 | 1 | 1 |
| R9 | mỗi mã một nghĩa, gợi ý, bỏ “VS0”, báo tại chỗ | **1** | 3 | 4,5 | 40 (mọi test so mã) |
| R11 | hàm dựng (record/enum payload) | **1** | 2 | 3,5 | 3 |
| R12 | `type X = T`, `type Email(str)`, bỏ `alias`/`newtype` | **1** | 1 | 2,5 | 2 |
| R16 | `let` che tên, `var` không khai lại | **1** | 1 | 1 | 2 |
| R17 | điều kiện bool + **gợi ý** `is_empty()` | **1** | 1 | 0,5 | 2 |
| R19b | tràn i64 **panic** (phá test VS0501 hiện có) | **1** | 1 | 1 | 1 |
| R19c | float literal cần chữ số sau `.`; `5.bit_or(3)` | **1** | 1 | 1 | 1 |
| **Tổng pha 1** | | | **30** | **≈ 62** | **≈ 120** |
| R13 | biến thể không tiền tố + prelude | 2 | 1 | 1,5 | 0 |
| R14 | method người dùng (`impl P:`, `self`, `var self`); **trùng tên ⇒ lỗi** — **đầu pha 2** | 2 | 2 | 5 | 0 |
| M1–M3 | Nguyên tắc mô hình (doc) | — | 0 | 1 | 0 |
| R15 | nối dòng `.`/`??`/`and`/`or` (phần `() [] {}` đã chạy) | 2 | 1 | 1,5 | 0 |
| R18 | pattern `as`, record, `while let` | 2 | 2 | 3,5 | 0 |
| R19a | `0o755`, bit method | 2 | 2 | 2,5 | 0 |
| R20 | `assert`, `use as`, `use {}`, list/range | 2 | 2 | 3 | 0 |
| R21 | pipeline chỉ shell/cmd (tài liệu) | 2 | 1 | 0,5 | 0 |
| R10 | đơn vị in chuẩn hoá + **cộng cùng loại hợp lệ**, khác loại lỗi | 2 | 2 | 3 | 3 (golden đơn vị) |
| R22 | Trait là kiểu (`list[Shape]`, bảng ảo, đóng gói); **bỏ ép ngược** | 2 | 2 | 9 | 0 |
| R23 | Gán lồng sâu trên `var` (`u.addr.city = x`); ca `U(addr: A(city: "A"))` | 2 | 3 | 4 | 0 |
| R24 | `.lazy()` → `Iterator[T]` duyệt lại; `filter/map` **giữ** collection; R20 **không** chờ R24 | 2 | 3 | 6 | 0 |
| R19d | khoảng trắng quanh `.` **không đổi nghĩa** (`5.bit_or(3)` ≡ `5. bit_or(3)`); `5.` ⇒ lỗi gợi ý `5.0`; fmt chuẩn hoá **không cách** sau `.` (VD-193) | 2 | 1 | 1 | 1 |
| **Tổng pha 2** | | | **21** | **≈ 39** | **≈ 4** |

## R1 · Chữ hoa — **Pha 1** (làm **chung một khối với R2**)
**Hiện trạng**: pattern nhận **cả `Some(v)` lẫn `some(v)`** (probe `c6`,`c8`); `None` là **builtin** in `<builtin None>` (probe `r2`); `lower.rs:53` có `None`; `true/false/none` viết thường.
**Đặc tả**: kiểu/trait/biến thể enum **UpperCamel** (`Some/None/Ok/Err/Circle`); biến/hàm/module/trường **snake_case**; hằng **UPPER_SNAKE**; `true/false` thường. **Dựng và pattern viết giống hệt nhau.** Pattern thường ⇒ **cảnh báo** + `vis fmt` **tự sửa**.
**Ca đỏ-trước**: `some(v)` ⇒ cảnh báo + fmt sửa `Some(v)`; `Some(1)`/`None` xanh; `let Some = 1` ⇒ lỗi. (**Bỏ** ca “`none` xanh” — `none` là từ khoá bị bỏ, xử ở R2.)
**Chuyển đổi**: `some` pattern → `Some`; thêm **VS0111** (cảnh báo) + fmt. **Không** xoá `None` ở R1 — `None` thành **biến thể của `Option`** ở **R2a**.
**Ảnh hưởng**: `tyck/pat.rs`, `lower.rs:53`, fmt, `all_codes`, golden, docs.
**Task**: **172b-R1a** (1,5 h cảnh báo) · **R1b** (1 h fmt sửa) — **gộp với R2a** (không còn R1c riêng). **Phụ thuộc**: R9 (mã mới).
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:53` (ST-178) — R1+R2 làm chung; bỏ “xoá builtin `None`” riêng ở R1c; bỏ ca “`none` xanh”.

## R2 · Option/Result là enum std — **Pha 1**
**Hiện trạng**: `none` là **từ khoá** (`token.rs:185`); `T??` ⇒ **VS0201** (probe `y2`); `Some(1)` dựng được, `some(1)` ⇒ VS0501 (probe `c4`); tự gói `T→T?` **đã có** (probe `n1`).
**Đặc tả**: `enum Option[T]: Some(T), None`; `enum Result[T,E]: Ok(T), Err(E)`; `T?` = `Option[T]`; **bỏ từ khoá `none`** (`None` là biến thể); **cho phép `T??`**; tự gói **đủ số lớp**, `None` **lớp ngoài cùng**, lớp trong `Some(None)`.
**Từ khoá `none`**: bị **bỏ**; dùng `none` ⇒ **VS0203** (“từ khoá đã bỏ”) kèm gợi ý `None`, `vis fmt` **tự sửa**.
**Ca đỏ-trước** (bắt buộc theo A0):
- `fn first[T](xs: list[T]) -> T?` gọi với `T = int?` ⇒ kiểu trả `int??`.
- `map[str, int?].get` phân biệt **không có khoá** ⇒ `None` vs **khoá = None** ⇒ `Some(None)` vs **có giá trị** ⇒ `Some(Some(v))`.
- `let x: int?? = 5` ⇒ `Some(Some(5))`; `let y: int?? = None` ⇒ `None`; `let z: int?? = Some(None)` ⇒ `Some(None)`.
**Chuyển đổi**: bỏ từ khoá `none` ⇒ **VS0203** + gợi ý `None` + fmt tự sửa; `None` thành biến thể; sửa VD-19 (A0).
**Ảnh hưởng**: `ty.rs`, parser (`T?` lặp), tyck gói nhiều lớp, interp/VM, `lower.rs`, docs.
**Task**: **172b-R2a** (3 h) enum std + bỏ từ khoá `none` (**gộp R1**); **R2b** (3 h) `T??` + gói nhiều lớp; **R2c** (2 h) ca `first`/`map.get`. **Phụ thuộc**: R1.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:62` (ST-178) — `None` là biến thể; từ khoá `none` ⇒ **VS0203** + gợi ý + fmt.

## R3 · Vắng mặt/lỗi: `?`, `??`, `if let`, `let … else` — **Pha 1**
**Hiện trạng** (probe mới, **khác** ST-172a):
- `??` **chạy**: `o ?? 9` ⇒ `5` (`p5`).
- **`if let` CÓ**: `if let v = o:` ⇒ `5` (`y6`); `if let Some(v) = o:` ⇒ **VS0412** “pattern ctor `Some` không khớp kiểu `int`” (`y4`) — pattern kiểm với **kiểu trong**, không phải `int?`.
- **`let … else:` CÓ**: `let x = r else:` + khối `return` ⇒ chạy (`y7`); `else 0` ⇒ **VS0201** (`y5`) — đúng ý A0: **không** có `f() else 0`.
**Đặc tả**: `?` đẩy lỗi lên; `??` giá trị mặc định; `if let` / `let … else` / `match` xử lý tại chỗ. **`let x = expr else e: <khối>`** — khối **bắt buộc thoát**; **Option không có `e`**; **không** có biểu thức `f() else 0`.
- **`if let` / `while let` nhận PATTERN**: viết `if let Some(v) = o:`. Dạng `if let v = o:` (pattern **luôn khớp**) ⇒ **lỗi** kèm gợi ý `Some(v)`. **Một quy tắc, không có dạng tắt.**
**Ca đỏ-trước**: `if let Some(v) = o:` chạy (sửa kiểm pattern theo `T?`); `if let v = o:` ⇒ **lỗi + gợi ý `Some(v)`**; `let x = r else:` không thoát ⇒ lỗi rõ; `f() else 0` ⇒ VS0201.
**Ảnh hưởng**: `tyck` (pattern `if let`/`while let`), parser `let else`, docs.
**Task**: **172b-R3a** (1,5 h sửa if-let pattern) · **R3b** (1 h ràng buộc khối thoát). **Phụ thuộc**: R2.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:75` (ST-178) — `if let`/`while let` nhận **pattern**; dạng `if let v = o:` ⇒ lỗi + gợi ý.

## R4 · Lỗi: `Error` record + `Json` enum + `AnyError` — **Pha 1**
**Hiện trạng**: probe `r4` (copy nguyên): `r4.vis:4:13: error VS0412: `?` trên `Result` chỉ dùng được khi hàm trả `Result[…, …]`, hàm này trả `str?` (§4.4)`; `json.decode(str)->V` (`API-STD.txt:15`); std trả `Result[_, str]`; `AnyError` chỉ là thiết kế (VD-20).
**Đặc tả**: mọi hàm std trả **`Result[T, Error]`** với `Error` là **record cài `trait Error { message, source, span }`** thay `str`; `?` tự chuyển `E: Error → AnyError` (VD-20); `json.decode → Result[Json, Error]`, `Json` là **enum** (`Null/Bool/Num/Str/Arr/Obj`), **bỏ `V`**; `main` có thể trả `Result[(), AnyError]` (in lỗi, **exit 1**).
**Ca chuẩn (2 dòng)**: `let s = fs.read(p)?` · `let v = json.decode(s)?`.
**Ca đỏ-trước**: hàm trả `Result[T, AnyError]` dùng `fs.read()?` + `json.decode()?` chạy; lỗi giữ `message/source/span`; `json.decode` trả `Json`; `main` trả `Err` ⇒ exit **1**.
**Bảng chuyển đổi**: `Result[_, str]` → `Result[_, Error]`; `V` → `Json`.
**Ảnh hưởng**: `tyck/expr.rs:177,194`, `ty.rs`, `stdlib.rs`, mọi test std, docs §4.4. **Phụ thuộc**: R2, R9.

## R5 · Gọi hàm: tên + record-theo-tên + method toàn std — **Pha 1**
**Hiện trạng**: `f(1, b: 5)` ⇒ **VS0201** (`n3`); record **chỉ** `{name: …}` (`exhaustive.rs:151`); `x.f(a)` mới một phần (`z1`: `str.trim`; `s.upper` chạy); `str(5)` chạy (`r3`).
**Đặc tả**: `f(1, b: 5)`; `--b` **chỉ** chế độ lệnh/`cmd`; `User(name: "An", age: 20)`, default **bỏ được**; `x.f(a) ≡ mod.f(x, a)` khi **tham số đầu của `f` có kiểu của `x`** — **toàn bộ std**; báo **xung đột** `std.str`/`str`/builtin `str(x)`.
- **R5b bị HUỶ (VD-210, 2026-11-18; chốt bảng theo VD-215)** — giữ bản ghi ẩn danh (`{ten: 1}` với khoá định danh trần là **bản ghi**, không bao giờ là biến; `{"ten": 1}` khoá chuỗi là **map**; `(expr)` là **cú pháp khoá tính toán chính thức**; trộn định danh trần với chuỗi/`(expr)` bất kể thứ tự ⇒ **lỗi `VS0207`**; `{[k]: v}` khoá danh sách ⇒ **cảnh báo `VS0208`**). VS0205 **dành riêng, không dùng**. ST-208 đổi thành **R5b′** (VD-214/VD-215).
- **Gọi dạng method chỉ áp cho hàm std có tham số đầu là KIỂU CỤ THỂ**; hàm **generic** có tham số đầu `T` **không** thành method của mọi kiểu.
- **`str(x)` vẫn là chuyển đổi**; module `std.str` **vẫn tồn tại**, hàm của nó gọi dạng method.
**Ca đỏ-trước**: `f(1, b: 5)` chạy; `f(b: 5, 1)` lỗi; `User(name: "a")` với `age: int = 0` ⇒ `age==0`; `" abc ".trim()` ⇒ `abc`; `str(5)` vẫn `5`.
**Ảnh hưởng**: parser args/record, tyck call/method, `stdlib.rs`, docs, test record cũ.
**Task**: **172b-R5a** (2 h) · **R5b** (2 h) · **R5c** (2 h) · **R5d** (2,5 h). **Phụ thuộc**: R3, R4.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:95` (ST-178) — `{ten: …}` ⇒ **VS0205**; method chỉ cho tham số đầu **kiểu cụ thể**; `str(x)` giữ là chuyển đổi.

## R6 · Chuỗi — **Pha 1**
**Hiện trạng**: `p"C:\x"` ⇒ **VS0102** (`r1`); `"a"+"b"` ⇒ VS0412 (`t1`); `"{n:.2}"` ⇒ VS0201 (`t10`); `print("a","b")` ⇒ VS0401 (`t4`).
**Đặc tả**: `p"…"` **không escape**, **có** nội suy; `str+str` **nối**; `{expr:spec}` **runtime**; `print(a,b,…)` **cách 1 dấu cách**, `print()` ⇒ `\n`.
**Ca đỏ-trước**: `p"C:\x"` in `C:\x`; `p"C:\thu muc\"` **hợp lệ** (không escape nên `\` cuối không ảnh hưởng); `p"a{b}"` nội suy; `"a"+"b"` ⇒ `ab`; `"{3.14159:.2}"` ⇒ `3.14`; `print("a","b")` ⇒ `a b`.
**Ảnh hưởng**: lexer `lit.rs`, `tyck/expr.rs:501,1303`, parser nội suy, interp, golden.
**Task**: **172b-R6a** (1,5 h) · **R6b** (1,5 h) · **R6c** (2,5 h) · **R6d** (1 h). **Phụ thuộc**: —.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:106` (ST-178) — thêm ví dụ `p"C:\thu muc\"` hợp lệ.

## R7 · Số — **Pha 1**
**Hiện trạng**: `(Div, Int, Int) => Type::Int` (`crates/vis-types/src/tyck/num.rs:57`); runtime `builtin.rs:464`; **test dựa vào `int/int=int`**: `crates/vis-interp/tests/run.rs:98` (`7/2`→`3`), `:100` (`-7/2`→`-3`), `:112` (`5mb/5mb`→`1`), `interp_v2.rs:77` (`50pct/50pct`→`1`), `run.rs:120,429` (chia 0 ⇒ VS0502); `const A = 1` ⇒ VS0201 (`t5`); `1+1.5`⇒`2.5`, `let f: float = 1` (`p3`,`p4`).
**Đặc tả**: chỉ `int→float` chuyển ngầm; **`/` luôn `float`**; **`//`** chia lấy phần nguyên; **`%` khớp `//`**; `const` suy kiểu từ literal; sửa **Q2** (A0).
**Ca đỏ-trước**: `7/2` ⇒ `3.5`; `7 // 2` ⇒ `3`; `-7 // 2` ⇒ `-4`; `7 % 2` ⇒ `1`; **`-7 % 2` ⇒ `1`** (khớp `//`); `5mb/5mb` ⇒ `1.0`; `const A = 1` ⇒ `int`.
**Đơn vị**: **cùng loại** chia nhau ⇒ **float**; **đơn vị chia số** ⇒ **đơn vị**, làm tròn **xuống** tới đơn vị nhỏ nhất (byte, ns).
**Test phải sửa**: 4 ca (run.rs:98,100,112; interp_v2.rs:77) — **Agent 0 duyệt đổi kỳ vọng**, **không** nới ngưỡng; chia 0 giữ.
**Ảnh hưởng**: `num.rs:57-65`, `builtin.rs:434,464`, parser `//`, test, docs Q2.
**Task**: **172b-R7a** (2 h) · **R7b** (1,5 h) · **R7c** (1 h). **Phụ thuộc**: —.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:114` (ST-178) — duyệt đổi 4 test; thêm `-7 % 2 ⇒ 1`; luật đơn vị khi chia.

## R8 · Chỉ số (xác minh) — **Pha 1**
**Hiện trạng**: `xs[0]?` ⇒ **VS0412** “`?` chỉ dùng cho `T?`/`Result`, được `int`” (`x2`) ⇒ **CHƯA**; `xs.get(1)` ⇒ **`some(2)`** (`x3`) ⇒ **CÓ**; `xs[5]` ⇒ VS0503 (`b4`); `s[0]` ⇒ VS0412 gợi ý; `s[0:2]` ⇒ VS0201 (`b3`).
**Đặc tả**: giữ Q5/VD-18; chỉ **xác minh**.
**Ca đỏ-trước**: `xs[i]?` chạy; `xs.get(i)`; `xs[i]` ngoài phạm vi ⇒ VS0503.
**Task**: **172b-R8** (1 h). **Phụ thuộc**: R2.

## R9 · Thông báo lỗi: mỗi mã một nghĩa — **Pha 1**
**Hiện trạng**: `VS0412` phủ 4 nhóm (`tyck/mod.rs:1125`, `tyck/expr.rs:501`, `mod.rs:818,1067`); `VS0501` phủ tính năng + tràn i64 (`p2`) + `some(1)` (`t7`); `VS0201` gộp cú pháp; `VS0301` gộp tên/kiểu; `VS0604` gộp `&`/`^`/tiến trình; `VS0701` gộp module/std/pkg; nhiều thông điệp còn **“VS0”**.
**Đặc tả**: mỗi mã **một nghĩa**; lỗi cú pháp hay gặp có **dòng “gợi ý:”**; **bỏ “VS0”**; từ khoá chưa làm (`newtype`…) **báo lỗi tại chỗ dùng**.
**Bảng chuyển đổi** (VD-178 duyệt; `API-CODES.txt` A0 sở hữu):
| Mã cũ | Nghĩa mới | Mã mới |
|---|---|---|
| VS0412 | kiểu không khớp (chung) | **VS0412** |
| | gán ràng buộc bất biến | **VS0413** |
| | toán tử không hỗ trợ cho kiểu | **VS0414** |
| | `async` sai chỗ | **VS0415** |
| | range không `int` | **VS0416** |
| VS0501 | tính năng chưa hỗ trợ | **VS0501** |
| | tràn i64 lúc chạy | **VS0507** |
| | giá trị không gửi được sang worker khác | **VS0508** |
| VS0201 | cú pháp chung | **VS0201** |
| | **từ khoá đã bỏ** (`alias`, `newtype`, `none`) — có gợi ý cách mới | **VS0203** |
| | **`const` cần kiểu khi vế phải KHÔNG phải literal** (R7 cho suy kiểu từ literal) | **VS0204** |
| | dựng record bằng `{…}` | **VS0205** |
| | `let…else` khối không thoát | **VS0206** |
| VS0301 | tên không tồn tại | **VS0301** |
| | kiểu không tồn tại | **VS0303** |
| VS0604 | tính năng chung | **VS0604** |
| | chạy nền `&` | **VS0605** |
| | `^`/tiến trình ngoài | **VS0606** |
| VS0701 | tệp module | **VS0701** |
| | `std` chưa hỗ trợ | **VS0705** |
| | `pkg` chưa hỗ trợ | **VS0706** |
| | hash gói không khớp `vis.lock`/sổ | **VS0707** |
**Gợi ý cho 5 mẫu VS0201**: `const A = f()` → “const cần kiểu” (literal thì suy được — R7); `s[a:b]` → “str không cắt”; `U(n: 1)` → “dựng record bằng tên kiểu” (R11 làm hợp lệ); `"{x:.2}"` → “format spec chưa hỗ trợ” (R6 làm hợp lệ); `|x|` → “closure dùng `x => expr`”.
**Ca đỏ-trước**: mỗi ca mới đúng 1 mã; không thông điệp nào chứa “VS0”; `newtype` báo tại dòng khai; lỗi chạy có mã vẫn in mã nhưng thoát **101** (C1).
**Ảnh hưởng**: `vis-diag`, mọi test so mã, `API-CODES.txt`, docs.
**Task**: **172b-R9a** (2 h) · **R9b** (1,5 h) · **R9c** (1 h). **Phụ thuộc**: —.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:130` (ST-178) — VS0203 = từ khoá đã bỏ; VS0204 = const cần kiểu khi không literal; thêm VS0508/VS0707; thoát 101.

## R10 · Đơn vị — **Pha 2**
**Hiện trạng**: `150m` in `150m`; `1h+30m` in `90m`; `5kb+5kb` ⇒ `10kb`; **`1kb+1kib` ⇒ `2024b`** (`y1`) — **trộn đơn vị KHÔNG báo lỗi**; `50pct+10pct` ⇒ `60pct` (`p9`).
**Đặc tả** (Agent 0 đính chính VD-178):
- **Cộng CÙNG LOẠI đơn vị là HỢP LỆ**, tự quy đổi: `1kb + 1kib`, `1h + 30m` chạy.
- **Chỉ KHÁC LOẠI** (`size + duration`, `pct + size`) mới **lỗi**.
- **In ra**: duration dạng **ghép** (`2h30m`); size theo **hệ thập phân**, **đơn vị lớn nhất có phần nguyên ≥ 1**, **tối đa 2 chữ số thập phân** (`1.5mb`); có **`.fmt_iec()`** cho `kib`/`mib`.
**Ca đỏ-trước**: `150m` ⇒ `2h30m`; `1500kb` ⇒ `1.5mb`; `1kb+1kib` ⇒ **hợp lệ** (tự quy đổi, không lỗi); `1h+30m` ⇒ `1h30m`; `5mb+1h` ⇒ **lỗi** (khác loại); `45s` ⇒ `45s`; `1.5mb.fmt_iec()` ⇒ `1.43mib`.
**Ảnh hưởng**: `Type::Duration/Size` Display, `tyck/num.rs`, golden, docs.
**Task**: **172b-R10a** (2 h) · **R10b** (1 h). **Phụ thuộc**: R7.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:164` (ST-178) — cộng cùng loại hợp lệ; chỉ khác loại lỗi; in ghép/thập phân + `.fmt_iec()`.

## Thứ tự thực thi (A0 chốt — **C3 của VD-178**)
**P0 (VD-170)** → **R9 (+ C1)** → **R6** → **R17** → **R1+R2** → **R3** → **R8** → **R4** → **R5** → **R11** → **R12** → **R16** → **R7** → **R19b** → **R19c**. Xen kẽ được: **VS3-5a**, **VS3-5d**. Sau pha 1: VS3-QĐ-1 → (QĐ-2, QĐ-3, QĐ-4) → QĐ-6; QĐ-5 phần còn lại sau R6. Pha 2 chạy song song VS3 khi không đụng cùng tệp.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:175` (ST-178) — thay thứ tự cũ bằng **C3**.

## Scripts theo bộ quy tắc mới (dạng sẽ có sau khi làm; số dòng vs Python)
**Script 1 — log** (`?`, `if let`, `print` nhiều đối số):
```
use std.fs
fn main():
    let noi = fs.read("/tmp/ergo/app.log")?
    let dong = noi.split("\n").filter(l => l.len() > 0)
    print("tong dong:", dong.len())
    print("ERROR:", dong.filter(l => l.contains("ERROR")).len())
    print("WARN :", dong.filter(l => l.contains("WARN")).len())
```
**7 dòng** vs Python **6** — hơn 1 dòng (`use std.fs`).

**Script 2 — JSON** (2 dòng bằng `?`):
```
use std.fs
use std.json
fn main() -> Result[(), AnyError]:
    let ban = {"ten": "An", "lop": "10A"}
    let ra = json.encode_pretty(ban)?
    fs.write("/tmp/ergo/out.json", ra)?
    let s = fs.read("/tmp/ergo/out.json")?
    let v = json.decode(s)?
    print("da ghi", ra.len(), "byte")
```
**8 dòng** vs Python **8** — **ngang** (nhờ `?` + `Result` main).

**Script 3 — CLI** (`cmd` + `--help` sau R5/R12):
```
use std.fs
cmd dem(tep: str, --dem: bool = false) -> ():
    let dong = fs.read(tep)?.split("\n").filter(l => l.len() > 0)
    if --dem:
        print(dong.len(), "dong")
    else:
        for l in dong:
            print(l)
```
**8 dòng** vs Python **8** — **ngang** (sau khi `cmd` thân script + `--help` xong; hiện `cmd` không trộn script/lệnh — `cf` VS0201, tham số không thay được — `c6/c7`).

---

# PHẦN BỔ SUNG · R11–R21 (VD-172)
**Căn cứ**: VD-172 (bổ sung vào phạm vi ST-172b) · probe HEAD `2f4e8302` (probe trong `/tmp/ergo/v2/*.vis`) · **giữ nguyên** các mục R1–R10 ở trên.
**Mỗi mục**: hiện trạng (probe) → đặc tả → ca đỏ-trước → ảnh hưởng/task. **Không bàn lại hướng.**

## R11 · Mọi hàm dựng là hàm — **Pha 1**
**Hiện trạng**: dựng record theo tên `U(n: 1)` ⇒ **VS0201** (`t9`); khai báo biến thể enum **mang dữ liệu** `Circle(float)` ⇒ **VS0201** (`c2`,`c3`); biến thể **đơn vị** gọi `Shape.Empty` **chạy** (`c1` in `Empty`); `Some(1)` prelude chạy.

**Mã probe nguyên văn (VD-173#4)**:
```
# c2.vis — khai báo biến thể mang dữ liệu
enum Shape:
    Circle(float), Empty
# vis run c2.vis ⇒ c2.vis:2:17: error VS0201: mong `:`, thật `)`

# c3.vis — khai báo biến thể mang dữ liệu (KHÔNG phải "gọi kèm tiền tố")
enum Shape:
    Circle(float)
# vis run c3.vis ⇒ c3.vis:2:17: error VS0201: mong `:`, thật `)`

# c1.vis — biến thể đơn vị (CHẠY)
enum Shape:
    Circle
    Empty
# vis run c1.vis ⇒ in `Empty`
```
⇒ **Kết luận**: biến thể **có dữ liệu** chưa khai báo được (VS0201), dù cheat sheet dựng `Circle(2.0)`; biến thể **đơn vị** khai báo + `Shape.Empty` **chạy**.

**Đặc tả**: record, biến thể enum, kiểu bọc đều gọi **như hàm**: `Circle(2.0)`, `Circle(r: 2.0)`; **tên đứng sau vị trí**; cùng luật với `fn`.
**Ca đỏ-trước**: `type C: r: float` + `C(2.0)` và `C(r: 2.0)`; `enum S: Circle(float)` + `Circle(2.0)`; `C(2.0, r: 1.0)` ⇒ lỗi (vị trí sau tên).
**Ảnh hưởng**: parser (biến thể mang dữ liệu), tyck gọi, lower/interp, docs.
**Task**: **R11a** (2 h) enum payload + gọi · **R11b** (1,5 h) record/wrapper ctor. **Phụ thuộc**: R2.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:225` (ST-178) — sửa chú thích probe `c3` (là **khai báo**, không phải “gọi kèm tiền tố”).

## R12 · Gộp khai báo kiểu (`type X = T`, `type Email(str)`) — **Pha 1**
**Hiện trạng**: `type X = int` ⇒ **VS0201** (`a1`); `type Email(str)` ⇒ **VS0201** (`a2`); **`alias X = int` CHẠY** (`d1`); `newtype Met = float` **bị bỏ qua im lặng** (`n11`) rồi dùng kiểu ⇒ VS0301 (`n8`).
**Đặc tả**: **`type X = T`** thay `alias`; **`type Email(str)`** thay `newtype` (dựng `Email("…")`, mở bằng `let Email(s) = e`); **bỏ từ khoá `alias`, `newtype`**.
**Ca đỏ-trước**: `type X = int`; `type Email(str)` + `Email("a@b")` + `let Email(s) = e`; `alias`/`newtype` ⇒ **VS0203** (từ khoá đã bỏ) + gợi ý cách mới.
**Ảnh hưởng**: parser khai báo, tyck, lower, `KEYWORDS` (`token.rs:185`), docs.
**Task**: **R12a** (2,5 h) `type =` + wrapper. **Phụ thuộc**: R9 (mã cho `alias/newtype`).
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:254` (ST-178) — `alias`/`newtype` ⇒ **VS0203**.

## R13 · Biến thể enum: prelude + tiền tố khi trùng — **Pha 2**
**Hiện trạng**: `Shape.Empty` (dùng `.`) **chạy** (`c1`); `Some/None/Ok/Err` prelude chạy; biến thể không tiền tố khi duy nhất — **chưa xác minh**; trùng tên giữa 2 enum — **chưa có** (do R11a chưa xong).
**Đặc tả**: không cần tiền tố khi tên **duy nhất**; trùng tên ⇒ **bắt buộc `Shape.Empty`**, lỗi biên dịch **có gợi ý**; `Some/None/Ok/Err` **luôn trong prelude**.
**Ca đỏ-trước**: `enum A: Empty` + `enum B: Empty` + `Empty` ⇒ lỗi + gợi ý `A.Empty`; `Shape.Empty` chạy; `Empty` đơn nhất chạy.
**Task**: **R13** (1,5 h). **Phụ thuộc**: R11a.

## R14 · Method người dùng: `impl P:` + `self`/`var self` — **Pha 2 (đầu)**
**Hiện trạng**: `impl P:` + `fn len(self)` ⇒ **VS0302** “khai báo trùng `len`” rồi **VS0403** “không phân giải được phương thức” (`a5`); `fn f(var self)` ⇒ **VS0403** (`a6`); method khai trong thân `type` ⇒ **VS0201** (`d2`). ⇒ **method người dùng CHƯA có**; hiện chỉ method builtin (`str`, `list`, `map`).
**Thiết kế đầy đủ**:
1. **Khai báo**: `impl P:` (P là kiểu bản ghi/trait? — **chỉ record/enum trước**); trong thân chỉ `fn`, không `type`/`const`.
2. **`self`**: `fn f(self) -> T` là **chỉ đọc** — không được gán `self.x = …` (lỗi biên dịch, gợi ý dùng `var self`).
3. **`var self`**: `fn f(var self) -> ()` **sửa được**; **chỉ gọi được trên biến `var`** (gọi trên `let` ⇒ lỗi), vì đây là thao tác đột biến tại chỗ (không copy).
4. **`Self`**: viết hoa, là bí danh của kiểu đang `impl`; dùng được ở kiểu trả/đối số/thân.
5. **Quan hệ trait**: `impl Trait for P` (nếu có) **không** đụng `impl P`; method trait gọi như method thường; trait có method mặc định dùng `self`.
6. **Quan hệ R5**: **trùng tên ⇒ LỖI**, **không có ưu tiên ngầm** (bỏ luật “method người dùng thắng method std”); `x.f(a)` ≡ `P.f(x, a)` vẫn đúng khi `f` là method người dùng; **xung đột tên** (method impl vs method std cùng tên cùng kiểu) ⇒ **lỗi biên dịch** có gợi ý đổi tên, không im lặng chọn một bên.
7. **`impl P:` chỉ viết trong CÙNG module khai báo `P`.**
**Ca đỏ-trước**: `p.tang()` trên `var p` chạy + đổi `x`; gọi trên `let p` ⇒ lỗi; `self.x = …` trong `fn f(self)` ⇒ lỗi có gợi ý; `Self` dùng được; trùng tên method impl/std ⇒ lỗi.
**Ảnh hưởng**: parser `impl`, tyck method lookup (**trùng tên ⇒ lỗi**, không ưu tiên ngầm), lower, docs; **0 test hiện có** bị ảnh hưởng (tính năng mới).
**Task**: **R14a** (3 h) `impl` + `self` + `Self` · **R14b** (2 h) `var self` + xung đột tên. **Phụ thuộc**: R11 (hàm dựng), R5 (method std).
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:268` (ST-178) — trùng tên ⇒ **lỗi** (bỏ ưu tiên ngầm); `impl P:` chỉ trong cùng module khai báo `P`.

### Mô hình ViScript — nguyên tắc nền (VD-174 mục 1–3) — **chỉ tài liệu**
1. **Không class, không kế thừa**; trừu tượng chỉ qua `type` + `trait` + `impl`; dùng lại bằng **method mặc định trong trait** hoặc **record chứa record**.
2. **Không lifetime, con trỏ, tham chiếu, `unsafe`, macro người dùng** (trước 1.0); sinh code chỉ qua **derives có sẵn**.
3. **Không tham chiếu dùng chung sửa được** (`Rc<RefCell>`) ở 1.0: trạng thái chung trong một luồng = **map + ID**; giữa luồng = **`Chan`**. Truyền theo giá trị ⇒ **không vòng tham chiếu**, đếm tham chiếu đủ.
**Việc**: Agent 0 ghi vào thiết kế; **không** task mã. (0 task, ~1 h tài liệu, 0 test.)

## R22 · Trait là kiểu (`list[Shape]`, gọi qua bảng ảo) — **Pha 2 (đầu, cùng R14)**
**Hiện trạng**: `trait Shape:` + `impl Shape for Circle:` **parse được** (probe `t1`: chạy, in `1`); nhưng **tên trait không phải kiểu**: `let xs: list[Shape] = []`, `let s: Shape = 5` ⇒ **VS0301** “kiểu `Shape` không có trong VS0 … và không được khai trong tệp” (`t6`,`t7`,`t8`) ⇒ **trait-as-type CHƯA CÓ**.
**Đặc tả (VD-174#4)**: trait dùng được làm **kiểu**; `list[Shape]` chứa **mọi kiểu cài `Shape`**; ViScript **tự đóng gói**, **không** cú pháp `Box`/`dyn`; gọi method **qua bảng ảo**.
- **Luật an toàn đối tượng**: trait dùng làm kiểu được khi mọi method của trait **không có generic riêng**, **không trả `Self`**, **không nhận tham số `Self`** (nếu vi phạm ⇒ lỗi biên dịch có gợi ý thêm `where Self: Sized` — hoặc tương đương).
- **Đóng gói tự động**: gán `Circle(…)`/`int` vào `Shape` ⇒ gói; gán ngược `Shape → Circle` ⇒ **lỗi tường minh** (không có downcast ở 1.0; **ép kiểu ngược ghi nợ sau 1.0** — **không** có “dùng `match` nếu cần”, vì không match được trên kiểu thật của trait).
- **Bảng ảo**: gọi `s.area()` trên `Shape` ⇒ dispatch theo kiểu thật.
**Ca đỏ-trước**: `list[Shape]` chứa **2 kiểu cài** (`Circle`, `Square`) + gọi `area()` qua vòng `for` ⇒ đúng kết quả từng kiểu; trait có method generic ⇒ lỗi an toàn đối tượng có gợi ý; `Shape → Circle` ⇒ lỗi.
**Ảnh hưởng**: `ty.rs` (trait là kiểu), tyck method dispatch, lower/VM (bảng ảo), docs; **0 test hiện có**.
**Task**: **R22a** (3 h) trait-as-type + an toàn đối tượng · **R22b** (3 h) 2 kiểu + bảng ảo + ca. **Phụ thuộc**: R14 (impl), R11.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:289` (ST-178) — bỏ “dùng `match` nếu cần” để ép ngược; ghi nợ sau 1.0.

## R23 · Gán trường lồng sâu trên `var` — **Pha 2 (đầu, cùng R14)**
**Hiện trạng**: `u.addr.city = "B"` ⇒ **VS0412** “gán **lồng** (`x.f[i]`/`x[i][j]`/`x.f.g`) **chưa hỗ trợ ở VS0** — gán lại cả biến `var` bằng `with`/giá trị mới (§4.1)” (`t2`) ⇒ **CHƯA CÓ**, thông điệp đã có sẵn.
**Đặc tả (VD-174#5)**: `u.addr.city = x` và `xs[i].name = x` **hợp lệ** trên biến `var`; **tự sao chép đúng đường dẫn** (copy-on-write từng tầng: sửa bản sao của `addr` rồi gán lại vào `u`), không tạo tham chiếu; trên `let` ⇒ lỗi như gán thường.
**Ca đỏ-trước**: `var u = U(addr: A(city: "A"))` + `u.addr.city = "B"` ⇒ `u.addr.city == "B"`; `xs[0].name = "x"`; trên `let u` ⇒ lỗi; đọc trước/sau để chứng minh không alias.
**Ảnh hưởng**: tyck gán (bỏ chặn VS0412 cho đường lồng), lower/interp/VM (copy-on-path), docs §4.1; **0 test hiện có**.
**Task**: **R23a** (2 h) tyck + lower · **R23b** (1,5 h) ca + `xs[i].f` · **R23c** (0,5 h) lỗi trên `let`. **Phụ thuộc**: R5 (record theo tên), R2.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:300` (ST-178) — ca đỏ-trước viết lại bằng `U(addr: A(city: "A"))` (đúng R5).

## R24 · `.lazy()` + `Iterator[T]` duyệt lại — **Pha 2 (đầu, cùng R14)** — **R20 KHÔNG chờ R24**
**Hiện trạng**: `xs.filter(…)`, `xs.map(…)` **trả list ngay** (eager; `t3`,`t4`: `print(ys)` in list); `.to_list()` ⇒ **VS0403** (`t3`); `.take` ⇒ **VS0403** (`t4`); `for y in ys` chạy (`t5`) ⇒ **chưa có iterator**.
**Đặc tả (VD-178 đảo quyết định VD-174 mục 6)**:
- `filter`, `map`, … trên `list`/`map`/`set` **giữ như hiện tại: trả collection NGAY** (eager).
- **`.lazy()`** trả **`Iterator[T]`** lười; **`range`** và **`fs.lines(p)`** mặc định **lười**.
- Iterator có **cùng bộ method**, cộng **`.to_list()`/`.to_map()`/`.to_set()`**; dùng trực tiếp trong **`for`**.
- Iterator **duyệt lại được**: mỗi lần duyệt chạy lại từ nguồn, **không** có “đã dùng hết”. Tài liệu ghi rõ **closure trong chuỗi lười có thể chạy lại**.
- **R20** (`enumerate`, `zip`, …) **không còn phải chờ R24**.
**Kiểu trả về chốt**:
| Method | Trả về |
|---|---|
| `filter(f)` / `map(f)` trên collection | **collection** (eager, như hiện tại) |
| `.lazy()` | `Iterator[T]` (lười) |
| `range`, `fs.lines(p)` | `Iterator[T]` (lười, mặc định) |
| `take(n)` trên iterator | `Iterator[T]` |
| `enumerate()` / `zip(other)` trên iterator | `Iterator[(int, T)]` / `Iterator[(T, U)]` |
| `to_list()` / `.to_map()` / `.to_set()` | collection cụ thể, **vật chất hoá** |
**Ca đỏ-trước**: `xs.filter(f).len()` chạy (eager, **không** cần `.to_list()`); `xs.lazy().filter(f).map(g).take(2).to_list()` ⇒ đúng; `for x in xs.lazy().map(g):` ⇒ đúng; `xs.lazy()` **không** in ra list được (phải `.to_list()` hoặc `for`); duyệt **hai lần** cùng một iterator ⇒ cùng kết quả (duyệt lại được); `zip` + `enumerate` đúng kiểu.
**Ảnh hưởng**: tyck method (kiểu iterator), interp/VM (đánh giá lười), `stdlib.rs` (thêm `lazy`/`to_*`), docs; **0 test hiện có**.
**Task**: **R24a** (3 h) kiểu iterator + đánh giá lười · **R24b** (2 h) `to_*` + `for` · **R24c** (1 h) `enumerate/zip`. **Phụ thuộc**: R5; **R20 không chờ R24**.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:308` (ST-178) — đảo: filter/map giữ collection; `.lazy()` trả `Iterator[T]`; duyệt lại được; R20 không chờ.

## R15 · Nối dòng + dấu phẩy cuối — **Pha 2**
**Hiện trạng**: gọi hàm nhiều dòng trong `()` **chạy** (`a7`: 3 dòng + dấu phẩy cuối); chuỗi `.method()` trên một dòng chạy (`d7`); nối dòng khi dòng mới bắt đầu `.`/`|`/`??`/`and`/`or` — **chưa xác minh** (riêng `|` đầu dòng trong script ⇒ VS0201, `d7`).
**Đặc tả**: tự nối trong `() [] {}`; nối khi dòng mới bắt đầu `.`, `|`, `??`, `and`, `or`; **dấu phẩy cuối** mọi nơi.
**Ca đỏ-trước**: `xs\n    .filter(...)\n    .len()`; `let x = a\n    ?? b`; `f(\n 1,\n 2,\n)`; `[1, 2,]`.
**Task**: **R15** (1,5 h). **Phụ thuộc**: —.

## R16 · `let` che tên; `var` không khai lại — **Pha 1**
**Hiện trạng**: `let x = 1; let x = "a"` ⇒ **VS0302** (`a8`) — cần **bỏ** cho `let`; `var x = 1; var x = 2` ⇒ **VS0302** (`a9`) — **đúng**.
**Đặc tả**: `let` **được che tên** trong cùng phạm vi; `var` **không** khai lại.
**Ca đỏ-trước**: `let x = 1; let x = "a"` chạy; `var x = 1; var x = 2` ⇒ lỗi.
**Task**: **R16** (1 h). **Phụ thuộc**: —.

## R17 · Điều kiện `if`/`while` chỉ `bool` + gợi ý — **Pha 1**
**Hiện trạng**: `if xs:` ⇒ **VS0412** “điều kiện `if`: cần `bool`, được `list[int]`” (`b1`) — **kiểu đúng**, **thiếu gợi ý** `xs.is_empty()`.
**Đặc tả**: chỉ `bool`; lỗi kèm **gợi ý** `xs.is_empty()` (với list/map/str).
**Ca đỏ-trước**: `if xs:` ⇒ VS0412 + `gợi ý: xs.is_empty()`; `if xs.is_empty():` chạy.
**Task**: **R17** (0,5 h). **Phụ thuộc**: R9 (dòng gợi ý).

## R18 · Pattern mở rộng + `while let` — **Pha 2**
**Hiện trạng**: literal + khoảng `1..=5` **chạy** (`b2`); tuple `(a, b)` **chạy** (`d3`); `as` ⇒ **VS0201** (`d4`); `while let` ⇒ **VS0201** (`d5`); record pattern `{name}` có trong test (`exhaustive.rs:151-163`).
**Đặc tả**: thêm record `User(name, age: a)`, `as`, **`while let`**; pattern đã có: literal, khoảng, tuple, record.
**Ca đỏ-trước**: `User(name, age: a)`; `v as x`; `while let v = o:`; `1..=5`.
**Task**: **R18a** (2 h) as + record pattern · **R18b** (1,5 h) while let. **Phụ thuộc**: R2.

## R19 · `0o755` · tràn i64 **panic** · bit method — **Pha 2 (R19a; R19b/R19c pha 1)**
**Hiện trạng**: `0o755` ⇒ **VS0201** (`b3`); tràn i64 runtime ⇒ **VS0501** “tràn i64 lúc chạy” (`p2`) — cần **panic**; `5.bit_or(3)` ⇒ **VS0104** (`b4`: lexer đọc `5.` là float) — **xung đột cú pháp** cần xử lý; bit method chưa có.
**Đặc tả**: `0o755`; tràn i64 **panic** (mã thoát **101** theo C1); bit là **method** `bit_or/bit_and/bit_xor/shl/shr`.
**Ca đỏ-trước**: `0o755` ⇒ 493; `i64::MAX+1` ⇒ panic 101; `5.bit_or(3)` ⇒ 7; `5 . bit_or(3)` phải như nhau (không phụ thuộc khoảng trắng).
**Task**: **R19a** (1,5 h) 0o · **R19b** (1 h) panic · **R19c** (1 h) bit method + lexer. **Phụ thuộc**: R7.
> **đã sửa**: `agent/JOURNAL/vs-ergo.md:354` (ST-178) — panic tràn i64 thoát **101** theo C1.


### R19c · Float literal chỉ khi có chữ số sau `.` — **Pha 1**
**Đặc tả (VD-173#2)**: float literal **chỉ** khi ngay sau `.` là **chữ số**; `5.bit_or(3)` = **int + gọi method**; `5.0` là float; `1.` và `1.e3` **vẫn bị cấm** (giữ `VS0104`).
**Ca đỏ-trước**: `5.bit_or(3) == 7`; `5.0` là float (`5.0 + 1` ⇒ `6.0`); `1.` ⇒ VS0104; `1.e3` ⇒ VS0104; `5 . bit_or(3)` như nhau.
**Ảnh hưởng**: lexer `lit.rs` (FLOAT), 1 khối test; **không** đổi `VS0104` cho trường hợp cấm.
**Task**: **R19c** (1 h) lexer + ca. **Phụ thuộc**: R7.

## R19d · Khoảng trắng quanh `.` — **Pha 2** (VD-193)
**Đặc tả**: khoảng trắng quanh `.` **không đổi nghĩa** — `5.bit_or(3)` và `5. bit_or(3)` là **một** biểu thức. Float literal **cần chữ số sau dấu chấm** (`5.0`); `5.` nếu hiện hợp lệ thì **đổi thành lỗi** gợi ý `5.0`. `vis fmt` **chuẩn hoá không cách sau `.`**.
**Ca đỏ-trước**: `5. bit_or(3)` ⇒ `7`; `5.` ⇒ lỗi + gợi ý `5.0`; `vis fmt` trên `5. bit_or(3)` ⇒ `5.bit_or(3)`.
**Phụ thuộc**: R19c (đã xong ở ST-190).

## R20 · `assert` · `use as` · `use {}` · list/range bổ sung — **Pha 2**
**Hiện trạng**: `assert true, "m"` ⇒ **VS0201** (`b5`); `use std.fs as f` ⇒ **VS0201** (`b6`); `use std.fs.{exists}` ⇒ **VS0201** (`d6`); `xs.enumerate()` ⇒ **VS0403** (`b7`); `(1..5).step(2)` ⇒ **VS0403** (`b8`).
**Đặc tả**: `assert cond, "thông điệp"`; `use std.fs as f`; `use std.fs.{read, write}`; `list` thêm `enumerate, zip, any, all, find, sort_by, is_empty`; range thêm `.step(n)`.
**Ca đỏ-trước**: 5 dạng trên chạy; `assert false, "m"` ⇒ panic/thoát 101.
**Task**: **R20a** (2 h) assert + use · **R20b** (1 h) list/range. **Phụ thuộc**: R5.

## R21 · Pipeline `|` chỉ shell/cmd (ghi thiết kế) — **Pha 2**
**Hiện trạng**: `1 | 2` trong script ⇒ **VS0201** (`d7`) — **đúng** hướng; `|>` là pipe biểu thức script (`parse/expr.rs:119`).
**Đặc tả**: `|` chỉ trong **chế độ shell** và **khối `cmd`**; code thường dùng chuỗi method; hai bên **dùng chung hàm std**; ghi vào **thiết kế** (Agent 0 sở hữu).
**Ca đỏ-trước**: `1 | 2` ⇒ lỗi + gợi ý; `x |> f` chạy.
**Task**: **R21** (0,5 h, chỉ tài liệu). **Phụ thuộc**: —.

## Kiểm thêm: generic `f[int](x)` vs chỉ số
**Hiện trạng**: `f[int](1)` **chạy** (`b9`) — parser phân biệt được; **không** xung đột với `xs[i]`.
**Ghi chú**: xung đột thật nằm ở **`5.bit_or(3)`** (`b4`: `5.` thành float) — thuộc **R19c**.

## Thứ tự thực thi (VD-172 bổ sung — **đã bị C3 của VD-178 thay**)
… → **R5** → **R11** → **R12** → **R13** → **R14** → **R15** → **R16** → **R17** → **R18** → **R7** → **R19** → **R10** → **R20** → **R21** (chỉ sửa tài liệu).
**Thứ tự hiện hành là C3** (xem mục “Thứ tự thực thi (A0 chốt — C3 của VD-178)” ở trên).
**Cheat sheet**: cập nhật theo **toàn bộ R1–R24** sau khi thực thi; **mọi ví dụ là doc-test** (VD-170 P0 mục 3).

---

## Bảng [SỬA] VD-178 đã áp (ST-178)
| [SỬA] VD-178 | Nội dung | đã sửa |
|---|---|---|
| C1 | Bảng mã thoát **0/1/2/101**; VS0502/VS0503 đổi kỳ vọng sang 101 (làm trong R9) | `agent/JOURNAL/vs-ergo.md:7` |
| R1 + R2 | Làm **chung một khối**; bỏ “xoá builtin `None`” riêng ở R1c; `None` là biến thể ở R2a; từ khoá `none` ⇒ **VS0203** + gợi ý + fmt | `agent/JOURNAL/vs-ergo.md:53` · `agent/JOURNAL/vs-ergo.md:62` |
| R3 | `if let`/`while let` nhận **pattern**; `if let v = o:` ⇒ lỗi + gợi ý `Some(v)`; một quy tắc, không dạng tắt | `agent/JOURNAL/vs-ergo.md:75` |
| R5 | `{ten: …}` trần ⇒ **VS0205**; method chỉ cho tham số đầu **kiểu cụ thể**; `str(x)` giữ là chuyển đổi | `agent/JOURNAL/vs-ergo.md:95` |
| R6 | Thêm ví dụ `p"C:\thu muc\"` **hợp lệ** | `agent/JOURNAL/vs-ergo.md:106` |
| R7 | Duyệt đổi **4 test** (run.rs:98,100,112; interp_v2.rs:77); thêm `-7 % 2 ⇒ 1`; đơn vị chia số ⇒ đơn vị, làm tròn xuống | `agent/JOURNAL/vs-ergo.md:114` |
| R9 | **VS0203** = từ khoá đã bỏ; **VS0204** = const cần kiểu khi không literal; thêm **VS0508**, **VS0707**; lỗi chạy có mã thoát **101** | `agent/JOURNAL/vs-ergo.md:130` |
| R10 | Cộng **cùng loại** hợp lệ; chỉ **khác loại** lỗi; in ghép/thập phân + `.fmt_iec()` | `agent/JOURNAL/vs-ergo.md:164` |
| R11 (c3) | Sửa chú thích probe `c3` (là **khai báo**, không phải “gọi kèm tiền tố”) | `agent/JOURNAL/vs-ergo.md:225` |
| R12 | `alias`/`newtype` ⇒ **VS0203** | `agent/JOURNAL/vs-ergo.md:254` |
| R14 | **Trùng tên ⇒ lỗi** (bỏ ưu tiên ngầm); `impl P:` chỉ trong cùng module khai báo `P` | `agent/JOURNAL/vs-ergo.md:268` |
| R19 | Panic tràn i64 thoát **101** theo C1 | `agent/JOURNAL/vs-ergo.md:354` |
| R22 | Bỏ “dùng `match` nếu cần” để ép ngược; ghi nợ sau 1.0 | `agent/JOURNAL/vs-ergo.md:289` |
| R23 | Ca đỏ-trước viết lại bằng `U(addr: A(city: "A"))` | `agent/JOURNAL/vs-ergo.md:300` |
| R24 | **Đảo**: `filter/map` giữ collection; `.lazy()` trả `Iterator[T]`; `range`/`fs.lines` lười; duyệt lại được; R20 không chờ R24 | `agent/JOURNAL/vs-ergo.md:308` |
| C3 | Thứ tự pha 1 mới (P0 → R9+C1 → R6 → R17 → R1+R2 → R3 → R8 → R4 → R5 → R11 → R12 → R16 → R7 → R19b → R19c) | `agent/JOURNAL/vs-ergo.md:175` |

> VD-225 (2026-11-19): ST-211 (VS3-5-pre) DONE — `ten_goc` (tách cả `/` và `\`) dùng ở mọi đường nghiêng r split.
