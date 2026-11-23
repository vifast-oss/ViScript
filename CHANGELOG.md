<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Nhật ký các đợt (CHANGELOG)

Tóm tắt theo từng đợt phát triển. Chi tiết đầy đủ ở [`agent/reports/`](agent/reports/) (mỗi đợt một cặp `.json` + `.md`).

## v0.5.0 — 2026-11-23 (Giai đoạn 3 — Pha 2 ngôn ngữ, ViShell MVP, macOS Tier 1)

> **Chưa tag.** Mốc `v0.5.0` mới ở trạng thái **prep** trên `main` `f2d3e73`; macOS arm64 Tier 1
> còn **chờ kết quả `ST-300m3`** (3× `bin/vis-gate-macos --full` + soak 4 h + `vis-wasm-sem`).

- **ST-302** (`d3ea467`, merge `b79ae3a`): Pha 2b — `async fn` trong `impl` (`self`/`var self`) +
  `async cmd` (đóng `#13`, giữ `--flag` riêng của `cmd`); `derives Json` sinh **mã thật**
  `to_json`/`from_json` cho `type`/`enum` (từ chối `cap`/`Chan`/`Task`/`fn` bằng `VS0412`, đóng `#8`);
  15 + 3 ca; gate Linux **3676/4**; diff 100k `lech=0`.
- **ST-302b** (`204af84`, merge `6bc4ac3`): `spawn obj.m(args)` / `spawn self.m(args)` cho method
  `async` (method đồng bộ ⇒ `VS0412`), 12 + 1 ca, snapshot formatter; gate Linux **3689/4**.
- **ST-304** (`6e0675a`, merge `8e73e81`): `crates/vish-edit` — soạn dòng đa nền, raw mode
  `rustix::termios` (Unix/macOS) + `crossterm` (Windows), đa dòng tự thụt lề, lịch sử append-only +
  tìm mờ, tô màu CST, Ctrl-C ⇒ 130; 0 `unsafe`; 8/8 + 9 Windows; gate Linux **3679/4**.
- **ST-304b** (`426d636`, merge `1eb2768`): completion theo kiểu (record field/enum variant/`cmd`
  flag/path) + POSIX job control (`&`, `jobs`/`fg`/`bg`/`kill`) + PTY giả; `vish-edit` **17/17**;
  gate Linux **3688/4**.
- **ST-305** (`fa1a70b`..`f2d3e73`, merge `374b4e8`): LSP hover in tham số generic + completion sau
  `std.`; extension `editors/vscode/` (language client `vis lsp --stdio`, TextMate grammar);
  `lsp_it` **155/155**, `vis-lsp` **29/29**; Windows quick **PASS**; refresh `agent/` + docs.
- **ST-306** (`90b2964`, merge `6093aef`): Plugin **tĩnh** Native Rust — `VIS_PLUGIN_ABI_V1`,
  `vis-plugin-abi` (`PluginValue`/`PluginCtx` chỉ quyền host cấp) + `vis-plugin-macros`
  (`#[vis::module]`/`#[vis::func]`), parity interp↔VM; 0 `unsafe`; gate Linux **3680/4**;
  Windows **8/8** + quick **91/91**.
- **ST-307** (`3d247c4` + hardening `f291152`, merge `ab6beb7`/`b5aa40b`): Plugin **động**
  WASI/WASM — `PluginWasi::nap` kiểm magic/version/`_start`/marker ABI trước khi chạy, sandbox chỉ
  preopen `cap` được trao, tắt cứng `--strict-caps`/`--sandbox`; mẫu `test/plugin-wasi` parity qua
  `node:wasi`; **10/10**; gate Linux **3690/4**; Windows quick **91/91**.
- **ST-308** (`79a6d52`, merge `a9eef01`): `crates/vish-term` — `Terminal` trait,
  `AnsiPosix`/`WinConsole`/`ViTermFramed` (JSON Lines `vish-term/1`), `Capabilities`/`Style`/SGR
  16-256-truecolor, `Table`/`Diagnostic`, `display_width` NFC; workspace **16 crate**; gate Linux
  **3691/4**.
