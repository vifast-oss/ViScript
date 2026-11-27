<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ROADMAP bàn giao — dòng ViScript

> **TRẠNG THÁI HIỆN HÀNH — 2026-11-27 (`VD-289`/`VD-306` — CHẾ ĐỘ PHÁT TRIỂN KIẾN TRÚC TỐI ƯU; `ST-309` tạm dừng vô thời hạn).
> Đọc khối này trước.**
> - **Đã duyệt `DONE` (`VD-290` + `VD-291` + `VD-292` + `VD-293` + `VD-294` + `VD-295`, 2026-11-25..27, nền `8ff19b42` / mã `7014fb7b` / tích hợp Sóng 5 `882b7ffa` / `main` @`4e03a19c`)**: hoàn tất Kiến trúc 5 Tầng DAG (`ST-312a..ST-313d`), hậu-kiểm `ST-314a`, trọn **Sóng 4 (`ST-315a..ST-315d DONE`)**, trọn **Sóng 5 (`ST-316a`, `ST-316b`, `ST-316d` DONE)** và **trọn Sóng 6 (`ST-317a`/`ST-317b`/`ST-317c` `DONE`)** — 23 crate/5 tầng (`vis-dag-check` + self-test), R1..R10 PASS (0 `unsafe`, R8 `39 ≤ 40`), `StdId` **167/167**, corpus **568/568** + WASI **395×2** + sem **390/390**; `vis_runtime::value_ops` một nguồn + true O(1) CoW `Rc::ptr_eq` cho `Map`/`List` (0 đổi VBC1), release `vis-bench` **writebench ~240×** / **listbench 13–21×**; đóng `#199` (25k `const` 1,604 → **0,196 s**), `#48`, `#7`, `#186`, `#145`, `#47`; DAG cấm `vis-host-native → vis-runtime` (+self-test), đóng `#198` (0 binary R8 mới), tách `host_parity.rs` 1.457 → 4 tệp < 500 dòng. Tích hợp gate **3.880/3.880 (4 skip)** + clippy exit 0 + `vis-diff 100k lech=0`; Windows full gần nhất **3.779/0/4 @`7014fb7b`**; macOS full `mps` còn nợ ở `ST-316c`. Sổ nợ: **`#200`** (P2 lệch interp↔VM `main` nhiều tham số `int`) & **`#201`** (P3 `argv` gán vào tham số `int` không kiểm kiểu) còn `mở`, VS3/correctness; **`VD-294`** phê chuẩn **`ST-317a`** (Value **64 → 32 B**, gate **3886/3886**, conformance **568/568**, `size_of::<Value>() == 32`) và **`ST-317c`** (StdId **167**, `lsp_it` **158/158**, `API-STD.txt` không đổi) `DONE`; đăng ký **`#202`** (P2 flake `host_parity::net_http`, stdout chứa ms thật); **`VD-295`** phê chuẩn **`ST-317b`** (worker orchestration một nguồn `orchestrate.rs` **135**, `value/mod.rs` **748**, `sqlite/mod.rs` **282**, `vis-lsp/lib.rs` **236**; gate **3906/3906 (5 ignored)** ≡ 3901/3901+4 skip, diff 100k `lech=0` 56,5 s, clippy 0 warning, W **10/10**) — **Sóng 6 đủ 3/3 `DONE`**; đăng ký **`#203`** (W2 frame W=1 lệch interp↔VM) & **`#204`** (non-frame W≥2 nuốt lỗi task) — sổ **205 = 85 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng**.
> - **Sóng 7 — `DONE` 9/9 (`VD-296`..`VD-299`, `main` @ `2cda3a68`)**: `ST-318a` (đóng `#203`/`#204`; frame W=1 parity + non-frame W≥2 `VS0506`; audit F3 PASS `5e79bf54`) + `ST-318h` (`vis-interp/src/frame.rs` 1030→**159** + 4 tệp; audit PASS `03c1dc1f`) + `ST-318b/c/d/e/f/g/i` (audit PASS từng lát); **R1: 0 tệp > 1.000** (max **1000**, `vis-cli/src/cli/run_cmd.rs`), 0 tệp > 1.500; sổ **208 = 85 mở / 104 đóng / 16 VS3 / 2 không làm / 1 điều hướng**; cổng cuối `main` **3912/3912 (4 skip)**, diff 100k `lech=0` 84,9 s, clippy/rules exit 0, R8 **39/40**, 0 `unsafe`; nợ mới `#206`/`#207`/`#208` (`VD-299`).
> - **Sóng 8 — `DONE` 5/5 (`VD-300`..`VD-303`, `main` @`ee8f252d`)**: `ST-319b` **`DONE`** (#207 đóng; merge `eb7f5830`, PASS `4dff8b31`), `ST-319c` **`DONE (chẩn đoán)`** (#208 mở ở `ST-319c`, đóng ở `ST-319d`, design `design-208-codegen.md`), `ST-319a` **`DONE`** (merge `07a5e2b3`, PASS `b2033ebd`; `#206` đóng), `ST-319c2` **`DONE`** (`#209` đóng; merge `1ec1105a`, PASS `afed0250`), `ST-319d` **`DONE`** (`#208` đóng; merge `ee8f252d`, PASS `64e84b25`; 32 test, gate 3929), `ST-319e` **`DONE`** (`#210` đóng; merge `a9c02abb`, PASS `4baa4d2d`; env module, gate 3922); WASI **398×2 (371+27)** — final2 `60064a5c…996bbb` @`8c883a16`; cổng cuối **3934/3934** (4 skip), diff 100k `lech=0` 79,7 s; sổ hiện hành **210 = 82 mở / 109 đóng / 16 VS3 / 2 không làm / 1 điều hướng**.
> - **Sóng 9 nhỏ — `DONE` 2/2 (`VD-304` mở, `VD-305` chốt; nền merge `56d349a6`)**: `ST-320a` **`DONE`** (audit PASS `2f60d023`, code `f133aea6`, 35 test nhánh, **`#211` đóng**) + `ST-320b` **`DONE`** (PASS `05dd131e`, F1 `7c807ba8`/F2 `985b0883`, **`#212` đóng**); merge `56d349a6`, cây gộp **38 test** (32+3+3, w25/w26/w27), blob **`61a82fd7`**, gate **`3940/3940`** (4 skip), diff 100k `lech=0` (143,5 s), clippy/rules exit 0. **`VD-305`** numbering R1 (ballot `b2fc7e12…`): `#213` free HOF / `#214` rò param / `#215` test W>1 **mở**, giao Sóng 10; sổ sau `VD-305` **215 = 84 mở / 112 đóng** (`81+3`/`110+2`/`210+5`; không lưu hành `85/111` hay `84/113`).
> - **Sóng 10 — `DONE` 4/4 (`VD-306`; `main` @`0b32c5dc`, gate **`3942/3942`** 4 skip / **40 test**)**: `ST-321a` **`DONE`** (merge `5d57d8e4`: tyck chữ ký `main` `VS0412`/`VS0401`, corpus **573/573**, **`#200`/`#201` đóng**); `ST-321b` **`DONE`** (merge `22d7108b`: journal **568/568** + `vistest` `.trap`/`.out_prefix`/`.exit`); `ST-321c` **`DONE`** (merge `0b32c5dc`: errno `ECONNREFUSED` 61/111 theo `target_os`); `ST-321d` **`DONE`** (merge `39effc26`: `cfg(unix)` import `Command` Windows); diff 100k `lech=0`, clippy/rules exit 0, R8 **39/40**. Sổ hiện hành **215 = 82 mở / 114 đóng / 16 VS3 / 2 không làm / 1 điều hướng** (`84−2`/`112+2`, tổng 215 giữ nguyên; `84/112`, `84/113` là số cũ/lịch sử).
> - **Sóng 11 — `DONE` 2/2 (`ST-322a`/`ST-322b`; code HEAD `0dc9207a` / docs HEAD `12cf227c`, gate **3942/3942** 4 skip / **40 test**)**: `ST-322a` R1 headroom `vis-cli/src/cli/run_cmd.rs` **1000 → 872** (tách `run_cmd/ho_tro.rs`, conformance **573/573**, diff 100k `lech=0`, merge `54e96658`); `ST-322b` sửa import dư `ngau_nhien_os` cho Windows tại `vis-host-native/src/db/sqlite/mod.rs` (merge `0dc9207a`, gate/clippy PASS). **Không cấp số nợ mới** — sổ giữ **215 = 82 mở / 114 đóng / 16 VS3 / 2 không làm / 1 điều hướng**. R8 **39/40**, 0 tệp > 1.000.
> - **Trục hiện hành — Sóng 11 `DONE` + `mps` peak2 PASS + `psvn` peak3 PASS + `mps` peak4 chạy + `ST-316c` `BLOCKED`**: phần `ps` đã xong — **WASI số mới nhất 400×2 (373 `.out` + 27 `.trap`)** @`304cb610`, sem **390/390**, suite **28/28**, log `/home/ps/vislogs/wasi-final-304cb610-20261127T032131Z.log` sha `9222838801ef23b2bc6ed9b0a6aa4fa8c58b3fa769b1520c4ad9570f6dad962f` (mốc cũ **398×2 (371+27)** @`8c883a16` là lịch sử); `bin/vis-targets` nhánh không `--wasm` đã cập nhật mốc hiển thị (docs-only, nhánh `--wasm` giữ nguyên đo thật). **Bench A/B PASS** @`304cb610`: paired 50 ô trung vị **Δmed = Δp90 = 0,0 %**, stdout deterministic 0 lệch; ô lẻ `opt-bench` interp **+11,7 % median** chỉ là follow-up "theo dõi" (A/A phẳng **0,0 %**, focused 10 vòng −0,6 %) — không có bằng chứng hồi quy có ý nghĩa (`wasi-bench-304cb610-20261127T032624Z.md`). Chain peak1 `/home/ps/vislogs/peak1-2026-11-27-chain.sh` sha `e239c317…` chạy 01:00Z trên `ff585af8`: **exit=1 — 16/18 PASS, 2 FAIL** (`psvn-gate-full` exit 101 `Command`, `mps-gate-full` exit 1 `net_loopback_parity` 61/111). Re-run peak2 @`304cb610`: **`mps-gate-full` PASS exit 0** (`peak2-mps-result-20261127T0320Z.md`, `net_loopback_parity` PASS) — fix `ST-321c` xác nhận trên macOS; **`psvn-gate-full` vẫn FAIL exit 101** (`peak2-psvn-result-20261127T0320Z.md`) do import dư `ngau_nhien_os`, đã vá bởi `ST-322b`; **`psvn` peak3 PASS exit 0 @code HEAD `0dc9207a`** (`peak3-psvn-result-20261127T033939Z.md`, sha log `a45b0024…`, 3863 passed / 0 failed, `host_conformance` 54 ca 0 lệch); **`mps` peak4 đang chạy @docs HEAD `12cf227c`** (code HEAD `0dc9207a`, **chưa có kết quả**) ⇒ `ST-316c`/`ST-309` giữ `BLOCKED`, **không** tuyên bố 3-OS/tag. **`R39` đã nộp** `304cb610` (`agent/reports/R39-viscript.{md,json}`, report-check **37 báo cáo / 386 hàng QUEUE**); `R38` giữ nguyên.
> - **Nền kỹ thuật đã đạt trước `VD-289`**: **3-OS Tier 1** (Linux, Windows x86_64 `ST-298`, macOS arm64 `ST-300m3`), WASI `wasm32-wasip1` runtime, kiến trúc 4 tầng `v0.5.5` (`3d85bd2`), bảo mật & hygiene `ST-311a..f` (đóng `#190`/`#194`, vá `#195`, 0 `unsafe`, R1..R9 PASS).
> - **Tag lịch sử đã chốt**: `v0.5.5` @ `3d85bd2`, `v0.5.0` @ `c06d7c6` (`R36`), `v0.4.5` @ `3daff62` (`R35`), `v0.4.1` @ `90844bc`, `v0.4.0` @ `c2accdf`. Các khối dated bên dưới là **lịch sử**, số cũ giữ nguyên (`VD-142`).
> - **Đọc chi tiết hiện hành**: [`ACTIVE.md`](ACTIVE.md), [`QUEUE.md`](QUEUE.md) (`ST-315a..ST-322b`), [`JOURNAL/quyet-dinh-a0-2026-11-27-vd305.md`](JOURNAL/quyet-dinh-a0-2026-11-27-vd305.md) (`VD-305`: chốt Sóng 9 + numbering); `R39` đã nộp `304cb610` (report-check **37/386**).

> **`v0.5.0` prep — 2026-11-23 (ST-305, chưa tag)** — workspace bump **0.5.0** (mọi crate
> `version.workspace = true`); `vis --version` ⇒ **`vis 0.5.0`**; `PHIEN_BAN`
> `+v050-release-2026-11-23`, khoá cache mới
> `fc20a81123cdb432a976e7d3d1fa15b7f4a2dd3b2276241bd0888e55bda53117`; `assert_ne` khoá 0.4.5
> `d92f31c2…` ⇒ cache cũ tự vô hiệu; `Cargo.lock` + `vis-host-wasm/Cargo.lock` đồng bộ 0.5.0.
> Bằng chứng GĐ 3: gate Linux **3.747/4**; `ST-302`/`302b` (`#13`/`#8` + spawn method),
> `ST-304`/`304b` (`vish-edit` 17/17), `ST-305` (`lsp_it` 155/155, `vis-lsp` 29/29, VS Code),
> `ST-306`/`307` (plugin tĩnh/WASI, Windows quick **91/91**), `ST-308` (`vish-term`).
> **macOS arm64 Tier 1: ĐÃ công bố** (`ST-300m3`: 3× full + soak 4 h **1.878 vòng** +
> `vis-wasm-sem` 388/390) — xem khối đầu tệp.
> **Chưa tạo tag, chưa merge `plan/v050-prep`.**

> **`v0.4.5` — 2026-11-23 (ST-298/VD-283, tag trên commit R35)** — workspace bump **0.4.5**;
> cache `PHIEN_BAN` `+v045-release-2026-11-23`, khoá mới
> `d92f31c28ed5e97a07976b83be0fb6a3c4c3914120e4e8f98e9fc832685418ce`; vá `bin/vis-dev` fuzz theo
> `CARGO_TARGET_DIR`; `vis-host-wasm/Cargo.lock` đồng bộ 0.4.5. **Windows Tier 1: đã công bố**
> — chờ soak `985` vòng / `0` fail / fd-RSS `free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0` / free `8204–8766 MB`.
>
> **Cập nhật 2026-11-23 (hậu tag `v0.4.1` + xác nhận Windows ST-297)** — **`v0.4.1` đã tag
> @ `90844bc`** (annotated, pushed); **`v0.4.0` @ `c2accdf`**. **ST-297** đã merge `f4cb136`:
> Windows **3× full 3611/0/3** (4 ca mới UNC/mixed/percent + `--grant` Windows), host 54/54,
> targets **18/18 + 17/1**; Linux **10×** `vis-lsp` **29/29**, `path_impl` **2/2** (`8b03077`);
> **`#149` đóng**. **ST-299b** đã merge `3b8049c` (`vis-loader` đa tệp trong WASI; native 29/29,
> wasm 28/28). **P2b-3/4 NO-GO** `16fba3a` (bench 4,0×, không đổi production); flake fix `044b289`.
> **ST-298** @ `f4cb136`: **3× `vis-gate-windows --full` PASS 3611/0/3** (host 54/54, targets
> 18/18 + 17/1, dirty 0); **soak 4h đang chạy** `985`/`0`/`free RAM 8204–8766 MB; proc/threads 0/0 sau mỗi vòng; fd/handle không đo được sau soak (NO_PROC; không còn tiến trình vis/cargo/rustc); dirty cuối 0`;
> **chưa** tuyên bố Windows Tier 1. **ST-300 DONE/merged** `3581c9a`+`8286caf`: `bin/vis-wasm-sem`
> **388/390 eligible = 99,5%** (2 stack-overflow interp, VM pass), suite cũ **28/28**, native
> **29/29**, row `wasm32-wasip1 (runtime)` trong `vis-targets`. **Final Linux soak**
> `45122b7`/`84e8eb6`: **10/10** (759 s), 3× diff 0, host 54/54, cache 5/5, wasm 28/28.
> **Cập nhật 2026-11-23 (`v0.4.0` — R33 CHỐT)** — workspace **0.4.0** (`version.workspace`),
> `vis --version` = `vis 0.4.0`; cache `PHIEN_BAN` ghim lại (`+v040-release-2026-11-23`).
> Linux final QA @ `e47b3ca`: gate **3656/4**, release **3661/0/4**, 2× soak **10/10**; Windows
> @ `e47b3ca`: full **3607/0/3**, release **3606/0/3**; Windows release `0.4.0` @ `8696edc`
> (`vis.exe` 11.530.240 B, sha256 `3813a1b3…cb065`). **Tag `v0.4.0` trên commit R33**; tag trước
> **`v0.3.6` @ `8fa2c09`**. Hai nhánh **READY chưa merge** — `plan/w4-st296a-tests` @ `f961b83`,
> `plan/w3-st296b-crates` @ `7c4c205` — **merge sau tag**.
> **Cập nhật 2026-11-22 (VD-281 — Duyệt `ST-290`/`ST-294` DONE, Chèn GĐ 1.5 `v0.4.1` Tối ưu Repo & DevEx)** — A0 duyệt **`ST-290` (`a48d5b87`, budget 40M + `VS0509`/`VS0510`)** và **`ST-294` (`cce2d0fb`, `async-h2` Windows + default-on non-wasm, Linux 3.607/4, Windows 3.563/0/3)** → **`DONE`**. Bổ sung nối `SharedBudget` vào `ST-292`, fix chịu tải `vd270_test_slowest` + `TMPDIR` ngoài `/tmp` vào `ST-296`, chèn **GĐ 1.5 (`v0.4.1`, `ST-296a`..`ST-296d`)** (gom 278 test binary → 12–15, tách `vis-loader`/`vis-lsp`, gom stdlib theo miền, dọn `agent/archive/` + `agent/ACTIVE.md` + `bin/vis-{gc-targets,jobs,status,gate-windows}`), tách `ST-304`/`ST-304b`, và chốt Plugin Phương án A (`Native static` + `WASI/WASM dynamic`, 0 `unsafe`). Chi tiết: `agent/archive/journal-pre-v040/quyet-dinh-a0-2026-11-22-vd281.md`, bảng điều khiển nhanh: `agent/ACTIVE.md`.
>
> **Cập nhật 2026-11-22 (VD-280 — Duyệt đóng GĐ 0 `v0.3.6`, MỞ GĐ 1 `v0.4.0`)** — A0 duyệt `APPROVED` toàn bộ `ST-284`..`ST-289` → **`DONE`** (tag `v0.3.6` @ `8fa2c09`, Windows tag verify `be51ae09`: `3538/0/3`, cancel `10/10`, host `54/54`). Mở **Giai đoạn 1 (`v0.4.0`)** với phân luồng tái cân bằng (`W1`: `ST-291`→`ST-292`; `W2`: `ST-294` + `ST-295`; `W3`: `ST-290`→`ST-293`; `W4`: nightly bù R32 + `#191` sớm + `ST-296`/`R33`), rào R1 đợt 2 và đăng ký chung `VS0509`/`VS0510` ở `ST-290`. Chi tiết: `agent/archive/journal-pre-v040/quyet-dinh-a0-2026-11-22-vd280.md`.
>
> **Cập nhật 2026-11-22 (Stage 0 `v0.3.6` @ `8fa2c09` — ĐÃ TAG)** — Giai đoạn 0 **đủ 6/6**:
> **DONE** `ST-284` (`83a9a10`/merge `e363a15`: `vm.rs` 1240, `frame.rs` 1042), `ST-285`
> (`ab5d292`/merge `246b6c5`: `value` 1186, `codegen` 1045, `tyck/expr` 1064, `interp/goi` 1155),
> `ST-286` (`f193c57`/merge `cfba662`: `async_net` 606, `quyen` 1190, `sqlite` 1245), `ST-287`
> (`03ed84c`/merge `f8f1d4b`: hook huỷ HTTP h1/h2 — `vs3_http_cancel` h1 **5/5**, h2 **8/8**),
> `ST-288` (`c513e7a`/merge `545a38d`: `VS0708` + `#142`; **Gatekeeper DONE 2026-11-22**), `ST-289`
> (`ff55056`/merge `84915aa`: `0.3.6`, `#111`/`#151`, khung R32). **Final QA** (fresh-clone
> `4c5dec6`): gate **3581/3**, release workspace **3586/0/3**, diff 100k seed `202611221700`
> **0 lệch**, **2× soak = 20/20 vòng** (593 s + 602 s); P2/P1/P3 14/8/10, h2 19 + feature 34+1,
> `pkg` 25, sig 10, http_cancel 5+8, host 54/54. Main merge **`a1930cb`**; R32 chốt phần Stage 0.
> **Windows final @ `a1930cb`**: workspace lần 2 **3538/0/3** nhưng `vs3_http_cancel` h1 **flake ~21 %**
> (`V:post` sau 10053); **`v0.3.6` đã tag @ `8fa2c09`** (ST-287 fix xong; Linux 3581/3, Windows 3538/0); GĐ 1..4 vẫn
> `BLOCKED` chờ A0 mở.
>
> **Cập nhật 2026-11-22 (VD-272..VD-279)** — Chốt **kế hoạch dài hạn hậu `v0.3.5`** (`v0.3.6` R0 Refactor & Hygiene → `v0.4.0` G4a Core → `v0.4.5` Windows Tier 1 & WASI → `v0.5.0` Pha 2 Ngôn ngữ & ViShell → `v0.6.0`/`v1.0.0-rc1` Plugin ABI & ViTerm) và **quy trình vai trò độc lập môi trường `VD-279`**. Nguồn quyết định: `agent/archive/journal-pre-v040/quyet-dinh-a0-2026-11-22.md`, `agent/DECISIONS.md` (`VD-272`..`VD-279`); hàng việc: `agent/QUEUE.md` (`ST-284`..`ST-309`; GĐ 0 `ST-284`..`ST-289`, GĐ 1..4 `BLOCKED` chờ cổng chặn cứng R0).
>
> **Cập nhật 2026-11-21 (VD-271)** — **VS3 ĐÃ MỞ**. Kế hoạch đầy đủ: `KE-HOACH-VS3-2026-11-21.md`; tiến độ: `VS3-TIEN-DO.md`. Phạm vi **tất cả tính năng** (async, kênh, net/TLS, quyền, LSP, gói, nền tảng, host, plugin/ViTerm), triển khai theo lát cắt **G0→G5**; **macOS ngoài phạm vi** *(lịch sử — `VD-284` đã đưa macOS arm64 vào phạm vi)*; **batch test 4 giờ** (không big-bang). Tag mốc: **`v0.3.5`** @ `03bb7b9`; `v0.3.4` @ `8fa1f96`, `v0.3.3` @ `bbf3484`, `v0.3.2` @ `864cc96`, `v0.3.1` @ `e689c3d`, `v0.3.0` @ `0d26527`, `v0.2.0` @ `42b700c`, `v0.1.0` @ `50675dd` giữ lịch sử; `pha1-rc1` giữ nguyên. Runtime async default đã flip (v0.2.0); HTTP/2 default-on (v0.3.0).
>
> **Cập nhật 2026-11-21 (VD-270)** — A0 tiếp tục đợt **tối ưu liên tục** VD-258..VD-270 (uỷ quyền đến 09:00 23/11); **chưa** mở pha 2/VS3, **chưa** tag mới (`pha1-rc1` = `05e42d0` giữ nguyên), runtime mặc định **không đổi**. HEAD lúc ghi `745851a` (08:04 21/11); số đo mới nhất trong `CHANGELOG.md`: `gate --fast` **2840 passed / 0 skipped**, `vis test test/sem` **525/525** cả interp và VM; core-test/release/toolchain/host 54/54 PASS. API-STD **136** mục. Số đo sẽ còn tăng trong đợt. Windows Tier 2 (xanh @ `c7cef71`, **chưa verify lại ở HEAD**); macOS **chưa verify**. **Đọc khối “Trạng thái sau VD-270” ở cuối tệp**; các khối R22/R23/R26/R27/R29 trong thân là lịch sử, số cũ không xoá (VD-142). Chi tiết từng mục: `JOURNAL/vd260-report.md`.
>
> **Cập nhật 2026-11-20 (VD-242)**: A0 mở đợt **“Ổn định sau pha 1”** — baseline `31b51d2`; chưa mở pha 2/VS3. Duyệt cả 10 parity + triage đủ 18 std/CLI; ngữ nghĩa chốt trong `DECISIONS.md` VD-242; bốn luồng Agent 2 tại `QUEUE.md` ST-237..ST-243. Tag `pha1-rc1` giữ nguyên; macOS chưa verify; Windows Tier 2 + verify commit cuối; RFC typed/CSV/str hoãn. Điều kiện mở pha 2 ở VD-242 §8. Các cập nhật dưới đây là lịch sử. **VD-243**: #172 `percent` báo `VS0507`, không bão hoà; gỡ blocker quyết định, ST-240 được chuẩn bị/sửa khi không xung đột tệp.

> **Cập nhật 2026-11-20**: HEAD `7e23c59` (origin/main). VD-winport DONE kỹ thuật (A0 VD-241); Windows verify exit 0 @ c7cef71/2d93101/5f4edd7/c12979f/da24c7d; Tier 2 giữ, Tier 1 không tuyên bố; pha1-rc1 nguyên; narrow freeze +9 std.sqlite.*; macOS không có máy. Đọc `agent/SNAPSHOT-2026-11-20.md`. VD-winport mở theo lệnh người dùng; Linux xanh toàn workspace, gate/nightly PASS (@ `f9430b9`: fuzz 18,769,716 ca); **Windows**: verify @ `6ccf367` → 7 suite đỏ → fix @ `f9430b9` → 1 ca → fix @ `c7cef71` → **exit 0 — toàn workspace xanh** (150 khối ok); Tier 2 giữ nguyên, chưa tuyên bố; `pha1-rc1` nguyên; narrow freeze chỉ `+9 std.sqlite.*`; macOS chưa có kết luận (mps bận). Đọc **`agent/SNAPSHOT-2026-11-20.md`** để biết đầy đủ trạng thái trước khi bắt đầu.
> **Cập nhật 2026-11-19**: đọc **khối cuối tệp** (“Trạng thái cuối sau R30”) trước; **pha 1 đóng trên ps** tại HEAD `49b33b8` (ST-205…ST-211, ST-226, ST-228 + VD-215..VD-228); mac chờ + ratify A0. Lưu ý nội dung thân tệp (các khối R22/R23/R25/R26/R29 trong thân) giữ nguyên lịch sử; mọi số R22 đã lỗi thời.

Tài liệu một trang cho người tiếp nhận: 8 hạng mục VS2, mức thật **đo được**, điều kiện đóng 100%,
nơi đọc và lệnh chạy. Không mục nào “xong” nếu chưa đạt cột điều kiện. Số đo cuối: xem khối
**“Trạng thái sau VD-270”** ở cuối tệp (R31 là baseline 20/11); các số R22 trong thân tệp là **lịch sử**.

## A. Tám hạng mục VS2 — mức thật cuối (không vượt mức)

| # | Hạng mục | Mức thật (đo được) | Bằng chứng | Còn thiếu để đóng 100% |
|---|---|---|---|---|
| 1 | **Async** | `async fn`/`await`/`spawn`/join; scheduler **một luồng, run-to-completion**, **không song song thật**, **không GC**; 10k task interp **316 ms** / VM **168 ms**; drain 100 đúng thứ tự | ST-60/61/94/98; `async_10k`; R22 §3 | **Song song thật**/work-stealing + suspend-resume; test tăng tốc ≥2 core; async method/cmd; GC/ownership |
| 2 | **Kênh** | `chan(n≥1)` send/recv/close; **`chan(0)` bị từ chối** `VS0505` ngay khi tạo (**không** rendezvous); `recv` sau close `none`; `send` sau close `VS0504`; deadlock `VS0505` không treo | ST-61/94; `kenh.rs`/`kenh_cap0.rs`; soak 10/10 | **Wait-set/rendezvous** `chan(0)` (hoặc tuyên bố từ chối vĩnh viễn có văn bản); `select`/timeout; test hai đường |
| 3 | **Net/HTTP** | Loopback TCP thật; **chunked** (+`;ext`, trailer bỏ), **redirect GET ≤5** (Location `..`/query-only/fragment/rỗng), **`http.head` không thân/không redirect**, timeout đọc 2 s, cap 8 MiB; **không TLS/HTTP2/keep-alive**; 21 ca http | ST-64/70/71/104/109; `net.rs` 463; R22 §3 | **TLS** (duyệt crate rustls/webpki), HTTP2, keep-alive, cookie/auth, HEAD redirect; ≥2 kết nối đồng thời; test Internet thật |
| 4 | **Quyền** | `cap[Dir/File/Net/Proc]`; `--strict-caps`; `--grant` Dir/File/Net = đường dẫn, **Proc = allowlist** (basename/canonical); `VS0604` exit 2; **chưa siết mặc định**; **chưa Landlock thật**; con **không sandbox** | ST-72b/81/93/97; `quyen.rs` 460 | **Landlock thật** Linux (duyệt crate `landlock` **hoặc** `unsafe` đóng) + **runner Linux**; siết mặc định; sandbox con |
| 5 | **LSP** | `--stdio`; diagnostics **module-aware**, UTF-16; **4 ca chẩn đoán đa tệp đúng tài liệu mở** (ST-117); completion sau `.`; **hover trường** + `didClose`; cache module phụ **0,466 ms** | ST-91/100/117; `lsp_std*.rs` 43 + `lsp_da_tep.rs` 4; R22 §3 | **Publish chẩn đoán tệp phụ**, hover method, generic/`list[P]`, bảng `std.str`, incremental sync |
| 6 | **Gói** | Registry **local**, manifest `files[]`, lock SHA-256, `.vis/store`, `use pkg.*`, `VS0701/0702`, lock `entry`, `pkg update` idempotent, `list --json`, `remove`; **chưa** resolver/HTTP/ký số | ST-64/66/69/80/90/102; `pkg.rs` 927 | **Semver ranges** + resolver; registry HTTP; chữ ký số; GC store; migration lock cũ |
| 7 | **Đa nền tảng** | **Chỉ `cargo check`**: ma trận **18/18 `PASS`**; **chưa runner** Windows/Linux/wasm | ST-58/73/86; `vis-targets` 18/18; R22 §3 | **Runner thật**: wasm32-wasip1 (wasmtime), Linux (Landlock), Windows; CI 54 host trên ≥2 nền |
| 8 | **Hợp chuẩn host** | **54 ca chạy thật macOS ×2, 0 lệch**; **VM 54/54**; cache **520×4 0 lệch**; net **không** nằm trong suite | ST-87/88/89/98; `HOST-BAO-DAM.md`; soak 10/10 | 54 ca × **4 nền thật**; net/TLS trong suite; CI lặp; đo hiệu năng từng nền |

### Quyết định Agent 0 cho điều kiện đóng (2026-11-16, VD-119..VD-131)

| # | Hạng mục | Trạng thái sau quyết định | Việc |
|---|---|---|---|
| 1 | Async | Chưa có trong VS2: song song, suspend, GC (VD-123) | VS3 thiết kế riêng |
| 2 | Kênh | Chưa có trong VS2: rendezvous, select, timeout (VD-123) | VS3 |
| 3 | Net/HTTP | Chưa có trong VS2: TLS, HTTP/2, keep-alive (VD-122) | VS3 |
| 4 | Quyền | Siết khi dùng gói: ST-130. Landlock: ST-132, chờ runner Linux | ST-130, ST-131, ST-132 |
| 5 | LSP | Một tài liệu là quyết định cuối (VD-126); nợ #104 đóng | không |
| 6 | Gói | Ranges + resolver: ST-127..129. Mạng/chữ ký: VS3 | ST-127..129 |
| 7 | Đa nền tảng | Linux: ST-121, ST-131. Windows, wasm: chỉ check trong VS2 (VD-121) | ST-121, ST-131 |
| 8 | Hợp chuẩn host | Nền thứ hai = Linux qua ST-131 | ST-131 |

R1 cứng ≤1.500 dòng/tệp (VD-128); **mục tiêu nội bộ Sóng 7 <1.000 đã đóng**: **0 tệp > 1.000** (max **1000** lúc đóng Sóng 7, `vis-cli/src/cli/run_cmd.rs` — nay **872** sau `ST-322a` @`0dc9207a`; `vis-interp/src/frame.rs` **159**).

## B. Lệnh chạy (tuần tự, **không** song song build/cổng khác — AGENTS §6)

```bash
cd viscript
cargo test --workspace                  # 1066 test
cargo test -p vis-cli --test sem        # 447 ca / 22 nhóm
target/debug/vis test test/sem          # 447 ok (interp)
target/debug/vis test --vm test/sem     # 447 ok + 0 SKIP (VM)
../vis-dev gate --fast                  # fmt, clippy, R1..R4, L0, report-check
bin/vis-soak                            # 10 vòng: gate×3, diff×3, host×2, cache×2 (1297 s lần cuối)
VIS_DIFF_N=100000 VIS_DIFF_SEED=20261115 ./bin/vis-diff   # 0 lệch
target/release/vis-parse-fuzz --seconds 60 --seed 20261115 # 0 vi phạm
./bin/vis-host-suite                    # 54/54 macOS
./bin/vis-targets                       # 18/18 check (số R22, TRƯỚC SQLite); 2026-11-20: **16/18** — nợ #156
```

## C. Nợ correctness đa tệp — **ĐÃ ĐÓNG** (ST-112 → ST-114b)

- `Span.file` + `lex_file`/`parse_file`/`LowerOptions.file`; `Nap.file_rel` chính **0**, module theo `use`;
  tyck dùng `d.span.file` (fallback heuristic cũ **chỉ** ngoài bảng + log); `vi_tri` đúng văn bản tệp.
- **6 ca** `chan_doan_da_tep.rs` red-first (5/6 đỏ → 6/6 xanh) + 4 ca LSP `lsp_da_tep.rs`; JSON/TAP đúng
  tệp/dòng cho a/b/cả hai/TAP/b dài.

## D. Điều kiện đóng **100%** (giữ nguyên danh sách; thêm số liệu cuối)

1. **`Span` tổng hợp `file=0`** — audit các node lower sinh; LSP **chưa publish chẩn đoán tệp phụ**.
2. **Landlock thật** Linux (crate `landlock` **hoặc** `unsafe` được duyệt) + **runner Linux** 54 host.
3. **Runner wasm32-wasip1** (wasmtime) + **Windows**; CI 4 nền.
4. **TLS/HTTP2/keep-alive** + HEAD redirect/cookie; test Internet thật.
5. **Async song song thật** + wait-set/rendezvous `chan(0)` (hoặc tuyên bố vĩnh viễn có văn bản).
6. **Siết mặc định caps** + sandbox tiến trình con; cập nhật host suite.
7. **Resolver/registry HTTP/ký số** cho gói + GC store.
8. **R1 nội bộ <1.000 (Sóng 7, `DONE`)**: **0 tệp > 1.000** — max **1000** lúc đóng Sóng 7 (`vis-cli/src/cli/run_cmd.rs`, nay **872** sau `ST-322a`), `vis-interp/src/frame.rs` **1030 → 159**
   (+4 tệp, max 366); `lower/cmd` max **680**, `parse/mod.rs` **69**, `parse/cmd` **39**; **0 tệp > 1.500** (luật R1 cứng).
9. **100 nợ R21** + nợ mới đóng hết; **R23** xác nhận lại toàn bộ số đo.

**Hiện trạng R22**: **1066 test**, sem **447/22**, `--vm` **0 SKIP**, parity **398+49**, diff **0 lệch**,
cache **520×4 0 lệch**, host **54/54 ×2**, soak **10/10 (1297 s)**, fuzz **0 vi phạm**, `FUNCS` **52**,
`all_codes` **32**, `VBC1` **53**, targets **18/18 check**, R1 max **1000** (R22 lịch sử: 1060). **VS2 chưa đóng** — 9 điều kiện.

## E. Nơi đọc

- [`QUEUE.md`](QUEUE.md) **134 hàng**; [`DECISIONS.md`](DECISIONS.md) VD-01..VD-118.
- `reports/R1..R22` (bắt đầu R21/R22); [`HOST-BAO-DAM.md`](HOST-BAO-DAM.md) — host/HTTP.
- [`STATE.md`](STATE.md) — trạng thái hiện tại.

### Trạng thái cuối sau R23 (đo ở `c96506ba`, soak 10/10 = 1447 s)

| # | Hạng mục | Trạng thái (đúng 3 loại của VD-131) | Ghi chú |
|---|---|---|---|
| 1 | Async | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | Kênh | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 |
| 3 | Net/HTTP | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback; không TLS/HTTP2/keep-alive (VD-122) |
| 4 | Quyền | **macOS đã đóng · Linux CHƯA đóng** | Landlock thật 12/12 ca trên Linux (ST-132); **rò rỉ TOCTOU 798/50 000 trên Linux** ⇒ nợ #128 |
| 5 | LSP | **đã đóng** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | Gói | **đã đóng** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration; mạng/chữ ký = VS3 |
| 7 | Đa nền tảng | **một phần** | Linux **chạy thật** (ST-131); Windows/wasm **chỉ check** (VD-121) |
| 8 | Hợp chuẩn host | **đã đóng** | 54/54 ca trên **cả** macOS và Linux, 0 lệch |

**Số khoá R23**: 1153 test (129 suite) macOS · 1147/6 Linux · sem 447/22 · `vis test` 447 ok · `--vm` 447 ok + 0 SKIP ·
differential 100k `lech=0` (3 lần: 174/162/181 s) · cache 520×4 `lech=0` · host 54/54 hai nền · targets 18/18 `check` ·
R1 max **1498** · FUNCS **52** · all_codes **33** · VBC1 **53** · allowlist **6** · soak **10/10 = 1447 s**.

### Trạng thái cuối **sau đợt 25** — R26, `head` `b1c07802` (2026-11-17)

> Khối này **thêm** sau khối R23; số cũ **không xoá** (VD-142). Nguồn: `reports/R26-viscript.md`.

**VS2 đóng về kỹ thuật 8/8 hạng mục** (**VD-151**, 2026-11-17). Từ đây **không** mở task VS2 mới trừ: nợ đã có
số trong [`SO-NO.md`](SO-NO.md), đính chính tài liệu, và báo cáo vòng. **Không** bắt đầu tính năng **VS3** cho tới
khi Agent 0 ban hành **tài liệu thiết kế VS3** (VD-148 giữ nguyên; hiến chương VS3 = `JOURNAL/quyet-dinh-a0-2026-11-17.md` §5).

| # | Hạng mục | Trạng thái (3 loại của VD-131) | Ghi chú |
|---|---|---|---|
| 1 | Async | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | Kênh | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 |
| 3 | Net/HTTP | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback; không TLS/HTTP2/keep-alive (VD-122) |
| 4 | Quyền | **ĐÓNG** (macOS giới hạn VD-145) | Linux Landlock thật **14/14** ca (thêm ca âm **GHI** ở ST-150) + TOCTOU **0/50 000**; macOS bảo đảm theo **đo** |
| 5 | LSP | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | Gói | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2; mạng/chữ ký = VS3 |
| 7 | Đa nền tảng | **ĐÓNG theo nghĩa kỹ thuật** | Linux **"chạy thật"** (`ps` gate PASS L0 **1169/0**) · Windows **"chỉ đo"** · wasm **"chỉ check"** |
| 8 | Hợp chuẩn host | **ĐÓNG** | 54/54 trên **macOS và Linux**; `vis-host-linux` **14/14**; Windows **39/54** (chỉ đo) |

**Số khoá sau đợt 25** (R26 §3; log trong `JOURNAL/`, hash đo từng nền theo VD-147(4)):

```
macOS (@ b1c07802) : gate --fast PASS · L0 **1175/0** (**130** suite; = 1166 + 9 ca ST-151) · 10/10 bước
        vis test 447/447 ×2 (0 SKIP) · diff 100k lech=0 · cache 520×4 0 lệch · host 54/54 · targets 18/18
        soak 10/10 = 1352 s (**đo ở HEAD cũ** 3079dede — không chạy lại) · R1 1498 · FUNCS 52 · all_codes 33 · VBC1 53
Linux ps (@ 32b79d4a/4a805115): gate --fast PASS · L0 **1169/0** (129 suite) — **số TRƯỚC 9 ca ST-151, CHƯA đo lại**
        · cargo test 1162/0 (ST-140) · `vis-host-linux` **14/14** (@ 9eeb936b) · vis test 447/447 ×2 · diff lech=0
        · cache 0 lệch · host 54/54 · targets 18/18 · soak 10/10 = 250 s (@ 2866f8c2) · TOCTOU 0/50 000
Windows psvn (@ 3dc13342; cổng @ 4a805115) — **CHỈ ĐO, "chưa hỗ trợ"**:
        self-test **12/12** (7/12 → 11/12 → 12/12) · cổng đi **qua 9/10 bước**, dừng ở **L0** (fail-fast `cap_proc`
        **150/4** vì `/bin/echo` — VS3) · cargo test **1053/100** · host **39/54** · targets **18/18**
Sổ nợ : **144 mục** = **91 mở** / **31 đóng** / **20 VS3** / 1 không làm / 1 điều hướng (`SO-NO.md`; §8)
```

**Đợt 25 đã đóng**: **6/6 hàng** (ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-153) — nợ **#136**, **#133**,
**#135** và **ca 7 self-test của ST-147** (không cấp số mới); thêm số **#142/#143** (VD-149).
**Còn lại**: nợ trong sổ (**91 mở** / **20 VS3**) và **Windows** (hỗ trợ thật = **VS3**).

### Trạng thái cuối **sau R26** — đêm 17/11 (2026-11-17), `head` `dd088322`

> Khối này **thêm** sau khối R26; số cũ **không xoá** (VD-142). Nguồn: `reports/R27-viscript.md`.

**Phần còn lại sau R26, đêm 17/11 đóng nốt** (ST-152 = R26 · **ST-154** · **ST-155** · **ST-156** = R27):

| Việc | Nội dung | Trạng thái |
|---|---|---|
| Nợ **#144** | Ca **dương** `read_dir` trên thư mục **có quyền ghi** (`vis-host-linux` ca 15): `exit 0` + `co.txt`, đối chứng ghi `exit 0`, âm cùng ca `exit 7` + `EACCES` | **ĐÓNG** (ST-154) — `ps` **15/15** |
| Nợ **#143** | `NamePolicy::Strict` cho mọi tệp có `use` — điều kiện VD-133 đã thoả (ca `st130b` xanh) | **chuyển VS3** (VD-154(b)) |
| Nợ **#145** | Chú thích `rt.rs` nói **sai** đường chặn `chan(0)`: tyck **không** kiểm `cap`, runtime mới chặn (`exit 1`) | **mở** — xử khi có task VS3 kênh |
| **Bộ đầu vào VS3** | `JOURNAL/st155-vs3-dieu-tra.md` **222 dòng**: 6 mục × 5 khối, **57** dẫn chứng `tệp:dòng`, `[ƯL]` **19–28 task ≈ 57–84 h**, §8 Top 5 câu hỏi, §9 6 việc đề nghị KHÔNG làm | **ĐÃ CÓ** (ST-155) — **chưa** có tài liệu thiết kế ⇒ **chưa** mở tính năng VS3 (VD-148) |

| # | Hạng mục | Trạng thái (3 loại của VD-131) | Ghi chú |
|---|---|---|---|
| 1 | Async | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123) |
| 2 | Kênh | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3; **#145** ghi lệch tài liệu–mã |
| 3 | Net/HTTP | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback; không TLS/HTTP2/keep-alive (VD-122) |
| 4 | Quyền | **ĐÓNG** (macOS giới hạn VD-145) | Linux Landlock thật **15/15** (thêm ca **dương** `read_dir` — đóng #144) + TOCTOU **0/50 000**; macOS bảo đảm theo **đo** |
| 5 | LSP | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126) |
| 6 | Gói | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2; mạng/chữ ký = VS3 |
| 7 | Đa nền tảng | **ĐÓNG theo nghĩa kỹ thuật** | Linux **"chạy thật"** (`ps` gate PASS L0 **1169/0**) · Windows **"chỉ đo"** (**9/10 bước**, self-test **12/12**, dừng **L0**) · wasm **"chỉ check"** |
| 8 | Hợp chuẩn host | **ĐÓNG** | 54/54 trên **macOS và Linux**; `vis-host-linux` **15/15**; Windows **39/54** (chỉ đo) |

