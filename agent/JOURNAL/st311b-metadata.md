# ST-311b — metadata Unix theo fd (đóng #194)

**Task**: ST-311b (VD-286) · **Nhánh**: `plan/st311b-fd-metadata` · **Commit**: `61bf4e1`
**Nền gốc**: `0bcb5af` (ST-311a/c DONE) · **Ngày**: 2026-11-24

## 1. Phạm vi đã làm

1. **Metadata theo fd** (`crates/vis-host-native/src/quyen/mo.rs` + `guard.rs`):
   - `size`, `is_file`, `is_dir`, `is_symlink`, `read_link`, `canonicalize`, `exists`, `list`,
     `walk` của `FsHost` nay mở **từ fd gốc** `cap` (`openat`/`openat2` + `O_NOFOLLOW`) rồi
     `statat`/`readlinkat`/`fstat`. **0 `unsafe`**, cùng nguồn `openat2` (R9).
   - `phan_giai` = realpath theo fd cho `canonicalize`/`exists`/`read_link`: đi theo symlink
     **trong** gốc, symlink trỏ `..`/tuyệt đối **ra ngoài** ⇒ `Err(XDEV)` ⇒ `VS0604`.
   - `FsGuard::loi_meta`: `ELOOP`/`EXDEV`/`ENOTDIR` ⇒ `VS0604` (thiếu `cap[Dir]`), còn lại giữ
     câu chữ host cũ.
2. **Chia `quyen/mod.rs` 1197 → 675 dòng**: tách `mod tests` sang `quyen/tests.rs` qua `#[path]`
   (giữ `super::*`, 0 đổi hành vi).
3. **Ca đua** (`quyen/st311b.rs`, `#![cfg(all(test, unix))]`): luồng phụ lật `root/sub`
   (thư mục ↔ symlink ra ngoài), luồng chính `exists_quyen("root/sub/secret")` — `secret` chỉ có
   ngoài root. Núm `VIS_ST311B_LAP` (mặc định 100 000).

## 2. Ca đua 100 000 vòng — 0 rò

- **Linux `ps`**: `lap=100000 ok_trong=97736 ro_ri=0 chan=2264` · log `/home/ps/tmp/st311b-race-linux.log`
  · sha256 `43b1a7d1a6962fb5b4cc2c816cefd32256844baf1af4c5088c11a72477aa3c08`
- **macOS `mps`** (Darwin 27.0.0, rustc 1.100.0): `lap=100000 ok_trong=91631 ro_ri=0 chan=8369`
  · log `/Users/mps/tmp/st311b-race-macos.log`
  · sha256 `5bbc7aedaa4b0128e91e3582c76890a24aa20c4de4c0949f98628260ffba96cc`
- Cả hai nền `ro_ri=0`, `ok_trong>0` (ca không "xanh vì chặn tất"). Đối chứng tĩnh:
  `symlink_giua_ra_ngoai_{size,exists}_la_vs0604` + `size_trong_root_tra_dung`.

## 3. Cổng

`./vis-dev gate --fast` @ `61bf4e1`: **3782 passed / 4 skipped** (≥ 3773/4). `vis-dev check` PASS
(fmt + clippy + R1..R9 + self-test + build + doc-examples). R9 (0 bản sao `openat2` ngoài
`vis-host-native`) và R5 (§8 == §4 của `SO-NO.md`) xanh.

## 4. Sổ nợ & tài liệu

- `agent/SO-NO.md`: **#194 `mở → đóng ở ST-311b`**; §8 **80 mở / 97 đóng / 16 VS3 / 2 không làm /
  1 điều hướng = 196** (R5 xanh).
- `agent/HOST-BAO-DAM.md`: khối cập nhật 2026-11-24 (metadata theo fd, đóng #194), **giữ caveat
  Windows** (không `openat`/danh tính tệp).

## 5. Vá ngoài phạm vi (ST-311c) — bắt buộc để gate xanh

`crates/vis-core/tests/core_it.rs` (thêm bởi ST-311c) có **kỳ vọng test lệch hợp đồng thật**,
làm `gate --fast` đỏ trên `0bcb5af`:

- `json_roundtrip_va_escape_duy_nhat`: `esc()` trả **nội dung không dấu bao**, test đưa thẳng vào
  `decode()` ⇒ lỗi "JSON không hợp lệ ở byte 0". Vá: bọc dấu `"`.
- `value_parse_size_duration_va_exit_code`: `parse_size` phân biệt SI (`kb`=1000)/IEC (`kib`=1024),
  test kỳ vọng `5kb=5120`; `parse_duration` trả **nanosecond**, test kỳ vọng `250ms=250`. Vá kỳ
  vọng đúng (thêm ca `5kib=5120`).

**Không phải lỗi ST-311b** — chỉ vá kỳ vọng test để cổng tái lập được; đã ghi chú ngay trong test.

## 6. Blocker

Không còn blocker: macOS race đã chạy trên `mps` thật (bundle git → worktree → chạy → dọn), 0/100k.
