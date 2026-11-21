# VS3 W1 — fuzz parser 4 giờ trên `d1f8f70` (2026-11-21/22)

- **Baseline code**: `d1f8f70` (merge `vs3/w1-runtime` sau vòng fuzz 30m `e9b9316`); nhánh
  `vs3/w1-runtime`. Sau khi fuzz xong mới `git merge main` (`0c911b3`) → merge commit `8f9dfc5`.
  **Không** push/merge lên main.
- **Binary**: `target/vs3-w1/release/vis-parse-fuzz`, build `19:10` từ code `e9b9316`;
  `d1f8f70` chỉ thêm journal so với `e9b9316` nên binary đúng với code `d1f8f70`
  (861.976 byte, 12 core).
- **Mục tiêu**: hardening parser dài 4 giờ liên tục, seed mới; không sửa sản phẩm,
  không GitHub Actions.

## Lệnh

```bash
nohup /home/ps/ViScript/target/vs3-w1/release/vis-parse-fuzz \
  --seconds 14400 --seed 20261121 \
  > /tmp/opencode/w1_fuzz_4h.log 2>&1 &
```

PID `1594885`, chạy nền; chờ bằng `while kill -0 1594885; do sleep 20; done`.

## Kết quả

```text
vis-parse-fuzz: 1385002208 ca, 31434142606 token, 96181 ca/s, 2182926 token/s, 14400.0 s, vi phạm 0
vis-parse-fuzz: PASS (không vi phạm)
fuzz_exit=0
```

| Chỉ số | Giá trị |
|---|---:|
| Thời lượng | 14.400 s (4 h 00 m 00 s) |
| Ca | **1.385.002.208** |
| Token | 31.434.142.606 |
| Tốc độ | 96.181 ca/s; 2.182.926 token/s |
| Vi phạm | **0** (panic/CST lossless/span/chẩn đoán đều sạch) |
| Exit | **0** (`PASS`) |
| Bắt đầu | 2026-11-21T19:42:22+09:00 |
| Kết thúc | 2026-11-21T23:42:22+09:00 |
| Seed | `20261121` |

- Không có seed lỗi cần lưu; không có ca FAIL, không cần giảm corpus.
- Tốc độ thấp hơn vòng 30m (`144.559 ca/s`) ~1,5× do tải máy trong khung 19:42–23:42;
  số ca/giây không ảnh hưởng kết luận (0 vi phạm trên 1,385 tỷ ca).
- Tổng luỹ kế 2 vòng hôm nay: 260.206.874 + 1.385.002.208 = **1.645.209.082 ca, 0 vi phạm**.

## Đĩa / tài nguyên

```text
/dev/nvme0n1p2  233G  209G   17G  93% /
```

- `df -h /home` sau fuzz: **17 G trống (93 %)** — trên ngưỡng 5 G, không cần dọn.
- Chỉ dùng target `vs3-w1`; không tạo target thứ hai.

## Kết luận

- Parser sạch qua 4 giờ liên tục, seed `20261121`, 1,385 tỷ ca: **PASS, 0 vi phạm**.
- Không phát hiện lỗi parser; không mở blocker mới; không sửa production.
- Log gốc: `/tmp/opencode/w1_fuzz_4h.log`.
