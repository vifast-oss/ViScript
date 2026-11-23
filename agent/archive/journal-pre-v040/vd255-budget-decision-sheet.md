<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-255 §4 · Phiếu quyết định ngắn: ngân sách tính phí

**Trạng thái**: CHỈ LÀ TÀI LIỆU — research/draft. **Không** sửa mã sản phẩm, **không** commit,
**không** đổi `VERSION`/cache/API. Mọi số và lựa chọn dưới đây **chờ A0**.
**Căn cứ**: VD-249 §3 (`agent/DECISIONS.md:2368-2385`), VD-250 §4 (`agent/DECISIONS.md:2487-2504`),
VD-252 §2 (`agent/DECISIONS.md:2634-2649`), VD-255 §4 (`agent/DECISIONS.md:2844-2850`);
chi tiết tại `agent/JOURNAL/vd252-charging-final.md`, `vd250-charging-spec.md`, `vd250-fee-repr.md`.

---

## 0. Bất biến đã chốt — giữ nguyên, không mở lại

Năm quy tắc VD-252 §2 (`agent/DECISIONS.md:2638-2645`; `vd252-charging-final.md:12-19`):

| # | Quy tắc (đã chốt) | Điểm chèn / hệ quả |
|---|---|---|
| 1 | Nút gọi chỉ tính `E_CALL`, **không** cộng `E_EXPR` cho chính nút đó; callee/đối số theo thứ tự ngôn ngữ, tính riêng khi chạy | `interp/goi.rs:128,208`; `vm.rs:809,1157` |
| 2 | Builtin chỉ `E_BUILTIN`, **không** `E_CALL`; tính một lần **trước** khi thực hiện, `Pending`/resume không tính lại | dispatcher chung `vis_interp::builtin`, VM `vm.rs:851` (`vd252-charging-final.md:15`) |
| 3 | Short-circuit: chỉ toán hạng **thực sự chạy**; charge ở đầu toán hạng, **sau** lệnh nhảy; không mất phí vì VM hạ thành nhảy | `format.rs:145-147`, `codegen.rs:900,908` |
| 4 | `E_LOOP` một lần/**mỗi lần thử điều kiện – tiến vòng, GỒM lần cuối sai**; `while`→guard, `loop`→back-edge, `for`→`IterNext`, `while let`→lần thử khớp pattern; thân không có `E_LOOP` riêng | `codegen.rs:691-701,721-756,758-767`; ánh xạ `vd252-charging-final.md:24-29` |
| 5 | Một `E_AWAIT`/**mỗi lần bắt đầu await trong source**; resume không tính lại; await lần hai là sự kiện mới **kể cả dùng cache** | charge **trước** suspend, `pc` resume sau điểm đó (`frame.rs:614-624`; cache `rt.rs:180-194`) |

**Thứ tự** (điểm dễ sai nhất): `f(g(1))` ⇒ `E_CALL(f)` + `E_CALL(g)` + `E_EXPR(1)` — **không** charge gọi
`f` trước khi đánh giá `g(1)`. Kiểm ngân sách **ngay trước tác dụng phụ tương ứng**; `charge` nguyên tử:
`w > remaining` ⇒ trap, không trừ/không thi hành; `w == remaining` ⇒ trừ về 0 rồi thi hành; **không
rollback** tác dụng phụ đã xong (`vd252-charging-final.md:37-50`). Hai engine cùng dãy `(mã, vị trí)`.

---

## 1. Catalog sự kiện + trọng số — ĐỀ XUẤT, CHƯA DUYỆT

Bảy số cũ `1/1/1/4/2/4/2` **chưa được duyệt** (`agent/DECISIONS.md:2493`; `vd250-charging-spec.md:6`);
VD-252 ghi rõ **chưa có danh mục đầy đủ** cho `E_STMT`, assignment, expression tổng hợp
(`agent/DECISIONS.md:2647`). Dưới đây là catalog **đầy đủ đề xuất**; mọi số là `[ĐỀ XUẤT]`, chờ A0.
Giữ ký hiệu `w(E_*)`; không ấn số lúc này.

| Mã sự kiện | Sự kiện ngữ nghĩa | Trọng số `[ĐỀ XUẤT]` | Ghi chú |
|---|---|---|---|
| `E_STMT` | Mỗi câu lệnh HIR bắt đầu thi hành | 1 | điểm `exec_stmt` `interp.rs:299-300` |
| `E_EXPR` | Mỗi biểu thức HIR được đánh giá, **gồm cả expression tổng hợp** (binop/unop/field/index/tuple/record/cast/literal) | 1 | điểm `eval` `interp.rs:644-645`; mỗi nút tổng hợp **một** `E_EXPR`, không cộng thêm cho từng phần con. Nút gọi → `E_CALL`, builtin → `E_BUILTIN` (quy tắc 1–2) |
| `E_ASSIGN` **(mới)** | Mỗi phép gán: khởi tạo `let`/`var`, gán lại, compound assign, **writeback ô capture** | 1 | tách khỏi `E_STMT` để phản ánh công ghi ô; `[CHỜ A0 CHỐT]` ranh giới writeback closure (ST-274) |
| `E_LOOP` | Mỗi lần thử điều kiện/tiến vòng, kể cả lần cuối sai | 1 | thay hai hàng rào `interp.rs:541,559` (`vd252-charging-final.md:31-33`) |
| `E_CALL` | Mỗi lời gọi hàm người dùng mở khung | 4 | `interp/goi.rs:128,208`; không gộp với cap độ sâu |
| `E_BUILTIN` | Mỗi builtin/phương thức (ranh giới host) | 2 | dispatcher dùng chung `builtin.rs`; `vm.rs:851` |
| `E_SPAWN` | Mỗi `spawn` tác vụ | 4 | `rt.rs:112`; `$spawn` hạ thành `CallBuiltin` `codegen.rs:1446-1472` |
| `E_AWAIT` | Mỗi lần bắt đầu `await`/join, kể cả kết quả cache | 2 | `rt.rs:178,242`; `$await` `codegen.rs:1439-1444` |

**Hiệu chỉnh bằng đo đạc, không kế thừa 10^7**: ngưỡng `10_000_000` (`interp.rs:541,559`;
`vm.rs:97`) là **hàng rào cũ sai đơn vị**, không được đổi tên thành `BUDGET`. Chọn trọng số/`BUDGET`
bằng phép đo VD-249 §6 (`agent/DECISIONS.md:2410-2417`): đo dãy sự kiện + thời gian monotonic trên
workload đại diện (2M vòng, gọi/đệ quy, nhiều task, I/O sát biên), lặp và báo phân lớp/phân bố; đặt
trọng số phản ánh công tương đối, `BUDGET` sao cho ca hợp lệ thoải mái dưới ngưỡng. **Không cam kết số
trước khi đo** (`vd249-budget-draft.md:32,106`).

---

## 2. Biểu diễn: bảng phụ vs `Instr::Charge` — CÙNG hợp đồng, CHƯA CHỌN

Hai phương án là **hai biểu diễn của cùng một hợp đồng** (dãy sự kiện §0–§1); bảng phụ **không tự động
được duyệt** chỉ vì không thêm opcode (`agent/DECISIONS.md:2499-2503`). Cả hai **[CHỜ A0 CHỌN]**.
So sánh trung lập (`vd250-fee-repr.md:25-44`):

| Tiêu chí | S — bảng phụ `FeeEntry` song song `code` | O — opcode `Instr::Charge(kind, weight)` |
|---|---|---|
| Điểm charge tự mô tả trong artifact | Có (`fee[pc]`) | Có (`Charge` nằm trong luồng lệnh) |
| Nhảy tới `pc` khác có mang charge không | Không — phụ thuộc `pc` đích | Có — charge là lệnh, nhảy qua là mất charge |
| Bề mặt giả mạo | Sửa `weight`/`kind`, xoá entry, dời entry | Xoá `Charge`, đổi `weight`/`kind`, đặt sai chỗ |
| Kích thước artifact | Mỗi `pc` một ô (nén như span `format.rs:13-16`) | Mỗi điểm sự kiện một lệnh + toán hạng |
| VM runtime | Một lần tra mảng, thay `vm.steps += 1` (`vm.rs:736`) | Một nhánh `match` thêm trong vòng fetch `vm.rs:716-735` |
| Giống cấu trúc hiện có | Có (span đã là bảng song song) | Có (mọi thứ khác đã là opcode; tag mới `format.rs:232-234`) |
| Kiểm verifier riêng | `len(fee)==len(code)`; entry khớp **loại lệnh tại chính `pc`**; bảng site (nếu có) phải kiểm chéo, không tin | `Charge(kind,w)` ngay trước lệnh tạo sự kiện; `Charge` ở `pc` lạ ⇒ từ chối; mọi `Charge` hợp lệ nhưng **không chứng minh được không thiếu** nếu không có derived expectation từ HIR |

Cả hai đều đổi định dạng ⇒ phải xử lý `VERSION`/cache (`format.rs:44`; `cache.rs:55,61`).
**Không ấn số VERSION**: chỉ **cấp một phiên bản định dạng chưa dùng SAU khi A0 chọn biểu diễn**
(`agent/DECISIONS.md:2502-2503`; `vd250-fee-repr.md:123-135`). Không đổi `MAGIC` (`format.rs:36`),
không đổi tag cũ (`format.rs:232-234`).

---

## 3. Chặn artifact né phí — verifier bắt buộc, CẢ HAI biểu diễn

**Không tin dữ liệu tính phí trong artifact chỉ vì decode thành công.** `decode` chỉ kiểm
magic/version/độ dài (`decode.rs:237-252`), `verify` chỉ kiểm cấu trúc/stack/span/version
(`verify.rs:314-316`); cache luôn qua `load_verified` (`cache.rs:10-11`, `format.rs:24-27`) — nhưng hiện
**không có khái niệm phí**. Phải thêm một bước **kiểm phí trong verifier**, với kỳ vọng **suy từ cấu trúc
lệnh** (derived expectation), **không đọc từ `fee`/`Charge` trong artifact** (`vd250-fee-repr.md:50`).

| Hình thức né phí | Kiểm tra đề xuất `[CHỜ A0 CHỐT]` |
|---|---|
| **Thiếu entry phí** | S: `len(fee)==len(code)`, mọi `pc` thuộc tập điểm sự kiện phải có entry; O: mỗi điểm sự kiện suy được phải có `Charge` ngay trước. Tập điểm sự kiện do verifier suy từ lệnh (`Call`/`CallBuiltin`/`CallMethod`/`CallValue`→call/builtin; `JumpIfFalse`/`IterNext`/`Jump` lùi→loop; `$spawn`/`$await`→`CallBuiltin`). Xem `vd250-fee-repr.md:52-59` |
| **Sai vị trí** | S: `FeeEntry` tại `pc` khớp **loại lệnh tại chính `pc`** (vd. `FeeWrongPosition`); O: `Charge` liền trước lệnh tạo sự kiện; dời sang `pc` khác hoặc nằm giữa hai lệnh lạ ⇒ `VerifyError`. `vd250-fee-repr.md:61-70` |
| **Zero / giả mạo** | Ưu tiên **không serialize `weight`**, chỉ serialize `kind`; runtime lấy `w = W[kind]` từ **bảng trọng số chuẩn đã chốt, có phiên bản** — không đọc từ artifact. Nếu vẫn serialize `weight`: `w > 0`, khớp trị chuẩn, và `kind` khớp loại điểm sự kiện (đổi `kind` sang mã nhỏ hơn là né phí). `vd250-fee-repr.md:72-79` |
| **Nhảy bỏ điểm charge** | Dựng CFG từ lệnh nhảy (`format.rs:145-151`): (1) mọi chu trình có ≥1 điểm charge; (2) **không có cạnh vào thân vòng bỏ qua guard charge**; (3) đích nhảy nằm trong `code`; (4) short-circuit `JumpIfNotTrue` (`codegen.rs:900,908`) bỏ charge toán hạng phải là đúng, nhưng **không** được bỏ charge nút điều khiển nếu A0 chọn charge nút đó. `vd250-fee-repr.md:81-89` |
| **Loop không bị tính** | Ngoài CFG: từ `Module.entry` đến mọi chu trình phải đi qua ≥1 charge (reachability); **mọi chu trình qua `Call` (đệ quy) cũng phải có charge** — đây là chỗ `MAX_STEPS` từng làm backstop, giờ **verifier là hàng rào**. `vd250-fee-repr.md:91-96` |
| **Resume tính trùng** | Quy ước: điểm charge `E_AWAIT`/`E_LOOP` **trước** suspend, `pc` lưu lúc suspend **sau** điểm charge ⇒ resume tự nhiên không charge lại (`vm.rs:716-735`). Nếu dùng cờ `charged` thay vì dời `pc`, cờ là **trạng thái nội bộ, không đọc từ artifact**. Test A→suspend→B→A resume, so dãy `(mã,w)` không có `E_AWAIT` thứ hai cho cùng biểu thức. `vd250-fee-repr.md:98-105` |

Nghiệm thu bằng **ca âm**: mỗi artifact thiếu/zero/dời/nhảy-qua/resume-trùng bị **từ chối ở verify,
trước khi VM chạy** — đặt tại `crates/vis-bytecode/tests/decode_neg.rs`, `verify.rs`
(`vd250-fee-repr.md:96`).

---

## 4. Ngân sách toàn lần chạy vs hạn ngạch CPU lượt chạy

Hai **bộ đếm riêng**, không gộp; hạn ngạch **reset mỗi lượt chạy**, có thể tích lũy từ **cùng sự kiện
ngữ nghĩa đã chuẩn hoá** (`agent/DECISIONS.md:2489-2492,2504`; `vd250-charging-spec.md:137-152`).

| Thuộc tính | Ngân sách ngữ nghĩa (VD-249) | Hạn ngạch CPU (VD-250) |
|---|---|---|
| Phạm vi | Một pool **toàn lần chạy**, gồm task; không reset spawn/await/join, không refund huỷ | Mỗi **lượt chạy** của task, reset khi bị nhường |
| Đơn vị | Sự kiện `E_*` (§1) | Cùng điểm sự kiện đã chuẩn hoá, nhưng chỉ là **bộ đếm nhường**, không phải chi phí |
| Ngưỡng | `BUDGET` chưa chọn — đo (§1) | `Q` chưa chọn — đo sau; không cam kết số |
| Vượt | Trap `VS0501`, wrapper `VS0506`, exit 101 (`cli/run.rs:13-21`) | Xếp task cuối `ready`; **không** báo lỗi, **không** trap |
| Nguồn thời gian | Không dùng wall-clock | Không dùng wall-clock |

**Không tái dùng `steps`**: interp đếm nút (`interp.rs:101,300,645`), VM đếm lệnh (`vm.rs:97,736`) —
gắn nhường vào đó phá parity đã đo (`vd249-exit101-triage.md:46`).

---

## 5. Giữ mở / ràng buộc (không nap dưới bảng trên)

- **Step-cap parity finding vẫn MỞ** (VD-249/ST-260): 2M vòng ⇒ interp exit 0 / VM trap `VS0506→VS0501`
  (`agent/DECISIONS.md:2649`; `vd251-lat1-report.md:108-111`). **Không sửa tạm bằng tăng ngưỡng**, không
  tuyên bố parity cho workload chạm giới hạn.
- **Không giữ `MAX_STEPS` (`vm.rs:97,745`) làm giới hạn chương trình thường** song song ngân sách mới —
  sẽ tái tạo lỗi parity (`agent/DECISIONS.md:2497-2498`). Nếu có **watchdog nội bộ**, phải là cơ chế
  **khác** hết ngân sách, báo **rõ tên/nguồn** trong chẩn đoán, chỉ dùng an toàn runtime/test, không
  dùng che crafted bytecode (`vd250-charging-spec.md:156-160`).
- **Ngân sách mới KHÔNG phải prerequisite của reactor** — không chặn Lát 2/Lát 3/coverage frame
  (`agent/DECISIONS.md:2648,2844-2850`). B2/fairness CPU vẫn đói, **không đổi tiêu chí**.
- **Không ấn `VERSION`**, không đổi cache/API; phạm vi là phiếu quyết định, mọi hiện thực hoá chỉ sau VD mới của A0.

---

## 6. Bảng quyết định A0 phải đưa ra

| # | Quyết định | Khuyến nghị (chưa duyệt) | Căn cứ |
|---|---|---|---|
| D1 | Chốt **catalog sự kiện đầy đủ** (§1), kể cả `E_STMT`, `E_ASSIGN`, composite-expression | Chấp nhận danh mục §1; gộp composite vào `E_EXPR` với một trọng số chung | `agent/DECISIONS.md:2647` |
| D2 | Chốt **trọng số + `BUDGET`**, hay chỉ chốt danh mục rồi đo? | Chỉ chốt danh mục; số đo theo §1, **không kế thừa 10^7** | `agent/DECISIONS.md:2493` |
| D3 | Chọn biểu diễn **S (bảng phụ)** hay **O (`Instr::Charge`)** | Nghiêng S, nhưng **không** coi "không thêm opcode" là tự động duyệt | `agent/DECISIONS.md:2499-2503` |
| D4 | Chấp nhận chi phí **verifier CFG + kiểm phí** cho cả hai (§3)? | Bắt buộc; nếu không, crafted bytecode né phí | `agent/DECISIONS.md:2500-2502` |
| D5 | Serialize `weight` hay chỉ `kind` + bảng chuẩn có phiên bản? | Chỉ `kind`; suy `w` từ bảng chuẩn | `vd250-fee-repr.md:72-79` |
| D6 | Quy ước resume: charge trước suspend + `pc` sau charge, hay cờ `charged`? | Quy ước `pc`; cờ chỉ dự phòng | `vd250-fee-repr.md:98-105` |
| D7 | Cấp **phiên bản định dạng chưa dùng** khi nào? | **Chỉ sau D3**; không gán 3 ngay, không đổi `MAGIC`/tag cũ | `agent/DECISIONS.md:2502-2503` |
| D8 | Watchdog nội bộ: cho phép distinct hay cấm? | Cho phép distinct, báo rõ, **không** làm giới hạn chương trình | `agent/DECISIONS.md:2497-2498` |
| D9 | Giữ finding step-cap parity OPEN? | Giữ mở; không tăng ngưỡng, không tuyên bố parity | `agent/DECISIONS.md:2649` |

> **Phạm vi**: phiếu này chỉ ghi nhận bất biến đã chốt và các quyết định còn mở; không sửa `interp.rs`,
> `vm.rs`, bytecode/verifier/cache, không thêm opcode, không đổi `VERSION`, không commit, không chạy gate.
