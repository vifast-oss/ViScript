<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# Gatekeeper — Kiểm toán tài liệu/trạng thái cuối Sóng 4 (`ST-315a..ST-315d`)

> **Vai trò**: Gatekeeper (Agent 1, phiên sạch) · **Nhánh**: `review/rdocs2`
> @ `8a7d7fed` (nền `ST-315b` docs) · **Ngày**: 2026-11-26
> **Máy**: `ps` (Linux); **không** dùng `mps`; **không** sửa mã sản phẩm; **không** push/merge.
> **Kết luận**: **FAIL (tài liệu)** — trạng thái 4 lát `REVIEW`, các cổng máy và bằng chứng Windows
> cơ bản đúng, nhưng `ST-315d` còn **overclaim** trong `JOURNAL/st315d-parity.md`; `QUEUE`/`STATE`/
> `SO-NO` còn **số liệu cũ** (10 test thay vì 12, plugin-host 28/28 thay vì 30/30); bản re-audit
> R-Parity `FAIL (nhẹ)` trên nhánh `review/rparity` chưa được phản ánh. Không phát hiện lỗi mã sản
> phẩm mới.

## 0. Bảng kiểm tổng

| # | Hạng mục | Kết quả | Ghi chú |
|---|---|---|---|
| 1 | `QUEUE.md` — 4 hàng `ST-315a..d` | **PASS (trạng thái) / FAIL (nhẹ, số liệu)** | Cả 4 hàng đúng `REVIEW`, không còn “đang thi công/`READY`” sai. Nhưng `ST-315d` còn “10 test hai đường”/“`host_parity` 10/10” trong khi bản cuối có **12** `#[test]`. |
| 2 | `STATE.md` / `ACTIVE.md` / `ROADMAP.md` | **PASS (trạng thái) / FAIL (nhẹ, số liệu)** | Cả 3 tệp ghi 4 lát `REVIEW`; `ST-309` `BLOCKED`; Sóng 5 `BLOCKED`. `STATE.md:16` còn `plugin-host 28/28` (bản cuối **30/30**). |
| 3 | `SO-NO.md` — `#197`, §8, R5 | **PASS** | `#197` đóng ở `ST-315c`; §8 **78 mở + 101 đóng + 16 VS3 + 2 không làm + 1 điều hướng = 198**; `vis-dev rules` R5 xanh. Dòng cập nhật còn số cũ `10/10 + 18/18` (không ảnh hưởng `#197`). |
| 4 | `API-CODES.txt` — `VS0304` + 57 mã | **PASS** | 57 mã trong tệp = 57 `all_codes()` (tập khớp tuyệt đối); diff so bản trước chỉ **thêm** dòng `VS0304`, chuỗi cũ không đổi. |
| 5 | Commit hygiene | **PASS** | `[ST-00]` `3ffd74d0`/`89e9469e` không có `.rs`; `[REVIEW]` chỉ journal; không có `.log`/`.out` trong repo; **5 worktree ≤ 8**; trong nhóm `plan/*` chỉ `plan/st309-qa` chưa merge (đúng `BLOCKED`). |
| 6 | Bằng chứng Windows (`ST-315b`, `ST-315c`) | **PASS** | Log tồn tại, sha local/remote khớp; **3777/0/4** + host 54/54; **3747/0/4** + host 54/54. Xem caveat: cả hai chạy trước harness 12 test cuối. |
| 7 | `JOURNAL/st315d-parity.md` — R-Parity, không overclaim | **FAIL (nhẹ)** | 17 ca lỗi `fs`, message `net`, 66/66 **đúng**; nhưng còn 3 câu overclaim và bỏ qua re-audit `FAIL (nhẹ)` trên `review/rparity`. |
| 8 | Cổng máy bắt buộc | **PASS** | `vis-dag-check`, `vis-dev rules`, `check_report.py --all`, `git diff --check` đều xanh. |

## 1. Trạng thái và số liệu `QUEUE` (mục 1–2)

### 1.1 Trạng thái — PASS

- `QUEUE.md:2228..2231`: `ST-315a` `REVIEW`, `ST-315b` `REVIEW`, `ST-315c` `REVIEW`,
  `ST-315d` `REVIEW`.
