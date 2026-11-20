<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Cheat sheet — ViScript ↔ Python ↔ Rust ↔ TS/JS

> **Đây là tài liệu HỌC, không phải nguồn sự thật của dự án.**
> Sinh ngày **2026-11-16** (repo ViFastOs), **chép sang repo ViScript ngày 2026-11-17** (ST-178); cập nhật
> **2026-11-21** (VD-270: CLI/LSP/`std.*` mới, API-STD cập nhật). Các ô ✔ cũ giữ nguyên từ lần kiểm chứng
> `target/debug/vis` @ `f4005aed` (111 lượt chạy snippet); mục thêm sau được khoá bởi test Rust tương ứng.
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
vis run x.vis                    # chạy (không cần build)
vis run --stdin --time < x.vis   # script từ stdin + thời gian chạy (stderr)
vis check x.vis                  # kiểm kiểu + cú pháp, in mã lỗi VSxxxx
vis check --stdin --json < x.vis
vis fmt x.vis                    # in bản đã định dạng, luôn 4 dấu cách
vis fmt --check --stat x.vis     # CI: <N> tệp, <M> cần sửa, <K> lỗi
vis fmt --stdin --mode command < x.vish
vis test --filter cong --fail-fast x.vis
vis test --repeat 3 --shuffle --seed 7 x.vis
vis explain VS0412               # giải thích mã lỗi
vis explain --list --json        # toàn bộ sổ mã, máy đọc
vis pkg add "vifast.org/vis/json@v1.2.3"   # gói + vis.lock (mô hình phi tập trung, VD-180)
```

**Mã thoát** (C1 của VD-178 — áp cho **mọi** chương trình):

| Mã | Nghĩa | Trạng thái |
|---|---|---|
| `0` | thành công | **[ĐÃ CÓ]** |
| `1` | `main` trả `Err` (lỗi dự kiến) | **[ĐÃ CÓ]** — từ **VD-241 §3/T5** (2026-11-19): `Result::Err` từ `main` ⇒ exit **1**; `Ok(..)` ⇒ exit **0**; hai engine đồng nhất |
| `2` | lỗi biên dịch/kiểm tra | **[ĐÃ CÓ]** |
| `101` | panic: lỗi lập trình (chia 0, index OOB, tràn i64, `assert`, `panic(...)`, kênh chết) | **[ĐÃ CÓ]** — `VS0502`/`VS0503`/`VS0505`/`VS0507` được trả **101** qua `vis` (R9); `[quyền]` ⇒ `2` |

> **`main -> int` không phải mã thoát**: `fn main() -> int` trả `3` vẫn exit **0** (VD-242 §7);
> chỉ `Result::Err` từ `main` ⇒ **1**, `Ok`/giá trị thường ⇒ **0**.

**Từ khoá** (lexer có 40 mục, xếp ABC; `alias`/`newtype` bị **từ chối** theo R12a và **một từ khoá cũ** bị từ chối theo R2 — xem `vis explain VS0203`):
`alias and async await break cmd const continue derives dyn elif else enum extern false fn for
has if impl in let loop match newtype not or pub ref return scope self trait true type use
var while with`

> **R12a**: `alias`/`newtype` vẫn là từ khoá nhưng bị từ chối khi dùng — báo **VS0203** + gợi ý
> `type X = T`, `type Email(str)`. R2: từ khoá cũ bị từ chối — dùng `None` (viết hoa). `test` và
> `assert` là **soft keyword** theo ngữ cảnh, không nằm trong bảng từ khoá của lexer.

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
| `[first, ..rest]` ✔<br>`[first, ..]` ✔<br>`[..]` / `[..rest]` ✔ | `case [first, *rest]:` | `[first, ..]` | — | **VD-260**: rest-pattern list; `[first, ..]`/`[first, ..rest]` cần **≥ 1** phần tử, `[..]`/`[..rest]` khớp **mọi** list kể cả rỗng; `..` phải ở cuối (hậu tố `[.., z]` ⇒ `VS0201`); `rest` là bản sao; không phải list/thiếu phần tử ⇒ không khớp, không trap |
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
| `User(name: "An")` ✔ | `User(name="An")` | `U { name: "An".into(), .. }` | `new User({name: "An"})` | dựng bản ghi theo **tên kiểu** + trường có tên (R5a/R11); **không** nhận đối số vị trí |
| `u with age: 21` ✔ | `dataclasses.replace(u, age=21)` | `U { age: 21, ..u }` | `{...u, age: 21}` | trả bản ghi **mới** |
| `var a: U = …`<br>`a.age = 21` ✔<br>`a.bs[0].name = "B"` ✔ | `a.age = 21` | `a.age = 21` | `a.age = 21` | **VD-268**: gán lồng `a.b.f`/`xs[i].f`/`m["k"].f` trên gốc `var`; gốc `let`/tham số ⇒ `VS0412` |
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
| `"ab".upper()` / `.lower()` / `.chars()` / `.contains("b")` / `.len()` ✔ | `.upper()` / `list(s)` / `"b" in s` / `len(s)` | `.to_uppercase()` / `.chars()` / `.contains('b')` / `.len()` | `.toUpperCase()` / `[...s]` / `.includes("b")` | **R5c**: `s.f(args)` ≡ `str.f(s, args)` cho mọi hàm `std.str` nhận `str` đầu; xem bảng phương thức §9 |
| `"a" + "b"` ✔ | `"a" + "b"` | `format!` / `String + &str` | `"a" + "b"` | **R6**: `str+str` **nối** ⇒ `ab` |
| `"{n:.2}"` ✔ | `f"{n:.2f}"` | `format!("{n:.2}")` | `n.toFixed(2)` | **R6**: `{expr:spec}` chạy ở runtime (`.N` = N chữ số thập phân) |

**Literal đặc biệt** (không ngôn ngữ nào trong 3 cái kia có):

| Viết | Kiểu | Ghi chú |
|---|---|---|
| `1_000`, `0xff`, `0b1010` ✔ | `int` (i64) | tràn i64 là **lỗi biên dịch** |
| `-9223372036854775808` / `-0x8000000000000000` ✔ | `int` (i64) | **VD-266**: `i64::MIN` viết trực tiếp hợp lệ (gộp dấu `-`); dương `9223372036854775808` ⇒ **VS0104**; `i64::MIN // -1` / `% -1` ⇒ trap **VS0507** (exit 101) |
| `3.14`, `1e-9` ✔ | `float` (f64) | `1.` **sai** (bắt buộc có số sau dấu chấm) |
| `5mb`, `5kib` ✔ | `size` | `kb`=1000, `kib`=1024 (chỉ thang **literal**); **R10**: in thập phân `b kb mb gb tb`, đơn vị lớn nhất có phần nguyên ≥ 1, tối đa **2** chữ số lẻ (**làm tròn nửa lên**, bỏ 0 thừa) — `1024` ⇒ `1.02kb`, `1500` ⇒ `1.5kb`; âm giữ dấu, `0` ⇒ `0b` |
| `250ms`, `2h30m` ✔ | `duration` | **R10**: in **ghép** `w d h m s ms us ns` từ lớn đến nhỏ, bỏ thành phần 0, không cách — `150m` ⇒ `2h30m`, `90s` ⇒ `1m30s`, `150ms` ⇒ `150ms`; `0` ⇒ `0ns`; cộng **cùng loại** hợp lệ (`1h+30m`), **khác loại** lỗi |
| `50pct` ✔ | `percent` | `%` **chỉ** là modulo; cộng **cùng loại** hợp lệ (`50pct + 10pct` ⇒ `60pct`), `pct + size` ⇒ **VS0414**; in `150pct` |

