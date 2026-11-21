<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 — bảng tiến độ (A0, mở 2026-11-21)

> **Nguồn kế hoạch**: `agent/KE-HOACH-VS3-2026-11-21.md`.
> **Trạng thái hợp lệ**: `TODO | WIP | REVIEW | DONE | BLOCKED`.
> **Luật**: mỗi lát phải có test **trước** khi merge; full gate tại mốc 4 giờ; không dồn test cuối tuần.
> macOS **ngoài phạm vi**; Linux là nền chính; Windows Tier 2 tới VS3-6.

## 1. Mốc hiện tại

| Mục | Giá trị |
|---|---|
| Baseline v0.1.0 | `50675dd` (tag `v0.1.0`) |
| Gate v0.1.0 | **3448 passed / 3 skipped** |
| Nightly baseline | PASS (fuzz 17.198.259 ca / 0 vi phạm) |
| `FUNCS` | **167** (mốc v0.1.0) |
| Tag | **`v0.2.0`** @ `42b700c`; `v0.1.0` @ `50675dd` lịch sử; `pha1-rc1` giữ nguyên; mục tiêu **v0.3.0** trước 09:00 T2 23/11 |
| Nền đo | Linux `ps`; macOS bỏ; Windows Tier 2 |

## 2. Bảng lát cắt

| Lát | Chủ | Test chính | Trạng thái | Ghi chú |
|---|---|---|---|---|
| VS3-1a — budget/step-cap | W1 | `vs3_budget.rs` + diff | **DONE (một phần)** — `3941619`; fuel 100M, parity 2M; BUDGET/Q/default còn mở |
| VS3-1b — cancellation/cleanup | W1 | `vs3_cancel.rs` | **DONE (một phần)** — `97676c6`; 14 ca; channel wait-set để VS3-2a |
| VS3-1c — coverage HIR | W1 | `vs3_coverage_matrix.rs` | **DONE (một phần)** — `7e1deed`; 10 ca, sửa `?.` frame; `Pipe`/nested async chặn 1d |
| VS3-1d — flip default `async` | W1 | `vs3_default.rs` + gate | **DONE (một phần)** — `1de3faa` + `098b649`; async cap≥1 wait-set; async net production còn mở |
| VS3-2a — wait-set/`chan(0)` | W1 | `vs3_chan0.rs` | **DONE (một phần)** — `d060f5a`; 13 ca; cap≥1/select còn mở |
| VS3-2b — timeout kênh (`select` hoãn 2c) | W5 | `vs3_chan_timeout.rs` | **DONE (một phần)** — `8da533b`; 17 ca `recv_timeout`/`send_timeout`; `select` VS3-2c |
| VS3-3a — spike worker/heap | W5 | `vs3_spike.md` + bench | **DONE** — `4bb20df`; chọn affine + per-worker heap (B1–B6) |
| VS3-3b — worker pool | W1 | `vs3_worker.rs` | **REVIEW** — prototype `3fcd7ef`; W1 tích hợp sau VS3-2a |
| VS3-4a — TLS client | W2 | `vs3_tls.rs` | **DONE** — `a5e3040`; 10 ca TLS, HTTP/1.1 giữ; HTTP/2/keep-alive còn mở |
| VS3-4b — keep-alive + IPv6 + registry HTTPS + cookie/auth | W2 | `vs3_keepalive.rs`, `vs3_registry_https.rs`, `vs3_cookie_auth.rs` | **DONE (một phần)** — `b7385e9` + `cc74dbc` + `096ea2c`; 8+6+9 ca |
| VS3-4b2 — HTTP/2 | W2 | `vs3_http2.rs` | **DONE** — `7d73268`; **default-on cho v0.3.0**; 7 ca h2; Windows h2 chờ verify |
| VS3-4b3 — h2 reuse | W2 | `vs3_http2.rs` | **DONE** — `28f9ea2`; reuse tuần tự theo authority, 7 ca; merge `64cb008` |
| VS3-5a — registry HTTP + chữ ký | W3 | `vs3_registry.rs` | **DONE** — `756bb1d`; 11 ca + SHA-256; chữ ký/GC còn mở |
| VS3-5b — chữ ký/GC/migrate | W3 | `vs3_pkg_gc.rs` | **DONE** — `cea1643`; Ed25519 + `pkg gc` + migrate; mã `VS0708` chờ A0 |
| VS3-5c — key rotation + GC nhiều lock | W5 | `vs3_pkg_gc_multi.rs` | **DONE** — `823b34c`; 3 ca multi-lock + 10 sig; chờ A0 chốt semantics `--root` |
| VS3-6 — Windows Tier 1 + wasm | W3 | `test/host`, local/psvn | TODO | sau P0/P1; **không GitHub Actions** |
| VS3-7 — LSP tệp phụ/incremental | W3 | `lsp_da_tep`, `lsp_incremental` | **DONE** — `1aefb88` + `3510232`; overlay + `change:2` UTF-16/CRLF |
| VS3-8 — Plugin/ViTerm | W3 | `vs3_plugin.rs` | TODO | P3 |

