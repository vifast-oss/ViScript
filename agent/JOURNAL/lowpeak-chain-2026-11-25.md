<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Chuỗi kiểm tra 0-LLM low-peak — 2026-11-25 11:18 UTC → 2026-11-26 00:55 UTC

> **Mục đích**: theo `VD-287`, giữ máy chạy kiểm chứng **không dùng LLM** suốt giờ thấp điểm và
> **dừng trước giờ cao điểm** 01:00 UTC 2026-11-26. Không soak/fuzz dài (chế độ `VD-289`).

- **Script**: `/home/ps/vislogs/lowpeak-chain-2026-11-25/chain.sh` (ngoài repo, R7)
- **PID**: `/home/ps/vislogs/lowpeak-chain-2026-11-25/pid`
- **Nhịp**: mỗi vòng (~10 phút) `vis-diff 100k` seed tăng; mỗi 4 vòng `vis-dev gate --fast`;
  mỗi 8 vòng `cargo clippy --workspace --all-targets -D warnings`; mỗi 6 vòng WASI suite/sem/
  conformance; mỗi 12 vòng `vis-targets --wasm` + `vis-host-suite` + `vis-dag-check`.
- **Vệ sinh**: mỗi vòng dọn `/tmp/vis-*`/`/tmp/vs3-*` cũ >30 phút của user; nếu đĩa gốc < 9 GB thì
  xoá `target/debug/incremental` (tái sinh được) — bài học `lp38` đỏ vì tmpfs đầy.
- **Bằng chứng**: `chain.log` (dòng START/END + rc), `heartbeat.txt` (vòng hiện tại),
  `disk.log`, `REPORT.md` + `SHA256SUMS` khi kết thúc.
- **Kiểm tra nhanh**: `cat /home/ps/vislogs/lowpeak-chain-2026-11-25/heartbeat.txt`.
- **Kết quả cuối**: cập nhật vào mục này sau 00:55 UTC (số vòng, số lỗi, sha log).
