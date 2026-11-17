<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS-ERGO · Bộ quy tắc công thái học ViScript (R1–R10) — ST-172b
**Trạng thái**: **CHỜ AGENT 0 DUYỆT** (cùng đợt 6 tài liệu VS3) · **ST-172b** · **Chỉ đọc, 0 dòng mã**
**Căn cứ**: **VD-171** (thay mục 7, 8, 10, 11, 13 của VD-170; sửa VD-19/Q2/§1.6) + `vs-ergo-facts.md` (ST-172a, APPROVED) + probe tại HEAD `6d01d8d3`.
**Mỗi quy tắc**: Hiện trạng (probe + `file:dòng`) → Đặc tả → Ca đỏ-trước → Bảng chuyển đổi mã (nếu có) → Ảnh hưởng → Task ≤ 3 h → Phụ thuộc. **Không bàn lại hướng.**

## R1 · Chữ hoa
**Hiện trạng**: pattern nhận **cả `Some(v)` lẫn `some(v)`** (probe `c6`,`c8`); `None` là **builtin** in `<builtin None>` (probe `r2`); `lower.rs:53` có `None`; `true/false/none` viết thường.
**Đặc tả**: kiểu/trait/biến thể enum **UpperCamel** (`Some/None/Ok/Err/Circle`); biến/hàm/module/trường **snake_case**; hằng **UPPER_SNAKE**; `true/false` thường. **Dựng và pattern viết giống hệt nhau.** Pattern thường ⇒ **cảnh báo** + `vis fmt` **tự sửa**.
**Ca đỏ-trước**: `some(v)` ⇒ cảnh báo + fmt sửa `Some(v)`; `Some(1)`/`none`/`None` xanh; `let Some = 1` ⇒ lỗi.
**Chuyển đổi**: `some`/`none` pattern → `Some`/`None`; xoá `None` khỏi builtin (`lower.rs:53`), thêm **VS0111** (cảnh báo) + fmt.
**Ảnh hưởng**: `tyck/pat.rs`, `lower.rs:53`, fmt, `all_codes`, golden, docs.
**Task**: **172b-R1a** (1,5 h cảnh báo) · **R1b** (1 h fmt sửa) · **R1c** (1 h xoá `None`). **Phụ thuộc**: R9 (mã mới).

## R2 · Option/Result là enum std
**Hiện trạng**: `none` là **từ khoá** (`token.rs:185`); `T??` ⇒ **VS0201** (probe `y2`); `Some(1)` dựng được, `some(1)` ⇒ VS0501 (probe `c4`); tự gói `T→T?` **đã có** (probe `n1`).
**Đặc tả**: `enum Option[T]: Some(T), None`; `enum Result[T,E]: Ok(T), Err(E)`; `T?` = `Option[T]`; **bỏ từ khoá `none`** (`None` là biến thể); **cho phép `T??`**; tự gói **đủ số lớp**, `None` **lớp ngoài cùng**, lớp trong `Some(None)`.
**Ca đỏ-trước** (bắt buộc theo A0):
- `fn first[T](xs: list[T]) -> T?` gọi với `T = int?` ⇒ kiểu trả `int??`.
- `map[str, int?].get` phân biệt **không có khoá** ⇒ `None` vs **khoá = None** ⇒ `Some(None)` vs **có giá trị** ⇒ `Some(Some(v))`.
- `let x: int?? = 5` ⇒ `Some(Some(5))`; `let y: int?? = None` ⇒ `None`; `let z: int?? = Some(None)` ⇒ `Some(None)`.
**Chuyển đổi**: bỏ `none` keyword; `None` thành biến thể; sửa VD-19 (A0).
**Ảnh hưởng**: `ty.rs`, parser (`T?` lặp), tyck gói nhiều lớp, interp/VM, `lower.rs`, docs.
**Task**: **172b-R2a** (3 h) enum std + bỏ `none`; **R2b** (3 h) `T??` + gói nhiều lớp; **R2c** (2 h) ca `first`/`map.get`. **Phụ thuộc**: R1.

