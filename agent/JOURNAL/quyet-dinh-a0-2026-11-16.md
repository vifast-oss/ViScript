# QUYẾT ĐỊNH AGENT 0 · Dòng ViScript · Đợt 23

**Ngày**: 2026-11-16 · **Từ**: Agent 0 · **Gửi**: Agent 2 (dòng ViScript) · **Căn cứ**: BAN-GIAO-ViScript-CHO-AGENT0.md (2026-11-16, commit cuối dòng ViScript `c1587398`)

Tệp này **tự chứa**. Agent 2 đọc hết §0 đến §2 trước khi làm bất cứ việc gì, rồi chạy theo §5 từ trên xuống.

---

## 0. Quyền thực thi (đọc trước)

Agent 0 **không có quyền ghi repo trực tiếp**. Vì vậy, theo **VD-119** (bên dưới):

1. Mọi văn bản Agent 0 soạn trong tệp này (hàng QUEUE, mục VD, đoạn AGENTS.md, đoạn docs/viscript/, ROADMAP) được Agent 2 **dán nguyên văn** vào đúng tệp đích.
2. Commit riêng, tiền tố `[ST-00]`, thông điệp có dòng `theo QUYET-DINH-A0-2026-11-16`. Chỉ chứa các tệp Agent 0 sở hữu; không trộn với code.
3. **Không diễn giải, không rút gọn, không thêm bớt.** Chỗ nào không áp được nguyên văn (dòng đích không tồn tại, cấu trúc tệp khác mô tả) ⇒ **BLOCKED**, ghi rõ dòng thực tế đang có, chuyển task kế.
4. Ủy quyền này **chỉ** áp dụng cho văn bản nằm trong tệp này. Không suy rộng sang việc tự sửa QUEUE/DECISIONS/docs ngoài phạm vi.

**Duyệt khi chạy liên tục**: khi một task xong và đạt tiêu chí, Agent 2 lật hàng QUEUE sang `**DONE**` và ghi ngay dưới hàng: `CHỜ DUYỆT Agent 0 · <hash> · <số đo chính>`. Agent 2 **không** tự ghi dòng `APPROVED`. Agent 0 sẽ duyệt hàng loạt ở tệp quyết định kế tiếp. Task phụ thuộc vào task "CHỜ DUYỆT" **được phép** chạy tiếp.

---

## 1. Đối chiếu bàn giao: chấp nhận gì, sửa gì

Agent 0 chấp nhận hiện trạng §3 và §5 của bàn giao làm **mốc** (sẽ xác nhận lại bằng R23). Các điểm sai/lệch trong chính tệp bàn giao, **không được chép lại**:

| # | Chỗ trong bàn giao | Sai | Đúng |
|---|---|---|---|
| 1 | §9 top 10, mục 8 "32 FUNCS API freeze" | lẫn hai số khoá | FUNCS = **52**; all_codes = **32** |
| 2 | §2 "đã tách 6 lần" | liệt kê **7** cặp | 7 lần tách |
| 3 | Tiêu đề "12 quyết định" | thực tế 15 (Q13..Q15 phụ) + 5 điểm review §10 | Tệp này quyết cả **20** |
| 4 | §10 "Bốn câu này phải xuất hiện" | liệt kê **5** ý | Xem **VD-131**: 5 câu bắt buộc |

---

## 2. Bảng quyết định

Nguyên tắc chung của Agent 0 cho đợt này: **ViScript không phải demo**. Thứ gì làm được và đo được trên máy hiện có thì làm thật; thứ gì cần môi trường chưa có thì **mở khoá môi trường trước**; thứ gì đổi ngữ nghĩa lớn thì **văn bản hoá trạng thái thật ngay** và tách thành dự án VS3 có thiết kế riêng, **không** tuyên bố "không bao giờ".

