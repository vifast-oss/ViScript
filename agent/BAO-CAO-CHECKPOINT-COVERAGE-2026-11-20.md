<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Checkpoint khép coverage frame & closure/writeback (ST-274/275, VD-255)

> **Trạng thái**: đã xong ST-274 + ST-275. **Chưa đóng checkpoint tích hợp hoàn toàn** — VD-255 §1/§3:
> runtime mặc định **không đổi**, tag/release **PAUSED**, chưa mở worker/GC/TLS/API. Các blocker
> `connect`/`write`, step-cap/ngân sách, fairness CPU và hợp đồng vòng đời vẫn mở có chủ đích.
>
> **Căn cứ**: VD-255 (`agent/DECISIONS.md`) · `agent/JOURNAL/vd255-coverage-report.md` ·
> `agent/JOURNAL/vd255-budget-decision-sheet.md` · bàn giao trước `agent/BAN-GIAO-A0-LAT3-2026-11-20.md`.

## 1. ST-274 — khép coverage theo 3 nhóm nhỏ

**Code**: `crates/vis-interp/src/frame.rs` tách thành `frame.rs` + `frame/proc.rs` + `frame/expr.rs`;
một bump visibility trong `interp.rs` (`types → pub(crate)`). `rt.rs`, `vm_frame.rs`, bytecode, VERSION,
cache, checker, API **không đổi**.

| Nhóm | Phạm vi đã phủ | Bằng chứng |
|---|---|---|
| **A — điều khiển** | `if`/`elif`/`else`, `if-let`, `match` (có guard), `while`/`loop`/`for`/`while-let`, `break`/`continue` (unwind qua `LoopMark` trên heap), short-circuit `&&`/`\|\|` (**nhánh bị bỏ không đánh giá**), `??`/`\|>` | test `lat255_coverage.rs`, source thật, cả 2 engine |
| **B — biểu thức/gọi** | field/index/`?.`, record/tuple/list/set/map/enum/opt/with, đối số tên + default, unary + toán tử còn thiếu, hằng cấp tệp | cùng test |
| **C — closure** | gọi closure như giá trị; ô capture được **snapshot vào `Rc` mới** lúc tạo (khớp ngữ nghĩa mặc định) và **dùng chung qua các lần gọi** ⇒ writeback thấy thay đổi qua suspend; nhiều frame closure cùng tồn tại trên heap | cùng test |

**Test**: `crates/vis-cli/tests/lat255_coverage.rs` **9/9** — đều đi **source → lower → check → HIR/bytecode
thật → `Interp`/`Vm` thật**, so byte stdout/exit/chẩn đoán/log/`DiagSnapshot` giữa hai engine. Các test
từ chối cũ `r9/r10` trong `lat2_reactor.rs` được đổi thành test dương (có căn cứ ngữ nghĩa).
Tổng: LAT1 8/8 + 1/1 · LAT2 13/13 · LAT3 12/12 · LAT255 9/9;
`cargo test -p vis-interp -p vis-cli` **1041/0**; `./vis-dev gate --fast` **1484/1484 PASS**.

**Fixture async**: thực tế có **26 tệp `.vis`** trong `test/sem/async/` (52 tệp nếu tính `.out`/`.err`),
**không phải 35** như một số báo cáo trước — đã ghi rõ trong `vd255-coverage-report.md`. Tất cả 26 được
phân loại; **không golden nào bị đổi**. Một ca `grid-await-nghich-thu-tu` chạy được trên cả hai engine
nhưng thứ tự task khác nhau (cooperative vs run-to-completion) — **đã ghi nhận, golden giữ nguyên**.

## 2. Còn mở trong coverage (không fallback để che)

- `?`, `chan`, `CmdBlock` — **từ chối rõ**, chưa hỗ trợ trong đường frame.
- Lỗi **final-join chưa được surfaced** trong đường frame test.
- VM codegen: **`await` trong thân closure** chưa hỗ trợ (đường test), dù interp đã phủ capture/writeback.
- **Chưa tính phí ngân sách** trong đường frame (theo thiết kế — ngân sách chưa được duyệt).
- Mọi cấu trúc chưa phủ vẫn bị từ chối bằng chẩn đoán; **không** rơi về runtime đồng bộ.

## 3. ST-275 — phiếu ngân sách ngắn (chờ A0)

`agent/JOURNAL/vd255-budget-decision-sheet.md`: 8 sự kiện + trọng số **đề xuất** (bổ sung `E_STMT`,
`E_ASSIGN`, expression tổng hợp); giữ nguyên 5 quy tắc đã chốt (VD-252); so sánh **bảng phụ vs
`Instr::Charge`** cho cùng hợp đồng; kiểm crafted-bytecode không né phí (thiếu/sai/zero entry, nhảy bỏ
charge, loop không charge, resume tính trùng — **không tin artifact chỉ vì decode được**); ngân sách tổng
vs hạn ngạch lượt là **hai bộ đếm riêng**, quota reset/turn. **D1–D9 chờ A0**; chưa chọn biểu diễn,
chưa gán `VERSION`, không đổi cache/API. Step-cap parity **vẫn mở**, không tăng ngưỡng tạm.

## 4. Gate & commit

- `./vis-dev gate --fast` trên cây trước khi commit checkpoint này: **1484/1484 PASS**
  (R1..R6 OK, readme-examples 20/20, doc-examples OK).
- Commit mã ST-274/275 nằm cùng commit tài liệu này; bàn giao ST-271 có hash
  `301e3ab56b1ebda300bd1613ba207e7a116d4056`; cây tích hợp sau README/tooling `916ba7b…`;
  trạng thái cây sạch sau commit.
- Cây tích hợp chung **đã xanh** — tuy VD-255 vẫn ghi "chưa đóng checkpoint tích hợp" vì các blocker
  chức năng ở §2 và §5 chưa khép. Không gọi đây là PASS cho mốc phát hành.

## 5. Điều kiện trước khi đổi runtime mặc định (VD-255 §5 — vẫn giữ)

Coverage HIR + closure/writeback **đủ theo ngôn ngữ hiện hành** *(phần lớn đã phủ, còn §2)* · `connect`/
`write` không chặn ngoài hợp đồng · ngân sách + step-cap parity xử lý *(phiếu D1–D9 chờ A0)* · vòng đời,
lỗi, cancellation, cleanup có hợp đồng + test · bằng chứng chống đói CPU theo phạm vi chọn · dependency
chuyển khỏi dev-only có chủ đích + kiểm nền tảng/giấy phép · gate + phép đo trên đúng ứng viên tích hợp.

**Kết luận cho A0**: động cơ async mới giờ chạy được phần lớn chương trình ViScript hiện có trên cả hai
engine, không fallback đồng bộ. Đây là điều kiện cần trước khi thay động cơ cũ — **chưa đủ điều kiện**.
Cần A0 chốt phiếu ngân sách (D1–D9) và quyết lát `connect`/`write`; tag/release tiếp tục PAUSED.