**Số khoá sau đêm 17/11** (R27 §3; log trong `JOURNAL/`, hash đo từng nền theo VD-147(4)):

```
macOS (@ dd088322) : gate --fast PASS · L0 **1175/0** (**130** suite) · 10/10 bước · check_report **27** báo cáo / **174** hàng
        vis test 447/447 ×2 (0 SKIP) · diff 100k lech=0 (157,4 s) · cache 520×4 0 lệch (25,43 s) · host 54/54 · targets 18/18
        soak 10/10 = 1352 s (**đo ở HEAD cũ** 3079dede — không chạy lại) · R1 1498 · FUNCS 52 · all_codes 33 · VBC1 53
Linux ps (@ 32b79d4a; Landlock @ d203b664; vis test @ ee58a518; soak @ 2866f8c2): gate --fast PASS · L0 **1169/0**
        (129 suite) — **số TRƯỚC 9 ca ST-151, CHƯA đo lại** · `vis-host-linux` **15/15** · vis test 447/447 ×2
        · diff lech=0 (40,9 s) · cache 0 lệch (6,33 s) · host 54/54 · targets 18/18 · soak 10/10 = 250 s · TOCTOU 0/50 000
Windows psvn (@ 3dc13342; cổng @ 4a805115) — **CHỈ ĐO, "chưa hỗ trợ"**:
        self-test **12/12** · cổng đi **qua 9/10 bước**, dừng ở **L0** (`cap_proc` **150/4** vì `/bin/echo` — VS3)
        · cargo test **1053/100** · host **39/54** · targets **18/18**
Sổ nợ : **146 mục** = **91 mở** / **32 đóng** / **21 VS3** / 1 không làm / 1 điều hướng (`SO-NO.md` §8; VD-155)
```

