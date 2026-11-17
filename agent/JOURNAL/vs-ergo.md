<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS-ERGO · Công thái học ViScript — 7 điểm ma sát + VS0412
**Trạng thái**: **CHỜ AGENT 0 DUYỆT** (cùng đợt 6 tài liệu VS3, VD-167(5)) · **ST-172** · **Chỉ đọc, 0 dòng mã**
**Căn cứ**: VD-167; probe bằng `target/debug/vis` (binary của cổng L0 **1179/0 @ `0df923c2`**) ngày 2026-11-17.
**Quy ước đường dẫn (VD-165 §2)**: tính từ **gốc ViScript**.

## §0. Phương pháp
Mỗi điểm: **hiện trạng** (dẫn `file:dòng` + probe tái lập được) · **đề xuất** (1–2 phương án, rủi ro) ·
**ca đỏ-trước** · **ảnh hưởng** mã/test/tài liệu. Probe viết ra `/tmp/ergo/*.vis`, chạy `vis run`; kết quả dưới đây là **thật**,
không suy đoán. Các điểm là **đề xuất**, không phải quyết định thiết kế (VD-166: thiết kế thuộc Agent 0).

## §1. (a) `str + str`
**Hiện trạng**: `let s = "a" + "b"` ⇒ `VS0412: toán tử `+`: `str` và `str` không cùng kiểu/đơn vị`
(`crates/vis-types/src/tyck/expr.rs:501`; phép toán số ở `crates/vis-types/src/tyck/num.rs:5,21`).
**Đề xuất**: hai phương án —
1. **Giữ lỗi, thêm gợi ý** (khuyến nghị): thông điệp thêm `— dùng "{a}{b}" để ghép chuỗi` (một dòng, không đổi ngữ nghĩa).
2. Cho phép `+` nối chuỗi — **đổi ngôn ngữ** (nhập nhằng với cộng số, đơn vị), cần Agent 0 quyết và có số liệu dùng.
**Ca đỏ-trước**: `"a" + "b"` ⇒ VS0412 **và** chuỗi chẩn đoán chứa `{a}{b}` (so khớp thông điệp);
ca âm: `1 + "b"` phải **không** đổi mã.
**Ảnh hưởng**: `tyck/expr.rs:501` (một dòng), golden thông điệp (`crates/vis-cli/tests/*`), doc-examples (nếu có ví dụ in lỗi).

## §2. (b) Chỉ số: `xs[i]` và `s[i]`, `s[a:b]`
**Hiện trạng**:
- `xs[0]` trả **`T`** (không phải `T?`): `let y: int = xs[0]` OK, `xs[0] + 1` OK (probe `b1`,`b6`).
- Ngoài phạm vi ⇒ **`VS0503` lúc chạy** (`b4`: “chỉ số 5 ngoài phạm vi (độ dài 2)”).
- `s[0]` ⇒ **VS0412** kèm gợi ý `.chars()/.graphemes()/.byte_at(i)?` (`b2`; `VD-18`).
- `s[0:2]` ⇒ **VS0201** (chưa có cú pháp cắt chuỗi) (`b3`).
**Đề xuất**: **giữ** `xs[i]: T` + `VS0503` (đổi sang `T?` là **đổi thiết kế**, phá mọi chỗ dùng `int`);
với `str`: **giữ không có** `s[i]`/`s[a:b]` (đúng VD-18 — chuỗi là grapheme, cắt theo byte là bẫy), chỉ cải thiện **thông điệp**
`VS0201` khi gặp `s[a:b]` thành gợi ý `s.chars()` (nếu Agent 0 đồng ý).
**Ca đỏ-trước**: `xs[5]` ⇒ VS0503; `s[0]` ⇒ VS0412 có gợi ý; `s[0:2]` ⇒ VS0201 (hoặc gợi ý mới).
**Ảnh hưởng**: `tyck/expr.rs` (index/str), parser slice (nếu đổi VS0201), tài liệu VD-18.