## R3 · Vắng mặt/lỗi: `?`, `??`, `if let`, `let … else`
**Hiện trạng** (probe mới, **khác** ST-172a):
- `??` **chạy**: `o ?? 9` ⇒ `5` (`p5`).
- **`if let` CÓ**: `if let v = o:` ⇒ `5` (`y6`); `if let Some(v) = o:` ⇒ **VS0412** “pattern ctor `Some` không khớp kiểu `int`” (`y4`) — pattern kiểm với **kiểu trong**, không phải `int?`.
- **`let … else:` CÓ**: `let x = r else:` + khối `return` ⇒ chạy (`y7`); `else 0` ⇒ **VS0201** (`y5`) — đúng ý A0: **không** có `f() else 0`.
**Đặc tả**: `?` đẩy lỗi lên; `??` giá trị mặc định; `if let` / `let … else` / `match` xử lý tại chỗ. **`let x = expr else e: <khối>`** — khối **bắt buộc thoát**; **Option không có `e`**; **không** có biểu thức `f() else 0`.
**Ca đỏ-trước**: `if let Some(v) = o:` chạy (sửa kiểm pattern theo `T?`); `let x = r else:` không thoát ⇒ lỗi rõ; `f() else 0` ⇒ VS0201.
**Ảnh hưởng**: `tyck` (pattern `if let`), parser `let else`, docs.
**Task**: **172b-R3a** (1,5 h sửa if-let pattern) · **R3b** (1 h ràng buộc khối thoát). **Phụ thuộc**: R2.

## R4 · Lỗi: `Error` record + `Json` enum + `AnyError`
**Hiện trạng**: probe `r4` (copy nguyên): `r4.vis:4:13: error VS0412: `?` trên `Result` chỉ dùng được khi hàm trả `Result[…, …]`, hàm này trả `str?` (§4.4)`; `json.decode(str)->V` (`API-STD.txt:15`); std trả `Result[_, str]`; `AnyError` chỉ là thiết kế (VD-20).
**Đặc tả**: mọi hàm std trả **`Result[T, Error]`** với `Error` là **record cài `trait Error { message, source, span }`** thay `str`; `?` tự chuyển `E: Error → AnyError` (VD-20); `json.decode → Result[Json, Error]`, `Json` là **enum** (`Null/Bool/Num/Str/Arr/Obj`), **bỏ `V`**; `main` có thể trả `Result[(), AnyError]` (in lỗi, **exit 1**).
**Ca chuẩn (2 dòng)**: `let s = fs.read(p)?` · `let v = json.decode(s)?`.
**Ca đỏ-trước**: hàm trả `Result[T, AnyError]` dùng `fs.read()?` + `json.decode()?` chạy; lỗi giữ `message/source/span`; `json.decode` trả `Json`; `main` trả `Err` ⇒ exit **1**.
**Bảng chuyển đổi**: `Result[_, str]` → `Result[_, Error]`; `V` → `Json`.
**Ảnh hưởng**: `tyck/expr.rs:177,194`, `ty.rs`, `stdlib.rs`, mọi test std, docs §4.4. **Phụ thuộc**: R2, R9.

## R5 · Gọi hàm: tên + record-theo-tên + method toàn std
**Hiện trạng**: `f(1, b: 5)` ⇒ **VS0201** (`n3`); record **chỉ** `{name: …}` (`exhaustive.rs:151`); `x.f(a)` mới một phần (`z1`: `str.trim`; `s.upper` chạy); `str(5)` chạy (`r3`).
**Đặc tả**: `f(1, b: 5)`; `--b` **chỉ** chế độ lệnh/`cmd`; `User(name: "An", age: 20)`, default **bỏ được**; `{…}` **chỉ map**; `x.f(a) ≡ mod.f(x, a)` khi **tham số đầu của `f` có kiểu của `x`** — **toàn bộ std**; báo **xung đột** `std.str`/`str`/builtin `str(x)`.
**Ca đỏ-trước**: `f(1, b: 5)` chạy; `f(b: 5, 1)` lỗi; `User(name: "a")` với `age: int = 0` ⇒ `age==0`; `{name: "a"}` ⇒ lỗi; `" abc ".trim()` ⇒ `abc`; `str(5)` vẫn `5`.
**Ảnh hưởng**: parser args/record, tyck call/method, `stdlib.rs`, docs, test record cũ.
**Task**: **172b-R5a** (2 h) · **R5b** (2 h) · **R5c** (2 h) · **R5d** (2,5 h). **Phụ thuộc**: R3, R4.

