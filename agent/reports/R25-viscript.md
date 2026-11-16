# R25 — ViScript, vòng 25 (chốt đợt 24g, 2026-11-16)

Báo cáo chốt **đợt 24g**. Nguồn quyết định: `QUYET-DINH-A0-2026-11-16g`
(`viscript/agent/JOURNAL/quyet-dinh-a0-2026-11-16g.md`), `viscript/agent/DECISIONS.md` —
**VD-131** (định nghĩa VS2 đóng + 5 câu bắt buộc), **VD-134/VD-148** (điều kiện "Linux chạy thật"),
**VD-142** (báo cáo đã commit không sửa), **VD-143** (danh sách hàng phụ thuộc), **VD-145/VD-146**
(giới hạn macOS / Windows chỉ đo), **VD-147(3)(4)** (sổ nợ duy nhất + hash đo từng nền) — và sổ nợ
duy nhất [`SO-NO.md`](../SO-NO.md). Báo cáo **không** sửa R1..R24; mọi sai sót của R24 đính chính ở §2.

## 0. Bảy hàng phụ thuộc — trạng thái tại thời điểm chốt (VD-143)

| Hàng | Nội dung | Trạng thái | Commit | Bằng chứng chính |
|---|---|---|---|---|
| **ST-138b** | gate `--fast` tại HEAD sau R24 (16g §2.2) | **DONE** | `c2d1ecee`, `addfe141` | PASS tại `69112900`: **L0 1159/0** (129 suite), 10/10 bước, `[exit=0]` (`st138b-gate-fast.log`) |
| **ST-140** | ca `symlink_canonicalize_hai_phia` theo **VD-144** | **DONE** | `02fb7320`, `02aa96af` | **đỏ-trước** trên `ps`; sau sửa ca xanh **hai nền**; `ps` **cargo test 1162/0** + **gate `--fast` PASS** (L0 1162/0, 10/10) |
| **ST-141** | log bằng chứng không ghi đè (**VD-147(5)**) | **DONE** | `f447f1b0`, `ee58a518` | `vis-soak` nhận `--log`/`VIS_SOAK_LOG`, cấm ghi đè; khôi phục `st113-soak.log`, tách `st138-soak.log`; `vis-host-suite` in nền thật (**54/54** cả hai nền) |
| **ST-142** | số Linux đầy đủ (nợ **#130**) | **DONE** | `fe2bbdde` | cùng HEAD `ee58a518` hai nền: `vis test` **447/447** ×2 đường, **0 SKIP**; diff 100k **lech=0**; cache 520×4 **0 lệch** |
| **ST-143** | soak 10 vòng trên `ps` | **DONE** | `e4062cd6`, `2866f8c2`, `1a275ffa` | **10/10 = 250 s** trên `ps`; `vis-linux` sync shim `vis-dev`/`vis` (chạy được `../vis-dev` trên `ps`) |
| **ST-144** | Windows: 3 việc **VD-146** + đo lại | **DONE** | `3dc13342`, `b1ee0b33` | `vis-targets` **18/18** ô; `cargo test` **1053/100**; host **39/54**; gate dừng **bước 5/10** (VS3) |
| **ST-145** | sổ nợ duy nhất `agent/SO-NO.md` (**VD-147(3)**) | **DONE** | `4f1c4d9f`, `43b19dfe` | **142 mục** (**92 mở** / 28 đóng / 20 VS3 / 1 không làm + 1 điều hướng), **trùng số 0**, tách `#109a`/`#109b` |

**ST-146 (chính báo cáo này): `READY` tại `head` = `43b19dfe`** — QUEUE tại `head` ghi `READY`; hàng
lật `DONE` ở **chính commit chứa báo cáo** (đúng **VD-143**: 7/7 hàng phụ thuộc đã `DONE` trước khi lật).
Không có hàng `BLOCKED`/`REVIEW` nào trong QUEUE ⇒ `blocked: []` trong JSON.

## 1. Tóm tắt và bảng task

1. **Đợt 24g xong 7/7 hàng phụ thuộc** (ST-138b, ST-140..ST-145), tất cả `DONE`; `check_report --all`
   **PASS 25 báo cáo / 164 hàng QUEUE**.
2. **Hạng mục 7 "Đa nền tảng" nay đủ điều kiện "Linux chạy thật"** (VD-134/VD-148): trên `ps`
   **cargo test 1162/0** (129 suite) **và gate `--fast` PASS** (L0 1162/0, 10/10 bước).
3. **Ba nền có số**: macOS 1159/0 + gate PASS; `ps` 1162/0 + gate PASS; `psvn` **chỉ đo** 1053/100
   (VS2 vẫn **"chưa hỗ trợ"** Windows/wasm theo VD-121/VD-146).
4. **Đính chính R24 đúng 3 điểm** (§2): hạng mục 7 Linux, `ST-135` JSON/MD, **hash đo từng nền**.
5. **Nợ trích theo số trong `SO-NO.md`**: **142 mục** = **92 mở** / 28 đóng / **20 VS3** / 1 không làm /
   1 điều hướng; **VS3 chưa mở tính năng** cho tới khi Agent 0 ban hành tài liệu thiết kế (VD-148).

**Tám hàng của đợt 24g** (đầy đủ 164 hàng ở bảng dưới, sinh từ `R25-viscript.json`):

| ID | Trạng thái | Commit | Cổng |
|---|---|---|---|
| ST-138b | DONE | `c2d1ecee`, `addfe141` | fast |
| ST-140 | DONE | `02fb7320`, `02aa96af` | fast |
| ST-141 | DONE | `f447f1b0`, `ee58a518` | fast |
| ST-142 | DONE | `fe2bbdde` | fast |
| ST-143 | DONE | `e4062cd6`, `2866f8c2`, `1a275ffa` | fast |
| ST-144 | DONE | `3dc13342`, `b1ee0b33` | fast |
| ST-145 | DONE | `4f1c4d9f`, `43b19dfe` | fast |
| ST-146 | **READY** (tại `head`) | — (commit báo cáo) | fast |

### 1.1 Bảng task đầy đủ — 164 hàng QUEUE, `status` khớp QUEUE tại `head` `43b19dfe`

| ID | Trạng thái | Commit | Cổng |
|---|---|---|---|
| ST-01 | DONE | `f6222a5`, `cbf512e` | fast |
| ST-02a | DONE | — | — |
| ST-02b | DONE | `c666f57`, `a48857b` | fast |
| ST-03 | DONE | `ada1ebc` | fast |
| ST-11 | DONE | `94a5342` | fast |
| ST-12 | DONE | `273a921` | fast |
| ST-12b | DONE | `dec75e3` | fast |
| ST-13 | DONE | `5e9a962` | fast |
| ST-14 | DONE | `4d52164`, `9aafdb8` | fast |
| ST-15b | DONE | `806e29a` | fast |
| ST-15 | DONE | `27aa6aa` | fast |
| ST-16 | DONE | `a89f036` | fast |
| ST-18 | DONE | `57d9e1a` | fast |
| ST-19 | DONE | `750815b` | fast |
| ST-20b | DONE | `516f668` | fast |
| ST-20 | DONE | `65cb63b` | fast |
| ST-21 | DONE | `70a5d38` | fast |
| ST-22 | DONE | `f071166` | fast |
| ST-23 | DONE | `213e691` | fast |
| ST-25 | DONE | `4a03c6d` | fast |
| ST-26 | DONE | `3a54757` | fast |
| ST-27 | DONE | `a98d0e8` | fast |
| ST-28 | DONE | `0423e67` | fast |
| ST-29 | DONE | `520c2fe`, `7633a56` | fast |
| ST-30 | DONE | `e11efae`, `02b97ad` | fast |
| ST-30b | DONE | — | — |
| ST-32 | DONE | `774369b` | fast |
| ST-33 | DONE | `f9e6db5` | fast |
| ST-34 | DONE | `4ababc6` | fast |
| ST-35 | DONE | `a0b5525` | fast |
| ST-36 | DONE | `97d3dd4` | fast |
| ST-37 | DONE | `02daf9f` | fast |
| ST-38 | DONE | `86f7804` | fast |
| ST-39 | DONE | `1c3f1d3` | fast |
| ST-40a | DONE | `9b97d75` | fast |
| ST-40b | DONE | `942bcff` | fast |
| ST-41 | DONE | `f2c256f` | fast |
| ST-42 | DONE | `331dc58` | fast |
| ST-43 | DONE | `a63914a` | fast |
| ST-44 | DONE | `32c5af5` | fast |
| ST-45 | DONE | `ea42dd1` | fast |
| ST-46 | DONE | `2207ea4` | fast |
| ST-47 | DONE | — | — |
| ST-48 | DONE | `739fc3e` | fast |
| ST-49 | DONE | `e0c084a` | fast |
| ST-50 | DONE | `da50f28` | fast |
| ST-51 | DONE | `e721858` | fast |
| ST-52 | DONE | — | — |
| ST-53 | DONE | `d68eb16` | fast |
| ST-54 | DONE | `4627fbb` | fast |
| ST-55 | DONE | — | — |
| ST-56 | DONE | `767b195` | fast |
| ST-56b | DONE | `dd04afb` | fast |
| ST-57 | DONE | `57ea7212` | fast |
| ST-58 | DONE | `abdc253` | fast |
| ST-59 | DONE | `3e2a3cd` | fast |
| ST-60 | DONE | `2542421` | fast |
| ST-60b | DONE | `f158085` | fast |
| ST-60c | DONE | `b254f03` | fast |
| ST-61 | DONE | `782f1b9` | fast |
| ST-62 | DONE | — | — |
| ST-63 | DONE | `29f236aa` | fast |
| ST-64 | DONE | `609d313d` | fast |
| ST-65 | DONE | `6514fce1` | fast |
| ST-66 | DONE | `f1654ef5` | fast |
| ST-67 | DONE | `c35a0c00` | fast |
| ST-68 | DONE | `ab1ed6c8` | fast |
| ST-69 | DONE | `40f32a40` | fast |
| ST-70 | DONE | `7743e0e2` | fast |
| ST-71 | DONE | `d85e9665` | fast |
| ST-72 | DONE | `4094a8d0` | fast |
| ST-72b | DONE | `0d7e9dba` | fast |
| ST-73 | DONE | `f9c201ba` | fast |
| ST-74 | DONE | `0af15902` | fast |
| ST-75 | DONE | `2602333d` | fast |
| ST-76 | DONE | — | fast |
| ST-77 | DONE | `920cb957` | fast |
| ST-78 | DONE | `3953f6bd` | fast |
| ST-79 | DONE | `2a62cace` | fast |
| ST-80 | DONE | `74a48476` | fast |
| ST-81 | DONE | `8aca7f7b` | fast |
| ST-82 | DONE | `9e113d5a` | fast |
| ST-83 | DONE | `7bac3c70` | fast |
| ST-84 | DONE | `2d3032b7` | fast |
| ST-85 | DONE | `12c0231c` | fast |
| ST-85b | DONE | `9dacea49` | fast |
| ST-86 | DONE | `1d690869` | fast |
| ST-87 | DONE | `0c23dad3` | fast |
| ST-87b | DONE | `32e9bebd` | fast |
| ST-88 | DONE | `b608ac16` | fast |
| ST-89 | DONE | `865342a8` | fast |
| ST-90 | DONE | `bdfd122b` | fast |
| ST-91 | DONE | `a2eb53e5` | fast |
| ST-92 | DONE | `a4586b44` | fast |
| ST-93 | DONE | `3e1ea655` | fast |
| ST-93b | DONE | `f79589ee`, `fc8717b4` | fast |
| ST-94 | DONE | `fe6d459b` | fast |
| ST-95 | DONE | `5940cb76` | fast |
| ST-96 | DONE | `d442721b` | fast |
| ST-97 | DONE | `0397c5c6` | fast |
| ST-98 | DONE | `3840d463` | fast |
| ST-99 | DONE | `f340b836` | fast |
| ST-100 | DONE | `45334bfc` | fast |
| ST-101 | DONE | `e2f25945` | fast |
| ST-101b | DONE | `494788bc` | fast |
| ST-102 | DONE | `c20c90cf` | fast |
| ST-103 | DONE | `7cf0143a` | fast |
| ST-104 | DONE | `2b55b102` | fast |
| ST-105 | DONE | `c3afa7b1` | fast |
| ST-106 | DONE | `5d7b1353` | fast |
| ST-107 | DONE | `d0fcf09d` | fast |
| ST-108 | DONE | `0cd6d471` | fast |
| ST-109 | DONE | `28c441cb` | fast |
| ST-110 | DONE | `5ae46d04` | fast |
| ST-111 | DONE | `3c67c298` | fast |
| ST-112 | DONE | `7c3f5a49` | fast |
| ST-113 | DONE | `36ed5922` | fast |
| ST-114a | DONE | `bf11a21f` | fast |
| ST-114b | DONE | `614d55e0` | fast |
| ST-115 | DONE | `6ca45ef0` | fast |
| ST-116 | DONE | `167d7cc4` | fast |
| ST-116b | DONE | `e34a8984`, `63d27ba4` | fast |
| ST-117 | DONE | `5183b81a` | fast |
| ST-118 | DONE | — | fast |
| ST-119 | DONE | `af69a4ae`, `324d1da9` | fast |
| ST-120 | DONE | `cfeee4c5`, `26e5f82c`, `6fb8c3de` | fast |
| ST-121 | DONE | `627c698a`, `3f89b207` | fast |
| ST-122 | DONE | `b0189956` | fast |
| ST-123 | DONE | `4c87cd59` | fast |
| ST-124 | DONE | `706dba00` | fast |
| ST-125 | DONE | `2a1ff16a` | fast |
| ST-126 | DONE | `1a1614cd` | fast |
| ST-127 | DONE | `769331ee` | fast |
| ST-128 | DONE | `e20eca56` | fast |
| ST-129 | DONE | `13aa18f2` | fast |
| ST-130 | DONE | `eab366d7`, `2c488ae4` | fast |
| ST-131 | DONE | `c9096552` | fast |
| ST-132 | DONE | `f9ced6c6`, `4f785f10` | fast |
| ST-132b | DONE | `a47a1ffe`, `ee73989a` | fast |
| ST-133 | DONE | `4f68110f` | fast |
| ST-134 | DONE | `6e6673bd`, `7881eb30` | fast |
| ST-135 | DONE | — | fast |
| ST-136 | DONE | `042cf378`, `5bed0dce`, `fdd60015` | fast |
| ST-137 | DONE | `e47cfb2f`, `a85193bd`, `e369698a` | fast |
| ST-138 | DONE | `a9f80d83`, `9e616960` | fast |
| ST-139 | DONE | `ef6489d1` | fast |
| ST-138b | DONE | `c2d1ecee`, `addfe141` | fast |
| ST-140 | DONE | `02fb7320`, `02aa96af` | fast |
| ST-141 | DONE | `f447f1b0`, `ee58a518` | fast |
| ST-142 | DONE | `fe2bbdde` | fast |
| ST-143 | DONE | `e4062cd6`, `2866f8c2`, `1a275ffa` | fast |
| ST-144 | DONE | `3dc13342`, `b1ee0b33` | fast |
| ST-145 | DONE | `4f1c4d9f`, `43b19dfe` | fast |
| ST-146 | READY | — | fast |
| ST-31 | DONE | `4748ec4` | fast |
| ST-24 | DONE | `c194ae7` | fast |
| ST-17 | DONE | `3cc8139`, `8059351` | fast |
| ST-04 | DONE | `663eacb` | fast |
| ST-05 | DONE | `49f2539` | fast |
| ST-06 | DONE | `590e9f7` | fast |
| ST-07 | DONE | `b149fe9` | fast |
| ST-08 | DONE | `2971a72` | fast |
| ST-09 | DONE | `ba777e9` | fast |
| ST-10 | DONE | `bc1dcf5` | fast |

## 2. Đính chính R24 (VD-142 — không sửa R24, chỉ đính chính ở đây)

| # | R24 nói (bản ghi lịch sử) | Đúng là (R25) | Căn cứ |
|---|---|---|---|
| 1 | R24 §4 ghi hạng mục 7 **"Linux: chạy thật"** | **Trái VD-134 ở thời điểm đó**: Linux khi ấy **1161/1** (129 suite) và gate **10/11 bước** (1 ca môi trường), nên trạng thái đúng là **"chờ"** tới ST-140. Nay **đã đủ điều kiện**: ST-140 cho **cargo test 1162/0** + **gate `--fast` PASS** trên `ps` ⇒ R25 ghi **Linux = "chạy thật"** | `R24-viscript.md` §4 hàng 7 · `JOURNAL/st140-cargo-test-ps.log`, `st140-gate-ps.log` · VD-134, VD-148 · dòng `ĐÍNH CHÍNH 2026-11-16 Agent 0` dưới hàng ST-138 trong `QUEUE.md` |
| 2 | R24 **JSON** ghi `ST-135 READY`, trong khi **MD §0** ghi `DONE` | **Không phải lỗi**: `check_report.py` đối chiếu `tasks[].status` với **QUEUE tại `head` của chính báo cáo** (`queue_at(head)`), mà `head` R24 = `3079dede`; tại commit đó QUEUE còn ghi `ST-135 READY`. Đó là **thiết kế** (báo cáo là ảnh chụp tại `head`), không phải mâu thuẫn nội dung | `viscript/test/report/check_report.py` (`queue_at(head)`, dòng 59–71 + 197) · `git show 3079dede:viscript/agent/QUEUE.md` · `R24-viscript.md` §0 |
| 3 | R24 **thiếu hash đo cho macOS và Linux** (chỉ Windows có `632e1fba`) | R25 ghi **hash từng nền** ở §3 (macOS, Linux `ps`, Windows `psvn`) và trong `perf.head_do` của JSON | VD-147(4) · §3 dưới đây |

Ba điểm trên trùng khớp nguyên văn dòng đính chính Agent 0 đã dán trong `QUEUE.md` (dưới hàng ST-138).

## 3. Số đo theo từng nền, kèm hash đo và log (VD-147(4))

Mọi số dưới đây lấy từ log trong `viscript/agent/JOURNAL/`; số nào không có log thì **không** ghi.
Log mang `st146` là log của chính task này.

### 3.1 macOS (máy dev `mps`, arm64) — **hash đo: `43b19dfe`** (cây trước commit báo cáo)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` | **PASS**, `[exit=0]`, **10/10** dòng `--- …: OK`, **L0 1159/0** (129 suite), 328 s — chạy **một** lần, không retry | `st146-gate-macos.log` (cổng cuối task; sha256 ở §3.4) |
| `cargo test --workspace` (= L0 của cổng) | **1159 passed / 0 failed**, 129 suite | `st145-gate-macos.log`, `st146-gate-macos.log` |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 | `st142-mac-vis-test.log`, `st142-mac-vis-test-vm.log` (HEAD `ee58a518`) |
| differential 100k seed `20261115` | **lech = 0**, **157,4 s**, 635/s (`loi_tinh=6230`, `chay=93770`) | `st142-mac-diff-100k.log` (HEAD `ee58a518`) |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **25,43 s** | `st142-mac-cache.log` (HEAD `ee58a518`) |
| host suite (54 ca) | **54/54** (`nền: Darwin arm64`) | `st141-host-suite-macos.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-macos.log` |
| soak 10 vòng | **10/10 = 1352 s** — **đo ở HEAD cũ**: log mở đầu ghi `HEAD 3079dede` (08:32Z, trước `7881eb30`); **không** chạy lại soak trong ST-146 | `st138-soak.log` |

**Hash từng lần đo trên macOS**: cổng cuối = `43b19dfe`; bộ số `vis test`/diff/cache = `ee58a518`
(ST-142); soak = `3079dede` (log của đợt trước, ghi nguyên trạng).

### 3.2 Linux (runner `ps` — Ubuntu 26.04.1, kernel 7.0.0-43-generic, rustc 1.99.1) — **hash đo: `02fb7320`** (cargo test/gate), `ee58a518` (vis test/diff/cache), `2866f8c2` (soak)

| Phép đo | Kết quả | Log |
|---|---|---|
| `cargo test --workspace` | **1162 passed / 0 failed**, 129 suite, `[exit=0]` | `st140-cargo-test-ps.log` (HEAD `02fb7320`) |
| `gate --fast` | **PASS**, `[exit=0]`, **10/10** dòng `--- …: OK`, **L0 1162/0** | `st140-gate-ps.log` (cây `02fb7320`; log ghi host HEAD `3aac2ed5` = commit dòng OS sau đó, `viscript/` không đổi) |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 | `st142-ps-vis-test.log`, `st142-ps-vis-test-vm.log` (HEAD `ee58a518`) |
| differential 100k seed `20261115` | **lech = 0**, **40,9 s**, 2448/s (`loi_tinh=6230`, `chay=93770`) | `st142-ps-diff-100k.log` (HEAD `ee58a518`) |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **6,33 s** | `st142-ps-cache.log` (HEAD `ee58a518`) |
| host suite (54 ca) | **54/54** (`nền: Linux x86_64`) | `st141-host-suite-ps.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-ps.log` |
| soak 10 vòng | **10/10 = 250 s** (gate 38/38/38 s · diff 41/40/41 s · host 1/0 s · cache 7/6 s) | `st143-soak.log` (HEAD `2866f8c2`) |
| ca đua TOCTOU đường đọc | **0/50 000** (2 lần; `ok_trong > 0`) | từ ST-137 (`R24` §2.2) |
| `vis-host-linux` (Landlock) | **13/13** ca, 3 ca âm tiến trình con | từ ST-132b |

**Lệch đã liệt kê (không sửa)**: chỉ **thời gian** — diff 157,4 s (macOS) vs 40,9 s (`ps`), cache 25,43 s
vs 6,33 s (≈3,9–4×) vì chi phí spawn tiến trình mỗi nền; `loi_tinh` **trùng 6230** ⇒ cùng bộ sinh ca.
Không lệch số đúng/sai, **không SKIP** nền nào.

### 3.3 Windows (`psvn` — Windows 10.0.26300, x86_64-pc-windows-msvc, rustc 1.99.1) — **CHỈ ĐO** (VD-121/VD-146), **hash đo: `3dc13342`**

| Phép đo | Kết quả | So với macOS / Linux |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **1053 đạt / 100 đỏ**, 129 suite, `[exit=101]` | 1159/0 · 1162/0 |
| `bin/vis-host-suite` (Git Bash) | **39/54**, `[exit=1]` (15 ca `not ok`, đúng 15 tên như ST-136) | 54/54 · 54/54 |
| `bin/vis-targets` | **18/18** PASS (ST-136: 8/18) | 18/18 · 18/18 |
| `gate --fast` (Git Bash) | **dừng ở bước 5/10**: `fmt` OK → **`clippy` OK** (hết đỏ `sandbox.rs:61`) → `luật R1..R4` OK → `self-test case âm` **5 ok / 7 not ok** ⇒ `[exit=1]` | PASS · PASS |

- **Không phải L0**: bước đỏ là **self-test `doc-examples`** — stderr của `doc_examples.py` trên Windows là
  **cp1252 + `backslashreplace`**, chuỗi kỳ vọng UTF-8 ⇒ lệch byte; **không còn** `UnicodeDecodeError`.
  Sửa tiếp là việc **VS3** (ngoài 3 việc VD-146 cho phép).
- **Không ca nào lệch logic ngôn ngữ** (theo thông điệp); 100 ca đỏ = 99 dòng `test … FAILED` + 1 gói
  `host_conformance` (15 ca con).
- Windows trong VS2 **vẫn "chưa hỗ trợ"** — đây là **số đo**, không phải tuyên bố hỗ trợ.

### 3.4 Log bằng chứng + sha256 (mọi tệp nằm trong `viscript/agent/JOURNAL/`)

| Nền | Tệp log | Byte | sha256 | Dùng cho |
|---|---|---|---|---|
| macOS | `st146-gate-macos.log` | 74 998 | `47b4055380bfdd5181441070250fe947855047e6af37176ee0fbbd1859cf1932` | cổng cuối task (L0 1159/0, 10/10) |
| macOS | `st145-gate-macos.log` | 75 217 | `8767e547fb32ae8890552b4ab683c01f3d6b87bfc9e8097ee706bd18793b356d` | cổng ST-145 (L0 1159/0) |
| macOS | `st140-gate-macos.log` | 74 560 | `62a25edeee8e0308bc3fd2684dff3945010baa00272d39dc7e9bcd49dffe3b63` | cổng ST-140 (L0 1159/0) |
| macOS | `st141-gate-macos.log` | 10 428 | `9e0fe26d5bb99fe8516aa5ad21c492c65139b0756fc57619ba3bfce3950a99ac` | cổng ST-141 lần 1 — đỏ T-INFRA (giữ nguyên) |
| macOS | `st141-gate-macos-lan2.log` | 74 194 | `d62cb72ed5218a0981191fc646cbedf743a6bcd5d0964c63d7f6f1f3411b20de` | cổng ST-141 chạy lại một lần (PASS) |
| macOS | `st138b-gate-fast.log` | 74 365 | `1c6e3d18b4eeb435a4a38cad09138c39169322ec85010590cc08f51916cc58d6` | cổng ST-138b tại 69112900 (L0 1159/0) |
| macOS | `st142-mac-vis-test.log` | 14 939 | `2667692dba0ef8b9a77c3a5d203cfa91a35b24427982d208e4b245e59ac1570c` | vis test 447 ok |
| macOS | `st142-mac-vis-test-vm.log` | 14 940 | `c71bf764667c00f2e65fcdd57e54412984ae5d0e164513005235e51c90ac8c12` | vis test --vm 447 ok |
| macOS | `st142-mac-diff-100k.log` | 742 | `f5842c13a06e47920d1c21f4c9f4b19b4cf0f22d9dcb701f4450d029859c7915` | diff 100k lech=0 (157,4 s) |
| macOS | `st142-mac-cache.log` | 1 144 | `d942e2228e09dcc72c78e0cc2453e26a6dab6c11da4b0db60cfb200b4d6a6e66` | cache 520×4 0 lệch (25,43 s) |
| macOS | `st141-host-suite-macos.log` | 2 628 | `60e3cc700ae20bfe9d68bb9506a03759f84b029b85c0b9544970e00c34cf1a1c` | host suite 54/54 |
| macOS | `st134-vis-targets-macos.log` | 1 062 | `a617adb1ef96e0889fe2fda454d81f99356e7010ef61d634139a81750fd30484` | targets 18/18 |
| macOS | `st138-soak.log` | 227 621 | `d79df3c2cf25f6ad6d37ba2cf418273d3fa8375b1935a8e6b2dacfaf610628f0` | soak 10/10 = 1352 s (HEAD cũ 3079dede) |
| Linux ps | `st140-cargo-test-ps.log` | 74 018 | `733a77b8bc9b64dd3550de2ce69b78efae2fc9bd4246279823ec6a92b4857219` | cargo test 1162/0 |
| Linux ps | `st140-gate-ps.log` | 76 267 | `647c925be791f17cce4e70207bff173e0dd6c3d1270663865e567b3e0bbdc627` | gate --fast PASS, L0 1162/0, 10/10 |
| Linux ps | `st140-gate-ps-lan1-sai-duong-dan.log` | 1 949 | `9c1272114295588394970aef9389a459ae5d13651c8f2fdf24ef10469970507c` | lần 1 sai đường dẫn (T-INFRA, exit 127) |
| Linux ps | `st142-ps-vis-test.log` | 15 798 | `0f6d717883c0aa7b2cef78ab845ee285c281559990b1a8aef906668ef6673663` | vis test 447 ok |
| Linux ps | `st142-ps-vis-test-vm.log` | 15 799 | `88a7bc6adedb646e1b6fe787f4e4bc83e265112982a09440a43d356cae9b9b8b` | vis test --vm 447 ok |
| Linux ps | `st142-ps-diff-100k.log` | 1 600 | `2b88adec765da11a41563955a7335391f74c63516b9766d61f9b94a511630722` | diff 100k lech=0 (40,9 s) |
| Linux ps | `st142-ps-cache.log` | 1 956 | `5e9d551220057a70b6b65813e7500654aa97dddfbd7b62d226b897fb43b74451` | cache 520×4 0 lệch (6,33 s) |
| Linux ps | `st141-host-suite-ps.log` | 3 668 | `6c1e985d3b136bf0440d7d25fa20500381b95cb8a9d6dc6416737d9a0fd77e4f` | host suite 54/54 |
| Linux ps | `st134-vis-targets-ps.log` | 1 731 | `bc356a5a45647f347ba7e6d4187cd403300a5819dfbcc6853fbab477fd2a2767` | targets 18/18 |
| Linux ps | `st143-soak.log` | 228 281 | `faf765e1332fdc38a4096aa5ca52c9625fc659d8a6a36f185ca17dad165ebdbe` | soak 10/10 = 250 s (HEAD 2866f8c2) |
| Linux ps | `st143-soak-lan1-gate-1-do.log` | 41 961 | `f3500cad30b312cbf08497fb3b5ab1fc6095755ddeadafe06af6c691f1b0032c` | soak lần 1 dừng gate-1 (T-INFRA, giữ nguyên) |
| Linux ps | `st143-shim-ps.log` | 2 204 | `94e3ee6363225ebe9e1d0967611e22ffc4af0a197c987697080c6d9735085cd9` | shim vis-dev/vis chạy được trên ps |
| Linux ps | `st143-soak-env.log` | 4 224 | `9adf2e31b1baa38dcd4f3f9cf8ca085ed16f1fe9de03ab58a35c1744a851ff81` | log môi trường VD-136 của soak |
| Windows psvn | `st144-cargo-test.log` | 239 226 | `8b03100dd8ac63f28f3c1d49f72563506828d4ba953835ce40a445c71070841e` | cargo test 1053/100 |
| Windows psvn | `st144-host-suite.log` | 1 226 | `f48f340a293112c54b4f9c8dfae273aa7cecaa8957ef2b928a424903b6996823` | host suite 39/54 |
| Windows psvn | `st144-vis-targets.log` | 1 793 | `9beaccc4934ccbdd9ee713cd4ca7e66c185274dd6e495c67a13839448d4192cf` | targets 18/18 |
| Windows psvn | `st144-gate-fast.log` | 7 748 | `498b1e5626b9055efd5d070f17247809963443e90dbee9c14d2ae9da54608bfe` | gate dừng bước 5/10 (self-test doc-examples) |
| Windows psvn | `st144-setup-psvn.log` | 856 | `a8e8f7c08354ddc13a7108615ec2b14318f7399a0ebe8bff982d62e2648402fc` | setup.ps1: 2 target đã cài |

### 3.5 Số khoá khác (kiểm lại hôm nay)

| Khoá | Số | Cách kiểm |
|---|---|---|
| R1 max (dòng/tệp) | **1498** (`crates/vis-types/src/tyck/expr.rs`) | `wc -l` |
| `FUNCS` | **52** | `grep -vcE '^#|^$' agent/API-STD.txt` |
| `all_codes` | **33** | `grep -vcE '^#|^$' agent/API-CODES.txt` |
| `VBC1` (thẻ) | **53** | `grep -vcE '^#|^$' agent/API-VBC1.txt` |
| allowlist `doc-examples` | **6** mục | `docs/viscript/doc-examples-allow.txt` |
| `doc-examples` | **24 khối = 18 xanh + 6 ngoại lệ đỏ đúng** | dòng `--- doc-examples: OK` trong log cổng |

## 4. Năm câu bắt buộc (nguyên văn, VD-131) và tám hạng mục VS2

### 4.1 Năm câu bắt buộc — nguyên văn

> **Async không song song.**
> **Kênh không rendezvous.**
> **Net không TLS.**
> **Quyền chỉ siết mặc định khi dùng gói.**
> **LSP chỉ publish tài liệu đang mở.**

### 4.2 Tám hạng mục VS2 — ba trạng thái (đã đóng đo được / chưa có trong VS2 → VS3 / chờ môi trường)

| # | Hạng mục | Trạng thái | Căn cứ |
|---|---|---|---|
| 1 | **Async** | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123); nợ #12, #16, #17, #19, #71, #72 |
| 2 | **Kênh** | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 (VD-123) |
| 3 | **Net/HTTP** | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback, không TLS/HTTP2/keep-alive (VD-122); nợ #21, #83 |
| 4 | **Quyền** | **ĐÓNG** (macOS có giới hạn **VD-145**) | macOS: runtime + `--strict-caps` + ST-123/124, **đo 0/50 000** nhưng **không bảo đảm theo cơ chế** (VD-145); Linux: Landlock thật **13/13** + **TOCTOU 0/50 000** sau ST-137; siết mặc định chỉ khi dùng gói (VD-124/VD-133) |
| 5 | **LSP** | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126); nợ #104 "không làm có lý do" |
| 6 | **Gói** | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration (ST-127..ST-129); mạng/chữ ký = VS3 (nợ #118) |
| 7 | **Đa nền tảng** | **ĐÓNG theo nghĩa kỹ thuật** — Linux **"chạy thật"**, Windows **chỉ đo**, wasm **chỉ check** | Linux: **`cargo test` 1162/0** + **gate `--fast` PASS** trên `ps` (ST-140) ⇒ đủ điều kiện VD-134; Windows: VD-146 (chỉ đo); wasm: VD-121 (chỉ check) |
| 8 | **Hợp chuẩn host** | **ĐÓNG** | **54/54** ca trên **macOS và Linux**, 0 lệch; `vis-host-linux` **13/13**; Windows **39/54** (chỉ đo) |

**Hạng mục 7 — ghi rõ ba mức**: Linux **"chạy thật"** (căn cứ ST-140: `cargo test 1162/0` + `gate --fast`
PASS trên `ps`, log `st140-cargo-test-ps.log` / `st140-gate-ps.log`); Windows **"chỉ đo"** (VD-146:
1053/100, host 39/54, targets 18/18, gate dừng bước 5/10); wasm **"chỉ check"** (VD-121: 18/18 ô `check`,
không chạy runtime).

## 5. Nợ — trích theo số trong `SO-NO.md` (VD-147(3)), và việc đợt 25 / VS3

Từ R25, nợ **chỉ trích bằng số trong sổ** [`SO-NO.md`](../SO-NO.md) — **không** mở lại R11..R24.

| Chỉ số sổ | Số |
|---|---|
| Tổng dòng bảng chính | **142** (dải 1..141; `#109` tách `#109a`/`#109b`) |
| **mở** | **92** |
| đóng | **28** |
| **chuyển VS3** | **20** |
| không làm có lý do | **1** (`#104`) |
| điều hướng (không phải nợ) | **1** (`#141`) |

**Ưu tiên đợt 25** (số trong sổ): **#140 / #109b** (đường **GHI** vẫn kiểm-rồi-mở — cần văn bản giới hạn
của Agent 0; dòng giới hạn trong `HOST-BAO-DAM.md` đã có từ `69112900`), **#129** (4 tệp test chép cùng
danh sách số dòng khối tài liệu), **#133** (ca `read_dir` mới kiểm luật chỉ-đọc, chưa kiểm luật ghi),
**#135** (`vis-linux` phải sync `vis-host-wasm/`; chưa có kiểm tự động danh sách sync), **#136** (phân loại
thân `cmd` theo tên lệnh: biến trùng tên lệnh muốn làm biểu thức — chưa có ca khoá), **#132** (3 ca âm dựng
helper bằng `rustc` thiếu `crt-static` ⇒ đỏ vì môi trường), **#138** (`/proc` phải đọc được trên Linux, nếu
không ⇒ `VS0604`), **#125** (`--sandbox` chưa cấp `EXECUTE` cho `/lib*`), **#117** (`@1.2.3` = ghim, khác
"không toán tử ⇒ `^`"), **#110–#112** (VM không đưa `argv`; tyck nhận `let e: cap[Dir] = json.decode(…)`;
hai bộ escape JSON).

