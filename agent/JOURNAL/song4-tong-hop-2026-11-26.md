<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Sóng 4 — Báo cáo tổng hợp bàn giao Agent 0 (2026-11-26)

> **Snapshot**: `main` sau `1e803f93` (+ commit tài liệu này) · **Máy**: `ps` (Linux) + `psvn`
> (Windows) · **Không dùng `mps`** (đang bận dự án khác) · **Chế độ**: `VD-289` Phát triển Kiến trúc.
> **Kết luận Gatekeeper**: cả 4 lát đạt cổng máy; **đề nghị A0 duyệt `DONE`** `ST-315a..ST-315d`
> (hoặc quyết định riêng cho caveat F1 bên dưới).

## 1. Trạng thái

| Lát | Trạng thái | Commit chính | Kết quả cốt lõi |
|---|---|---|---|
| `ST-315a` | `REVIEW` | `50bb7885` | `budget_charge` O(1) bằng chỉ mục `pc→(start,len)`; bỏ TLS mỗi sự kiện, flush `budget_used` một lần. Release `opt-bench` **VM 344 ms** (trước **610 ms**, −44 %), interp 329 ms; KPI VM ≤ interp+5 % **PASS**; review R-A **PASS** (engine-only +2,7 %) |
| `ST-315b` | `REVIEW` | `7dc21cf2` / merge `8ca08441` | `Value::Map(Rc<VisMap>)` CoW + chỉ mục băm lười (ngưỡng 8), Q1–Q3 giữ; `size_of::<Value>() == 64`; `opt-bench` **45/42 ms**, `mapbench N=316` **48/50 ms** (~27×), N=1 không hồi quy; review R-B **PASS** |
| `ST-315c` | `REVIEW` | `fbcbc139` + `801719e9` | Đóng **`#197`** (`Truncate` cho tệp đơn `(write)` + ca #19); W3 M2 ống trần 8 MiB/kill `VS0501` + RAII dọn driver; `vis-soak` log ngoài repo; review R-C **PASS** + re-audit **PASS** |
| `ST-315d` | `REVIEW` | `c2b051c9..0ba3e8f2`, `d2b7eda8`, `a45438e6`, `25790c97`, `7014fb7b` | Mã **`VS0304`** vi/en/ja + Tarjan **lặp** (100k cạnh, 25k e2e không abort); SCC lớn 1 chẩn đoán có trần; gợi ý `VS0204`; parity **66/66** (12 test); review R-D **PASS** + re-audit **PASS**, R-Parity **PASS**, R-Docs2 **PASS** |

## 2. Cổng máy (bằng chứng cuối)

| Cổng | Kết quả |
|---|---|
| `./vis-dev gate --fast` (Linux, target ngoài repo) | **PASS** — 3.856 test/66 binary, 0 fail, 4 skip + doc-test |
| `./vis-dev check` | **PASS** — fmt/clippy/R1..R9/DAG/doc-examples 31/readme 20/20 |
| `cargo clippy --workspace --all-targets -- -D warnings` | **PASS** |
| `VIS_DIFF_N=100000 ./bin/vis-diff` | **lech=0** (nhiều lượt, gồm sau merge W2) |
| `stdlib_diff` (`vis-vm --test vm_it`) | **0 lệch** (73 hàm/400 chương trình) |
| **Windows full `psvn`** | `ST-315b` **3.777/0/4 @`c6724b46`**; `ST-315d` parity **3.779/0/4 @`7014fb7b`** (host_parity 11/11, 64/66 do symlink `cfg(unix)`), targets core 47/0 + sqlite 46+1 expected-red, clippy PASS |
| R8/R1/R2/R7 | R8 **39 ≤ 40**, 0 `unsafe`, log ngoài repo, worktree đã dọn còn **1** |

## 3. Review độc lập (đã merge báo cáo)

- **R-A `ST-315a` PASS**: differential 25 bảng phí khớp 100 %, 11 mốc trap/`VS0510`, `MAX_PER_PC`,
  stale index, frame/async; F2 đã thêm test.
- **R-B `ST-315b` PASS**: fuzz `VisMap`↔`Vec` 0 lệch; F3 đã vá (index sau clone: 25,9 µs → 0,10 µs/get),
  F5 đã siết (`==64` + 64 khoá phân biệt); F1/F2/F4/F6 ghi nhận thấp/info.
- **R-C `ST-315c` PASS + re-audit PASS**: repro panic UTF-8 không còn; RAII/kill/8 MiB đúng.
- **R-D `ST-315d` PASS + re-audit PASS**: Tarjan lặp; SCC 5k chỉ 1 chẩn đoán 265 B/1,65 s/RSS 30 MB.
- **R-Parity PASS Linux + Windows**: 12 test, 17 ca lỗi `fs`, message `net`, `cwd-khop` Windows;
  F-R1 (mutation một lời gọi trùng) là **caveat chấp nhận** — xoá toàn bộ lời gọi mới đỏ, mất hành
  vi bị test `fs_20` bắt.
- **R-Docs2 PASS**: QUEUE/STATE/SO-NO/API-CODES/commit hygiene khớp; Windows pending đã chốt sau đó.

## 4. Quyết định cần A0

1. **Duyệt `DONE`** cho `ST-315a..ST-315d` hay yêu cầu chỉnh thêm (không còn blocker kỹ thuật).
2. **F1 `ST-315b` — header HTTP trùng tên**: `Value::map` gộp last-wins nên `http.head`/`async_http`/
   `fs_net::dispatch` **mất `Set-Cookie`/`Warning`/`Link` trùng** (trước giữ đủ cặp, `get` trả first).
   Cần chọn: (a) ghi CHANGELOG/wontfix (map là ánh xạ, không biểu diễn multi-value), hay
   (b) đổi kiểu header sang list/`map[str, list[str]]` (breaking API — nên là lát riêng).
3. **macOS `mps`**: chưa đo `ST-315b`/parity/3-OS trên macOS; hoãn sang `ST-316c` khi máy rảnh.
   Windows symlink 64/66 là chủ đích (`#[cfg(unix)]`), không phải thiếu sót.
4. **Caveat thấp**: `budget_idx` 8 B/lệnh; `MapIndex` cấp cho khoá phức tạp; JSON đệm cặp trùng;
   +48 B/map rỗng — không chặn, có thể tối ưu ở Sóng 5/6.

## 5. Việc kế tiếp

- A0 duyệt xong ⇒ mở **Sóng 5**: `ST-316a` (`Value::List` CoW + `bin/vis-bench`), `ST-316b`
  (nợ `#186/#7/#48/#145`), `ST-316c` (macOS `mps` + Go/No-Go `ST-309`).
- Trong cao điểm 2 (06:00–10:00 UTC): chỉ chạy 0-LLM (soak/fuzz) theo `VD-287`; không mở feature mới.