## R6 · Chuỗi
**Hiện trạng**: `p"C:\x"` ⇒ **VS0102** (`r1`); `"a"+"b"` ⇒ VS0412 (`t1`); `"{n:.2}"` ⇒ VS0201 (`t10`); `print("a","b")` ⇒ VS0401 (`t4`).
**Đặc tả**: `p"…"` **không escape**, **có** nội suy; `str+str` **nối**; `{expr:spec}` **runtime**; `print(a,b,…)` **cách 1 dấu cách**, `print()` ⇒ `\n`.
**Ca đỏ-trước**: `p"C:\x"` in `C:\x`; `p"a{b}"` nội suy; `"a"+"b"` ⇒ `ab`; `"{3.14159:.2}"` ⇒ `3.14`; `print("a","b")` ⇒ `a b`.
**Ảnh hưởng**: lexer `lit.rs`, `tyck/expr.rs:501,1303`, parser nội suy, interp, golden.
**Task**: **172b-R6a** (1,5 h) · **R6b** (1,5 h) · **R6c** (2,5 h) · **R6d** (1 h). **Phụ thuộc**: —.

## R7 · Số
**Hiện trạng**: `(Div, Int, Int) => Type::Int` (`crates/vis-types/src/tyck/num.rs:57`); runtime `builtin.rs:464`; **test dựa vào `int/int=int`**: `crates/vis-interp/tests/run.rs:98` (`7/2`→`3`), `:100` (`-7/2`→`-3`), `:112` (`5mb/5mb`→`1`), `interp_v2.rs:77` (`50pct/50pct`→`1`), `run.rs:120,429` (chia 0 ⇒ VS0502); `const A = 1` ⇒ VS0201 (`t5`); `1+1.5`⇒`2.5`, `let f: float = 1` (`p3`,`p4`).
**Đặc tả**: chỉ `int→float` chuyển ngầm; **`/` luôn `float`**; **`//`** chia lấy phần nguyên; **`%` khớp `//`**; `const` suy kiểu từ literal; sửa **Q2** (A0).
**Ca đỏ-trước**: `7/2` ⇒ `3.5`; `7 // 2` ⇒ `3`; `-7 // 2` ⇒ `-4`; `7 % 2` ⇒ `1`; `5mb/5mb` ⇒ `1.0`; `const A = 1` ⇒ `int`.
**Test phải sửa**: 4 ca trên (run.rs:98,100,112; interp_v2.rs:77) — **đổi kỳ vọng** theo ngữ nghĩa mới, **không** nới ngưỡng; chia 0 giữ.
**Ảnh hưởng**: `num.rs:57-65`, `builtin.rs:434,464`, parser `//`, test, docs Q2.
**Task**: **172b-R7a** (2 h) · **R7b** (1,5 h) · **R7c** (1 h). **Phụ thuộc**: —.

## R8 · Chỉ số (xác minh)
**Hiện trạng**: `xs[0]?` ⇒ **VS0412** “`?` chỉ dùng cho `T?`/`Result`, được `int`” (`x2`) ⇒ **CHƯA**; `xs.get(1)` ⇒ **`some(2)`** (`x3`) ⇒ **CÓ**; `xs[5]` ⇒ VS0503 (`b4`); `s[0]` ⇒ VS0412 gợi ý; `s[0:2]` ⇒ VS0201 (`b3`).
**Đặc tả**: giữ Q5/VD-18; chỉ **xác minh**.
**Ca đỏ-trước**: `xs[i]?` chạy; `xs.get(i)`; `xs[i]` ngoài phạm vi ⇒ VS0503.
**Task**: **172b-R8** (1 h). **Phụ thuộc**: R2.

## R9 · Thông báo lỗi: mỗi mã một nghĩa
**Hiện trạng**: `VS0412` phủ 4 nhóm (`tyck/mod.rs:1125`, `tyck/expr.rs:501`, `mod.rs:818,1067`); `VS0501` phủ tính năng + tràn i64 (`p2`) + `some(1)` (`t7`); `VS0201` gộp cú pháp; `VS0301` gộp tên/kiểu; `VS0604` gộp `&`/`^`/tiến trình; `VS0701` gộp module/std/pkg; nhiều thông điệp còn **“VS0”**.
**Đặc tả**: mỗi mã **một nghĩa**; lỗi cú pháp hay gặp có **dòng “gợi ý:”**; **bỏ “VS0”**; từ khoá chưa làm (`newtype`…) **báo lỗi tại chỗ dùng**.
**Bảng chuyển đổi đề xuất** (Agent 0 chốt số; `API-CODES.txt` A0 sở hữu):
| Mã cũ | Nghĩa mới | Mã mới |
|---|---|---|
| VS0412 | kiểu không khớp (chung) | **VS0412** |
| | gán ràng buộc bất biến | **VS0413** |
| | toán tử không hỗ trợ cho kiểu | **VS0414** |
| | `async` sai chỗ | **VS0415** |
| | range không `int` | **VS0416** |
| VS0501 | tính năng chưa hỗ trợ | **VS0501** |
| | tràn i64 lúc chạy | **VS0507** |
| VS0201 | cú pháp chung | **VS0201** |
| | `newtype` chưa hạ | **VS0203** |
| | `const` thiếu kiểu | **VS0204** |
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
**Gợi ý cho 5 mẫu VS0201**: `const A = 1` → “const cần kiểu”; `s[a:b]` → “str không cắt”; `U(n: 1)` → “dựng record bằng tên kiểu”; `"{x:.2}"` → “format spec chưa hỗ trợ”; `|x|` → “closure dùng `x => expr`”.
**Ca đỏ-trước**: mỗi ca mới đúng 1 mã; không thông điệp nào chứa “VS0”; `newtype` báo tại dòng khai.
**Ảnh hưởng**: `vis-diag`, mọi test so mã, `API-CODES.txt`, docs.
**Task**: **172b-R9a** (2 h) · **R9b** (1,5 h) · **R9c** (1 h). **Phụ thuộc**: —.

