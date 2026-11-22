<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Phiếu đề xuất kiến trúc — nâng cấp async thực dụng một luồng (VD-249)

> **TRẠNG THÁI: ĐỀ XUẤT — CHỜ A0 CHỐT.** Không phải quyết định, không phải triển khai.
> **Tag/release vẫn PAUSED**; baseline lịch sử `dbae70a`. **Chưa** mở VS3 toàn phần, worker pool,
> GC, TLS, hay API công khai mới. **Không dùng lại ước lượng dòng/tuần-người cũ làm cam kết.**
>
> **Căn cứ**: VD-249 (`agent/DECISIONS.md`), 3 luồng ST-261/262/263:
> `agent/JOURNAL/vd249-budget-draft.md`, `.../vd249-exit101-triage.md`,
> `.../vd249-async-design-draft.md`, `.../vd249-lifecycle-measure.md`; báo cáo
> `agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md` (đã đính chính đoạn HTTP).

---

## 0. Ba phát hiện cần A0 biết trước khi chọn

1. **exit 101 không phải lỗi.** VM exit 101 là **mã thoát CLI chủ ý** cho lỗi `VS05xx`
   (`crates/vis-cli/src/cli/run.rs:13-21`), không phải panic (`vd249-exit101-triage.md`). Lỗi là
   **vị trí đặt hàng rào**: interp đếm vòng lặp (`interp.rs:541`), VM đếm từng lệnh
   (`vm.rs:97,737`) — cùng ngưỡng 10⁷ nhưng khác đơn vị.
2. **#148 không thể sửa ngay theo nghĩa cũ.** `timeout` của `http.get_timeout` đã được
   **VD-183 Q2(1)** chốt là *giữa hai lần đọc, không phải tổng*. Hơn nữa mã hiện tại đã tính
   deadline **một lần cho mỗi response**, nên slow-drip trong một response **đã bị chặn**; phần
   còn hở là **chuỗi redirect** (mỗi hop deadline mới, ≤ ~6× timeout). Theo VD-249 §5, phải A0
   chótes lại hợp đồng trước khi đổi mã (`vd249-lifecycle-measure.md` §1).
3. **Tăng bộ nhớ đo được là có thật nhưng có nguyên nhân rõ.** 60 060 task qua 20 sóng:
   RSS 10,9 → 28,7 MB, ~**320 B/task**, **không bằng phẳng**; control không spawn task thì RSS
   bằng phẳng ~9,5 MB. `Rc<Task>` vẫn **reachable** qua `Scheduler.tasks` — không phải rò rỉ
   unreachable, mà là **tăng không giới hạn theo workload spawn lặp**. A0 cần quyết có chấp nhận
   trong mốc này không; tôi **không** coi "không có GC" tự thân là blocker.

---

## 1. Quyết định kiến trúc cần A0 chốt

> Mỗi dòng là một quyết định; phần đề xuất là **khuyến nghị**, phần thay thế là phương án đã rõ.
> Chi tiết nằm trong bốn draft journal ở trên.

### 1.1 Hướng async (Q1)

| Đề xuất | Thay thế | Lý do |
|---|---|---|
| **A** — state-machine/frame trên heap + selector mỏng, `Scheduler` tự sắp thứ tự wake | **B** — event loop có sẵn (`smol`/tokio current-thread) + lớp task tự viết; **C** — generator/busy-poll chỉ làm spike lát 0 | A giữ được **tất định + byte-identical interp↔VM** (bất biến cứng), đúng 3 use case mà không mở worker/GC/API. B phủ OS nhanh nhưng thứ tự wake do dependency quyết, phải ép lại. C lặp lại 4,11 s busy-poll, không đạt mục tiêu B1. |

**Rủi ro của A**: interp dùng Rust call stack ẩn (`host.rs:26-35`), VM dùng `Frame` heap có PC
(`vm.rs:124-135`) — hai đường suspend dễ lệch. Đây là rủi ro số 1.

### 1.2 Hợp đồng tính phí ngân sách (A0-1..8, thay cho step-cap)

