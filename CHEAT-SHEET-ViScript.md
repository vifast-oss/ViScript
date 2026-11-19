<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Cheat sheet — ViScript ↔ Python ↔ Rust ↔ TS/JS

> **Đây là tài liệu HỌC, không phải nguồn sự thật của dự án.**
> Sinh ngày **2026-11-16** (repo ViFastOs), **chép sang repo ViScript ngày 2026-11-17** (ST-178) và cập nhật theo
> **VD-178** (C1 mã thoát, R3 `if let` pattern, R10 đơn vị, R24 `.lazy()`). ✔/✖ giữ nguyên từ lần kiểm chứng
> `target/debug/vis` @ `f4005aed` (111 lượt chạy snippet); các mục **mới theo quyết định** được đánh nhãn trạng thái.
> Nguồn sự thật của ngôn ngữ: code + `test/sem/**`,
> [`docs/SPEC-lex.md`](docs/SPEC-lex.md), [`docs/ViScript-thiet-ke.md`](docs/ViScript-thiet-ke.md).
>
> **Doc-test (P0-3)**: ✔ = đã chạy thật, **tự động** bởi
> [`crates/vis-cli/tests/cheatsheet.rs`](crates/vis-cli/tests/cheatsheet.rs) (**11 ca** — `test/cheatsheet/*.vis`).

**Quy ước**

- ✔ = **đã chạy thật** đúng như ô ViScript trong lần kiểm chứng.
- ✖ = thiết kế có nhưng **chưa có tại `<commit>`**; cột ghi chú nêu cách thay thế.
- **[ĐÃ CÓ]** = chạy được ở HEAD hiện tại; **[ĐANG LÀM]** = đã có nhưng ngữ nghĩa sẽ đổi theo bộ quy tắc R1–R24;
  **[DỰ KIẾN]** = thiết kế đã chốt, **chưa** có trong mã (thuộc pha 1/pha 2 của `agent/JOURNAL/vs-ergo.md`).
- Cột Python/Rust/TS chỉ để **so cho quen mắt**, không phải cú pháp ViScript.
- Bảng "Mức hiện thực VS2" ở cuối tài liệu thiết kế là nơi ghi chênh lệch thiết kế ↔ code.

**Chạy thử**

```bash
vis run x.vis          # chạy (không cần build)
vis check x.vis        # kiểm kiểu + cú pháp, in mã lỗi VSxxxx
vis fmt x.vis          # định dạng, luôn 4 dấu cách
vis test x.vis         # chạy mọi `test fn`
vis explain VS0412     # giải thích mã lỗi
vis pkg add "vifast.org/vis/json@v1.2.3"   # gói + vis.lock (mô hình phi tập trung, VD-180)
```

**Mã thoát** (C1 của VD-178 — áp cho **mọi** chương trình):

| Mã | Nghĩa | Trạng thái |
|---|---|---|
| `0` | thành công | **[ĐÃ CÓ]** |
| `1` | `main` trả `Err` (lỗi dự kiến) | **[ĐÃ CÓ]** — từ **VD-241 §3/T5** (2026-11-19): `Result::Err` từ `main` ⇒ exit **1**; `Ok(..)` ⇒ exit **0**; hai engine đồng nhất |
| `2` | lỗi biên dịch/kiểm tra | **[ĐÃ CÓ]** |
| `101` | panic: lỗi lập trình (chia 0, index OOB, tràn i64, assert, kênh chết) | **[ĐÃ CÓ]** — `VS0502`/`VS0503`/`VS0505`/`VS0507` được trả **101** qua `vis` (R9); `[quyền]` ⇒ `2` |

**Từ khoá** (đủ, không thêm bớt):
`let var const fn cmd type enum trait impl pub use if elif else match for in while
loop break continue return and or not true false self async await scope test extern derives with has dyn ref`

> **R12a**: `alias`, `newtype` **bị bỏ** khỏi danh sách từ khoá; thay bằng `type X = T`,
> `type Email(str)` (dùng `alias`/`newtype` ⇒ **VS0203** + gợi ý). R2: từ khoá cũ đã bỏ — dùng `None` (viết hoa).

---

