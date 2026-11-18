<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# CHO AGENT 0 — TIẾN ĐỘ DỰ ÁN + VIỆC CẦN REVIEW VÀ QUYẾT ĐỊNH

**Ngày soạn**: 2026-11-18 (13:35 JST) · **HEAD**: `6a32233` · **Người soạn**: Agent 2 (dòng ViScript)
**Người nhận**: Agent 0 — người ra quyết định, **không đọc mã và tài liệu của dự án**.

> **Tệp này tự chứa.** A0 không cần mở bất kỳ tệp nào khác. Mọi con số đều ghi kèm nguồn
> (tên tệp/log) để Agent 1/2 đối chiếu, nhưng **không cần đọc** mới hiểu được vấn đề.
> Mỗi mục quyết định có: **chuyện gì → bằng chứng → lựa chọn → đề xuất → nếu không quyết thì sao**.

---

## 0. ĐỌC NHANH (TL;DR)

**Dự án đang ở đâu**: ViScript (ngôn ngữ lập trình tiếng Việt, viết bằng Rust) đã **đóng VS2 — 8/8 hạng mục
kỹ thuật** (bản 2.0 chạy được: async, kênh, HTTP, quyền, LSP, gói, đa nền tảng, hợp chuẩn host). Đã **tách
thành repo riêng** `github.com/vifast-oss/ViScript` (public). **6 tài liệu thiết kế VS3** (bản 3.0) đã được
A0 duyệt có sửa và đã áp xong. Hiện đang chạy **pha 1 công thái học** — 30 task sửa ngôn ngữ cho dễ dùng
hơn, **bắt buộc xong trước VS3**; đã xong **18 hàng**, còn **4 nhóm quy tắc ≈ 20 giờ**.

**5 việc A0 cần quyết để không chặn**:

| # | Việc | Vì sao gấp |
|---|---|---|
| **Q1** | `#147` — macOS chậm hơn Linux **5,70×**: sửa hay chấp nhận là giới hạn nền? | Mỗi lần chạy cổng trên mac mất **326 s** thay vì 39 s; là nợ mở có số đo hiệu năng rõ ràng |
| **Q2** | Số mặc định an toàn cho HTTP (timeout 10 s/30 s, redirect 5, body 8 MiB) | VS3-3 (TLS) **không thể bắt đầu** nếu chưa chốt số |
| **Q3** | Cho phép sửa **1 dòng** cho Windows (`check.rs`) + ca khoá | Cổng Windows đang dừng ở bước 5/10 vì lỗi này |
| **Q4** | Xác nhận **16 dấu [SỬA]** của VD-178 đã áp đúng → **mở VS3** | VS3 chỉ mở sau khi A0 xác nhận (VD-148 + VD-178) |
| **Q5** | Hai tài liệu A0 sở hữu cần cập nhật (`HOST-BAO-DAM.md` §4, `docs/ViScript-thiet-ke.md`) | Đang mâu thuẫn với quyết định mới (mô hình gói, macOS, hardlink, Windows) |

**9 việc còn lại** (Q6–Q14) không chặn nhưng nên quyết trong đợt này — xem §3.3.

---

## 1. DỰ ÁN LÀ GÌ (30 giây)

**ViScript** là ngôn ngữ lập trình **dễ học cho người Việt**: cú pháp gọn như Python, kiểm tra kiểu như Rust,
mọi tên biến và thông báo lỗi bằng tiếng Việt. Bỏ hẳn con trỏ, `unsafe`, kế thừa lớp. Mục tiêu: học sinh
cấp 3 đọc 10 phút là viết được chương trình đầu tiên.

- **Mã nguồn**: Rust, ~8 crate, chạy trên macOS / Linux / Windows (Windows mới ở mức "chỉ đo").
- **Quy mô hiện tại**: **1 211 test** trong **136 suite**, tất cả xanh; **218 hàng công việc** đã đóng;
  **29 báo cáo vòng** (R1..R29); **148 dòng nợ** được theo dõi.
- **Cách làm việc**: có 3 agent — **Agent 0** (ra quyết định, không đọc mã), **Agent 1** (điều phối, soạn
  tài liệu), **Agent 2** (viết mã, chạy cổng, ghi log). Mọi thay đổi ngôn ngữ phải qua A0.

**Thuật ngữ dùng trong tệp này**:

