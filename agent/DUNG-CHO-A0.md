# DỪNG VÀ CHỜ A0 — T-DQ (2026-11-19 ~12:00) → **A0 ĐÃ QUYẾT 2026-11-19 (đợt ergonomics) — xem mục cuối**

> Ghi theo VD-241 §6. Không làm tiếp nhánh liên quan tới mục này. Cổng các việc khác A0 đã giao (đợt ergonomics thu thập) vẫn tiếp tục vì chỉ đọc — không đổi mã.
>
> **⇒ CẬP NHẬT 2026-11-19 (sau khi A0 đọc bộ báo cáo): các nhánh dưới đây được MỞ LẠI đúng theo quyết định A0 (DECISIONS.md, bản ghi cùng ngày "Quyết định Agent 0 · 2026-11-19 — đợt đánh giá công thái học").**
> **Đợt VD-241 §7 (2026-11-19, nhánh `rev/vd241-option-none-fix-2026-11-19`):** finding `let e: int?? = None` (VS0412, "VD-238 một lớp") đã được A0 chốt là **bug, sửa** — `None` hợp lệ khi lớp **ngoài cùng** là `Option` (`T?`/`T??`/…); `list[int?]`/`map[str,int?]`/`Result[..,int?]` vẫn VS0412. Thực thi + corpus mới (`long-none-e`, `none-three-states`, `long-none-param`, `long-none-return`), hai engine byte-identical; auto-lift **không đổi**. — xem `agent/QD-OPTION-LONG-2026-11-19.md` §3 hàng e.
> Tóm: Option lồng **lồng thật** (Option[Option[T]], pattern strict, lift 1 lớp, không pattern-lift); pattern ctor lồng VM + JSON encode được phép sửa (kiểm riêng, không mặc định cùng gốc); escape `\u{…}` sửa riêng; checker mở rộng có chọn lọc; `main -> Err` ⇒ exit 1; toolchain tái lập (không hardcode `~/.cargo/bin`, báo rõ compiler + yêu cầu phiên bản); `<fn>` chốt cả hai engine; channel giữ một giá trị; `pha1-rc1` bảo toàn; process API + stdlib chờ báo cáo.
> Các nội dung dưới đây giữ nguyên làm biên bản của giai đoạn dừng.

## Đã dừng: `T??` / optional lồng (`int??`) — hai engine lệch byte, bên đúng chưa xác định theo spec

- **HEAD ghi nhận**: `6fb72a7` (`main`, `2026-11-19`).
- **Hiện tượng** (`/tmp/opencode/ergo-c/BAO-CAO-ERGO-CASES.md`, chạy thật, cả hai engine, `vis check` exit 0 mọi biến thể):
  annot `let x: int?? = …` được `vis check` chấp nhận (exit 0); lúc chạy interp luôn khớp arm `Some(Some(v))`
  (kể cả khi giá trị chỉ có 1 tầng), VM khớp arm `Some(v)`; VM khi `let … = None` crash `VS0501` (exit 101);
  khi chỉ arm `Some(Some(v))` + arm `None`, VM **in 0 byte, exit 0** — kết quả sai im lặng; VM trap sớm hơn
  interp tại `match` qua biên hàm. `--dump-bytecode` (`c02e2.vis`): `JumpIfFalse(15)` trỏ vào chính `Unwrap(0)`
  ở đầu thân arm 1 — codegen optional lồng hỏng. Tái lập độc lập: `/tmp/opencode/ergo-c/verify-int-nested.vis`.
- **Vì sao không biết bên nào đúng**: thiết kế §4.4/EBNF (`docs/ViScript-thiet-ke.md:1358` và các dòng 220, 363, 498) nói `T??` là **lỗi kiểu**; còn §7.3 + VD-178 (dòng 1153, 1183) lại mô tả tự nâng nhiều lớp `T → T? → T??` ở biên gán/trả/đối số (R2 code chấp nhận `int??`); VD-227/VD-228 chỉ nói lift **1 tầng**. **Không có VD nào phủ `T??`** — VD-241 §3 cấm đổi ngữ nghĩa khi A0 chưa quyết.
- **Vai trò dừng**: theo VD-241 §6 ("Hai engine lệch byte mà không xác định được bên nào đúng theo spec"), mọi việc **sửa/định nghĩa lại `int??`** bị DỪNG cho tới khi Agent 0 quyết. Trong lúc chờ:
  - **Không** sửa `codegen` (`crates/vis-bytecode/src/codegen/*`) cho `Opt(Opt(_))`;
  - **Không** chặn annot `int??` ở checker (đó là đổi ngữ nghĩa);
  - **Không** thêm ca sem lock `T??` giả định theo một bên;
  - Tài liệu pha 2/ghi chú chỉ **mô tả hiện tượng** (xem `VISCRIPT-ERGONOMICS-DIAGNOSTICS.md` + `VISCRIPT-ERGONOMICS-FACTS.md` §Option lồng).