## 1. Khai báo & giá trị

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `print("hi")` ✔ | `print("hi")` | `println!("hi")` | `console.log("hi")` | **R6**: `print(a, b, …)` nhiều đối số, cách nhau **1 dấu cách**; `print()` ⇒ dòng trống |
| `let x = 1` ✔ | `x = 1` | `let x = 1;` | `const x = 1` | **bất biến** — gán lại ⇒ `VS0413` |
| `var x = 1`<br>`x += 1` ✔ | `x = 1` | `let mut x = 1;` | `let x = 1` | biến đổi được |
| `const N: int = 10` ✔ | — | `const N: i64 = 10;` | `const N = 10` | **bắt buộc ghi kiểu** (`const N = 10` ⇒ `VS0201`) |
| `let x: int = 1` ✔ | `x: int = 1` | `let x: i64 = 1;` | `const x: number = 1` | kiểu đặt sau `:` |
| `7 / 2` → `3.5`<br>`7 // 2` → `3`<br>`x //= 2` ✔ | `7 / 2` → `3.5`<br>`7 // 2` → `3`<br>`x //= 2` | `7 / 2` → `3` (int)<br>`7.0 / 2.0` → `3.5` | `7 / 2` → `3.5` | **R7**: `/` **luôn** `float`; `//` chia lấy phần nguyên (làm tròn **xuống**); `%` khớp `//` (`-7 % 2` = `1`); `//=` gán ghép |
| `let x = 1` → `let x = 2` (cùng phạm vi) ✔ | OK | OK | OK | **R16**: `let` **được che tên** trong cùng phạm vi; `var x = 1` → `var x = 2` ⇒ `VS0302` |
| `# chú thích` | `#` | `//` | `//` | `##` là doc-comment |
| `assert 1 + 1 == 2` ✔<br>`assert cond, "thông điệp"` ✔ | `assert ...` | `assert!(...)` | `console.assert(...)` | **R20a**: `assert cond, "msg"` (1–2 đối số); `assert false` ⇒ panic/thoát **101** |

## 2. Hàm & closure

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `fn add(a: int, b: int) -> int:`<br>`    a + b` ✔ | `def add(a, b):`<br>`    return a + b` | `fn add(a: i64, b: i64) -> i64 { a + b }` | `function add(a: number, b: number) { return a + b }` | **biểu thức cuối khối = giá trị trả về** |
| `fn f(a: int, b: int = 2) -> int:` ✔ | `def f(a, b=2)` | — | `function f(a, b = 2)` | tham số mặc định |
| `fn f(a: int, b: int) -> int:`<br>`f(1, b: 5)` ✔ | `def f(a, b=2)`<br>`f(1, b=5)` | — | — | **R5a**: đối số **có tên** `b: 5` khớp tham số theo **tên**; `--b` chỉ ở chế độ lệnh/`cmd` |
| `x => x * 2` ✔ | `lambda x: x * 2` | `\|x\| x * 2` | `x => x * 2` | |
| `fn(x: int) -> int:`<br>`    x + 1` ✔ | `lambda x: x + 1` | `\|x: i64\| x + 1` | `(x: number) => x + 1` | closure nhiều dòng |
| `fn id[T](x: T) -> T:` ✔ | `def id[T](x: T) -> T:` (3.12) | `fn id<T>(x: T) -> T` | `function id<T>(x: T): T` | generic dùng **`[]`**, không `<>` |
| `fn fact(n: int) -> int:`<br>`    if n <= 1: 1 else: n * fact(n - 1)` ✔ | đệ quy | đệ quy | đệ quy | `if` là **biểu thức** |
| `test fn cong():` ✔ | `def test_cong():` | `#[test] fn cong()` | `test("cong", ...)` | chạy bằng `vis test` → `ok 1 - cong` |

