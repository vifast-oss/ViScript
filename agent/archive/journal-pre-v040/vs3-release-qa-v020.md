<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 QA — v0.2.0 candidate `ec29204` / code `42b700c`

> **Người chạy**: W3 (QA/release), 2026-11-21 16:22–16:30 JST.
> **Phạm vi**: chỉ đo/ghi log; **không sửa code/docs chung**, không đụng `STATE`/`ROADMAP`/
> `VS3-TIEN-DO`, không tag/push/merge.
> **Máy**: Linux `ps`; macOS ngoài phạm vi. **Không dùng GitHub Actions** — toàn bộ đo local.

## 0. Mốc

| Mục | Giá trị |
|---|---|
| `main` | **`ec29204`** (docs v0.2.0 candidate) |
| Code candidate | **`42b700c`** — merge VS3 W5e timeout kênh (VS3-2b), sau 1d/3b |
| Branch QA | `vs3/w3-registry`, merge `main` → merge commit **`5180848`**; cây sản phẩm = `42b700c` |
| Tag `v0.1.0` | vẫn `50675dd`, không đổi |
| Gate kỳ vọng | **3480 passed / 3 skipped** |
| Disk | đầu **6,8G**; thấp nhất **5,5G** (> 5G); sau dọn target QA cũ còn **14G** |
| Toolchain | `~/.cargo/bin/cargo` 1.100.0 |

## 1. Kết quả

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `./vis-dev gate --fast` | **PASS — 3480 passed / 3 skipped** | 169,66 s (nextest 60,00 s) | `/tmp/opencode/qa-w3-v020-gate.log` |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz **14.728.191 ca / 0 vi phạm**; fmt 475 tệp bất động | 120,76 s | `/tmp/opencode/qa-w3-v020-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211625 ./bin/vis-diff` | **PASS — 100.000 ca, lệch 0** | 73,88 s | `/tmp/opencode/qa-w3-v020-diff.log` |
| 4 | `./bin/vis-host-suite` | **PASS — 54/54** + `host_posix_real` PASS | 1,26 s | `/tmp/opencode/qa-w3-v020-host.log` |
| 5 | `./bin/vis-targets` | **PASS** — core **18/18**; sqlite **16 PASS + 2 EXPECTED_RED** (#156) | 9,02 s | `/tmp/opencode/qa-w3-v020-targets.log` |
| 6 | `cargo test -p vis-cli --test vs3_http2 --features http2` (bổ sung, default-off) | **PASS — 5/5 h2** | 6,32 s | `/tmp/opencode/qa-w3-v020-http2.log` |

- **Gate**: `vis-dev check` PASS; nextest **3480/3480 + 3 skipped**, khớp candidate. Các nhóm mới:
  `vs3_default` **7/7**, `vs3_chan_timeout` **17/17**, `vs3_worker` **8/8** (đều pass).
- **Nightly**: fuzz parser 120 s: `14.728.191 ca, 334.229.911 token, 0 vi phạm`; vòng tròn fmt
  `475 tệp bất động, bỏ qua 78 ca lỗi cú pháp`.
- **Diff**: prelude 2.000 ca seed cố định `20261115` lệch 0; 100k seed **mới** `202611211625`
  lệch 0 (93.769 chạy, 6.231 lỗi tĩnh).
- **Host**: 54/54 + bằng chứng POSIX thật (`/bin/echo`, `grep`, `sh`, …) trên Linux.
- **Targets**: core 18/18; sqlite 16 PASS + 2 `EXPECTED_RED` đúng nợ #156 (`ring`/`libsqlite3-sys`
  cần C toolchain khi check chéo msvc/wasm), không có FAIL.
- **HTTP/2 feature**: 5 ca h2 local pass khi bật `--features http2`; đây **không** phải default-on
  (xem §3). Gate mặc định không chạy h2 vì feature default-off.

## 2. Checklist v0.2.0

