<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 — bảng tiến độ (A0, mở 2026-11-21)

> **Nguồn kế hoạch**: `agent/KE-HOACH-VS3-2026-11-21.md`.
> **Trạng thái hợp lệ**: `TODO | WIP | REVIEW | DONE | BLOCKED`.
> **Luật**: mỗi lát phải có test **trước** khi merge; full gate tại mốc 4 giờ; không dồn test cuối tuần.
> macOS **ngoài phạm vi**; Linux là nền chính; Windows Tier 2 tới VS3-6.

> **Cập nhật 2026-11-22 (VD-272..VD-279, A0 commit `1c58ba3`)** — A0 chốt **roadmap hậu `v0.3.5`**:
> `v0.3.6` R0 Refactor & Hygiene → `v0.4.0` G4a Core → `v0.4.5` Windows Tier 1 & WASI → `v0.5.0` Pha 2 & ViShell → `v0.6.0`/`v1.0.0-rc1` Plugin ABI & ViTerm.
> **Cổng chặn cứng**: trước ST-284..286, không mở task tính năng chạm `vm.rs`, `frame.rs`, `value.rs`,
> `codegen.rs`, `tyck/expr.rs`, `interp/goi.rs`, `async_net.rs`, `quyen.rs`, `db/sqlite/mod.rs`;
> các tệp lõi phải **≤1.250 dòng**, 0 đổi hành vi. **4 worktree** (`W1` runtime, `W2` net/sec, `W3` platform, `W4` QA/docs);
> W5 đã nghỉ. Baseline v0.3.5 @ `03bb7b9`; Stage 0 `ST-284`..`ST-289`: **đủ 6/6 DONE**
> (cập nhật 2026-11-22 @ **`92e381b`** / code `4c5dec6` — final QA Linux xanh; **tag `v0.3.6` CHƯA tạo vì blocker h1 cancel Windows**, chờ fix ST-287; xem §2c).

## 1. Mốc hiện tại

| Mục | Giá trị |
|---|---|
| Baseline v0.1.0 | `50675dd` (tag `v0.1.0`) |
| Gate v0.1.0 | **3448 passed / 3 skipped** |
| Nightly baseline | PASS (fuzz 17.198.259 ca / 0 vi phạm) |
| `FUNCS` | **167** (mốc v0.1.0) |
| Tag | **`v0.3.5`** @ `03bb7b9` (patch cancel-before; Windows async net/TLS, async-h2 default-off, evaluator P1-P3); **`v0.3.6` CHƯA tag** @ `4c5dec6` (Linux final QA xanh; **blocker h1 cancel Windows — chờ fix ST-287**); `v0.3.4` @ `8fa1f96`, `v0.3.3` @ `bbf3484`, `v0.3.2` @ `864cc96`, `v0.3.1` @ `e689c3d`, `v0.3.0` @ `0d26527`, `v0.2.0` @ `42b700c`, `v0.1.0` @ `50675dd` lịch sử; `pha1-rc1` giữ nguyên |
| Nền đo | Linux `ps`; macOS bỏ; Windows Tier 2 |

## 2. Bảng lát cắt

