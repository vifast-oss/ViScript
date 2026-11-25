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
- **Đã chạy (cập nhật dần)**: vòng 1 `11:18–11:19` seed `202611260001`: **100.000, lech=0, 72,3 s**
  (`r001-diff-202611260001.log`); vòng 2 `11:29–11:31` seed `202611260002`: **100.000, lech=0, 68,5 s**
  (`r002-diff-202611260002.log`); vòng 3 `11:41–11:42` seed `202611260003`: 100.000, lech=0
  (`r003-...`); vòng 4 `11:52–11:54` seed `202611260004`: 100.000, lech=0 + **gate rc=0** (`r004-...`);
  vòng 5 `12:04–12:05` seed `202611260005`: 100.000, lech=0; vòng 6 `12:15–12:18` seed `202611260006`:
  100.000, lech=0 + **WASI suite 28/28, sem 390/390, conformance 379×2, cả 3 exit 0** (`r006-*.log`);
  vòng 7–11 (12:28–13:14): mỗi vòng 100.000, **lech=0**; vòng 8 thêm **gate rc=0 + clippy rc=0**;
  vòng 12 (13:24–13:28): 100.000 lech=0, **gate rc=0**, **`vis-targets --wasm` rc=0** (đo lại sem
  390/390, suite 28/28, conformance 379×2), **host-suite 54/54**, **DAG OK** — tất cả exit 0;
  vòng 13–16 (13:37–14:12): mỗi vòng 100.000 **lech=0**; vòng 16 thêm **gate + clippy rc=0**;
  vòng 17–18 (14:23–14:36): 100.000 lech=0; vòng 18 **WASI suite 28/28 + sem 390/390 +
  conformance 379×2, cả 3 exit 0** (`r018-wasm-*.log`);
  vòng 19–24 (14:46–15:45): **600.000 chương trình, cả 6 vòng lech=0**; vòng 20 gate rc=0;
  vòng 24 **gate + clippy + `vis-targets --wasm` (sem 390/390, suite 28/28, conformance 379×2) +
  host-suite 54/54 + DAG, tất cả rc=0** (`r024-*.log`). Tổng tới đây: **24 vòng = 2,4 M chương
  trình diff, 0 lệch**; 6× gate, 3× clippy, 3× WASI (18, 24, 12 targets), 2× host/DAG.
  Song song ngoài chuỗi: Windows full `psvn` @`92440689` **3.737/0**, release bench, fuzz 30 phút.
