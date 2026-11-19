<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Nhật ký các đợt (CHANGELOG)

Tóm tắt theo từng đợt phát triển. Chi tiết đầy đủ ở [`agent/reports/`](agent/reports/) (mỗi đợt một cặp `.json` + `.md`).

| Đợt | HEAD (mac) | Cổng `--fast` | Số task trong báo cáo |
|---|---|---|---|
| R1 | `b4d7d469` | PASS | 11 |
| R2 | `43107a40` | PASS | 20 |
| R3 | `d971db57` | PASS | 27 |
| R4 | `0463512b` | PASS | 36 |
| R5 | `10353111` | PASS | 49 |
| R6 | `e0f6c234` | PASS | 49 |
| R7 | `29c13395` | PASS | 53 |
| R8 | `bb8393fe` | PASS | 58 |
| R9 | `d3b11346` | PASS | 61 |
| R10 | `8e57eef2` | PASS | 85 |
| R11 | `2f490b41` | PASS | 86 |
| R12 | `8b234a1f` | PASS | 92 |
| R13 | `c1228f1c` | PASS | 100 |
| R14 | `5f068f77` | PASS | 104 |
| R15 | `d012624a` | PASS | 108 |
| R16 | `005b12a8` | PASS | 112 |
| R17 | `e446481c` | PASS | 117 |
| R18 | `637bbebb` | PASS | 121 |
| R19 | `4bb3d0c7` | PASS | 125 |
| R20 | `536f802d` | PASS | 127 |
| R21 | `fcf3ae63` | PASS | 130 |
| R22 | `87fd4f9d` | PASS | 134 |
| R23 | `37d57145` | PASS | 153 |
| R24 | `5e364155` | PASS | 156 |
| R25 | `43b19dfe` | PASS | 164 |
| R26 | `b1c07802` | PASS | 171 |
| R27 | `dd088322` | PASS | 174 |
| R28 | `fdb21d15` | PASS | 177 |
| R29 | `39041b3f` | PASS | 197 |
| R30 | `0d5e0e57` | PASS | 223 (222 DONE) |

## Mốc đáng chú ý

- **R29** — số 3 nền cùng HEAD: mac **1184/0**, ps **1188/0**, psvn **152/4** (4 ca `cap_proc` do thiếu `/bin/echo`, `/bin/sh` trên Windows).
- **R30** — pha 1 công thái học (C3 VD-178), tách repo; trạng thái theo `agent/reports/R30-viscript.{md,json}`.
- **2026-11-20** — VD-winport: Linux xanh toàn workspace @ `c7cef71` (gate ×3 PASS; nightly PASS @ `f9430b9`); **Windows**: @ `6ccf367` **7 suite đỏ** → @ `f9430b9` **1 ca đỏ** → @ `c7cef71` & **@ `2d93101` exit 0, toàn workspace XANH** (150 khối ok, 0 đỏ) — VD-winport hoàn tất fix A–E; `pha1-rc1` nguyên; Tier 2 giữ.
- **ST-174** — đóng nợ **#146** (TOCTOU macOS): vá `symlink_metadata` + chứng minh tất định 100/100, 6 triệu vòng 0 đỏ.
- **ST-171** — tách repo ViScript khỏi ViFastOs: 459 commit giữ lịch sử, gate mac/ps khớp R29.
- **ST-176** — nợ **#147**: mac 1489 s vs ps 261 s = 5,70×; nguyên nhân tạo tiến trình/syscall macOS (chưa đóng).
- **ST-173** — Landlock chỉ Linux: mac hết lỗi biên dịch, ps 15/15 không đổi.
