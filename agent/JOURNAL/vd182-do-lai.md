# VD-182 + VD-195 — đo lại `bin/vis-dev rules` và `test/report/check_report.py`

**Ngày**: 2026-11-18 (giờ Nhật) · **Người làm**: agent `scripts-vd182` · **Nợ**: #147 (VD-182) + luật §8↔§4 (VD-195)
**HEAD lúc đo**: `6a32233` (`[ST-200] docs: QUEUE hàng ST-200 (brand icon)`)
**Phạm vi thay đổi**: **đúng 2 tệp** — `bin/vis-dev`, `test/report/check_report.py`. **0 dòng** trong `crates/`.
**Không commit** (theo yêu cầu). Hai tệp đang `M` trong `git status`.

---

## 0. Vì sao chậm (đo lại, không suy đoán)

| Nguyên nhân | Bản cũ | Bản mới |
|---|---|---|
| `rules`: `find` đi qua `target/` (2,2 GB, 433 000 tệp) | **3 lượt** | **1 lượt**, `-type d -name target -prune` |
| `rules`: R1 `wc -l` + `tr` **mỗi tệp** | 922 × 2 = 1 844 tiến trình | 3 tiến trình (`wc -l` gom lô 400 tệp) |
| `rules`: R2 `awk` **mỗi tệp** `.rs` | 224 tiến trình | 1 tiến trình |
| `rules`: R3 `head` + `grep` **mỗi tệp** | 922 × 2 = 1 844 tiến trình | 2 tiến trình (`printf` + 1 `awk`) |
| **`rules` — tổng tiến trình** | **3 915** | **8** |
| `report-check`: `git cat-file -e` **mỗi commit** | 3 107 tiến trình | 1 (`cat-file --batch-check`) |
| `report-check`: `git cat-file -e` **head** (2 lần/báo cáo: dòng 107 + dòng 108) | 29 × 2 = 58 | 0 (dùng lại kết quả lô) |
| `report-check`: `git show` + `git merge-base` **mỗi báo cáo** | 29 × 2 = 58 | 2 (`cat-file --batch` + `rev-list HEAD`) |
| **`report-check` — tổng tiến trình git** | **3 107 + 58 + 58 = 3 223** | **3** |

Cây lúc đo: **3 121 tệp** (trừ `target/`), trong đó **922** tệp thuộc R1/R3, **224** tệp `.rs` (R2),
**29** báo cáo `R*-viscript.json`, **231** hàng QUEUE.

---

## 1. Số đo thời gian (3 lần mỗi phép, ghi **trung vị**)

### 1.1 macOS (máy này, 12 core) — **có `cargo test` của agent khác chạy song song**, load 4,7–6,8

| Phép | Bản cũ (3 lần, giây) | **Trung vị** | Bản mới (3 lần, giây) | **Trung vị** | Nhanh hơn |
|---|---|---|---|---|---|
| `bash bin/vis-dev rules` | 28,36 · 27,51 · 26,84 | **27,51 s** | 1,33 · 1,27 · 1,22 | **1,27 s** | **21,7×** |
| `python3 test/report/check_report.py --all` | 128,05 · 145,49 · 171,34 | **145,49 s** | 0,77 · 0,78 · 0,81 | **0,78 s** | **186,5×** |

Lần đo thứ 4 của `rules` cũ (cùng tải với bản mới): **25,00 s** — trung vị 3 lần vẫn là 27,51 s.

### 1.2 Linux `ps` (12 core, load 0,74) — cùng cây, cùng HEAD, cùng 2 tệp

| Phép | Bản cũ (3 lần, giây) | **Trung vị** | Bản mới (3 lần, giây) | **Trung vị** | Nhanh hơn |
|---|---|---|---|---|---|
| `bash bin/vis-dev rules` | 6,06 · 6,27 · 6,04 | **6,06 s** | 0,16 · 0,14 · 0,14 | **0,14 s** | **43,3×** |
| `python3 test/report/check_report.py --all` | 5,09 · 5,03 · 5,08 | **5,08 s** | 0,11 · 0,12 · 0,12 | **0,12 s** | **42,3×** |

