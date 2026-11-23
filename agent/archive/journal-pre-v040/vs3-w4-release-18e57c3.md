<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify release `v0.1.0` @ `18e57c3` (Windows psvn)

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Commit A0 đề xuất tag**: `18e57c3ff9c371eaf42346176ea1d07b84f7bf88`
> (`[VS3][W2] docs: loopback fix merged, gate 3448, cho W4 verify`).
> **Kết quả**: exact `18e57c3` **đỏ 1 ca test-only**; candidate `777af63` (RC + fix harness
> cookie) **xanh hoàn toàn**. Đề xuất A0 đưa fix test-only này vào trước khi tag.

## 0. TL;DR

| Mốc | `cargo test --workspace` | `vis-dev check` | Host suite | Exit |
|---|---|---|---|---|
| **`18e57c3`** (đúng commit đề xuất tag) | 344 block; **3403 passed / 1 failed** / 3 ignored | **PASS** | **54/54** | 101 |
| **`777af63`** (`18e57c3` + fix test-only) | 344/344 block; **3404 passed / 0 failed** / 3 ignored | **PASS** | **54/54** | **0** |

Ca đỏ duy nhất của `18e57c3`: `vs3_cookie_auth::set_cookie_qua_redirect_duoc_gui_lai` — **test-only**
(server test Windows thừa hưởng non-blocking của listener). Không có lỗi sản phẩm trên RC này.

## 1. Exact `18e57c3` — kết quả

- Checkout đúng `18e57c3` trên `psvn`, cây sạch (`git status --porcelain` = 0).
- `cargo test --workspace --no-fail-fast`: **344 block; 3403 passed / 1 failed / 3 ignored**;
  1 target đỏ: `-p vis-cli --test vs3_cookie_auth`.
- `bin/vis-dev check`: **PASS** toàn bộ 13 bước (fmt, clippy `-D warnings`, R1..R6, toolchain,
  release, self-test, explain, report, build, doc/readme examples).
- `bin/vis-host-suite`: **54/54 PASS** (`host_posix_real` bỏ qua — Windows dùng `test-child`).

### Ca đỏ — test-only

```text
vs3_cookie_auth::set_cookie_qua_redirect_duoc_gui_lai
server đọc path "/" thay vì "/a"
client: lỗi đọc từ `C1`: ... (os error 10053)
```

Nguyên nhân: `TcpListener` test đặt non-blocking; trên Windows socket do `accept` trả về **thừa
hưởng non-blocking**, nên server đọc request rỗng/không đủ và trả sai route; client thấy 10053.
Đây là cùng lớp lỗi đã sửa ở `vs3_registry`/`vs3_registry_https`; không phải lỗi sản phẩm.

## 2. Candidate `777af63` — xanh

- `d77b7f4` — merge `main` `18e57c3` vào `vs3/w4-windows`.
- `777af63` — `[VS3][W4-fix] cookie auth test Windows: accepted socket blocking (het 10053)`:
  `set_nonblocking(false)`, read timeout 5 s, đóng chiều ghi + drain trước khi drop.
- Stress `vs3_cookie_auth` **10 vòng: 0 fail**.
- Full suite: **344/344 block; 3404 passed / 0 failed / 3 ignored; exit 0**.
- `vis-dev check`: **PASS**; `vis-host-suite`: **54/54**.

### Các mảng release được xác nhận trên `777af63`

| Mảng | Kết quả |
|---|---|
| `vs3_tls` (rustls/ring, IPv6, cert) | 12/12 |
| `vs3_keepalive` | 8/8 |
| `vs3_registry_sig` (Ed25519/`ring`) | 10/10 |
| `vs3_registry_https` | 6/6 |
| `vs3_chan0` | 13/13 |
| `lsp_incremental` (UTF-16/CRLF) | 4/4 |
| `vs3_cookie_auth` | 11/11 |
| `vs3_coverage_matrix` | 10/10 (trên `18e57c3`) |

Ba lỗi trước đây đã được xác nhận sửa trong `18e57c3`: overlay LSP (`3d6bfe4`), registry test
(`5dec89b`), `net.rs` loopback `localhost` (`d9046a9`).