- **ST-300m2** (`101a4ad`, merge `5741d23`): đo độc quyền `mps` arm64 — đóng **`#147`** (soak 10
  vòng: `mps` **379,7 s** / `ps` **517,1 s = 0,73×**; gate **0,93×**) và **`#152`** (24 cặp A/B:
  interp **1,0385**, VM **1,0626**).
- **macOS arm64 Tier 1**: **CHƯA công bố** — `ST-300m3` (3× `vis-gate-macos --full` + soak 4 h +
  `vis-wasm-sem` ≥ 388/390) **đang chạy/chờ kết quả**; chỉ tuyên bố khi đủ bằng chứng.
- **Version** 0.5.0 (`version.workspace` một nguồn); `vis --version` ⇒ **`vis 0.5.0`**; cache
  `PHIEN_BAN` `+v050-release-2026-11-23`, khoá
  `fc20a81123cdb432a976e7d3d1fa15b7f4a2dd3b2276241bd0888e55bda53117`; `assert_ne` khoá 0.4.5
  `d92f31c2…` ⇒ cache cũ tự vô hiệu; `vis-host-wasm/Cargo.lock` đồng bộ 0.5.0. **Tag `v0.5.0`:
  CHƯA** (trước: `v0.4.5` @ `3daff62`, `v0.4.1` @ `90844bc`, `v0.4.0` @ `c2accdf`).

## v0.4.5 — 2026-11-23 (Giai đoạn 2 — Platforms)

- **ST-297** (`f4cb136`): đóng `#149`; LSP URI UNC/Windows + `--grant` path; Windows 3× full **3611/0/3**;
  Linux 10× `vis-lsp` 29/29 + `path_impl` 2/2.
- **ST-298** (`f4cb136`): Windows **3× `vis-gate-windows --full` PASS 3611/0/3**; soak 4h
  **985 vòng, fail 0, flake 1**, free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0; **Windows Tier 1** công bố (caveat `FsGuard`,
  uỷ quyền `VD-283`).
- **ST-299a/299b**: `WasiHost` + `vis-loader` đa tệp trên WASI; native **29/29**, wasm **28/28**.
- **ST-300**: `bin/vis-wasm-sem` **388/390 eligible = 99,5%** (VM 100%), suite **28/28**, nợ `#193`
  (2 ca stack-overflow interp); row `wasm32-wasip1 (runtime)` trong `vis-targets`.
- **Version** 0.4.5; cache `PHIEN_BAN` `+v045-release-2026-11-23`, khoá
  `d92f31c28ed5e97a07976b83be0fb6a3c4c3914120e4e8f98e9fc832685418ce`; fix `bin/vis-dev` fuzz theo
  `CARGO_TARGET_DIR`. **Tag `v0.4.5` trên commit R35 này**; trước: `v0.4.1 @ 90844bc`, `v0.4.0 @ c2accdf`.

## v0.4.1 — 2026-11-23 (Giai đoạn 1.5)

- **Phiên bản**: workspace **0.4.1** (`version.workspace = true`); `vis --version` ⇒ **`vis 0.4.1`**;
  `PHIEN_BAN` thêm `+v041-release-2026-11-23`, khoá cache ghim lại (VD-129) — cache 0.4.0 tự vô
  hiệu; `VERSION` VBC1/ABI không đổi.
- **ST-296a** (`53990c0`): gom test binary — `vis-cli` **280 → 22** binary; gate **352 → 92**
  binary; gate **3656/4**; wall **304 → 210,77 s**.
- **ST-296b** (`067e1a5`/merge `1b27cb8`): tách **`vis-loader`** + **`vis-lsp`**, gom
  `[workspace.dependencies]`; gate **3656/4**; Windows post-merge **3607/0/3**; `lsp` **27/27**,
  `loader` **29/29**, targets core **18/18**.
- **ST-296c** (`ab8d393`): tách tệp sát R1 — `vis-types/stdlib.rs` **1456 → 23 + 11 module**;
  sqlite tests **1495 → 68 + 3**; `lower/expr.rs` **1456 → 702 + 2**; `lower.rs` **1446 → 420 + 2**;
  interp stdlib **68 → 17 tệp**; gate **3656/4**, diff 100k **0 lệch**.
- **ST-296d** (`a1730c5`): **63 tệp R100** vào `agent/archive/`, `DECISIONS-INDEX` **271 VD**;
  `agent/` rời **77 → 19**; **report-check PASS 33/329**.
