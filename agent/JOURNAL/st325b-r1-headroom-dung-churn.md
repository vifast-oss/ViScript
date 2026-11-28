<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-325b — R1 headroom `vm_frame.rs` — **DỪNG** (churn > 15)

- **Ngày**: 2026-11-28 · **Vai**: Builder (Agent 2) · **Nhánh**: `plan/st325b`
- **Nền**: `main` @ `37ae9c35d998ea490d21642fdaca6e610a550c7b`
- **Worktree**: `/home/ps/viswt/st325b` — **không** chạm repo chính, **không** merge/push.
- **Trạng thái**: **DỪNG ở bước 1 (đo/churn trước)**. **Không sửa một dòng mã**, không tách tệp,
  không chạy cổng. Chỉ commit checkpoint ghi lý do theo đúng điều kiện dừng của đề bài.
- **Nguồn đề xuất**: `agent/QUEUE.md` dòng ST-325b (W · Sóng 14 / R1 headroom), mở bởi
  `37ae9c35 [ST-00] VD-311: mo Song 14 (ST-325a/b/c/d)`.

## 0. TL;DR

Mục tiêu đề ra: `crates/vis-vm/src/vm_frame.rs` **958 dòng** → **< 900** bằng cách move
nguyên văn theo trách nhiệm sang submodule `vm_frame/` + barrel `pub use` (giữ API `Vm`/frame
methods, 0 unsafe, không bump `API_VBC1`/cache).

**Điều kiện DỪNG trong đề bài kích hoạt**: churn **23 commit / 90 ngày** > ngưỡng **15**.
Theo lệnh "*nếu churn rất cao (>15) hoặc API công khai đổi khi tách ⇒ DỪNG, journal ghi lý do,
commit checkpoint (không sửa ẩu)*" — tác vụ bị dừng trước khi đụng vào code.

## 1. Đo churn (bước 1 — bắt buộc)

Lệnh: `git log --oneline -20 -- crates/vis-vm/src/vm_frame.rs` và
`git log --since='2026-08-30' --oneline -- crates/vis-vm/src/vm_frame.rs | wc -l`.

| Chỉ số | Giá trị | Ngưỡng | Kết luận |
|---|---:|---:|---|
| Tổng commit chạm tệp (toàn lịch sử) | **23** | — | — |
| Commit trong 90 ngày (từ 2026-08-30) | **23** | > 15 ⇒ DỪNG | **DỪNG** |
|LOC hiện tại | **958** | mục tiêu < 900 | chưa đạt (không sửa) |
| Disk `/` | 29 G available | < 5 G ⇒ DỰNG | OK, không chạm ngưỡng |
| Disk `/tmp` (tmpfs) | 5.2 G available | — | OK (dùng TMPDIR đĩa nếu chạy) |

Mốc 90 ngày (`2026-08-30`) muộn hơn **mọi** commit của tệp: 23 commit trên đều nằm trong
`2026-11-20 … 2026-11-26` — tức **toàn bộ lịch sử tệp dồn trong 7 ngày**, mật độ ~3,3 commit/ngày.

20 commit gần nhất (đã liệt kê đủ trong đề bài) cho thấy các tay sửa lớn, liên tiếp chạm cùng tệp:
`ST-316a` (Value::List CoW), `ST-315a` (budget_charge O(1)), `ST-313a` (tách vis-runtime,
"vm_frame < 1000 LOC"), `ST-312a`, `ST-311a` (vá #195 loop_ends O(1) + watchdog + tách
`vm/exec.rs`), `ST-301`, `ST-292`, `ST-290`, `VS3/W5`, `VS3/W1` (×4), `VD-260/259/258/257/256`…

Đây là vùng nóng đang được chỉnh sửa tích cực bởi nhiều luồng (W1/W5, ST-311…316), không phải
tệp ổn định có thể move nguyên văn an toàn.

## 2. Lý do dừng (diễn giải)

1. **Churn vượt ngưỡng rõ ràng**: 23 > 15. Đề bài quy định đây là điều kiện dừng cứng.
2. **Rủi ro thực tế**: một phần tách đã tồn tại (`crates/vis-vm/src/vm_frame/budget.rs`, 278 dòng)
   nhưng 958 dòng còn lại vẫn là mã lõi frame/suspend/budget được 23 commit gần đây đụng vào.
   Move nguyên văn lúc này dễ gây xung đột merge với các nhánh W1/W5 đang mở, và mỗi lần move
   làm các diff sau đó của W1/W5 mất ngữ cảnh blame.
3. **Không sửa ẩu**: đúng chỉ dẫn, không cố cắt/gộp hàm để đạt <900 bằng tay, không đổi chữ ký,
   không đụng API `Vm`/frame methods.

## 3. Những điều đã kiểm nhưng KHÔNG kích hoạt

- Disk `/` 29 G ≫ 5 G ⇒ không dừng vì disk.
- Không có thay đổi mã ⇒ không phát sinh thay đổi API công khai (điều kiện dừng thứ hai không
  được kiểm chứng vì không tách; ghi nhận để A0 biết).

## 4. Cổng / kiểm chứng

| Hạng mục | Trạng thái | Ghi chú |
|---|---|---|
| `cargo fmt --all` | **không chạy** | không có thay đổi mã để format |
| `cargo test -p vis-vm -p vis-cli -p vis-conformance` | **không chạy** | dừng ở bước 1 |
| `./vis-dev gate --fast` | **không chạy** | dừng ở bước 1 |
| `VIS_DIFF_N=100000 ./bin/vis-diff` (`lech=0`) | **không chạy** | dừng ở bước 1; không đụng runtime/VM |
| `cargo clippy --workspace --all-targets -- -D warnings` | **không chạy** | dừng ở bước 1 |
| Log `/home/ps/vislogs/st325b-*.log` | **không tạo** | không có lệnh nào chạy |

`git status` trước commit: sạch (không có file mã bị đổi) + 1 tệp journal mới.

## 5. Commit checkpoint

`[ST-325b] R1 headroom: tach vm_frame.rs DUNG: churn/API`

Chỉ chứa journal này. Không merge, không push, không đụng `agent/QUEUE.md`/`STATE.md`/
`DECISIONS.md` (đây là việc của Agent 0/Gatekeeper).

## 6. Đề xuất cho Agent 0

Cần một quyết định trước khi Sóng 14 tiếp tục với mục tiêu này:

- **(a) Chấp nhận DỪNG** (khuyến nghị): giữ `vm_frame.rs` 958 dòng, đánh dấu ST-325b là
  `BLOCKED/churn`, chuyển headroom sang tệp ít churn hơn (`vm/exec.rs` 730, `vm.rs` 703).
- **(b) Nới ngưỡng churn** cho phép tách dù 23/90 ngày — cần A0 ghi rõ lý do vào `DECISIONS.md`,
  và nên đợi các nhánh W1/W5 đóng trước để tránh conflict.
- **(c) Chờ ổn định**: dời ST-325b sang Sóng sau, đo lại churn khi các ST của Sóng 14 merge xong.

Không có lựa chọn nào được thực hiện bởi Builder ở đây — chỉ báo cáo và checkpoint.