## Đã dừng: VM mis-compile pattern ctor lồng + Json do code dựng bị encode sai

- **HEAD ghi nhận**: `6fb72a7`; bằng chứng
  (`/tmp/opencode/ergo-b/BAO-CAO.md` §Bài 3; repro `t19.vis`, `t23.vis` — **tái lập độc lập**: interp in `ten = An`,
  VM in `khong phai str`; interp in `nested ctor = An`, VM in `Some khac`):
  - `match json.decode(…): Ok(v): match v: Obj(m): match m.get("name"): Some(Str(t)): …` — **VM trượt arm `Some(Str(t))`
    vào `Some(_)`/catch-all** (interp đúng). `Some(Some(5))` trên VM **không chạy nhánh nào**.
    `Some(EnumUser(…))` (nullary enum người dùng) cũng trượt.
  - Giá trị `Json` do code dựng rồi `json.encode(…)` ra `{"variant":"Arr","value":[]}` thay vì `[]` (VM only; interp đúng).
  - Checker (`vis check`) **xanh** ở mọi biến thể trên.
- **Spec bên đúng**: interp khớp văn bản quy tắc match hiện hành (mẫu lồng khớp theo cấu trúc giá trị tại biên
  `std.json::decode` — ctct `docs/ViScript-thiet-ke.md` §match + `std/json`); VM đi qua `codegen/than.rs` + `Instr::Match*` —
  suy ra từ `--dump-bytecode`, nhánh `Some(_)` bắt all pattern lồng không phân biệt được. **Không có VD nào chốt
  pattern lồng ngữ nghĩa** — lệch thuộc lớp "hai engine lệch byte, bên đúng chưa xác định theo spec" (VD-241 §6).
- **Vai trò dừng**: không sửa codegen VM cho ctor lồng, không đổi doc JSON encode, không thêm sem lock một bên.
  Mọi việc phục vụ hai hiện tượng trên là **BLOCKED-CHO-A0**; báo cáo kem câu hỏi + đề xuất (doctest/integration
  test byte-identical cho mọi pattern lồng sau khi A0 chốt).

## Câu hỏi cho Agent 0 (xếp theo mức ảnh hưởng)

1. **Chuẩn `T??`**: annot `int??` là lỗi kiểu (§4.4 trên văn bản) hay hợp lệ `Opt(Opt(T))` thật? (đề nghị: chọn một
   và ratify bằng một VD; nếu (ii), sửa codegen VM trước, add ca khoá).
2. **VM pattern lồng**: coi là bug VM (ưu tiên sửa) — đề nghị VD cho phép nhánh (a): sửa `codegen` cho mọi mẫu ctor
   lồng, add differential 473 → 473+n roundtrip; nhánh (b): nếu A0 thấy thiết kế không bảo đảm ctor lồng thì ghi nợ và
   cấm mẫu lồng theo annot (không recommend — `std.json` dùng `Some(Str(..))` ở mọi ví dụ JSON thật).
3. **Json do code dựng + `json.encode` trên VM**: lỗi codegen VM độc lập (rất có thể cùng gốc với (2)); đề nghị VD gộp.
4. **`vis check` xanh cho lỗi chỉ nổ runtime** (tên không tồn tại ca 14, closure unannotate kiểu sai ca 7, record dựng
   thiếu field ca 10, map key thiếu `m["k"]` ca 10, `chan(0)` ca 13, destructure không phải tuple ca 20 — `ergo-c` report +
   `ergo-b` t23/t19): là **chính sách** — đề nghị A0 chọn: (a) mở rộng tyck (đụng ngữ nghĩa — ratify riêng),
   (b) giữ 'check chỉ ngăn một phần' + tài liệu rõ (rẻ, không đổi nghĩa), (c) tắt strict-caps mặc định thì cũng nên
   ghi rõ ở README. Đề xuất (b) ngay, (a) sau nếu A0 muốn.
5. **`fn main` trả `Err` ⇒ exit 0 im lặng** (VD-178 R4 `[DỰ KIẾN]` chưa nối dây — VM/interp đều vậy; `RunOutput.exit`
   được tính nhưng CLI không đọc): đề nghị VD nối dây `exit = 1` hoặc ghi rõ "Err không làm đỏ process" (chọn một).
6. **`print(f)` Show closure lệch hai engine** (`<fn>` interp vs `<hàm>` VM — cosmetic): đề nghị VD chốt một bên.

## Các nhiệm vụ VD-241 khác chạy bình thường; chỉ nhánh `int??` / VM pattern-lồng dừng chờ A0

- Đợt **phân tích công thái học A0 yêu cầu (2026-11-19)** tiếp tục (chỉ đọc, không đổi mã): thu thập → `VISCRIPT-ERGONOMICS-{FACTS,EXAMPLES,DIAGNOSTICS}.md`.
- `#152-vế-mac` / `#147-mac`: vẫn chờ mps (người dùng chạy prompt trên mac).
- `pha1-rc1` tuyệt đối không di chuyển/xoá.