## R10 · Đơn vị
**Hiện trạng**: `150m` in `150m`; `1h+30m` in `90m`; `5kb+5kb` ⇒ `10kb`; **`1kb+1kib` ⇒ `2024b`** (`y1`) — **trộn đơn vị KHÔNG báo lỗi**; `50pct+10pct` ⇒ `60pct` (`p9`).
**Đặc tả**: in **chuẩn hoá** (`1h30m`, `1.5mb`); **xác minh** trộn đơn vị ⇒ theo A0 **báo lỗi** (nếu chốt vậy, đổi auto-convert sang VS0414/VS0412).
**Ca đỏ-trước**: `150m` ⇒ `2h30m`; `1500kb` ⇒ `1.5mb`; `1kb+1kib` ⇒ lỗi (hoặc ghi rõ nếu giữ auto-convert); `45s` ⇒ `45s`.
**Ảnh hưởng**: `Type::Duration/Size` Display, `tyck/num.rs`, golden, docs.
**Task**: **172b-R10a** (2 h) · **R10b** (1 h). **Phụ thuộc**: R7.

## Thứ tự thực thi (A0 chốt)
**P0 của VD-170** (`newtype` báo lỗi tại chỗ dùng, bỏ “VS0”, cheat sheet doc-test) → **R9** → **R6** → **R1 + R2** → **R3 + R4** → **R5** → **R7** → **R10**. Tất cả **TRƯỚC VS3**.

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

## R11 · Mọi hàm dựng là hàm
**Hiện trạng**: dựng record theo tên `U(n: 1)` ⇒ **VS0201** (`t9`); khai báo biến thể enum **mang dữ liệu** `Circle(float)` ⇒ **VS0201** (`c2`,`c3`); biến thể **đơn vị** gọi `Shape.Empty` **chạy** (`c1` in `Empty`); `Some(1)` prelude chạy.
**Đặc tả**: record, biến thể enum, kiểu bọc đều gọi **như hàm**: `Circle(2.0)`, `Circle(r: 2.0)`; **tên đứng sau vị trí**; cùng luật với `fn`.
**Ca đỏ-trước**: `type C: r: float` + `C(2.0)` và `C(r: 2.0)`; `enum S: Circle(float)` + `Circle(2.0)`; `C(2.0, r: 1.0)` ⇒ lỗi (vị trí sau tên).
**Ảnh hưởng**: parser (biến thể mang dữ liệu), tyck gọi, lower/interp, docs.
**Task**: **R11a** (2 h) enum payload + gọi · **R11b** (1,5 h) record/wrapper ctor. **Phụ thuộc**: R2.

## R12 · Gộp khai báo kiểu (`type X = T`, `type Email(str)`)
**Hiện trạng**: `type X = int` ⇒ **VS0201** (`a1`); `type Email(str)` ⇒ **VS0201** (`a2`); **`alias X = int` CHẠY** (`d1`); `newtype Met = float` **bị bỏ qua im lặng** (`n11`) rồi dùng kiểu ⇒ VS0301 (`n8`).
**Đặc tả**: **`type X = T`** thay `alias`; **`type Email(str)`** thay `newtype` (dựng `Email("…")`, mở bằng `let Email(s) = e`); **bỏ từ khoá `alias`, `newtype`**.
**Ca đỏ-trước**: `type X = int`; `type Email(str)` + `Email("a@b")` + `let Email(s) = e`; `alias`/`newtype` ⇒ lỗi (hoặc cảnh báo chuyển).
**Ảnh hưởng**: parser khai báo, tyck, lower, `KEYWORDS` (`token.rs:185`), docs.
**Task**: **R12a** (2,5 h) `type =` + wrapper. **Phụ thuộc**: R9 (mã cho `alias/newtype`).

