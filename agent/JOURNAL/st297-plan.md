<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
# ST-297 plan — rà `PATH-SPLIT` còn lại (Windows/UNC/mixed-slash)

> **Người làm**: W3, 2026-11-23 · **nhánh** `plan/w3-st296b-crates` @ **`7c4c205`**
> (`ST-296b` đã xong; **không** merge `main`, không push/tag).
> **Phạm vi task**: **CHỈ ĐỌC + KẾ HOẠCH**, không sửa code/`SO-NO.md`/`QUEUE.md`.
> Nguồn cũ: `agent/PATH-SPLIT-AUDIT-2026-11-18.md` (gọi tắt **AUDIT-18**), `#149` trong
> `agent/SO-NO.md` §4. Đĩa cuối **15 G** (> 10 G).

## 1. Kết luận nhanh

- **`check.rs:191` của AUDIT-18 đã được ST-211 sửa**: hiện `crates/vis-cli/src/cli/check.rs:219`
  dùng `ten_goc(path)` (`rsplit(['/', '\\'])`, dòng 70–71), có unit test `ten_goc_du` phủ
  `C:\…`, `C:/…`, mixed `C:/…\…`, `/…`, tên có khoảng trắng, rỗng, đuôi `/`.
- **Trong 5 nhóm được giao, 0 chỗ tách OS path thủ công còn *sai***. Các hit thật:
  1. `fsmock.rs` tách `/` cho **filesystem ảo** của `--fs-mock` (không phải OS path).
  2. `vistest.rs:493` tách `/` cho cú pháp `--shard I/N` (không phải path).
  3. `check.rs:71` tách **cả** `/` và `\` (đã đúng).
- **Phát hiện mới trong phạm vi**: LSP chuyển **file URI ↔ path** còn thủ công và **chưa có UNC**;
  `workspace/symbol` + `workspace/definition` dựng URI bằng `format!("file://{}", p.display())`
  (sai trên Windows, không percent-encode). `--grant cap[Proc]` phân loại path bằng
  `contains('/')` (path `C:\…` bị coi là tên trần ⇒ dò `PATH`).
- **`#149`**: `check.rs` coi như **đã đóng**; đề nghị **giữ mở** cho LSP URI nếu A0 đồng ý mở
  rộng định nghĩa, còn `workspace.rs` + `quyen_path.rs` nên là **nợ mới** (chi tiết §4).
- **`std.path`** (`crates/vis-interp/src/stdlib.rs`, AUDIT-18 §3.1) **ngoài danh sách crate của
  task này** (`vis-interp` không nằm trong `vis-cli`/`vis-loader`/`vis-lsp`/`vis-host-*`/
  `vis-types`) — vẫn thuộc `#149` nhưng để task riêng, không đưa vào thứ tự dưới.

## 2. Cách rà (tái lập)

Đã chạy trên cây hiện tại (chỉ `src/` + `tests/` của 5 nhóm):

```bash
D="crates/vis-cli/src crates/vis-loader/src crates/vis-lsp/src crates/vis-types/src vis-host-linux vis-host-wasm"
grep -rnF ".split('/')" $D --include=*.rs
grep -rnF ".rsplit('/')" $D --include=*.rs
grep -rnF ".split_once('/')" $D --include=*.rs
grep -rnF ".rsplit_once('/')" $D --include=*.rs
grep -rnF "['/', '\\\\']" $D --include=*.rs
grep -rnF "replace('\\\\'" $D --include=*.rs
grep -rnF "starts_with('/')" $D --include=*.rs
grep -rnF "ends_with('/')" $D --include=*.rs
grep -rnF "trim_end_matches('/')" $D --include=*.rs
grep -rnF '.join("/")' $D --include=*.rs
```

