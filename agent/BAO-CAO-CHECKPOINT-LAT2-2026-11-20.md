<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Checkpoint sau Lát 2 — chờ A0 duyệt dependency & Lát 3 (VD-252/253, 2026-11-20)

> **Trạng thái**: Lát 2 đạt trong phạm vi **reactor giả**, **DỪNG TẠI ĐÂY** — VD-252 §7: không tự
> mở Lát 3. Tag/release **PAUSED**; baseline lịch sử `dbae70a`; chưa mở worker/GC/TLS/API mới.
> Runtime mặc định, bytecode trên đĩa, `VERSION`, cache key và API ngôn ngữ **không đổi**.
>
> **Căn cứ**: VD-252, VD-253 (`agent/DECISIONS.md`) · ST-268/269/270 (`agent/QUEUE.md`) ·
> `agent/JOURNAL/vd252-{lat2-report,coverage-matrix,charging-final,selector-proposal}.md`.

## 1. Lát 2 — reactor giả trên đường frame thật (ST-268)

**Sản phẩm**: `crates/vis-interp/src/rt.rs` (ReactorOp/Event/Reg, `Wait::Reactor`, helper
`frame_reactor_poll/fire/idle/shutdown`, `diag_snapshot` + `regs`/`timers`), `stdlib.rs` (chỉ khi
`frame.enabled`, `net.read`/`net.accept` consult reactor ⇒ `Pending`), `frame.rs` + `vm_frame.rs`
(`Work::Redispatch` resume builtin không eval lại đối số), test `crates/vis-cli/tests/lat2_reactor.rs`.

**13/13 test xanh**, byte-identical stdout/chẩn đoán/log/snapshot giữa **hai engine cùng trace**.
Bất biến reactor đã có test: wake trùng, wake cũ/gen cũ, readiness giả → WouldBlock đăng ký lại,
đóng/timeout/đăng ký lại vô hiệu event cũ, EOF/lỗi/timeout gỡ waiter **đúng một lần**, nhiều task
cùng chờ; timer monotonic chọn deadline gần nhất; **không** `sleep(5 ms)` busy-poll trên đường thử.

**B1 (mục tiêu chồng lấp)** — 20 kết nối trễ 200 ms, 3 lần chạy:
- startup ~0,585 ms; **workload 201,2–201,5 ms** trên đường frame thử;
- đường mặc định vẫn **4,03–4,04 s** — không tuyên bố CLI mặc định có async I/O.

**B2** vẫn đói CPU: `CPU_DONE == IO_START`, `IO_DONE−IO_START` 201/202 ms — ghi rõ **chưa sửa**,
không đổi tiêu chí để gọi là đạt fairness. **Step-cap parity vẫn mở**, không tăng ngưỡng tạm.

**Explicit-rejection (VD-252 §1)**: `&&`/`||`/`??`/`|>` bị từ chối **trước khi đánh giá operand**;
hằng cấp tệp có chẩn đoán đúng; closure capture bị chặn rõ — mỗi chỗ có test âm, golden mặc định
không đổi. Ma trận đầy đủ: `agent/JOURNAL/vd252-coverage-matrix.md`.

## 2. Giới hạn trung thực

- Đây là **reactor giả deterministic**, **không phải mạng thật async**. `connect`/`write` vẫn đồng bộ
  và **có thể chặn scheduler**; chưa gọi là hoàn thành async mạng thực dụng.
- **Readiness thật không làm được với std thuần** mà không dùng selector dependency, `unsafe` trong
  mã dự án (bị R2 cấm), hoặc sleep-poll (VD-252 cấm). Vì A0 chưa duyệt crate, phần real reactor
  **chưa triển khai** — cần A0 chọn dependency trước.
- Đường frame test vẫn phủ tập HIR hạn chế; checker **không thu hẹp**; cấu trúc unsupported bị từ
  chối rõ (không fallback đồng bộ). Snapshot về 0 chứng minh cleanup **trong workload đã kiểm**.