| Quyết định | Đề xuất | Thay thế / lưu ý |
|---|---|---|
| Sự kiện tính phí | `E_STMT`=1, `E_EXPR`=1, `E_LOOP`=1, `E_CALL`=4, `E_BUILTIN`=2, `E_SPAWN`=4, `E_AWAIT`=2 — **sự kiện ngữ nghĩa có mặt ở cả hai engine** | Có gộp `E_EXPR` vào `E_STMT` không? |
| Ngân sách | **Một pool cho toàn lần chạy, gồm task**; không reset khi spawn/await/join; không refund khi huỷ | — |
| Biên | Kiểm tra **trước tác dụng phụ**; `w == remaining` cho phép, sự kiện sau trap; không rollback phần đã xong | Có trap ngay khi chạm 0 không? |
| Giới hạn riêng | Bộ nhớ, độ sâu gọi (2000), lồng await (256), thời gian chờ I/O — **giữ riêng**, không gộp vào `steps` | — |
| Biểu diễn bytecode | **Bảng phụ sự kiện** song song với `code` (không thêm opcode) ⇒ đổi `Module`, tăng `VERSION` 2→3, cache tự invalid | `Instr::Charge` (thêm opcode, đổi tag — cần duyệt riêng); giữ nguyên định dạng (bị loại: là "đếm lệnh" đội lốt) |
| Mã lỗi | Tái dùng `VS0501` / `VS0506` với thông điệp riêng ⇒ exit 101 giữ nguyên | Mã mới ngoài `VS05xx` ⇒ phải sửa bảng exit |
| Backstop | Giữ `MAX_STEPS` tạm, nhưng mọi guard phải đi qua cùng hàm `charge()`; bỏ là quyết định riêng | — |
| Test | Vòng lặp, đệ quy, nhiều task, **tác dụng phụ sát biên**, parity `(stdout, exit, chẩn đoán)` — không chỉ một ca dài | Cần test-hook nội bộ để đặt budget nhỏ |

### 1.3 Cơ chế (Q2–Q9)

| Quyết định | Đề xuất | Thay thế |
|---|---|---|
| Suspend/resume | Frame heap; VM tái dùng `Frame { pc, locals, captured }`; interp đưa continuation ra heap. **Lát đầu chỉ suspend tại câu lệnh trực tiếp**, công bố giới hạn (chưa hỗ trợ `f(g(await x))`) | Full expression-level ngay |
| `MAX_TASK_NEST` | Giữ ngữ nghĩa `VS0501` tới khi có ca mới | Định nghĩa lại khi lên heap |
| Scheduler | `ready` chứa task sẵn sàng; nhường tại `await`, I/O, **và hạn ngạch CPU** | Chỉ nhường tại await/I/O (đủ B1, chưa đủ B2) |
| Đơn vị hạn ngạch | **Đơn vị mới tách khỏi `steps`** (đang lệch interp/VM); con số hiệu chỉnh bằng đo, không cam kết | Gắn vào `steps` — bị loại vì phá parity |
| I/O | Giữ bề mặt ngôn ngữ **đồng bộ**; registry nội bộ trong host; `read`/`accept` không chặn trước, `connect`/`write` chặn ngắn và **công bố rõ** | Đổi public `NetHost` (không khuyến nghị) |
| Timer | Một heap `Instant` monotonic, tie-break `(deadline, task_id)` | Timer của event loop (phương án B) |
| Cancellation | Huỷ **hợp tác tại điểm nhường**; gỡ fd/timer ngay, dọn tài nguyên khi frame tới điểm chờ tiếp; dùng mã cũ | Preempt giữa lệnh (không đề xuất) |
| Vòng đời | Cache/`VS0506`/join theo thứ tự tạo **giữ nguyên**; task còn pending lúc main exit ⇒ ép huỷ, đóng handle, báo bằng mã cũ | Chờ deadline rồi huỷ |
| Giới hạn | Cap nội bộ (số task, hàng đợi, buffer), A0 ấn số sau khi đo; trap tất định | Opt-in kiểu `--strict-caps` |
| Host | Runtime chung trong `vis-interp::rt`; adapter selector theo nền; Windows chỉ đặt hợp đồng trước | Làm cả 3 nền ngay |

### 1.4 #148 — chọn hợp đồng (A/B/C)