**Đêm 17/11 đã đóng**: **8 hàng công việc** (ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-153 · **ST-154** ·
**ST-155**) + **2 hàng báo cáo vòng** (ST-152 · ST-156); **không** hàng nào `BLOCKED`.
**Còn lại**: nợ trong sổ (**91 mở** / **21 VS3**) và **Windows** (hỗ trợ thật = **VS3**).
**Việc kế**: **Agent 0** trả lời **5 câu hỏi** (`st155` §8) rồi ban hành **tài liệu thiết kế VS3** (6 quyết định riêng).

### Trạng thái cuối **sau R29** — tách repo + pha 1 công thái học (2026-11-18, HEAD `2f62022`)

> Khối này **thêm** sau khối R27; số cũ **không xoá** (VD-142). Nguồn: `reports/R29-viscript.md`,
> `agent/JOURNAL/2026-11-17-*.md`, `agent/archive/journal-pre-v040/2026-11-18.md`, `git log` từ `39041b3f`.

**Đã đóng trong đợt R29 (17/11):** **ST-174** (vá nợ #146 — TOCTOU symlink, phương án A VD-176; ca A đỏ-trước 100/100,
sau vá 100/100 mac+ps, 6 000 000 vòng 0 đỏ; gate mac 3 lần PASS L0 **1184/0**) · **ST-175** (`vis-remote` từ chối cây bẩn) ·
**ST-176** (`vis-lock`; #147 mở: mac 1489 s vs ps 261 s = **5,70×** > 3× — chưa đóng) · **ST-178** (sửa 6 tài liệu VS3 theo VD-178,
0 dòng mã) · **ST-171** (**tách repo** — repo `https://github.com/vifast-oss/ViScript` public, 459 commit giữ lịch sử, gốc repo = cây
ViScript; repo cũ đã xoá cây `viscript/` + `VISCRIPT.md` trỏ sang).