**Không trích bằng số**: `#113b`/`#114b` (§7.3 của sổ — **chưa có số chuẩn**); đề nghị Agent 0 cấp
**142** và **143** (không đánh lại số cũ).

**Việc đợt 25 / VS3 (VD-148)**: VS3 **chưa** bắt đầu tính năng khi Agent 0 chưa ban hành tài liệu thiết kế
VS3; trong lúc chờ, dòng ViScript chỉ làm **dọn dẹp, đo và trả nợ VS2**. Ba đề nghị cụ thể: (1) đính chính
`HOST-BAO-DAM.md` (mục "bộ hợp chuẩn host" còn câu "chạy THẬT trên macOS" + "Windows/Linux/wasm: CHƯA ĐO");
(2) cấp số **142/143** cho `#113b`/`#114b`; (3) chốt xác nhận **ID `ST-138b`** (xem §6).

## 6. Ngoài kế hoạch, T-INFRA, câu hỏi cho Agent 0, kết luận

### 6.1 Ngoài kế hoạch / cần Agent 0 biết (mỗi mục 1–3 dòng)

| # | Việc | Ghi chú |
|---|---|---|
| (a) | **ĐÍNH CHÍNH ID** | 16g §4 đánh số hàng gate là **`ST-139`**, nhưng `ST-139` đã dùng cho PR #24 (`ef6489d1`) ⇒ Agent 1 đã ghi hàng đó thành **`ST-138b`** (chú thích ngay dưới hàng ST-139 trong `QUEUE.md`). **Xin Agent 0 xác nhận `ST-138b` hoặc đổi số** ở tệp quyết định kế tiếp |
| (b) | `bin/vis-soak` rò biến môi trường | `VIS_SOAK_LOG` (do ST-141 tạo) rò sang lệnh con ⇒ vòng `gate-1` của soak lần 1 đỏ **T-INFRA**; ST-143 vá **2 dòng `unset`** ở `2866f8c2` rồi chạy lại **một** lần (10/10) |
| (c) | `bin/vis-linux` sync shim | ST-143 thêm 2 dòng `rsync` cho shim gốc (`vis-dev`, `vis`) ⇒ `cd viscript && ../vis-dev` **chạy được trên `ps`** (`rules` exit 0, sha256 hai phía trùng) |
| (d) | Windows: self-test `doc_examples.py` | stderr **cp1252 + `backslashreplace`** ⇒ 7 fixture "bị bắt nhưng sai lý do"; gate Windows **dừng bước 5/10**. Sửa tiếp là **VS3** (ngoài 3 việc VD-146) |
| (e) | `psvn` ở **detached HEAD** | Đầu ST-144 `psvn` đứng ở `632e1fba` (detached); `git pull --ff-only` trần báo "specify which branch"; đã `git checkout main` rồi pull ⇒ `3dc13342`, cây sạch |
| (f) | BOM của `infra/windows/setup.ps1` | PS 5.1 đọc tệp **không BOM** theo cp1252 ⇒ hỏng script; kiểm lại hôm nay: 3 byte đầu `ef bb bf` (**có BOM**), JOURNAL ST-144 §1 ghi "BOM giữ nguyên". **Rủi ro còn nguyên**: công cụ sửa tệp có thể cắt BOM ⇒ cần kiểm BOM sau mỗi lần sửa tệp `.ps1` |
| (g) | `HOST-BAO-DAM.md` còn câu cũ | Mục "Bộ hợp chuẩn host" (dòng **41**, **55**, và bảng dòng **36**) vẫn ghi "chạy **THẬT** trên macOS" + "**Windows/Linux/wasm: CHƯA ĐO**", trong khi Linux **54/54** và Windows **39/54** đã đo. **Lưu ý**: tệp chỉ có **106 dòng** — **không có** dòng 679 như kế hoạch ghi; câu "chạy thật trên macOS" bản quyết định nằm ở `JOURNAL/quyet-dinh-a0-2026-11-16.md:256` (tệp của Agent 0) |
| (h) | "11 bước" trong kế hoạch | 16g §4 ghi tiêu chí ST-140 "gate --fast trên ps PASS **11/11**", nhưng cổng chỉ có **10** bước `--- …: OK` (cả macOS và `ps`; `st140-gate-ps.log`, `st145-gate-macos.log`) ⇒ **không có bước thứ 11**; đếm đúng là **10/10** |

