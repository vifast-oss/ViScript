<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Windows h1 cancel — ma trận đo flake trên `a1930cb`

> **Người đo**: subagent VS3-W4, 2026-11-22 · **time-box 1h** · **chỉ đo, không sửa code**.
> **Mốc**: `psvn` `/c/Users/ps/viscript-moi` checkout **`a1930cbe84231c2362626dca9bd919a906fab0b9`**
> (`merge ST-289 QA: final v0.3.6 4c5dec6`), **dirty 0 lúc bắt đầu**; Windows `MINGW64_NT-10.0-26300`,
> 6 CPU, cargo/rustc 1.100.0. Test binary: `target/debug/build/vis-cli/5d8a72d6e6759a34/out/
> vs3_http_cancel-5d8a72d6e6759a34.exe`.
> **Kết luận**: flake **xác nhận**, phụ thuộc **cách chạy** — ca riêng 20/20; full suite default
> **8/10 pass**; full suite `--test-threads=1` **2/10 pass**; 2 tiến trình song song **10/10**.
> Không tag/push/merge; không GitHub Actions.

## 0. TL;DR

| Pha | Cách chạy | Kết quả | Tỉ lệ fail |
|---|---|---|---:|
| **A** | ca `cancel_http_h1_lap_nhieu_vong_khong_ro` × **20** (`--test-threads=1`) | **20/20 pass** | **0 %** |
| **B** | full `vs3_http_cancel` (5 ca) × **10**, mặc định | **8/10 pass** | **20 %** |
| **C** | 2 tiến trình full suite song song × **5 vòng** (10 process-run) | **10/10 pass** | **0 %** |
| **D** | full suite × **10**, `--test-threads=1` | **2/10 pass** | **80 %** |
| **E** | từng ca riêng × **10** (5 ca) | **46/50 pass** | **8 %** |

- **Ca đỏ không cố định**: B2/B8 = `mot_task_khong_anh_huong_task_khac`; D1/D4/D7/D9 =
  `dong_socket_va_parity`; D5/D6/D8/D10 = `mot_task...`; (phiên trước) `lap_nhieu_vong`,
  `server_khong_tra_header`.
- **Ca riêng (E)**: `server_khong_tra_header` **3/10 fail**, `dong_socket_va_parity` **1/10**,
  ba ca còn lại 0/10 ⇒ flake **nội tại từng ca**, không chỉ do chạy chung.
- **Hai tiến trình song song không làm tăng fail** (C 10/10) ⇒ không phải tranh chấp tài nguyên
  liên tiến trình/port toàn cục; nặng hơn khi **tuần tự trong cùng process** (D 80 %).

## 1. Lệnh / phạm vi

- A: `cargo test -p vis-cli --test vs3_http_cancel cancel_http_h1_lap_nhieu_vong_khong_ro -- --test-threads=1` ×20.
- B: `cargo test -p vis-cli --test vs3_http_cancel` ×10.
- C: chạy **trực tiếp** binary test (đã build) 2 tiến trình/lần ×5 vòng, không qua cargo (tránh lock).
- D: `cargo test -p vis-cli --test vs3_http_cancel -- --test-threads=1` ×10.
- E: mỗi ca `cargo test --test vs3_http_cancel <tên ca>` ×10.
- Không sửa mã; không đổi feature; không chạy `--features async-h2` (Unix-only trên Windows).

## 2. Kết quả chi tiết

### A — ca riêng ×20
`A1..A20 rc=0`, mỗi lần `1 passed / 0 failed` (~0,88 s). **Không tái hiện** khi chỉ chạy ca này.

### B — full suite default ×10
| Lần | Kết quả | Ca đỏ |
|---|---|---|
| B1, B3–B7, B9, B10 | 5/5 pass | — |
| **B2** | **4/5** | `mot_task_khong_anh_huong_task_khac` — `task còn lại phải xong: "V:pre\nloi: ... 10053\n"` |
| **B8** | **4/5** | `mot_task...` — `task bị huỷ phải dừng: "V:pre\nloi: ... 10053\nV:post\nok\n"` |

### C — 2 tiến trình song song ×5 vòng
`C1..C5`: cả `p1` và `p2` đều `rc=0`, `5 passed / 0 failed` (0,92–1,00 s/process) ⇒ **10/10**.

### D — full suite `--test-threads=1` ×10
| Lần | Kết quả | Ca đỏ |
|---|---|---|
| D2, D3 | 5/5 pass | — |
| **D1, D4, D7, D9** | **4/5** | `dong_socket_va_parity` — V:post sau `10053` |
| **D5, D10** | **4/5** | `mot_task...` — V:post sau `10053` (D5 kèm `ok`) |
| **D6** | **4/5** | `mot_task...` — V:post, rồi `C2` cũng `10053` |
| **D8** | **4/5** | `mot_task...` — `task còn lại phải xong`, `C2` `10053` |