Kết quả thô trên `src/` (đã đối chiếu tay từng hit; test helper không tính):
`split('/')` = **1** (`fsmock`); `rsplit('/')` = **0**; `split_once('/')` = **1** (`vistest`
`--shard`, không phải path); `rsplit_once('/')` = **0**; `['/', '\\']` = **1** (`check::ten_goc`,
đúng); `replace('\\', "/")` = **6** (3 ở `modules`, 2 ở LSP URI, 1 escape JSON ở
`vis-types/lower/cmd.rs`); `starts_with('/')` = **3** (`fsmock` 1, LSP URI 2);
`ends_with('/')` = **2** (`tar_b64` tar spec, `fsmock`); `trim_end_matches('/')` = **4**
(2 URL registry, `pkg` URL, `fsmock`); `.join("/")` = **2** (`modules` module path, không phải OS).
**0 hit**: `split('\\')`, `rsplit('/')`, `rsplit("...")`, `splitn('/')`, `Path::from(s.split(...))`.

## 3. Danh sách điểm (9 điểm rà soát, 3 điểm cần sửa code)

| # | Vị trí | Hiện trạng / rủi ro | Phân loại | Sửa? |
|---|---|---|---|---|
| 1 | `crates/vis-lsp/src/lib.rs:787-824` (`uri_thanh_duong_voi`) | `strip_prefix("file://")`; nhận drive `C:`/`C:\`, `%5C`, `%3A`; **không nhận UNC** `file://server/share/a.vis` (authority bị coi là path tương đối ⇒ `None`). Mixed slash chỉ xử lý khi có drive, không có test UNC. | **#149 (đề nghị mở rộng)** — chuyển URI→path thủ công | **Có** |
| 2 | `crates/vis-lsp/src/workspace.rs:99,145` | `format!("file://{}", p.display())` cho `workspace/symbol` + `definition`: Windows `C:\…` ⇒ `file://C:\…` (thiếu `/`, sai dấu); UNC ⇒ `file://\\server\…`; khoảng trắng/UTF-8 không percent-encode. | **Nợ mới** (path→URI, không phải tách path) | **Có** |
| 3 | `crates/vis-cli/src/quyen_path.rs:67` (`phan_giai_lenh`) | `if !v.contains('/')` ⇒ dò `PATH`. Trên Windows `C:\…\x.exe` / `\\server\…\x.exe` không có `/` ⇒ dò PATH sai; `C:/…` lại đúng. AUDIT-18 §3.3 xếp “thấp, giữ”, nhưng đây là lỗi Windows thật. | **Nợ mới** (phân loại path, không tách) | **Có** |
| 4 | `crates/vis-cli/src/cli/check.rs:70-71,219` | `ten_goc` tách **cả** `/` và `\`; `check_all_loaded` fallback dùng `ten_goc`. Đã đúng từ ST-211. | **#149 — đã đóng phần code** | Không; chỉ thêm test `--json --syntax` |
| 5 | `crates/vis-cli/src/fsmock.rs:242-264` (`mkdir_all`) | `starts_with('/')` + `split('/')` + `ends_with('/')`: filesystem ảo mô phỏng POSIX, không đụng OS path; `\` là ký tự thường. | Không thuộc `#149` (virtual FS) | Không; test tài liệu hoá |
| 6 | `crates/vis-loader/src/modules.rs:186,189,211` | `replace('\\', "/")` khi dựng `FileUnit.rel`/`pkg/<tên>/<ver>/…` — **chuẩn hoá** sang `/` cho cache/diagnostics, không tách. | Không thuộc `#149` | Không; test mixed/Unicode nếu chạm |
| 7 | `crates/vis-loader/src/modules.rs:249,446,570` | `.join("/")`/`contains('/')` cho **đường dẫn ngôn ngữ** (`use ./a/b`, dotted path) — không phải OS path. | Không thuộc `#149` | Không |
| 8 | `crates/vis-loader/src/lock_meta.rs:64` (`kiem_path`) | Từ chối `\`, `..`, tuyệt đối: path trong `vis.lock`/tar theo spec là **tương đối, `/`**. Đúng có chủ đích. | Không thuộc `#149` | Không; thêm test reject `src\main.vis` |
| 9 | `crates/vis-types/src/lower/khai_bao.rs:110`; `crates/vis-cli/src/vistest.rs:493`; `vis-host-*` | `contains('/')` cho `use` specifier; `split_once('/')` cho `--shard I/N`; `vis-host-*` **0 hit** path split; `vis-types` **0** OS path. | Không thuộc `#149` | Không |

