<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Phiếu đề xuất tag 0.x — ViScript, 2026-11-20

> **TRẠNG THÁI: TẠM DỪNG (PAUSED) theo VD-247 bị A0 thu hồi bởi VD-248 — 2026-11-20.**
> Quyền tạo/push tag và công bố release đã bị **tạm dừng** cho tới khi có đánh giá mức sẵn sàng
> sử dụng thực tế (`agent/BAO-CAO-SAN-SANG-THUC-TE-2026-11-20.md`). Phiếu này chỉ là đề xuất lưu
> hồ sơ; **không** được thực hiện. Tag duy nhất vẫn là `pha1-rc1` (= `05e42d0…`).
>
> *(Lời gốc trước khi tạm dừng, giữ lại để truy nguyên:)* **ĐỀ XUẤT — CHƯA THỰC HIỆN.** Phát hành theo VD-247/A0 (chấp nhận nghiệm thu có
> phạm vi, nhưng **chưa cho phép tag/phát hành**). Phiếu này **không** tạo tag, **không** push tag,
> **không** công bố release, **không** mở pha 2/VS3. Chờ A0 phê duyệt tên và phạm vi.
>
> **Căn cứ**: `agent/DECISIONS.md` VD-247 · baseline đã khóa
> `dbae70a050415fe1c88a1545c7b011abb05d9aa2` · `agent/BAO-CAO-NGHIEM-THU-2026-11-20.md` ·
> `agent/BAO-CAO-RATIFICATION-2026-11-20.md` · `agent/TARGET-MATRIX-2026-11-20.md`.

## 1. Tên phiên bản đề xuất (chưa dùng)

| Mục | Giá trị |
|---|---|
| Tag đề xuất | **`v0.1.0`** |
| Cargo workspace version | `0.1.0` (`Cargo.toml` + các crate) — khớp tag, không cần bump |
| Kiểm tra chưa dùng | `git tag -l` hiện chỉ có **`pha1-rc1`** (= `05e42d0…`, **không được di chuyển**); chưa có tag `0.x` nào ⇒ `v0.1.0` chưa dùng |
| Phương án thay thế (nếu A0 muốn prerelease) | `v0.1.0-rc1` — cũng chưa dùng; chỉ đổi khi A0 chọn |

Không tự ý tag `pha1-rc1` lên commit mới; không tái dùng tên tag cũ.

## 2. Release notes (dự kiến)

- **Ổn định sau pha 1** (VD-242..VD-246): Option/optional chaining, map/JSON, `percent` tràn lỗi
  `VS0507`, SQLite MVP tùy chọn, phiên bản/cache bytecode `VERSION = 2`, hàng rào runtime cho
  global chưa khởi tạo và capture/local ngoài miền (`VS0501`, exit 1).
- **Độ đúng hai engine**: 28 findings reproduced; parity/std/CLI đã sửa; interp↔VM byte-identical
  trên 525 ca semantics TAP; 8+8 ca crafted-bytecode do gate `cargo test` chạy.
- **Tương thích**: runtime cũ/new từ chối artifact không tương thích; cache sau ST-252 **không**
  bump (không đổi output chương trình hợp lệ) — có căn cứ trong `JOURNAL/st252-review.md`.
- **Tooling**: `vis`/`vis-dev` chọn cargo đúng `rust-version` (#185); vòng tròn fmt gom spawn
  (ST-257); L0 test dùng `cargo-nextest` trên Linux, fallback `cargo test` khi không có (ST-258).
- **Target**: tách cấu hình core (`--no-default-features`, 18/18) và sqlite default
  (16 PASS + 2 EXPECTED_RED); `agent/TARGET-MATRIX-2026-11-20.md`.
- **Hồ sơ nghiệm thu**: baseline `dbae70a`, bảng nghiệm thu + bảng ratification truy nguyên từng
  mục; A0 ratify phạm vi đã mô tả (VD-247), không phê duyệt trắng.

## 3. Artifact dự kiến

| Artifact | Nguồn / cấu hình |
|---|---|
| Source archive của tag (GitHub tự sinh) | `dbae70a`, sau khi A0 phê duyệt và tạo tag |
| `vis` CLI — Linux `x86_64-unknown-linux-gnu` | build từ `dbae70a`, toolchain `rustc/cargo 1.99.1`, feature mặc định |
| `vis` CLI — Windows `x86_64-pc-windows-msvc` (**Tier 2**) | build từ `dbae70a`, toolchain `rustc/cargo 1.100.0`; **chỉ** theo đường `cargo test` đã nghiệm thu |
| Bộ tài liệu phát hành | `SPEC.md`, `docs/SPEC-lex.md`, `agent/TARGET-MATRIX-2026-11-20.md`, `agent/BAO-CAO-NGHIEM-THU-2026-11-20.md`, `agent/BAO-CAO-RATIFICATION-2026-11-20.md` |

**Không** dự kiến: binary macOS/wasm32 runtime, artifact từ các ô `EXPECTED_RED`, artifact build
từ commit khác `dbae70a`. Commit báo cáo/tài liệu sau phép đo là riêng, không thay baseline.

## 4. Giới hạn hỗ trợ (bắt buộc đi kèm release notes)

| Phạm vi | Mức |
|---|---|
| Linux | **Đã nghiệm thu** — gate, fallback, fresh-clone, semantics hai engine |
| Windows | **Tier 2**, **chỉ** qua `cargo test --workspace --no-fail-fast` (1422/0) |
| Windows + nextest | **Chưa đạt** (1416/1417, `tcp_explain_tong`, `WSAECONNABORTED`, 1/3) — nợ **#191**; không dùng làm gate; nguyên nhân chưa phân lập |
| macOS | **Chưa verify** — không tuyên bố hỗ trợ; chỉ chặn tuyên bố macOS |
| wasm32 / msvc (cấu hình sqlite) | **CHECK biên dịch chéo**, 2 ô `EXPECTED_RED`; **không** chứng nhận runtime, **không** đổi nhãn thành đã hỗ trợ (#156) |
| Pha 2 / VS3 / API mới | **Chưa mở** |
| Nợ mở | **96 mở / 70 đóng** (tổng 192); #190/#186 giữ mở, không chặn theo điều kiện đã duyệt; **không** yêu cầu đóng hết trước mốc này |

## 5. Điều kiện để chuyển thành tag thật

1. A0 phê duyệt **tên tag** (`v0.1.0` hoặc phương án thay thế) và phạm vi nêu trên.
2. A0 ra lệnh **tạo/push tag** — phiếu này tự thân **không** thực hiện bước đó.
3. Release notes công bố phải giữ nguyên các dòng "Chưa đạt / Chưa verify / CHECK only" ở mục 4;
   **không** diễn giải "mọi đường kiểm đều xanh".

> Agent 1 đã khóa hồ sơ theo VD-247 bằng thay đổi tài liệu, không đo lại, không đổi commit đã đo.
> Chờ phê duyệt; không tự tạo tag, push tag, công bố release, hoặc mở pha 2/VS3.
