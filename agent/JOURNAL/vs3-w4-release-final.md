<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify release `v0.1.0` @ `041ee50` (Windows psvn)

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Commit verify**: `041ee50ee96711fe4127a0344413159d064c1f08` — `[VS3] docs: W1c coverage merged,
> gate 3445 pass` (code `ccecc97`), checkout **đúng** trên `psvn`, cây sạch.
> **Kết luận**: **KHÔNG xanh — chưa nên tag `v0.1.0` tại `041ee50` nếu yêu cầu Windows Tier 2.**
> 5 ca đỏ / 3 target, trong đó **2 lỗi sản phẩm** (một lỗi thuộc `net.rs` — ngoài phạm vi W4).

## 0. TL;DR

| Lệnh @ `041ee50` | Kết quả | Exit |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **344 block; 3396 passed / 5 failed / 3 ignored; 3 target đỏ** | **101** |
| `bin/vis-dev check` | **PASS** (fmt, clippy, R1..R6, toolchain, release, self-test, explain, report, build, doc/readme examples) | 0 |
| `bin/vis-host-suite` | **54/54 PASS** (`host_posix_real` bỏ qua — Windows dùng `test-child`) | 0 |
| `git status --porcelain` cuối | 0 mục; HEAD `041ee50` | 0 |

**Lưu ý**: `vis-dev check` xanh **không** chứng minh workspace xanh — cổng này không chạy toàn bộ
`cargo test`; 5 ca đỏ nằm ngoài phạm vi cổng.

## 1. 5 ca đỏ — phân loại

| # | Target | Ca | Thông điệp chính | Lớp | Fix hiện có? |
|---:|---|---|---|---|---|
| 1 | `vis-cli --bin vis` | `modules::tests_overlay::buffer_mo_thang_dia_va_close_tra_lai_dia` | `left: "dia\n"`, `right: "buffer\n"` — overlay LSP bị bỏ qua, đọc đĩa | **product** | Có trên nhánh W4: `5f85240`, **chưa có trong `041ee50`** |
| 2 | `vs3_cookie_auth` | `cookie_path_va_cross_host` | `localhost:…` ⇒ `No connection could be made … (os error 10061)` | **product/portability** (`net.rs`) | **Chưa** — `net.rs` ngoài phạm vi W4 |
| 3 | `vs3_registry_https` | `add_qua_https_roi_cai_dung` | TLS ghi `C1`: `os error 10053` | test-only | Có trên nhánh W4: `64e3512`, **chưa có trong `041ee50`** |
| 4 | `vs3_registry_https` | `add_https_co_chu_ky` | TLS ghi `C1`: `os error 10053` | test-only | như trên |
| 5 | `vs3_registry_https` | `https_khong_root_test_loi_ro` | TLS handshake: `os error 10053` | test-only | như trên |

### 1.1. Product — overlay LSP sai khoá trên Windows (deterministic)

