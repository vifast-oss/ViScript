# ROADMAP pha 2 — NHÁP (VD-241 §4 việc 6, 2026-11-19)

> **Chỉ là nháp — KHÔNG thực hiện khi chưa có VD của Agent 0.** Mọi mục dưới đây là ứng viên để A0
> chọn/xếp thứ tự. Không mục nào được bắt tay; mỗi mục ghi: **lợi ích · rủi ro · số golden dự kiến đổi ·
> phụ thuộc**. Các con số golden là **ước lượng nháp** (đếm từ corpus hiện tại 473 ca; phải đo lại trước khi làm).

**HEAD khi nháp**: `6295c8e` (sau merge VD-240 grid). Nội bộ: `pha1-rc1` == `05e42d0` đã đóng pha 1.

---

## Ứng viên

### A0-1 · `#151` — doc-examples đặt tên theo **định danh ổn định** (tài liệu, VD-198(2))
- **Vấn đề**: tệp `test/doc-examples/doc-<tên>-<dòng>.vis` đổi tên hàng loạt khi chèn một đoạn văn; dễ **che giấu thay đổi kỳ vọng thật** trong đám tên đổi; review khó.
- **Hướng**: tên theo **slug tiêu đề mục + số thứ tự khối** trong mục (hoặc nhãn tường minh trong khối); regenerate một lượt, cập nhật bộ kiểm.
- **Lợi ích**: diff review dễ, lỗi khi chèn văn không còn ngập vàng; bảo vệ kỳ vọng thật của doc-examples.
- **Rủi ro**: số ca `doc-examples` hiện ~24 khối; một lần đổi tên hàng loạt (một commit, dễ review nếu tách riêng).
- **Golden**: 0 đổi nội dung (chỉ đổi **tên tệp** + khớp với `doc_excluded.rs`/allowlist/bảng toạ độ `hir.rs`). Công ~1–1,5 ngày.
- **Phụ thuộc**: không. Có thể làm sớm nhất trong các mục.

