<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ViScript — Ca dùng dễ nhầm & thông báo lỗi thật (2026-11-19)

> **Agent 0 · dữ liệu công thái học, 2026-11-19.** Thu thập theo yêu cầu A0 ("15–20 ca dễ nhầm").
> **KHÔNG** sửa ngữ nghĩa, **KHÔNG** sinh golden, **KHÔNG** đổi SPEC. Mọi ca chạy thật bằng binary tại HEAD dưới đây.
>
> **HEAD**: `6fb72a7` (`main`) · binary: `/home/ps/ViScript/target/debug/vis` (debug) · máy: `ps` (Linux x86_64) · tải lúc đo: ~0,3–0,9 · **Hai engine được so từng ca** (`vis run` interp vs `vis run --vm --no-cache`).
> **Đo bởi Agent 2 (đợt C)**; hiện tượng lệch-byte **`int??`** được Agent 1 tái chạy độc lập tại `/tmp/opencode/ergo-c/verify-int-nested.vis` — khớp báo cáo. VM pattern-lồng (t19/t23) được Agent 1 tái chạy tại `/tmp/opencode/ergo-b/t19.vis`, `t23.vis` — khớp báo cáo.
> **Ràng buộc VD-241 §3**: không chỉnh gì liên quan ngữ nghĩa; chỉ ghi nhận để A0 đọc.
>
> Exit code quan sát: `vis run` = `0` ok · `2` lỗi biên dịch · `101` trap runtime · `vis check` = `0` ok / `1` có lỗi ·
> Các ca lệch hai engine (**LỆCH**) thuộc nhóm **BLOCKED-CHO-A0** — xem `agent/DUNG-CHO-A0.md` và câu hỏi cuối tệp.

- **Ngày**: 2026-11-19 · **HEAD**: `6fb72a7` · **Binary**: `/home/ps/ViScript/target/debug/vis`
- **Lệnh**: `vis run <f>` (engine **interp**) và `vis run --vm <f>` (engine **VM**); compile-lỗi chạy cả `vis check` (TAP).
- Exit code quan sát được: `vis run` = **0** (ok), **2** (lỗi biên dịch), **101** (trap runtime / crash); **check = 0** (ok, kể cả cảnh báo), **1** (có lỗi).
- Mọi ca đã CHẠY THẬT. Cùng lỗi biên dịch ⇒ hai engine in giống nhau (ghi "giống hai engine"). Thông báo nguyên văn ở stderr; check in dòng `# VSxxxx …` trên stdout sau TAP. Scratch: `/tmp/opencode/ergo-c/*.vis`.
- **Không sửa repo.** Khuyến nghị sửa chỉ ghi khi output gợi ý; không thì đánh dấu *(kinh nghiệm — chưa được tool gợi ý)*.

| # | Ca | vis check | vis run | Hai engine |
|---|---|---|---|---|
| 1 | `None` gán `int` | VS0412 (exit 1) | VS0412 (exit 2) | giống |
| 2 | `int??` (Option lồng) | exit 0 | chạy được — **hai engine lệch nhau** | **LỆCH** |
| 3 | record `{a = 1}` vs map `{"a": 1}` | VS0201 / exit 0 | VS0201 / print ok | giống |
| 4 | binding pattern trùng biến ngoài | exit 0 | ok, shadow đúng phạm vi | giống |
| 5 | `match` thiếu nhánh | VS0402 (exit 1) | exit 2 | giống |
| 6 | sai số tham số / tên | VS0401 | exit 2 | giống |
| 7 | closure không annot kiểu tham số | **exit 0 (pass)** | runtime crash VS0501 (exit 101) | giống |
| 8 | shadow builtin/module | exit 0 (+VS0211) | ok / `int` không phải hàm | giống |
| 9 | `?` sai chỗ | VS0412 (exit 1) | exit 2 | giống |
| 10 | field/key không tồn tại | **exit 0** (thiếu field/map) | runtime crash (exit 101) | giống |
| 11 | thụt lề / xuống dòng | VS0109 / VS0201 | exit 2 | giống |
| 12 | precedence & so sánh | VS0201 / VS0412 | exit 2 hoặc ok | giống |
| 13 | `chan(0)` / recv kênh rỗng | **exit 0** | VS0505 (exit 101) | giống |
| 14 | gọi hàm/tên không tồn tại | **exit 0 (pass)** | VS0501 "chưa hỗ trợ ở VS0" (101) | giống |
| 15 | `return None` trong fn `-> int` | VS0412 VD-238 | exit 2 | giống |
| 16 | `Some(x)` khi đã Option | exit 0 / VS0412 | nested honestly; VM lệch ở match | **LỆCH** |
| 17 | truyền `42` cho `int?` | exit 0 | lift ngầm ở biên hàm | giống |
| 18 | match sai biến thể enum | VS0403+VS0402 | exit 2 | giống |
| 19 | `let x = None` không annot | exit 0 | print None, match ok | giống |
| 20 | destructure tuple lên giá trị không phải tuple | **exit 0 (pass)** | VS0501 khác nhau hai engine | giống exit, **khác message** |

