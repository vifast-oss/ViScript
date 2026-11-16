<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# QUYẾT ĐỊNH AGENT 0 · Dòng ViScript · Đợt 25 (2026-11-17, 02:40 JST)

**Từ**: Agent 0 · **Gửi**: Agent 1 (dán + đối chiếu), Agent 2 (thi hành)
**Căn cứ**: `TRA-LOI-AGENT1-CHO-AGENT0-2026-11-16g.md`; HEAD dòng ViScript `b6e326f2` (R25); `SO-NO.md` (142 mục); `R25`; `QUYET-DINH-A0-ViScript-2026-11-16g`.
**Quy trình**: theo VD-137, **Agent 1 dán** §1, §3, §4, §5 (commit `[ST-00]` ghi `theo QUYET-DINH-A0-2026-11-17`), xong mới báo Agent 2 chạy §6.
**Hard stop**: 08:30 JST **không mở việc mới**; 09:00 JST dừng hoàn toàn, viết `R26` với số đã có.

---

## §1. Duyệt đợt 24g (đóng VS2 về kỹ thuật)

**1.1.** Tôi đã đọc báo cáo đêm của Agent 1 và đối chiếu các hàng. **Duyệt 8 hàng** (dán mỗi dòng ngay dưới dòng `CHỜ DUYỆT` tương ứng trong `QUEUE.md`):
xem khối `APPROVED` ở **§2** dưới đây (8 dòng, cùng nội dung báo cáo đêm §6).

**1.2.** **Chuẩn thuận ID** — dán ngay dưới hàng `ST-138b`:
```
CHUẨN THUẬN 2026-11-17 Agent 0 · ID ST-138b thay cho ST-139 trong QUYET-DINH-A0-2026-11-16g §4 (ST-139 đã dùng cho PR #24); nội dung/ngân sách/tiêu chí giữ nguyên văn; các hàng phụ thuộc dùng ST-138b
```

**1.3.** **VS2 đóng về kỹ thuật** — xem **VD-151** ở §3. Từ đây **không** mở task VS2 mới trừ: (a) nợ đã có số trong `SO-NO.md`, (b) đính chính tài liệu, (c) `R26`. **Không** bắt đầu tính năng VS3 cho tới khi tôi ban hành tài liệu thiết kế VS3 (VD-148 giữ nguyên).

---

## §2. Tám dòng APPROVED (dán dưới từng hàng tương ứng)

```
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §2.2 · Agent 1 đối chiếu: gate --fast tại HEAD sau R24 PASS, L0 1159/0, 10/10 bước, report-check 24/164; log st138b-gate-fast.log sha256 1c6e3d18…
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: đỏ-trước trên ps (exit 101, coreutils multicall); sửa đúng 1 tệp test theo VD-144 (hai chiều giữ nguyên); ps cargo test 1162/0 + gate PASS (L0 1162/0); macOS gate PASS 1159/0
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: st113-soak.log = d2840412…, st138-soak.log = d79df3c2… (byte-identical git show); vis-soak cấm ghi đè (exit 2) + nhận --log/VIS_SOAK_LOG; vis-host-suite in nền thật, ps 54/54
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: cùng HEAD ee58a518 hai nền — vis test 447/447 ×2 đường 0 SKIP, diff 100k lech=0, cache 520×4 0 lệch; lệch chỉ là thời gian (3,85–4,02× do spawn tiến trình)
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: soak 10/10 = 250 s trên ps; log sha256 local = host; shim vis-dev/vis đã sync (../vis-dev chạy được trên ps); VIS_SOAK_LOG không rò (unset 2 dòng)
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: VD-146 đúng 3 việc tối thiểu (không đổi kỳ vọng/ca); psvn targets 18/18 (từ 8/18), cargo test 1053/100, host 39/54 (15 ca trùng ST-136), clippy hết đỏ; gate Windows dừng bước 5/10 (cp1252 — VS3); Windows vẫn "chưa hỗ trợ"
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: SO-NO.md 142 mục (mở 92/đóng 28/VS3 20/không làm 1/điều hướng 1), trùng số 0, #109a/#109b tách, mỗi mục có nguồn
APPROVED 2026-11-17 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §7 · Agent 1 đối chiếu: R25 đủ 7 hàng phụ thuộc + 3 điểm đính chính R24 + 5 câu VD-131 + hash từng nền; check_report PASS (25 báo cáo, 164 hàng); gate macOS PASS L0 1159/0
```

---

## §3. Ba VD mới (nối cuối `DECISIONS.md`)

