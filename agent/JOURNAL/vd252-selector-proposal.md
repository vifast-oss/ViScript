<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VD-252 / ST-269 · Đề xuất dependency selector cho reactor Lát 2

**Trạng thái**: **DRAFT — CHỜ A0 DUYỆT**. research + draft only; **không** sửa `Cargo.toml`/
`Cargo.lock`, không merge, không commit, không chạy gate, không chọn crate làm phương án final.
**Căn cứ**: VD-252 §4 (`agent/DECISIONS.md:2665-2672`), chính sách dependency
(`agent/JOURNAL/vd250-dependency-policy.md`), báo cáo Lát 1 §4 (`vd251-lat1-report.md:112-113`).
Nguồn số liệu crate: crates.io API, truy cập 2026-11-20; **phải đối chiếu lại bằng `cargo tree` /
`cargo metadata` sau khi A0 duyệt** (A0 chê phiếu trước vì thiếu tên/version/feature/transitive).

---

## 0. Luật A0 áp cho phiếu này

- **Không áp tự động allowlist ViFastOs**; QD nằm ở repo khác (`AGENTS.md:6-7`), mọi xung đột trình A0
  (`agent/DECISIONS.md:2670`; `vd250-dependency-policy.md:10-24`).
- **Không thêm `unsafe` trong mã dự án** để né (`CONTRIBUTING.md:33`; R2 `bin/vis-dev:242-253`);
  `unsafe` **nằm trong dependency** phải đánh giá riêng (`vd250-dependency-policy.md:92-99`).
- **Cần A0 duyệt phương án cụ thể trước khi merge** (`agent/DECISIONS.md:2667-2670`).
- **Windows giữ runtime mặc định**; đường thử Linux-only phải giới hạn rõ, không làm Windows
  build/test hỏng (`agent/DECISIONS.md:2671-2672`).
- **Đính chính (VD-253)**: MSRV đã nâng `rust-version = 1.100.0` theo chỉ thị "nâng hết lên 1.100.0";
  toolchain dev/CI cũng ghim `1.100.0`. VD-252 §6 ("không tự nâng MSRV") đã bị **override** bởi VD-253.
  Phiếu này không đụng các tệp đó; số liệu crate ở dưới vẫn dùng MSRV riêng của crate (mio 1.71 ≤ 1.100.0).
- Không đổi `VERSION`/cache/API ngôn ngữ; selector chỉ là adapter host (`vd250-dependency-policy.md:71`).

---

## 1. Làm được gì với ZERO dependency, và cái gì thực sự cần selector

| Việc | ZERO dependency? | Bằng chứng / lý do |
|---|---|---|
| Reactor **giả** theo trace: `fake_sources`, `events`, `frame_fire`, `frame_wake_one`, FIFO/wake-gen, duplicate/stale, ownership, final-join, suspend→resume đúng chỗ | **CÓ — đã có sẵn** | `FrameConfig` `frame.rs:27-33`; các hàm `frame_*` trong `rt.rs` (`vd251-lat1-report.md:52-57`); 8 tiêu chí Lát 1 chạy bằng trace giả (`vd251-lat1-report.md:31-45`) |
| Chứng minh parity hai engine cùng trace, cleanup đúng 1 lần, wake trùng không tạo lượt chạy trùng | **CÓ** | A0 nghiệm thu reactor giả `agent/DECISIONS.md:2676-2678`; không cần OS selector |
| **I/O mạng thật không chặn**: `read`/`accept` gặp `WouldBlock` ⇒ đăng ký readiness, trả `Pending`, bỏ vòng `sleep(5 ms)` | **KHÔNG** | Cần nguồn readiness từ kernel (epoll/kqueue/IOCP). `std::net` là blocking ⇒ chặn scheduler; gọi trực tiếp syscall `epoll_*` qua `libc` đòi hỏi `unsafe` trong **mã dự án** ⇒ bị R2 cấm (`bin/vis-dev:242-253`) |
| Timer monotonic thật | **Một phần** | Có thể dùng `Instant` (std, monotonic, zero-dep) cho thời gian; nhưng để **chờ cùng registry** với readiness thì cần selector có timer/peek — A0 cho phép timer tối thiểu, không bắt cùng registry (`DECISIONS.md:2653-2656`) |

