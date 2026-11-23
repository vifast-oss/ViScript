<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-171 · psvn: 1 ca đỏ mới trên clone sạch — `audit_span` (2026-11-17)
**Hiện tượng**: gate `psvn` trên clone mới `/c/Users/ps/viscript-moi` (HEAD `ede97dc`) dừng ở L0:
`audit_span::moi_node_hir_mang_dung_file_cua_tep_sinh_ra_no` — `phải audit được ≥10 đơn vị đa tệp (thấy 0) — fixture thiếu?`
(60 passed / 1 failed trong suite `vis-cli --bin vis`).

**Đã kiểm**:
- `git ls-files` mac = **2210**, psvn = **2210**; `git status --porcelain` psvn = **0** ⇒ clone đủ tệp, không thiếu fixture.
- Test quét `test/` tìm tệp có `use "./…"` hoặc `use pkg.` (hàm `co_the_da_tep`), rồi `modules::load_full`; trên psvn **0** đơn vị đa tệp được nạp ⇒ nghi **phân giải đường dẫn `./` trên Windows** (hoặc `load_full` trả `None` vì lý do nền), **không** phải thiếu tệp.
- R29 psvn (repo cũ, `39041b3f`) chỉ chạy tới 16 suite / 152 ca trước khi dừng ở `cap_proc` ⇒ ca này **chưa từng chạy** trên psvn trong R29 ⇒ **không** phải hồi quy do tách repo, mà là **lỗi Windows chưa từng lộ**.

**Kết luận**: không chặn ST-171 (mac/ps khớp R29); cần **task riêng** điều tra `load_full` với `use "./…"` trên Windows (thuộc VS3-5 nhóm a/b — kiểm sau khi có helper 5a).