- **Lock fix final QA** (`11e8120`): `vis-host-wasm/Cargo.lock` ghim **0.4.1** (trước đó 0.4.0 làm
  `bin/vis-targets` bẩn cây trên Windows) — đã xác nhận chạy lại sạch.
- **Final QA Linux @ `5d7791c`**: fresh-clone gate **3656/4**; release **3661/0/4** (445 s);
  **soak 10/10 = 530 s** (3× gate 3656/4, 3× diff 100k `lech=0`, host 54/54, cache 5/5).
- **Final QA Windows @ `5d7791c` + `11e8120`**: full **3607/0/3** (478 s); release **3606/0/3**
  (384 s); worker_frame 12/12+6/6, P1/P2/P3 8/14/10, select 25, h2 33, cancel 8, lsp 27, loader 29;
  `vis 0.4.1`, khoá cache `42f53893…7c0f`.
- **ST-297** (`f4cb136`, merged sau tag): LSP URI/UNC + `--grant` Windows (**`#149` đóng**); gate
  **3660/4** (4 ca mới UNC/mixed/percent + `--grant` Windows), Windows **3× full 3611/0/3**, host
  54/54, targets **18/18 + 17/1**; Linux **10×** `vis-lsp` **29/29**, `path_impl` **2/2** (`8b03077`).
- **ST-299b** (`3b8049c`, merged): `vis-loader` vào `vis-wasm-run` — nạp **đa tệp + `use`** trên
  WASI; native **29/29**, wasm **28/28**.
- **ST-300** (`3581c9a` + `8286caf`, merged): harness `bin/vis-wasm-sem` **388/390 eligible =
  99,5%** (2 stack-overflow interp, VM pass); suite cũ **28/28**, native **29/29**, row
  **`wasm32-wasip1 (runtime)`** trong `vis-targets`.
- **Final Linux soak** (`45122b7`/`84e8eb6`): **10/10** (759 s), 3× diff 100k `lech=0`, host
  54/54, cache 5/5, wasm **28/28**.
- **P2b-3/4** (`16fba3a`, merged): **NO-GO** — bench tỉ lệ **4,0×**, không đạt ≤1,5×; **không đổi
  production**.
- **Flake fix** (`044b289`, test-only): `vs3_worker::spawn_shared_stress_rounds_read_only` poll
  **2 s** sau join (T-INFRA; **0/60** đỏ dưới tải).
- **ST-298** @ `f4cb136`: **3× `vis-gate-windows --full` PASS 3611/0/3** (host 54/54, targets
  18/18 + 17/1, dirty 0); **soak 4h đang chạy** — `985` vòng, `0` fail,
  fd/RSS `free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`; **chưa** tuyên bố Windows Tier 1.
- **Tag `v0.4.1` @ `90844bc`** (annotated, pushed); tag trước **`v0.4.0` @ `c2accdf`**.

## v0.4.0 — 2026-11-23

- **Phiên bản**: workspace **0.4.0** (`version.workspace = true` cho mọi crate thành viên);
  `vis --version` ⇒ **`vis 0.4.0`**; `PHIEN_BAN` thêm nhãn `+v040-release-2026-11-23`, khoá cache
  ghim lại (VD-129) — cache 0.3.6 tự vô hiệu, `VERSION` VBC1/ABI không đổi.
- **ST-290** (`3627f3bc`, merge `a48d5b8`): ngân sách ngữ nghĩa mặc định **40M** interp↔VM,
  `VS0509`/`VS0510` (sổ **56** mã), cache VM có trailer `VFEE`; workspace **3594/0/3**, diff 0.
- **ST-291** (`269ff71`, merge `8cc3588`): P2b-1/2 lazy/batched submit + compact `Vec` proxy;
  batch 10k **29,1 ms** interp / **28,6–29,7 ms** VM; gate **3638/4**, Windows verify **3588/0/3**.
- **ST-292** (`eca4e82`, merge `e47b3ca`): P4a/P4b frame + worker pump, `VS0509`/`VS0510` cho
  cross-write/budget, CLI `vis run --workers <N>` (mặc định 1); 12+6 ca frame; gate Linux
  **3656/4**, diff 100k **0 lệch**.
