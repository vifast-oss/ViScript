<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ViScript

**ViScript là một ngôn ngữ lập trình dễ học, viết cho người Việt.** Câu lệnh đọc gần như tiếng Việt, lỗi báo bằng tiếng Việt, và những thứ dễ gây lỗi trong ngôn ngữ khác (con trỏ, `unsafe`, kế thừa lớp) thì ViScript **không có**.

> Mục tiêu: học sinh cấp 3 đọc 10 phút là viết được chương trình đầu tiên; người mới được nhắc lỗi rõ ràng thay vì những dòng khó hiểu.

## Chạy thử trong 2 phút

```sh
git clone https://github.com/vifast-oss/ViScript.git
cd ViScript
cargo build --release
./vis run vi-du/xin-chao.vis
```

```
Xin chào, ViScript!
```

## Đọc README này thế nào?

ViScript đang trong giai đoạn hoàn thiện **bộ quy tắc công thái học R1–R24** (đã được duyệt — xem [`agent/JOURNAL/vs-ergo.md`](agent/JOURNAL/vs-ergo.md)). Mỗi ví dụ dưới đây ghi rõ trạng thái:

| Nhãn | Nghĩa |
|---|---|
| **[ĐÃ CÓ]** | chạy được ngay ở bản hiện tại |
| **[ĐANG LÀM]** | đã có, nhưng ngữ nghĩa sẽ đổi theo R1–R24 (pha 1) |
| **[DỰ KIẾN]** | thiết kế đã chốt, chưa có trong mã (pha 1 hoặc pha 2) |

---

## Công thái học: 24 quy tắc đã duyệt

Đây là phần **khác biệt cốt lõi** của ViScript — không phải “thêm tính năng”, mà là **bớt thứ gây lỗi cho người viết**.

| Quy tắc | Hiện tại | Sau khi làm | Pha |
|---|---|---|---|
| **R1** Chữ hoa | `some(v)` và `Some(v)` đều nhận | kiểu/biến thể **UpperCamel** (`Some`, `None`, `Ok`, `Err`); viết thường ⇒ **cảnh báo** + `vis fmt` tự sửa | 1 |
| **R2** Option/Result là enum std | `none` là từ khoá; `T??` lỗi | `Option[T]`/`Result[T,E]` là enum thường; bỏ `none`; `T??` vẫn lỗi | 1 |
| **R3** `if let` nhận pattern | `if let v = o:` (lấy giá trị) | `if let Some(v) = o:`; `while let`; `let … else` khối thoát | 1 |
| **R4** Lỗi có kiểu | lỗi chuỗi tự do | `Error` record + `Json` enum + `AnyError` ở ranh giới | 1 |
| **R5** Gọi hàm | tham số theo vị trí | tên đối số, record theo tên, method cho **mọi** hàm std | 1 |
| **R6** Chuỗi | `str + str` **lỗi** | `str + str` hợp lệ; `p"…"` thô; format spec; `print(a, b)` nhiều đối số | 1 |
| **R7** Số | `/` là chia nguyên | `/` → **float**, `//` chia nguyên, `%` **không âm** (`-7 % 2 == 1`), const suy kiểu | 1 |
| **R8** Chỉ số | `xs[i]?` lỗi | `xs[i]?` và `xs.get(i)` trả `Option` | 1 |
| **R9** Mã lỗi | nhiều nghĩa dùng chung mã | **mỗi mã một nghĩa**, có gợi ý sửa, báo tại chỗ, bỏ tiền tố `VS0` | 1 |
| **R10** Đơn vị | in thô (`150m`) | in chuẩn hoá (`2h30m`, `1.5mb`), cộng **cùng loại** hợp lệ, khác loại lỗi | 2 |
| **R11** Hàm dựng | record/enum tạo bằng cú pháp riêng | **mọi hàm dựng đều là hàm** (nhất quán) | 1 |
| **R12** Gộp khai báo kiểu | `alias` + `newtype` | `type X = T`, `type Email(str)`; bỏ `alias`/`newtype` | 1 |
| **R13** Biến thể enum | phải viết tiền tố | prelude + tiền tố khi trùng tên | 2 |
| **R14** Method người dùng | chỉ hàm std có method | `impl P:` + `self`/`var self`; **trùng tên method ⇒ lỗi** | 2 |
| **R15** Nối dòng | phải xuống dòng | nối bằng `.`, `??`, `and`, `or` | 2 |
| **R16** `let` che tên | `let` trùng tên ⇒ **lỗi** | `let` **che tên** được; `var` không khai lại | 1 |
| **R17** Điều kiện | `if xs:` lỗi khó hiểu | chỉ nhận `bool` + **gợi ý** `is_empty()` | 1 |
| **R18** Pattern mở rộng | pattern cơ bản | `as`, record, `while let` | 2 |
| **R19** Số & bit | `0o755` lỗi; tràn i64 **không panic** | `0o755`; tràn i64 **panic** (thoát 101); bit method; float literal cần chữ số sau `.` | 1–2 |
| **R20** Tiện ích | thiếu | `assert cond, "msg"`, `use as`, `use {}`, list/range bổ sung | 2 |
| **R21** Pipeline | `\|` trong script | `\|` **chỉ** trong shell/cmd | 2 |
| **R22** Trait là kiểu | không dùng được `list[Shape]` | trait là kiểu, gọi qua bảng ảo; bỏ “ép ngược bằng match” | 2 |
| **R23** Gán lồng sâu | chỉ gán trường phẳng | `u.addr.city = "A"` trên `var` | 2 |
| **R24** Iterator lười | `filter/map` trả collection ngay | `filter/map` **giữ** collection; `.lazy()` trả `Iterator[T]` duyệt lại được | 2 |

