# ST-132b · Ca âm Landlock thứ ba — tiến trình con `read_dir` thư mục ngoài (2026-11-16)

Người làm: Agent 2 dòng ViScript. Căn cứ: `KE-HOACH-CHOT-23f.md` §2 **ST-132b**, **VD-138**
("bằng chứng Landlock của ST-132 tính cho hạng mục 4 sau khi đủ **3** ca âm ở tiến trình con"),
**VD-120** (ca âm phải chứng minh **kernel** chặn, không phải `FsGuard`), **VD-136** (log môi trường).
Runner: `bin/vis-linux` backend `ssh` → host `ps`.

## 0. Kết luận một dòng

**13/13 ca xanh trên `ps`** (12 ca cũ + 1 ca mới). Ca mới
`con_read_dir_ngoai_pham_vi_bi_kernel_chan` cho tiến trình con `read_dir` thư mục **ngoài** phạm vi ⇒
`EACCES`/`Permission denied` (kernel chặn ở `openat` thư mục, helper **không** đi qua `FsGuard`), có
**đối chứng dương** (`read_dir` **trong** phạm vi liệt kê được, cùng ca) và **đỏ-trước** (cùng helper,
**không** áp ruleset ⇒ liệt kê được thư mục ngoài). Đủ **3** ca âm ở tiến trình con theo VD-138.

## 1. Đã làm

| Tệp | Việc | Dòng |
|---|---|---|
| `vis-host-linux/tests/landlock.rs` | thêm ca **13** `con_read_dir_ngoai_pham_vi_bi_kernel_chan`; helper tĩnh thêm chế độ `ls <thư mục>` (`fs::read_dir`, in tên đã sắp xếp) | 393 dòng/tệp (≤1.500) |

Không sửa sản phẩm, không sửa ca cũ, không `#[ignore]`, không `cfg` bỏ ca, không `unsafe`, không
crate mới. `--sandbox` có sẵn `ReadDir` trong `chi_doc()`/`handle_access(toan_bo())` nên **không** phải
nới ruleset để ca xanh.

## 2. Bằng chứng (log + sha256)

- Log đầy đủ (đỏ-trước + 13 ca + môi trường): `JOURNAL/st132b-do-truoc.log`
  **sha256 `24502ed04a95b23514d7f333e7072549c2547f95f1f6baa1a05bfa410f90d65f`** (24 502 byte).
- Tệp đo: `vis-host-linux/tests/landlock.rs`
  **sha256 `fa570a60345027da6b9a84511765962d0ab6c14b2aadb632b3e1cd678e0e3e30`**
  (đo trên cây chưa commit; commit `[ST-132b]` chỉ chứa đúng tệp này + JOURNAL + QUEUE).
- Commit đang đo: **`926022e4`** (HEAD lúc đo).

### 2.1 Đỏ-trước (A trong log)

Cùng helper tĩnh (`crt-static`), **không** áp ruleset:

```
=== KHÔNG áp Landlock ruleset (đỏ-trước): cùng helper liệt kê thư mục NGOÀI ===
bi-mat.txt
EXIT=0
sha256 helper: c389d1cdd27d70b9aadcd8cd60d8d43a1e796d33b3e6048b227019ccd925e640
```

Nghĩa: nếu **không** có ruleset thì `read_dir` thư mục ngoài **được**; nên `EACCES` ở lần chạy sau là
do **kernel** (Landlock), không phải do quyền POSIX/thư mục. Trong chính ca kiểm, dòng này được in lại
bằng `println!` và khẳng định `code=0` + thấy `bi-mat.txt` trước khi gọi `ap(...)`:

```
ĐỎ-TRƯỚC (không áp ruleset): exit=Some(0) stdout="bi-mat.txt"
```

### 2.2 Mười ba ca (B trong log)

```
running 13 tests
test duong_dan_khong_ton_tai_la_loi_he_thong ... ok
test ghi_ngoai_bi_chan ... ok
test doc_trong_thu_muc_duoc_phep ... ok
test luat_cho_mot_tep_khong_mo_ca_thu_muc ... ok
test khong_co_luat_nao_thi_chan_het ... ok
test ghi_trong_thu_muc_duoc_phep ... ok
test doc_ngoai_bi_kernel_chan ... ok
test ghi_trong_thu_muc_chi_doc_bi_chan ... ok
test nhieu_thu_muc_duoc_cap ... ok
ĐỎ-TRƯỚC (không áp ruleset): exit=Some(0) stdout="bi-mat.txt"
test con_read_dir_ngoai_pham_vi_bi_kernel_chan ... ok
test con_doc_trong_pham_vi_chay_duoc ... ok
test con_doc_ngoai_pham_vi_bi_kernel_chan ... ok
test con_ghi_ngoai_pham_vi_bi_kernel_chan ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
```