**Điểm cần sửa code = 3** (#1, #2, #3). **Điểm test/verify = 1** (#4). **Điểm giữ nguyên có
chủ đích = 5** (#5–#9). Ngoài bảng: `std.path` của `vis-interp` (AUDIT-18 §3.1) vẫn là nợ `#149`
riêng, **không** sửa trong ST-297.

## 4. `#149` — đề xuất chốt phạm vi

- **Đã xong**: `check.rs` (ST-211, `ten_goc`) — không còn `rsplit('/')` chỉ tách `/`.
- **Đề nghị đưa vào `#149`**: #1 LSP `uri_thanh_duong_voi` (vì đang tự chuẩn hoá `\`/`/` cho
  đường dẫn tệp; mở rộng định nghĩa “path split” sang URI file).
- **Đề nghị nợ mới (không gộp `#149`)**: #2 `workspace.rs` path→URI (serialize URI, không tách);
  #3 `quyen_path.rs` phân loại path `--grant` (không tách). Nếu A0 muốn một đầu mối, có thể gộp
  cả 3 vào `#149` và ghi rõ “mở rộng: URI + phân loại path”.
- **Không sửa** `SO-NO.md`/`QUEUE.md` trong task này (theo lệnh); chỉ ghi đề xuất ở đây.

## 5. Test Windows/UNC/mixed-slash cần thêm

### 5.1 `vis-lsp` — URI → path (unit, `uri_thanh_duong_voi` trong `lib.rs`)

- `file://server/share/a.vis` (Windows) ⇒ `//server/share/a.vis` — **ca đỏ hiện tại**.
- `file:////server/share/a.vis` ⇒ `//server/share/a.vis`.
- `file://server/share/a%20b/%C3%A9.vis` ⇒ `//server/share/a b/é.vis`.
- `file:///C:/a\b.vis`, `file:///C%3A%5Ca%5Cb.vis`, `file:///c:/a/b.vis` (đã có, giữ).
- mixed `file:///C:/a\b/c.vis` ⇒ `C:/a/b/c.vis`.
- `windows=false`: `file://server/share/a.vis` ⇒ `None` (authority không phải path cục bộ);
  `file:////server/share/a.vis` ⇒ `//server/share/a.vis` (POSIX `//` giữ nguyên).
- `%2F`/`%5C`/`%25` + space/UTF-8; URI sai (`http://`, `file://`) ⇒ `None`, không panic.

### 5.2 `vis-lsp` — path → URI (unit, helper mới `duong_thanh_uri`, có cờ `windows`)

- POSIX `/tmp/a b/é.vis` ⇒ `file:///tmp/a%20b/%C3%A9.vis`.
- Windows `C:\a b\é.vis` ⇒ `file:///C:/a%20b/%C3%A9.vis` (**ca đỏ hiện tại**).
- UNC `\\server\share\a b.vis` ⇒ `file://server/share/a%20b.vis`.
- `#`, `?`, `%` phải encode; `/` trong path giữ; drive `:` giữ.
- Round-trip `uri_thanh_duong_voi(duong_thanh_uri(p, windows), windows) == p` cho POSIX/Windows/UNC.
- Tích hợp `workspace/symbol` + `definition`: root có khoảng trắng/Unicode; assert `location.uri`
  bắt đầu `file:///` (POSIX) hoặc `file:///C:/` (Windows), không có `\`.

### 5.3 `vis-cli` — `check --json --syntax` + `.vish` fallback

- Trên Linux: `.vish` có lỗi cú pháp + lỗi kiểu; `--json --syntax` phải giữ đúng chẩn đoán
  syntax (không rò chẩn đoán kiểu), `files[0].file` là tên gốc.
- `#[cfg(windows)]`: gọi với `C:\…\a.vish`, `C:/…/a.vish`, mixed `C:/…\a.vish`; assert cùng
  `file`/`diagnostics` như đường `/` (khoá fallback `ten_goc`).
- Unit (đã có) `ten_goc_du`; bổ sung `\\server\share\a.vish` ⇒ `a.vish` (UNC hiện chưa có ca).

### 5.4 `vis-cli` — `--grant cap[Proc]`

- `#[cfg(windows)]`: copy `test-child` vào temp; `--grant p=C:\…\test-child.exe` chạy được;
  `--grant p=\\server\share\…` chạy được (nếu môi trường có UNC) hoặc báo lỗi “không phân giải
  được” **không** phải “không thấy trong PATH”.
- Unix: tên `a\b` vẫn là **một** tên tệp (không đổi hành vi); `--grant p=./a\b` xử lý như path
  tương đối nếu quyết định sửa `contains('/')`.
- `C:/…` (forward slash) phải giữ nguyên hành vi hiện tại.

### 5.5 `vis-loader` — giữ nguyên có chủ đích

- `kiem_path("src\\main.vis")` ⇒ `Err`; `kiem_path("src/main.vis")` ⇒ `Ok` (lock/tar spec `/`).
- `FileUnit.rel` của module lồng nhau luôn dùng `/` trên Windows (chạy psvn); tên có khoảng
  trắng/Unicode.
- `mkdir_all("a\\b")` (fsmock) là **một** thành phần tên `a\b`, không tạo `a/b` (tài liệu hoá
  POSIX-only; không sửa).

## 6. Thứ tự thực hiện + ước lượng

| Bước | Việc | Ước lượng | Ghi chú |
|---|---|---|---|
| 0 | Thêm test đỏ (5.1 UNC, 5.2 Windows URI, 5.4 `C:\`) **trước** khi sửa; giữ test cũ xanh | **0,75 h** | RED-BEFORE, không đổi hành vi |
| 1 | #1 `vis-lsp/src/lib.rs`: UNC authority + mixed slash trong `uri_thanh_duong_voi`; chạy 5.1 | **1 h** | Ưu tiên vì LSP là cửa vào Windows |
| 2 | #2 `vis-lsp/src/workspace.rs`: helper `duong_thanh_uri` (percent-encode, drive, UNC), thay 2 call site; 5.2 | **1,25 h** | Dùng chung với #1 để round-trip |
| 3 | #3 `quyen_path.rs`: nhận diện `\`/drive/UNC là path (không dò PATH); 5.4 | **1 h** | Cẩn thận Unix `\` là ký tự thường |
| 4 | #4 test `check --json --syntax` + UNC `ten_goc`; 5.3 | **0,5 h** | Không sửa code |
| 5 | 5.5 test loader/fsmock; chạy `cargo test` + `./vis-dev gate --fast` + psvn cho 5.1–5.4 | **0,75 h** | Cần psvn cho UNC/Windows thật |
| | **Tổng** | **~5,25 h** (1 ngày) | Nếu chỉ #1+#2: ~3 h |

Thứ tự đề xuất: **0 → 1 → 2 → 3 → 4 → 5**. Lý do: test đỏ trước; LSP URI là nơi Windows/UNC
ảnh hưởng nhiều nhất; `workspace` dùng chung helper; `--grant` độc lập; test `check` chỉ xác nhận.

## 7. Rủi ro / blocker

- **Không blocker kỹ thuật** để lập kế hoạch. Cần **psvn** để chạy ca `#[cfg(windows)]`/UNC thật;
  trên Linux chỉ mô phỏng được qua cờ `windows: bool` (giống `uri_thanh_duong_voi` hiện có).
- UNC chuẩn LSP là `file://server/share/path` (authority), còn `file:////server/share` là path
  `//server/share`; cần chốt cả hai để không phá POSIX.
- `--grant` trên Unix: `\` là ký tự tên tệp hợp lệ, nên fix phải **có điều kiện nền tảng** hoặc
  dùng `Path`/`components` thay vì chỉ thêm `contains('\\')` toàn cục.
- `std.path` của `vis-interp` vẫn ngoài phạm vi; nếu A0 muốn đóng `#149` hoàn toàn phải mở task
  riêng cho `noi_duong`/`basename`/`dirname`/`ext`/`stem`.
- Không sửa code/`SO-NO`/`QUEUE`; không push/merge/tag; đĩa cuối **15 G**.
