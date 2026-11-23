<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-274 / VD-255 · Báo cáo đóng khoảng trống coverage — đường frame test

**Trạng thái**: mã đã sửa + test passed, **KHÔNG commit** (theo lệnh giao).
**Phạm vi**: evaluator frame phía **interp** (`frame.rs` + 2 module con) và test coverage
mới. Đường **VM** chạy bytecode thật nên hầu hết cấu trúc đã có sẵn; không sửa `vm_frame.rs`.
Runtime mặc định, bytecode/VERSION/cache/checker **không đổi**.

---

## 0. Bất biến cứng — xác nhận

| Bất biến | Kết quả |
|---|---|
| Checker acceptance không thu hẹp | ✅ Không sửa checker; `tien_trinh()` trong mọi test mới đều `check().is_clean()`. Các fixture âm vẫn bị checker từ chối (test `z_all_async_fixtures…`: class `negative`). |
| Không fallback âm thầm sang đường đồng bộ | ✅ Mọi cấu trúc ngoài phạm vi Trap RÕ ngay tại điểm gặp (test `z_unsupported_structures_still_rejected_clearly`: `?`, `chan` ⇒ exit=1 + "chưa được đường frame test hỗ trợ", stdout rỗng). |
| Resume không re-eval / side-effect đúng 1 lần | ✅ Các ca A1/A2/A3/C1/C2 so stdout **chính xác** (đếm `BG`, `SIDE`, `ADD`); pending lưu continuation có đối số đã eval (`Redispatch`, `ApplyAwait` re-push operand). |
| Runtime mặc định không đổi | ✅ `vis run` / `vis run --vm` chạy nguyên văn; test so khớp default-path ở B1/C1/fixtures. Không sửa `interp.rs::run_program`, `vm.rs`, `vm_frame.rs`. |
| VERSION / bytecode / cache / API | ✅ Không đổi `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, VERSION, opcode, cache-key. Không thêm CLI flag hay API ngôn ngữ. |

---

## 1. Tiêu chí chấp nhận → bằng chứng

| # | Tiêu chí (VD-255 §3) | Bằng chứng |
|---|---|---|
| 1 | Mỗi ô coverage có test **hoặc** lý do không áp dụng | §3 (A/B/C: 8 test dương) + §5 (ô ngoài phạm vi có lý do + hành vi từ chối). |
| 2 | Test dùng **nguồn thật** → lower → check → HIR/bytecode thật → 2 engine | `lat255_coverage.rs::chay` (source → `vis_types::lower_with` → `check` → `vis_bytecode::compile` → `run_i` + `run_v`), so stdout/exit/diag/log/snapshot byte-for-byte. |
| 3 | Suspend ở điều kiện / đối số / toán hạng / thân vòng / nested call | A1 (điều kiện if), A2 (thân while), A3 (toán hạng `and`), B2 (đối số + chỉ số), C1 (đối số của closure call). |
| 4 | Resume không re-evaluate; side effect 1 lần | A1 `matches("BG")==1`; A2 `matches("BG")==1`; A3 stdout exactly `"SIDE\nand-yes\nor-yes\nSIDE\nfalse\n"` (RHS `or` bị skip); B2/C1/C2 so stdout tuyệt đối. |
| 5 | Closure chia ô bắt → writeback qua suspend | `c1_closure_shared_capture_writeback_across_suspend`: `add(await a)` treo ⇒ `add(7)` thấy `n=40` ⇒ in `47`; C2: 3 frame gọi cùng closure thấy `2 → 42 → 42`. Ô bắt được **snapshot vào `Rc` mới lúc tạo** (khớp runtime mặc định), **dùng chung giữa các lần gọi** ⇒ writeback; `Rc` không tự tham chiếu (sửa lỗi rò handle đã phát hiện). |
| 6 | 26 fixture async đều được **phân loại**; golden không đổi hàng loạt | `z_all_async_fixtures_classified_and_run_through_frame_path` + bảng §4. **0 file `.out`/`.err` nào bị sửa.** |
| 7 | Không fallback đồng bộ để che ô hở | Xem §0 + §5. |

---

## 2. Validation (số liệu thật)

- `./vis-dev check`: **PASS** (fmt OK, clippy OK, luật R1..R6 OK, self-test âm OK, build OK, doc-examples OK, readme-examples 20/20).
- `cargo test -p vis-interp -p vis-cli`: **1041 passed / 0 failed** (108 tổng kết test-suite).
- Frame-specific: `lat1_frame` **8/8**, `lat2_reactor` **13/13**, `lat3_real` **12/12**, `lat255_coverage` **9/9**.
- Kích thước file (R1 ≤1500): `frame.rs` 1166, `frame/proc.rs` 936, `frame/expr.rs` 726 dòng.

---

## 3. Chi tiết thay đổi theo nhóm (file:line)

### Cấu trúc mã
`crates/vis-interp/src/frame.rs` bị tách thành 3 file để qua R1 (pattern giống `interp/{goi,pat}.rs`):
- `frame.rs` — docs, `FrameConfig`/`FrameOutput`, `Work`/`Callee`/`LoopState`/`CallFrame`/`IFrame`,
  helper (`block_works:453`, `initial_work:469`, `bind_pat_env:494`, `field_of:613`,
  `index_of:649`), driver `run_frame_for_test:718`, `eval_expr_in:987`,
  `step_frame_owned:1033`, `if_work:1065`, `args_works:1108`.
- `frame/proc.rs` — `process_work` (continuation stack; 936 dòng).
- `frame/expr.rs` — `eval_work:7`, `exec_work:340`, `call_value:525`,
  `classify_callee:600`, `dispatch:637`.

Một dòng visibility duy nhất ở `crates/vis-interp/src/interp.rs`: field `types` ⇒ `pub(crate)`.
`rt.rs` **không đổi** (mọi instrument debug đã gỡ). `vm_frame.rs` **không đổi**.

### Group A — điều khiển
| Cấu trúc | Vị trí |
|---|---|
| `if`/`elif`/`else` (stmt + expr), `if let` | `frame.rs::if_work:1065`; `proc.rs::EvalCond:189`, `ApplyCond:223`, `ApplyCondLet:236`; `expr.rs::eval_work` (`HExpr::If`) / `exec_work` (`HStmt::If`) |
| `match` (stmt + expr) + guard + pattern | `proc.rs::MatchArm:627`, `MatchGuard:672`, `MatchSubj:693`; `expr.rs` (`HExpr::Match`/`HStmt::Match`); binding `frame.rs::bind_pat_env:494` |
| `while` / `loop` / `for` / `while let` | `frame.rs::LoopState/LoopKindState`; `proc.rs::LoopInit:263`, `LoopStep:275`, `LoopEnter:327`, `LoopExit:333` |
| `break` / `continue` | `proc.rs::DoBreak:340`, `DoContinue:358` (unwind work stack tới `LoopMark` gần nhất) |
| `&&` / `||` (short-circuit) | `expr.rs` (`BinOp::And/Or` ⇒ `ApplyShortCircuit`), `proc.rs::ApplyShortCircuit:587`, `ApplyBoolRhs:605` |
| `??` / `\|>` | `proc.rs::ApplyCoalesce:615`, `ApplyPipe`/`ApplyPipeCall` |
| Test | `lat255_coverage.rs::a1:123`, `a2:167`, `a3:197` |

Charging: **không implement ngân sách** (đúng lệnh); thứ tự eval/charge được giữ nguyên bằng
cách đẩy operand theo thứ tự nguồn và chỉ eval vế thực sự chạy (xem `vd252-charging-final.md`).

### Group B — biểu thức & gọi
| Cấu trúc | Vị trí |
|---|---|
| Field / `?.` / index | `expr.rs` (`HExpr::Field/Index`), `proc.rs::ApplyField:387`, `ApplyIndex:412`, `frame.rs::field_of:613`, `index_of:649` |
| Record / tuple / list / set / map / enum / opt / `with` / Interp | `expr.rs::eval_work`; `proc.rs::EvalRecordFields:458`, `BuildRecord/BuildEnum/BuildList/BuildTuple/BuildSet/BuildMap`, `WithCollect:497`, `InterpParts` |
| Đối số tên + default | `frame.rs::args_works:1108`, `take_call_args_names:1140`; `expr.rs::dispatch` (named + default qua `eval_expr_in`), `proc.rs::ApplyNamedCall:708` |
| Toán tử một ngôi / còn lại | `proc.rs::ApplyUn:480`, `ApplyBin`; `crate::ops::{bin,un}` thật |
| Hằng cấp tệp | `frame.rs::run_frame_for_test:718` — evaluate const init trước main vào `globals: Env` dùng chung (mirror `interp.rs:229`) |
| Gán `var` / trường / phần tử | `proc.rs::ApplyAssignVar:845`, `ApplyAssignField`, `ApplyAssignIndex` (dùng `gan_truong`/`gan_chi_so` thật) |
| Test | `lat255_coverage.rs::b1:237`, `b2:273` |

### Group C — closure & writeback
| Cấu trúc | Vị trí |
|---|---|
| Gọi closure / builtin value / record ctor | `expr.rs::call_value:525`; `proc.rs::ApplyValueCall:729`, `RedispatchValue:746` |
| Ô bắt chia sẻ + writeback | `proc.rs::MakeLambda:829` (snapshot `Rc` mới), `call_value` (params ghi vào `c.env` chia sẻ, hoàn nguyên khi pop frame) |
| Nhiều frame qua suspend | `frame.rs::run_frame_for_test` — `frames: HashMap<u32, IFrame>` giữ nguyên `calls`/`vals` qua `Abort::Pending`; `need_join:886` xếp task `tao` bị await vào ready |
| Test | `lat255_coverage.rs::c1:308`, `c2:343` |

---

## 4. Phân loại 26 fixture `test/sem/async/*.vis`

> Lệnh giao ghi **35 ca**; số liệu thực tế trên đĩa là **26 tệp `.vis`** (19 `.out` + 7 `.err`).
> Báo cáo dùng số thực tế và khẳng định đủ 26 đều có một dòng trong bảng phân loại.

| Lớp | Số | Fixture | Hành vi |
|---|---|---|---|
| `frame` | 16 | async-goi-async, async-khong-goi-khong-chay, await-co-ban, await-hai-lan-cache, await-trong-vong-lap, de-quy-async, grid-await-if, hai-task-thu-tu, print-trong-task-thu-tu, spawn-nhieu-join-cuoi, spawn-trong-ham, task-in-trong-than, task-khong-await-van-chay, task-tra-list, task-tra-record, task-tra-str | Chạy QUA frame path, stdout **bằng `.out`**, 2 engine đồng nhau. |
| `frame-err` | 1 | than-task-loi-await | Chạy qua frame path, exit=1, `VS0506`. |
| `frame-sched` | 1 | grid-await-nghich-thu-tu | Frame path chạy được, 2 engine đồng nhau **nhưng thứ tự task là cooperative FIFO** (`task 1` trước `task 2`) khác golden run-to-completion (`task 2` trước `task 1`). Golden **không đổi**; runtime mặc định vẫn trả đúng golden (test khẳng định). Đây là hệ quả cơ chế, KHÔNG phải lỗi semantics. |
| `out-join-err` | 1 | than-task-loi-join | **NGOÀI phạm vi VD-255**: lỗi VS0506 chỉ phát khi final join (task không ai await); frame path hiện trả exit=0. Runtime mặc định vẫn báo VS0506 đúng `.err`. Không sửa ở đợt này. |
| `negative` | 5 | async-lam-gia-tri, await-khong-phai-task, spawn-co-ten, spawn-khong-async, task-khong-phai-int | Checker từ chối (VS0412/VS0201) — fixture âm, không chạy frame. |
| `chan` | 2 | grid-chan-hai-task-kenh, grid-kenh-list | Dùng kênh ⇒ ngoài scope; frame path **từ chối RÕ** (exit=1, "chưa được … hỗ trợ"); golden `.out` giữ nguyên, runtime mặc định vẫn đúng. |

**Golden changes: KHÔNG CÓ.** Không tệp `.out`/`.err` nào trong `test/sem/` bị sửa.
Thay đổi test duy nhất ngoài file mới: `lat2_reactor.rs::r9/r10` được đổi từ "expect rejection"
sang "expect support" vì `&&`/`||` và hằng cấp tệp giờ là tính năng được phủ (VD-255), kèm
comment giải thích; số test vẫn 13.

---

## 5. Các ô CHƯA phủ — trung thực

| Cấu trúc / tình huống | Hành vi hiện tại | Lý do |
|---|---|---|
| `HExpr::Try` (`?`), `HExpr::ChanNew` (`chan`), `HExpr::CmdBlock`, `HExpr::Error` | Trap RÕ `"frame test: HExpr … chưa được đường frame test hỗ trợ"` ngay khi gặp (trước khi đẩy operand) | Ngoài scope VD-255; test z_... kiểm chứng. |
| Gán ghép trên **trường/phần tử** (`x.f += 1`, `x[i] *= 2`) | Trap RÕ "gán ghép … chưa được đường frame test hỗ trợ" | Chỉ phép `=` (dùng `gan_truong`/`gan_chi_so` thật); checker/runtime mặc định không đổi. |
| Lỗi ở **final join** (task lỗi không ai await) | Frame path exit=0 (xem `than-task-loi-join`) | Cần lan lỗi join cuối qua driver — không nằm trong các group A/B/C; để lại, ghi rõ ở §4. |
| **Suspend bên trong thân closure** (`let f = fn(): await t`) | Không có test parity 2 engine: interp chạy được, nhưng codegen VM sinh bytecode lỗi (`bytecode hỏng: ngăn xếp rỗng`) cho biến thể này | Lỗi bytecode/codegen VM có sẵn, ngoài phạm vi sửa của subtask. Các test C đặt `await` ở **đối số lời gọi closure** — vẫn là suspend + writeback + multi-frame thật trên cả 2 engine. |
| `with … \|>` (pipe ở vị trí biểu thức phức) | Test B1 dùng `with` rồi gán biến, không dùng pipe; VM từ chối pipe ở vị trí đó ("toán tử `Pipe` phải xử lý ở tầng biểu thức") | Hạn chế codegen VM có sẵn; interp hỗ trợ `ApplyPipeCall` nhưng không dùng để tuyên bố parity. |
| `Set`/`Map`/`EnumLit`/`Interp`/`let-else` | Đã implement continuation, **chưa có test riêng** trong `lat255_coverage.rs` | Nằm trong mã nhưng chưa được ca dương chuyên biệt; coverage được tính theo ô scope VD-255 (§1), các ô này là hạ tầng không bắt buộc. |
| Ngân sách/điểm charge (`E_CALL`, `E_LOOP`, `E_EXPR`, …) | **Không implement** | Theo lệnh: chỉ giữ thứ tự eval/charge, không đụng budget (`vd252-charging-final.md` còn mở trọng số). |

---

## 6. Kết luận

Khoảng trống coverage giữa "checker chấp nhận" và "đường frame test phủ" đã được thu hẹp
cho toàn bộ phạm vi VD-255 §3 (A/B/C) trên **cả hai engine**, với 9 test nguồn thật mới và
26/26 fixture async được phân loại. Runtime mặc định, bytecode/VERSION/cache/checker và
mọi golden **giữ nguyên**. Phần còn lại (§5) đã nêu rõ cả hành vi từ chối lẫn lý do, không
dùng fallback đồng bộ để che.
