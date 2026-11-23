<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-234 — Audit parity interp ↔ VM: 9 lệch **xác nhận** (chỉ đo, KHÔNG sửa)

**Ngày**: 2026-11-20 · **HEAD đo**: `4ee06c3` (+ sửa ST-233 chưa commit) · **Máy**: Linux x86_64,
kernel `7.0.0-43-generic`, rustc `1.99.1` · **Binary**: `target/debug/vis` (dựng sau ST-230; mã ngôn ngữ
không đổi từ `4ee06c3`).

## 0. Cách đo (tái lập)

```bash
VIS=/home/ps/ViScript/target/debug/vis
$VIS run  agent/JOURNAL/st234-repro/<ca>.vis
$VIS run --vm --no-cache agent/JOURNAL/st234-repro/<ca>.vis
```

Mỗi ca dưới đây được **hai lần** chạy độc lập: (a) agent audit đọc mã + chạy; (b) phiên này chạy lại
cả hai engine và đối chiếu byte. **Tất cả 9 ca đều tái hiện** — không ca nào là suy đoán.

## 1. Bảng tổng hợp

| # | Ca | interp | VM | Bên sai | Sửa (ước lượng) |
|---|---|---|---|---|---|
| 1 | `for` lồng + `break` | `0 1 done`, exit 0 | lặp vô hạn tới `VS0501 chạy quá lâu (>10^7 lệnh)`, exit 101 | **VM** | 80–150 dòng (pop iterator khi break) |
| 2 | `while let` trong `if` | `after`, exit 0 | codegen lỗi: `hai đường tới với độ sâu 1 và 0`, exit 1 | **VM** | 10–20 dòng (Pop trước `end`) |
| 3 | `let` cấp tệp dùng trong hàm | `101`, exit 0 | `Add chưa hỗ trợ cho 1 và ()`, exit 101 | **VM** (interp đúng mô hình script phẳng) | 100–200 dòng (global slot) |
| 4 | `const` cấp tệp | `10 11`, exit 0 | `()` rồi `VS0501`, exit 101 | **VM** | 20–40 dòng (inline const) |
| 5 | `?.` trên base optional | `An` (thiếu `Some`) | `Some(An)` | **interp** | 10–30 dòng |
| 6 | `?.` trên base **không** optional | `An`, `2`, exit 0 | `Some(An)` rồi `Some(An) không có phương thức len`, exit 101 | **VM** + cần chốt checker | 10–30 dòng |
| 7 | `match` rơi khỏi mọi nhánh | trap `VS0501 match không khớp nhánh nào`, exit 101 | `after`, exit 0 (im lặng) | **VM** | 30–60 dòng (trap) |
| 8 | closure ghi biến bắt `var` | `11 21 1` | `11 11 1` | **VM** (mất trạng thái) | 60–120 dòng (ô chia sẻ) |
| 9 | or-pattern `1 | x` | `2` | `()` | **VM** (bind vế đầu) | 30–50 dòng |
| 10 | `set{1,2,2,3,1}` | `[1, 2, 3]` | `[1, 2, 2, 3, 1]` | **VM** (không dedup) | 10–20 dòng |

(Ghi chú: bảng có **10** dòng vì ca `set` phát hiện thêm khi kiểm lại; sổ nợ đánh **#157..#166** —
xem §3.)

## 2. Chi tiết từng ca (chương trình + đầu ra thật)

### Ca 1 — `for` lồng + `break` (VM lặp vô hạn)

`for-long-break.vis`:
```vis
fn main():
    for i in 0..2:
        for j in 0..3:
            break
        print(i)
    print("done")
```
- interp: `0\n1\ndone\n`, exit 0.
- VM: in `0\n1\n1\n1\n…` (hơn 5,6 MB) tới `error VS0501: chạy quá lâu (>10^7 lệnh)`, exit 101.
- Nguyên nhân: `crates/vis-bytecode/src/codegen.rs:603-631` + `crates/vis-vm/src/vm.rs:840-858` —
  `break` không pop iterator của vòng trong; `IterNext` của vòng ngoài dùng iterator sai.

### Ca 2 — `while let` (VM từ chối module do stack lệch)

`while-let.vis`:
```vis
fn main():
    if true:
        var x: int? = None
        while let Some(v) = x:
            print(v)
    print("after")
```
- interp: `after\n`, exit 0.
- VM: stdout rỗng; stderr `vis run --vm: codegen lỗi: mã sinh ra không hợp lệ: hàm 0 pc 26: hai đường tới với độ sâu 1 và 0`; exit 1.
- Nguyên nhân: `codegen.rs:633-659` — nhánh sai của `cond()` không `Pop` trước `end`;
  `crates/vis-bytecode/src/verify.rs:93` bắt được.

### Ca 3 — `let` cấp tệp trong hàm (VM thấy `()`)

`let-cap-tep.vis`:
```vis
let base = 100

fn add(x: int) -> int:
    x + base

print(add(1))
```
- interp: `101\n`, exit 0.
- VM: `error VS0501: Add chưa hỗ trợ cho 1 và ()`, exit 101.
- Nguyên nhân: `codegen.rs:805-850` (biến cấp tệp thành ô cục bộ mới) + wrapper `main` `codegen.rs:305-340`;
  interp dùng env phẳng `interp.rs:637-660`.

