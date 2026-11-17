<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Công cụ trong `bin/`

| Tệp | Việc | Ai dùng |
|---|---|---|
| `vis` | chạy chương trình `.vis` (shim gọi `crates/vis-cli`) | mọi người |
| `vis-dev` | cổng phát triển: `check`, `gate --fast`, `nightly`, `doc-examples`, `rules` | người đóng góp |
| `vis-diff` | so sánh kết quả hai lần chạy (differential) | kiểm thử |
| `vis-soak` | chạy lặp nhiều vòng để tìm lỗi chập chờn | kiểm thử |
| `vis-lock` | giữ khoá `~/.vifast-measure.lock` khi đo thời gian (macOS không có `flock`) | đo hiệu năng |
| `vis-remote` | chạy lệnh trên máy `ps`/`psvn` qua ssh, log ngoài repo, đọc mã thoát từ tệp | đo đa nền |
| `vis-linux` | chạy cổng trong container Linux | đo đa nền |
| `vis-targets` | kiểm crate `vis-host-*` theo nền | kiểm thử |
| `vis-host-suite` | bộ kiểm host (Linux/Windows) | kiểm thử |

Quy ước: mọi script có header SPDX `#`, log mang mã task (`ST-##`), không ghi đè log cũ.
