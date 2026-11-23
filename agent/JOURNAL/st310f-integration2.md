<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-310f — tích hợp2: họp `main` `8906c70` vào nhánh `v0.5.5`

> **Phạm vi**: nhánh `plan/v055-integration2` tạo từ **`f0ff9cc`** (tích hợp Phase B
> `ST-310a..f`), merge **`main` `8906c70`** (`--no-ff`) để lấy **R36 final2** + vá
> **ST-309** Darwin fallback. **Không** merge vào `main`, **không** push/tag, **không**
> ssh `mps`/`psvn`, **không** bump `0.5.5`.
> Worktree riêng: `/home/ps/tmp/v055-integration2` (không checkout `/home/ps/ViScript`).
> Môi trường: Rust **1.100.0**, `CARGO_BUILD_JOBS=3`, `TMPDIR=/home/ps/tmp`.
> Thời điểm: 2026-11-23 (đêm), time-box tới 09:00.

## 1. Merge

| Mục | Giá trị |
|---|---|
| Base nhánh | `f0ff9cc` — `[ST-310f] tích hợp v0.5.5` |
| Nguồn hợp nhất | `main` `8906c70` — `[ST-309] fix Darwin fallback O_NOFOLLOW` |
| Kiểu merge | `git merge --no-ff main` (giữ commit merge, không fast-forward) |
| Commit merge | `[ST-310f] integration2: hop main 8906c70` |

`main` hơn base đúng **2 commit**: `979d28e` (R36 final2, docs-only) và `8906c70`
(ST-309, sửa `quyen/guard.rs` + `quyen/mo.rs`). Toàn bộ mã ST-310a..f của base được giữ.

## 2. Conflict & cách xử lý

Git **tự động** hợp nhất rename-vs-modify của `ST-310f` cho
`crates/vis-host-native/src/quyen/{guard,mo}.rs` (base phẳng → `quyen/` lồng nhau), nên
**không có conflict văn bản** ở hai tệp này. Đã kiểm lại từng hunk để chắc chắn không mất bên nào:

- `quyen/guard.rs`: giữ `Errno::NOTDIR` trong `FsGuard::loi_mo` cạnh `LOOP`/`XDEV`
  ⇒ symlink/không-phải-thư-mục trên Darwin vẫn ánh xạ **`VS0604`** (ST-309).
- `quyen/mo.rs`: giữ fallback Darwin **bỏ `O_DIRECTORY`** ở `mo_goc`/`mo_duoi_goc`/
  `tao_thu_muc_de_quy` để symlink trả `ELOOP` (không `ENOTDIR`) — đúng ST-309; **đồng thời**
  giữ nguyên `mo_goc`, `mo_tep_doc`, `mo_tep_ghi` của **ST-310c** (plugin tĩnh dùng lại
  primitive `openat2`, không bản sao syscall).

Conflict văn bản thật (3 tệp, đều thuộc tài liệu R36):

| Tệp | Xử lý |
|---|---|
| `agent/reports/R36-viscript.md` | Lấy **bản `main` final2** (QA cuối @`e9c62cb`, 339 hàng QUEUE); chỉ đổi câu "sinh lại" sang `python3 test/report/gen_report_json.py --round 36` vì ST-310f đã hợp nhất script. |
| `agent/reports/R36-viscript.json.license` | Lấy **bản `main` final2** (giữ nguyên attribution lịch sử `gen-r36-json.py`). |
| `agent/JOURNAL/gen-r36-json.py` | **Xoá** theo ST-310f (modify/delete): script đã hợp nhất vào `test/report/gen_report_json.py`, tránh hai nguồn sinh JSON. |

`agent/reports/R36-viscript.json` auto-merge **trùng byte** bản `main` (đã kiểm `git diff main`: rỗng);
JSON hợp lệ, `head=e9c62cb…`, `round=36`, 339 task.

## 3. Giữ tài liệu & nợ

- **R36 docs**: giữ bản final2 của `main` (`.md`, `.json`, `.json.license`); không quay về bản QA prep.
- **`#194`**: giữ trong `agent/SO-NO.md` (race metadata-only FS, `st309-security.md` §4).
- **`#195`**: giữ trong R36 `.md`/`.json` + `agent/JOURNAL/st309-qa.md` + `tag-v050.md`
  (đề xuất VM back-edge O(n²)/fuel, **chưa cấp số chính thức**).
- **Không** bump `0.5.5` (workspace vẫn `0.5.0`), **không** tag, **không** push.

## 4. Kết quả kiểm trên cây merge

| Hạng mục | Kết quả |
|---|---|
| `./vis-dev gate --fast` | **PASS — 3773 passed / 0 failed / 4 skipped**; fmt/clippy/R1..R9/toolchain/release/self-test/build/doc-examples/readme-examples/report-check đều PASS |
| `bin/vis-targets` | **PASS — 18/18 crate workspace**; `core` **38 PASS / 0 EXPECTED_RED / 0 FAIL**; `sqlite` **36 PASS / 2 EXPECTED_RED (#156) / 0 FAIL** |
| Diff 100k (`bin/vis-diff`, seed `20261115`) | **LECH = 0** — 100.000 chương trình, 93.770 chạy, 6.230 lỗi tính, 79,3 s |
| `cargo check --workspace --all-targets` | **PASS** — 0 error, **0 warning** |
| `vis-host-wasm`: `cargo check --target wasm32-wasip1` | **PASS** |
| `git diff --cached --check` | sạch (rc 0) |

Log đầy đủ (ngoài repo): `/home/ps/tmp/st310f-int2-logs/`:

| Tệp | sha256 |
|---|---|
| `gate-fast.log` | `7d7f19a14f4417c44a8b4d59bbc0eaef306af4ce7800351fc3f72ee3df54cba2` |
| `vis-targets.log` | `6e9e91bfd975253efc5261fe368e18734f21690062575301a411be4f917cfa97` |
| `diff-100k.log` | `6cbf9de6e47beced41b05304d2e6d6aeaa05fabb035a5d241b45755b581a21c4` |
| `cargo-check.log` | `80c710fcd6e2ee73035d9c71b7109cfa07e0ac955da2ab5685875a69ad0659a9` |
| `wasm-check.log` | `c8bf6b0d4d72f78b133e679de90e87ecb6b7bd93a6842f3d5f80b7ec2cc029b2` |

## 5. Còn lại / blocker

- Nhánh **local only**, chưa merge/push/tag; chờ A0/Gatekeeper nghiệm thu.
- Chưa có bằng chứng chạy thật 3-OS/macOS cho ST-309 (chỉ `cargo check` chéo + logic Darwin);
  không thay thế Tier 1.
- `#194` vẫn **mở**; `#195` vẫn **đề xuất** (chưa cấp số).