- Không còn câu “đang thi công”/“`READY`” cho 4 lát này. Các chữ `READY` còn lại trong `QUEUE.md`
  thuộc task cũ (`ST-05`, `ST-09`, `ST-212`, `ST-241`, `ST-243`, `ST-248`, `ST-159`), không thuộc Sóng 4.
- `STATE.md:16`, bảng `ACTIVE.md:18..21`, `ROADMAP.md:7`: cả 4 lát `REVIEW`.
- `ST-309` vẫn `BLOCKED` (`QUEUE.md:2191`, `STATE.md:14`, `ACTIVE.md:7`, `ROADMAP.md:4`);
  `ST-316a..c` vẫn `BLOCKED` (`QUEUE.md:2232..2234`, `ACTIVE.md:22..24`).

### 1.2 Bằng chứng theo commit — PASS, trừ số test `ST-315d`

| Lát | Commit / bằng chứng | Đối chiếu |
|---|---|---|
| `ST-315a` | `50bb7885` (mã), `96c6ad45` (review PASS), `54597d30` (F2), `c6724b46` (docs) | `opt-bench` interp **329** / VM **344** / unlimited **342** ms; sha `6574583…7cb48c` khớp `/home/ps/vislogs/st315a_opt_bench.md`; `vis-diff 100k lech=0`; `lat256_budget` **9 passed + 1 ignored**; `vis-vm` **56/56**. |
| `ST-315b` | `7dc21cf2` (mã), `8ca08441` (merge), `c6724b46` (Windows docs) | `Value::Map(Rc<VisMap>)`; `size_of::<Value>() == 64`; `st315b_map_cow` **6/6**; `opt-bench` **48/46 ms**; `mapbench N=316` **49,5 ms** so nền **1.328 ms** (~27×, bảng journal); Windows **3777/0/4**. |
| `ST-315c` | `fbcbc139` (mã), `987b98b8` (Windows docs), `801719e9` (R-C) | `#197` `Truncate`; landlock **19/19**; plugin-host cuối **30/30** (12 lib + 18 tích hợp); Windows **3747/0/4**. |
| `ST-315d` | `c2b051c9` + `ee0dcd8d` + `5f8f8012` + `0ba3e8f2` (VS0304/Tarjan), `9bbb77dd`; parity `a45438e6`/`25790c97` | VS0304 + 57 mã; Tarjan lặp; parity **66/66**; **nhưng** hàng QUEUE còn “10 test hai đường”/“`host_parity` 10/10”. |
| Docs | `89e9469e` (`[ST-00]` sửa Tarjan/ACTIVE/ROADMAP/sha), `8a7d7fed` (Windows `ST-315b`) | Các lỗi của báo cáo `review-song4-docs.md` trước đã được xử lý. |

**Lệch cụ thể (F1):** `QUEUE.md:2231` được sửa lần cuối bởi `d2b7eda8` (13:32) — **trước** bản vá
parity `a45438e6`/`25790c97`. Harness hiện tại có **12** `#[test]` (không phải 10); trong đó **3**
test khoá/self-test không gọi `vis` (`phu_du_66_ham_host`, `moi_ham_host_co_loi_goi_trong_harness`,
`bo_chu_thich_vis_khong_tinh_lenh_bi_comment`), **9** test chạy hai đường. Hàng QUEUE chưa nhắc
`a45438e6`/`25790c97` và còn số 10.

## 2. `JOURNAL/st315d-parity.md` — FAIL (nhẹ, overclaim)

Các số liệu chính **đúng**:

- `fs_loi_moi_ham_parity` có đúng **17** nhãn lỗi (`read`, `size`, `remove`, `mkdir`, `mkdir_all`,
  `copy`, `rename`, `read_link`, `canonicalize`, `list`, `walk`, `glob`, `append`, `write`,
  `read_bytes`, `read_lines`, `write_bytes`), in nguyên văn `e.message`.
- `net_loopback_parity` in `e.message` cho các nhánh lỗi; timeout/handle được ghim.
- `66/66` hàm host có lời gọi trong script; `HOST_66` 66 tên duy nhất; R8 **39 ≤ 40**.

Các câu **overclaim** còn lại:

