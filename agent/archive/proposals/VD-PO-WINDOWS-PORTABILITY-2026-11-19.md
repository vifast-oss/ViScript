# VD-PO-Windows-portability — proposal (sở tái lập) — 2026-11-19

> Agent 1 → A0 · **chờ A0 dispatch lệnh mở VD** · dựa trên kế hoạch chi tiết `agent/PLAN-WINDOWS-L0-PORTABILITY-2026-11-19.md`.
> **Đã mở 2026-11-19** (lệnh A0-dispatch ủy thác); **đang triển khai** — nhóm A–E đã fix; Windows verify tiến độ xem `agent/SNAPSHOT-2026-11-20.md`.

## 0. Tóm tắt

Đề xuất mở VD **"Windows test-suite portability"** (Tên: `VD-PO-Windows-portability`). Mục tiêu duy nhất: các test hiện diện dưới dạng POSIX/Unix trên Windows trở thành xanh — hoặc port sạch sang Windows. **Không** mở rộng ngôn ngữ, **không** thay đổi ngữ nghĩa, **không** đụng `pha1-rc1`.

## 1. Phạm vi

### In
- (A) **POSIX binaries** — capture bằng `test-child` Rust binary emulating echo/cat/grep/printf/true/false/sleep/sh exit và hàm `target/…/test-child` từ Cargo.toml, dùng `env!("CARGO_BIN_EXE_test-child")` thay `/bin/*`. Plan chi tiết: `agent/PLAN-WINDOWS-L0-PORTABILITY-2026-11-19.md`.
- (B) **fixture path / CRLF / JSON expectation** — Windows path forward-slash; CRLF normalized via helper `read_file_normalized(path) -> String` (strip `\r\n→\n`).
- (C) WSL stub `sync_list` — Direct-call thay `bash -lc`.
- (D) WinSock test `tcp_tu_choi` — assert "refused" OR "connection timed out" với timeout; Tam() ref route.
- (E) `vsh1b::long_tren_thu_muc_that` — assert `mode == "-" || mode == "100600"` (helper).

**Chuẩn expose**:
- Trên Windows: `cargo test --workspace --no-fail-fast` → **không lỗi** (failures = 0). Hoặc nếu bỏ ca thực sự POSIX-only (unix integration only), số lượng ca bị `#[cfg(unix)]` phải **không vượt quá** ~50% tổng số ca liên quan process (bù lại bằng dedicated `windows` integration ca ánh xạ tính năng).

### Out
- Không đổi API/ngữ nghĩa, không thêm opcode, không đổi VBC1.
- Không giới thiệu độ an toàn/quyền mới; không mở async mới; không hoàn thiện `net http TLS`/`HTTPS`; không thêm dict-ordering mới ngoài những gì đã tồn tại.

## 2. Thời điểm mở VD

- **Sau khi A0 ra lệnh** — hiện tại READY (pha 1 đóng chính thức); VD chỉ mở khi A0 xác nhận.
- Khi mở: implementation do Agent 2 thực hiện theo plan; review chéo sau implementation (VD-portability khác tác giả).

## 3. Giá thành ước lượng

| Loại | Số ca | Kinh phí |
|---|---|---|
| A test-child harness | ~44 | 1 phiên Agent 2 (~2-3 giờ) |
| B path/CRLF fix | ~45 | 1 phiên + fixture sweep |
| D winSock | 4-5 | < 1 giờ |
| E pattern | 1 | < 30 phút |
| C WSL stub | 4 | < 1 giờ |
| Verify (Windows re-verify) | — | 1 phiên |

Tổng: 2-3 phiên; không động đến API freeze VBC1 (hoàn toàn input-only).

## 4. Blockers / Risks

1. **Hội tụ Windows+Linux** — Rust `test-child` chay hoàn toàn trên MSVC + GNU; chỉ cần Cargo.toml có `[[bin]]` test_child.
2. **TestChild behavioral fidelity** — `test-child grep -E "X" file`, `test-child cat`, `test-child sh -c "exit 3"` phải khớp một số Rust pattern-matching cho từng lệnh mẫu. Không require extra deps nếu in-memory.
3. **Synchronize với `report-check`** — 30 báo cáo robust vs `windows-addendum` (AGENTS.md mention `vis-dev` PATH phải chứa cargo 1.99.1+ — file "r14c" `QUY DINH CA THÊ 75`).

## 5. Approval A0 yêu cầu

1. **Approve mở VD-portability** — implicit semantics:
   - Xác nhận tên VD: `VD-PO-Windows-portability`
   - Tác động (như trên)
   - VD ở cấp `agent/` — không di chuyển `pha1-rc1`; bỏ giai đoạn B-3/`typed serialization`/Tier 1 trước khi mở.
2. **Hoặc quyết định giữ block** — thì không cần làm gì khác.

## 6. Ref assets

- `agent/PLAN-WINDOWS-L0-PORTABILITY-2026-11-19.md`
- `agent/WINDOWS-BASELINE-2026-11-19.md` + `Addendum`
- `agent/DEMOS.PPPPP-PO-Windows-portability.md` (sẽ tạo khi bắt đầu portability)
- `agent/QUEUE.md` L1994: READY
- `agent/DEM-2026-11-18.md`: Addendum #6 audit docs

---

**Status tài liệu**: nháp, chưa có nội dung implementation. Vd sẽ được viết đầy đủ nếu A0 dispatch "PORTING APPROVED".
