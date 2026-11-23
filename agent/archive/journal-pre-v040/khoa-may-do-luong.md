<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Khoá máy cho phép đo thời gian — giao thức dùng chung hai dòng (VD-177 §3)
**Mục đích**: tránh đo thời gian khi dòng còn lại đang build/soak/QEMU trên cùng máy.

## Cách dùng
```sh
flock "$HOME/.vifast-measure.lock" -c '<lệnh đo>'
```
- **Mọi** phép đo thời gian/hiệu năng (soak, gate có tính giờ, ngưỡng thời gian) **phải** chạy trong `flock` trên.
- Dòng còn lại: **không build, không chạy QEMU** khi khoá đang bị giữ; chờ hoặc làm việc không tốn CPU.
- Log đo **phải** ghi: `load` trước/sau (`sysctl -n vm.loadavg` trên mac; `cat /proc/loadavg` trên Linux), **có giữ khoá hay không**.
- Phép đo **không** giữ khoá: chỉ ghi là **“cận trên”**, **không** dùng làm số chuẩn.

## Trạng thái
- Chưa tự động hoá trong `vis-remote` (sẽ làm trong ST-175/ST-176); hiện **thủ công** theo giao thức này.
- Ví dụ log hợp lệ: `# lock=yes load_truoc="3.1 2.8 3.3" load_sau="0.9 1.4 2.0"`.