## 3. BLOCKED-ENV cross-target (giữ nguyên)

- `psvn` chỉ cài `x86_64-pc-windows-msvc`; thiếu `wasm32-unknown-unknown` và
  `x86_64-unknown-linux-gnu` ⇒ `bin/vis-targets` không đo được (exit 1).
- Test `targets_wasm_vis_types` tự báo BLOCKED-ENV và skip — **không** phải bằng chứng xanh.
- Không ảnh hưởng kết luận host Windows; vẫn là caveat khi tuyên bố Tier 2.

## 4. Kết luận / đề xuất

- **Sản phẩm trên `18e57c3` xanh**: 0 lỗi product; chỉ còn 1 nợ test-only ở harness cookie.
- **Đề xuất A0**: đưa commit test-only `777af63` (chỉ 1 tệp `crates/vis-cli/tests/vs3_cookie_auth.rs`)
  vào `18e57c3` — cherry-pick hoặc merge — rồi tag `v0.1.0` tại commit kết quả (hoặc tag chính
  `777af63` nếu lịch sử nhánh được chấp nhận). Khi đó Windows xanh **0 failed**.
- Nếu A0 bắt buộc tag đúng `18e57c3`: phải ghi rõ Windows có **1 ca test-only đỏ** (flaky harness),
  không tuyên bố “Windows workspace 0 failed”; sản phẩm vẫn không có lỗi.
- Không tag/push trong lần này.

## 5. Log

| Lệnh | Log | sha256 | Exit |
|---|---|---|---|
| `cargo test` @ `18e57c3` | `/tmp/opencode/psvn-rc-cargo.log` | `9c79d7d6…f5f9e` | **101** |
| gate + host @ `18e57c3` | `/tmp/opencode/psvn-rc-exact-gate.log` | `5e38f5c5…d1b2` | 0 |
| stress cookie 10 vòng @ `777af63` | `/tmp/opencode/psvn-rc2-stress.log` | `5d84d8f6…0777c` | 0 |
| `cargo test` @ `777af63` | `/tmp/opencode/psvn-rc2-cargo.log` | `df584325…4457` | **0** |
| `vis-dev check` @ `777af63` | `/tmp/opencode/psvn-rc2-visdev-check.log` | `8e604288…4b13` | 0 |
| `vis-host-suite` @ `777af63` | `/tmp/opencode/psvn-rc2-host-suite.log` | `07f1903c…4f0a` | 0 |
| trạng thái cuối @ `18e57c3` | `/tmp/opencode/psvn-rc-final-state.log` | — | 0 |

Log máy đích: `vis-remote-20261121T045401Z.log` (cargo exact), `…T050936Z.log` (gate/host exact),
`…T050118Z.log` (cargo candidate), `…T050815Z.log` (check candidate), `…T050918Z.log` (host candidate),
`…T051044Z.log` (trạng thái cuối).

## 6. Tái lập

```bash
# exact RC
git bundle create /tmp/opencode/w4rc.bundle 041ee50..origin/main
scp /tmp/opencode/w4rc.bundle psvn:C:/Users/ps/vislogs/w4rc.bundle
bin/vis-remote psvn -- bash -lc \
  'cd /c/Users/ps/viscript-moi && git fetch /c/Users/ps/vislogs/w4rc.bundle refs/remotes/origin/main:refs/heads/w4rc-test && git switch --detach w4rc-test'
bin/vis-remote psvn --log /tmp/opencode/psvn-rc-cargo.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && cargo test --workspace --no-fail-fast'

# candidate có fix test-only
git bundle create /tmp/opencode/w4rc2.bundle 18e57c3..vs3/w4-windows
scp /tmp/opencode/w4rc2.bundle psvn:C:/Users/ps/vislogs/w4rc2.bundle
bin/vis-remote psvn -- bash -lc \
  'cd /c/Users/ps/viscript-moi && git fetch /c/Users/ps/vislogs/w4rc2.bundle refs/heads/vs3/w4-windows:refs/heads/w4rc2-test && git switch --detach w4rc2-test'
bin/vis-remote psvn --log /tmp/opencode/psvn-rc2-cargo.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && cargo test --workspace --no-fail-fast'
```
