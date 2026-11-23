<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — ma trận hỗ trợ Windows (thực tế sau các đợt)

> **Người tổng hợp**: subagent VS3-W4, 2026-11-22.
> **Mốc**: `main` **`c29263f`** (code = `3aec977`; Linux gate **3571/3**); branch `vs3/w4-windows`.
> **Phạm vi**: chỉ Windows x86_64 (MSVC native trên `psvn`); macOS ngoài phạm vi. Mọi số liệu lấy
> từ journal/log đã đo, không suy diễn từ Linux.

## 1. Bảng ma trận

| Mảng | Trạng thái Windows | Bằng chứng chính | Giới hạn / ghi chú |
|---|---|---|---|
| **Async net/TLS/HTTP-1.1** (`AsyncNet`, `Nguon`, `host_poll`) | **HỖ TRỢ** (`cfg(any(unix, windows))`) | 10 vòng × (`vs3_default` 22/22 + `vs3_tls` 22/22) @ `54f813f` (đã gộp vào `a8100ab`); soak 137 vòng / 8 499 test @ `83b4002`; release workspace **3528/0/3** @ `3aec977` | Wasm không có backend (đường cũ). Loopback-only theo chính sách VD-100/105 |
| **Sync HTTP/2** (`vs3_http2`) | **HỖ TRỢ** | **19/19** debug và release (debug: 5 vòng @ `a8100ab`; release @ `a8100ab`, `3aec977`) | Client sync; server test loopback; h2 default-on từ v0.3.0 |
| **Async HTTP/2** (`async-h2`) | **KHÔNG hỗ trợ Windows** — **Unix-only, default-off** | `cargo check --features async-h2` trên Windows = **101** (`h2_async.rs:15 cfg(…, unix)`) | Expected red; không sửa, không tuyên bố. Feature giữ default-off toàn workspace |
| **Worker pool** (`vs3_worker`) | **HỖ TRỢ ở mức opt-in/test-only** | **14/14** debug + release (nhiều vòng, soak) | Chưa nối evaluator mặc định; W=1 shared env, W>1 snapshot; cross-write W>1 bị từ chối |
| **Evaluator P1** (`vs3_worker_eval_local`) | **HỖ TRỢ — opt-in, CPU non-frame** | **8/8** debug + release | Frame/`chan(0)`/host trong worker: chưa (P1 test xác nhận từ chối rõ) |
| **Evaluator P2** (`vs3_worker_eval_parallel`) | **HỖ TRỢ — opt-in, CPU non-frame** | **14/14** debug + release; soak P2 **233 vòng / 1 802 s**, 0 fail, 10 150 test | Không dùng reactor/frame; `snapshot_tai_spawn_w1_khac_w2_co_chu_dinh`, cancel safe-point |
| **Evaluator P3** (`vs3_worker_eval_vm`) | **HỖ TRỢ — opt-in, CPU non-frame** | **10/10** debug + release @ `d9e33f3` | Parity VM với W1/W2/W4; chưa nối frame runtime |
| **CLI** (`--version`, `-V`, `doctor`) | **HỖ TRỢ** | `vis 0.1.0`; `doctor` → `os windows`, `arch x86_64`, `sqlite on`, `features sqlite,tls,pkg-sig,http2`; `run --version` rc 2 | Release artifact ~11,2 MB (xem §3) |
| **Host conformance** (`bin/vis-host-suite`) | **HỖ TRỢ** | **54/54** trên Windows (`host_posix_real` bỏ qua — Windows dùng `test-child`) | Chỉ đo nền đang chạy |
| **Target matrix** (`bin/vis-targets`) | core **18/18**; sqlite **17 PASS + 1 EXPECTED_RED** | Mỗi lần verify/soak | Ô đỏ duy nhất `vis-cli × wasm32` do C toolchain `libsqlite3-sys`/`ring` — nợ **#156**; chỉ **CHECK**, không chạy wasm |
| **Wasm runtime** | **KHÔNG hỗ trợ** (EXPECTED_RED #156) | `vis-targets` | Không có std::process/net/fs runtime trên wasm32-unknown-unknown |
| **macOS** | **Ngoài phạm vi** đợt này | — | `mps` là đích riêng; không suy diễn từ Windows/Linux |

## 2. Số liệu theo mốc (Windows)

| Mốc | Nội dung | Kết quả |
|---|---|---|
| `54f813f` | Fix flake sync-net `tcp_explain_tong` (`shutdown(Write)` + drain) | 20 lần chạy `net` 9/9, **0 lần 10053** |
| `a8100ab` | Verify sau merge Windows async net | 2 workspace **3496/0/3**; 5 vòng targeted 22/22+22/22+9/9; release 10 752 512 B |
| `83b4002` | Soak Windows 30 phút | **137 vòng / 1 811 s**, 0 fail; 5×h2 debug 19/19 + 1 release 19/19; worker 14/14 |
| `c943793` | Verify P2 | workspace **3518/0/3**; P2 14/14 debug+release; P1 8/8; 3 vòng targeted; release 11 098 112 B |
| `d9e33f3` | Verify P3 VM | workspace **3528/0/3**; P3 **10/10** debug+release; P2 14/14; P1 8/8 |
| `ffe202b` | Soak P2 30 phút | **233 vòng / 1 802 s**, 0 fail; 10 150 test; free memory +37 MB; `vis_proc=0` mọi vòng |
| `3aec977` | Release-mode Windows | `cargo test --release --workspace` **3528/0/3** (531 s); 3 vòng release xanh; h2 19/19; worker 14/14; release 11 205 120 B, sha256 `929d081a…2bbf2` |

- Debug workspace tiến hoá theo P1/P2/P3: 3496 → 3504 → 3518 → **3528** (3 ignored không đổi).
- Release workspace cùng số pass như debug ⇒ **không có ca debug-only bị skip** ở release (theo số đếm).

## 3. Lệnh kiểm tra chuẩn trên `psvn`

Chạy trong worktree sạch (detached đúng commit cần đo), dùng `cargo` 1.100.0:

```bash
# Cây sạch + mốc
git -C /c/Users/ps/viscript-moi worktree add --detach /c/Users/ps/viscript-verify <commit>
cd /c/Users/ps/viscript-verify && git status --porcelain   # phải rỗng

# Toàn workspace (debug + release)
cargo test --workspace --no-fail-fast
cargo test --release --workspace --no-fail-fast

# Async net/TLS + sync net + sync h2
cargo test -p vis-cli --test vs3_default --test vs3_tls --test net
cargo test --release -p vis-cli --test vs3_http2

# Evaluator (opt-in) + worker
cargo test -p vis-cli --test vs3_worker_eval_local
cargo test -p vis-cli --test vs3_worker_eval_parallel
cargo test -p vis-cli --test vs3_worker_eval_vm
cargo test --release -p vis-cli --test vs3_worker_eval_parallel
cargo test --release -p vis-cli --test vs3_worker

# Host / targets / release + CLI
bin/vis-host-suite
bin/vis-targets
cargo build --release -p vis-cli
target/release/vis.exe --version
target/release/vis.exe doctor
printf 'print(6 * 7)\n' > %TEMP%/smoke.vis
target/release/vis.exe check %TEMP%/smoke.vis
target/release/vis.exe run   %TEMP%/smoke.vis

# Windows + async-h2: PHẢI đỏ (expected)
cargo check -p vis-cli --features async-h2   # rc 101, h2_async.rs cfg(..., unix)
```

Khi chuyển file/log sang `psvn`, dùng `bin/vis-remote psvn` hoặc `git bundle` + worktree; log ghi
ngoài repo (`/c/Users/ps/vislogs/`) để không chặn `git pull`.

## 4. Caveat đo lường

1. **Mỗi ca test là một process riêng** (`cargo test` spawn test binary; nhiều ca spawn thêm
   `vis.exe`): không đo handle/thread count **xuyên tiến trình**.
   - Thay thế: `AsyncNet::dem_tay_dang_mo()` (test 200 vòng mở/đóng, kỳ vọng 0),
     P2 `muoi_vong_threads_fd_rss_khong_ro`, và đếm `(Get-Process vis).Count` sau mỗi vòng
     (soak P2: **0/233** lần).
2. **Bộ nhớ**: chỉ đo `FreePhysicalMemory` toàn máy trước/sau soak (P2: 10 088 336 KB →
   10 125 896 KB, +37 MB; không giảm). Không phải RSS riêng của `vis.exe`.
3. **Thời gian**: Windows timer thô (~15,6 ms); test timeout phải có biên. Các ca hiện dùng
   timeout ≥50 ms và đã xanh ổn định.
4. **AFD/IOCP**: `mio` dùng AFD poll; LSP/VPN/antivirus chen giữa có thể ảnh hưởng readiness.
   `psvn` sạch trong các đợt đo; nếu gặp treo lạ, kiểm môi trường trước khi coi là lỗi sản phẩm.
5. **Cross-compile**: máy Linux không có target `x86_64-pc-windows-gnu`/mingw
   (`BLOCKED-ENV`); bằng chứng Windows là **MSVC native trên `psvn`**. `vis-targets` chỉ
   `cargo check` cho msvc/wasm, **không** chạy runtime.
6. **Async-h2**: mọi tuyên bố Windows **không** áp dụng cho `async-h2`; feature default-off,
   compile trên Windows là expected red.

## 5. Kết luận

- Windows x86_64 (MSVC): **async net/TLS/HTTP-1.1, sync h2, worker pool + evaluator P1/P2/P3
  (opt-in/test-only, CPU non-frame), CLI/doctor, host 54/54, targets 18/18 + 17/1** đã được đo
  xanh ở cả debug và release, kèm soak 30 phút ×2 (137 vòng và 233 vòng).
- **Chưa hỗ trợ**: async-h2 trên Windows (Unix-only/default-off), wasm runtime (#156), macOS
  (ngoài phạm vi), frame/`chan(0)`/host **bên trong worker evaluator** (mới CPU non-frame).
- Không blocker.