## 2b. Đang chạy (21/11, 5 luồng)

| Luồng | Worktree / branch | Phạm vi | Trạng thái |
|---|---|---|---|
| **W1** | `vs3-w1-runtime` / `vs3/w1-runtime` | VS3-1a/1b/1c/1d/2a | **DONE (một phần)** — `1de3faa`; async default + Pipe; cap>0/net async còn mở |
| **W2** | `vs3-w2-tls` / `vs3/w2-tls` | VS3-4a/4b/4b2/4b3 | **DONE** — h2 reuse + **default-on** cho v0.3.0; Windows h2 chờ W4 |
| **W3** | `vs3-w3-registry` / `vs3/w3-registry` | VS3-5a/5b/7 + QA | **DONE (QA v0.2.0)** — `629acf0`; gate 3480/3, nightly 14,7M fuzz 0 vi phạm, diff 100k 0 lệch, host 54/54, targets 18/18; fresh-clone chờ |
| **W4** | `vs3-w4-windows` / `vs3/w4-windows` | Windows fixes + local verify | **DONE (v0.2.0)** — `ec29204` 3436/0; targets 18/18+17/1; **không GitHub Actions** |
| **W5** | `vs3-w5-spike` / `vs3/w5-spike` | VS3-3a/3b/5c + 2b | **DONE (một phần)** — `...` + `8da533b` + `1844370`; timeout + bench example (không ignore) |

**Quy tắc tích hợp**: subagent commit local trong nhánh; A0 review + chạy targeted test; merge tuần tự
vào `main`; full `gate --fast` tại M1–M4. Không subagent nào push/merge trực tiếp.

### Sự kiện 21/11 — Windows @ `v0.1.0-rc1` KHÔNG xanh

- `cargo test --workspace --no-fail-fast`: **3218 passed / 38 failed / 3 ignored**, 20/329 target đỏ.
- `vis-dev check`: clippy đỏ (`unused variable: marker`).
- 2 finding **sản phẩm**: `fs.read_link` symlink ra ngoài root không chặn; `fs.canonicalize` trả `Err`
  dữ liệu thay vì `VS0604`.
- 35 ca còn lại: 25 portability + 10 test-only; 1 BLOCKED-ENV (`wasm32` std target).
- **Quyết định**: giữ `v0.1.0-rc1` làm mốc lịch sử, **không promote `v0.1.0`**; W4 đang sửa; tag
  `v0.1.0-rc2` sau khi `vis-dev check` xanh + workspace 0 product/test failure.
- Báo cáo: `agent/JOURNAL/vs3-w4-windows-verify.md`.

### Sự kiện 21/11 — release `041ee50` đỏ 5 ca trên Windows

- `cargo test --workspace`: **3396 passed / 5 failed / 3 ignored** (344 block); `vis-dev check` PASS;
  host 54/54. `vis-dev check` không chạy full `cargo test` nên không đủ kết luận.
- 5 ca: 2 **product** (overlay LSP tra canonical; `localhost` phân giải `::1` trước khi server bind
  `127.0.0.1`) + 3 **test-only** (registry HTTPS socket non-blocking).
- Đã cherry-pick `3d6bfe4` (overlay) + `5dec89b` (registry test); W2 sửa `net.rs` xong (`d9046a9`:
  thử lần lượt mọi địa chỉ loopback, IPv4 trước) → merge `8776947`; gate Linux **3448/3**.
- **Không tag `v0.1.0`** cho tới khi W4 verify lại toàn bộ trên commit mới.
- Báo cáo: `agent/JOURNAL/vs3-w4-release-final.md`.

## 3. Mốc tích hợp cuối tuần

| Mốc | Giờ (JST) | Việc | Kết quả |
|---|---|---|---|
| M1 | T7 19:00 | merge wave 1 + `gate --fast` | **PASS — 3332 passed / 4 skipped** (21/11) |
| M2 | CN 02:00 | merge wave 2 + gate + nightly nhẹ | **PASS — 3364 passed / 3 skipped** (21/11) |
| M3 | CN 14:00 | merge wave 3 + gate + diff 100k | **gate PASS — 3487 passed / 3 skipped**; diff 100k W3 PASS |
| M4 | CN 21:00 | hardening + gate + nightly đầy đủ | PASS |
| M5 | T2 06:00 | fresh-clone + Windows (nếu runner) + báo cáo | tag `v0.3.0` nếu xanh; dừng 09:00 |

## 3a. Vòng v0.2.0/v0.3.0 (21/11 15:40 → 23/11 09:00)

