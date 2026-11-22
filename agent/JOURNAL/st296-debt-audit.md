<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296 — Debt audit lịch sử VS2/VS3 (`VD-281` §2.6)

> **Người rà**: W3 (Gatekeeper QA), 2026-11-22 · **docs-only**: không sửa `SO-NO.md`/`QUEUE.md`/code.
> **Baseline**: `plan/w3-platform` merge `main` **`b39a7c3`** (gate **3638/4**; release **3643/0/4**).
> **Mục đích**: đề xuất cho W4/A0 cập nhật sổ nợ; mọi kết luận kèm commit/test/lý do.
> **Không** tag/push/merge; không GitHub Actions.

## 1. Bảng đề xuất

| # | Trạng thái sổ | Bằng chứng (commit + test/log) | Đề xuất | Ghi chú |
|---|---|---|---|---|
| **16** | chuyển VS3 | `d060f5a` (VS3-2a wait-set + close/cancel); `vs3_chan0` **13/13**, gồm `chan0_close_danh_thuc_receiver_tra_none`, `chan0_close_danh_thuc_tat_ca_receiver`, `chan0_close_idempotent_va_recv_none` | **ĐÓNG** | `close` idempotent + đánh thức waiter đúng; test xác nhận receiver `None`, sender `VS0504` |
| **17** | chuyển VS3 | `8da533b` (`recv_timeout`/`send_timeout`, `vs3_chan_timeout` **17/17`) + `82064e9` (`select_*`, `vs3_select` **25/25**) | **ĐÓNG** | Timeout + select 4 hàm, fairness/cancel/đóng; interp↔VM byte-identical |
| **19** | chuyển VS3 | `d060f5a`; `vs3_chan0` **13/13** (`chan0_*` send/recv rendezvous, close, cancel) | **CẦN A0** | `cap=0` chạy trên **frame/async**; runtime đồng bộ vẫn từ chối (`rt.rs` `tao_kenh`: `cap == 0 && !frame.enabled`). Nếu A0 chấp nhận phạm vi frame ⇒ đóng; nếu đòi cả `vis run` sync ⇒ giữ mở |
| **21** | chuyển VS3 | TLS `a5e3040`; keep-alive `b7385e9`; test `vs3_tls` **21/21**, `vs3_keepalive` **8/8**, `vs3_http2` **34+1** | **ĐÓNG** | HTTPS/rustls, keep-alive, HTTP/2; loopback policy giữ nguyên có chủ đích |
| **26** | mở | `9efa420` (ST-84 completion/hover module-aware); test exact `lsp_completion_thay_ham_module`, `lsp_hover_ham_module` **pass** | **ĐÓNG** | Completion/hover thấy item `pub` của `use ./x` + `std.*`; detail nêu nguồn, khử trùng |
| **27** | mở | `3510232` (incremental `change:2` UTF-16/CRLF); `lsp_incremental` **4/4**; full sync vẫn hỗ trợ | **GIỮ MỞ / CẦN A0** | Full sync đã có thêm incremental; còn **một luồng** và **EOF ⇒ exit 0** (chủ ý cho test/CI), khác spec LSP — cần A0 chốt có đóng phần còn lại hay chỉ đóng phần incremental |
| **31** | mở | entry `837d5ff` (ST-90, `pkg` **25/25**); GC `cea1643` (`vs3_pkg_gc` **5/5**, `--root` multi-lock `vs3_pkg_gc_multi` **4/4**); **còn** lock chỉ tìm ở `main_dir` (`modules.rs` dùng `self.main_dir.join("vis.lock")`) | **CẦN A0** | 2/3 vế đã xong (entry + GC store); vế “lock chỉ main_dir” vẫn đúng ⇒ tách nợ hoặc đóng có phạm vi |
| **84** | mở | `096ea2c` cookie jar + Basic auth; `vs3_cookie_auth` **11/11** (Set-Cookie→redirect→cookie, Path/Secure/Max-Age, auth **không** cross-host); `vs3_keepalive` **8/8** | **ĐÓNG** | Cookie giữ qua redirect cùng host; auth chỉ cùng origin (an toàn), cross-host bị bỏ; 303 POST không áp dụng vì API chỉ GET/HEAD; “giới hạn loopback” là policy `tach_url` có chủ đích |
| **98** | mở | `1aefb88` (VS3-7 publish chẩn đoán tệp phụ + overlay); `lsp_da_tep` **8/8** | **GIỮ MỞ / CẦN A0** | Publish cho **tài liệu đang mở** phụ thuộc; chưa publish chẩn đoán tệp phụ **chưa mở** theo file-id. Cần A0 quyết có mở rộng LSP đa tài liệu hay đóng theo phạm vi hiện tại |
| **191** | mở | W4 `JOURNAL/st296-pre-qa.md`: `cargo nextest run --workspace --no-fail-fast` trên `psvn` @ `f6939721` — **350 binary, 3583/3583 passed, 3 skipped, 0 failed/flake**, 213,4 s test / 481 s tổng, log sha256 `412a4e43…b47b` | **ĐÓNG** | Không còn `tcp_explain_tong`/WSAECONNABORTED; nextest chạy xanh trên Windows; A0 cần lật sổ và giữ ghi chú nextest là đường QA hợp lệ |

## 2. Test/bằng chứng đã chạy trong phiên

```text
vs3_chan0 13/13 · vs3_chan_timeout 17/17 · vs3_select 25/25
vs3_tls 21/21 · vs3_keepalive 8/8 · vs3_cookie_auth 11/11
vs3_pkg_gc 5/5 · pkg 25/25 · lsp_da_tep 8/8 · lsp_incremental 4/4
lsp_completion_thay_ham_module / lsp_hover_ham_module exact → pass
./vis-dev check → PASS (fmt, clippy, R1..R6, report-check 32/327, doc-examples, readme 20/20)
```

Không sửa `SO-NO.md`/`QUEUE.md`; W4/A0 áp dụng bảng trên theo `VD-281 §2.6`.

## 3. Blocker

Không có blocker kỹ thuật. Các mục **CẦN A0** (`#19`, `#31`, `#27`, `#98`) là quyết định **phạm vi đóng nợ**,
không phải lỗi mới. `#191` đã có bằng chứng xanh để đóng. Đĩa cuối **31 G** (> 10 G).
