<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# VS3-W4 — verify Windows `psvn` @ `v0.1.0-rc1` (`0985030`)

> **Người đo**: subagent VS3-W4 (chuyên trách `psvn`), 2026-11-21.
> **Phạm vi**: chỉ đo — **không** sửa mã sản phẩm, **không** đổi test/kỳ vọng, **không** cài dependency,
> **không** push/merge.
> **Kết luận ngắn**: **KHÔNG promote `v0.1.0`** tại `0985030`. Windows **không xanh**:
> `vis-dev check` đỏ ở clippy; `cargo test --workspace --no-fail-fast` đỏ **20 target / 38 ca**.
> Phần lớn là nợ **portability/test-only** của loạt test VD-270 mới, nhưng có **2 ca mức sản phẩm**
> liên quan ràng buộc `cap[Dir]` cần A0 triage. Host suite vẫn **54/54 PASS**.

## 0. TL;DR

| Bước | Lệnh | Kết quả | Mã thoát |
|---|---|---|---|
| Danh tính mốc | `git rev-parse HEAD` + `git describe --tags --exact-match` | `09850303…` = `v0.1.0-rc1`, cây sạch | 0 |
| Đường chuẩn Windows | `cargo test --workspace --no-fail-fast` | **20 target đỏ**; 3218 passed / **38 failed** / 3 ignored (329 block) | **101** |
| Cổng | `bin/vis-dev check` | fmt OK; **clippy FAIL** (`unused variable: marker`); dừng cổng | **1** |
| Hợp chuẩn host | `bin/vis-host-suite` | **54/54 PASS**; `host_posix_real` bỏ qua (Windows dùng `test-child`) | 0 |
| Ma trận target | `bin/vis-targets` | MSVC host 9/9 PASS; **mọi ô wasm32/Linux FAIL vì thiếu std của target** | 1 (BLOCKED-ENV) |

## 1. Mốc đo và danh tính

| Mục | Giá trị |
|---|---|
| Repo trên `psvn` | `/c/Users/ps/viscript-moi` |
| HEAD trước khi đo | `6cf99360824d0badc0ec5310217ca2193715f732` (`main`, cây sạch) |
| HEAD được đo | `098503032e8d7fb29de8448d306db88174ac1f4c` (detached) |
| Tag khớp | `v0.1.0-rc1` → đúng `09850303…` (`git describe --exact-match` = `v0.1.0-rc1`) |
| Chủ đề commit | `[VD-270] report: QA cuoi dot A0 (gate/core/nightly)` |
| Trạng thái cây lúc đo | `git status --porcelain` rỗng (0 mục), trước **và** sau toàn bộ lần đo |
| Fetch | `6cf99360..dc40c2cb main -> origin/main`; tag mới `v0.1.0-rc1` |

Không có thay đổi cục bộ nào trên `psvn` bị mất: cây sạch trước khi `git switch --detach`, và vẫn sạch sau khi chạy.

## 2. Môi trường đo

| Mục | Giá trị |
|---|---|
| Máy | `psvn` (Windows thật, Git Bash) |
| OS/kernel | `MINGW64_NT-10.0-26300 3.5.4-395fda67.x86_64 x86_64` |
| CPU | 6 |
| cargo | `1.100.0 (3acc79c0c 2026-11-05)` tại `/c/Users/ps/.cargo/bin/cargo` |
| rustc | `1.100.0 (0ef1bc386 2026-11-09)` |
| Target cài sẵn | **chỉ** `x86_64-pc-windows-msvc` (không có `wasm32-unknown-unknown`, không có `x86_64-unknown-linux-gnu`) |
| Ổ đĩa | `C:` còn 208G |
| Build cache | `target/` ~3.0G có sẵn |

## 3. Lệnh, kết quả, log

### 3.1. Fetch + checkout đúng mốc

```bash
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-fetch.log -- \
  bash -lc 'cd /c/Users/ps/viscript-moi && git status --porcelain && git fetch --tags origin && git switch --detach 0985030 && git log -1 --oneline'
```

