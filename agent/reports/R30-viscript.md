# R30 — ViScript, vòng 30 (đợt 30: tách repo + pha 1 công thái học, 2026-11-17 → 2026-11-18)

Đợt sau **R29** (`reports/R29-viscript.md`). Phạm vi: **tách repo ViScript** (ST-171 & các hàng phụ),
**pha 1 công thái học** theo C3 của VD-178 (ST-179..ST-203), cổng sổ (ST-197/201). **HEAD đo của R30:
`0d5e0e57c5c2ccac4d1c5904de7d07900137cb49`** (`0d5e0e5`) — commit chứa hàng QUEUE ST-204 (`READY`);
báo cáo này lật **ST-204 → DONE** ở chính commit chứa nó.

## 0. Ba hàng phụ thuộc + chính báo cáo — trạng thái tại thời điểm chốt (VD-143)

| Hàng | Nội dung | Trạng thái | Commit | Bằng chứng chính |
|---|---|---|---|---|
| **ST-202** | R19b — tràn i64 ⇒ `VS0507` + exit 101 mọi đường | **DONE** | `6dcd88f` | ca đỏ-trước `loi/doi-dau-tran`; harness 464→465; workspace **1215/0**; gate **PASS** |
| **ST-203** | R2 — bỏ từ khoá `none` (`VS0203`), `T??` lồng tuỳ ý, tự nâng nhiều lớp | **DONE** | `9a0bed6` | 4 ca đỏ-trước (`loi/none-bo`, `int-qhap-gan`, `map-get-lot`, `t-qhap-generic`); harness 465→**469**; workspace **1217/0**; gate **PASS** |
| **ST-201** | Đồng bộ tài liệu sau đợt pha 1 | **DONE** | `767d7b0` | ROADMAP “sau R29”; SO-NO đồng bộ đợt 29 (`2f62022`); ST-197 bổ sung hồi tố (`ed490d2`) |
| **ST-204** | **R30** (chính báo cáo này) | **READY tại `0d5e0e5`** | — (commit báo cáo) | `check_report --all` PASS; gate `--fast` PASS tại `0d5e0e5` |

## 1. Tóm tắt

1. **Repo ViScript tách riêng, public**: `https://github.com/vifast-oss/ViScript` — 459 commit giữ lịch sử
   (filter-repo), gốc repo **là** cây ViScript; repo cũ (ViFastOs) đã xoá cây `viscript/` + `VISCRIPT.md`
   trỏ sang. Từ ST-171, mọi việc diễn ra trên repo mới (remote `origin` = GitHub).
2. **Pha 1 công thái học chạy đầy đủ đêm 17→18/11** theo C3: P0a/P0-3 (ST-179/181), R9+C1 (ST-182),
   R6 (ST-183), R17 (ST-184), R3b/R3a (ST-185/186), R16 (ST-187), R12a (ST-188), R11 (ST-189),
   R7 (ST-190, +7b ST-193), R8 (ST-191), R5a/R5c (ST-192/193), R1a (ST-194), R20a/R20b (ST-195/196),
   **R19b** (ST-202), **R2** (ST-203). **R4 (Error record + Json enum)** chưa làm.
3. **VS2 đóng 8/8 giữ nguyên** (VD-151). **Sổ nợ giữ 147 mục** = 89 mở / 35 đóng / 21 VS3 / 1 không làm / 1 điều hướng
   (không đổi từ R29; `#147` — mac chậm hơn ps **5,70×** — vẫn **mở**, chờ A0).
4. **Cổng trên máy `ps` (Linux, host của đợt này)**: `gate --fast` **PASS 10/10** tại `0d5e0e5`
   (report-check **29 báo cáo / 223 hàng**, doc-examples **24 khối: 19 xanh + 5 ngoại lệ**);
   `cargo test --workspace --no-fail-fast` **1217 passed / 0 failed** (136 suite);
   `vis test test/sem` và `--vm`: **469/469** mỗi đường.
5. **Không đo lại macOS/Windows trong R30** — macOS chuẩn vẫn là R29 (`L0 1184/0`); Windows = **VS3** (VD-146).
6. **Lệch interp/VM pre-existing** phát hiện khi làm R2: `print(x)` khi `x` là giá trị **coerced chưa tag** —
   interp in `5`, VM in `Some(5)` (VM chèn wrap lúc lower, interp elide). Không ca sem nào vấp; ghi nhận, chờ A0.