Cách đo trên `ps`: `rsync -a --exclude 'target/' ./ root@ps:/root/vd182-ps/` (6,5 s), rồi
`/usr/bin/time -f "%e" …` 3 lần; bản cũ chép sang thành `bin/vis-dev.old` + `test/report/check_report.old.py`.
Thư mục `/root/vd182-ps` là bản sao tạm, **không** đụng cây làm việc nào khác.

### 1.3 Tỉ lệ mac/ps (con số #147 quan tâm)

| Phép | Cũ | **Mới** |
|---|---|---|
| `rules` | 27,51 / 6,06 = **4,54×** | 1,27 / 0,14 = **9,07×** |
| `report-check` | 145,49 / 5,08 = **28,6×** | 0,78 / 0,12 = **6,5×** |

**Khoảng cách tuyệt đối** co lại rất mạnh: `rules` 21,45 s → **1,13 s** (19× nhỏ hơn);
`report-check` 140,4 s → **0,66 s** (213× nhỏ hơn).

### 1.4 Khoảng cách còn lại là **CPU**, không phải chờ I/O

| Phép (bản mới) | mac `user+sys` | ps `user+sys` | Tỉ lệ CPU |
|---|---|---|---|
| `rules` | 0,76–0,85 + 0,44–0,48 ≈ **1,25 s** | 0,10–0,12 + 0,05 ≈ **0,16 s** | **7,8×** |
| `report-check` | 0,60–0,61 + 0,10–0,12 ≈ **0,71 s** | 0,09–0,10 + 0,01–0,02 ≈ **0,11 s** | **6,5×** |

Bộ công cụ hai bên khác nhau (đây là phần lớn khoảng cách còn lại, **không** phải tạo tiến trình):

| | mac | ps |
|---|---|---|
| bash | **3.2.57** | 5.3.9 |
| awk | **BWK awk 20200816** | mawk 1.3.4 |
| python3 | **3.9.6** | 3.14.4 |

Đo thành phần bản mới trên mac (load 5,8): `find` 0,10 s · `wc` 922 tệp 0,19 s · `awk` R3 0,17 s ·
`awk` R2 (224 tệp `.rs`, 3,1 MB) **0,46 s** · vòng `case` bash trên 3 121 tệp 0,19 s ⇒ cộng ≈ 1,11 s.
Không còn tiến trình nào chiếm quá 0,12 s (8 tiến trình × 15,4 ms).

---

## 2. Chứng minh output **GIỐNG HỆT** bản cũ trên toàn cây tại cùng HEAD

### 2.1 `rules`

```bash
cd /Users/mps/dvchd/ViScript
bash bin/vis-dev      rules > /tmp/vd182-old-rules-1.out 2>&1   # bản CŨ
bash bin/vis-dev.new  rules > /tmp/vd182-new-rules-1.out 2>&1   # bản MỚI
diff /tmp/vd182-old-rules-1.out /tmp/vd182-new-rules-1.out
```

```
$ diff /tmp/vd182-old-rules-1.out /tmp/vd182-new-rules-1.out && echo "DIFF RỖNG"
DIFF RỖNG
$ cat /tmp/vd182-old-rules-1.out
--- rules: OK
```

### 2.2 `report-check`

```bash
python3 test/report/check_report.py      --all > /tmp/vd182-old-rep-1.out 2>&1   # bản CŨ
python3 test/report/check_report.new.py  --all > /tmp/vd182-new-rep-1.out 2>&1   # bản MỚI
diff /tmp/vd182-old-rep-1.out /tmp/vd182-new-rep-1.out
```

```
$ diff /tmp/vd182-old-rep-1.out /tmp/vd182-new-rep-1.out && echo "DIFF RỖNG"
DIFF RỖNG
$ cat /tmp/vd182-old-rep-1.out
report-check: PASS (29 báo cáo, 231 hàng QUEUE phủ đủ)
```

### 2.3 Sau khi `mv` đè, chạy lại trên **đường dẫn thật**