**Pha 1 công thái học (đêm 17→18/11, theo C3 của VD-178) — đã làm:** ST-179 (P0a `newtype` báo lỗi dòng khai + bỏ “VS0”) ·
ST-180 (R19c float literal cần chữ số sau `.`) · ST-181 (P0-3 cheat sheet thành doc-test, 11 ca khoá) · ST-182 (R9 — 49 mã,
thoát **101**) · ST-183 (R6 — `p"…"`, `str+str`, format spec `{expr:spec}`, `print(a,b)`) · ST-184 (R17 — điều kiện chỉ `bool`) ·
ST-185/186 (R3b/R3a — `let … else`, `if let` nhận pattern) · ST-187 (R16 — `let` che tên) · ST-188 (R12a — `type X = T`, `type Email(str)`) ·
ST-189 (R11 — mọi hàm dựng là hàm) · ST-190 (R7 — `/` float, `//`, `%`) · ST-191 (R8 — `xs[i]?`) · ST-192 (R5a — named args, mặc định bản ghi) ·
ST-193 (R5c — method toàn std) · ST-194 (R1a — `VS0111` + fmt tự sửa `Some/Ok/Err`) · ST-195 (R20a — `assert cond, "msg"`) ·
ST-196 (R20b — `enumerate/any/all/find`) · ST-197 (review chéo R7/R8; hàng QUEUE bổ sung hồi tố 18/11) · ST-198 (PR #26 port — record ẩn danh in `{a: int}`) ·
ST-199 (PR #27 port — nhóm `( … )` đầu tệp `.vish`) · ST-200 (brand — icon ViScript/ViShell).

**Số đo (ST-198/199, `st198-199-gate-macos-lan2.log`)**: `gate --fast` **PASS** (10/10 bước) — **136 suite, 1211 test, 0 đỏ**;
**Số đo (ST-202, 2026-11-18)**: `gate --fast` **PASS** — **136 suite, 1215 test, 0 đỏ**; `vis test` **465/465** ×2 đường;
**Số đo (ST-203, 2026-11-18)**: `gate --fast` **PASS** — **136 suite, 1217 test, 0 đỏ**; `vis test` **469/469** ×2 đường;
`sem` **464** ca; check gate macOS @ R29 HEAD `39041b3f`: L0 **1184/0** (131 suite); ps clone sạch: L0 **1188/0**; Windows **chỉ đo**
(9/10 bước, L0 152/4 — VS3 mục 5, không tuyên bố hỗ trợ); VS2 **đóng 8/8** vẫn giữ nguyên (VD-151).

**Còn thiếu — pha 1 (C3):** ~~R1+R2~~ (R1a xong ở ST-194; R2 xong ở ST-203 — `none` bỏ, `T??`, fmt tự sửa; **enum-hoá `Option/Result`** — VD-208 chốt **A**: tầng chạy = enum std thật (done ở ST-217); tầng kiểu giữ dựng sẵn) ·
**R4** (`Error` record + `Json` enum + `AnyError`; mọi hàm std trả `Result[T, Error]`) · **R5b′** (huỷ R5b theo **VD-210** — giữ bản ghi ẩn danh, chốt quy tắc khoá `{…}` + gợi ý, ≤ 2 h) ·
**R5d** (chuẩn hoá `str(x)` vs method std.str) · ~~R19b~~ (đóng ở ST-202) · R20a phần 2 (`use std.fs as f`, `use std.fs.{..}`) ·
R20b phần 2 (`(1..5).step(n)`). Pha 2 (R13–R24) và VS3 **chưa mở** (VD-148; chờ pha 1 xong).

**Sổ nợ**: **147 mục = 89 mở / 35 đóng / 21 VS3 / 1 không làm / 1 điều hướng** (R29 §4; #147 mở chờ A0 — macOS phạt syscall là giới hạn nền hay gộp `report-check`/`rules`).

**Bất thường đã ghi nhận/bổ sung 18/11**: ST-197 thiếu hàng QUEUE khi làm (đã bổ sung hồi tố, commit `ed490d2`); `SO-NO.md` §8/header còn số cũ
(đã đồng bộ, commit `2f62022`); `ROADMAP.md` thiếu khối sau R29 (khối này); chưa có báo cáo **R30** cho đợt pha 1 (STATE/QUEUE là nguồn thật trong lúc chờ).

### Trạng thái cuối sau R30 — pha 1 đóng trên **ps** (2026-11-19 sáng, HEAD `49b33b8`)

> Khối này **thêm** sau khối R29; số cũ **không xoá** (VD-142). Nguồn: `BAO-CAO-A0-2026-11-19-0900.md`,
> `agent/DEM-2026-11-18.md`, `agent/archive/journal-pre-v040/2026-11-18.md`, `git log` từ commit `a570d77` (HEAD ban đầu của phiên, nay `d27e78d`).

**Đã đóng trong đợt R30 (18–19/11):** **ST-205** (R4a — `trait Error: Show` 3 phương thức mặc định-less; `Json` enum dựng sẵn
`EnumId(0)`; bản ghi `Error` `RecId(0)`; `AnyError` := alias bản ghi `Error` (**VD-231**, superseded VD-222 §5); `print(e)` chỉ `{message}`; std fns → `Result[T, Error]`) ·
**ST-206** (R4b — `?` giữ nguyên; VD-222 #5) · **ST-207** (R4c — migrate std; `vis test` 469/469 cả hai đường) ·
**ST-208** (R5b′ — quy tắc khoá `{…}` theo VD-215: trộn loại khoá bất kể thứ tự ⇒ **VS0207**, gợi ý theo khoá đầu; `{[k]: …}`
⇒ cảnh báo **VS0208** tắt bằng `{([k]): …}`; ca khoá `vd_215.rs` 6/6 hai đường) ·
**ST-209** (R5d — warnings `VS0211` khi khai báo trùng tên module std; `vd_223.rs` 5/5; VD-224 đảo VD-223(i)) ·
**ST-226** (R18b — `while let PAT = EXPR:`; `vd_226.rs` 8/8 hai đường byte-identical) ·
**VD-227** (interp tự nâng `T → T?` tại `let x: int? = expr`; mirror `Instr::MakeSome` — `vd_227.rs` 6/6 byte-identical) ·
**ST-211** (VS3-5-pre — `check.rs` root fallback `ten_goc()` tách cả `/` và `\\`; ca khoá `ten_goc_du`) ·
**ST-213** (VD-186 patches — `HOST-BAO-DAM.md` + `docs/ViScript-thiet-ke.md` Phụ lục C; nhãn `[thiết kế VS3-6, chưa triển khai]`).

**Ledger:** `VD-215` … `VD-228` ghi trong `DECISIONS.md`; `vd_205.rs`, `vd_206.rs`, `vd_213.rs`, `vd_215.rs`, `vd_216.rs`, `vd_218.rs`,
`vd_223.rs`, `vd_226.rs`, `vd_227.rs` tất cả xanh; `./vis-dev gate --fast` **PASS 10/10** mọi commit; `vis explain` = **53 mã**
(vắng `VS0206/VS0209/VS0210`); `api_dong_bang` (freeze) **xanh** — `FUNCS`+`CODES` khớp đúng;
**report-check**: **PASS 30 báo cáo, 242 hàng QUEUE phủ đủ**; clone sạch ps gate **PASS 10/10**; **nightly** PASS (fuzz 18,98 M ca, 0 violation).

**Ngắn gọn khi bàn giao tiếp theo (sau 9h sáng 2026-11-19):**

- **pha 1 đóng trên ps; mac chờ**: gate `--fast` ps **PASS 10/10**; mac không đo được trong khung; R30 không viết lại;
  A0 ratify VD-221..VD-228 (hoặc revert) bắt buộc trước khi nói “pha 1 đóng chính thức”.
- `**ST-212**` (VS3-QĐ-1 — async song song thật): **HOÃN** — `agent/vd212-thiet-ke-ghi-chu.md`; re-scope ước tính ~2 tuần.
- `**ST-228**` (proxy A0 — `fn f() -> int?:` return `T → T?` lift): **BLOCKED-CHO-A0** — Option A/B/C đề xuất.
- `**#147**`: ps đo đủ; mac chưa đo; ngưỡng 3× của VD-197 vẫn áp nếu mac ≥ 10 s.
- `**#149**`: `agent/PATH-SPLIT-AUDIT-2026-11-18.md` đã viết — `check.rs` sạch sau ST-211; các chỗ khác liệt kê ở audit, chỉ sửa khi đụng tệp.
- `**#151**`: VS3 pha 2 (fixture đặt tên theo số dòng — `VD-198`);
- `**#152**`: giữ “chờ đo chuẩn” (máy có tải, VM 1,129 / interp 1,109 — cả hai **CHƯA RÕ**);
- `**check_report.py --all**`: PASS 30 báo cáo · 242 hàng QUEUE.
- `vis` hiện thực đang chạy (phase 1) bằng `./vis-dev gate --fast` và `vis test test/sem` đều **xanh**.


### Đóng khung VD-220 ~09:00 (sáng 2026-11-19) — HEAD cuối khung `c98adb4`

> Khối này **thêm** (sau khối “sau R30”) theo VD-142 — số cũ không xoá.

**Chứng minh cuối khung** (fresh clone ps từ origin/main @ `0259e20`): gate **PASS** toàn bộ
(fmt, clippy, luật R1..R6, self-test case âm, build, doc-examples 24 khối, L0 tests);
`report-check` **PASS (30 báo cáo, 243 hàng QUEUE)**; `cargo test --workspace` **147 binary xanh / 0 đỏ**;
`vis test test/sem` **469/469** ×2 (interp, VM); `vis-dev nightly` **PASS** (fuzz 0 vi phạm; fmt vòng tròn bất động;
fuzz 10’ = **94 028 787 ca** ở `894e826`); `./vis explain --list` = **53 mã** khớp `all_codes()`;
`cargo doc --workspace --no-deps` exit 0 (22 warnings doc-link style, không mạch của session).

**Mã đã merge đợt này** (branch main): VD-222..VD-229, ST-205..ST-229
(recap: R4a/R4b/R4c, R5b′, R5d, while-let, VD-227, VD-228, VD-229, VD-218…); **R4a/R4b/R4c DONE**;
pha 1 **đóng trên ps** — mac chờ; **A0 ratify/revert VD-221..VD-229** — Q1..Q12 trong `BAO-CAO-A0-2026-11-19-0900.md`.

**Chờ ngoài khung** (không thuộc proxy authority để buột): mac measurement #147 (VD-197/206);
phase-1 đóng chính thức trên mac; ST-212 async song song thật (deferred, `agent/vd212-thiet-ke-ghi-chu.md`);
#151 VS3 pha 2; #152 “chờ đo chuẩn” (VD-217: VM 1,154 / interp 1,108 — khoảng cắt 1,10); #149 (path-split — audit liệt kê, sửa khi đụng tệp).

### Đóng pha 1 chính thức (2026-11-19 ~10:55, VD-237) — tag `pha1-rc1` == `05e42d0`

> Khối **thêm** sau khối “Đóng khung VD-220 ~09:00” (VD-142); số cũ không xoá; VD-236 phê duyệt addendum thay rewrite R30.
**Cả hai máy `ps` (Linux) và `mps` (arm64 mac)**
- gate `--fast` **PASS 10/10** (clone sạch, checkout `pha1-rc1`)
- `vis test test/sem` **469/469** interp
- `vis test --vm test/sem` **469/469** VM
- `cargo test --workspace --no-fail-fast` **147 binary test xanh / 0 đỏ**
- fuzz / nightly PASS; `vis explain --list` = **53 mã** khớp `all_codes()`; report-check **30/243** ✓
**VD-231 DONE** (`fa4c2d8`), **VD-232 DONE** (`2ee8c1e`), **VD-238 DONE** (`05e42d0`), **VD-235→VD-240** hướng (b) cho phase 2 (test grid, sau đóng pha 1), **VD-239 (a,c,d)** đạt (clone-clean ps tại tag; mac đo đủ; báo cáo sửa 6 mục).
**VD-186 patch nộp**; pha 2 mở: **VD-240 (b)** `+ VD-156` (trait default cho Error), **#151/#152/#149** giữ.
**Báo cáo đóng** = `agent/BAO-CAO-A0-2026-11-19-0900.md` §12 (VD-237) + addendum này.


### Windows — Tier 2 (2026-11-19, A0, @ `349af67`) — khối thêm, không sửa lịch sử

> Theo quyết định A0 "Windows tier 2 tại `349af67`" (`agent/DECISIONS.md`, block A0 2026-11-19 — Windows Tier 2).

Windows = **Tier 2 — product verified** @ `349af67`: compiler, interpreter, VM, module loading,
CRLF handling, doc-examples, LSP cơ bản đã kiểm chứng trên máy Windows thật (psvn). `cargo test --workspace`
**chưa** là cổng bắt buộc trên Windows — **101 ca đỏ / 25 suite** đã phân loại sơ bộ (lớp portability/VS3),
**chưa** được port và xác nhận độc lập từng ca. Sem 483/483 ×2 (interp, VM); fmt/clippy/R1..R6/
report-check/build/doc-examples PASS.

- VD "Windows test-suite portability" được mở **sau khi pha 1 đóng chính thức**, base `349af67`.
- `pha1-rc1` (`05e42d0…`) không bị di chuyển; Tier 1 chỉ sau VD portability + required CI.
- Windows gate tối thiểu sau VD: formatter/check cần thiết, build, test sản phẩm đa nền tảng, sem ×2, doc-examples, LSP/path fixtures; Unix-only tests không tính trong Windows count.

### Trạng thái sau VD-270 — đợt tối ưu liên tục (2026-11-21)

> Khối **thêm** sau khối Windows Tier 2; số cũ **không xoá** (VD-142). Nguồn: `JOURNAL/vd260-report.md`,
> `JOURNAL/vd258-plan.md`, `JOURNAL/vd258-progress.md`, `STATE.md`, `DECISIONS.md` VD-258..VD-270, `SO-NO.md`.
> HEAD lúc ghi `745851a` (21/11 08:04). Số đo cổng dưới đây lấy từ mốc mới nhất (`745851a`) và
> `JOURNAL/vd260-report.md`; đợt đang chạy nên số sẽ còn tăng.

**Bối cảnh**: sau R31 (baseline `31b51d2`, VD-242), A0 mở đợt tối ưu liên tục 20–21/11. Đợt này **không**
mở pha 2/VS3, **không** đổi runtime mặc định (frame/budget/quota chỉ trên đường test), **không** tag mới;
`pha1-rc1` = `05e42d0` giữ nguyên. Từ baseline đến `330d86c` là **476 commit**.

| # | Hạng mục | Mức thật (21/11) | Còn thiếu để đóng 100% |
|---|---|---|---|
| 1 | **Async** | runtime mặc định tuần tự/run-to-completion; **đường test** có suspend/resume + reactor + timer + cancellation | worker/GC/song song thật; chưa bật default; step-cap parity |
| 2 | **Kênh** | `chan(n≥1)` send/recv/close; `chan(0)` `VS0505`; frame test phủ đầy/rỗng/đóng | rendezvous/select/timeout (VS3) |
| 3 | **Net/HTTP** | loopback, chunked, redirect, HEAD, **deadline tổng** (ST-265) | TLS/HTTP2/keep-alive; test Internet thật |
| 4 | **Quyền** | runtime mọi nền; **Landlock Linux**; đường đọc đã đóng khe symlink (Linux) | macOS chưa verify; đường ghi TOCTOU (#140/#109b); Windows mức thấp hơn |
| 5 | **LSP** | definition/symbol/references/rename/signatureHelp/folding/formatting/typeDefinition/implementation/hover/completion | publish chẩn đoán tệp phụ; incremental sync |
| 6 | **Gói** | registry cục bộ, semver + resolver, lock SHA-256, `pkg update/list/remove` | registry HTTP, chữ ký số, GC store |
| 7 | **Đa nền tảng** | Linux chạy thật; Windows **Tier 2**; wasm **check** | runner macOS/Windows/wasm; macOS chưa verify |
| 8 | **Hợp chuẩn host** | Linux 54/54 + POSIX real | macOS chưa verify; Windows 39/54 (chỉ đo) |

**Đã làm (tóm tắt; chi tiết từng mục ở `JOURNAL/vd260-report.md`)**:

- **Checker/soundness**: `Result?` không bị coi là hai lớp Option; `None` có kiểu; record ctor/default;
  `?.` lồng; lift Option ở ctor/`with`/`push` và biên nhánh; or-pattern; `let … else` buộc thoát mọi nhánh
  (`VS0206`); refutable `let`/`for` bị từ chối tĩnh; generic field/instantiation; suy kiểu sau pattern lồng.
- **Frame/async (chỉ đường test)**: lát 0–3 (suspend/resume, reactor read/accept + timer, `mio = 1.2.5`),
  coverage `?`/`chan`/`cmd`/field-index assign/closure/`await` trong closure, cancellation/lifecycle,
  fairness/Q thử nghiệm. **Chưa bật runtime mặc định**, chưa worker/GC, chưa TLS/HTTP2.
- **Ngân sách**: instrumentation 8 sự kiện + bảng phụ theo PC, verifier cấu trúc/CFG, fuel VM riêng,
  tương thích VERSION/cache; **chưa bật ngân sách mặc định**; step-cap parity còn mở.
- **SQLite**: feature tùy chọn (`default = ["sqlite"]`), trần cell/overhead/byte, authorizer chặn
  `dbstat`/trang thô, `query` chỉ đọc, cache schema + prepared statement; thêm `query_one`,
  `execute_batch`, `tables`/`columns`/`indexes`/`index_columns`/`primary_keys`/`index_exists`/
  `schema_sql`/`explain`.
- **Stdlib**: mở rộng `std.fs` (read_lines/append/mkdir/remove/walk/glob/copy/rename/size/is_file/
  is_dir/read_bytes/write_bytes), `std.path` (normalize/relative/is_absolute/is_relative/join_all/
  with_extension), `std.str` (find/rfind/count/pad/trim/splitn/parse_int/parse_float/strip/replace_*),
  `std.list` (zip/min/max/median/sum/product/sum_float/mean/contains/index_of/binary_search/flatten/
  windows), `std.sqlite`; `panic`; `i64::MIN`; list rest pattern `[a, ..]`/`[a, ..r]`.
  `agent/API-STD.txt` hiện **135** mục.
- **CLI/LSP**: `run/check --stdin`, `fmt --stdin [--check] [--mode]`, `fmt --check --json|--list|--diff|--stat`,
  `test --filter/--fail-fast/--list/--repeat/--shuffle/--timeout/--json-out`, `explain --json`,
  `doctor`, `run --time`; LSP definition/documentSymbol/references/rename/prepareRename/signatureHelp/
  foldingRange/workspace symbol/documentHighlight/selectionRange/formatting/typeDefinition/implementation/
  hover/completion `std.*`.
- **Hiệu năng**: range lười (RSS 146 MB → ~9,5 MB @2M), release strip 7,26 → 6,23 MB, cache SQLite
  (6,8× với 300 bảng), enum `Rc<str>` (Value 72 → 64 B), cache LSP theo `(uri, text)`.

**Số đo cuối khối này**:

```
gate --fast @ 745851a : PASS — 2840 passed / 0 skipped
vis test test/sem     : 525/525 (interp) và 525/525 (--vm)
core-test / release / toolchain / host 54/54 @ a71c85b : PASS
nightly gần nhất      : PASS — fuzz 10.880.217 ca / 246.857.797 token / 0 vi phạm, fmt 475 tệp bất động @ da6b12c
Windows               : Tier 2 — workspace xanh @ c7cef71 (20/11), CHƯA verify lại ở HEAD
macOS                 : CHƯA verify (không có máy Darwin; `vis-remote mac` từ chối nếu không Darwin)
tag                   : chỉ pha1-rc1 = 05e42d0 (không di chuyển)
```

**Còn mở / chưa tuyên bố**: step-cap parity; P3 enum alloc (#152); #190 `pop_frame`; #191 Windows nextest;
#147/#152 blocked phần cứng; TLS/HTTP2/keep-alive; runner wasm; runner macOS (Landlock **đã có** trên Linux,
15/15); budget/fuel/Q chưa chốt default. **Đợt chưa đóng** — chờ mốc A0 09:00 23/11.

### VS3 — mục tiêu G0..G5 (2026-11-21, VD-271) — *lịch sử; `VD-284` đã đưa macOS arm64 vào phạm vi*

> Kế hoạch đầy đủ + bảng bao phủ tính năng: `agent/KE-HOACH-VS3-2026-11-21.md`.
> Tiến độ từng lát: `agent/VS3-TIEN-DO.md`. **macOS ngoài phạm vi**.

| Mốc | Mục tiêu | Tag |
|---|---|---|
| **G0** | Baseline sau pha 1 + VS3 wave 1 (Linux gate 3448/3; Windows 3404/0) | **`v0.1.0`** (tag tại commit này; `rc1` lịch sử) |
| **G1** | Nền VS3: 1a/1b/2a/4a/5a + test; 1c/1d/2b/4b/5b test/kế hoạch; 3a spike | — |
| **G2** | Async default + kênh (1a–1d, 2a–2b) | `v0.2.0` |
| **G3** | TLS/HTTP2/keep-alive + registry HTTP/chữ ký/GC | `v0.3.0` |
| **G4** | Worker pool + Windows Tier 1 + wasm runner | `v0.4.0` |
| **G5** | LSP tệp phụ/incremental + plugin/ViTerm | `v0.5.0` / `v1.0.0` candidate |

**Luật**: mọi tính năng VS3 phải có lát + test + docs + gate; không bỏ mảng nào; không tuyên bố xong
khi bảng `VS3-TIEN-DO.md` còn `TODO/BLOCKED` trong phạm vi mốc.

### Stage 0 — `v0.3.6` (2026-11-22, tag `8fa2c09`, code `0645700`)

> Nguồn: `QUEUE.md` ST-284..289, `VS3-TIEN-DO.md` §2c, `JOURNAL/st284-refactor.md`,
> `JOURNAL/st286-tach-module.md`, `JOURNAL/st288-goi.md`, `JOURNAL/st28{4,6}-diff-100k.log`,
> `agent/reports/R32-viscript.md`. Chi tiết từng task: QUEUE + R32.

| Task | Trạng thái | Commit | Bằng chứng chính |
|---|---|---|---|
| ST-284 — R0a `vm.rs`/`frame.rs` | **DONE** | `83a9a10` (merge `e363a15`) | `vm.rs` 1240, `frame.rs` 1042 dòng (≤1250); gate `3571/3`; diff 100k `20261115` **0 lệch** |
| ST-285 — R0b `value`/`codegen`/`tyck`/`interp/goi` | **DONE** | `ab5d292` (merge `246b6c5`) | 1186 / 1045 / 1064 / 1155 dòng (≤1250); gate `3576/3`; diff 100k `20261115` **0 lệch** |
| ST-286 — R0c `async_net`/`quyen`/`sqlite` | **DONE** | `f193c57` (merge `cfba662`) | 606 / 1190 / 1245 dòng; gate PASS; diff 100k `20261122` **0 lệch** (93 770 ca) |
| ST-287 — hook huỷ HTTP request | **DONE** | `03ed84c` (merge `f8f1d4b`) | h1 **5/5**, h2 **8/8** `vs3_http_cancel`; `cancel_task_handles` + `RST_STREAM`/đóng socket; không rò tay, parity interp↔VM; gate `3581/3`; diff 100k `20261122` **0 lệch** (`JOURNAL/st287-http-cancel.md`) |
| ST-288 — `VS0708` + `#142` + `pkg gc --root` | **DONE** (Gatekeeper 2026-11-22) | `c513e7a` (merge `545a38d`) | `VS0708` vi/en/ja + explain 54 + API-CODES 54; 6 ca âm chữ ký/khoá đều `VS0708`; soft pin #142; `gc --root` 4/4; `pkg` 25/25, sig 10/10. QUEUE còn REVIEW — chờ A0 lật |
| ST-289 — version `0.3.6` + `#111`/`#151` + R32 | **DONE** | `ff55056` (merge `84915aa`) | 8 crate `version.workspace`; `vis 0.3.6`; 24 fixture slug R100%; R32 khung |

**Gate Stage 0 — final QA** @ **`4c5dec6`**: fresh-clone `gate --fast` **3581 passed / 3 skipped**;
release workspace **3586 passed / 0 failed / 3 ignored**; P2 14, P1 8, P3 10; h2 19 + feature 34+1;
`pkg` 25, sig 10, http_cancel 5+8, host 54/54; diff 100k seed `202611221700` **0 lệch**;
**2× soak 20/20 vòng** (593 s + 602 s). Soak huỷ HTTP 1 giờ (`vs3_http_cancel` 5/5 + 8/8 mỗi vòng,
fd/RSS/threads phẳng) ghi `JOURNAL/st287-soak-1h.md`. Windows final @ `a1930cb`: workspace lần 2
**3538/0/3** nhưng `vs3_http_cancel::cancel_http_h1_lap_nhieu_vong_khong_ro` **flake ~21 %**
(`V:post` sau 10053); feature h2 **BLOCKED-ENV** (Unix-only). **`v0.3.6` đã tag @ `8fa2c09` — ST-287 fix xong
rồi verify lại Windows.** **Stage 0 đủ 6/6** — R32 chốt phần Stage 0; GĐ 1 (`ST-290..`) vẫn
`BLOCKED` chờ A0 mở.
