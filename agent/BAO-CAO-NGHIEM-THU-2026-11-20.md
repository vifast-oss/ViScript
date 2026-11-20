<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Báo cáo nghiệm thu cuối — dòng ViScript, 2026-11-20

> **Mục đích**: chốt **một bảng nghiệm thu duy nhất** cho ứng viên phát hành cuối theo yêu cầu
> Agent 0. Không gắn tag, không tuyên bố phát hành. Nếu sau bảng này chỉ có commit tài liệu/sổ,
> commit đã đo giữ nguyên và commit báo cáo được ghi riêng.
>
> **Giới hạn phạm vi**: macOS giữ **"chưa verify"** (VD-242 Q7); điều đó chỉ chặn tuyên bố hỗ trợ
> macOS, **không** chặn kết luận Linux/Windows với phạm vi nêu rõ ở bảng. Ma trận target là
> **`cargo check --target` biên dịch chéo**, không phải chạy runtime trên mọi target.

## Bảng nghiệm thu (duy nhất)

| # | Hạng mục | Kết quả đo / Trạng thái | Bằng chứng (`agent/JOURNAL/`) |
|---|---|---|---|
| 1 | **Commit sản phẩm đã đo + trạng thái cây** | `9acaa825c657436e278b23a7cf6de4bf6f0840b9` (=`origin/main`=HEAD khi đo). Cây **sạch**: fresh-clone từ `origin/main` cho `git status --porcelain` rỗng (0 thay đổi). Product code lần cuối đổi ở `7427de0`; `8fd59d5` (test path Windows), `75576e6` rồi `9acaa82` (tooling ST-257) là các commit sau đó, đều đã nằm trong phép đo cuối. Bản báo cáo này là **commit riêng**, không làm đổi commit đã đo. | `nghiem-thu-2026-11-20-fresh-gate.log` (đầu log: HEAD + tree 0); `git rev-parse HEAD origin/main` |
| 2 | **Toolchain / nền tảng / features** | **Linux**: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`, `Linux 7.0.0-43-generic x86_64`. **Windows** (host `psvn`, repo `/c/Users/ps/viscript-moi`): `rustc 1.100.0 (0ef1bc386 2026-11-09)`, `cargo 1.100.0 (3acc79c0c 2026-11-05)`. **Features**: mặc định `default = ["sqlite"]`; cấu hình core dùng `--no-default-features`. Cache bytecode `VERSION = 2` (VD-244); **không** bump thêm sau ST-252 (VD-246) — không có output chương trình hợp lệ nào đổi. | `nghiem-thu-2026-11-20-win-toolchain.log`; `crates/vis-cli/Cargo.toml`; `agent/JOURNAL/st252-review.md` |
| 3 | **Linux gate** | `./vis-dev check`: **PASS** — fmt OK, clippy OK, luật R1..R6 OK, self-test case âm OK, `vis explain` OK, báo cáo R\*-viscript OK, fixture âm OK, build OK, doc-examples OK (24 khối: 19 xanh + 5 ngoại lệ đỏ đúng; 12 fixture QD §7.1..7.3). `./vis-dev gate --fast`: **PASS** (L0 test OK). `./vis-dev nightly --skip-gate --fuzz-minutes 0`: **PASS** — vòng tròn fmt 475 tệp bất động, bỏ qua 78 ca lỗi cú pháp (cơ chế ST-257 đã negative-test: `fmt --check` trả 1 trước / 0 sau `--write`). | `nghiem-thu-2026-11-20-fresh-gate.log`, `nghiem-thu-2026-11-20-nightly.log` |
| 4 | **Semantics — hai engine** | `./vis test test/sem` (interp) **525/525**, exit 0. `./vis test --vm test/sem` (VM) **525/525**, exit 0. Hai đường interp/VM cùng output. Bao gồm 8 ca runtime trap global (VD-245/#188) và 8 ca capture/local (VD-246/#189). | `nghiem-thu-2026-11-20-sem-interp.tap`, `nghiem-thu-2026-11-20-sem-vm.tap` |
| 5 | **Fresh-clone** | Clone từ `https://github.com/vifast-oss/ViScript.git` @ `origin/main` = `9acaa82`, tree sạch. `./vis-dev gate --fast` **PASS**; semantics 525/525 × 2; `./bin/vis-targets` exit 0; `nightly --skip-gate --fuzz-minutes 0` **PASS**. | `nghiem-thu-2026-11-20-fresh-gate.log`, `nghiem-thu-2026-11-20-sem-*.tap`, `nghiem-thu-2026-11-20-targets.txt` |
| 6 | **Windows** | Trên `9acaa82` (xác nhận `git rev-parse HEAD` = `9acaa82` sau `pull --ff-only` từ `75576e6`). `/c/Users/ps/.cargo/bin/cargo test --manifest-path /c/Users/ps/viscript-moi/Cargo.toml --workspace --no-fail-fast` → **1422 passed / 0 failed**, exit 0. Ở lần đo trước (`8fd59d5`) có 1 ca đỏ do kỳ vọng path (`rt/b.vis` vs `rt\b.vis`), đã sửa ở ST-255 chỉ trong kỳ vọng test, giữ nguyên tiêu chí "trỏ đúng module + dòng"; không đổi sản phẩm. | `nghiem-thu-2026-11-20-win-pull.log`, `nghiem-thu-2026-11-20-win-ws-test.log` (log máy: `/c/Users/ps/vislogs/final3-ws-20261120T043325Z.log`) |
| 7 | **Ma trận target + giới hạn hỗ trợ** | `./bin/vis-targets` exit 0. **core** (`--no-default-features`): **PASS=18 · EXPECTED_RED=0 · FAIL=0**. **sqlite** (default): **PASS=16 · EXPECTED_RED=2 · FAIL=0** — 2 ô `vis-cli` × (`x86_64-pc-windows-msvc`, `wasm32-unknown-unknown`) đúng nợ #156 (`libsqlite3-sys` cần C toolchain). Giới hạn: bảng chỉ là **CHECK biên dịch chéo**; ô xanh của `std::process`/`std::net`/`std::fs` trên wasm32 không có nghĩa chạy được. **macOS chưa verify**. **Windows = Tier 2** (bằng chứng runtime là phép đo ở dòng 6, không suy từ bảng này). #156 đã đóng dưới dạng tách cấu hình + công bố ma trận; không đổi SQLite MVP (`rusqlite = "=0.37.0"`, bundled, hooks, VERSION/opcode/freeze API). | `nghiem-thu-2026-11-20-targets.txt`; `agent/TARGET-MATRIX-2026-11-20.md` |
| 8 | **Nợ mở + lý do chặn** | Sổ nợ: **191 = 95 mở / 70 đóng / 23 VS3 / 2 không làm / 1 điều hướng**. Liên quan phát hành: **#190** `pop_frame` writeback silent skip — **mở, KHÔNG chặn**: phán quyết "*chưa tìm thấy đường tới từ bytecode đã verify*"; nâng thành blocker ngay nếu phát hiện đường tới (VD-246 §4). **#186** lệch chẩn đoán ST-28 — **mở, KHÔNG chặn** theo luật A0 khi cả hai engine từ chối trước khi có side-effect. **#147/#152** macOS — **mở, chặn phần cứng**: không có máy Darwin thật (`vis-remote mac` trả Linux); chỉ chặn tuyên bố macOS. Các nợ còn lại là technical debt nền, không phát sinh từ đợt ổn định này. | `agent/SO-NO.md` §8; `agent/BLOCKED.md` §18 (#190), §8 (#147/#152), §12 (#172 → VD-243) |
| 9 | **Trạng thái ratification** | **Đã đóng sổ, không còn ngữ nghĩa "chưa có quyết định" trong phạm vi phát hành.** VD-241 ratify toàn bộ VD-215..VD-240; VD-242..VD-246 là quyết định A0 trực tiếp (ổn định; percent lỗi `VS0507`; tương thích VERSION/cache; runtime trap global; capture/local trap). Closure-param lift bị loại sang **#155 / VS3** — không ratify trong bản này, đúng thiết kế. A0 muốn đảo mục nào = quyết định mới (VD mới kèm phân tích revert), không phải phần "chưa ratify" của ST-244. | `agent/BAO-CAO-RATIFICATION-2026-11-20.md`; `agent/DECISIONS.md` VD-241..VD-246 |

## Kết luận ngắn

Ứng viên `9acaa82` đã vượt **toàn bộ** phép đo bắt buộc: gate Linux, semantics hai engine,
fresh-clone và Windows workspace test — xanh trên một **cùng một commit sạch**. Không còn ngữ
nghĩa phát hành nào chưa được ratify. Không nợ nào được xác nhận chặn phát hành Linux/Windows
trong phạm vi nêu trên; macOS vẫn là tuyên bố duy nhất bị chặn do thiếu phần cứng.

**Khuyến nghị gửi Agent 0**: chấp nhận bảng này làm nghiệm thu cuối cho phạm vi **Linux + Windows
Tier 2**, giữ macOS "not verified". **Không gắn tag** khi chưa có chỉ thị mới của A0.