| Từ | Nghĩa |
|---|---|
| **VS2 / VS3** | Bản 2.0 / bản 3.0 của ngôn ngữ. VS2 đã đóng; VS3 đang thiết kế. |
| **ST-nnn** | Một hàng công việc (task) trong sổ `QUEUE.md`. Mỗi task ≤ 3 giờ. |
| **VD-nnn** | Một quyết định của Agent 0 (đánh số tăng dần; mới nhất **VD-181**). |
| **Cổng (gate)** | Bộ kiểm tra bắt buộc trước khi commit: định dạng, lint, luật cấu trúc, build, tài liệu, test. |
| **L0** | Tầng test nhanh nhất trong cổng (1 211 test hiện tại). |
| **Nợ** | Khuyết điểm đã biết, có số, theo dõi trong `SO-NO.md` (148 dòng). |
| **Pha 1 công thái học** | 30 task sửa ngôn ngữ cho dễ dùng (R1..R24), **phải xong trước VS3**. |
| **Đỏ-trước** | Viết test cho hành vi mới, chạy thấy **đỏ** (đúng lý do), rồi mới sửa mã cho **xanh**. |

---

## 2. TIẾN ĐỘ ĐO ĐƯỢC (số liệu, không phải cảm nhận)

### 2.1 Số khoá tại HEAD `6a32233`

| Chỉ số | Số | Nguồn |
|---|---:|---|
| Test trong cổng (L0) | **1 211 / 0 đỏ** | `JOURNAL/st198-199-gate-macos-lan2.log` |
| Suite | **136** | như trên |
| Hàng công việc đã đóng | **218 / 218** (0 READY, 0 BLOCKED, 0 REVIEW) | `agent/QUEUE.md` |
| Báo cáo vòng | **29** (R1..R29) | `agent/reports/` |
| Dòng nợ | **148** = **88 mở** / **37 đóng** / **21 chuyển VS3** / 1 không làm / 1 điều hướng | `agent/SO-NO.md` §4 |
| Mã lỗi có tên | **50** | `agent/API-CODES.txt` |
| Tài liệu thiết kế VS3 | **6** (đã duyệt có sửa) | `JOURNAL/vs3-qd-1..6.md` |
| Cổng macOS | **PASS 10/10**, 326 s | `st170-gate-macos.log` |
| Cổng Linux (`ps`) | **PASS 10/10**, 39 s | `st170-gate-ps.log` |
| Cổng Windows (`psvn`) | **9/10 bước**, dừng ở L0 | `st170-gate-psvn.log` |

> **Lưu ý sổ nợ**: dòng tóm tắt §8 của `SO-NO.md` còn ghi **147 dòng / 89 mở / 34 đóng** — **đã cũ** so với
> bảng chính (148/88/37). Đây là việc sổ của Agent 2, không cần A0 quyết; ghi ra để A0 không bị lệch số.

### 2.2 Đã đóng gần đây (đợt 29 → 18/11)

| Việc | Kết quả |
|---|---|
| **Tách repo** (ST-171) | Repo `ViScript` riêng, giữ **459 commit** lịch sử, public trên GitHub; cổng PASS trên mac + Linux |
| **Nợ #146** (đua symlink macOS) | **ĐÓNG**: vá 1 dòng + ca tất định 100/100 trên mac **và** Linux; 6 000 000 vòng 0 đỏ |
| **Nợ #147** (mac chậm hơn Linux) | **MỞ**: 5,70× > ngưỡng 3×; nguyên nhân là **tạo tiến trình/syscall của macOS**, không phải CPU |
| **6 tài liệu VS3** | Duyệt có sửa (VD-178); 16 dấu [SỬA] đã áp xong (ST-178) |
| **Mô hình gói** | Chốt **phi tập trung kiểu Go** (VD-180) + tên `vifast.org/vis/<tên>` (VD-181) |
| **Pha 1 công thái học** | **18 hàng ST-179..ST-196** đã xong: P0, R9, R6, R17, R1, R3, R8, R5a, R5c, R11, R12, R16, R7, R19b, R19c, R20a, R20b |
| **PR #26/#27** (từ ViFastOs) | Port sang repo mới, giữ tác giả; 6/6 và 10/10 ca đỏ-trước → xanh |
| **Brand** | Icon ViScript + ViShell (SVG + PNG 5 kích thước), logo README |

### 2.3 Còn lại của pha 1 (bắt buộc trước VS3)

| Nhóm | Nội dung | Task còn | Ước lượng |
|---|---|---|---:|
| **R2** | `Option`/`Result` là enum chuẩn; `None` là biến thể; **bỏ từ khoá `none`**; cho phép `T??` | 3 | 8 h |
| **R4** | `Error` là record + `Json` là enum + `AnyError`; mọi hàm std trả `Result[T, Error]` | 3 | 8 h |
| **R5b** | `{…}` **chỉ là map**; dựng record bằng `{ten: …}` ⇒ lỗi `VS0205` có gợi ý | 1 | 2 h |
| **R5d** | `--b` chỉ ở chế độ lệnh/`cmd`; báo xung đột tên `std.str`/`str`/`str(x)` | 1 | 2,5 h |
| **Tổng** | | **8** | **≈ 20,5 h** |