- `dat_noi_dung_mo` lưu buffer theo `chuan_hoa(path)` (canonical, Windows thêm `\\?\`), nhưng
  `doc_cache_phu` tra bằng **đường dẫn thô** ⇒ trượt overlay, đọc nội dung đĩa.
- Đây là hành vi sản phẩm: LSP `didOpen`/`didChange` (buffer chưa ghi đĩa) bị bỏ qua trên Windows.
- Fix đã có trên nhánh W4: `5f85240` (tra thô + canonical, xoá cả hai khoá) — **không** nằm trong
  `041ee50` (`git merge-base --is-ancestor 5f85240 041ee50` = false).

### 1.2. Product/portability — `localhost` trên Windows (`net.rs`)

- `crates/vis-cli/src/net.rs::chi_loopback` chỉ lấy **địa chỉ đầu tiên** từ `to_socket_addrs`.
  Trên Windows, `localhost` thường phân giải `::1` trước; test server bind `127.0.0.1` ⇒
  `connect_timeout(::1)` bị từ chối (10061).
- Test `vs3_cookie_auth::cookie_path_va_cross_host` là ca hợp lệ (kiểm cookie theo host
  `127.0.0.1` vs `localhost`); **không** sửa test để né lỗi.
- **Không sửa** vì `net.rs` thuộc luồng khác (lệnh cấm). Cần chủ `net.rs` sửa: thử **lần lượt**
  các địa chỉ phân giải (hoặc ưu tiên IPv4 cho loopback) thay vì chỉ `ds.next()`.

### 1.3. Test-only — registry HTTPS server (đã có fix ngoài main)

- Socket do `accept` trả về **thừa hưởng non-blocking** trên Windows ⇒ TLS handshake/read trả
  `WouldBlock`/10053. Fix `64e3512` (set blocking + close êm) đã stress 10/10 trên nhánh W4,
  nhưng **không** nằm trong `041ee50`.

## 2. Các mảng được yêu cầu — trạng thái @ `041ee50`

| Mảng | Kết quả |
|---|---|
| `vs3_tls` (rustls/ring, IPv6 loopback, cert) | **12/12 PASS** |
| `vs3_keepalive` (keep-alive/IPv6) | **8/8 PASS** |
| `vs3_registry_sig` (Ed25519/`ring`) | **10/10 PASS** |
| `vs3_chan0` | **13/13 PASS** |
| `lsp_incremental` (UTF-16/CRLF) | **4/4 PASS** |
| `vs3_coverage_matrix` | **10/10 PASS** |
| `vs3_registry_https` | **3/6 FAIL** (test-only, xem §1.3) |
| `vs3_cookie_auth` | **8/9 FAIL** (product `net.rs`, xem §1.2) |

`ring`/`rustls` build và chạy thật trên Windows; **không** BLOCKED-ENV C toolchain.

## 3. BLOCKED-ENV cross-target (giữ nguyên)

- `psvn` chỉ cài `x86_64-pc-windows-msvc`; thiếu `wasm32-unknown-unknown` và
  `x86_64-unknown-linux-gnu` ⇒ `bin/vis-targets` không đo được (exit 1).
- Test `targets_wasm_vis_types` tự báo BLOCKED-ENV và skip (không giả xanh).
- Đây là caveat của mọi lần verify Windows, không phải lỗi của `041ee50`.

## 4. Kết luận và đề xuất

**Không tag `v0.1.0` tại `041ee50` dưới nhãn “Windows Tier 2 verified”.** Có 2 lỗi sản phẩm thật:

1. Overlay LSP Windows (fix sẵn `5f85240`).
2. `localhost` loopback trên Windows trong `net.rs` (chưa fix; cần chủ `net.rs`).

Cộng thêm 1 nợ test-only (`vs3_registry_https`, fix sẵn `64e3512`).

**Đề xuất cho A0** (feature freeze — A0 quyết):

- **Ưu tiên**: cho phép cherry-pick/merge `5f85240` + `64e3512` và một fix `net.rs` (thử lần lượt
  địa chỉ phân giải / ưu tiên IPv4 loopback); sau đó W4 chạy lại đúng quy trình trên commit mới.
- Hoặc: tag `041ee50` nhưng ghi rõ **Windows chưa verify xanh** (5 ca đỏ: 2 product + 3 test-only),
  không tuyên bố Tier 2.
- **Không** sửa test `vs3_cookie_auth` để né lỗi `localhost`; ca này phản ánh hành vi sản phẩm.

## 5. Log

| Lệnh | Log cục bộ | sha256 | Exit |
|---|---|---|---|
| `cargo test` @ `041ee50` | `/tmp/opencode/psvn-final-cargo.log` | `e2dde24e…a1b1` | **101** |
| `vis-dev check` @ `041ee50` | `/tmp/opencode/psvn-final-visdev-check.log` | `f8c6d3d3…ad22` | 0 |
| `vis-host-suite` @ `041ee50` | `/tmp/opencode/psvn-final-host-suite.log` | `9afeac74…8bee` | 0 |
| trạng thái cuối | `/tmp/opencode/psvn-final-state.log` | — | 0 |
| (tham chiếu) `cargo test` @ nhánh W4 `64e3512` | `/tmp/opencode/psvn-release2-cargo.log` | `37ec7143…4406` | 0 (341 block) |

Log máy đích: `vis-remote-20261121T043038Z.log` (cargo @ `041ee50`), `…T043818Z.log` (check),
`…T043918Z.log` (host), `…T043951Z.log` (trạng thái cuối).

## 6. Tái lập

```bash
git bundle create /tmp/opencode/w4final.bundle f71fba0..origin/main
scp /tmp/opencode/w4final.bundle psvn:C:/Users/ps/vislogs/w4final.bundle
bin/vis-remote psvn -- bash -lc \
  'cd /c/Users/ps/viscript-moi && git fetch /c/Users/ps/vislogs/w4final.bundle refs/remotes/origin/main:refs/heads/w4final-test && git switch --detach w4final-test'

bin/vis-remote psvn --log /tmp/opencode/psvn-final-cargo.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && cargo test --workspace --no-fail-fast'
bin/vis-remote psvn --log /tmp/opencode/psvn-final-visdev-check.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-dev check'
bin/vis-remote psvn --log /tmp/opencode/psvn-final-host-suite.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-host-suite'
```