---

## 1 — `None` gán cho `int` (`c01.vis`)

**Ý định**: khai báo `x: int` rồi gán `None` (quên không annot `int?`).
```
fn main():
    let x: int = None
    print(x)
```
**Thực tế** (hai engine giống): biên dịch lỗi, không chạy.
- run: stderr `c01.vis:3:18: error VS0412: \`khai báo \`let\`\`: \`None\` chỉ gán được cho \`T?\`, được \`int\` (VD-238)` · exit **2**
- check: `not ok 1 - …` + `# VS0412 3:18 …(VD-238)` · exit **1**. Không có gợi ý `int?` trong message ngoài vế "chỉ gán được cho `T?`" — *(kinh nghiệm — chưa được tool gợi ý: annot `int?`)*.
- Cùng kiểu thông điệp ở ca 15 (`return`).

## 2 — `int??` (Option lồng, VD-178 R2) — **hai engine lệch nhau, VM im lặng sai** (`c02*.vis`)

> **CẬP NHẬT 2026-11-19 (VD-241 §3 + §7, A0 chốt — đợt Option lồng + fix `None`):** các hiện tượng đo lúc đó **đã được sửa theo đúng quyết định A0** — `T??` là `Option[Option[T]]` lồng thật (pattern khớp cấu trúc cả hai engine); `None` trần nay hợp lệ khi lớp **ngoài cùng** là `Option` (VD-238 gate sửa). Các số đo/repro trong § này giữ nguyên **lịch sử** (bản ghi lúc 6fb72a7) — xem `agent/QD-OPTION-LONG-2026-11-19.md` và `VISCRIPT-ERGONOMICS-FACTS.md` (CẬP NHẬT 2026-11-19).

**Ý định**: dùng `int??` như "optional 2 tầng" (được phép theo VD-178 R2: hậu tố `?` đếm 2 lớp).
```
let x: int?? = Some(Some(42))   # hoặc Some(42), hoặc 42, hoặc None — xem dưới
match x:
    Some(Some(v)): print("hai {v}")
    Some(v):      print("mot {v}")
    None:         print("none")
```
**Thực tế**:
- Cả hai engine luôn **print giống nhau** (`Some(Some(42))` hoặc `Some(42)`, đúng giá trị gán) — nhưng **match lệch nhau**:
  - `let x: int?? = Some(42)` / `= 42` / `= Some(Some(42))`: interp khớp arm **`Some(Some(v))`** (v=42); VM khớp arm **`Some(v)`** (v=42 hoặc v=`Some(42)` tùy nguồn). Với `= Some(42)`: interp in `x=Some(42)` (một tầng) rồi vẫn vào arm hai tầng — **print và match của interp không nhất quán với nhau**.
  - `let x: int?? = None` → interp arm `None` (in `arm3`); **VM crash**: `c02k.vis:4:5: error VS0501: không lấy được đối số thứ 0` · exit **101**.
  - arm1 `Some(Some(v))` + `None` (`c02e2.vis`, không có arm2): **VM không in gì, exit 0** — kết quả sai **im lặng**; interp in `hai 42`.
