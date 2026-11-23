<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-257 / stream C — cancellation hợp tác & lifecycle/cleanup (đường frame test)

- Ngày: 2026-11-20
- Nhánh: `work/vd257-c-lifecycle` (base `fc297ba`)
- Phạm vi sửa (đúng ownership): `crates/vis-interp/src/rt.rs`, test mới
  `crates/vis-cli/tests/lat257_c_lifecycle.rs`, báo cáo này. `stdlib.rs` và `net.rs`
  **không cần đổi** — handle mạng là chuỗi mờ do host cấp, đường frame không bao giờ đóng
  handle chỉ vì task tạo nó kết thúc (xem §4). Không đụng `frame/**`, `interp.rs`,
  `builtin.rs`, `vis-vm/**`, `vis-bytecode/**`, `cache.rs`, manifest/lock, DECISIONS/QUEUE,
  journal/fixture/golden của stream khác. Không CLI flag, không API ngôn ngữ, không unsafe,
  không dependency, không đổi bytecode/VERSION/cache-key.

## 0. Khẳng định biên (đọc đầu)

Cancellation được hiện thực **hoàn toàn ở lớp scheduler** (`rt.rs`), dùng chung bởi hai engine
vì cả hai đều gọi các helper `frame_*`. Driver (`frame.rs`/`vm_frame.rs`) **không bị sửa**
(ngoài ownership) — nên cơ chế "dừng tại safe point" được gắn vào đúng các safe point mà
driver đã gọi sẵn: `frame_next` (mỗi vòng lặp scheduler), `frame_fire` / `frame_reactor_fire`
(khi fire event), `frame_wake_one` (khi đánh thức). Task đang chạy (`step_frame`) **không bao
giờ bị cắt giữa lệnh**; chỉ task đang treo (`Pending`/có đăng ký chờ) hoặc một request đã ghi
nhận mới được xử lý — đúng yêu cầu cooperative.

**Runtime mặc định (`vis run`, `frame.enabled == false`) và final join KHÔNG đổi.** Mọi nhánh
mới nằm sau `frame.enabled` hoặc sau các collection rỗng mặc định (`cancel_req`, `cancelled`,
`cancel_on_seq`); enum `TrangThai::Huy` chỉ do helper test gán. Task không await vẫn được join
cuối (test `c4`). Không tự huỳnh task khi main kết thúc.

## 1. Contract → evidence table

| # | Contract (VD-257 §4 / yêu cầu giao việc) | Bằng chứng (test + dòng) | Engine |
|---|---|---|---|
| 1 | Cancellation request được **ghi nhận nội bộ**; không API/CLI | `frame_cancel_request_for_test` `rt.rs:786`; trigger `frame_cancel_on_seq_for_test` `rt.rs:391`, thread-local `rt.rs:386`, drain ở `frame_enable_for_test` `rt.rs:487`. Log `cancelreq:1`, `cancelon:0:1` trong `c1`. | cả hai (cùng `rt.rs`) |
| 2 | **Safe point**: dừng source, không preempt giữa lệnh | Xử lý trong `frame_next` `rt.rs:551`, `frame_fire` `rt.rs:578`, `frame_reactor_fire` `rt.rs:996`, guard `frame_wake_one` `rt.rs:636`. `c1`: stdout `main\nV:pre\n` (không `V:post`), `V:pre` đúng 1 lần, log có `cancelwait:1`/`cancel:1` nhưng **không** `wake:1`. | interp + VM (`c1`) |
| 3 | Reg/timer/waiter gỡ **đúng một lần**; không late wake, token gen được tôn trọng | `frame_cancel_one` `rt.rs:809` (deregister + xoá token/results/fired/external/ready). `c2` baseline; `c5b` `deregistered==[0]`; `c7` close ⇒ `deregistered==[0]`; `c8` timeout ⇒ `deregistered==[0]`; `c10` token0 stale sau cancel, mở lại ⇒ token1. | cả hai (engine) + direct (`Scripted`) |
| 4 | Không đóng language handle thoát task / shared | `frame_cancel_one` chỉ `deregister` đăng ký chờ, **không** gọi `close`. `c5b`: `closes.is_empty()`; `c5`: handle "C1" dùng được ở task khác sau khi task đầu kết thúc (`Ok(AAA)` rồi `Ok(BBB)`). | cả hai + direct |
| 5 | Task còn handle vẫn trả cache/error theo hợp đồng | `await_task` nhánh cache `rt.rs:1564` (giữ nguyên) + nhánh `Huy` `rt.rs:1592`. `c6`: `good` await 2 lần ⇒ `42\n42\n` dù có task khác bị huỷ/reclaim cùng phiên. | interp + VM |
| 6 | Task hết handle + join xong ⇒ thu hồi | `frame_reclaim` `rt.rs:718` mở rộng predicate sang `Huy`. `c3`: `reclaim:1`/`reclaim:2`, `live_tasks==0`, snapshot baseline. | interp + VM |
| 7 | Final join giữ nguyên; không auto-cancel | `c4`: `main\nlonely\n`, log không có dòng `cancel*`, snapshot baseline. | interp + VM |
| 8 | Close/timeout/error trong wait: deregister 1 lần, không wake nhầm | `c7` (close), `c8` (timeout deadline), `c9` (Error ⇒ `Err` đúng 1 lần, task khác vẫn `Ok(OKB)`), `c10` (stale token). | engine + direct |