```
$ bash bin/vis-dev rules
--- rules: OK                                   [exit=0]   diff với bản cũ: RỖNG
$ python3 test/report/check_report.py --all
report-check: PASS (29 báo cáo, 231 hàng QUEUE phủ đủ)   [exit=0]   diff với bản cũ: RỖNG
```

### 2.4 Không nới ngưỡng, không xoá phép kiểm

`R1 > 1500` giữ nguyên; R2 giữ nguyên regex `(^|[^A-Za-z0-9_])unsafe([^A-Za-z0-9_]|$)` + bỏ `//…` trước khi
so; R3 giữ nguyên "5 dòng đầu" + sidecar `.license`; R4 giữ nguyên; mọi phép kiểm của `check_report.py`
(11 khoá, `project`, `round`, `head` tồn tại + là tổ tiên HEAD, `gates`, `sim`, `coverage`, `mutants`,
`perf`, `tasks` khớp QUEUE, `commits[]` tồn tại, phủ đủ QUEUE, `blocked[]`, `YC-` trong BLOCKED) giữ nguyên
**từng câu chữ**. Chỉ đổi **cách lấy dữ liệu** (gom tiến trình), không đổi **điều kiện**.

---

## 3. Ca âm có chủ đích — **cả bản cũ và bản mới đều bắt**

Dựng trong `/tmp/vd182-neg/` (không ghi gì vào repo).

### (a) Tệp vượt 1.500 dòng — `rules --root`

```bash
mkdir -p /tmp/vd182-neg/a
{ echo '// SPDX-License-Identifier: MIT OR Apache-2.0'
  for i in $(seq 1 1501); do echo "// dòng $i"; done; } > /tmp/vd182-neg/a/long.rs   # 1 502 dòng
bash bin/vis-dev     rules --root /tmp/vd182-neg/a
bash bin/vis-dev.new rules --root /tmp/vd182-neg/a
```

```
R1 /tmp/vd182-neg/a/long.rs: 1502 dòng > 1500
[exit=1]        ← hai bản in DÒNG GIỐNG HỆT NHAU
```

### (b) Báo cáo thiếu mục bắt buộc — `check_report.py --file`

```bash
cp agent/reports/R29-viscript.json /tmp/vd182-neg/b/bad-md.json
grep -v '^## 3\.' agent/reports/R29-viscript.md > /tmp/vd182-neg/b/bad-md.md   # bỏ mục '## 3.'
python3 test/report/check_report.py     --file /tmp/vd182-neg/b/bad-md.json
python3 test/report/check_report.new.py --file /tmp/vd182-neg/b/bad-md.json
```

```
not ok - bad-md.md: thiếu mục '## 3.'
# 1 mục sai
[exit=1]        ← hai bản in DÒNG GIỐNG HỆT NHAU
```

### (c) Báo cáo sai commit hash — `check_report.py --file --json-only`

```bash
python3 - <<'EOF'
import json; from pathlib import Path
d = json.loads(Path('agent/reports/R29-viscript.json').read_text())
d['tasks'][0]['commits'] = ['0123456789abcdef0123456789abcdef01234567']
Path('/tmp/vd182-neg/c/bad-commit.json').write_text(json.dumps(d, ensure_ascii=False, indent=2))
EOF
python3 test/report/check_report.py     --file /tmp/vd182-neg/c/bad-commit.json --json-only
python3 test/report/check_report.new.py --file /tmp/vd182-neg/c/bad-commit.json --json-only
```

```
not ok - bad-commit.json: ST-01 commit '0123456789abcdef0123456789abcdef01234567' không tồn tại trong git
# 1 mục sai
[exit=1]        ← hai bản in DÒNG GIỐNG HỆT NHAU
```

### (d) VD-195 — §8 sửa sai trong **bản copy tạm** ⇒ luật R5 phải đỏ

```bash
mkdir -p /tmp/vd182-neg/d/agent
cp agent/SO-NO.md /tmp/vd182-neg/d/agent/SO-NO.md
sed -i '' 's/^| `mở` | \*\*91\*\* |$/| `mở` | **90** |/' /tmp/vd182-neg/d/agent/SO-NO.md
bash bin/vis-dev.new rules --root /tmp/vd182-neg/d
```