## R13 · Biến thể enum: prelude + tiền tố khi trùng
**Hiện trạng**: `Shape.Empty` (dùng `.`) **chạy** (`c1`); `Some/None/Ok/Err` prelude chạy; biến thể không tiền tố khi duy nhất — **chưa xác minh**; trùng tên giữa 2 enum — **chưa có** (do R11a chưa xong).
**Đặc tả**: không cần tiền tố khi tên **duy nhất**; trùng tên ⇒ **bắt buộc `Shape.Empty`**, lỗi biên dịch **có gợi ý**; `Some/None/Ok/Err` **luôn trong prelude**.
**Ca đỏ-trước**: `enum A: Empty` + `enum B: Empty` + `Empty` ⇒ lỗi + gợi ý `A.Empty`; `Shape.Empty` chạy; `Empty` đơn nhất chạy.
**Task**: **R13** (1,5 h). **Phụ thuộc**: R11a.

## R14 · Method người dùng: `impl P:` + `self`/`var self`
**Hiện trạng**: `impl P:` + `fn len(self)` ⇒ **VS0302** “khai báo trùng `len`” rồi **VS0403** “không phân giải được phương thức” (`a5`); `fn f(var self)` ⇒ **VS0403** (`a6`); method khai trong thân `type` ⇒ **VS0201** (`d2`). ⇒ **method người dùng CHƯA có**; hiện chỉ method builtin (`str`, `list`, `map`).
**Đặc tả**: `impl P:` cho method riêng; `fn f(self)` **chỉ đọc**; `fn f(var self)` **sửa được**, **chỉ gọi trên biến `var`**; `Self` viết hoa.
**Ca đỏ-trước**: `p.tang()` trên `var p` chạy + đổi `x`; gọi trên `let p` ⇒ lỗi; `self.x = …` trong `fn f(self)` ⇒ lỗi.
**Ảnh hưởng**: parser `impl`, tyck method lookup, lower, docs.
**Task**: **R14a** (3 h) `impl` + `self` · **R14b** (2 h) `var self`. **Phụ thuộc**: R11.

## R15 · Nối dòng + dấu phẩy cuối
**Hiện trạng**: gọi hàm nhiều dòng trong `()` **chạy** (`a7`: 3 dòng + dấu phẩy cuối); chuỗi `.method()` trên một dòng chạy (`d7`); nối dòng khi dòng mới bắt đầu `.`/`|`/`??`/`and`/`or` — **chưa xác minh** (riêng `|` đầu dòng trong script ⇒ VS0201, `d7`).
**Đặc tả**: tự nối trong `() [] {}`; nối khi dòng mới bắt đầu `.`, `|`, `??`, `and`, `or`; **dấu phẩy cuối** mọi nơi.
**Ca đỏ-trước**: `xs\n    .filter(...)\n    .len()`; `let x = a\n    ?? b`; `f(\n 1,\n 2,\n)`; `[1, 2,]`.
**Task**: **R15** (1,5 h). **Phụ thuộc**: —.

## R16 · `let` che tên; `var` không khai lại
**Hiện trạng**: `let x = 1; let x = "a"` ⇒ **VS0302** (`a8`) — cần **bỏ** cho `let`; `var x = 1; var x = 2` ⇒ **VS0302** (`a9`) — **đúng**.
**Đặc tả**: `let` **được che tên** trong cùng phạm vi; `var` **không** khai lại.
**Ca đỏ-trước**: `let x = 1; let x = "a"` chạy; `var x = 1; var x = 2` ⇒ lỗi.
**Task**: **R16** (1 h). **Phụ thuộc**: —.

## R17 · Điều kiện `if`/`while` chỉ `bool` + gợi ý
**Hiện trạng**: `if xs:` ⇒ **VS0412** “điều kiện `if`: cần `bool`, được `list[int]`” (`b1`) — **kiểu đúng**, **thiếu gợi ý** `xs.is_empty()`.
**Đặc tả**: chỉ `bool`; lỗi kèm **gợi ý** `xs.is_empty()` (với list/map/str).
**Ca đỏ-trước**: `if xs:` ⇒ VS0412 + `gợi ý: xs.is_empty()`; `if xs.is_empty():` chạy.
**Task**: **R17** (0,5 h). **Phụ thuộc**: R9 (dòng gợi ý).

