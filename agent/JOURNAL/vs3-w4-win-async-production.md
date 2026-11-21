<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — production hoá Windows async net/TLS (mio `Source`, IOCP/AFD)

> **Người làm**: subagent VS3-W4, 2026-11-22.
> **Branch**: `vs3/w4-windows` (KHÔNG merge/push/tag vào `main`).
> **Mốc**: `main` `9241870` (async-h2 default-off) → merge `origin/main` `73f63ff`
> (gồm W5 soak + `vs3_worker.rs` 698 dòng) vào branch; xung đột duy nhất `async_net.rs`.
> **Kết quả**: Windows async net/TLS/HTTP/1.1 **đã chạy production** (10 vòng targeted + 3 vòng
> workspace xanh, 1 flake sync-net có sẵn); gate `--fast` còn **R1 `vs3_worker.rs` 1926 dòng**
> (có sẵn trên main, ngoài phạm vi). Không dependency mới ngoài `mio 1.2.5`.

## 0. TL;DR

| Hạng mục | Kết quả |
|---|---|
| Merge `main` | 1 xung đột `async_net.rs` — giải bằng bookkeeping theo **khoá nguồn** thay fd; async-h2 **giữ nguyên** trên Unix |
| Windows async net/TLS/HTTP/1.1 | **CÓ** — 10 vòng `vs3_default`+`vs3_tls` 22/22 + 22/22, 0 fail |
| Windows workspace | **3 vòng xanh** (WS1 3496/0/3, WS3 3496/0/3, WS4 3496/0/3); WS2 flake sync-net |
| Linux | targeted 22/19/21; async-h2 29/29 + 1 ignored; workspace **3544/0/3**; clippy/fmt OK |
| fd/handle leak | ca `async_net_khong_ro_ri_tay_sau_200_vong` xanh mỗi vòng (đếm nội bộ = 0) |
| Invariant host_poll | ca `host_tls_tren_socket_reactor_tra_loi_ro` xanh mỗi vòng TLS (Windows) |
| Gate `--fast` | **FAIL** chỉ vì `R1 crates/vis-cli/tests/vs3_worker.rs: 1926 dòng > 1500` (có sẵn trên `origin/main` `73f63ff`, W5 thêm 698 dòng) |
| async-h2 trên Windows | **chưa hỗ trợ** — feature default-off, `h2_async.rs` vẫn `cfg(unix)`; `cargo check --features async-h2` trên Windows đỏ (ghi rõ, không tuyên bố) |
| Blocker | Không có blocker sản phẩm; 2 nợ **có sẵn trên main**: R1 `vs3_worker` + flake `net.rs::tcp_explain_tong` |

## 1. Merge `main` và xung đột

- `git merge origin/main` (`73f63ff`, chứa `9241870` async-h2 + W5 soak): chỉ **1** xung đột
  nội dung trong `crates/vis-cli/src/async_net.rs`.
- Main đã tách `async_net_h2.rs` + `async_net_host.rs`; `register`/`deregister`/`wait` dùng
  `fd`/`SourceFd` + nhiều token/fd cho h2.
- Giải pháp: giữ **nguyên** logic h2/session của main, thay lớp đăng ký bằng abstraction
  `Nguon` + bookkeeping theo **khoá nguồn canonical** (không dùng `fd` thô):
  - `token_nguon: token → khoá`, `nguon_tokens: khoá → {token}`, `poll_token/poll_nguon`,
    `nguon_wants: khoá → (đọc, ghi)`.
  - `Nguon::Stream/Listen` đăng ký **trực tiếp** đối tượng mio.
  - `Nguon::H2` (thực chất là `H2Async::dang_ky_nguon`) vẫn dùng `SourceFd` **bên trong**
    `net/h2_async.rs` (module Unix-only) vì socket nằm trong connection h2, không mượn được
    `&mut MioStream`. `async_net.rs` **không còn** `SourceFd`/`AsRawFd`/`RawFd`.
- Chuyển khoá `N:…`→`C<n>`, `T:…`→`C<n>`/`H<n>` bằng `doi_khoa_nguon` để token/interest/
  registration không stale (Windows đăng ký gắn với đối tượng, không với fd).
- Tách `crates/vis-cli/src/async_net_nguon.rs` (282 dòng): `la_would_block`, `Nguon`,
  `cho_san_sang`, bookkeeping, invariant — giữ `async_net.rs` **1483 dòng** (R1 ≤ 1500).

## 2. P1/P2 đã làm

