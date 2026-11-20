<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Báo cáo nghiệm thu cuối — dòng ViScript, 2026-11-20

> **Mục đích**: chốt **một bảng nghiệm thu duy nhất** cho ứng viên phát hành cuối theo yêu cầu
> Agent 0. Không gắn tag, không tuyên bố phát hành. Nếu sau bảng này chỉ có commit tài liệu/sổ,
> commit đã đo giữ nguyên và commit báo cáo được ghi riêng.
>
> **Lịch sử báo cáo**: `30a7d1d` là bản đầu (đo trên `9acaa82`); sau khi A0/Agent chủ trì quyết
> định đưa ST-258 vào, bản **hiện tại** đo lại toàn bộ trên `dbae70a` và là bản có hiệu lực.
>
> **Giới hạn phạm vi**: macOS giữ **"chưa verify"** (VD-242 Q7); điều đó chỉ chặn tuyên bố hỗ trợ
> macOS, **không** chặn kết luận Linux/Windows với phạm vi nêu rõ ở bảng. Ma trận target là
> **`cargo check --target` biên dịch chéo**, không phải chạy runtime trên mọi target.

## Bảng nghiệm thu (duy nhất)

| # | Hạng mục | Kết quả đo / Trạng thái | Bằng chứng (`agent/JOURNAL/`) |
|---|---|---|---|
| 1 | **Commit sản phẩm đã đo + trạng thái cây** | `dbae70a050415fe1c88a1545c7b011abb05d9aa2` (=`origin/main`=HEAD khi đo). Cây **sạch**: fresh-clone từ `origin/main` cho `git status --porcelain` rỗng (0 thay đổi). Product code lần cuối đổi ở `7427de0`; các commit sau (`8fd59d5` test path Windows, `75576e6` ST-257 tooling, `9acaa82` ST-257, `dbae70a` ST-258 tooling) đều nằm trong phép đo cuối. Bản báo cáo này là **commit riêng**, không làm đổi commit đã đo. | `nghiem-thu-2026-11-20-fresh-gate.log` (đầu log: HEAD + tree 0); `git rev-parse HEAD origin/main` |
| 2 | **Toolchain / nền tảng / features** | **Linux**: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`, `Linux 7.0.0-43-generic x86_64`; `cargo-nextest 0.9.152`. **Windows** (host `psvn`, repo `/c/Users/ps/viscript-moi`): `rustc 1.100.0 (0ef1bc386 2026-11-09)`, `cargo 1.100.0 (3acc79c0c 2026-11-05)`; `cargo-nextest 0.9.152` (cài thêm theo phép đo này). **Features**: mặc định `default = ["sqlite"]`; core dùng `--no-default-features`. Cache bytecode `VERSION = 2` (VD-244); **không** bump thêm sau ST-252 (VD-246) — không có output chương trình hợp lệ nào đổi. | `nghiem-thu-2026-11-20-win-toolchain.log`; log cài nextest `/c/Users/ps/vislogs/win-nextest-install-20261120T044715Z.log`; `crates/vis-cli/Cargo.toml`; `agent/JOURNAL/st252-review.md` |
| 3 | **Linux gate** | `./vis-dev check`: **PASS** — fmt, clippy, luật R1..R6, self-test case âm, `vis explain`, báo cáo R\*, fixture âm, build, doc-examples (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3). `./vis-dev gate --fast`: **PASS** với `cargo-nextest`: **1430/1430 passed** (7,98 s) + **5 doc-test** = 1435 (khớp đường `cargo test` cũ); đường fallback (`VIS_NO_NEXTEST=1`, máy không có nextest) cũng **PASS**. **Tổng gate ~11,6 s so với ~40 s trước khi tối ưu (~3,4×)**. `nightly --skip-gate --fuzz-minutes 0`: **PASS** — vòng tròn fmt 475 tệp bất động, bỏ qua 78 ca lỗi cú pháp. | `nghiem-thu-2026-11-20-fresh-gate.log`, `nghiem-thu-2026-11-20-fresh-gate-nextest.log`, `nghiem-thu-2026-11-20-nightly.log` |
| 4 | **Semantics — hai engine** | `./vis test test/sem` (interp) **525/525**, exit 0. `./vis test --vm test/sem` (VM) **525/525**, exit 0. Hai đường interp/VM cùng output. Bao gồm 8 ca runtime trap global (VD-245/#188) và 8 ca capture/local (VD-246/#189). | `nghiem-thu-2026-11-20-sem-interp.tap`, `nghiem-thu-2026-11-20-sem-vm.tap` |
| 5 | **Fresh-clone** | Clone từ `https://github.com/vifast-oss/ViScript.git` @ `origin/main` = `dbae70a`, tree sạch. `./vis-dev gate --fast` **PASS** (nextest 1430/1430 + 5 doc); semantics 525/525 × 2; `./bin/vis-targets` exit 0; `nightly --skip-gate --fuzz-minutes 0` **PASS**. | `nghiem-thu-2026-11-20-fresh-gate.log`, `nghiem-thu-2026-11-20-sem-*.tap`, `nghiem-thu-2026-11-20-targets.txt` |
| 6 | **Windows** | Trên `dbae70a` (xác nhận `git rev-parse HEAD` = `dbae70a` sau `pull --ff-only`). Lệnh nghiệm thu `/c/Users/ps/.cargo/bin/cargo test --manifest-path /c/Users/ps/viscript-moi/Cargo.toml --workspace --no-fail-fast` → **1422 passed / 0 failed**, exit 0. Cross-check thêm bằng nextest: cài `cargo-nextest 0.9.152` và chạy full `cargo nextest run --workspace --no-fail-fast` → **1417 chạy / 1416 passed / 1 failed** (49,21 s); ca fail duy nhất là **network có sẵn flaky** `tcp_explain_tong` (`os error 10053` / WSAECONNABORTED, **1 pass / 3 lần chạy riêng**) — race của test socket, **không phải sản phẩm** và **không nằm trong lệnh nghiệm thu**; `cargo test` chính thức vẫn xanh 1422/0. Ở `8fd59d5` từng có 1 ca đỏ do kỳ vọng path (`rt/b.vis` vs `rt\b.vis`), đã sửa ở ST-255 chỉ trong kỳ vọng test, giữ nguyên tiêu chí "trỏ đúng module + dòng". | `nghiem-thu-2026-11-20-win-pull.log`, `nghiem-thu-2026-11-20-win-ws-test.log`, `nghiem-thu-2026-11-20-win-nextest-crosscheck.log`, `nghiem-thu-2026-11-20-win-nextest-full.log`, `nghiem-thu-2026-11-20-win-nextest-flaky{1,2,3}.log` (log máy: `/c/Users/ps/vislogs/final4-ws-20261120T044513Z.log`) |
| 7 | **Ma trận target + giới hạn hỗ trợ** | `./bin/vis-targets` exit 0. **core** (`--no-default-features`): **PASS=18 · EXPECTED_RED=0 · FAIL=0**. **sqlite** (default): **PASS=16 · EXPECTED_RED=2 · FAIL=0** — 2 ô `vis-cli` × (`x86_64-pc-windows-msvc`, `wasm32-unknown-unknown`) đúng nợ #156 (`libsqlite3-sys` cần C toolchain). Giới hạn: bảng chỉ là **CHECK biên dịch chéo**; ô xanh của `std::process`/`std::net`/`std::fs` trên wasm32 không có nghĩa chạy được. **macOS chưa verify**. **Windows = Tier 2** (bằng chứng runtime là phép đo ở dòng 6, không suy từ bảng này). #156 đã đóng dưới dạng tách cấu hình + công bố ma trận; không đổi SQLite MVP (`rusqlite = "=0.37.0"`, bundled, hooks, VERSION/opcode/freeze API). | `nghiem-thu-2026-11-20-targets.txt`, `...targets.txt.license`; `agent/TARGET-MATRIX-2026-11-20.md` |
| 8 | **Nợ mở + lý do chặn** | Sổ nợ: **191 = 95 mở / 70 đóng / 23 VS3 / 2 không làm / 1 điều hướng**. Liên quan phát hành: **#190** `pop_frame` writeback silent skip — **mở, KHÔNG chặn**: phán quyết "*chưa tìm thấy đường tới từ bytecode đã verify*"; nâng thành blocker ngay nếu phát hiện đường tới (VD-246 §4). **#186** lệch chẩn đoán ST-28 — **mở, KHÔNG chặn** theo luật A0 khi cả hai engine từ chối trước khi có side-effect. **#147/#152** macOS — **mở, chặn phần cứng**: không có máy Darwin thật (`vis-remote mac` trả Linux); chỉ chặn tuyên bố macOS. Các nợ còn lại là technical debt nền, không phát sinh từ đợt ổn định này. | `agent/SO-NO.md` §8; `agent/BLOCKED.md` §18 (#190), §8 (#147/#152), §12 (#172 → VD-243) |
| 9 | **Trạng thái ratification** | **Đã đóng sổ, không còn ngữ nghĩa "chưa có quyết định" trong phạm vi phát hành.** VD-241 ratify toàn bộ VD-215..VD-240; VD-242..VD-246 là quyết định A0 trực tiếp (ổn định; percent lỗi `VS0507`; tương thích VERSION/cache; runtime trap global; capture/local trap). Closure-param lift bị loại sang **#155 / VS3** — không ratify trong bản này, đúng thiết kế. A0 muốn đảo mục nào = quyết định mới (VD mới kèm phân tích revert), không phải phần "chưa ratify" của ST-244. | `agent/BAO-CAO-RATIFICATION-2026-11-20.md`; `agent/DECISIONS.md` VD-241..VD-246 |