- **ST-293** (`82064e9`, merge `6d6bfe9`): `select_recv*`/`select_send*` trên frame runtime;
  **25 ca**, fairness/waiter, interp↔VM byte-identical; gate **3632/4**, diff 0.
- **ST-294** (`39cc81cc`, merge `e35dcf6`): `async-h2` bỏ giới hạn Unix, chạy Windows qua
  `Nguon`/`mio` và bật **mặc định non-wasm**; Windows h2 **33/33**, Linux gate **3607/4**.
- **ST-295** (`ba4023f`, merge `cd76da5`): `rustix =1.1.6` `openat2(RESOLVE_BENEATH|
  RESOLVE_NO_SYMLINKS)` + fallback `O_NOFOLLOW`; ca đua ghi **100/100 → 0/100**; Landlock `#125`
  (Execute + libs) **18/18**; gate **3613/4**, diff 0.
- **ST-296d** (`b36f8a5` + `4b2484f`): luật **R7** (log >50 KB phải ngoài repo/legacy), nextest
  `retries=0`, `bin/vis-{gc-targets,jobs,status,gate-windows}`; Windows quick **pkg 25 / h2 33 /
  cancel 8 / select 25**.
- **ST-299a** (`5cc1903`, merge `060a744`): `WasiHost` thật cho `wasm32-wasip1` + `vis-wasm-run`;
  native **27/27**, wasm **26/26** trên wasmtime 51 + Node 26; binary **3.430.132 B**.
- **Timing fix** `6c7e188`: `vd270_test_slowest` chịu tải (không `#[ignore]`), giữ kiểm regression.
- **Linux final QA @ `e47b3ca`**: gate **3656 passed / 4 skipped** (352 binary, 195,84 s);
  release **3661/0/4** (445,18 s); **2× soak 10/10** (531 s + 642 s; mỗi soak 3× gate 3656/4,
  3× diff 100k seed `20261115` `lech=0`, 2× host 54/54, 2× cache 5/5); diff ngoài soak seed
  `202611230230` **0 lệch** (100.000; 69,2 s); worker_frame **12/12**, frame_vm **6/6**, P1 **8/8**,
  P2 **14/14**, P3 **10/10**, h2 **34+1**, cancel **8/8**, select **25/25**, pkg **25/25**, host
  **54/54**; budget `--workers 4 --max-budget 5000` → **101 + VS0506/VS0510**.
- **Windows final QA @ `e47b3ca`**: full **3607/0/3** (361 block, 700 s); release **3606/0/3**
  (534 s); worker_frame 12/12 + 6/6, P1 8, P2 14, P3 10, select 25, h2 33, cancel 8, host 54/54,
  targets **18/18 + 17/1**. Release **0.4.0 @ `8696edc`**: `vis.exe` **11.530.240 B**, sha256
  `3813a1b3e7aa2f8608aaf0a168e2dffbc0d904554bf0a096fee5927a821cb065`, `vis 0.4.0`; version 2/2,
  cache 3/3 (khoá `22572d0b…3a8b`), budget parity 101/VS0510, `--workers 0` rc 2.
- **Tag `v0.4.0` trên commit R33** (sau `./vis-dev check` xanh); tag trước **`v0.3.6` @
  `8fa2c09`**. Nhánh **READY chưa merge**, xét sau tag: `plan/w4-st296a-tests` @ `f961b83`
  (gom test binary) và `plan/w3-st296b-crates` @ `7c4c205` (tách `vis-loader`/`vis-lsp`).

## v0.3.6 — 2026-11-22

