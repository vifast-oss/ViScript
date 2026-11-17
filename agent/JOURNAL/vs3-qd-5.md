<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-5 · Windows "hỗ trợ thật"
**Trạng thái**: **ĐÃ DUYỆT CÓ SỬA** (VD-178) · đã áp [SỬA] tại **ST-178** · **Phụ thuộc**: **độc lập** (chạy song song được) · **Mở đường cho**: —

## 1. Mục tiêu đo được (theo VD-162)
1. Chạy **native**, không cần Git Bash/MSYS trong `PATH`.
2. Host suite **54/54** trên `psvn`.
3. `cargo test --workspace` **0 đỏ** — hoặc chỉ còn các ca thuộc **danh sách kỳ vọng theo nền** đã được Agent 0 duyệt (mỗi ca 1 dòng lý do).
4. Cổng Windows **10/10** và có **CI runner Windows** (self-hosted `psvn` hoặc runner dịch vụ) chạy tự động.
5. Trước khi đủ 4 điều trên: vẫn tuyên bố **“chỉ đo”**.

## 2. Hiện trạng (dẫn chứng)
- Baseline `0df923c2`: `cargo test` **1068 / 105 đỏ** (131 suite); self-test **12/12**; cổng dừng **L0** (`cap_proc` 150/4 fail-fast) — `JOURNAL/st161-*`.
- Host suite **39/54**; targets **18/18** — `JOURNAL/st144-*`.
- Phân loại 105 ca: **a=48 / b=15 / c=42** — `JOURNAL/st169-bang-phan-loai-windows.md`.

## 3. Kế hoạch (theo VD-162(2)(3) + VD-178 QĐ-5)
- **Nhóm a (48)**: thay `/bin/*` + `bash`/`sh` bằng **helper repo-built** (một binary test), dùng **cùng kỳ vọng** cả 3 nền.
  - Helper **chỉ cài các lệnh con mà test thật sự dùng**: `echo`, `cat`, `grep`, `printf`, `true`, `false`, `sleep`.
  - **Không viết lại `sh -c`**; test đang dùng `sh -c` được **viết lại bằng các lệnh con của helper**.
  > **đã sửa**: `agent/JOURNAL/vs3-qd-5.md:@@L1@@` (ST-178) — bỏ `sh -c` khỏi helper, chốt 7 lệnh con.
- **Nhóm b (15)**: **mặc định coi là lỗi sản phẩm cho tới khi chứng minh ngược lại**.
  - `lsp` (5): nghi chuyển đổi URI ↔ đường dẫn Windows (`file:///C:/…`). **Sửa sản phẩm.**
  - `net_server` (3), `net` (1): điều tra; **không** tăng timeout của test trước khi biết nguyên nhân. Riêng kết nối tới **cổng đóng** trên Windows: ViScript phải báo **“bị từ chối”**, **không** được báo “quá thời gian”.
  - `json_ra` (5): ca thiếu `python3` chuyển **nhóm a** (helper); ca **tên tệp đặc biệt** Windows cấm chuyển **nhóm c**; còn lại điều tra.
  - `host_conformance` (1): TAP — điều tra.
  - Chuyển một ca **b → c** cần **Agent 0 duyệt**.
  > **đã sửa**: `agent/JOURNAL/vs3-qd-5.md:@@L2@@` (ST-178) — chốt hướng nhóm b (trước: “điều tra từng ca”).
