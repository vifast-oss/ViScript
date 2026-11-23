<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3 release/QA — baseline `v0.1.0` / main → checklist `v0.2.0`, `v0.3.0`

> **Người chạy**: W3 (release/QA), 2026-11-21 15:40–15:55 JST.
> **Phạm vi**: chỉ đo/ghi log; **không sửa code sản phẩm**, không đụng `STATE.md`/`ROADMAP.md`/
> `VS3-TIEN-DO.md`, không tag/push/merge.
> **Máy**: Linux `ps`; macOS **ngoài phạm vi**; Windows chỉ đọc kết quả lane W4, không tuyên bố mới.

## 0. Mốc và môi trường

| Mục | Giá trị |
|---|---|
| Branch | `vs3/w3-registry` (worktree `.../vs3-w3-registry`) |
| HEAD QA | `f710e5b` — **cùng cây** với `main` (`git diff --quiet HEAD main` = 0) |
| Merge trước QA | `git merge main` → fast-forward `f710e5b` (main đã gồm `77b71fb` + các merge W1/W2/W4) |
| Tag baseline | `v0.1.0` (annotated object `e43d9f9`) → commit **`50675dd`**; `v0.1.0-rc1` giữ nguyên |
| Gate baseline kỳ vọng | **3448 passed / 3 skipped** (theo tag `v0.1.0`) |
| Disk đầu/cuối | **21G → 16G** (`df -h /home`); không lần nào < 5G |
| Cargo | `~/.cargo/bin/cargo` 1.100.0 (đúng `rust-toolchain.toml`) |
| Target dùng | `/home/ps/ViScript/target` (dùng lại artifact main cùng cây); fresh-clone dùng target riêng |

> **Ghi chú tuần tự**: gate → nightly → diff → host → targets chạy lần lượt, không chạy song song
> trong cùng một lệnh. Lúc fresh-clone, `pgrep` ghi nhận lane W1 và main có build cục bộ đang chạy
> (không phải do QA khởi động); fresh-clone vẫn PASS nhưng thời gian 299 s có thể bị ảnh hưởng.

## 1. Kết quả baseline

| # | Lệnh | Kết quả | Thời gian | Log |
|---|---|---|---|---|
| 1 | `CARGO_TARGET_DIR=/home/ps/ViScript/target ./vis-dev gate --fast` | **PASS** — 3448 passed / 3 skipped | 140,98 s (nextest 29,25 s) | `/tmp/opencode/qa-w3-gate.log` |
| 2 | `./vis-dev nightly --skip-gate --fuzz-minutes 2` | **PASS** — fuzz 18.398.553 ca / 0 vi phạm; fmt 475 tệp bất động | 120,75 s | `/tmp/opencode/qa-w3-nightly.log` |
| 3 | `VIS_DIFF_N=100000 VIS_DIFF_SEED=20261121 ./bin/vis-diff` | **PASS** — 100.000 chương trình, **lech=0**; 93.770 chạy, 6.230 lỗi tĩnh | 75,68 s | `/tmp/opencode/qa-w3-diff100k.log` |
| 4 | `./bin/vis-host-suite` | **PASS** — 54/54 ca + `host_posix_real` PASS (Linux x86_64) | 1,65 s | `/tmp/opencode/qa-w3-host.log` |
| 5 | `./bin/vis-targets` | **PASS** — core 18/18; sqlite 16 PASS + **2 EXPECTED_RED** (`vis-cli` × msvc/wasm, #156) | 37,44 s | `/tmp/opencode/qa-w3-targets.log` |
| 6 | Fresh clone `v0.1.0` (`50675dd`) + `./vis-dev gate --fast` | **PASS** — 3448 passed / 3 skipped | 299,44 s | `/tmp/opencode/qa-w3-clone-v010b.log` |

### 1.1 Chi tiết

- **Gate**: `vis-dev check` PASS (fmt/clippy/R1..R6/self-test/build/doc-examples), L0 nextest
  **3448/3448 + 3 skipped**; đúng số tag `v0.1.0`, không hồi quy.
- **Nightly**: dùng `--skip-gate` vì gate vừa chạy ngay trước (ST-257); fuzz parser 120 s:
  `18.398.553 ca, 417.555.438 token, 0 vi phạm`; vòng tròn fmt `475 tệp bất động, bỏ qua 78 ca lỗi cú pháp`.
- **Diff**: seed **mới** `20261121` (khác các log cũ `20261115`); prelude 2.000 ca seed cố định
  `20261115` vẫn `lech=0`; 100k `lech=0`, tốc độ 1.322 chương trình/s.
- **Host**: đủ 54 ca `PASS`; thêm bằng chứng POSIX thật (`/bin/echo`, `grep`, `sh`, …) — **chỉ Linux**.
- **Targets**: không thiếu target ⇒ **không có BLOCKED-ENV**. Core 18/18. SQLite 16/18 + 2 ô
  `EXPECTED_RED` đúng nợ #156 (`ring`/`libsqlite3-sys` cần C toolchain khi `cargo check` chéo
  msvc/wasm); đây là giới hạn môi trường/check, **không** phải test sản phẩm đỏ.
