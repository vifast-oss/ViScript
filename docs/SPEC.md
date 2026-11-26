<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# SPEC — Đặc tả ngôn ngữ ViScript

**Trạng thái:** bản gốc ST-309 sinh từ cây mã **`c4337ce`** (`0.5.0`, 2026-11-23), đã được Agent 0
đồng bộ theo kiến trúc 5 tầng DAG và hậu Sóng 4 (**`0.6.0`**, `VD-289`..`VD-292`, 2026-11-26, nền
`8ff19b42` / mã `7014fb7b`). Tệp này mô tả **hành vi đã hợp nhất trên `main`**; tính năng chỉ có trên nhánh chưa
merge hoặc chỉ là thiết kế **không** thuộc đặc tả chuẩn tắc và được liệt kê ở §12.

**Thứ tự nguồn sự thật** (khi tệp này lệch): (1) mã + test trong repo; (2) [`SPEC-lex.md`](SPEC-lex.md)
cho từ vựng; (3) tệp này; (4) [`ViScript-thiet-ke.md`](ViScript-thiet-ke.md) (đích thiết kế).
Các từ **PHẢI**, **KHÔNG**, **NÊN** là yêu cầu chuẩn tắc. "Hiện thực" nghĩa là hành vi của
`crates/` tại mốc trên, không phải ý định thiết kế.

**Quy ước vị trí:** mọi offset là **byte** trong văn bản đã NFC; `line` 1-based; `col` đếm theo
**Unicode scalar**, 1-based; tab được tính **4 cột** (và phát `VS0107`).

---

## 1. Tổng quan

ViScript là ngôn ngữ **kiểu tĩnh**, có hai chế độ cú pháp dùng chung một HIR, một bộ kiểm kiểu và
hai engine chạy (interpreter cây và VM bytecode):

| Chế độ | Tệp | Vai trò |
|---|---|---|
| Script | `.vis` | chương trình thông thường: khai báo, biểu thức, câu lệnh, `fn main` |
| Lệnh | `.vish`, REPL, khối `cmd { … }` | pipeline lệnh; hạ bằng **cùng** bảng chữ ký (`cmd_lower`) rồi vào HIR |

Nguyên tắc bắt buộc:

1. Tệp nguồn là **UTF-8**, được chuẩn hoá **NFC toàn tệp trước khi lex**.
2. Không có kiểu `any`, không có `null`; dữ liệu chưa rõ cấu trúc dùng `Json` và chuyển đổi tường minh.
3. Chữ ký hàm cấp tệp và trường bản ghi **phải ghi kiểu**; thân hàm suy kiểu cục bộ.
4. Ngữ nghĩa **giá trị** (gán là sao chép về ngữ nghĩa; cài đặt bằng chia sẻ + copy-on-write).
5. Không quyền ngầm: mọi truy cập hệ thống đi qua `cap[...]` do host trao.
6. Lỗi là giá trị (`Result`/`T?`); `panic`/`assert` chỉ cho lỗi lập trình.
7. Thứ tự đánh giá **trái sang phải**, tất định; `and`/`or`/`??` ngắn mạch.
8. Thay đổi ngữ nghĩa ngôn ngữ phải qua Agent 0 (VD-08).

---

## 2. Từ vựng

Phần này tóm tắt **chuẩn tắc** của [`SPEC-lex.md`](SPEC-lex.md) và bổ sung các điểm mã hiện tại
đã chốt. Nếu mâu thuẫn, `SPEC-lex.md` thắng cho từ vựng.

### 2.1 Tệp, mã hoá, định danh

- Dòng đầu tuỳ chọn `#!/usr/bin/env vis` là chú thích.
- Định danh theo **UAX #31** cộng `_`; chữ số không đứng đầu; so sánh sau NFC.
- Ví dụ hợp lệ: `số_lượng`, `tính_tổng`, `_tmp`, `User`.
- Từ khoá là tiếng Anh (danh sách §2.2). `test`, `assert`, `spawn` là **từ khoá mềm** theo ngữ cảnh.

### 2.2 Từ khoá

Lexer có **40** token từ khoá; **39** từ khoá hiện hành (xếp ABC) — token còn lại là dạng viết thường của `None`, bị từ chối:

```text
alias and async await break cmd const continue derives dyn elif else enum extern false fn for
has if impl in let loop match newtype not or pub ref return scope self trait true type use
var while with
```

Quy tắc đặc biệt:

- `test` **không** nằm trong bảng; chỉ là từ khoá khi đứng đầu item và token kế là `fn`
  (`test fn tên():`). Ở chỗ khác `test` là định danh.
- `assert` là từ khoá mềm ở đầu câu lệnh (xem §3.3); `assert(...)` vẫn là lời gọi.
- `spawn` là định danh mềm: `spawn f(args)` là biểu thức spawn; `spawn(args)` là lời gọi hàm tên `spawn`.
- `alias`, `newtype` và literal viết thường của `None` vẫn là từ khoá của lexer nhưng **bị từ chối** khi dùng (xem §3.1).
- `Some`, `None`, `Ok`, `Err` là **định danh** (constructor/biến thể), không phải từ khoá.

### 2.3 Bảng token

| Kind | Luật | Ví dụ |
|---|---|---|
| `IDENT` | UAX #31 + `_`, NFC | `số_lượng` |
| `KW` | bảng §2.2 | `let`, `fn` |
| `INT` | `[0-9](_?[0-9])*`, `0x…`, `0b…`; `_` chỉ giữa hai chữ số | `42`, `1_000`, `0xff`, `0b101` |
| `FLOAT` | `INT '.' [0-9]+ ([eE][+-]?[0-9]+)?` hoặc `INT [eE][+-]?[0-9]+` | `3.14`, `1e-9` |
| `CHAR` | `'` đúng một scalar `'` | `'a'`, `'ố'` |
| `STRING` | `"…"` có escape + nội suy | `"xin {name}"` |
| `RAW_STRING` | `r"…"` không escape/nội suy | `r"C:\raw"` |
| `MULTI_STRING` | `"""…"""` có escape/nội suy | `"""a\nb"""` |
| `RAW_MULTI` | `r"""…"""` thô | |
| `BYTES` | `b"…"` ASCII + `\xNN` | `b"\x00"` |
| `PATH` | `p"…"` có escape/nội suy; kiểu `path` | `p"./a.txt"` |
| `SIZE` | `INT` + `b\|kb\|mb\|gb\|tb\|kib\|mib\|gib\|tib`, không cách | `5kb`, `5kib` |
| `DURATION` | `INT` + `ns\|us\|ms\|s\|m\|h\|d\|w`, ghép giảm dần, không cách | `250ms`, `2h30m` |
| `PERCENT` | `(INT\|FLOAT)` + `pct` | `50pct`, `12.5pct` |
| `PUNCT` | dấu câu trong §2.4 | `??`, `\|>`, `..=` |
| `NEWLINE` / `INDENT` / `DEDENT` | theo §2.6 | |
| `COMMENT` / `DOC_COMMENT` | `#…` / `##…` đầu dòng | |
| `WORD`, `DOLLAR`, `CARET`, `SET_LBRACE` | chỉ chế độ lệnh / `set{` dính liền | `-la`, `$x`, `^git`, `set{` |

`0b` đứng một mình là `SIZE(0,b)`, không phải tiền tố số; `0x` đứng một mình là lỗi cú pháp.

### 2.4 Dấu câu

```
( ) [ ] { } , : . .. ..= ; -> => = == != < <= > >= + - * / // % ** & | |> ? ?. ?? ! @ # ## ^ ~ \
```

Quy tắc maximal munch:

| Cặp | Luật |
|---|---|
| `.` / `..` / `..=` | dài nhất thắng; `1..2` là `INT RANGE INT` |
| `?` / `?.` / `??` | dài nhất; `x??y` = `x ?? y`; `x?.y` = optional field |
| `\|` / `\|>` | dài nhất; `\|>` pipe biểu thức, `\|` or-pattern/pipe lệnh |
| `*` / `**` | `**` luỹ thừa; `*` nhân/glob |
| `%` | chỉ modulo; `50pct` là percent; `50%` bị từ chối (`VS0202`) |
| `set{` | dính liền là `SET_LBRACE`; `set {` là `IDENT(set)` + `{` |
| `#` | script luôn là chú thích; lệnh chỉ khi đứng đầu từ |
| `@` | luôn là `PUNCT(@)`; ngữ cảnh `cap` do parser/kiểm kiểu quyết |

### 2.5 Literal

- **`int`**: `i64`; tràn literal là **lỗi biên dịch** `VS0104`. `-9223372036854775808` là
  `i64::MIN` hợp lệ (dấu `-` được gộp với literal); `9223372036854775808` dương là `VS0104`.
- **`float`**: `f64`; bắt buộc có chữ số sau `.`; không có literal `NaN`/`inf`. `1.` sai;
  `1..2` là range. `float` in `NaN`, `inf`, `-inf` theo IEEE.
- **`bool`**: `true`, `false`.
- **`char`**: đúng một Unicode scalar sau NFC.
- **Chuỗi**: năm dạng `"…"`, `r"…"`, `"""…"""`, `r"""…"""`, `b"…"`, cộng literal `p"…"` cho `path`.
  Escape hợp lệ: `\n \t \r \0 \\ \" \' \{ \} \u{X..X}` (1–6 hex, ≤ `0x10FFFF`, không surrogate).
  `\xNN` **chỉ** trong `b"…"`. Nội suy `{expr}` hoặc `{expr:spec}`; `{{`/`}}` cho ngoặc thật.
  `r"…"`/`r"""…"""` không escape, không nội suy.
- **`bytes`**: `b"…"` chỉ ASCII + `\xNN`; ký tự > `0x7F` là `VS0106`.
- **`size`**: byte; hậu tố `b kb mb gb tb kib mib gib tib`; `kb`=1000, `kib`=1024.
- **`duration`**: nanosecond; hậu tố `ns us ms s m h d w`, ghép **giảm dần** trong một token.
- **`percent`**: tỉ lệ lưu trong `0..1`; literal dùng `pct` (`50pct` = 0.5).
- **`None`** là constructor của `Option`; literal viết thường của `None` bị từ chối `VS0203`.
- **List/map/set/record**:
  - `[a, b]` → `list[T]`;
  - `{k: v}` → map nếu mọi khoá là biểu thức/chuỗi; **record** nếu mọi khoá là định danh trần;
  - trộn hai chế độ → `VS0207`; `{[k]: v}` (khoá là list) → cảnh báo `VS0208`;
  - `{}` là map rỗng; `set{a, b}` là **list** đã khử trùng (xem §12).