## R18 · Pattern mở rộng + `while let`
**Hiện trạng**: literal + khoảng `1..=5` **chạy** (`b2`); tuple `(a, b)` **chạy** (`d3`); `as` ⇒ **VS0201** (`d4`); `while let` ⇒ **VS0201** (`d5`); record pattern `{name}` có trong test (`exhaustive.rs:151-163`).
**Đặc tả**: thêm record `User(name, age: a)`, `as`, **`while let`**; pattern đã có: literal, khoảng, tuple, record.
**Ca đỏ-trước**: `User(name, age: a)`; `v as x`; `while let v = o:`; `1..=5`.
**Task**: **R18a** (2 h) as + record pattern · **R18b** (1,5 h) while let. **Phụ thuộc**: R2.

## R19 · `0o755` · tràn i64 **panic** · bit method
**Hiện trạng**: `0o755` ⇒ **VS0201** (`b3`); tràn i64 runtime ⇒ **VS0501** “tràn i64 lúc chạy” (`p2`) — cần **panic**; `5.bit_or(3)` ⇒ **VS0104** (`b4`: lexer đọc `5.` là float) — **xung đột cú pháp** cần xử lý; bit method chưa có.
**Đặc tả**: `0o755`; tràn i64 **panic** (mã thoát 101); bit là **method** `bit_or/bit_and/bit_xor/shl/shr`.
**Ca đỏ-trước**: `0o755` ⇒ 493; `i64::MAX+1` ⇒ panic 101; `5.bit_or(3)` ⇒ 7; `5 . bit_or(3)` phải như nhau (không phụ thuộc khoảng trắng).
**Task**: **R19a** (1,5 h) 0o · **R19b** (1 h) panic · **R19c** (1 h) bit method + lexer. **Phụ thuộc**: R7.

## R20 · `assert` · `use as` · `use {}` · list/range bổ sung
**Hiện trạng**: `assert true, "m"` ⇒ **VS0201** (`b5`); `use std.fs as f` ⇒ **VS0201** (`b6`); `use std.fs.{exists}` ⇒ **VS0201** (`d6`); `xs.enumerate()` ⇒ **VS0403** (`b7`); `(1..5).step(2)` ⇒ **VS0403** (`b8`).
**Đặc tả**: `assert cond, "thông điệp"`; `use std.fs as f`; `use std.fs.{read, write}`; `list` thêm `enumerate, zip, any, all, find, sort_by, is_empty`; range thêm `.step(n)`.
**Ca đỏ-trước**: 5 dạng trên chạy; `assert false, "m"` ⇒ panic/thoát 101.
**Task**: **R20a** (2 h) assert + use · **R20b** (1 h) list/range. **Phụ thuộc**: R5.

## R21 · Pipeline `|` chỉ shell/cmd (ghi thiết kế)
**Hiện trạng**: `1 | 2` trong script ⇒ **VS0201** (`d7`) — **đúng** hướng; `|>` là pipe biểu thức script (`parse/expr.rs:119`).
**Đặc tả**: `|` chỉ trong **chế độ shell** và **khối `cmd`**; code thường dùng chuỗi method; hai bên **dùng chung hàm std**; ghi vào **thiết kế** (Agent 0 sở hữu).
**Ca đỏ-trước**: `1 | 2` ⇒ lỗi + gợi ý; `x |> f` chạy.
**Task**: **R21** (0,5 h, chỉ tài liệu). **Phụ thuộc**: —.

## Kiểm thêm: generic `f[int](x)` vs chỉ số
**Hiện trạng**: `f[int](1)` **chạy** (`b9`) — parser phân biệt được; **không** xung đột với `xs[i]`.
**Ghi chú**: xung đột thật nằm ở **`5.bit_or(3)`** (`b4`: `5.` thành float) — thuộc **R19c**.

## Thứ tự thực thi (VD-172 bổ sung)
… → **R5** → **R11** → **R12** → **R13** → **R14** → **R15** → **R16** → **R17** → **R18** → **R7** → **R19** → **R10** → **R20** → **R21** (chỉ sửa tài liệu).
**Cheat sheet**: cập nhật theo **toàn bộ R1–R21** sau khi thực thi; **mọi ví dụ là doc-test** (VD-170 P0 mục 3).