- **Fresh clone**:
  - Lần 1 tại `/tmp/opencode/...`: **FAIL do hết chỗ** — `/tmp` là tmpfs 7,2G, target build 4,8G
    làm `rustc-LLVM ERROR: IO failure on output stream: Disk quota exceeded`; đã xoá ngay.
  - Lần 2 tại `/home/ps/vis-qa-v010` (ngoài repo, cùng ổ đĩa còn 17G), target riêng
    `/home/ps/vis-qa-v010-target`: **PASS 3448/3**, checkout đúng `50675dd`; đã xoá clone + target
    sau khi ghi log (không để lại rác trong repo).
  - Không có thay đổi nào trong repo từ clone; đây là gate **nguồn sạch** tại tag.

## 2. Checklist `v0.2.0` (theo VD-271 bổ sung)

> Điều kiện chung: code + test + docs + gate/nightly/fresh-clone xanh; không tuyên bố DONE nếu thiếu.
> Trạng thái dưới đây là **QA check**, không thay bảng của A0.

### 2.1 VS3-1d — flip async default cho chương trình có `async fn`

| # | Hạng mục | Tiêu chí QA | Trạng thái |
|---|---|---|---|
| 1d.1 | Chọn runtime | Chỉ chọn frame runtime khi HIR **có** `async fn`; chương trình sync giữ đường cũ, byte-identical | TODO (W1) |
| 1d.2 | `Pipe` parity | `Pipe` parity với interp **hoặc** bị từ chối rõ trước flip; không fallback đồng bộ im lặng | TODO (W1) |
| 1d.3 | Guard scheduler | Lỗi scheduler/không hỗ trợ có chẩn đoán rõ, không panic, không treo | TODO (W1) |
| 1d.4 | Cache/phiên bản | Nếu cache key đổi ⇒ bump `PHIEN_BAN` + test tương thích; nếu không, chứng minh byte-identical | TODO (W1) |
| 1d.5 | Bằng chứng | Test async/sync + interp↔VM parity + `gate --fast`/nightly; không hồi quy 3448 | TODO |

### 2.2 VS3-2b — timeout kênh

| # | Hạng mục | Tiêu chí QA | Trạng thái |
|---|---|---|---|
| 2b.1 | API nhận | `recv_timeout(ms) -> T?`; **None = timeout hoặc closed**, tài liệu ghi rõ | TODO (W1) |
| 2b.2 | API gửi | `send_timeout(ms, v) -> bool`; **false = timeout hoặc closed** | TODO (W1) |
| 2b.3 | `select` | **Hoãn VS3-2c** theo VD-271; không tự thêm cú pháp/API ngoài quyết định | Theo dõi |
| 2b.4 | Ca biên | `chan(0)`/`chan(n)`, timeout=0, đóng kênh, nhiều waiter, không treo; chạy cả interp/VM | TODO |
| 2b.5 | Bằng chứng | Test targeted + gate/nightly; docs/cheat-sheet cập nhật | TODO |

### 2.3 VS3-3b — worker pool bản đầu (opt-in/test-only)

| # | Hạng mục | Tiêu chí QA | Trạng thái |
|---|---|---|---|
| 3b.1 | Thứ tự | Tích hợp **sau** VS3-1d; không đụng `rt.rs`/`Chan` trước | WIP (W1/W5) |
| 3b.2 | Handle | `TaskId`/proxy opaque; không `Arc<Mutex<Task>>` toàn cục | Prototype only |
| 3b.3 | Env/affine | W=1 giữ shared env; W>1 snapshot top-level + **từ chối** ghi chéo rõ | Prototype only |
| 3b.4 | Print | Buffer theo task + merge **spawn order**; W=1/W=4 byte-identical | Prototype only |
| 3b.5 | Budget | Một pool atomic + chunk reservation; default off | Prototype only |
| 3b.6 | Công tắc | Worker **opt-in/test-only** tới nghiệm thu; default vẫn single-thread + VERSION/cache | Chưa |
| 3b.7 | Bằng chứng | Ca W=1/2/4, race/join/error, benchmark CPU-only + payload matrix; gate | Chưa |

### 2.4 Điều kiện tag `v0.2.0`

- [ ] 1d.1–1d.5 xanh; sync program **byte-identical**; không fallback im lặng.
- [ ] 2b.1–2b.5 xanh; `select` vẫn hoãn.
- [ ] 3b.1–3b.7 xanh; worker **không** bật default.
- [ ] `gate --fast` (≥ 3448/3, không nới), nightly PASS, diff 100k lệch 0, host suite 54/54.
- [ ] Fresh-clone gate xanh tại commit tag; CHANGELOG/release notes ghi rõ phần chưa có.
- [ ] **A0** tạo/push tag; W3 không tự tag.

## 3. Checklist `v0.3.0` (theo VD-271 bổ sung)

