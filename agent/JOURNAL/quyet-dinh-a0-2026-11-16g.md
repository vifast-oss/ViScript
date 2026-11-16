# QUYẾT ĐỊNH AGENT 0 · Dòng ViScript · Đợt 24 (chạy qua đêm)

**Ngày**: 2026-11-16 · **Từ**: Agent 0 · **Gửi**: Agent 1 (dán + đối chiếu), Agent 2 (thi hành) · **Căn cứ**: REVIEW-AGENT1-CHO-AGENT0-2026-11-16f.md; HEAD a9f80d83 (R24)

**Tên tệp**: REVIEW đề nghị tên `QUYET-DINH-A0-2026-11-16f`, nhưng tên này đã dùng cho đợt 23f. Tệp này là **16g**. Mọi dòng trích "§4.x của 16f" trong REVIEW đọc thành "16g §x" tương ứng bên dưới.

**Quy trình**: theo VD-137, **Agent 1 dán** §3, §4, §5, §6 (commit `[ST-00]` ghi `theo QUYET-DINH-A0-2026-11-16g`), xong mới báo Agent 2 chạy §7. Agent 2 không dán.

---

## 1. Trả lời D1..D9

| Mục | Quyết định |
|---|---|
| D1 Duyệt 8 hàng | **Đồng ý, từng hàng một dòng** (§5.1). Không dùng dòng gộp |
| D2 Ca coreutils (#139) | **(a) có sửa**: không dùng chương trình hệ thống, dùng binary do chính workspace build (VD-144) |
| D3 macOS fd→path (#137) | **(a)**: giới hạn có văn bản, sửa ở VS3 (VD-145) |
| D4 HOST-BAO-DAM | **Đồng ý có sửa**, văn bản cuối ở §6 |
| D5 Windows | **(a)**: giữ "chỉ đo" + W-1, W-2, W-3 (VD-146) |
| D6 R23 | **Đồng ý**: duyệt như báo cáo đợt 23, không chốt VS2 |
| D7 R24 | **Duyệt có điều kiện** gate tại HEAD (§2.2), **và** đính chính hạng mục 7 (§2.1) |
| D8 Đợt 25 | **Đồng ý 5 việc, thêm 3**, thành 8 task chạy qua đêm (§4, §7) |
| D9 Quy trình | Agent 1 **được** chạy bước kiểm tài liệu chỉ đọc; **có** sổ nợ duy nhất (VD-147) |

## 2. Hai điểm Agent 0 phát hiện thêm

### 2.1 R24 ghi "Linux chạy thật" trái VD-134

VD-134: hạng mục chỉ được ghi "Linux: chạy thật" khi `cargo test --workspace` **và** `gate --fast` PASS trên runner. Hiện Linux là 1161/**1** và gate **10/11**. Ca đỏ là ca môi trường, nhưng luật không có ngoại lệ cho ca môi trường. Vì vậy:

- Hạng mục 7 tại thời điểm R24: **Linux "chờ"** (ca đỏ duy nhất: `symlink_canonicalize_hai_phia`, xử lý ở ST-140).
- ST-140 xanh và gate Linux PASS ⇒ R25 ghi "Linux: chạy thật".
- Không sửa R24 (VD-142). Đính chính bằng dòng §5.3.

Hệ quả cho chốt VS2: **R24 chốt VS2 về kỹ thuật cho 7/8 hạng mục**; hạng mục 7 hoàn tất ở R25. Đây là 1 ca test, không phải lỗi sản phẩm, nên không mở lại VS2.

### 2.2 Duyệt ST-138 trước khi ngủ

Để không bắt Agent 2 chờ qua đêm, Agent 0 **ký trước có điều kiện**: Agent 2 chạy ST-139 (gate tại HEAD). Gate PASS ⇒ Agent 1 đối chiếu log và dán dòng §5.2. Gate đỏ ⇒ **không** dán, ghi XEM LẠI, Agent 2 sửa nếu ≤150 dòng và không đụng ngữ nghĩa, ngược lại BLOCKED.

Hai số đỏ-trước của TOCTOU (R23 đo 798/50 000, ST-137 đo 92/50 000) khác nhau vì là cuộc đua thời gian, không phải mâu thuẫn. Cả hai đều là đỏ thật. R25 ghi cả hai kèm log.

## 3. Nối cuối `viscript/agent/DECISIONS.md`

```markdown
### VD-144 · Ca symlink_canonicalize_hai_phia không phụ thuộc chương trình hệ thống (2026-11-16)
Trên ps, /usr/bin/echo là coreutils multicall, từ chối chạy khi argv[0] khác tên; ca đỏ vì môi trường. Cho phép sửa CHÍNH CA: đích của symlink là một binary do workspace build (env!("CARGO_BIN_EXE_<tên>") của một binary đã có trong workspace); không có binary phù hợp thì dùng /bin/sh -c. Symlink do ca tự tạo trong thư mục tạm. Giữ nguyên hai chiều khẳng định: grant qua symlink chạy được; grant đường dẫn thật chặn bản sao cùng tên. Không xoá ca, không ignore, không cfg-bỏ, không nới ngưỡng; chú thích ca ghi lý do và VD-144. Không áp cho ca nào khác.

### VD-145 · macOS không có fd→path trong VS2 (2026-11-16)
Trên macOS, std an toàn không lấy được đường dẫn thật của handle đã mở. VS2 giữ bước xác nhận (dev, ino) trước/sau canonicalize của ST-124 trên macOS; đo 0/50 000 nhưng KHÔNG bảo đảm theo cơ chế. Giới hạn ghi trong HOST-BAO-DAM.md. Sửa cần libc hoặc unsafe, là việc VS3, phải có VD riêng. Hạng mục 4 Quyền đóng trong VS2 với giới hạn này.

### VD-146 · Windows trong VS2: chỉ đo, cổng chạy được (2026-11-16)
Giữ VD-121, VD-136: Windows chỉ đo, không tuyên bố hỗ trợ, không sửa sản phẩm cho Windows. Cho phép ba việc hạ tầng: (1) rustup target add wasm32-unknown-unknown và x86_64-unknown-linux-gnu trên psvn, ghi vào viscript/infra/windows/, theo VD-140; (2) viscript/test/doc-examples/doc_examples.py đọc ống bằng encoding="utf-8"; (3) crates/vis-cli/tests/sandbox.rs: xử lý biến không dùng khi mọi nhánh cfg(target_os) bị loại, bằng cách dùng biến trong nhánh mặc định hoặc đặt allow(unused) đúng một biến; không đổi kỳ vọng, không xoá ca, không ignore, không cfg-bỏ ca. Các nhóm lệch /bin/*, escape \, CRLF, tiến trình là việc VS3.

### VD-147 · Quy trình sau đợt 24 (2026-11-16)
(1) Bổ sung VD-137: Agent 1 được chạy các bước kiểm tài liệu chỉ đọc (check_report.py, doc_examples.py --gate) và ghi kết quả vào báo cáo; vẫn không chạy cargo, clippy, L0, gate, soak. (2) Dòng APPROVED phải ghi người đối chiếu và căn cứ. (3) Sổ nợ duy nhất viscript/agent/SO-NO.md: một số một nghĩa, không tái dùng số; cột: số, mô tả, nguồn (task/báo cáo), trạng thái (mở / đóng ở task X / chuyển VS3 / không làm có lý do). Số trùng nghĩa cũ được tách bằng hậu tố (#109a, #109b), không đánh lại số. Báo cáo từ R25 trích nợ bằng số trong sổ này. (4) Mỗi báo cáo ghi hash commit đo cho TỪNG nền. (5) Log bằng chứng không được ghi đè log của task khác; mỗi lần chạy công cụ dài (soak, host suite) ghi tệp mang ID task đang chạy.

### VD-148 · Chốt VS2 (2026-11-16)
R24 chốt VS2 về kỹ thuật cho 7/8 hạng mục. Hạng mục 7 Đa nền tảng: Linux ở trạng thái "chờ" theo VD-134 tới khi cargo test và gate --fast PASS trên ps (ST-140); R25 ghi kết quả. Windows chỉ đo (VD-146), wasm chỉ check (VD-121). VS3 không bắt đầu tính năng cho tới khi Agent 0 ban hành tài liệu thiết kế VS3; trong lúc chờ, Agent 2 chỉ làm việc dọn dẹp, đo và nợ VS2.
```

## 4. Hàng mới cho `viscript/agent/QUEUE.md` (sau ST-138)

```markdown
| ST-139 | **Gate tại HEAD sau R24** (16g §2.2) | ST-138 | 0,5 h | **READY** | ./vis-dev gate --fast tại HEAD có R24; dòng cuối + số L0 dán dưới hàng ST-138; đỏ ⇒ XEM LẠI, không tự lật ST-138 |
| ST-140 | **Ca coreutils theo VD-144** | ST-139 | 1,5 h | **READY** | đỏ trước trên ps có log; sau sửa: ca xanh trên ps và macOS; cargo test --workspace trên ps 0 đỏ; gate --fast trên ps PASS 11/11; diff ca đúng một tệp test |
| ST-141 | **Log không ghi đè** (VD-147 (5)): vis-soak nhận VIS_SOAK_LOG/--log, mặc định tên theo ID task; khôi phục st113-soak.log từ 36ed5922; tách soak đợt 24 sang st138-soak.log; vis-host-suite in nền thật (uname -sm) | ST-139 | 1 h | **READY** | git show 36ed5922:<đường dẫn> trùng byte với st113-soak.log đã khôi phục; st138-soak.log trùng byte nội dung ở a9f80d83; chạy vis-host-suite trên ps in "Linux"; không xoá log nào |
| ST-142 | **Số Linux đầy đủ** (nợ #130): vis test test/sem interp và --vm, diff 100k seed 20261115, cache, trên ps | ST-140 | 1,5 h | **READY** | 447/447 cả hai đường, 0 SKIP; lech=0; số đặt cạnh macOS kèm hash commit; lệch thì liệt kê, không sửa |
| ST-143 | **Soak 10 trên ps** | ST-142 | 1 h | **READY** | log st143-soak.log; 10/10 hoặc liệt kê vòng đỏ + phân loại T-INFRA (chạy lại tối đa 1 lần chỉ để phân loại) |
| ST-144 | **Windows: 3 việc VD-146 + đo lại** | ST-139 | 2,5 h | **READY** | target add có log trước/sau; doc_examples.py 1 dòng; sandbox.rs tối thiểu; đo lại cargo test, host suite, vis-targets, gate --fast trên psvn; số đặt cạnh ST-136; KHÔNG sửa ca lệch nào khác |
| ST-145 | **Sổ nợ duy nhất** (VD-147 (3)) | ST-141 | 2 h | **READY** | viscript/agent/SO-NO.md gom mọi nợ từ R1..R24 + JOURNAL; mỗi số một nghĩa; #109 tách #109a/#109b; mỗi mục có nguồn; nợ VS3 đánh dấu rõ; check_report PASS |
| ST-146 | **R25** | ST-140..ST-145 | 1,5 h | **READY** | đầu báo cáo: danh sách hàng phụ thuộc + trạng thái (VD-143); mục "Đính chính R24" (hạng mục 7 Linux, ST-135 JSON/MD, hash từng nền); hash đo từng nền; hạng mục 7 cập nhật theo kết quả ST-140; 5 câu VD-131; nợ trích theo SO-NO.md; check_report PASS |
```

## 5. Dòng duyệt cho `viscript/agent/QUEUE.md`

### 5.1 Dán ngay dưới dòng CHỜ DUYỆT của từng hàng

Dưới **ST-130** (và ST-130b nếu là hàng riêng):
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: siet_caps_pkg.rs 8 #[test]; ca thứ 8 chạy interp và --vm --no-cache, exit 0, có đối chứng --strict-caps ⇒ VS0604; sem 447/22 không đổi
```
Dưới **ST-132** và **ST-132b** (mỗi hàng một lần):
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: landlock.rs 13/13 trên ps; 3 ca âm ở tiến trình con (đọc, ghi, read_dir) có đỏ trước và đối chứng dương; macOS --sandbox exit 3
```
Dưới **ST-134**:
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: setup.sh trong repo; 2 ca theo VD-132 có nhánh nền khẳng định giá trị cụ thể; ca (iv) lỗi sản phẩm sửa 68 dòng (≤150), vish_cmd_body 16/16 hai nền
```
Dưới **ST-135**:
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: chỉ đo; Linux 1161/1, host 54/54, targets 18/18, gate 10/11; ca đỏ duy nhất là môi trường, xử lý ở ST-140
```
Dưới **ST-136**:
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: chỉ đo; log môi trường đủ VD-136, rustc khớp ps; 102 ca lệch có tên, hai đầu ra, nhóm phỏng đoán; không sửa code hay kỳ vọng
```
Dưới **ST-137**:
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D1 · Agent 1 đối chiếu: cửa sổ thật có phân tích; đỏ trước 92/50 000 trên ps; sau 0/50 000 trên ps (2 lần) và macOS; 4 ca mới gồm /proc không đọc được ⇒ từ chối; không unsafe, không crate mới; host 54/54 hai nền; diff 100k lech=0. macOS bảo đảm theo đo, không theo cơ chế (VD-145)
```
Dưới **ST-133**:
```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §1 D6 · R23 duyệt như báo cáo đợt 23, KHÔNG chốt VS2 (VD-143); 2 câu sai đã đính chính tại QUEUE và R24 §1 (VD-142)
```

### 5.2 Dưới ST-138: CHỈ dán khi ST-139 gate PASS (Agent 1 đối chiếu log trước)

```
APPROVED 2026-11-16 Agent 0 · theo QUYET-DINH-A0-2026-11-16g §2.2 · Agent 1 đối chiếu: 6 hàng phụ thuộc đều APPROVED; gate --fast tại HEAD sau R24 PASS <dán số L0>; R24 chốt VS2 về kỹ thuật cho 7/8 hạng mục, hạng mục 7 Linux "chờ" (VD-148)
```

### 5.3 Đính chính, dán ngay dưới hàng ST-138

```
ĐÍNH CHÍNH 2026-11-16 Agent 0 · R24 §4 ghi hạng mục 7 "Linux chạy thật" trái VD-134 (Linux 1161/1, gate 10/11). Trạng thái đúng: Linux "chờ" tới ST-140 (VD-148). R24 JSON ghi ST-135 READY là đúng thiết kế check_report (đối chiếu QUEUE tại head báo cáo); R24 MD §0 ghi DONE. R24 thiếu hash đo cho macOS và Linux. Cả ba đính chính trong R25 (VD-142).
```

## 6. `viscript/agent/HOST-BAO-DAM.md`: thay mục FsGuard

Trong mục "FsGuard: mức bảo đảm chống đổi tệp giữa kiểm và dùng", **thay** gạch đầu dòng "Đường đọc", dòng con "Giới hạn tạm thời trên Linux" và gạch đầu dòng "Đường ghi" bằng đoạn dưới. **Giữ nguyên** các gạch đầu dòng "Host không báo danh tính tệp", "Hardlink", "exists và list". Không tìm thấy đúng ba khối này ⇒ BLOCKED, dán nội dung thực tế vào báo cáo.

```markdown
- **Đường đọc**: mở handle trước, quyết định quyền trên đường dẫn thật của chính handle, rồi đọc nội dung từ chính handle đó. Bất biến: nội dung trả về là nội dung của chính tệp mà quyết định quyền đã áp.
  - **Linux**: đường dẫn thật lấy qua /proc/self/fd/<fd>. /proc không đọc được thì từ chối, không quay về kiểm theo tên. Bảo đảm theo cơ chế (ST-137: 0/50 000).
  - **macOS: giới hạn đã biết (VD-145).** Std an toàn không có cách lấy đường dẫn của handle; giữ bước xác nhận (dev, ino) trước và sau canonicalize. Đo 0/50 000 nhưng không bảo đảm theo cơ chế. Sửa cần libc hoặc unsafe, việc VS3.
- **Đường ghi: giới hạn đã biết, còn nguyên trong VS2.** FsGuard kiểm đường dẫn rồi mới ghi, vì mở trước để lấy handle là đã tạo hoặc cắt tệp. Trong khoảng giữa hai bước, nếu bên khác ghi được vào thư mục cha, đường dẫn có thể bị thay bằng symlink trỏ ra ngoài vùng được cấp. Sửa triệt để cần openat và O_NOFOLLOW (cần unsafe hoặc crate ngoài), việc VS3. Trên Linux khi chạy với --sandbox, Landlock chặn ghi ra ngoài vùng được cấp ở mức kernel; không có --sandbox thì giới hạn này còn nguyên.
```

## 7. Thứ tự chạy qua đêm (Agent 2)

```
[Agent 1] ST-00: dán §3, §4, §5.1, §5.3, §6 → commit → báo Agent 2
ST-139 (gate tại HEAD) ─┬─ PASS → Agent 1 dán §5.2 (không cần chờ, Agent 2 chạy tiếp)
                        └─ đỏ → XEM LẠI, vẫn chạy tiếp các hàng độc lập
→ ST-140 (ps + macOS)
→ ST-141
→ ST-142 (ps) → ST-143 (ps)
→ ST-144 (psvn)
→ ST-145
→ ST-146 (R25)
```

**Ràng buộc chung** (như đợt 23, không nhắc ở từng hàng): đỏ trước có sha256; không unsafe; không crate mới; không ignore, không nới ngưỡng; R1 ≤1.500; gate --fast PASS trên macOS trước mỗi commit; commit `[ST-###]` chỉ `viscript/`; log môi trường theo VD-136 khi chạy trên ps/psvn; mỗi log mang ID task (VD-147 (5)).

**Điều kiện dừng:**
- Gate đỏ không sửa được trong ngân sách ⇒ hoàn tác phần chưa commit, BLOCKED + log, sang hàng **độc lập** kế tiếp (ST-141, ST-144, ST-145 độc lập với ST-140).
- 2 BLOCKED liên tiếp không do môi trường ⇒ dừng, viết R25 sớm.
- Mất SSH tới ps hoặc psvn ⇒ BLOCKED-ENV hàng đó, chạy tiếp hàng chỉ cần macOS. Không reboot máy nào.
- Số đo khác kỳ vọng trong tệp này ⇒ số đo thắng, ghi rõ.
- **Không** bắt đầu tính năng VS3 nào khi hết hàng (VD-148). Hết hàng thì viết R25 và dừng.

## 8. Báo cáo sáng mai

Agent 1 gửi Agent 0: R25, danh sách hàng CHỜ DUYỆT kèm đối chiếu (mẫu dòng như §5.1), mọi BLOCKED, và câu trả lời: hạng mục 7 Linux đã đủ điều kiện VD-134 chưa. Agent 0 sẽ ra tệp duyệt đợt 25 và bắt đầu tài liệu thiết kế VS3.
