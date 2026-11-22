<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Bàn giao sau Lát 3 — chờ A0 quyết bước tiếp (VD-254, 2026-11-20)

> **Trạng thái**: Lát 3 đạt trong phạm vi **đường frame test-only**. **DỪNG TẠI ĐÂY** — VD-254:
> tag/release **PAUSED**; chưa mở worker/GC/TLS; chưa đổi runtime mặc định/VERSION/cache/API.
> Baseline lịch sử `dbae70a`; toolchain/MSRV `1.100.0` (VD-252/253, không sửa hồ sơ cũ).
>
> **Căn cứ**: `agent/DECISIONS.md` VD-252/253/254 · `agent/JOURNAL/vd254-{lat3-report,third-party-notice}.md`
> · `agent/BAO-CAO-CHECKPOINT-LAT2-2026-11-20.md` (đã đính chính nhãn B1).

## 1. Commit tích hợp & diff dependency

- **Commit mã (ST-271)**: `301e3ab56b1ebda300bd1613ba207e7a116d4056` — adapter `mio` trên đường frame,
  test `crates/vis-cli/tests/lat3_real.rs`. **Commit báo cáo này**: cùng hash `301e3ab…` (báo cáo được
  commit chung với mã). **Trạng thái cây**: sạch. **Commit cây tích hợp hiện tại**:
  `916ba7b355c211a7d5bd5b39fa692f87474ecf56` — bổ sung nhóm README/tooling/tree-sitter/ví dụ theo
  chỉ thị trực tiếp của chủ trì; gate cây chung **1475/1475 PASS** (R3 đã khép với sidecar `.license`).
- **Manifest**: `crates/vis-cli/Cargo.toml:60` — `[target.'cfg(target_os = "linux")'.dev-dependencies]`
  `mio = { version = "=1.2.5", default-features = false, features = ["os-poll", "net"] }`.
  **dev-dependency** ⇒ không lọt vào artifact `vis` phát hành; chỉ Linux, không vào core/wasm.
- **Cargo.lock** (đã review): đúng `mio 1.2.5`, `source = registry+...crates.io-index`, checksum
  `20521a903f02b0a573cd6840e9d0a5a0cc043d361fe924092cd70f8556cc9c4a`; chỉ `vis-cli` thêm `mio`;
  **không** `polling`/`rustix`/`log`; các entry `wasi`, `windows-sys`, `windows-link` là cfg-gated
  của mio (có trong lockfile resolved cho mọi target nhưng **không biên dịch trên Linux**);
  **không** bump dependency không liên quan. Không dùng `mio::unix::SourceFd` (cần `os-ext` chưa duyệt)
  — dùng `mio::net::{TcpStream,TcpListener}::from_std`.
- **Giấy phép/thông báo bên thứ ba**: `agent/JOURNAL/vd254-third-party-notice.md` — MIT, checklist
  lockfile 11 ô; `unsafe` nằm trong dependency, project unsafe = 0 (`CONTRIBUTING.md:33`, R2).
  **Không** có `THIRD-PARTY.md`/`deny.toml` hiện hành — thông tin ghi tại chỗ khai báo kèm số VD.

## 2. Test mạng thật / reactor giả / cleanup

- **Mạng thật** (`lat3_real.rs`, **12/12**): `read` dữ liệu/EOF/lỗi/timeout; `accept` chưa có client
  rồi có client; nhiều kết nối chờ đồng thời; đóng khi đang chờ; token **generation** chống FD tái
  sử dụng; stale event; **không đánh giá lại đối số/không lặp side-effect** khi builtin resume; cleanup
  về baseline sau khi thả handle; lỗi selector truyền thành trap rõ; không hot-loop `WouldBlock`.
- **Reactor giả** (Lát 2, **13/13**) vẫn chạy parity theo trace giữa interp/VM — không thay thế bằng
  mạng thật. Tổng `cargo test -p vis-interp -p vis-cli`: **1032 passed / 0 failed**
  (LAT1 8/8 + 1/1, LAT2 13/13, LAT3 12/12).

## 3. B1-real-frame so với baseline