```
R5 /tmp/vd182-neg/d/agent/SO-NO.md: mở: §8 90 ≠ §4 91;
[exit=1]
```

Đối chứng **âm của luật** (bản cũ **không** có R5 ⇒ xanh trên cùng cây sai — chứng minh luật là **mới**):

```
$ bash bin/vis-dev rules --root /tmp/vd182-neg/d
--- rules: OK
[exit=0]
```

Đối chứng **dương** (bản copy **nguyên vẹn** ⇒ xanh):

```
$ bash bin/vis-dev.new rules --root /tmp/vd182-neg/d-ok
--- rules: OK
[exit=0]
```

Ba biến thể sai khác nữa của R5 (đều đỏ, đúng câu):

```
R5 …/agent/SO-NO.md: tổng: §8 150 ≠ §4 151;
R5 …/agent/SO-NO.md: đóng: §8 36 ≠ §4 37;
R5 …/agent/SO-NO.md: §8 thiếu hàng: chuyển VS3;
```

### (e) Bộ ca âm sẵn có vẫn xanh (chạy lại, không sửa kỳ vọng)

- `test/fixtures/run_fixtures.sh` ca 1–4 (`rules --root`): **output bản cũ = bản mới, từng byte** —
  `R3 … thiếu SPDX…` · `R1 … 1502 dòng > 1500` · `R2 …:2: có 'unsafe' …` · `R4: vis-host-vifast …`.
- `test/report/run_fixtures.sh` 5 fixture (`--file --json-only`): **output bản cũ = bản mới, từng byte**
  (15 · 14 · 15 · 13 · 16 dòng) — `bad-head` (head), `bad-commit` (commit), `bad-status` (queue),
  `missing-task` (thiếu task), `bad-project` (project).

---

## 4. Luật R5 (VD-195) — thiết kế

Thêm vào `run_rules()` (nên `./vis-dev check` **và** `./vis-dev rules` đều có; `--root` dùng được cho ca âm):

