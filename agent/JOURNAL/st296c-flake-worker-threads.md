<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296c — phân loại flake `vs3_worker::spawn_shared_stress_rounds_read_only`

**Kết luận: T-INFRA (test race), KHÔNG phải hồi quy ST-291/292.** Không sửa production.
**Nhánh**: `plan/w1-flake-worker-threads` từ `main` `a1730c5`. Không push/merge/tag.

## 1. Hiện tượng

Gate ST-296c (nextest, tải cao) đỏ **một lần**:
`rò worker thread: 2 -> 4` tại `crates/vis-cli/tests/vs3_worker.rs` (assert ngay sau vòng stress).
Chạy riêng xanh. Baseline `threads0=2`; test tạo `WorkerPool` W=1/4 nhiều lần, mỗi lần `join()`.

## 2. Tái hiện dưới tải

| Đợt | Cấu hình | Tải | Số lần | Đỏ | `threads1` |
|---|---|---|---:|---:|---|
| A | `VS3_W5_SHARED_ROUNDS=10`, 30×`yes` | 7 → 43 | 20 | **0** | — |
| B | `VS3_W5_SHARED_ROUNDS=20`, 30×`yes` | 33 → 48 | 40 | **0** | **2 = threads0** (mọi lần đo) |
| C | sau patch, load 4,2 | 4,2 | 1 | 0 | 2 → 2 (poll) |

Tổng **60 vòng tải, 0 lần đỏ**; không thấy thread thứ 3/4 tồn tại bền. Nguồn thread duy nhất
trong test là worker của `WorkerPool`; `join()` đã `h.join()` từng worker và `Drop` cũng join.

## 3. Nguyên nhân (T-INFRA)

- Assert đọc `/proc/self/status` **ngay** sau `pool.join()`; dưới tải cao, mục task của worker
  vừa join có thể còn hiện trong một lần đọc (kernel/scheduler thu hồi), tạo 2→4 nhất thời.
- Test ST-292 tương tự đã có **poll 500 ms**; test này thiếu. Không có bằng chứng rò thật:
  60 vòng đều trả `threads1=2`; RSS/Arc/outcome đều đúng.

## 4. Sửa test (có căn cứ, không che rò)

- Đọc `threads1`, nếu > `threads0 + 1` thì poll mỗi 10 ms tới **tối đa 2 s**, ghi `max`.
- Chỉ assert sau khi hết hạn ⇒ rò thật (thread không thoát) **vẫn đỏ**; thêm dòng
  `SHARED_THREADS_SAU_JOIN` để chẩn đoán.
- Không `#[ignore]`, không nới ngưỡng thread, không bỏ ca. Không đổi production/API.

## 5. Bằng chứng sau sửa

`cargo test -p vis-cli --test vs3_worker spawn_shared_stress_rounds_read_only -- --exact --nocapture`
(load 4,2) ⇒ `SHARED_THREADS_SAU_JOIN threads0=2 threads1=2 max=2`, **1 passed**.

**Chưa chạy gate/soak/diff** theo chỉ đạo W3 đang soak trên main 0.4.1; sẽ chạy khi được báo.