### 6.2 T-INFRA — ghi trung thực, không giấu (D2-07: chạy lại tối đa một lần chỉ để phân loại)

| Task | Lần 1 | Phân loại | Xử lý |
|---|---|---|---|
| ST-141 | gate macOS đỏ ở `vistest::tests::stderr_nhieu_khong_bi_qua_han_oan` (stderr về 0/220000 byte dưới tải L0) | **T-INFRA** (flake tải), chạy riêng 5/5 xanh | Giữ nguyên log `st141-gate-macos.log`, chạy lại **một** lần ⇒ PASS (`st141-gate-macos-lan2.log`); **không** nới ngưỡng, **không** sửa ca |
| ST-143 | soak lần 1 dừng ở `gate-1` (31 s, L0 1161/1) vì biến `VIS_SOAK_LOG` rò vào `cargo test` ⇒ `soak_script` gọi lại `vis-soak` thấy tệp log đang ghi ⇒ exit 2 | **lỗi của chính mình** (không phải cổng đỏ): sửa **gốc** ở `2866f8c2` (2 dòng `unset`) | Chạy lại **một** lần ⇒ **10/10 = 250 s**; log lần 1 giữ nguyên `st143-soak-lan1-gate-1-do.log` |
| ST-140 | lệnh gate `ps` lần 1 trả **127** vì kế hoạch chép nguyên văn `cd viscript && ../vis-dev` mà shim gốc chưa sync | **T-INFRA đường gọi**, **không** phải cổng đỏ | Dùng `bin/vis-linux`; sau đó ST-143 sync shim để đường `../vis-dev` chạy được; log `st140-gate-ps-lan1-sai-duong-dan.log` giữ nguyên |