Sau pha 1: **VS3-QĐ-1 (async song song thật)** → QĐ-2/3/4 → QĐ-6 → QĐ-5 phần còn lại.

---

## 3. VIỆC CẦN A0 QUYẾT ĐỊNH

### 3.1 NHÓM A — ĐANG CHẶN (quyết để chạy tiếp)

---

#### Q1 · `#147` — macOS chậm hơn Linux **5,70×**: sửa hay chấp nhận?

**Chuyện gì**: Dự án đo hiệu năng trên 3 nền. macOS (máy dev) chạy **chậm hơn Linux 5,70×** khi chạy
cùng một bộ kiểm tra. Ngưỡng A0 đã đặt (VD-177 §2b) là **3×**; vượt ngưỡng ⇒ nợ **#147** phải mở.

**Bằng chứng** (đo có khoá máy, máy rảnh, cùng HEAD):

| Phép đo | macOS | Linux | Tỉ lệ |
|---|---:|---:|---:|
| Soak 10 vòng (bộ kiểm tra đầy đủ) | **1 489 s** | **261 s** | **5,70×** |
| Cổng `--fast` | 326,2 s | 39,2 s | 8,32× |
| `vis-dev check` (kiểm tra tĩnh) | 190,0 s | 12,3 s | **15,5×** |
| ├ `report-check` (gọi `git` từng báo cáo) | 111,9 s | 4,9 s | **22,8×** |
| ├ `rules` (một `wc`/`grep` mỗi tệp, 873 tệp) | 49,3 s | 6,0 s | **8,3×** |
| `cargo test` (biên dịch + chạy test) | 111,2 s | 25,1 s | 4,42× |
| Tạo tiến trình trần (20 lần) | 15,4 ms/lần | 1,6 ms/lần | **9,6×** |

**Nguyên nhân đo được**: **không phải CPU** — là **chi phí tạo tiến trình và syscall của macOS**. Các script
kiểm tra gọi **một tiến trình cho mỗi tệp** (873 tệp × `wc`/`grep`; mỗi báo cáo × `git`), nên macOS trả giá
gấp ~10 lần Linux cho mỗi lần tạo tiến trình.

**Lựa chọn**:

| | Cách làm | Ưu | Nhược | Công |
|---|---|---|---|---|
| **A** | **Gộp** `rules` thành **một** lượt `find`+`awk`/python; gom lệnh `git` của `report-check` thành **một** lượt. Đo lại. | Bỏ được **~150 s trong 190 s** của `check` trên mac; không đụng mã sản phẩm | Chỉ sửa script dev; nếu vẫn >3× thì vẫn phải quyết tiếp | 2 h |
| **B** | **Chấp nhận**: ghi "macOS phạt syscall là **giới hạn nền**", đóng #147 kèm văn bản. | Không tốn công; trung thực | Cổng mac vẫn 326 s/task ⇒ mỗi task chậm ~5 phút | 0,5 h |
| **C** | **A rồi B**: làm A trước, đo lại; nếu vẫn >3× thì chấp nhận B. | Có số liệu trước khi kết luận; không tự lừa mình | Tốn 2,5 h | 2,5 h |

**Đề xuất**: **(C)**. Làm A (chỉ sửa script dev, **0 dòng mã sản phẩm**), đo lại; nếu vẫn >3× thì đóng #147
bằng văn bản "giới hạn nền macOS" — **không** hạ ngưỡng, **không** đổi cách đo.

**Nếu không quyết**: #147 tiếp tục mở; mỗi task trên mac vẫn tốn thêm ~5 phút cổng; không chặn tính năng.

---

#### Q2 · Số mặc định an toàn cho HTTP (VS3-QĐ-3)

**Chuyện gì**: VS3 sẽ thêm **TLS + keep-alive** cho HTTP. Thiết kế QĐ-3 yêu cầu **mặc định an toàn bắt
buộc** (timeout, giới hạn redirect, giới hạn body). Agent 1 đã đề xuất số; **A0 phải chốt** thì VS3-3 mới
được bắt đầu.

**Bằng chứng**: `JOURNAL/vs3-qd-3.md:14` ghi rõ "**đề xuất, chờ A0 chốt**"; chưa dùng số này để "đạt" gì.

**Số đề xuất**:

