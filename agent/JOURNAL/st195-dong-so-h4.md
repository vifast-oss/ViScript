<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# `st195` — đóng sổ `#195` trên `H4` (watchdog 2 h) — patch docs, không mã

- **Việc**: `st195` (nhãn docs; thực thi mã thuộc **`ST-311a`**, Phase C `ST-309`).
- **Nền**: `main` = `ff585af8` (`[ST-00] docs: dong bo HEAD/VD/WASI/R38 truoc peak`).
- **Phạm vi**: **chỉ docs** — `agent/SO-NO.md`, `agent/reports/R37-viscript.md`,
  `agent/STATE.md`, journal này. **0 tệp `crates/`**, **0 commit**, **0 cargo/ssh/fuzz**.
  Patch sinh từ worktree tạm `/home/ps/viswt/st195-patch` (detached `ff585af8`).

## 1. Bằng chứng đã xác minh (đọc lại tại chỗ, không suy từ báo cáo)

| Hạng mục | Giá trị | Nguồn |
|---|---|---|
| Head nghiệm thu | **`H4 = db67eabadd1a4b51f8721a242015bb3130479bbf`** | `/home/ps/vislogs/v060/R37-h4-draft.md:5`; `QUEUE.md` `ST-311f` |
| Thời lượng | **7.200,34 s** (2 h) | `watchdog2h-h4.log:10` (`finished in 7200.34s`) |
| Số ca | **74.811.821** (`tong=74811821`) | `watchdog2h-h4.log:6` |
| Vi phạm | **`vi_pham=0`** | `watchdog2h-h4.log:6` |
| Ca chậm nhất | **`max_case_ms=23355`** = 23,355 s **< 300 s** | `watchdog2h-h4.log:6` |
| Phân rã ca | `decode_tu_choi=48.566.277` · `verify_tu_choi=2.298.921` · `chay_duoc=23.946.623` | `watchdog2h-h4.log:6` |
| sha256 log | **`df24613e802793a63d746cbf4df6b2058c4a06cd26bf63407f6dfd9194045daf`** | `sha256sum` + `sha256-h4.txt` (khớp) |
| Tag `v0.6.0`/`v1.0.0-rc1` | **chưa có** | `R37-viscript.md:5-6`; `STATE.md` |

- Log ngoài repo (R7): `/home/ps/vislogs/v060/watchdog2h-h4.log` (10 dòng, không chép vào repo).
- Harness: `crates/vis-vm/tests/fuzz_mutate.rs` (watchdog + `deadline` `VS0501-TIME` từ `ST-311a`,
  commit `525f5054`); seed release `0x5EED_2026_1121` (61.111 ca / max 4,55 s / 0 vi phạm)
  là bằng chứng phụ, **không** thay thế watchdog 2 h.

## 2. Quyết định đóng sổ

1. **`#195` `mở → đóng ở ST-311a/H4`** — đủ điều kiện R37 §4.2: watchdog 2 h **0 ca > 300 s**,
   `vi_pham=0`, log + sha256 ngoài repo; `SO-NO.md` ghi đóng (điều kiện cuối của mục).
2. **§8 `SO-NO.md`: 210 = 81 mở + 110 đóng + 16 VS3 + 2 không làm + 1 điều hướng**
   (chỉ dịch 1 dòng `mở → đóng`; tổng 210, gap `#205` giữ nguyên).
3. **Tag vẫn chưa có**: đóng `#195` **không** đồng nghĩa đủ điều kiện tag; còn soak 24 h,
   fuzz parser 8 h, JSON `R37` + A0 duyệt. Không tuyên bố phát hành trong patch này.
4. **R37**: dòng 9/10 bảng §1 và dòng “Fuzz có VM + watchdog” §3 → **ĐẠT (H4)** kèm log/sha;
   **giữ lịch sử `@3d85bd2`** (“CHƯA ĐO — harness watchdog chưa có”) trong cùng ô.
   §4.2 nghiệm thu/chặn tag → `#195` đã đóng; tiêu đề §4.1/§4.2 cập nhật trạng thái thật
   (`#194` đóng ở `ST-311b`, `#195` đóng ở `ST-311a/H4`) — 2 tiêu đề này trước đó còn ghi
   “mở”/“đề xuất, chưa có dòng `SO-NO`”, trái với `SO-NO.md` hiện hành.
5. **`STATE.md:46`**: 82/109 → **81/110** + dòng `#195` đóng kèm log/sha ngắn.

## 3. Ghi chú nhãn (để A0 không nhầm số)

- Yêu cầu gốc ghi “header thêm mốc **ST-195**”. Trong sổ, **`ST-195` là task cũ đã `DONE`**
  (`QUEUE.md:466`, `assert cond, "msg"` — R20a), không liên quan nợ `#195`.
- Vì luật “không tái dùng số” (`SO-NO.md` §0), mốc trong header được ghi là
  **`ST-311a/H4`** + nhãn việc docs **`st195`** (tên journal), **không** cấp/tái dùng `ST-195`.
- Nếu A0 muốn đúng chữ “ST-195” thì đây là **quyết định đặt số**, không phải sửa kỹ thuật.

## 4. Kiểm máy của patch

- `git -C /home/ps/ViScript apply --check <patch>` **exit 0** (áp được lên `main` `ff585af8`).
- Diff chỉ **4 tệp docs**: `agent/SO-NO.md`, `agent/reports/R37-viscript.md`,
  `agent/STATE.md`, `agent/JOURNAL/st195-dong-so-h4.md`; **0 `crates/`**.
- `main` không đổi: HEAD `ff585af8`, `git status --porcelain` rỗng; worktree tạm đã gỡ sau khi sinh patch.
- Không chạy `cargo`, không `ssh`, không fuzz (theo lệnh điều phối; chain armed).