## 3. Điều khiển

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `if x > 0: ...`<br>`elif x < 0: ...`<br>`else: ...` ✔ | y hệt | `if x > 0 { } else if ...` | `if (x > 0) { } else if ...` | `elif` (không phải `else if`) |
| `let s = if ok: "y" else: "n"` ✔ | `"y" if ok else "n"` | `let s = if ok { "y" } else { "n" };` | `ok ? "y" : "n"` | `if` là biểu thức khi có `else`; **`ok` là tên ctor `Result`** — không dùng làm tên biến (ca test dùng `b`) |
| `for i in 0..3:` ✔ | `for i in range(3):` | `for i in 0..3 {` | `for (let i = 0; i < 3; i++)` | `..` **không** gồm cận trên |
| `for i in 0..=2:` ✔ | `for i in range(3):` | `for i in 0..=2 {` | như trên | `..=` gồm cận trên |
| `for x in [1, 2, 3]:` ✔ | `for x in [1,2,3]:` | `for x in xs {` | `for (const x of xs)` | |
| `for (k, v) in m:` ✔ | `for k, v in m.items():` | `for (k, v) in &m {` | `for (const [k,v] of Object.entries(m))` | |
| `while i < 3:`<br>`    i += 1` ✔ | y hệt | `while i < 3 { }` | `while (i < 3)` | |
| `loop:`<br>`    if done(): break` ✔ | `while True:` | `loop { if done() { break } }` | `while (true)` | `continue` cũng có ✔ |

## 4. `match` & pattern

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `match s:`<br>`    Circle(r): "tròn {r}"`<br>`    Empty: "rỗng"` ✔ | `match s:`<br>`    case Circle(r): ...` | `match s {`<br>`    Shape::Circle(r) => ...,`<br>`    Shape::Empty => ... }` | `switch (s.kind)` | **phải xét hết**; thiếu nhánh ⇒ `VS0402` ("`match` thiếu nhánh: B") |
| `Circle(r) if r > 100.0: "lớn"` ✔ | `case Circle(r) if r > 100:` | `Shape::Circle(r) if r > 100.0 => ...` | — | guard **không** tính vào coverage |
| `A(x) \| B(x): x` ✔ | `case A(x) \| B(x):` | `A(x) \| B(x) => x` | — | or-pattern; `\|` **không** phải bit-or |
| `_` | `case _:` | `_ =>` | `default:` | bắt buộc với kiểu vô hạn (`int`, `str`…) |
| `if let Some(v) = x:`<br>`    print(v)` ✔ | `if (v := x) is not None:` | `if let Some(v) = x {` | `if (x != null)` | có `else:` ✔; **R3a**: `if let`/`while let` nhận **pattern** (`Some(v)`/`Ok(v)`/`Err(e)`); dạng `if let v = x:` (luôn khớp) ⇒ **lỗi + gợi ý `Some(v)`** |
| `while let Some(v) = x:`<br>`    print(v)` ✔ | `while (v := next_it()) is not None:` | `while let Some(v) = iter.next() {` | `while (v = iter.next()) != null` | **R18b**: cấu trúc `while let` nhận **pattern** (`Some(v)`/`Ok(v)`/…); pattern không khớp ⇒ **thoát vòng**, không panic; codegen dùng `HCond::Let` chung với `if let` |