- Qua biên hàm (`c02h.vis`, `f(x: int??)` in thứ tự `f(Some(42))`/`f(Some(Some(42)))`/`f(None)`; arm1→1, arm2→2, arm3→3): interp in `1 1 3`; VM in `2 2` rồi **trap** tại `match x:` (`c02h.vis:3:5: error VS0501: không lấy được đối số thứ 0`) · exit **101**.
- Bytecode VM (`--dump-bytecode`, ca `c02e2.vis`): arm1 sinh `JumpIfFalse(15)` trỏ vào **chính** `Unwrap(0)` ở đầu thân arm1 — hai nhánh hội tụ về cùng đích, điều kiện arm1 bị nuốt (codegen VM hỏng cho optional lồng). Đây là **bằng chứng bytecode**, không phải suy đoán.
- `vis check`: exit **0** cho mọi biến thể trên (không chặn).
- Quyết định hiện hành (VD-227/228) chỉ nói lift 1 tầng (`T→T?`); **không có VD nào phủ ngữ nghĩa `int??`** — hai engine tự do lệch.
- **Khuyến nghị (kinh nghiệm — chưa được tool gợi ý)**: đừng dùng `T??` trong mã người dùng; annot 1 tầng `int?` và xử lý lồng bằng `match` từng lớp.

## 3 — Record `{a = 1}` vs map `{"a": 1}` — khóa bare là **tên trường**, không nội suy biến (`c03*.vis`)

**Ý định**: tạo map/record nhanh; hoặc nghĩ `{ten: 1}` lấy `ten` làm giá trị của biến.
```
let r = {a = 1}    # = thay :
let m = {"a": 1}   # map, đúng
let ten = "truong"; let r2 = {ten: 1}
```
**Thực tế**:
- `{a = 1}`: `c03.vis:3:16: error VS0201: mong \`:\`, thật \`=\` · exit **2** (hai engine giống; check exit 1). Sửa theo lỗi: dùng `:`.
- `{"a": 1}` + `m["a"]` → in `{"a": 1}` / `1` · exit **0**.
- `{ten: 1}` (bare key) → in `{ten: 1}` — **`ten` là tên trường cố định**, KHÔNG phải biến. Không ai báo lỗi/cảnh báo. *(kinh nghiệm — chưa được tool gợi ý: muốn key động thì dùng map với khóa chuỗi và `m.get(...)`)*.

## 4 — Binding trong pattern trùng biến ngoài — shadow chỉ trong arm (`c04.vis`)

**Ý định**: dùng `x` vừa làm biến ngoài vừa làm binding của `A(x)`.
```
let x = 99
match H.A(42):
    A(x): print("A {x}")
print("ngoai {x}")
```
**Thực tế**: in `A 42` rồi `ngoai 99` — binding arm **đè** `x` chỉ trong arm; ngoài arm `x = 99`. exit **0**, check exit 0, hai engine giống. Không cảnh báo nào — hợp lệ hoàn toàn, nhưng dễ đọc nhầm là "gán đè biến ngoài".

## 5 — `match` thiếu nhánh (`c05.vis`, `c05b.vis`)

**Ý định**: liệt kê vài arm, quên `B` (enum 2 biến thể) / quên `None` (Option).
```
enum H: A(x: int) / B(y: int)
match h:
    A(x): x
```
**Thực tế**:
- `c05.vis:6:5: error VS0402: \`match\` thiếu nhánh: B` · exit **2** (check: `not ok …` + `# VS0402 6:5 …thiếu nhánh: B` · exit 1).
- Option: `c05b.vis:4:5: error VS0402: \`match\` thiếu nhánh: None` · exit **2**.
- Message chỉ ra **đúng tên nhánh thiếu** — sửa được ngay: thêm arm hoặc `_`/`B(_)`.
  - *(bonus)* arm trùng/chồng hoàn toàn (`A(x)` rồi `A(y)`, `c05c.vis`): **không có chẩn đoán nào** — exit 0, im lặng (arm thứ hai chết nhưng check pass).

