# R27 — ViScript, vòng 27 (chốt đêm 17/11, 2026-11-17)

Báo cáo chốt **đêm 17/11** — phần **sau R26**. Nguồn quyết định: bản bổ sung **17c**
([`JOURNAL/quyet-dinh-a0-2026-11-17c.md`](../JOURNAL/quyet-dinh-a0-2026-11-17c.md)) §1–§6 và
[`DECISIONS.md`](../DECISIONS.md) — **VD-154** (đính chính `#140`; `#143` → **VS3**; cấp **`#145`**),
**VD-155** (số sổ cuối sau ST-154 là **146 = 91/32/21**), cùng các luật còn hiệu lực **VD-142** (báo cáo đã
commit không sửa), **VD-143** (danh sách hàng phụ thuộc), **VD-145/VD-146** (macOS / Windows **chỉ đo**),
**VD-147(3)(4)(5)** (sổ nợ duy nhất · hash đo từng nền · không ghi đè log), **VD-148** (chưa mở VS3),
**VD-151** (VS2 đóng kỹ thuật 8/8). Sổ nợ duy nhất: [`SO-NO.md`](../SO-NO.md) §8.
Báo cáo **không** sửa R1..R26; mọi điểm cần nói lại về R26 ghi ở **§2**.

## 0. Bốn hàng phụ thuộc + chính báo cáo — trạng thái tại thời điểm chốt (VD-143)

| Hàng | Nội dung | Trạng thái | Commit | Bằng chứng chính |
|---|---|---|---|---|
| **ST-152** | **R26** + cập nhật `ROADMAP.md`/`STATE.md` (chốt đợt 25) | **DONE** | `a9888091` | R26: 6/6 hàng đợt 25 `DONE`, VS2 đóng kỹ thuật **8/8** (VD-151), gate macOS PASS L0 **1175/0**, check_report **26** báo cáo / **171** hàng QUEUE tại `head` `b1c07802` |
| **ST-154** | Nợ **#144** (ca **dương** `read_dir` trên thư mục **có quyền ghi**) + gộp việc sổ theo **VD-154** | **DONE** | `d203b664`, `704ea55f`, `bca7370e` | ca **15** `con_read_dir_thu_muc_co_quyen_ghi_van_liet_ke_duoc`: `exit 0` + `co.txt`, đối chứng ghi `exit 0`, âm cùng ca `exit 7` + `EACCES` ⇒ `ps` **15/15** (`st154-landlock-ps.log`, sha256 tệp đo khớp hai phía `7102e90b…`); sổ **146** = 91/32/21/1/1 (`st154-kiem-trung.log`); gate macOS **PASS** 10/10, L0 **1175/0** (`st154-gate-macos-lan2.log`; lần 1 đỏ **fmt**, không phải T-INFRA) |
| **ST-155** | Điều tra **CHỈ ĐỌC** 6 mục hiến chương VS3 để làm bộ đầu vào tài liệu thiết kế | **DONE** | `7970432e` | [`st155-vs3-dieu-tra.md`](../JOURNAL/st155-vs3-dieu-tra.md) **222 dòng**, 6 mục × 5 khối, **57** dẫn chứng `tệp:dòng`, `[ƯL]` **19–28** task ≈ **57–84 h**, §8 **Top 5 câu hỏi**, §9 **6 việc đề nghị KHÔNG làm**; **không** sửa dòng mã nào; gate macOS PASS L0 **1175/0** (`st155-gate-macos.log`) |
| **ST-156** | **R27** + cập nhật `ROADMAP.md`/`STATE.md` (chốt đêm 17/11) | **READY tại `head`** | — (commit báo cáo) | chính báo cáo này; `head` = `dd0883222b080a6a1f2b8bdfe28afd385db3b5a4` |