## 5. Dữ liệu

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `[1, 2, 3]` / `list[int]` ✔ | `[1,2,3]` / `list[int]` | `vec![1,2,3]` / `Vec<i64>` | `[1,2,3]` / `number[]` | |
| `xs[i]` (panic nếu ngoài) ✔<br>`xs[i]?` → `T?` ✔<br>`xs.get(i)` → `T?` ✔ | `xs[i]` (IndexError)<br>`xs[i] if i < len(xs) else None` | `xs[i]` (panic)<br>`xs.get(i)` → `Option<&T>` | `xs[i]` (undefined) | **R8**: `xs[i]?` chỉ số **có kiểm tra** (không panic); `xs[i]` ngoài phạm vi ⇒ `VS0503`; `xs[-1]` panic có thông điệp |
| `xs.enumerate()` ✔<br>`xs.any(f)` / `xs.all(f)` ✔<br>`xs.find(f)` → `T?` ✔ | `enumerate(xs)`<br>`any(f, xs)` / `all(f, xs)`<br>`next((x for x in xs if f(x)), None)` | `xs.iter().enumerate()`<br>`xs.iter().any(f)` / `.all(f)`<br>`xs.iter().find(\|x\| f(x))` | `xs.entries()`<br>`xs.some(f)` / `xs.every(f)`<br>`xs.find(f)` | **R20b**: `any`/`all`/`find` nhận hàm **1 tham số** trả `bool`; `enumerate` trả `list[tuple[int, T]]` |
| `{"a": 1, "b": 2}` / `map[str, int]` ✔ | `{"a": 1}` | `HashMap::from([("a",1)])` | `new Map([["a",1]])` | **VD-215** quy tắc `{…}` (xem bảng dưới) |
| `set{1, 2}` ✔ | `{1, 2}` | `HashSet::from([1,2])` | `new Set([1,2])` | `set{` **dính liền** |
| `(1, "a")` ✔ | `(1, "a")` | `(1, "a")` | `[1, "a"] as const` | `let (n, s) = t` ✔ |
| `type U:`<br>`    name: str`<br>`    age: int` ✔ | `@dataclass`<br>`class U:` | `struct U { name: String, age: i64 }` | `type U = { name: string; age: number }` | tên `type`, không `struct` |
| `let u: U = {name: "An", age: 20}` ✔ | `U(name="An", age=20)` | `U { name: "An".into(), age: 20 }` | `const u: U = {name: "An", age: 20}` | record literal + **phải ghi kiểu** |
| `u with age: 21` ✔ | `dataclasses.replace(u, age=21)` | `U { age: 21, ..u }` | `{...u, age: 21}` | trả bản ghi **mới** |
| `enum Shape:`<br>`    Circle(float), Empty` ✔ | `Enum` không mang dữ liệu | `enum Shape { Circle(f64), Empty }` | `type Shape = {kind:'circle', r:number} \| {kind:'empty'}` | **R11**: biến thể mang dữ liệu **vị trí** `Circle(float)` (hoặc có tên `Circle(r: float)`); nhiều biến thể **cùng dòng** ngăn `,` |
| `Circle(2.0)` ✔ | `Circle(2.0)` | `Shape::Circle(2.0)` | `{kind:'circle', r:2.0}` | dựng variant bằng `Tên(...)`; **R11**: `U(n: 1)` dựng bản ghi theo **tên kiểu** |
| `a == b` trên bản ghi ✔ | `a == b` | `a == b` | `a === b` | so theo giá trị |
| `type Email(str)` ✔ | `NewType('Email', str)` | `struct Email(String);` | branded type | **R12a**: newtype **trong suốt** — `Email("a@b")` dựng, `let Email(s) = e` mở; `newtype` ⇒ **VS0203** |
| `type Id = int` ✔ | `Id = int` | `type Id = i64;` | `type Id = number;` | **R12a**: `type X = T` thay `alias`; `alias` ⇒ **VS0203** + gợi ý |

**Quy tắc `{…}` (VD-215)** — chế độ literal quyết theo **toàn bộ** khoá, không chỉ khoá đầu:

| Viết | Nghĩa |
|---|---|
| `{ten: 1, tuoi: 2}` | **bản ghi** (mọi khoá phải là định danh trần) |
| `{"ten": 1}` | **map** |
| `{(k): 1}`, `{(k + "_id"): 1}` — `(expr)` là **cú pháp khoá tính toán chính thức** | **map** |
| trộn chuỗi và `(expr)` | **map** hợp lệ |
| trộn định danh trần với chuỗi hoặc `(expr)`, **bất kể thứ tự** ⇒ **VS0207** | bản ghi không tồn tại kiểu này |
| `{[k]: 1}` (bẫy JavaScript — khoá là danh sách) | **map** + **cảnh báo `VS0208`**; tắt bằng `{([k]): 1}` |
| `{}` | như hiện trạng hiện tại (map rỗng) |