| Lát | Chủ | Test chính | Trạng thái | Ghi chú |
|---|---|---|---|---|
| VS3-1a — budget/step-cap | W1 | `vs3_budget.rs` + diff | **DONE (một phần)** — `3941619`; fuel 100M, parity 2M; BUDGET/Q/default còn mở |
| VS3-1b — cancellation/cleanup | W1 | `vs3_cancel.rs` | **DONE (một phần)** — `97676c6`; 14 ca; channel wait-set để VS3-2a |
| VS3-1c — coverage HIR | W1 | `vs3_coverage_matrix.rs` | **DONE (một phần)** — `7e1deed`; 10 ca, sửa `?.` frame; `Pipe`/nested async chặn 1d |
| VS3-1d — flip default `async` | W1 | `vs3_default.rs` + gate | **DONE (một phần)** — `d0e7998`; async net/TLS/HTTP/1.1 Unix **+ Windows** (W4 production hoá); async h2 **design-only** (W2 giữ `net/h2.rs`), feature default-off/Unix-only |
| VS3-1d/h2 — async HTTP/2 | W1/W2 | design + spike + tích hợp | **DONE (một phần)** — `c17ac7e`; `h2_core` chung, feature `async-h2` default-off; 4 stream/1 conn **cold 1780 mẫu**, Set-Cookie h1+h2; 34 pass + fd 5→5; cap hot-spin + cold-wake **đã sửa** (1000 cold/4000 cap); vẫn default-off; cancel hook + Windows mở |
| VS3-2a — wait-set/`chan(0)` | W1 | `vs3_chan0.rs` | **DONE (một phần)** — `d060f5a`; 13 ca; cap≥1/select còn mở |
| VS3-2b — timeout kênh (`select` hoãn 2c) | W5 | `vs3_chan_timeout.rs` | **DONE (một phần)** — `8da533b`; 17 ca `recv_timeout`/`send_timeout`; `select` VS3-2c |
| VS3-3a — spike worker/heap | W5 | `vs3_spike.md` + bench | **DONE** — `4bb20df`; chọn affine + per-worker heap (B1–B6) |
| VS3-3b — worker pool | W5 | `vs3_worker.rs` | **DONE (một phần)** — `c94de13`; stress 10k + TaskId/proxy proto feature default-off; chưa nối evaluator |
| VS3-4a — TLS client | W2 | `vs3_tls.rs` | **DONE** — `a5e3040`; 10 ca TLS, HTTP/1.1 giữ; HTTP/2/keep-alive còn mở |
| VS3-4b — keep-alive + IPv6 + registry HTTPS + cookie/auth | W2 | `vs3_keepalive.rs`, `vs3_registry_https.rs`, `vs3_cookie_auth.rs` | **DONE (một phần)** — `b7385e9` + `cc74dbc` + `096ea2c`; 8+6+9 ca |
| VS3-4b2 — HTTP/2 | W2 | `vs3_http2.rs` | **DONE** — `7d73268`; **default-on**; 8 ca h2; Windows verify 3452/0 |
| VS3-4b3..4b6 — h2 reuse/idle/hardening/multiplex | W2 | `vs3_http2.rs`, h2 unit | **DONE (một phần)** — `735ef39`; 13 ca h2 + prototype 4 stream test-only; production vẫn tuần tự |
| VS3-5a — registry HTTP + chữ ký | W3 | `vs3_registry.rs` | **DONE** — `756bb1d`; 11 ca + SHA-256; chữ ký/GC còn mở |
| VS3-5b — chữ ký/GC/migrate | W3 | `vs3_pkg_gc.rs` | **DONE** — `cea1643`; Ed25519 + `pkg gc` + migrate; mã `VS0708` cấp ở **ST-288** |
| VS3-5c — key rotation + GC nhiều lock | W5 | `vs3_pkg_gc_multi.rs` | **DONE** — `823b34c`; 3 ca multi-lock + 10 sig; semantics `--root` khoá ở **ST-288** (4 ca) |
| VS3-6 — Windows Tier 1 + wasm | W3 | `test/host`, local/psvn | TODO | sau P0/P1; **không GitHub Actions** |
| VS3-7 — LSP tệp phụ/incremental | W3 | `lsp_da_tep`, `lsp_incremental` | **DONE** — `1aefb88` + `3510232`; overlay + `change:2` UTF-16/CRLF |
| VS3-8 — Plugin/ViTerm | W3 | `vs3_plugin.rs` | TODO | P3 |

## 2b. Đang chạy (21/11, 5 luồng)

| Luồng | Worktree / branch | Phạm vi | Trạng thái |
|---|---|---|---|
| **W1** | `vs3-w1-runtime` / `vs3/w1-runtime` | VS3-1a..1d/2a + fuzz | **DONE (một phần)** — async default+cap+net+TLS+HTTP Unix **và Windows** (W4, 10 vòng psvn); fuzz 4h chạy trên d1f8f70 |
| **W2** | `vs3-w2-tls` / `vs3/w2-tls` | VS3-4a/4b/4b2..4b6 | **DONE** — h2 100 vòng v0.3.3; async-h2 production default-off (29 pass) |
| **W3** | `vs3-w3-registry` / `vs3/w3-registry` | VS3-5a/5b/7 + QA | **DONE (QA)** — soak 10/10 v0.3.3 646 s; nightly 13,7M fuzz 0; fresh-clone PASS; đang 20× soak |
| **W4** | `vs3-w4-windows` / `vs3/w4-windows` | Windows fixes + CLI + async net | **DONE (Windows async net/TLS)** — `12e961c`; 22/22+22/22×10, workspace 3496/0×2, host 54/54; async-h2 Unix-only |
| **W5** | `vs3-w5-spike` / `vs3/w5-spike` | VS3-3a/3b/5c/2b + evaluator | **DONE (P1/P2/P3)** — `spawn_shared` 6M spawn; evaluator P1/P2/P3 `736dc5c` (VM 2,3×); gate 3571/3; frame P4/cancel sản phẩm còn mở |

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

## 2c. Stage 0 — `v0.3.6` (2026-11-22, `main` `4c5dec6`)