**ST-156 (chính báo cáo này): `READY` tại `head` = `dd0883222b080a6a1f2b8bdfe28afd385db3b5a4`** — QUEUE tại
`head` ghi `READY`; hàng lật `DONE` ở **chính commit chứa báo cáo**, sau khi `check_report --all` PASS và cổng
macOS `--fast` PASS trên cây đang chốt (đúng **VD-143**: 3/3 hàng phụ thuộc **ST-152/ST-154/ST-155** đã `DONE`
trước khi lật). Không còn hàng `BLOCKED`/`REVIEW` nào trong QUEUE ⇒ `blocked: []` trong JSON.

## 1. Tóm tắt

1. **Sau R26, đêm đóng nốt phần còn lại**: **#144** đóng (ST-154), **#143 → VS3** (VD-154(b)),
   **#145** được cấp (VD-154(c) — chú thích `rt.rs` nói sai đường chặn `chan(0)`), và **ST-155** dựng
   **bộ đầu vào thiết kế VS3** (điều tra chỉ đọc, 222 dòng, 57 dẫn chứng).
2. **Đêm 17/11 đóng 8 hàng công việc** — ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-153 · ST-154 ·
   ST-155 — cộng **2 hàng báo cáo vòng** (ST-152 = R26, ST-156 = R27). Tất cả `DONE`; **không** hàng nào
   `BLOCKED`.
3. **Ba nền có số**: macOS gate **PASS 10/10**, L0 **1175/0** (130 suite); `ps` gate **PASS 10/10**, L0
   **1169/0** (129 suite — **đo trước** 9 ca ST-151, **chưa đo lại**) + `vis-host-linux` **15/15**;
   `psvn` **chỉ đo** — self-test **12/12**, cổng đi qua **9/10 bước** rồi dừng ở **L0** (`cap_proc` 150/4).
4. **Nợ trích theo số trong sổ**: **146 mục** = **91 mở** / **32 đóng** / **21 VS3** / 1 không làm /
   1 điều hướng (VD-155; dải 1..145). Đóng trong đợt: **#144**; chuyển: **#143**; thêm: **#145**.
5. **Không** bắt đầu tính năng VS3 (VD-148 giữ nguyên). VS3 **đã có bộ đầu vào** nhưng **chưa** có tài liệu
   thiết kế: **Agent 0** phải trả lời **5 câu hỏi** (`st155` §8) trước.
6. **Một lần cổng đỏ trong đêm** — ST-154 lần 1 đỏ ở bước **fmt** (không phải `T-INFRA`, không retry lấy
   xanh): sửa **khoảng trắng** rồi chạy **lần 2** PASS; log đỏ **giữ nguyên** (§6.1).

**Bốn hàng của đêm 17/11 (phần sau R26)** — đầy đủ **174** hàng ở §1.1, sinh từ `R27-viscript.json`:

| ID | Trạng thái | Commit | Cổng |
|---|---|---|---|
| ST-152 | DONE | `a9888091` | fast |
| ST-154 | DONE | `d203b664`, `704ea55f`, `bca7370e` | fast |
| ST-155 | DONE | `7970432e` | fast |
| ST-156 | **READY** (tại `head`) | — (commit báo cáo) | fast |

### 1.1 Bảng task đầy đủ — 174 hàng QUEUE, `status` khớp QUEUE tại `head` `dd088322`

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
| ST-146 | DONE | `b6e326f2` | fast |
| ST-147 | DONE | `fd16d7d1`, `c2692ae2` | fast |
| ST-148 | DONE | `cfce6b86`, `ac514fe7` | fast |
| ST-149 | DONE | `6ce972ef`, `8abb0c81` | fast |
| ST-150 | DONE | `9eeb936b`, `fdf69023` | fast |
| ST-151 | DONE | `9bd7d022`, `b1c07802` | fast |
| ST-152 | DONE | `a9888091` | fast |
| ST-153 | DONE | `4a805115`, `1263f7ea`, `32b79d4a` | fast |
| ST-154 | DONE | `d203b664`, `704ea55f`, `bca7370e` | fast |
| ST-155 | DONE | `7970432e` | fast |
| ST-156 | **READY** (tại `head`) | — | fast |
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

## 2. Đính chính (VD-142 — không sửa R26, chỉ ghi ở đây)

