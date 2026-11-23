<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-296 test/harness — TMPDIR soak ngoài repo + LSP timing chịu tải

> **Người làm**: W3 (Gatekeeper QA), 2026-11-23 · **baseline** merge `main` **`d53d258`**.
> **Phạm vi**: chỉ test/harness: `bin/vis-soak` + test LSP timing; không sửa sản phẩm.
> Không push/merge/tag; không GitHub Actions.

## 1. `bin/vis-soak` — TMPDIR ngoài repo

- Bỏ mặc định `$PWD/target` (nguyên nhân self-test R1/R2/R3 trong soak không bắt được case âm:
  cây tạm nằm trong `target/`, bị `vis-dev` prune).
- Thứ tự chọn gốc TMP mới:
  1. `VIS_SOAK_TMPDIR` (nếu có);
  2. `TMPDIR` sẵn có (nếu có);
  3. `${XDG_CACHE_HOME:-$HOME/.cache}` — mặc định **ngoài repo**.
- Luôn tạo thư mục con `vis-soak-tmp-<pid>` và `trap` dọn (`EXIT INT TERM HUP`);
  `--dry-run` vẫn trả về **trước** khối tạo thư mục ⇒ không tạo cache/tmp.
- Kiểm:
  - `sh -n bin/vis-soak` → OK;
  - `XDG_CACHE_HOME=... ./bin/vis-soak --dry-run` in đúng **10 vòng**, trả 0, **không** tạo
    `vis-soak-tmp-*`;
  - `cargo test -p vis-cli --test soak_script` → **2/2**;
  - soak thật không đặt `TMPDIR`/`VIS_SOAK_TMPDIR`: `TMPDIR=/home/ps/.cache/vis-soak-tmp-496953`,
    **10/10 vòng OK** (852 s), 3 gate **3638/4**, 3 diff `lech=0`, host **54/54**, cache **5/5**
    (self-test R1/R2/R3 xanh, không workaround). Log `/tmp/opencode/st296-soak-verify.log`.

## 2. `lsp::module::tests::hieu_nang_cache_5_tep_duoi_2ms` — chịu tải

- Giữ nguyên test (không `#[ignore]`); đổi phép đo:
  - 2 lượt **warmup**;
  - **median 9 vòng** (thay min-3 dễ đỏ oan khi scheduler chen giữa vòng);
  - ngưỡng **10 ms** — căn cứ: dưới tải ≥20, min-3 từng đo **2,134 ms**; median 9 đo được
    **0,557–0,804 ms** thường và **4,804 ms** một lần dưới load 30; 10 ms vẫn bắt hồi quy cache
    lớn hơn 10 ms. Ghi cả min để điều tra.
- Chạy `32` CPU burner (load **29,39 / 30,96**), test ×10: **10/10 PASS**; median
  **0,557–4,804 ms**. Log `/tmp/opencode/lsp-load20-10.log`.

## 3. Cổng cuối

| Lệnh | Kết quả |
|---|---|
| `sh -n bin/vis-soak` | OK |
| `--dry-run` | 10 vòng, rc 0, **không tạo tmp** |
| `cargo test -p vis-cli --test soak_script` | **2/2** |
| LSP test ×10 dưới load ≥20 | **10/10** |
| soak 10/10 mặc định TMP ngoài repo | **10/10 (852 s)** |
| `./vis-dev gate --fast` | **PASS — 3638 passed / 4 skipped** (3:12,83) |
| fmt / clippy / R1..R6 / report-check | OK (33 báo cáo, 327 hàng QUEUE) |

## 4. Blocker

Không còn blocker harness/timing: self-test trong soak xanh **không** cần `VIS_SOAK_TMPDIR`, test
LSP chịu được load ≥20. Không sửa sản phẩm; không tag/push/merge. Đĩa cuối **28 G** (> 10 G).