## 3. Ca mới khẳng định gì (đọc kỹ để không nói quá)

`con_read_dir_ngoai_pham_vi_bi_kernel_chan`:

1. **Đỏ-trước**: chạy helper `ls <ngoai>` **trước** `ap(...)` ⇒ `code=0`, stdout có `bi-mat.txt`.
2. `ap(&[ChoPhep { duong: trong, ghi: false, la_tep: false }])` — ruleset chỉ đọc trên `trong`.
3. Chạy helper `ls <ngoai>` ⇒ `code=7` (helper tự thoát 7 khi `std::io::Error`) + stderr chứa
   `Permission denied` ⇒ `PermissionDenied` = `EACCES` **ở `openat` thư mục**; helper gọi thẳng
   `std::fs::read_dir`, **không** có `FsGuard` nào ⇒ **kernel** chặn (VD-120).
4. **Đối chứng dương cùng ca**: helper `ls <trong>` ⇒ `code=0`, stdout có `duoc.txt` — không phải
   "xanh vì chặn tất".
5. Dọn cả hai thư mục tạm.

Chạy bằng `root` trên `ps`: quyền POSIX (DAC) không chặn `root`; `EACCES` chỉ còn giải thích bằng
**Landlock ruleset** — thêm một lớp chứng minh "kernel chứ không phải quyền tệp".

## 4. Log môi trường (VD-136, in ở đầu lần đo)

```
host: ps
Linux 7.0.0-43-generic x86_64
os: Ubuntu 26.04.1 LTS
git HEAD: 926022e4e87c9bc88215cd2294a27b367b263b6f
git status (vis-host-linux):  M vis-host-linux/tests/landlock.rs;
sha256 vis-host-linux/tests/landlock.rs: fa570a60345027da6b9a84511765962d0ab6c14b2aadb632b3e1cd678e0e3e30
rustc 1.99.1 (58cf8f9c3 2026-10-13)
host: x86_64-unknown-linux-gnu
release: 1.99.1
rustup target list --installed: x86_64-unknown-linux-gnu
```

## 5. Lệnh tái lập

```bash
# trên runner ps (bin/vis-linux đồng bộ cây rồi chạy)
bash viscript/bin/vis-linux run bash -c 'cd vis-host-linux && cargo test -- --nocapture'
```

## 6. Hai nền

| Nền | Kết quả | Ghi chú |
|---|---|---|
| Linux (`ps`, kernel 7.0.0-43-generic, Landlock ABI 8) | **13/13 ca xanh** (12 cũ + 1 mới) | ca mới: `read_dir` ngoài ⇒ `Permission denied`; trong ⇒ liệt kê được |
| macOS (máy dev) | **không áp dụng**: crate `vis-host-linux` chỉ biên dịch trên Linux (`landlock = 0.4.7` dùng `libc::prctl` của Linux) | **không** phải bỏ ca: tệp đã có `#![cfg(target_os = "linux")]` từ ST-132; macOS đo `--sandbox` = exit 3 ở `crates/vis-cli/tests/sandbox.rs` (ST-134) |

## 7. Nợ mới (đánh số tiếp sau 131)

- **132** — ba ca âm tiến trình con dựng helper bằng `rustc` ngay trong ca; nếu runner thiếu
  `target-feature=+crt-static`/`rustc` thì ca đỏ vì **môi trường**, không phải sản phẩm (thông điệp
  `rustc phải dựng được helper tĩnh` đã phân biệt được). Chưa có bước `setup` kiểm `rustc` tĩnh.
- **133** — ca mới chỉ kiểm `read_dir` với luật **chỉ đọc** (`ghi: false`); chưa có ca `read_dir`
  trên thư mục được cấp **quyền ghi** để khoá hành vi `READ_DIR|WRITE` cùng lúc.

## 8. Chỗ lệch kế hoạch

- Kế hoạch 23f ghi "thêm **một** ca" ⇒ đã làm đúng **một** `#[test]`; nhưng để chứng minh
  `read_dir` (không phải `read` tệp) ca phải mở rộng helper tĩnh bằng chế độ `ls` (~10 dòng trong
  chuỗi nguồn helper). Không thêm ca thứ hai, không đổi ca cũ.
- macOS **không đo được** crate này (xem §6) — đúng thiết kế từ ST-132, không phải thiếu sót mới.
- Đo trên cây **chưa commit** (các agent khác đang sửa `quyen.rs`, `siet_caps_pkg.rs`, infra); đã ghi
  rõ HEAD + sha256 tệp đo để tái lập đúng mã đã đo.