### A0-2 · `#149` — Tách đường dẫn thủ công còn rải rác (sửa lặp khi chạm tệp, VD-184(3))
- **Vấn đề**: `ten_goc` (ST-211) mới sạch `check.rs`; mọi chỗ còn lại trong
  `agent/PATH-SPLIT-AUDIT-2026-11-18.md` vẫn tách đường dẫn thủ công (`/` + `\` không nhất quán trên Windows/mac).
- **Lợi ích**: loại hẳn lớp lỗi nền tảng trên Windows (VS3-5a..5e) và mac; đồng nhất hoá `Path::file_name()`-style API.
- **Rủi ro**: chạm nhiều tệp; hành vi đường dẫn phải byte-identical trên 3 nền; test Windows chưa có (VS3-5a chưa chạy).
- **Golden**: 0 đổi (không đụng ngữ nghĩa); kỳ vọng 0 ca đỏ sau sửa. Công ~0,5–1 ngày + test tay trên `psvn`.
- **Phụ thuộc**: thứ tự hợp lệ khi VS3-5a Windows khởi động; có thể làm song song với mục khác nếu chỉ `check.rs`-adjacent.

### A0-3 · **`lambda annotation`** (cú pháp annot tham số lambda, dẫn dắt cho A0-4)
- **Vấn đề**: `lambda := 'fn' '(' params? ')' ['->' type] ':' suite | lambda_params '=>' expr` —
  `lambda_params := IDENT | '(' IDENT (',' IDENT)* ')'` (dòng 1302–1304 `docs/ViScript-thiet-ke.md`):
  **không annot được** trên params lambda — `(x: int?) =>` cho `VS0201` — nên closure không mang được kiểu tham số tường minh.
- **Hướng**: mở `lambda_params := IDENT | '(' (IDENT [':' type])* ')'` (EBNF + SPEC-lex qua A0); checker forward annot; interp/VM không đổi (closure params vốn untyped ở runtime — annot chỉ cho checker).
- **Lợi ích**: mở khoá annotation cho closure **các** use-case: lift params (A0-4), `#155`, đối số `g(…)` lift an toàn khi lambda annot.
- **Rủi ro**: đụng **SPEC-lex** + EBNF + cheat-sheet; parse đường `lambda_params` vs nhóm `(expr)`/tuple dễ nhầm — cần ca corpus dày ~15–25 ca.
- **Golden**: 0 đổi nghĩa cho lambda cũ; **thêm ca mới** (≥ 10 ca mới ~= +10 tới +20 ca sem) + regenerate doc-examples nếu EBNF docs cần khớp.
- **Phụ thuộc**: quyết định SPEC của A0 (ràng buộc §"`(` IDENT … `)`" vs `(expr)`).

### A0-4 · `#155` — **Lift tham số closure khi lambda annot có** (VD-232, sau A0-3)
- **Vấn đề**: closure-biên return đã nâng (VD-229), nhưng **closure params không nâng** được vì không annot (A0-3); phần code lift params cho closure chưa có — A0 chốt: **ghi nợ, không xếtp lịch khi chưa có syntax**.
- **Lợi ích**: khớp ngữ nghĩa với HFn (tham số annot `int?` nhận `42`); bỏ đám xử-lí ngoại lệ `Option-wrap` thủ công ở callsite.
- **Rủi ro**: không làm được nếu không có A0-3; chạm `call_closure_trong` + `lift_params_opt` (VM) + `Cho.ret_opt`; byte-identical hai engine **phải** giữ.
- **Golden**: 0 đổi nghĩa cũ; +~6–10 ca khoá. Công ~0,5–1 ngày sau A0-3.
- **Phụ thuộc**: A0-3 (lambda annotation + SPEC).

### A0-5 · **`T??`** — chốt nghĩa optional lồng (ngữ nghĩa, mâu thuẫn nội bộ sẵn trong thiết kế)
- **Hiện trạng mâu thuẫn** (code là nguồn sự thật):
  - EBNF + §4.4 (dòng 1358, 220, 363, 498): "`T??` là **lỗi kiểu** (optional không lồng)".
  - §7.3 + VD-178 (dòng 1153, 1183): tự nâng nhiều lớp `T → T? → T??` ở biên (`coerce`/R2 code **chấp nhận** `T??` khi annot là `T??`).
  - Hiện frontend phanh annot `T??` ra khỏi kiểu? — **chưa kiểm chứng** (cần đo: parse annot `int??`).
- **Hướng (A0 chọn)**:
  (i) Giữ nguyên lỗi kiểu — sửa dòng §7.3/VD-178 text cho khớp code; mọi `T??` annot bị `VS0412`; sun.
  (ii) Cho phép annot `T??` = `Opt(Opt(T))` thật — checker theo; lift không đổi (R2); VM/interp giữ nguyên biểu diễn.
- **Lợi ích**: kết thúc mâu thuẫn nội bộ bốn năm chưa gỡ; (ii) mở đường cho kích thước thật của Option-stack (monet mô `.get().get()` trong JSON/generic).
- **Rủi ro**: đổi ngữ nghĩa nếu chọn (ii); SPEC-lex đụng nếu annot mới hợp lệ; (i) rẻ nhưng cắt một đường "nhiều lớp" đã ghi.
- **Golden**: (i) 0 đổi; (ii) dự kiến +8–15 ca mới, 0 golden đổi cũ (annot `T??` hiện corpus không có ca). Công ~0,5 ngày (kèm đo/kiểm Parse trước).
- **Phụ thuộc**: quyết định A0 (SPEC-level).

### A0-6 · **async (a)** — song song thật đa luồng (VS3-QĐ-1; VD-235/VD-240 chỉ là hướng ưu tiên, **chưa quyết**)
- **Vấn đề**: scheduler một luồng run-to-completion (VD-240b grid pin), `Value` không `Send`/`Sync` toàn bộ;
  (a) chi phí 3,5–5 tuần-người, đụng toàn runtime; rủi ro lệch hai engine cao.
- **Hướng khi mở lại** (VD-240): **deep-copy / byte-serialize qua kênh** (khớp VD-178 QĐ-1) cho `Value` gửi qua boundary —
  **không** chuyển `Value` sang `Arc+Mutex`; (b) worker pool trên nền đó.
- **Lợi ích**: CPU-bound `spawn` song song thật; I/O chờ chồng chéo.
- **Rủi ro**: **lớn nhất mọi mục** — lệch `Value::Enum` capture closure, `RefCell` state, `Chan` internals, `VS0501/VS0505` timing; phải **tổng viết lại** kênh như async boundary; test grid VD-240 (473 ca) sẽ cần chạy song song N vòng để bắt flake.
- **Golden**: 0 golden đổi nếu byte-serialize (giá trị giữ byte-identical); +fuzz/soak nặng. Lần đầu có thể đổi output thứ tự in (non-deterministic thread) — cần quy tắc tất định hoặc chấp nhận nondeterminism + doc-examples ghi chú.
- **Phụ thuộc**: quyết định kiến trúc A0 (rẻ nhất khi sau #151/đổi doc-examples? không — phụ thuộc ít). **Chưa xếp lịch** theo VD-240.

### A0-7 · `#152` — chờ đo chuẩn opt-bench (**KHÔNG** xếp tối ưu; chỉ đo, VD-241 §3)
- **Trạng thái**: đo A/B (A=`7c256df` trước enum-hoá vs B=HEAD) theo VD-241 §4 việc 3 — **ps**; mac cùng HEAD qua người dùng; **chưa có số máy rảnh đủ 20 phút** ở lần viết này; giữ "chờ đo chuẩn".
- **Nếu khoảng B/A nằm **hẳn một phía 1,10**** ở cả VM và interp ⇒ đóng; **nếu lệch** ⇒ giữ mở, **KHÔNG** tối ưu ctor-enum (VD-241 cấm).
- **Lợi ích**: số đầy đủ chốt nợ; **Rủi ro**: không (đo, không đổi code).
- **Golden**: 0. Công: 2 h (ps) + run mac + ghi SO-NO.

### A0-8 · `#154` — "Phương án B" (impl_variant; **không xếp lịch**, VD-208/VD-204(5))
- Ghi nhận để không quên: ctor-enum tối ưu hoá "A" (index hằng thay `String` cho variant name) sẽ gộp đính hướng chữ hằng nếu #152 đóng mở hơn 10%; hiện **hoãn vô thời hạn**.
- **Golden**: chưa định lượng (chỉ xếp vào hàng sau #152 đóng).

---

## Thứ tự gợi ý (A0 chọn — KHÔNG phải cam kết)

`A0-1 → A0-2 → A0-5 → A0-3 → A0-4 → (A0-7 khi có số mac) → **A0-6 bài tính riêng**`

A0-7 là đo đơn thuần, độc lập; A0-6 chỉ khởi động khi có VD riêng từ A0.