- **Nhóm c (42)**: **VS3-5b chạy SAU R6** (R6 sửa thẳng nguyên nhân `VS0102` khi test chèn đường dẫn Windows có `\` vào mã `.vis`). Luật test mới:
  - Test **không** chèn đường dẫn vào chuỗi thường; dùng `p"…"`, hoặc truyền qua **argv/biến môi trường**.
  - Golden/TAP so đường dẫn **tương đối** với thư mục test.
  - **Mục tiêu: danh sách kỳ vọng theo nền về 0.** Ca nào còn lại mới vào danh sách, **Agent 0 duyệt từng dòng**.
  > **đã sửa**: `agent/JOURNAL/vs3-qd-5.md:@@L3@@` (ST-178) — đổi thứ tự 5b (sau R6) + luật test mới.
- **CI**: runner Windows cho `gate --fast` + `vis-host-suite`; log ngoài repo (đã có `vis-remote`).
- **Xen kẽ pha 1**: VS3-5a và VS3-5d được chạy xen kẽ trong pha 1 (không đụng ngôn ngữ).

## 3b. Bảng tóm tắt theo suite (đầy đủ 105 dòng: `agent/JOURNAL/st169-bang-phan-loai-windows.md`)

| Suite | a | b | c | Tổng |
|---|---|---|---|---|
| `cap_proc` | 4 | 0 | 0 | 4 |
| `cap_proc_grant` | 6 | 0 | 0 | 6 |
| `chan_doan_da_tep` | 0 | 0 | 5 | 5 |
| `diag_cmp` | 0 | 0 | 1 | 1 |
| `extern` | 10 | 0 | 0 | 10 |
| `grant_proc_canon` | 5 | 0 | 0 | 5 |
| `host_conformance` | 0 | **1** | 0 | **1** |
| `json_ra` | 0 | 5 | 1 | 6 |
| `ls_open` | 0 | 0 | 1 | 1 |
| `lsp` | 0 | 5 | 1 | 6 |
| `lsp_da_tep` | 0 | 0 | 4 | 4 |
| `net` | 0 | 1 | 0 | 1 |
| `net_server` | 0 | 3 | 0 | 3 |
| `ngoai_vm` | 3 | 0 | 0 | 3 |
| `pipeline` | 14 | 0 | 0 | 14 |
| `quyen` | 0 | 0 | 8 | 8 |
| `sem` | 0 | 0 | 1 | 1 |
| `siet_caps_pkg` | 0 | 0 | 1 | 1 |
| `soak_script` | 1 | 0 | 0 | 1 |
| `span_tep` | 0 | 0 | 1 | 1 |
| `st83_no_nho` | 0 | 0 | 2 | 2 |
| `std_fs_json` | 0 | 0 | 7 | 7 |
| `strict_caps` | 0 | 0 | 2 | 2 |
| `sync_list` | 4 | 0 | 0 | 4 |
| `vis_test` | 0 | 0 | 3 | 3 |
| `vish_cmd_body` | 0 | 0 | 2 | 2 |
| `vish_main` | 1 | 0 | 0 | 1 |
| `vm_main_argv` | 0 | 0 | 1 | 1 |
| `vsh1b` | 0 | 0 | 1 | 1 |
| **Tổng** | **48** | **15** | **42** | **105** |

> **đã sửa**: `agent/JOURNAL/vs3-qd-5.md:@@L4@@` (ST-178) — thêm hàng `host_conformance` (b=1); tổng **48/15/42 = 105** (trước: 48/14/42 = 104, thiếu ca `host_conformance_suite`).

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-5a | Helper repo-built (`vis-posix` test binary) thay `/bin/*` — **chỉ** `echo/cat/grep/printf/true/false/sleep` | 48 ca (a) xanh trên cả 3 nền; không sửa kỳ vọng; không `sh -c` | — |
| VS3-5b | Sửa 42 ca (c) dùng `p"…"`/argv + golden tương đối — **sau R6** | số ca (c) còn lại ≤ danh sách duyệt (mục tiêu **0**) | 5a, **R6** |
| VS3-5c | Điều tra 15 ca (b), phân loại lại | mỗi ca kết luận + sửa hoặc chuyển (c) (A0 duyệt) | 5a |
| VS3-5d | CI runner Windows (psvn) chạy cổng + host | log tự động, `git pull` không chặn | 5a |
| VS3-5e | Tài liệu danh sách kỳ vọng theo nền + tuyên bố mức hỗ trợ | Agent 0 duyệt danh sách; R báo số | 5b, 5c |

## 5. Ca đỏ-trước
`vis-host-suite` trên `psvn` (39/54 → 54/54); `cargo test` 105 → 0 (hoặc = danh sách duyệt).

## 6. Cổng
Trên `psvn`: `gate --fast` 10/10, host 54/54, `cargo test` 0 đỏ; trên mac/ps: L0 **không đổi**.

## 7. Rủi ro
(1) Không có CI ⇒ kết quả phụ thuộc SSH; (2) “native” có thể kéo theo sửa lớn về tiến trình/tín hiệu; (3) danh sách kỳ vọng theo nền dễ bị lạm dụng ⇒ Agent 0 duyệt từng ca; (4) 5b phụ thuộc R6 ⇒ nếu R6 trượt, 5b trượt theo.

## 8. Bảng [SỬA] VD-178 đã áp (ST-178)
| [SỬA] VD-178 | Nội dung | đã sửa |
|---|---|---|
| QĐ-5 mục 1 | Số liệu **48/15/42 = 105** (thêm `host_conformance_suite`) | `agent/JOURNAL/vs3-qd-5.md:@@L4@@` |
| QĐ-5 mục 2 | VS3-5b **sau R6**; luật test mới (`p"…"`/argv, golden tương đối, mục tiêu 0) | `agent/JOURNAL/vs3-qd-5.md:@@L3@@` |
| QĐ-5 mục 3 | Helper **chỉ** 7 lệnh con; **không** `sh -c` | `agent/JOURNAL/vs3-qd-5.md:@@L1@@` |
| QĐ-5 mục 4 | Nhóm b **mặc định lỗi sản phẩm**; lsp 5, net_server 3, net 1, json_ra 5, host_conformance 1 | `agent/JOURNAL/vs3-qd-5.md:@@L2@@` |
| QĐ-5 mục 5 | VS3-5a/5d xen kẽ pha 1 | `agent/JOURNAL/vs3-qd-5.md:@@L5@@` |