## 2. Số đo tại HEAD `0d5e0e5` (máy `ps`, Linux 7.0.0-43, Ubuntu 26.04.1)

| Phép đo | Kết quả | Log/nguồn |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **1217 passed / 0 failed** (136 suite) | `JOURNAL/st204-r30.md` |
| `vis test test/sem` | **469 ok** | lần đo trên máy `ps` |
| `vis test --vm test/sem` | **469 ok**, 0 SKIP | lần đo trên máy `ps` |
| `./vis-dev gate --fast` | **PASS 10/10** | `JOURNAL/st204-r30.md` |
| doc-examples | **24 khối: 19 xanh + 5 ngoại lệ** | trong gate |
| workspace suites locked | sem **469**; `LOI_BIEN_DICH` **51**; `LOI_LUC_CHAY` **35**; roundtrip **469** | các tệp test |
| host_conformance | PASS (54 ca × nhiều lần chạy) | trong `cargo test --workspace` |
| gate trên commit code cuối | PASS tại `9a0bed6` (ST-203) và `0d5e0e5` (ST-204 READY) | gate-r30.log |

macOS/Windows: **không đo lại** trong R30 (§6). Số mac chuẩn của R29 và của ST-171 (gate `L0 1184/0`/`1188/0`)
vẫn là tham chiếu gần nhất; commit tách repo giữ nguyên cây mã (`viscript/` tree hash khớp giữa mac và clone ps @ `39041b3f`).

## 3. Tình trạng task tại QUEUE@head `0d5e0e5`

`QUEUE.md` tại head có **223 hàng** = **222 `DONE`** + **1 `READY`** (**ST-204**, chính báo cáo này). **0** hàng
`BLOCKED`/`REVIEW`. JSON `tasks` phủ đủ 223 hàng, `status` khớp QUEUE (máy kiểm `check_report.py`).

**Hàng mới so với R29** (R29 có 197 hàng; +26): **ST-171** (tách repo, 13 commit nhỏ a..m), **ST-178** (sửa 6 tài liệu VS3
theo VD-178), **ST-179..ST-200** (pha 1 + brand), **ST-197** (hàng QUEUE bổ sung hồi tố ngày 18/11), **ST-201**,
**ST-202**, **ST-203**, **ST-204**. Bảng đầy đủ 223 hàng: xem `agent/QUEUE.md`; commits theo từng hàng: trường `commits[]` trong JSON.

Các mốc chính (đầy đủ ở JOURNAL):

| Hàng | Nội dung | Trạng thái |
|---|---|---|
| ST-171 | Tách repo ViScript riêng, public; repo cũ xoá cây `viscript/`; `vis-remote`, clone ps/psvn | **DONE** |
| ST-178 | VD-178: sửa 6 tài liệu VS3 + st169 + vs-ergo (C1 mã thoát, C3 thứ tự) — 0 dòng mã | **DONE** |
| ST-179..ST-196 | Pha 1 theo C3 (P0, R9, R6, R17, R3a/b, R16, R12a, R11, R7, R8, R5a/R5c, R1a, R20a/b) | **DONE** |
| ST-197 | Review chéo R7/R8 (commit `e082bc6` + `ae3986d`; **hàng QUEUE lọt sổ, bổ sung hồi tố**) | **DONE** |
| ST-198/199 | Port PR #26 (record ẩn danh in `{a: int}`) và PR #27 (vị trí lỗi nhóm `( … )` đầu tệp `.vish`) | **DONE** |
| ST-200 | Brand: icon ViScript + ViShell (SVG + PNG) + README logo | **DONE** |
| ST-201 | Đồng bộ ROADMAP/STATE/SO-NO sau đợt pha 1 | **DONE** |
| ST-202 | **R19b**: tràn i64 mọi đường ⇒ `VS0507` + exit 101 | **DONE** |
| ST-203 | **R2**: bỏ từ khoá `none` (`VS0203`), `T??` lồng tuỳ ý, tự nâng nhiều lớp | **DONE** |

## 4. Nợ — giữ nguyên theo R29 §4

Sổ: **147 mục** = **89 mở / 35 đóng / 21 VS3 / 1 không làm / 1 điều hướng**; dải 1..147 trùng số **0**. Trong đợt 30 không
mở/đóng nợ nào (ST-202/ST-203 xử lý theo phiếu pha 1, không đụng sổ). **`#147`** (mac chậm hơn ps **5,70×**, ngưỡng 3×) —
vẫn **mở**, chờ A0 quyết: (i) gộp `report-check`/`rules` để bỏ tiến trình-per-tệp, hay (ii) ghi nhận “macOS phạt syscall là
giới hạn nền”. **`#145`** (chú thích `rt.rs` về `chan(0)`) — xử trong VS3 (VD-154). R5b (`{…}` chỉ map + VS0205) — **mâu thuẫn
thiết kế với ST-198/json_sort** (bản ghi ẩn danh `{a: 1}` là hợp lệ và được test khoá) — **không tự quyết**; cần A0 chốt.

