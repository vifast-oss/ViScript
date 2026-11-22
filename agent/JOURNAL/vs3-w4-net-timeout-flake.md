<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — sửa flake test-only `net.rs::tcp_het_gio_doc` (Windows, soak 4h)

> **Người làm**: subagent VS3-W4, 2026-11-22.
> **Bối cảnh**: soak 4h tag `v0.3.4` dừng ở vòng 1150 vì `tcp_het_gio_doc` FAILED tại
> `crates/vis-cli/tests/net.rs` — assertion wall-clock `dt ≤ 400 ms`.
> **Phân loại**: **test-only / biên quá chặt**, không phải sản phẩm treo và không phải cancel race.
> Sửa tối thiểu: nới biên lên **3 s** kèm lý do đo được; không đổi code sản phẩm.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Chẩn đoán | 400 ms là wall-clock **cả process** (spawn + DLL/antivirus + connect + timeout 80 ms), không phải riêng `net.read`; đo thật 87–130 ms lúc ấm, **1,41 s** lần đầu sau build |
| Sửa | Chỉ test: biên 400 ms → **3 000 ms**, ghi lý do trong doc-comment; giữ assert stdout `quá thời gian` + exit 0 |
| 50 lần dưới tải (`psvn`) | **50/50 PASS** (2 CPU burner chạy nền) |
| 3 vòng default/tls/net | **22/22 + 22/22 + 9/9** mỗi vòng |
| `./vis-dev check` (Linux) | **PASS** (fmt, clippy, R1–R6, self-test, build, doc-examples) |
| Log | `psvn` `w4q-measure2.log`, `w4q-final.log` |
| Blocker | **Không có** |

## 1. Đọc test và chẩn đoán

`tcp_het_gio_doc` (sync net):
- Server loopback nhận kết nối, **ngủ 300 ms** rồi ghi `muon`.
- Client chạy `net.read(t, 64, 80)` ⇒ phải trả `Err` "quá thời gian" sau ~80 ms.
- Test đo `Instant::now()` quanh **cả tiến trình** `vis run` (`chay`), rồi assert
  `dt.as_millis() <= 400`.

Vì sao 400 ms quá chặt:
- 400 ms bao gồm **spawn process Windows** (nạp DLL, antivirus) + connect + 80 ms timeout +
  teardown; không phải thời gian riêng của `net.read`.
- Đo trên `psvn` (instrument tạm `eprintln!`, sau đó gỡ):
  - **100 lượt engine** (50 lần chạy × interp + VM) khi ấm: **87–130 ms**;
  - **lần chạy đầu sau build (nguội)**: **1,41 s** — vượt 400 ms dù hoàn toàn bình thường.
- Flake soak 4h xảy ra ở vòng 1150 dưới tải dài: chỉ cần máy chậm thêm vài trăm ms là vượt biên.
- Treo thật (lỗi sản phẩm) sẽ không trả về sau 80 ms mà kéo dài hàng giây; biên 3 s vẫn bắt được.

Kết luận: **đo timeout thật vẫn đúng** (assert stdout/exit), chỉ **biên wall-clock quá chặt**.

## 2. Thay đổi (test-only)

`crates/vis-cli/tests/net.rs`:
- Doc-comment ca 3: nêu rõ biên cũ 400 ms quá chặt, số đo 87–130 ms ấm / 1,41 s nguội,
  và 3 s = ~2,1× cold / ~23× warm.
- Assertion: `dt.as_millis() <= 3_000` (thay 400), thông điệp lỗi nêu "wall-clock cả process".
- **Không** đổi thân test (server 300 ms, `read(...,80)`, assert `quá thời gian`, exit 0),
  không đổi `vis`/host/sản phẩm.

## 3. Kiểm chứng

### 3.1. `psvn` (Windows MSVC) — 50 lần dưới tải

- 2 tiến trình CPU burner PowerShell chạy nền trong suốt 50 lần.
- `cargo test -p vis-cli --test net tcp_het_gio_doc`: **50/50 rc 0**, 0 fail.
- Log: `/c/Users/ps/vislogs/w4q-final.log`
  (`97eaec510c0b37d47815b279a21fc3821beb0d785b7ca1a75aba5b42c9a88f37`).
- Log đo biên (instrument tạm, 100 mẫu): `/c/Users/ps/vislogs/w4q-measure2.log`
  (`2ca8d6a5212832e40a8e214c2ac5bcadc5e0a5da1fca607e78539273ff60aa0d`).

### 3.2. 3 vòng targeted

| Vòng | net | vs3_default | vs3_tls | Thời gian |
|---|---:|---:|---:|---:|
| R1 | 9/9 | 22/22 | 22/22 | 16 s (build) |
| R2 | 9/9 | 22/22 | 22/22 | 12 s |
| R3 | 9/9 | 22/22 | 22/22 | 12 s |

- **0 fail, 0 flake**; `10053` không tái hiện.

### 3.3. Linux

- `cargo test -p vis-cli --test net tcp_het_gio_doc`: **1/1** pass.
- `./vis-dev check`: **PASS** (fmt OK, clippy OK, R1–R6 OK, self-test, build, doc-examples).
- `./vis-dev check` log: `/tmp/opencode/w4q-check.log`.

## 4. Đánh giá flake 4h còn lại

- Flake đã sửa là **bound test**, không phải sản phẩm; tag `v0.3.4` vẫn có race cancel-before
  (đã sửa ở `1edec5d`) nên `v0.3.5` vẫn cần.
- Khi soak 4h lại trên `v0.3.5`: theo dõi thêm `tcp_het_gio_doc` và free memory; nếu còn fail
  với dt > 3 s thì mới coi là treo thật.
- Không thấy rò process trong 50 lần (`vis` exit sạch); free memory không đo lại ở task này
  (chỉ chạy 50 lần ngắn + 3 vòng).

## 5. Kết luận

- **Sửa test-only tối thiểu**, có số đo căn cứ: 400 ms → 3 000 ms; hành vi timeout vẫn được kiểm.
- 50/50 dưới tải + 3 vòng targeted + `vis-dev check` xanh.
- **Không blocker.**
