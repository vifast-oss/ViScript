<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Checkpoint ngân sách & connect/write (VD-256, ST-276/277/278)

> **Trạng thái**: D1–D9 đã hiện thực ở mức **subset có thật**, ngân sách **chưa bật làm mặc định**.
> Checkpoint tích hợp Lát 3 đã khép tại `4150396`; runtime mặc định **không đổi**, tag/release **PAUSED**,
> chưa mở worker/GC/TLS. Step-cap parity, coverage còn lại, fairness CPU và Windows runtime vẫn mở.
>
> **Căn cứ**: VD-256 (`agent/DECISIONS.md`) · `agent/JOURNAL/vd256-{budget-report,connect-write-report}.md`.

## 1. D1–D9 → hiện thực

| Mục | Kết quả |
|---|---|
| D3 biểu diễn | **Bảng phụ S** theo PC, nhiều sự kiện có thứ tự tại 1 PC · `crates/vis-bytecode/src/fee.rs`, `fee_shared.rs` |
| D5 weight | Trailer `VFEE` serialize **chỉ `pc/kind/site`**; trọng số do runtime lấy từ bảng chuẩn có phiên bản (`WEIGHT_TABLE_VERSION=1`) |
| D7 VERSION | `fee::VERSION = 4` — **3 đã là sentinel lạ** trong `tests/decode_neg.rs:52-54` / `tests/verify.rs:68-69`, nên không ấn 3; cache key + hash ghim cập nhật **cùng đợt** (`cache.rs:28,90,211`). Core VBC1 vẫn `VERSION=2` (trailer bổ sung, không đổi MAGIC/opcode); A0 có thể yêu cầu patch riêng nếu muốn nhảy core lên 4 |
| D8 watchdog | `SAFETY_FUEL = 10^7`, mã **`VS0501-FUEL`** riêng, reset mỗi `run`, chạy sau verifier; khác chẩn đoán với `VS0501-BUDGET` |
| D9 step-cap | **Giữ OPEN**; `MAX_STEPS` chỉ alias của fuel; **không** tăng ngưỡng, không cam kết parity ở workload chạm giới hạn |
| D1/D2/D4/D6 | Catalog chung `FeeKind` (8 sự kiện) dùng bởi bytecode + interp; test so **dãy sự kiện có thứ tự** byte-identical (không chỉ tổng); verifier kiểu/site/PC/kích thước/CFG + từ chối bảng rỗng; continuation lưu trạng thái đã charge, PC sau điểm phí, không vượt thao tác chưa xong |

**Catalog chỉnh (A0 §3)** đã áp: loại phí writeback nội bộ/copy tham số/spill-restore/temporary; `E_ASSIGN` tính theo **câu gán source** (destructuring = 1, không theo số ô engine ghi); `E_EXPR` tính cho nút tổng hợp **và** con thực thi, không tính nút do lowering sinh; `await` source tính `E_AWAIT`, **final join không giả thành await source**; internal `$spawn`/`$await` không cộng `E_BUILTIN`; phân biệt method host/user (đây là một trong các caveat §3).

**Tách nghĩa vụ**: parity source (compiler + interp cùng định nghĩa, test trace có thứ tự) **không** được suy từ opcode; bảng phụ **không** tự nhận là bằng chứng instrumentation đầy đủ. Với bytecode không đáng tin: verifier cấu trúc + CFG, **không chấp nhận bảng phí rỗng**; fuel VM riêng, nếu kích hoạt trên chương trình compiler sinh trong miền hỗ trợ ⇒ lỗi thiết kế. Resume tính trùng bắt bằng **test runtime** (`resume_khong_charge_lai_nhom_da_xong`), không gán cho verifier.

## 2. connect/write (ST-277)