Tổng: `cargo test -p vis-cli --test lat257_c_lifecycle` → **11 passed / 0 failed**.
`cargo fmt -- --check` → sạch.

## 2. Thay đổi trạng thái / chuyển tiếp trong `rt.rs` (có file:line)

- `TrangThai` thêm `Huy` (`rt.rs:49`) — terminal sau huỷ; **không** phải `Xong`, `kq` có thể vẫn
  `None` (không bịa mã lỗi). Runtime mặc định không bao giờ gán.
- `FrameTest` thêm `cancel_req` (`rt.rs:356`), `cancelled` (`rt.rs:358`), `cancel_on_seq`
  (`rt.rs:361`) — toàn bộ test/debug-only.
- Thread-local trigger `FRAME_CANCEL_TEST` (`rt.rs:386`) + setter/clear (`rt.rs:391`, `397`);
  drained trong `frame_enable_for_test` (`rt.rs:487`) để hai engine thấy cùng một trigger.

| Sự kiện | Trước | Sau | Hành động |
|---|---|---|---|
| `frame_cancel_request_for_test(id)` | bất kỳ | request ghi nhận (`cancel_req`) | Idempotent; **chưa** dọn gì (`rt.rs:786`) |
| `frame_fire`/`frame_reactor_fire` gặp `seq` khớp | request | request + xử lý ngay | Ghi nhận rồi xử lý TRƯỚC khi complete target/wake (`rt.rs:578`, `996`) |
| `frame_next` (safe point) | request còn tồn | xử lý | `frame_cancel_process_for_test` (`rt.rs:551`) |
| `frame_cancel_one` trên task `Moi`/`Pending` | đang treo | `Huy` | Gỡ waiter/reg/token/external/ready, deregister adapter, log `cancel:*`, wake awaiter (`rt.rs:809`) |
| `frame_cancel_one` trên task đã `Xong` | `Xong` | `Xong` (giữ cache) | `cancelskip:<id>:done`, **không** xoá `kq` (`rt.rs:825`) |
| Request trùng / task đã huỷ | — | — | `cancelagain`/`cancelgone`, idempotent |
| `frame_reclaim` | `Huy` + joined + hết handle | bỏ khỏi `tasks` | `reclaim:<id>` (`rt.rs:718`) |
| `await_task` gặp `Huy` | `Huy` | trả cache nếu có, else `VS0506` | Không tạo mã mới (`rt.rs:1592`) |

## 3. Cleanup / deregistration — bằng chứng

`frame_cancel_one` (`rt.rs:809`) thực hiện theo thứ tự:

1. Collect awaiter `Wait::Task(id)` TRƯỚC khi sửa `waiters`.
2. `waiters.retain(|w| w.frame != id)` ⇒ log `cancelwait:<id>` (mỗi waiter một lần).
3. Với mỗi `ReactorReg` có `r.frame == id`: `backend.deregister(r.token, …)` **mỗi token đúng
   một lần** (log `canceldereg`), xoá `token_map`, `results`, `fired` ⇒ event cũ thành stale.
4. `external.retain(id != …)`, `ready.retain(f != id)` ⇒ không re-queue, không late wake.
5. Đặt `Huy`; `cancelled.insert(id)`; log `cancel:<id>`.
6. Wake awaiter qua `frame_wake_one` — nhưng guard `cancelled.contains(&w.frame)`
   (`rt.rs:636`) chặn việc đưa chính task đã huỷ vào `ready`, kể cả khi có event đến muộn.

Các helper cleanup có sẵn (`frame_reactor_cancel_handle`, `frame_real_done`,
`frame_idle_real`, `frame_reactor_shutdown`) được **tái dùng nguyên văn**, không sửa.
`c7`/`c8`/`c10` dùng adapter `Scripted` đếm `deregister`; `c8` ép deadline quá hạn bằng
cách gán trực tiếp `ReactorReg.deadline` (không sleep) ⇒ đường timeout tất định.

## 4. Handle thoát task vs tài nguyên do task sở hữu

| Đối tượng | Owner | Bị gỡ khi huỷ task? | Bằng chứng |
|---|---|---|---|
| Waiter / đăng ký reactor `(key, gen, token)` / timer / `external` / `ready` | Scheduler (task-owned pending) | **CÓ**, đúng một lần | `c2`, `c5b`, `c7`, `c8`, `c10` |
| Language handle `"C<n>"` / `"L<n>"` (`Value::Str`) | Host `Net` / adapter + giá trị ngôn ngữ | **KHÔNG** — không gọi `close` | `c5b` (`closes.is_empty()`), `c5` (dùng lại "C1" sau khi task đầu kết thúc) |
| Result cache `kq` | Task, giữ theo hợp đồng cache | **KHÔNG** xoá | `c6` (`42\n42\n`) + nhánh cache `await_task` giữ nguyên |