### 2.1 VS3-1d — async default + Pipe parity

| # | Hạng mục | Bằng chứng QA | Trạng thái |
|---|---|---|---|
| 1d.1 | Chọn frame runtime khi có `async fn` | `vs3_default` 7/7; gate 3480/3 | **PASS (local)** |
| 1d.2 | Sync byte-identical, `chan(0)` vẫn từ chối | `sync_khong_doi_chan0_van_tu_choi`, `cli_async_chan0_va_sync_byte_identical` | **PASS** |
| 1d.3 | `Pipe` parity | `pipe_parity_default_va_async` PASS | **PASS** |
| 1d.4 | Guard scheduler không treo | `guard_scheduler_trap_ro_khong_treo` PASS | **PASS** |
| 1d.5 | Cache/phiên bản + không hồi quy | gate + nightly + diff 100k | **PASS (local)** |
| — | Tàn dư A0 ghi | A0 vẫn ghi “cap>0/net async W3” là phần mở; xác nhận trước khi tag | **Cần A0 chốt** |

### 2.2 VS3-2b — timeout kênh

| # | Hạng mục | Bằng chứng QA | Trạng thái |
|---|---|---|---|
| 2b.1 | `recv_timeout(ms) -> T?` (None = timeout/closed) | `vs3_chan_timeout` 17/17 | **PASS (local)** |
| 2b.2 | `send_timeout(ms, v) -> bool` (false = timeout/closed) | `vs3_chan_timeout` 17/17 | **PASS (local)** |
| 2b.3 | `select` hoãn VS3-2c | Không có API `select` mới trong candidate | **Đúng phạm vi** |
| 2b.4 | Ca biên (0/n, đóng, nhiều waiter, không treo) | 17 ca timeout/close/deadlock | **PASS** |

### 2.3 VS3-3b — worker opt-in

| # | Hạng mục | Bằng chứng QA | Trạng thái |
|---|---|---|---|
| 3b.1 | Opt-in/test-only, default single-thread | `vs3_worker` 8/8; default không đổi | **PASS (local)** |
| 3b.2 | `TaskId`/proxy, không đổi ABI | `value_task_show_va_taskid_khong_doi_abi` | **PASS** |
| 3b.3 | Snapshot W>1 + từ chối ghi chéo | `w1_shared_env_va_wn_snapshot`, `cross_write_w1_allowed_wn_refused` | **PASS** |
| 3b.4 | Print merge spawn order | `merge_spawn_order_khi_completion_lech`, `w1_w2_w4_cung_output_byte_identical` | **PASS** |
| 3b.5 | Budget atomic/chunk + không deadlock/cancel | 3 ca budget/cancel PASS | **PASS** |
| 3b.6 | “Bản đầu” cho v0.2.0 | Pool opt-in có; A0 ghi **chưa nối evaluator** | **Một phần — A0 quyết tag** |

### 2.4 Điều kiện tag v0.2.0

- [x] Gate local **3480/3**, nightly PASS, diff 100k lệch 0, host 54/54, core targets 18/18.
- [x] 1d/2b có test trong gate; 3b opt-in có test, default không đổi.
- [ ] **Windows psvn** verify tại `42b700c`/`ec29204` (QA này không chạy; W4 sở hữu).
- [ ] **Fresh-clone** tại commit tag — phiên này không chạy (đĩa thấp); lần trước `v0.1.0` PASS.
- [ ] A0 chốt tàn dư 1d (`cap>0`/net async) và 3b “chưa nối evaluator”.
- [ ] A0 tạo/push tag; W3 không tự tag.

## 3. Checklist v0.3.0

### 3.1 HTTP/2

