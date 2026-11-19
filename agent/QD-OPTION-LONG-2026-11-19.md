# Option lồng — bảng quy tắc pin (trước khi triển khai) — 2026-11-19

> **Phạm vi**: thực hiện quyết định A0 2026-11-19 (`agent/DECISIONS.md` — bản ghi chưa gán số VD "Quyết định Agent 0 · 2026-11-19 — đợt đánh giá công thái học").
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
| e | `None` | `int??` | **comment nguyên văn**: "thiếu lớp ngoài" | ✘ **error** — theo VD-238 mở rộng bất biến: `None` trần chỉ hợp lệ ở `T?`, không hợp lệ ở `T??` (khớp "thiếu lớp ngoài" + VD-238 nhất quán) |
| f | `Some(None)` | `int??` | **comment nguyên văn**: "có lớp ngoài, thiếu lớp trong" | ✔ **hợp lệ** — lớp ngoài explicit `Some`; lớp trong `None` là `Option[int]` khớp chính xác (rule 1). Xem Q-OPTION-2 §4 |

## 4. Câu hỏi còn treo cho A0 (không tự quyết)

- **Q-OPTION-1 (e)**: `None` trần cho `T??` — quyết định là **error** theo VD-238-mở-rộng (thiếu "lớp ngoài"); đây là lựa chọn bảo thủ nhất, khớp comment nguyên văn A0 nhất và khớp VD-238 (None trần chỉ ở `T?`). **Xác nhận?**
- **Q-OPTION-2 (f)**: `Some(None)` cho `int??` — Agent 1 pin **hợp lệ** (bọc outer explicit, inner `None` = `Option[int]` chính danh). Comment nguyên văn A0 "có lớp ngoài, thiếu lớp trong" có thể đọc ngược thành "thiếu lớp trong ⇒ error". **Xác nhận f hợp lệ, hay A0 muốn f error ("thiếu lớp trong")?**
- **Q-OPTION-3**: `T??` là `int → Option[Option[int]]` trong mọi vị trí annot/kiểu generic (suy luận `list[int??]`, `map[str, int??]`) — xác nhận không giới hạn khác.

## 5. Ca kiểm chứng bắt buộc (làm trước khi merge)

`test/sem/optional/long-*.vis` (+`.out` hoặc `.err`), tối thiểu:

1. `long-exact-b.vis` — `let b: int?? = Some(Some(42)); match b: Some(Some(v)): print(v); _: print("x")` ⇒ stdout `42`.
2. `long-wrap-c.vis` — `let c: int?? = Some(42)`; `match c: Some(Some(v)): print(v) / Some(v): print("one {v}") / None: print("none")` ⇒ `Some(Some(7))`-block; **interp và VM phải cùng in một dòng**, không "Some khac" lệch nhánh.
3. `long-reject-d.vis` — `let d: int?? = 42` ⇒ **check đỏ** (`VS0412`, lỗi biên dịch), exit 2.
4. `long-reject-e.vis` — `let e: int?? = None` ⇒ **check đỏ** (VD-238 ngoài bất biến).
5. `long-f.vis` — `let f: int?? = Some(None)` — verdict theo Q-OPTION-2; nếu hợp lệ: `match f: Some(Some(_)): print("s-s") / Some(None): print("s-None") / None: print("none")` ⇒ in `s-None`.
6. `long-param-b.vis` — truyền `Some(42)` vào `fn g(x: int??) -> int: …` (đổi lift 1 lớp ở đối số); hai engine in cùng.
7. `long-return.vis` — `fn g() -> int??: Some(42)` — return lift một lớp.
8. `nested-match-vm.vis` — pattern `Some(Str(t))` với `std.json` decode — **phải khớp arm đúng** hai engine (pattern VM đã sửa).
9. `json-encode-owned.vis` — `json.encode(Arr([]))` do **code dựng** — in `[]` (VM lẫn interp).

## 6. Backcompat / khả năng phá vỡ

- Corpus `test/sem` **không có** ca `int??` ⇒ không golden đổi vì thêm ca.
- Lift chỉ ở biên VD-227/228/229 ⇒ `Optional` hiện hữu không đổi nghĩa; `T??` annot hiện đã parse và được checker chấp nhận-phẳng ⇒ **đổi hành vi của `T??` annot từ phẳng thành lồng thật**, đúng theo A0.
- `Value`/ABI bytecode/format không đổi (Option biểu diễn qua `Value::Enum(Some|None, [..])` đã nested-ready từ trước — xác minh khi sửa VM).
- Hai engine phải **byte-identical** sau khi sửa (không chỉ stdout — cả stderr phù hợp + exit code + dữ liệu output).