- **Stage 0 — R0 refactor 0 đổi hành vi**: `vm.rs` **1240**, `frame.rs` **1042**, `value.rs` **1186**, `codegen.rs` **1045**, `tyck/expr.rs` **1064**, `interp/goi.rs` **1155** dòng (ngưỡng ≤1.250); tách `vm/host.rs`, `frame/worker.rs`, `value/show_eq.rs`, `codegen/cau.rs`, `tyck/expr/goi.rs`, `interp/goi_ham.rs`; 0 đổi API/ABI/cache/golden.
- **Gói**: `VS0708` (xác minh chữ ký/khoá tin cậy registry; vi/en/ja; sổ **54** mã, **8** mã module); `pkg update <tên>` dùng **soft pin** — đóng **#142**; `pkg gc --root` lặp/lồng dedupe lock/workspace, không tạo `.vis`.
- **HTTP cancel h1/h2**: `FrameReactorBackend::cancel_task_handles` + safe point `frame_cancel_one` — h1 đóng socket, h2 `RST_STREAM` đúng stream/giữ session dùng chung; `vs3_http_cancel` **5/5** mặc định, **8/8** feature `async-h2`; soak 1 giờ fd/RSS/threads phẳng. Windows fix: test fixture `accept` thừa nonblocking → server blocking + trigger byte; product guard `cancelled` + close order (deregister trước drop socket).
- **Phiên bản**: **0.3.6** (`version.workspace = true` cho 8 crate); `vis --version` = `vis 0.3.6`.
- **Linux — fresh-clone `4c5dec6`**: gate **3581 passed / 3 skipped**; `cargo test --release --workspace --no-fail-fast` **3586 passed / 0 failed / 3 ignored**; P2 **14/14**, P1 **8/8**, P3 **10/10**; h2 **19/19**, feature **34+1**; `pkg` **25/25**, `vs3_registry_sig` **10/10**, `vs3_http_cancel` **5+8**, host **54/54**; diff 100k seed `202611221700` **0 lệch**; **2× soak 10/10 = 20/20 vòng** (593 s + 602 s); `./vis-dev check` PASS.
- **Windows ST-287 fix @ `6b2a2161`**: `vs3_http_cancel` serial **20/20**, default **10/10**, exact lap **20/20** (trước: serial 2/10 pass, default 8/10); workspace **355 block, 3538 passed / 0 failed / 3 ignored**, dirty 0; root cause test fixture (`accept` thừa nonblocking trên Windows; scheduler log không có `cancelreq`/`cancel:1`). Feature h2 **BLOCKED-ENV** (Unix-only). **`v0.3.6` đã tag @ `8fa2c09`** (code `0645700`; Linux 3581/3, Windows 3538/0).

## v0.3.5 — 2026-11-22

- **Fix race cancel-before-run**: `WorkerPool` đặt cờ huỷ **trước** khi enqueue (`spawn_inner`, `spawn_cancelled`, `spawn_shared_with_cancel`); `spawn_parallel` truyền `cancel_before`, bỏ `handle.cancel()` sau spawn. 20.000 vòng sau fix 0 fail; P2 **10× = 140/140**, cancel-before **1.000/1.000**.
- **Windows**: workspace **3528/0/3**, P2 10× 14/14, P1 8/8, P3 10/10 debug+release; release 11.205.120 B; host 54/54.
- **Test-only**: nới biên `net::tcp_het_gio_doc` 400 ms → **3 s** (đo wall-clock cả process 87–130 ms ấm / 1,41 s nguội); 50/50 dưới tải.
- Linux gate **3571/3**; `vis-dev check` PASS. Tag `v0.3.4` chứa race cancel-before, **v0.3.5 thay thế**.
- Không GitHub Actions; macOS ngoài phạm vi.

## v0.3.4 — 2026-11-22

- **Windows async net/TLS/HTTP-1.1**: backend `mio::Source` trực tiếp (IOCP/AFD), `Nguon`, invariant `host_poll`, mã lỗi 10035/10036; psvn **22/22 + 22/22 ×10**, soak **420 vòng / 18.024 test**, workspace **3528/0/3** debug+release, host 54/54, targets 18/18 + 17/1; flake test-only `tcp_explain_tong` (10053) fixed. Async-h2 vẫn **Unix-only**.
- **Async HTTP/2 (feature `async-h2`, default-off)**: `H2Core` dùng chung sync+async; `H<n>` nhiều frame/1 session; cold-start **1.000 vòng** 4 stream/1 connection, cap **10.000 vòng**, soak **2 giờ / 11.052 vòng** (fd 9, RSS 5,9 MB, threads 6), Set-Cookie h1+h2; sync **19/19**, feature **34+1**; cancel task giữa request còn mở.
- **Evaluator worker P1/P2/P3 (opt-in hidden)**: `WorkerMode::{Local,Parallel}`, `run_workers_*` interp+VM; W=1/2/4, snapshot, budget, cancel, cross-write; 10k task **byte-identical**; P2 soak **69.362.370 task**, P3 soak **38.342.130 task**; VM nhanh **~2,3×**; CPU non-frame, frame/P4 chưa, 10k task nhỏ còn <1×.
- **Worker `spawn_shared` (opt-in/test-only)**: `Arc<SendValue>` read-only; map1k **5,62 ms** vs producer 24,63 ms; 100 vòng/6M spawn, Arc về 1.
- **Linux**: gate **3571/3**; fuzz **1.385.002.208 ca** (seed 20261121) + **987.234.956 ca** (seed 20261122), **0 vi phạm**; diff 100k **0 lệch** (nhiều seed); soak **20/20** (fresh-clone b37aa41) + **30/30**.
- **Windows**: workspace **3528/0/3** debug+release; release `vis.exe` **11.205.120 B**; `vis --version`/doctor/smoke OK.
- **Fresh-clone `b37aa41`**: gate **3571/3**, host 54/54, diff 0.
- Không GitHub Actions; macOS ngoài phạm vi.

