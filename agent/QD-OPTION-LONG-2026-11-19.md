# Option lồng — bảng quy tắc pin (trước khi triển khai) — 2026-11-19

> **Phạm vi**: thực hiện quyết định A0 2026-11-19 (`agent/DECISIONS.md` — bản ghi chưa gán số VD "Quyết định Agent 0 · 2026-11-19 — đợt đánh giá công thái học").
> **Đồng bộ 2026-11-19 (sau lệnh A0 Windows-349af67)**: mục Q-OPTION-1/2/3 đã được đồng bộ theo quyết định Option-lồng đã nhận (xem §4); không còn "chờ A0".
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
| e | `None` | `int??` | **verdict nguyên văn A0**: ✘ | ✘ **error** — checker đang chặn (`VS0412 2:20`, VD-238) ✓ khớp nguyên văn |
| f | `Some(None)` | `int??` | **verdict nguyên văn A0**: ✘ — "có lớp ngoài, thiếu lớp trong" | ✘ **error** (theo nguyên văn A0). **Hiện trạng engine chấp nhận** (`vis check` rc 0; corpus pin hợp lệ: `test/sem/optional/long-f.vis`, `int-qhap-gan.vis`) ⇒ **lệch so với nguyên văn A0** — chưa sửa mã; xem DEM + DECISIONS block A0 2026-11-19 (Windows Tier 2) §7. |

## 4. Trạng thái các mục treo (đồng bộ với quyết định A0 2026-11-19 — không còn ghi "chờ A0")

- **Q-OPTION-1 (e)** — `None` trần cho `T??`: **ĐÃ CÓ QUYẾT ĐỊNH** (nguyên văn A0 2026-11-19): **error** ("thiếu lớp ngoài"). Engine hiện chặn đúng (`VS0412`, VD-238) ⇒ **không cần sửa**.
- **Q-OPTION-2 (f)** — `Some(None)` cho `int??`: **ĐÃ CÓ QUYẾT ĐỊNH** (nguyên văn A0 2026-11-19): **error** — "có lớp ngoài, thiếu lớp trong". **Lệch hiện trạng**: engine chấp nhận (`vis check` rc 0; corpus `long-f.vis` + `int-qhap-gan.vis` pin hợp lệ). **Chưa sửa mã trong đợt này** (không tự đổi ngữ nghĩa); đề nghị đợt riêng: VS0412 cho ca f + corpus cập nhật theo (`long-f.vis` → `.err`, `int-qhap-gan.vis` rà lại). Chờ lệnh của A0 để mở VD/đợt này.
- **Q-OPTION-3** — `T??` áp cho mọi vị trí annot/kiểu generic: **không có nguyên văn riêng** của A0 cho kênh này; giữ kết cấu: `T??` = `Option[Option[T]]` thật, áp **mọi vị trí annot bên trong kiểu** (annot biến, tham số, trả về, annot generic `list[int??]`, `map[str, int??]`, annot closure return); auto-lift chỉ ở biên chuyển đổi annot/đối số/return như §1–§2. Không suy rộng luật nào khác khi chưa có nguyên văn.

## 5. Ca kiểm chứng bắt buộc (làm trước khi merge)

`test/sem/optional/long-*.vis` (+`.out` hoặc `.err`), tối thiểu:

1. `long-exact-b.vis` — `let b: int?? = Some(Some(42)); match b: Some(Some(v)): print(v); _: print("x")` ⇒ stdout `42`.
2. `long-wrap-c.vis` — `let c: int?? = Some(42)`; `match c: Some(Some(v)): print(v) / Some(v): print("one {v}") / None: print("none")` ⇒ `Some(Some(7))`-block; **interp và VM phải cùng in một dòng**, không "Some khac" lệch nhánh.
3. `long-reject-d.vis` — `let d: int?? = 42` ⇒ **check đỏ** (`VS0412`, lỗi biên dịch), exit 2.
4. `long-reject-e.vis` — `let e: int?? = None` ⇒ **check đỏ** (VD-238 ngoài bất biến).
5. `long-f.vis` — `let f: int?? = Some(None)` — verdict theo §4 (nguyên văn A0: **✘ error**, hiện trạng engine chấp nhận, corpus pin hợp lệ ⇒ **lệch A0**, đề nghị đợt riêng).
6. `long-param-b.vis` — truyền `Some(42)` vào `fn g(x: int??) -> int: …` (đổi lift 1 lớp ở đối số); hai engine in cùng.
7. `long-return.vis` — `fn g() -> int??: Some(42)` — return lift một lớp.
8. `nested-match-vm.vis` — pattern `Some(Str(t))` với `std.json` decode — **phải khớp arm đúng** hai engine (pattern VM đã sửa).
9. `json-encode-owned.vis` — `json.encode(Arr([]))` do **code dựng** — in `[]` (VM lẫn interp).

## 6. Backcompat / khả năng phá vỡ

- Corpus `test/sem` **không có** ca `int??` ⇒ không golden đổi vì thêm ca.
- Lift chỉ ở biên VD-227/228/229 ⇒ `Optional` hiện hữu không đổi nghĩa; `T??` annot hiện đã parse và được checker chấp nhận-phẳng ⇒ **đổi hành vi của `T??` annot từ phẳng thành lồng thật**, đúng theo A0.
- `Value`/ABI bytecode/format không đổi (Option biểu diễn qua `Value::Enum(Some|None, [..])` đã nested-ready từ trước — xác minh khi sửa VM).
- Hai engine phải **byte-identical** sau khi sửa (không chỉ stdout — cả stderr phù hợp + exit code + dữ liệu output).