> Nguồn: `QUEUE.md` ST-284..289, `JOURNAL/st284-refactor.md`, `JOURNAL/st285-refactor.md`,
> `JOURNAL/st286-tach-module.md`, `JOURNAL/st287-http-cancel.md`, `JOURNAL/st287-soak-1h.md`,
> `JOURNAL/st288-goi.md`, `JOURNAL/st28{4,5,6,7}-diff-100k.log`, `JOURNAL/stage0-win-0d057e5.md`,
> `JOURNAL/final-win-v036.md`, `JOURNAL/final-qa-v036.md`, `agent/reports/R32-viscript.md`.
> **Stage 0 đủ 6/6 DONE**; R32 đã chốt phần Stage 0. Không push/merge/tag từ W4; không GitHub Actions.

| Task | Trạng thái | Commit | Bằng chứng |
|---|---|---|---|
| **ST-284** — R0a `vm.rs`/`frame.rs` | **DONE** | `83a9a10` (merge `e363a15`) | `vm.rs` **1240** dòng, `frame.rs` **1042** (ngưỡng 1250); 0 đổi API/ABI/cache; gate `3571/3`; diff 100k seed `20261115` **lech=0** |
| **ST-285** — R0b `value.rs`/`codegen.rs`/`tyck/expr.rs`/`interp/goi.rs` | **DONE** | `ab5d292` (merge `246b6c5`) | `value` **1186**, `codegen` **1045**, `tyck/expr` **1064**, `interp/goi` **1155** dòng (≤1250); tách `value/show_eq.rs`, `codegen/cau.rs`, `tyck/expr/goi.rs`, `interp/goi_ham.rs`; 0 đổi golden/API/cache; gate `3576/3`; diff 100k seed `20261115` **lech=0** (200,5 s) |
| **ST-286** — R0c `async_net.rs`/`quyen.rs`/`db/sqlite/mod.rs` | **DONE** | `f193c57` (merge `cfba662`) | `async_net` **606**, `quyen` **1190**, `sqlite/mod.rs` **1245** dòng; module con tách giữ API; gate PASS; diff 100k seed `20261122` **lech=0** (93 770 ca chạy) |
| **ST-287** — hook huỷ giữa HTTP request | **DONE — BLOCKER Windows** | `03ed84c` (merge `f8f1d4b`) | `FrameReactorBackend::cancel_task_handles` + safe point `frame_cancel_one`; Linux: h1 **5/5**, h2 **8/8**, đỏ-trước 5/5 + 7/7, soak 1 giờ fd/RSS/threads phẳng; gate `3581/3`; diff 100k **lech=0**. **Windows: `cancel_http_h1_lap_nhieu_vong_khong_ro` flake ~21 %** (3/14 suite; `V:post` sau 10053) — chờ fix |
| **ST-288** — `VS0708` + `pkg gc --root` + `#142` | **DONE** (Gatekeeper 2026-11-22) | `c513e7a` (merge `545a38d`) | `VS0708` vi/en/ja + `vis explain --list` **54** + `API-CODES.txt` **54**; 6 ca âm (gồm key rotation) đều nêu `VS0708`; soft pin `pkg update` (`update_mot_ten_*` 2/2 + unit) ⇒ **#142 đóng**; `gc --root` lặp/lồng dedupe; `pkg` 25/25, `vs3_registry_sig` 10/10, `vs3_pkg_gc_multi` 4/4. QUEUE còn `REVIEW` — chờ A0 lật sổ |
| **ST-289** — version `0.3.6` + `#111`/`#151` + R32 | **DONE** (docs + QA) | `ff55056` (merge `84915aa`); QA `c69127f`/`a997e06`; docs `705afcf` | `version.workspace = true` 8 crate; `vis 0.3.6`; **#111** khoá `VS0412`; **#151** 24 fixture slug (R100%); Linux final QA §2c; R32 cập nhật blocker Windows; **tag chưa tạo** |

**Cổng Stage 0 — final QA** @ **`4c5dec6`** (fresh-clone Linux): `./vis-dev gate --fast` **PASS — 3581 passed / 0 failed / 3 skipped**
(fmt, clippy, R1..R6, self-test, doc-examples 24 khối, L0 nextest); `cargo test --release --workspace --no-fail-fast`
**3586 passed / 0 failed / 3 ignored**; P2 14/14, P1 8/8, P3 10/10; h2 19/19 + feature 34+1; `pkg` 25/25,
`vs3_registry_sig` 10/10, `vs3_http_cancel` 5/5 + feature 8/8, host 54/54; diff 100k seed `202611221700`
**0 lệch**; **2× soak 10/10 = 20/20 vòng** (593 s + 602 s), mỗi soak 3 gate 3581/3 + 3 diff 0 lệch + 2 host 54/54
và 2 cache 5/5.