## v0.3.3 — 2026-11-21

- **Fix HTTP/2 flow-control**: release capacity sau mỗi chunk; body >64 KiB không còn timeout; 8 MiB cap đúng, header 64 KiB, trailer/GOAWAY/timeout.
- Linux gate **3535/3**; Windows psvn **3469/0**; `vs3_http2` **19/19** debug+release; fresh-clone PASS; host 54/54.
- Không GitHub Actions; macOS ngoài phạm vi.

## v0.3.2 — 2026-11-21

- **Async HTTP/1.1 cooperative** (Unix): state machine trên reactor, GET/HEAD/redirect/chunked/EOF, TLS nonblocking, cap 8 MiB; sync giữ nguyên.
- h2 stress 100 request/4 authority; worker stress 50k; TaskId prototype.
- Linux gate **3528/3**; Windows psvn **3462/0** (fix test-only clippy); fresh-clone PASS; host 54/54.
- Windows async HTTP/net chưa có backend. Không GitHub Actions; macOS ngoài phạm vi.
- CLI: `vis --version`/`-V`; `vis doctor` liệt kê đủ `sqlite,tls,pkg-sig,http2`.
- Windows test flake `net::tcp_explain_tong` (RST 10054) fixed test-only; psvn 3/3 3464/0.

## v0.3.1 — 2026-11-21

- **Async net production** (mio) + **async TLS** (rustls nonblocking) trên Unix; loopback read/accept/connect/write/HTTPS.
- **HTTP/2 hardening**: GOAWAY/RST/trailer/204/header cap 64 KiB, retry một lần.
- **Worker stress** 5×10k/50k task, cancel/budget/print/cross-write; 12 ca; h2 soak 5×100 request.
- **h2 multiplex prototype** 4 stream/1 connection, test-only; production vẫn queue tuần tự.
- **TaskId/Value prototype** (feature `taskid-proto` default-off) + proxy completion; ABI mặc định giữ nguyên.
- Linux gate **3512/3**, nightly 18,99M fuzz 0; Windows psvn **3463/0**; fresh-clone PASS; host 54/54.
- Windows async net chưa có backend (test gated).
- Không GitHub Actions; macOS ngoài phạm vi.

## v0.3.0 — 2026-11-21

- **HTTP/2 default-on** (h2 + ALPN, fallback HTTP/1.1, queue reuse 4 authority, idle 15 s); `http2` vào default non-wasm.
- Async `chan(n≥1)` wait-set trên frame (W1f); worker-copy payload lớn (list10k 3,18×, map1k 3,14×).
- Linux gate **3496/3**; Windows psvn **3452/0**; fresh-clone `0d26527` PASS; host 54/54; targets 18/18.
- Sau tag: async net production reactor (mio) đã merge main, vào `v0.3.1`. Không GitHub Actions; macOS ngoài phạm vi.

## v0.2.0 — 2026-11-21