1. `:55` — “**không còn nhóm nào 0 ca lỗi**”. Sai: nhóm `time` (2 hàm) không có nhánh lỗi, chỉ
   kiểm `now_ms > 0` và `monotonic_ms` không giảm. Re-audit R-Parity đã bắt đúng điểm này (F-R4).
   Bản `347e9cae` trên `main` sửa câu này, nhưng **không có trong snapshot `8a7d7fed`**.
2. `:37` — “**mọi ca** chạy cả hai đường `vis run` và `vis run --vm --no-cache`”. Sai: 3/12 test
   chỉ kiểm bảng `HOST_66`/bộ bóc chú thích, không chạy engine.
3. `:122` — “**mỗi nhóm** có ca lỗi in thông điệp thật”. Sai vì `time` không có nhánh lỗi; câu này
   vẫn còn ở bản `347e9cae` trên `main`.
4. `:59` — “`net`: **8 nhánh lỗi**” là under-count/không rõ; script `SRC_NET` có nhiều nhánh lỗi hơn
   (listen trùng cổng, `accept_timeout`, đọc/ghi sau `close`, `close_listen` lần 2,
   `accept`/`local_addr` sau đóng, tay `C99`, connect bị từ chối…). Bản `347e9cae` đã đổi thành
   “**các nhánh lỗi**”, nhưng snapshot này chưa có.

**Re-audit chưa được phản ánh (F4):** nhánh `review/rparity` (worktree `/home/ps/ViScript-rparity`)
có báo cáo `agent/JOURNAL/review-st315d-parity.md` tại `067f9a26` (tip `78965856` sau merge `main`)
kết luận **FAIL (nhẹ)**:

- **F-R1 [M]**: bọc **một** lời gọi `fs.read_bytes(` trong `SRC_FS` (còn 2 lời gọi khác) thì test
  `moi_ham_host_co_loi_goi_trong_harness` **vẫn xanh**; chỉ khi bọc **cả 3** mới đỏ. Nếu hợp đồng
  A0 là “xoá một lời gọi phải đỏ” thì chưa đạt; journal `st315d-parity.md` chưa nêu hợp đồng này.
- **F-R2 [M]**: `cwd-khop` so `fs.current_dir()` với `fs.canonicalize(".")`; trên Windows
  `canonicalize` trả dạng verbatim `\\?\C:\…` còn `current_dir` trả `C:\…` ⇒ có nguy cơ `false`
  (đỏ) trên `psvn`. Chưa có bằng chứng Windows cho harness 12 test.
- `main` đã có `347e9cae` (14:18) vá `cwd-khop` bằng cách canonicalize **cả hai vế** và sửa F-R4
  trong journal, nhưng: (a) chưa có log `psvn`; (b) báo cáo FAIL trên `review/rparity` chưa được
  merge/phản ánh vào `QUEUE`/`STATE`; (c) nhánh `review/rdocs2` đang đứng trước `347e9cae`.
  Vì vậy **chưa thể coi `ST-315d` là PASS 3-OS** chỉ từ các tài liệu trong snapshot này.

## 3. `SO-NO.md`, `API-CODES.txt` (mục 3–4)

- `SO-NO.md:286`: `#197` = “**đóng ở ST-315c**”, mô tả đúng `ReadFile + WriteFile + Truncate` và
  ca #19; `vis-host-linux --test landlock` **19/19**.
- §8 `SO-NO.md:402..411`: **198 = 78 + 101 + 16 + 2 + 1**; `./vis-dev rules` R5 **PASS**
  (`--- rules: OK`).
- Ghi chú nhỏ: `SO-NO.md:20` còn “plugin-host lib **10/10** + tích hợp **18/18**” (bản cuối
  **12/12 + 18/18 = 30/30**); header `SO-NO.md:83` còn “185 dòng” và `:288` “21 mục” là số cũ có
  từ trước Sóng 4 — không tính FAIL Sóng 4 nhưng nên dọn.
- `API-CODES.txt:32` có `VS0304`; đếm **57** dòng mã, `all_codes().count() == 57`; so tập mã
  `agent/API-CODES.txt` ↔ `crates/vis-diag/src/lib.rs` **khớp tuyệt đối**; diff
  `c2b051c9^..c2b051c9` chỉ **thêm** dòng `VS0304`, không đổi chuỗi cũ.