**Kết luận**: adapter + test reactor giả làm được ngay, **không dependency mới** (A0 cho phép rõ,
`agent/DECISIONS.md:2667-2668`). Selector chỉ thực sự cần khi chuyển sang **readiness thật** trên
mạng thật để đạt mục tiêu B1 chồng lấp (~0,3 s — mục tiêu đo, không ngưỡng CI cứng, `DECISIONS.md:2679-2681`).

---

## 2. Phương án ưu tiên — `mio` 1.2.5  **[CHỜ A0 DUYỆT]**

| Trường | Giá trị |
|---|---|
| Tên + version ghim | `mio = "=1.2.5"` (ghim tuyệt đối theo tiền lệ `rusqlite = "=0.37.0"` `crates/vis-cli/Cargo.toml:42-47`; bản mới nhất 2026-11-13, checksum `20521a90…`) |
| License | **MIT** (crates.io `versions[1.2.5].license`) — nằm trong allowlist tự chứa `docs/ViScript-thiet-ke.md:926` (phải A0 xác nhận vẫn hiệu lực) |
| Platform | Linux (epoll), macOS/iOS/BSD (kqueue), Windows (IOCP/wepoll qua `windows-sys`), wasi. **Không** hỗ trợ `wasm32-unknown-unknown` ⇒ chỉ khai ở host adapter, không vào core crate |
| MSRV | `rust-version = 1.71` (`crates.io`) ≤ 1.99.1 ✓ |
| Features | `default = ["log"]`, `net`, `os-ext`, `os-poll`. **Đề xuất `default-features = false, features = ["os-poll", "net"]`** ⇒ bỏ optional `log`, giữ API `Poll`/`Registry`/`Token`/`Events` |
| Vai trò | Chỉ adapter `net.read`/`net.accept` trên đường frame test, khai trong `[target.'cfg(target_os = "linux")'.dependencies]` của crate host; core crate giữ nguyên wasm32 |

**Cây transitive** (từ registry metadata; xác nhận bằng `cargo tree -p mio -e normal,build,dev` sau duyệt):

```
mio 1.2.5
├─ libc ^0.2.190            [cfg(any(unix, hermit, wasi))]  normal  ← ĐÃ có trong Cargo.lock
├─ wasi ^0.11.0             [cfg(target_os = "wasi")]       normal  ← không kéo khi build Linux/Windows
├─ log ^0.4.8               optional                        normal  ← TẮT (default-features = false)
├─ windows-sys ^0.61        [cfg(windows)]                  normal  ← Windows bị cfg-linux loại khỏi graph
└─ (dev) env_logger ^0.11, rand ^0.9                                ← không vào artifact release
```

**Unsafe trong dependency**: có — `mio` dùng FFI/syscall trong `src/sys/*` (epoll/kqueue/IOCP, qua
`libc`/`windows-sys`). Đây là **unsafe của crate**, không phải unsafe của dự án (project unsafe vẫn = 0);
API public ta gọi (`Poll::wait`, `Registry::register`) là safe. Cần đánh giá riêng theo
`vd250-dependency-policy.md:97`: có lọt vào API runtime không (không), có nằm core không (không),
có audit/Miri phía crate không (phải ghi trong exception report). **Không** tự coi là sạch.

**Lý do ưu tiên**: transitive nhẹ nhất trong các ứng viên (trên Linux chỉ thêm `mio` + `libc` vốn đã
có); API readiness rõ, khớp "adapter I/O mỏng" (VD-250 §1 `agent/DECISIONS.md:2443`); **không** kéo
executor/futures/worker ⇒ không vi phạm "một luồng do ViScript quản lý".

---

## 3. Phương án thay thế ngắn — `polling` 3.11.0  **[CHỜ A0 DUYỆT]**