- **Async default** cho chương trình có `async fn` (interp + VM); sync giữ nguyên; Pipe parity; guard 200k.
- **Timeout kênh**: `recv_timeout(ms)->T?`, `send_timeout(ms,v)->bool`; 17 ca.
- **Worker opt-in**: TaskId/proxy, snapshot W>1, print merge, budget atomic; B7 CPU-only 2,59–2,69×.
- HTTP/2 có feature `http2` **default-off**; TLS/keep-alive/cookie/auth, registry HTTP(S)/Ed25519/GC, LSP tệp phụ/incremental.
- Linux gate **3480/3**; nightly 14.728.191 fuzz 0 vi phạm; diff 100k 0 lệch; host 54/54; targets 18/18.
- Windows psvn **3436 passed / 0 failed / 3 ignored**; `vis-dev check` PASS; host 54/54; targets 18/18 + 17/1.
- Fresh-clone `42b700c` PASS; **macOS ngoài phạm vi**; **không GitHub Actions**.
- Còn mở: async net production, worker evaluator, HTTP/2 default-on.

## v0.1.0 — 2026-11-21

- Baseline sau pha 1 + VS3 wave 1: async frame test path (budget/cancel/chan0), TLS/keep-alive/cookie/auth,
  registry HTTP(S) + Ed25519 + GC/migrate, LSP tệp phụ/incremental, Windows fixes.
- Linux gate **3448 passed / 3 skipped**; Windows `cargo test --workspace` **3404 passed / 0 failed**
  (cây tương đương `777af63`); host suite **54/54**.
- **macOS ngoài phạm vi/chưa verify**. Cross-target wasm/Linux trên psvn **BLOCKED-ENV** (thiếu std target).
- Runtime mặc định vẫn single-thread; async default **chưa flip**; HTTP/2 chưa có.

| Đợt | HEAD (mac) | Cổng `--fast` | Số task trong báo cáo |
|---|---|---|---|
| R1 | `b4d7d469` | PASS | 11 |
| R2 | `43107a40` | PASS | 20 |
| R3 | `d971db57` | PASS | 27 |
| R4 | `0463512b` | PASS | 36 |
| R5 | `10353111` | PASS | 49 |
| R6 | `e0f6c234` | PASS | 49 |
| R7 | `29c13395` | PASS | 53 |
| R8 | `bb8393fe` | PASS | 58 |
| R9 | `d3b11346` | PASS | 61 |
| R10 | `8e57eef2` | PASS | 85 |
| R11 | `2f490b41` | PASS | 86 |
| R12 | `8b234a1f` | PASS | 92 |
| R13 | `c1228f1c` | PASS | 100 |
| R14 | `5f068f77` | PASS | 104 |
| R15 | `d012624a` | PASS | 108 |
| R16 | `005b12a8` | PASS | 112 |
| R17 | `e446481c` | PASS | 117 |
| R18 | `637bbebb` | PASS | 121 |
| R19 | `4bb3d0c7` | PASS | 125 |
| R20 | `536f802d` | PASS | 127 |
| R21 | `fcf3ae63` | PASS | 130 |
| R22 | `87fd4f9d` | PASS | 134 |
| R23 | `37d57145` | PASS | 153 |
| R24 | `5e364155` | PASS | 156 |
| R25 | `43b19dfe` | PASS | 164 |
| R26 | `b1c07802` | PASS | 171 |
| R27 | `dd088322` | PASS | 174 |
| R28 | `fdb21d15` | PASS | 177 |
| R29 | `39041b3f` | PASS | 197 |
| R30 | `0d5e0e57` | PASS | 223 (222 DONE) |
| R31 | `31b51d2e` | PASS | 249 (baseline VD-242) |

> Ghi chú (2026-11-21): số API trong từng mục dưới đây là số tại **nhánh lúc viết**; sau merge lấy
> `agent/API-STD.txt` (hiện **163** mục). **R32 chưa có** — đợt A0 đã dừng @ `fc01b9a`;
> nhật ký đầy đủ ở `agent/archive/journal-pre-v040/vd260-report.md`; **VS3 mở** theo VD-271.

## Mốc đáng chú ý

Phần lịch sử **trước `v0.1.0`** (mốc phát triển, đo lường và quyết định cũ) đã tách sang
[`agent/archive/CHANGELOG-pre-v0.1.0.md`](agent/archive/CHANGELOG-pre-v0.1.0.md) — ST-310f,
giữ nguyên nội dung theo **VD-142**; không sửa bản lưu.
