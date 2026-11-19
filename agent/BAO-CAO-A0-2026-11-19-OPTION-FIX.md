# BAO-CAO-A0 — Đợt sửa Option lồng: `None` theo kiểu đích Option (VD-241 §7)

Gửi A0 · từ ps (Agent 1) · 2026-11-19 · **đã merge `main` @ `3b2c67e`** (fresh-clone gate PASS)

## 1. Phạm vi A0 cho phép & kết quả thực thi

> A0 duyệt mở đợt sửa Option riêng ngay — không chờ đóng pha 1, không gộp Windows portability, không đổi auto-lift/pattern/`MakeSome`.
> Windows Tier 2 giữ nguyên đóng @ `349af67`; `pha1-rc1` nguyên vẹn `05e42d0…`.

Hoàn tất fix + review chéo (khác tác giả) + merge `main` `3b2c67e` (fast-forward từ `rev/vd241-option-none-fix-2026-11-19`, rebase lên `7bf7c23`, không đổi nội dung):

- `./vis-dev gate --fast`: **PASS** (11/11 bước: fmt/clippy/R1..R6/self-test/explain/report-check/build/doc-examples/L0 **1297/0**)
- `cargo test --workspace --no-fail-fast`: **1297 pass / 0 failed**
- `vis test test/sem`: **486/486** · `vis test --vm test/sem`: **486/486** (byte-identical)
- **Fresh clone** (`https://github.com/vifast-oss/ViScript` @ `3b2c67e`): gate **PASS**, sem **486/486 ×2**

## 2. Nguyên nhân (xác nhận bằng mã) & cách sửa

**Nguyên nhân (một điểm sai, không phải ba lỗi độc lập)**:
- `crates/vis-types/src/tyck/vd238.rs` · `la_none_chep_duoc_opt_1` — chỉ chấp nhận `Type::Opt(inner)` với `inner` **không** phải `Opt` (tức đúng một lớp Option).
- Dùng ở **3 chỗ** trong `crates/vis-types/src/tyck/mod.rs`:
  1. dòng **~524** `coerce_e` (gán `let`, gán lại, đối số, `let … else`, hằng);
  2. dòng **~943** `kiem_than` (thân-hàm cuối là `None` trần);
  3. dòng **~1133** Return-arm (`return None`).
- Đo trước fix: `let e: int?? = None` ⇒ `VS0412 3:20` ("`None` chỉ gán được cho `T?`… (VD-238)").

**Fix**: `la_none_dich_opt` — chỉ xét **lớp ngoài cùng**:
`Type::Opt(_)` ⇒ hợp lệ (mọi độ sâu: `int?`, `int??`, `int???`); `Never`/`Error` giữ nguyên; mọi annot ngoài cùng non-Option vẫn **VS0412** — kể cả `list[int?]`, `map[str,int?]`, `Result[.., int?]` (pin âm mới, `vd_205`); alias/type-alias phân giải về `Type::Opt` **trước** gate ⇒ được chấp nhận; tham số kiểu mở `T` (vd `Hop[T]`) vẫn `TypeRef` ⇒ vẫn **VS0412**.

Không đụng: auto-lift, `lift_opt`, pattern matching, flatten, `MakeSome`, ABI VBC1, API-STD, SPEC-lex.

## 3. Corpus (đổi 1 · thêm 4 · xoá 1; không sinh golden hàng loạt)

| Tệp | Thay đổi | Lý do |
|---|---|---|
| `test/sem/optional/long-reject-e.vis` + `.err` → `long-none-e.vis` + `.out` (`sn`) | `.err` → xanh | `int?? = None` hợp lệ (VD-241 §7) |
| `test/sem/optional/none-three-states.vis` + `.out` (`42`/`sn`/`none-out`) | **mới** | Ba trạng thái phân biệt được qua `x = None` (biên gán không lift) — chứng minh không flatten |
| `test/sem/optional/long-none-param.vis` + `.out` (`1000`) | **mới** | `g(None)` với `x: int??` hợp lệ (tham số biên lift ⇒ `Some(None)`) |
| `test/sem/optional/long-none-return.vis` + `.out` (`sn`) | **mới** | Thân-hàm `-> int??` bằng `None` trần hợp lệ (Replace arm `kiem_than`) |
| `crates/vis-cli/tests/vd_205.rs` | +3 ca âm (`list[int?]`, `map[str,int?]`, `Result[int?,str]`) + 1 test fn xanh `none_dich_ngoai_cung_option_la_ok` (5 ca, hai engine byte-identical) | Khoá hai chiều |

**Số ca**: 483 → **486** (**436 `.vis`** + 50 `.vish`); **`.err` 89 → 88**. Mọi khoá cứng đã cập nhật đồng bộ kèm lý do: `sem.rs`, `vis_test.rs` (SO_CA), `diag_cmp.rs` (486/398/53/35), `roundtrip.rs`, `vs_interp.rs`, `fuzz_mutate.rs` (394→398), `tin_tyck.rs`, CHEAT-SHEET (486), FACTS §CẬP NHẬT VD-241 §7.
> ⚠️ Lệch lệnh nguyên văn đã nêu ("so_ca tổng vẫn 483" + "thêm 3 ca"): không thể thoả cả hai; giữ nguyên mọi ca cũ + 3 ca mới ⇒ tổng buộc 486. Nếu A0 muốn đúng 483, phải chỉ rõ ca nào bị thay.