- Cây sạch (công cụ in `# ST-175: cay SACH`), fetch OK, checkout OK.
- `HEAD is now at 09850303 [VD-270] report: QA cuoi dot A0 (gate/core/nightly)`.
- Exit 0. Log: `/tmp/opencode/psvn-rc1-fetch.log` (sha256 `54e5b429…23bd`),
  log trên máy đích `/c/Users/ps/vislogs/vis-remote-20261121T024028Z.log`.

### 3.2. Đường chuẩn Windows — `cargo test --workspace --no-fail-fast`

```bash
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-cargo.log -- \
  bash -lc 'cd /c/Users/ps/viscript-moi && date -u +START=%FT%TZ && cargo test --workspace --no-fail-fast'
```

- Thời gian: `02:43:00Z` → `02:48:59Z` (~6 phút, có cache).
- **Exit 101.** `error: 20 targets failed` (danh sách đầy đủ trong log, dòng ~5900).
- Tổng hợp từ 329 block `test result:`:

| Chỉ số | Số |
|---|---|
| Block test (unit + integration + doc) | 329 |
| Block xanh | 309 |
| Block đỏ | **20** |
| Test passed | **3218** |
| Test failed | **38** |
| Test ignored | 3 |

- Log: `/tmp/opencode/psvn-rc1-cargo.log` (sha256 `775499a8…486d`);
  log đích `/c/Users/ps/vislogs/vis-remote-20261121T024258Z.log`.

### 3.3. Cổng — `bin/vis-dev check`

```bash
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-visdev-check.log -- \
  bash -lc 'cd /c/Users/ps/viscript-moi && date -u +START=%FT%TZ && bin/vis-dev check'
```

- `fmt: OK`.
- **`clippy: FAIL`** — cổng dừng ngay, các bước sau (R1..R6, toolchain, release, fixtures,
  `cargo build`, doc-examples, readme-examples) **không chạy**:
  ```text
  error: unused variable: `marker`
    --> crates\vis-cli\tests\vd270_vistest_timeout.rs:75:23
     |
  75 | fn don_con(_pid: u32, marker: &str) {
     |                       ^^^^^^ help: if this is intentional, prefix it with an underscore: `_marker`
     |
     = note: `-D unused-variables` implied by `-D warnings`
  error: could not compile `vis-cli` (test "vd270_vistest_timeout") due to 1 previous error
  ```
- Nguyên nhân: `marker` chỉ được dùng trong khối `#[cfg(unix)]` (`pkill -f <marker>`); khối
  `#[cfg(windows)]` dùng `taskkill /T` và không dùng `marker`. Clippy chạy `-D warnings` nên Windows đỏ.
  Đây là **lỗi test-only** (helper test), nhưng là **cổng cứng** — không thể coi Windows xanh.
- **Exit 1.** Log: `/tmp/opencode/psvn-rc1-visdev-check.log` (sha256 `784e25f9…33c8`);
  log đích `/c/Users/ps/vislogs/vis-remote-20261121T025022Z.log`.

### 3.4. Hợp chuẩn host — `bin/vis-host-suite`

```bash
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-host-suite.log -- \
  bash -lc 'cd /c/Users/ps/viscript-moi && date -u +START=%FT%TZ && bin/vis-host-suite'
```

- **`54/54 ca xanh`** (nền `MINGW64_NT-10.0-26300 x86_64`).
- `host_posix_real: BỎ QUA (Windows dùng test-child)` — đúng thiết kế VD-259.
- **Exit 0.** Log: `/tmp/opencode/psvn-rc1-host-suite.log` (sha256 `3da64eae…e1d`);
  log đích `/c/Users/ps/vislogs/vis-remote-20261121T025116Z.log`.

### 3.5. Ma trận target — `bin/vis-targets` (BLOCKED-ENV)

```bash
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-targets.log -- \
  bash -lc 'cd /c/Users/ps/viscript-moi && date -u +START=%FT%TZ && bin/vis-targets'
```

- `core`: 8 crate × MSVC **8/8 PASS**; `vis-diag × wasm32` PASS (không cần std); còn lại wasm32/Linux
  **9 FAIL** — tất cả `error[E0463]: can't find crate for std/core`.
