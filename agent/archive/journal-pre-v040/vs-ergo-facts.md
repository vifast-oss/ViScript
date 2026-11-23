<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS-ERGO-FACTS · Dữ kiện ViScript (ST-172a) — **chỉ dữ kiện, không đề xuất**
**Trạng thái**: gửi Agent 0 (VD-169) · **HEAD khi soạn**: `823047ac` · **Binary probe**: `target/debug/vis` của cổng L0 **1179/0 @ `0df923c2`**
**Nhãn trạng thái**: **[ĐÃ CÓ]** có trong mã và chạy được · **[THIẾT KẾ CÓ, CHƯA LÀM]** đặc tả/thiết kế có, mã chưa · **[CHƯA CÓ]** không có ở cả hai.
**Quy ước**: đường dẫn tính từ **gốc ViScript**; probe ghi `vis run <tệp>` (tệp trong `/tmp/ergo/`).

## Mục lục
1. Cú pháp
2. Hệ kiểu
3. Ngữ nghĩa giá trị
4. Lỗi
5. Thư viện chuẩn
6. Tính năng riêng
7. Trải nghiệm thật
8. Quyết định đã chốt

---

## 1. Cú pháp

### 1.1 Từ khoá — **40**, `[ĐÃ CÓ]` (`crates/vis-syntax/src/lex/token.rs:185`)
`alias and async await break cmd const continue derives dyn elif else enum extern false fn for has if impl in let loop match newtype none not or pub ref return scope self trait true type use var while with`
- **Không** là từ khoá: `test` (ngữ cảnh `test fn`), `as` (đã bỏ — dùng `.into`? xem §2.5), `set` (chỉ `set{` là token riêng).
- `newtype`, `panic` **không** có trong danh sách ở trên? — `newtype` **có** trong bảng (ở trên), nhưng **chưa hạ** (§2.6).

### 1.2 Dấu câu — **38**, `[ĐÃ CÓ]` (`token.rs:200`)
`..= != ** -> .. <= == => >= ?. ?? |> ! & ( ) * + , - . / : ; < = > ? @ [ \ ] ^ { | } ~ %`
- `#`/`##` **không** là dấu câu: luôn thành chú thích (SPEC-lex §5).
- `$`, `^`, `set{` là kind riêng (`DOLLAR`, `CARET`, `SET_LBRACE`).

### 1.3 Toán tử + độ ưu tiên (thấp → cao) — `[ĐÃ CÓ]`, thang trong `crates/vis-syntax/src/parse/expr.rs`
| Bậc | Toán tử | Hàm parser |
|---|---|---|
| 1 | `\|>` (pipe biểu thức) | `pipe_expr` (`parse/expr.rs:119`) |
| 2 | `or`, `\|\|` | `or_expr` (:128) |
| 3 | `and`, `&&` | `and_expr` (:137) |
| 4 | `not`, `!` | `not_expr` (:146) |
| 5 | `== != < <= > >=` (không liên kết chuỗi) | `cmp_expr` (:155) |
| 6 | `??` | `coalesce` (:182) |
| 7 | `..`, `..=` | `range_expr` (:192) |
| 8 | `+ -` | `add_expr` (:208) |
| 9 | `* / %` | `mul_expr` (:222) |
| 10 | `- +` một ngôi | `unary_expr` (:244) |
| 11 | `**` (phải kết hợp) | `power_expr` (:253) |
| 12 | gọi/hậu tố: `f(…)`, `x[i]`, `x.f`, `?`, `?.` | `postfix` (:280) |

### 1.4 Tóm tắt ngữ pháp `[ĐÃ CÓ]` — mô tả theo hàm parser
- Khai báo cấp tệp: `fn`, `cmd`, `type`, `enum`, `trait`, `impl`, `const`, `use`, `alias`, `extern`, `scope` (`parse/khai_bao.rs`).
- `fn` → thân bằng khối thụt lề; `cmd` → thân **một pipeline** hoặc khối (`parse/cmd.rs`).
- Biểu thức: `expr → lambda → pipe → or → and → not → cmp → coalesce → range → add → mul → unary → power → postfix` (chuỗi gọi ở §1.3).
- Lambda: `IDENT => expr`, `( a, b ) => expr`, `() => expr`, `fn(...) -> T: suite` (`parse/expr.rs:22`).
- Mẫu: `match` với nhánh `pat: suite`; pattern gồm literal, `Some(x)`, `some(x)`, `none`, `ok(x)`, `err(x)`, record `{a, b, ..}`, list `[a, b, ..]` (`tyck/pat.rs`).
- EBNF đầy đủ: **không** có trong `SPEC-lex` (chỉ từ vựng); thiết kế §9 có phác thảo. `[THIẾT KẾ CÓ, CHƯA LÀM]` (EBNF chính thức chưa viết).