| # | Điểm | Ghi chú R27 | Căn cứ |
|---|---|---|---|
| (a) | **Số sổ: R26 ghi 144/91/20 — ĐÚNG tại `head` của R26** | R26 công bố **144 mục = 91 mở / 31 đóng / 20 VS3 / 1 không làm / 1 điều hướng** tại `head` `b1c07802` và **số đó đúng** ở thời điểm ấy. Sau ST-154 sổ thành **146 mục = 91 mở / 32 đóng / 21 VS3 / 1 không làm / 1 điều hướng** (VD-155) vì ba việc: `#143` mở→VS3 (mở −1, VS3 +1), thêm `#144` **đóng** (đóng +1), thêm `#145` **mở** (mở +1). ⇒ **R26 không sai**; trích số **146/91/32/21** từ R27 trở đi | VD-155 · `SO-NO.md` §8 · `st154-kiem-trung.log` (đếm lại: **146** dòng, 0 trùng, dải 1..145) |
| (b) | **Hiến chương VS3 §5: `#142` → `#140`** | Câu "Quyền: … nợ `#142/#109b`" trong hiến chương VS3 **sai số**: `#142` (VD-149) là **`pkg update` ghim mọi gói khác**; nợ **đường GHI** là **`#140`** (và `#109b`). Từ nay trích **`#140/#109b`**; bản dán cũ **giữ nguyên** (lịch sử), dòng này là **đính chính chính thức** | VD-154(a) · 17c §1 · `SO-NO.md` #140/#142/#109b |
| (c) | **Cơ sở L0 nay là 1175** | Lịch sử L0 macOS: **1159** (R25) → **1163** (ST-149, +4) → **1166** (ST-153, +3) → **1175** (ST-151, +9, **130** suite). Con số **1175** không đổi trong đêm 17/11 (ST-154/ST-155/ST-156 đều đo **1175/0**) | `st148/149/153/151-gate-macos*.log` · `st154-gate-macos-lan2.log` · `st155-gate-macos.log` · `st156-gate-macos.log` |
| (d) | **`ps` 1169 là số TRƯỚC +9 ca ST-151** | `ps` đo **1169/0** tại `32b79d4a` (ST-153), khi `sync_list.rs` (9 ca) **chưa có**. **Chưa đo lại** sau 9 ca ⇒ **không** ghi 1178 như số đo; 1178 chỉ là **dự kiến** (1169 + 9). Muốn có số thật phải chạy lại cổng trên `ps` (ngoài ngân sách đêm nay) | `st153-gate-ps.log` · `st151-gate-macos-lan2.log` |
| (e) | **Hai ca host Windows "xanh" không chứng minh gì** | `suite.txt:45` (`/bin/sh -c "exit 3"`) và `:46` (`/khong/co/lenh`) **kỳ vọng lỗi** ⇒ "xanh" vì chương trình **không tồn tại** trên Windows, **không** phải vì hỗ trợ đúng. Ghi ra để **không ai trích nhầm** hai ca này làm bằng chứng "Windows chạy được"; khi làm VS3 mục 5 phải thay bằng ca âm/dương thật | 17c §4.2 · `st155-vs3-dieu-tra.md` §5 · `test/host/suite.txt:45-46` |
| (f) | **Số hàng QUEUE là 174, không phải 173** | "173 hàng" là số hàng **trước** khi `dd088322` mở hàng **ST-156** (171 của R26 + ST-154 + ST-155 = 173). Tại `head` `dd088322` QUEUE có **174** hàng = **173 `DONE` + ST-156 `READY`**; `check_report` in đúng số nó đếm được (**174**). R27 ghi **174** ở mọi chỗ | `git show dd088322:viscript/agent/QUEUE.md` · `python3 viscript/test/report/check_report.py --all` |

**Ghi chú thêm (không phải lỗi của R26)**: caveat **soak macOS** "đo ở HEAD cũ `3079dede`" (**R24**) vẫn
**giữ nguyên**; đêm nay **không** chạy lại soak (luật cứng: không nightly/soak).

## 3. Số đo theo từng nền, kèm hash đo và log (VD-147(4))