### 2.6 NEWLINE, INDENT, DEDENT

Thuật toán lexer (chuẩn, `SPEC-lex` §4):

1. Đơn vị thụt lề = mức thụt **đầu tiên > 0** trong tệp; mọi mức sau phải là bội của nó; sai `VS0109`.
2. Dòng trống/chỉ có chú thích **không** sinh NEWLINE/INDENT/DEDENT.
3. Trong `() [] {}` bỏ qua NEWLINE/INDENT/DEDENT; đóng ngoặc trả trạng thái ngoài.
4. Chuỗi nhiều dòng nuốt newline bên trong trước khi tính cột.
5. Dòng bắt đầu bằng `|>` ở độ sâu 0 được nối với dòng trước (không sinh NEWLINE).
6. `DEDENT` ngầm ở EOF và cuối mỗi input REPL.
7. Tab phát `VS0107` (cảnh báo-lỗi), tính 4 cột, **vẫn lex tiếp**.

### 2.7 Chế độ lệnh — từ vựng

- Metachar: khoảng trắng, `( ) [ ] { } ; & $ ' " |`; `#` khi đứng đầu từ; `^` ở đầu stage.
- `word` là chuỗi không chứa metachar; `"…"` có nội suy; `'…'` thô; `\` chỉ có nghĩa trong chuỗi.
- `$x` biến phiên; `$env.NAME` biến môi trường; `--` kết thúc cờ; `-5` là số nếu tham số kiểu số,
  ngược lại là cờ; `>` `<` là so sánh, **không** redirect.
- `.vish` và REPL: `shell_item := use | let | fn | cmd | pipeline`.

---

## 3. Cú pháp

Ký hiệu EBNF: `?` tuỳ chọn, `*` lặp 0+, `+` lặp 1+, `|` tuyển, `(* … *)` chú thích. Token viết
HOA do lexer sinh; phần còn lại là terminal.

### 3.1 Tệp và item

```ebnf
file        := NEWLINE* (item | stmt)* EOF
item        := use_decl | fn_decl | cmd_decl | extern_decl | type_item
             | enum_decl | trait_decl | impl_decl | const_decl | test_decl

use_decl    := 'use' use_path
use_path    := ['.' ['/']] IDENT (('.' | '/') IDENT)*
dotted_path := IDENT ('.' IDENT)*

fn_decl     := ['pub'] ['async'] 'fn' IDENT generics? '(' params? ')' ['->' type] ':' suite
cmd_decl    := ['pub'] ['async'] 'cmd' IDENT '(' params? ')' ['->' type] ':' suite
extern_decl := 'extern' 'cmd' IDENT '(' params? ')'
test_decl   := 'test' 'fn' IDENT '(' ')' ':' suite
const_decl  := ['pub'] 'const' IDENT ':' type '=' expr

params      := param (',' param)*
param       := ['...'] IDENT ':' type ['@' path_lit] ['(' grant_list ')'] ['=' expr]
             | ['--'] IDENT ':' type ['@' path_lit] ['(' grant_list ')'] ['=' expr]
             | ['var'] 'self' | '&' 'self'
grant_list  := IDENT (',' IDENT)*

generics    := '[' generic (',' generic)* ']'
generic     := IDENT [':' (dotted_path | 'has' record_type)]

type_item   := ['pub'] 'type' IDENT generics? derives? (
                 '=' type
               | '(' type ')'
               | ':' NEWLINE INDENT field+ DEDENT
               )
field       := IDENT ':' type ['=' expr] NEWLINE
derives     := 'derives' dotted_path (',' dotted_path)*

enum_decl   := ['pub'] 'enum' IDENT generics? derives? (
                 '=' type
               | '(' type ')'
               | ':' NEWLINE INDENT variant+ DEDENT
               )
variant     := IDENT ['(' variant_field (',' variant_field)* ')'] [','] NEWLINE
variant_field := [IDENT ':'] type

trait_decl  := ['pub'] 'trait' IDENT generics? ':' [supertraits ':'] NEWLINE INDENT trait_item+ DEDENT
supertraits := dotted_path (',' dotted_path)*
trait_item  := ['pub'] 'fn' IDENT '(' params? ')' ['->' type] [':' suite] NEWLINE

impl_decl   := 'impl' [dotted_path 'for'] type generics? ':' NEWLINE INDENT impl_item+ DEDENT
impl_item   := ['async'] fn_decl
```

Ghi chú cú pháp:

- `type X = T` là **alias trong suốt**; `type X(T)` là **newtype trong suốt**; `type X:` là bản ghi.
  `alias` và `newtype` (hai từ khoá cũ) bị từ chối bằng `VS0203` kèm gợi ý.
- `enum X = T` / `enum X(T)` được parser chấp nhận như alias/newtype (cùng node), nhưng cách viết
  chuẩn là `type`.
- `pub`/`async` đứng trước `fn/cmd/type/enum/trait/const`; `async` trên `type`/`enum`/`trait`/
  `const` bị kiểm kiểu từ chối (`VS0415`). `async` được hỗ trợ cho `fn` cấp tệp, `fn` trong `impl`
  và `cmd`.
- `const` **bắt buộc** ghi kiểu (`const N = 1` là `VS0204`, chốt ở `VD-291`); chu trình phụ thuộc
  giữa các `const` bị bắt tĩnh bằng **`VS0304`** (`ST-315d`, Tarjan lặp; runtime vẫn giữ hàng rào sâu).
- `test fn` không nhận tham số, không có kiểu trả; thân là `suite`.
- Trường bản ghi bất biến; `= expr` là giá trị mặc định.
- Biến thể enum có thể mang trường **vị trí** (`Circle(float)`) hoặc **có tên** (`Circle(r: float)`);
  nhiều biến thể cùng dòng ngăn bằng `,` (`Circle(float), Empty`).
- `impl` chỉ nhận `type` đã khai trong tệp/đơn vị nạp (record/enum); `impl Trait for Type` và
  `impl Type` (phương thức nội tại) đều hợp lệ. Phương thức trong `impl` không nhận `pub`; chỉ
  `async` được đặt trước `fn`.
- `use std.fs` dùng module chuẩn; `use ./util` nạp `util.vis` tương đối; `use pkg.tên[.module]`
  nạp gói đã cài (xem §8).

### 3.2 Khối và câu lệnh

```ebnf
suite       := simple_stmt NEWLINE
             | NEWLINE INDENT stmt+ DEDENT
simple_stmt := let_stmt | var_stmt | return_stmt | break_stmt | continue_stmt
             | assert_stmt | assign_or_expr
stmt        := simple_stmt | for_stmt | while_stmt | loop_stmt | scope_stmt
             | if_stmt | match_stmt

let_stmt    := 'let' pattern [':' type] '=' expr ['else' [IDENT] ':' suite]
var_stmt    := 'var' ('self' | IDENT) [':' type] '=' expr
assign_or_expr := expr ('=' expr | aug_op expr)?
aug_op      := '+=' | '-=' | '*=' | '/=' | '//=' | '%='
return_stmt := 'return' [expr]
break_stmt  := 'break'
continue_stmt := 'continue'
assert_stmt := 'assert' expr [',' expr]

for_stmt    := 'for' pattern 'in' expr ':' suite
while_stmt  := 'while' if_cond ':' suite
loop_stmt   := 'loop' ':' suite
scope_stmt  := 'scope' [IDENT] ':' suite
if_stmt     := 'if' if_cond ':' suite ('elif' if_cond ':' suite)* ['else' ':' suite]
if_cond     := 'let' pattern '=' expr | expr
match_stmt  := 'match' expr ':' NEWLINE INDENT match_arm+ DEDENT
match_arm   := pattern ['if' expr] ':' suite
```

- Khối một dòng (`if x: return x`) **không** sinh INDENT/DEDENT. Trong ngoặc, khối một dòng kết
  thúc ngầm ở `) ] } ,` hoặc đầu dòng vật lý mới (VD-266).
- `let` nhận **pattern** (tuple/record/list/enum). `else` của `let` phải **thoát trên mọi nhánh**
  (`return`/`break`/`continue`/`panic` hoặc `if`/`match` mọi nhánh thoát), nếu không `VS0206`.
- `var` chỉ nhận một tên (hoặc `self` trong `impl`); không nhận pattern.
- Gán: vế trái là **đường** (`x`, `a.b`, `xs[i].f`) trên gốc `var`; gán vào `let`/tham số là `VS0413`.
- `assert` nhận 1–2 biểu thức; `assert cond, "msg"`; sai ⇒ trap `VS0501` (exit `101`).
- `match` phải có ít nhất một nhánh; guard `if` **không** tính vào phủ.
- `scope` hiện là **khối có nhãn tuỳ chọn**; không phải cơ chế structured-concurrency (xem §12).

### 3.3 Biểu thức

```ebnf
expr        := lambda | pipe_expr
lambda      := 'fn' '(' [lambda_param (',' lambda_param)*] ')' ['->' type] ':' suite
             | '(' [lambda_param (',' lambda_param)*] ')' ['->' type] '=>' expr
             | IDENT '=>' expr
lambda_param := IDENT [':' type]

pipe_expr   := or_expr ('|>' or_expr)*
or_expr     := and_expr ('or' and_expr)*
and_expr    := not_expr ('and' not_expr)*
not_expr    := 'not' not_expr | cmp_expr
cmp_expr    := coalesce [cmp_op coalesce]                 (* không kết hợp *)
cmp_op      := '==' | '!=' | '<' | '<=' | '>' | '>='
coalesce    := range_expr ['??' coalesce]                 (* kết hợp phải *)
range_expr  := add_expr [('..' | '..=') add_expr]         (* không kết hợp *)
add_expr    := mul_expr (('+' | '-') mul_expr)*
mul_expr    := unary_expr (('*' | '/' | '//' | '%') unary_expr)*
unary_expr  := '-' unary_expr | power_expr
power_expr  := postfix ['**' unary_expr]                  (* kết hợp phải *)

postfix     := primary postfix_op*
postfix_op  := '.' (IDENT | KW) | '?.' (IDENT | KW)
             | '[' type (',' type)* ']' args              (* gọi generic *)
             | args | '[' expr ']' | '?'
             | 'with' with_field (',' with_field)*