Lập luận: handle mạng là chuỗi mờ do host cấp/trả cho chương trình (`net.rs:46-50`
trong baseline), có thể `return` từ task rồi dùng ở task khác. Đóng socket vì "task tạo nó
kết thúc" là sai ownership. Vì vậy `frame_cancel_one` **chỉ** huỳnh đăng ký chờ; adapter
`close()` chỉ được gọi bởi `net.close` do chương trình gọi (đường `stdlib.rs:685` không đổi).

## 5. Reclamation & diag_snapshot

- `c3`: sau final join, `snapshot == {live_tasks:0, frames:0, waiters:0, ready:0, regs:0,
  timers:0}` ở **cả hai engine**; log có `reclaim:1` (task bị huỷ) và `reclaim:2` (fake source).
- `c2`: cùng snapshot baseline cho kịch bản huỷ trong fake wait.
- `c4`: baseline cho task không await (final join chạy xong mới thu hồi).
- DiagSnapshot **không thêm field** (các test lat2/lat256 dùng struct literal 6 field) —
  bằng chứng huỷ nằm ở `frame.log` (`cancelon:`, `cancelreq:`, `cancelwait:`, `cancelregs:`,
  `canceldereg:`, `cancel:`, `cancelwake:`).

## 6. BLOCKER ghi cho A0 (không tự quyết)

1. **Mã lỗi/chuỗi kết quả khi await một task đã huỷ.** Không có mã cancellation chuyên biệt
   nào được A0 chốt. Implementation **không tạo mã mới**: task huỷ có `trang_thai = Huy`, `kq`
   giữ nguyên (thường `None`); nếu có người `await` task huỷ mà không có cache, `await_task`
   rơi về `VS0506` có sẵn với message `tác vụ đã bị huỷ…` (`rt.rs:1592`). Đây là **tái dùng**
   mã cũ, không phải lựa chọn mã mới, nhưng *hợp đồng kết quả khi huỷ* vẫn cần A0 chốt
   (VS0506? VS0505? giá trị Unit? message nào?). Phần này **chưa được ratify** — mark BLOCKER.
2. **Chính sách "lỗi của task đầu ảnh hưởng các task khác".** Không đổi: `frame_final_join`
   vẫn chỉ surfaced lỗi đầu tiên đã cache; cancellation của một task **không** kéo theo huỷ
   task khác. Nếu A0 muốn "first error ⇒ cancel remaining tasks", đó là đổi policy và nằm
   ngoài stream C — **không hiện thực**, mark BLOCKER. (Không ai được tự chọn policy mới.)
3. **Trigger injection là test-only.** Do không được sửa `FrameConfig` (thuộc `frame/**`,
   ngoài ownership), trigger huỷ được truyền qua thread-local rồi drain ở `frame_enable_for_test`.
   Đây là hooks test/debug, không phải cơ chế huỷ cho sản phẩm; một API cancellation nội bộ
   thật (nếu A0 muốn) cần thiết kế riêng và nằm ngoài ownership stream này.

## 7. Giới hạn phủ (thẳng thắn)

- Không chạy full gate / full suite / benchmark / `cargo clean` theo đúng discipline của leader
  (leader chạy serial). Chỉ chạy targeted test + `cargo fmt --check`.
- Test direct dùng adapter `Scripted` (không kernel); cơ chế token/gen/deregister được kiểm
  chứng tất định. Đường socket thật (mio) của lat256 **không chạy lại ở stream này** (tiết kiệm
  tài nguyên, tránh đụng fixture/golden của stream khác) — nhưng mã `frame_real_poll` được gọi
  qua adapter scripted nên nhánh deregister/timeout vẫn được phủ.
- Chỉ test được huỷ khi task đang ở safe point đã treo (fake wait / reactor wait). Không có
  cơ chế huỷ task đang chạy CPU — đúng yêu cầu "never preempt mid-operation"; task CPU chạy
  tới điểm nhường tiếp theo mới xét huỷ (và trong đường frame hiện tại, request chỉ sinh ra
  từ event/fire ⇒ luôn rơi đúng safe point).
- Không đổi `stdlib.rs`/`net.rs`; không đụng default runtime. `cargo test -p vis-cli --test
  lat257_c_lifecycle` → 11/11; fmt sạch; các test lat1/lat2/lat256 **không được chạy lại**
  ở stream này theo discipline (thay đổi là additive/gated nên không ảnh hưởng đường cũ).

## 8. Kết luận ngắn

Cancellation hợp tác + cleanup nội bộ đã hiện thực và kiểm chứng 11/11 trên cả hai engine
(thông qua `rt.rs` dùng chung). Handle ngôn ngữ sống sót; tài nguyên chờ gỡ đúng một lần;
no-handle task thu hồi; final join và runtime mặc định nguyên vẹn. Hai quyết định còn mở
(mã/result khi huỷ, first-error policy) đã ghi BLOCKER ở §6 và **không** tự áp đặt.