## 6. Chuỗi & literal

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `"Xin chào {name}"` ✔ | `f"Xin chào {name}"` | `format!("Xin chào {name}")` | `` `Xin chào ${name}` `` | nội suy **ngay trong `"..."`**, không cần tiền tố |
| `"{{ok}}"` ✔ | `"{{}}"` | `"{{}}"` | — | `{{` `}}` cho ra ngoặc thật; in `{ok}` |
| `r"C:\x"` ✔ | `r"C:\x"` | `r"C:\x"` | `String.raw` | thô, **không** escape, không nội suy |
| `"""a`<br>`b"""` ✔ | `"""a`<br>`b"""` | `"a\nb"` | template literal | nhiều dòng |
| `b"\x41"` ✔ | `b"\x41"` | `b"\x41"` | `Uint8Array` | chỉ ASCII + `\xNN` |
| `p"./x"` ✔ | `Path("./x")` | `Path::new("./x")` | — | literal đường dẫn, kiểu `path` |
| `"a,b".split(",")` ✔ | `"a,b".split(",")` | `"a,b".split(',')` | `"a,b".split(",")` | |
| `"ab".upper()` / `.lower()` / `.chars()` / `.contains("b")` / `.len()` ✔ | `.upper()` / `list(s)` / `"b" in s` / `len(s)` | `.to_uppercase()` / `.chars()` / `.contains('b')` / `.len()` | `.toUpperCase()` / `[...s]` / `.includes("b")` | **đúng 6** method của `str` |
| `"a" + "b"` ✔ | `"a" + "b"` | `format!` / `String + &str` | `"a" + "b"` | **R6**: `str+str` **nối** ⇒ `ab` |
| `"{n:.2}"` ✔ | `f"{n:.2f}"` | `format!("{n:.2}")` | `n.toFixed(2)` | **R6**: `{expr:spec}` chạy ở runtime (`.N` = N chữ số thập phân) |

**Literal đặc biệt** (không ngôn ngữ nào trong 3 cái kia có):

| Viết | Kiểu | Ghi chú |
|---|---|---|
| `1_000`, `0xff`, `0b1010` ✔ | `int` (i64) | tràn i64 là **lỗi biên dịch** |
| `3.14`, `1e-9` ✔ | `float` (f64) | `1.` **sai** (bắt buộc có số sau dấu chấm) |
| `5mb`, `5kib` ✔ | `size` | `kb`=1000, `kib`=1024; **R10 [DỰ KIẾN]**: in `1.5mb` (thập phân, đơn vị lớn nhất có phần nguyên ≥ 1, ≤ 2 chữ số), `.fmt_iec()` cho `kib`/`mib` |
| `250ms`, `2h30m` ✔ | `duration` | hiện in `150m`; **R10 [DỰ KIẾN]**: in dạng **ghép** `2h30m`; cộng **cùng loại** hợp lệ (`1h+30m`), **khác loại** lỗi |
| `50pct` ✔ | `percent` | `%` **chỉ** là modulo; **R10 [DỰ KIẾN]**: cộng cùng loại hợp lệ, `pct + size` lỗi |

## 7. Optional & xử lý lỗi

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `int?` / `None` ✔ | `int \| None` / `None` | `Option<i64>` / `None` | `number \| undefined` | không có `null` toàn nơi; **R2**: từ khoá cũ đã bỏ ⇒ **VS0203** — dùng `None` |
| `x ?? 7` ✔ | `x if x is not None else 7` | `x.unwrap_or(7)` | `x ?? 7` | |
| `u?.age` ✔ | — | `u.map(\|u\| u.age)` | `u?.age` | |
| `Some(5)` (dựng) ✔<br>`Some(v)` (pattern) ✔ | — | `Some(5)` / `Some(v) =>` | — | **R2/VD-212**: dựng **và** pattern đều viết hoa (`Some(v)`, `None`); dạng viết thường trong pattern ⇒ **lỗi VS0203** + gợi ý dạng đúng; `vis fmt` tự sửa trước khi biên dịch |
| `Result[int, str]`, `Ok(7)`, `Err("hỏng")` ✔ | exception | `Result<i64, String>` | `throw` / union | |
| `let v = f()?` ✔ | `v = f()` (try/except) | `let v = f()?;` | `try { f() }` | |
| `let v = f() else e:`<br>`    print(e)`<br>`    0` ✔ | `try/except` | `match f() { ... }` | `catch` | **R3b**: `Ok`/`Some` bóc giá trị; `Err`/`None` chạy khối `else` (nhận lỗi nếu có `else e:`); khối `else` phải thoát |
| `assert cond` ✔ | `assert` | `assert!(...)` | `console.assert` | |
| `panic("hỏng")` ✖ | `raise` | `panic!` | `throw` | chưa có ở VS0 |
| `AnyError` ✔ | — | `Box<dyn Error>` | `unknown` | alias `Error` (VD-231) — dùng được; sai kiểu báo `VS0412` |

## 8. Trait, `impl`, `derives`