## 4. Review chéo (Agent 2 khác tác giả) — verdict **SHIP**

- Bảng cổng re-run trong worktree: workspace 1297/0 · sem 486/486 ×2 · gate 11/11.
- Mutation 1 (revert gate về single-layer): 4 ca khoá đỏ (`long-none-e`, `none-three-states`, `long-none-param`, `long-none-return`) + `vd_205::none_dich_ngoai_cung_option_la_ok` đỏ (5/5) + 8 test/5 target pin đỏ — **khoá đủ**.
- Mutation 2 (nới gate thành "annotation **chứa** Opt ở bất kỳ đâu" — đệ quy List/Map/Result/Fn/TypeRef): **3 ca âm mới phải đỏ** — cả ba đều đỏ dưới mutant ⇒ ca âm không thừa; mutant bị chặn.
- Hai engine **byte-identical** ở mọi ca mới và 8/8 `.out`.
- Corpus: toàn bộ hunk đều thuộc nhóm hợp lệ (hành vi 1 điểm · corpus · pins · docs) — SPEC-lex/SPEC/freeze/`pha1-rc1`/lift/pattern/interp/VM/codegen **không** xuất hiện trong diff.
- Không tuyên bố Windows/macOS (chưa chạy).

## 5. Phát hiện hành vi runtime — cần A0 chốt (CHƯA sửa gì)

1. **`let e: int?? = None` lúc chạy == `Some(None)`** (biên `let` lift **một** lớp theo VD-227/228/229, không đổi); VM/interp giống nhau. Hai cách lấy `None` thật: **`x = None`** (gán lại cùng binding — biên gán không lift) hoặc **annot spelled-out** `Option[Option[int]]` (hoặc alias `type Y = Option[Option[int]]`).
2. **Bất đối xứng annot spell** (`TypeRef` vs sugar, pre-existing từ VD-227/228/241 §3, không phải fix này, hai engine vẫn khớp nhau):
   - `let z: int?? = None` → `Some(None)` · `let z: Option[Option[int]] = None` → `None` · `let z: Y = None` (alias) → `None`
   - Hệ quả: `match` đầu tiên cho cùng nghĩa kiểu bắt arm khác nhau (`sn-arm` vs `none-arm`).
3. **So_ca 483→486** (nêu ở §3).
4. **CHEAT-SHEET** đã cập nhật 486. **Docs thiết kế/SPEC** `docs/ViScript-thiet-ke.md` vẫn còn ~4 chỗ + §7.3 ghi `T??` là lỗi kiểu ("SPEC-lex chỉ qua A0" — chưa đụng).
5. **Doc-comment của gate** (`vd238.rs`): tham số kiểu mở (`fn f[T](Hop[T])`) vẫn từ chối bằng `TypeRef` (đo: `Hop[T] = None` ⇒ VS0412) — đúng cơ chế; ca `let b: T = None` trong `fn f[T]` cho **VS0303** (resolve fall-through cũ) — không phải lỗi fix; ghi chú ở đây để tránh tranh cãi kỳ vọng sau này.
6. **Windows/macOS chưa re-verify bản fix** (code đổi): mac đánh giá tại `a4602cb` **trước** fix — #147/#152 cần đo lại tại HEAD mới; Windows Tier 2 vẫn gắn với `349af67` (đã đóng đúng phạm vi cũ).

## 6. Câu hỏi A0 (không tự quyết)

1. **486 chốt** hay muốn đúng 483 (nếu 3 ca mới nên thay vào ca nào)?
2. **Runtime `let x: T?? = None` == `Some(None)`**: giữ hiện tại (ghi chú tài liệu) hay mở VD riêng sửa lift cho bare None? (Phạm vi fix này không gồm; không tự mở.)
3. **Bất đối xứng sugar vs spelled-out/alias ở biên `let`**: VD riêng (resolve `TypeRef` bọc Option → `Type::Opt` trước lift) hay ghi nhận vào spec/annot cố định?
4. **Báo cáo/lịch sử** `agent/BAO-CAO-A0-2026-11-19-1521.md` ("sem 483", "long-reject-e .err khoá") giữ lịch sử; không cập nhật — A0 duyệt đúng ý không?
5. **Doc** `docs/ViScript-thiet-ke.md` 4 chỗ + §7.3 (`T??` là lỗi kiểu) — A0 có muốn cập nhật?
6. **Mac/#147/#152/Windows re-verify bản fix**: A0 có muốn ưu tiên đo tại HEAD mới (mac rảnh) trước khi vòng thực hiện tiếp theo không?

## 7. Trạng thái không đổi

Windows = Tier 2 @ `349af67` (đã đóng phạm vi); `pha1-rc1` `05e42d0…` nguyên vẹn; VD portability vẫn BLOCKED (chờ pha 1 đóng chính thức); Phase B-3 vẫn chờ ratify riêng; lambda annotation / #155 quy trình riêng; Q-OPTION-1/2 đồng bộ theo đính chính A0 (e/f hợp lệ theo kiểu đích); Q-OPTION-3 giữ nguyên; mac #147/#152 hoãn chờ máy rảnh (load < 0,3).
