<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-318b — đóng `#202`: ổn định output `host_parity::net_http` (bỏ số ms đo thật)

- **Nhánh**: `plan/st318b`, nền `2b5ccb51` (`main`; `VD-296` đăng ký lát này nằm trên
  `origin/main` `2c7dbef6`, ngoài nền worktree).
- **Worktree**: `/home/ps/viswt/st318b`; máy `ps` (Linux), target riêng của worktree.
- **Commit code**: `68aeb4b3` — `[ST-318b] #202: on dinh output host_parity net_http (bo ms that)`.
- **Phạm vi**: chỉ `crates/vis-conformance/tests/misc_api_it/host_parity/net_http.rs`;
  **0 chạm sản phẩm**, 0 tệp test cấp 1 mới (R8 giữ 39/40), không `#[ignore]`, không bỏ ca.

## 1. Nguyên nhân

`http_bon_ham_parity` chạy cùng script bằng interp rồi `--vm --no-cache` và harness
`hai_duong` so **từng byte** stdout. Hai dòng timeout thật in `e.message`:

```
slow=quá thời gian chờ dữ liệu từ `C9` sau 78 ms
slowhead=quá thời gian chờ dữ liệu từ `C10` sau 79 ms
```

`crates/vis-http/src/lib.rs` tính `con_lai_ms(han)` = **phần deadline còn lại** sau connect rồi
truyền xuống `net.connect_http`/`net.read`; `crates/vis-host-native/src/net.rs` in đúng
`timeout_ms` nhận được (`… sau {timeout_ms} ms`). Vì hai engine chạy tuần tự, phần còn lại lệch
1 ms (78 vs 79) tuỳ thời điểm; test so byte-exact nên đỏ oan dù chức năng HTTP đúng.

Bằng chứng gốc: `/home/ps/vislogs/review-st317c-gate.log` dòng 2095–2096 — `left` có
`slow=… sau 78 ms`, `right` có `slow=… sau 79 ms`; các dòng còn lại giống hệt.

## 2. Tái lập

- Chạy **20 lần** ca cũ (bản chưa sửa) tuần tự trên máy rảnh: **20/20 xanh**
  (`/home/ps/vislogs/st318b-repro.log`, sha256 `5321a62e…64e9c0`).
- Chạy **5 đợt × 10 process song song**: **50/50 xanh**
  (`/home/ps/vislogs/st318b-repro-par.log`, sha256 `2dd3a9ab…c7f9d9`).
- Đo trực tiếp 20 cặp `vis run` ↔ `vis run --vm --no-cache` trên server HTTP Python ngủ 300 ms,
  timeout 80 ms: cả 40 lần in `sau 79 ms`
  (`/home/ps/vislogs/st318b-repro-ms2.log`).
- Kết luận: trên máy rảnh hiện tại xác suất lệch thấp nên không tái hiện được trong 70 lần;
  nhưng log `gate --fast` lịch sử đã bắt đúng cặp `78/79 ms`, và code ở §1 xác nhận số ms là
  **phần deadline còn lại đo thật**, không phải hằng số. Flake có sẵn từ nền, không do lát này.

## 3. Cách sửa (test-only)

Thêm `chuan_hoa_ms`: quét chuỗi, tại mỗi `" ms"` lùi qua **dãy chữ số ASCII** ngay trước đó và
thay bằng `<N>`; mọi byte khác (tên hàm, handle `C9`/`C10`, chữ “sau”, đơn vị `ms`, cổng, body)
giữ nguyên. Ca đích gọi `hai_duong_chuan_hoa(..., chuan_hoa_ms)` thay `hai_duong`.

Assert không nới:
- vẫn đòi đúng **7 dòng**, 4 dòng OK/HEAD/`get_timeout`/`head_timeout` khớp từng byte;
- hai dòng timeout vẫn phải `starts_with("slow=")`/`"slowhead="`, chứa `"quá thời gian"` **và**
  kết thúc bằng `" sau <N> ms"` — chứng minh thông điệp hết giờ còn nguyên cấu trúc;
- cổng đóng vẫn phải `Err`, server vẫn phải thấy đủ **12** request đúng method/đường dẫn của cả
  hai engine.
- Thêm unit test `chuan_hoa_ms_chi_thay_so_do_thuc` khoá bộ chuẩn hoá: thay `78`/`79` → `<N>`,
  không đổi chuỗi không có số trước ` ms`, `123 ms` → `<N> ms`.

## 4. Cổng

| Mục | Kết quả | Log |
|---|---|---|
| Ca đích **20×** liên tiếp (bản cuối) | **20/20 PASS**, 0 flake | `/home/ps/vislogs/st318b-fixed-20-final.log`, sha256 `65a8acef…61157` |
| `host_parity::net_http` (4 ca, gồm unit test mới) | 4/4 PASS | — |
| `./vis-dev gate --fast` | **PASS** (exit 0; `lib_api_it` **399** test) | `/home/ps/vislogs/st318b-gate-fast.log`, sha256 `36c211e1…2a5d3` |
| `./vis-dev rules` | **OK** | — |

Không chạy `vis-diff 100k`: lát chỉ sửa test, không chạm compiler/runtime/VM. Không `unsafe`,
không đổi API, không đổi VBC1.

## 5. Còn lại

- A0 cập nhật `agent/SO-NO.md` `#202` `mở → đóng` và hàng `ST-318b` trong `QUEUE.md` khi nghiệm
  thu (worktree nền `2b5ccb51` chưa có `VD-296`/hàng `ST-318b`, nên lát này không sửa hai tệp đó).
- Không cần chạy lại trên Windows/macOS: chuẩn hoá thuần Rust, không phụ thuộc OS; thông điệp
  timeout vẫn được so byte-exact sau khi bỏ đúng token số.