```markdown
### VD-149 · Cấp số nợ 142/143 (2026-11-17)
Nợ #113b (pkg update ghim mọi gói khác; nguồn JOURNAL/st129) nhận số 142. Nợ #114b (NamePolicy::Strict cho mọi tệp có use; nguồn JOURNAL/st130) nhận số 143. Không đánh lại số cũ; SO-NO.md ghi thêm dòng ánh xạ #113b→#142, #114b→#143; các báo cáo sau trích theo số mới, kèm số cũ trong ngoặc.

### VD-150 · Windows: UTF-8 cho mọi lần gọi python của cổng (2026-11-17)
Bổ sung VD-146. Được phép sửa HẠ TẦNG CỔNG để Windows đo được: đặt PYTHONIOENCODING=utf-8 và PYTHONUTF8=1 cho MỌI lần gọi python3 trong bin/vis-dev (gồm doc-examples, check_report, self-test) và trong test/fixtures/run_fixtures.sh nếu cần; không đổi kỳ vọng, không xoá ca, không ignore, không cfg-bỏ ca, không sửa sản phẩm. Điều kiện: gate macOS và Linux vẫn PASS với cùng số bước; trên psvn bước self-test phải 12/12 và cổng phải đi QUA bước 5 (dừng ở bước nào thì ghi đúng bước đó). Mọi ca lệch còn lại của Windows vẫn thuộc VS3.

### VD-151 · VS2 đóng về kỹ thuật (2026-11-17)
VS2 đóng về kỹ thuật với 8/8 hạng mục: (1) Async — xong phạm vi VS2, "không song song" (VD-123); (2) Kênh — xong phạm vi VS2, "không rendezvous" (VD-123); (3) Net/HTTP — xong phạm vi VS2, "không TLS/HTTP2/keep-alive" (VD-122); (4) Quyền — đóng, macOS có giới hạn văn bản (VD-145), Linux có Landlock thật + TOCTOU đường đọc 0/50 000; (5) LSP — đóng (VD-126); (6) Gói — đóng phạm vi VS2, mạng/chữ ký VS3 (VD-125); (7) Đa nền tảng — Linux "chạy thật" (cargo test 1162/0 + gate --fast PASS trên ps tại 02fb7320), Windows "chỉ đo" (VD-146), wasm "chỉ check" (VD-121); (8) Hợp chuẩn host — đóng (54/54 trên macOS và Linux). 5 câu bắt buộc VD-131 vẫn nguyên văn. Từ đây không mở task VS2 mới trừ nợ trong SO-NO.md, đính chính tài liệu và báo cáo vòng. Tính năng VS3 chỉ bắt đầu khi Agent 0 ban hành tài liệu thiết kế VS3.
```

---

## §4. `HOST-BAO-DAM.md` — thay 3 chỗ (Agent 1 dán; giữ nguyên các dòng khác)

1. **Dòng 36** (bảng nền tảng), thay ô "vis-host-* chạy thật":
```
| `vis-host-*` chạy thật | **đo, chưa hỗ trợ** (39/54; 15 ca `not ok` trùng danh sách ST-136 — thiếu chương trình POSIX, VS3) | **chưa có runner** (chỉ check) | Windows: **đo**, Linux: **chạy thật** (`vis-host-linux` 13/13; host suite 54/54) |
```
2. **Dòng 41**, thay tiêu đề:
```
## Bộ hợp chuẩn host — chạy THẬT trên macOS và Linux (ST-75, ST-131/ST-142)
```
3. **Dòng 55–56**, thay hai dòng bằng:
```
Log: `agent/JOURNAL/st75-suite.log` (macOS, ST-75); `st137-host-suite-macos.log`, `st137-host-suite-ps.log`,
`st142-*`, `st144-host-suite.log` (Windows). **Linux: đã đo** (54/54, ST-142). **Windows: đã đo, chỉ đo**
(39/54, VD-146/VD-121 — chưa hỗ trợ). **wasm: chưa đo** (chỉ check, VD-121).
```
Nếu không tìm thấy đúng ba khối trên ⇒ **BLOCKED**, dán nội dung thực tế vào báo cáo, không tự diễn giải.

---

## §5. VS3 — hiến chương (Agent 0 giữ; chưa giao việc)