## 6 — Sai số tham số / tên tham số lạ (`c06*.vis`)

**Ý định**: gọi `cong(1)` (thiếu), `cong(1,2,3)` (thừa), `chao(dau:"xin", ten:"An")` (đặt tên, sai thứ tự), `chao(nam: "An")` (tên không tồn tại).
**Thực tế** (hai engine giống; check in dòng `# VS0401` tương ứng):
- thiếu: `c06.vis:5:11: error VS0401: hàm \`cong\` cần 2 đối số (tối thiểu 2), được 1` · exit 2
- thừa: `c06b.vis:5:11: error VS0401: … được 3` · exit 2
- named tùy thứ tự: in `xin An` · exit **0** — hợp lệ, không cảnh báo.
- tên lạ: **hai** lỗi: `…được 1` + `c06d.vis:5:21: error VS0401: hàm \`chao\` không có tham số tên \`nam\` · exit 2.
- Không có gợi ý "ý bạn là `ten`?" — chỉ liệt kê lỗi, người dùng tự dò. *(kinh nghiệm)*

## 7 — Closure không annot kiểu tham số — **check PASS, runtime crash** (`c07*.vis`)

**Ý định**: `let f = fn(x): x + 1` (không annot `x`), hoặc thân closure không suy luận được kiểu.
```
let f = fn(x): x + "chuoi"   # x không annot; x thực tế int
print(f(1))
```
**Thực tế**:
- `fn(x): x + 1`, `fn(x): print(x)`, `fn(x): x` — **chạy được** (tham số nhận `Type::Error`, mọi phép trên `Error` bị nuốt) · exit 0.
- `fn(x): x + "chuoi"`: **`vis check` exit 0 (ok 1)** nhưng cả hai engine trap: `c07d.vis:3:20: error VS0501: \`Add\` chưa hỗ trợ cho \`1\` và \`chuoi\` ở VS0` · exit **101**.
- Closure unannotate khi **không** có mặt use-site: `print(f)` → interp in `<fn>`, VM in `<hàm>` — **lệch Show** (cosmetic, exit 0).
- Annot đầy đủ `fn(x: int) -> int: x + 1` → in `42`, exit 0.
- Thông điệp `VS0501 … ở VS0` dẫn người dùng nghĩ "tính năng chưa hỗ trợ" thay vì "kiểu không suy luận được". *(kinh nghiệm — chưa được tool gợi ý: annot kiểu tham số closure)*

## 8 — Shadow builtin/module (`c08*.vis`)

**Ý định**: đặt tên biến `print`, `len`, `std`, `list`.
```
let print = 42
print(print)
```
**Thực tế**:
- `print` bị đè: `c08.vis:4:5: error VS0412: \`int\` không phải hàm` · exit 2 (lỗi ở **chỗ gọi**, không ở chỗ khai báo).
- `let len = 42`: in `42` · exit 0 (không cảnh báo).
- `let std = 42`: in `42` · exit 0 (không cảnh báo).
- `use std.list` + `let list = 42`: in `42` · exit 0, kèm **warning VS0211** trên stderr: `c08d.vis:4:9: warning VS0211: tên \`list\` là module std đã import — \`list.xxx(...)\` được hiểu là thành viên của giá trị, không phải module`. check exit 0 (warning không fail). Hai engine giống hệt.

## 9 — `?` sai chỗ — non-Result / `main` không trả Result (`c09*.vis`)