| # | Hạng mục | Bằng chứng / trạng thái |
|---|---|---|
| 4b2.1 | `h2` review license/unsafe/transitive | W2 có báo cáo `BAO-CAO-VS3-4B2-H2-IMPLEMENT`; chưa có bảng review độc lập trong QA này |
| 4b2.2 | Feature default-off, HTTP/1.1 mặc định | `http2` default-off trong `Cargo.toml`; gate mặc định không chạy h2 |
| 4b2.3 | Test h2 local | **PASS 5/5** với `--features http2`; chưa kiểm trên Windows |
| 4b2.4 | **Default-on** (v0.3.0) | **CHƯA**: thử `959d1aa` (+7,8% release) chưa merge; chỉ sau tag v0.2.0 + W4 verify h2 Windows |
| 4b2.5 | Không hồi quy HTTP/1.1/TLS/registry | gate 3480/3 + `vs3_tls`/keepalive/registry PASS |
| 4b2.6 | Fallback nếu h2 không qua | Ghi rõ, giữ HTTP/1.1, HTTP/2 → `v0.3.1` (theo VD-271 §5) |

### 3.2 Windows verify (local/psvn, không GitHub Actions)

- [ ] W4 chạy `cargo test --workspace --no-fail-fast` + `vis-dev check` trên psvn tại candidate
  `42b700c`/`ec29204`; **0 product failure**.
- [ ] `vis-targets` psvn: core 18/18; sqlite 17/1 (theo W4) và không thêm ô đỏ ngoài #156.
- [ ] Verify h2 (khi default-on) trên Windows; nếu chưa, giữ default-off cho v0.3.0.
- [ ] Không tuyên bố Windows Tier 1; macOS ngoài phạm vi.

### 3.3 Net/package ổn định + tag

- [x] Gate local có TLS, keep-alive, cookie/auth, registry HTTPS, Ed25519/GC (các suite tương ứng pass).
- [ ] Chạy lại `vs3_http2 --features http2` trên Windows/CI local nếu default-on.
- [ ] Gate/nightly/diff/host/targets + fresh-clone tại commit tag `v0.3.0`; release notes/CHANGELOG.
- [ ] A0 tag/push; không di chuyển `v0.1.0`/`v0.2.0`.

## 4. Blocker / rủi ro

1. **Windows v0.2.0 chưa verify trong phiên này** — cần W4 psvn tại `42b700c` trước khi A0 tag.
2. **3b “chưa nối evaluator”** và **1d tàn dư cap>0/net async** — A0 phải chốt có đủ điều kiện tag
   v0.2.0 hay không.
3. **HTTP/2 default-on chưa merge** (`959d1aa` chỉ thử); chỉ mở sau tag v0.2.0 + Windows h2.
4. **Đĩa thấp**: low-water 5,5G (ngưỡng dừng 5G). Đã xoá target QA cũ 8,2G, còn 14G; nếu chạy
   fresh-clone/full Windows cần dọn thêm.
5. **2 ô EXPECTED_RED** msvc/wasm do #156 C toolchain — không đổi nhãn PASS.
6. **Không GitHub Actions**: mọi kết quả trên là local Linux; không dùng workflow làm bằng chứng.
7. **Fresh-clone v0.2.0 chưa chạy** — cần trước tag (phiên trước `v0.1.0` fresh-clone PASS, không
   thay thế cho candidate mới).

## 5. Lệnh tái lập

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=/home/ps/ViScript/target

./vis-dev gate --fast
./vis-dev nightly --skip-gate --fuzz-minutes 2
VIS_DIFF_N=100000 VIS_DIFF_SEED=202611211625 ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets
cargo test -p vis-cli --test vs3_http2 --features http2   # default-off, kiểm h2 local
```

**Kết luận**: candidate `42b700c` (`ec29204`) **xanh local Linux**: gate **3480/3**, nightly PASS,
diff 100k lệch 0, host 54/54, core targets 18/18, h2 feature 5/5. Chưa đủ để W3 tự tag v0.2.0:
còn Windows psvn, fresh-clone và A0 chốt tàn dư 1d/3b. v0.3.0 **chưa** default-on HTTP/2.