| Phương án | Nội dung | Đánh giá |
|---|---|---|
| **A (đề xuất)** | Re-chốt `timeout` thành **một deadline monotonic cho toàn lời gọi** (connect + mọi redirect + reads); hoist `han` lên `http_get`/`http_head`; per-read clamp 2 s giữ nguyên; `net.close` như hiện nay | Khớp "không thêm API timeout mới"; **phải** ghi rõ supersede VD-183 Q2(1); hành vi đổi ở redirect ≥2 hop ⇒ cần ca đỏ-trước |
| **B** | Giữ nghĩa VD-183, thêm `timeout_total` (~120 s) như nợ #148 | Đúng nợ cũ nhưng **vi phạm** "không API mới" — cần A0 ratify ngoại lệ |
| **C** | Đóng #148 với "đã bounded mỗi response; redirect ≤ 6× timeout" | Chỉ nếu A0 chấp nhận deadline nhân lên là "tổng" |

### 1.5 Vòng đời bộ nhớ (A0 quyết)

- Đề xuất: **không coi absence of GC là blocker**; yêu cầu rõ ràng hơn là "tăng không giới hạn trong
  workload mục tiêu". Với 60k task tăng ~320 B/task — cần A0 chốt có chấp nhận trong mốc đầu hay
  yêu cầu cap/giải phóng sớm hơn.
- Công cụ: đề xuất hook chẩn đoán `VIS_RUNTIME_DIAG=1` (stderr, đếm task/handle sống) — **chưa
  implement**, chờ A0 duyệt. Không thêm cờ CLI phát hành.

---

## 2. Bất biến phải giữ

1. **Byte-identical interp↔VM** cho mọi fixture hiện hữu; `diff_100k` lệch 0; 525 ca semantics và
   L0 không giảm.
2. `spawn` không chạy thân tới `await`/join; **join cuối theo thứ tự tạo**; task không await vẫn chạy.
3. Lỗi task vẫn `VS0506` giữ **mã gốc + span lúc tạo**; cache trả lại lần hai.
4. `chan(0)` ⇒ `VS0505`; `send` sau close ⇒ `VS0504`; exit 101 cho `VS05xx`.
5. Mạng **loopback-only**; không TLS/HTTP2; không `chan(0)`/select.
6. **Không public API**, không `unsafe` mới, không `Send+Sync`, không worker pool, không GC.
7. Không đổi Cargo VERSION/opcode **trước khi A0 chốt**; nếu chọn bảng phụ/opcode `Charge` thì đổi
   `VERSION` + tự invalidate cache một lần, báo riêng.

---

## 3. Ảnh hưởng ngôn ngữ / API / bytecode

| Tầng | Dự kiến |
|---|---|
| Ngôn ngữ | Bề mặt `async fn`/`spawn`/`await`/`net.*` **giữ nguyên**. Có thể đổi chi tiết `MAX_TASK_NEST`, thông điệp "task pending lúc exit", và (nếu A0 chọn A ở §1.4) nghĩa `timeout` — **đều phải chốt**. |
| API | Chỉ kiểu nội bộ (`pub(crate)`/private); adapter nằm ở host crate; `trait Host`/`Value` không đổi. |
| Bytecode | VM đã có `Frame` có PC ⇒ có thể không cần opcode mới **nếu** tái dùng `Call`/`Return` + marker nội bộ. Nếu cần `Suspend/Yield` thật hoặc bảng phụ sự kiện ⇒ đổi định dạng + `VERSION` + hash cache — báo riêng. |

---

## 4. Lát cắt triển khai + tiêu chí nghiệm thu

> Mỗi lát chạy được và đo được độc lập; baseline **B1** (20 × 200 ms: 4,11 s) và **B2**
> (CPU+I/O: `CPU_DONE` = `IO_START` cùng ms) luôn chạy lại, kể cả khi lát chưa sửa được chúng.