## 3. Đề xuất dependency — chờ A0 duyệt (ST-269)

Số liệu đã **kiểm chứng độc lập** qua crates.io API + registry cache:

| | Ưu tiên | Thay thế |
|---|---|---|
| Crate | **`mio = "=1.2.5"`** | `polling = "=3.11.0"` |
| License | MIT | Apache-2.0 OR MIT |
| MSRV crate | 1.71 (≤ workspace 1.100.0) | 1.71 |
| Config | `default-features = false, features = ["os-poll","net"]` → **log OFF** | không features; tracing optional OFF |
| Transitive trên Linux | chỉ `mio`; `libc` **đã có** trong Cargo.lock | kéo `rustix`/`linux-raw-sys`/`cfg-if` — **chưa có** trong lock |
| wasm32 | `compile_error!` — không hỗ trợ (đặt dưới `[target.'cfg(target_os="linux")'.dependencies]`, resolver 2 ⇒ core/wasm không đổi) | tương tự |
| Unsafe | nằm **trong crate** (164 chỗ); project unsafe = 0 | 110 chỗ + rustix ⇒ diện tích lớn hơn |

**Không merge crate nào** trước A0 duyệt phương án cụ thể. Không áp allowlist ViFastOs; không thêm
`unsafe` trong mã dự án. Windows giữ runtime mặc định; không có cfg Linux-only hiện tại (std thuần),
nên **không có hồi quy Windows**.

## 4. Toolchain & MSRV (VD-252 + VD-253)

- `rust-toolchain.toml`: `channel = "1.100.0"` (thay `stable` chuyển động).
- **MSRV nâng `1.99.1 → 1.100.0`** theo VD-253 (chủ trì override VD-252 §6): `Cargo.toml`,
  `vis-host-wasm/Cargo.toml`, `vis-host-linux/Cargo.toml`.
- **Không sửa** báo cáo nghiệm thu lịch sử ghi `1.99.1`. Không biến `cargo clean` thành bước bắt buộc.
- `./vis-dev gate --fast`: **1463/1463 PASS** dưới 1.100.0 (sau clean build; lỗi `NotFound` trước đó
  là artifact stale do drift stable channel, đã khắc phục bằng clean — không kết luận quá mức về
  chuỗi nguyên nhân).

## 5. Đề xuất Lát 3 để A0 duyệt (CHƯA triển khai)

Sau khi A0 **duyệt crate cụ thể** (đề xuất `mio = "=1.2.5"`), Lát 3 sẽ nối **readiness thật**:
`read`/`accept` đăng ký với selector, `Pending` khi `WouldBlock`, scheduler chờ registry/timer, bỏ
poll; đo lại B1 trên mạng thật (20×200 ms chồng lấp, ~0,3 s là mục tiêu đo, không ngưỡng CI), kiểm
EOF/lỗi/timeout/cleanup và wake cũ/FD tái sử dụng. `connect`/`write` vẫn công bố là có thể chặn.
Không đổi VERSION/cache/API; Windows tiếp tục giữ runtime mặc định.

## 6. Quyết định A0 cần

1. Chấp nhận checkpoint Lát 2 (reactor giả, B1 chồng lấp, explicit rejection, B2/step-cap vẫn mở).
2. **Duyệt hoặc từ chối `mio = "=1.2.5"`** (hoặc `polling`) — điều kiện để mở real reactor Lát 3.
3. Duyệt/không Lát 3. Nhắc lại: tag PAUSED; VS3/worker/GC/TLS/API chưa mở.

**Kết luận**: Lát 2 đã chứng minh **task đang chờ mạng thật sự trả quyền cho task khác** (201 ms so
với 4,03 s tuần tự) trong đường frame thử — đúng mục tiêu VD-252, không phải đổi `sleep` bằng vòng
poll khác. Real readiness vẫn chờ A0 duyệt dependency; dừng ở đây cho tới khi có quyết định.