| Mục | Quyết định | Khác đề xuất của lead? | VD |
|---|---|---|---|
| Q1 Landlock | **Làm thật (a)**, có điều kiện: sau khi runner Linux chạy được (Q2) và `lsm` trong runner có `landlock`. Duyệt **có điều kiện** crate `landlock` (chỉ trong `vis-host-linux`, `cfg(target_os="linux")`). Trong lúc chờ: văn bản "cưỡng chế ở tầng runtime; kernel sandbox chưa có" | Lead đề xuất (c) trước. A0 chọn (c) **tạm thời** + (a) ngay khi có runner | 120 |
| Q2 Runner Linux | **(a) container trên máy dev**. Bước đầu là thăm dò công cụ có sẵn. Không tự cài phần mềm hệ thống; thiếu ⇒ BLOCKED-ENV báo người dùng | Giống | 120 |
| Q3 wasm | **Chỉ check trong VS2** (c). VS3 sẽ xét (b) "thư viện nhúng chạy trên wasip1" vì liên quan sandbox ứng dụng của ViFastOs | Giống, nhưng **không** đóng vĩnh viễn | 121 |
| Q4 Windows | **Chỉ check trong VS2** | Giống | 121 |
| Q5 TLS | **Ngoài VS2** (b), văn bản "HTTP chỉ loopback, chưa có HTTPS". Xét `rustls` ở VS3 sau khi có runner Linux | Giống, nhưng ghi "chưa", không ghi "ngoài phạm vi" | 122 |
| Q6 Async song song | **(a) văn bản hoá ngay** "async = task tuần tự có join, không song song, không suspend giữa thân hàm". (b) thành **dự án VS3** có tài liệu thiết kế riêng do Agent 0 viết | Giống | 123 |
| Q7 chan(0) | **Từ chối trong VS2** (giữ VS0505 lúc chạy, exit 1). **Không** tuyên bố vĩnh viễn: rendezvous đi cùng VS3 async | Khác: lead đề xuất "vĩnh viễn" | 123 |
| Q8 Siết caps | **(c)**: chương trình có dùng gói ⇒ mặc định siết; thoát bằng cờ tường minh. (b) ghi nợ VS3 | Giống | 124 |
| Q9 Gói | **(a)** ranges + resolver tối giản, không mạng. Ngữ nghĩa do A0 chốt ở VD-125 | Giống | 125 |
| Q10 LSP tệp phụ | **(a)** một-tài-liệu, đóng nợ #104 thành "không làm, có lý do" | Giống | 126 |
| Q11 Span file=0 | **(a)** audit + sửa | Giống | 127 |
| Q12 R1 | **(b)** luật là ≤1.500; bỏ mục tiêu nội bộ ≤1.000 | Giống | 128 |
| Q13 VBC1 mang file | **Không** trong VS2 | Giống | 129 |
| Q14 Đóng băng API std | **Có**, bằng tệp danh sách + test chặn | Giống, thêm cơ chế chặn | 129 |
| Q15 Cam kết tương thích | **Có** cho FUNCS, all_codes, thẻ VBC1. Luật ở VD-129 | Mới | 129 |
| §10(a) Value::Cap | Review có ca kiểm | Mới | 130 |
| §10(b) FsGuard TOCTOU | Review; sửa nếu không cần `unsafe`, nếu cần ⇒ ghi giới hạn có văn bản | Mới | 130 |
| §10(c) --grant so basename | **Không chấp nhận.** Đổi sang so **đường dẫn tuyệt đối đã canonicalize** | Khác: lead coi là "rộng có chủ đích" | 130 |
| §10(d) vis test chạy lại câu lệnh cấp tệp | Giữ nợ, không làm đợt này | Giống | - |
| §10(e) json_esc | Fuzz chuỗi ngẫu nhiên 100k | Mới | 130 |

### Định nghĩa "VS2 đóng" (VD-131)

VS2 **đóng** khi ST-119..ST-133 đều DONE hoặc BLOCKED-ENV có văn bản, và R23 xác nhận lại toàn bộ số đo. Khi đó 8 hạng mục ở ROADMAP mỗi hạng mục phải ở một trong ba trạng thái: **đã đóng (đo được)**, **chưa có trong VS2 (có văn bản, chuyển VS3)**, hoặc **chờ môi trường (BLOCKED-ENV, có lệnh tái lập)**. Không còn trạng thái "dở dang không tên".

---

## 3. Văn bản dán vào `viscript/agent/DECISIONS.md` (nguyên văn, nối cuối tệp)