| Tham số | Đề xuất | Hành vi khi vượt |
|---|---:|---|
| `timeout_connect` | **10 s** | lỗi rõ ràng |
| `timeout_read` | **30 s** | lỗi rõ ràng |
| `max_redirect` | **5** | chỉ GET (giữ như hiện tại) |
| `max_body` | **8 MiB** | lỗi rõ, **không** cắt im lặng |

**Lựa chọn**: (A) chốt đúng số đề xuất; (B) đổi số (A0 nêu số); (C) hoãn tới khi làm VS3-3.

**Đề xuất**: **(A)** — các số này khớp với hành vi hiện có của VS2 (body 8 MiB, redirect 5, timeout đọc 2 s
ở VS2; VS3 nâng lên 30 s cho TLS) và không mở thêm bề mặt tấn công.

**Nếu không quyết**: VS3-3 (TLS) không thể bắt đầu; VS3-1/2/4 vẫn chạy được.

---

#### Q3 · Cho phép sửa **1 dòng** cho Windows (`check.rs`)

**Chuyện gì**: Trên Windows, cờ `vis check --syntax` **không có tác dụng** vì mã tách tên tệp bằng
`path.rsplit('/')` — trên Windows đường dẫn dùng `\`, nên phần "tên tệp" luôn khác rỗng và nhánh `--syntax`
bị bỏ qua. Hệ quả: **self-test ca 7 đỏ** (11/12), cổng Windows **dừng ở bước 5/10**.

**Bằng chứng**: cùng một tệp gọi bằng **tên tương đối** trong thư mục hiện tại ⇒ `ok 1 - lf.vis` (exit 0);
gọi bằng **đường dẫn tuyệt đối** `C:\…` ⇒ `not ok … # VS0301`. `--syntax` trên Windows hành xử y như không
có cờ. Log: `JOURNAL/st147-chan-doan-psvn.log` (sha256 khớp hai phía).

**Vì sao phải hỏi A0**: VD-146 quy định "Windows **chỉ đo**, không sửa sản phẩm cho Windows"; VD-150 chỉ
cho sửa **hạ tầng cổng**, không cho sửa mã sản phẩm. Đây là **lỗi sản phẩm thật**, không phải lỗi test.

**Lựa chọn**:

| | Cách làm | Ưu | Nhược | Công |
|---|---|---|---|---|
| **A** | Cho phép sửa **1 dòng** (`Path::file_name`, hoặc tách cả `'/'` và `'\\'`) **kèm ca khoá** cho **cả hai** dấu phân cách, trong VS3 | Sửa đúng gốc; ca khoá chống tái phát; mở đường cho cổng Windows | Phải mở 1 task VS3 nhỏ | 1 h |
| **B** | Để tới VS3-5 (Windows "hỗ trợ thật") | Không phải quyết bây giờ | Cổng Windows tiếp tục dừng ở bước 5; self-test 11/12 | 0 |
| **C** | Sửa ngay trong VS2 | Nhanh | Trái VD-146 (Windows chỉ đo) | 1 h |

**Đề xuất**: **(A)** — cấp **VD + task VS3** cho đúng một dòng + ca khoá hai dấu phân cách. Không sửa kỳ
vọng ca nào; không `#[ignore]`.

**Nếu không quyết**: cổng Windows mãi dừng ở bước 5; không thể tuyên bố "hỗ trợ thật" (VS3-5).

---

#### Q4 · Xác nhận **16 dấu [SỬA]** của VD-178 → **mở VS3**

**Chuyện gì**: VD-178 duyệt **6 tài liệu thiết kế VS3** theo kiểu "duyệt **có sửa**": 9 nhóm quyết định
(QĐ-1…QĐ-6 + C1 + C2 + C3), bên trong có **16 dấu [SỬA]**. ST-178 đã áp **đủ 16 dấu** và ghi "đã sửa:
tệp:dòng" cho từng dấu. **VD-148** đặt điều kiện: VS3 chỉ bắt đầu tính năng khi A0 đã ban hành tài liệu
thiết kế VS3 — A0 đã duyệt (VD-178), nên việc còn lại là **xác nhận 16 dấu [SỬA] đã áp đúng** để chính
thức mở VS3.

**Bằng chứng**: `JOURNAL/st178-bao-cao.md` liệt kê đủ 16 dấu + C1/C3 + các mục "áp dụng"; `check_report
--all` PASS; **0 dòng mã** bị sửa.

**Lựa chọn**: (A) xác nhận và **mở VS3**; (B) yêu cầu Agent 1 gửi diff tóm tắt trước; (C) hoãn.