| Nhãn | Số liệu | Phạm vi |
|---|---|---|
| **B1-sim** | ~201 ms | reactor giả (Lát 2, kế thừa, **không đo lại**) |
| **B1-real-default** | 4,03–4,04 s | runtime mặc định, socket thật (kế thừa) |
| **B1-real-frame** | N=1 ~201 ms · N=5 ~202 ms · N=20 ~202–203 ms (startup 0,8–10,7 ms), 3 lần chạy | **mio readiness thật**, đường frame test, cả 2 engine |

Chồng lấp rõ: 20 kết nối × 200 ms không tăng gần 4 s. ~0,3 s là **mục tiêu quan sát, không ngưỡng CI**.
**Không** ép thứ tự output giữa kết nối độc lập; **không** dùng tỷ lệ B1-sim/real-default làm tuyên bố
tăng tốc mạng sản phẩm.

## 4. Windows / core / wasm — có hồi quy hay không

Theo báo cáo tích hợp: `cargo check` target **Windows-msvc** và **wasm32 core PASS**; `mio` **vắng mặt**
trong cả hai graph (dev-dependency dưới cfg Linux). Cấu hình default/sqlite trên hai target vẫn
**EXPECTED_RED** do `libsqlite3-sys` thiếu C toolchain (`lib.exe`/`clang`) — **lỗi có sẵn**, không liên
quan mio. **Lưu ý**: cfg là thiết kế phòng ngừa, **không phải bằng chứng không hồi quy**; các lệnh
`check --target` trên là bằng chứng được báo cáo, A0 chưa tự chạy lại.

## 5. Coverage & blocker còn lại

- **Đã đạt**: read/accept không chặn trên đường frame test; scheduler trả quyền khi `Pending`; token
  generation/WouldBlock/close/timeout/FD-reuse/nhiều waiter/selector-error đúng hợp đồng.
- **Còn mở (không sửa ở Lát 3)**: `connect`/`write` vẫn đồng bộ, **có thể chặn** ⇒ chưa gọi là async
  mạng hoàn thiện; **B2** vẫn đói CPU; **step-cap parity** (interp đếm vòng vs VM đếm lệnh, 10⁷) vẫn mở,
  không tăng ngưỡng tạm; **coverage HIR** đường frame test còn hạn chế và **closure writeback** chưa
  kiểm chứng — phải khép trước khi đổi runtime mặc định (thành công reactor **không thay thế** nghĩa vụ
  này). unsupported vẫn **từ chối rõ**, không fallback đồng bộ.

## 6. Vấn đề ngoài phạm vi cần A0 biết

Trong lúc tích hợp có tiến trình song song tạo các tệp không thuộc VD-254: `README.md`, `bin/vis-dev`,
`infra/linguist/tree-sitter-viscript/**`, `test/readme-examples.py`, `vi-du/*.vis`. Một trong số đó
(`infra/linguist/tree-sitter-viscript/package.json`) **làm `./vis-dev gate --fast` đỏ ở R3** (thiếu
sidecar `.license`). Tôi **không sửa/không commit** các tệp đó. Validate mã của VD-254 bằng
`cargo test -p vis-interp -p vis-cli` → **1032/0**; không có lỗi R1..R6 nào từ file của VD-254.
`./vis-dev check` của riêng luồng tích hợp từng PASS trước khi các tệp ngoài xuất hiện.

## 7. Quyết định A0 tiếp theo

1. Chấp nhận Lát 3 trong phạm vi **read/accept không chặn trên đường test**.
2. Quyết cách xử lý các tệp ngoài phạm vi §6 (để tiến trình đó tự hoàn tất, hay gộp riêng).
3. Chỉ sau khi khép coverage HIR + closure/writeback mới trình đổi runtime mặc định — **chưa làm**.
4. Tag/release **PAUSED**; worker/GC/TLS/API mới **chưa mở**; không dùng ước lượng cũ làm cam kết.

**Kết luận**: Lát 3 đã chứng minh **readiness mạng thật** (`mio`/epoll) đưa task chờ về `Pending` và
scheduler chạy task khác, với B1-real-frame ~201 ms thay vì 4 s tuần tự — trên cả hai engine, chỉ đường
test. Đó là nền móng trước khi考虑 đổi runtime mặc định; dừng ở đây chờ A0.