| Trường | Giá trị |
|---|---|
| Tên + version ghim | `polling = "=3.11.0"` (bản mới nhất 2025-09-14, checksum `5d0e4f59…`) |
| License | **Apache-2.0 OR MIT** — hợp lệ theo allowlist tự chứa `docs/ViScript-thiet-ke.md:926` |
| Platform | Linux (epoll qua `rustix`), macOS/BSD (kqueue), Windows (IOCP), Fuchsia, Hermit, VxWorks. **Không** `wasm32-unknown-unknown` |
| MSRV | `rust-version = 1.71` ✓ |
| Features | **Không có feature nào được khai báo** ở 3.11.0 (`features: {}`); `tracing` là dependency optional. Ít phải chọn feature |
| Vai trò | Cùng phạm vi host adapter/Linux-only như mio |

**Cây transitive** (registry metadata; phải `cargo tree` đối chiếu):

```
polling 3.11.0
├─ cfg-if ^1                                      normal
├─ rustix ^1.0.5   [cfg(any(unix, fuchsia, vxworks))]  normal  features: event, fs, pipe, process, std, time
│   └─ (dự kiến) bitflags / errno / linux-raw-sys     ← CHƯA có trong Cargo.lock hiện tại; phải kiểm chéo
├─ hermit-abi ^0.5 [cfg(target_os = "hermit")]     normal
├─ windows-sys ^0.61, concurrent-queue ^2.2.0, pin-project-lite ^0.2.9  [cfg(windows)]
├─ tracing ^0.1.37 optional                        normal
└─ (dev) easy-parallel, fastrand, libc, signal-hook, socket2
```

**Unsafe trong dependency**: có, và **nhiều bề mặt hơn mio** — syscall trong `polling` và trong
`rustix` (FFI syscall, có thể kéo `linux-raw-sys` với binding/asm theo kiến trúc). Cần exception
report riêng; project unsafe vẫn giữ 0. Không tự coi là tương đương mio về unsafe.

**Vì sao chỉ là thay thế**: interface gọn, không cần feature, nhưng kéo `rustix` + transitive mới vào
graph hiện tại ⇒ chi phí review license/unsafe cao hơn; lợi ích không bù được sự đơn giản của mio
cho đúng nhu cầu `read`/`accept` readiness.

---

## 4. Loại có lý do (ngắn)

| Ứng viên | Lý do loại |
|---|---|
| `tokio` (net/time) | Kéo runtime/executor, thứ tự wake do dependency quyết, mâu thuẫn "một luồng do ViScript quản lý" (`DE-XUAT-KIEN-TRUC-ASYNC-2026-11-20.md:43`) |
| `async-io` / `smol` | Kéo `futures`/executor — VD-250 chưa mở worker/API mới (`agent/DECISIONS.md:2452`) |
| `crossbeam` / `futures` | Shared heap/parallel; không cần cho readiness đơn luồng |

---

## 5. Điều kiện bắt buộc trước khi merge  **[CHỜ A0 DUYỆT]**

1. A0 duyệt **tên + version ghim + feature + target gating** của một phương án cụ thể (§2 hoặc §3).
2. Chạy `cargo tree -p <crate> -e normal,build,dev --prefix none`, `cargo metadata --format-version 1`,
   đối chiếu license từng crate transitive + unsafe inventory — artifact đính kèm exception report
   (`vd250-dependency-policy.md:56-77`, §2.3 mẫu báo cáo).
3. Ghi khai báo tại chỗ kèm comment `VD-252` + license như tiền lệ `sha2` (`crates/vis-bytecode/Cargo.toml:2-4`).
4. Giữ cfg Linux-only: Windows build/test không hồi quy, runtime mặc định Windows không đổi; không lọt
   selector vào core crate (wasm32).
5. Không đổi `VERSION`/cache/API ngôn ngữ; không thêm unsafe project; không đụng `rust-toolchain.toml`.

**Không chọn crate final ở phiếu này.** Cả hai phương án đều ở trạng thái **chờ A0 duyệt**.