Trên **đường frame test**: `connect` → `Pending`, readiness rồi kiểm **kết quả nối thật** (`SO_ERROR`/`peer_addr`); `write` **partial + offset**, resume **không gửi lại**; **deadline không reset** theo readiness; token generation + cleanup đúng 1 lần; peer không đọc bị chặn bởi deadline. Chỉ đổi `stdlib.rs`; `net.rs`/`rt.rs`/API/runtime mặc định/VERSION **không đổi**. Test `lat256_net.rs` **9/9**, lặp 3 lần 8,15–8,20 s (nối thất bại, peer chậm, write lớn backpressure, đóng giữa chừng, task khác tiến triển).

## 3. Lỗi final-join (ST-278)

`frame_final_join` giờ **surfaced** `VS0506` kèm mã gốc/span trên cả hai engine — chương trình **không exit thành công** vì lỗi bị nuốt. **Không đổi** chính sách hủy các task còn lại trong cùng bản sửa (A0 §6). Có 3 dòng wiring trong `frame.rs`, đã ghi rõ trong report.

## 4. Đo & đề xuất (CHƯA DUYỆT, KHÔNG phải default)

| Workload | wall (debug) | events | cost đề xuất |
|---|---|---|---|
| loop 2M | 3,43 s | 18.000.008 | 18.000.008 |
| recursion 200 | 7,1 ms | 2.211 | 2.815 |
| tasks 1000 + final join | 5,5 ms | 14.008 | 17.008 |
| expr-chain 100k | 387 ms | 2.000.008 | 2.000.008 |

**Đề xuất**: trọng số v1 `E_STMT/E_EXPR/E_ASSIGN/E_LOOP=1`, `E_BUILTIN/E_AWAIT=2`, `E_CALL/E_SPAWN=4`;
`BUDGET = 20.000.000` (~1,1× ca loop 2M, **không kế thừa 10⁷**); fuel VM giữ 10⁷ lệnh. Chưa có số `Q`
(hạn ngạch lượt chạy) — nợ riêng, không tự ấn. Cần đo release/phân lớp sâu hơn trước khi A0 chốt.

## 5. Validation

- `./vis-dev gate --fast` trên cây tích hợp: **1512 passed / 1 skipped (measurement) / 0 failed**.
- `cargo test -p vis-interp -p vis-cli -p vis-bytecode`: **1151 / 0 / 1 ignored**; `lat256_budget` 8/1 ignored; `lat256_net` 9/9 ×3; `cargo test -p vis-vm`: ok.
- `./vis-dev check`: PASS (fmt/clippy/R1..R6/self-test/report/doc+readme examples).

## 6. Trung thực về phạm vi & còn mở

Instrumentation là **subset có thật**, không phải đầy đủ: chưa phủ hết field/index assignment parity,
phân biệt user-vs-host method, và double-charge khi suspend/resume thật trong mọi đường. Bảng phụ **không**
được coi là bằng chứng source instrument đầy đủ. Ngân sách mới **chưa bật**, chưa thay guard cũ.

Còn mở có chủ đích: `?`/`chan`/`CmdBlock`, `await` trong thân closure trên VM, cancellation/cleanup đầy
đủ, fairness CPU, Windows runtime, step-cap parity, BUDGET/trọng số/Q chưa chốt. **Điều kiện đổi runtime
mặc định (VD-255 §5) vẫn chưa đạt.**

## 7. Quyết định A0 tiếp

1. Chốt hay chỉnh trọng số v1 / `BUDGET = 20.000.000` / fuel 10⁷ (đều **chưa approved**).
2. Có chấp nhận `fee::VERSION=4` + core VBC1 giữ 2, hay yêu cầu patch riêng nhảy coreVersion.
3. Chốt `Q` (hạn ngạch CPU) — hiện chưa có số.
4. Tiếp tục các gap coverage/fairness/Windows trước khi考虑 đổi runtime mặc định.

Tag/release PAUSED; worker/GC/TLS chưa mở. **Ngân sách là luật ổn định của chương trình, không phải hóa
đơn cho thao tác nội bộ khác nhau giữa interp và VM.**