## 4. Commit hygiene (mục 5)

- `[ST-00]` trong dải: `3ffd74d0` (mở Sóng 4) và `89e9469e` (đồng bộ Tarjan/ACTIVE/ROADMAP/sha).
  Cả hai **0 tệp `.rs`**; chỉ chạm `agent/*`, `docs/SPEC*.md`, `docs/ViScript-thiet-ke.md`.
- `[REVIEW]` (`6cae7215`, `5bd56cd5`, `86df7ea6`, `7b0a8694`, `2fc47130`, `c51d19c8`,
  `6b2da318`) chỉ thêm journal, không có mã sản phẩm.
- Không có `.log`/`.out`/`.exit` trong `git diff 3ffd74d0..HEAD`; log bằng chứng nằm ở
  `/home/ps/vislogs/` (R7).
- `git worktree list`: **5** worktree (`ViScript`, `r315b`, `rdocs2`, `rparity`, `rwin2`) **≤ 8**.
  Chỉ `plan/st309-qa` chưa merge (đúng `ST-309` `BLOCKED`); không còn `plan/*` đã merge.
- Quan sát: worktree `/home/ps/ViScript-r315b` giữ nhánh `review/r315b` đã merge (`8ca08441`);
  `review/r4docs` cũng đã merge nhưng không còn worktree. Nếu áp dụng `VD-288` cho mọi nhánh đã
  merge (không chỉ `plan/*`), nên dọn trong 24h. Không chặn cổng này.
- Quan sát: `d2b7eda8` (`[ST-315d]`) có 2 dòng `vis-plugin-host/src/wasi/runner.rs` (RAII ST-315c)
  — đã được ghi trong `review-st315d.md` R8 như vấn đề traceability, không phải mã sai.

## 5. Bằng chứng Windows (mục 6)

### `ST-315b` — `JOURNAL/review-st315b-win.md`

- `psvn`, `main` @ `c6724b466a10f52857f950c82bbc38ec5e8bfe3a`, cây sạch.
- `cargo clippy --workspace --all-targets -D warnings` + `cargo test --workspace --no-fail-fast`
  + host riêng + `vis-targets`: exit 0.
- Tổng log: **3777 passed / 0 failed / 4 ignored**, **90 block**; host **54/54**; targets core
  **47 PASS**, sqlite **46 PASS + 1 EXPECTED_RED**.
- Local `/home/ps/vislogs/st315b-win-full-c6724b46.log` (254 319 B) sha256
  `a24d7b916981aaef89194d9fb49751610840ef207b868f4d99548fec245a0496` — khớp journal.
- Remote `/c/Users/ps/vislogs/vis-remote-20261126T050511Z-3170059.log` sha256
  `71f6bd6172f94d8d73be08a01515c1cd2972ea84545b6e8d6faa2b048ab805fe` — đọc lại trên `psvn`,
  khớp `# sha256(log_tren_may_dich)` ở cuối log local.

### `ST-315c` — `JOURNAL/st315c-w3.md`

- `psvn`, `main` @ `c2b051c91acf409d7a9dba0c22120d8569d321e1`, cây sạch.
- Tổng log: **3747 passed / 0 failed / 4 ignored**, **90 block**; host **54/54**; targets
  core 47 / sqlite 46 + 1 EXPECTED_RED.
- Local `.log` sha256 `a2c20514cde524d760d75b0b1d636512c8d58de20a1110df682bb04d9dd1d776`;
  `.out` sha256 `d2cec75750d69c037107f8d569dd4ed8fb7ec7c56f885fd0c731f49bda1c076d`;
  remote `/c/Users/ps/vislogs/vis-remote-20261126T032705Z-1881899.log` sha256
  `11cbf1ace73bb7f4cfc220deada1d183362ab1c295bb571c5a5c323884f82124` — đọc lại trên `psvn`,
  khớp journal.
- **Caveat:** cả hai lượt Windows chạy tại `c6724b46`/`c2b051c9`, **trước** bản vá parity
  `a45438e6`/`25790c97` và trước `cwd-khop` `347e9cae`; chưa có lượt Windows cho harness 12 test
  cuối. Đây là khoảng trống bằng chứng, không phải log sai.