with_field  := IDENT ':' or_expr

literal     := INT | FLOAT | CHAR | 'true' | 'false'
             | STRING | RAW_STRING | MULTI_STRING | RAW_MULTI
             | BYTES | PATH | SIZE | DURATION | PERCENT
path_lit    := PATH | STRING
map_or_record := '{' [entry (',' entry)* [',']] '}'
entry       := IDENT ':' expr | expr ':' expr
match_body  := NEWLINE INDENT match_arm+ DEDENT
             | (* trong ngoặc: nhánh theo dòng vật lý/cột, VD-266 *)

args        := '(' [arg (',' arg)* [',']] ')'
arg         := expr | '--' IDENT [':' expr] | IDENT ':' expr

primary     := literal | 'self' | IDENT | 'ref' '(' expr ')'
             | 'await' postfix | 'spawn' postfix
             | 'if' if_cond ':' suite ('elif' if_cond ':' suite)* 'else' ':' suite
             | 'match' expr ':' match_body
             | 'cmd' '{' pipeline '}'
             | '(' ')' | '(' expr ')'
             | '(' expr ',' expr (',' expr)* [','] ')'
             | '[' [expr (',' expr)* [',']] ']'
             | 'set{' [expr (',' expr)* [',']] '}'
             | map_or_record