**Mã thoát chuẩn (C1)**: `0` thành công · `1` `main` trả `Err` · `2` lỗi biên dịch · **`101` panic** (tràn số, chỉ số ngoài phạm vi, chia 0, assert).

---

## So sánh với Python và Rust

### 1. Xin chào

**ViScript** — không cần `fn main`, `;`, ngoặc nhọn:
```vis
print("Xin chào, ViScript!")
```
**Python**: `print("Xin chào, ViScript!")` · **Rust**: `fn main() { println!("Xin chào, ViScript!"); }`

### 2. Biến và nội suy chuỗi

**ViScript**
```vis
let ten = "Minh"
print("Chào {ten}!")
```
**Python**: `ten = "Minh"; print(f"Chào {ten}!")` · **Rust**: `let ten = "Minh"; println!("Chào {ten}!");`

> `let` là bất biến; muốn đổi dùng `var`. **[ĐANG LÀM R16]**: `let` được che tên cùng phạm vi (hiện báo `VS0302`).

### 3. Hàm

**ViScript**
```vis
fn binh_phuong(x: int) -> int:
    x * x

print(binh_phuong(7))     # 49
```
**Python**: `def binh_phuong(x: int) -> int: return x * x` · **Rust**: `fn binh_phuong(x: i64) -> i64 { x * x }`

> Thân hàm thụt lề; dòng cuối là giá trị trả về (không cần `return`).

### 4. Danh sách: map / filter

**ViScript**
```vis
let ds = [1, 2, 3, 4]
print(ds.map(x => x * 2))      # [2, 4, 6, 8]
print(ds.filter(x => x > 2))   # [3, 4]
```
**Python**: `[x * 2 for x in ds]` · **Rust**: `ds.iter().map(|x| x * 2).collect::<Vec<_>>()`

> **[DỰ KIẾN R24]**: `filter/map` vẫn trả collection ngay; muốn lười thì `.lazy()` → `Iterator[T]`.

### 5. Đơn vị đo — điểm riêng của ViScript

**ViScript**
```vis
print(1kb + 1kib)     # 2024b
print(2h + 30m)       # 150m  → [DỰ KIẾN R10] in "2h30m"
```
**Python**: không có đơn vị, phải tự quy đổi (`kb + kib` — byte hay KB? không ai kiểm).
**Rust**: không có đơn vị trong ngôn ngữ, cần crate `uom`.

> Đơn vị là **một phần của kiểu**: cộng sai loại là lỗi biên dịch, không phải lỗi lúc chạy.

### 6. Giá trị có thể thiếu (optional)

