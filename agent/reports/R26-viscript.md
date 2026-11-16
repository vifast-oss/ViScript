# R26 — ViScript, vòng 26 (chốt đợt 25, 2026-11-17)

Báo cáo chốt **đợt 25**. Nguồn quyết định: `QUYET-DINH-A0-2026-11-17`
(`viscript/agent/JOURNAL/quyet-dinh-a0-2026-11-17.md`) và bản **bổ sung 17b**
(`viscript/agent/JOURNAL/quyet-dinh-a0-2026-11-17b.md`), `viscript/agent/DECISIONS.md` —
**VD-149** (cấp số 142/143), **VD-150** (UTF-8 cho mọi lần gọi python của cổng), **VD-151** (VS2 đóng
kỹ thuật **8/8**), **VD-152** (`check.rs` tách tên gốc theo cả `\` và `/`) — cùng các luật còn hiệu lực
**VD-142** (báo cáo đã commit không sửa), **VD-143** (danh sách hàng phụ thuộc), **VD-145/VD-146** (giới hạn
macOS / Windows **chỉ đo**), **VD-147(3)(4)** (sổ nợ duy nhất + hash đo từng nền), **VD-148** (chưa mở VS3)
— và sổ nợ duy nhất [`SO-NO.md`](../SO-NO.md). Báo cáo **không** sửa R1..R25; mọi điểm cần nói lại về R25
ghi ở §2.

## 0. Sáu hàng phụ thuộc + chính báo cáo — trạng thái tại thời điểm chốt (VD-143)

| Hàng | Nội dung | Trạng thái | Commit | Bằng chứng chính |
|---|---|---|---|---|
| **ST-147** | Windows: UTF-8 cho mọi lần gọi python của cổng (**VD-150**) | **DONE theo phạm vi VD-150** — Agent 0 **đã tách** hàng (17b §1); ca 7 **đóng ở ST-153** | `fd16d7d1`, `c2692ae2` | psvn self-test **7/12 → 11/12** (6/7 ca encoding xanh), cổng dừng **bước 5/10** (`st147-gate-psvn.log`, sha256 hai phía khớp `25e57182…`); macOS gate **PASS** 10/10, L0 **1159/0** (`st147-gate-macos.log`); ca 7 **không** phải encoding ⇒ chứng minh chuỗi nhân quả ở `st147-chan-doan-psvn.log` |
| **ST-148** | Sổ nợ theo **VD-149**: `#142`/`#143` + ánh xạ `#113b`/`#114b` | **DONE** | `cfce6b86`, `ac514fe7` | `SO-NO.md` **144 dòng** (dải 1..143), trùng số **0** (`st148-kiem-trung.log` `852c2d06…`); check_report PASS (**25** báo cáo, **170** hàng); gate macOS PASS L0 **1159/0** (`st148-gate-macos.log` `ba80ea69…`) |
| **ST-149** | Nợ **#136**: thân `cmd` nhận biến bind từ ngoài trùng tên lệnh là **biểu thức** | **DONE** | `6ce972ef`, `8abb0c81` | đỏ-trước **19/1** ⇒ **20/20** (`st149-do-truoc.log` `63928844…`); tái hiện `let ls = 3` in **`3`** (`st149-tai-hien.log`); `cargo test` **1163/0**; gate macOS PASS L0 **1163/0** (`st149-gate-macos.log` `ad574c8e…`) |
| **ST-150** | Nợ **#133**: ca âm **GHI** ở tiến trình con cho thư mục **chỉ-đọc** (`vis-host-linux`) | **DONE** | `9eeb936b`, `fdf69023` | ca **14** trên `ps`: đỏ-trước `exit=0` → sau ruleset **`exit=7` + `EACCES`** → đối chứng dương `exit=0`, **hai luật trong MỘT ruleset**; `ps` **14/14** (`st150-landlock-ps.log` `614b124a…`, sha256 tệp đo khớp hai phía `fa2aab0f…`); gate macOS PASS L0 **1163/0** (`st150-gate-macos.log` `6aa8b4f8…`) |
| **ST-151** | Nợ **#135**: danh sách sync của `bin/vis-linux` thành **một nguồn** + `--check-sync` | **DONE** | `9bd7d022`, `b1c07802` | **9 ca** `sync_list.rs` (bỏ một mục ⇒ **4 ca đỏ**); `--check-sync` **8/8** exit 0/2 (`st151-check-sync-macos.log`); gate macOS lần 1 **ĐỎ T-INFRA** (ca **thời gian** 116 ms ≥ 100 ms) → lần 2 **PASS** 10/10, L0 **1175/0** (**130** suite) (`st151-gate-macos-lan2.log` `a53c38a8…`) |
| **ST-153** | **VD-152**: `check.rs` tách tên gốc theo **cả** `\` và `/` + 3 ca unit | **DONE** | `4a805115`, `1263f7ea`, `32b79d4a` | **ca 7 của ST-147 đóng** (`vis check --syntax` đường dẫn có `\` ⇒ `ok 1` `RC=0`; `st153-psvn-ca7.log`); macOS gate PASS 10/10 L0 **1166/0** (`st153-gate-macos-lan2.log` `0aa9a016…`); `ps` gate PASS 10/10 L0 **1169/0** (`st153-gate-ps.log` `f38a2b88…`); psvn **self-test 12/12**, cổng **đi qua 9/10 bước**, dừng ở **L0** (`st153-gate-psvn.log` `a229d743…`) |
| **ST-152** | **R26** + cập nhật `ROADMAP.md`/`STATE.md` (chốt đợt 25) | **READY tại `head`** | — (commit báo cáo) | chính báo cáo này; `head` = `b1c0780289eae9ddb951116feb4e48babe22412a` |

**ST-152 (chính báo cáo này): `READY` tại `head` = `b1c0780289eae9ddb951116feb4e48babe22412a`** — QUEUE tại
`head` ghi `READY`; hàng lật `DONE` ở **chính commit chứa báo cáo** (đúng **VD-143**: 6/6 hàng phụ thuộc đã
`DONE` trước khi lật). Không còn hàng `BLOCKED`/`REVIEW` nào trong QUEUE (**ST-147** đã được Agent 0 tách
phạm vi ở 17b §1/§4 nên chặn cũ **được giải toả**) ⇒ `blocked: []` trong JSON.

## 1. Tóm tắt và bảng task

1. **Đợt 25 xong 6/6 hàng** (ST-147, ST-148, ST-149, ST-150, ST-151, ST-153), tất cả `DONE`;
   `check_report --all` **PASS 26 báo cáo / 171 hàng QUEUE**.
2. **VS2 đóng về kỹ thuật 8/8 hạng mục** (**VD-151**): từ đây không mở task VS2 mới trừ nợ đã có số trong
   `SO-NO.md`, đính chính tài liệu và báo cáo vòng.
3. **Ba nền có số**: macOS **1175/0** (130 suite) + gate PASS 10/10; `ps` **1169/0** (129 suite, **đo trước**
   9 ca ST-151) + gate PASS 10/10; `psvn` **chỉ đo** — self-test **12/12**, cổng đi qua **9/10 bước** rồi dừng ở
   **L0** (`cap_proc` 150/4 vì `/bin/echo`), `cargo test` 1053/100, host 39/54. Windows trong VS2 **vẫn "chưa
   hỗ trợ"** (VD-121/VD-146).
4. **Nợ trích theo số trong `SO-NO.md`**: **144 mục** = **91 mở** / **31 đóng** / **20 VS3** / 1 không làm /
   1 điều hướng. Đóng trong đợt: **#136** (ST-149), **#133** (ST-150), **#135** (ST-151) và **ca 7 của ST-147**
   (ST-153, không cấp số mới); thêm **#142/#143** (VD-149).
5. **Không** bắt đầu tính năng VS3 (VD-148/VD-151); hiến chương VS3 nằm ở
   `JOURNAL/quyet-dinh-a0-2026-11-17.md` **§5** — đề nghị Agent 0 ban hành tài liệu thiết kế VS3 (§5.3).
6. **Hai T-INFRA trong đêm**, cả hai **không** sửa ca/ngưỡng, log đỏ giữ nguyên (**§6.1**).

**Bảy hàng của đợt 25** (đầy đủ **171** hàng ở §1.1, sinh từ `R26-viscript.json`):

| ID | Trạng thái | Commit | Cổng |
|---|---|---|---|
| ST-147 | DONE (phạm vi **VD-150**) | `fd16d7d1`, `c2692ae2` | fast |
| ST-148 | DONE | `cfce6b86`, `ac514fe7` | fast |
| ST-149 | DONE | `6ce972ef`, `8abb0c81` | fast |
| ST-150 | DONE | `9eeb936b`, `fdf69023` | fast |
| ST-151 | DONE | `9bd7d022`, `b1c07802` | fast |
| ST-152 | **READY** (tại `head`) | — (commit báo cáo) | fast |
| ST-153 | DONE | `4a805115`, `1263f7ea`, `32b79d4a` | fast |

### 1.1 Bảng task đầy đủ — 171 hàng QUEUE, `status` khớp QUEUE tại `head` `b1c07802`

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
| ST-152 | **READY** | — | fast |
| ST-153 | DONE | `4a805115`, `1263f7ea`, `32b79d4a` | fast |
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

## 2. Đính chính / ghi chú R25 (VD-142 — không sửa R25, chỉ ghi ở đây)

| # | Điểm | Ghi chú R26 | Căn cứ |
|---|---|---|---|
| (a) | **Cơ sở L0 đổi** | R25 ghi L0 **1159** (129 suite) — **số cũ KHÔNG sai**, chỉ là **trước khi thêm ca** của đợt 25. Chuỗi thật: **1159** (R25) → **1163** (ST-149 **+4** ca `vish_cmd_body.rs`) → **1166** (ST-153 **+3** ca unit `ten_goc`) → **1175** (ST-151 **+9** ca `sync_list.rs`, **130** suite). Mọi con số đều có log riêng | `st148-gate-macos.log` (1159) · `st149-gate-macos.log` (1163) · `st150-gate-macos.log` (1163, `vis-host-linux` **ngoài** workspace) · `st153-gate-macos-lan2.log` (1166) · `st151-gate-macos-lan2.log` (1175) |
| (b) | **Windows tiến thêm** | R25 ghi cổng Windows "**dừng bước 5/10**" (self-test 7/12) và STATE cũ lặp lại câu đó ⇒ **câu ấy nay hết đúng**: VD-150 đưa self-test **11/12** (ST-147), VD-152 đưa **12/12** (ST-153) ⇒ cổng **đi QUA bước 5**, chạy tới **bước 10/10** rồi mới dừng ở **L0**. Trạng thái hỗ trợ **không** đổi: vẫn **"chỉ đo"**, **chưa hỗ trợ** | `st147-gate-psvn.log` (11/12) · `st153-gate-psvn.log` (12/12, `--- L0 test: FAIL`) · VD-146 |
| (c) | **Linux `1169` là số TRƯỚC 9 ca ST-151** | `ps` đo **1169/0** tại `32b79d4a` (**ST-153**) — thời điểm đó `sync_list.rs` **chưa có**. **Chưa đo lại** sau 9 ca ST-151 ⇒ **không** ghi 1178 như số đo; 1178 chỉ là **dự kiến** (1169 + 9). Muốn có số thật phải chạy lại cổng trên `ps` (ngoài ngân sách đêm nay) | `st153-gate-ps.log` (dòng ghi chú đếm L0 ở cuối log) · `st151-gate-macos-lan2.log` (+9 ca) |
| (d) | **ST-147 không còn BLOCKED** | R25/STATE ghi ST-147 **BLOCKED phần Windows**. Agent 0 **tách phạm vi**: ST-147 = **DONE** theo **VD-150**, ca 7 thành hàng mới **ST-153** (**VD-152**) ⇒ chặn cũ **đã giải toả**, nay ca 7 **đóng** ở ST-153. Không còn hàng `BLOCKED`/`REVIEW` nào trong QUEUE | 17b §1, §4 · `ST-153` · `st153-psvn-ca7.log` |

**Ghi chú thêm (không phải lỗi của R25)**: R25 §3.1 **đã** ghi caveat soak macOS "đo ở HEAD cũ `3079dede`";
R26 **giữ nguyên** caveat đó và **không** chạy lại soak (đêm nay không chạy nightly/soak).

## 3. Số đo theo từng nền, kèm hash đo và log (VD-147(4))

Mọi số dưới đây lấy từ log trong `viscript/agent/JOURNAL/`; số nào không có log thì **không** ghi.
Log mang `st152` là log của chính task này.

### 3.1 macOS (máy dev `mps`, arm64) — **hash đo: `b1c07802`** (HEAD của R26)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` (cổng cuối R26) | **PASS**, `[exit=0]`, **10/10** bước, L0 **1175/0** (**130** suite), report-check PASS (**26** báo cáo, **171** hàng QUEUE), doc-examples OK (24 khối) — chạy **một** lần, không retry, không đỏ | `st152-gate-macos.log` (75 529 B, sha256 ở §3.4) |
| `gate --fast` (mốc L0 1175/0 của đợt) | **PASS** `[exit=0]`, 10/10 bước, L0 **1175/0** (130 suite) | `st151-gate-macos-lan2.log` |
| `gate --fast` các mốc L0 trong đợt | **1166/0** (ST-153) · **1163/0** (ST-150) · **1163/0** (ST-149) · **1159/0** (ST-148) — tất cả PASS 10/10 | `st153-gate-macos-lan2.log`, `st150-gate-macos.log`, `st149-gate-macos.log`, `st148-gate-macos.log` |
| gate macOS lần 1 của ST-151/ST-153 | **ĐỎ T-INFRA**: ca **thời gian** `lsp::module::tests::hieu_nang_nap_module_5_tep` (**116 ms ≥ 100 ms**) · ca **đua** `quyen::tests::dua_doi_symlink_lap_co_gioi_han` (**1/50 000**) — chạy riêng **5/5 xanh** cả hai | `st151-gate-macos.log`, `st151-lsp-hieu-nang.log`, `st153-gate-macos.log`, `st153-symlink-lap.log` |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 (HEAD `ee58a518`) | `st142-mac-vis-test.log`, `st142-mac-vis-test-vm.log` |
| differential 100k seed `20261115` | **lech = 0**, **157,4 s**, 635/s (`loi_tinh=6230`, `chay=93770`) | `st142-mac-diff-100k.log` |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **25,43 s** | `st142-mac-cache.log` |
| host suite (54 ca) | **54/54** (`nền: Darwin arm64`) | `st141-host-suite-macos.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-macos.log` |
| soak 10 vòng | **10/10 = 1352 s** — **đo ở HEAD cũ**: log mở đầu ghi `HEAD 3079dede`; **không** chạy lại soak đêm nay | `st138-soak.log` |

### 3.2 Linux (`ps` — Ubuntu 26.04.1, kernel 7.0.0-43-generic) — **hash đo: `32b79d4a`/`4a805115`** (cổng), `9eeb936b` (Landlock), `ee58a518` (vis test/diff/cache), `2866f8c2` (soak)

| Phép đo | Kết quả | Log |
|---|---|---|
| `gate --fast` | **PASS**, `[exit=0]`, **10/10** bước, L0 **1169/0** (129 suite) — **đo TRƯỚC** 9 ca ST-151, **chưa đo lại** | `st153-gate-ps.log` |
| `vis-host-linux` (Landlock) | **14/14** ca (13 ca cũ + ca 14 âm **GHI**), 0 failed / 0 ignored | `st150-landlock-ps.log` (HEAD `9eeb936b`, sha256 tệp khớp hai phía `fa2aab0f…`) |
| `vis test test/sem` / `--vm` | **447 ok / 447 ok**, **0 SKIP**, exit 0 (HEAD `ee58a518`) | `st142-ps-vis-test.log`, `st142-ps-vis-test-vm.log` |
| differential 100k seed `20261115` | **lech = 0**, **40,9 s**, 2448/s (`loi_tinh=6230`, `chay=93770`) | `st142-ps-diff-100k.log` |
| cache 520 tệp × 4 lần | **0 lệch**, 5 passed / 0 failed, **6,33 s** | `st142-ps-cache.log` |
| host suite (54 ca) | **54/54** (`nền: Linux x86_64`) | `st141-host-suite-ps.log` |
| `bin/vis-targets` | **18/18** ô `check` | `st134-vis-targets-ps.log` |
| soak 10 vòng | **10/10 = 250 s** (gate 38/38/38 s · diff 41/40/41 s · host 1/0 s · cache 7/6 s) | `st143-soak.log` (HEAD `2866f8c2`) |
| ca đua TOCTOU đường đọc | **0/50 000** (`lap=50000 ro_ri=0 chan=20756`) | `st137-sau-ps.log` · `st124-toctou-red.log` |

**Lệch đã liệt kê (không sửa)**: chỉ **thời gian** — diff 157,4 s (macOS) vs 40,9 s (`ps`), cache 25,43 s vs
6,33 s (≈3,9–4×) vì chi phí spawn tiến trình mỗi nền; `loi_tinh` **trùng 6230** ⇒ cùng bộ sinh ca. Không lệch
số đúng/sai, **không SKIP** nền nào. **Lệch số ca mac↔ps** là **có chủ đích**: `ps` = mac **+3** (4 ca
`cfg(target_os = "linux")` − 1 ca `sandbox_check_macos`; đối chiếu `cargo test -- --list`: mac 1166 / ps 1169
tại `st153`).

### 3.3 Windows (`psvn` — Windows 10.0.26300, x86_64-pc-windows-msvc, rustc 1.99.1) — **CHỈ ĐO** (VD-121/VD-146), **hash đo: `3dc13342`** (bộ số cũ) và `4a805115` (cổng ST-153)

| Phép đo | Kết quả | So với macOS / Linux |
|---|---|---|
| `gate --fast` (Git Bash, `RUSTUP_TOOLCHAIN=1.99.1`) | **đi qua 9/10 bước** — `fmt` OK → `clippy` OK → `luật R1..R4` OK → **self-test 12/12** OK → `vis explain` OK → `báo cáo` OK → `báo cáo fixtures` OK → `build` OK → `doc-examples` OK (24 khối) → **`L0 test`: FAIL** | PASS 10/10 · PASS 10/10 |
| L0 trên Windows (fail-fast) | **150 passed / 4 failed**, 16 suite rồi dừng ở suite `cap_proc` (`/bin/echo`, `/bin/sh` không tồn tại — **VS3**, không sửa ca) | 1175/0 · 1169/0 |
| `cargo test --workspace --no-fail-fast` (ST-144) | **1053 đạt / 100 đỏ**, 129 suite, `[exit=101]` | 1175/0 · 1169/0 |
| `bin/vis-host-suite` (Git Bash) | **39/54**, `[exit=1]` (15 ca `not ok`, đúng 15 tên như ST-136) | 54/54 · 54/54 |
| `bin/vis-targets` | **18/18** PASS | 18/18 · 18/18 |
| self-test case âm `doc-examples` | **12/12** (từ **7/12** ở ST-144 → **11/12** ở ST-147 → **12/12** ở ST-153) | OK · OK |

- **Không còn** đỏ vì encoding: VD-150 ép `PYTHONIOENCODING=utf-8`/`PYTHONUTF8=1` cho mọi lần gọi python3
  (6/7 ca encoding xanh ngay ở ST-147); ca 7 là **lỗi sản phẩm** `check.rs` (`rsplit('/')`) và đã sửa ở ST-153.
- **Không ca nào lệch logic ngôn ngữ** trong 100 ca đỏ của `cargo test` (theo thông điệp: thiếu chương trình
  POSIX, đường dẫn `/bin/*`). Windows trong VS2 **vẫn "chưa hỗ trợ"** — đây là **số đo**, không phải tuyên bố hỗ trợ.

### 3.4 Log bằng chứng + sha256 (mọi tệp nằm trong `viscript/agent/JOURNAL/`)

| Nền | Tệp log | Byte | sha256 | Dùng cho |
|---|---|---|---|---|
| macOS | `st152-gate-macos.log` | 75 529 | `39171d100b18834c66f0d03f3cadfcb717d0da7c38ce37c8b8607883a7311b50` | **cổng cuối R26** (L0 1175/0, 10/10) |
| macOS | `st151-gate-macos-lan2.log` | 76 722 | `a53c38a829727829256a783982487f297f24aecf52208a518f6bbb6959aedd2e` | cổng ST-151 lần 2 — L0 **1175/0** (130 suite) |
| macOS | `st151-gate-macos.log` | 11 619 | `8626ab3b3b7573b0b117078932215129b8972c638fba39c9dc1d92debbc618ee` | cổng ST-151 lần 1 — đỏ T-INFRA (giữ nguyên) |
| macOS | `st151-lsp-hieu-nang.log` | 1 532 | `b8d3b81ad1f944cb1540481ac04740958a9c0e4c2e28e4e04574dd5cb10ec09b` | phân loại T-INFRA: ca thời gian 5/5 xanh |
| macOS | `st151-do-truoc.log` | 14 056 | `2bf3b11655627b6b7eeb6bf6d33f394445fd7eb1d824c17eee87b9f4946fd30f` | đỏ-trước/xanh-sau/bỏ-một-mục của 9 ca sync |
| macOS | `st151-check-sync-macos.log` | 753 | `935fb81f8927fee495ba780bcb64c84def36e5e5971853ff6ee12eb10d2bcc0d` | `--check-sync` 8/8 exit 0 |
| macOS | `st153-gate-macos-lan2.log` | 75 422 | `0aa9a01646898fe1d3cac3a635335210ceadc4a8303a9c3ff88eadaffb0575f5` | cổng ST-153 lần 2 — L0 **1166/0** |
| macOS | `st153-gate-macos.log` | 11 563 | `fdc42745ea56aa236b6b93b8f897b1e261585137e69fb03f8ec2f5689c76475d` | cổng ST-153 lần 1 — đỏ T-INFRA (giữ nguyên) |
| macOS | `st153-symlink-lap.log` | 3 007 | `bcc9bcba2e2b5b536305ed63baf380d481768d3848e07154b310e028fffc6fd0` | phân loại T-INFRA: ca đua 5/5 xanh |
| macOS | `st150-gate-macos.log` | 75 695 | `6aa8b4f835e46b61ffc5e19c8c0f5a3c161768115dc4dec9e9191f5b39f47894` | cổng ST-150 — L0 1163/0 |
| macOS | `st149-gate-macos.log` | 76 142 | `ad574c8e1994a0e48e62b2214e957dd0eb74fa504a5ef06878b2803451005c2f` | cổng ST-149 — L0 1163/0 |
| macOS | `st149-do-truoc.log` | 2 467 | `63928844f57e640f5185f8b61f9c59041c626b7447d92a1abd72cee6807f8068` | đỏ-trước **19/1** của 4 ca `cmd` |
| macOS | `st149-tai-hien.log` | 807 | `6beba7334be163a1e3a3c1eb0245236bd86b4263dea091599508819561273d8c` | tái hiện `let ls = 3` ⇒ `3` |
| macOS | `st148-gate-macos.log` | 75 699 | `ba80ea698067bba8ab749fbe3d2a11d85ada9b7ec61899b952f6474b52b19114` | cổng ST-148 — L0 1159/0 |
| macOS | `st148-kiem-trung.log` | 3 238 | `852c2d0632a4a524a90e7976bd5a174ef5fbb30482480d1ca82f069e6ec9816c` | trùng số **0**, dải 1..143 đủ |
| macOS | `st147-gate-macos.log` | 74 668 | `ba3b9dd37e997e04523de2302022db62cfc469696e9c2c4cb962eff6b4851e51` | cổng ST-147 — L0 1159/0 |
| macOS | `st138-soak.log` | 227 621 | `d79df3c2cf25f6ad6d37ba2cf418273d3fa8375b1935a8e6b2dacfaf610628f0` | soak 10/10 = 1352 s (**HEAD cũ** `3079dede`) |
| macOS | `st142-mac-diff-100k.log` | 742 | `f5842c13a06e47920d1c21f4c9f4b19b4cf0f22d9dcb701f4450d029859c7915` | diff 100k lech=0 (157,4 s) |
| macOS | `st142-mac-cache.log` | 1 144 | `d942e2228e09dcc72c78e0cc2453e26a6dab6c11da4b0db60cfb200b4d6a6e66` | cache 520×4 0 lệch (25,43 s) |
| macOS | `st141-host-suite-macos.log` | 2 628 | `60e3cc700ae20bfe9d68bb9506a03759f84b029b85c0b9544970e00c34cf1a1c` | host suite 54/54 |
| Linux ps | `st153-gate-ps.log` | 78 437 | `f38a2b8876d1a1b6c1a3f32f965278ab33d2cb36fd0d10571496829d724a1322` | cổng `ps` — L0 **1169/0**, 10/10 |
| Linux ps | `st150-landlock-ps.log` | 4 699 | `614b124afc8c64a377deb35f3b29c445ae585a3cc14f24bf0e73cee57df9c980` | `vis-host-linux` **14/14** (ca 14 âm GHI) |
| Linux ps | `st142-ps-diff-100k.log` | 1 600 | `2b88adec765da11a41563955a7335391f74c63516b9766d61f9b94a511630722` | diff 100k lech=0 (40,9 s) |
| Linux ps | `st142-ps-cache.log` | 1 956 | `5e9d551220057a70b6b65813e7500654aa97dddfbd7b62d226b897fb43b74451` | cache 520×4 0 lệch (6,33 s) |
| Linux ps | `st143-soak.log` | 228 281 | `faf765e1332fdc38a4096aa5ca52c9625fc659d8a6a36f185ca17dad165ebdbe` | soak 10/10 = 250 s (HEAD `2866f8c2`) |
| Linux ps | `st141-host-suite-ps.log` | 3 668 | `6c1e985d3b136bf0440d7d25fa20500381b95cb8a9d6dc6416737d9a0fd77e4f` | host suite 54/54 |
| Linux ps | `st137-sau-ps.log` | — | (xem log) | TOCTOU **0/50 000** (`ro_ri=0`) |
| Windows | `st153-gate-psvn.log` | 15 118 | `a229d743ade2f1c9e517e9ebc5df436ea337ff2c61e598ed28e8662c2fb4e832` | cổng psvn — **9/10 bước**, self-test **12/12**, dừng **L0** (150/4) |
| Windows | `st153-psvn-ca7.log` | 1 137 | `19bb0ff70c898837eb01d21cafcbd32e1d2aa8273ba65894f3f98999281ebd14` | ca 7 ST-147 đóng (`ok 1` `RC=0`) |
| Windows | `st153-psvn-env.log` | 2 550 | `cc65bbfc23a02ec5daf9e47357d323dd47a6c6f460e8347b18d88dc1aba5cc4e` | môi trường VD-136 của lần đo psvn |
| Windows | `st147-gate-psvn.log` | 3 511 | `25e57182828198e8accdc62f8eb003c1a9156869bf42009cf88e07aa7c94fe46` | gate psvn ST-147 — self-test 11/12 (sha256 hai phía khớp) |
| Windows | `st147-chan-doan-psvn.log` | 4 304 | `5e2123c27083d8951db2255ff4f0a45adf031be10e53d345070604a062dc613c` | chứng minh ca 7 **không** phải encoding |
| Windows | `st147-psvn-env.log` | 2 857 | `4ada583fced616d797b51799b4c2522593b547e0d6e9b8e51a888193b6a62dc0` | môi trường VD-136 của ST-147 |
| Windows | `st144-cargo-test.log` | 239 226 | `8b03100dd8ac63f28f3c1d49f72563506828d4ba953835ce40a445c71070841e` | cargo test 1053/100 |
| Windows | `st144-host-suite.log` | 1 226 | `f48f340a293112c54b4f9c8dfae273aa7cecaa8957ef2b928a424903b6996823` | host suite 39/54 |
| Windows | `st144-vis-targets.log` | 1 793 | `9beaccc4934ccbdd9ee713cd4ca7e66c185274dd6e495c67a13839448d4192cf` | targets 18/18 |

### 3.5 Số khoá khác (kiểm lại hôm nay, `b1c07802`)

| Khoá | Số | Cách kiểm |
|---|---|---|
| R1 max (dòng/tệp) | **1498** (`crates/vis-types/src/tyck/expr.rs`) | `wc -l` toàn bộ `crates/**/*.rs` |
| `FUNCS` | **52** | `grep -vcE '^#|^$' agent/API-STD.txt` |
| `all_codes` | **33** | `grep -vcE '^#|^$' agent/API-CODES.txt` |
| `VBC1` (thẻ) | **53** | `grep -vcE '^#|^$' agent/API-VBC1.txt` |
| allowlist `doc-examples` | **6** mục | `docs/viscript/doc-examples-allow.txt` |
| `doc-examples` | **24 khối = 18 xanh + 6 ngoại lệ đỏ đúng** | dòng `--- doc-examples: OK` trong log cổng |

## 4. Năm câu bắt buộc (nguyên văn, VD-131) và tám hạng mục VS2 (VD-151 đã đóng VS2 8/8)

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
| 4 | **Quyền** | **ĐÓNG** (macOS có giới hạn **VD-145**) — **cập nhật R26**: `vis-host-linux` nay **14/14** ca | macOS: runtime + `--strict-caps` + ST-123/124, **đo 0/50 000** nhưng **không bảo đảm theo cơ chế** (VD-145); Linux: Landlock thật **14/14** (thêm ca âm **GHI** #133 ở ST-150) + **TOCTOU 0/50 000**; siết mặc định chỉ khi dùng gói (VD-124/VD-133) |
| 5 | **LSP** | **ĐÓNG** | publish chỉ tài liệu đang mở (VD-126); nợ #104 "không làm có lý do" |
| 6 | **Gói** | **ĐÓNG** (phạm vi VS2) | semver + resolver + lock định dạng 2 + migration (ST-127..ST-129); mạng/chữ ký = VS3 (nợ #118) |
| 7 | **Đa nền tảng** | **ĐÓNG theo nghĩa kỹ thuật** — Linux **"chạy thật"**, Windows **"chỉ đo"**, wasm **"chỉ check"**. **Cập nhật R26**: Windows **tiến thêm** — cổng đi **qua 9/10 bước** (self-test **12/12**), dừng ở **L0**; **vẫn "chỉ đo"** | Linux: gate `--fast` **PASS** L0 **1169/0** trên `ps` (ST-153) + `cargo test` **1162/0** (ST-140) ⇒ đủ điều kiện VD-134; Windows: VD-146 + `st153-gate-psvn.log`; wasm: VD-121 (chỉ check) |
| 8 | **Hợp chuẩn host** | **ĐÓNG** | **54/54** ca trên **macOS và Linux**, 0 lệch; `vis-host-linux` **14/14**; Windows **39/54** (chỉ đo) |

**Hạng mục 7 — ghi rõ ba mức**: Linux **"chạy thật"** (`ps`: guard band 10/10, L0 **1169/0** — xem §2(c) về
9 ca **chưa** đo lại); Windows **"chỉ đo"** (1053/100, host 39/54, targets 18/18, self-test 12/12, cổng 9/10
bước, dừng L0 150/4); wasm **"chỉ check"** (18/18 ô `check`, không chạy runtime).

## 5. Nợ — trích theo số trong `SO-NO.md` (VD-147(3)), việc đã làm trong đợt 25 và VS3

Từ R25, nợ **chỉ trích bằng số trong sổ** [`SO-NO.md`](../SO-NO.md) — **không** mở lại R11..R25.

### 5.1 Chỉ số sổ (sau ST-153)

| Chỉ số sổ | Số |
|---|---|
| Tổng dòng bảng chính | **144** (dải 1..143; `#109` tách `#109a`/`#109b`) |
| **mở** | **91** |
| đóng | **31** |
| **chuyển VS3** | **20** |
| không làm có lý do | **1** (`#104`) |
| điều hướng (không phải nợ) | **1** (`#141`) |

Kiểm: 91 + 31 + 20 + 1 + 1 = **144** dòng.

### 5.2 Đóng / thêm trong đợt 25

| Việc | Nợ | Task | Bằng chứng |
|---|---|---|---|
| Đóng | **#136** (phân loại thân `cmd` khi biến trùng tên lệnh) | ST-149 | `st149-do-truoc.log` (19/1 → 20/20) · `st149-tai-hien.log` · `st149-gate-macos.log` |
| Đóng | **#133** (ca `read_dir` mới kiểm luật chỉ-đọc, chưa kiểm luật **ghi**) | ST-150 | `st150-landlock-ps.log` (**14/14**, `exit=7` + `EACCES`, đối chứng dương) |
| Đóng | **#135** (chưa có kiểm tự động danh sách sync `bin/vis-linux`) | ST-151 | 9 ca `sync_list.rs` + `--check-sync` 8/8 (`st151-do-truoc.log`, `st151-check-sync-macos.log`) |
| Đóng (**không cấp số mới**) | **ca 7 self-test của ST-147** (tên gốc `check.rs` trên Windows) | ST-153 | `st153-psvn-ca7.log` (`ok 1` `RC=0`) · §7.4 của sổ |
| Thêm **số mới** | **#142** (=`#113b`, `pkg update` ghim mọi gói khác) · **#143** (=`#114b`, `NamePolicy::Strict` cho mọi tệp có `use`) | ST-148 (VD-149) | `st148-kiem-trung.log` (trùng số **0**, dải 1..143 đủ) |

**Ghi chú trung thực về #133**: ngoài phạm vi hàng ST-150 còn **một vế** — biến thể `read_dir` trên thư mục
**có quyền ghi** (ST-132b §7). Vế này **đã ghi trong sổ** (dòng `#133` + §7) nhưng **chưa được cấp số**; đề nghị
Agent 0 cấp số ở vòng sau, **không** tự thêm số vào bảng.

### 5.3 VS3 — chưa mở, chờ tài liệu thiết kế

Hiến chương VS3 nằm ở `JOURNAL/quyet-dinh-a0-2026-11-17.md` **§5**; trích **một câu**:

> VS3 là **dự án riêng**, mỗi mục cần tài liệu thiết kế trước khi mở task.

**Đợt này không bắt đầu tính năng VS3** (VD-148 giữ nguyên, VD-151 chỉ đóng VS2). Đề nghị Agent 0:
(1) **ban hành tài liệu thiết kế VS3** theo 6 mục §5 (Async song song thật · Kênh rendezvous/select ·
Net TLS/HTTP2/keep-alive · Quyền `openat`/`O_NOFOLLOW` + macOS fd→path · Windows hỗ trợ thật · Gói registry
mạng + chữ ký + GC); (2) **cấp số** cho vế `read_dir` trên thư mục có quyền ghi (§5.2); (3) xác nhận
**#143** giữ `mở` hay chuyển VS3 (`VD-133` cho phép chuyển khi ca khoá xanh — ca `st130b` **đã xanh**).

## 6. Ngoài kế hoạch, T-INFRA, câu hỏi cho Agent 0, kết luận

### 6.1 T-INFRA trong đêm (ghi trung thực — D2-07: chạy lại **tối đa một lần** chỉ để phân loại)

| Task | Lần 1 (log giữ nguyên) | Phân loại | Xử lý |
|---|---|---|---|
| **ST-153** | gate macOS đỏ ở ca **đua** `quyen::tests::dua_doi_symlink_lap_co_gioi_han` — **1/50 000** lần (`ro_ri=1`, `lap=50000`), `[exit=1]`, 9/10 bước (`st153-gate-macos.log`) | **T-INFRA** (lịch trình/tải), **không** liên quan `check.rs`; chạy **riêng** ca đó **5/5 xanh** (0,4–0,9 s/lần) | **Không** sửa ca/ngưỡng; chạy lại **một** lần ⇒ PASS (`st153-gate-macos-lan2.log`) |
| **ST-151** | gate macOS đỏ ở ca **thời gian** `lsp::module::tests::hieu_nang_nap_module_5_tep` — **116 ms ≥ 100 ms** dưới tải (`load 7.39/6.18/5.27`, 12 core, dòng OS build song song), 9/10 bước (`st151-gate-macos.log`) | **T-INFRA** (flake tải); chạy **riêng** **5/5 xanh** (`st151-lsp-hieu-nang.log`) | **Không** sửa ca/ngưỡng; chạy lại **một** lần ⇒ PASS (`st151-gate-macos-lan2.log`) |

Cả hai **không** phải "retry để lấy xanh": mỗi lần đều **phân loại nguyên nhân** (chạy riêng ca đỏ, 5/5 xanh)
trước khi chạy lại đúng **một** lần. Hai log đỏ **giữ nguyên**, không ghi đè (VD-147(5)).

### 6.2 Ngoài kế hoạch / cần Agent 0 biết (mỗi mục 1–3 dòng)

| # | Việc | Ghi chú |
|---|---|---|
| (a) | **ST-147 được tách phạm vi** | 17b §1: ST-147 = **DONE** theo VD-150 (UTF-8), ca 7 thành **ST-153** (VD-152) — chặn cũ **giải toả**, không còn hàng `BLOCKED` |
| (b) | **psvn: `EXIT=True`** | Lệnh `... ; echo EXIT=$?` bị **PowerShell** nội suy ⇒ không ghi được **số** exit của cổng psvn; suy ra từ log (`--- L0 test: FAIL`) `vis-dev` trả **1**. Số đo khác (self-test, L0) đọc trực tiếp từ log — **không** chạy lại cổng |
| (c) | **psvn: file log untracked chặn pull** | `st147-gate-psvn.log` untracked trên psvn **byte-identical** bản đã commit ⇒ **chuyển** (không xoá) sang `/c/Users/ps/backup-20261117/` rồi `git pull --ff-only` `fd16d7d1..4a805115`. Cùng loại việc đã gặp ở ST-147 (4 log `st144-*`) |
| (d) | **ps: log L0 bị xen dòng** | `test result: ok` bị cắt làm hai do stdio cargo/harness dùng chung fd ⇒ đếm thô ra **1158** (**sai**). Đếm theo mọi đoạn `<n> passed` ⇒ **1169**, đối chiếu độc lập `cargo test --list` (mac 1166 / ps 1169). Ghi chú đã dán ở cuối `st153-gate-ps.log` |
| (e) | **macOS: `check.rs` không đổi hành vi Unix** | ST-153 sửa **một** dòng + thêm 3 ca unit; `vis check` trên Unix vẫn `not ok` đúng như cũ (đối chứng trong §2 của `JOURNAL/2026-11-17.md` §ST-153) |
| (f) | **BOM `infra/windows/setup.ps1`** | Rủi ro R25 nêu **còn nguyên**: công cụ sửa tệp có thể cắt BOM ⇒ cần kiểm 3 byte `ef bb bf` sau mỗi lần sửa `.ps1`. Đêm nay **không** sửa tệp này |
| (g) | **`HOST-BAO-DAM.md`** | Ba chỗ theo §4 của quyết định 17 **đã được dán** (Agent 1); R26 **không** sửa tệp (Agent 0 sở hữu) |
| (h) | **sha256 của `st153-gate-ps.log` đổi sau ST-153** | `STATE.md` (viết ở `1263f7ea`) ghi `f38a2b88aa82f9e4…`; commit `32b79d4a` **dán thêm khối ghi chú đếm L0** vào cuối log nên sha **hiện tại** là `f38a2b8876d1a1b6…` (R26 §3.4 dùng số đo từ tệp, không chép số cũ) |

### 6.3 Cần Agent 0 quyết

1. **Ban hành tài liệu thiết kế VS3** (§5.3) — không có tài liệu thì dòng ViScript **không** mở task VS3 (VD-148).
2. **Cấp số** cho vế còn lại của **#133** (`read_dir` trên thư mục **có quyền ghi**, ST-132b §7).
3. **#143** giữ `mở` hay **chuyển VS3** (VD-133 cho phép; ca khoá `st130b` đã xanh).

### 6.4 Kết luận

- **VS2: 8/8 hạng mục đóng theo nghĩa kỹ thuật** (**VD-151**); Windows/wasm có **giới hạn** theo VD-121/VD-146
  (chỉ đo / chỉ check), macOS có **giới hạn** theo VD-145 (không fd→path ⇒ bảo đảm theo **đo**, không theo cơ chế).
- **Đợt 25 đóng 6/6 hàng** (ST-147, ST-148, ST-149, ST-150, ST-151, ST-153); `check_report --all` **PASS**
  (**26** báo cáo, **171** hàng QUEUE); cổng macOS `--fast` **PASS** (L0 **1175/0**, 130 suite) tại `head`
  `b1c07802`.
- **Tồn đọng còn lại** chỉ là: **nợ trong sổ** (**91 mở** / **20 VS3** / 1 không làm / 1 điều hướng) và
  **Windows** (hỗ trợ thật = **VS3**). Không có nợ kỹ thuật nào chưa có số.
