# Option lồng — bảng quy tắc pin (trước khi triển khai) — 2026-11-19

> **Phạm vi**: thực hiện quyết định A0 2026-11-19 (`agent/DECISIONS.md` — bản ghi chưa gán số VD "Quyết định Agent 0 · 2026-11-19 — đợt đánh giá công thái học").
> **Đồng bộ 2026-11-19 (sau lệnh A0 Windows-349af67)**: mục Q-OPTION-1/2/3 đã được đồng bộ theo quyết định Option-lồng đã nhận (xem §4); không còn "chờ A0".
> **ĐÍNH CHÍNH 2026-11-19 (lệnh A0 tiếp theo, cùng ngày)**: cách diễn giải trước của Agent 1 ("thiếu lớp ngoài/trong" ⇒ SAI KIỂU/VS0412) là **hiểu nhầm chú thích A0**. A0 làm rõ: ba trạng thái giá trị (`None` · `Some(None)` · `Some(Some(42))`) **đều là giá trị hợp lệ** khi kiểu đích rõ — "giá trị vắng mặt hợp lệ" khác "sai kiểu". Cơ bản: **không** VS0412-cho-f, **không** đổi `long-f.vis` thành `.err`. `let e: int?? = None` cũng hợp lệ khi kiểu đích đã xác định — nếu engine thực sự từ chối ⇒ **finding cần tái lập**, không giữ làm "khớp quyết định".
> **SPEC-lex.md KHÔNG bị sửa trong đợt này** (luật: SPEC chỉ qua A0). Doc tham chiếu được ghi dạng addendum (xem `docs/ViScript-thiet-ke.md` note).
> **Không có VD nào phủ `T??` trước đợt này** — `agent/DUNG-CHO-A0.md` dừng nhánh; A0 mở lại theo quyết định của bộ này.

## 1. Định nghĩa kiểu (đúng nguyên văn A0)

- `T?` ≡ **`Option[T]`** (enum std thật từ VD-208).
- `T??` ≡ **`Option[Option[T]]`**, **không làm phẳng**, không "bỏ lớp trong", không "gộp lớp".
- `Some(x)` luôn thêm **đúng một** lớp (`Some : U -> Option[U]`).
- Pattern khớp sát **theo cấu trúc giá trị thật**: không tự bọc lớp, không tự bóc lớp. `Some(Some(v))` **không** khớp `Some(42)`; `Some(v)` **không** khớp `42` dù annot mong `int??`.
- Auto-lift **chỉ ở chuyển đổi annot/gán/đối số/return** (biên VD-227/228/229), **tối đa một lớp** mỗi lần chuyển đổi; **không** lặp nâng vượt nhiều lớp trong một lần; **không** nâng ngầm trong pattern.

## 2. Thứ tự kiểm tra chuyển đổi (tham chiếu VD-227/228/229)

Với expected `T_đích`, source `T_nguồn`:

1. `T_nguồn = T_đích` ⇒ giữ nguyên (lớp khớp nguyên văn).
2. `T_đích = Option[U]` và `T_nguồn = U` ⇒ nâng một lớp (`x -> Some(x)`) — bao gồm `T_nguồn = Option[U]` với `T_đích = Option[Option[U]]` (nâng một lớp ở lớp **ngoài**).
3. Không lặp (không một lần chuyển đổi vượt > 1 lớp).
4. Không nâng trong pattern; không nâng theo annot lambda params.

## 3. Bảng ca (theo nguyên văn A0, verdict do Agent 1 pin — xem §4 câu hỏi)

