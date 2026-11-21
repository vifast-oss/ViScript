<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify lại Windows sau merge W1b/W2/W2b/W3b/W5

> **Người đo**: subagent VS3-W4 (`psvn`), 2026-11-21.
> **Mốc verify cuối**: `281be5d` — merge `main` `2d6ca2b` vào `vs3/w4-windows` + một fix **test-only**
> cho registry server Windows.
> **Kết luận**: **xanh — đề xuất promote `v0.1.0` từ `281be5d`**. Không tạo tag, không push.

## 0. TL;DR — các mốc

| Mốc | Nội dung | `cargo test --workspace` | `vis-dev check` | Host suite |
|---|---|---|---|---|
| `ed30ebb` (báo cáo trước) | W4-fix trên main cũ | 3258 passed / 0 failed (329 block) | PASS | 54/54 |
| `bf7b398` | merge `edbb313` (W1b cancellation, W2 TLS, W3b Ed25519/GC) | **3324 / 0** (337 block) | (không chạy) | (không chạy) |
| `4b95160` | merge `2d6ca2b` (W2b keep-alive/IPv6, W5) | 3339 / **1 failed** (338 block) | — | — |
| **`281be5d`** | `4b95160` + fix test registry Windows | **3340 / 0** (338 block) | **PASS** | **54/54** |

## 1. Chuỗi merge và commit

- `bf7b398` — `Merge branch 'main' into vs3/w4-windows` (`edbb313`).
- `4b95160` — `Merge branch 'main' into vs3/w4-windows` (`2d6ca2b`; gồm `b730661` W2b
  keep-alive/IPv6 và `963df16` W5).
- `281be5d` — `[VS3][W4-fix] registry test Windows: accepted socket blocking + close em`
  (chỉ 2 tệp test, +24/−4). **Đây là commit đã verify và đề xuất tag.**

Không đụng `net.rs`, `pkg.rs`, `frame/**` trong phần W4 tự sửa; các thay đổi ở các tệp đó đến từ
merge của main.

## 2. Kết quả cuối @ `281be5d` (Windows thật, `psvn`)

| Hạng mục | Kết quả |
|---|---|
| `cargo test --workspace --no-fail-fast` | **338/338 block xanh; 3340 passed / 0 failed / 3 ignored; exit 0** |
| `bin/vis-dev check` | **PASS** — fmt, clippy `-D warnings`, R1..R6, toolchain, release, self-test, explain, report, build, doc-examples, readme-examples (20/20) |
| `bin/vis-host-suite` | **54/54 PASS**; `host_posix_real` bỏ qua (Windows dùng `test-child`) |
| Cây `psvn` sau đo | `281be5d`, `git status --porcelain` = 0 mục |

### Xác nhận đặc biệt theo yêu cầu

| Mảng | Bằng chứng |
|---|---|
| **rustls/ring/TLS** | `ring v0.17.14` + `rustls v0.23.46` **biên dịch được** trên Windows (`psvn-merge2-cargo.log` dòng 50–51); `vs3_tls` **12/12 PASS**, gồm `https_ipv6_loopback`, hai request cùng kết nối, từ chối cert tự ký/sai tên, lỗi TLS không panic |
| **Keep-alive / IPv6** | `vs3_keepalive` **8/8 PASS**; TLS IPv6 loopback PASS |
| **pkg Ed25519 (`ring`)** | `vs3_registry_sig` **9/9 PASS** (ký đúng cài, sai/tamper/thiếu/sai định dạng bị từ chối); `vs3_registry` **11/11**; `vs3_pkg_gc` **5/5** |
| **C toolchain** | Không BLOCKED-ENV: `rusqlite` bundled + `ring` build OK (cc tự tìm VS 2022; `lib.exe`/`cl.exe` không có trong PATH nhưng không cần). |

## 3. Ca đỏ trung gian @ `4b95160` — test-only, đã sửa

- Lần chạy đầu tại `4b95160`: `vs3_registry_sig::chu_ky_sai_dinh_dang` đỏ với
  `lỗi đọc từ C1: ... (os error 10053)` (WSAECONNABORTED).
- Chạy lặp binary: tái hiện **1/3**; sau đó `vs3_registry::index_hong_loi_ro` nhận **HTTP 404** thay
  vì index hỏng — server test trả lời sai vì đọc request rỗng.
- **Root cause**: listener test đặt non-blocking; trên Windows socket do `accept` trả về **thừa
  hưởng non-blocking**, nên `read` trả `WouldBlock` ngay khi request chưa tới ⇒ server trả 404/đóng
  sớm ⇒ client thấy 404 hoặc 10053. Linux không gặp vì socket accepted là blocking.
