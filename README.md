<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ViScript

**ViScript là một ngôn ngữ lập trình dễ học, viết cho người Việt.** Câu lệnh đọc gần như tiếng Việt, thông báo lỗi cũng bằng tiếng Việt, và những thứ dễ gây lỗi trong các ngôn ngữ khác (con trỏ, `unsafe`, kế thừa lớp) thì ViScript **không có**.

> Mục tiêu: học sinh cấp 3 đọc 10 phút là viết được chương trình đầu tiên; người mới vẫn được nhắc lỗi rõ ràng thay vì những dòng khó hiểu.

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

## ViScript khác Python và Rust ở đâu?

| | Python | Rust | ViScript |
|---|---|---|---|
| Dễ đọc | ✅ | ⚠️ nhiều ký hiệu | ✅ thụt lề, câu lệnh ngắn |
| An toàn bộ nhớ | ⚠️ chạy mới biết | ✅ | ✅ không con trỏ, không `unsafe` |
| Tốc độ | chậm | rất nhanh | nhanh (máy ảo bytecode) |
| Đơn vị đo trong kiểu | ❌ | ❌ (cần crate) | ✅ `1kb + 1kib = 2024b` |
| Thông báo lỗi tiếng Việt | ❌ | ❌ | ✅ kèm mã tra cứu |

Nói ngắn: **cú pháp gần Python, độ an toàn gần Rust, thêm đơn vị đo và lỗi tiếng Việt.**

---

## 1. Xin chào

**ViScript**
```vis
print("Xin chào, ViScript!")
```

**Python**
```python
print("Xin chào, ViScript!")
```

**Rust**
```rust
fn main() {
    println!("Xin chào, ViScript!");
}
```

> ViScript không cần `fn main`, không cần `;`, không cần ngoặc nhọn.

## 2. Biến và nội suy chuỗi

**ViScript**
```vis
let ten = "Minh"
print("Chào {ten}!")
```

**Python**
```python
ten = "Minh"
print(f"Chào {ten}!")
```

**Rust**
```rust
let ten = "Minh";
println!("Chào {ten}!");
```

> `let` là bất biến (không đổi giá trị). Muốn đổi thì dùng `var`.

## 3. Hàm

**ViScript**
```vis
fn binh_phuong(x: int) -> int:
    x * x

print(binh_phuong(7))     # 49
```

**Python**
```python
def binh_phuong(x: int) -> int:
    return x * x

print(binh_phuong(7))
```

**Rust**
```rust
fn binh_phuong(x: i64) -> i64 {
    x * x
}

fn main() {
    println!("{}", binh_phuong(7));
}
```

> Thân hàm thụt lề; dòng cuối là giá trị trả về (không cần `return`).

## 4. Danh sách: map / filter

**ViScript**
```vis
let ds = [1, 2, 3, 4]
print(ds.map(x => x * 2))      # [2, 4, 6, 8]
print(ds.filter(x => x > 2))   # [3, 4]
```

**Python**
```python
ds = [1, 2, 3, 4]
print([x * 2 for x in ds])
print([x for x in ds if x > 2])
```

**Rust**
```rust
let ds = vec![1, 2, 3, 4];
let nhan_doi: Vec<i64> = ds.iter().map(|x| x * 2).collect();
let lon_hon_2: Vec<&i64> = ds.iter().filter(|x| **x > 2).collect();
println!("{nhan_doi:?} {lon_hon_2:?}");
```

> ViScript viết như bạn nghĩ, không cần `collect()` hay `|x|`.

## 5. Đơn vị đo — điểm riêng của ViScript

**ViScript**
```vis
print(1kb + 1kib)     # 2024b
print(2h + 30m)       # 150m  (R10 sẽ in "2h30m")
```

**Python** — không có đơn vị, phải tự quy đổi:
```python
kb, kib = 1000, 1024
print(kb + kib)       # 2024 — nhưng là byte hay KB? Không ai kiểm
```

**Rust** — không có đơn vị trong ngôn ngữ, cần crate `uom`:
```rust
// phải thêm crate uom, khai báo đơn vị, rồi mới cộng được
```

> Trong ViScript, đơn vị là **một phần của kiểu**: cộng sai loại là lỗi biên dịch, không phải lỗi lúc chạy.

## 6. Giá trị có thể thiếu (optional)

**ViScript**
```vis
let ds = [10, 20, 30]
let o = ds.get(1)          # some(20)
if let v = o:
    print("có {v}")        # có 20
else:
    print("không có")
```

**Python**
```python
ds = [10, 20, 30]
o = ds[1] if len(ds) > 1 else None
if o is not None:
    print(f"có {o}")
else:
    print("không có")
```

**Rust**
```rust
let ds = vec![10, 20, 30];
if let Some(v) = ds.get(1) {
    println!("có {v}");
} else {
    println!("không có");
}
```