## §3. (c) Pattern `Some(v)` / `some(v)` / `none`
**Hiện trạng**: **cả hai pattern đã chạy**: `let o = Some(1)` rồi `match o:` với `Some(v):` **và** `some(v):` đều in `1`
(probe `c6`, `c8`); `none` khớp (`c7`). **Constructor** chữ thường `some(1)` ⇒ `VS0501: tên `some` chưa hỗ trợ ở VS0` (`c4`)
— chỉ `Some(1)` (hoa) dựng được; `none` là literal.
**Đề xuất**: **giữ pattern như hiện tại** (đã thoả yêu cầu A0) và **thống nhất constructor**: hoặc chỉ `Some`/`None` (hiện tại,
khuyến nghị — khớp `Ok`/`Err`), hoặc cho phép `some`/`none` chữ thường (đổi ngôn ngữ). Ghi rõ trong tài liệu ngôn ngữ.
**Ca đỏ-trước** (khoá): pattern `Some(v)` và `some(v)` cho **cùng** kết quả trên cùng giá trị; `none`; constructor `some(1)` ⇒ VS0501
(hoặc xanh nếu Agent 0 cho phép).
**Ảnh hưởng**: `crates/vis-types/src/tyck/pat.rs:131` (arity `some`), parser pattern, docs.

## §4. (d) Giá trị mặc định của `type` khi dựng record
**Hiện trạng**: `type U:` khai trường không có giá trị mặc định; dựng `{name: "a", age: 1}` **đòi đủ** trường
(`crates/vis-types/tests/exhaustive.rs:151`). Chưa có cú pháp `field: T = expr` ⇒ **nợ**, không phải thiết kế mới.
**Đề xuất**: cho phép `field: T = expr` trong `type`; khi dựng thiếu ⇒ áp default; thiếu trường **không** default ⇒ lỗi rõ
(`VS0403`/`VS0412`). Đây là **thiết kế mới** ⇒ **Agent 0 quyết**; nếu đồng ý, cấp số nợ và tách task.
**Ca đỏ-trước**: `type U: n: int = 0` + `{name: "a"}` ⇒ `n == 0`; `type V: n: int` + `{}` ⇒ lỗi.
**Ảnh hưởng**: parser `type`, tyck record, lower/codegen, match exhaustive, tài liệu §type.

## §5. (e) `print` nhiều đối số
**Hiện trạng**: `print("a", "b")` ⇒ **VS0401** “cần 1 đối số, được 2” (`crates/vis-types/src/tyck/expr.rs:1303-1319`);
`print()` cũng VS0401.
**Đề xuất** (khuyến nghị): cho phép `print(a, b, …)` in cách nhau **một dấu cách**, newline cuối; `print()` in newline.
Rủi ro thấp (chỉ nới arity của một builtin), nhưng là **đổi ngôn ngữ** ⇒ Agent 0 quyết.
**Ca đỏ-trước**: `print("a","b")` ⇒ `a b
`; `print()` ⇒ `
`; `print(x)` giữ nguyên.
**Ảnh hưởng**: `tyck/expr.rs:1303`, interp print, golden, docs.

## §6. (f) Dạng phương thức `s.f(…)`
**Hiện trạng**: `s.upper()`, `s.split(",")`, `s.len()` **chạy** (probe `f1`,`f2`); `s.trim()`/`s.replace(...)` ⇒ **VS0403**
(“`str` không có phương thức … (ST-13: len upper lower split chars contains)”, `crates/vis-types/src/tyck/expr.rs:1004`);
`use std.str` + `upper("x")` ⇒ **VS0501** (“tên `std` chưa hỗ trợ ở VS0”) — **module `std` chưa mở ở VS0**.
**Đề xuất**: (i) với bộ hàm **đang có**, dạng `s.f(…)` đã đúng — chỉ cần **khoá** bằng ca; (ii) thêm `trim/replace/…` là tính năng mới
(cần Agent 0); (iii) `std.str.f(s, …)` chưa tồn tại ⇒ đừng hứa ở tài liệu cho tới khi có module std.
**Ca đỏ-trước**: `s.trim()` ⇒ VS0403 (hoặc xanh nếu mở); `std.str.upper("x")` ⇒ VS0501.
**Ảnh hưởng**: `tyck/expr.rs:1004`, builtin method table, docs/cheatsheet.