## Ghi nhận thêm về tối ưu build/test (theo yêu cầu A0/Agent chủ trì)

- **ST-257**: vòng tròn fmt gom ~950 lần spawn `vis fmt` thành đúng 2 lệnh `fmt --write` +
  `fmt --check` (475 tệp, 0 va chạm tên; negative-test xác nhận kiểm không vô hiệu).
- **ST-258**: L0 test dùng `cargo-nextest` (song song) + `cargo test --doc` khi có nextest;
  fallback y hệt `cargo test --workspace` khi không có; `VIS_NO_NEXTEST=1` ép fallback để đối
  chiếu. Đã cài `cargo-nextest 0.9.152` trên cả Linux và Windows.
- **Đo lại thời gian (warm, Linux)**: `check` 2,74 s · `cargo build` 0,03 s ·
  `cargo test --workspace` (cũ) 37,6 s · `cargo test --doc` 0,57 s · **gate --fast mới 11,6 s**.
  Nút thắt là 144 test binary chạy tuần tự (4 binary đầu ~23 s); nextest chạy song song ⇒ ~3,4×.
- **Chưa có** (không cần cho nghiệm thu, chỉ là tùy chọn nếu muốn tăng tốc build lạnh):
  `sccache`, `mold`/`lld`, cargo config/profile tùy chỉnh — compile warm chỉ 0,06 s nên không
  phải nút thắt hiện tại. Không thay đổi gì thêm trong phạm vi ổn định này.

## Kết luận ngắn

Ứng viên `dbae70a` đã vượt **toàn bộ** phép đo bắt buộc: gate Linux, semantics hai engine,
fresh-clone và Windows workspace test — xanh trên **cùng một commit sạch**. Không còn ngữ
nghĩa phát hành nào chưa được ratify. Không nợ nào được xác nhận chặn phát hành Linux/Windows
trong phạm vi nêu trên; macOS vẫn là tuyên bố duy nhất bị chặn do thiếu phần cứng.

**Khuyến nghị gửi Agent 0**: chấp nhận bảng này làm nghiệm thu cuối cho phạm vi **Linux + Windows
Tier 2**, giữ macOS "not verified". **Không gắn tag** khi chưa có chỉ thị mới của A0.