> `if let v = o:` lấy giá trị bên trong nếu có, không thì chạy `else`.
> (Bản đầy đủ `if let Some(v) = o:` sẽ có ở pha 1 — hiện dùng dạng trên.)

## 7. Bản ghi (record) — thay cho class

**ViScript**
```vis
type Diem:
    x: int
    y: int

let p: Diem = {x: 1, y: 2}
print(p.x + p.y)      # 3
```

**Python**
```python
from dataclasses import dataclass

@dataclass
class Diem:
    x: int
    y: int

p = Diem(1, 2)
print(p.x + p.y)
```

**Rust**
```rust
struct Diem { x: i64, y: i64 }

fn main() {
    let p = Diem { x: 1, y: 2 };
    println!("{}", p.x + p.y);
}
```

> Không có class, không kế thừa — chỉ có bản ghi (dữ liệu) và hàm (hành vi).

## 8. Xử lý lỗi

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

**Python**
```python
def chia(a: int, b: int) -> int | None:
    if b == 0:
        return None
    return a // b

kq = chia(10, 2)
if kq is None:
    print("chia lỗi")
else:
    print(kq)
```

**Rust**
```rust
fn chia(a: i64, b: i64) -> Option<i64> {
    if b == 0 { return None; }
    Some(a / b)
}

fn main() {
    let Some(kq) = chia(10, 2) else {
        println!("chia lỗi");
        return;
    };
    println!("{kq}");
}
```

> `let … else` buộc bạn xử lý trường hợp thiếu **ngay tại chỗ**, không quên.

---

## Khi viết sai thì sao?

ViScript báo lỗi bằng tiếng Việt, kèm **mã lỗi** để tra cứu:

```
bai.vis:2:5: error VS0412: toán tử `+`: `str` và `str` không cùng kiểu/đơn vị
```

Gõ `./vis explain VS0412` để xem giải thích và cách sửa.

## Các lệnh hay dùng

| Lệnh | Việc |
|---|---|
| `./vis run <tệp.vis>` | chạy chương trình |
| `./vis check <tệp…>` | kiểm tra lỗi mà không chạy |
| `./vis fmt --write <tệp…>` | tự căn lại thụt lề, khoảng trắng |
| `./vis explain <mã>` | giải thích một mã lỗi (vi/en/ja) |
| `./vis lsp --stdio` | bật gợi ý cho trình soạn thảo (VS Code…) |
| `./vis pkg add <gói>` | thêm gói (đang phát triển) |

## Vì sao gọi là “tối ưu công thái học”?

“Công thái học” ở đây nghĩa là **thiết kế cho người viết dễ chịu và ít lỗi**, không phải chạy nhanh hơn một chút.

| Điểm | Trạng thái |
|---|---|
| Thụt lề thay ngoặc nhọn, câu lệnh ngắn | **đã có** |
| Thông báo lỗi tiếng Việt + mã tra cứu (`vis explain`) | **đã có** |
| Nội suy chuỗi `{expr}` | **đã có** |
| Đơn vị đo trong kiểu (`1kb + 1kib = 2024b`) | **đã có** |
| `if let` / `let … else` cho giá trị có thể thiếu | **đã có** |
| Đường dẫn viết thô `p"C:\thu\muc"` (không phải escape) | **đã có** |
| Tự căn định dạng (`vis fmt`), gợi ý trong editor (LSP) | **đã có** |
| Không con trỏ, không `unsafe`, không kế thừa lớp | **đã có** |
| `str + str` cho phép cộng chuỗi | **đang làm** (pha 1) |
| `let` che biến cùng phạm vi (không báo lỗi) | **đang làm** (pha 1) |
| `%` luôn trả số không âm (`-7 % 2 == 1`) | **đang làm** (pha 1) |
| Chia hai đơn vị cùng loại ra số, chia số ra đơn vị | **đang làm** (pha 1) |
| `if let Some(v) = o:` (pattern đầy đủ) | **đang làm** (pha 1) |
| Iterator lười `.lazy()` cho danh sách lớn | **dự kiến** (pha 2) |
| Gán lồng sâu `u.addr.city = "A"` | **dự kiến** (pha 2) |

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

- Mọi thay đổi phải qua cổng: `./vis-dev gate --fast` (định dạng, clippy, luật cấu trúc, toàn bộ test).
- Quy ước commit: `[ST-##] <tóm tắt>`; quyết định thiết kế ghi trong `agent/DECISIONS.md`.
- Đọc [`AGENTS.md`](AGENTS.md) và [`CONTRIBUTING.md`](CONTRIBUTING.md) trước khi sửa mã.

## Giấy phép

MIT OR Apache-2.0 — dùng, sửa, chia sẻ tự do.