| Ca | Nguồn | Expected | Verdict A0 | Pin hiện tại |
|----|-------|----------|-----------|--------------|
| a | `42` (int) | `int?` | ✔ hợp lệ (nâng một lớp) | ✔ lift một lớp (`14678` ∘ rule 2) |
| b | `Some(Some(42))` | `int??` | ✔ hợp lệ (đúng kiểu) | ✔ rule 1 |
| c | `Some(42)` | `int??` | ✔ hợp lệ (thêm một lớp ngoài) | ✔ rule 2 (nguồn = `Option[int]` = đúng `U`, lift một lớp ở lớp ngoài) |
| d | `42` | `int??` | ✘ **lỗi: cần nâng hai lớp** | ✘ error `VS0412` (lift một lần không đủ; không lặp) |
| e | `None` | `int??` | **giá trị hợp lệ** — "Option ngoài là None" (A0 làm rõ 2026-11-19) | **valid** theo A0. **FIX (VD-241 §7, A0 2026-11-19):** `vis check` xanh; gate `None` ở `tyck/vd238.rs::la_none_dich_opt` chỉ yêu cầu lớp **ngoài cùng** là `Option` ⇒ `int?`/`int??`/`int???`/`Option[Option[int]]` đều hợp lệ; `list[int?]`/`map[str,int?]`/`Result[.., int?]` vẫn VS0412. Corpus: `long-reject-e` → **`long-none-e.vis`** xanh (+`none-three-states`, `long-none-param`, `long-none-return`). **CẬP NHẬT (đợt coercion Option, 2026-11-19, cùng ngày):** runtime cũng giữ **`None` lớp ngoài** (trước đó `let` lift bọc thành `Some(None)` — sai theo bảng giá trị A0); `long-none-e.out` cập nhật `sn`→`none-out` (xem addendum cuối tệp) |
| f | `Some(None)` | `int??` | **giá trị hợp lệ** — "Option ngoài là Some, Option trong là None" (A0 làm rõ 2026-11-19) | **valid** — engine chấp nhận đúng (`vis check` rc 0); corpus pin hợp lệ (`long-f.vis`, `int-qhap-gan.vis`). Không đổi `.err`, không VS0412 |

## 4. Trạng thái các mục treo (đính chính theo lệnh A0 2026-11-19 — không còn "chờ A0")

- **Q-OPTION-1 (e)** — `None` trần cho `int??`: **A0 làm rõ**: **hợp lệ** khi kiểu đích đã xác định ("Option ngoài là None") — "giá trị vắng mặt hợp lệ" ≠ "sai kiểu". **Agent 2 probe (2026-11-19 ~15:15)**: `vis check` rc=1 `VS0412 2:20` (VD-238) · interp rc=2 · VM rc=2 — **finding tái lập ổn định cả ba cửa**, chỉ ở khai báo `let` (arm match chưa tới được); VM/interp nhất quán; không crash. **FIX (VD-241 §7, A0 2026-11-19, nhánh `rev/vd241-option-none-fix-2026-11-19`)**: nguyên nhân = `tyck/vd238.rs::la_none_chep_duoc_opt_1` ép "một lớp" (`Opt(x) mà x không phải Opt`) ở mọi vị trí kiểm kiểu (`coerce_e` mod.rs:524, thân-hàm `kiem_than` mod.rs:943, Return-arm mod.rs:1133). Sửa thành `la_none_dich_opt`: chỉ phân biệt **lớp ngoài cùng** — `Type::Opt(_)` ⇒ hợp lệ; non-Option ngoài cùng (kể cả `list[int?]`/`map[str,int?]`/`Result[..,int?]`) ⇒ VS0412. **CẬP NHẬT (đợt coercion Option, 2026-11-19)**: Auto-lift **không giữ như cũ** nữa — `None` trần ở biên annot/tham số/return/`let`/`var`/gán lại **giữ nguyên** `None` lớp ngoài theo bảng giá trị A0 (`lift.rs::lift_opt` không bọc `None`; `$lift-opt` dùng chung hai engine); `Some(x)` nâng đúng **một lớp**. Cả **gán lại** `x = v` với `x` là `var` có annot `Option` giờ nâng một lớp (trước không nâng). VM `Unwrap` không trap arm không khớp (arm hỏng hình ⇒ `Unit`); arm test là hàm tổng như interp. Spelled-out/alias phân giải về `Type::Opt` ở lowering ⇒ ba spell tương đương. Xem addendum cuối tệp.
- **Q-OPTION-2 (f)** — `Some(None)` cho `int??`: **A0 làm rõ**: **hợp lệ** ("Option ngoài là Some, Option trong là None"). Engine chấp nhận đúng. **Không** VS0412-cho-f; **không** đổi `long-f.vis` thành `.err`. `int-qhap-gan.vis` rà theo quy tắc cấu trúc, không đổi hàng loạt.
- **Q-OPTION-3** — `T??` áp cho mọi vị trí annot/kiểu generic: `T??` = `Option[Option[T]]` thật, áp mọi vị trí annot bên trong kiểu (annot biến, tham số, trả về, annot generic `list[int??]`, `map[str, int??]`, annot closure return); auto-lift chỉ ở biên chuyển đổi annot/đối số/return như §1–§2; không suy rộng luật khác khi chưa có nguyên văn. Ba trạng thái giá trị phải phân biệt được: `None` (ngoài None), `Some(None)` (ngoài Some trong None), `Some(Some(42))` (cả hai giá trị).