### 3.1 VS3-4b2 — HTTP/2 (nếu `h2` qua review)

| # | Hạng mục | Tiêu chí QA | Trạng thái |
|---|---|---|---|
| 4b2.1 | Dependency review | `h2` license/unsafe/transitive được duyệt; ghim version; không kéo bất ngờ | WIP (W2) |
| 4b2.2 | Feature/công tắc | Feature `http2` **default-off**; ALPN default-off; HTTP/1.1 là mặc định | WIP |
| 4b2.3 | Bằng chứng local | Test server h2 local: multiplex/keep-alive, header/body, lỗi rõ; **không** hồi quy HTTP/1.1 | WIP |
| 4b2.4 | Fallback | Nếu h2 không khả thi: ghi rõ trong release notes, tag `v0.3.0` với HTTP/1.1 hoàn chỉnh, HTTP/2 → `v0.3.1` | Theo dõi |
| 4b2.5 | TLS/registry | TLS + registry HTTP(S) hiện có không hồi quy; chữ ký/GC giữ test | TODO verify |

### 3.2 Net/package ổn định

- [ ] `vs3_tls`, `vs3_keepalive`, `vs3_cookie_auth`, `vs3_registry*`, `vs3_pkg_gc*` xanh trong gate.
- [ ] Registry HTTPS + Ed25519 + GC/migrate không hồi quy; lỗi mạng/hash/chữ ký rõ.
- [ ] Không thêm dependency ngoài danh sách đã duyệt.

### 3.3 Windows verify

- [ ] W4 chạy lại `cargo test --workspace` + `vis-dev check` trên `psvn` tại commit tag candidate;
  **0 product failure**, test-only/portability đã đóng hoặc ghi rõ.
- [ ] `bin/vis-targets` core 18/18; sqlite EXPECTED_RED chỉ đúng #156 (C toolchain), không thêm ô đỏ.
- [ ] Không tuyên bố Windows Tier 1; macOS **ngoài phạm vi**.

### 3.4 Điều kiện tag `v0.3.0`

- [ ] 4b2.1–4b2.5 (hoặc quyết định fallback có văn bản).
- [ ] Net/package ổn định + Windows verify.
- [ ] Gate/nightly/diff 100k/host suite/fresh-clone xanh; cây sạch.
- [ ] Release notes + CHANGELOG; **A0** tag/push; không di chuyển tag cũ.

## 4. Blocker / rủi ro QA

1. **`v0.2.0` chưa thể tag**: 1d, 2b, 3b đang WIP; checklist §2 chưa có bằng chứng.
2. **`v0.3.0` phụ thuộc `h2`**: chưa có review dependency/HTTP2 test; nếu không qua, phải chốt
   fallback theo VD-271 §5 (ghi rõ, HTTP/2 sang `v0.3.1`).
3. **Windows**: QA này **không** chạy Windows; chỉ ghi nhận tag `v0.1.0` có 3404/0 từ release trước
   và lane W4 đang verify. Không dùng số cũ làm bằng chứng cho `v0.3.0`.
4. **Cross-target**: 2 ô `EXPECTED_RED` của `vis-cli` (msvc/wasm) do #156 (C toolchain) — không phải
   lỗi sản phẩm, nhưng không được đổi nhãn thành PASS khi chưa có toolchain.
5. **Fresh-clone `/tmp`**: tmpfs 7,2G không đủ; lần sau dùng thư mục trên `/home` và target riêng,
   hoặc dọn target trước. Đã ghi bằng chứng lần 2 PASS.
6. **Build chéo lane**: fresh-clone chạy khi W1/main có build cục bộ; thời gian 299 s có nhiễu.
   Không có test đỏ; cần lặp lại trên máy rảnh nếu muốn số thời gian chuẩn.

## 5. Lệnh tái lập

```bash
# Tại worktree W3, HEAD f710e5b (cùng cây main); tag v0.1.0 = 50675dd
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_TARGET_DIR=/home/ps/ViScript/target

./vis-dev gate --fast
./vis-dev nightly --skip-gate --fuzz-minutes 2      # gate đã chạy ngay trước
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261121 ./bin/vis-diff
./bin/vis-host-suite
./bin/vis-targets

# Fresh-clone (dùng ổ /home, KHÔNG dùng /tmp tmpfs 7,2G)
git clone --quiet --branch v0.1.0 --depth 1 . /home/ps/vis-qa-v010
cd /home/ps/vis-qa-v010
CARGO_TARGET_DIR=/home/ps/vis-qa-v010-target ./vis-dev gate --fast
# xoá /home/ps/vis-qa-v010 + /home/ps/vis-qa-v010-target sau khi ghi log
```

**Kết luận QA**: baseline `v0.1.0`/main **xanh** trên Linux (gate 3448/3, nightly PASS, diff 100k
0 lệch, host 54/54, core targets 18/18, fresh-clone 3448/3). Chưa đủ điều kiện tag `v0.2.0`/`v0.3.0`;
checklist và blocker ở §2–§4. Không tag/push/merge trong phiên này.