## 5. Việc kế

1. **ST-204 → DONE** ở chính commit chứa báo cáo này (máy kiểm PASS trước khi lật).
2. **R4 — `Error` record + `Json` enum + `AnyError`** (pha 1, còn thiếu lớn nhất; mọi hàm std trả `Result[T, Error]`,
   `?` tự chuyển `AnyError`, `main -> Result[(), AnyError]` thoát **1** khi `Err`). Phụ thuộc R2 (đã xong).
3. **R5d** (chuẩn hoá `str(x)` vs method `std.str`), **R20a phần 2** (`use std.fs as f`, `use std.fs.{…}`),
   **R20b phần 2** (`range.step(n)`).
4. **R5b** — chờ A0 (xem §4). **R1+R2 phần còn lại**: enum-hoá `Option`/`Result` trong prelude (spec R2 “enum std”) —
   việc lớn, cần A0 xếp lịch; phiên này chỉ hạ phần **hành vi** (bỏ `none`, `T??`, tự nâng nhiều lớp).
5. **#147** — chờ A0. **VS3** — chưa mở (VD-148); sau pha 1: VS3-QĐ-1 → (QĐ-2,3,4) → QĐ-6; QĐ-5 sau R6.
6. **`check_report --all`** và **gate `--fast`** phải PASS ở repo mới cho mọi đợt đóng vòng (STD check_report đã chuyển vào repo ViScript ở ST-171).

## 6. Hạn chế bằng chứng (ghi trung thực)

1. **Không có số macOS mới** trong R30 — pha 1 chạy trên máy `ps` (Linux). Các commit không đổi mã ViScript
   ngoài phạm vi pha 1 (tách repo + ST-197 review/fix, ST-198/199 port, ST-200 brand, ST-201 docs, ST-202/203 code nhỏ).
   Số chuẩn macOS gần nhất = R29 (`L0 1184/0` @ repo cũ `39041b3f`) + ST-171 (`L0 1184/0`, gate PASS sau tách).
2. **Windows không đo** (VD-146 “chỉ đo”, VS3 mục 5) — psvn vẫn dừng ở L0 với 4 ca `cap_proc` đỏ (`/bin/echo`).
3. **`print(x)` của giá trị coerced chưa tag lệch interp/VM** (`5` vs `Some(5)`) — pre-existing, không ca nào chạm;
   chưa sửa (tránh đổi kỳ vọng khi chưa có A0).
4. **ST-197 lọt sổ** — 2 commit không hàng QUEUE/JOURNAL ở thời điểm làm; đã bổ sung hồi tố ở `ed490d2` (không giấu).
5. **Sao chép test chéo từ ViFastOs**: ST-198/199 là port PR #26/#27 (tác giả `khoajunior`, giữ nguyên tác giả) —
   ViFastOs đã xoá cây `viscript/` nên PR không merge được ở đó.
6. **Repo remote `github.com/vifast-oss/ViScript`** — public; tách bằng `git filter-repo`; push qua credential sẵn có trên máy.
7. Gate phiên này chạy trên **Linux** (`ps`); máy này **không** nói về hiệu năng/độ đúng trên macOS ngoại trừ qua R29/ST-171.

## 7. Kết luận

1. **Repo ViScript độc lập, sống**: tách xong, push public, cổng PASS trên Linux; lịch sử đầy đủ.
2. **Pha 1 công thái học gần xong** theo C3 — còn **R4** (lớn), nhỏ là **R5d**, phần 2 **R20a/R20b**, và **R5b** chờ A0.
3. **Số đo**: workspace **1217/0**; `vis test` **469/469** ×2; sem **469**; doc-examples **24 khối**; nợ **147** không đổi.
4. **Không** mở VS3; **không** đổi kỳ vọng/ca ngoài các đổi đã duyệt (VD-178); không retry-lấy-xanh ở đợt này.
5. **ST-204** lật `DONE` ở chính commit này; `check_report --all` kỳ vọng PASS (30 báo cáo, 223 hàng).