**ViScript — hiện tại**
```vis
let ds = [10, 20, 30]
let o = ds.get(1)          # some(20)
if let v = o:
    print("có {v}")        # có 20
else:
    print("không có")
```
**ViScript — sau pha 1 (R2/R3)**
```vis
let o = ds.get(1)          # Some(20)
if let Some(v) = o:
    print("có {v}")
else:
    print("không có")
```
**Python**: `if (v := ds[1] if len(ds) > 1 else None) is not None: …`
**Rust**: `if let Some(v) = ds.get(1) { … } else { … }`

### 7. Bản ghi (record) — thay cho class

**ViScript**
```vis
type Diem:
    x: int
    y: int

let p: Diem = {x: 1, y: 2}
print(p.x + p.y)      # 3
```
**Python**: `@dataclass class Diem: x: int; y: int` · **Rust**: `struct Diem { x: i64, y: i64 }`

> Không class, không kế thừa — chỉ **bản ghi** (dữ liệu) + **hàm** (hành vi). **[DỰ KIẾN R14]**: `impl Diem:` thêm method.

### 8. Xử lý lỗi

**ViScript**
```vis
fn chia(a: int, b: int) -> int?:
    if b == 0:
        return none
    a / b

let kq = chia(10, 2) else:
    print("chia lỗi")
    return
print(kq)                 # 5
```
**Python**: `try/except` hoặc `if kq is None` · **Rust**: `let Some(kq) = chia(10, 2) else { … };`

> `let … else` buộc xử lý trường hợp thiếu **ngay tại chỗ**. **[DỰ KIẾN R2]**: `none` → `None`; **[R7]**: `/` ra float, dùng `//` khi muốn chia nguyên.

### 9. Method và trait — sau pha 2 (R14/R22)

**ViScript [DỰ KIẾN]**
```vis
type Diem:
    x: int
    y: int

impl Diem:
    fn tong(self) -> int:
        self.x + self.y

let p: Diem = {x: 1, y: 2}
print(p.tong())          # 3
```
**Python**: `class Diem: def tong(self): return self.x + self.y` · **Rust**: `impl Diem { fn tong(&self) -> i64 { self.x + self.y } }`

> Khác Python: **không kế thừa**, chỉ `impl` cho kiểu đã khai; trùng tên method là **lỗi**.

---

## Khi viết sai thì sao?

```
bai.vis:2:5: error VS0412: toán tử `+`: `str` và `str` không cùng kiểu/đơn vị
```

Gõ `./vis explain VS0412` để xem giải thích và cách sửa. **[ĐANG LÀM R9]**: mỗi mã một nghĩa, có gợi ý sửa tại chỗ.

## Các lệnh hay dùng

| Lệnh | Việc |
|---|---|
| `./vis run <tệp.vis>` | chạy chương trình |
| `./vis check <tệp…>` | kiểm tra lỗi mà không chạy |
| `./vis fmt --write <tệp…>` | tự căn lại thụt lề, khoảng trắng |
| `./vis explain <mã>` | giải thích một mã lỗi (vi/en/ja) |
| `./vis lsp --stdio` | bật gợi ý cho trình soạn thảo (VS Code…) |
| `./vis pkg add <gói>` | thêm gói (mô hình phi tập trung, `vis.lock`) |

## Cấu trúc repo

```
crates/    8 crate Rust: từ vựng, kiểu, thông dịch, máy ảo, CLI…
bin/       công cụ: vis, vis-dev, vis-fmt, vis-remote…
test/      bộ kiểm: ví dụ chạy được, fixture, báo cáo
docs/      đặc tả ngôn ngữ (SPEC-lex.md) và thiết kế
vi-du/     ví dụ chạy được cho người mới
agent/     sổ quyết định, QUEUE, nhật ký, báo cáo từng đợt
```

## Góp ý / phát triển

- Mọi thay đổi phải qua cổng: `./vis-dev gate --fast`.
- Quy ước commit: `[ST-##] <tóm tắt>`; quyết định thiết kế ghi trong `agent/DECISIONS.md`.
- Đọc [`AGENTS.md`](AGENTS.md) và [`CONTRIBUTING.md`](CONTRIBUTING.md) trước khi sửa mã.
- Bảng đối chiếu chi tiết hơn (ViScript ↔ Python ↔ Rust ↔ TS/JS): [`CHEAT-SHEET-ViScript.md`](CHEAT-SHEET-ViScript.md).

## Giấy phép

MIT OR Apache-2.0 — dùng, sửa, chia sẻ tự do.