### E — từng ca riêng ×10
| Ca | Fail |
|---|---:|
| `cancel_http_h1_dong_socket_va_parity` | **1/10** |
| `cancel_http_h1_lap_nhieu_vong_khong_ro` | 0/10 |
| `cancel_http_h1_mot_task_khong_anh_huong_task_khac` | 0/10 |
| `cancel_http_h1_sau_huy_van_goi_duoc` | 0/10 |
| `cancel_http_h1_server_khong_tra_header` | **3/10** |

## 3. Mẫu lỗi (pattern)

1. **Dạng chính — huỷ không dừng task**: stdout có
   `V:pre` → `loi: lỗi đọc C1/C2: ... os error 10053` → **`V:post`** (đôi khi kèm `ok`).
   Task đã bị huỷ vẫn chạy tiếp nhánh lỗi socket Windows thay vì dừng ở safe point.
2. **Dạng phụ — task anh em bị đóng theo**: ca 2-task báo `task còn lại phải xong` với
   `C2 ... 10053` / `phản hồi HTTP không đủ header (kết nối đóng sớm)` — dấu hiệu đường huỷ/đóng
   socket ảnh hưởng connection của task không bị huỷ (B2, D8; D6 có cả hai).
3. **Phụ thuộc lịch chạy**: cùng binary, cùng ca — chạy riêng 20/20 (A/C), chạy cả suite tuần tự
   80 % đỏ (D), chạy song song default 20 % (B). Nhiều khả năng do **turn-counter/cancel trigger
   (test-only) so với đường lỗi `WSAECONNABORTED 10053`**: khi lỗi socket trả về, runtime resume
   task trước khi terminal cancel được áp; tuần tự làm biên timing lệch rõ hơn.
4. **Không phải do tải máy**: C (2 tiến trình song song) 0/10; D (tuần tự, nhẹ tải) 8/10.
5. **Không phải chỉ test harness**: ca riêng `server_khong_tra_header` đỏ 3/10 và
   `dong_socket_va_parity` 1/10 khi chạy một mình ⇒ hành vi runtime là một phần nguyên nhân.

## 4. Ghi chú môi trường

- Bắt đầu: HEAD `a1930cb`, `git status --porcelain` **rỗng** (07:22:46Z).
- **07:26:12Z** — tệp `crates/vis-cli/tests/vs3_http_cancel.rs` bị **một tác nhân khác** sửa
  (chỉ **thông điệp assert**: thêm `stdout/log`; nội dung kiểm không đổi). Pha A–D và phần lớn E
  chạy trước thời điểm này; E có thể chạy đè vài giây cuối với assert message mới — **không đổi
  pass/fail**. Tôi **không** revert/không đụng thay đổi đó; cây `psvn` hiện **bẩn** vì thay đổi
  của tác nhân kia, không phải của phiên này.
- Log thô: `agent/JOURNAL/win-h1-cancel-matrix.log` (nguồn
  `/c/Users/ps/vislogs/matrix-summary.log`, sha256 `89d31df3…2e4c7d`).
- Log lỗi tiêu biểu trên `psvn`: `matrix-b2.log` (`7c2f5c48…ea96b`), `matrix-b8.log`
  (`d8bbf141…5e5bf2`), `matrix-d1.log` (`e58181bd…b1f67e`), `matrix-d8.log` (`46f7fe42…432344`);
  E: `matrix-e-cancel_http_h1_server_khong_tra_header-<i>.log`.
- df: `psvn` `C:` còn **172 G**; local `/` còn **9,2 G** (>5 G).

## 5. Kết luận / khuyến nghị

- Blocker h1 cancel trên Windows **tái hiện được** ở commit `a1930cb`; **không** phải flake đơn lẻ
  của một ca: 3/5 ca từng đỏ ít nhất một lần; phụ thuộc mạnh vào cách chạy (tuần tự làm tăng vọt).
- **Chưa nên tag `v0.3.6`** nếu muốn tuyên bố Windows xanh. Nếu chấp nhận waiver Tier 2: ghi
  known-issue này (kèm log) và **không** tuyên bố Windows xanh.
- Hướng điều tra tiếp (không làm trong phiên này): kiểm tra thứ tự **áp terminal cancel** so với
  **resume từ lỗi socket** trong `frame_cancel_one`/`AsyncNet` trên Windows (10053), và vì sao
  đóng socket của task bị huỷ có thể làm hỏng connection của task anh em; sau đó lặp A/B/D.