## 7. Optional & xử lý lỗi

| ViScript | Python | Rust | TS/JS | Ghi chú |
|---|---|---|---|---|
| `int?` / `None` ✔ | `int \| None` / `None` | `Option<i64>` / `None` | `number \| undefined` | không có `null` toàn nơi; **R2**: từ khoá cũ đã bỏ ⇒ **VS0203** — dùng `None` |
| `x ?? 7` ✔ | `x if x is not None else 7` | `x.unwrap_or(7)` | `x ?? 7` | |
| `u?.age` ✔<br>`o?.m()` ✔ | — | `u.map(\|u\| u.age)` | `u?.age` | **VD-269**: `recv?.m(args)` gọi method qua **một** lớp `Option`; `None` ⇒ `None` (không đánh giá đối số), kết quả lift **một** lớp |
| `Some(5)` (dựng) ✔<br>`Some(v)` (pattern) ✔ | — | `Some(5)` / `Some(v) =>` | — | **R2/VD-212**: dựng **và** pattern đều viết hoa (`Some(v)`, `None`); dạng viết thường trong pattern ⇒ **lỗi VS0203** + gợi ý dạng đúng; `vis fmt` tự sửa trước khi biên dịch |
| `Result[int, str]`, `Ok(7)`, `Err("hỏng")` ✔ | exception | `Result<i64, String>` | `throw` / union | |
| `let v = f()?` ✔ | `v = f()` (try/except) | `let v = f()?;` | `try { f() }` | |
| `let v = f() else:`<br>`    print(e)`<br>`    return 0` ✔ | `try/except` | `match f() { ... }` | `catch` | **R3b/VD-263**: `Ok`/`Some` bóc giá trị; `Err`/`None` chạy khối `else` (nhận lỗi nếu có `else e:`); khối `else` phải **thoát mọi nhánh** (`return`/`break`/`continue` hoặc `if`/`match` mọi nhánh thoát) — rơi xuống cuối ⇒ `VS0206` |
| `assert cond` ✔ | `assert` | `assert!(...)` | `console.assert` | |
| `panic("hỏng")` ✔ | `raise` | `panic!` | `throw` | **VD-267**: builtin trả `never`; dừng chương trình bằng trap `VS0501` (exit **101**) ở **cả hai** engine; dùng được ở mọi vị trí biểu thức (nhánh `else`, `return panic(...)`); `panic(Error)` lấy `message` |
| `AnyError` ✔ | — | `Box<dyn Error>` | `unknown` | alias `Error` (VD-231) — dùng được; sai kiểu báo `VS0412` |

## 8. Trait, `impl`, `derives`

| ViScript | Python | Rust | TS/JS |
|---|---|---|---|
| `trait Hien:`<br>`    fn hien(self) -> str` ✔ | `class P(Hien):` / `Protocol` | `trait Hien { fn hien(&self) -> String; }` | `interface Hien { hien(): string }` |
| `impl Hien for P:`<br>`    fn hien(self) -> str: ...` ✔ | kế thừa | `impl Hien for P { ... }` | `class P implements Hien` |
| `impl P:`<br>`    fn tuoi(self) -> int: ...` ✔ | method thường | `impl P { fn tuoi(&self) -> i64 }` | `class P { tuoi() }` |
| `type P derives Show:` ✔ | — | `#[derive(Debug)]` | — |
| `dyn Hien` ✖ | — | `dyn Trait` | — |