```markdown
### VD-119 · Quy trình: Agent 0 soạn, Agent 2 dán (2026-11-16)
Agent 0 không ghi repo trực tiếp. Văn bản trong tệp QUYET-DINH-A0-<ngày>.md được Agent 2 dán nguyên văn vào tệp Agent 0 sở hữu, commit riêng [ST-00] ghi "theo QUYET-DINH-A0-<ngày>". Không diễn giải; không áp được nguyên văn thì BLOCKED. Task xong ghi "CHỜ DUYỆT Agent 0 · <hash> · <số đo>"; chỉ Agent 0 ghi APPROVED qua tệp quyết định kế tiếp. Task phụ thuộc được chạy tiếp khi task trước ở trạng thái CHỜ DUYỆT.

### VD-120 · Runner Linux bằng container + Landlock có điều kiện (2026-11-16)
Runner Linux chạy bằng container trên máy dev. Không tự cài phần mềm hệ thống; thiếu công cụ thì BLOCKED-ENV và báo người dùng. Crate `landlock` được duyệt CÓ ĐIỀU KIỆN: chỉ trong vis-host-linux, chỉ cfg(target_os="linux"), toàn bộ phụ thuộc bắc cầu có giấy phép thuộc allowlist D2-21 (kiểm bằng cargo metadata, ghi log); một phụ thuộc ngoài allowlist thì BLOCKED. Không thêm `unsafe` trong code của ta. Landlock bật bằng cờ `--sandbox` (opt-in trong VS2); nền không hỗ trợ trả exit 3 như --sandbox-check. Ca âm phải chứng minh KERNEL chặn (tiến trình con bỏ qua FsGuard vẫn bị EACCES), không phải runtime chặn. Trước khi có runner: văn bản "cưỡng chế ở tầng runtime; kernel sandbox chưa có".

### VD-121 · wasm và Windows: chỉ cargo check trong VS2 (2026-11-16)
Mức bằng chứng wasm32 và windows-msvc trong VS2 là "biên dịch kiểm tra" (VD-104). Không tuyên bố hỗ trợ. VS3 xét đưa vis-interp/vis-types chạy được trên wasm32-wasip1 như thư viện nhúng.

### VD-122 · HTTP: chưa có HTTPS trong VS2 (2026-11-16)
HTTP client chỉ nói HTTP/1.1 loopback, không TLS, không HTTP/2, không keep-alive. Đây là trạng thái "chưa có", không phải "ngoài phạm vi vĩnh viễn". rustls được xét ở VS3 sau khi có runner Linux.

### VD-123 · Async và chan(0) trong VS2 (2026-11-16)
Ngữ nghĩa VS2: async là task chạy tuần tự, một luồng, run-to-completion khi join; không song song; không suspend giữa thân hàm; không GC. chan(0) bị từ chối lúc tạo bằng VS0505 (exit 1). Cả hai là trạng thái VS2, được văn bản hoá trong thiết kế và ROADMAP. Async đồng thời thật + rendezvous + select/timeout là dự án VS3 có tài liệu thiết kế riêng do Agent 0 viết; không làm trong task 3 h.

### VD-124 · Siết caps mặc định cho chương trình dùng gói (2026-11-16)
Nếu đồ thị import của chương trình có ít nhất một module `pkg.*`, chế độ strict-caps bật mặc định. Thoát bằng cờ tường minh `--no-strict-caps`, khi đó in một dòng cảnh báo ra stderr (không ra stdout). Chương trình không dùng gói: hành vi giữ nguyên. Siết mặc định toàn cục là nợ VS3.

### VD-125 · Semver ranges + resolver tối giản (2026-11-16)
Phiên bản: MAJOR.MINOR.PATCH số nguyên; prerelease/build metadata bị từ chối có chẩn đoán. Ràng buộc: `^`, `~`, `=`, `>`, `>=`, `<`, `<=`; dấu phẩy là AND; không có toán tử thì hiểu là `^` (giống Cargo, kể cả luật ^0.x). Mỗi gói có đúng một phiên bản trong đồ thị (VS2). Resolver tất định: chọn phiên bản cao nhất thoả mọi ràng buộc, quay lui khi xung đột; không thoả được thì mã lỗi MỚI trong khối VS07xx kèm chuỗi ràng buộc gây xung đột. Lock hiện có còn thoả ràng buộc thì giữ nguyên (`pkg install` không đổi lock); `pkg update` mới giải lại. Lock định dạng cũ được đọc và ghi lại định dạng mới; ghi lại hai lần cho ra cùng byte. Không mạng, không chữ ký (VS3).

### VD-126 · LSP publish một tài liệu (2026-11-16)
LSP chỉ publish chẩn đoán cho tài liệu client đang mở; lỗi ở module phụ được báo qua vị trí import trong tài liệu đang mở. Nợ #104 đóng ở trạng thái "không làm, có lý do": publish cho URI client chưa mở gây hiển thị không nhất quán giữa các client.

### VD-127 · Span tổng hợp phải mang file (2026-11-16)
Mọi node HIR do lower sinh ra (không copy từ CST) phải mang file-id của node nguồn sinh ra nó. Có test duyệt HIR của mọi fixture đa tệp: không node nào thuộc module phụ có file = 0.

### VD-128 · R1 chỉ có một ngưỡng: ≤1.500 dòng/tệp (2026-11-16)
Bỏ mục tiêu nội bộ ≤1.000. Không mở task tách tệp chỉ để đạt con số không có trong luật.

### VD-129 · Đóng băng API std và cam kết tương thích (2026-11-16)
Ba danh sách là cam kết tương thích: hàm std (FUNCS), mã chẩn đoán (all_codes), thẻ VBC1. Chúng được ghi thành tệp viscript/agent/API-STD.txt, API-CODES.txt, API-VBC1.txt (Agent 0 sở hữu sau lần tạo đầu). Test so code với tệp. Luật: không xoá, không đổi tên, không đổi ý nghĩa, không tái dùng số mã/số thẻ. Thêm mới chỉ khi có VD nêu đích danh tên được thêm. Đổi ý nghĩa thẻ VBC1 bắt buộc lên định dạng mới (VBC2). Khoá cache bytecode phải đổi khi bảng thẻ đổi. VBC1 không mang file trong Span ở VS2.

### VD-130 · Kết luận review độc lập vòng 10+ (2026-11-16)
(a) Value::Cap không được tạo từ dữ liệu ngoài (json.decode, đọc tệp, đối số) và không lộ quyền khi in/serialize. (b) FsGuard: kiểm TOCTOU; sửa nếu không cần `unsafe`, nếu cần thì ghi giới hạn có văn bản trong HOST-BAO-DAM.md. (c) `--grant` cho Proc so khớp ĐƯỜNG DẪN TUYỆT ĐỐI ĐÃ CANONICALIZE, không so basename. Giá trị grant không có `/` được phân giải qua PATH một lần lúc khởi động thành đường dẫn tuyệt đối. (e) json_esc phải qua fuzz chuỗi ngẫu nhiên 100k, đối chiếu json.decode và python3 json.loads.

### VD-131 · Định nghĩa VS2 đóng + 5 câu bắt buộc (2026-11-16)
VS2 đóng khi ST-119..ST-133 DONE hoặc BLOCKED-ENV có văn bản và R23 xác nhận số đo. Mỗi hạng mục ROADMAP ở một trong ba trạng thái: đã đóng (đo được) / chưa có trong VS2 (chuyển VS3, có văn bản) / chờ môi trường (BLOCKED-ENV, có lệnh tái lập). Mọi báo cáo R## từ R23 phải có nguyên văn 5 câu: "Async không song song." "Kênh không rendezvous." "Net không TLS." "Quyền chỉ siết mặc định khi dùng gói." "LSP chỉ publish tài liệu đang mở." Câu nào hết đúng thì Agent 0 sửa bằng VD mới.
```