Mọi số dưới đây lấy từ log trong `viscript/agent/JOURNAL/`; số nào không có log thì **không** ghi.
Log mang `st156` là log của chính task này.

### 3.1 macOS (máy dev `mps`, arm64) — **hash đo: `dd088322`** (HEAD của R27)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` (cổng cuối R27) | **PASS**, `[exit=0]`, **10/10** bước, L0 **1175/0** (**130** suite), report-check PASS (**27** báo cáo, **174** hàng QUEUE), doc-examples OK (24 khối) — chạy **một** lần, không retry | `st156-gate-macos.log` (sha256 ở §3.4) |
| `gate --fast` (mốc L0 1175/0 của đêm 17/11) | **PASS** `[exit=0]`, 10/10, L0 **1175/0** | `st154-gate-macos-lan2.log`, `st155-gate-macos.log` |
| `gate --fast` ST-154 **lần 1** | **ĐỎ `--- fmt: FAIL`** — rustfmt đòi xuống dòng **một** `assert!` trong ca 15; **không** phải T-INFRA ⇒ sửa **khoảng trắng**, chạy lại lần 2; log đỏ **giữ nguyên** | `st154-gate-macos.log` |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 (HEAD `ee58a518`) | `st142-mac-vis-test.log`, `st142-mac-vis-test-vm.log` |
| differential 100k seed `20261115` | **lech = 0**, **157,4 s**, 635/s (`loi_tinh=6230`, `chay=93770`) | `st142-mac-diff-100k.log` |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **25,43 s** | `st142-mac-cache.log` |
| host suite (54 ca) | **54/54** (`nền: Darwin arm64`) | `st141-host-suite-macos.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-macos.log` |
| soak 10 vòng | **10/10 = 1352 s** — **đo ở HEAD cũ**: log mở đầu ghi `HEAD 3079dede`; **không** chạy lại soak đêm nay | `st138-soak.log` |

### 3.2 Linux (`ps` — Ubuntu 26.04.1, kernel 7.0.0-43-generic) — **hash đo: `32b79d4a`** (cổng), `d203b664` (Landlock), `ee58a518` (vis test/diff/cache), `2866f8c2` (soak)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` | **PASS**, `[exit=0]`, **10/10** bước, L0 **1169/0** (129 suite) — **đo TRƯỚC** 9 ca ST-151, **chưa đo lại** | `st153-gate-ps.log` |
| `vis-host-linux` (Landlock) | **15/15** ca (13 ca cũ + ca 14 âm **GHI** + ca 15 **dương** `read_dir` trên thư mục có quyền ghi), 0 failed / 0 ignored | `st154-landlock-ps.log` (HEAD `d203b664`, sha256 tệp đo **hai phía khớp** `7102e90b…`) |
| `vis-host-linux` lần đo **1** (trước `cargo fmt`) | **15/15** xanh nhưng cây **chưa fmt** ⇒ gate lần 1 đỏ ở `fmt`; log giữ nguyên | `st154-landlock-ps-lan1-truoc-fmt.log` |
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
| L0 trên Windows (fail-fast) | **150 passed / 4 failed**, 16 suite rồi dừng ở suite `cap_proc` (`/bin/echo`, `/bin/sh` không tồn tại — **VS3**, không sửa ca) | 1175/0 · 1169/0 |
| `cargo test --workspace --no-fail-fast` (ST-144) | **1053 đạt / 100 đỏ**, 129 suite, `[exit=101]` | 1175/0 · 1169/0 |
| `bin/vis-host-suite` (Git Bash) | **39/54**, `[exit=1]` (15 ca `not ok`) | 54/54 · 54/54 |
| `bin/vis-targets` | **18/18** PASS | 18/18 · 18/18 |
| self-test case âm `doc-examples` | **12/12** (7/12 → 11/12 ở ST-147 → **12/12** ở ST-153) | OK · OK |

- **Đêm 17/11 không đo lại Windows**: cả bốn hàng của đêm (ST-154/ST-155/ST-156) **không** chạm mã Windows;
  số Windows ở bảng trên là số **đã có log**, giữ nguyên hash đo tương ứng (VD-147(4)).