`derives` sinh sẵn: `Eq`, `Show`, `Json` — không cần viết tay (Ord/Hash/Default chưa có).

## 9. Thư viện chuẩn (đóng băng — `agent/API-STD.txt`)

Gọi dạng **module.chấm_hàm**, sau khi `use std.<module>`:

| Module | Hàm chính | Ví dụ đã chạy ✔ |
|---|---|---|
| `std.fs` | `exists` `size` `is_file` `is_dir` `list` `read` `read_lines` `read_bytes` `walk` `glob` `write` `write_bytes` `append` `mkdir` `mkdir_all` `remove` `copy` `rename` | `fs.read_lines("a.txt")` → `Ok(["a", "b"])`; `fs.walk(".")` → tương đối, thư mục kèm `/`, DFS, **bỏ symlink**; `fs.glob(".", "**/*.txt")` → lọc `walk`, sắp theo **byte**, mẫu không `/` khớp **basename** (`*` `?` `**`); `mkdir` **một cấp**, `mkdir_all` **đệ quy** (đã có thư mục ⇒ `Ok`, tệp/symlink trên đường ⇒ `Err`, **không** đi theo symlink), `remove` **chỉ tệp**; `copy`/`rename` **chỉ tệp** (từ chối thư mục + symlink) và **từ chối đích đã tồn tại** (không ghi đè), `copy` chép **từng byte**, `rename` cùng hệ tệp; `size` = số **byte** tệp thường, `is_file`/`is_dir` dùng `symlink_metadata` (**không** đi theo symlink: symlink ⇒ `false`/`false`, `size` ⇒ `Err`), thiếu path ⇒ predicate `Ok(false)`; `read_bytes`/`write_bytes` = byte thô `list[int]` (`0..=255`, gồm `0x00`/`0xFF`), `write_bytes` tạo/cắt tệp, kiểm **hết** phần tử trước khi ghi (sai ⇒ `Err`, không tạo/cắt) |
| `std.list` | `sort` `sort_by` `take` `drop` `chunk` `windows` `concat` `flatten` `reverse` `unique` `zip` `min` `max` `sum` `product` `mean` `median` `sum_float` `is_empty` `index_of` `last_index_of` `contains` `binary_search` | `list.sort([3,1,2])` → `[1,2,3]`; `list.sum([1,2,3])` → `6`, `list.product([1,2,3])` → `6` (gộp **trái → phải** bằng `+`/`*` **checked** `i64`; rỗng ⇒ `0`/`1`; tràn ở **bất kỳ bước nào** ⇒ trap `VS0507`, không wrap; **chỉ** `list[int]`); `list.mean([1.0, 2.5])` → `Some(1.75)`, `list.mean([])` → `None` (rỗng ⇒ `None`, không chia 0; **chỉ** `list[float]`; cộng **trái → phải** `f64` rồi chia `len`, **IEEE**: `NaN`/`±inf` lan truyền, `+inf + -inf` ⇒ `NaN`, không trap); `list.median([3.0, 1.0, 2.0])` → `Some(2)`, `list.median([1.0, 2.0, 3.0, 4.0])` → `Some(2.5)` (rỗng ⇒ `None`; **chỉ** `list[float]`; sao chép rồi sắp tăng, danh sách gốc **không đổi**; **có `NaN` ở bất kỳ vị trí ⇒ `None`** tất định; `±inf` sắp bình thường; lẻ ⇒ giữa, chẵn ⇒ `(a + b) / 2.0`, không trap); `list.sum_float([1.5, 2.5])` → `4`, `list.sum_float([])` → `0` (tổng **chỉ** `list[float]`, trả `float` trần; hạt giống `+0.0` nên rỗng ⇒ `0`; cộng **trái → phải** `f64`, **IEEE**: `NaN`/`±inf` lan truyền, `+inf + -inf` ⇒ `NaN`, tràn ⇒ `±inf`, không trap; mọi tổng bằng 0 đều `+0` — kể cả `[-0.0]` — khác `mean`/`median` lấy phần tử đầu làm hạt giống); `list.binary_search([10,20,20,30], 20)` → `Some(1)` (xs **đã sắp tăng** theo thứ tự `sort`/`min`; trả **biên dưới** — vị trí **đầu** của cụm bằng; không có/rỗng ⇒ `None`; chưa sắp ⇒ không xác định nhưng **không trap**); `list.windows([1,2,3], 2)` → `[[1,2],[2,3]]` (cửa sổ **trượt**, `chunk` là nhóm rời; `n <= 0` ⇒ trap `VS0501`, `n > len` ⇒ `[]`); `list.is_empty([])` → `true`; `list.index_of([10,20,30,20], 20)` → `Some(1)`, `last_index_of` → `Some(3)` (0-based, so cấu trúc như `==`; không có/rỗng ⇒ `None`); `list.contains([10,20,30], 20)` → `true` (cùng phép so cấu trúc như `index_of`; không có/rỗng ⇒ `false`, không trap); `list.flatten([[1,2],[],[3]])` → `[1,2,3]` (nối **một cấp**; list con rỗng không đóng góp; phần tử vốn là list **giữ nguyên**: `[[[1],[2]]]` → `[[1],[2]]`); `list.zip([1,2],[3])` → `[[1,3]]` |
| `std.map` | `keys` `values` `merge` `remove` `get_or` `is_empty` `contains_key` | `map.keys(m)`; `map.is_empty({})` → `true`; `map.contains_key({"a": 1}, "a")` → `true` (khoá so **cấu trúc** như `map.has`/`map.get`, gồm list/map/record/enum lồng nhau; rỗng/không có ⇒ `false`, không trap; sugar `m.contains_key(k)`) |
| `std.str` | `split` `splitn` `join` `upper` `lower` `title` `trim` `trim_start` `trim_end` `trim_matches` `trim_start_matches` `trim_end_matches` `replace` `replace_first` `replace_last` `replace_n` `parse_int` `parse_float` `repeat` `starts_with` `ends_with` `contains` `strip_prefix` `strip_suffix` `find` `rfind` `count` `lines` `split_whitespace` `pad_left` `pad_right` `center` `byte_len` `char_at` `char_count` `is_empty` `is_ascii` `is_blank` | `str.join(["a","b"], "-")` → `a-b`; `str.splitn("a,b,c", ",", 2)` → `[a, b,c]` (tối đa `n` phần, **phần cuối** giữ nguyên phần còn lại kể cả dấu phân cách; `n=0` ⇒ `[]`, `n=1` ⇒ `[s]`, `n>số phần` ⇒ như `split`; dấu cuối sinh phần rỗng: `"a,b,"` chia 3 ⇒ `[a, b, ]`; `n<0` ⇒ trap `VS0501`; `sep` rỗng theo **đúng nghĩa Rust** `str::splitn("")`: `"abc"` chia 2 ⇒ `[, abc]`, chia 1 ⇒ `[abc]`, `""` chia 2 ⇒ `[, ]`, `"a"` chia 3 ⇒ `[, a, ]`); `str.title("hELLo wORLD")` → `Hello World`, `str.title("hello-world")` → `Hello-world`, `str.title("ßig")` → `SSig` (hoa **scalar đầu** mỗi từ, thường phần còn lại; từ tách theo **khoảng trắng Unicode** và khoảng trắng giữ nguyên từng scalar; dấu câu **không** mở từ mới; `ß` nở `SS`); `str.center("ab", 5, "-")` → `-ab--`; `str.pad_left("đ", 3, "-")` → `--đ` (đệm theo **ký tự Unicode**, `fill` rỗng ⇒ trap `VS0501`, `center` dư bên phải); `str.trim_matches("--x--", "-")` → `x`, `str.trim_start_matches("--x--", "-")` → `x--`, `str.trim_end_matches("--x--", "-")` → `--x` (cắt **tập scalar** `chars`, lặp tới scalar không thuộc tập; `chars` rỗng ⇒ nguyên vẹn, hết ⇒ `""`); `str.strip_prefix("banana", "ban")` → `Some(ana)`, `str.strip_suffix("banana", "ana")` → `Some(ban)` (bỏ **một lần** đúng đầu/cuối; không khớp ⇒ `None`; affix rỗng ⇒ `Some(s)`; `"aaa"` bỏ `"a"` ⇒ `Some(aa)`); `str.find("đ😀x", "x")` → `Some(2)`; `str.replace_first("banana", "na", "X")` → `baXna`, `str.replace_first("aaa", "aa", "b")` → `ba`, `str.replace_first("abc", "", "-")` → `-abc` (chỉ thay **lần khớp đầu**, cùng đơn vị **scalar Unicode** với `replace`/`find`; không khớp ⇒ nguyên `s`; `from` rỗng ⇒ chèn `to` một lần ở đầu); `str.replace_last("banana", "na", "X")` → `banaX`, `str.replace_last("aaa", "aa", "b")` → `ab`, `str.replace_last("abc", "", "-")` → `abc-` (chỉ thay **lần khớp cuối**, cùng đơn vị **scalar Unicode** với `replace`/`replace_first`/`rfind`; không khớp ⇒ nguyên `s`; `from` rỗng ⇒ nối `to` một lần ở cuối); `str.replace_n("banana", "na", "X", 1)` → `baXna`, `str.replace_n("aaaa", "aa", "b", 2)` → `bb`, `str.replace_n("abc", "", "-", 2)` → `-a-bc`, `str.replace_n("abc", "a", "b", -1)` → trap `VS0501` (thay tối đa `n` lần khớp **không chồng** đầu tiên, **trái → phải**, đúng Rust `replacen`; `n = 0` ⇒ nguyên `s`, `n` quá số lần ⇒ thay tất cả; `from` rỗng ⇒ chèn tại mọi **biên scalar** theo `n`; `n < 0` ⇒ trap); `str.rfind("banana", "na")` → `Some(4)`; `str.count("aaaa", "aa")` → `2` (offset **scalar Unicode**, `count` **không chồng**; needle rỗng: `Some(0)`/`Some(len)`/`len+1`); `str.parse_int("42")` → `Some(42)`, `str.parse_int("0x10")` → `None` (parse thập phân **chặt**, sai/tràn ⇒ `None`); `str.parse_float(".5")` → `Some(0.5)`, `str.parse_float("1e3")` → `Some(1000)`, `str.parse_float("-1e-400")` → `Some(-0)`, `str.parse_float("inf")`/`str.parse_float("1e400")` → `None` (cú pháp thập phân Rust: dấu, `.5`/`5.`, mũ `e`/`E`; **từ chối mọi non-finite** `inf`/`-inf`/`NaN`/tràn ⇒ `None`; underflow thành `±0.0` vẫn `Some`; sai cú pháp/rỗng ⇒ `None`); `str.byte_len("đ😀x")` → `7`, `str.char_count("đ😀x")` → `3`, `str.char_at("đ😀x", 1)` → `Some(😀)`, `str.char_at("đ😀x", -1)`/`str.char_at("abc", 3)`/`str.char_at("", 0)` → `None` (`byte_len` = **byte UTF-8**, `char_count` = **scalar Unicode**, **không** phải grapheme; `char_at` = scalar thứ `i` **zero-based**, âm/quá dài/rỗng ⇒ `None`, **không trap**, **không** cắt byte; `is_empty` cho `str`/`list`/`map`; `str.is_ascii("abc\n\0")` → `true`, `str.is_ascii("abcđ")` → `false` — đúng ⇔ **mọi byte** thuộc `0x00..=0x7F`, rỗng ⇒ `true`, sugar `s.is_ascii()`); `str.is_blank("")` → `true`, `str.is_blank(" \t\r\n")` → `true`, `str.is_blank(" a ")` → `false` (rỗng hoặc **mọi scalar** `char::is_whitespace` ⇒ `true`; ZWSP/BOM/MVS/dấu kết hợp **không** phải khoảng trắng; sugar `s.is_blank()`) |
| `std.path` | `join` `join_all` `basename` `dirname` `ext` `stem` `is_abs` `is_absolute` `is_relative` `normalize` `relative` `with_extension` `with_stem` | `path.join("a","b")` → `a/b`; `path.is_absolute("/a")` → `true`, `path.is_absolute("")` → `false` (**đúng** quy tắc **từ vựng** của `path.is_abs`: cùng helper/cờ nền tảng, rỗng ⇒ **không** tuyệt đối, `//` ⇒ tuyệt đối, `\`/NUL là ký tự **thường**, không đĩa/`cwd`/symlink; **không sugar** — luôn gọi `path.is_absolute(p)`); `path.is_relative("a/b")` → `true`, `path.is_relative("")` → `true`, `path.is_relative("/a")`/`path.is_relative("//")` → `false` (**đúng** phần bù **từ vựng** của `path.is_abs`: cùng quy tắc POSIX, không đĩa/`cwd`; rỗng ⇒ tương đối; `\`/NUL là ký tự **thường**; **không sugar** — luôn gọi `path.is_relative(p)`); `path.join_all(["a","b","c"])` → `a/b/c`, `path.join_all([])` → `""` (nối **list[str]** trái→phải bằng **đúng** quy tắc `path.join`: gấp trái, `[]` ⇒ rỗng, một phần tử ⇒ chính nó, thành phần tuyệt đối ở giữa/cuối **thay** phần đã ghép, thành phần rỗng theo `path.join` (`["a",""]` ⇒ `a/`), `/` cuối của acc bị bỏ khi ghép tiếp; **không sugar** — luôn gọi `path.join_all(parts)`); `path.normalize("/a//b/./c/../d/")` → `/a/b/d` (chuẩn hoá **từ vựng** POSIX, thuần chuỗi: gộp `/`, bỏ `.`, `..` phân giải với thành phần thường trước; `../a` giữ `..`, `/..` → `/`; rỗng ⇒ `.`, chỉ toàn `/` ⇒ `/`; bỏ `/` cuối trừ gốc; `\` và NUL là ký tự **thường**; sugar `p.normalize()`); `path.relative("a/b", "a/c")` → `../c`, `path.relative("/a/b", "/a/c/d")` → `../c/d` (tương đối **từ vựng** POSIX: chuẩn hoá hai đầu bằng **cùng** logic `normalize`, bỏ tiền tố chung, phát `..` cho mỗi thành phần `from` còn lại rồi nối phần `to`; cùng loại ⇒ kết quả **tương đối**, kể cả hai tuyệt đối; trộn tuyệt đối/tương đối ⇒ trả `to` đã chuẩn hoá, **không** dùng `cwd`; `..` là thành phần **thường** khi so tiền tố; rỗng ⇒ `.`; **không sugar** — luôn gọi `path.relative(from, to)`); `path.with_extension("a/b.txt", "md")` → `a/b.md`, `path.with_extension("a/b", "txt")` → `a/b.txt`, `path.with_extension("a/b.txt", "")` → `a/b` (thay/đặt/bỏ phần mở rộng **cuối** của **thành phần cuối**, thuần chuỗi: có ext ⇒ thay; chưa có ext ⇒ nối `.` + ext; ext rỗng ⇒ xoá **kể cả** dấu chấm, không ext + rỗng ⇒ nguyên văn; tên bắt đầu `.` **không** có ext theo `path.ext` (`".gitignore"` + `txt` ⇒ `.gitignore.txt`, `.a.b` + `txt` ⇒ `.a.b.txt`); dấu chấm trong **thư mục** không phải ext; `/` cuối bị bỏ khi có tên tệp (`"a/b/"` + `md` ⇒ `a/b.md`, + `""` ⇒ `a/b`); **không có tên tệp** (`""`, `/`, `.`, `..`, `a/..`) ⇒ nguyên văn; `ext` dùng **nguyên văn** (`"a"` + `".md"` ⇒ `a..md`; `"foo."` + `txt` ⇒ `foo..txt` vì `path.ext("foo.")` = `""`); **không sugar** — luôn gọi `path.with_extension(p, ext)`); `path.with_stem("a/b.txt", "c")` → `a/c.txt`, `path.with_stem("a/b", "c")` → `a/c`, `path.with_stem("a/b.txt", "")` → `a/.txt` (thay **stem** của **thành phần cuối**, thuần chuỗi, **giữ nguyên** phần mở rộng hiện có: stem lấy **đúng** `path.stem`, ext lấy **đúng** `path.ext`; chưa có ext ⇒ thay **toàn bộ** tên; stem rỗng vẫn giữ `.` + ext, không ext + stem rỗng ⇒ tên rỗng (`"a/b"` ⇒ `a/`); tên bắt đầu `.` **không** có ext (`.gitignore` + `c` ⇒ `c`, `.a.b` + `c` ⇒ `c`, khác Rust `c.b`); `foo.` + `c` ⇒ `c` (khác Rust `c.`); `/` cuối bị bỏ khi có tên tệp; **không có tên tệp** (`""`, `/`, `.`, `..`, `a/..`, `a/.`) ⇒ nguyên văn — khác Rust; tiền tố giữ **nguyên văn** (`a//b` + `c` ⇒ `a//c`, `a/./b` + `c` ⇒ `a/./c`); stem không bị kiểm (`x/y`, `c.d`, `\`, NUL dùng nguyên văn); **không sugar** — luôn gọi `path.with_stem(p, stem)`) |
| `std.time` | `now_ms()` `monotonic_ms()` | |
| `std.json` | `encode` `decode` `encode_pretty` | `json.decode("[1,2]")` → `Ok(Arr([Num(1), Num(2)]))` (enum dựng sẵn `Json`: `Null Bool Num Str Arr Obj`; lỗi std → bản ghi `Error { message, source: Error?, span: str? }`) |
| `std.net` | `listen` `connect` `read` `write` `accept` `close` + `*_timeout`, `local_addr`, `close_listen` | loopback, **không TLS** |
| `std.http` | `get` `head` + `_timeout` | HTTP/1.1 loopback, chunked, redirect GET ≤5 |
| `std.sqlite` | `open` `open_with` `close` `execute` `execute_batch` `query` `query_one` `explain` `tables` `table_exists` `index_exists` `columns` `indexes` `index_columns` `primary_keys` `schema_sql` `schema_version` `last_insert_rowid` `changes` `begin` `commit` `rollback` `in_transaction` | `sqlite.query_one(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(1)])` → `Ok(RowSet)` tối đa 1 hàng; `sqlite.execute_batch(h, "CREATE TABLE t (a); INSERT INTO t VALUES ('x;y');")` → `Ok(2)` (tách `;` ngoài chuỗi/comment); `sqlite.explain(h, "SELECT a FROM t WHERE a = ?", [SqlValue.Integer(1)])` → `Ok(RowSet)` 4 cột `id,parent,notused,detail`; `sqlite.tables(h)` → `Ok([Alpha, v_alpha])` (tên bảng/view schema chính, sắp **byte**, bỏ `sqlite_%`, giữ hoa/thường); `sqlite.table_exists(h, "t")` → `Ok(true)` (bảng **hoặc view** schema chính tên đúng qua `sqlite_master` `type IN ('table','view')`; tên **bind tham số**, so khớp **BINARY** — phân biệt hoa/thường; thiếu/index/trigger/TEMP ⇒ `Ok(false)`; `sqlite_%` theo truy vấn — `sqlite_sequence` ⇒ `true`, `sqlite_master`/autoindex ⇒ `false`); `sqlite.index_exists(h, "i_t")` → `Ok(true)` (index **schema chính** tên đúng qua `sqlite_master` `type = 'index'`; tên **bind tham số**, so khớp **BINARY** — phân biệt hoa/thường; thiếu/rỗng/bảng/view/trigger/TEMP ⇒ `Ok(false)`; autoindex `sqlite_autoindex_%` theo truy vấn — đúng tên ⇒ `true`); `sqlite.columns(h, "t")` → `Ok([zeta, alpha])` (tên cột schema chính theo **thứ tự khai báo**, tên đối tượng **bind tham số**, thiếu ⇒ `Ok([])`); `sqlite.indexes(h, "t")` → `Ok([Alpha_idx, zeta_idx])` (tên index **người dùng tạo** của bảng schema chính, sắp **byte**, bỏ autoindex `sqlite_autoindex_%`, tên bảng **bind tham số**, thiếu/view ⇒ `Ok([])`); `sqlite.index_columns(h, "i_ba")` → `Ok([b, a])` (cột của index schema chính theo **thứ tự index** `(b, a)`, **không** sắp alphabet; tên index **schema-global** — index bảng khác vẫn thấy theo tên; mục **biểu thức** bị **bỏ qua**, index thiếu/autoindex/TEMP ⇒ `Ok([])`; tên index **bind tham số**); `sqlite.primary_keys(h, "t")` → `Ok([id])` (tên cột **khoá chính** của bảng schema chính theo **thứ tự khoá** `pk` 1..N — `PRIMARY KEY (b, a)` ⇒ `[b, a]`, **không** sắp alphabet; tên bảng **bind tham số**; không PK/view/thiếu ⇒ `Ok([])`); `sqlite.schema_sql(h, "t")` → `Ok(Some(CREATE TABLE t (a TEXT)))` (văn bản SQL khai báo của table/view/index/trigger **schema chính** theo **tên** bind, **phân biệt hoa/thường**; thiếu/`sql` NULL như autoindex ⇒ `Ok(None)`); `sqlite.schema_version(h)` → `Ok(1)` (schema cookie `schema_version` của **schema chính** (`main`) đọc qua `PRAGMA schema_version`, **chỉ đọc**; mỗi lời gọi đọc lại nên DDL trên **cùng connection** làm giá trị **tăng** — `CREATE TABLE` ⇒ tăng, truy vấn chỉ đọc/ghi dữ liệu ⇒ **không** đổi; DDL bảng **TEMP** chỉ đổi `temp.schema_version` nên cookie `main` **không** đổi; `:memory:` mới ⇒ `0`, DB tệp có thể bắt đầu ở số **không âm** bất kỳ; lỗi handle/host ⇒ `Err(Error)`) |

Method trên giá trị (không cần `use`):

| Kiểu | Method ✔ |
|---|---|
| `list` | `len` `get` `first` `last` `push` `map` `filter` `each` `sum` `join` `enumerate` `any` `all` `find` `is_empty` + sugar `sort` `sort_by` `take` `drop` `chunk` `windows` `concat` `reverse` `unique` `zip` `min` `max` `index_of` `last_index_of` `contains` `binary_search` `product` `mean` `median` `sum_float` — `map`/`filter` **trả list ngay** (eager); `sum` là method **lõi** (vẫn nhận `list[float]`; trên `list[int]` cộng checked như `list.sum`), `product` là sugar R5c của `list.product` (chỉ `list[int]`), `mean` là sugar R5c của `list.mean` (chỉ `list[float]`, trả `float?`; rỗng ⇒ `None`), `median` là sugar R5c của `list.median` (chỉ `list[float]`, trả `float?`; rỗng/`NaN` ⇒ `None`), `sum_float` là sugar R5c của `list.sum_float` (chỉ `list[float]`, trả `float`; rỗng ⇒ `0`; hạt giống `+0.0` nên mọi tổng 0 đều `+0`) **[ĐÃ CÓ]** |
| `Iterator[T]` | `.lazy()` trả iterator; cùng bộ method + `.to_list()`/`.to_map()`/`.to_set()`; **duyệt lại được** — **[DỰ KIẾN]** (R24). `for` trên range **đã lười** (VD-264: không cấp phát N phần tử); list/str/map vẫn vật chất hoá |
| `map` | `len` `get` `has` `keys` `values` `is_empty` + sugar `get_or` `merge` `remove` |
| `str` | `len` `chars` `upper` `lower` `split` `splitn` `contains` `is_empty` + sugar `trim` `trim_start` `trim_end` `trim_matches` `trim_start_matches` `trim_end_matches` `replace` `replace_first` `replace_last` `replace_n` `parse_int` `parse_float` `repeat` `starts_with` `ends_with` `find` `rfind` `count` `lines` `split_whitespace` `pad_left` `pad_right` `center` `byte_len` `char_at` `char_count` `is_ascii` `is_blank` |

## 10. Chế độ shell (chỉ ViScript có)

| Viết | Ghi chú |
|---|---|
| `ls ./photos \| where size > 5mb \| sort_by size --desc \| take 10` | pipeline trong `.vish` / ViShell |
| `pub cmd ls(path: path = p".") -> stream[FileEntry]:` | `cmd` = hàm + cờ `--x`; tự sinh `--help` **[DỰ KIẾN]** — chưa có ở HEAD (P0-3) |
| `cmd { ls (dir) \| where size > 5mb }` | dùng pipeline ngay trong `.vis` |
| `extern cmd git(sub: GitSub, --oneline: bool, ...rest: str)` | khai chữ ký cho chương trình ngoài |
| `^ten` | ép gọi chương trình **ngoài** |
| `\| complete` | chặn lỗi pipeline, trả `{stdout, stderr, code, how}` |

## 10b. LSP (`vis lsp --stdio`)

| Nhóm | Method / việc |
|---|---|
| Soạn thảo | `completion` (kể cả module `std.*`), `hover` (biến, trường, biến thể enum, phương thức người dùng, phương thức lõi + builtin), `signatureHelp`, `documentHighlight`, `selectionRange` |
| Điều hướng | `definition` (trong tệp + qua `rootUri`), `references`, `documentSymbol`, `workspace/symbol`, `foldingRange` |
| Sửa | `rename` + `prepareRename`, `formatting` (dùng `vis fmt`) |
| Chẩn đoán | publish diagnostics khi mở/đổi/lưu; module-aware; UTF-16 |

## 11. Những thứ **không có** (và thay bằng gì)

| Không có | Thay bằng | Python | Rust | TS/JS |
|---|---|---|---|---|
| `class`, kế thừa | `type` + `impl` | `class` | `struct` + `impl` | `class` |
| `try/catch`, exception | `Result` + `?` + `else err:` | `try/except` | `Result` | `try/catch` |
| `null` / `undefined` | `None` + `T?` (**R2** xong) | `None` | `Option` | `null/undefined` |
| `switch` | `match` | `match` | `match` | `switch` |
| `any` | `Json` + chuyển đổi tường minh | — | — | `any` |
| `1 < x < 10` | `1 < x and x < 10` | **hợp lệ** | lỗi | lỗi |
| `s[i]`, `s[1:3]` | `.chars()`, `.char_at(i)`, `.byte_len()`, `.char_count()`, `xs.get(i)` | **hợp lệ** | không có | **hợp lệ** |
| `"a" + "b"` | **hợp lệ** (R6) | **hợp lệ** | **hợp lệ** | **hợp lệ** |
| `1.` (float) | `1.0` | **hợp lệ** | **hợp lệ** | **hợp lệ** |
| `++`, `--` | `x += 1` | — | — | **có** |
| macro (`println!`) | `print(...)` | — | **có** | — |
| `unsafe`, con trỏ, borrow | ngữ nghĩa giá trị + `ref[T]` | — | **có** | — |

## 12. Mười một cái bẫy cho người mới

1. **`let` là bất biến** — ngược với `let` của JS. Muốn gán lại: `var`.
2. **`var` không phải `var` của JS** (không function-scope) — nó chỉ là "biến đổi được", giống `let mut`.
3. **`"a" + "b"` nối được** (R6) — nhưng chỉ `str + str`; `"a" + 1` vẫn lỗi (`VS0414`).
4. **`match` phải xét hết**; nhánh có `if` (guard) **không** được tính là đã xét.
5. **`Some`/`None`/`Ok`/`Err` viết hoa** (cả dựng và pattern) — **R1a/R2/VD-212**: dạng viết thường trong pattern ⇒ **lỗi `VS0203`** (khác `VS0111` cảnh báo của R1a — nay là lỗi); `vis fmt` tự sửa; từ khoá cũ ⇒ **VS0203**.
6. **Record literal `{…}` phải liệt kê đủ trường** — giá trị mặc định của `type` chỉ tự áp khi dựng bằng **tên kiểu** (`User(name: "An")`), **không** áp cho `{name: …}`.
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
  `User(name: "An")` **đã có** (R5a/R11: dựng bản ghi theo **tên kiểu** + trường có tên; **không** nhận đối số vị trí);
  `AnyError` **nay là alias bản ghi `Error` (VD-231)** — dùng được; sai kiểu báo `VS0412`; format spec `{x:.2}` **đã có** (R6);
  `dyn` **chưa dùng được**; `newtype` báo **VS0203** tại dòng khai (ST-179);
  `r"…"`/`b"…"`/`p"…"` **đã bóc tiền tố** (R6a: `print(p"C:\x")` in `C:\x`);
  `async` chạy **tuần tự một luồng** (VD-240, 2026-11-19: giữ một luồng, **không** đa luồng; test grid `test/sem/async/grid-*` pin): spawn **enqueue** task, task thực thi run-to-completion **khi await**; task được await chạy theo thứ tự **await**; task **không** await vẫn được chạy (khi kênh chặn hoặc ở join cuối) theo thứ tự tạo; **kênh FIFO**; không work-stealing — `recv` hàng trống, `send` đầy hàng, `chan(0)` đều bị chặn (`VS0505`); **hai engine byte-identical**; interp = VM.
  **Chan trả một giá trị `<kênh>` duy nhất** (không phải tuple `(tx, rx)` như thiết kế §742 tip cũ); dùng `c.send/c.recv`, không destructure; show `<fn>` đồng nhất cho cả hai engine.
- **Theo VD-178 (ST-178) — nhãn trạng thái**:
  - **[ĐÃ CÓ]** `filter`/`map` trả collection ngay; `if let Some(v) = o:` (R3a); `let x = r else:` (R3b); `while let PAT = EXPR:` (**R18b** — cấu trúc vòng lặp `if let`-style với pattern, thoát khi `None`); đơn vị `5mb`/`2h30m` chạy **và in chuẩn hoá** (`1.5mb`, `2h30m` — R10, VD-267); `p"…"` **thô hoàn toàn** + nội suy (R6a); `str+str` nối (R6b); `print(a, b)` + `print()` (R6d); `{expr:.N}` (R6c); mã thoát **101** cho panic (C1/R9); `assert cond` / `assert cond, "msg"` (R20a); `panic(msg)` builtin trả `never`, trap `VS0501` exit **101** ở cả hai engine (VD-267). **VD-222/225/227**: `json.decode(s) → Result[Json, Error]` ở interp và VM (JSON enum thật: `Null/Bool/Num/Str/Arr/Obj`); `Error` bản ghi dựng sẵn `{message, source: Error?, span: str?}`; `print(e)` in chỉ `{message}`; **`AnyError` = alias bản ghi `Error` (RecId(0))** (VD-231; `print(e)` in chỉ `{message}`); interp **và** VM tự nâng `T → T?` ở `let x: int? = …` (VD-227) **và** ở biên `fn f() -> T?` (VD-228 — `None`/`Some` không bọc đôi; argument `g(42)` + closure-biên (`fn(x) -> T?`) đều auto-lift (VD-229)).
  - **[ĐÃ CÓ]** pattern viết thường đã **nâng** `VS0111` (R1a) ⇒ **lỗi `VS0203`** (VD-212); từ khoá cũ (R2) đã bỏ — **VS0203**.
  - **[ĐÃ CÓ]** `main` trả `Err` ⇒ exit **1**; `assert cond, "msg"` (R20a); `if let`/`while let`. **VD-241 đợt coercion Option (2026-11-19)**: `T?? = Option[Option[int]]` lồng thật ở **runtime**; `None` trần ở mọi biên (let/var/gán lại/đối số/thân hàm/return) giữ **`None` lớp ngoài**; `Some(x)` nâng đúng một lớp; spelled-out `Option[Option[int]]` ≡ sugar `int??` ≡ alias — cả ba cùng kiểu/runtime (`optional/long-option-matrix.vis`).
  - **[DỰ KIẾN]** `.lazy()` + `Iterator[T]` duyệt lại (R24); `for` trên range **đã lười** (VD-264). **Lưu ý còn treo**: interp ↔ VM — interp auto-lift `let x: int? = expr` (VD-227) và `fn f() -> T?` (VD-228 Option B) đều auto-lift; closure-boundary (`fn(x) -> T?`) và argument `g(42)` auto-lift từ **VD-229**; lambda params không annot ⇒ không lift; `chan(0)` vẫn bị chặn `VS0505`.
- **Nguồn đọc thêm**: `test/sem/**` (525 ca ngữ nghĩa chạy thật: 487 + 27 ca `sqlite/` VD-241 Gate 1 + 11 ca `sqlite/` VD-241 pre-merge),
  [`docs/SPEC-lex.md`](docs/SPEC-lex.md) (từ vựng + EBNF),
  mục "Mức hiện thực VS2" cuối [`docs/ViScript-thiet-ke.md`](docs/ViScript-thiet-ke.md).