---

## 4. Hàng dán vào `viscript/agent/QUEUE.md` (nguyên văn, nối sau ST-118)

Định dạng 6 cột như hàng cũ. Tất cả **READY** trừ chỗ ghi phụ thuộc môi trường.

```markdown
| ST-119 | **Văn bản quản trị**: AGENTS.md (root, chỉ phần ViScript) + viscript/AGENTS.md theo Phụ lục A; STATE.md viết lại header ≤60 dòng (G1/G2/G3) | ST-118 | 1,5 h | **READY** | `grep -c 'VD-88' AGENTS.md viscript/AGENTS.md` = 0; header STATE.md có hash c1587398, vòng 23, task kế; check_report PASS; gate PASS |
| ST-120 | **Đồng bộ thiết kế**: docs/viscript/ViScript-thiet-ke.md + ROADMAP.md + HOST-BAO-DAM.md theo Phụ lục B/C/D (G5/G6, Q3..Q7, Q10, Q12) | ST-119 | 2,5 h | **READY** | 4 dòng trỏ G6 đã chèn; cổng doc-examples PASS; không có số đo viết tay mới trong docs/; gate PASS |
| ST-121 | **Thăm dò runner Linux** (Q2a): phát hiện docker/podman/colima; nếu có: Dockerfile + bin/vis-linux chạy `uname -r`, `cat /sys/kernel/security/lsm`, build | ST-118 | 1 h | **READY** | log thăm dò có sha256; có công cụ ⇒ container build xanh + in lsm; không có ⇒ BLOCKED-ENV + lệnh người dùng cần chạy |
| ST-122 | **Span tổng hợp mang file** (Q11, VD-127): test duyệt HIR fixture đa tệp + sửa các chỗ sinh node | ST-118 | 3 h | **READY** | test duyệt: 0 node module phụ có file=0; ≥4 ca mới chẩn đoán rơi trên node tổng hợp ở module phụ, JSON+TAP đúng tệp/dòng, cả interp và --vm; parity giữ phân hoạch |
| ST-123 | **--grant Proc theo đường dẫn tuyệt đối** (VD-130c) | ST-118 | 2 h | **READY** | red: bản sao `echo` ở thư mục tạm chạy được khi grant /bin/echo; sau sửa: VS0604 exit 2; ≥6 ca (symlink, PATH, tương đối, trùng tên) × interp/--vm |
| ST-124 | **Review FsGuard TOCTOU + Value::Cap** (VD-130a/b) | ST-118 | 3 h | **READY** | JOURNAL/st124-review.md: mỗi phát hiện có ca tái hiện hoặc lý do không tái hiện được; sửa không cần unsafe ⇒ sửa + ca; cần unsafe ⇒ ghi giới hạn vào HOST-BAO-DAM.md theo Phụ lục D2 |
| ST-125 | **Fuzz json_esc 100k** (VD-130e) | ST-118 | 1,5 h | **READY** | seed cố định; 100.000 chuỗi; roundtrip json.decode và python3 json.loads; lech=0; có ký tự điều khiển, U+0000, U+2028/2029, ngoài BMP, `"` `\` |
| ST-126 | **Đóng băng API + tương thích** (Q14/Q15, VD-129): tạo API-STD.txt, API-CODES.txt, API-VBC1.txt + test so + kiểm khoá cache | ST-118 | 2,5 h | **READY** | 3 tệp sinh từ code, đã sắp xếp; test đỏ khi xoá/đổi tên/thêm không khai; báo cáo: khoá cache có đổi khi bảng thẻ đổi không (đo bằng ca) |
| ST-127 | **Semver: kiểu Version + VersionReq + so khớp** (Q9a, VD-125) | ST-126 | 2,5 h | **READY** | ≥40 ca đơn vị phủ mọi toán tử + ^0.x + từ chối prerelease; không crate mới |
| ST-128 | **Resolver tất định + registry nhiều phiên bản** (VD-125) | ST-127 | 3 h | **READY** | ≥12 ca: chọn cao nhất, quay lui, xung đột ⇒ mã VS07xx mới (cập nhật đủ ≥7 chỗ đếm + API-CODES.txt theo VD-125); chạy 2 lần cùng byte |
| ST-129 | **pkg CLI dùng resolver + migration lock cũ** (VD-125) | ST-128 | 2,5 h | **READY** | install không đổi lock khi còn thoả; update giải lại; lock cũ ⇒ lock mới, ghi lại 2 lần cùng byte; pkg list --json có version_req |
| ST-130 | **Siết caps khi dùng gói** (Q8c, VD-124) | ST-129 | 3 h | **READY** | ≥6 ca × interp/--vm: có pkg + thiếu cap ⇒ VS0604 exit 2; --no-strict-caps ⇒ chạy + 1 dòng cảnh báo stderr; không pkg ⇒ hành vi cũ, 447 sem không đổi; >150 dòng ⇒ kế hoạch (VD-116) |
| ST-131 | **Chạy đầy đủ trên runner Linux**: cargo test, gate --fast, host suite 54 | ST-121 | 3 h | **READY nếu ST-121 DONE**, ngược lại BLOCKED-ENV | số đo Linux đặt cạnh macOS; mọi lệch được liệt kê, KHÔNG sửa trong task này |
| ST-132 | **Landlock thật** (Q1a, VD-120) | ST-131 | 3 h | **READY nếu lsm có landlock**, ngược lại BLOCKED-ENV | log giấy phép phụ thuộc; ≥8 ca cfg(linux) xanh trên runner, trong đó ≥3 ca âm chứng minh kernel chặn ở tiến trình con; macOS: --sandbox exit 3, ghi "chưa đo" |
| ST-133 | **R23 + soak 10 + ROADMAP chốt VS2** (VD-131) | tất cả trên | 1,5 h | **READY** | R23 có đủ số khoá, 5 câu bắt buộc, trạng thái 8 hạng mục theo 3 loại; soak 10/10; check_report PASS |
```

---

## 5. Thứ tự chạy liên tục + điều kiện dừng

**Thứ tự** (đã sắp để việc cần người dùng lộ ra sớm):

```
ST-00 (dán §3 + §4, commit) → ST-119 → ST-120 → ST-121 (thăm dò Linux)
→ ST-122 → ST-123 → ST-124 → ST-125 → ST-126
→ ST-127 → ST-128 → ST-129 → ST-130
→ ST-131 → ST-132 (bỏ qua nếu BLOCKED-ENV)
→ ST-133
```

**Điều kiện dừng / rẽ nhánh:**

- Gate đỏ không sửa được trong ngân sách ⇒ hoàn tác thay đổi chưa commit, BLOCKED + log, sang task **độc lập** kế tiếp. Task phụ thuộc vào task BLOCKED thì cũng BLOCKED, không thử.
- **2 BLOCKED liên tiếp không phải do môi trường** ⇒ dừng đợt, viết R23 sớm (ghi rõ dừng ở đâu).
- Flake ⇒ T-INFRA + log, chạy lại **tối đa một lần** chỉ để phân loại.
- Số đo khác kỳ vọng trong tệp này ⇒ **số đo thắng**, ghi rõ vào báo cáo, không ép cho khớp (bài học §12 bàn giao).
- Sau ST-126, ST-130, ST-133: soak 10 vòng tuần tự (không chạy song song build khác).

---

## 6. Brief từng task

Ràng buộc chung cho **mọi** task (không nhắc lại ở từng mục): red-first có sha256 (không có log đỏ thật thì ghi lý do); giữ các số khoá trừ khi task cho phép đổi (1066 test sẽ tăng, ghi số mới; sem 447/22, parity 398+49, FUNCS 52, all_codes 32, VBC1 53 chỉ đổi khi task nói rõ); không `unsafe`; không crate mới (trừ ST-132 theo VD-120); R1 ≤1.500; `../vis-dev gate --fast` PASS; `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff` lech=0 với task chạm front end/interp/VM; commit `[ST-###]` chỉ `viscript/` bằng `git commit --only -- <paths>`; fetch + `rebase --autostash` rồi push; cập nhật STATE + JOURNAL. Dùng `~/.cargo/bin/cargo`.

### ST-119 · Văn bản quản trị
- Root AGENTS.md dùng chung với dòng OS: **chỉ** sửa dòng/đoạn nói về ViScript; diff của tệp này phải dán đầy đủ vào báo cáo.
- Áp **Phụ lục A**. Nguyên tắc: tệp luật **không chứa số đếm** (số vòng, số test, VD mới nhất) để không lỗi thời nữa; chỉ trỏ tới nơi lấy số.
- STATE.md: viết lại **chỉ header** (≤60 dòng): HEAD dòng ViScript, vòng hiện tại (23), task đang làm/kế, số khoá đo được mới nhất kèm lệnh tái lập, BLOCKED hiện có. Phần lịch sử bên dưới giữ nguyên, thêm một dòng phân cách `--- LỊCH SỬ (không cập nhật) ---`.
- Red-first: không có log đỏ thật; ghi log `grep` trước/sau làm bằng chứng.

### ST-120 · Đồng bộ thiết kế
- Chèn nguyên văn **Phụ lục B** thành mục mới ở cuối docs/viscript/ViScript-thiet-ke.md.
- Tại 4 chỗ lệch G6 (cap[...], chan, assert, .vish fn main), chèn **một dòng** ngay dưới đoạn liên quan: `> Mức hiện thực VS2: xem mục "Mức hiện thực VS2".` Không sửa câu gốc.
- Áp Phụ lục C vào ROADMAP.md, Phụ lục D1 vào HOST-BAO-DAM.md.
- Nếu cổng doc-examples quét khối code trong docs: Phụ lục B **không** chứa khối code; nếu cổng vẫn đỏ ⇒ BLOCKED, báo nguyên nhân.

### ST-121 · Thăm dò runner Linux
- Chạy: `command -v docker podman colima nerdctl lima`, và với công cụ tìm thấy: `docker info` hoặc tương đương. Ghi log.
- **Không** tự cài phần mềm hệ thống. Không có gì ⇒ BLOCKED-ENV, ghi vào báo cáo đúng một khối "Người dùng cần làm" với lệnh cài đề xuất, rồi đi tiếp ST-122.
- Có công cụ: `viscript/infra/linux/Dockerfile` (image Rust chính thức, ghi digest đã kéo vào log), `viscript/bin/vis-linux` mount repo **chỉ đọc**, target dir là volume riêng. Lần này chỉ: build + in `uname -r` + nội dung `/sys/kernel/security/lsm` + có thể gọi syscall landlock hay không (do seccomp của container). Không chạy test đầy đủ (đó là ST-131).
- Nếu seccomp mặc định chặn landlock: **ghi lại**, không tự nới. Agent 0 sẽ quyết ở tệp sau.

### ST-122 · Span tổng hợp mang file
- Bước 1 (đỏ): viết test duyệt toàn bộ HIR của mọi fixture đa tệp, đếm node thuộc module phụ có file = 0. Ghi số đếm vào log đỏ. Nếu số đếm = 0 với fixture hiện có: viết thêm fixture cố ý dùng các cấu trúc mà lower phải sinh node (tự tìm trong lower bằng cách liệt kê mọi chỗ dựng Span không copy từ CST, dán danh sách vào JOURNAL).
- Bước 2: sửa từng chỗ dựng để lấy file của node nguồn.
- ≥4 ca sem mới có chẩn đoán rơi đúng node tổng hợp ở module phụ. Thêm ca sem ⇒ cập nhật đủ ≥7 chỗ đếm; báo phân hoạch parity mới (đừng tự cộng, đo).

### ST-123 · --grant Proc theo đường dẫn tuyệt đối
- Red: tạo thư mục tạm, chép `/bin/echo` vào đó, chạy chương trình gọi bản sao với `--grant p=/bin/echo` ⇒ log cho thấy hiện được phép.
- Luật mới theo VD-130c. Ca tối thiểu: đường dẫn tuyệt đối khớp; bản sao trùng tên bị chặn; symlink tới binary được grant (canonicalize hai phía); grant không có `/` phân giải qua PATH; grant đường dẫn tương đối; binary không tồn tại lúc khởi động ⇒ chẩn đoán rõ.
- Ca cũ của host suite đổi kỳ vọng chỉ khi ca đó **dựa vào** so basename; liệt kê từng ca đổi + lý do trong báo cáo (đây là đổi ngữ nghĩa có VD, không phải sửa kỳ vọng cho xanh).

### ST-124 · Review FsGuard TOCTOU + Value::Cap
- Đây là task **phân tích có ca**, không phải viết tính năng. Kết quả chính là JOURNAL/st124-review.md.
- FsGuard: mô tả chuỗi kiểm-rồi-mở hiện tại (dòng code), thử tái hiện đổi symlink giữa kiểm và mở bằng ca có luồng phụ (lặp có giới hạn, không phụ thuộc thời gian tuyệt đối). Có cách sửa không cần `unsafe` (ví dụ mở rồi kiểm lại metadata của handle) ⇒ sửa + ca. Không có ⇒ ghi giới hạn (Phụ lục D2).
- Value::Cap: chứng minh bằng ca rằng json.decode, đọc tệp, đối số dòng lệnh, `to json`, `print`, so sánh không tạo hoặc lộ Cap. Mỗi mục: ca + kết quả.

### ST-125 · Fuzz json_esc
- Bộ sinh có seed cố định, ghi seed vào log. Đối chiếu ba phía: chuỗi gốc, json.decode(escape), python3 json.loads(escape). Lệch nào cũng là lỗi; sửa json_esc, không sửa bộ sinh để né.

### ST-126 · Đóng băng API
- Sinh 3 tệp từ code (một mục một dòng, sắp xếp ổn định; FUNCS ghi cả chữ ký). Lần tạo đầu do Agent 2 làm theo ủy quyền VD-119; sau đó tệp thuộc Agent 0.
- Test: code ≠ tệp ⇒ đỏ, thông báo nêu rõ "thêm/xoá/đổi" mục nào và nhắc VD-129.
- Kiểm khoá cache: làm một ca đổi bảng thẻ giả lập trong test (không đổi code thật), xem cache có bị dùng nhầm không. Không đổi ⇒ sửa nếu ≤150 dòng, ngược lại kế hoạch.

### ST-127..ST-129 · Gói
- Ngữ nghĩa theo VD-125, không tự thêm toán tử khác. Đo định dạng lock hiện tại **trước** khi thiết kế định dạng mới; dán ví dụ lock cũ và mới vào JOURNAL.
- Mã lỗi mới: chọn số kế tiếp trong khối VS07xx, thêm vào API-CODES.txt (được phép vì VD-125 nêu đích danh), explain 3 ngôn ngữ, cập nhật đủ chỗ đếm. Báo all_codes mới.

### ST-130 · Siết caps khi dùng gói
- Mục tiêu và tiêu chí ở hàng QUEUE. Cách cài đặt do Agent 2 đo và chọn; nếu ước lượng >150 dòng thì viết kế hoạch (VD-116) và dừng task, không làm nửa vời.

### ST-131 · Chạy đầy đủ trên Linux
- Chỉ đo và liệt kê lệch. Mỗi lệch: ca, đầu ra macOS, đầu ra Linux, phỏng đoán nguyên nhân có ghi "phỏng đoán". Không sửa trong task này; Agent 0 mở task sửa sau.

### ST-132 · Landlock thật
- Trước khi thêm crate: `cargo metadata` liệt kê giấy phép toàn bộ phụ thuộc bắc cầu, so với allowlist D2-21, ghi log. Một phụ thuộc ngoài allowlist ⇒ BLOCKED.
- Phạm vi: chỉ hệ tệp (Dir/File đọc/ghi theo cap khai báo). Net vẫn cưỡng chế ở runtime. Tiến trình con thừa hưởng ruleset: ghi rõ đây là bằng chứng "sandbox con" ở mức hệ tệp.
- Ca âm kernel: tiến trình con thử đọc tệp ngoài ruleset bằng đường không qua FsGuard ⇒ phải nhận PermissionDenied.

### ST-133 · R23
- Chạy lại toàn bộ §6 của bàn giao (8 lệnh) + số đo Linux nếu có. Báo cáo theo VD-131. ROADMAP: mỗi hạng mục một trong ba trạng thái.

---

## 7. Phụ lục văn bản (dán nguyên văn)

### Phụ lục A · AGENTS.md (root và viscript/)

**A1.** Mọi chỗ ghi `VD-01..VD-88` (hoặc dải VD cố định khác) trong phần ViScript, thay bằng:

```markdown
VD-01 trở đi trong viscript/agent/DECISIONS.md (số mới nhất: `grep -o 'VD-[0-9]\+' viscript/agent/DECISIONS.md | sort -t- -k2 -n | tail -1`)
```

**A2.** Đoạn mô tả tiến độ dòng ViScript (root §8 "VÒNG 1..9 đều DONE"; viscript/AGENTS.md dòng 30-32 mô tả vòng mới nhất), thay **chỉ phần ViScript** bằng:

```markdown
**Trạng thái dòng ViScript**: không ghi ở tệp này. Đọc theo thứ tự: viscript/agent/ROADMAP.md (8 hạng mục × mức thật), báo cáo mới nhất `ls viscript/agent/reports/ | sort -V | tail -1`, rồi viscript/agent/QUEUE.md. Tệp luật không chứa số đếm (số vòng, số test, VD mới nhất) để không lỗi thời.
**Quy trình hiện hành**: VD-119 (Agent 0 soạn, Agent 2 dán nguyên văn, commit [ST-00]).
```

### Phụ lục B · Mục mới cuối docs/viscript/ViScript-thiet-ke.md

```markdown
## Mức hiện thực VS2 (đồng bộ 2026-11-16)

Mục này mô tả ngữ nghĩa đã hiện thực, không chứa số đo. Số đo nằm trong viscript/agent/reports/. Khi mục này khác phần thiết kế phía trên, mục này đúng với code hiện tại; phần phía trên là đích thiết kế.

- **Quyền cap[...]**: là kiểu thật từ vòng 11 (VD-106). Chỉ cưỡng chế khi chương trình khai báo, hoặc khi chạy với --strict-caps. Cưỡng chế ở biên host (tầng runtime), vi phạm báo VS0604 exit 2. Kernel sandbox (Landlock) chưa có; --sandbox-check chỉ thăm dò.
- **Async**: task chạy tuần tự, một luồng, run-to-completion khi join. Không song song, không suspend giữa thân hàm, không GC. Đồng thời thật là đích VS3.
- **Kênh**: chan(n) với n ≥ 1 có send/recv/close. chan(0) bị từ chối lúc tạo bằng VS0505. Rendezvous, select và timeout là đích VS3.
- **assert <điều kiện>**: có từ ST-83.
- **.vish và fn main**: trong tệp .vish, fn main chỉ dùng để khai báo cap; thân có câu lệnh thì báo VS0201 (ST-93b). Thụt lề cấp tệp trong .vish có ngữ nghĩa; vis fmt không chuẩn hoá thụt lề .vish.
- **HTTP**: HTTP/1.1 loopback, có chunked, redirect GET có giới hạn, HEAD, timeout đọc. Chưa có TLS, HTTP/2, keep-alive.
- **Gói**: registry cục bộ, lock có SHA-256. Chưa có registry mạng và chữ ký.
- **LSP**: chỉ publish chẩn đoán cho tài liệu đang mở (VD-126).
- **Nền tảng**: chạy thật trên macOS; Linux, Windows, wasm32 ở mức biên dịch kiểm tra (VD-104, VD-121).
```

### Phụ lục C · ROADMAP.md

Thêm mục mới ngay sau bảng §A, **không** sửa bảng §A (bảng là lịch sử mức đo của R22):

```markdown
### Quyết định Agent 0 cho điều kiện đóng (2026-11-16, VD-119..VD-131)

| # | Hạng mục | Trạng thái sau quyết định | Việc |
|---|---|---|---|
| 1 | Async | Chưa có trong VS2: song song, suspend, GC (VD-123) | VS3 thiết kế riêng |
| 2 | Kênh | Chưa có trong VS2: rendezvous, select, timeout (VD-123) | VS3 |
| 3 | Net/HTTP | Chưa có trong VS2: TLS, HTTP/2, keep-alive (VD-122) | VS3 |
| 4 | Quyền | Siết khi dùng gói: ST-130. Landlock: ST-132, chờ runner Linux | ST-130, ST-131, ST-132 |
| 5 | LSP | Một tài liệu là quyết định cuối (VD-126); nợ #104 đóng | không |
| 6 | Gói | Ranges + resolver: ST-127..129. Mạng/chữ ký: VS3 | ST-127..129 |
| 7 | Đa nền tảng | Linux: ST-121, ST-131. Windows, wasm: chỉ check trong VS2 (VD-121) | ST-121, ST-131 |
| 8 | Hợp chuẩn host | Nền thứ hai = Linux qua ST-131 | ST-131 |

R1: ngưỡng duy nhất ≤1.500 dòng/tệp (VD-128); bỏ mục tiêu nội bộ ≤1.000.
```

### Phụ lục D · HOST-BAO-DAM.md

**D1.** Thêm vào phần "cấm ngôn từ":

```markdown
- Cấm nói "async song song", "chạy đồng thời", "rendezvous", "HTTPS", "sandbox kernel" khi mô tả VS2 (VD-122, VD-123, VD-120).
- Cấm nói "hỗ trợ Windows/wasm/Linux" khi chỉ có cargo check (VD-104, VD-121).
```

**D2.** (Chỉ dùng trong ST-124 nếu TOCTOU cần `unsafe` để sửa) thêm:

```markdown
- Giới hạn đã biết: FsGuard kiểm đường dẫn rồi mới mở; trong khoảng giữa hai bước, đường dẫn có thể bị thay bởi symlink nếu kẻ tấn công ghi được vào thư mục cha. Sửa triệt để cần thao tác theo handle (cần unsafe hoặc crate ngoài), chưa làm trong VS2. Landlock (khi có) chặn trường hợp này ở mức kernel trên Linux.
```

---

## 8. Báo cáo gửi về Agent 0

Mỗi task: hash, lệnh + số đo, log đỏ + sha256, danh sách tệp đổi, nợ mới (đánh số tiếp 105+), lệch so với tệp này (nếu có). Cuối đợt: R23 theo VD-131. Gửi R23 + danh sách hàng "CHỜ DUYỆT" cho Agent 0 để ra tệp quyết định đợt 24.
