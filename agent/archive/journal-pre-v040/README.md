<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# JOURNAL tiền-`v0.4.0` (ST-310f)

> **Chỉ đọc / bất biến.** Đây là bản lưu nguyên vị trí của **1.137 tệp** từng nằm trong
> `agent/JOURNAL/` tại tag `v0.4.0` (`c2accdf`, 2026-11-23). Chỉ `git mv`, **không sửa nội dung**,
> không xoá; lịch sử Git đi theo tệp (`git log --follow -- <đường-dẫn-mới>`), theo **VD-142**.

## 1. Phạm vi

| Loại | Số tệp |
|---|---:|
| `.log` (bằng chứng cổng/soak) | 734 |
| `.md` (nhật ký task, quyết định A0) | 313 |
| `.license` (sidecar R3) | 65 |
| `.vis` (repro ST-234) | 10 |
| `.txt` | 8 |
| `.rs` (spike VS3-W5) | 3 |
| `.tsv` | 2 |
| `.tap` | 2 |
| **Tổng** | **1.137** |

Tệp được tạo **sau** `v0.4.0` (GĐ 1.5/2/3) vẫn ở `agent/JOURNAL/`; `.gitkeep` giữ nguyên.
Hai script đã tách riêng theo ST-310f: `test/report/gen_report_json.py` (hợp nhất
`gen-r35/r36-json.py`) và `test/wasm/st300-sem-native-baseline.sh`.

## 2. Cách tra

- Tên tệp **giữ nguyên** so với `agent/JOURNAL/`; thư mục con (`st234-repro/`, `vs3-w5-spike/`)
  cũng giữ nguyên.
- Mọi tham chiếu cũ dạng `agent/JOURNAL/<tên>` trong tài liệu lịch sử tương ứng
  `agent/archive/journal-pre-v040/<tên>`; không viết lại tài liệu cũ.
- Danh sách tệp `.log`/`.log.*`/`.txt` từng > 50 KB (R7, VD-282) nay nằm trong
  `test/gate/r7-legacy-logs.txt` với **đường dẫn archive**; R7 chỉ quét `agent/JOURNAL/`
  nên các tệp đã lưu không còn tham gia cổng.
- Tìm theo nội dung: `rg "<chuỗi>" agent/archive/journal-pre-v040/`.
