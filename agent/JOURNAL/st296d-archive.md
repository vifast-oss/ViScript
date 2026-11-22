<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296d — quy hoạch `agent/archive/` + `DECISIONS-INDEX` + ACTIVE (GĐ 1.5)

> **Người làm**: W3, 2026-11-23 · nhánh **`plan/w3-st296d-archive`** từ main **`1b27cb8`**
> (`v0.4.0` @ `c2accdf`; ST-296a/296b đã merge). **Không** push/merge/tag; không bump version,
> không xuất R34. Chỉ tài liệu `agent/`; **0 dòng code sản phẩm**.

## 1. Archive (giữ 100% lịch sử `VD-142`)

- **63 tệp** chuyển bằng `git mv` (100% R100, **0 sửa nội dung**): 44 báo cáo/handoff/snapshot/đo
  lường rời → `agent/archive/reports/`; 18 đề xuất/thiết kế/RFC/audit (gồm 3 tệp `DU-THAO-VD186/`)
  → `agent/archive/proposals/`; `STATE.md` cũ (2.256 dòng) → `agent/archive/state/STATE-2026-11-22.md`.
- Thêm `agent/archive/README.md` (quy ước bất biến) + `agent/archive/INDEX.md` (bảng 63 dòng
  `đường gốc → vị trí lưu`, dung lượng, R100).
- `agent/STATE.md` mới là **chỉ mục 16 dòng** trỏ archive/ACTIVE/reports; không chép lại số cũ.
- **Không** động `agent/reports/R##-*` (100 tệp, 2,0 M) hay `agent/JOURNAL/`: `check_report.py`
  vẫn đọc đúng đường cũ; các tệp active (`QUEUE`, `DECISIONS`, `SO-NO`, `ROADMAP`, …) giữ nguyên.

## 2. `DECISIONS-INDEX.md`

- Sinh tự động từ `agent/DECISIONS.md`: **271 mục VD** (282 số, thiếu `VD-230..VD-240` vì
  `DECISIONS.md` không định nghĩa — ghi rõ, không bịa), mỗi dòng tiêu đề 1 dòng + trạng thái
  (`đã chốt`, `bổ sung`, `đính chính`, `+N mục`) + số dòng nguồn.
- `DECISIONS.md` **không sửa** (sha/nội dung giữ nguyên; nguồn sự thật vẫn là tệp gốc).

## 3. `ACTIVE.md`

- Cập nhật 42 dòng (≤ 100): tag **`v0.4.0` @ `c2accdf`**; ST-296a merge `53990c0`, ST-296b merge
  `067e1a5`; ST-296c `plan/w1-st296c-stdlib @ 28738fe`, ST-297 `plan/w2-st297-paths`,
  ST-296d `plan/w3-st296d-archive`; GĐ 1.5 chưa bump/tag `v0.4.1`.
- Đã dùng `bin/vis-status` để chốt HEAD/dirty/đĩa/target (HEAD `1b27cb8`, dirty 0, disk 25 G,
  target 721 M lúc bắt đầu).

## 4. Đo trước/sau (dùng cho R34/`v0.4.1`)

| Chỉ số | Trước | Sau |
|---|---:|---:|
| Tệp rời ở `agent/` (maxdepth 1) | **77** | **19** |
| `agent/archive/` | 0 | **63 tệp / 1,4 M** |
| `agent/` **không tính archive** | **31 M** | **29 M** |
| `agent/reports/` | **2,0 M / 100 tệp** | **2,0 M / 100 tệp** (không đổi) |
| `report-check --all` | **PASS**, 33 báo cáo/329 hàng, **1,18 s** | **PASS**, 33/329, **0,84 s** |
| `./vis-dev check` wall | **68,60 s** | **21,70 s** (lần đầu) / **8,16 s** (lần cuối, target ấm) — không so tuyệt đối |
| `target/` | 721 M | 721 M |
| Đĩa trống | 25 G | **20 G** (> 10 G) |

## 5. Cổng

- `python3 test/report/check_report.py --all` → **PASS** (33 báo cáo, 329 hàng QUEUE).
- `./vis-dev check` → **PASS**: fmt, clippy, R1..R7, toolchain, release, doc-examples,
  report-check, readme.
- Không sửa `QUEUE`/`SO-NO`/`DECISIONS`/`reports`; không bump `0.4.1`; không xuất R34.

## 6. Blocker

Không. Một số liên kết trong tài liệu cũ (đã archive/hoặc lịch sử) vẫn ghi đường dẫn `agent/...`
cũ; `archive/INDEX.md` là bảng tra. Không push/merge/tag.
