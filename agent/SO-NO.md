# SO-NO.md — Sổ nợ duy nhất của dòng ViScript

**Task**: ST-145 (lập sổ) · **ST-148** (cập nhật số 142/143 theo **VD-149**) · **ST-149** (đóng **#136**) ·
**ST-153** (ghi chú: ca 7 self-test của ST-147 **đóng ở ST-153**, §7.4 — không cấp số mới) ·
**ST-151** (đóng **#135**) ·
**ST-154** (đóng **#144**; `#143` → **chuyển VS3** theo **VD-154(b)**; thêm **#145** theo **VD-154(c)**) ·
**ST-157** (đóng **#110** — VM đưa `argv` vào tham số **không-`cap`** của `main`, parity interp↔VM theo **VD-156**) ·
**ST-158** (đóng **#129** — danh sách 7 khối tài liệu bị loại trừ (VD-36) về **một nguồn** `test/doc_excluded.rs`;
4 tệp test dùng `include!`, không chép số) ·
**ST-237** (ma trận 28 phát hiện: 28/28 reproduced; **#176 bác bỏ có bằng chứng** theo `SPEC-lex.md:44` ⇒ `không làm có lý do`; **#172** chờ A0 chọn bão hoà hay `VS0507` — `BLOCKED.md` §12) ·
**ST-242** (host identity + addendum macOS: **#147/#152 → blocked do phần cứng** — không có máy Darwin thật, `vis-remote mac` trả Linux; **vẫn giữ "mở" và vẫn đếm**, không xoá số để giảm tổng; §8 **185 = 121 mở / 38 đóng / 23 VS3 / 2 không làm / 1 điều hướng**) ·
**ST-238** (sửa xong Đợt A; **#157/#158/#159/#160/#163/#164/#167/#169/#181 → đóng ở ST-238**; review từng finding `JOURNAL/st238-review.md`; §8 **185 = 112 mở / 47 đóng / 23 VS3 / 2 không làm / 1 điều hướng**) ·
**ST-239** (Đợt B; **#161/#162/#165/#166/#170 → đóng ở ST-239**; review `JOURNAL/st239-review.md`; §8 **187 = 109 mở / 52 đóng / 23 VS3 / 2 không làm / 1 điều hướng**) ·
**A0 bổ sung sau ST-238** ( thêm **#185** wrapper/toolchain, **#186** lệch chẩn đoán ST-28; §8 **187 = 114 mở / 47 đóng / 23 VS3 / 2 không làm / 1 điều hướng**) ·
**ST-240** (đóng 13 nợ std/CLI #168/#171/#172/#173/#174/#175/#177/#178/#179/#180/#182/#183/#184; §8 **188 = 97 mở / 65 đóng**) ·
**ST-248** (thêm **#187** stale cache khi đổi codegen; `BLOCKED.md` §16) → **ST-249 đóng #187**; §8 **188 = 96 mở / 66 đóng** ·
**ST-247/VD-245** (điều tra miền global xong; thêm **#188**: global chưa khởi tạo ⇒ `Unit`; A0 chọn runtime trap; **ST-250 đóng #188**, thêm **#189** finding capture/local; **VD-246/ST-252 đóng #189**, thêm **#190** finding `pop_frame`; helper toolchain đóng **#185**, ST-254 đóng **#156**; §8 **191 = 95 mở / 70 đóng**) ·
**Luật gốc**: `DECISIONS.md` **VD-147(3)** (kèm VD-142, VD-143, VD-145, VD-148, VD-149, VD-153, VD-154, VD-156)
**Ngày lập**: 2026-11-16 · **Người ghi**: Agent 2 dòng ViScript · **HEAD khi lập**: `b1ee0b33`
**Cập nhật**: 2026-11-23 (**ST-297 đóng #149** — `f4cb136` (nhánh `4cad7a0`): 4 ca mới UNC/mixed/percent + `--grant` Windows; 3× full Windows **3611/0/3**, Linux 10× `vis-lsp` **29/29** + `path_impl` **2/2`; §8 **192 = 84 mở / 87 đóng / 18 VS3 / 2 không làm / 1 điều hướng**); trước đó 2026-11-22 (**Stage 0 `v0.3.6` đủ 6/6** @ `f8f1d4b` — `ST-284` R0a (`vm.rs` **1240** / `frame.rs` **1042**), `ST-285` R0b (`value` **1186** / `codegen` **1045** / `tyck/expr` **1064** / `interp/goi` **1155**), `ST-286` R0c (`async_net` **606** / `quyen` **1190** / `sqlite` **1245**), `ST-287` hook huỷ HTTP h1/h2 (`03ed84c`/merge `f8f1d4b`; h1 **5/5**, h2 **8/8**, không rò tay), `ST-288` `VS0708` + **#142**, `ST-289` `0.3.6` + **#111**/**#151**; §8 **192 = 93 mở / 74 đóng / 22 VS3 / 2 không làm / 1 điều hướng** — ST-287 không thêm nợ mới); trước đó 2026-11-22 (**ST-289 phần chuẩn bị** — đóng **#111** (gán `json.decode` vào `cap[Dir]` = **VS0412** tĩnh, khoá bằng `cap_khong_lo.rs` ca 1b) và **#151** (fixture tài liệu đổi sang **slug ổn định**, 24 tệp đổi tên thuần, 0 đổi byte)); trước đó 2026-11-21 (**ST-265/VD-250 đóng #148** — deadline tổng cho toàn bộ `http.get`/`http.head`, supersede phần timeout của VD-183; §8 **192 = 95 mở / 71 đóng / 23 VS3 / 2 không làm / 1 điều hướng**); trước đó 2026-11-20 (**VD-247/A0 chốt nghiệm thu có phạm vi; ST-259 thêm #191** — finding Windows nextest chưa đạt; §8 **192 = 96 mở / 70 đóng / 23 VS3 / 2 không làm / 1 điều hướng**); trước đó 2026-11-20 (**ST-156 đóng #156** — ma trận core/sqlite; §8 **191 = 95 mở / 70 đóng**); trước đó 2026-11-20 (**toolchain #185 đóng**; §8 **191 = 96 mở / 69 đóng**); trước đó 2026-11-20 (**ST-252** — đóng **#189**, thêm **#190**; §8 **191 = 97 mở / 68 đóng**); trước đó 2026-11-20 (**ST-250** — đóng **#188**, thêm **#189**; §8 **190 = 97 mở / 67 đóng**); trước đó 2026-11-20 (**ST-247/VD-245** — thêm **#188**, ST-250 mở; §8 **189 = 97 mở / 66 đóng**); trước đó 2026-11-20 (**ST-249** — đóng **#187** stale cache; §8 **188 = 96 mở / 66 đóng**); trước đó 2026-11-20 (**ST-240** — đóng 13 nợ std/CLI, **#172** gồm cả `BLOCKED.md` §12; **ST-248** thêm **#187**; §8 **188 = 97 mở / 65 đóng**); trước đó 2026-11-20 (**ST-239** — đóng **#161/#162/#165/#166/#170**; §8 **187 = 109 mở / 52 đóng**); trước đó 2026-11-20 (**A0 bổ sung** — thêm **#185** wrapper/toolchain và **#186** lệch chẩn đoán ST-28; §8 **187 = 114 mở / 47 đóng**); trước đó 2026-11-20 (**ST-238** — đóng **#157/#158/#159/#160/#163/#164/#167/#169/#181**; §8 **185 = 112 mở / 47 đóng**); trước đó 2026-11-20 (**ST-242** — **#147/#152 blocked do phần cứng nhưng vẫn mở/vẫn đếm**; thêm note §8; tổng **185 = 121 mở**); trước đó 2026-11-20 (**ST-237** — ma trận 28 phát hiện: 28/28 reproduced; **#176 → không làm có lý do** (SPEC-lex §44); **#172 chờ A0** qua `BLOCKED.md` §12; §8 **185 = 121 mở / 38 đóng / 23 VS3 / 2 không làm / 1 điều hướng**); trước đó 2026-11-20 (**ST-231** — thêm **#156**: `vis-targets` còn **16/18** sau SQLite MVP, `libsqlite3-sys` cần C toolchain cho msvc/wasm32; chỉ đo + ghi nợ); trước đó 2026-11-19 (**VD-231/VD-232** — **#155** thêm: nợ "lift params closure khi có annotation lambda", chuyển VS3); trước đó 2026-11-18 (**VD-182…VD-196** … VD-222; #152 đo chuẩn máy rảnh 19/11 VM 1,154 / interp 1,108 — khoảng cắt 1,10 giữ "chờ đo chuẩn"; **185 mục = 122 mở / 38 đóng / 23 VS3 / 1 không làm / 1 điều hướng**; **#147 mở lại** (VD-197), **#151** chuyển VS3, **#152** thêm) · **HEAD khi cập nhật**: `89f6e6b` (cập nhật lần cuối dưới repo HEAD này; lịch sử tương ứng)
**Tệp duy nhất**: `viscript/agent/SO-NO.md` — từ R25, mọi báo cáo trích nợ **bằng số trong sổ này**.

## 0. Mục đích

Sổ này là **nguồn duy nhất** về nợ kỹ thuật của dòng ViScript kể từ đợt 25 (VD-147(3)):

- Mỗi nợ có **một số**, **một nghĩa**; **không tái dùng số**, **không đánh lại số** (số đã dùng giữ nguyên nghĩa cũ).
- Mỗi dòng đủ **mô tả + nguồn + trạng thái** để đọc mà không cần mở báo cáo cũ.
- Bốn trạng thái: `mở` · `đóng ở <task>` · `chuyển VS3` · `không làm có lý do`.
- Nợ **VS3** gom ở §5, nợ **không làm** gom ở §6, **số trùng** xử lý ở §7.

## 1. Nguồn đã quét

Máy trích: `JOURNAL/st145-trich-no.log` (lệnh + sha256 từng báo cáo + thời điểm).

| Nguồn | Dải số | Ghi chú |
|---|---|---|
| `R11-viscript.md` §5 | 1..37 | danh sách gộp đầy đủ đầu tiên (R10 §5 + ST-63..ST-75) |
| `R12-viscript.md` §5 | 1..44 | gộp đủ R11 §5 (37 mục) + ST-77..ST-81 |
| `R13-viscript.md` §5 | 45..58 | nợ mới ST-83..ST-87b |
| `R14-viscript.md` §5 | 59..67 | nợ mới ST-89..ST-91 |
| `R15-viscript.md` §5 | 68..74 | nợ mới ST-93..ST-94 |
| `R16-viscript.md` §5 | 75..77 | nợ mới ST-96..ST-98 |
| `R17-viscript.md` §5 | 78..82 | nợ mới ST-100..ST-102 |
| `R18-viscript.md` §5 | 83..91 | nợ mới ST-104..ST-107 |
| `R19-viscript.md` §5 | 92..96 | nợ mới ST-108..ST-111 |
| `R21-viscript.md` §5 | 97..100 | nợ mới vòng 21 (ST-114a..ST-115) |
| `R22-viscript.md` §5 | 101..104 | nợ mới vòng 22 (ST-116..ST-118) |
| `R23-viscript.md` §6 | 105..131 | dãy **chuẩn hoá** sau khi ba agent đợt 23 đánh số trùng |
| `R24-viscript.md` §5 | 132..141 | nợ mới đợt 24 (ST-132b/134/135/137/R24) |
| `JOURNAL/st122..st132` §nợ | 105..116 (số cũ) | nguồn gốc từng nợ + mức VS3; xem §7 |
| `QUEUE.md` | — | dòng `nợ #…` + hàng `CHỜ DUYỆT` để chốt trạng thái |
| `DECISIONS.md` | — | VD-109/122/123/124/125/126/133/135/141/144/145/146 để chốt trạng thái; **VD-149** cấp số **142/143** cho `#113b`/`#114b` (ST-148); **VD-153** cấp **#144**; **VD-154** chuyển `#143` → VS3 + cấp **#145** (ST-154) |
| `HOST-BAO-DAM.md` | — | dòng giới hạn **đường ghi** (nợ #109b/#140) |

Báo cáo **không** có dòng nợ đánh số: `R1` (chưa có sổ nợ đánh số) và `R20` (ghi “giữ nguyên R19 §5”).
`R2..R10` có danh sách nợ **riêng của từng vòng** (số 1..N, khác dãy R11 trở đi); dãy chuẩn của sổ bắt đầu ở **R11 §5**.

## 2. Quy ước số

1. Dãy số của sổ là dãy **liên tục 1..145**: 1..141 theo thứ tự R11 → R24 (R23 đã chuẩn hoá dãy 105..131),
   tiếp **142/143** cấp ngày 2026-11-17 theo **VD-149** từ hai nhãn tạm `#113b`/`#114b` (§7.3), tiếp **144** cấp theo
   **VD-153** (vế còn lại của `#133`: `read_dir` trên thư mục **có quyền ghi**) và **145** cấp theo **VD-154(c)**
   (chú thích `rt.rs` nói sai đường chặn `chan(0)`).
2. **Không tái dùng số** đã có, **không đánh lại** số cũ; nợ mới nhận số **147** trở đi (đã cấp tới **#184**; A0 bổ sung 20/11 cấp **#185/#186**, ST-248 thêm **#187**, ST-247 thêm **#188**, ST-250 thêm **#189**, ST-252 thêm **#190**, VD-247 thêm **#191**).
3. Số trùng nghĩa cũ được **tách bằng hậu tố** `a/b`, **không** đổi số gốc: `#109a`/`#109b` (xem §7.1).
4. Hậu tố `a/b` **không** tạo dãy mới: `#109a` và `#109b` là hai nợ khác nghĩa của cùng số cũ 109.
5. Nguồn của mỗi mục ghi dạng `R## §5`/`R## §6`, kèm `ST-##`, `JOURNAL/<tệp>` hoặc `VD-###` khi cần chốt trạng thái.
6. Số **142/143** cấp theo **VD-149** (không tái dùng số cũ); báo cáo trích theo số mới, **kèm số cũ trong ngoặc**:
   `#142` (`#113b`), `#143` (`#114b`). Ánh xạ ghi ở §7.3.

## 3. Cách cập nhật (luật cho R25 trở đi)

- **Chỉ thêm** dòng mới ở cuối bảng (số kế tiếp); **không xoá** dòng, **không sửa số**, **không đổi nghĩa**.
- Cấp số chỉ khi có văn bản quyết định (như **VD-149**): số mới **không** tái dùng số cũ, nghĩa của số cũ **không** đổi;
  nhãn tạm cũ (§7.3) giữ làm ánh xạ, không thay số cũ.
- Đóng nợ: ghi `đóng ở <task>` **kèm nguồn** (báo cáo/hàng QUEUE/VD) — không đóng bằng suy đoán.
- Chuyển VS3: ghi `chuyển VS3` + lý do 1 dòng ở §5.
- Không làm: ghi `không làm có lý do` + văn bản quyết định (VD) ở §6.
- Báo cáo R## **không** sửa nội dung (VD-142); sai sót đính chính bằng dòng trong QUEUE + mục “Đính chính R##” ở báo cáo kế tiếp.
- Nợ phát sinh trong task: thêm số mới **ngay trong task đó**, không chờ báo cáo.

## 4. Bảng nợ chính (185 dòng: 1..184, #109 tách a/b)

| Số | Mô tả | Nguồn | Trạng thái |
|---|---|---|---|
| 1 | Shell-let chưa hạ: `let out = ls -l` qua `check` nhưng `run` báo `out` chưa bind. | R11 §5 + R12 §5 | mở |
| 2 | `match` không khớp ở ngữ cảnh giá trị vẫn đẩy `false` thay vì `VS0501` — nghi lệch interp ↔ VM, chưa dựng được ca tối thiểu. | R11 §5 + R12 §5 | mở |
| 3 | Gán lồng sâu hơn một tầng trường (`x.f[i][j]`) là `VS0412` có chủ đích. | R11 §5 + R12 §5 | mở |
| 4 | Thân `impl` chưa tyck đầy đủ — mới kiểm đích gán. | R11 §5 + R12 §5 | mở |
| 5 | Siêu trait bắc cầu mới kiểm một tầng. | R11 §5 + R12 §5 | mở |
| 6 | `impl_pairs` per-tệp: `impl` sinh từ `derives` ở tệp khác không thấy; dispatch xuyên tệp per-tệp. | R11 §5 + R12 §5 | mở |
| 7 | `co_float_sau` không soi bản ghi lồng (`Eq` derives). | R11 §5 + R12 §5 | mở |
| 8 | `derives Json` chưa sinh mã (chỉ kiểm tĩnh). | R11 §5 + R12 §5 | **đóng ở ST-302** — sinh `to_json`/`from_json` cho record/enum; 15 ca `vd284_derives_json.rs`; từ chối `cap`/`Chan`/`fn` bằng `VS0412` |
| 9 | `Ty::Fn` thật cho `sort_by` (đang `Ty::FnKhoa`), map khoá không-`str`, thứ tự khoá map. | R11 §5 + R12 §5 | mở |
| 10 | Nhóm nhiều pipeline + kiểu tĩnh lấy stage đầu; lambda `where` không chú thích ⇒ `{error}`. | R11 §5 + R12 §5 | mở |
| 11 | `lower/cmd.rs` 1.491 dòng — sát ngưỡng R1, cần tách trước khi thêm tính năng. (ST-106: `cmd.rs` 1491→1060) | R11 §5 + R12 §5 · R18 §6 (ST-106 tách `cmd.rs`) | đóng ở ST-106 |
| 12 | Async chưa song song thật: scheduler run-to-completion, không nhường, không GC (chu trình `Rc`); `await` lồng sát ngưỡng khung 256 | R11 §5 + R12 §5 · R23 §4 hàng 1 · VD-123 | chuyển VS3 |
| 13 | `async` trên method/`cmd` ⇒ `VS0201`; `async fn` không nhận `--tên`. | R11 §5 + R12 §5 | **đóng ở ST-302** — `async fn` trong `impl` + `async cmd`; 3 ca `vd284_async_impl.rs`; `--flag` của `cmd` giữ riêng |
| 14 | `p.x += 1` là lỗi cú pháp `VS0201` (front end chặn trước tyck). | R11 §5 + R12 §5 | mở |
| 15 | Drain chỉ task `ready` (`spawn`): task `Moi` do gọi `async fn` chưa chạy ⇒ `recv` có thể `VS0505`. | R11 §5 + R12 §5 | mở |
| 16 | `close` idempotent nhưng không đánh thức ai (không wait-set). | R11 §5 + R12 §5 · VD-123 · `d060f5a` · `JOURNAL/st296-debt-audit.md` | đóng ở VS3 (`d060f5a`, VD-282: wait-set + close đánh thức receiver `None`/sender `VS0504`, `vs3_chan0` 13/13) |
| 17 | Không `select`/timeout trên kênh. | R11 §5 + R12 §5 · R23 §4 hàng 2 · VD-123 · `8da533b` · `82064e96` (ST-293) | đóng ở ST-293 (`VD-282`: `recv_timeout`/`send_timeout` 17/17 + `select_*` 4 hàm 25/25 interp↔VM) |
| 18 | `Chan` không truyền qua biên host. | R11 §5 + R12 §5 | mở |
| 19 | `cap = 0` thực tế không dùng được (hai phía chặn đều drain rồi `VS0505`). | R11 §5 + R12 §5 · VD-123 · `d060f5a` · `JOURNAL/st296-debt-audit.md` | đóng ở VS3 (`d060f5a`, VD-282 có phạm vi: `chan(0)` rendezvous chạy trên frame/async `vs3_chan0` 13/13; runtime đồng bộ đơn luồng từ chối đúng thiết kế) |
| 20 | Dung lượng âm là lỗi lúc chạy `VS0501`; `Chan` in `<kênh>` | R11 §5 + R12 §5 | mở |
| 21 | Net chỉ loopback, host khác ⇒ Err; không TLS/keep-alive (redirect + chunked đã có ở ST-104/ST-105) — phần còn lại là VS3 | R11 §5 + R12 §5 · R23 §4 hàng 3 · VD-122 · `a5e3040` · `b7385e9` · ST-294 | đóng ở VS3/ST-294 (`VD-282`: TLS `vs3_tls` 21/21 + keep-alive `vs3_keepalive` 8/8 + HTTP/2 `vs3_http2` 33/33; chính sách loopback giữ có chủ đích) |
| 22 | Một luồng: server phục vụ tuần tự một client, không `select`/epoll; handle per-process. | R11 §5 + R12 §5 | mở |
| 23 | `net.read` 3 đối số; `std.net.connect` dạng đầy đủ chưa hạ; bỏ `is_open`; `read` trả `str` lossy; connect mặc định 1 s | R11 §5 + R12 §5 | mở |
| 24 | `vis check` chẩn đoán = exit 1; thiếu `use std.http` ⇒ `VS0501` lúc chạy (không `VS0301`); `vis check` permissive với tên cấp tệp | R11 §5 + R12 §5 | mở |
| 25 | Cap body 8 MiB, header 64 KiB; HTTP chỉ `http://`; ca net/http nằm ở `net*.rs`/`http.rs`, không trong sem | R11 §5 + R12 §5 | mở |
| 26 | LSP module-aware chỉ ở chẩn đoán (ST-80); completion/hover vẫn `phan_tich` một tệp; chẩn đoán tệp phụ không publish | R11 §5 + R12 §5 · `9efa420` (ST-84) · `JOURNAL/st296-debt-audit.md` | đóng ở ST-84 (`VD-282`: completion/hover module-aware thấy `pub` của `use ./x` + `std.*`) |
| 27 | Chỉ full sync (`didChange` toàn văn), một luồng; EOF ⇒ exit 0 (khác spec LSP — cần cho test/CI). | R11 §5 + R12 §5 · `3510232` · `JOURNAL/st296-debt-audit.md` | đóng ở VS3 (`3510232`, VD-282 có phạm vi: incremental `change:2` `lsp_incremental` 4/4; một luồng & `EOF → exit 0` là hợp đồng test/CI có chủ ý) |
| 28 | Completion chưa theo ngữ cảnh sau `.`/kiểu; hover chưa field; biến gộp mọi phạm vi. | R11 §5 + R12 §5 | mở |
| 29 | Lower/tyck vẫn chạy khi lex/parse lỗi ⇒ có thể thêm nhiễu chẩn đoán | R11 §5 + R12 §5 | mở |
| 30 | Chưa resolver dải: phụ thuộc ghim phiên bản chính xác (đã có ở ST-128); phần còn lại — ký số, registry HTTP — chuyển #118 | R11 §5 + R12 §5 · VD-125 | đóng ở ST-128 |
| 31 | Lock chưa lưu `entry` (dùng quy ước); lock chỉ tìm ở `main_dir`; `.vis/store` không GC. | R11 §5 + R12 §5 · `837d5ff` (ST-90) · `cea1643` (ST-288) · VD-282 | mở — thu hẹp theo `VD-282`: đã xong `entry` (ST-90) & `pkg gc --root` (ST-288); chỉ còn vế tìm `vis.lock` leo ngược thư mục cha ngoài `main_dir` (ưu tiên thấp GĐ 3) |
| 32 | Chưa `pkg remove`/`update` (đã có ở ST-90/ST-102); `use pkg.*` chỉ nạp tệp có trong lock; chưa kiểm toàn vẹn cây ngoài `file_sha` | R11 §5 + R12 §5 | mở |
| 33 | Siết mặc định vẫn là nợ: `--strict-caps` opt-in; muốn siết mặc định phải chuyển toàn bộ fixture/host suite sang khai `cap` | R11 §5 + R12 §5 · R23 §4 hàng 4 · VD-124 | chuyển VS3 |
| 34 | Landlock chỉ probe ở mọi nền; Linux chưa chạy thật; `vis-cli` chưa dùng crate `vis-host-linux` (đã đóng khi Landlock thật chạy 13/13 ca) | R11 §5 + R12 §5 · R23 §4 hàng 4 · R24 §4 hàng 4 | đóng ở ST-132/132b |
| 35 | Ca sem `cap/` mới kiểm tyck; giá trị runtime chỉ đến từ tham số `main` | R11 §5 + R12 §5 | mở |
| 36 | Chưa có runner Windows/wasm/Linux ⇒ mọi ô chỉ là check biên dịch chéo; `vis-cli` trên wasm không chạy; chưa đo build/link Windows (ST-131/ST-136: Linux chạy thật, Windows đã đo; wasm chỉ check theo VD-121) | R11 §5 + R12 §5 · R23 §4 hàng 7 · VD-121/VD-136 | đóng ở ST-131/ST-136 |
| 37 | Net không nằm trong suite host (24 ca riêng); suite phụ thuộc lệnh macOS (`/bin/echo`, `/bin/cat`, `/usr/bin/grep`…) | R11 §5 + R12 §5 | mở |
| 38 | `.vish` không chuẩn hoá thụt lề (ST-77): `vis fmt` giữ thụt lề gốc của khối `cmd`, không tự chuẩn hoá độ sâu; muốn chuẩn hoá phải sửa 68 fixture `.vish` | R12 §5 | mở |
| 39 | `usage_explain` chỉ tiếng Việt (ST-78): trợ giúp in “7 khối, 32 mã” đếm động nhưng không theo `--lang`; `--list` vẫn in kèm cột severity | R12 §5 | mở |
| 40 | `assert` chỉ dạng gọi hàm (ST-79): `assert(đk)`; chưa hỗ trợ `assert đk` như tài liệu; mã lỗi dùng `VS0501` “assert thất bại” (không thêm mã mới). (ST-83: đã thêm `assert đk`) | R12 §5 · R13 §5 #45 | đóng ở ST-83 |
| 41 | Runner `test` biến đổi văn bản (ST-79): mỗi `test fn` chạy bằng bản sao tạm đổi thành `fn main`; câu lệnh cấp tệp chạy lại cho mỗi test; `async fn` trong thân `test` không hỗ trợ | R12 §5 | mở |
| 42 | LSP completion/hover chưa module-aware (ST-80): chỉ chẩn đoán đi đường module; gợi ý/hover vẫn một tệp; xem thêm nợ 26. | R12 §5 | mở |
| 43 | Siết mặc định chưa làm: `--strict-caps` opt-in; `--sandbox-check`/`--dry` không áp cờ | R12 §5 · VD-124 (ST-130 siết mặc định khi dùng gói) | chuyển VS3 |
| 44 | `fs.exists` ngoài quyền trả `false` im lặng thay vì trap `VS0604` như read/write/list (ST-83: nay trap `VS0604`) | R12 §5 · R13 §5 (đầu mục) · VD-109 | đóng ở ST-83 |
| 45 | `assert` câu lệnh chỉ một biểu thức, không message (ST-83): `assert đk`; chưa `assert đk, "msg"`, chưa đếm/nhãn; mã lỗi dùng `VS0501` (không mã mới). | R13 §5 | mở |
| 46 | Biến tên `assert`: `assert` là từ khoá câu lệnh; dùng `assert` làm định danh (biến/hàm) bị nhập nhằng — chưa có luật đặt tên/chẩn đoán riêng. | R13 §5 | mở |
| 47 | Trợ giúp `explain` dịch tay 3 ngôn ngữ (ST-83): câu vi/en/ja viết trong `main.rs`; thêm mã mới phải sửa 3 chỗ; chưa có nguồn dữ liệu chung với `vis explain` | R13 §5 | mở |
| 48 | `is_pub` mới giữ cho `fn` (ST-84): HIR `HRec/HEnum/HAlias/HConst/HTrait` không giữ cờ `pub` ⇒ LSP gợi ý cả item không `pub` của module; cần thêm cờ vào HIR | R13 §5 | mở |
| 49 | Completion chưa theo ngữ cảnh sau `.`: vẫn lọc tiền tố trên toàn bộ item; sau `x.` chưa gợi ý field/method theo kiểu — phần còn lại chuyển #65 | R13 §5 · R14 §5 (đầu mục) | đóng ở ST-91 |
| 50 | Hover chưa field (ST-84): hover biến/hàm có; field bản ghi/chấm chưa. | R13 §5 | mở |
| 51 | `--indent-report` fallback ước lượng (ST-85): khi thân `cmd` chỉ là lệnh, đếm theo dòng (`cmd`/`impl`/`type` mở khối) chứ không theo CST đầy đủ | R13 §5 | mở |
| 52 | `dong_thut_sai` heuristic văn bản (ST-85b): dấu `:` cột 1 trong chuỗi/comment có thể bị coi mở khối; khối một dòng `cmd f(): x` không mở khối; nối dòng `\` ngoài ngoặc chưa hỗ trợ | R13 §5 | mở |
| 53 | `khai::tach` chưa dọn gốc (ST-85b): vẫn coi dòng thụt là tiền tố khai báo; nay có `VS0201` chặn nhưng logic tách chưa sửa tận gốc. | R13 §5 | mở |
| 54 | `topmix2` đổi hành vi (ST-85b): dòng cấp tệp thứ hai thụt lề trước đây chạy như lệnh ngoài, nay báo `VS0201` (đúng §9.5 nhưng đổi hành vi với tệp sai thiết kế) | R13 §5 | mở |
| 55 | Bộ sinh fuzz mẫu cố định (ST-86): 8 nhóm là mẫu cố định, chưa ghép tổ hợp sâu; kênh chỉ cặp cân bằng; chưa sinh `use pkg.*` thành công; `@`/`(write)` mới ở mức cú pháp | R13 §5 | mở |
| 56 | 7 ca `.vish` khác hai đường có chủ đích: lệnh ngoài chạy ở interp nhưng VM báo `VS0604` (VM chưa có host `Ngoai`) (ST-89: VM có host `Ngoai` ⇒ 0 SKIP) | R13 §5 · R14 §5 (đầu mục) | đóng ở ST-89 |
| 57 | Frontend chạy mỗi lần (ST-87b): mọi lần `--vm` vẫn parse/lower/check (~1 ms tệp nhỏ) rồi mới tra cache codegen; không còn đường bỏ frontend | R13 §5 | mở |
| 58 | Ca chống nhánh chết đọc mã nguồn (ST-87b): `khong_con_fast_path_khoa_v1` grep `main.rs`/`cache.rs`; brittle nếu tách hàm/đổi tên. | R13 §5 | mở |
| 59 | `--vm` vẫn một luồng (ST-89): host `Ngoai` không đổi scheduler; lệnh ngoài chạy tuần tự, không có song song/async cho process. | R14 §5 | mở |
| 60 | Lệnh ngoài chưa qua `cap` (ST-89/ST-90): `exec.*`/`Ngoai` chạy không cần khai quyền; chưa có `cap[Exec]` — phần còn lại chuyển #68 | R14 §5 · R15 §5 (đầu mục) | đóng ở ST-93 |
| 61 | `vis test` dùng `--no-cache` (ST-89): mọi ca test chạy `--vm --no-cache` để tránh cache ảnh hưởng; chưa có chế độ test cache ấm. | R14 §5 | mở |
| 62 | Lock cũ không tự nâng cấp (ST-90): lock thiếu `entry` chỉ fallback; chỉ khi `add`/`remove` ghi lại mới có `entry`; không có `pkg migrate`/chuẩn hoá lock (ST-129: lock định dạng 2 + migration theo VD-125) | R14 §5 · VD-125 · R24 §4 hàng 6 | đóng ở ST-129 |
| 63 | `entry` chưa kiểm đuôi (ST-90): chỉ kiểm đường dẫn tương đối + có trong `files`; chưa bắt buộc `.vis`/tồn tại đúng kiểu tệp. | R14 §5 | mở |
| 64 | `remove` không dọn dependency mồ côi (ST-90): xoá `http` để lại `util` trong lock/store (đúng “chỉ xoá tên được gọi”); chưa có `pkg remove --prune`/GC tham chiếu | R14 §5 | mở |
| 65 | LSP chỉ receiver đơn giản: `a.b.`, `(x).`, `f().`, generic/`list[P]`/`Opt[P]`, `impl` cho alias/enum chưa mở thành viên; chưa có bảng phương thức `std.str` | R14 §5 | mở |
| 66 | `entry`/thành viên chưa cache (ST-91): mỗi completion sau `.` parse/lower lại với sentinel `__vis` (~1 lần phân tích); chưa cache theo phiên/tệp. | R14 §5 | mở |
| 67 | `self` in gọn không kiểu (ST-91): chữ ký method hiển thị `fn show(self) -> str` (bỏ `: {error}`); chưa suy kiểu `self` thật cho hover/detail. | R14 §5 | mở |
| 68 | `cap[Proc]` không allowlist: quyền chạy mọi tiến trình, không giới hạn danh sách/đường dẫn; chưa áp cho tiến trình con (ST-97: `--grant p=<lệnh>` = allowlist) | R15 §5 · QUEUE ST-97 | đóng ở ST-97 |
| 69 | `.vish` có cả pipeline cấp tệp và `fn main` ⇒ HIR bỏ `main` để pipeline chạy; `main` chỉ khai `cap` (đổi ngữ nghĩa) | R15 §5 | mở |
| 70 | `pass` nhận diện theo tên (ST-93b): thân `main` chỉ có `HStmt::Expr(Var("pass"))` mới được coi là rỗng; chưa có kiểm “khai báo kiểu trong thân” | R15 §5 | mở |
| 71 | Không rendezvous: `chan(0)` bị từ chối runtime `VS0505` nhưng exit 1, không phải 2; chưa có wait-set | R15 §5 · R23 §4 hàng 2 · VD-123 | chuyển VS3 |
| 72 | Drain không wait-set: join cuối chạy task `Moi` theo hàng đợi, không có cơ chế chờ `recv`/`send` đúng nghĩa | R15 §5 · VD-123 | chuyển VS3 |
| 73 | `--grant` không áp cho `Proc` (ST-93): `--grant tên=path` chỉ có nghĩa Dir/File/Net; `cap[Proc]` không có tham số đường dẫn nên không nhận grant. (ST-97: `cap[Proc]` đã nhận `--grant p=`) | R15 §5 · R16 §6 (ST-97) | đóng ở ST-97 |
| 74 | Tiến trình con không sandbox (ST-93): `cap[Proc]` cho phép spawn, nhưng tiến trình con không bị Landlock/seccomp/namespace; mới chỉ kiểm ở biên host | R15 §5 | mở |
| 75 | Quét sổ mã chỉ `crates/*/src/**`: không quét test/script/`build.rs`; `DU_PHONG` rỗng; chưa đối chiếu mã trong tài liệu/spec | R16 §5 | mở |
| 76 | Allowlist `cap[Proc]` còn rộng: khớp basename (ST-123 đã đổi sang canonical), không giới hạn đối số, không hỗ trợ dạng `:` | R16 §5 | mở |
| 77 | Soak chưa đủ 30 phút + phụ thuộc PATH: 8 vòng 818 s; script giả định `cargo` ở `~/.cargo/bin`; chưa soak Linux/Windows | R16 §5 | mở |
| 78 | JSON thiếu `severity` (ST-101): `check --json` chỉ `code/line/col/message`; chưa phân biệt warning/error; chưa có `source`/`range` UTF-16 như LSP. (ST-110: đã có `severity`) | R17 §5 · R19 §4 (ST-110) | đóng ở ST-110 |
| 79 | Chẩn đoán tệp phụ dưới `file` chính (ST-101): `files[].diagnostics` gộp theo tệp đối số, không có trường `phu`/`file` gốc của chẩn đoán (ST-110: `files` theo tệp nạp) | R17 §5 · R19 §4 (ST-110) | đóng ở ST-110 |
| 80 | Cắt message 200 ký tự (ST-101): `test --json` cắt theo ký tự (không byte), thêm “…” | R17 §5 | mở |
| 81 | Cache LSP global (`thread_local`) + tệp dùng `pkg` luôn nạp lại; `didClose` xoá toàn bộ cache; hover chỉ khi tên duy nhất | R17 §5 | mở |
| 82 | `pkg update` chưa đủ resolver/blob: chỉ ghi gói đích, không cập nhật phụ thuộc; blob cũ không GC; registry cục bộ | R17 §5 | mở |
| 83 | Chưa TLS/HTTP2/keep-alive: `http.get` chỉ `http://`, một request/kết nối, không nén. | R18 §5 · R23 §4 hàng 3 · VD-122 | chuyển VS3 |
| 84 | Redirect chỉ GET/không cookie-auth: 303 không đổi POST, không giữ `Cookie`/`Authorization`; không giới hạn host đích ngoài loopback (vẫn `tach_url` chặn) | R18 §5 · `096ea2c` · `JOURNAL/st296-debt-audit.md` | đóng ở VS3 (`096ea2c`, VD-282: cookie jar + same-origin auth `vs3_cookie_auth` 11/11; loopback giữ có chủ đích) |
| 85 | Chunk trailer bị bỏ qua: dừng ở chunk `0`, không đọc/không kiểm trailer. | R18 §5 | mở |
| 86 | `Location` tương đối chưa resolve `..`/query-only: chỉ ghép thư mục hiện tại; `../x`/`?q` chưa chuẩn. (ST-109: `Location` đủ `..`/query-only/fragment) | R18 §5 · R19 §6 (ST-109) | đóng ở ST-109 |
| 87 | `--timeout` chỉ `test fn`: fixture `.out/.err` không có timeout; không timeout theo ca/bộ | R18 §5 | mở |
| 88 | `--timeout` không kill cây cháu: chỉ kill `vis run`; cháu (nếu có) có thể sống. | R18 §5 | mở |
| 89 | Pipe stdout/stderr có thể nghẽn: `wait_with_output` sau khi con kết thúc; ca in quá buffer có thể treo. (ST-139: 3/3 unit + 7/7 tích hợp xanh) | R18 §5 · QUEUE ST-139 · PR #24 · `ef6489d1` · JOURNAL/st139-do-truoc.log | đóng ở ST-139 |
| 90 | `lower.rs` 1482 vẫn max R1: ST-106 chỉ tách `cmd.rs`; muốn max ≤1200 phải tách `lower.rs`. (ST-108: `lower.rs` 1482→993) | R18 §5 · R19 §6 (ST-108) | đóng ở ST-108 |
| 91 | `ha_stage` 782 vẫn lớn: chưa tách tiếp; `cmd.rs` còn 1060. | R18 §5 | mở |
| 92 | Span HIR chưa có file-id (correctness): mỗi tệp hạ với span bắt đầu 0 ⇒ câu lệnh cấp tệp chính có thể bị quy sang module (ST-114a/b: `Span.file`) | R19 §5 · R21 §6 · R22 §1 | đóng ở ST-114a/b |
| 93 | `HEAD` không theo redirect: trả 302 + `Location`, không đọc thân (đã chọn, ghi tài liệu); chưa `HEAD` keep-alive/HTTP2. | R19 §5 | mở |
| 94 | `FUNCS` 52 cần review: thêm `http.head`/`head_timeout` và `Ty::ResultHead` (`map[str, {error}]`); test đếm 50→52 đã sửa kèm lý do | R19 §5 | mở |
| 95 | `cmd.rs` 1060 vẫn max R1: dưới 1.200 nhưng `ha_stage` 782 dòng chưa tách; tách tiếp nếu muốn <900. | R19 §5 | mở |
| 96 | `check --json` nhiều đối số có thể lặp module; `files` chưa có metadata `main`/`phu`; TAP vẫn báo chẩn đoán đầu | R19 §5 | mở |
| 97 | Span tổng hợp `file=0`: vài node do lower sinh dùng `Span::new` mặc định ⇒ chẩn đoán có thể về tệp chính; chưa audit | R21 §5 | mở |
| 98 | LSP chưa publish chẩn đoán tệp phụ theo file-id — hành vi LSP không đổi; mới có module-aware. | R21 §5 · `1aefb88` · VD-126 · `JOURNAL/st296-debt-audit.md` | đóng ở VS3 (`1aefb88`, VD-282 có phạm vi: publish chẩn đoán tài liệu đang mở + overlay `lsp_da_tep` 8/8; tệp chưa mở giữ ngoài phạm vi theo `VD-126`/`#104`) |
| 99 | Bytecode `Span` không mang file — `VBC1` giữ 53; nếu cần debug đa tệp trong VM phải bump format. | R21 §5 | mở |
| 100 | `parse/mod.rs` 1452 dòng gần trần R1 1500; thêm chức năng parse đa tệp dễ vượt — cần tách trước khi mở rộng | R21 §5 | mở |
| 101 | `lower/cmd.rs` 1060 — dưới luật R1 1500 nhưng trên mục tiêu nội bộ ≤1000; `parse/cmd.rs` 1007 cũng vậy; cần task tách riêng nếu muốn ≤1000 toàn cục. | R22 §5 | mở |
| 102 | `main.rs` 591 dòng nhưng `run_run` vẫn 352 dòng trong main (giữ `cache::khoa_multi` cho ca `cache_fastpath`) | R22 §5 | mở |
| 103 | Span tổng hợp `file=0` — node do lower sinh vẫn mặc định tệp chính; chưa audit hết. | R22 §5 | mở |
| 104 | LSP chưa publish chẩn đoán tệp phụ cho tài liệu chính; mới bảo đảm tài liệu đang mở đúng tệp. | R22 §5 · VD-126 | không làm có lý do |
| 105 | `HSym` của builtin vẫn `Span::new(0,0)` — bảng ký hiệu dùng chung, builtin không thuộc tệp nào; cần chốt `Option<Span>`/`u16::MAX` | R23 §6 · ST-122 | mở |
| 106 | `RecDef`/`FieldDef` của prelude + `cmdsig` vẫn `file=0`; audit chưa phủ `TypeTable` | R23 §6 · ST-122 | mở |
| 107 | `test/sem` không chứa được ca đa tệp (5 harness một tầng + `vistest::gom` đệ quy) | R23 §6 · ST-122 | mở |
| 108 | `Command::new(tên)` để OS phân giải `PATH` lần nữa sau khi kiểm | R23 §6 · ST-123 | mở |
| 109a | `next_block()` trả `801` trong khi khối `07xx` vẫn còn chỗ (`704` đã cấp): hàm là “khối mới”, không phải “số kế tiếp” | JOURNAL/st128-resolver.md §8 · ST-128 · QUEUE ST-128 · R23 §6 #116 | chuyển VS3 |
| 109b | Đường GHI vẫn kiểm-rồi-mở (`FsGuard::write`): đổi symlink giữa `kiem_fs` và `std::fs::write` có thể truncate/ghi ngoài root; sửa cần `O_NOFOLLOW`/`openat` = VS3; **dòng giới hạn đã có trong `HOST-BAO-DAM.md`**; sửa kỹ thuật thuộc VS3 | R23 §6 · ST-124 · R24 §5 #140 · **ST-295** (`ba4023fb`/`cd76da56`) · `JOURNAL/st295-baove.md` | đóng ở ST-295 (`VD-282`: `rustix =1.1.6` `openat2`/`O_NOFOLLOW` chung đọc+ghi trong `quyen_mo.rs`, race ghi tất định **100/100 → 0/100**, đã gỡ cảnh báo ghi ở `HOST-BAO-DAM.md`) |
| 110 | VM không đưa `argv` vào tham số không-`cap` của `main` (interp `list[str]`, VM `()`) — **ST-157**: VM chạy thẳng `main` (bỏ qua "vỏ" `Call(main, 0); Halt` của codegen) và đặt `argv` vào mọi tham số **không-`cap`**, `Value::Cap` đúng chỉ số (ST-72b); ca mới `vm_main_argv.rs` 4 ca **so byte** hai đường | R23 §6 · ST-124 · ST-157 · `JOURNAL/st157-do-truoc.log` · QUEUE ST-157 | đóng ở ST-157 |
| 111 | tyck chấp nhận `let e: cap[Dir] = json.decode(...)` | R23 §6 · ST-124 · **ST-289** | **đóng ở ST-289** (khoá bằng ca đỏ-trước có sẵn `crates/vis-cli/tests/cap_khong_lo.rs` ca (1b) `gan_json_vao_bien_cap_khong_sinh_quyen`: `json.decode` trả `Result[Json, Error]` ⇒ gán vào `cap[Dir]` là lỗi tĩnh **VS0412** exit 2, `--strict-caps`, stdout rỗng — không có đường cấp quyền từ JSON; `cargo test -p vis-cli --test cap_khong_lo` xanh) |
| 112 | Thực tế có 2 bộ escape JSON (`json_esc` + `json_tai`); VD-130e “nguồn duy nhất” chưa đúng | R23 §6 · ST-124 | mở |
| 113 | `TEN_THE` không ràng buộc lúc biên dịch | R23 §6 · ST-126 · JOURNAL/st126-dong-bang-api.md §8 | chuyển VS3 |
| 114 | hash nguồn VBC1 ≠ khoá cache (hai giá trị khác nhau, không đối chiếu ở đâu) | R23 §6 · ST-126 · JOURNAL/st126-dong-bang-api.md §8 | chuyển VS3 |
| 115 | quy ước "điền 0" khi so sánh thiếu thành phần phiên bản | R23 §6 · ST-128 · JOURNAL/st127-semver.md §8 | chuyển VS3 |
| 116 | `next_block()` trả 801 trong khi khối 07xx còn chỗ ⇒ cấp số mã mới phải chọn tay | R23 §6 · ST-128 · JOURNAL/st128-resolver.md §8 | chuyển VS3 |
| 117 | `@1.2.3` = ghim, khác "không toán tử ⇒ `^`" | R23 §6 · ST-128 | mở |
| 118 | yanked, registry mạng, chữ ký (VS3) | R23 §6 · ST-128/129 · JOURNAL/st128-resolver.md §8 | chuyển VS3 |
| 119 | lock không ghi quan hệ phụ thuộc giữa gói | R23 §6 · ST-129 · JOURNAL/st129-lock.md §9 | chuyển VS3 |
| 120 | `vis test`/host suite chưa áp strict-caps | R23 §6 · ST-130 · VD-133 | chuyển VS3 |
| 121 | `pipeline.rs` bám tiền tố thông điệp grep (lệch 1 §4.4) (ST-134: `pipeline` 16/16 hai nền) | R23 §6 · ST-131 · R23 §4.4 · QUEUE ST-134 | đóng ở ST-134 |
| 122 | `sandbox.rs:59` chưa `cfg` theo nền (lệch 3) (ST-134: `sandbox` 6/6 hai nền) | R23 §6 · ST-131 · R23 §4.4 · QUEUE ST-134 | đóng ở ST-134 |
| 123 | runner thiếu `wasm32` target (lệch 5) (ST-134: `setup.sh` cài `wasm32`, targets 18/18) | R23 §6 · ST-131 · R23 §4.4 · QUEUE ST-134 | đóng ở ST-134 |
| 124 | `vish_cmd_body.rs:312` đường dẫn thư mục tạm (lệch 6) (ST-134: `vish_cmd_body` 16/16 hai nền) | R23 §6 · ST-131 · R23 §4.4 · QUEUE ST-134 | đóng ở ST-134 |
| 125 | `--sandbox` chưa cấp `EXECUTE` cho `/lib*` ⇒ lệnh ngoài không chạy được dưới sandbox | R23 §6 · ST-132 · **ST-295** (`ba4023fb`/`cd76da56`) · `JOURNAL/st295-baove.md` | đóng ở ST-295 (`VD-282`: Landlock ruleset xử lý `Execute`, cấp chỉ-đọc+`Execute` cho `/lib*` & binary canonicalize trong grant; `vis-host-linux` 18/18, `st295_sandbox_proc` 3/3) |
| 126 | `unicode-ident` (`(MIT OR Apache-2.0) AND Unicode-3.0`, build-time) ngoài allowlist hẹp — cần Agent 0 quyết (VD-135: hợp lệ theo QD §9.3) | R23 §6 · ST-132 · VD-135 · QUEUE đính chính 2026-11-16 | đóng ở ST-132 (VD-135) |
| 127 | `ap_dung` mới dùng `ABI::V1`; ABI 8 có thêm `REFER`/`TRUNCATE`/`IOCTL_DEV` chưa xét | R23 §6 · ST-132 · JOURNAL/st132-landlock.md §7 | chuyển VS3 |
| 128 | RÒ RỈ TOCTOU trên Linux 798/50 000 — lỗi sản phẩm, ưu tiên cao nhất đợt 24 (ST-137: 0/50 000 trên `ps` và macOS) | R23 §6 · ST-133 · VD-141 · QUEUE ST-137 | đóng ở ST-137 |
| 129 | Cùng danh sách số dòng khối tài liệu bị chép ở 4 tệp test (`check.rs`, `fmt.rs`, `doc_fixtures.rs`, `hir.rs`) — **ST-158**: gom về **một nguồn** `viscript/test/doc_excluded.rs` (`const DOC_EXCLUDED: [usize; 7]`, không đổi số); 4 tệp test dùng `include!("../../../test/doc_excluded.rs")`; vis-syntax dùng bản **6** khối dẫn xuất `DOC_EXCLUDED_SYNTAX` (khối 666 parse sạch từ ST-27); 4 target trước/sau giống: 13/13/3/10 | R23 §6 · ST-133 · ST-158 · `JOURNAL/st158-truoc.log` · `JOURNAL/st158-sau.log` · `JOURNAL/st158-gate-macos.log` · QUEUE ST-158 | đóng ở ST-158 |
| 130 | Linux chưa chạy `vis test`/differential/cache (mới có `cargo test` + host suite) (ST-142: 447/447 hai đường, `lech=0`) | R23 §6 · ST-133 · QUEUE ST-142 · JOURNAL/st142-ps-vis-test.log · JOURNAL/st142-ps-diff-100k.log · JOURNAL/st142-ps-cache.log | đóng ở ST-142 |
| 131 | hàng `ST-132b` do Agent 2 tự thêm vào QUEUE (ngoài ủy quyền) — cần Agent 0 nhận hoặc xoá (đính chính: hàng do Agent 0 mở, không phải Agent 2 tự thêm) | R23 §6 · ST-133 · R24 §1 hàng 5 | đóng ở ST-00 (đính chính R23/R24 §1) |
| 132 | 3 ca âm dựng helper bằng `rustc`; thiếu `crt-static` ⇒ đỏ vì môi trường — **đóng ở ST-166**: đo được **6 ca** dùng chung `dung_helper` đỏ khi PATH thiếu `rustc`/`cc` (`linker 'cc' not found`); sửa **chỉ phần dựng helper** (`duong_rustc()`, `path_dung_helper()`, `+crt-static` **chỉ Linux**) ⇒ trên `ps` **15/15** ở cả 3 môi trường (chuẩn, đã-từng-đỏ, PATH rỗng); mac: phần dựng helper xanh (`rustc -O` exit 0), **suite landlock vẫn không biên dịch trên mac** vì `landlock 0.4.7` Linux-only ⇒ **câu hỏi riêng cho A0** (có `cfg`-hoá dependency trong `Cargo.toml` không); `vis-targets` 18/18 — **A0 XÁC NHẬN đóng (VD-175)** | R24 §5 · ST-132b · ST-166 · `JOURNAL/st166-*.log` | **đóng ở ST-166** |
| 133 | Ca `read_dir` mới kiểm luật chỉ-đọc, chưa kiểm luật ghi — **ST-150 đã bù**: ca `con_ghi_vao_thu_muc_chi_doc_bi_kernel_chan` (ca 14) cho tiến trình con **ghi** vào thư mục chỉ-đọc ⇒ `EACCES`, kèm đỏ-trước + đối chứng dương cùng ca + hai luật trong **một** ruleset. Biến thể `read_dir` trên thư mục **có quyền ghi** (ST-132b §7) **không** thuộc phạm vi hàng ST-150 — đề nghị cấp số ở đợt sau, chưa đưa vào bảng này | R24 §5 · ST-132b · QUEUE ST-150 · `JOURNAL/st150-landlock-ps.log` · `JOURNAL/st150-gate-macos.log` | đóng ở ST-150 |
| 134 | `setup.sh` cài thêm `x86_64-pc-windows-msvc` (ngoài `wasm32`) để đạt 18/18 — cần Agent 0 chuẩn thuận (VD-146(1) + ST-144: 2/2 target đã cài) | R24 §5 · ST-134 · VD-146(1) · QUEUE ST-144 | đóng ở ST-144 |
| 135 | `vis-linux` phải sync cả `vis-host-wasm/`; chưa có kiểm tự động danh sách sync — **ST-151 đã kiểm**: danh sách là **một nguồn** (`SYNC_DIRS`/`SYNC_FILES` ở đầu tệp, `sync_tree()` chỉ đọc hai biến) + **`--check-sync`** (8/8 mục, exit **0** đủ · **2** thiếu, không ssh/rsync) + **9 ca** `crates/vis-cli/tests/sync_list.rs` (bỏ một mục ⇒ đỏ) | R24 §5 · ST-134 · QUEUE ST-151 · `JOURNAL/st151-do-truoc.log` · `JOURNAL/st151-gate-macos-lan2.log` | đóng ở ST-151 |
| 136 | Phân loại thân `cmd` theo tên lệnh: biến trùng tên lệnh muốn làm biểu thức bị hiểu là lệnh (đường thoát `( … )`), chưa có ca khoá — **ST-149 đã sửa**: biến bind từ ngoài (`let`/`var`/`const` cấp tệp + tham số `cmd`/`fn`) **che** tên lệnh ⇒ thân là **biểu thức**; không bind ⇒ luật ST-134 nguyên vẹn | R24 §5 · ST-134 · QUEUE ST-149 · `JOURNAL/st149-do-truoc.log` · `JOURNAL/st149-tai-hien.log` | đóng ở ST-149 |
| 137 | macOS vẫn dùng bước xác nhận hai-syscall của ST-124 (không có `/proc`, std an toàn không có fd→path) — đo 0/50 000 nhưng không bảo đảm theo cơ chế | R24 §5 · ST-137 · VD-145 · **ST-300m1** | **đóng ở ST-300m1** — macOS dùng cùng đường `openat(O_NOFOLLOW)` (`quyen_mo.rs`, 0 `unsafe`); errno so bằng `rustix::io::Errno` (`LOOP` Darwin = 62, `XDEV`) nên symlink thoát gốc trả `VS0604` `[quyền]`; `quyen_st174` + `quyen_st295` trên APFS `mps` PASS, rò **0/100**; không cần `/proc` |
| 138 | `/proc` phải đọc được trên Linux, nếu không ⇒ `VS0604` (hành vi đúng, nhưng người vận hành phải biết) — **đóng ở ST-165**: đo trên `ps` thấy `/proc/self/{status,cmdline}` **đọc được**, không `VS0604`; cái sai là **chẩn đoán** (không nêu đích `/dev/null`, không phân biệt `pipe:/socket:`); sửa `quyen.rs` (`kiem_canon` nêu đích phân giải + nhánh phi-đường-dẫn; `xac_nhan_handle` thêm hướng dẫn procfs) — **Agent 1 ghi nhận, CHỜ A0 XÁC NHẬN** | R24 §5 · ST-137 · ST-165 · `JOURNAL/st165-proc-sau.log`, `st165-proc-khong-doc-duoc.log` | **đóng ở ST-165** |
| 139 | Ca `symlink_canonicalize_hai_phia` đỏ trên Linux vì coreutils multicall — cần VD thứ 3 theo VD-132 hoặc cài `echo` thật trên runner (VD-144 + ST-140: ca xanh hai nền) | R24 §5 · ST-135 · VD-144 · QUEUE ST-140 · JOURNAL/st140-red-ps.log · JOURNAL/st140-cargo-test-ps.log | đóng ở ST-140 |
| 140 | Nợ cũ #109b (đường ghi vẫn kiểm-rồi-mở) vẫn mở; cần một dòng giới hạn trong `HOST-BAO-DAM.md` (Agent 0) — dòng đó đã có từ commit 69112900; **sửa kỹ thuật thuộc VS3** | R24 §5 · HOST-BAO-DAM.md mục FsGuard · **ST-295** (`ba4023fb`/`cd76da56`) | đóng ở ST-295 (`VD-282`: đóng cùng `#109b`, đường ghi dùng `openat2`/`O_NOFOLLOW`, race 0/100, đã gỡ cảnh báo ở `HOST-BAO-DAM.md`) |
| 141 | Mục điều hướng: nợ cũ #121..#124, #128, #129, #130 đã xử lý hoặc chuyển VS3 — xem R23 §6 | R24 §5 | không phải nợ (điều hướng) |
| 142 | **`pkg update <tên>` ghim mọi gói khác** (`=<version>`): gói được update cần nâng dep chung thì resolver báo `VS0704` thay vì nâng; cần quyết định chính sách (trước là nhãn tạm `#113b`) — **đóng ở ST-288**: thay ghim cứng bằng **soft pin** trong resolver (bản đang có được thử trước, không chặn nâng khi manifest của gói được update đòi hỏi); gói không liên quan vẫn giữ phiên bản; ca `update_mot_ten_nang_duoc_dep_chung` + unit `soft_pin_giu_ban_cu_nhung_khong_chan_nang` | JOURNAL/st129-lock.md §9 · QUEUE ST-129 · VD-149 · **ST-288** · `JOURNAL/st288-goi.md` | đóng ở ST-288 |
| 143 | **`NamePolicy::Strict` bật cho mọi tệp có `use`** (`modules.rs:606`), không chỉ tệp trong cây module; từng che lỗi VD-124 (trước là nhãn tạm `#114b`) | JOURNAL/st130-siet-caps.md §9 · QUEUE ST-130 · VD-133 · VD-149 · **VD-154(b)** | chuyển VS3 — VD-154(b): chuyển VS3 vì ca st130b xanh |
| 144 | **Vế 2 của `#133`**: chưa có ca **dương** `read_dir` trên thư mục **CÓ quyền ghi** ở tiến trình con (ST-132b §7) — **ST-154 đã bù**: ca 15 `con_read_dir_thu_muc_co_quyen_ghi_van_liet_ke_duoc` trong `vis-host-linux/tests/landlock.rs` (**MỘT** ruleset **hai** luật: `trong` chỉ-đọc + `trong-ghi` ghi; `co.txt` ghi **trước** khi áp ruleset; helper `ls trong-ghi` ⇒ `exit 0` + danh sách có `co.txt`; helper `ghi` trong `trong-ghi` ⇒ `exit 0` rồi `ls` thấy `co.txt` + `moi.txt`; âm cùng ca: `ls ngoai` ⇒ `exit 7` + `Permission denied`) | JOURNAL/st154-landlock-ps.log · QUEUE ST-154 · VD-153 | đóng ở ST-154 |
| 145 | **Chú thích `rt.rs` nói sai đường chặn `chan(0)`**: `rt.rs:128-129` nói literal `chan(0)` bị **tyck** chặn (exit 2); thực tế tyck **không** kiểm `cap` (`tyck/expr/kenh.rs:16-33` — `kiem_chan_new` chỉ kiểm `n` là `int`), **runtime** mới chặn (exit 1, `kenh_cap0.rs:57-68`); sửa chú thích hoặc thêm kiểm tyck | JOURNAL/st155-vs3-dieu-tra.md §2 · VD-154(c) | mở (xử khi có task VS3 kênh) |
| 147 | **mac chậm hơn ps** — đo lại có khoá (ST-176, máy rảnh): mac **1489 s** (số chuẩn, `lock=yes`, load 1,32→1,81) vs ps **261 s** (clone sạch, `lock=yes`) = **5,70×** > 3×. Nguyên nhân: **tạo tiến trình + syscall macOS**, không phải CPU: `check` **15,5×** (190,0/12,3 s) chiếm 59 % gate mac; `report-check.py` **22,8×** (111,9/4,9 s — gọi `git` từng báo cáo); `vis-dev rules` **8,3×** (49,3/6,0 s — một `wc`/`grep` mỗi tệp); chi phí tạo tiến trình trần **9,6×** (15,4/1,6 ms); ca `#146` 50 000 vòng **20,8×** (8,51/0,41 s). 10 suite chênh lớn nhất = 77 % tổng chênh (`cache_khong_che` +23,1 s, `vis` +10,1, `diag_cmp` +10,0, `host_conformance` +7,2 …) | ST-167 · VD-177 · ST-176 · **VD-182** · ST-204 · **VD-197** | **đóng ở ST-300m2** — đo độc quyền `mps` arm64 (load 1,80→7,67; `vis-lock`) vs `ps` (1,00→3,54), cùng HEAD `c3b7290`: soak 10 vòng **379,7 s vs 517,1 s = 0,73×**; `gate --fast` **103,6 vs 112,0 s = 0,93×**; test nextest **82,3 vs 71,0 s = 1,16×**; compile **22,5 vs 57,0 s = 0,39×**; mọi phép mac ≥10 s đều <3× (soak 380 s vẫn <3× ngay với ps lịch sử 261 s ⇒ 1,46×). `#147` đóng; bảng thành phần `JOURNAL/st300m2-mac-bench.md` §2. | **+ VD-233-frame (19/11 ~09:45, HEAD `2ee8c1e`, máy ps, 5 vòng lấy min/median)**: `./vis-dev rules` (report-check) min **0,192 s** / median **0,206 s**; `python3 test/report/check_report.py --all` min **0,129 s** / median **0,131 s** — nguồn: máy này lúc ghi; có thể tái lập; phiên cạnh VD-220 (`test/vd220-147.sh` lịch sử dùng máy ngoài).
| 146 | **Ca đua `dua_doi_symlink_lap_co_gioi_han` đỏ 1/50 000** (R28 §5 #1) — **đóng ở ST-174 (VD-176, phương án A)**: TOCTOU thật chỉ macOS (`std::fs::metadata` đi theo symlink sau `canonicalize`); vá **1 dòng** `symlink_metadata` (`quyen.rs:513`) + **điểm chèn `#[cfg(all(test, unix))]`** giữa `canonicalize` và bước kiểm; **chứng minh tất định**: ca A đỏ **100/100** trên mã cũ (`ro_ri=100`), sau vá **100/100 xanh** trên **mac và ps**, ca B đối chứng `TRONG` 100/100, **ca âm** bỏ điểm chèn ⇒ “không chạy được điểm chèn”; **6 000 000 vòng** mac `ro_ri=0`; gate mac **3 lần liên tiếp PASS** L0 **1184/0**; `cfg(test)` không lọt build (`strings`/`nm` = 0); giới hạn **hardlink** ghi `HOST-BAO-DAM.md` | R28 §5 #1 · VD-158 · VD-176 · ST-163 · ST-174 · `JOURNAL/st174-*.log`, `st174-ghi-chu.md` | **đóng ở ST-174** |

| 148 | **HTTP chưa có hạn chót TỔNG cho một request** — `timeout_read` chỉ là **giữa hai lần đọc**, nên server nhỏ giọt (slow-drip) giữ request mở vô hạn. VD-183(6) yêu cầu **mở nợ mới** thay vì thêm tham số "vô hạn/tắt" | VD-183(6) · VD-178 QĐ-3 | **đóng ở ST-265** (VD-250) — một deadline monotonic cho **toàn lời gọi** (connect/redirect/đọc), mỗi bước chờ dùng thời gian còn lại, không reset theo hop; giữ clamp 2 s/đọc; supersede phần timeout của VD-183; ca slow-drip + redirect đỏ-trước (1,01 s) → xanh (~300 ms); `JOURNAL/vd250-http148.md` |
| 149 | **Tách đường dẫn thủ công còn rải rác** — VD-184(3): ST-211 (VD-225) vá `check.rs` (`root` dùng `ten_goc` tách `/` và `\`) + ca khoá `ten_goc_du`; **mọi chỗ khác** liệt kê ở `agent/PATH-SPLIT-AUDIT-2026-11-18.md` (KHÔNG sửa) | VD-184(3) · VD-146 · ST-153 · ST-211 | **đóng ở ST-297** — `f4cb136` (nhánh `4cad7a0`): 4 ca mới (UNC/mixed/percent + `--grant` Windows), 3× full Windows **3611/0/3**, Linux 10× `vis-lsp` **29/29** + `path_impl` **2/2**; chờ W4 xác nhận Windows chỉ để hoàn tất sổ |
| 150 | **psvn `audit_span` đỏ** — gate psvn dừng ở **L0** (9/10) vì ca `audit_span`; VD-192: thành **nợ mới** kèm log **clone sạch**, **không** chặn mac/Linux | VD-192 · VD-188 · R29 §psvn | **mở** — cần log clone sạch + phân loại (mã hay nền); không chặn đóng pha 1 |
| 151 | **Tệp kiểm thử tài liệu đặt tên theo SỐ DÒNG** — `test/doc-examples/doc-<tên>-<dòng>.vis`; chèn một đoạn văn là phải sinh lại cả loạt, và rất dễ **che giấu một thay đổi kỳ vọng thật** giữa đám tệp đổi tên. Hướng sửa: đặt tên theo **định danh ổn định** (slug tiêu đề mục + số thứ tự khối trong mục, hoặc nhãn tường minh trong khối) | VD-198(2) · VD-186 · ST-213 · **ST-289** | **đóng ở ST-289** — tên fixture nay `doc-<slug-tài-liệu>-<slug-tiêu-đề>-<số-thứ-tự-trong-mục>.<ext>` (`test/doc-examples/doc_examples.py`, hàm `slugify` + `md_blocks`); 24 tệp đổi tên **thuần** (0 đổi byte nội dung), `--regen` tái lập; allowlist vẫn theo `tệp:dòng` (ngoài phạm vi #151) |
| 152 | **Enum-hoá `Option`/`Result` thành enum std làm chậm hơn ngưỡng 10 %** (VD-204 (5)): benchmark vi mô `opt-bench` (100 000 vòng; ps; số tham khảo): VM **0,17 → 0,21 s**, interp **0,31 → 0,35 s**. **VD-217 re-đo A/B xen kẽ** (2026-11-18, 24 cặp A,B, máy có tải, **tham khảo**): A = **7c256df** · B = **HEAD b12c880** **VD-220 làn B re-đo** (2026-11-18 ~20:00, **load 1,27 — không phải máy rảnh**, **tham khảo**): A = **7c256df** · B = **`a9c38ad`** — VM **175,2 → 197,9 ms** (B/A **1,129**, mediana cặp 1,129, khoảng **1,062..1,167**) · interp **312,4 → 346,5 ms** (B/A **1,109**, mediana cặp 1,108, khoảng **1,085..1,206**); **CẢ HAI "CHƯA RÕ"** (khoảng VM chớm quá 1,10 một chút, min interp 1,085 < 1,10) ⇒ **giữ "chờ đo chuẩn"**, chờ khung **07:30–07:50** (B4 script `test/vd220-147.sh` sau khi merge — đo cùng HEAD mac+ps, máy rảnh) — VM **174,9 → 201,8 ms** (B/A **= 1,154**, mediana cặp 1,152, khoảng **0,975..1,442**) · interp **311,3 → 355,8 ms** (B/A **= 1,143**, mediana cặp 1,139, khoảng **1,095..1,200**). **CẢ HAI CHƯA RÕ** (VM khoảng cắt ngang 1,10; interp min 1,095 < 1,10) ⇒ **giữ "chờ đo chuẩn"** (máy rảnh + khung 20 phút); tối ưu ctor-enum **chưa** xếp lịch | VD-204 (5) · VD-217 (ST-219) · ST-217 · `JOURNAL/2026-11-18.md` | **đóng ở ST-300m2** — A/B 24 cặp trên `mps` arm64 (load **1,93** sau 60 s chờ; không soak/gate; xen kẽ AB/BA; `--no-cache`): interp **113,597 → 117,965 ms, B/A 1,0385** (cặp median 1,0419; 1,0067..1,7178) · VM **119,416 → 126,892 ms, B/A 1,0626** (cặp median 1,0638; 1,0301..1,1249); **cả hai median <1,10** ⇒ enum-hoá không chậm >10 % trên macOS arm64 (khớp ps 1,00/1,02). `#152` đóng; bảng `JOURNAL/st300m2-mac-bench.md` §3. | **+ VD-241-frame (19/11 ~11:40, ps, HEAD `ac49a08`, máy rảnh load 0,07 → 0,48 trong khi chạy)**: A = `7c256df` (trước enum-hoá) · B = `ac49a08` — `test/bench/opt-bench.vis` (tái dẫn nguồn từ JD 2026-11-18: 100 000 vòng `map.get` + `??`, in `38995450`; hai đường chạy hai binary đều in `38995450`). **24 cặp xen kẽ**: interp trung vị 335,6 → 335,6 ms (**B/A = 0,9999**; cặp min 0,926 / median 1,001 / max 1,078) · VM 354,8 → 360,6 ms (**B/A = 1,0165**; min 0,993 / median 1,016 / max 1,099). **Cả hai khoảng hẳn dưới 1,10** ⇒ enum-hoá **không** chậm hơn ngưỡng 10 % tại ps. **Chưa đóng #152** — thiếu vế mac (VD-241 cần ps **và** mac cùng HEAD); **KHÔNG** tối ưu ctor-enum; giữ "chờ đo chuẩn" tới khi mac đo xong hoặc A0 quyết | VD-204 (5) · VD-217 · VD-228 · **VD-241 việc 3** |
| **+ VD-228-frame (19/11 ~02:00, máy rảnh load 0,26, release, 24 cặp, `/tmp/opencode/ab-result-vd228.json`)**: A = `7c256df` · B = `1855762` — VM trung vị **174,9 → 201,8 ms** (B/A **= 1,154**; mediana cặp 1,161; khoảng **1,088..1,210**) · interp **311,4 → 345,1 ms** (B/A **= 1,108**; mediana cặp 1,107; khoảng **1,074..1,171**) — cả hai vẫn **CHƯA RÕ** theo VD-217 (khoảng cắt ngang 1,10 cả hai phía; mediana cặp cả hai >1,10 nhưng min <1,10) ⇒ **giữ “chờ đo chuẩn”**; số này là số sạch (máy rảnh), thay phiên “máy có tải” |
| 153 | **Lệch interp↔VM: builtin làm giá trị** — `let f = Some` / truyền `Some` vào `map` ở VM trả `() không phải hàm` (**VS0501**), interp chạy được; bytecode thiếu mã `MakeBuiltin` (VBC1 đóng băng) ⇒ VM dựng giá trị qua builtin nội bộ **`$value:<tên>`** (dùng lại `CallBuiltin`), `None` vẫn là literal `Option`; VM `TestCtor` chuyển về phân biệt hoa thường | VD-208 (2) · ST-217 · `JOURNAL/2026-11-18.md` | đóng ở ST-217 |
| 154 | **Tầng kiểu `Option`/`Result` giữ dựng sẵn** (VD-208 chốt **A** — VD-204 (1) làm rõ: chỉ tầng chạy là enum std thật). Phương án **B** (bảng kiểu nội hàm + intern lazy, xóa `Type::Opt`) **không xếp lịch** — ước lượng: 1–2 ngày, 10–20 golden đổi byte, `hop_nhat` thêm tải; chỉ mở lại khi có tính năng thật sự cần (ví dụ người dùng `impl` trait cho `Option`) | VD-208 · ST-217 · `BLOCKED.md` §8 | mở |
| 155 | **Lift tham số closure khi lambda annot có** — VD-232 (Agent 0, 2026-11-19): lambda hiện **`(x: int?) =>` không parse** (`VS0201`), và `(x) -> int? suite`-form cũng không parse return-annot cho lambda params; closure-biên return đã nâng (VD-229), nhưng **closure params không nâng được** (không có annot) — phần code lift params cho closure **chưa có ≈ 0 dòng**; ghi nợ để làm khi có syntax annot lambda | VD-232 · `JOURNAL/vs-ergo.md` · `vd212-thiet-ke-ghi-chu.md` | **đóng ở ST-301** — parser `lambda_param := IDENT [':' type]` + `['->' type] =>`; auto-lift params closure interp+VM; 22 ca `vd283_lambda_annot.rs` |
| 156 | **`vis-targets` còn 16/18 sau SQLite MVP**: `vis-cli` × `x86_64-pc-windows-msvc` và × `wasm32-unknown-unknown` đỏ vì `libsqlite3-sys` (rusqlite `bundled`) chạy `cc-rs` cần C toolchain cho target (msvc: thiếu `lib.exe`; wasm32: không có `cc`); tiêu chí VS2 "targets 18/18" (R22 §3; ST-74/75/134) **không còn đo được** trên host Linux. **Chưa** sửa mã/script/kỳ vọng; cần A0 quyết: (a) feature `sqlite` optional + ma trận đo `--no-default-features`, (b) ghi 2 ô `EXPECTED_RED` có văn bản, (c) cross C toolchain trong CI | ST-231 · `JOURNAL/st231-targets.log` (sha256 `79eeefbe3084ee3df7ef44407771c1747637f8b6216ebb4ff3117c8650d359b4`) · VD-241 (SQLite narrow freeze) | **đóng ở ST-254** (nợ #156; các commit nhãn `ST-156` là nhãn nhầm — task `ST-156` trong QUEUE là R27 lịch sử) — feature `sqlite` tùy chọn; core `--no-default-features` **18/18**, sqlite default **16 PASS + 2 EXPECTED_RED có lý do**, 0 FAIL; ma trận `agent/TARGET-MATRIX-2026-11-20.md`. Theo A0: không bắt buộc mọi target xanh.
| 157 | **VM: `for` lồng + `break` pop sai iterator** ⇒ lặp vô hạn tới `VS0501 chạy quá lâu (>10^7 lệnh)`; interp `0 1 done` exit 0 (`codegen.rs:603-631`, `vm.rs:840-858`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-238** |
| 158 | **VM: `while let` nhánh sai để giá trị trên stack** ⇒ verify từ chối `hai đường tới với độ sâu 1 và 0`; interp `after` exit 0 (`codegen.rs:633-659`, `verify.rs:93`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-238** |
| 159 | **VM: biến `let` cấp tệp không thấy trong hàm** ⇒ `()`; interp `101` exit 0 (`codegen.rs:805-850`, `interp.rs:637-660`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-238** |
| 160 | **VM: bỏ qua `const` cấp tệp** ⇒ `()` rồi `VS0501`; interp `10 11` exit 0 (`codegen.rs:233-250`, `interp.rs:224-228`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-238** |
| 161 | **`?.` base optional: interp thiếu `Some`** ⇒ interp `An`, VM `Some(An)` (kiểu `str?` ⇒ VM đúng) (`interp.rs:748-772`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-239** |
| 162 | **`?.` base KHÔNG optional: VM bọc sai** ⇒ VM `Some(An)`/`không có phương thức len`; interp `An 2` exit 0; **cần A0 chốt** cấm hay no-op (`codegen.rs:1018-1046`, `tyck/truong.rs`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-239** |
| 163 | **VM: `match` rơi khỏi mọi nhánh im lặng** ⇒ `after` exit 0; interp trap `VS0501 match không khớp nhánh nào` exit 101 (`codegen.rs:710-753`, `:1448-1472`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-238** |
| 164 | **VM: closure ghi biến bắt `var` reset giữa các lần gọi** ⇒ `11 11 1`; interp `11 21 1` (`codegen.rs:355-370`, `vm.rs:441-461`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-238** |
| 165 | **VM: or-pattern bind theo vế đầu** ⇒ `()`; interp `2` (`codegen/pattern.rs:232-239`) | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-239** |
| 166 | **VM: `set{...}` không dedup** ⇒ `[1, 2, 2, 3, 1]`; interp `[1, 2, 3]` | ST-234 · `JOURNAL/st234-lech-vm-interp.md` (repro `JOURNAL/st234-repro/`) | **đóng ở ST-239** |
| 167 | **`net.read` cấp phát theo `max_bytes` người dùng** ⇒ **abort** (exit 134); mong đợi `Err`/cap 8 MiB (`stdlib.rs:477-489`, `net.rs:185-192`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-238** |
| 168 | **`std.list.sort` im lặng không sắp** khi không so được (NaN/bản ghi) ⇒ `[3, NaN, 1, 2]`; `sort_by` cùng lõi lại báo `VS0501` (`stdlib.rs:266-271`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 169 | **`json.encode` xuất `inf`/`NaN`** cho `percent` vô cực ⇒ JSON không hợp lệ (`stdlib.rs:704-705`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-238** |
| 170 | **`eq_value` map sai khi khoá trùng** (`{"x":1,"x":1} == {"x":1,"x":2}` ⇒ `true`); literal giữ khoá trùng, `map.get` đầu / `json.decode` cuối (`value.rs:292-296`, `interp.rs:796-803`, `vm.rs:736`) — dedupe literal **cần A0** | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-239** |
| 171 | **`ls --long`: mtime > 2262 tràn `u128→i64`** ⇒ in năm 1861 (`ngoai.rs:347-352`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 172 | **Số học `percent` bão hoà im lặng** (và `show` bão hoà) thay vì `VS0507`/đúng giá trị (`builtin.rs:419-455`, `value.rs:421-428`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` · ST-237 (`JOURNAL/st237-ma-tran-28.md`) · **VD-243** · ST-240 (`JOURNAL/st240-review.md`) | **đóng ở ST-240** — `percent` vượt miền ⇒ `VS0507` exit ≠ 0; literal không biểu diễn được bị từ chối tại `lit_value` (hai engine cùng **loại** lỗi, mã khác nhau — VD-243 §1); `BLOCKED.md` §12 đóng |
| 173 | **`json.encode` mất dấu float** (`1.0`→`1`) ⇒ round-trip đổi kiểu; có thể là chủ ý — **cần A0** (`stdlib.rs:697-700,818-826`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 174 | **`std.path` sai ở gốc `/`**: `basename("/")=""`, `dirname("/")="."`; POSIX `"/"` (`stdlib.rs:935-955`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 175 | **`json.decode` nhận `01`/`-01`** (JSON không hợp lệ; Python từ chối) (`json.rs:240-291`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 176 | **`-9223372036854775808` không viết được** — lexer tràn trước khi áp `-` (tầng literal) — **KHÔNG phải lỗi**: `SPEC-lex.md:21,40,44` chốt `-922…808 = -(922…)` tràn ⇒ `VS0104`; dùng `i64.min` hoặc `-922…807 - 1` | ST-235 · `JOURNAL/st235-audit-std-cli.md` · ST-237 (`JOURNAL/st237-ma-tran-28.md`, bác bỏ có bằng chứng) | **không làm có lý do** |
| 177 | **Trap trong module bị gán sai tệp + dòng/cột** (báo cuối tệp chính) (`interp.rs:56-64`, `main.rs:520-548`; đo: `rt/a.vis:4:1` thay vì `rt/b.vis:2:5`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 178 | **CRLF: chẩn đoán `tyck` lệch 1 dòng/cột** (`check`, `--json`, `run`) — `modules.rs:861` dùng text gốc, span trên text chuẩn hoá | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 179 | **LSP `publishDiagnostics` range sai với tệp CRLF trên đĩa** (`lsp/chan_doan.rs:51`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 180 | **`vis check --json`: warning mềm (`VS0211`) làm `ok:false` + exit 1** trong khi TAP exit 0 (`cli/check.rs:251-253,285-293`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 181 | **`vis run --dry`/`--sandbox-check`: tệp sai cú pháp vẫn exit 0** và in “không khai báo quyền” (`main.rs:298-305`, `quyen.rs:101-190`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-238** |
| 182 | **`vis fmt --check/--write` bỏ qua im lặng** tệp người dùng nêu tên nếu nằm trong `test/fmt` / `test/check` / `test/doc-examples` hoặc có sibling `.err` (`cli/fmt.rs:149-155`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 183 | **`vis fmt` nhiều tệp: chẩn đoán không nêu tên tệp** (`cli/fmt.rs:185,200`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 184 | **LSP framing: header cụt không có `Content-Length` bị coi là EOF sạch** (exit 0) (`lsp/khung.rs:31-36`) | ST-235 · `JOURNAL/st235-audit-std-cli.md` | **đóng ở ST-240** |
| 185 | **Wrapper/toolchain**: lệnh chuẩn `./vis` có thể âm thầm chọn rustc/cargo khác `vis-dev` (thực tế `vis test` cần `PATH="$HOME/.cargo/bin:$PATH"` để dùng 1.99.1, nếu không dừng với lỗi “rustc 1.93.1 is not supported”). Cần cách chọn đúng hoặc **báo lỗi rõ trước** fresh-clone/release verification; **không** sửa chen vào codegen | A0 bổ sung 2026-11-20 (sau ST-238) · `QUEUE` ST-238 ghi rõ commit từng phép đo · **bin/_vis-toolchain.sh + test/toolchain/run.sh (VD-241/VD-246)** | **đóng** — `bin/vis`/`bin/vis-dev` dùng chung helper chọn cargo đúng `rust-version`; PATH cũ + không fallback ⇒ exit 78 + thông báo rõ, PATH cũ + rustup default ⇒ fallback đúng; self-test “thiếu cargo báo rõ (78)” xanh |
| 186 | **Lệch chẩn đoán cho chương trình KHÔNG hợp lệ**: có cả `let` cấp tệp lẫn `fn main` thì interp/VM vẫn lệch thông điệp (gọi là ST-28 trong báo cáo ST-238). Phân loại: lệch chẩn đoán; **giữ mở**. Nâng mức ngay nếu một bên vẫn thực thi có tác dụng phụ thay vì cả hai cùng từ chối | A0 bổ sung 2026-11-20 · `JOURNAL/st238-review.md` §2 · ST-238 (không phải hồi quy) | **mở** — chờ xử lý riêng, không chặn đợt này |
| 187 | **Cache không vô hiệu khi đổi logic codegen**: `cache::PHIEN_BAN` là chuỗi cố định, không đổi theo logic nên `vis run --vm` (không `--no-cache`) có thể nạp bytecode sinh **trước ST-248** và trả `5`/`7` thay vì `Some(5)`/`Some(7)`. Test Rust/`vis test --vm` biên dịch mới nên không dính; người dùng có cache v2 từ sau ST-246 vẫn dính. Cần bump `PHIEN_BAN` + re-pin khoá ghim + test. | ST-248 (`JOURNAL/st248-review.md` §2) · `BLOCKED.md` §16 · ST-249 | **đóng ở ST-249** — `PHIEN_BAN` += `+st248-field-index-lift-2026-11-20`, khoá ghim re-pin `66c332a9… → ad63ae98…`, test khẳng định khoá cũ không còn trùng |
| 188 | **Global hợp lệ chỉ số nhưng chưa khởi tạo ⇒ `Unit` thành công**: verifier đếm tĩnh mọi lệnh ghi (kể cả nhánh không chạy/hàm chưa gọi) nên mở miền; runtime `unwrap_or(Unit)` trả giá trị mặc định, **không chẩn đoán**. A0 chọn **runtime trap** (VD-245), không chấp nhận giữ `Unit`, chưa mở CFG definite-assignment. | ST-247 (`JOURNAL/st247-review.md`) · `BLOCKED.md` §14 · **VD-245** · ST-250 (`JOURNAL/st250-review.md`) | **đóng ở ST-250** — `Vec<Option<Value>>`, `doc_global` trap `VS0501` exit 1, 8/8 test, `cargo test -p vis-vm` xanh, `vis test` 525/525 ×2 |
| 189 | **Finding riêng ST-250 — capture/local đọc ngoài miền vẫn trả `Unit` (mức Trung bình/Thấp)**: `Instr::LoadCaptured(i)` trong `vm.rs` dùng `get(i).unwrap_or(Unit)` và verifier (`verify.rs:449`) **không kiểm biên capture** ⇒ crafted bytecode có thể đọc capture ngoài phạm vi thành `Unit` thành công (cùng lớp lỗi với global, nhưng là đường local/capture). `LoadLocal` nhánh local cũng `unwrap_or(Unit)` nhưng verifier chặn `s ≥ nlocals` và khung khởi tạo sẵn `Unit` ⇒ chỉ defense-in-depth (Thấp). A0 yêu cầu ghi finding riêng, **không tuyên bố đã an toàn** chỉ vì global đã sửa. | ST-250 (`JOURNAL/st250-review.md` §4) · `BLOCKED.md` §17 · **VD-246 / ST-252** (`JOURNAL/st252-review.md`) | **đóng ở ST-252** — verifier giữ `ClosureFree`, runtime `doc_local`/`doc_captured`/`ghi_local` trap `VS0501`, 8/8 ca, hết thành công giả |
| 190 | **Finding riêng ST-252 — `pop_frame` writeback silent skip** (`vm.rs:686`): `if let (Some(dst), Some(src)) = (c.get_mut(*i), fr.locals.get(*slot))` bỏ qua khi ô mirror ngoài miền. **Khác ngữ nghĩa** (ghi ngược trạng thái closure, không phải đọc trả `Unit`) và **không thể xảy ra** với bytecode đã verify + thực thi (mọi `LoadCaptured(i)` prologue mirror đã trap trước writeback). Đổi `pop_frame` (`Option<Frame>` → `R<…>`) là refactor 3 call-site, vượt phạm vi “không viết lại VM”. | ST-252 (`JOURNAL/st252-review.md` §2) · `BLOCKED.md` §18 | **mở** — ghi riêng, không tự sửa; **chưa tìm thấy đường tới từ bytecode đã verify** (A0, không viết thành bảo đảm tuyệt đối); nâng blocker ngay nếu có đường tới thực tế |
| 191 | **Windows nextest chưa đạt (VD-247/A0)**: `cargo nextest run --workspace --no-fail-fast` trên `dbae70a` cho **1417 chạy / 1416 passed / 1 failed**; ca fail `tcp_explain_tong` gặp `WSAECONNABORTED` (chạy riêng đạt **1/3**). **Nguyên nhân chưa được phân lập đầy đủ** — không kết luận “race của test socket, không phải sản phẩm”. Nextest **không** được dùng làm đường nghiệm thu Windows tại mốc này; đường chuẩn đã nghiệm thu là `cargo test --workspace --no-fail-fast` (1422/0). Cần đảm bảo cài nextest không tự động đưa người dùng sang đường gate chưa được hỗ trợ mà tài liệu vẫn gọi là đạt. | A0/VD-247 (2026-11-20) · `QUEUE` ST-259 · `ST-296` (`15271f12`) · `JOURNAL/st296-pre-qa.md` | đóng ở ST-296 (`VD-282`: `cargo nextest run --workspace --no-fail-fast` trên `psvn` @ `f6939721` — **350 binaries, 3.583/3.583 passed, 3 skipped, 0 fail/flake**, 213,4 s test / 481 s tổng, sha256 `412a4e43…b47b`; công nhận nextest là đường QA Windows hợp lệ) |
| 192 | **Tồn đọng `P4c` của `ST-292` (VM `--workers` reactor & cancel test)**: entry CLI VM `--workers > 1` chưa truyền `FrameReactorBackend` (main frame dùng async net dưới `--vm --workers` trả `VS0505`) và thiếu ca test cancel worker chuyên biệt qua trigger frame | `ST-292` · `JOURNAL/st292-p4-frame-worker.md` §4 · `VD-283` | **đóng ở ST-301** — CLI VM `--workers > 1` đã truyền `FrameReactorBackend`; ca `vm_p4b_cancel_worker_qua_trigger_frame` + parity frame W=1/2/4 trong `vs3_worker_frame_vm` |
| 193 | **WASI stack overflow ở `vis-interp` cho đệ quy sâu (2/390 ca)**: `test/sem/ham/de-quy-1000.vis` và `de-quy-vo-han.vis` gặp `RuntimeError: memory access out of bounds` ở `vis-interp` trên `wasm32-wasip1`, trong khi `vis-vm` pass **390/390 = 100%** (tổng coverage 388/390 = 99,5%) | `ST-300` · `JOURNAL/st300-sem-wasm.md` §3 · `VD-283` | mở — theo dõi riêng (tăng stack WASI / trampoline interp) |

## 5. Nợ chuyển VS3 (21 mục)

| Số | Lý do chuyển VS3 (1 dòng) | Nguồn |
|---|---|---|
| 12 | VD-123: async song song thật + GC là dự án VS3 có tài liệu thiết kế riêng | R11 §5 + R12 §5 · R23 §4 hàng 1 · VD-123 |
| 16 | VD-123: wait-set/đánh thức khi `close` thuộc rendezvous — VS3 | R11 §5 + R12 §5 · VD-123 |
| 17 | VD-123: `select`/timeout trên kênh là VS3 | R11 §5 + R12 §5 · R23 §4 hàng 2 · VD-123 |
| 19 | VD-123: `chan(0)`/rendezvous là VS3; VS2 từ chối bằng `VS0505` | R11 §5 + R12 §5 · VD-123 |
| 21 | VD-122: TLS/keep-alive chưa có trong VS2, xét rustls ở VS3 | R11 §5 + R12 §5 · R23 §4 hàng 3 · VD-122 |
| 33 | VD-124: siết mặc định toàn cục là nợ VS3 | R11 §5 + R12 §5 · R23 §4 hàng 4 · VD-124 |
| 43 | VD-124: như #33; ST-130 chỉ siết mặc định khi chương trình dùng gói | R12 §5 · VD-124 (ST-130 siết mặc định khi dùng gói) |
| 71 | VD-123: rendezvous + wait-set là VS3 (VS2 giữ `chan(0)` ⇒ `VS0505` exit 1) | R15 §5 · R23 §4 hàng 2 · VD-123 |
| 72 | VD-123: wait-set cho drain là VS3 | R15 §5 · VD-123 |
| 83 | VD-122: TLS/HTTP2/keep-alive là VS3 | R18 §5 · R23 §4 hàng 3 · VD-122 |
| 109a | `JOURNAL/st128` §8 ghi mức VS3; trùng nội dung #116 | JOURNAL/st128-resolver.md §8 · ST-128 · QUEUE ST-128 · R23 §6 #116 |
| 113 | `JOURNAL/st126` §8 ghi mức VS3 | R23 §6 · ST-126 · JOURNAL/st126-dong-bang-api.md §8 |
| 114 | `JOURNAL/st126` §8 ghi mức VS3 | R23 §6 · ST-126 · JOURNAL/st126-dong-bang-api.md §8 |
| 115 | `JOURNAL/st127` §8 ghi mức VS3 | R23 §6 · ST-128 · JOURNAL/st127-semver.md §8 |
| 116 | `JOURNAL/st128` §8 ghi mức VS3 | R23 §6 · ST-128 · JOURNAL/st128-resolver.md §8 |
| 118 | `R23` §6 ghi “(VS3)”; `JOURNAL/st128` §8 mức VS3 | R23 §6 · ST-128/129 · JOURNAL/st128-resolver.md §8 |
| 119 | `JOURNAL/st129` §9 ghi mức VS3 | R23 §6 · ST-129 · JOURNAL/st129-lock.md §9 |
| 120 | VD-133: nợ #115 (số cũ) chuyển VS3 | R23 §6 · ST-130 · VD-133 |
| 127 | `JOURNAL/st132` §7 ghi VS3 | R23 §6 · ST-132 · JOURNAL/st132-landlock.md §7 |
| 143 | VD-154(b): điều kiện VD-133 đã thoả (**ca `st130b` xanh** từ đợt 23) ⇒ `NamePolicy::Strict` chỉ bật cho tệp **trong cây module** là việc VS3 | JOURNAL/st130-siet-caps.md §9 · VD-133 · VD-149 · VD-154(b) |
| 155 | VD-232: lambda params không parse annotation (VS0201) — lift params closure chỉ sau khi có syntax annot lambda; closure-biên return đã nâng (VD-229);  | VD-232 · `vd212-thiet-ke-ghi-chu.md` |

## 6. Nợ “không làm có lý do” (1 mục)

| Số | Mô tả | Lý do không làm | Nguồn |
|---|---|---|---|
| 104 | LSP chưa publish chẩn đoán tệp phụ cho tài liệu chính | **VD-126**: LSP chỉ publish chẩn đoán cho tài liệu client đang mở; lỗi module phụ báo qua vị trí import. Publish cho URI chưa mở gây hiển thị không nhất quán giữa các client | R22 §5 · VD-126 · R23 §4 hàng 5 |

Ghi chú: các mục **có chủ đích** trong báo cáo nhưng **chưa có văn bản đóng** vẫn giữ `mở`, chờ Agent 0:
#3 (gán lồng sâu `VS0412` có chủ đích), #27 (LSP EOF ⇒ exit 0 khác spec, cần cho test/CI).

## 7. Số trùng nghĩa — tách bằng hậu tố (không đổi số gốc)

### 7.1 `#109` — hai nghĩa, đã tách trong bảng chính

| Số trong sổ | Nghĩa | Nguồn | Trạng thái |
|---|---|---|---|
| **#109a** | `next_block()` trả `801` khi khối `07xx` còn chỗ (ST-128) | `JOURNAL/st128-resolver.md` §8 · `QUEUE` ST-128 | chuyển VS3 (trùng nội dung **#116**) |
| **#109b** | **Đường GHI** vẫn kiểm-rồi-mở ⇒ cần dòng giới hạn trong `HOST-BAO-DAM.md` | `R23` §6 · `R24` §5 #140 | mở (giới hạn còn nguyên; dòng giới hạn đã có từ `69112900`) |

**Nguồn mâu thuẫn**: `R23 §6` (#109 = đường ghi) **vs** `JOURNAL/st128-resolver.md §8` + `QUEUE ST-128` (#109 = next_block).
Sổ ghi **cả hai**, không chọn một; `#116` (số R23 chuẩn hoá) giữ nguyên nghĩa next_block ⇒ trùng nội dung `#109a`.

### 7.2 Các số từng mang ≥2 nghĩa (đợt 23: ba agent cùng bắt đầu từ 105)

`R23 §6` ghi: “Ba agent làm song song trong đợt 23 đánh số nợ trùng nhau (mỗi người bắt đầu từ 105).
R23 chuẩn hoá lại một dãy duy nhất theo thứ tự hợp nhất.” Bảng ánh xạ số cũ → số trong sổ:

| Số cũ | Nghĩa (nguồn) | Số trong sổ |
|---|---|---|
| 105 | `HSym` builtin `Span::new(0,0)` (`JOURNAL/st122-span-tep.md` §8) | #105 |
| 105 | `TEN_THE` không ràng buộc (`JOURNAL/st126-dong-bang-api.md` §8) | #113 |
| 105 | `--sandbox` chưa cấp `EXECUTE` cho `/lib*` (`JOURNAL/st132-landlock.md` §7) | #125 |
| 106 | `RecDef`/`FieldDef` prelude `file=0` (`JOURNAL/st122` §8) | #106 |
| 106 | hash nguồn VBC1 ≠ khoá cache (`JOURNAL/st126` §8) | #114 |
| 106 | `unicode-ident` ngoài allowlist (`JOURNAL/st132` §7) | #126 |
| 107 | `test/sem` không chứa được ca đa tệp (`JOURNAL/st122` §8) | #107 |
| 107 | `#![allow(dead_code)]` của `semver.rs` (`JOURNAL/st127-semver.md` §8) | đóng ở ST-129 |
| 107 | `ap_dung` mới dùng `ABI::V1` (`JOURNAL/st132` §7) | #127 |
| 108 | quy ước “điền 0” khi so phiên bản (`JOURNAL/st127` §8) | #115 |
| 108 | `pipeline.rs` bám tiền tố grep (`JOURNAL/st131-linux.md` §5) | #121 |
| 109 | `next_block()` trả 801 (`JOURNAL/st128-resolver.md` §8) | #109a / #116 |
| 109 | `sandbox.rs:59` chưa `cfg` theo nền (`JOURNAL/st131` §5) | #122 |
| 110 | `@1.2.3` = ghim (`JOURNAL/st128` §8) | #117 |
| 110 | runner thiếu `wasm32` (`JOURNAL/st131` §5) | #123 |
| 111 | yanked/registry mạng/chữ ký (`JOURNAL/st128` §8) | #118 |
| 111 | `vish_cmd_body.rs:312` đường dẫn thư mục tạm (`JOURNAL/st131` §5) | #124 |
| 112 | `Command::new(tên)` phân giải `PATH` lần nữa (`JOURNAL/st123-va-grant.md` §8) | #108 |
| 112 | lock không ghi quan hệ phụ thuộc (`JOURNAL/st129-lock.md` §9) | #119 |
| 113 | VM không đưa `argv` vào tham số `main` (`JOURNAL/st124-review.md` §7) | #110 |
| 113 | `pkg update` ghim mọi gói khác (`JOURNAL/st129` §9) | **#113b → #142** (VD-149; §7.3) |
| 114 | tyck chấp nhận `let e: cap[Dir] = json.decode(…)` (`JOURNAL/st124` §7) | #111 |
| 114 | `NamePolicy::Strict` bật cho mọi tệp có `use` (`JOURNAL/st130-siet-caps.md` §9) | **#114b → #143** (VD-149; §7.3) |
| 115 | đường GHI kiểm-rồi-mở (`JOURNAL/st124` §7) | #109b |
| 115 | `vis test`/host suite chưa áp strict-caps (`JOURNAL/st130` §9) | #120 |
| 116 | hai bộ escape JSON (`JOURNAL/st125-fuzz-json-esc.md` §7) | #112 |

**Nguồn mâu thuẫn (ghi cả hai, không chọn một):**

- `VD-133` nhắc **#114**/**#115** theo nghĩa **cũ** (`NamePolicy::Strict` / `vis test`–host suite) **vs** `R23 §6`
  (#114 = hash nguồn VBC1, #115 = quy ước điền 0). Sổ giữ số của `R23 §6`; nghĩa cũ ghi ở bảng trên.
- `QUEUE ST-128` ghi “nợ #109” = `next_block` ⇒ **#109a**; `QUEUE ST-124` ghi “nợ #115” = đường ghi ⇒ **#109b**.
- `QUEUE ST-125` ghi “nợ #116” = hai bộ escape JSON ⇒ **#112** (R23 dùng #116 cho `next_block`).

### 7.3 Hai nợ của đợt 23 **không** được R23 gộp vào dãy chuẩn — nay đã có số **142/143** (VD-149)

| Nhãn tạm | Nghĩa | Nguồn | Số chuẩn + trạng thái |
|---|---|---|---|
| **#113b** | `pkg update <tên>` ghim mọi gói khác (`=<version>`): gói được update cần nâng dep chung thì resolver báo `VS0704` thay vì nâng; cần quyết định chính sách | `JOURNAL/st129-lock.md` §9 (số cũ 113) · `QUEUE` ST-129 | **#142** (VD-149) — bảng chính: **đóng ở ST-288** (soft pin; VS2 **không** sửa) |
| **#114b** | `NamePolicy::Strict` bật cho **mọi** tệp có `use` (`modules.rs:606`), không chỉ tệp trong cây module; từng che lỗi VD-124 | `JOURNAL/st130-siet-caps.md` §9 (số cũ 114) · `VD-133` · `JOURNAL/st130b-ca-thu-8.md` | **#143** (VD-149) — bảng chính: **chuyển VS3** từ ST-154 (**VD-154(b)**; trước đó **mở**); VD-133 cho phép chuyển VS3 khi ca khoá xanh (ca `st130b` **đã xanh**) |

**Ánh xạ chính thức (VD-149, 2026-11-17): `#113b → #142`, `#114b → #143`.**
Nhãn `#113b`/`#114b` là **số cũ + hậu tố**, **không** phải số của dãy chuẩn: giữ lại ở đây làm ánh xạ,
**không** đánh lại số cũ, **không** tái dùng số **142/143** cho nghĩa khác. Báo cáo R26 trở đi trích bằng
số mới, **kèm số cũ trong ngoặc** — ví dụ `#142` (`#113b`).

### 7.4 Ca 7 self-test của ST-147 **không phải nợ có số** — **đóng ở ST-153** (VD-152, ghi chú)

| Mục | Nội dung |
|---|---|
| Triệu chứng | psvn: self-test case âm **11/12** — ca **7** `doc-examples: ngoại lệ cũ đã xanh`; 6 ca encoding khác đã xanh nhờ **VD-150** (ST-147) |
| Nguyên nhân | `crates/vis-cli/src/cli/check.rs` tách tên gốc bằng `rsplit('/')`; trên Windows (`\`) `phu` **luôn** khác rỗng ⇒ bộ lọc `--syntax` (QD §7.4) **giữ** chẩn đoán **kiểu** ⇒ khối `x = 1` đỏ ⇒ câu `ngoại lệ cũ: …` không bao giờ in (`JOURNAL/st147-chan-doan-psvn.log`) |
| Sửa | **ST-153** (VD-152): MỘT hàm `ten_goc` tách tên gốc theo **cả** `/` và `\`, dùng đúng chỗ cũ; `phu`/`--syntax` **không** đổi gì khác; **+3 ca unit** chạy mọi nền |
| Bằng chứng | psvn @ `4a805115`: self-test **12/12** + cổng **đi QUA bước 5** (dừng bước 10 `L0`, `cap_proc` 150/4 — VS3) — `JOURNAL/st153-gate-psvn.log`; `vis check --syntax C:\…\a.vis` ⇒ `ok 1` `RC=0` (trước ST-153: `not ok … VS0301`) — `JOURNAL/st153-psvn-ca7.log` |
| Trạng thái | **đóng ở ST-153** — **không** cấp số nợ mới: không phải nợ VS3 (lỗi sản phẩm cross-platform đã sửa, không phải giới hạn có lý do) |

## 8. Tóm tắt

| Chỉ số | Số |
|---|---|
| Tổng dòng bảng chính | **194** (1..193; #109 tách `a`/`b`) |
| `mở` | **81** |
| `đóng` | **94** |
| `chuyển VS3` | **16** |
| `không làm có lý do` | **2** |
| Không phải nợ (điều hướng `#141`) | **1** |
| Ngoài dải chuẩn (nhãn tạm `#113b`, `#114b`) | **0** — đã nhận số **#142/#143** (VD-149, §7.3) |
| Mục **không truy được nguồn** (không đưa vào sổ) | **0** |

Kiểm: 81 mở + 94 đóng + 16 VS3 + 2 không làm + 1 điều hướng = **194** dòng (ST-238: #176 `mở → không làm`; ST-238/239/240/249/250/252: 9+5+13+1+1+1 mục `mở → đóng`; **ST-265: #148 `mở → đóng`**; **ST-288: #142 `mở → đóng`**; **ST-289: #111 `mở → đóng`, #151 `VS3 → đóng`**; **VD-282 (GĐ 1 / ST-293 / ST-295 / ST-296): đóng 12 mục** — 8 `mở → đóng` (`#26`, `#27`, `#84`, `#98`, `#109b`, `#125`, `#140`, `#191`) + 4 `VS3 → đóng` (`#16`, `#17`, `#19`, `#21`); `#31` giữ `mở` thu hẹp; **ST-297 (`f4cb136`): `#149` `mở → đóng`**; **VD-283: thêm `#192` (`mở`, P4c VM reactor/cancel) và `#193` (`mở`, WASI interp stack 2/390)**; thêm #185/#186/#187/#188/#189/#190/#191; **ST-300m1: #137 `VS3 → đóng`**; **ST-300m2: #147/#152 `mở → đóng`**; **ST-302: #8/#13 `mở → đóng`**).
**Stage 0 `v0.3.6` (2026-11-22, `f8f1d4b`)** — trạng thái nợ theo lát (không thêm dòng nợ mới):
`ST-284` DONE (`83a9a10`, merge `e363a15`; R0a 0 đổi hành vi), `ST-285` DONE (`ab5d292`, merge `246b6c5`;
R0b 0 đổi hành vi), `ST-286` DONE (`f193c57`, merge `cfba662`; R0c 0 đổi hành vi), `ST-287` DONE
(`03ed84c`, merge `f8f1d4b`; hook huỷ HTTP h1/h2 — h1 **5/5**, h2 **8/8**, không rò tay, parity
interp↔VM; soak 1 giờ fd/RSS/threads phẳng), `ST-288` DONE
(`c513e7a`, merge `545a38d`; Gatekeeper độc lập 2026-11-22 — `VS0708`/soft pin/gc `--root` xanh;
đóng **#142**; QUEUE còn REVIEW chờ A0 lật), `ST-289` DONE (`ff55056`, merge `84915aa`; đóng
**#111**/**#151**). **Stage 0 đủ 6/6 DONE**; R32 đã chốt phần Stage 0, còn full/nightly/fresh-clone
trước tag `v0.3.6`. `./vis-dev gate --fast` @ `f8f1d4b`: **3581 passed / 3 skipped**; Windows @
`0d057e5`: **3533/0/3**, `vis 0.3.6` (`JOURNAL/stage0-win-0d057e5.md`); diff 100k seed `20261115` /
`20261122` đều **lech=0** (`JOURNAL/st28{4,5,6,7}-diff-100k.log`).
**Đợt VD-258..VD-270 (21/11)**: ngoài **#148** đã đóng ở ST-265, các gap còn mở của đợt (step-cap parity;
budget/fuel/Q chưa chốt default; coverage frame; nested async guard 200 000; parser chưa nhận `[.., z]`;
method `self`/gán sâu) **chưa cấp số nợ** — theo dõi ở `JOURNAL/vd260-report.md` §6 và
`BAO-CAO-BAN-GIAO-2026-11-20.md` §2; chỉ cấp số khi A0 chốt, không tự thêm để tránh trùng.
**Luật (VD-195)**: bảng này phải **bằng** số đếm thật của §4; `./vis-dev check` đỏ nếu lệch (luật **R5** trong `bin/vis-dev`).
**#148/#149/#150** thêm bởi **VD-183(6)/VD-184(3)/VD-192**; **#151** thêm bởi **VD-198(2)** (18/11); **#152** thêm bởi **VD-204 (5)**; **VD-217 re-đo A/B (24 cặp): VM 1,154 / interp 1,143 — chưa rõ ⇒ giữ chờ đo chuẩn** (18/11, ST-219); **VD-220 làn B re-đo (load 1,27): VM 1,129 / interp 1,109 — chưa rõ** (18/11, nhánh dem/B merge `a0f2100`); **#153** thêm bởi **VD-208 (2)** rồi đóng ở ST-217; **#154** thêm bởi **VD-208** (phương án B, không xếp lịch) (18/11). **+ VD-231/VD-232 (19/11)**: **#155** (lift params closure khi lambda annot có) chuyển VS3. **VD-211** không sinh nợ (tầng kiểu đã cấm `==` khác kiểu — VS0412 "so sánh khác kiểu").
**#147 đóng ở ST-204 rồi MỞ LẠI bởi VD-197** ⇒ đóng −1, mở +1.
**ST-242 (20/11, VD-242 Q7)**: **#147/#152** cần đo trên macOS nhưng không có máy Darwin thật (`vis-remote mac --check` trả Linux) ⇒ gắn nhãn **BLOCKED-PHẦN-CỨNG**, **giữ nguyên "mở"**; `mở` vẫn **121**, tổng vẫn **185** (ST-237 đã chuyển #176 sang `không làm`, nên `mở` = 121 chứ không phải 122). Nhãn blocked là căn cứ, **không phải hạng mục đếm mới** — R5 vẫn xanh.

**Vì sao 146 chứ không phải 144 — đối chiếu VD-154(b) (ghi lại thay vì giấu):** mức sổ **trước** ST-154 là **144 dòng** =
91 mở / 31 đóng / 20 VS3 / 1 không làm / 1 điều hướng (ST-151). ST-154 làm **ba** việc trên sổ: (1) `#143` **mở → chuyển VS3**
(VD-154(b)) ⇒ mở −1, VS3 +1 → **90 mở / 31 đóng / 21 VS3 = 144 dòng**, đúng **nguyên văn** con số của VD-154(b); (2) thêm dòng
**#144** (`VD-153`, **đóng ở ST-154**) ⇒ đóng +1; (3) thêm dòng **#145** (`VD-154(c)`, **mở**) ⇒ mở +1. Luật sổ (§3: "chỉ thêm
dòng mới ở cuối bảng, không xoá dòng"; VD-147(3): mỗi nợ **một số, một dòng**) ⇒ tổng thành **146 dòng**. Vậy `90/31/21` là số
**đúng** cho bảng 144 dòng sau bước (1), còn số **cuối cùng** của sổ sau ST-154 là `91/32/21/1/1 = 146`; hàng QUEUE ST-154 ghi
`90 mở/31 đóng/21 VS3` là số của bước (1), cần Agent 0 đính chính nếu muốn R27 trích số cuối. Máy kiểm:
`JOURNAL/st154-kiem-trung.log`.

**Đóng trong đợt 24g (ST-148 đối chiếu lại, chỉ ghi khi có bằng chứng trong repo):** `#89` (ST-139 · PR #24 ·
`ef6489d1` · `JOURNAL/st139-do-truoc.log`), `#130` (ST-142 · `JOURNAL/st142-ps-*.log`: 447/447 hai đường,
`lech=0`), `#139` (ST-140 · `JOURNAL/st140-red-ps.log` + `st140-cargo-test-ps.log`), `#126` (ST-132 · VD-135),
`#131` (ST-00 · đính chính R23/R24 §1). Giữ **mở**: `#109b`/`#140` (đường ghi — dòng giới hạn đã có trong
`HOST-BAO-DAM.md`; sửa kỹ thuật thuộc VS3).

**Đóng trong đợt 25:** `#136` (ST-149 · nợ phân loại thân `cmd` — biến bind từ ngoài che tên lệnh;
đỏ-trước `JOURNAL/st149-do-truoc.log`, tái hiện `JOURNAL/st149-tai-hien.log`, `cargo test --workspace` 1163/0),
`#133` (ST-150 · ca âm **ghi** ở tiến trình con cho thư mục **chỉ-đọc**: `exit=7` + `Permission denied`/`EACCES`,
tệp không được tạo, đối chứng dương ghi được; `ps` 14/14 xanh — `JOURNAL/st150-landlock-ps.log`),
`#135` (ST-151 · danh sách sync của `bin/vis-linux` thành **một nguồn** + `--check-sync` + **9 ca** khoá: bỏ một mục ⇒ đỏ;
`JOURNAL/st151-do-truoc.log` + `JOURNAL/st151-gate-macos-lan2.log`),
`#144` (ST-154 · ca 15 `con_read_dir_thu_muc_co_quyen_ghi_van_liet_ke_duoc`: **dương** `read_dir` trên thư mục **có quyền ghi**
⇒ `exit 0` + `co.txt`, đối chứng ghi `exit 0`, âm ngoài phạm vi `exit 7` + `Permission denied`; `ps` **15/15** xanh —
`JOURNAL/st154-landlock-ps.log`).

**Chuyển VS3 trong đợt 25:** `#143` (ST-154 · **VD-154(b)**: ca khoá `st130b` **đã xanh** ⇒ điều kiện VD-133 đã thoả).
**Thêm số trong đợt 25:** `#144` (VD-153), `#145` (VD-154(c) — chú thích `rt.rs` nói sai đường chặn `chan(0)`, **mở**).
**Thêm số 2026-11-20 (ST-231):** `#156` — `vis-targets` còn **16/18** sau SQLite MVP (`libsqlite3-sys` cần C toolchain cho msvc/wasm32); chỉ **đo + ghi nợ**, không sửa mã/script/kỳ vọng; cần A0 quyết (`JOURNAL/st231-targets.log`).
**Thêm số 2026-11-20 (ST-234):** `#157..#166` — **10 lệch interp ↔ VM** đã xác nhận độc lập (`for` lồng+break, `while let`, `let`/`const` cấp tệp, `?.` hai loại base, `match` rơi nhánh, closure ghi biến bắt, or-pattern, `set` dedup); chi tiết + repro trong `JOURNAL/st234-lech-vm-interp.md`; **chỉ đo + ghi nợ**, chưa sửa; ca `?.` base không-optional cần A0 chốt ngữ nghĩa.
**Thêm số 2026-11-20 (ST-235):** `#167..#184` — **18 phát hiện** từ audit std/builtins + CLI (abort `net.read`, sort im lặng, JSON `inf`/mất float, map khoá trùng, mtime tràn, percent bão hoà, `std.path` gốc, trap module sai tệp, CRLF lệch dòng, LSP range/framing, `check --json` warning, `run --dry` bỏ lỗi cú pháp, `fmt` bỏ qua tệp); chi tiết + repro `JOURNAL/st235-audit-std-cli.md`; **chỉ đo + ghi nợ**, chưa sửa; #170/#173 cần A0 chốt.

**Đóng trong đợt 28 (ST-157):** `#110` — VM đưa `argv` (`list[str]`) vào tham số **không-`cap`** của `main` như interp
(`interp.rs:221-234`; ST-72b giữ `Value::Cap` đúng **chỉ số**), ca đỏ-trước **3/4 đỏ** (interp `[a, b]` vs VM `()`,
`JOURNAL/st157-do-truoc.log`), sau sửa **4/4 xanh** + `cap_khong_lo::doi_so_dong_lenh_la_str` (ca **ghim** sự lệch cũ)
cập nhật để khẳng định **parity** thay vì ghim lệch. Sổ **146 mục = 90 mở / 33 đóng / 21 VS3 / 1 không làm / 1 điều hướng**.

## 9. Máy kiểm của task này

**ST-145 (lập sổ)**:

- Trích nguồn: `JOURNAL/st145-trich-no.log` (script `/tmp/st145-trich-no.py`, sha256 ghi trong log).
- Kiểm trùng số trong bảng chính: `JOURNAL/st145-kiem-trung.log` ⇒ **0 số trùng** (142 dòng).
- `python3 test/report/check_report.py --all` ⇒ PASS (24 báo cáo, 164 hàng QUEUE).
- `cd viscript && ../vis-dev gate --fast` ⇒ PASS (L0 1159/0, report-check 24/164, doc-examples 24 khối) —
  log `JOURNAL/st145-gate-macos.log` (header VD-136 + sha256 + `[exit=0]`).

**ST-148 (cập nhật theo VD-149, HEAD `b855caa3`)**:

- Thêm số: **#142** (`#113b`), **#143** (`#114b`); ánh xạ ở §7.3 — hai số **không** tái dùng số cũ.
- Kiểm trùng số + dải + nguồn + ánh xạ + khớp §8: `JOURNAL/st148-kiem-trung.log` (script `/tmp/st148-kiem-trung.py`,
  sha256 trong log) ⇒ **PASS, 0 số trùng**, dải **1..143** đủ, 144 dòng, trạng thái **94 mở / 28 đóng / 20 VS3 /
  1 không làm / 1 điều hướng**.
- `python3 test/report/check_report.py --all` ⇒ **PASS (25 báo cáo, 170 hàng QUEUE)** — `JOURNAL/st148-check-report.log`.
- `cd viscript && ../vis-dev gate --fast` ⇒ **PASS**, `[exit=0]`, **10/10 bước OK**, L0 **1159/0** trên 129 suite,
  report-check PASS (25/170), doc-examples OK (24 khối) — `JOURNAL/st148-gate-macos.log` (header VD-136 + sha256
  + `[exit=0]`).

**ST-149 (nợ #136, HEAD `456e8a4a` + mã/ca ST-149)**:

- Đỏ-trước: `cargo test -p vis-cli --test vish_cmd_body -- --test-threads=1` ⇒ ca
  `bien_bind_cap_tep_trung_ten_lenh_la_bieu_thuc` in **bảng `ls`** thay vì `3` (19/20 xanh) —
  `JOURNAL/st149-do-truoc.log` (sha256 `63928844f57e640f5185f8b61f9c59041c626b7447d92a1abd72cee6807f8068`).
- Sửa: `crates/vis-types/src/lower/khai.rs` **+37/−6** (helper `ten_bien_bind_toi` + `la_lenh_da_biet` loại trừ biến
  bind từ ngoài); **4 ca mới** (20 ca) trong `crates/vis-cli/tests/vish_cmd_body.rs`; tái hiện in **`3`**
  (`JOURNAL/st149-tai-hien.log`, sha256 `6beba7334be163a1e3a3c1eb0245236bd86b4263dea091599508819561273d8c`).
- `cargo test --workspace` ⇒ **1163/0** (129 suite; 1159 + 4 ca) — `JOURNAL/st149-cargo-test.log`
  (sha256 `96ab2705eedf4450d41e6108d7b6430b71bcd5b75bf76fc9e64330fdf47e2237`).
- `cd viscript && ../vis-dev gate --fast` ⇒ **PASS**, `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1163/0** (129 suite),
  report-check PASS (25 báo cáo, 171 hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st149-gate-macos.log`
  (header VD-136 + sha256 + `[exit=0]`, sha256 `ad574c8e1994a0e48e62b2214e957dd0eb74fa504a5ef06878b2803451005c2f`).

**ST-150 (nợ #133, HEAD `8abb0c81` + ca ST-150 chưa commit, đo trên `ps`)**:

- Ca 14 `con_ghi_vao_thu_muc_chi_doc_bi_kernel_chan` trong `vis-host-linux/tests/landlock.rs` (**+101** dòng, ca cũ
  không sửa): đỏ-trước `exit=0` + nội dung `truoc ruleset`; sau ruleset (2 luật trong **một** `ap`) tiến trình con
  `ghi` vào thư mục chỉ-đọc ⇒ `exit=7` + `loi: Permission denied (os error 13)` (`EACCES`), `x.txt` **không** được tạo;
  đối chứng dương cùng ca `ghi` vào thư mục `ghi: true` ⇒ `exit=0` + `duoc ghi`.
- `ps`: **14/14 xanh** (13 ca cũ vẫn xanh), `test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
  finished in 0.33s` — `JOURNAL/st150-landlock-ps.log` (sha256 `614b124a…`, 4 699 byte, header VD-136).
- sha256 tệp đo **hai phía khớp**: local `fa2aab0fd1bfd122fb11eee1783adee50639e935b8dde324b6b64d37deaa1461` =
  `ssh root@ps sha256sum /root/vifast-linux/vis-host-linux/tests/landlock.rs` (cùng giá trị); HEAD `8abb0c81` hai phía.
- `cd viscript && ../vis-dev gate --fast` ⇒ **PASS**, `[exit=0]`, **10/10** bước, L0 **1163/0** (129 suite),
  report-check PASS (25 báo cáo, 171 hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st150-gate-macos.log`
  (header VD-136 + sha256 + `[exit=0]`, sha256 `6aa8b4f8…`). `vis-host-linux` **không** thuộc workspace `viscript/`.
- Sau khi sửa sổ (`#133` đóng): đếm lại **144 dòng** = **92 mở / 30 đóng / 20 VS3 / 1 không làm / 1 điều hướng**;
  `python3 test/report/check_report.py --all` ⇒ **PASS (25 báo cáo, 171 hàng QUEUE)** — `JOURNAL/st150-check-report.log`.

**ST-151 (nợ #135, HEAD `32b79d4a` + mã/ca ST-151 chưa commit)**:

- `bin/vis-linux` **+44/−10**: `SYNC_DIRS="viscript docs/viscript vis-host-linux vis-host-wasm .git"` +
  `SYNC_FILES="AGENTS.md vis-dev vis"` ở đầu tệp (sau `set -u`); `sync_tree()` chỉ đọc hai biến (bỏ 5 chỗ rải trong vòng lặp
  + `rsync` `AGENTS.md` + vòng `vis-dev`/`vis`). **Không đổi hành vi**: 8 lời gọi `rsync` của bản cũ và bản mới trên cùng một
  cây giả (stub `rsync`/`ssh`) **giống hệt** (thứ tự, `--delete`, exclude, đích); `AGENTS.md` vẫn best-effort
  (`2>/dev/null || true`), hai shim vẫn `die 3` — `JOURNAL/st151-do-truoc.log` §[0].
- `--check-sync`: in 8/8 mục `có|thiếu`; `exit=0` khi đủ, `exit=2` khi thiếu (`thiếu n/8`); **không** ssh/rsync
  (ca tĩnh khoá `cmd_check_sync` không chứa `ssh`/`rsync`; nhánh `--check-sync` không gọi `pick_backend`).
- **9 ca** `crates/vis-cli/tests/sync_list.rs` (khuôn `soak_script.rs`): đỏ-trước `error: no test target named sync_list`
  (exit **101**) ⇒ xanh 9/9; **bỏ `vis-host-wasm` khỏi `SYNC_DIRS`** ⇒ **4 ca đỏ**; **rải lại `for d in AGENTS.md vis-dev vis`**
  ⇒ ca `sync_tree_chi_dung_hai_bien` đỏ; hoàn nguyên sha256 `aedf781a…` — `JOURNAL/st151-do-truoc.log`
  (sha256 `2bf3b11655627b6b7eeb6bf6d33f394445fd7eb1d824c17eee87b9f4946fd30f`).
- `macOS` gate lần 1 **ĐỎ** ở L0 — ca **THỜI GIAN** `lsp::module::tests::hieu_nang_nap_module_5_tep` (`116 ms ≥ 100 ms`
  dưới tải `7.39/6.18/5.27`, 12 core) ⇒ **T-INFRA** (D2-07), chạy riêng **5/5 xanh** (`JOURNAL/st151-lsp-hieu-nang.log`);
  log đỏ giữ nguyên `JOURNAL/st151-gate-macos.log` (sha256 `8626ab3b…`). Lần 2 **PASS** `[exit=0]`, **10/10** bước,
  L0 **1175/0** (**130** suite; = 1166 + 9 ca), report-check PASS (25 báo cáo, 171 hàng QUEUE), doc-examples OK (24 khối)
  — `JOURNAL/st151-gate-macos-lan2.log` (sha256 `a53c38a829727829256a783982487f297f24aecf52208a518f6bbb6959aedd2e`).
- `--check-sync` trên macOS: `exit=0`, 8/8 `có` — `JOURNAL/st151-check-sync-macos.log` (sha256 `935fb81f8927fee495ba780bcb64c84def36e5e5971853ff6ee12eb10d2bcc0d`).
- Sau khi sửa sổ (`#135` đóng): đếm lại **144 dòng** = **91 mở / 31 đóng / 20 VS3 / 1 không làm / 1 điều hướng**;
  trùng số 0, dải **1..143**.

**ST-154 (nợ #144 + sổ theo VD-154, HEAD `0582913b` + ca 15/SO-NO/log chưa commit)**:

- Ca **15** `con_read_dir_thu_muc_co_quyen_ghi_van_liet_ke_duoc` trong `vis-host-linux/tests/landlock.rs`
  (**+137** dòng, **không** sửa ca nào cũ, không `#[ignore]`/`cfg`-bỏ): `co.txt` ghi **trước** khi áp ruleset; **một** ruleset
  **hai** luật (`trong` `ghi:false` + `trong-ghi` `ghi:true`, `kq.so_luat == 2` như ca 14) · **dương**: helper `ls trong-ghi`
  ⇒ **`exit 0`** + danh sách `co.txt`; **đối chứng luật ghi có hiệu lực**: helper `ghi trong-ghi/moi.txt` ⇒ `exit 0`, `ls` lại
  thấy **`co.txt` + `moi.txt`** · **âm cùng ca**: helper `ls ngoai` ⇒ **`exit 7`** + `loi: Permission denied (os error 13)`
  (`EACCES`) · đỏ-trước in `exit=0` + `co.txt`; helper gọi thẳng `std::fs::read_dir`, **không** qua `FsGuard` ⇒ **kernel** chặn.
- `ps`: **15/15 xanh** (`test result: ok. 15 passed; 0 failed; 0 ignored`, 0,35 s; 14 ca cũ vẫn xanh) — kernel
  `7.0.0-43-generic`, Ubuntu 26.04.1, rustc 1.99.1, `lsm` có `landlock` — `JOURNAL/st154-landlock-ps.log`
  (5 278 byte, sha256 `cb4ab4ab1b57f5d653b19addd21cced7dab877a74eb9557876ecb7865b95dbca`, header VD-136 + `### EXIT_VIS_LINUX_RUN=0`).
- **sha256 tệp đo HAI PHÍA khớp**: local `7102e90b3a484a9128fe981240090e7ccd43a7e9f96c6c05f4576ac57e871627` =
  `ssh root@ps sha256sum` (cùng giá trị), 25 454 byte. Lần đo **1** (trước `cargo fmt`) giữ nguyên ở
  `JOURNAL/st154-landlock-ps-lan1-truoc-fmt.log` — 15/15 xanh, chỉ khác khoảng trắng trong **một** `assert!` của ca 15.
- Sổ theo **VD-154**: `#143` → **chuyển VS3** (ghi rõ `VD-154(b): chuyển VS3 vì ca st130b xanh`, **giữ** dòng lịch sử);
  `#144` **đóng ở ST-154** (vế 2 của `#133`); thêm **`#145`** (**mở**, xử khi có task VS3 kênh). Sổ **146 dòng** =
  **91 mở / 32 đóng / 21 VS3 / 1 không làm / 1 điều hướng**; trùng số **0**, dải **1..145** — `JOURNAL/st154-kiem-trung.log`
  (PASS 10/10 mục kiểm; sha256 của chính log này ghi ở hàng QUEUE ST-154 + JOURNAL §ST-154 — không tự trích trong tệp đang được băm).
- `90 mở/31 đóng/21 VS3` (VD-154(b), lặp ở hàng QUEUE ST-154) là số của bảng **144 dòng** sau bước đổi `#143` và **trước**
  khi thêm hai dòng `#144`/`#145`; sổ cuối cùng **146 dòng = 91/32/21/1/1** — đối chiếu ở §8 + `st154-kiem-trung.log`.
- `cd viscript && ../vis-dev gate --fast` ⇒ **PASS**, `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1175/0** (**130** suite),
  report-check PASS (**26** báo cáo, **173** hàng QUEUE), doc-examples OK (24 khối) — `JOURNAL/st154-gate-macos-lan2.log`
  (76 725 byte, sha256 `aae5447557dc20f71b8ebfcf9eea31b42bf74f538d957ed98941e0d04ae9053b`, header VD-136 + `[exit=0]`).
  **Lần 1 ĐỎ** ở bước đầu `--- fmt: FAIL` (rustfmt đòi xuống dòng **một** `assert!` của ca 15) — không phải `T-INFRA`:
  sửa fmt rồi chạy **lần 2**, log đỏ giữ nguyên `JOURNAL/st154-gate-macos.log` (2 449 byte, sha256
  `3b0a12649934c5df71b5f5e2bd6c4d264b8d193cb3d541ae117c94704cea2429`).
- `python3 viscript/test/report/check_report.py --all` ⇒ **PASS (26 báo cáo, 173 hàng QUEUE)** — `JOURNAL/st154-check-report.log`.


**ST-157 (đóng #110, HEAD `567350e1`)**:

- `cargo test -p vis-cli` ⇒ **xanh**: 77 suite, **778 / 0** (gồm ca mới `vm_main_argv` **4/4**).
- Ca đỏ-trước: `cargo test -p vis-cli --test vm_main_argv` **3/4 ĐỎ** (interp `[a, b]` vs `--vm` `()`) — `JOURNAL/st157-do-truoc.log`
  (`[exit=101]`), chạy **trước** khi sửa.
- `cd viscript && ../vis-dev gate --fast` ⇒ **PASS** `[exit=0]`, **10/10** bước `--- …: OK`, L0 **1179/0** (**131** dòng
  `test result`, cơ sở 1175 + 4 ca mới) — `JOURNAL/st157-gate-macos.log` (header VD-136 + sha256 + `[exit=0]`).
- `target/debug/vis test test/sem` ⇒ **447/447**, exit 0 — `JOURNAL/st157-vis-test.log`; `--vm` cũng **447/447**
  (0 `not ok`) — `JOURNAL/st157-vis-test-vm.log`.
- Sổ: `#110` **mở → đóng ở ST-157**; §8 **146 = 90/33/21/1/1**; §9 khối này.