| Lát | Việc | Tiêu chí nghiệm thu |
|---|---|---|
| **0 — Spike frame** | Prototype pause/resume một `async fn` tại await giả; thử suspend câu lệnh trực tiếp vs biểu thức lồng | Task A dừng → B chạy → A resume; output byte-identical với tuần tự trên cả 2 engine; xác định rõ giới hạn interp. B1/B2 giữ nguyên làm baseline. |
| **1 — Scheduler có frame** | `ready` chứa frame sẵn sàng; tách `await_task` thành bắt đầu/tiếp tục; inject `Pending` giả; **chưa đụng reactor** | Ca synthetic "chờ giả + MARK spawn sau" ⇒ MARK in trước; 35 ca `test/sem/async/*.vis` + `diff_100k` không đổi; `VS0506`/cache/join/không-await giữ nguyên. B1 vẫn 4,11 s (ghi rõ). |
| **2 — I/O mạng không chặn** | Registry/selector cho `read`+`accept`, bỏ `sleep(5ms)`; `connect`/`write` chặn ngắn, công bố; timer tối thiểu | **B1: 4,11 s → ~một lần latency server (~0,3 s)** trên cả 2 engine; 20/20 đọc ok; thứ tự tất định; fixture async không đổi. B2 vẫn đói (ghi rõ). |
| **3 — Timer/deadline/huỷ/cleanup** | Timer heap; deadline tổng HTTP theo §1.4; huỷ tại điểm nhường, gỡ fd/timer; đóng handle | Timeout fires **không busy-poll**; số fd/timer về baseline sau huỷ; task huỷ trả mã cũ, không panic; slow-drip không gia hạn vô hạn (theo hợp đồng A0 chọn). |
| **4 — Hạn ngạch CPU chống đói** | Điểm nhường theo đơn vị A0 chốt, **tách khỏi `steps`**; xử lý phát hiện ST-260 | **B2: `IO_START` xuất hiện trước `CPU_DONE`**, CPU vẫn xong, `IO_DONE − IO_START ≈ 201 ms`; parity byte-identical; step cap/depth không bị né. |
| **5 — Giới hạn & vòng đời** | Cap nội bộ; backpressure trap; đo số task/tài nguyên sống; đánh giá riêng stack 256 MiB | Vượt cap ⇒ trap có chẩn đoán, không treo/OOM; chạy dài số task/tài nguyên ổn định (phân biệt cache vs ròi); mọi handle đóng ở mọi đường thoát; parity giữ nguyên. |

**Phần còn chặn — công bố ngay**: `fs.*`, SQLite, `exec.*`, `connect`/`write` vẫn đồng bộ ở lát 1;
chỉ dần chuyển ở lát 2/5. Lý do: use case 1 (script tự động hoá) cần chủ yếu suspend/timer/huỷ,
không cần I/O file nonblocking ngay. Không mở TLS/HTTP2/registry, `chan(0)`/select.

---

## 5. Rủi ro & phần chưa biết

| Rủi ro / ẩn số | Ghi chú |
|---|---|
| Parity frame | Interp Rust stack ẩn vs VM frame heap — rủi ro số 1; mỗi lát phải so byte |
| Đơn vị hạn ngạch | Không gắn vào `steps` (interp đếm vòng, VM đếm lệnh); cần A0 chốt đơn vị mới |
| Dependency & giấy phép | Allowlist QD §9.3 nằm ở repo `ViFastOs`; phải `cargo metadata` rà crate + transitive trước khi cam kết A/B |
| Cancellation & `Rc` | Không GC ⇒ chu trình vẫn là nợ; cần định nghĩa owner tài nguyên = task |
| Đo lường | Cần đếm task/tài nguyên sống (hook chưa được duyệt); B1/B2 phải lặp + báo phân bố |
| Windows | Không CI, adapter chưa định nghĩa; feature này chỉ đặt hợp đồng |
| Stack 256 MiB | Giữ tạm, ghi nợ đánh giá; **không** dùng làm lời giải async dài hạn |
| #148 | Nghĩa `timeout` đã chốt ở VD-183; chọn A/B/C trước khi sửa |
| Bộ nhớ 320 B/task | Có chấp nhận trong mốc đầu hay yêu cầu cap/giải phóng sớm — A0 quyết |

---

## 6. Đề xuất kết luận cho A0

Giữ **tag/release PAUSED** và `dbae70a` làm checkpoint. Chốt các quyết định ở §1 (đặc biệt Q1 hướng
A, hợp đồng ngân sách, §1.4 phương án #148, §1.5 dung lượng bộ nhớ). Sau khi chốt mới cho triển
khai Lát 0→5, mỗi lát chạy B1/B2 và dừng trình A0 nếu lát nào làm đổi bất biến §2. Không revert kết
quả đã đạt; không chạy audit vô hạn; không dùng ước lượng cũ làm cam kết.
