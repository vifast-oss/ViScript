# SPDX-License-Identifier: MIT OR Apache-2.0
# CHƯA DÙNG trong VS2: runner Linux là máy ps qua SSH (VD-136, VD-138). Tệp chưa build, chưa có digest.
# Nền Linux của dòng ViScript (ST-121, VD-120) — backend `docker` của `bin/vis-linux`.
#
# Mục đích: có một nền Linux **chạy được** để đo các hạng mục VS2 còn thiếu (Landlock thật,
# hợp chuẩn host nền thứ hai). Image chỉ chứa toolchain; cây nguồn mount **chỉ đọc** ở
# `/src`, thư mục build là volume riêng `/target` (không ghi vào repo của người dùng).
#
# Ghim phiên bản theo `rust-version` của workspace (`Cargo.toml` gốc: 1.99.1).
# Ghi digest thật đã kéo vào log của lần build đầu (ST-121 yêu cầu).
FROM rust:1.99.1-bookworm

ENV CARGO_TARGET_DIR=/target \
    CARGO_TERM_COLOR=never \
    RUST_BACKTRACE=1

# Công cụ chỉ để ĐO, không phải phụ thuộc của sản phẩm:
#   python3  — harness báo cáo + các ca kiểm viết bằng Python
#   rsync    — không cần trong container (đồng bộ do `bin/vis-linux` làm ở phía host)
#   unzip/xz — không cần
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
CMD ["bash", "viscript/bin/vis-dev", "gate", "--fast"]
