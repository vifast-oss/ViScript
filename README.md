<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<p align="center">
  <img src="resources/brand/viscript-icon.svg" alt="ViScript" width="104" height="104">
</p>

# ViScript

**Ngôn ngữ lập trình dễ học cho người Việt.**

Cú pháp gọn như Python, kiểm tra kiểu như Rust — và mọi thứ bạn đọc (tên biến, thông báo lỗi) đều bằng tiếng Việt. ViScript bỏ hẳn những thứ dễ gây lỗi: con trỏ, `unsafe`, kế thừa lớp.

> Mục tiêu: học sinh cấp 3 đọc 10 phút là viết được chương trình đầu tiên; người mới được nhắc lỗi rõ ràng thay vì những dòng khó hiểu.

## Chạy thử trong 2 phút

```sh
git clone https://github.com/vifast-oss/ViScript.git
cd ViScript
./vis run vi-du/xin-chao.vis
```

Kết quả:

```text
Xin chào, ViScript!
```

> Lần chạy đầu sẽ tự biên dịch (vài phút). Cần cài Rust trước — xem [rustup.rs](https://rustup.rs).

## Vì sao học ViScript?

| Điểm | Nghĩa là gì |
|---|---|
| **Tên tiếng Việt** | Đặt tên biến, tên hàm bằng tiếng Việt tự nhiên: `let diem = 8.5`, `fn xep_loai(diem: float)`. Lỗi cũng báo bằng tiếng Việt. |
| **Gọn như Python** | Thụt lề thay ngoặc nhọn, không dấu `;`, không cần `fn main`. |
| **An toàn như Rust** | Kiểu tĩnh — kiểm tra trước khi chạy; không con trỏ, không `unsafe`, không kế thừa. |
| **Đơn vị đo** | `1kb + 1kib`, `2h + 30m` — cộng sai loại là lỗi biên dịch, không phải lỗi lúc chạy. |
| **Chạy ngay** | Một lệnh `./vis run`, không cần cấu hình dự án. |

## Nhìn là hiểu — cú pháp trong 10 phút

Mỗi mục dưới đây là một chương trình hoàn chỉnh: copy vào tệp `.vis` rồi `./vis run` là chạy.

### 1. In ra màn hình

```vis
print("Xin chào, ViScript!")
```

```text
Xin chào, ViScript!
```

### 2. Biến: `let` và `var`

```vis
let ten = "Minh"      # let: không đổi được
var tuoi = 17         # var: đổi được
tuoi += 1
print("Chào {ten}, {tuoi} tuổi")
```

```text
Chào Minh, 18 tuổi
```

> `let` giống `const` của JavaScript; `var` giống `let mut` của Rust. Muốn đổi giá trị thì dùng `var`.

### 3. Hàm

```vis
fn binh_phuong(x: int) -> int:
    x * x

print(binh_phuong(7))
```

```text
49
```

> Dòng cuối của hàm là giá trị trả về — không cần `return`. Thụt lề bằng 4 dấu cách.

### 4. Rẽ nhánh

```vis
fn xep_loai(diem: float) -> str:
    if diem >= 8.0:
        "Giỏi"
    elif diem >= 6.5:
        "Khá"
    else:
        "Cần cố gắng"

print(xep_loai(8.5))
print(xep_loai(6.0))
```

```text
Giỏi
Cần cố gắng
```

> `elif` chứ không phải `else if`. `if` cũng là biểu thức: `let s = if dung: "có" else: "không"`.

### 5. Vòng lặp

```vis
for i in 0..3:
    print("lần {i}")

var n = 3
while n > 0:
    print(n)
    n -= 1
```

```text
lần 0
lần 1
lần 2
3
2
1
```

> `0..3` là 0, 1, 2 (không gồm 3). Muốn gồm 3 thì viết `0..=3`.

### 6. Danh sách

```vis
let ds = [1, 2, 3, 4]
print(ds.map(x => x * 2))
print(ds.filter(x => x > 2))
print(ds.sum())
```

```text
[2, 4, 6, 8]
[3, 4]
10
```

> `map`/`filter` nhận hàm ngắn `x => ...`. Danh sách còn có `.len()`, `.get(i)`, `.first()`, `.last()`, `.join("-")`.

### 7. Map (từ điển)

```vis
let tuoi = {"An": 17, "Bình": 18}
print(tuoi["An"])
print(tuoi.keys())
```

```text
17
[An, Bình]
```

### 8. Bản ghi (record) — thay cho class

```vis
type HocSinh:
    ten: str
    diem: float

let hs: HocSinh = {ten: "Minh", diem: 8.5}
print("{hs.ten}: {hs.diem}")

let hs2 = hs with diem: 9.0
print(hs2.diem)
```

```text
Minh: 8.5
9
```

> Không có class, không kế thừa. Dữ liệu là **bản ghi**, hành vi là **hàm**. `with` tạo bản ghi mới, không sửa bản cũ.

### 9. Enum + match

```vis
enum Hinh:
    Tron(r: float)
    Vuong(canh: float)

fn dien_tich(h: Hinh) -> float:
    match h:
        Tron(r): 3.14 * r * r
        Vuong(c): c * c

print(dien_tich(Hinh.Tron(2.0)))
print(dien_tich(Hinh.Vuong(3.0)))
```

```text
12.56
9
```

> `match` phải xét hết mọi trường hợp — thiếu nhánh là lỗi biên dịch. Đây là cách ViScript bắt bạn xử lý đủ trường hợp.

### 10. Giá trị có thể thiếu (optional)

```vis
let ds = [10, 20, 30]
let o = ds.get(1)          # some(20)
if let Some(v) = o:
    print("có {v}")
else:
    print("không có")
print(ds.get(9) ?? -1)     # -1
```

```text
có 20
-1
```

> `int?` là "int hoặc không có". `??` là giá trị thay thế khi không có. `if let Some(v) = o:` lấy giá trị ra an toàn — **phải viết pattern** (`Some(v)`), dạng `if let v = o:` là lỗi.

### 11. Xử lý lỗi (Result)

```vis
fn chia(a: int, b: int) -> Result[int, str]:
    if b == 0:
        return Err("chia cho 0")
    Ok(a / b)

print(chia(10, 2))
print(chia(10, 0))
```

```text
Ok(5)
Err(chia cho 0)
```

> Không có `try/catch`. Hàm có thể lỗi thì trả `Result[T, E]`; chỗ gọi dùng `?` để chuyển lỗi lên trên.

### 12. Đơn vị đo

```vis
print(1kb + 1kib)     # 2024b
print(2h + 30m)       # 150m
print(5mb)            # 5mb
```

```text
2024b
150m
5mb
```

> Đơn vị là **một phần của kiểu**: `1kb + 1kib` hợp lệ, `1kb + 1h` là lỗi biên dịch.

### 13. Trait và impl (nâng cao)

```vis
trait Hien:
    fn hien(self) -> str

type Diem:
    x: int
    y: int

impl Hien for Diem:
    fn hien(self) -> str:
        "({self.x}, {self.y})"

let p: Diem = {x: 1, y: 2}
print(p.hien())
```

```text
(1, 2)
```

> `trait` giống interface: khai báo "làm được gì", `impl` viết "làm thế nào". Không có kế thừa.

### 14. Mẹo mới — viết nhanh hơn

```vis
type User:
    name: str
    age: int = 0

type Id = int
type Email(str)

enum Shape:
    Circle(float), Empty

fn cong(a: int, b: int) -> int:
    a + b

fn main():
    # `/` luôn ra float; `//` chia lấy phần nguyên; `%` khớp `//` (không âm)
    print(7 / 2)      # 3.5
    print(7 // 2)     # 3
    print(-7 % 2)     # 1

    # `let` che tên được trong cùng phạm vi
    let x = 1
    let x = "a"
    print(x)          # a

    # đối số có tên + trường bản ghi có mặc định
    print(cong(1, b: 5))      # 6
    print(User(name: "An"))   # User(name: An, age: 0)

    # `type X = T` (thay `alias`), `type Email(str)` (thay `newtype`)
    let e = Email("a@b")
    let Email(s) = e
    print(s)          # a@b

    # enum mang dữ liệu vị trí + nhiều biến thể cùng dòng
    print(Shape.Circle(2.0))   # Circle(2)

    # `xs[i]?` — chỉ số an toàn, trả `T?` thay vì panic
    let xs = [10, 20, 30]
    print(xs[9]?)     # None

    # `if let` phải có pattern
    let o: int? = Some(5)
    if let Some(v) = o:
        print(v)      # 5

    # `let … else` — thoát sớm khi lỗi
    let r: Result[int, str] = Err("hong")
    let y = r else err:
        return
    print(y)
```

> Đây là các cải tiến của **pha 1 công thái học** (R3, R5a, R6, R7, R8, R11, R12a, R16, R17) — mỗi cái đều có ca test trong `test/sem/`.

## Bẫy thường gặp (đọc trước khi viết)

| Bẫy | Cách đúng |
|---|---|
| `let` là **bất biến** — gán lại là lỗi | Muốn đổi giá trị thì dùng `var` |
| `"a" + "b"` **nối** được (từ R6) | `"a" + "b"` ⇒ `ab`; hoặc nội suy `"{a}{b}"` |
| `print(a, b)` in **cách nhau 1 dấu cách** (từ R6) | `print("a", "b")` ⇒ `a b`; `print()` ⇒ dòng trống |
| Đặt biến tên `Ok`, `Err`, `Some`, `None` | Trùng tên hàm dựng — đổi tên khác (`dung`, `loi`, …) |
| `match` thiếu nhánh | Phải xét **hết** mọi trường hợp, hoặc thêm `_` |
| `ds[5]` ngoài phạm vi | Kiểm tra `.len()` trước, hoặc dùng `.get(5)` trả `int?` |
| `1 < x < 10` | Viết `1 < x and x < 10` |

## Bảng tra nhanh: ViScript ↔ Python ↔ Rust

| Bạn muốn… | ViScript | Python | Rust |
|---|---|---|---|
| In | `print("hi")` | `print("hi")` | `println!("hi");` |
| Khai báo (không đổi) | `let x = 1` | `x = 1` | `let x = 1;` |
| Khai báo (đổi được) | `var x = 1` | `x = 1` | `let mut x = 1;` |
| Hàm | `fn f(x: int) -> int:` | `def f(x):` | `fn f(x: i64) -> i64 { }` |
| Nếu | `if x > 0:` / `elif` / `else:` | y hệt | `if x > 0 { } else if …` |
| Lặp | `for i in 0..3:` | `for i in range(3):` | `for i in 0..3 { }` |
| Danh sách | `[1, 2, 3]` | `[1, 2, 3]` | `vec![1, 2, 3]` |
| Map | `{"a": 1}` | `{"a": 1}` | `HashMap::from([...])` |
| Bản ghi | `type U:` + `{...}` | `@dataclass` | `struct U { }` |
| Enum | `enum S:` + `S.Variant(...)` | `Enum` | `enum S { }` |
| Match | `match x:` | `match x:` | `match x { }` |
| Thiếu giá trị | `int?` / `None` / `??` | `int \| None` | `Option<i64>` |
| Lỗi | `Result[T, E]` / `?` | `try/except` | `Result<T, E>` |
| Đơn vị | `1kb + 1kib` | — | — |
| Nối chuỗi | `"a" + "b"` | `"a" + "b"` | `format!("{a}{b}")` |
| In nhiều giá trị | `print("a", "b")` | `print("a", "b")` | `println!("{} {}", a, b);` |
| Định dạng số | `"{x:.2}"` | `f"{x:.2f}"` | `format!("{x:.2}")` |
| Rỗng? | `xs.is_empty()` | `not xs` | `xs.is_empty()` |

**Nếu bạn đã biết Python**: ViScript giống ở thụt lề, `for … in`, `elif`; khác ở chỗ có kiểu tĩnh, không có `class`, và không có `try/catch`.

**Nếu bạn đã biết Rust**: ViScript giống ở kiểu tĩnh, `Result`, `match`; khác ở chỗ không cần quản lý bộ nhớ, không `&`/`mut`, và cú pháp ngắn hơn.

## Cùng một bài toán — 3 ngôn ngữ

**FizzBuzz**: in 1..15; chia hết 3 thì "Fizz", chia hết 5 thì "Buzz", cả hai thì "FizzBuzz".

**ViScript**

```vis
for i in 1..=15:
    if i % 15 == 0:
        print("FizzBuzz")
    elif i % 3 == 0:
        print("Fizz")
    elif i % 5 == 0:
        print("Buzz")
    else:
        print(i)
```

**Python**

```python
for i in range(1, 16):
    if i % 15 == 0:
        print("FizzBuzz")
    elif i % 3 == 0:
        print("Fizz")
    elif i % 5 == 0:
        print("Buzz")
    else:
        print(i)
```

**Rust**

```rust
fn main() {
    for i in 1..=15 {
        if i % 15 == 0 {
            println!("FizzBuzz");
        } else if i % 3 == 0 {
            println!("Fizz");
        } else if i % 5 == 0 {
            println!("Buzz");
        } else {
            println!("{i}");
        }
    }
}
```

> ViScript gần Python nhất: cùng thụt lề, cùng `elif`, cùng `for … in`. Khác ở chỗ có kiểu tĩnh và không cần `return` ở cuối hàm.

## Ví dụ đầy đủ — chạy được ngay

### Ví dụ 1 — Xếp loại học lực

```vis
type HocSinh:
    ten: str
    diem: float

fn xep_loai(diem: float) -> str:
    if diem >= 8.0:
        "Giỏi"
    elif diem >= 6.5:
        "Khá"
    elif diem >= 5.0:
        "Trung bình"
    else:
        "Cần cố gắng"

let lop: list[HocSinh] = [
    {ten: "An", diem: 8.5},
    {ten: "Bình", diem: 6.0},
    {ten: "Chi", diem: 9.5},
    {ten: "Dũng", diem: 4.5},
]

for hs in lop:
    print("{hs.ten}: {hs.diem} — {xep_loai(hs.diem)}")
```

```text
An: 8.5 — Giỏi
Bình: 6 — Trung bình
Chi: 9.5 — Giỏi
Dũng: 4.5 — Cần cố gắng
```

### Ví dụ 2 — Quản lý chi tiêu

```vis
use std.list

type Khoan:
    ten: str
    so_tien: int

let chi: list[Khoan] = [
    {ten: "Cơm trưa", so_tien: 30000},
    {ten: "Trà sữa", so_tien: 25000},
    {ten: "Sách", so_tien: 80000},
    {ten: "Xe buýt", so_tien: 7000},
]

let tong = chi.map(k => k.so_tien).sum()
print("Tổng chi: {tong} đồng")

let lon = chi.filter(k => k.so_tien >= 25000)
print("Khoản từ 25.000 đồng:")
for k in lon:
    print("  {k.ten}: {k.so_tien}")

let sap = list.sort_by(chi, k => k.so_tien)
print("Từ nhỏ đến lớn: {sap.map(k => k.ten)}")
```

```text
Tổng chi: 142000 đồng
Khoản từ 25.000 đồng:
  Cơm trưa: 30000
  Trà sữa: 25000
  Sách: 80000
Từ nhỏ đến lớn: [Xe buýt, Trà sữa, Cơm trưa, Sách]
```

### Ví dụ 3 — Đơn vị đo: thời gian và dung lượng

```vis
let hoc = 2h + 30m
let choi = 45m
print("Học: {hoc}")
print("Chơi: {choi}")
print("Tổng: {hoc + choi}")

let anh = 5mb
let video = 1gb
print("Tổng dung lượng: {anh + video}")
```

```text
Học: 150m
Chơi: 45m
Tổng: 195m
Tổng dung lượng: 1005mb
```

### Ví dụ 4 — JSON: lưu và đọc dữ liệu

```vis
use std.json

type HocSinh:
    ten: str
    diem: float

let hs: HocSinh = {ten: "Minh", diem: 8.5}
let chuoi = json.encode(hs)
print(chuoi)

let lai = json.decode(chuoi)
print(lai)
```

```text
{"ten":"Minh","diem":8.5}
Ok({"ten": Minh, "diem": 8.5})
```

### Ví dụ 5 — Viết test ngay trong tệp

```vis
fn cong(a: int, b: int) -> int:
    a + b

test fn cong_so_duong():
    assert cong(1, 2) == 3

test fn cong_so_am():
    assert cong(-1, -2) == -3
```

Chạy `./vis test vi-du/test.vis`:

```text
TAP version 13
1..2
ok 1 - cong_so_duong
ok 2 - cong_so_am
```

## Khi viết sai thì sao?

```text
bai.vis:3:7: error VS0412: toán tử `+`: `str` và `str` không cùng kiểu/đơn vị
```

Gõ `./vis explain VS0412` để xem nghĩa của mã lỗi (có tiếng Việt, English, 日本語).

> **ĐÃ CÓ (R9)**: mỗi mã lỗi một nghĩa, kèm gợi ý sửa ngay tại chỗ.

## Trạng thái ngôn ngữ

ViScript đang phát triển. Mọi ví dụ trên đều **chạy được ở bản hiện tại**. Một số thứ sẽ đổi trong thời gian tới:

| Tính năng | Trạng thái |
|---|---|
| Cú pháp cơ bản, bản ghi, enum, match, trait | ✅ ĐÃ CÓ |
| Optional, Result, `?`, `else` | ✅ ĐÃ CÓ |
| Đơn vị đo | ✅ ĐÃ CÓ (cách in sẽ đổi theo R10) |
| Thư viện chuẩn (fs, json, http, net…) | ✅ ĐÃ CÓ (API đóng băng) |
| `str + str`, `print(a, b)`, format `{x:.2}`, `p"…"` | ✅ ĐÃ CÓ (R6) |
| `if let Some(v) = o:` (pattern) | ✅ ĐÃ CÓ (R3a) |
| `let x = r else:` | ✅ ĐÃ CÓ (R3b) |
| `/` ra float, `//` chia nguyên, `%` khớp `//` | ✅ ĐÃ CÓ (R7) |
| `xs[i]?` chỉ số an toàn | ✅ ĐÃ CÓ (R8) |
| `enum Shape: Circle(float), Empty` + `U(n: 1)` | ✅ ĐÃ CÓ (R11) |
| `type X = T`, `type Email(str)` | ✅ ĐÃ CÓ (R12a) |
| `let` che tên cùng phạm vi | ✅ ĐÃ CÓ (R16) |
| `if xs:` ⇒ gợi ý `xs.is_empty()` | ✅ ĐÃ CÓ (R17) |
| `f(1, b: 5)` đối số có tên | ✅ ĐÃ CÓ (R5a) |
| Method người dùng (`impl P:`) | 🔜 DỰ KIẾN (R14) |
| `.lazy()` iterator | 🔜 DỰ KIẾN (R24) |
| Windows hỗ trợ đầy đủ | 🔜 VS3 |

Danh sách đầy đủ 24 quy tắc công thái học: [`agent/JOURNAL/vs-ergo.md`](agent/JOURNAL/vs-ergo.md).

## Các lệnh hay dùng

| Lệnh | Việc |
|---|---|
| `./vis run <tệp.vis>` | chạy chương trình |
| `./vis check <tệp…>` | kiểm tra lỗi mà không chạy |
| `./vis fmt --write <tệp…>` | tự căn lại thụt lề, khoảng trắng |
| `./vis test <tệp.vis>` | chạy mọi `test fn` trong tệp |
| `./vis explain <mã>` | giải thích mã lỗi (vi/en/ja) |
| `./vis lsp --stdio` | gợi ý cho trình soạn thảo (VS Code…) |
| `./vis pkg add <gói>` | thêm gói (mô hình phi tập trung, `vis.lock`) |

## Học tiếp

- [`CHEAT-SHEET-ViScript.md`](CHEAT-SHEET-ViScript.md) — bảng tra cứu đầy đủ ViScript ↔ Python ↔ Rust ↔ TS/JS.
- [`vi-du/`](vi-du/) — ví dụ chạy được cho người mới.
- [`docs/SPEC-lex.md`](docs/SPEC-lex.md) — đặc tả từ vựng + EBNF.
- [`docs/ViScript-thiet-ke.md`](docs/ViScript-thiet-ke.md) — thiết kế ngôn ngữ.
- [`test/sem/`](test/sem/) — bộ ca ngữ nghĩa chạy thật.

## Cấu trúc repo

```text
crates/    8 crate Rust: từ vựng, kiểu, thông dịch, máy ảo, CLI…
bin/       công cụ: vis, vis-dev, vis-fmt, vis-remote…
test/      bộ kiểm: ví dụ chạy được, fixture, báo cáo
docs/      đặc tả ngôn ngữ (SPEC-lex.md) và thiết kế
vi-du/     ví dụ chạy được cho người mới
agent/     sổ quyết định, QUEUE, nhật ký, báo cáo từng đợt
resources/ tài nguyên thương hiệu: icon ViScript + ViShell (brand/)
```

## Góp ý / phát triển

- Mọi thay đổi phải qua cổng: `./vis-dev gate --fast`.
- Quy ước commit: `[ST-##] <tóm tắt>`; quyết định thiết kế ghi trong [`agent/DECISIONS.md`](agent/DECISIONS.md).
- Đọc [`AGENTS.md`](AGENTS.md) và [`CONTRIBUTING.md`](CONTRIBUTING.md) trước khi sửa mã.

## Giấy phép

MIT OR Apache-2.0 — dùng, sửa, chia sẻ tự do.

Logo, wordmark và tài nguyên thương hiệu trong [`resources/brand/`](resources/brand/) **không** thuộc
phạm vi hai giấy phép đó — xem [`TRADEMARKS.md`](TRADEMARKS.md).