**Đề xuất**: **(A)** — xác nhận; A0 có thể kiểm ngẫu nhiên sau. Kèm xác nhận cách hiểu "9 mục [SỬA]" =
**9 nhóm quyết định / 16 dấu** (xem Q14).

**Nếu không quyết**: VS3 bị khoá; chỉ còn pha 1 chạy được.

---

#### Q5 · Hai tài liệu **A0 sở hữu** cần cập nhật

**Chuyện gì**: Có 2 tài liệu chỉ A0 được sửa, hiện **mâu thuẫn** với quyết định mới:

| Tệp | Cần sửa gì | Vì sao |
|---|---|---|
| `agent/HOST-BAO-DAM.md` §4 | Bỏ câu "macOS còn khe hở bản chất"; ghi **hardlink** là giới hạn còn lại; ghi **Windows mức bảo đảm thấp hơn** | QĐ-4 mới (đi từng thành phần đường dẫn) làm macOS **hết** khe hở bản chất; #146 đã đóng |
| `docs/ViScript-thiet-ke.md` | Cập nhật mô hình gói: bỏ "registry sau này", "vis.lock khoá phiên bản + hash nội dung"; ghi mô hình **phi tập trung kiểu Go** + bảng host → vai trò | VD-180/181 đã thay mô hình cũ |

**Lựa chọn**: (A) A0 tự sửa; (B) A0 **uỷ quyền** Agent 1 soạn, A0 duyệt diff; (C) hoãn.

**Đề xuất**: **(B)** — Agent 1 soạn đúng phạm vi, gửi diff; A0 duyệt. Nhanh hơn và vẫn giữ quyền quyết.

**Nếu không quyết**: tài liệu bảo đảm an toàn và thiết kế ngôn ngữ **nói sai** so với mã; người mới đọc sẽ
hiểu sai.

---

### 3.2 NHÓM B — CẦN QUYẾT TRƯỚC KHI MỞ VS3

---

#### Q6 · Thứ tự còn lại của pha 1

**Chuyện gì**: Pha 1 còn **4 nhóm** (R2, R4, R5b, R5d ≈ 20,5 h). Thứ tự C3 của VD-178 là
`P0 → R9(+C1) → R6 → R17 → R1+R2 → R3 → R8 → R4 → R5 → R11 → R12 → R16 → R7 → R19b → R19c`.
Đã xong: P0, R9, R6, R17, R1, R3, R8, R11, R12, R16, R7, R19b, R19c, R5a, R5c. **Còn lại: R2, R4, R5b, R5d.**

**Lựa chọn**:

| | Thứ tự | Ưu | Nhược |
|---|---|---|---|
| **A** | **R2 → R4 → R5b → R5d** (đúng C3) | R2/R4 là nền (Option/Result, Error); R5b/R5d phụ thuộc R4 | R2/R4 là 2 task lớn nhất (8 h mỗi cái) |
| **B** | R5b → R5d → R2 → R4 | Xong nhanh 2 task nhỏ, có cảm giác tiến | R5b/R5d **cần** R4 (Error) ⇒ làm trước sẽ phải làm lại |
| **C** | R2 → R5b → R5d → R4 | Cân bằng | R5b/R5d vẫn phụ thuộc R4 |

**Đề xuất**: **(A)** — R2 và R4 là nền của mọi thứ sau; R5b/R5d làm sau R4.

**Nếu không quyết**: Agent 2 đang làm R2 (đã bắt đầu); không chặn, nhưng nếu A0 muốn đổi thứ tự thì phải
nói **trước khi R2 xong**.

---

#### Q7 · Windows "hỗ trợ thật" — kế hoạch VS3-5

**Chuyện gì**: Windows hiện **"chỉ đo"** (VD-146). VS3-QĐ-5 định nghĩa "hỗ trợ thật" = chạy **native**
(không cần Git Bash/MSYS), host **54/54**, `cargo test` **0 đỏ** (hoặc chỉ còn danh sách kỳ vọng theo nền
**A0 duyệt**), cổng **10/10**, có **CI runner Windows**.

**Bằng chứng**: **105 ca đỏ** đã phân loại **48 (a) / 15 (b) / 42 (c)**; host **39/54**; cổng **9/10**.

**Lựa chọn**: (A) theo đúng VS3-5a..5e; (B) giảm phạm vi (chỉ 5a + 5d); (C) hoãn.

**Đề xuất**: **(A)** — 5a (chương trình trợ giúp do repo tự build thay `/bin/*`) và 5d (CI runner) **chạy
song song được với pha 1**; 5b phụ thuộc R6 (đã xong); 5c điều tra 15 ca (b).

**Nếu không quyết**: Windows tiếp tục "chỉ đo"; không chặn VS3 khác.