### 1.5 Thụt lề, xuống dòng, chú thích `[ĐÃ CÓ]` (SPEC-lex §4, §5)
- Đơn vị thụt lề = **mức thụt đầu tiên > 0**; mọi mức sau là bội của nó; sai ⇒ `VS0109`. Tab ⇒ `VS0107` (cảnh báo-lỗi), tab = 4 cột.
- Trong `() [] {}` bỏ qua NEWLINE/INDENT/DEDENT; dòng bắt đầu `|>` nối với dòng trước (SPEC-lex:69).
- Dòng trống/chỉ chú thích: không sinh NEWLINE.
- Chú thích: `#` (script: luôn; chế độ lệnh: chỉ khi đầu từ), `##` doc-comment (SPEC-lex:163-164).

### 1.6 Chuỗi `[ĐÃ CÓ]` (SPEC-lex §3)
| Dạng | Escape | Nội suy | Ghi chú |
|---|---|---|---|
| `"…"` | có | có | |
| `r"…"` | **không** | **không** | `\` là ký tự thường |
| `"""…"""` | có | có | nhiều dòng |
| `r"""…"""` | không | không | |
| `b"…"` | chỉ `\xNN` | không | ASCII; >0x7F ⇒ `VS0106` |
| `p"…"` | có | có | literal `path` |

- Nội suy: `{expr}` hoặc `{expr:spec}` **ở tầng token** (SPEC-lex:58); `{{`/`}}` là ngoặc literal; `{` không đóng ⇒ `VS0108`.
- **`{expr:spec}` mới là token, runtime CHƯA hỗ trợ**: `"{n}"` in `3`; `"{n:.2}"` ⇒ **VS0201** (probe `g1`,`g3`) ⇒ **[THIẾT KẾ CÓ, CHƯA LÀM]**.
- Escape hợp lệ: `\n \t \r \0 \\ \" \' \{ \} \u{X..X}` (SPEC-lex:57).

## 2. Hệ kiểu

### 2.1 Số `[ĐÃ CÓ]`
- `INT` = **`i64`**; literal tràn ⇒ **lỗi biên dịch `VS0104`** (SPEC-lex:40,55). `FLOAT` = **`f64`**; không literal `NaN`/`inf` (dùng `float.nan`/`float.inf`, SPEC-lex:41).
- **Tràn lúc chạy** ⇒ **`VS0501`** “tràn i64 lúc chạy (VD-43 #2)” (probe `p2`: `i64::MAX + 1`).
- **Trộn int/float được**: `1 + 1.5` ⇒ `2.5` (probe `p3`) `[ĐÃ CÓ]`.
- Kiểu số khác: `size` (SIZE literal), `duration`, `percent` (`ty.rs:10,35,87,108`).

### 2.2 `str` `[ĐÃ CÓ]`
- Tệp UTF-8, **NFC toàn tệp trước khi lex** (SPEC-lex:9); `col` tính theo scalar.
- `len()` đếm **Unicode scalar (code point)**, **không** phải grapheme: `"ố".len()`=1; `"👨‍👩‍👧".len()`=**5**; `"🇻🇳".len()`=**2** (probe `p1`,`q2`).
- `[CHƯA CÓ]` `s[i]`, `s[a:b]` (probe `b2`,`b3`); gợi ý `.chars()/.graphemes()/.byte_at(i)?`.

### 2.3 Suy kiểu `[ĐÃ CÓ]` (một phần)
- `let` suy từ vế phải; **`const` bắt buộc ghi kiểu** (`const A = 1` ⇒ VS0201, probe `n4`).
- Tham số `fn`/`cmd` và kiểu trả về **bắt buộc** ghi (trừ `cmd` không trả).
- Closure: suy từ ngữ cảnh/thân (thiết kế §4:247-248).
- Record literal **bắt buộc** đủ trường (trừ khi có default — chưa có, §2.6).

### 2.4 Optional `[ĐÃ CÓ]`
- **Tự gói `T → T?`**: `let x: int? = 5` chạy, in `5` (probe `n1`).
- `??` chạy: `o ?? 9` ⇒ `5` (probe `p5`).
- Pattern **nhận cả `Some(v)` và `some(v)`** (probe `c6`,`c8`); `none` khớp; constructor **chỉ `Some(1)`** (chữ thường `some(1)` ⇒ VS0501, probe `c4`).
- `?.` có token (`?.”, `token.rs:200`) nhưng **chưa tìm thấy ca dùng** trong test/mã ⇒ cần xác nhận `[CHƯA RÕ]`.
- `if let`: **không có** trong `KEYWORDS` và không có ca test `.vis` ⇒ `[CHƯA CÓ]`.

### 2.5 Chuyển kiểu ngầm `[ĐÃ CÓ]` (hẹp)
- `int → float` ngầm: `let f: float = 1` chạy (probe `p4`).
- Số học trộn int/float ⇒ float (probe `p3`).
- Không thấy chuyển ngầm `str → path` (thiết kế:257 nói **không**).
- Không có `as` (đã bỏ khỏi bảng từ khoá, `token.rs:182`).

### 2.6 Generic, trait, impl, derives `[ĐÃ CÓ]` (một phần)
- Generic hàm: `fn id[T](x: T) -> T` chạy, gọi `id(5)`/`id("a")` (probe `q3`); có test `crates/vis-types/tests/generic.rs`, `crates/vis-cli/tests/generic_cli.rs:30`.
- `trait`/`impl`/`derives` có trong từ khoá + test (`crates/vis-types/tests/trait.rs`); chưa thống kê giới hạn.
- `newtype` **có trong bảng từ khoá nhưng KHÔNG hạ**: dòng `newtype Met = float` **bị bỏ qua im lặng** (probe `n11`: chương trình chạy, exit 0), dùng `Met` ⇒ **VS0301** (probe `n8`) ⇒ **[THIẾT KẾ CÓ, CHƯA LÀM] + bẫy im lặng**.
- `enum`: từ khoá + test (`exhaustive.rs`); `[ĐÃ CÓ]`.

## 3. Ngữ nghĩa giá trị

### 3.1 list/map/set/record khi truyền vào hàm — **sao chép** `[ĐÃ CÓ]`
Probe `q1`: `fn them(xs: list[int]): xs.push(3)` + `let a=[1,2]; them(a); print(a.len())` ⇒ **`2`** (không đổi). Tức **truyền theo giá trị (copy)**, `push` trong hàm không ảnh hưởng biến ngoài.
- `var` bên ngoài **không** bị đổi qua tham số (cùng probe).
- Chia sẻ trạng thái biến đổi: dùng `ref[T]` (thiết kế:428) — `[THIẾT KẾ CÓ, CHƯA XÁC MINH]`.

### 3.2 Closure `[ĐÃ CÓ]`
- Cú pháp: `x => expr`, `(a,b) => expr`, `() => expr`, `fn(...) -> T: suite` (`parse/expr.rs:22`).
- **Bắt biến theo giá trị** (thiết kế:428). Muốn chia sẻ trạng thái biến đổi: `ref[T]`.
- Probe `p7` dùng `|y: int|` ⇒ VS0201 (đó **không** phải cú pháp closure của ViScript — cú pháp là `=>`).

### 3.3 Shadowing `[ĐÃ CÓ]`
- Trong khối lồng: `let x = 1; if …: let x = 2; print(x)` ⇒ `2`, ra ngoài `1` (probe `p8`).
- Cùng phạm vi: khai báo trùng ⇒ `VS0302` (probe: `let take`...; cũng gặp ở `e.vish`).

---

## 4. Lỗi

### 4.1 `Result`, `?`, `else` `[ĐÃ CÓ]` (một phần)
- `Result[T, E]` + `?` **chỉ** khi hàm trả đúng `Result`/`Option`: sai ⇒ **VS0412** với câu “`?` trên `Result` chỉ dùng được khi hàm trả `Result[…, …]`, hàm này trả `{…}` (§4.4)” (`crates/vis-types/src/tyck/expr.rs:177,194`; probe `n9`).
- `ok(x)`/`err(e)` là pattern + constructor; `match` bắt buộc xét hết (`VS0402`).
- `else e:` trên biểu thức: **không có** — `print(f() else 0)` ⇒ VS0201 (probe `z2`).
- **`AnyError`**: thiết kế có (VD-20, §21.1#13: `trait Error { message, source, span }`, `AnyError` xoá kiểu, `?` tự chuyển, `main -> Result[(), AnyError]`) nhưng **mã chưa có** ⇒ **[THIẾT KẾ CÓ, CHƯA LÀM]**.

### 4.2 Hàm dùng cả `fs` và `json` — hiện phải viết thế nào `[ĐÃ CÓ]`
Không có `AnyError` ⇒ phải **match lồng** từng bước (không dùng `?` chéo kiểu được):
```
use std.fs
use std.json
fn doc(p: str) -> Result[map[str, str], str]:
    match fs.read(p):
        ok(s):
            match json.decode(s):
                ok(v): ...
                err(e): err(e)
        err(e): err(e)
```
(Đây là mẫu viết được hôm nay; `?` sẽ báo VS0412 — probe `n9`.)

### 4.3 Toàn bộ `all_codes` — **33 mã** `[ĐÃ CÓ]` (`crates/vis-diag/src/lib.rs:207-260`; `agent/API-CODES.txt`)
| Mã | Mức | Thông điệp (đóng băng) | Ghi chú đa nghĩa |
|---|---|---|---|
| VS0101 | error | chuỗi chưa đóng | |
| VS0102 | error | escape không hợp lệ | **(hay gặp #2)** dùng khi `\` trong chuỗi thường |
| VS0103 | error | ký tự lạ ngoài bảng token | |
| VS0104 | error | literal số sai/tràn i64 | |
| VS0105 | error | char literal không đúng 1 scalar | |
| VS0106 | error | byte string có ký tự > 0x7F | |
| VS0107 | warning | tab (cảnh báo-lỗi; tab = 4) | |
| VS0108 | error | nội suy `{` không đóng | |
| VS0109 | error | thụt lề không khớp đơn vị/DEDENT sai | |
| VS0110 | error | `^` sai ngữ cảnh | |
| VS0201 | error | lỗi cú pháp: token không mong đợi/thiếu vế | **đa nghĩa** (mọi lỗi cú pháp) |
| VS0202 | error | `%` dùng như phần trăm — viết `50pct` | |
| VS0301 | error | tên không tồn tại | **đa nghĩa** (biến, kiểu `newtype`) |
| VS0302 | error | khai báo trùng trong cùng phạm vi | |
| VS0401 | error | sai số lượng hoặc thứ tự đối số | |
| VS0402 | error | `match` không xét hết trường hợp | |
| VS0403 | error | trường, kiểu hoặc phương thức không tồn tại | |
| VS0405 | error | không có trường này trên kiểu bản ghi | |
| VS0412 | error | kiểu không khớp | **đa nghĩa: 4 nhóm** (kiểu; gán `let`; toán tử; async/range) |
| VS0501 | error | lỗi runtime: phép toán/tính năng chưa hỗ trợ ở VS0 | **đa nghĩa** (tính năng chưa hỗ trợ + tràn i64 + `some(1)`) |
| VS0502 | error | chia hoặc lấy dư cho 0 | |
| VS0503 | error | chỉ số ngoài phạm vi | |
| VS0504 | error | gửi vào kênh đã `close` | |
| VS0505 | error | deadlock kênh | |
| VS0506 | error | tác vụ (`async fn`) lỗi | |
| VS0601 | error | không có lệnh này | |
| VS0602 | error | sai đối số lệnh | (cũng dùng cho `--help` chưa có) |
| VS0603 | error | pipeline không nối được | |
| VS0604 | error | tính năng chưa hỗ trợ ở VS0 (vd. `&`) | **đa nghĩa** |
| VS0701 | error | không thấy tệp module (hoặc `std`/`pkg` chưa hỗ trợ) | **đa nghĩa** |
| VS0702 | error | vòng phụ thuộc giữa các tệp module | |
| VS0703 | error | `use` trùng hoặc mơ hồ | |
| VS0704 | error | không giải được ràng buộc gói | |

### 4.4 Mười cái bẫy — **văn bản thật** (copy nguyên output, probe `t1..t10`)
```
t1.vis:2:13: error VS0412: toán tử `+`: `str` và `str` không cùng kiểu/đơn vị
t2.vis:3:5: error VS0412: không gán được cho `let`/tham số (chỉ `var` mới gán được, §4.1)
t3.vis:3:11: error VS0412: `str` không có `s[i]` — dùng `.chars()`, `.graphemes()`, `.byte_at(i)?` (VD-18)
t4.vis:2:5: error VS0401: `print` cần 1 đối số, được 2
t5.vis:1:9: error VS0201: mong `:`, thật `=`
t6.vis:3:18: error VS0301: tên không tồn tại: kiểu `Met` không có trong VS0 (VD-43) và không được khai trong tệp
t7.vis:2:11: error VS0501: tên `some` chưa hỗ trợ ở VS0
t8.vis:3:14: error VS0201: mong `]`, thật `:`
t9.vis:4:16: error VS0201: mong `)`, thật `:`
t10.vis:2:13: error VS0201: mong xuống dòng sau câu lệnh, thật `:`
```
Ghi chú: `t5` (`const A = 1`), `t8` (`s[0:2]`), `t9` (`U(n: 1)`), `t10` (`"{x:.2}"`), `t4` (`print` 2 đối số) đều báo **VS0201/VS0401 không kèm gợi ý** — người dùng chỉ thấy “mong X, thật Y”.

## 5. Thư viện chuẩn

### 5.1 API std — **51 hàm**, `[ĐÃ CÓ]` (`agent/API-STD.txt:7-58`; sinh từ `crates/vis-types/src/stdlib.rs::FUNCS`)
| Module | Hàm (chữ ký) | Gọi được dạng method? |
|---|---|---|
| `fs` | `exists(str)->bool`, `list(str)->Result[list[str],str]`, `read(str)->Result[str,str]`, `write(str,str)->Result[(),str]` | **không** |
| `http` | `get(str)->Result[str,str]`, `get_timeout(str,int)`, `head(str)->Result[map[str,{error}],str]`, `head_timeout(str,int)` | **không** |
| `json` | `decode(str)->V`, `encode(T)->str`, `encode_pretty(T)->str` | **không** |
| `list` | `chunk(list[T],int)->list[list[T]]`, `concat`, `drop`, `reverse`, `sort`, `sort_by(list[T],fn(T)->int\|str)`, `take`, `unique` | **có** (cùng tên với phương thức `list` … xem 5.3) |
| `map` | `get_or(map[K,V],str,V)->V`, `keys`, `merge`, `remove`, `values` | **có** một phần (`get/has/keys/values`) |
| `net` | `accept(str)->Result[str,str]`, `accept_timeout`, `close`, `close_listen`, `connect`, `connect_timeout`, `listen`, `local_addr`, `read(str,int,int)`, `write(str,str)` | **không** |
| `path` | `basename(str)->str`, `dirname`, `ext`, `is_abs`, `join(str,str)`, `stem` | **không** |
| `str` | `contains(str,str)->bool`, `ends_with`, `join(list[str],str)`, `lower(str)->str`, `repeat(str,int)`, `replace(str,str,str)`, `split(str,str)->list[str]`, `starts_with`, `trim(str)->str`, `upper(str)->str` | **một phần**: `len/upper/lower/split/chars/contains`; **không** `trim/replace/…` (VS0403) |
| `time` | `monotonic_ms()->int`, `now_ms()->int` | **không** |

- Cách gọi: `use std.<mod>` rồi `mod.fn(args)` (probe `z1`: `use std.str` + `str.upper("abc")` ⇒ `ABC`, `str.trim` ⇒ `x`).
- **Không** gọi được `use std.str` + `upper("x")` (tên trần) ⇒ VS0501 (probe `f4`); **không** có `std.str.upper` trước `use` (probe `f3` ⇒ VS0501 “tên `std` chưa hỗ trợ”).

### 5.2 Builtin toàn cục `[ĐÃ CÓ]` (`crates/vis-types/src/lower.rs:53`; `crates/vis-types/tests/names.rs:480`)
`print`, `len`, `str`, `echo`, `lines`, `take`, `Ok`, `Err`, `Some`, `None`, `ref` (+ `float.nan`, `float.inf`, `i64.min`).
- `print`: **đúng 1 đối số** (VS0401 nếu 0 hoặc ≥2; `tyck/expr.rs:1303-1319`).
- `len`: phương thức `.len()` cho `str`/`list`/`map`.
- `Some/None`: literal — `Some(1)` dựng được, `none` viết thường; `some(1)` chữ thường ⇒ VS0501 (bất đối xứng, xem §2.4).

### 5.3 Phương thức theo kiểu (từ thông điệp VS0403) `[ĐÃ CÓ]`
- `str`: `len upper lower split chars contains` (`tyck/expr.rs:1004`).
- `list`: `len get first last push map filter each sum join` (`tyck/expr.rs:970`).
- `map`: `len get has keys values` (`tyck/expr.rs:1039`).
- Closure trong `map/filter/each` dùng cú pháp `x => expr` / `fn(...) -> T:` (xem §3.2); dạng `|x|` **không** phải cú pháp ViScript (probe `k1`,`p7`).

## 6. Tính năng riêng

### 6.1 Literal đơn vị `[ĐÃ CÓ]` (SPEC-lex:30-32; `lex/lit.rs:70`)
- `size` = `b kb mb gb tb kib mib gib tib` (**kb=1000**, kib=1024 — VD/SPEC-lex:30).
- `duration` = `ns us ms s m h d w`, ghép **giảm dần** không cách (`2h30m` là **một** token).
- `percent` = `INT|FLOAT` + `pct` (`50pct`); **`50%` bỏ** (VS0202, VD-13).
- Phép toán cùng đơn vị `[ĐÃ CÓ]`: `5kb+5kb` ⇒ `10kb`; `1h+30m` ⇒ **`90m`**; `50pct+10pct` ⇒ `60pct` (probe `p9`). Cách in: **giữ đơn vị gốc nhỏ nhất** (`150m` in `150m`, không `2h30m` — probe `n7`,`n10`).
- Trộn đơn vị khác nhau (`1kb+1kib`) — chưa probe; thiết kế nói trộn ⇒ VS0412 (theo `tyck/num.rs:5`). `[CHƯA XÁC MINH]`.

### 6.2 `cmd` `[ĐÃ CÓ]` (một phần)
- Khai báo `cmd ten(a: T, --co: bool = false) -> ()`; gọi `ten "x" --co`.
- Thân `cmd` **không được trộn** biểu thức script và lệnh: probe `cf` ⇒ VS0201 “thân `cmd`: lẫn biểu thức script và lệnh — chọn một kiểu”.
- **`--help` KHÔNG tự sinh**: `greet --help` ⇒ **VS0602** “`greet` không có cờ `--help`” (probe `c2`) ⇒ **[THIẾT KẾ CÓ, CHƯA LÀM]** (nếu thiết kế có).
- Cờ `bool` dùng được: `greet "An" --loud` chạy (probe `cg` in `hi An`).

### 6.3 Pipeline `[ĐÃ CÓ]`
- Lệnh có sẵn (`crates/vis-types/src/cmdsig.rs`): `take`, `ls`, `open`, `get`, `select`, `sort_by`, `where`, `each`, `first`, `count`.
- Kiểu chảy qua: `ls` ⇒ luồng `BảnGhi` (file entry); `where` nhận closure; `select` chiếu cột; `sort_by` theo khoá (xem `buildin`/cmd tests). `[ĐÃ CÓ]` một phần; kiểu dữ liệu chính xác cần đọc `cmdsig.rs:116-205`.

### 6.4 Module, `use`, `pub`, `vis pkg` `[ĐÃ CÓ]` (một phần)
- `use <mod>` cho `std.<mod>` (probe `z1`) và tệp module; lỗi: `VS0701` (không thấy tệp/std/pkg chưa hỗ trợ), `VS0702` (vòng), `VS0703` (trùng/mơ hồ).
- `pub` cho mục công khai (`KEYWORDS` có `pub`).
- `vis pkg` **có**: `add <tên>[@ver] [--entry] [--registry <dir>]`, `install`, `update [<tên>]`, `list [--json]`, `remove <tên>` (probe `z`); registry là **thư mục cục bộ** ở VS0.

## 7. Trải nghiệm thật

### 7.1 Ba script chạy thật (đã chạy `vis run`, dán đúng output)
**Script 1 — đọc log, đếm mức** (`/tmp/ergo/s1.vis`, 20 dòng):
```
use std.fs
fn main():
    match fs.read("/tmp/ergo/app.log"):
        ok(noi):
            let dong = noi.split("\n")
            var n = 0
            var e = 0
            var w = 0
            for l in dong:
                if l.len() > 0:
                    n = n + 1
                    if l.contains("ERROR"):
                        e = e + 1
                    if l.contains("WARN"):
                        w = w + 1
            print("tong dong: {n}")
            print("ERROR: {e}")
            print("WARN : {w}")
        err(x):
            print("loi doc: {x}")
```
Python tương đương:
```python
import sys
d = open("/tmp/ergo/app.log", encoding="utf-8").read().split("\n")
d = [l for l in d if l]
print(f"tong dong: {len(d)}")
print(f"ERROR: {sum('ERROR' in l for l in d)}")
print(f"WARN : {sum('WARN' in l for l in d)}")
```
Output thật: `tong dong: 5` · `ERROR: 2` · `WARN : 1`.

**Script 2 — ghi/đọc JSON** (`/tmp/ergo/s2.vis`, 18 dòng):
```
use std.fs
use std.json
fn main():
    let ban = {"ten": "An", "lop": "10A"}
    let ra = json.encode_pretty(ban)
    match fs.write("/tmp/ergo/out.json", ra):
        ok(_):
            print("da ghi out.json ({ra.len()} byte)")
        err(e):
            print("loi ghi: {e}")
    match fs.read("/tmp/ergo/out.json"):
        ok(s):
            let v = json.decode(s)
            print("doc lai ok, {s.len()} byte")
        err(x):
            print("loi doc: {x}")
```
Python tương đương:
```python
import json
ban = {"ten": "An", "lop": "10A"}
ra = json.dumps(ban, ensure_ascii=False, indent=2)
open("/tmp/ergo/out.json", "w", encoding="utf-8").write(ra)
print(f"da ghi out.json ({len(ra.encode())} byte)")
s = open("/tmp/ergo/out.json", encoding="utf-8").read()
json.loads(s)
print(f"doc lai ok, {len(s.encode())} byte")
```
Output thật: `da ghi out.json (33 byte)` · `doc lai ok, 33 byte`.

**Script 3 — công cụ CLI có cờ** (`/tmp/ergo/s3.vis`, 25 dòng; `fn main(args: list[str])`):
```
use std.fs
fn main(args: list[str]):
    var dem = false
    var tep = ""
    for a in args:
        if a == "--dem":
            dem = true
        else:
            tep = a
    if tep.len() == 0:
        print("dung: s3.vis <tep> [--dem]")
        return
    match fs.read(tep):
        ok(noi):
            let dong = noi.split("\n")
            var n = 0
            for l in dong:
                if l.len() > 0:
                    n = n + 1
            if dem:
                print("{n} dong")
            else:
                for l in dong:
                    if l.len() > 0:
                        print(l)
        err(e):
            print("loi: {e}")
```
Python tương đương:
```python
import sys
dem = "--dem" in sys.argv
tep = [a for a in sys.argv[1:] if a != "--dem"]
if not tep:
    print("dung: s3.py <tep> [--dem]"); sys.exit()
d = [l for l in open(tep[0], encoding="utf-8").read().split("\n") if l]
print(f"{len(d)} dong") if dem else print("\n".join(d))
```
Output thật: `vis run s3.vis -- /tmp/ergo/app.log --dem` ⇒ `5 dong`.

### 7.2 Lỗi Agent 2 hay gặp (đếm từ `agent/JOURNAL/*.log`, mẫu lớn)
| Hạng | Mã | Số lần | Nguyên nhân điển hình |
|---|---|---|---|
| 1 | `VS0501` | 289 | gọi tính năng chưa hỗ trợ (`some`, `panic`, `std` trước `use`), tràn i64 |
| 2 | `VS0102` | 258 | `\` trong chuỗi thường (đường dẫn Windows!) — dùng `r"…"` |
| 3 | `VS0201` | 147 | cú pháp: `=>` thay `:`, `const A = 1`, `s[a:b]`, `U(n: …)`, `{x:.2}` |
| 4 | `VS0301` | 38 | tên/kiểu không tồn tại (gồm `newtype` bị bỏ qua) |
| 5 | `VS0701` | 25 | `use std.*`/module chưa hỗ trợ, thiếu tệp module |
| 6 | `VS0412` | 22 | gán `let`, `str+str`, `?` sai kiểu |
| 7 | `VS0403` | 15 | phương thức không có (`trim/replace/graphemes/byte_len`) |
| 8 | `VS0604` | 7 | `^`/`&`/tiến trình chưa hỗ trợ |
| 9 | `VS0505` | 5 | deadlock kênh |
| 10 | `VS0107` | 3 | tab |

## 8. Quyết định đã chốt (có lý do trong `docs/viscript/ViScript-thiet-ke.md`)

### 8.1 §21 “Các lựa chọn đã chốt” (Q1–Q8, dòng 1102-1119)
| Mã | Chốt | Lý do (tóm) |
|---|---|---|
| Q1 | Từ khoá **tiếng Anh**; định danh Unicode; lỗi có tiếng Việt | giống JS/TS/Rust; từ khoá có dấu gõ chậm |
| Q2 | `int / int` ⇒ **`int`**; lint khi chia literal/kết quả dùng với float | nhất quán kiểu tĩnh, không chuyển ngầm |
| Q3 | Thụt lề vào **nhất quán bất kỳ**, ra `vis fmt` **4**; tab = VS0107 + 4 cột | dán mã không chết; một kiểu đầu ra |
| Q4 | Tên **ViScript** (`.vis`, `vis`), **ViShell** (`.vish`, `vish`) | ngắn, nhất quán hệ ViFast |
| Q5 | `xs[i]` **panic** nêu chỉ số+độ dài; `xs[i]?`/`xs.get(i)` trả `T?`; **cấm `s[i]`** | lỗi lập trình hiện rõ; vẫn có đường an toàn |
| Q6 | Giấy phép **MIT OR Apache-2.0** | giống Rust, có patent grant |
| Q7 | `decimal` **không** ở lõi, là thư viện | giữ lõi nhỏ |
| Q8 | Bit ops là **hàm** (`bit_and/shl/ushr`…), **`\|` không bao giờ là toán tử nhị phân** | giữ or-pattern/pipe |

### 8.2 §21.1 (15 chốt sau review, dòng 1120-1136) — tóm
NFC toàn tệp; float phải có chữ số sau `.`; `1..2` là range; literal tràn i64 = lỗi biên dịch; size units `b…tib` (**kb=1000**); duration `ns…w` ghép giảm dần là một token; **bỏ `50%` → `50pct`**; năm dạng chuỗi; `#` theo vị trí; `set{`; `%` theo số bị chia, `%0` panic; bảng ưu tiên **12 mức** (postfix → `**` → một ngôi → `* / %` → `+ -` → `..` → `??` → so sánh → `not` → `and` → `or` → `|>`); optional tự nâng `T→T?`; guard không tính coverage; `trait Error`/`AnyError`; mã thoát 0/1/2/101/126/127/130.

### 8.3 §22 VD-01…VD-20 (dòng 1137-1160) — những cái ảnh hưởng công thái học
| Mã | Chốt |
|---|---|
| VD-01 | Kiểu tĩnh bắt buộc, **không `any`, không null** |
| VD-03 | **RC + copy-on-write**, không GC truy vết; chu trình chỉ qua `ref`, phá bằng `weak` |
| VD-05 | Mọi truy cập hệ thống **qua capability** (không hàm toàn cục) |
| VD-07 | **Không JIT**, không `eval` chuỗi trong chế độ hạn chế |
| VD-09 | Từ khoá Anh, định danh Unicode NFC, thông báo vi/en/ja |
| VD-13 | Bỏ `50%` → `50pct`; `%` chỉ modulo |
| VD-14 | `#` chú thích theo vị trí; `https://…#frag` giữ nguyên |
| VD-15 | `\|` **không** là toán tử nhị phân trong biểu thức |
| VD-16 | **12 mức ưu tiên** như §8.2 |
| VD-17 | Float không `Eq/Ord/Hash`; `percent == float` theo tỉ lệ |
| VD-18 | **Cấm `s[i]`**; `xs[i]?` trả `T?`; `xs[-1]` panic; `byte_len/char_count/grapheme_count` (thiết kế) |
| VD-19 | Optional tự nâng `T→T?` ở gán/trả/đối số; `T??` lỗi; `?` trên `T?`; pattern `none/some/ok/err` |
| VD-20 | `trait Error { message, source, span }`; `AnyError`; `?` tự chuyển; `main -> Result[(), AnyError]`; mã thoát |

### 8.4 Đã chốt **không làm**
Không `class`/kế thừa (**VD-01/VD-03**: record + trait); không `any`/null; không JIT/`eval` (VD-07); không redirect `>`/`<` trong chế độ lệnh (SPEC-lex:96 — ghi tệp bằng `| save`); không `as` (đã bỏ khỏi từ khoá); không literal `50%`; không `defer` (21.1#14); không `s[i]` (VD-18); `\|` không phải toán tử nhị phân (VD-15).
---

## ĐÍNH CHÍNH (2026-11-17, sau probe cho VD-171)
- **`if let` CÓ** (probe `y6`: `if let v = o:` ⇒ in `5`); bản §2.4 ghi “chưa có” là **sai**. Dạng `if let Some(v) = o:` ⇒ **VS0412** “pattern ctor `Some` không khớp kiểu `int`” (pattern kiểm sai tầng).
- **`let … else:` CÓ** (probe `y7`: `let x = r else:` + khối `return` chạy); `else 0` ⇒ VS0201 (khối bắt buộc).
- **`T??` CHƯA có** (`y2` ⇒ VS0201); **`xs[i]?` CHƯA có** (`x2` ⇒ VS0412); **`xs.get(i)` CÓ** (`x3` ⇒ `some(2)`).
- **Trộn đơn vị KHÔNG báo lỗi**: `1kb+1kib` ⇒ `2024b` (probe `y1`).
- `None` là builtin in `<builtin None>` (`r2`); `p"C:\x"` vẫn báo VS0102 (`r1`) — p-string đang xử lý escape.
