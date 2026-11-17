# R28 — ViScript, vòng 28 (đợt 28 khép lại 10:00 JST, 2026-11-17)

Báo cáo chốt **đợt 28** — phần **sau R27**. Nguồn quyết định: bản **17d**
([`JOURNAL/quyet-dinh-a0-2026-11-17d.md`](../JOURNAL/quyet-dinh-a0-2026-11-17d.md)) §1–§3 và
[`DECISIONS.md`](../DECISIONS.md) — **VD-156** (cho phép sửa tối thiểu hai nợ **#110** và **#129**), cùng các luật
còn hiệu lực **VD-131** (5 câu), **VD-142** (báo cáo đã commit không sửa), **VD-143** (danh sách hàng phụ thuộc),
**VD-145/VD-146** (macOS / Windows **chỉ đo**), **VD-147(3)(4)(5)** (sổ nợ duy nhất · hash đo từng nền · không ghi
đè log), **VD-148** (chưa mở VS3), **VD-151** (VS2 đóng kỹ thuật 8/8). Sổ nợ duy nhất: [`SO-NO.md`](../SO-NO.md) §8.
Báo cáo **không** sửa R1..R27; mọi điểm cần nói lại về R27 ghi ở **§2**.

## 0. Ba hàng phụ thuộc + chính báo cáo — trạng thái tại thời điểm chốt (VD-143)

| Hàng | Nội dung | Trạng thái | Commit | Bằng chứng chính |
|---|---|---|---|---|
| **ST-156** | **R27** + cập nhật `ROADMAP.md`/`STATE.md` (chốt đêm 17/11) | **DONE** | `c9249b79` | R27: gate macOS PASS L0 **1175/0** (130 suite), report-check **27** báo cáo / **174** hàng QUEUE tại `head` `dd088322` (`st156-gate-macos.log`) |
| **ST-157** | Nợ **#110**: VM đưa `argv` (`list[str]`) vào tham số **không-`cap`** của `main` (parity với interp) | **DONE** | `567350e1`, `4bdede6c` | ca đỏ-trước `crates/vis-cli/tests/vm_main_argv.rs` **3/4 ĐỎ** (interp `[a, b]` vs VM `()`), sau sửa **4/4 xanh** cùng byte; `cargo test -p vis-cli` **778/0**; gate macOS PASS L0 **1179/0** (131 suite) — log `st157-*` |
| **ST-158** | Nợ **#129**: gom danh sách **số dòng khối tài liệu** bị loại trừ về **một nguồn** (4 tệp test) | **DONE** | `8036b0a2`, `fdb21d15` | một nguồn `viscript/test/doc_excluded.rs` (`DOC_EXCLUDED: [usize; 7]`), 4 tệp `include!` không chép số; 4 target **trước = sau** 13/0 · 13/0 · 3/0 · 10/0; gate macOS PASS L0 **1179/0**; check_report **27/177** — log `st158-*` |
| **ST-159** | **R28** (chốt phần sau R27) | **READY tại `head`** | — (commit báo cáo) | chính báo cáo này; `head` = `fdb21d15e474c853600ffa730347e88334dc991e` |

**ST-159 (chính báo cáo này): `READY` tại `head` = `fdb21d15e474c853600ffa730347e88334dc991e`** — QUEUE tại
`head` ghi `READY`; hàng lật `DONE` ở **chính commit chứa báo cáo**, sau khi `check_report --all` PASS (**28** báo
cáo, **177** hàng) và gate macOS `--fast` PASS trên cây đang chốt (đúng **VD-143**: 2/2 hàng phụ thuộc
**ST-157/ST-158** đã `DONE` trước khi lật). QUEUE không còn hàng `BLOCKED`/`REVIEW` ⇒ `blocked: []` trong JSON.

## 1. Tóm tắt

1. **Sau R27, đợt 28 đóng nốt hai nợ có số trong sổ**: **#110** (parity `argv` interp↔VM, ST-157) và **#129**
   (danh sách khối tài liệu về một nguồn, ST-158). Không mở task VS3, không đụng Windows (VS3 mục 5).
2. **Đêm 17/11 đóng 12 hàng** — ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-152 · ST-153 · ST-154 · ST-155 ·
   ST-156 · ST-157 · ST-158 — cộng hàng báo cáo vòng **ST-159** (chính báo cáo này) = **13 hàng**. Tất cả `DONE`;
   **không** hàng nào `BLOCKED`.
3. **Ba nền có số**: macOS gate **PASS 10/10**, L0 **1179/0** (**131** suite); `ps` gate **PASS 10/10**, L0
   **1169/0** (129 suite — **đo trước** 9 ca ST-151, **chưa đo lại**) + `vis-host-linux` **15/15**; `psvn`
   **chỉ đo** — self-test **12/12**, cổng đi qua **9/10 bước** rồi dừng ở **L0** (`cap_proc` 150/4).
4. **Nợ trích theo số trong sổ**: **146 mục** = **89 mở** / **34 đóng** / **21 VS3** / 1 không làm / 1 điều hướng
   (dải 1..145). Đóng trong đợt 28: **#110**, **#129**.
5. **Không** bắt đầu tính năng VS3 (VD-148 giữ nguyên): bộ đầu vào `st155-vs3-dieu-tra.md` đã có, nhưng **chưa**
   có tài liệu thiết kế — **Agent 0** phải trả lời **5 câu hỏi** (`st155` §8) trước.
6. **Không có lần cổng đỏ nào trong đợt 28**: ST-157, ST-158 và ST-159 mỗi task chạy **một** lần gate macOS
   `--fast`, cả ba PASS `[exit=0]` 10/10 bước (không retry — D2-07).

**Ba hàng của đợt 28 (phần sau R27)** — đầy đủ **177** hàng ở `R28-viscript.json`:

| ID | Trạng thái | Commit | Số đo / cổng |
|---|---|---|---|
| ST-157 | DONE | `567350e1`, `4bdede6c` | ca `vm_main_argv` **4/4** (trước 3/4 ĐỎ) · `cargo test -p vis-cli` **778/0** · gate PASS, L0 **1179/0** |
| ST-158 | DONE | `8036b0a2`, `fdb21d15` | 4 target **13/0 · 13/0 · 3/0 · 10/0** (trước = sau) · gate PASS, L0 **1179/0** · check_report **27/177** |
| ST-159 | **READY** (tại `head`) | — (commit báo cáo) | gate macOS PASS, L0 **1179/0** · check_report **28/177** · sổ **146 = 89/34/21** |

## 2. Đính chính / ghi chú (VD-142 — không sửa R27, chỉ ghi ở đây)

| # | Điểm | Ghi chú R28 | Căn cứ |
|---|---|---|---|
| (a) | **Cơ sở L0: 1175 → 1179** | Lịch sử L0 macOS: **1159** (R25) → **1163** (ST-149) → **1166** (ST-153) → **1175** (ST-151, **130** suite) → **1179** (ST-157, **131** suite, **+4 ca mới** của `vm_main_argv.rs`). Con số **1175** trong R27 **đúng tại `head` của R27** (`dd088322`); từ R28 trích **1179/0 · 131 suite** | `st156-gate-macos.log` (**1175**) · `st157-gate-macos.log` / `st158-gate-macos.log` / `st159-gate-macos.log` (**1179**) |
| (b) | **Một ca cũ phải đổi kỳ vọng — bắt buộc, không phải nới ngưỡng** | `crates/vis-cli/tests/cap_khong_lo.rs::doi_so_dong_lenh_la_str` từ **ST-124** đã **ghim chính sự lệch #110** (VM trả `()` + từ chối `VS0501`). Đóng **#110** ⇒ ca đó **buộc** phải đỏ nếu giữ nguyên ⇒ **cập nhật một lần** để khẳng định **parity** (`[cap[Dir] @ p"/"]` + `1`, `exit 0`), **giữ bất biến** ST-124 (không đường nào tạo `Cap`). Đây là **ca duy nhất** phải đổi trong đợt; căn cứ **VD-156**, **không** phải nới ngưỡng (QD §4 luật 4) | VD-156 · `QUEUE.md` dòng `> CHỜ DUYỆT Agent 1 · 567350e1` · `st157-do-truoc.log` |
| (c) | **Nguồn khối tài liệu: 7 số, nhưng `doc_fixtures.rs` loại trừ 6 khối — ghi rõ, không giấu** | Một nguồn `test/doc_excluded.rs` giữ **7** số dòng (`305, 432, 445, 614, 666, 745, 1420`). Tệp `crates/vis-syntax/tests/doc_fixtures.rs` loại trừ **6** khối vì khối **666** (`extern cmd git(…, ...rest: str)`) **parse sạch từ ST-27**; nguồn dẫn xuất `DOC_EXCLUDED_SYNTAX: [usize; 6]` tính bằng **const block** (không chép số). **Không đổi** số kỳ vọng nào | `viscript/test/doc_excluded.rs` · `st158-sau.log` · `QUEUE.md` dòng `> CHỜ DUYỆT Agent 1 · 8036b0a2` |
| (d) | **`ps` và `psvn` chưa đo lại sau ST-151/ST-157 — không suy ra số mới** | `ps` đo **1169/0** tại `32b79d4a` (ST-153) khi 9 ca `sync_list.rs` và 4 ca `vm_main_argv.rs` **chưa có**; **chưa đo lại** ⇒ **không** ghi 1178/1182 như số đo (chỉ là **dự kiến**). `psvn` giữ số của **ST-153/ST-144** (chưa đo lại, Windows **chỉ đo**) | `st153-gate-ps.log` · `st153-gate-psvn.log` · `st144-cargo-test.log` |

**Ghi chú thêm (không phải lỗi của R27)**: caveat **soak macOS** "đo ở HEAD cũ `3079dede`" (**R24**) vẫn **giữ
nguyên**; đợt 28 **không** chạy lại soak (luật cứng: không nightly/soak).

## 3. Số đo theo từng nền, kèm hash đo và log (VD-147(4))

Mọi số dưới đây lấy từ log trong `viscript/agent/JOURNAL/`; số nào không có log thì **không** ghi.
Log mang `st159` là log của chính task này.

### 3.1 macOS (máy dev `mps`, arm64) — **hash đo: `fdb21d15`** (HEAD của R28)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` (cổng cuối R28) | **PASS**, `[exit=0]`, **10/10** bước, L0 **1179/0** (**131** suite), report-check PASS (**28** báo cáo, **177** hàng QUEUE), doc-examples OK (24 khối) — chạy **một** lần, không retry | `st159-gate-macos.log` (sha256 ở §3.4b) |
| `gate --fast` mốc L0 **1179/0** | **PASS** `[exit=0]`, 10/10, L0 **1179/0** (131 suite) | `st157-gate-macos.log`, `st158-gate-macos.log` |
| `cargo test -p vis-cli` (ST-157) | **778/0** (77 suite, gồm `vm_main_argv` **4/4**) | ghi trong `QUEUE.md` hàng ST-157 |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 (HEAD `ee58a518`) | `st142-mac-vis-test.log`, `st142-mac-vis-test-vm.log` |
| differential 100k seed `20261115` | **lech = 0**, **157,4 s**, 635/s (`loi_tinh=6230`, `chay=93770`) | `st142-mac-diff-100k.log` |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **25,43 s** | `st142-mac-cache.log` |
| host suite (54 ca) | **54/54** (`nền: Darwin arm64`) | `st141-host-suite-macos.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-macos.log` |
| soak 10 vòng | **10/10 = 1352 s** — **đo ở HEAD cũ**: log mở đầu ghi `HEAD 3079dede`; **không** chạy lại soak | `st138-soak.log` |

### 3.2 Linux (`ps` — Ubuntu 26.04.1, kernel 7.0.0-43-generic) — **hash đo: `32b79d4a`** (cổng), `d203b664` (Landlock), `ee58a518` (vis test/diff/cache), `2866f8c2` (soak)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` | **PASS**, `[exit=0]`, **10/10** bước, L0 **1169/0** (129 suite) — **đo TRƯỚC** 9 ca ST-151 và 4 ca ST-157, **chưa đo lại** (§2(d)) | `st153-gate-ps.log` |
| `vis-host-linux` (Landlock) | **15/15** ca (13 ca cũ + ca 14 âm **GHI** + ca 15 **dương** `read_dir` trên thư mục có quyền ghi), 0 failed / 0 ignored | `st154-landlock-ps.log` (HEAD `d203b664`, sha256 tệp đo khớp hai phía `7102e90b…`) |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 (HEAD `ee58a518`) | `st142-ps-vis-test.log`, `st142-ps-vis-test-vm.log` |
| differential 100k seed `20261115` | **lech = 0**, **40,9 s**, 2448/s (`loi_tinh=6230`, `chay=93770`) | `st142-ps-diff-100k.log` |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **6,33 s** | `st142-ps-cache.log` |
| host suite (54 ca) | **54/54** (`nền: Linux x86_64`) | `st141-host-suite-ps.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-ps.log` |
| soak 10 vòng | **10/10 = 250 s** (gate 38/38/38 s · diff 41/40/41 s · host 1/0 s · cache 7/6 s) | `st143-soak.log` (HEAD `2866f8c2`) |
| ca đua TOCTOU đường đọc | **0/50 000** (`lap=50000 ro_ri=0 chan=20756`) | `st137-sau-ps.log` · `st124-toctou-red.log` |

**Lệch đã liệt kê (không sửa)**: chỉ **thời gian** — diff 157,4 s (macOS) vs 40,9 s (`ps`), cache 25,43 s vs
6,33 s (≈3,9–4×) vì chi phí spawn tiến trình mỗi nền; `loi_tinh` **trùng 6230** ⇒ cùng bộ sinh ca. Không lệch
số đúng/sai, **không SKIP** nền nào.

### 3.3 Windows (`psvn` — Windows 10.0.26300, x86_64-pc-windows-msvc, rustc 1.99.1) — **CHỈ ĐO** (VD-121/VD-146), **hash đo: `3dc13342`** (bộ số cũ) và `4a805115` (cổng ST-153)

| Phép đo | Kết quả | So với macOS / Linux |
|---|---|---|
| `gate --fast` (Git Bash, `RUSTUP_TOOLCHAIN=1.99.1`) | **đi qua 9/10 bước** — `fmt` OK → `clippy` OK → `luật R1..R4` OK → **self-test 12/12** OK → `vis explain` OK → `báo cáo` OK → `báo cáo fixtures` OK → `build` OK → `doc-examples` OK (24 khối) → **`L0 test`: FAIL** | PASS 10/10 · PASS 10/10 |
| L0 trên Windows (fail-fast) | **150 passed / 4 failed**, 16 suite rồi dừng ở suite `cap_proc` (`/bin/echo`, `/bin/sh` không tồn tại — **VS3**, không sửa ca) | 1179/0 · 1169/0 |
| `cargo test --workspace --no-fail-fast` (ST-144) | **1053 đạt / 100 đỏ**, 129 suite, `[exit=101]` | 1179/0 · 1169/0 |
| `bin/vis-host-suite` (Git Bash) | **39/54**, `[exit=1]` (15 ca `not ok`) | 54/54 · 54/54 |
| `bin/vis-targets` | **18/18** PASS | 18/18 · 18/18 |
| self-test case âm `doc-examples` | **12/12** (7/12 → 11/12 ở ST-147 → **12/12** ở ST-153) | OK · OK |

- **Đợt 28 không đo lại Windows**: ST-157/ST-158/ST-159 **không** chạm mã Windows; số Windows ở bảng trên là số
  **đã có log**, giữ nguyên hash đo tương ứng (VD-147(4)).
- **Không ca nào lệch logic ngôn ngữ** trong 100 ca đỏ của `cargo test` (theo thông điệp: thiếu chương trình
  POSIX, đường dẫn `/bin/*`). Windows trong VS2 **vẫn "chưa hỗ trợ"** — đây là **số đo**, không phải tuyên bố hỗ trợ.

### 3.4 Log bằng chứng + sha256 (mọi tệp nằm trong `viscript/agent/JOURNAL/`)

| Nền | Tệp log | Byte | sha256 | Dùng cho |
|---|---|---|---|---|
| macOS | `st159-gate-macos.log` | 76 707 | `3fd48eedbd3d9d60146caa8fc55084c38f84df3e792171fc14f58dc25b6d467f` | **cổng cuối R28** — `[exit=0]`, **10/10** bước, L0 **1179/0**, report-check **28/177** |
| macOS | `st158-gate-macos.log` | 77 408 | `3b85215b0251f0c0c337cc6fd69087458017accd9eac40db02d25b1ae2476e48` | cổng ST-158 — L0 **1179/0**, report-check **27/177** |
| macOS | `st157-gate-macos.log` | 76 995 | `aa170105d192f05d703ad6472b6fbe0e771a3cfa163626578ca5fe5788a912d5` | cổng ST-157 — L0 **1179/0** (131 suite) |
| macOS | `st157-do-truoc.log` | 1 877 | `e574118bb77e20629c4bff80954c5e4162340e4749e689fd42a9a3ab02b29b2f` | ca đỏ-trước `vm_main_argv` **3/4 ĐỎ** (`[exit=101]`) |
| macOS | `st157-kiem-trung.log` | 1 529 | `c36ac0bdbb2c303fd2b6d75e103d962b4bf4e7db5a0d1b88dfe83159fcc30192` | máy kiểm sổ sau #110: **146 = 90/33/21/1/1** |
| macOS | `st158-truoc.log` / `st158-sau.log` | 4 180 / 4 258 | `16de77043dbdf71270cc0a9278aee4c303324540c97f5e12551c722d039f3791` / `a0aa5ed891d1f7707cfbbeb24afd2624b2a187fec944ade1566f419aa7696adf` | 4 target trước/sau #129: **13/0 · 13/0 · 3/0 · 10/0** |
| macOS | `st158-check-report.log` | 321 | `ca1df8d056cb4fe4218661c1681a3f9a5ad884b6535ef930d30d3b3e6fd16217` | `check_report --all` — **27** báo cáo, **177** hàng |
| macOS | `st157-check-report.log` | 278 | `334950546f0af12cadd0c934b0d1988f04f09cb9784180d9c201eca02577ea20` | `check_report --all` — **27** báo cáo, **177** hàng |
| macOS | `st155-vs3-dieu-tra.md` | 21 970 | `90a3fb630de95306d2cd788e3e30a94d4c9dbb696c7f68c82787e535ea0028a7` | bộ đầu vào VS3 (**222 dòng**, 57 dẫn chứng) |
| macOS | `st156-gate-macos.log` | 75 643 | `cbb34236179f5dffb3cfe971816ca46bbf724b3550a5220402efc629fa6656d8` | cổng cuối R27 (số cũ, không sửa) |
| Linux ps | `st154-landlock-ps.log` | 5 278 | `cb4ab4ab1b57f5d653b19addd21cced7dab877a74eb9557876ecb7865b95dbca` | `vis-host-linux` **15/15** |
| Linux ps | `st153-gate-ps.log` | 78 437 | `f38a2b8876d1a1b6c1a3f32f965278ab33d2cb36fd0d10571496829d724a1322` | cổng `ps` — L0 **1169/0** |
| Windows | `st153-gate-psvn.log` | 15 118 | `a229d743ade2f1c9e517e9ebc5df436ea337ff2c61e598ed28e8662c2fb4e832` | cổng psvn — **9/10 bước**, dừng **L0** |
| Windows | `st144-cargo-test.log` | 239 226 | `8b03100dd8ac63f28f3c1d49f72563506828d4ba953835ce40a445c71070841e` | cargo test **1053/100** |
| Windows | `st144-host-suite.log` | 1 226 | `f48f340a293112c54b4f9c8dfae273aa7cecaa8957ef2b928a424903b6996823` | host suite **39/54** |

### 3.4b Log của chính R28

`st159-gate-macos.log` — **76 707** byte, **2 040** dòng, sha256
`3fd48eedbd3d9d60146caa8fc55084c38f84df3e792171fc14f58dc25b6d467f`; header VD-136 (máy/OS/kernel/rustc/cargo/
python3/nproc/load/HEAD/git status + sha256 ba tệp R28 + `check_report.py`) + **10/10** bước `--- …: OK` +
`report-check: PASS (28 báo cáo, 177 hàng QUEUE phủ đủ)` + `[exit=0]` + `# thoi_diem_ket_thuc=2026-11-17T00:00:42Z`.

| Tệp | Byte | sha256 |
|---|---|---|
| `st159-gate-macos.log` | 76 707 | `3fd48eedbd3d9d60146caa8fc55084c38f84df3e792171fc14f58dc25b6d467f` |
| `reports/R28-viscript.json` | (xem `wc -c`) | `187d272f8034051ca78b47c5392e25f152d0024d4a5d27967ea63cb0e453e9bb` (băm trong header cổng — **khớp**: JSON không đổi sau khi chạy) |
| `reports/R28-viscript.md` (bản lúc chạy cổng) | — | `95f4bdb3740ff59b637217357a19183f8304e7c93f806a342367eee13f747701` — mục §3.4b này điền **sau**, nên MD đã đổi sau cổng; **không** ảnh hưởng kết luận (VD-136 chỉ băm tệp JSON) |
| `JOURNAL/st159-gate-macos-lan1-shell-fail.log` | 1 448 | (lần 1: script bọc của **chính tôi** hỏng vì `PATH` thiếu `~/.cargo/bin` + đường log tương đối sau `cd` ⇒ **cổng CHƯA chạy**; không phải cổng đỏ, không retry lấy xanh — D2-07) |

### 3.5 Số khoá khác (kiểm lại hôm nay, `fdb21d15`)

| Khoá | Số | Cách kiểm |
|---|---|---|
| R1 max (dòng/tệp) | **1498** (`crates/vis-types/src/tyck/expr.rs`) | `wc -l` toàn bộ `crates/**/*.rs` |
| `FUNCS` | **52** | `grep -vcE '^#|^$' agent/API-STD.txt` |
| `all_codes` | **33** | `grep -vcE '^#|^$' agent/API-CODES.txt` |
| `VBC1` (thẻ) | **53** | `grep -vcE '^#|^$' agent/API-VBC1.txt` |
| allowlist `doc-examples` | **6** mục | `docs/viscript/doc-examples-allow.txt` |
| `doc-examples` | **24 khối = 18 xanh + 6 ngoại lệ đỏ đúng** | dòng `--- doc-examples: OK` trong log cổng |

## 4. Năm câu bắt buộc (nguyên văn, VD-131)

> **Async không song song.**
> **Kênh không rendezvous.**
> **Net không TLS.**
> **Quyền chỉ siết mặc định khi dùng gói.**
> **LSP chỉ publish tài liệu đang mở.**

## 5. Tám hạng mục VS2 — ĐÓNG 8/8 (VD-151), cập nhật hạng mục 4 và 7

Giữ **nguyên kết luận R27**; hai hạng mục được cập nhật số đo của đợt 28 (không đổi kết luận).

| # | Hạng mục | Trạng thái (3 loại của VD-131) | Căn cứ |
|---|---|---|---|
| 1 | **Async** | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123); nợ #12, #16, #17, #19, #71, #72 |
| 2 | **Kênh** | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 (VD-123); **`#145`** ghi lệch tài liệu–mã của đường chặn `chan(0)` |
| 3 | **Net/HTTP** | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback, không TLS/HTTP2/keep-alive (VD-122); nợ #21, #83 |
| 4 | **Quyền** | **ĐÓNG** (macOS có giới hạn **VD-145**) — **cập nhật**: `vis-host-linux` **15/15** ca | Linux: Landlock thật **15/15** + **TOCTOU 0/50 000**; macOS: runtime + `--strict-caps` + ST-123/124, **đo 0/50 000** nhưng **không bảo đảm theo cơ chế** (VD-145); siết mặc định chỉ khi dùng gói (VD-124/VD-133) |
| 5 | **LSP** | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126); nợ #104 "không làm có lý do" |
| 6 | **Gói** | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration (ST-127..ST-129); mạng/chữ ký = VS3 (nợ #118) |
| 7 | **Đa nền tảng** | **ĐÓNG theo nghĩa kỹ thuật** — Linux **"chạy thật"**, Windows **"chỉ đo"**, wasm **"chỉ check"**. **Cập nhật**: Windows cổng đi **9/10 bước** rồi **dừng ở L0**, **vẫn "chỉ đo"** | Linux: gate `--fast` **PASS** L0 **1169/0** trên `ps` (§2(d)); Windows: VD-146 + `st153-gate-psvn.log` (9/10 bước, L0 150/4); wasm: VD-121 (chỉ check) |
| 8 | **Hợp chuẩn host** | **ĐÓNG** | **54/54** ca trên **macOS và Linux**, 0 lệch; `vis-host-linux` **15/15**; Windows **39/54** (chỉ đo) |

**Hạng mục 7 — ghi rõ ba mức**: Linux **"chạy thật"** (`ps`: guard band 10/10, L0 **1169/0** — xem §2(d) về
13 ca **chưa** đo lại); Windows **"chỉ đo"** (cargo test 1053/100, host 39/54, targets 18/18, self-test 12/12,
cổng **9/10 bước**, dừng L0 150/4); wasm **"chỉ check"** (18/18 ô `check`, không chạy runtime).

## 6. Nợ — trích theo số trong `SO-NO.md` (VD-147(3))

Nợ **chỉ trích bằng số trong sổ** [`SO-NO.md`](../SO-NO.md) — **không** mở lại R11..R27.

### 6.1 Chỉ số sổ (sau ST-158, `st158-sau`)

| Chỉ số sổ | Số |
|---|---|
| Tổng dòng bảng chính | **146** (dải 1..145; `#109` tách `#109a`/`#109b`) |
| **mở** | **89** |
| đóng | **34** |
| **chuyển VS3** | **21** |
| không làm có lý do | **1** (`#104`) |
| điều hướng (không phải nợ) | **1** (`#141`) |

Kiểm: 89 + 34 + 21 + 1 + 1 = **146** dòng — máy kiểm `st158-sau.log` / `st157-kiem-trung.log` khớp (0 số
trùng, dải 1..145 đủ).

### 6.2 Đóng trong đợt 28 (sau R27)

| Việc | Nợ | Task | Bằng chứng |
|---|---|---|---|
| **Đóng** | **#110** (interp↔VM parity `argv` cho tham số không-`cap` của `main`) | ST-157 | `st157-do-truoc.log` **3/4 ĐỎ** → `vm_main_argv` **4/4 xanh** cùng byte `[a, b]`; `cargo test -p vis-cli` **778/0**; sổ `st157-kiem-trung.log` **146 = 90/33/21/1/1** |
| **Đóng** | **#129** (danh sách số dòng khối tài liệu chép ở 4 tệp test) | ST-158 | `st158-truoc.log` = `st158-sau.log` (**13/0 · 13/0 · 3/0 · 10/0**); sổ **146 = 89/34/21/1/1** |
| Thêm mới | **0** | — | đợt 28 **không** cấp số nợ mới |

**Vẫn đúng chỗ, không đổi trong đợt**: **`#140` / `#109b`** — đường **GHI** kiểm-rồi-mở, trạng thái sổ là
**`mở`**, phần **sửa kỹ thuật thuộc VS3** (giới hạn đã ghi trong `HOST-BAO-DAM.md` §4); **`#142`**
(`pkg update <tên>` ghim mọi gói khác) **`mở`**; **`#137`**, **`#109a`** ở nhóm **VS3**.

### 6.3 VS2 đã đóng — không mở task VS2 mới

**VD-151**: VS2 đóng kỹ thuật **8/8**. Từ đây **không** mở task VS2 mới trừ: nợ đã có số trong `SO-NO.md`,
đính chính tài liệu, và báo cáo vòng.

## 7. VS3 — đã có **bộ đầu vào**, **chưa** bắt đầu tính năng (VD-148)

`JOURNAL/st155-vs3-dieu-tra.md` (222 dòng, sha256 `90a3fb63…`) là **bộ đầu vào thiết kế**:

- **6 mục** hiến chương §5 (Async · Kênh · Net/TLS · Quyền ghi · Windows · Gói), mỗi mục đúng **5 khối**
  (Hiện trạng · Việc cần cho VS3 · Kích thước `[ƯL]` · Rủi ro · Câu hỏi cho Agent 0) + **57** dẫn chứng `tệp:dòng`.
- **Kích thước `[ƯL]`: 19–28 task ≈ 57–84 h** (≥ 6–9 phiên 3 h); ma trận phụ thuộc (1 → 2/3/4; 3 → 6).
- **§8 — 5 câu hỏi chờ Agent 0** (sẽ online **10:00**): (1) mô hình bộ nhớ/GC; (2) chính sách crate + ngoại lệ
  `unsafe` (D2-17); (3) Windows "hỗ trợ thật" gồm gì và có được **sửa kỳ vọng** test không; (4) `select` là cú
  pháp hay hàm std (tên `select` đã là lệnh chiếu cột); (5) chốt nợ đường GHI là **`#140`**.
- **§9 sáu việc đề nghị KHÔNG làm**: HTTP/2 · siết caps mặc định toàn cục · runtime wasm · pty/tty · registry
  trần HTTP ra Internet · đóng gói GNU coreutils (GPL).

**Chưa có tài liệu thiết kế VS3 ⇒ chưa mở tính năng VS3** (VD-148 giữ nguyên). Đề nghị Agent 0: (1) trả lời
**5 câu** §8 rồi **ban hành 6 quyết định riêng** (mỗi mục một quyết định); (2) không gộp thành một tài liệu;
(3) **không** mở hàng VS3 nào trước khi có thiết kế.

## 8. Hạn chế bằng chứng (ghi trung thực)

| # | Hạn chế | Hệ quả |
|---|---|---|
| (a) | **Không có CI** trong repo — **không** tồn tại `.github/` | Mọi số **Windows** phụ thuộc SSH `psvn` do người chạy; không có cổng tự động lặp lại được |
| (b) | **Windows đo qua SSH `psvn`** ⇒ khi hỏng là `T-INFRA` **không retry được** (D2-07) | Không có đường "chạy lại để phân loại" cho Windows; đợt 28 **không** đo lại Windows |
| (c) | **Soak macOS ở HEAD cũ** `3079dede` (R24) | Số soak **10/10 = 1352 s** không đại diện HEAD `fdb21d15`; **không** chạy lại trong đợt (luật cứng: không soak) |
| (d) | **`ps` 1169 chưa đo lại** sau +9 ca ST-151 và +4 ca ST-157 (§2(d)) | Số Linux L0 **thấp hơn** thực tế dự kiến; **không** được trích 1178/1182 như số đo |
| (e) | **Windows "chỉ đo"** (VD-121/VD-146) | Không được trích Windows như "hỗ trợ"; 15 ca host + ~100 ca cargo test đỏ là nợ VS3 |
| (f) | **macOS bảo đảm theo đo, không theo cơ chế** (VD-145: không có fd→path) | TOCTOU **0/50 000** trên macOS là **kết quả đo**, không phải bảo đảm hình thức |

## 9. Kết luận

- **VS2: ĐÓNG 8/8 hạng mục** về kỹ thuật (**VD-151**); Windows/wasm có giới hạn theo VD-121/VD-146 (chỉ đo /
  chỉ check), macOS có giới hạn theo VD-145 (không fd→path ⇒ bảo đảm theo **đo**).
- **Đêm 17/11 đóng 12 hàng** (ST-147..ST-158, gồm 2 hàng báo cáo vòng R26/R27) **+ ST-159** (chính báo cáo
  R28); **không** hàng nào `BLOCKED`.
- **Sổ nợ**: **146 mục = 89 mở / 34 đóng / 21 VS3 / 1 không làm / 1 điều hướng**; đóng trong đợt 28 **#110**
  (ST-157) và **#129** (ST-158); **không** thêm số mới.
- **Cổng + máy kiểm tại `head` `fdb21d15`**: `check_report --all` **PASS (28 báo cáo, 177 hàng QUEUE)**; gate
  macOS `--fast` **PASS** `[exit=0]` 10/10 bước, L0 **1179/0** (131 suite).
- **Việc kế tiếp**: **Agent 0 trả lời 5 câu hỏi VS3** (`st155` §8) rồi ban hành tài liệu thiết kế VS3 — dòng
  ViScript **không** mở tính năng VS3 trước đó (VD-148).