### Ca 4 — `const` cấp tệp (VM bỏ qua)

`const-cap-tep.vis`:
```vis
const N: int = 10

fn f() -> int:
    N + 1

fn main():
    print(N)
    print(f())
```
- interp: `10\n11\n`, exit 0.
- VM: `()\n` rồi `VS0501: Add chưa hỗ trợ cho () và 1`, exit 101.
- Nguyên nhân: `codegen.rs:233-250` — vòng item không xử lý `HItem::Const`; interp nạp ở `interp.rs:224-228`.

### Ca 5 — `?.` trên base optional (interp thiếu `Some`)

`hoi-cham-opt.vis`:
```vis
type U:
    name: str
    age: int

fn main():
    let u: U? = Some({name: "An", age: 3})
    print(u?.name)
    let n = u?.name
    print(n)
    print(n ?? "?")
    let a: U? = None
    print(a?.name)
```
- interp: `An\nAn\nAn\nNone\n`, exit 0.
- VM: `Some(An)\nSome(An)\nAn\nNone\n`, exit 0.
- Kiểu của `u?.name` là `str?` ⇒ VM đúng, **interp thiếu `lift_opt(1)`** (`interp.rs:748-772`).

### Ca 6 — `?.` trên base **không** optional (VM bọc sai)

`hoi-cham-thuong.vis`:
```vis
type U:
    name: str

fn main():
    let u: U = {name: "An"}
    print(u?.name)
    let x: int = u?.name.len()
    print(x)
```
- interp: `An\n2\n`, exit 0.
- VM: `Some(An)\n` rồi `error VS0501: Some(An) không có phương thức len`, exit 101.
- `codegen.rs:1018-1046` luôn bọc; checker (`tyck/truong.rs`) cho base không-optional ⇒ **cần chốt**:
  cấm `?.` trên base không-optional (`VS0412`) hay coi là no-op.

### Ca 7 — `match` rơi khỏi mọi nhánh (VM im lặng)

`match-roi.vis`:
```vis
fn main():
    let o: int? = Some(2)
    match o:
        Some(1): print("one")
        None: print("none")
    print("after")
```
- interp: stdout rỗng; stderr `error VS0501: match không khớp nhánh nào (không xét hết bị lọt)`; exit 101.
- VM: `after\n`, exit 0 — **nuốt lỗi**.
- Nguyên nhân: `codegen.rs:710-753` (nhánh rơi chỉ `Pop`) + `:1448-1472` (`tra_gia_tri` đẩy `false`).
- Liên quan nợ cũ **#2** (`match` ngữ cảnh giá trị đẩy `false`) — nay có ca tái hiện.

### Ca 8 — closure ghi biến bắt `var` (VM reset)

`closure-ghi.vis`:
```vis
fn main():
    var x = 1
    let f = fn() -> int:
        x = x + 10
        x
    print(f())
    print(f())
    print(x)
```
- interp: `11\n21\n1\n`, exit 0.
- VM: `11\n11\n1\n`, exit 0.
- Nguyên nhân: `codegen.rs:355-370` (mỗi lần gọi `LoadCaptured` → `Assign` vào ô cục bộ mới);
  `vm.rs:441-461` copy `f.captured`.

### Ca 9 — or-pattern (VM bind vế đầu)

`or-pattern.vis`:
```vis
fn main():
    let n = 2
    match n:
        1 | x: print(x)
```
- interp: `2\n`, exit 0.
- VM: `()\n`, exit 0.
- Nguyên nhân: `crates/vis-bytecode/src/codegen/pattern.rs:232-239` dùng `alts.first()`.

### Ca 10 — `set` không dedup (VM)

`set-dedup.vis`:
```vis
fn main():
    print(set{1, 2, 2, 3, 1})
```
- interp: `[1, 2, 3]`, exit 0.
- VM: `[1, 2, 2, 3, 1]`, exit 0.

## 3. Vì sao differential 100k **không** bắt được (lỗ hổng độ phủ)

`bin/vis-diff` chạy **0 lệch** trên 100 000 chương trình (seed 20261115) trong phiên này, nhưng bộ sinh
không sinh các tổ hợp: `for` lồng + `break`; `while let` trong khối lồng; biến/`const` cấp tệp dùng
trong hàm; `?.` trên hai loại base; `match` có pattern lồng không phủ hết; closure ghi biến bắt;
or-pattern có bind; `set` literal trùng. ⇒ **0 lệch của differential không chứng minh parity**; cần mở
rộng bộ sinh (thuộc VS3/phase 2, cần A0 xếp lịch).

## 4. Trạng thái và việc cần làm

- **Không sửa dòng mã nào** trong ST-234; 10 ca ghi thành nợ **#157..#166** trong `agent/SO-NO.md`
  (đều **mở**), nhóm ưu tiên: ca 1 (treo/vô hạn) > ca 2 (VM từ chối module) > ca 4/3 (const/biến cấp
  tệp) > ca 7 (im lặng) > ca 8 > ca 5/6 > ca 9/10.
- Ca 6 cần **A0 chốt ngữ nghĩa** (`?.` trên base không-optional) trước khi sửa; các ca còn lại là
  sửa lỗi thuần (interp hoặc VM theo thiết kế đã có).
- Repro: `agent/JOURNAL/st234-repro/*.vis` (10 tệp, SPDX `#`).