---

#### Q8 · Mô hình gói — triển khai

**Chuyện gì**: VD-180/181 đã chốt mô hình **phi tập trung kiểu Go**: tên gói = **đường dẫn**
(`vifast.org/vis/<tên>`, `github.com/<chủ>/<repo>`); phiên bản = **tag git semver**; chọn bằng **MVS**
(không bộ giải ràng buộc); tin cậy bằng **`vis.lock` sha256** + **`sum.vifast.org`** (sổ hash chỉ ghi thêm);
`VS0707` = hash lệch; **không** lệnh bỏ qua, chỉ `VIS_SUMDB=off`; `VIS_PRIVATE=<mẫu>`; `proxy.vifast.org`
**chỉ thiết kế**, `VIS_PROXY=direct` mặc định.

**Bằng chứng**: `vs3-qd-6.md` đã viết lại theo VD-180/181; **4 ca đỏ-trước bắt buộc** (tag đổi nội dung ·
MVS v1.2/v1.4 · vanity → GitHub · offline + cache).

**Lựa chọn**: (A) triển khai đúng thiết kế trong VS3-6; (B) chia nhỏ (MVS trước, sumdb sau); (C) hoãn.

**Đề xuất**: **(A)** — nhưng **chia task ≤3 h** theo đúng 4 ca đỏ-trước; sumdb/proxy là **thiết kế**, chưa
vận hành ở VS3.

**Nếu không quyết**: VS3-6 không mở; các VS3 khác vẫn chạy.

---

### 3.3 NHÓM C — KHÔNG CHẶN, QUYẾT KHI CÓ THỜI GIAN

---

#### Q9 · `#142` — `pkg update X` ghim mọi gói khác

**Chuyện gì**: `pkg update <tên>` hiện **ghim mọi gói khác** (`=<version>`); nếu gói được update cần nâng
một phụ thuộc chung, bộ giải báo `VS0704` thay vì nâng.

**Lựa chọn**: (A) đổi chính sách: `pkg update X` chỉ đổi **X + phụ thuộc bắt buộc của X** (đúng VD-180);
(B) giữ nguyên; (C) thêm cờ `--allow-dep-bump`.

**Đề xuất**: **(A)** — VD-180 đã nói đúng điều này; làm trong VS3-6.

---

#### Q10 · `#145` — chú thích sai về `chan(0)`

**Chuyện gì**: Chú thích trong mã nói `chan(0)` bị **tyck** chặn (exit 2); thực tế tyck **không** kiểm, chỉ
**runtime** chặn (exit 1). VS3-QĐ-2 đã chốt `chan(0)` = **rendezvous hợp lệ** ⇒ chú thích sai sẽ bị xoá và
chặn runtime bị bỏ.

**Lựa chọn**: (A) xử trong VS3-2 (đã có trong thiết kế); (B) sửa chú thích ngay; (C) để.

**Đề xuất**: **(A)** — VS3-2 đã bao gồm #145.

---

#### Q11 · `#140/#109b` — đường **ghi** vẫn kiểm-rồi-mở

**Chuyện gì**: Đường ghi tệp vẫn **kiểm rồi mở** (`FsGuard::write`): kẻ đổi symlink giữa bước kiểm và bước
ghi có thể truncate/ghi ra ngoài thư mục gốc. Sửa cần `O_NOFOLLOW`/`openat` = **VS3**.

**Lựa chọn**: (A) làm theo VS3-QĐ-4 (đi từng thành phần từ fd gốc, đọc+ghi **chung một hàm**); (B) giữ
giới hạn đã ghi trong `HOST-BAO-DAM.md`.