- **Không ca nào lệch logic ngôn ngữ** trong 100 ca đỏ của `cargo test` (theo thông điệp: thiếu chương trình
  POSIX, đường dẫn `/bin/*`). Windows trong VS2 **vẫn "chưa hỗ trợ"** — đây là **số đo**, không phải tuyên bố hỗ trợ.
- **Đọc kèm §2(e)**: hai ca host "xanh" `/bin/sh -c "exit 3"` và `/khong/co/lenh` **không** chứng minh gì.

### 3.4 Log bằng chứng + sha256 (mọi tệp nằm trong `viscript/agent/JOURNAL/`)

| Nền | Tệp log | Byte | sha256 | Dùng cho |
|---|---|---|---|---|
| macOS | `st156-gate-macos.log` | 75 643 | `cbb34236179f5dffb3cfe971816ca46bbf724b3550a5220402efc629fa6656d8` | **cổng cuối R27** — `[exit=0]`, **10/10** bước, L0 **1175/0**, report-check **27/174** |
| macOS | `st155-gate-macos.log` | 76 461 | `551c671251976021352a8e4f29b4a4f189b8fb8bf1be857c8ac29fbd6c38347c` | cổng ST-155 — L0 **1175/0** |
| macOS | `st155-vs3-dieu-tra.md` | 21 970 | `90a3fb630de95306d2cd788e3e30a94d4c9dbb696c7f68c82787e535ea0028a7` | bộ đầu vào VS3 (**222 dòng**, 57 dẫn chứng) |
| macOS | `st154-gate-macos-lan2.log` | 76 725 | `aae5447557dc20f71b8ebfcf9eea31b42bf74f538d957ed98941e0d04ae9053b` | cổng ST-154 lần 2 — L0 **1175/0** |
| macOS | `st154-gate-macos.log` | 2 449 | `3b0a12649934c5df71b5f5e2bd6c4d264b8d193cb3d541ae117c94704cea2429` | cổng ST-154 lần 1 — đỏ **fmt** (không phải T-INFRA) |
| macOS | `st154-kiem-trung.log` | 3 148 | `ffd9b22f3b68edaa0a60270f11d8a97092a6407def2bb3787bd5a282738d4ae7` | máy kiểm sổ: **146** dòng, 0 trùng, dải 1..145 |
| macOS | `st154-check-report.log` | 322 | `342f3c26080fad7fb7e58fe3ab8ec11a7d391b621ba53de5eb878cbe62d8a650` | `check_report --all` của ST-154 (**26** báo cáo, **173** hàng) |
| Linux ps | `st154-landlock-ps.log` | 5 278 | `cb4ab4ab1b57f5d653b19addd21cced7dab877a74eb9557876ecb7865b95dbca` | `vis-host-linux` **15/15** (ca 15 dương) |
| Linux ps | `st154-landlock-ps-lan1-truoc-fmt.log` | 4 963 | `c9b79b9088f074478a7dbacdbae1ff6ed001ac0e7428c3e8de4b64cdcda7645d` | lần đo 1 (trước `cargo fmt`) — 15/15 |
| macOS | `st152-gate-macos.log` | 75 529 | `39171d100b18834c66f0d03f3cadfcb717d0da7c38ce37c8b8607883a7311b50` | cổng cuối R26 (số cũ, không sửa) |
| Linux ps | `st153-gate-ps.log` | 78 437 | `f38a2b8876d1a1b6c1a3f32f965278ab33d2cb36fd0d10571496829d724a1322` | cổng `ps` — L0 **1169/0** |
| Windows | `st153-gate-psvn.log` | 15 118 | `a229d743ade2f1c9e517e9ebc5df436ea337ff2c61e598ed28e8662c2fb4e832` | cổng psvn — **9/10 bước**, self-test **12/12**, dừng **L0** |
| Windows | `st144-cargo-test.log` | 239 226 | `8b03100dd8ac63f28f3c1d49f72563506828d4ba953835ce40a445c71070841e` | cargo test **1053/100** |
| Windows | `st144-host-suite.log` | 1 226 | `f48f340a293112c54b4f9c8dfae273aa7cecaa8957ef2b928a424903b6996823` | host suite **39/54** |
| macOS | `st143-soak.log` / `st138-soak.log` | 228 281 / 227 621 | (xem R26 §3.4) | soak `ps` **250 s** / macOS **1352 s** (HEAD cũ) |