**Ý định**: `let x = 40 + 2?` hoặc `let v = f()?` trong `fn main` (không annot `-> Result/optional`).
**Thực tế** (hai engine giống):
- non-Result: `c09.vis:3:18: error VS0412: \`?\` chỉ dùng cho \`T?\`/\`Result\`, được \`int\` · exit 2.
- trong main (f trả Result): `c09b.vis:5:13: error VS0412: \`?\` trên \`Result\` chỉ dùng được khi hàm trả \`Result[…, …]\`, hàm này trả \`()\` (§4.4)` · exit 2. Message **nói rõ điều kiện của hàm chứa** — sửa được: annot `-> Result[int, str]` cho main hoặc bóc bằng `match`/*(kinh nghiệm)*.
- `?` trên `int?` trong main: `c09c.vis:5:13: error VS0412: \`?\` trên \`int?\` chỉ dùng được khi hàm trả \`int?\`, hàm này trả \`()\` (§4.4)` · exit 2.

## 10 — Field/key không tồn tại (`c10*.vis`)

**Ý định**: `U(name: "An")` thiếu `age`; truy cập field gõ sai `u.ag`; `m["khong-co"]` vs `m.get(...)`.
```
type U: name: str / age: int
let u = U(name: "An")
print(u.age)
```
**Thực tế**:
- **Thiếu field lúc dựng**: `U(name: "An")` → in `U(name: An)` · exit 0, check `ok 1` (**exit 0**) — **không chẩn đoán**. Nhưng `u.age` về sau: cả hai engine trap `c10b.vis:7:11: error VS0501: bản ghi không có trường \`age\` · exit **101**, check vẫn `ok 1`.
- Field gõ sai trên record đầy đủ: `c10c.vis:7:11: error VS0405: bản ghi không có trường \`ag\` (VD-43 #6)` · exit 2.
- map key thiếu: `m.get("khong-co")` → in `None`, chương trình chạy tiếp · exit 0. `m["khong-co"]` → in trước stdout `None` rồi trap `c10d.vis:5:11: error VS0503: khoá \`khong-co\` không có trong map` · exit **101**, check `ok 1`.
- Tóm lại: **mọi lỗi "thiếu key/field" chỉ nổ ở runtime; `vis check` không chặn**; chỉ field tồn tại-sai-chính-tả-kiểu-tĩnh mới là VS0405 ở check. *(kinh nghiệm — chưa được tool gợi ý: dùng `.get(...)` và `match`/`??` hoặc annot record đầy đủ)*

## 11 — Thụt lề / xuống dòng bất thường (`c11*.vis`)

**Ý định**: indent 2 cột; rớt dòng sau `let x =`; tab/INDENT lạ trong block.
```
fn f(a: int) -> int:
  let x = a + 1   # 2 cột
  x
```
**Thực tế**:
- Indent 2 cột: chạy đúng (in `2`), hai engine giống; nhưng `vis fmt --check` exit **1** — chỉ **fmt** coi là lệch (run/check đều exit 0).
- `let x =` rồi NEWLINE: `c11b.vis:3:12: error VS0201: mong biểu thức, thật \`<NEWLINE>\` · exit 2.
- INDENT lạ giữa block (`   let y` ở cột 3): **cascade**: `c11d.vis:4:1: error VS0109: thụt lề không khớp đơn vị/DEDENT sai` + `c11d.vis:5:1: error VS0201: mong biểu thức, thật \`    \` (INDENT)` · exit 2 (lỗi đầu chỉ vị trí cột-lệcht, lỗi sau là hệ quả — đừng sửa lỗi thứ hai trước).
- Xuống dòng **trong ngoặc** (`cong(a: int,`\n`    b: int)`): hợp lệ, exit 0.

## 12 — Precedence & so sánh (`c12*.vis`)

**Ý định**: `1 + 2 * 3`; `o ?? 10 + 5`; `not a and b`; `x == 1 == y`; `1 == "1"`.
**Thực tế** (hai engine giống):
- `1 + 2 * 3` → `7`. `o ?? 10 + 5` → `15` (tức `??` **lỏng hơn** `+`). `o ?? 0 == 3` → `true` (tức `??` **chặt hơn** `==`). `not a and b` → `(not a) and b` (in `false` khi a=true,b=false; khác `not (a and b)` = `true` — `not` chặt hơn `and`).
- `x == 1 == y`: `c12c.vis:5:18: error VS0201: toán tử so sánh không kết hợp — thêm ngoặc (A.5); thật \`==\` · exit 2. Message **nói rõ phải thêm ngoặc**.
- `1 == "1"`: `c12d.vis:3:11: error VS0412: so sánh khác kiểu: \`int\` và \`str\` · exit 2.

## 13 — `chan(0)` và recv kênh rỗng — **runtime VS0505** (`c13*.vis`)

**Ý định**: `chan[int](0)` (rendezvous), `recv()` khi kênh rỗng không task nào chạy, `close()` rồi `recv()`.
```
let c = chan[int](0)
let _ = c.send(1)
```
**Thực tế** (hai engine giống; **check exit 0 cho cả ba ca** — không chặn được):
- `c13.vis:3:20: error VS0505: \`chan[T](0)\`: kênh capacity 0 (rendezvous) cần scheduler thật — chưa hỗ trợ ở vòng này; dùng capacity ≥ 1` · exit **101**. (Tài liệu thiết kế §9 nói chan(0) bị từ chối bằng VS0505 — đúng; VD-178 QĐ-2 nói "chan(0) hợp lệ = rendezvous" nhưng **mã hiện hành từ chối** — code là sự thật.)
- `recv` kênh rỗng, không task khác: `c13c.vis:4:11: error VS0505: deadlock: \`recv\` trên kênh **rỗng** và không còn tác vụ nào chạy được` · exit **101** (không treo — detector bắn).
- `close()` rồi `recv()`: in `None` · exit **0** (recv trên kênh đã đóng trả `None`, không phải lỗi).
- Lưu ý cách API thật: `chan` trả **một giá trị `<kênh>`** (không phải tuple `(send, recv)` như thiết kế §742); dùng `c.send/c.recv`. Đừng destructure (xem ca 20).

## 14 — Gọi hàm / tên không tồn tại — **check PASS, runtime VS0501** (`c14*.vis`)

**Ý định**: gõ sai tên `tinh_tong2` (ý là `tinh_tong`).
```
fn tinh_tong(a: int, b: int) -> int: a + b
print(tinh_tong2(1, 2))
```
**Thực tế**: **`vis check` exit 0 (`ok 1`)**; cả hai engine trap tại điểm gọi:
`c14.vis:5:11: error VS0501: tên \`tinh_tong2\` chưa hỗ trợ ở VS0` · exit **101**.
- Biến lạ cũng vậy: `print(bien_khong_co)` → check exit 0, `c14b.vis:3:11: error VS0501: tên \`bien_khong_co\` chưa hỗ trợ ở VS0` · exit 101.
- Thông điệp **không** gợi ý tên gần đúng; chữ "chưa hỗ trợ ở VS0" làm người dùng tưởng tính năng chưa có, không phải "tên không tồn tại". `vis explain VS0501` chỉ in một dòng `VS0501 — lỗi: lỗi runtime: phép toán hoặc tính năng chưa hỗ trợ`, không có gợi ý sửa. Không truncation/"did you mean" nào. *(kinh nghiệm: rà lại chính tả tên hàm; annot đầy đủ để checker bắt sớm — nhưng check vẫn không bắt được trường hợp này)*

## 15 — `return None` / sai kiểu trong fn trả `int` — VD-238 (`c15*.vis`)

**Ý định**: `return None` (quên `int?`), hoặc trả `str`.
```
fn f() -> int:
    return None
```
**Thực tế** (hai engine giống):
- `c15.vis:3:5: error VS0412: \`return\`: \`None\` chỉ hợp lệ ở \`T?\`, được \`int\` (VD-238)` · exit 2 (check exit 1, `# VS0412 3:5 …(VD-238)`).
- Thân cuối là `None` trần (`fn f() -> int:` + dòng `None`): cùng thông điệp `…(VD-238)` tại `c15c.vis:3:5` · exit 2.
- Trả `str`: `c15b.vis:3:5: error VS0412: \`return\`: cần \`int\`, được \`str\` · exit 2.
- Message VD-238 chỉ rõ ràng buộc `T?` — sửa được: đổi annot thành `int?` *(khuyến nghị trùng với chính message)*.

## 16 — `Some(x)` khi `x` đã Option — nested "thành thật", nhưng VM lệch ở match (`c16*.vis`)

**Ý định**: `let y = Some(x)` với `x: int?` — lo double-wrap.
```
let x: int? = Some(42)
let y = Some(x)
```
**Thực tế**:
- Không collapse: y = `Some(Some(42))` (in chứng) · exit 0 — kiểu `int??` đẩy lồng lên tự nhiên.
- `Some(x)` trả về từ fn annot `-> int?`: `c16b.vis:4:5: error VS0412: giá trị trả về: cần \`int?\`, được \`int??\` · exit 2.
- Gán `int??` cho `int?`: `c16c.vis:3:19: error VS0412: khai báo \`let\`: cần \`int?\`, được \`int??\` · exit 2.
- **Match nested Option**: arm1 `Some(Some(v))` + arm2 `Some(v)`: interp in arm1 (`hai 42`); **VM in arm2 (`mot`)** — và khi chỉ có arm1 + None (`c02e2.vis`): **VM in không gì, exit 0** (im lặng). `vis check` `ok 1` cho mọi biến thể.
- Không double-wrap âm thầm (khác Rust `Some(Some(x))` vs `.flatten()` — ở đây kiểu là `int??` tường minh).
- Tóm: lồng `Some` ở **dữ liệu** thì thành thật; lồng ở **pattern/codegen** thì VM hỏng (xem ca 2). *(kinh nghiệm — chưa được tool gợi ý: tránh int?? hoàn toàn)*

## 17 — Truyền `42` cho tham số `int?` — lift ngầm ở biên hàm (`c17*.vis`)

**Ý định**: `f(42)` với `f(x: int?)`; `f("chuoi")`.
```
fn f(x: int?) -> str: match x: Some(v): "some {v}" / None: "none"
print(f(42))
```
**Thực tế** (hai engine giống; check khớp):
- `f(42)` → in `x=Some(42)` bên trong, tức **đối số plain được nâng thành `Some(42)` ở biên hàm** · exit 0. (Code: `opt_lift_maybe(v, matches!(p.ty, Type::Opt(_)))` — "VD-228b". LƯU Ý: văn bản VD-228 (DECISIONS.md) nói "đối số … **chưa nâng**", nhưng mã đang nâng — tài liệu và mã lệch nhau, code thắng.)
- `Option/Result` **không** bị bọc hai lớp khi truyền (tính theo code: giữ nguyên `Some/None/Ok/Err`).
- `f("chuoi")`: `c17b.vis:7:13: error VS0412: đối số: cần \`int?\`, được \`str\` · exit 2.
- `f(None)` → `none` · exit 0.

## 18 — Match sai tên biến thể enum (`c18.vis`)

**Ý định**: arm `Vang` không tồn tại trong `enum Mau` (chỉ có `Do`, `Xanh`).
```
match m:
    Vang: print("vang")
    Xanh: print("xanh")
```
**Thực tế**:
- **Hai lỗi cùng lúc**: `c18.vis:8:9: error VS0403: enum \`Mau\` không có biến thể \`Vang\` · exit 2; và `c18.vis:7:5: error VS0402: \`match\` thiếu nhánh: Do` · exit 2.
- `vis check` chỉ in **lỗi đầu** (`# VS0403 …`) — người dùng sửa Vang→Do rồi mới thấy VS0402 (thiếu Do) ở lần chạy sau; sửa theo thứ tự lỗi không tối ưu.

## 19 — `let x = None` không annot — kiểu gì, in ra sao (`c19.vis`)

**Ý định**: bỏ annot, để checker suy `None`.
```
let x = None
print(x)
match x: None: print("none-arm") / _: print("other")
```
**Thực tế**: in `None` rồi `none-arm` · exit 0, check exit 0, hai engine giống. Là **đường hợp lệ duy nhất** để "gỡ" giá trị None-trắng (VD-238 chặn mọi annot non-`T?`); match arm `None` nhận diện ổn. Không có cảnh báo "unused/untyped None".

## 20 — Destructure tuple lên giá trị không phải tuple — check pass, runtime crash, **hai engine hai message** (`c20*.vis`, `c13b.vis`)

**Ý định**: `let (a, b) = 42` — hoặc nhầm cú pháp tuple-destructure lên kênh: `let (tx, rx) = chan[int](1)` (thiết kế §742 từng viết chan trả `(Sender, Receiver)`).
**Thực tế** (destructure tuple thật `(3,4)` từ biểu thức/hàm: **ok**, in `7`, exit 0):
- `let (a, b) = 42`: **`vis check` exit 0**; interp trap `c20c.vis:4:11: error VS0501: tên \`a\` chưa có giá trị lúc chạy` (lỗi tại **chỗ dùng** `a`); VM trap `c20c.vis:3:5: error VS0501: không chỉ số được \`42\` bằng \`0\` · exit **101** (lỗi tại **chỗ destructure**). Hai engine: **cùng exit 101, cùng VS0501, khác vị trí và nội dung message**.
- `let (a, b) = 42` vs `let (tx, rx) = chan[int](1)`: y hệt — interp `c13b.vis:4:11: error VS0501: tên \`rx\` chưa có giá trị lúc chạy`; VM `c13b.vis:3:5: error VS0501: không chỉ số được \`<kênh>\` bằng `0` · exit 101, check exit 0.
- Không có diagnostics "giá trị không phải tuple" ở check — checker permissive (VD-44 #1) với destructure.

---

## Tổng hợp finding lớn (ưu tiên xử lý)

1. **`int??`/optional lồng hỏng khác nhau hai engine** (ca 2, 16): interp mis-fire arm lồng; **VM crash VS0501 (exit 101) khi arm`None` duy nhất, và VM in zero-byte exit 0 im lặng khi chỉ arm `Some(Some(v))`**; `vis check` exit 0; `--dump-bytecode` cho thấy `JumpIfFalse` trỏ đúng entry fall-through của thân arm. Không có VD nào phủ `T??` semantics ⇒ đề nghị Agent 0/task riêng chốt (hoặc cấm annot `??` ở user code cho tới khi sửa codegen VM). **CẬP NHẬT 2026-11-19 (VD-241 §3/§7, A0 chốt): fixed** — `T??` = lồng thật, pattern khớp cấu trúc, byte-identical hai engine trên corpus `optional/long-*`; `None` hợp lệ khi lớp **ngoài cùng** là `Option` (sau VD-238 gate fix) — số đo cũ giữ lịch sử; xem `VISCRIPT-ERGONOMICS-FACTS.md` CẬP NHẬT 2026-11-19.
2. **`vis check` PASS (exit 0) cho nhiều lỗi chỉ nổ runtime**: gọi/đọc tên không tồn tại (ca 14), closure unannotate kiểu sai (ca 7), record dựng thiếu field (ca 10), map key thiếu `m["k"]` (ca 10), `chan(0)` (ca 13), destructure giá trị không phải tuple (ca 20). Message runtime `VS0501 … chưa hỗ trợ ở VS0` / `tên không có giá trị` làm người dùng hiểu nhầm.
3. **Thông điệp runtime không gợi ý sửa và lệch hai engine**: `<fn>` vs `<hàm>` (Show closure), destructure (ca 20), VS0501 chung chung. `vis explain VSxxxx` chỉ có 1 dòng, không có near-match/gợi ý.
4. **Tài liệu vs mã lệch**: thiết kế §742/VD-19 nói chan trả tuple / "đối số không nâng" — mã: chan trả một `<kênh>`, biên hàm **có** lift (VD-228b trong `goi.rs`); `chan(0)` VD-178 QĐ-2 "hợp lệ rendezvous" nhưng mã từ chối bằng VS0505. Báo cáo dựa trên hành vi chạy thật.
5. Ca nhỏ không có chẩn đoán: arm match chết (ca 5b), `{bare-key: …}` là tên trường cố định (ca 3), 2-cột indent chỉ fmt bắt (ca 11), `len`/`std` shadow im lặng (ca 8).

**Ràng buộc đã tôn trọng**: không sửa repo, không commit; mọi ca chạy bằng `/home/ps/ViScript/target/debug/vis` tại HEAD `6fb72a7`; stdout/stderr/exit code ghi nguyên văn; hai engine được so từng ca; vùng scratch duy nhất `/tmp/opencode/ergo-c/`.