**Đề xuất**: **(A)** — VS3-QĐ-4 đã duyệt; đây là **cùng cơ chế** với đường đọc (đã vá #146).

---

#### Q12 · `psvn` — 1 ca đỏ mới `audit_span`

**Chuyện gì**: Trên **clone sạch** ở Windows, 1 ca mới đỏ `audit_span` (0 đơn vị đa tệp; `use "./…"` trên
Windows). **Không** phải hồi quy tách repo (R29 trên psvn chưa từng chạy ca này — dừng ở `cap_proc`).

**Lựa chọn**: (A) điều tra trong VS3-5; (B) điều tra ngay; (C) bỏ qua.

**Đề xuất**: **(A)** — thuộc nhóm Windows, không chặn mac/Linux.

---

#### Q13 · `5. bit_or(3)` có khoảng trắng

**Chuyện gì**: R19c (ST-180) đã sửa để `5.bit_or(3)` = int + gọi method. Nhưng `5. bit_or(3)` (có **khoảng
trắng** sau `.`) **vẫn** `VS0104`. Đặc tả R19c nói "**không phụ thuộc khoảng trắng**".

**Lựa chọn**: (A) nới để cho phép khoảng trắng sau `.`; (B) giữ như hiện tại (chỉ `.` **ngay** trước định
danh); (C) cấm cả hai (quay lại như cũ).

**Đề xuất**: **(A)** — đúng đặc tả; nhưng đây là **ca biên**, có thể để pha 2.

---

#### Q14 · Cách hiểu "9 mục [SỬA]" của VD-178

**Chuyện gì**: VD-178 ghi "9 chỗ [SỬA]"; ST-178 đếm được **9 nhóm quyết định** (QĐ-1…QĐ-6 + C1 + C2 + C3)
với **16 dấu [SỬA]** bên trong (QĐ-1×2, QĐ-2×1, QĐ-3×1, QĐ-4×2, QĐ-5×1, QĐ-6×1, C2×8).

**Lựa chọn**: (A) xác nhận cách hiểu 9 nhóm / 16 dấu; (B) yêu cầu đếm khác.

**Đề xuất**: **(A)** — chỉ là việc sổ; ST-178 đã liệt kê đủ 16 dấu để A0 đối chiếu.

---

## 4. VIỆC A0 CẦN **BIẾT** (không cần quyết)

1. **Nợ #146 đã đóng** (đua symlink macOS): vá **1 dòng** `metadata` → `symlink_metadata` + **điểm chèn
   `#[cfg(test)]`** chứng minh **tất định** (ca A đỏ 100/100 trên mã cũ, xanh 100/100 sau vá trên **mac và
   Linux**; ca B đối chứng đọc đúng; ca âm bỏ điểm chèn ⇒ báo "không chạy được điểm chèn"; **6 000 000
   vòng** 0 đỏ). Giới hạn **hardlink** còn lại là **chính sách**, không phải lỗi đua.
2. **Repo đã tách** thành `github.com/vifast-oss/ViScript` (public, giữ 459 commit). `vis-host-linux` và
   `vis-host-wasm` **giữ trong repo mới** (đã chốt); repo cũ `ViFastOs` đã xoá hai thư mục.
3. **PR #26/#27** từ ViFastOs đã port sang repo mới, giữ tác giả (khoajunior); 2 PR đó **không merge được**
   ở ViFastOs vì cây `viscript/` đã bị xoá ở ST-171.
4. **Khe hở bằng chứng cũ**: 3 lần chạy cổng Linux của ST-161/166/167 chạy trên **cây bẩn** của dòng OS;
   phần ViScript **sạch theo kiểm hồi tố** ⇒ giữ làm **tham chiếu**, **không** dùng làm số chuẩn. Số Linux
   chuẩn của R29 là cổng **một lần trên clone sạch**.
5. **mac đo trên cây làm việc** (không phải clone sạch): `git status` chỉ có log `st170-*` do chính task
   tạo; **không** tệp mã nào bị sửa. Đây là thông lệ của dòng ViScript (mac là nơi ghi log).
6. **Windows chỉ đo** (VD-146): số ca Windows **không** so trực tiếp với mac/Linux; ~105 ca đỏ thuộc VS3-5.
7. **5 câu hỏi VS3** trong `JOURNAL/st155-vs3-dieu-tra.md` §8 **đã được A0 trả lời** ở VD-160…VD-164 —
   **không** cần trả lời lại.
8. **Sổ nợ**: 148 dòng = 88 mở / 37 đóng / 21 VS3 / 1 không làm / 1 điều hướng. Dòng tóm tắt §8 của
   `SO-NO.md` còn ghi 147/89/34 — **cũ**, Agent 2 sẽ cập nhật (không cần A0).
9. **Không có lần cổng đỏ nào do mã mới** trong đợt 29: 2 lần đỏ duy nhất là **#146** (đã đóng). Ba lần
   cổng của R29 (mac/Linux/Windows) **không retry**.
10. **Mọi thay đổi kỳ vọng test** trong VD-178 (C1 mã thoát 101, R7 đổi 4 test, R1+R2, R3, R5, R9, R10,
    R14, R22, R23, R24) **đã được A0 duyệt** — Agent 2 **không** hỏi lại.

---

## 5. BẢNG TỔNG HỢP QUYẾT ĐỊNH

| # | Việc | Loại | Đề xuất | Công | Chặn gì nếu không quyết |
|---|---|---|---|---|---|
| **Q1** | `#147` mac chậm 5,70× | Chọn hướng | **(C)** sửa script dev rồi đo lại; nếu vẫn >3× thì chấp nhận giới hạn nền | 2,5 h | Không chặn; nợ mở |
| **Q2** | Số mặc định HTTP | Chốt số | **(A)** 10 s / 30 s / 5 / 8 MiB | 0 | **VS3-3 (TLS)** |
| **Q3** | Sửa 1 dòng Windows | Cấp phép | **(A)** 1 dòng + ca khoá hai dấu phân cách, trong VS3 | 1 h | Cổng Windows dừng bước 5 |
| **Q4** | Mở VS3 | Xác nhận | **(A)** xác nhận 16 dấu [SỬA] → mở VS3 | 0 | **Toàn bộ VS3** |
| **Q5** | 2 tài liệu A0 sở hữu | Uỷ quyền | **(B)** Agent 1 soạn, A0 duyệt diff | 1 h | Tài liệu nói sai |
| **Q6** | Thứ tự pha 1 còn lại | Chốt thứ tự | **(A)** R2 → R4 → R5b → R5d | 0 | Không chặn (đang làm R2) |
| **Q7** | Windows "hỗ trợ thật" | Chốt kế hoạch | **(A)** VS3-5a..5e, 5a/5d song song pha 1 | 0 | Không chặn |
| **Q8** | Mô hình gói | Chốt kế hoạch | **(A)** VS3-6 theo 4 ca đỏ-trước | 0 | Không chặn |
| **Q9** | `#142` `pkg update` | Chốt chính sách | **(A)** chỉ đổi X + phụ thuộc bắt buộc | 0 | Không chặn |
| **Q10** | `#145` chú thích `chan(0)` | Chốt nơi xử | **(A)** trong VS3-2 | 0 | Không chặn |
| **Q11** | `#140/#109b` đường ghi | Chốt nơi xử | **(A)** VS3-QĐ-4 | 0 | Không chặn |
| **Q12** | `psvn` `audit_span` | Chốt nơi xử | **(A)** VS3-5 | 0 | Không chặn |
| **Q13** | `5. bit_or(3)` khoảng trắng | Chốt hành vi | **(A)** cho phép khoảng trắng (pha 2) | 0,5 h | Không chặn |
| **Q14** | "9 mục [SỬA]" | Xác nhận cách hiểu | **(A)** 9 nhóm / 16 dấu | 0 | Không chặn |

**Tổng công nếu A0 chọn đúng đề xuất**: ≈ **5 giờ** (Q1 2,5 h + Q3 1 h + Q5 1 h + Q13 0,5 h), phần còn lại
là quyết định **0 công**.

---

## 6. PHỤ LỤC — NGUỒN VÀ CÁCH ĐỐI CHIẾU

A0 **không cần** mở các tệp dưới đây; chúng chỉ để Agent 1/2 chứng minh số liệu.

| Nội dung | Nguồn |
|---|---|
| Số đo 3 nền cùng HEAD `39041b3f` | `agent/reports/R29-viscript.md` §2 |
| Nợ #146 (đã đóng) | `agent/SO-NO.md` dòng #146 · `JOURNAL/st174-*.log` |
| Nợ #147 (đang mở) | `agent/SO-NO.md` dòng #147 · `JOURNAL/st176-bao-cao.md` |
| 6 tài liệu VS3 | `agent/JOURNAL/vs3-qd-1.md` … `vs3-qd-6.md` |
| 16 dấu [SỬA] đã áp | `agent/JOURNAL/st178-bao-cao.md` §2–§5 |
| Số mặc định HTTP | `agent/JOURNAL/vs3-qd-3.md:14` |
| Lỗi Windows `check.rs` | `agent/BLOCKED.md` §5 · `JOURNAL/st147-chan-doan-psvn.log` |
| Pha 1 công thái học | `agent/JOURNAL/vs-ergo.md` (bảng tổng + R1..R24) |
| Tiến độ pha 1 | `agent/JOURNAL/2026-11-18.md` · `agent/QUEUE.md` hàng ST-179..ST-196 |
| Sổ nợ | `agent/SO-NO.md` §4 (bảng chính) |
| Quyết định gần nhất | `agent/DECISIONS.md` VD-175…VD-181 |

**Cách A0 trả lời**: chỉ cần ghi mã quyết định + lựa chọn, ví dụ:

```
Q1 = C   Q2 = A   Q3 = A   Q4 = A   Q5 = B
Q6 = A   Q7 = A   Q8 = A   Q9 = A   Q10 = A
Q11 = A  Q12 = A  Q13 = A  Q14 = A
```

Agent 1 sẽ chuyển thành **VD-182** trở đi và mở task tương ứng.