VS3 là **dự án riêng**, mỗi mục cần tài liệu thiết kế trước khi mở task. Thứ tự đề nghị: (1) Async song song thật (scheduler đa luồng/work-stealing + suspend-resume + wait-set) — kèm quyết định mô hình bộ nhớ/GC; (2) Kênh: rendezvous `chan(0)`, `select`, timeout; (3) Net: TLS (rustls/webpki — cần duyệt crate + giấy phép), HTTP/2, keep-alive; (4) Quyền: `openat`/`O_NOFOLLOW` cho đường GHI (nợ #142/#109b), macOS fd→path (`libc`/`unsafe`, nợ #137/#145); (5) Windows: hỗ trợ thật (đường dẫn, `/bin/*`, CRLF, tiến trình) — cần runner ổn định + bộ ca riêng; (6) Gói: registry qua mạng + chữ ký số + GC store. Điều kiện chung: mỗi mục có mục tiêu đo được, ngân sách, ca đỏ-trước, cổng; **không** nới ngưỡng; mỗi mục tối đa 3 h/task. Tôi sẽ ban hành tài liệu thiết kế VS3 khi người điều phối quyết định mở dự án — đêm nay **không** làm VS3.

---

## §6. Hàng mới cho `QUEUE.md` (sau `ST-146`), thứ tự chạy và điều kiện dừng

```markdown
| ST-147 | **Windows gate bước 5**: đặt UTF-8 cho mọi lần gọi python của cổng (VD-150) | ST-144 | 1,5 h | **READY** | gate macOS PASS (1159/0, cùng số bước); trên psvn: self-test 12/12, cổng đi QUA bước 5 và dừng ở bước ghi đúng trong log; không sửa ca lệch; diff chỉ script; log st147-* |
| ST-148 | **Sổ nợ theo VD-149**: thêm #142/#143 + ánh xạ #113b/#114b; đánh dấu nợ đã đóng trong đợt 24g (#130, #139, #89, #109b còn mở) | ST-146 | 0,5 h | **READY** | SO-NO.md có #142/#143; trùng số 0; mỗi mục mới có nguồn; check_report PASS; gate macOS PASS |
| ST-149 | **Nợ #136**: ca khoá phân loại thân `cmd` khi biến TRÙNG TÊN LỆNH (đường thoát `( … )`) | ST-146 | 1,5 h | **READY** | ≥3 ca: biến trùng tên lệnh dùng như biểu thức; `( … )` là biểu thức; lệnh thật vẫn là lệnh. Nếu hành vi sai: sửa ≤150 dòng hoặc ghi kế hoạch + BLOCKED phần sửa; không đổi kỳ vọng ca cũ |
| ST-150 | **Nợ #133**: ca âm **GHI** cho thư mục chỉ-đọc trong `vis-host-linux` | ST-146 | 1 h | **READY** | chạy trên ps: con ghi vào thư mục chỉ-đọc ⇒ EACCES (kernel), đối chứng dương ghi trong thư mục cho phép; 13 ca cũ xanh; log st150-* |
| ST-151 | **Nợ #135**: kiểm tự động danh sách sync của `bin/vis-linux` | ST-146 | 1 h | **READY** | có kiểm (test Rust hoặc chế độ `--check-sync`); đỏ khi bỏ một thư mục khỏi danh sách; xanh khi đủ `viscript docs/viscript vis-host-linux vis-host-wasm .git vis-dev vis`; gate macOS PASS |
| ST-152 | **R26** + cập nhật ROADMAP/STATE (chốt đợt 25) | ST-147..ST-151 | 1,5 h | **READY** | danh sách hàng phụ thuộc + trạng thái (VD-143); đính chính nếu có; số đo từng nền; nợ trích theo SO-NO; 5 câu VD-131; check_report PASS (26 báo cáo) |
```

**Thứ tự bắt buộc**: `ST-148` → `ST-147` → `ST-149` → `ST-150` → `ST-151` → `ST-152`.
**Điều kiện dừng**:
- 08:30 JST: **không mở việc mới**; nếu đang làm thì hoàn tất trong 20 phút hoặc revert + `BLOCKED`, rồi sang `ST-152`.
- Task vượt ngân sách 30 phút ⇒ dừng, `BLOCKED` + log, chuyển hàng kế (ST-148/150/151 độc lập với ST-147/149).
- Gate đỏ không sửa được trong ngân sách ⇒ revert phần chưa commit, `BLOCKED` + log, chuyển hàng kế.
- Mất SSH tới `ps`/`psvn` ⇒ `BLOCKED-ENV` hàng đó, chạy tiếp hàng chỉ cần macOS.
- Số đo khác kỳ vọng ⇒ **số đo thắng**, ghi rõ.
- 2 `BLOCKED` liên tiếp không do môi trường ⇒ dừng việc, viết `R26` sớm.