## §7. (g) Format spec `"{n:.2}"`
**Hiện trạng**: `"{n}"` chạy (in `3`); `"{n:.2}"` và `"{n:.2f}"` ⇒ **VS0201** (lỗi cú pháp tại dấu `:`) — chưa có spec.
**Đề xuất**: hỗ trợ `{expr:spec}` với `spec` = `.N` (số chữ số thập phân, chỉ cho `float`, làm tròn half-even? — Agent 0 chốt);
spec sai ⇒ **mã lỗi riêng** (không dùng VS0201 chung). Đây là **thiết kế** ⇒ Agent 0 quyết.
**Ca đỏ-trước**: `"{3.14159:.2}"` ⇒ `3.14`; `"{3:.2}"` ⇒ lỗi rõ (không phải VS0201 chung); `"{n:xx}"` ⇒ lỗi rõ.
**Ảnh hưởng**: parser chuỗi nội suy (vis-syntax), interp format, docs §chuỗi.

## §8. VS0412 có dùng cho **hai lỗi khác nhau** không? — **CÓ**
**Kết luận**: **Có.** `VS0412` (định nghĩa “kiểu không khớp”, `crates/vis-diag/src/lib.rs:207-260`) đang phủ **ít nhất 4 nhóm lỗi**:

| Nhóm | Bằng chứng | Vị trí phát |
|---|---|---|
| Kiểu không khớp chung | `let n: int = "x"` | `tyck/mod.rs:42` (định nghĩa), nhiều nơi |
| **Gán cho ràng buộc bất biến** (`let`/tham số) | `let x = 1; x = 2` ⇒ `VS0412: không gán được cho `let`/tham số (chỉ `var` …)` (probe `r1`) | `tyck/mod.rs:1125` |
| **Toán tử không hỗ trợ cho kiểu** (`str + str`, trộn đơn vị) | `"a" + "b"` ⇒ VS0412 (probe `a`) | `tyck/expr.rs:501`, `tyck/num.rs:5,21` |
| `async`/range sai chỗ | `async` trên method; range không `int` | `tyck/mod.rs:818,1067` |

**Đề xuất tách** (Agent 0 chốt số; 04xx còn trống 0404, 0406–0411):
- `VS0413` = **gán cho ràng buộc bất biến** (let/tham số) — thông điệp “dùng `var`”.
- `VS0414` = **toán tử không hỗ trợ cho kiểu** (`str+str`, đơn vị) — tách khỏi “kiểu không khớp”.
- `VS0415` = **`async` đặt sai chỗ**; `VS0416` = **range không `int`** (nếu muốn tách tiếp).
**Ảnh hưởng**: `crates/vis-diag/src/lib.rs` (bảng mã), `agent/API-CODES.txt` + `.license`, **mọi test so mã** (`vec!["VS0412"]`),
tài liệu/cheatsheet. **Không** làm trong đêm nay (0 dòng mã).

## §9. Cheat sheet
`CHEAT-SHEET-ViScript.md:12` có nhãn ✖ “thiết kế có nhưng **VS0 chưa có**” ⇒ đã đổi thành
**“chưa có tại `<commit>`”** (để khi repo tách/commit mới, người đọc biết mốc). Tệp đang **không được track**;
khi tách repo (ST-171) phải đưa vào (hoặc sinh lại).

## §10. Tổng hợp ưu tiên (đề nghị)
1. **(a) gợi ý `{a}{b}`** — 1 dòng, rủi ro thấp, làm sớm.
2. **(g) format spec** — hữu ích, nhưng là thiết kế (Agent 0 chốt spec/làm tròn/mã lỗi).
3. **(e) print nhiều đối số** — thân thiện, rủi ro thấp (nới arity 1 builtin).
4. **(c) thống nhất `Some`/`some`** — pattern đã đúng; chỉ cần tài liệu khoá.
5. **(b) `s[a:b]`** — **không** mở; chỉ cải thiện thông điệp.
6. **(f) `std.str.f`** — chờ module `std` (VS3); với bộ hàm hiện có thì `s.f()` đã đúng.
7. **(d) default của `type`** — thiết kế mới, cần cấp nợ + task riêng.