1. `cfg(any(unix, windows))` ở `lib.rs`, `async_net.rs`, 7 gate `main.rs`, Cargo target dep;
   wasm vẫn fallback cũ (`cfg(not(any(unix, windows)))`).
2. `set_nonblocking(true)` trước `MioStream/MioListener::from_std` cho `connect`/`listen`
   (spike đã có, giữ; đây cũng là lỗi tiềm ẩn trên Unix).
3. `la_would_block(&e)`: `ErrorKind::WouldBlock` + mã thô **115** (Unix `EINPROGRESS`),
   **10035** (`WSAEWOULDBLOCK`), **10036** (`WSAEINPROGRESS`); thay mọi so sánh `Some(115)`.
4. `register` gặp `AlreadyExists` → `reregister` (không deregister+register); trên Windows
   `reregister` bỏ qua `Registry` truyền vào nên chỉ dùng trên đúng `Poll` sở hữu.
5. Invariant `host_poll` vs reactor (Windows): `dang_reactor_win(tay)` kiểm `tokens` của reactor;
   host TLS `read`/`write` gặp socket reactor-owned trả lỗi rõ
   `"tay … đang thuộc reactor scheduler (Windows: một socket chỉ thuộc một Poll)"` — không
   deregister mù. `cho_san_sang` trả `LoiChoSanSang::DangReactor` (kể cả `AlreadyExists`).
6. `wait` phát event cho **mọi** token của cùng khoá nguồn (h2 nhiều frame / 1 connection);
   log đổi `epoll_wait` → `poll readiness`.
7. Test mới:
   - `async_net_khong_ro_ri_tay_sau_200_vong` (mọi nền): 200 vòng listen/connect/accept/read/write/
     close, `dem_tay_dang_mo()==0`.
   - `host_tls_tren_socket_reactor_tra_loi_ro` (`cfg(windows)`): host TLS đọc trên socket đã
     đăng ký reactor phải lỗi rõ.
   - Mở gate `vs3_default` (4 net + 9 HTTP) và `vs3_tls` (21) sang `cfg(any(unix, windows))`.

## 3. Kết quả test

### 3.1. Linux (branch `vs3/w4-windows`)

| Lệnh | Kết quả |
|---|---|
| `cargo test -p vis-cli --test vs3_default --test vs3_tls --test vs3_http2` | **22 / 21 / 19**, 0 failed |
| `cargo test -p vis-cli --features async-h2 --test vs3_http2` | **29 passed + 1 ignored**, 0 failed |
| `cargo test --workspace --no-fail-fast` | **349 block; 3544 passed / 0 failed / 3 ignored** |
| `./vis-dev gate --fast` | fmt OK, clippy OK; **R1 `vs3_worker.rs` 1926** ⇒ FAIL (có sẵn trên main) |
| `cargo check -p vis-cli --all-targets [--features async-h2]` | PASS cả hai |

### 3.2. `psvn` (Windows MSVC, cargo/rustc 1.100.0, 6 CPU)

Worktree sạch tại `32b750d`; log `/c/Users/ps/vislogs/w4prod-*`.