| ViScript | Python | Rust | TS/JS |
|---|---|---|---|
| `trait Hien:`<br>`    fn hien(self) -> str` ✔ | `class P(Hien):` / `Protocol` | `trait Hien { fn hien(&self) -> String; }` | `interface Hien { hien(): string }` |
| `impl Hien for P:`<br>`    fn hien(self) -> str: ...` ✔ | kế thừa | `impl Hien for P { ... }` | `class P implements Hien` |
| `type P derives Show:` ✔ | — | `#[derive(Debug)]` | — |
| `dyn Hien` ✖ | — | `dyn Trait` | — |

`derives` sinh sẵn: `Eq`, `Show`, `Json` — không cần viết tay (Ord/Hash/Default chưa có).

## 9. Thư viện chuẩn (đóng băng — `agent/API-STD.txt`)

Gọi dạng **module.chấm_hàm**, sau khi `use std.<module>`:

| Module | Hàm chính | Ví dụ đã chạy ✔ |
|---|---|---|
| `std.fs` | `exists` `list` `read` `write` | `fs.exists(".")` → `true` |
| `std.list` | `sort` `take` `drop` `chunk` `concat` `reverse` `unique` | `list.sort([3,1,2])` → `[1,2,3]` |
| `std.map` | `keys` `values` `merge` `remove` `get_or` | `map.keys(m)` |
| `std.str` | `split` `join` `upper` `lower` `trim` `replace` `repeat` `starts_with` `ends_with` `contains` | `str.join(["a","b"], "-")` → `a-b` |
| `std.path` | `join` `basename` `dirname` `ext` `stem` `is_abs` | `path.join("a","b")` → `a/b` |
| `std.time` | `now_ms()` `monotonic_ms()` | |
| `std.json` | `encode` `decode` `encode_pretty` | `json.decode("[1,2]")` → `Ok(Arr([Num(1), Num(2)]))` (enum dựng sẵn `Json`: `Null Bool Num Str Arr Obj`; lỗi std → bản ghi `Error { message, source: Error?, span: str? }`) |
| `std.net` | `listen` `connect` `read` `write` `accept` `close` | loopback, **không TLS** |
| `std.http` | `get` `head` + `_timeout` | HTTP/1.1 loopback, chunked, redirect GET ≤5 |

Method trên giá trị (không cần `use`):

| Kiểu | Method ✔ |
|---|---|
| `list` | `len` `get` `first` `last` `push` `map` `filter` `each` `sum` `join` — `map`/`filter` **trả list ngay** (eager) **[ĐÃ CÓ]** |
| `Iterator[T]` | `.lazy()` trả iterator; cùng bộ method + `.to_list()`/`.to_map()`/`.to_set()`; **duyệt lại được**; `range`/`fs.lines` mặc định lười — **[DỰ KIẾN]** (R24) |
| `map` | `len` `get` `has` `keys` `values` |
| `str` | `len` `upper` `lower` `split` `chars` `contains` |

## 10. Chế độ shell (chỉ ViScript có)

| Viết | Ghi chú |
|---|---|
| `ls ./photos \| where size > 5mb \| sort_by size --desc \| take 10` | pipeline trong `.vish` / ViShell |
| `pub cmd ls(path: path = p".") -> stream[FileEntry]:` | `cmd` = hàm + cờ `--x`; tự sinh `--help` **[DỰ KIẾN]** — chưa có ở HEAD (P0-3) |
| `cmd { ls (dir) \| where size > 5mb }` | dùng pipeline ngay trong `.vis` |
| `extern cmd git(sub: GitSub, --oneline: bool, ...rest: str)` | khai chữ ký cho chương trình ngoài |
| `^ten` | ép gọi chương trình **ngoài** |
| `\| complete` | chặn lỗi pipeline, trả `{stdout, stderr, code, how}` |

## 11. Những thứ **không có** (và thay bằng gì)