- **§4**: mỗi hàng dữ liệu (cột 1 khớp `^[0-9]+[a-z]?$`) ⇒ chuẩn hoá cột 4 (bỏ `` ` ``/`*`) rồi phân loại
  `mở` / `đóng` / `chuyển VS3` / `không làm` / `điều hướng`. Trạng thái **lạ** ⇒ đỏ (nêu số hàng).
- **§8**: đọc bảng `Chỉ số | Số`, lấy **số nguyên đầu tiên** của cột 2; khớp nhãn
  `Tổng…` / `mở` / `đóng` / `chuyển VS3` / `không làm…` / `…điều hướng…`. Thiếu hàng ⇒ đỏ.
- So **6 chỉ số** (tổng dòng + 5 trạng thái); lệch ⇒ in `R5 <tệp>: <chỉ số>: §8 X ≠ §4 Y;` và `exit 1`.
- Không có `agent/SO-NO.md` trong cây ⇒ **bỏ qua** (cây tạm của self-test R1..R4 không có tệp này).
- **Một** tiến trình `awk` cho cả hai bảng — không tạo tiến trình mỗi hàng.

**Số thật lúc đo** (`awk -f` chạy trực tiếp trên `agent/SO-NO.md`):

```
§4 = 151 dòng = 91 mở / 37 đóng / 21 chuyển VS3 / 1 không làm / 1 điều hướng
§8 = 151 dòng = 91 mở / 37 đóng / 21 chuyển VS3 / 1 không làm / 1 điều hướng   ⇒ R5 XANH
```

**Diễn biến**: lúc tôi bắt đầu (14:03) §8 còn **147/89/34** trong khi §4 đã **148 dòng** ⇒ R5 **sẽ đỏ**
đúng như dự kiến. Lead sửa §8 song song; lúc đo (14:14 trở đi) §8 đã khớp §4 (151 = 91/37/21/1/1) ⇒ R5 xanh.
**Tôi không sửa §8** — chỉ đọc.

**Một thay đổi câu chữ có chủ đích**: `do_check` in `--- luật R1..R4` → `--- luật R1..R5` (bộ luật đã
tăng; không test nào grep chuỗi này — đã `grep` toàn repo, chỉ có trong `agent/JOURNAL/` lịch sử).
`rules` vẫn in `--- rules: OK` **nguyên văn**.

---

## 5. Kết luận #147

**Tỉ lệ mac/ps sau khi sửa: `rules` 9,07× · `report-check` 6,5× — vẫn > 3×.**
Theo tiêu chí của VD-182, **đóng #147** với câu:

> **giới hạn nền macOS (tạo tiến trình 9,6×)**

Kèm số đo để không hiểu sai bản chất (đây là phần **quan trọng** cho lần sau):

- Hai phép đã **nhanh hơn 21,7×** (mac) và **43,3×** (ps) với `rules`; **186,5×** (mac) và **42,3×** (ps)
  với `report-check`. Khoảng cách tuyệt đối mac−ps co từ **21,45 s → 1,13 s** và **140,4 s → 0,66 s**.
- Số tiến trình: `rules` **3 915 → 8**; `report-check` **3 223 → 3**. Đề xuất (i) của #147
  ("gộp `report-check`/`rules` để không tạo tiến trình mỗi tệp") **đã làm xong**.
- **Nhưng** khoảng cách còn lại **không** còn do tạo tiến trình: 8 tiến trình × 15,4 ms ≈ **0,12 s**
  trên tổng 1,13 s. Đo `user+sys` cho thấy phần lớn là **CPU của công cụ cũ trên mac**:
  bash **3.2.57** vs 5.3.9, **BWK awk** vs mawk 1.3.4, **Python 3.9.6** vs 3.14.4 (tỉ lệ CPU 7,8× / 6,5×).
  Nói cách khác: #147 đóng vì **nền macOS + bộ công cụ hệ thống cũ**, không phải vì mã còn tạo tiến trình
  mỗi tệp — và câu "giới hạn nền macOS" ở trên vẫn đúng tinh thần, chỉ khác là **thủ phạm đã dịch từ
  `fork/exec` sang `awk`/`bash`/`python` bản cũ**.

**Việc còn lại (không thuộc VD-182, ghi để A0 quyết)**: nếu muốn kéo mac/ps xuống ≤ 3× thì phải
(a) cài `gawk`/bash 5/python 3.12+ trên mac, hoặc (b) viết lại R1–R3 bằng **một** `python3` (mac có 3.9.6,
chênh ít hơn awk), hoặc (c) chấp nhận: thời gian tuyệt đối 1,27 s / 0,78 s đã dưới mọi ngưỡng thực dụng.

---

## 6. Tệp đã đổi

| Tệp | Dòng | sha256 (mới) | sha256 (cũ) |
|---|---|---|---|
| `bin/vis-dev` | 299 → **432** | `3cfbcd3002dfa86f59f9c8e67edd88098d2681a6dbd46b9b57f573226dbf7dcf` | `95ad13dee8d0bab158d6e9e0b17d8a521146dacd2ff175abd398136426b1572c` |
| `test/report/check_report.py` | 208 → **305** | `1f160dcaa5b0ad50259c17f0d0a563b5a9b44089674475a81083f531e21a0d7f` | `fe25ce52cc3b547edccb1441a449ec5bbe33130484133858aa275e4555411a2b` |

`git status --short` chỉ có hai tệp này thuộc về tôi; các tệp `crates/` đang `M` là của agent khác
chạy song song — **tôi không đụng** (`git diff --stat -- crates/` không có dòng nào do tôi tạo).

Quy trình: phát triển trong `bin/vis-dev.new` + `test/report/check_report.new.py`, kiểm xong **cả hai**
mới `mv` đè (đổi tên nguyên tử). Không chạy `cargo`, không chạy `./vis-dev check`, không chạy
`./vis-dev gate` — chỉ `bash bin/vis-dev.new rules`, `python3 …check_report.new.py …`, `bash -n`, `diff`.