## 6. Cổng máy đã chạy lại (mục 8)

```text
$ ./bin/vis-dag-check
vis-dag-check: OK — 23 crate / 5 tầng, 0 cạnh cấm

$ ./vis-dev rules
vis-dev: chọn cargo /home/ps/.cargo/bin/cargo (1.100.0; yêu cầu ≥ 1.100.0)
--- rules: OK

$ python3 test/report/check_report.py --all
report-check: PASS (35 báo cáo, 361 hàng QUEUE phủ đủ)

$ git diff --check
(không lỗi; exit 0)
```

## 7. Đề xuất cho A0

1. **Chưa duyệt `ST-315d` `DONE`** cho tới khi chốt R-Parity:
   - chạy `cargo test -p vis-conformance --test lib_api_it host_parity` trên `psvn` với
     `347e9cae` (hoặc bản sau) và ghi log ngoài repo; xác nhận `cwd-khop` xanh;
   - chốt hợp đồng F-R1: “mỗi hàm cần ≥ 1 lời gọi” (như journal `347e9cae` giải thích) hay
     “xoá **một** lời gọi phải đỏ” (như re-audit `067f9a26` đòi). Nếu chọn vế sau, phải siết test.
2. **Cập nhật tài liệu** (Agent 2, sau khi A0 chốt):
   - `QUEUE.md:2231`: 12 `#[test]` (9 hai đường + 3 khoá/self-test), thêm `a45438e6`/`25790c97`
     và kết quả re-audit R-Parity;
   - `STATE.md:16`: `plugin-host 30/30` (12 lib + 18 tích hợp), không còn 28/28;
   - `SO-NO.md:20`: `12/12 + 18/18`; dọn header cũ `:83`/`:288` nếu có dịp;
   - `JOURNAL/st315d-parity.md`: áp `:55`/`:59` theo bản `347e9cae`; sửa thêm `:37` (9/12 test
     chạy hai đường, 3 test khoá/self-test) và `:122` (“trừ `time`”) — hai câu này `347e9cae`
     chưa sửa;
   - `JOURNAL/st315d-vs0304.md:5`: đổi `READY (còn parity)` vì parity đã merge.
3. **Đồng bộ snapshot**: `main` đã tiến thêm `347e9cae` sau nền `8a7d7fed`; nhánh docs nên rebase/
   cherry-pick hoặc ít nhất ghi rõ mốc này trước khi A0 duyệt. `review/rparity` (`78965856`) chưa
   merge — cần merge báo cáo hoặc lưu quyết định “chấp nhận F-R1/F-R2 có caveat”.
4. **Vệ sinh nhẹ**: dọn worktree/nhánh `review/r315b` đã merge; giữ `plan/st309-qa` (chưa merge,
   `ST-309` `BLOCKED`); sửa câu KPI `st315b-map-cow.md:111` (1.429 ms) cho khớp bảng `:109`
   (1.328 ms) và `QUEUE` (~27×).

## 8. Kết luận

- **PASS**: trạng thái 4 lát `REVIEW`; `ST-309`/Sóng 5 `BLOCKED`; `#197` đóng đúng; §8 = 198;
  `VS0304` + 57 mã khớp `all_codes()`; `[ST-00]` sạch, không `.rs`; log ngoài repo; worktree 5 ≤ 8;
  hai log Windows 3777/0/4 và 3747/0/4 khớp sha local/remote; 4 cổng máy xanh.
- **FAIL (tài liệu)**: `ST-315d` còn overclaim trong `st315d-parity.md` (time không có ca lỗi;
  3/12 test không chạy hai đường; “8 nhánh net” cũ) và số liệu `10 test`/`28/28` cũ trong
  `QUEUE`/`STATE`/`SO-NO`; re-audit R-Parity `FAIL (nhẹ)` trên `review/rparity` chưa được phản ánh;
  snapshot docs đứng trước `347e9cae`.
- **Không có lỗi mã sản phẩm mới** trong phạm vi tài liệu này; các sai lệch đều sửa được bằng
  cập nhật tài liệu/ánh xạ bằng chứng, trừ quyết định hợp đồng F-R1 và lượt Windows cuối.
