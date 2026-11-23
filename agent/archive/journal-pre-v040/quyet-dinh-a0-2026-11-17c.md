<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# QUYẾT ĐỊNH AGENT 0 (bổ sung 2) · Dòng ViScript · 2026-11-17, 06:42 JST

**Căn cứ**: `JOURNAL/st155-vs3-dieu-tra.md` (ST-155, chỉ đọc); `R26`; hiến chương VS3 ở `JOURNAL/quyet-dinh-a0-2026-11-17.md` §5; `SO-NO.md`.

## §1. Đính chính hiến chương VS3 §5 (lỗi của tôi)
Câu "Quyền: … nợ `#142/#109b`" là **sai số**: `#142` (VD-149) là **`pkg update` ghim mọi gói khác**; nợ **đường GHI** là **`#140`** (và `#109b`). Từ nay trích **`#140/#109b`**. Không sửa lại bản dán cũ (giữ nguyên lịch sử); dòng này là **đính chính chính thức**.

## §2. `#143` → **chuyển VS3** (điều kiện VD-133 đã thoả)
VD-133 nói nợ `#114` (`NamePolicy::Strict`) chuyển VS3 **khi ca khoá trên xanh**; ca thứ 8 `st130b` **đã xanh** từ đợt 23 ⇒ **chuyển VS3**. `SO-NO.md`: `#143` → **VS3**; sổ thành **90 mở / 31 đóng / 21 VS3** / 1 không làm / 1 điều hướng (144 mục). R27 phải **đính chính** con số 91/20 của R26.

## §3. Cấp số `#145` cho lệch tài liệu–mã `chan(0)`
`rt.rs:128-129` nói literal `chan(0)` bị **tyck** chặn (exit 2); thực tế **runtime** mới chặn (exit 1, `kenh_cap0.rs:57-68`), `tyck/expr/kenh.rs:16-33` **không** kiểm `cap`. Ghi **#145** = "chú thích `rt.rs` nói sai đường chặn `chan(0)`; sửa chú thích hoặc thêm kiểm tyck" — **mở**, làm khi có task VS3 kênh (không phải VS2).

## §4. Ghi nhận từ ST-155 (không cấp số)
1. **Không có CI** trong repo (không `.github/`): mọi số Windows phụ thuộc SSH `psvn` ⇒ rủi ro `T-INFRA` (VD-136), **không** retry được (D2-07). Ghi vào R27 §"hạn chế bằng chứng".
2. Hai ca host Windows "xanh" (`suite.txt:45` `/bin/sh -c "exit 3"`, `:46` `/khong/co/lenh`) **kỳ vọng lỗi** ⇒ xanh vì chương trình **không tồn tại**, **không** chứng minh hỗ trợ ⇒ khi làm VS3 mục 5 phải thay bằng ca âm/dương thật.
3. `libc` **đã có trong `Cargo.lock`** (bắc cầu, 0 dòng dùng) ⇒ "dùng libc" **không** thêm cây phụ thuộc, nhưng vẫn cần VD cho `unsafe` (D2-17).

## §5. Hàng mới / lật hàng
- **ST-154** (đã có trong QUEUE, `READY`): làm **nợ #144** + gộp việc sổ của §2/§3 (đổi `#143`→VS3, thêm `#145`) + cập nhật đếm; `SO-NO` phải khớp script đếm.
- **ST-155 = DONE** (Agent 1 đã đối chiếu: 222 dòng, 57 dẫn chứng, gate macOS PASS L0 1175/0, check_report 26/173).
- **Không** mở hàng VS3 nào; tài liệu thiết kế VS3 do Agent 0 soạn sau khi trả lời 5 câu hỏi (`st155` §8) — **chưa** quyết trong đêm nay.

## §6. Điều kiện dừng đêm nay
07:50 JST: **không** mở việc mới. 08:10: dừng hoàn toàn, cập nhật báo cáo bàn giao; `R27` (nếu mở) chỉ ghi số đã có. Mọi thứ chưa làm được để `READY` cho đợt sau, **không** BLOCKED giả.