- **Sửa (test-only)**: `set_nonblocking(false)` cho socket accepted; read timeout 5 s; sau khi ghi
  response thì `shutdown(Write)` + đọc nốt tới EOF/timeout ngắn thay vì `shutdown(Both)`.
  Áp cho `vs3_registry.rs` và `vs3_registry_sig.rs`.
- **Kiểm tra**: 10 vòng mỗi binary (`vs3_registry`, `vs3_registry_sig`) đều **0 fail**; sau đó full
  workspace xanh. Không đổi mã sản phẩm, không nới assertion.

## 4. BLOCKED-ENV (không phải lỗi sản phẩm)

- `bin/vis-targets` vẫn exit 1 vì `psvn` **chỉ cài** `x86_64-pc-windows-msvc` (thiếu
  `wasm32-unknown-unknown`, `x86_64-unknown-linux-gnu`). Không cài theo lệnh.
- Test `targets_wasm_vis_types` trong full suite tự dò sysroot, in BLOCKED-ENV và bỏ qua
  (`1 passed`, không giả xanh). Đây là caveat duy nhất khi tuyên bố Windows Tier 2.

## 5. Kết luận

**Đề xuất promote `v0.1.0` từ `281be5d`.** Windows Tier 2 được verify lại trên code mới nhất:

1. Workspace Windows **0 đỏ** (3340 test pass; 338/338 block).
2. Cổng `vis-dev check` **PASS**, gồm clippy `-D warnings` và readme/doc examples.
3. Host suite **54/54**.
4. TLS/rustls/ring + keep-alive/IPv6 + Ed25519 registry/GC đều build và test thật trên Windows.
5. Fix flaky registry chỉ là test harness; sản phẩm không đổi.

**Caveat phải ghi khi cắt tag**: ma trận cross-target `vis-targets` chưa đo trên `psvn`
(BLOCKED-ENV do thiếu std target). Nếu A0 yêu cầu ma trận là điều kiện promote, cần cài target trên
`psvn` rồi chạy lại; không coi skip BLOCKED-ENV là bằng chứng xanh.

## 6. Log

| Lệnh | Log cục bộ | sha256 | Exit |
|---|---|---|---|
| `cargo test` @ `bf7b398` | `/tmp/opencode/psvn-merge-cargo.log` | — | 0 |
| `cargo test` @ `4b95160` (1 đỏ) | `/tmp/opencode/psvn-merge2-cargo.log` | — | 101 |
| stress registry 10 vòng @ `281be5d` | `/tmp/opencode/psvn-merge4-stress.log` | `d6284ab4…d4fe` | 0 |
| **`cargo test` @ `281be5d`** | `/tmp/opencode/psvn-merge4-cargo.log` | `ac9d4427…2c9e` | **0** |
| **`vis-dev check` @ `281be5d`** | `/tmp/opencode/psvn-merge4-visdev-check.log` | `ae56badb…5433` | **0** |
| **`vis-host-suite` @ `281be5d`** | `/tmp/opencode/psvn-merge4-host-suite.log` | `bb5586c2…b52e` | **0** |
| `bin/vis-targets` @ `281be5d` (BLOCKED-ENV) | `/tmp/opencode/psvn-merge4-targets.log` | — | 1 |
| trạng thái cuối | `/tmp/opencode/psvn-merge4-final-state.log` | — | 0 |

Log máy đích (mới nhất): `vis-remote-20261121T035900Z.log` (cargo final),
`…T040523Z.log` (check), `…T040650Z.log` (host), `…T040739Z.log` (vis-targets),
`…T040707Z.log` (trạng thái cuối).

## 7. Tái lập (không push)

```bash
# bundle từ 0985030 → nhánh W4 (psvn đã có 0985030)
git bundle create /tmp/opencode/w4merge.bundle 0985030..vs3/w4-windows
scp /tmp/opencode/w4merge.bundle psvn:C:/Users/ps/vislogs/w4merge.bundle
bin/vis-remote psvn -- bash -lc \
  'cd /c/Users/ps/viscript-moi && git fetch /c/Users/ps/vislogs/w4merge.bundle refs/heads/vs3/w4-windows:refs/heads/w4merge-test && git switch --detach w4merge-test'

bin/vis-remote psvn --log /tmp/opencode/psvn-merge4-cargo.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && cargo test --workspace --no-fail-fast'
bin/vis-remote psvn --log /tmp/opencode/psvn-merge4-visdev-check.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-dev check'
bin/vis-remote psvn --log /tmp/opencode/psvn-merge4-host-suite.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-host-suite'
```
