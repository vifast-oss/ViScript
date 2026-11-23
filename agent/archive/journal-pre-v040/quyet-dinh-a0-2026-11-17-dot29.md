# QUYẾT ĐỊNH AGENT 0 · ĐỢT 29 · 17/11/2026 (chạy qua đêm 17→18/11)

**Từ**: Agent 0 · **Gửi**: Agent 1 (phần A, B), Agent 2 (phần C)
**Căn cứ**: BAO-CAO-DEM-AGENT1-CHO-AGENT0-2026-11-17.md, HEAD cc88ca23
**Mã quyết định đợt này**: VD-157 … VD-166 · **Task**: ST-160 … ST-170 · **Báo cáo**: R29

---

## PHẦN A. DUYỆT ĐÊM 17/11

### A1. Ký 13 hàng + 3 báo cáo (§3.1)
Ký **toàn bộ 13 dòng APPROVED** đúng nguyên văn Agent 1 soạn ở §3.1. Agent 1 dán dưới từng hàng trong QUEUE.md (ST-160).

### A2. VD-157 · Hợp thức hoá quyết định 17/17b/17c/17d và giới hạn quyền ban hành đêm
- Quyết định 17, 17b, 17c, 17d do Agent 1 ban hành thay Agent 0: **công nhận**.
- Từ đợt 29, khi Agent 1 đóng vai Agent 0 qua đêm, **chỉ được** ban hành cho: nợ nhỏ không cần thiết kế, việc sổ, đo đạc, báo cáo.
- **Không được** tự quyết (phải để lại cho Agent 0, ghi BLOCKED-CHO-A0): thiết kế/API mới, chính sách crate/unsafe/giấy phép, **đổi kỳ vọng của ca cũ**, đổi ngưỡng hoặc cách đo ngưỡng.
- Ghi chú: sự cố §5 #9 (đổi kỳ vọng cap_khong_lo::doi_so_dong_lenh_la_str) là đúng về kỹ thuật và **được công nhận hồi tố**, nhưng theo VD-157 lần sau phải chờ Agent 0.

### A3. VD-158 · Sự cố §5 #1 KHÔNG được xếp T-INFRA
Ca đua `dua_doi_symlink_lap_co_gioi_han` đỏ **1/50 000** là **tín hiệu có thể là lỗi thật** (TOCTOU đường đọc), không phải nhiễu hạ tầng cho tới khi chứng minh được. Đổi loại thành **CHƯA-PHÂN-LOẠI**, mở nợ **#146**, điều tra ở ST-163.

### A4. VD-159 · Ca thời gian §5 #2
Giữ ngưỡng **100 ms**. Đổi phép đo của `hieu_nang_nap_module_5_tep` từ "1 lần" sang **min của 5 lần chạy** + ghi load average vào log. Đây là đổi cách đo, **Agent 0 duyệt rõ tại đây**, không tính là nới ngưỡng. Làm ở ST-164.

### A5. Chốt §3.3, §3.4, §3.5
- §3.3: **Đồng ý**. Windows giữ "chỉ đo", **không** sửa kỳ vọng ca cho tới khi tài liệu VS3 mục 5 được duyệt.
- §3.4: **Đồng ý**. #145 mở (VS3 kênh), #142 mở (VS3 gói), đường GHI = **#140/#109b** (VS3).
- §3.5: **Đồng ý**. Đêm nay làm #138, #132.

---

## PHẦN B. TRẢ LỜI 5 CÂU HỎI VS3 (cho Agent 1 soạn 6 tài liệu quyết định)

### B1. VD-160 · Mô hình bộ nhớ: heap riêng mỗi luồng (giữ §7.4)
- **Chọn**: mỗi luồng worker có heap riêng, bộ đếm tham chiếu **không atomic**. **Loại** phương án Arc/Mutex toàn cục.
- Send/Sync:

| Kiểu | Send | Sync | Ghi chú |
|---|---|---|---|
| Value | **Không** | **Không** | thuộc heap của một worker |
| Task | **Không** | **Không** | chạy trên executor đơn luồng của worker sở hữu |
| Chan | **Có** | **Có** | Arc + khoá bên trong; là đường duy nhất qua luồng |
| Host | **Có** | **Có** | Arc, đối tượng capability dùng chung, bất biến sau khởi tạo |

- Giá trị đi qua Chan giữa 2 worker: **sao chép sâu** (int, float, bool, str, bytes, list/dict/record của các kiểu này). Closure, handle tệp, Task **không truyền được** ⇒ lỗi kiểm tra kiểu nếu biết lúc biên dịch, nếu không thì mã lỗi lúc chạy (Agent 1 đề xuất mã trong all_codes).
- Tối ưu chia sẻ bytes/str lớn bất biến qua Arc: **hoãn**, ghi nợ, chỉ làm khi có số đo.
- Kiểm tra bắt buộc: chứng minh `Value: !Send` bằng **doctest compile_fail** (không thêm crate). Ca đỏ-trước: gửi closure qua kênh liên luồng phải ra lỗi.

