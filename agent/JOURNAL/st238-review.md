<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-238 — review Agent 1 trước merge (VD-242 / lệnh A0 “review từng finding”)

**Nguồn**: worktree `vd242-st238-fixA`, commit Agent 2 `93368c3`. **Người review**: Agent 1. **Ngày**: 2026-11-20.
**Nguyên tắc**: không đóng cả nhóm bằng một dòng gate xanh; đối chiếu từng finding với căn cứ đã chốt (VD-242, ST-234, ST-235, VD-243).

## 1. Từng finding

| Finding | Kết quả đúng có căn cứ | Bằng chứng Agent 2 | Đối chiếu của Agent 1 | Verdict |
|---|---|---|---|---|
| Parity 1 — `for` lồng + `break` | `0 1 done`, exit 0; `break` thoát vòng trong | đỏ-trước VM lặp tới `VS0501` (101); sau sửa VM = interp | Sửa ở `Th::dat_pc`/`loop_ends` (đích danh nguyên nhân `codegen.rs:603-631` + `vm.rs:840-858`); có `st238_parity.rs` | **SHIP / đóng #157** |
| Parity 2 — `while let` trong `if` | `after`, exit 0; nhánh sai `Pop` giá trị pattern | đỏ-trước VM lỗi verifier “hai đường tới độ sâu 1 và 0” | Sửa đúng chỗ `Pop` nhánh sai; verifier vẫn chạy (không hạ ngưỡng); `break` nhảy qua `Pop` có test | **SHIP / đóng #158** |
| Parity 3 — `let` cấp tệp trong hàm | `101`, exit 0; mô hình script phẳng | đỏ-trước VM `Add … 1 và ()` | Dùng `GLOBAL_SLOT_BASE = 0x8000` ánh xạ sang kho chung; **không thêm opcode**, `VERSION`/serialization không đổi (đã grep `format.rs`/`lib.rs`/`verify.rs`); A0 chưa xếp lại mô hình #159/#160 ⇒ giữ phạm vi | **SHIP / đóng #159** |
| Parity 4 — `const` cấp tệp | `10 11`, exit 0 | đỏ-trước VM `()` rồi `VS0501` | Inline tại chỗ dùng (`HExpr value`), khớp interp | **SHIP / đóng #160** |
| Parity 7 — `match` rơi nhánh | Báo lỗi nhất quán, **không im lặng `after`** — VD-242 §3.1 | đỏ-trước VM exit 0; sau sửa trap y hệt interp exit 101 | Sentinel `$vis_match_no_arm` là builtin nội bộ (quy ước `$` như `$value:`), **không** lọt bề mặt người dùng; cùng thông điệp/span | **SHIP / đóng #163** |
| Parity 8 — closure ghi biến bắt `var` | `11 21 1`, exit 0; ô bắt chia sẻ giữa các lần gọi | đỏ-trước VM `11 11 1` | Dùng `Rc<RefCell>` theo địa chỉ closure + write-back khi pop khung; có test 2 biến bắt/2 closure | **SHIP / đóng #164** |
| #167 `net.read` abort | `Err` có cap 8 MiB; kiểm âm/overflow/chuyển kiểu; OOM input không chấp nhận — VD-242 §4 | đỏ-trước exit **134**; sau sửa `Err(… 8388608 byte)`, exit 0, hai engine cùng byte | Có hàng rào thứ hai ở host (`net.rs`); lỗi là **dữ liệu** ⇒ `Err`, không trap (VD-241 pre-merge §1) | **SHIP / đóng #167** |
| #169 `json.encode` non-finite | Lỗi, không `inf`/`NaN`, không tự `null` — VD-242 Q4b + §3.5 | đỏ-trước in `inf` exit 0; sau sửa `VS0501 … giá trị số không hữu hạn`, exit 101 | Chặn **trước** encode bằng `co_so_phi_huu_han()` đệ thuỳ list/map/record/enum; đúng Q4b (override đề xuất `null` của audit) | **SHIP / đóng #169** |
| #181 `--dry`/`--sandbox-check` nuốt lỗi cú pháp | `VS0201`, exit 2 (khớp `vis check`) | đỏ-trước exit 0 + “không khai báo quyền” | Kiểm parse trước khi xét quyền; đúng ST-235 #181, không đổi ngữ nghĩa quyền | **SHIP / đóng #181** |

## 2. Điểm báo rõ (không chặn merge)

1. **Quy ước ô global `0x8000`**: là ánh xạ nội bộ VM + miễn trừ `LocalOverflow` trong verifier cho ô `>= GLOBAL_SLOT_BASE`; **không** thêm opcode, không đổi `VERSION`/`MAGIC`/định dạng VBC1. Agent 2 đề nghị nếu A0 muốn đưa vào hợp đồng VBC1 thì mở VD riêng — Agent 1 **ghi nhận**, không mở VD trong đợt này (VD-242 cấm đổi bytecode).
2. **Hồi quy đã tự sửa trong lúc làm**: khai `var` dùng `f.slot` thay vì `slot_cua` ⇒ 10k async task VM trả `()`; đã fix + test lại xanh. Ghi nhận để theo dõi, không phải finding mới.
3. **Nợ cũ ngoài phạm vi**: (a) ca 5/6/9/10 thuộc luồng B (ca 6 đã có VD-242 Q2 + §3.2); (b) chương trình **không hợp lệ** có cả `let` cấp tệp lẫn `fn main` vẫn lệch thông điệp giữa hai engine (ST-28, có từ trước, **không** phải hồi quy ST-238) ⇒ ghi nợ riêng nếu cần, không đưa vào đợt này.

## 3. Kết luận review

**SHIP từng finding (9/9).** Merge vào `main` theo đúng phụ thuộc codegen, sau đó chạy lại `./vis-dev check`, `./vis-dev gate --fast`, `vis test test/sem` và `--vm` **trên commit tích hợp**, không dùng kết quả worktree thay kết quả main.