| Việc | Chủ | Trạng thái | Điều kiện xong |
|---|---|---|---|
| **VS3-1d** async default + Pipe parity | W1 | **DONE (một phần)** — async chạy frame, sync byte-identical, guard rõ; cap>0/net async W3 |
| **VS3-3b** worker opt-in | W5 | **DONE (một phần)** — pool opt-in + TaskId/proxy + print merge + budget atomic; B7 CPU-only 2,69×; payload lớn chưa đạt; chưa nối evaluator |
| **VS3-4b2/4b3** HTTP/2 + reuse | W2 | **DONE** — reuse `28f9ea2` merged; **default-on** v0.3.0; 7 ca h2; Windows chờ |
| **VS3-6** Windows local | W4 | **DONE (một phần)** — tag v0.1.0 3404/0; psvn target 18/18+17/1; **không GitHub Actions** |
| **QA/release** | W3 | **WIP** | gate/nightly/diff/host + checklist v0.2/v0.3 |
| **VS3-2b** timeout kênh | W5 | **DONE (một phần)** — `recv_timeout`/`send_timeout`, 17 ca; `select` VS3-2c |

**v0.3.0 default-on**: W2 thử `959d1aa` (+7,8% release, wasm không kéo h2) — **chỉ merge sau khi tag
v0.2.0 và W4 verify h2 trên Windows**. Trước đó main giữ `http2` default-off.

**v0.2.0 đã tag** @ `42b700c`: Linux 3480/3; Windows 3436/0; fresh-clone PASS; host 54/54;
targets 18/18. Tàn dư ghi trong tag: async net production, worker evaluator, HTTP/2 default-off.

**v0.3.0**: merge W1f (`098b649`, async cap≥1), W2 h2 reuse `28f9ea2` + default-on `959d1aa`,
W5 bench example `1844370`; W3/W4 verify; tag khi xanh. Không GitHub Actions.

**Dừng**: 09:00 thứ Hai 23/11/2026; sau 08:00 không mở task lớn mới, chỉ đóng/tag/báo cáo.

## 3b. A0 chốt VS3-3b (từ spike W5)

- Mô hình **affine + heap theo luồng + copy/move tại biên**; **không** `Arc`/COW toàn cục, không shared mutable heap.
- Task handle **opaque `TaskId`/proxy**, không `Arc<Mutex<Task>>` toàn cục.
- Channel `Arc<Mutex<ChanBuf<Wire>>>` + `Condvar`; chấp nhận 2 lần chuyển đổi; chỉ cân nhắc leaf-`Arc` nếu đo được.
- **Top-level `var`**: worker pool chỉ chạy chương trình không có ghi chéo worker; spawn nhận snapshot; ghi chéo ⇒ từ chối rõ ở chế độ parallel.
- `print`: buffer theo task + merge tất định để W=1/W=4 byte-identical.
- Ngân sách: một pool **atomic** dùng chung, không thread-local.
- Benchmark gate: CPU-only + payload matrix; ghi topology/load; ngưỡng 2,4× có caveat (hybrid 2P+8E đo 2,45×).
- Worker pool **opt-in/test-only**; default vẫn single-thread tới khi nghiệm thu + VERSION/cache.
- **B1**: chưa cấp mã mới cho ghi chéo worker; VS3-3b dùng lỗi rõ, A0 cấp mã sau.
- **B2**: W=1 giữ **shared env** như hiện tại; W>1 snapshot + từ chối ghi chéo.
- **B3**: print merge **spawn order**; output task lỗi/cancel giữ tới trap, lỗi surfaced ở join.
- **B4**: budget atomic + **chunk reservation** (fetch_sub block) để giảm tranh chấp; default off.
- **B5**: `TaskId`/proxy, không đổi ABI `Value` cho tới VS3-3b; W1 sở hữu.
- **B6**: tích hợp W1 **sau VS3-2a**.
- **B7**: gate 3b dùng CPU-only + payload matrix, ghi topology/load; RSS chỉ tham khảo.
  **Kết quả W5 (21/11)**: CPU-only W=4 **2,69×** (PASS ≥2,4); payload nhỏ/record/enum/str đạt;
  `list10k` 1,93× và `map1k` 0,77× fail vì copy trên producer — worker-copy đạt 3,42×/3,12×;
  chunk **256**; map allocator/drop còn mở. Benchmark chuyển example (không `#[ignore]`) đang sửa.

## 4. Quy tắc cập nhật

- Mỗi lần merge: cập nhật dòng lát tương ứng + commit hash + test đã chạy.
- `BLOCKED` phải ghi lý do + người/cái gì gỡ; không để trạng thái mơ hồ.
- Không đánh dấu `DONE` khi chưa có test trong repo + gate tương ứng.
- macOS không xuất hiện trong bảng nghiệm thu; chỉ ghi chú “ngoài phạm vi”.