- `sqlite`: 8 crate × MSVC **8/8 PASS** (ô `vis-cli × msvc` từng `EXPECTED_RED` nay xanh — script nhắc
  rà soát nợ #156); `vis-diag × wasm32` PASS; wasm32/Linux **8 FAIL** cùng lý do thiếu std target.
- Nguyên nhân: `rustup target list --installed` trên `psvn` chỉ có `x86_64-pc-windows-msvc`.
- **Không cài target** (theo lệnh cấm cài dependency) ⇒ đây là **BLOCKED-ENV**, không phải lỗi sản phẩm.
- **Exit 1.** Log: `/tmp/opencode/psvn-rc1-targets.log` (sha256 `f79d2d4f…6539`);
  log đích `/c/Users/ps/vislogs/vis-remote-20261121T025218Z.log`.

## 4. Phân loại 38 ca đỏ

Phân loại theo yêu cầu: **product** (hành vi sản phẩm sai/gap), **portability** (sản phẩm đúng theo
nền nhưng test/kỳ vọng viết cho POSIX), **test-only** (fixture/kỳ vọng của test sai, sản phẩm đúng),
**BLOCKED-ENV** (thiếu môi trường).

| # | Target | Ca đỏ | Bằng chứng chính | Loại |
|---:|---|---|---|---|
| 1 | `cheatsheet` | `cheatsheet_moi_vi_du_chay_duoc` | `09-stdlib.vis`: mong `a/b`, thực `a\b` | portability |
| 2 | `st240_std_cli` | `json_non_finite_va_path_goc` | mong `/\n/`, thực `\\\n\\` | portability |
| 3 | `stdlib` | `std_70_ca_qua_binary` | `path.dirname("/")` mong `/`, được `\`; `path.is_abs("/")` mong `true`, được `false` | portability |
| 4 | `targets` | `targets_wasm_vis_types` | `can't find crate for std` — chưa cài `wasm32-unknown-unknown` | **BLOCKED-ENV** |
| 5 | `vd270_batch_trigger` | 3 ca (`trigger_*`) | `VS0102: escape không hợp lệ` — fixture nhúng đường dẫn Windows chưa escape | test-only |
| 6 | `vd270_db_batch` | `migration_nhieu_cau_va_tach_an_toan` | cùng `VS0102` | test-only |
| 7 | `vd270_db_explain` | `plan_select_tham_so_va_with` | cùng `VS0102` | test-only |
| 8 | `vd270_fmt_list` | `tep_khong_doc_duoc_exit_2` | mong `No such file or directory`, được `The system cannot find the file specified` | test-only |
| 9 | `vd270_fmt_stat` | `tep_khong_doc_duoc_exit_2` | cùng lỗi chuỗi OS | test-only |
| 10 | `vd270_fs_canonicalize` | `quyen_ngoai_root_bi_chan` | mong exit 2 + `VS0604`; thực exit 0, stdout `ERR` | **product** (xem §5) |
| 11 | `vd270_fs_current_dir` | `goi_that_tra_tuyet_doi_va_ton_tai` | mong `\\?\C:\…`, được `C:\…` | portability |
| 12 | `vd270_fs_is_symlink` | `loi_metadata_la_err_khong_trap` | `tep.txt/con`: Linux `ENOTDIR` ⇒ `err`; Windows `NotFound` ⇒ `Ok(false)` | portability |
| 13 | `vd270_fs_read_link` | `dich_tuong_doi_tuyet_doi_va_dangling` | đích tuyệt đối: mong `C:/…`, được `C:\…` | portability |
| 14 | `vd270_fs_read_link` | `quyen_ngoai_root_bi_chan` | symlink ra ngoài root: mong exit 2 + `VS0604`; thực exit 0, stdout `OK` | **product** (xem §5) |
| 15 | `vd270_path_is_absolute` | 4 ca | kỳ vọng POSIX (`/` tuyệt đối, `\` ký tự thường) | portability |
| 16 | `vd270_path_is_relative` | 4 ca | cùng gốc POSIX | portability |
| 17 | `vd270_path_join_all` | 7 ca | `Path::join` trên Windows dùng `\`; test mong `/` | portability |
| 18 | `vd270_path_strip_prefix` | `probe_rust_path_strip_prefix` | test probe `Path::strip_prefix("//srv/share","//")` — Linux `Ok`, Windows `Err` | test-only |
| 19 | `vd270_sqlite_database_list` | 2 ca | `VS0102: escape không hợp lệ` (đường dẫn Windows trong fixture) | test-only |
| 20 | `vd270_with_stem` | `khong_co_phan_mo_rong` | Windows coi `a\c` là thư mục `a` + tệp `c`; `with_stem` giữ `a\c`, test mong `c` | portability |
| 21 | `vis-interp --lib` | 3 ca `pathisabs::*`, `pathrelbool::*` | unit test khẳng định `/` tuyệt đối trên Windows | portability |

**Tổng theo loại**: product **2**, BLOCKED-ENV **1**, test-only **10**, portability **25** (2+1+10+25 = 38).

Ghi chú về gốc portability: `std.path.is_absolute`/`is_relative`/`join`/`with_stem` **cố ý** truyền
`cfg!(windows)` vào `la_tuyet_doi`/`noi_duong` (xem `crates/vis-interp/src/stdlib/pathisabs.rs`,
`pathjoinall.rs`), tức hợp đồng là **theo nền**, không phải POSIX-cố-định. Các test VD-270 lại viết
kỳ vọng POSIX không có nhánh `cfg!(windows)`. Đây là nợ test, không phải lỗi runtime.

## 5. Phát hiện mức sản phẩm (cần A0 triage)

### 5.1. `fs.read_link` cho symlink trỏ ra ngoài `cap[Dir]` — **không** chặn

- Ca: `vd270_fs_read_link::quyen_ngoai_root_bi_chan`.
- Dựng: `cap[Dir] @ p"<root>"`; tạo symlink `<root>/link-ra-ngoai` → `../ngoai.txt` (đích **ngoài** root).
- Chương trình: `fs.read_link("<root>/link-ra-ngoai")`.
- Kỳ vọng (hợp đồng ghi trong test + `quyen.rs::read_link`): kiểm quyền canonicalize **trước**,
  symlink trỏ ngoài ⇒ `VS0604`, exit 2, không in kết quả.
- **Thực tế**: exit **0**, stdout `OK`, stderr rỗng. Tức `read_link` trả `Ok` cho đích ngoài root.
- Đây là **vượt ràng buộc cap** (đọc *đích* symlink ngoài root). Chưa xác định được đây là
  guard bỏ sót hay `canonicalize` trên Windows rơi vào nhánh dự phòng; **không sửa mã** theo phạm vi.
- Mức độ: cần A0 đánh giá — tối thiểu phải sửa hoặc ghi rõ Windows Tier 2 **không** bao gồm
  ràng buộc `cap[Dir]` chặt như Linux.

### 5.2. `fs.canonicalize` ngoài root — bị từ chối nhưng **sai mã**

- Ca: `vd270_fs_canonicalize::quyen_ngoai_root_bi_chan`.
- Cùng cách dựng; kỳ vọng exit 2 + `VS0604`.
- **Thực tế**: exit 0, stdout `ERR`, stderr rỗng — thao tác không trả đường dẫn ngoài (không lộ dữ liệu),
  nhưng lỗi được trả như `Err(Error)` dữ liệu thay vì trap quyền `VS0604`.
- Mức độ thấp hơn 5.1 (fail-closed), nhưng lệch hợp đồng chẩn đoán.

## 6. Kết luận

**Không promote `v0.1.0` tại `0985030`.**

Lý do theo đúng điều kiện A0 (VD-271: “`v0.1.0` sau khi Windows Tier 2 được verify lại”):

1. `cargo test --workspace --no-fail-fast` **đỏ**: 20/329 target, 38 ca — không đạt “workspace xanh”.
2. `bin/vis-dev check` **đỏ** ở clippy — cổng bắt buộc của dòng ViScript không qua trên Windows.
3. Có **2 ca mức sản phẩm** về `cap[Dir]` (5.1, 5.2) chưa được phân xử.

Đồng thời ghi nhận mặt xanh, để không đánh giá quá mức độ hỏng:

- **3218 test pass**, **309/329 block xanh**, gồm toàn bộ ngôn ngữ/VM/sem/LSP hiện có.
- **Host suite 54/54 PASS** trên Windows thật.
- 35/38 ca đỏ là **portability/test-only** của loạt test VD-270 mới; sửa test (không sửa sản phẩm)
  là hướng chính.
- `vis-targets` **không kết luận được** trên `psvn` vì thiếu std target — cần cài target (có phê duyệt)
  hoặc loại `vis-targets` khỏi phạm vi đo Windows.

**Khuyến nghị**:

1. Giữ `v0.1.0-rc1` làm baseline VS3 **chỉ khi** A0 ghi rõ “chấp nhận rc1 dù Windows chưa verify lại”.
2. Mở việc sửa nợ Windows (test-only/portability) trước khi cắt `v0.1.0`:
   - `vd270_vistest_timeout.rs`: `marker` chỉ dùng ở `cfg(unix)` ⇒ clippy Windows đỏ.
   - Nhóm path: thêm nhánh kỳ vọng `cfg!(windows)` (hoặc so với `std::path::Path` của host) cho
     `path_is_absolute/is_relative/join_all/with_stem`, `stdlib`/`cheatsheet`/`st240`, unit test
     `pathisabs`/`pathrelbool`.
   - Fixture có đường dẫn Windows: escape `\` trước khi nhúng vào mã ViScript
     (`batch_trigger`, `db_batch`, `db_explain`, `sqlite_database_list`).
   - Chuỗi lỗi OS: so theo `ErrorKind`/mã, không so nguyên văn thông điệp Windows.
3. Triage 5.1/5.2 ở mức sản phẩm; nếu chưa sửa, ghi rõ giới hạn Windows Tier 2.
4. Sau khi sửa, chạy lại **đúng** quy trình này trên `0985030`+ (hoặc commit mới) trước khi promote.

## 7. Tái lập

```bash
# 1) mốc
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-fetch.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && git status --porcelain && git fetch --tags origin && git switch --detach 0985030 && git log -1 --oneline'

# 2) đường chuẩn Windows (KHÔNG nextest)
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-cargo.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && cargo test --workspace --no-fail-fast'

# 3) cổng + hợp chuẩn host
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-visdev-check.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-dev check'
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-host-suite.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-host-suite'

# 4) ma trận target (cần wasm32 + x86_64-unknown-linux-gnu trên psvn)
bin/vis-remote psvn --log /tmp/opencode/psvn-rc1-targets.log -- bash -lc \
  'cd /c/Users/ps/viscript-moi && bin/vis-targets'
```

## 8. Bằng chứng

Log cục bộ (ngoài repo):

| Tệp | sha256 | Exit |
|---|---|---|
| `/tmp/opencode/psvn-rc1-fetch.log` | `54e5b429a6529924285799dbb5e6d5ea7ff182719a1bc3e4595bfbac56eb23bd` | 0 |
| `/tmp/opencode/psvn-rc1-cargo.log` | `775499a8ef2618ec3472a8c3f6985a151ac5473ec9370369703f98b8ffbf486d` | 101 |
| `/tmp/opencode/psvn-rc1-visdev-check.log` | `784e25f9e3b1b0bfd1653d5ad01bc5f27f590fa714ba4ffe64e423e4ea9e33c8` | 1 |
| `/tmp/opencode/psvn-rc1-host-suite.log` | `3da64eae15ade6f2f018eb6e39ca46552bb5193d45664fc76ee377401f281e1d` | 0 |
| `/tmp/opencode/psvn-rc1-targets.log` | `f79d2d4f82191d518eac2d9cb6e447da4a1849ec9d74f261bae03d5e15b06539` | 1 |

Log trên máy đích: `/c/Users/ps/vislogs/vis-remote-20261121T024028Z.log` (fetch),
`…T024258Z.log` (cargo test), `…T025022Z.log` (vis-dev check), `…T025116Z.log` (host suite),
`…T025218Z.log` (targets), `…T025321Z.log` (trạng thái cuối).

Trạng thái cuối trên `psvn`: HEAD `09850303…`, tag `v0.1.0-rc1`, `git status --porcelain` = 0 mục.