| Không có | Thay bằng | Python | Rust | TS/JS |
|---|---|---|---|---|
| `class`, kế thừa | `type` + `impl` | `class` | `struct` + `impl` | `class` |
| `try/catch`, exception | `Result` + `?` + `else err:` | `try/except` | `Result` | `try/catch` |
| `null` / `undefined` | `None` + `T?` (**R2** xong) | `None` | `Option` | `null/undefined` |
| `switch` | `match` | `match` | `match` | `switch` |
| `any` | `Json` + chuyển đổi tường minh | — | — | `any` |
| `1 < x < 10` | `1 < x and x < 10` | **hợp lệ** | lỗi | lỗi |
| `s[i]`, `s[1:3]` | `.chars()`, `xs.get(i)` | **hợp lệ** | không có | **hợp lệ** |
| `"a" + "b"` | **hợp lệ** (R6) | **hợp lệ** | **hợp lệ** | **hợp lệ** |
| `1.` (float) | `1.0` | **hợp lệ** | **hợp lệ** | **hợp lệ** |
| `++`, `--` | `x += 1` | — | — | **có** |
| macro (`println!`) | `print(...)` | — | **có** | — |
| `unsafe`, con trỏ, borrow | ngữ nghĩa giá trị + `ref[T]` | — | **có** | — |

## 12. Mười cái bẫy cho người mới

1. **`let` là bất biến** — ngược với `let` của JS. Muốn gán lại: `var`.
2. **`var` không phải `var` của JS** (không function-scope) — nó chỉ là "biến đổi được", giống `let mut`.
3. **`"a" + "b"` nối được** (R6) — nhưng chỉ `str + str`; `"a" + 1` vẫn lỗi (`VS0414`).
4. **`match` phải xét hết**; nhánh có `if` (guard) **không** được tính là đã xét.
5. **`Some`/`None`/`Ok`/`Err` viết hoa** (cả dựng và pattern) — **R1a/R2/VD-212**: dạng viết thường trong pattern ⇒ **lỗi `VS0203`** (khác `VS0111` cảnh báo của R1a — nay là lỗi); `vis fmt` tự sửa; từ khoá cũ ⇒ **VS0203**.
6. **Record literal phải liệt kê đủ trường** — giá trị mặc định của `type` **không** tự áp:
   `type U: email: str? = None` thì vẫn phải viết `{name: …, email: None}`.
7. **`const` phải ghi kiểu**: `const N: int = 10`.
8. **`?` chỉ dùng cho `T?`/`Result`** — **R8**: `xs[i]?` là **ngoại lệ** (chỉ số có kiểm tra, trả `T?`); `xs[i]` ngoài phạm vi vẫn panic `VS0503`; chỉ số âm dùng `.last()` / `.get(i)`.
9. **Biến thể enum viết hoa chữ đầu** — **R1a/VD-212**: `Some`/`None`/`Ok`/`Err`; viết hoa chữ thường trong **pattern** ⇒ **lỗi `VS0203`** kèm gợi ý dạng đúng; `vis fmt` tự sửa trước khi biên dịch.
10. **`print` nhận nhiều đối số** (R6), cách nhau 1 dấu cách — giống Python; `print()` in dòng trống.
11. **Thụt lề**: đơn vị thụt lề do **cấp đầu tiên trong tệp** quyết định; trộn 2 và 4 dấu cách là lỗi; tab báo `VS0107` (vẫn lex tiếp để `vis fmt` sửa).

## 13. Phụ lục — cách kiểm chứng & chênh lệch đã biết

- **Cách kiểm**: mỗi ô ✔ là một snippet độc lập chạy `vis run`; ✖ là snippet trả mã lỗi, đã đọc thông điệp.
  Tổng **111 lượt chạy** tại thời điểm 2026-11-16 (`vis test` tại HEAD hiện tại — xem `test/cheatsheet/`).
- **Doc-test (P0-3)**: `cargo test -p vis-cli --test cheatsheet` chạy **11 ca** `test/cheatsheet/*.vis`
  (mỗi ca = một chương trình hoàn chỉnh ghép từ ô ✔; so **stdout** nếu tệp có dòng `# kỳ vọng:`).