## 5. Ca kiểm chứng bắt buộc (làm trước khi merge)

`test/sem/optional/long-*.vis` (+`.out` hoặc `.err`), tối thiểu:

1. `long-exact-b.vis` — `let b: int?? = Some(Some(42)); match b: Some(Some(v)): print(v); _: print("x")` ⇒ stdout `42`.
2. `long-wrap-c.vis` — `let c: int?? = Some(42)`; `match c: Some(Some(v)): print(v) / Some(v): print("one {v}") / None: print("none")` ⇒ `Some(Some(7))`-block; **interp và VM phải cùng in một dòng**, không "Some khac" lệch nhánh.
3. `long-reject-d.vis` — `let d: int?? = 42` ⇒ **check đỏ** (`VS0412`, lỗi biên dịch), exit 2.
4. `long-none-e.vis` (đổi tên từ `long-reject-e.vis`; `.err` xoá, thêm `.out`) — `let e: int?? = None` ⇒ **check xanh** (VD-241 §7); corpus in **`none-out`** (biên `let` giữ nguyên `None` theo bảng giá trị A0 — CẬP NHẬT 2026-11-19, đợt coercion Option; trước đó in `sn`).
5. `long-f.vis` — `let f: int?? = Some(None)` — **hợp lệ** theo A0 (2026-11-19); engine chấp nhận; corpus giữ nguyên.
6. `long-param-b.vis` — truyền `Some(42)` vào `fn g(x: int??) -> int: …` (đổi lift 1 lớp ở đối số); hai engine in cùng.
7. `long-return.vis` — `fn g() -> int??: Some(42)` — return lift một lớp.
8. `nested-match-vm.vis` — pattern `Some(Str(t))` với `std.json` decode — **phải khớp arm đúng** hai engine (pattern VM đã sửa).
9. `json-encode-owned.vis` — `json.encode(Arr([]))` do **code dựng** — in `[]` (VM lẫn interp).

## 6. Backcompat / khả năng phá vỡ

- Corpus `test/sem` **không có** ca `int??` ⇒ không golden đổi vì thêm ca.
- Lift chỉ ở biên VD-227/228/229 ⇒ `Optional` hiện hữu không đổi nghĩa; `T??` annot hiện đã parse và được checker chấp nhận-phẳng ⇒ **đổi hành vi của `T??` annot từ phẳng thành lồng thật**, đúng theo A0.
- `Value`/ABI bytecode/format không đổi (Option biểu diễn qua `Value::Enum(Some|None, [..])` đã nested-ready từ trước — xác minh khi sửa VM).
- Hai engine phải **byte-identical** sau khi sửa (không chỉ stdout — cả stderr phù hợp + exit code + dữ liệu output).


## 7. Addendum 2026-11-19 (lệnh A0 cùng ngày — đợt coercion Option, nhánh `rev/vd241-option-coercion-2026-11-19`)

- **Bảng giá trị runtime giờ khớp A0 ở mọi biên**: `None`⇒`None` · `Some(None)`⇒`Some(None)` · `Some(Some(42))`⇒giữ nguyên · `Some(42)`⇒**nâng một lớp** · `42`⇒lỗi `VS0412` (không nâng hai lớp; kể cả spelled-out/alias — `vd_205::vd241_forty_two_opt2_la_loi`).
- **Ba spell tương đương**: spelled-out `Option[Option[int]]` và alias `type Y = …` phân giải về `Type::Opt` **ở lowering** (`lower::named_type`) — sugar/spelled/alias cùng HIR ⇒ cùng coercion, cùng runtime, cùng kết quả `match`.
- **Biên gán lại nâng một lớp** khi đích là `var` có annot `Option` (cả hai engine); `x = None` giữ `None` (bước 1 bảng giá trị A0); gán vào `let` vẫn bất biến (tyck).
- **Expected output đổi**: `long-none-e` sn→**none-out** · `long-none-param` 1000→**9999** · `long-none-return` sn→**none-out** · `t-qhap-generic` dòng 2 → **None**; `none-three-states` giữ nguyên. **Corpus 487** (+`optional/long-option-matrix.vis`).
- **VM `Unwrap`** trong `pattern::test` không trap arm không khớp (đẩy `Unit`; test con cho `false`) — như interp; không mở opcode/ABI.
- **Chưa** nâng ở đích trường/chỉ số (`x.f = v`, `x[i] = v`) và `var` không annot — chờ A0 chốt (addendum báo cáo §8.6/§10).
- Lịch sử phía trên giữ nguyên; đính chính áp cho trạng thái hiện hành.