```

Quy tắc phân biệt (lookahead):

- `{ IDENT ':' … }` là **record literal**; nếu mọi khoá là biểu thức/chuỗi thì **map**; trộn ⇒ `VS0207`.
  Khoá tính toán chính thức là `(expr)`; `{[k]: v}` cảnh báo `VS0208`.
- `name[types](args)` là **gọi generic** khi sau `]` là `(`; ngược lại `name[expr]` là index.
- `'(' … ')' '=>'` (hoặc `-> type =>`) là lambda; ngược lại là nhóm/tuple.
- `set{` dính liền là set literal; `set {` là định danh + khối/map.
- `cmd` + `{` là khối lệnh; `cmd` khác là định danh.
- `spawn` chỉ là spawn khi token kế là định danh hoặc `self`; `spawn(...)` là lời gọi hàm.
- `if`/`match` dùng như biểu thức **phải có `else`/đủ nhánh**; dạng câu lệnh thì `else` tuỳ chọn.
- `|` **không** là toán tử nhị phân trong biểu thức; chỉ có trong `or_pattern` và pipeline lệnh.

### 3.4 Pattern

```ebnf
pattern     := range_pattern ('|' range_pattern)*
range_pattern := atom_pattern [('..' | '..=') [atom_pattern]]
atom_pattern := IDENT ['(' [pattern (',' pattern)*] ')']
              | 'true' | 'false' | literal | '-' INT
              | '(' [pattern (',' pattern)*] ')'
              | '[' [pattern (',' pattern)*] [',' '..' [IDENT]] ']'
              | '{' [record_pat (',' record_pat)*] [','] '}'
record_pat  := IDENT [':' pattern] | '..'
```

- `None`, `Some(x)`, `Ok(v)`, `Err(e)` là **định danh** được phân giải thành biến thể; viết
  thường của `None`/`Some` là `VS0203`.
- Tên biến thể là **trần** (`Circle(r)`); dạng có tiền tố (`Shape.Circle(r)`) **chưa** được parser
  hỗ trợ (xem §12).
- List rest: `[a, ..]`, `[a, ..rest]` cần ≥1 phần tử; `[..]`/`[..rest]` khớp mọi list kể cả rỗng;
  `..` phải ở cuối.
- Pattern bản ghi `{name, age}` / `{name: n, ..}`; `..` bỏ các trường còn lại.
- Range pattern chỉ dùng cho kiểu sắp được; `for` chỉ nhận range `int` (`VS0416`).

### 3.5 Kiểu

```ebnf
type        := type_primary ('?' | '??')*
type_primary := dotted_path ['[' type (',' type)* ']']
              | 'ref' '[' type ']'          (* cú pháp; tyck từ chối, xem §12 *)
              | 'weak' '[' type ']'         (* cú pháp; tyck từ chối, xem §12 *)
              | 'cap' '[' dotted_path ']'
              | 'fn' '(' [type (',' type)*] ')' '->' type
              | '(' [type (',' type)*] ')'
              | '{' record_type '}'
record_type := IDENT ':' type (',' IDENT ':' type)*
```

- `T?` = `Option[T]`; `T??` = `Option[Option[T]]`; mỗi token `?` thêm **một** lớp, token `??`
  thêm **hai** lớp. `T???` hợp lệ. Không làm phẳng optional.
- `cap[Kind]` chỉ nhận `Kind` ∈ {`Dir`, `File`, `Net`, `Proc`, `Env`, `Term`}; `(write)` chỉ hợp lệ
  trên `Dir`/`File`; `@ path`/`@ "host"` chỉ xuất hiện trên **tham số**, không trong kiểu.
- `fn(T, …) -> U` là kiểu hàm; `()` là unit; `(A, B)` là tuple; `{a: A, b: B}` là record ẩn danh.
- `list[T]`, `map[K, V]`, `Result[T, E]`, `Option[T]`, `Chan[T]`, `Task[T]` là kiểu dựng sẵn.

### 3.6 Chế độ lệnh

```ebnf
shell_file  := shell_item*
shell_item  := use_decl | fn_decl | cmd_decl | shell_let | pipeline NEWLINE
shell_let   := 'let' IDENT [':' type] '=' pipeline

pipeline    := stage ('|' stage)* ['&']
stage       := word_cmd arg* | '(' expr ')'
word_cmd    := WORD | '^' WORD | '$' IDENT ('.' IDENT)*
arg         := WORD | flag | STRING | RAW_STRING | '(' expr ')' | closure_arg | '[' word ']'
flag        := '--' IDENT ['=' expr] | '-' IDENT ['=' expr] | '--'
closure_arg := IDENT '=>' closure_body | field_shorthand
closure_body := expr            (* parser cắt tới | ; & NEWLINE ở độ sâu 0 *)
field_shorthand := expr         (* biểu thức dùng `it` ngầm; cmd_lower phân giải *)
```

- Pipeline nối bằng `|` ở **độ sâu ngoặc 0**; `;` tách nhiều pipeline trong một dòng; `&` chỉ ở
  **cuối dòng** (chạy nền — hiện `VS0605`).
- `^tên` buộc gọi chương trình ngoài (hiện `VS0606` nếu không có host); tên lạ là chương trình ngoài
  (trừ `--no-extern`).
- `|>` trong chế độ lệnh là **lỗi cú pháp**.
- Tham số `--name`/`-n` bind vào tham số `--name` của `cmd`; `--` kết thúc cờ.
- `$x` biến phiên; `$env.NAME` biến môi trường; biến chưa bind là lỗi.
- `.vish`: dòng cột 1 bắt đầu bằng `use/fn/cmd/type/enum/trait/impl/const/...` là khai báo; `let`
  ở cột 1 là khai báo nếu RHS là biểu thức script và mọi tên đã bind, ngược lại là shell-let.
- `fn main` trong `.vish` chỉ dùng để khai báo `cap`; nếu thân có câu lệnh ⇒ `VS0201` (ST-93b).

---

## 4. Hệ kiểu và ngữ nghĩa tĩnh

### 4.1 Kiểu dựng sẵn

| Nhóm | Kiểu |
|---|---|
| Số | `int` (`i64`), `float` (`f64`) |
| Logic/ký tự | `bool`, `char` |
| Văn bản | `str` (UTF-8, bất biến), `bytes` |
| Hệ thống | `path`, `size`, `duration`, `percent` |
| Tập hợp | `list[T]`, `map[K, V]`, tuple `(A, B, …)` |
| Rỗng/lỗi | `()`, `never`, `T?`, `Result[T, E]`, `Error`, `AnyError` |
| Bất đồng bộ | `Task[T]`, `Chan[T]` |
| Động | `Json`, `SqlValue`, `RowSet` (chỉ khi `use std.sqlite`) |
| Quyền | `cap[Dir]`, `cap[File]`, `cap[Net]`, `cap[Proc]`, `cap[Env]`, `cap[Term]` |
| Hàm | `fn(A, B) -> C` |

- `Option[T]` tương đương `T?`; `Result[T, E]` là enum thật; `Error` là bản ghi dựng sẵn
  `{message: str, source: Error?, span: str?}`; `AnyError` là **alias** của `Error`.
- `Json` là enum dựng sẵn `Null | Bool(bool) | Num(int|float) | Str(str) | Arr(list[Json]) | Obj(map[str, Json])`.
- `SqlValue`/`RowSet` chỉ hiện khi `use std.sqlite`.
- `never` là kiểu của `panic(...)` và biểu thức phân kỳ (`return`, `break`, `continue`).
- **Không** có trong mã hiện tại: `ref[T]`, `weak[T]`, `set[T]`, `stream[T]`, `Future[T]`,
  `Iterator[T]`, `datetime`, `instant`, `Service[P]`, `dyn Trait`, `decimal`, `bigint` (xem §12).

### 4.2 Kiểu người dùng

```text
type User derives Eq, Show, Json:
    name: str
    age: int?
    email: str? = None

enum Shape derives Show:
    Circle(r: float)
    Rect(w: float, h: float)
    Empty

type UserId = int          # alias trong suốt
type Email(str)            # newtype trong suốt; Email("a@b") / let Email(s) = e
```

- Trường bản ghi bất biến; `u with age: 21` trả bản ghi mới.
- Tạo bản ghi: `User(name: "An", age: 20)` (tham số **có tên**, có thể bỏ trường có mặc định);
  literal `{name: "An", age: 20}` cần kiểu đích.
- Enum: dựng bằng `Circle(2.0)` hoặc `Circle(r: 2.0)`; pattern dùng tên trần.
- `derives` chỉ nhận **`Eq`, `Show`, `Json`**. `Eq` từ chối trường `float`/hàm; `Json` từ chối
  `cap`/`Chan`/`Task`/hàm; derive trên kiểu generic bị từ chối `VS0403`.
- `type`/`enum` generic dùng `[T]`; khởi tạo `Hop[int]`.
- Alias/newtype **trong suốt**: cùng kiểu, cùng runtime; không có kiểm kiểu mới.

### 4.3 Suy kiểu và chuyển đổi

- Suy kiểu **cục bộ hai chiều** trong một hàm; không suy toàn chương trình.
- Bắt buộc ghi: chữ ký hàm cấp tệp (kiểu trả khi khác `()`), trường bản ghi, `const`, kiểu khai
  báo `type`/`enum`.
- `let`/`var` có thể bỏ kiểu nếu RHS đủ thông tin; literal rỗng `[]`/`{}` hiện được chấp nhận
  với kiểu `{error}` (xem §12 — không bắt buộc chú thích như thiết kế).
- Không chuyển đổi ngầm giữa số; ngoại lệ **duy nhất**: literal `int` ở vị trí `float`
  (`let x: float = 1` hợp lệ; biến `int` thì không).
- `str(x)` = `x.show()`; in dùng `Show`. Các hàm chuyển đổi `int(x)`/`float(x)`/`path(x)` **chưa**
  có trong mã hiện tại (xem §12).
- `str` không có `s[i]`; dùng `.chars()`, `.byte_len()`, `.char_at(i)`, `.split(...)` (các hàm
  hiện có trong API `std.str`).

### 4.4 Optional và `Result`

- Tự nâng `T → T?` **một lớp** ở: `let`, gán lại `var`, đối số, `return`, biểu thức cuối thân hàm
  (hoặc closure `fn(…) -> T?:`) và trường của constructor bản ghi `T(...)`. **Không** nâng bên
  trong literal `{…}`, phần tử `list`, hay entry `map` — ở đó phải bọc `Some(…)` cho đồng nhất.
  Không nâng lặp nhiều lớp.
- Với đích `T??`:

  | Nguồn | Kết quả |
  |---|---|
  | `None` | `None` lớp ngoài |
  | `Some(None)` | giữ `Some(None)` |
  | `Some(Some(x))` | giữ |
  | `Some(x)` | nâng một lớp → `Some(Some(x))` |
  | `x` (không optional) | `VS0412` (không nâng hai lớp) |

- `?` trên `T?` chỉ trong hàm trả `U?`; trên `Result[T, E]` chỉ trong hàm trả `Result[U, F]`.
- `?.` trên `T?` trả `T?`; gọi method qua `?.` không đánh giá đối số khi `None`.
- `??` nhận vế phải cùng kiểu đã bỏ optional; ngắn mạch.
- `xs[i]` panic (`VS0503`) khi ngoài phạm vi/âm; `xs[i]?` trả `T?`; `.get(i)` trả `T?`.
- `if let PAT = EXPR:` / `while let PAT = EXPR:` nhận pattern; pattern không khớp thì rẽ nhánh
  `else`/thoát vòng.
- `let PAT = EXPR else: …` dùng cho pattern có thể không khớp; khối `else` phải thoát.

### 4.5 `match` và phủ

- Phải xét hết; thiếu nhánh là `VS0402` (liệt kê nhánh thiếu).
- Guard `if` **không** chứng minh phủ; kiểu vô hạn (`int`, `float`, `str`, …) bắt buộc có `_`.
- Range pattern không dùng để chứng minh phủ kiểu vô hạn.
- Or-pattern `A(x) | B(x)`; guard, pattern lồng, list rest, record pattern.
- `match` là biểu thức khi mọi nhánh cho giá trị; dùng làm câu lệnh cũng được.

### 4.6 Số học và toán tử

Bảng ưu tiên (cao → thấp):

| Mức | Nhóm | Toán tử | Kết hợp |
|---|---|---|---|
| 1 | postfix | `.` `?.` `()` `[]` `?` `with` `[T]` | trái |
| 2 | luỹ thừa | `**` | phải |
| 3 | một ngôi | `-` | phải |
| 4 | nhân | `* / // %` | trái |
| 5 | cộng | `+ -` | trái |
| 6 | khoảng | `.. ..=` | không kết hợp |
| 7 | coalesce | `??` | phải |
| 8 | so sánh | `== != < <= > >=` | không kết hợp |
| 9 | phủ định | `not` | phải |
| 10 | hội | `and` | trái, ngắn mạch |
| 11 | tuyển | `or` | trái, ngắn mạch |
| 12 | nối | `\|>` | trái |

Hệ quả: `-2 ** 2` = `-(2 ** 2)` = `-4`; `2 ** 3 ** 2` = `512`; `1 < x < 10` là lỗi cú pháp;
`a ?? b == c` = `(a ?? b) == c`; `not a == b` = `not (a == b)`.

Kiểu kết quả (bảng chuẩn của `tyck/num.rs`):

| Toán tử | Kiểu hợp lệ | Kết quả |
|---|---|---|
| `+ -` | `int`, `float`, `size`, `duration`, `percent` cùng kiểu; `str + str` | cùng kiểu / `str` |
| `*` | `int*int`, `float*float`, `size*int`, `size*percent`, `duration*int`, `duration*percent`, `percent*int` | kiểu đơn vị tương ứng |
| `/` | `int/int` | **luôn `float`** |
| `//` | `int/int` | `int` (làm tròn **xuống**) |
| `%` | `int%int`, `size%int`, `size%size`, `duration%int`, `duration%duration`, `percent%percent` | cùng kiểu |
| `**` | `int**int`, `float**float` | cùng kiểu; mũ âm với `int` là trap `VS0501` |
| `size / size`, `duration / duration` | | `float` |
| `size / int`, `duration / int`, `percent / int` | | cùng kiểu |
| `percent / percent` | | `float` |

Ngữ nghĩa số:

- `int` tràn lúc chạy = trap `VS0507`; literal tràn = lỗi biên dịch `VS0104`.
- `/0`, `//0`, `%0` với `int` = trap `VS0502`; `float / 0.0` theo IEEE (±inf/NaN, không trap).
- `//` làm tròn xuống; `%` cùng dấu **số chia** (`-7 % 2 == 1`, `7 % -2 == -1`).
- `NaN == NaN` là `false`; float không có `Eq`; so sánh thứ tự dùng `total_cmp` trong sắp xếp.
- `size`/`duration`/`percent` hiển thị theo R10: `size` thang thập phân (`1024` → `1.02kb`),
  `duration` ghép giảm dần (`150m` → `2h30m`), `percent` dạng `50pct`; phép khác đơn vị là
  `VS0412`/`VS0414`.
- Bit: thiết kế có `bit_and/bit_or/bit_xor/bit_not/shl/shr/ushr`, nhưng **chưa có** trong API
  `std.*` tại mốc này (xem §12). `|` **không** phải bit-or.

### 4.7 Hàm, closure, generic, trait

- Tham số: vị trí, mặc định `= expr`, có tên `--x: T` (script truyền `f(--x: 1)`), variadic
  `...rest: T` đặt cuối. Đối số có tên `b: 5` khớp theo tên, phải đứng sau đối số vị trí.
- Biểu thức cuối thân hàm là giá trị trả về; `return` trả sớm; `never` tương thích mọi kiểu.
- Tham số bất biến; chỉ `var self` được gán. `&self` được chấp nhận và hạ như `self`.
- Closure: `x => expr`, `(a, b) => expr`, `(x: T) -> U => expr`, `fn(x: T) -> U: suite`.
  Closure bắt biến theo giá trị; không có closure đa hình.
- Generic: `fn f[T](x: T) -> T`; ràng buộc `[T: Show]`, `[R: has {size: size}]`; gọi tường minh
  `f[int](x)`.
- Trait: `trait T:` + method chữ ký (có thể có thân mặc định); `impl T for X:`; `impl X:` method
  nội tại. Không có `dyn`/trait object; không có kế thừa cài đặt ngoài method mặc định.
- `Show` dựng sẵn: `fn show(self) -> str`; `Error: Show` với `message/source/span`.
- `derives Show/Eq/Json` sinh method thật; `Json` sinh `to_json() -> Json` và
  `Type.from_json(j: Json) -> Result[Type, Error]`.
- Không có overload theo kiểu; khai trùng tên cùng phạm vi là `VS0302`.

### 4.8 Phạm vi, che tên, bất biến

- `let` bất biến; `var` đổi được; `let` **được che tên** trong cùng phạm vi (R16); `var` khai lại
  cùng phạm vi là `VS0302`.
- Tham số, biến `for`, binding `match`/`if let` cũng là khai báo; che tên ngoài, không khai lại
  trong cùng phạm vi.
- Gán cho `let`/tham số là `VS0413`; gán lồng `a.b.f = v`/`xs[i].f = v` chỉ trên gốc `var`.

### 4.9 Giá trị, bộ nhớ, thứ tự

- Mọi kiểu trừ `cap`/`chan`/`task`/`fn` có **ngữ nghĩa giá trị**; gán/sao chép độc lập về ngữ nghĩa.
- Cài đặt RC + copy-on-write; không GC truy vết; không có chu trình qua giá trị thường.
- `map` giữ **thứ tự chèn** (`Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười từ `ST-315b`); khoá
  trùng trong literal tĩnh là `VS0412`, khoá tính toán trùng thì runtime **last-wins** (giữ khoá gốc
  và vị trí chèn đầu; `remove` giữ thứ tự còn lại, `set` lại chèn cuối). So sánh map theo ánh xạ,
  không theo thứ tự.
- `set{…}` khử trùng theo `==`, nhưng kiểu là `list[T]` (xem §12).
- Đánh giá trái sang phải: toán hạng, phần tử literal, trường bản ghi, đối số (vị trí trước, tên sau).
- `and`/`or`/`??` ngắn mạch; toán hạng phải không được đánh giá khi trái quyết định.
- `print(a, b, …)` in từng giá trị bằng `show`, cách nhau **một** dấu cách; `print()` in dòng trống.
  `print(cap)` bị kiểm kiểu từ chối `VS0412` (`VD-106`); dùng `str(cap)` nếu cần chuỗi hiển thị.
- `show`: `str` in nguyên văn; `char` bọc `'…'` và escape điều khiển; `bytes` bọc `b"…"`;
  list `[a, b]`; map `{k: v}`; record `Tên(f: v, …)`; enum `BiếnThể(v, …)`; `Error` chỉ in
  `message`; task `<tác vụ>`, kênh `<kênh>`, cap `<cap[Kind] root>` (qua `str(cap)`).

---

## 5. Bất đồng bộ, ngân sách, worker

### 5.1 `async`, `Task`, `await`, `spawn`

- `async fn` cấp tệp, `async fn` trong `impl`, và `async cmd` được hỗ trợ. `async` trên trait
  method bị từ chối; `async` trên biến/thân hàm khác là `VS0415`.
- Gọi `async fn` **không chạy thân**; trả `Task[T]`. `await t` chạy/hoàn tất task và trả `T`.
- `spawn f(args)` / `spawn obj.m(args)` tạo task đã đăng ký; chỉ nhận hàm/phương thức `async`.
  `async fn` không dùng làm **giá trị** (không có `Future`).
- Mô hình mặc định một luồng; chương trình có `async` chạy trên **runtime frame suspend/resume +
  reactor/timer** (cả interp và VM); thứ tự khởi tạo/enqueue tất định, kênh FIFO.
- `Task` không huỷ được từ ngôn ngữ; không có cú pháp `select` tổng quát (kênh có `recv_timeout`/
  `send_timeout` + `select_recv*`/`select_send*`); `scope` chưa phải structured concurrency (§12).
- Lỗi trong thân task bọc `VS0506` (kèm mã gốc); `await`/join ném lại.

### 5.2 Kênh

- `chan[T](n)` trả `Chan[T]`; `n >= 1` trên đường đồng bộ; `chan(0)` bị từ chối `VS0505` ở đường
  thường và hỗ trợ rendezvous trên đường frame `async` (xem §12/thiết kế).
- Method/hàm kênh: `send(v)`, `recv() -> T?`, `close()`, `recv_timeout`/`send_timeout`, và
  `select_recv*`/`select_send*` (đường frame); gửi sau `close` là `VS0504`; recv sau close trả hết
  hàng rồi `None`.
- Kênh đầy/rỗng mà không còn task chạy được ⇒ deadlock `VS0505` (không treo).
- Kênh truyền được qua tham số/`Chan[T]`; giá trị gửi phải `Sendable`; vi phạm `VS0508`.

### 5.3 Ngân sách thực thi

- Mặc định **40.000.000** đơn vị ngữ nghĩa cho mỗi lần chạy (`--max-budget N` để đổi; `N > 0`).
- `--unlimited-budget` tắt ngân sách ngữ nghĩa; hàng rào fuel nội bộ của VM vẫn giữ.
- Vượt ngân sách ⇒ `VS0510`, exit `101`.
- Ngân sách dùng chung cho worker/task khi `--workers > 1`; worker reserve theo lô, trả dư.

### 5.4 Worker song song

- `--workers N` (N ≥ 1) chọn số worker evaluator cho tác vụ CPU; `N = 1` giữ đường tuần tự cũ.
- `N > 1` dùng worker pool opt-in; mỗi worker có heap riêng; task nhận snapshot biến top-level.
- Ghi biến cấp tệp từ worker khác worker sở hữu ⇒ `VS0509` (không có chia sẻ ngầm).
- Không work-stealing; không song song mặc định; async vẫn một luồng logic.

---

## 6. Quyền `cap[...]` và host

### 6.1 Mô hình

- Không hàm toàn cục nào chạm hệ thống; thư viện nhận `cap[Dir]`/`cap[File]`/`cap[Net]`/`cap[Proc]`.
- Nguồn quyền: tham số `main` khai `cap`, hoặc `ctx` của phiên (khi có), hoặc dẫn xuất hẹp hơn.
- `print` là stdout của tiến trình, không phải cap. `std.fs`/`std.net`/`std.http` cần cap tương ứng
  khi cưỡng chế; thao tác tệp của `std.sqlite` cũng đi qua kiểm tra FS của host khi siết.
- `cap` không so sánh được (`==` là `VS0412`); không truyền quyền ngầm cho plugin.

### 6.2 Khai báo trên `main`

```text
fn main(data: cap[Dir] @ p"./data", out: cap[Dir] @ p"./out" (write), net: cap[Net] @ "api.example.com") -> Result[(), Error]:
    Ok(())
```

- `@` trên `cap[Dir]`/`cap[File]` là `path` literal (`p"…"` hoặc `"…"`); trên `cap[Net]` là host
  string. `(write)` chỉ cho `Dir`/`File`; `cap[Net] (write)` là `VS0412`.
- `--grant tên=đường-dẫn` **thay root** của cap đã khai, không thêm quyền mới; tên không khai là
  lỗi dùng (exit 2). `cap[Proc]` nhận nhiều `--grant` làm allowlist lệnh.
- `--dry` in danh sách cap (stdout) rồi thoát 0, không chạy.

### 6.3 Cưỡng chế

- Mặc định: chỉ cưỡng chế khi chương trình khai báo `cap`; `--strict-caps` siết **mọi** truy cập
  fs/net/proc, kể cả khi không khai.
- Chương trình dùng gói `use pkg.*` **mặc định siết** (VD-124); `--no-strict-caps` thoát và in
  cảnh báo stderr.
- Vi phạm quyền ở biên host ⇒ message có dấu `[quyền]`, mã `VS0604`, exit **2**.
- `--sandbox` áp Landlock trên Linux cho **hệ tệp**; `cap[Net]`/`cap[Proc]` vẫn cưỡng chế runtime.
  Nền khác Linux: `--sandbox` không cưỡng chế được ⇒ exit **3**.
- `--sandbox-check` in 3 dòng (landlock / cưỡng chế runtime / cap điều kiện) và thoát `0` (Linux có
  Landlock), `3` (không có/nền khác), `4` (không đọc được LSM), không chạy chương trình.
- Đường đọc, ghi và truy vấn siêu dữ liệu (`size`/`list`/`exists`/`walk`/`read_link`/`canonicalize`)
  trên Unix dùng `openat`/`openat2` + `statat`/`readlinkat` theo `fd` (`ST-295`/`ST-311b`, `#194` đóng);
  Windows chỉ kiểm theo đường dẫn (`FsGuard`), không có danh tính tệp.

### 6.4 Mức bảo đảm theo nền

| Nền | Cưỡng chế runtime | Kernel sandbox | Trạng thái |
|---|---|---|---|
| Linux (`ps`) | `FsGuard`/`NetGuard` (`openat2`/`statat`), `VS0604` | Landlock `ABI::V3` (`--sandbox`, `vis-host-linux` trong workspace; `ReadFile\|WriteFile\|Truncate` cho tệp đơn `(write)`, `#197` đóng ở `ST-315c`) | chạy thật, host suite 54/54; `vis-host-linux` 19/19 |
| Windows x86_64 | cùng mã, **chỉ theo đường dẫn** | không | **Tier 1** từ `v0.4.5`; caveat `FsGuard` |
| macOS arm64 | cùng mã (`openat`/`statat`, `FsGuard`/`NetGuard`) | không (Darwin không có Landlock) | **Tier 1 đã công bố** từ `v0.5.0` (`ST-300m3`: 3× full + soak 4 h 1.878 vòng, host 54/54); chỉ arm64; clippy **PASS @`311d2288`**; **chưa chạy lại full gate trên `0.6.0`** (`mps` bỏ qua theo chỉ đạo, bù ở `ST-316c`) |
| `wasm32-wasip1` | `WasiHost` preopen; từ chối net/proc/sqlite | preopen của runtime | runner thật (runtime có phạm vi); suite 28/28, `vis-wasm-sem` **390/390** (`#193` đóng với `-zstack-size=12 MiB` + `node --stack-size=4096`), `vis-wasm-conformance` **393×2** |
| `wasm32-unknown-unknown` | — | — | **chỉ `cargo check` biên dịch chéo**, không chạy |

Không được viết "macOS/Windows có Landlock" hoặc "wasm32-unknown-unknown chạy được".

---

## 7. Chẩn đoán, mã lỗi, mã thoát

### 7.1 Định dạng

```text
tệp.vis:dòng:cột: error VS0412: kiểu không khớp
```

- `dòng`/`cột` 1-based; cột theo scalar; `vis check --json` phát một object JSON nén.
- Mỗi tệp lỗi cú pháp chỉ báo **một** lỗi đầu (không dồn).
- `vis explain VSxxxx` giải thích mã; `--lang vi|en|ja`; `--list` in toàn bộ sổ; `--json`.

### 7.2 Sổ mã

Sổ đóng băng **57 mã** đồng bộ theo `VD-292` (nguồn máy đọc: `agent/API-CODES.txt`):

| Mã | Mức | Nghĩa ngắn |
|---|---|---|
| `VS0101` | error | chuỗi chưa đóng |
| `VS0102` | error | escape không hợp lệ |
| `VS0103` | error | ký tự lạ ngoài bảng token |
| `VS0104` | error | literal số sai/tràn `i64` |
| `VS0105` | error | char literal không đúng 1 scalar |
| `VS0106` | error | byte string có ký tự > `0x7F` |
| `VS0107` | warning | tab (cảnh báo-lỗi; tab = 4) |
| `VS0108` | error | nội suy `{` không đóng |
| `VS0109` | error | thụt lề không khớp đơn vị/DEDENT sai |
| `VS0110` | error | `^` sai ngữ cảnh |
| `VS0111` | warning | tên biến thể nên viết hoa — **ngừng phát từ VD-212** (giữ số) |
| `VS0201` | error | lỗi cú pháp: token không mong đợi/thiếu vế |
| `VS0202` | error | `%` dùng như phần trăm — viết `50pct` |
| `VS0203` | error | từ khoá đã bỏ — dùng cú pháp mới |
| `VS0204` | error | `const` cần ghi kiểu tường minh *(chuỗi sổ đóng băng `API-CODES.txt` giữ theo `VD-129`)* |
| `VS0205` | error | dựng bản ghi bằng `{…}` — dùng `Tên(trường: …)` hoặc khoá chuỗi |
| `VS0206` | error | khối `else` của `let … else` phải thoát |
| `VS0207` | error | trộn loại khoá trong `{…}` |
| `VS0208` | warning | `{[k]: …}` khoá là danh sách |
| `VS0211` | warning | tên `X` trùng module std đã import |
| `VS0301` | error | tên không tồn tại |
| `VS0302` | error | khai báo trùng trong cùng phạm vi |
| `VS0303` | error | kiểu không tồn tại |
| `VS0304` | error | chu trình phụ thuộc hằng số (`const`) *(cấp ở `VD-291`/`ST-315d`)* |
| `VS0401` | error | sai số lượng hoặc thứ tự đối số |
| `VS0402` | error | `match` không xét hết trường hợp |
| `VS0403` | error | trường, kiểu hoặc phương thức không tồn tại |
| `VS0405` | error | không có trường này trên kiểu bản ghi |
| `VS0412` | error | kiểu không khớp |
| `VS0413` | error | gán vào ràng buộc bất biến (`let`) |
| `VS0414` | error | toán tử không hỗ trợ cho kiểu này |
| `VS0415` | error | `async` sai chỗ |
| `VS0416` | error | range chỉ nhận `int` (cho `for`) |
| `VS0501` | error | lỗi runtime: phép toán/tính năng chưa hỗ trợ (gồm `panic`/`assert`) |
| `VS0502` | error | chia hoặc lấy dư cho 0 |
| `VS0503` | error | chỉ số ngoài phạm vi |
| `VS0504` | error | gửi vào kênh đã `close` |
| `VS0505` | error | deadlock kênh |
| `VS0506` | error | tác vụ `async fn` lỗi (kèm mã gốc) |
| `VS0507` | error | tràn `i64` lúc chạy |
| `VS0508` | error | giá trị không gửi được sang tác vụ khác |
| `VS0509` | error | ghi biến cấp tệp chéo worker (`W > 1`) |
| `VS0510` | error | vượt ngân sách thực thi ngữ nghĩa |
| `VS0601` | error | không có lệnh này trong bảng chữ ký |
| `VS0602` | error | sai đối số lệnh |
| `VS0603` | error | pipeline không nối được |
| `VS0604` | error | tính năng chưa hỗ trợ (gồm vi phạm quyền) |
| `VS0605` | error | chạy nền `&` chưa hỗ trợ |
| `VS0606` | error | chương trình ngoài `^…` chưa hỗ trợ |
| `VS0701` | error | không thấy tệp module |
| `VS0702` | error | vòng phụ thuộc module |
| `VS0703` | error | `use` trùng hoặc mơ hồ |
| `VS0704` | error | không giải được ràng buộc gói |
| `VS0705` | error | `std` chưa hỗ trợ |
| `VS0706` | error | `pkg` chưa hỗ trợ |
| `VS0707` | error | hash gói không khớp `vis.lock`/sổ |
| `VS0708` | error | chữ ký/khoá tin cậy registry không hợp lệ |

### 7.3 Mã thoát

| Mã | Nghĩa |
|---|---|
| `0` | thành công; `main` trả `Ok`/giá trị thường |
| `1` | `main` trả `Err`; `vis check`/`vis fmt --check` có chẩn đoán; `vis test` có ca đỏ |
| `2` | lỗi dùng, lỗi cú pháp/kiểu **trước khi chạy**, vi phạm quyền `[quyền]` |
| `3` | `--sandbox`/`--sandbox-check` trên nền không cưỡng chế được |
| `4` | `--sandbox-check` không đọc được trạng thái LSM |
| `101` | trap runtime (`VS05xx`), `panic`, `assert` sai, vượt ngân sách |
| `126`/`127`/`130` | dự phòng POSIX cho chương trình ngoài/bị huỷ (khi host tương ứng có) |

`main -> int` **không** phải mã thoát: trả `3` vẫn thoát `0`. Chỉ `Err` của `main` mới thoát `1`.

---

## 8. Module, gói, thư viện chuẩn

### 8.1 Module

- Mỗi tệp là một module; `pub` xuất item, mặc định riêng tư.
- `use std.fs`, `use ./util`, `use pkg.tên[.module]`.
- Nạp đa tệp: DFS hậu duyệt theo thứ tự `use`, mỗi tệp một lần; cấm vòng (`VS0702`), `use` trùng
  (`VS0703`), thiếu tệp (`VS0701`).
- `std.*` chỉ là các module chuẩn có trong §8.3; module khác là `VS0705`.
- `pkg.*` đọc `.vis/pkg` theo `vis.lock`, kiểm SHA-256 nội dung; lệch ⇒ `VS0701` (gói bị sửa).

### 8.2 Gói (`vis pkg`)

- Mô hình **đang chạy**: registry thư mục cục bộ (mặc định `./registry`) hoặc HTTP(S); mỗi gói có
  `manifest.json` khai `name`, `version`, `entry`, `deps`, `files[{path, sha256}]`.
- Nội dung tệp được chuẩn hoá (bỏ BOM, CRLF→LF, đúng một LF cuối) khi ingest; hash cây theo
  `vispkg1` + danh sách `(path, sha256)` sắp theo byte.
- `vis.lock` định dạng 2 ghi `roots` + `version_req` hiệu lực + mọi gói đã cài; `install` chỉ đọc
  lock + `.vis/store/<sha256>`, không cần registry.
- Resolver semver chọn **một** phiên bản cho mỗi gói, cao nhất thoả mọi ràng buộc; xung đột ⇒
  `VS0704`. Không phải MVS.
- HTTP(S): tải `index.json`, chọn phiên bản, tải tarball; kiểm SHA-256 tarball và từng tệp trước khi
  ghi `.vis`; hash sai ⇒ `VS0707`. `--registry-key`/`VIS_REGISTRY_KEY` bật kiểm **Ed25519**
  (thông điệp `vispkg-sig1`); sai/thiếu ⇒ `VS0708`.
- `gc` dọn blob/phiên bản không còn trong lock; `migrate` nâng lock cũ lên định dạng 2.
- **Chưa** có: `vis.toml` làm nguồn phụ thuộc, MVS, `sum.vifast.org`, proxy, tải theo đường dẫn.

### 8.3 Thư viện chuẩn (mặt API đóng băng)

Gọi `module.hàm(...)` sau `use std.module`. Danh sách chữ ký đầy đủ ở `agent/API-STD.txt`; đây là
các nhóm hiện có:

| Module | Hàm |
|---|---|
| `std.fs` | `append canonicalize copy current_dir exists glob is_dir is_file is_symlink list mkdir mkdir_all read read_bytes read_lines read_link remove rename size walk write write_bytes` |
| `std.path` | `ancestors basename components dirname ext is_abs is_absolute is_relative join join_all normalize relative stem strip_prefix with_extension with_file_name with_stem` |
| `std.str` | `split rsplit splitn split_once rsplit_once split_inclusive split_terminator join upper lower title trim trim_start trim_end trim_matches trim_start_matches trim_end_matches replace replace_first replace_last replace_n parse_int parse_float repeat starts_with ends_with contains strip_prefix strip_suffix find rfind count matches lines split_whitespace pad_left pad_right center byte_len char_at char_count is_empty is_ascii is_ascii_digit is_ascii_alphabetic is_ascii_alphanumeric is_ascii_whitespace is_blank` |
| `std.list` | `sort sort_by take drop chunk chunks_exact windows concat flatten reverse unique zip min max sum product mean median sum_float min_float max_float is_empty index_of last_index_of contains binary_search` |
| `std.map` | `keys values merge remove get_or is_empty contains_key` |
| `std.json` | `encode decode encode_pretty` |
| `std.time` | `now_ms monotonic_ms` |
| `std.net` | `listen connect read write accept close close_listen local_addr accept_timeout connect_timeout` |
| `std.http` | `get head get_timeout head_timeout` (HTTP/1.1 & HTTP/2; `http://` chỉ loopback, `https://` qua TLS; `res.headers` kiểu `map[str, str]` áp **last-wins** cho header trùng tên theo `VD-292`, cookie jar nội bộ vẫn xử lý đủ toàn bộ `Set-Cookie` thô) |
| `std.sqlite` | `open open_with close execute execute_batch query query_one explain tables table_exists index_exists columns indexes index_columns primary_keys schema_sql data_version schema_version user_version application_id journal_mode database_list last_insert_rowid changes begin commit rollback in_transaction` |

Method sugar (`s.f(args)` ≡ `str.f(s, args)` cho hàm nhận `str` đầu) có cho `str`, `list`, `map`;
`std.*` chỉ nhận đối số **vị trí**, không nhận `--named`.

---

## 9. CLI `vis`

### 9.1 Tổng quan

```text
vis --version | vis -V
vis explain [--json] [--lang vi|en|ja] <VSxxxx> | vis explain --list [--json]
vis run [cờ] <tệp.vis> [-- args…]
vis check [cờ] <tệp…> | vis check --stdin
vis test [cờ] [thư mục|tệp]
vis fmt ... | vis lsp --stdio | vis pkg ... | vis cache ... | vis plugin ...
vis new <dir> [--template bin|lib] | vis doc <path> [--format md|json] | vis repl
vis doctor [--json] | vis version [--json]
```

### 9.2 `vis run`

Cờ chính: `--tap`, `--vm`, `--workers N`, `--stdin`, `--time`, `--max-budget N`,
`--unlimited-budget`, `--dump-bytecode`, `--dump-deps`, `--dry`, `--sandbox`, `--sandbox-check`,
`--strict-caps`, `--no-strict-caps`, `--grant tên=path`, `--no-cache`, `--sqlite-debug`.

- `--stdin`: đọc UTF-8 từ stdin như tệp ảo `-`; không đọc/ghi cache; không đi với `--dump-*`/`--dry`.
- `--time`: một dòng `# time: <ms> ms` (hoặc `us`) ra stderr, chỉ khi runtime đã chạy.
- `--tap`: `TAP version 13`, `1..1`, `ok|not ok 1 - <tên> (steps=…)`.
- `--workers`: xem §5.4; `--max-budget`: xem §5.3.
- `--dry`: in cap rồi thoát 0; `--sandbox-check`: in 3 dòng chẩn đoán, không chạy.
- Cache bật mặc định cho đường VM; `--no-cache` tắt; bytecode luôn qua `load_verified`.
- `main(args: list[str])` nhận các đối số sau `--` (xem §12 cho giới hạn kiểm tra chữ ký).

### 9.3 `vis check`

- `--syntax`: chỉ lex/parse/hạ cấu trúc, bỏ kiểm kiểu; `--no-extern`: tắt chương trình ngoài;
  `--json`: một object JSON nén; `--stdin`: đọc stdin như `-`.
- Exit: `0` sạch, `1` có chẩn đoán, `2` dùng sai/không đọc được/không phải UTF-8.

### 9.4 `vis test`

- Chạy `test fn` trong tệp/thư mục và fixture `<tên>.vis|.vish` + `.out`/`.err`.
- Fixture không có `.err`: exit 0 và stdout byte-khớp `.out` (bỏ dòng SPDX đầu của `.out`).
- Có `.err`: phải đỏ đúng mã; `VS05xx` ⇒ exit `101`, còn lại `2`.
- Cờ: `--vm`, `--strict-caps`, `--timeout <ms>` (≥50), `--json`, `--json-out <path>`,
  `--filter <substr>`, `--fail-fast`, `--list`, `--repeat N`, `--shuffle [--seed u64]`,
  `--shard I/N`, `--jobs N`, `--slowest N`.
- TAP: `TAP version 13`, `1..N`, `ok|not ok <i> - <tên>`; exit `0`/`1`/`2`.
- Mặc định tìm `test/sem` từ cwd leo lên; truyền đường dẫn để đổi.

### 9.5 `vis fmt`

- Đọc CST, chuẩn hoá: NFC, CRLF→LF, tab→4 dấu cách, bỏ khoảng trắng cuối dòng, đúng một newline
  cuối; **không** đổi token/chú thích; `.vish` chỉ chuẩn hoá an toàn (không tự đổi độ sâu khối).
- `vis fmt <tệp|thư-mục>` in ra stdout; `--write` ghi đè; `--check` exit 0/1/2;
  `--json`, `--list`, `--diff`, `--stat`; `--stdin [--check] [--mode script|command]`;
  `--indent-report`.
- Tệp lỗi cú pháp: không sửa, exit 2.

### 9.6 `vis pkg`

```text
vis pkg add <tên>[@<req>] [--entry <path>] [--registry <dir|http://…|https://…>] [--registry-key <64-hex>]…
vis pkg install | update [<tên>] [--registry …] [--registry-key …]…
vis pkg list [--json] | remove <tên> | gc [--dry-run] [--lock <path>]… [--root <dir>]…
vis pkg migrate
```

Xem §8.2. Mọi lỗi gói là `vis pkg: …` + exit 2 (trừ lỗi ngôn ngữ `VS07xx` khi nạp).

### 9.7 `vis cache`

- `vis cache --path` in thư mục cache (`$XDG_CACHE_HOME/vifast/vis` hoặc `~/.cache/vifast/vis`).
- `vis cache --clear` xoá tệp cache; in số tệp đã xoá ra stderr.
- Cache khoá theo **hash closure**: nội dung tệp chính + phụ thuộc + phiên bản compiler + VBC1 +
  bảng thẻ + chế độ; bytecode hỏng/lệch được biên dịch lại, không bao giờ chạy chưa xác minh.

### 9.8 `vis plugin`

- `vis plugin check <plugin.wasm> [--cap-dir <path>[=ro|rw]]… [--strict-caps] [--sandbox]`.
- `vis plugin run <plugin.wasm> <hàm> [giá trị…] [--cap-dir …]…`.
- Giá trị: `int:5`, `str:abc`, `bool:true`, `float:1.5`, `bytes:<hex>`, `unit`; không tiền tố ⇒ `str`.
- `--cap-dir` trao `cap[Dir]` tại guest `/cap0`, `/cap1`, …; mặc định `ro`.
- `--strict-caps`/`--sandbox` tắt cứng plugin động (`VS0604`) trước khi đọc tệp.
- Exit 0 sạch; 2 lỗi dùng/quyền/ABI.

### 9.9 `vis new`, `vis doc`, `vis repl`

- `vis new <dir> [--template bin|lib]`: tạo dự án chạy được (`main.vis`, `test/sem/smoke.vis`,
  README); từ chối thư mục đã có; exit 2 khi lỗi.
- `vis doc <path> [--format md|json]`: trích `##` gắn với item cấp tệp bằng CST; JSON gồm
  `path` + `items[{line, signature, doc[]}]`; lỗi parse ⇒ exit 2.
- `vis repl`: đọc từng dòng; chỉ commit khi nạp + kiểm kiểu + chạy sạch; lệnh `:help`, `:type <expr>`,
  `:reset`, `:quit`; lỗi không làm mất trạng thái trước.

### 9.10 `vis lsp`, `vis doctor`, `vis version`, `vis explain`

- `vis lsp --stdio`: JSON-RPC, incremental sync UTF-16; completion/hover/definition/references/
  rename/signatureHelp/foldingRange/workspace symbol/highlight/selectionRange/formatting/
  typeDefinition/implementation/semanticTokens (theo README).
- `vis doctor [--json]`: 5 dòng tất định `vis/os/arch/sqlite/features` hoặc object JSON 5 khoá
  `version/os/arch/sqlite/features`; không đọc môi trường/tệp/mạng.
- `vis version [--json]`: `vis 0.6.0` hoặc `{"name":"vis","version":"0.6.0"}`.
- `vis explain`: xem §7.1.

---

## 10. ABI plugin

### 10.1 Plugin tĩnh — `VIS_PLUGIN_ABI_V1`

Crate `vis-plugin-abi` + `vis-plugin-macros` (ST-306) cung cấp ABI Rust **ổn định** cho plugin
liên kết tĩnh:

- `PluginValue`: `Unit | Bool | Int | Float | Str | Bytes | List | Map | Option | Result | Cap`.
  Chuyển đổi tường minh với `Value`; hàm/task/kênh không qua biên.
- `PluginCtx`: **chỉ** chứa các `Cap` host truyền vào; `read_text`/`write_text` kiểm canonical
  path nằm trong root và cờ `write`; không đọc env, không tự mở tệp ngoài cap.
- `Error`: `code` + `message`; thiếu quyền `VS0604`, lỗi I/O/kiểu `VS0501`; `vis::Error` là tên
  phía Rust.
- Macro `#[vis::module(name = "…")]` + `#[vis::func]` sinh `__vis_module() -> PluginModule`,
  chữ ký ViScript và wrapper chuyển kiểu lúc build; tham số đầu `ctx: &PluginCtx` không lộ vào
  chữ ký ViScript.
- Crate `#![forbid(unsafe_code)]`. Bản `vis` phát hành **không** kèm plugin tĩnh nào; tích hợp
  cần người dùng tự build (xem §12).

### 10.2 Plugin động — WASI/WASM

- Plugin là **WASI command** (`wasm32-wasip1`, export `_start`) và export marker
  `vis_plugin_abi_v1`; host kiểm marker trên byte wasm **trước khi chạy**, rồi gọi chế độ `abi`
  và đối chiếu `VIS_PLUGIN_ABI_V1=1`. Lệch/không phải plugin ⇒ `VS0501`.
- Giao thức stdout: chế độ `abi` → `VIS_PLUGIN_ABI_V1=1`; chế độ `manifest` → các dòng
  `module=<tên>`, `abi=<số>`, `func=<tên>;<p1,p2>;<ret>`, `cap=<Kind>;ro|rw`, kết thúc `ok`;
  chế độ `call` → `ok` + giá trị mã hoá hoặc `err` + mã + hex thông điệp.
- Codec văn bản tự tả: `u:`, `b:1:`, `i:5:`, `f:1.5:`, `s:<n>:<hex>`, `y:<n>:<hex>`,
  `l:<n>:` + phần tử, `m:<n>:` + cặp, `o:1:`/`o:0:`, `r:1:`/`r:0:<n>:<hex>`,
  `c:<klen>:<hex>:<rlen>:<hex>:<0|1>:`.
- Sandbox: chỉ preopen đúng `cap[Dir]` được trao tại `/cap0`, `/cap1`, …; không quyền ngầm;
  đường ngoài preopen bị WASI từ chối (`VS0604`). Không cấp net/proc/sqlite.
- Giới hạn: wasm ≤ 64 MiB; một lượt chạy mặc định 20 s; runtime dò `wasmtime` CLI → Python
  `wasmtime` → Node `node:wasi`, ghim bằng `VIS_WASI_RUNTIME`.
- Tắt cứng khi `--strict-caps`/`--sandbox`; không có đường vòng.

---

## 11. Nền tảng và kiểm chứng

### 11.1 Bảng nền tảng

Xem §6.4. Tóm tắt: Linux chạy thật + Landlock (`vis-host-linux` trong workspace); Windows Tier 1
(path-only guard); macOS arm64 Tier 1 (không Landlock); WASI preopen (runtime có phạm vi,
`vis-wasm-sem` 390/390 với stack 12 MiB + Node 4096 KiB); `wasm32-unknown-unknown` chỉ check.

### 11.2 Bộ kiểm trong repo

| Bộ | Nội dung |
|---|---|
| `test/sem/**` | ca ngữ nghĩa `.vis`/`.vish` + `.out`/`.err`, chạy cả interp và VM |
| `test/conformance/**` & `crates/vis-conformance` | corpus kiểm chuẩn dữ liệu `.vis` (+ `.out`/`.out_prefix`/`.err`/`.trap`/`.exit`/`.fmt`/`.skip-windows`), chạy thẳng API thư viện cả interp và VM (563 ca native; `vis-wasm-conformance` 393×2 WASI) |
| `test/expr`, `crates/vis-syntax/tests` | golden từ vựng/parser/biểu thức; NFC, cột scalar, lỗi đầu |
| `test/doc-examples` | mọi khối `vis`/`vish-session` trong `docs/*.md` phải parse/check được (trừ allowlist) |
| `test/cmd`, `test/modules`, `test/pkg`, `test/plugin-wasi` | chế độ lệnh, module, gói, plugin WASI |
| `crates/*/tests`, `vis-host-linux/tests` | unit/integration từng crate, host conformance, LSP, security, Landlock |
| `./vis-dev gate --fast` | fmt + clippy + luật R1..R9 + DAG 5 tầng + self-test + build + doc-examples + L0 |
| `./vis-dev nightly` | gate + fuzz parser + vòng tròn fmt |

### 11.3 Cách kiểm chứng bản đặc tả này

- Từ vựng đối chiếu `crates/vis-syntax/src/lex/token.rs` (`Kind::ALL` 25, `KEYWORDS` 40,
  `PUNCTS` 39) và `docs/SPEC-lex.md`.
- Cú pháp đối chiếu `crates/vis-syntax/src/parse/{mod,expr,khai_bao,cmd}.rs`; mọi EBNF ở §3 có
  luật sinh tương ứng trong parser (trừ các mục ghi “cú pháp nhưng tyck từ chối”).
- Ngữ nghĩa đối chiếu `crates/vis-hir/src/**` (HIR/Type/`StdId`+`FUNCS`), `crates/vis-types/src/tyck/**`,
  `crates/vis-core/src/**`, `crates/vis-http/src/**`, `crates/vis-runtime/src/**`,
  `crates/vis-interp/src/**`, `crates/vis-bytecode/src/**`, `crates/vis-vm/src/**`,
  `test/sem/**` và `test/conformance/**`.
- `./vis-dev doc-examples` chạy trên toàn bộ khối `vis`/`vish-session` của `docs/` (kết quả ghi ở
  journal ST-309); các ví dụ trong tệp này cũng được `vis check` trong cổng đó.
- `python3 test/report/check_report.py --all` kiểm báo cáo vòng (không sửa báo cáo).
- `git diff --check` kiểm khoảng trắng cuối dòng.

### 11.4 Giới hạn của bản đặc tả

Bản này **không** thay thế:

- `docs/SPEC-lex.md` — chuẩn từ vựng chi tiết (bảng token, 60 golden, mã `VS01xx`).
- `agent/API-STD.txt` — chữ ký std đóng băng (167 hàm tại mốc này); §8.3 chỉ liệt kê tên nhóm.
- `agent/API-VBC1.txt` — bảng thẻ bytecode VBC1; định dạng nhị phân không thuộc đặc tả ngôn ngữ.
- `agent/DECISIONS.md` — lịch sử quyết định (`VD-01..VD-292`, `R1..R37`) và các nợ đã biết.

---

## 12. Phần chưa đặc tả đầy đủ / chỉ có thiết kế

Mục này **không** mô tả tính năng như đã có. Đây là các khoảng trống đã kiểm tra tại `c4337ce` (đồng bộ thêm ở `VD-291`/`VD-292`):

| Hạng mục | Trạng thái thực tế | Ghi chú |
|---|---|---|
| `ref[T]`, `weak[T]` | Parser nhận cú pháp; tyck báo `VS0303` “chưa hỗ trợ” | `ref(expr)` được chấp nhận và **hạ như chính expr**; không có tham chiếu/ô nhớ |
| `set[T]`, `set{…}` | `set{…}` là **list** đã khử trùng; không có kiểu `set` | `let s: set[int]` ⇒ `VS0303` |
| `stream[T]`, `Future[T]`, `Iterator[T]`, `.lazy()` | Không có; `VS0303` | `for` trên range **được tối ưu lười**, nhưng không có kiểu iterator |
| `dyn Trait`, trait object | Không có; `dyn` là từ khoá nhưng parser báo `VS0201` | Không có `impl Trait` động |
| `datetime`, `instant` | Không có trong `Type::builtin`; `VS0303` | `std.time` chỉ có `now_ms`/`monotonic_ms` |
| `scope` structured concurrency | `scope` chỉ là khối có nhãn; task không bị huỷ theo scope | Có timeout kênh + `select_recv*`/`select_send*`; không có `select` tổng quát/cancel cây |
| `chan(0)` rendezvous | Đường thường bị từ chối `VS0505`; đường frame `async` hỗ trợ rendezvous | Kênh đường đồng bộ phải có dung lượng ≥ 1 |
| `async`/`Future` lười | Không có `Future`; gọi `async fn` trả `Task[T]` ngay | Task chạy theo scheduler một luồng |
| `spawn` biểu thức phức tạp | Chỉ nhận lời gọi hàm/phương thức `async` | `spawn f` không gọi là lỗi cú pháp/kiểu |
| `async` trait method | Bị từ chối `VS0415` | Chỉ `fn` cấp tệp/`impl`/`cmd` |
| `int(x)`, `float(x)`, `path(x)` | Không có; `VS0501` “chưa hỗ trợ” | Chỉ `str(x)`; dùng `str.parse_int`/`str.parse_float` |
| `std.text`, `std.coll`, `std.stream`, `std.proc`, `std.fmt`, `std.crypto`, `std.term`, `std.testing`, `std.sync`, `std.shell` | Không có; `VS0705` | `test fn` là cú pháp lõi, không phải module |
| `std.fs`/`std.path` nhận `path` | Chữ ký hiện nhận `str` | `path` chủ yếu là literal/kiểu so sánh |
| `vis.toml`, MVS, `sum.vifast.org`, proxy | Chưa có | `vis pkg` dùng `vis.lock` + registry hiện tại (§8.2) |
| `cmd` tự sinh `--help` | Chưa có | `cmd` vẫn là hàm + cờ; không có help tự động |
| `from json`, `save`, `walk` trong pipeline | Chưa có như lệnh shell | Chỉ `to json` là sugar; còn lại gọi `std.*` |
| Enum pattern có tiền tố `Shape.Circle` | Parser không nhận (`VS0201`) | Dùng tên trần `Circle(r)` |
| `if let v = x` irrefutable | Được chấp nhận (không báo lỗi) | Khác thiết kế “phải là pattern” |
| `let xs = []`/`let m = {}` | Được chấp nhận với `{error}` | Không bắt buộc chú thích kiểu như thiết kế |
| `main` chữ ký | Không kiểm chặt: 0 tham số, hoặc 1 tham số nhận `list[str]`; >1 tham số thiếu đối số runtime | Không dùng làm đặc tả ổn định |
| `vis fmt` | Chỉ chuẩn hoá thụt lề/NFC/newline, không định dạng lại biểu thức | `.vish` không tự đổi độ sâu khối |
| `vis plugin` liên kết tĩnh | ABI/macro có, nhưng bản `vis` phát hành không đăng ký plugin tĩnh | Cần bản build tuỳ biến |
| `vis lsp` chi tiết giao thức | Chỉ mô tả tính năng ở mức CLI; không có đặc tả JSON-RPC đầy đủ | Cần đặc tả riêng nếu làm client khác |
| `Error.span` | Kiểu `str?` và **không ổn định** (VD-231) | Có thể đổi sang kiểu có cấu trúc |
| `size`/`duration`/`percent` biên | Có giới hạn biểu diễn/rounding (R10); các ca cực trị cần test riêng | Không mô tả hết mọi ca tràn/round |

Các mục trên **không** được trích dẫn như hành vi chuẩn tắc. Nếu cần dùng, phải kiểm lại mã/test
tại HEAD và mở quyết định mới (VD-08).

---

## Phụ lục A — Bảng ưu tiên và tương thích cú pháp

Bảng ưu tiên ở §4.6 là bản chuẩn. Các ghi chú tương thích:

- `else` của `if` biểu thức là **bắt buộc**; `else` của `if` câu lệnh là tuỳ chọn.
- `elif` lặp được; `else` chỉ một.
- `match` dạng biểu thức dùng cùng cú pháp thân; trong ngoặc, ranh giới nhánh theo dòng vật lý
  và cột (VD-266).
- `|>` nối dòng ở độ sâu 0; dòng bắt đầu `|>` không sinh NEWLINE.
- `?`/`??` trong kiểu là hậu tố lặp; `T???` = 3 lớp.
- `?` trong biểu thức là postfix try; `?.` là optional field/method; `??` là coalesce.
- `..`/`..=` không kết hợp; `**` kết hợp phải; so sánh không kết hợp.
- `with` bám chặt hơn `|>`; `u with age: 21 |> f` = `(u with age: 21) |> f`.

## Phụ lục B — Ví dụ tổng hợp

```vis
# Chương trình: đếm tần suất từ, in ra màn hình.
use std.str

fn main():
    let cau = "viết một lần, chạy nhiều nơi"
    let tu = cau.split(" ")
    let dem = map.keys({})
    print("có {tu.len()} từ")
```

```vis
type User derives Eq, Show, Json:
    name: str
    age: int?
    email: str? = None

enum Shape:
    Circle(r: float)
    Rect(w: float, h: float)
    Empty

fn area(s: Shape) -> float:
    match s:
        Circle(r): 3.14 * r * r
        Rect(w, h): w * h
        Empty: 0.0

let u: User = User(name: "An", age: None, email: None)
print(u)
print(area(Circle(2.0)))
```

```vis
fn first[T](xs: list[T]) -> T?:
    if xs.is_empty(): None else: Some(xs[0])

fn main():
    let a = first([10, 20]) ?? 0
    let b = first([])
    if let Some(v) = b:
        print(v)
    else:
        print("rỗng")
    print(a)
```

```vis
trait Chào:
    fn chao(self) -> str

type Người derives Show:
    ten: str
    tuoi: int

impl Chào for Người:
    fn chao(self) -> str:
        "Xin chào {self.ten}"

fn main():
    let n: Người = Người(ten: "An", tuoi: 20)
    print(n.chao())
```

```vis
async fn cong(a: int, b: int) -> int:
    a + b

async fn gui(c: Chan[int], n: int) -> ():
    let _ = c.send(n)

fn main():
    let t = cong(1, 2)
    print(await t)
    let c = chan[int](1)
    let w = spawn gui(c, 7)
    let _ = await w
    match c.recv():
        Some(v): print(v)
        None: print("hết")
```

```vis
fn main(data: cap[Dir] @ p"./data", out: cap[Dir] @ p"./out" (write)) -> Result[(), Error]:
    Ok(())
```

```vis
cmd mot_dong(p: str) -> list[str]:
    cmd { echo p | lines | take 1 }

fn main():
    print(mot_dong("xin chào"))
```

## Phụ lục C — Bảng đối chiếu nhanh

| Tính năng | Cú pháp | Test tiêu biểu |
|---|---|---|
| `let`/`var` | `let x = 1`, `var y = 2` | `test/sem/pham-vi` |
| Hàm/closure | `fn f(...)`, `x => x + 1` | `test/sem/ham`, `test/sem/so-hoc` |
| Optional | `T?`, `T??`, `??`, `?.`, `?` | `test/sem/optional`, `test/sem/result` |
| Record/enum | `type U: …`, `enum E: …` | `test/sem/enum`, `test/sem/long-du-lieu` |
| Match/pattern | `match`, `\|`, `..`, `[a, ..r]` | `test/sem/match-nang-cao` |
| Generic/trait | `fn f[T]`, `impl T for X` | `test/sem/generic`, `test/sem/trait` |
| Async/kênh | `async fn`, `spawn`, `await`, `chan[T]` | `test/sem/async`, `test/sem/kenh` |
| Quyền | `cap[Dir]`, `@`, `(write)` | `test/sem/cap` |
| Chế độ lệnh | pipeline, `cmd { … }` | `test/sem/vish`, `test/cmd` |
| Gói/module | `use ./x`, `use pkg.x`, `vis pkg` | `test/modules`, `test/pkg` |
| Plugin | ABI/WASI | `crates/vis-plugin-abi`, `test/plugin-wasi` |
| Host/nền | fs/net/proc, Landlock/FsGuard | `test/host`, `vis-host-linux`, `vis-host-wasm` |

---

*Hết `docs/SPEC.md` — ST-309.*