### 3.4b Log của chính R27

`st156-gate-macos.log` — **75 643** byte, **2 024** dòng, sha256 `cbb34236179f5dffb3cfe971816ca46bbf724b3550a5220402efc629fa6656d8`;
header VD-136 (máy/OS/kernel/rustc/cargo/python3/nproc/load/HEAD/git status + sha256 tệp JSON của chính R27) +
**10/10** bước `--- …: OK` + `[exit=0]` + `# ket_thuc_utc=2026-11-16T22:06:03Z`.

| Tệp | Byte | sha256 |
|---|---|---|
| `st156-gate-macos.log` | 75 643 | `cbb34236179f5dffb3cfe971816ca46bbf724b3550a5220402efc629fa6656d8` |

### 3.5 Số khoá khác (kiểm lại hôm nay, `dd088322`)

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

Giữ **nguyên kết luận R26**; hai hạng mục được cập nhật số đo của đêm 17/11.

| # | Hạng mục | Trạng thái (3 loại của VD-131) | Căn cứ |
|---|---|---|---|
| 1 | **Async** | **chưa có trong VS2** → VS3 | tuần tự một luồng, run-to-completion (VD-123); nợ #12, #16, #17, #19, #71, #72 |
| 2 | **Kênh** | **chưa có trong VS2** → VS3 | `chan(0)` bị từ chối `VS0505`; rendezvous/select/timeout = VS3 (VD-123); **`#145`** ghi lệch tài liệu–mã của đường chặn `chan(0)` |
| 3 | **Net/HTTP** | **chưa có trong VS2** → VS3 | HTTP/1.1 loopback, không TLS/HTTP2/keep-alive (VD-122); nợ #21, #83 |
| 4 | **Quyền** | **ĐÓNG** (macOS có giới hạn **VD-145**) — **cập nhật R27**: `vis-host-linux` nay **15/15** ca | Linux: Landlock thật **15/15** (**thêm ca dương `read_dir` trên thư mục có quyền ghi** = đóng **#144**, `st154-landlock-ps.log`) + **TOCTOU 0/50 000**; macOS: runtime + `--strict-caps` + ST-123/124, **đo 0/50 000** nhưng **không bảo đảm theo cơ chế** (VD-145); siết mặc định chỉ khi dùng gói (VD-124/VD-133) |
| 5 | **LSP** | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126); nợ #104 "không làm có lý do" |
| 6 | **Gói** | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration (ST-127..ST-129); mạng/chữ ký = VS3 (nợ #118) |
| 7 | **Đa nền tảng** | **ĐÓNG theo nghĩa kỹ thuật** — Linux **"chạy thật"**, Windows **"chỉ đo"**, wasm **"chỉ check"**. **Cập nhật R27**: Windows **9/10 bước** (self-test **12/12**), dừng ở **L0**; **vẫn "chỉ đo"** | Linux: gate `--fast` **PASS** L0 **1169/0** trên `ps` (ST-153, §2(d)); Windows: VD-146 + `st153-gate-psvn.log` (9/10 bước, L0 150/4); wasm: VD-121 (chỉ check) |
| 8 | **Hợp chuẩn host** | **ĐÓNG** | **54/54** ca trên **macOS và Linux**, 0 lệch; `vis-host-linux` **15/15**; Windows **39/54** (chỉ đo) |

**Hạng mục 7 — ghi rõ ba mức**: Linux **"chạy thật"** (`ps`: guard band 10/10, L0 **1169/0** — xem §2(d) về
9 ca **chưa** đo lại); Windows **"chỉ đo"** (cargo test 1053/100, host 39/54, targets 18/18, self-test 12/12,
cổng **9/10 bước**, dừng L0 150/4); wasm **"chỉ check"** (18/18 ô `check`, không chạy runtime).

## 6. Nợ — trích theo số trong `SO-NO.md` (VD-147(3), VD-155)

Nợ **chỉ trích bằng số trong sổ** [`SO-NO.md`](../SO-NO.md) — **không** mở lại R11..R26.

### 6.1 Chỉ số sổ (sau ST-154, VD-155)

| Chỉ số sổ | Số |
|---|---|
| Tổng dòng bảng chính | **146** (dải 1..145; `#109` tách `#109a`/`#109b`) |
| **mở** | **91** |
| đóng | **32** |
| **chuyển VS3** | **21** |
| không làm có lý do | **1** (`#104`) |
| điều hướng (không phải nợ) | **1** (`#141`) |

Kiểm: 91 + 32 + 21 + 1 + 1 = **146** dòng — đếm lại bằng máy: `st154-kiem-trung.log` (**0** số trùng, dải
**1..145** đủ).

### 6.2 Đóng / chuyển / thêm trong đợt (đêm 17/11, phần sau R26)

| Việc | Nợ | Task | Bằng chứng |
|---|---|---|---|
| **Đóng** | **#144** (vế 2 của `#133`: ca **dương** `read_dir` trên thư mục **có quyền ghi**) | ST-154 | `st154-landlock-ps.log` — `ps` **15/15**, ca 15 `exit 0` + `co.txt`, đối chứng ghi `exit 0`, âm cùng ca `exit 7` + `EACCES`; sha256 tệp đo khớp hai phía `7102e90b…` |
| **Chuyển VS3** | **#143** (`NamePolicy::Strict` cho mọi tệp có `use`) | ST-154 (VD-154(b)) | điều kiện VD-133 đã thoả (ca khoá `st130b` xanh từ đợt 23); sổ: `#143` → `chuyển VS3 — VD-154(b)` |
| **Thêm (mở)** | **#145** (`rt.rs:128-129` nói sai đường chặn `chan(0)`: tyck **không** kiểm `cap`, **runtime** mới chặn `exit 1`) | ST-154 (VD-154(c)) | nguồn: `st155-vs3-dieu-tra.md` §2; xử khi có **task VS3 kênh** |

**Vẫn đúng chỗ, không đổi trong đợt**: **`#140` / `#109b`** — đường **GHI** kiểm-rồi-mở, trạng thái sổ là
**`mở`**, phần **sửa kỹ thuật thuộc VS3** (dòng giới hạn đã có trong `HOST-BAO-DAM.md` §4); **`#142`**
(`pkg update <tên>` ghim mọi gói khác) **`mở`**; **`#137`**, **`#109a`** ở nhóm **VS3** (§5 của sổ).

### 6.3 VS2 đã đóng — không mở task VS2 mới

**VD-151**: VS2 đóng kỹ thuật **8/8**. Từ đây **không** mở task VS2 mới trừ: nợ đã có số trong `SO-NO.md`,
đính chính tài liệu, và báo cáo vòng.

## 7. VS3 — đã có **bộ đầu vào**, **chưa** bắt đầu tính năng (VD-148)

`JOURNAL/st155-vs3-dieu-tra.md` (222 dòng, sha256 `90a3fb63…`) là **bộ đầu vào thiết kế**:

- **6 mục** hiến chương §5 (Async · Kênh · Net/TLS · Quyền ghi · Windows · Gói), mỗi mục đúng **5 khối**
  (Hiện trạng · Việc cần cho VS3 · Kích thước `[ƯL]` · Rủi ro · Câu hỏi cho Agent 0) + **57** dẫn chứng `tệp:dòng`.
- **Kích thước `[ƯL]`: 19–28 task ≈ 57–84 h** (≥ 6–9 phiên 3 h); **§7** bảng tổng hợp + ma trận phụ thuộc (1 → 2/3/4; 3 → 6).
- **§8 Top 5 câu hỏi** Agent 0 phải trả lời **trước khi** viết thiết kế: (1) mô hình bộ nhớ/GC; (2) chính sách
  crate + ngoại lệ `unsafe` (D2-17); (3) Windows "hỗ trợ thật" gồm gì và có được **sửa kỳ vọng** test không;
  (4) `select` là cú pháp hay hàm std (tên `select` đã là lệnh chiếu cột); (5) chốt nợ đường GHI là **`#140`**.
- **§9 sáu việc đề nghị KHÔNG làm**: HTTP/2 · siết caps mặc định toàn cục · runtime wasm · pty/tty · registry
  trần HTTP ra Internet · đóng gói GNU coreutils (GPL).

**Chưa có tài liệu thiết kế VS3 ⇒ chưa mở tính năng VS3** (VD-148 giữ nguyên). Đề nghị Agent 0:
(1) trả lời **5 câu** §8 rồi **ban hành 6 quyết định riêng** (mỗi mục một quyết định); (2) không gộp thành một
tài liệu; (3) **không** mở hàng VS3 nào trước khi có thiết kế.

## 8. Hạn chế bằng chứng (ghi trung thực)

| # | Hạn chế | Hệ quả |
|---|---|---|
| (a) | **Không có CI** trong repo — **không** tồn tại `.github/` | Mọi số **Windows** phụ thuộc SSH `psvn` do người chạy; không có cổng tự động lặp lại được. Số Windows trong báo cáo là **số đo một lần**, không tái lập bằng máy khác |
| (b) | **Windows đo qua SSH `psvn`** ⇒ khi hỏng là `T-INFRA` **không retry được** (D2-07) | Không có đường "chạy lại để phân loại" cho Windows; mọi lần đo phải trọn vẹn hoặc bỏ (đêm nay **không** đo lại Windows) |
| (c) | **Soak macOS ở HEAD cũ** `3079dede` (R24) | Số soak **10/10 = 1352 s** không đại diện HEAD `dd088322`; **không** chạy lại trong đêm (luật cứng: không soak) |
| (d) | **`ps` 1169 chưa đo lại** sau +9 ca ST-151 (§2(d)) | Số Linux L0 **thấp hơn** thực tế dự kiến (1178) — **không** được trích 1178 như số đo |
| (e) | **Windows "chỉ đo"** (VD-121/VD-146), 2 ca host "xanh" không chứng minh gì (§2(e)) | Không được trích Windows như "hỗ trợ"; 15 ca host + ~100 ca cargo test đỏ là nợ VS3 |
| (f) | **macOS bảo đảm theo đo, không theo cơ chế** (VD-145: không có fd→path) | TOCTOU **0/50 000** trên macOS là **kết quả đo**, không phải bảo đảm hình thức |

## 9. Kết luận

- **VS2: ĐÓNG 8/8 hạng mục** về kỹ thuật (**VD-151**); Windows/wasm có giới hạn theo VD-121/VD-146 (chỉ đo /
  chỉ check), macOS có giới hạn theo VD-145 (không fd→path ⇒ bảo đảm theo **đo**).
- **Đêm 17/11 đóng 8 hàng công việc** (ST-147 · ST-148 · ST-149 · ST-150 · ST-151 · ST-153 · **ST-154** ·
  **ST-155**) + **2 hàng báo cáo vòng** (ST-152 · ST-156). **Không** hàng nào `BLOCKED`.
- **Sổ nợ**: **146 mục = 91 mở / 32 đóng / 21 VS3 / 1 không làm / 1 điều hướng** (VD-155); đóng trong đợt này
  **#144**, chuyển **#143**, thêm **#145**.
- **Cổng + máy kiểm tại `head` `dd088322`**: `check_report --all` **PASS (27 báo cáo, 174 hàng QUEUE)**; gate
  macOS `--fast` **PASS** `[exit=0]` 10/10 bước, L0 **1175/0** (130 suite).
- **Việc kế tiếp**: **tài liệu thiết kế VS3** do **Agent 0** ban hành **sau khi** trả lời **5 câu hỏi** trong
  `st155` §8 — dòng ViScript **không** mở tính năng VS3 trước đó (VD-148).