### B2. VD-161 · Chính sách crate, giấy phép, unsafe
**Đính chính câu hỏi**: ring bản 0.17.x hiện đã khai giấy phép **Apache-2.0 AND ISC**, không còn là OpenSSL ([lib.rs](https://lib.rs/crates/ring)). webpki-roots là **CDLA-Permissive-2.0** (chỉ yêu cầu giữ ghi chú bản quyền và văn bản giấy phép), không phải MPL. Agent 1 phải **xác minh lại bằng `cargo metadata` tại đúng phiên bản ghim**, không dựa vào trí nhớ.

- **Danh sách giấy phép cho phép**: MIT, Apache-2.0, ISC, BSD-2/3-Clause, Unicode-3.0, Zlib; **CDLA-Permissive-2.0 chỉ cho dữ liệu chứng chỉ gốc**. Ngoài danh sách ⇒ BLOCKED-CHO-A0.
- **TLS**: rustls + **rustls-platform-verifier** (dùng kho chứng chỉ của hệ điều hành; Linux dùng CA bundle hệ thống, WASM dùng webpki roots). Không nhúng webpki-roots trực tiếp trừ wasm. Crypto provider: ring.
- **Bảo mật phụ thuộc**: rustls-webpki phải **≥ 0.103.16** (RUSTSEC-2026-0382). Tài liệu mục TLS phải có bước kiểm advisories trong cổng (đề xuất cargo-deny: licenses + advisories); Agent 1 ghi rõ cài đặt trên mac/ps/psvn và xử lý khi máy không có mạng.
- **unsafe**: crate của viscript giữ `#![forbid(unsafe_code)]`. **Không mở ngoại lệ D2-17**. openat/openat2 dùng **rustix** (API an toàn). Crate phụ thuộc được phép có unsafe bên trong, nhưng mỗi crate mới phải có dòng trong bảng phụ thuộc (tên, phiên bản, giấy phép, lý do, có unsafe hay không).
- Nếu mục nào chứng minh được là **không thể** làm không unsafe ⇒ BLOCKED-CHO-A0 kèm vị trí và lý do, Agent 0 duyệt từng chỗ.

### B3. VD-162 · Windows "hỗ trợ thật"
Định nghĩa (mục tiêu đo được của VS3 mục 5):
1. Chạy **native**, không cần Git Bash/MSYS trong PATH. Loại "giữ Git Bash" và "đóng gói POSIX".
2. 15 ca host phụ thuộc `/bin/*`: **thay bằng chương trình trợ giúp do repo tự build** (vd. một binary test có các lệnh con echo/cat/exit/sleep), dùng **cùng kỳ vọng trên cả 3 nền**. Không sửa kỳ vọng.
3. ~100 ca cargo test đỏ: Agent 1 phân loại từng ca vào 3 nhóm: (a) phụ thuộc `/bin/*` ⇒ xử như điểm 2; (b) lỗi thật của viscript trên Windows ⇒ sửa; (c) khác biệt bản chất nền (bit quyền POSIX, signal, symlink…) ⇒ được phép **kỳ vọng theo nền** (mở rộng VD-132) nhưng phải nằm trong **một tệp danh sách duy nhất** có lý do từng ca, Agent 0 duyệt danh sách.
4. Chỉ được tuyên bố "hỗ trợ thật" khi: cổng Windows **10/10**, host **54/54**, và có **CI runner Windows** (self-hosted trên psvn hoặc runner dịch vụ) chạy cổng tự động. Trước đó vẫn là "chỉ đo".

### B4. VD-163 · select kênh: hàm std, không thêm cú pháp
- Dùng **hàm std** `recv_any(chans, timeout=None)` trả về `(chi_so, gia_tri)`; hết thời gian trả `none` (Agent 1 khớp với quy ước kiểu trả về hiện có). Không đụng tên `select` (lệnh chiếu cột, builtin.rs:333-337).
- Không làm nhánh gửi (send-case) ở VS3. Cú pháp riêng chỉ xét lại khi có số liệu dùng thực tế.
- #145 (chú thích rt.rs về chan(0)) xử cùng task này.

### B5. VD-164 · Nợ đường GHI và ma trận phụ thuộc
- **Xác nhận** đường GHI là **#140/#109b** (VD-154a).
- **Xác nhận** ma trận: mục 1 → mục 2/3/4; mục 3 → mục 6. Mục 5 (Windows) độc lập, có thể chạy song song.
- Tài liệu VS3 phải ghi ma trận này **kèm tên mục**, không chỉ số.

---

## PHẦN C. GIAO VIỆC ĐÊM 17→18/11

**Quy tắc chung**: mỗi task ≤ 3 h; đỏ-trước khi sửa lỗi; không nới ngưỡng; không #[ignore]; không unsafe; không crate mới; log không ghi đè; gate --fast sau mỗi task có mã. **Không mở hàng VS3** (VD-148) tới khi Agent 0 duyệt 6 tài liệu.

**Thứ tự chạy**: ST-160 → ST-162 → ST-161 → ST-163 → ST-164 → ST-165 → ST-166 → ST-167 → ST-168 → ST-169 → ST-170.

| Task | Ai | Nội dung | Điều kiện xong |
|---|---|---|---|
| **ST-160** | Agent 1 | Dán 13 APPROVED (A1) + ghi VD-157…VD-164 vào JOURNAL/quyet-dinh-a0-2026-11-17-dot29.md; mở nợ **#146** (VD-158) | check_report PASS; sổ **147** mục, 0 trùng |
| **ST-161** | Agent 2 | Đo lại tại HEAD mới nhất: ps gate --fast; psvn cargo test + self-test + cổng (chỉ đo) | ps **dự kiến L0 1182/0** (1169 + 9 ST-151 + 4 ST-157). Lệch thì giải thích theo từng suite, không suy đoán. psvn ghi số mới thay cho 1053/100 @3dc13342 |
| **ST-162** | Agent 2 | Chuẩn hoá script bọc cổng từ xa (sự cố §5 #4, #5, #6): đường tuyệt đối cho cargo và log; mã thoát ghi ra tệp `.exit` riêng (không dựa vào `$?` qua PowerShell; trên psvn đọc `$LASTEXITCODE`); log psvn ghi ra thư mục bị .gitignore hoặc ngoài repo | Ca kiểm: lệnh giả thoát mã 3 ⇒ tệp .exit chứa 3 trên cả ps và psvn; git pull trên psvn không bị chặn sau 1 lần chạy cổng |
| **ST-163** | Agent 2 | Điều tra #146 (ca đua symlink, VD-158): chạy **500 000** vòng trên mac và ps, thu đúng assertion hỏng | Phân loại có bằng chứng: (a) lỗi của test ⇒ sửa test, giữ ý nghĩa ca; (b) lỗi TOCTOU thật ⇒ ca đỏ-trước + sửa + BLOCKED-CHO-A0 nếu cần đổi thiết kế. Nếu **không tái hiện** trong 3 h: thêm thông tin chẩn đoán vào thông điệp lỗi của ca, giữ #146 mở, ghi số vòng đã chạy |
| **ST-164** | Agent 2 | VD-159: `hieu_nang_nap_module_5_tep` đo min của 5 lần, ngưỡng 100 ms giữ nguyên, log ghi load average | 3 lần gate liên tiếp xanh ca này; diff chỉ đụng phần đo |
| **ST-165** | Agent 2 | Nợ **#138** (/proc) | Đỏ-trước → xanh; #138 đóng; gate PASS |
| **ST-166** | Agent 2 | Nợ **#132** (crt-static) | Đỏ-trước → xanh; targets vẫn 18/18 cả 3 nền; #132 đóng |
| **ST-167** | Agent 2 | Soak 10 vòng **cùng một HEAD** trên mac và ps (mac 1352 s ở HEAD cũ so với ps 250 s) | Ghi 2 số cùng HEAD. Nếu chênh > 3 lần: mở nợ điều tra hiệu năng, **không sửa đêm nay** |
| **ST-168** | Agent 1 | Soạn **6 tài liệu quyết định VS3** theo phần B (mỗi mục: mục tiêu đo được, API, ca đỏ-trước, chia task ≤3 h, cổng, phụ thuộc). Chỉ đọc, 0 dòng mã | 6 tệp trong JOURNAL/vs3-qd-1…6.md; mỗi tệp dẫn chứng file:dòng; mục 2 VS3 (TLS) có bảng phụ thuộc theo B2; mục 5 có bảng phân loại ~100 ca theo B3 |
| **ST-169** | Agent 1 | Bảng phân loại 100 ca Windows (đầu vào cho ST-168 mục 5), dựa trên log ST-161 | Mỗi ca 1 dòng: tên, nhóm (a/b/c), bằng chứng log |
| **ST-170** | Agent 2 | Báo cáo **R29** | check_report PASS; head, blocked, số đo 3 nền cùng HEAD |

**Nếu xong sớm**: dừng, không tự mở nợ khác ngoài #138/#132. Không đụng #140/#142/#145.

---

## PHẦN D. ĐIỀU AGENT 0 SẼ KIỂM SÁNG 18/11
1. ps L0 có đúng 1182/0 không, nếu không thì giải thích có khớp từng suite không.
2. #146: đã phân loại (a) hay (b), hay chưa tái hiện; số vòng đã chạy.
3. Soak mac và ps cùng HEAD.
4. 6 tài liệu VS3: có đủ mục tiêu đo được và ca đỏ-trước; Agent 0 duyệt xong mới mở hàng VS3 đầu tiên (mục 1 async song song).
