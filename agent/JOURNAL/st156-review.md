<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-156 — review Agent 1 (SQLite optional + ma trận core/SQLite, VD-242 Q6-A)

**Nguồn**: worktree `vd242-st252-sqlite-feature`, commit Agent 2 `a0e2efd`. **Người review**: Agent 1. **Ngày**: 2026-11-20.

## 1. Đối chiếu VD-242 Q6-A / §5 SQLite-target

| Yêu cầu | Kết quả | Verdict |
|---|---|---|
| SQLite là feature tùy chọn, default bật cho bản host | `default=["sqlite"]`, `sqlite=["dep:rusqlite"]`, `rusqlite optional=true`, giữ `=0.37.0`/`bundled`/`hooks` | **Đạt** |
| Tách ma trận core / SQLite, có tên riêng | `core (--no-default-features)`: 18 PASS / 0 EXPECTED_RED / 0 FAIL; `sqlite (default, cấu hình cũ)`: 16 PASS / 2 EXPECTED_RED / 0 FAIL | **Đạt** |
| Không đổi đỏ thành xanh bằng thay cấu hình không nhãn | 2 ô cũ vẫn đỏ, in `EXPECTED_RED` + lý do `#156: libsqlite3-sys cần C toolchain...`; 16/18 lịch sử giữ nguyên | **Đạt** |
| Mức bằng chứng | Ghi rõ **chỉ `cargo check --target`**, không chạy Windows/wasm | **Đạt** |
| Core build từ chối `std.sqlite` rõ, không panic/hành vi giả | `db/disabled.rs` giữ chữ ký + impl `DbHost`; `hop_nhat_fs` trả `Result.Err(Error)`; thử cả interp + `--vm` với `test/sem/sqlite/mo-dong.vis` ra đúng chẩn đoán | **Đạt** |
| Không đổi VERSION/opcode/freeze API | Đúng | **Đạt** |

## 2. Finding đã xử lý đúng phạm vi

`crates/vis-cli/tests/sqlite.rs` dùng trực tiếp `rusqlite::Connection` (WAL) ⇒ với `--no-default-features` lỗi `E0433`. Agent 2 **không đụng `tests/`** (ngoài quyền) mà thêm `[[test]] name="sqlite" required-features=["sqlite"]` trong `Cargo.toml` (file được cấp quyền). Default vẫn chạy test như cũ. **Chấp nhận** — không đổi kỳ vọng, không hạ tiêu chí.

Noise `/tmp/vrepfx.out: Permission denied` của `test/report/run_fixtures.sh` là có sẵn (5 fixture âm đều `ok`, report-check PASS), không phải hồi quy.

## 3. Kết luận

**SHIP ST-156.** Merge `a0e2efd`; #156 có thể **đóng theo phạm vi A0 đã chốt** (ma trận đúng cấu hình + phạm vi hỗ trợ rõ; không bắt buộc mọi target xanh) — phần C toolchain msvc/wasm32 vẫn là giới hạn được công bố, không phải blocker phát hành. Sau merge: gate/fresh-clone → Windows trên ứng viên cuối → phiếu nghiệm thu tổng hợp.
