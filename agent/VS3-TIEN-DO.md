<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 — bảng tiến độ (A0, mở 2026-11-21)

> **Nguồn kế hoạch**: `agent/KE-HOACH-VS3-2026-11-21.md`.
> **Trạng thái hợp lệ**: `TODO | WIP | REVIEW | DONE | BLOCKED`.
> **Luật**: mỗi lát phải có test **trước** khi merge; full gate tại mốc 4 giờ; không dồn test cuối tuần.
> macOS **ngoài phạm vi**; Linux là nền chính; Windows Tier 2 tới VS3-6.

## 1. Mốc hiện tại

| Mục | Giá trị |
|---|---|
| Baseline VS3 | `fc01b9a` (đợt A0 dừng 21/11) |
| Gate baseline | **3298 passed / 3 skipped** |
| Nightly baseline | PASS (fuzz 17.198.259 ca / 0 vi phạm) |
| `FUNCS` | **167** |
| Tag | `pha1-rc1` (không di chuyển); `v0.1.0-rc1` dự kiến cuối tuần |
| Nền đo | Linux `ps`; macOS bỏ; Windows Tier 2 |

## 2. Bảng lát cắt

| Lát | Chủ | Test chính | Trạng thái | Ghi chú |
|---|---|---|---|---|
| VS3-1a — budget/step-cap | W1 | `vs3_budget.rs` + diff | TODO | critical path |
| VS3-1b — cancellation/cleanup | W1 | `vs3_cancel.rs` | TODO | sau 1a |
| VS3-1c — coverage HIR | W1 | `vs3_coverage.rs` | TODO | ma trận đầy đủ |
| VS3-1d — flip default `async` | W1 | `vs3_default.rs` + gate | TODO | chỉ sau 1a–1c |
| VS3-2a — wait-set/`chan(0)` | W1 | `vs3_chan0.rs` | TODO | song song 1b |
| VS3-2b — `select`/timeout | W1 | `vs3_select.rs` | TODO | sau 2a |
| VS3-3a — spike worker/heap | W1 | `vs3_spike.md` + bench | TODO | chỉ đo, không production |
| VS3-3b — worker pool | W1 | `vs3_worker.rs` | TODO | sau spike + quyết định |
| VS3-4a — TLS client | W2 | `vs3_tls.rs` | TODO | rustls review trước |
| VS3-4b — HTTP/2/keep-alive | W2 | `vs3_http2.rs` | TODO | sau 4a |
| VS3-5a — registry HTTP + chữ ký | W2 | `vs3_registry.rs` | TODO | độc lập |
| VS3-5b — GC store/lock | W2 | `vs3_pkg_gc.rs` | TODO | sau 5a |
| VS3-6 — Windows Tier 1 + wasm | W3 | `test/host`, CI | TODO | sau P0/P1 |
| VS3-7 — LSP tệp phụ/incremental | W3 | `lsp_da_tep` mở rộng | TODO | P2 |
| VS3-8 — Plugin/ViTerm | W3 | `vs3_plugin.rs` | TODO | P3 |

## 3. Mốc tích hợp cuối tuần

| Mốc | Giờ (JST) | Việc | Kết quả |
|---|---|---|---|
| M1 | T7 19:00 | merge wave 1 + `gate --fast` | gate xanh |
| M2 | CN 02:00 | merge wave 2 + gate + nightly nhẹ | gate xanh |
| M3 | CN 14:00 | merge wave 3 + gate + diff 100k | 0 lệch |
| M4 | CN 21:00 | hardening + gate + nightly đầy đủ | PASS |
| M5 | T2 06:00 | fresh-clone + Windows (nếu runner) + báo cáo | tag rc1 nếu xanh |

## 4. Quy tắc cập nhật

- Mỗi lần merge: cập nhật dòng lát tương ứng + commit hash + test đã chạy.
- `BLOCKED` phải ghi lý do + người/cái gì gỡ; không để trạng thái mơ hồ.
- Không đánh dấu `DONE` khi chưa có test trong repo + gate tương ứng.
- macOS không xuất hiện trong bảng nghiệm thu; chỉ ghi chú “ngoài phạm vi”.
