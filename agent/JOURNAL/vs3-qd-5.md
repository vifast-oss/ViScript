<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-QĐ-5 · Windows "hỗ trợ thật"
**Trạng thái**: CHỜ AGENT 0 DUYỆT (VD-148) · **Phụ thuộc**: **độc lập** (chạy song song được) · **Mở đường cho**: —

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

## 3. Kế hoạch (theo VD-162(2)(3))
- **Nhóm a (48)**: thay `/bin/*` + `bash`/`sh` bằng **helper repo-built** (một binary test có `echo/cat/grep/printf/true/false/sleep/sh -c`), dùng **cùng kỳ vọng** cả 3 nền.
- **Nhóm b (15)**: điều tra từng ca; sửa sản phẩm nếu là lỗi thật; nếu là khác biệt bản chất ⇒ chuyển sang **c** và ghi lý do.
- **Nhóm c (42)**: **trước hết** sửa test dùng `/`/đường dẫn tương đối (đề nghị ST-169 §4); nếu còn ⇒ đưa vào **danh sách kỳ vọng theo nền** (một tệp duy nhất) để Agent 0 duyệt.
- **CI**: runner Windows cho `gate --fast` + `vis-host-suite`; log ngoài repo (đã có `vis-remote`).


## 3b. Bảng tóm tắt theo suite (đầy đủ 105 dòng: `agent/JOURNAL/st169-bang-phan-loai-windows.md`)

| Suite | a | b | c | Tổng |
|---|---|---|---|---|
| `cap_proc` | 4 | 0 | 0 | 4 |
| `cap_proc_grant` | 6 | 0 | 0 | 6 |
| `chan_doan_da_tep` | 0 | 0 | 5 | 5 |
| `diag_cmp` | 0 | 0 | 1 | 1 |
| `extern` | 10 | 0 | 0 | 10 |
| `grant_proc_canon` | 5 | 0 | 0 | 5 |
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
| **Tổng** | **48** | **14** | **42** | **104** |

## 4. Chia task ≤ 3 h
| Task | Nội dung | Điều kiện xong | Phụ thuộc |
|---|---|---|---|
| VS3-5a | Helper repo-built (`vis-posix` test binary) thay `/bin/*` | 48 ca (a) xanh trên cả 3 nền; không sửa kỳ vọng | — |
| VS3-5b | Sửa 42 ca (c) dùng `/` + golden tương đối | số ca (c) còn lại ≤ danh sách duyệt | 5a |
| VS3-5c | Điều tra 15 ca (b), phân loại lại | mỗi ca kết luận + sửa hoặc chuyển (c) | 5a |
| VS3-5d | CI runner Windows (psvn) chạy cổng + host | log tự động, `git pull` không chặn | 5a |
| VS3-5e | Tài liệu danh sách kỳ vọng theo nền + tuyên bố mức hỗ trợ | Agent 0 duyệt danh sách; R báo số | 5b, 5c |

## 5. Ca đỏ-trước
`vis-host-suite` trên `psvn` (39/54 → 54/54); `cargo test` 105 → 0 (hoặc = danh sách duyệt).

## 6. Cổng
Trên `psvn`: `gate --fast` 10/10, host 54/54, `cargo test` 0 đỏ; trên mac/ps: L0 **không đổi**.

## 7. Rủi ro
(1) Không có CI ⇒ kết quả phụ thuộc SSH; (2) “native” có thể kéo theo sửa lớn về tiến trình/tín hiệu; (3) danh sách kỳ vọng theo nền dễ bị lạm dụng ⇒ Agent 0 duyệt từng ca.