| Vòng | Lệnh | Kết quả | Thời gian |
|---|---|---|---:|
| R1–R10 | `cargo test -p vis-cli --test vs3_default --test vs3_tls` | mỗi vòng **22/22 + 22/22**, 0 fail | 9–10 s (R1 56 s do build) |
| WS1 | `cargo test --workspace --no-fail-fast` | **3496 / 0 / 3** | 582 s |
| WS2 | như trên | **3495 / 1 / 3** — flake `net.rs::tcp_explain_tong` (`os error 10053`) | 479 s |
| WS3 | như trên | **3496 / 0 / 3** | 501 s |
| WS4 | như trên | **3496 / 0 / 3** | 485 s |
| host | `bin/vis-host-suite` | **54/54** | — |
| targets | `bin/vis-targets` | core **18/18**; sqlite **17 + 1 EXPECTED_RED** (#156), 0 FAIL | — |

- 10 vòng targeted gồm ca fd-leak (200 vòng) và invariant host_poll (Windows) — **xanh mọi vòng**.
- **WS2 flake**: `tcp_explain_tong` chỉ đỏ ở VM, `lỗi đọc từ C2: … os error 10053`
  (WSAECONNABORTED) — race của **test server sync** (`tests/net.rs`), đúng lớp flake Windows đã
  ghi ở `vs3-w4-71d52b4-stress` (trước là 10054); **không liên quan async_net**, ngoài phạm vi W4.
  WS3+WS4 chạy lại liên tiếp xanh ⇒ đủ **2 vòng workspace xanh liên tiếp** sau flake.
- `cargo check --features async-h2` trên Windows: **đỏ** (module `h2_async` Unix-only) — feature
  default-off, không tuyên bố hỗ trợ; xem §6.

## 4. Docs đã cập nhật (đủ điều kiện: 10 vòng + 2 workspace xanh)

- `crates/vis-cli/src/async_net.rs`: đầu module nói **Linux/Unix + Windows** dùng backend;
  async-h2 vẫn Unix-only.
- `crates/vis-cli/tests/vs3_default.rs`: bỏ ghi chú "Windows chưa có backend", nêu backend đã
  production hoá; async-h2 default-off.
- `agent/VS3-TIEN-DO.md`: dòng VS3-1d và luồng W1 ghi async net/TLS/HTTP/1.1 **Unix + Windows**
  (W4), async-h2 vẫn design-only/Unix-only.

## 5. Tệp thay đổi

| Tệp | Thay đổi |
|---|---|
| `crates/vis-cli/src/async_net.rs` | merge h2; bỏ fd registration; `Nguon`; `set_nonblocking`; invariant; `wait` fan-out; 1483 dòng |
| `crates/vis-cli/src/async_net_nguon.rs` | **mới** — `la_would_block`, `Nguon`, `cho_san_sang`, bookkeeping (282 dòng) |
| `crates/vis-cli/src/async_net_h2.rs` | `H2Async` thêm `dang_ky_nguon`/`dang_ky_lai_nguon`/`huy_nguon` (SourceFd nội bộ, Unix) |
| `crates/vis-cli/src/lib.rs`, `main.rs`, `Cargo.toml` | `cfg(any(unix, windows))`, target dep mio |
| `crates/vis-cli/tests/vs3_default.rs`, `vs3_tls.rs` | mở gate Windows + 2 ca mới |
| `agent/VS3-TIEN-DO.md` | trạng thái Windows |

## 6. Phần còn thiếu / nợ (không chặn Windows async net)

1. **R1 `vs3_worker.rs` 1926 dòng** — do W5 thêm 698 dòng **trên main** (`73f63ff`), không phải
   thay đổi của W4. `vis-dev gate --fast` đỏ ở rules trước khi tới L0; `check`/`clippy` xanh.
   Cần W5 tách tệp (hoặc A0 quyết định nới R1 cho test) — W4 không sửa ngoài phạm vi.
2. **Flake sync-net `tcp_explain_tong` (10053)** — test-only Windows, có sẵn; nên áp cùng cách
   `0bd3a64` (shutdown+drain) cho helper `server_http_sau`/`server_n` còn lại.
3. **Windows + `--features async-h2`** — chưa hỗ trợ; h2 async vẫn Unix-only. Muốn port cần
   `KenhTls`/`H2Async` lộ socket hoặc duplicate handle + test 10 vòng riêng. **Không tuyên bố**.
4. Chưa chạy `cargo clippy -D warnings`/fuzz riêng cho Windows (ngoài phạm vi time-box).

## 7. Bằng chứng / log

| Log `psvn` | sha256 |
|---|---|
| `w4prod-10round.log` | `374b0c0a74c0a1598e99939c618f9118e8a09860c865ad6dc2507e9871d959bf` |
| `w4prod-workspace.log` (WS1+WS2) | `685402ad1650f09c713c1f3d5831006eb64ca9af84712d90a8f8fbb60300a143` |
| `w4prod-workspace3.log` | `53d60067d10101e19be25a7eab584035b038f5ef11d98400643689471821d5e5` |
| `w4prod-workspace4.log` | `d96fcbd32da1ac0611cf7e4ceef271273673b4c4b3bf39390d9ecab3cce1e599` |

- Linux: `/tmp/opencode/w4-prod2-linux-workspace.log` (3544/0/3), `w4-prod-gate-final.log`.
- Local `psvn` worktree `32b750d`, dirty 0, `C:` còn **175G**; local `/` còn **17G** (>5G).
- Không tag/push/merge; không GitHub Actions; `main` không bị sửa (`e272007` ở local, origin
  `73f63ff`).

## 8. Kết luận

- **Windows async net/TLS/HTTP/1.1 đã hỗ trợ** ở mức đã đo: 10 vòng targeted + 2 vòng workspace
  xanh liên tiếp (WS3, WS4) + host/targets xanh; fd-leak và invariant host_poll có test riêng.
- **Async-h2 vẫn chưa hỗ trợ trên Windows** (default-off, Unix-only) — không tuyên bố.
- Gate `--fast` chưa xanh vì **R1 có sẵn trên main**, không do W4.