- **Chênh lệch thiết kế ↔ code đã gặp** (theo thứ tự nguồn sự thật, **code thắng**):
  `User(name: "An")` (thiết kế §3.2) **chưa có** → dùng `{...}` + `let u: U = …`;
  `AnyError` **nay là alias bản ghi `Error` (VD-231)** — dùng được; sai kiểu báo `VS0412`; format spec `{x:.2}` **đã có** (R6);
  `dyn` **chưa dùng được**; `panic(...)` **chưa có**; `newtype` báo **VS0203** tại dòng khai (ST-179);
  `r"…"`/`b"…"`/`p"…"` **đã bóc tiền tố** (R6a: `print(p"C:\x")` in `C:\x`);
  `async` chạy **tuần tự một luồng** (VD-240, 2026-11-19: giữ một luồng, **không** đa luồng; test grid `test/sem/async/grid-*` pin): spawn **enqueue** task, task thực thi run-to-completion **khi await**; task được await chạy theo thứ tự **await**; task **không** await vẫn được chạy (khi kênh chặn hoặc ở join cuối) theo thứ tự tạo; **kênh FIFO**; không work-stealing — `recv` hàng trống, `send` đầy hàng, `chan(0)` đều bị chặn (`VS0505`); **hai engine byte-identical**; interp = VM.
  **Chan trả một giá trị `<kênh>` duy nhất** (không phải tuple `(tx, rx)` như thiết kế §742 tip cũ); dùng `c.send/c.recv`, không destructure; show `<fn>` đồng nhất cho cả hai engine.
- **Theo VD-178 (ST-178) — nhãn trạng thái**:
  - **[ĐÃ CÓ]** `filter`/`map` trả collection ngay; `if let Some(v) = o:` (R3a); `let x = r else:` (R3b); `while let PAT = EXPR:` (**R18b** — cấu trúc vòng lặp `if let`-style với pattern, thoát khi `None`); đơn vị `5mb`/`2h30m` chạy; `p"…"` **thô hoàn toàn** + nội suy (R6a); `str+str` nối (R6b); `print(a, b)` + `print()` (R6d); `{expr:.N}` (R6c); mã thoát **101** cho panic (C1/R9); `assert cond` (không có thông điệp). **VD-222/225/227**: `json.decode(s) → Result[Json, Error]` ở interp và VM (JSON enum thật: `Null/Bool/Num/Str/Arr/Obj`); `Error` bản ghi dựng sẵn `{message, source: Error?, span: str?}`; `print(e)` in chỉ `{message}`; `AnyError = alias bản ghi `Error` (RecId(0))** (VD-231; `print(e)` in chỉ `{message}`); interp **và** VM tự nâng `T → T?` ở `let x: int? = …` (VD-227) **và** ở biên `fn f() -> T?` (VD-228 — `None`/`Some` không bọc đôi; argument `g(42)` + closure-biên (`fn(x) -> T?`) đều auto-lift (VD-229)).
  - **[ĐANG LÀM]** đơn vị sẽ **in chuẩn hoá** (R10); pattern viết thường đã **nâng** `VS0111` (R1a) ⇒ **lỗi `VS0203`** (VD-212); từ khoá cũ (R2) đã bỏ — **VS0203**.
  - **[ĐÃ CÓ]** `main` trả `Err` ⇒ exit **1**; `assert cond, "msg"` (R20a); `if let`/`while let`. **VD-241 đợt coercion Option (2026-11-19)**: `T?? = Option[Option[int]]` lồng thật ở **runtime**; `None` trần ở mọi biên (let/var/gán lại/đối số/thân hàm/return) giữ **`None` lớp ngoài**; `Some(x)` nâng đúng một lớp; spelled-out `Option[Option[int]]` ≡ sugar `int??` ≡ alias — cả ba cùng kiểu/runtime (`optional/long-option-matrix.vis`).
  - **[DỰ KIẾN]** `.lazy()` + `Iterator[T]` duyệt lại (R24). **Lưu ý còn treo**: interp ↔ VM — interp auto-lift `let x: int? = expr` (VD-227) và `fn f() -> T?` (VD-228 Option B) đều auto-lift; closure-boundary (`fn(x) -> T?`) và argument `g(42)` auto-lift từ **VD-229**; lambda params không annot ⇒ không lift; `chan(0)` vẫn bị chặn `VS0505`.
- **Nguồn đọc thêm**: `test/sem/**` (525 ca ngữ nghĩa chạy thật: 487 + 27 ca `sqlite/` VD-241 Gate 1 + 11 ca `sqlite/` VD-241 pre-merge),
  [`docs/SPEC-lex.md`](docs/SPEC-lex.md) (từ vựng + EBNF),
  mục "Mức hiện thực VS2" cuối [`docs/ViScript-thiet-ke.md`](docs/ViScript-thiet-ke.md).
