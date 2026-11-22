<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-300 — baseline native interp↔VM cho `test/sem/**/*.vis`

**Nhánh**: `plan/w1-sem-native-baseline` từ `main` **`16fba3a`**. Không merge/push/tag.
**Phạm vi**: chỉ đo baseline native; **không** sửa code sản phẩm, không đụng `bin/vis-wasm-suite`,
không chạy gate/soak/diff (W2/W4 đang chạy).

## 1. Binary + lệnh chạy

- Binary release: `/home/ps/ViScript/target/vs3-w1-sem/release/vis`
  sha256 `275c985e99e1ff66585e268b2d2736707a02073d8b960506211267754b23c071` (build từ `16fba3a`).
- **Interp**: `vis run <tệp.vis>`
- **VM**: `vis run --vm <tệp.vis>` (`--workers` mặc định 1, `--max-budget` mặc định 40M).
- Mỗi lần: `timeout 20`, cwd = gốc repo, `TMPDIR=/home/ps/tmp`; stdout lưu file tạm rồi sha256.
- Script tái lập: `agent/JOURNAL/st300-sem-native-baseline.sh`.

## 2. Kết quả

| Chỉ số | Giá trị |
|---|---:|
| Tệp `test/sem/**/*.vis` | **475** |
| Chạy được độc lập (không timeout/crash/read error) | **475** |
| Loại trừ | **0** |
| Exit 0 (chương trình sạch) | **397** |
| Exit 2 (lỗi kiểu/parse **cố ý**, có `.err`) | 48 |
| Exit 101 (trap runtime **cố ý**) | 30 |
| **Parity interp↔VM (rc + sha256 stdout)** | **475/475** |

- TSV: **`/home/ps/vislogs/sem-native-baseline.tsv`**
  sha256 **`482b35c276d76fca9200770f78b3cba891535b09b65dbce0fc52bce6379a308d`**
  (475 dòng dữ liệu + header; chạy lại 2 lần **cùng sha** — tất định trên máy này).
- Loại trừ: `/home/ps/vislogs/sem-native-exclusions.tsv` — chỉ header, **0 tệp**.
- Cột TSV đúng hợp đồng: `path, category, interp_rc, interp_stdout_sha256, vm_rc, vm_stdout_sha256`;
  `category` = thư mục cấp 1 dưới `test/sem`.
- 78 ca non-zero là **ca âm có chủ đích** (`.err`/trap), không phải “không có entrypoint”:
  đã kiểm 15 ca đầu — toàn `VS0412`/`VS0405`/`VS0501`/`VS0506`… đúng kỳ vọng; không ca nào
  thiếu `main`/entrypoint.

Phân bố category (tệp): sqlite 38, optional 34, async 26, list 25, ham 25, ban-ghi 25, loi 24,
match-nang-cao 23, chuoi-phuong-thuc 22, chuoi 22, so-hoc-mep 21, long-du-lieu 21, result 20,
generic 20, so-hoc 19, pham-vi 19, map 19, enum 19, dieu-khien 19, kenh 15, trait 13, cap 6.

## 3. Ghi chú cho ST-300 (wasm suite)

- 397 ca exit 0 là tập “chạy sạch” phù hợp so byte với native; 78 ca còn lại vẫn chạy được
  nhưng là ca âm (rc ≠ 0, stdout rỗng) — wasm runner có thể dùng để khoá **mã lỗi/rc** hoặc
  loại khỏi mẫu ≥85 % tuỳ hợp đồng; baseline đã có đủ rc/hash cho cả hai.
- Không thấy khác biệt interp↔VM trên toàn bộ 475 tệp ở chế độ mặc định (W=1, không args).
- Không có loại trừ do timeout/crash/entrypoint trên máy đo.