Ba lần trên **không** phải "retry để lấy xanh": mỗi lần đều **phân loại nguyên nhân** và **sửa gốc** (hoặc
ghi nhận T-INFRA) trước khi chạy lại đúng **một** lần.

### 6.3 Cần Agent 0 quyết

1. **ID `ST-138b`**: xác nhận (hoặc đổi số) cho hàng gate của đợt 24g — 16g §4 vẫn ghi `ST-139`.
2. **Số nợ 142/143** cho hai nợ đợt 23 chưa có số chuẩn (`#113b` = `pkg update` ghim mọi gói khác;
   `#114b` = `NamePolicy::Strict` cho mọi tệp có `use`).
3. **`HOST-BAO-DAM.md`**: đính chính câu "chạy THẬT trên macOS"/"Windows/Linux/wasm: CHƯA ĐO" (mục 6.1 (g));
   tệp do Agent 0 sở hữu.

### 6.4 Kết luận

- **VS2: 8/8 hạng mục đóng theo nghĩa kỹ thuật**; Windows/wasm có **giới hạn** theo VD-121/VD-146 (chỉ
  đo / chỉ check), macOS có **giới hạn** theo VD-145 (không có fd→path ⇒ bảo đảm theo **đo**, không theo cơ chế).
- **Ba hạng mục chuyển VS3** theo VD-122/VD-123 (Async, Kênh, Net/HTTP) — **không** mở tính năng VS3 cho tới
  khi Agent 0 ban hành tài liệu thiết kế VS3 (VD-148).
- **Đợt 24g xong**: 7/7 hàng phụ thuộc `DONE`; `check_report --all` **PASS** (25 báo cáo, 164 hàng QUEUE);
  cổng macOS `--fast` **PASS** (L0 1159/0) tại `head` `43b19dfe`.