**Windows final @ `a1930cb`** (`psvn`, Tier 2): workspace lần 2 **3538/0/3** (463 s, đúng kỳ vọng);
lần 1 **3533/5/3** (528 s) — 5/5 `vs3_http_cancel` h1 đỏ, chạy chồng 10 phút cuối soak `f80f774`.
**`vs3_http_cancel::cancel_http_h1_lap_nhieu_vong_khong_ro` flake ~21 %** (3/14 lần suite, kể cả
`--test-threads=1`; ca riêng ×10 **10/10**): task bị huỷ vẫn in `V:post` sau `WSAECONNABORTED 10053`.
Feature `async-h2` trên Windows **BLOCKED-ENV** rc 101 (Unix-only), không đo 8/8. Targeted khác:
P2 14/14, P1 8/8, P3 10/10; h2 19, `pkg` 25, sig 10, default 22, TLS 22, net 9; host 54/54;
targets 18/18 + 17/1; release `vis.exe` 11 228 672 B (`JOURNAL/final-win-v036.md`).

**Trạng thái Stage 0**: **đủ 6/6 DONE** trên `main` @ **`4c5dec6`** (merge **`a1930cb`**, docs `92e381b`);
R32 chốt phần Stage 0 (`agent/reports/R32-viscript.md`); docs phát hành cập nhật (CHANGELOG `v0.3.6`,
STATE/ROADMAP/VS3-TIEN-DO). **`v0.3.6` CHƯA tag — blocker h1 cancel Windows; chờ fix ST-287 rồi verify lại
(`vs3_http_cancel` ×10 + workspace 1 vòng).** Giai đoạn 1 (`ST-290..`) vẫn `BLOCKED` chờ A0 mở.

## 3. Mốc tích hợp cuối tuần

| Mốc | Giờ (JST) | Việc | Kết quả |
|---|---|---|---|
| M1 | T7 19:00 | merge wave 1 + `gate --fast` | **PASS — 3332 passed / 4 skipped** (21/11) |
| M2 | CN 02:00 | merge wave 2 + gate + nightly nhẹ | **PASS — 3364 passed / 3 skipped** (21/11) |
| M3 | CN 14:00 | merge wave 3 + gate + diff 100k | **gate PASS — 3530 passed / 3 skipped**; soak 10/10 v0.3.2 PASS |
| M4 | CN 21:00 | hardening + gate + nightly đầy đủ | PASS |
| M5 | T2 06:00 | fresh-clone + Windows (nếu runner) + báo cáo | `v0.3.0` đã tag; hardening tới 09:00 |

## 3a. Vòng v0.2.0/v0.3.0 (21/11 15:40 → 23/11 09:00)

| Việc | Chủ | Trạng thái | Điều kiện xong |
|---|---|---|---|
| **VS3-1d** async default + Pipe parity | W1 | **DONE (một phần)** — async chạy frame, sync byte-identical, guard rõ; cap>0/net async W3 |
| **VS3-3b** worker opt-in | W5 | **DONE (một phần)** — pool opt-in + TaskId/proxy + print merge + budget atomic; B7 CPU-only 2,69×; payload lớn chưa đạt; chưa nối evaluator |
| **VS3-4b2/4b3** HTTP/2 + reuse | W2 | **DONE** — default-on v0.3.0; 8 ca h2; Windows verify 3452/0 |
| **VS3-6** Windows local | W4 | **DONE (một phần)** — tag v0.1.0 3404/0; psvn target 18/18+17/1; **không GitHub Actions** |
| **QA/release** | W3 | **WIP** | gate/nightly/diff/host + checklist v0.2/v0.3 |
| **VS3-2b** timeout kênh | W5 | **DONE (một phần)** — `recv_timeout`/`send_timeout`, 17 ca; `select` VS3-2c |

**v0.3.0 HTTP/2 default-on**: đã merge và verify Windows; release +~0,73 MB, wasm không kéo h2.

**v0.2.0 đã tag** @ `42b700c`: Linux 3480/3; Windows 3436/0; fresh-clone PASS; host 54/54;
targets 18/18. Tàn dư ghi trong tag: async net production, worker evaluator, HTTP/2 default-off.

**v0.3.0 đã tag** @ `0d26527`.
**v0.3.1 đã tag** @ `e689c3d`.
**v0.3.5 đã tag** @ `03bb7b9` (thay `v0.3.4` @ `8fa1f96` do race cancel-before): Linux **3571/3**; Windows **3528/0**; fresh-clone 7e72afd PASS; cancel 1000/1000; feature h2 34+1 ×3; soak 20/20; diff 0. Tiếp tục hardening tới 09:00 T2. Không GitHub Actions.

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
